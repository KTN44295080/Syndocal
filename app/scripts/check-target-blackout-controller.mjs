import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";
const source = await readFile(new URL("../src/createTargetBlackoutController.ts", import.meta.url), "utf8");
const calls = [];
let finish;
const exports = {};
const mock = {
  executeTargetBlackout: async (_invoke, target, enabled) => {
    calls.push([target, enabled]);
    await new Promise(resolve => { finish = resolve; });
  },
  executeBlackoutRelease: async () => calls.push(["release-safety"]),
};
new Function("require", "exports", ts.transpileModule(source, {
  compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 },
}).outputText)(name => {
  assert.equal(name, "./outputControlController"); return mock;
}, exports);
const messages = [];
let refreshes = 0;
const controller = exports.createTargetBlackoutController({
  invoke() { throw new Error("unexpected direct IPC"); },
  safety: { engage: async () => calls.push(["engage-safety"]) },
  refreshSnapshot: async () => { refreshes++; },
  setMessage: message => messages.push(message),
});
const first = controller.setLightingBlackout(true);
await controller.setVideoBlackout(true);
await controller.releaseSafetyBlackout();
assert.deepEqual(calls, [["lighting", true]], "pending ordinary operation must not race another target or release S0");
await controller.engageSafetyBlackout();
assert.deepEqual(calls.at(-1), ["engage-safety"], "emergency engage must bypass an ordinary pending operation");
finish(); await first;
for (const [method, target] of [["setVideoBlackout", "video"], ["setAllBlackout", "both"]]) {
  const work = controller[method](false); finish(); await work;
  assert.deepEqual(calls.at(-1), [target, false]);
}
await controller.releaseSafetyBlackout();
assert.deepEqual(calls.at(-1), ["release-safety"]);
mock.executeTargetBlackout = async () => { throw new Error("rejected"); };
await controller.setLightingBlackout(false);
assert.deepEqual(messages, ["Error: rejected"]);
await controller.releaseSafetyBlackout();
assert.equal(refreshes, 6, "failed action must not pretend to refresh success; pending must reset");
console.log("target blackout controller: PASS (target scope, single pending action, emergency priority, error recovery)");
