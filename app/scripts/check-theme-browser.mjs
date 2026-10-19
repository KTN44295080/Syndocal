import assert from "node:assert/strict";
import { existsSync } from "node:fs";
import { mkdir, readFile, unlink, writeFile } from "node:fs/promises";
import { createRequire } from "node:module";
import { homedir } from "node:os";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { createServer } from "vite";
import { typeRoleAudit } from "./type-roles-audit-lib.mjs";

// Browser plugin/browser skill is not available in this session. The task
// explicitly requires the existing app's browser fixture harness. This uses
// those actual app routes in an isolated Playwright context, never native I/O.
const require = createRequire(import.meta.url);
let chromium;
try { ({ chromium } = require("playwright")); } catch {
  ({ chromium } = require(process.env.PLAYWRIGHT_MODULE_PATH ?? resolve(homedir(),
    ".cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright")));
}
const appRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const artifacts = resolve(process.env.SYNDOCAL_THEME_ARTIFACT_DIR ?? resolve(appRoot, "../target/qa/theme-13a"));
const stateFixtureName = `.theme-state-proof-${process.pid}.tsx`;
const stateFixturePath = resolve(appRoot, stateFixtureName);
const themes = ["tally", "graphite", "ice", "cobalt", "forest", "ember", "neon", "safe"];
const configurations = [
  { locale: "en", width: 1920, height: 1080 },
  { locale: "ja", width: 1366, height: 768 },
];
const workspaces = [
  { id: "lighting-edit", fixture: "timeline", workspace: "control", mode: "edit" },
  { id: "setup-patch", fixture: "patch", workspace: "setup" },
  { id: "video-edit", fixture: "operator-vj", workspace: "control", mode: "mixer" },
  { id: "control-perform", fixture: "vj-bank", workspace: "control", mode: "perform" },
  { id: "timeline", fixture: "timeline-layered", workspace: "control", mode: "live" },
];
const report = { typeRoles: [], timestamp: new Date().toISOString(), browserPlugin: "unavailable", fallbackReason: "Browser plugin not available; task requires existing fixture harness", softwareOnly: true, screenshots: [], themes: [], textContrast: [], interactions: [], errors: [], warnings: [], baselineAssetNotices: [], failedResponses: [] };
let server, browser;
await mkdir(artifacts, { recursive: true });
try {
  // This isolated synthetic-props fixture mounts the real Perform component.
  // No App/backend, media file, native output or hardware transport is used.
  await writeFile(stateFixturePath, `
import '/src/theme.css'; import '/src/styles.css'; import '/src/controls.css'; import '/src/components/VideoPerformLayout.css';
import { createSignal } from 'solid-js'; import {render} from 'solid-js/web';
import {VideoPerformDesk} from '/src/components/VideoPerformDesk';
import {viewportFixtureData} from '/src/viewportFixtureData';
import {emptyLiveVideoMonitorBusState} from '/src/createLiveVideoMonitorController';
const noop=()=>{};
const [blackout,setBlackout]=createSignal(false);
const assets=Array.from({length:6},(_,i)=>({id:i+1,label:['Program source','Queued source','Unreadable media','Preview source','Loaded source','Missing source'][i],source:{kind:'File',name:'Synthetic source',path:'fixture://theme/'+i}}));
const slot=(id,asset)=>({id,media_asset_id:asset,in_point_ms:0,out_point_ms:8000,loop_mode:'Loop',speed:1,cue_points:[],launch_quantization:'Immediate',effect_overrides:[]});
const layer=(id,label,slots)=>({...structuredClone(viewportFixtureData.videoLayer),id,label,clip_slots:slots,isf_effect:null});
const layers=[layer(1,'01 Program Layer',[slot(101,1),slot(102,2),slot(103,3)]),layer(2,'02 Preview Layer',[slot(201,4),slot(202,5),slot(203,6)])];
const runtimes=[{layer_id:1,active_slot_id:101,queued_slot_id:102,playing:true,playhead_ms:1000},{layer_id:2,active_slot_id:201,queued_slot_id:null,playing:false,playhead_ms:1000}];
const [selected,setSelected]=createSignal(102);
const bank={layers,layer:layers[0],runtime:runtimes[0],assets,thumbnails:{},get selectedSlotId(){return selected()},transitionKind:'Cut',transitionDurationUnit:'Milliseconds',transitionDurationMilliunits:750,onSelect:setSelected,onSelectLayer:noop,onQueue:noop,onPreview:noop,onMore:noop,onImport:noop,onSetTransitionKind:noop,onSetTransitionDurationUnit:noop,onSetTransitionDurationMilliunits:noop};
const liveMonitors={preview:{...emptyLiveVideoMonitorBusState('error'),error:'Synthetic monitor error'},program:{...emptyLiveVideoMonitorBusState('live'),fps:60,renderMs:1,encodeMs:1},previewLabel:'Preview source',programLabel:'Program source',previewTransport:{layer_id:2,playing:false,position_ms:1000,duration_ms:8000,speed:1,loop_enabled:false},previewTransportBusy:false,previewTransportError:null,previewTransportBackendAvailable:true,onSetPreviewPlaying:noop,onSeekPreview:noop,onSetPreviewSpeed:noop,onClearPreview:noop,onRetry:noop};
const props={performContext:{runtimes,keyboardEnabled:false,onOpenEditor:noop},mediaLibrary:{assets,availabilityById:{3:{kind:'unreadable',error:'Synthetic unreadable media'},6:{kind:'missing'}},lastImportReport:null},clipGrid:{previewLayerId:2,thumbnailsAuthorized:true,previewBackendAvailable:true,previewBusy:false,onStagePreview:noop,onStop:noop,fadeMs:0,onRequestThumbnails:noop},clipSlotBank:bank,clipSlotTake:{enabled:true,label:'TAKE',onTake:noop},liveMonitors,layerList:{isfRuntimeErrors:[],onSetLayerState:noop,onSetBlendMode:noop},masterControls:{masterOpacity:1,get blackout(){return blackout()},onSetMasterOpacity:noop,onSetBlackout:setBlackout}};
window.themeProof={setBlackout};
document.documentElement.dataset.theme=localStorage.getItem('syndocal.uiTheme.v1')||'tally';
render(()=><main class="app" style="height:100vh;grid-template-rows:56px minmax(0,1fr);gap:0;padding:0"><header style="height:56px;padding:12px;box-sizing:border-box">Actual Perform components with synthetic props — Program / Preview / Queued / Error. Software-only state presentation.</header><div class="layout layoutSharedWorkspace layoutControl controlModePerform"><VideoPerformDesk {...props}/></div></main>,document.body);
`);
  server = await createServer({ root: appRoot, cacheDir: resolve(artifacts, "vite-cache"), logLevel: "error", server: { host: "127.0.0.1", port: Number(process.env.SYNDOCAL_THEME_PORT ?? 5263), strictPort: true },
    plugins: [{ name: "theme-state-proof", configureServer(server) { server.middlewares.use((request, response, next) => {
      if (!request.url?.startsWith("/theme-state-proof")) return next();
      response.setHeader("Content-Type", "text/html");
      response.end('<html><head><title>Syndocal theme state proof</title><link rel="icon" href="data:,"></head><body><script type="module" src="/' + stateFixtureName + '"></script></body></html>');
    }); } }] });
  await server.listen();
  const origin = `http://127.0.0.1:${server.httpServer.address().port}`;
  report.origin = origin;
  const executablePath = [process.env.CHROME_PATH, "C:/Program Files/Google/Chrome/Application/chrome.exe", "C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe"].filter(Boolean).find(existsSync);
  browser = await chromium.launch({ headless: true, ...(executablePath ? { executablePath } : {}) });
  report.browserVersion = browser.version();
  const context = await browser.newContext({ viewport: configurations[0] });
  const page = await context.newPage();
  const observe = (target) => {
    target.on("pageerror", error => report.errors.push(String(error)));
    target.on("console", message => {
      if (message.type() === "warning") report.warnings.push(message.text());
      if (message.type() === "error") {
        const entry = { message: message.text(), location: message.location() };
        // index.html has no favicon declaration at the unchanged baseline.
        // Keep the exact missing-resource evidence, distinct from app errors.
        if (entry.location.url?.endsWith("/favicon.ico")) report.baselineAssetNotices.push(entry);
        else report.errors.push(entry);
      }
    });
    target.on("response", response => { if (response.status() >= 400) report.failedResponses.push({ url: response.url(), status: response.status() }); });
    target.on("dialog", dialog => dialog.type() === "beforeunload" ? dialog.accept() : dialog.dismiss());
  };
  observe(page);
  const waitApp = async (target = page) => {
    await target.locator(".app").waitFor();
    assert.equal(await target.locator("vite-error-overlay").count(), 0, "no framework error overlay");
    assert.ok((await target.locator("body").innerText()).length > 100, "meaningful app content");
  };
  const openMenu = async () => {
    if (!(await page.locator(".appProjectMenu").count())) await page.locator(".appMenuButton").click();
  };
  const selectTheme = async (theme) => {
    await openMenu();
    await page.locator(`[data-theme-option="${theme}"]`).click();
    await page.waitForFunction(theme => document.documentElement.dataset.theme === theme, theme);
    assert.equal(await page.evaluate(() => localStorage.getItem("syndocal.uiTheme.v1")), theme);
    assert.equal(await page.locator(`[data-theme-option="${theme}"]`).getAttribute("aria-pressed"), "true");
  };
  await page.goto(`${origin}/?syndocalViewportFixture=timeline`);
  await waitApp();
  report.title = await page.title();
  assert.match(report.title, /Syndocal/i);
  assert.equal(await page.getAttribute("html", "data-theme"), "tally", "default theme is tally");
  const pane = await context.newPage();
  observe(pane);
  await pane.goto(`${origin}/?syndocalViewportFixture=timeline&syndocalPaneWindow=stage`);
  await waitApp(pane);
  for (const theme of themes) {
    await selectTheme(theme);
    await pane.waitForFunction(theme => document.documentElement.dataset.theme === theme, theme);
    const tokens = await page.evaluate(() => {
      const style = getComputedStyle(document.documentElement);
      const names = ["disabled-text", "disabled-bg", "bg", "panel", "raised", "card", "inset", "text", "muted", "dim", "accent", "program", "preview", "queued", "error", "warn", "ok", "danger", "selected", "latched", "on-latched", "on-program", "on-preview", "on-queued", "on-error", "on-warn", "on-ok", "on-accent", "on-selected", "key", "key-hover", "key-down"];
      const canvas = document.createElement("canvas"), ctx = canvas.getContext("2d", { willReadFrequently: true });
      return Object.fromEntries(names.map(name => {
        const raw = style.getPropertyValue(`--theme-${name}`).trim();
        if (!raw) return [name, null];
        const probe = document.createElement("i"); probe.style.color = `var(--theme-${name})`; document.body.append(probe);
        const resolved = getComputedStyle(probe).color; probe.remove();
        ctx.fillStyle = resolved; ctx.fillRect(0, 0, 1, 1);
        return [name, { raw, resolved, rgb: [...ctx.getImageData(0, 0, 1, 1).data].slice(0, 3) }];
      }));
    });
    const luminance = rgb => rgb.map(v => v / 255).map(v => v <= .04045 ? v / 12.92 : ((v + .055) / 1.055) ** 2.4).reduce((sum, v, i) => sum + v * [.2126, .7152, .0722][i], 0);
    const pairs = [];
    for (const foreground of ["text", "muted", "dim"]) for (const background of ["bg", "panel", "raised", "card", "inset", "key", "key-hover", "key-down"]) pairs.push([foreground, background]);
    for (const background of ["program", "preview", "queued", "error", "warn", "ok", "accent", "selected", "latched"]) pairs.push([`on-${background}`, background]);
    pairs.push(["disabled-text", "disabled-bg"]);
    const contrasts = pairs.map(([foreground, background]) => {
      assert.ok(tokens[foreground] && tokens[background], `defined semantic contrast tokens ${foreground}/${background}`);
      const a = luminance(tokens[foreground].rgb), b = luminance(tokens[background].rgb);
      const ratio = (Math.max(a, b) + .05) / (Math.min(a, b) + .05);
      // The mockup guarantees all text levels on surfaces s2/s3/s4. Keys
      // use primary ink. Dim text on held keys is exploratory: the actual
      // rendered-text audit below detects any UI that uses that pairing.
      const required = !(foreground === "dim" && background === "key-down");
      // Disabled text stays dimmer than normal text but must remain legible as "disabled" (3:1), not look broken.
      const minimum = foreground === "disabled-text" ? 3 : 4.5;
      return { foreground, background, ratio, passes: ratio >= minimum, required };
    });
    report.themes.push({ theme, tokens, contrasts });
    report.interactions.push({ theme, immediate: true, persisted: true, openPaneSynced: true });
  }
  await page.reload(); await waitApp();
  assert.equal(await page.getAttribute("html", "data-theme"), "safe", "theme persists on reload");
  await openMenu();
  assert.equal(await page.locator(".uiThemeOption").count(), 8, "eight theme options");
  assert.equal(await page.locator(".uiThemeSwatch").count(), 8, "all themes show a swatch");
  await page.locator('.uiLocaleMenu button').filter({ hasText: "Japanese" }).click();
  await pane.waitForFunction(() => document.documentElement.lang === "ja");
  assert.equal(await page.getAttribute("html", "lang"), "ja");
  const japaneseNames = await page.locator(".uiThemeOption").allTextContents();
  assert.ok(japaneseNames.every(label => /[\u3040-\u30ff\u4e00-\u9fff]/.test(label)), "Japanese theme names localized");
  report.japaneseNames = japaneseNames;
  await page.locator('.uiScaleMenu:not(.uiLocaleMenu) button').filter({ hasText: "110%" }).click();
  await pane.waitForFunction(() => getComputedStyle(document.documentElement).getPropertyValue("--ui-scale").trim() === "1.1");
  report.interactions.push({ uiLocaleLiveSync: true, uiScaleLiveSync: true, persistedReload: true });
  await pane.close();
  for (const configuration of configurations) {
    await page.setViewportSize({ width: configuration.width, height: configuration.height });
    for (const theme of themes) {
      await page.evaluate(({ theme, locale }) => { localStorage.setItem("syndocal.uiTheme.v1", theme); localStorage.setItem("syndocal.uiLocale.v1", locale); localStorage.setItem("syndocal.uiScale.v1", "100"); }, { theme, locale: configuration.locale });
      for (const workspace of workspaces) {
        await page.goto(`${origin}/?syndocalViewportFixture=${workspace.fixture}`);
        await waitApp();
        await page.locator(`[data-workspace-option="${workspace.workspace}"]`).click();
        if (workspace.mode) await page.locator(`[data-control-mode-option="${workspace.mode}"]`).click();
        assert.equal(await page.getAttribute("html", "data-theme"), theme);
        assert.equal(await page.getAttribute("html", "lang"), configuration.locale);
        await page.evaluate(async () => { await document.fonts.ready; await new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))); });
        const filename = `${workspace.id}-${theme}-${configuration.width}x${configuration.height}-${configuration.locale}.png`;
        await page.screenshot({ path: resolve(artifacts, filename) });
        report.screenshots.push({ workspace: workspace.id, theme, ...configuration, path: filename });
        const stageTokens = await page.evaluate(() => {
          const floor = document.querySelector(".mappingPersistentStage .stageFloor, .visualizerStage .stageFloor, .controlStage .stageFloor");
          if (!floor) return null;
          const minor = document.querySelector(".mappingPersistentStage .stageGridMinor, .visualizerStage .stageGridMinor, .controlStage .stageGridMinor");
          const major = document.querySelector(".mappingPersistentStage .stageGridMajor, .visualizerStage .stageGridMajor, .controlStage .stageGridMajor");
          const resolveColor = value => {
            const probe = document.createElement("i");
            probe.style.color = value;
            document.body.append(probe);
            const color = getComputedStyle(probe).color;
            probe.remove();
            return color;
          };
          return {
            floorFill: getComputedStyle(floor).fill,
            minorStroke: minor ? getComputedStyle(minor).stroke : null,
            majorStroke: major ? getComputedStyle(major).stroke : null,
            themeBg: resolveColor("var(--theme-bg)"),
            themeLine: resolveColor("var(--theme-line)"),
            themeLineStrong: resolveColor("var(--theme-line-strong)"),
          };
        });
        if (stageTokens) {
          assert.equal(stageTokens.floorFill, stageTokens.themeBg, `${workspace.id}/${theme}: stageFloor fill follows --theme-bg`);
          if (stageTokens.minorStroke) assert.equal(stageTokens.minorStroke, stageTokens.themeLine, `${workspace.id}/${theme}: stageGridMinor stroke follows --theme-line`);
          if (stageTokens.majorStroke) assert.equal(stageTokens.majorStroke, stageTokens.themeLineStrong, `${workspace.id}/${theme}: stageGridMajor stroke follows --theme-line-strong`);
        }
        if (workspace.id === "setup-patch" && configuration.locale === "en") {
          const setupCase = await page.evaluate(() => {
            const fixtureName = document.querySelector(".mappingFixtureRowButton strong");
            const groupTab = document.querySelector('.mappingGroupStrip > button[data-group-tab="front"] [data-no-localize], .mappingGroupStrip > button[data-group-tab="front"]');
            const treeCategory = document.querySelector(".patchProfileTreeBranchRow strong");
            const setupChromeTab = document.querySelector('[data-workspace-option="setup"]');
            return {
              fixtureName: fixtureName ? { text: fixtureName.innerText.trim(), source: fixtureName.textContent.trim(), textTransform: getComputedStyle(fixtureName).textTransform } : null,
              groupTab: groupTab ? { text: groupTab.innerText.trim(), textTransform: getComputedStyle(groupTab).textTransform } : null,
              treeCategory: treeCategory ? { text: treeCategory.innerText.trim(), textTransform: getComputedStyle(treeCategory).textTransform } : null,
              setupChromeTab: setupChromeTab ? { text: setupChromeTab.innerText.trim(), textTransform: getComputedStyle(setupChromeTab).textTransform } : null,
            };
          });
          assert.equal(setupCase.fixtureName?.text, "Video", `${theme} setup fixture label keeps mixed case`);
          assert.equal(setupCase.fixtureName?.text, setupCase.fixtureName?.source, `${theme} setup fixture label rendered text equals source`);
          assert.equal(setupCase.fixtureName?.textTransform, "none", `${theme} setup fixture label text-transform is none`);
          assert.ok(setupCase.groupTab?.text.startsWith("front"), `${theme} group tab keeps lowercase 'front'`);
          assert.equal(setupCase.groupTab?.textTransform, "none", `${theme} group tab text-transform is none`);
          assert.equal(setupCase.treeCategory?.textTransform, "none", `${theme} catalog tree row text-transform is none`);
          assert.notEqual(setupCase.treeCategory?.text, setupCase.treeCategory?.text.toUpperCase(), `${theme} catalog tree row keeps mixed case`);
          assert.equal(setupCase.setupChromeTab?.textTransform, "uppercase", `${theme} Setup workspace chrome tab is uppercase`);
          assert.equal(setupCase.setupChromeTab?.text, "SETUP", `${theme} Setup workspace chrome tab renders uppercase`);
        }
        if (workspace.id === "control-perform" && configuration.locale === "en") {
          const performCase = await page.evaluate(() => {
            const layerName = document.querySelector(".videoPerformMatrix .videoPerformLayerName");
            const clipTitle = document.querySelector(".videoPerformMatrix .videoPerformClipTitle");
            const monitorTitle = document.querySelector('[data-live-video-monitor="preview"] header span');
            const monitorChrome = document.querySelector('[data-live-video-monitor="preview"] header strong');
            const matrixHeaderChrome = document.querySelector(".videoPerformMatrixCorner > strong");
            return {
              layerName: layerName ? { text: layerName.innerText.trim(), source: layerName.textContent.trim(), textTransform: getComputedStyle(layerName).textTransform } : null,
              clipTitle: clipTitle ? { text: clipTitle.innerText.trim(), source: clipTitle.textContent.trim(), textTransform: getComputedStyle(clipTitle).textTransform } : null,
              monitorTitle: monitorTitle ? { text: monitorTitle.innerText.trim(), source: monitorTitle.textContent.trim(), textTransform: getComputedStyle(monitorTitle).textTransform } : null,
              monitorChrome: monitorChrome ? { text: monitorChrome.innerText.trim(), textTransform: getComputedStyle(monitorChrome).textTransform } : null,
              matrixHeaderChrome: matrixHeaderChrome ? { text: matrixHeaderChrome.innerText.trim(), textTransform: getComputedStyle(matrixHeaderChrome).textTransform } : null,
            };
          });
          assert.equal(performCase.layerName?.text, "Opener Loop", `${theme} Perform layer name keeps mixed case`);
          assert.equal(performCase.layerName?.text, performCase.layerName?.source, `${theme} Perform layer name rendered text equals source`);
          assert.equal(performCase.layerName?.textTransform, "none", `${theme} Perform layer name text-transform is none`);
          assert.equal(performCase.clipTitle?.text, "Viewport Clip 1", `${theme} Perform clip title keeps mixed case`);
          assert.equal(performCase.clipTitle?.text, performCase.clipTitle?.source, `${theme} Perform clip title rendered text equals source`);
          assert.equal(performCase.clipTitle?.textTransform, "none", `${theme} Perform clip title text-transform is none`);
          assert.equal(performCase.monitorTitle?.text, performCase.monitorTitle?.source, `${theme} Perform monitor title span keeps mixed case`);
          assert.equal(performCase.monitorTitle?.textTransform, "none", `${theme} Perform monitor title span text-transform is none`);
          assert.equal(performCase.monitorChrome?.textTransform, "uppercase", `${theme} Perform monitor badge strong is uppercase`);
          assert.equal(performCase.monitorChrome?.text, "PREVIEW", `${theme} Perform monitor badge strong renders uppercase`);
          // Task 14: "Layers / Clips" is a column-header label (UI role, normal case); only tabs, pane titles, action keys and status badges are chrome.
          assert.equal(performCase.matrixHeaderChrome?.textTransform, "none", `${theme} Perform matrix header label is normal case (UI role)`);
        }
        if (["lighting-edit", "setup-patch", "control-perform"].includes(workspace.id) && configuration.locale === "en") {
          // Task 13b: every workspace pane header shares the Broadcast chrome: 44px, a
          // condensed-uppercase title and the raised header surface. Task 14: the title is calm:
          // transparent fill, primary text colour and a 3px accent bar, never an inverse block.
          const chrome = await page.evaluate(() => {
            const headers = [...document.querySelectorAll(".workspacePaneHeader, .controlContextHeader:not(.timelineArrangerHeader), .mappingPersistentWorkspaceGrid > .mappingGroupStrip, .layout > * > .panelHeader, .layout > * > * > .panelHeader, .layout > * > * > * > .panelHeader, .layout .patchProfileBrowserHeader")]
              .filter(element => element.getBoundingClientRect().width > 0);
            return headers.map(header => {
              const title = header.querySelector("h2, h3, :scope > span");
              const probe = document.createElement("i");
              probe.style.color = "var(--theme-accent)"; document.body.append(probe);
              const accent = getComputedStyle(probe).color; probe.remove();
              return {
                cls: header.className.toString().split(" ")[0],
                height: Math.round(header.getBoundingClientRect().height),
                titleHeight: title ? Math.round(title.getBoundingClientRect().height) : 0,
                title: title?.textContent?.trim() ?? "",
                titleTransform: title ? getComputedStyle(title).textTransform : null,
                titleBackground: title ? getComputedStyle(title).backgroundColor : null,
                accent,
              };
            });
          });
          assert.ok(chrome.length >= 2, `${theme} ${workspace.id} shows the shared pane headers ${JSON.stringify(chrome)}`);
          for (const header of chrome) {
            assert.ok(header.height >= 44 || (workspace.id === "control-perform" ? false : header.titleHeight >= 36), `${theme} ${workspace.id} ${header.cls} pane header is 44px ${JSON.stringify(header)}`);
            assert.equal(header.titleTransform, "uppercase", `${theme} ${workspace.id} ${header.cls} title is condensed uppercase`);
            assert.match(header.titleBackground, /^(rgba\(0, 0, 0, 0\)|transparent)$/, `${theme} ${workspace.id} ${header.cls} title is not an inverse block (${header.titleBackground})`);
            assert.notEqual(header.titleBackground, header.accent, `${theme} ${workspace.id} ${header.cls} title does not fill with the accent token`);
          }
          if (workspace.id === "control-perform") assert.equal(chrome.filter(header => header.cls === "workspacePaneHeader").length, 3, `${theme} Perform has Matrix, Output and Inspector pane headers`);
        }
        const textContrast = await page.evaluate(() => {
          const ctx = document.createElement("canvas").getContext("2d", { willReadFrequently: true });
          const rgba = value => { ctx.clearRect(0, 0, 1, 1); ctx.fillStyle = value; ctx.fillRect(0, 0, 1, 1); return [...ctx.getImageData(0, 0, 1, 1).data].map((v, index) => index === 3 ? v / 255 : v); };
          const luminance = rgb => rgb.slice(0, 3).map(v => v / 255).map(v => v <= .04045 ? v / 12.92 : ((v + .055) / 1.055) ** 2.4).reduce((sum, v, i) => sum + v * [.2126, .7152, .0722][i], 0);
          const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
          let node, measured = 0, gradientsSkipped = 0;
          const failures = [];
          while ((node = walker.nextNode())) {
            const text = node.textContent.trim(), el = node.parentElement;
            if (!text || !/[A-Za-z0-9\u3040-\u30ff\u4e00-\u9fff]/.test(text) || el.closest(":disabled,[aria-disabled=true],.disabled,script,style")) continue;
            const range = document.createRange(); range.selectNodeContents(node); const rect = range.getBoundingClientRect();
            if (!rect.width || !rect.height || rect.bottom < 0 || rect.top >= innerHeight || rect.right < 0 || rect.left >= innerWidth) continue;
            const style = getComputedStyle(el);
            if (style.visibility !== "visible" || Number(style.opacity) < 1) continue;
            const backgrounds = []; let current = el, gradient = false, faded = false;
            while (current) { const cs = getComputedStyle(current); gradient ||= cs.backgroundImage !== "none"; faded ||= Number(cs.opacity) < 1; backgrounds.push(rgba(cs.backgroundColor)); current = current.parentElement; }
            if (gradient || faded) { gradientsSkipped++; continue; }
            let bg = [255, 255, 255];
            for (const color of backgrounds.reverse()) bg = bg.map((v, i) => v * (1 - color[3]) + color[i] * color[3]);
            const ink = rgba(style.color), fg = bg.map((v, i) => v * (1 - ink[3]) + ink[i] * ink[3]), a = luminance(fg), b = luminance(bg);
            const ratio = (Math.max(a, b) + .05) / (Math.min(a, b) + .05);
            const minimum = parseFloat(style.fontSize) >= 24 || (parseFloat(style.fontSize) >= 18.66 && Number(style.fontWeight) >= 700) ? 3 : 4.5;
            measured++;
            if (ratio + .001 < minimum) {
              const ancestry = []; let ancestor = el;
              for (let depth = 0; ancestor && depth < 5; depth++, ancestor = ancestor.parentElement) ancestry.push(`${ancestor.tagName.toLowerCase()}.${ancestor.className}`);
              failures.push({ text: text.slice(0, 100), element: `${el.tagName.toLowerCase()}.${el.className}`, ancestry, foreground: style.color, background: bg, ratio, minimum });
            }
          }
          return { measured, gradientsSkipped, failures };
        });
        report.textContrast.push({ workspace: workspace.id, theme, ...configuration, ...textContrast });
        // Task 14 type roles: no visible text under 13px, interactive labels and list items at 14px,
        // the condensed chrome face only where the stylesheet declares --type-role: chrome, and calm pane titles.
        const typeRoles = await page.evaluate(typeRoleAudit);
        report.typeRoles.push({ workspace: workspace.id, theme, ...configuration, measured: typeRoles.measured, minPx: typeRoles.minPx,
          under: typeRoles.under, controlsUnder: typeRoles.controlsUnder, condensedOutside: typeRoles.condensedOutside,
          uncalmTitles: typeRoles.titles.filter(title => !(title.transparentFill && title.primaryText && title.bar && !title.accentFill)) });
        console.log(`captured ${filename}`);
      }
    }
    await openMenu();
    await page.locator(".uiThemeMenu").scrollIntoViewIfNeeded();
    const menuGeometry = await page.locator(".appProjectMenu").evaluate(menu => {
      const bounds = menu.getBoundingClientRect();
      return { x: bounds.x, y: bounds.y, right: bounds.right, bottom: bounds.bottom, viewportWidth: innerWidth, viewportHeight: innerHeight, scrollHeight: menu.scrollHeight, clientHeight: menu.clientHeight };
    });
    assert.ok(menuGeometry.x >= 0 && menuGeometry.y >= 0 && menuGeometry.right <= configuration.width && menuGeometry.bottom <= configuration.height, `${configuration.width}×${configuration.height} settings menu containment`);
    for (const theme of themes) {
      await selectTheme(theme);
      const option = await page.locator(`[data-theme-option="${theme}"]`).boundingBox();
      assert.ok(option.y >= menuGeometry.y && option.y + option.height <= menuGeometry.bottom + 1, `${configuration.width}: ${theme} menu option reachable by real click`);
    }
    await page.screenshot({ path: resolve(artifacts, `theme-settings-${configuration.width}x${configuration.height}-${configuration.locale}.png`) });
    report.interactions.push({ settingsConfiguration: configuration, menuGeometry, allThemeOptionsReachable: true });
  }
  const output = await context.newPage(); observe(output);
  for (const theme of themes) {
    await page.evaluate(theme => localStorage.setItem("syndocal.uiTheme.v1", theme), theme);
    await output.goto(`${origin}/?videoOutputId=1`);
    await output.locator("body.outputBody").waitFor({ state: "attached" });
    assert.equal(await output.evaluate(() => getComputedStyle(document.body).backgroundColor), "rgb(0, 0, 0)", `output body stays black for ${theme}`);
  }
  report.outputBodyBlackAllThemes = true;
  await output.close();
  report.stateSamplers = [];
  const sampler = await context.newPage(); observe(sampler);
  await sampler.setViewportSize({ width: 1920, height: 1080 });
  for (const theme of themes) {
    await page.evaluate(theme => localStorage.setItem("syndocal.uiTheme.v1", theme), theme);
    await sampler.goto(`${origin}/theme-state-proof`);
    await sampler.locator('[data-video-perform-cell-state="LIVE"]').waitFor();
    for (const state of ["LIVE", "Preview", "Queued", "Error", "Missing", "Loaded"]) assert.ok(await sampler.locator(`[data-video-perform-cell-state="${state}"]`).count() > 0, `${theme}: real matrix renders ${state}`);
    const semantic = await sampler.evaluate(() => {
      const color = selector => { const el = document.querySelector(selector), cs = getComputedStyle(el); return { color: cs.color, backgroundColor: cs.backgroundColor, borderColor: cs.borderColor, textTransform: cs.textTransform, text: el.innerText.trim(), source: el.textContent.trim() }; };
      const token = role => { const el=document.createElement('i'); el.style.color=`var(--theme-${role})`; document.body.append(el); const value=getComputedStyle(el).color; el.remove(); return value; };
      return { program: color('.live .videoPerformClipState'), preview: color('.preview .videoPerformClipState'), queued: color('.queued .videoPerformClipState'), error: color('.error .videoPerformClipState'), take: color('[data-video-perform-take]'), latched: color('.videoPerformLayerName[aria-pressed="true"]'), clipTitle: color('.live .videoPerformClipTitle'), previewMonitor: color('[data-live-video-monitor="preview"] header'), programMonitor: color('[data-live-video-monitor="program"] header'), monitorError: color('.liveVideoMonitorStatus.error'), tokens: Object.fromEntries(['program','preview','queued','error','danger','latched','inset','text','on-program','on-preview','on-queued','on-error','on-latched'].map(role=>[role,token(role)])) };
    });
    for (const state of ["program", "preview", "queued", "error"]) {
      assert.equal(semantic[state].backgroundColor, semantic.tokens[state], `${theme} ${state} state fill`);
      assert.equal(semantic[state].color, semantic.tokens[`on-${state}`], `${theme} ${state} state ink`);
    }
    assert.equal(semantic.take.borderColor, semantic.tokens.program, `${theme} Take uses armed program border`);
    assert.equal(semantic.take.backgroundColor, semantic.tokens.inset, `${theme} Take uses inset fill at rest`);
    assert.notEqual(semantic.take.backgroundColor, semantic.program.backgroundColor, `${theme} Take rest fill is distinct from solid LIVE badge`);
    assert.equal(semantic.take.color, semantic.tokens.text, `${theme} Take uses primary text ink at rest`);
    assert.equal(semantic.take.textTransform, "uppercase", `${theme} Take action verb is uppercase chrome`);
    assert.equal(semantic.latched.text, "01 Program Layer", `${theme} state proof layer name keeps mixed case`);
    assert.equal(semantic.latched.textTransform, "none", `${theme} state proof layer name text-transform is none`);
    assert.equal(semantic.clipTitle.text, "Program source", `${theme} state proof clip title keeps mixed case`);
    assert.equal(semantic.clipTitle.textTransform, "none", `${theme} state proof clip title text-transform is none`);
    await sampler.locator('[data-video-perform-take]').hover();
    const takeHover = await sampler.locator('[data-video-perform-take]').evaluate(el => ({ backgroundColor: getComputedStyle(el).backgroundColor, color: getComputedStyle(el).color }));
    assert.equal(takeHover.backgroundColor, semantic.tokens.program, `${theme} Take hover fills with program tally`);
    assert.equal(takeHover.color, semantic.tokens["on-program"], `${theme} Take hover uses on-program ink`);
    await sampler.mouse.move(0, 0);
    assert.equal(semantic.previewMonitor.backgroundColor, semantic.tokens.preview, `${theme} Preview monitor tally`);
    assert.equal(semantic.programMonitor.backgroundColor, semantic.tokens.program, `${theme} Program monitor tally`);
    assert.equal(semantic.monitorError.backgroundColor, semantic.tokens.error, `${theme} monitor error semantic fill`);
    assert.equal(semantic.monitorError.color, semantic.tokens["on-error"], `${theme} monitor error semantic ink`);
    assert.equal(semantic.latched.backgroundColor, semantic.tokens.latched, `${theme} latched key fill`);
    assert.equal(semantic.latched.color, semantic.tokens["on-latched"], `${theme} latched key ink`);
    await sampler.locator('[data-video-perform-blackout]').click();
    assert.equal(await sampler.locator('[data-video-perform-blackout]').getAttribute('aria-pressed'), 'true', 'synthetic blackout changes rendered state');
    semantic.blackout = await sampler.locator('[data-video-perform-blackout]').evaluate(el => ({ backgroundColor: getComputedStyle(el).backgroundColor, color: getComputedStyle(el).color }));
    assert.equal(semantic.blackout.backgroundColor, semantic.tokens.danger, `${theme} engaged blackout uses danger fill`);
    assert.equal(semantic.blackout.color, semantic.tokens["on-program"], `${theme} engaged blackout uses readable program ink`);
    const filename = `perform-state-sampler-${theme}-1920x1080-en.png`;
    await sampler.screenshot({ path: resolve(artifacts, filename) });
    report.stateSamplers.push({ theme, path: filename, semantic });
  }
  await sampler.close();
  // A contact sheet is a browser canvas composition of the captured pixels.
  // Both language/viewport sets are included: two rows per set, four themes per row.
  const sheet = await context.newPage();
  for (const workspace of workspaces) {
    const images = await Promise.all(report.screenshots.filter(item => item.workspace === workspace.id).map(async item => ({ ...item, data: (await readFile(resolve(artifacts, item.path))).toString("base64") })));
    await sheet.setViewportSize({ width: 1920, height: 1288 });
    await sheet.setContent('<body style="margin:0"><canvas id="sheet" width="1920" height="1288"></canvas></body>');
    await sheet.evaluate(async images => {
      const canvas = document.getElementById("sheet"), ctx = canvas.getContext("2d");
      ctx.fillStyle = "#08090b"; ctx.fillRect(0, 0, canvas.width, canvas.height);
      for (let index = 0; index < images.length; index++) {
        const item = images[index], x = index % 4 * 480, y = Math.floor(index / 4) * 322;
        ctx.fillStyle = "#ffffff"; ctx.font = "16px Segoe UI"; ctx.fillText(`${item.workspace} / ${item.theme} / ${item.width}×${item.height} ${item.locale.toUpperCase()}`, x + 10, y + 22);
        const image = new Image(); image.src = `data:image/png;base64,${item.data}`; await image.decode();
        ctx.drawImage(image, x, y + 32, 480, 270);
      }
    }, images);
    await sheet.locator("canvas").screenshot({ path: resolve(artifacts, `${workspace.id}-themes-contact-sheet.png`) });
  }
  await sheet.close();
  assert.equal(report.screenshots.length, 80, "complete screenshot matrix");
  assert.deepEqual(report.errors, [], "no JavaScript runtime exceptions");
  const contrastFailures = report.themes.flatMap(item => item.contrasts.filter(pair => pair.required && !pair.passes).map(pair => ({ theme: item.theme, ...pair })));
  report.contrastFailures = contrastFailures;
  assert.deepEqual(contrastFailures, [], "WCAG AA 4.5:1 token foreground/background semantic pairs");
  const renderedTextFailures = report.textContrast.flatMap(item => item.failures.map(failure => ({ workspace: item.workspace, theme: item.theme, locale: item.locale, ...failure })));
  report.renderedTextFailureCount = renderedTextFailures.length;
  assert.equal(renderedTextFailures.length, 0, `WCAG AA on visible solid-background text: ${JSON.stringify(renderedTextFailures.slice(0, 12))}`);
  const roleFailures = report.typeRoles.flatMap(item => [
    ...item.under.map(entry => ({ rule: "text under 13px", workspace: item.workspace, theme: item.theme, locale: item.locale, ...entry })),
    ...item.controlsUnder.map(entry => ({ rule: "interactive label or list item under 14px", workspace: item.workspace, theme: item.theme, locale: item.locale, ...entry })),
    ...item.condensedOutside.map(entry => ({ rule: "condensed face outside chrome", workspace: item.workspace, theme: item.theme, locale: item.locale, ...entry })),
    ...item.uncalmTitles.map(entry => ({ rule: "pane title is not calm", workspace: item.workspace, theme: item.theme, locale: item.locale, ...entry })),
  ]);
  report.typeRoleFailureCount = roleFailures.length;
  assert.equal(roleFailures.length, 0, `type roles (13px floor, 14px controls, chrome-only condensed face, calm pane titles): ${JSON.stringify(roleFailures.slice(0, 8))}`);
  report.passed = true;
} catch (error) {
  report.passed = false;
  report.failure = String(error.stack ?? error);
  console.error(report.failure);
  process.exitCode = 1;
} finally {
  await writeFile(resolve(artifacts, "browser-theme-report.json"), JSON.stringify(report, null, 2));
  await browser?.close();
  await server?.close();
  await unlink(stateFixturePath).catch(() => {});
}
