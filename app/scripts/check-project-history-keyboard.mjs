import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import ts from "typescript";

// Execute the production factory and its real pure dependencies, with no Tauri,
// browser, or copied recovery implementation.
const sourceRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../src");
const cache = new Map();
function load(name) {
  const file = path.resolve(sourceRoot, `${name}.ts`);
  if (cache.has(file)) return cache.get(file);
  const result = ts.transpileModule(readFileSync(file, "utf8"), {
    compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.CommonJS },
    fileName: file,
    reportDiagnostics: true,
  });
  assert.equal(result.diagnostics?.filter((item) => item.category === ts.DiagnosticCategory.Error).length ?? 0, 0);
  const module = { exports: {} };
  cache.set(file, module.exports);
  const requireLocal = (specifier) => {
    assert.ok(specifier.startsWith("./"), `unexpected external dependency: ${specifier}`);
    return load(specifier.slice(2));
  };
  new Function("require", "module", "exports", result.outputText)(requireLocal, module, module.exports);
  return module.exports;
}
const { createAppKeyboardController } = load("createAppKeyboardController");
// DOM-shaped focus targets exercise the real classifier/controller without
// launching a browser. Native text-editing behavior itself remains a UI gate.
class FocusElement {
  constructor(tagName, { type = "text", readOnly = false, editable = false, parent = null } = {}) {
    Object.assign(this, { tagName: tagName.toUpperCase(), type, readOnly, isContentEditable: editable, parent });
  }
  closest() {
    if (["INPUT", "TEXTAREA"].includes(this.tagName) || this.isContentEditable) return this;
    return this.parent?.closest() ?? null;
  }
}
globalThis.HTMLElement = FocusElement;
const calls = [];
const nothing = () => {};
const options = new Proxy({
  snapshot: () => ({ cues: [], video: { blackout: false }, blackout: false }),
  activeTimeline: () => ({ playing: false, duration_ms: 1000 }),
  workspaceTab: () => "control",
  mappingStageTool: () => "select",
  mappingDrag: () => null,
  mappingMarquee: () => null,
  mappingViewportPanDrag: () => null,
  timelineSurfaceActive: () => true,
  undoProject: () => calls.push("undo"),
  redoProject: () => calls.push("redo"),
  playTimeline: () => calls.push("play"),
  triggerNextCue: () => calls.push("next"),
}, { get: (target, key) => key in target ? target[key] : nothing });
const { handleControlKeyDown } = createAppKeyboardController(options);
const press = (target, extra = {}) => {
  let prevented = false;
  const before = calls.length;
  handleControlKeyDown({ target, code: "KeyZ", key: "z", ctrlKey: true,
    metaKey: false, shiftKey: false, altKey: false, repeat: false,
    preventDefault() { prevented = true; }, ...extra });
  return { calls: calls.slice(before), prevented };
};
const historyKeys = [
  [{}, "undo"],
  [{ ctrlKey: false, metaKey: true }, "undo"],
  [{ shiftKey: true }, "redo"],
  [{ code: "KeyY", key: "y" }, "redo"],
];
const nonText = [
  ...["range", "checkbox", "radio", "color", "file", "button", "submit", "reset", "date", "time", "month", "week", "datetime-local"].map(type => new FocusElement("input", { type })),
  new FocusElement("select"),
  new FocusElement("input", { readOnly: true }),
  new FocusElement("textarea", { readOnly: true }),
];
for (const target of nonText) {
  for (const [key, action] of historyKeys) {
    assert.deepEqual(press(target, key), { calls: [action], prevented: true }, `${target.tagName}/${target.type}: project history`);
  }
  assert.deepEqual(press(target, { code: "Space", key: " ", ctrlKey: false }), { calls: [], prevented: false }, "history-specific guard must not enable transport shortcuts in controls");
}
const editable = new FocusElement("div", { editable: true });
const text = [
  ...["text", "search", "url", "tel", "email", "password", "number"].map(type => new FocusElement("input", { type })),
  new FocusElement("textarea"), editable,
  new FocusElement("span", { parent: editable }),
];
for (const target of text) {
  for (const [key] of historyKeys) {
    assert.deepEqual(press(target, key), { calls: [], prevented: false }, `${target.tagName}/${target.type}: preserve native text undo`);
  }
}
for (const target of [null, new FocusElement("button"), new FocusElement("div")]) {
  assert.deepEqual(press(target), { calls: ["undo"], prevented: true });
  assert.deepEqual(press(target, { altKey: true }), { calls: [], prevented: false });
  assert.deepEqual(press(target, { repeat: true }), { calls: [], prevented: false });
}
console.log("PASS actual keyboard controller: Ctrl/Meta+Z, Ctrl+Shift+Z and Ctrl+Y reach project history from non-text controls; text editors retain native Undo and input transport guards remain intact.");
