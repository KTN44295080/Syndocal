import assert from "node:assert/strict";
import fs from "node:fs/promises";
import ts from "../node_modules/typescript/lib/typescript.js";
const source = await fs.readFile(new URL("../src/createSceneEditWriteBarrier.ts", import.meta.url), "utf8");
const exports = {};
Function("exports", ts.transpileModule(source, { compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 } }).outputText)(exports);
let mode = "edit", ack, value = 42000, saved = null;
const queued = [];
const barrier = exports.createSceneEditWriteBarrier(() => mode === "edit");
const write = barrier.track(async () => {
  await new Promise(resolve => ack = resolve);
  queued.push(value);
});
let switched = false;
const transition = (async () => {
  await barrier.settle();
  saved = queued.at(-1);
  mode = "live"; switched = true;
})();
await Promise.resolve();
assert.equal(switched, false, "Live cannot enter before the native Edit write ACK");
ack(); await write; await transition;
value = 8000;
await barrier.track(async () => { /* live value update without scene capture */ });
assert.equal(saved, 42000, "later Live value cannot replace delayed Edit capture");
const failed = exports.createSceneEditWriteBarrier(() => true);
let rejectWrite;
const failure = failed.track(() => new Promise((_, reject) => rejectWrite = reject));
const settling = failed.settle();
rejectWrite(new Error("native rejected"));
await assert.rejects(failure, /native rejected/);
await assert.rejects(settling, /native rejected/);
await failed.settle();
const app = await fs.readFile(new URL("../src/App.tsx", import.meta.url), "utf8");
const ast = ts.createSourceFile("App.tsx", app, ts.ScriptTarget.Latest, true, ts.ScriptKind.TSX);
const expected = new Set(["setAttribute","setGroupAttribute","setFixtureTypeControlValue","setFixtureColor","setTouchBindingValue"]);
const visit = n => {
  if (ts.isVariableDeclaration(n) && expected.has(n.name.getText(ast))) {
    assert.match(n.initializer.getText(ast), /sceneEditWriteBarrier\.track\(async/);
    expected.delete(n.name.getText(ast));
  }
  ts.forEachChild(n,visit);
};
visit(ast); assert.equal(expected.size,0);
const change = app.slice(app.indexOf("const changeControlFaderWriteMode"),app.indexOf("const changeControlFaderWriteMode")+1200);
assert.ok(change.indexOf("await sceneEditWriteBarrier.settle()") < change.indexOf("await flushControlEditLookUpdates()"));
assert.ok(change.indexOf("await flushControlEditLookUpdates()") < change.indexOf("setControlFaderWriteMode(mode)"));
console.log("PASS: delayed native Edit ACK and capture finish before Live; failed writes drain; five real App write paths are owned");
