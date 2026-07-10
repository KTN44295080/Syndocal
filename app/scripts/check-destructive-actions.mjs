import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const source = await readFile(new URL("../src/destructiveActions.ts", import.meta.url), "utf8");
const transpiled = ts.transpileModule(source, {
  compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 },
  fileName: "destructiveActions.ts",
});
const actions = await import(`data:text/javascript;base64,${Buffer.from(transpiled.outputText).toString("base64")}`);

assert.equal(
  actions.destructiveActionPrompt("fixture", "Front Spot 1"),
  'Remove fixture "Front Spot 1"?\n\nThis action cannot be undone.',
);

let prompt = "";
assert.equal(
  actions.confirmDestructiveAction("cue", "Opening", (message) => {
    prompt = message;
    return false;
  }),
  false,
);
assert.equal(prompt, 'Remove cue "Opening"?\n\nThis action cannot be undone.');

console.log("destructive action helpers ok");
