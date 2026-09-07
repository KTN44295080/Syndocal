import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import ts from 'typescript';

const sourceText = async (relativePath) =>
  readFile(new URL(relativePath, import.meta.url), 'utf8');
const quoted = (source, pattern, quote) => {
  const match = source.match(pattern);
  assert.ok(match, `Canonical operation declaration was not found: ${pattern}`);
  return [...match[1].matchAll(new RegExp(`${quote}([^${quote}]+)${quote}`, 'g'))]
    .map((item) => item[1]);
};

const rustCanonicalIds = quoted(
  await sourceText('../src-tauri/src/agent_bridge_wire.rs'),
  /CANONICAL_OPERATION_IDS:\s*&\[&str\]\s*=\s*&\[\s*([\s\S]*?)\s*\];/,
  '"',
);
const typescriptCanonicalIds = quoted(
  await sourceText('../src/agentBridgeControlPlane.ts'),
  /CANONICAL_TAURI_COMMANDS\s*=\s*\{\s*([\s\S]*?)\s*\}\s*as const;/,
  '"',
).filter((id) => id.startsWith('syndocal.'));
const nodeCanonicalIds = quoted(
  await sourceText('../../tools/syndocal-mcp/server.mjs'),
  /const CANONICAL_OPERATION_IDS = new Set\(\[\s*([\s\S]*?)\s*\]\);/,
  "'",
);
assert.equal(rustCanonicalIds.length, 47);
assert.deepEqual([...typescriptCanonicalIds].sort(), [...rustCanonicalIds].sort());
assert.deepEqual([...nodeCanonicalIds].sort(), [...rustCanonicalIds].sort());

// Transpile actual production modules and resolve only their actual local dependency.
const modules = new Map();
for (const name of ['fixtureTransformConfirmation', 'outputControlController', 'agentBridgeBlackout', 'agentBridgeControlPlane', 'agentBridgeRecording', 'agentBridgeTools', 'agentBridgeRuntime']) {
  const source = await readFile(new URL(`../src/${name}.ts`, import.meta.url), 'utf8');
  const exports = {};
  const compiled = ts.transpileModule(source, {
    compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 },
    reportDiagnostics: true,
  });
  assert.equal(compiled.diagnostics?.filter(d => d.category === ts.DiagnosticCategory.Error).length, 0);
  new Function('require', 'exports', compiled.outputText)(dependency => {
    assert.ok(modules.has(dependency), `Unexpected production dependency: ${dependency}`);
    return modules.get(dependency);
  }, exports);
  modules.set(`./${name}`, exports);
}
const { executeAgentBridgeRequest: execute } = modules.get('./agentBridgeTools');
const { startAgentBridgeRuntime: start } = modules.get('./agentBridgeRuntime');
const token = { project_epoch: 4, project_revision: 9, checkpoint_hash: 'checkpoint-A' };
const fixture = { id: 17, label: 'Moving head', position: { x: 0, y: 1, z: 2 }, rotation: { pitch: 3, yaw: 4, roll: 5 } };
const desired = { position: { x: 10, y: 20, z: 30 }, rotation: { pitch: 45, yaw: 90, roll: 135 } };
const timelineRuntime = { transport_epoch: 4, transport_generation: 12, loop_runtime: { status: 'idle' }, follow_runtime: { epoch: 4, generation: 0 } };
const output = { id: 1, label: 'Main', enabled: true, composition_id: 9, width: 1920, height: 1080, mapping: { secret: 'must not be returned' } };
const ownership = { role: 'Both', effective_role: 'Both', desired_role: 'Both', persisted_role: 'Both', state: 'Ready', generation: 7, epoch: 8, lighting_allowed: true, video_allowed: true, lighting_reason: 'OwnedByMachineRole', video_reason: 'OwnedByMachineRole', error: null };
const request = (method, params = {}) => ({ rendererGeneration: 5, requestId: 'canonical-request', method, params });
const mutation = request('fixtures.set_transform', { fixtureId: fixture.id, ...desired, expectedProject: token });
const bundle = (item = fixture, project = token) => ({ ...project, timeline_runtime: structuredClone(timelineRuntime), snapshot: {
  fixtures: item ? [structuredClone(item)] : [],
  blackout: true,
  authored_blackout: false,
  safety_blackout_engaged: true,
  authored_video: { blackout: false },
  video: { blackout: true, outputs: Array.from({ length: 66 }, (_, index) => ({ ...output, id: index + 1, label: `Output ${index + 1}` })) },
  timeline: { id: 3, playing: true, position_ms: 1250, duration_ms: 5000 },
} });
let groups = 0;
groups++;

{
  const calls = [];
  const result = await execute(async (command, args) => { calls.push([command, args]); return bundle(); }, request('fixtures.get', { fixtureId: 17 }));
  assert.equal(result.ok, true);
  assert.deepEqual(result.fixture, fixture);
  assert.deepEqual(result.project, token);
  assert.deepEqual(calls, [['get_project_authority_bundle', {}]]);
  const runtimeCalls = [];
  const runtime = await execute(async (command, args) => {
    runtimeCalls.push([command, args]);
    if (command === 'get_project_authority_bundle') return bundle();
    if (command === 'get_output_ownership_status') return ownership;
    assert.fail(`Unexpected runtime call ${command}`);
  }, request('runtime.get'));
  assert.deepEqual(runtimeCalls, [['get_project_authority_bundle', {}], ['get_output_ownership_status', undefined]]);
  assert.equal(runtime.ok, true);
  assert.deepEqual(runtime.project, token);
  assert.deepEqual(runtime.timeline_runtime, timelineRuntime);
  assert.deepEqual(runtime.timeline, { id: 3, playing: true, position_ms: 1250, duration_ms: 5000 });
  assert.equal(runtime.blackout, true);
  assert.equal(runtime.authored_blackout, false);
  assert.equal(runtime.safety_blackout_engaged, true);
  assert.deepEqual(runtime.video.outputs[0], { id: 1, name: 'Output 1', enabled: true, composition_id: 9, dimensions: { width: 1920, height: 1080 } });
  assert.equal(runtime.video.outputs.length, 64);
  assert.equal(runtime.video.total, 66);
  assert.equal(runtime.video.truncated, true);
  assert.equal(runtime.video.authored_blackout, true);
  assert.deepEqual(runtime.observations.output_ownership_status, ownership);
  const registry = {
    schema: { name: 'syndocal.control-plane.canonical-operation-registry', version: 4 },
    canonical_operations: [{
      operation_id: 'syndocal.query.snapshot.v1',
      class: 'discovery',
      risk: 'r0',
      capabilities: ['read_only', 'registry_discovery'],
      request_schema: { name: 'syndocal.query.snapshot.v1.request', version: 1 },
      response_schema: { name: 'syndocal.query.snapshot.v1.response', version: 1 },
      idempotency: 'read_only',
      audit: 'not_applicable',
      adapter_policy: 'local_window_read_only',
      receipt_policy: 'fail_closed',
      rate_policy: 'fail_closed',
      payload_policy: 'fail_closed',
      consent_policy: 'fail_closed',
      derived_adapters: [{
        adapter: 'local_tauri_window', binding_id: 'tauri_command:get_snapshot',
        source_key: { family: 'tauri_command', source_id: 'get_snapshot' },
      }],
    }],
    source_inventory: [{
      source_key: { family: 'tauri_command', source_id: 'get_snapshot' },
      disposition: { kind: 'operation' },
    }],
  };
  const capabilities = await execute(async (command, args) => {
    assert.equal(command, 'get_control_plane_canonical_registry');
    assert.deepEqual(args, undefined);
    return registry;
  }, request('control_plane.get_capabilities'));
  assert.equal(capabilities.ok, true);
  assert.equal(capabilities.control_plane.canonical_operation_count, 1);
  assert.deepEqual(capabilities.control_plane.source_inventory_by_family, { tauri_command: 1 });
  assert.deepEqual(capabilities.agent_bridge.operations.slice(-3), ['control_plane.get_capabilities', 'recording.get_status', 'control_plane.execute']);
  const recordingStatus = await execute(async (command, args) => {
    assert.equal(command, 'video_output_recording_status');
    assert.deepEqual(args, undefined);
    return {
      active: true, output_id: 2, path: 'C:/recordings/take.mp4', width: 1920, height: 1080,
      frame_rate: 30, frames_written: 12, dropped_frames: 0, audio_requested: true,
      audio_included: true, audio_track_count: 1, started_unix_ms: 1234, last_error: null,
    };
  }, request('recording.get_status'));
  assert.equal(recordingStatus.ok, true);
  assert.equal(recordingStatus.recording.active, true);
  assert.equal(recordingStatus.recording.frames_written, 12);
  const canonical = await execute(async (command, args) => {
    assert.equal(command, 'get_control_plane_query_capabilities');
    assert.deepEqual(args, {});
    return { supported: true };
  }, request('control_plane.execute', {
    operationId: 'syndocal.query.control_plane.capabilities.v1',
    request: {},
  }));
  assert.deepEqual(canonical, {
    ok: true,
    operation_id: 'syndocal.query.control_plane.capabilities.v1',
    result: { supported: true },
  });
  const rejectedCanonical = await execute(async () => assert.fail('unreviewed canonical operation must not invoke Tauri'), request('control_plane.execute', {
    operationId: 'syndocal.query.not_reviewed.v1',
    request: {},
  }));
  assert.equal(rejectedCanonical.ok, false);
  assert.equal(rejectedCanonical.error.code, 'request_rejected');
  const failedRuntime = await execute(async (command) => {
    if (command === 'get_project_authority_bundle') return bundle();
    throw new Error('ownership observation unavailable');
  }, request('runtime.get'));
  assert.equal(failedRuntime.ok, false);
  assert.equal(failedRuntime.error.code, 'request_rejected');
  const list = await execute(async () => ({ ...token, snapshot: { fixtures: Array.from({ length: 258 }, (_, i) => ({ ...fixture, id: i + 1 })) } }), request('fixtures.list'));
  assert.equal(list.fixtures.length, 256); assert.equal(list.total, 258); assert.equal(list.truncated, true);
  groups++;
}
{
  const calls = [];
  const result = await execute(async (command, args) => {
    calls.push([command, args]);
    assert.equal(command, 'get_project_authority_bundle');
    assert.deepEqual(args, { expectedEpoch: 4, expectedRevision: 9, expectedCheckpointHash: 'checkpoint-A' });
    throw new Error('Stale project revision');
  }, mutation);
  assert.equal(result.ok, false); assert.equal(result.error.code, 'request_rejected');
  assert.equal(calls.length, 1, 'stale native preflight must reject before mutation');
  groups++;
}
{
  const calls = [];
  const after = { ...token, project_revision: 10, checkpoint_hash: 'checkpoint-B' };
  const result = await execute(async (command, args) => {
    calls.push([command, args]);
    if (calls.length === 1) return bundle();
    if (calls.length === 2) return undefined;
    return bundle({ ...fixture, ...desired }, after);
  }, mutation);
  assert.deepEqual(calls, [
    ['get_project_authority_bundle', { expectedEpoch: 4, expectedRevision: 9, expectedCheckpointHash: 'checkpoint-A' }],
    ['set_fixture_transform', { fixtureId: 17, ...desired, __expectedProjectEpoch: 4, __expectedProjectRevision: 9, __expectedCheckpointHash: 'checkpoint-A' }],
    ['get_project_authority_bundle', {}],
  ]);
  assert.equal(result.ok, true); assert.equal(result.verification, 'committed_project_state');
  assert.deepEqual(result.project, after); assert.deepEqual(result.fixture.rotation, desired.rotation);
  groups++;
}
{
  for (const [actual, project] of [[fixture, token], [null, token], [{ ...fixture, ...desired }, { ...token, project_epoch: 5 }]]) {
    let calls = 0;
    const result = await execute(async () => (++calls === 1 ? bundle() : calls === 2 ? undefined : bundle(actual, project)), mutation);
    assert.equal(result.ok, false); assert.equal(result.error.code, 'verification_failed');
    assert.equal(calls, 3);
  }
  for (const failingCall of [2, 3]) {
    let calls = 0;
    const result = await execute(async () => { calls++; if (calls === failingCall) throw new Error('ACK/read unavailable'); return bundle(); }, mutation);
    assert.equal(result.error.code, 'mutation_not_confirmed');
  }
  let calls = 0;
  assert.equal((await execute(async () => { calls++; }, request('arbitrary.invoke'))).error.code, 'unknown_method');
  assert.equal(calls, 0);
  groups++;
}

const outputToken = (revision, checkpointHash) => ({
  project_epoch: 4,
  project_revision: revision,
  checkpoint_hash: checkpointHash,
});
const outputFence = (project, publicationGeneration) => ({
  process_incarnation: 1,
  session_incarnation: 2,
  project_epoch: project.project_epoch,
  project_revision: project.project_revision,
  project_checkpoint_hash: project.checkpoint_hash,
  project_publication_generation: publicationGeneration,
  output_epoch: 6,
  output_generation: 7,
  safety_blackout_epoch: 8,
  safety_blackout_generation: 9,
});
const outputLease = { lease_id: 'lease-1111111111111111', generation: 3 };
const outputLeaseQuery = {
  operation_id: 'syndocal.output.lease.authority.query.v1',
  statuses: [{ status: 'held_active', authority: outputLease, resources: ['lighting', 'video'] }],
};
const outputBundle = (project, authoredBlackout) => {
  const value = bundle(null, project);
  delete value.snapshot.authored_video;
  value.snapshot.blackout = true;
  value.snapshot.safety_blackout_engaged = true;
  value.snapshot.video.blackout = authoredBlackout;
  return value;
};
const outputReceipt = (args, before, after) => ({
  type: 'receipt',
  receipt: {
    operation_id: 'syndocal.output.blackout.set.v2',
    request_id: args.request.request_id,
    shape_sha256: 'c'.repeat(64),
    argument_fingerprint: 'd'.repeat(64),
    audit_sequence: 14,
    fence_before: before,
    fence_after: after,
    outcome: 'applied',
    lease_result: {
      authority: outputLease,
      resources: ['lighting', 'video'],
      phase: 'held_active',
      outcome: 'authorized',
      audit_sequence: 15,
      changes: [{
        lease_id: outputLease.lease_id,
        before_generation: outputLease.generation,
        after_generation: outputLease.generation,
        before_resources: ['lighting', 'video'],
        after_resources: ['lighting', 'video'],
        before_phase: 'held_active',
        after_phase: 'held_active',
      }],
    },
  },
});
const runVideoBlackout = async ({ enabled, beforeProject, beforeAuthored, afterProject, afterFenceProject, afterAuthored = enabled, effects }) => {
  const beforeFence = outputFence(beforeProject, beforeProject.project_revision);
  const afterFence = outputFence(afterFenceProject, afterFenceProject.project_revision);
  afterFence.project_publication_generation = beforeFence.project_publication_generation + 1;
  const calls = [];
  const invoke = async (command, args) => {
    calls.push([command, args]);
    if (command === 'get_project_authority_bundle') {
      return args?.expectedEpoch === undefined
        ? outputBundle(afterProject, afterAuthored)
        : outputBundle(beforeProject, beforeAuthored);
    }
    if (command === 'query_output_lease_authority_v1') return structuredClone(outputLeaseQuery);
    if (command === 'query_output_control_authority_v1') {
      return { operation_id: 'syndocal.query.output.control.authority.v1', fence: structuredClone(beforeFence) };
    }
    if (command === 'set_blackout_output_control_v2') {
      assert.equal(args.request.action.target, 'video');
      assert.equal(args.request.action.enabled, enabled);
      assert.deepEqual(args.request.action.lease, outputLease);
      return outputReceipt(args, beforeFence, afterFence);
    }
    assert.fail(`Unexpected video blackout call ${command}`);
  };
  const result = await execute(invoke, request('output.set_video_blackout', {
    enabled,
    expectedProject: beforeProject,
  }), effects);
  return { result, calls, beforeFence, afterFence };
};

{
  const firstHookCalls = [];
  const first = await runVideoBlackout({
    enabled: true,
    beforeProject: outputToken(9, 'a'.repeat(64)),
    beforeAuthored: false,
    afterProject: outputToken(10, 'b'.repeat(64)),
    afterFenceProject: outputToken(10, 'b'.repeat(64)),
    effects: {
      refreshProjectAuthority: async receipt => { assert.equal(receipt.outcome, 'applied'); firstHookCalls.push('authority'); },
      refreshSnapshot: async () => { firstHookCalls.push('snapshot'); },
    },
  });
  assert.equal(first.result.ok, true);
  assert.equal(first.result.verification, 'committed_project_state');
  assert.deepEqual(first.result.project, outputToken(10, 'b'.repeat(64)));
  assert.equal(first.result.video.blackout, true);
  assert.equal(first.result.video.authored_blackout, true);
  assert.equal(first.result.video.safety_blackout_engaged, true);
  assert.equal(first.result.receipt.outcome, 'applied');
  assert.deepEqual(first.calls.map(([command]) => command), [
    'get_project_authority_bundle',
    'query_output_lease_authority_v1',
    'query_output_control_authority_v1',
    'query_output_lease_authority_v1',
    'set_blackout_output_control_v2',
    'get_project_authority_bundle',
  ]);
  assert.deepEqual(first.calls[4][1].request.expected_fence, first.beforeFence);
  assert.deepEqual(firstHookCalls, ['authority', 'snapshot']);

  const second = await runVideoBlackout({
    enabled: false,
    beforeProject: outputToken(10, 'b'.repeat(64)),
    beforeAuthored: true,
    afterProject: outputToken(11, 'e'.repeat(64)),
    afterFenceProject: outputToken(11, 'e'.repeat(64)),
    effects: {
      refreshProjectAuthority: async () => {},
      refreshSnapshot: async () => {},
    },
  });
  assert.equal(second.result.ok, true);
  assert.equal(second.result.video.blackout, false, 'the public video bit is the authored target');
  assert.equal(second.result.video.authored_blackout, false, 'authored video state is the requested target');
  assert.equal(second.result.video.safety_blackout_engaged, true, 'independent S0 state must not be used as video readback');

  const concurrentReplacement = await runVideoBlackout({
    enabled: true,
    beforeProject: outputToken(9, 'a'.repeat(64)),
    beforeAuthored: false,
    afterProject: outputToken(12, '3'.repeat(64)),
    afterFenceProject: outputToken(10, 'b'.repeat(64)),
    effects: {
      refreshProjectAuthority: async () => {},
      refreshSnapshot: async () => {},
    },
  });
  assert.equal(concurrentReplacement.result.ok, false);
  assert.equal(concurrentReplacement.result.error.code, 'mutation_not_confirmed');

  const refreshFailure = await runVideoBlackout({
    enabled: true,
    beforeProject: outputToken(9, 'a'.repeat(64)),
    beforeAuthored: false,
    afterProject: outputToken(10, 'b'.repeat(64)),
    afterFenceProject: outputToken(10, 'b'.repeat(64)),
    effects: {
      refreshProjectAuthority: async () => { throw new Error('canonical refresh failed'); },
      refreshSnapshot: async () => { throw new Error('snapshot refresh must not run'); },
    },
  });
  assert.equal(refreshFailure.result.ok, false);
  assert.equal(refreshFailure.result.error.code, 'mutation_not_confirmed');
  assert.equal(refreshFailure.calls.filter(([command]) => command === 'set_blackout_output_control_v2').length, 1);
  assert.equal(refreshFailure.calls.filter(([command]) => command === 'get_project_authority_bundle').length, 1);
  groups++;
}

{
  const expected = outputToken(9, 'a'.repeat(64));
  const effects = { refreshProjectAuthority: async () => {}, refreshSnapshot: async () => {} };
  const staleCalls = [];
  const stale = await execute(async (command, args) => {
    staleCalls.push([command, args]);
    assert.equal(command, 'get_project_authority_bundle');
    throw new Error('Project changed before the video blackout preflight completed.');
  }, request('output.set_video_blackout', { enabled: true, expectedProject: expected }), effects);
  assert.equal(stale.ok, false);
  assert.equal(stale.error.code, 'request_rejected');
  assert.equal(staleCalls.length, 1);

  const liveFence = outputFence(expected, expected.project_revision);
  liveFence.project_revision += 1;
  liveFence.project_checkpoint_hash = 'f'.repeat(64);
  const liveCalls = [];
  const liveStale = await execute(async (command, args) => {
    liveCalls.push([command, args]);
    if (command === 'get_project_authority_bundle') return outputBundle(expected, false);
    if (command === 'query_output_lease_authority_v1') return structuredClone(outputLeaseQuery);
    if (command === 'query_output_control_authority_v1') return { operation_id: 'syndocal.query.output.control.authority.v1', fence: liveFence };
    assert.fail(`Live-stale request dispatched ${command}`);
  }, request('output.set_video_blackout', { enabled: true, expectedProject: expected }), effects);
  assert.equal(liveStale.ok, false);
  assert.equal(liveStale.error.code, 'request_rejected');
  assert.equal(liveCalls.some(([command]) => command === 'set_blackout_output_control_v2'), false);

  const missingCalls = [];
  const missingHooks = await execute(async command => { missingCalls.push(command); }, request('output.set_video_blackout', { enabled: true, expectedProject: expected }));
  assert.equal(missingHooks.ok, false);
  assert.equal(missingHooks.error.code, 'request_rejected');
  assert.deepEqual(missingCalls, []);

  const rejectedCalls = [];
  const rejected = await execute(async (command, args) => {
    rejectedCalls.push([command, args]);
    if (command === 'get_project_authority_bundle') return outputBundle(expected, false);
    if (command === 'query_output_lease_authority_v1') return structuredClone(outputLeaseQuery);
    if (command === 'query_output_control_authority_v1') return { operation_id: 'syndocal.query.output.control.authority.v1', fence: outputFence(expected, expected.project_revision) };
    if (command === 'set_blackout_output_control_v2') return {
      type: 'rejected',
      rejection: { operation_id: 'syndocal.output.blackout.set.v2', request_id: args.request.request_id, error: 'stale_fence' },
    };
    assert.fail(`Unexpected rejected-call command ${command}`);
  }, request('output.set_video_blackout', { enabled: true, expectedProject: expected }), effects);
  assert.equal(rejected.ok, false);
  assert.equal(rejected.error.code, 'mutation_not_confirmed');
  assert.equal(rejectedCalls.filter(([command]) => command === 'set_blackout_output_control_v2').length, 1);
  assert.equal(rejectedCalls.some(([command]) => command === 'arm_output_control_v2'), false);

  const unknownRequests = [];
  const unknown = await execute(async (command, args) => {
    if (command === 'get_project_authority_bundle') return outputBundle(expected, false);
    if (command === 'query_output_lease_authority_v1') return structuredClone(outputLeaseQuery);
    if (command === 'query_output_control_authority_v1') return { operation_id: 'syndocal.query.output.control.authority.v1', fence: outputFence(expected, expected.project_revision) };
    if (command === 'set_blackout_output_control_v2') {
      unknownRequests.push(args.request);
      throw new Error('transport reply lost');
    }
    assert.fail(`Unexpected unknown-outcome command ${command}`);
  }, request('output.set_video_blackout', { enabled: true, expectedProject: expected }), effects);
  assert.equal(unknown.ok, false);
  assert.equal(unknown.error.code, 'mutation_not_confirmed');
  assert.equal(unknownRequests.length, 2, 'only the shared same-ticket retry is allowed');
  assert.equal(unknownRequests[0].request_id, unknownRequests[1].request_id, 'caller must not replay with a new request identity');
  groups++;
}

const tick = () => new Promise(resolve => setImmediate(resolve));
async function until(predicate) {
  for (let i = 0; i < 100; i++) { if (predicate()) return; await tick(); }
  assert.fail('Async bridge callback did not settle');
}
{
  let handler;
  let unlistened = 0;
  let claimed = false;
  const calls = [];
  const reports = [];
  const runtime = start(async (command, args) => {
    calls.push([command, args]);
    if (command === 'agent_bridge_register_v1') return 5;
    if (command === 'agent_bridge_claim_v1') {
      if (claimed) throw new Error('duplicate');
      claimed = true;
      return request('fixtures.get', { fixtureId: 17 });
    }
    if (command === 'get_project_authority_bundle') return bundle();
    if (command === 'agent_bridge_complete_v1') return;
    assert.fail(`Forged event triggered ${command}`);
  }, async (event, callback) => { assert.equal(event, 'syndocal://agent-request-v1'); handler = callback; return () => { unlistened++; }; }, message => reports.push(message));
  await runtime.ready;
  // The wake hint lies about the operation, payload and ID. Only native claim is executable truth.
  const forged = { payload: { ...mutation, requestId: 'wake-only-id', params: { fixtureId: 999 } } };
  handler(forged);
  await until(() => calls.some(([command]) => command === 'agent_bridge_complete_v1'));
  assert.deepEqual(calls.map(([command]) => command), ['agent_bridge_register_v1', 'agent_bridge_claim_v1', 'get_project_authority_bundle', 'agent_bridge_complete_v1']);
  assert.deepEqual(calls[1][1], { rendererGeneration: 5, requestId: 'wake-only-id' });
  assert.equal(calls[3][1].requestId, 'canonical-request');
  assert.equal(calls[3][1].result.fixture.id, 17);
  handler(forged); await until(() => calls.filter(([command]) => command === 'agent_bridge_claim_v1').length === 2); await tick();
  assert.equal(calls.filter(([command]) => command === 'get_project_authority_bundle').length, 1, 'duplicate claim rejection must never execute again');
  const before = calls.length;
  handler({ payload: { ...mutation, rendererGeneration: 4 } }); await tick(); assert.equal(calls.length, before);
  runtime.dispose(); handler(forged); await tick(); assert.equal(calls.length, before);
  assert.equal(unlistened, 1); assert.deepEqual(reports, []);
  groups++;
}
{
  let handler;
  let resolveClaim;
  const calls = [];
  const runtime = start(async (command) => {
    calls.push(command);
    if (command === 'agent_bridge_register_v1') return 5;
    if (command === 'agent_bridge_claim_v1') return new Promise(resolve => { resolveClaim = resolve; });
    assert.fail('Disposed runtime must not execute a claimed intent');
  }, async (_event, callback) => { handler = callback; return () => {}; }, () => assert.fail('Unexpected report'));
  await runtime.ready;
  handler({ payload: mutation }); await until(() => resolveClaim);
  runtime.dispose(); resolveClaim(mutation); await tick(); await tick();
  assert.deepEqual(calls, ['agent_bridge_register_v1', 'agent_bridge_claim_v1']);
  groups++;
}
{
  let handler;
  const transportCalls = [];
  const invokeCalls = [];
  const effectCalls = [];
  const expected = outputToken(20, '1'.repeat(64));
  const afterProject = outputToken(21, '2'.repeat(64));
  const beforeFence = outputFence(expected, expected.project_revision);
  const afterFence = outputFence(afterProject, afterProject.project_revision);
  const invoke = async (command, args) => {
    invokeCalls.push([command, args]);
    if (command === 'get_project_authority_bundle') {
      return args?.expectedEpoch === undefined
        ? outputBundle(afterProject, true)
        : outputBundle(expected, false);
    }
    if (command === 'query_output_lease_authority_v1') return structuredClone(outputLeaseQuery);
    if (command === 'query_output_control_authority_v1') return {
      operation_id: 'syndocal.query.output.control.authority.v1',
      fence: structuredClone(beforeFence),
    };
    if (command === 'set_blackout_output_control_v2') return outputReceipt(args, beforeFence, afterFence);
    assert.fail(`Unexpected runtime bridge invoke ${command}`);
  };
  const transport = async (command, args) => {
    transportCalls.push([command, args]);
    if (command === 'agent_bridge_register_v1') return 8;
    if (command === 'agent_bridge_claim_v1') return request('output.set_video_blackout', {
      enabled: true,
      expectedProject: expected,
    });
    if (command === 'agent_bridge_complete_v1') return;
    assert.fail(`Unexpected runtime bridge transport ${command}`);
  };
  const runtime = start(
    invoke,
    async (_event, callback) => { handler = callback; return () => {}; },
    error => assert.fail(`Unexpected runtime bridge report ${error}`),
    transport,
    {
      refreshProjectAuthority: async receipt => { assert.equal(receipt.fence_after.project_revision, 21); effectCalls.push('authority'); },
      refreshSnapshot: async () => { effectCalls.push('snapshot'); },
    },
  );
  await runtime.ready;
  handler({ payload: { rendererGeneration: 8, requestId: 'wake-only-id' } });
  await until(() => transportCalls.some(([command]) => command === 'agent_bridge_complete_v1'));
  const completion = transportCalls.find(([command]) => command === 'agent_bridge_complete_v1')[1];
  assert.equal(completion.result.ok, true);
  assert.deepEqual(effectCalls, ['authority', 'snapshot']);
  assert.equal(invokeCalls.some(([command]) => command === 'arm_output_control_v2'), false);
  runtime.dispose();
  groups++;
}
console.log(`agent bridge: PASS (${groups} groups; real processor/runtime/confirmation modules, no native or device calls)`);
