import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const appRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");

const host = "127.0.0.1";
const vitePort = 5189;
const cdpPort = 9239;
const baseUrl = `http://${host}:${vitePort}/?syndocalViewportFixture=vj-bank`;
const viewports = [
  { width: 1920, height: 1080 },
  { width: 1920, height: 1032 },
  { width: 1366, height: 768 },
  { width: 1280, height: 720 },
  { width: 860, height: 520 },
];
const chromeCandidates = [
  process.env.CHROME_PATH,
  "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe",
  "C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe",
  "C:\\Program Files\\Microsoft\\Edge\\Application\\msedge.exe",
].filter(Boolean);

const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const stopChild = async (child) => {
  if (!child || child.exitCode !== null) return;
  const exited = new Promise((resolve) => child.once("exit", resolve));
  child.kill();
  await Promise.race([exited, sleep(2_000)]);
};
const waitFor = async (check, label, timeoutMs = 30_000) => {
  const deadline = Date.now() + timeoutMs;
  let lastError;
  while (Date.now() < deadline) {
    try {
      const value = await check();
      if (value) return value;
    } catch (error) { lastError = error; }
    await sleep(100);
  }
  throw new Error(`Timed out waiting for ${label}${lastError ? `: ${lastError}` : ""}`);
};

class CdpClient {
  constructor(url) {
    this.socket = new WebSocket(url);
    this.nextId = 0;
    this.pending = new Map();
    this.socket.onmessage = (event) => {
      const message = JSON.parse(event.data);
      if (!message.id) return;
      const pending = this.pending.get(message.id);
      if (!pending) return;
      this.pending.delete(message.id);
      if (message.error) pending.reject(new Error(message.error.message));
      else pending.resolve(message.result);
    };
  }
  async ready() {
    if (this.socket.readyState === WebSocket.OPEN) return;
    await new Promise((resolve, reject) => {
      this.socket.onopen = resolve;
      this.socket.onerror = reject;
    });
  }
  send(method, params = {}) {
    const id = ++this.nextId;
    return new Promise((resolve, reject) => {
      this.pending.set(id, { resolve, reject });
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
const measureBank = (client, mode) => evaluate(client, `(() => {
  const candidates = [...document.querySelectorAll('[data-video-clip-slot-bank=${mode}]')];
  const root = candidates.find((candidate) => {
    const rect = candidate.getBoundingClientRect();
    const style = getComputedStyle(candidate);
    return rect.width > 0 && rect.height > 0 && style.display !== 'none' && style.visibility !== 'hidden';
  });
  const app = document.querySelector('.app');
  if (!(root instanceof HTMLElement) || !(app instanceof HTMLElement)) return null;
  const grid = root.querySelector('.videoClipSlotBankGrid');
  const pads = [...root.querySelectorAll('.videoClipSlotPad')];
  const primary = [...root.querySelectorAll('.videoClipSlotPrimary')];
  const rootRect = root.getBoundingClientRect();
  const gridStyle = grid instanceof HTMLElement ? getComputedStyle(grid) : null;
  const columns = gridStyle ? gridStyle.gridTemplateColumns.split(' ').filter(Boolean).length : 0;
  const badMedia = [...root.querySelectorAll('img')].filter((image) => getComputedStyle(image).objectFit !== 'contain').length;
  const shortTargets = [...root.querySelectorAll('button, select, input')].filter((control) => {
    const rect = control.getBoundingClientRect();
    return rect.width > 0 && rect.height > 0 && rect.height < 43.5;
  }).length;
  return {
    mode: root.dataset.videoClipSlotBank,
    padCount: pads.length,
    indices: pads.map((pad) => Number(pad.dataset.videoClipSlotIndex)),
    columns,
    overflowY: getComputedStyle(root).overflowY,
    rootScrollable: root.scrollHeight >= root.clientHeight,
    rootInViewport: rootRect.left >= -1 && rootRect.right <= innerWidth + 1,
    rootRect: [rootRect.width, rootRect.height],
    occupied: pads.filter((pad) => pad.classList.contains('occupied')).length,
    active: pads.filter((pad) => pad.classList.contains('active')).length,
    queued: pads.filter((pad) => pad.classList.contains('queued')).length,
    pending: pads.filter((pad) => pad.classList.contains('pending')).length,
    importVisible: [...root.querySelectorAll('button')].some((button) => button.textContent?.trim() === 'Import' && button.getBoundingClientRect().height > 0),
    cancelVisible: [...root.querySelectorAll('button')].some((button) => button.textContent?.trim() === 'Cancel Queue' && button.getBoundingClientRect().height > 0),
    transitionKind: [...root.querySelectorAll('select')].find((select) => select.closest('label')?.textContent?.includes('Transition'))?.value ?? null,
    transitionKindOptions: [...root.querySelectorAll('select')].find((select) => select.closest('label')?.textContent?.trim().startsWith('Transition'))?.options.length ?? 0,
    transitionDurationUnit: [...root.querySelectorAll('select')].find((select) => select.closest('label')?.textContent?.includes('Duration unit'))?.value ?? null,
    transitionDurationUnitOptions: [...root.querySelectorAll('select')].find((select) => select.closest('label')?.textContent?.includes('Duration unit'))?.options.length ?? 0,
    transitionDuration: [...root.querySelectorAll('input[type=number]')].find((input) => input.closest('label')?.textContent?.includes('Transition duration'))?.value ?? null,
    transitionDurationEnabled: Boolean([...root.querySelectorAll('input[type=number]')].find((input) => input.closest('label')?.textContent?.includes('Transition duration') && !input.disabled)),
    layerSelectorVisible: Boolean([...root.querySelectorAll('select')].find((select) => select.closest('label')?.textContent?.includes('Layer') && select.getBoundingClientRect().height > 0)),
    inspectorTriggerVisible: Boolean([...root.querySelectorAll('button')].find((button) => button.textContent?.trim() === 'More' && button.getBoundingClientRect().height > 0)),
    docFixed: document.documentElement.scrollWidth === document.documentElement.clientWidth
      && document.documentElement.scrollHeight === document.documentElement.clientHeight
      && app.scrollWidth === app.clientWidth && app.scrollHeight === app.clientHeight,
    primaryCount: primary.length,
    badMedia,
    shortTargets,
  };
})()`);
const measureEffectCatalog = (client) => evaluate(client, `(() => {
  const root = [...document.querySelectorAll('.videoEffectScopeCatalog')].find((candidate) => {
    const rect = candidate.getBoundingClientRect();
    const style = getComputedStyle(candidate);
    return rect.width > 0 && rect.height > 0 && style.display !== 'none' && style.visibility !== 'hidden';
  });
  const app = document.querySelector('.app');
  if (!(root instanceof HTMLElement) || !(app instanceof HTMLElement)) return null;
  const rect = root.getBoundingClientRect();
  const stages = [...root.querySelectorAll('[data-effect-stage-id]')];
  const controls = [...root.querySelectorAll('[data-effect-control]')];
  const shortTargets = [...root.querySelectorAll('button, select, input')].filter((control) => {
    const bounds = control.getBoundingClientRect();
    return bounds.width > 0 && bounds.height > 0 && bounds.height < 43.5;
  }).length;
  return {
    open: root.open,
    rect: [rect.width, rect.height],
    setupCount: root.querySelectorAll('.videoEffectCatalogSetupPanel').length,
    scopeKinds: [...root.querySelectorAll('[data-effect-scope]')].map((node) => node.getAttribute('data-effect-scope')),
    stageIds: stages.map((node) => Number(node.getAttribute('data-effect-stage-id'))),
    effectIds: stages.map((node) => Number(node.getAttribute('data-effect-id'))),
    controlCount: controls.filter((control) => control.getBoundingClientRect().height > 0).length,
    presetOptions: root.querySelectorAll('.videoEffectScopeActions select option').length,
    shortTargets,
    docFixed: document.documentElement.scrollWidth === document.documentElement.clientWidth
      && document.documentElement.scrollHeight === document.documentElement.clientHeight
      && app.scrollWidth === app.clientWidth && app.scrollHeight === app.clientHeight,
  };
})()`);
const measureTransitionBus = (client, compact) => evaluate(client, `(() => {
  const candidates = [...document.querySelectorAll('.videoTransitionBusPanel${compact ? ".compact" : ":not(.compact)"}')];
  const root = candidates.find((candidate) => {
    const rect = candidate.getBoundingClientRect();
    const style = getComputedStyle(candidate);
    return rect.width > 0 && rect.height > 0 && style.display !== 'none' && style.visibility !== 'hidden';
  });
  if (!(root instanceof HTMLElement)) return null;
  const active = root.querySelector('.videoTransitionBus.active');
  const progress = active?.querySelector('progress');
  const buttons = [...(active?.querySelectorAll('button') ?? [])];
  const rect = root.getBoundingClientRect();
  return {
    rect: [rect.width, rect.height],
    activeCount: root.querySelectorAll('.videoTransitionBus.active').length,
    progress: progress instanceof HTMLProgressElement ? progress.value : null,
    reverse: buttons.some((button) => button.textContent?.trim() === 'Reverse' && !button.disabled),
    release: buttons.some((button) => button.textContent?.trim() === 'Release' && !button.disabled),
    shortTargets: [...root.querySelectorAll('button, select, input')].filter((control) => {
      const bounds = control.getBoundingClientRect();
      return bounds.width > 0 && bounds.height > 0 && bounds.height < 43.5;
    }).length,
  };
})()`);

let vite;
let chrome;
let client;
let profileDir;
try {
  const chromePath = chromeCandidates.find((candidate) => existsSync(candidate));
  assert.ok(chromePath, "Chrome or Edge executable is required for the Clip Slot browser gate");
  vite = spawn(process.execPath, [resolve(appRoot, "node_modules/vite/bin/vite.js"), "--host", host, "--port", String(vitePort), "--strictPort"], {
    cwd: appRoot, stdio: "ignore",
  });
  await waitFor(async () => (await fetch(baseUrl)).ok, "Vite fixture server");
  profileDir = await mkdtemp(join(tmpdir(), "syndocal-clip-slot-browser-"));
  chrome = spawn(chromePath, [
    "--headless=new", "--disable-gpu", "--no-first-run", "--no-default-browser-check",
    `--remote-debugging-port=${cdpPort}`, `--user-data-dir=${profileDir}`, "about:blank",
  ], { stdio: "ignore" });
  const target = await waitFor(async () => {
    const response = await fetch(`http://${host}:${cdpPort}/json/new?${encodeURIComponent(baseUrl)}`, { method: "PUT" });
    return response.ok ? response.json() : null;
  }, "headless browser CDP target");
  client = new CdpClient(target.webSocketDebuggerUrl);
  await client.ready();
  await client.send("Page.enable");
  await client.send("Runtime.enable");

  for (const viewport of viewports) {
    await client.send("Emulation.setDeviceMetricsOverride", { ...viewport, deviceScaleFactor: 1, mobile: false });
    await client.send("Page.navigate", { url: baseUrl });
    await waitFor(() => evaluate(client, "document.querySelector('.app') && document.readyState === 'complete'"), "app mount");
    assert.equal(await click(client, '[data-workspace-option="control"]'), true);
    assert.equal(await click(client, '[data-control-mode-option="mixer"]'), true);
    await waitFor(() => evaluate(client, "document.querySelector('[data-video-clip-slot-bank=edit]')?.querySelectorAll('.videoClipSlotPad').length === 32"), "Edit Clip Slot bank");
    const edit = await measureBank(client, "edit");
    assert.equal(edit.padCount, 32);
    assert.deepEqual(edit.indices, Array.from({ length: 32 }, (_, index) => index));
    assert.ok(edit.columns >= 2 && edit.columns <= 8, `Edit columns ${edit.columns} remain a grid at ${viewport.width}x${viewport.height}`);
    assert.equal(edit.overflowY, "auto");
    assert.equal(edit.rootInViewport && edit.docFixed && edit.badMedia === 0, true);
    assert.ok(edit.rootRect[0] > 0 && edit.rootRect[1] > 0, "Edit root is rendered with nonzero geometry");
    assert.equal(edit.occupied, 32, "Edit fixture mounts 32 authored occupied slots");
    assert.deepEqual([edit.active, edit.queued, edit.pending], [1, 1, 1], "Edit renders independent runtime active/queued/pending truth");
    assert.equal(edit.importVisible && edit.layerSelectorVisible && edit.inspectorTriggerVisible, true, "Edit exposes persistent Import, layer selector, and inspector trigger");
    assert.equal(edit.shortTargets, 0, `Edit preserves 44px controls at ${viewport.width}x${viewport.height}`);
    assert.equal(await click(client, '.videoMixerLayerPane .videoEffectScopeCatalog > summary'), true);
    await waitFor(() => evaluate(client, "document.querySelector('.videoMixerLayerPane .videoEffectScopeCatalog[open]') !== null"), "Scoped FX catalog disclosure");
    await evaluate(client, "document.querySelector('.videoMixerLayerPane .videoEffectScopeCatalog')?.scrollIntoView({block:'nearest'})");
    const effects = await measureEffectCatalog(client);
    assert.ok(effects?.rect[0] > 0 && effects?.rect[1] > 0, "Scoped FX catalog has nonzero visible geometry");
    assert.equal(effects.setupCount, 1, "Scoped FX group/preset authoring is mounted");
    assert.deepEqual(effects.scopeKinds.sort(), ["composition", "group", "output", "output", "transition"], "Transition Bus, Composition, Group, and every Output scope are rendered from stable catalog truth");
    assert.deepEqual(effects.stageIds.sort((a, b) => a - b), [6_203, 6_204, 6_205, 6_206]);
    assert.deepEqual(effects.effectIds.sort((a, b) => a - b), [6_303, 6_304, 6_305, 6_306]);
    assert.equal(effects.controlCount, 4, "Stable scoped stage parameters are present for Transition Bus, Composition, Group, and Output chains");
    assert.ok(effects.presetOptions >= 2, "Each scoped chain can select the backend-authored preset");
    assert.equal(effects.shortTargets, 0, `Scoped FX preserves 44px controls at ${viewport.width}x${viewport.height}`);
    assert.equal(effects.docFixed, true, `Expanded Scoped FX keeps app/document outer scroll fixed at ${viewport.width}x${viewport.height}`);
    await evaluate(client, "document.querySelector('.videoTransitionBusPanel:not(.compact)')?.scrollIntoView({block:'nearest'})");
    const editBus = await measureTransitionBus(client, false);
    assert.ok(editBus?.rect[0] > 0 && editBus?.rect[1] > 0, "Edit transition bus is visible with nonzero geometry");
    assert.deepEqual([editBus.activeCount, editBus.progress, editBus.reverse, editBus.release, editBus.shortTargets], [1, 500, true, true, 0], "Edit exposes exact active runtime progress, reversible Take, anytime Release, and 44px controls");

    assert.equal(await click(client, '[data-workspace-option="touch"]'), true);
    assert.equal(await click(client, '[data-control-domain="video"]'), true, "Control Video domain is explicitly selected");
    await waitFor(() => evaluate(client, "document.querySelector('[data-video-clip-slot-bank=control]')?.querySelectorAll('.videoClipSlotPad').length === 32"), "Control Clip Slot bank");
    const control = await measureBank(client, "control");
    assert.equal(control.padCount, 32);
    assert.deepEqual(control.indices, edit.indices, "Edit and Control mount the same stable 32 pad identities");
    assert.ok(control.columns >= 2 && control.columns <= 8, `Control columns ${control.columns} remain a grid at ${viewport.width}x${viewport.height}`);
    assert.equal(control.rootInViewport && control.docFixed && control.badMedia === 0, true);
    assert.ok(control.rootRect[0] > 0 && control.rootRect[1] > 0, "Control root is actually visible with nonzero geometry");
    assert.equal(control.occupied, 32, "Control fixture mounts the same 32 authored occupied slots");
    assert.deepEqual([control.active, control.queued, control.pending], [1, 1, 1], "Control renders active/queued/pending runtime truth");
    assert.deepEqual(
      [control.transitionKind, control.transitionKindOptions, control.transitionDurationUnit, control.transitionDurationUnitOptions, control.transitionDuration, control.transitionDurationEnabled],
      ["Crossfade", 9, "Milliseconds", 3, "750", true],
      "Control exposes all transition modes and deterministic millisecond/beat/bar duration units",
    );
    await evaluate(client, `(() => {
      const root = [...document.querySelectorAll('[data-video-clip-slot-bank=control]')].find((candidate) => candidate.getBoundingClientRect().height > 0);
      const select = [...(root?.querySelectorAll('select') ?? [])].find((candidate) => candidate.closest('label')?.textContent?.includes('Duration unit'));
      if (!(select instanceof HTMLSelectElement)) return false;
      select.value = 'Beats';
      select.dispatchEvent(new Event('input', { bubbles: true }));
      return true;
    })()`);
    const musical = await measureBank(client, "control");
    assert.deepEqual(
      [musical.transitionDurationUnit, musical.transitionDuration],
      ["Beats", "0.75"],
      "Control converts the shared fixed-point duration to beat units without changing identity",
    );
    assert.equal(control.importVisible && control.cancelVisible && control.layerSelectorVisible && control.inspectorTriggerVisible, true, "Control exposes Import, cancel, layer selector, and inspector trigger");
    const take = await evaluate(client, `(() => { const button = document.querySelector('.touchDomainVideo [data-video-clip-slot-take]'); const rect = button?.getBoundingClientRect(); return button instanceof HTMLButtonElement && Boolean(rect && rect.width > 0 && rect.height >= 44) && !button.disabled; })()`);
    assert.equal(take, true, "Control has one visible enabled dominant queued Take target");
    await evaluate(client, "document.querySelector('.touchDomainVideo .videoTransitionBusPanel.compact')?.scrollIntoView({block:'nearest'})");
    const controlBus = await measureTransitionBus(client, true);
    assert.ok(controlBus?.rect[0] > 0 && controlBus?.rect[1] > 0, "Control transition bus is visible with nonzero geometry");
    assert.deepEqual([controlBus.activeCount, controlBus.progress, controlBus.reverse, controlBus.release, controlBus.shortTargets], [1, 500, true, true, 0], "Control exposes the same active runtime truth and reversible 44px controls");
    assert.equal(control.shortTargets, 0, `Control preserves 44px controls at ${viewport.width}x${viewport.height}`);
    console.log(`pass clip-slot-bank ${viewport.width}x${viewport.height} pads=${edit.padCount}/${control.padCount} cols=${edit.columns}/${control.columns} outer-scroll=0 targets<44=${edit.shortTargets}/${control.shortTargets}`);
  }
  console.log("Video Clip Slot B4 real-browser geometry gate passed.");
} finally {
  client?.close();
  await stopChild(chrome);
  await stopChild(vite);
  if (profileDir) {
    for (let attempt = 0; attempt < 5; attempt += 1) {
      try {
        await rm(profileDir, { recursive: true, force: true });
        break;
      } catch (error) {
        if (attempt === 4) throw error;
        await sleep(250);
      }
    }
  }
}
