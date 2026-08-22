import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const appRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const host = "127.0.0.1";
const vitePort = 5197;
const cdpPort = 9247;
const baseUrl = `http://${host}:${vitePort}/?syndocalViewportFixture=timeline-layered`;
const screenshotDir = resolve(
  process.env.SYNDOCAL_CONTROL_SCREENSHOT_DIR ?? "C:\\TEMP\\syndocal-control-ui-checkpoints",
);
const productMinimumWindow = { width: 960, height: 640 };
// The show-core uses these four physical desktop classes. Smaller browser and
// detached-pane tests remain supplemental and do not define this acceptance.
const viewports = [
  { width: 3840, height: 2160 },
  { width: 2560, height: 1440 },
  { width: 1920, height: 1080 },
  { width: 1280, height: 720 },
];
const browserCandidates = [
  process.env.CHROME_PATH,
  "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe",
  "C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe",
  "C:\\Program Files\\Microsoft\\Edge\\Application\\msedge.exe",
].filter(Boolean);

const sleep = (milliseconds) => new Promise((resolveSleep) => setTimeout(resolveSleep, milliseconds));
const stopChild = async (child) => {
  if (!child || child.exitCode !== null) return;
  const exited = new Promise((resolveExit) => child.once("exit", resolveExit));
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
    } catch (error) {
      lastError = error;
    }
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

  close() {
    this.socket.close();
  }
}

const evaluate = async (client, expression) => {
  const result = await client.send("Runtime.evaluate", {
    expression,
    awaitPromise: true,
    returnByValue: true,
  });
  if (result.exceptionDetails) throw new Error(result.exceptionDetails.text);
  return result.result.value;
};

const clickVisible = async (client, selector) => evaluate(client, `(() => {
  const visible = (element) => {
    if (!(element instanceof HTMLElement)) return false;
    const rect = element.getBoundingClientRect();
    const style = getComputedStyle(element);
    return rect.width > 0 && rect.height > 0 && style.display !== 'none' && style.visibility !== 'hidden';
  };
  const element = [...document.querySelectorAll(${JSON.stringify(selector)})].find(visible);
  if (!(element instanceof HTMLElement)) return false;
  element.click();
  return true;
})()`);

const isVisibleExpression = (elementExpression) => `(() => {
  const element = ${elementExpression};
  if (!(element instanceof HTMLElement)) return false;
  const rect = element.getBoundingClientRect();
  const style = getComputedStyle(element);
  return rect.width > 0 && rect.height > 0 && style.display !== 'none' && style.visibility !== 'hidden';
})()`;

const pressEscape = async (client) => {
  await client.send("Input.dispatchKeyEvent", { type: "keyDown", key: "Escape", code: "Escape" });
  await client.send("Input.dispatchKeyEvent", { type: "keyUp", key: "Escape", code: "Escape" });
};

const pressTab = async (client) => {
  await client.send("Input.dispatchKeyEvent", { type: "keyDown", key: "Tab", code: "Tab" });
  await client.send("Input.dispatchKeyEvent", { type: "keyUp", key: "Tab", code: "Tab" });
};

const exerciseExpandedTabOrder = async (client) => {
  const focused = await evaluate(client, `(() => {
    const toggle = document.querySelector('[data-timeline-pane-expand-toggle]');
    if (!(toggle instanceof HTMLElement)) return false;
    toggle.focus();
    return document.activeElement === toggle;
  })()`);
  assert.equal(focused, true, "Timeline expand toggle accepts focus before expanded Tab traversal");
  const sequence = [];
  let lowerBandHit = false;
  let upperPortalHit = false;
  for (let index = 0; index < 80; index += 1) {
    await pressTab(client);
    const state = await evaluate(client, `(() => {
      const active = document.activeElement;
      const lower = document.querySelector('[data-workspace-pane="lower"]');
      const upper = document.querySelector('[data-timeline-arranger-upper]');
      const header = document.querySelector('[data-timeline-arranger-header]');
      const visible = (element) => {
        if (!(element instanceof HTMLElement)) return false;
        const rect = element.getBoundingClientRect();
        const style = getComputedStyle(element);
        return rect.width > 0 && rect.height > 0 && style.display !== 'none' && style.visibility !== 'hidden';
      };
      return {
        tag: active?.tagName ?? '',
        label: active?.getAttribute('aria-label') ?? active?.getAttribute('title') ?? active?.textContent?.trim().slice(0, 48) ?? '',
        lower: Boolean(lower && active && lower.contains(active)),
        upperPortal: Boolean(upper && active && upper.contains(active) && visible(active)),
        header: Boolean(header && active && header.contains(active) && visible(active)),
      };
    })()`);
    sequence.push(state);
    lowerBandHit ||= state.lower;
    upperPortalHit ||= state.upperPortal;
  }
  return {
    lowerBandHit,
    upperPortalHit,
    visited: sequence.filter((state) => state.upperPortal || state.lower).slice(0, 12),
  };
};

const exercisePopupLastTarget = async (client, selector) => evaluate(client, `(async () => {
  const popup = document.querySelector(${JSON.stringify(selector)});
  if (!(popup instanceof HTMLElement)) return null;
  for (const details of popup.querySelectorAll('details')) details.open = true;
  await new Promise((resolveFrame) => requestAnimationFrame(resolveFrame));
  const candidates = [...popup.querySelectorAll('button, input, select, textarea, [role="button"]')].filter((element) => {
    if (!(element instanceof HTMLElement)) return false;
    const rect = element.getBoundingClientRect();
    const style = getComputedStyle(element);
    return rect.width > 0 && rect.height > 0 && style.display !== 'none' && style.visibility !== 'hidden' && !element.hasAttribute('disabled') && !element.closest('details:not([open])');
  });
  const target = candidates.at(-1);
  if (!(target instanceof HTMLElement)) return null;
  target.scrollIntoView({ block: 'nearest', inline: 'nearest' });
  popup.scrollTop = popup.scrollHeight;
  await new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame)));
  const rectArray = (rect) => [rect.x, rect.y, rect.width, rect.height, rect.right, rect.bottom];
  const popupRect = popup.getBoundingClientRect();
  const targetRect = target.getBoundingClientRect();
  const layout = popup.closest('.layout') ?? document.querySelector('.layout');
  const layoutRect = layout?.getBoundingClientRect() ?? null;
  const within = (inner, outer) => Boolean(
    outer && inner.left >= outer.left - 1 && inner.top >= outer.top - 1 &&
    inner.right <= outer.right + 1 && inner.bottom <= outer.bottom + 1
  );
  const visibleLeft = Math.max(targetRect.left, popupRect.left, layoutRect?.left ?? 0, 0);
  const visibleTop = Math.max(targetRect.top, popupRect.top, layoutRect?.top ?? 0, 0);
  const visibleRight = Math.min(targetRect.right, popupRect.right, layoutRect?.right ?? innerWidth, innerWidth);
  const visibleBottom = Math.min(targetRect.bottom, popupRect.bottom, layoutRect?.bottom ?? innerHeight, innerHeight);
  const point = {
    x: visibleLeft + Math.max(0, visibleRight - visibleLeft) / 2,
    y: visibleTop + Math.max(0, visibleBottom - visibleTop) / 2,
  };
  const hit = visibleRight > visibleLeft && visibleBottom > visibleTop ? document.elementFromPoint(point.x, point.y) : null;
  return {
    popup: rectArray(popupRect),
    layout: layoutRect ? rectArray(layoutRect) : null,
    target: rectArray(targetRect),
    targetLabel: target.getAttribute('aria-label') ?? target.textContent?.trim().slice(0, 64) ?? target.tagName,
    targetHeight: targetRect.height,
    popupInsideLayout: layoutRect ? within(popupRect, layoutRect) : false,
    popupInsideViewport: popupRect.left >= -1 && popupRect.top >= -1 && popupRect.right <= innerWidth + 1 && popupRect.bottom <= innerHeight + 1,
    targetInsidePopup: within(targetRect, popupRect),
    hit: hit === target || target.contains(hit),
    point,
    scrollTop: popup.scrollTop,
    scrollHeight: popup.scrollHeight,
    clientHeight: popup.clientHeight,
  };
})()`);

const rectArray = (rect) => [rect.x, rect.y, rect.width, rect.height, rect.right, rect.bottom];

const capture = async (client, name) => {
  const result = await client.send("Page.captureScreenshot", {
    format: "png",
    captureBeyondViewport: false,
  });
  const path = join(screenshotDir, name);
  await writeFile(path, Buffer.from(result.data, "base64"));
  return path;
};

const measureLighting = (client) => evaluate(client, `(() => {
  const visible = (element) => {
    if (!(element instanceof HTMLElement)) return false;
    const rect = element.getBoundingClientRect();
    const style = getComputedStyle(element);
    return rect.width > 0 && rect.height > 0 && style.display !== 'none' && style.visibility !== 'hidden';
  };
  const rectArray = (value) => [value.x, value.y, value.width, value.height, value.right, value.bottom];
  const rect = (element) => element instanceof Element ? rectArray(element.getBoundingClientRect()) : null;
  const panel = document.querySelector('.editDomainUpperPanel');
  const surface = panel?.querySelector('.sceneMatrixPanel');
  const header = surface?.querySelector('.sceneMatrixSurfaceHeader');
  const cards = [...(surface?.querySelectorAll('.sceneMatrixCard') ?? [])].filter(visible);
  const cardScroll = surface?.querySelector('.sceneMatrixCards');
  const headerControls = [...(header?.querySelectorAll('button') ?? [])].filter(visible);
  const viewport = { x: 0, y: 0, right: innerWidth, bottom: innerHeight };
  const contained = (value) => Boolean(value && value[0] >= -1 && value[1] >= -1 && value[4] <= viewport.right + 1 && value[5] <= viewport.bottom + 1);
  const documentElement = document.documentElement;
  const app = document.querySelector('.app');
  return {
    panel: rect(panel),
    surface: rect(surface),
    header: rect(header),
    cardCount: cards.length,
    cardRect: rect(cards[0]),
    bankCount: surface?.querySelectorAll('[data-scene-matrix-column]').length ?? 0,
    cardScrollHeight: cardScroll?.scrollHeight ?? 0,
    cardClientHeight: cardScroll?.clientHeight ?? 0,
    headerControlHeights: headerControls.map((control) => control.getBoundingClientRect().height),
    contained: contained(rect(surface)),
    outerScroll: {
      document: [documentElement.scrollWidth - documentElement.clientWidth, documentElement.scrollHeight - documentElement.clientHeight],
      app: app ? [app.scrollWidth - app.clientWidth, app.scrollHeight - app.clientHeight] : null,
    },
  };
})()`);

const measureTopbarIdentity = (client) => evaluate(client, `(() => {
  const topbar = document.querySelector('.topbar');
  const project = document.querySelector('.topbarProject');
  const brand = project?.querySelector(':scope > strong');
  const label = project?.querySelector(':scope > span');
  const workspaceButton = project?.querySelector('.workspaceOperationsButton');
  const status = document.querySelector('.status');
  const windowControls = document.querySelector('[data-window-controls]');
  const state = (element) => element instanceof HTMLElement ? {
    text: element.textContent?.trim() ?? '',
    clientWidth: element.clientWidth,
    scrollWidth: element.scrollWidth,
    display: getComputedStyle(element).display,
    rect: [element.getBoundingClientRect().x, element.getBoundingClientRect().y, element.getBoundingClientRect().width, element.getBoundingClientRect().height],
  } : null;
  return {
    topbar: state(topbar),
    project: state(project),
    brand: state(brand),
    label: state(label),
    workspaceButton: state(workspaceButton),
    status: state(status),
    windowControls: state(windowControls),
  };
})()`);

const measureVideo = (client) => evaluate(client, `(() => {
  const visible = (element) => {
    if (!(element instanceof HTMLElement)) return false;
    const rect = element.getBoundingClientRect();
    const style = getComputedStyle(element);
    return rect.width > 0 && rect.height > 0 && style.display !== 'none' && style.visibility !== 'hidden';
  };
  const rectArray = (value) => [value.x, value.y, value.width, value.height, value.right, value.bottom];
  const rect = (element) => element instanceof Element ? rectArray(element.getBoundingClientRect()) : null;
  const panel = document.querySelector('.videoControlPanelLibrary');
  const header = panel?.querySelector(':scope > .panelHeader');
  const rail = panel?.querySelector('.videoMediaLibraryRail');
  const surface = panel?.querySelector('.videoMediaLibrarySurface');
  const list = panel?.querySelector('.videoMediaLibraryList');
  const cards = [...(panel?.querySelectorAll('.videoMediaLibraryItem') ?? [])].filter(visible);
  const disclosure = panel?.querySelector('[data-edit-video-import-disclosure]');
  const importSurface = panel?.querySelector('[data-edit-video-import-surface]');
  const summary = disclosure?.querySelector(':scope > summary');
  const inViewport = (value) => Boolean(value && value[0] >= -1 && value[1] >= -1 && value[4] <= innerWidth + 1 && value[5] <= innerHeight + 1);
  const documentElement = document.documentElement;
  const app = document.querySelector('.app');
  const listStyle = list instanceof HTMLElement ? getComputedStyle(list) : null;
  const importStyle = importSurface instanceof HTMLElement ? getComputedStyle(importSurface) : null;
  const popupReachability = (popup) => {
    const target = [...(popup?.querySelectorAll('button, input, select, textarea, [role="button"]') ?? [])].find(visible);
    if (!(target instanceof HTMLElement)) return { target: false, hit: false, point: null };
    const targetRect = target.getBoundingClientRect();
    const point = { x: targetRect.left + Math.min(targetRect.width / 2, Math.max(1, targetRect.width - 1)), y: targetRect.top + Math.min(targetRect.height / 2, Math.max(1, targetRect.height - 1)) };
    const hit = document.elementFromPoint(point.x, point.y);
    return { target: true, hit: hit === target || target.contains(hit), point };
  };
  const importReachability = popupReachability(importSurface);
  return {
    panel: rect(panel),
    header: rect(header),
    rail: rect(rail),
    surface: rect(surface),
    list: rect(list),
    cardCount: cards.length,
    cardRect: rect(cards[0]),
    listOverflowY: listStyle?.overflowY ?? '',
    listScrollable: (list?.scrollHeight ?? 0) > (list?.clientHeight ?? 0),
    importOpen: disclosure instanceof HTMLDetailsElement && disclosure.open,
    importSummary: rect(summary),
    importSurface: rect(importSurface),
    importSurfaceControlCount: importSurface?.querySelectorAll('input, select, textarea, button').length ?? 0,
    importSurfaceOverflowY: importStyle?.overflowY ?? '',
    importSurfaceScrollable: (importSurface?.scrollHeight ?? 0) > (importSurface?.clientHeight ?? 0),
    importSurfaceInViewport: inViewport(rect(importSurface)),
    importSurfaceElementFromPointReachable: importReachability.hit,
    importSurfaceHitPoint: importReachability.point,
    outerScroll: {
      document: [documentElement.scrollWidth - documentElement.clientWidth, documentElement.scrollHeight - documentElement.clientHeight],
      app: app ? [app.scrollWidth - app.clientWidth, app.scrollHeight - app.clientHeight] : null,
    },
  };
})()`);

const measureTimeline = (client) => evaluate(client, `(() => {
  const visible = (element) => {
    if (!(element instanceof HTMLElement)) return false;
    const rect = element.getBoundingClientRect();
    const style = getComputedStyle(element);
    return rect.width > 0 && rect.height > 0 && style.display !== 'none' && style.visibility !== 'hidden';
  };
  const rectArray = (value) => [value.x, value.y, value.width, value.height, value.right, value.bottom];
  const rect = (element) => element instanceof Element ? rectArray(element.getBoundingClientRect()) : null;
  const viewport = { x: 0, y: 0, right: innerWidth, bottom: innerHeight };
  const contained = (value) => Boolean(value && value[0] >= -1 && value[1] >= -1 && value[4] <= viewport.right + 1 && value[5] <= viewport.bottom + 1);
  const livePanel = document.querySelector('.liveControlPanel');
  const arrangerHeader = document.querySelector('[data-timeline-arranger-header]');
  const upper = document.querySelector('[data-timeline-arranger-upper]');
  const timelineSurface = [...document.querySelectorAll('.timelineShowSurface')].find(visible);
  const lanes = [...document.querySelectorAll('[data-timeline-layer-gutter]')].filter(visible);
  const shelf = [...document.querySelectorAll('[data-timeline-source-shelf]')].find(visible);
  const shelfHeader = shelf?.querySelector('.timelineExternalSourceShelfHeader');
  const shelfBody = shelf?.querySelector('[role=tabpanel]');
  const context = shelf?.closest('[data-workspace-pane="lower-right"]');
  const sourceCards = [...(shelf?.querySelectorAll('.timelineExternalSourceCard') ?? [])].filter(visible);
  const tools = document.querySelector('.timelineToolsDisclosure');
  const toolsPanel = document.querySelector('.timelineToolsDisclosure[open] .timelineToolsDisclosurePanel');
  const mixer = document.querySelector('.groupLiveMixerDisclosure');
  const mixerPanel = document.querySelector('.groupLiveMixerDisclosure[open] .groupLiveMixerDisclosurePanel');
  const panelStyle = toolsPanel instanceof HTMLElement ? getComputedStyle(toolsPanel) : null;
  const mixerStyle = mixerPanel instanceof HTMLElement ? getComputedStyle(mixerPanel) : null;
  const shelfStyle = shelf instanceof HTMLElement ? getComputedStyle(shelf) : null;
  const bodyStyle = shelfBody instanceof HTMLElement ? getComputedStyle(shelfBody) : null;
  const documentElement = document.documentElement;
  const app = document.querySelector('.app');
  const band = document.querySelector('.mappingPersistentWorkspaceBand');
  const visibleFocusable = (root) => [...(root?.querySelectorAll('button, input, select, textarea, summary, [tabindex]') ?? [])].filter(visible).length;
  const popupReachability = (popup) => {
    const target = [...(popup?.querySelectorAll('button, input, select, textarea, [role="button"]') ?? [])].find(visible);
    if (!(target instanceof HTMLElement)) return { target: false, hit: false, point: null };
    const targetRect = target.getBoundingClientRect();
    const point = { x: targetRect.left + Math.min(targetRect.width / 2, Math.max(1, targetRect.width - 1)), y: targetRect.top + Math.min(targetRect.height / 2, Math.max(1, targetRect.height - 1)) };
    const hit = document.elementFromPoint(point.x, point.y);
    return { target: true, hit: hit === target || target.contains(hit), point };
  };
  const toolsReachability = popupReachability(toolsPanel);
  const mixerReachability = popupReachability(mixerPanel);
  return {
    livePanel: rect(livePanel),
    arrangerHeader: rect(arrangerHeader),
    upper: rect(upper),
    timelineSurface: rect(timelineSurface),
    laneCount: lanes.length,
    laneRect: rect(lanes[0]),
    shelf: rect(shelf),
    context: rect(context),
    shelfHeader: rect(shelfHeader),
    shelfHeading: rect(shelf?.querySelector('h3')),
    shelfBody: rect(shelfBody),
    sourceCardCount: sourceCards.length,
    shelfOverflowY: shelfStyle?.overflowY ?? '',
    shelfOuterScroll: (shelf?.scrollHeight ?? 0) - (shelf?.clientHeight ?? 0),
    shelfBodyOverflowY: bodyStyle?.overflowY ?? '',
    shelfBodyScroll: (shelfBody?.scrollHeight ?? 0) - (shelfBody?.clientHeight ?? 0),
    toolsOpen: tools instanceof HTMLDetailsElement && tools.open,
    toolsPanel: rect(toolsPanel),
    toolsPanelInViewport: contained(rect(toolsPanel)),
    toolsPanelElementFromPointReachable: toolsReachability.hit,
    toolsPanelHitPoint: toolsReachability.point,
    toolsPanelOverflowY: panelStyle?.overflowY ?? '',
    toolsInteractiveCount: [...(toolsPanel?.querySelectorAll('button, input, select, output') ?? [])].filter(visible).length,
    toolsSummary: rect(tools?.querySelector(':scope > summary')),
    mixerOpen: mixer instanceof HTMLDetailsElement && mixer.open,
    mixerPanel: rect(mixerPanel),
    mixerPanelInViewport: contained(rect(mixerPanel)),
    mixerPanelElementFromPointReachable: mixerReachability.hit,
    mixerPanelHitPoint: mixerReachability.point,
    mixerPanelOverflowY: mixerStyle?.overflowY ?? '',
    mixerInteractiveCount: [...(mixerPanel?.querySelectorAll('button, input, select, output') ?? [])].filter(visible).length,
    mixerSummary: rect(mixer?.querySelector(':scope > summary')),
    expanded: band?.getAttribute('data-timeline-pane-expanded') === 'true',
    bandInert: band instanceof HTMLElement && (band.hasAttribute('inert') || band.inert === true),
    bandAriaHidden: band?.getAttribute('aria-hidden') === 'true',
    band: rect(band),
    lowerVisibleFocusable: visibleFocusable(band),
    outerScroll: {
      document: [documentElement.scrollWidth - documentElement.clientWidth, documentElement.scrollHeight - documentElement.clientHeight],
      app: app ? [app.scrollWidth - app.clientWidth, app.scrollHeight - app.clientHeight] : null,
    },
  };
})()`);

const assertOuterScrollFixed = (state, label) => {
  assert.deepEqual(state.outerScroll.document, [0, 0], `${label} document outer scroll remains fixed`);
  assert.ok(!state.outerScroll.app || state.outerScroll.app.every((value) => value <= 1), `${label} app outer scroll remains fixed`);
};

const rectHeight = (value) => value?.[3] ?? 0;
const rectWidth = (value) => value?.[2] ?? 0;
const assertContained = (value, label, width = 0, height = 0) => {
  assert.ok(value && value[2] >= width && value[3] >= height, `${label} has useful geometry`);
  assert.ok(value[0] >= -1 && value[1] >= -1 && value[4] <= width + 1 && value[5] <= height + 1, `${label} stays inside viewport`);
};

const assertMainWindowMinimum = async () => {
  const config = JSON.parse(await readFile(resolve(appRoot, "src-tauri", "tauri.conf.json"), "utf8"));
  const mainWindow = config?.app?.windows?.find((candidate) => candidate?.title === "Syndocal") ?? config?.app?.windows?.[0];
  assert.deepEqual(
    { width: mainWindow?.minWidth, height: mainWindow?.minHeight },
    productMinimumWindow,
    "Tauri main window keeps its 960x640 restore minimum",
  );
};

let vite;
let browser;
let client;
let profileDir;
try {
  const browserPath = browserCandidates.find((candidate) => existsSync(candidate));
  assert.ok(browserPath, "Chrome or Edge is required for the Control upper workspace browser gate");
  await assertMainWindowMinimum();
  await mkdir(screenshotDir, { recursive: true });
  vite = spawn(process.execPath, [resolve(appRoot, "node_modules/vite/bin/vite.js"), "--host", host, "--port", String(vitePort), "--strictPort"], {
    cwd: appRoot,
    stdio: "ignore",
  });
  await waitFor(async () => (await fetch(baseUrl)).ok, "Vite fixture server");
  profileDir = await mkdtemp(join(tmpdir(), "syndocal-control-upper-workspaces-"));
  browser = spawn(browserPath, [
    "--headless=new",
    "--disable-gpu",
    "--no-first-run",
    "--no-default-browser-check",
    `--remote-debugging-port=${cdpPort}`,
    `--user-data-dir=${profileDir}`,
    "about:blank",
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
    await sleep(80);
    const topbarIdentity = await measureTopbarIdentity(client);
    assert.ok(topbarIdentity.brand && topbarIdentity.brand.clientWidth >= topbarIdentity.brand.scrollWidth, `Syndocal brand is not clipped at ${viewport.width}x${viewport.height}: ${JSON.stringify(topbarIdentity)}`);
    assert.ok(topbarIdentity.workspaceButton && topbarIdentity.workspaceButton.clientWidth >= topbarIdentity.workspaceButton.scrollWidth, `Workspaces control label is not clipped at ${viewport.width}x${viewport.height}: ${JSON.stringify(topbarIdentity)}`);
    const projectBounds = topbarIdentity.project?.rect;
    const brandBounds = topbarIdentity.brand?.rect;
    const workspaceBounds = topbarIdentity.workspaceButton?.rect;
    const topbarBounds = topbarIdentity.topbar?.rect;
    const statusBounds = topbarIdentity.status?.rect;
    const windowControlBounds = topbarIdentity.windowControls?.rect;
    assert.ok(projectBounds && brandBounds && workspaceBounds && brandBounds[0] >= projectBounds[0] - 1 && brandBounds[0] + brandBounds[2] <= projectBounds[0] + projectBounds[2] + 1 && workspaceBounds[0] >= projectBounds[0] - 1 && workspaceBounds[0] + workspaceBounds[2] <= projectBounds[0] + projectBounds[2] + 1 && brandBounds[0] + brandBounds[2] <= workspaceBounds[0] + 1, `Topbar identity/Workspaces controls remain contained and non-overlapping at ${viewport.width}x${viewport.height}: ${JSON.stringify(topbarIdentity)}`);
    assert.ok(statusBounds && statusBounds[2] > 0 && statusBounds[3] > 0 && topbarIdentity.status.display !== "none", `Topbar transport/status controls remain visibly rendered at ${viewport.width}x${viewport.height}: ${JSON.stringify(topbarIdentity.status)}`);
    assert.ok(windowControlBounds && windowControlBounds[2] > 0 && windowControlBounds[3] > 0 && topbarIdentity.windowControls.display !== "none", `Window controls remain visibly rendered at ${viewport.width}x${viewport.height}: ${JSON.stringify(topbarIdentity.windowControls)}`);
    assert.ok(topbarBounds && statusBounds && windowControlBounds && statusBounds[0] >= topbarBounds[0] - 1 && windowControlBounds[0] >= statusBounds[0] - 1 && windowControlBounds[0] + windowControlBounds[2] <= topbarBounds[0] + topbarBounds[2] + 1, `Topbar transport/status/window controls remain contained at ${viewport.width}x${viewport.height}: ${JSON.stringify(topbarIdentity)}`);
    assert.ok(topbarIdentity.status.clientWidth >= topbarIdentity.status.scrollWidth, `Topbar transport/status controls require no hidden horizontal scroll at ${viewport.width}x${viewport.height}: ${JSON.stringify(topbarIdentity.status)}`);
    assert.ok(topbarIdentity.windowControls.clientWidth >= topbarIdentity.windowControls.scrollWidth, `Window controls are not clipped at ${viewport.width}x${viewport.height}: ${JSON.stringify(topbarIdentity.windowControls)}`);

    // Lighting: the authored upper bank/scene surface must own real height and
    // mount visible scene cards; a hidden legacy surface is not sufficient.
    assert.equal(await clickVisible(client, '[data-workspace-option="control"]'), true, "select Edit workspace");
    await waitFor(() => evaluate(client, 'document.querySelector(".layout.layoutSharedWorkspace.layoutControl") !== null'), "Edit workspace");
    assert.equal(await clickVisible(client, '[data-control-mode-option="edit"]'), true, "select Lighting domain");
    await waitFor(() => evaluate(client, 'document.querySelector(".layout.layoutSharedWorkspace.layoutControl.controlModeEdit") !== null'), "Lighting domain");
    const lighting = await waitFor(async () => {
      const value = await measureLighting(client);
      return value?.cardCount > 0 ? value : false;
    }, "Lighting upper bank/scene surface");
    assert.ok(rectHeight(lighting.panel) >= 160, `Lighting upper panel remains useful at ${viewport.width}x${viewport.height}`);
    assert.ok(rectHeight(lighting.surface) >= 140 && rectWidth(lighting.surface) > 0, "Lighting scene surface has useful geometry");
    assert.ok(lighting.bankCount > 0 && lighting.cardCount > 0 && lighting.cardRect?.[2] > 0, "Lighting renders bank columns and visible scene cards");
    assert.ok(lighting.contained, "Lighting scene surface is contained in the viewport");
    assertOuterScrollFixed(lighting, "Lighting");
    await capture(client, `control-lighting-${viewport.width}x${viewport.height}.png`);
    console.log(`${viewport.width}x${viewport.height} Lighting: upper=${Math.round(rectHeight(lighting.panel))}px banks=${lighting.bankCount} cards=${lighting.cardCount}`);

    // Video: the Media Library body is in-flow with its own list scrollport;
    // Import Media is an accessible 44px disclosure whose full surface remains
    // reachable without causing document/app scrolling.
    assert.equal(await clickVisible(client, '[data-control-mode-option="mixer"]'), true, "select Video domain");
    await waitFor(() => evaluate(client, 'document.querySelector(".layout.layoutSharedWorkspace.layoutControl.controlModeMixer") !== null'), "Video domain");
    const video = await waitFor(async () => {
      const value = await measureVideo(client);
      return value?.cardCount > 0 ? value : false;
    }, "Video Media Library surface");
    assert.ok(rectHeight(video.panel) >= 160 && rectHeight(video.header) >= 40, "Video library panel/header have useful geometry");
    assert.ok(rectHeight(video.surface) >= 120 && rectHeight(video.list) >= 80, "Video library body/list have useful geometry");
    assert.ok(video.cardCount > 0 && (video.listOverflowY === "auto" || video.listOverflowY === "scroll"), "Video mounts media cards with an internal list scrollport");
    assert.ok(video.importSummary?.[3] >= 43.5, "Import Media preserves its 44px disclosure target");
    assertOuterScrollFixed(video, "Video");
    await capture(client, `control-video-${viewport.width}x${viewport.height}.png`);
    assert.equal(await clickVisible(client, '[data-edit-video-import-disclosure] > summary'), true, "open Import Media");
    const videoImport = await waitFor(async () => {
      const value = await measureVideo(client);
      return value?.importOpen ? value : false;
    }, "Import Media disclosure");
    assert.ok(videoImport.importSurfaceControlCount > 0, "Import Media mounts its complete form");
    assert.ok(videoImport.importSurface?.[2] > 0 && videoImport.importSurface?.[3] > 0, "Import Media form has useful geometry");
    assert.ok(videoImport.importSurfaceInViewport, `Import Media remains vertically reachable at ${viewport.width}x${viewport.height}: ${JSON.stringify({ panel: videoImport.panel, header: videoImport.header, surface: videoImport.importSurface, viewport: [viewport.width, viewport.height] })}`);
    assert.equal(videoImport.importSurfaceElementFromPointReachable, true, `Import Media form is reachable by elementFromPoint: ${JSON.stringify(videoImport.importSurfaceHitPoint)}`);
    assert.ok(videoImport.importSurfaceOverflowY === "auto" || videoImport.importSurfaceOverflowY === "scroll", "Import Media owns overflow internally");
    const videoImportLast = await exercisePopupLastTarget(client, '[data-edit-video-import-surface]');
    assert.ok(videoImportLast?.popupInsideLayout && videoImportLast.popupInsideViewport, `Import Media popup remains inside its layout and viewport: ${JSON.stringify(videoImportLast)}`);
    assert.ok(videoImportLast?.targetInsidePopup && videoImportLast.hit, `Import Media last action remains scroll-reachable and hit-testable: ${JSON.stringify(videoImportLast)}`);
    assert.ok((videoImportLast?.targetHeight ?? 0) >= 24, `Import Media last action keeps a usable control height: ${JSON.stringify(videoImportLast)}`);
    assertOuterScrollFixed(videoImport, "Video Import Media");
    await capture(client, `control-video-import-open-${viewport.width}x${viewport.height}.png`);

    // Timeline: inspect upper arranger, the two header disclosures, and the
    // lower Sources shelf before exercising expansion/Escape priority.
    assert.equal(await clickVisible(client, '[data-control-mode-option="live"]'), true, "select Timeline domain");
    await waitFor(() => evaluate(client, 'document.querySelector(".layout.layoutSharedWorkspace.layoutControl.controlModeLive") !== null'), "Timeline domain");
    const timeline = await waitFor(async () => {
      const value = await measureTimeline(client);
      return value?.laneCount > 0 && value?.sourceCardCount > 0 ? value : false;
    }, "Timeline upper arranger and Sources shelf");
    assert.ok(rectHeight(timeline.livePanel) >= 200, "Timeline upper arranger has useful height");
    assert.ok(rectHeight(timeline.upper) >= 140 && rectHeight(timeline.timelineSurface) > 0, "Timeline lane surface has useful geometry");
    assert.ok(timeline.laneCount > 0 && timeline.laneRect?.[3] > 0, "Timeline mounts visible lanes");
    assert.ok(timeline.shelf && rectHeight(timeline.shelf) > 0 && rectHeight(timeline.context) > 0, "Timeline Sources shelf is rendered in the lower context pane");
    assert.ok(Math.abs((timeline.shelf?.[3] ?? 0) - (timeline.context?.[3] ?? 0)) <= 2, "Sources shelf fills the lower context pane");
    assert.ok(rectHeight(timeline.shelfHeader) >= 32 && timeline.shelfHeading?.[3] > 0, "Sources header is readable and not clipped");
    assert.ok(timeline.sourceCardCount > 0 && (timeline.shelfBodyOverflowY === "auto" || timeline.shelfBodyOverflowY === "scroll"), "Sources body owns internal scrolling and mounts source cards");
    assert.equal(timeline.shelfOverflowY, "hidden", "Sources shelf has no outer scrollport");
    assert.equal(timeline.shelfOuterScroll, 0, "Sources shelf outer scroll is zero");
    assertOuterScrollFixed(timeline, "Timeline");

    assert.equal(await clickVisible(client, '.timelineToolsDisclosure > summary'), true, "open Timeline Tools/Performance");
    const toolsOpen = await waitFor(async () => {
      const value = await measureTimeline(client);
      return value?.toolsOpen ? value : false;
    }, "Timeline Tools/Performance disclosure");
    assert.ok(toolsOpen.toolsPanelInViewport, `Timeline Tools/Performance remains contained at ${viewport.width}x${viewport.height}`);
    assert.equal(toolsOpen.toolsPanelElementFromPointReachable, true, `Timeline Tools/Performance is reachable by elementFromPoint: ${JSON.stringify(toolsOpen.toolsPanelHitPoint)}`);
    assert.ok(toolsOpen.toolsInteractiveCount > 0, "Timeline Tools/Performance remains interactive");
    assert.ok(toolsOpen.toolsSummary?.[3] >= 43.5, "Timeline Tools preserves its 44px summary target");
    const toolsLast = await exercisePopupLastTarget(client, '.timelineToolsDisclosure[open] .timelineToolsDisclosurePanel');
    assert.ok(toolsLast?.popupInsideLayout && toolsLast.popupInsideViewport, `Timeline Tools popup remains inside its layout and viewport: ${JSON.stringify(toolsLast)}`);
    assert.ok(toolsLast?.targetInsidePopup && toolsLast.hit, `Timeline Tools last action remains scroll-reachable and hit-testable: ${JSON.stringify(toolsLast)}`);
    assert.ok((toolsLast?.targetHeight ?? 0) >= 24, `Timeline Tools last action keeps a usable control height: ${JSON.stringify(toolsLast)}`);
    await capture(client, `control-timeline-tools-open-${viewport.width}x${viewport.height}.png`);
    assert.equal(await clickVisible(client, '.groupLiveMixerDisclosure > summary'), true, "open Live Mixer");
    const mixerOpen = await waitFor(async () => {
      const value = await measureTimeline(client);
      return value?.mixerOpen ? value : false;
    }, "Live Mixer disclosure");
    assert.equal(mixerOpen.toolsOpen, false, "Live Mixer and Tools disclosures are mutually exclusive in the gate sequence");
    assert.ok(mixerOpen.mixerPanelInViewport, `Live Mixer remains contained at ${viewport.width}x${viewport.height}`);
    assert.equal(mixerOpen.mixerPanelElementFromPointReachable, true, `Live Mixer is reachable by elementFromPoint: ${JSON.stringify(mixerOpen.mixerPanelHitPoint)}`);
    assert.ok(mixerOpen.mixerInteractiveCount > 0, "Live Mixer remains interactive");
    assert.ok(mixerOpen.mixerSummary?.[3] >= 43.5, "Live Mixer preserves its 44px summary target");
    const mixerLast = await exercisePopupLastTarget(client, '.groupLiveMixerDisclosure[open] .groupLiveMixerDisclosurePanel');
    assert.ok(mixerLast?.popupInsideLayout && mixerLast.popupInsideViewport, `Live Mixer popup remains inside its layout and viewport: ${JSON.stringify(mixerLast)}`);
    assert.ok(mixerLast?.targetInsidePopup && mixerLast.hit, `Live Mixer last action remains scroll-reachable and hit-testable: ${JSON.stringify(mixerLast)}`);
    assert.ok((mixerLast?.targetHeight ?? 0) >= 24, `Live Mixer last action keeps a usable control height: ${JSON.stringify(mixerLast)}`);
    await capture(client, `control-timeline-live-mixer-open-${viewport.width}x${viewport.height}.png`);
    await evaluate(client, "document.querySelector('.groupLiveMixerDisclosure')?.removeAttribute('open')");
    await sleep(40);

    // Expansion is a keyboard-priority state machine: the first Escape closes
    // an open disclosure, and only the next Escape restores the split.
    assert.equal(await clickVisible(client, '[data-timeline-pane-expand-toggle]'), true, "expand Timeline pane");
    await waitFor(() => evaluate(client, 'document.querySelector(".mappingPersistentWorkspaceBand")?.getAttribute("data-timeline-pane-expanded") === "true"'), "expanded Timeline pane");
    const expanded = await measureTimeline(client);
    assert.equal(expanded.expanded, true, "Timeline expansion state is applied");
    assert.equal(expanded.bandInert, true, "Expanded Timeline marks the lower band inert");
    assert.equal(expanded.bandAriaHidden, true, "Expanded Timeline hides the lower band from assistive technology");
    assert.ok(rectHeight(expanded.band) <= 2.5, `Expanded Timeline collapses the lower band: ${JSON.stringify({ band: expanded.band })}`);
    const expandedTabOrder = await exerciseExpandedTabOrder(client);
    assert.equal(expandedTabOrder.lowerBandHit, false, `Expanded Timeline Tab traversal never enters the lower band: ${JSON.stringify(expandedTabOrder.visited)}`);
    assert.equal(expandedTabOrder.upperPortalHit, true, "Expanded Timeline Tab traversal reaches an upper Portal control");

    // Menus are above Timeline disclosures/drawers in the Escape stack. One
    // Escape must close exactly the menu and leave the pane expanded.
    assert.equal(await clickVisible(client, '[data-project-drop-surface]'), true, "open Project menu while Timeline is expanded");
    await waitFor(() => evaluate(client, 'document.querySelector(".appProjectMenu") !== null'), "expanded Project menu");
    await pressEscape(client);
    const projectMenuEscape = await evaluate(client, '({ menuOpen: document.querySelector(".appProjectMenu") !== null, expanded: document.querySelector(".mappingPersistentWorkspaceBand")?.getAttribute("data-timeline-pane-expanded") ?? null })');
    assert.deepEqual(projectMenuEscape, { menuOpen: false, expanded: "true" }, "Escape closes only the Project menu before Timeline collapse");
    assert.equal(await evaluate(client, `(() => {
      const group = document.querySelector('[data-group-tab]');
      if (!(group instanceof HTMLElement)) return false;
      group.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true, clientX: 24, clientY: 24 }));
      return document.querySelector('[data-group-context-menu]') !== null;
    })()`), true, "open group context menu while Timeline is expanded");
    await pressEscape(client);
    const groupMenuEscape = await evaluate(client, '({ menuOpen: document.querySelector("[data-group-context-menu]") !== null, expanded: document.querySelector(".mappingPersistentWorkspaceBand")?.getAttribute("data-timeline-pane-expanded") ?? null })');
    assert.deepEqual(groupMenuEscape, { menuOpen: false, expanded: "true" }, "Escape closes only the group context menu before Timeline collapse");

    assert.equal(await clickVisible(client, '.timelineToolsDisclosure > summary'), true, "open Tools before Escape priority check");
    await waitFor(() => evaluate(client, 'document.querySelector(".timelineToolsDisclosure")?.open === true'), "expanded Tools disclosure");
    await evaluate(client, `(() => {
      const panel = document.querySelector('.timelineToolsDisclosure[open] .timelineToolsDisclosurePanel');
      if (panel instanceof HTMLElement) panel.scrollTop = 0;
    })()`);
    await sleep(40);
    const expandedTools = await measureTimeline(client);
    assert.equal(expandedTools.toolsPanelInViewport, true, "Expanded Timeline Tools popup remains contained");
    assert.equal(expandedTools.toolsPanelElementFromPointReachable, true, "Expanded Timeline Tools popup remains elementFromPoint reachable");
    await capture(client, `control-timeline-tools-expanded-open-${viewport.width}x${viewport.height}.png`);
    assert.equal(await evaluate(client, `(() => {
      const panel = document.querySelector('.timelineToolsDisclosure[open] .timelineToolsDisclosurePanel');
      const target = panel?.querySelector('button, input, select, textarea, [role="button"]');
      if (!(target instanceof HTMLElement)) return false;
      target.focus();
      return panel.contains(document.activeElement);
    })()`), true, "Expanded Timeline Tools accepts focus inside the open popup");
    await pressEscape(client);
    const firstEscape = await evaluate(client, '(() => { const details = document.querySelector(".timelineToolsDisclosure"); const summary = details?.querySelector(":scope > summary"); return { toolsOpen: details?.open ?? null, expanded: document.querySelector(".mappingPersistentWorkspaceBand")?.getAttribute("data-timeline-pane-expanded") ?? null, focusReturned: document.activeElement === summary }; })()');
    console.log(String(viewport.width) + "x" + String(viewport.height) + " first Escape: " + JSON.stringify(firstEscape));
    assert.deepEqual(firstEscape, { toolsOpen: false, expanded: "true", focusReturned: true }, "first Escape closes Tools, restores summary focus, and precedes collapse");
    await pressEscape(client);
    const secondEscape = await evaluate(client, 'document.querySelector(".mappingPersistentWorkspaceBand")?.getAttribute("data-timeline-pane-expanded") ?? null');
    assert.equal(secondEscape, "false", "second Escape collapses Timeline");
    const collapsed = await measureTimeline(client);
    assert.equal(collapsed.expanded, false, "Second Escape restores normal Timeline split");
    assert.equal(collapsed.bandInert, false, "Collapsed Timeline removes inert from the lower band");
    assert.equal(collapsed.bandAriaHidden, false, "Collapsed Timeline restores the lower band to assistive technology");
    assert.ok(rectHeight(collapsed.shelf) > 0 && collapsed.sourceCardCount > 0, "Collapsed Timeline returns the Sources shelf");
    assert.ok(collapsed.lowerVisibleFocusable > 0, "Collapsed Timeline restores visible lower-band controls");
    assert.equal(await evaluate(client, `(() => {
      const band = document.querySelector('.mappingPersistentWorkspaceBand');
      if (!(band instanceof HTMLElement) || band.hasAttribute('inert')) return false;
      const visible = (element) => {
        if (!(element instanceof HTMLElement)) return false;
        const rect = element.getBoundingClientRect();
        const style = getComputedStyle(element);
        return rect.width > 0 && rect.height > 0 && style.display !== 'none' && style.visibility !== 'hidden' && !element.hasAttribute('disabled');
      };
      const target = [...band.querySelectorAll('button, input, select, textarea, summary, [tabindex]')].find(visible);
      if (!(target instanceof HTMLElement)) return false;
      target.focus();
      return document.activeElement === target;
    })()`), true, "Collapsed Timeline returns a lower-band control to the focus sequence");
    assertOuterScrollFixed(collapsed, "Timeline after Escape");
    await capture(client, `control-timeline-${viewport.width}x${viewport.height}.png`);
    console.log(`${viewport.width}x${viewport.height} Timeline: upper=${Math.round(rectHeight(collapsed.upper))}px lanes=${collapsed.laneCount} sources=${collapsed.sourceCardCount} lower=${Math.round(rectHeight(collapsed.shelf))}px`);
  }
  console.log(`Control upper workspace browser gate passed; screenshots=${screenshotDir}`);
} finally {
  client?.close();
  await stopChild(browser);
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
