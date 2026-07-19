import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const source = await readFile(new URL("../src/statusModel.ts", import.meta.url), "utf8");
const transpiled = ts.transpileModule(source, {
  compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 },
  fileName: "statusModel.ts",
});
const model = await import(`data:text/javascript;base64,${Buffer.from(transpiled.outputText).toString("base64")}`);

assert.equal(model.appStatusTone("Saved project show.sdc"), "success");
assert.equal(model.appStatusTone("No MIDI input selected."), "warning");
assert.equal(model.appStatusTone("DMX address conflict: U0 A1"), "error");
assert.equal(model.appStatusTone("Timeline playing."), "info");
assert.deepEqual(model.appStatusFromMessage(""), { text: "Ready", tone: "info" });
assert.deepEqual(
  model.appStatusFromMessage("Save blocked.", "timeline-drafts-block-save"),
  { text: "Save blocked.", tone: "warning", key: "timeline-drafts-block-save" },
);

console.log("status model helpers ok");
