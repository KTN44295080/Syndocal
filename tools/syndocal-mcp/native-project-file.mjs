import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { createHash, randomUUID } from 'node:crypto';
import { openNativeStdioSession } from './native-stdio-session.mjs';
import { nativeTapBpm } from './native-tap-bpm.mjs';

// Real authenticated stdio MCP against the process-verified private native profile.
// All artifacts and targets are individually owned by this run; no dialogs/DOM actions.
export async function nativeProjectFile(backend, options, checks) {
  const directory = await fs.mkdtemp(path.join(os.tmpdir(), 'syndocal-typed-file-'));
  const ids = { save: 'syndocal.project.save.v1', saveAs: 'syndocal.project.save_as.v1',
    template: 'syndocal.project.template.save.v1', authority: 'syndocal.query.project.file.authority.v1',
    status: 'syndocal.query.project.file.status.v1', ack: 'syndocal.project.file.acknowledge.v1' };
  const digest = bytes => createHash('sha256').update(bytes).digest('hex');
  const checkpoint = () => backend.invoke('get_project_checkpoint', { midiMappings: [], oscMappings: [], dmxMappings: [] });
  const state = async () => {
    const project = await checkpoint(), bundle = await backend.invoke('get_project_authority_bundle');
    // The bundle embeds a live observational snapshot (beat/frame/latency clocks).
    // Compare the complete persistence image separately and every authority field;
    // ordinary worker ticks are not authored file mutations.
    delete bundle.snapshot;
    return { project, bundle, output: await backend.invoke('get_output_ownership_status') };
  };
  const grant = (capability, operationId) => backend.invoke('agent_authority_grant_v1', {
    principalId: options.principalId, principalIncarnation: options.principalIncarnation,
    grant: { adapter: 'external_mcp', capability, operation_id: operationId, project_id: null } });
  let mcp;
  try {
    mcp = await openNativeStdioSession(options);
    const send = async (operationId, request, id = randomUUID()) => {
      let receipt = await mcp.call('syndocal_execute_control_plane', { requestId: id, operationId, request: { request } });
      const until = Date.now() + 15000;
      while (receipt.status === 'pending' && Date.now() < until) {
        await new Promise(resolve => setTimeout(resolve, 100));
        receipt = await mcp.call('syndocal_get_request_status', { requestId: id });
      }
      return receipt;
    };
    const success = async (operationId, request) => {
      const receipt = await send(operationId, request);
      assert.equal(receipt.status, 'completed', JSON.stringify(receipt));
      assert.equal(receipt.result.ok, true, JSON.stringify(receipt.result));
      return receipt.result.result;
    };
    const authority = (operation_id, destination) => success(ids.authority, { schema_version: 1, operation_id, destination });
    const request = value => ({ schema_version: 1, operation_id: value.operation_id, request_id: value.next_request_id,
      expected_fence: value.fence, expected_path_generation: value.path_generation,
      expected_disposition_generation: value.disposition_generation, destination: value.destination,
      expected_target_sha256: value.target_sha256 });
    const prepare = async (operation_id, destination) => request({ ...await authority(operation_id, destination), operation_id });
    const failed = async (operationId, value, message) => {
      const before = await state();
      const receipt = await send(operationId, value);
      assert.equal(receipt.status, 'completed', JSON.stringify(receipt));
      assert.equal(receipt.result.ok, false, JSON.stringify(receipt.result));
      assert.match(receipt.result.error.message, message);
      assert.deepEqual(await state(), before, 'Rejected file operation preserves full project/authority and closed output gates');
      return receipt;
    };
    const registry = await backend.invoke('get_control_plane_canonical_registry');
    for (const operationId of [ids.save, ids.saveAs, ids.template, ids.ack]) {
      const operation = registry.canonical_operations.find(value => value.operation_id === operationId);
      assert.equal(operation.risk, 'r5'); assert.equal(operation.adapter_policy, 'local_window_project_publication');
      assert.equal(operation.audit, 'immutable'); assert.equal(operation.receipt_policy, 'exact_terminal_receipt');
      assert.equal(operation.derived_adapters.length, 1);
    }
    checks.push({ check: 'external-file-four-r5-operations-and-two-r0-query-policies', passed: true });
    const destination = path.join(directory, '日本語.sdc');
    const missingRead = await send(ids.authority, { schema_version: 1, operation_id: ids.saveAs, destination });
    assert.equal(missingRead.status, 'rejected'); assert.equal(missingRead.error, 'agent_missing_grant');
    await grant('read', ids.authority); await grant('read', ids.status);
    const value = await prepare(ids.saveAs, destination);
    const safeMode = await send(ids.saveAs, value);
    assert.equal(safeMode.status, 'rejected'); assert.equal(safeMode.error, 'agent_safe_mode_denied');
    await backend.invoke('agent_authority_promote_v1', { principalId: options.principalId, principalIncarnation: options.principalIncarnation });
    const missingFile = await send(ids.saveAs, value);
    assert.equal(missingFile.status, 'rejected'); assert.equal(missingFile.error, 'agent_missing_grant');
    for (const operationId of [ids.save, ids.saveAs, ids.template, ids.ack]) await grant('file', operationId);
    checks.push({ check: 'external-file-exact-read-file-grants-and-safe-mode-without-human-approval', passed: true });
    for (const field of ['owner_id', 'origin_id', 'principal', 'skip_confirmation']) {
      await failed(ids.saveAs, { ...value, [field]: true }, /project_file_request_invalid/);
    }
    await failed(ids.saveAs, { ...value, destination: 'relative.sdc' }, /absolute/);
    await failed(ids.saveAs, { ...value, expected_path_generation: value.expected_path_generation + 1 }, /stale_fence/);
    await failed(ids.saveAs, { ...value, expected_fence: { ...value.expected_fence,
      project_publication_generation: value.expected_fence.project_publication_generation + 1 } }, /session_fence|stale_fence/);
    await failed(ids.saveAs, { ...value, request_id: Number.MAX_SAFE_INTEGER + 1 }, /invalid project file request/);
    checks.push({ check: 'external-file-rejects-forged-identity-invalid-target-and-unsafe-or-stale-fences', passed: true });

    await fs.writeFile(destination, 'racing creator', { flag: 'wx' });
    await failed(ids.saveAs, value, /target_changed/);
    assert.equal(await fs.readFile(destination, 'utf8'), 'racing creator');
    await fs.rm(destination);
    checks.push({ check: 'external-file-expected-absent-target-appeared-is-never-overwritten', passed: true });

    const before = await state(); const start = performance.now();
    const saved = await success(ids.saveAs, value); const elapsedMs = performance.now() - start;
    assert.equal(saved.phase, 'succeeded'); assert.equal(saved.target_path, value.destination);
    const bytes = await fs.readFile(value.destination); assert.equal(saved.artifact_sha256, digest(bytes));
    assert.deepEqual(JSON.parse(bytes), before.project, 'Saved bytes contain the complete intended project');
    const after = await state(); assert.deepEqual(after.project, before.project);
    assert.equal(after.bundle.current_project_path, value.destination);
    assert.equal(after.bundle.recovery_authority_serial, before.bundle.recovery_authority_serial + 1);
    assert.equal(after.output.lighting_allowed, false); assert.equal(after.output.video_allowed, false);
    assert.deepEqual(await success(ids.status, value), saved);
    assert.deepEqual(await success(ids.saveAs, value), saved);
    assert.deepEqual(await state(), after, 'Domain replay has no second serial/path/disposition effect');
    await failed(ids.saveAs, { ...value, destination: path.join(directory, 'another.sdc') }, /shape_conflict/);
    checks.push({ check: 'external-file-save-as-full-byte-roundtrip-durable-status-exact-replay-and-changed-shape',
      passed: true, receipt: saved, bytes: bytes.length, elapsedMs });
    const ack = await success(ids.ack, value); assert.equal(ack.phase, 'acknowledged');
    assert.deepEqual(await success(ids.ack, value), ack); assert.deepEqual(await success(ids.status, value), ack);
    assert.equal((await authority(ids.saveAs, destination)).next_request_id, 2);
    checks.push({ check: 'external-file-durable-ack-tombstone-exact-reack-and-sequence-advance', passed: true });

    await nativeTapBpm(backend, checks);
    const dirtyProject = await checkpoint();
    assert.ok(dirtyProject.snapshot.clock.bpm > 70 && dirtyProject.snapshot.clock.bpm < 90);
    const regular = await prepare(ids.save, destination);
    assert.equal(regular.expected_target_sha256, digest(bytes));
    const changedBytes = Buffer.from('external changed bytes'); await fs.writeFile(destination, changedBytes);
    await failed(ids.save, regular, /target_changed/); assert.deepEqual(await fs.readFile(destination), changedBytes);
    await fs.writeFile(destination, bytes);
    const regularSaved = await success(ids.save, regular); assert.equal(regularSaved.phase, 'succeeded');
    assert.equal(regularSaved.artifact_sha256, digest(await fs.readFile(destination)));
    assert.deepEqual(JSON.parse(await fs.readFile(destination)), dirtyProject, 'Save persists the complete newly authored Tap project');
    assert.notEqual(regularSaved.artifact_sha256, digest(bytes), 'Save replaces the previous 120 BPM bytes with the new tempo');
    await success(ids.ack, regular);
    checks.push({ check: 'external-file-current-path-save-existing-hash-fence-and-success', passed: true, receipt: regularSaved });

    const beforeTemplate = await state();
    const template = await prepare(ids.template, path.join(directory, '日本語.sdctemplate'));
    const templateSaved = await success(ids.template, template); assert.equal(templateSaved.phase, 'succeeded');
    const templateBytes = await fs.readFile(template.destination);
    assert.equal(templateSaved.artifact_sha256, digest(templateBytes));
    const exported = JSON.parse(templateBytes);
    const combinedTemplate = { ...exported.project };
    for (const key of ['midi_mappings', 'osc_mappings', 'dmx_mappings', 'dj_track_triggers']) {
      assert.deepEqual(exported[key], beforeTemplate.project[key] ?? [], `Exact ${key} export`);
      if (exported[key].length) combinedTemplate[key] = exported[key];
    }
    // Native ClockSnapshot.bpm is f32. Project JSON passes through serde Value
    // (f64 JSON number); direct template serialization uses shortest f32 text.
    // Both must round-trip to the identical native value. Every other leaf
    // remains under the exact full-project comparison.
    const expectedTemplate = structuredClone(beforeTemplate.project);
    combinedTemplate.snapshot.clock.bpm = Math.fround(combinedTemplate.snapshot.clock.bpm);
    expectedTemplate.snapshot.clock.bpm = Math.fround(expectedTemplate.snapshot.clock.bpm);
    assert.deepEqual(combinedTemplate, expectedTemplate, 'Template retains the complete native project and all mappings');
    assert.deepEqual(await state(), beforeTemplate, 'Template does not adopt its path or clear/change project disposition');
    await success(ids.ack, template);
    checks.push({ check: 'external-file-template-full-project-export-without-path-or-disposition-change', passed: true,
      receipt: templateSaved, bytes: templateBytes.length });

    const next = await prepare(ids.saveAs, path.join(directory, 'second.sdc'));
    assert.equal(next.request_id, 2);
    assert.equal((await success(ids.saveAs, next)).phase, 'succeeded');
    await success(ids.ack, next);
    checks.push({ check: 'external-file-second-save-as-after-ack-and-closed-output-gates', passed: true });
  } finally {
    await mcp?.close();
    assert.equal(path.dirname(directory), await fs.realpath(os.tmpdir()));
    assert.ok(path.basename(directory).startsWith('syndocal-typed-file-'));
    await fs.rm(directory, { recursive: true, force: true });
  }
}
