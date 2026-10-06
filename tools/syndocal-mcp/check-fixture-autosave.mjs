import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import { verifyFixtureAutosave } from './native-fixture-autosave.mjs';
const oracle = JSON.parse(await fs.readFile(new URL('../../qa/migration/authored-control-project.json', import.meta.url)));
const expected = structuredClone(oracle);
for (const key of ['midi_mappings', 'osc_mappings', 'dmx_mappings', 'dj_track_triggers']) expected[key] ??= [];
const project = structuredClone(expected);
const mappings = {};
for (const key of ['midi_mappings', 'osc_mappings', 'dmx_mappings', 'dj_track_triggers']) { mappings[key] = project[key]; delete project[key]; }
const value = { version: 1, app: 'Syndocal', id: 101, created_at_unix_ms: 101,
  source_path: 'owned-fixture.sdc', reason: 'autosave', project, ...mappings };
const context = { name: 'backup-101.json', source: 'owned-fixture.sdc', expected, startedAt: 100, observedAt: 102 };
const check = (object, overrides = {}) => verifyFixtureAutosave(Buffer.from(JSON.stringify(object)), { ...context, ...overrides });
assert.equal(check(value).id, 101);
const geometry = value.project.custom_profiles[0].geometries[0].model_dimensions;
geometry.x = 0.42; expected.custom_profiles[0].geometries[0].model_dimensions.x = Math.fround(0.42);
assert.equal(check(value).floatMatches, 1);
let rejected = 0;
for (const [alter, overrides] of [
  [v => { v.reason = 'manual'; }], [v => { v.source_path = 'unowned.sdc'; }],
  [v => { v.version = 2; }], [v => { v.app = 'Other'; }],
  [v => { v.id = Number.MAX_SAFE_INTEGER + 1; }], [v => { v.created_at_unix_ms = 99; }],
  [v => { v.created_at_unix_ms = 103; }], [v => { v.unrecognized = true; }],
  [v => { v.project.midi_mappings = []; }], [v => { v.osc_mappings.push({ address: '/changed' }); }],
  [v => { v.project.snapshot.fixtures[0].id += 0.000001; }],
  [v => { v.project.custom_profiles[0].geometries[0].model_dimensions.x = 0.43; }],
  [v => { v.project.snapshot.timeline.duration_ms += 0.000001; }],
  [v => { delete v.project.snapshot.touch_surface; }],
  [() => {}, { name: 'backup-102.json' }],
]) { const candidate = structuredClone(value); alter(candidate); assert.throws(() => check(candidate, overrides)); rejected++; }
assert.throws(() => verifyFixtureAutosave(Buffer.from('{'), context)); rejected++;
console.log(`Fixture autosave recognition PASS: 2 valid full-image cases, ${rejected} unsafe/changed rejections; no deletion or native/device calls`);
