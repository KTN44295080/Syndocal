import assert from 'node:assert/strict';
import { createHash, randomUUID } from 'node:crypto';
import { performance } from 'node:perf_hooks';

const runtimeOperation = 'syndocal.query.runtime.generations.v1';
const eventsOperation = 'syndocal.query.events.observations.v1';
const projectOperation = 'syndocal.query.project.authority.v1';
const outputOperation = 'syndocal.query.output.ownership.v1';

// Actual native observations and authenticated immutable MCP requests. A slow
// consumer renews its real single-use cursor by reading one event at a time;
// neither retention, TTL, the 4/s runtime rate nor engine state is patched.
export async function nativeControllerEventPressureProbe(h) {
  const { executeAgentBridgeCanonicalOperation } = await import('../../app/src/agentBridgeControlPlane.ts');
  const backend = h.backend();
  const principal = h.principal();
  const before = await h.checkpoint();
  const wireBefore = await backend.invoke('get_snapshot');
  const runtimeBefore = { duration_ms: wireBefore.snapshot.timeline.duration_ms, ...wireBefore.timeline_runtime };
  assert.equal(runtimeBefore.duration_ms, 4000);
  assert.equal(runtimeBefore.loop_runtime.a_ms, 0);
  assert.equal(runtimeBefore.loop_runtime.b_ms, 4000);
  const digest = value => createHash('sha256').update(JSON.stringify(value)).digest('hex');
  for (const operation of [runtimeOperation, eventsOperation, projectOperation, outputOperation]) {
    await h.grant('read', operation);
  }
  const initial = (await h.send({ operationId: runtimeOperation, request: { request: { limit: 10 } } })).result;
  assert.equal(initial.ok, true);
  const initialPage = initial.result;
  assert.equal(initialPage.items.length, 5);
  const subscription = (await h.send({ operationId: eventsOperation,
    request: { request: { limit: 1, expected_fence: initialPage.snapshot_fence } } })).result;
  const subscriptionCheck = { check: 'native-mcp-snapshot-fence-roundtrip-starts-event-subscription',
    passed: false, initialPage, subscription };
  h.checks.push(subscriptionCheck);
  assert.equal(subscription.ok, true, JSON.stringify(subscription));
  assert.deepEqual(subscription.result.events, []);
  assert.equal(subscription.result.gap, undefined);
  let cursor = subscription.result.next_cursor;
  assert.equal(typeof cursor, 'string');
  subscriptionCheck.passed = true;
  let cursorIssuedAt = performance.now();
  let consumedGeneration = initialPage.snapshot_fence.event_stream_generation;
  const rendererGeneration = await backend.invoke('agent_bridge_register_v1');
  const pending = [];
  let completedReads = 0;
  const admit = async (operationId, request) => {
    const args = { requestId: randomUUID(), operationId, request };
    const receipt = await h.call('syndocal_execute_control_plane', args);
    assert.equal(receipt.status, 'pending', JSON.stringify(receipt));
    assert.equal(receipt.requestId, args.requestId);
    assert.equal(receipt.result, undefined);
    return args;
  };
  const completeRead = async intent => {
    const claim = await backend.invoke('agent_bridge_claim_v1', { rendererGeneration, requestId: intent.requestId });
    assert.equal(claim.principalId, principal.principalId);
    assert.equal(claim.principalIncarnation, principal.principalIncarnation);
    assert.equal(claim.method, 'control_plane.execute');
    assert.deepEqual(claim.params, { operationId: intent.operationId, request: intent.request });
    assert.ok([runtimeOperation, eventsOperation, projectOperation, outputOperation].includes(intent.operationId));
    // The production canonical adapter invokes the actual native query. Its
    // result, not a synthesized completion, is retained by the existing broker.
    const result = await executeAgentBridgeCanonicalOperation(backend.invoke, claim.params);
    await backend.invoke('agent_bridge_complete_v1', { rendererGeneration, requestId: intent.requestId, result });
    const receipt = await h.call('syndocal_get_request_status', { requestId: intent.requestId });
    assert.equal(receipt.status, 'completed');
    assert.deepEqual(receipt.result, result);
    completedReads++;
    return result.result;
  };
  const refill = async () => {
    while (pending.length < 64) pending.push(await admit(runtimeOperation, { request: { limit: 10 } }));
  };
  // Only one previously admitted read is actually executed to free capacity.
  // A fresh subscriber request occupies that slot until its real completion.
  const read = async (operationId, request) => {
    assert.equal(pending.length, 64);
    await completeRead(pending.shift());
    const intent = await admit(operationId, request);
    const result = await completeRead(intent);
    await refill();
    return result;
  };
  const fill = await h.stable('event-pressure-64-pending-reads', h.full, async () => {
    await refill();
    const overflow = await h.call('syndocal_get_runtime_status', {});
    assert.equal(overflow.status, 'rejected');
    assert.equal(overflow.error, 'inflight_capacity');
    assert.equal((await h.call('syndocal_get_request_status', { requestId: overflow.requestId })).status, 'unknown');
    return { pendingCount: pending.length, overflowError: overflow.error };
  });
  h.checks.push({ check: 'native-event-subscriber-and-64-real-pending-reads', passed: true,
    ...fill.result, ...fill.observation, initialFence: initialPage.snapshot_fence, rendererGeneration,
    runtimeBefore });
  const refreshedSubscription = await read(eventsOperation, { request: { limit: 1, cursor } });
  assert.deepEqual(refreshedSubscription.events, []);
  assert.equal(refreshedSubscription.gap, undefined);
  cursor = refreshedSubscription.next_cursor;
  cursorIssuedAt = performance.now();

  let applied = 0, enabled = false, maxCommitMs = 0, maxCaptureMs = 0;
  let latestPage = initialPage;
  const queryOverloads = [];
  const nativeRead = async (command, params = {}) => {
    for (let attempt = 0; attempt < 3; attempt++) {
      const wire = await backend.evaluate(`window.__TAURI_INTERNALS__.invoke(${JSON.stringify(command)}, ${JSON.stringify(params)})
        .then(value=>({value}),error=>({error}))`);
      if (wire.error === undefined) return wire.value;
      if (wire.error?.code === 'overloaded' && attempt < 2) {
        queryOverloads.push({ command, error: wire.error });
        await h.pause(100);
        continue; // A fresh pre-admission read; never retry an uncertain mutation.
      }
      assert.fail(`Native read ${command}: ${JSON.stringify(wire.error)}`);
    }
  };
  const commit = async () => {
    const authority = await nativeRead('query_timeline_loop_runtime_authority_v1');
    const started = performance.now();
    const response = await backend.invoke('commit_timeline_loop_runtime_v1', { request: {
      operation_id: 'syndocal.runtime.timeline.loop.commit.v1', authority_id: authority.authority_id,
      request_id: h.nextOutputRequestId(), expected_fence: authority.fence,
      action: { kind: 'set_enabled', enabled: !enabled },
    } });
    maxCommitMs = Math.max(maxCommitMs, performance.now() - started);
    assert.equal(response.kind, 'receipt', JSON.stringify(response));
    assert.equal(response.result.outcome, 'applied');
    assert.ok(response.result.loop_generation_after > authority.fence.source_loop_generation);
    enabled = !enabled; applied++;
    const capturedAt = performance.now();
    latestPage = await nativeRead('query_control_plane_runtime_generations', { request: { limit: 10 } });
    maxCaptureMs = Math.max(maxCaptureMs, performance.now() - capturedAt);
    assert.equal(latestPage.items.find(item => item.domain === 'timeline.loop').active, enabled);
    assert.equal(latestPage.snapshot_fence.project_epoch, initialPage.snapshot_fence.project_epoch);
    assert.equal(latestPage.snapshot_fence.project_revision, initialPage.snapshot_fence.project_revision);
    assert.equal(latestPage.snapshot_fence.project_checkpoint_hash, initialPage.snapshot_fence.project_checkpoint_hash);
  };
  const slowPages = [];
  let gapPage;
  const floodStarted = performance.now();
  const flood = await h.stable('event-pressure-real-runtime-publication-and-retention-overflow', h.full, async () => {
    for (let index = 0; index < 2400; index++) {
      const started = performance.now();
      await commit();
      if (applied % 64 === 0) {
        const cursorAgeMs = performance.now() - cursorIssuedAt;
        assert.ok(cursorAgeMs < 50000, 'Read the real cursor before its 60-second TTL');
        const page = await read(eventsOperation, { request: { limit: 1, cursor } });
        slowPages.push({ cursorAgeMs, snapshotFence: page.snapshot_fence, events: page.events, gap: page.gap });
        if (page.gap) { gapPage = page; break; }
        assert.equal(page.events.length, 1);
        assert.equal(page.events[0].generation, consumedGeneration + 1);
        consumedGeneration = page.events[0].generation;
        cursor = page.next_cursor;
        assert.equal(typeof cursor, 'string');
        cursorIssuedAt = performance.now();
        console.log(`event pressure: ${applied} applied Loop changes; stream ${latestPage.snapshot_fence.event_stream_generation}; slow consumer ${consumedGeneration}; pending 64`);
      }
      // Leave the established local token bucket intact (4/s, burst 8).
      await h.pause(Math.max(0, 275 - (performance.now() - started)));
    }
    assert.ok(gapPage, 'Actual native retention must expire before the bounded 2400-mutation ceiling');
    return { applied, elapsedMs: performance.now() - floodStarted, maxCommitMs, maxCaptureMs,
      slowPages, queryOverloads, publishedFence: latestPage.snapshot_fence };
  });
  h.checks.push({ check: 'native-publisher-and-live-image-continue-under-64-pending-slots', passed: true,
    ...flood.result, ...flood.observation });
  assert.deepEqual(gapPage.events, []);
  assert.equal(gapPage.next_cursor, undefined);
  assert.equal(gapPage.gap.reason, 'retention_expired');
  assert.equal(gapPage.gap.resnapshot_required, true);
  assert.equal(gapPage.gap.previous_generation, consumedGeneration);
  assert.equal(gapPage.gap.previous_stream_epoch, initialPage.snapshot_fence.event_stream_epoch);
  assert.equal(gapPage.gap.next_stream_epoch, initialPage.snapshot_fence.event_stream_epoch);
  assert.equal(gapPage.gap.next_available_generation, latestPage.snapshot_fence.event_stream_generation - 2048 + 1);
  assert.ok(gapPage.gap.next_available_generation > consumedGeneration + 1);
  h.checks.push({ check: 'native-retained-event-gap-requires-fresh-snapshot', passed: true, gapPage });

  const resync = await h.stable('event-pressure-authoritative-resnapshot-and-delta-convergence', h.full, async () => {
    const runtime = await read(runtimeOperation, { request: { limit: 10 } });
    const project = await read(projectOperation, { request: { limit: 10 } });
    const output = await read(outputOperation, { request: { limit: 10 } });
    assert.deepEqual(project.snapshot_fence, runtime.snapshot_fence);
    assert.deepEqual(output.snapshot_fence, runtime.snapshot_fence);
    const client = new Map(runtime.items.map(item => [item.domain, item]));
    const subscribed = await read(eventsOperation, { request: { limit: 1, expected_fence: runtime.snapshot_fence } });
    assert.deepEqual(subscribed.events, []);
    assert.equal(subscribed.gap, undefined);
    let nextCursor = subscribed.next_cursor;
    let nextGeneration = runtime.snapshot_fence.event_stream_generation;
    const deltas = [];
    await commit(); await h.pause(275); await commit();
    while (nextGeneration < latestPage.snapshot_fence.event_stream_generation) {
      const page = await read(eventsOperation, { request: { limit: 1, cursor: nextCursor } });
      assert.equal(page.gap, undefined);
      assert.equal(page.events.length, 1);
      const event = page.events[0];
      assert.equal(event.generation, nextGeneration + 1);
      assert.equal(event.stream_epoch, runtime.snapshot_fence.event_stream_epoch);
      assert.equal(event.payload.kind, 'runtime_generation_changed');
      client.set(event.payload.payload.domain, event.payload.payload);
      nextGeneration = event.generation;
      nextCursor = page.next_cursor;
      deltas.push(event);
    }
    const empty = await read(eventsOperation, { request: { limit: 1, cursor: nextCursor } });
    assert.deepEqual(empty.events, []);
    assert.equal(empty.gap, undefined);
    assert.deepEqual(empty.snapshot_fence, latestPage.snapshot_fence);
    assert.deepEqual([...client.values()].sort((a, b) => a.domain.localeCompare(b.domain)), latestPage.items);
    assert.deepEqual(await h.checkpoint(), before, 'Runtime Loop pressure cannot alter the authored project/checkpoint');
    return { runtimeSnapshot: runtime, projectSnapshot: project, outputSnapshot: output,
      deltas, convergedFence: empty.snapshot_fence, authoredCheckpointSha256: digest(before) };
  });
  h.checks.push({ check: 'native-subscriber-resnapshots-and-converges-to-exact-canonical-state', passed: true,
    ...resync.result, ...resync.observation });
  for (const intent of pending) assert.equal((await h.call('syndocal_get_request_status', { requestId: intent.requestId })).status, 'pending');
  h.checks.push({ check: 'native-event-pressure-retains-all-64-accepted-requests', passed: true,
    pendingCount: pending.length, completedReads, applied, sourceFixtureUnchangedByRuntime: true });
}
