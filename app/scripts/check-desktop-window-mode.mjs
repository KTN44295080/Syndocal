import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const config = JSON.parse(await readFile(new URL("../src-tauri/tauri.conf.json", import.meta.url), "utf8"));
const capability = JSON.parse(
  await readFile(new URL("../src-tauri/capabilities/main.json", import.meta.url), "utf8"),
);
const source = await readFile(new URL("../src/desktopWindowMode.ts", import.meta.url), "utf8");
const controller = await readFile(
  new URL("../src/components/DesktopWindowModeController.tsx", import.meta.url),
  "utf8",
);
const main = await readFile(new URL("../src/main.tsx", import.meta.url), "utf8");

assert.equal(config.app.windows[0].maximized, true, "the primary desktop window must start maximized");
assert.ok(
  capability.permissions.includes("core:window:allow-set-fullscreen"),
  "the main window must be allowed to change fullscreen state",
);
assert.ok(main.includes("<DesktopWindowModeController>"), "desktop window control must wrap the app entrypoint");
assert.ok(controller.includes("appWindow.setFullscreen(next)"), "fullscreen changes must use the Tauri window API");
assert.ok(controller.includes('window.addEventListener("resize"'), "native window-mode changes must be resynchronized");
assert.ok(controller.includes('window.removeEventListener("keydown"'), "the global shortcut listener must be cleaned up");
assert.ok(controller.includes('aria-live="polite"'), "window-mode feedback must be announced accessibly");

const transpiled = ts.transpileModule(source, {
  compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 },
  fileName: "desktopWindowMode.ts",
});
const shortcuts = await import(`data:text/javascript;base64,${Buffer.from(transpiled.outputText).toString("base64")}`);
const event = (overrides = {}) => ({
  code: "KeyA",
  repeat: false,
  altKey: false,
  ctrlKey: false,
  metaKey: false,
  shiftKey: false,
  isComposing: false,
  defaultPrevented: false,
  editableTarget: false,
  ...overrides,
});

assert.equal(shortcuts.desktopWindowShortcutAction(event({ code: "F11" }), false), "toggleFullscreen");
assert.equal(shortcuts.desktopWindowShortcutAction(event({ code: "F11", editableTarget: true }), false), "toggleFullscreen");
assert.equal(shortcuts.desktopWindowShortcutAction(event({ code: "Escape" }), true), "exitFullscreen");
assert.equal(shortcuts.desktopWindowShortcutAction(event({ code: "Escape" }), false), null);
assert.equal(
  shortcuts.desktopWindowShortcutAction(
    event({ code: "Escape", editableTarget: true, defaultPrevented: true }),
    true,
  ),
  null,
  "a form editor that consumes Escape must keep control of the key",
);
for (const guarded of [
  { code: "F11", repeat: true },
  { code: "F11", ctrlKey: true },
  { code: "F11", isComposing: true },
  { code: "F11", defaultPrevented: true },
]) {
  assert.equal(shortcuts.desktopWindowShortcutAction(event(guarded), false), null);
}

console.log("maximized startup and safe desktop fullscreen shortcuts ok");
