import assert from 'node:assert/strict';
import { randomUUID } from 'node:crypto';
import { performance } from 'node:perf_hooks';

// Real stdio MCP and native broker under live software-only ArtDMX output.
// Timing statistics are observations, not whole audio/video/UI budget proof.
export async function nativeControllerBurstProbe(api) {
  const timing = start => {
    const times = api.packetTimes().slice(start);
    const intervals = times.slice(1).map((value, i) => value - times[i]).sort((a, b) => a - b);
    return { frames: times.length, intervalCount: intervals.length,
      medianMs: intervals[Math.floor(intervals.length * .5)] ?? null,
      p95Ms: intervals[Math.floor(intervals.length * .95)] ?? null,
      maxMs: intervals.at(-1) ?? null };
  };
  const before = await api.state();
  let startFrame = api.packetTimes().length;
  const idle = await api.stable('idle-before-native-burst', api.full, () => api.pause(2000));
  api.checks.push({ check: 'native-burst-idle-live-image-baseline', passed: true,
    ...idle.observation, timing: timing(startFrame) });

  await api.grant('read', 'syndocal.query.control_plane.capabilities.v1');
  startFrame = api.packetTimes().length;
  const discovery = await api.stable('10000-stdio-discovery-requests', api.full, async () => {
    const started = performance.now();
    const replies = await Promise.all(Array.from({ length: 10_000 }, () => api.rpc('tools/list', {})));
    const completed = replies.filter(reply => reply.result);
    const overloaded = replies.filter(reply => reply.error?.code === -32005);
    const check = { check: 'native-10000-discovery-result-accounting', passed: false,
      completed: completed.length, overloaded: overloaded.length,
      other: replies.filter(reply => !reply.result && reply.error?.code !== -32005).slice(0, 4) };
    api.checks.push(check);
    assert.ok(completed.length >= 1);
    assert.ok(overloaded.length > 0);
    assert.equal(completed.length + overloaded.length, 10_000, 'Every discovery has one explicit result');
    assert.ok(completed.every(reply => reply.result.tools.length === 10));
    assert.deepEqual(await api.state(), before);
    assert.deepEqual((await api.rpc('ping', {})).result, {});
    check.passed = true;
    return { requests: replies.length, completed: completed.length, overloaded: overloaded.length,
      elapsedMs: performance.now() - started };
  });
  api.checks.push({ check: 'native-10000-discovery-rejects-before-dispatch-and-preserves-live-image', passed: true,
    ...discovery.result, ...discovery.observation, timing: timing(startFrame) });

  const operationId = 'syndocal.output.lighting.master.set.v2';
  const { fence } = await api.query('syndocal.query.output.control.authority.v1');
  const intents = Array.from({ length: 10_000 }, () => ({ requestId: randomUUID(), operationId,
    request: { request: { operation_id: operationId, request_id: api.nextOutputRequestId(),
      expected_fence: fence, action: { kind: 'set_lighting_master', role: 'lighting',
        master_milliunits: 1000, lease: api.active } } } }));
  startFrame = api.packetTimes().length;
  let admitted;
  const output = await api.stable('10000-stdio-r4-master-requests', api.full, async () => {
    const started = performance.now();
    const receipts = await Promise.all(intents.map(intent => api.call('syndocal_execute_control_plane', intent)));
    const overloads = [], dispatched = [];
    for (let index = 0; index < receipts.length; index++) {
      let receipt = receipts[index];
      assert.equal(receipt.requestId, intents[index].requestId);
      if (receipt.error === 'sidecar_overloaded') {
        assert.equal(receipt.status, 'rejected');
        overloads.push({ receipt, intent: intents[index] });
      } else {
        const until = performance.now() + 15000;
        while (receipt.status === 'pending' && performance.now() < until) {
          await api.pause(100);
          receipt = await api.call('syndocal_get_request_status', { requestId: receipt.requestId });
        }
        assert.equal(receipt.status, 'completed', 'Admitted requests must not disappear');
        if (receipt.result.ok !== true) {
          assert.equal(receipt.result.result.type, 'rejected');
          assert.ok(['stale_fence', 'rate_limited'].includes(receipt.result.result.rejection.error), JSON.stringify(receipt));
        }
        dispatched.push({ receipt, intent: intents[index] });
      }
    }
    assert.ok(overloads.length > 0);
    assert.equal(overloads.length + dispatched.length, 10_000);
    admitted = dispatched.find(item => item.receipt.result.ok === true);
    assert.ok(admitted, 'At least one immutable R4 intent commits without individual approval');
    api.accepted(admitted.receipt);
    const unsent = await api.call('syndocal_get_request_status', { requestId: overloads[0].receipt.requestId });
    assert.equal(unsent.status, 'unknown', 'Pre-dispatch rejection must not create a native request');
    assert.deepEqual(await api.state(), before);
    return { requests: receipts.length, completed: dispatched.length,
      committed: dispatched.filter(item => item.receipt.result.ok === true).length,
      nativeRejected: dispatched.filter(item => item.receipt.result.ok !== true).length,
      overloaded: overloads.length, elapsedMs: performance.now() - started,
      committedRequestId: admitted.receipt.requestId, unsentRequestId: unsent.requestId };
  });
  api.checks.push({ check: 'native-10000-r4-master-intents-explicit-rejection-and-no-admitted-loss', passed: true,
    ...output.result, ...output.observation, timing: timing(startFrame) });

  const replay = await api.send({ operationId: admitted.intent.operationId, request: admitted.intent.request }, admitted.intent.requestId);
  assert.deepEqual(replay, admitted.receipt, 'Accepted intent replay is byte-equivalent and never executes twice');
  api.accepted((await api.output(operationId, { kind: 'set_lighting_master', role: 'lighting',
    master_milliunits: 500, lease: api.active })).receipt);
  await api.awaitImage(api.half);
  const changed = await api.state();
  assert.equal(changed.master, .5);
  assert.deepEqual(changed.ownership, before.ownership);
  assert.deepEqual(changed.routes, before.routes);
  assert.deepEqual(changed.attributes, before.attributes);
  const recovered = await api.stable('explicit-post-burst-half-master', api.half);
  api.checks.push({ check: 'native-post-burst-receipt-replay-and-new-explicit-output-intent', passed: true,
    replayRequestId: replay.requestId, master: changed.master, ...recovered.observation });
}
