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

const undoLine = "You can undo this from the Project menu or with Ctrl/Cmd+Z.";

// Each question is a dialog description (title, consequence, labels, tone); the
// in-app dialog host renders it, so nothing here depends on a browser prompt.
assert.deepEqual(actions.destructiveActionDialog("fixture", "Front Spot 1"), {
  title: 'Remove fixture "Front Spot 1"?',
  message: undoLine,
  confirmLabel: "Remove",
  tone: "danger",
});

let asked = null;
assert.equal(
  await actions.confirmDestructiveAction("cue", "Opening", async (options) => {
    asked = options;
    return false;
  }),
  false,
  "declining returns false",
);
assert.equal(asked.title, 'Remove cue "Opening"?');
assert.equal(asked.tone, "danger", "a removal starts focused on Cancel");
assert.equal(await actions.confirmDestructiveAction("cue", "Opening", () => true), true, "a synchronous answer is accepted");
assert.equal(await actions.confirmDestructiveAction("cue", "Opening", () => undefined), false, "an unanswered question never confirms");

const cueNone = actions.cueRemovalDialog("Opening", 0, 0);
assert.equal(cueNone.title, 'Remove cue "Opening"?');
assert.deepEqual(cueNone.message, [
  "No timeline placements are linked to this Cue.",
  "No incoming jumps will be changed.",
  undoLine,
]);
const cueMany = actions.cueRemovalDialog("Scale Cue 001", 499, 2);
assert.deepEqual(cueMany.message, [
  "499 timeline placements will also be removed.",
  "2 incoming jumps will be cleared.",
  undoLine,
]);
let cueRemovalInvocations = 0;
assert.equal(await actions.confirmCueRemoval("Opening", 3, 1, async () => false), false);
assert.equal(await actions.confirmCueRemoval("Opening", 3, 1, async () => {
  cueRemovalInvocations += 1;
  return true;
}), true);
assert.equal(cueRemovalInvocations, 1);

const placement = actions.timelinePlacementRemovalDialog(500, "500 · Scale Cue 500", 32_000);
assert.equal(placement.title, "Remove Scene Block #500?");
assert.match(placement.message[0], /Scale Cue 500/);
assert.match(placement.message[0], /32000 ms span/);
assert.equal(placement.message.at(-1), undoLine);
assert.equal(actions.timelinePlacementRemovalDialog(7, "Cue", 0).title, "Remove timeline point #7?");
let placementRemovalInvocations = 0;
assert.equal(await actions.confirmTimelinePlacementRemoval(500, "Scale Cue 500", 32_000, async () => false), false);
assert.equal(await actions.confirmTimelinePlacementRemoval(500, "Scale Cue 500", 32_000, async () => {
  placementRemovalInvocations += 1;
  return true;
}), true);
assert.equal(placementRemovalInvocations, 1);

// Exercise the actual App entry point: a saved/clean show must still require
// consent (the in-app dialog, never a browser prompt) before the existing
// dirty-state check or any replacement work starts.
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
  asked = null;
  const sandbox = {
    newProjectInFlight: false,
    workspaceTab: () => workspace,
    controlMode: () => mode,
    confirmDialog: async (options) => { prompts += 1; asked = options; return consent; },
    newProjectDuringShowDialog: () => ({ title: "Create a new project and stop this show?", tone: "danger" }),
    confirmDiscardProjectChanges: async () => { discardChecks += 1; return false; },
    setMessage: () => {},
    invoke: () => assert.fail("a canceled New request must never mutate the project"),
  };
  await runInNewContext(newProjectCode, sandbox)();
  assert.equal(prompts, expectedPrompts, `${workspace}/${mode}: show confirmation is Perform-only`);
  if (expectedPrompts > 0) assert.equal(asked.tone, "danger", "the show-stopping question starts focused on Cancel");
  assert.equal(discardChecks, expectedDiscardChecks, `${workspace}/${mode}: declined show confirmation stops before project checks`);
  assert.equal(sandbox.newProjectInFlight, false, "a declined request releases its in-flight guard");
}

console.log("destructive action helpers ok");
