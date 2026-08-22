import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { mkdtemp, rm } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { tmpdir } from "node:os";
import { fileURLToPath } from "node:url";

const appRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const host = "127.0.0.1";
const vitePort = 5194;
const cdpPort = 9244;
const baseUrl = `http://${host}:${vitePort}/?syndocalViewportFixture=timeline-layered`;
const browserPath = [
  process.env.CHROME_PATH,
  "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe",
  "C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe",
  "C:\\Program Files\\Microsoft\\Edge\\Application\\msedge.exe",
].filter(Boolean).find((candidate) => existsSync(candidate));
const sleep = (ms) => new Promise((resolveSleep) => setTimeout(resolveSleep, ms));
const waitFor = async (check, label, timeoutMs = 30_000) => {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    try {
      if (await check()) return;
    } catch {
      // The Vite and CDP listeners may not have bound their ports yet.
    }
    await sleep(100);
  }
  throw new Error(`Timed out waiting for ${label}`);
};
const stopChild = async (child) => {
  if (!child || child.exitCode !== null) return;
  const exited = new Promise((resolveExit) => child.once("exit", resolveExit));
  child.kill();
  await Promise.race([exited, sleep(2_000)]);
};

class CdpClient {
  constructor(url) {
    this.socket = new WebSocket(url);
    this.nextId = 0;
    this.pending = new Map();
    this.socket.onmessage = (event) => {
      const message = JSON.parse(event.data);
      const pending = this.pending.get(message.id);
      if (!pending) return;
      this.pending.delete(message.id);
      if (message.error) pending.reject(new Error(message.error.message));
      else pending.resolve(message.result);
    };
  }
  async ready() {
    if (this.socket.readyState === WebSocket.OPEN) return;
    await new Promise((resolveReady, reject) => {
      this.socket.onopen = resolveReady;
      this.socket.onerror = reject;
    });
  }
  send(method, params = {}) {
    const id = ++this.nextId;
    return new Promise((resolveSend, reject) => {
      this.pending.set(id, { resolve: resolveSend, reject });
      this.socket.send(JSON.stringify({ id, method, params }));
    });
  }
  close() { this.socket.close(); }
}

const evaluate = async (client, expression) => {
  const result = await client.send("Runtime.evaluate", { expression, awaitPromise: true, returnByValue: true });
  if (result.exceptionDetails) throw new Error(result.exceptionDetails.text);
  return result.result.value;
};
const click = (client, selector) => evaluate(client, `(() => {
  const element = document.querySelector(${JSON.stringify(selector)});
  if (!(element instanceof HTMLElement)) return false;
  element.click();
  return true;
})()`);

let vite;
let browser;
let client;
let profileDir;
try {
  assert.ok(browserPath, "Chrome or Edge is required for the Cue Audio browser gate");
  vite = spawn(process.execPath, [resolve(appRoot, "node_modules/vite/bin/vite.js"), "--host", host, "--port", String(vitePort), "--strictPort"], { cwd: appRoot, stdio: "ignore" });
  await waitFor(async () => (await fetch(baseUrl)).ok, "Vite fixture server");
  profileDir = await mkdtemp(join(tmpdir(), "syndocal-timeline-cue-audio-"));
  browser = spawn(browserPath, ["--headless=new", "--disable-gpu", "--no-first-run", "--no-default-browser-check", `--remote-debugging-port=${cdpPort}`, `--user-data-dir=${profileDir}`, "about:blank"], { stdio: "ignore" });
  const target = await (async () => {
    let created = null;
    await waitFor(async () => {
      const response = await fetch(`http://${host}:${cdpPort}/json/new?${encodeURIComponent(baseUrl)}`, { method: "PUT" });
      if (!response.ok) return false;
      created = await response.json();
      return true;
    }, "browser target");
    return created;
  })();
  client = new CdpClient(target.webSocketDebuggerUrl);
  await client.ready();
  await client.send("Page.enable");
  await client.send("Runtime.enable");
  await client.send("Emulation.setDeviceMetricsOverride", { width: 1280, height: 720, deviceScaleFactor: 1, mobile: false });
  await waitFor(() => evaluate(client, "document.querySelector('.app') && document.readyState === 'complete'"), "app mount");
  assert.equal(await click(client, '[data-workspace-option="control"]'), true);
  assert.equal(await click(client, '[data-edit-domain-navigation] [data-control-mode-option="live"]'), true);
  await waitFor(() => click(client, '[data-timeline-desk-surface="show"]'), "Timeline Show tab");
  assert.equal(await click(client, ".timelineToolsDisclosure > summary"), true);
  await waitFor(() => evaluate(client, "document.querySelector('.timelineToolsDisclosure[open] .timelinePerformanceEditor') !== null"), "Timeline performance disclosure");
  await evaluate(client, `(() => {
    const clone = (value) => structuredClone(value);
    const mock = {
      calls: [],
      endpoints: [],
      settings: { version: 1, route: 'follow_program', device_name: null, topology_fingerprint: null, click_gain: 1, guide_gain: 0.85 },
      statusRevision: 0,
    };
    const status = () => ({
      runtimeIncarnation: 41,
      statusRevision: ++mock.statusRevision,
      desiredSettings: clone(mock.settings),
      appliedSettings: clone(mock.settings),
      settingsRevision: 7,
      lifecycle: 'running',
      requestedDeviceName: mock.settings.device_name,
      resolvedDeviceName: mock.settings.route === 'explicit_device' ? mock.settings.device_name : 'Program Output',
      requestedTopologyFingerprint: mock.settings.topology_fingerprint,
      observedTopologyFingerprint: mock.endpoints.length ? 'topology-fingerprint-a' : null,
      topologyGeneration: mock.endpoints.length ? 1 : 0,
      endpoints: clone(mock.endpoints),
      outputClockEpoch: 2,
      scheduleGeneration: 3,
      sourceFence: 4,
      nextOutputFrame: 5,
      callbackLive: true,
      faultCode: 'None',
      faultCount: 0,
      faultSequence: 0,
      lastError: null,
      rotationCount: 1,
      stallCount: 0,
      configCount: 1,
    });
    window.__syndocalCueAudioMock = mock;
    window.__TAURI_INTERNALS__ = {
      invoke: async (command, args = {}) => {
        mock.calls.push({ command, args: clone(args) });
        if (command === 'list_audio_output_devices') {
          mock.endpoints = [
            { name: 'Duplicate Program', occurrences: 2, selectable: false },
            { name: 'Exact Program', occurrences: 1, selectable: true },
          ];
          return ['Duplicate Program', 'Duplicate Program', 'Exact Program'];
        }
        if (command === 'get_timeline_cue_audio_status') return status();
        if (command === 'set_machine_timeline_cue_audio_settings') {
          mock.settings = clone(args.settings);
          return status();
        }
        throw new Error('Unexpected Cue Audio invoke: ' + command);
      },
    };
  })()`);
  assert.equal(await click(client, ".timelineCueAudioEditor > summary"), true);
  assert.equal(await evaluate(client, `(() => {
    const button = [...document.querySelectorAll('.timelineCueAudioEditor button')]
      .find((candidate) => candidate.textContent?.trim() === 'Refresh outputs');
    if (!(button instanceof HTMLButtonElement)) return false;
    button.click();
    return true;
  })()`), true);
  await waitFor(() => evaluate(client, "window.__syndocalCueAudioMock.calls.map((call) => call.command).join(',') === 'list_audio_output_devices,get_timeline_cue_audio_status'"), "manual list then Cue Audio status");
  assert.equal(await evaluate(client, `(() => {
    const route = document.querySelector('.timelineCueAudioEditor select');
    if (!(route instanceof HTMLSelectElement)) return false;
    route.value = 'explicit_device';
    route.dispatchEvent(new Event('change', { bubbles: true }));
    return true;
  })()`), true);
  const pendingProof = await evaluate(client, `(() => {
    const output = document.querySelector('[data-timeline-cue-audio-output]');
    if (!(output instanceof HTMLSelectElement)) return null;
    const duplicate = [...output.options].find((option) => option.value === 'Duplicate Program');
    return {
      value: output.value,
      duplicateDisabled: duplicate?.disabled === true,
      calls: structuredClone(window.__syndocalCueAudioMock.calls),
    };
  })()`);
  assert.deepEqual(pendingProof, {
    value: '',
    duplicateDisabled: true,
    calls: [
      { command: 'list_audio_output_devices', args: {} },
      { command: 'get_timeline_cue_audio_status', args: {} },
    ],
  }, "Explicit route is a pending local form: it never auto-selects a duplicate or mutates Follow Program");
  assert.equal(await evaluate(client, `(() => {
    const output = document.querySelector('[data-timeline-cue-audio-output]');
    if (!(output instanceof HTMLSelectElement)) return false;
    output.value = 'Duplicate Program';
    output.dispatchEvent(new Event('change', { bubbles: true }));
    return true;
  })()`), true);
  assert.deepEqual(await evaluate(client, "structuredClone(window.__syndocalCueAudioMock.calls)"), pendingProof.calls, "ambiguous duplicate cannot be configured");
  assert.equal(await evaluate(client, `(() => {
    const output = document.querySelector('[data-timeline-cue-audio-output]');
    if (!(output instanceof HTMLSelectElement)) return false;
    output.value = 'Exact Program';
    output.dispatchEvent(new Event('change', { bubbles: true }));
    return true;
  })()`), true);
  await waitFor(() => evaluate(client, "window.__syndocalCueAudioMock.calls.length === 3"), "exact explicit Cue Audio settings mutation");
  const proof = await evaluate(client, `(() => {
    const cue = document.querySelector('.timelineCueAudioEditor');
    const panel = document.querySelector('.timelineToolsDisclosurePanel');
    if (!(cue instanceof HTMLDetailsElement) || !(panel instanceof HTMLElement)) return null;
    const summary = cue.querySelector(':scope > summary');
    const controls = [...cue.querySelectorAll('input, select, button')].filter((element) => {
      const rect = element.getBoundingClientRect();
      return rect.width > 0 && rect.height > 0;
    });
    const short = controls.filter((element) => element.getBoundingClientRect().height < 43.5).length;
    const style = getComputedStyle(panel);
    return {
      open: cue.open,
      lifecycle: cue.querySelector(':scope > summary output')?.textContent?.trim(),
      controls: controls.length,
      short,
      checkboxes: cue.querySelectorAll('input[type="checkbox"]').length,
      summaryHeight: summary?.getBoundingClientRect().height ?? 0,
      internalScroll: style.overflowY === 'auto' || style.overflowY === 'scroll',
      fixedOuter: document.documentElement.scrollWidth === document.documentElement.clientWidth
        && document.documentElement.scrollHeight === document.documentElement.clientHeight,
      topAuthorities: ["[data-timeline-metronome]", "[data-timeline-guide]"]
        .every((selector) => document.querySelector(selector)?.getAttribute('aria-pressed') !== null),
    };
  })()`);
  assert.deepEqual(proof, {
    open: true,
    lifecycle: "Running",
    controls: 5,
    short: 0,
    checkboxes: 0,
    summaryHeight: proof.summaryHeight,
    internalScroll: true,
    fixedOuter: true,
    topAuthorities: true,
  });
  assert.deepEqual(await evaluate(client, "structuredClone(window.__syndocalCueAudioMock.calls)"), [
    { command: 'list_audio_output_devices', args: {} },
    { command: 'get_timeline_cue_audio_status', args: {} },
    {
      command: 'set_machine_timeline_cue_audio_settings',
      args: { settings: { version: 1, route: 'explicit_device', device_name: 'Exact Program', topology_fingerprint: 'topology-fingerprint-a', click_gain: 1, guide_gain: 0.85 } },
    },
  ], "Cue Audio invokes manual list, canonical status, then only the exact selected topology-fenced settings");
  assert.ok(proof.summaryHeight >= 43.5, "Cue Audio disclosure summary preserves a 44px target");
  console.log("1280x720 Cue Audio: disclosure, Click/Guide authority, 44px controls, internal scroll, and no local enable passed");
} finally {
  client?.close();
  await stopChild(browser);
  await stopChild(vite);
  if (profileDir) await rm(profileDir, { recursive: true, force: true });
}
