import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import { createReadStream } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { createHash, randomUUID } from 'node:crypto';
import { nativeInputDiagnostics } from './native-input-diagnostics.mjs';
import { nativeAuthoredControlProject } from './native-authored-control-project.mjs';

// Only the isolated QA project's path-based native load command is used.
// No dialog, media/device I/O, Enable/Arm action or normal-profile mutation.
export async function nativeProjectJson(backend, checks, { inputDiagnostics = false, authoredControls = false } = {}) {
  const directory = await fs.mkdtemp(path.join(os.tmpdir(), 'syndocal-project-json-'));
  const ownerId = `project-json-${randomUUID()}`;
  const digest = bytes => createHash('sha256').update(bytes).digest('hex');
  const fileDigest = async file => {
    const hash = createHash('sha256');
    for await (const chunk of createReadStream(file)) hash.update(chunk);
    return hash.digest('hex');
  };
  const authority = () => backend.invoke('get_project_authority_bundle');
  const checkpoint = () => backend.invoke('get_project_checkpoint', {
    midiMappings: [], oscMappings: [], dmxMappings: [],
  });
  const observe = async () => {
    const bundle = await authority();
    const keys = ['project_epoch', 'project_revision', 'checkpoint_hash',
      'timeline_transport_epoch', 'timeline_transport_generation',
      'current_project_path', 'publication_generation', 'publication_kind', 'mapping_replacement_generation',
      'authority_disposition', 'authority_disposition_generation', 'recovery_authority_serial',
      'recovery_authority_last_transition', 'path_generation', 'history_generation'];
    for (const key of keys) assert.ok(Object.hasOwn(bundle, key) && bundle[key] !== undefined,
      `Authority observation must carry ${key}`);
    return { token: Object.fromEntries(keys.map(key => [key, bundle[key]])),
      checkpoint: await checkpoint(), ownership: await backend.invoke('get_output_ownership_status') };
  };
  const load = async file => {
    const bundle = await authority();
    const args = { path: file, ownerId, expectedEpoch: bundle.project_epoch,
      expectedRevision: bundle.project_revision, expectedCheckpointHash: bundle.checkpoint_hash };
    return backend.evaluate(`window.__TAURI_INTERNALS__.invoke('load_project_path', ${JSON.stringify(args)})
      .then(value=>({ok:true,value}),error=>({ok:false,error:String(error)}))`);
  };
  try {
    await backend.invoke('register_project_transaction_owner', { ownerId });
    const initial = await observe();
    assert.equal(initial.ownership.lighting_allowed, false);
    assert.equal(initial.ownership.video_allowed, false);
    assert.deepEqual(initial.checkpoint.snapshot.fixtures, []);
    assert.deepEqual(initial.checkpoint.snapshot.video.outputs, []);
    assert.deepEqual(initial.checkpoint.snapshot.video.layers, []);
    assert.deepEqual(initial.checkpoint.snapshot.timeline.audio_clips ?? [], []);
    const project = initial.checkpoint;
    project.snapshot.clock.bpm = 97;
    const valid = Buffer.from(JSON.stringify(project));
    const validPath = path.join(directory, 'valid-日本語.sdc');
    await fs.writeFile(validPath, valid, { flag: 'wx' });
    const accepted = await load(validPath);
    assert.equal(accepted.ok, true, JSON.stringify({ error: accepted.error }));
    const before = await observe();
    assert.equal(before.checkpoint.snapshot.clock.bpm, 97);
    assert.equal(before.ownership.lighting_allowed, false);
    assert.equal(before.ownership.video_allowed, false);
    assert.equal(digest(await fs.readFile(validPath)), digest(valid));
    checks.push({ check: 'native-project-json-valid-unicode-path-load', passed: true,
      sourceSha256: digest(valid), token: before.token, ownership: before.ownership });
    if (inputDiagnostics) await nativeInputDiagnostics({ seed: project, backup: false, load, observe, before, digest, checks,
      write: async (_index, name, value) => {
        const file = path.join(directory, `diagnostic-${name}.sdc`);
        const bytes = Buffer.from(JSON.stringify(value));
        await fs.writeFile(file, bytes, { flag: 'wx' });
        return { file, target: file, bytes };
      } });
    const json = valid.toString('utf8');
    assert.ok(json.includes('"version":1') && json.includes('"bpm":97'));
    const cases = [
      ['duplicate-version', Buffer.from(`{"version":0,${json.slice(1)}`), /duplicate object key/],
      ['duplicate-identical', Buffer.from(`{"version":1,${json.slice(1)}`), /duplicate object key/],
      ['duplicate-escaped-key', Buffer.from(`{"ver\\u0073ion":1,${json.slice(1)}`), /duplicate object key/],
      ['duplicate-nested-bpm', Buffer.from(json.replace('"bpm":97', '"bpm":11,"bpm":97')), /duplicate object key/],
      ['truncated', valid.subarray(0, valid.length - 8), /EOF|end of|expected|parse|JSON/],
      ['invalid-utf8', Buffer.from([0xff, ...valid.subarray(0, 32)]), /UTF-8|utf-8|utf8/],
      ['future-version', Buffer.from(json.replace('"version":1', '"version":2')), /Unsupported project version 2/],
    ];
    const rejections = [];
    for (const [name, bytes, error] of cases) {
      const file = path.join(directory, `${name}.sdc`);
      await fs.writeFile(file, bytes, { flag: 'wx' });
      const rejected = await load(file);
      assert.equal(rejected.ok, false, JSON.stringify({ name, ok: rejected.ok, error: rejected.error }));
      assert.match(rejected.error, error);
      assert.deepEqual(await observe(), before, `${name}: active project/authority/output must be unchanged`);
      assert.equal(digest(await fs.readFile(file)), digest(bytes), `${name}: source bytes must be unchanged`);
      rejections.push({ name, sourceSha256: digest(bytes), byteLength: bytes.length, error: rejected.error });
    }
    const oversized = path.join(directory, 'oversized.sdc');
    const handle = await fs.open(oversized, 'wx');
    try { await handle.truncate(64 * 1024 * 1024 + 1); }
    finally { await handle.close(); }
    const oversizedSha256 = await fileDigest(oversized);
    const rejectedSize = await load(oversized);
    assert.equal(rejectedSize.ok, false);
    assert.match(rejectedSize.error, /limit.*67108864|67108864.*limit/);
    assert.deepEqual(await observe(), before);
    assert.equal(await fileDigest(oversized), oversizedSha256);
    rejections.push({ name: 'oversized', byteLength: (await fs.stat(oversized)).size,
      sourceSha256: oversizedSha256, error: rejectedSize.error });
    checks.push({ check: 'native-project-json-hostile-load-preserves-active-project-and-source-bytes', passed: true,
      rejections, beforeToken: before.token, afterToken: (await observe()).token });
    if (authoredControls) await nativeAuthoredControlProject({ directory, load, observe, checks,
      observeTransport: async () => {
        const { snapshot } = await backend.invoke('get_snapshot');
        return { timelineId: snapshot.timeline.id, playing: snapshot.timeline.playing,
          positionMs: snapshot.timeline.position_ms };
      } });
  } finally {
    assert.equal(path.dirname(path.resolve(directory)), path.resolve(os.tmpdir()));
    assert.ok(path.basename(directory).startsWith('syndocal-project-json-'));
    await fs.rm(directory, { recursive: true, force: true });
  }
}
