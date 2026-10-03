import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import path from 'node:path';
import { createHash, randomUUID } from 'node:crypto';

// Retire the automatic bridge renderer through its existing native registration
// boundary. Real authenticated MCP admission, immutable claim and native execute
// remain intact; withholding broker completion makes crash stages deterministic.
export async function nativeControllerInFlightProbe(h) {
  const digest = bytes => createHash('sha256').update(bytes).digest('hex');
  const exists = file => fs.stat(file).then(() => true, error => {
    if (error.code === 'ENOENT') return false;
    throw error;
  });
  for (const family of ['output', 'file']) {
    for (const stage of ['queued', 'claimed', 'native_committed']) {
      const label = `${family}-${stage}`;
      const before = await h.state();
      assert.equal(before.master, 1);
      assert.equal(before.ownership.lighting_allowed, true);
      const active = (await h.query('syndocal.output.lease.authority.query.v1')).statuses
        .find(entry => entry.status === 'held_active')?.authority;
      assert.ok(active);
      const principal = h.principal();
      const descriptor = h.backend().descriptor;
      const { fence } = await h.backend().invoke('query_output_control_authority_v1');
      await h.grant('file', 'syndocal.diagnostics.export.v1');
      const rendererGeneration = await h.backend().invoke('agent_bridge_register_v1');
      assert.ok(Number.isSafeInteger(rendererGeneration) && rendererGeneration > 0);
      const requestId = randomUUID();
      const destination = path.join(h.directory, `${label}.zip`);
      assert.equal(await exists(destination), false);
      const tool = family === 'output' ? 'syndocal_execute_control_plane' : 'syndocal_export_diagnostics';
      const operationId = family === 'output' ? 'syndocal.output.lighting.master.set.v2' : 'syndocal.diagnostics.export.v1';
      const params = family === 'output' ? { operationId, request: { request: {
        operation_id: operationId, request_id: h.nextOutputRequestId(), expected_fence: fence,
        action: { kind: 'set_lighting_master', role: 'lighting', master_milliunits: 500, lease: active },
      } } } : { destination };
      const admission = await h.stable(`${label}-admitted-full`, h.full, async () => {
        const admitted = await h.call(tool, { requestId, ...params });
        assert.equal(admitted.status, 'pending');
        assert.equal(admitted.requestId, requestId);
        assert.equal(admitted.result, undefined);
        if (stage !== 'queued') {
          const claimed = await h.backend().invoke('agent_bridge_claim_v1', { rendererGeneration, requestId });
          assert.equal(claimed.principalId, principal.principalId);
          assert.equal(claimed.principalIncarnation, principal.principalIncarnation);
          assert.equal(claimed.method, family === 'output' ? 'control_plane.execute' : 'diagnostics.export');
          assert.deepEqual(claimed.params, params);
        }
        assert.deepEqual(await h.state(), before);
        assert.equal(await exists(destination), false);
      });

      let exportBytes;
      if (stage === 'native_committed') {
        const result = await h.backend().invoke('agent_bridge_execute_native_v1', { rendererGeneration, requestId });
        assert.equal(result.ok, true);
        assert.equal(result.operation_id, operationId);
        if (family === 'output') {
          assert.equal(result.result.type, 'receipt');
          await h.awaitImage(h.half);
          assert.deepEqual(await h.state(), { ...before, master: 0.5 });
        } else {
          exportBytes = await fs.readFile(destination);
          assert.equal(result.sha256, digest(exportBytes));
          assert.equal(result.bytes, exportBytes.length);
          assert.equal(exportBytes.readUInt32LE(0), 0x04034b50);
          assert.deepEqual(await h.state(), before);
        }
        // Native execute returns its domain result before the renderer calls
        // agent_bridge_complete_v1. Never publish that broker completion here.
        const again = await h.backend().evaluate(`window.__TAURI_INTERNALS__.invoke('agent_bridge_execute_native_v1',
          ${JSON.stringify({ rendererGeneration, requestId })}).then(value=>({value}),error=>({error:String(error)}))`);
        assert.deepEqual(again, { error: 'request_not_executable' });
      }
      const expected = family === 'output' && stage === 'native_committed' ? h.half : h.full;
      const pending = await h.stable(`${label}-pending-before-crash`, expected, async () => {
        const status = await h.status(requestId);
        assert.equal(status.status, 'pending', 'The external broker must still have no terminal reply');
        assert.equal(status.result, undefined);
        if (exportBytes) assert.ok((await fs.readFile(destination)).equals(exportBytes));
        else assert.equal(await exists(destination), false);
      });
      h.checks.push({ check: `native-inflight-${label}-immutable-admission-and-unconfirmed-broker-state`,
        passed: true, requestId, rendererGeneration, operationId,
        admittedPhase: admission.observation, ...pending.observation,
        ...(exportBytes ? { exportSha256: digest(exportBytes), exportBytes: exportBytes.length } : {}) });

      await h.closeStdio();
      let stoppedCount;
      await h.restart(async () => {
        await h.pause(250);
        stoppedCount = h.packets();
        await h.pause(500);
        h.healthy();
        assert.equal(h.packets(), stoppedCount);
      });
      const freshDescriptor = h.backend().descriptor;
      assert.notEqual(freshDescriptor.instanceId, descriptor.instanceId);
      const ownership = await h.ownership();
      assert.equal(ownership.lighting_allowed, false);
      assert.equal(ownership.video_allowed, false);
      assert.equal(h.packets(), stoppedCount);
      await h.backend().invoke('register_project_transaction_owner', { ownerId: h.ownerId });
      await h.openStdio();
      await h.promote();
      for (const operationId of ['syndocal.query.output.control.authority.v1', 'syndocal.output.lease.authority.query.v1']) {
        await h.grant('read', operationId);
      }
      for (const operationId of ['syndocal.output.lease.acquire.v2', 'syndocal.output.ownership.arm.v2', 'syndocal.output.lighting.master.set.v2']) {
        await h.grant('output', operationId);
      }
      await h.grant('file', 'syndocal.diagnostics.export.v1');
      assert.deepEqual((await h.query('syndocal.output.lease.authority.query.v1')).statuses, [{ status: 'unavailable' }]);
      const freshFence = (await h.query('syndocal.query.output.control.authority.v1')).fence;
      assert.notEqual(freshFence.process_incarnation, fence.process_incarnation);
      assert.notEqual(freshFence.session_incarnation, fence.session_incarnation);
      h.checks.push({ check: `native-inflight-${label}-actual-crash-new-process-authority-and-closed-gates`, passed: true,
        terminatedProcessId: descriptor.processId, restartedProcessId: freshDescriptor.processId, packetsAtStop: stoppedCount });

      const hidden = await h.status(requestId);
      assert.equal(hidden.status, 'unknown');
      assert.equal(hidden.result, undefined);
      const replay = await h.call(tool, { requestId, ...params });
      if (h.principal().principalIncarnation === principal.principalIncarnation) {
        assert.equal(replay.status, 'unknown', 'Same immutable persisted shape must not be dispatched again');
        assert.equal(replay.result, undefined);
      } else {
        // The persisted shape includes the authenticated principal incarnation.
        assert.equal(replay.status, 'rejected');
        assert.equal(replay.error, 'request_conflict');
      }
      const conflict = structuredClone(params);
      if (family === 'output') conflict.request.request.action.master_milliunits = 750;
      else conflict.destination = path.join(h.directory, `${label}-changed.zip`);
      const conflicted = await h.call(tool, { requestId, ...conflict });
      assert.equal(conflicted.status, 'rejected');
      assert.equal(conflicted.error, 'request_conflict');
      if (family === 'output') h.rejected(await h.send(params), 'stale_fence');
      if (exportBytes) assert.ok((await fs.readFile(destination)).equals(exportBytes));
      else assert.equal(await exists(destination), false);
      if (family === 'file') assert.equal(await exists(conflict.destination), false);
      assert.equal(h.packets(), stoppedCount, 'Old pending intent or replay must not resume output');
      h.checks.push({ check: `native-inflight-${label}-unknown-is-not-retried-exact-replay-fenced-and-shape-conflict`,
        passed: true, lookupStatus: hidden.status, exactReplayStatus: replay.status, filePreserved: Boolean(exportBytes),
        destinationAbsent: !exportBytes });

      await h.load(h.file);
      const restored = await h.state();
      assert.equal(restored.master, 1);
      assert.equal(restored.ownership.lighting_allowed, false);
      assert.equal(restored.ownership.video_allowed, false);
      const oldLease = await h.output('syndocal.output.lighting.master.set.v2', {
        kind: 'set_lighting_master', role: 'lighting', master_milliunits: 0, lease: active,
      });
      if (oldLease.receipt.result?.result?.rejection?.error !== 'forbidden') {
        console.log(JSON.stringify({ diagnostic: `${label}-unclaimed-old-lease`, ...oldLease }));
      }
      h.rejected(oldLease.receipt);
      const current = h.accepted((await h.output('syndocal.output.lease.acquire.v2', { kind: 'acquire_lease', role: 'lighting' })).receipt)
        .lease_result.authority;
      await h.pause(500);
      h.healthy();
      assert.equal(h.packets(), stoppedCount, 'Reload and fresh acquisition cannot implicitly Arm');
      h.accepted((await h.output('syndocal.output.ownership.arm.v2', { kind: 'arm', role: 'lighting', lease: current })).receipt);
      await h.awaitImage(h.full);
      const rearmed = await h.stable(`${label}-separate-explicit-rearm-full`, h.full);
      assert.deepEqual(await h.state(), { ...restored, ownership: await h.ownership() });
      assert.equal((await h.ownership()).lighting_allowed, true);
      assert.equal((await h.ownership()).video_allowed, false);
      if (exportBytes) assert.ok((await fs.readFile(destination)).equals(exportBytes));
      h.checks.push({ check: `native-inflight-${label}-fresh-project-and-lease-stay-silent-until-separate-explicit-arm`,
        passed: true, ...rearmed.observation });
    }
  }
}
