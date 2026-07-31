import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const config = JSON.parse(await readFile(new URL("../src-tauri/tauri.conf.json", import.meta.url), "utf8"));
const nativeAcceptanceConfig = JSON.parse(
  await readFile(new URL("../src-tauri/tauri.native-acceptance.conf.json", import.meta.url), "utf8"),
);
const capability = JSON.parse(
  await readFile(new URL("../src-tauri/capabilities/main.json", import.meta.url), "utf8"),
);
const source = await readFile(new URL("../src/desktopWindowMode.ts", import.meta.url), "utf8");
const controller = await readFile(
  new URL("../src/components/DesktopWindowModeController.tsx", import.meta.url),
  "utf8",
);
const workspaceChrome = await readFile(
  new URL("../src/components/WorkspaceChrome.tsx", import.meta.url),
  "utf8",
);
const styles = await readFile(new URL("../src/styles.css", import.meta.url), "utf8");
const main = await readFile(new URL("../src/main.tsx", import.meta.url), "utf8");
const backend = await readFile(new URL("../src-tauri/src/main.rs", import.meta.url), "utf8");
const packageJson = JSON.parse(await readFile(new URL("../package.json", import.meta.url), "utf8"));
const nativeAcceptance = await readFile(
  new URL("./check-native-window-acceptance.ps1", import.meta.url),
  "utf8",
);

assert.equal(config.app.windows[0].maximized, true, "the primary desktop window must start maximized");
assert.equal(config.app.windows[0].decorations, false, "the primary desktop window must be frameless");
assert.equal(config.app.windows[0].resizable, true, "the frameless primary window must stay resizable");
assert.notEqual(
  nativeAcceptanceConfig.identifier,
  config.identifier,
  "native acceptance must use an isolated application identifier",
);
assert.equal(nativeAcceptanceConfig.app.windows[0].title, "Syndocal QA - Native 1920 Acceptance");
assert.equal(nativeAcceptanceConfig.app.windows[0].width, 1920);
assert.equal(nativeAcceptanceConfig.app.windows[0].height, 1080);
assert.equal(nativeAcceptanceConfig.app.windows[0].maximized, true);
assert.equal(
  nativeAcceptanceConfig.app.windows[0].decorations,
  false,
  "native acceptance must exercise the frameless main-window contract",
);
assert.equal(packageJson.scripts["check:native-window"], "node scripts/run-native-window-acceptance.mjs");
assert.ok(
  packageJson.scripts["check:release-ui"].includes("check:native-window"),
  "the primary release UI gate must include real native maximized/fullscreen acceptance",
);
assert.ok(nativeAcceptance.includes('[string]$MinimumMaximizedClient = "1920x1000"'));
assert.ok(nativeAcceptance.includes('[string]$ExpectedFullscreen = "1920x1080"'));
assert.ok(nativeAcceptance.includes("-VirtualKey 0x7A"), "native acceptance must exercise F11");
assert.ok(nativeAcceptance.includes("-VirtualKey 0x1B"), "native acceptance must exercise Escape restore");
assert.ok(
  nativeAcceptance.includes("-Expected $expectedFullscreenSize -AllowedTolerancePx 0") &&
    nativeAcceptance.includes("-Expected $maximized -AllowedTolerancePx 0"),
  "native acceptance must require exact fullscreen and maximized restoration dimensions",
);
assert.ok(nativeAcceptance.includes("Save-ClientScreenshot"), "native acceptance must capture full-size visual evidence");
assert.ok(
  backend.includes("AcceleratorKeyPressedEventHandler::create") &&
    backend.includes("add_AcceleratorKeyPressed") &&
    backend.includes("args.SetHandled(true)") &&
    backend.includes("physical.WasKeyDown.as_bool()") &&
    backend.includes("shortcut_window") &&
    backend.includes(".emit(DESKTOP_FULLSCREEN_SHORTCUT_EVENT, ())") &&
    backend.includes(".emit(DESKTOP_ESCAPE_SHORTCUT_EVENT, ())"),
  "Windows must synchronously own WebView2 accelerators, reject repeats, and forward F11/Escape arbitration to the DOM",
);
assert.ok(
  !nativeAcceptance.includes("AppActivate") && nativeAcceptance.includes("GetForegroundWindow() -ne $Handle"),
  "native acceptance must inject keys only after verifying the exact isolated QA window handle",
);
assert.ok(
  nativeAcceptance.includes("$preexistingQaWindow = Find-WindowByTitle") &&
    nativeAcceptance.includes("$qaProcess.StartTime.ToUniversalTime()") &&
    nativeAcceptance.includes("[IO.Path]::GetFullPath($qaProcess.Path) -ine $expectedQaExecutable") &&
    nativeAcceptance.includes("if ($qaWindowVerified -and $qaWindow -ne [IntPtr]::Zero)"),
  "native acceptance must reject stale same-title windows and bind evidence to the current isolated executable",
);
assert.ok(
  nativeAcceptance.includes("PrintWindow($Handle, $deviceContext, 3)") &&
    nativeAcceptance.includes("Save-VerifiedClientScreenshot") &&
    nativeAcceptance.includes("UniqueSampledColors -ge 32") &&
    nativeAcceptance.includes("visual_metrics"),
  "native acceptance must capture the exact HWND and reject blank or visually unready evidence",
);
assert.ok(
  capability.permissions.includes("core:window:allow-maximize"),
  "the main window must be allowed to enforce its operational maximized mode",
);
assert.ok(
  capability.permissions.includes("core:window:allow-set-fullscreen"),
  "the main window must be allowed to change fullscreen state",
);
for (const permission of [
  "core:window:allow-close",
  "core:window:allow-minimize",
  "core:window:allow-start-dragging",
  "core:window:allow-start-resize-dragging",
  "core:window:allow-toggle-maximize",
]) {
  assert.ok(capability.permissions.includes(permission), `frameless chrome requires ${permission}`);
}
assert.ok(
  main.includes("shouldMountDesktopWindowModeController(window.location.search)"),
  "the app entrypoint must scope desktop window control by route",
);
assert.ok(main.includes("<DesktopWindowModeController>"), "desktop window control must wrap the primary app");
assert.match(
  main,
  /shouldMountDesktopWindowModeController\(window\.location\.search\)\s*\?\s*\([\s\S]*?<DesktopWindowModeController>[\s\S]*?<App \/>[\s\S]*?<\/DesktopWindowModeController>[\s\S]*?\)\s*:\s*\(\s*<App \/>/,
  "video output routes must render App directly, outside DesktopWindowModeController",
);
assert.ok(controller.includes("appWindow.setFullscreen(next)"), "fullscreen changes must use the Tauri window API");
assert.ok(
  controller.includes(".listen(DESKTOP_FULLSCREEN_SHORTCUT_EVENT") &&
    controller.includes(".listen(DESKTOP_ESCAPE_SHORTCUT_EVENT, forwardNativeEscape)") &&
    controller.includes('querySelectorAll<HTMLDialogElement>("dialog[open]")') &&
    controller.includes('new Event("cancel", { cancelable: true })') &&
    controller.includes("if (dialog.dispatchEvent(cancelEvent)) dialog.close()") &&
    controller.includes("if (closeOpenDialogForNativeEscape()) return") &&
    controller.includes('new KeyboardEvent("keydown"') &&
    controller.includes("document.activeElement ?? window") &&
    controller.includes("unlistenNativeFullscreen?.()") &&
    controller.includes("unlistenNativeEscape?.()"),
  "the DOM controller must own native F11/Escape arbitration and clean up both listeners",
);
assert.ok(
  controller.includes("await appWindow.maximize()"),
  "runtime startup must enforce maximized mode when the platform does not honor the config default",
);
assert.ok(controller.includes('window.addEventListener("resize"'), "native window-mode changes must be resynchronized");
assert.ok(controller.includes('window.removeEventListener("keydown"'), "the global shortcut listener must be cleaned up");
assert.ok(controller.includes('aria-live="polite"'), "window-mode feedback must be announced accessibly");
assert.ok(
  controller.includes("DESKTOP_RESIZE_DIRECTIONS") &&
    controller.includes("await appWindow.startResizeDragging(direction)") &&
    controller.includes("await appWindow.isFullscreen()") &&
    controller.includes("await appWindow.isMaximized()") &&
    controller.includes("<DesktopWindowResizeZones />"),
  "windowed frameless mode must expose eight native resize-drag boundaries and suppress them while maximized/fullscreen",
);
assert.ok(
  styles.includes(".desktopResizeZoneNorthEast") &&
    styles.includes(".desktopResizeZoneSouthEast") &&
    styles.includes(".desktopResizeZoneSouthWest") &&
    styles.includes(".desktopResizeZoneNorthWest") &&
    styles.includes('html[data-window-mode="maximized"] .desktopResizeZones') &&
    styles.includes('html[data-window-mode="fullscreen"] .desktopResizeZones'),
  "frameless resize zones must include corners and stay inactive outside windowed mode",
);
assert.ok(
  workspaceChrome.includes('<header class="topbar" data-tauri-drag-region>') &&
    workspaceChrome.includes('<div class="topbarLeft" data-tauri-drag-region ref={projectMenuRoot}>') &&
    workspaceChrome.includes('<div class="status" data-tauri-drag-region>') &&
    workspaceChrome.includes("<strong data-tauri-drag-region>Syndocal</strong>") &&
    workspaceChrome.includes("<span data-tauri-drag-region>{props.projectLabel}</span>") &&
    workspaceChrome.includes('data-window-control="minimize"') &&
    workspaceChrome.includes('data-window-control="maximize"') &&
    workspaceChrome.includes('data-window-control="close"'),
  "the T25-E topbar must expose each top-level empty background as a drag region and keep all three window controls",
);
assert.doesNotMatch(
  workspaceChrome,
  /<(?:button|input|select|textarea)\b[^>]*data-tauri-drag-region/,
  "interactive topbar controls must not become native drag regions",
);
assert.ok(
  workspaceChrome.includes('data-project-menu-action="save"') &&
    workspaceChrome.includes('data-project-menu-action="load"') &&
    !workspaceChrome.includes('class="projectAction"'),
  "Save and Load must be absent from the topbar while remaining available in the project menu",
);
assert.ok(
  workspaceChrome.includes('aria-label="最小化"') &&
    workspaceChrome.includes('aria-label="最大化または元に戻す"') &&
    workspaceChrome.includes('aria-label="閉じる"'),
  "window controls must expose Japanese accessible names",
);
assert.ok(
  workspaceChrome.includes("await appWindow.minimize()") &&
    workspaceChrome.includes("await appWindow.toggleMaximize()") &&
    workspaceChrome.includes("await appWindow.close()") &&
    !workspaceChrome.includes("appWindow.destroy()"),
  "custom controls must use native minimize/toggle and route close through CloseRequested instead of a forced destroy",
);
assert.ok(
  workspaceChrome.includes("if (!isTauriRuntime()) return"),
  "window controls must remain browser-rendered no-ops outside Tauri",
);
assert.ok(
  controller.includes("const documentRoot = document.documentElement") &&
    controller.includes("documentRoot.setAttribute(") &&
    controller.includes("documentRoot.setAttribute(DESKTOP_WINDOW_MODE_ATTRIBUTE, nextMode)") &&
    controller.includes("documentRoot.removeAttribute(DESKTOP_WINDOW_MODE_ATTRIBUTE)"),
  "the controller must publish window mode on the document root and remove an attribute it owns",
);
assert.match(
  controller,
  /if \(hadPreviousWindowMode\) \{[\s\S]*?documentRoot\.setAttribute\(DESKTOP_WINDOW_MODE_ATTRIBUTE, previousWindowMode \?\? ""\);[\s\S]*?\} else \{[\s\S]*?documentRoot\.removeAttribute\(DESKTOP_WINDOW_MODE_ATTRIBUTE\);/,
  "cleanup must restore a pre-existing document window-mode value instead of overwriting it",
);

const transpiled = ts.transpileModule(source, {
  compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 },
  fileName: "desktopWindowMode.ts",
});
const shortcuts = await import(`data:text/javascript;base64,${Buffer.from(transpiled.outputText).toString("base64")}`);

assert.equal(shortcuts.DESKTOP_WINDOW_MODE_ATTRIBUTE, "data-window-mode");
assert.equal(shortcuts.desktopWindowModeFromWindowState(false, false), "windowed");
assert.equal(shortcuts.desktopWindowModeFromWindowState(false, true), "maximized");
assert.equal(shortcuts.desktopWindowModeFromWindowState(true, false), "fullscreen");
assert.equal(shortcuts.desktopWindowModeFromWindowState(true, true), "fullscreen");

assert.equal(shortcuts.shouldMountDesktopWindowModeController(""), true);
assert.equal(shortcuts.shouldMountDesktopWindowModeController("?syndocalViewportFixture=primary"), true);
assert.equal(shortcuts.shouldMountDesktopWindowModeController("?syndocalPaneWindow=stage"), false);
assert.equal(shortcuts.shouldMountDesktopWindowModeController("?syndocalPaneWindow=timeline"), false);
assert.equal(shortcuts.shouldMountDesktopWindowModeController("?videoOutputId=1"), false);
assert.equal(shortcuts.shouldMountDesktopWindowModeController("?testPattern=1&videoOutputId=12"), false);
assert.equal(shortcuts.shouldMountDesktopWindowModeController("?videoOutputId=0"), true);
assert.equal(shortcuts.shouldMountDesktopWindowModeController("?videoOutputId=invalid"), true);
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
    event({ code: "Escape", editableTarget: false, defaultPrevented: true }),
    true,
  ),
  null,
  "a drawer or other non-editable surface that consumes Escape must keep fullscreen",
);
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

console.log("primary-only window mode controller and safe desktop fullscreen shortcuts ok");
