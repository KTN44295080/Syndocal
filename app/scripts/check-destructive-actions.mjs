import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { runInNewContext } from "node:vm";
import ts from "typescript";

// Owner policy (2026-10-20): removals that Undo restores, and live Perform
// actions, act at once. Only irreversible data loss (discarding unsaved drafts,
// deleting a backup or abandoning an unfinished publication intent) may still ask.
const read = (path) => readFile(new URL(`../src/${path}`, import.meta.url), "utf8");
const appSource = await read("App.tsx");

const noConfirmation = {
  "App.tsx": [
    /confirmRemoval\(/, /confirmCueRemoval\(/, /removeFixturesDialog/, /newProjectDuringShowDialog/,
    /disarmDjLinkDialog/, /returnDjControlDialog/, /blackoutDialog/,
    /installUpdateDialog/, /rotateDjLinkTokenDialog/, /adoptMediaReplacementDialog/,
  ],
  "components/AgentAuthorityPanel.tsx": [/confirmDialog\(/],
  "components/ArtRdmPanel.tsx": [/confirmationDialog|pendingConfirmation/],
  "components/VideoFxAuditionBar.tsx": [/confirmTake|data-fx-audition-take-confirm/],
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
    flushSceneEffectDraft: async () => true,
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

// Only the RDM SET that erases persistent configuration asks. Normal SET,
// read-only GET and discovery retain their explicit single action.
const rdm = await read("components/ArtRdmPanel.tsx");
const parseHex = rdm.match(/const parseHexU16 = ([\s\S]*?)\n};/);
const discardsConfiguration = rdm.match(/export const rdmSetDiscardsConfiguration = ([\s\S]*?);/);
assert.ok(parseHex && discardsConfiguration);
const rdmPolicyCode = ts.transpileModule(`const parseHexU16 = ${parseHex[1]}\n}; const underTest = ${discardsConfiguration[1]}; underTest;`, {
  compilerOptions: { module: ts.ModuleKind.None, target: ts.ScriptTarget.ES2022 },
}).outputText;
const rdmPolicy = runInNewContext(rdmPolicyCode);
for (const [command, pid, expected] of [["Set", "0090", true], ["Set", "0x0090", true], ["Set", " 90 ", true], ["Get", "0090", false], ["Set", "00F0", false], ["Set", "0060", false]]) {
  assert.equal(rdmPolicy(command, pid), expected, `${command} PID ${pid}: factory-default data loss policy`);
}
assert.match(rdm, /rdmSetDiscardsConfiguration\(command\(\), parameterId\(\)\)[\s\S]*?!await confirmDialog\(restoreRdmFactoryDefaultsDialog\(\)\)/);
assert.match(rdm, /const startFullDiscovery = \(\) => \{ void executeFullDiscovery\(\); \};/, "discovery does not ask a Yes/No question");

// Execute the real updater handler: drafts absent from the backup can never
// reach the platform installer, including a draft arriving during backup.
const initializerFor = (name) => {
  let found;
  const visit = (node) => {
    if (ts.isVariableDeclaration(node) && node.name.getText(appTree) === name) found = node.initializer;
    ts.forEachChild(node, visit);
  };
  visit(appTree);
  assert.ok(found, `${name}: real handler exists`);
  return ts.transpileModule(`const underTest = ${found.getText(appTree)}; underTest;`, {
    compilerOptions: { module: ts.ModuleKind.None, target: ts.ScriptTarget.ES2022 },
  }).outputText;
};
const updateCode = initializerFor("installAvailableApplicationUpdate");
for (const scenario of ["clean", "timeline-dirty", "invalid-fx", "valid-fx", "fx-save-failed", "late-timeline", "detached", "pane-opening"]) {
  const events = [];
  let busy = false;
  let timelineDirty = scenario === "timeline-dirty";
  let fxDirty = ["invalid-fx", "valid-fx", "fx-save-failed"].includes(scenario);
  const sandbox = {
    applicationUpdateCheck: () => ({ available: true, version: "1.2.3" }),
    applicationUpdateBusy: () => busy,
    setApplicationUpdateBusy: (value) => { busy = value; },
    paneWindow: null,
    poppedPanes: () => scenario === "detached" ? ["timeline"] : [],
    pendingPaneWindowOperations: () => scenario === "pane-opening" ? { timeline: true } : {},
    timelineEditorDirty: () => timelineDirty,
    sceneEffectDraftDirty: () => fxDirty,
    sceneEffectDraftSignature: () => scenario === "invalid-fx" ? null : "saved-FX",
    sceneEffectOperationsPending: () => 0,
    flushSceneEffectDraft: async () => {
      events.push("flush-fx");
      await Promise.resolve();
      if (scenario === "fx-save-failed") return false;
      fxDirty = false;
      return true;
    },
    setApplicationUpdateError: () => {},
    setApplicationUpdateProgress: () => {},
    setMessage: () => {},
    confirmDialog: () => assert.fail("the updater must not add a generic confirmation"),
    startProjectPublication: async () => {
      assert.equal(busy, true, "workspace stays owned during backup");
      assert.equal(fxDirty, false, "valid FX save finishes before backup captures its image");
      events.push("backup");
      if (scenario === "late-timeline") timelineDirty = true;
      return { state: "succeeded", request: { id: 1 } };
    },
    invoke: async (command) => {
      assert.equal(command, "install_application_update");
      assert.equal(busy, true, "workspace stays owned during platform download/install");
      events.push("install");
    },
    settleProjectPublicationTerminal: async () => { events.push("ack"); },
  };
  await runInNewContext(updateCode, sandbox)();
  const allowed = ["clean", "valid-fx"].includes(scenario);
  assert.equal(events.includes("install"), allowed, `${scenario}: install admission`);
  assert.equal(busy, false, `${scenario}: failures/success release UI ownership`);
  if (scenario === "late-timeline") assert.deepEqual(events, ["flush-fx", "backup", "ack"], "late draft blocks installer while exact backup is acknowledged");
}
const workspaceView = await read("components/AppWorkspaceView.tsx");
assert.match(workspaceView, /class=\{`layout \$\{touchLayoutClass\(\)\}`\}\s*inert=\{applicationUpdateBusy\(\)\}/, "update owns editor UI during backup/download/handoff without disabling the emergency header");
assert.match(appSource, /const handleAppKeyDown = \(event: KeyboardEvent\) => \{\s*if \(applicationUpdateBusy\(\)\) return;/, "shortcuts cannot create drafts while updater owns the workspace");
assert.match(appSource, /const confirmDiscardProjectChanges = async[\s\S]*?if \(!await prepareSceneEffectDraftForProjectBoundary\(\)\) return false;/, "New/Open/Restore must save FX or ask before actual invalid-draft loss");
assert.match(appSource, /flushSceneEffectDraft = \(options = \{\}\) => \{[\s\S]*?return saveSceneEffectDraft\(\{ auto: true \}\);/, "FX flush awaits the actual save rather than launching a detached write");
console.log("update/draft data-loss admission: PASS (clean, saved FX, invalid FX, failed save, late Timeline draft, detached and opening pane)");

assert.match(await read("components/WorkspaceChrome.tsx"), /class="appProjectMenu" role="menu" inert=\{props.applicationUpdateBusy\}/, "update also blocks project-menu edits while emergency header controls remain active");

const fxBoundaryCode = initializerFor("prepareSceneEffectDraftForProjectBoundary");
for (const [scenario, initialSignature, dirtyAfterFlush, flushResult, expected] of [
  ["saved", "valid", false, true, true],
  ["invalid-discard-accepted", null, true, true, true],
  ["invalid-discard-cancelled", null, true, false, false],
  ["save-failed", "valid", true, false, false],
  ["edit-changed-during-save", "valid", true, true, false],
]) {
  let flushSettled = false;
  const sandbox = {
    sceneEffectDraftSignature: () => initialSignature,
    sceneEffectDraftDirty: () => { assert.equal(flushSettled, true); return dirtyAfterFlush; },
    flushSceneEffectDraft: async () => { await Promise.resolve(); flushSettled = true; return flushResult; },
    setMessage: () => {},
  };
  assert.equal(await runInNewContext(fxBoundaryCode, sandbox)(), expected, `${scenario}: project/close FX boundary`);
}
console.log("FX project/close boundary: PASS (await save, save failure, changed edit, accepted/cancelled irretrievable draft loss)");

// An Open already waiting for native event readiness or an existing pane
// queue must recheck the updater ownership when that await resumes.
const paneOpenCode = initializerFor("openPaneWindow");
for (const gap of ["events-ready", "queued", "clean"]) {
  let busy = false;
  let coreCalls = 0;
  let releaseEvents;
  let releaseQueue;
  let signalQueued;
  const eventsReady = gap === "events-ready" ? new Promise((resolve) => { releaseEvents = resolve; }) : Promise.resolve(true);
  const queueWait = gap === "queued" ? new Promise((resolve) => { releaseQueue = resolve; }) : Promise.resolve();
  const queued = new Promise((resolve) => { signalQueued = resolve; });
  const sandbox = {
    applicationUpdateBusy: () => busy,
    setMessage: () => {},
    isTauriRuntime: () => true,
    paneWindowEventsReady: eventsReady,
    enqueuePaneWindowOperation: async (_pane, action) => { signalQueued(); await queueWait; await action(); },
    openPaneWindowCore: async () => { coreCalls++; return true; },
  };
  const result = runInNewContext(paneOpenCode, sandbox)("timeline");
  if (gap === "events-ready") { busy = true; releaseEvents(true); }
  if (gap === "queued") { await queued; busy = true; releaseQueue(); }
  assert.equal(await result, gap === "clean", `${gap}: detached editor open admission`);
  assert.equal(coreCalls, gap === "clean" ? 1 : 0, `${gap}: no delayed editor open can cross update ownership`);
}

// A valid FX save inside draft preflight advances project authority. Capture
// the live authority after that save, before dispatching Open/Restore.
for (const [name, command, argument] of [
  ["loadProject", "load_project", undefined],
  ["loadProjectPath", "load_project_path", "C:/show.sdc"],
  ["loadProjectBackup", "load_project_backup", { id: 1, created_at_unix_ms: 0 }],
]) {
  let revision = 1;
  let invoked = false;
  const events = [];
  const sandbox = {
    captureProjectAuthorityIdentity: () => { events.push("capture"); return { project_epoch: 1, project_revision: revision, checkpoint_hash: String(revision).repeat(64) }; },
    confirmDiscardProjectChanges: async () => { await Promise.resolve(); events.push("flush"); revision = 2; return true; },
    projectTransactionOwnerId: "owner",
    invoke: async (actualCommand, request) => {
      invoked = true; events.push("invoke");
      assert.equal(actualCommand, command);
      assert.equal(request.expectedRevision, 2, `${name}: expected revision includes the awaited FX save`);
      assert.equal(request.expectedCheckpointHash, "2".repeat(64));
      return { path: "loaded.sdc", profiles: [] };
    },
    applyLoadedProjectResult: async () => ({}),
    projectAuthorityApplicationResultIsCurrent: () => true,
    setMessage: () => {},
  };
  await runInNewContext(initializerFor(name), sandbox)(argument);
  assert.equal(invoked, true, `${name}: approved action dispatches`);
  assert.deepEqual(events, ["flush", "capture", "invoke"], `${name}: flush/capture/dispatch ordering`);
}
const functionBodyCode = (source, name, parameters) => {
  const tree = ts.createSourceFile("controller.ts", source, ts.ScriptTarget.Latest, true, ts.ScriptKind.TS);
  const declaration = tree.statements.find((node) => ts.isFunctionDeclaration(node) && node.name?.text === name);
  assert.ok(declaration?.body, `${name}: real function body`);
  const asyncPrefix = declaration.modifiers?.some((modifier) => modifier.kind === ts.SyntaxKind.AsyncKeyword) ? "async " : "";
  return ts.transpileModule(`const underTest = ${asyncPrefix}function(${parameters}) ${declaration.body.getText(tree)}; underTest;`, {
    compilerOptions: { module: ts.ModuleKind.None, target: ts.ScriptTarget.ES2022 },
  }).outputText;
};
{
  let revision = 1;
  let invoked = false;
  const options = {
    daslightProjectImportBusy: () => false,
    captureProjectAuthorityIdentity: () => ({ project_epoch: 1, project_revision: revision, checkpoint_hash: String(revision).repeat(64) }),
    confirmDiscardProjectChanges: async () => { revision = 2; return true; },
    setDaslightProjectImportBusy: () => {}, setMessage: () => {},
    projectTransactionOwnerId: "owner",
    invoke: async (_command, request) => { invoked = true; assert.equal(request.expectedRevision, 2); return null; },
  };
  const factory = runInNewContext(functionBodyCode(await read("dvcImportController.ts"), "createDvcImportController", "options"));
  await factory(options).importDaslightProject();
  assert.equal(invoked, true, "DVC import captures after FX save");
}
console.log("pane-open updater gaps and Open/Restore/import authority after FX save: PASS");

{
  let revision = 1;
  let invoked = false;
  const context = {
    captureProjectAuthorityIdentity: () => ({ project_epoch: 1, project_revision: revision, checkpoint_hash: String(revision).repeat(64) }),
    confirmDiscardProjectChanges: async () => { revision = 2; return true; },
    projectTransactionOwnerId: "owner", setMessage: () => {},
    invoke: async (_command, request) => { invoked = true; assert.equal(request.expectedRevision, 2); return null; },
  };
  const loadTemplate = runInNewContext(functionBodyCode(await read("phase1Actions.ts"), "loadUserTemplateAction", "context"));
  await loadTemplate(context);
  assert.equal(invoked, true, "user template captures authority after FX save");
}
