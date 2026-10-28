// Browser fixture for the 2026-10-28 usability items 1, 2 and 5 (UI lane A):
//  1. the empty Perform matrix starts the first show (picker or drop),
//  2. Video BO is one honest toggle (no confirmation claim, no separate Clear BO),
//  5. FX stays inside Perform (Add effect opens Browser) and the full editor
//     targets the selected layer.
// It mounts the real components with in-memory callbacks; no Tauri backend,
// native window, file picker or output is involved.
import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { createServer } from "vite";

const optionalPort = (name) => {
  const raw = process.env[name];
  if (raw === undefined) return 0;
  const value = Number(raw);
  assert.ok(Number.isInteger(value) && value >= 1 && value <= 65535, name + " must be a port from 1 to 65535");
  return value;
};
const vitePort = optionalPort("SYNDOCAL_VITE_PORT");
const cdpPort = optionalPort("SYNDOCAL_CDP_PORT");
const appRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const evidenceDir = resolve(process.env.SYNDOCAL_PERFORM_FIRST_SHOW_SCREENSHOT_DIR ?? join(appRoot, "../target/qa/perform-first-show"));
const browserPath = [process.env.CHROME_PATH, "C:/Program Files/Google/Chrome/Application/chrome.exe", "C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe", "C:/Program Files/Microsoft/Edge/Application/msedge.exe"].filter(Boolean).find(existsSync);
assert.ok(browserPath, "Chromium browser is required");
const modulePath = "/src/__perform-first-show-fixture.tsx";
const moduleId = join(appRoot, modulePath.slice(1)).replaceAll("\\", "/");

const fixture = String.raw`
import { createRoot, createSignal } from "solid-js";
import { render } from "solid-js/web";
import { VideoPerformMatrix } from "/src/components/VideoPerformMatrix.tsx";
import { VideoMasterControlsPanel } from "/src/components/VideoControlOutputsPanel.tsx";
import { VideoPerformQuickFx } from "/src/components/VideoPerformQuickFx.tsx";
import { EditVideoInspector } from "/src/components/EditVideoInspector.tsx";
import { droppedFilePaths, fileUriToPath, isVideoPerformFirstShowDropTarget } from "/src/videoPerformFirstShow.ts";
import { installUiLocalization } from "/src/uiLocalization.ts";
import "/src/theme.css"; import "/src/styles.css";

createRoot(() => {
const [locale, setLocale] = createSignal("en");
const [available, setAvailable] = createSignal(true);
const [busy, setBusy] = createSignal(false);
const [backend, setBackend] = createSignal(true);
const [blackout, setBlackout] = createSignal(false);
const [fxLayer, setFxLayer] = createSignal({ id: 2, label: "Strobe Wash", isf_effect: null });
// An object signal, like the application snapshot: every refresh is a new layer object.
const [appLayer, setAppLayer] = createSignal({ id: 2 });
const setAppLayerId = (id) => setAppLayer({ id });
const [openFx, setOpenFx] = createSignal(true);
const [inspectorMounted, setInspectorMounted] = createSignal(true);
const log = { starts: [], editorOpens: 0, blackouts: [], browserOpens: 0, fxOpened: 0 };
const bank = { mode: "control", layers: [], layer: null, runtime: null, assets: [], thumbnails: {}, selectedSlotId: null,
  onSelectLayer: () => {}, onSelect: () => {}, onQueue: () => {}, onPreview: () => {}, onMore: () => {}, onImport: () => { throw new Error("library import is not the empty Perform action"); },
  onCancelQueue: () => {}, onSetQuantization: () => {}, transitionKind: "Cut", transitionDurationUnit: "Milliseconds", transitionDurationMilliunits: 750,
  onSetTransitionKind: () => {}, onSetTransitionDurationUnit: () => {}, onSetTransitionDurationMilliunits: () => {} };
const fxLayers = [{ id: 1, label: "Opener Loop", isf_effect: null }, { id: 2, label: "Strobe Wash", isf_effect: null }, { id: 3, label: "Logo Sting", isf_effect: null }];
const layerFx = { layers: fxLayers, onImportIsf: () => {}, onApplyBuiltinIsf: () => {}, onSetIsfEffect: () => {}, onMoveIsfEffect: () => {}, onRemoveIsfEffect: () => {},
  onSetIsfEffectEnabled: () => {}, onResetIsfEffect: () => {}, onSetIsfControl: () => {}, onTriggerIsfEvent: () => {} };
let localizer;
Object.assign(window, { __firstShow: { log, setAvailable, setBusy, setBackend, setBlackout, setFxLayer, setAppLayerId, setOpenFx, setInspectorMounted,
  setLocale: (value) => { setLocale(value); localizer?.refresh(); },
  droppedFilePaths, fileUriToPath, isVideoPerformFirstShowDropTarget } });
render(() => <div class="firstShowStage">
  <div class="firstShowMatrix" style={{ width: "900px", height: "260px" }}>
    <VideoPerformMatrix bank={bank} layerList={{ layers: [] }} runtimes={[]} availabilityById={{}} previewLayerId={null} thumbnailsAuthorized={false}
      onStop={() => {}} onTake={() => {}}
      firstShow={{ get available() { return available(); }, get busy() { return busy(); }, get backendAvailable() { return backend(); },
        onStart: (paths) => { log.starts.push(paths === undefined ? null : [...paths]); }, onOpenEditor: () => { log.editorOpens++; } }} />
  </div>
  <div class="videoControlPanelMixer" style={{ width: "520px" }}>
    <VideoMasterControlsPanel masterOpacity={1} blackout={blackout()} onSetMasterOpacity={() => {}}
      onSetBlackout={(value) => { log.blackouts.push(value); setBlackout(value); }} />
  </div>
  <div data-video-perform-pane="inspector" style={{ width: "446px", height: "236px" }}>
    <VideoPerformQuickFx layer={fxLayer()} layerList={{ layers: fxLayers, isfEventPulseBusy: false }} onOpenEditor={() => {}}
      onOpenBrowser={() => { log.browserOpens++; }} />
  </div>
  {inspectorMounted() ? <div style={{ width: "700px" }}><EditVideoInspector asset={null} availabilityById={{}} layerFx={layerFx}
    selectedLayerId={appLayer().id} openFxEditor={openFx()} onFxEditorOpened={() => { log.fxOpened++; setOpenFx(false); }} /></div> : null}
</div>, document.getElementById("root"));
localizer = installUiLocalization(document.body, locale);
document.documentElement.dataset.theme = "tally";
window.__firstShowReady = true;
});
`;
const plugin = {
  name: "perform-first-show-fixture", enforce: "pre",
  resolveId(id) { if (id === modulePath) return moduleId; },
  load(id) { if (id === moduleId) return fixture; },
  configureServer(server) {
    server.middlewares.use((request, response, next) => {
      if (request.url !== "/__perform-first-show") return next();
      response.setHeader("Content-Type", "text/html");
      response.end(`<!doctype html><html lang="en" data-theme="tally"><head><meta charset="utf-8"><title>Perform first show fixture</title><style>html,body{margin:0;background:var(--theme-bg,#000)}.firstShowStage{display:grid;gap:12px;padding:8px}.firstShowStage>div{background:var(--theme-panel);box-sizing:border-box}</style></head><body><div id="root"></div><script type="module" src="${modulePath}"></script></body></html>`);
    });
  },
};

const delay = (ms) => new Promise((done) => setTimeout(done, ms));
async function until(check, label = "condition", timeout = 30_000) {
  const limit = Date.now() + timeout; let last;
  while (Date.now() < limit) { try { const value = await check(); if (value) return value; } catch (error) { last = error; } await delay(60); }
  throw new Error(`Timed out: ${label}${last ? ` (${last.message})` : ""}`);
}
let server, browser, profile, socket;
const pending = new Map(); const errors = []; let nextId = 0;
const send = (method, params = {}, timeout = 20_000) => new Promise((done, reject) => {
  const id = ++nextId; const timer = setTimeout(() => { pending.delete(id); reject(new Error(`CDP timeout: ${method}`)); }, timeout);
  pending.set(id, { done, reject, timer }); socket.send(JSON.stringify({ id, method, params }));
});
const evaluate = async (expression) => { const value = await send("Runtime.evaluate", { expression, returnByValue: true, awaitPromise: true }); assert.ok(!value.exceptionDetails, JSON.stringify(value.exceptionDetails)); return value.result.value; };
const fs = (code) => evaluate(`(async()=>{const f=window.__firstShow;${code}})()`);
const text = (selector) => evaluate(`document.querySelector(${JSON.stringify(selector)})?.innerText.replace(/\\s+/g,' ').trim()??null`);
const click = (selector) => evaluate(`(()=>{const e=document.querySelector(${JSON.stringify(selector)});if(!e)throw Error('missing '+${JSON.stringify(selector)});if(e.disabled)throw Error('disabled '+${JSON.stringify(selector)});e.click();return true})()`);
const settle = () => delay(120);
const shot = async (name) => { await mkdir(evidenceDir, { recursive: true }); const { data } = await send("Page.captureScreenshot", { format: "png" }, 30_000); await writeFile(join(evidenceDir, `${name}.png`), Buffer.from(data, "base64")); };
// Dispatches a page-level file drop (Chromium builds a real Files DataTransfer).
const drop = (selector, uriList) => evaluate(`(()=>{const t=document.querySelector(${JSON.stringify(selector)});const d=new DataTransfer();d.items.add(new File(['x'],'clip.mp4',{type:'video/mp4'}));${uriList ? `d.setData('text/uri-list',${JSON.stringify(uriList)});` : ""}
  const over=new DragEvent('dragover',{bubbles:true,cancelable:true,dataTransfer:d});t.dispatchEvent(over);const highlighted=t.hasAttribute('data-file-drag-over');
  const dropEvent=new DragEvent('drop',{bubbles:true,cancelable:true,dataTransfer:d});t.dispatchEvent(dropEvent);return {accepted:over.defaultPrevented,highlighted,handled:dropEvent.defaultPrevented,cleared:!t.hasAttribute('data-file-drag-over')}})()`);

try {
  server = await createServer({ root: appRoot, server: { host: "127.0.0.1", port: vitePort, strictPort: vitePort !== 0 }, plugins: [plugin], clearScreen: false, logLevel: "error" });
  await server.listen();
  const profileRoot = resolve(appRoot, "../target/qa/browser-profiles");
  await mkdir(profileRoot, { recursive: true });
  profile = await mkdtemp(join(profileRoot, "syndocal-perform-first-show-"));
  browser = spawn(browserPath, ["--headless=new", "--no-first-run", "--no-default-browser-check", "--remote-debugging-port=" + cdpPort, `--user-data-dir=${profile}`, "about:blank"], { windowsHide: true, stdio: "ignore" });
  const port = cdpPort || await until(async () => Number((await readFile(join(profile, "DevToolsActivePort"), "utf8")).split("\n")[0]), "DevTools port");
  const page = await until(async () => (await (await fetch(`http://127.0.0.1:${port}/json/list`)).json()).find((entry) => entry.type === "page"), "DevTools page");
  socket = new WebSocket(page.webSocketDebuggerUrl);
  await new Promise((done, reject) => { socket.addEventListener("open", done, { once: true }); socket.addEventListener("error", reject, { once: true }); });
  socket.addEventListener("message", ({ data }) => {
    const value = JSON.parse(data);
    if (value.method === "Runtime.exceptionThrown") errors.push(value.params.exceptionDetails);
    if (value.method === "Runtime.consoleAPICalled" && ["error", "warning"].includes(value.params.type)) errors.push(value.params);
    const call = pending.get(value.id); if (!call) return; clearTimeout(call.timer); pending.delete(value.id);
    if (value.error) call.reject(new Error(JSON.stringify(value.error))); else call.done(value.result);
  });
  await send("Runtime.enable"); await send("Page.enable");
  await send("Emulation.setDeviceMetricsOverride", { width: 1280, height: 1000, deviceScaleFactor: 1, mobile: false });
  await send("Page.navigate", { url: `${server.resolvedUrls.local[0]}__perform-first-show` });
  await until(() => evaluate("window.__firstShowReady === true"), "fixture mounted").catch(async (error) => {
    throw new Error(`${error.message}; page errors: ${JSON.stringify(errors).slice(0, 2000)}`);
  });
  await settle();

  // ---- Drop-path helper (shared with App's native drop router) ----
  assert.equal(await fs("return f.fileUriToPath('file:///C:/Clips/Opener%20Loop.mp4')"), "C:\\Clips\\Opener Loop.mp4", "drive file URI becomes a Windows path");
  assert.equal(await fs("return f.fileUriToPath('file://media-server/share/a.mov')"), "\\\\media-server\\share\\a.mov", "UNC file URI becomes a UNC path");
  assert.equal(await fs("return f.fileUriToPath('https://example.invalid/a.mp4')"), null, "non-file URIs are rejected");
  assert.deepEqual(await fs("return f.droppedFilePaths({getData:()=>'# comment\\r\\nfile:///D:/a.mp4\\r\\nhttps://x.invalid/b\\r\\n'})"), ["D:\\a.mp4"], "uri-list keeps only file URIs");
  assert.deepEqual(await fs("return f.droppedFilePaths(null)"), [], "no transfer yields no paths");

  // ---- Item 1: empty Perform starts the first show ----
  const empty = "[data-video-perform-empty]";
  const start = "[data-video-perform-first-show]";
  assert.equal(await text(empty), "No video layers. Pick a file or drop it here. Pick a file", "empty Perform invites a file pick or drop");
  assert.doesNotMatch(await text(empty), /above|Import/, "the dead 'add a file above' text and library-only Import are gone");
  assert.equal(await evaluate(`document.querySelector(${JSON.stringify(empty)}).hasAttribute('data-video-perform-first-show-drop')`), true, "the empty matrix is the native drop target");
  assert.equal(await fs(`const r=document.querySelector(${JSON.stringify(empty)}).getBoundingClientRect();return f.isVideoPerformFirstShowDropTarget(document.elementFromPoint(r.left+12,r.top+12))`), true, "App's drop router recognises any point of the empty matrix");
  const startBox = await evaluate(`(()=>{const r=document.querySelector(${JSON.stringify(start)}).getBoundingClientRect();return {w:r.width,h:r.height}})()`);
  assert.ok(startBox.h >= 44 && startBox.w >= 44, "the start button keeps a full control size " + JSON.stringify(startBox));
  await click(start);
  assert.deepEqual(await fs("return f.log.starts"), [null], "Pick a file runs the first-run flow with its picker (no paths)");
  assert.deepEqual(await drop(empty, "file:///C:/Clips/Opener%20Loop.mp4\r\nfile:///C:/Clips/Logo.png"), { accepted: true, highlighted: true, handled: true, cleared: true }, "a file drag is accepted, highlighted and consumed");
  assert.deepEqual((await fs("return f.log.starts")).at(-1), ["C:\\Clips\\Opener Loop.mp4", "C:\\Clips\\Logo.png"], "a drop with paths skips the picker");
  await drop(empty, null);
  assert.equal((await fs("return f.log.starts")).at(-1), null, "a drop without readable paths opens the same picker instead of doing nothing");
  await shot("en-empty-perform");
  await fs("f.setBusy(true)"); await settle();
  assert.equal(await evaluate(`document.querySelector(${JSON.stringify(start)}).disabled`), true, "busy setup cannot start twice");
  assert.match(await text(start), /Setting Up/, "busy state is named");
  const startsBeforeBusyDrop = (await fs("return f.log.starts")).length;
  assert.equal((await drop(empty, "file:///C:/a.mp4")).handled, false, "a drop while busy is not consumed");
  assert.equal((await fs("return f.log.starts")).length, startsBeforeBusyDrop, "a drop while busy starts nothing");
  await fs("f.setBusy(false);f.setBackend(false)"); await settle();
  assert.equal(await evaluate(`document.querySelector(${JSON.stringify(start)}).disabled`), true, "the browser preview cannot pick native files");
  assert.equal(await evaluate(`document.querySelector(${JSON.stringify(start)}).title`), "Desktop required", "the disabled reason is stated");
  await fs("f.setBackend(true);f.setAvailable(false)"); await settle();
  assert.equal(await text(empty), "No video layers. Add one with Import Media in Edit > Video. Open Edit > Video", "a non-empty show (media already imported) points to where layers are added");
  assert.equal(await evaluate(`document.querySelector(${JSON.stringify(empty)}).hasAttribute('data-video-perform-first-show-drop')`), false, "a non-empty show is not a first-show drop target");
  await click("[data-video-perform-open-editor]");
  assert.equal(await fs("return f.log.editorOpens"), 1, "the editor button opens Edit > Video");
  await fs("f.setAvailable(true);f.setLocale('ja')"); await settle();
  assert.equal(await text(empty), "映像レイヤーがありません。ファイルを選ぶか、ここにドロップしてください。 ファイルを選ぶ", "Japanese empty Perform copy");
  await shot("ja-empty-perform");
  await fs("f.setLocale('en')"); await settle();

  // ---- Item 2: Edit > Video BO is one toggle with a visible state ----
  const bo = ".videoMasterControls [data-video-master-blackout]";
  assert.equal(await evaluate("document.querySelectorAll('.videoMasterControls button').length"), 3, "Out, Full and one Video BO toggle");
  assert.equal(await evaluate("[...document.querySelectorAll('.videoMasterControls button')].some(b=>/Clear BO/.test(b.textContent))"), false, "no separate Clear BO button");
  assert.equal((await text(bo)).toLowerCase(), "video bo: off", "off state is named");
  assert.equal(await evaluate(`document.querySelector(${JSON.stringify(bo)}).getAttribute('aria-pressed')`), "false");
  await click(bo); await settle();
  assert.equal((await text(bo)).toLowerCase(), "video bo: on", "on state is named");
  assert.equal(await evaluate(`document.querySelector(${JSON.stringify(bo)}).getAttribute('aria-pressed')`), "true");
  assert.equal(await evaluate(`document.querySelector(${JSON.stringify(bo)}).classList.contains('engaged')`), true, "engaged styling marks the on state");
  await click(bo); await settle();
  assert.deepEqual(await fs("return f.log.blackouts"), [true, false], "the same toggle turns Video BO on and off directly");
  await fs("f.setLocale('ja')"); await settle();
  assert.equal(await text(bo), "映像BO: オフ", "Japanese off state");
  await click(bo); await settle();
  assert.equal(await text(bo), "映像BO: オン", "Japanese on state");
  await shot("ja-edit-video-bo-on");
  await click(bo); await fs("f.setLocale('en')"); await settle();

  // ---- Item 5: FX stays inside Perform; full editor targets the selected layer ----
  const addEffect = "[data-perform-fx-add-effect]";
  assert.match(await text("[data-perform-quick-fx]"), /No layer FX yet\. Add effect/, "an empty layer offers Add effect");
  assert.doesNotMatch(await text("[data-perform-quick-fx]"), /full editor/, "the empty FX tab no longer sends the user to another workspace");
  await click(addEffect);
  assert.equal(await fs("return f.log.browserOpens"), 1, "Add effect opens the neighbouring Browser tab");
  await fs("f.setLocale('ja')"); await settle();
  assert.match(await text("[data-perform-quick-fx]"), /レイヤーFXはまだありません。 エフェクトを追加/, "Japanese Add effect");
  await fs("f.setLocale('en');f.setFxLayer(null)"); await settle();
  assert.equal(await evaluate(`Boolean(document.querySelector(${JSON.stringify(addEffect)}))`), false, "no layer: nothing to add to");
  assert.match(await text("[data-perform-quick-fx]"), /Select a layer for Quick FX\./);

  const fxSelect = "[data-edit-video-fx-layer]";
  assert.equal(await evaluate("document.querySelector('.editVideoAdvancedDisclosure').open"), true, "Full FX editor request opens the layer effects");
  assert.equal(await fs("return f.log.fxOpened"), 1, "the open request is consumed once");
  assert.equal(await evaluate(`document.querySelector(${JSON.stringify(fxSelect)}).value`), "2", "the editor targets the selected layer, not the first one");
  await fs("f.setAppLayerId(3)"); await settle();
  assert.equal(await evaluate(`document.querySelector(${JSON.stringify(fxSelect)}).value`), "3", "the editor follows a new layer selection");
  await evaluate(`(()=>{const s=document.querySelector(${JSON.stringify(fxSelect)});s.value='1';s.dispatchEvent(new Event('change',{bubbles:true}))})()`); await settle();
  assert.equal(await evaluate(`document.querySelector(${JSON.stringify(fxSelect)}).value`), "1", "a local pick in the editor is honoured");
  await fs("f.setAppLayerId(3)"); await settle();
  assert.equal(await evaluate(`document.querySelector(${JSON.stringify(fxSelect)}).value`), "1", "a snapshot refresh of the same selected layer keeps the local pick");
  await fs("f.setAppLayerId(2)"); await settle();
  assert.equal(await evaluate(`document.querySelector(${JSON.stringify(fxSelect)}).value`), "2", "a new application selection replaces the local pick");
  await fs("f.setInspectorMounted(false)"); await settle(); await fs("f.setInspectorMounted(true)"); await settle();
  assert.equal(await evaluate("document.querySelector('.editVideoAdvancedDisclosure').open"), false, "an ordinary visit to Edit keeps the disclosure closed");
  assert.equal(await fs("return f.log.fxOpened"), 1);

  assert.equal(await evaluate("document.documentElement.scrollWidth<=window.innerWidth"), true, "fixture stays inside the viewport");
  assert.deepEqual(errors, [], JSON.stringify(errors).slice(0, 2000));
  console.log(`Perform first show browser: drop-path parsing, empty Perform pick/drop/busy/desktop/non-empty states (EN/JA), one Video BO toggle with on/off state (EN/JA), Add effect opens Browser, full FX editor targets the selected layer; evidence ${evidenceDir}`);
} finally {
  // Close Chromium through CDP first: a killed launcher can leave its child
  // processes holding the profile and keep this script alive.
  if (socket?.readyState === WebSocket.OPEN) await send("Browser.close", {}, 5_000).catch(() => {});
  socket?.close();
  for (const call of pending.values()) clearTimeout(call.timer);
  if (browser && browser.exitCode === null) { const exited = new Promise((done) => browser.once("exit", done)); browser.kill(); await Promise.race([exited, delay(3000)]); }
  await server?.close();
  if (profile) await rm(profile, { recursive: true, force: true, maxRetries: 5, retryDelay: 200 }).catch(() => {});
}
