import assert from 'node:assert/strict';
import dgram from 'node:dgram';
import fs from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { createHash, randomUUID } from 'node:crypto';
import { performance } from 'node:perf_hooks';
import { openNativeStdioSession } from './native-stdio-session.mjs';
import { nativeControllerRestartProbe } from './native-controller-restart.mjs';
import { nativeControllerExpiryProbe } from './native-controller-expiry.mjs';
import { nativeControllerInFlightProbe } from './native-controller-inflight.mjs';
import { nativeControllerBurstProbe } from './native-controller-burst.mjs';
import { nativeControllerSafetyPressureProbe } from './native-controller-safety-pressure.mjs';

// Software loopback only: an owned ephemeral receiver, one isolated QA route,
// and real authenticated MCP operations. Retirement/transfer must preserve DMX.
export async function nativeControllerOutput(backend, options, checks, { restartLifecycle, leaseExpiry = false, inFlightCrash = false, burstRequests = false, safetyPressure } = {}) {
  assert.ok(!(restartLifecycle && leaseExpiry), 'Live restart and expiry require separate lanes');
  assert.ok(!inFlightCrash || restartLifecycle, 'In-flight crash requires the owned restart lifecycle');
  assert.ok(!burstRequests || !(restartLifecycle || leaseExpiry || inFlightCrash), 'Burst requires its own live request lane');
  assert.ok(!safetyPressure || (['saturation', 'kill_switch'].includes(safetyPressure)
    && !(restartLifecycle || leaseExpiry || inFlightCrash || burstRequests)), 'Safety pressure requires its own live lane');
  const directory = await fs.mkdtemp(path.join(os.tmpdir(), 'syndocal-controller-output-'));
  const receiver = dgram.createSocket('udp4');
  const ownerId = `controller-output-${randomUUID()}`;
  const digest = bytes => createHash('sha256').update(bytes).digest('hex');
  const pause = ms => new Promise(resolve => setTimeout(resolve, ms));
  const readOverloads = [];
  let numericRequestId = Date.now();
  let mcp;
  let failure;
  let registeredOwnerId = ownerId;
  let cleanupFile;
  let projectLoaded = false;
  let bound = false;
  let packetError;
  let latest;
  let packetCount = 0;
  let phase;
  const packetTimes = [];
  const zero = Buffer.alloc(512);
  const imageTimes = [];
  const full = Buffer.alloc(512);
  // Independently authored fixture: Dimmer 65535, Pan/Tilt 32768, all else 0.
  // EightBit writes the high byte; a 0.5 master rounds 65535*0.5 to 32768.
  full[0] = 255; full[4] = 128; full[6] = 128;
  const half = Buffer.from(full); half[0] = 128;
  receiver.on('error', error => { packetError ??= error; });
  receiver.on('message', (packet, source) => {
    try {
      assert.equal(source.address, '127.0.0.1');
      assert.equal(packet.length, 530);
      assert.equal(packet.subarray(0, 8).toString('binary'), 'Art-Net\0');
      assert.equal(packet.readUInt16LE(8), 0x5000);
      assert.equal(packet.readUInt16BE(10), 14);
      assert.equal(packet[13], 0);
      assert.equal(packet.readUInt16LE(14), 0);
      assert.equal(packet.readUInt16BE(16), 512);
      latest = Buffer.from(packet.subarray(18));
      packetCount++;
      if (burstRequests && packetTimes.length < 10000) packetTimes.push(performance.now());
      if (safetyPressure && imageTimes.length < 10000) imageTimes.push({ at: performance.now(), zero: latest.equals(zero) });
      if (phase) {
        assert.ok(latest.equals(phase.expected), `${phase.name}: unexpected 512-channel image`);
        phase.frames++;
      }
    } catch (error) { packetError ??= error; }
  });
  const healthy = () => { if (packetError) throw packetError; };
  const stable = async (name, expected, action = async () => {}) => {
    healthy();
    assert.ok(latest?.equals(expected), `${name}: starting image`);
    const observation = { name, expected, frames: 0 };
    phase = observation;
    try {
      const result = await action();
      await pause(500);
      healthy();
      assert.ok(observation.frames >= 10, `${name}: live sender must continue, not merely retain a cached frame`);
      return { result, observation: { phase: name, frames: observation.frames,
        payloadSha256: digest(expected), channels1to8: [...expected.subarray(0, 8)] } };
    } finally { phase = undefined; }
  };
  const awaitImage = async expected => {
    const until = Date.now() + 5000;
    while (Date.now() < until) {
      healthy();
      if (latest?.equals(expected)) return;
      await pause(20);
    }
    assert.fail(`No expected ArtDMX image; received ${packetCount} packets, latest ${latest?.subarray(0, 8).toString('hex')}`);
  };
  const authority = () => backend.invoke('get_project_authority_bundle');
  const checkpoint = () => backend.invoke('get_project_checkpoint', { midiMappings: [], oscMappings: [], dmxMappings: [] });
  const ownership = () => backend.invoke('get_output_ownership_status');
  const load = async file => {
    const bundle = await authority();
    const args = { path: file, ownerId: registeredOwnerId, expectedEpoch: bundle.project_epoch,
      expectedRevision: bundle.project_revision, expectedCheckpointHash: bundle.checkpoint_hash };
    const result = await backend.evaluate(`window.__TAURI_INTERNALS__.invoke('load_project_path', ${JSON.stringify(args)})
      .then(value=>({ok:true,value}),error=>({ok:false,error:String(error)}))`);
    assert.equal(result.ok, true, JSON.stringify(result));
  };
  const state = async () => {
    const project = await checkpoint();
    return { ownership: await ownership(), output: project.snapshot.output,
      routes: project.snapshot.dmx_outputs, master: project.snapshot.lighting_master,
      blackout: project.snapshot.blackout, safety: project.snapshot.safety_blackout_engaged,
      attributes: project.snapshot.fixtures[0].attribute_values };
  };
  const grant = (capability, operationId) => backend.invoke('agent_authority_grant_v1', {
    principalId: options.principalId, principalIncarnation: options.principalIncarnation,
    grant: { adapter: 'external_mcp', capability, operation_id: operationId, project_id: null },
  });
  const send = async (params, requestId = randomUUID()) => {
    let receipt = await mcp.call('syndocal_execute_control_plane', { requestId, ...params });
    const until = Date.now() + 15000;
    while (receipt.status === 'pending' && Date.now() < until) {
      await pause(100);
      receipt = await mcp.call('syndocal_get_request_status', { requestId });
    }
    assert.notEqual(receipt.status, 'pending', 'An uncertain mutation must not be replayed');
    return receipt;
  };
  const query = async operationId => {
    for (let attempt = 0; attempt < 3; attempt++) {
      const receipt = await send({ operationId, request: {} });
      assert.equal(receipt.status, 'completed');
      const error = receipt.result.error;
      const typed = error?.native_query?.code === 'overloaded'
        && error.native_query.retryable === true && error.native_query.resnapshot_required === false;
      const fence = /QueryError \{ code: Overloaded,/.test(error?.message ?? '')
        && /retryable: true/.test(error?.message ?? '');
      if (receipt.result.ok === false && error?.code === 'request_rejected' && (typed || fence) && attempt < 2) {
        readOverloads.push({ operationId, requestId: receipt.requestId, error });
        await pause(100);
        continue; // New read intent after terminal Overloaded only, never a mutation.
      }
      assert.equal(receipt.result.ok, true, JSON.stringify(receipt.result));
      assert.equal(receipt.result.operation_id, operationId);
      return receipt.result.result;
    }
    assert.fail('Read attempt bound');
  };
  const output = async (operationId, action) => {
    const { fence } = await query('syndocal.query.output.control.authority.v1');
    const params = { operationId, request: { request: { operation_id: operationId,
      request_id: numericRequestId++, expected_fence: fence, action } } };
    return { params, receipt: await send(params) };
  };
  const accepted = receipt => {
    assert.equal(receipt.status, 'completed');
    assert.equal(receipt.result.ok, true, JSON.stringify(receipt.result));
    assert.equal(receipt.result.result.type, 'receipt', JSON.stringify(receipt.result));
    return receipt.result.result.receipt;
  };
  const rejected = (receipt, error = 'forbidden') => {
    assert.equal(receipt.status, 'completed');
    assert.equal(receipt.result.ok, false, JSON.stringify(receipt.result));
    assert.equal(receipt.result.result.type, 'rejected', JSON.stringify(receipt.result));
    assert.equal(receipt.result.result.rejection.error, error);
  };
  try {
    await new Promise((resolve, reject) => {
      receiver.once('error', reject);
      receiver.bind(0, '127.0.0.1', () => { receiver.removeListener('error', reject); bound = true; resolve(); });
    });
    const address = receiver.address();
    assert.equal(address.address, '127.0.0.1');
    assert.ok(address.port > 0);
    await backend.invoke('register_project_transaction_owner', { ownerId });
    const project = await checkpoint();
    const empty = structuredClone(project);
    assert.deepEqual(project.snapshot.fixtures, []);
    assert.equal((await ownership()).lighting_allowed, false);
    assert.equal((await ownership()).video_allowed, false);
    const fixtureSource = await fs.readFile(new URL('../../qa/migration/authored-control-project.json', import.meta.url));
    const fixture = JSON.parse(fixtureSource);
    project.custom_profiles = fixture.custom_profiles;
    project.snapshot.fixtures = [fixture.snapshot.fixtures[0]];
    project.snapshot.fixtures[0].group_ids = [];
    project.snapshot.fixtures[0].attribute_values.find(value => value.attribute === 'Dimmer').value = 65535;
    project.snapshot.lighting_master = 1;
    const route = { enabled: true, protocol: 'ArtNet', target_ip: '127.0.0.1', port: address.port,
      universe: 0, serial_port: '', serial_baud_rate: 57600 };
    project.snapshot.output = { ...route };
    project.snapshot.dmx_outputs = [{ ...route }];
    empty.snapshot.output = { ...route, enabled: false };
    empty.snapshot.dmx_outputs = [{ ...route, enabled: false }];
    cleanupFile = path.join(directory, 'empty-disabled.sdc');
    await fs.writeFile(cleanupFile, JSON.stringify(empty), { flag: 'wx' });
    assert.deepEqual(project.snapshot.effects, []);
    assert.deepEqual(project.snapshot.node_graphs, []);
    assert.deepEqual(project.snapshot.video.outputs, []);
    assert.deepEqual(project.snapshot.video.layers, []);
    assert.deepEqual(project.snapshot.timeline.audio_clips, []);
    assert.equal(project.snapshot.timeline.playing, false);
    const file = path.join(directory, 'loopback.sdc');
    const bytes = Buffer.from(JSON.stringify(project));
    await fs.writeFile(file, bytes, { flag: 'wx' });
    await load(file);
    projectLoaded = true;
    await pause(500);
    const loaded = await state();
    assert.deepEqual(loaded.routes, [route]);
    assert.equal(loaded.master, 1);
    assert.equal(loaded.blackout, false);
    assert.equal(loaded.safety, false);
    assert.equal(loaded.ownership.lighting_allowed, false);
    assert.equal(loaded.ownership.video_allowed, false);
    assert.equal(packetCount, 0);
    checks.push({ check: 'native-loopback-project-load-does-not-arm', passed: true,
      fixtureSourceSha256: digest(fixtureSource), projectSha256: digest(bytes), destination: address,
      ownership: loaded.ownership, packets: packetCount });

    mcp = await openNativeStdioSession(options);
    await grant('read', 'syndocal.query.output.control.authority.v1');
    await grant('read', 'syndocal.output.lease.authority.query.v1');
    const denied = await output('syndocal.output.lease.acquire.v2', { kind: 'acquire_lease', role: 'lighting' });
    assert.equal(denied.receipt.status, 'rejected');
    assert.equal(denied.receipt.error, 'agent_safe_mode_denied');
    await backend.invoke('agent_authority_promote_v1', {
      principalId: options.principalId, principalIncarnation: options.principalIncarnation,
    });
    const missing = await output('syndocal.output.lease.acquire.v2', { kind: 'acquire_lease', role: 'lighting' });
    assert.equal(missing.receipt.status, 'rejected');
    assert.equal(missing.receipt.error, 'agent_missing_grant');
    for (const operationId of ['syndocal.output.lease.acquire.v2', 'syndocal.output.lease.force_transfer.v2',
      'syndocal.output.lease.relinquish.v2', 'syndocal.output.ownership.arm.v2',
      'syndocal.output.lighting.master.set.v2']) await grant('output', operationId);
    const acquireStarted = performance.now();
    const acquired = accepted((await output('syndocal.output.lease.acquire.v2', { kind: 'acquire_lease', role: 'lighting' })).receipt);
    const active = acquired.lease_result.authority;
    assert.equal(packetCount, 0);
    checks.push({ check: 'external-loopback-exact-grant-safe-mode-and-acquire-without-output', passed: true,
      generation: active.generation });

    const arm = accepted((await output('syndocal.output.ownership.arm.v2', { kind: 'arm', role: 'lighting', lease: active })).receipt);
    await awaitImage(full);
    const armed = await state();
    assert.equal(armed.ownership.lighting_allowed, true);
    assert.equal(armed.ownership.video_allowed, false);
    const live = await stable('armed-full', full);
    checks.push({ check: 'external-r4-arm-without-dialog-produces-received-live-artdmx', passed: true,
      receipt: arm, state: armed, ...live.observation });

    const finish = async () => {
      // Native project replacement explicitly disarms runtime output. The
      // retired raw role route cannot be used, even for QA cleanup.
      await load(cleanupFile);
      projectLoaded = false;
      const stopped = await ownership();
      assert.equal(stopped.lighting_allowed, false);
      assert.equal(stopped.video_allowed, false);
      await pause(250);
      const count = packetCount;
      await pause(500);
      healthy();
      assert.equal(packetCount, count, 'Explicit project replacement must stop the owned UDP sender');
      assert.equal(digest(await fs.readFile(file)), digest(bytes));
      checks.push({ check: 'explicit-native-project-replacement-stops-owned-loopback-sender-and-source-is-unchanged', passed: true,
        ownership: stopped, packetsTotal: packetCount, readOverloads });
    };
    if (safetyPressure) {
      await nativeControllerSafetyPressureProbe({ backend: () => backend,
        principal: () => ({ principalId: options.principalId, principalIncarnation: options.principalIncarnation }),
        grant, state, stable, awaitImage, full, zero, active, directory, checks,
        imageTimes: () => imageTimes, nextOutputRequestId: () => numericRequestId++,
        call: (name, args) => mcp.call(name, args) }, safetyPressure);
      await finish();
      return;
    }
    if (burstRequests) {
      await nativeControllerBurstProbe({ grant, query, output, send, accepted, state, stable,
        awaitImage, full, half, pause, active, packetTimes: () => packetTimes,
        nextOutputRequestId: () => numericRequestId++, rpc: (method, params) => mcp.rpc(method, params),
        call: (name, args) => mcp.call(name, args), checks });
      await finish();
      return;
    }
    if (leaseExpiry) {
      await nativeControllerExpiryProbe({ grant, query, output, send, accepted, rejected,
        state, stable, awaitImage, full, half, pause, healthy, active, armed, acquireStarted,
        packets: () => packetCount, checks });
      await finish();
      return;
    }
    if (restartLifecycle) {
      const probe = inFlightCrash ? nativeControllerInFlightProbe : nativeControllerRestartProbe;
      await probe({ backend: () => backend,
        restart: async observeStopped => { backend = await restartLifecycle(observeStopped); },
        closeStdio: async () => { await mcp.close(); mcp = undefined; },
        openStdio: async () => { mcp = await openNativeStdioSession(options); },
        status: requestId => mcp.call('syndocal_get_request_status', { requestId }),
        call: (name, args) => mcp.call(name, args),
        nextOutputRequestId: () => numericRequestId++,
        principal: () => ({ principalId: options.principalId, principalIncarnation: options.principalIncarnation }),
        promote: () => backend.invoke('agent_authority_promote_v1', {
          principalId: options.principalId, principalIncarnation: options.principalIncarnation,
        }),
        grant, query, output, send, accepted, rejected, load, file, ownerId, directory,
        ownership, state, stable, awaitImage, full, half, pause, healthy,
        packets: () => packetCount, checks });
      await finish();
      return;
    }

    const retired = await stable('owner-retirement-preserves-full', full, async () => {
      registeredOwnerId = `replacement-${randomUUID()}`;
      await backend.invoke('register_project_transaction_owner', { ownerId: registeredOwnerId });
      assert.deepEqual((await query('syndocal.output.lease.authority.query.v1')).statuses, [{ status: 'unavailable' }]);
      assert.deepEqual(await state(), armed);
    });
    checks.push({ check: 'registered-native-owner-retirement-preserves-live-image-and-gates', passed: true, ...retired.observation });

    const stale = await stable('retired-lease-rejected-preserves-full', full, async () => {
      rejected((await output('syndocal.output.lighting.master.set.v2', {
        kind: 'set_lighting_master', role: 'lighting', master_milliunits: 0, lease: active,
      })).receipt);
      rejected((await output('syndocal.output.lease.force_transfer.v2', { kind: 'force_transfer_lease', lease: active })).receipt, 'forbidden');
      assert.deepEqual(await state(), armed);
    });
    checks.push({ check: 'retired-generation-output-and-transfer-reject-with-no-live-delta', passed: true, ...stale.observation });

    const orphaned = { ...active, generation: active.generation + 1 };
    let transferred;
    const transfer = await stable('force-transfer-preserves-full', full, async () => {
      transferred = await output('syndocal.output.lease.force_transfer.v2', { kind: 'force_transfer_lease', lease: orphaned });
      const committed = accepted(transferred.receipt);
      assert.equal(committed.lease_result.authority.generation, orphaned.generation + 1);
      assert.deepEqual(await send(transferred.params, transferred.receipt.requestId), transferred.receipt);
      const conflict = structuredClone(transferred.params);
      conflict.request.request.action.lease.generation++;
      const conflicted = await send(conflict, transferred.receipt.requestId);
      assert.equal(conflicted.status, 'rejected');
      assert.equal(conflicted.error, 'request_conflict');
      assert.deepEqual(await state(), armed);
    });
    const current = accepted(transferred.receipt).lease_result.authority;
    checks.push({ check: 'external-r4-force-transfer-replay-and-conflict-preserve-live-image', passed: true,
      generationBefore: orphaned.generation, generationAfter: current.generation, ...transfer.observation });

    const changed = await output('syndocal.output.lighting.master.set.v2', {
      kind: 'set_lighting_master', role: 'lighting', master_milliunits: 500, lease: current,
    });
    accepted(changed.receipt);
    await awaitImage(half);
    const mastered = await state();
    assert.equal(mastered.master, 0.5);
    assert.deepEqual(mastered.ownership, armed.ownership);
    assert.deepEqual(mastered.routes, armed.routes);
    assert.deepEqual(mastered.attributes, armed.attributes);
    const replay = await stable('explicit-half-and-replay', half, async () => {
      assert.deepEqual(await send(changed.params, changed.receipt.requestId), changed.receipt);
      rejected((await output('syndocal.output.lighting.master.set.v2', {
        kind: 'set_lighting_master', role: 'lighting', master_milliunits: 0, lease: active,
      })).receipt);
      assert.deepEqual(await state(), mastered);
    });
    checks.push({ check: 'new-owner-explicit-r4-master-changes-live-dmx-once-old-generation-rejects', passed: true,
      master: mastered.master, ...replay.observation });

    const relinquished = await stable('relinquish-preserves-half', half, async () => {
      accepted((await output('syndocal.output.lease.relinquish.v2', { kind: 'relinquish_output_lease', lease: current })).receipt);
      assert.deepEqual((await query('syndocal.output.lease.authority.query.v1')).statuses, [{ status: 'unavailable' }]);
      rejected((await output('syndocal.output.lighting.master.set.v2', {
        kind: 'set_lighting_master', role: 'lighting', master_milliunits: 0, lease: current,
      })).receipt);
      assert.deepEqual(await state(), mastered);
    });
    checks.push({ check: 'lease-relinquishment-preserves-live-image-and-rejects-subsequent-output', passed: true, ...relinquished.observation });

    await finish();
  } catch (error) { failure = error; throw error; }
  finally {
    // Even a failed probe must disarm its own QA profile before returning to
    // the lifecycle runner, which terminates this exact isolated executable.
    try { if (projectLoaded) await load(cleanupFile); }
    catch (error) { if (!failure) throw error; }
    finally {
      try { await mcp?.close(); }
      finally {
        if (bound) await new Promise(resolve => receiver.close(resolve));
        else receiver.close();
        assert.equal(path.dirname(path.resolve(directory)), path.resolve(os.tmpdir()));
        assert.ok(path.basename(directory).startsWith('syndocal-controller-output-'));
        await fs.rm(directory, { recursive: true, force: true });
      }
    }
  }
}
