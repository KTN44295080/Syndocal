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
globalThis.HTMLElement = class {};
let playing = false;
let duration = 1000;
let surface = true;
let rootPlaying = false;
const calls = [];
const nothing = () => {};
const options = new Proxy({
  snapshot: () => ({ timeline: { playing: rootPlaying, duration_ms: 0 }, cues: [{ id: 1 }], video: { blackout: false }, blackout: false }),
  activeTimeline: () => ({ playing, duration_ms: duration }),
  workspaceTab: () => "control",
  mappingStageTool: () => "select",
  mappingDrag: () => null,
  mappingMarquee: () => null,
  mappingViewportPanDrag: () => null,
  cuePadStartIndex: () => 0,
  timelineSurfaceActive: () => surface,
  playTimeline: () => { calls.push("play"); playing = true; },
  pauseTimeline: () => { calls.push("pause"); playing = false; },
  triggerNextCue: () => calls.push("next"),
  triggerPreviousCue: () => calls.push("previous"),
}, { get: (target, key) => key in target ? target[key] : nothing });
const { handleControlKeyDown } = createAppKeyboardController(options);
let prevented = 0;
const press = (extra = {}) => handleControlKeyDown({ code: "Space", key: " ", target: null, repeat: false, shiftKey: false, ctrlKey: false, metaKey: false, altKey: false, preventDefault() { prevented++; }, ...extra });
press();
press();
press();
assert.deepEqual(calls, ["play", "pause", "play"], "Space toggles selected child runtime even with stopped empty root");
assert.equal(prevented, 3);
press({ repeat: true });
assert.equal(calls.length, 3, "held Space does not toggle repeatedly");
playing = false; rootPlaying = true;
press();
assert.equal(calls.at(-1), "play", "playing root cannot override selected stopped child");
playing = false; duration = 0;
const beforeEmpty = calls.length;
press();
assert.equal(calls.length, beforeEmpty, "empty selected Timeline neither plays nor triggers another cue");
surface = false;
press();
assert.equal(calls.at(-1), "next");
surface = true;
press({ shiftKey: true });
assert.equal(calls.at(-1), "previous");
for (const modifier of ["ctrlKey", "metaKey", "altKey"]) {
  const count = calls.length;
  press({ [modifier]: true });
  assert.equal(calls.length, count, `${modifier}+Space stays unassigned`);
}
const app = readFileSync(path.join(sourceRoot, "App.tsx"), "utf8");
assert.match(app, /createAppKeyboardController\(\{[\s\S]*?snapshot,[\s\S]*?activeTimeline,[\s\S]*?playTimeline,[\s\S]*?pauseTimeline,/, "App injects selected timeline and canonical transport callbacks");
console.log("PASS selected Timeline Space play/pause/resume, root-child mismatch, empty, repeat, modifier and outside-surface routing");