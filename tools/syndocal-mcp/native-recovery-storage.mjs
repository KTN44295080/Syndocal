import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';

// Own only the isolated acceptance profile's recovery key. Exercise the real
// startup reader after a native restart; never replace App callbacks or IPC.
export async function nativeRecoveryStorage(backend, checks, { restart, currentBackend }) {
  const key = 'syndocal.projectRecovery.v1';
  const digest = raw => raw === null ? null : createHash('sha256').update(raw).digest('hex');
  const pause = ms => new Promise(resolve => setTimeout(resolve, ms));
  const read = () => backend.evaluate(`({raw:localStorage.getItem(${JSON.stringify(key)}),
    footer:document.querySelector('.appStatusLine')?.textContent ?? ''})`);
  const write = raw => backend.evaluate(raw === null
    ? `localStorage.removeItem(${JSON.stringify(key)});true`
    : `localStorage.setItem(${JSON.stringify(key)},${JSON.stringify(raw)});true`);
  const observe = async () => {
    const bundle = await backend.invoke('get_project_authority_bundle');
    const checkpoint = await backend.invoke('get_project_checkpoint', {
      midiMappings: [], oscMappings: [], dmxMappings: [],
    });
    const ownership = await backend.invoke('get_output_ownership_status');
    assert.equal(ownership.lighting_allowed, false);
    assert.equal(ownership.video_allowed, false);
    assert.equal(bundle.snapshot.timeline.playing, false);
    assert.deepEqual(checkpoint.snapshot.fixtures, []);
    assert.deepEqual(checkpoint.snapshot.video.outputs, []);
    return { checkpoint, recoverySerial: bundle.recovery_authority_serial,
      checkpointHash: bundle.checkpoint_hash };
  };
  const original = (await read()).raw;
  const before = await observe();
  const checkpoint = { version: 1, app: 'Syndocal', saved_at: '2026-10-04T00:00:00.000Z',
    source_path: null, signature: 'isolated-native-recovery-preservation', project: before.checkpoint };
  const cases = [
    ['malformed-json', '{broken-json'],
    ['empty-bytes', ''],
    ['stored-null', 'null'],
    ['future-v4', JSON.stringify({ version: 4, kind: 'checkpoint', recovery_authority_serial: before.recoverySerial, checkpoint })],
    ['unsupported-v2', JSON.stringify({ version: 2, kind: 'checkpoint', recovery_authority_serial: before.recoverySerial, checkpoint })],
    ['invalid-v3-checkpoint', JSON.stringify({ version: 3, kind: 'checkpoint', recovery_authority_serial: before.recoverySerial,
      checkpoint: { ...checkpoint, signature: null } })],
    ['negative-v3-tombstone', JSON.stringify({ version: 3, kind: 'tombstone', recovery_authority_serial: -1 })],
  ];
  try {
    for (const [name, raw] of cases) {
      const priorInstance = backend.descriptor.instanceId;
      const check = { check: `native-recovery-startup-${name}-preserves-and-rejects`, passed: false,
        sourceSha256: digest(raw), sourceBytes: Buffer.byteLength(raw), priorInstance };
      checks.push(check);
      await write(raw);
      backend = await restart();
      assert.notEqual(backend.descriptor.instanceId, priorInstance, 'Each recovery observation uses a new native process');
      check.instanceId = backend.descriptor.instanceId;
      const deadline = Date.now() + 20000;
      let observed;
      while (Date.now() < deadline) {
        observed = await read();
        check.observedSha256 = digest(observed.raw);
        check.footer = observed.footer;
        assert.equal(observed.raw, raw, 'Native startup must preserve exact recovery bytes');
        if (/Browser recovery is unavailable:.*Stored data was kept/.test(observed.footer)) break;
        await pause(100);
      }
      assert.match(observed.footer, /Browser recovery is unavailable:.*Stored data was kept.*verified project backup/);
      assert.ok(!observed.footer.includes('isolated-native-recovery-preservation'), 'Diagnostics must not reflect recovery content');
      if (name === 'future-v4' || name === 'unsupported-v2') {
        assert.match(observed.footer, /Unsupported browser recovery format.*compatible Syndocal version/);
      } else assert.match(observed.footer, /Browser recovery data is invalid/);
      await pause(1000);
      assert.equal((await read()).raw, raw, 'Recovery bytes survive the settled startup');
      assert.deepEqual(await observe(), before, 'Rejected recovery must not replace the native checkpoint or advance its journal');
      check.passed = true;
    }
  } finally {
    // Restore even after the negative baseline fails, then prove persistence
    // across one more native restart. Failure remains visible in its own group.
    backend = currentBackend();
    assert.ok(backend, 'QA recovery restoration requires its owned native backend');
    await write(original);
    backend = await restart();
    const restored = await read();
    assert.equal(restored.raw, original, 'QA recovery key must be restored exactly');
    assert.deepEqual(await observe(), before);
    checks.push({ check: 'native-recovery-owned-profile-restored-after-restart', passed: true,
      originalSha256: digest(original), restoredSha256: digest(restored.raw), originalBytes: original === null ? null : Buffer.byteLength(original) });
  }
  return backend;
}
