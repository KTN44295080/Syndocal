import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { mkdtemp, readFile, rm } from "node:fs/promises";
import { Socket } from "node:net";
import { dirname, join, resolve } from "node:path";
import { tmpdir } from "node:os";
import { fileURLToPath } from "node:url";
import { cueAudioEventTargets, hitVerifiedCdpClick, installCueAudioMock } from "./check-timeline-cue-audio-fixture-lib.mjs";

const appRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const host = "127.0.0.1";
const vitePort = 5194;
const phaseACdpPort = 9244;
const phaseBCdpPort = 9245;
const fixtureUrl = `http://${host}:${vitePort}/?syndocalViewportFixture=timeline-layered`;
const defaultUrl = `http://${host}:${vitePort}/`;
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
      const result = await check();
      if (result) return result;
    } catch {
      // The Vite and CDP listeners may not have bound their ports yet.
    }
    await sleep(100);
  }
  throw new Error(`Timed out waiting for ${label}`);
};
const stopChild = async (child, label, endpoint = null) => {
  if (!child) return;
  if (child.exitCode === null) {
    const exited = new Promise((resolveExit) => child.once("exit", resolveExit));
    child.kill();
    await Promise.race([exited, sleep(2_000)]);
  }
  if (endpoint) {
    await waitFor(() => {
      if (child.exitCode !== null) return true;
      try {
        process.kill(child.pid, 0);
        return false;
      } catch {
        return true;
      }
    }, `${label} child exits exactly`, 5_000);
    await waitFor(async () => {
      try {
        const response = await fetch(`http://${host}:${endpoint.port}/json/version`);
        if (response.ok) return false;
        const socket = await probeTcpPort(endpoint.port);
        return !socket;
      } catch {
        try {
          return !(await probeTcpPort(endpoint.port));
        } catch {
          return false;
        }
      }
    }, `${label} endpoint stops responding`, 5_000);
  }
};
const probeTcpPort = (port) => new Promise((resolveProbe, rejectProbe) => {
  const socket = new Socket();
  let settled = false;
  const settle = (callback, value) => {
    if (settled) return;
    settled = true;
    socket.destroy();
    callback(value);
  };
  socket.setTimeout(500);
  socket.once("connect", () => settle(resolveProbe, true));
  socket.once("timeout", () => settle(rejectProbe, new Error(`TCP probe timed out on ${host}:${port}`)));
  socket.once("error", (error) => {
    if (error.code === "ECONNREFUSED") settle(resolveProbe, false);
    else settle(rejectProbe, error);
  });
  socket.connect(port, host);
});
const assertPortUnused = async (port, label) => {
  let serving;
  try {
    serving = await probeTcpPort(port);
  } catch (error) {
    throw new Error(`${label} CDP port ${port} is not provably unused: ${error.message}`);
  }
  if (serving) {
    throw new Error(`${label} CDP port ${port} is already serving before spawn`);
  }
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
const cueAudioMockSource = `(${installCueAudioMock.toString()})(${JSON.stringify(cueAudioEventTargets)});`;
const readBrowserEndpoint = async (profileDir, expectedPort, label) => {
  const activePortPath = join(profileDir, "DevToolsActivePort");
  const activePortContents = (await readFile(activePortPath, "utf8")).trim().split(/\r?\n/);
  assert.equal(activePortContents.length, 2, `${label} DevToolsActivePort has exactly port and websocket path`);
  const [activePortText, browserWebSocketPath] = activePortContents;
  const activePort = Number(activePortText);
  if (expectedPort !== 0) {
    assert.equal(activePort, expectedPort, `${label} DevToolsActivePort port matches requested CDP port`);
  }
  assert.match(browserWebSocketPath, /^\/devtools\/browser\/[A-Za-z0-9_-]+$/, `${label} DevToolsActivePort websocket path is a browser endpoint`);
  const response = await fetch(`http://${host}:${activePort}/json/version`);
  assert.ok(response.ok, `${label} /json/version responds on the profile's CDP port`);
  const version = await response.json();
  const browserWebSocketUrl = new URL(version.webSocketDebuggerUrl);
  assert.equal(browserWebSocketUrl.protocol, "ws:", `${label} browser websocket uses ws`);
  assert.equal(browserWebSocketUrl.hostname, host, `${label} browser websocket host matches CDP endpoint`);
  assert.equal(Number(browserWebSocketUrl.port), activePort, `${label} browser websocket port matches DevToolsActivePort`);
  assert.equal(browserWebSocketUrl.pathname, browserWebSocketPath, `${label} browser websocket path matches DevToolsActivePort and /json/version`);
  return { port: activePort, browserWebSocketPath, browserWebSocketUrl: version.webSocketDebuggerUrl };
};
const openTarget = async (url, endpoint, preload = false) => {
  let created = null;
  await readBrowserEndpoint(endpoint.profileDir, endpoint.port, endpoint.label);
  const browserClient = new CdpClient(endpoint.browserWebSocketUrl);
  await browserClient.ready();
  const { targetId } = await browserClient.send("Target.createTarget", { url: "about:blank" });
  browserClient.close();
  await waitFor(async () => {
    const response = await fetch(`http://${host}:${endpoint.port}/json/list`);
    if (!response.ok) return false;
    const targets = await response.json();
    created = targets.find((target) => target.id === targetId && target.type === "page" && typeof target.webSocketDebuggerUrl === "string");
    return created ?? false;
  }, `${endpoint.label} target`);
  const targetWebSocketUrl = new URL(created.webSocketDebuggerUrl);
  assert.equal(targetWebSocketUrl.protocol, "ws:", `${endpoint.label} target websocket uses ws`);
  assert.equal(targetWebSocketUrl.hostname, host, `${endpoint.label} target websocket host matches CDP endpoint`);
  assert.equal(Number(targetWebSocketUrl.port), endpoint.port, `${endpoint.label} target websocket port matches fresh profile endpoint`);
  const nextClient = new CdpClient(created.webSocketDebuggerUrl);
  await nextClient.ready();
  await nextClient.send("Page.enable");
  await nextClient.send("Runtime.enable");
  await nextClient.send("Emulation.setDeviceMetricsOverride", { width: 1280, height: 720, deviceScaleFactor: 1, mobile: false });
  if (preload) {
    await nextClient.send("Page.addScriptToEvaluateOnNewDocument", { source: cueAudioMockSource });
  }
  await nextClient.send("Page.navigate", { url });
  return nextClient;
};
const launchBrowser = async (profilePrefix, reservedPort, label) => {
  const nextProfileDir = await mkdtemp(join(tmpdir(), profilePrefix));
  await assertPortUnused(reservedPort, label);
  const nextBrowser = spawn(browserPath, [
    "--headless=new",
    "--disable-gpu",
    "--no-first-run",
    "--no-default-browser-check",
    "--remote-debugging-address=127.0.0.1",
    // Chrome writes DevToolsActivePort when it selects the port. The profile
    // file is the authority for the actual per-phase CDP endpoint.
    "--remote-debugging-port=0",
    `--user-data-dir=${nextProfileDir}`,
    "about:blank",
  ], { stdio: "ignore" });
  const endpoint = await waitFor(async () => {
    try {
      return await readBrowserEndpoint(nextProfileDir, 0, label);
    } catch {
      return false;
    }
  }, `${profilePrefix} browser endpoint`);
  return { process: nextBrowser, profileDir: nextProfileDir, endpoint: { ...endpoint, profileDir: nextProfileDir, label } };
};
const prepareTimelineSurface = async (client, label) => {
  assert.equal(await click(client, '[data-workspace-option="control"]'), true, `${label} opens Control workspace`);
  assert.equal(await click(client, '[data-edit-domain-navigation] [data-control-mode-option="live"]'), true, `${label} opens Live mode`);
  await waitFor(() => click(client, '[data-timeline-desk-surface="show"]'), `${label} Timeline Show tab`);
  assert.equal(await click(client, ".timelineToolsDisclosure > summary"), true, `${label} opens Timeline tools`);
  await waitFor(() => evaluate(client, "document.querySelector('.timelineToolsDisclosure[open] .timelinePerformanceEditor') !== null"), `${label} Timeline performance disclosure`);
};
const openCueAudioPanel = async (client, label) => {
  await evaluate(client, "window.__syndocalCueAudioMock.cueAudioCalls = []; window.__syndocalCueAudioMock.captureCueAudioCalls = true; window.__syndocalCueAudioMock.manualRefreshListSeen = false;");
  assert.equal(await click(client, "[data-timeline-cue-audio-open-setup]"), true, `${label} navigates from Timeline status to Setup I/O Audio`);
  await waitFor(() => evaluate(client, "document.querySelector('[data-io-connection=\"audio\"]') !== null"), `${label} Setup I/O deck`);
  assert.equal(await click(client, '[data-io-connection="audio"] .setupIoConnectionSelect'), true, `${label} selects the Audio workbench`);
  await waitFor(() => evaluate(client, `(() => [...document.querySelectorAll('[data-timeline-cue-audio-routing] button')]
    .some((candidate) => candidate.textContent?.trim() === 'Refresh Windows outputs'))()`), `${label} Setup Cue Audio Refresh button`);
  await waitFor(() => evaluate(client, "window.__syndocalCueAudioMock?.cueAudioCalls?.map((call) => call.command).join(',') === 'list_audio_output_devices,get_timeline_cue_audio_status'"), `${label} automatic Setup list then Cue Audio status`);
};
const runTrustedCueAudioRefresh = async (client, label) => {
  await evaluate(client, "window.__syndocalCueAudioMock.cueAudioCalls = []; window.__syndocalCueAudioMock.captureCueAudioCalls = true; window.__syndocalCueAudioMock.manualRefreshListSeen = false;");
  assert.equal(await evaluate(client, `(() => {
    const button = [...document.querySelectorAll('[data-timeline-cue-audio-routing] button')]
      .find((candidate) => candidate.textContent?.trim() === 'Refresh Windows outputs');
    if (!(button instanceof HTMLButtonElement)) return false;
    button.scrollIntoView({ block: 'center', inline: 'nearest' });
    button.addEventListener('click', (event) => { window.__syndocalCueAudioMock.refreshClickTrusted = event.isTrusted; }, { once: true });
    return true;
  })()`), true, `${label} installs trusted Refresh observer`);
  const refreshTarget = await hitVerifiedCdpClick(client, "[data-timeline-cue-audio-routing] button", "Refresh Windows outputs", `${label} Cue Audio Refresh outputs`);
  assert.equal(refreshTarget.pointerSequence, "mouseMoved>mousePressed>mouseReleased", `${label} Refresh uses a real CDP pointer sequence`);
  assert.equal(await evaluate(client, "window.__syndocalCueAudioMock.refreshClickTrusted"), true, `${label} Refresh click is trusted browser input`);
  await waitFor(() => evaluate(client, "window.__syndocalCueAudioMock.cueAudioCalls.map((call) => call.command).join(',') === 'list_audio_output_devices,get_timeline_cue_audio_status'"), `${label} manual list then Cue Audio status`);
  return refreshTarget;
};
const installCueAudioStatusEndpointOverride = async (client, label) => {
  assert.equal(await evaluate(client, `(() => {
    const internals = window.__TAURI_INTERNALS__;
    const mock = window.__syndocalCueAudioMock;
    if (!internals || typeof internals.invoke !== 'function' || !mock) return false;
    if (internals.__timelineCueAudioStatusEndpointOverrideInstalled) return true;
    const invoke = internals.invoke;
    internals.invoke = async (command, args = {}) => {
      if (command === 'get_timeline_cue_audio_status' && Array.isArray(mock.nextStatusEndpoints)) {
        mock.endpoints = structuredClone(mock.nextStatusEndpoints);
      }
      if (command === 'get_timeline_cue_audio_status' && mock.statusOnlyGateArmed === true) {
        mock.statusOnlyGateArmed = false;
        mock.statusOnlyCallCount = (mock.statusOnlyCallCount ?? 0) + 1;
        await new Promise((resolveStatusOnly) => { mock.releaseStatusOnly = resolveStatusOnly; });
      }
      return invoke(command, args);
    };
    internals.__timelineCueAudioStatusEndpointOverrideInstalled = true;
    return true;
  })()`), true, `${label} installs deterministic status endpoint override`);
};
const setCueAudioStatusEndpoints = (client, endpoints) => evaluate(client, `window.__syndocalCueAudioMock.nextStatusEndpoints = ${JSON.stringify(endpoints)};`);
const readCueAudioLedger = (client) => evaluate(client, `(() => {
  const mock = window.__syndocalCueAudioMock;
  const calls = mock?.calls ?? [];
  return {
    calls: structuredClone(calls),
    knownCommands: structuredClone(mock?.knownCommands ?? []),
    rejected: structuredClone(mock?.rejected ?? []),
    ownerCallCount: calls.filter((call) => call.command === "register_project_transaction_owner").length,
    programAudioCallCount: calls.filter((call) => call.command === "set_program_audio_handoff_config").length,
    registrationArgs: structuredClone(mock?.registrationArgs ?? null),
    programAudioHandoffConfigs: structuredClone(mock?.programAudioHandoffConfigs ?? []),
    authorityBundleDeliveryCount: mock?.authorityBundleDeliveryCount ?? 0,
    eventSubscriptions: calls
      .filter((call) => call.command === "plugin:event|listen")
      .map((call) => ({ event: call.args.event, target: call.args.target })),
  };
})()`);
const assertCueAudioLedger = (proof, label, intentionalUnknownCount) => {
  const unexpectedCalls = proof.calls.filter((call) =>
    call.command !== "cue_audio_unknown_probe" && !proof.knownCommands.includes(call.command));
  assert.deepEqual(unexpectedCalls, [], `${label} organic IPC ledger is fully allowlisted`);
  assert.equal(
    proof.calls.filter((call) => call.command === "cue_audio_unknown_probe").length,
    intentionalUnknownCount,
    `${label} has only the intentional unknown probe command`,
  );
  const unexpectedRejected = proof.rejected.filter((entry) => entry.command !== "cue_audio_unknown_probe");
  assert.deepEqual(unexpectedRejected, [], `${label} has no delayed unknown or degraded IPC rejection`);
  assert.equal(
    proof.rejected.filter((entry) => entry.command === "cue_audio_unknown_probe").length,
    intentionalUnknownCount,
    `${label} has only the intentional unknown probe rejection`,
  );
};

let vite;
let browser;
let client;
let profileDir;
let browserEndpoint;
let phaseAProfileDir;
let phaseABrowserPid;
let phaseACdpEndpointPort;
let phaseBBrowserPid;
try {
  assert.ok(browserPath, "Chrome or Edge is required for the Cue Audio browser gate");
  vite = spawn(process.execPath, [resolve(appRoot, "node_modules/vite/bin/vite.js"), "--host", host, "--port", String(vitePort), "--strictPort"], { cwd: appRoot, stdio: "ignore" });
  await waitFor(async () => (await fetch(fixtureUrl)).ok, "Vite fixture server");
  const phaseA = await launchBrowser("syndocal-timeline-cue-audio-phase-a-", phaseACdpPort, "phase A");
  browser = phaseA.process;
  profileDir = phaseA.profileDir;
  browserEndpoint = phaseA.endpoint;
  phaseACdpEndpointPort = browserEndpoint.port;
  phaseAProfileDir = profileDir;
  phaseABrowserPid = browser.pid;
  client = await openTarget(fixtureUrl, browserEndpoint);
  await waitFor(() => evaluate(client, "document.querySelector('.app') && document.readyState === 'complete'"), "app mount");
  assert.equal(await evaluate(client, "window.__TAURI_INTERNALS__ === undefined"), true, "phase A mounts as a pure browser without a Tauri preload");
  await prepareTimelineSurface(client, "phase A");
  await evaluate(client, cueAudioMockSource);
  await openCueAudioPanel(client, "phase A");
  await runTrustedCueAudioRefresh(client, "phase A");
  assert.equal(await evaluate(client, `(() => {
    const route = document.querySelector('[data-audio-output-field="timeline-cue-route"] select');
    if (!(route instanceof HTMLSelectElement)) return false;
    route.value = 'explicit_device';
    route.dispatchEvent(new Event('input', { bubbles: true }));
    return true;
  })()`), true);
  const pendingProof = await evaluate(client, `(() => {
    const output = document.querySelector('[data-timeline-cue-audio-output]');
    if (!(output instanceof HTMLSelectElement)) return null;
    const duplicate = [...output.options].find((option) => option.value === 'Duplicate Program');
    return {
      value: output.value,
      duplicateDisabled: duplicate?.disabled === true,
      calls: structuredClone(window.__syndocalCueAudioMock.cueAudioCalls),
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
    output.dispatchEvent(new Event('input', { bubbles: true }));
    return true;
  })()`), true);
  assert.deepEqual(await evaluate(client, "structuredClone(window.__syndocalCueAudioMock.cueAudioCalls)"), pendingProof.calls, "ambiguous duplicate cannot be configured");
  await evaluate(client, "window.__syndocalCueAudioMock.captureCueAudioCalls = true;");
  assert.equal(await evaluate(client, `(() => {
    const output = document.querySelector('[data-timeline-cue-audio-output]');
    if (!(output instanceof HTMLSelectElement)) return false;
    output.value = 'Exact Program';
    output.dispatchEvent(new Event('input', { bubbles: true }));
    return true;
  })()`), true);
  await waitFor(() => evaluate(client, "window.__syndocalCueAudioMock.cueAudioCalls.length === 3"), "exact explicit Cue Audio settings mutation");
  const proof = await evaluate(client, `(() => {
    const cue = document.querySelector('[data-timeline-cue-audio-routing]');
    const panel = document.querySelector('[data-io-workbench-body]');
    if (!(cue instanceof HTMLElement) || !(panel instanceof HTMLElement)) return null;
    const controls = [...cue.querySelectorAll('input, select, button')].filter((element) => {
      const rect = element.getBoundingClientRect();
      return rect.width > 0 && rect.height > 0;
    });
    const short = controls.filter((element) => element.getBoundingClientRect().height < 43.5).length;
    const style = getComputedStyle(panel);
    return {
      present: true,
      lifecycle: cue.querySelector('[data-timeline-cue-audio-status]')?.textContent?.trim(),
      controls: controls.length,
      short,
      checkboxes: cue.querySelectorAll('input[type="checkbox"]').length,
      internalScroll: style.overflowY === 'auto' || style.overflowY === 'scroll',
      fixedOuter: document.documentElement.scrollWidth === document.documentElement.clientWidth
        && document.documentElement.scrollHeight === document.documentElement.clientHeight,
      topAuthorities: ["[data-timeline-metronome]", "[data-timeline-guide]"]
        .every((selector) => document.querySelector(selector)?.getAttribute('aria-pressed') !== null),
    };
  })()`);
  assert.deepEqual(proof, {
    present: true,
    lifecycle: "Running",
    controls: 5,
    short: 0,
    checkboxes: 0,
    internalScroll: true,
    fixedOuter: true,
    topAuthorities: true,
  });
  assert.deepEqual(await evaluate(client, "structuredClone(window.__syndocalCueAudioMock.cueAudioCalls)"), [
    { command: 'list_audio_output_devices', args: {} },
    { command: 'get_timeline_cue_audio_status', args: {} },
    {
      command: 'set_machine_timeline_cue_audio_settings',
      args: { settings: { version: 1, route: 'explicit_device', device_name: 'Exact Program', topology_fingerprint: 'topology-fingerprint-a', click_gain: 1, guide_gain: 0.85 } },
    },
  ], "Cue Audio invokes manual list, canonical status, then only the exact selected topology-fenced settings");
  assert.equal(await evaluate(client, "document.querySelector('[data-timeline-cue-audio-output]')?.value"), "Exact Program", "exact desired output remains visibly selected after configuration");
  await installCueAudioStatusEndpointOverride(client, "phase A");
  await setCueAudioStatusEndpoints(client, [
    { name: "Exact Program", occurrences: 1, selectable: true },
    { name: "Duplicate Program", occurrences: 2, selectable: false },
  ]);
  await runTrustedCueAudioRefresh(client, "phase A reordered refresh");
  const reorderedProof = await evaluate(client, `(() => {
    const output = document.querySelector('[data-timeline-cue-audio-output]');
    const resolved = document.querySelector('.audioOutputTimelineCueAudioRuntime');
    return {
      value: output instanceof HTMLSelectElement ? output.value : null,
      options: output instanceof HTMLSelectElement ? [...output.options].map((option) => option.value) : [],
      desired: window.__syndocalCueAudioMock.settings.device_name,
      resolved: resolved?.textContent?.trim() ?? null,
      calls: structuredClone(window.__syndocalCueAudioMock.cueAudioCalls),
    };
  })()`);
  assert.deepEqual(reorderedProof, {
    value: "Exact Program",
    options: ["", "Exact Program", "Duplicate Program"],
    desired: "Exact Program",
    resolved: "Explicit WDM device · Exact Program",
    calls: [
      { command: "list_audio_output_devices", args: {} },
      { command: "get_timeline_cue_audio_status", args: {} },
    ],
  }, "endpoint reorder and refresh preserve the stable desired selection without a settings mutation");
  assert.equal(await evaluate(client, `(() => {
    const output = document.querySelector('[data-timeline-cue-audio-output]');
    if (!(output instanceof HTMLSelectElement)) return null;
    output.value = 'Duplicate Program';
    return output.value;
  })()`), "Duplicate Program", "phase A models native selection drift before the authoritative refresh");
  await runTrustedCueAudioRefresh(client, "phase A selection-drift refresh");
  await waitFor(() => evaluate(client, "document.querySelector('[data-timeline-cue-audio-output]')?.value === 'Exact Program'"), "phase A exact selection recovery");
  const selectionRecoveryProof = await evaluate(client, `(() => {
    const calls = window.__syndocalCueAudioMock.cueAudioCalls;
    return {
      statusCount: calls.filter((call) => call.command === 'get_timeline_cue_audio_status').length,
      listCount: calls.filter((call) => call.command === 'list_audio_output_devices').length,
      configureCount: calls.filter((call) => call.command === 'set_machine_timeline_cue_audio_settings').length,
      desired: window.__syndocalCueAudioMock.settings.device_name,
      value: document.querySelector('[data-timeline-cue-audio-output]')?.value ?? null,
      resolved: document.querySelector('.audioOutputTimelineCueAudioRuntime')?.textContent?.trim() ?? null,
    };
  })()`);
  assert.deepEqual(selectionRecoveryProof, {
    statusCount: 1,
    listCount: 1,
    configureCount: 0,
    desired: "Exact Program",
    value: "Exact Program",
    resolved: "Explicit WDM device · Exact Program",
  }, "authoritative Setup refresh repairs DOM selection drift without configuration mutation");
  await setCueAudioStatusEndpoints(client, [
    { name: "Exact Program", occurrences: 1, selectable: true },
    { name: "Exact Program", occurrences: 1, selectable: true },
  ]);
  await runTrustedCueAudioRefresh(client, "phase A duplicate-row refresh");
  assert.equal(await evaluate(client, `(() => {
    const output = document.querySelector('[data-timeline-cue-audio-output]');
    if (!(output instanceof HTMLSelectElement)) return false;
    output.value = 'Exact Program';
    output.dispatchEvent(new Event('input', { bubbles: true }));
    output.value = '';
    return true;
  })()`), true, "phase A duplicate rows exercise configure admission");
  const duplicateRowsProof = await evaluate(client, `(() => {
    const output = document.querySelector('[data-timeline-cue-audio-output]');
    const rows = output instanceof HTMLSelectElement
      ? [...output.options].filter((option) => option.value === 'Exact Program')
      : [];
    return {
      value: output instanceof HTMLSelectElement ? output.value : null,
      rowCount: rows.length,
      allDisabled: rows.length === 2 && rows.every((option) => option.disabled),
      calls: structuredClone(window.__syndocalCueAudioMock.cueAudioCalls),
    };
  })()`);
  assert.deepEqual(duplicateRowsProof, {
    value: "",
    rowCount: 2,
    allDisabled: true,
    calls: [
      { command: "list_audio_output_devices", args: {} },
      { command: "get_timeline_cue_audio_status", args: {} },
    ],
  }, "two same-name rows remain blank and disabled without configure IPC");
  await setCueAudioStatusEndpoints(client, [
    { name: "Exact Program", occurrences: 2, selectable: true },
  ]);
  await runTrustedCueAudioRefresh(client, "phase A contradictory-occurrence refresh");
  assert.equal(await evaluate(client, `(() => {
    const output = document.querySelector('[data-timeline-cue-audio-output]');
    if (!(output instanceof HTMLSelectElement)) return false;
    output.value = 'Exact Program';
    output.dispatchEvent(new Event('input', { bubbles: true }));
    output.value = '';
    return true;
  })()`), true, "phase A contradictory occurrence exercises configure admission");
  const contradictoryOccurrenceProof = await evaluate(client, `(() => {
    const output = document.querySelector('[data-timeline-cue-audio-output]');
    const row = output instanceof HTMLSelectElement
      ? [...output.options].find((option) => option.value === 'Exact Program')
      : null;
    return {
      value: output instanceof HTMLSelectElement ? output.value : null,
      disabled: row?.disabled === true,
      calls: structuredClone(window.__syndocalCueAudioMock.cueAudioCalls),
    };
  })()`);
  assert.deepEqual(contradictoryOccurrenceProof, {
    value: "",
    disabled: true,
    calls: [
      { command: "list_audio_output_devices", args: {} },
      { command: "get_timeline_cue_audio_status", args: {} },
    ],
  }, "occurrences > 1 remains blank and disabled without configure IPC");
  await setCueAudioStatusEndpoints(client, [
    { name: "Other Program", occurrences: 1, selectable: true },
  ]);
  await runTrustedCueAudioRefresh(client, "phase A missing refresh");
  const missingProof = await evaluate(client, `(() => {
    const output = document.querySelector('[data-timeline-cue-audio-output]');
    const missing = output instanceof HTMLSelectElement
      ? [...output.options].find((option) => option.value === 'Exact Program')
      : null;
    return {
      value: output instanceof HTMLSelectElement ? output.value : null,
      missingDisabled: missing?.disabled === true,
      calls: structuredClone(window.__syndocalCueAudioMock.cueAudioCalls),
    };
  })()`);
  assert.deepEqual(missingProof, {
    value: "Exact Program",
    missingDisabled: true,
    calls: [
      { command: "list_audio_output_devices", args: {} },
      { command: "get_timeline_cue_audio_status", args: {} },
    ],
  }, "missing desired output stays visibly fail-closed without implicit configuration");
  await setCueAudioStatusEndpoints(client, [
    { name: "Exact Program", occurrences: 2, selectable: true },
  ]);
  await runTrustedCueAudioRefresh(client, "phase A ambiguous refresh");
  const ambiguousProof = await evaluate(client, `(() => {
    const output = document.querySelector('[data-timeline-cue-audio-output]');
    const ambiguous = output instanceof HTMLSelectElement
      ? [...output.options].find((option) => option.value === 'Exact Program')
      : null;
    return {
      value: output instanceof HTMLSelectElement ? output.value : null,
      ambiguousDisabled: ambiguous?.disabled === true,
      ambiguousLabel: ambiguous?.textContent?.trim() ?? null,
      calls: structuredClone(window.__syndocalCueAudioMock.cueAudioCalls),
    };
  })()`);
  assert.deepEqual(ambiguousProof, {
    value: "Exact Program",
    ambiguousDisabled: true,
    ambiguousLabel: "Exact Program (2 matching outputs; ambiguous)",
    calls: [
      { command: "list_audio_output_devices", args: {} },
      { command: "get_timeline_cue_audio_status", args: {} },
    ],
  }, "ambiguous desired output stays visibly fail-closed without implicit configuration");
  console.log("PHASE A pure-browser fixture Cue Audio: Setup ownership, Timeline navigation, 44px controls, internal scroll, and no duplicate route passed");
  client.close();
  client = null;
  await stopChild(browser, "phase A browser", browserEndpoint);
  browser = null;
  browserEndpoint = null;
  await rm(profileDir, { recursive: true, force: true });
  assert.equal(existsSync(phaseAProfileDir), false, "phase B starts after phase A profile cleanup");
  profileDir = null;

  const phaseB = await launchBrowser("syndocal-timeline-cue-audio-phase-b-", phaseBCdpPort, "phase B");
  browser = phaseB.process;
  profileDir = phaseB.profileDir;
  browserEndpoint = phaseB.endpoint;
  phaseBBrowserPid = browser.pid;
  assert.notEqual(phaseBBrowserPid, phaseABrowserPid, "phase B uses a fresh Chrome process");
  assert.notEqual(profileDir, phaseAProfileDir, "phase B uses a fresh Chrome user-data-dir");
  assert.notEqual(browserEndpoint.port, phaseACdpEndpointPort, "phase B uses a distinct profile-bound CDP port");
  client = await openTarget(defaultUrl, browserEndpoint, true);
  await waitFor(() => evaluate(client, "document.querySelector('.app') && document.readyState === 'complete'"), "phase B app mount");
  assert.equal(await evaluate(client, "window.location.search"), "", "phase B uses the plain default URL without a viewport-fixture claim");
  await waitFor(() => evaluate(client, `(() => {
    const mock = window.__syndocalCueAudioMock;
    return mock?.registrationArgs !== null && mock?.programAudioHandoffConfigs?.length === 1;
  })()`), "phase B owner registration and Program Audio startup handoff");
  const startupProof = await readCueAudioLedger(client);
  assertCueAudioLedger(startupProof, "phase B startup milestone", 0);
  await waitFor(async () => {
    const proof = await readCueAudioLedger(client);
    return proof.calls.filter((call) => call.command === "get_external_video_transport_status").length >= 1;
  }, "phase B external video transport status poll");
  const preProbeProof = await readCueAudioLedger(client);
  assertCueAudioLedger(preProbeProof, "phase B pre-probe organic startup", 0);
  const transportStatusPolls = preProbeProof.calls.filter((call) => call.command === "get_external_video_transport_status");
  assert.ok(transportStatusPolls.length >= 1, "phase B observes at least one external video transport status poll");
  for (const poll of transportStatusPolls) {
    assert.deepEqual(poll.args, {}, "phase B external video transport status poll uses exact empty args");
  }
  assert.ok(preProbeProof.authorityBundleDeliveryCount >= 1, "phase B receives the production-shaped project authority bundle");
  assert.equal(preProbeProof.ownerCallCount, 1, "phase B owner registration occurs exactly once");
  assert.equal(preProbeProof.programAudioCallCount, 1, "phase B Program Audio startup occurs exactly once");
  assert.deepEqual(Object.keys(preProbeProof.registrationArgs).sort(), ["ownerId"], "phase B owner registration payload is exact");
  assert.match(preProbeProof.registrationArgs.ownerId, /^renderer:[A-Za-z0-9_.:-]{1,119}$/, "phase B owner registration is renderer-scoped");
  assert.deepEqual(preProbeProof.programAudioHandoffConfigs[0], { enabled: false, volume: 0.8, deviceName: null }, "phase B clean-profile Program Audio payload is exact");
  assert.ok(preProbeProof.eventSubscriptions.length > 0, "phase B startup registers known events");
  for (const subscription of preProbeProof.eventSubscriptions) {
    assert.deepEqual(subscription.target, cueAudioEventTargets[subscription.event], `phase B event target is exact for ${subscription.event}`);
  }
  const unknownCommandProof = await evaluate(client, `window.__TAURI_INTERNALS__.invoke("cue_audio_unknown_probe", {}).then(
    () => ({ rejected: false, reason: null }),
    (error) => ({ rejected: true, reason: String(error?.message ?? error) }),
  )`);
  assert.equal(unknownCommandProof.rejected, true, "phase B unknown Cue Audio commands fail closed");
  assert.match(unknownCommandProof.reason, /Unexpected Cue Audio invoke/, "phase B unknown Cue Audio rejection is explicit");
  await prepareTimelineSurface(client, "phase B");
  await openCueAudioPanel(client, "phase B");
  await runTrustedCueAudioRefresh(client, "phase B");
  assert.equal(await evaluate(client, "window.__syndocalCueAudioMock.programAudioHandoffConfigs.length"), 1, "phase B clean-profile Program Audio is emitted once");
  const finalProof = await readCueAudioLedger(client);
  assertCueAudioLedger(finalProof, "phase B final ledger", 1);
  console.log("PHASE B plain-default strict Tauri: production authority startup, allowlisted IPC, owner/Program Audio, unknown rejection, and trusted Refresh passed");
} finally {
  client?.close();
  await stopChild(browser, "active browser", browserEndpoint);
  await stopChild(vite, "Vite");
  if (profileDir) await rm(profileDir, { recursive: true, force: true });
}
