import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import path from 'node:path';
import { createHash } from 'node:crypto';
import { isDeepStrictEqual } from 'node:util';

function assertWhole(actual, expected, label, pointer = '') {
  if (isDeepStrictEqual(actual, expected)) return;
  if (actual && expected && typeof actual === 'object' && typeof expected === 'object'
      && Array.isArray(actual) === Array.isArray(expected)) {
    const actualKeys = Object.keys(actual).sort();
    const expectedKeys = Object.keys(expected).sort();
    assert.deepEqual(actualKeys, expectedKeys, `${label}: keys at ${pointer || '/'}`);
    for (const key of expectedKeys) {
      assertWhole(actual[key], expected[key], label,
        `${pointer}/${key.replaceAll('~', '~0').replaceAll('/', '~1')}`);
    }
  }
  assert.fail(`${label}: mismatch at ${pointer || '/'}: actual=${JSON.stringify(actual)}, expected=${JSON.stringify(expected)}`);
}

// The frozen fixture owns the expectation. Native adaptation removes every
// video source/output and its references, so no capture/transport is opened.
// Graph Live Audio is a declaration only: no device enable command is issued.
export async function nativeAuthoredControlProject({ directory, load, observe, observeTransport, checks }) {
  const fixtureBytes = await fs.readFile(new URL('../../qa/migration/authored-control-project.json', import.meta.url));
  const digest = value => createHash('sha256').update(value).digest('hex');
  const manifest = JSON.parse(await fs.readFile(new URL('../../qa/migration/authored-control-project-oracle.json', import.meta.url), 'utf8'));
  const oracleSha256Lf = digest(fixtureBytes.toString('utf8').replaceAll('\r\n', '\n'));
  assert.equal(oracleSha256Lf, manifest.reference_sha256_lf, 'Frozen authored oracle identity');
  const expected = JSON.parse(fixtureBytes);
  const snapshot = expected.snapshot;
  snapshot.clock.bpm = 97;
  // Stopped Timeline automation is intentionally evaluated at position zero
  // (the same contract used by paused Seek). Author the neutral native fixture
  // at that fixed first keyframe, independently of any candidate observation.
  snapshot.fixtures[0].attribute_values.find(value => value.attribute === 'Dimmer').value = 65535;
  snapshot.video.layers = [];
  delete snapshot.video.media_assets;
  snapshot.video.compositions = [{ id: 1, label: 'Main', layer_ids: [], output_ids: [], timeline_layer_ids: [] }];
  snapshot.video.outputs = [];
  snapshot.video.mapping_presets = [];
  for (const cue of snapshot.cues) {
    cue.video_targets = [];
    cue.video_output_targets = [];
  }
  snapshot.timeline.video_automations = [];
  for (const timeline of snapshot.timeline_bank) timeline.video_automations = [];
  for (const effect of snapshot.effects) {
    effect.video_targets = [];
    effect.lfo.video_targets = [];
  }
  const audioOutput = snapshot.node_graphs[0].nodes.find(node => node.id === 15).output;
  audioOutput.fixture_ids = [1];
  audioOutput.attribute = 'ColorRed';
  audioOutput.video_targets = [];
  const assertNoMediaPaths = value => {
    if (!value || typeof value !== 'object') return;
    assert.ok(!Object.hasOwn(value, 'path') || value.path === null, 'No media/audio file path allowed');
    for (const child of Object.values(value)) assertNoMediaPaths(child);
  };
  assertNoMediaPaths(expected);
  const neutralFile = path.join(directory, 'authored-controls-stopped.sdc');
  const neutralSource = Buffer.from(JSON.stringify(expected));
  await fs.writeFile(neutralFile, neutralSource, { flag: 'wx' });
  const neutralAccepted = await load(neutralFile);
  assert.equal(neutralAccepted.ok, true, JSON.stringify({ error: neutralAccepted.error }));
  const neutral = await observe();
  assert.equal(neutral.ownership.lighting_allowed, false);
  assert.equal(neutral.ownership.video_allowed, false);
  assertWhole(neutral.checkpoint, expected, 'Stopped native authored checkpoint');
  assert.deepEqual(await fs.readFile(neutralFile), neutralSource);
  checks.push({ check: 'native-authored-control-stopped-whole-checkpoint', passed: true,
    oracleSha256Lf, sourceSha256: digest(neutralSource), neutralDimmerValue: 65535,
    timelineBankCount: 2, effectsEnabled: snapshot.effects[0].enabled,
    graphEnabled: snapshot.node_graphs[0].enabled, ownership: neutral.ownership, token: neutral.token });
  const candidate = structuredClone(expected);
  candidate.snapshot.timeline.playing = true;
  candidate.snapshot.timeline.position_ms = 750;
  for (const [index, timeline] of candidate.snapshot.timeline_bank.entries()) {
    timeline.playing = true;
    timeline.position_ms = 500 + 1000 * index;
  }
  candidate.snapshot.node_graphs[0].audio_runtime = [{ node_id: 14,
    input_value: 0.75, output_value: 0.5, source_available: true,
    safety_zeroed: false, held: true, feature_sequence: 987 }];
  const source = Buffer.from(JSON.stringify(candidate));
  const file = path.join(directory, 'authored-controls-日本語.sdc');
  await fs.writeFile(file, source, { flag: 'wx' });
  const accepted = await load(file);
  assert.equal(accepted.ok, true, JSON.stringify({ error: accepted.error }));
  const transportAfterLoad = await observeTransport();
  assert.equal(transportAfterLoad.playing, false, 'A project load must not resume Timeline playback');
  assert.equal(transportAfterLoad.positionMs, 0);
  await new Promise(resolve => setTimeout(resolve, 250));
  assert.deepEqual(await observeTransport(), transportAfterLoad, 'Stopped transport must remain at zero across engine ticks');
  const before = await observe();
  assert.equal(before.ownership.lighting_allowed, false);
  assert.equal(before.ownership.video_allowed, false);
  assertWhole(before.checkpoint, expected, 'Native runtime-contaminated authored checkpoint');
  assert.deepEqual(await fs.readFile(file), source, 'Load must not rewrite the original runtime-contaminated file');
  const canonicalBytes = Buffer.from(JSON.stringify(before.checkpoint));
  const canonicalFile = path.join(directory, 'authored-controls-canonical.sdc');
  await fs.writeFile(canonicalFile, canonicalBytes, { flag: 'wx' });
  const reloaded = await load(canonicalFile);
  assert.equal(reloaded.ok, true, JSON.stringify({ error: reloaded.error }));
  const after = await observe();
  const transportAfterReload = await observeTransport();
  assert.equal(transportAfterReload.playing, false);
  assert.equal(transportAfterReload.positionMs, 0);
  assertWhole(after.checkpoint, expected, 'Native reload');
  assert.equal(digest(Buffer.from(JSON.stringify(after.checkpoint))), digest(canonicalBytes));
  assert.equal(after.ownership.lighting_allowed, false);
  assert.equal(after.ownership.video_allowed, false);
  checks.push({ check: 'native-authored-control-whole-checkpoint-load-reload', passed: true,
    oracleSha256Lf, sourceSha256: digest(source), canonicalSha256: digest(canonicalBytes),
    transportAfterLoad, transportAfterReload, stoppedObservationMs: 250,
    timelineBankCount: 2, graphNodes: 5, graphEdges: 3, effects: 1, touchPages: 2, touchControls: 8,
    removedVideoSourcesForNativeProbe: true, ownership: after.ownership,
    beforeToken: before.token, afterToken: after.token });
}
