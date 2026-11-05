import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync, readdirSync, mkdtempSync, mkdirSync, rmSync, copyFileSync, writeFileSync, existsSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { runInNewContext } from 'node:vm';

const projectUrl = new URL('../../tools/distribution/syphon-friend/syphon-acceptance-project.sdc', import.meta.url);
const collector = new URL('../../tools/distribution/collect-syphon-acceptance.command', import.meta.url);
const project = JSON.parse(readFileSync(projectUrl, 'utf8'));
const collectorSource = readFileSync(collector, 'utf8');
const validator = collectorSource.match(/\/\/ BEGIN DIAGNOSTIC VALIDATOR[^\n]*\n([\s\S]*?)\/\/ END DIAGNOSTIC VALIDATOR/)[1];
const commit = 'a'.repeat(40);
const kit = { schema_version: 2, commit, version: '1.2.0-alpha.84', syphon_input_enabled: true };
const goodReport = () => ({ schema_version: 2, process_id: 123, app_version: kit.version, build_commit: commit,
  input_enabled: true, external_client_verified: false, routes: { '1': { output_id: 1, server_name: 'Syndocal-Syphon-Test',
    native: { macos_version: '15.0', metal_device_name: 'Test Metal', has_unified_memory: true },
    frames_published: 500, last_publish_latency_ms: 0.2, width: 1920, height: 1080, route_state: 'native_closed',
    native_retired: true, last_error: null, output_ownership: { effective_role: 'Standby' }, observed_unix_ms: Date.now() } }, inputs: { '2': {
      input_id: 2, source_id: 2, selected_server: JSON.stringify({name:'Simple Server',app_name:'Simple Server',uuid:'test-uuid'}),
      frames_received: 500, width: 640, height: 480, last_receive_latency_ms: 0.3,
      native_retired: true, route_state: 'native_closed', last_error: null, observed_unix_ms: Date.now()
    } } });
const validate = (report, manifest = kit) => JSON.parse(JSON.stringify(runInNewContext(`${validator}\ncheckDiagnostic(report, kit, now)`, { report, kit: manifest, now: Date.now() })));

test('friend project has no file/device/network sources or enabled lighting', () => {
  assert.equal(project.version, 2);
  assert.equal(project.app, 'Syndocal');
  const s = project.snapshot;
  assert.deepEqual(s.fixtures, []);
  assert.deepEqual(s.dmx_outputs, []);
  assert.equal(s.output.enabled, false);
  assert.equal(s.video.layers.length, 1);
  const l = s.video.layers[0];
  assert.deepEqual(l.source.kind, { Generator: { Noise: { seed: 92, scale: 5, speed: 100, color_a: [0, 0, 0, 255], color_b: [255, 255, 255, 255] } } });
  assert.equal(l.source.path, null);
  assert.equal(l.isf_effect.enabled, true);
  assert.match(l.isf_effect.source, /sin\(TIME\)/);
  assert.doesNotMatch(l.isf_effect.source, /"IMPORTED"|"TYPE":"image"/);
  assert.deepEqual(s.video.compositions[0].layer_ids, [l.id]);
  assert.equal(s.video.outputs.length, 1);
  assert.equal(s.video.outputs[0].kind, 'SyphonServer');
  assert.equal(s.video.outputs[0].endpoint_name, 'Syndocal-Syphon-Test');
  assert.equal(s.video.outputs[0].width, 1920);
  assert.equal(s.video.outputs[0].height, 1080);
});

test('fixture regenerates deterministically', () => {
  const before = readFileSync(projectUrl, 'utf8');
  const run = spawnSync(process.execPath, [fileURLToPath(new URL('../../tools/distribution/syphon-friend/make-project.mjs', import.meta.url))], { encoding: 'utf8' });
  assert.equal(run.status, 0, run.stderr);
  assert.equal(readFileSync(projectUrl, 'utf8'), before);
});

test('diagnostic checks require exact build, schema, publication, close and plausible timestamp', () => {
  const good = validate(goodReport());
  assert.equal(good.matched_build, true);
  assert.equal(good.test_route_published, true);
  assert.equal(good.test_route_native_retired, true);
  assert.equal(good.test_route_1920x1080, true);
  assert.equal(good.input_received, true);
  assert.equal(good.input_no_native_error, true);
  assert.equal(good.input_native_retired, true);
  for (const change of [
    r => { r.schema_version = 1; },
    r => { r.input_enabled = false; },
    r => { r.build_commit = 'b'.repeat(40); },
    r => { r.app_version = 'other'; },
    r => { r.routes['1'].server_name = 'Other'; },
    r => { r.routes['1'].observed_unix_ms -= 86401000; },
    r => { r.routes['1'].observed_unix_ms += 301000; },
  ]) {
    const report = goodReport(); change(report);
    const result = validate(report);
    assert.equal(result.test_route_published, false);
    assert.equal(result.test_route_native_retired, false);
  }
  const empty = goodReport(); empty.routes['1'].frames_published = 0;
  assert.equal(validate(empty).test_route_published, false);
  const open = goodReport(); open.routes['1'].native_retired = false;
  assert.equal(validate(open).test_route_native_retired, false);
  const failed = goodReport(); failed.routes['1'].last_error = 'Metal error'; failed.routes['1'].route_state = 'native_closed_after_error';
  assert.equal(validate(failed).test_route_native_retired, false);
  assert.equal(validate(failed).test_route_no_native_error, false);
  assert.equal(validate(null).report_schema_valid, false);
  assert.equal(validate(goodReport(), null).matched_build, false);
});

test('collector creates an explicitly incomplete package when evidence is missing', { skip: process.platform !== 'darwin' }, () => {
  const home = mkdtempSync(join(tmpdir(), 'syphon-collector-test-'));
  try {
    const desktop = join(home, 'Desktop'); mkdirSync(desktop);
    const run = spawnSync('/bin/bash', [fileURLToPath(collector), '--app', join(home, 'absent', 'Syndocal.app')], { env: { ...process.env, HOME: home }, encoding: 'utf8', input: '' });
    assert.equal(run.status, 0, run.stderr);
    const folder = readdirSync(desktop).find(n => !n.endsWith('.zip'));
    assert.ok(folder);
    const manifest = JSON.parse(readFileSync(join(desktop, folder, 'collection.json'), 'utf8'));
    for (const key of ['app_identity_verified', 'codesign_verified', 'syphon_linkage_verified', 'framework_present']) assert.equal(manifest[key], false);
    for (const key of ['report_json_valid', 'report_schema_valid', 'matched_build', 'test_route_published', 'test_route_native_retired']) assert.equal(manifest.diagnostic_checks[key], false);
    assert.equal(manifest.screenshots_collected, 0);
    assert.equal(manifest.physical_acceptance, 'requires_owner_review');
    assert.equal(manifest.syphon_input, 'requires_owner_review');
    assert.ok(readdirSync(desktop).some(n => n.endsWith('.zip')));
  } finally { rmSync(home, { recursive: true, force: true }); }
});

for (const [name, report, expected] of [
  ['matching report', JSON.stringify(goodReport()), true],
  ['invalid JSON', '{this is invalid JSON', false],
  ['old schema', JSON.stringify({ ...goodReport(), schema_version: 1 }), false],
  ['different build', JSON.stringify({ ...goodReport(), build_commit: 'b'.repeat(40) }), false],
]) {
  test(`macOS collector validates and retains ${name}`, { skip: process.platform !== 'darwin' }, () => {
    const home = mkdtempSync(join(tmpdir(), 'syphon-collector-test-'));
    try {
      const desktop = join(home, 'Desktop'); mkdirSync(desktop);
      const sourceKit = join(home, 'kit'); mkdirSync(sourceKit);
      const script = join(sourceKit, 'collect-syphon-acceptance.command'); copyFileSync(collector, script);
      writeFileSync(join(sourceKit, 'kit-manifest.json'), JSON.stringify(kit));
      const logs = join(home, 'Library/Application Support/jp.seraf.ktn.syndocal/logs'); mkdirSync(logs, { recursive: true });
      writeFileSync(join(logs, 'syphon-acceptance.json'), report);
      const run = spawnSync('/bin/bash', [script, '--app', join(home, 'absent', 'Syndocal.app')], { env: { ...process.env, HOME: home }, encoding: 'utf8', input: '' });
      assert.equal(run.status, 0, run.stderr);
      const folder = join(desktop, readdirSync(desktop).find(n => !n.endsWith('.zip')));
      assert.equal(readFileSync(join(folder, 'syphon-acceptance.json'), 'utf8'), report);
      const collected = JSON.parse(readFileSync(join(folder, 'collection.json'), 'utf8'));
      assert.equal(collected.diagnostic_checks.matched_build, expected);
      assert.equal(collected.diagnostic_checks.test_route_published, expected);
      assert.equal(collected.diagnostic_checks.test_route_native_retired, expected);
      assert.equal(collected.physical_acceptance, 'requires_owner_review');
      assert.ok(existsSync(`${folder}.zip`));
    } finally { rmSync(home, { recursive: true, force: true }); }
  });
}


test('Kit 2 input evidence rejects absent, malformed, stale and failed-close records', () => {
  for (const change of [
    r => { r.inputs = {}; },
    r => { r.inputs['2'].frames_received = 0; },
    r => { r.inputs['2'].width = 0; },
    r => { r.inputs['2'].height = 8193; },
    r => { r.inputs['2'].selected_server = ''; },
    r => { r.inputs['2'].last_receive_latency_ms = -1; },
    r => { r.inputs['2'].observed_unix_ms -= 86401000; },
    r => { r.inputs['2'].observed_unix_ms += 301000; },
  ]) {
    const report = goodReport(); change(report);
    assert.equal(validate(report).input_received, false);
    assert.equal(validate(report).input_native_retired, false);
    assert.equal(validate(report).test_route_published, true, 'bad input never changes output evidence');
  }
  const open = goodReport(); open.inputs['2'].native_retired = false;
  assert.equal(validate(open).input_received, true);
  assert.equal(validate(open).input_native_retired, false);
  const failed = goodReport(); failed.inputs['2'].last_error = 'close failed';
  failed.inputs['2'].route_state = 'failed';
  failed.inputs['2'].native_retired = false;
  assert.equal(validate(failed).input_native_retired, false);
  assert.equal(validate(failed).input_no_native_error, false);
  const old = goodReport(); old.schema_version = 1;
  assert.equal(validate(old).input_received, false);
  assert.equal(validate(goodReport(), {...kit, schema_version:3}).matched_build, false);
  const concurrent = goodReport();
  concurrent.inputs['3'] = {...concurrent.inputs['2'], input_id:3, native_retired:false, route_state:'constructed'};
  assert.equal(validate(concurrent).input_received, true);
  assert.equal(validate(concurrent).input_native_retired, false, 'one closed client cannot hide another live owner');
  concurrent.inputs['3'].native_retired = true;
  concurrent.inputs['3'].route_state = 'native_closed_after_error';
  concurrent.inputs['3'].last_error = 'publisher vanished before recovery';
  assert.equal(validate(concurrent).input_native_retired, true, 'retirement and expected restart errors are separate evidence');
});

test('Kit 2 guide and collector include the external input observation and third screenshot', () => {
  const guide = readFileSync(new URL('../../tools/distribution/syphon-friend/SYPHON-FRIEND-TEST-GUIDE.md', import.meta.url), 'utf8');
  const observation = readFileSync(new URL('../../tools/distribution/syphon-friend/observations.txt', import.meta.url), 'utf8');
  assert.match(guide, /Simple Server/);
  assert.match(guide, /screenshot3\.png/);
  assert.match(guide, /20 min/);
  assert.match(observation, /Missing-server error and restart recovery/);
  assert.match(collectorSource, /for name in screenshot1\.png screenshot2\.png screenshot3\.png/);
});
