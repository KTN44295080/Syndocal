import assert from 'node:assert/strict';
import { performance } from 'node:perf_hooks';

// Production-selected monotonic TTL while the native sender is live. Expiry,
// renewal rejection, recovery and relinquishment change authority only.
export async function nativeControllerExpiryProbe(h) {
  for (const operationId of ['syndocal.output.lease.renew.v2', 'syndocal.output.lease.recover.v2']) {
    await h.grant('output', operationId);
  }
  const leases = () => h.query('syndocal.output.lease.authority.query.v1');
  const held = [{ status: 'held_active', authority: h.active, resources: ['lighting'] }];
  assert.deepEqual((await leases()).statuses, held);
  const observations = [];
  const expired = await h.stable('real-ttl-preserves-full', h.full, async () => {
    let hidden = false;
    // Do not shorten TTL, substitute a clock or send any renewal while waiting.
    while (performance.now() - h.acquireStarted < 75000) {
      h.healthy();
      const view = await leases();
      const elapsedMs = performance.now() - h.acquireStarted;
      const packets = h.packets();
      const previous = observations.at(-1);
      const newFrames = previous ? packets - previous.packets : null;
      if (previous) assert.ok(newFrames >= 10, 'Native sender must remain live at every TTL observation interval');
      observations.push({ elapsedMs, statuses: view.statuses, packets, newFrames });
      if (view.statuses.length === 1 && view.statuses[0].status === 'unavailable') {
        hidden = true;
        break;
      }
      assert.deepEqual(view.statuses, held, 'No implicit lease renewal or ownership change');
      await h.pause(5000);
    }
    assert.ok(hidden, 'Production lease must actually expire within the bounded real-time drill');
    assert.ok(observations.at(-1).elapsedMs >= 59000, 'The real production TTL must not be shortened');
    assert.deepEqual(await h.state(), h.armed, 'Expiry must preserve runtime gates and output state');
  });
  h.checks.push({ check: 'native-live-real-monotonic-ttl-expiry-preserves-complete-artdmx-image-and-gates',
    passed: true, observations, ...expired.observation });

  // R0 hides expired authority without mutating the registry. This explicit
  // failed renewal observes expiry and advances the orphan generation once.
  const orphaned = { ...h.active, generation: h.active.generation + 1 };
  const orphanedStatuses = [{ status: 'held_orphaned', authority: orphaned, resources: ['lighting'] }];
  let renewal;
  const rejectedRenewal = await h.stable('expired-renew-replay-preserves-full', h.full, async () => {
    renewal = await h.output('syndocal.output.lease.renew.v2', { kind: 'renew_lease', lease: h.active });
    h.rejected(renewal.receipt);
    assert.deepEqual((await leases()).statuses, orphanedStatuses);
    assert.deepEqual(await h.send(renewal.params, renewal.receipt.requestId), renewal.receipt);
    assert.deepEqual((await leases()).statuses, orphanedStatuses, 'Terminal replay cannot orphan twice');
    assert.deepEqual(await h.state(), h.armed);
  });
  h.checks.push({ check: 'native-live-expired-renew-rejection-and-terminal-replay-orphan-once-without-output-delta',
    passed: true, renewalRejection: renewal.receipt.result.result, orphaned,
    ...rejectedRenewal.observation });

  const stale = await h.stable('expired-generation-rejects-preserves-full', h.full, async () => {
    h.rejected((await h.output('syndocal.output.lease.renew.v2', { kind: 'renew_lease', lease: h.active })).receipt);
    h.rejected((await h.output('syndocal.output.ownership.arm.v2', { kind: 'arm', role: 'lighting', lease: h.active })).receipt);
    h.rejected((await h.output('syndocal.output.lighting.master.set.v2', {
      kind: 'set_lighting_master', role: 'lighting', master_milliunits: 0, lease: h.active,
    })).receipt);
    assert.deepEqual((await leases()).statuses, orphanedStatuses);
    assert.deepEqual(await h.state(), h.armed);
  });
  h.checks.push({ check: 'native-live-expired-generation-renew-arm-and-master-reject-with-no-image-or-generation-delta',
    passed: true, ...stale.observation });

  let recovery, current;
  const recovered = await h.stable('explicit-recovery-preserves-full', h.full, async () => {
    recovery = await h.output('syndocal.output.lease.recover.v2', { kind: 'recover_lease', lease: orphaned });
    current = h.accepted(recovery.receipt).lease_result.authority;
    assert.deepEqual(current, { ...h.active, generation: h.active.generation + 2 });
    const statuses = [{ status: 'held_active', authority: current, resources: ['lighting'] }];
    assert.deepEqual((await leases()).statuses, statuses);
    assert.deepEqual(await h.send(recovery.params, recovery.receipt.requestId), recovery.receipt);
    const conflict = structuredClone(recovery.params);
    conflict.request.request.action.lease.generation++;
    const conflicted = await h.send(conflict, recovery.receipt.requestId);
    assert.equal(conflicted.status, 'rejected');
    assert.equal(conflicted.error, 'request_conflict');
    assert.deepEqual((await leases()).statuses, statuses);
    assert.deepEqual(await h.state(), h.armed, 'Recovery cannot Arm or otherwise change physical state');
  });
  h.checks.push({ check: 'native-live-explicit-expired-lease-recovery-replay-and-conflict-preserve-full-image',
    passed: true, recoveredAuthority: current, ...recovered.observation });

  const changed = await h.output('syndocal.output.lighting.master.set.v2', {
    kind: 'set_lighting_master', role: 'lighting', master_milliunits: 500, lease: current,
  });
  h.accepted(changed.receipt);
  await h.awaitImage(h.half);
  const mastered = await h.state();
  assert.deepEqual(mastered, { ...h.armed, master: 0.5 });
  const explicit = await h.stable('recovered-owner-explicit-half-and-replay', h.half, async () => {
    assert.deepEqual(await h.send(changed.params, changed.receipt.requestId), changed.receipt);
    h.rejected((await h.output('syndocal.output.lighting.master.set.v2', {
      kind: 'set_lighting_master', role: 'lighting', master_milliunits: 0, lease: h.active,
    })).receipt);
    assert.deepEqual(await h.state(), mastered);
  });
  h.checks.push({ check: 'native-live-recovered-owner-separate-explicit-master-changes-dmx-once-and-expired-owner-rejects',
    passed: true, master: mastered.master, ...explicit.observation });

  const relinquished = await h.stable('recovered-relinquish-preserves-half', h.half, async () => {
    h.accepted((await h.output('syndocal.output.lease.relinquish.v2', {
      kind: 'relinquish_output_lease', lease: current,
    })).receipt);
    assert.deepEqual((await leases()).statuses, [{ status: 'unavailable' }]);
    h.rejected((await h.output('syndocal.output.lighting.master.set.v2', {
      kind: 'set_lighting_master', role: 'lighting', master_milliunits: 0, lease: current,
    })).receipt);
    assert.deepEqual(await h.state(), mastered);
  });
  h.checks.push({ check: 'native-live-recovered-lease-relinquishment-preserves-half-image-and-blocks-further-output',
    passed: true, ...relinquished.observation });
}
