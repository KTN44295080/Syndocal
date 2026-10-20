// Browser plugin not available; use the repository's raw Chromium CDP approach.
// This browser fixture verifies layout/control routing, never native or physical output.
import assert from "node:assert/strict";
import ts from "typescript";
import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { reserveBrowserCheckPort } from "./browser-check-ports.mjs";
import { tmpdir } from "node:os";
import { basename, dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { createServer } from "vite";

const appRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const evidenceDir = resolve(process.env.SYNDOCAL_IA_SCREENSHOT_DIR ?? join(appRoot, "../target/qa/ia-15"));
const browserPath = [process.env.CHROME_PATH,
  "C:/Program Files/Google/Chrome/Application/chrome.exe",
  "C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe",
  "C:/Program Files/Microsoft/Edge/Application/msedge.exe",
].filter(Boolean).find(existsSync);
assert.ok(browserPath, "Set CHROME_PATH to an installed Chromium browser");
const sleep = (ms) => new Promise((done) => setTimeout(done, ms));
async function waitFor(check, label, timeout = 30_000) {
  const deadline = Date.now() + timeout;
  let lastError;
  while (Date.now() < deadline) {
    try { const result = await check(); if (result) return result; } catch (error) { lastError = error; }
    await sleep(75);
  }
  throw new Error(`Timed out: ${label}${lastError ? ` (${lastError.message})` : ""}`);
}

class Cdp {
  pending = new Map();
  nextId = 0;
  errors = [];
  dialogs = [];
  acceptDialogs = true;
  async connect(url) {
    this.socket = new WebSocket(url);
    await new Promise((done, reject) => {
      const timer = setTimeout(() => reject(new Error("CDP connection timed out")), 10_000);
      this.socket.addEventListener("open", () => { clearTimeout(timer); done(); }, { once: true });
      this.socket.addEventListener("error", () => { clearTimeout(timer); reject(new Error("CDP connection failed")); }, { once: true });
    });
    this.socket.addEventListener("message", ({ data }) => {
      const message = JSON.parse(data);
      if (message.method === "Runtime.exceptionThrown") this.errors.push(message.params.exceptionDetails);
      if (message.method === "Runtime.consoleAPICalled" && ["error", "warning"].includes(message.params.type)) this.errors.push(message.params);
      if (message.method === "Page.javascriptDialogOpening") { this.dialogs.push(message.params); void this.send("Page.handleJavaScriptDialog", { accept: this.acceptDialogs }); }
      const call = this.pending.get(message.id);
      if (!call) return;
      clearTimeout(call.timer);
      this.pending.delete(message.id);
      if (message.error) call.reject(new Error(`${call.method}: ${JSON.stringify(message.error)}`));
      else call.done(message.result);
    });
    this.socket.addEventListener("close", () => {
      for (const call of this.pending.values()) { clearTimeout(call.timer); call.reject(new Error("CDP disconnected")); }
      this.pending.clear();
    });
  }
  send(method, params = {}, timeout = 15_000) {
    return new Promise((done, reject) => {
      const id = ++this.nextId;
      const timer = setTimeout(() => { this.pending.delete(id); reject(new Error(`CDP timeout: ${method}`)); }, timeout);
      this.pending.set(id, { done, reject, timer, method });
      this.socket.send(JSON.stringify({ id, method, params }));
    });
  }
  async evaluate(expression) {
    const result = await this.send("Runtime.evaluate", { expression, returnByValue: true, awaitPromise: true });
    assert.ok(!result.exceptionDetails, JSON.stringify(result.exceptionDetails));
    return result.result.value;
  }
  // Headless Chromium occasionally stalls one screenshot (most often with a modal
  // dialog open); evidence capture retries instead of failing the layout contract.
  async capture(params = { format: "png" }, timeout = 30_000) {
    for (let attempt = 1; ; attempt++) {
      try { return await this.send("Page.captureScreenshot", params, timeout); }
      catch (error) {
        if (attempt >= 3 || !/CDP timeout/.test(error.message)) throw error;
        await this.send("Page.bringToFront").catch(() => {});
      }
    }
  }
  async key(key, code = key, virtualKey) {
    const params = { key, code, ...(virtualKey ? { windowsVirtualKeyCode: virtualKey, nativeVirtualKeyCode: virtualKey } : {}) };
    await this.send("Input.dispatchKeyEvent", { type: "keyDown", ...params, ...(key === "Enter" ? { text: "\r", unmodifiedText: "\r" } : {}) });
    await this.send("Input.dispatchKeyEvent", { type: "keyUp", ...params });
  }
}

// Exercise the canonical modules, including one-time migration and stable IDs.
async function moduleFromSource(path) {
  const source = await readFile(new URL(path, import.meta.url), "utf8");
  const code = ts.transpileModule(source, { compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 } }).outputText;
  return import(`data:text/javascript;base64,${Buffer.from(code).toString("base64")}`);
}
const modes = await moduleFromSource("../src/uiModes.ts");
const storage = await moduleFromSource("../src/workspaceLayoutStorage.ts");
assert.deepEqual(modes.editDomainModes.map(x => x.id), ["edit", "mixer", "live"]);
// Control keeps at most four tabs. Timeline playback is the fourth and reuses the stable "live" mode id.
assert.deepEqual(modes.controlDomainModes.map(x => [x.id, x.mode]), [["lighting", "edit"], ["video", "perform"], ["both", "both"], ["timeline", "live"]]);
assert.ok(modes.controlDomainModes.length <= 4, "Control has at most four tabs");
assert.deepEqual(modes.controlModes.map(x => x.id), ["edit", "live", "mixer", "both", "perform"], "persisted ControlMode ids are unchanged");
assert.equal(modes.controlModeForDomain("timeline"), "live");
assert.equal(modes.controlDomainForMode("live"), "timeline");
assert.equal(modes.isTimelinePlaybackWorkspace("touch", "live"), true);
assert.equal(modes.isTimelinePlaybackWorkspace("control", "live"), false, "Edit > Timeline is authoring");
assert.equal(modes.workspaceModeForTabShortcut("touch", "Digit4"), "live", "Alt+4 selects Control > Timeline");
assert.equal(modes.workspaceModeForTabShortcut("control", "Digit4"), null, "Edit has only three tabs");
for (const [mode, domain] of [["perform", "video"], ["both", "both"]]) {
  const old = { workspace_tab: "control", control_mode: mode, perform_top_split_ratio: .66, perform_lower_split_ratio: .6 };
  const migrated = storage.workspaceLayoutFromUnknown(old);
  assert.equal(migrated.workspace_tab, "touch");
  assert.equal(migrated.control_mode, mode);
  assert.equal(migrated.touch_control_domain, domain);
  assert.equal(migrated.perform_top_split_ratio, .66);
  assert.equal(migrated.perform_lower_split_ratio, .6);
  assert.deepEqual(storage.workspaceLayoutFromUnknown(migrated), migrated, "migration is idempotent");
}
// Before Task 15 the Control domain was ephemeral and always booted Lighting.
// A dormant mode saved from Edit must not silently select a new live domain.
for (const mode of ["edit", "mixer", "live", "perform", "both"]) {
  for (const domain of [undefined, "invalid", null]) {
    const legacy = storage.workspaceLayoutFromUnknown({workspace_tab:"touch",control_mode:mode,touch_control_domain:domain});
    assert.equal(legacy.workspace_tab,"touch");
    assert.equal(legacy.touch_control_domain,"lighting",`legacy touch/${mode} defaults to historical Lighting`);
    assert.equal(legacy.control_mode,"edit",`legacy touch/${mode} normalizes the effective Lighting mode`);
  }
}
// Setup with a dormant Perform/Both Edit mode never reopens a removed Edit desk.
for (const [mode,domain] of [["perform","video"],["both","both"]]) {
  const dormant = storage.workspaceLayoutFromUnknown({workspace_tab:"setup",control_mode:mode});
  assert.equal(dormant.workspace_tab,"setup");
  assert.equal(dormant.control_mode,"edit","Edit never restores Perform/Both");
  assert.equal(dormant.touch_control_domain,domain,"dormant live desk is remembered as the equivalent Control domain");
  assert.deepEqual(storage.workspaceLayoutFromUnknown(dormant),dormant,"dormant migration is idempotent");
}
for (const [mode,domain] of [["perform","video"],["both","both"]]) {
  const current = storage.workspaceLayoutFromUnknown({workspace_tab:"touch",control_mode:mode,touch_control_domain:domain});
  assert.equal(current.workspace_tab,"touch");
  assert.equal(current.control_mode,mode);
  assert.equal(current.touch_control_domain,domain,"explicit current Control domain survives restore");
}
// Task 21: Timeline authoring (Edit, workspace "control") is separate from playback (Control, workspace
// "touch"). Both reuse the stable "live" mode id; the domain field disambiguates and is additive.
{
  const playback = storage.workspaceLayoutFromUnknown({workspace_tab:"touch",control_mode:"live",touch_control_domain:"timeline"});
  assert.equal(playback.workspace_tab,"touch");
  assert.equal(playback.control_mode,"live");
  assert.equal(playback.touch_control_domain,"timeline","Control > Timeline survives restore");
  assert.deepEqual(storage.workspaceLayoutFromUnknown(playback),playback,"playback layout is idempotent");
  // The domain alone (stale mode) still lands on the playback surface rather than a removed desk.
  const staleMode = storage.workspaceLayoutFromUnknown({workspace_tab:"touch",control_mode:"edit",touch_control_domain:"timeline"});
  assert.equal(staleMode.control_mode,"live");
  const authoring = storage.workspaceLayoutFromUnknown({workspace_tab:"control",control_mode:"live",timeline_desk_surface:"automation"});
  assert.equal(authoring.workspace_tab,"control","a saved Edit > Timeline layout still opens Edit > Timeline");
  assert.equal(authoring.control_mode,"live");
  assert.equal(authoring.timeline_desk_surface,"automation","authoring desk surface ids are unchanged");
  assert.equal(authoring.touch_control_domain,"lighting","no Control domain is invented for an Edit layout");
  const future = storage.workspaceLayoutFromUnknown({workspace_tab:"touch",control_mode:"live",touch_control_domain:"timeline-v2"});
  assert.equal(future.touch_control_domain,"lighting","unknown future domains fail closed to Lighting");
  assert.equal(future.control_mode,"edit");
}
// No second transport path: the playback panel only receives callbacks and imports no backend module,
// and App wires them to the same ticketed commands as the top bar.
{
  const panelSource = await readFile(new URL("../src/components/ControlTimelinePlaybackPanel.tsx", import.meta.url), "utf8");
  assert.doesNotMatch(panelSource, /@tauri-apps|invoke\s*\(|from "\.\.\/App"/, "playback panel owns no backend call");
  const appSource = await readFile(new URL("../src/App.tsx", import.meta.url), "utf8");
  const panelUse = appSource.slice(appSource.indexOf("<ControlTimelinePlaybackPanel"), appSource.indexOf("/>", appSource.indexOf("onOpenTimelineEditor")));
  for (const [prop, command] of [["onGo","triggerNextCue"],["onBack","triggerPreviousCue"],["onTriggerCue","triggerCue"],["onPlay","playTimeline"],["onPause","pauseTimeline"],["onSeek","seekTimeline"],["onSetBlackout","setBlackout"],["onSetVideoBlackout","requestPerformVideoBlackout"],["onSetAllBlackout","setAllBlackout"],["onToggleFadePause","setCueFadePaused"],["onRelease","releaseCueById"]]) {
    assert.match(panelUse, new RegExp(`${prop}=[^\\n]*${command}|${prop}=\\{[\\s\\S]*?${command}`), `${prop} uses ${command}`);
  }
  assert.match(appSource, /timelineSurfaceActive: \(\) => workspaceTab\(\) === "control" && controlMode\(\) === "live"/, "Space stays GO on the playback surface");
  assert.match(await readFile(new URL("../src/createAppKeyboardController.ts", import.meta.url), "utf8"), /workspaceTab\(\) === "touch" && \(mode === "edit" \|\| mode === "live"\)/, "L / Alt+4 route to the Control domain");
}
let server, browser, cdp, profile;
const results = [];
let currentStep = "startup";
try {
  await mkdir(evidenceDir, { recursive: true });
  const vitePort = await reserveBrowserCheckPort("SYNDOCAL_VIEWPORT_VITE_PORT");
  server = await createServer({ root: appRoot, cacheDir: resolve(evidenceDir, "ia-vite-cache"), server: { host: "127.0.0.1", port: vitePort, strictPort: true }, clearScreen: false });
  await server.listen();
  // Complete Vite dependency discovery before opening the first cold document.
  await server.environments.client.depsOptimizer?.scanProcessing;
  await server.transformRequest("/src/App.tsx");
  const origin = server.resolvedUrls.local[0];
  const port = await reserveBrowserCheckPort("SYNDOCAL_CDP_PORT");
  profile = await mkdtemp(join(tmpdir(), "syndocal-ia-check-"));
  browser = spawn(browserPath, ["--headless=new", "--disable-gpu", "--disable-background-timer-throttling", "--disable-renderer-backgrounding", "--no-first-run", "--no-default-browser-check", `--remote-debugging-port=${port}`, `--user-data-dir=${profile}`, "about:blank"], { windowsHide: true, stdio: "ignore" });
  const targets = await waitFor(async () => {
    const response = await fetch(`http://127.0.0.1:${port}/json/list`, { signal: AbortSignal.timeout(1000) });
    if (!response.ok) return null;
    const candidates = await response.json();
    return candidates.some((target) => target.type === "page") ? candidates : null;
  }, "owned browser debugger");
  let target = targets.find(x => x.type === "page");
  const connect = async () => { cdp = new Cdp(); await cdp.connect(target.webSocketDebuggerUrl); await cdp.send("Page.enable"); await cdp.send("Runtime.enable"); await cdp.send("Page.bringToFront"); };
  await connect();
  const freshTarget = async () => {
    const next = await (await fetch(`http://127.0.0.1:${port}/json/new?about:blank`, {method:"PUT"})).json();
    await fetch(`http://127.0.0.1:${port}/json/close/${target.id}`);
    cdp.socket.close(); target = next; await connect();
  };
  for (const {locale,width,height} of [{locale:"ja",width:1366,height:768},{locale:"en",width:1920,height:1080}]) {
    for (const workspace of ["control", "touch"]) {
      const tabs = workspace === "control" ? ["edit", "mixer", "live"] : ["lighting", "video", "both", "timeline"];
      const labels = locale === "ja" ? (workspace === "control" ? ["\u7167\u660e", "\u6620\u50cf", "\u30bf\u30a4\u30e0\u30e9\u30a4\u30f3"] : ["\u7167\u660e", "\u6620\u50cf", "\u4e21\u65b9", "\u30bf\u30a4\u30e0\u30e9\u30a4\u30f3"]) : (workspace === "control" ? ["Lighting", "Video", "Timeline"] : ["Lighting", "Video", "Both", "Timeline"]);
      for (const tab of tabs) {
        currentStep = `${workspace}/${tab}/${locale}:start`; console.log(currentStep);
        if (workspace === "control" && tab === "edit") {
        if (results.length) await freshTarget();
        await cdp.send("Emulation.setDeviceMetricsOverride", {width,height,deviceScaleFactor:1,mobile:false});
        await cdp.send("Page.addScriptToEvaluateOnNewDocument", {source:`localStorage.setItem('syndocal.uiLocale.v1',${JSON.stringify(locale)});localStorage.setItem('syndocal.uiTheme.v1','tally');localStorage.setItem('syndocal.workspaceLayout.v1',JSON.stringify({workspace_tab:'control',control_mode:'edit'}));`});
        await cdp.send("Page.navigate",{url:`${origin}?syndocalViewportFixture=vj-bank&iaCase=${workspace}-${tab}-${locale}`});
        await waitFor(() => cdp.evaluate("!!document.querySelector('[data-workspace-option=control]')"), "workspace chrome");
        }
        currentStep += ":select-workspace";
        await cdp.evaluate(`document.querySelector('[data-workspace-option=${workspace}]').click()`);
        const nav = workspace === "control" ? "[data-edit-domain-navigation]" : "[data-control-domain-navigation]";
        const attr = workspace === "control" ? "data-control-mode-option" : "data-control-domain";
        await waitFor(() => cdp.evaluate(`!!document.querySelector('${nav} [${attr}=${tab}]')`), "workspace tabs");
        const actual = await cdp.evaluate(`(()=>{const tabs=[...document.querySelectorAll('${nav} [role=tab]')];return {ids:tabs.map(x=>x.getAttribute('${attr}')),labels:tabs.map(x=>x.textContent.trim())}})()`);
        assert.deepEqual(actual.ids,tabs,"exact ordered workspace tab IDs");
        assert.deepEqual(actual.labels,labels,"exact localized workspace tab labels");
        currentStep += ":select-tab";
        await cdp.evaluate(`document.querySelector('${nav} [${attr}=${tab}]').click()`);
        currentStep += ":fonts"; console.log(currentStep);
        await cdp.evaluate("document.fonts.ready.then(()=>true)");
        await cdp.send("Page.bringToFront");
        if (workspace === "control") {
          assert.equal(await cdp.evaluate("document.querySelectorAll('[data-edit-domain-navigation] [data-control-mode-option=perform], [data-edit-domain-navigation] [data-control-mode-option=both]').length"),0);
          assert.equal(await cdp.evaluate("document.querySelectorAll('[data-video-perform-layout],.controlBothPanel,[data-control-timeline-playback]').length"),0);
          if (tab === "live") {
            // Edit > Timeline is authoring only: arranger, block/cue editing, source shelf, inspector drawers.
            await waitFor(()=>cdp.evaluate("!!document.querySelector('[data-timeline-arranger-upper]') && !!document.querySelector('[data-timeline-source-shelf]')"),"Edit Timeline mounts the arranger and source shelf");
            assert.equal(await cdp.evaluate("document.querySelectorAll('.liveControlPanel > .liveStatusGrid, .liveControlPanel .sceneMatrixPanel, .liveControlPanel .liveCuePadSurface, .liveControlPanel .liveFadeMeter, #live-status-inspector').length"),0,"Edit Timeline renders no hidden playback status, cue matrix, pads or fade meter");
            assert.equal(await cdp.evaluate("document.querySelectorAll('[data-playback-action=go],[data-playback-cue-list]').length"),0,"no playback controls in Edit Timeline");
            assert.equal(await cdp.evaluate("document.querySelectorAll('[data-global-operator-action=go]').length"),1,"the global top-bar GO stays available in Edit");
          }
        } else if (tab === "timeline") {
          assert.equal(await cdp.evaluate("document.querySelectorAll('[data-control-timeline-playback]').length"),1,"Control > Timeline owns exactly one playback surface");
          assert.equal(await cdp.evaluate("document.querySelectorAll('[data-timeline-arranger-upper],[data-timeline-source-shelf],.timelineOperatorBar,[data-video-perform-layout],.controlBothPanel').length"),0,"playback surface carries no authoring or other-domain surface");
          const probe = await cdp.evaluate(`(()=>{const q=(s)=>document.querySelector(s);const hit=(s)=>{const e=q(s);if(!e)return null;const r=e.getBoundingClientRect();return {w:Math.round(r.width),h:Math.round(r.height)}};
            return {panel:q('[data-control-timeline-playback]')?.getAttribute('role'),
              labelled:q('[data-control-timeline-playback]')?.getAttribute('aria-labelledby'),
              go:hit('[data-playback-action=go]'),back:hit('[data-playback-action=back]'),release:hit('[data-playback-action=release]'),
              toggle:hit('[data-playback-action=timeline-toggle]'),dmx:hit('[data-playback-action=dmx-blackout]'),vid:hit('[data-playback-action=video-blackout]'),all:hit('[data-playback-action=all-blackout]'),
              cue:hit('[data-playback-cue-id]'),seek:hit('[data-playback-seek]'),
              current:!!q('[data-playback-current]'),next:!!q('[data-playback-next]'),position:!!q('[data-playback-position]'),
              cueList:!!q('[data-playback-cue-list]'),legend:q('[data-playback-shortcut-legend]')?.textContent.length>0,
              globalGo:document.querySelectorAll('[data-global-operator-action=go]').length,
              panelFontOk:[...document.querySelectorAll('[data-control-timeline-playback] :is(small,span,li,dd,dt,button)')].every(e=>!e.textContent.trim()||parseFloat(getComputedStyle(e).fontSize)>=13)}})()`);
          assert.equal(probe.panel,"tabpanel");
          assert.equal(probe.labelled,"control-domain-timeline");
          for (const key of ["go","back","release","toggle","dmx","vid","all","seek"]) assert.ok(probe[key] && probe[key].h >= 44, `${key} keeps a 44px target: ${JSON.stringify(probe[key])}`);
          assert.ok(probe.go.h >= 56, "GO is the primary, largest target");
          for (const key of ["current","next","position","cueList","legend"]) assert.equal(probe[key], true, `${key} readout is present`);
          assert.equal(probe.globalGo,1,"the top-bar GO is the only global GO; the surface adds none");
          assert.equal(probe.panelFontOk,true,"body text is 13px minimum");
          if (probe.cue) assert.ok(probe.cue.h >= 44, `cue rows keep large touch targets: ${JSON.stringify(probe.cue)}`);
          // Same transport/cue contract as the top bar: GO and Back live in both and are enabled together.
          assert.equal(await cdp.evaluate("document.querySelector('[data-playback-action=go]').disabled === document.querySelector('[data-global-operator-action=go]').disabled && document.querySelector('[data-playback-action=back]').disabled === document.querySelector('[data-global-operator-action=back]').disabled"),true,"playback GO/Back enabled state matches the top-bar GO/Back");
          assert.equal(await cdp.evaluate("document.querySelectorAll('[data-playback-action=go]').length"),1);
          assert.equal(await cdp.evaluate("[...document.querySelectorAll('[data-control-domain-navigation] [role=tab]')].length"),4,"Control keeps at most four tabs");
        } else if (tab === "video") {
          assert.equal(await cdp.evaluate("document.querySelectorAll('[data-video-perform-layout]').length"),1,"Control Video owns the single Perform desk");
          await cdp.evaluate("document.querySelector('[data-control-domain=video]').focus()");
          for (const [key, domain] of [["ArrowRight","both"],["ArrowLeft","video"],["Home","lighting"],["End","timeline"],["ArrowRight","lighting"],["ArrowLeft","timeline"],["ArrowLeft","both"],["ArrowLeft","video"]]) {
            await cdp.key(key);
            await waitFor(()=>cdp.evaluate(`document.activeElement?.getAttribute('data-control-domain') === '${domain}' && document.activeElement?.getAttribute('aria-selected') === 'true'`), `Control tab ${key} selects and focuses ${domain} while Perform is mounted`);
          }
          assert.equal(await cdp.evaluate("document.querySelectorAll('[data-video-perform-layout]').length"),1,"tab roving returns to the unchanged Perform desk");
          await cdp.evaluate("document.querySelector('[data-control-domain=video]').focus()");
          await cdp.key("2","Digit2",50);
          assert.equal(await cdp.evaluate("document.activeElement?.getAttribute('data-video-perform-cell-index')"),"1","row key launches the corresponding slot from Video tab chrome");
          await cdp.evaluate("document.querySelector('[data-control-video-tools]').click()");
          await waitFor(()=>cdp.evaluate("!!document.querySelector('.controlVideoToolsDialog[open]')"),"retained live video controls open");
          await cdp.evaluate("document.querySelector('.controlVideoToolsDialog [data-video-clip-slot-index=\"0\"] .videoClipSlotPadActions button:last-child').click()");
          await waitFor(()=>cdp.evaluate("!!document.querySelector('.controlVideoToolsDialog [data-video-clip-slot-inspector]:not([data-video-clip-slot-inspector-embedded])')"),"retained live tools open one clip inspector");
          assert.equal(await cdp.evaluate("document.querySelectorAll('#video-clip-slot-inspector-title').length"),1,"live tools never duplicate floating inspector title IDs");
          await cdp.evaluate("document.querySelector('.controlVideoToolsDialog > header button').click()");
          await waitFor(()=>cdp.evaluate("!document.querySelector('.controlVideoToolsDialog[open]') && document.querySelectorAll('[data-video-clip-slot-inspector]:not([data-video-clip-slot-inspector-embedded])').length === 0"),"closing live tools reaps the floating inspector").catch(async error => {
            error.message += ` state=${JSON.stringify(await cdp.evaluate("({dialogOpen:!!document.querySelector('.controlVideoToolsDialog[open]'),dialogs:document.querySelectorAll('.controlVideoToolsDialog').length,inspectors:document.querySelectorAll('[data-video-clip-slot-inspector]:not([data-video-clip-slot-inspector-embedded])').length})"))}`;
            throw error;
          });
          await cdp.evaluate("document.querySelector('[data-control-domain=video]').focus()");
          await cdp.key("3","Digit3",51);
          assert.equal(await cdp.evaluate("document.activeElement?.getAttribute('data-video-perform-cell-index')"),"2","closing nested live tools restores row-key launch");
        } else if (tab === "both") {
          assert.equal(await cdp.evaluate("document.querySelectorAll('.controlBothPanel').length"),1,"Control Both owns exactly one combined overview");
          assert.equal(await cdp.evaluate("document.querySelectorAll('.touchBothLivePanel').length"),0,"retired duplicate Both surface is unmounted");
        }
        assert.equal(await cdp.evaluate("document.documentElement.getAttribute('data-theme')"),"tally");
        const filename = `${workspace === "control" ? "edit" : "control"}-${tab}-${width}x${height}-${locale}-tally.png`;
        const screenshot = await cdp.capture();
        await writeFile(join(evidenceDir,filename),Buffer.from(screenshot.data,"base64"));
        assert.deepEqual(cdp.errors,[],`${filename}: browser errors and warnings`);
        results.push({workspace,tab,locale,width,height,screenshot:join(evidenceDir,filename),tabs:actual});
        console.log(`PASS ${filename}`);
      }
    }
  }
  await freshTarget();
  await cdp.send("Page.addScriptToEvaluateOnNewDocument", {source:"localStorage.setItem('syndocal.uiLocale.v1','en');localStorage.setItem('syndocal.workspaceLayout.v1',JSON.stringify({workspace_tab:'control',control_mode:'live'}));"});
  await cdp.send("Page.navigate",{url:`${origin}?syndocalViewportFixture=workspace-operator&syndocalOperatorLock=partial`});
  await waitFor(()=>cdp.evaluate("document.querySelector('[data-workspace-option=setup]')?.disabled === true"),"Partial lock fixture");
  await cdp.key("F3","F3",114);
  await waitFor(()=>cdp.evaluate("!!document.querySelector('[data-video-perform-layout]') && document.querySelector('[data-workspace-option=touch]')?.getAttribute('aria-pressed') === 'true'"),"Partial F3 retains reachable Control Video");
  assert.equal(await cdp.evaluate("document.querySelector('[data-control-domain=lighting]')?.disabled"),true,"Partial still blocks Control Lighting");
  await cdp.key("e","KeyE",69);
  assert.equal(await cdp.evaluate("document.querySelector('[data-control-domain=video]')?.getAttribute('aria-selected')"),"true","Partial rejects the Lighting shortcut without ejecting Video");
  await cdp.evaluate("document.querySelector('[data-control-domain=both]').click()");
  await waitFor(()=>cdp.evaluate("document.querySelectorAll('.controlBothPanel').length === 1"),"Partial retains the relocated Both overview");
  await cdp.evaluate("document.querySelector('[data-control-domain=timeline]').click()");
  await waitFor(()=>cdp.evaluate("document.querySelectorAll('[data-control-timeline-playback]').length === 1"),"Partial operators can run the show from Control > Timeline");
  await cdp.evaluate("document.querySelector('[data-control-domain=both]').click()");
  await waitFor(()=>cdp.evaluate("document.querySelectorAll('.controlBothPanel').length === 1"),"Partial returns to Both");
  await cdp.evaluate("document.querySelector('[data-control-domain=both]').focus()");
  await cdp.key("Home","Home",36);
  assert.equal(await cdp.evaluate("document.activeElement?.getAttribute('data-control-domain')"),"video","Partial roving skips disabled Lighting");
  await cdp.key("F2","F2",113);
  await waitFor(()=>cdp.evaluate("document.querySelector('[data-workspace-option=control]')?.getAttribute('aria-pressed') === 'true'"),"Partial Edit workspace route");
  assert.equal(await cdp.evaluate("document.querySelector('[data-edit-domain-navigation] [data-control-mode-option=edit]')?.disabled"),true,"Partial still blocks Edit Lighting");
  assert.deepEqual(cdp.errors,[],"Partial routing creates no browser errors");
  await freshTarget();
  await cdp.send("Page.addScriptToEvaluateOnNewDocument",{source:"localStorage.setItem('syndocal.uiLocale.v1','en');localStorage.setItem('syndocal.workspaceLayout.v1',JSON.stringify({workspace_tab:'control',control_mode:'perform'}));"});
  // No viewport fixture here: it deliberately overrides saved navigation state.
  await cdp.send("Page.navigate",{url:origin});
  await waitFor(()=>cdp.evaluate("!!document.querySelector('[data-video-perform-layout]')"),"persisted Edit Perform migrates to Control Video on actual boot");
  assert.equal(await cdp.evaluate("document.querySelector('[data-workspace-option=touch]')?.getAttribute('aria-pressed')"),"true");
  assert.equal(await cdp.evaluate("document.querySelector('[data-control-domain=video]')?.getAttribute('aria-selected')"),"true");
  // Control reuses the mode signal for its domains; Edit must still return to the
  // creation tab that was active before the operator visited Control.
  await freshTarget();
  await cdp.send("Page.addScriptToEvaluateOnNewDocument",{source:"localStorage.setItem('syndocal.uiLocale.v1','en');localStorage.setItem('syndocal.workspaceLayout.v1',JSON.stringify({workspace_tab:'control',control_mode:'live'}));"});
  await cdp.send("Page.navigate",{url:origin});
  await waitFor(()=>cdp.evaluate("document.querySelector('[data-edit-domain-navigation] [data-control-mode-option=live]')?.getAttribute('aria-selected') === 'true'"),"boot on Edit Timeline");
  await cdp.evaluate("document.querySelector('[data-workspace-option=touch]').click()");
  await waitFor(()=>cdp.evaluate("!!document.querySelector('[data-control-domain-navigation]')"),"Control workspace");
  await cdp.evaluate("document.querySelector('[data-workspace-option=control]').click()");
  await waitFor(()=>cdp.evaluate("document.querySelector('[data-edit-domain-navigation] [data-control-mode-option=live]')?.getAttribute('aria-selected') === 'true'"),"Edit returns to the last creation tab (Timeline)");
  assert.deepEqual(cdp.errors,[],"workspace round trip creates no browser errors");
  // Task 21 persistence + keyboard: Control > Timeline restores on boot, Alt+4 / L reach it from Control,
  // and Edit > Timeline remains reachable (F2 returns to the last Edit tab, the header button opens it).
  await freshTarget();
  await cdp.send("Page.addScriptToEvaluateOnNewDocument",{source:"localStorage.setItem('syndocal.uiLocale.v1','en');localStorage.setItem('syndocal.workspaceLayout.v1',JSON.stringify({workspace_tab:'touch',control_mode:'live',touch_control_domain:'timeline'}));"});
  await cdp.send("Page.navigate",{url:origin});
  await waitFor(()=>cdp.evaluate("!!document.querySelector('[data-control-timeline-playback]')"),"persisted Control > Timeline boots the playback surface");
  assert.equal(await cdp.evaluate("document.querySelector('[data-control-domain=timeline]')?.getAttribute('aria-selected')"),"true");
  assert.equal(await cdp.evaluate("document.querySelector('[data-workspace-option=touch]')?.getAttribute('aria-pressed')"),"true");
  assert.equal(await cdp.evaluate("document.querySelectorAll('[data-timeline-arranger-upper]').length"),0,"booting Control > Timeline mounts no authoring arranger");
  await cdp.key("1","Digit1",49); // not a Control shortcut without Alt; must not leave the surface
  assert.equal(await cdp.evaluate("document.querySelector('[data-control-domain=timeline]')?.getAttribute('aria-selected')"),"true");
  const altKey = async (key,code,virtualKey) => {
    const params = {key,code,windowsVirtualKeyCode:virtualKey,nativeVirtualKeyCode:virtualKey,modifiers:1};
    await cdp.send("Input.dispatchKeyEvent",{type:"keyDown",...params});
    await cdp.send("Input.dispatchKeyEvent",{type:"keyUp",...params});
  };
  await altKey("1","Digit1",49);
  await waitFor(()=>cdp.evaluate("document.querySelector('[data-control-domain=lighting]')?.getAttribute('aria-selected')==='true'"),"Alt+1 selects Control Lighting");
  await altKey("4","Digit4",52);
  await waitFor(()=>cdp.evaluate("document.querySelector('[data-control-domain=timeline]')?.getAttribute('aria-selected')==='true' && !!document.querySelector('[data-control-timeline-playback]')"),"Alt+4 selects Control > Timeline");
  await altKey("3","Digit3",51);
  await waitFor(()=>cdp.evaluate("document.querySelector('[data-control-domain=both]')?.getAttribute('aria-selected')==='true'"),"Alt+3 selects Control Both");
  await cdp.evaluate("document.activeElement?.blur?.()");
  await cdp.key("l","KeyL",76);
  await waitFor(()=>cdp.evaluate("document.querySelector('[data-control-domain=timeline]')?.getAttribute('aria-selected')==='true' && document.querySelector('[data-workspace-option=touch]')?.getAttribute('aria-pressed')==='true'"),"L from Control selects Control > Timeline, not the authoring tab");
  assert.equal(await cdp.evaluate("document.querySelectorAll('[data-timeline-arranger-upper]').length"),0,"L never opens the arranger from Control");
  // Space is GO here, so it must not toggle the Timeline transport (the transport shortcut is T / the button).
  const transportBefore = await cdp.evaluate("document.querySelector('[data-playback-action=timeline-toggle]')?.getAttribute('aria-pressed')");
  await cdp.evaluate("document.activeElement?.blur?.()");
  await cdp.key(" ","Space",32);
  await sleep(250);
  assert.equal(await cdp.evaluate("document.querySelector('[data-playback-action=timeline-toggle]')?.getAttribute('aria-pressed')"),transportBefore,"Space does not toggle the Timeline transport on the playback surface");
  // The explicit route to authoring.
  await cdp.evaluate("document.querySelector('[data-playback-open-editor]').click()");
  await waitFor(()=>cdp.evaluate("document.querySelector('[data-edit-domain-navigation] [data-control-mode-option=live]')?.getAttribute('aria-selected')==='true' && !!document.querySelector('[data-timeline-arranger-upper]')"),"Edit Timeline opens the authoring surface");
  assert.equal(await cdp.evaluate("document.querySelectorAll('[data-control-timeline-playback]').length"),0);
  await cdp.evaluate("document.querySelector('[data-workspace-option=touch]').click()");
  await waitFor(()=>cdp.evaluate("document.querySelector('[data-control-domain=timeline]')?.getAttribute('aria-selected')==='true' && !!document.querySelector('[data-control-timeline-playback]')"),"Control returns to its remembered Timeline playback domain");
  await cdp.key("F2","F2",113);
  await waitFor(()=>cdp.evaluate("document.querySelector('[data-edit-domain-navigation] [data-control-mode-option=live]')?.getAttribute('aria-selected')==='true'"),"F2 returns to Edit > Timeline");
  assert.deepEqual(cdp.errors,[],"Control > Timeline keyboard and persistence create no browser errors");
  // Standby: the always-visible blackout keys stay in every workspace and keep their place
  // (aria-disabled, never removed) while explaining that this machine is not driving outputs.
  await cdp.evaluate("window.__syndocalOutputDriveFixture='standby'");
  await waitFor(()=>cdp.evaluate("document.querySelector('[data-global-operator-action=video-blackout]')?.getAttribute('aria-disabled') === 'true'"),"top-bar blackout keys show Standby");
  assert.deepEqual(await cdp.evaluate("['dmx-blackout','video-blackout','all-blackout'].map(a=>{const b=document.querySelector('[data-global-operator-action='+a+']');return [!!b,b?.disabled,b?.getAttribute('aria-disabled'),b?.title]})"),
    ['dmx-blackout','video-blackout','all-blackout'].map(()=>[true,false,'true','Standby: this machine is not driving outputs']),"DMX/VID/ALL stay present and focusable with the Standby reason");
  await cdp.evaluate("window.__syndocalOutputDriveFixture='driving'");
  await waitFor(()=>cdp.evaluate("!document.querySelector('[data-global-operator-action=video-blackout]')?.hasAttribute('aria-disabled')"),"top-bar blackout keys recover when outputs are driven");
  await cdp.evaluate("window.__syndocalOutputDriveFixture=undefined");
  await writeFile(join(evidenceDir,"workspace-ia-results.json"),JSON.stringify({softwareOnly:true,results,migration:"module: both/perform idempotent and ratios retained; browser: legacy Edit Perform boots Control Video",errors:cdp.errors},null,2));
  console.log(`Workspace IA: ${results.length} tab/locale screenshots, Control > Timeline playback split and persisted migration passed`);
} catch(error) {
  const diagnostic = await cdp?.evaluate("({url:location.href,text:document.body.innerText,html:document.body.innerHTML.slice(0,2000)})").catch(()=>null);
  await writeFile(join(evidenceDir,"workspace-ia-failure.json"),JSON.stringify({currentStep,error:error.stack,diagnostic,errors:cdp?.errors,results},null,2));
  throw error;
} finally {
  if(cdp?.socket?.readyState===WebSocket.OPEN){await cdp.send("Browser.close").catch(()=>{});cdp.socket.close();}
  let browserExitError;
  if (browser && browser.exitCode === null && browser.signalCode === null) {
    const exited = () => browser.exitCode !== null || browser.signalCode !== null;
    try {
      await waitFor(exited, "owned browser exit", 5_000);
    } catch {
      try {
        browser.kill();
        await waitFor(exited, "owned browser exit after forced termination", 5_000);
      } catch (error) {
        browserExitError = error;
      }
    }
  }
  await server?.close();
  if (browserExitError) throw browserExitError;
  if(profile){assert.equal(resolve(dirname(profile)),resolve(tmpdir()));assert.ok(basename(profile).startsWith("syndocal-ia-check-"));await rm(profile,{recursive:true,force:true,maxRetries:15,retryDelay:250});}
}
