import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import ts from 'typescript';

// Transpile actual production modules and resolve only their actual local dependency.
const modules = new Map();
for (const name of ['fixtureTransformConfirmation', 'agentBridgeTools', 'agentBridgeRuntime']) {
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
const request = (method, params = {}) => ({ rendererGeneration: 5, requestId: 'canonical-request', method, params });
const mutation = request('fixtures.set_transform', { fixtureId: fixture.id, ...desired, expectedProject: token });
const bundle = (item = fixture, project = token) => ({ ...project, snapshot: { fixtures: item ? [structuredClone(item)] : [] } });
let groups = 0;

{
  const calls = [];
  const result = await execute(async (command, args) => { calls.push([command, args]); return bundle(); }, request('fixtures.get', { fixtureId: 17 }));
  assert.equal(result.ok, true);
  assert.deepEqual(result.fixture, fixture);
  assert.deepEqual(result.project, token);
  assert.deepEqual(calls, [['get_project_authority_bundle', {}]]);
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
console.log(`agent bridge: PASS (${groups} groups; real processor/runtime/confirmation modules, no native or device calls)`);
