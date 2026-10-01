import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { createHash, randomUUID } from 'node:crypto';
import { openNativeStdioSession } from './native-stdio-session.mjs';

// QA-only profile, empty project: leases change backend authority, never output.
export async function nativeExternalHighRisk(backend, options, checks) {
  const directory = await fs.mkdtemp(path.join(os.tmpdir(), 'syndocal-mcp-high-risk-'));
  let numericRequestId = Date.now();
  const outputIntents = new Map();
  const readOverloads = [];
  let mcp;
  const grant = async (capability, operationId) => backend.invoke('agent_authority_grant_v1', {
    principalId: options.principalId, principalIncarnation: options.principalIncarnation,
    grant: { adapter: 'external_mcp', capability, operation_id: operationId, project_id: null },
  });
  const send = async (method, params, id = randomUUID()) => {
    const name = method === 'diagnostics.export' ? 'syndocal_export_diagnostics' : 'syndocal_execute_control_plane';
    let receipt = await mcp.call(name, { requestId: id, ...params });
    const until = Date.now() + 15000;
    while (receipt.status === 'pending' && Date.now() < until) {
      await new Promise(resolve => setTimeout(resolve, 100));
      receipt = await mcp.call('syndocal_get_request_status', { requestId: id });
    }
    return receipt;
  };
  const query = async operationId => {
    for (let attempt = 0; attempt < 3; attempt++) {
      // This is a new read intent only after a terminal, explicitly retryable
      // lock-contention rejection. Never retry pending/unknown or a mutation.
      const receipt = await send('control_plane.execute', { operationId, request: {} });
      assert.equal(receipt.status, 'completed');
      if (receipt.result.ok === false && receipt.result.error?.code === 'request_rejected'
        && /QueryError \{ code: Overloaded,/.test(receipt.result.error.message)
        && /retryable: true/.test(receipt.result.error.message) && attempt < 2) {
        readOverloads.push({ operationId, requestId: receipt.requestId, error: receipt.result.error });
        await new Promise(resolve => setTimeout(resolve, 100));
        continue;
      }
      assert.equal(receipt.result.ok, true, JSON.stringify(receipt.result));
      assert.equal(receipt.result.operation_id, operationId);
      return receipt.result.result;
    }
    assert.fail('Canonical read attempt bound');
  };
  const queryLeases = () => query('syndocal.output.lease.authority.query.v1');
  const output = async (operationId, action, requestId) => {
    const { fence } = await query('syndocal.query.output.control.authority.v1');
    const id = randomUUID();
    const params = { operationId, request: {
      request: { operation_id: operationId, request_id: requestId, expected_fence: fence, action },
    } };
    outputIntents.set(id, params);
    const receipt = await send('control_plane.execute', params, id);
    if (receipt.status === 'completed') assert.ok(receipt.result.result, JSON.stringify(receipt.result));
    return receipt;
  };
  try {
    mcp = await openNativeStdioSession(options);
    const ungrantedQuery = await send('control_plane.execute', { operationId: 'syndocal.output.lease.authority.query.v1', request: {} });
    assert.equal(ungrantedQuery.status, 'rejected');
    assert.equal(ungrantedQuery.error, 'agent_missing_grant');
    await grant('read', 'syndocal.output.lease.authority.query.v1');
    await grant('read', 'syndocal.query.output.control.authority.v1');
    assert.deepEqual((await queryLeases()).statuses, [{ status: 'unavailable' }]);
    checks.push({ check: 'external-lease-authority-read-exact-grant-safe-mode-empty', passed: true, retryableReadOverloads: readOverloads });
    const destination = path.join(directory, 'diagnostics.zip');
    const denied = await send('diagnostics.export', { destination });
    assert.equal(denied.status, 'rejected');
    assert.equal(denied.error, 'agent_safe_mode_denied');
    await backend.invoke('agent_authority_promote_v1', {
      principalId: options.principalId, principalIncarnation: options.principalIncarnation,
    });
    const missing = await send('diagnostics.export', { destination });
    assert.equal(missing.status, 'rejected');
    assert.equal(missing.error, 'agent_missing_grant');
    await grant('file', 'syndocal.diagnostics.export.v1');
    const exportId = randomUUID();
    const exported = await send('diagnostics.export', { destination }, exportId);
    assert.equal(exported.status, 'completed');
    assert.equal(exported.result.ok, true);
    const bytes = await fs.readFile(destination);
    assert.equal(exported.result.sha256, createHash('sha256').update(bytes).digest('hex'));
    assert.equal(exported.result.bytes, bytes.length);
    assert.deepEqual(await send('diagnostics.export', { destination }, exportId), exported);
    const conflict = await send('diagnostics.export', { destination: path.join(directory, 'replacement.zip') }, exportId);
    assert.equal(conflict.status, 'rejected');
    assert.equal(conflict.error, 'request_conflict');
    const overwrite = await send('diagnostics.export', { destination });
    assert.equal(overwrite.status, 'completed');
    assert.equal(overwrite.result.ok, false);
    assert.match(overwrite.result.error.message, /already exists/);
    assert.ok((await fs.readFile(destination)).equals(bytes));
    checks.push({ check: 'external-r5-no-approval-export-exact-grant-safe-mode-no-overwrite-replay', passed: true, sha256: exported.result.sha256, bytes: bytes.length });

    const missingOutput = await output('syndocal.output.lease.acquire.v2', { kind: 'acquire_lease', role: 'lighting' }, numericRequestId++);
    assert.equal(missingOutput.status, 'rejected');
    assert.equal(missingOutput.error, 'agent_missing_grant');
    for (const operationId of ['syndocal.output.lease.acquire.v2', 'syndocal.output.lease.renew.v2', 'syndocal.output.lease.force_transfer.v2', 'syndocal.output.lease.relinquish.v2']) await grant('output', operationId);
    // A lease changes authority only. Do not Arm or activate physical output.
    const acquired = await output('syndocal.output.lease.acquire.v2', { kind: 'acquire_lease', role: 'lighting' }, numericRequestId++);
    assert.equal(acquired.status, 'completed');
    assert.equal(acquired.result.result.type, 'receipt', JSON.stringify(acquired.result));
    const lease = acquired.result.result.receipt.lease_result.authority;
    const renewed = await output('syndocal.output.lease.renew.v2', { kind: 'renew_lease', lease }, numericRequestId++);
    assert.equal(renewed.result.result.type, 'receipt', JSON.stringify(renewed.result));
    const active = renewed.result.result.receipt.lease_result.authority;
    assert.equal(active.generation, lease.generation + 1);
    const selfTransfer = await output('syndocal.output.lease.force_transfer.v2', { kind: 'force_transfer_lease', lease: active }, numericRequestId++);
    assert.equal(selfTransfer.result.result.type, 'rejected');
    assert.equal(selfTransfer.result.result.rejection.error, 'forbidden');
    const beforeRotation = await queryLeases();
    assert.ok(beforeRotation.statuses.some(status => status.status === 'held_active'
      && status.authority.lease_id === active.lease_id && status.authority.generation === active.generation));
    checks.push({ check: 'external-r4-acquire-renew-same-owner-transfer-rejected', passed: true });

    // Use the existing owner-replacement lifecycle. Retirement orphans the old
    // lease and advances its generation once; the new owner can transfer it.
    await backend.invoke('register_project_transaction_owner', { ownerId: `high-risk-${randomUUID()}` });
    const afterRotation = await queryLeases();
    assert.deepEqual(afterRotation.statuses, [{ status: 'unavailable' }]);
    const staleTransfer = await output('syndocal.output.lease.force_transfer.v2', { kind: 'force_transfer_lease', lease: active }, numericRequestId++);
    assert.equal(staleTransfer.result.result.type, 'rejected');
    assert.equal(staleTransfer.result.result.rejection.error, 'forbidden');
    const orphaned = { ...active, generation: active.generation + 1 };
    const transferred = await output('syndocal.output.lease.force_transfer.v2', { kind: 'force_transfer_lease', lease: orphaned }, numericRequestId++);
    assert.equal(transferred.status, 'completed');
    assert.equal(transferred.result.result.type, 'receipt', JSON.stringify(transferred.result));
    const current = transferred.result.result.receipt.lease_result.authority;
    assert.equal(current.generation, orphaned.generation + 1);
    assert.deepEqual(await send('control_plane.execute', outputIntents.get(transferred.requestId), transferred.requestId), transferred);
    const relinquished = await output('syndocal.output.lease.relinquish.v2', { kind: 'relinquish_output_lease', lease: current }, numericRequestId++);
    assert.equal(relinquished.result.result.type, 'receipt', JSON.stringify(relinquished.result));
    const ownership = await backend.invoke('get_output_ownership_status');
    assert.equal(ownership.lighting_allowed, false);
    assert.equal(ownership.video_allowed, false);
    assert.deepEqual((await queryLeases()).statuses, [{ status: 'unavailable' }]);
    checks.push({ check: 'external-r4-no-dialog-owner-transfer-stale-generation-rejected-relinquish-without-output', passed: true, ownership });
    const staleRenderer = await backend.evaluate(`window.__TAURI_INTERNALS__.invoke('agent_bridge_execute_native_v1', {rendererGeneration:0,requestId:${JSON.stringify(exportId)}}).then(() => ({ok:true}), error => ({error:String(error)}))`);
    assert.deepEqual(staleRenderer, { error: 'stale_renderer' });
    checks.push({ check: 'native-execution-stale-renderer-rejected', passed: true });
  } finally {
    try { await mcp?.close(); }
    finally {
      assert.equal(path.dirname(path.resolve(directory)), path.resolve(os.tmpdir()));
      assert.ok(path.basename(directory).startsWith('syndocal-mcp-high-risk-'));
      await fs.rm(directory, { recursive: true, force: true });
    }
  }
}
