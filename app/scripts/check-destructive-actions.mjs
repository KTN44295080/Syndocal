import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { runInNewContext } from "node:vm";
import ts from "typescript";

const source = await readFile(new URL("../src/destructiveActions.ts", import.meta.url), "utf8");
const transpiled = ts.transpileModule(source, {
  compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 },
  fileName: "destructiveActions.ts",
});
const actions = await import(`data:text/javascript;base64,${Buffer.from(transpiled.outputText).toString("base64")}`);

assert.equal(
  actions.destructiveActionPrompt("fixture", "Front Spot 1"),
  'Remove fixture "Front Spot 1"?\n\nYou can undo this action from the Project menu or with Ctrl/Cmd+Z.',
);

let prompt = "";
assert.equal(
  actions.confirmDestructiveAction("cue", "Opening", (message) => {
    prompt = message;
    return false;
  }),
  false,
);
assert.equal(prompt, 'Remove cue "Opening"?\n\nYou can undo this action from the Project menu or with Ctrl/Cmd+Z.');

assert.equal(
  actions.cueRemovalPrompt("Opening", 0, 0),
  'Remove cue "Opening"?\n\nNo timeline placements are linked to this Cue.\nNo incoming jumps will be changed.\n\nYou can undo this action from the Project menu or with Ctrl/Cmd+Z.',
);
assert.equal(
  actions.cueRemovalPrompt("Scale Cue 001", 499, 2),
  'Remove cue "Scale Cue 001"?\n\n499 timeline placements will also be removed.\n2 incoming jumps will be cleared.\n\nYou can undo this action from the Project menu or with Ctrl/Cmd+Z.',
);
let cueRemovalInvocations = 0;
assert.equal(actions.confirmCueRemoval("Opening", 3, 1, () => false), false);
assert.equal(actions.confirmCueRemoval("Opening", 3, 1, () => {
  cueRemovalInvocations += 1;
  return true;
}), true);
assert.equal(cueRemovalInvocations, 1);

const placementPrompt = actions.timelinePlacementRemovalPrompt(500, "500 · Scale Cue 500", 32_000);
assert.match(placementPrompt, /Scene Block #500/);
assert.match(placementPrompt, /Scale Cue 500/);
assert.match(placementPrompt, /32000 ms span/);
assert.match(placementPrompt, /undo/i);
let placementRemovalInvocations = 0;
assert.equal(actions.confirmTimelinePlacementRemoval(500, "Scale Cue 500", 32_000, () => false), false);
assert.equal(actions.confirmTimelinePlacementRemoval(500, "Scale Cue 500", 32_000, () => {
  placementRemovalInvocations += 1;
  return true;
}), true);
assert.equal(placementRemovalInvocations, 1);

// Exercise the actual App entry point: a saved/clean show must still require
// consent before the existing dirty-state check or any replacement work starts.
const appSource = await readFile(new URL("../src/App.tsx", import.meta.url), "utf8");
const appTree = ts.createSourceFile("App.tsx", appSource, ts.ScriptTarget.Latest, true, ts.ScriptKind.TSX);
let newProjectInitializer;
const visit = (node) => {
  if (ts.isVariableDeclaration(node) && node.name.getText(appTree) === "newProject") {
    newProjectInitializer = node.initializer;
  }
  ts.forEachChild(node, visit);
};
visit(appTree);
assert.ok(newProjectInitializer, "canonical New Project entry point exists");
const newProjectCode = ts.transpileModule(`const underTest = ${newProjectInitializer.getText(appTree)}; underTest;`, {
  compilerOptions: { module: ts.ModuleKind.None, target: ts.ScriptTarget.ES2022 },
}).outputText;
for (const [workspace, mode, consent, expectedPrompts, expectedDiscardChecks] of [
  ["control", "perform", false, 1, 0],
  ["control", "perform", true, 1, 1],
  ["control", "video", false, 0, 1],
  ["setup", "perform", false, 0, 1],
]) {
  let prompts = 0;
  let discardChecks = 0;
  const sandbox = {
    newProjectInFlight: false,
    workspaceTab: () => workspace,
    controlMode: () => mode,
    window: { confirm: () => { prompts += 1; return consent; } },
    translateUiText: (text) => text,
    uiLocale: () => "en",
    confirmDiscardProjectChanges: async () => { discardChecks += 1; return false; },
    setMessage: () => {},
    invoke: () => assert.fail("a canceled New request must never mutate the project"),
  };
  await runInNewContext(newProjectCode, sandbox)();
  assert.equal(prompts, expectedPrompts, `${workspace}/${mode}: show confirmation is Perform-only`);
  assert.equal(discardChecks, expectedDiscardChecks, `${workspace}/${mode}: declined show confirmation stops before project checks`);
  assert.equal(sandbox.newProjectInFlight, false, "a declined request releases its in-flight guard");
}

console.log("destructive action helpers ok");
