import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { runInNewContext } from "node:vm";
import ts from "typescript";

// Owner policy (2026-10-20): removals that Undo restores, and live Perform
// actions, act at once. Only irreversible data loss (discarding unsaved drafts,
// deleting a backup, installing an update, rotating or revoking access, adopting
// replacement media) may still ask.
const read = (path) => readFile(new URL(`../src/${path}`, import.meta.url), "utf8");
const appSource = await read("App.tsx");

const noConfirmation = {
  "App.tsx": [
    /confirmRemoval\(/, /confirmCueRemoval\(/, /removeFixturesDialog/, /newProjectDuringShowDialog/,
    /disarmDjLinkDialog/, /returnDjControlDialog/, /blackoutDialog/,
  ],
  "components/EditableTouchSurface.tsx": [/confirmDialog\(/],
  "components/RemoteControlPanel.tsx": [/confirmDialog\(/],
  "components/TimelineBankPanel.tsx": [/confirmDialog\(/],
  "components/TimelineSceneBlocksEditor.tsx": [/confirmTimelinePlacementRemoval|confirmDialog\(/],
  "components/VideoClipSlotInspectorPanel.tsx": [/confirmDialog\(/],
  "components/VideoEffectCatalogSetupPanel.tsx": [/confirmDialog\(/],
  "components/VideoEffectScopePanel.tsx": [/confirmDialog\(/],
  "components/VideoLayerGroupsPanel.tsx": [/confirmDialog\(/],
  "components/VideoPerformMatrix.tsx": [/confirmDialog\(/],
  "components/VideoTransitionBusPanel.tsx": [/confirmDialog\(/],
};
for (const [file, patterns] of Object.entries(noConfirmation)) {
  const source = file === "App.tsx" ? appSource : await read(file);
  for (const pattern of patterns) assert.doesNotMatch(source, pattern, `${file} must not ask before ${pattern}`);
}

// Custom removal dialogs only remain where the operator must choose where
// contents go; an empty layer or a plain item is removed at once.
const timelineEvents = await read("components/TimelineCueEventsPanel.tsx");
assert.doesNotMatch(timelineEvents, /itemRemoveDialog|audioClipRemoveDialog/, "timeline items and audio clips are removed without a dialog");
assert.match(timelineEvents, /pendingRemoveEventCount\(\) === 0 && pendingRemoveAudioClipCount\(\) === 0\) \{\s*confirmLayerRemoval\(\);/, "an empty timeline layer is removed without a dialog");
assert.match(await read("components/TimelineLayerToolbar.tsx"), /pendingRemoveEventCount\(\) === 0\) \{\s*confirmRemoval\(\);/, "an empty toolbar layer is removed without a dialog");
const mappingStrips = await read("components/MappingFilterStrips.tsx");
assert.match(mappingStrips, /setContextMenu\(null\);\s*void props\.onDeleteGroup\(groupId\);/, "a mapping group is deleted without a dialog");
assert.doesNotMatch(mappingStrips, /data-group-delete-dialog/, "the mapping-group delete dialog is gone");
const sceneMatrix = await read("components/SceneMatrixPanel.tsx");
assert.doesNotMatch(sceneMatrix, /data-scene-matrix-(scene-)?delete-dialog/, "the scene and Bank delete dialogs are gone");
assert.match(sceneMatrix, /void props\.onRemoveCueList\(bank\.id, true\);/, "a Bank is deleted without a dialog");
assert.match(sceneMatrix, /void props\.onRemoveCue\(cue\.id, true\);/, "a scene is deleted without a dialog");

// New Project never asks about the running show, but unsaved changes are still protected.
const appTree = ts.createSourceFile("App.tsx", appSource, ts.ScriptTarget.Latest, true, ts.ScriptKind.TSX);
let newProjectInitializer = null;
const visit = (node) => {
  if (ts.isVariableDeclaration(node) && node.name.getText(appTree) === "newProject") newProjectInitializer = node.initializer;
  ts.forEachChild(node, visit);
};
visit(appTree);
assert.ok(newProjectInitializer, "canonical New Project entry point exists");
const newProjectCode = ts.transpileModule(`const underTest = ${newProjectInitializer.getText(appTree)}; underTest;`, {
  compilerOptions: { module: ts.ModuleKind.None, target: ts.ScriptTarget.ES2022 },
}).outputText;
for (const [workspace, mode] of [["touch", "perform"], ["control", "video"], ["setup", "perform"]]) {
  let prompts = 0;
  let discardChecks = 0;
  const sandbox = {
    newProjectInFlight: false,
    workspaceTab: () => workspace,
    controlMode: () => mode,
    confirmDialog: async () => { prompts += 1; return true; },
    confirmDiscardProjectChanges: async () => { discardChecks += 1; return false; },
    setMessage: () => {},
    invoke: () => assert.fail("a canceled New request must never mutate the project"),
  };
  await runInNewContext(newProjectCode, sandbox)();
  assert.equal(prompts, 0, `${workspace}/${mode}: no show confirmation`);
  assert.equal(discardChecks, 1, `${workspace}/${mode}: unsaved changes are still checked`);
  assert.equal(sandbox.newProjectInFlight, false, "a canceled request releases its in-flight guard");
}

// Blackout acts at once; a machine that cannot drive outputs is still explained in-app.
{
  const blackoutInitializer = appSource.match(/const requestPerformVideoBlackout = async \(enabled: boolean\) => \{([\s\S]*?)\n  \};/);
  assert.ok(blackoutInitializer, "guarded Perform blackout request exists");
  const body = blackoutInitializer[1];
  assert.doesNotMatch(body, /confirmDialog/, "Blackout does not ask");
  assert.match(body, /drive\.kind === "blocked"[\s\S]*?BLACKOUT_STATUS_KEY[\s\S]*?return;/, "a blocked machine gets a keyed in-app message");
  assert.match(appSource, /const applyOutputDrive = [\s\S]*?clearStatusByKey\(BLACKOUT_STATUS_KEY\)/, "the refusal note retires when the output state changes");
}

console.log("removals and live actions act without confirmation; irreversible actions keep theirs");
