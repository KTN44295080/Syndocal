import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { basename, dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { createServer } from "vite";

const appRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const evidence = resolve(process.env.SYNDOCAL_NDI_QA_DIR ?? join(appRoot, "../target/qa/ndi-input/browser"));
const port = Number(process.env.SYNDOCAL_CHECK_PORT ?? 5390);
const cdpPort = Number(process.env.SYNDOCAL_CDP_PORT ?? 9390);
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
const fixturePath = "/src/__ndi-input-fixture.tsx";
const fixtureId = join(appRoot, fixturePath.slice(1)).replaceAll("\\", "/");
const fixtureSource = String.raw`
import { createMemo, createSignal, Show } from "solid-js";
import { render } from "solid-js/web";
import { VideoSourceCreatePanel } from "/src/components/VideoSourceCreatePanel.tsx";
import { MediaLibraryHealthDialog } from "/src/components/MediaHealthDialog.tsx";
import { buildNdiInputHealthRows } from "/src/ndiInputHealth.ts";
import { installUiLocalization, translateUiText } from "/src/uiLocalization.ts";
import "/src/theme.css";
import "/src/styles.css";
const locale = new URLSearchParams(location.search).get("locale") === "ja" ? "ja" : "en";
document.documentElement.dataset.theme = "tally";
function Fixture() {
  const [kind, setKind] = createSignal<any>("Ndi");
  const [path, setPath] = createSignal("");
  const [mounted, setMounted] = createSignal(true);
  const layer: any = { id: 7, label: "収録カメラ / Stage camera", source: { kind: "Ndi", name: "Stage NDI" }, state: { enabled: true } };
  const source = { route_id: 7, identity: "v1:3:ndi:9:Stage NDI", generation: 1, state: "Live", issue: null };
  const route = { direction: "Input", route_id: 7, label: layer.label, backend_id: "ndi", endpoint_name: "Stage NDI" };
  const initial = { active_routes: [route], active_count: 1, live_sources: [source], live_source_faults: [], capture_faults: [], ownership_allowed: false };
  const [status, setStatus] = createSignal<any>(initial);
  const ndiInputs = createMemo(() => buildNdiInputHealthRows([layer], status()));
  const requests: any[] = [];
  const added: string[] = [];
  let dialog!: HTMLDialogElement;
  const invokeCommand: any = (command: string) => {
    if (command !== "list_ndi_sources_v1") throw new Error("Unexpected fixture command: " + command);
    return new Promise((resolve, reject) => requests.push({ resolve, reject, settled: false }));
  };
  const finish = (value: any, reject = false) => {
    const request = requests.find(item => !item.settled);
    if (!request) throw new Error("No pending discovery");
    request.settled = true;
    if (reject) request.reject(new Error("Scripted discovery failure")); else request.resolve(value);
  };
  (window as any).__ndiFixture = {
    pending: () => requests.filter(item => !item.settled).length,
    resolveNext: (value: any) => finish(value), rejectNext: () => finish(null, true),
    currentPath: path, added: () => added.slice(), setMounted,
    translate: (text: string) => translateUiText(text, locale),
    health: (mode: string) => {
      if (mode === "unavailable") { setStatus(null); return; }
      const next: any = structuredClone(initial);
      if (mode === "no_signal" || mode === "lost") next.live_source_faults = [{ route_id: 7, backend_id: "ndi", label: layer.label,
        message: mode === "lost" ? "NDI input signal lost for 'Stage NDI'" : "NDI input has no signal for 'Stage NDI'" }];
      setStatus(next);
    },
    refreshHealth: () => setStatus(structuredClone(status())),
  };
  return <main class="panel controlPanel" style={{ padding: "12px", width: "min(560px, 100%)", "box-sizing": "border-box" }}>
    <Show when={mounted()}>
      <VideoSourceCreatePanel sourceKind={kind()} label="NDI fixture" path={path()} onSetSourceKind={setKind} onSetLabel={() => {}}
        onSetPath={setPath} onBrowseSource={() => {}} onImportMultiple={() => {}} onAddLayer={() => { added.push(path()); }} invokeCommand={invokeCommand} />
    </Show>
    <button type="button" data-fixture-open-health onClick={() => dialog.showModal()}>Media health…</button>
    <MediaLibraryHealthDialog library={{ assets: [], availabilityById: {}, get ndiInputs() { return ndiInputs(); }, backendAvailable: true,
      activeOperations: [], onVerify: () => {}, onRelink: () => {} }} ref={element => { dialog = element; }} />
  </main>;
}
render(() => <Fixture />, document.getElementById("fixture-root")!);
installUiLocalization(document.body, () => locale);
`;

const fixturePlugin = {
  name: "ndi-input-ui-fixture", enforce: "pre",
  resolveId(id) { if (id === fixturePath) return fixtureId; },
  load(id) { if (id === fixtureId) return fixtureSource; },
  configureServer(instance) {
    instance.middlewares.use((request, response, next) => {
      if (!request.url?.startsWith("/__ndi-input")) return next();
      response.setHeader("Content-Type", "text/html");
      response.end('<!doctype html><meta name="viewport" content="width=device-width, initial-scale=1"><title>NDI input UI fixture</title><div id="fixture-root"></div><script type="module" src="' + fixturePath + '"></script>');
    });
  },
};
const catalog = { state: "available", sources: [{ name: "Camera A", endpoint_name: "Studio (Camera A)" }, { name: "Camera B", endpoint_name: "Backup (Camera B)" }], message: null };
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
  await server.listen();
  await server.environments.client.depsOptimizer?.scanProcessing;
  await server.transformRequest(fixturePath);
  profile = await mkdtemp(join(evidence, "browser-profile-"));
  browser = spawn(browserPath, ["--headless=new", "--disable-gpu", "--disable-background-timer-throttling", "--disable-renderer-backgrounding", "--no-first-run", "--no-default-browser-check", "--remote-debugging-port=" + cdpPort, "--user-data-dir=" + profile, "about:blank"], { windowsHide: true, stdio: "ignore" });
  let spawnError;
  browser.on("error", error => { spawnError = error; });
  const target = await waitFor(async () => {
    if (spawnError) throw spawnError;
    if (browser.exitCode !== null) throw new Error("Owned browser exited before CDP was ready");
    const targets = await (await fetch("http://127.0.0.1:" + cdpPort + "/json/list")).json();
    return targets.find(candidate => candidate.type === "page");
  }, "owned browser page");
  cdp = new Cdp();
  await cdp.connect(target.webSocketDebuggerUrl);
  await cdp.send("Page.enable"); await cdp.send("Runtime.enable");
  const fx = (name, ...args) => cdp.evaluate("window.__ndiFixture[" + JSON.stringify(name) + "](..." + JSON.stringify(args) + ")");
  const click = selector => cdp.evaluate("document.querySelector(" + JSON.stringify(selector) + ").click()");
  const discoveryState = () => cdp.evaluate("document.querySelector('[data-ndi-discovery-state]')?.dataset.ndiDiscoveryState");
  const addDisabled = () => cdp.evaluate("document.querySelector('[data-video-add-layer]').disabled");
  const healthState = () => cdp.evaluate("document.querySelector('[data-media-health-ndi-row]')?.dataset.ndiState");
  const refresh = async () => { await click("[data-video-ndi-refresh]"); await waitFor(() => fx("pending"), "pending discovery"); };
  const resolveCatalog = async value => { await fx("resolveNext", value); await waitFor(async () => (await discoveryState()) !== "loading", "discovery completed"); };
  const select = endpoint => cdp.evaluate("(() => { const element = document.querySelector('[data-video-ndi-select]'); element.value = " + JSON.stringify(endpoint) + "; element.dispatchEvent(new Event('change', { bubbles: true })); })()");
  const capture = async filename => {
    const shot = await cdp.send("Page.captureScreenshot", { format: "png" }, 30000);
    await writeFile(join(evidence, filename), Buffer.from(shot.data, "base64"));
  };
  for (const [locale, width, height] of [["en", 1280, 720], ["ja", 390, 780]]) {
    currentCase = locale + "-" + width + "x" + height;
    const url = "http://127.0.0.1:" + port + "/__ndi-input?locale=" + locale;
    await cdp.send("Emulation.setDeviceMetricsOverride", { width, height, deviceScaleFactor: 1, mobile: false });
    await cdp.send("Page.navigate", { url });
    await waitFor(() => cdp.evaluate("location.href === " + JSON.stringify(url) + " && document.readyState === 'complete' && !!window.__ndiFixture"), currentCase + " mounted");
    await waitFor(async () => await fx("pending") === 1, "initial discovery started");
    assert.equal(await addDisabled(), true);
    await resolveCatalog(catalog);
    assert.equal(await discoveryState(), "available");
    assert.equal(await fx("currentPath"), "", "catalog does not auto-select the first source");
    assert.equal(await addDisabled(), true);
    assert.deepEqual(await cdp.evaluate("[...document.querySelectorAll('[data-video-ndi-select] option')].slice(1).map(option => option.textContent)"), ["Camera A", "Camera B"], "sender names survive JA localization");
    await select("Studio (Camera A)");
    await waitFor(async () => !(await addDisabled()), "explicit source admission");
    await click("[data-video-add-layer]");
    assert.deepEqual(await fx("added"), ["Studio (Camera A)"], "Add passes the opaque endpoint");
    await refresh();
    assert.equal(await addDisabled(), true);
    assert.equal(await fx("currentPath"), "");
    await fx("rejectNext");
    await waitFor(async () => await discoveryState() === "error", "failed refresh visible");
    assert.equal(await addDisabled(), true);
    assert.equal(await cdp.evaluate("document.querySelectorAll('[data-video-ndi-select] option').length"), 1);
    await refresh();
    await resolveCatalog({ state: "runtime_missing", sources: [], message: null });
    assert.equal(await discoveryState(), "runtime_missing");
    assert.equal(await addDisabled(), true);
    const missingText = locale === "ja" ? "NDIランタイムがインストールされていません" : "NDI runtime not installed";
    await waitFor(() => cdp.evaluate("document.querySelector('[data-ndi-discovery-state]').textContent.includes(" + JSON.stringify(missingText) + ")"), "localized missing-runtime state");
    await refresh();
    await resolveCatalog({ ...catalog, sources: [catalog.sources[0], catalog.sources[0]] });
    assert.equal(await discoveryState(), "error", "ambiguous identities reject the complete catalog");
    assert.equal(await addDisabled(), true);
    await refresh();
    await fx("setMounted", false);
    assert.equal(await cdp.evaluate("document.querySelectorAll('[data-video-ndi-source]').length"), 0);
    await fx("setMounted", true);
    await waitFor(async () => await fx("pending") === 2, "remounted request started");
    await fx("resolveNext", catalog);
    assert.equal(await discoveryState(), "loading", "disposed response cannot populate the new picker");
    assert.equal(await addDisabled(), true);
    assert.equal(await fx("currentPath"), "");
    await resolveCatalog(catalog);
    assert.equal(await addDisabled(), true);
    await select("Backup (Camera B)");
    await waitFor(async () => !(await addDisabled()), "new endpoint selected");
    assert.deepEqual(await fx("added"), ["Studio (Camera A)"], "discovery never adds a layer itself");
    const layout = await cdp.evaluate("(() => { const panel = document.querySelector('[data-video-ndi-source]'); return { overflow: panel.scrollWidth - panel.clientWidth, badControls: [...panel.querySelectorAll('button,select')].filter(element => element.getBoundingClientRect().height < 44 || parseFloat(getComputedStyle(element).fontSize) < 14).length, smallBody: [...panel.querySelectorAll('.fieldHint')].filter(element => parseFloat(getComputedStyle(element).fontSize) < 13).length }; })()");
    assert.ok(layout.overflow <= 1, currentCase + ": picker overflows");
    assert.equal(layout.badControls, 0, "44px targets and 14px labels");
    assert.equal(layout.smallBody, 0, "body text at least 13px");
    await capture("picker-" + currentCase + ".png");
    await click("[data-fixture-open-health]");
    await waitFor(() => cdp.evaluate("document.querySelector('[data-media-health-dialog]').open"), "health dialog open");
    for (const [state, english] of [["connected", "Connected"], ["no_signal", "No signal"], ["lost", "Signal lost"], ["unavailable", "Unavailable"]]) {
      await fx("health", state);
      await waitFor(async () => await healthState() === state, "health " + state);
      const expected = await fx("translate", english);
      if (locale === "ja") assert.notEqual(expected, english, "Japanese badge copy");
      await waitFor(() => cdp.evaluate("document.querySelector('[data-ndi-health-label]').textContent.trim() === " + JSON.stringify(expected)), "translated health badge");
    }
    await fx("health", "lost");
    await waitFor(async () => await healthState() === "lost", "lost diagnostic available");
    await cdp.evaluate("(() => { const summary = document.querySelector('[data-media-health-ndi-row] details summary'); summary.click(); summary.focus(); })()");
    await fx("refreshHealth");
    assert.equal(await cdp.evaluate("document.querySelector('[data-media-health-ndi-row] details').open"), true, "polling preserves open diagnostics");
    assert.equal(await cdp.evaluate("document.activeElement === document.querySelector('[data-media-health-ndi-row] details summary')"), true, "polling preserves keyboard focus");
    assert.equal(await cdp.evaluate("document.querySelectorAll('[data-media-health-row]').length"), 0, "NDI is not a missing local file");
    assert.equal(await cdp.evaluate("document.querySelector('[data-media-health-verify-all]').disabled"), true, "NDI never enters file verification");
    for (const theme of ["cobalt", "ember", "forest", "graphite", "ice", "neon", "safe", "tally"]) {
      await cdp.evaluate("document.documentElement.dataset.theme = " + JSON.stringify(theme));
      const overflow = await cdp.evaluate("(() => { const dialog = document.querySelector('[data-media-health-dialog]'); return dialog.scrollWidth - dialog.clientWidth; })()");
      assert.ok(overflow <= 1, currentCase + "/" + theme + ": health overflows");
    }
    await capture("health-" + currentCase + ".png");
    await click("[data-media-health-close]");
    assert.deepEqual(cdp.errors, [], "no browser exceptions");
    results.push({ case: currentCase, status: "pass", checks: "exact selection, refresh/failure, missing-runtime, late result, health, poll focus, JA/EN, eight themes" });
  }
  console.log(JSON.stringify({ status: "pass", results, boundary: "real Solid components with scripted discovery/health; no native app or LAN proof" }, null, 2));
} catch (error) {
  await writeFile(join(evidence, "failure.json"), JSON.stringify({ currentCase, error: error.stack, results, runtimeErrors: cdp?.errors }, null, 2));
  throw error;
} finally {
  if (cdp?.socket?.readyState === WebSocket.OPEN) { await cdp.send("Browser.close").catch(() => {}); cdp.socket.close(); }
  if (browser && browser.exitCode === null && browser.signalCode === null) {
    try { await waitFor(() => browser.exitCode !== null || browser.signalCode !== null, "owned browser exit", 5000); }
    catch { browser.kill(); }
  }
  if (server) await server.close();
  if (profile) {
    assert.equal(dirname(resolve(profile)), evidence);
    assert.ok(basename(profile).startsWith("browser-profile-"));
    await rm(profile, { recursive: true, force: true, maxRetries: 5, retryDelay: 200 });
  }
  await writeFile(join(evidence, "result.json"), JSON.stringify({ status: results.length === 2 ? "pass" : "fail", results, cleanup: "owned browser/Vite closed; owned profile removed" }, null, 2));
}
