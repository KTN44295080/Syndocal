import assert from 'node:assert/strict';

// Own software-loopback route only. A rejected replacement must have no
// lifecycle side effects; an accepted replacement must retire managed output.
export async function nativeProjectReplacementPreflight({ backend, promote, grant, output, accepted,
  authority, checkpoint, ownership, stable, awaitImage, full, zero, pause, packets, healthy, ownerId, load, file, checks }) {
  await promote();
  await grant('output', 'syndocal.output.enable.v2');
  await grant('output', 'syndocal.output.blackout.release.v2');
  // This bundle also includes the free-running preview clock. Compare the
  // complete authority metadata separately from its volatile rendered image.
  const readAuthority = async () => { const { snapshot, ...metadata } = await authority(); return metadata; };
  const before = await readAuthority();
  const enabled = accepted((await output('syndocal.output.enable.v2', { kind: 'enable_output' })).receipt);
  const gates = await ownership();
  assert.equal(gates.lighting_allowed, true);
  assert.equal(gates.video_allowed, true);
  checks.push({ check: 'native-managed-enable-before-project-replacement-preflight', passed: true,
    receipt: enabled, ownership: gates });
  const invoke = (command, args) => backend.evaluate(`window.__TAURI_INTERNALS__.invoke(${JSON.stringify(command)}, ${JSON.stringify(args)})
    .then(value=>({ok:true,value}),error=>({ok:false,error:String(error)}))`);
  const live = await checkpoint();
  const expected = { ownerId, expectedEpoch: before.project_epoch, expectedRevision: before.project_revision,
    expectedCheckpointHash: before.checkpoint_hash };
  for (const command of ['new_project', 'load_project_path']) {
    for (const [field, value] of [['expectedEpoch', before.project_epoch + 1],
      ['expectedRevision', before.project_revision + 1], ['expectedCheckpointHash', '0'.repeat(64)]]) {
      const args = { ...expected, [field]: value, ...(command === 'load_project_path' ? { path: file } : {}) };
      const check = { check: `native-${command}-${field}-rejection-preserves-project-and-live-managed-output`, passed: false };
      checks.push(check);
      const observed = await stable(`${command}-${field}-rejects-with-live-managed-output`, full, async () => {
        const result = await invoke(command, args);
        const after = { authority: await readAuthority(), checkpoint: await checkpoint(), ownership: await ownership() };
        Object.assign(check, {
          request: args, result, authorityBefore: before, authorityAfter: after.authority,
          checkpointUnchanged: JSON.stringify(after.checkpoint) === JSON.stringify(live),
          ownershipBefore: gates, ownershipAfter: after.ownership });
        assert.equal(result.ok, false, JSON.stringify(result));
        assert.match(result.error, /authority|epoch|revision|checkpoint/i);
        assert.deepEqual(await readAuthority(), before);
        assert.deepEqual(await checkpoint(), live);
        assert.deepEqual(await ownership(), gates);
        return result;
      });
      Object.assign(check, { passed: true, ...observed.observation });
    }
  }
  // Valid Open exercises the platform-backed path; valid New the default path.
  await load(file);
  assert.equal((await readAuthority()).project_epoch, before.project_epoch + 1);
  assert.equal((await readAuthority()).project_revision, 0);
  assert.equal((await ownership()).lighting_allowed, false);
  assert.equal((await ownership()).video_allowed, false);
  await awaitImage(zero);
  await pause(250);
  const stopped = packets();
  await pause(500); healthy();
  assert.equal(packets(), stopped, 'Valid Open must stop its managed loopback sender');
  checks.push({ check: 'native-valid-open-retires-managed-output', passed: true });
  const reenabled = accepted((await output('syndocal.output.enable.v2', { kind: 'enable_output' })).receipt);
  assert.equal((await backend.invoke('get_snapshot')).snapshot.safety_blackout_engaged, true,
    'Managed replacement leaves S0 engaged; Enable alone cannot silently release it');
  await awaitImage(zero);
  const safeReenable = await stable('native-explicit-reenable-retains-s0-zero-image', zero);
  checks.push({ check: 'native-enable-after-project-replacement-retains-s0-and-live-zero-image', passed: true, ...safeReenable.observation });
  accepted((await output('syndocal.output.blackout.release.v2', { kind: 'release_blackout', lease: reenabled.lease_result.authority })).receipt);
  assert.equal((await backend.invoke('get_snapshot')).snapshot.safety_blackout_engaged, false);
  await awaitImage(full);
  const renewedLive = await stable('native-reenabled-full-image-before-valid-new', full);
  checks.push({ check: 'native-explicit-reenable-after-valid-open-produces-live-whole-image', passed: true, ...renewedLive.observation });
  const current = await authority();
  const created = await invoke('new_project', { ownerId, expectedEpoch: current.project_epoch,
    expectedRevision: current.project_revision, expectedCheckpointHash: current.checkpoint_hash });
  assert.equal(created.ok, true, JSON.stringify(created));
  assert.equal((await authority()).project_epoch, current.project_epoch + 1);
  assert.deepEqual((await checkpoint()).snapshot.fixtures, []);
  assert.equal((await ownership()).lighting_allowed, false);
  assert.equal((await ownership()).video_allowed, false);
  await awaitImage(zero);
  await pause(250);
  const afterNew = packets();
  await pause(500); healthy();
  assert.equal(packets(), afterNew, 'Valid New must stop its managed loopback sender');
  checks.push({ check: 'native-valid-new-retires-managed-output-and-publishes-empty-project', passed: true });
}
