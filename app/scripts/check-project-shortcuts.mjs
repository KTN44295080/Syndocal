import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const transpile = (source, fileName) => ts.transpileModule(source, {
  compilerOptions: {
    module: ts.ModuleKind.ESNext,
    target: ts.ScriptTarget.ES2022,
    importsNotUsedAsValues: ts.ImportsNotUsedAsValues.Remove,
  },
  fileName,
}).outputText;
const dataUrl = (source) => `data:text/javascript;base64,${Buffer.from(source).toString("base64")}`;

const helperSource = await readFile(new URL("../src/hotkeyHelpers.ts", import.meta.url), "utf8");
const uiModesSource = await readFile(new URL("../src/uiModes.ts", import.meta.url), "utf8");
const helperUrl = dataUrl(transpile(helperSource, "hotkeyHelpers.ts"));
const uiModesUrl = dataUrl(transpile(uiModesSource, "uiModes.ts"));
const appShortcutSource = (await readFile(new URL("../src/appShortcutActions.ts", import.meta.url), "utf8"))
  .replace('"./hotkeyHelpers"', JSON.stringify(helperUrl))
  .replace('"./uiModes"', JSON.stringify(uiModesUrl));
const appShortcutModule = await import(dataUrl(transpile(appShortcutSource, "appShortcutActions.ts")));
const {
  APP_SHORTCUT_ACTION_KINDS,
  executeAppShortcut,
  resolveAppShortcut,
} = appShortcutModule;

const projectSource = await readFile(new URL("../src/projectFileShortcuts.ts", import.meta.url), "utf8");
const projectModule = await import(dataUrl(transpile(projectSource, "projectFileShortcuts.ts")));
const {
  PROJECT_FILE_SHORTCUT_ACTION_KINDS,
  dispatchProjectFileShortcut,
  resolveProjectFileShortcut,
} = projectModule;

const expectedAppKinds = [
  "newProject", "undoProject", "redoProject", "setWorkspaceTab", "selectSetupMode",
  "toggleMappingHotkeyHelp", "closeMappingHotkeyHelp", "applyMappingSelectionManagement",
  "duplicateSelectedMappingFixtures", "setMappingStageTool", "toggleMappingLayer",
  "toggleMappingSelectionFlag", "applyMappingViewportAction", "applyMappingSelectionAction",
  "nudgeSelectedMappingFixtures", "removeSelectedMappingFixtures", "cancelMappingInteraction",
  "setControlMode", "triggerPreviousCue", "triggerNextCue", "triggerCue",
  "toggleCueFadePaused", "toggleTimelinePlayback", "toggleTimelineLoop", "scaleTimelineLoop",
  "setTimelineLoopA", "setTimelineLoopB", "toggleBlackout", "toggleVideoBlackout", "tapBpm",
];
assert.deepEqual(APP_SHORTCUT_ACTION_KINDS, expectedAppKinds, "the App inventory is exactly the supported 30 action kinds");
assert.deepEqual(
  PROJECT_FILE_SHORTCUT_ACTION_KINDS,
  ["saveProject", "saveProjectAs", "loadProject"],
  "the project-file inventory is exactly Save, Save As, and Open",
);

const event = (code, modifiers = {}) => ({
  code,
  key: code === "Slash" && modifiers.shiftKey ? "?" : "",
  repeat: false,
  altKey: false,
  ctrlKey: false,
  metaKey: false,
  shiftKey: false,
  ...modifiers,
});
const baseContext = {
  workspaceTab: "control",
  editable: false,
  mappingHotkeyHelpOpen: false,
  mappingStageTool: "select",
  mappingInteractionActive: false,
  normalizedMappingSnapSize: 2,
  cueIds: [101, 102, 103, 104, 105, 106, 107, 108, 109, 110],
  cuePadStartIndex: 0,
  activeFadePaused: false,
  timelinePlaying: false,
  timelineDurationMs: 1000,
  timelineSurfaceActive: false,
  timelineLoopEnabled: false,
  timelineLoopAvailable: true,
  blackout: false,
  videoBlackout: false,
};
const resolve = (code, modifiers = {}, context = {}) => resolveAppShortcut(event(code, modifiers), { ...baseContext, ...context });

// Conflict precedence is part of the public shortcut contract.
for (const [code, tab] of [
  ["Digit1", "patch"], ["Digit2", "video"], ["Digit3", "io"],
  ["Numpad1", "patch"], ["Numpad2", "video"], ["Numpad3", "io"],
]) {
  assert.deepEqual(
    resolve(code, { altKey: true }, { workspaceTab: "setup" }),
    { kind: "selectSetupMode", tab },
    `Alt+${code} must select Setup ${tab}`,
  );
  assert.equal(
    resolve(code, { altKey: true, ctrlKey: true }, { workspaceTab: "setup" }),
    null,
    `Alt+Ctrl+${code} must remain isolated from Setup navigation`,
  );
}
assert.deepEqual(resolve("KeyL", {}, { timelineSurfaceActive: true }), { kind: "toggleTimelineLoop", enabled: true });
assert.deepEqual(resolve("KeyL"), { kind: "setControlMode", mode: "live" });
assert.deepEqual(resolve("KeyA", { shiftKey: true }, { timelineSurfaceActive: true }), { kind: "setTimelineLoopA" });
assert.deepEqual(resolve("KeyB", { shiftKey: true }, { timelineSurfaceActive: true }), { kind: "setTimelineLoopB" });
assert.deepEqual(resolve("KeyB", { shiftKey: true }), null, "Shift+B remains unassigned outside Timeline");

const representativeCases = [
  ["newProject", "KeyN", { ctrlKey: true }, {}],
  ["undoProject", "KeyZ", { ctrlKey: true }, {}],
  ["redoProject", "KeyY", { ctrlKey: true }, {}],
  ["setWorkspaceTab", "F3", {}, {}],
  ["selectSetupMode", "Digit2", { altKey: true }, { workspaceTab: "setup" }],
  ["toggleMappingHotkeyHelp", "Slash", { shiftKey: true }, { workspaceTab: "setup" }],
  ["closeMappingHotkeyHelp", "Escape", {}, { workspaceTab: "setup", mappingHotkeyHelpOpen: true }],
  ["applyMappingSelectionManagement", "KeyA", { ctrlKey: true }, { workspaceTab: "setup" }],
  ["duplicateSelectedMappingFixtures", "KeyD", { ctrlKey: true }, { workspaceTab: "setup" }],
  ["setMappingStageTool", "KeyS", {}, { workspaceTab: "setup" }],
  ["toggleMappingLayer", "KeyL", {}, { workspaceTab: "setup" }],
  ["toggleMappingSelectionFlag", "KeyQ", {}, { workspaceTab: "setup" }],
  ["applyMappingViewportAction", "KeyF", {}, { workspaceTab: "setup" }],
  ["applyMappingSelectionAction", "Digit4", {}, { workspaceTab: "setup" }],
  ["nudgeSelectedMappingFixtures", "ArrowUp", { shiftKey: true }, { workspaceTab: "setup" }],
  ["removeSelectedMappingFixtures", "Delete", {}, { workspaceTab: "setup" }],
  ["cancelMappingInteraction", "Escape", {}, { workspaceTab: "setup", mappingInteractionActive: true }],
  ["setControlMode", "KeyE", {}, {}],
  ["triggerPreviousCue", "Space", { shiftKey: true }, {}],
  ["triggerNextCue", "Space", {}, {}],
  ["triggerCue", "Digit1", {}, {}],
  ["toggleCueFadePaused", "KeyP", {}, {}],
  ["toggleTimelinePlayback", "KeyT", {}, {}],
  ["toggleTimelineLoop", "KeyL", {}, { timelineSurfaceActive: true }],
  ["scaleTimelineLoop", "BracketRight", {}, { timelineSurfaceActive: true }],
  ["setTimelineLoopA", "KeyA", { shiftKey: true }, { timelineSurfaceActive: true }],
  ["setTimelineLoopB", "KeyB", { shiftKey: true }, { timelineSurfaceActive: true }],
  ["toggleBlackout", "KeyB", {}, {}],
  ["toggleVideoBlackout", "KeyV", {}, {}],
  ["tapBpm", "KeyK", {}, {}],
];
for (const [kind, code, modifiers, context] of representativeCases) {
  assert.equal(resolve(code, modifiers, context)?.kind, kind, `${code} must retain its ${kind} route`);
}
assert.deepEqual(
  representativeCases.map(([kind]) => kind),
  expectedAppKinds,
  "the regression table has one ordered witness for every supported App action",
);

// Existing aliases resolve to one semantic action and execute that action once.
assert.deepEqual(resolve("Digit1"), resolve("Numpad1"), "cue top-row and numpad aliases agree");
assert.deepEqual(
  resolve("Digit2", {}, { workspaceTab: "setup" }),
  resolve("Numpad2", {}, { workspaceTab: "setup" }),
  "mapping top-row and numpad aliases agree",
);
const executionCalls = [];
const executor = new Proxy({}, {
  get: (_target, property) => (...args) => executionCalls.push([property, ...args]),
});
executeAppShortcut(resolve("Numpad1"), executor);
assert.deepEqual(executionCalls, [["triggerCue", 101]], "one alias gesture executes exactly one callback");

// Editable controls retain only the established global/project behavior.
assert.equal(resolve("KeyZ", { ctrlKey: true }, { editable: true }), null, "native editable undo is not intercepted");
assert.equal(resolve("KeyB", {}, { editable: true }), null, "plain control shortcuts are blocked in editable controls");
assert.equal(resolve("Digit1", { altKey: true }, { workspaceTab: "setup", editable: true }), null, "Setup Alt+Digit is blocked in editable controls");
assert.deepEqual(resolve("F1", {}, { editable: true }), { kind: "setWorkspaceTab", tab: "setup" }, "workspace F-keys remain global");
assert.deepEqual(resolve("KeyN", { ctrlKey: true }, { editable: true }), { kind: "newProject" }, "New remains global");

// Exercise a finite, nonvacuous key/modifier/workspace/editable/state matrix and
// prove every declared App action kind has at least one reachable witness.
const codes = [
  "KeyN", "KeyZ", "KeyY", "F1", "F2", "F3", "Digit0", "Digit1", "Digit2", "Digit3", "Digit4", "Digit5",
  "Numpad0", "Numpad1", "Numpad2", "Numpad3", "Numpad4", "Numpad5", "Slash", "Escape", "KeyA", "KeyB",
  "KeyD", "KeyE", "KeyF", "KeyG", "KeyH", "KeyI", "KeyK", "KeyL", "KeyM", "KeyO", "KeyP", "KeyQ",
  "KeyR", "KeyS", "KeyT", "KeyV", "KeyW", "KeyX", "Equal", "Minus", "NumpadAdd", "NumpadSubtract",
  "BracketLeft", "BracketRight", "ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown", "Delete", "Backspace", "Space", "Enter",
];
const modifierMatrix = [
  {}, { shiftKey: true }, { ctrlKey: true }, { ctrlKey: true, shiftKey: true },
  { metaKey: true }, { altKey: true }, { altKey: true, shiftKey: true }, { repeat: true },
];
const stateMatrix = [
  {},
  { mappingHotkeyHelpOpen: true, mappingStageTool: "place", mappingInteractionActive: true },
  { mappingHotkeyHelpOpen: false, mappingStageTool: "place", mappingInteractionActive: true },
  { timelineSurfaceActive: true, timelineLoopEnabled: true, timelineLoopAvailable: true },
  { timelineSurfaceActive: true, timelineLoopAvailable: false, activeFadePaused: null, timelineDurationMs: 0, cueIds: [] },
  { timelineSurfaceActive: true, timelinePlaying: true, activeFadePaused: true },
];
const reachableKinds = new Set();
let matrixCases = 0;
for (const workspaceTab of ["setup", "control", "touch"]) {
  for (const editable of [false, true]) {
    for (const state of stateMatrix) {
      for (const code of codes) {
        for (const modifiers of modifierMatrix) {
          matrixCases += 1;
          const action = resolveAppShortcut(event(code, modifiers), { ...baseContext, ...state, workspaceTab, editable });
          if (action) {
            assert.ok(expectedAppKinds.includes(action.kind), `matrix produced undeclared action ${action.kind}`);
            reachableKinds.add(action.kind);
          }
        }
      }
    }
  }
}
assert.ok(matrixCases >= 10_000, `shortcut matrix must remain nonvacuous (got ${matrixCases})`);
assert.deepEqual([...reachableKinds].sort(), [...expectedAppKinds].sort(), "all 30 declared App actions are reachable in the finite matrix");

const projectEvent = (code, modifiers = {}) => ({
  code,
  repeat: false,
  altKey: false,
  ctrlKey: true,
  metaKey: false,
  shiftKey: false,
  preventDefault: () => {},
  ...modifiers,
});
const projectKinds = new Set();
for (const code of codes) {
  for (const modifiers of modifierMatrix) {
    const action = resolveProjectFileShortcut(projectEvent(code, modifiers));
    if (action) projectKinds.add(action.kind);
  }
}
assert.deepEqual([...projectKinds].sort(), [...PROJECT_FILE_SHORTCUT_ACTION_KINDS].sort(), "all 3 project-file actions are reachable");

const projectCalls = [];
let prevented = 0;
const actions = {
  saveProject: () => projectCalls.push("save"),
  saveProjectAs: () => projectCalls.push("save-as"),
  loadProject: () => projectCalls.push("open"),
};
const dispatch = (code, modifiers = {}) => dispatchProjectFileShortcut({
  ...projectEvent(code, modifiers),
  preventDefault: () => { prevented += 1; },
}, actions);
assert.equal(dispatch("KeyS"), true);
assert.deepEqual(projectCalls, ["save"]);
projectCalls.length = 0;
assert.equal(dispatch("KeyS", { ctrlKey: false, metaKey: true, shiftKey: true }), true);
assert.deepEqual(projectCalls, ["save-as"], "Cmd alias invokes Save As exactly once");
projectCalls.length = 0;
assert.equal(dispatch("KeyO"), true);
assert.deepEqual(projectCalls, ["open"]);
assert.equal(prevented, 3, "each handled project gesture is consumed exactly once");
assert.equal(dispatch("KeyO", { shiftKey: true }), false, "Ctrl+Shift+O remains unassigned");
assert.deepEqual(projectCalls, ["open"], "unassigned project gestures execute nothing");

const controllerSource = await readFile(new URL("../src/createAppKeyboardController.ts", import.meta.url), "utf8");
assert.match(controllerSource, /if \(dispatchProjectFileShortcut\(event, options\)\) return;/, "project shortcuts remain first in the application handler");
assert.match(controllerSource, /executeAppShortcut\(action, shortcutExecutor\);/, "resolved App actions use the exhaustive executor");
assert.doesNotMatch(appShortcutSource, /setAllBlackout/, "the unused all-blackout API is not presented as a reachable shortcut action");
const appSource = await readFile(new URL("../src/App.tsx", import.meta.url), "utf8");
assert.match(appSource, /window\.addEventListener\("keydown", handleAppKeyDown\)/, "the tested controller remains mounted");
assert.match(appSource, /handleControlKeyDown\(event\)/, "the mounted application handler delegates to the controller");

console.log(`project/app shortcuts: ${matrixCases} matrix cases; App=30 reachable kinds; ProjectFile=3 reachable kinds`);
