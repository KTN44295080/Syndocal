import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { createHash, randomUUID } from 'node:crypto';
import { openNativeStdioSession } from './native-stdio-session.mjs';

// Empty isolated QA project only. Actual typed R5 operations enter through a
// real authenticated MCP sidecar. No physical routes or native dialogs are used.
export async function nativeProjectReplacement(backend, options, checks) {
  const directory = await fs.mkdtemp(path.join(os.tmpdir(), 'syndocal-typed-replacement-'));
  const authorityId = 'syndocal.query.project.replacement.authority.v1';
  const newId = 'syndocal.project.new.v1', openId = 'syndocal.project.open.v1';
  const digest = bytes => createHash('sha256').update(bytes).digest('hex');
  let requestId = Date.now(), mcp;
  const grant = (capability, operationId) => backend.invoke('agent_authority_grant_v1', {
    principalId: options.principalId, principalIncarnation: options.principalIncarnation,
    grant: { adapter: 'external_mcp', capability, operation_id: operationId, project_id: null },
  });
  const send = async (operationId, request, id = randomUUID()) => {
    let receipt = await mcp.call('syndocal_execute_control_plane', { requestId: id, operationId, request });
    const deadline = Date.now() + 15000;
    while (receipt.status === 'pending' && Date.now() < deadline) {
      await new Promise(resolve => setTimeout(resolve, 100));
      receipt = await mcp.call('syndocal_get_request_status', { requestId: id });
    }
    return receipt;
  };
  const fence = async () => {
    const receipt = await send(authorityId, { request: {} });
    assert.equal(receipt.status, 'completed');
    assert.equal(receipt.result.ok, true, JSON.stringify(receipt.result));
    return receipt.result.result.fence;
  };
  const request = (expected, action) => ({ schema_version: 1,
    operation_id: action.kind === 'new' ? newId : openId, request_id: requestId++, expected_fence: expected, action });
  const execute = (value, id) => send(value.operation_id, { request: value }, id);
  const checkpoint = () => backend.invoke('get_project_checkpoint', { midiMappings: [], oscMappings: [], dmxMappings: [] });
  const state = async () => {
    const bundle = await backend.invoke('get_project_authority_bundle');
    return { authority: { epoch: bundle.project_epoch, revision: bundle.project_revision,
      hash: bundle.checkpoint_hash, generation: bundle.publication_generation, path: bundle.current_project_path },
      checkpoint: await checkpoint(), ownership: await backend.invoke('get_output_ownership_status') };
  };
  const rejected = async (value, code) => {
    const before = await state(), receipt = await execute(value);
    assert.equal(receipt.status, 'completed', JSON.stringify(receipt));
    assert.equal(receipt.result.ok, false, JSON.stringify(receipt.result));
    assert.deepEqual(receipt.result.result, { status: 'rejected', value: { request_id: value.request_id, code } });
    assert.deepEqual(await state(), before, 'Rejected New/Open preserves authority, complete authored image and output gates');
    checks.push({ check: `external-project-${value.action.kind}-${code}-${value.request_id}-preserves-complete-state`, passed: true });
  };
  try {
    mcp = await openNativeStdioSession(options);
    const registry = await backend.invoke('get_control_plane_canonical_registry');
    for (const id of [newId, openId]) {
      const operation = registry.canonical_operations.find(operation => operation.operation_id === id);
      assert.equal(operation.risk, 'r5');
      assert.equal(operation.adapter_policy, 'local_window_project_replacement');
      assert.equal(operation.audit, 'immutable');
      assert.equal(operation.receipt_policy, 'exact_terminal_receipt');
      assert.equal(operation.derived_adapters.length, 1);
      assert.equal(operation.derived_adapters[0].adapter, 'local_tauri_window');
    }
    checks.push({ check: 'external-project-new-open-canonical-r5-policy-exact-local-projection', passed: true });
    const missingRead = await send(authorityId, { request: {} });
    assert.equal(missingRead.status, 'rejected'); assert.equal(missingRead.error, 'agent_missing_grant');
    await grant('read', authorityId);
    const initial = await fence();
    const create = request(initial, { kind: 'new' });
    const safeMode = await execute(create);
    assert.equal(safeMode.status, 'rejected'); assert.equal(safeMode.error, 'agent_safe_mode_denied');
    await backend.invoke('agent_authority_promote_v1', { principalId: options.principalId,
      principalIncarnation: options.principalIncarnation });
    const missingFile = await execute(create);
    assert.equal(missingFile.status, 'rejected'); assert.equal(missingFile.error, 'agent_missing_grant');
    await grant('file', newId);
    const ungrantedOpen = await execute(request(initial, { kind: 'open', path: path.join(directory, 'absent.sdc'), expected_file_sha256: 'a'.repeat(64) }));
    assert.equal(ungrantedOpen.status, 'rejected'); assert.equal(ungrantedOpen.error, 'agent_missing_grant');
    await grant('file', openId);
    checks.push({ check: 'external-project-authority-read-and-new-open-exact-file-grants-safe-mode', passed: true });

    for (const field of ['owner_id', 'principal', 'skip_confirmation', 'confirmation_origin']) {
      const before = await state();
      const bad = { ...request(await fence(), { kind: 'new' }), [field]: true };
      const result = await execute(bad);
      assert.equal(result.status, 'completed'); assert.equal(result.result.ok, false);
      // The renderer has already handed execution to the native dispatcher.
      // Its conservative error envelope remains uncertain; prove the exact
      // native parse rejection and complete unchanged state separately.
      assert.equal(result.result.error.code, 'mutation_not_confirmed');
      assert.match(result.result.error.message, /agent_bridge_arguments_invalid/);
      assert.deepEqual(await state(), before);
    }
    checks.push({ check: 'external-project-new-rejects-forged-owner-principal-and-consent-origin', passed: true });
    for (const field of ['process_incarnation', 'session_incarnation', 'project_epoch', 'project_revision', 'project_publication_generation']) {
      const current = await fence(); current[field]++;
      await rejected(request(current, { kind: 'new' }), 'forbidden');
    }
    const badHash = await fence(); badHash.project_checkpoint_hash = '0'.repeat(64);
    await rejected(request(badHash, { kind: 'new' }), 'forbidden');

    const source = await checkpoint();
    const initialProject = structuredClone(source);
    source.snapshot.clock.bpm = 93;
    const file = path.join(directory, '日本語.sdc'), bytes = Buffer.from(JSON.stringify(source));
    await fs.writeFile(file, bytes, { flag: 'wx' });
    const openAction = { kind: 'open', path: file, expected_file_sha256: digest(bytes) };
    await rejected(request(await fence(), { ...openAction, expected_file_sha256: '0'.repeat(64) }), 'file_changed');
    await rejected(request(await fence(), { ...openAction, path: 'relative.sdc' }), 'invalid_request');
    const badFile = path.join(directory, 'invalid.sdc'), invalid = Buffer.from('{}');
    await fs.writeFile(badFile, invalid, { flag: 'wx' });
    await rejected(request(await fence(), { kind: 'open', path: badFile, expected_file_sha256: digest(invalid) }), 'invalid_project');

    const open = request(await fence(), openAction), openUuid = randomUUID();
    const opened = await execute(open, openUuid);
    assert.equal(opened.status, 'completed'); assert.equal(opened.result.ok, true, JSON.stringify(opened.result));
    assert.equal(opened.result.result.status, 'receipt');
    const afterOpen = await state();
    assert.equal(afterOpen.authority.epoch, open.expected_fence.project_epoch + 1);
    assert.equal(afterOpen.authority.revision, 0);
    assert.equal(afterOpen.authority.generation, open.expected_fence.project_publication_generation + 1);
    assert.equal(afterOpen.authority.path, file);
    assert.equal(afterOpen.checkpoint.snapshot.clock.bpm, 93);
    assert.deepEqual(afterOpen.checkpoint, source, 'Open publishes the complete intended authored image');
    assert.equal(afterOpen.ownership.lighting_allowed, false); assert.equal(afterOpen.ownership.video_allowed, false);
    assert.deepEqual(await fs.readFile(file), bytes);
    checks.push({ check: 'external-project-open-applies-exact-file-with-no-individual-confirmation', passed: true,
      receipt: opened.result.result, sourceSha256: digest(bytes), bytes: bytes.length });
    assert.deepEqual(await execute(open, openUuid), opened, 'Broker UUID replay is exact');
    assert.deepEqual((await execute(open)).result, opened.result, 'Domain replay with another broker UUID is exact');
    assert.deepEqual(await state(), afterOpen, 'Replays do not replace the project again');
    const changed = { ...open, expected_fence: await fence() };
    await rejected(changed, 'conflict');
    checks.push({ check: 'external-project-open-broker-and-domain-replay-no-replacement-or-changed-shape', passed: true });

    const fresh = request(await fence(), { kind: 'new' });
    const created = await execute(fresh);
    assert.equal(created.status, 'completed'); assert.equal(created.result.ok, true, JSON.stringify(created.result));
    const afterNew = await state();
    const newCheck = { check: 'external-project-new-applies-default-project-with-no-individual-confirmation', passed: false,
      receipt: created.result.result, before: afterOpen.authority, after: afterNew.authority, checkpoint: afterNew.checkpoint };
    checks.push(newCheck);
    assert.equal(afterNew.authority.epoch, fresh.expected_fence.project_epoch + 1);
    assert.equal(afterNew.authority.generation, fresh.expected_fence.project_publication_generation + 1);
    assert.equal(afterNew.authority.revision, 0); assert.equal(afterNew.authority.path, null);
    assert.equal(afterNew.checkpoint.snapshot.clock.bpm, 120);
    assert.deepEqual(afterNew.checkpoint, initialProject, 'New restores the complete canonical default project');
    assert.equal(afterNew.ownership.lighting_allowed, false); assert.equal(afterNew.ownership.video_allowed, false);
    assert.deepEqual(await fs.readFile(file), bytes);
    newCheck.passed = true;

    // Author the expected migration from the already-observed canonical
    // default: the only removed legacy fields are the Timeline bank and Main
    // composition. Never bless the candidate's readback as the expectation.
    const expectedLegacy = structuredClone(initialProject);
    expectedLegacy.snapshot.clock.bpm = 91;
    const legacySource = structuredClone(expectedLegacy);
    legacySource.snapshot.timeline_bank = [];
    legacySource.snapshot.video.compositions = [];
    const legacyFile = path.join(directory, '旧形式.sdc');
    const legacyBytes = Buffer.from(JSON.stringify(legacySource));
    await fs.writeFile(legacyFile, legacyBytes, { flag: 'wx' });
    const legacyRequest = request(await fence(), { kind: 'open', path: legacyFile, expected_file_sha256: digest(legacyBytes) });
    const started = performance.now();
    const legacyOpened = await execute(legacyRequest);
    const elapsedMs = performance.now() - started;
    assert.equal(legacyOpened.status, 'completed');
    assert.equal(legacyOpened.result.ok, true, JSON.stringify(legacyOpened.result));
    const afterLegacy = await state();
    const terminal = legacyOpened.result.result.value.outcome.authority;
    assert.equal(afterLegacy.authority.epoch, legacyRequest.expected_fence.project_epoch + 1);
    assert.equal(afterLegacy.authority.revision, 0);
    assert.equal(afterLegacy.authority.generation, legacyRequest.expected_fence.project_publication_generation + 1);
    assert.equal(afterLegacy.authority.hash, terminal.checkpoint_hash);
    assert.equal(afterLegacy.authority.path, legacyFile);
    assert.deepEqual(afterLegacy.checkpoint, expectedLegacy, 'Legacy Open publishes the independently specified complete canonical image');
    assert.equal(afterLegacy.ownership.lighting_allowed, false);
    assert.equal(afterLegacy.ownership.video_allowed, false);
    assert.deepEqual(await fs.readFile(legacyFile), legacyBytes, 'Migration never rewrites the source');
    checks.push({ check: 'external-project-legacy-open-canonical-receipt-matches-complete-state', passed: true,
      receipt: legacyOpened.result.result, sourceSha256: digest(legacyBytes), bytes: legacyBytes.length, elapsedMs });
    assert.deepEqual((await execute(legacyRequest)).result, legacyOpened.result);
    assert.deepEqual(await state(), afterLegacy, 'Legacy replay cannot introduce an extra mutation');
    checks.push({ check: 'external-project-legacy-open-exact-replay-preserves-authority-and-image', passed: true });
  } finally {
    if (mcp) await mcp.close();
    assert.equal(path.dirname(directory), await fs.realpath(os.tmpdir()));
    assert.ok(path.basename(directory).startsWith('syndocal-typed-replacement-'));
    await fs.rm(directory, { recursive: true, force: true });
  }
}
