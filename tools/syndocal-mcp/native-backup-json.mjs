import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import path from 'node:path';
import { createReadStream } from 'node:fs';
import { createHash, randomUUID } from 'node:crypto';
import { nativeInputDiagnostics } from './native-input-diagnostics.mjs';

// Only individually owned files in the enclosing runner's private QA profile.
export async function nativeBackupJson(backend, checks, { inputDiagnostics = false } = {}) {
  const profile = path.resolve(process.env.LOCALAPPDATA, 'jp.seraf.ktn.syndocal.qa.mcp-lifecycle');
  const directory = path.join(profile, 'project-backups');
  assert.equal(path.dirname(directory), profile);
  const existed = await fs.stat(directory).then(() => true, error => {
    if (error.code === 'ENOENT') return false;
    throw error;
  });
  if (!existed) await fs.mkdir(directory);
  const owned = [];
  const ownerId = `backup-json-${randomUUID()}`;
  const digest = bytes => createHash('sha256').update(bytes).digest('hex');
  const fileDigest = async file => {
    const hash = createHash('sha256');
    for await (const chunk of createReadStream(file)) hash.update(chunk);
    return hash.digest('hex');
  };
  const observe = async () => {
    const bundle = await backend.invoke('get_project_authority_bundle');
    const keys = ['project_epoch', 'project_revision', 'checkpoint_hash',
      'timeline_transport_epoch', 'timeline_transport_generation', 'current_project_path',
      'publication_generation', 'publication_kind', 'mapping_replacement_generation',
      'authority_disposition', 'authority_disposition_generation', 'recovery_authority_serial',
      'recovery_authority_last_transition', 'path_generation', 'history_generation'];
    for (const key of keys) assert.ok(Object.hasOwn(bundle, key) && bundle[key] !== undefined);
    return { token: Object.fromEntries(keys.map(key => [key, bundle[key]])),
      checkpoint: await backend.invoke('get_project_checkpoint', { midiMappings: [], oscMappings: [], dmxMappings: [] }),
      ownership: await backend.invoke('get_output_ownership_status') };
  };
  const load = async backupId => {
    const bundle = await backend.invoke('get_project_authority_bundle');
    const args = { backupId, ownerId, expectedEpoch: bundle.project_epoch,
      expectedRevision: bundle.project_revision, expectedCheckpointHash: bundle.checkpoint_hash };
    return backend.evaluate(`window.__TAURI_INTERNALS__.invoke('load_project_backup', ${JSON.stringify(args)})
      .then(value=>({ok:true,value}),error=>({ok:false,error:String(error)}))`);
  };
  const write = async (id, bytes) => {
    const file = path.join(directory, `backup-${id}.json`);
    await fs.writeFile(file, bytes, { flag: 'wx' });
    owned.push(file);
    return file;
  };
  try {
    assert.deepEqual(await fs.readdir(directory), [], 'Private QA backup directory must start empty');
    await backend.invoke('register_project_transaction_owner', { ownerId });
    const initial = await observe();
    assert.equal(initial.ownership.lighting_allowed, false);
    assert.equal(initial.ownership.video_allowed, false);
    assert.deepEqual(initial.checkpoint.snapshot.fixtures, []);
    assert.deepEqual(initial.checkpoint.snapshot.video.layers, []);
    assert.deepEqual(initial.checkpoint.snapshot.video.outputs, []);
    assert.deepEqual(initial.checkpoint.snapshot.timeline.audio_clips ?? [], []);
    const id = Date.now();
    const envelope = { version: 1, app: initial.checkpoint.app, id, created_at_unix_ms: id,
      source_path: null, reason: 'private JSON acceptance 日本語', project: initial.checkpoint };
    envelope.project.snapshot.clock.bpm = 89;
    const valid = Buffer.from(JSON.stringify(envelope));
    const validPath = await write(id, valid);
    assert.equal((await load(id)).ok, true);
    const before = await observe();
    assert.equal(before.checkpoint.snapshot.clock.bpm, 89);
    assert.equal(before.token.authority_disposition, 'unsaved_replacement');
    assert.equal(before.token.current_project_path, null);
    assert.equal(before.ownership.lighting_allowed, false);
    assert.equal(before.ownership.video_allowed, false);
    assert.equal(await fileDigest(validPath), digest(valid));
    checks.push({ check: 'native-backup-json-valid-restores-unsaved-private-project', passed: true,
      sourceSha256: digest(valid), token: before.token, ownership: before.ownership });
    if (inputDiagnostics) await nativeInputDiagnostics({ seed: envelope, backup: true, load, observe, before, digest, checks,
      write: async (index, _name, value) => {
        value.id = id + 100 + index;
        const bytes = Buffer.from(JSON.stringify(value));
        return { file: await write(value.id, bytes), target: value.id, bytes };
      } });
    const changes = [
      ['id-mismatch', value => ({ ...value, id: value.id + 1 }), /backup ID.*filename|filename.*backup ID/],
      ['zero-id', value => ({ ...value, id: 0 }), /backup ID.*positive|positive.*backup ID/],
      ['future-backup-version', value => ({ ...value, version: 2 }), /Unsupported project backup version 2/],
      ['future-project-version', value => ({ ...value, project: { ...value.project, version: 2 } }), /Unsupported project version 2/],
      ['foreign-app', value => ({ ...value, app: 'other' }), /app|application|Syndocal/],
    ];
    const cases = changes.map(([name, change, error], index) => {
      const caseId = id + index + 1;
      return { name, id: caseId, bytes: Buffer.from(JSON.stringify(change({ ...envelope, id: caseId }))), error };
    });
    const text = JSON.stringify({ ...envelope, id: id + 10 });
    cases.push(
      { name: 'duplicate-ignored-envelope', id: id + 10, bytes: Buffer.from(`{"future":0,"future":1,${text.slice(1)}`), error: /duplicate object key/ },
      { name: 'duplicate-ignored-project', id: id + 11, bytes: Buffer.from(JSON.stringify({ ...envelope, id: id + 11 }).replace('"project":{', '"project":{"extra":{"x":0,"x":1},')), error: /duplicate object key/ },
      { name: 'truncated', id: id + 12, bytes: valid.subarray(0, valid.length - 8), error: /EOF|end of|expected|JSON/ },
      { name: 'invalid-utf8', id: id + 13, bytes: Buffer.from([0xff, 0x7b]), error: /UTF-8|utf-8|utf8/ },
    );
    const rejections = [];
    for (const item of cases) {
      const file = await write(item.id, item.bytes);
      const result = await load(item.id);
      assert.equal(result.ok, false, JSON.stringify({ name: item.name, ok: result.ok, error: result.error }));
      assert.match(result.error, item.error);
      assert.deepEqual(await observe(), before, `${item.name}: active state must be preserved`);
      assert.equal(await fileDigest(file), digest(item.bytes));
      rejections.push({ name: item.name, byteLength: item.bytes.length, sourceSha256: digest(item.bytes), error: result.error });
    }
    const oversized = await write(id + 20, Buffer.alloc(0));
    const handle = await fs.open(oversized, 'r+');
    try { await handle.truncate(128 * 1024 * 1024 + 1); } finally { await handle.close(); }
    const oversizedHash = await fileDigest(oversized);
    const result = await load(id + 20);
    assert.equal(result.ok, false);
    assert.match(result.error, /limit.*134217728|134217728.*limit/);
    assert.deepEqual(await observe(), before);
    assert.equal(await fileDigest(oversized), oversizedHash);
    rejections.push({ name: 'oversized', byteLength: (await fs.stat(oversized)).size, sourceSha256: oversizedHash, error: result.error });
    const listed = await backend.invoke('list_project_backups');
    assert.deepEqual(listed.map(value => value.id), [id], 'Only the valid owned backup may be listed');
    assert.equal(await fileDigest(validPath), digest(valid));
    checks.push({ check: 'native-backup-json-rejects-corruption-without-state-or-file-mutation', passed: true,
      rejections, listedIds: listed.map(value => value.id), beforeToken: before.token, afterToken: (await observe()).token });
    assert.equal((await load(listed[0].id)).ok, true);
    const restored = await observe();
    assert.equal(restored.token.project_epoch, before.token.project_epoch + 1);
    assert.equal(restored.checkpoint.snapshot.clock.bpm, 89);
    assert.equal(restored.token.authority_disposition, 'unsaved_replacement');
    assert.equal(restored.token.current_project_path, null);
    assert.equal(restored.ownership.lighting_allowed, false);
    assert.equal(restored.ownership.video_allowed, false);
    assert.equal(await fileDigest(validPath), digest(valid));
    checks.push({ check: 'native-backup-json-explicit-older-restore-after-corrupt-newer-files', passed: true,
      selectedId: listed[0].id, token: restored.token, ownership: restored.ownership });
  } finally {
    for (const file of owned) {
      assert.equal(path.dirname(path.resolve(file)), directory);
      assert.match(path.basename(file), /^backup-[0-9]+\.json$/);
      await fs.unlink(file);
    }
    if (!existed) await fs.rmdir(directory);
  }
}
