import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { assertTimelineSourceTargetReveal } from "./timeline-source-target-reveal-proof.mjs";

// PROOF BOUNDARY: this gate drives the timeline-layered browser fixture. Its
// split/lane-move fixture callbacks intercept requests before
// commitTimelineAdvanced and native Tauri IPC. Every assertion below is
// fixture-request, DOM-projection, and geometry proof only; it never
// evidences native, backend, transaction-owner, persistence, or
// physical-output completion.
const appRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const host = "127.0.0.1";
const vitePort = 5193;
const cdpPort = 9243;
const baseUrl = `http://${host}:${vitePort}/?syndocalViewportFixture=timeline-layered`;
const screenshotDir = process.env.SYNDOCAL_CONTROL_SCREENSHOT_DIR || process.env.SYNDOCAL_TIMELINE_SCREENSHOT_DIR
  ? resolve(process.env.SYNDOCAL_CONTROL_SCREENSHOT_DIR || process.env.SYNDOCAL_TIMELINE_SCREENSHOT_DIR)
  : null;
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
const dispatchTimelineWheel = (client, {
  deltaX = 0,
  deltaY = 0,
  deltaMode = 0,
  shiftKey = false,
  ctrlKey = false,
  altKey = false,
  clientXRatio = 0.25,
  targetSelector = '.timelineOverview',
} = {}) => evaluate(client, `(function() {
  const overview = document.querySelector('.timelineOverview');
  const scrollport = document.querySelector('.timelineLayerScrollport');
  const app = document.querySelector('.app');
  const target = document.querySelector(${JSON.stringify(targetSelector)});
  if (!(overview instanceof SVGSVGElement) || !(scrollport instanceof HTMLElement) || !(target instanceof Element)) return null;
  const rect = overview.getBoundingClientRect();
  const before = {
    start: Number(overview.getAttribute('data-visible-start-ms')),
    end: Number(overview.getAttribute('data-visible-end-ms')),
    overviewWidth: rect.width,
    overviewHeight: rect.height,
    scrollTop: scrollport.scrollTop,
    documentTop: document.documentElement.scrollTop,
    documentLeft: document.documentElement.scrollLeft,
    bodyTop: document.body.scrollTop,
    bodyLeft: document.body.scrollLeft,
    appTop: app instanceof HTMLElement ? app.scrollTop : 0,
    appLeft: app instanceof HTMLElement ? app.scrollLeft : 0,
  };
  const event = new WheelEvent('wheel', {
    bubbles: true,
    cancelable: true,
    clientX: rect.left + rect.width * ${JSON.stringify(clientXRatio)},
    clientY: rect.top + rect.height / 2,
    deltaX: ${JSON.stringify(deltaX)},
    deltaY: ${JSON.stringify(deltaY)},
    deltaMode: ${JSON.stringify(deltaMode)},
    shiftKey: ${JSON.stringify(shiftKey)},
    ctrlKey: ${JSON.stringify(ctrlKey)},
    altKey: ${JSON.stringify(altKey)},
  });
  const dispatchResult = target.dispatchEvent(event);
  const after = {
    start: Number(overview.getAttribute('data-visible-start-ms')),
    end: Number(overview.getAttribute('data-visible-end-ms')),
    overviewWidth: rect.width,
    overviewHeight: rect.height,
    scrollTop: scrollport.scrollTop,
    documentTop: document.documentElement.scrollTop,
    documentLeft: document.documentElement.scrollLeft,
    bodyTop: document.body.scrollTop,
    bodyLeft: document.body.scrollLeft,
    appTop: app instanceof HTMLElement ? app.scrollTop : 0,
    appLeft: app instanceof HTMLElement ? app.scrollLeft : 0,
  };
  return { dispatchResult, defaultPrevented: event.defaultPrevented, before, after };
})()`);
const readTimelineVisibleWindow = (client) => evaluate(client, `(() => {
  const overview = document.querySelector('.timelineOverview');
  if (!(overview instanceof SVGSVGElement)) return null;
  const start = Number(overview.getAttribute('data-visible-start-ms'));
  const end = Number(overview.getAttribute('data-visible-end-ms'));
  return { start, end, span: end - start };
})()`);
const exerciseTimelineWheel = async (client, viewport) => {
  const reset = async () => {
    assert.equal(await click(client, '[data-timeline-tool="fit-all"]'), true, "Timeline wheel fixture restores Fit All");
    await sleep(30);
    assert.equal(await evaluate(client, `(() => {
      const scrollport = document.querySelector('.timelineLayerScrollport');
      if (!(scrollport instanceof HTMLElement)) return false;
      scrollport.scrollTop = 0;
      return true;
    })()`), true);
  };
  await reset();
  const initial = await readTimelineVisibleWindow(client);
  assert.ok(initial && initial.span > 0, `Timeline wheel fixture exposes a visible window at ${viewport.width}x${viewport.height}`);

  const deltaXOnly = await dispatchTimelineWheel(client, { deltaX: 40 });
  assert.ok(deltaXOnly, `Timeline wheel deltaX-only event dispatched at ${viewport.width}x${viewport.height}`);
  assert.equal(deltaXOnly.defaultPrevented, true, "unmodified deltaX-dominant wheel is consumed by the Timeline");
  assert.equal(deltaXOnly.after.scrollTop, deltaXOnly.before.scrollTop, "deltaX-only wheel does not scroll Timeline lanes");
  assert.equal(deltaXOnly.after.end - deltaXOnly.after.start, initial.span, "deltaX-only pan preserves visible span");
  assert.ok(deltaXOnly.after.start > initial.start, "deltaX-only wheel advances the visible time window");

  await reset();
  const shifted = await dispatchTimelineWheel(client, { deltaX: 40, deltaY: 240, shiftKey: true });
  assert.ok(shifted, `Timeline Shift wheel event dispatched at ${viewport.width}x${viewport.height}`);
  assert.equal(shifted.defaultPrevented, true, "Shift wheel is consumed by the Timeline");
  assert.equal(shifted.after.scrollTop, shifted.before.scrollTop, "Shift wheel does not scroll Timeline lanes");
  assert.equal(shifted.after.end - shifted.after.start, initial.span, "Shift pan preserves visible span");
  assert.ok(shifted.after.start > initial.start, "Shift wheel pans along the time axis");
  assert.ok(
    Math.abs((shifted.after.start - initial.start) - (40 / Math.max(1, shifted.before.overviewWidth)) * initial.span) < 1,
    "Shift wheel prefers deltaX over its larger deltaY fallback",
  );

  await reset();
  const shiftedLineFallback = await dispatchTimelineWheel(client, { deltaY: 1, deltaMode: 1, shiftKey: true });
  assert.ok(shiftedLineFallback && shiftedLineFallback.defaultPrevented, "Shift wheel falls back to a line-mode deltaY");
  assert.ok(shiftedLineFallback.after.start > initial.start, "Shift line-mode deltaY pans the time axis");

  await reset();
  const shiftedPageFallback = await dispatchTimelineWheel(client, { deltaY: 1, deltaMode: 2, shiftKey: true });
  assert.ok(shiftedPageFallback && shiftedPageFallback.defaultPrevented, "Shift wheel converts a page-mode deltaY");
  assert.ok(shiftedPageFallback.after.start > initial.start, "Shift page-mode deltaY pans the time axis");

  await reset();
  const ctrlBefore = await readTimelineVisibleWindow(client);
  const ctrlBeforeGrid = await evaluate(client, `(() => {
    const ruler = document.querySelector('.timelineRuler');
    if (!(ruler instanceof Element)) return null;
    return {
      majorStepMs: Number(ruler.getAttribute('data-timeline-grid-major-step-ms')),
      minorStepMs: Number(ruler.getAttribute('data-timeline-grid-minor-step-ms')),
      subdivision: Number(ruler.getAttribute('data-timeline-grid-subdivision-count')),
      lineCount: Number(ruler.getAttribute('data-timeline-grid-line-count')),
    };
  })()`);
  const ctrl = await dispatchTimelineWheel(client, { deltaY: -40, ctrlKey: true, clientXRatio: 0.25 });
  assert.ok(ctrl, `Ctrl wheel event dispatched at ${viewport.width}x${viewport.height}`);
  assert.equal(ctrl.defaultPrevented, true, "Ctrl wheel is consumed by the Timeline");
  assert.equal(ctrl.after.scrollTop, ctrl.before.scrollTop, "Ctrl wheel does not scroll Timeline lanes");
  assert.ok(ctrl.after.end - ctrl.after.start < ctrlBefore.span, "Ctrl wheel zooms the visible time window");
  const anchorMs = ctrlBefore.start + ctrlBefore.span * 0.25;
  const beforeRatio = (anchorMs - ctrlBefore.start) / ctrlBefore.span;
  const afterRatio = (anchorMs - ctrl.after.start) / (ctrl.after.end - ctrl.after.start);
  assert.ok(Math.abs(afterRatio - beforeRatio) < 0.001, `Ctrl zoom keeps its pointer anchor: before=${beforeRatio} after=${afterRatio}`);
  const ctrlAfterGrid = await evaluate(client, `(() => {
    const ruler = document.querySelector('.timelineRuler');
    if (!(ruler instanceof Element)) return null;
    return {
      majorStepMs: Number(ruler.getAttribute('data-timeline-grid-major-step-ms')),
      minorStepMs: Number(ruler.getAttribute('data-timeline-grid-minor-step-ms')),
      subdivision: Number(ruler.getAttribute('data-timeline-grid-subdivision-count')),
      lineCount: Number(ruler.getAttribute('data-timeline-grid-line-count')),
    };
  })()`);
  assert.ok(
    ctrlBeforeGrid && ctrlAfterGrid && (
      ctrlAfterGrid.majorStepMs !== ctrlBeforeGrid.majorStepMs ||
      ctrlAfterGrid.minorStepMs !== ctrlBeforeGrid.minorStepMs ||
      ctrlAfterGrid.subdivision !== ctrlBeforeGrid.subdivision ||
      ctrlAfterGrid.lineCount !== ctrlBeforeGrid.lineCount
    ),
    `Ctrl zoom updates the visible adaptive ruler grid model: before=${JSON.stringify(ctrlBeforeGrid)} after=${JSON.stringify(ctrlAfterGrid)}`,
  );

  await reset();
  const ctrlShiftBefore = await readTimelineVisibleWindow(client);
  const ctrlShift = await dispatchTimelineWheel(client, { deltaX: 120, deltaY: -40, ctrlKey: true, shiftKey: true });
  assert.ok(ctrlShift && ctrlShift.defaultPrevented, "Ctrl+Shift wheel is consumed by the Timeline");
  assert.ok(ctrlShift.after.end - ctrlShift.after.start < ctrlShiftBefore.span, "Ctrl has priority over Shift and zooms instead of panning");
  assert.equal(ctrlShift.after.scrollTop, ctrlShift.before.scrollTop, "Ctrl+Shift wheel does not scroll Timeline lanes");

  await reset();
  const alt = await dispatchTimelineWheel(client, { deltaX: 40, deltaY: 40, shiftKey: true, ctrlKey: true, altKey: true });
  assert.ok(alt, `Alt wheel event dispatched at ${viewport.width}x${viewport.height}`);
  assert.equal(alt.defaultPrevented, false, "Alt-containing wheel remains unhandled");
  assert.equal(alt.dispatchResult, true, "Alt-containing wheel does not cancel browser dispatch");
  assert.deepEqual(alt.after, alt.before, "Alt-containing wheel performs no custom mutation");

  if (viewport.width === 860) {
    await reset();
    const scrollRange = await createDeterministicScrollRange(client, {
      scrollportSelector: '.timelineLayerScrollport',
      contentSelector: '.timelineLayerScrollContent',
      axis: 'top',
    });
    assert.ok((scrollRange?.range ?? 0) > 0, "Timeline wheel fixture has a deterministic vertical lane-scroll range");
    const gutterSelector = '[data-timeline-layer-gutter]';
    assert.ok(await evaluate(client, `document.querySelector(${JSON.stringify(gutterSelector)}) instanceof Element`), "Timeline layered gutter is available as a wheel target");
    const gutterCtrlBefore = await readTimelineVisibleWindow(client);
    const gutterCtrl = await dispatchTimelineWheel(client, { deltaY: -40, ctrlKey: true, clientXRatio: 0.25, targetSelector: gutterSelector });
    assert.ok(gutterCtrl && gutterCtrl.defaultPrevented, "Ctrl wheel from the layered gutter is consumed");
    assert.ok(gutterCtrl.after.end - gutterCtrl.after.start < gutterCtrlBefore.span, "Ctrl wheel from the layered gutter zooms the visible time window");
    await reset();
    const gutterShiftBefore = await readTimelineVisibleWindow(client);
    const gutterShift = await dispatchTimelineWheel(client, { deltaX: 40, shiftKey: true, targetSelector: gutterSelector });
    assert.ok(gutterShift && gutterShift.defaultPrevented, "Shift wheel from the layered gutter is consumed");
    assert.equal(gutterShift.after.end - gutterShift.after.start, gutterShiftBefore.span, "Shift wheel from the layered gutter preserves visible span");
    assert.ok(gutterShift.after.start > gutterShiftBefore.start, "Shift wheel from the layered gutter pans time");
    await reset();
    const vertical = await dispatchTimelineWheel(client, { deltaY: 32, targetSelector: gutterSelector });
    assert.ok(vertical && vertical.defaultPrevented, "plain vertical wheel is consumed by the Timeline");
    assert.ok(vertical.after.scrollTop > vertical.before.scrollTop, "plain vertical wheel from the layered gutter scrolls the owning lane scrollport");
    assert.deepEqual(
      [vertical.after.start, vertical.after.end, vertical.after.documentTop, vertical.after.documentLeft, vertical.after.appTop, vertical.after.appLeft],
      [vertical.before.start, vertical.before.end, 0, 0, 0, 0],
      "plain vertical wheel leaves the time window and outer document/app scroll unchanged",
    );
    await evaluate(client, `(() => {
      const scrollport = document.querySelector('.timelineLayerScrollport');
      if (scrollport instanceof HTMLElement) scrollport.scrollTop = 0;
      return true;
    })()`);
    const verticalBoundary = await dispatchTimelineWheel(client, { deltaY: -32, targetSelector: gutterSelector });
    assert.ok(verticalBoundary && verticalBoundary.defaultPrevented, "plain vertical wheel from the layered gutter remains consumed at the lane-scroll boundary");
    assert.equal(verticalBoundary.after.scrollTop, 0, "plain vertical boundary wheel does not produce negative scroll");
    await restoreDeterministicScrollRange(client, {
      contentSelector: '.timelineLayerScrollContent',
      axis: 'top',
      previousInlineSize: scrollRange.previousInlineSize,
    });
  }
  await reset();
  console.log(`${viewport.width}x${viewport.height}: Timeline wheel contract (plain lane scroll, Shift/deltaX time pan, Ctrl anchored zoom, Alt no-op) passed`);
};
const openTimelineContextMenuGroup = (client, groupId) => evaluate(client, `(() => {
  const group = document.querySelector('.timelineItemContextMenu details[data-timeline-context-menu-group="${groupId}"]');
  if (!(group instanceof HTMLDetailsElement)) return false;
  const summary = group.querySelector(':scope > summary');
  if (!(summary instanceof HTMLElement)) return false;
  if (!group.open) summary.click();
  return group.open;
})()`);
const openTimelineVideoContextMenu = (client, clientY = 180) => evaluate(client, `(() => {
  const clip = document.querySelector('[data-timeline-video-clip-id="800"]');
  if (!(clip instanceof Element)) return false;
  if ('focus' in clip && typeof clip.focus === 'function') clip.focus();
  clip.dispatchEvent(new MouseEvent('contextmenu', {
    bubbles: true,
    cancelable: true,
    clientX: 240,
    clientY: ${JSON.stringify(clientY)},
  }));
  return document.querySelector('.timelineItemContextMenu') !== null;
})()`);
const openTimelineContextMenuActions = (client) => evaluate(client, `(() => {
  const actions = document.querySelector('.timelineItemContextMenu > details.timelineItemContextMenuActions');
  const summary = actions?.querySelector(':scope > summary');
  if (!(actions instanceof HTMLDetailsElement) || !(summary instanceof HTMLElement)) return false;
  if (!actions.open) summary.click();
  return actions.open;
})()`);
const openTimelineLayerContextMenu = (client) => evaluate(client, `(() => {
  const trigger = document.querySelector('[data-timeline-layer-menu-trigger]');
  if (!(trigger instanceof HTMLElement)) return false;
  trigger.click();
  return document.querySelector('.timelineLayerContextMenu') !== null;
})()`);
const openSceneMatrixContextMenu = (client, triggerSelector, menuSelector) => evaluate(client, `(() => {
  const trigger = document.querySelector(${JSON.stringify(triggerSelector)});
  if (!(trigger instanceof HTMLElement)) return false;
  const rect = trigger.getBoundingClientRect();
  trigger.dispatchEvent(new MouseEvent('contextmenu', {
    bubbles: true,
    cancelable: true,
    clientX: rect.left + Math.max(2, rect.width / 2),
    clientY: rect.top + Math.max(2, rect.height / 2),
  }));
  return document.querySelector(${JSON.stringify(menuSelector)}) !== null;
})()`);
const observeNativeScroll = (client, { scrollportSelector, axis, menuSelector }) => evaluate(client, `(async () => {
  const scrollport = document.querySelector(${JSON.stringify(scrollportSelector)});
  if (!(scrollport instanceof HTMLElement)) return null;
  const scrollProperty = ${JSON.stringify(axis)} === 'top' ? 'scrollTop' : 'scrollLeft';
  const dimension = ${JSON.stringify(axis)} === 'top' ? 'Height' : 'Width';
  const range = scrollport['scroll' + dimension] - scrollport['client' + dimension];
  const before = scrollport[scrollProperty];
  const delta = Math.max(1, Math.floor(range / 2));
  const target = before < range ? Math.min(range, before + delta) : Math.max(0, before - delta);
  if (!(range > 0) || target === before) {
    return { range, before, target, after: before, nativeScrollObserved: false, menuPresent: document.querySelector(${JSON.stringify(menuSelector)}) !== null };
  }
  const nativeScrollObserved = await new Promise((resolve) => {
    let settled = false;
    const onScroll = () => settle(true);
    const timeout = window.setTimeout(() => settle(false), 500);
    const settle = (observed) => {
      if (settled) return;
      settled = true;
      window.clearTimeout(timeout);
      scrollport.removeEventListener('scroll', onScroll);
      window.setTimeout(() => resolve(observed), 0);
    };
    scrollport.addEventListener('scroll', onScroll, { once: true });
    scrollport[scrollProperty] = target;
  });
  return {
    range,
    before,
    target,
    after: scrollport[scrollProperty],
    nativeScrollObserved,
    menuPresent: document.querySelector(${JSON.stringify(menuSelector)}) !== null,
  };
})()`);
const createDeterministicScrollRange = (client, { scrollportSelector, contentSelector, axis }) => evaluate(client, `(async () => {
  const scrollport = document.querySelector(${JSON.stringify(scrollportSelector)});
  const content = document.querySelector(${JSON.stringify(contentSelector)});
  if (!(scrollport instanceof HTMLElement) || !(content instanceof HTMLElement)) return null;
  const property = ${JSON.stringify(axis)} === 'top' ? 'height' : 'width';
  const dimension = ${JSON.stringify(axis)} === 'top' ? 'Height' : 'Width';
  const previousInlineSize = content.style[property];
  const initialRange = scrollport['scroll' + dimension] - scrollport['client' + dimension];
  if (!(initialRange > 0)) {
    content.style[property] = String(scrollport['client' + dimension] + 64) + 'px';
    await new Promise((resolve) => requestAnimationFrame(resolve));
  }
  return {
    initialRange,
    range: scrollport['scroll' + dimension] - scrollport['client' + dimension],
    previousInlineSize,
  };
})()`);
const restoreDeterministicScrollRange = (client, { contentSelector, axis, previousInlineSize }) => evaluate(client, `(() => {
  const content = document.querySelector(${JSON.stringify(contentSelector)});
  if (!(content instanceof HTMLElement)) return false;
  content.style[${JSON.stringify(axis)} === 'top' ? 'height' : 'width'] = ${JSON.stringify(previousInlineSize)};
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
    item.scrollIntoView({ block: 'center', inline: 'nearest' });
    const sourceLayerId = item.getAttribute('data-timeline-layer-id');
    const sourceLane = document.querySelector('[data-timeline-layer-gutter][data-timeline-layer-id="' + sourceLayerId + '"]');
    if (!(sourceLane instanceof Element)) return null;
    const hitSurface = item.querySelector('.timelineVideoClipBody, .timelineAudioClipBody, .timelineSceneBlockBody, :scope > rect') ?? item;
    const itemRect = hitSurface.getBoundingClientRect();
    const safeInset = hitSurface.matches('.timelineSceneBlockBody') ? 24 : 8;
    const moveBandY = itemRect.top + Math.min(4, Math.max(1, itemRect.height / 4));
    const centerY = itemRect.top + itemRect.height / 2;
    const sourceYs = item.matches('.timelineAutomationRange')
      ? [itemRect.top + itemRect.height / 2]
      // Scene blocks reserve the lower band for selection. Clips may use their
      // interior body, but blocks must start in their upper move band.
      : item.matches('.timelineMarker.sceneBlock') ? [moveBandY, centerY] : [centerY, moveBandY];
    const excluded = '.timelineAutomationHandle, .timelineAutomationKeyframeGroup, .timelineSceneBlockResizeHandle, .timelineSceneBlockFadeHandle, .timelineVideoClipResizeHandle, .timelineVideoClipFadeHandle, .timelineAudioClipResizeHandle, .timelineAudioClipFadeHandle';
    // Prefer the interior body before edge-adjacent points: a compact clip can
    // turn the legacy 8px candidate into a resize handle rather than a lane move.
    const sourceXs = [
      itemRect.left + itemRect.width * 0.5,
      itemRect.left + itemRect.width * 0.25,
      itemRect.left + itemRect.width * 0.75,
      itemRect.left + Math.min(safeInset, Math.max(1, itemRect.width / 2)),
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
      sourceSize: [itemRect.width, itemRect.height],
    };
  })()`);
  assert.ok(points && points.sourceSize[0] > 0 && points.sourceSize[1] > 0, `visible lane drag source geometry for ${selector}`);
  const modifiers = isolate ? 1 : 0;
  await client.send("Input.dispatchMouseEvent", { type: "mouseMoved", ...points.source, modifiers });
  await client.send("Input.dispatchMouseEvent", { type: "mousePressed", ...points.source, modifiers, button: "left", buttons: 1, clickCount: 1 });
  // The compact fixture cannot co-display distant source and destination
  // lanes. Scroll the same timeline container while pointer capture is live,
  // then remeasure the destination lane's actual SVG hit surface before drop.
  const target = await evaluate(client, `(() => {
    const target = document.querySelector('.timelineLayerRowBackground[data-timeline-layer-id="${targetLayerId}"]');
    if (!(target instanceof Element)) return null;
    target.scrollIntoView({ block: 'center', inline: 'nearest' });
    const rect = target.getBoundingClientRect();
    const candidateXs = [
      Math.max(rect.left + 8, Math.min(rect.right - 8, ${JSON.stringify(points.source.x)} + 24)),
      rect.left + rect.width * 0.75,
      rect.left + rect.width * 0.5,
      rect.left + rect.width * 0.25,
    ];
    const point = candidateXs.map((x) => ({ x, y: rect.top + rect.height / 2 })).find((candidate) =>
      document.elementFromPoint(candidate.x, candidate.y)
        ?.closest('[data-timeline-layer-id]')?.getAttribute('data-timeline-layer-id') === ${JSON.stringify(String(targetLayerId))},
    );
    if (!point) return null;
    return {
      point,
      size: [rect.width, rect.height],
      layerAtPoint: document.elementFromPoint(point.x, point.y)
        ?.closest('[data-timeline-layer-id]')?.getAttribute('data-timeline-layer-id') ?? null,
    };
  })()`);
  assert.ok(target && target.size[0] > 0 && target.size[1] > 0, `visible lane drag target geometry for ${selector}`);
  assert.equal(target.layerAtPoint, String(targetLayerId), `pointer target resolves lane ${targetLayerId} for ${selector}`);
  await client.send("Input.dispatchMouseEvent", { type: "mouseMoved", ...target.point, modifiers, button: "left", buttons: 1 });
  await client.send("Input.dispatchMouseEvent", { type: "mouseReleased", ...target.point, modifiers, button: "left", buttons: 0, clickCount: 1 });
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
  // The destination gutter may be left of the clip on compact layouts. Keep
  // asserting a genuine signed pointer-time change instead of assuming its
  // direction from the old canvas-only drop coordinate.
  assert.notEqual(call.delta_ms, 0, `${label} carries a nonzero signed pointer delta`);
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
  }, `${label} fixture-result DOM time state`);
  for (const item of call.items) {
    assert.deepEqual(
      await mountedTimelineItemTimes(client, item),
      applied.after[timelineLaneItemKey(item)],
      `${label} mounts fixture-result ${timelineLaneItemKey(item)} times (never native ACK evidence)`,
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
    cueAudioOpen: Boolean(document.querySelector('.timelinePerformanceEditor .timelineCueAudioEditor[open]')),
    cueAudioControls: document.querySelectorAll('.timelineCueAudioEditorBody input, .timelineCueAudioEditorBody select, .timelineCueAudioEditorBody button').length,
    cueAudioState: document.querySelector('.timelineCueAudioEditor > summary output')?.textContent?.trim() ?? '',
    cueAudioCheckboxes: document.querySelectorAll('.timelineCueAudioEditorBody input[type="checkbox"]').length,
    phaseLabels: phases.map((phase) => phase.textContent?.trim() ?? ''),
    guidePressed: document.querySelector('.timelineOperatorBar [data-timeline-guide]')?.getAttribute('aria-pressed') ?? '',
    loopPressed: document.querySelector('.timelineOperatorBar [data-timeline-loop-toggle]')?.getAttribute('aria-pressed') ?? '',
    loopState: document.querySelector('.timelineOperatorBar .timelineLoopState')?.textContent?.trim() ?? '',
    loopScaleControls: buttons.filter((button) => ['Halve loop length', 'Double loop length'].includes(button.title)).length,
    snapState: document.querySelector('[data-timeline-snap-state]')?.textContent?.trim() ?? '',
    edgeMagnetState: document.querySelector('[data-timeline-edge-magnet-state]')?.getAttribute('data-timeline-edge-magnet-state') ?? '',
    timelineGrid: (() => {
      const ruler = root.querySelector('.timelineRuler');
      if (!(ruler instanceof Element)) return null;
      return {
        major: ruler.querySelectorAll('line.major').length,
        minor: ruler.querySelectorAll('line.minor').length,
        labels: ruler.querySelectorAll('text').length,
        lineCount: Number(ruler.getAttribute('data-timeline-grid-line-count')),
        subdivision: Number(ruler.getAttribute('data-timeline-grid-subdivision-count')),
        majorStepMs: Number(ruler.getAttribute('data-timeline-grid-major-step-ms')),
        minorStepMs: Number(ruler.getAttribute('data-timeline-grid-minor-step-ms')),
        labelsOnMinor: [...ruler.querySelectorAll('.minor text')].length,
      };
    })(),
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

const measureArrangerGeometry = (client) => evaluate(client, `(() => {
  const livePanel = document.querySelector('.layoutSharedWorkspace.controlModeLive > .liveControlPanel');
  const header = livePanel?.querySelector(':scope > .timelineArrangerHeader');
  const host = livePanel?.querySelector(':scope > .timelineArrangerUpperHost');
  const desk = document.querySelector('.faders.timelineDesk-show');
  const surface = desk?.querySelector('.timelineShowSurface');
  // TimelineCueEventsPanel owns the overview through its local TimelinePanel
  // wrapper; keep the surface as the upper-boundary owner without requiring a
  // brittle direct-child relationship.
  const frame = surface?.querySelector('.timelineOverviewFrame');
  const scrollport = frame?.querySelector('.timelineLayerScrollport');
  const lowerBand = document.querySelector('.layoutSharedWorkspace.controlModeLive > .mappingPersistentWorkspaceBand');
  const lowerContext = lowerBand?.querySelector('[data-workspace-pane="lower-right"]');
  const sourceShelf = lowerContext?.querySelector(':scope > [data-timeline-source-shelf]');
  const sourceShelfHeader = sourceShelf?.querySelector(':scope > .timelineExternalSourceShelfHeader');
  const sourceShelfBody = sourceShelf?.querySelector(':scope > [role="tabpanel"]');
  const rect = (element) => {
    if (!(element instanceof Element)) return null;
    const bounds = element.getBoundingClientRect();
    return [bounds.left, bounds.top, bounds.right, bounds.bottom, bounds.width, bounds.height];
  };
  return {
    live: rect(livePanel),
    header: rect(header),
    host: rect(host),
    desk: rect(desk),
    surface: rect(surface),
    frame: rect(frame),
    scrollport: rect(scrollport),
    lowerBand: rect(lowerBand),
    lowerContext: rect(lowerContext),
    sourceShelf: rect(sourceShelf),
    sourceShelfHeader: rect(sourceShelfHeader),
    sourceShelfBody: rect(sourceShelfBody),
    sourceShelfRows: sourceShelf instanceof HTMLElement ? getComputedStyle(sourceShelf).gridTemplateRows : '',
    sourceShelfOverflowY: sourceShelf instanceof HTMLElement ? getComputedStyle(sourceShelf).overflowY : '',
    sourceShelfOuterScroll: sourceShelf instanceof HTMLElement ? [sourceShelf.scrollHeight, sourceShelf.clientHeight] : null,
    sourceShelfHeaderScroll: sourceShelfHeader instanceof HTMLElement ? [sourceShelfHeader.scrollHeight, sourceShelfHeader.clientHeight] : null,
    sourceShelfBodyOverflowY: sourceShelfBody instanceof HTMLElement ? getComputedStyle(sourceShelfBody).overflowY : '',
    liveRows: livePanel instanceof HTMLElement ? getComputedStyle(livePanel).gridTemplateRows : '',
    hostChildren: host instanceof Element ? [...host.children].map((element) => element.className) : [],
    deskParent: desk instanceof Element ? desk.parentElement?.className ?? null : null,
    deskParentTag: desk instanceof Element ? desk.parentElement?.tagName ?? null : null,
    deskParentRect: desk instanceof Element ? rect(desk.parentElement) : null,
    deskParentStyle: desk instanceof Element && desk.parentElement ? (() => {
      const style = getComputedStyle(desk.parentElement);
      return { display: style.display, width: style.width, height: style.height, rows: style.gridTemplateRows, columns: style.gridTemplateColumns, overflow: style.overflow };
    })() : null,
    deskParentMatchesPortalRule: desk instanceof Element && desk.parentElement
      ? desk.parentElement.matches('.layout.layoutSharedWorkspace.layoutControl.controlModeLive > .liveControlPanel > .timelineArrangerUpperHost > div')
      : false,
    deskAncestors: desk instanceof Element ? (() => {
      const rows = [];
      let node = desk.parentElement;
      while (node && rows.length < 5) {
        rows.push({ tag: node.tagName, className: node.className, rect: rect(node) });
        node = node.parentElement;
      }
      return rows;
    })() : [],
    deskRows: desk instanceof HTMLElement ? getComputedStyle(desk).gridTemplateRows : '',
    surfaceRows: surface instanceof HTMLElement ? getComputedStyle(surface).gridTemplateRows : '',
    headerShortTargets: header instanceof Element
      ? [...header.querySelectorAll('button, summary')].filter((element) => {
          const bounds = element.getBoundingClientRect();
          return bounds.width > 0 && bounds.height > 0 && bounds.height < 43.5;
        }).length
      : -1,
    shortTargetDetails: header instanceof Element
      ? [...header.querySelectorAll('button, summary')].map((element) => {
          const bounds = element.getBoundingClientRect();
          return bounds.width > 0 && bounds.height > 0 && bounds.height < 43.5
            ? [element.tagName, element.getAttribute('title') ?? element.getAttribute('aria-label') ?? element.textContent?.trim() ?? '', bounds.width, bounds.height]
            : null;
        }).filter(Boolean)
      : [],
    duplicateSurfaces: document.querySelectorAll('.timelineShowSurface').length,
    legacyHeaders: surface?.querySelectorAll(':scope > .panelHeader').length ?? -1,
    legacyTools: surface?.querySelectorAll(':scope > .timelineToolStrip').length ?? -1,
  };
})()`);

const assertArrangerGeometry = (geometry, viewport) => {
  assert.ok(geometry?.live && geometry.header && geometry.host && geometry.desk && geometry.surface && geometry.frame && geometry.scrollport && geometry.lowerContext && geometry.sourceShelf && geometry.sourceShelfHeader && geometry.sourceShelfBody, `Timeline arranger and lower source shelf geometry are mounted at ${viewport.width}x${viewport.height}: ${JSON.stringify(geometry)}`);
  const [liveLeft, liveTop, liveRight, liveBottom, liveWidth] = geometry.live;
  const [headerLeft, headerTop, headerRight, headerBottom, headerWidth, headerHeight] = geometry.header;
  const [hostLeft, hostTop, hostRight, hostBottom, hostWidth, hostHeight] = geometry.host;
  const [deskLeft, deskTop, deskRight, deskBottom] = geometry.desk;
  const [surfaceLeft, surfaceTop, surfaceRight, surfaceBottom] = geometry.surface;
  const [frameLeft, frameTop, frameRight, frameBottom, , frameHeight] = geometry.frame;
  const [scrollLeft, scrollTop, scrollRight, scrollBottom] = geometry.scrollport;
  const [contextLeft, contextTop, contextRight, contextBottom, , contextHeight] = geometry.lowerContext;
  const [shelfLeft, shelfTop, shelfRight, shelfBottom, , shelfHeight] = geometry.sourceShelf;
  const [, sourceHeaderTop, , sourceHeaderBottom, , sourceHeaderHeight] = geometry.sourceShelfHeader;
  const [, sourceBodyTop, , sourceBodyBottom, , sourceBodyHeight] = geometry.sourceShelfBody;
  assert.ok(headerHeight >= 47.5 && headerHeight <= 48.5, `Timeline header owns one 48px row at ${viewport.width}x${viewport.height}: ${JSON.stringify(geometry)}`);
  assert.ok(Math.abs(headerTop - liveTop) <= 1 && Math.abs(hostTop - headerBottom) <= 1 && Math.abs(hostBottom - liveBottom) <= 1, `Timeline header and arranger are contiguous and consume the Live panel at ${viewport.width}x${viewport.height}: ${JSON.stringify(geometry)}`);
  assert.ok(Math.abs(headerLeft - hostLeft) <= 1 && Math.abs(headerRight - hostRight) <= 1 && Math.abs(headerWidth - hostWidth) <= 2 && liveWidth - headerWidth <= 12, `Timeline header and arranger share the Live panel width at ${viewport.width}x${viewport.height}: ${JSON.stringify(geometry)}`);
  assert.ok(hostLeft >= liveLeft && hostRight <= liveRight && hostHeight > 0, `Timeline arranger spans the Live panel remainder at ${viewport.width}x${viewport.height}: ${JSON.stringify(geometry)}`);
  assert.ok(deskLeft >= hostLeft - 1 && deskRight <= hostRight + 1 && deskTop >= hostTop - 1 && deskBottom <= hostBottom + 1, `Timeline desk stays inside its portal host at ${viewport.width}x${viewport.height}: ${JSON.stringify(geometry)}`);
  assert.ok(surfaceLeft >= hostLeft - 1 && surfaceRight <= hostRight + 1 && surfaceTop >= hostTop - 1 && surfaceBottom <= hostBottom + 1, `Timeline surface stays inside its portal host at ${viewport.width}x${viewport.height}: ${JSON.stringify(geometry)}`);
  assert.ok(frameLeft >= surfaceLeft - 1 && frameRight <= surfaceRight + 1 && frameTop >= surfaceTop - 1 && frameBottom <= surfaceBottom + 1 && frameHeight >= hostHeight - 3, `Timeline ruler/lane frame fills the arranger instead of collapsing to its bottom edge at ${viewport.width}x${viewport.height}: ${JSON.stringify(geometry)}`);
  assert.ok(scrollLeft >= frameLeft - 1 && scrollRight <= frameRight + 1 && scrollTop >= frameTop - 1 && scrollBottom <= frameBottom + 1, `Timeline layer scroll stays internal at ${viewport.width}x${viewport.height}: ${JSON.stringify(geometry)}`);
  assert.ok(shelfLeft >= contextLeft - 1 && shelfRight <= contextRight + 1 && Math.abs(shelfTop - contextTop) <= 1 && shelfBottom <= contextBottom + 1 && shelfHeight > 0 && shelfHeight <= contextHeight + 1, `Timeline Sources shelf is contained and top-aligned in the lower-right pane without owning its unused lower whitespace at ${viewport.width}x${viewport.height}: ${JSON.stringify(geometry)}`);
  assert.ok(sourceHeaderHeight >= 36 && sourceHeaderTop >= shelfTop && sourceBodyTop >= sourceHeaderBottom && sourceBodyBottom <= shelfBottom + 1 && sourceBodyHeight >= 120, `Timeline Sources header remains readable and its body owns the remaining height at ${viewport.width}x${viewport.height}: ${JSON.stringify(geometry)}`);
  assert.ok(geometry.sourceShelfOverflowY === 'hidden' && geometry.sourceShelfOuterScroll?.[0] <= geometry.sourceShelfOuterScroll?.[1] + 1 && geometry.sourceShelfHeaderScroll?.[0] <= geometry.sourceShelfHeaderScroll?.[1] + 1 && ['auto', 'scroll'].includes(geometry.sourceShelfBodyOverflowY), `Timeline Sources removes the clipped outer scrollbar and confines overflow to the body at ${viewport.width}x${viewport.height}: ${JSON.stringify(geometry)}`);
  assert.deepEqual([geometry.headerShortTargets, geometry.duplicateSurfaces, geometry.legacyHeaders, geometry.legacyTools], [0, 1, 0, 0], `Timeline keeps one arranger, no duplicate legacy chrome, and full-size header controls at ${viewport.width}x${viewport.height}`);
};

const measureControlDomainGeometry = (client, domain) => evaluate(client, `(() => {
  const domain = ${JSON.stringify(domain)};
  const rect = (element) => {
    if (!(element instanceof Element)) return null;
    const bounds = element.getBoundingClientRect();
    return [bounds.left, bounds.top, bounds.right, bounds.bottom, bounds.width, bounds.height];
  };
  const visible = (element) => {
    if (!(element instanceof HTMLElement)) return false;
    const bounds = element.getBoundingClientRect();
    const style = getComputedStyle(element);
    return bounds.width > 0 && bounds.height > 0 && style.display !== 'none' && style.visibility !== 'hidden';
  };
  const activeDomain = document.querySelector('[data-edit-domain-navigation] [data-control-mode-option][aria-selected="true"]')?.getAttribute('data-control-mode-option') ?? '';
  const upper = document.querySelector('[data-workspace-pane="upper"]');
  const lower = document.querySelector('[data-workspace-pane="lower"]');
  if (domain === 'edit') {
    const matrix = upper?.querySelector('.sceneMatrixPanel');
    const header = matrix?.querySelector('.sceneMatrixSurfaceHeader');
    const scroller = matrix?.querySelector('.sceneMatrixScroller');
    const card = scroller?.querySelector('[data-scene-matrix-cue-id]');
    return { activeDomain, upper: rect(upper), lower: rect(lower), primary: rect(matrix), header: rect(header), content: rect(scroller), item: rect(card), primaryVisible: visible(matrix), contentVisible: visible(scroller), itemVisible: visible(card), primaryPosition: matrix ? getComputedStyle(matrix).position : '' };
  }
  const library = upper?.matches('[data-video-media-library="true"]') ? upper : document.querySelector('[data-video-media-library="true"]');
  const header = library?.querySelector(':scope > .panelHeader');
  const clip = library?.querySelector('.videoMixerClipPane');
  const surface = library?.querySelector('.videoMediaLibrarySurface');
  const card = surface?.querySelector('.videoMediaLibraryItem');
  const importSummary = library?.querySelector('[data-edit-video-import-disclosure] > summary');
  return { activeDomain, upper: rect(upper), lower: rect(lower), primary: rect(library), header: rect(header), content: rect(clip), surface: rect(surface), item: rect(card), importSummary: rect(importSummary), primaryVisible: visible(library), contentVisible: visible(clip), surfaceVisible: visible(surface), itemVisible: visible(card), importVisible: visible(importSummary), surfacePosition: surface ? getComputedStyle(surface).position : '', panelRows: library ? getComputedStyle(library).gridTemplateRows : '', clipRow: clip ? getComputedStyle(clip).gridRowStart : '' };
})()`);

const assertControlDomainGeometry = (geometry, viewport, domain) => {
  assert.equal(geometry?.activeDomain, domain, `${domain} is the selected Control domain at ${viewport.width}x${viewport.height}`);
  assert.ok(geometry.upper && geometry.lower && geometry.primary && geometry.header && geometry.content, `${domain} mounts the authored upper surface at ${viewport.width}x${viewport.height}: ${JSON.stringify(geometry)}`);
  const [, upperTop, , upperBottom, , upperHeight] = geometry.upper;
  const [, lowerTop] = geometry.lower;
  const [primaryLeft, primaryTop, primaryRight, primaryBottom, , primaryHeight] = geometry.primary;
  const [upperLeft, , upperRight] = geometry.upper;
  const [contentLeft, contentTop, contentRight, contentBottom, , contentHeight] = geometry.content;
  assert.ok(upperBottom <= lowerTop + 1, `${domain} upper surface does not overlap the lower band at ${viewport.width}x${viewport.height}: ${JSON.stringify(geometry)}`);
  assert.ok(geometry.primaryVisible && primaryLeft >= upperLeft - 1 && primaryRight <= upperRight + 1 && primaryTop >= upperTop - 1 && primaryBottom <= upperBottom + 1 && primaryHeight >= upperHeight - 24, `${domain} primary content fills and stays inside the upper pane at ${viewport.width}x${viewport.height}: ${JSON.stringify(geometry)}`);
  assert.ok(geometry.contentVisible && contentLeft >= primaryLeft - 1 && contentRight <= primaryRight + 1 && contentTop >= primaryTop - 1 && contentBottom <= primaryBottom + 1 && contentHeight >= 120, `${domain} content owns usable height instead of a collapsed rail at ${viewport.width}x${viewport.height}: ${JSON.stringify(geometry)}`);
  if (domain === 'edit') {
    assert.ok(geometry.itemVisible && geometry.item && geometry.item[5] >= 44, `Lighting exposes at least one usable Scene card at ${viewport.width}x${viewport.height}: ${JSON.stringify(geometry)}`);
    return;
  }
  assert.ok(geometry.surfaceVisible && geometry.surface && geometry.surfacePosition !== 'absolute' && geometry.surface[5] >= 120, `Video Media Library surface is in-flow and usable at ${viewport.width}x${viewport.height}: ${JSON.stringify(geometry)}`);
  assert.ok(geometry.itemVisible && geometry.item && geometry.item[5] >= 48, `Video exposes at least one usable media card at ${viewport.width}x${viewport.height}: ${JSON.stringify(geometry)}`);
  assert.ok(geometry.importVisible && geometry.importSummary && geometry.importSummary[5] >= 43.5, `Video keeps Import Media reachable at ${viewport.width}x${viewport.height}: ${JSON.stringify(geometry)}`);
  assert.equal(geometry.clipRow, '2', `Video clip/library pane stays in explicit row 2 at ${viewport.width}x${viewport.height}: ${JSON.stringify(geometry)}`);
};

const saveScreenshot = async (client, name) => {
  if (!screenshotDir) return;
  const screenshot = await client.send("Page.captureScreenshot", { format: "png", captureBeyondViewport: false, fromSurface: true });
  await writeFile(join(screenshotDir, name), Buffer.from(screenshot.data, "base64"));
};

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
  if (screenshotDir) await mkdir(screenshotDir, { recursive: true });

  for (const viewport of viewports) {
    await client.send("Emulation.setDeviceMetricsOverride", { ...viewport, deviceScaleFactor: 1, mobile: false });
    await client.send("Page.navigate", { url: baseUrl });
    await waitFor(() => evaluate(client, "document.querySelector('.app') && document.readyState === 'complete'"), "app mount");
    assert.equal(await click(client, '[data-workspace-option="control"]'), true);

    assert.equal(await click(client, '[data-edit-domain-navigation] [data-control-mode-option="edit"]'), true);
    await waitFor(() => evaluate(client, "document.querySelector('.editDomainUpperPanel > .sceneMatrixPanel')?.getBoundingClientRect().height > 120"), "Lighting Banks and Scenes surface");
    const lightingGeometry = await measureControlDomainGeometry(client, 'edit');
    assertControlDomainGeometry(lightingGeometry, viewport, 'edit');
    // This fixture renders two Banks, so create a scoped 64px content overflow
    // at its constrained viewport. The listener still receives only the
    // browser-native scroll event from the production scroll host.
    if (viewport.width === 860) {
      const sceneMatrixScrollRange = await createDeterministicScrollRange(client, {
        scrollportSelector: '.sceneMatrixScroller',
        contentSelector: '.sceneMatrixColumns',
        axis: 'left',
      });
      assert.ok(
        (sceneMatrixScrollRange?.range ?? 0) > 0,
        `Scene Matrix scroll host has a deterministic horizontal range: ${JSON.stringify(sceneMatrixScrollRange)}`,
      );
      assert.equal(
        await openSceneMatrixContextMenu(client, '[data-scene-matrix-cue-id]', '.sceneMatrixSceneContextMenu'),
        true,
        "Scene Matrix Scene context menu opens for an external-scroll dismissal",
      );
      await sleep(30);
      const sceneContextScrollDismissal = await observeNativeScroll(client, {
        scrollportSelector: '.sceneMatrixScroller',
        axis: 'left',
        menuSelector: '.sceneMatrixSceneContextMenu',
      });
      assert.deepEqual(
        [
          (sceneContextScrollDismissal?.range ?? 0) > 0,
          sceneContextScrollDismissal?.after !== sceneContextScrollDismissal?.before,
          sceneContextScrollDismissal?.nativeScrollObserved,
          sceneContextScrollDismissal?.menuPresent,
        ],
        [true, true, true, false],
        `Scene Matrix Scene context menu closes after a browser-native horizontal ancestor scroll: ${JSON.stringify(sceneContextScrollDismissal)}`,
      );
      await evaluate(client, `(() => {
        const scrollport = document.querySelector('.sceneMatrixScroller');
        if (!(scrollport instanceof HTMLElement)) return false;
        scrollport.scrollLeft = 0;
        return true;
      })()`);
      await sleep(30);
      assert.equal(
        await openSceneMatrixContextMenu(client, '[data-scene-matrix-column-header]', '.sceneMatrixBankContextMenu:not(.sceneMatrixSceneContextMenu)'),
        true,
        "Scene Matrix Bank context menu opens for an external-scroll dismissal",
      );
      await sleep(30);
      const bankContextScrollDismissal = await observeNativeScroll(client, {
        scrollportSelector: '.sceneMatrixScroller',
        axis: 'left',
        menuSelector: '.sceneMatrixBankContextMenu:not(.sceneMatrixSceneContextMenu)',
      });
      assert.deepEqual(
        [
          (bankContextScrollDismissal?.range ?? 0) > 0,
          bankContextScrollDismissal?.after !== bankContextScrollDismissal?.before,
          bankContextScrollDismissal?.nativeScrollObserved,
          bankContextScrollDismissal?.menuPresent,
        ],
        [true, true, true, false],
        `Scene Matrix Bank context menu closes after a browser-native horizontal ancestor scroll: ${JSON.stringify(bankContextScrollDismissal)}`,
      );
      assert.equal(
        await restoreDeterministicScrollRange(client, {
          contentSelector: '.sceneMatrixColumns',
          axis: 'left',
          previousInlineSize: sceneMatrixScrollRange.previousInlineSize,
        }),
        true,
        "Scene Matrix deterministic scroll range restores its original inline width",
      );
    }
    await saveScreenshot(client, `control-lighting-${viewport.width}x${viewport.height}.png`);

    assert.equal(await click(client, '[data-edit-domain-navigation] [data-control-mode-option="mixer"]'), true);
    await waitFor(() => evaluate(client, "document.querySelector('[data-video-media-library=\"true\"] .videoMediaLibraryItem')?.getBoundingClientRect().height > 48"), "Video Media Library surface");
    const videoGeometry = await measureControlDomainGeometry(client, 'mixer');
    assertControlDomainGeometry(videoGeometry, viewport, 'mixer');
    await saveScreenshot(client, `control-video-${viewport.width}x${viewport.height}.png`);

    assert.equal(await click(client, '[data-edit-domain-navigation] [data-control-mode-option="live"]'), true);
    assert.equal(await waitFor(() => click(client, '[data-timeline-desk-surface="show"]'), "Timeline Show tab"), true);
    await waitFor(() => evaluate(client, "document.querySelectorAll('.timelineVideoClip').length === 1 && document.querySelectorAll('.timelineAudioClip').length === 2"), "authored Timeline media clips");
    await exerciseTimelineWheel(client, viewport);
    const arrangerGeometry = await measureArrangerGeometry(client);
    assertArrangerGeometry(arrangerGeometry, viewport);
    if (viewport.width === 1280) {
      await assertTimelineSourceTargetReveal({ client, click, evaluate, waitFor });
    }
    await saveScreenshot(client, `control-timeline-${viewport.width}x${viewport.height}.png`);
    assert.equal(await click(client, '.timelineToolsDisclosure > summary'), true);
    await waitFor(() => evaluate(client, "document.querySelector('.timelineToolsDisclosure[open] .timelinePerformanceEditor')?.getBoundingClientRect().height > 0"), "Timeline performance disclosure");
    assert.equal(await click(client, '.timelinePerformanceEditor [data-timeline-bank] > summary'), true);
    assert.equal(await click(client, '.timelinePerformanceEditor .timelineCueAudioEditor > summary'), true);
    assert.equal(await click(client, '.timelinePerformanceEditor .timelinePhaseEditor > summary'), true);
    await sleep(100);
    const targetProof = [];
    for (const selector of ['.timelinePerformanceEditor [data-timeline-bank]', '.timelinePerformanceEditor .timelineCueAudioEditor', '.timelinePerformanceEditor .timelinePhaseEditor', '.timelineOperatorBar > .timelineLoopControls']) {
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
    assert.equal(await openTimelineContextMenuGroup(client, "timing"), true, "Timeline Timing disclosure opens for Split actions");
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
    }, "Alt-isolated Split fixture proof: fresh right item DOM, the fixture request carries only the chosen member, and fresh focus is restored (no native IPC exercised)");
    const videoSelected = await evaluate(client, `(() => {
      const clip = document.querySelector('[data-timeline-video-clip-id="800"]');
      if (!(clip instanceof Element)) return false;
      clip.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
      clip.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true, clientX: 240, clientY: 180 }));
      return true;
    })()`);
    assert.equal(videoSelected, true);
    await sleep(80);
    const menuStructure = await evaluate(client, `(() => {
      const menu = document.querySelector('.timelineItemContextMenu');
      if (!(menu instanceof HTMLElement)) return null;
      const actionsRoot = menu.querySelector(':scope > details.timelineItemContextMenuActions');
      if (!(actionsRoot instanceof HTMLDetailsElement)) return null;
      const rootSummary = actionsRoot.querySelector(':scope > summary');
      const initiallyCollapsedRoot = !actionsRoot.open;
      actionsRoot.open = true;
      const groups = [...actionsRoot.querySelectorAll(':scope > .timelineItemContextMenuActionsGroups > details[data-timeline-context-menu-group]')];
      const collapsed = groups.every((group) => !group.open);
      const summaries = groups.map((group) => group.querySelector(':scope > summary')?.textContent?.trim() ?? '');
      const groupActionHeights = groups.map((group) => {
        const summary = group.querySelector(':scope > summary');
        if (!(summary instanceof HTMLElement)) return [];
        summary.click();
        const heights = [...group.querySelectorAll('button')].map((button) => button.getBoundingClientRect().height);
        summary.click();
        return heights;
      });
      const summaryHeights = groups.map((group) => group.querySelector(':scope > summary')?.getBoundingClientRect().height ?? 0);
      const rootSummaryHeight = rootSummary instanceof HTMLElement ? rootSummary.getBoundingClientRect().height : 0;
      actionsRoot.open = false;
      return {
        groupCount: groups.length,
        summaries,
        collapsed,
        initiallyCollapsedRoot,
        groupActionCounts: groupActionHeights.map((heights) => heights.length),
        allGroupActions44: groupActionHeights.flat().every((height) => height >= 44),
        nativeDisclosureSemantics: menu.getAttribute('role') === 'group'
          && !rootSummary?.hasAttribute('role')
          && groups.every((group) => group instanceof HTMLDetailsElement
            && !group.querySelector(':scope > summary')?.hasAttribute('role')),
        summaryHeights,
        rootSummaryHeight,
        compactHeight: menu.getBoundingClientRect().height,
        width: menu.getBoundingClientRect().width,
      };
    })()`);
    assert.deepEqual(
      [
        menuStructure?.groupCount,
        menuStructure?.summaries,
        menuStructure?.collapsed,
        menuStructure?.initiallyCollapsedRoot,
        menuStructure?.groupActionCounts,
        menuStructure?.allGroupActions44,
        menuStructure?.nativeDisclosureSemantics,
        menuStructure?.summaryHeights,
      ],
      [4, ["Selection", "Clipboard", "Timing", "Lane"], true, true, [2, 3, 8, 2], true, true, [44, 44, 44, 44]],
      "Timeline item menu nests its four native disclosure groups inside one initially closed native Actions disclosure with visible 44px actions",
    );
    assert.ok(
      menuStructure?.compactHeight >= 150 && menuStructure.compactHeight <= 158
        && menuStructure.rootSummaryHeight >= 43.5 && menuStructure.rootSummaryHeight <= 44.5
        && menuStructure.width >= 208 && menuStructure.width <= 212,
      `Timeline item menu keeps the shortened Actions-root closed geometry at 210px width: ${JSON.stringify(menuStructure)}`,
    );
    const bottomEdgePlacement = await evaluate(client, `(() => {
      const clip = document.querySelector('[data-timeline-video-clip-id="800"]');
      if (!(clip instanceof Element)) return null;
      const viewportHeight = window.innerHeight;
      clip.dispatchEvent(new MouseEvent('contextmenu', {
        bubbles: true,
        cancelable: true,
        clientX: 240,
        clientY: viewportHeight - 1,
      }));
      const menu = document.querySelector('.timelineItemContextMenu');
      if (!(menu instanceof HTMLElement)) return null;
      return { top: menu.getBoundingClientRect().top, viewportHeight };
    })()`);
    assert.ok(
      bottomEdgePlacement
        && menuStructure?.compactHeight
        && Math.abs(bottomEdgePlacement.top - (bottomEdgePlacement.viewportHeight - menuStructure.compactHeight)) <= 1,
      `Timeline item menu bottom-edge placement uses its shortened closed height instead of the legacy 298px clamp: ${JSON.stringify({ bottomEdgePlacement, compactHeight: menuStructure?.compactHeight })}`,
    );
    await evaluate(client, `(() => {
      const clip = document.querySelector('[data-timeline-video-clip-id="800"]');
      if (!(clip instanceof Element)) return false;
      clip.dispatchEvent(new MouseEvent('contextmenu', {
        bubbles: true,
        cancelable: true,
        clientX: 240,
        clientY: 180,
      }));
      return true;
    })()`);
    await sleep(30);
    const retainedInputSemantics = await evaluate(client, `(() => {
      const menu = document.querySelector('.timelineItemContextMenu');
      const summary = menu?.querySelector(':scope > details > summary');
      const overview = document.querySelector('.timelineOverview');
      if (!(menu instanceof HTMLElement) || !(summary instanceof HTMLElement) || !(overview instanceof Element)) return null;
      summary.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true, cancelable: true, button: 0 }));
      const insidePointerRetained = document.querySelector('.timelineItemContextMenu') === menu;
      const preventedEscape = new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true });
      preventedEscape.preventDefault();
      window.dispatchEvent(preventedEscape);
      const preventedEscapeRetained = document.querySelector('.timelineItemContextMenu') === menu;
      const priorGesture = overview.getAttribute('data-timeline-gesture-active');
      overview.setAttribute('data-timeline-gesture-active', 'true');
      window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true }));
      if (priorGesture === null) overview.removeAttribute('data-timeline-gesture-active');
      else overview.setAttribute('data-timeline-gesture-active', priorGesture);
      return {
        insidePointerRetained,
        preventedEscapeRetained,
        gestureEscapeRetained: document.querySelector('.timelineItemContextMenu') === menu,
      };
    })()`);
    assert.deepEqual(
      retainedInputSemantics,
      { insidePointerRetained: true, preventedEscapeRetained: true, gestureEscapeRetained: true },
      "inside pointer input and reserved Escape states retain the Timeline item menu",
    );
    assert.equal(
      await openTimelineVideoContextMenu(client, viewport.height - 1),
      true,
      "Timeline item menu opens near the lower edge for internal-scroll retention",
    );
    await sleep(30);
    assert.equal(await openTimelineContextMenuActions(client), true, "Timeline item Actions disclosure opens for internal scroll proof");
    assert.equal(await openTimelineContextMenuGroup(client, "timing"), true, "Timeline item Timing actions open for internal scroll proof");
    await sleep(30);
    const itemMenuInternalScroll = await observeNativeScroll(client, {
      scrollportSelector: '.timelineItemContextMenu',
      axis: 'top',
      menuSelector: '.timelineItemContextMenu',
    });
    assert.deepEqual(
      [
        (itemMenuInternalScroll?.range ?? 0) > 0,
        itemMenuInternalScroll?.after !== itemMenuInternalScroll?.before,
        itemMenuInternalScroll?.nativeScrollObserved,
        itemMenuInternalScroll?.menuPresent,
      ],
      [true, true, true, true],
      `Timeline item menu retains its internally scrolled expanded actions after a browser-native scroll: ${JSON.stringify(itemMenuInternalScroll)}`,
    );
    // Keep this test-only overflow local to the real Timeline scroll host; it
    // lets the browser dispatch the same event produced by all user scroll
    // inputs without altering production layout.
    if (viewport.width === 860) {
      const timelineLayerScrollRange = await createDeterministicScrollRange(client, {
        scrollportSelector: '.timelineLayerScrollport',
        contentSelector: '.timelineLayerScrollContent',
        axis: 'top',
      });
      assert.ok(
        (timelineLayerScrollRange?.range ?? 0) > 0,
        `Timeline layer scroll host has a deterministic vertical range: ${JSON.stringify(timelineLayerScrollRange)}`,
      );
      const itemMenuExternalScrollDismissal = await observeNativeScroll(client, {
        scrollportSelector: '.timelineLayerScrollport',
        axis: 'top',
        menuSelector: '.timelineItemContextMenu',
      });
      assert.deepEqual(
        [
          (itemMenuExternalScrollDismissal?.range ?? 0) > 0,
          itemMenuExternalScrollDismissal?.after !== itemMenuExternalScrollDismissal?.before,
          itemMenuExternalScrollDismissal?.nativeScrollObserved,
          itemMenuExternalScrollDismissal?.menuPresent,
        ],
        [true, true, true, false],
        `Timeline item menu closes after a browser-native vertical ancestor scroll: ${JSON.stringify(itemMenuExternalScrollDismissal)}`,
      );
      assert.equal(
        await restoreDeterministicScrollRange(client, {
          contentSelector: '.timelineLayerScrollContent',
          axis: 'top',
          previousInlineSize: timelineLayerScrollRange.previousInlineSize,
        }),
        true,
        "Timeline deterministic scroll range restores its original inline height",
      );
      assert.equal(await openTimelineVideoContextMenu(client), true, "Timeline item menu reopens after ancestor-scroll dismissal");
      await sleep(30);
    }
    const state = await measure(client);
    assert.ok(state.rect[0] > 0 && state.rect[1] > 0, "Timeline surface has visible nonzero geometry");
    assert.ok(
      state.timelineGrid && state.timelineGrid.major > 0 && state.timelineGrid.minor > 0
        && state.timelineGrid.lineCount === state.timelineGrid.major + state.timelineGrid.minor
        && state.timelineGrid.labels === state.timelineGrid.major
        && state.timelineGrid.labelsOnMinor === 0
        && state.timelineGrid.subdivision >= 1
        && state.timelineGrid.majorStepMs > state.timelineGrid.minorStepMs,
      `Timeline renders bounded major/minor grid lines with major-only labels: ${JSON.stringify(state.timelineGrid)}`,
    );
    assert.equal(state.snapState, "Snap: Grid 500 ms · Edges: ON", "new Timeline sessions expose the Grid 500 ms + edge magnet defaults");
    assert.equal(state.edgeMagnetState, "on", "new Timeline sessions expose edge magnet enabled separately from grid quantization");
    assert.deepEqual([state.bankOpen, state.bankItems, state.bankActive, state.followLegend], [true, 2, 1, true]);
    assert.match(state.followState, /transitioning 50%/);
    assert.deepEqual(
      [state.followRuntimeDetails, state.followRuntimeBadges >= 1, state.operatorFollowState, state.operatorFollowAbort, state.followAbortButtons >= 1, state.followAbortShortTargets],
      [1, true, "Follow: transitioning 50%", 1, true, 0],
      "Visible Operator and Performance Timeline surfaces expose runtime Follow truth with 44px abort targets",
    );
    assert.equal(await evaluate(client, `(() => {
      const outside = document.createElement('button');
      outside.type = 'button';
      outside.id = 'timeline-menu-outside-focus-target';
      outside.addEventListener('pointerdown', () => outside.focus(), { once: true });
      document.body.append(outside);
      outside.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true, cancelable: true, button: 0 }));
      return true;
    })()`), true);
    await sleep(30);
    assert.deepEqual(
      await evaluate(client, `(() => {
        const outside = document.getElementById('timeline-menu-outside-focus-target');
        const result = {
          closed: document.querySelector('.timelineItemContextMenu') === null,
          outsideFocused: document.activeElement === outside,
        };
        outside?.remove();
        return result;
      })()`),
      { closed: true, outsideFocused: true },
      "outside primary pointer closes the Timeline item menu without stealing focus from the outside target",
    );
    assert.equal(
      await evaluate(client, "document.querySelector('.timelineItemContextMenu') === null"),
      true,
      "the Timeline item menu starts closed before the pre-prevented outside dismissal regression",
    );
    await evaluate(client, `(() => {
      window.__syndocalTimelineMenuOutsidePrePrevented = false;
      window.__syndocalTimelineMenuOutsidePrePreventer = (event) => {
        if (event.button !== 0) return;
        if (event.target instanceof Element && event.target.closest('.timelineItemContextMenu')) return;
        event.preventDefault();
        window.__syndocalTimelineMenuOutsidePrePrevented = true;
      };
      window.addEventListener('pointerdown', window.__syndocalTimelineMenuOutsidePrePreventer, { capture: true });
    })()`);
    assert.equal(await openTimelineVideoContextMenu(client), true, "Timeline item menu mounts under an already-installed upstream capture pre-preventer");
    await sleep(30);
    await evaluate(client, `(() => {
      const outside = document.createElement('button');
      outside.type = 'button';
      document.body.append(outside);
      outside.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true, cancelable: true, button: 0 }));
      outside.remove();
    })()`);
    await sleep(30);
    assert.deepEqual(
      [
        await evaluate(client, "window.__syndocalTimelineMenuOutsidePrePrevented === true"),
        await evaluate(client, "document.querySelector('.timelineItemContextMenu') === null"),
      ],
      [true, true],
      "an upstream capture listener that pre-prevents the outside primary pointerdown no longer keeps the Timeline item menu open",
    );
    await evaluate(client, `(() => {
      window.removeEventListener('pointerdown', window.__syndocalTimelineMenuOutsidePrePreventer, { capture: true });
      delete window.__syndocalTimelineMenuOutsidePrePreventer;
      delete window.__syndocalTimelineMenuOutsidePrePrevented;
    })()`);
    const abortFocusProof = await evaluate(client, `(() => {
      const button = document.querySelector('[data-timeline-follow-operator-abort]');
      if (!(button instanceof HTMLButtonElement)) return null;
      button.focus();
      const rect = button.getBoundingClientRect();
      const proof = {
        enabled: !button.disabled,
        connected: button.isConnected,
        visible: rect.width > 0 && rect.height > 0,
        target44: rect.width >= 44 && rect.height >= 44,
      };
      button.click();
      return proof;
    })()`);
    assert.deepEqual(
      [abortFocusProof?.enabled, abortFocusProof?.connected, abortFocusProof?.visible, abortFocusProof?.target44],
      [true, true, true, true],
      `Timeline Follow abort control is enabled and mounted before activation: ${JSON.stringify(abortFocusProof)}`,
    );
    await sleep(50);
    assert.equal(
      await evaluate(client, "document.activeElement?.matches?.('[data-timeline-follow-operator-abort]') === true"),
      true,
      "visible Operator Timeline Follow abort retains focus after its bounded runtime-only result",
    );
    assert.equal(await openTimelineVideoContextMenu(client), true, "Timeline item menu reopens after outside dismissal");
    await sleep(30);
    await evaluate(client, "document.body.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true, cancelable: true, button: 2 }))");
    await sleep(30);
    assert.equal(
      await evaluate(client, "document.querySelector('.timelineItemContextMenu') === null"),
      true,
      "outside secondary pointer dismisses the stale Timeline item menu",
    );
    assert.equal(await openTimelineVideoContextMenu(client), true, "Timeline item menu reopens for Escape dismissal");
    await sleep(30);
    const escapeDismissed = await evaluate(client, `(() => {
      const event = new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true });
      window.dispatchEvent(event);
      return event.defaultPrevented;
    })()`);
    await sleep(30);
    assert.deepEqual(
      [
        escapeDismissed,
        await evaluate(client, "document.querySelector('.timelineItemContextMenu') === null"),
        await evaluate(client, "document.activeElement?.getAttribute('data-timeline-video-clip-id') ?? ''"),
      ],
      [true, true, "800"],
      "Escape dismisses the Timeline item menu and restores focus to its invoker",
    );
    if (viewport.width === 860) {
      assert.equal(await openTimelineLayerContextMenu(client), true, "Timeline layer menu opens for ancestor-scroll dismissal");
      await sleep(30);
      const timelineLayerMenuScrollRange = await createDeterministicScrollRange(client, {
        scrollportSelector: '.timelineLayerScrollport',
        contentSelector: '.timelineLayerScrollContent',
        axis: 'top',
      });
      assert.ok(
        (timelineLayerMenuScrollRange?.range ?? 0) > 0,
        `Timeline layer menu host has a deterministic vertical range: ${JSON.stringify(timelineLayerMenuScrollRange)}`,
      );
      const layerMenuScrollDismissal = await observeNativeScroll(client, {
        scrollportSelector: '.timelineLayerScrollport',
        axis: 'top',
        menuSelector: '.timelineLayerContextMenu',
      });
      assert.deepEqual(
        [
          (layerMenuScrollDismissal?.range ?? 0) > 0,
          layerMenuScrollDismissal?.after !== layerMenuScrollDismissal?.before,
          layerMenuScrollDismissal?.nativeScrollObserved,
          layerMenuScrollDismissal?.menuPresent,
        ],
        [true, true, true, false],
        `Timeline layer menu closes after a browser-native vertical ancestor scroll: ${JSON.stringify(layerMenuScrollDismissal)}`,
      );
      assert.equal(
        await restoreDeterministicScrollRange(client, {
          contentSelector: '.timelineLayerScrollContent',
          axis: 'top',
          previousInlineSize: timelineLayerMenuScrollRange.previousInlineSize,
        }),
        true,
        "Timeline layer menu deterministic scroll range restores its original inline height",
      );
    }
    assert.equal(await openTimelineVideoContextMenu(client), true, "Timeline item menu reopens for action execution");
    await sleep(30);
    assert.equal(state.phaseEditorOpen, true);
    assert.deepEqual(
      [state.cueAudioOpen, state.cueAudioControls, state.cueAudioState, state.cueAudioCheckboxes],
      [true, 0, 'Loading settings', 0],
      "Timeline keeps a read-only Cue Audio status surface; editable routing controls live only in Setup I/O Audio",
    );
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
    assert.equal(await openTimelineContextMenuGroup(client, "timing"), true, "Timeline Timing disclosure opens for linked Split dispatch");
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
    }, "real Split menu action sends the exact linked selection, primary, playhead, and isolate flag to the fixture callback (fixture request proof only, never authoritative/native completion)");
    assert.deepEqual(
      [splitProof.menuClosed, splitProof.focusedVideoId, splitProof.freshVideoMounted, splitProof.freshAudioMounted, splitProof.selectedVideo, splitProof.selectedAudio],
      [true, "1800", 1, 1, 1, 1],
      "Fixture-result DOM proof only: the split fixture response mounts fresh right IDs, closes the menu, restores their linked selection, and focuses the fresh primary item (not a native acknowledgment)",
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
      assert.equal(await openTimelineContextMenuGroup(client, "timing"), true, `${splitCase.kind} opens the Timing disclosure for Split`);
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
    assert.equal(await openTimelineContextMenuGroup(client, "clipboard"), true, "Timeline Clipboard disclosure opens for Copy");
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
      const laneDragGeometry = await evaluate(client, `(() => {
        const performanceDetails = [...document.querySelectorAll('.timelinePerformanceEditor details')];
        for (const detail of performanceDetails) {
          if (detail instanceof HTMLDetailsElement && detail.open) {
            detail.querySelector(':scope > summary')?.click();
          }
        }
        const toolsDisclosure = document.querySelector('.timelineToolsDisclosure');
        if (toolsDisclosure instanceof HTMLDetailsElement && toolsDisclosure.open) {
          toolsDisclosure.querySelector(':scope > summary')?.click();
        }
        const source = document.querySelector('[data-timeline-video-clip-id="800"]');
        const target = document.querySelector('[data-timeline-layer-gutter][data-timeline-layer-id="15"]');
        if (!(source instanceof Element) || !(target instanceof Element)) return null;
        source.scrollIntoView({ block: 'center', inline: 'nearest' });
        target.scrollIntoView({ block: 'center', inline: 'nearest' });
        const sourceRect = source.getBoundingClientRect();
        const targetRect = target.getBoundingClientRect();
        return {
          detailsClosed: performanceDetails.every((detail) => !(detail instanceof HTMLDetailsElement) || !detail.open)
            && (!(toolsDisclosure instanceof HTMLDetailsElement) || !toolsDisclosure.open),
          source: [sourceRect.width, sourceRect.height],
          target: [targetRect.width, targetRect.height],
        };
      })()`);
      assert.deepEqual(
        [laneDragGeometry?.detailsClosed, laneDragGeometry?.source[0] > 0, laneDragGeometry?.source[1] > 0, laneDragGeometry?.target[0] > 0, laneDragGeometry?.target[1] > 0],
        [true, true, true, true, true],
        "1280 linked lane drag closes performance disclosures and remeasures visible source and target geometry",
      );
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
        "five-domain lane fixture-result DOM state",
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
          ], "Fixture request payload proof: Scene pointer crosses Lighting to the explicit Video lane and projects the group ordinal");
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
        "Scene pointer drop on Audio emits no mutation request (fixture or native)",
      );
      const beforeInvalidPointer = await evaluate(client, "window.__syndocalTimelineLaneMoveFixtureCalls.length");
      await dragTimelineItemToLane(client, '[data-timeline-automation-kind="video"][data-timeline-automation-id="2"]', 10);
      assert.equal(await evaluate(client, "window.__syndocalTimelineLaneMoveFixtureCalls.length"), beforeInvalidPointer, "wrong-kind pointer drop emits no mutation request (fixture or native)");
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
        "Fixture request payload proof: Scene keyboard navigation crosses the canonical Video-to-Lighting boundary",
      );
      const beforeContextMove = beforeCrossKindKeyboard + 1;
      assert.equal(await evaluate(client, `(() => {
        const item = document.querySelector('[data-timeline-automation-kind="video"][data-timeline-automation-id="2"]');
        if (!(item instanceof Element)) return false;
        item.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true, clientX: 300, clientY: 220 }));
        return true;
      })()`), true);
      await sleep(50);
      assert.equal(await openTimelineContextMenuGroup(client, "lane"), true, "Timeline Lane disclosure opens for lane actions");
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
        "keyboard-operable context action sends the exact five-domain group request to the fixture callback (fixture request proof only)",
      );
      assert.deepEqual(
        [laneMoveFinal.audioLane, laneMoveFinal.eventLane, laneMoveFinal.videoLane, laneMoveFinal.lightingAutomationLane, laneMoveFinal.videoAutomationLane],
        [10, 12, 14, 12, 14],
        "Fixture-result local apply mounts every explicit lane target (DOM projection of the fixture response, not native completion)",
      );
      assert.deepEqual(
        [laneMoveFinal.selectedVideo, laneMoveFinal.selectedAudio, laneMoveFinal.selectedEvent, laneMoveFinal.selectedAutomation],
        [1, 1, 1, 1],
        "Fixture-result apply preserves the logical linked selection across every visible domain",
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
        "Scene keyboard move with no compatible canonical predecessor emits no mutation request (fixture or native)",
      );
      assert.equal(await evaluate(client, `(() => {
        const item = document.querySelector('[data-timeline-automation-kind="video"][data-timeline-automation-id="2"]');
        if (!(item instanceof Element)) return false;
        item.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowUp', shiftKey: true, bubbles: true, cancelable: true }));
        return true;
      })()`), true);
      await sleep(50);
      assert.equal(await evaluate(client, "window.__syndocalTimelineLaneMoveFixtureCalls.length"), beforeContextMove + 1, "boundary keyboard move emits no mutation request (fixture or native)");
    }
    assert.equal(targetProof.every((proof) => proof?.rect[0] > 0 && proof?.rect[1] > 0 && proof.count > 0 && proof.short === 0), true, `Timeline bank, Cue Audio, Phase, and loop controls preserve 44px targets at ${viewport.width}x${viewport.height}: ${JSON.stringify(targetProof)}`);
    assert.equal(state.shortTargets, 0, `Visible Timeline performance controls preserve 44px targets at ${viewport.width}x${viewport.height}`);
    assert.equal(state.fixedOuter, true, `Timeline disclosures keep app/document outer scroll fixed at ${viewport.width}x${viewport.height}`);
    console.log(`${viewport.width}x${viewport.height}: phases=${state.phaseLabels.join('/')} bank=${state.bankItems} media=${state.videoClips}+${state.audioClips} selected=${state.selectedVideo}+${state.selectedAudio} (fixture request/DOM/geometry proof only -- not native or authoritative completion)`);
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
    "real pointer gestures send every linked Timeline domain to the direct-resize trim fixture and preserve Alt isolate (fixture request/DOM proof, never native completion)",
  );
  assert.deepEqual(
    directResizeCalls.scene.map(({ event_id, edge }) => [event_id, edge]),
    [[102, "end"]],
    "unlinked Scene resize retains its local fixture route (harness callback, not native IPC)",
  );
  assert.deepEqual(directResizeCalls.audio.map(({ id }) => id), [202], "unlinked Audio resize retains its local fixture route (harness callback, not native IPC)");
  assert.equal(directResizeCalls.audio[0].start_ms > 4_400, true, "unlinked Audio start edge moves later");
  assert.equal(directResizeCalls.audio[0].offset_ms > 400, true, "unlinked Audio start trim advances source offset");
  assert.equal(directResizeCalls.audio[0].duration_ms < 800, true, "unlinked Audio start trim shortens duration");
  assert.deepEqual(directResizeCalls.video.map(({ id }) => id), [302], "unlinked Video resize retains its local fixture route (harness callback, not native IPC)");
  assert.equal(directResizeCalls.video[0].start_ms, 7_000, "unlinked Video end trim preserves start");
  assert.equal(directResizeCalls.video[0].offset_ms, 400, "unlinked Video end trim preserves source offset");
  assert.equal(directResizeCalls.video[0].duration_ms > 800, true, "unlinked Video end edge extends duration");
  assert.deepEqual(
    directResizeCalls.automation.map(({ kind, automation_id, edge }) => [kind, automation_id, edge]),
    [["lighting", 402, "start"], ["video", 502, "end"]],
    "unlinked Lighting and Video automation resize retain their local fixture routes (harness callbacks, not native IPC)",
  );
  assert.equal(
    directResizeCalls.trim.every(({ boundary_ms }) => Number.isInteger(boundary_ms) && boundary_ms >= 0),
    true,
    "linked direct-resize fixture requests carry finite non-negative integer millisecond boundaries",
  );
  const trimByKind = new Map(directResizeCalls.trim.map((call) => [call.item.kind, call]));
  assert.equal(trimByKind.get("lighting_event").boundary_ms > 500, true, "linked Scene start edge moves later");
  assert.equal(trimByKind.get("audio_clip").boundary_ms > 3_900, true, "linked Audio end edge moves later");
  assert.equal(trimByKind.get("video_clip").boundary_ms > 5_700, true, "linked Video start edge moves later");
  assert.equal(trimByKind.get("lighting_automation").boundary_ms > 9_100, true, "linked Lighting automation end edge moves later");
  assert.equal(trimByKind.get("video_automation").boundary_ms > 8_300, true, "linked Video automation start edge moves later");
  console.log("direct resize fixture: linked Scene/Audio/Video/Lighting automation/Video automation + unlinked local routes + Alt isolate ok (fixture request/DOM proof only -- not native completion)");
} finally {
  client?.close();
  await stopChild(browser);
  await stopChild(vite);
  if (profileDir) await rm(profileDir, { recursive: true, force: true });
}
