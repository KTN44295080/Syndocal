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
  { width: 860, height: 520 },
  { width: 1280, height: 720 },
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
const dragTimelineResize = async (client, selector, handleSelector, edge, isolate = false) => {
  assert.equal(await evaluate(client, `(() => {
    const element = document.querySelector(${JSON.stringify(selector)});
    if (!(element instanceof Element)) return false;
    element.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
    return true;
  })()`), true, `select ${selector}`);
  await sleep(30);
  const point = await evaluate(client, `(() => {
    const element = document.querySelector(${JSON.stringify(selector)});
    const handle = element?.querySelector(${JSON.stringify(handleSelector)});
    if (!(handle instanceof Element)) return null;
    const rect = handle.getBoundingClientRect();
    return { x: rect.left + rect.width / 2, y: rect.top + rect.height / 2, width: rect.width, height: rect.height };
  })()`);
  assert.ok(point && point.width > 0 && point.height > 0, `visible ${edge} resize handle for ${selector}`);
  const modifiers = isolate ? 1 : 0;
  const targetX = point.x + (edge === "start" ? 24 : 28);
  await client.send("Input.dispatchMouseEvent", { type: "mouseMoved", x: point.x, y: point.y, modifiers });
  await client.send("Input.dispatchMouseEvent", { type: "mousePressed", x: point.x, y: point.y, modifiers, button: "left", buttons: 1, clickCount: 1 });
  await client.send("Input.dispatchMouseEvent", { type: "mouseMoved", x: targetX, y: point.y, modifiers, button: "left", buttons: 1 });
  await client.send("Input.dispatchMouseEvent", { type: "mouseReleased", x: targetX, y: point.y, modifiers, button: "left", buttons: 0, clickCount: 1 });
  await sleep(40);
};
const dragTimelineItemToLane = async (client, selector, targetLayerId, isolate = false) => {
  const points = await evaluate(client, `(() => {
    const item = document.querySelector(${JSON.stringify(selector)});
    const target = document.querySelector('[data-timeline-layer-gutter][data-timeline-layer-id="${targetLayerId}"]');
    if (!(item instanceof Element) || !(target instanceof Element)) return null;
    const sourceLayerId = item.getAttribute('data-timeline-layer-id');
    const sourceLane = document.querySelector('[data-timeline-layer-gutter][data-timeline-layer-id="' + sourceLayerId + '"]');
    if (!(sourceLane instanceof Element)) return null;
    const hitSurface = item.querySelector('.timelineVideoClipBody, .timelineAudioClipBody, .timelineSceneBlockBody, :scope > rect') ?? item;
    const itemRect = hitSurface.getBoundingClientRect();
    const targetRect = target.getBoundingClientRect();
    const safeInset = hitSurface.matches('.timelineSceneBlockBody') ? 24 : 8;
    const sourceYs = item.matches('.timelineAutomationRange')
      ? [itemRect.top + itemRect.height / 2]
      : [itemRect.top + Math.min(4, Math.max(1, itemRect.height / 4))];
    const excluded = '.timelineAutomationHandle, .timelineAutomationKeyframeGroup, .timelineSceneBlockResizeHandle, .timelineSceneBlockFadeHandle, .timelineVideoClipResizeHandle, .timelineVideoClipFadeHandle, .timelineAudioClipResizeHandle, .timelineAudioClipFadeHandle';
    const sourceXs = [
      itemRect.left + Math.min(safeInset, Math.max(1, itemRect.width / 2)),
      itemRect.left + itemRect.width * 0.25,
      itemRect.left + itemRect.width * 0.5,
      itemRect.left + itemRect.width * 0.75,
    ];
    const preferredSourcePoints = sourceYs.flatMap((candidateY) => sourceXs.map((candidateX) => ({ x: candidateX, y: candidateY })));
    const automationSourcePoints = [];
    if (item.matches('.timelineAutomationRange')) {
      for (let candidateY = itemRect.top + 0.25; candidateY < itemRect.bottom; candidateY += 0.25) {
        for (let candidateX = itemRect.left + 3; candidateX < itemRect.right - 3; candidateX += 1) {
          automationSourcePoints.push({ x: candidateX, y: candidateY });
        }
      }
    }
    const sourcePoint = [...preferredSourcePoints, ...automationSourcePoints].find((candidate) => {
      const hit = document.elementFromPoint(candidate.x, candidate.y);
      return hit && item.contains(hit) && !hit.closest(excluded);
    });
    const resolvedSource = sourcePoint ?? { x: itemRect.left + itemRect.width / 2, y: itemRect.top + itemRect.height / 2 };
    return {
      source: resolvedSource,
      target: { x: Math.min(window.innerWidth - 8, resolvedSource.x + 12), y: targetRect.top + targetRect.height / 2 },
      sourceSize: [itemRect.width, itemRect.height],
      targetSize: [targetRect.width, targetRect.height],
    };
  })()`);
  assert.ok(points && points.sourceSize[0] > 0 && points.sourceSize[1] > 0 && points.targetSize[0] > 0 && points.targetSize[1] > 0, `visible lane drag geometry for ${selector}`);
  const modifiers = isolate ? 1 : 0;
  await client.send("Input.dispatchMouseEvent", { type: "mouseMoved", ...points.source, modifiers });
  await client.send("Input.dispatchMouseEvent", { type: "mousePressed", ...points.source, modifiers, button: "left", buttons: 1, clickCount: 1 });
  await client.send("Input.dispatchMouseEvent", { type: "mouseMoved", ...points.target, modifiers, button: "left", buttons: 1 });
  await client.send("Input.dispatchMouseEvent", { type: "mouseReleased", ...points.target, modifiers, button: "left", buttons: 0, clickCount: 1 });
  await sleep(100);
};
const timelineLaneItemKey = (item) => `${item.kind}:${item.event_id ?? item.clip_id ?? item.automation_id}`;
const mountedTimelineItemTimes = (client, item) => {
  const selector = item.kind === "lighting_event"
    ? `[data-timeline-event-id="${item.event_id}"]`
    : item.kind === "video_clip"
      ? `[data-timeline-video-clip-id="${item.clip_id}"]`
      : item.kind === "audio_clip"
        ? `[data-timeline-audio-clip-id="${item.clip_id}"]`
        : `[data-timeline-automation-kind="${item.kind === "lighting_automation" ? "lighting" : "video"}"][data-timeline-automation-id="${item.automation_id}"]`;
  const attribute = item.kind === "lighting_event"
    ? "data-timeline-start-ms"
    : item.kind === "video_clip"
      ? "data-timeline-video-start-ms"
      : item.kind === "audio_clip"
        ? "data-timeline-audio-start-ms"
        : "data-timeline-keyframe-times";
  return evaluate(client, `(() => {
    const value = document.querySelector(${JSON.stringify(selector)})?.getAttribute(${JSON.stringify(attribute)});
    return value === null || value === undefined || value === '' ? null : value.split(',').map(Number);
  })()`);
};
const assertLaneTimeShiftMounted = async (client, callIndex, call, label) => {
  assert.equal(Number.isInteger(call.delta_ms), true, `${label} carries an integer delta`);
  assert.equal(Math.sign(call.delta_ms), 1, `${label} carries the positive signed pointer delta`);
  const applied = await evaluate(client, `structuredClone(window.__syndocalTimelineLaneMoveFixtureApplied[${callIndex}])`);
  const keys = call.items.map(timelineLaneItemKey);
  for (const key of keys) {
    assert.deepEqual(
      applied.after[key],
      applied.before[key].map((timeMs) => timeMs + call.delta_ms),
      `${label} shifts every ${key} authored time by the same delta`,
    );
  }
  const baselineKey = keys[0];
  for (const key of keys.slice(1)) {
    assert.equal(
      applied.after[key][0] - applied.after[baselineKey][0],
      applied.before[key][0] - applied.before[baselineKey][0],
      `${label} preserves the ${key} relative offset`,
    );
  }
  await waitFor(async () => {
    const mounted = await Promise.all(call.items.map((item) => mountedTimelineItemTimes(client, item)));
    return mounted.every((times, index) => JSON.stringify(times) === JSON.stringify(applied.after[keys[index]]));
  }, `${label} ACK time state`);
  for (const item of call.items) {
    assert.deepEqual(
      await mountedTimelineItemTimes(client, item),
      applied.after[timelineLaneItemKey(item)],
      `${label} mounts authoritative ${timelineLaneItemKey(item)} times`,
    );
  }
};
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
  const shortTargets = [...document.querySelectorAll('.timelinePerformanceEditor button, .timelinePerformanceEditor input, .timelinePerformanceEditor select, .timelineOperatorBar > .timelineLoopControls button, .timelineOperatorBar [data-timeline-guide], [data-timeline-follow-operator-abort]')].filter((element) => {
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
    followRuntimeDetails: bank?.querySelectorAll('[data-timeline-follow-runtime-details]').length ?? 0,
    followRuntimeBadges: document.querySelectorAll('[data-timeline-follow-runtime-badge]').length,
    operatorFollowState: document.querySelector('[data-timeline-follow-operator-badge]')?.textContent?.trim() ?? '',
    operatorFollowAbort: document.querySelectorAll('[data-timeline-follow-operator-abort]').length,
    followAbortButtons: document.querySelectorAll('[data-timeline-follow-abort]').length,
    followAbortShortTargets: [...document.querySelectorAll('[data-timeline-follow-abort], [data-timeline-follow-operator-abort]')].filter((element) => {
      const bounds = element.getBoundingClientRect();
      return bounds.width > 0 && bounds.height > 0 && (bounds.width < 43.5 || bounds.height < 43.5);
    }).length,
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
    videoResizeHandles: root.querySelectorAll('[data-timeline-video-resize]').length,
    audioResizeHandles: root.querySelectorAll('[data-timeline-audio-resize]').length,
    groupEnabled: Boolean([...root.querySelectorAll('.timelineItemContextMenu button')].find((button) => button.textContent?.trim() === 'Group selected' && !button.disabled)),
    ungroupEnabled: Boolean([...root.querySelectorAll('.timelineItemContextMenu button')].find((button) => button.textContent?.trim() === 'Ungroup' && !button.disabled)),
    copyEnabled: Boolean([...root.querySelectorAll('.timelineItemContextMenu button')].find((button) => button.textContent?.trim() === 'Copy selected' && !button.disabled)),
    pasteEnabled: Boolean([...root.querySelectorAll('.timelineItemContextMenu button')].find((button) => button.textContent?.trim() === 'Paste at playhead' && !button.disabled)),
    duplicateEnabled: Boolean([...root.querySelectorAll('.timelineItemContextMenu button')].find((button) => button.textContent?.trim() === 'Duplicate selected' && !button.disabled)),
    splitEnabled: Boolean([...root.querySelectorAll('.timelineItemContextMenu button')].find((button) => button.textContent?.trim() === 'Split at playhead' && !button.disabled)),
    nudgeEarlierEnabled: Boolean([...root.querySelectorAll('.timelineItemContextMenu button')].find((button) => button.textContent?.trim() === 'Nudge earlier' && !button.disabled)),
    nudgeLaterEnabled: Boolean([...root.querySelectorAll('.timelineItemContextMenu button')].find((button) => button.textContent?.trim() === 'Nudge later' && !button.disabled)),
    rippleEarlierEnabled: Boolean([...root.querySelectorAll('.timelineItemContextMenu button')].find((button) => button.textContent?.trim() === 'Ripple earlier' && !button.disabled)),
    rippleLaterEnabled: Boolean([...root.querySelectorAll('.timelineItemContextMenu button')].find((button) => button.textContent?.trim() === 'Ripple later' && !button.disabled)),
    quantizeEnabled: Boolean([...root.querySelectorAll('.timelineItemContextMenu button')].find((button) => button.textContent?.trim() === 'Quantize to grid' && !button.disabled)),
    trimStartEnabled: Boolean([...root.querySelectorAll('.timelineItemContextMenu button')].find((button) => button.textContent?.trim() === 'Trim start to playhead' && !button.disabled)),
    trimEndEnabled: Boolean([...root.querySelectorAll('.timelineItemContextMenu button')].find((button) => button.textContent?.trim() === 'Trim end to playhead' && !button.disabled)),
    deleteEnabled: Boolean([...root.querySelectorAll('.timelineItemContextMenu button')].find((button) => button.textContent?.trim() === 'Delete selected' && !button.disabled)),
    itemMenuRect: itemMenu instanceof HTMLElement ? (() => { const bounds = itemMenu.getBoundingClientRect(); return [bounds.left, bounds.top, bounds.right, bounds.bottom]; })() : null,
    itemMenuBottomReachable: itemMenu instanceof HTMLElement ? (() => {
      itemMenu.scrollTop = itemMenu.scrollHeight;
      const bounds = itemMenu.getBoundingClientRect();
      const close = [...itemMenu.querySelectorAll('button')].find((button) => button.textContent?.trim() === 'Close');
      if (!(close instanceof HTMLButtonElement)) return false;
      const closeBounds = close.getBoundingClientRect();
      return closeBounds.top >= bounds.top - 0.5 && closeBounds.bottom <= bounds.bottom + 0.5;
    })() : false,
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
    const singleMemberSelected = await evaluate(client, `(() => {
      const clip = document.querySelector('[data-timeline-audio-clip-id="700"]');
      if (!(clip instanceof Element)) return false;
      clip.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true, altKey: true }));
      return true;
    })()`);
    assert.equal(singleMemberSelected, true);
    await sleep(50);
    const singleMemberState = await measure(client);
    assert.deepEqual(
      [singleMemberState.selectedVideo, singleMemberState.selectedAudio],
      [0, 1],
      "Alt selection temporarily isolates one linked A/V member",
    );
    await evaluate(client, `(() => {
      window.__syndocalTimelineSplitFixtureCalls = [];
      window.__syndocalTimelineSplitFixture = async (request, timelineBank, activeTimelineId) => {
        window.__syndocalTimelineSplitFixtureCalls.push(structuredClone(request));
        const bank = structuredClone(timelineBank);
        const active = bank.find((timeline) => timeline.id === activeTimelineId);
        const source = active.audio_clips.find((clip) => clip.id === 700);
        active.audio_clips.push({ ...source, id: 1700, start_ms: request.boundary_ms, duration_ms: 600, fade_in_ms: 0 });
        return {
          authoring: {},
          timeline_bank: bank,
          active_timeline_id: activeTimelineId,
          selected_items: [{ kind: 'audio_clip', clip_id: 1700 }],
          mutation: {},
        };
      };
      const clip = document.querySelector('[data-timeline-audio-clip-id="700"]');
      clip?.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true, clientX: 240, clientY: 180 }));
    })()`);
    await sleep(50);
    assert.equal(await evaluate(client, `(() => {
      const button = document.querySelector('[data-timeline-split-action]');
      if (!(button instanceof HTMLButtonElement) || button.disabled) return false;
      button.click();
      return true;
    })()`), true);
    await waitFor(
      () => evaluate(client, "window.__syndocalTimelineSplitFixtureCalls?.length === 1"),
      "isolated Timeline Split fixture dispatch",
    );
    await sleep(80);
    const isolatedSplitProof = await evaluate(client, `(() => ({
      call: structuredClone(window.__syndocalTimelineSplitFixtureCalls[0]),
      focusedAudioId: document.activeElement?.getAttribute('data-timeline-audio-clip-id') ?? '',
    }))()`);
    assert.deepEqual(isolatedSplitProof, {
      call: {
        kind: "split_items",
        items: [{ kind: "audio_clip", clip_id: 700 }],
        primary: { kind: "audio_clip", clip_id: 700 },
        boundary_ms: 1_000,
        isolate: true,
      },
      focusedAudioId: "1700",
    }, "Alt-isolated Split mounts a fresh right item, sends only the chosen member, and restores fresh focus");
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
    assert.match(state.followState, /transitioning 50%/);
    assert.deepEqual(
      [state.followRuntimeDetails, state.followRuntimeBadges >= 1, state.operatorFollowState, state.operatorFollowAbort, state.followAbortButtons >= 1, state.followAbortShortTargets],
      [1, true, "Follow: transitioning 50%", 1, true, 0],
      "Visible Operator and Performance Timeline surfaces expose runtime Follow truth with 44px abort targets",
    );
    const abortFocusRetained = await evaluate(client, `(() => {
      const button = document.querySelector('.timelinePerformanceEditor [data-timeline-follow-abort]');
      if (!(button instanceof HTMLButtonElement)) return false;
      button.focus();
      button.click();
      return true;
    })()`);
    assert.equal(abortFocusRetained, true, "Timeline Follow abort control is keyboard-focusable");
    await sleep(50);
    assert.equal(
      await evaluate(client, "document.activeElement?.matches?.('[data-timeline-follow-abort]') === true"),
      true,
      "Timeline Follow abort retains focus after its bounded runtime-only result",
    );
    assert.equal(state.phaseEditorOpen, true);
    assert.deepEqual([state.guideAudioOpen, state.guideAudioControls, state.guideAudioState], [true, 3, 'Ready']);
    assert.deepEqual(state.phaseLabels, ["Intro", "Verse", "Chorus"]);
    assert.deepEqual([state.guidePressed, state.loopPressed, state.loopState, state.loopScaleControls], ["true", "true", "LOOP ×2", 2]);
    assert.deepEqual([state.videoClips, state.audioClips], [1, 3], "isolated Split mounts one fresh Audio right-side block");
    assert.deepEqual([state.selectedVideo, state.selectedAudio], [1, 1], "selecting either member selects the linked A/V group");
    assert.deepEqual(
      [state.videoResizeHandles, state.audioResizeHandles],
      [2, 2],
      "selected linked A/V members expose both direct trim edges",
    );
    assert.deepEqual([state.groupEnabled, state.ungroupEnabled, state.copyEnabled, state.pasteEnabled, state.duplicateEnabled, state.splitEnabled, state.nudgeEarlierEnabled, state.nudgeLaterEnabled, state.rippleEarlierEnabled, state.rippleLaterEnabled, state.quantizeEnabled, state.trimStartEnabled, state.trimEndEnabled, state.deleteEnabled], [false, true, true, false, true, true, true, true, true, true, true, true, true, true], "the context menu exposes linked-group copy/duplicate/split/nudge/ripple/quantize/trim/release/delete and disables an empty clipboard");
    assert.ok(state.itemMenuRect && state.itemMenuRect[0] >= 0 && state.itemMenuRect[1] >= 0 && state.itemMenuRect[2] <= viewport.width && state.itemMenuRect[3] <= viewport.height, `Timeline group menu stays inside ${viewport.width}x${viewport.height}`);
    assert.equal(state.itemMenuBottomReachable, true, "the internally scrolling Timeline item menu reaches its final action");
    assert.equal(state.menuShortTargets, 0, "Timeline group context actions preserve 44px targets");
    await evaluate(client, `(() => {
      window.__syndocalTimelineSplitFixtureCalls = [];
      window.__syndocalTimelineSplitFixture = async (request, timelineBank, activeTimelineId) => {
        window.__syndocalTimelineSplitFixtureCalls.push(structuredClone(request));
        const bank = structuredClone(timelineBank);
        const active = bank.find((timeline) => timeline.id === activeTimelineId);
        const sourceVideo = active.video_clips.find((clip) => clip.id === 800);
        const sourceAudio = active.audio_clips.find((clip) => clip.id === 700);
        active.video_clips.push({ ...sourceVideo, id: 1800, start_ms: request.boundary_ms, duration_ms: 600, fade_in_ms: 0 });
        active.audio_clips.push({ ...sourceAudio, id: 1801, start_ms: request.boundary_ms, duration_ms: 600, fade_in_ms: 0 });
        active.item_groups.push({ id: 1820, members: [{ kind: 'video_clip', clip_id: 1800 }, { kind: 'audio_clip', clip_id: 1801 }] });
        return {
          authoring: {},
          timeline_bank: bank,
          active_timeline_id: activeTimelineId,
          selected_items: [{ kind: 'video_clip', clip_id: 1800 }, { kind: 'audio_clip', clip_id: 1801 }],
          mutation: {},
        };
      };
    })()`);
    assert.equal(await evaluate(client, `(() => {
      const button = document.querySelector('[data-timeline-split-action]');
      if (!(button instanceof HTMLButtonElement) || button.disabled) return false;
      button.click();
      return true;
    })()`), true);
    await waitFor(
      () => evaluate(client, "window.__syndocalTimelineSplitFixtureCalls?.length === 1"),
      "Timeline Split fixture dispatch",
    );
    await sleep(80);
    const splitProof = await evaluate(client, `(() => ({
      call: structuredClone(window.__syndocalTimelineSplitFixtureCalls[0]),
      menuClosed: document.querySelector('.timelineItemContextMenu') === null,
      focusedVideoId: document.activeElement?.getAttribute('data-timeline-video-clip-id') ?? '',
      freshVideoMounted: document.querySelectorAll('[data-timeline-video-clip-id="1800"]').length,
      freshAudioMounted: document.querySelectorAll('[data-timeline-audio-clip-id="1801"]').length,
      selectedVideo: document.querySelectorAll('.timelineVideoClip.selected').length,
      selectedAudio: document.querySelectorAll('.timelineAudioClip.selected').length,
    }))()`);
    assert.deepEqual(splitProof.call, {
      kind: "split_items",
      items: [{ kind: "video_clip", clip_id: 800 }, { kind: "audio_clip", clip_id: 700 }],
      primary: { kind: "video_clip", clip_id: 800 },
      boundary_ms: 1_000,
      isolate: false,
    }, "real Split menu action dispatches the exact linked selection, primary, playhead, and isolate flag");
    assert.deepEqual(
      [splitProof.menuClosed, splitProof.focusedVideoId, splitProof.freshVideoMounted, splitProof.freshAudioMounted, splitProof.selectedVideo, splitProof.selectedAudio],
      [true, "1800", 1, 1, 1, 1],
      "Split mounts fresh right IDs, closes the menu, restores their linked selection, and focuses the fresh primary item",
    );
    const additionalSplitCases = [
      { selector: '.timelineMarker[data-timeline-event-id]', kind: 'lighting_event', field: 'event_id', attribute: 'data-timeline-event-id' },
      { selector: '[data-timeline-automation-kind="lighting"][data-timeline-automation-id]', kind: 'lighting_automation', field: 'automation_id', attribute: 'data-timeline-automation-id' },
      { selector: '[data-timeline-automation-kind="video"][data-timeline-automation-id]', kind: 'video_automation', field: 'automation_id', attribute: 'data-timeline-automation-id' },
    ];
    for (const splitCase of additionalSplitCases) {
      const opened = await evaluate(client, `(() => {
        const target = document.querySelector(${JSON.stringify(splitCase.selector)});
        if (!(target instanceof Element)) return null;
        const id = Number(target.getAttribute(${JSON.stringify(splitCase.attribute)}));
        const item = { kind: ${JSON.stringify(splitCase.kind)}, [${JSON.stringify(splitCase.field)}]: id };
        window.__syndocalTimelineSplitFixtureCalls = [];
        window.__syndocalTimelineSplitFixture = async (request, timelineBank, activeTimelineId) => {
          window.__syndocalTimelineSplitFixtureCalls.push(structuredClone(request));
          return {
            authoring: {},
            timeline_bank: structuredClone(timelineBank),
            active_timeline_id: activeTimelineId,
            selected_items: [structuredClone(item)],
            mutation: {},
          };
        };
        target.dispatchEvent(new MouseEvent('contextmenu', {
          bubbles: true,
          cancelable: true,
          altKey: true,
          clientX: 260,
          clientY: 190,
        }));
        return item;
      })()`);
      assert.ok(opened, `${splitCase.kind} opens the production Timeline item menu`);
      await sleep(40);
      assert.equal(await evaluate(client, `(() => {
        const button = document.querySelector('[data-timeline-split-action]');
        if (!(button instanceof HTMLButtonElement) || button.disabled) return false;
        button.click();
        return true;
      })()`), true);
      await waitFor(
        () => evaluate(client, "window.__syndocalTimelineSplitFixtureCalls?.length === 1"),
        `${splitCase.kind} Timeline Split fixture dispatch`,
      );
      await sleep(80);
      const proof = await evaluate(client, `(() => ({
        call: structuredClone(window.__syndocalTimelineSplitFixtureCalls[0]),
        focused: document.activeElement?.getAttribute(${JSON.stringify(splitCase.attribute)}) ?? '',
      }))()`);
      assert.deepEqual(proof.call.items, [opened], `${splitCase.kind} Split dispatches its standalone item`);
      assert.deepEqual(proof.call.primary, opened, `${splitCase.kind} Split preserves its primary item`);
      assert.equal(proof.call.isolate, true, `${splitCase.kind} Split preserves Alt isolation`);
      assert.equal(proof.focused, String(opened[splitCase.field]), `${splitCase.kind} Split restores item focus`);
    }
    assert.equal(await evaluate(client, `(() => {
      const clip = document.querySelector('[data-timeline-video-clip-id="800"]');
      if (!(clip instanceof Element)) return false;
      clip.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true, clientX: 240, clientY: 180 }));
      return true;
    })()`), true);
    await sleep(50);
    assert.equal(await evaluate(client, `(() => {
      const button = [...document.querySelectorAll('.timelineItemContextMenu button')]
        .find((candidate) => candidate.textContent?.trim() === 'Copy selected');
      if (!(button instanceof HTMLButtonElement) || button.disabled) return false;
      button.click();
      return true;
    })()`), true);
    await sleep(50);
    assert.equal(
      await evaluate(client, "document.activeElement?.getAttribute('data-timeline-video-clip-id') ?? ''"),
      "800",
      "closing the Timeline item menu returns focus to the invoking clip",
    );
    assert.equal(await evaluate(client, `(() => {
      const clip = document.querySelector('[data-timeline-video-clip-id="800"]');
      if (!(clip instanceof Element)) return false;
      clip.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true, clientX: 240, clientY: 180 }));
      return true;
    })()`), true);
    await sleep(50);
    assert.equal((await measure(client)).pasteEnabled, true, "copy binds a pasteable linked selection to this Timeline");
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
    if (viewport.width === 1280) {
      await evaluate(client, `(() => {
        window.__syndocalTimelineLaneMoveFixtureCalls = [];
        window.__syndocalTimelineLaneMoveFixtureApplied = [];
        const key = (item) => item.kind + ':' + (item.event_id ?? item.clip_id ?? item.automation_id);
        window.__syndocalTimelineLaneMoveFixture = async (request, timelineBank, activeTimelineId) => {
          window.__syndocalTimelineLaneMoveFixtureCalls.push(structuredClone(request));
          const bank = structuredClone(timelineBank);
          const active = bank.find((timeline) => timeline.id === activeTimelineId);
          if (window.__syndocalTimelineLaneMoveFixtureCalls.length === 1) {
            const event = active.events[0];
            const group = active.item_groups.find((candidate) => candidate.members.some((member) => member.kind === 'video_clip' && member.clip_id === 800));
            window.__syndocalTimelineLaneMoveEventId = event.id;
            event.layer_id = 13;
            event.time_ms = 1_000;
            active.video_clips.find((clip) => clip.id === 800).layer_id = 15;
            active.video_clips.find((clip) => clip.id === 800).start_ms = 5_000;
            active.audio_clips.find((clip) => clip.id === 700).layer_id = 11;
            active.audio_clips.find((clip) => clip.id === 700).start_ms = 3_500;
            const lightingAutomation = active.automations.find((automation) => automation.id === 1);
            lightingAutomation.timeline_layer_id = 13;
            lightingAutomation.keyframes = lightingAutomation.keyframes.map((frame, index) => ({ ...frame, time_ms: 1_000 + index * 1_000 }));
            const videoAutomation = active.video_automations.find((automation) => automation.id === 2);
            videoAutomation.timeline_layer_id = 15;
            videoAutomation.keyframes = videoAutomation.keyframes.map((frame, index) => ({ ...frame, time_ms: 1_500 + index * 1_000 }));
            group.members = [
              { kind: 'lighting_event', event_id: event.id },
              { kind: 'video_clip', clip_id: 800 },
              { kind: 'audio_clip', clip_id: 700 },
              { kind: 'lighting_automation', automation_id: 1 },
              { kind: 'video_automation', automation_id: 2 },
            ];
          }
          const captureTimes = () => ({
            [key({ kind: 'lighting_event', event_id: window.__syndocalTimelineLaneMoveEventId })]: [active.events.find((event) => event.id === window.__syndocalTimelineLaneMoveEventId).time_ms],
            [key({ kind: 'video_clip', clip_id: 800 })]: [active.video_clips.find((clip) => clip.id === 800).start_ms],
            [key({ kind: 'audio_clip', clip_id: 700 })]: [active.audio_clips.find((clip) => clip.id === 700).start_ms],
            [key({ kind: 'lighting_automation', automation_id: 1 })]: active.automations.find((automation) => automation.id === 1).keyframes.map((frame) => frame.time_ms),
            [key({ kind: 'video_automation', automation_id: 2 })]: active.video_automations.find((automation) => automation.id === 2).keyframes.map((frame) => frame.time_ms),
          });
          const beforeTimes = captureTimes();
          const shift = (value) => Math.max(0, value + request.delta_ms);
          for (const item of request.items) {
            if (item.kind === 'lighting_event') {
              const event = active.events.find((candidate) => candidate.id === item.event_id);
              if (event) event.time_ms = shift(event.time_ms);
            } else if (item.kind === 'video_clip') {
              const clip = active.video_clips.find((candidate) => candidate.id === item.clip_id);
              if (clip) clip.start_ms = shift(clip.start_ms);
            } else if (item.kind === 'audio_clip') {
              const clip = active.audio_clips.find((candidate) => candidate.id === item.clip_id);
              if (clip) clip.start_ms = shift(clip.start_ms);
            } else {
              const collection = item.kind === 'lighting_automation' ? active.automations : active.video_automations;
              const automation = collection.find((candidate) => candidate.id === item.automation_id);
              if (automation) automation.keyframes = automation.keyframes.map((frame) => ({ ...frame, time_ms: shift(frame.time_ms) }));
            }
          }
          for (const target of request.lane_targets) {
            const item = target.item;
            if (item.kind === 'lighting_event') {
              const event = active.events.find((candidate) => candidate.id === item.event_id);
              if (event) event.layer_id = target.target_layer_id;
            } else if (item.kind === 'video_clip') {
              const clip = active.video_clips.find((candidate) => candidate.id === item.clip_id);
              if (clip) clip.layer_id = target.target_layer_id;
            } else if (item.kind === 'audio_clip') {
              const clip = active.audio_clips.find((candidate) => candidate.id === item.clip_id);
              if (clip) clip.layer_id = target.target_layer_id;
            } else {
              const collection = item.kind === 'lighting_automation' ? active.automations : active.video_automations;
              const automation = collection.find((candidate) => candidate.id === item.automation_id);
              if (automation) automation.timeline_layer_id = target.target_layer_id;
            }
          }
          window.__syndocalTimelineLaneMoveFixtureApplied.push({ before: beforeTimes, after: captureTimes() });
          return {
            authoring: {}, timeline_bank: bank, active_timeline_id: activeTimelineId,
            selected_items: structuredClone(request.items), mutation: {},
          };
        };
        window.__syndocalTimelineLaneMoveItemKey = key;
      })()`);
      await dragTimelineItemToLane(client, '[data-timeline-video-clip-id="800"]', 15);
      await waitFor(() => evaluate(client, "window.__syndocalTimelineLaneMoveFixtureCalls?.length === 1"), "Video linked lane move");
      const firstLaneCall = await evaluate(client, "structuredClone(window.__syndocalTimelineLaneMoveFixtureCalls[0])");
      assert.equal(firstLaneCall.kind, "move_items_to_lanes");
      assert.deepEqual(firstLaneCall.items, [{ kind: "video_clip", clip_id: 800 }, { kind: "audio_clip", clip_id: 700 }]);
      assert.deepEqual(firstLaneCall.lane_targets, [
        { item: { kind: "video_clip", clip_id: 800 }, target_layer_id: 15 },
        { item: { kind: "audio_clip", clip_id: 700 }, target_layer_id: 11 },
      ]);
      assert.equal(firstLaneCall.isolate, false);
      await assertLaneTimeShiftMounted(client, 0, firstLaneCall, "Video linked lane move");
      await waitFor(
        () => evaluate(client, `document.querySelector('.timelineMarker[data-timeline-event-id="' + window.__syndocalTimelineLaneMoveEventId + '"]')?.getAttribute('data-timeline-layer-id') === '13'`),
        "five-domain lane fixture ACK state",
      );
      const eventSelector = await evaluate(client, `'.timelineMarker[data-timeline-event-id="' + window.__syndocalTimelineLaneMoveEventId + '"]'`);
      const laneMoveEventId = await evaluate(client, "Number(window.__syndocalTimelineLaneMoveEventId)");
      assert.ok(eventSelector, "five-domain lane fixture exposes a Lighting event");
      const pointerCases = [
        [eventSelector, 14, "lighting_event"],
        ['[data-timeline-audio-clip-id="700"]', 11, "audio_clip"],
        ['[data-timeline-automation-kind="lighting"][data-timeline-automation-id="1"]', 12, "lighting_automation"],
        ['[data-timeline-automation-kind="video"][data-timeline-automation-id="2"]', 15, "video_automation"],
      ];
      for (const [selector, targetLayerId, expectedKind] of pointerCases) {
        const beforeCalls = await evaluate(client, "window.__syndocalTimelineLaneMoveFixtureCalls.length");
        await dragTimelineItemToLane(client, selector, targetLayerId);
        await waitFor(
          () => evaluate(client, `window.__syndocalTimelineLaneMoveFixtureCalls?.length === ${beforeCalls + 1}`),
          `${expectedKind} five-domain lane move`,
        );
        const call = await evaluate(client, `structuredClone(window.__syndocalTimelineLaneMoveFixtureCalls[${beforeCalls}])`);
        assert.equal(call.primary.kind, expectedKind);
        assert.equal(call.items.length, 5, `${expectedKind} pointer move expands all five linked domains`);
        assert.equal(call.lane_targets.length, 5, `${expectedKind} pointer move carries five explicit targets`);
        if (expectedKind === "lighting_event") {
          assert.deepEqual(call.lane_targets, [
            { item: { kind: "lighting_event", event_id: laneMoveEventId }, target_layer_id: 14 },
            { item: { kind: "video_clip", clip_id: 800 }, target_layer_id: 14 },
            { item: { kind: "audio_clip", clip_id: 700 }, target_layer_id: 10 },
            { item: { kind: "lighting_automation", automation_id: 1 }, target_layer_id: 12 },
            { item: { kind: "video_automation", automation_id: 2 }, target_layer_id: 14 },
          ], "Scene pointer crosses Lighting to the explicit Video lane and projects the group ordinal");
        }
        await assertLaneTimeShiftMounted(client, beforeCalls, call, `${expectedKind} five-domain lane move`);
      }
      const isolatedHorizontalCases = [
        ['[data-timeline-video-clip-id="800"]', "video_clip"],
        [eventSelector, "lighting_event"],
        ['[data-timeline-audio-clip-id="700"]', "audio_clip"],
        ['[data-timeline-automation-kind="lighting"][data-timeline-automation-id="1"]', "lighting_automation"],
        ['[data-timeline-automation-kind="video"][data-timeline-automation-id="2"]', "video_automation"],
      ];
      for (const [selector, expectedKind] of isolatedHorizontalCases) {
        const currentLayerId = await evaluate(client, `Number(document.querySelector(${JSON.stringify(selector)})?.getAttribute('data-timeline-layer-id'))`);
        assert.equal(Number.isInteger(currentLayerId), true, `${expectedKind} exposes its current resolved lane`);
        const beforeCalls = await evaluate(client, "window.__syndocalTimelineLaneMoveFixtureCalls.length");
        await dragTimelineItemToLane(client, selector, currentLayerId, true);
        await waitFor(
          () => evaluate(client, `window.__syndocalTimelineLaneMoveFixtureCalls?.length === ${beforeCalls + 1}`),
          `${expectedKind} Alt horizontal lane move`,
        );
        const call = await evaluate(client, `structuredClone(window.__syndocalTimelineLaneMoveFixtureCalls[${beforeCalls}])`);
        assert.deepEqual([call.primary.kind, call.items.length, call.lane_targets.length, call.isolate], [expectedKind, 1, 1, true]);
        assert.equal(call.lane_targets[0].target_layer_id, currentLayerId, `${expectedKind} Alt horizontal move retains its resolved lane`);
        assert.notEqual(call.delta_ms, 0, `${expectedKind} Alt horizontal move carries its real pointer delta`);
      }
      const beforeIsolatedPointer = await evaluate(client, "window.__syndocalTimelineLaneMoveFixtureCalls.length");
      await dragTimelineItemToLane(client, '[data-timeline-audio-clip-id="700"]', 10, true);
      await waitFor(() => evaluate(client, `window.__syndocalTimelineLaneMoveFixtureCalls?.length === ${beforeIsolatedPointer + 1}`), "Alt-isolated Audio lane move");
      const isolatedPointerCall = await evaluate(client, `structuredClone(window.__syndocalTimelineLaneMoveFixtureCalls[${beforeIsolatedPointer}])`);
      assert.deepEqual(isolatedPointerCall.items, [{ kind: "audio_clip", clip_id: 700 }]);
      assert.deepEqual(isolatedPointerCall.lane_targets, [{ item: { kind: "audio_clip", clip_id: 700 }, target_layer_id: 10 }]);
      assert.equal(isolatedPointerCall.isolate, true);
      const beforeSceneAudioPointer = await evaluate(client, "window.__syndocalTimelineLaneMoveFixtureCalls.length");
      await dragTimelineItemToLane(client, eventSelector, 10);
      assert.equal(
        await evaluate(client, "window.__syndocalTimelineLaneMoveFixtureCalls.length"),
        beforeSceneAudioPointer,
        "Scene pointer drop on Audio invokes no authoritative command",
      );
      const beforeInvalidPointer = await evaluate(client, "window.__syndocalTimelineLaneMoveFixtureCalls.length");
      await dragTimelineItemToLane(client, '[data-timeline-automation-kind="video"][data-timeline-automation-id="2"]', 10);
      assert.equal(await evaluate(client, "window.__syndocalTimelineLaneMoveFixtureCalls.length"), beforeInvalidPointer, "wrong-kind pointer drop invokes no authoritative command");
      const beforeIsolatedKeyboard = beforeInvalidPointer;
      assert.equal(await evaluate(client, `(() => {
        const item = document.querySelector('[data-timeline-audio-clip-id="700"]');
        if (!(item instanceof Element)) return false;
        item.focus();
        item.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowDown', shiftKey: true, altKey: true, bubbles: true, cancelable: true }));
        return true;
      })()`), true);
      await waitFor(() => evaluate(client, `window.__syndocalTimelineLaneMoveFixtureCalls?.length === ${beforeIsolatedKeyboard + 1}`), "Alt keyboard lane move");
      const isolatedKeyboardCall = await evaluate(client, `structuredClone(window.__syndocalTimelineLaneMoveFixtureCalls[${beforeIsolatedKeyboard}])`);
      assert.deepEqual([isolatedKeyboardCall.primary.kind, isolatedKeyboardCall.isolate, isolatedKeyboardCall.delta_ms], ["audio_clip", true, 0]);
      assert.equal(isolatedKeyboardCall.items.length, 1);
      const beforeGroupKeyboard = beforeIsolatedKeyboard + 1;
      assert.equal(await evaluate(client, `(() => {
        const item = document.querySelector(${JSON.stringify(eventSelector)});
        if (!(item instanceof Element)) return false;
        item.focus();
        item.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowUp', shiftKey: true, bubbles: true, cancelable: true }));
        return true;
      })()`), true);
      await waitFor(() => evaluate(client, `window.__syndocalTimelineLaneMoveFixtureCalls?.length === ${beforeGroupKeyboard + 1}`), "group keyboard lane move");
      const keyboardGroupCall = await evaluate(client, `structuredClone(window.__syndocalTimelineLaneMoveFixtureCalls[${beforeGroupKeyboard}])`);
      assert.deepEqual([keyboardGroupCall.primary.kind, keyboardGroupCall.items.length, keyboardGroupCall.delta_ms], ["lighting_event", 5, 0]);
      const beforeCrossKindKeyboard = beforeGroupKeyboard + 1;
      assert.equal(await evaluate(client, `(() => {
        const item = document.querySelector(${JSON.stringify(eventSelector)});
        if (!(item instanceof Element)) return false;
        item.focus();
        item.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowUp', shiftKey: true, bubbles: true, cancelable: true }));
        return true;
      })()`), true);
      await waitFor(() => evaluate(client, `window.__syndocalTimelineLaneMoveFixtureCalls?.length === ${beforeCrossKindKeyboard + 1}`), "Scene Video-to-Lighting keyboard lane move");
      const crossKindKeyboardCall = await evaluate(client, `structuredClone(window.__syndocalTimelineLaneMoveFixtureCalls[${beforeCrossKindKeyboard}])`);
      assert.deepEqual(
        [crossKindKeyboardCall.primary.kind, crossKindKeyboardCall.lane_targets[0].target_layer_id, crossKindKeyboardCall.items.length, crossKindKeyboardCall.delta_ms],
        ["lighting_event", 13, 5, 0],
        "Scene keyboard navigation crosses the canonical Video-to-Lighting boundary",
      );
      const beforeContextMove = beforeCrossKindKeyboard + 1;
      assert.equal(await evaluate(client, `(() => {
        const item = document.querySelector('[data-timeline-automation-kind="video"][data-timeline-automation-id="2"]');
        if (!(item instanceof Element)) return false;
        item.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true, clientX: 300, clientY: 220 }));
        return true;
      })()`), true);
      await sleep(50);
      assert.equal(await evaluate(client, `(() => {
        const button = [...document.querySelectorAll('.timelineItemContextMenu button')]
          .find((candidate) => candidate.textContent?.trim() === 'Move lane up');
        if (!(button instanceof HTMLButtonElement) || button.disabled) return false;
        button.click();
        return true;
      })()`), true);
      await waitFor(() => evaluate(client, `window.__syndocalTimelineLaneMoveFixtureCalls?.length === ${beforeContextMove + 1}`), "context menu lane move");
      await sleep(100);
      const laneMoveFinal = await evaluate(client, `(() => ({
        call: structuredClone(window.__syndocalTimelineLaneMoveFixtureCalls[${beforeContextMove}]),
        videoLane: Number(document.querySelector('[data-timeline-video-clip-id="800"]')?.getAttribute('data-timeline-layer-id')),
        audioLane: Number(document.querySelector('[data-timeline-audio-clip-id="700"]')?.getAttribute('data-timeline-layer-id')),
        eventLane: Number(document.querySelector(${JSON.stringify(eventSelector)})?.getAttribute('data-timeline-layer-id')),
        lightingAutomationLane: Number(document.querySelector('[data-timeline-automation-kind="lighting"][data-timeline-automation-id="1"]')?.getAttribute('data-timeline-layer-id')),
        videoAutomationLane: Number(document.querySelector('[data-timeline-automation-kind="video"][data-timeline-automation-id="2"]')?.getAttribute('data-timeline-layer-id')),
        selectedVideo: document.querySelectorAll('.timelineVideoClip.selected').length,
        selectedAudio: document.querySelectorAll('.timelineAudioClip.selected').length,
        selectedEvent: document.querySelectorAll('.timelineMarker.selected').length,
        selectedAutomation: document.querySelectorAll('.timelineAutomationRange.selected').length,
        focusedKind: document.activeElement?.getAttribute('data-timeline-automation-kind') ?? '',
        focusedId: document.activeElement?.getAttribute('data-timeline-automation-id') ?? '',
      }))()`);
      assert.deepEqual(
        [laneMoveFinal.call.primary.kind, laneMoveFinal.call.items.length, laneMoveFinal.call.delta_ms, laneMoveFinal.call.isolate],
        ["video_automation", 5, 0, false],
        "keyboard-operable context action dispatches the exact five-domain group request",
      );
      assert.deepEqual(
        [laneMoveFinal.audioLane, laneMoveFinal.eventLane, laneMoveFinal.videoLane, laneMoveFinal.lightingAutomationLane, laneMoveFinal.videoAutomationLane],
        [10, 12, 14, 12, 14],
        "ACK local apply mounts every explicit lane target",
      );
      assert.deepEqual(
        [laneMoveFinal.selectedVideo, laneMoveFinal.selectedAudio, laneMoveFinal.selectedEvent, laneMoveFinal.selectedAutomation],
        [1, 1, 1, 1],
        "ACK preserves the logical linked selection across every visible domain",
      );
      assert.deepEqual([laneMoveFinal.focusedKind, laneMoveFinal.focusedId], ["video", "2"], "ACK restores focus to the logical primary item");
      assert.equal(await evaluate(client, `(() => {
        const item = document.querySelector(${JSON.stringify(eventSelector)});
        if (!(item instanceof Element)) return false;
        item.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowUp', shiftKey: true, bubbles: true, cancelable: true }));
        return true;
      })()`), true);
      await sleep(50);
      assert.equal(
        await evaluate(client, "window.__syndocalTimelineLaneMoveFixtureCalls.length"),
        beforeContextMove + 1,
        "Scene keyboard move with no compatible canonical predecessor invokes no authoritative command",
      );
      assert.equal(await evaluate(client, `(() => {
        const item = document.querySelector('[data-timeline-automation-kind="video"][data-timeline-automation-id="2"]');
        if (!(item instanceof Element)) return false;
        item.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowUp', shiftKey: true, bubbles: true, cancelable: true }));
        return true;
      })()`), true);
      await sleep(50);
      assert.equal(await evaluate(client, "window.__syndocalTimelineLaneMoveFixtureCalls.length"), beforeContextMove + 1, "boundary keyboard move invokes no authoritative command");
    }
    assert.equal(targetProof.every((proof) => proof?.rect[0] > 0 && proof?.rect[1] > 0 && proof.count > 0 && proof.short === 0), true, `Timeline bank, Guide audio, Phase, and loop controls preserve 44px targets at ${viewport.width}x${viewport.height}: ${JSON.stringify(targetProof)}`);
    assert.equal(state.shortTargets, 0, `Visible Timeline performance controls preserve 44px targets at ${viewport.width}x${viewport.height}`);
    assert.equal(state.fixedOuter, true, `Timeline disclosures keep app/document outer scroll fixed at ${viewport.width}x${viewport.height}`);
    console.log(`${viewport.width}x${viewport.height}: phases=${state.phaseLabels.join('/')} bank=${state.bankItems} media=${state.videoClips}+${state.audioClips} selected=${state.selectedVideo}+${state.selectedAudio}`);
  }

  client.close();
  const directResizeUrl = `http://${host}:${vitePort}/scripts/fixtures/timeline-direct-resize.html`;
  const directResizeTarget = await waitFor(async () => {
    const response = await fetch(`http://${host}:${cdpPort}/json/new?${encodeURIComponent(directResizeUrl)}`, { method: "PUT" });
    return response.ok ? response.json() : null;
  }, "direct-resize browser target");
  client = new CdpClient(directResizeTarget.webSocketDebuggerUrl);
  await client.ready();
  await client.send("Page.enable");
  await client.send("Runtime.enable");
  await client.send("Emulation.setDeviceMetricsOverride", { width: 1280, height: 720, deviceScaleFactor: 1, mobile: false });
  await waitFor(
    () => evaluate(client, "window.__timelineDirectResizeFixture?.ready === true"),
    "Timeline direct-resize fixture",
  );
  await dragTimelineResize(client, '[data-timeline-event-id="101"]', '.timelineSceneBlockResizeHandle.start', "start");
  await dragTimelineResize(client, '[data-timeline-event-id="102"]', '.timelineSceneBlockResizeHandle.end', "end");
  await dragTimelineResize(client, '[data-timeline-audio-clip-id="201"]', '[data-timeline-audio-resize="end"]', "end");
  await dragTimelineResize(client, '[data-timeline-audio-clip-id="202"]', '[data-timeline-audio-resize="start"]', "start");
  await dragTimelineResize(client, '[data-timeline-video-clip-id="301"]', '[data-timeline-video-resize="start"]', "start", true);
  await dragTimelineResize(client, '[data-timeline-video-clip-id="302"]', '[data-timeline-video-resize="end"]', "end");
  await dragTimelineResize(client, '[data-timeline-automation-kind="lighting"][data-timeline-automation-id="401"]', '.timelineAutomationHandle.end', "end");
  await dragTimelineResize(client, '[data-timeline-automation-kind="lighting"][data-timeline-automation-id="402"]', '.timelineAutomationHandle.start', "start");
  await dragTimelineResize(client, '[data-timeline-automation-kind="video"][data-timeline-automation-id="501"]', '.timelineAutomationHandle.start', "start", true);
  await dragTimelineResize(client, '[data-timeline-automation-kind="video"][data-timeline-automation-id="502"]', '.timelineAutomationHandle.end', "end");
  const directResizeCalls = await evaluate(client, "structuredClone(window.__timelineDirectResizeFixture.calls)");
  assert.deepEqual(
    directResizeCalls.trim.map(({ item, edge, isolate }) => [item.kind, edge, isolate]),
    [
      ["lighting_event", "start", false],
      ["audio_clip", "end", false],
      ["video_clip", "start", true],
      ["lighting_automation", "end", false],
      ["video_automation", "start", true],
    ],
    "real pointer gestures route every linked Timeline domain to authoritative trim and preserve Alt isolate",
  );
  assert.deepEqual(
    directResizeCalls.scene.map(({ event_id, edge }) => [event_id, edge]),
    [[102, "end"]],
    "unlinked Scene resize retains its legacy route",
  );
  assert.deepEqual(directResizeCalls.audio.map(({ id }) => id), [202], "unlinked Audio resize retains its legacy route");
  assert.equal(directResizeCalls.audio[0].start_ms > 4_400, true, "unlinked Audio start edge moves later");
  assert.equal(directResizeCalls.audio[0].offset_ms > 400, true, "unlinked Audio start trim advances source offset");
  assert.equal(directResizeCalls.audio[0].duration_ms < 800, true, "unlinked Audio start trim shortens duration");
  assert.deepEqual(directResizeCalls.video.map(({ id }) => id), [302], "unlinked Video resize retains its legacy route");
  assert.equal(directResizeCalls.video[0].start_ms, 7_000, "unlinked Video end trim preserves start");
  assert.equal(directResizeCalls.video[0].offset_ms, 400, "unlinked Video end trim preserves source offset");
  assert.equal(directResizeCalls.video[0].duration_ms > 800, true, "unlinked Video end edge extends duration");
  assert.deepEqual(
    directResizeCalls.automation.map(({ kind, automation_id, edge }) => [kind, automation_id, edge]),
    [["lighting", 402, "start"], ["video", 502, "end"]],
    "unlinked Lighting and Video automation resize retain their legacy routes",
  );
  assert.equal(
    directResizeCalls.trim.every(({ boundary_ms }) => Number.isInteger(boundary_ms) && boundary_ms >= 0),
    true,
    "linked direct-resize boundaries are finite non-negative integer milliseconds",
  );
  const trimByKind = new Map(directResizeCalls.trim.map((call) => [call.item.kind, call]));
  assert.equal(trimByKind.get("lighting_event").boundary_ms > 500, true, "linked Scene start edge moves later");
  assert.equal(trimByKind.get("audio_clip").boundary_ms > 3_900, true, "linked Audio end edge moves later");
  assert.equal(trimByKind.get("video_clip").boundary_ms > 5_700, true, "linked Video start edge moves later");
  assert.equal(trimByKind.get("lighting_automation").boundary_ms > 9_100, true, "linked Lighting automation end edge moves later");
  assert.equal(trimByKind.get("video_automation").boundary_ms > 8_300, true, "linked Video automation start edge moves later");
  console.log("direct resize: linked Scene/Audio/Video/Lighting automation/Video automation + unlinked legacy routes + Alt isolate ok");
} finally {
  client?.close();
  await stopChild(browser);
  await stopChild(vite);
  if (profileDir) await rm(profileDir, { recursive: true, force: true });
}
