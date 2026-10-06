import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';

// Recognition only, never permission to delete an unacknowledged GUI backup.
// Known f32 leaves account for native direct serialization versus serde Value;
// IDs, mappings, keys and every other authored leaf retain exact comparisons.
const nativeFloat = pointer => /^(?:\/custom_profiles\/\d+|\/snapshot\/fixtures\/\d+)\/geometries\/\d+\/(?:matrix\/\d+|model_dimensions\/[xyz]|beam_angle_deg|field_angle_deg|beam_radius)$/.test(pointer)
  || /^\/snapshot\/(?:stage_map_presets\/\d+\/)?stage_objects\/\d+\/(?:x|z|width|depth|rotation_deg)$/.test(pointer);

export function verifyFixtureAutosave(bytes, { name, source, expected, startedAt, observedAt }) {
  assert.ok(bytes.length <= 128 * 1024 * 1024);
  const value = JSON.parse(bytes.toString('utf8'));
  assert.deepEqual(Object.keys(value).sort(), ['app', 'created_at_unix_ms', 'dj_track_triggers', 'dmx_mappings',
    'id', 'midi_mappings', 'osc_mappings', 'project', 'reason', 'source_path', 'version']);
  assert.equal(value.version, 1); assert.equal(value.app, 'Syndocal');
  assert.ok(Number.isSafeInteger(value.id) && value.id > 0);
  assert.equal(name, `backup-${value.id}.json`);
  assert.equal(value.reason, 'autosave'); assert.equal(value.source_path, source);
  assert.ok(Number.isSafeInteger(value.created_at_unix_ms)
    && value.created_at_unix_ms >= startedAt && value.created_at_unix_ms <= observedAt);
  const actual = { ...value.project };
  for (const key of ['midi_mappings', 'osc_mappings', 'dmx_mappings', 'dj_track_triggers']) {
    assert.ok(!Object.hasOwn(actual, key), 'Envelope mappings must not shadow project mappings');
    actual[key] = value[key];
  }
  let floatMatches = 0;
  const compare = (left, right, pointer = '') => {
    if (typeof left === 'number' || typeof right === 'number') {
      assert.ok(Number.isFinite(left) && Number.isFinite(right), pointer);
      if (left !== right && nativeFloat(pointer)) {
        assert.equal(Math.fround(left), Math.fround(right), pointer); floatMatches++; return;
      }
    }
    if (left && right && typeof left === 'object' && typeof right === 'object') {
      assert.equal(Array.isArray(left), Array.isArray(right), pointer);
      assert.deepEqual(Object.keys(left).sort(), Object.keys(right).sort(), pointer);
      for (const key of Object.keys(left)) compare(left[key], right[key], `${pointer}/${key}`);
    } else assert.deepEqual(left, right, pointer);
  };
  compare(expected, actual);
  return { id: value.id, source, sha256: createHash('sha256').update(bytes).digest('hex'),
    bytes: bytes.length, createdAt: value.created_at_unix_ms, floatMatches };
}
