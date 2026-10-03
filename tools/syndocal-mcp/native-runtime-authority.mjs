import assert from 'node:assert/strict';
import { randomUUID } from 'node:crypto';
import { openNativeStdioSession } from './native-stdio-session.mjs';

const queries = [
  ['syndocal.query.runtime.timeline.transport.authority.v1', 'query_timeline_transport_authority_v1', 'syndocal.runtime.timeline.transport.set_playing.v1'],
  ['syndocal.query.runtime.timeline.loop.authority.v1', 'query_timeline_loop_runtime_authority_v1', 'syndocal.runtime.timeline.loop.commit.v1'],
  ['syndocal.query.runtime.timeline.follow.abort.authority.v1', 'query_timeline_follow_abort_authority_v1', 'syndocal.runtime.timeline.follow.abort.v1'],
];

// Read actual native authorities through the authenticated MCP and native IPC
// surfaces. Concurrent ordinary reads create real contention; no locks, invoke
// functions, ownership, command queues or response codes are patched.
export async function nativeRuntimeAuthority(backend, options, checks) {
  let mcp;
  const ownerId = `runtime-authority-${randomUUID()}`;
  await backend.invoke('register_project_transaction_owner', { ownerId });
  const initialAuthority = await backend.invoke('get_project_authority_bundle');
  const checkpoint = () => backend.invoke('get_project_checkpoint', { midiMappings: [], oscMappings: [], dmxMappings: [] });
  let before, beforeAuthority, outputBefore, runtimeBefore;
  const send = async operationId => {
    const requestId = randomUUID();
    let receipt = await mcp.call('syndocal_execute_control_plane', { requestId, operationId, request: {} });
    const until = Date.now() + 15000;
    while (receipt.status === 'pending' && Date.now() < until) {
      await new Promise(resolve => setTimeout(resolve, 50));
      receipt = await mcp.call('syndocal_get_request_status', { requestId });
    }
    assert.equal(receipt.requestId, requestId);
    return receipt;
  };
  const verify = (value, operationId) => {
    assert.equal(value.operation_id, operationId);
    assert.match(value.authority_id, /^[A-Za-z0-9_-]{22}$/);
    const { project } = value.fence;
    assert.equal(project.project_epoch, beforeAuthority.project_epoch);
    assert.equal(project.project_revision, beforeAuthority.project_revision);
    assert.equal(project.project_checkpoint_hash, beforeAuthority.checkpoint_hash);
    for (const key of ['process_incarnation', 'session_incarnation']) {
      assert.ok(Number.isSafeInteger(project[key]) && project[key] > 0);
    }
  };
  try {
    mcp = await openNativeStdioSession(options);
    const denied = await send(queries[0][0]);
    assert.equal(denied.status, 'rejected');
    assert.equal(denied.error, 'agent_missing_grant');
    checks.push({ check: 'runtime-authority-external-read-requires-exact-grant', passed: true });
    for (const [operationId] of queries) {
      await backend.invoke('agent_authority_grant_v1', {
        principalId: options.principalId, principalIncarnation: options.principalIncarnation,
        grant: { adapter: 'external_mcp', capability: 'read', operation_id: operationId, project_id: null },
      });
    }
    const idleNative = await backend.evaluate(`window.__TAURI_INTERNALS__.invoke('query_timeline_follow_abort_authority_v1')
      .then(value=>({ok:true,value}),error=>({ok:false,error}))`);
    assert.deepEqual(idleNative, { ok: false, error: { code: 'stale_fence' } });
    const idleReceipt = await send(queries[2][0]);
    const idle = { check: 'runtime-authority-idle-follow-native-error-survives-mcp', passed: false,
      native: idleNative, receipt: idleReceipt };
    checks.push(idle);
    assert.equal(idleReceipt.status, 'completed');
    assert.equal(idleReceipt.result.ok, false);
    assert.equal(idleReceipt.result.error.code, 'request_rejected');
    assert.deepEqual(idleReceipt.result.error.native_runtime, idleNative.error);
    idle.passed = true;
    // A fresh process has no Follow generation. The normal New operation
    // establishes one without a device, media source or output activation.
    await backend.invoke('new_project', { ownerId, expectedEpoch: initialAuthority.project_epoch,
      expectedRevision: initialAuthority.project_revision, expectedCheckpointHash: initialAuthority.checkpoint_hash });
    before = await checkpoint();
    beforeAuthority = await backend.invoke('get_project_authority_bundle');
    outputBefore = await backend.invoke('get_output_ownership_status');
    runtimeBefore = (await backend.invoke('get_snapshot')).timeline_runtime;
    assert.equal(outputBefore.lighting_allowed, false);
    assert.equal(outputBefore.video_allowed, false);
    const canonical = { check: 'runtime-authority-three-canonical-native-mcp-reads', passed: false, reads: [] };
    checks.push(canonical);
    for (const [operationId, command, targetOperation] of queries) {
      const native = await backend.evaluate(`window.__TAURI_INTERNALS__.invoke(${JSON.stringify(command)})
        .then(value=>({ok:true,value}),error=>({ok:false,error}))`);
      canonical.reads.push({ operationId, native });
      assert.equal(native.ok, true, JSON.stringify(native));
      verify(native.value, targetOperation);
      const receipt = await send(operationId);
      canonical.reads.at(-1).receipt = receipt;
      assert.equal(receipt.status, 'completed');
      assert.equal(receipt.result.ok, true, JSON.stringify(receipt.result));
      verify(receipt.result.result, targetOperation);
    }
    canonical.passed = true;

    const pressure = { check: 'runtime-authority-concurrent-native-reads-preserve-error-codes', passed: false,
      rounds: 0, authorityAttempts: 0, successfulAuthorities: 0, errors: [], companionReads: 0,
      successfulCompanions: 0, companionErrors: {} };
    checks.push(pressure);
    for (let round = 0; round < 64; round++) {
      const result = await backend.evaluate(`(async () => {
        const invoke = window.__TAURI_INTERNALS__.invoke;
        const capture = (command, args) => invoke(command, args).then(value => ({ok:true,value}), error => ({ok:false,error}));
        const companions = Array.from({length:32}, () => capture('get_project_checkpoint_bundle', ${JSON.stringify({
          expectedEpoch: beforeAuthority.project_epoch, expectedRevision: beforeAuthority.project_revision,
          expectedCheckpointHash: beforeAuthority.checkpoint_hash,
        })}));
        const authorities = ${JSON.stringify(queries.map(q => q[1]))}.map(command => capture(command));
        const [companionResults, authorityResults] = await Promise.all([Promise.all(companions), Promise.all(authorities)]);
        return { companions:companionResults.map(r => r.ok ? {ok:true, token:{
          project_epoch:r.value.project_epoch, project_revision:r.value.project_revision,
          checkpoint_hash:r.value.checkpoint_hash }} : {ok:false,error:r.error}), authorities:authorityResults };
      })()`);
      pressure.rounds++;
      pressure.companionReads += result.companions.length;
      for (const companion of result.companions) {
        if (companion.ok) {
          assert.deepEqual(companion.token, { project_epoch: beforeAuthority.project_epoch,
            project_revision: beforeAuthority.project_revision, checkpoint_hash: beforeAuthority.checkpoint_hash });
          pressure.successfulCompanions++;
        } else {
          // Recovery capture intentionally uses try_lock admission. A call is
          // either captured or explicitly refused as busy, never lost. This
          // producer is not a promise that all offered reads are admitted.
          assert.equal(companion.error, 'Project recovery capture is busy; retry');
          pressure.companionErrors[companion.error] = (pressure.companionErrors[companion.error] ?? 0) + 1;
        }
      }
      for (let index = 0; index < queries.length; index++) {
        const response = result.authorities[index];
        pressure.authorityAttempts++;
        if (response.ok) { verify(response.value, queries[index][2]); pressure.successfulAuthorities++; }
        else {
          pressure.errors.push({ round, command: queries[index][1], error: response.error });
          assert.deepEqual(Object.keys(response.error), ['code'], 'Authority errors expose only the fixed runtime code');
          assert.ok(['overloaded', 'busy'].includes(response.error.code), JSON.stringify(pressure.errors.at(-1)));
        }
      }
    }
    assert.ok(pressure.successfulCompanions > 0, 'Pressure must include actual successful checkpoint captures');
    assert.equal(pressure.successfulCompanions + Object.values(pressure.companionErrors).reduce((a,b) => a+b, 0),
      pressure.companionReads, 'Every offered companion read has an explicit result');
    assert.ok(pressure.errors.some(row => row.error.code === 'overloaded'),
      'The native pressure phase must actually observe contention');
    pressure.passed = true;
    for (const [operationId, , targetOperation] of queries) {
      const receipt = await send(operationId);
      assert.equal(receipt.status, 'completed');
      assert.equal(receipt.result.ok, true, JSON.stringify(receipt.result));
      verify(receipt.result.result, targetOperation);
    }
    assert.deepEqual(await checkpoint(), before);
    assert.deepEqual(await backend.invoke('get_output_ownership_status'), outputBefore);
    assert.deepEqual((await backend.invoke('get_snapshot')).timeline_runtime, runtimeBefore);
    checks.push({ check: 'runtime-authority-fresh-mcp-reads-recover-with-authored-runtime-output-unchanged', passed: true });
  } finally { await mcp?.close(); }
}
