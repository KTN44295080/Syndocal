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
const kit = { schema_version: 1, commit, version: '1.2.0-alpha.83', syphon_input_enabled: false };
const goodReport = () => ({ schema_version: 1, process_id: 123, app_version: kit.version, build_commit: commit,
  input_enabled: false, external_client_verified: false, routes: { '1': { output_id: 1, server_name: 'Syndocal-Syphon-Test',
    native: { macos_version: '15.0', metal_device_name: 'Test Metal', has_unified_memory: true },
    frames_published: 500, last_publish_latency_ms: 0.2, width: 1920, height: 1080, route_state: 'native_closed',
    native_retired: true, last_error: null, output_ownership: { effective_role: 'Standby' }, observed_unix_ms: Date.now() } } });
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
  for (const change of [
    r => { r.schema_version = 2; },
    r => { r.input_enabled = true; },
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
    assert.equal(manifest.syphon_input, 'not_built');
    assert.ok(readdirSync(desktop).some(n => n.endsWith('.zip')));
  } finally { rmSync(home, { recursive: true, force: true }); }
});

for (const [name, report, expected] of [
  ['matching report', JSON.stringify(goodReport()), true],
  ['invalid JSON', '{this is invalid JSON', false],
  ['old schema', JSON.stringify({ ...goodReport(), schema_version: 2 }), false],
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
