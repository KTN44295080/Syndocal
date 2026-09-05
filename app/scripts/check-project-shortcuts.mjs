import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import ts from "typescript";

const appRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const appShortcutPath = path.join(appRoot, "src", "appShortcutActions.ts");
const projectFileShortcutPath = path.join(appRoot, "src", "projectFileShortcuts.ts");
const keyboardShortcutSourceManifestPath = path.join(
  appRoot,
  "src",
  "keyboard-shortcut-source-manifest.json",
);

const unwrapExpression = (expression) => {
  let current = expression;
  while (ts.isParenthesizedExpression(current)
    || ts.isAsExpression(current)
    || ts.isSatisfiesExpression(current)
    || ts.isNonNullExpression(current)) current = current.expression;
  return current;
};

const finiteStringLiterals = (type, seen = new Set()) => {
  if (seen.has(type)) return null;
  seen.add(type);
  if (type.isStringLiteral()) return new Set([type.value]);
  if (!type.isUnion()) return null;
  const values = new Set();
  for (const member of type.types) {
    const memberValues = finiteStringLiterals(member, seen);
    if (!memberValues) return null;
    for (const value of memberValues) values.add(value);
  }
  return values;
};

const propertyName = (name) => (ts.isIdentifier(name) || ts.isStringLiteralLike(name)) ? name.text : null;

const sourceIdForShortcutActionKind = (kind) => {
  if (!/^[a-z][A-Za-z0-9]*$/.test(kind)) {
    throw new Error(`shortcut action kind must be lower camel ASCII: ${JSON.stringify(kind)}`);
  }
  const sourceId = `${kind.replace(/[A-Z]/g, (letter) => `_${letter.toLowerCase()}`)}_v1`;
  if (!/^[a-z][a-z0-9_]*_v[1-9][0-9]*$/.test(sourceId)) {
    throw new Error(`shortcut source id is not versioned lower_snake ASCII: ${JSON.stringify(sourceId)}`);
  }
  return sourceId;
};

const setDifference = (left, right) => [...left].filter((value) => !right.has(value));

const assertSameSet = (actual, expected, label) => {
  const missing = setDifference(expected, actual);
  const extra = setDifference(actual, expected);
  assert.deepEqual(
    { missing, extra },
    { missing: [], extra: [] },
    `${label} must have no missing or extra action kinds`,
  );
};

const sourceManifestDeclaration = (sourceFile, declarationName) => {
  for (const statement of sourceFile.statements) {
    if (!ts.isVariableStatement(statement)) continue;
    for (const declaration of statement.declarationList.declarations) {
      if (!ts.isIdentifier(declaration.name) || declaration.name.text !== declarationName) continue;
      if ((statement.declarationList.flags & ts.NodeFlags.Const) === 0 || !declaration.initializer) {
        throw new Error(`${declarationName} must be an initialized const declaration`);
      }
      const object = unwrapExpression(declaration.initializer);
      if (!ts.isObjectLiteralExpression(object)) {
        throw new Error(`${declarationName} must be a static object literal`);
      }
      const keys = [];
      const seen = new Set();
      for (const property of object.properties) {
        if (!ts.isPropertyAssignment(property) || property.name && ts.isComputedPropertyName(property.name)) {
          throw new Error(`${declarationName} must not contain dynamic, computed, or spread entries`);
        }
        const key = propertyName(property.name);
        if (!key || property.initializer.kind !== ts.SyntaxKind.TrueKeyword) {
          throw new Error(`${declarationName} entries must be explicit true keyed literals`);
        }
        if (seen.has(key)) throw new Error(`${declarationName} contains duplicate key ${key}`);
        seen.add(key);
        keys.push(key);
      }
      if (keys.length === 0) throw new Error(`${declarationName} must not be empty`);
      return keys;
    }
  }
  throw new Error(`${declarationName} declaration is missing`);
};

const actionKindUnion = (checker, sourceFile, typeName) => {
  for (const statement of sourceFile.statements) {
    if (!ts.isTypeAliasDeclaration(statement) || statement.name.text !== typeName) continue;
    const symbol = checker.getSymbolAtLocation(statement.name);
    if (!symbol) throw new Error(`${typeName} has no type symbol`);
    const actionType = checker.getDeclaredTypeOfSymbol(symbol);
    const kind = actionType.getProperty("kind");
    if (!kind) throw new Error(`${typeName} must discriminate on kind`);
    const declaration = kind.valueDeclaration ?? kind.declarations?.[0] ?? sourceFile;
    const values = finiteStringLiterals(checker.getTypeOfSymbolAtLocation(kind, declaration));
    if (!values || values.size === 0) {
      throw new Error(`${typeName}["kind"] must be a finite string-literal union`);
    }
    return [...values];
  }
  throw new Error(`${typeName} declaration is missing`);
};

const loadShortcutManifestProgram = () => {
  const configPath = path.join(appRoot, "tsconfig.json");
  const config = ts.readConfigFile(configPath, ts.sys.readFile);
  if (config.error) throw new Error(ts.flattenDiagnosticMessageText(config.error.messageText, "\n"));
  const parsed = ts.parseJsonConfigFileContent(config.config, ts.sys, appRoot, undefined, configPath);
  if (parsed.errors.length > 0) {
    throw new Error(parsed.errors.map((error) => ts.flattenDiagnosticMessageText(error.messageText, "\n")).join("\n"));
  }
  const program = ts.createProgram(parsed.fileNames, parsed.options);
  const diagnostics = [...program.getSyntacticDiagnostics(), ...program.getSemanticDiagnostics()];
  if (diagnostics.length > 0) {
    throw new Error(diagnostics.map((diagnostic) => ts.flattenDiagnosticMessageText(diagnostic.messageText, "\n")).join("\n"));
  }
  const appShortcutSourceFile = program.getSourceFile(appShortcutPath);
  const projectFileShortcutSourceFile = program.getSourceFile(projectFileShortcutPath);
  if (!appShortcutSourceFile || !projectFileShortcutSourceFile) {
    throw new Error("shortcut source files are missing from the TypeScript program");
  }
  return { checker: program.getTypeChecker(), appShortcutSourceFile, projectFileShortcutSourceFile };
};

const expectedKeyboardShortcutSourceManifest = ({
  appKinds,
  projectFileKinds,
}) => ({
  schema_version: 1,
  keyboard_app: [...appKinds].map(sourceIdForShortcutActionKind).sort(),
  keyboard_project_file: [...projectFileKinds].map(sourceIdForShortcutActionKind).sort(),
});

const validateKeyboardShortcutSourceManifest = (raw, expected) => {
  const errors = [];
  let parsed;
  try {
    parsed = JSON.parse(raw);
  } catch (error) {
    return [`keyboard shortcut source manifest is malformed JSON: ${error.message}`];
  }
  if (!parsed || Array.isArray(parsed) || typeof parsed !== "object") {
    errors.push("keyboard shortcut source manifest must be an object");
  } else {
    const keys = Object.keys(parsed).sort();
    const expectedKeys = ["keyboard_app", "keyboard_project_file", "schema_version"];
    if (JSON.stringify(keys) !== JSON.stringify(expectedKeys)) {
      errors.push("keyboard shortcut source manifest has missing or unknown top-level fields");
    }
    if (parsed.schema_version !== 1) errors.push("keyboard shortcut source manifest schema_version must be 1");
    for (const family of ["keyboard_app", "keyboard_project_file"]) {
      const sources = parsed[family];
      if (!Array.isArray(sources) || sources.some((source) => typeof source !== "string")) {
        errors.push(`${family} must be an array of source-id strings`);
        continue;
      }
      if (new Set(sources).size !== sources.length) errors.push(`${family} contains duplicate source ids`);
      if (sources.some((source, index) => index > 0 && source <= sources[index - 1])) {
        errors.push(`${family} source ids must be strictly byte-sorted`);
      }
      if (sources.some((source) => !/^[a-z][a-z0-9_]*_v[1-9][0-9]*$/.test(source))) {
        errors.push(`${family} contains a non-versioned lower_snake source id`);
      }
    }
  }
  const expectedRaw = `${JSON.stringify(expected, null, 2)}\n`;
  if (raw !== expectedRaw) {
    errors.push("keyboard shortcut source manifest is not byte-exactly generated from the typed keyed manifests");
  }
  return errors;
};

const runShortcutInventorySelfTests = () => {
  const expected = {
    schema_version: 1,
    keyboard_app: ["new_project_v1"],
    keyboard_project_file: ["save_project_v1"],
  };
  const raw = `${JSON.stringify(expected, null, 2)}\n`;
  assert.deepEqual(validateKeyboardShortcutSourceManifest(raw, expected), []);
  assert.ok(
    validateKeyboardShortcutSourceManifest("{", expected).some((error) => error.includes("malformed")),
    "malformed JSON must fail closed",
  );
  assert.ok(
    validateKeyboardShortcutSourceManifest(
      '{"schema_version":1,"keyboard_app":["new_project_v1","new_project_v1"],"keyboard_project_file":["save_project_v1"]}',
      expected,
    ).some((error) => error.includes("duplicate") || error.includes("byte-exactly")),
    "duplicate JSON source ids must fail closed",
  );
  assert.ok(
    validateKeyboardShortcutSourceManifest(
      `${JSON.stringify({ ...expected, unexpected: [] }, null, 2)}\n`,
      expected,
    ).some((error) => error.includes("unknown") || error.includes("byte-exactly")),
    "unknown JSON fields must fail closed",
  );
  assert.ok(
    validateKeyboardShortcutSourceManifest(
      raw,
      { ...expected, keyboard_app: ["renamed_project_v1"] },
    ).some((error) => error.includes("byte-exactly")),
    "source-id drift from the typed keyed manifest must fail closed",
  );
  assert.throws(() => sourceManifestDeclaration(ts.createSourceFile(
    "fixture.ts",
    "const MANIFEST = { ...dynamic };",
    ts.ScriptTarget.ES2022,
  ), "MANIFEST"), /dynamic/);
};

runShortcutInventorySelfTests();

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
  APP_SHORTCUT_ACTION_SOURCE_MANIFEST,
  executeAppShortcut,
  resolveAppShortcut,
} = appShortcutModule;

const projectSource = await readFile(new URL("../src/projectFileShortcuts.ts", import.meta.url), "utf8");
const projectModule = await import(dataUrl(transpile(projectSource, "projectFileShortcuts.ts")));
const {
  PROJECT_FILE_SHORTCUT_ACTION_KINDS,
  PROJECT_FILE_SHORTCUT_ACTION_SOURCE_MANIFEST,
  dispatchProjectFileShortcut,
  resolveProjectFileShortcut,
} = projectModule;

// The AST/type-checker path is intentional: source discovery must not depend
// on a textual regex over the union or its keyed manifest.
const {
  checker: shortcutManifestChecker,
  appShortcutSourceFile,
  projectFileShortcutSourceFile,
} = loadShortcutManifestProgram();
const typedAppKinds = actionKindUnion(shortcutManifestChecker, appShortcutSourceFile, "AppShortcutAction");
const typedProjectFileKinds = actionKindUnion(
  shortcutManifestChecker,
  projectFileShortcutSourceFile,
  "ProjectFileShortcutAction",
);
const staticAppKinds = sourceManifestDeclaration(
  appShortcutSourceFile,
  "APP_SHORTCUT_ACTION_SOURCE_MANIFEST",
);
const staticProjectFileKinds = sourceManifestDeclaration(
  projectFileShortcutSourceFile,
  "PROJECT_FILE_SHORTCUT_ACTION_SOURCE_MANIFEST",
);
assertSameSet(new Set(staticAppKinds), new Set(typedAppKinds), "App shortcut keyed manifest and discriminated union");
assertSameSet(
  new Set(staticProjectFileKinds),
  new Set(typedProjectFileKinds),
  "project-file shortcut keyed manifest and discriminated union",
);
assert.deepEqual(
  Object.keys(APP_SHORTCUT_ACTION_SOURCE_MANIFEST),
  staticAppKinds,
  "runtime App keyed manifest must be the compiler-checked static manifest",
);
assert.deepEqual(
  Object.keys(PROJECT_FILE_SHORTCUT_ACTION_SOURCE_MANIFEST),
  staticProjectFileKinds,
  "runtime project-file keyed manifest must be the compiler-checked static manifest",
);
const keyboardShortcutSourceExpected = expectedKeyboardShortcutSourceManifest({
  appKinds: staticAppKinds,
  projectFileKinds: staticProjectFileKinds,
});
const keyboardShortcutSourceRaw = await readFile(keyboardShortcutSourceManifestPath, "utf8");
assert.deepEqual(
  validateKeyboardShortcutSourceManifest(keyboardShortcutSourceRaw, keyboardShortcutSourceExpected),
  [],
  "keyboard source manifest must exactly match the typed shortcut unions and keyed manifests",
);

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

assert.deepEqual(resolve("Space", {}, { timelineSurfaceActive: true, timelinePlaying: false, timelineDurationMs: 1000 }), { kind: "toggleTimelinePlayback", operation: "play" });
assert.deepEqual(resolve("Space", {}, { timelineSurfaceActive: true, timelinePlaying: true }), { kind: "toggleTimelinePlayback", operation: "pause" });
assert.deepEqual(resolve("Space", {}, { timelineSurfaceActive: true, timelinePlaying: false, timelineDurationMs: 0 }), { kind: "toggleTimelinePlayback", operation: "none" });
assert.deepEqual(resolve("Space", { shiftKey: true }, { timelineSurfaceActive: true }), { kind: "triggerPreviousCue", enabled: true }, "Shift+Space retains previous-cue behavior");
assert.deepEqual(resolve("Space"), { kind: "triggerNextCue", enabled: true }, "Space outside Timeline retains GO");
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
assert.equal(matrixCases, 15_552, `shortcut matrix contract drifted (got ${matrixCases})`);
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

console.log(`project/app shortcuts: ${matrixCases} matrix cases; App=30 reachable kinds; ProjectFile=3 reachable kinds; source manifest byte-exact`);
