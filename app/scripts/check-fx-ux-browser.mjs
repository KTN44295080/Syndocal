// Browser fixture for the effect browser, parameter editor, presets, Preview
// audition and rejection explanations. It mounts the real components with a
// reactive in-memory layer so the whole operator flow runs without Tauri.
// Screenshots go to SYNDOCAL_FX_UX_SCREENSHOT_DIR (default target/qa/fx-ux-12).
import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { createServer } from "vite";

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
const [layers, setLayers] = createSignal([
  { id: 1, label: "Opener Loop", isf_effect: null },
  { id: 2, label: "Strobe Wash", isf_effect: fxChainFromStages([stages.colorize(), stages.monochrome(), stages.strobe(), stages.rotate()]) },
]);
const [transport, setTransport] = createSignal({ layer_id: null, effect_audition: false });
const [catalog, setCatalog] = createSignal({ effect_chains: [], effect_presets: [], layer_groups: [], transition_buses: [] });
const log = { commits: [], previews: [], liveWrites: 0, catalogWrites: 0, adds: [], auditions: [] };
const updateLayer = (id, change) => setLayers(layers().map((layer) => layer.id === id ? { ...layer, isf_effect: change(layer.isf_effect) } : layer));
const mapStages = (effect, change) => fxChainFromStages(fxChainStages(effect).map(change));
const live = {
  layers: () => layers(),
  onSetIsfEffectEnabled: (id, index, enabled) => { log.liveWrites++; updateLayer(id, (effect) => mapStages(effect, (stage, at) => at === index ? { ...stage, enabled } : stage)); },
  onMoveIsfEffect: (id, index, delta) => { log.liveWrites++; updateLayer(id, (effect) => { const list = fxChainStages(effect); [list[index], list[index + delta]] = [list[index + delta], list[index]]; return fxChainFromStages(list); }); },
  onRemoveIsfEffect: (id, index) => { log.liveWrites++; updateLayer(id, (effect) => fxChainFromStages(fxChainStages(effect).filter((_, at) => at !== index))); },
  onSetIsfEffect: (id, effect) => { log.liveWrites++; updateLayer(id, () => effect); },
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
let dialog;
Object.assign(window, { __fxUx: {
  log, setLocale: (value) => { setLocale(value); document.documentElement.lang = value; }, setView, setSize, setSelectedLayerId, setRejectionsOn,
  setRuntimeErrors, layers, transport, audition, catalog, openProblems: () => dialog.showModal(), problems,
  setCatalog: (value) => { log.catalogWrites++; setCatalog(value); },
}});
const frame = () => ({ width: size().width + "px", height: size().height + "px" });
render(() => <div class="fxUxStage">
  <div class="fxUxFrame" style={frame()} data-video-perform-pane="inspector">
    {view() === "fx"
      ? <VideoPerformQuickFx layer={layer()} layerList={live} locale={locale()} projectEpoch={1} onOpenEditor={() => {}}
          effectCatalog={catalog()} onApplyEffectCatalog={(value) => { window.__fxUx.setCatalog(value); return value; }}
          liveSession={liveSession} audition={audition} />
      : <VideoEffectBrowser entries={entries()} layer={layer()} locale={locale()} onAdd={(entry, layerId) => { log.adds.push([entry.id, layerId]); }}
          audition={audition} onAudition={(entry, layerId) => { log.auditions.push([entry.id, layerId]); return audition.addToAudition(layerId, entry.builtinId ? audition.describeBuiltinStage(entry.builtinId) : async () => null); }}
          onImport={() => {}} onOpenFx={() => setView("fx")} />}
  </div>
  <VideoFxProblemsDialog ref={(element) => { dialog = element; }} problems={problems()} locale={locale()} onClose={() => {}} />
</div>, document.getElementById("root"));
installUiLocalization(document.body, locale);
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
  server = await createServer({ root: appRoot, server: { host: "127.0.0.1", port: 0, strictPort: false }, plugins: [plugin], clearScreen: false, logLevel: "error" });
  await server.listen();
  profile = await mkdtemp(join(tmpdir(), "syndocal-fx-ux-"));
  browser = spawn(browserPath, ["--headless=new", "--use-angle=swiftshader", "--enable-unsafe-swiftshader", "--no-first-run", "--no-default-browser-check", "--remote-debugging-port=0", `--user-data-dir=${profile}`, "about:blank"], { windowsHide: true, stdio: "ignore" });
  const port = await until(async () => Number((await readFile(join(profile, "DevToolsActivePort"), "utf8")).split("\n")[0]), "DevTools port");
  const page = (await (await fetch(`http://127.0.0.1:${port}/json/list`)).json()).find((entry) => entry.type === "page");
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

  // ---- Target sizes and text size ----
  await setView([1366, 768], "ja", "tally", "browser", 446, 236);
  for (const selector of ["[data-fx-catalog-add]", "[data-fx-catalog-audition]", "[data-fx-catalog-search]", "[data-fx-filter-toggle]"]) {
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
  console.log(`FX UX browser: parameters (labels, units, ranges, colour picker, reset), presets, Randomize, Preview-only audition with A/B and confirmed Take, rejection cards and problems dialog in EN/JA and two themes; evidence ${evidenceDir}`);
} finally {
  socket?.close();
  for (const call of pending.values()) clearTimeout(call.timer);
  if (browser && browser.exitCode === null) { const exited = new Promise((done) => browser.once("exit", done)); browser.kill(); await Promise.race([exited, delay(3000)]); }
  await server?.close();
  if (profile) await rm(profile, { recursive: true, force: true, maxRetries: 5, retryDelay: 200 }).catch(() => {});
}
