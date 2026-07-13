import { spawn } from "node:child_process";
import { createHash } from "node:crypto";
import { existsSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { inflateSync } from "node:zlib";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const appRoot = resolve(scriptDir, "..");
const largeShowMode = process.argv.includes("--large-show");
const vjEmptyMode = process.argv.includes("--vj-empty");
const sceneBlockOnlyMode = process.argv.includes("--scene-block-only");
const sceneBlockHourOnlyMode = process.argv.includes("--scene-block-hour-only");
const sceneBlockOverlapOnlyMode = process.argv.includes("--scene-block-overlap-only");
const viewportFixture = process.env.SYNDOCAL_VIEWPORT_FIXTURE ?? (largeShowMode ? "large-show" : vjEmptyMode ? "vj-empty" : "timeline");
const defaultUrl =
  viewportFixture === "none"
    ? "http://127.0.0.1:5173/"
    : `http://127.0.0.1:5173/?syndocalViewportFixture=${encodeURIComponent(viewportFixture)}`;
const appUrl = process.env.SYNDOCAL_VIEWPORT_URL ?? defaultUrl;
const fixtureUrl = (fixture) => {
  const url = new URL(appUrl);
  url.searchParams.set("syndocalViewportFixture", fixture);
  return url.toString();
};
const shouldStartVite = appUrl === defaultUrl && process.env.SYNDOCAL_VIEWPORT_NO_SERVER !== "1";
const shouldCheckTimelineAutomation = new URL(appUrl).searchParams.get("syndocalViewportFixture") === "timeline";
const vitePort = 5173;
const cdpPort = Number(process.env.SYNDOCAL_CDP_PORT ?? 9227);
const screenshotDir = process.env.SYNDOCAL_VIEWPORT_SCREENSHOT_DIR
  ? resolve(process.env.SYNDOCAL_VIEWPORT_SCREENSHOT_DIR)
  : null;
const primaryOperationalViewport = { width: 1920, height: 1080 };
const measuredClientSizeViewport = { width: 1920, height: 1032 };
const extendedCeilingViewport = { width: 2048, height: 1152 };
const compactFallbackViewports = [
  { width: 1366, height: 768 },
  { width: 1280, height: 720 },
];
const allViewports = [
  primaryOperationalViewport,
  measuredClientSizeViewport,
  extendedCeilingViewport,
  ...compactFallbackViewports,
];
const requestedViewportMatch = process.env.SYNDOCAL_VIEWPORT_ONLY?.match(/^(\d+)x(\d+)$/);
const requestedViewport = requestedViewportMatch
  ? { width: Number(requestedViewportMatch[1]), height: Number(requestedViewportMatch[2]) }
  : null;
const viewports = requestedViewport
  ? [requestedViewport]
  : largeShowMode
  ? [primaryOperationalViewport]
  : process.env.SYNDOCAL_VIEWPORT_SINGLE === "1"
    ? [primaryOperationalViewport]
    : allViewports;
const fullWindowTimelineViewports = requestedViewport
  ? [requestedViewport]
  : [primaryOperationalViewport, measuredClientSizeViewport];
const captureAllViewportScreenshots = process.env.SYNDOCAL_VIEWPORT_CAPTURE_ALL === "1";
const matchesViewport = (candidate, reference) =>
  candidate.width === reference.width && candidate.height === reference.height;
const isPrimaryOperationalViewport = (viewport) => matchesViewport(viewport, primaryOperationalViewport);
const isMeasuredClientSizeViewport = (viewport) => matchesViewport(viewport, measuredClientSizeViewport);
const shouldCaptureViewport = (viewport) =>
  Boolean(screenshotDir) && (
    isPrimaryOperationalViewport(viewport) ||
    isMeasuredClientSizeViewport(viewport) ||
    captureAllViewportScreenshots
  );
const viewportRole = ({ width, height }) => {
  if (matchesViewport({ width, height }, primaryOperationalViewport)) return "primary-maximized";
  if (matchesViewport({ width, height }, measuredClientSizeViewport)) return "measured-client-size-browser";
  if (matchesViewport({ width, height }, extendedCeilingViewport)) return "extended-ceiling";
  return "compact-fallback";
};
const setupTabs = [
  { area: "Lighting", tab: "Library", id: "library" },
  { area: "Lighting", tab: "Profiles", id: "profiles" },
  { area: "Lighting", tab: "Patch", id: "patch" },
  { area: "Video", tab: "Outputs", id: "video" },
  { area: "Mapping", tab: "Stage Map", id: "mapping" },
  { area: "I/O", tab: "DMX", id: "dmx" },
  { area: "I/O", tab: "MIDI", id: "midi" },
  { area: "I/O", tab: "OSC", id: "osc" },
  { area: "I/O", tab: "Remote", id: "remote" },
];
const controlTabs = [
  { id: "edit", label: "Live Edit" },
  { id: "live", label: "Timeline" },
  { id: "mixer", label: "VJ Desk" },
];
const viewportRecentProjects = [
  "C:/shows/front-room.sdc",
  "C:/shows/main-stage.sdc",
  "C:/shows/projector-map-test.sdc",
  "C:/shows/festival/live-floor.sdc",
  "C:/shows/long/path/with/a/very-long-syndocal-project-name-for-menu-containment.sdc",
  "C:/shows/backup.sdc",
];
const viewportRecoveryCheckpoint = {
  version: 1,
  app: "Syndocal",
  saved_at: "2026-07-09T12:34:00.000Z",
  source_path: "C:/shows/recovered-main-stage.sdc",
  signature: "viewport-recovery",
  project: {
    version: 1,
    app: "Syndocal",
    custom_profiles: [],
    snapshot: {
      fixtures: [],
      cues: [],
    },
  },
};

const sleep = (ms) => new Promise((resolveSleep) => setTimeout(resolveSleep, ms));

const paethPredictor = (left, up, upperLeft) => {
  const estimate = left + up - upperLeft;
  const leftDistance = Math.abs(estimate - left);
  const upDistance = Math.abs(estimate - up);
  const upperLeftDistance = Math.abs(estimate - upperLeft);
  return leftDistance <= upDistance && leftDistance <= upperLeftDistance
    ? left
    : upDistance <= upperLeftDistance ? up : upperLeft;
};

function decodeScreenshotPng(buffer) {
  if (buffer.length < 24 || buffer.subarray(0, 8).toString("hex") !== "89504e470d0a1a0a") {
    throw new Error("capture is not a PNG");
  }
  let offset = 8;
  let width = 0;
  let height = 0;
  let bitDepth = 0;
  let colorType = 0;
  let interlace = 0;
  const imageChunks = [];
  while (offset + 12 <= buffer.length) {
    const length = buffer.readUInt32BE(offset);
    const type = buffer.toString("ascii", offset + 4, offset + 8);
    const data = buffer.subarray(offset + 8, offset + 8 + length);
    if (type === "IHDR") {
      width = data.readUInt32BE(0);
      height = data.readUInt32BE(4);
      bitDepth = data[8];
      colorType = data[9];
      interlace = data[12];
    } else if (type === "IDAT") {
      imageChunks.push(data);
    } else if (type === "IEND") {
      break;
    }
    offset += length + 12;
  }
  if (bitDepth !== 8 || ![2, 6].includes(colorType) || interlace !== 0 || width <= 0 || height <= 0) {
    throw new Error(`unsupported PNG layout ${width}x${height} depth=${bitDepth} color=${colorType} interlace=${interlace}`);
  }
  const bytesPerPixel = colorType === 6 ? 4 : 3;
  const stride = width * bytesPerPixel;
  const inflated = inflateSync(Buffer.concat(imageChunks));
  const rgba = Buffer.alloc(width * height * 4);
  let inputOffset = 0;
  let previous = Buffer.alloc(stride);
  for (let y = 0; y < height; y += 1) {
    const filter = inflated[inputOffset];
    inputOffset += 1;
    const raw = inflated.subarray(inputOffset, inputOffset + stride);
    inputOffset += stride;
    const row = Buffer.allocUnsafe(stride);
    for (let x = 0; x < stride; x += 1) {
      const left = x >= bytesPerPixel ? row[x - bytesPerPixel] : 0;
      const up = previous[x] ?? 0;
      const upperLeft = x >= bytesPerPixel ? previous[x - bytesPerPixel] : 0;
      const predictor = filter === 0
        ? 0
        : filter === 1
          ? left
          : filter === 2
            ? up
            : filter === 3
              ? Math.floor((left + up) / 2)
              : filter === 4
                ? paethPredictor(left, up, upperLeft)
                : NaN;
      if (!Number.isFinite(predictor)) throw new Error(`unsupported PNG filter ${filter}`);
      row[x] = (raw[x] + predictor) & 0xff;
    }
    for (let x = 0; x < width; x += 1) {
      const source = x * bytesPerPixel;
      const target = (y * width + x) * 4;
      rgba[target] = row[source];
      rgba[target + 1] = row[source + 1];
      rgba[target + 2] = row[source + 2];
      rgba[target + 3] = colorType === 6 ? row[source + 3] : 255;
    }
    previous = row;
  }
  return { width, height, rgba };
}

function screenshotContentStats(decoded) {
  let sampled = 0;
  let exactBlack = 0;
  let bright = 0;
  let accent = 0;
  const colors = new Set();
  for (let y = 0; y < decoded.height; y += 4) {
    for (let x = 0; x < decoded.width; x += 4) {
      const index = (y * decoded.width + x) * 4;
      const red = decoded.rgba[index];
      const green = decoded.rgba[index + 1];
      const blue = decoded.rgba[index + 2];
      sampled += 1;
      if (red === 0 && green === 0 && blue === 0) exactBlack += 1;
      if (red + green + blue >= 300) bright += 1;
      if (red >= 145 && green >= 65 && green <= 205 && blue <= 145) accent += 1;
      if (colors.size < 512) colors.add(`${red >> 3},${green >> 3},${blue >> 3}`);
    }
  }
  return {
    sampled,
    exactBlackRatio: exactBlack / Math.max(1, sampled),
    brightRatio: bright / Math.max(1, sampled),
    accentRatio: accent / Math.max(1, sampled),
    sampledColorCount: colors.size,
  };
}

function screenshotDifferenceRatio(left, right) {
  if (left.width !== right.width || left.height !== right.height) return 1;
  let compared = 0;
  let changed = 0;
  for (let y = 0; y < left.height; y += 4) {
    for (let x = 0; x < left.width; x += 4) {
      const index = (y * left.width + x) * 4;
      compared += 1;
      const delta = Math.abs(left.rgba[index] - right.rgba[index]) +
        Math.abs(left.rgba[index + 1] - right.rgba[index + 1]) +
        Math.abs(left.rgba[index + 2] - right.rgba[index + 2]);
      if (delta > 18) changed += 1;
    }
  }
  return changed / Math.max(1, compared);
}

async function captureSettledViewport(client, outputPath) {
  await client.send("Page.bringToFront");
  await client.evaluate(`(async () => {
    if (document.fonts?.ready) await document.fonts.ready;
    window.dispatchEvent(new Event('resize'));
    document.documentElement.getBoundingClientRect();
    await new Promise((resolveFrame) => requestAnimationFrame(() =>
      requestAnimationFrame(() => requestAnimationFrame(resolveFrame))));
  })()`);
  await sleep(220);
  // Early compositor readbacks can contain unrasterized tiles after a large
  // SVG/list fixture is mounted. Warm successive frames before the QA capture.
  for (let warmup = 0; warmup < 3; warmup += 1) {
    await client.send("Page.captureScreenshot", {
      format: "png",
      fromSurface: true,
      captureBeyondViewport: false,
    });
    await sleep(220);
  }
  const expectedViewport = await client.evaluate(`({ width: window.innerWidth, height: window.innerHeight })`);
  let accepted = null;
  let verification = null;
  for (let attempt = 1; attempt <= 6; attempt += 1) {
    const firstCapture = await client.send("Page.captureScreenshot", {
      format: "png",
      fromSurface: true,
      captureBeyondViewport: false,
    });
    await sleep(80);
    const secondCapture = await client.send("Page.captureScreenshot", {
      format: "png",
      fromSurface: true,
      captureBeyondViewport: false,
    });
    const firstBuffer = Buffer.from(firstCapture.data, "base64");
    const secondBuffer = Buffer.from(secondCapture.data, "base64");
    const first = decodeScreenshotPng(firstBuffer);
    const second = decodeScreenshotPng(secondBuffer);
    const firstStats = screenshotContentStats(first);
    const secondStats = screenshotContentStats(second);
    const differenceRatio = screenshotDifferenceRatio(first, second);
    const dimensionsValid = first.width === expectedViewport.width && first.height === expectedViewport.height &&
      second.width === expectedViewport.width && second.height === expectedViewport.height;
    const contentValid = [firstStats, secondStats].every((stats) =>
      stats.exactBlackRatio < 0.12 &&
      stats.brightRatio > 0.002 &&
      stats.accentRatio > 0.00005 &&
      stats.sampledColorCount >= 24,
    );
    verification = {
      verified: false,
      attempt,
      width: second.width,
      height: second.height,
      dimensionsValid,
      contentValid,
      differenceRatio,
      firstHash: createHash("sha256").update(firstBuffer).digest("hex"),
      secondHash: createHash("sha256").update(secondBuffer).digest("hex"),
      firstStats,
      secondStats,
    };
    if (dimensionsValid && contentValid && differenceRatio <= 0.01) {
      verification.verified = true;
      accepted = secondBuffer;
      break;
    }
    await sleep(180);
  }
  if (!accepted || !verification) {
    throw new Error(`unable to verify stable viewport capture: ${JSON.stringify(verification)}`);
  }
  writeFileSync(outputPath, accepted);
  return verification;
}

function executableExists(command) {
  if (command.includes("\\") || command.includes("/")) {
    return existsSync(command);
  }
  const pathDirs = (process.env.PATH ?? "").split(process.platform === "win32" ? ";" : ":");
  const extensions = process.platform === "win32" ? ["", ".exe", ".cmd", ".bat"] : [""];
  return pathDirs.some((dir) => extensions.some((extension) => existsSync(join(dir, `${command}${extension}`))));
}

function findBrowser() {
  const envCandidates = [process.env.CHROME_PATH, process.env.EDGE_PATH].filter(Boolean);
  const platformCandidates =
    process.platform === "win32"
      ? [
          join(process.env.PROGRAMFILES ?? "C:\\Program Files", "Google\\Chrome\\Application\\chrome.exe"),
          join(process.env["PROGRAMFILES(X86)"] ?? "C:\\Program Files (x86)", "Google\\Chrome\\Application\\chrome.exe"),
          join(process.env.LOCALAPPDATA ?? "", "Google\\Chrome\\Application\\chrome.exe"),
          join(process.env.PROGRAMFILES ?? "C:\\Program Files", "Microsoft\\Edge\\Application\\msedge.exe"),
          join(process.env["PROGRAMFILES(X86)"] ?? "C:\\Program Files (x86)", "Microsoft\\Edge\\Application\\msedge.exe"),
        ]
      : process.platform === "darwin"
        ? [
            "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
            "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge",
            "/Applications/Chromium.app/Contents/MacOS/Chromium",
          ]
        : ["google-chrome-stable", "google-chrome", "chromium", "chromium-browser", "microsoft-edge"];
  return [...envCandidates, ...platformCandidates].find((candidate) => candidate && executableExists(candidate)) ?? null;
}

function startProcess(command, args, options = {}) {
  const child = spawn(command, args, {
    cwd: options.cwd,
    stdio: options.stdio ?? "ignore",
    windowsHide: true,
    shell: false,
    env: { ...process.env, ...(options.env ?? {}) },
  });
  child.on("error", (error) => {
    console.error(`Failed to start ${command}: ${error.message}`);
  });
  return child;
}

async function stopProcess(child) {
  if (!child || child.killed) {
    return;
  }
  try {
    child.kill("SIGTERM");
  } catch {
    // Ignore cleanup races.
  }
  for (let attempt = 0; attempt < 20; attempt += 1) {
    if (child.exitCode !== null || child.signalCode !== null) {
      return;
    }
    await sleep(50);
  }
  try {
    child.kill("SIGKILL");
  } catch {
    // Ignore cleanup races.
  }
}

async function waitForHttp(url, description) {
  let lastError = null;
  for (let attempt = 0; attempt < 80; attempt += 1) {
    try {
      const response = await fetch(url);
      if (response.ok) {
        return response;
      }
      lastError = new Error(`${response.status} ${response.statusText}`);
    } catch (error) {
      lastError = error;
    }
    await sleep(250);
  }
  throw new Error(`${description} did not become ready: ${lastError?.message ?? "unknown error"}`);
}

class CdpClient {
  constructor(webSocketUrl) {
    this.webSocketUrl = webSocketUrl;
    this.nextId = 1;
    this.pending = new Map();
    this.socket = null;
  }

  async connect() {
    this.socket = new WebSocket(this.webSocketUrl);
    await new Promise((resolveConnect, rejectConnect) => {
      this.socket.addEventListener("open", resolveConnect, { once: true });
      this.socket.addEventListener("error", rejectConnect, { once: true });
    });
    this.socket.addEventListener("message", (event) => {
      const data = JSON.parse(event.data);
      if (!data.id || !this.pending.has(data.id)) {
        return;
      }
      const { resolve: resolveCall, reject } = this.pending.get(data.id);
      this.pending.delete(data.id);
      if (data.error) {
        reject(new Error(JSON.stringify(data.error)));
        return;
      }
      resolveCall(data.result);
    });
  }

  send(method, params = {}) {
    if (!this.socket) {
      throw new Error("CDP socket is not connected.");
    }
    const id = this.nextId;
    this.nextId += 1;
    this.socket.send(JSON.stringify({ id, method, params }));
    return new Promise((resolveCall, reject) => {
      this.pending.set(id, { resolve: resolveCall, reject });
    });
  }

  async evaluate(expression) {
    const result = await this.send("Runtime.evaluate", {
      expression,
      awaitPromise: true,
      returnByValue: true,
    });
    if (result.exceptionDetails) {
      const description = result.exceptionDetails.exception?.description ?? result.exceptionDetails.text ?? "Runtime evaluation failed.";
      const location = result.exceptionDetails.stackTrace?.callFrames?.[0];
      const suffix = location ? ` at ${location.url || "<evaluation>"}:${location.lineNumber + 1}:${location.columnNumber + 1}` : "";
      const expressionPreview = expression.replace(/\s+/g, " ").slice(0, 180);
      throw new Error(`${description}${suffix} [expression: ${expressionPreview}]`);
    }
    return result.result.value;
  }

  close() {
    this.socket?.close();
  }
}

async function createCdpClient() {
  const response = await fetch(`http://127.0.0.1:${cdpPort}/json/new?${encodeURIComponent(appUrl)}`, {
    method: "PUT",
  });
  if (!response.ok) {
    throw new Error(`Failed to create Chrome target: ${response.status} ${response.statusText}`);
  }
  const target = await response.json();
  const client = new CdpClient(target.webSocketDebuggerUrl);
  await client.connect();
  await client.send("Runtime.enable");
  await client.send("Page.enable");
  return client;
}

async function waitForApp(client) {
  for (let attempt = 0; attempt < 80; attempt += 1) {
    const ready = await client.evaluate("Boolean(document.querySelector('.app'))");
    if (ready) {
      return;
    }
    await sleep(100);
  }
  throw new Error("Syndocal app shell did not mount.");
}

async function seedViewportLocalStorage(client) {
  await client.evaluate(`(() => {
    window.localStorage.setItem('syndocal.recentProjects.v1', ${JSON.stringify(JSON.stringify(viewportRecentProjects))});
    window.localStorage.setItem('syndocal.projectRecovery.v1', ${JSON.stringify(JSON.stringify(viewportRecoveryCheckpoint))});
    window.localStorage.setItem('syndocal.workspaceLayout.v1', JSON.stringify({
      workspace_tab: 'setup',
      setup_sub_tab: 'patch',
      control_mode: 'edit',
      timeline_desk_surface: 'show',
      edit_desk_surface: 'attributes',
      control_category: 'position',
    }));
  })()`);
}

async function clickByText(client, text) {
  const clicked = await client.evaluate(`(() => {
    const wanted = ${JSON.stringify(text)}.toLowerCase();
    const nodes = [...document.querySelectorAll('button, [role="tab"], a')];
    const exact = nodes.find((node) => (node.textContent || '').trim().toLowerCase() === wanted);
    const loose = exact || nodes.find((node) => (node.textContent || '').trim().toLowerCase().includes(wanted));
    if (!loose) {
      return false;
    }
    loose.click();
    return true;
  })()`);
  if (!clicked) {
    throw new Error(`Could not find tab/button text: ${text}`);
  }
}

async function clickVisibleByText(client, selector, text) {
  const clicked = await client.evaluate(`(() => {
    const wanted = ${JSON.stringify(text)}.toLowerCase();
    const nodes = [...document.querySelectorAll(${JSON.stringify(selector)})]
      .filter((node) => {
        const rect = node.getBoundingClientRect();
        const style = window.getComputedStyle(node);
        return rect.width > 0 && rect.height > 0 && style.display !== 'none' && style.visibility !== 'hidden';
      });
    const exact = nodes.find((node) => (node.textContent || '').trim().toLowerCase() === wanted);
    const loose = exact || nodes.find((node) => (node.textContent || '').trim().toLowerCase().includes(wanted));
    if (!loose) {
      return false;
    }
    loose.click();
    return true;
  })()`);
  if (!clicked) {
    throw new Error(`Could not find visible ${selector} text: ${text}`);
  }
}

async function evaluatePageFunction(client, callback, ...args) {
  const serializedArgs = args.map((argument) => JSON.stringify(argument)).join(",");
  return client.evaluate(`(${callback.toString()})(${serializedArgs})`);
}

async function clickVisibleSelector(client, selector) {
  const clicked = await client.evaluate(`(() => {
    const node = document.querySelector(${JSON.stringify(selector)});
    if (!node || node.disabled) return false;
    const rect = node.getBoundingClientRect();
    const style = window.getComputedStyle(node);
    if (rect.width <= 0 || rect.height <= 0 || style.display === 'none' || style.visibility === 'hidden') {
      return false;
    }
    node.click();
    return true;
  })()`);
  if (!clicked) {
    throw new Error(`Could not find visible selector: ${selector}`);
  }
}

async function selectVisibleOption(client, selector, value) {
  const selected = await client.evaluate(`(() => {
    const wanted = ${JSON.stringify(value)};
    const select = [...document.querySelectorAll(${JSON.stringify(selector)})]
      .filter((candidate) => candidate instanceof HTMLSelectElement)
      .filter((candidate) => {
        const rect = candidate.getBoundingClientRect();
        const style = window.getComputedStyle(candidate);
        return rect.width > 0 && rect.height > 0 && style.display !== 'none' && style.visibility !== 'hidden';
      })
      .find((candidate) => [...candidate.options].some((option) => option.value === wanted));
    if (!select) {
      return false;
    }
    select.value = wanted;
    select.dispatchEvent(new InputEvent('input', { bubbles: true }));
    select.dispatchEvent(new Event('change', { bubbles: true }));
    return select.value === wanted;
  })()`);
  if (!selected) {
    throw new Error(`Could not select visible ${selector} option: ${value}`);
  }
}

async function pressKey(client, code, key = code, modifiers = 0) {
  await client.send("Input.dispatchKeyEvent", { type: "keyDown", code, key, modifiers });
  await client.send("Input.dispatchKeyEvent", { type: "keyUp", code, key, modifiers });
}

async function focusTimelineOverlapClusterWithTab(client, track) {
  await evaluatePageFunction(client, () => {
    const revealPlayhead = [...document.querySelectorAll(".timelineViewportToolbar button")]
      .find((button) => (button.textContent || "").trim() === "Reveal Playhead");
    revealPlayhead?.focus();
  });
  let active = null;
  let tabsToCluster = 0;
  for (let tab = 1; tab <= 5; tab += 1) {
    await pressKey(client, "Tab");
    active = await evaluatePageFunction(client, () => ({
      className: document.activeElement?.getAttribute("class") ?? "",
      track: document.activeElement?.getAttribute("data-overlap-track") ?? "",
      ariaLabel: document.activeElement?.getAttribute("aria-label") ?? "",
    }));
    if (active.track === track) {
      tabsToCluster = tab;
      break;
    }
  }
  await client.send("Accessibility.enable");
  const accessibility = await client.send("Accessibility.getFullAXTree");
  const axButtonFound = (accessibility.nodes ?? []).some((node) =>
    node.role?.value === "button" &&
    String(node.name?.value ?? "").includes(track + " overlap") &&
    String(node.name?.value ?? "").includes("250"));
  await client.send("Accessibility.disable");
  const tabOrderStats = await evaluatePageFunction(client, () => ({
    markerTabStopCount: document.querySelectorAll('.timelineMarker[tabindex="0"]').length,
    clusterTabStopCount: document.querySelectorAll('.timelineOverlapCluster[tabindex="0"]').length,
    overviewRole: document.querySelector(".timelineOverview")?.getAttribute("role") ?? "",
  }));
  return { active, tabsToCluster, axButtonFound, ...tabOrderStats };
}

async function readTimelineRulerBounds(client) {
  return evaluatePageFunction(client, () => {
    const overview = document.querySelector(".timelineOverview");
    const viewBoxWidth = overview instanceof SVGSVGElement ? overview.viewBox.baseVal.width : 0;
    const bounds = [...document.querySelectorAll("[data-timeline-ruler-ms] text")].map((label) => {
      const box = label instanceof SVGGraphicsElement ? label.getBBox() : null;
      return box
        ? { x: box.x, right: box.x + box.width, width: box.width, text: label.textContent ?? "" }
        : null;
    }).filter(Boolean);
    return {
      count: bounds.length,
      viewBoxWidth,
      minimumX: bounds.length > 0 ? Math.min(...bounds.map((box) => box.x)) : null,
      maximumRight: bounds.length > 0 ? Math.max(...bounds.map((box) => box.right)) : null,
      contained: bounds.length > 0 && bounds.every((box) =>
        box.x >= -0.05 && box.right <= viewBoxWidth + 0.05),
    };
  });
}

async function checkWorkspaceLayoutPersistence(client) {
  await clickVisibleByText(client, ".workspaceTabs button", "Control");
  await clickVisibleByText(client, ".controlModeTabs button", "Timeline");
  await clickVisibleByText(client, ".timelineDeskTabs button", "Playback");
  await sleep(100);
  await client.send("Page.navigate", { url: appUrl });
  await waitForApp(client);
  const restored = await client.evaluate(`(() => ({
    workspace: (document.querySelector('.workspaceTabs button.active')?.textContent || '').trim(),
    controlMode: (document.querySelector('.controlModeTabs button.active')?.textContent || '').trim(),
    desk: (document.querySelector('.timelineDeskTabs button.active')?.textContent || '').trim(),
  }))()`);
  if (restored.workspace !== "Control" || restored.controlMode !== "Timeline" || restored.desk !== "Playback") {
    throw new Error(`Workspace layout did not restore: ${JSON.stringify(restored)}`);
  }

  await clickVisibleByText(client, ".appMenuButton", "...");
  await clickVisibleByText(client, ".appProjectMenu button", "Reset Layout");
  await sleep(100);
  const reset = await client.evaluate(`(() => {
    const stored = JSON.parse(window.localStorage.getItem('syndocal.workspaceLayout.v1') || '{}');
    return {
      workspace: (document.querySelector('.workspaceTabs button.active')?.textContent || '').trim(),
      setupTab: (document.querySelector('.setupModeTabs button.active')?.textContent || '').trim(),
      stored,
    };
  })()`);
  if (
    reset.workspace !== "Setup" ||
    reset.setupTab !== "Patch" ||
    reset.stored.workspace_tab !== "setup" ||
    reset.stored.setup_sub_tab !== "patch" ||
    reset.stored.control_mode !== "edit"
  ) {
    throw new Error(`Workspace layout reset failed: ${JSON.stringify(reset)}`);
  }
  console.log("pass workspace-layout-persistence");
}

async function checkKeyboardNavigation(client) {
  const firstWorkspaceFocus = await client.evaluate(`(() => {
    const button = [...document.querySelectorAll('.workspaceTabs button')]
      .find((candidate) => (candidate.textContent || '').trim() === 'Setup');
    button?.focus();
    return (document.activeElement?.textContent || '').trim();
  })()`);
  await pressKey(client, "Tab", "Tab");
  const tabOrder = await client.evaluate(`(() => {
    const active = document.activeElement;
    const style = active ? getComputedStyle(active) : null;
    return {
      label: (active?.textContent || '').trim(),
      outlineStyle: style?.outlineStyle ?? '',
      outlineWidth: Number.parseFloat(style?.outlineWidth ?? '0'),
    };
  })()`);

  await pressKey(client, "F1");
  await sleep(60);
  await clickByText(client, "Mapping");
  await clickByText(client, "Stage Map");
  await sleep(80);
  const setupEditableGuard = await client.evaluate(`(async () => {
    const visible = (element) => {
      const rect = element.getBoundingClientRect();
      const style = getComputedStyle(element);
      return !element.disabled && rect.width > 0 && rect.height > 0 && style.display !== 'none' && style.visibility !== 'hidden';
    };
    const input = [...document.querySelectorAll('.layoutSetup input:not([type="checkbox"])')].find(visible);
    const activeSetupBefore = (document.querySelector('.setupModeTabs button.active')?.textContent || '').trim();
    const activeToolBefore = (document.querySelector('.mappingViewportControls strong')?.textContent || '').trim();
    input?.focus();
    input?.dispatchEvent(new KeyboardEvent('keydown', { bubbles: true, cancelable: true, code: 'Digit1', key: '1' }));
    input?.dispatchEvent(new KeyboardEvent('keydown', { bubbles: true, cancelable: true, code: 'KeyP', key: 'p' }));
    await new Promise((resolveFrame) => requestAnimationFrame(resolveFrame));
    const activeSetupAfter = (document.querySelector('.setupModeTabs button.active')?.textContent || '').trim();
    const activeToolAfter = (document.querySelector('.mappingViewportControls strong')?.textContent || '').trim();
    return {
      found: Boolean(input),
      setupModePreserved: activeSetupBefore.length > 0 && activeSetupAfter === activeSetupBefore,
      mappingToolPreserved: activeToolBefore.length > 0 && activeToolAfter === activeToolBefore,
    };
  })()`);

  await pressKey(client, "F2");
  await sleep(80);
  const controlEditableGuard = await client.evaluate(`(async () => {
    const visible = (element) => {
      const rect = element.getBoundingClientRect();
      const style = getComputedStyle(element);
      return !element.disabled && rect.width > 0 && rect.height > 0 && style.display !== 'none' && style.visibility !== 'hidden';
    };
    const input = [...document.querySelectorAll('.layoutControl input:not([type="checkbox"]), .layoutControl select')].find(visible);
    const activeModeBefore = (document.querySelector('.controlModeTabs button.active')?.textContent || '').trim();
    input?.focus();
    input?.dispatchEvent(new KeyboardEvent('keydown', { bubbles: true, cancelable: true, code: 'KeyL', key: 'l' }));
    await new Promise((resolveFrame) => requestAnimationFrame(resolveFrame));
    const activeModeAfter = (document.querySelector('.controlModeTabs button.active')?.textContent || '').trim();
    return {
      found: Boolean(input),
      modePreserved: activeModeBefore === 'Live Edit' && activeModeAfter === activeModeBefore,
    };
  })()`);

  const result = {
    firstWorkspaceFocus,
    tabOrder,
    setupEditableGuard,
    controlEditableGuard,
  };
  result.passed =
    firstWorkspaceFocus === "Setup" &&
    tabOrder.label === "Control" &&
    tabOrder.outlineStyle !== "none" &&
    tabOrder.outlineWidth >= 1 &&
    setupEditableGuard.found &&
    setupEditableGuard.setupModePreserved &&
    setupEditableGuard.mappingToolPreserved &&
    controlEditableGuard.found &&
    controlEditableGuard.modePreserved;
  await client.evaluate(`(() => {
    window.__syndocalKeyboardNavigationCheck = ${JSON.stringify(result)};
  })()`);
  return result.passed;
}

async function measure(client, label) {
  return await client.evaluate(`(async () => {
    await new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame)));
    const visibleCount = (selector) => [...document.querySelectorAll(selector)]
      .filter((element) => {
        const rect = element.getBoundingClientRect();
        const style = window.getComputedStyle(element);
        return rect.width > 0 && rect.height > 0 && style.display !== 'none' && style.visibility !== 'hidden';
      }).length;
    const fullyVisibleCount = (selector, containerSelector) => {
      const container = containerSelector ? document.querySelector(containerSelector) : null;
      const containerRect = container?.getBoundingClientRect() ?? {
        left: 0,
        top: 0,
        right: innerWidth,
        bottom: innerHeight,
      };
      return [...document.querySelectorAll(selector)]
        .filter((element) => {
          const rect = element.getBoundingClientRect();
          const style = window.getComputedStyle(element);
          return rect.width > 0 && rect.height > 0 && style.display !== 'none' && style.visibility !== 'hidden' &&
            rect.left >= Math.max(0, containerRect.left) - 1 &&
            rect.right <= Math.min(innerWidth, containerRect.right) + 1 &&
            rect.top >= Math.max(0, containerRect.top) - 1 &&
            rect.bottom <= Math.min(innerHeight, containerRect.bottom) + 1;
        }).length;
    };
    const lastControlReachableWhenScrolled = (containerSelector, controlSelector) => {
      const container = document.querySelector(containerSelector);
      const controls = container ? [...container.querySelectorAll(controlSelector)] : [];
      const control = controls.at(-1);
      if (!container || !control) return false;
      const scrollableAncestors = [];
      for (let ancestor = container.parentElement; ancestor; ancestor = ancestor.parentElement) {
        const overflowY = getComputedStyle(ancestor).overflowY;
        if (/(auto|scroll)/.test(overflowY) && ancestor.scrollHeight > ancestor.clientHeight + 1) {
          scrollableAncestors.push(ancestor);
        }
      }
      const ancestorScrollTops = scrollableAncestors.map((ancestor) => ancestor.scrollTop);
      const previousScrollTop = container.scrollTop;
      for (const ancestor of [...scrollableAncestors].reverse()) {
        ancestor.scrollTop = ancestor.scrollHeight;
      }
      container.scrollTop = container.scrollHeight;
      const containerRect = container.getBoundingClientRect();
      const controlRect = control.getBoundingClientRect();
      const reachable = controlRect.width > 0 && controlRect.height > 0 &&
        controlRect.left >= Math.max(0, containerRect.left) - 1 &&
        controlRect.right <= Math.min(innerWidth, containerRect.right) + 1 &&
        controlRect.top >= Math.max(0, containerRect.top) - 1 &&
        controlRect.bottom <= Math.min(innerHeight, containerRect.bottom) + 1;
      container.scrollTop = previousScrollTop;
      scrollableAncestors.forEach((ancestor, index) => {
        ancestor.scrollTop = ancestorScrollTops[index];
      });
      return reachable;
    };
    const shrunkenDirectChildCount = (selector) => {
      const parent = document.querySelector(selector);
      return parent
        ? [...parent.children].filter((element) => element.scrollHeight > element.clientHeight + 1).length
        : 0;
    };
    const directChildOverlapCount = (selector) => {
      const parent = document.querySelector(selector);
      if (!parent) return 0;
      const rects = [...parent.children]
        .map((element) => element.getBoundingClientRect())
        .filter((rect) => rect.width > 0 && rect.height > 0)
        .sort((left, right) => left.top - right.top);
      return rects.slice(1).filter((rect, index) => rects[index].bottom > rect.top + 1).length;
    };
    const documentElement = document.documentElement;
    const body = document.body;
    const app = document.querySelector('.app');
    const layout = document.querySelector('.layout');
    const documentStyle = window.getComputedStyle(documentElement);
    const bodyStyle = window.getComputedStyle(body);
    const appStyle = app ? window.getComputedStyle(app) : null;
    const layoutStyle = layout ? window.getComputedStyle(layout) : null;
    const appRect = app ? app.getBoundingClientRect() : null;
    const layoutRect = layout ? layout.getBoundingClientRect() : null;
    const compactMappingStage = document.querySelector('.layoutSetup.setupMode-patch .mappingVisualizer.compact .visualizerStage');
    const compactMappingStageRect = compactMappingStage?.getBoundingClientRect() ?? null;
    const videoSetupSidebar = document.querySelector('.videoSetupPanel .videoSetupSidebar');
    const videoSetupSidebarRect = videoSetupSidebar?.getBoundingClientRect() ?? null;
    const videoSetupOutputDesk = document.querySelector('.videoSetupPanel .videoSetupOutputDesk');
    const videoSetupOutputDeskRect = videoSetupOutputDesk?.getBoundingClientRect() ?? null;
    const videoSetupRoutingPane = document.querySelector('.videoSetupPanel .videoSetupRoutingPane');
    const videoSetupRoutingPaneRect = videoSetupRoutingPane?.getBoundingClientRect() ?? null;
    const videoSetupMapPane = document.querySelector('.videoSetupPanel .videoSetupMapPane');
    const videoSetupMapPaneRect = videoSetupMapPane?.getBoundingClientRect() ?? null;
    const videoSetupInspectorPane = document.querySelector('.videoSetupPanel .videoSetupInspectorPane');
    const videoSetupInspectorPaneRect = videoSetupInspectorPane?.getBoundingClientRect() ?? null;
    const videoSetupProjectorSurface = document.querySelector('.videoSetupPanel .projectorMapSurface');
    const videoSetupProjectorSurfaceRect = videoSetupProjectorSurface?.getBoundingClientRect() ?? null;
    const videoSetupPreviewCard = document.querySelector('.videoSetupPanel .videoSetupInspectorPane .videoOutputPreviewCard');
    const videoSetupPreviewCardRect = videoSetupPreviewCard?.getBoundingClientRect() ?? null;
    const videoSetupActionDock = document.querySelector('.videoSetupPanel .videoOutputActionDock');
    const videoSetupActionDockRect = videoSetupActionDock?.getBoundingClientRect() ?? null;
    const profileLoadPanel = document.querySelector('.setupMode-library .profileLoadPanel');
    const profileLoadPanelRect = profileLoadPanel?.getBoundingClientRect() ?? null;
    const loadedProfileSummaryPanel = document.querySelector('.setupMode-library .loadedProfileSummaryPanel');
    const loadedProfileSummaryPanelRect = loadedProfileSummaryPanel?.getBoundingClientRect() ?? null;
    const customProfileAttributePane = document.querySelector('.setupMode-profiles .customProfileAttributePane');
    const customProfileAttributePaneRect = customProfileAttributePane?.getBoundingClientRect() ?? null;
    const customProfilePreviewDesk = document.querySelector('.setupMode-profiles .customProfilePreviewDesk');
    const customProfilePreviewDeskRect = customProfilePreviewDesk?.getBoundingClientRect() ?? null;
    const dmxOutputConfigPanel = document.querySelector('.setupMode-dmx .dmxOutputConfigPanel');
    const dmxOutputConfigPanelRect = dmxOutputConfigPanel?.getBoundingClientRect() ?? null;
    const outputDiagnosticsDesk = document.querySelector('.setupMode-dmx .outputDiagnosticsDesk');
    const outputDiagnosticsDeskRect = outputDiagnosticsDesk?.getBoundingClientRect() ?? null;
    const lightingRuntimeDesk = document.querySelector('.setupMode-dmx .lightingRuntimeDesk');
    const lightingRuntimeDeskRect = lightingRuntimeDesk?.getBoundingClientRect() ?? null;
    const midiMappingEditorDesk = document.querySelector('.setupMode-midi .mappingEditorDesk');
    const midiMappingEditorDeskRect = midiMappingEditorDesk?.getBoundingClientRect() ?? null;
    const midiMappingListDesk = document.querySelector('.setupMode-midi .mappingListDesk');
    const midiMappingListDeskRect = midiMappingListDesk?.getBoundingClientRect() ?? null;
    const oscMappingEditorDesk = document.querySelector('.setupMode-osc .mappingEditorDesk');
    const oscMappingEditorDeskRect = oscMappingEditorDesk?.getBoundingClientRect() ?? null;
    const oscMappingListDesk = document.querySelector('.setupMode-osc .mappingListDesk');
    const oscMappingListDeskRect = oscMappingListDesk?.getBoundingClientRect() ?? null;
    const remoteServerDesk = document.querySelector('.setupMode-remote .remoteServerDesk');
    const remoteServerDeskRect = remoteServerDesk?.getBoundingClientRect() ?? null;
    const remoteEndpointDesk = document.querySelector('.setupMode-remote .remoteEndpointDesk');
    const remoteEndpointDeskRect = remoteEndpointDesk?.getBoundingClientRect() ?? null;
    const cuePanel = document.querySelector('.cuePanel');
    const cueEditToggle = document.querySelector('.cuePanelEditToggle');
    const cueHost = document.querySelector('.layoutControl.controlModeLive .faders');
    const timelinePanel = document.querySelector('.layoutControl.controlModeLive .timelinePanel');
    const timelinePanelRect = timelinePanel?.getBoundingClientRect() ?? null;
    const sceneBlockWorkspace = document.querySelector('.sceneBlockWorkspace');
    const sceneBlockWorkspaceRect = sceneBlockWorkspace?.getBoundingClientRect() ?? null;
    const sceneBlockList = document.querySelector('.sceneBlockList');
    const sceneBlockComposer = document.querySelector('.sceneBlockComposer');
    const moveEffectForm = document.querySelector('.effectInspectorPane > .effectForm');
    const moveEffectEditorPanel = document.querySelector('.moveEffectEditorPanel');
    const moveEffectEditorSurface = document.querySelector('.moveEffectEditorSurface');
    const moveEffectPathDesk = document.querySelector('.moveEffectPathDesk');
    const moveEffectInspector = document.querySelector('.moveEffectInspector');
    const moveEffectPathCanvas = document.querySelector('.moveEffectPathCanvas');
    const moveEffectPointList = document.querySelector('.moveEffectPointList');
    const positionToolPane = document.querySelector('.positionToolPane');
    let positionToolPaneLastControlReachable = false;
    if (positionToolPane) {
      const initialScrollTop = positionToolPane.scrollTop;
      const controls = [...positionToolPane.querySelectorAll('button, input, select')]
        .filter((element) => !element.disabled);
      const lastControl = controls.at(-1);
      positionToolPane.scrollTop = positionToolPane.scrollHeight;
      await new Promise((resolveFrame) => requestAnimationFrame(resolveFrame));
      if (lastControl) {
        const paneRect = positionToolPane.getBoundingClientRect();
        const controlRect = lastControl.getBoundingClientRect();
        positionToolPaneLastControlReachable =
          controlRect.top >= paneRect.top - 1 && controlRect.bottom <= paneRect.bottom + 1;
      }
      positionToolPane.scrollTop = initialScrollTop;
      await new Promise((resolveFrame) => requestAnimationFrame(resolveFrame));
    }
    let moveEffectLastControlReachable = false;
    if (moveEffectForm && moveEffectEditorPanel) {
      const initialScrollTop = moveEffectForm.scrollTop;
      const controls = [...moveEffectEditorPanel.querySelectorAll('button, input, select')]
        .filter((element) => !element.disabled && !element.closest('.moveEffectPointList'))
        .filter((element) => {
          const rect = element.getBoundingClientRect();
          const style = window.getComputedStyle(element);
          return rect.width > 0 && rect.height > 0 && style.display !== 'none' && style.visibility !== 'hidden';
        });
      const lastControl = controls.at(-1);
      moveEffectForm.scrollTop = moveEffectForm.scrollHeight;
      await new Promise((resolveFrame) => requestAnimationFrame(resolveFrame));
      if (lastControl) {
        const formRect = moveEffectForm.getBoundingClientRect();
        const controlRect = lastControl.getBoundingClientRect();
        moveEffectLastControlReachable =
          controlRect.top >= formRect.top - 1 && controlRect.bottom <= formRect.bottom + 1;
      }
      moveEffectForm.scrollTop = initialScrollTop;
      await new Promise((resolveFrame) => requestAnimationFrame(resolveFrame));
    }
    let moveEffectLastPointReachable = false;
    if (moveEffectPointList) {
      const initialScrollTop = moveEffectPointList.scrollTop;
      const lastRow = [...moveEffectPointList.querySelectorAll('.moveEffectPointRow')].at(-1);
      moveEffectPointList.scrollTop = moveEffectPointList.scrollHeight;
      await new Promise((resolveFrame) => requestAnimationFrame(resolveFrame));
      if (lastRow) {
        const listRect = moveEffectPointList.getBoundingClientRect();
        const rowRect = lastRow.getBoundingClientRect();
        moveEffectLastPointReachable = rowRect.top >= listRect.top - 1 && rowRect.bottom <= listRect.bottom + 1;
      }
      moveEffectPointList.scrollTop = initialScrollTop;
      await new Promise((resolveFrame) => requestAnimationFrame(resolveFrame));
    }
    const moveEffectFormRect = moveEffectForm?.getBoundingClientRect() ?? null;
    const moveEffectEditorPanelRect = moveEffectEditorPanel?.getBoundingClientRect() ?? null;
    const moveEffectEditorSurfaceRect = moveEffectEditorSurface?.getBoundingClientRect() ?? null;
    const moveEffectPathDeskRect = moveEffectPathDesk?.getBoundingClientRect() ?? null;
    const moveEffectInspectorRect = moveEffectInspector?.getBoundingClientRect() ?? null;
    const moveEffectPathCanvasRect = moveEffectPathCanvas?.getBoundingClientRect() ?? null;
    window.scrollTo(9999, 9999);
    await new Promise((resolveFrame) => requestAnimationFrame(resolveFrame));
    const movedX = window.scrollX;
    const movedY = window.scrollY;
    window.scrollTo(0, 0);
    await new Promise((resolveFrame) => requestAnimationFrame(resolveFrame));
    return {
      label: ${JSON.stringify(label)},
      innerWidth,
      innerHeight,
      documentClientWidth: documentElement.clientWidth,
      documentScrollWidth: documentElement.scrollWidth,
      documentClientHeight: documentElement.clientHeight,
      documentScrollHeight: documentElement.scrollHeight,
      bodyScrollWidth: body.scrollWidth,
      bodyScrollHeight: body.scrollHeight,
      appClientWidth: app ? app.clientWidth : null,
      appScrollWidth: app ? app.scrollWidth : null,
      appClientHeight: app ? app.clientHeight : null,
      appScrollHeight: app ? app.scrollHeight : null,
      appPosition: appStyle?.position ?? '',
      appOverflowX: appStyle?.overflowX ?? '',
      appOverflowY: appStyle?.overflowY ?? '',
      appRectWidth: appRect ? Math.round(appRect.width) : null,
      appRectHeight: appRect ? Math.round(appRect.height) : null,
      documentOverflowX: documentStyle.overflowX,
      documentOverflowY: documentStyle.overflowY,
      bodyOverflowX: bodyStyle.overflowX,
      bodyOverflowY: bodyStyle.overflowY,
      layoutOverflowX: layoutStyle?.overflowX ?? '',
      layoutOverflowY: layoutStyle?.overflowY ?? '',
      layoutClientWidth: layout ? layout.clientWidth : null,
      layoutClientHeight: layout ? layout.clientHeight : null,
      layoutScrollWidth: layout ? layout.scrollWidth : null,
      layoutScrollHeight: layout ? layout.scrollHeight : null,
      layoutRectWidth: layoutRect ? Math.round(layoutRect.width) : null,
      layoutRectHeight: layoutRect ? Math.round(layoutRect.height) : null,
      visibleAppStatusLineCount: visibleCount('.appStatusLine[role="status"]'),
      appStatusTone: document.querySelector('.appStatusLine')?.getAttribute('data-status-tone') ?? '',
      visibleProjectMenuCount: visibleCount('.appProjectMenu'),
      visibleProjectMenuItemCount: visibleCount('.appProjectMenu button[role="menuitem"]'),
      visibleProjectMenuShortcutCount: document.querySelectorAll('.appProjectMenu button[aria-keyshortcuts]').length,
      visibleRecentProjectMenuItemCount: visibleCount('.appProjectMenu .recentProjectMenuItem'),
      visibleRecoveryProjectMenuItemCount: visibleCount('.appProjectMenu .recoveryProjectMenuItem'),
      visibleUpdateMenuLabelCount: [...document.querySelectorAll('.appProjectMenu .appProjectMenuLabel span')]
        .filter((node) => (node.textContent || '').trim() === 'Application Updates').length,
      visibleUpdateCheckButtonCount: [...document.querySelectorAll('.appProjectMenu button[role="menuitem"]')]
        .filter((button) => (button.textContent || '').includes('Check for Updates')).length,
      visibleUserTemplateMenuLabelCount: [...document.querySelectorAll('.appProjectMenu .appProjectMenuLabel span')]
        .filter((node) => (node.textContent || '').trim() === 'User Templates').length,
      visibleUserTemplateActionCount: [...document.querySelectorAll('.appProjectMenu button[role="menuitem"]')]
        .filter((button) => /(?:New from|Save as) Template/.test(button.textContent || '')).length,
      documentLanguage: document.documentElement.lang,
      visibleJapaneseLanguageLabelCount: [...document.querySelectorAll('.appProjectMenu .appProjectMenuLabel span')]
        .filter((node) => (node.textContent || '').trim() === 'UI言語').length,
      visibleJapaneseSaveButtonCount: [...document.querySelectorAll('.appProjectMenu button[role="menuitem"] span')]
        .filter((node) => (node.textContent || '').trim() === '保存').length,
      preservedUserFixtureLabelCount: [...document.querySelectorAll('.fixtureList .fixture strong')]
        .filter((node) => ['Video', 'Save', 'Output'].includes((node.textContent || '').trim())).length,
      translatedUserFixtureCollisionCount: [...document.querySelectorAll('.fixtureList .fixture strong')]
        .filter((node) => ['映像', '保存', '出力'].includes((node.textContent || '').trim())).length,
      timelineAutomationRangeCount: document.querySelectorAll('.timelineOverview .timelineAutomationRange').length,
      timelineAutomationHandleCount: document.querySelectorAll('.timelineOverview .timelineAutomationHandle').length,
      timelineAutomationKeyframeCount: document.querySelectorAll('.timelineOverview .timelineAutomationKeyframe').length,
      timelineAutomationChipCount: document.querySelectorAll('.timelineKeyframeStrip button').length,
      timelineAutomationCurveCount: document.querySelectorAll('.timelineKeyframeCurve').length,
      timelineAutomationValueInputCount: document.querySelectorAll('.timelineKeyframeValue').length,
      timelineAutomationEnabledToggleCount: document.querySelectorAll('.timelineAutomationEnabled input[type="checkbox"]').length,
      timelineAutomationScopeCount: document.querySelectorAll('.timelineAutomationScope').length,
      timelineAutomationBatchButtonCount: document.querySelectorAll('.timelineAutomationBatchActions button').length,
      timelineAutomationGroupButtonCount: [...document.querySelectorAll('.timelineAutomationActions button')]
        .filter((button) => (button.textContent || '').trim().toLowerCase() === 'group').length,
      timelineAutomationRangeWidths: [...document.querySelectorAll('.timelineOverview .timelineAutomationRange > rect:not(.timelineAutomationHandle)')]
        .map((rect) => Number(rect.getAttribute('width') || 0)),
      mappingUseInEffectsButtonCount: [...document.querySelectorAll('.mappingSelectionPanel button')]
        .filter((button) => (button.textContent || '').trim().toLowerCase() === 'use in effects').length,
      mappingWaveDraftButtonCount: [...document.querySelectorAll('.mappingSelectionPanel button')]
        .filter((button) => (button.textContent || '').trim().toLowerCase() === 'wave draft').length,
      visibleMappingHotkeyHelpCount: visibleCount('.mappingHotkeyHelp'),
      mappingHotkeyHelpKeyCount: document.querySelectorAll('.mappingHotkeyHelp kbd').length,
      mappingFilterVerticalClipCount: [...document.querySelectorAll('.setupMode-mapping .mappingGroupStrip, .setupMode-mapping .mappingTypeStrip')]
        .filter((element) => element.scrollHeight > element.clientHeight + 1).length,
      mappingViewportShrunkenChildCount: shrunkenDirectChildCount('.setupMode-mapping .mappingStageViewport'),
      mappingViewportChildOverlapCount: directChildOverlapCount('.setupMode-mapping .mappingStageViewport'),
      mappingStageHeight: Math.round(document.querySelector('.setupMode-mapping .mappingStageViewport .visualizerStage')?.getBoundingClientRect().height ?? 0),
      mappingSidebarHorizontalOverflowPx: (() => {
        const sidebar = document.querySelector('.setupMode-mapping .mappingSelectionPanel');
        return sidebar ? Math.max(0, sidebar.scrollWidth - sidebar.clientWidth) : 0;
      })(),
      mappingSidebarClippedControlCount: (() => {
        const sidebar = document.querySelector('.setupMode-mapping .mappingSelectionPanel');
        if (!sidebar) return 0;
        const bounds = sidebar.getBoundingClientRect();
        return [...sidebar.querySelectorAll('button, input, select')].filter((element) => {
          const rect = element.getBoundingClientRect();
          return rect.width > 0 && (rect.left < bounds.left - 1 || rect.right > bounds.right + 1);
        }).length;
      })(),
      visiblePatchActionRowCount: visibleCount('.patchActionRow'),
      visiblePatchAutoButtonCount: [...document.querySelectorAll('.fieldWithAction button')]
        .filter((button) => (button.textContent || '').trim().toLowerCase() === 'auto').length,
      visiblePatchPrimaryButtonCount: [...document.querySelectorAll('.patchActionRow button.primary')]
        .filter((button) => (button.textContent || '').trim().toLowerCase().startsWith('patch fixture')).length,
      visiblePatchNextFreeButtonCount: [...document.querySelectorAll('.patchActionRow button')]
        .filter((button) => (button.textContent || '').trim().toLowerCase().startsWith('next free')).length,
      visiblePatchFootprintCount: visibleCount('.footprint'),
      visibleDmxAddressGridCount: visibleCount('.dmxAddressGrid'),
      dmxAddressCellCount: document.querySelectorAll('.dmxAddressCell').length,
      dmxAddressOccupiedCellCount: document.querySelectorAll('.dmxAddressCell.occupied').length,
      dmxAddressPlannedCellCount: document.querySelectorAll('.dmxAddressCell.planned').length,
      visibleDmxFixtureBlockCount: visibleCount('.dmxPatchFixtureBlock'),
      compactMappingStageWidth: compactMappingStageRect ? Math.round(compactMappingStageRect.width) : 0,
      compactMappingStageHeight: compactMappingStageRect ? Math.round(compactMappingStageRect.height) : 0,
      videoSetupSidebarWidth: videoSetupSidebarRect ? Math.round(videoSetupSidebarRect.width) : 0,
      videoSetupOutputDeskWidth: videoSetupOutputDeskRect ? Math.round(videoSetupOutputDeskRect.width) : 0,
      visibleVideoSetupRoutingPaneCount: visibleCount('.videoSetupPanel .videoSetupRoutingPane'),
      visibleVideoSetupMapPaneCount: visibleCount('.videoSetupPanel .videoSetupMapPane'),
      visibleVideoSetupInspectorPaneCount: visibleCount('.videoSetupPanel .videoSetupInspectorPane'),
      videoSetupRoutingPaneWidth: videoSetupRoutingPaneRect ? Math.round(videoSetupRoutingPaneRect.width) : 0,
      videoSetupMapPaneWidth: videoSetupMapPaneRect ? Math.round(videoSetupMapPaneRect.width) : 0,
      videoSetupInspectorPaneWidth: videoSetupInspectorPaneRect ? Math.round(videoSetupInspectorPaneRect.width) : 0,
      videoSetupMapPaneHeight: videoSetupMapPaneRect ? Math.round(videoSetupMapPaneRect.height) : 0,
      videoSetupMapPaneOverflowPx: videoSetupMapPane
        ? Math.max(0, videoSetupMapPane.scrollHeight - videoSetupMapPane.clientHeight)
        : 0,
      videoSetupProjectorSurfaceContained: Boolean(
        videoSetupProjectorSurfaceRect &&
        videoSetupMapPaneRect &&
        videoSetupProjectorSurfaceRect.left >= videoSetupMapPaneRect.left - 1 &&
        videoSetupProjectorSurfaceRect.right <= videoSetupMapPaneRect.right + 1 &&
        videoSetupProjectorSurfaceRect.top >= videoSetupMapPaneRect.top - 1 &&
        videoSetupProjectorSurfaceRect.bottom <= Math.min(videoSetupMapPaneRect.bottom, innerHeight) + 1
      ),
      videoSetupPreviewContained: Boolean(
        videoSetupPreviewCardRect &&
        videoSetupInspectorPaneRect &&
        videoSetupPreviewCardRect.left >= videoSetupInspectorPaneRect.left - 1 &&
        videoSetupPreviewCardRect.right <= videoSetupInspectorPaneRect.right + 1 &&
        videoSetupPreviewCardRect.top >= videoSetupInspectorPaneRect.top - 1 &&
        videoSetupPreviewCardRect.bottom <= Math.min(videoSetupInspectorPaneRect.bottom, innerHeight) + 1
      ),
      visibleVideoSetupActionDockCount: visibleCount('.videoSetupPanel .videoOutputActionDock'),
      videoSetupActionDockInViewport: Boolean(
        videoSetupActionDockRect &&
        videoSetupInspectorPaneRect &&
        videoSetupActionDockRect.left >= videoSetupInspectorPaneRect.left - 1 &&
        videoSetupActionDockRect.right <= videoSetupInspectorPaneRect.right + 1 &&
        videoSetupActionDockRect.top >= videoSetupInspectorPaneRect.top - 1 &&
        videoSetupActionDockRect.bottom <= Math.min(videoSetupInspectorPaneRect.bottom, innerHeight) + 1 &&
        videoSetupActionDockRect.height >= 96
      ),
      videoSetupCriticalActionInViewportCount: ['toggle-blackout', 'preview', 'open-window'].filter((action) =>
        [...document.querySelectorAll('.videoSetupPanel .videoOutputActionDock button[data-action]')].some((button) => {
          const rect = button.getBoundingClientRect();
          return button.dataset.action === action &&
            rect.left >= 0 && rect.right <= innerWidth + 1 && rect.top >= 0 && rect.bottom <= innerHeight + 1;
        })
      ).length,
      visibleVideoSetupDisplayActionCount: visibleCount('.videoSetupPanel .videoOutputActionDock button[data-action="open-window"]'),
      visibleProfileLoadPanelCount: visibleCount('.setupMode-library .profileLoadPanel'),
      visibleLoadedProfileSummaryPanelCount: visibleCount('.setupMode-library .loadedProfileSummaryPanel'),
      profileLoadPanelWidth: profileLoadPanelRect ? Math.round(profileLoadPanelRect.width) : 0,
      loadedProfileSummaryPanelWidth: loadedProfileSummaryPanelRect ? Math.round(loadedProfileSummaryPanelRect.width) : 0,
      visibleCustomProfileWorkbenchCount: visibleCount('.setupMode-profiles .customProfileWorkbench'),
      customProfileAttributePaneWidth: customProfileAttributePaneRect ? Math.round(customProfileAttributePaneRect.width) : 0,
      customProfilePreviewDeskWidth: customProfilePreviewDeskRect ? Math.round(customProfilePreviewDeskRect.width) : 0,
      visibleCustomProfileActionCount: visibleCount('.setupMode-profiles .customProfileActions button'),
      visibleCustomProfileDmxMapCount: visibleCount('.setupMode-profiles .customProfileDmxMap'),
      visibleDmxOutputConfigPanelCount: visibleCount('.setupMode-dmx .dmxOutputConfigPanel'),
      visibleArtRdmPanelCount: visibleCount('.setupMode-dmx .artRdmPanel'),
      visibleOutputDiagnosticsDeskCount: visibleCount('.setupMode-dmx .outputDiagnosticsDesk'),
      visibleLightingRuntimeDeskCount: visibleCount('.setupMode-dmx .lightingRuntimeDesk'),
      dmxRouteItemCount: document.querySelectorAll('.setupMode-dmx .dmxRoutes .timelineItem').length,
      visibleSerialPortIdentityCount: visibleCount('.setupMode-dmx .serialPortIdentity'),
      visibleSerialProtocolRecommendationCount: visibleCount('.setupMode-dmx .serialPortIdentity button'),
      dmxOutputConfigPanelWidth: dmxOutputConfigPanelRect ? Math.round(dmxOutputConfigPanelRect.width) : 0,
      outputDiagnosticsDeskWidth: outputDiagnosticsDeskRect ? Math.round(outputDiagnosticsDeskRect.width) : 0,
      lightingRuntimeDeskWidth: lightingRuntimeDeskRect ? Math.round(lightingRuntimeDeskRect.width) : 0,
      dmxEndpointShrunkenChildCount: shrunkenDirectChildCount('.setupMode-dmx .dmxEndpointDesk'),
      dmxEndpointChildOverlapCount: directChildOverlapCount('.setupMode-dmx .dmxEndpointDesk'),
      midiMappingEditorDeskWidth: midiMappingEditorDeskRect ? Math.round(midiMappingEditorDeskRect.width) : 0,
      midiMappingListDeskWidth: midiMappingListDeskRect ? Math.round(midiMappingListDeskRect.width) : 0,
      oscMappingEditorDeskWidth: oscMappingEditorDeskRect ? Math.round(oscMappingEditorDeskRect.width) : 0,
      oscMappingListDeskWidth: oscMappingListDeskRect ? Math.round(oscMappingListDeskRect.width) : 0,
      remoteServerDeskWidth: remoteServerDeskRect ? Math.round(remoteServerDeskRect.width) : 0,
      remoteEndpointDeskWidth: remoteEndpointDeskRect ? Math.round(remoteEndpointDeskRect.width) : 0,
      visibleStandbySyncDeskCount: visibleCount('.setupMode-remote .standbySyncDesk'),
      visibleStandbyRoleOptionCount: document.querySelectorAll('.setupMode-remote .standbySyncDesk select option').length,
      visibleStandbyActionButtonCount: visibleCount('.setupMode-remote .standbySyncDesk > .buttonRow button'),
      visibleDmxGridSummaryCount: visibleCount('.dmxGridSummary'),
      visibleFixtureSetupEditorCount: visibleCount('.fixtureSetupEditor'),
      visibleUseProfileForPatchButtonCount: [...document.querySelectorAll('.fixtureSetupEditor button')]
        .filter((button) => (button.textContent || '').trim().toLowerCase() === 'use profile for patch').length,
      visibleDuplicateFixtureButtonCount: [...document.querySelectorAll('.fixtureSetupEditor button')]
        .filter((button) => (button.textContent || '').trim().toLowerCase() === 'duplicate fixture').length,
      controlModeTabCount: document.querySelectorAll('.controlModeTabs button').length,
      visibleLiveControlPanelCount: visibleCount('.liveControlPanel'),
      liveControlPanelHeight: Math.round(document.querySelector('.liveControlPanel')?.getBoundingClientRect().height ?? 0),
      visibleLiveFadeMeterCount: visibleCount('.liveControlPanel > .liveFadeMeter'),
      liveFadeMeterGridRow: getComputedStyle(document.querySelector('.liveControlPanel > .liveFadeMeter') ?? document.body).gridRowStart,
      liveMasterGridRow: getComputedStyle(document.querySelector('.liveControlPanel > .liveMasterGrid') ?? document.body).gridRowStart,
      liveFadeAboveMaster: (() => {
        const fade = document.querySelector('.liveControlPanel > .liveFadeMeter')?.getBoundingClientRect();
        const master = document.querySelector('.liveControlPanel > .liveMasterGrid')?.getBoundingClientRect();
        return Boolean(fade && master && fade.bottom <= master.top + 1);
      })(),
      visibleControlStagePanelCount: visibleCount('.controlStagePanel'),
      visibleControlStageCount: visibleCount('.controlStage'),
      controlStageWidth: Math.round(document.querySelector('.controlStage')?.getBoundingClientRect().width ?? 0),
      controlStageHeight: Math.round(document.querySelector('.controlStage')?.getBoundingClientRect().height ?? 0),
      controlStageViewBoxAspect: (() => {
        const values = (document.querySelector('.controlStage')?.getAttribute('viewBox') || '')
          .trim()
          .split(/\\s+/)
          .map(Number);
        return values.length === 4 && values[3] > 0 ? values[2] / values[3] : 0;
      })(),
      controlStageGridCoverage: (() => {
        const stageRect = document.querySelector('.controlStage')?.getBoundingClientRect();
        const gridRect = document.querySelector('.controlStage .stageGrid')?.getBoundingClientRect();
        if (!stageRect || !gridRect || stageRect.width <= 0 || stageRect.height <= 0) return 0;
        return Math.min(gridRect.width / stageRect.width, gridRect.height / stageRect.height);
      })(),
      controlStageFixtureMinSize: Math.min(
        ...[...document.querySelectorAll('.controlStage .stageFixture')].map((fixture) => {
          const rect = fixture.getBoundingClientRect();
          return Math.min(rect.width, rect.height);
        }),
      ),
      visibleControlStageReferenceLabelCount: visibleCount('.controlStage .controlStageObject text, .controlStage .stageVideoSurface2d text'),
      controlWorkSurfaceOverflowCount: [...document.querySelectorAll(
        '.layoutControl .faders, .layoutControl .videoControlPanel, .layoutControl .videoOutputControlList, .layoutControl .videoLayerList'
      )]
        .filter((element) => {
          const rect = element.getBoundingClientRect();
          const style = window.getComputedStyle(element);
          return rect.width > 0 && rect.height > 0 && style.display !== 'none' && style.visibility !== 'hidden';
        })
        .filter((element) => element.scrollHeight > element.clientHeight + 1 || element.scrollWidth > element.clientWidth + 1)
        .length,
      controlWorkSurfaceUnsafeOverflowCount: [...document.querySelectorAll(
        '.layoutControl .faders, .layoutControl .videoControlPanel, .layoutControl .videoOutputControlList, .layoutControl .videoLayerList'
      )]
        .filter((element) => {
          const rect = element.getBoundingClientRect();
          const style = window.getComputedStyle(element);
          return rect.width > 0 && rect.height > 0 && style.display !== 'none' && style.visibility !== 'hidden';
        })
        .filter((element) => {
          const style = window.getComputedStyle(element);
          const verticalOverflow = element.scrollHeight > element.clientHeight + 1 && !['auto', 'scroll'].includes(style.overflowY);
          const horizontalOverflow = element.scrollWidth > element.clientWidth + 1 && !['auto', 'scroll'].includes(style.overflowX);
          return verticalOverflow || horizontalOverflow;
        })
        .length,
      controlWorkSurfaceOverflows: [...document.querySelectorAll(
        '.layoutControl .faders, .layoutControl .videoControlPanel, .layoutControl .videoOutputControlList, .layoutControl .videoLayerList'
      )]
        .filter((element) => {
          const rect = element.getBoundingClientRect();
          const style = window.getComputedStyle(element);
          return rect.width > 0 && rect.height > 0 && style.display !== 'none' && style.visibility !== 'hidden' &&
            (element.scrollHeight > element.clientHeight + 1 || element.scrollWidth > element.clientWidth + 1);
        })
        .map((element) => ({
          className: element.className,
          client: [element.clientWidth, element.clientHeight],
          scroll: [element.scrollWidth, element.scrollHeight],
        })),
      effectTargetValue: (() => {
        const select = [...document.querySelectorAll('.effectEditor select')]
          .find((candidate) => [...candidate.options].some((option) => option.value === 'selection'));
        return select ? select.value : '';
      })(),
      effectTypeValue: (() => {
        const select = [...document.querySelectorAll('.effectEditor select')]
          .find((candidate) => [...candidate.options].some((option) => option.value === 'PositionWave'));
        return select ? select.value : '';
      })(),
      effectCommonAttributeValue: (() => {
        const attributeLabel = [...document.querySelectorAll('.effectForm > label')]
          .find((candidate) => [...candidate.childNodes]
            .some((node) => node.nodeType === Node.TEXT_NODE && (node.textContent || '').trim() === 'Attribute'));
        const select = attributeLabel?.querySelector('select');
        return select ? select.value : '';
      })(),
      visibleFixtureEditSurfaceCount: visibleCount('.fixtureEditSurface'),
      visibleGroupControlBannerCount: visibleCount('.groupControlBanner'),
      visibleCuePanelCount: visibleCount('.cuePanel'),
      visibleCueLivePanelCount: visibleCount('.cuePanelLive'),
      visibleCueFormCount: visibleCount('.cueForm'),
      visibleCueEditOnlyCount: visibleCount('.cueEditOnly'),
      visibleCueLiveGoCount: visibleCount('.cueLiveGo'),
      visibleCueEffectRecallEditorCount: visibleCount('.cueEffectRecallEditor'),
      visibleCueEditToggleCount: visibleCount('.cuePanelEditToggle'),
      cueEditToggleExpanded: cueEditToggle?.getAttribute('aria-expanded') ?? '',
      cueEditToggleControls: cueEditToggle?.getAttribute('aria-controls') ?? '',
      cueEditToggleMinTargetSize: cueEditToggle
        ? Math.round(Math.min(cueEditToggle.getBoundingClientRect().width, cueEditToggle.getBoundingClientRect().height))
        : 0,
      cuePanelOverflowY: cuePanel ? window.getComputedStyle(cuePanel).overflowY : '',
      cuePanelVerticalOverflowPx: cuePanel ? Math.max(0, cuePanel.scrollHeight - cuePanel.clientHeight) : 0,
      cuePanelHorizontalOverflowPx: cuePanel ? Math.max(0, cuePanel.scrollWidth - cuePanel.clientWidth) : 0,
      cueHostHorizontalOverflowPx: cueHost ? Math.max(0, cueHost.scrollWidth - cueHost.clientWidth) : 0,
      cueHostVerticalOverflowPx: cueHost ? Math.max(0, cueHost.scrollHeight - cueHost.clientHeight) : 0,
      cueCaptureScopeValue: document.querySelector('#cue-store-form select')?.value ?? '',
      cuePreviewScopeLabel: (document.querySelector('.cuePreviewHeader strong')?.textContent || '').trim(),
      cuePreviewStatValues: [...document.querySelectorAll('.cuePreviewStats > span > strong')]
        .map((node) => Number((node.textContent || '').trim())),
      cueStoreButtonDisabled: Boolean(document.querySelector('#cue-store-form button.primary')?.disabled),
      visibleCueScopeErrorCount: visibleCount('.cueScopeHint.invalid'),
      visiblePlaybackDeskSurfaceCount: visibleCount('.playbackDeskSurface'),
      visibleProgrammerPanelCount: visibleCount('.programmerPanel'),
      visibleReferencePalettePanelCount: visibleCount('.referencePalettePanel'),
      visiblePlaybackExecutorPanelCount: visibleCount('.playbackExecutorPanel'),
      visiblePlaybackMasterCount: visibleCount('.playbackMasterControl input[type="range"]'),
      visibleTimelinePanelCount: visibleCount('.timelinePanel'),
      visibleTimelineDeskTabCount: visibleCount('.timelineDeskTabs button'),
      visibleTimelineShowSurfaceCount: visibleCount('.timelineShowSurface'),
      visibleTimelineAutomationSurfaceCount: visibleCount('.timelineAutomationSurface'),
      timelineTimeStatClipped: (() => {
        const container = document.querySelector('.timelineTimeStat');
        const stat = document.querySelector('.timelineTimeStat strong');
        return Boolean(
          (container && container.scrollWidth > container.clientWidth + 1) ||
          (stat && stat.scrollWidth > stat.clientWidth + 1)
        );
      })(),
      timelineTimeStatTitle: document.querySelector('.timelineTimeStat')?.getAttribute('title') ?? '',
      visibleSceneBlockWorkspaceCount: visibleCount('.sceneBlockWorkspace'),
      sceneBlockRowCount: document.querySelectorAll('.sceneBlockRow').length,
      visibleSceneBlockRowCount: visibleCount('.sceneBlockRow'),
      timelineSceneBlockCount: document.querySelectorAll('.timelineOverview .timelineMarker.sceneBlock').length,
      timelinePointEventCount: document.querySelectorAll('.timelineOverview .timelineMarker.pointEvent').length,
      timelineActiveSceneBlockCount: document.querySelector('.timelineHeaderMeta.executingLive')
        ? document.querySelectorAll('.timelineOverview .timelineMarker.sceneBlock.underPlayhead').length : 0,
      timelineActivePointEventCount: document.querySelector('.timelineHeaderMeta.executingLive')
        ? document.querySelectorAll('.timelineOverview .timelineMarker.pointEvent.underPlayhead').length : 0,
      sceneBlockLaneScopeHintCount: [...document.querySelectorAll('.sceneBlockLaneScopeHint')]
        .filter((node) => (node.textContent || '').trim() === 'FULL CUE' && (node.title || '').includes('full linked Cue fires')).length,
      sceneBlockPlaybackJumpHintCount: [...document.querySelectorAll('.sceneBlockPlaybackJumpHint')]
        .filter((node) => (node.textContent || '').trim() === 'PLAY ONLY' && (node.title || '').includes('MTC/LTC')).length,
      visibleSceneBlockLinkBadgeCount: visibleCount('.sceneBlockLinkBadge'),
      visibleSceneBlockComposerControlCount: visibleCount('.sceneBlockComposer input, .sceneBlockComposer select, .sceneBlockComposer button'),
      fullyVisibleSceneBlockComposerControlCount: fullyVisibleCount(
        '.sceneBlockComposer input, .sceneBlockComposer select, .sceneBlockComposer button',
        '.sceneBlockWorkspace',
      ),
      visibleSceneBlockRowActionCount: visibleCount('.sceneBlockRowActions button'),
      fullyVisibleSceneBlockRowActionCount: fullyVisibleCount(
        '.sceneBlockRowActions button',
        '.sceneBlockWorkspace',
      ),
      sceneBlockWorkspaceWidth: sceneBlockWorkspaceRect ? Math.round(sceneBlockWorkspaceRect.width) : 0,
      sceneBlockWorkspaceHeight: sceneBlockWorkspaceRect ? Math.round(sceneBlockWorkspaceRect.height) : 0,
      timelinePanelWidth: timelinePanelRect ? Math.round(timelinePanelRect.width) : 0,
      sceneBlockWorkspaceHorizontalOverflowPx: sceneBlockWorkspace
        ? Math.max(0, sceneBlockWorkspace.scrollWidth - sceneBlockWorkspace.clientWidth)
        : 0,
      sceneBlockWorkspaceVerticalOverflowPx: sceneBlockWorkspace
        ? Math.max(0, sceneBlockWorkspace.scrollHeight - sceneBlockWorkspace.clientHeight)
        : 0,
      sceneBlockLastControlReachable: lastControlReachableWhenScrolled(
        '.sceneBlockWorkspace',
        '.sceneBlockComposer input, .sceneBlockComposer select, .sceneBlockComposer button, .sceneBlockRow input, .sceneBlockRow select, .sceneBlockRow button',
      ),
      sceneBlockListHorizontalOverflowPx: sceneBlockList
        ? Math.max(0, sceneBlockList.scrollWidth - sceneBlockList.clientWidth)
        : 0,
      sceneBlockComposerHorizontalOverflowPx: sceneBlockComposer
        ? Math.max(0, sceneBlockComposer.scrollWidth - sceneBlockComposer.clientWidth)
        : 0,
      visibleEditDeskTabCount: visibleCount('.editDeskTabs button'),
      visibleEffectEditorCount: visibleCount('.effectEditor'),
      visibleEffectWorkbenchCount: visibleCount('.effectWorkbench'),
      visibleEffectLibraryPaneCount: visibleCount('.effectLibraryPane'),
      visibleEffectInspectorPaneCount: visibleCount('.effectInspectorPane'),
      visibleEffectRackPaneCount: visibleCount('.effectRackPane'),
      visibleEffectRecipeDockCount: visibleCount('.effectRecipeDock'),
      visibleEffectActionDockCount: visibleCount('.effectActionDock'),
      visibleEffectRackTabCount: visibleCount('.effectRackTabs button'),
      visibleEffectFamilyButtonCount: visibleCount('.effectFamilyRail button'),
      visibleActiveEffectFamilyButtonCount: visibleCount('.effectFamilyRail button.active[aria-pressed="true"]'),
      visibleEffectLibraryCardCount: visibleCount('.sampleEffectPresetCard'),
      visibleTargetRequiredEffectCardCount: visibleCount('.sampleEffectPresetCard[data-requires-target="true"]'),
      visibleColorEffectEditorCount: visibleCount('.colorEffectEditor'),
      visibleColorEffectStopCount: visibleCount('.colorEffectStopRow'),
      visibleColorEffectAddButtonCount: [...document.querySelectorAll('.colorEffectPaletteFooter button')]
        .filter((button) => {
          const rect = button.getBoundingClientRect();
          const style = window.getComputedStyle(button);
          return rect.width > 0 && rect.height > 0 && style.display !== 'none' && style.visibility !== 'hidden' &&
            (button.textContent || '').trim().toLowerCase() === 'add stop';
        }).length,
      visibleColorEffectRemoveButtonCount: visibleCount('.colorEffectStopAction.remove'),
      visibleColorEffectGradientCount: visibleCount('.colorEffectGradientPreview'),
      colorEffectGradientRenderedCount: [...document.querySelectorAll('.colorEffectGradientPreview')]
        .filter((element) => {
          const rect = element.getBoundingClientRect();
          const style = window.getComputedStyle(element);
          return rect.width > 0 && rect.height > 0 && style.display !== 'none' && style.visibility !== 'hidden' &&
            style.backgroundImage.includes('linear-gradient');
        }).length,
      visibleColorEffectAlgorithmSelectCount: [...document.querySelectorAll('.colorEffectModeGrid select')]
        .filter((select) => {
          const rect = select.getBoundingClientRect();
          const style = window.getComputedStyle(select);
          const optionValues = [...select.options].map((option) => option.value);
          return rect.width > 0 && rect.height > 0 && style.display !== 'none' && style.visibility !== 'hidden' &&
            ['Cycle', 'Bounce', 'Sequence', 'Random'].every((value) => optionValues.includes(value));
        }).length,
      colorEffectAlgorithmValue: (() => {
        const select = [...document.querySelectorAll('.colorEffectModeGrid select')]
          .find((candidate) => [...candidate.options].some((option) => option.value === 'Cycle'));
        return select ? select.value : '';
      })(),
      visibleColorEffectInterpolationSelectCount: [...document.querySelectorAll('.colorEffectModeGrid select')]
        .filter((select) => {
          const rect = select.getBoundingClientRect();
          const style = window.getComputedStyle(select);
          const optionValues = [...select.options].map((option) => option.value);
          return rect.width > 0 && rect.height > 0 && style.display !== 'none' && style.visibility !== 'hidden' &&
            ['Rgb', 'HsvShortest', 'HsvLongest'].every((value) => optionValues.includes(value));
        }).length,
      colorEffectInterpolationValue: (() => {
        const select = [...document.querySelectorAll('.colorEffectModeGrid select')]
          .find((candidate) => [...candidate.options].some((option) => option.value === 'HsvShortest'));
        return select ? select.value : '';
      })(),
      visibleLegacyEffectAttributeCount: [...document.querySelectorAll('.effectForm > label')]
        .filter((label) => {
          const rect = label.getBoundingClientRect();
          const style = window.getComputedStyle(label);
          return rect.width > 0 && rect.height > 0 && style.display !== 'none' && style.visibility !== 'hidden' &&
            [...label.childNodes].some((node) => node.nodeType === Node.TEXT_NODE && (node.textContent || '').trim() === 'Attribute');
        }).length,
      visibleLegacyEffectWaveformCount: visibleCount('.effectShapePresetPanel') +
        visibleCount('.effectBpmPeriodPanel') + visibleCount('.waveStagePicker'),
      visibleLegacyEffectVideoTargetCount: visibleCount('.videoEffectTarget') + visibleCount('.videoTargetPositionPanel'),
      colorEffectDraftSummaryIsCurrent: (() => {
        const text = document.querySelector('.effectTargetHint')?.textContent || '';
        return text.includes('whole-fixture colour') && text.includes('Color Random / 3 stops') && !text.includes('LFO ');
      })(),
      colorEffectStatusIsCurrent: (() => {
        const text = document.querySelector('.appStatusText')?.textContent || '';
        return text.includes('Prepared a multi-color draft') && !text.includes('position wave draft');
      })(),
      colorEffectDisabledVideoOptionCount: [...document.querySelectorAll('.effectEditor select')]
        .filter((select) => [...select.options].some((option) => option.value === 'selection'))
        .flatMap((select) => [...select.options])
        .filter((option) => option.value === 'video' && option.disabled).length,
      colorEffectUnlabeledActionCount: [...document.querySelectorAll('.colorEffectStopAction')]
        .filter((button) => {
          const rect = button.getBoundingClientRect();
          const style = window.getComputedStyle(button);
          return rect.width > 0 && rect.height > 0 && style.display !== 'none' && style.visibility !== 'hidden' &&
            !(button.getAttribute('aria-label') || '').trim();
        }).length,
      colorEffectHorizontalOverflowPx: (() => {
        const editor = document.querySelector('.colorEffectEditor');
        return editor ? Math.max(0, editor.scrollWidth - editor.clientWidth) : -1;
      })(),
      colorEffectEditorContained: (() => {
        const editor = document.querySelector('.colorEffectEditor');
        const form = document.querySelector('.effectInspectorPane > .effectForm');
        if (!editor || !form) return false;
        const editorRect = editor.getBoundingClientRect();
        const formRect = form.getBoundingClientRect();
        return editorRect.left >= formRect.left - 1 && editorRect.right <= formRect.right + 1;
      })(),
      visibleChaserEditorCount: visibleCount('.chaserEffectEditor'),
      visibleChaserStepCount: visibleCount('.chaserStepRow'),
      visibleChaserFeatureCount: visibleCount('.chaserFeatureRow'),
      visibleChaserPreviewCellCount: visibleCount('.chaserPreviewCell'),
      visibleActiveChaserPreviewCellCount: visibleCount('.chaserPreviewCell.active'),
      visibleChaserDirectionButtonCount: visibleCount('.chaserDirectionGrid button'),
      visibleChaserReplaceButtonCount: visibleCount('.chaserStepRow button[title="Replace with current target"]'),
      chaserDirectionValue: document.querySelector('.chaserPreviewStrip')?.dataset.direction ?? '',
      chaserActiveStepCount: Number(document.querySelector('.chaserPreviewStrip')?.dataset.activeStepCount ?? 0),
      chaserSizePercent: Number(document.querySelector('.chaserPreviewStrip')?.dataset.sizePercent ?? -1),
      chaserPhasePercent: Number(document.querySelector('.chaserPreviewStrip')?.dataset.phasePercent ?? -1),
      chaserFadingEnabled: Boolean(document.querySelector('.chaserFadingToggle input')?.checked),
      chaserDisabledVideoOptionCount: [...document.querySelectorAll('.effectEditor select')]
        .filter((select) => [...select.options].some((option) => option.value === 'selection'))
        .flatMap((select) => [...select.options])
        .filter((option) => option.value === 'video' && option.disabled).length,
      chaserUnlabeledActionCount: [...document.querySelectorAll('.chaserEffectEditor button')]
        .filter((button) => {
          const rect = button.getBoundingClientRect();
          const style = window.getComputedStyle(button);
          const visible = rect.width > 0 && rect.height > 0 && style.display !== 'none' && style.visibility !== 'hidden';
          const hasText = (button.textContent || '').trim().length > 0;
          return visible && !hasText && !(button.getAttribute('aria-label') || '').trim();
        }).length,
      chaserHorizontalOverflowPx: (() => {
        const editor = document.querySelector('.chaserEffectEditor');
        return editor ? Math.max(0, editor.scrollWidth - editor.clientWidth) : -1;
      })(),
      chaserEditorContained: (() => {
        const editor = document.querySelector('.chaserEffectEditor');
        const form = document.querySelector('.effectInspectorPane > .effectForm');
        if (!editor || !form) return false;
        const editorRect = editor.getBoundingClientRect();
        const formRect = form.getBoundingClientRect();
        return editorRect.left >= formRect.left - 1 && editorRect.right <= formRect.right + 1;
      })(),
      visibleMoveEffectEditorCount: visibleCount('.moveEffectEditorPanel'),
      visibleMoveEffectPathDeskCount: visibleCount('.moveEffectPathDesk'),
      visibleMoveEffectInspectorCount: visibleCount('.moveEffectInspector'),
      visibleMoveEffectPathCanvasCount: visibleCount('.moveEffectPathCanvas'),
      visibleMoveEffectDirectionButtonCount: visibleCount('.moveEffectDirectionGrid button'),
      visibleMoveEffectTransformInputCount: visibleCount('.moveEffectTransformGrid input'),
      visibleMoveEffectCoordinateButtonCount: visibleCount('.moveEffectSegmented[aria-label="Move coordinate mode"] button'),
      visibleMoveEffectInterpolationButtonCount: visibleCount('.moveEffectSegmented[aria-label="Move interpolation"] button'),
      visibleMoveEffectClosedToggleCount: visibleCount('.moveEffectClosedToggle input[type="checkbox"]'),
      visibleMoveEffectPointRowCount: visibleCount('.moveEffectPointRow'),
      moveEffectEditorWidth: moveEffectEditorPanelRect ? Math.round(moveEffectEditorPanelRect.width) : 0,
      moveEffectEditorHeight: moveEffectEditorPanelRect ? Math.round(moveEffectEditorPanelRect.height) : 0,
      moveEffectSurfaceWidth: moveEffectEditorSurfaceRect ? Math.round(moveEffectEditorSurfaceRect.width) : 0,
      moveEffectSurfaceHeight: moveEffectEditorSurfaceRect ? Math.round(moveEffectEditorSurfaceRect.height) : 0,
      moveEffectPathDeskWidth: moveEffectPathDeskRect ? Math.round(moveEffectPathDeskRect.width) : 0,
      moveEffectInspectorWidth: moveEffectInspectorRect ? Math.round(moveEffectInspectorRect.width) : 0,
      moveEffectPathCanvasWidth: moveEffectPathCanvasRect ? Math.round(moveEffectPathCanvasRect.width) : 0,
      moveEffectPathCanvasHeight: moveEffectPathCanvasRect ? Math.round(moveEffectPathCanvasRect.height) : 0,
      moveEffectEditorHorizontalOverflowPx: moveEffectEditorPanel
        ? Math.max(0, moveEffectEditorPanel.scrollWidth - moveEffectEditorPanel.clientWidth)
        : -1,
      moveEffectSurfaceHorizontalOverflowPx: moveEffectEditorSurface
        ? Math.max(0, moveEffectEditorSurface.scrollWidth - moveEffectEditorSurface.clientWidth)
        : -1,
      moveEffectPathDeskHorizontalOverflowPx: moveEffectPathDesk
        ? Math.max(0, moveEffectPathDesk.scrollWidth - moveEffectPathDesk.clientWidth)
        : -1,
      moveEffectInspectorHorizontalOverflowPx: moveEffectInspector
        ? Math.max(0, moveEffectInspector.scrollWidth - moveEffectInspector.clientWidth)
        : -1,
      moveEffectFormHorizontalOverflowPx: moveEffectForm
        ? Math.max(0, moveEffectForm.scrollWidth - moveEffectForm.clientWidth)
        : -1,
      moveEffectFormVerticalOverflowPx: moveEffectForm
        ? Math.max(0, moveEffectForm.scrollHeight - moveEffectForm.clientHeight)
        : -1,
      moveEffectFormOverflowY: moveEffectForm ? window.getComputedStyle(moveEffectForm).overflowY : '',
      moveEffectPointListVerticalOverflowPx: moveEffectPointList
        ? Math.max(0, moveEffectPointList.scrollHeight - moveEffectPointList.clientHeight)
        : -1,
      moveEffectPointListOverflowY: moveEffectPointList ? window.getComputedStyle(moveEffectPointList).overflowY : '',
      moveEffectLastControlReachable,
      moveEffectLastPointReachable,
      moveEffectEditorContained: Boolean(
        moveEffectEditorPanelRect &&
        moveEffectFormRect &&
        moveEffectEditorPanelRect.left >= moveEffectFormRect.left - 1 &&
        moveEffectEditorPanelRect.right <= moveEffectFormRect.right + 1
      ),
      moveEffectCanvasContained: Boolean(
        moveEffectPathCanvasRect &&
        moveEffectPathDeskRect &&
        moveEffectPathCanvasRect.left >= moveEffectPathDeskRect.left - 1 &&
        moveEffectPathCanvasRect.right <= moveEffectPathDeskRect.right + 1 &&
        moveEffectPathCanvasRect.top >= moveEffectPathDeskRect.top - 1 &&
        moveEffectPathCanvasRect.bottom <= moveEffectPathDeskRect.bottom + 1
      ),
      moveEffectColumnsSideBySide: Boolean(
        moveEffectPathDeskRect &&
        moveEffectInspectorRect &&
        moveEffectInspectorRect.left >= moveEffectPathDeskRect.right - 1 &&
        Math.abs(moveEffectPathDeskRect.top - moveEffectInspectorRect.top) <= 2
      ),
      moveEffectSurfaceWidthCoverage: moveEffectEditorPanelRect && moveEffectEditorSurfaceRect
        ? Number((moveEffectEditorSurfaceRect.width / Math.max(1, moveEffectEditorPanelRect.width)).toFixed(3))
        : 0,
      moveEffectColumnAreaCoverage: moveEffectEditorSurfaceRect && moveEffectPathDeskRect && moveEffectInspectorRect
        ? Number((
            (moveEffectPathDeskRect.width * moveEffectPathDeskRect.height + moveEffectInspectorRect.width * moveEffectInspectorRect.height) /
            Math.max(1, moveEffectEditorSurfaceRect.width * moveEffectEditorSurfaceRect.height)
          ).toFixed(3))
        : 0,
      moveEffectUnusedRightPx: moveEffectEditorSurfaceRect && moveEffectPathDeskRect && moveEffectInspectorRect
        ? Math.round(Math.max(0, moveEffectEditorSurfaceRect.right - Math.max(moveEffectPathDeskRect.right, moveEffectInspectorRect.right)))
        : -1,
      moveEffectUnusedBottomPx: moveEffectEditorSurfaceRect && moveEffectPathDeskRect && moveEffectInspectorRect
        ? Math.round(Math.max(0, moveEffectEditorSurfaceRect.bottom - Math.max(moveEffectPathDeskRect.bottom, moveEffectInspectorRect.bottom)))
        : -1,
      visibleNodeGraphPanelCount: visibleCount('.nodeGraphPanel'),
      nodeGraphAudioSourceOptionCount: [...document.querySelectorAll('.nodeGraphPanel option')]
        .filter((option) => (option.textContent || '').trim().toLowerCase() === 'audio fft').length,
      nodeGraphLiveAudioOptionCount: [...document.querySelectorAll('.nodeGraphPanel option')]
        .filter((option) => (option.textContent || '').trim().toLowerCase() === 'live input').length,
      visiblePanTiltPadCount: visibleCount('.panTiltPad'),
      visibleColorPlaneCount: visibleCount('.colorPlane'),
      visiblePositionReadoutCount: visibleCount('.positionReadoutStrip'),
      visibleColorReadoutCount: visibleCount('.colorReadoutStrip'),
      visiblePositionConsoleCount: visibleCount('.positionConsoleSurface'),
      visiblePositionToolDeckCount: visibleCount('.positionToolDeck'),
      visiblePositionToolTabCount: visibleCount('.positionToolTabs button'),
      visibleActivePositionToolTabCount: visibleCount('.positionToolTabs button.active[aria-selected="true"]'),
      visiblePositionDirectControlCount: visibleCount('.positionDirectGrid input, .positionDirectGrid select'),
      visiblePositionNudgeButtonCount: visibleCount('.positionNudgeGrid button'),
      visiblePositionTargetButtonCount: visibleCount('.positionTargetGrid button'),
      visiblePositionTransformButtonCount: visibleCount('.positionTransformRow button'),
      visiblePositionFavoriteButtonCount: visibleCount('.positionFavoriteGrid .positionFavorite'),
      visiblePositionAxisSliderCount: visibleCount('.axisSliderRack input[type="range"]'),
      visibleControlLimitPanelCount: visibleCount('.positionToolPane .controlLimitPanel'),
      fullyVisiblePositionDirectControlCount: fullyVisibleCount(
        '.positionDirectGrid input, .positionDirectGrid select',
        '.positionToolPane',
      ),
      fullyVisiblePositionNudgeButtonCount: fullyVisibleCount('.positionNudgeGrid button', '.positionToolPane'),
      fullyVisiblePositionTargetButtonCount: fullyVisibleCount('.positionTargetGrid button', '.positionToolPane'),
      fullyVisiblePositionTransformButtonCount: fullyVisibleCount('.positionTransformRow button', '.positionToolPane'),
      fullyVisiblePositionFavoriteButtonCount: fullyVisibleCount(
        '.positionFavoriteGrid .positionFavorite',
        '.positionToolPane',
      ),
      positionConsoleWidth: Math.round(document.querySelector('.positionConsoleSurface')?.getBoundingClientRect().width ?? 0),
      positionConsoleHeight: Math.round(document.querySelector('.positionConsoleSurface')?.getBoundingClientRect().height ?? 0),
      positionPadWidth: Math.round(document.querySelector('.panTiltPad')?.getBoundingClientRect().width ?? 0),
      positionPadHeight: Math.round(document.querySelector('.panTiltPad')?.getBoundingClientRect().height ?? 0),
      positionToolDeckWidth: Math.round(document.querySelector('.positionToolDeck')?.getBoundingClientRect().width ?? 0),
      positionToolPaneHeight: Math.round(document.querySelector('.positionToolPane')?.getBoundingClientRect().height ?? 0),
      positionConsoleHorizontalOverflowPx: (() => {
        const surface = document.querySelector('.positionConsoleSurface');
        return surface ? Math.max(0, surface.scrollWidth - surface.clientWidth) : -1;
      })(),
      positionToolPaneHorizontalOverflowPx: (() => {
        const pane = document.querySelector('.positionToolPane');
        return pane ? Math.max(0, pane.scrollWidth - pane.clientWidth) : -1;
      })(),
      positionToolPaneVerticalOverflowPx: (() => {
        const pane = document.querySelector('.positionToolPane');
        return pane ? Math.max(0, pane.scrollHeight - pane.clientHeight) : -1;
      })(),
      positionToolPaneOverflowY: (() => {
        const pane = document.querySelector('.positionToolPane');
        return pane ? window.getComputedStyle(pane).overflowY : '';
      })(),
      positionToolPaneLastControlReachable,
      positionConsoleContained: (() => {
        const surface = document.querySelector('.positionConsoleSurface');
        const panel = surface?.closest('.visualControlPanel');
        if (!surface || !panel) return false;
        const surfaceRect = surface.getBoundingClientRect();
        const panelRect = panel.getBoundingClientRect();
        return surfaceRect.left >= panelRect.left - 1 && surfaceRect.right <= panelRect.right + 1 &&
          surfaceRect.top >= panelRect.top - 1 && surfaceRect.bottom <= panelRect.bottom + 1;
      })(),
      positionPrimaryAndToolsSideBySide: (() => {
        const primary = document.querySelector('.positionPrimaryDeck')?.getBoundingClientRect();
        const tools = document.querySelector('.positionToolDeck')?.getBoundingClientRect();
        return Boolean(primary && tools && tools.left >= primary.right - 1 && Math.abs(primary.top - tools.top) <= 2);
      })(),
      visibleAttributeTargetSummaryCount: visibleCount('.attributeTargetSummary'),
      visibleGroupAttributeTargetSummaryCount: visibleCount('.attributeTargetSummary.group'),
      effectTargetHintCount: visibleCount('.effectTargetHint'),
      effectTargetHintText: [...document.querySelectorAll('.effectTargetHint')]
        .map((element) => (element.textContent || '').trim().replace(/\\s+/g, ' '))
        .join(' | '),
      effectTargetMapSelectionOptionCount: [...document.querySelectorAll('.effectEditor option')]
        .filter((option) => (option.textContent || '').trim().toLowerCase().startsWith('map selection')).length,
      visibleRawMonitorCount: visibleCount('.rawMonitor'),
      visibleVideoControlPanelCount: visibleCount('.videoControlPanel'),
      visibleVideoMixerClipPaneCount: visibleCount('.videoMixerClipPane'),
      visibleVideoMixerProgramPaneCount: visibleCount('.videoMixerProgramPane'),
      visibleVideoMixerLayerPaneCount: visibleCount('.videoMixerLayerPane'),
      visibleVideoMonitorPanelCount: visibleCount('.liveVideoMonitorPanel'),
      visibleVideoPreviewBusCount: visibleCount('[data-live-video-monitor="preview"]'),
      visibleVideoProgramBusCount: visibleCount('[data-live-video-monitor="program"]'),
      visibleVjPreviewTransportCount: visibleCount('.vjPreviewTransport'),
      visibleVjPreviewTransportButtonCount: visibleCount('.vjPreviewTransportControls button'),
      disabledVjPreviewTransportButtonCount: [...document.querySelectorAll('.vjPreviewTransportControls button')]
        .filter((button) => button.disabled).length,
      undersizedVjPreviewTransportButtonCount: [...document.querySelectorAll('.vjPreviewTransportControls button')]
        .filter((button) => {
          const rect = button.getBoundingClientRect();
          return rect.width < 44 || rect.height < 44;
        }).length,
      visibleVjPreviewStageButtonCount: visibleCount('.videoClipPreview'),
      disabledVjPreviewStageButtonCount: [...document.querySelectorAll('.videoClipPreview')]
        .filter((button) => button.disabled).length,
      undersizedVjPreviewStageButtonCount: [...document.querySelectorAll('.videoClipPreview')]
        .filter((button) => {
          const rect = button.getBoundingClientRect();
          return rect.width < 44 || rect.height < 44;
        }).length,
      visibleVjFirstRunCount: visibleCount('.vjFirstRunEmptyState'),
      visibleVjFirstRunButtonCount: visibleCount('.vjFirstRunEmptyState button'),
      fullyVisibleVjFirstRunButtonCount: [...document.querySelectorAll('.vjFirstRunEmptyState button')]
        .filter((element) => {
          const rect = element.getBoundingClientRect();
          return rect.left >= 0 && rect.top >= 0 && rect.right <= window.innerWidth && rect.bottom <= window.innerHeight;
        }).length,
      disabledVjFirstRunButtonCount: [...document.querySelectorAll('.vjFirstRunEmptyState button')]
        .filter((button) => button.disabled).length,
      visibleVjFirstRunSafetyCount: [...document.querySelectorAll('.vjFirstRunSafety')]
        .filter((element) => (element.textContent || '').includes('No output window opens automatically')).length,
      visibleVideoProgramRefreshCount: [...document.querySelectorAll('.videoMixerProgramPane button')]
        .filter((button) => (button.textContent || '').trim().toLowerCase() === 'refresh').length,
      visibleVideoMasterControlCount: visibleCount('.videoMasterControls'),
      visibleVideoMasterFaderCount: visibleCount('.videoMasterFader input[type="range"]'),
      visibleVideoClipGridCount: visibleCount('.videoClipGridPanel'),
      visibleVideoClipPadCount: visibleCount('.videoClipPad'),
      visibleVideoClipTakeButtonCount: [...document.querySelectorAll('.videoClipPreviewBar button')]
        .filter((button) => (button.textContent || '').trim().toLowerCase() === 'take').length,
      visibleVideoClipAudioButtonCount: visibleCount('.videoClipAudio'),
      visibleVideoProgramAudioToggleCount: visibleCount('.videoProgramAudioToggle input[type="checkbox"]'),
      visibleVideoAbDeckCount: visibleCount('.videoAbDeck'),
      visibleVideoDeckLoadButtonCount: visibleCount('.videoClipDeckLoad button'),
      visibleVideoRecordingBarCount: visibleCount('.videoRecordingBar'),
      visibleLiveAudioInputBarCount: visibleCount('.liveAudioInputBar'),
      liveAudioInputBarDomCount: document.querySelectorAll('.liveAudioInputBar').length,
      visibleLiveAudioRailCount: visibleCount('.videoMixerClipPane > .liveAudioInputBar'),
      visibleEmbeddedLiveAudioCount: visibleCount('.videoClipGridPanel > .liveAudioInputBar'),
      liveAudioRailHeight: (() => {
        const rail = document.querySelector('.videoMixerClipPane > .liveAudioInputBar');
        return rail ? rail.getBoundingClientRect().height : 0;
      })(),
      liveAudioRailBelowMaster: (() => {
        const rail = document.querySelector('.videoMixerClipPane > .liveAudioInputBar')?.getBoundingClientRect();
        const master = document.querySelector('.videoMixerClipPane > .videoMasterControls')?.getBoundingClientRect();
        return Boolean(rail && master && rail.top >= master.bottom - 1);
      })(),
      liveAudioRailAboveClipGrid: (() => {
        const rail = document.querySelector('.videoMixerClipPane > .liveAudioInputBar')?.getBoundingClientRect();
        const grid = document.querySelector('.videoMixerClipPane > .videoClipGridPanel')?.getBoundingClientRect();
        return Boolean(rail && grid && rail.bottom <= grid.top + 1);
      })(),
      liveAudioRailOverflowX: (() => {
        const rail = document.querySelector('.videoMixerClipPane > .liveAudioInputBar');
        return rail ? Math.max(0, rail.scrollWidth - rail.clientWidth) : 0;
      })(),
      liveAudioRailOverflowY: (() => {
        const rail = document.querySelector('.videoMixerClipPane > .liveAudioInputBar');
        return rail ? Math.max(0, rail.scrollHeight - rail.clientHeight) : 0;
      })(),
      liveAudioRailMeterCount: visibleCount('.videoMixerClipPane > .liveAudioInputBar [role="meter"]'),
      liveAudioRailPoliteRegionCount: visibleCount('.videoMixerClipPane > .liveAudioInputBar [aria-live="polite"]'),
      fullyVisibleVideoClipPadCount: (() => {
        const grid = document.querySelector('.videoMixerClipPane > .videoClipGridPanel')?.getBoundingClientRect();
        if (!grid) return 0;
        return [...document.querySelectorAll('.videoMixerClipPane .videoClipPad')].filter((element) => {
          const rect = element.getBoundingClientRect();
          return rect.width > 0 && rect.height > 0 && rect.top >= grid.top - 1 && rect.bottom <= grid.bottom + 1;
        }).length;
      })(),
      visibleVideoOutputControlListCount: visibleCount('.videoOutputControlList'),
      visibleVideoOutputItemCount: visibleCount('.videoOutputControlItem'),
      visibleVideoOutputSelectedItemCount: visibleCount('.videoOutputControlItem.selected'),
      visibleVideoMixerOutputDeckCount: visibleCount('.videoMixerOutputDeck'),
      visibleVideoMixerOutputFaderCount: visibleCount('.videoControlPanelMixer .videoMixerOutputDeck input[type="range"]'),
      visibleVideoMixerOutputSelectButtonCount: [...document.querySelectorAll('.videoControlPanelMixer .videoMixerOutputDeck button')]
        .filter((button) => (button.textContent || '').trim().toLowerCase() === 'sel').length,
      visibleVideoLayerListCount: visibleCount('.videoLayerList'),
      visibleVideoLayerItemCount: visibleCount('.videoLayerItem'),
      visibleBuiltinVideoFxSelectCount: visibleCount('.videoIsfPanel select[aria-label="Built-in FX"]'),
      visibleVideoMixerLayerDeckCount: visibleCount('.videoMixerLayerDeck'),
      visibleVideoMixerLayerFaderCount: visibleCount('.videoControlPanelMixer .videoMixerLayerDeck input[type="range"]'),
      visibleVideoMixerLayerButtonCount: visibleCount('.videoControlPanelMixer .videoMixerLayerDeck button'),
      visibleVideoDeckPagerCount: visibleCount('.videoControlPanelMixer .deckPager'),
      visibleVideoMixerDiagnosticsCount: visibleCount('.videoMixerDiagnostics'),
      visibleVideoMixerSetupToolsCount: visibleCount('.videoMixerSetupTools'),
      visibleVideoMixerAutomationToolsCount: visibleCount('.videoMixerAutomationTools'),
      visibleOutputPanelCount: visibleCount('.output.controlPanel'),
      visibleSetupVideoPanelCount: visibleCount('.videoSetupPanel'),
      visibleSetupVideoOutputDeckCount: visibleCount('.videoSetupPanel .videoOutputDeck'),
      visibleSetupVideoOutputActiveDeckCount: visibleCount('.videoSetupPanel .videoOutputDeck.active'),
      visibleSetupVideoOutputDetailPaneCount: visibleCount('.videoSetupPanel .videoOutputDetailPane'),
      visibleVideoOutputMappingPanelCount: visibleCount('.videoSetupPanel .videoOutputMapping'),
      visibleVideoOutputBlendControlsCount: visibleCount('.videoSetupPanel .videoOutputBlendControls'),
      visibleProjectorMapEditorCount: visibleCount('.videoSetupPanel .projectorMapEditor'),
      visibleProjectorMapHandleCount: visibleCount('.videoSetupPanel .projectorMapEditor .projectorMapHandle'),
      visibleProjectorKeystoneHandleCount: visibleCount('.videoSetupPanel .projectorMapEditor .projectorMapKeystoneHandle'),
      visibleProjectorScaleHandleCount: visibleCount('.videoSetupPanel .projectorMapEditor .projectorMapScaleHandle'),
      visibleProjectorRotateHandleCount: visibleCount('.videoSetupPanel .projectorMapEditor .projectorMapRotateHandle'),
      visibleProjectorAspectModeButtonCount: visibleCount('.videoSetupPanel .projectorMapModeRow button'),
      visibleProjectorAspectPresetButtonCount: visibleCount('.videoSetupPanel .projectorMapAspectPresetRow button'),
      visibleProjectorResetPoseButtonCount: [...document.querySelectorAll('.videoSetupPanel button')]
        .filter((button) => (button.textContent || '').trim().toLowerCase() === 'reset pose').length,
      visibleMappingProjectorButtonCount: visibleCount('.mappingSelectionPanel button.projector'),
      visibleMappingProjectorControlsCount: visibleCount('.mappingSelectionPanel .mappingProjectorControls'),
      visibleMappingProjectorWarpGridCount: visibleCount('.mappingSelectionPanel .mappingProjectorWarpGrid'),
      visibleMappingProjectorActionButtonCount: visibleCount('.mappingSelectionPanel .mappingProjectorActionRow button'),
      visibleMappingProjectorResetPoseButtonCount: [...document.querySelectorAll('.mappingSelectionPanel button')]
        .filter((button) => (button.textContent || '').trim().toLowerCase() === 'reset pose').length,
      visibleStageVideoSurfaceCount: visibleCount('.mappingStageViewport .stageVideoSurface2d'),
      visibleTouchCuePanelCount: visibleCount('.touchCuePanel'),
      visibleTouchSafetyDeckCount: visibleCount('.touchSafetyDeck'),
      visibleTouchSafetyGuardButtonCount: visibleCount('.touchSafetyDeck .touchGuardRow button'),
      visibleTouchGoDeckCount: visibleCount('.touchGoDeck'),
      visibleTouchCuePadCount: visibleCount('.touchCuePadGrid .liveCuePad'),
      visibleTouchStagePanelCount: visibleCount('.touchStagePanel'),
      visibleTouchStageCount: visibleCount('.touchStage'),
      visibleTouchFixturePanelCount: visibleCount('.touchFixturePanel'),
      visibleTouchFixtureScrollerCount: visibleCount('.touchFixtureScroller'),
      visibleTouchRemotePanelCount: visibleCount('.touchRemotePanel'),
      visibleTouchRemoteUrlItemCount: visibleCount('.touchRemotePanel .remoteUrlItem'),
      visibleTouchRemoteCopyButtonCount: visibleCount('.touchRemotePanel .remoteUrlItem button'),
      visibleTouchRemoteOpenButtonCount: visibleCount('.touchRemotePanel .remoteUrlOpenButton'),
      visibleTouchVideoPanelCount: visibleCount('.touchVideoPanel'),
      visibleTouchVideoOutputDeckCount: visibleCount('.touchVideoPanel .touchVideoOutputDeck'),
      visibleTouchVideoSelectedOutputDeckCount: visibleCount('.touchVideoPanel .touchVideoOutputDeck.selected'),
      visibleTouchVideoOutputFaderCount: visibleCount('.touchVideoPanel .touchVideoOutputDeck input[type="range"]'),
      visibleTouchVideoOutputButtonCount: visibleCount('.touchVideoPanel .touchVideoOutputDeck button'),
      visibleTouchVideoOutputSelectButtonCount: visibleCount('.touchVideoPanel .touchVideoOutputSelect'),
      visibleTouchVideoDeckCount: visibleCount('.touchVideoPanel .touchVideoDeck'),
      visibleTouchVideoLayerFaderCount: visibleCount('.touchVideoPanel .touchVideoDeck input[type="range"]'),
      visibleTouchMasterGridCount: visibleCount('.touchMasterGrid'),
      touchUndersizedTargetCount: [...document.querySelectorAll('.layoutTouch button, .layoutTouch .buttonLink, .layoutTouch input:not([type="checkbox"]), .layoutTouch select')]
        .filter((element) => {
          const rect = element.getBoundingClientRect();
          const style = window.getComputedStyle(element);
          return rect.width > 0 && rect.height > 0 && style.display !== 'none' && style.visibility !== 'hidden';
        })
        .filter((element) => {
          const rect = element.getBoundingClientRect();
          return rect.width < 47.5 || rect.height < 47.5;
        }).length,
      touchUndersizedTargets: [...document.querySelectorAll('.layoutTouch button, .layoutTouch .buttonLink, .layoutTouch input:not([type="checkbox"]), .layoutTouch select')]
        .filter((element) => {
          const rect = element.getBoundingClientRect();
          const style = window.getComputedStyle(element);
          return rect.width > 0 && rect.height > 0 && style.display !== 'none' && style.visibility !== 'hidden' && (rect.width < 47.5 || rect.height < 47.5);
        })
        .slice(0, 40)
        .map((element) => {
          const rect = element.getBoundingClientRect();
          return element.tagName.toLowerCase() + '.' + (element.className || '-') + ' ' + Math.round(rect.width) + 'x' + Math.round(rect.height) + ' ' + (element.textContent || element.getAttribute('aria-label') || element.getAttribute('type') || '').trim().replace(/\\s+/g, ' ').slice(0, 42);
        }),
      touchMomentaryFlashPassed: window.__syndocalTouchMomentaryCheck?.passed === true,
      touchMomentaryFlashResult: window.__syndocalTouchMomentaryCheck ?? null,
      keyboardNavigationPassed: window.__syndocalKeyboardNavigationCheck?.passed === true,
      keyboardNavigationResult: window.__syndocalKeyboardNavigationCheck ?? null,
      timelineOverviewVisible: (() => {
        const overview = document.querySelector('.timelineOverview');
        if (!overview) {
          return false;
        }
        const rect = overview.getBoundingClientRect();
        return rect.width > 20 && rect.height > 20;
      })(),
      movedX,
      movedY,
      canWindowScrollX: movedX > 0,
      canWindowScrollY: movedY > 0
    };
  })()`);
}

function hasTimelineAutomationVisuals(result) {
  if (
    result.label.startsWith("control-live-cues-") ||
    result.label.startsWith("control-live-automation-") ||
    result.label.startsWith("control-live-playback-")
  ) {
    return true;
  }
  return (
    result.timelineOverviewVisible &&
    result.timelineAutomationRangeCount >= 2 &&
    result.timelineAutomationHandleCount >= 4 &&
    result.timelineAutomationKeyframeCount >= 4 &&
    result.timelineAutomationChipCount >= 4 &&
    result.timelineAutomationCurveCount >= 4 &&
    result.timelineAutomationValueInputCount >= 4 &&
    result.timelineAutomationEnabledToggleCount >= 2 &&
    result.timelineAutomationScopeCount >= 2 &&
    result.timelineAutomationBatchButtonCount >= 4 &&
    result.timelineAutomationGroupButtonCount >= 1 &&
    result.timelineAutomationRangeWidths.length >= 2 &&
    result.timelineAutomationRangeWidths.every((width) => width > 0)
  );
}

function hasExpectedSceneBlocks(result) {
  if (!/^control-live-\d+x\d+$/.test(result.label)) {
    return true;
  }
  const fullSizeWindow = result.innerWidth >= 1920 && result.innerHeight >= 1032;
  const minimumWorkspaceHeight = fullSizeWindow ? 230 : 100;
  const composerControlsReachable = fullSizeWindow
    ? result.fullyVisibleSceneBlockComposerControlCount === 9
    : result.sceneBlockLastControlReachable && result.sceneBlockWorkspaceVerticalOverflowPx > 0;
  const rowActionsReachable = fullSizeWindow
    ? result.fullyVisibleSceneBlockRowActionCount === 9
    : result.visibleSceneBlockRowActionCount === 9 && result.sceneBlockLastControlReachable;
  const linkBadgesReachable = fullSizeWindow
    ? result.visibleSceneBlockLinkBadgeCount >= 3
    : result.visibleSceneBlockLinkBadgeCount >= 2 && result.sceneBlockLastControlReachable;
  return (
    result.visibleSceneBlockWorkspaceCount === 1 &&
    result.sceneBlockRowCount === 3 &&
    result.visibleSceneBlockRowCount >= 2 &&
    result.timelineSceneBlockCount === 2 &&
    result.timelinePointEventCount === 1 &&
    result.timelineActiveSceneBlockCount === 1 &&
    result.timelineActivePointEventCount === 0 &&
    result.sceneBlockLaneScopeHintCount === 4 &&
    result.sceneBlockPlaybackJumpHintCount === 4 &&
    linkBadgesReachable &&
    result.visibleSceneBlockComposerControlCount === 9 &&
    composerControlsReachable &&
    rowActionsReachable &&
    result.sceneBlockWorkspaceWidth >= result.timelinePanelWidth * 0.97 &&
    result.sceneBlockWorkspaceHeight >= minimumWorkspaceHeight &&
    result.sceneBlockWorkspaceHorizontalOverflowPx === 0 &&
    result.sceneBlockListHorizontalOverflowPx === 0 &&
    result.sceneBlockComposerHorizontalOverflowPx === 0
    && !result.timelineTimeStatClipped
  );
}

function hasExpectedProjectMenu(result) {
  if (!result.label.startsWith("project-menu-")) {
    return true;
  }
  return (
    result.visibleProjectMenuCount === 1 &&
    result.visibleProjectMenuItemCount >= 6 &&
    result.visibleProjectMenuShortcutCount >= 4 &&
    result.visibleRecentProjectMenuItemCount >= 5 &&
    result.visibleRecoveryProjectMenuItemCount >= 1 &&
    result.visibleUpdateMenuLabelCount === 1 &&
    result.visibleUpdateCheckButtonCount === 1 &&
    result.visibleUserTemplateMenuLabelCount === 1 &&
    result.visibleUserTemplateActionCount === 2
  );
}

function hasExpectedLocalization(result) {
  if (!result.label.startsWith("localization-ja-")) {
    return true;
  }
  return (
    result.documentLanguage === "ja" &&
    result.visibleProjectMenuCount === 1 &&
    result.visibleJapaneseLanguageLabelCount === 1 &&
    result.visibleJapaneseSaveButtonCount === 1 &&
    result.preservedUserFixtureLabelCount === 3 &&
    result.translatedUserFixtureCollisionCount === 0
  );
}

function hasExpectedKeyboardNavigation(result) {
  if (!result.label.startsWith("control-edit-keyboard-")) {
    return true;
  }
  return result.keyboardNavigationPassed;
}

function isContained(result) {
  const widthTolerance = result.documentClientWidth + 1;
  const heightTolerance = result.documentClientHeight + 1;
  const appWidthTolerance = (result.appClientWidth ?? result.documentClientWidth) + 1;
  const appHeightTolerance = (result.appClientHeight ?? result.documentClientHeight) + 1;
  const layoutWidthTolerance = (result.layoutClientWidth ?? result.documentClientWidth) + 1;
  const layoutHeightTolerance = (result.layoutClientHeight ?? result.documentClientHeight) + 1;
  const isHiddenOverflow = (value) => value === "hidden" || value === "clip";
  return (
    !result.canWindowScrollX &&
    !result.canWindowScrollY &&
    isHiddenOverflow(result.documentOverflowX) &&
    isHiddenOverflow(result.documentOverflowY) &&
    isHiddenOverflow(result.bodyOverflowX) &&
    isHiddenOverflow(result.bodyOverflowY) &&
    result.appPosition === "fixed" &&
    isHiddenOverflow(result.appOverflowX) &&
    isHiddenOverflow(result.appOverflowY) &&
    isHiddenOverflow(result.layoutOverflowX) &&
    isHiddenOverflow(result.layoutOverflowY) &&
    result.documentScrollWidth <= widthTolerance &&
    result.documentScrollHeight <= heightTolerance &&
    result.bodyScrollWidth <= widthTolerance &&
    result.bodyScrollHeight <= heightTolerance &&
    (result.appScrollWidth ?? 0) <= appWidthTolerance &&
    (result.appScrollHeight ?? 0) <= appHeightTolerance &&
    (result.layoutScrollWidth ?? 0) <= layoutWidthTolerance &&
    (result.layoutScrollHeight ?? 0) <= layoutHeightTolerance &&
    result.appRectWidth === result.innerWidth &&
    result.appRectHeight === result.innerHeight
  );
}

function hasNoOuterOverflow(result) {
  return (
    isContained(result) &&
    result.documentScrollWidth === result.documentClientWidth &&
    result.documentScrollHeight === result.documentClientHeight &&
    result.bodyScrollWidth === result.documentClientWidth &&
    result.bodyScrollHeight === result.documentClientHeight &&
    result.appScrollWidth === result.appClientWidth &&
    result.appScrollHeight === result.appClientHeight &&
    result.layoutScrollWidth === result.layoutClientWidth &&
    result.layoutScrollHeight === result.layoutClientHeight &&
    result.cueHostHorizontalOverflowPx === 0 &&
    result.cueHostVerticalOverflowPx === 0
  );
}

function hasExpectedControlModeSurface(result) {
  if (!result.label.startsWith("control-")) {
    return true;
  }
  if (result.controlModeTabCount < 3) {
    return false;
  }
  if (
    !result.label.startsWith("control-mixer-") &&
    !result.label.startsWith("control-edit-effects-") &&
    (
      result.visibleLiveControlPanelCount !== 1 ||
      result.visibleControlStagePanelCount !== 1 ||
      result.visibleControlStageCount !== 1 ||
      result.controlStageWidth < 520 ||
      result.controlStageHeight < 190 ||
      result.controlStageViewBoxAspect < (result.label.startsWith("control-live-") ? 1.44 : 2) ||
      result.controlStageGridCoverage < 0.95 ||
      result.controlStageFixtureMinSize < 12 ||
      result.visibleControlStageReferenceLabelCount > 1
    )
  ) {
    return false;
  }
  if (result.label.startsWith("control-edit-position-")) {
    const hasPositionConsole =
      result.visiblePanTiltPadCount >= 1 &&
      result.visiblePositionReadoutCount >= 1 &&
      result.visiblePositionConsoleCount === 1 &&
      result.visiblePositionToolDeckCount === 1 &&
      result.visiblePositionToolTabCount === 2 &&
      result.visibleActivePositionToolTabCount === 1 &&
      result.visiblePositionAxisSliderCount === 2 &&
      result.positionConsoleHorizontalOverflowPx <= 1 &&
      result.positionToolPaneHorizontalOverflowPx <= 1 &&
      ["auto", "scroll"].includes(result.positionToolPaneOverflowY) &&
      result.positionConsoleContained &&
      result.positionConsoleHeight >= 120 &&
      result.positionToolPaneHeight >= 96 &&
      result.positionToolPaneLastControlReachable &&
      result.visibleAttributeTargetSummaryCount >= 1 &&
      result.visibleGroupAttributeTargetSummaryCount >= 1 &&
      result.visibleEditDeskTabCount === 3 &&
      result.controlWorkSurfaceUnsafeOverflowCount === 0 &&
      result.visibleCuePanelCount === 0 &&
      result.visibleTimelinePanelCount === 0 &&
      result.visibleVideoControlPanelCount === 0;
    if (result.label.startsWith("control-edit-position-limits-")) {
      return (
        hasPositionConsole &&
        result.visibleControlLimitPanelCount === 1 &&
        result.visiblePositionDirectControlCount === 0 &&
        result.visiblePositionNudgeButtonCount === 0 &&
        result.visiblePositionTargetButtonCount === 0 &&
        result.visiblePositionTransformButtonCount === 0 &&
        result.visiblePositionFavoriteButtonCount === 0
      );
    }
    const hasPrimaryPositionTools =
      result.visibleControlLimitPanelCount === 0 &&
      result.visiblePositionDirectControlCount === 3 &&
      result.visiblePositionNudgeButtonCount === 5 &&
      result.visiblePositionTargetButtonCount === 9 &&
      result.visiblePositionTransformButtonCount === 3 &&
      result.visiblePositionFavoriteButtonCount >= 3 &&
      result.fullyVisiblePositionDirectControlCount === 3 &&
      result.fullyVisiblePositionNudgeButtonCount >= 4;
    const hasFullScreenOperationalLayout =
      result.innerWidth < primaryOperationalViewport.width ||
      result.innerHeight < primaryOperationalViewport.height ||
      (
        result.positionConsoleWidth >= 700 &&
        result.positionToolDeckWidth >= 300 &&
        result.positionPadWidth >= 170 &&
        result.positionPadWidth <= 420 &&
        result.positionPadHeight >= 170 &&
        result.positionPadHeight <= 360 &&
        result.positionPadWidth / Math.max(1, result.positionPadHeight) >= 0.75 &&
        result.positionPadWidth / Math.max(1, result.positionPadHeight) <= 1.35 &&
        result.positionPrimaryAndToolsSideBySide &&
        result.fullyVisiblePositionTargetButtonCount === 9 &&
        result.fullyVisiblePositionTransformButtonCount === 3 &&
        result.fullyVisiblePositionFavoriteButtonCount >= 1
      );
    return (
      hasPositionConsole &&
      hasPrimaryPositionTools &&
      hasFullScreenOperationalLayout
    );
  }
  if (result.label.startsWith("control-edit-color-")) {
    return (
      result.visibleColorPlaneCount >= 1 &&
      result.visibleColorReadoutCount >= 1 &&
      result.visibleAttributeTargetSummaryCount >= 1 &&
      result.visibleGroupAttributeTargetSummaryCount >= 1 &&
      result.visibleEditDeskTabCount === 3 &&
      result.controlWorkSurfaceUnsafeOverflowCount === 0 &&
      result.visibleCuePanelCount === 0 &&
      result.visibleTimelinePanelCount === 0 &&
      result.visibleVideoControlPanelCount === 0
    );
  }
  if (result.label.startsWith("control-edit-")) {
    if (result.label.startsWith("control-edit-effects-")) {
      const hasFxDesk =
        result.visibleLiveControlPanelCount === 0 &&
        result.visibleControlStagePanelCount === 0 &&
        result.visibleControlStageCount === 0 &&
        result.visibleEffectWorkbenchCount === 1 &&
        result.visibleEffectLibraryPaneCount === 1 &&
        result.visibleEffectInspectorPaneCount === 1 &&
        result.visibleEffectRackPaneCount === 1 &&
        result.visibleEffectRecipeDockCount === 1 &&
        result.visibleEffectActionDockCount === 1 &&
        result.visibleEffectRackTabCount === 2;
      if (result.label.startsWith("control-edit-effects-graphs-")) {
        return hasFxDesk && result.visibleNodeGraphPanelCount === 1 && result.controlWorkSurfaceUnsafeOverflowCount === 0;
      }
      if (result.label.startsWith("control-edit-effects-move-editor-")) {
        const hasMoveEditor =
          hasFxDesk &&
          result.effectTypeValue === "Move" &&
          result.visibleMoveEffectEditorCount === 1 &&
          result.visibleMoveEffectPathDeskCount === 1 &&
          result.visibleMoveEffectInspectorCount === 1 &&
          result.visibleMoveEffectPathCanvasCount === 1 &&
          result.visibleMoveEffectDirectionButtonCount === 3 &&
          result.visibleMoveEffectTransformInputCount === 5 &&
          result.visibleMoveEffectCoordinateButtonCount === 2 &&
          result.visibleMoveEffectInterpolationButtonCount === 2 &&
          result.visibleMoveEffectClosedToggleCount === 1 &&
          result.visibleMoveEffectPointRowCount >= 2 &&
          result.visibleLegacyEffectAttributeCount === 0 &&
          result.visibleLegacyEffectWaveformCount === 0 &&
          result.visibleLegacyEffectVideoTargetCount === 0 &&
          result.moveEffectEditorHorizontalOverflowPx <= 1 &&
          result.moveEffectSurfaceHorizontalOverflowPx <= 1 &&
          result.moveEffectPathDeskHorizontalOverflowPx <= 1 &&
          result.moveEffectInspectorHorizontalOverflowPx <= 1 &&
          result.moveEffectFormHorizontalOverflowPx <= 1 &&
          ["auto", "scroll"].includes(result.moveEffectFormOverflowY) &&
          ["auto", "scroll"].includes(result.moveEffectPointListOverflowY) &&
          result.moveEffectLastControlReachable &&
          result.moveEffectLastPointReachable &&
          result.moveEffectEditorContained &&
          result.controlWorkSurfaceUnsafeOverflowCount === 0;
        const hasPrimaryOperationalDensity =
          viewportRole({ width: result.innerWidth, height: result.innerHeight }) !== "primary-maximized" ||
          (
            result.moveEffectEditorWidth >= 780 &&
            result.moveEffectSurfaceWidth >= 780 &&
            result.moveEffectPathDeskWidth >= 360 &&
            result.moveEffectInspectorWidth >= 380 &&
            result.moveEffectPathCanvasWidth >= 320 &&
            result.moveEffectPathCanvasHeight >= 300 &&
            result.moveEffectCanvasContained &&
            result.moveEffectColumnsSideBySide &&
            result.moveEffectSurfaceWidthCoverage >= 0.98 &&
            result.moveEffectColumnAreaCoverage >= 0.98 &&
            result.moveEffectColumnAreaCoverage <= 1.02 &&
            result.moveEffectUnusedRightPx <= 2 &&
            result.moveEffectUnusedBottomPx <= 2
          );
        return hasMoveEditor && hasPrimaryOperationalDensity;
      }
      if (result.label.startsWith("control-edit-effects-color-editor-")) {
        return (
          hasFxDesk &&
          result.effectTypeValue === "Color" &&
          result.visibleColorEffectEditorCount === 1 &&
          result.visibleColorEffectStopCount === 3 &&
          result.visibleColorEffectAddButtonCount === 1 &&
          result.visibleColorEffectRemoveButtonCount === 3 &&
          result.visibleColorEffectGradientCount === 1 &&
          result.colorEffectGradientRenderedCount === 1 &&
          result.visibleColorEffectAlgorithmSelectCount === 1 &&
          result.colorEffectAlgorithmValue === "Random" &&
          result.visibleColorEffectInterpolationSelectCount === 1 &&
          result.colorEffectInterpolationValue === "HsvLongest" &&
          result.visibleLegacyEffectAttributeCount === 0 &&
          result.visibleLegacyEffectWaveformCount === 0 &&
          result.visibleLegacyEffectVideoTargetCount === 0 &&
          result.colorEffectDraftSummaryIsCurrent &&
          result.colorEffectStatusIsCurrent &&
          result.colorEffectDisabledVideoOptionCount === 1 &&
          result.colorEffectUnlabeledActionCount === 0 &&
          result.colorEffectHorizontalOverflowPx <= 1 &&
          result.colorEffectEditorContained &&
          result.controlWorkSurfaceUnsafeOverflowCount === 0
        );
      }
      if (result.label.startsWith("control-edit-effects-chaser-editor-")) {
        return (
          hasFxDesk &&
          result.effectTypeValue === "Chaser" &&
          result.visibleChaserEditorCount === 1 &&
          result.visibleChaserStepCount === 6 &&
          result.visibleChaserFeatureCount === 2 &&
          result.visibleChaserPreviewCellCount === 6 &&
          result.visibleActiveChaserPreviewCellCount === 2 &&
          result.visibleChaserDirectionButtonCount === 4 &&
          result.visibleChaserReplaceButtonCount === 6 &&
          result.chaserDirectionValue === "Reverse" &&
          result.chaserActiveStepCount === 2 &&
          result.chaserSizePercent === 37 &&
          result.chaserPhasePercent === 50 &&
          result.chaserFadingEnabled &&
          result.visibleLegacyEffectAttributeCount === 0 &&
          result.visibleLegacyEffectWaveformCount === 0 &&
          result.visibleLegacyEffectVideoTargetCount === 0 &&
          result.chaserDisabledVideoOptionCount === 1 &&
          result.chaserUnlabeledActionCount === 0 &&
          result.chaserHorizontalOverflowPx <= 1 &&
          result.chaserEditorContained &&
          result.controlWorkSurfaceUnsafeOverflowCount === 0
        );
      }
      if (result.label.startsWith("control-edit-effects-colour-")) {
        return (
          hasFxDesk &&
          result.visibleEffectEditorCount === 1 &&
          result.visibleEffectFamilyButtonCount === 8 &&
          result.visibleActiveEffectFamilyButtonCount === 1 &&
          result.visibleEffectLibraryCardCount === 1 &&
          result.visibleTargetRequiredEffectCardCount === 1 &&
          result.controlWorkSurfaceUnsafeOverflowCount === 0
        );
      }
      return (
        hasFxDesk &&
        result.visibleEditDeskTabCount === 3 &&
        result.visibleFixtureEditSurfaceCount === 0 &&
        result.visibleEffectEditorCount === 1 &&
        result.visibleEffectFamilyButtonCount === 8 &&
        result.visibleActiveEffectFamilyButtonCount === 1 &&
        result.visibleEffectLibraryCardCount === 13 &&
        result.visibleTargetRequiredEffectCardCount === 4 &&
        result.visibleNodeGraphPanelCount === 0 &&
        result.effectTargetHintCount >= 1 &&
        result.visibleRawMonitorCount === 0 &&
        result.controlWorkSurfaceUnsafeOverflowCount === 0
      );
    }
    if (result.label.startsWith("control-edit-dmx-")) {
      return (
        result.visibleEditDeskTabCount === 3 &&
        result.visibleFixtureEditSurfaceCount === 0 &&
        result.visibleEffectEditorCount === 0 &&
        result.visibleRawMonitorCount === 1 &&
        result.controlWorkSurfaceUnsafeOverflowCount === 0
      );
    }
    return (
      result.visibleFixtureEditSurfaceCount > 0 &&
      result.visibleEffectEditorCount === 0 &&
      result.visibleRawMonitorCount === 0 &&
      result.visibleEditDeskTabCount === 3 &&
      result.controlWorkSurfaceUnsafeOverflowCount === 0 &&
      result.effectTargetHintCount === 0 &&
      result.effectTargetMapSelectionOptionCount >= 1 &&
      result.visibleCuePanelCount === 0 &&
      result.visibleTimelinePanelCount === 0 &&
      result.visibleVideoControlPanelCount === 0
    );
  }
  if (result.label.startsWith("control-live-")) {
    if (result.liveControlPanelHeight < 330 || result.liveControlPanelHeight > 342) return false;
    if (
      result.visibleLiveFadeMeterCount !== 1 ||
      result.liveFadeMeterGridRow !== "5" ||
      result.liveMasterGridRow !== "6" ||
      !result.liveFadeAboveMaster
    ) return false;
    if (result.label.startsWith("control-live-playback-")) {
      return (
        result.visibleTimelineDeskTabCount === 4 &&
        result.visiblePlaybackDeskSurfaceCount === 1 &&
        result.visibleProgrammerPanelCount === 1 &&
        result.visibleReferencePalettePanelCount === 1 &&
        result.visiblePlaybackExecutorPanelCount === 1 &&
        result.visiblePlaybackMasterCount === 1 &&
        result.visibleCuePanelCount === 0 &&
        result.visibleTimelinePanelCount === 0 &&
        result.controlWorkSurfaceUnsafeOverflowCount === 0
      );
    }
    if (result.label.startsWith("control-live-cues-")) {
      const cueToggleIsAccessible =
        result.visibleCueEditToggleCount === 1 &&
        result.cueEditToggleMinTargetSize >= 44 &&
        result.cueEditToggleControls === "cue-list-editor cue-store-form cue-effect-capture-editor cue-list-items";
      if (result.label.startsWith("control-live-cues-effects-only-")) {
        return (
          result.visibleTimelineDeskTabCount === 4 &&
          result.visibleCuePanelCount === 1 &&
          result.visibleCueLivePanelCount === 1 &&
          result.visibleCueFormCount === 1 &&
          result.visibleCueEffectRecallEditorCount >= 1 &&
          cueToggleIsAccessible &&
          result.cueEditToggleExpanded === "true" &&
          result.cueCaptureScopeValue === "effects" &&
          result.cuePreviewScopeLabel === "Effects Only" &&
          result.cuePreviewStatValues.length === 5 &&
          result.cuePreviewStatValues.every((value) => value === 0) &&
          result.cueStoreButtonDisabled &&
          result.visibleCueScopeErrorCount === 1 &&
          result.cuePanelOverflowY === "auto" &&
          result.cuePanelHorizontalOverflowPx <= 1 &&
          result.cueHostVerticalOverflowPx <= 1 &&
          result.visibleTimelinePanelCount === 0 &&
          result.controlWorkSurfaceUnsafeOverflowCount === 0
        );
      }
      if (result.label.startsWith("control-live-cues-edit-")) {
        return (
          result.visibleTimelineDeskTabCount === 4 &&
          result.visibleCuePanelCount === 1 &&
          result.visibleCueLivePanelCount === 1 &&
          result.visibleCueFormCount === 1 &&
          result.visibleCueEffectRecallEditorCount >= 1 &&
          result.visibleCueEditOnlyCount > 0 &&
          cueToggleIsAccessible &&
          result.cueEditToggleExpanded === "true" &&
          result.cuePanelOverflowY === "auto" &&
          result.cuePanelVerticalOverflowPx > 0 &&
          result.cuePanelHorizontalOverflowPx <= 1 &&
          result.cueHostVerticalOverflowPx <= 1 &&
          result.visibleTimelinePanelCount === 0 &&
          result.controlWorkSurfaceUnsafeOverflowCount === 0
        );
      }
      return (
        result.visibleTimelineDeskTabCount === 4 &&
        result.visibleCuePanelCount === 1 &&
        result.visibleCueLivePanelCount === 1 &&
        result.visibleCueFormCount === 0 &&
        result.visibleCueEffectRecallEditorCount === 0 &&
        result.visibleCueEditOnlyCount === 0 &&
        cueToggleIsAccessible &&
        result.cueEditToggleExpanded === "false" &&
        result.visibleTimelinePanelCount === 0 &&
        result.controlWorkSurfaceUnsafeOverflowCount === 0
      );
    }
    if (result.label.startsWith("control-live-automation-")) {
      return (
        result.visibleTimelineDeskTabCount === 4 &&
        result.visibleCuePanelCount === 0 &&
        result.visibleTimelinePanelCount === 1 &&
        result.visibleTimelineShowSurfaceCount === 0 &&
        result.visibleTimelineAutomationSurfaceCount === 1 &&
        result.controlWorkSurfaceUnsafeOverflowCount === 0
      );
    }
    return (
      result.visibleCuePanelCount === 0 &&
      result.visibleCueLivePanelCount === 0 &&
      result.visibleCueFormCount === 0 &&
      result.visibleCueEditOnlyCount === 0 &&
      result.visibleTimelinePanelCount > 0 &&
      result.visibleTimelineDeskTabCount === 4 &&
      result.visibleTimelineShowSurfaceCount === 1 &&
      result.controlWorkSurfaceUnsafeOverflowCount === 0 &&
      result.visibleFixtureEditSurfaceCount === 0 &&
      result.visibleEffectEditorCount === 0 &&
      result.visibleRawMonitorCount === 0 &&
      result.visibleVideoControlPanelCount === 0
    );
  }
  if (result.label.startsWith("control-mixer-")) {
    return (
      result.visibleLiveControlPanelCount === 0 &&
      result.visibleControlStagePanelCount === 0 &&
      result.visibleControlStageCount === 0 &&
      result.visibleVideoControlPanelCount > 0 &&
      result.visibleVideoMixerClipPaneCount === 1 &&
      result.visibleVideoMixerProgramPaneCount === 1 &&
      result.visibleVideoMixerLayerPaneCount === 1 &&
      result.visibleVideoMonitorPanelCount === 1 &&
      result.visibleVideoPreviewBusCount === 1 &&
      result.visibleVideoProgramBusCount === 1 &&
      result.visibleVjPreviewTransportCount === 1 &&
      result.visibleVjPreviewTransportButtonCount === 4 &&
      result.disabledVjPreviewTransportButtonCount === 4 &&
      result.undersizedVjPreviewTransportButtonCount === 0 &&
      result.visibleVjPreviewStageButtonCount > 0 &&
      result.disabledVjPreviewStageButtonCount === result.visibleVjPreviewStageButtonCount &&
      result.undersizedVjPreviewStageButtonCount === 0 &&
      result.visibleVideoProgramRefreshCount === 0 &&
      result.visibleVideoMasterControlCount > 0 &&
      result.visibleVideoMasterFaderCount > 0 &&
      result.visibleVideoClipGridCount > 0 &&
      result.visibleVideoClipPadCount > 0 &&
      result.visibleVideoClipTakeButtonCount > 0 &&
      result.visibleVideoClipAudioButtonCount > 0 &&
      result.visibleVideoProgramAudioToggleCount > 0 &&
      result.visibleVideoAbDeckCount > 0 &&
      result.visibleVideoDeckLoadButtonCount >= 2 &&
      result.visibleVideoRecordingBarCount > 0 &&
      result.visibleLiveAudioInputBarCount === 1 &&
      result.liveAudioInputBarDomCount === 1 &&
      result.visibleLiveAudioRailCount === 1 &&
      result.visibleEmbeddedLiveAudioCount === 0 &&
      result.liveAudioRailHeight > 0 &&
      result.liveAudioRailHeight <= 70 &&
      result.liveAudioRailBelowMaster === true &&
      result.liveAudioRailAboveClipGrid === true &&
      result.liveAudioRailOverflowX <= 1 &&
      result.liveAudioRailOverflowY <= 1 &&
      result.liveAudioRailMeterCount === 3 &&
      result.liveAudioRailPoliteRegionCount === 1 &&
      result.fullyVisibleVideoClipPadCount >= 1 &&
      result.visibleVideoOutputControlListCount > 0 &&
      result.visibleVideoOutputItemCount > 0 &&
      result.visibleVideoOutputSelectedItemCount > 0 &&
      result.visibleVideoMixerOutputDeckCount >= result.visibleVideoOutputItemCount &&
      result.visibleVideoMixerOutputFaderCount >= result.visibleVideoOutputItemCount &&
      result.visibleVideoMixerOutputSelectButtonCount >= result.visibleVideoOutputItemCount &&
      result.visibleVideoLayerListCount > 0 &&
      result.visibleVideoLayerItemCount > 0 &&
      result.visibleBuiltinVideoFxSelectCount > 0 &&
      result.visibleVideoMixerLayerDeckCount >= result.visibleVideoLayerItemCount &&
      result.visibleVideoMixerLayerFaderCount >= result.visibleVideoLayerItemCount &&
      result.visibleVideoMixerLayerButtonCount >= 5 &&
      result.visibleVideoDeckPagerCount >= 2 &&
      result.controlWorkSurfaceUnsafeOverflowCount === 0 &&
      result.visibleVideoMixerDiagnosticsCount === 0 &&
      result.visibleVideoMixerSetupToolsCount === 0 &&
      result.visibleVideoMixerAutomationToolsCount === 0 &&
      result.visibleOutputPanelCount === 0
    );
  }
  return true;
}

function hasExpectedSetupSurface(result) {
  if (result.label.startsWith("setup-library-")) {
    return (
      result.visibleProfileLoadPanelCount >= 1 &&
      result.visibleLoadedProfileSummaryPanelCount >= 1 &&
      result.profileLoadPanelWidth >= 250 &&
      result.loadedProfileSummaryPanelWidth >= 760
    );
  }
  if (result.label.startsWith("setup-profiles-")) {
    return (
      result.visibleCustomProfileWorkbenchCount >= 1 &&
      result.customProfileAttributePaneWidth >= 460 &&
      result.customProfilePreviewDeskWidth >= 420 &&
      result.visibleCustomProfileActionCount >= 3 &&
      result.visibleCustomProfileDmxMapCount >= 1
    );
  }
  if (result.label.startsWith("setup-dmx-")) {
    return (
      result.visibleDmxOutputConfigPanelCount >= 1 &&
      result.visibleArtRdmPanelCount >= 1 &&
      result.visibleOutputDiagnosticsDeskCount >= 1 &&
      result.visibleLightingRuntimeDeskCount >= 1 &&
      result.dmxRouteItemCount === 128 &&
      result.visibleSerialPortIdentityCount === 1 &&
      result.visibleSerialProtocolRecommendationCount === 1 &&
      result.dmxOutputConfigPanelWidth >= 250 &&
      result.outputDiagnosticsDeskWidth >= 420 &&
      result.lightingRuntimeDeskWidth >= 260 &&
      result.dmxEndpointShrunkenChildCount === 0 &&
      result.dmxEndpointChildOverlapCount === 0
    );
  }
  if (result.label.startsWith("setup-midi-")) {
    return result.midiMappingEditorDeskWidth >= 430 && result.midiMappingListDeskWidth >= 430;
  }
  if (result.label.startsWith("setup-osc-")) {
    return result.oscMappingEditorDeskWidth >= 430 && result.oscMappingListDeskWidth >= 430;
  }
  if (result.label.startsWith("setup-remote-")) {
    return (
      result.remoteServerDeskWidth >= 360 &&
      result.remoteEndpointDeskWidth >= 500 &&
      result.visibleStandbySyncDeskCount === 1 &&
      result.visibleStandbyRoleOptionCount === 2 &&
      result.visibleStandbyActionButtonCount === 3
    );
  }
  if (result.label.startsWith("setup-patch-")) {
    return (
      result.visiblePatchActionRowCount >= 1 &&
      result.visiblePatchAutoButtonCount >= 1 &&
      result.visiblePatchPrimaryButtonCount >= 1 &&
      result.visiblePatchNextFreeButtonCount >= 1 &&
      result.visiblePatchFootprintCount >= 1 &&
      result.visibleDmxAddressGridCount >= 1 &&
      result.dmxAddressCellCount === 128 &&
      result.dmxAddressOccupiedCellCount > 0 &&
      result.dmxAddressPlannedCellCount > 0 &&
      result.visibleDmxFixtureBlockCount > 0 &&
      result.compactMappingStageWidth >= 420 &&
      result.compactMappingStageHeight >= 180 &&
      result.visibleDmxGridSummaryCount >= 1 &&
      result.visibleFixtureSetupEditorCount >= 1 &&
      result.visibleUseProfileForPatchButtonCount >= 1 &&
      result.visibleDuplicateFixtureButtonCount >= 1
    );
  }
  if (result.label.startsWith("setup-mapping-")) {
    return (
      result.mappingUseInEffectsButtonCount >= 1 &&
      result.mappingWaveDraftButtonCount >= 1 &&
      result.visibleMappingProjectorButtonCount >= 1 &&
      result.visibleMappingProjectorControlsCount >= 1 &&
      result.visibleMappingProjectorWarpGridCount >= 1 &&
      result.visibleMappingProjectorActionButtonCount >= 4 &&
      result.visibleMappingProjectorResetPoseButtonCount >= 1 &&
      result.visibleStageVideoSurfaceCount >= 1 &&
      result.mappingFilterVerticalClipCount === 0 &&
      result.mappingViewportChildOverlapCount === 0 &&
      result.mappingStageHeight >= 180 &&
      result.mappingSidebarHorizontalOverflowPx <= 1 &&
      result.mappingSidebarClippedControlCount === 0
    );
  }
  if (result.label.startsWith("setup-video-")) {
    return (
      result.visibleSetupVideoPanelCount >= 1 &&
      result.videoSetupSidebarWidth >= 220 &&
      result.videoSetupOutputDeskWidth >= 780 &&
      result.visibleVideoSetupRoutingPaneCount === 1 &&
      result.visibleVideoSetupMapPaneCount === 1 &&
      result.visibleVideoSetupInspectorPaneCount === 1 &&
      result.videoSetupRoutingPaneWidth >= 220 &&
      result.videoSetupMapPaneWidth >= 498 &&
      result.videoSetupInspectorPaneWidth >= 288 &&
      result.videoSetupMapPaneHeight >= 520 &&
      result.videoSetupMapPaneOverflowPx <= 1 &&
      result.videoSetupProjectorSurfaceContained &&
      result.videoSetupPreviewContained &&
      result.visibleVideoSetupActionDockCount === 1 &&
      result.videoSetupActionDockInViewport &&
      result.videoSetupCriticalActionInViewportCount === 3 &&
      result.visibleVideoSetupDisplayActionCount === 1 &&
      result.visibleSetupVideoOutputDeckCount >= 1 &&
      result.visibleSetupVideoOutputActiveDeckCount >= 1 &&
      result.visibleSetupVideoOutputDetailPaneCount >= 1 &&
      result.visibleVideoOutputMappingPanelCount >= 1 &&
      result.visibleVideoOutputBlendControlsCount >= 1 &&
      result.visibleProjectorMapEditorCount >= 1 &&
      result.visibleProjectorMapHandleCount >= 4 &&
      result.visibleProjectorKeystoneHandleCount >= 2 &&
      result.visibleProjectorScaleHandleCount >= 2 &&
      result.visibleProjectorRotateHandleCount >= 1 &&
      result.visibleProjectorAspectModeButtonCount >= 3 &&
      result.visibleProjectorAspectPresetButtonCount >= 3 &&
      result.visibleProjectorResetPoseButtonCount >= 1
    );
  }
  return true;
}

function hasExpectedMappingWaveDraft(result) {
  if (!result.label.startsWith("mapping-wave-draft-")) {
    return true;
  }
  return (
    result.visibleEffectEditorCount > 0 &&
    result.effectTargetHintCount >= 1 &&
    result.effectTargetHintText.toLowerCase().includes("map selection") &&
    result.effectTargetHintText.toLowerCase().includes("wave") &&
    result.effectTargetValue === "selection" &&
    result.effectTypeValue === "PositionWave" &&
    result.effectCommonAttributeValue.length > 0
  );
}

function hasExpectedMappingHotkeyHelp(result) {
  if (!result.label.startsWith("mapping-hotkey-help-")) {
    return true;
  }
  return result.visibleMappingHotkeyHelpCount === 1 && result.mappingHotkeyHelpKeyCount >= 20;
}

function hasExpectedTouchSurface(result) {
  if (!result.label.startsWith("touch-")) {
    return true;
  }
  return (
    result.visibleTouchCuePanelCount > 0 &&
    result.visibleTouchSafetyDeckCount === 1 &&
    result.visibleTouchSafetyGuardButtonCount === 4 &&
    result.visibleTouchGoDeckCount > 0 &&
    result.visibleTouchCuePadCount >= 4 &&
    result.visibleTouchStagePanelCount > 0 &&
    result.visibleTouchStageCount > 0 &&
    result.visibleTouchFixturePanelCount > 0 &&
    result.visibleTouchFixtureScrollerCount > 0 &&
    result.visibleTouchRemotePanelCount === 0 &&
    result.visibleTouchVideoPanelCount > 0 &&
    result.visibleTouchVideoOutputDeckCount > 0 &&
    result.visibleTouchVideoSelectedOutputDeckCount > 0 &&
    result.visibleTouchVideoOutputFaderCount >= result.visibleTouchVideoOutputDeckCount &&
    result.visibleTouchVideoOutputButtonCount >= 4 &&
    result.visibleTouchVideoOutputSelectButtonCount >= result.visibleTouchVideoOutputDeckCount &&
    result.visibleTouchVideoDeckCount > 0 &&
    result.visibleTouchVideoLayerFaderCount >= 2 &&
    result.visibleTouchMasterGridCount > 0 &&
    result.touchUndersizedTargetCount === 0 &&
    result.touchMomentaryFlashPassed
  );
}

async function checkTouchMomentaryFlash(client) {
  return await client.evaluate(`(async () => {
    const findFlashButton = () => [...document.querySelectorAll('.touchDimmerQuickRow button.momentary')]
      .find((candidate) => {
        const rect = candidate.getBoundingClientRect();
        const style = window.getComputedStyle(candidate);
        return !candidate.disabled && rect.width > 0 && rect.height > 0 && style.display !== 'none' && style.visibility !== 'hidden';
      });
    let button = findFlashButton();
    if (!button) {
      const dimmerCategory = [...document.querySelectorAll('.touchAttributeCategoryRail button')]
        .find((candidate) => (candidate.textContent || '').trim().toLowerCase().startsWith('dimmer'));
      dimmerCategory?.click();
      await new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame)));
      button = findFlashButton();
    }
    if (!button) {
      window.__syndocalTouchMomentaryCheck = { passed: false, found: false, activeOnPress: false, inactiveOnRelease: false };
      return false;
    }
    button.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true, pointerId: 1, pointerType: 'touch', isPrimary: true }));
    await new Promise((resolveFrame) => requestAnimationFrame(resolveFrame));
    const activeOnPress = button.classList.contains('active');
    button.dispatchEvent(new PointerEvent('pointerup', { bubbles: true, pointerId: 1, pointerType: 'touch', isPrimary: true }));
    await new Promise((resolveFrame) => requestAnimationFrame(resolveFrame));
    const inactiveOnRelease = !button.classList.contains('active');
    window.__syndocalTouchMomentaryCheck = {
      passed: activeOnPress && inactiveOnRelease,
      found: true,
      activeOnPress,
      inactiveOnRelease,
    };
    return window.__syndocalTouchMomentaryCheck.passed;
  })()`);
}

async function runViewport(client, viewport) {
  await client.send("Emulation.setDeviceMetricsOverride", {
    width: viewport.width,
    height: viewport.height,
    deviceScaleFactor: 1,
    mobile: false,
  });
  await client.send("Page.navigate", { url: appUrl });
  await waitForApp(client);
  await seedViewportLocalStorage(client);
  await client.send("Page.navigate", { url: appUrl });
  await waitForApp(client);

  if (viewport.width === 1280) {
    await checkWorkspaceLayoutPersistence(client);
  }

  const results = [];
  await pressKey(client, "F2");
  await sleep(120);
  await checkKeyboardNavigation(client);
  results.push(await measure(client, `control-edit-keyboard-${viewport.width}x${viewport.height}`));
  await pressKey(client, "F1");
  await sleep(80);
  await clickByText(client, "Mapping");
  await clickByText(client, "Stage Map");
  await sleep(120);
  results.push(await measure(client, `setup-mapping-keyboard-${viewport.width}x${viewport.height}`));
  await client.evaluate(`document.querySelector('button[aria-label="Keyboard shortcut help"]')?.click()`);
  await sleep(120);
  results.push(await measure(client, `mapping-hotkey-help-${viewport.width}x${viewport.height}`));
  await pressKey(client, "Escape", "Escape");
  await sleep(80);
  await pressKey(client, "F3");
  await sleep(120);
  await checkTouchMomentaryFlash(client);
  results.push(await measure(client, `touch-keyboard-${viewport.width}x${viewport.height}`));
  await pressKey(client, "F1");
  await sleep(80);
  await clickVisibleByText(client, ".appMenuButton", "...");
  await sleep(120);
  results.push(await measure(client, `project-menu-${viewport.width}x${viewport.height}`));
  await clickVisibleByText(client, ".appMenuButton", "...");
  await sleep(80);
  if (viewport.width === 1366) {
    await client.evaluate(`window.localStorage.setItem('syndocal.uiScale.v1', '110')`);
    await client.send("Page.navigate", { url: appUrl });
    await waitForApp(client);
    await pressKey(client, "F2");
    await sleep(120);
    results.push(await measure(client, `interface-scale-110-${viewport.width}x${viewport.height}`));
    await client.evaluate(`window.localStorage.setItem('syndocal.uiScale.v1', '100')`);
    await client.send("Page.navigate", { url: appUrl });
    await waitForApp(client);
    await client.evaluate(`window.localStorage.setItem('syndocal.uiLocale.v1', 'ja')`);
    await client.send("Page.navigate", { url: appUrl });
    await waitForApp(client);
    await pressKey(client, "F1");
    await sleep(120);
    await clickVisibleByText(client, ".appMenuButton", "...");
    await sleep(120);
    results.push(await measure(client, `localization-ja-${viewport.width}x${viewport.height}`));
    await client.evaluate(`window.localStorage.setItem('syndocal.uiLocale.v1', 'en')`);
    await client.send("Page.navigate", { url: appUrl });
    await waitForApp(client);
  }
  await clickByText(client, "Setup");
  for (const setupTab of setupTabs) {
    await clickByText(client, setupTab.area);
    await clickByText(client, setupTab.tab);
    if (setupTab.id === "dmx") {
      await client.evaluate(`(() => {
        const select = document.querySelector('.setupMode-dmx .dmxOutputConfigPanel select');
        if (!(select instanceof HTMLSelectElement)) return;
        select.value = 'EnttecUsbPro';
        select.dispatchEvent(new InputEvent('input', { bubbles: true }));
      })()`);
    }
    await sleep(180);
    if (shouldCaptureViewport(viewport)) {
      mkdirSync(screenshotDir, { recursive: true });
      const screenshot = await client.send("Page.captureScreenshot", { format: "png", fromSurface: true });
      writeFileSync(join(screenshotDir, `setup-${setupTab.id}-${viewport.width}x${viewport.height}.png`), screenshot.data, "base64");
    }
    results.push(await measure(client, `setup-${setupTab.id}-${viewport.width}x${viewport.height}`));
  }
  await clickByText(client, "Setup");
  await clickByText(client, "Mapping");
  await clickByText(client, "Stage Map");
  await sleep(120);
  await clickByText(client, "Pick Visible");
  await sleep(120);
  await clickByText(client, "Wave Draft");
  await sleep(180);
  results.push(await measure(client, `mapping-wave-draft-${viewport.width}x${viewport.height}`));
  await clickByText(client, "Control");
  for (const controlTab of controlTabs) {
    await clickByText(client, controlTab.label);
    await sleep(320);
    if (shouldCaptureViewport(viewport)) {
      mkdirSync(screenshotDir, { recursive: true });
      const screenshot = await client.send("Page.captureScreenshot", { format: "png", fromSurface: true });
      writeFileSync(join(screenshotDir, `control-${controlTab.id}-${viewport.width}x${viewport.height}.png`), screenshot.data, "base64");
    }
    results.push(await measure(client, `control-${controlTab.id}-${viewport.width}x${viewport.height}`));
    if (controlTab.id === "edit") {
      await clickVisibleByText(client, ".attributeCategoryRail button", "Position");
      await sleep(120);
      if (shouldCaptureViewport(viewport)) {
        const screenshot = await client.send("Page.captureScreenshot", { format: "png", fromSurface: true });
        writeFileSync(
          join(screenshotDir, `control-edit-static-position-${viewport.width}x${viewport.height}.png`),
          screenshot.data,
          "base64",
        );
      }
      results.push(await measure(client, `control-edit-position-${viewport.width}x${viewport.height}`));
      await client.evaluate("document.querySelector('#position-tool-tab-position')?.focus()");
      await pressKey(client, "ArrowRight");
      await sleep(120);
      if (shouldCaptureViewport(viewport)) {
        const screenshot = await client.send("Page.captureScreenshot", { format: "png", fromSurface: true });
        writeFileSync(
          join(screenshotDir, `control-edit-static-position-limits-${viewport.width}x${viewport.height}.png`),
          screenshot.data,
          "base64",
        );
      }
      results.push(await measure(client, `control-edit-position-limits-${viewport.width}x${viewport.height}`));
      await clickVisibleSelector(client, "#position-tool-tab-position");
      await sleep(80);
      await clickVisibleByText(client, ".attributeCategoryRail button", "Color");
      await sleep(120);
      results.push(await measure(client, `control-edit-color-${viewport.width}x${viewport.height}`));
      await clickVisibleByText(client, ".editDeskTabs button", "Effects");
      await sleep(120);
      if (shouldCaptureViewport(viewport)) {
        const screenshot = await client.send("Page.captureScreenshot", { format: "png", fromSurface: true });
        writeFileSync(join(screenshotDir, `control-edit-effects-${viewport.width}x${viewport.height}.png`), screenshot.data, "base64");
      }
      results.push(await measure(client, `control-edit-effects-${viewport.width}x${viewport.height}`));
      await clickVisibleByText(client, ".effectRackTabs button", "Graphs");
      await sleep(80);
      results.push(await measure(client, `control-edit-effects-graphs-${viewport.width}x${viewport.height}`));
      await clickVisibleByText(client, ".effectRackTabs button", "Stack");
      await clickVisibleByText(client, ".effectFamilyRail button", "Colour");
      await sleep(80);
      results.push(await measure(client, `control-edit-effects-colour-${viewport.width}x${viewport.height}`));
      await clickVisibleByText(client, ".effectFamilyRail button", "All");
      await selectVisibleOption(client, ".effectEditor select", "Color");
      await sleep(120);
      const initialColorStopCount = await client.evaluate("document.querySelectorAll('.colorEffectStopRow').length");
      if (initialColorStopCount !== 3) {
        throw new Error(`Expected 3 initial Color stops, found ${initialColorStopCount}`);
      }
      await clickVisibleByText(client, ".colorEffectPaletteFooter button", "Add stop");
      await sleep(40);
      const addedColorStopCount = await client.evaluate("document.querySelectorAll('.colorEffectStopRow').length");
      if (addedColorStopCount !== 4) {
        throw new Error(`Expected Add stop to create a fourth Color stop, found ${addedColorStopCount}`);
      }
      const removedColorStop = await client.evaluate(`(() => {
        const buttons = [...document.querySelectorAll('.colorEffectStopAction.remove:not(:disabled)')]
          .filter((candidate) => {
            const rect = candidate.getBoundingClientRect();
            return rect.width > 0 && rect.height > 0;
          });
        const button = buttons[1] || buttons[0];
        button?.click();
        return Boolean(button);
      })()`);
      if (!removedColorStop) {
        throw new Error("Could not exercise Color stop removal");
      }
      await selectVisibleOption(client, ".colorEffectModeGrid select", "Random");
      await selectVisibleOption(client, ".colorEffectModeGrid select", "HsvLongest");
      await sleep(240);
      if (shouldCaptureViewport(viewport)) {
        const screenshot = await client.send("Page.captureScreenshot", { format: "png", fromSurface: true });
        writeFileSync(join(screenshotDir, `control-edit-effects-color-editor-${viewport.width}x${viewport.height}.png`), screenshot.data, "base64");
      }
      results.push(await measure(client, `control-edit-effects-color-editor-${viewport.width}x${viewport.height}`));
      await selectVisibleOption(client, ".effectEditor select", "Chaser");
      await sleep(100);
      await clickVisibleByText(client, ".chaserDirectionGrid button", "Reverse");
      await client.evaluate(`(() => {
        const setInput = (input, value) => {
          if (!input) return false;
          input.value = String(value);
          input.dispatchEvent(new InputEvent('input', { bubbles: true }));
          input.dispatchEvent(new Event('change', { bubbles: true }));
          return true;
        };
        const labels = [...document.querySelectorAll('.chaserEffectEditor label, .effectActionDock > label')];
        const pixels = labels.find((label) => (label.textContent || '').trim().startsWith('Pixels on'))?.querySelector('input');
        const size = labels.find((label) => (label.textContent || '').trim().startsWith('Size'))?.querySelector('input[type="range"]');
        const phase = labels.find((label) => (label.textContent || '').trim().startsWith('Phase'))?.querySelector('input');
        const fading = document.querySelector('.chaserFadingToggle input');
        const results = [setInput(pixels, 2), setInput(size, 37), setInput(phase, 0.5)];
        if (fading && !fading.checked) fading.click();
        if (results.some((result) => !result)) throw new Error('Could not update responsive Chaser controls');
      })()`);
      await clickVisibleByText(client, ".chaserFeatureFooter button", "Add feature");
      await clickVisibleByText(client, ".chaserTargetDock button", "Append current");
      await sleep(180);
      if (shouldCaptureViewport(viewport)) {
        const screenshot = await client.send("Page.captureScreenshot", { format: "png", fromSurface: true });
        writeFileSync(join(screenshotDir, `control-edit-effects-chaser-editor-${viewport.width}x${viewport.height}.png`), screenshot.data, "base64");
      }
      results.push(await measure(client, `control-edit-effects-chaser-editor-${viewport.width}x${viewport.height}`));
      await selectVisibleOption(client, ".effectEditor select", "Move");
      await sleep(180);
      if (shouldCaptureViewport(viewport)) {
        mkdirSync(screenshotDir, { recursive: true });
        const screenshot = await client.send("Page.captureScreenshot", { format: "png", fromSurface: true });
        writeFileSync(join(screenshotDir, `control-edit-effects-move-editor-${viewport.width}x${viewport.height}.png`), screenshot.data, "base64");
      }
      results.push(await measure(client, `control-edit-effects-move-editor-${viewport.width}x${viewport.height}`));
      await clickVisibleByText(client, ".editDeskTabs button", "DMX");
      await sleep(120);
      results.push(await measure(client, `control-edit-dmx-${viewport.width}x${viewport.height}`));
      await clickVisibleByText(client, ".editDeskTabs button", "Attributes");
    }
    if (controlTab.id === "live") {
      await clickVisibleByText(client, ".timelineDeskTabs button", "Cues");
      await sleep(120);
      results.push(await measure(client, `control-live-cues-${viewport.width}x${viewport.height}`));
      await clickVisibleByText(client, ".cuePanelEditToggle", "Edit Cues");
      await sleep(120);
      results.push(await measure(client, `control-live-cues-edit-${viewport.width}x${viewport.height}`));
      await selectVisibleOption(client, "#cue-store-form select", "effects");
      await sleep(120);
      results.push(await measure(client, `control-live-cues-effects-only-${viewport.width}x${viewport.height}`));
      if (shouldCaptureViewport(viewport)) {
        const screenshot = await client.send("Page.captureScreenshot", { format: "png", fromSurface: true });
        writeFileSync(join(screenshotDir, `control-live-cues-effects-only-${viewport.width}x${viewport.height}.png`), screenshot.data, "base64");
      }
      await selectVisibleOption(client, "#cue-store-form select", "all");
      await sleep(80);
      await clickVisibleByText(client, ".cuePanelEditToggle", "Done");
      await sleep(120);
      results.push(await measure(client, `control-live-cues-closed-${viewport.width}x${viewport.height}`));
      await clickVisibleByText(client, ".timelineDeskTabs button", "Automation");
      await sleep(120);
      results.push(await measure(client, `control-live-automation-${viewport.width}x${viewport.height}`));
      await clickVisibleByText(client, ".timelineDeskTabs button", "Playback");
      await sleep(120);
      results.push(await measure(client, `control-live-playback-${viewport.width}x${viewport.height}`));
      await clickVisibleByText(client, ".timelineDeskTabs button", "Show");
      if (shouldCaptureViewport(viewport)) {
        await captureSettledViewport(
          client,
          join(screenshotDir, `control-live-${viewport.width}x${viewport.height}.png`),
        );
      }
    }
  }
  await clickByText(client, "Touch");
  await sleep(180);
  await checkTouchMomentaryFlash(client);
  if (shouldCaptureViewport(viewport)) {
    mkdirSync(screenshotDir, { recursive: true });
    const screenshot = await client.send("Page.captureScreenshot", { format: "png", fromSurface: true });
    writeFileSync(join(screenshotDir, `touch-${viewport.width}x${viewport.height}.png`), screenshot.data, "base64");
  }
  results.push(await measure(client, `touch-${viewport.width}x${viewport.height}`));
  return results;
}

async function openCueFixture(client, viewport, fixture) {
  await client.send("Emulation.setDeviceMetricsOverride", {
    width: viewport.width,
    height: viewport.height,
    deviceScaleFactor: 1,
    mobile: false,
  });
  await client.send("Page.navigate", { url: fixtureUrl(fixture) });
  await waitForApp(client);
  await clickVisibleByText(client, ".workspaceTabs button", "Control");
  await clickVisibleByText(client, ".controlModeTabs button", "Timeline");
  await clickVisibleByText(client, ".timelineDeskTabs button", "Cues");
  await sleep(120);
  await clickVisibleByText(client, ".cuePanelEditToggle", "Edit Cues");
  await sleep(120);
}

async function runCueRecallViewport(client, viewport) {
  await openCueFixture(client, viewport, "cue-recall");
  await selectVisibleOption(client, "#cue-store-form select", "effects");
  await sleep(120);
  await clickVisibleByText(client, ".cueItem .cueEffectRecallEditor > summary", "Effect Recall");
  await sleep(80);
  await clickVisibleByText(client, ".cueItem .cueEffectRecallToolbar .buttonRow button", "Clear");
  await sleep(120);
  const actionReached = await client.evaluate(`(() => {
    const action = document.querySelector('.cueItem .cueSaveRecall');
    if (!action) return false;
    action.scrollIntoView({ block: 'center', inline: 'nearest' });
    return true;
  })()`);
  await sleep(120);
  const containment = await measure(client, `cue-recall-${viewport.width}x${viewport.height}`);
  const stats = await client.evaluate(`(() => {
    const panel = document.querySelector('.cuePanel');
    const panelRect = panel?.getBoundingClientRect();
    const actions = [...document.querySelectorAll('.cueItem .cueSaveDetails, .cueItem .cueSaveRecall, .cueItem .cueUpdateLook')];
    const fullyVisible = (element) => {
      if (!element || !panelRect) return false;
      const rect = element.getBoundingClientRect();
      const style = getComputedStyle(element);
      return style.display !== 'none' && style.visibility !== 'hidden' &&
        rect.width > 0 && rect.height > 0 &&
        rect.left >= Math.max(0, panelRect.left) && rect.right <= Math.min(innerWidth, panelRect.right) &&
        rect.top >= Math.max(0, panelRect.top) && rect.bottom <= Math.min(innerHeight, panelRect.bottom);
    };
    const saveDetails = document.querySelector('.cueItem .cueSaveDetails');
    const saveRecall = document.querySelector('.cueItem .cueSaveRecall');
    const updateLook = document.querySelector('.cueItem .cueUpdateLook');
    const recallDetails = document.querySelector('.cueItem .cueEffectRecallEditor');
    return {
      scope: document.querySelector('#cue-store-form select')?.value ?? '',
      recallDetailsOpen: Boolean(recallDetails?.open),
      recallCount: (recallDetails?.querySelector('summary small')?.textContent || '').trim(),
      saveDetailsLabel: (saveDetails?.textContent || '').trim(),
      saveRecallLabel: (saveRecall?.textContent || '').trim(),
      updateLookLabel: (updateLook?.textContent || '').trim(),
      saveDetailsDisabled: Boolean(saveDetails?.disabled),
      saveRecallDisabled: Boolean(saveRecall?.disabled),
      updateLookDisabled: Boolean(updateLook?.disabled),
      actionCount: actions.length,
      fullyVisibleActionCount: actions.filter(fullyVisible).length,
    };
  })()`);
  if (shouldCaptureViewport(viewport)) {
    mkdirSync(screenshotDir, { recursive: true });
    const screenshot = await client.send("Page.captureScreenshot", { format: "png", fromSurface: true });
    writeFileSync(
      join(screenshotDir, `cue-recall-open-${viewport.width}x${viewport.height}.png`),
      screenshot.data,
      "base64",
    );
  }
  const passed = Boolean(
    actionReached &&
    hasNoOuterOverflow(containment) &&
    stats.scope === "effects" &&
    stats.recallDetailsOpen &&
    stats.recallCount === "0 / 1" &&
    stats.saveDetailsLabel === "Save Details" &&
    stats.saveRecallLabel === "Save Recall" &&
    stats.updateLookLabel === "Update Look" &&
    !stats.saveDetailsDisabled &&
    !stats.saveRecallDisabled &&
    !stats.updateLookDisabled &&
    stats.actionCount === 3 &&
    stats.fullyVisibleActionCount === 3
  );
  return { label: `cue-recall-${viewport.width}x${viewport.height}`, passed, containment, stats };
}

async function runCueNodeGraphViewport(client, viewport) {
  await openCueFixture(client, viewport, "cue-node-graph");
  const allScope = await client.evaluate(`(() => {
    const store = document.querySelector('#cue-store-form button.primary');
    return {
      scope: document.querySelector('#cue-store-form select')?.value ?? '',
      storeDisabled: Boolean(store?.disabled),
      scopeErrorCount: document.querySelectorAll('.cueScopeHint.invalid').length,
    };
  })()`);
  await selectVisibleOption(client, "#cue-store-form select", "video");
  await sleep(120);
  const containment = await measure(client, `cue-node-graph-${viewport.width}x${viewport.height}`);
  const videoScope = {
    scope: containment.cueCaptureScopeValue,
    storeDisabled: containment.cueStoreButtonDisabled,
    scopeErrorCount: containment.visibleCueScopeErrorCount,
    graphCount: containment.cuePreviewStatValues[4] ?? 0,
  };
  const passed = Boolean(
    hasNoOuterOverflow(containment) &&
    allScope.scope === "all" &&
    !allScope.storeDisabled &&
    allScope.scopeErrorCount === 0 &&
    videoScope.scope === "video" &&
    !videoScope.storeDisabled &&
    videoScope.scopeErrorCount === 0 &&
    videoScope.graphCount === 1
  );
  return { label: `cue-node-graph-${viewport.width}x${viewport.height}`, passed, containment, allScope, videoScope };
}

async function runCueRecallLargeViewport(client, viewport) {
  await openCueFixture(client, viewport, "cue-recall-large");
  const before = await client.evaluate(`(() => ({
    cueCount: document.querySelectorAll('.cueItem').length,
    captureRows: document.querySelectorAll('#cue-effect-capture-editor .cueEffectRecallRow').length,
    cueRows: document.querySelectorAll('.cueItem .cueEffectRecallRow').length,
    closedCueEditors: [...document.querySelectorAll('.cueItem .cueEffectRecallEditor')].filter((editor) => !editor.open).length,
    filterCount: document.querySelectorAll('#cue-effect-capture-editor .cueEffectRecallFilter').length,
  }))()`);
  await clickVisibleByText(client, ".cueItem .cueEffectRecallEditor > summary", "Effect Recall");
  await sleep(120);
  const after = await client.evaluate(`(() => ({
    captureRows: document.querySelectorAll('#cue-effect-capture-editor .cueEffectRecallRow').length,
    cueRows: document.querySelectorAll('.cueItem .cueEffectRecallRow').length,
    openCueEditors: [...document.querySelectorAll('.cueItem .cueEffectRecallEditor')].filter((editor) => editor.open).length,
    filterCount: document.querySelectorAll('.cueItem .cueEffectRecallFilter').length,
  }))()`);
  const containment = await measure(client, `cue-recall-large-${viewport.width}x${viewport.height}`);
  const passed = Boolean(
    hasNoOuterOverflow(containment) &&
    before.cueCount === 12 &&
    before.captureRows === 48 &&
    before.cueRows === 0 &&
    before.closedCueEditors === 12 &&
    before.filterCount === 1 &&
    after.captureRows === 48 &&
    after.cueRows === 48 &&
    after.openCueEditors === 1 &&
    after.filterCount === 1
  );
  return { label: `cue-recall-large-${viewport.width}x${viewport.height}`, passed, containment, before, after };
}

async function runEffectStackLargeViewport(client, viewport) {
  await client.send("Emulation.setDeviceMetricsOverride", {
    width: viewport.width,
    height: viewport.height,
    deviceScaleFactor: 1,
    mobile: false,
  });
  await client.send("Page.navigate", { url: fixtureUrl("cue-recall-large") });
  await waitForApp(client);
  await clickVisibleByText(client, ".workspaceTabs button", "Control");
  await clickVisibleByText(client, ".controlModeTabs button", "Live Edit");
  await clickVisibleByText(client, ".editDeskTabs button", "Effects");
  await sleep(160);
  const stats = await client.evaluate(`(async () => {
    const rows = () => [...document.querySelectorAll('.effectListRows .effectItem')];
    const list = document.querySelector('.effectListRows');
    const beforeCount = rows().length;
    const total = Number(list?.getAttribute('aria-rowcount') || 0);
    for (let page = 0; page < 80; page += 1) {
      const next = [...document.querySelectorAll('.effectListPager button')]
        .find((button) => (button.textContent || '').trim() === 'Next');
      if (!next || next.disabled) break;
      next.click();
      await Promise.resolve();
    }
    await new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame)));
    const afterRows = rows();
    return {
      total,
      beforeCount,
      afterCount: afterRows.length,
      pagerCount: document.querySelectorAll('.effectListPager').length,
      pageLabel: (document.querySelector('.effectListPager span')?.textContent || '').trim(),
      reachedLastEffect: afterRows.some((row) => (row.textContent || '').includes('Viewport Effect 500')),
    };
  })()`);
  const containment = await measure(client, `effect-stack-large-${viewport.width}x${viewport.height}`);
  const passed = Boolean(
    stats &&
    stats.total === 500 &&
    stats.beforeCount <= 10 &&
    stats.afterCount <= 10 &&
    stats.pagerCount === 1 &&
    stats.pageLabel.includes('50 / 50') &&
    stats.reachedLastEffect &&
    hasNoOuterOverflow(containment) &&
    containment.controlWorkSurfaceUnsafeOverflowCount === 0
  );
  return { label: `effect-stack-large-${viewport.width}x${viewport.height}`, passed, containment, stats };
}

async function openTimelineShowFixture(client, viewport, fixture) {
  await client.send("Emulation.setDeviceMetricsOverride", {
    width: viewport.width,
    height: viewport.height,
    deviceScaleFactor: 1,
    mobile: false,
  });
  await client.send("Page.navigate", { url: fixtureUrl(fixture) });
  await waitForApp(client);
  await clickVisibleByText(client, ".workspaceTabs button", "Control");
  await clickVisibleByText(client, ".controlModeTabs button", "Timeline");
  await clickVisibleByText(client, ".timelineDeskTabs button", "Show");
  await sleep(180);
}

async function runSceneBlockHourViewport(client, viewport) {
  await openTimelineShowFixture(client, viewport, "scene-block-hour");

  const initialStats = await evaluatePageFunction(client, () => {
    const range = document.querySelector(".timelineVisibleRange");
    const overview = document.querySelector(".timelineOverview");
    const overviewRect = overview?.getBoundingClientRect();
    return {
      showDurationMs: Number(range?.getAttribute("data-show-duration-ms")),
      editExtentMs: Number(range?.getAttribute("data-edit-extent-ms")),
      visibleStartMs: Number(range?.getAttribute("data-visible-start-ms")),
      visibleEndMs: Number(range?.getAttribute("data-visible-end-ms")),
      toolbarButtonCount: document.querySelectorAll(".timelineViewportToolbar button").length,
      rulerTickCount: document.querySelectorAll("[data-timeline-ruler-ms]").length,
      overviewNodeCount: document.querySelectorAll(".timelineOverview *").length,
      overviewWidth: overviewRect?.width ?? 0,
      initialMarkerCount: document.querySelectorAll(".timelineMarker.sceneBlock").length,
    };
  });
  const initialRulerBounds = await readTimelineRulerBounds(client);

  const selectedByRow = await evaluatePageFunction(client, async () => {
    const search = document.querySelector(".sceneBlockRowSearch");
    if (!(search instanceof HTMLInputElement)) return false;
    search.value = "500";
    search.dispatchEvent(new InputEvent("input", { bubbles: true, inputType: "insertText", data: "500" }));
    await new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame)));
    const row = document.querySelector('.sceneBlockRow[data-scene-block-id="500"]');
    if (!(row instanceof HTMLElement)) return false;
    row.click();
    await new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(() => requestAnimationFrame(resolveFrame))));
    return true;
  });

  const revealStats = await evaluatePageFunction(client, () => {
    const range = document.querySelector(".timelineVisibleRange");
    const marker = document.querySelector('.timelineMarker.sceneBlock[data-timeline-event-id="500"]');
    const body = marker?.querySelector(".timelineSceneBlockBody");
    const bodyRect = body?.getBoundingClientRect();
    const svgRect = marker?.ownerSVGElement?.getBoundingClientRect();
    return {
      visibleStartMs: Number(range?.getAttribute("data-visible-start-ms")),
      visibleEndMs: Number(range?.getAttribute("data-visible-end-ms")),
      editExtentMs: Number(range?.getAttribute("data-edit-extent-ms")),
      terminalMarkerRendered: Boolean(marker),
      terminalMarkerSelected: marker?.classList.contains("selected") ?? false,
      terminalMarkerStartMs: Number(marker?.getAttribute("data-timeline-start-ms")),
      terminalMarkerWidth: bodyRect?.width ?? 0,
      terminalMarkerCenterX: bodyRect ? bodyRect.left + bodyRect.width / 2 : 0,
      overviewWidth: svgRect?.width ?? 0,
      firstMarkerRendered: Boolean(document.querySelector('.timelineMarker[data-timeline-event-id="1"]')),
      playheadRendered: Boolean(document.querySelector(".timelinePlayhead")),
      rulerTickCount: document.querySelectorAll("[data-timeline-ruler-ms]").length,
      overviewNodeCount: document.querySelectorAll(".timelineOverview *").length,
    };
  });
  const revealRulerBounds = await readTimelineRulerBounds(client);

  const dragGeometry = await evaluatePageFunction(client, () => {
    const marker = document.querySelector('.timelineMarker.sceneBlock[data-timeline-event-id="500"]');
    const body = marker?.querySelector(".timelineSceneBlockBody");
    const range = document.querySelector(".timelineVisibleRange");
    const bodyRect = body?.getBoundingClientRect();
    const svgRect = marker?.ownerSVGElement?.getBoundingClientRect();
    if (!marker || !bodyRect || !svgRect) return null;
    return {
      x: bodyRect.left + bodyRect.width / 2,
      y: bodyRect.top + bodyRect.height / 2,
      bodyLeft: bodyRect.left,
      svgWidth: svgRect.width,
      startMs: Number(marker.getAttribute("data-timeline-start-ms")),
      previewStartMs: Number(marker.getAttribute("data-timeline-preview-start-ms")),
      visibleStartMs: Number(range?.getAttribute("data-visible-start-ms")),
      visibleEndMs: Number(range?.getAttribute("data-visible-end-ms")),
    };
  });
  let dragStats = null;
  if (dragGeometry) {
    const markerSnapshot = () => evaluatePageFunction(client, () => {
      const marker = document.querySelector('.timelineMarker.sceneBlock[data-timeline-event-id="500"]');
      const bodyRect = marker?.querySelector(".timelineSceneBlockBody")?.getBoundingClientRect();
      return {
        bodyLeft: bodyRect?.left ?? null,
        bodyCenterX: bodyRect ? bodyRect.left + bodyRect.width / 2 : null,
        startMs: Number(marker?.getAttribute("data-timeline-start-ms")),
        previewStartMs: Number(marker?.getAttribute("data-timeline-preview-start-ms")),
      };
    });
    await client.send("Input.dispatchMouseEvent", {
      type: "mousePressed",
      x: dragGeometry.x,
      y: dragGeometry.y,
      button: "left",
      buttons: 1,
      clickCount: 1,
    });
    await client.send("Input.dispatchMouseEvent", {
      type: "mouseMoved",
      x: dragGeometry.x + 2,
      y: dragGeometry.y,
      button: "left",
      buttons: 1,
    });
    const tiny = await markerSnapshot();
    await client.send("Input.dispatchMouseEvent", {
      type: "mouseMoved",
      x: dragGeometry.x + 60,
      y: dragGeometry.y,
      button: "left",
      buttons: 1,
    });
    const moved = await markerSnapshot();
    await client.send("Input.dispatchMouseEvent", {
      type: "mouseReleased",
      x: dragGeometry.x + 60,
      y: dragGeometry.y,
      button: "left",
      buttons: 0,
      clickCount: 1,
    });
    await sleep(80);
    await evaluatePageFunction(client, () =>
      new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame))),
    );
    const settled = await markerSnapshot();
    const spanMs = dragGeometry.visibleEndMs - dragGeometry.visibleStartMs;
    dragStats = {
      ...dragGeometry,
      tiny,
      moved,
      settled,
      expectedDeltaMs: (60 / dragGeometry.svgWidth) * spanMs,
      actualPreviewDeltaMs: moved.previewStartMs - dragGeometry.previewStartMs,
    };
  }

  const zoomStats = await evaluatePageFunction(client, async () => {
    const read = () => {
      const range = document.querySelector(".timelineVisibleRange");
      const marker = document.querySelector('.timelineMarker.sceneBlock[data-timeline-event-id="500"]');
      const bodyRect = marker?.querySelector(".timelineSceneBlockBody")?.getBoundingClientRect();
      return {
        startMs: Number(range?.getAttribute("data-visible-start-ms")),
        endMs: Number(range?.getAttribute("data-visible-end-ms")),
        markerCenterX: bodyRect ? bodyRect.left + bodyRect.width / 2 : null,
      };
    };
    const before = read();
    const zoomOut = [...document.querySelectorAll(".timelineViewportToolbar button")]
      .find((button) => (button.textContent || "").trim() === "Zoom Out");
    const zoomOutClicked = zoomOut instanceof HTMLButtonElement && !zoomOut.disabled;
    if (zoomOutClicked) zoomOut.click();
    await new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame)));
    return { zoomOutClicked, before, after: read() };
  });

  const navigationStats = await evaluatePageFunction(client, async () => {
    const button = (label) => [...document.querySelectorAll(".timelineViewportToolbar button")]
      .find((candidate) => (candidate.textContent || "").trim() === label);
    const clickAndSettle = async (label) => {
      const target = button(label);
      if (!(target instanceof HTMLButtonElement) || target.disabled) return false;
      target.click();
      await new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame)));
      return true;
    };
    const read = () => {
      const range = document.querySelector(".timelineVisibleRange");
      return {
        startMs: Number(range?.getAttribute("data-visible-start-ms")),
        endMs: Number(range?.getAttribute("data-visible-end-ms")),
        showDurationMs: Number(range?.getAttribute("data-show-duration-ms")),
        editExtentMs: Number(range?.getAttribute("data-edit-extent-ms")),
      };
    };
    const playheadTitle = document.querySelector(".timelineTimeStat")?.getAttribute("title") ?? "";
    const playheadMatch = playheadTitle.match(/^\s*(-?\d+(?:\.\d+)?)/);
    const playheadMs = playheadMatch ? Number(playheadMatch[1]) : Number.NaN;
    const beforePanNext = read();
    const panNextClicked = await clickAndSettle("Pan Next");
    const afterPanNext = read();
    const panPreviousClicked = await clickAndSettle("Pan Prev");
    const afterPanPrevious = read();
    const zoomInClicked = await clickAndSettle("Zoom In");
    const afterZoomIn = read();
    const fitAllClicked = await clickAndSettle("Fit All");
    const afterFit = read();
    const revealSelectedClicked = await clickAndSettle("Reveal Selected");
    const afterRevealSelected = read();
    const revealPlayheadClicked = await clickAndSettle("Reveal Playhead");
    const afterRevealPlayhead = read();
    return {
      playheadMs,
      panNextClicked,
      panPreviousClicked,
      zoomInClicked,
      fitAllClicked,
      revealSelectedClicked,
      revealPlayheadClicked,
      beforePanNext,
      afterPanNext,
      afterPanPrevious,
      afterZoomIn,
      afterFit,
      afterRevealSelected,
      afterRevealPlayhead,
      rulerTickCount: document.querySelectorAll("[data-timeline-ruler-ms]").length,
    };
  });
  await openTimelineShowFixture(client, viewport, "scene-block-hour");
  await evaluatePageFunction(client, async () => {
    const search = document.querySelector(".sceneBlockRowSearch");
    if (!(search instanceof HTMLInputElement)) return;
    search.value = "500";
    search.dispatchEvent(new InputEvent("input", { bubbles: true, inputType: "insertText", data: "500" }));
    await new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame)));
    document.querySelector('.sceneBlockRow[data-scene-block-id="500"]')?.click();
    await new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame)));
  });
  await clickVisibleByText(client, ".timelineViewportToolbar button", "Reveal Selected");
  await sleep(80);
  const finalRevealStats = await evaluatePageFunction(client, () => {
    const range = document.querySelector(".timelineVisibleRange");
    const bodyRect = document.querySelector(
      '.timelineMarker.sceneBlock[data-timeline-event-id="500"] .timelineSceneBlockBody',
    )?.getBoundingClientRect();
    return {
      startMs: Number(range?.getAttribute("data-visible-start-ms")),
      endMs: Number(range?.getAttribute("data-visible-end-ms")),
      terminalMarkerWidth: bodyRect?.width ?? 0,
      rulerTickCount: document.querySelectorAll("[data-timeline-ruler-ms]").length,
    };
  });
  const finalRulerBounds = await readTimelineRulerBounds(client);
  let screenshotVerification = null;
  if (shouldCaptureViewport(viewport)) {
    mkdirSync(screenshotDir, { recursive: true });
    screenshotVerification = await captureSettledViewport(
      client,
      join(screenshotDir, "scene-block-hour-" + viewport.width + "x" + viewport.height + ".png"),
    );
  }
  const containment = await measure(client, "scene-block-hour-" + viewport.width + "x" + viewport.height);
  const passed = Boolean(
    selectedByRow &&
    initialStats.showDurationMs === 3_600_000 &&
    initialStats.editExtentMs >= 3_605_000 &&
    initialStats.visibleStartMs === 0 &&
    initialStats.visibleEndMs === 3_600_000 &&
    initialStats.initialMarkerCount === 500 &&
    initialStats.toolbarButtonCount === 7 &&
    initialStats.rulerTickCount > 1 &&
    initialStats.rulerTickCount <= 64 &&
    initialRulerBounds.count === initialStats.rulerTickCount &&
    initialRulerBounds.contained &&
    initialStats.overviewNodeCount < 3_500 &&
    revealStats.visibleStartMs === 3_597_000 &&
    revealStats.visibleEndMs === 3_602_000 &&
    revealStats.terminalMarkerRendered &&
    revealStats.terminalMarkerSelected &&
    revealStats.terminalMarkerStartMs === 3_599_000 &&
    revealStats.terminalMarkerWidth >= 32 &&
    !revealStats.firstMarkerRendered &&
    !revealStats.playheadRendered &&
    revealStats.rulerTickCount > 1 &&
    revealStats.rulerTickCount <= 64 &&
    revealRulerBounds.count === revealStats.rulerTickCount &&
    revealRulerBounds.contained &&
    revealStats.overviewNodeCount < 3_500 &&
    dragStats !== null &&
    dragStats.expectedDeltaMs > 0 &&
    dragStats.expectedDeltaMs < dragStats.visibleEndMs - dragStats.visibleStartMs &&
    dragStats.tiny.previewStartMs === dragStats.previewStartMs &&
    Math.abs(dragStats.tiny.bodyLeft - dragStats.bodyLeft) < 1 &&
    Math.abs(dragStats.actualPreviewDeltaMs - dragStats.expectedDeltaMs) <= 2 &&
    Math.abs(dragStats.moved.bodyLeft - (dragStats.bodyLeft + 60)) <= 2 &&
    dragStats.settled.startMs > 3_599_000 &&
    Math.abs(dragStats.settled.startMs - dragStats.moved.previewStartMs) <= 2 &&
    Math.abs(dragStats.settled.startMs - (dragStats.startMs + dragStats.expectedDeltaMs)) <= 2 &&
    Math.abs(dragStats.settled.bodyLeft - dragStats.moved.bodyLeft) <= 2 &&
    zoomStats.zoomOutClicked &&
    zoomStats.before.markerCenterX !== null &&
    zoomStats.after.markerCenterX !== null &&
    Math.abs(zoomStats.after.markerCenterX - zoomStats.before.markerCenterX) <= 2 &&
    zoomStats.after.endMs - zoomStats.after.startMs > zoomStats.before.endMs - zoomStats.before.startMs &&
    navigationStats.panNextClicked &&
    navigationStats.panPreviousClicked &&
    navigationStats.zoomInClicked &&
    navigationStats.fitAllClicked &&
    navigationStats.revealSelectedClicked &&
    navigationStats.revealPlayheadClicked &&
    navigationStats.afterPanNext.startMs > navigationStats.beforePanNext.startMs &&
    navigationStats.afterPanNext.endMs <= navigationStats.afterPanNext.editExtentMs &&
    navigationStats.afterPanPrevious.startMs < navigationStats.afterPanNext.startMs &&
    navigationStats.afterPanPrevious.endMs - navigationStats.afterPanPrevious.startMs ===
      navigationStats.afterPanNext.endMs - navigationStats.afterPanNext.startMs &&
    navigationStats.afterZoomIn.endMs - navigationStats.afterZoomIn.startMs <
      navigationStats.afterPanPrevious.endMs - navigationStats.afterPanPrevious.startMs &&
    navigationStats.afterFit.startMs === 0 &&
    navigationStats.afterFit.endMs === navigationStats.afterFit.showDurationMs &&
    navigationStats.afterRevealSelected.endMs - navigationStats.afterRevealSelected.startMs === 5_000 &&
    navigationStats.afterRevealPlayhead.endMs - navigationStats.afterRevealPlayhead.startMs === 5_000 &&
    Number.isFinite(navigationStats.playheadMs) &&
    Math.abs(
      (navigationStats.afterRevealPlayhead.startMs + navigationStats.afterRevealPlayhead.endMs) / 2 -
      navigationStats.playheadMs
    ) <= 1 &&
    navigationStats.afterRevealPlayhead.startMs !== navigationStats.afterRevealSelected.startMs &&
    navigationStats.afterRevealPlayhead.endMs !== navigationStats.afterRevealSelected.endMs &&
    navigationStats.rulerTickCount > 1 &&
    navigationStats.rulerTickCount <= 64 &&
    finalRevealStats.startMs === 3_597_000 &&
    finalRevealStats.endMs === 3_602_000 &&
    finalRevealStats.terminalMarkerWidth >= 32 &&
    finalRevealStats.rulerTickCount > 1 &&
    finalRevealStats.rulerTickCount <= 64 &&
    finalRulerBounds.count === finalRevealStats.rulerTickCount &&
    finalRulerBounds.contained &&
    (!shouldCaptureViewport(viewport) || screenshotVerification?.verified === true) &&
    hasNoOuterOverflow(containment)
  );
  return {
    label: "scene-block-hour-" + viewport.width + "x" + viewport.height,
    passed,
    initialStats,
    initialRulerBounds,
    revealStats,
    revealRulerBounds,
    dragStats,
    zoomStats,
    navigationStats,
    finalRevealStats,
    finalRulerBounds,
    containment,
    screenshotVerification,
  };
}

async function runSceneBlockOverlapTrack(client, viewport, track, expectedIds, activationKind) {
  await openTimelineShowFixture(client, viewport, "scene-block-large");
  const initialStats = await evaluatePageFunction(client, (wantedTrack, wantedIds) => {
    const badges = [...document.querySelectorAll(".timelineOverlapCluster")];
    const badge = badges.find((candidate) => candidate.getAttribute("data-overlap-track") === wantedTrack);
    const rect = badge?.querySelector("rect")?.getBoundingClientRect();
    const badgeRect = badge?.getBoundingClientRect();
    const members = (badge?.getAttribute("data-overlap-members") || "")
      .split(",")
      .filter(Boolean)
      .map(Number);
    const hit = badgeRect
      ? document.elementFromPoint(
          badgeRect.left + badgeRect.width / 2,
          badgeRect.top + badgeRect.height / 2,
        )?.closest(".timelineOverlapCluster")
      : null;
    const marker = document.querySelector(
      '.timelineMarker.sceneBlock[data-timeline-event-id="' + wantedIds.at(-1) + '"]',
    );
    const markerBodyRect = marker?.querySelector(".timelineSceneBlockBody")?.getBoundingClientRect();
    const markerTitle = (marker?.querySelector("title")?.textContent || "").trim();
    const markerLoopCount = Number(marker?.getAttribute("data-timeline-loop-count"));
    const badgeStartMs = Number(badge?.getAttribute("data-overlap-start-ms"));
    const badgeEndMs = Number(badge?.getAttribute("data-overlap-end-ms"));
    const badgeTimeSpan = `${badgeStartMs} to ${badgeEndMs} ms`;
    const badgeAriaLabel = badge?.getAttribute("aria-label") ?? "";
    const badgeTitle = (badge?.querySelector("title")?.textContent || "").trim();
    const markerHit = markerBodyRect
      ? document.elementFromPoint(
          markerBodyRect.left + markerBodyRect.width / 2,
          markerBodyRect.top + markerBodyRect.height / 2,
        )?.closest(".timelineMarker")
      : null;
    return {
      badgeCount: badges.length,
      lightingCount: Number(
        badges.find((candidate) => candidate.getAttribute("data-overlap-track") === "Lighting")
          ?.getAttribute("data-overlap-count"),
      ),
      videoCount: Number(
        badges.find((candidate) => candidate.getAttribute("data-overlap-track") === "Video")
          ?.getAttribute("data-overlap-count"),
      ),
      members,
      membershipExact: members.length === wantedIds.length &&
        new Set(members).size === wantedIds.length &&
        wantedIds.every((memberId) => members.includes(memberId)),
      ariaLabel: badgeAriaLabel,
      title: badgeTitle,
      startMs: badgeStartMs,
      endMs: badgeEndMs,
      ariaHasTimeSpan: badgeAriaLabel.includes(badgeTimeSpan),
      titleHasTimeSpan: badgeTitle.includes(badgeTimeSpan),
      role: badge?.getAttribute("role") ?? "",
      tabIndex: badge?.getAttribute("tabindex") ?? "",
      hitTrack: hit?.getAttribute("data-overlap-track") ?? "",
      badgeWidth: rect?.width ?? 0,
      badgeHeight: rect?.height ?? 0,
      markerHitId: markerHit?.getAttribute("data-timeline-event-id") ?? "",
      markerLoopCount,
      markerTitle,
      visibleBlockLabelsHaveIterationSuffix: [...document.querySelectorAll(".timelineSceneBlockLabel")]
        .some((label) => /(?:\bx|×)\d+\s*$/.test((label.textContent || "").trim())),
      markerCount: document.querySelectorAll(".timelineMarker.sceneBlock").length,
      overviewNodeCount: document.querySelectorAll(".timelineOverview *").length,
      rulerTickCount: document.querySelectorAll("[data-timeline-ruler-ms]").length,
    };
  }, track, expectedIds);

  await evaluatePageFunction(client, async () => {
    if (typeof window.__syndocalPauseSceneBlockFixtureChurn === "function") {
      window.__syndocalPauseSceneBlockFixtureChurn();
    }
    await new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame)));
  });
  const keyboardAccessStats = await focusTimelineOverlapClusterWithTab(client, track);
  await evaluatePageFunction(client, () => {
    const overview = document.querySelector(".timelineOverview");
    overview?.removeAttribute("data-overlap-state-handler-ms");
    overview?.removeAttribute("data-overlap-state-track");
    overview?.removeAttribute("data-overlap-state-microtask-ms");
  });
  if (activationKind === "keyboard") {
    await pressKey(client, "Enter");
  } else {
    const badgeGeometry = await evaluatePageFunction(client, (wantedTrack) => {
      const badge = [...document.querySelectorAll(".timelineOverlapCluster")]
        .find((candidate) => candidate.getAttribute("data-overlap-track") === wantedTrack);
      const rect = badge?.getBoundingClientRect();
      return rect ? { x: rect.left + rect.width / 2, y: rect.top + rect.height / 2 } : null;
    }, track);
    if (badgeGeometry) {
      await client.send("Input.dispatchMouseEvent", {
        type: "mousePressed",
        x: badgeGeometry.x,
        y: badgeGeometry.y,
        button: "left",
        buttons: 1,
        clickCount: 1,
      });
      await client.send("Input.dispatchMouseEvent", {
        type: "mouseReleased",
        x: badgeGeometry.x,
        y: badgeGeometry.y,
        button: "left",
        buttons: 0,
        clickCount: 1,
      });
    }
  }
  let stateActivationTimingStats = null;
  for (let attempt = 0; attempt < 20; attempt += 1) {
    stateActivationTimingStats = await evaluatePageFunction(client, (wantedTrack) => {
      const overview = document.querySelector(".timelineOverview");
      const stateTrack = overview?.getAttribute("data-overlap-state-track") ?? "";
      const handlerValue = overview?.getAttribute("data-overlap-state-handler-ms") ?? null;
      const microtaskValue = overview?.getAttribute("data-overlap-state-microtask-ms") ?? null;
      const stateHandlerElapsedMs = handlerValue === null ? null : Number(handlerValue);
      const stateMicrotaskElapsedMs = microtaskValue === null ? null : Number(microtaskValue);
      return {
        stateTrack,
        stateHandlerElapsedMs: Number.isFinite(stateHandlerElapsedMs) ? stateHandlerElapsedMs : null,
        stateMicrotaskElapsedMs: Number.isFinite(stateMicrotaskElapsedMs) ? stateMicrotaskElapsedMs : null,
        committedFilterVisible: stateTrack === wantedTrack && Boolean(
          document.querySelector(".sceneBlockOverlapFilter"),
        ),
      };
    }, track);
    if (stateActivationTimingStats?.stateMicrotaskElapsedMs !== null) break;
    await sleep(25);
  }
  // The two frames below are a correctness boundary for DOM reads only. Their
  // elapsed time is deliberately not measured; only resulting state is asserted.
  const postFrameCorrectnessStats = await evaluatePageFunction(client, async (wantedTrack, wantedIds) => {
    await new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame)));
    const rows = [...document.querySelectorAll(".sceneBlockRow")];
    const rowIds = rows.map((row) => Number(row.getAttribute("data-scene-block-id")));
    const pagerLabel = (document.querySelector(".sceneBlockPager span")?.textContent || "").trim();
    const filter = document.querySelector(".sceneBlockOverlapFilter");
    const range = document.querySelector(".timelineVisibleRange");
    const beforePanStartMs = Number(range?.getAttribute("data-visible-start-ms"));
    const panNext = [...document.querySelectorAll(".timelineViewportToolbar button")]
      .find((button) => (button.textContent || "").trim() === "Pan Next");
    panNext?.click();
    await new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame)));
    const afterPanStartMs = Number(range?.getAttribute("data-visible-start-ms"));
    const badgeAfterPan = [...document.querySelectorAll(".timelineOverlapCluster")]
      .find((candidate) => candidate.getAttribute("data-overlap-track") === wantedTrack);
    const membersAfterPan = (badgeAfterPan?.getAttribute("data-overlap-members") || "")
      .split(",")
      .filter(Boolean)
      .map(Number);
    const filterPersistedAfterPan = Boolean(document.querySelector(".sceneBlockOverlapFilter"));
    const fitAll = [...document.querySelectorAll(".timelineViewportToolbar button")]
      .find((button) => (button.textContent || "").trim() === "Fit All");
    fitAll?.click();
    await new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame)));
    const pagerBeforeLiveFollow = (document.querySelector(".sceneBlockPager span")?.textContent || "").trim();
    if (typeof window.__syndocalSetSceneBlockFixtureState === "function") {
      window.__syndocalSetSceneBlockFixtureState(1000, true);
      await new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame)));
    }
    const pagerAfterLiveFollow = (document.querySelector(".sceneBlockPager span")?.textContent || "").trim();
    const filterPersistedDuringLiveFollow = Boolean(document.querySelector(".sceneBlockOverlapFilter"));
    if (typeof window.__syndocalSetSceneBlockFixtureState === "function") {
      window.__syndocalSetSceneBlockFixtureState(1000, false);
      await new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame)));
    }
    return {
      filterText: (filter?.textContent || "").trim(),
      pagerLabel,
      rowCount: rows.length,
      rowIds,
      rowsBelongToCluster: rowIds.every((rowId) => wantedIds.includes(rowId)),
      beforePanStartMs,
      afterPanStartMs,
      filterPersistedAfterPan,
      badgePersistedAfterPan: Boolean(badgeAfterPan),
      membershipPersistedAfterPan: membersAfterPan.length === wantedIds.length &&
        new Set(membersAfterPan).size === wantedIds.length &&
        wantedIds.every((memberId) => membersAfterPan.includes(memberId)),
      pagerBeforeLiveFollow,
      pagerAfterLiveFollow,
      filterPersistedDuringLiveFollow,
    };
  }, track, expectedIds);
  const activationStats = postFrameCorrectnessStats === null || stateActivationTimingStats === null
    ? null
    : { ...stateActivationTimingStats, ...postFrameCorrectnessStats, keyboardAccessStats };

  const probeIds = track === "Lighting" ? [1, 251, 499] : [2, 250, 500];
  const searchStats = await evaluatePageFunction(client, async (wantedIds) => {
    const search = document.querySelector(".sceneBlockRowSearch");
    if (!(search instanceof HTMLInputElement)) return null;
    const foundIds = [];
    const setSearch = async (value) => {
      search.value = value;
      search.dispatchEvent(new InputEvent("input", { bubbles: true, inputType: "insertText", data: value }));
      await new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame)));
    };
    for (const eventId of wantedIds) {
      await setSearch(String(eventId));
      const row = document.querySelector('.sceneBlockRow[data-scene-block-id="' + eventId + '"]');
      if (row) foundIds.push(eventId);
      if (eventId === wantedIds[1]) {
        row?.click();
        await new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame)));
      }
    }
    const selectedMarkerId = document.querySelector(".timelineMarker.sceneBlock.selected")
      ?.getAttribute("data-timeline-event-id") ?? "";
    const lastMarkerId = [...document.querySelectorAll(".timelineMarker.sceneBlock")]
      .at(-1)?.getAttribute("data-timeline-event-id") ?? "";
    await setSearch("");
    const clearFilter = [...document.querySelectorAll(".sceneBlockOverlapFilter button")]
      .find((button) => (button.textContent || "").includes("Clear overlap filter"));
    clearFilter?.click();
    await new Promise((resolveFrame) =>
      requestAnimationFrame(() => requestAnimationFrame(() => requestAnimationFrame(resolveFrame))),
    );
    const selectedRow = document.querySelector(
      '.sceneBlockRow.selected[data-scene-block-id="' + wantedIds[1] + '"]',
    );
    const listRect = document.querySelector(".sceneBlockList")?.getBoundingClientRect();
    const selectedRowRect = selectedRow?.getBoundingClientRect();
    const result = {
      foundIds,
      selectedMarkerId,
      lastMarkerId,
      filterCleared: !document.querySelector(".sceneBlockOverlapFilter"),
      pagerLabelAfterClear: (document.querySelector(".sceneBlockPager span")?.textContent || "").trim(),
      rowCountAfterClear: document.querySelectorAll(".sceneBlockRow").length,
      selectedRowVisibleAfterClear: Boolean(
        selectedRowRect && listRect &&
        selectedRowRect.top >= listRect.top - 1 &&
        selectedRowRect.bottom <= listRect.bottom + 1,
      ),
      selectedRowIdAfterClear: selectedRow?.getAttribute("data-scene-block-id") ?? "",
      lastMarkerIdAfterClear: [...document.querySelectorAll(".timelineMarker.sceneBlock")]
        .at(-1)?.getAttribute("data-timeline-event-id") ?? "",
      overviewNodeCountAfterClear: document.querySelectorAll(".timelineOverview *").length,
    };
    await setSearch(String(wantedIds[1]));
    const pagerBeforeSearchLive = (document.querySelector(".sceneBlockPager span")?.textContent || "").trim();
    if (typeof window.__syndocalSetSceneBlockFixtureState === "function") {
      window.__syndocalSetSceneBlockFixtureState(1000, true);
      await new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame)));
    }
    result.searchGuardPassed =
      (document.querySelector(".sceneBlockPager span")?.textContent || "").trim() === pagerBeforeSearchLive &&
      Boolean(document.querySelector('.sceneBlockRow[data-scene-block-id="' + wantedIds[1] + '"]'));
    if (typeof window.__syndocalSetSceneBlockFixtureState === "function") {
      window.__syndocalSetSceneBlockFixtureState(1000, false);
      await new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame)));
    }
    await setSearch("");
    const dirtyRow = document.querySelector(".sceneBlockRow");
    const dirtyStartInput = dirtyRow?.querySelector('.sceneBlockRowFields input[type="number"]');
    const originalDirtyStart = dirtyStartInput instanceof HTMLInputElement ? dirtyStartInput.value : "";
    if (dirtyStartInput instanceof HTMLInputElement) {
      dirtyStartInput.value = String(Number(originalDirtyStart) + 1);
      dirtyStartInput.dispatchEvent(new InputEvent("input", { bubbles: true, inputType: "insertText" }));
      await new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame)));
    }
    const pagerBeforeDirtyLive = (document.querySelector(".sceneBlockPager span")?.textContent || "").trim();
    if (typeof window.__syndocalSetSceneBlockFixtureState === "function") {
      window.__syndocalSetSceneBlockFixtureState(1000, true);
      await new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame)));
    }
    result.dirtyGuardPassed =
      dirtyRow?.classList.contains("dirty") === true &&
      (document.querySelector(".sceneBlockPager span")?.textContent || "").trim() === pagerBeforeDirtyLive;
    if (typeof window.__syndocalSetSceneBlockFixtureState === "function") {
      window.__syndocalSetSceneBlockFixtureState(1000, false);
    }
    if (dirtyStartInput instanceof HTMLInputElement) {
      dirtyStartInput.value = originalDirtyStart;
      dirtyStartInput.dispatchEvent(new InputEvent("input", { bubbles: true, inputType: "insertText" }));
      await new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame)));
    }
    return result;
  }, probeIds);
  const containment = await measure(client, "scene-block-overlap-" + track.toLowerCase() + "-" + viewport.width + "x" + viewport.height);
  const passed = Boolean(
    initialStats.badgeCount === 2 &&
    initialStats.lightingCount === 250 &&
    initialStats.videoCount === 250 &&
    initialStats.membershipExact &&
    initialStats.ariaLabel.includes("250") &&
    Number.isFinite(initialStats.startMs) &&
    initialStats.endMs > initialStats.startMs &&
    initialStats.ariaHasTimeSpan &&
    initialStats.titleHasTimeSpan &&
    initialStats.title === initialStats.ariaLabel &&
    initialStats.role === "button" &&
    initialStats.tabIndex === "0" &&
    initialStats.hitTrack === track &&
    initialStats.badgeWidth >= 20 &&
    initialStats.badgeHeight >= 10 &&
    initialStats.markerHitId === String(expectedIds.at(-1)) &&
    initialStats.markerLoopCount > 1 &&
    initialStats.markerTitle.includes(` x ${initialStats.markerLoopCount}`) &&
    !initialStats.visibleBlockLabelsHaveIterationSuffix &&
    initialStats.markerCount === 500 &&
    initialStats.overviewNodeCount < 3_500 &&
    initialStats.rulerTickCount > 1 &&
    initialStats.rulerTickCount <= 64 &&
    activationStats !== null &&
    activationStats.stateHandlerElapsedMs < 100 &&
    activationStats.stateMicrotaskElapsedMs < 100 &&
    activationStats.stateTrack === track &&
    activationStats.committedFilterVisible &&
    activationStats.keyboardAccessStats.active?.track === track &&
    activationStats.keyboardAccessStats.tabsToCluster > 0 &&
    activationStats.keyboardAccessStats.tabsToCluster <= 3 &&
    activationStats.keyboardAccessStats.axButtonFound &&
    activationStats.keyboardAccessStats.markerTabStopCount === 1 &&
    activationStats.keyboardAccessStats.clusterTabStopCount === 2 &&
    activationStats.keyboardAccessStats.overviewRole === "group" &&
    activationStats.filterText.includes("250") &&
    activationStats.pagerLabel.includes("/ 250") &&
    activationStats.rowCount > 0 &&
    activationStats.rowCount <= 12 &&
    activationStats.rowsBelongToCluster &&
    activationStats.afterPanStartMs > activationStats.beforePanStartMs &&
    activationStats.filterPersistedAfterPan &&
    activationStats.badgePersistedAfterPan &&
    activationStats.membershipPersistedAfterPan &&
    activationStats.pagerAfterLiveFollow === activationStats.pagerBeforeLiveFollow &&
    activationStats.filterPersistedDuringLiveFollow &&
    searchStats !== null &&
    searchStats.foundIds.length === probeIds.length &&
    searchStats.foundIds.every((eventId, index) => eventId === probeIds[index]) &&
    searchStats.selectedMarkerId === String(probeIds[1]) &&
    searchStats.lastMarkerId === String(probeIds[1]) &&
    searchStats.filterCleared &&
    searchStats.pagerLabelAfterClear.includes("/ 500") &&
    searchStats.rowCountAfterClear > 0 &&
    searchStats.rowCountAfterClear <= 12 &&
    searchStats.selectedRowVisibleAfterClear &&
    searchStats.selectedRowIdAfterClear === String(probeIds[1]) &&
    searchStats.lastMarkerIdAfterClear === String(probeIds[1]) &&
    searchStats.overviewNodeCountAfterClear < 3_500 &&
    searchStats.searchGuardPassed &&
    searchStats.dirtyGuardPassed &&
    hasNoOuterOverflow(containment)
  );
  return {
    track,
    passed,
    initialStats,
    activationStats,
    searchStats,
    containment,
  };
}

async function runPartiallyClippedMarkerDrag(client, eventId) {
  await clickVisibleByText(client, ".timelineViewportToolbar button", "Pan Next");
  await evaluatePageFunction(client, () =>
    new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame))),
  );
  const geometry = await evaluatePageFunction(client, (wantedEventId) => {
    const marker = document.querySelector(
      '.timelineMarker.sceneBlock[data-timeline-event-id="' + wantedEventId + '"]',
    );
    const body = marker?.querySelector(".timelineSceneBlockBody");
    const bodyRect = body?.getBoundingClientRect();
    const svgRect = marker?.ownerSVGElement?.getBoundingClientRect();
    const range = document.querySelector(".timelineVisibleRange");
    if (!marker || !bodyRect || !svgRect) return null;
    const x = bodyRect.left + bodyRect.width / 2;
    const y = bodyRect.top + bodyRect.height / 2;
    return {
      x,
      y,
      hitId: document.elementFromPoint(x, y)?.closest(".timelineMarker")
        ?.getAttribute("data-timeline-event-id") ?? "",
      svgWidth: svgRect.width,
      originalStartMs: Number(marker.getAttribute("data-timeline-start-ms")),
      originalPreviewStartMs: Number(marker.getAttribute("data-timeline-preview-start-ms")),
      visibleStartMs: Number(range?.getAttribute("data-visible-start-ms")),
      visibleEndMs: Number(range?.getAttribute("data-visible-end-ms")),
      bodyLeft: bodyRect.left,
      bodyRight: bodyRect.right,
      svgLeft: svgRect.left,
      svgRight: svgRect.right,
    };
  }, eventId);
  if (!geometry) return null;
  const markerSnapshot = () => evaluatePageFunction(client, (wantedEventId) => {
    const marker = document.querySelector(
      '.timelineMarker.sceneBlock[data-timeline-event-id="' + wantedEventId + '"]',
    );
    const bodyRect = marker?.querySelector(".timelineSceneBlockBody")?.getBoundingClientRect();
    return {
      startMs: Number(marker?.getAttribute("data-timeline-start-ms")),
      previewStartMs: Number(marker?.getAttribute("data-timeline-preview-start-ms")),
      bodyLeft: bodyRect?.left ?? null,
    };
  }, eventId);
  await client.send("Input.dispatchMouseEvent", {
    type: "mousePressed",
    x: geometry.x,
    y: geometry.y,
    button: "left",
    buttons: 1,
    clickCount: 1,
  });
  await client.send("Input.dispatchMouseEvent", {
    type: "mouseMoved",
    x: geometry.x + 2,
    y: geometry.y,
    button: "left",
    buttons: 1,
  });
  const tiny = await markerSnapshot();
  await client.send("Input.dispatchMouseEvent", {
    type: "mouseMoved",
    x: geometry.x + 6,
    y: geometry.y,
    button: "left",
    buttons: 1,
  });
  const moved = await markerSnapshot();
  await client.send("Input.dispatchMouseEvent", {
    type: "mouseReleased",
    x: geometry.x + 6,
    y: geometry.y,
    button: "left",
    buttons: 0,
    clickCount: 1,
  });
  await sleep(80);
  await evaluatePageFunction(client, () =>
    new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame))),
  );
  const settled = await markerSnapshot();
  const visibleSpanMs = geometry.visibleEndMs - geometry.visibleStartMs;
  return {
    ...geometry,
    tiny,
    moved,
    settled,
    expectedDeltaMs: (6 / geometry.svgWidth) * visibleSpanMs,
    actualPreviewDeltaMs: moved.previewStartMs - geometry.originalPreviewStartMs,
  };
}

async function runSceneBlockOverlapViewport(client, viewport) {
  const lightingIds = Array.from({ length: 250 }, (_, index) => index * 2 + 1);
  const videoIds = Array.from({ length: 250 }, (_, index) => index * 2 + 2);
  const lighting = await runSceneBlockOverlapTrack(client, viewport, "Lighting", lightingIds, "keyboard");
  const video = await runSceneBlockOverlapTrack(client, viewport, "Video", videoIds, "mouse");

  await openTimelineShowFixture(client, viewport, "scene-block-large");
  const switchStats = await evaluatePageFunction(client, async () => {
    if (typeof window.__syndocalPauseSceneBlockFixtureChurn === "function") {
      window.__syndocalPauseSceneBlockFixtureChurn();
    }
    const activate = async (track) => {
      const badge = [...document.querySelectorAll(".timelineOverlapCluster")]
        .find((candidate) => candidate.getAttribute("data-overlap-track") === track);
      badge?.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      await new Promise((resolveFrame) =>
        requestAnimationFrame(() => requestAnimationFrame(() => requestAnimationFrame(resolveFrame))),
      );
      return badge;
    };
    const lightingBadge = await activate("Lighting");
    const selectedAfterLighting = document.querySelector(".timelineMarker.sceneBlock.selected")
      ?.getAttribute("data-timeline-event-id") ?? "";
    const lightingFilterText = (document.querySelector(".sceneBlockOverlapFilter")?.textContent || "").trim();
    const videoBadge = await activate("Video");
    const videoMemberIds = (videoBadge?.getAttribute("data-overlap-members") || "")
      .split(",")
      .filter(Boolean)
      .map(Number);
    const selectedAfterVideo = document.querySelector(".timelineMarker.sceneBlock.selected")
      ?.getAttribute("data-timeline-event-id") ?? "";
    const rowIds = [...document.querySelectorAll(".sceneBlockRow")]
      .map((row) => Number(row.getAttribute("data-scene-block-id")));
    return {
      selectedAfterLighting,
      selectedAfterVideo,
      lightingFilterText,
      videoFilterText: (document.querySelector(".sceneBlockOverlapFilter")?.textContent || "").trim(),
      lightingMemberIds: (lightingBadge?.getAttribute("data-overlap-members") || "")
        .split(",")
        .filter(Boolean)
        .map(Number),
      videoMemberIds,
      rowIds,
      selectedWasOutsideVideo: !videoMemberIds.includes(Number(selectedAfterLighting)),
      selectedReconciledIntoVideo: videoMemberIds.includes(Number(selectedAfterVideo)),
      rowsBelongToVideo: rowIds.every((eventId) => videoMemberIds.includes(eventId)),
      pagerLabel: (document.querySelector(".sceneBlockPager span")?.textContent || "").trim(),
    };
  });
  const clippedDragStats = await runPartiallyClippedMarkerDrag(
    client,
    Number(switchStats.selectedAfterVideo),
  );
  const finalStats = await evaluatePageFunction(client, () => ({
    videoFilterVisible: (document.querySelector(".sceneBlockOverlapFilter")?.textContent || "").includes("Video overlap ×250"),
    lightingBadgeVisible: Boolean(
      document.querySelector('.timelineOverlapCluster[data-overlap-track="Lighting"]'),
    ),
    videoBadgeVisible: Boolean(
      document.querySelector('.timelineOverlapCluster[data-overlap-track="Video"]'),
    ),
    pagerLabel: (document.querySelector(".sceneBlockPager span")?.textContent || "").trim(),
    overviewNodeCount: document.querySelectorAll(".timelineOverview *").length,
    selectedMarkerId: document.querySelector(".timelineMarker.sceneBlock.selected")
      ?.getAttribute("data-timeline-event-id") ?? "",
    selectedMarkerAria: document.querySelector(".timelineMarker.sceneBlock.selected")
      ?.getAttribute("aria-label") ?? "",
  }));
  await evaluatePageFunction(client, async () => {
    if (typeof window.__syndocalPauseSceneBlockFixtureChurn === "function") {
      window.__syndocalPauseSceneBlockFixtureChurn();
    }
    document.body.style.display = "none";
    document.body.getBoundingClientRect();
    document.body.style.display = "";
    document.body.getBoundingClientRect();
    await new Promise((resolveFrame) =>
      requestAnimationFrame(() => requestAnimationFrame(() => requestAnimationFrame(resolveFrame))),
    );
  });
  await sleep(320);
  let screenshotVerification = null;
  if (shouldCaptureViewport(viewport)) {
    mkdirSync(screenshotDir, { recursive: true });
    screenshotVerification = await captureSettledViewport(
      client,
      join(screenshotDir, "scene-block-overlap-" + viewport.width + "x" + viewport.height + ".png"),
    );
  }
  const containment = await measure(client, "scene-block-overlap-" + viewport.width + "x" + viewport.height);
  const passed = Boolean(
    lighting.passed &&
    video.passed &&
    switchStats.lightingFilterText.includes("Lighting overlap ×250") &&
    switchStats.videoFilterText.includes("Video overlap ×250") &&
    switchStats.selectedWasOutsideVideo &&
    switchStats.selectedReconciledIntoVideo &&
    switchStats.rowsBelongToVideo &&
    switchStats.rowIds.length > 0 &&
    switchStats.rowIds.length <= 12 &&
    switchStats.pagerLabel.includes("/ 250") &&
    clippedDragStats !== null &&
    clippedDragStats.hitId === switchStats.selectedAfterVideo &&
    clippedDragStats.originalStartMs < clippedDragStats.visibleStartMs &&
    clippedDragStats.bodyLeft < clippedDragStats.svgLeft &&
    clippedDragStats.bodyRight > clippedDragStats.svgLeft &&
    clippedDragStats.tiny.previewStartMs === clippedDragStats.originalPreviewStartMs &&
    Math.abs(clippedDragStats.actualPreviewDeltaMs - clippedDragStats.expectedDeltaMs) <= 2 &&
    Math.abs(
      clippedDragStats.settled.startMs -
      (clippedDragStats.originalStartMs + clippedDragStats.expectedDeltaMs)
    ) <= 2 &&
    clippedDragStats.settled.startMs < clippedDragStats.visibleStartMs &&
    finalStats.videoFilterVisible &&
    finalStats.lightingBadgeVisible &&
    finalStats.videoBadgeVisible &&
    finalStats.pagerLabel.includes("/ 250") &&
    finalStats.overviewNodeCount < 3_500 &&
    finalStats.selectedMarkerId === switchStats.selectedAfterVideo &&
    finalStats.selectedMarkerAria.includes("Block #" + switchStats.selectedAfterVideo) &&
    finalStats.selectedMarkerAria.includes("duration") &&
    (!shouldCaptureViewport(viewport) || screenshotVerification?.verified === true) &&
    hasNoOuterOverflow(containment)
  );
  return {
    label: "scene-block-overlap-" + viewport.width + "x" + viewport.height,
    passed,
    lighting,
    video,
    switchStats,
    clippedDragStats,
    finalStats,
    containment,
    screenshotVerification,
  };
}

async function runSceneBlockLargeViewport(client, viewport) {
  await client.send("Emulation.setDeviceMetricsOverride", {
    width: viewport.width,
    height: viewport.height,
    deviceScaleFactor: 1,
    mobile: false,
  });
  await client.send("Page.navigate", { url: fixtureUrl("scene-block-large") });
  await waitForApp(client);
  await clickVisibleByText(client, ".workspaceTabs button", "Control");
  await clickVisibleByText(client, ".controlModeTabs button", "Timeline");
  await clickVisibleByText(client, ".timelineDeskTabs button", "Show");
  await sleep(180);
  const idleMutationStats = await client.evaluate(`(() => new Promise((resolveIdle) => {
    const target = document.querySelector('.timelinePanel');
    if (!target) {
      resolveIdle({ records: -1, addedNodes: -1, removedNodes: -1 });
      return;
    }
    let records = 0;
    let addedNodes = 0;
    let removedNodes = 0;
    let classAttributeMutations = 0;
    const samples = [];
    const observer = new MutationObserver((mutations) => {
      for (const mutation of mutations) {
        if (mutation.type === 'attributes') {
          classAttributeMutations += 1;
          continue;
        }
        if (mutation.type !== 'childList') continue;
        records += 1;
        addedNodes += mutation.addedNodes.length;
        removedNodes += mutation.removedNodes.length;
        if (samples.length < 8) {
          const describe = (node) => node.nodeType === Node.TEXT_NODE
            ? '#text:' + (node.textContent || '').trim().slice(0, 32)
            : node.nodeName + '.' + (node.className?.baseVal ?? node.className ?? '');
          samples.push({
            target: describe(mutation.target),
            added: [...mutation.addedNodes].map(describe),
            removed: [...mutation.removedNodes].map(describe),
          });
        }
      }
    });
    observer.observe(target, { childList: true, attributes: true, attributeFilter: ['class'], subtree: true });
    setTimeout(() => {
      observer.disconnect();
      resolveIdle({ records, addedNodes, removedNodes, classAttributeMutations, samples });
    }, 1100);
  }))()`);
  await client.evaluate(`(async () => {
    window.__syndocalPauseSceneBlockFixtureChurn?.();
    await new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame)));
  })()`);
  const activeTransitionStats = await client.evaluate(`(async () => {
    const overview = document.querySelector('.timelineOverview');
    const markersBefore = [...document.querySelectorAll('.timelineMarker.sceneBlock')];
    if (!overview || typeof window.__syndocalSetSceneBlockFixtureState !== 'function') return null;
    const readMarkerVisual = () => {
      const marker = document.querySelector(
        '.timelineOverview .timelineMarker.sceneBlock.underPlayhead:not(.selected)'
      );
      const body = marker?.querySelector('.timelineSceneBlockBody');
      const style = body ? getComputedStyle(body) : null;
      return {
        overviewExecutingLive: overview.classList.contains('executingLive'),
        headerExecutingLive: Boolean(document.querySelector('.timelineHeaderMeta.executingLive')),
        perMarkerActiveClassCount: document.querySelectorAll(
          '.timelineOverview .timelineMarker.sceneBlock.active'
        ).length,
        markerId: marker?.getAttribute('data-timeline-event-id') ?? '',
        stroke: style?.stroke ?? '',
        strokeWidth: style?.strokeWidth ?? '',
        strokeDasharray: style?.strokeDasharray ?? '',
      };
    };
    let childListMutations = 0;
    let addedNodes = 0;
    let removedNodes = 0;
    let classAttributeMutations = 0;
    const observer = new MutationObserver((mutations) => {
      for (const mutation of mutations) {
        if (mutation.type === 'childList') {
          childListMutations += 1;
          addedNodes += mutation.addedNodes.length;
          removedNodes += mutation.removedNodes.length;
        } else if (mutation.type === 'attributes') {
          classAttributeMutations += 1;
        }
      }
    });
    observer.observe(overview, { childList: true, attributes: true, attributeFilter: ['class'], subtree: true });
    const startedAt = performance.now();
    window.__syndocalSetSceneBlockFixtureState(1000, true);
    await new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame)));
    const liveMarkerVisual = readMarkerVisual();
    const activeCount = overview.classList.contains('executingLive')
      ? document.querySelectorAll('.timelineOverview .timelineMarker.sceneBlock.underPlayhead').length : 0;
    const liveRowCount = document.querySelectorAll('.sceneBlockRow.live').length;
    window.__syndocalSetSceneBlockFixtureState(1000, false);
    await new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame)));
    const stoppedMarkerVisual = readMarkerVisual();
    const elapsedMs = performance.now() - startedAt;
    observer.disconnect();
    const markersAfter = [...document.querySelectorAll('.timelineMarker.sceneBlock')];
    return {
      activeCount,
      liveRowCount,
      liveMarkerVisual,
      stoppedMarkerVisual,
      stoppedActiveCount: overview.classList.contains('executingLive')
        ? document.querySelectorAll('.timelineOverview .timelineMarker.sceneBlock.underPlayhead').length : 0,
      stoppedUnderPlayheadCount: document.querySelectorAll('.timelineMarker.sceneBlock.underPlayhead').length,
      stoppedLiveRowCount: document.querySelectorAll('.sceneBlockRow.live').length,
      childListMutations,
      addedNodes,
      removedNodes,
      classAttributeMutations,
      elapsedMs,
      markerIdentityStable: markersBefore.length === markersAfter.length &&
        markersBefore.every((marker, index) => marker === markersAfter[index]),
    };
  })()`);
  const dragGeometry = await client.evaluate(`(() => {
    const marker = document.querySelector('.timelineMarker.sceneBlock[data-timeline-event-id="500"]');
    const body = marker?.querySelector('.timelineSceneBlockBody');
    const svg = marker?.ownerSVGElement;
    if (!marker || !body || !svg) return null;
    const rect = body.getBoundingClientRect();
    const svgRect = svg.getBoundingClientRect();
    return {
      x: rect.left + rect.width / 2,
      y: rect.top + rect.height / 2,
      svgWidth: svgRect.width,
    };
  })()`);
  let markerDragStats = null;
  if (dragGeometry) {
    const markerX = () => client.evaluate(`(() => {
      const marker = document.querySelector('.timelineMarker.sceneBlock[data-timeline-event-id="500"]');
      const body = marker?.querySelector('.timelineSceneBlockBody');
      return body ? body.getBoundingClientRect().left : null;
    })()`);
    const initialX = await markerX();
    await client.send('Input.dispatchMouseEvent', {
      type: 'mousePressed',
      x: dragGeometry.x,
      y: dragGeometry.y,
      button: 'left',
      buttons: 1,
      clickCount: 1,
    });
    await client.send('Input.dispatchMouseEvent', {
      type: 'mouseMoved',
      x: dragGeometry.x + 2,
      y: dragGeometry.y,
      button: 'left',
      buttons: 1,
    });
    const tinyMoveX = await markerX();
    await client.send('Input.dispatchMouseEvent', {
      type: 'mouseMoved',
      x: dragGeometry.x + 60,
      y: dragGeometry.y,
      button: 'left',
      buttons: 1,
    });
    const committedMoveX = await markerX();
    await client.send('Input.dispatchMouseEvent', {
      type: 'mouseReleased',
      x: dragGeometry.x + 60,
      y: dragGeometry.y,
      button: 'left',
      buttons: 0,
      clickCount: 1,
    });
    await client.evaluate(`new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame)))`);
    const settledMouseX = await markerX();
    const nonMouseInitialX = await markerX();
    const nonMouseGeometry = await client.evaluate(`(() => {
      const body = document.querySelector('.timelineMarker.sceneBlock[data-timeline-event-id="500"] .timelineSceneBlockBody');
      if (!body) return null;
      const rect = body.getBoundingClientRect();
      return { x: rect.left + rect.width / 2, y: rect.top + rect.height / 2 };
    })()`);
    const markerTouchAction = await client.evaluate(`(() => {
      const marker = document.querySelector('.timelineMarker.sceneBlock[data-timeline-event-id="500"]');
      return marker ? getComputedStyle(marker).touchAction : '';
    })()`);
    await client.send('Input.dispatchMouseEvent', {
      type: 'mousePressed',
      x: nonMouseGeometry?.x ?? dragGeometry.x,
      y: nonMouseGeometry?.y ?? dragGeometry.y,
      button: 'left',
      buttons: 1,
      clickCount: 1,
      pointerType: 'pen',
    });
    await client.send('Input.dispatchMouseEvent', {
      type: 'mouseMoved',
      x: (nonMouseGeometry?.x ?? dragGeometry.x) + 2,
      y: nonMouseGeometry?.y ?? dragGeometry.y,
      button: 'left',
      buttons: 1,
      pointerType: 'pen',
    });
    const nonMouseTinyMoveX = await markerX();
    await client.send('Input.dispatchMouseEvent', {
      type: 'mouseMoved',
      x: (nonMouseGeometry?.x ?? dragGeometry.x) + 60,
      y: nonMouseGeometry?.y ?? dragGeometry.y,
      button: 'left',
      buttons: 1,
      pointerType: 'pen',
    });
    const nonMouseCommittedMoveX = await markerX();
    await client.send('Input.dispatchMouseEvent', {
      type: 'mouseReleased',
      x: (nonMouseGeometry?.x ?? dragGeometry.x) + 60,
      y: nonMouseGeometry?.y ?? dragGeometry.y,
      button: 'left',
      buttons: 0,
      clickCount: 1,
      pointerType: 'pen',
    });
    await client.evaluate(`new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame)))`);
    const settledNonMouseX = await markerX();
    markerDragStats = {
      initialX,
      tinyMoveX,
      committedMoveX,
      expectedCommittedX: initialX + 60,
      settledMouseX,
      markerTouchAction,
      nonMousePointerType: 'pen',
      nonMouseInitialX,
      nonMouseTinyMoveX,
      nonMouseCommittedMoveX,
      expectedNonMouseCommittedX: nonMouseInitialX + 60,
      settledNonMouseX,
    };
  }
  const showStats = await client.evaluate(`(async () => {
    const rows = () => [...document.querySelectorAll('.sceneBlockRow')];
    const beforeRows = rows();
    const sourceOptionCount = document.querySelectorAll('.sceneBlockRow .sceneBlockSourceField option').length;
    const sourceChangeButtonCount = document.querySelectorAll('.sceneBlockRow .sceneBlockSourceButton').length;
    const composerSourceOptionCount = document.querySelectorAll('.sceneBlockComposer .sceneBlockSourceField option').length;
    const rowJumpOptionCount = document.querySelectorAll('.sceneBlockRow .sceneBlockAfterField option').length;
    const totalOptionCount = document.querySelectorAll('.sceneBlockWorkspace option').length;
    const marker493 = document.querySelector('.timelineMarker.sceneBlock[data-timeline-event-id="493"]');
    marker493?.dispatchEvent(new MouseEvent('click', { bubbles: true }));
    await new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame)));
    const list = document.querySelector('.sceneBlockList');
    const workspace = document.querySelector('.sceneBlockWorkspace');
    const rowVisible = (row) => {
      if (!row || !list || !workspace) return false;
      const rect = row.getBoundingClientRect();
      const listRect = list.getBoundingClientRect();
      const workspaceRect = workspace.getBoundingClientRect();
      return rect.top >= Math.max(0, listRect.top, workspaceRect.top) - 1 &&
        rect.bottom <= Math.min(window.innerHeight, listRect.bottom, workspaceRect.bottom) + 1;
    };
    const firstSelectionRow = document.querySelector('.sceneBlockRow.selected[data-scene-block-id="493"]');
    const firstMarkerRevealVisible = rowVisible(firstSelectionRow);
    const firstButton = [...document.querySelectorAll('.sceneBlockPager button')]
      .find((button) => (button.textContent || '').trim() === 'First');
    firstButton?.click();
    await new Promise((resolveFrame) => requestAnimationFrame(resolveFrame));
    marker493?.dispatchEvent(new MouseEvent('click', { bubbles: true }));
    await new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame)));
    const repeatedSelectionVisible = rowVisible(
      document.querySelector('.sceneBlockRow.selected[data-scene-block-id="493"]'),
    );
    const search = document.querySelector('.sceneBlockRowSearch');
    if (search instanceof HTMLInputElement) {
      search.value = 'L1 cue 500';
      search.dispatchEvent(new Event('input', { bubbles: true }));
    }
    await new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame)));
    const cueIdentitySearchReached500 = rows().some((row) => row.getAttribute('data-scene-block-id') === '500');
    const clear = [...document.querySelectorAll('.sceneBlockPager button')]
      .find((button) => (button.textContent || '').trim() === 'Clear');
    clear?.click();
    await new Promise((resolveFrame) => requestAnimationFrame(resolveFrame));
    const last = [...document.querySelectorAll('.sceneBlockPager button')]
      .find((button) => (button.textContent || '').trim() === 'Last');
    last?.click();
    await new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame)));
    const afterRows = rows();
    const row500 = document.querySelector('.sceneBlockRow[data-scene-block-id="500"]');
    const row500StartInput = row500?.querySelector('.sceneBlockRowFields input[type="number"]');
    const row500Save = row500?.querySelector('.sceneBlockRowActions .primary');
    const cleanSaveDisabled = row500Save instanceof HTMLButtonElement && row500Save.disabled;
    const originalStart = row500StartInput instanceof HTMLInputElement ? row500StartInput.value : '';
    if (row500StartInput instanceof HTMLInputElement) {
      row500StartInput.value = String(Number(originalStart) + 100);
      row500StartInput.dispatchEvent(new Event('input', { bubbles: true }));
    }
    await new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame)));
    const dirtyClassVisible = row500?.classList.contains('dirty') ?? false;
    const dirtyBadgeVisible = Boolean(row500?.querySelector('.sceneBlockUnsavedBadge'));
    const dirtySaveEnabled = row500Save instanceof HTMLButtonElement && !row500Save.disabled;
    const topbarDirtyVisible = document.querySelector('.topbarProject')?.classList.contains('dirty') ?? false;
    const projectLabelDirtyVisible = (document.querySelector('.topbarProject span')?.textContent || '').trim().endsWith('*');
    const globalSave = document.querySelector('.projectAction[title^="Save project"]');
    globalSave?.click();
    await Promise.resolve();
    const globalSaveBlocked = (document.querySelector('.appStatusText')?.textContent || '')
      .includes('Save each modified Scene Block row');
    if (row500StartInput instanceof HTMLInputElement) {
      row500StartInput.value = originalStart;
      row500StartInput.dispatchEvent(new Event('input', { bubbles: true }));
    }
    await new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame)));
    const cleanAfterRevert = !(row500?.classList.contains('dirty') ?? true) &&
      (row500Save instanceof HTMLButtonElement && row500Save.disabled);
    const svg = document.querySelector('.timelineOverview');
    const overviewBackground = svg?.querySelector('.timelineOverviewBg');
    const svgRect = svg?.getBoundingClientRect();
    const backgroundRect = overviewBackground?.getBoundingClientRect();
    const longLabel = document.querySelector('.timelineMarker[data-timeline-event-id="500"] .timelineSceneBlockLabel');
    const longBody = document.querySelector('.timelineMarker[data-timeline-event-id="500"] .timelineSceneBlockBody');
    const renderedBlockLabelFontSize = longLabel && svgRect
      ? parseFloat(getComputedStyle(longLabel).fontSize) * (svgRect.height / 44)
      : 0;
    const longLabelWithinBody = Boolean(longLabel && longBody &&
      longLabel.getBBox().x + longLabel.getBBox().width <= longBody.getBBox().x + longBody.getBBox().width + 0.5);
    return {
      overviewBlockCount: document.querySelectorAll('.timelineOverview .timelineMarker.sceneBlock').length,
      overviewLoopLineCount: document.querySelectorAll('.timelineOverview .timelineSceneBlockLoop').length,
      overviewNodeCount: document.querySelectorAll('.timelineOverview *').length,
      beforeRowCount: beforeRows.length,
      afterRowCount: afterRows.length,
      sourceOptionCount,
      sourceChangeButtonCount,
      composerSourceOptionCount,
      rowJumpOptionCount,
      totalOptionCount,
      pagerCount: document.querySelectorAll('.sceneBlockPager').length,
      pagerLabel: (document.querySelector('.sceneBlockPager span')?.textContent || '').trim(),
      reachedLastBlock: afterRows.some((row) => row.getAttribute('data-scene-block-id') === '500'),
      firstMarkerRevealVisible,
      repeatedSelectionVisible,
      cueIdentitySearchReached500,
      selectedMarkerRenderedLast: [...document.querySelectorAll('.timelineOverview .timelineMarker')]
        .at(-1)?.getAttribute('data-timeline-event-id') === '493',
      cleanSaveDisabled,
      dirtyClassVisible,
      dirtyBadgeVisible,
      dirtySaveEnabled,
      topbarDirtyVisible,
      projectLabelDirtyVisible,
      globalSaveBlocked,
      cleanAfterRevert,
      overviewBackgroundFits: Boolean(svgRect && backgroundRect &&
        Math.abs(svgRect.left - backgroundRect.left) <= 2 && Math.abs(svgRect.right - backgroundRect.right) <= 2),
      renderedBlockLabelFontSize,
      longLabelWithinBody,
      sharedJumpPickerCount: document.querySelectorAll('.sceneBlockJumpPicker').length,
      fieldLabelFontSize: parseFloat(getComputedStyle(document.querySelector('.sceneBlockRowFields label')).fontSize),
      semanticHintFontSize: parseFloat(getComputedStyle(document.querySelector('.sceneBlockFieldLabel small')).fontSize),
      columnGuideFontSize: parseFloat(getComputedStyle(document.querySelector('.sceneBlockColumnGuide')).fontSize),
      timeLabel: (document.querySelector('.timelineTimeStat strong')?.textContent || '').trim(),
      timeTitle: document.querySelector('.timelineTimeStat')?.getAttribute('title') ?? '',
      timeStatClipped: (() => {
        const container = document.querySelector('.timelineTimeStat');
        const value = container?.querySelector('strong');
        return Boolean(
          (container && container.scrollWidth > container.clientWidth + 1) ||
          (value && value.scrollWidth > value.clientWidth + 1)
        );
      })(),
    };
  })()`);
  const showContainment = await measure(client, `scene-block-large-show-${viewport.width}x${viewport.height}`);
  const sourcePickerStats = await client.evaluate(`(async () => {
    const sourceButton = [...document.querySelectorAll('.sceneBlockRow .sceneBlockSourceButton')].at(-1);
    if (!(sourceButton instanceof HTMLButtonElement)) return null;
    const expectedCueId = Number(sourceButton.closest('.sceneBlockRow')?.getAttribute('data-source-cue-id'));
    sourceButton.click();
    await new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame)));
    let picker = document.querySelector('.sceneBlockSourcePicker');
    picker?.scrollIntoView({ block: 'nearest', inline: 'nearest' });
    await new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame)));
    const initialSelect = picker?.querySelector('select');
    const initialSelectedCueId = initialSelect instanceof HTMLSelectElement ? Number(initialSelect.value) : null;
    const initialSelectedLabel = initialSelect instanceof HTMLSelectElement
      ? (initialSelect.selectedOptions[0]?.textContent || '').trim()
      : '';
    const initialOptionCount = picker?.querySelectorAll('option').length ?? 0;
    const initialQuery = picker?.querySelector('input[type="search"]');
    const apply = [...(picker?.querySelectorAll('button') ?? [])]
      .find((button) => (button.textContent || '').trim() === 'Apply Source');
    if (initialQuery instanceof HTMLInputElement) {
      initialQuery.value = 'no such source cue';
      initialQuery.dispatchEvent(new Event('input', { bubbles: true }));
    }
    await new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame)));
    const noMatchOptionCount = picker?.querySelectorAll('option').length ?? 0;
    const applyDisabledWithNoMatches = apply instanceof HTMLButtonElement && apply.disabled;
    if (initialQuery instanceof HTMLInputElement) {
      initialQuery.value = '';
      initialQuery.dispatchEvent(new Event('input', { bubbles: true }));
    }
    await new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame)));
    const restoredSelect = picker?.querySelector('select');
    const restoredCueId = restoredSelect instanceof HTMLSelectElement ? Number(restoredSelect.value) : null;
    if (apply instanceof HTMLButtonElement) apply.click();
    await new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame)));
    const preservedAfterApply = sourceButton.textContent?.includes('Scale Cue 500') ?? false;
    const closedAfterApply = document.querySelectorAll('.sceneBlockSourcePicker').length === 0;
    sourceButton.click();
    await new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame)));
    picker = document.querySelector('.sceneBlockSourcePicker');
    picker?.scrollIntoView({ block: 'nearest', inline: 'nearest' });
    await new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame)));
    const query = picker?.querySelector('input[type="search"]');
    if (query instanceof HTMLInputElement) {
      query.value = 'L1 cue 800 Scale Cue 500';
      query.dispatchEvent(new Event('input', { bubbles: true }));
    }
    await new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame)));
    const select = picker?.querySelector('select');
    const workspace = document.querySelector('.sceneBlockWorkspace');
    const pickerRect = picker?.getBoundingClientRect();
    const workspaceRect = workspace?.getBoundingClientRect();
    const reachable = Boolean(pickerRect && workspaceRect &&
      pickerRect.width > 0 && pickerRect.height > 0 &&
      pickerRect.left >= Math.max(0, workspaceRect.left) - 1 &&
      pickerRect.right <= Math.min(window.innerWidth, workspaceRect.right) + 1 &&
      pickerRect.top >= Math.max(0, workspaceRect.top) - 1 &&
      pickerRect.bottom <= Math.min(window.innerHeight, workspaceRect.bottom) + 1);
    const sourcePickerControls = [...(picker?.querySelectorAll('input, select, button') ?? [])];
    const fullyVisibleControlCount = sourcePickerControls.filter((control) => {
      if (!workspaceRect) return false;
      const rect = control.getBoundingClientRect();
      return rect.width > 0 && rect.height > 0 &&
        rect.left >= Math.max(0, workspaceRect.left) - 1 &&
        rect.right <= Math.min(window.innerWidth, workspaceRect.right) + 1 &&
        rect.top >= Math.max(0, workspaceRect.top) - 1 &&
        rect.bottom <= Math.min(window.innerHeight, workspaceRect.bottom) + 1 &&
        (!pickerRect || (
          rect.left >= pickerRect.left - 1 && rect.right <= pickerRect.right + 1 &&
          rect.top >= pickerRect.top - 1 && rect.bottom <= pickerRect.bottom + 1
        ));
    }).length;
    const cancel = [...(picker?.querySelectorAll('button') ?? [])]
      .find((button) => (button.textContent || '').trim() === 'Cancel');
    if (cancel instanceof HTMLButtonElement) cancel.click();
    await Promise.resolve();
    return {
      pickerCount: picker ? 1 : 0,
      expectedCueId,
      initialOptionCount,
      initialSelectedCueId,
      initialSelectedLabel,
      noMatchOptionCount,
      applyDisabledWithNoMatches,
      restoredCueId,
      preservedAfterApply,
      closedAfterApply,
      filteredOptionCount: picker?.querySelectorAll('option').length ?? 0,
      filteredValue: select instanceof HTMLSelectElement ? Number(select.value) : null,
      filteredLabel: select instanceof HTMLSelectElement ? (select.selectedOptions[0]?.textContent || '').trim() : '',
      reachable,
      controlCount: sourcePickerControls.length,
      fullyVisibleControlCount,
      closedAfterCancel: document.querySelectorAll('.sceneBlockSourcePicker').length === 0,
    };
  })()`);
  const pickerStats = await client.evaluate(`(async () => {
    const sourceButton = document.querySelector('.sceneBlockRow .sceneBlockSourceButton');
    if (sourceButton instanceof HTMLButtonElement) sourceButton.click();
    await new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame)));
    const sourcePickerCountBeforeJump = document.querySelectorAll('.sceneBlockSourcePicker').length;
    const rowAfterSelect = document.querySelector('.sceneBlockRow .sceneBlockAfterField select');
    if (rowAfterSelect) {
      rowAfterSelect.value = 'pick';
      rowAfterSelect.dispatchEvent(new Event('input', { bubbles: true }));
    }
    await new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame)));
    const sourcePickerCountAfterJump = document.querySelectorAll('.sceneBlockSourcePicker').length;
    const workspace = document.querySelector('.sceneBlockWorkspace');
    const list = document.querySelector('.sceneBlockList');
    const picker = document.querySelector('.sceneBlockJumpPicker');
    const pager = document.querySelector('.sceneBlockPager');
    const lastAction = [...document.querySelectorAll('.sceneBlockRowActions button')].at(-1) ?? null;
    const fullyVisibleWithin = (element, container) => {
      if (!element || !container) return false;
      const rect = element.getBoundingClientRect();
      const containerRect = container.getBoundingClientRect();
      return rect.width > 0 && rect.height > 0 &&
        rect.left >= Math.max(0, containerRect.left) - 1 &&
        rect.right <= Math.min(window.innerWidth, containerRect.right) + 1 &&
        rect.top >= Math.max(0, containerRect.top) - 1 &&
        rect.bottom <= Math.min(window.innerHeight, containerRect.bottom) + 1;
    };
    const fullyVisibleInWorkspace = (element) => fullyVisibleWithin(element, workspace);
    const reveal = async (element, scrollport = null) => {
      if (!element) return false;
      element.scrollIntoView({ block: 'nearest', inline: 'nearest' });
      await new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame)));
      return fullyVisibleInWorkspace(element) && (!scrollport || fullyVisibleWithin(element, scrollport));
    };
    const pickerReachable = await reveal(picker);
    const pickerControls = [...(picker?.querySelectorAll('select, button') ?? [])];
    const fullyVisiblePickerControlCount = pickerControls.filter((control) =>
      fullyVisibleInWorkspace(control) && fullyVisibleWithin(control, picker)).length;
    const pagerReachable = await reveal(pager);
    const lastActionReachable = await reveal(lastAction, list);
    return {
      pickerCount: document.querySelectorAll('.sceneBlockJumpPicker').length,
      sourcePickerCountBeforeJump,
      sourcePickerCountAfterJump,
      pickerOptionCount: document.querySelectorAll('.sceneBlockJumpPicker option').length,
      pagerCount: document.querySelectorAll('.sceneBlockPager').length,
      rowCount: document.querySelectorAll('.sceneBlockRow').length,
      rowActionCount: document.querySelectorAll('.sceneBlockRowActions button').length,
      listHeight: list ? Math.round(list.getBoundingClientRect().height) : 0,
      pickerReachable,
      pickerControlCount: pickerControls.length,
      fullyVisiblePickerControlCount,
      pagerReachable,
      lastActionReachable,
    };
  })()`);
  const composerPickerStats = await client.evaluate(`(async () => {
    const openJumpPicker = document.querySelector('.sceneBlockJumpPicker');
    const cancelOpenJump = [...(openJumpPicker?.querySelectorAll('button') ?? [])]
      .find((button) => (button.textContent || '').trim() === 'Cancel');
    cancelOpenJump?.click();
    await Promise.resolve();
    const sourceButton = document.querySelector('.sceneBlockComposerSourceButton');
    sourceButton?.click();
    await new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame)));
    const sourcePicker = document.querySelector('.sceneBlockSourcePicker');
    const sourceInitialOptionCount = sourcePicker?.querySelectorAll('option').length ?? 0;
    const sourceQuery = sourcePicker?.querySelector('input[type="search"]');
    if (sourceQuery instanceof HTMLInputElement) {
      sourceQuery.value = 'L1 cue 800 Scale Cue 500';
      sourceQuery.dispatchEvent(new Event('input', { bubbles: true }));
    }
    await new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame)));
    const sourceFilteredOptionCount = sourcePicker?.querySelectorAll('option').length ?? 0;
    const sourceApply = [...(sourcePicker?.querySelectorAll('button') ?? [])]
      .find((button) => (button.textContent || '').trim() === 'Apply Source');
    sourceApply?.click();
    await Promise.resolve();
    const afterButton = document.querySelector('.sceneBlockComposerAfterButton');
    afterButton?.click();
    await new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame)));
    const jumpPicker = document.querySelector('.sceneBlockJumpPicker');
    const jumpInitialOptionCount = jumpPicker?.querySelectorAll('option').length ?? 0;
    const jumpQuery = jumpPicker?.querySelector('input[type="search"]');
    if (jumpQuery instanceof HTMLInputElement) {
      jumpQuery.value = '500';
      jumpQuery.dispatchEvent(new Event('input', { bubbles: true }));
    }
    await new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame)));
    const jumpFilteredOptionCount = jumpPicker?.querySelectorAll('option').length ?? 0;
    const jumpSelect = jumpPicker?.querySelector('select');
    if (jumpSelect instanceof HTMLSelectElement && jumpSelect.options.length > 0) {
      jumpSelect.value = jumpSelect.options[0].value;
      jumpSelect.dispatchEvent(new Event('input', { bubbles: true }));
    }
    const jumpApply = [...(jumpPicker?.querySelectorAll('button') ?? [])]
      .find((button) => (button.textContent || '').trim() === 'Apply Target');
    const jumpApplyEnabledAfterChoice = jumpApply instanceof HTMLButtonElement && !jumpApply.disabled;
    jumpApply?.click();
    await Promise.resolve();
    return {
      sourceInitialOptionCount,
      sourceFilteredOptionCount,
      sourceClosedAfterApply: document.querySelectorAll('.sceneBlockSourcePicker').length === 0,
      jumpInitialOptionCount,
      jumpFilteredOptionCount,
      jumpApplyEnabledAfterChoice,
      jumpClosedAfterApply: document.querySelectorAll('.sceneBlockJumpPicker').length === 0,
      composerEagerOptionCount: document.querySelectorAll(
        '.sceneBlockComposer .sceneBlockSourceField option, .sceneBlockComposer .sceneBlockAfterField option',
      ).length,
    };
  })()`);
  const pickerContainment = await measure(client, `scene-block-large-picker-${viewport.width}x${viewport.height}`);
  let screenshotVerification = null;
  if (shouldCaptureViewport(viewport)) {
    mkdirSync(screenshotDir, { recursive: true });
    screenshotVerification = await captureSettledViewport(
      client,
      join(screenshotDir, `scene-block-large-${viewport.width}x${viewport.height}.png`),
    );
  }
  const sourceOpenStats = await client.evaluate(`(async () => {
    const openSource = document.querySelector('.sceneBlockRow[data-scene-block-id="500"] .sceneBlockOpenSourceButton');
    openSource?.click();
    await new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(() => requestAnimationFrame(resolveFrame))));
    const cue500 = document.querySelector('.cueItem[data-cue-id="800"]');
    const cuePanel = document.querySelector('.cuePanel');
    const cueSurfaceTab = [...document.querySelectorAll('.timelineDeskTabs button')]
      .find((button) => (button.textContent || '').trim() === 'Cues');
    const cueHeader = cue500?.querySelector('.cueMetaLine');
    const headerRect = cueHeader?.getBoundingClientRect();
    const panelRect = cuePanel?.getBoundingClientRect();
    return {
      cue500Visible: Boolean(headerRect && panelRect && headerRect.width > 0 && headerRect.height > 0 &&
        headerRect.top >= Math.max(0, panelRect.top) - 1 && headerRect.bottom <= Math.min(window.innerHeight, panelRect.bottom) + 1),
      cuePanelEditing: cuePanel?.classList.contains('cuePanelEditing') ?? false,
      cuesSurfaceActive: cueSurfaceTab?.classList.contains('active') ?? false,
      pagerLabel: (document.querySelector('.cuePager span')?.textContent || '').trim(),
    };
  })()`);
  await client.evaluate(`(async () => {
    for (let page = 0; page < 60; page += 1) {
      const previous = document.querySelector('.cuePager button[aria-label="Previous cue page"]');
      if (!(previous instanceof HTMLButtonElement) || previous.disabled) break;
      previous.click();
      await Promise.resolve();
    }
    await new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame)));
  })()`);
  const cueStats = await client.evaluate(`(() => ({
    cueRowCount: document.querySelectorAll('.cueItem').length,
    placementChipCount: document.querySelectorAll('.cueTimelinePlacementChip').length,
    placementChipButtonCount: document.querySelectorAll('.cueTimelinePlacementChip button').length,
    placementMoreButtonCount: document.querySelectorAll('.cueTimelinePlacementMore').length,
    placementMoreLabel: (document.querySelector('.cueTimelinePlacementMore')?.textContent || '').trim(),
  }))()`);
  const cueContainment = await measure(client, `scene-block-large-cues-${viewport.width}x${viewport.height}`);
  const role = viewportRole(viewport);
  const requiresFullScaleVisualSignoff = role !== "compact-fallback";
  const activeTransitionBudgetMs = role === "compact-fallback"
    ? 4_000
    : role === "extended-ceiling" ? 2_000 : 1_200;
  const passed = Boolean(
    showStats.overviewBlockCount === 500 &&
    showStats.overviewLoopLineCount <= 400 &&
    showStats.overviewNodeCount < 3_500 &&
    idleMutationStats.records === 0 &&
    idleMutationStats.addedNodes === 0 &&
    idleMutationStats.removedNodes === 0 &&
    idleMutationStats.classAttributeMutations === 0 &&
    activeTransitionStats !== null &&
    activeTransitionStats.activeCount === 500 &&
    activeTransitionStats.liveRowCount > 0 &&
    activeTransitionStats.liveMarkerVisual.overviewExecutingLive &&
    activeTransitionStats.liveMarkerVisual.headerExecutingLive &&
    activeTransitionStats.liveMarkerVisual.perMarkerActiveClassCount === 0 &&
    activeTransitionStats.liveMarkerVisual.markerId !== '' &&
    !activeTransitionStats.stoppedMarkerVisual.overviewExecutingLive &&
    !activeTransitionStats.stoppedMarkerVisual.headerExecutingLive &&
    activeTransitionStats.stoppedMarkerVisual.perMarkerActiveClassCount === 0 &&
    activeTransitionStats.liveMarkerVisual.stroke !== activeTransitionStats.stoppedMarkerVisual.stroke &&
    activeTransitionStats.liveMarkerVisual.strokeDasharray !== activeTransitionStats.stoppedMarkerVisual.strokeDasharray &&
    activeTransitionStats.stoppedMarkerVisual.strokeDasharray !== 'none' &&
    activeTransitionStats.stoppedActiveCount === 0 &&
    activeTransitionStats.stoppedUnderPlayheadCount === 500 &&
    activeTransitionStats.stoppedLiveRowCount === 0 &&
    activeTransitionStats.childListMutations === 0 &&
    activeTransitionStats.addedNodes === 0 &&
    activeTransitionStats.removedNodes === 0 &&
    activeTransitionStats.classAttributeMutations <= 4 &&
    activeTransitionStats.markerIdentityStable &&
    activeTransitionStats.elapsedMs < activeTransitionBudgetMs &&
    markerDragStats !== null &&
    Math.abs(markerDragStats.tinyMoveX - markerDragStats.initialX) < 1 &&
    Math.abs(markerDragStats.committedMoveX - markerDragStats.expectedCommittedX) < 2 &&
    Math.abs(markerDragStats.settledMouseX - markerDragStats.committedMoveX) < 2 &&
    markerDragStats.markerTouchAction === 'none' &&
    markerDragStats.nonMousePointerType === 'pen' &&
    Math.abs(markerDragStats.nonMouseTinyMoveX - markerDragStats.nonMouseInitialX) < 1 &&
    Math.abs(markerDragStats.nonMouseCommittedMoveX - markerDragStats.expectedNonMouseCommittedX) < 2 &&
    Math.abs(markerDragStats.settledNonMouseX - markerDragStats.nonMouseCommittedMoveX) < 2 &&
    showStats.beforeRowCount <= 12 &&
    showStats.afterRowCount <= 12 &&
    showStats.sourceOptionCount === 0 &&
    showStats.sourceChangeButtonCount === showStats.beforeRowCount &&
    showStats.composerSourceOptionCount === 0 &&
    showStats.rowJumpOptionCount <= 48 &&
    showStats.totalOptionCount < 200 &&
    showStats.pagerCount === 1 &&
    showStats.pagerLabel.includes('493-500 / 500') &&
    showStats.reachedLastBlock &&
    (!requiresFullScaleVisualSignoff || showStats.firstMarkerRevealVisible) &&
    (!requiresFullScaleVisualSignoff || showStats.repeatedSelectionVisible) &&
    showStats.cueIdentitySearchReached500 &&
    showStats.selectedMarkerRenderedLast &&
    showStats.cleanSaveDisabled &&
    showStats.dirtyClassVisible &&
    showStats.dirtyBadgeVisible &&
    showStats.dirtySaveEnabled &&
    showStats.topbarDirtyVisible &&
    showStats.projectLabelDirtyVisible &&
    showStats.globalSaveBlocked &&
    showStats.cleanAfterRevert &&
    showStats.overviewBackgroundFits &&
    (!requiresFullScaleVisualSignoff || showStats.renderedBlockLabelFontSize >= 9) &&
    showStats.longLabelWithinBody &&
    showStats.sharedJumpPickerCount === 0 &&
    showStats.fieldLabelFontSize >= 10 &&
    showStats.semanticHintFontSize >= 9 &&
    showStats.columnGuideFontSize >= 9 &&
    showStats.timeLabel.includes(' / ') &&
    showStats.timeTitle.startsWith('1000 / ') &&
    showStats.timeTitle.endsWith(' ms') &&
    !showStats.timeStatClipped &&
    sourcePickerStats !== null &&
    sourcePickerStats.pickerCount === 1 &&
    sourcePickerStats.initialOptionCount <= 80 &&
    sourcePickerStats.initialSelectedCueId === sourcePickerStats.expectedCueId &&
    sourcePickerStats.initialSelectedLabel.includes('Scale Cue 500') &&
    sourcePickerStats.noMatchOptionCount === 0 &&
    sourcePickerStats.applyDisabledWithNoMatches &&
    sourcePickerStats.restoredCueId === sourcePickerStats.expectedCueId &&
    sourcePickerStats.preservedAfterApply &&
    sourcePickerStats.closedAfterApply &&
    sourcePickerStats.filteredOptionCount === 1 &&
    sourcePickerStats.filteredLabel.includes('Scale Cue 500') &&
    sourcePickerStats.reachable &&
    sourcePickerStats.controlCount > 0 &&
    sourcePickerStats.fullyVisibleControlCount === sourcePickerStats.controlCount &&
    sourcePickerStats.closedAfterCancel &&
    pickerStats.pickerCount === 1 &&
    pickerStats.sourcePickerCountBeforeJump === 1 &&
    pickerStats.sourcePickerCountAfterJump === 0 &&
    pickerStats.pickerOptionCount <= 80 &&
    pickerStats.pagerCount === 1 &&
    pickerStats.rowCount <= 12 &&
    pickerStats.rowActionCount > 0 &&
    pickerStats.listHeight > 0 &&
    pickerStats.pickerReachable &&
    pickerStats.pickerControlCount > 0 &&
    pickerStats.fullyVisiblePickerControlCount === pickerStats.pickerControlCount &&
    pickerStats.pagerReachable &&
    pickerStats.lastActionReachable &&
    composerPickerStats.sourceInitialOptionCount <= 80 &&
    composerPickerStats.sourceFilteredOptionCount === 1 &&
    composerPickerStats.sourceClosedAfterApply &&
    composerPickerStats.jumpInitialOptionCount <= 80 &&
    composerPickerStats.jumpFilteredOptionCount >= 1 &&
    composerPickerStats.jumpFilteredOptionCount <= 80 &&
    composerPickerStats.jumpApplyEnabledAfterChoice &&
    composerPickerStats.jumpClosedAfterApply &&
    composerPickerStats.composerEagerOptionCount === 0 &&
    sourceOpenStats.cue500Visible &&
    sourceOpenStats.cuePanelEditing &&
    sourceOpenStats.cuesSurfaceActive &&
    sourceOpenStats.pagerLabel.includes('42 / 42') &&
    cueStats.cueRowCount === 12 &&
    cueStats.placementChipCount === 8 &&
    cueStats.placementChipButtonCount === 24 &&
    cueStats.placementMoreButtonCount === 1 &&
    cueStats.placementMoreLabel.includes('491 more') &&
    (!shouldCaptureViewport(viewport) || screenshotVerification?.verified === true) &&
    hasNoOuterOverflow(showContainment) &&
    hasNoOuterOverflow(pickerContainment) &&
    hasNoOuterOverflow(cueContainment)
  );
  return {
    label: `scene-block-large-${viewport.width}x${viewport.height}`,
    passed,
    showStats,
    idleMutationStats,
    activeTransitionStats,
    markerDragStats,
    sourcePickerStats,
    pickerStats,
    cueStats,
    showContainment,
    pickerContainment,
    composerPickerStats,
    sourceOpenStats,
    cueContainment,
    screenshotVerification,
  };
}

async function runLargeShowViewport(client, viewport) {
  await client.send("Emulation.setDeviceMetricsOverride", {
    width: viewport.width,
    height: viewport.height,
    deviceScaleFactor: 1,
    mobile: false,
  });
  await client.send("Page.navigate", { url: appUrl });
  await waitForApp(client);
  await pressKey(client, "F1");
  await sleep(120);
  await clickByText(client, "Mapping");
  await clickByText(client, "Stage Map");
  await sleep(250);

  const stats = await client.evaluate(`(async () => {
    const list = document.querySelector('.virtualizedMappingFixtureList');
    if (!(list instanceof HTMLElement)) return null;
    const beforeCount = list.querySelectorAll('.mappingFixtureRowButton').length;
    const total = Number(list.getAttribute('aria-rowcount'));
    const viewportHeight = list.clientHeight;
    const scrollHeight = list.scrollHeight;
    list.scrollTop = list.scrollHeight;
    list.dispatchEvent(new Event('scroll', { bubbles: true }));
    await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
    const afterRows = [...list.querySelectorAll('.mappingFixtureRowButton')];
    return {
      total,
      beforeCount,
      afterCount: afterRows.length,
      viewportHeight,
      scrollHeight,
      reachedLastFixture: afterRows.some((row) => (row.textContent || '').includes('Large Fixture 2000')),
    };
  })()`);
  const containment = await measure(client, `large-show-${viewport.width}x${viewport.height}`);
  return { stats, containment };
}

async function runEmptyVjViewport(client, viewport) {
  await client.send("Emulation.setDeviceMetricsOverride", {
    width: viewport.width,
    height: viewport.height,
    deviceScaleFactor: 1,
    mobile: false,
  });
  await client.send("Page.navigate", { url: appUrl });
  await waitForApp(client);
  await clickByText(client, "Control");
  await clickByText(client, "VJ Desk");
  await sleep(180);
  if (shouldCaptureViewport(viewport)) {
    mkdirSync(screenshotDir, { recursive: true });
    const screenshot = await client.send("Page.captureScreenshot", { format: "png", fromSurface: true });
    writeFileSync(join(screenshotDir, `vj-empty-${viewport.width}x${viewport.height}.png`), screenshot.data, "base64");
  }
  return measure(client, `vj-empty-${viewport.width}x${viewport.height}`);
}

async function main() {
  const browser = findBrowser();
  if (!browser) {
    throw new Error("No Chrome/Edge/Chromium executable found. Set CHROME_PATH to run viewport containment checks.");
  }

  let viteProcess = null;
  let browserProcess = null;
  let client = null;
  const profileDir = mkdtempSync(join(tmpdir(), "syndocal-cdp-"));

  try {
    if (shouldStartVite) {
      viteProcess = startProcess(
        process.execPath,
        ["node_modules/vite/bin/vite.js", "--host", "127.0.0.1", "--port", String(vitePort), "--strictPort"],
        { cwd: appRoot },
      );
    }
    await waitForHttp(appUrl, "Syndocal dev server");

    browserProcess = startProcess(browser, [
      "--headless=new",
      "--hide-scrollbars",
      "--disable-crash-reporter",
      "--disable-crashpad",
      `--remote-debugging-port=${cdpPort}`,
      `--user-data-dir=${profileDir}`,
      `--window-size=${primaryOperationalViewport.width},${primaryOperationalViewport.height}`,
      "about:blank",
    ]);
    await waitForHttp(`http://127.0.0.1:${cdpPort}/json/version`, "Chrome DevTools Protocol");

    client = await createCdpClient();
    console.log(
      `viewport contract primary-browser=${primaryOperationalViewport.width}x${primaryOperationalViewport.height} measured-client-size-browser=${measuredClientSizeViewport.width}x${measuredClientSizeViewport.height} extended-browser=${extendedCeilingViewport.width}x${extendedCeilingViewport.height} fallback-browsers=${compactFallbackViewports.map((viewport) => `${viewport.width}x${viewport.height}`).join(",")} screenshots=${captureAllViewportScreenshots ? "all" : "large-browser-fixtures"}`,
    );
    if (largeShowMode) {
      const result = await runLargeShowViewport(client, viewports[0]);
      const passed = Boolean(
        result.stats &&
        result.stats.total === 2_000 &&
        result.stats.beforeCount <= 20 &&
        result.stats.afterCount <= 20 &&
        result.stats.viewportHeight > 0 &&
        result.stats.scrollHeight > result.stats.viewportHeight &&
        result.stats.reachedLastFixture &&
        isContained(result.containment)
      );
      console.log(`${passed ? "pass" : "fail"} large-show virtualization ${JSON.stringify(result.stats)}`);
      if (!passed) throw new Error(`Large-show UI virtualization failed: ${JSON.stringify(result)}`);
      return;
    }
    if (vjEmptyMode) {
      for (const viewport of viewports) {
        const result = await runEmptyVjViewport(client, viewport);
        const passed =
          isContained(result) &&
          result.visibleVideoControlPanelCount === 1 &&
          result.visibleVjFirstRunCount === 1 &&
          result.visibleVjFirstRunButtonCount === 1 &&
          result.fullyVisibleVjFirstRunButtonCount === 1 &&
          result.disabledVjFirstRunButtonCount === 1 &&
          result.visibleVjFirstRunSafetyCount === 1 &&
          result.visibleVjPreviewTransportCount === 1 &&
          result.visibleVjPreviewTransportButtonCount === 4 &&
          result.disabledVjPreviewTransportButtonCount === 4 &&
          result.undersizedVjPreviewTransportButtonCount === 0;
        console.log(`${passed ? "pass" : "fail"} empty VJ first-run ${viewport.width}x${viewport.height} ${JSON.stringify({
          visible: result.visibleVjFirstRunCount,
          buttons: result.visibleVjFirstRunButtonCount,
          fullyVisibleButtons: result.fullyVisibleVjFirstRunButtonCount,
          disabled: result.disabledVjFirstRunButtonCount,
          safety: result.visibleVjFirstRunSafetyCount,
          previewTransportButtons: result.visibleVjPreviewTransportButtonCount,
          disabledPreviewTransportButtons: result.disabledVjPreviewTransportButtonCount,
        })}`);
        if (!passed) throw new Error(`Empty VJ first-run viewport failed: ${JSON.stringify(result)}`);
      }
      return;
    }
    if (sceneBlockHourOnlyMode) {
      const results = [];
      for (const viewport of fullWindowTimelineViewports) {
        const result = await runSceneBlockHourViewport(client, viewport);
        results.push(result);
        console.log(`${result.passed ? "pass" : "fail"} ${result.label} ${JSON.stringify({
          initialStats: result.initialStats,
          initialRulerBounds: result.initialRulerBounds,
          revealStats: result.revealStats,
          revealRulerBounds: result.revealRulerBounds,
          dragStats: result.dragStats,
          zoomStats: result.zoomStats,
          navigationStats: result.navigationStats,
          finalRevealStats: result.finalRevealStats,
          finalRulerBounds: result.finalRulerBounds,
          screenshotVerification: result.screenshotVerification,
        })}`);
      }
      const failures = results.filter((result) => !result.passed);
      if (failures.length > 0) {
        throw new Error(`Scene Block hour viewport failed: ${JSON.stringify(failures)}`);
      }
      return;
    }
    if (sceneBlockOverlapOnlyMode) {
      const results = [];
      for (const viewport of fullWindowTimelineViewports) {
        const result = await runSceneBlockOverlapViewport(client, viewport);
        results.push(result);
        console.log(`${result.passed ? "pass" : "fail"} ${result.label} ${JSON.stringify({
          lighting: result.lighting,
          video: result.video,
          switchStats: result.switchStats,
          clippedDragStats: result.clippedDragStats,
          finalStats: result.finalStats,
          screenshotVerification: result.screenshotVerification,
        })}`);
      }
      const failures = results.filter((result) => !result.passed);
      if (failures.length > 0) {
        throw new Error(`Scene Block overlap viewport failed: ${JSON.stringify(failures)}`);
      }
      return;
    }
    if (sceneBlockOnlyMode) {
      const sceneBlockResults = [];
      for (const viewport of viewports) {
        sceneBlockResults.push(await runSceneBlockLargeViewport(client, viewport));
      }
      for (const result of sceneBlockResults) {
        console.log(`${result.passed ? "pass" : "fail"} ${result.label} ${JSON.stringify({
          markerDragStats: result.markerDragStats,
          activeTransitionStats: result.activeTransitionStats,
          showStats: result.showStats,
          sourcePickerStats: result.sourcePickerStats,
          composerPickerStats: result.composerPickerStats,
          sourceOpenStats: result.sourceOpenStats,
          screenshotVerification: result.screenshotVerification,
        })}`);
      }
      const failures = sceneBlockResults.filter((result) => !result.passed);
      if (failures.length > 0) throw new Error(`Scene Block viewport failed: ${JSON.stringify(failures)}`);
      return;
    }

    const results = [];
    for (const viewport of viewports) {
      results.push(...(await runViewport(client, viewport)));
    }
    const cueRecallResults = [];
    const cueRecallLargeResults = [];
    const effectStackLargeResults = [];
    const sceneBlockLargeResults = [];
    const cueNodeGraphResults = [];
    for (const viewport of viewports) {
      cueRecallResults.push(await runCueRecallViewport(client, viewport));
      cueRecallLargeResults.push(await runCueRecallLargeViewport(client, viewport));
      effectStackLargeResults.push(await runEffectStackLargeViewport(client, viewport));
      cueNodeGraphResults.push(await runCueNodeGraphViewport(client, viewport));
    }
    const sceneBlockScaleViewports = viewports.filter((viewport) =>
      isPrimaryOperationalViewport(viewport) ||
      isMeasuredClientSizeViewport(viewport) ||
      matchesViewport(viewport, compactFallbackViewports[0]),
    );
    for (const viewport of sceneBlockScaleViewports.length > 0 ? sceneBlockScaleViewports : viewports.slice(0, 1)) {
      sceneBlockLargeResults.push(await runSceneBlockLargeViewport(client, viewport));
    }

    const failures = results.filter((result) => !isContained(result));
    const cueRecallFailures = cueRecallResults.filter((result) => !result.passed);
    const cueRecallLargeFailures = cueRecallLargeResults.filter((result) => !result.passed);
    const effectStackLargeFailures = effectStackLargeResults.filter((result) => !result.passed);
    const sceneBlockLargeFailures = sceneBlockLargeResults.filter((result) => !result.passed);
    const cueNodeGraphFailures = cueNodeGraphResults.filter((result) => !result.passed);
    const setupSurfaceFailures = results.filter((result) => !hasExpectedSetupSurface(result));
    const projectMenuFailures = results.filter((result) => !hasExpectedProjectMenu(result));
    const localizationFailures = results.filter((result) => !hasExpectedLocalization(result));
    const keyboardNavigationFailures = results.filter((result) => !hasExpectedKeyboardNavigation(result));
    const mappingHotkeyHelpFailures = results.filter((result) => !hasExpectedMappingHotkeyHelp(result));
    const mappingWaveDraftFailures = results.filter((result) => !hasExpectedMappingWaveDraft(result));
    const controlModeFailures = results.filter((result) => !hasExpectedControlModeSurface(result));
    const touchSurfaceFailures = results.filter((result) => !hasExpectedTouchSurface(result));
    const timelineAutomationFailures = shouldCheckTimelineAutomation
      ? results.filter((result) => result.label.startsWith("control-live-") && !hasTimelineAutomationVisuals(result))
      : [];
    const sceneBlockFailures = shouldCheckTimelineAutomation
      ? results.filter((result) => !hasExpectedSceneBlocks(result))
      : [];
    const statusLineFailures = results.filter(
      (result) => result.visibleAppStatusLineCount !== 1 || !["info", "success", "warning", "error"].includes(result.appStatusTone),
    );
    for (const result of results) {
      const status = isContained(result) ? "pass" : "fail";
      const timelineSuffix = result.label.startsWith("control-live-")
        ? ` timelineAutomation=${result.timelineAutomationRangeCount}/${result.timelineAutomationHandleCount}/${result.timelineAutomationKeyframeCount}/${result.timelineAutomationChipCount}/${result.timelineAutomationCurveCount}/${result.timelineAutomationValueInputCount}/${result.timelineAutomationEnabledToggleCount}/${result.timelineAutomationScopeCount}/${result.timelineAutomationBatchButtonCount}/${result.timelineAutomationGroupButtonCount}`
        : "";
      const sceneBlockSuffix = /^control-live-\d+x\d+$/.test(result.label)
        ? ` sceneBlocks=${result.timelineSceneBlockCount}/${result.timelinePointEventCount}/${result.sceneBlockRowCount} workspace=${result.sceneBlockWorkspaceWidth}x${result.sceneBlockWorkspaceHeight}/${result.timelinePanelWidth} controls=${result.fullyVisibleSceneBlockComposerControlCount}/${result.visibleSceneBlockComposerControlCount} actions=${result.fullyVisibleSceneBlockRowActionCount}/${result.visibleSceneBlockRowActionCount} reachable=${result.sceneBlockLastControlReachable ? 1 : 0} overflow=${result.sceneBlockWorkspaceHorizontalOverflowPx}/${result.sceneBlockWorkspaceVerticalOverflowPx}/${result.sceneBlockListHorizontalOverflowPx}/${result.sceneBlockComposerHorizontalOverflowPx}`
        : "";
      const projectMenuSuffix = result.label.startsWith("project-menu-")
        ? ` projectMenu=${result.visibleProjectMenuCount}/${result.visibleProjectMenuItemCount}/${result.visibleProjectMenuShortcutCount}/${result.visibleRecentProjectMenuItemCount}/${result.visibleRecoveryProjectMenuItemCount}/${result.visibleUpdateMenuLabelCount}/${result.visibleUpdateCheckButtonCount}/${result.visibleUserTemplateMenuLabelCount}/${result.visibleUserTemplateActionCount}`
        : "";
      const keyboardSuffix = result.label.startsWith("control-edit-keyboard-")
        ? ` keyboard=${result.keyboardNavigationPassed ? 'pass' : 'fail'}`
        : "";
      const touchSuffix = result.label.startsWith("touch-")
        ? ` touch=${result.visibleTouchCuePanelCount}/${result.visibleTouchGoDeckCount}/${result.visibleTouchCuePadCount}/${result.visibleTouchStagePanelCount}/${result.visibleTouchStageCount}/${result.visibleTouchFixturePanelCount}/${result.visibleTouchFixtureScrollerCount}/${result.visibleTouchRemotePanelCount}/${result.visibleTouchRemoteUrlItemCount}/${result.visibleTouchRemoteCopyButtonCount}/${result.visibleTouchRemoteOpenButtonCount}/${result.visibleTouchVideoPanelCount}/${result.visibleTouchVideoOutputDeckCount}/${result.visibleTouchVideoSelectedOutputDeckCount}/${result.visibleTouchVideoOutputFaderCount}/${result.visibleTouchVideoOutputButtonCount}/${result.visibleTouchVideoOutputSelectButtonCount}/${result.visibleTouchVideoDeckCount}/${result.visibleTouchVideoLayerFaderCount}/${result.visibleTouchMasterGridCount}`
        : "";
      const mappingSuffix = result.label.startsWith("setup-mapping-")
        ? ` mapping=${result.mappingUseInEffectsButtonCount}/${result.mappingWaveDraftButtonCount}/${result.visibleMappingProjectorButtonCount}/${result.visibleMappingProjectorControlsCount}/${result.visibleMappingProjectorWarpGridCount}/${result.visibleMappingProjectorActionButtonCount}/${result.visibleMappingProjectorResetPoseButtonCount}/${result.visibleStageVideoSurfaceCount}`
        : "";
      const mappingHotkeyHelpSuffix = result.label.startsWith("mapping-hotkey-help-")
        ? ` mappingHelp=${result.visibleMappingHotkeyHelpCount}/${result.mappingHotkeyHelpKeyCount}`
        : "";
      const patchSuffix = result.label.startsWith("setup-patch-")
        ? ` patch=${result.visiblePatchActionRowCount}/${result.visiblePatchAutoButtonCount}/${result.visiblePatchPrimaryButtonCount}/${result.visiblePatchNextFreeButtonCount}/${result.visiblePatchFootprintCount}/${result.visibleDmxAddressGridCount}/${result.dmxAddressCellCount}/${result.dmxAddressOccupiedCellCount}/${result.dmxAddressPlannedCellCount}/${result.visibleDmxFixtureBlockCount}/${result.compactMappingStageWidth}x${result.compactMappingStageHeight}/${result.visibleDmxGridSummaryCount}/${result.visibleFixtureSetupEditorCount}/${result.visibleUseProfileForPatchButtonCount}/${result.visibleDuplicateFixtureButtonCount}`
        : "";
      const outputSetupSuffix = result.label.startsWith("setup-video-")
        ? ` outputSetup=${result.visibleSetupVideoPanelCount}/${result.visibleSetupVideoOutputDeckCount}/${result.visibleSetupVideoOutputActiveDeckCount}/${result.visibleSetupVideoOutputDetailPaneCount}/${result.visibleVideoOutputMappingPanelCount}/${result.visibleVideoOutputBlendControlsCount}/${result.visibleProjectorMapEditorCount}/${result.visibleProjectorMapHandleCount}/${result.visibleProjectorKeystoneHandleCount}/${result.visibleProjectorScaleHandleCount}/${result.visibleProjectorRotateHandleCount}/${result.visibleProjectorAspectModeButtonCount}/${result.visibleProjectorAspectPresetButtonCount}/${result.visibleProjectorResetPoseButtonCount}/${result.videoSetupSidebarWidth}w/${result.videoSetupOutputDeskWidth}w panes=${result.videoSetupRoutingPaneWidth}/${result.videoSetupMapPaneWidth}/${result.videoSetupInspectorPaneWidth} actions=${result.videoSetupCriticalActionInViewportCount}`
        : "";
      const waveDraftSuffix = result.label.startsWith("mapping-wave-draft-")
        ? ` waveDraft=${result.effectTargetValue}/${result.effectTypeValue}/${result.effectCommonAttributeValue || 'none'}`
        : "";
      const editVisualSuffix = result.label.startsWith("control-edit-position-") || result.label.startsWith("control-edit-color-")
        ? ` editVisual=${result.visiblePanTiltPadCount}/${result.visiblePositionReadoutCount}/${result.visibleColorPlaneCount}/${result.visibleColorReadoutCount}/${result.visibleGroupControlBannerCount}/${result.visibleAttributeTargetSummaryCount}/${result.visibleGroupAttributeTargetSummaryCount}`
        : "";
      const positionVisualSuffix = result.label.startsWith("control-edit-position-")
        ? ` positionDesk=${result.positionConsoleWidth}x${result.positionConsoleHeight} pad=${result.positionPadWidth}x${result.positionPadHeight} tools=${result.positionToolDeckWidth}w tabs=${result.visiblePositionToolTabCount}/${result.visibleActivePositionToolTabCount} live=${result.visiblePositionDirectControlCount}/${result.visiblePositionNudgeButtonCount}/${result.visiblePositionTargetButtonCount}/${result.visiblePositionTransformButtonCount}/${result.visiblePositionFavoriteButtonCount} limits=${result.visibleControlLimitPanelCount} overflow=${result.positionConsoleHorizontalOverflowPx}/${result.positionToolPaneHorizontalOverflowPx}/${result.positionToolPaneVerticalOverflowPx}`
        : "";
      const colorEffectSuffix = result.label.startsWith("control-edit-effects-color-editor-")
        ? ` colorFx=${result.effectTypeValue}/${result.visibleColorEffectEditorCount}/${result.visibleColorEffectStopCount}/${result.visibleColorEffectAddButtonCount}/${result.visibleColorEffectRemoveButtonCount}/${result.visibleColorEffectAlgorithmSelectCount}/${result.visibleColorEffectInterpolationSelectCount}/${result.visibleColorEffectGradientCount} legacy=${result.visibleLegacyEffectAttributeCount}/${result.visibleLegacyEffectWaveformCount}/${result.visibleLegacyEffectVideoTargetCount} overflow=${result.colorEffectHorizontalOverflowPx}`
        : "";
      const chaserEffectSuffix = result.label.startsWith("control-edit-effects-chaser-editor-")
        ? ` chaserFx=${result.effectTypeValue}/${result.visibleChaserStepCount}/${result.visibleChaserFeatureCount}/${result.chaserDirectionValue}/${result.chaserActiveStepCount}/${result.chaserSizePercent}/${result.chaserPhasePercent} active=${result.visibleActiveChaserPreviewCellCount} replace=${result.visibleChaserReplaceButtonCount} fading=${result.chaserFadingEnabled ? 1 : 0} legacy=${result.visibleLegacyEffectAttributeCount}/${result.visibleLegacyEffectWaveformCount}/${result.visibleLegacyEffectVideoTargetCount} overflow=${result.chaserHorizontalOverflowPx}`
        : "";
      const moveEffectSuffix = result.label.startsWith("control-edit-effects-move-editor-")
        ? ` moveFx=${result.effectTypeValue}/${result.visibleMoveEffectPointRowCount} editor=${result.moveEffectEditorWidth}x${result.moveEffectEditorHeight} path=${result.moveEffectPathDeskWidth}w canvas=${result.moveEffectPathCanvasWidth}x${result.moveEffectPathCanvasHeight} inspector=${result.moveEffectInspectorWidth}w columns=${result.moveEffectColumnsSideBySide ? 2 : 1} coverage=${result.moveEffectSurfaceWidthCoverage}/${result.moveEffectColumnAreaCoverage} unused=${result.moveEffectUnusedRightPx}/${result.moveEffectUnusedBottomPx} scroll=${result.moveEffectFormVerticalOverflowPx}/${result.moveEffectPointListVerticalOverflowPx} reachable=${result.moveEffectLastControlReachable ? 1 : 0}/${result.moveEffectLastPointReachable ? 1 : 0} contained=${result.moveEffectEditorContained ? 1 : 0}/${result.moveEffectCanvasContained ? 1 : 0} overflow=${result.moveEffectEditorHorizontalOverflowPx}/${result.moveEffectFormHorizontalOverflowPx}`
        : "";
      const mixerSuffix = result.label.startsWith("control-mixer-")
        ? ` mixer=${result.visibleVideoOutputItemCount}/${result.visibleVideoOutputSelectedItemCount}/${result.visibleVideoMixerOutputDeckCount}/${result.visibleVideoMixerOutputFaderCount}/${result.visibleVideoMixerOutputSelectButtonCount}/${result.visibleVideoLayerItemCount}/${result.visibleBuiltinVideoFxSelectCount}/${result.visibleVideoMixerLayerDeckCount}/${result.visibleVideoMixerLayerFaderCount}/${result.visibleVideoMixerLayerButtonCount}`
        : "";
      console.log(
        `${status} [${viewportRole({ width: result.innerWidth, height: result.innerHeight })}] ${result.label} document=${result.documentScrollWidth}x${result.documentScrollHeight} app=${result.appScrollWidth}x${result.appScrollHeight} moved=${result.movedX},${result.movedY}${keyboardSuffix}${timelineSuffix}${sceneBlockSuffix}${touchSuffix}${projectMenuSuffix}${mappingSuffix}${mappingHotkeyHelpSuffix}${patchSuffix}${outputSetupSuffix}${waveDraftSuffix}${editVisualSuffix}${positionVisualSuffix}${colorEffectSuffix}${chaserEffectSuffix}${moveEffectSuffix}${mixerSuffix}`,
      );
    }
    for (const result of cueRecallResults) {
      console.log(
        `${result.passed ? "pass" : "fail"} ${result.label} actions=${result.stats.fullyVisibleActionCount}/${result.stats.actionCount} recall=${result.stats.recallCount} outer=${result.containment.cueHostHorizontalOverflowPx}/${result.containment.cueHostVerticalOverflowPx}`,
      );
    }
    for (const result of cueRecallLargeResults) {
      console.log(
        `${result.passed ? "pass" : "fail"} ${result.label} rows=${result.before.captureRows}+${result.before.cueRows}->${result.after.captureRows}+${result.after.cueRows} open=${result.after.openCueEditors}`,
      );
    }
    for (const result of effectStackLargeResults) {
      console.log(
        `${result.passed ? "pass" : "fail"} ${result.label} rows=${result.stats.beforeCount}->${result.stats.afterCount}/${result.stats.total} page=${result.stats.pageLabel}`,
      );
    }
    for (const result of sceneBlockLargeResults) {
      console.log(
        `${result.passed ? "pass" : "fail"} ${result.label} rows=${result.showStats.beforeRowCount}->${result.showStats.afterRowCount}/500 page=${result.showStats.pagerLabel} overview=${result.showStats.overviewNodeCount} picker=${result.pickerStats.pickerCount}/${result.pickerStats.pickerOptionCount} reachable=${result.pickerStats.pickerReachable ? 1 : 0}/${result.pickerStats.pagerReachable ? 1 : 0}/${result.pickerStats.lastActionReachable ? 1 : 0} cuePlacements=${result.cueStats.placementChipCount}+${result.cueStats.placementMoreButtonCount} idle=${result.idleMutationStats.records}/${result.idleMutationStats.addedNodes}/${result.idleMutationStats.removedNodes} samples=${JSON.stringify(result.idleMutationStats.samples ?? [])}`,
      );
    }
    for (const result of cueNodeGraphResults) {
      console.log(
        `${result.passed ? "pass" : "fail"} ${result.label} all=${result.allScope.storeDisabled ? "disabled" : "enabled"}/${result.allScope.scopeErrorCount} video=${result.videoScope.storeDisabled ? "disabled" : "enabled"}/${result.videoScope.scopeErrorCount} graphs=${result.videoScope.graphCount}`,
      );
    }
    if (
      failures.length > 0 ||
      cueRecallFailures.length > 0 ||
      cueRecallLargeFailures.length > 0 ||
      effectStackLargeFailures.length > 0 ||
      sceneBlockLargeFailures.length > 0 ||
      cueNodeGraphFailures.length > 0 ||
      keyboardNavigationFailures.length > 0 ||
      setupSurfaceFailures.length > 0 ||
      projectMenuFailures.length > 0 ||
      localizationFailures.length > 0 ||
      mappingHotkeyHelpFailures.length > 0 ||
      mappingWaveDraftFailures.length > 0 ||
      controlModeFailures.length > 0 ||
      touchSurfaceFailures.length > 0 ||
      timelineAutomationFailures.length > 0 ||
      sceneBlockFailures.length > 0 ||
      statusLineFailures.length > 0
    ) {
      console.error(
        JSON.stringify(
          {
            viewport: failures,
            cueRecall: cueRecallFailures,
            cueRecallLarge: cueRecallLargeFailures,
            effectStackLarge: effectStackLargeFailures,
            sceneBlockLarge: sceneBlockLargeFailures,
            cueNodeGraph: cueNodeGraphFailures,
            keyboardNavigation: keyboardNavigationFailures,
            setupSurface: setupSurfaceFailures,
            projectMenu: projectMenuFailures,
            localization: localizationFailures,
            mappingHotkeyHelp: mappingHotkeyHelpFailures,
            mappingWaveDraft: mappingWaveDraftFailures,
            controlMode: controlModeFailures,
            touchSurface: touchSurfaceFailures,
            timelineAutomation: timelineAutomationFailures,
            sceneBlocks: sceneBlockFailures,
            statusLine: statusLineFailures,
          },
          null,
          2,
        ),
      );
      throw new Error(
        `${failures.length} viewport containment check(s), ${cueRecallFailures.length} Cue Recall fixture check(s), ${cueRecallLargeFailures.length} large Cue Recall DOM-budget check(s), ${effectStackLargeFailures.length} large live-effect DOM-budget check(s), ${sceneBlockLargeFailures.length} large Scene Block DOM-budget/layout check(s), ${cueNodeGraphFailures.length} Cue Node Graph fixture check(s), ${keyboardNavigationFailures.length} keyboard navigation check(s), ${setupSurfaceFailures.length} setup surface check(s), ${projectMenuFailures.length} project menu check(s), ${localizationFailures.length} localization check(s), ${mappingHotkeyHelpFailures.length} mapping hotkey help check(s), ${mappingWaveDraftFailures.length} mapping wave draft check(s), ${controlModeFailures.length} control mode surface check(s), ${touchSurfaceFailures.length} touch surface check(s), ${timelineAutomationFailures.length} timeline automation visual check(s), ${sceneBlockFailures.length} Scene Block full-window check(s), ${statusLineFailures.length} status line check(s) failed.`,
      );
    }
  } finally {
    client?.close();
    await stopProcess(browserProcess);
    await stopProcess(viteProcess);
    try {
      rmSync(profileDir, { recursive: true, force: true });
    } catch (error) {
      console.warn(`Viewport check passed, but cleanup could not remove ${profileDir}: ${error.message}`);
    }
  }
}

main().catch((error) => {
  console.error(error.message);
  process.exitCode = 1;
});
