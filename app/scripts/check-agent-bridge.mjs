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
// Managed listing and exact-artifact deletion use immutable native adapters.
assert.equal(rustCanonicalIds.length, 72);
assert.deepEqual([...typescriptCanonicalIds].sort(), [...rustCanonicalIds].sort());
assert.deepEqual([...nodeCanonicalIds].sort(), [...rustCanonicalIds].sort());

// Transpile actual production modules and resolve only their actual local dependency.
const modules = new Map();
for (const name of ['fixtureTransformConfirmation', 'agentBridgeControlPlane', 'agentBridgeRecording', 'agentBridgeTools', 'agentBridgeRuntime']) {
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
const { executeAgentBridgeCanonicalOperation: executeCanonical, CANONICAL_TAURI_COMMANDS, NATIVE_TIMELINE_MUTATIONS } = modules.get('./agentBridgeControlPlane');
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

// Exercise actual adapter admission before any invoke or mutation notification.
for (const operationId of Object.getOwnPropertyNames(Object.prototype)) {
  let invokes = 0, notifications = 0;
  await assert.rejects(executeCanonical(async () => { invokes++; return {}; },
    { operationId, request: {} }, () => { notifications++; }), /not executable/);
  assert.equal(invokes, 0, `${operationId} must not reach invoke`);
  assert.equal(notifications, 0, `${operationId} must not mark a mutation started`);
  const result = await execute(async () => { invokes++; return {}; },
    request('control_plane.execute', { operationId, request: {} }));
  assert.equal(result.ok, false);
  assert.equal(result.error.code, 'request_rejected');
  assert.equal(invokes, 0);
}
for (const [operationId, expectedCommand] of Object.entries(CANONICAL_TAURI_COMMANDS)) {
  const calls = []; let notifications = 0;
  const payload = {};
  if (NATIVE_TIMELINE_MUTATIONS.has(operationId)) {
    await assert.rejects(executeCanonical(async () => { calls.push('retired-direct-invoke'); return {}; },
      { operationId, request: payload }, () => { notifications++; }), /immutable native request/);
    assert.deepEqual(calls, []); assert.equal(notifications, 0);
    for (const ok of [false, true]) {
      const result = {ok, operation_id: operationId, result: {kind:ok?'receipt':'rejected'}};
      const actual = await execute(async (command,args) => {
        assert.equal(command,'agent_bridge_execute_native_v1');
        assert.deepEqual(args,{rendererGeneration:5,requestId:'canonical-request'});
        return result;
      },request('control_plane.execute',{operationId,request:{untrusted:'must never enter a direct invoke'}}));
      assert.deepEqual(actual,result,'typed native refusal must remain false rather than become a success wrapper');
    }
    continue;
  }
  await executeCanonical(async (command, args) => { calls.push([command, args]); return {}; },
    { operationId, request: payload }, () => { notifications++; });
  assert.deepEqual(calls, [[expectedCommand, payload]]);
  assert.equal(notifications, operationId.startsWith('syndocal.query.')
    || operationId === 'syndocal.output.lease.authority.query.v1' ? 0 : 1);
}
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
      state: 'Recording', active: true, output_id: 2, path: 'C:/recordings/take.mp4', width: 1920, height: 1080,
      frame_rate: 30, frames_written: 12, dropped_frames: 0, audio_requested: true,
      audio_included: true, audio_track_count: 1, started_unix_ms: 1234, last_error: null,
    };
  }, request('recording.get_status'));
  assert.equal(recordingStatus.ok, true);
  assert.equal(recordingStatus.recording.state, 'Recording');
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
  const leaseQuery = await execute(async (command, args) => {
    assert.equal(command, 'query_output_lease_authority_v1');
    assert.deepEqual(args, {});
    return { statuses: [{ status: 'unavailable' }] };
  }, request('control_plane.execute', {
    operationId: 'syndocal.output.lease.authority.query.v1', request: {},
  }));
  assert.deepEqual(leaseQuery.result, { statuses: [{ status: 'unavailable' }] });
  assert.equal(leaseQuery.operation_id, 'syndocal.output.lease.authority.query.v1');
  const failedLeaseQuery = await execute(async (command) => {
    assert.equal(command, 'query_output_lease_authority_v1');
    throw new Error('lease observation unavailable');
  }, request('control_plane.execute', {
    operationId: 'syndocal.output.lease.authority.query.v1', request: {},
  }));
  assert.equal(failedLeaseQuery.error.code, 'request_rejected');
  const typedError = { code: 'overloaded', message: 'query service overloaded', retryable: true, resnapshot_required: false };
  let typedCalls = 0;
  const typedFailure = await execute(async () => { typedCalls++; throw structuredClone(typedError); },
    request('control_plane.execute', { operationId: 'syndocal.output.lease.authority.query.v1', request: {} }));
  assert.deepEqual(typedFailure, { ok: false, error: {
    code: 'request_rejected', message: typedError.message, native_query: typedError,
  } });
  assert.equal(typedCalls, 1, 'typed retry information must not cause an automatic retry');
  for (const invalid of [{ ...typedError, credential: 'must-not-leak' },
    { ...typedError, retryable: 'true' }, { ...typedError, message: 'x'.repeat(1025) },
    { ...typedError, code: 'invalid code' }]) {
    const result = await execute(async () => { throw invalid; },
      request('control_plane.execute', { operationId: 'syndocal.output.lease.authority.query.v1', request: {} }));
    assert.equal(result.error.native_query, undefined);
    assert.equal(result.error.message, 'Native operation failed with an unrecognized error response.');
    assert.equal(JSON.stringify(result).includes('must-not-leak'), false);
  }
  const uncertain = await execute(async () => { throw typedError; }, request('control_plane.execute', {
    operationId: 'syndocal.output.lease.acquire.v2', request: {},
  }));
  assert.equal(uncertain.error.code, 'mutation_not_confirmed');
  assert.equal(uncertain.error.native_query, undefined, 'uncertain mutation must not receive read retry information');
  groups++;
  for (const operationId of ['syndocal.query.project.file.authority.v1',
    'syndocal.query.project.backup.authority.v1']) {
    for (const typedError of [
      { code: 'overloaded', message: 'query service overloaded', retryable: true, resnapshot_required: false },
      { code: 'unavailable', message: 'query service unavailable', retryable: true, resnapshot_required: false },
      { code: 'forbidden', message: 'operation forbidden', retryable: false, resnapshot_required: false },
    ]) {
      let calls = 0;
      const result = await execute(async (command, args) => {
        calls++;
        assert.equal(command, 'agent_bridge_execute_native_v1');
        assert.deepEqual(args, { rendererGeneration: 5, requestId: 'canonical-request' });
        throw structuredClone(typedError);
      }, request('control_plane.execute', { operationId, request: {} }));
      assert.deepEqual(result, { ok: false, error: {
        code: 'request_rejected', message: typedError.message, native_query: typedError,
      } });
      assert.equal(calls, 1, 'File query error metadata must not cause an automatic replay');
    }
    const oldString = await execute(async () => { throw 'project_file_request_invalid'; },
      request('control_plane.execute', { operationId, request: {} }));
    assert.deepEqual(oldString, { ok: false, error: {
      code: 'request_rejected', message: 'project_file_request_invalid',
    } });
  }
  const uncertainFile = await execute(async () => { throw typedError; },
    request('control_plane.execute', { operationId: 'syndocal.project.save.v1', request: {} }));
  assert.equal(uncertainFile.error.code, 'mutation_not_confirmed');
  assert.equal(uncertainFile.error.native_query, undefined);
  groups++;
  for (const operationId of ['syndocal.query.runtime.timeline.transport.authority.v1',
    'syndocal.query.runtime.timeline.loop.authority.v1', 'syndocal.query.runtime.timeline.follow.abort.authority.v1']) {
    for (const code of ['invalid_request', 'forbidden', 'stale_fence', 'conflict', 'busy', 'overloaded', 'publication_failed', 'internal']) {
      let calls = 0;
      const result = await execute(async () => { calls++; throw { code }; },
        request('control_plane.execute', { operationId, request: {} }));
      assert.deepEqual(result, { ok: false, error: { code: 'request_rejected',
        message: `Native runtime authority read rejected: ${code}.`, native_runtime: { code } } });
      assert.equal(calls, 1, 'typed runtime rejection cannot automatically replay a request');
    }
  }
  groups++;
  for (const invalid of [{ code: 'overloaded', credential: 'must-not-leak' },
    { code: 'overloaded', message: 'must-not-leak' }, { code: 'unknown' }, { code: 7 }, {},
    { code: 'x'.repeat(1025) }, { code: ['overloaded'] }]) {
    const result = await execute(async () => { throw invalid; }, request('control_plane.execute', {
      operationId: 'syndocal.query.runtime.timeline.loop.authority.v1', request: {},
    }));
    assert.equal(result.error.native_runtime, undefined);
    assert.equal(result.error.message, 'Native operation failed with an unrecognized error response.');
    assert.equal(JSON.stringify(result).includes('must-not-leak'), false);
  }
  for (const [operationId, expectedCode] of [
    ['syndocal.runtime.timeline.loop.commit.v1', 'mutation_not_confirmed'],
    ['syndocal.query.output.control.authority.v1', 'request_rejected'],
    ['syndocal.output.lease.acquire.v2', 'mutation_not_confirmed'],
  ]) {
    const result = await execute(async () => { throw { code: 'overloaded' }; },
      request('control_plane.execute', { operationId, request: {} }));
    assert.equal(result.error.code, expectedCode);
    assert.equal(result.error.native_runtime, undefined, 'only reviewed runtime authority reads receive this error wire');
  }
  groups++;
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

// Video BO forwards only the immutable native UUID. Native acceptance owns the actual domain/readback checks.
{
  for(const ok of [true,false]){
    const calls=[];const result={ok,verification:ok?'committed_project_state':undefined,result:ok?undefined:{type:'rejected'}};
    const actual=await execute(async(command,args)=>{calls.push([command,args]);return result;},
      request('output.set_video_blackout',{enabled:true,expectedProject:{forged:'must not be forwarded'}}));
    assert.deepEqual(actual,result);assert.deepEqual(calls,[['agent_bridge_execute_native_v1',{rendererGeneration:5,requestId:'canonical-request'}]]);
  }
  let calls=0;const uncertain=await execute(async command=>{assert.equal(command,'agent_bridge_execute_native_v1');calls++;throw new Error('reply lost');},
    request('output.set_video_blackout',{enabled:false,expectedProject:{}}));
  assert.equal(calls,1,'uncertain native execution must never retry');assert.equal(uncertain.ok,false);
  assert.equal(uncertain.error.code,'mutation_not_confirmed');groups++;
}

const tick = () => new Promise(resolve => setImmediate(resolve));
async function until(predicate) {
  for(let i=0;i<100;i++){if(predicate())return;await tick();}
  assert.fail("Async bridge callback did not settle");
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
  let handler;const transportCalls=[],invokeCalls=[],result={ok:true,verification:'committed_project_state'};
  const runtime=start(async(command,args)=>{invokeCalls.push([command,args]);assert.equal(command,'agent_bridge_execute_native_v1');
      assert.deepEqual(args,{rendererGeneration:5,requestId:'canonical-request'});return result;},
    async(_event,callback)=>{handler=callback;return ()=>{};},error=>assert.fail(error),async(command,args)=>{
      transportCalls.push([command,args]);if(command==='agent_bridge_register_v1')return 8;
      if(command==='agent_bridge_claim_v1')return request('output.set_video_blackout',{enabled:true,expectedProject:{}});
      if(command==='agent_bridge_complete_v1')return;assert.fail(command);
    });
  await runtime.ready;handler({payload:{rendererGeneration:8,requestId:'wake-only-id'}});
  await until(()=>transportCalls.some(([command])=>command==='agent_bridge_complete_v1'));
  assert.equal(invokeCalls.length,1);assert.deepEqual(transportCalls.find(([command])=>command==='agent_bridge_complete_v1')[1].result,result);
  runtime.dispose();groups++;
}

{
  for (const item of [...['syndocal.query.project.backup.delete.journal.v1','syndocal.query.project.backup.delete.journal.status.v1','syndocal.project.backup.delete.journal.manage.v1','syndocal.query.project.backup.delete.status.v1', 'syndocal.project.backup.delete.v1', 'syndocal.query.project.backup.list.v1', 'syndocal.query.project.backup.inspect.v1', 'syndocal.project.backup.create.v1', 'syndocal.query.project.backup.authority.v1', 'syndocal.project.save.v1', 'syndocal.project.save_as.v1', 'syndocal.project.template.save.v1',
    'syndocal.query.project.file.authority.v1', 'syndocal.query.project.file.status.v1', 'syndocal.project.file.acknowledge.v1']
    .map(operationId => request('control_plane.execute', { operationId, request: { request: { exact: 'immutable-backend-copy' } } })),
    request('diagnostics.export', { destination: 'C:/new.zip' }),
    request('control_plane.execute', { operationId: 'syndocal.project.new.v1', request: { request: { exact: 'immutable-backend-copy' } } }),
    request('control_plane.execute', { operationId: 'syndocal.project.open.v1', request: { request: { exact: 'immutable-backend-copy' } } }),
    request('control_plane.execute', { operationId: 'syndocal.project.backup.restore.v1', request: { request: { exact: 'immutable-backend-copy' } } }),
    request('control_plane.execute', { operationId: 'syndocal.output.ownership.arm.v2', request: { request: { exact: 'immutable-backend-copy' } } })]) {
    const calls = [];
    const result = await execute(async (command, args) => {
      calls.push([command, args]);
      return { ok: true };
    }, item);
    assert.equal(result.ok, true);
    assert.deepEqual(calls, [['agent_bridge_execute_native_v1', { rendererGeneration: 5, requestId: 'canonical-request' }]]);
  }
  groups++;
}
console.log(`agent bridge: PASS (${groups} groups; real processor/runtime/confirmation modules, no native or device calls)`);
