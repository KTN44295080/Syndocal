import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { basename, dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { createServer } from "vite";

const appRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const evidence = resolve(process.env.SYNDOCAL_SYPHON_QA_DIR ?? join(appRoot, "../target/qa/task100-syphon-input/browser"));
const port = Number(process.env.SYNDOCAL_CHECK_PORT ?? 5398);
const cdpPort = Number(process.env.SYNDOCAL_CDP_PORT ?? 9398);
for (const candidate of [port, cdpPort]) assert.ok(Number.isInteger(candidate) && candidate > 0 && candidate <= 65535);
assert.notEqual(port, cdpPort);
const browserPath = [process.env.CHROME_PATH, "C:/Program Files/Google/Chrome/Application/chrome.exe", "C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe", "C:/Program Files/Microsoft/Edge/Application/msedge.exe"].filter(Boolean).find(existsSync);
assert.ok(browserPath, "Set CHROME_PATH to an installed Chromium browser");
const delay = (ms) => new Promise(done => setTimeout(done, ms));
async function waitFor(check, label, timeout = 30000) {
  const deadline = Date.now() + timeout;
  let lastError;
  while (Date.now() < deadline) {
    try { const result = await check(); if (result) return result; } catch (error) { lastError = error; }
    await delay(75);
  }
  throw new Error("Timed out: " + label + (lastError ? " (" + lastError.message + ")" : ""));
}
class Cdp {
  pending = new Map(); nextId = 0; errors = [];
  async connect(url) {
    this.socket = new WebSocket(url);
    await new Promise((done, reject) => {
      const timer = setTimeout(() => reject(new Error("CDP connect timeout")), 10000);
      this.socket.addEventListener("open", () => { clearTimeout(timer); done(); }, { once: true });
      this.socket.addEventListener("error", () => { clearTimeout(timer); reject(new Error("CDP connection failed")); }, { once: true });
    });
    this.socket.addEventListener("message", ({ data }) => {
      const message = JSON.parse(data);
      if (message.method === "Runtime.exceptionThrown") this.errors.push(message.params.exceptionDetails);
      if (message.method === "Runtime.consoleAPICalled" && ["error", "warning"].includes(message.params.type)) this.errors.push(message.params);
      if (message.method === "Log.entryAdded" && ["error", "warning"].includes(message.params.entry.level)) this.errors.push(message.params.entry);
      const call = this.pending.get(message.id);
      if (!call) return;
      clearTimeout(call.timer); this.pending.delete(message.id);
      if (message.error) call.reject(new Error(call.method + ": " + JSON.stringify(message.error))); else call.done(message.result);
    });
    this.socket.addEventListener("close", () => {
      for (const call of this.pending.values()) { clearTimeout(call.timer); call.reject(new Error("CDP disconnected")); }
      this.pending.clear();
    });
  }
  send(method, params = {}, timeout = 15000) {
    return new Promise((done, reject) => {
      const id = ++this.nextId;
      const timer = setTimeout(() => { this.pending.delete(id); reject(new Error("CDP timeout: " + method)); }, timeout);
      this.pending.set(id, { done, reject, timer, method });
      this.socket.send(JSON.stringify({ id, method, params }));
    });
  }
  async evaluate(expression) {
    const result = await this.send("Runtime.evaluate", { expression, returnByValue: true, awaitPromise: true });
    assert.ok(!result.exceptionDetails, JSON.stringify(result.exceptionDetails));
    return result.result.value;
  }
}
// Browser plugin not available. Reuse the repository's isolated Chromium/CDP
// component-fixture workflow; this never launches the native application.
const fixturePath = "/src/__syphon-input-fixture.tsx";
const fixtureId = join(appRoot, fixturePath.slice(1)).replaceAll("\\", "/");
const fixtureSource = String.raw`
import { createSignal, Show } from "solid-js";
import { render } from "solid-js/web";
import { VideoSourceCreatePanel } from "/src/components/VideoSourceCreatePanel.tsx";
import { installUiLocalization } from "/src/uiLocalization.ts";
import "/src/theme.css";
import "/src/styles.css";
const locale = new URLSearchParams(location.search).get("locale") === "ja" ? "ja" : "en";
document.documentElement.dataset.theme = "tally";
function Fixture() {
  const [kind, setKind] = createSignal<any>("Syphon");
  const [path, setPath] = createSignal("");
  const [mounted, setMounted] = createSignal(true);
  const requests: any[] = []; const added: string[] = [];
  const invokeCommand: any = (command: string) => {
    if (command !== "discover_syphon_sources") throw new Error("Unexpected fixture command: " + command);
    return new Promise((resolve, reject) => requests.push({ resolve, reject, settled: false }));
  };
  const finish = (value: any, reject = false) => {
    const request = requests.find(item => !item.settled);
    if (!request) throw new Error("No pending discovery");
    request.settled = true;
    if (reject) request.reject(new Error("Scripted discovery failure")); else request.resolve(value);
  };
  (window as any).__syphonFixture = { pending: () => requests.filter(item => !item.settled).length,
    resolveNext: (value: any) => finish(value), rejectNext: () => finish(null, true), currentPath: path, added: () => added.slice(), setMounted };
  return <main class="panel controlPanel" style={{ padding: "12px", width: "min(560px, 100%)", "box-sizing": "border-box" }}>
    <Show when={mounted()}><VideoSourceCreatePanel sourceKind={kind()} label="Syphon fixture" path={path()}
      onSetSourceKind={setKind} onSetLabel={() => {}} onSetPath={setPath} onBrowseSource={() => {}}
      onImportMultiple={() => {}} onAddLayer={() => { added.push(path()); }} invokeCommand={invokeCommand} /></Show>
  </main>;
}
render(() => <Fixture />, document.getElementById("fixture-root")!);
installUiLocalization(document.body, () => locale);
`;
const fixturePlugin = {
  name: "syphon-input-ui-fixture", enforce: "pre",
  resolveId(id) { if (id === fixturePath) return fixtureId; },
  load(id) { if (id === fixtureId) return fixtureSource; },
  configureServer(instance) {
    instance.middlewares.use((request, response, next) => {
      if (!request.url?.startsWith("/__syphon-input")) return next();
      response.setHeader("Content-Type", "text/html");
      response.end('<!doctype html><meta name="viewport" content="width=device-width, initial-scale=1"><link rel="icon" href="data:,"><title>Syphon input UI fixture</title><div id="fixture-root"></div><script type="module" src="' + fixturePath + '"></script>');
    });
  },
};
const catalog = [{ name: "RGB corners", app_name: "Simple Server", uuid: "server-a" }, { name: "RGB corners", app_name: "Backup", uuid: "server-b" }];
const endpoint = source => JSON.stringify({ uuid: source.uuid, name: source.name, app_name: source.app_name });
let server; let browser; let cdp; let profile; let currentCase = "startup";
const results = [];
try {
  await mkdir(evidence, { recursive: true });
  const { createServer: createTcpServer } = await import("node:net");
  const reservation = createTcpServer();
  await new Promise((done, reject) => { reservation.once("error", reject); reservation.listen(cdpPort, "127.0.0.1", done); });
  await new Promise((done, reject) => reservation.close(error => error ? reject(error) : done()));
  server = await createServer({ root: appRoot, cacheDir: join(evidence, "vite-cache"), clearScreen: false,
    server: { host: "127.0.0.1", port, strictPort: true }, plugins: [fixturePlugin] });
  await server.listen(); await server.environments.client.depsOptimizer?.scanProcessing; await server.transformRequest(fixturePath);
  profile = await mkdtemp(join(evidence, "browser-profile-"));
  browser = spawn(browserPath, ["--headless=new", "--disable-gpu", "--disable-background-timer-throttling", "--disable-renderer-backgrounding", "--no-first-run", "--no-default-browser-check", "--remote-debugging-port=" + cdpPort, "--user-data-dir=" + profile, "about:blank"], { windowsHide: true, stdio: "ignore" });
  const target = await waitFor(async () => {
    const targets = await (await fetch("http://127.0.0.1:" + cdpPort + "/json/list")).json();
    return targets.find(candidate => candidate.type === "page");
  }, "owned browser page");
  cdp = new Cdp(); await cdp.connect(target.webSocketDebuggerUrl); await cdp.send("Page.enable"); await cdp.send("Runtime.enable"); await cdp.send("Log.enable");
  const fx = (name, ...args) => cdp.evaluate("window.__syphonFixture[" + JSON.stringify(name) + "](..." + JSON.stringify(args) + ")");
  const click = selector => cdp.evaluate("document.querySelector(" + JSON.stringify(selector) + ").click()");
  const state = () => cdp.evaluate("document.querySelector('[data-syphon-discovery-state]')?.dataset.syphonDiscoveryState");
  const addDisabled = () => cdp.evaluate("document.querySelector('[data-video-add-layer]').disabled");
  const refresh = async () => { await click("[data-video-syphon-refresh]"); await waitFor(() => fx("pending"), "pending discovery"); };
  const resolveCatalog = async value => { await fx("resolveNext", value); await waitFor(async () => (await state()) !== "loading", "discovery completed"); };
  const select = value => cdp.evaluate("(() => { const e = document.querySelector('[data-video-syphon-select]'); e.value = " + JSON.stringify(value) + "; e.dispatchEvent(new Event('change', { bubbles: true })); })()");
  for (const locale of ["en", "ja"]) for (const width of [1920, 960]) {
    currentCase = locale + "-" + width + "x1080";
    const url = "http://127.0.0.1:" + port + "/__syphon-input?locale=" + locale;
    await cdp.send("Emulation.setDeviceMetricsOverride", { width, height: 1080, deviceScaleFactor: 1, mobile: false });
    await cdp.send("Page.navigate", { url });
    await waitFor(() => cdp.evaluate("location.href === " + JSON.stringify(url) + " && document.title === 'Syphon input UI fixture' && !!window.__syphonFixture"), currentCase + " mounted");
    await waitFor(async () => await fx("pending") === 1, "initial discovery");
    assert.equal(await addDisabled(), true);
    await resolveCatalog(catalog); assert.equal(await state(), "available");
    assert.equal(await fx("currentPath"), "", "no automatic first server");
    assert.equal(await addDisabled(), true);
    assert.deepEqual(await cdp.evaluate("[...document.querySelectorAll('[data-video-syphon-select] option')].slice(1).map(e => e.textContent)"), ["Simple Server / RGB corners", "Backup / RGB corners"], "sender names survive localization");
    await select(endpoint(catalog[0])); await waitFor(async () => !(await addDisabled()), "explicit server admitted");
    await click("[data-video-add-layer]"); assert.deepEqual(await fx("added"), [endpoint(catalog[0])]);
    await refresh(); assert.equal(await addDisabled(), true); assert.equal(await fx("currentPath"), "");
    await fx("rejectNext"); await waitFor(async () => await state() === "error", "failed refresh");
    assert.equal(await addDisabled(), true);
    await refresh(); await resolveCatalog([]); assert.equal(await addDisabled(), true);
    const emptyText = locale === "ja" ? "Syphonサーバーが見つかりません" : "No Syphon servers found";
    await waitFor(() => cdp.evaluate("document.querySelector('[data-syphon-discovery-state]').textContent.includes(" + JSON.stringify(emptyText) + ")"), "localized empty state");
    await refresh(); await resolveCatalog([catalog[0], catalog[0]]); assert.equal(await state(), "error");
    await refresh(); await fx("setMounted", false); await fx("setMounted", true);
    await waitFor(async () => await fx("pending") === 2, "remounted request");
    await fx("resolveNext", catalog); assert.equal(await state(), "loading", "disposed response ignored");
    await resolveCatalog(catalog); await select(endpoint(catalog[1])); await waitFor(async () => !(await addDisabled()), "new server selection");
    const layout = await cdp.evaluate("(() => { const p = document.querySelector('[data-video-syphon-source]'); return { overflow: p.scrollWidth-p.clientWidth, smallControls: [...p.querySelectorAll('button,select')].filter(e => e.getBoundingClientRect().height < 44 || parseFloat(getComputedStyle(e).fontSize) < 14).length, blank: document.body.innerText.trim().length < 30, overlay: !!document.querySelector('vite-error-overlay') }; })()");
    assert.ok(layout.overflow <= 1); assert.equal(layout.smallControls, 0); assert.equal(layout.blank, false); assert.equal(layout.overlay, false);
    assert.deepEqual(cdp.errors, [], "no browser exceptions");
    const shot = await cdp.send("Page.captureScreenshot", { format: "png" }, 30000);
    await writeFile(join(evidence, "picker-" + currentCase + ".png"), Buffer.from(shot.data, "base64"));
    results.push({ case: currentCase, status: "pass", checks: "exact selection; refresh invalidation; failure; empty; duplicate rejection; disposed reply; JA/EN; target size and containment" });
  }
  console.log(JSON.stringify({ status: "pass", results, boundary: "real Solid component with scripted discovery; no native or physical Syphon proof" }, null, 2));
} catch (error) {
  await writeFile(join(evidence, "failure.json"), JSON.stringify({ currentCase, error: error.stack, results, runtimeErrors: cdp?.errors }, null, 2)); throw error;
} finally {
  if (cdp?.socket?.readyState === WebSocket.OPEN) { await cdp.send("Browser.close").catch(() => {}); cdp.socket.close(); }
  if (browser && browser.exitCode === null && browser.signalCode === null) {
    try { await waitFor(() => browser.exitCode !== null || browser.signalCode !== null, "owned browser exit", 5000); } catch { browser.kill(); }
  }
  if (server) await server.close();
  if (profile) { assert.equal(dirname(resolve(profile)), evidence); assert.ok(basename(profile).startsWith("browser-profile-")); await rm(profile, { recursive: true, force: true, maxRetries: 5, retryDelay: 200 }); }
  await writeFile(join(evidence, "result.json"), JSON.stringify({ status: results.length === 4 ? "pass" : "fail", results, cleanup: "owned browser/Vite closed; owned profile removed" }, null, 2));
}
