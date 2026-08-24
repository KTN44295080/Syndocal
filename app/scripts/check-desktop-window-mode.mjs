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
const keyboardController = await readFile(
  new URL("../src/createAppKeyboardController.ts", import.meta.url),
  "utf8",
);
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
const app = await readFile(new URL("../src/App.tsx", import.meta.url), "utf8");
const uiLocalization = await readFile(new URL("../src/uiLocalization.ts", import.meta.url), "utf8");
const backend = await readFile(new URL("../src-tauri/src/main.rs", import.meta.url), "utf8");
const packageJson = JSON.parse(await readFile(new URL("../package.json", import.meta.url), "utf8"));
const nativeAcceptance = await readFile(
  new URL("./check-native-window-acceptance.ps1", import.meta.url),
  "utf8",
);
const appSourceFile = ts.createSourceFile(
  "App.tsx",
  app,
  ts.ScriptTarget.ES2022,
  true,
  ts.ScriptKind.TSX,
);
const projectReadInitializationHelpers = [
  "captureProjectAuthorityIdentity",
  "isProjectAuthorityIdentityCurrent",
  "beginProjectReadGeneration",
  "captureProjectReadGuard",
  "projectReadGuardIsCurrent",
];
const projectReadHelperDeclarations = new Map(
  projectReadInitializationHelpers.map((name) => [name, []]),
);
const projectReadHelperCalls = new Map(
  projectReadInitializationHelpers.map((name) => [name, []]),
);
const visitProjectReadInitialization = (node) => {
  if (
    ts.isVariableDeclaration(node)
    && ts.isIdentifier(node.name)
    && projectReadHelperDeclarations.has(node.name.text)
  ) {
    projectReadHelperDeclarations.get(node.name.text).push(node.getStart(appSourceFile));
  }
  if (
    ts.isCallExpression(node)
    && ts.isIdentifier(node.expression)
    && projectReadHelperCalls.has(node.expression.text)
  ) {
    projectReadHelperCalls.get(node.expression.text).push(node.getStart(appSourceFile));
  }
  ts.forEachChild(node, visitProjectReadInitialization);
};
visitProjectReadInitialization(appSourceFile);
for (const helper of projectReadInitializationHelpers) {
  const declarations = projectReadHelperDeclarations.get(helper);
  const calls = projectReadHelperCalls.get(helper);
  assert.equal(declarations.length, 1, `${helper} must have exactly one lexical declaration`);
  assert.ok(calls.length > 0, `${helper} must remain exercised by App`);
  assert.ok(
    declarations[0] < Math.min(...calls),
    `${helper} must be initialized before every App call to prevent a native-startup TDZ`,
  );
}
const mainRuntimeActiveStart = app.indexOf("  const mainRuntimeActive = (current: EngineSnapshot) => {");
const protectedCloseRequestStart = app.indexOf(
  "  const protectedCloseRequestForCurrentState = (current: EngineSnapshot = latestEngineSnapshot): ProtectedCloseRequest | null => {",
);
const protectedCloseUnknownRequestStart = app.indexOf(
  "  const protectedCloseUnknownRequest = (): MainProtectedCloseRequest => ({",
);
const closeProtectionRequiredStart = app.indexOf("  const closeProtectionRequired = () =>");
const protectedCloseMainDetailStart = app.indexOf(
  "  const protectedCloseMainDetail = (request: MainProtectedCloseRequest) => {",
);
const protectedCloseCopyStart = app.indexOf("  const protectedCloseCopy = () => {");
const scheduleApprovedNativeCloseStart = app.indexOf("  const scheduleApprovedNativeClose = () => {");
const protectedCloseRefreshStart = app.indexOf(
  "  const refreshSnapshotForProtectedClose = async (): Promise<EngineSnapshot | null> => {",
);
const protectedCloseCleanupStart = app.indexOf("  onCleanup(() => {", protectedCloseCopyStart);
const protectedCloseCleanupEnd = app.indexOf("  });", protectedCloseCleanupStart);
assert.ok(
  mainRuntimeActiveStart >= 0 &&
    protectedCloseRequestStart > mainRuntimeActiveStart &&
    protectedCloseUnknownRequestStart > protectedCloseRequestStart &&
    closeProtectionRequiredStart > mainRuntimeActiveStart &&
    protectedCloseMainDetailStart > closeProtectionRequiredStart &&
    protectedCloseCopyStart > protectedCloseMainDetailStart &&
    scheduleApprovedNativeCloseStart > protectedCloseCopyStart &&
    protectedCloseRefreshStart > protectedCloseCopyStart &&
    protectedCloseCleanupStart > protectedCloseRefreshStart &&
    protectedCloseCleanupEnd > protectedCloseCleanupStart,
  "protected-close predicates must remain statically discoverable",
);
const mainRuntimeActiveSource = app.slice(mainRuntimeActiveStart, closeProtectionRequiredStart);
const protectedCloseRequestSource = app.slice(protectedCloseRequestStart, closeProtectionRequiredStart);
const protectedCloseUnknownRequestSource = app.slice(protectedCloseUnknownRequestStart, closeProtectionRequiredStart);
const closeProtectionRequiredSource = app.slice(closeProtectionRequiredStart, protectedCloseMainDetailStart);
const protectedCloseMainDetailSource = app.slice(protectedCloseMainDetailStart, protectedCloseCopyStart);
const protectedCloseCopySource = app.slice(protectedCloseCopyStart, protectedCloseCleanupStart);
const scheduleApprovedNativeCloseSource = app.slice(scheduleApprovedNativeCloseStart, protectedCloseRefreshStart);
const protectedCloseRefreshSource = app.slice(protectedCloseRefreshStart, protectedCloseCleanupStart);
const protectedCloseCleanupSource = app.slice(protectedCloseCleanupStart, protectedCloseCleanupEnd + "  });".length);
const protectedCloseSource = app.slice(mainRuntimeActiveStart, protectedCloseCleanupStart);
const closeRequestedStart = app.indexOf("      .onCloseRequested(async (event) => {");
const closeRequestedEnd = app.indexOf("      .then((unlisten)", closeRequestedStart);
assert.ok(closeRequestedStart >= 0 && closeRequestedEnd > closeRequestedStart, "CloseRequested source must remain discoverable");
const closeRequestedSource = app.slice(closeRequestedStart, closeRequestedEnd);

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
assert.ok(
  keyboardController.includes("const hasMappingInteraction =") &&
    keyboardController.includes('options.mappingStageTool() !== "select"') &&
    keyboardController.includes("if (!hasMappingInteraction) return"),
  "an idle Setup workspace must leave Escape available to exit fullscreen",
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
    backend.includes(".is_fullscreen()") &&
    backend.includes("event_window.unmaximize()") &&
    backend.includes("event_window.set_fullscreen(true)") &&
    backend.includes("event_window.set_fullscreen(false)") &&
    backend.includes("event_window.maximize()") &&
    backend.includes("DESKTOP_ESCAPE_SHORTCUT_EVENT") &&
    backend.includes("shortcut_webview.ExecuteScript(") &&
    backend.includes("detail.consumed") &&
    backend.includes('if result != "true"') &&
    backend.includes("fallback_window.maximize()"),
  "Windows must own WebView2 F11/Escape, reject repeats, preserve real DOM Escape consumers, and restore maximized mode through a native fallback",
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
assert.ok(controller.includes("if (!next) await appWindow.maximize()"), "leaving fullscreen must restore the maximized operator workspace");
assert.ok(
  controller.includes('window.addEventListener(DESKTOP_ESCAPE_SHORTCUT_EVENT, handleNativeEscape)') &&
    controller.includes('querySelectorAll<HTMLDialogElement>("dialog[open]")') &&
    controller.includes("forwardingNativeEscape = true") &&
    controller.includes('if (forwardingNativeEscape && event.code === "Escape") return') &&
    controller.includes("detail.consumed = true") &&
    controller.includes("detail.consumed = keyEvent.defaultPrevented") &&
    controller.includes('window.removeEventListener(DESKTOP_ESCAPE_SHORTCUT_EVENT, handleNativeEscape)'),
  "the DOM controller must arbitrate native Escape in dialog, editor, then fullscreen order without recursive handling",
);
assert.ok(
  controller.includes("await appWindow.maximize()"),
  "runtime startup must enforce maximized mode when the platform does not honor the config default",
);
assert.ok(controller.includes('window.addEventListener("resize"'), "native window-mode changes must be resynchronized");
assert.ok(controller.includes('window.removeEventListener("keydown"'), "the global shortcut listener must be cleaned up");
assert.ok(controller.includes('aria-live="polite"'), "window-mode feedback must be announced accessibly");
assert.ok(
  app.includes("data-protected-close-dialog") &&
    app.includes("event.preventDefault()") &&
    app.includes("approveNativeCloseOnce()") &&
    app.includes("await getCurrentWindow().close()") &&
    app.includes("const consumeNativeCloseApproval =") &&
    app.includes("protectedCloseCompletionInFlight") &&
    !app.includes("const confirmProtectedClose =") &&
    !protectedCloseSource.includes("snapshot()") &&
    !protectedCloseSource.includes("UNSAVED SESSION"),
  "protected native close requests must use the operator-styled in-app dialog and reissue one approved close",
);
assert.match(
  closeRequestedSource,
  /\.onCloseRequested\(async \(event\) => \{\s*if \(consumeNativeCloseApproval\(\)\) \{\s*return;\s*\}\s*if \(paneWindow === "timeline"\) \{[\s\S]*?event\.preventDefault\(\);\s*setProtectedCloseRequest\(closeRequest\);\s*return;\s*\}\s*if \(protectedCloseRefreshInFlight \|\| protectedCloseRequest\(\) !== null\) \{\s*event\.preventDefault\(\);\s*return;\s*\}\s*event\.preventDefault\(\);\s*const freshSnapshot = await refreshSnapshotForProtectedClose\(\);/,
  "an approved native close must bypass the listener, keep timeline synchronous, suppress duplicate main checks, and prevent the main close before awaiting a bounded fresh snapshot",
);
assert.ok(
  closeRequestedSource.includes("protectedCloseRefreshInFlight") &&
    closeRequestedSource.includes("protectedCloseRequest() !== null") &&
    !closeRequestedSource.includes("await refreshSnapshot(false, false)") &&
    closeRequestedSource.includes("await refreshSnapshotForProtectedClose()"),
  "main CloseRequested must reject the old unbounded refresh await and suppress repeated checks while one is pending",
);
assert.ok(
  app.includes("const PROTECTED_CLOSE_REFRESH_TIMEOUT_MS = 2_000") &&
    protectedCloseRefreshSource.includes("PROTECTED_CLOSE_REFRESH_TIMEOUT_MS") &&
    protectedCloseRefreshSource.includes("protectedCloseRefreshInFlight") &&
    protectedCloseRefreshSource.includes("refreshSnapshot(false, false)") &&
    protectedCloseRefreshSource.includes("window.setTimeout") &&
    protectedCloseRefreshSource.includes(".then(settle)") &&
    protectedCloseRefreshSource.includes(".catch(() => settle(null))") &&
    protectedCloseRefreshSource.includes("protectedCloseRefreshInFlight = false"),
  "close-specific refresh must be bounded, fail closed on rejection/timeout, and release its duplicate-check guard",
);
const closeRefreshIndex = closeRequestedSource.indexOf("await refreshSnapshotForProtectedClose()");
const closePreventIndex = closeRequestedSource.lastIndexOf("event.preventDefault();", closeRefreshIndex);
assert.ok(
  closePreventIndex >= 0 && closeRefreshIndex > closePreventIndex,
  "main CloseRequested must prevent the initial close before awaiting its bounded fresh snapshot",
);
assert.ok(
  closeRequestedSource.includes("protectedCloseRequestForCurrentState(freshSnapshot)") &&
    !closeRequestedSource.includes("protectedCloseRequestForCurrentState(latestEngineSnapshot)") &&
    closeRequestedSource.includes("setProtectedCloseRequest(protectedCloseUnknownRequest())"),
  "main CloseRequested must capture from the fresh snapshot and fail closed with an unknown-output request",
);
assert.ok(
  closeRequestedSource.includes("scheduleApprovedNativeClose()") &&
    scheduleApprovedNativeCloseSource.includes("window.setTimeout(() =>") &&
    protectedCloseSource.includes("void completeProtectedClose();"),
  "a fresh clean close must schedule the approved reissue on a later task",
);
assert.ok(
  scheduleApprovedNativeCloseSource.includes("protectedCloseRefreshInFlight") &&
    scheduleApprovedNativeCloseSource.includes("closeRequestListenerDisposed") &&
    scheduleApprovedNativeCloseSource.includes("protectedCloseRequest() !== null") &&
    !scheduleApprovedNativeCloseSource.match(/if \(protectedCloseRequest\(\) !== null\) return;/),
  "the scheduled approved close must not bypass a newer refresh check and must guard unmount",
);
assert.ok(
  protectedCloseRefreshSource.includes("protectedCloseRefreshTimeoutId") &&
    protectedCloseRefreshSource.includes("let cancelRefresh: (() => void) | undefined") &&
    protectedCloseRefreshSource.includes("cancelRefresh = () => settle(null)") &&
    protectedCloseRefreshSource.includes("protectedCloseRefreshCancel = cancelRefresh") &&
    protectedCloseRefreshSource.includes("protectedCloseRefreshCancel === cancelRefresh") &&
    protectedCloseRefreshSource.includes("if (closeRequestListenerDisposed)") &&
    protectedCloseCleanupSource.includes("protectedCloseRefreshCancel?.()") &&
    protectedCloseCleanupSource.includes("window.clearTimeout(scheduledApprovedNativeCloseTimer)") &&
    protectedCloseCleanupSource.includes("window.clearTimeout(protectedCloseRefreshTimeoutId)") &&
    closeRequestedSource.includes("const freshSnapshot = await refreshSnapshotForProtectedClose();") &&
    closeRequestedSource.includes("if (closeRequestListenerDisposed) return;"),
  "close timers and post-await work must be owned and stopped when the close listener is disposed",
);
assert.equal(
  (app.match(/await getCurrentWindow\(\)\.close\(\)/g) ?? []).length,
  1,
  "protected close confirmation must issue exactly one native close call",
);
assert.ok(
  app.includes("let latestEngineSnapshot = initialEngineSnapshot;") &&
    protectedCloseRequestSource.includes("mainRuntimeActive(current)") &&
    protectedCloseRequestSource.includes("current: EngineSnapshot = latestEngineSnapshot") &&
    closeProtectionRequiredSource.includes("protectedCloseRequestForCurrentState() !== null") &&
    !mainRuntimeActiveSource.includes("createMemo") &&
    mainRuntimeActiveSource.includes("timelineExecutionIsLive(") &&
    mainRuntimeActiveSource.includes("current.timeline.playing") &&
    mainRuntimeActiveSource.includes("current.clock.source") &&
    mainRuntimeActiveSource.includes("current.clock.external_sync_locked") &&
    mainRuntimeActiveSource.includes("current.clock.external_sync_age_ms") &&
    mainRuntimeActiveSource.includes("current.direct_child_timeline_transports?.some(") &&
    mainRuntimeActiveSource.includes("transport.playing") &&
    mainRuntimeActiveSource.includes("Boolean(current.active_fade)") &&
    mainRuntimeActiveSource.includes("current.active_cue_id !== null") &&
    mainRuntimeActiveSource.includes("current.active_cue_id !== undefined") &&
    mainRuntimeActiveSource.includes("Object.keys(current.active_group_cue_ids ?? {}).length > 0") &&
    mainRuntimeActiveSource.includes("current.video.layers.some((layer) => layer.state.playing)") &&
    mainRuntimeActiveSource.includes("current.effects.some((effect) => effect.enabled)") &&
    mainRuntimeActiveSource.includes("current.programmer.enabled") &&
    mainRuntimeActiveSource.includes("!current.programmer.blind") &&
    mainRuntimeActiveSource.includes("current.programmer.values.length > 0") &&
    !mainRuntimeActiveSource.includes("dmx_outputs") &&
    !mainRuntimeActiveSource.includes("current.output") &&
    !mainRuntimeActiveSource.includes("current.video.outputs"),
  "main runtime activity must use latest authoritative snapshot signals, including timeline clock freshness, video/effects/programmer activity, not configured output routes",
);
assert.ok(
  app.includes("type ProtectedCloseRequest =") &&
    app.includes('reason: "timeline-dirty"') &&
    app.includes('reason: "dirty-only" | "runtime-only" | "dirty-and-runtime" | "output-state-unknown"') &&
    app.includes("projectDirty: boolean") &&
    app.includes("timelineDirty: boolean") &&
    app.includes("runtimeActive: boolean | null") &&
    protectedCloseRequestSource.includes("const projectIsDirty = projectDirty()") &&
    protectedCloseRequestSource.includes("const timelineIsDirty = timelineEditorDirty()") &&
    protectedCloseRequestSource.includes("const runtimeIsActive = mainRuntimeActive(current)") &&
    app.includes("setProtectedCloseRequest(closeRequest)"),
  "protected close must capture a discriminated dirty/runtime reason object from the latest snapshot",
);
assert.ok(
  protectedCloseUnknownRequestSource.includes('reason: "output-state-unknown"') &&
    protectedCloseUnknownRequestSource.includes("projectDirty: projectDirty()") &&
    protectedCloseUnknownRequestSource.includes("timelineDirty: timelineEditorDirty()") &&
    protectedCloseUnknownRequestSource.includes("runtimeActive: null"),
  "refresh failure must capture dirty flags without pretending that runtime is inactive",
);
assert.match(
  protectedCloseRequestSource,
  /if \(paneWindow === "timeline"\) \{[\s\S]*?return timelineEditorDirty\(\)\s*\?\s*\{ pane: "timeline", reason: "timeline-dirty" \}\s*:\s*null;/,
  "timeline child-pane close protection must remain dirty-only",
);
assert.ok(
  !protectedCloseMainDetailSource.includes("projectDirty()") &&
    !protectedCloseMainDetailSource.includes("timelineEditorDirty()") &&
    !protectedCloseMainDetailSource.includes("mainRuntimeActive(") &&
    !protectedCloseCopySource.includes("projectDirty()") &&
    !protectedCloseCopySource.includes("timelineEditorDirty()") &&
    !protectedCloseCopySource.includes("mainRuntimeActive("),
  "protected-close copy must use captured request fields and cannot drift from live state while open",
);
for (const copy of [
  "OUTPUT STATE UNKNOWN",
  "Close Anyway",
  "Discard and Close Anyway",
  "LIVE OUTPUT ACTIVE",
  "Playback/live output is active. Stop and close?",
  "Stop and Close",
  "UNSAVED CHANGES",
  "Discard and Close",
  "UNSAVED CHANGES + LIVE OUTPUT",
  "Discard, Stop and Close",
  "Project changes will be discarded.",
  "Timeline edits will be discarded.",
  "Project changes and Timeline edits will be discarded.",
  "Project changes will be discarded. Playback/live output will stop.",
  "Timeline edits will be discarded. Playback/live output will stop.",
  "Project changes and Timeline edits will be discarded. Playback/live output will stop.",
  "Playback/live output state could not be verified before closing. Keep Syndocal open to avoid an unsafe shutdown.",
  "Project changes will be discarded. Playback/live output state could not be verified before closing.",
  "Timeline edits will be discarded. Playback/live output state could not be verified before closing.",
  "Project changes and Timeline edits will be discarded. Playback/live output state could not be verified before closing.",
]) {
  assert.ok(protectedCloseSource.includes(`\"${copy}\"`), `protected-close copy must cover ${copy}`);
}
assert.ok(
  protectedCloseCopySource.includes('if (request.reason === "output-state-unknown")') &&
    protectedCloseCopySource.includes('eyebrow: translateUiText("OUTPUT STATE UNKNOWN"') &&
    protectedCloseCopySource.includes('translateUiText("Close Anyway"') &&
    protectedCloseCopySource.includes('translateUiText("Discard and Close Anyway"') &&
    protectedCloseCopySource.includes('if (request.reason === "runtime-only")') &&
    protectedCloseCopySource.includes('eyebrow: translateUiText("LIVE OUTPUT ACTIVE"') &&
    protectedCloseCopySource.includes('confirm: translateUiText("Stop and Close"') &&
    protectedCloseCopySource.includes('if (request.reason === "dirty-and-runtime")') &&
    protectedCloseCopySource.includes('eyebrow: translateUiText("UNSAVED CHANGES + LIVE OUTPUT"') &&
    protectedCloseCopySource.includes('confirm: translateUiText("Discard, Stop and Close"') &&
    protectedCloseCopySource.includes('eyebrow: translateUiText("UNSAVED CHANGES"') &&
    protectedCloseCopySource.includes('confirm: translateUiText("Discard and Close"'),
  "protected-close copy must expose distinct runtime-only, dirty-only, and combined eyebrow/action cases",
);
assert.ok(
  protectedCloseMainDetailSource.includes("Playback/live output state could not be verified before closing.") &&
    protectedCloseMainDetailSource.includes("Keep Syndocal open to avoid an unsafe shutdown.") &&
  protectedCloseMainDetailSource.includes("Playback/live output is active. Stop and close?") &&
    protectedCloseMainDetailSource.includes("Playback/live output will stop.") &&
    protectedCloseMainDetailSource.includes("request.projectDirty") &&
    protectedCloseMainDetailSource.includes("request.timelineDirty") &&
    protectedCloseMainDetailSource.includes("request.runtimeActive") &&
    !protectedCloseMainDetailSource.includes("Live DMX output"),
  "protected-close detail must capture unknown, runtime-only, dirty-only, and combined consequences with generic playback/live wording",
);
const unknownCopyStart = protectedCloseCopySource.indexOf('if (request.reason === "output-state-unknown")');
const runtimeOnlyCopyStart = protectedCloseCopySource.indexOf('if (request.reason === "runtime-only")');
const dirtyAndRuntimeCopyStart = protectedCloseCopySource.indexOf('if (request.reason === "dirty-and-runtime")');
const dirtyOnlyCopyStart = protectedCloseCopySource.search(
  /return\s*\{\s*eyebrow:\s*translateUiText\("UNSAVED CHANGES"/,
);
assert.ok(unknownCopyStart >= 0 && runtimeOnlyCopyStart > unknownCopyStart && dirtyAndRuntimeCopyStart > runtimeOnlyCopyStart && dirtyOnlyCopyStart > dirtyAndRuntimeCopyStart);
const unknownCopySource = protectedCloseCopySource.slice(unknownCopyStart, runtimeOnlyCopyStart);
const runtimeOnlyCopySource = protectedCloseCopySource.slice(runtimeOnlyCopyStart, dirtyAndRuntimeCopyStart);
const dirtyOnlyCopySource = protectedCloseCopySource.slice(dirtyOnlyCopyStart);
assert.ok(
  !unknownCopySource.includes("UNSAVED SESSION") &&
    !unknownCopySource.includes("UNSAVED CHANGES") &&
  !runtimeOnlyCopySource.includes("UNSAVED SESSION") &&
    !runtimeOnlyCopySource.includes("Close Without Saving") &&
    !dirtyOnlyCopySource.toLowerCase().includes("output"),
  "runtime-only copy must not claim unsaved-session state, and dirty-only copy must not mention output",
);
for (const [english, japanese] of [
  ["OUTPUT STATE UNKNOWN", "出力状態不明"],
  ["Close Anyway", "確認せずに閉じる"],
  ["Discard and Close Anyway", "破棄して確認せずに閉じる"],
  ["LIVE OUTPUT ACTIVE", "ライブ出力中"],
  ["Stop and Close", "停止して閉じる"],
  ["Discard and Close", "破棄して閉じる"],
  ["Discard, Stop and Close", "破棄して停止して閉じる"],
  ["UNSAVED CHANGES", "未保存の変更"],
  ["UNSAVED CHANGES + LIVE OUTPUT", "未保存の変更 + ライブ出力中"],
  ["Playback/live output is active. Stop and close?", "再生／ライブ出力が有効です。停止して閉じますか？"],
  ["Playback/live output will stop.", "再生／ライブ出力は停止します。"],
  ["Playback/live output state could not be verified before closing. Keep Syndocal open to avoid an unsafe shutdown.", "閉じる前に再生／ライブ出力の状態を確認できませんでした。安全のためSyndocalを開いたままにしてください。"],
  ["Project changes will be discarded. Playback/live output state could not be verified before closing.", "プロジェクトの変更は破棄されます。閉じる前に再生／ライブ出力の状態を確認できませんでした。"],
  ["Timeline edits will be discarded. Playback/live output state could not be verified before closing.", "タイムライン編集は破棄されます。閉じる前に再生／ライブ出力の状態を確認できませんでした。"],
  ["Project changes and Timeline edits will be discarded. Playback/live output state could not be verified before closing.", "プロジェクトの変更とタイムライン編集は破棄されます。閉じる前に再生／ライブ出力の状態を確認できませんでした。"],
]) {
  assert.ok(uiLocalization.includes(english) && uiLocalization.includes(japanese), `localization must cover ${english}`);
}
assert.ok(
  styles.includes(".protectedCloseDialog") &&
    styles.includes(".protectedCloseMark") &&
    styles.includes(".protectedCloseActions button.danger"),
  "protected close must use the dedicated console-style visual hierarchy",
);
assert.ok(
  app.includes('const confirmDiscardProjectChanges = (actionLabel: string): Promise<boolean> => {') &&
    !app.slice(
      app.indexOf('const confirmDiscardProjectChanges = (actionLabel: string): Promise<boolean> => {'),
      app.indexOf('  const applyEngineSnapshot ='),
    ).includes("window.confirm") &&
    app.includes("data-project-discard-dialog") &&
    app.includes('role="alertdialog"') &&
    app.includes("data-project-discard-cancel") &&
    app.includes("data-project-discard-confirm") &&
    app.includes('class="protectedCloseDialog"') &&
    app.includes('class="protectedCloseFrame"') &&
    app.includes('class="protectedCloseMark"') &&
    app.includes('class="protectedCloseActions"') &&
    app.includes('if (!await confirmDiscardProjectChanges("create a new project"))'),
  "project replacement must use the in-app protected-close alertdialog instead of a browser confirmation",
);
for (const copy of [
  "Discard unsaved changes?",
  "Create a new project?",
  "Unsaved project changes will be discarded before continuing.",
  "Unsaved Timeline edits will be discarded before continuing.",
  "Unsaved project changes and Timeline edits will be discarded before continuing.",
  "Discard and Continue",
]) {
  assert.ok(
    uiLocalization.includes(copy),
    `project-discard localization must cover ${copy}`,
  );
}
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
