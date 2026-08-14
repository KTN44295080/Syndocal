import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const appRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const host = "127.0.0.1";
const vitePort = 5193;
const cdpPort = 9243;
const baseUrl = `http://${host}:${vitePort}/?syndocalViewportFixture=timeline-layered`;
const viewports = [
  { width: 1920, height: 1080 },
  { width: 1366, height: 768 },
  { width: 1280, height: 720 },
  { width: 860, height: 520 },
];
const browsers = [
  process.env.CHROME_PATH,
  "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe",
  "C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe",
  "C:\\Program Files\\Microsoft\\Edge\\Application\\msedge.exe",
].filter(Boolean);
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const waitFor = async (check, label, timeoutMs = 30_000) => {
  const deadline = Date.now() + timeoutMs;
  let lastError;
  while (Date.now() < deadline) {
    try {
      const result = await check();
      if (result) return result;
    } catch (error) { lastError = error; }
    await sleep(100);
  }
  throw new Error(`Timed out waiting for ${label}${lastError ? `: ${lastError}` : ""}`);
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
const measure = (client) => evaluate(client, `(() => {
  const root = document.querySelector('.timelineShowSurface');
  const app = document.querySelector('.app');
  if (!(root instanceof HTMLElement) || !(app instanceof HTMLElement)) return null;
  const rect = root.getBoundingClientRect();
  const visible = (element) => {
    if (!(element instanceof Element)) return false;
    const bounds = element.getBoundingClientRect();
    const style = getComputedStyle(element);
    return bounds.width > 0 && bounds.height > 0 && style.display !== 'none' && style.visibility !== 'hidden';
  };
  const buttons = [...document.querySelectorAll('.timelineOperatorBar button')];
  const phases = [...document.querySelectorAll('.timelinePerformanceEditor .timelinePhaseBand')];
  const itemMenu = root.querySelector('.timelineItemContextMenu');
  const selectedVideo = [...root.querySelectorAll('.timelineVideoClip.selected')];
  const selectedAudio = [...root.querySelectorAll('.timelineAudioClip.selected')];
  const bank = document.querySelector('.timelinePerformanceEditor [data-timeline-bank]');
  const shortTargets = [...document.querySelectorAll('.timelinePerformanceEditor button, .timelinePerformanceEditor input, .timelinePerformanceEditor select, .timelineOperatorBar > .timelineLoopControls button, .timelineOperatorBar [data-timeline-guide]')].filter((element) => {
    const bounds = element.getBoundingClientRect();
    return bounds.width > 0 && bounds.height > 0 && bounds.height < 43.5;
  }).length;
  return {
    rect: [rect.width, rect.height],
    bankOpen: bank instanceof HTMLDetailsElement && bank.open,
    bankItems: bank?.querySelectorAll('.timelineBankList > li').length ?? 0,
    bankActive: bank?.querySelectorAll('.timelineBankList > li.active').length ?? 0,
    followLegend: [...(bank?.querySelectorAll('legend') ?? [])].some((node) => node.textContent?.includes('Follow to next Timeline')),
    followState: bank?.querySelector('.timelineFollowState')?.textContent?.trim() ?? '',
    phaseEditorOpen: Boolean(document.querySelector('.timelinePerformanceEditor .timelinePhaseEditor[open]')),
    guideAudioOpen: Boolean(document.querySelector('.timelinePerformanceEditor .timelineGuideAudioEditor[open]')),
    guideAudioControls: document.querySelectorAll('.timelineGuideAudioEditorBody input, .timelineGuideAudioEditorBody select').length,
    guideAudioState: document.querySelector('.timelineGuideAudioEditor > summary output')?.textContent?.trim() ?? '',
    phaseLabels: phases.map((phase) => phase.textContent?.trim() ?? ''),
    guidePressed: document.querySelector('.timelineOperatorBar [data-timeline-guide]')?.getAttribute('aria-pressed') ?? '',
    loopPressed: document.querySelector('.timelineOperatorBar [data-timeline-loop-toggle]')?.getAttribute('aria-pressed') ?? '',
    loopState: document.querySelector('.timelineOperatorBar .timelineLoopState')?.textContent?.trim() ?? '',
    loopScaleControls: buttons.filter((button) => ['Halve loop length', 'Double loop length'].includes(button.title)).length,
    videoClips: root.querySelectorAll('.timelineVideoClip').length,
    audioClips: root.querySelectorAll('.timelineAudioClip').length,
    selectedVideo: selectedVideo.length,
    selectedAudio: selectedAudio.length,
    groupEnabled: Boolean([...root.querySelectorAll('.timelineItemContextMenu button')].find((button) => button.textContent?.trim() === 'Group selected' && !button.disabled)),
    ungroupEnabled: Boolean([...root.querySelectorAll('.timelineItemContextMenu button')].find((button) => button.textContent?.trim() === 'Ungroup' && !button.disabled)),
    duplicateEnabled: Boolean([...root.querySelectorAll('.timelineItemContextMenu button')].find((button) => button.textContent?.trim() === 'Duplicate selected' && !button.disabled)),
    nudgeEarlierEnabled: Boolean([...root.querySelectorAll('.timelineItemContextMenu button')].find((button) => button.textContent?.trim() === 'Nudge earlier' && !button.disabled)),
    nudgeLaterEnabled: Boolean([...root.querySelectorAll('.timelineItemContextMenu button')].find((button) => button.textContent?.trim() === 'Nudge later' && !button.disabled)),
    quantizeEnabled: Boolean([...root.querySelectorAll('.timelineItemContextMenu button')].find((button) => button.textContent?.trim() === 'Quantize to grid' && !button.disabled)),
    deleteEnabled: Boolean([...root.querySelectorAll('.timelineItemContextMenu button')].find((button) => button.textContent?.trim() === 'Delete selected' && !button.disabled)),
    itemMenuRect: itemMenu instanceof HTMLElement ? (() => { const bounds = itemMenu.getBoundingClientRect(); return [bounds.left, bounds.top, bounds.right, bounds.bottom]; })() : null,
    menuShortTargets: [...root.querySelectorAll('.timelineItemContextMenu button')].filter((button) => {
      const bounds = button.getBoundingClientRect();
      return bounds.width > 0 && bounds.height > 0 && bounds.height < 43.5;
    }).length,
    shortTargets,
    fixedOuter: document.documentElement.scrollWidth === document.documentElement.clientWidth
      && document.documentElement.scrollHeight === document.documentElement.clientHeight
      && app.scrollWidth === app.clientWidth && app.scrollHeight === app.clientHeight,
  };
})()`);

let vite;
let browser;
let client;
let profileDir;
try {
  const browserPath = browsers.find((candidate) => existsSync(candidate));
  assert.ok(browserPath, "Chrome or Edge is required for the Timeline performance browser gate");
  vite = spawn(process.execPath, [resolve(appRoot, "node_modules/vite/bin/vite.js"), "--host", host, "--port", String(vitePort), "--strictPort"], { cwd: appRoot, stdio: "ignore" });
  await waitFor(async () => (await fetch(baseUrl)).ok, "Vite fixture server");
  profileDir = await mkdtemp(join(tmpdir(), "syndocal-timeline-performance-"));
  browser = spawn(browserPath, ["--headless=new", "--disable-gpu", "--no-first-run", "--no-default-browser-check", `--remote-debugging-port=${cdpPort}`, `--user-data-dir=${profileDir}`, "about:blank"], { stdio: "ignore" });
  const target = await waitFor(async () => {
    const response = await fetch(`http://${host}:${cdpPort}/json/new?${encodeURIComponent(baseUrl)}`, { method: "PUT" });
    return response.ok ? response.json() : null;
  }, "browser target");
  client = new CdpClient(target.webSocketDebuggerUrl);
  await client.ready();
  await client.send("Page.enable");
  await client.send("Runtime.enable");

  for (const viewport of viewports) {
    await client.send("Emulation.setDeviceMetricsOverride", { ...viewport, deviceScaleFactor: 1, mobile: false });
    await client.send("Page.navigate", { url: baseUrl });
    await waitFor(() => evaluate(client, "document.querySelector('.app') && document.readyState === 'complete'"), "app mount");
    assert.equal(await click(client, '[data-workspace-option="control"]'), true);
    assert.equal(await click(client, '[data-workspace-pane="lower-right"] [data-lighting-context-tab="timeline"]'), true);
    assert.equal(await waitFor(() => click(client, '[data-timeline-desk-surface="show"]'), "Timeline Show tab"), true);
    await waitFor(() => evaluate(client, "document.querySelectorAll('.timelineVideoClip').length === 1 && document.querySelectorAll('.timelineAudioClip').length === 2"), "authored Timeline media clips");
    assert.equal(await click(client, '.timelineToolsDisclosure > summary'), true);
    await waitFor(() => evaluate(client, "document.querySelector('.timelineToolsDisclosure[open] .timelinePerformanceEditor')?.getBoundingClientRect().height > 0"), "Timeline performance disclosure");
    assert.equal(await click(client, '.timelinePerformanceEditor [data-timeline-bank] > summary'), true);
    assert.equal(await click(client, '.timelinePerformanceEditor .timelineGuideAudioEditor > summary'), true);
    assert.equal(await click(client, '.timelinePerformanceEditor .timelinePhaseEditor > summary'), true);
    await sleep(100);
    const targetProof = [];
    for (const selector of ['.timelinePerformanceEditor [data-timeline-bank]', '.timelinePerformanceEditor .timelineGuideAudioEditor', '.timelinePerformanceEditor .timelinePhaseEditor', '.timelineOperatorBar > .timelineLoopControls']) {
      targetProof.push(await evaluate(client, `(() => {
        const scope = document.querySelector(${JSON.stringify(selector)});
        if (!(scope instanceof HTMLElement)) return null;
        scope.scrollIntoView({ block: 'center' });
        const rect = scope.getBoundingClientRect();
        const controls = [...scope.querySelectorAll('button, input, select')].filter((element) => {
          const bounds = element.getBoundingClientRect();
          return bounds.width > 0 && bounds.height > 0;
        });
        return { rect: [rect.width, rect.height], count: controls.length, short: controls.filter((element) => element.getBoundingClientRect().height < 43.5).length };
      })()`));
      await sleep(50);
    }
    const videoSelected = await evaluate(client, `(() => {
      const clip = document.querySelector('[data-timeline-video-clip-id="800"]');
      if (!(clip instanceof Element)) return false;
      clip.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
      clip.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true, clientX: 240, clientY: 180 }));
      return true;
    })()`);
    assert.equal(videoSelected, true);
    await sleep(80);
    const state = await measure(client);
    assert.ok(state.rect[0] > 0 && state.rect[1] > 0, "Timeline surface has visible nonzero geometry");
    assert.deepEqual([state.bankOpen, state.bankItems, state.bankActive, state.followLegend], [true, 2, 1, true]);
    assert.match(state.followState, /TRANS 50%/);
    assert.equal(state.phaseEditorOpen, true);
    assert.deepEqual([state.guideAudioOpen, state.guideAudioControls, state.guideAudioState], [true, 3, 'Ready']);
    assert.deepEqual(state.phaseLabels, ["Intro", "Verse", "Chorus"]);
    assert.deepEqual([state.guidePressed, state.loopPressed, state.loopState, state.loopScaleControls], ["true", "true", "LOOP ×2", 2]);
    assert.deepEqual([state.videoClips, state.audioClips], [1, 2]);
    assert.deepEqual([state.selectedVideo, state.selectedAudio], [1, 1], "selecting either member selects the linked A/V group");
    assert.deepEqual([state.groupEnabled, state.ungroupEnabled, state.duplicateEnabled, state.nudgeEarlierEnabled, state.nudgeLaterEnabled, state.quantizeEnabled, state.deleteEnabled], [false, true, true, true, true, true, true], "the context menu exposes linked-group duplicate/nudge/quantize/release/delete and prevents nested grouping");
    assert.ok(state.itemMenuRect && state.itemMenuRect[0] >= 0 && state.itemMenuRect[1] >= 0 && state.itemMenuRect[2] <= viewport.width && state.itemMenuRect[3] <= viewport.height, `Timeline group menu stays inside ${viewport.width}x${viewport.height}`);
    assert.equal(state.menuShortTargets, 0, "Timeline group context actions preserve 44px targets");
    assert.equal(await evaluate(client, `(() => {
      const button = [...document.querySelectorAll('.timelineItemContextMenu button')]
        .find((candidate) => candidate.textContent?.trim() === 'Delete selected');
      if (!(button instanceof HTMLButtonElement) || button.disabled) return false;
      button.click();
      return true;
    })()`), true);
    const deleteDialog = await waitFor(() => evaluate(client, `(() => {
      const dialog = document.querySelector('[data-timeline-item-remove-dialog]');
      if (!(dialog instanceof HTMLDialogElement) || !dialog.open) return null;
      const rect = dialog.getBoundingClientRect();
      const buttons = [...dialog.querySelectorAll('button')].filter((button) => button.getBoundingClientRect().height > 0);
      return {
        rect: [rect.width, rect.height],
        text: dialog.textContent ?? '',
        short: buttons.filter((button) => button.getBoundingClientRect().height < 43.5).length,
      };
    })()`), "linked delete confirmation");
    assert.ok(deleteDialog.rect[0] > 0 && deleteDialog.rect[1] > 0, "linked delete alert dialog is visibly mounted");
    assert.match(deleteDialog.text, /2\s+selected Timeline item\(s\)/);
    assert.match(deleteDialog.text, /Linked group members/);
    assert.equal(deleteDialog.short, 0, "linked delete alert dialog preserves 44px actions");
    await evaluate(client, "document.querySelector('[data-timeline-item-remove-dialog]')?.close()");
    assert.equal(targetProof.every((proof) => proof?.rect[0] > 0 && proof?.rect[1] > 0 && proof.count > 0 && proof.short === 0), true, `Timeline bank, Guide audio, Phase, and loop controls preserve 44px targets at ${viewport.width}x${viewport.height}: ${JSON.stringify(targetProof)}`);
    assert.equal(state.shortTargets, 0, `Visible Timeline performance controls preserve 44px targets at ${viewport.width}x${viewport.height}`);
    assert.equal(state.fixedOuter, true, `Timeline disclosures keep app/document outer scroll fixed at ${viewport.width}x${viewport.height}`);
    console.log(`${viewport.width}x${viewport.height}: phases=${state.phaseLabels.join('/')} bank=${state.bankItems} media=${state.videoClips}+${state.audioClips} selected=${state.selectedVideo}+${state.selectedAudio}`);
  }
} finally {
  client?.close();
  await stopChild(browser);
  await stopChild(vite);
  if (profileDir) await rm(profileDir, { recursive: true, force: true });
}
