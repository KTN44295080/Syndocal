import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { createHash, randomUUID } from 'node:crypto';
import { openNativeStdioSession } from './native-stdio-session.mjs';

// Real native shell/GPU lifecycle on an individually owned, enabled Display
// in an empty QA project. No media, audio, active DMX route or fixture exists.
export async function nativeDisplayWindow(backend, options, checks) {
  const directory = await fs.mkdtemp(path.join(os.tmpdir(), 'syndocal-mcp-display-window-'));
  const ownerId = `display-window-${randomUUID()}`;
  let mcp;
  let numericRequestId = Date.now();
  let lease;
  const intents = new Map();
  const grant = (capability, operationId) => backend.invoke('agent_authority_grant_v1', {
    principalId: options.principalId, principalIncarnation: options.principalIncarnation,
    grant: { adapter: 'external_mcp', capability, operation_id: operationId, project_id: null },
  });
  const send = async (params, id = randomUUID()) => {
    let receipt = await mcp.call('syndocal_execute_control_plane', { requestId: id, ...params });
    const until = Date.now() + 15000;
    while (receipt.status === 'pending' && Date.now() < until) {
      await new Promise(resolve => setTimeout(resolve, 50));
      receipt = await mcp.call('syndocal_get_request_status', { requestId: id });
    }
    assert.equal(receipt.requestId, id);
    return receipt;
  };
  const query = async operationId => {
    const receipt = await send({ operationId, request: {} });
    assert.equal(receipt.status, 'completed');
    assert.equal(receipt.result.ok, true, JSON.stringify(receipt.result));
    return receipt.result.result;
  };
  const authority = () => query('syndocal.query.output.control.authority.v1');
  const observation = () => query('syndocal.query.video.output_window_observation.v1');
  const output = async (operationId, action, expectedFence = undefined) => {
    const fence = expectedFence ?? (await authority()).fence;
    const id = randomUUID();
    const params = { operationId, request: { request: {
      operation_id: operationId, request_id: numericRequestId++, expected_fence: fence, action,
    } } };
    intents.set(id, params);
    return send(params, id);
  };
  const accepted = receipt => {
    assert.equal(receipt.status, 'completed', JSON.stringify(receipt));
    assert.equal(receipt.result.ok, true, JSON.stringify(receipt.result));
    assert.equal(receipt.result.result.type, 'receipt');
    const value = receipt.result.result.receipt;
    if (value.lease_result?.authority) lease = value.lease_result.authority;
    return value;
  };
  const closed = value => {
    assert.equal(value.outputs.length, 1);
    assert.equal(value.outputs[0].output_id, '1');
    assert.equal(value.outputs[0].live_open, false);
    assert.equal(value.outputs[0].native_window_handle_decimal, null);
  };
  try {
    await backend.invoke('register_project_transaction_owner', { ownerId });
    const initial = await backend.invoke('get_project_authority_bundle');
    const checkpoint = await backend.invoke('get_project_checkpoint', { midiMappings: [], oscMappings: [], dmxMappings: [] });
    assert.deepEqual(checkpoint.snapshot.fixtures, []);
    assert.deepEqual(checkpoint.snapshot.video.layers, []);
    assert.deepEqual(checkpoint.snapshot.video.media_assets ?? [], []);
    assert.deepEqual(checkpoint.snapshot.video.outputs, []);
    assert.deepEqual(checkpoint.snapshot.timeline.audio_clips ?? [], []);
    assert.equal(checkpoint.snapshot.timeline.playing, false);
    // Fresh project defaults can stage an enabled DMX config behind denied
    // ownership. This owned fixture explicitly disables it before any Arm.
    const disabledRoute = { enabled: false, protocol: 'ArtNet', target_ip: '127.0.0.1', port: 65221,
      universe: 0, serial_port: '', serial_baud_rate: 57600 };
    checkpoint.snapshot.output = { ...disabledRoute };
    checkpoint.snapshot.dmx_outputs = [{ ...disabledRoute }];
    const monitors = await backend.invoke('list_video_display_monitors');
    const targets = monitors.filter(monitor => monitor.isEditorMonitor);
    assert.equal(targets.length, 1, 'Resolve exactly the QA editor monitor');
    const monitor = targets[0];
    assert.match(monitor.identity, /^[a-f0-9]{64}$/);
    // The native presentation contract requires an enabled Display even when
    // its composition is empty. Window existence stays independent of that
    // authored enablement; all DMX routes remain explicitly disabled.
    checkpoint.snapshot.video.outputs = [{ id: 1, label: 'MCP Display QA', kind: 'Display', enabled: true,
      composition_id: 1, fullscreen: false, monitor_id: monitor.index, monitor_identity: monitor.identity,
      width: 320, height: 180, endpoint_name: null, opacity: 1, blackout: false }];
    const composition = checkpoint.snapshot.video.compositions.find(value => value.id === 1);
    assert.ok(composition);
    assert.deepEqual(composition.layer_ids, []);
    composition.output_ids = [1];
    const file = path.join(directory, 'closed-display.sdc');
    const bytes = Buffer.from(JSON.stringify(checkpoint));
    await fs.writeFile(file, bytes, { flag: 'wx' });
    const loadArgs = { path: file, ownerId, expectedEpoch: initial.project_epoch,
      expectedRevision: initial.project_revision, expectedCheckpointHash: initial.checkpoint_hash };
    const loaded = await backend.evaluate(`window.__TAURI_INTERNALS__.invoke('load_project_path', ${JSON.stringify(loadArgs)})
      .then(value=>({ok:true,value}),error=>({ok:false,error:String(error)}))`);
    assert.equal(loaded.ok, true, JSON.stringify(loaded));
    const authoredBefore = await backend.invoke('get_project_checkpoint', { midiMappings: [], oscMappings: [], dmxMappings: [] });
    assert.equal(authoredBefore.snapshot.output.enabled, false);
    assert.deepEqual(authoredBefore.snapshot.dmx_outputs, [disabledRoute]);
    mcp = await openNativeStdioSession(options);
    await grant('read', 'syndocal.query.output.control.authority.v1');
    await grant('read', 'syndocal.query.video.output_window_observation.v1');
    await backend.invoke('agent_authority_promote_v1', {
      principalId: options.principalId, principalIncarnation: options.principalIncarnation,
    });
    await grant('output', 'syndocal.output.lease.acquire.v2');
    await grant('output', 'syndocal.output.ownership.arm.v2');
    accepted(await output('syndocal.output.lease.acquire.v2', { kind: 'acquire_lease', role: 'both' }));
    const staleFence = (await authority()).fence;
    accepted(await output('syndocal.output.ownership.arm.v2', { kind: 'arm', role: 'both', lease }));
    const armed = await backend.invoke('get_output_ownership_status');
    assert.equal(armed.lighting_allowed, true);
    assert.equal(armed.video_allowed, true);
    closed(await observation());
    const operationId = 'syndocal.output.display.window.set_open.v2';
    const denied = await output(operationId, { kind: 'set_display_window_open', output_id: 1, open: true, lease });
    assert.equal(denied.status, 'rejected');
    assert.equal(denied.error, 'agent_missing_grant');
    closed(await observation());
    checks.push({ check: 'native-editor-display-exact-target-and-missing-grant-rejection', passed: true,
      target: { index: monitor.index, identity: monitor.identity, isEditorMonitor: monitor.isEditorMonitor },
      sourceSha256: createHash('sha256').update(bytes).digest('hex') });
    await grant('output', operationId);
    const stale = await output(operationId, { kind: 'set_display_window_open', output_id: 1, open: true, lease }, staleFence);
    assert.equal(stale.status, 'completed');
    assert.equal(stale.result.ok, false);
    assert.equal(stale.result.result.type, 'rejected');
    assert.equal(stale.result.result.rejection.error, 'stale_fence');
    closed(await observation());
    checks.push({ check: 'native-editor-display-stale-fence-rejected-before-shell', passed: true });
    const opening = { check: 'native-external-editor-display-open-without-human-confirmation', passed: false };
    checks.push(opening);
    const opened = await output(operationId, { kind: 'set_display_window_open', output_id: 1, open: true, lease });
    opening.receipt = opened;
    accepted(opened);
    const live = await observation();
    assert.equal(live.outputs.length, 1);
    assert.equal(live.outputs[0].output_id, '1');
    assert.equal(live.outputs[0].live_open, true);
    assert.match(live.outputs[0].native_window_handle_decimal, /^[1-9][0-9]*$/);
    opening.observation = live;
    opening.passed = true;
    assert.deepEqual(await send(intents.get(opened.requestId), opened.requestId), opened);
    assert.deepEqual(await observation(), live, 'Reply replay must keep the same exact native HWND');
    accepted(await output(operationId, { kind: 'set_display_window_open', output_id: 1, open: false, lease }));
    closed(await observation());
    accepted(await output(operationId, { kind: 'set_display_window_open', output_id: 1, open: true, lease }));
    const reopened = await observation();
    assert.equal(reopened.outputs[0].live_open, true);
    assert.match(reopened.outputs[0].native_window_handle_decimal, /^[1-9][0-9]*$/);
    accepted(await output(operationId, { kind: 'set_display_window_open', output_id: 1, open: false, lease }));
    closed(await observation());
    assert.deepEqual(await backend.invoke('get_project_checkpoint', { midiMappings: [], oscMappings: [], dmxMappings: [] }),
      authoredBefore, 'Arm/window shell lifecycle must not mutate the authored fixture');
    assert.deepEqual(await fs.readFile(file), bytes);
    checks.push({ check: 'native-external-editor-display-receipt-replay-close-reopen-and-authored-preservation', passed: true,
      initialHandle: live.outputs[0].native_window_handle_decimal, reopenedHandle: reopened.outputs[0].native_window_handle_decimal });
    const final = await backend.invoke('get_project_authority_bundle');
    await backend.invoke('new_project', { ownerId, expectedEpoch: final.project_epoch,
      expectedRevision: final.project_revision, expectedCheckpointHash: final.checkpoint_hash });
    const ownership = await backend.invoke('get_output_ownership_status');
    assert.equal(ownership.lighting_allowed, false);
    assert.equal(ownership.video_allowed, false);
    assert.deepEqual((await observation()).outputs, []);
    checks.push({ check: 'native-display-owned-shell-retired-empty-project-disarmed', passed: true, ownership });
  } finally {
    try { await mcp?.close(); }
    finally {
      assert.equal(path.dirname(path.resolve(directory)), path.resolve(os.tmpdir()));
      assert.ok(path.basename(directory).startsWith('syndocal-mcp-display-window-'));
      await fs.rm(directory, { recursive: true, force: true });
    }
  }
}
