import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const shortcutSource = await readFile(new URL("../src/projectFileShortcuts.ts", import.meta.url), "utf8");
const shortcutModule = ts.transpileModule(shortcutSource, {
  compilerOptions: {
    module: ts.ModuleKind.ESNext,
    target: ts.ScriptTarget.ES2022,
    importsNotUsedAsValues: ts.ImportsNotUsedAsValues.Remove,
  },
  fileName: "projectFileShortcuts.ts",
});
const shortcutUrl = `data:text/javascript;base64,${Buffer.from(shortcutModule.outputText).toString("base64")}`;
const { dispatchProjectFileShortcut } = await import(shortcutUrl);

const calls = [];
const actions = {
  saveProject: () => calls.push("save"),
  saveProjectAs: () => calls.push("save-as"),
  loadProject: () => calls.push("open"),
};

const dispatch = ({ code, shiftKey = false, ctrlKey = true, metaKey = false }) => {
  let prevented = 0;
  const handled = dispatchProjectFileShortcut({
    repeat: false,
    altKey: false,
    ctrlKey,
    metaKey,
    shiftKey,
    code,
    preventDefault: () => { prevented += 1; },
  }, actions);
  return { handled, prevented };
};

assert.deepEqual(dispatch({ code: "KeyS" }), { handled: true, prevented: 1 }, "Ctrl+S consumes one key gesture");
assert.deepEqual(calls, ["save"], "Ctrl+S invokes only Save");

calls.length = 0;
assert.deepEqual(dispatch({ code: "KeyS", shiftKey: true }), { handled: true, prevented: 1 }, "Ctrl+Shift+S consumes one key gesture");
assert.deepEqual(calls, ["save-as"], "Ctrl+Shift+S invokes only Save As");

calls.length = 0;
assert.deepEqual(dispatch({ code: "KeyO" }), { handled: true, prevented: 1 }, "Ctrl+O consumes one key gesture");
assert.deepEqual(calls, ["open"], "Ctrl+O invokes only Open");

calls.length = 0;
assert.deepEqual(dispatch({ code: "KeyO", shiftKey: true }), { handled: false, prevented: 0 }, "Ctrl+Shift+O remains unassigned");
assert.deepEqual(calls, [], "Ctrl+Shift+O must not open a project");

calls.length = 0;
assert.deepEqual(dispatch({ code: "KeyS", ctrlKey: false, metaKey: true }), { handled: true, prevented: 1 }, "Cmd+S shares the one-gesture route");
assert.deepEqual(calls, ["save"], "Cmd+S invokes only Save");

const controllerSource = await readFile(new URL("../src/createAppKeyboardController.ts", import.meta.url), "utf8");
assert.match(
  controllerSource,
  /if \(dispatchProjectFileShortcut\(event, options\)\) return;/,
  "the application keyboard controller must use the tested dispatcher",
);
const appSource = await readFile(new URL("../src/App.tsx", import.meta.url), "utf8");
assert.match(
  appSource,
  /window\.addEventListener\("keydown", handleAppKeyDown\)/,
  "the tested controller must remain mounted on the application window",
);
assert.match(
  appSource,
  /handleControlKeyDown\(event\)/,
  "the mounted application handler must delegate to the tested controller",
);

console.log("project shortcuts: 13 assertions passed (Save=1 key, Save As=1 key, Open=1 key)");
