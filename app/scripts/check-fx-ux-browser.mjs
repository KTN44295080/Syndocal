// Browser fixture for the effect browser, parameter editor, presets, Preview
// audition and rejection explanations. It mounts the real components with a
// reactive in-memory layer so the whole operator flow runs without Tauri.
// Screenshots go to SYNDOCAL_FX_UX_SCREENSHOT_DIR (default target/qa/fx-ux-12).
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
const evidenceDir = resolve(process.env.SYNDOCAL_FX_UX_SCREENSHOT_DIR ?? join(appRoot, "../target/qa/fx-ux-12"));
const browserPath = [process.env.CHROME_PATH, "C:/Program Files/Google/Chrome/Application/chrome.exe", "C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe", "C:/Program Files/Microsoft/Edge/Application/msedge.exe"].filter(Boolean).find(existsSync);
assert.ok(browserPath, "Chromium browser is required");
const modulePath = "/src/__fx-ux-fixture.tsx";
const moduleId = join(appRoot, modulePath.slice(1)).replaceAll("\\", "/");

const fixture = String.raw`
import { createMemo, createRoot, createSignal } from "solid-js";
import { render } from "solid-js/web";
import { VideoPerformQuickFx } from "/src/components/VideoPerformQuickFx.tsx";
import { VideoEffectBrowser } from "/src/components/VideoEffectBrowser.tsx";
import { VideoIsfEffectPanel } from "/src/components/VideoIsfEffectPanel.tsx";
import { VideoFxProblemsDialog } from "/src/components/VideoFxProblemsDialog.tsx";
import { buildFxCatalog, builtinFxEntries } from "/src/fxCatalogModel.ts";
import { createFxAuditionSession } from "/src/createFxAuditionSession.ts";
import { fxChainFromStages, fxChainStages } from "/src/fxAuditionModel.ts";
import { collectFxProblems } from "/src/fxRejectionExplanation.ts";
import { installUiLocalization } from "/src/uiLocalization.ts";
import "/src/theme.css"; import "/src/styles.css";

const control = (name, kind, value, minimum, maximum, extras = {}) => ({ name, kind, value, default: [...value], minimum, maximum, labels: [], values: [], ...extras });
const z = [0, 0, 0, 0];
const builtin = (id, controls) => { const entry = builtinFxEntries.find((item) => item.builtinId === id); return { enabled: true, label: entry.label, source: entry.source, categories: [], controls }; };
const stages = {
  invert: () => builtin("invert", []),
  colorize: () => builtin("colorize", [control("tint", "Color", [0.1, 0.7, 1, 1], z, [1, 1, 1, 1]), control("amount", "Float", [0.75, 0, 0, 0], z, [1, 0, 0, 0])]),
  monochrome: () => builtin("monochrome", [control("amount", "Float", [1, 0, 0, 0], z, [1, 0, 0, 0])]),
  rotate: () => builtin("rotate", [control("angle", "Float", [0.5, 0, 0, 0], [-3.14159, 0, 0, 0], [3.14159, 0, 0, 0]), control("center", "Point2d", [0.5, 0.5, 0, 0], z, [1, 1, 0, 0])]),
  strobe: () => builtin("strobe", [control("rate", "Float", [8, 0, 0, 0], [0.5, 0, 0, 0], [30, 0, 0, 0]), control("duty", "Float", [0.5, 0, 0, 0], [0.05, 0, 0, 0], [0.95, 0, 0, 0])]),
  mirror: () => builtin("mirror", [control("mode", "Long", [0, 0, 0, 0], z, [2, 0, 0, 0], { labels: ["Horizontal", "Vertical", "Both"], values: [0, 1, 2] })]),
};
createRoot(() => {
const [locale, setLocale] = createSignal("en");
const [rejectionsOn, setRejectionsOn] = createSignal(false);
const [runtimeErrors, setRuntimeErrors] = createSignal([]);
const initialLayers = () => [
  { id: 1, label: "Opener Loop", isf_effect: null },
  { id: 2, label: "Strobe Wash", isf_effect: fxChainFromStages([stages.colorize(), stages.monochrome(), stages.strobe(), stages.rotate()]) },
  { id: 3, label: "Single Stage", isf_effect: fxChainFromStages([stages.invert()]) },
];
const [layers, setLayers] = createSignal(initialLayers());
const [transport, setTransport] = createSignal({ layer_id: null, effect_audition: false });
const [catalog, setCatalog] = createSignal({ effect_chains: [], effect_presets: [], layer_groups: [], transition_buses: [] });
const log = { commits: [], previews: [], liveWrites: 0, moves: 0, sets: 0, catalogWrites: 0, adds: [], auditions: [] };
const [bpm, setBpm] = createSignal(120);
const updateLayer = (id, change) => setLayers(layers().map((layer) => layer.id === id ? { ...layer, isf_effect: change(layer.isf_effect) } : layer));
const mapStages = (effect, change) => fxChainFromStages(fxChainStages(effect).map(change));
const live = {
  layers: () => layers(),
  onSetIsfEffectEnabled: (id, index, enabled) => { log.liveWrites++; updateLayer(id, (effect) => mapStages(effect, (stage, at) => at === index ? { ...stage, enabled } : stage)); },
  onMoveIsfEffect: (id, index, delta) => { log.liveWrites++; log.moves++; updateLayer(id, (effect) => { const list = fxChainStages(effect); [list[index], list[index + delta]] = [list[index + delta], list[index]]; return fxChainFromStages(list); }); },
  onRemoveIsfEffect: (id, index) => { log.liveWrites++; updateLayer(id, (effect) => fxChainFromStages(fxChainStages(effect).filter((_, at) => at !== index))); },
  onSetIsfEffect: (id, effect) => { log.liveWrites++; log.sets++; updateLayer(id, () => effect); },
  onSetIsfControl: (id, index, name, value) => { log.liveWrites++; updateLayer(id, (effect) => mapStages(effect, (stage, at) => at === index ? { ...stage, controls: stage.controls.map((item) => item.name === name ? { ...item, value } : item) } : stage)); },
  onTriggerIsfEvent: () => { log.liveWrites++; },
  isfEventPulseBusy: false,
};
const liveSession = { current: () => null, busy: () => false, canUndo: () => false, undo: async () => {}, compare: async () => {}, run: async (_id, action) => action() };
const audition = createFxAuditionSession({
  layers: () => layers(),
  transport,
  stagePreview: async (layerId) => { setTransport({ layer_id: layerId, effect_audition: false }); return true; },
  setPreviewEffect: async (layerId, active, effect) => { log.previews.push({ layerId, active, labels: effect ? fxChainStages(effect).map((stage) => stage.label) : null }); setTransport({ layer_id: layerId, effect_audition: active }); return transport(); },
  describeBuiltin: async (id) => { const make = stages[id]; return make ? fxChainFromStages([make()]) : null; },
  commit: async (layerId, effect) => { log.commits.push({ layerId, labels: effect ? fxChainStages(effect).map((stage) => stage.label) : [] }); updateLayer(layerId, () => effect); },
});
const badSource = '/*{"INPUTS":[{"NAME":"inputImage","TYPE":"image"},{"NAME":"music","TYPE":"audioFFT"}]}*/ void main(){}';
const rejections = [
  { label: "Rejected ISF", source: badSource, message: "ISF audio/audioFFT textures are not supported by this runtime; use Syndocal Audio FFT nodes" },
  { label: "Rejected ISF", source: '/*{"INPUTS":[]}*/\nvoid main(){\n for (int i=0;i<9;i++){}\n}', message: "ISF source uses forbidden or unsupported construct 'for ('" },
];
const [imported, setImported] = createSignal([]);
const entries = createMemo(() => buildFxCatalog({ imported: imported(), rejections: rejectionsOn() ? rejections : [], layers: layers(), runtimeErrors: [] }));
const problems = createMemo(() => collectFxProblems({ layers: layers(), runtimeErrors: runtimeErrors(), importRejections: rejectionsOn() ? rejections : [], locale: locale() }));
const [selectedLayerId, setSelectedLayerId] = createSignal(2);
const [view, setView] = createSignal("fx");
const [size, setSize] = createSignal({ width: 446, height: 236 });
const layer = () => layers().find((item) => item.id === selectedLayerId());
let dialog; let localizer;
Object.assign(window, { __fxUx: {
  log, setBpm, resetLayers: () => setLayers(initialLayers()), setLocale: (value) => { setLocale(value); document.documentElement.lang = value; localizer?.refresh(); }, setView, setSize, setSelectedLayerId, setRejectionsOn,
  setRuntimeErrors, layers, transport, audition, catalog, openProblems: () => dialog.showModal(), problems,
  setCatalog: (value) => { log.catalogWrites++; setCatalog(value); },
}});
const frame = () => ({ width: size().width + "px", height: size().height + "px" });
render(() => <div class="fxUxStage">
  <div class="fxUxFrame" style={frame()} data-video-perform-pane="inspector">
    {view() === "full"
      ? <VideoIsfEffectPanel layerId={layer().id} layerLabel={layer().label} effect={layer().isf_effect} locale={locale()} bpm={bpm()} projectEpoch={1} liveSession={liveSession}
          effectCatalog={catalog()} onApplyEffectCatalog={(value) => { window.__fxUx.setCatalog(value); return value; }}
          onImport={() => {}} onApplyBuiltin={() => {}} onResetEffect={() => {}}
          onSetEffect={live.onSetIsfEffect} onMoveEffect={live.onMoveIsfEffect} onRemoveEffect={live.onRemoveIsfEffect} onSetEffectEnabled={live.onSetIsfEffectEnabled}
          onSetControl={live.onSetIsfControl} onTriggerEvent={live.onTriggerIsfEvent} />
      : view() === "fx"
      ? <VideoPerformQuickFx layer={layer()} layerList={live} locale={locale()} bpm={bpm()} projectEpoch={1} onOpenEditor={() => {}}
          effectCatalog={catalog()} onApplyEffectCatalog={(value) => { window.__fxUx.setCatalog(value); return value; }}
          liveSession={liveSession} audition={audition} />
      : <VideoEffectBrowser entries={entries()} layer={layer()} locale={locale()} onAdd={(entry, layerId) => { log.adds.push([entry.id, layerId]); }}
          audition={audition} onAudition={(entry, layerId) => { log.auditions.push([entry.id, layerId]); return audition.addToAudition(layerId, entry.builtinId ? audition.describeBuiltinStage(entry.builtinId) : async () => null); }}
          onImport={() => {}} onOpenFx={() => setView("fx")} />}
  </div>
  <VideoFxProblemsDialog ref={(element) => { dialog = element; }} problems={problems()} locale={locale()} onClose={() => {}} />
</div>, document.getElementById("root"));
localizer = installUiLocalization(document.body, locale);
document.documentElement.dataset.theme = "tally";
window.__fxUxReady = true;
});
`;
const plugin = {
  name: "fx-ux-fixture", enforce: "pre",
  resolveId(id) { if (id === modulePath) return moduleId; },
  load(id) { if (id === moduleId) return fixture; },
  configureServer(server) {
    server.middlewares.use((request, response, next) => {
      if (request.url !== "/__fx-ux") return next();
      response.setHeader("Content-Type", "text/html");
      response.end(`<!doctype html><html lang="en" data-theme="tally"><head><meta charset="utf-8"><title>FX UX fixture</title><style>html,body{margin:0;background:var(--theme-bg,#000);overflow:hidden}.fxUxStage{padding:0}.fxUxFrame{overflow:hidden;background:var(--theme-panel);box-sizing:border-box}</style></head><body><div id="root"></div><script type="module" src="${modulePath}"></script></body></html>`);
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
const key = async (name, code = name, vk) => { const params = { key: name, code, ...(vk ? { windowsVirtualKeyCode: vk, nativeVirtualKeyCode: vk } : {}) }; await send("Input.dispatchKeyEvent", { type: "keyDown", ...params, ...(name === "Enter" ? { text: "\r", unmodifiedText: "\r" } : {}) }); await send("Input.dispatchKeyEvent", { type: "keyUp", ...params }); };
const shot = async (name) => { await mkdir(evidenceDir, { recursive: true }); const { data } = await send("Page.captureScreenshot", { format: "png", captureBeyondViewport: false }, 30_000); await writeFile(join(evidenceDir, `${name}.png`), Buffer.from(data, "base64")); };
const ux = (code) => evaluate(`(async()=>{const u=window.__fxUx;${code}})()`);
const click = (selector) => evaluate(`(()=>{const e=document.querySelector(${JSON.stringify(selector)});if(!e)throw Error('missing '+${JSON.stringify(selector)});if(e.disabled)throw Error('disabled '+${JSON.stringify(selector)});e.click();return true})()`);
const text = (selector) => evaluate(`document.querySelector(${JSON.stringify(selector)})?.innerText??null`);
const settle = async () => { await delay(120); };
const setView = async (viewport, locale, theme, view, width, height, layer = 2) => {
  await send("Emulation.setDeviceMetricsOverride", { width: viewport[0], height: viewport[1], deviceScaleFactor: 1, mobile: false });
  await ux(`u.setLocale(${JSON.stringify(locale)});u.setSize({width:${width},height:${height}});u.setSelectedLayerId(${layer});u.setView(${JSON.stringify(view)});document.documentElement.dataset.theme=${JSON.stringify(theme)};`);
  await settle();
};
const geometry = (selector) => evaluate(`[...document.querySelectorAll(${JSON.stringify(selector)})].filter(e=>e.getBoundingClientRect().width).map(e=>{const r=e.getBoundingClientRect();return {w:Math.round(r.width),h:Math.round(r.height),text:(e.innerText||e.getAttribute('aria-label')||'').slice(0,40),font:parseFloat(getComputedStyle(e).fontSize)}})`);

try {
  server = await createServer({ root: appRoot, server: { host: "127.0.0.1", port: vitePort, strictPort: vitePort !== 0 }, plugins: [plugin], clearScreen: false, logLevel: "error" });
  await server.listen();
  const profileRoot = resolve(appRoot, "../target/qa/browser-profiles");
  await mkdir(profileRoot, { recursive: true });
  profile = await mkdtemp(join(profileRoot, "syndocal-fx-ux-"));
  browser = spawn(browserPath, ["--headless=new", "--use-angle=swiftshader", "--enable-unsafe-swiftshader", "--no-first-run", "--no-default-browser-check", "--remote-debugging-port=" + cdpPort, `--user-data-dir=${profile}`, "about:blank"], { windowsHide: true, stdio: "ignore" });
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
  await send("Page.navigate", { url: `${server.resolvedUrls.local[0]}__fx-ux` });
  await until(() => evaluate("window.__fxUxReady === true"), "fixture mounted").catch(async (error) => {
    throw new Error(`${error.message}; page errors: ${JSON.stringify(errors).slice(0, 2000)}; body: ${(await evaluate("document.body.innerText").catch(() => "")).slice(0, 600)}`);
  });

  // ---- Parameter editing (maximised Inspector pane), English then Japanese ----
  await setView([1920, 1080], "en", "tally", "fx", 700, 620);
  await click('[data-perform-fx-stage="0"] [data-perform-fx-select]');
  await settle();
  const labels = await evaluate("[...document.querySelectorAll('[data-perform-fx-control] .videoFxParameterHead')].map(e=>e.innerText.replace(/\\s+/g,' '))");
  assert.equal(labels.length, 2, "both parameters render");
  assert.match(labels[0], /^Tint\s+#1AB3FF · 100 %$/, "colour value reads as a hex colour with its opacity");
  assert.match(labels[1], /^Amount\s+75 %$/, "a 0-1 float reads as a percentage");
  assert.equal(await evaluate("document.querySelectorAll('[data-perform-fx-control=tint] input[type=color]').length"), 1, "colour parameters get a picker");
  assert.equal(await evaluate("document.querySelectorAll('[data-perform-fx-control] .videoFxParameterFoot button').length"), 2, "every parameter can reset to default");
  assert.match(await text("[data-perform-fx-control=amount] .videoFxParameterFoot"), /Range\s*:\s*0 % – 100 %/, "range is shown with units");
  await shot("params-en-tally-1920x1080");
  // A parameter change is live when no audition is open, and is reported through the live commands only.
  await ux("const input=document.querySelector('[data-perform-fx-control=amount] input[type=range]');input.value='0.4';input.dispatchEvent(new Event('change',{bubbles:true}));");
  await settle();
  assert.equal(await ux("return u.log.liveWrites"), 1, "live edits use the live command");
  assert.match(await text("[data-perform-fx-control=amount] .videoFxParameterHead output"), /40 %/);
  await click('[data-perform-fx-control=amount] .videoFxParameterFoot button'); await settle();
  assert.match(await text("[data-perform-fx-control=amount] .videoFxParameterHead output"), /75 %/, "reset returns the declared default");
  // A point parameter has a 2D pad: a click publishes one value, the arrow keys step it.
  await click('[data-perform-fx-stage="3"] [data-perform-fx-select]'); await settle();
  const pad = await evaluate("(()=>{const r=document.querySelector('[data-fx-point-pad]').getBoundingClientRect();return {x:r.left,y:r.top,w:r.width,h:r.height}})()");
  const writesBeforePad = await ux("return u.log.liveWrites");
  await send("Input.dispatchMouseEvent", { type: "mousePressed", x: pad.x + pad.w * 0.25, y: pad.y + pad.h * 0.25, button: "left", buttons: 1, clickCount: 1 });
  await send("Input.dispatchMouseEvent", { type: "mouseReleased", x: pad.x + pad.w * 0.25, y: pad.y + pad.h * 0.25, button: "left", buttons: 0, clickCount: 1 });
  await settle();
  assert.equal(await ux("return u.log.liveWrites"), writesBeforePad + 1, "a pad gesture publishes exactly once");
  const centre = await ux("return u.layers()[1].isf_effect.stack[2].controls.find(c=>c.name==='center').value");
  assert.ok(Math.abs(centre[0] - 0.25) < 0.03 && Math.abs(centre[1] - 0.75) < 0.03, `pad maps the click to x 0.25 / y 0.75 (${centre})`);
  await evaluate("document.querySelector('[data-fx-point-pad]').focus()"); await key("ArrowRight", "ArrowRight", 39); await settle();
  assert.ok(Math.abs((await ux("return u.layers()[1].isf_effect.stack[2].controls.find(c=>c.name==='center').value[0]")) - 0.26) < 0.005, "the arrow key steps the point by one percent");
  assert.match(await text("[data-perform-fx-control=center] .videoFxParameterHead output"), /0\.26, 0\.75/, "the point reads as X, Y values");
  await shot("params-point-pad-en-tally-1920x1080");
  // Selecting the strobe stage shows Hz units; presets are listed for built-ins.
  await click('[data-perform-fx-stage="2"] [data-perform-fx-select]'); await settle();
  assert.match(await text("[data-perform-fx-control=rate] .videoFxParameterHead output"), /8 Hz/, "strobe rate carries its unit");
  await evaluate("document.querySelector('[data-fx-presets]').open=true"); await settle();
  assert.ok((await evaluate("[...document.querySelectorAll('[data-fx-presets] select option')].map(e=>e.textContent)")).includes("Slow pulse"), "built-in presets are offered");
  await shot("presets-en-tally-1920x1080");
  // Save, recall and delete a named preset through the project catalog.
  await evaluate("const e=document.querySelector('[data-fx-presets] input[type=text]');e.focus();e.value='Club strobe';e.dispatchEvent(new Event('input',{bubbles:true}))");
  await click("[data-fx-preset-save]"); await settle();
  assert.equal(await ux("return u.catalog().effect_presets.length"), 1, "preset saved into the project catalog");
  assert.equal(await ux("return u.catalog().effect_presets[0].label"), "Club strobe");
  await evaluate("const s=document.querySelector('[data-fx-presets] select');s.value=[...s.options].find(o=>o.textContent==='Club strobe').value;s.dispatchEvent(new Event('change',{bubbles:true}))"); await settle();
  await click("[data-fx-preset-recall]"); await settle();
  await click("[data-fx-preset-delete]"); await settle();
  assert.equal(await ux("return u.catalog().effect_presets.length"), 0, "preset deleted from the project catalog");
  // Randomize stays inside the declared range and Undo restores the previous value.
  await click("[data-fx-randomize]"); await settle();
  const rate = Number(await evaluate("document.querySelector('[data-perform-fx-control=rate] input[type=range]').value"));
  assert.ok(rate >= 0.5 && rate <= 30, `randomized rate ${rate} stays within 0.5-30`);

  // ---- Browser tab: rejection cards in both languages and themes ----
  await ux("u.setRejectionsOn(true)");
  for (const [viewport, locale, theme, width, height] of [[[1366, 768], "ja", "tally", 446, 236], [[1920, 1080], "en", "cobalt", 628, 408], [[1920, 1080], "ja", "cobalt", 900, 640]]) {
    await setView(viewport, locale, theme, "browser", width, height);
    await evaluate("document.querySelector('[data-effect-browser]').scrollTop=999999"); await settle();
    const rejectionCards = await evaluate("document.querySelectorAll('[data-fx-card-state=rejected]').length");
    assert.equal(rejectionCards, 2, "rejected imports appear as cards");
    await evaluate("document.querySelector('[data-fx-card-state=rejected]').scrollIntoView({block:'start'})"); await settle();
    const explained = await text("[data-fx-card-state=rejected] [data-fx-rejection]");
    assert.match(explained, locale === "ja" ? /音声/ : /Audio inputs/, `${locale}: reason is plain language`);
    assert.ok(!/ParseErrors|Span \{/.test(explained.split("\n").slice(0, 3).join("\n")), "stack text is not the headline");
    assert.equal(await evaluate("document.querySelector('[data-fx-card-state=rejected] [data-fx-catalog-add]').disabled"), true, "rejected cards cannot be added");
    assert.equal(await evaluate("document.querySelector('[data-fx-card-state=rejected] [data-fx-catalog-audition]').disabled"), true, "rejected cards cannot be auditioned");
    await shot(`rejection-${locale}-${theme}-${viewport[0]}x${viewport[1]}`);
  }
  // The media-health dialog lists each reason and keeps the raw text expandable.
  await setView([1366, 768], "ja", "tally", "fx", 446, 236);
  await ux("u.openProblems()"); await settle();
  assert.equal(await evaluate("document.querySelectorAll('[data-fx-problem]').length"), 2);
  assert.equal(await evaluate("document.querySelector('[data-fx-problem] details').open"), false, "raw text starts collapsed");
  await evaluate("document.querySelector('[data-fx-problem] details').open=true"); await settle();
  assert.match(await text("[data-fx-problem] pre"), /audioFFT/, "raw engine text is available to experts");
  await shot("problems-ja-tally-1366x768");
  await evaluate("document.querySelector('[data-fx-problems-dialog]').close()");
  await ux("u.setRejectionsOn(false)");

  // ---- Narrow Inspector (1366x768): the slider keeps a usable length beside its number field ----
  await setView([1366, 768], "ja", "tally", "fx", 446, 236);
  await click('[data-perform-fx-stage="2"] [data-perform-fx-select]'); await settle();
  const narrow = await evaluate("(()=>{const r=document.querySelector('[data-perform-fx-control=rate] input[type=range]').getBoundingClientRect();return Math.round(r.width)})()");
  assert.ok(narrow >= 120, `the slider stays usable in the narrow Inspector (${narrow}px)`);
  await shot("params-ja-tally-1366x768");

  // ---- Audition: Preview only, A/B, Take ----
  await setView([1920, 1080], "en", "tally", "browser", 700, 560, 1);
  const programBefore = await ux("return JSON.stringify(u.layers())");
  await click('[data-fx-catalog-card="builtin:invert"] [data-fx-catalog-audition]'); await settle();
  await ux("");
  assert.equal(await ux("return u.log.commits.length"), 0, "Try never commits");
  assert.equal(await ux("return JSON.stringify(u.layers())"), programBefore, "Try never changes the live layer");
  assert.match(await text("[data-fx-auditioning]"), /Auditioning on Preview/);
  await setView([1920, 1080], "en", "tally", "fx", 700, 560, 1);
  assert.equal(await evaluate("document.querySelector('[data-fx-audition-bar]').dataset.fxAuditionActive"), "true");
  assert.equal(await evaluate("document.querySelectorAll('[data-perform-fx-stage]').length"), 1, "the draft chain is listed");
  assert.match(await text("[data-fx-audition-bar]"), /AUDITION\s+·\s+PREVIEW ONLY/i);
  await shot("audition-en-tally-1920x1080");
  // Draft edits stay in the draft and Preview.
  await click('[data-perform-fx-stage="0"] [data-perform-fx-select]');
  const writesBefore = await ux("return u.log.liveWrites");
  await ux("u.audition.setEnabled(1,0,false)"); await settle();
  assert.equal(await ux("return u.log.liveWrites"), writesBefore, "draft edits never use the live commands");
  assert.equal(await ux("return u.log.commits.length"), 0);
  await click('[data-fx-audition-side="A"]'); await settle();
  assert.equal(await evaluate("document.querySelector('[data-fx-audition-side=A]').getAttribute('aria-pressed')"), "true");
  assert.equal(await ux("return u.log.previews.at(-1).active"), false, "A shows the live chain on Preview");
  await click('[data-fx-audition-side="B"]'); await settle();
  assert.equal(await ux("return u.log.previews.at(-1).active"), true);
  // Take needs a second explicit click.
  await click("[data-fx-audition-take]"); await settle();
  assert.equal(await ux("return u.log.commits.length"), 0, "the first Take click only arms the confirmation");
  assert.match(await text("[data-fx-audition-take-confirm]"), /confirm/i);
  await click("[data-fx-audition-take-cancel]"); await settle();
  assert.equal(await ux("return u.log.commits.length"), 0);
  await click("[data-fx-audition-take]"); await click("[data-fx-audition-take-confirm]"); await settle();
  assert.equal(await ux("return u.log.commits.length"), 1, "the confirmed Take commits once");
  assert.equal(await ux("return u.audition.state()"), null, "Take ends the audition");
  assert.equal(await evaluate("document.querySelector('[data-fx-audition-bar]').dataset.fxAuditionActive"), "false");

  // Japanese audition bar.
  await setView([1366, 768], "ja", "cobalt", "fx", 446, 236, 2);
  await click("[data-fx-audition-begin]"); await settle();
  await shot("audition-ja-cobalt-1366x768");
  await click("[data-fx-audition-discard]"); await settle();
  assert.equal(await ux("return u.audition.state()"), null);

  // ---- Task 18 D1: tempo-synced beat and bar units ----
  await ux("u.resetLayers();u.setBpm(120)");
  await setView([1920, 1080], "en", "tally", "fx", 700, 620);
  await click('[data-perform-fx-stage="2"] [data-perform-fx-select]'); await settle();
  const syncBox = "[data-perform-fx-control=rate] [data-fx-tempo-sync]";
  // Serialised in the page: CDP flattens store proxies, JSON.stringify is what the project writer sees.
  const rateControl = async () => JSON.parse(await ux("return JSON.stringify(u.layers()[1].isf_effect.stack[1].controls.find(c=>c.name==='rate'))"));
  const pickCount = (value) => evaluate(`(()=>{const s=document.querySelector(${JSON.stringify(syncBox + " [data-fx-tempo-count]")});s.value=${JSON.stringify(String(value))};s.dispatchEvent(new Event('change',{bubbles:true}))})()`).then(settle);
  const readout = () => text(`${syncBox} [data-fx-tempo-readout]`);
  const visibleRate = () => evaluate(`({
    header: document.querySelector('[data-perform-fx-control=rate] .videoFxParameterHead output').textContent,
    range: document.querySelector('[data-perform-fx-control=rate] input[type=range]').value,
    number: document.querySelector('[data-perform-fx-control=rate] input[type=number]').value,
    accessible: document.querySelector('[data-perform-fx-control=rate] input[type=range]').getAttribute('aria-valuetext')
  })`);
  assert.equal(await evaluate("document.querySelectorAll('[data-fx-tempo-sync]').length"), 1, "only the Strobe rate offers beat and bar units, not the duty ratio");
  assert.equal(await evaluate(`document.querySelector(${JSON.stringify(syncBox)}).dataset.fxTempoSync`), "hz", "controls start as a raw Hz value");
  assert.deepEqual(await evaluate(`[...document.querySelectorAll(${JSON.stringify(syncBox + " [data-fx-tempo-unit]")})].map(e=>e.textContent+':'+e.getAttribute('aria-pressed'))`), ["Hz:true", "Beats:false", "Bars:false"]);
  for (const item of await geometry(`${syncBox} button`)) { assert.ok(item.h >= 44 && item.w >= 44 && item.font >= 14, `tempo unit button keeps 44px and 14px ${JSON.stringify(item)}`); }
  let sets = await ux("return u.log.sets"); let writes = await ux("return u.log.liveWrites");
  await click(`${syncBox} [data-fx-tempo-unit=beats]`); await settle();
  assert.equal(await ux("return u.log.sets"), sets + 1, "choosing a unit is one write through the ticketed chain command");
  let rateNow = await rateControl();
  assert.deepEqual(rateNow.tempo_sync, { unit: "beats", count: 1 }, "the choice is stored on the control, which is project state");
  assert.equal(rateNow.value[0], 2, "120 BPM, one cycle per beat = 2 Hz");
  assert.equal(await readout(), "One cycle per 1 beat = 2 Hz at 120 BPM");
  assert.equal(await evaluate("document.querySelector('[data-perform-fx-control=rate] input[type=range]').disabled && document.querySelector('[data-perform-fx-control=rate] input[type=number]').disabled"), true, "a synced value is not hand-edited");
  assert.deepEqual(await evaluate(`[...document.querySelectorAll(${JSON.stringify(syncBox + " [data-fx-tempo-count] option")})].map(e=>e.textContent)`), ["1/4", "1/2", "1", "2", "4"]);
  assert.equal(await evaluate(`document.querySelector(${JSON.stringify(syncBox + " [data-fx-tempo-count]")}).getAttribute('aria-label')`), "Beats per cycle");
  await pickCount(0.25);
  rateNow = await rateControl();
  assert.deepEqual([rateNow.tempo_sync.count, rateNow.value[0]], [0.25, 8], "1/4 beat at 120 BPM = 8 Hz");
  assert.equal(await readout(), "One cycle per 1/4 beat = 8 Hz at 120 BPM");
  await click(`${syncBox} [data-fx-tempo-unit=bars]`); await settle();
  rateNow = await rateControl();
  assert.deepEqual([rateNow.tempo_sync.unit, rateNow.tempo_sync.count, rateNow.value[0]], ["bars", 0.25, 2], "a bar is four beats: 1/4 bar = 1 beat = 2 Hz");
  assert.equal(await readout(), "One cycle per 1/4 bar = 2 Hz at 120 BPM");
  assert.equal(await evaluate(`document.querySelector(${JSON.stringify(syncBox + " [data-fx-tempo-count]")}).getAttribute('aria-label')`), "Bars per cycle");
  assert.equal(await evaluate("document.querySelectorAll('[data-fx-tempo-resync]').length"), 0, "no re-sync is offered while the value follows the clock");
  await shot("tempo-sync-en-tally-1920x1080");
  // Live clock changes update every visible rate without publishing authored state.
  writes = await ux("return u.log.liveWrites");
  const authoredRate = await rateControl();
  await ux("u.setBpm(90)"); await settle();
  assert.deepEqual(await rateControl(), authoredRate, "a tempo change preserves authored values and metadata");
  assert.equal(await ux("return u.log.liveWrites"), writes, "a tempo change never publishes a project write");
  assert.equal(await readout(), "One cycle per 1/4 bar = 1.5 Hz at 90 BPM");
  assert.deepEqual(await visibleRate(), { header: "1.5 Hz", range: "1.5", number: "1.5", accessible: "1.5 Hz" });
  assert.equal(await evaluate("document.querySelectorAll('[data-fx-tempo-resync]').length"), 0, "runtime following has no re-sync action");
  await ux("u.setBpm(180)"); await settle();
  assert.deepEqual(await visibleRate(), { header: "3 Hz", range: "3", number: "3", accessible: "3 Hz" });
  assert.deepEqual(await rateControl(), authoredRate);
  assert.equal(await ux("return u.log.liveWrites"), writes);
  // Retained engine BPM keeps the nominal rate; native tests cover loss of clock pulses.
  await ux("u.setBpm(180)"); await settle();
  assert.deepEqual(await visibleRate(), { header: "3 Hz", range: "3", number: "3", accessible: "3 Hz" });
  assert.equal(await ux("return u.log.liveWrites"), writes);
  // Invalid clock data uses the saved raw value and returns to live following on recovery.
  await ux("u.setBpm(0)"); await settle();
  assert.deepEqual(await visibleRate(), { header: "2 Hz", range: "2", number: "2", accessible: "2 Hz" });
  assert.match(await readout(), /2 Hz.*Tempo unavailable; using the saved rate\./);
  assert.deepEqual(await rateControl(), authoredRate);
  assert.equal(await ux("return u.log.liveWrites"), writes);
  await ux("u.setBpm(90)"); await settle();
  assert.equal(await readout(), "One cycle per 1/4 bar = 1.5 Hz at 90 BPM");
  // A request outside the control's range is limited and says so.
  await pickCount(4);
  rateNow = await rateControl();
  assert.equal(rateNow.value[0], 0.5, "4 bars at 90 BPM = 0.094 Hz is limited to the 0.5 Hz minimum");
  assert.equal(await evaluate("document.querySelectorAll('[data-fx-tempo-clamped]').length"), 1, "the limit is shown");
  assert.equal(await evaluate("document.querySelectorAll('[data-fx-tempo-resync]').length"), 0, "a value at its limit is in sync");
  // Randomize leaves a synced control alone; the other controls still change.
  await click(`${syncBox} [data-fx-tempo-unit=beats]`); await settle();
  const beforeRandom = (await rateControl()).value[0];
  await click("[data-fx-randomize]"); await settle();
  rateNow = await rateControl();
  assert.deepEqual([rateNow.tempo_sync.unit, rateNow.value[0]], ["beats", beforeRandom], "Randomize keeps a synced control on the clock");
  // Hz captures the currently displayed rate after a tempo change.
  writes = await ux("return u.log.liveWrites");
  await ux("u.setBpm(240)"); await settle();
  assert.equal((await rateControl()).value[0], beforeRandom);
  assert.equal(await ux("return u.log.liveWrites"), writes);
  assert.deepEqual(await visibleRate(), { header: "1 Hz", range: "1", number: "1", accessible: "1 Hz" });
  await click(`${syncBox} [data-fx-tempo-unit=hz]`); await settle();
  rateNow = await rateControl();
  assert.equal(rateNow.tempo_sync ?? null, null, "Hz removes the sync");
  assert.equal(rateNow.value[0], 1, "Hz captures the effective rate instead of restoring a stale authored rate");
  assert.equal(await ux("return u.log.liveWrites"), writes + 1, "switching to Hz is one explicit write");
  // Reset default replaces the sync with the raw default.
  await click(`${syncBox} [data-fx-tempo-unit=bars]`); await settle();
  await click("[data-perform-fx-control=rate] .videoFxParameterFoot button"); await settle();
  rateNow = await rateControl();
  assert.deepEqual([rateNow.tempo_sync ?? null, rateNow.value[0]], [null, 8], "Reset default ends the sync and restores the declared default");
  // Without a usable tempo the beat and bar units are off, with the reason.
  await ux("u.setBpm(0)"); await settle();
  assert.equal(await evaluate(`[...document.querySelectorAll(${JSON.stringify(syncBox + " [data-fx-tempo-unit]")})].map(e=>e.disabled).join()`), "false,true,true", "beats and bars need a tempo");
  assert.equal(await evaluate(`document.querySelector(${JSON.stringify(syncBox + " [data-fx-tempo-unit=beats]")}).title`), "Tempo unavailable");
  await ux("u.setBpm(120)"); await settle();
  // Japanese.
  await ux("u.setLocale('ja')"); await settle();
  await click(`${syncBox} [data-fx-tempo-unit=beats]`); await settle();
  assert.equal(await readout(), "1拍ごとに1回 ＝ 2 Hz（120 BPM）");
  assert.deepEqual(await evaluate(`[...document.querySelectorAll(${JSON.stringify(syncBox + " [data-fx-tempo-unit]")})].map(e=>e.textContent)`), ["Hz", "拍", "小節"]);
  assert.equal(await evaluate(`document.querySelector(${JSON.stringify(syncBox + " [data-fx-tempo-count]")}).getAttribute('aria-label')`), "1周期あたりの拍数");
  assert.equal(await evaluate(`document.querySelector(${JSON.stringify(syncBox + " [role=group]")}).getAttribute('aria-label')`), "時間の単位");
  writes = await ux("return u.log.liveWrites");
  await ux("u.setBpm(100)"); await settle();
  assert.equal(await readout(), "1拍ごとに1回 ＝ 1.667 Hz（100 BPM）");
  const japaneseRate = await visibleRate();
  assert.equal(japaneseRate.header, "1.667 Hz");
  assert.equal(japaneseRate.number, "1.667");
  assert.equal(japaneseRate.accessible, "1.667 Hz");
  assert.ok(Math.abs(Number(japaneseRate.range) - 100 / 60) < 0.00001);
  assert.equal(await evaluate("document.querySelectorAll('[data-fx-tempo-resync]').length"), 0);
  assert.equal(await ux("return u.log.liveWrites"), writes);
  await shot("tempo-sync-ja-tally-1920x1080");
  await ux("u.setBpm(120);u.setLocale('en')"); await settle();
  // During an audition the choice goes to the Preview draft, never to Program.
  await click("[data-fx-audition-begin]"); await settle();
  writes = await ux("return u.log.liveWrites");
  await click(`${syncBox} [data-fx-tempo-unit=bars]`); await settle();
  assert.equal(await ux("return u.log.liveWrites"), writes, "an audition edits only its draft");
  assert.deepEqual(await ux("return u.audition.state().draft.stack[1].controls.find(c=>c.name==='rate').tempo_sync"), { unit: "bars", count: 1 });
  const authoredDraft = await ux("return JSON.stringify(u.audition.state().draft)");
  const previews = await ux("return u.log.previews.length");
  await ux("u.setBpm(240)"); await settle();
  assert.equal(await readout(), "One cycle per 1 bar = 1 Hz at 240 BPM");
  assert.equal(await ux("return JSON.stringify(u.audition.state().draft)"), authoredDraft, "clock changes do not rewrite the Preview draft");
  assert.equal(await ux("return u.log.previews.length"), previews, "clock changes do not republish Preview");
  assert.equal(await ux("return u.log.liveWrites"), writes, "clock changes during audition never reach Program");
  await click("[data-fx-audition-discard]"); await settle();
  // An imported shader with a rate-like Float qualifies; a plain amount does not (model check covers names).

  // ---- Task 18 D2: drag-and-drop reorder of chain stages ----
  await ux("u.resetLayers()");
  const order = (layer = 1) => ux(`const e=u.layers()[${layer}].isf_effect;return [e.label,...e.stack.map(s=>s.label)].join('>')`);
  const drag = (from, to, { drop = true, payload = true } = {}) => evaluate(`(()=>{const a=document.querySelector(${JSON.stringify(from)}),b=document.querySelector(${JSON.stringify(to)});const dt=new DataTransfer();
    const fire=(el,type)=>{const e=new DragEvent(type,{bubbles:true,cancelable:true,dataTransfer:dt});el.dispatchEvent(e);return e};
    if(!${payload}) dt.setData('application/x-syndocal-fx','builtin:invert');
    const start=${payload} ? fire(a,'dragstart') : null;
    const over=fire(b,'dragover'); const edge=b.dataset.fxDrop||null; const dragging=a.dataset.fxDragging||null;
    const dropped=${drop} ? fire(b,'drop') : null; if(start) fire(a,'dragend');
    return {overPrevented:over.defaultPrevented,edge,dragging,dropPrevented:dropped?dropped.defaultPrevented:null,after:b.dataset.fxDrop||null};})()`);
  const rowSel = (index) => `[data-perform-fx-stage="${index}"]`;
  await setView([1920, 1080], "en", "tally", "fx", 700, 620, 2);
  assert.equal(await order(), "Colorize>Monochrome>Strobe>Rotate");
  assert.equal(await evaluate("[...document.querySelectorAll('[data-perform-fx-stage]')].every(e=>e.draggable&&e.dataset.fxDraggable==='true'&&e.title.length>0)"), true, "every stage row is a drag source with a hint");
  assert.equal(await evaluate("[...document.querySelectorAll('.videoPerformQuickFxActions button')].map(e=>e.textContent).slice(0,2).join()"), "Up,Down", "the keyboard buttons stay");
  sets = await ux("return u.log.sets"); let moves = await ux("return u.log.moves"); writes = await ux("return u.log.liveWrites");
  let result = await drag(rowSel(0), rowSel(1));
  assert.deepEqual([result.overPrevented, result.edge, result.dragging, result.dropPrevented, result.after], [true, "after", "true", true, null], "dragover accepts the drop and shows the insertion edge; it clears after the drop");
  await settle();
  assert.equal(await order(), "Monochrome>Colorize>Strobe>Rotate");
  assert.deepEqual([await ux("return u.log.moves"), await ux("return u.log.sets")], [moves + 1, sets], "a neighbour drag is exactly the Up/Down command");
  assert.equal(await evaluate(`document.querySelector(${JSON.stringify(rowSel(1))}).dataset.fxSelected`), "true", "the dragged stage stays selected");
  result = await drag(rowSel(0), rowSel(3)); await settle();
  assert.equal(await order(), "Colorize>Strobe>Rotate>Monochrome", "a longer drag inserts the stage at the drop position");
  assert.deepEqual([await ux("return u.log.moves"), await ux("return u.log.sets"), await ux("return u.log.liveWrites")], [moves + 1, sets + 1, writes + 2], "a longer drag is one write through the ticketed chain command");
  assert.equal(await evaluate(`document.querySelector(${JSON.stringify(rowSel(3))}).dataset.fxSelected`), "true");
  result = await drag(rowSel(3), rowSel(0)); await settle();
  assert.equal(result.edge, "before");
  assert.equal(await order(), "Monochrome>Colorize>Strobe>Rotate");
  writes = await ux("return u.log.liveWrites");
  result = await drag(rowSel(1), rowSel(1)); await settle();
  assert.equal(await ux("return u.log.liveWrites"), writes, "dropping a stage on itself writes nothing");
  assert.equal(result.edge, null);
  result = await drag(rowSel(0), rowSel(2), { drop: false }); await settle();
  assert.equal(await evaluate("document.querySelectorAll('[data-fx-drop]').length"), 0, "a cancelled drag leaves no marker");
  assert.equal(await ux("return u.log.liveWrites"), writes, "a cancelled drag writes nothing");
  // An effect card dragged over the chain is not mistaken for a stage reorder.
  result = await drag(rowSel(0), rowSel(1), { payload: false, drop: false });
  assert.deepEqual([result.overPrevented, result.edge], [false, null], "an effect-card drag is ignored by the reorder");
  // The Up/Down buttons still work.
  await click(`${rowSel(0)} [data-perform-fx-select]`); await settle();
  moves = await ux("return u.log.moves");
  await evaluate("[...document.querySelectorAll('.videoPerformQuickFxActions button')].find(e=>e.textContent==='Down').click()"); await settle();
  assert.equal(await ux("return u.log.moves"), moves + 1);
  assert.equal(await order(), "Colorize>Monochrome>Strobe>Rotate");
  // A single stage has nothing to reorder.
  await setView([1920, 1080], "en", "tally", "fx", 700, 620, 3);
  assert.equal(await evaluate("document.querySelector('[data-perform-fx-stage=\"0\"]').draggable"), false, "one stage is not draggable");
  // During an audition a drag edits the Preview draft only.
  await setView([1920, 1080], "en", "tally", "fx", 700, 620, 2);
  await click("[data-fx-audition-begin]"); await settle();
  writes = await ux("return u.log.liveWrites");
  await ux("u.audition.setEnabled(2,0,false)"); await settle();
  assert.equal(await ux("return u.layers()[1].isf_effect.enabled"), true, "showing an audition draft never writes into the live chain objects");
  await ux("u.audition.setEnabled(2,0,true)"); await settle();
  await drag(rowSel(0), rowSel(2)); await settle();
  assert.equal(await ux("return u.log.liveWrites"), writes, "an audition reorder never reaches Program");
  assert.equal(await ux("const d=u.audition.state().draft;return [d.label,...d.stack.map(s=>s.label)].join('>')"), "Monochrome>Strobe>Colorize>Rotate");
  assert.equal(await order(), "Colorize>Monochrome>Strobe>Rotate", "the live chain is unchanged");
  await click("[data-fx-audition-discard]"); await settle();
  await shot("reorder-en-tally-1920x1080");
  // The Full FX editor shares the behaviour.
  await setView([1920, 1080], "en", "tally", "full", 700, 620, 2);
  await evaluate("document.querySelector('.videoIsfPanel').open=true"); await settle();
  const fullRow = (index) => `[data-video-isf-stage-index="${index}"]`;
  assert.equal(await evaluate("[...document.querySelectorAll('[data-video-isf-stage-index]')].every(e=>e.draggable&&e.title.length>0)"), true);
  assert.equal(await evaluate("document.querySelectorAll('[data-video-isf-action=move-up]').length"), 4, "Up and Down stay in the Full FX editor");
  sets = await ux("return u.log.sets"); moves = await ux("return u.log.moves");
  await drag(fullRow(1), fullRow(2)); await settle();
  assert.deepEqual([await ux("return u.log.moves"), await ux("return u.log.sets"), await order()], [moves + 1, sets, "Colorize>Strobe>Monochrome>Rotate"], "a neighbour drag uses the move command");
  await drag(fullRow(0), fullRow(3)); await settle();
  assert.deepEqual([await ux("return u.log.moves"), await ux("return u.log.sets"), await order()], [moves + 1, sets + 1, "Strobe>Monochrome>Rotate>Colorize"], "a longer drag is one set-effect write");
  await drag(fullRow(3), fullRow(0)); await settle();
  assert.equal(await order(), "Colorize>Strobe>Monochrome>Rotate");
  assert.equal(await evaluate(`document.querySelector(${JSON.stringify(fullRow(0))}).dataset.fxDrop ?? null`), null);
  await ux("u.resetLayers()");

  // ---- Task 18 D3: always-visible category chips ----
  await setView([1366, 768], "en", "tally", "browser", 446, 236);
  const chipRow = "[data-fx-category-chips]";
  assert.equal(await evaluate("document.querySelectorAll('[data-fx-filter-toggle]').length"), 0, "the category drop-down is gone");
  const chipNames = await evaluate(`[...document.querySelectorAll('${chipRow} [data-fx-category]')].map(e=>e.dataset.fxCategory)`);
  assert.equal(chipNames[0], "All"); assert.ok(chipNames.includes("Colour") && chipNames.length >= 3, `chips list All and each category present ${chipNames}`);
  assert.equal(await evaluate(`document.querySelector('${chipRow}').getAttribute('role')`), "group");
  assert.deepEqual(await evaluate(`[...document.querySelectorAll('${chipRow} [aria-pressed=true]')].map(e=>e.dataset.fxCategory)`), ["All"]);
  const narrowChips = await evaluate(`(()=>{const row=document.querySelector('${chipRow}'),frame=document.querySelector('.fxUxFrame').getBoundingClientRect(),first=row.querySelector('[data-fx-category]').getBoundingClientRect(),s=getComputedStyle(row.parentElement),search=document.querySelector('[data-fx-catalog-search]').getBoundingClientRect();return {overflowX:s.overflowX,wrap:getComputedStyle(row).flexWrap,scrollbarWidth:s.scrollbarWidth,sameRow:Math.abs(first.top-search.top)<=8,firstInside:first.left>=frame.left&&first.right<=frame.right&&first.top>=frame.top&&first.bottom<=frame.bottom}})()`);
  assert.deepEqual(narrowChips, { overflowX: "auto", wrap: "nowrap", scrollbarWidth: "none", sameRow: true, firstInside: true }, "narrow panes keep one scrolling row beside the search (no scrollbar height), with the first chip visible");
  await click(`${chipRow} [data-fx-category=Colour]`); await settle();
  assert.deepEqual(await evaluate("[...document.querySelectorAll('[data-fx-catalog-card]')].map(e=>e.dataset.fxCatalogCard).sort()"), ["builtin:colorize", "builtin:invert", "builtin:monochrome"], "a chip filters in one click");
  assert.deepEqual(await evaluate(`[...document.querySelectorAll('${chipRow} [aria-pressed=true]')].map(e=>e.dataset.fxCategory)`), ["Colour"]);
  await click(`${chipRow} [data-fx-category=All]`); await settle();
  assert.ok((await evaluate("document.querySelectorAll('[data-fx-catalog-card]').length")) >= 14);
  // A pane too narrow for every chip scrolls the row sideways, and focus brings the last chip into view.
  await ux("u.setSize({width:300,height:236})"); await settle();
  // The scrollbar is hidden (it would add height), so the wheel scrolls the strip sideways until its end, then the list.
  const wheel = (deltaY) => evaluate(`(()=>{const s=document.querySelector('${chipRow}').parentElement;const before=s.scrollLeft;const e=new WheelEvent('wheel',{deltaY:${deltaY},bubbles:true,cancelable:true});s.dispatchEvent(e);return {prevented:e.defaultPrevented,moved:s.scrollLeft-before}})()`);
  await evaluate(`document.querySelector('${chipRow}').parentElement.scrollLeft=0`);
  let wheeled = await wheel(40);
  assert.deepEqual([wheeled.prevented, wheeled.moved > 0], [true, true], "the wheel scrolls the chip strip sideways");
  wheeled = await wheel(-400);
  assert.deepEqual([wheeled.prevented, wheeled.moved < 0], [true, true], "and back");
  wheeled = await wheel(-40);
  assert.equal(wheeled.prevented, false, "at the start the wheel scrolls the effect list instead");
  await evaluate(`(()=>{const s=document.querySelector('${chipRow}').parentElement;s.scrollLeft=s.scrollWidth})()`);
  assert.equal((await wheel(40)).prevented, false, "and at the end");
  await evaluate(`document.querySelector('${chipRow}').parentElement.scrollLeft=0`);
  await evaluate(`document.querySelector('${chipRow} [data-fx-category]').focus()`);
  for (let step = 1; step < chipNames.length; step++) await key("Tab", "Tab", 9);
  assert.equal(await evaluate("document.activeElement.dataset.fxCategory"), chipNames.at(-1), "Tab reaches the last chip");
  await delay(400);
  // The browser keeps the focused chip reachable by scrolling the row (not the page).
  await evaluate("document.activeElement.scrollIntoView({inline:'nearest'})"); await delay(150);
  const sideways = await evaluate(`(()=>{const strip=document.querySelector('${chipRow}').parentElement;const last=[...strip.querySelectorAll('[data-fx-category]')].at(-1).getBoundingClientRect(),box=strip.getBoundingClientRect();return {scrollable:strip.scrollWidth>strip.clientWidth,scrolled:strip.scrollLeft>0&&last.right<=box.right+1,outer:document.documentElement.scrollWidth<=window.innerWidth}})()`);
  assert.deepEqual(sideways, { scrollable: true, scrolled: true, outer: true }, "the chip row scrolls inside the pane and never the page");
  // Wide panes wrap instead of scrolling; the Japanese labels show.
  await setView([1920, 1080], "ja", "cobalt", "browser", 900, 640);
  const wide = await evaluate(`(()=>{const row=document.querySelector('${chipRow}');return {wrap:getComputedStyle(row).flexWrap,scrollable:row.scrollWidth>row.clientWidth+1||row.parentElement.scrollWidth>row.parentElement.clientWidth+1,labels:[...row.querySelectorAll('[data-fx-category]')].slice(0,2).map(e=>e.textContent)}})()`);
  assert.deepEqual(wide, { wrap: "wrap", scrollable: false, labels: ["すべて", "色"] });
  await evaluate("document.querySelector('[data-effect-browser]').scrollTop=0"); await settle();
  await shot("chips-ja-cobalt-1920x1080");
  await setView([1366, 768], "en", "tally", "browser", 446, 236);
  await evaluate(`document.querySelector('[data-effect-browser]').scrollTop=0;document.querySelector('${chipRow}').parentElement.scrollLeft=0`); await settle();
  await shot("chips-en-tally-1366x768");

  // ---- Target sizes and text size ----
  await setView([1366, 768], "ja", "tally", "browser", 446, 236);
  for (const selector of ["[data-fx-catalog-add]", "[data-fx-catalog-audition]", "[data-fx-catalog-search]", "[data-fx-category]"]) {
    for (const item of await geometry(selector)) { assert.ok(item.h >= 44 && item.w >= 44, `${selector} keeps a 44px target ${JSON.stringify(item)}`); assert.ok(item.font >= 14, `${selector} text is at least 14px`); }
  }
  await setView([1920, 1080], "en", "tally", "fx", 700, 620);
  for (const selector of ["[data-fx-randomize]", "[data-fx-undo]", ".videoFxParameterFoot button", "[data-fx-audition-begin]"]) {
    for (const item of await geometry(selector)) { assert.ok(item.h >= 44 && item.w >= 44, `${selector} keeps a 44px target ${JSON.stringify(item)}`); assert.ok(item.font >= 14, `${selector} text is at least 14px`); }
  }
  // Every visible text node in the new surfaces is at least 14px.
  const smallText = (root) => evaluate(`(()=>{const out=[];for(const host of document.querySelectorAll(${JSON.stringify(root)})){const walker=document.createTreeWalker(host,NodeFilter.SHOW_TEXT);for(let node=walker.nextNode();node;node=walker.nextNode()){if(!node.textContent.trim())continue;const el=node.parentElement;const r=el.getBoundingClientRect();if(!r.width||!r.height)continue;const size=parseFloat(getComputedStyle(el).fontSize);if(size<14)out.push(el.tagName+':'+node.textContent.trim().slice(0,30)+':'+size)}}return out})()`);
  await setView([1920, 1080], "en", "tally", "fx", 700, 620);
  await evaluate("document.querySelector('[data-fx-presets]').open=true");
  assert.deepEqual(await smallText("[data-fx-parameter-editor], .videoFxLiveSafety, [data-fx-audition-bar]"), [], "parameter editor text is at least 14px");
  await click("[data-fx-audition-begin]"); await settle();
  assert.deepEqual(await smallText("[data-fx-audition-bar], .videoPerformQuickFx"), [], "audition text is at least 14px");
  await click("[data-fx-audition-discard]");
  await setView([1920, 1080], "ja", "tally", "browser", 900, 640);
  assert.deepEqual(await smallText("[data-effect-browser]"), [], "browser text is at least 14px");
  assert.deepEqual(errors, [], JSON.stringify(errors));
  console.log(`FX UX browser: parameters (labels, units, ranges, colour picker, reset), presets, Randomize, Preview-only audition with A/B and confirmed Take, rejection cards and problems dialog in EN/JA and two themes; tempo-synced beat/bar units (live BPM, zero clock-driven writes, range limit, Hz capture, audition draft), drag-and-drop stage reorder in the Perform FX tab and the Full FX editor, and always-visible category chips; evidence ${evidenceDir}`);
} finally {
  socket?.close();
  for (const call of pending.values()) clearTimeout(call.timer);
  if (browser && browser.exitCode === null) { const exited = new Promise((done) => browser.once("exit", done)); browser.kill(); await Promise.race([exited, delay(3000)]); }
  await server?.close();
  if (profile) await rm(profile, { recursive: true, force: true, maxRetries: 5, retryDelay: 200 }).catch(() => {});
}
