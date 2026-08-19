import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const [controllerSource, appSource, overlaySource, operatorPolicySource] = await Promise.all([
  readFile(new URL("../src/safetyBlackoutRuntimeController.ts", import.meta.url), "utf8"),
  readFile(new URL("../src/App.tsx", import.meta.url), "utf8"),
  readFile(new URL("../src/components/OperatorLockOverlay.tsx", import.meta.url), "utf8"),
  readFile(new URL("../src/operatorPolicy.ts", import.meta.url), "utf8"),
]);

const transpiled = ts.transpileModule(controllerSource, {
  compilerOptions: {
    module: ts.ModuleKind.ESNext,
    target: ts.ScriptTarget.ES2022,
    importsNotUsedAsValues: ts.ImportsNotUsedAsValues.Remove,
  },
  fileName: "safetyBlackoutRuntimeController.ts",
});
const runtime = await import(
  `data:text/javascript;base64,${Buffer.from(transpiled.outputText).toString("base64")}`,
);
const hash = (character) => character.repeat(64);
const receipt = (request, outcome = "applied") => ({
  kind: "receipt",
  result: {
    operation_id: runtime.safetyBlackoutEngageOperationId,
    request_id: request.request_id,
    shape_sha256: hash("a"),
    audit_sequence: 1,
    outcome,
  },
});
const rejection = (request, code) => ({
  kind: "rejected",
  result: {
    operation_id: runtime.safetyBlackoutEngageOperationId,
    request_id: request.request_id,
    error: { code },
  },
});

// The worker synthetically applies the latch before losing the first IPC
// reply. Recovery must send the same no-target request object exactly once.
const replyLossCalls = [];
const replyLoss = runtime.createSafetyBlackoutRuntimeController({
  invoke: async (command, args) => {
    assert.equal(command, "safety_blackout_engage_v1");
    const request = args?.request;
    replyLossCalls.push(request);
    assert.deepEqual(Object.keys(request).sort(), ["operation_id", "request_id"]);
    if (replyLossCalls.length === 1) throw new Error("synthetic lost IPC reply after apply");
    return receipt(request);
  },
});
assert.equal(await replyLoss.engage(), "applied");
assert.equal(replyLossCalls.length, 2);
assert.strictEqual(replyLossCalls[1], replyLossCalls[0], "retry must reuse the exact request object");
assert.deepEqual(replyLossCalls[1], replyLossCalls[0]);

// Typed terminal failures and unknown future discriminators cannot be
// mistaken for reply loss and therefore receive no replay.
let forbiddenCalls = 0;
const forbidden = runtime.createSafetyBlackoutRuntimeController({
  invoke: async (_command, args) => {
    forbiddenCalls += 1;
    return rejection(args?.request, "forbidden");
  },
});
await assert.rejects(forbidden.engage(), /forbidden/);
assert.equal(forbiddenCalls, 1);

let unknownCalls = 0;
const unknown = runtime.createSafetyBlackoutRuntimeController({
  invoke: async () => (unknownCalls += 1, { kind: "future_terminal", result: {} }),
});
await assert.rejects(unknown.engage(), /invalid response identity/);
assert.equal(unknownCalls, 1);

// Production wiring keeps engage separate from legacy release, while Full
// Lock exposes only safer-direction button actions.
assert.match(appSource, /if \(enabled\) \{\s*await safetyBlackoutRuntime\.engage\(\);/s);
assert.match(
  appSource,
  /await executeOutputControl\(\s*invoke,\s*\{\s*kind: "release_blackout"/s,
  "App release must use the authenticated R4 OutputControl controller",
);
assert.doesNotMatch(
  appSource,
  /invoke\("set_blackout",\s*\{\s*enabled:\s*false\s*\}\)/,
  "S0 safety-blackout release must never fall back to the legacy setter",
);
assert.match(
  appSource,
  /const invokeSafetyBlackoutRuntime = async <T,>\(\s*command:[\s\S]*?if \(command !== "safety_blackout_engage_v1"\)/,
  "S0 engage must retain its narrow, separate ingress",
);
assert.match(overlaySource, /onClick=\{\(\) => props\.onSetBlackout\(true\)\}/);
assert.doesNotMatch(overlaySource, /onSetBlackout\(!props\.blackout\)/);
assert.doesNotMatch(overlaySource, /onSetAllBlackout\(false\)/);
assert.match(overlaySource, /BLACKOUT RELEASE LOCKED/);
assert.match(operatorPolicySource, /"safety_blackout_engage_v1"/);

console.log("safety blackout runtime contract: PASS");
