import assert from 'node:assert/strict';
import { randomUUID } from 'node:crypto';

// Native process loss is distinct from lease owner retirement. The lifecycle
// runner owns exact-path termination/relaunch; the common output helper owns
// the receiver/project/stdio resources and independent 512-channel oracle.
export async function nativeControllerRestartProbe({ backend, restart, closeStdio, openStdio,
  grant, promote, query, output, send, status, accepted, rejected, load, file, ownerId,
  ownership, state, stable, awaitImage, full, half, pause, healthy, packets, checks }) {
  const beforeFence = (await query('syndocal.query.output.control.authority.v1')).fence;
  const beforeDescriptor = backend().descriptor;
  const activeStatus = await query('syndocal.output.lease.authority.query.v1');
  const active = activeStatus.statuses.find(entry => entry.status === 'held_active')?.authority;
  assert.ok(active, 'Live initial lease required');
  const changed = await output('syndocal.output.lighting.master.set.v2', {
    kind: 'set_lighting_master', role: 'lighting', master_milliunits: 500, lease: active,
  });
  accepted(changed.receipt);
  await awaitImage(half);
  const mastered = await state();
  assert.equal(mastered.master, 0.5);
  const live = await stable('pre-crash-half', half);
  checks.push({ check: 'pre-crash-live-native-master-half-image', passed: true, ...live.observation });
  const disconnected = await stable('stdio-disconnect-preserves-half', half, async () => {
    await closeStdio();
    assert.deepEqual(await state(), mastered);
  });
  checks.push({ check: 'live-stdio-disconnect-is-not-native-owner-or-output-loss', passed: true, ...disconnected.observation });

  let stoppedCount;
  await restart(async () => {
    // The runner has already verified this exact QA process is terminal. Allow
    // queued loopback datagrams to drain, then require real sender silence.
    await pause(250);
    stoppedCount = packets();
    await pause(500);
    healthy();
    assert.equal(packets(), stoppedCount, 'Terminated native process must stop its UDP sender');
  });
  checks.push({ check: 'live-native-process-termination-stops-loopback-sender', passed: true,
    terminatedProcessId: beforeDescriptor.processId, packetsAtStop: stoppedCount });

  const fresh = await ownership();
  const afterDescriptor = backend().descriptor;
  assert.notEqual(afterDescriptor.instanceId, beforeDescriptor.instanceId);
  assert.equal(fresh.lighting_allowed, false);
  assert.equal(fresh.video_allowed, false);
  assert.equal(packets(), stoppedCount, 'Native restart must not resume previous sender');
  // Reuse the owner text deliberately. New backend process/session authority,
  // not a renderer-selected string or a persisted desired role, owns the gate.
  await backend().invoke('register_project_transaction_owner', { ownerId });
  await openStdio();
  await grant('read', 'syndocal.query.output.control.authority.v1');
  await grant('read', 'syndocal.output.lease.authority.query.v1');
  await promote();
  for (const operationId of ['syndocal.output.lease.acquire.v2', 'syndocal.output.ownership.arm.v2',
    'syndocal.output.lighting.master.set.v2']) await grant('output', operationId);
  assert.deepEqual((await query('syndocal.output.lease.authority.query.v1')).statuses, [{ status: 'unavailable' }]);
  const freshFence = (await query('syndocal.query.output.control.authority.v1')).fence;
  assert.notEqual(freshFence.process_incarnation, beforeFence.process_incarnation);
  assert.notEqual(freshFence.session_incarnation, beforeFence.session_incarnation);
  await pause(500);
  healthy();
  assert.equal(packets(), stoppedCount);
  checks.push({ check: 'restart-same-owner-text-does-not-reclaim-lease-or-arm', passed: true,
    ownership: fresh, terminatedProcessId: beforeDescriptor.processId, restartedProcessId: afterDescriptor.processId,
    instanceIdChanged: true, processIncarnationChanged: true, sessionIncarnationChanged: true });

  const hidden = await status(changed.receipt.requestId);
  assert.equal(hidden.status, 'unknown');
  assert.equal(hidden.result, undefined);
  const replay = await send(changed.params, changed.receipt.requestId);
  // Ledger::new deliberately clears previous-process completion. The exact
  // persisted identity remains unknown and is never enqueued for execution.
  assert.equal(replay.status, 'unknown');
  assert.equal(replay.result, undefined);
  const conflict = structuredClone(changed.params);
  conflict.request.request.action.master_milliunits = 0;
  const changedReplay = await send(conflict, changed.receipt.requestId);
  assert.equal(changedReplay.status, 'rejected');
  assert.equal(changedReplay.error, 'request_conflict');
  // New broker identity cannot make a cached old process fence current.
  rejected(await send(changed.params, randomUUID()), 'stale_fence');
  checks.push({ check: 'restart-old-terminal-hidden-exact-replay-unknown-conflict-and-old-process-fence-rejects', passed: true });

  await load(file);
  const restored = await state();
  assert.equal(restored.ownership.lighting_allowed, false);
  assert.equal(restored.ownership.video_allowed, false);
  assert.equal(restored.master, 1, 'Explicit private source reload restores independently authored full master');
  assert.deepEqual((await query('syndocal.output.lease.authority.query.v1')).statuses, [{ status: 'unavailable' }]);
  rejected((await output('syndocal.output.ownership.arm.v2', { kind: 'arm', role: 'lighting', lease: active })).receipt);
  rejected((await output('syndocal.output.lighting.master.set.v2', {
    kind: 'set_lighting_master', role: 'lighting', master_milliunits: 0, lease: active,
  })).receipt);
  assert.deepEqual(await state(), restored);
  await pause(500);
  healthy();
  assert.equal(packets(), stoppedCount);
  checks.push({ check: 'restart-explicit-project-load-and-stale-lease-output-do-not-arm', passed: true, ownership: restored.ownership });

  const acquired = accepted((await output('syndocal.output.lease.acquire.v2', { kind: 'acquire_lease', role: 'lighting' })).receipt);
  const current = acquired.lease_result.authority;
  assert.deepEqual(await state(), restored);
  await pause(500);
  healthy();
  assert.equal(packets(), stoppedCount, 'Fresh acquisition is authority only');
  checks.push({ check: 'restart-new-exact-lease-acquisition-does-not-resume-output', passed: true, generation: current.generation });

  const rearmed = await output('syndocal.output.ownership.arm.v2', { kind: 'arm', role: 'lighting', lease: current });
  accepted(rearmed.receipt);
  await awaitImage(full);
  const resumed = await state();
  assert.equal(resumed.ownership.lighting_allowed, true);
  assert.equal(resumed.ownership.video_allowed, false);
  assert.equal(resumed.master, 1);
  assert.deepEqual(resumed.routes, restored.routes);
  assert.deepEqual(resumed.attributes, restored.attributes);
  const resumedLive = await stable('explicit-rearm-full', full, async () => {
    assert.deepEqual(await send(rearmed.params, rearmed.receipt.requestId), rearmed.receipt);
    assert.deepEqual(await state(), resumed);
  });
  assert.ok(packets() > stoppedCount, 'Re-Arm requires fresh packets, never a cached image');
  checks.push({ check: 'restart-new-explicit-r4-arm-resumes-full-live-dmx-and-replay-is-idempotent', passed: true,
    receipt: accepted(rearmed.receipt), ...resumedLive.observation });
}
