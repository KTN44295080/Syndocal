import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import ts from 'typescript';
const source = await readFile(new URL('../src/agentBridgeBootstrap.ts', import.meta.url), 'utf8');
const exports = {};
new Function('exports', ts.transpileModule(source, {
  compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 },
}).outputText)(exports);
const { startDeferredAgentBridge: start } = exports;
function harness() {
  let resolve, reject;
  const promise = new Promise((a, b) => { resolve = a; reject = b; });
  const calls = [], errors = [];
  const args = [() => {}, () => {}, error => errors.push(error), () => {}, {}];
  const bridge = start(...args, () => promise);
  const module = { startAgentBridgeRuntime(...actual) {
    calls.push(actual);
    return { ready: Promise.resolve(), dispose: () => calls.push('disposed') };
  } };
  return { bridge, resolve, reject, calls, errors, args, module };
}
{
  const h = harness(); h.bridge.dispose(); h.resolve(h.module); await h.bridge.ready;
  assert.deepEqual(h.calls, [], 'unmount during loading must never register a bridge');
}
{
  const h = harness(); h.resolve(h.module); await h.bridge.ready;
  assert.deepEqual(h.calls, [h.args], 'forward canonical dependencies without replacements');
  h.bridge.dispose(); h.bridge.dispose(); assert.deepEqual(h.calls, [h.args, 'disposed']);
}
{
  const h = harness(); h.reject(Error('load failed')); await h.bridge.ready;
  assert.match(h.errors[0], /load failed/); assert.deepEqual(h.calls, []);
}
{
  const h = harness(); h.bridge.dispose(); h.reject(Error('late')); await h.bridge.ready;
  assert.deepEqual(h.errors, [], 'late load rejection must not update an unmounted UI');
}
console.log('PASS 4 deferred agent bridge lifecycle groups');
