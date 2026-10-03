import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import path from 'node:path';
import { randomUUID } from 'node:crypto';
import { performance } from 'node:perf_hooks';

// Use the existing renderer-registration boundary to retain real immutable
// external requests. Saturate all 64 broker slots, then exercise the independent
// production local S0 path. No DOM, invoke patch, fake engine or physical route.
export async function nativeControllerSafetyPressureProbe(h, mode) {
  const backend = h.backend();
  const before = await h.state();
  const principal = h.principal();
  const destination = path.join(h.directory, `blocked-${mode}.zip`);
  await h.grant('file', 'syndocal.diagnostics.export.v1');
  const { fence } = await backend.invoke('query_output_control_authority_v1');
  const generation = await backend.invoke('agent_bridge_register_v1');
  const masterOperation = 'syndocal.output.lighting.master.set.v2';
  const outputIntent = value => ({ requestId: randomUUID(), operationId: masterOperation,
    request: { request: { operation_id: masterOperation, request_id: h.nextOutputRequestId(),
      expected_fence: fence, action: { kind: 'set_lighting_master', role: 'lighting',
        master_milliunits: value, lease: h.active } } } });
  const r4 = outputIntent(500);
  const r5 = { requestId: randomUUID(), destination };
  const pendingIds = [];
  for (const [name, args] of [['syndocal_execute_control_plane', r4], ['syndocal_export_diagnostics', r5]]) {
    const receipt = await h.call(name, args);
    assert.equal(receipt.status, 'pending');
    assert.equal(receipt.requestId, args.requestId);
    pendingIds.push(receipt.requestId);
    const claimed = await backend.invoke('agent_bridge_claim_v1', { rendererGeneration: generation, requestId: receipt.requestId });
    assert.equal(claimed.principalId, principal.principalId);
    assert.equal(claimed.principalIncarnation, principal.principalIncarnation);
    assert.deepEqual(claimed.params, name === 'syndocal_export_diagnostics' ? { destination }
      : { operationId: r4.operationId, request: r4.request });
  }
  const fill = await h.stable(`${mode}-64-pending-native-requests`, h.full, async () => {
    for (let index = 2; index < 64; index++) {
      const receipt = await h.call('syndocal_get_runtime_status', {});
      assert.equal(receipt.status, 'pending');
      pendingIds.push(receipt.requestId);
    }
    const rejected = await h.call('syndocal_get_runtime_status', {});
    assert.equal(rejected.status, 'rejected');
    assert.equal(rejected.error, 'inflight_capacity');
    assert.equal((await h.call('syndocal_get_request_status', { requestId: rejected.requestId })).status, 'unknown');
    assert.equal(pendingIds.length, 64);
    assert.equal(new Set(pendingIds).size, 64);
    assert.deepEqual(await h.state(), before);
    return { pendingCount: pendingIds.length, overflowError: rejected.error };
  });
  h.checks.push({ check: `native-${mode}-64-pending-r4-r5-and-read-requests-with-no-live-delta`,
    passed: true, ...fill.result, ...fill.observation, rendererGeneration: generation,
    claimedRequestIds: [r4.requestId, r5.requestId] });

  const intents = Array.from({ length: 10_000 }, () => outputIntent(1000));
  let received = 0;
  const floodStarted = performance.now();
  const repliesPromise = Promise.all(intents.map(intent => h.call('syndocal_execute_control_plane', intent)
    .then(receipt => { received++; return receipt; })));
  // Keep rejection observed while local native actions run, even on a failure.
  repliesPromise.catch(() => {});
  const outstandingAtLocalAction = intents.length - received;
  assert.ok(outstandingAtLocalAction > 0);
  const priorityRequest = { operation_id: 'syndocal.safety.blackout.engage.v1',
    request_id: h.nextOutputRequestId() };
  if (mode === 'kill_switch') await backend.invoke('agent_authority_kill_switch_v1');
  const outstandingAtS0 = intents.length - received;
  assert.ok(outstandingAtS0 > 0, 'The local S0 action must overlap outstanding flood requests, including after Kill Switch');
  const issuedAt = performance.now();
  const localReceipt = await backend.invoke('safety_blackout_engage_v1', { request: priorityRequest });
  const localElapsedMs = performance.now() - issuedAt;
  assert.equal(localReceipt.kind, 'receipt', JSON.stringify(localReceipt));
  assert.equal(localReceipt.result.outcome, 'applied');
  assert.equal(localReceipt.result.request_id, priorityRequest.request_id);
  await h.awaitImage(h.zero);
  const firstZero = h.imageTimes().find(sample => sample.at >= issuedAt && sample.zero);
  assert.ok(firstZero, 'A new all-zero packet must be received after the local action');
  const replies = await repliesPromise;
  const expectedNativeError = mode === 'kill_switch' ? 'agent_kill_switch_active' : 'inflight_capacity';
  for (let index = 0; index < replies.length; index++) {
    assert.equal(replies[index].requestId, intents[index].requestId);
    assert.equal(replies[index].status, 'rejected');
    assert.ok(['sidecar_overloaded', expectedNativeError].includes(replies[index].error), JSON.stringify(replies[index]));
  }
  const sidecarOverloads = replies.filter(r => r.error === 'sidecar_overloaded').length;
  assert.ok(sidecarOverloads > 0 && sidecarOverloads < 10_000);
  const snapshot = (await backend.invoke('get_snapshot')).snapshot;
  assert.equal(snapshot.blackout, true);
  assert.equal(snapshot.safety_blackout_engaged, true);
  const dark = await h.stable(`${mode}-local-priority-all-zero`, h.zero);
  h.checks.push({ check: `native-${mode}-local-priority-blackout-during-10000-external-intents`, passed: true,
    requests: replies.length, sidecarOverloads, nativeRejected: replies.length - sidecarOverloads,
    nativeError: expectedNativeError, outstandingAtLocalAction, outstandingAtS0, localElapsedMs,
    firstZeroPacketMs: firstZero.at - issuedAt, floodElapsedMs: performance.now() - floodStarted,
    receipt: localReceipt, ...dark.observation });

  if (mode === 'saturation') {
    // All accepted requests remain recoverable as pending through S0. Do not
    // manufacture completion or execute them to clear capacity.
    for (const requestId of pendingIds) {
      assert.equal((await h.call('syndocal_get_request_status', { requestId })).status, 'pending');
    }
    await backend.invoke('agent_authority_revoke_v1', principal);
  }
  const refusal = mode === 'kill_switch' ? 'agent_kill_switch_active' : 'agent_principal_revoked';
  for (const requestId of [r4.requestId, r5.requestId]) {
    const execute = () => backend.evaluate(`window.__TAURI_INTERNALS__.invoke('agent_bridge_execute_native_v1',
      ${JSON.stringify({ rendererGeneration: generation, requestId })})
      .then(value=>({value}),error=>({error:String(error)}))`);
    assert.deepEqual(await execute(), { error: refusal });
    assert.deepEqual(await execute(), { error: 'request_not_executable' });
    const hidden = await h.call('syndocal_get_request_status', { requestId });
    assert.equal(hidden.status, 'rejected');
    assert.equal(hidden.error, refusal);
    assert.equal(hidden.result, undefined);
  }
  assert.equal(await fs.stat(destination).then(() => true, error => { if (error.code === 'ENOENT') return false; throw error; }), false);
  assert.deepEqual(await h.state(), before, 'S0/revocation do not rewrite authored output/fixture state');
  const denied = await h.call('syndocal_execute_control_plane', outputIntent(0));
  assert.equal(denied.status, 'rejected');
  assert.equal(denied.error, refusal);
  const retained = await h.stable(`${mode}-revoked-r4-r5-cannot-change-zero`, h.zero);
  h.checks.push({ check: `native-${mode}-revoked-claimed-r4-r5-reject-and-no-export-or-authored-delta`,
    passed: true, executionError: refusal, consumedReplayError: 'request_not_executable',
    retainedPendingCount: mode === 'saturation' ? pendingIds.length : null,
    destinationAbsent: true, ...retained.observation });

  assert.deepEqual(await backend.invoke('safety_blackout_engage_v1', { request: priorityRequest }), localReceipt);
  const again = await backend.invoke('safety_blackout_engage_v1', { request: { ...priorityRequest,
    request_id: h.nextOutputRequestId() } });
  assert.equal(again.kind, 'receipt');
  assert.equal(again.result.outcome, 'no_op');
  const invalid = await backend.evaluate(`window.__TAURI_INTERNALS__.invoke('safety_blackout_engage_v1',
    ${JSON.stringify({ request: { ...priorityRequest, request_id: h.nextOutputRequestId(), enabled: false } })})
    .then(()=>({accepted:true}),error=>({error:String(error)}))`);
  assert.match(invalid.error, /unknown field.*enabled/);
  const replay = await h.stable(`${mode}-local-replay-and-no-release-shape`, h.zero);
  h.checks.push({ check: `native-${mode}-local-s0-replay-no-op-and-target-payload-rejection`,
    passed: true, replayAuditSequence: localReceipt.result.audit_sequence,
    noOpAuditSequence: again.result.audit_sequence, ...replay.observation });
}
