import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { createHash, randomUUID } from 'node:crypto';
import { openNativeStdioSession } from './native-stdio-session.mjs';
import { nativeAuthoredControlProject } from './native-authored-control-project.mjs';
import { verifyFixtureAutosave } from './native-fixture-autosave.mjs';

// Real MCP restore of an independently checked authored graph and all four
// mapping families. Only private files and this run's managed backup are owned.
export async function nativeProjectBackupRestore(backend, options, checks) {
  const directory = await fs.mkdtemp(path.join(os.tmpdir(), 'syndocal-backup-restore-'));
  const managed = path.join(process.env.LOCALAPPDATA, options.profileId, 'project-backups');
  const ids = { authority: 'syndocal.query.project.replacement.authority.v1',
    open: 'syndocal.project.open.v1', new: 'syndocal.project.new.v1', restore: 'syndocal.project.backup.restore.v1',
    backupAuthority: 'syndocal.query.project.backup.authority.v1', create: 'syndocal.project.backup.create.v1',
    inspect: 'syndocal.query.project.backup.inspect.v1', ack: 'syndocal.project.file.acknowledge.v1' };
  const digest = bytes => createHash('sha256').update(bytes).digest('hex');
  const baseline = (await fs.readdir(managed)).sort();
  const baselineHashes = new Map(await Promise.all(baseline.map(async name => [name, digest(await fs.readFile(path.join(managed, name)))])));
  const baselineSummaries = await backend.invoke('list_project_backups');
  assert.ok(baselineSummaries.length < 10, 'Do not enter retention against unowned backups');
  const startedAt = Date.now();
  let mcp, backupRequest, saved, backupBytes, expected, source, acknowledged = false, sequence = Date.now();
  const fixtureAutosaves = [];
  const grant = (capability, operationId) => backend.invoke('agent_authority_grant_v1', {
    principalId: options.principalId, principalIncarnation: options.principalIncarnation,
    grant: { adapter: 'external_mcp', capability, operation_id: operationId, project_id: null } });
  const send = async (operationId, request) => {
    const requestId = randomUUID();
    let result = await mcp.call('syndocal_execute_control_plane', { requestId, operationId, request: { request } });
    const deadline = Date.now() + 15000;
    while (result.status === 'pending' && Date.now() < deadline) {
      await new Promise(resolve => setTimeout(resolve, 100));
      result = await mcp.call('syndocal_get_request_status', { requestId });
    }
    return result;
  };
  const success = async (operationId, request) => {
    const result = await send(operationId, request);
    assert.equal(result.status, 'completed', JSON.stringify(result));
    assert.equal(result.result.ok, true, JSON.stringify(result.result));
    return result.result.result;
  };
  const fence = async () => (await success(ids.authority, {})).fence;
  const request = async action => ({ schema_version: 1, operation_id: ids[action.kind === 'restore_backup' ? 'restore' : action.kind],
    request_id: sequence++, expected_fence: await fence(), action });
  const checkpoint = async observed => {
    const bundle = observed ?? await backend.invoke('get_project_authority_bundle');
    return backend.invoke('get_project_checkpoint', { midiMappings: bundle.midi_mappings,
      oscMappings: bundle.osc_mappings, dmxMappings: bundle.dmx_mappings });
  };
  const state = async () => {
    const bundle = await backend.invoke('get_project_authority_bundle');
    delete bundle.snapshot;
    return { project: await checkpoint(bundle), bundle, output: await backend.invoke('get_output_ownership_status') };
  };
  const rejected = async (value, code) => {
    const before = await state(), result = await send(ids.restore, value);
    assert.equal(result.status, 'completed', JSON.stringify(result));
    assert.equal(result.result.ok, false, JSON.stringify(result));
    assert.deepEqual(result.result.result, { status: 'rejected', value: { request_id: value.request_id, code } });
    assert.deepEqual(await state(), before, 'Rejected restore preserves complete project, authority and output gates');
  };
  try {
    mcp = await openNativeStdioSession(options);
    for (const id of [ids.authority, ids.backupAuthority, ids.inspect]) await grant('read', id);
    for (const id of [ids.open, ids.new]) await grant('file', id);
    const load = async filename => {
      const value = await request({ kind: 'open', path: filename, expected_file_sha256: digest(await fs.readFile(filename)) });
      const result = await success(ids.open, value);
      assert.equal(result.status, 'receipt');
      return { ok: true };
    };
    await nativeAuthoredControlProject({ directory, load, checks,
      observe: async () => {
        const current = await state();
        return { checkpoint: current.project, ownership: current.output,
          token: { epoch: current.bundle.project_epoch, revision: current.bundle.project_revision, hash: current.bundle.checkpoint_hash } };
      },
      observeTransport: async () => {
        const { snapshot } = await backend.invoke('get_snapshot');
        return { timelineId: snapshot.timeline.id, playing: snapshot.timeline.playing, positionMs: snapshot.timeline.position_ms };
      } });
    expected = JSON.parse(await fs.readFile(path.join(directory, 'authored-controls-canonical.sdc')));
    const common = { action: 'LightingMaster', fixture_id: null, attribute: null, group_id: null,
      cue_id: null, layer_id: null, output_id: null, video_param: null, cue_point_index: null, duration_ms: null, low: 0, high: 1 };
    expected.midi_mappings = [{ ...common, channel: 0, message: 'ControlChange', number: 7 }];
    expected.osc_mappings = [{ ...common, address: '/restore/master' }];
    expected.dmx_mappings = [{ ...common, universe: 0, channel: 12 }];
    expected.dj_track_triggers = [{ id: 'restore-dj', selector: { contentId: 'restore-proof', title: null, artist: null,
      titleContains: null, fallbackDeck: null }, timelineId: expected.snapshot.timeline.id, retrigger: 'once_per_play_session' }];
    source = path.join(directory, '全マッピング.sdc');
    const sourceBytes = Buffer.from(JSON.stringify(expected));
    await fs.writeFile(source, sourceBytes, { flag: 'wx' });
    await load(source);
    assert.deepEqual(await checkpoint(), expected, 'All authored data and four independently specified mapping families loaded');
    const backupAuthority = await success(ids.backupAuthority, { schema_version: 1 });
    backupRequest = { schema_version: 1, operation_id: ids.create, request_id: backupAuthority.next_request_id,
      expected_fence: backupAuthority.fence, expected_path_generation: backupAuthority.path_generation,
      expected_disposition_generation: backupAuthority.disposition_generation,
      destination: backupAuthority.destination, expected_target_sha256: null };
    saved = await success(ids.create, backupRequest);
    backupBytes = await fs.readFile(saved.target_path);
    assert.equal(saved.artifact_sha256, digest(backupBytes));
    await success(ids.ack, backupRequest); acknowledged = true;
    const inspected = await success(ids.inspect, { schema_version: 1, backup_id: saved.backup.id });
    assert.equal(inspected.artifact_sha256, digest(backupBytes));
    assert.equal(inspected.restore_source_path, source);
    const action = { kind: 'restore_backup', backup_id: saved.backup.id,
      expected_file_sha256: inspected.artifact_sha256, expected_source_path: inspected.restore_source_path };
    const created = await success(ids.new, await request({ kind: 'new' }));
    assert.equal(created.status, 'receipt');
    const empty = await state();
    for (const field of ['midi_mappings', 'osc_mappings', 'dmx_mappings', 'dj_track_triggers']) assert.deepEqual(empty.bundle[field], []);
    const original = await request(action), denied = await send(ids.restore, original);
    assert.equal(denied.status, 'rejected'); assert.equal(denied.error, 'agent_missing_grant');
    assert.deepEqual(await state(), empty);
    await grant('file', ids.restore);
    checks.push({ check: 'external-backup-restore-exact-file-grant-no-human-confirmation', passed: true });
    for (const field of ['owner_id', 'principal', 'skip_confirmation', 'confirmation_origin']) {
      const before = await state(), forged = { ...await request(action), [field]: true };
      const result = await send(ids.restore, forged);
      assert.equal(result.status, 'completed'); assert.equal(result.result.ok, false);
      assert.match(result.result.error.message, /agent_bridge_arguments_invalid/);
      assert.deepEqual(await state(), before);
    }
    await rejected(await request({ ...action, expected_file_sha256: '0'.repeat(64) }), 'file_changed');
    await rejected(await request({ ...action, expected_source_path: null }), 'file_changed');
    await rejected(await request({ ...action, backup_id: Number.MAX_SAFE_INTEGER }), 'invalid_project');
    for (const field of ['process_incarnation', 'session_incarnation', 'project_epoch', 'project_revision', 'project_publication_generation']) {
      const value = await request(action); value.expected_fence[field]++;
      await rejected(value, 'forbidden');
    }
    const staleHash = await request(action); staleHash.expected_fence.project_checkpoint_hash = '0'.repeat(64);
    await rejected(staleHash, 'forbidden');
    try {
      await fs.writeFile(saved.target_path, '{}');
      await rejected(await request(action), 'invalid_project');
    } finally { await fs.writeFile(saved.target_path, backupBytes); }
    checks.push({ check: 'external-backup-restore-forged-identity-changed-bytes-source-missing-malformed-and-all-stale-fences-preserve-complete-state', passed: true });
    const value = await request(action), start = performance.now();
    const receipt = await success(ids.restore, value), elapsedMs = performance.now() - start;
    assert.equal(receipt.status, 'receipt');
    const after = await state(), terminal = receipt.value.outcome.authority;
    assert.deepEqual(after.project, expected, 'Restore returns the independent complete authored graph and all four mappings');
    assert.equal(after.bundle.project_epoch, value.expected_fence.project_epoch + 1);
    assert.equal(after.bundle.project_revision, 0);
    assert.equal(after.bundle.publication_generation, value.expected_fence.project_publication_generation + 1);
    assert.equal(after.bundle.checkpoint_hash, terminal.checkpoint_hash);
    assert.equal(after.bundle.current_project_path, source);
    assert.equal(after.bundle.authority_disposition, 'unsaved_replacement');
    assert.equal(after.output.lighting_allowed, false); assert.equal(after.output.video_allowed, false);
    assert.deepEqual(await fs.readFile(saved.target_path), backupBytes);
    assert.deepEqual(await fs.readFile(source), sourceBytes);
    checks.push({ check: 'external-backup-restore-full-authored-image-four-mappings-unsaved-source-and-closed-outputs', passed: true,
      receipt, artifactSha256: digest(backupBytes), bytes: backupBytes.length, elapsedMs,
      mappingCounts: { midi: 1, osc: 1, dmx: 1, dj: 1 }, timelineBankCount: 2 });
    // Domain replay must return its terminal without rereading a disappeared artifact.
    try {
      await fs.unlink(saved.target_path);
      assert.deepEqual(await success(ids.restore, value), receipt);
      assert.deepEqual(await state(), after);
    } finally { await fs.writeFile(saved.target_path, backupBytes, { flag: 'wx' }); }
    await rejected({ ...value, action: { ...action, expected_source_path: null } }, 'conflict');
    assert.deepEqual(await state(), after);
    checks.push({ check: 'external-backup-restore-terminal-replay-with-missing-artifact-and-changed-shape-conflict', passed: true });
  } finally {
    try {
      if (saved) {
        if (backupBytes) await fs.writeFile(saved.target_path, backupBytes);
        if (!acknowledged) await success(ids.ack, backupRequest);
        await backend.invoke('delete_project_backup', { backupId: saved.backup.id });
      }
    } finally {
      await mcp?.close();
      const finalNames = (await fs.readdir(managed)).sort();
      const additions = finalNames.filter(name => !baseline.includes(name));
      assert.ok(additions.length <= 4, 'Bound the number of recognized scheduled QA autosaves');
      for (const name of additions) {
        const bytes = await fs.readFile(path.join(managed, name));
        const verified = verifyFixtureAutosave(bytes, { name, source, expected, startedAt, observedAt: Date.now() });
        const inspected = await backend.invoke('inspect_project_backup_control_plane_v1', {
          request: { schema_version: 1, backup_id: verified.id } });
        assert.equal(inspected.artifact_sha256, verified.sha256, 'Native bounded unique-key inspection validates the exact autosave bytes');
        assert.equal(inspected.restore_source_path, source);
        fixtureAutosaves.push(verified); // Preserve: this is not our acknowledged explicit backup.
      }
      assert.deepEqual(finalNames, [...baseline, ...fixtureAutosaves.map(value => `backup-${value.id}.json`)].sort());
      if (saved) assert.ok(!finalNames.includes(`backup-${saved.backup.id}.json`), 'The acknowledged owned explicit backup is removed');
      for (const [name, hash] of baselineHashes) assert.equal(digest(await fs.readFile(path.join(managed, name))), hash);
      const summaries = await backend.invoke('list_project_backups');
      assert.deepEqual(summaries.filter(value => !fixtureAutosaves.some(auto => auto.id === value.id)), baselineSummaries);
      assert.equal(summaries.length, baselineSummaries.length + fixtureAutosaves.length);
      assert.equal(path.dirname(path.resolve(directory)), path.resolve(os.tmpdir()));
      assert.ok(path.basename(directory).startsWith('syndocal-backup-restore-'));
      await fs.rm(directory, { recursive: true, force: true });
    }
  }
  checks.push({ check: 'external-backup-restore-owned-cleanup-preserves-baseline-managed-files-and-summaries', passed: true,
    fixtureAutosavesPreserved: fixtureAutosaves });
}
