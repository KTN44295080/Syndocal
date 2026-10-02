import assert from 'node:assert/strict';
import { randomUUID } from 'node:crypto';
import { performance } from 'node:perf_hooks';
import { openNativeStdioSession } from './native-stdio-session.mjs';

// Empty isolated QA project. Lease authority only; never Enable/Arm or actuate output.
// Default persisted DMX configuration is Enabled; the runtime ownership gate
// blocks sender creation. Preserve both the config and the closed runtime gate.
// The stdio adapter is not the registered native window's lease owner.
export async function nativeLeaseExpiry(backend, options, checks) {
  const pause = ms => new Promise(resolve => setTimeout(resolve, ms));
  let mcp, sequence = Date.now();
  const readOverloads = [];
  const grant = (capability, operationId) => backend.invoke('agent_authority_grant_v1', {
    principalId: options.principalId, principalIncarnation: options.principalIncarnation,
    grant: { adapter: 'external_mcp', capability, operation_id: operationId, project_id: null },
  });
  const send = async (params, requestId = randomUUID()) => {
    let receipt = await mcp.call('syndocal_execute_control_plane', { requestId, ...params });
    const deadline = performance.now() + 15000;
    while (receipt.status === 'pending' && performance.now() < deadline) {
      await pause(100);
      receipt = await mcp.call('syndocal_get_request_status', { requestId });
    }
    assert.equal(receipt.status, 'completed', JSON.stringify(receipt));
    return receipt;
  };
  const query = async operationId => {
    for (let attempt = 0; attempt < 3; attempt++) {
      const receipt = await send({ operationId, request: {} });
      const error = receipt.result.error;
      const overloaded = error?.native_query?.code === 'overloaded'
        && error.native_query.retryable === true && error.native_query.resnapshot_required === false;
      // Output-fence query returns Rust's string error; lease query returns
      // canonical QueryError JSON. Both current native routes are supported.
      const fenceOverload = /QueryError \{ code: Overloaded,/.test(error?.message ?? '')
        && /retryable: true/.test(error?.message ?? '');
      if (receipt.result.ok === false && error?.code === 'request_rejected'
        && (overloaded || fenceOverload) && attempt < 2) {
        readOverloads.push({ operationId, requestId: receipt.requestId, error: receipt.result.error });
        await pause(100);
        continue; // new terminally rejected read intent; never retry a mutation
      }
      assert.equal(receipt.result.ok, true, JSON.stringify(receipt.result));
      return receipt.result.result;
    }
    assert.fail('Canonical read attempt bound');
  };
  const leases = () => query('syndocal.output.lease.authority.query.v1');
  const output = async (operationId, action, expectedOk = true) => {
    const { fence } = await query('syndocal.query.output.control.authority.v1');
    const params = { operationId, request: { request: {
      operation_id: operationId, request_id: sequence++, expected_fence: fence, action,
    } } };
    const id = randomUUID(), receipt = await send(params, id);
    assert.equal(receipt.result.ok, expectedOk, JSON.stringify(receipt.result));
    return { id, params, receipt, response: receipt.result.result };
  };
  const observe = async () => ({
    ownership: await backend.invoke('get_output_ownership_status'),
    image: await backend.evaluate(`window.__TAURI_INTERNALS__.invoke('get_snapshot').then(({snapshot:s})=>({
      output:s.output,dmxOutputs:s.dmx_outputs,blackout:s.blackout,authoredBlackout:s.authored_blackout,
      videoOutputs:s.video.outputs,videoBlackout:s.video.blackout,videoMaster:s.video.master_opacity}))`),
  });
  try {
    await backend.invoke('agent_authority_promote_v1', {
      principalId: options.principalId, principalIncarnation: options.principalIncarnation,
    });
    for (const id of ['syndocal.output.lease.authority.query.v1', 'syndocal.query.output.control.authority.v1']) await grant('read', id);
    for (const id of ['acquire', 'renew', 'recover', 'relinquish']) await grant('output', `syndocal.output.lease.${id}.v2`);
    mcp = await openNativeStdioSession(options);
    const before = await observe();
    assert.equal(before.ownership.lighting_allowed, false);
    assert.equal(before.ownership.video_allowed, false);
    assert.deepEqual(before.image.videoOutputs, []);
    assert.deepEqual((await leases()).statuses, [{ status: 'unavailable' }]);
    const acquireStarted = performance.now();
    const acquired = await output('syndocal.output.lease.acquire.v2', { kind: 'acquire_lease', role: 'lighting' });
    assert.equal(acquired.response.type, 'receipt', JSON.stringify(acquired.response));
    const authority = acquired.response.receipt.lease_result.authority;
    const held = await leases();
    assert.deepEqual(held.statuses, [{ status: 'held_active', authority, resources: ['lighting'] }]);
    const firstSidecarPid = mcp.processId;
    await mcp.close(); // waits for this owned child to exit; backend stays alive
    mcp = await openNativeStdioSession(options);
    assert.notEqual(mcp.processId, firstSidecarPid);
    const afterReconnect = await leases();
    assert.deepEqual(afterReconnect.statuses, held.statuses);
    assert.deepEqual(await observe(), before, 'adapter loss/reconnect cannot change native output state');
    checks.push({ check: 'native-stdio-disconnect-reconnect-retains-native-owner-without-output-change', passed: true,
      firstSidecarPid, secondSidecarPid: mcp.processId, authority, held, afterReconnect, before });

    // Real backend-selected TTL is 60 seconds. Do not substitute a fake clock
    // or send renewals while waiting. R0 queries conservatively hide expiry.
    const observations = [];
    let expired;
    while (performance.now() - acquireStarted < 75000) {
      const elapsedMs = performance.now() - acquireStarted;
      const view = await leases();
      observations.push({ elapsedMs, statuses: view.statuses });
      if (view.statuses.length === 1 && view.statuses[0].status === 'unavailable') {
        expired = view; break;
      }
      assert.deepEqual(view.statuses, held.statuses, 'the standalone lease must not be renewed implicitly');
      await pause(5000);
    }
    assert.ok(expired, 'native lease must expire without a mutation or synthetic clock');
    assert.ok(observations.at(-1).elapsedMs >= 59000, 'production TTL must not be shortened by the harness');
    assert.deepEqual(await observe(), before, 'authority expiry alone cannot change native output state');
    checks.push({ check: 'native-monotonic-lease-expiry-hides-expired-authority-without-output-change', passed: true,
      observations, expired, retryableReadOverloads: readOverloads });
    const renew = await output('syndocal.output.lease.renew.v2', { kind: 'renew_lease', lease: authority }, false);
    assert.equal(renew.response.type, 'rejected');
    assert.equal(renew.response.rejection.error, 'forbidden');
    assert.deepEqual(await send(renew.params, renew.id), renew.receipt, 'terminal failed renewal must replay without executing twice');
    const orphaned = await leases();
    assert.deepEqual(orphaned.statuses, [{ status: 'held_orphaned',
      authority: { ...authority, generation: authority.generation + 1 }, resources: ['lighting'] }]);
    const stale = await output('syndocal.output.lease.renew.v2', { kind: 'renew_lease', lease: authority }, false);
    assert.equal(stale.response.type, 'rejected');
    assert.equal(stale.response.rejection.error, 'forbidden');
    assert.deepEqual((await leases()).statuses, orphaned.statuses);
    checks.push({ check: 'native-expired-lease-rejects-renew-and-cached-generation-with-terminal-replay', passed: true,
      renewalRejection: renew.response, staleGenerationRejection: stale.response, orphaned });

    const recovered = await output('syndocal.output.lease.recover.v2', { kind: 'recover_lease', lease: orphaned.statuses[0].authority });
    assert.equal(recovered.response.type, 'receipt', JSON.stringify(recovered.response));
    const current = recovered.response.receipt.lease_result.authority;
    assert.equal(current.generation, authority.generation + 2);
    assert.deepEqual((await leases()).statuses, [{ status: 'held_active', authority: current, resources: ['lighting'] }]);
    assert.deepEqual(await observe(), before, 'explicit lease recovery does not Arm output');
    const relinquished = await output('syndocal.output.lease.relinquish.v2', { kind: 'relinquish_output_lease', lease: current });
    assert.equal(relinquished.response.type, 'receipt', JSON.stringify(relinquished.response));
    assert.deepEqual((await leases()).statuses, [{ status: 'unavailable' }]);
    const after = await observe();
    assert.deepEqual(after, before);
    checks.push({ check: 'native-expired-lease-explicit-recovery-relinquish-keeps-output-blocked', passed: true,
      recovered: recovered.response, relinquished: relinquished.response, after });
  } finally { await mcp?.close(); }
}
