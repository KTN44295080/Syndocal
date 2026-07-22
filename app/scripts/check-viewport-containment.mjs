import { spawn } from "node:child_process";
import { createHash } from "node:crypto";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { inflateSync } from "node:zlib";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const appRoot = resolve(scriptDir, "..");
const largeShowMode = process.argv.includes("--large-show");
const vjEmptyMode = process.argv.includes("--vj-empty");
const liveAudioOnlyMode = process.argv.includes("--live-audio-only");
const fullscreenVjOnlyMode = process.argv.includes("--fullscreen-vj");
const operatorVjOnlyMode = process.argv.includes("--operator-vj-only");
const autoVjOnlyMode = process.argv.includes("--auto-vj-only");
const audioReactiveOnlyMode = process.argv.includes("--audio-reactive-only");
const sceneBlockOnlyMode = process.argv.includes("--scene-block-only");
const sceneBlockHourOnlyMode = process.argv.includes("--scene-block-hour-only");
const sceneBlockOverlapOnlyMode = process.argv.includes("--scene-block-overlap-only");
const sceneMatrixOnlyMode = process.argv.includes("--scene-matrix-only");
const timelineSlimOnlyMode = process.argv.includes("--timeline-slim-only");
const patchOnlyMode = process.argv.includes("--patch-only");
const persistentBandOnlyMode = process.argv.includes("--persistent-band-only");
const workspaceSplitOnlyMode = process.argv.includes("--workspace-split-only");
const workspaceShellOnlyMode = process.argv.includes("--workspace-shell-only");
const fxVisualOnlyMode = process.argv.includes("--fx-visual-only");
const sceneLiveModifierOnlyMode = process.argv.includes("--scene-live-modifier-only");
const groupStrobeOnlyMode = process.argv.includes("--group-strobe-only");
const viewportTraceEnabled = process.env.SYNDOCAL_VIEWPORT_TRACE === "1";
const viewportFixture = process.env.SYNDOCAL_VIEWPORT_FIXTURE ?? (
  largeShowMode
    ? "large-show"
    : sceneLiveModifierOnlyMode || groupStrobeOnlyMode
      ? "scene-matrix"
      : fxVisualOnlyMode
        ? "fx-visual"
      : operatorVjOnlyMode
        ? "operator-vj"
        : autoVjOnlyMode
          ? "auto-vj"
          : audioReactiveOnlyMode
            ? "audio-reactive"
            : vjEmptyMode || liveAudioOnlyMode || fullscreenVjOnlyMode
              ? "vj-empty"
              : "timeline"
);
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
    : audioReactiveOnlyMode
      ? [primaryOperationalViewport, compactFallbackViewports[0]]
      : fullscreenVjOnlyMode || operatorVjOnlyMode
        ? [primaryOperationalViewport, ...compactFallbackViewports]
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
  { id: "mixer", label: "Mixer" },
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
const traceViewport = (message) => {
  if (viewportTraceEnabled) console.log(`[viewport-trace] ${message}`);
};

function comparePersistentBandMeasurements(setupBefore, control, setupAfter) {
  // T15-P replaces T10's fixed Groups / Stage / Selections / Context columns
  // with a resizable lower-left / lower-right split. Compare stable pane hosts
  // and split boundaries; drawer content and the stage SVG are intentionally
  // excluded because they may open or resize inside the unchanged host.
  const partNames = ["groups", "stage", "context"];
  const rectDelta = (left, right) => {
    if (!left || !right) return Number.POSITIVE_INFINITY;
    return Math.max(
      Math.abs(left.x - right.x),
      Math.abs(left.y - right.y),
      Math.abs(left.width - right.width),
      Math.abs(left.height - right.height),
    );
  };
  const setupToControl = Object.fromEntries(partNames.map((part) => [
    part,
    rectDelta(setupBefore.persistentBandRects[part], control.persistentBandRects[part]),
  ]));
  const controlToSetup = Object.fromEntries(partNames.map((part) => [
    part,
    rectDelta(control.persistentBandRects[part], setupAfter.persistentBandRects[part]),
  ]));
  const splitterNames = ["upperLower", "lowerLeftRight"];
  const setupToControlSplitters = Object.fromEntries(splitterNames.map((splitter) => [
    splitter,
    rectDelta(setupBefore.workspaceSplitterRects?.[splitter], control.workspaceSplitterRects?.[splitter]),
  ]));
  const controlToSetupSplitters = Object.fromEntries(splitterNames.map((splitter) => [
    splitter,
    rectDelta(control.workspaceSplitterRects?.[splitter], setupAfter.workspaceSplitterRects?.[splitter]),
  ]));
  const ratioDeltas = {
    setupToControlTop: Math.abs((setupBefore.workspaceSplitRatios?.top ?? -1) - (control.workspaceSplitRatios?.top ?? -2)),
    setupToControlLower: Math.abs((setupBefore.workspaceSplitRatios?.lower ?? -1) - (control.workspaceSplitRatios?.lower ?? -2)),
    controlToSetupTop: Math.abs((control.workspaceSplitRatios?.top ?? -1) - (setupAfter.workspaceSplitRatios?.top ?? -2)),
    controlToSetupLower: Math.abs((control.workspaceSplitRatios?.lower ?? -1) - (setupAfter.workspaceSplitRatios?.lower ?? -2)),
  };
  return {
    invariant:
      [
        ...Object.values(setupToControl),
        ...Object.values(controlToSetup),
        ...Object.values(setupToControlSplitters),
        ...Object.values(controlToSetupSplitters),
      ].every((delta) => delta <= 1) &&
      Object.values(ratioDeltas).every((delta) => delta <= 0.001),
    rectsByWorkspace: {
      setupBefore: setupBefore.persistentBandRects,
      control: control.persistentBandRects,
      setupAfter: setupAfter.persistentBandRects,
    },
    splitterRectsByWorkspace: {
      setupBefore: setupBefore.workspaceSplitterRects,
      control: control.workspaceSplitterRects,
      setupAfter: setupAfter.workspaceSplitterRects,
    },
    ratiosByWorkspace: {
      setupBefore: setupBefore.workspaceSplitRatios,
      control: control.workspaceSplitRatios,
      setupAfter: setupAfter.workspaceSplitRatios,
    },
    deltas: {
      setupToControl,
      controlToSetup,
      setupToControlSplitters,
      controlToSetupSplitters,
      ratioDeltas,
    },
  };
}

function persistentBandRectsExactlyEqual(left, right) {
  if (!left || !right) return false;
  return (
    Math.abs(left.x - right.x) <= 1 &&
    Math.abs(left.y - right.y) <= 1 &&
    Math.abs(left.width - right.width) <= 1 &&
    Math.abs(left.height - right.height) <= 1
  );
}

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

async function failIfPortOccupied(port, description) {
  // A stale listener makes this run silently attach to a leftover instance
  // (stale Vite serves old code; a wedged Chromium never completes the CDP
  // WebSocket handshake), which presents as an indefinite low-CPU hang.
  // Failing fast with the owning pid is always cheaper than that hang.
  let occupied = false;
  try {
    await fetch(`http://127.0.0.1:${port}/`, { signal: AbortSignal.timeout(1500) });
    occupied = true;
  } catch (error) {
    occupied = error?.name === "TimeoutError";
  }
  if (!occupied) {
    return;
  }
  let owner = "unknown";
  try {
    const { execSync } = await import("node:child_process");
    const lines = execSync(`netstat -ano -p tcp | findstr :${port}`, { encoding: "utf8" })
      .split(/\r?\n/)
      .filter((line) => line.includes("LISTENING"));
    owner = lines.map((line) => line.trim().split(/\s+/).at(-1)).join(",") || "unknown";
  } catch {
    // netstat parsing is best-effort; the error below is still actionable.
  }
  throw new Error(
    `${description} port ${port} is already in use (pid ${owner}) - a previous harness run leaked. ` +
      `Kill it first: taskkill /PID ${owner} /T /F`,
  );
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
  if (process.platform === "win32" && typeof child.pid === "number") {
    // child.kill() only reaches the direct child on Windows; Vite/Chromium
    // grandchildren survive and keep ports 5173/9227, poisoning the next run.
    try {
      const { execSync } = await import("node:child_process");
      execSync(`taskkill /PID ${child.pid} /T /F`, { stdio: "ignore" });
      return;
    } catch {
      // Fall through to the portable path below.
    }
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
    this.closedReason = null;
  }

  async connect() {
    this.socket = new WebSocket(this.webSocketUrl);
    await new Promise((resolveConnect, rejectConnect) => {
      this.socket.addEventListener("open", resolveConnect, { once: true });
      this.socket.addEventListener("error", rejectConnect, { once: true });
    });
    // Pending CDP calls must reject when the socket dies. Without this, a
    // browser killed mid-run leaves send() promises unresolved forever: the
    // run either hangs on the Vite child handle or, if every child is gone,
    // the event loop drains and Node exits 0 with zero assertions executed
    // (reproduced on this machine by killing Chrome between fx-visual
    // resolutions). Rejecting routes the failure through main()'s
    // catch/finally so children are cleaned up and the exit code is honest.
    const failAllPending = (reason) => {
      this.closedReason = reason;
      if (this.pending.size === 0) {
        return;
      }
      const waiters = [...this.pending.values()];
      this.pending.clear();
      const error = new Error(reason);
      for (const { reject } of waiters) {
        reject(error);
      }
    };
    this.socket.addEventListener("close", (event) => {
      failAllPending(`CDP socket closed mid-run (code ${event.code}); the browser died or was killed.`);
    });
    this.socket.addEventListener("error", () => {
      failAllPending("CDP socket errored mid-run; the browser died or the connection was reset.");
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
    if (this.closedReason) {
      return Promise.reject(new Error(this.closedReason));
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

async function waitForClientCondition(client, expression, description) {
  let lastValue = null;
  for (let attempt = 0; attempt < 100; attempt += 1) {
    lastValue = await client.evaluate(expression);
    if (lastValue) return lastValue;
    await sleep(25);
  }
  throw new Error(description + " did not settle. Last value: " + JSON.stringify(lastValue));
}

function installLiveAudioMockInPage() {
  const delay = (ms) => new Promise((resolveDelay) => window.setTimeout(resolveDelay, ms));
  const clone = (value) => JSON.parse(JSON.stringify(value));
  const stoppedStatus = () => ({
    running: false,
    stale: false,
    safety_clear_pending: false,
    device_id: null,
    device_name: null,
    backend: null,
    sample_format: null,
    sample_rate: 0,
    channels: 0,
    configured_buffer_frames: null,
    applied_buffer_frames: null,
    channel_mix: { mode: "average_all" },
    bass: 0,
    mid: 0,
    high: 0,
    bands: Array.from({ length: 16 }, () => 0),
    band_count: 0,
    rms: 0,
    peak: 0,
    spectral_flux: 0,
    onset: false,
    onset_strength: 0,
    bpm: null,
    bpm_confidence: 0,
    beat_phase: 0,
    feature_sequence: 0,
    analyzed_windows: 0,
    dropped_chunks: 0,
    dropped_frames: 0,
    backend_xruns: 0,
    callback_count: 0,
    last_callback_frames: 0,
    min_callback_frames: 0,
    max_callback_frames: 0,
    capture_to_worker_us: 0,
    max_capture_to_worker_us: 0,
    queue_depth: 0,
    queue_capacity: 4,
    queue_depth_high_water: 0,
    last_error: null,
  });
  const mock = {
    calls: [],
    status: stoppedStatus(),
    deviceGeneration: 0,
    asioGeneration: 0,
  };
  const invoke = async (command, args = {}) => {
    if (command === "get_live_video_monitor_frame") {
      const packet = Array.from({ length: 40 }, () => 0);
      packet[0] = 0x53;
      packet[1] = 0x59;
      packet[2] = 0x4c;
      packet[3] = 0x56;
      packet[4] = 1;
      packet[5] = 1;
      packet[6] = args.monitorKind === "preview" ? 1 : 0;
      return packet;
    }
    mock.calls.push({ command, args: clone(args) });
    if (command === "live_audio_input_backends") {
      await delay(40);
      return [
        {
          id: "wasapi_shared",
          label: "WASAPI shared",
          built: true,
          requires_explicit_device: false,
          distribution: "MIT default artifact",
        },
        {
          id: "asio",
          label: "ASIO",
          built: true,
          requires_explicit_device: true,
          distribution: "GPL-3.0-or-proprietary feature build",
        },
      ];
    }
    if (command === "list_audio_input_devices") {
      await delay(120);
      if (args.backend === "asio") {
        mock.asioGeneration += 1;
        const generation = mock.asioGeneration;
        return [{
          id: `viewport-asio-studio-g${generation}`,
          name: "Viewport ASIO Studio Driver",
          label: "Viewport ASIO Studio Driver · ASIO",
          backend: "ASIO",
        }];
      }
      mock.deviceGeneration += 1;
      if (mock.deviceGeneration === 1) {
        return [
          { id: "viewport-wasapi-studio-g1", name: "Viewport Studio Microphone", label: "Viewport Studio Microphone · WASAPI", backend: "WASAPI" },
          { id: "viewport-wasapi-line-g1", name: "Viewport Line Input", label: "Viewport Line Input · WASAPI", backend: "WASAPI" },
        ];
      }
      if (mock.deviceGeneration === 2) {
        return [
          { id: "viewport-wasapi-studio-g2", name: "Viewport Studio Microphone", label: "Viewport Studio Microphone · WASAPI", backend: "WASAPI" },
          { id: "viewport-wasapi-line-g2", name: "Viewport Line Input", label: "Viewport Line Input · WASAPI", backend: "WASAPI" },
        ];
      }
      return [
        { id: "viewport-wasapi-studio-g3a", name: "Viewport Studio Microphone", label: "Viewport Studio Microphone · WASAPI (#1)", backend: "WASAPI" },
        { id: "viewport-wasapi-studio-g3b", name: "Viewport Studio Microphone", label: "Viewport Studio Microphone · WASAPI (#2)", backend: "WASAPI" },
        { id: "viewport-wasapi-line-g3", name: "Viewport Line Input", label: "Viewport Line Input · WASAPI", backend: "WASAPI" },
      ];
    }
    if (command === "get_live_audio_input_capabilities") {
      await delay(100);
      const sampleRate = Number(args.sampleRate) || 48_000;
      const asio = args.backend === "asio";
      return {
        device_id: args.deviceId ?? null,
        device_name: args.deviceId
          ? (asio ? "Viewport ASIO Studio Driver" : "Viewport Studio Microphone")
          : "System default",
        backend: asio ? "ASIO" : "WASAPI",
        default_config: {
          channels: 2,
          sample_rate: 48_000,
          sample_format: "f32",
        },
        supported_configs: [
          {
            channels: 2,
            min_sample_rate: 44_100,
            max_sample_rate: 192_000,
            sample_format: "f32",
            buffer_size: { kind: "range", min_frames: 64, max_frames: 8_192 },
          },
        ],
        resolved_config: {
          channels: 2,
          sample_rate: sampleRate,
          sample_format: "f32",
          buffer_size: { kind: "range", min_frames: 64, max_frames: 8_192 },
        },
        max_capture_frames: 8_192,
      };
    }
    if (command === "start_live_audio_input") {
      await delay(120);
      const request = args.request ?? {};
      mock.status = {
        running: true,
        stale: false,
        safety_clear_pending: false,
        device_id: request.device_id ?? null,
        device_name: "Viewport Studio Microphone",
        backend: "WASAPI",
        sample_format: request.sample_format ?? "f32",
        sample_rate: request.sample_rate ?? 48_000,
        channels: request.stream_channels ?? 2,
        configured_buffer_frames: request.buffer_frames ?? null,
        applied_buffer_frames: request.buffer_frames ?? 512,
        channel_mix: request.channel_mix ?? { mode: "average_all" },
        bass: 0.42,
        mid: 0.58,
        high: 0.76,
        bands: Array.from({ length: 16 }, (_, index) => (index + 1) * 0.05),
        band_count: 16,
        rms: 0.64,
        peak: 0.91,
        spectral_flux: 0.33,
        onset: true,
        onset_strength: 0.82,
        bpm: 128,
        bpm_confidence: 0.87,
        beat_phase: 0.25,
        feature_sequence: 123,
        analyzed_windows: 123_456,
        dropped_chunks: 9_999,
        dropped_frames: 999_999,
        backend_xruns: 7,
        callback_count: 999_999,
        last_callback_frames: 8_192,
        min_callback_frames: 64,
        max_callback_frames: 8_192,
        capture_to_worker_us: 999_900,
        max_capture_to_worker_us: 9_999_900,
        queue_depth: 4,
        queue_capacity: 4,
        queue_depth_high_water: 4,
        last_error: null,
      };
      return clone(mock.status);
    }
    if (command === "stop_live_audio_input") {
      await delay(120);
      mock.status = {
        ...stoppedStatus(),
        safety_clear_pending: true,
        device_id: "viewport-wasapi-studio-g3a",
        device_name: "Viewport Studio Microphone",
        backend: "WASAPI",
        sample_format: "f32",
        sample_rate: 192_000,
        channels: 2,
        configured_buffer_frames: 8_192,
        applied_buffer_frames: 8_192,
        channel_mix: { mode: "stereo_pair", left_channel_index: 0, right_channel_index: 1 },
        callback_count: 999_999,
        last_callback_frames: 8_192,
        min_callback_frames: 64,
        max_callback_frames: 8_192,
        queue_capacity: 4,
        queue_depth_high_water: 4,
      };
      return clone(mock.status);
    }
    if (command === "live_audio_input_status") return clone(mock.status);
    if (command === "live_audio_input_levels") {
      return {
        running: mock.status.running,
        stale: mock.status.stale,
        safety_clear_pending: mock.status.safety_clear_pending,
        bass: mock.status.bass,
        mid: mock.status.mid,
        high: mock.status.high,
        bands: clone(mock.status.bands),
        band_count: mock.status.band_count,
        rms: mock.status.rms,
        peak: mock.status.peak,
        spectral_flux: mock.status.spectral_flux,
        onset: mock.status.onset,
        onset_strength: mock.status.onset_strength,
        bpm: mock.status.bpm,
        bpm_confidence: mock.status.bpm_confidence,
        beat_phase: mock.status.beat_phase,
        feature_sequence: mock.status.feature_sequence,
      };
    }
    throw new Error("Unexpected live-audio viewport invoke: " + command);
  };
  Object.defineProperty(window, "__TAURI_INTERNALS__", {
    configurable: true,
    writable: true,
    value: { invoke },
  });
  window.__syndocalLiveAudioMock = mock;
}

async function installLiveAudioInvokeMock(client) {
  await client.evaluate("(" + installLiveAudioMockInPage.toString() + ")()");
}

function readLiveAudioRailStateInPage() {
  const rail = document.querySelector(".videoMixerClipPane > .liveAudioInputBar");
  if (!rail) return null;
  const action = rail.querySelector('[data-live-audio-action="transport"]');
  const refresh = rail.querySelector('[data-live-audio-action="refresh"]');
  const device = rail.querySelector('[data-live-audio-control="device"]');
  const backend = rail.querySelector('[data-live-audio-control="backend"]');
  const sampleRate = rail.querySelector('[data-live-audio-control="rate"]');
  const buffer = rail.querySelector('[data-live-audio-control="buffer"]');
  const mix = rail.querySelector('[data-live-audio-control="mix"]');
  const optionValues = (select) =>
    select ? [...select.options].map((option) => option.value) : [];
  const optionLabels = (select) =>
    select ? [...select.options].map((option) => (option.textContent ?? "").trim()) : [];
  const meters = [...rail.querySelectorAll('.liveAudioMeters [role="meter"]')].map((meter) => ({
    label: meter.getAttribute("aria-label") ?? "",
    now: Number(meter.getAttribute("aria-valuenow") ?? -1),
    valueText: meter.getAttribute("aria-valuetext") ?? "",
    title: meter.getAttribute("title") ?? "",
  }));
  const visualBands = [...rail.querySelectorAll(".liveAudioBandSpectrum > em")].map((band) => ({
    title: band.getAttribute("title") ?? "",
    transform: band.querySelector("i")?.style.transform ?? "",
    role: band.getAttribute("role") ?? "",
  }));
  return {
    health: rail.getAttribute("data-health") ?? "",
    backend: backend?.value ?? rail.getAttribute("data-live-audio-backend") ?? "",
    backendState: rail.getAttribute("data-live-audio-backend-state") ?? "",
    backendBuilt: rail.getAttribute("data-live-audio-backend-built") ?? "",
    backendOptions: optionValues(backend),
    actionText: (action?.textContent ?? "").trim(),
    actionDisabled: Boolean(action?.disabled),
    refreshDisabled: Boolean(refresh?.disabled),
    deviceDisabled: Boolean(device?.disabled),
    deviceTitle: device?.getAttribute("title") ?? "",
    deviceInvalid: device?.getAttribute("aria-invalid") ?? "",
    selectedDevice: device?.value ?? "",
    deviceOptions: optionValues(device),
    deviceOptionLabels: optionLabels(device),
    selectedSampleRate: sampleRate?.value ?? "",
    sampleRateOptions: optionValues(sampleRate),
    selectedBuffer: buffer?.value ?? "",
    bufferOptions: optionValues(buffer),
    mixOptions: optionValues(mix),
    configFormat: (rail.querySelector(".liveAudioConfigFormat")?.textContent ?? "").trim(),
    meters,
    visualBands,
    onset: rail.querySelector(".liveAudioMeters")?.getAttribute("data-onset") ?? "",
    rhythm: (rail.querySelector(".liveAudioTelemetryRhythm")?.textContent ?? "").trim(),
    xrunWarning: rail.querySelector(".liveAudioTelemetryXrun")?.classList.contains("warning") ?? false,
    telemetry: [...rail.querySelectorAll(":scope > .liveAudioTelemetry > span")]
      .map((node) => (node.textContent ?? "").trim().replace(/\s+/g, " ")),
    safetyMessage: (rail.querySelector(".liveAudioSafetyMessage")?.textContent ?? "").trim(),
    announcement: (rail.querySelector(".liveAudioHealthAnnouncement")?.textContent ?? "").trim(),
  };
}

async function readLiveAudioRailState(client) {
  return client.evaluate("(" + readLiveAudioRailStateInPage.toString() + ")()");
}

function installOperatorVjMockInPage() {
  const clone = (value) => JSON.parse(JSON.stringify(value));
  const mock = {
    calls: [],
    transactionId: 0,
    effectsByLayerId: {},
    snapshotReadCount: 0,
    diagnosticsReadCount: 0,
    eventPulses: [],
    historyStatus: {
      can_undo: true,
      can_redo: true,
      undo_depth: 4,
      redo_depth: 2,
      undo_label: "Existing edit",
      redo_label: "Existing redo",
    },
    lastSnapshot: null,
  };
  const readSnapshot = () => {
    const source = window.__syndocalReadOperatorVjFixtureSnapshot?.();
    if (!source) throw new Error("Operator VJ fixture snapshot bridge is unavailable");
    const snapshot = clone(source);
    for (const layer of snapshot.video.layers) {
      if (Object.prototype.hasOwnProperty.call(mock.effectsByLayerId, layer.id)) {
        layer.isf_effect = clone(mock.effectsByLayerId[layer.id]);
      }
    }
    mock.snapshotReadCount += 1;
    mock.lastSnapshot = clone(snapshot);
    return snapshot;
  };
  const currentEffect = (layerId) => {
    if (Object.prototype.hasOwnProperty.call(mock.effectsByLayerId, layerId)) {
      return clone(mock.effectsByLayerId[layerId]);
    }
    const source = window.__syndocalReadOperatorVjFixtureSnapshot?.();
    return clone(source?.video?.layers?.find((layer) => layer.id === layerId)?.isf_effect ?? null);
  };
  const effectStages = (effect) => {
    if (!effect) return [];
    const { stack, ...first } = effect;
    return [first, ...(stack ?? [])];
  };
  const effectFromStages = (stages) => {
    if (stages.length === 0) return null;
    const [first, ...stack] = stages;
    return { ...first, stack };
  };
  const monitorPacket = (kind) => {
    const packet = Array.from({ length: 40 }, () => 0);
    packet[0] = 0x53;
    packet[1] = 0x59;
    packet[2] = 0x4c;
    packet[3] = 0x56;
    packet[4] = 1;
    packet[5] = 1;
    packet[6] = kind === "preview" ? 1 : 0;
    return packet;
  };
  const invoke = async (command, args = {}) => {
    mock.calls.push({ command, args: clone(args) });
    if (command === "begin_project_transaction") {
      mock.transactionId += 1;
      return mock.transactionId;
    }
    if (command === "commit_project_transaction") {
      mock.historyStatus = {
        can_undo: true,
        can_redo: false,
        undo_depth: mock.historyStatus.undo_depth + 1,
        redo_depth: 0,
        undo_label: "Set Video Layer Isf Effect",
        redo_label: null,
      };
      return clone(mock.historyStatus);
    }
    if (command === "cancel_project_transaction") {
      return clone(mock.historyStatus);
    }
    if (command === "set_video_layer_isf_effect") {
      mock.effectsByLayerId[args.layerId] = clone(args.effect);
      return null;
    }
    if (command === "set_video_layer_isf_effect_enabled") {
      const effect = currentEffect(args.layerId);
      if (!effect) throw new Error("Operator VJ fixture effect is unavailable");
      if (args.stageIndex === 0) effect.enabled = args.enabled;
      else if (effect.stack?.[args.stageIndex - 1]) effect.stack[args.stageIndex - 1].enabled = args.enabled;
      else throw new Error("Operator VJ fixture stage is unavailable");
      mock.effectsByLayerId[args.layerId] = effect;
      return null;
    }
    if (command === "move_video_layer_isf_effect") {
      const stages = effectStages(currentEffect(args.layerId));
      const targetIndex = args.stageIndex + args.delta;
      if (!stages[args.stageIndex] || !stages[targetIndex]) throw new Error("Operator VJ fixture move is unavailable");
      [stages[args.stageIndex], stages[targetIndex]] = [stages[targetIndex], stages[args.stageIndex]];
      mock.effectsByLayerId[args.layerId] = effectFromStages(stages);
      return null;
    }
    if (command === "remove_video_layer_isf_effect") {
      const stages = effectStages(currentEffect(args.layerId));
      if (!stages[args.stageIndex]) throw new Error("Operator VJ fixture removal is unavailable");
      stages.splice(args.stageIndex, 1);
      mock.effectsByLayerId[args.layerId] = effectFromStages(stages);
      return null;
    }
    if (command === "pulse_video_layer_isf_event") {
      const effect = currentEffect(args.layerId);
      const stage = args.stageIndex === 0 ? effect : effect?.stack?.[args.stageIndex - 1];
      const control = stage?.controls?.find((candidate) => candidate.name === args.controlName);
      if (!control || control.kind !== "Event") throw new Error("Operator VJ fixture Event is unavailable");
      control.value = [1, 0, 0, 0];
      mock.eventPulses.push({ ...clone(args), values: [1] });
      mock.effectsByLayerId[args.layerId] = effect;
      await new Promise((resolve) => window.setTimeout(resolve, 300));
      control.value = [0, 0, 0, 0];
      mock.eventPulses[mock.eventPulses.length - 1].values.push(0);
      mock.effectsByLayerId[args.layerId] = effect;
      return null;
    }
    if (command === "get_snapshot") return readSnapshot();
    if (command === "get_video_preview_diagnostics") {
      mock.diagnosticsReadCount += 1;
      const layerOneEffect = currentEffect(1);
      const layerOneStages = effectStages(layerOneEffect);
      const monochromeIndex = layerOneStages.findIndex((stage) => stage.label === "Monochrome");
      const monochromeEnabled = monochromeIndex >= 0 && layerOneStages[monochromeIndex]?.enabled === true;
      return {
        queue_count: 0,
        frame_queue_capacity: 0,
        still_image_cache_len: 0,
        decoder_cache_len: 0,
        decoder_diagnostics: {
          total_requests: 0,
          hap_requests: 0,
          hap_successes: 0,
          hap_failures: 0,
          libav_requests: 0,
          libav_successes: 0,
          libav_failures: 0,
          cli_fallback_requests: 0,
          cli_fallback_successes: 0,
          cli_fallback_failures: 0,
          deferred_requests: 0,
          decode_failures: 0,
          hap_cache_len: 0,
          libav_cache_len: 0,
          libav_session_count: 0,
          libav_session_open_count: 0,
          libav_session_reset_count: 0,
          libav_sequential_continue_count: 0,
          libav_frame_reuse_count: 0,
          libav_working_set_eviction_count: 0,
          libav_session_error_count: 0,
          cli_cache_len: 0,
        },
        isf_pipeline_count: 3,
        isf_last_stack_stage_count: 3,
        isf_last_stack_render_us: 620,
        last_isf_error: null,
        isf_stage_errors: monochromeEnabled ? [{
          layer_id: 1,
          stage_index: monochromeIndex,
          stage_label: "Monochrome",
          message: "Fixture GPU stage error",
        }] : [],
        prefetch_count: 0,
        prefetch_interval_ms: 0,
        bpm: 120,
        layer_queues: [],
        output_decode_previews: [],
      };
    }
    if (command === "get_live_video_monitor_frame") return monitorPacket(args.monitorKind);
    throw new Error("Unexpected Operator VJ viewport invoke: " + command);
  };
  Object.defineProperty(window, "__TAURI_INTERNALS__", {
    configurable: true,
    writable: true,
    value: { invoke },
  });
  window.__syndocalOperatorVjMock = mock;
}

async function installOperatorVjInvokeMock(client) {
  await client.evaluate("(" + installOperatorVjMockInPage.toString() + ")()");
}

function readOperatorVjStateInPage() {
  const visible = (element) => {
    if (!(element instanceof HTMLElement)) return false;
    const rect = element.getBoundingClientRect();
    const style = getComputedStyle(element);
    return rect.width > 0 && rect.height > 0 && style.display !== "none" && style.visibility !== "hidden";
  };
  const layerRoots = [...document.querySelectorAll("[data-video-isf-layer-id]")].filter(visible);
  const layerOne = document.querySelector('[data-video-isf-layer-id="1"]');
  const bypass = layerOne?.querySelector('[data-video-isf-action="bypass"]');
  const advanced = layerOne?.querySelector('[data-video-isf-action="advanced"]');
  const builtin = layerOne?.querySelector('[data-video-isf-action="builtin"]');
  const addIsf = layerOne?.querySelector('[data-video-isf-action="add-isf"]');
  const pager = document.querySelector(".videoLayerList.compact > .deckPager");
  const outputButtons = [...document.querySelectorAll(".videoOutputRailButton[data-video-output-id]")]
    .filter(visible)
    .map((button) => ({
      id: Number(button.getAttribute("data-video-output-id")),
      selected: button.getAttribute("aria-current") === "true",
      label: (button.querySelector(".videoOutputRailLabel")?.textContent ?? "").trim(),
      state: [...(button.querySelector(".videoOutputRailState")?.classList ?? [])]
        .find((name) => name.startsWith("state-")) ?? "",
      stateText: (button.querySelector(".videoOutputRailState")?.textContent ?? "").trim(),
    }));
  const visibleOutputDetails = [...document.querySelectorAll("[data-video-output-detail-id]")]
    .filter(visible)
    .map((item) => ({
      id: Number(item.getAttribute("data-video-output-detail-id")),
      label: (item.querySelector(":scope > div:first-child > strong")?.textContent ?? "").trim(),
    }));
  const program = document.querySelector('[data-live-video-monitor="program"]');
  const focused = document.activeElement;
  const mock = window.__syndocalOperatorVjMock;
  return {
    lang: document.documentElement.lang,
    windowMode: document.documentElement.getAttribute("data-window-mode") ?? "",
    layerIds: layerRoots.map((root) => Number(root.getAttribute("data-video-isf-layer-id"))),
    advancedDomCount: document.querySelectorAll(".videoIsfAdvanced").length,
    layerOne: layerOne ? {
      label: (layerOne.querySelector(".videoIsfQuickStatus [data-no-localize]")?.textContent ?? "").trim(),
      statusText: (layerOne.querySelector(".videoIsfQuickStatus")?.textContent ?? "").trim().replace(/\s+/g, " "),
      controlCount: layerOne.querySelectorAll(".videoIsfAdvanced .videoIsfControl").length,
      bypassPressed: bypass?.getAttribute("aria-pressed") ?? "",
      bypassText: (bypass?.textContent ?? "").trim(),
      bypassActive: bypass?.classList.contains("active") ?? false,
      bypassAriaLabel: bypass?.getAttribute("aria-label") ?? "",
      advancedExpanded: advanced?.getAttribute("aria-expanded") ?? "",
      advancedText: (advanced?.textContent ?? "").trim(),
      advancedAriaLabel: advanced?.getAttribute("aria-label") ?? "",
      builtinAriaLabel: builtin?.getAttribute("aria-label") ?? "",
      builtinDisabled: builtin?.disabled ?? null,
      addIsfDisabled: addIsf?.disabled ?? null,
      stackCountText: (layerOne.querySelector(".videoIsfStackActions > span")?.textContent ?? "").trim(),
      controlRows: [...layerOne.querySelectorAll(".videoIsfAdvanced .videoIsfControl")]
        .map((row) => ({
          kind: row.getAttribute("data-video-isf-control-kind") ?? "",
          name: row.getAttribute("data-video-isf-control-name") ?? "",
          controls: [...row.querySelectorAll("input, button")].map((control) => ({
            tag: control.tagName.toLowerCase(),
            type: control instanceof HTMLInputElement ? control.type : "button",
            step: control instanceof HTMLInputElement ? control.step : "",
            ariaLabel: control.getAttribute("aria-label") ?? "",
          })),
        })),
      stackLabels: [...layerOne.querySelectorAll(".videoIsfStackRow .videoIsfStageSelect strong")]
        .map((label) => (label.textContent ?? "").trim()),
      stackRowCount: layerOne.querySelectorAll(".videoIsfStackRow").length,
      selectedStackIndex: Number(layerOne.querySelector(".videoIsfStackRow.selected")?.getAttribute("data-video-isf-stage-index") ?? -1),
      runtimeErrorCount: layerOne.querySelectorAll(".videoIsfRuntimeBadge").length,
      focusedAction: focused?.getAttribute("data-video-isf-action") ?? "",
      focusedStageIndex: Number(focused?.closest?.("[data-video-isf-stage-index]")?.getAttribute("data-video-isf-stage-index") ?? -1),
    } : null,
    pagerHeading: (pager?.querySelector("strong")?.textContent ?? "").trim(),
    pagerText: (pager?.querySelector("span")?.textContent ?? "").trim().replace(/\s+/g, " "),
    outputButtons,
    selectedOutputId: outputButtons.find((output) => output.selected)?.id ?? null,
    visibleOutputDetails,
    programLabel: (program?.querySelector("header span")?.textContent ?? "").trim(),
    mock: mock ? {
      calls: cloneForOperatorVjRead(mock.calls),
      snapshotReadCount: mock.snapshotReadCount,
      diagnosticsReadCount: mock.diagnosticsReadCount,
      eventPulses: cloneForOperatorVjRead(mock.eventPulses),
      historyStatus: cloneForOperatorVjRead(mock.historyStatus),
      lastLayerOneEnabled: mock.lastSnapshot?.video?.layers?.find((layer) => layer.id === 1)?.isf_effect?.enabled ?? null,
      storedLayerOneEnabled: mock.effectsByLayerId?.[1]?.enabled ?? null,
      storedEventValue: mock.effectsByLayerId?.[1]?.controls?.find((control) => control.name === "pulse")?.value?.[0] ?? null,
    } : null,
  };
}

function cloneForOperatorVjRead(value) {
  return JSON.parse(JSON.stringify(value));
}

async function readOperatorVjState(client) {
  return client.evaluate(`(() => {
    const cloneForOperatorVjRead = ${cloneForOperatorVjRead.toString()};
    return (${readOperatorVjStateInPage.toString()})();
  })()`);
}

function measureOperatorVjLayoutInPage(layerId) {
  const rect = (element) => {
    if (!(element instanceof HTMLElement)) return null;
    const bounds = element.getBoundingClientRect();
    return { left: bounds.left, top: bounds.top, right: bounds.right, bottom: bounds.bottom, width: bounds.width, height: bounds.height };
  };
  const contained = (child, parent) => Boolean(child && parent &&
    child.left >= parent.left - 1 && child.right <= parent.right + 1 &&
    child.top >= parent.top - 1 && child.bottom <= parent.bottom + 1);
  const viewport = { left: 0, top: 0, right: innerWidth, bottom: innerHeight };
  const containerSelectors = {
    clipPane: ".videoMixerClipPane",
    programPane: ".videoMixerProgramPane",
    layerPane: ".videoMixerLayerPane",
    layerList: ".videoLayerList.compact",
    outputList: ".videoOutputControlList.compact",
    outputRail: ".videoOutputSelectorRail",
  };
  const containers = Object.entries(containerSelectors).map(([name, selector]) => {
    const element = document.querySelector(selector);
    const style = element ? getComputedStyle(element) : null;
    const overflowX = element ? Math.max(0, element.scrollWidth - element.clientWidth) : -1;
    const overflowY = element ? Math.max(0, element.scrollHeight - element.clientHeight) : -1;
    return {
      name,
      found: Boolean(element),
      overflowX,
      overflowY,
      overflowXMode: style?.overflowX ?? "",
      overflowYMode: style?.overflowY ?? "",
      unsafeX: overflowX > 1 && !/(auto|scroll)/.test(style?.overflowX ?? ""),
      unsafeY: overflowY > 1 && !/(auto|scroll)/.test(style?.overflowY ?? ""),
      rect: rect(element),
    };
  });
  const layerPane = rect(document.querySelector(".videoMixerLayerPane"));
  const programPane = rect(document.querySelector(".videoMixerProgramPane"));
  const layerRoot = rect(document.querySelector(`[data-video-isf-layer-id="${layerId}"]`));
  const advancedElement = document.querySelector(`[data-video-isf-layer-id="${layerId}"] .videoIsfAdvanced`);
  const advanced = rect(advancedElement);
  const advancedHorizontalBounds = advanced
    ? { ...advanced, top: Number.NEGATIVE_INFINITY, bottom: Number.POSITIVE_INFINITY }
    : null;
  const layerActions = [...document.querySelectorAll(
    `[data-video-isf-layer-id="${layerId}"] [data-video-isf-action]`,
  )]
    .filter((action) => !action.closest(".videoIsfAdvanced"))
    .map(rect);
  const advancedControls = [...document.querySelectorAll(
    `[data-video-isf-layer-id="${layerId}"] .videoIsfAdvanced input, ` +
    `[data-video-isf-layer-id="${layerId}"] .videoIsfAdvanced select, ` +
    `[data-video-isf-layer-id="${layerId}"] .videoIsfAdvanced button`,
  )].map(rect);
  const outputRail = rect(document.querySelector(".videoOutputSelectorRail"));
  const outputDetail = rect([...document.querySelectorAll("[data-video-output-detail-id]")]
    .find((element) => element.getBoundingClientRect().width > 0 && element.getBoundingClientRect().height > 0));
  const programMonitor = rect(document.querySelector('[data-live-video-monitor="program"]'));
  const programLabel = rect(document.querySelector('[data-live-video-monitor="program"] header span'));
  const clipGridElement = document.querySelector(".videoMixerClipPane .videoClipGrid");
  const audioSyncElement = document.querySelector(".videoMixerClipPane .videoAudioSyncStatus");
  const recordingTelemetryElement = document.querySelector(".videoMixerClipPane .videoRecordingBar span");
  const recordingTelemetryRect = rect(recordingTelemetryElement);
  const railButtons = [...document.querySelectorAll(".videoOutputRailButton[data-video-output-id]")].map(rect);
  const targets = [
    ["clipPane", containers.find((entry) => entry.name === "clipPane")?.rect, viewport],
    ["programPane", programPane, viewport],
    ["layerPane", layerPane, viewport],
    ["layerRoot", layerRoot, layerPane],
    ["advanced", advanced, layerPane],
    ["outputRail", outputRail, programPane],
    ["outputDetail", outputDetail, programPane],
    ["programMonitor", programMonitor, programPane],
    ["programLabel", programLabel, programMonitor],
    ...layerActions.map((action, index) => [`layerAction${index + 1}`, action, layerRoot]),
    ...advancedControls.map((control, index) => [`advancedControl${index + 1}`, control, advancedHorizontalBounds]),
    ...railButtons.map((button, index) => [`outputRailButton${index + 1}`, button, outputRail]),
  ].filter(([, child]) => child);
  const rectContainment = targets.map(([name, child, parent]) => ({ name, contained: contained(child, parent), rect: child }));
  return {
    innerWidth,
    innerHeight,
    containers,
    advancedViewport: advanced ? {
      height: advanced.height,
      clientHeight: advancedElement?.clientHeight ?? 0,
      scrollHeight: advancedElement?.scrollHeight ?? 0,
    } : null,
    lowHeightClipGrid: {
      clipGridRow: clipGridElement ? getComputedStyle(clipGridElement).gridRowStart : "",
      audioSyncRow: audioSyncElement ? getComputedStyle(audioSyncElement).gridRowStart : "",
      recordingTelemetryText: (recordingTelemetryElement?.textContent ?? "").trim(),
      recordingTelemetryVisible: Boolean(recordingTelemetryRect && recordingTelemetryRect.width > 0 && recordingTelemetryRect.height > 0),
    },
    unsafeOverflowCount: containers.filter((entry) => !entry.found || entry.unsafeX || entry.unsafeY).length,
    rectContainment,
    outsideRectCount: rectContainment.filter((entry) => !entry.contained).length,
  };
}

async function measureOperatorVjLayout(client, layerId) {
  return evaluatePageFunction(client, measureOperatorVjLayoutInPage, layerId);
}

function installAutoVjMockInPage() {
  const delay = (ms) => new Promise((resolveDelay) => window.setTimeout(resolveDelay, ms));
  const clone = (value) => JSON.parse(JSON.stringify(value));
  const offStatus = (showRevision = 0) => ({
    mode: "Off",
    armed: false,
    hold: false,
    show_revision: showRevision,
    action_sequence: 0,
    last_consumed_boundary: null,
    next_boundary_beat: null,
    last_action: null,
    action_log: [],
    fault: null,
    live_audio_beat_counter: 0,
    last_live_audio_feature_sequence: null,
  });
  const mock = {
    calls: [],
    transactionId: 0,
    audioActiveLayerIds: [1],
    programAudioHandoffConfig: null,
    autoVj: {
      config: {
        eligible_layer_ids: [],
        seed: 0,
        beats_per_change: 4,
        transition_ms: 500,
        avoid_immediate_repeat: true,
        rhythm_source: "Clock",
      },
      status: offStatus(),
    },
  };
  const validateConfig = (config) => {
    const ids = config?.eligible_layer_ids;
    if (!Array.isArray(ids) || ids.length === 0) {
      throw new Error("Auto VJ requires at least one candidate video layer");
    }
    if (new Set(ids).size !== ids.length || ids.some((id) => !Number.isInteger(id) || id <= 0)) {
      throw new Error("Auto VJ candidate video layer IDs must be unique and non-zero");
    }
    if (config.rhythm_source !== "Clock" && config.rhythm_source !== "LiveAudio") {
      throw new Error("Auto VJ rhythm source is invalid");
    }
  };
  const readSnapshot = () => {
    const source = window.__syndocalReadAutoVjFixtureSnapshot?.();
    if (!source) throw new Error("Auto VJ fixture snapshot bridge is unavailable");
    const snapshot = clone(source);
    snapshot.video.auto_vj = clone(mock.autoVj);
    return snapshot;
  };
  const invoke = async (command, args = {}) => {
    if (command === "get_live_video_monitor_frame") {
      const packet = Array.from({ length: 40 }, () => 0);
      packet[0] = 0x53;
      packet[1] = 0x59;
      packet[2] = 0x4c;
      packet[3] = 0x56;
      packet[4] = 1;
      packet[5] = 1;
      packet[6] = args.monitorKind === "preview" ? 1 : 0;
      return packet;
    }
    mock.calls.push({ command, args: clone(args) });
    if (command === "begin_project_transaction") {
      mock.transactionId += 1;
      return mock.transactionId;
    }
    if (command === "commit_project_transaction" || command === "cancel_project_transaction") {
      return {
        can_undo: command === "commit_project_transaction",
        can_redo: false,
        undo_depth: command === "commit_project_transaction" ? mock.transactionId : 0,
        redo_depth: 0,
        undo_label: command === "commit_project_transaction" ? "Set Auto Vj Config" : null,
        redo_label: null,
      };
    }
    if (command === "set_auto_vj_config") {
      await delay(45);
      validateConfig(args.config);
      const revision = mock.autoVj.status.show_revision + 1;
      mock.autoVj = { config: clone(args.config), status: offStatus(revision) };
      return null;
    }
    if (command === "set_auto_vj_armed") {
      await delay(45);
      if (args.armed) {
        validateConfig(mock.autoVj.config);
        mock.autoVj.status = {
          ...offStatus(mock.autoVj.status.show_revision),
          mode: "Armed",
          armed: true,
          last_consumed_boundary: 0,
          next_boundary_beat: mock.autoVj.config.beats_per_change,
        };
      } else {
        mock.autoVj.status = offStatus(mock.autoVj.status.show_revision);
      }
      return null;
    }
    if (command === "set_auto_vj_hold") {
      await delay(45);
      if (!mock.autoVj.status.armed) throw new Error("Auto VJ must be armed before Hold can change");
      if (args.hold) {
        mock.autoVj.status = { ...mock.autoVj.status, mode: "Hold", hold: true };
      } else {
        const beat = mock.autoVj.config.beats_per_change;
        const action = {
          sequence: 1,
          boundary_index: 1,
          beat,
          layer_id: mock.autoVj.config.eligible_layer_ids[1] ?? mock.autoVj.config.eligible_layer_ids[0],
          transition_ms: mock.autoVj.config.transition_ms,
          selection_token: 1_234_567,
          seed: mock.autoVj.config.seed,
          show_revision: mock.autoVj.status.show_revision,
          trigger: mock.autoVj.config.rhythm_source === "LiveAudio" ? "LiveAudioOnset" : "ClockBoundary",
          live_audio_feature_sequence: mock.autoVj.config.rhythm_source === "LiveAudio" ? 77 : null,
        };
        mock.autoVj.status = {
          ...mock.autoVj.status,
          mode: "Running",
          hold: false,
          action_sequence: 1,
          last_consumed_boundary: 1,
          next_boundary_beat: beat * 2,
          last_action: action,
          action_log: [action],
          live_audio_beat_counter: mock.autoVj.config.rhythm_source === "LiveAudio" ? beat : 0,
          last_live_audio_feature_sequence: action.live_audio_feature_sequence,
        };
      }
      return null;
    }
    if (command === "get_snapshot") {
      await delay(20);
      return readSnapshot();
    }
    if (command === "get_video_layer_thumbnail") {
      return {
        layer_id: args.layerId,
        width: 1,
        height: 1,
        pts_ms: 0,
        duration_ms: 1,
        format: "Rgba8",
        data: [32, 38, 42, 255],
      };
    }
    if (command === "list_audio_output_devices") {
      await delay(15);
      return ["Viewport ASIO Program Output"];
    }
    if (command === "set_program_audio_handoff_config") {
      await delay(15);
      if (
        typeof args.enabled !== "boolean" ||
        typeof args.volume !== "number" ||
        !Number.isFinite(args.volume) ||
        (args.deviceName !== null && typeof args.deviceName !== "string")
      ) {
        throw new Error("Invalid Program audio handoff config: " + JSON.stringify(args));
      }
      mock.programAudioHandoffConfig = clone(args);
      return null;
    }
    if (command === "stop_video_layer_audio_monitor") {
      await delay(15);
      mock.audioActiveLayerIds = mock.audioActiveLayerIds.filter((layerId) => layerId !== args.layerId);
      return {
        output_open: mock.audioActiveLayerIds.length > 0,
        active_layer_ids: clone(mock.audioActiveLayerIds),
        resync_count: 0,
        last_drift_ms: 0,
        max_abs_drift_ms: 0,
        last_sync_error: null,
      };
    }
    if (command === "play_video_layer_audio_monitor") {
      await delay(15);
      mock.audioActiveLayerIds = [args.layerId];
      return {
        output_open: true,
        active_layer_ids: clone(mock.audioActiveLayerIds),
        resync_count: 1,
        last_drift_ms: 0,
        max_abs_drift_ms: 0,
        last_sync_error: null,
      };
    }
    throw new Error("Unexpected Auto VJ viewport invoke: " + command);
  };
  Object.defineProperty(window, "__TAURI_INTERNALS__", {
    configurable: true,
    writable: true,
    value: { invoke },
  });
  window.__syndocalAutoVjMock = mock;
}

async function installAutoVjInvokeMock(client) {
  await client.evaluate("(" + installAutoVjMockInPage.toString() + ")()");
}

async function setProgramAudioMonitorVolume(client, volume) {
  return client.evaluate(`(() => {
    const input = document.querySelector('.videoClipGridSettings input[type="number"][max="2"]');
    if (!input) return false;
    input.value = ${JSON.stringify(volume)};
    input.dispatchEvent(new InputEvent('input', { bubbles: true, inputType: 'insertText' }));
    return true;
  })()`);
}

async function refreshProgramAudioOutputDevices(client) {
  return client.evaluate(`(() => {
    const button = document.querySelector('.videoAudioDeviceField button');
    if (!button) return false;
    button.click();
    return true;
  })()`);
}

async function setProgramAudioOutputDevice(client, deviceName) {
  return client.evaluate(`(() => {
    const select = document.querySelector('.videoAudioDeviceField select');
    if (!select) return false;
    select.value = ${JSON.stringify(deviceName)};
    select.dispatchEvent(new InputEvent('input', { bubbles: true, inputType: 'insertText' }));
    return select.value === ${JSON.stringify(deviceName)};
  })()`);
}

async function setProgramAudioEnabled(client, enabled) {
  return client.evaluate(`(() => {
    const input = document.querySelector('.videoProgramAudioToggle input[type="checkbox"]');
    if (!input) return false;
    input.checked = ${JSON.stringify(enabled)};
    input.dispatchEvent(new Event('change', { bubbles: true }));
    return input.checked === ${JSON.stringify(enabled)};
  })()`);
}

function readAutoVjStateInPage() {
  const strip = document.querySelector(".videoMixerClipPane > .autoVjStrip");
  if (!strip) return null;
  const buttons = [...strip.querySelectorAll(":scope > button")];
  const arm = buttons[0];
  const hold = buttons[1];
  const rhythm = strip.querySelector(".autoVjRhythm select");
  const configControls = [...strip.querySelectorAll("select, input, .autoVjEligiblePicker > div > button")];
  const rect = strip.getBoundingClientRect();
  const readout = strip.querySelector(".autoVjReadout");
  return {
    mode: strip.getAttribute("data-mode") ?? "",
    modeText: (strip.querySelector(".autoVjIdentity strong")?.textContent ?? "").trim(),
    rhythmSource: strip.getAttribute("data-rhythm-source") ?? "",
    rhythmValue: rhythm?.value ?? "",
    rhythmOptions: rhythm ? [...rhythm.options].map((option) => option.value) : [],
    armText: (arm?.textContent ?? "").trim(),
    armDisabled: Boolean(arm?.disabled),
    armPressed: arm?.getAttribute("aria-pressed") ?? "",
    holdText: (hold?.textContent ?? "").trim(),
    holdDisabled: Boolean(hold?.disabled),
    holdPressed: hold?.getAttribute("aria-pressed") ?? "",
    clipSummary: (strip.querySelector(".autoVjEligiblePicker > summary")?.textContent ?? "").trim().replace(/\s+/g, " "),
    clipSummaryTitle: strip.querySelector(".autoVjEligiblePicker > summary")?.getAttribute("title") ?? "",
    selectedClipCount: strip.querySelectorAll('.autoVjEligiblePicker input[type="checkbox"]:checked').length,
    configControlCount: configControls.length,
    disabledConfigControlCount: configControls.filter((control) => control.disabled).length,
    readout: (readout?.querySelector("span")?.textContent ?? "").trim(),
    secondaryReadout: (readout?.querySelector("small")?.textContent ?? "").trim(),
    statusCount: strip.querySelectorAll('[role="status"]').length,
    accessibleMeterCount: document.querySelectorAll('.videoMixerClipPane > .liveAudioInputBar [role="meter"]').length,
    visualBandCount: document.querySelectorAll(".videoMixerClipPane > .liveAudioInputBar .liveAudioBandSpectrum > em").length,
    horizontalOverflowPx: Math.max(0, strip.scrollWidth - strip.clientWidth),
    outsideViewport:
      rect.left < -0.5 ||
      rect.top < -0.5 ||
      rect.right > window.innerWidth + 0.5 ||
      rect.bottom > window.innerHeight + 0.5,
  };
}

async function readAutoVjState(client) {
  return client.evaluate("(" + readAutoVjStateInPage.toString() + ")()");
}

function setAutoVjRhythmSourceInPage(value) {
  const select = document.querySelector(".videoMixerClipPane > .autoVjStrip .autoVjRhythm select");
  if (!select || select.disabled || ![...select.options].some((option) => option.value === value)) return false;
  select.value = value;
  select.dispatchEvent(new InputEvent("input", { bubbles: true, inputType: "insertReplacementText" }));
  return true;
}

async function setAutoVjRhythmSource(client, value) {
  return client.evaluate(
    "(" + setAutoVjRhythmSourceInPage.toString() + ")(" + JSON.stringify(value) + ")",
  );
}

function clickAutoVjButtonInPage(index) {
  const button = document.querySelectorAll(".videoMixerClipPane > .autoVjStrip > button")[index];
  if (!button || button.disabled) return false;
  button.click();
  return true;
}

async function clickAutoVjButton(client, index) {
  return client.evaluate("(" + clickAutoVjButtonInPage.toString() + ")(" + index + ")");
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
    const names = (node) => [
      (node.textContent || '').trim(),
      node.getAttribute('aria-label') || '',
      node.getAttribute('title') || '',
    ].map((value) => value.toLowerCase()).filter(Boolean);
    const exact = nodes.find((node) => names(node).some((name) => name === wanted));
    const loose = exact || nodes.find((node) => names(node).some((name) => name.includes(wanted)));
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
      .find((button) => [
        (button.textContent || "").trim(),
        button.getAttribute("aria-label") || "",
        button.getAttribute("title") || "",
      ].includes("Reveal Playhead"));
    revealPlayhead?.focus();
  });
  let active = null;
  let tabsToCluster = 0;
  for (let tab = 1; tab <= 40; tab += 1) {
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
    desk: document.querySelector('.timelineDeskTabs button.active')?.getAttribute('data-timeline-desk-surface') || '',
  }))()`);
  if (restored.workspace !== "Control" || restored.controlMode !== "Timeline" || restored.desk !== "playback") {
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

async function measurePersistentBand(client, label) {
  return await client.evaluate(`(() => {
    const visibleElements = (selector) => [...document.querySelectorAll(selector)].filter((element) => {
      const rect = element.getBoundingClientRect();
      const style = getComputedStyle(element);
      return rect.width > 0 && rect.height > 0 && style.display !== 'none' && style.visibility !== 'hidden';
    });
    const measuredRect = (selector) => {
      const element = visibleElements(selector)[0];
      if (!element) return null;
      const rect = element.getBoundingClientRect();
      const precision = (value) => Math.round(value * 100) / 100;
      return { x: precision(rect.x), y: precision(rect.y), width: precision(rect.width), height: precision(rect.height) };
    };
    const app = document.querySelector('.app');
    const layout = document.querySelector('.layout');
    const splitters = {
      upperLower: document.querySelector('[data-workspace-splitter="upper-lower"]'),
      lowerLeftRight: document.querySelector('[data-workspace-splitter="lower-left-right"]'),
    };
    const drawer = document.querySelector('[data-workspace-selection-drawer]');
    const drawerToggle = document.querySelector('[data-workspace-selection-drawer-toggle]');
    return {
      label: ${JSON.stringify(label)},
      visiblePersistentBandCount: visibleElements('.mappingPersistentWorkspaceBand').length,
      visiblePersistentGroupsCount: visibleElements('[data-persistent-band-part="groups"]').length,
      visiblePersistentStageCount: visibleElements('[data-persistent-band-part="stage"]').length,
      visiblePersistentSelectionsCount: visibleElements('[data-workspace-selection-drawer][open]').length,
      visiblePersistentContextCount: visibleElements('[data-persistent-band-part="context"]').length,
      visibleSelectionDrawerToggleCount: visibleElements('[data-workspace-selection-drawer-toggle]').length,
      selectionDrawerOpen: drawer instanceof HTMLDetailsElement && drawer.open,
      selectionDrawerExpandedMatches:
        drawer instanceof HTMLDetailsElement &&
        drawerToggle?.getAttribute('aria-expanded') === String(drawer.open),
      visibleWorkspaceSplitterCount: visibleElements('[data-workspace-splitter]').length,
      visibleControlStagePanelCount: visibleElements('.controlStagePanel').length,
      visibleControlStageCount: visibleElements('.controlStage').length,
      persistentBandRects: {
        groups: measuredRect('[data-persistent-band-part="groups"]'),
        stage: measuredRect('[data-workspace-pane="lower-left"]'),
        selections: measuredRect('[data-workspace-selection-drawer][open]'),
        context: measuredRect('[data-workspace-pane="lower-right"]'),
      },
      workspacePaneRects: {
        upper: measuredRect('[data-workspace-pane="upper"]'),
        lower: measuredRect('[data-workspace-pane="lower"]'),
      },
      workspaceSplitterRects: {
        upperLower: measuredRect('[data-workspace-splitter="upper-lower"]'),
        lowerLeftRight: measuredRect('[data-workspace-splitter="lower-left-right"]'),
      },
      workspaceSplitRatios: {
        top: Number(layout?.getAttribute('data-upper-lower-ratio') ?? NaN),
        lower: Number(layout?.getAttribute('data-lower-left-right-ratio') ?? NaN),
      },
      workspaceSplitterAccessibility: Object.fromEntries(Object.entries(splitters).map(([name, splitter]) => [name, splitter ? {
        role: splitter.getAttribute('role') ?? '',
        orientation: splitter.getAttribute('aria-orientation') ?? '',
        label: splitter.getAttribute('aria-label') ?? '',
        minimum: Number(splitter.getAttribute('aria-valuemin')),
        maximum: Number(splitter.getAttribute('aria-valuemax')),
        current: Number(splitter.getAttribute('aria-valuenow')),
        tabIndex: splitter.tabIndex,
      } : null])),
      outerContained:
        window.scrollX === 0 && window.scrollY === 0 &&
        document.documentElement.scrollWidth === document.documentElement.clientWidth &&
        document.documentElement.scrollHeight === document.documentElement.clientHeight &&
        document.body.scrollWidth === document.documentElement.clientWidth &&
        document.body.scrollHeight === document.documentElement.clientHeight &&
        (!app || (app.scrollWidth === app.clientWidth && app.scrollHeight === app.clientHeight)) &&
        (!layout || (layout.scrollWidth === layout.clientWidth && layout.scrollHeight === layout.clientHeight)),
    };
  })()`);
}

async function measureTimelinePaneExpansionState(client) {
  return await client.evaluate(`(async () => {
    await new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame)));
    const isVisible = (element) => {
      if (!element) return false;
      const rect = element.getBoundingClientRect();
      const style = getComputedStyle(element);
      return rect.width > 0 && rect.height > 0 && style.display !== 'none' && style.visibility !== 'hidden';
    };
    const measuredRect = (selector) => {
      const element = [...document.querySelectorAll(selector)].find(isVisible);
      if (!element) return null;
      const rect = element.getBoundingClientRect();
      const precision = (value) => Math.round(value * 100) / 100;
      return {
        x: precision(rect.x),
        y: precision(rect.y),
        width: precision(rect.width),
        height: precision(rect.height),
      };
    };
    const splitterState = (name, rootSelector, axis) => {
      const splitter = [...document.querySelectorAll('[data-workspace-splitter="' + name + '"]')].find(isVisible);
      const root = document.querySelector(rootSelector);
      if (!splitter || !root) return null;
      const splitterRect = splitter.getBoundingClientRect();
      const rootRect = root.getBoundingClientRect();
      const splitterSize = axis === 'horizontal' ? splitterRect.height : splitterRect.width;
      const usable = (axis === 'horizontal' ? rootRect.height : rootRect.width) - splitterSize;
      const first = axis === 'horizontal'
        ? splitterRect.top - rootRect.top
        : splitterRect.left - rootRect.left;
      return {
        minimum: Number(splitter.getAttribute('aria-valuemin')),
        maximum: Number(splitter.getAttribute('aria-valuemax')),
        current: Number(splitter.getAttribute('aria-valuenow')),
        renderedRatio: usable > 0 ? first / usable : Number.NaN,
      };
    };
    const documentElement = document.documentElement;
    const body = document.body;
    const app = document.querySelector('.app');
    const layout = document.querySelector('[data-workspace-split-root="true"]');
    const band = document.querySelector('.mappingPersistentWorkspaceBand');
    const toggle = document.querySelector('[data-timeline-pane-expand-toggle]');
    const drawer = document.querySelector('[data-workspace-selection-drawer]');
    return {
      persistentBandRect: measuredRect('.mappingPersistentWorkspaceBand'),
      persistentBandRects: {
        groups: measuredRect('[data-persistent-band-part="groups"]'),
        stage: measuredRect('[data-workspace-pane="lower-left"]'),
        selections: measuredRect('[data-workspace-selection-drawer][open]'),
        context: measuredRect('[data-workspace-pane="lower-right"]'),
      },
      workspaceSplitterRects: {
        upperLower: measuredRect('[data-workspace-splitter="upper-lower"]'),
        lowerLeftRight: measuredRect('[data-workspace-splitter="lower-left-right"]'),
      },
      workspaceSplitterState: {
        upperLower: splitterState('upper-lower', '[data-workspace-split-root="true"]', 'horizontal'),
        lowerLeftRight: splitterState('lower-left-right', '[data-workspace-pane="lower"]', 'vertical'),
      },
      workspaceSplitRatios: {
        top: Number(layout?.getAttribute('data-upper-lower-ratio') ?? NaN),
        lower: Number(layout?.getAttribute('data-lower-left-right-ratio') ?? NaN),
      },
      selectionDrawerOpen: drawer instanceof HTMLDetailsElement && drawer.open,
      workspaceStorageRaw: window.localStorage.getItem('syndocal.workspaceLayout.v1') ?? '',
      timelinePaneExpanded: band?.classList.contains('timelinePaneExpanded') ?? false,
      timelinePaneExpandToggleVisible: isVisible(toggle),
      timelinePaneExpandToggleNamed: Boolean(toggle?.getAttribute('aria-label')),
      viewport: [innerWidth, innerHeight],
      documentAndAppScrollZero:
        window.scrollX === 0 && window.scrollY === 0 &&
        documentElement.scrollWidth === documentElement.clientWidth &&
        documentElement.scrollHeight === documentElement.clientHeight &&
        body.scrollWidth === documentElement.clientWidth &&
        body.scrollHeight === documentElement.clientHeight &&
        (!app || (app.scrollWidth === app.clientWidth && app.scrollHeight === app.clientHeight)),
      scrollMetrics: {
        window: [window.scrollX, window.scrollY],
        document: [
          documentElement.scrollWidth,
          documentElement.clientWidth,
          documentElement.scrollHeight,
          documentElement.clientHeight,
        ],
        app: app ? [app.scrollWidth, app.clientWidth, app.scrollHeight, app.clientHeight] : null,
      },
    };
  })()`);
}

async function runTimelinePaneExpansionCheck(client, viewport) {
  await clickVisibleByText(client, '.workspaceTabs button', 'Control');
  await clickVisibleByText(client, '.controlModeTabs button', 'Timeline');
  await sleep(120);
  const before = await measureTimelinePaneExpansionState(client);
  const expandToggleClicked = await client.evaluate(`(() => {
    const toggle = document.querySelector('[data-timeline-pane-expand-toggle]');
    if (!toggle || toggle.disabled) return false;
    const rect = toggle.getBoundingClientRect();
    const style = getComputedStyle(toggle);
    if (rect.width <= 0 || rect.height <= 0 || style.display === 'none' || style.visibility === 'hidden') {
      return false;
    }
    toggle.click();
    return true;
  })()`);
  await sleep(120);
  const expanded = await measureTimelinePaneExpansionState(client);
  const resizedViewport = {
    width: viewport.width + 37,
    height: viewport.height + 23,
  };
  await client.send("Emulation.setDeviceMetricsOverride", {
    width: resizedViewport.width,
    height: resizedViewport.height,
    deviceScaleFactor: 1,
    mobile: false,
  });
  await sleep(160);
  const expandedResized = await measureTimelinePaneExpansionState(client);
  await pressKey(client, 'Escape', 'Escape');
  await sleep(160);
  const restoredResized = await measureTimelinePaneExpansionState(client);
  await client.send("Emulation.setDeviceMetricsOverride", {
    width: viewport.width,
    height: viewport.height,
    deviceScaleFactor: 1,
    mobile: false,
  });
  await sleep(160);
  const restored = await measureTimelinePaneExpansionState(client);
  const beforeContextWidth = before.persistentBandRects.context?.width ?? 0;
  const expandedContextWidth = expanded.persistentBandRects.context?.width ?? 0;
  const expandedBandWidth = expanded.persistentBandRect?.width ?? 0;
  const splitterAriaMatchesRendered = (state) => Boolean(
    state &&
    Number.isFinite(state.minimum) &&
    Number.isFinite(state.maximum) &&
    Number.isFinite(state.current) &&
    Number.isFinite(state.renderedRatio) &&
    state.minimum <= state.current &&
    state.current <= state.maximum &&
    Math.abs(state.current - Math.round(state.renderedRatio * 100)) <= 1
  );
  const timelinePaneExpansionConditions = [
    ['timelineExpandToggleVisible', () => Boolean(before.timelinePaneExpandToggleVisible)],
    ['timelineExpandToggleNamed', () => Boolean(before.timelinePaneExpandToggleNamed)],
    ['expandToggleClicked', () => Boolean(expandToggleClicked)],
    ['expandedStateApplied', () => Boolean(expanded.timelinePaneExpanded)],
    ['expandedContextWidthGrew', () => Boolean(expandedContextWidth > beforeContextWidth + 1)],
    ['expandedContextApproximatelyFullBandWidth', () => Boolean(
      expandedBandWidth > 0 && Math.abs(expandedBandWidth - expandedContextWidth) <= 4
    )],
    ['expandedLowerLeftPaneAndDrawerHidden', () => Boolean(
      expanded.persistentBandRects.stage === null && expanded.persistentBandRects.selections === null
    )],
    ['expandedGroupsStripHiddenWithLowerLeftPane', () => Boolean(expanded.persistentBandRects.groups === null)],
    ['expandedLowerSplitterHidden', () => Boolean(expanded.workspaceSplitterRects.lowerLeftRight === null)],
    ['expandedUpperSplitterBoundaryPreserved', () => persistentBandRectsExactlyEqual(
      before.workspaceSplitterRects.upperLower,
      expanded.workspaceSplitterRects.upperLower,
    )],
    ['expandedSplitRatiosRemainStable', () => Boolean(
      Math.abs(expanded.workspaceSplitRatios.top - before.workspaceSplitRatios.top) <= 0.001 &&
      Math.abs(expanded.workspaceSplitRatios.lower - before.workspaceSplitRatios.lower) <= 0.001
    )],
    ['expandedStateDoesNotPersistTransientGeometry', () => expanded.workspaceStorageRaw === before.workspaceStorageRaw],
    ['expandedDocumentAndAppScrollZero', () => Boolean(expanded.documentAndAppScrollZero)],
    ['expandedResizeKeepsFocusAndPreferredRatios', () => Boolean(
      expandedResized.timelinePaneExpanded &&
      Math.abs(expandedResized.workspaceSplitRatios.top - before.workspaceSplitRatios.top) <= 0.001 &&
      Math.abs(expandedResized.workspaceSplitRatios.lower - before.workspaceSplitRatios.lower) <= 0.001 &&
      expandedResized.workspaceStorageRaw === before.workspaceStorageRaw
    )],
    ['expandedResizeKeepsUpperSplitterAriaAndOuterScroll', () => Boolean(
      splitterAriaMatchesRendered(expandedResized.workspaceSplitterState.upperLower) &&
      expandedResized.workspaceSplitterState.lowerLeftRight === null &&
      expandedResized.documentAndAppScrollZero
    )],
    ['resizeEscapeRestoresBothSplitterAria', () => Boolean(
      !restoredResized.timelinePaneExpanded &&
      splitterAriaMatchesRendered(restoredResized.workspaceSplitterState.upperLower) &&
      splitterAriaMatchesRendered(restoredResized.workspaceSplitterState.lowerLeftRight)
    )],
    ['resizeEscapePreservesUpperBoundary', () => Boolean(
      persistentBandRectsExactlyEqual(
        expandedResized.workspaceSplitterRects.upperLower,
        restoredResized.workspaceSplitterRects.upperLower,
      ) &&
      persistentBandRectsExactlyEqual(
        expandedResized.persistentBandRect,
        restoredResized.persistentBandRect,
      )
    )],
    ['resizeEscapePreservesPreferredRatiosStorageAndOuterScroll', () => Boolean(
      Math.abs(restoredResized.workspaceSplitRatios.top - before.workspaceSplitRatios.top) <= 0.001 &&
      Math.abs(restoredResized.workspaceSplitRatios.lower - before.workspaceSplitRatios.lower) <= 0.001 &&
      restoredResized.workspaceStorageRaw === before.workspaceStorageRaw &&
      restoredResized.documentAndAppScrollZero
    )],
    ['escapeClearedExpandedState', () => Boolean(!restored.timelinePaneExpanded)],
    ['escapeRestoredGroupsRectWithinOnePixel', () => persistentBandRectsExactlyEqual(
      before.persistentBandRects.groups,
      restored.persistentBandRects.groups,
    )],
    ['escapeRestoredLowerLeftRectWithinOnePixel', () => persistentBandRectsExactlyEqual(
      before.persistentBandRects.stage,
      restored.persistentBandRects.stage,
    )],
    ['escapeRestoredLowerRightRectWithinOnePixel', () => persistentBandRectsExactlyEqual(
      before.persistentBandRects.context,
      restored.persistentBandRects.context,
    )],
    ['escapeRestoredLowerSplitterWithinOnePixel', () => persistentBandRectsExactlyEqual(
      before.workspaceSplitterRects.lowerLeftRight,
      restored.workspaceSplitterRects.lowerLeftRight,
    )],
    ['escapeRestoredDrawerState', () => restored.selectionDrawerOpen === before.selectionDrawerOpen],
    ['escapeRestoredSplitRatios', () => Boolean(
      Math.abs(restored.workspaceSplitRatios.top - before.workspaceSplitRatios.top) <= 0.001 &&
      Math.abs(restored.workspaceSplitRatios.lower - before.workspaceSplitRatios.lower) <= 0.001
    )],
    ['escapeKeptWorkspaceStorageExact', () => restored.workspaceStorageRaw === before.workspaceStorageRaw],
    ['restoredDocumentAndAppScrollZero', () => Boolean(restored.documentAndAppScrollZero)],
    ['originalViewportRestoresBothSplitterAria', () => Boolean(
      restored.viewport[0] === viewport.width &&
      restored.viewport[1] === viewport.height &&
      splitterAriaMatchesRendered(restored.workspaceSplitterState.upperLower) &&
      splitterAriaMatchesRendered(restored.workspaceSplitterState.lowerLeftRight)
    )],
  ];
  const checks = Object.fromEntries(timelinePaneExpansionConditions.map(([name, check]) => {
    try {
      return [name, check()];
    } catch {
      return [name, false];
    }
  }));
  const failedChecks = Object.entries(checks).filter(([, passed]) => !passed).map(([name]) => name);
  return {
    label: `timeline-pane-expansion-${viewport.width}x${viewport.height}`,
    passed: failedChecks.length === 0,
    checks,
    failedChecks,
    before,
    expanded,
    expandedResized,
    restoredResized,
    restored,
  };
}

async function measureLayeredTimelineDeskState(client) {
  return await client.evaluate(`(async () => {
    await new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame)));
    const isVisible = (element) => {
      if (!element) return false;
      const rect = element.getBoundingClientRect();
      const style = getComputedStyle(element);
      return rect.width > 0 && rect.height > 0 && style.display !== 'none' && style.visibility !== 'hidden';
    };
    const visibleElements = (selector, root = document) => [...root.querySelectorAll(selector)].filter(isVisible);
    const gutterSelector = '[data-timeline-layer-id][data-timeline-layer-gutter]';
    const sectionHeaders = visibleElements('[data-timeline-section-kind]');
    const gutters = visibleElements(gutterSelector);
    const expectedLayerIdsByKind = {
      Audio: new Set(['10', '11']),
      Lighting: new Set(['12', '13']),
      Video: new Set(['14', '15']),
    };
    const sectionLayerCounts = sectionHeaders.map((header) => {
      const explicitCountAttribute = header.getAttribute('data-timeline-section-count');
      const explicitCount = explicitCountAttribute === null ? Number.NaN : Number(explicitCountAttribute);
      if (Number.isFinite(explicitCount) && explicitCount >= 0) return explicitCount;
      const visibleCount = Number((header.textContent || '').match(/\\d+/)?.[0]);
      if (Number.isFinite(visibleCount) && visibleCount >= 0) return visibleCount;
      const sectionRoot = header.matches(gutterSelector) ? null : header;
      const nestedGutters = sectionRoot ? visibleElements(gutterSelector, sectionRoot) : [];
      if (nestedGutters.length > 0) return nestedGutters.length;
      const parentGutters = header.parentElement ? visibleElements(gutterSelector, header.parentElement) : [];
      if (parentGutters.length > 0) return parentGutters.length;
      const kind = header.getAttribute('data-timeline-section-kind') || '';
      const expectedIds = expectedLayerIdsByKind[kind];
      return expectedIds ? gutters.filter((gutter) => expectedIds.has(gutter.getAttribute('data-timeline-layer-id') || '')).length : 0;
    });
    const targetGutter = document.querySelector('[data-timeline-layer-id="12"][data-timeline-layer-gutter]');
    const targetToggle = targetGutter?.querySelector('[data-timeline-layer-mute-toggle]') ?? null;
    const targetMarkers = visibleElements('.timelineMarker[data-timeline-layer-id="12"][data-timeline-layer-muted]');
    const audioClips = visibleElements('.timelineAudioClip[data-timeline-layer-kind="Audio"]');
    const audioWaveforms = [...document.querySelectorAll('.timelineAudioClip [data-timeline-audio-waveform]')];
    const audioFadeRamps = [...document.querySelectorAll('.timelineAudioClip [data-timeline-audio-fade-ramp]')];
    const audioClipAddButtons = visibleElements('[data-timeline-section-kind="Audio"] [data-timeline-add-audio-clip]');
    const lightingClipAddButtons = visibleElements('[data-timeline-section-kind="Lighting"] [data-timeline-add-audio-clip]');
    const superSceneBlocks = visibleElements('.timelineMarker.sceneBlock[data-super-scene="true"]');
    const superSceneSourceLinks = visibleElements('[data-super-scene-source-link]');
    const frame = document.querySelector('.timelineOverviewFrame');
    const scrollports = visibleElements('[data-timeline-layer-scrollport]');
    const documentElement = document.documentElement;
    const body = document.body;
    const app = document.querySelector('.app');
    const overflowTargets = visibleElements(
      '.timelinePanel, .timelineShowSurface, .timelineOverviewFrame, [data-timeline-layer-scrollport], .sceneBlockWorkspace',
    );
    const unsafeOverflow = overflowTargets.flatMap((element) => {
      const style = getComputedStyle(element);
      const horizontalTolerance = element.matches('.sceneBlockWorkspace') ? 4 : 1;
      const unsafeX = element.scrollWidth > element.clientWidth + horizontalTolerance && !/(auto|scroll)/.test(style.overflowX);
      const unsafeY = element.scrollHeight > element.clientHeight + 1 && !/(auto|scroll)/.test(style.overflowY);
      return unsafeX || unsafeY
        ? [{
            selector: element.getAttribute('data-timeline-layer-scrollport') !== null
              ? '[data-timeline-layer-scrollport]'
              : String(element.className || element.tagName),
            client: [element.clientWidth, element.clientHeight],
            scroll: [element.scrollWidth, element.scrollHeight],
            overflow: [style.overflowX, style.overflowY],
            unsafeX,
            unsafeY,
          }]
        : [];
    });
    return {
      sectionKinds: sectionHeaders.map((header) => header.getAttribute('data-timeline-section-kind') || ''),
      sectionLayerCounts,
      gutterLayerIds: gutters.map((gutter) => gutter.getAttribute('data-timeline-layer-id') || ''),
      targetToggleVisible: isVisible(targetToggle),
      targetTogglePressed: targetToggle?.getAttribute('aria-pressed') ?? '',
      targetLayerMuted: targetGutter?.getAttribute('data-timeline-layer-muted') === 'true' ||
        targetToggle?.getAttribute('aria-pressed') === 'false',
      targetMarkerMutedStates: targetMarkers.map((marker) => marker.getAttribute('data-timeline-layer-muted') === 'true'),
      audioClipCount: audioClips.length,
      audioClipLayerKinds: audioClips.map((clip) => clip.getAttribute('data-timeline-layer-kind') || ''),
      audioWaveformCount: audioWaveforms.length,
      audioWaveformDeclaredNodeCounts: audioWaveforms.map((waveform) =>
        Number(waveform.getAttribute('data-timeline-audio-waveform-node-count') || 0)),
      audioFadeRampCount: audioFadeRamps.length,
      audioClipAddButtonCount: audioClipAddButtons.length,
      lightingClipAddButtonCount: lightingClipAddButtons.length,
      superSceneBlockCount: superSceneBlocks.length,
      superSceneSourceLinkCount: superSceneSourceLinks.length,
      overviewNodeCount: document.querySelectorAll('.timelineOverview *').length,
      scrollportCount: scrollports.length,
      scrollportOverflowSafe: scrollports.every((scrollport) => {
        const style = getComputedStyle(scrollport);
        return scrollport.scrollHeight <= scrollport.clientHeight + 1 || /(auto|scroll)/.test(style.overflowY);
      }),
      frameHeight: Math.round(frame?.getBoundingClientRect().height ?? 0),
      documentAndAppScrollZero:
        window.scrollX === 0 && window.scrollY === 0 &&
        documentElement.scrollWidth === documentElement.clientWidth &&
        documentElement.scrollHeight === documentElement.clientHeight &&
        body.scrollWidth === documentElement.clientWidth &&
        body.scrollHeight === documentElement.clientHeight &&
        (!app || (app.scrollWidth === app.clientWidth && app.scrollHeight === app.clientHeight)),
      scrollMetrics: {
        window: [window.scrollX, window.scrollY],
        document: [
          documentElement.scrollWidth,
          documentElement.clientWidth,
          documentElement.scrollHeight,
          documentElement.clientHeight,
        ],
        app: app ? [app.scrollWidth, app.clientWidth, app.scrollHeight, app.clientHeight] : null,
      },
      unsafeOverflowCount: unsafeOverflow.length,
      unsafeOverflow,
    };
  })()`);
}

async function runLayeredTimelineDeskCheck(client, viewport) {
  await client.send('Page.navigate', { url: fixtureUrl('timeline-layered') });
  await waitForApp(client);
  await clickVisibleByText(client, '.workspaceTabs button', 'Control');
  await clickVisibleByText(client, '.controlModeTabs button', 'Timeline');
  await clickVisibleByText(client, '.timelineDeskTabs button', 'Show');
  await sleep(120);
  const before = await measureLayeredTimelineDeskState(client);
  const muteToggleClicked = await client.evaluate(`(() => {
    const gutter = document.querySelector('[data-timeline-layer-id="12"][data-timeline-layer-gutter]');
    const toggle = gutter?.querySelector('[data-timeline-layer-mute-toggle]');
    if (!(toggle instanceof HTMLButtonElement) || toggle.disabled) return false;
    const rect = toggle.getBoundingClientRect();
    const style = getComputedStyle(toggle);
    if (rect.width <= 0 || rect.height <= 0 || style.display === 'none' || style.visibility === 'hidden') return false;
    toggle.click();
    return true;
  })()`);
  await sleep(160);
  const muted = await measureLayeredTimelineDeskState(client);
  const dragTimelineElement = async (selector, deltaX, deltaY = 0, duringDrag = null) => {
    const point = await client.evaluate(`(() => {
      const element = document.querySelector(${JSON.stringify(selector)});
      if (!element) return null;
      const rect = element.getBoundingClientRect();
      const x = rect.left + rect.width / 2;
      const y = rect.top + rect.height / 2;
      const hit = document.elementFromPoint(x, y);
      return {
        x,
        y,
        width: rect.width,
        height: rect.height,
        hitTag: hit?.tagName ?? '',
        hitClass: hit?.getAttribute('class') ?? '',
        hitResize: hit?.getAttribute('data-timeline-scene-block-resize') ?? '',
        hitFade: hit?.getAttribute('data-timeline-scene-block-fade') ?? '',
        hitEventId: hit?.closest('.timelineMarker')?.getAttribute('data-timeline-event-id') ?? '',
      };
    })()`);
    if (!point) return { dragged: false, during: null };
    await client.send('Input.dispatchMouseEvent', {
      type: 'mousePressed',
      x: point.x,
      y: point.y,
      button: 'left',
      buttons: 1,
      clickCount: 1,
    });
    await client.send('Input.dispatchMouseEvent', {
      type: 'mouseMoved',
      x: point.x + deltaX,
      y: point.y + deltaY,
      button: 'left',
      buttons: 1,
    });
    await sleep(32);
    const during = duringDrag ? await duringDrag() : null;
    await client.send('Input.dispatchMouseEvent', {
      type: 'mouseReleased',
      x: point.x + deltaX,
      y: point.y + deltaY,
      button: 'left',
      buttons: 0,
      clickCount: 1,
    });
    await sleep(160);
    return { dragged: true, during, point };
  };
  const selectFirstSceneBlock = async () => client.evaluate(`(() => {
    const marker = [...document.querySelectorAll('.timelineMarker.sceneBlock')]
      .find((candidate) => candidate.getBoundingClientRect().width >= 48);
    if (!marker) return null;
    marker.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
    const rect = marker.getBoundingClientRect();
    return {
      id: marker.getAttribute('data-timeline-event-id'),
      startMs: Number(marker.getAttribute('data-timeline-start-ms')),
      previewStartMs: Number(marker.getAttribute('data-timeline-preview-start-ms')),
      previewEndMs: Number(marker.getAttribute('data-timeline-preview-end-ms')),
      rate: marker.getAttribute('data-timeline-rate'),
      width: rect.width,
    };
  })()`);
  const readSelectedSceneBlock = async () => client.evaluate(`(() => {
    const marker = document.querySelector('.timelineMarker.sceneBlock.selected');
    if (!marker) return null;
    return {
      id: marker.getAttribute('data-timeline-event-id'),
      startMs: Number(marker.getAttribute('data-timeline-start-ms')),
      previewStartMs: Number(marker.getAttribute('data-timeline-preview-start-ms')),
      previewEndMs: Number(marker.getAttribute('data-timeline-preview-end-ms')),
      rate: marker.getAttribute('data-timeline-rate'),
      previewRate: marker.getAttribute('data-timeline-preview-rate'),
      fadeInMs: Number(marker.getAttribute('data-timeline-fade-in-ms')),
      fadeOutMs: Number(marker.getAttribute('data-timeline-fade-out-ms')),
      fadeInRamp: Boolean(marker.querySelector('[data-timeline-block-fade-ramp="in"]')),
      fadeOutRamp: Boolean(marker.querySelector('[data-timeline-block-fade-ramp="out"]')),
      liveStamp: marker.querySelector('[data-timeline-live-stamp]')?.textContent?.trim() ?? '',
    };
  })()`);

  const directControls = await client.evaluate(`(() => ({
    stretchToggle: Boolean(document.querySelector('[data-timeline-stretch-mode-toggle]')),
    rateButton: Boolean(document.querySelector('[data-timeline-stretch-mode="RATE"]')),
    windowButton: Boolean(document.querySelector('[data-timeline-stretch-mode="WINDOW"]')),
    magnetToggle: Boolean(document.querySelector('[data-timeline-magnet-toggle]')),
    armButton: Boolean(document.querySelector('.timelineDirectToolbar [data-timeline-arm-cue]')),
  }))()`);
  await client.evaluate(`document.querySelector('[data-timeline-stretch-mode="RATE"]')?.click()`);
  const rateBefore = await selectFirstSceneBlock();
  await sleep(32);
  const rateDrag = await dragTimelineElement(
    '.timelineMarker.sceneBlock.selected [data-timeline-scene-block-resize="end"]',
    -48,
    0,
    readSelectedSceneBlock,
  );
  const rateAfter = await readSelectedSceneBlock();

  await client.send('Page.navigate', { url: fixtureUrl('timeline-layered') });
  await waitForApp(client);
  await clickVisibleByText(client, '.workspaceTabs button', 'Control');
  await clickVisibleByText(client, '.controlModeTabs button', 'Timeline');
  await clickVisibleByText(client, '.timelineDeskTabs button', 'Show');
  await sleep(120);
  await client.evaluate(`document.querySelector('[data-timeline-stretch-mode="WINDOW"]')?.click()`);
  const windowBefore = await selectFirstSceneBlock();
  await sleep(32);
  const windowDrag = await dragTimelineElement(
    '.timelineMarker.sceneBlock.selected [data-timeline-scene-block-resize="end"]',
    Math.max(42, (windowBefore?.width ?? 140) * 0.3),
  );
  const windowAfter = await readSelectedSceneBlock();

  const fadeInDrag = await dragTimelineElement(
    '.timelineMarker.sceneBlock.selected [data-timeline-scene-block-fade="in"]',
    36,
    0,
    readSelectedSceneBlock,
  );
  const fadeInAfter = await readSelectedSceneBlock();
  const fadeOutDrag = await dragTimelineElement(
    '.timelineMarker.sceneBlock.selected [data-timeline-scene-block-fade="out"]',
    -36,
    0,
    readSelectedSceneBlock,
  );
  const fadeOutAfter = await readSelectedSceneBlock();

  const escapeBefore = await readSelectedSceneBlock();
  const escapePoint = await client.evaluate(`(() => {
    const marker = document.querySelector('.timelineMarker.sceneBlock.selected');
    if (!marker) return null;
    const rect = marker.getBoundingClientRect();
    return { x: rect.left + rect.width / 2, y: rect.top + 4 };
  })()`);
  let escapeDuring = null;
  if (escapePoint) {
    await client.send('Input.dispatchMouseEvent', {
      type: 'mousePressed', x: escapePoint.x, y: escapePoint.y,
      button: 'left', buttons: 1, clickCount: 1,
    });
    await client.send('Input.dispatchMouseEvent', {
      type: 'mouseMoved', x: escapePoint.x + 52, y: escapePoint.y,
      button: 'left', buttons: 1,
    });
    await sleep(32);
    escapeDuring = await readSelectedSceneBlock();
    await client.send('Input.dispatchKeyEvent', { type: 'keyDown', key: 'Escape', code: 'Escape' });
    await client.send('Input.dispatchKeyEvent', { type: 'keyUp', key: 'Escape', code: 'Escape' });
    await client.send('Input.dispatchMouseEvent', {
      type: 'mouseReleased', x: escapePoint.x + 52, y: escapePoint.y,
      button: 'left', buttons: 0, clickCount: 1,
    });
    await sleep(64);
  }
  const escapeAfter = await readSelectedSceneBlock();

  const wheelBefore = await client.evaluate(`(() => {
    const canvas = document.querySelector('.timelineOverview');
    if (!canvas) return null;
    const rect = canvas.getBoundingClientRect();
    const ratio = 0.63;
    const startMs = Number(canvas.getAttribute('data-visible-start-ms'));
    const endMs = Number(canvas.getAttribute('data-visible-end-ms'));
    return {
      x: rect.left + rect.width * ratio,
      y: rect.top + rect.height / 2,
      width: rect.width,
      ratio,
      startMs,
      endMs,
      cursorTimeMs: startMs + (endMs - startMs) * ratio,
      hitClass: document.elementFromPoint(
        rect.left + rect.width * ratio,
        rect.top + rect.height / 2,
      )?.getAttribute('class') ?? '',
    };
  })()`);
  if (wheelBefore) {
    await client.send('Input.dispatchMouseEvent', {
      type: 'mouseWheel',
      x: wheelBefore.x,
      y: wheelBefore.y,
      deltaX: 0,
      deltaY: -120,
    });
    await sleep(96);
  }
  const wheelAfter = await client.evaluate(`(() => {
    const canvas = document.querySelector('.timelineOverview');
    const documentElement = document.documentElement;
    const app = document.querySelector('.app');
    if (!canvas) return null;
    const startMs = Number(canvas.getAttribute('data-visible-start-ms'));
    const endMs = Number(canvas.getAttribute('data-visible-end-ms'));
    return {
      startMs,
      endMs,
      documentAndAppScrollZero:
        window.scrollX === 0 && window.scrollY === 0 &&
        documentElement.scrollWidth === documentElement.clientWidth &&
        documentElement.scrollHeight === documentElement.clientHeight &&
        (!app || (app.scrollWidth === app.clientWidth && app.scrollHeight === app.clientHeight)),
    };
  })()`);
  const wheelCursorDriftPx = wheelBefore && wheelAfter
    ? Math.abs(
        (wheelAfter.startMs + (wheelAfter.endMs - wheelAfter.startMs) * wheelBefore.ratio) -
        wheelBefore.cursorTimeMs
      ) / Math.max(1, wheelAfter.endMs - wheelAfter.startMs) * wheelBefore.width
    : Number.POSITIVE_INFINITY;

  const placementBefore = await client.evaluate(`document.querySelectorAll('.timelineMarker.sceneBlock').length`);
  await client.evaluate(`document.querySelector('.timelineDirectToolbar [data-timeline-arm-cue]')?.click()`);
  const placementDispatched = await client.evaluate(`(() => {
    const row = document.querySelector('[data-timeline-layer-id="13"].timelineLayerRowBackground');
    if (!row) return false;
    const scrollport = row.closest('.timelineLayerScrollport');
    if (scrollport instanceof HTMLElement) {
      const rowTop = Number(row.getAttribute('y'));
      const rowHeight = Number(row.getAttribute('height'));
      if (Number.isFinite(rowTop) && Number.isFinite(rowHeight)) {
        scrollport.scrollTop = Math.max(0, rowTop + rowHeight / 2 - scrollport.clientHeight / 2);
      }
    }
    const rect = row.getBoundingClientRect();
    const hitLayer = document.elementFromPoint(
      rect.left + rect.width * 0.82,
      rect.top + rect.height / 2,
    )?.closest('[data-timeline-layer-id]');
    if (hitLayer?.getAttribute('data-timeline-layer-id') !== '13') return false;
    row.dispatchEvent(new MouseEvent('dblclick', {
      bubbles: true,
      cancelable: true,
      clientX: rect.left + rect.width * 0.82,
      clientY: rect.top + rect.height / 2,
    }));
    return true;
  })()`);
  await sleep(180);
  const placementAfter = await client.evaluate(`document.querySelectorAll('.timelineMarker.sceneBlock').length`);
  const superSceneOpened = await client.evaluate(`(() => {
    const marker = document.querySelector('.timelineMarker.sceneBlock[data-super-scene="true"]');
    if (!marker) return false;
    const rect = marker.getBoundingClientRect();
    marker.dispatchEvent(new MouseEvent('dblclick', {
      bubbles: true,
      cancelable: true,
      clientX: rect.left + rect.width / 2,
      clientY: rect.top + rect.height / 2,
    }));
    return true;
  })()`);
  await sleep(180);
  const childTimeline = await client.evaluate(`(() => {
    const visible = (selector) => [...document.querySelectorAll(selector)].filter((element) => {
      const rect = element.getBoundingClientRect();
      const style = getComputedStyle(element);
      return rect.width > 0 && rect.height > 0 && style.display !== 'none' && style.visibility !== 'hidden';
    });
    const frame = document.querySelector('.timelineOverviewFrame');
    const documentElement = document.documentElement;
    const body = document.body;
    const app = document.querySelector('.app');
    return {
      breadcrumbVisible: visible('[data-timeline-breadcrumb]').length === 1,
      breadcrumbLabel: document.querySelector('[data-child-timeline-label]')?.textContent?.trim() ?? '',
      layerIds: visible('[data-timeline-layer-id][data-timeline-layer-gutter]')
        .map((element) => element.getAttribute('data-timeline-layer-id') || ''),
      blockCount: visible('.timelineMarker.sceneBlock').length,
      audioClipCount: visible('.timelineAudioClip[data-timeline-layer-kind="Audio"]').length,
      overviewNodeCount: document.querySelectorAll('.timelineOverview *').length,
      frameHeight: Math.round(frame?.getBoundingClientRect().height ?? 0),
      documentAndAppScrollZero:
        window.scrollX === 0 && window.scrollY === 0 &&
        documentElement.scrollWidth === documentElement.clientWidth &&
        documentElement.scrollHeight === documentElement.clientHeight &&
        body.scrollWidth === documentElement.clientWidth &&
        body.scrollHeight === documentElement.clientHeight &&
        (!app || (app.scrollWidth === app.clientWidth && app.scrollHeight === app.clientHeight)),
    };
  })()`);
  const superSceneExitClicked = await client.evaluate(`(() => {
    const button = document.querySelector('[data-timeline-breadcrumb] button');
    if (!(button instanceof HTMLButtonElement)) return false;
    button.click();
    return true;
  })()`);
  await sleep(180);
  const superSceneRestored = await measureLayeredTimelineDeskState(client);
  const direct = {
    controls: directControls,
    rate: { before: rateBefore, drag: rateDrag, after: rateAfter },
    window: { before: windowBefore, drag: windowDrag, after: windowAfter },
    fadeIn: { drag: fadeInDrag, after: fadeInAfter },
    fadeOut: { drag: fadeOutDrag, after: fadeOutAfter },
    escape: { before: escapeBefore, during: escapeDuring, after: escapeAfter },
    wheel: { before: wheelBefore, after: wheelAfter, cursorDriftPx: wheelCursorDriftPx },
    placement: { before: placementBefore, after: placementAfter, dispatched: placementDispatched },
    superScene: { opened: superSceneOpened, child: childTimeline, exitClicked: superSceneExitClicked, restored: superSceneRestored },
  };
  const expectedFrameHeight = before.frameHeight;
  const minimumFrameHeight = viewport.height <= 800 ? 120 : 148;
  const conditions = [
    ['layeredDeskFixtureHasSixGutters', () =>
      before.gutterLayerIds.length === 6 &&
      new Set(before.gutterLayerIds).size === 6 &&
      ['10', '11', '12', '13', '14', '15'].every((layerId) => before.gutterLayerIds.includes(layerId))],
    ['layeredDeskSectionHeadersPresent', () => before.sectionKinds.length === 3],
    ['layeredDeskSectionOrderAudioLightingVideo', () => JSON.stringify(before.sectionKinds) === JSON.stringify(['Audio', 'Lighting', 'Video'])],
    ['layeredDeskSectionCountsTwoEach', () => JSON.stringify(before.sectionLayerCounts) === JSON.stringify([2, 2, 2])],
    ['layeredDeskTargetMuteToggleVisible', () => Boolean(before.targetToggleVisible)],
    ['layeredDeskTargetLaneHasMarkers', () => before.targetMarkerMutedStates.length > 0],
    ['layeredDeskMuteToggleClicked', () => Boolean(muteToggleClicked)],
    ['layeredDeskLayerMuteStateChanged', () => !before.targetLayerMuted && muted.targetLayerMuted],
    ['layeredDeskMarkersReflectMutedState', () =>
      before.targetMarkerMutedStates.length > 0 &&
      before.targetMarkerMutedStates.every((state) => !state) &&
      muted.targetMarkerMutedStates.length === before.targetMarkerMutedStates.length &&
      muted.targetMarkerMutedStates.every(Boolean)],
    ['timelineAudioFixtureHasTwoVisibleClipsOnAudioLanes', () =>
      before.audioClipCount === 2 && before.audioClipLayerKinds.every((kind) => kind === 'Audio')],
    ['timelineAudioClipWaveformUsesOneNodePerClip', () =>
      before.audioWaveformCount === before.audioClipCount &&
      before.audioWaveformDeclaredNodeCounts.every((count) => count === 1)],
    ['timelineAudioClipConfiguredFadeDrawsRamp', () => before.audioFadeRampCount >= 1],
    ['timelineAudioAddAffordancePresentOnlyOnAudioSection', () =>
      before.audioClipAddButtonCount === 1 && before.lightingClipAddButtonCount === 0],
    ['timelineAudioClipsKeepOverviewNodeBudget', () => before.overviewNodeCount < 3_500],
    ['superSceneSourceBlockAndLinkVisible', () => before.superSceneBlockCount === 1 && before.superSceneSourceLinkCount === 1],
    ['superSceneBreadcrumbOpensChildTimeline', () =>
      direct.superScene.opened &&
      direct.superScene.child?.breadcrumbVisible === true &&
      direct.superScene.child?.breadcrumbLabel === 'Shin'],
    ['superSceneChildTimelineShowsThreeSourceLinkedLanes', () =>
      JSON.stringify(direct.superScene.child?.layerIds) === JSON.stringify(['50', '51', '52'])],
    ['superSceneChildTimelineShowsTwoLightingBlocksAndOneAudioClip', () =>
      direct.superScene.child?.blockCount === 2 && direct.superScene.child?.audioClipCount === 1],
    ['superSceneChildTimelineKeepsOverviewNodeBudget', () => direct.superScene.child?.overviewNodeCount < 3_500],
    ['superSceneBreadcrumbRoundTripRestoresSourceBlock', () =>
      direct.superScene.exitClicked &&
      direct.superScene.restored?.superSceneBlockCount === 1 &&
      direct.superScene.restored?.superSceneSourceLinkCount === 1],
    ['superSceneBreadcrumbRoundTripKeepsReachableFrame', () =>
      (direct.superScene.child?.frameHeight ?? 0) >= minimumFrameHeight - 2 &&
      (direct.superScene.restored?.frameHeight ?? 0) >= minimumFrameHeight - 2 &&
      direct.superScene.restored?.scrollportOverflowSafe === true],
    ['superSceneBreadcrumbRoundTripKeepsDocumentAndAppScrollZero', () =>
      direct.superScene.child?.documentAndAppScrollZero === true &&
      direct.superScene.restored?.documentAndAppScrollZero === true],
    ['timelineAudioFixtureKeepsDocumentAndAppScrollZero', () => before.documentAndAppScrollZero],
    ['layeredDeskInternalScrollportPresent', () => before.scrollportCount === 1 && before.scrollportOverflowSafe],
    ['layeredDeskDocumentAndAppScrollZero', () => before.documentAndAppScrollZero && muted.documentAndAppScrollZero],
    ['layeredDeskUnsafeOverflowZero', () => before.unsafeOverflowCount === 0 && muted.unsafeOverflowCount === 0],
    ['layeredDeskFrameRespectsMinimum', () =>
      before.frameHeight >= minimumFrameHeight &&
      Math.abs(muted.frameHeight - expectedFrameHeight) <= 1],
    ['timelineStretchModeTogglePresent', () =>
      direct.controls.stretchToggle && direct.controls.rateButton && direct.controls.windowButton],
    ['timelineMagnetTogglePresent', () => direct.controls.magnetToggle],
    ['timelineRateStretchChangesLiveRateBadge', () =>
      direct.rate.drag.dragged &&
      Number.isFinite(Number(direct.rate.drag.during?.previewRate)) &&
      Number(direct.rate.drag.during?.previewRate) > 0 &&
      /\[[0-9.]+x\]/.test(direct.rate.drag.during?.liveStamp ?? '') &&
      direct.rate.after?.rate !== direct.rate.before?.rate],
    ['timelineWindowStretchChangesDurationOnly', () =>
      direct.window.drag.dragged &&
      direct.window.after?.previewEndMs !== direct.window.before?.previewEndMs &&
      direct.window.after?.startMs === direct.window.before?.startMs &&
      direct.window.after?.rate === direct.window.before?.rate],
    ['timelineFadeInHandleSetsValueAndDrawsRamp', () =>
      direct.fadeIn.drag.dragged &&
      (direct.fadeIn.drag.during?.fadeInMs ?? 0) > 0 &&
      (direct.fadeIn.after?.fadeInMs ?? 0) > 0 &&
      direct.fadeIn.after?.fadeInRamp],
    ['timelineFadeOutHandleSetsValueAndDrawsRamp', () =>
      direct.fadeOut.drag.dragged &&
      (direct.fadeOut.drag.during?.fadeOutMs ?? 0) > 0 &&
      (direct.fadeOut.after?.fadeOutMs ?? 0) > 0 &&
      direct.fadeOut.after?.fadeOutRamp],
    ['timelineEscapeCancelRestoresGeometry', () =>
      direct.escape.before && direct.escape.during && direct.escape.after &&
      direct.escape.during.previewStartMs !== direct.escape.before.previewStartMs &&
      direct.escape.after.previewStartMs === direct.escape.before.previewStartMs &&
      direct.escape.after.previewEndMs === direct.escape.before.previewEndMs],
    ['timelineWheelZoomKeepsCursorTimeWithinTwoPixels', () =>
      direct.wheel.before && direct.wheel.after &&
      direct.wheel.after.endMs - direct.wheel.after.startMs < direct.wheel.before.endMs - direct.wheel.before.startMs &&
      direct.wheel.cursorDriftPx <= 2],
    ['timelineWheelZoomKeepsDocumentAndAppScrollZero', () =>
      direct.wheel.after?.documentAndAppScrollZero === true],
    ['timelineArmedCueDoubleClickPlacesNaturalBlock', () =>
      direct.controls.armButton && direct.placement.dispatched && direct.placement.after === direct.placement.before + 1],
  ];
  const checks = Object.fromEntries(conditions.map(([name, check]) => {
    try {
      return [name, check()];
    } catch {
      return [name, false];
    }
  }));
  const failedChecks = Object.entries(checks).filter(([, passed]) => !passed).map(([name]) => name);
  const result = {
    label: `layered-timeline-desk-${viewport.width}x${viewport.height}`,
    passed: failedChecks.length === 0,
    checks,
    failedChecks,
    before,
    muted,
    direct,
    muteToggleClicked,
    expectedFrameHeight,
  };
  await client.send('Page.navigate', { url: appUrl });
  await waitForApp(client);
  return result;
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
    const measuredRect = (selector) => {
      const element = document.querySelector(selector);
      if (!element) return null;
      const rect = element.getBoundingClientRect();
      const style = window.getComputedStyle(element);
      if (rect.width <= 0 || rect.height <= 0 || style.display === 'none' || style.visibility === 'hidden') return null;
      const precision = (value) => Math.round(value * 100) / 100;
      return {
        x: precision(rect.x),
        y: precision(rect.y),
        width: precision(rect.width),
        height: precision(rect.height),
      };
    };
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
    const controlReachableWhenScrolled = (container, control) => {
      if (!container || !control) return false;
      const ancestors = [];
      for (let ancestor = control.parentElement; ancestor; ancestor = ancestor.parentElement) {
        ancestors.push(ancestor);
      }
      const scrollState = ancestors.map((ancestor) => ({
        element: ancestor,
        left: ancestor.scrollLeft,
        top: ancestor.scrollTop,
      }));
      const scrollableAncestors = ancestors.filter((ancestor) => {
        const style = getComputedStyle(ancestor);
        return (
          (/(auto|scroll)/.test(style.overflowY) && ancestor.scrollHeight > ancestor.clientHeight + 1) ||
          (/(auto|scroll)/.test(style.overflowX) && ancestor.scrollWidth > ancestor.clientWidth + 1)
        );
      });
      for (let pass = 0; pass < 3; pass += 1) {
        for (const ancestor of scrollableAncestors) {
          const ancestorRect = ancestor.getBoundingClientRect();
          const controlRect = control.getBoundingClientRect();
          if (controlRect.bottom > ancestorRect.bottom - 1) {
            ancestor.scrollTop += controlRect.bottom - ancestorRect.bottom + 1;
          } else if (controlRect.top < ancestorRect.top + 1) {
            ancestor.scrollTop -= ancestorRect.top - controlRect.top + 1;
          }
          if (controlRect.right > ancestorRect.right - 1) {
            ancestor.scrollLeft += controlRect.right - ancestorRect.right + 1;
          } else if (controlRect.left < ancestorRect.left + 1) {
            ancestor.scrollLeft -= ancestorRect.left - controlRect.left + 1;
          }
        }
      }
      const clippedAncestors = [container, ...ancestors].filter((element, index, values) => {
        if (values.indexOf(element) !== index) return false;
        const style = getComputedStyle(element);
        return /(auto|scroll|hidden|clip)/.test(style.overflowX) || /(auto|scroll|hidden|clip)/.test(style.overflowY);
      });
      const visibleBounds = clippedAncestors.reduce((bounds, element) => {
        const rect = element.getBoundingClientRect();
        return {
          left: Math.max(bounds.left, rect.left),
          right: Math.min(bounds.right, rect.right),
          top: Math.max(bounds.top, rect.top),
          bottom: Math.min(bounds.bottom, rect.bottom),
        };
      }, { left: 0, right: innerWidth, top: 0, bottom: innerHeight });
      const controlRect = control.getBoundingClientRect();
      const reachable = controlRect.width > 0 && controlRect.height > 0 &&
        controlRect.left >= visibleBounds.left - 1 &&
        controlRect.right <= visibleBounds.right + 1 &&
        controlRect.top >= visibleBounds.top - 1 &&
        controlRect.bottom <= visibleBounds.bottom + 1;
      scrollState.forEach(({ element, left, top }) => {
        element.scrollLeft = left;
        element.scrollTop = top;
      });
      return reachable;
    };
    const lastControlReachableWhenScrolled = (containerSelector, controlSelector) => {
      const container = document.querySelector(containerSelector);
      const controls = container ? [...container.querySelectorAll(controlSelector)] : [];
      return controlReachableWhenScrolled(container, controls.at(-1));
    };
    const lastVisibleControlReachableWhenScrolled = (containerSelector, controlSelector) => {
      const container = document.querySelector(containerSelector);
      const controls = container
        ? [...container.querySelectorAll(controlSelector)].filter((element) => {
          const rect = element.getBoundingClientRect();
          const style = getComputedStyle(element);
          return rect.width > 0 && rect.height > 0 && style.display !== 'none' && style.visibility !== 'hidden';
        })
        : [];
      return controlReachableWhenScrolled(container, controls.at(-1));
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
    const cueContextDrawer = document.querySelector('[data-timeline-context-drawer-panel="cue"]');
    const cueContextDrawerBody = cueContextDrawer?.querySelector('.timelineContextDrawerBody') ?? null;
    const cueHostRect = cueHost?.getBoundingClientRect() ?? null;
    const cueContextDrawerRect = cueContextDrawer?.getBoundingClientRect() ?? null;
    const cueContextDrawerBodyRect = cueContextDrawerBody?.getBoundingClientRect() ?? null;
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
      const controls = [...moveEffectEditorPanel.querySelectorAll('button, input, select')]
        .filter((element) => !element.disabled && !element.closest('.moveEffectPointList'))
        .filter((element) => {
          const rect = element.getBoundingClientRect();
          const style = window.getComputedStyle(element);
          return rect.width > 0 && rect.height > 0 && style.display !== 'none' && style.visibility !== 'hidden';
        });
      moveEffectLastControlReachable = controlReachableWhenScrolled(moveEffectForm, controls.at(-1));
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
    const patchGrid = document.querySelector('.dmxAddressGrid');
    const patchCells = patchGrid ? [...patchGrid.querySelectorAll('.dmxAddressCell')] : [];
    let patchGridMetrics = null;
    if (patchGrid instanceof HTMLElement) {
      const addressValues = patchCells
        .map((cell) => Number(cell.getAttribute('data-dmx-address')))
        .filter(Number.isFinite)
        .sort((left, right) => left - right);
      const cellRects = patchCells.map((cell) => cell.getBoundingClientRect());
      const roundedUnique = (values) => new Set(values.map((value) => Math.round(value * 10) / 10)).size;
      const gridStyle = getComputedStyle(patchGrid);
      const directGridRows = [...patchGrid.children].filter((child) => child.getAttribute('role') === 'row');
      const gridCells = [...patchGrid.querySelectorAll('[role="gridcell"]')];
      const lastCell = patchGrid.querySelector('[data-dmx-address="512"]');
      const setupPatchAddressDesk = patchGrid.closest('.setupPatchAddressDesk');
      const initialScroll = {
        gridLeft: patchGrid.scrollLeft,
        gridTop: patchGrid.scrollTop,
        windowX: window.scrollX,
        windowY: window.scrollY,
        documentLeft: documentElement.scrollLeft,
        documentTop: documentElement.scrollTop,
        bodyLeft: body.scrollLeft,
        bodyTop: body.scrollTop,
        appLeft: app?.scrollLeft ?? 0,
        appTop: app?.scrollTop ?? 0,
        layoutLeft: layout?.scrollLeft ?? 0,
        layoutTop: layout?.scrollTop ?? 0,
        deskLeft: setupPatchAddressDesk?.scrollLeft ?? 0,
        deskTop: setupPatchAddressDesk?.scrollTop ?? 0,
      };
      patchGrid.scrollLeft = patchGrid.scrollWidth;
      patchGrid.scrollTop = patchGrid.scrollHeight;
      await new Promise((resolveFrame) => requestAnimationFrame(resolveFrame));
      const gridRectAtEnd = patchGrid.getBoundingClientRect();
      const lastCellRectAtEnd = lastCell?.getBoundingClientRect() ?? null;
      const endReachable = Boolean(
        lastCellRectAtEnd &&
        lastCellRectAtEnd.left >= gridRectAtEnd.left - 1 &&
        lastCellRectAtEnd.right <= gridRectAtEnd.right + 1 &&
        lastCellRectAtEnd.top >= gridRectAtEnd.top - 1 &&
        lastCellRectAtEnd.bottom <= gridRectAtEnd.bottom + 1
      );
      const outerScrollUnchangedAtEnd =
        window.scrollX === initialScroll.windowX &&
        window.scrollY === initialScroll.windowY &&
        documentElement.scrollLeft === initialScroll.documentLeft &&
        documentElement.scrollTop === initialScroll.documentTop &&
        body.scrollLeft === initialScroll.bodyLeft &&
        body.scrollTop === initialScroll.bodyTop &&
        (app?.scrollLeft ?? 0) === initialScroll.appLeft &&
        (app?.scrollTop ?? 0) === initialScroll.appTop &&
        (layout?.scrollLeft ?? 0) === initialScroll.layoutLeft &&
        (layout?.scrollTop ?? 0) === initialScroll.layoutTop &&
        (setupPatchAddressDesk?.scrollLeft ?? 0) === initialScroll.deskLeft &&
        (setupPatchAddressDesk?.scrollTop ?? 0) === initialScroll.deskTop;

      const dispatchGridKey = async (key, modifiers = {}) => {
        const target = document.activeElement;
        if (!(target instanceof HTMLElement)) return '';
        target.dispatchEvent(new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true, ...modifiers }));
        await Promise.resolve();
        await new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame)));
        return document.activeElement?.getAttribute('data-dmx-address') ?? '';
      };
      const firstCell = patchGrid.querySelector('[data-dmx-address="1"]');
      firstCell?.focus({ preventScroll: true });
      const arrowRightAddress = await dispatchGridKey('ArrowRight');
      const arrowDownAddress = await dispatchGridKey('ArrowDown');
      const controlEndAddress = await dispatchGridKey('End', { ctrlKey: true });
      const focusedEndCell = document.activeElement;
      const focusedEndRect = focusedEndCell?.getBoundingClientRect() ?? null;
      const gridRectAfterKeyboard = patchGrid.getBoundingClientRect();
      const keyboardVisibleLeft = gridRectAfterKeyboard.left + patchGrid.clientLeft;
      const keyboardVisibleTop = gridRectAfterKeyboard.top + patchGrid.clientTop;
      const keyboardVisibleRight = keyboardVisibleLeft + patchGrid.clientWidth;
      const keyboardVisibleBottom = keyboardVisibleTop + patchGrid.clientHeight;
      const controlEndFullyVisible = Boolean(
        focusedEndRect &&
        focusedEndRect.left >= keyboardVisibleLeft - 1 &&
        focusedEndRect.right <= keyboardVisibleRight + 1 &&
        focusedEndRect.top >= keyboardVisibleTop - 1 &&
        focusedEndRect.bottom <= keyboardVisibleBottom + 1
      );
      const controlEndVisibility = focusedEndRect ? {
        cell: [focusedEndRect.left, focusedEndRect.top, focusedEndRect.right, focusedEndRect.bottom],
        viewport: [keyboardVisibleLeft, keyboardVisibleTop, keyboardVisibleRight, keyboardVisibleBottom],
        scroll: [patchGrid.scrollLeft, patchGrid.scrollTop],
      } : null;
      const outerScrollUnchangedAfterKeyboard =
        window.scrollX === initialScroll.windowX &&
        window.scrollY === initialScroll.windowY &&
        documentElement.scrollLeft === initialScroll.documentLeft &&
        documentElement.scrollTop === initialScroll.documentTop &&
        body.scrollLeft === initialScroll.bodyLeft &&
        body.scrollTop === initialScroll.bodyTop &&
        (app?.scrollLeft ?? 0) === initialScroll.appLeft &&
        (app?.scrollTop ?? 0) === initialScroll.appTop &&
        (layout?.scrollLeft ?? 0) === initialScroll.layoutLeft &&
        (layout?.scrollTop ?? 0) === initialScroll.layoutTop &&
        (setupPatchAddressDesk?.scrollLeft ?? 0) === initialScroll.deskLeft &&
        (setupPatchAddressDesk?.scrollTop ?? 0) === initialScroll.deskTop;
      await dispatchGridKey('Home', { ctrlKey: true });

      patchGrid.scrollLeft = initialScroll.gridLeft;
      patchGrid.scrollTop = initialScroll.gridTop;
      if (setupPatchAddressDesk) {
        setupPatchAddressDesk.scrollLeft = initialScroll.deskLeft;
        setupPatchAddressDesk.scrollTop = initialScroll.deskTop;
      }
      await new Promise((resolveFrame) => requestAnimationFrame(resolveFrame));
      patchGridMetrics = {
        pageSelectorCount: document.querySelectorAll('select[aria-label="DMX address page"]').length,
        addressCount: addressValues.length,
        uniqueAddressCount: new Set(addressValues).size,
        firstAddress: addressValues.at(0) ?? null,
        lastAddress: addressValues.at(-1) ?? null,
        rowCount: roundedUnique(cellRects.map((rect) => rect.top)),
        columnCount: roundedUnique(cellRects.map((rect) => rect.left)),
        minCellWidth: cellRects.length > 0 ? Math.min(...cellRects.map((rect) => rect.width)) : 0,
        maxCellWidth: cellRects.length > 0 ? Math.max(...cellRects.map((rect) => rect.width)) : 0,
        minCellHeight: cellRects.length > 0 ? Math.min(...cellRects.map((rect) => rect.height)) : 0,
        maxCellHeight: cellRects.length > 0 ? Math.max(...cellRects.map((rect) => rect.height)) : 0,
        clientWidth: patchGrid.clientWidth,
        clientHeight: patchGrid.clientHeight,
        scrollWidth: patchGrid.scrollWidth,
        scrollHeight: patchGrid.scrollHeight,
        overflowX: gridStyle.overflowX,
        overflowY: gridStyle.overflowY,
        endReachable,
        outerScrollUnchangedAtEnd,
        accessibleName: patchGrid.getAttribute('aria-label') ?? '',
        rowCountAria: Number(patchGrid.getAttribute('aria-rowcount') ?? 0),
        columnCountAria: Number(patchGrid.getAttribute('aria-colcount') ?? 0),
        directRowRoleCount: directGridRows.length,
        gridCellRoleCount: gridCells.length,
        invalidDirectGridChildCount: [...patchGrid.children]
          .filter((child) => child.getAttribute('role') !== 'row').length,
        addressButtonsOutsideGridCells: patchCells
          .filter((cell) => cell.parentElement?.getAttribute('role') !== 'gridcell').length,
        missingCellNameCount: patchCells.filter((cell) => !(cell.getAttribute('aria-label') || '').trim()).length,
        plannedCellMissingStateNameCount: patchCells
          .filter((cell) => cell.classList.contains('planned') && !(cell.getAttribute('aria-label') ?? '').includes('pending fixture '))
          .length,
        conflictCellMissingStateNameCount: patchCells
          .filter((cell) => cell.classList.contains('plannedConflict') && !(cell.getAttribute('aria-label') ?? '').includes('conflict'))
          .length,
        tabStopCount: patchCells.filter((cell) => cell.tabIndex === 0).length,
        arrowRightAddress,
        arrowDownAddress,
        controlEndAddress,
        controlEndFullyVisible,
        controlEndVisibility,
        outerScrollUnchangedAfterKeyboard,
        universeSelectorNamed: Boolean(document.querySelector('select[aria-label="DMX universe"]')),
        viewToggleGrouped: Boolean(document.querySelector('.viewToggle[role="group"][aria-label="DMX map view"]')),
        readoutNamed: Boolean(document.querySelector('.dmxPatchAddressReadout[aria-label="Current DMX address"]')),
        unnamedOverviewSegmentCount: document.querySelectorAll('.dmxPatchSegment:not([aria-label])').length,
      };
    }
    window.scrollTo(9999, 9999);
    await new Promise((resolveFrame) => requestAnimationFrame(resolveFrame));
    const movedX = window.scrollX;
    const movedY = window.scrollY;
    window.scrollTo(0, 0);
    await new Promise((resolveFrame) => requestAnimationFrame(resolveFrame));
    return {
      label: ${JSON.stringify(label)},
      windowMode: documentElement.getAttribute('data-window-mode') ?? '',
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
      visiblePersistentBandCount: visibleCount('.mappingPersistentWorkspaceBand'),
      visiblePersistentGroupsCount: visibleCount('[data-persistent-band-part="groups"]'),
      visiblePersistentStageCount: visibleCount('[data-persistent-band-part="stage"]'),
      visiblePersistentSelectionsCount: visibleCount('[data-workspace-selection-drawer][open]'),
      visiblePersistentContextCount: visibleCount('[data-persistent-band-part="context"]'),
      visibleSelectionDrawerToggleCount: visibleCount('[data-workspace-selection-drawer-toggle]'),
      visibleWorkspaceSplitterCount: visibleCount('[data-workspace-splitter]'),
      persistentBandRects: {
        groups: measuredRect('[data-persistent-band-part="groups"]'),
        stage: measuredRect('[data-workspace-pane="lower-left"]'),
        selections: measuredRect('[data-workspace-selection-drawer][open]'),
        context: measuredRect('[data-workspace-pane="lower-right"]'),
      },
      workspacePaneRects: {
        upper: measuredRect('[data-workspace-pane="upper"]'),
        lower: measuredRect('[data-workspace-pane="lower"]'),
      },
      workspaceSplitterRects: {
        upperLower: measuredRect('[data-workspace-splitter="upper-lower"]'),
        lowerLeftRight: measuredRect('[data-workspace-splitter="lower-left-right"]'),
      },
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
      visibleJapaneseLiveAudioStoppedCount: [...document.querySelectorAll('.liveAudioHealthAnnouncement')]
        .filter((node) => (node.textContent || '').trim() === 'ライブ音声入力は停止中です。').length,
      visibleJapaneseLiveAudioMeterLabelCount: [...document.querySelectorAll('.liveAudioMeters [role="meter"]')]
        .filter((node) => ['低域レベル', '中域レベル', '高域レベル'].includes(node.getAttribute('aria-label') || '')).length,
      visibleJapaneseLiveAudioMeterValueCount: [...document.querySelectorAll('.liveAudioMeters [role="meter"]')]
        .filter((node) => (node.getAttribute('aria-valuetext') || '') === '0パーセント').length,
      visibleJapaneseLiveAudioIoUnavailableCount: [...document.querySelectorAll('.liveAudioConfigFormat')]
        .filter((node) => (node.textContent || '').trim() === 'I/Oを利用できません').length,
      preservedUserFixtureLabelCount: [...document.querySelectorAll('.fixtureList .fixture strong, .mappingFixtureList .mappingFixtureRowButton strong')]
        .filter((node) => ['Video', 'Save', 'Output'].includes((node.textContent || '').trim())).length,
      translatedUserFixtureCollisionCount: [...document.querySelectorAll('.fixtureList .fixture strong, .mappingFixtureList .mappingFixtureRowButton strong')]
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
      mappingSidebarUnsafeOverflowCount: [...document.querySelectorAll('.mappingSelectionsColumn, .mappingSetupContextContent')]
        .filter((element) => {
          const style = getComputedStyle(element);
          const unsafeX = element.scrollWidth > element.clientWidth + 1 && !['auto', 'scroll'].includes(style.overflowX);
          const unsafeY = element.scrollHeight > element.clientHeight + 1 && !['auto', 'scroll'].includes(style.overflowY);
          return unsafeX || unsafeY;
        }).length,
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
      patchGridMetrics,
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
      videoSetupMappingLastControlReachable: lastControlReachableWhenScrolled(
        '.videoSetupPanel .videoSetupMapPane > .videoOutputMapping',
        'button, input, select',
      ),
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
      videoSetupActionDockHeight: videoSetupActionDockRect ? Math.round(videoSetupActionDockRect.height) : 0,
      videoSetupActionDockLastActionReachable: lastControlReachableWhenScrolled(
        '.videoSetupPanel .videoOutputActionDock',
        'button',
      ),
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
      calibratedEmitterDetailCount: [...document.querySelectorAll('.setupMode-library .profileFunctionRow small b')]
        .filter((detail) => (detail.textContent ?? '').startsWith('Emitter ') && (detail.textContent ?? '').includes(' xyY '))
        .length,
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
      controlStageFixtureMinSize: (() => {
        const sizes = [...document.querySelectorAll('.controlStage .stageFixtureHitTarget')].map((target) => {
          const rect = target.getBoundingClientRect();
          return Math.min(rect.width, rect.height);
        });
        return sizes.length > 0 ? Math.min(...sizes) : 0;
      })(),
      controlStageFixtureGlyphMetrics: [...document.querySelectorAll('.controlStage .stageFixture')].map((fixture) => {
        const shapeRect = fixture.querySelector('.stageFixtureShape')?.getBoundingClientRect();
        const hitRect = fixture.querySelector('.stageFixtureHitTarget')?.getBoundingClientRect();
        return {
          kind: [...fixture.classList].find((className) => className.startsWith('kind-'))?.slice(5) ?? 'unknown',
          glyphWidth: Math.round((shapeRect?.width ?? 0) * 100) / 100,
          glyphHeight: Math.round((shapeRect?.height ?? 0) * 100) / 100,
          hitWidth: Math.round((hitRect?.width ?? 0) * 100) / 100,
          hitHeight: Math.round((hitRect?.height ?? 0) * 100) / 100,
        };
      }),
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
      visibleCueContextDrawerCount: visibleCount('[data-timeline-context-drawer-panel="cue"]'),
      cueContextDrawerBodyOverflowY: cueContextDrawerBody ? window.getComputedStyle(cueContextDrawerBody).overflowY : '',
      cueContextDrawerBodyVerticalOverflowPx: cueContextDrawerBody
        ? Math.max(0, cueContextDrawerBody.scrollHeight - cueContextDrawerBody.clientHeight)
        : 0,
      cueContextDrawerBodyHorizontalOverflowPx: cueContextDrawerBody
        ? Math.max(0, cueContextDrawerBody.scrollWidth - cueContextDrawerBody.clientWidth)
        : 0,
      cueContextDrawerContained: Boolean(
        cueHostRect && cueContextDrawerRect &&
        cueContextDrawerRect.left >= cueHostRect.left - 1 &&
        cueContextDrawerRect.right <= cueHostRect.right + 1 &&
        cueContextDrawerRect.top >= cueHostRect.top - 1 &&
        cueContextDrawerRect.bottom <= cueHostRect.bottom + 1
      ),
      cueContextDrawerBodyContained: Boolean(
        cueContextDrawerRect && cueContextDrawerBodyRect &&
        cueContextDrawerBodyRect.left >= cueContextDrawerRect.left - 1 &&
        cueContextDrawerBodyRect.right <= cueContextDrawerRect.right + 1 &&
        cueContextDrawerBodyRect.top >= cueContextDrawerRect.top - 1 &&
        cueContextDrawerBodyRect.bottom <= cueContextDrawerRect.bottom + 1
      ),
      cueContextDrawerLastControlReachable: lastVisibleControlReachableWhenScrolled(
        '[data-timeline-context-drawer-panel="cue"] .timelineContextDrawerBody',
        'button:not(:disabled), input:not(:disabled), select:not(:disabled), textarea:not(:disabled)',
      ),
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
      killButtonCount: visibleCount('.liveTransportGrid .killButton'),
      killClearCount: visibleCount('.liveTransportGrid .killClear'),
      killButtonBorderIsRed: (() => {
        const button = [...document.querySelectorAll('.liveTransportGrid .killButton')]
          .find((node) => node.getBoundingClientRect().width > 0);
        if (!button) return false;
        const parts = window.getComputedStyle(button).borderTopColor.match(/\\d+/g)?.map(Number) ?? [];
        return parts.length >= 3 && parts[0] > parts[1] + 20 && parts[0] > parts[2] + 20;
      })(),
      liveStatusMinFontPx: (() => {
        const nodes = [...document.querySelectorAll('.liveStatusItem span, .liveStatusItem strong')]
          .filter((node) => node.getBoundingClientRect().width > 0);
        if (nodes.length === 0) return 0;
        return Math.min(...nodes.map((node) => parseFloat(window.getComputedStyle(node).fontSize) || 0));
      })(),
      liveCueIdentityChipCount: visibleCount('.liveCueIdentityChip'),
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
      sceneBlockLinkBadgeCount: document.querySelectorAll('.sceneBlockLinkBadge').length,
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
      sceneBlockWorkspaceOverflowY: sceneBlockWorkspace ? window.getComputedStyle(sceneBlockWorkspace).overflowY : '',
      sceneBlockLastControlReachable: lastControlReachableWhenScrolled(
        '.sceneBlockWorkspace',
        '.sceneBlockComposer input, .sceneBlockComposer select, .sceneBlockComposer button, .sceneBlockRow input, .sceneBlockRow select, .sceneBlockRow button',
      ),
      sceneBlockLastComposerControlReachable: lastControlReachableWhenScrolled(
        '.sceneBlockWorkspace',
        '.sceneBlockComposer input, .sceneBlockComposer select, .sceneBlockComposer button',
      ),
      sceneBlockLastRowActionReachable: lastControlReachableWhenScrolled(
        '.sceneBlockList',
        '.sceneBlockRowActions button',
      ),
      sceneBlockListHorizontalOverflowPx: sceneBlockList
        ? Math.max(0, sceneBlockList.scrollWidth - sceneBlockList.clientWidth)
        : 0,
      sceneBlockListVerticalOverflowPx: sceneBlockList
        ? Math.max(0, sceneBlockList.scrollHeight - sceneBlockList.clientHeight)
        : 0,
      sceneBlockListOverflowY: sceneBlockList ? window.getComputedStyle(sceneBlockList).overflowY : '',
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
      visibleEffectFamilyButtonCount: visibleCount('.effectFamilyChooser button'),
      visibleActiveEffectFamilyButtonCount: visibleCount('.effectFamilyChooser button.active[aria-pressed="true"]'),
      activeEffectFamily: document.querySelector('.effectFamilyChooser button.active[aria-pressed="true"]')
        ?.getAttribute('data-effect-family') ?? '',
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
        .filter((option) => (option.textContent || '').trim().toLowerCase() === 'audio reactive').length,
      nodeGraphLiveAudioOptionCount: [...document.querySelectorAll('.nodeGraphPanel option')]
        .filter((option) => (option.textContent || '').trim().toLowerCase() === 'live input').length,
      visibleAudioReactiveRackCount: visibleCount('.audioReactiveRack'),
      visibleAudioReactiveRackMeterCount: visibleCount('.audioReactiveRack meter'),
      visibleAudioReactiveCanvasCount: visibleCount('.audioReactiveMode .nodeGraphCanvas'),
      visibleAudioReactiveVjStripCount: visibleCount('.audioReactiveVjStrip'),
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
      visibleVideoMixerOuterHeaderCount: visibleCount('.videoControlPanelMixer > .panelHeader'),
      videoMixerOuterHeaderHeight: Math.round(
        document.querySelector('.videoControlPanelMixer > .panelHeader')?.getBoundingClientRect().height ?? 0,
      ),
      videoMixerBodyTopGap: (() => {
        const panel = document.querySelector('.videoControlPanelMixer')?.getBoundingClientRect();
        const pane = document.querySelector('.videoMixerClipPane')?.getBoundingClientRect();
        return panel && pane ? Math.round(pane.top - panel.top) : -1;
      })(),
      videoMixerClipPaneWidth: Math.round(
        document.querySelector('.videoMixerClipPane')?.getBoundingClientRect().width ?? 0,
      ),
      videoMixerProgramPaneWidth: Math.round(
        document.querySelector('.videoMixerProgramPane')?.getBoundingClientRect().width ?? 0,
      ),
      videoMixerLayerPaneWidth: Math.round(
        document.querySelector('.videoMixerLayerPane')?.getBoundingClientRect().width ?? 0,
      ),
      videoMonitorPreviewWidth: Math.round(
        document.querySelector('[data-live-video-monitor="preview"]')?.getBoundingClientRect().width ?? 0,
      ),
      videoMonitorProgramWidth: Math.round(
        document.querySelector('[data-live-video-monitor="program"]')?.getBoundingClientRect().width ?? 0,
      ),
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
      liveAudioRailControlOverflowY: (() => {
        const controls = document.querySelector('.videoMixerClipPane > .liveAudioInputBar .liveAudioInputControls');
        return controls ? Math.max(0, controls.scrollHeight - controls.clientHeight) : 0;
      })(),
      liveAudioRailPrimaryControlMinHeight: (() => {
        const controls = [...document.querySelectorAll(
          '.videoMixerClipPane > .liveAudioInputBar .liveAudioInputControls select, .videoMixerClipPane > .liveAudioInputBar .liveAudioInputControls button',
        )];
        return controls.length > 0
          ? Math.round(Math.min(...controls.map((element) => element.getBoundingClientRect().height)))
          : 0;
      })(),
      liveAudioRailConfigControlMinHeight: (() => {
        const controls = [...document.querySelectorAll(
          '.videoMixerClipPane > .liveAudioInputBar .liveAudioConfigControls select',
        )];
        return controls.length > 0
          ? Math.round(Math.min(...controls.map((element) => element.getBoundingClientRect().height)))
          : 0;
      })(),
      visibleLiveAudioConfigLabelCount: [...document.querySelectorAll(
        '.videoMixerClipPane > .liveAudioInputBar .liveAudioConfigControls label > span',
      )].filter((element) => {
        const rect = element.getBoundingClientRect();
        const style = getComputedStyle(element);
        return rect.width >= 8 && rect.height >= 8 && style.visibility !== 'hidden' && style.clipPath === 'none';
      }).length,
      liveAudioRailMeterCount: visibleCount('.videoMixerClipPane > .liveAudioInputBar [role="meter"]'),
      liveAudioRailPoliteRegionCount: visibleCount('.videoMixerClipPane > .liveAudioInputBar [aria-live="polite"]'),
      liveAudioRailTelemetryBadgeCount: visibleCount('.videoMixerClipPane > .liveAudioInputBar .liveAudioTelemetry > span'),
      liveAudioRailTelemetryText: [...document.querySelectorAll(
        '.videoMixerClipPane > .liveAudioInputBar .liveAudioTelemetry > span',
      )].map((element) => (element.textContent || '').trim().replace(/\\s+/g, ' ')).join(' | '),
      liveAudioRailTelemetryTruncatedCount: [...document.querySelectorAll(
        '.videoMixerClipPane > .liveAudioInputBar .liveAudioTelemetry > span, .videoMixerClipPane > .liveAudioInputBar .liveAudioTelemetryIo > i',
      )].filter((element) => element.scrollWidth - element.clientWidth > 1).length,
      liveAudioRailCriticalTelemetryOverflowCount: [...document.querySelectorAll(
        '.videoMixerClipPane > .liveAudioInputBar .liveAudioTelemetryOvr, .videoMixerClipPane > .liveAudioInputBar .liveAudioTelemetryLatency, .videoMixerClipPane > .liveAudioInputBar .liveAudioTelemetryQueue',
      )].filter((element) => element.scrollWidth - element.clientWidth > 1).length,
      liveAudioRailFullscreenCriticalOverflowCount: [...document.querySelectorAll(
        '.videoMixerClipPane > .liveAudioInputBar .liveAudioTelemetryOvr, .videoMixerClipPane > .liveAudioInputBar .liveAudioTelemetryXrun, .videoMixerClipPane > .liveAudioInputBar .liveAudioTelemetryLatency, .videoMixerClipPane > .liveAudioInputBar .liveAudioTelemetryIo, .videoMixerClipPane > .liveAudioInputBar .liveAudioTelemetryIo > i, .videoMixerClipPane > .liveAudioInputBar .liveAudioTelemetryQueue',
      )].filter((element) => element.scrollWidth - element.clientWidth > 1).length,
      liveAudioRailTelemetryOutsideCount: (() => {
        const telemetry = document.querySelector('.videoMixerClipPane > .liveAudioInputBar .liveAudioTelemetry');
        if (!telemetry) return 0;
        const bounds = telemetry.getBoundingClientRect();
        return [...telemetry.children].filter((element) => {
          const rect = element.getBoundingClientRect();
          return rect.left < bounds.left - 1 || rect.right > bounds.right + 1 || rect.top < bounds.top - 1 || rect.bottom > bounds.bottom + 1;
        }).length;
      })(),
      videoClipGridClientHeight: (() => {
        const grid = document.querySelector('.videoMixerClipPane > .videoClipGridPanel .videoClipGrid');
        return grid ? grid.clientHeight : 0;
      })(),
      fullyVisibleVideoClipPadCount: (() => {
        const grid = document.querySelector('.videoMixerClipPane > .videoClipGridPanel .videoClipGrid')?.getBoundingClientRect();
        if (!grid) return 0;
        return [...document.querySelectorAll('.videoMixerClipPane .videoClipPad')].filter((element) => {
          const rect = element.getBoundingClientRect();
          return rect.width > 0 && rect.height > 0 && rect.left >= grid.left - 1 && rect.right <= grid.right + 1 && rect.top >= grid.top - 1 && rect.bottom <= grid.bottom + 1;
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
      visibleBuiltinVideoFxSelectCount: visibleCount('.videoIsfPanel [data-video-isf-action="builtin"]'),
      visibleVideoMixerLayerDeckCount: visibleCount('.videoMixerLayerDeck'),
      visibleVideoMixerLayerFaderCount: visibleCount('.videoControlPanelMixer .videoMixerLayerDeck input[type="range"]'),
      visibleVideoMixerLayerButtonCount: visibleCount('.videoControlPanelMixer .videoMixerLayerDeck button'),
      visibleVideoDeckPagerCount: visibleCount('.videoControlPanelMixer .deckPager'),
      mixerDrawerBarCount: visibleCount('[data-mixer-drawer-toggle]'),
      mixerDrawerOpenCount: [...document.querySelectorAll('[data-mixer-drawer-toggle]')]
        .filter((bar) => bar.getAttribute('aria-expanded') === 'true').length,
      videoClipGridScrollDelta: (() => {
        const grid = document.querySelector('.videoMixerClipPane .videoClipGrid');
        return grid ? grid.scrollHeight - grid.clientHeight : -1;
      })(),
      videoMixerMonitorHeightRatio: (() => {
        const pane = document.querySelector('.videoMixerProgramPane');
        const monitors = document.querySelector('.liveVideoMonitorPanel');
        if (!pane || !monitors) return -1;
        const paneHeight = pane.getBoundingClientRect().height - 40;
        if (paneHeight <= 0) return -1;
        return Number((monitors.getBoundingClientRect().height / paneHeight).toFixed(3));
      })(),
      fullyVisibleVideoLayerItemCount: (() => {
        const pane = document.querySelector('.videoMixerLayerPane');
        if (!pane) return 0;
        const paneRect = pane.getBoundingClientRect();
        return [...document.querySelectorAll('.videoMixerLayerPane .videoLayerItem')].filter((item) => {
          const rect = item.getBoundingClientRect();
          return rect.height > 0 && rect.top >= paneRect.top - 1 && rect.bottom <= paneRect.bottom + 1;
        }).length;
      })(),
      visibleMixerKillButtonCount: visibleCount('.videoControlPanelMixer .killButton'),
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
      visibleTouchSurfaceCount: visibleCount('[data-touch-surface]'),
      visibleTouchModeToggleCount: visibleCount('.touchSurfaceModeToggle button'),
      visibleTouchPageTabCount: visibleCount('.touchPageTabs button[aria-pressed]'),
      visibleTouchPageAddCount: visibleCount('.touchPageAdd'),
      visibleTouchPageRemoveCount: visibleCount('.touchPageRemove'),
      touchSurfaceMode: document.querySelector('[data-touch-surface]')?.getAttribute('data-touch-mode') ?? '',
      touchDefaultPresetVisible:
        document.querySelector('[data-touch-surface]')?.getAttribute('data-touch-default-preset') === 'true' &&
        document.querySelector('.touchSurfaceGrid')?.getAttribute('data-touch-page') === 'Default Desk',
      touchActivePageLabel: document.querySelector('.touchSurfaceGrid')?.getAttribute('data-touch-page') ?? '',
      visibleTouchPlacedControlCount: visibleCount('[data-touch-control]'),
      touchPlacedKinds: [...new Set([...document.querySelectorAll('[data-touch-control]')]
        .filter((element) => {
          const rect = element.getBoundingClientRect();
          const style = window.getComputedStyle(element);
          return rect.width > 0 && rect.height > 0 && style.display !== 'none' && style.visibility !== 'hidden';
        })
        .map((element) => element.getAttribute('data-touch-kind')))].filter(Boolean).sort(),
      touchComposedCheckPassed: window.__syndocalTouchSurfaceCheck?.passed === true,
      touchComposedCheckResult: window.__syndocalTouchSurfaceCheck ?? null,
      touchDocumentAndAppScrollZero:
        window.scrollX === 0 && window.scrollY === 0 &&
        (!app || (app.scrollLeft === 0 && app.scrollTop === 0)),
      touchPlacedUndersizedCount: [...document.querySelectorAll('[data-touch-control]')]
        .filter((element) => {
          const rect = element.getBoundingClientRect();
          const style = window.getComputedStyle(element);
          return rect.width > 0 && rect.height > 0 && style.display !== 'none' && style.visibility !== 'hidden';
        })
        .filter((element) => {
          const rect = element.getBoundingClientRect();
          return rect.width < 47.5 || rect.height < 47.5;
        }).length,
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
    result.timelineAutomationCurveCount >= 2 &&
    result.timelineAutomationValueInputCount >= 2 &&
    result.timelineAutomationEnabledToggleCount >= 1 &&
    result.timelineAutomationScopeCount >= 1 &&
    result.timelineAutomationBatchButtonCount >= 2 &&
    result.timelineAutomationGroupButtonCount >= 1 &&
    result.timelineAutomationRangeWidths.length >= 2 &&
    result.timelineAutomationRangeWidths.every((width) => width > 0)
  );
}

function hasExpectedKillZone(result) {
  // T5: the three blackouts form a KILL visual family (red-family borders,
  // never the GO accent), All Clear sits beside them, Active/Next cue carry
  // identity chips, and Live Desk status text respects the 11px floor.
  if (!/^control-live-\d+x\d+$/.test(result.label)) {
    return true;
  }
  return (
    result.killButtonCount === 3 &&
    result.killClearCount === 1 &&
    result.killButtonBorderIsRed === true &&
    result.liveStatusMinFontPx >= 11 &&
    result.liveCueIdentityChipCount >= 1
  );
}

function hasExpectedSceneBlocks(result) {
  if (!/^control-live-\d+x\d+$/.test(result.label)) {
    return true;
  }
  // T15 removes the always-on Cues / Scene Blocks desk from the default Show
  // surface. The blocks remain rendered on the timeline; editing is opened on
  // demand through Block Properties and is covered by the focused drawer and
  // 500-block contracts.
  return (
    result.visibleSceneBlockWorkspaceCount === 0 &&
    result.sceneBlockRowCount === 0 &&
    result.visibleSceneBlockRowCount === 0 &&
    result.timelineSceneBlockCount === 2 &&
    result.timelinePointEventCount === 1 &&
    result.timelineActiveSceneBlockCount === 1 &&
    result.timelineActivePointEventCount === 0 &&
    result.sceneBlockLaneScopeHintCount === 0 &&
    result.sceneBlockPlaybackJumpHintCount === 0 &&
    result.sceneBlockLinkBadgeCount === 0 &&
    result.visibleSceneBlockComposerControlCount === 0 &&
    result.visibleSceneBlockRowActionCount === 0 &&
    !result.timelineTimeStatClipped
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
  if (result.label.startsWith("localization-ja-live-audio-")) {
    return (
      result.documentLanguage === "ja" &&
      result.liveAudioAcceptancePassed === true &&
      result.liveAudioAcceptance?.locale === "ja" &&
      result.liveAudioAcceptance?.checks?.localizedSystemDefaultTitle === true &&
      result.liveAudioAcceptance?.checks?.refreshShowsChecking === true &&
      result.liveAudioAcceptance?.checks?.uniqueGenerationRemap === true &&
      result.liveAudioAcceptance?.checks?.ambiguousIdentityFailsClosed === true &&
      result.liveAudioAcceptance?.checks?.explicitReselectionUnlocksStart === true &&
      result.liveAudioAcceptance?.checks?.startShowsChecking === true &&
      result.liveAudioAcceptance?.checks?.localizedNonzeroMeters === true &&
      result.liveAudioAcceptance?.checks?.safetyClearGatesRestart === true
    );
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

function expectsPersistentWorkspaceBand(result) {
  return (
    result.label.startsWith("setup-") ||
    result.label.startsWith("control-edit-") ||
    result.label.startsWith("control-live-") ||
    result.label.startsWith("mapping-") ||
    result.label.startsWith("interface-scale-") ||
    result.label.startsWith("persistent-band-invariance-")
  );
}

function hasExpectedPersistentWorkspaceBand(result) {
  if (!expectsPersistentWorkspaceBand(result)) return true;
  const rects = result.persistentBandRects ?? {};
  const oldControlPreviewAbsent = !result.label.startsWith("control-") || (
    result.visibleControlStagePanelCount === 0 && result.visibleControlStageCount === 0
  );
  return (
    result.visiblePersistentBandCount === 1 &&
    result.visiblePersistentGroupsCount === 1 &&
    result.visiblePersistentStageCount === 1 &&
    result.visibleSelectionDrawerToggleCount === 1 &&
    result.visiblePersistentContextCount === 1 &&
    result.visibleWorkspaceSplitterCount === 2 &&
    rects.groups?.width >= 428 && rects.groups?.height >= 24 &&
    rects.stage?.width >= 428 && rects.stage?.height >= 120 &&
    rects.context?.width >= 478 && rects.context?.height >= 120 &&
    oldControlPreviewAbsent
  );
}

function hasExpectedPersistentBandInvariance(result) {
  return !result.label.startsWith("persistent-band-invariance-") || result.persistentBandInvariant === true;
}

function hasExpectedTimelinePaneExpansion(result) {
  return !result.label.startsWith("persistent-band-invariance-") || result.timelinePaneExpansion?.passed === true;
}

function hasExpectedLayeredTimelineDesk(result) {
  return !result.label.startsWith("persistent-band-invariance-") || result.layeredTimelineDesk?.passed === true;
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
    (
      result.visibleLiveControlPanelCount !== 1 ||
      result.visiblePersistentBandCount !== 1 ||
      result.visiblePersistentStageCount !== 1 ||
      result.visibleSelectionDrawerToggleCount !== 1 ||
      result.visiblePersistentContextCount !== 1 ||
      result.visibleWorkspaceSplitterCount !== 2 ||
      result.visibleControlStagePanelCount !== 0 ||
      result.visibleControlStageCount !== 0
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
      result.positionConsoleHeight >= 80 &&
      result.positionToolPaneHeight >= 60 &&
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
      result.visiblePositionFavoriteButtonCount >= 3;
    const hasContextPaneOperationalLayout =
      result.positionConsoleWidth >= 360 &&
      result.positionToolDeckWidth >= 180 &&
      result.positionPadWidth >= 72 &&
      result.positionPadHeight >= 72 &&
      result.positionPadWidth / Math.max(1, result.positionPadHeight) >= 0.75 &&
      result.positionPadWidth / Math.max(1, result.positionPadHeight) <= 1.35 &&
      result.positionPrimaryAndToolsSideBySide;
    return (
      hasPositionConsole &&
      hasPrimaryPositionTools &&
      hasContextPaneOperationalLayout
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
        result.visibleLiveControlPanelCount === 1 &&
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
        const hasContextPaneOperationalDensity =
          result.moveEffectEditorWidth >= 180 &&
          result.moveEffectSurfaceWidth >= 180 &&
          result.moveEffectPathDeskWidth >= 180 &&
          result.moveEffectInspectorWidth >= 180 &&
          result.moveEffectPathCanvasWidth >= 160 &&
          result.moveEffectPathCanvasHeight >= 120 &&
          result.moveEffectCanvasContained &&
          result.moveEffectSurfaceWidthCoverage >= 0.98 &&
          result.moveEffectColumnAreaCoverage >= 0.98 &&
          result.moveEffectColumnAreaCoverage <= 1.02 &&
          result.moveEffectUnusedRightPx <= 2 &&
          result.moveEffectUnusedBottomPx <= 2;
        return hasMoveEditor && hasContextPaneOperationalDensity;
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
      if (result.label.startsWith("control-edit-effects-color-family-")) {
        return (
          hasFxDesk &&
          result.visibleEffectEditorCount === 1 &&
          result.visibleEffectFamilyButtonCount === 9 &&
          result.visibleActiveEffectFamilyButtonCount === 1 &&
          result.visibleEffectLibraryCardCount === 1 &&
          result.visibleTargetRequiredEffectCardCount === 1 &&
          result.controlWorkSurfaceUnsafeOverflowCount === 0
        );
      }
      const expectedRecipeCountByFamily = {
        "COLOR FX": 1,
        "CHASER FX": 1,
        "MOVE FX": 1,
        "VALUE FX": 4,
        "CURVE FX": 2,
        "MAPPINGS": 3,
        "COLOR MAPPINGS": 1,
      };
      const expectedTargetRequiredCountByFamily = {
        "COLOR FX": 1,
        "CHASER FX": 1,
        "MOVE FX": 1,
        "VALUE FX": 0,
        "CURVE FX": 0,
        "MAPPINGS": 0,
        "COLOR MAPPINGS": 1,
      };
      return (
        hasFxDesk &&
        result.visibleEditDeskTabCount === 3 &&
        result.visibleFixtureEditSurfaceCount === 0 &&
        result.visibleEffectEditorCount === 1 &&
        result.visibleEffectFamilyButtonCount === 9 &&
        result.visibleActiveEffectFamilyButtonCount === 1 &&
        result.visibleEffectLibraryCardCount === expectedRecipeCountByFamily[result.activeEffectFamily] &&
        result.visibleTargetRequiredEffectCardCount === expectedTargetRequiredCountByFamily[result.activeEffectFamily] &&
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
    const upperPaneHeight = result.workspacePaneRects?.upper?.height ?? 0;
    if (upperPaneHeight <= 0 || Math.abs(result.liveControlPanelHeight - upperPaneHeight) > 4) return false;
    if (
      result.visibleLiveFadeMeterCount !== 1 ||
      result.liveFadeMeterGridRow !== "4" ||
      result.liveMasterGridRow !== "5" ||
      !result.liveFadeAboveMaster
    ) return false;
    if (result.label.startsWith("control-live-playback-")) {
      return (
        result.visibleTimelineDeskTabCount === 3 &&
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
      const cueDrawerIsContained =
        result.visibleTimelineDeskTabCount === 3 &&
        result.visibleTimelinePanelCount === 1 &&
        result.visibleTimelineShowSurfaceCount === 1 &&
        result.visibleTimelineAutomationSurfaceCount === 0 &&
        result.visiblePlaybackDeskSurfaceCount === 0 &&
        result.visibleCueContextDrawerCount === 1 &&
        result.visibleCuePanelCount === 1 &&
        result.visibleCueLivePanelCount === 1 &&
        result.cuePanelOverflowY === "visible" &&
        result.cuePanelHorizontalOverflowPx <= 1 &&
        result.cuePanelVerticalOverflowPx <= 1 &&
        ["auto", "scroll"].includes(result.cueContextDrawerBodyOverflowY) &&
        result.cueContextDrawerBodyHorizontalOverflowPx <= 1 &&
        result.cueContextDrawerContained &&
        result.cueContextDrawerBodyContained &&
        result.cueContextDrawerLastControlReachable &&
        result.cueHostHorizontalOverflowPx <= 1 &&
        result.cueHostVerticalOverflowPx <= 1 &&
        result.controlWorkSurfaceUnsafeOverflowCount === 0;
      if (result.label.startsWith("control-live-cues-effects-only-")) {
        return (
          cueDrawerIsContained &&
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
          result.cueContextDrawerBodyVerticalOverflowPx > 0
        );
      }
      if (
        result.label.startsWith("control-live-cues-edit-") ||
        /^control-live-cues-\d+x\d+$/.test(result.label)
      ) {
        return (
          cueDrawerIsContained &&
          result.visibleCueFormCount === 1 &&
          result.visibleCueEffectRecallEditorCount >= 1 &&
          result.visibleCueEditOnlyCount > 0 &&
          cueToggleIsAccessible &&
          result.cueEditToggleExpanded === "true" &&
          result.cueContextDrawerBodyVerticalOverflowPx > 0
        );
      }
      return (
        cueDrawerIsContained &&
        result.visibleCueFormCount === 0 &&
        result.visibleCueEffectRecallEditorCount === 0 &&
        result.visibleCueEditOnlyCount === 0 &&
        cueToggleIsAccessible &&
        result.cueEditToggleExpanded === "false"
      );
    }
    if (result.label.startsWith("control-live-automation-")) {
      return (
        result.visibleTimelineDeskTabCount === 3 &&
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
      result.visibleTimelineDeskTabCount === 3 &&
      result.visibleTimelineShowSurfaceCount === 1 &&
      result.controlWorkSurfaceUnsafeOverflowCount === 0 &&
      result.visibleFixtureEditSurfaceCount === 0 &&
      result.visibleEffectEditorCount === 0 &&
      result.visibleRawMonitorCount === 0 &&
      result.visibleVideoControlPanelCount === 0
    );
  }
  if (result.label.startsWith("control-mixer-")) {
    const liveTelemetryExpected = result.label.startsWith("control-mixer-live-");
    const liveTelemetryIsValid =
      !liveTelemetryExpected ||
      (result.liveAudioRailTelemetryBadgeCount === 6 &&
        result.liveAudioRailCriticalTelemetryOverflowCount === 0 &&
        result.liveAudioRailTelemetryOutsideCount === 0);
    const mixerChecks = {
      exclusiveSurface:
        result.visibleLiveControlPanelCount === 0 &&
        result.visibleControlStagePanelCount === 0 &&
        result.visibleControlStageCount === 0 &&
        result.visibleVideoControlPanelCount > 0,
      threePaneMixer:
        result.visibleVideoMixerClipPaneCount === 1 &&
        result.visibleVideoMixerProgramPaneCount === 1 &&
        result.visibleVideoMixerLayerPaneCount === 1,
      liveMonitors:
        result.visibleVideoMonitorPanelCount === 1 &&
        result.visibleVideoPreviewBusCount === 1 &&
        result.visibleVideoProgramBusCount === 1,
      previewTransport:
        result.visibleVjPreviewTransportCount === 1 &&
        result.visibleVjPreviewTransportButtonCount === 4 &&
        result.disabledVjPreviewTransportButtonCount === 4 &&
        result.undersizedVjPreviewTransportButtonCount === 0,
      previewStage:
        result.visibleVjPreviewStageButtonCount > 0 &&
        result.disabledVjPreviewStageButtonCount === result.visibleVjPreviewStageButtonCount &&
        result.undersizedVjPreviewStageButtonCount === 0 &&
        result.visibleVideoProgramRefreshCount === 0,
      masterAndClips:
        result.visibleVideoMasterControlCount > 0 &&
        result.visibleVideoMasterFaderCount > 0 &&
        result.visibleVideoClipGridCount > 0 &&
        result.visibleVideoClipPadCount > 0 &&
        result.visibleVideoClipTakeButtonCount > 0,
      audioAndDecks:
        result.visibleVideoClipAudioButtonCount > 0 &&
        result.visibleVideoProgramAudioToggleCount > 0 &&
        result.visibleVideoAbDeckCount > 0 &&
        result.visibleVideoDeckLoadButtonCount >= 2 &&
        result.visibleVideoRecordingBarCount > 0,
      liveAudioRail: liveTelemetryExpected
        ? result.visibleLiveAudioInputBarCount === 1 &&
          result.liveAudioInputBarDomCount === 1 &&
          result.visibleLiveAudioRailCount === 1 &&
          result.visibleEmbeddedLiveAudioCount === 0 &&
          result.liveAudioRailHeight > 0 &&
          result.liveAudioRailHeight <= 70 &&
          result.liveAudioRailBelowMaster === true &&
          result.liveAudioRailAboveClipGrid === true &&
          result.liveAudioRailOverflowX <= 1 &&
          result.liveAudioRailOverflowY <= 1 &&
          result.liveAudioRailControlOverflowY <= 1 &&
          result.liveAudioRailMeterCount === 3 &&
          result.liveAudioRailPoliteRegionCount === 1 &&
          result.mixerDrawerBarCount === 3 &&
          result.mixerDrawerOpenCount >= 1
        : // T6 default state: the three drawers are collapsed and the rail is
          // hidden until its drawer opens (the live scenario opens it first).
          result.mixerDrawerBarCount === 3 &&
          result.mixerDrawerOpenCount === 0 &&
          result.visibleLiveAudioInputBarCount === 0 &&
          result.liveAudioInputBarDomCount === 1 &&
          result.visibleEmbeddedLiveAudioCount === 0,
      liveTelemetry: liveTelemetryIsValid,
      clipGrid:
        // T6 renegotiation: the old >=90px floor assumed two stacked text rows
        // per pad; thumbnail-first pads guarantee one usable >=56px row, and
        // the full 12-pad bank contract is asserted by the vj-bank scenarios.
        result.videoClipGridClientHeight >= 56 &&
        result.fullyVisibleVideoClipPadCount >= 1 &&
        (liveTelemetryExpected || result.videoClipGridScrollDelta <= 1),
      monitorDominance: result.videoMixerMonitorHeightRatio >= 0.65,
      killVocabulary: result.visibleMixerKillButtonCount >= 1,
      outputDecks:
        result.visibleVideoOutputControlListCount > 0 &&
        result.visibleVideoOutputItemCount > 0 &&
        result.visibleVideoOutputSelectedItemCount > 0 &&
        result.visibleVideoMixerOutputDeckCount >= result.visibleVideoOutputItemCount &&
        result.visibleVideoMixerOutputFaderCount >= result.visibleVideoOutputItemCount &&
        result.visibleVideoMixerOutputSelectButtonCount >= result.visibleVideoOutputItemCount,
      layerDecks:
        result.visibleVideoLayerListCount > 0 &&
        result.visibleVideoLayerItemCount > 0 &&
        result.visibleBuiltinVideoFxSelectCount > 0 &&
        result.visibleVideoMixerLayerDeckCount >= result.visibleVideoLayerItemCount &&
        result.visibleVideoMixerLayerFaderCount >= result.visibleVideoLayerItemCount &&
        result.visibleVideoMixerLayerButtonCount >= 5 &&
        result.visibleVideoDeckPagerCount >= 2,
      noLegacyPanels:
        result.controlWorkSurfaceUnsafeOverflowCount === 0 &&
        result.visibleVideoMixerDiagnosticsCount === 0 &&
        result.visibleVideoMixerSetupToolsCount === 0 &&
        result.visibleVideoMixerAutomationToolsCount === 0 &&
        result.visibleOutputPanelCount === 0,
    };
    result.controlModeFailedChecks = Object.entries(mixerChecks)
      .filter(([, passed]) => !passed)
      .map(([name]) => name);
    return result.controlModeFailedChecks.length === 0;
  }
  return true;
}

function hasExpectedContinuousPatchGrid(result) {
  const metrics = result.patchGridMetrics;
  if (!metrics) return false;
  const hasBoundedInternalScroll =
    (metrics.scrollHeight > metrics.clientHeight + 1 && ["auto", "scroll"].includes(metrics.overflowY)) ||
    (metrics.scrollWidth > metrics.clientWidth + 1 && ["auto", "scroll"].includes(metrics.overflowX));
  const avoidsUnneededHorizontalScroll =
    result.innerWidth < 1600 || metrics.scrollWidth <= metrics.clientWidth + 1;
  return (
    metrics.pageSelectorCount === 0 &&
    metrics.addressCount === 512 &&
    metrics.uniqueAddressCount === 512 &&
    metrics.firstAddress === 1 &&
    metrics.lastAddress === 512 &&
    metrics.rowCount === 16 &&
    metrics.columnCount === 32 &&
    metrics.minCellWidth >= 16 &&
    metrics.maxCellWidth <= 24 &&
    metrics.minCellHeight >= 16 &&
    metrics.maxCellHeight <= 24 &&
    hasBoundedInternalScroll &&
    avoidsUnneededHorizontalScroll &&
    metrics.endReachable &&
    metrics.outerScrollUnchangedAtEnd &&
    /^Universe \d+ DMX addresses 1 to 512$/.test(metrics.accessibleName) &&
    metrics.rowCountAria === 16 &&
    metrics.columnCountAria === 32 &&
    metrics.directRowRoleCount === 16 &&
    metrics.gridCellRoleCount === 512 &&
    metrics.invalidDirectGridChildCount === 0 &&
    metrics.addressButtonsOutsideGridCells === 0 &&
    metrics.missingCellNameCount === 0 &&
    metrics.plannedCellMissingStateNameCount === 0 &&
    metrics.conflictCellMissingStateNameCount === 0 &&
    metrics.tabStopCount === 1 &&
    metrics.arrowRightAddress === "2" &&
    metrics.arrowDownAddress === "34" &&
    metrics.controlEndAddress === "512" &&
    metrics.controlEndFullyVisible &&
    metrics.outerScrollUnchangedAfterKeyboard &&
    metrics.universeSelectorNamed &&
    metrics.viewToggleGrouped &&
    metrics.readoutNamed &&
    metrics.unnamedOverviewSegmentCount === 0
  );
}

function hasExpectedSetupSurface(result) {
  if (result.label.startsWith("setup-library-")) {
    return (
      result.visibleProfileLoadPanelCount >= 1 &&
      result.visibleLoadedProfileSummaryPanelCount >= 1 &&
      result.profileLoadPanelWidth >= 250 &&
      result.loadedProfileSummaryPanelWidth >= 760 &&
      result.calibratedEmitterDetailCount >= 3
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
      result.dmxAddressCellCount === 512 &&
      result.dmxAddressOccupiedCellCount > 0 &&
      result.dmxAddressPlannedCellCount > 0 &&
      result.visibleDmxFixtureBlockCount > 0 &&
      result.visibleDmxGridSummaryCount >= 1 &&
      result.visibleFixtureSetupEditorCount >= 1 &&
      result.visibleUseProfileForPatchButtonCount >= 1 &&
      result.visibleDuplicateFixtureButtonCount >= 1 &&
      hasExpectedContinuousPatchGrid(result)
    );
  }
  if (result.label.startsWith("setup-mapping-")) {
    return (
      result.mappingUseInEffectsButtonCount >= 1 &&
      result.mappingWaveDraftButtonCount >= 1 &&
      result.visibleMappingProjectorButtonCount >= 1 &&
      result.visibleMappingProjectorControlsCount >= 1 &&
      // T9: Mapping is a lighting-only floor plan. Projection warp/keystone/corner editing
      // moved to Setup > Video, so the Mapping sidebar has no warp grid and no Reset Pose,
      // and the projection-surfaces layer defaults OFF (0 surface DOM nodes on the stage).
      result.visibleMappingProjectorWarpGridCount === 0 &&
      result.visibleMappingProjectorActionButtonCount >= 4 &&
      result.visibleMappingProjectorResetPoseButtonCount === 0 &&
      result.visibleStageVideoSurfaceCount === 0 &&
      result.mappingFilterVerticalClipCount === 0 &&
      result.mappingViewportChildOverlapCount === 0 &&
      result.mappingStageHeight >= 180 &&
      result.mappingSidebarUnsafeOverflowCount === 0
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
      result.videoSetupMapPaneHeight >= 200 &&
      result.videoSetupMapPaneOverflowPx <= 1 &&
      result.videoSetupMappingLastControlReachable &&
      result.videoSetupPreviewContained &&
      result.visibleVideoSetupActionDockCount === 1 &&
      result.videoSetupActionDockHeight >= 40 &&
      result.videoSetupActionDockLastActionReachable &&
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
    result.effectTargetHintText.toLowerCase().includes("mapping") &&
    result.effectTargetValue === "selection" &&
    result.effectTypeValue === "Mapping" &&
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
  const expectedPage = result.label.startsWith("touch-composed-") ? "Viewport Touch" : "Default Desk";
  return (
    result.visibleTouchSurfaceCount === 1 &&
    result.visibleTouchModeToggleCount === 2 &&
    result.visibleTouchPageTabCount >= 1 &&
    result.visibleTouchPageAddCount === 1 &&
    result.visibleTouchPageRemoveCount === 1 &&
    result.touchSurfaceMode === "live" &&
    result.touchActivePageLabel === expectedPage &&
    (expectedPage !== "Default Desk" || result.touchDefaultPresetVisible) &&
    result.visibleTouchPlacedControlCount >= 8 &&
    JSON.stringify(result.touchPlacedKinds) === JSON.stringify([
      "Button",
      "ColorWheel",
      "Dial",
      "Fader",
      "Image",
      "IncrementalWheel",
      "Label",
      "XyGrid",
    ]) &&
    result.visibleTouchSafetyDeckCount === 1 &&
    result.visibleTouchGoDeckCount > 0 &&
    result.visibleTouchMasterGridCount > 0 &&
    result.visibleTouchSafetyGuardButtonCount === 4 &&
    result.touchComposedCheckPassed &&
    result.touchDocumentAndAppScrollZero &&
    result.touchPlacedUndersizedCount === 0 &&
    result.visibleTouchRemotePanelCount === 0 &&
    result.touchUndersizedTargetCount === 0
  );
}

async function checkEditableTouchSurface(client, expectedPage, expectedDefaultPreset) {
  return await client.evaluate(`(async () => {
    const expectedPage = ${JSON.stringify(expectedPage)};
    const expectedDefaultPreset = ${JSON.stringify(expectedDefaultPreset)};
    const surface = document.querySelector('[data-touch-surface]');
    const safetyDeck = document.querySelector('.touchSafetyDeck');
    const modeButtons = [...document.querySelectorAll('.touchSurfaceModeToggle button')];
    const editButton = modeButtons[0];
    const liveButton = modeButtons[1];
    if (!surface || !safetyDeck || !editButton || !liveButton) {
      window.__syndocalTouchSurfaceCheck = { passed: false, found: false };
      return false;
    }

    liveButton.click();
    await new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame)));
    const initialControls = [...surface.querySelectorAll('[data-touch-control]')];
    const initialControlCount = initialControls.length;
    const initialKinds = [...new Set(initialControls.map((control) => control.getAttribute('data-touch-kind')))].filter(Boolean).sort();
    const operatedKinds = [];

    const button = surface.querySelector('[data-touch-kind="Button"] .touchPlacedButton:not(:disabled)');
    button?.click();
    if (button) operatedKinds.push('Button');

    for (const kind of ['Fader', 'Dial']) {
      const input = surface.querySelector('[data-touch-kind="' + kind + '"] input[type="range"]:not(:disabled)');
      if (input instanceof HTMLInputElement) {
        input.value = '0.37';
        input.dispatchEvent(new InputEvent('input', { bubbles: true }));
        operatedKinds.push(kind);
      }
    }

    const incremental = surface.querySelector('[data-touch-kind="IncrementalWheel"] button:not(:disabled)');
    incremental?.click();
    if (incremental) operatedKinds.push('IncrementalWheel');

    const color = surface.querySelector('[data-touch-kind="ColorWheel"] input[type="color"]:not(:disabled)');
    if (color instanceof HTMLInputElement) {
      color.value = '#22aa88';
      color.dispatchEvent(new InputEvent('input', { bubbles: true }));
      operatedKinds.push('ColorWheel');
    }

    const xy = surface.querySelector('[data-touch-kind="XyGrid"] .touchPlacedXy:not(:disabled)');
    if (xy) {
      const rect = xy.getBoundingClientRect();
      xy.dispatchEvent(new PointerEvent('pointerdown', {
        bubbles: true,
        pointerId: 91,
        pointerType: 'touch',
        isPrimary: true,
        clientX: rect.left + rect.width * 0.64,
        clientY: rect.top + rect.height * 0.36,
      }));
      operatedKinds.push('XyGrid');
    }

    await new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame)));
    const liveActivationKinds = [...surface.querySelectorAll('[data-touch-live-activated="true"]')]
      .map((element) => element.closest('[data-touch-kind]')?.getAttribute('data-touch-kind'))
      .filter(Boolean);
    const liveOperationsPassed = ['Button', 'Fader', 'Dial', 'IncrementalWheel', 'ColorWheel', 'XyGrid']
      .every((kind) => operatedKinds.includes(kind) && liveActivationKinds.includes(kind));

    editButton.click();
    await new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame)));
    const paletteLabels = [...document.querySelectorAll('.touchControlPalette button')]
      .map((candidate) => (candidate.textContent || '').trim());
    const expectedPalette = ['Label', 'Image', 'Button', 'Fader', 'Dial', 'Incremental Wheel', 'Color Wheel', 'XY Grid'];
    const editModeSameSurface = document.querySelector('[data-touch-surface]') === surface &&
      surface.getAttribute('data-touch-mode') === 'edit' &&
      surface.querySelectorAll('[data-touch-control]').length === initialControlCount;
    const editHandlesPresent = surface.querySelectorAll('.touchEditMoveHandle').length === initialControlCount &&
      surface.querySelectorAll('.touchEditResizeHandle').length === initialControlCount;
    const pageManagementPresent = document.querySelectorAll('.touchPageTabs button[aria-pressed]').length >= 1 &&
      document.querySelectorAll('.touchPageAdd').length === 1 &&
      document.querySelectorAll('.touchPageRemove').length === 1;
    const safetyDeckInEdit = document.querySelector('.touchSafetyDeck') === safetyDeck;

    liveButton.click();
    await new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame)));
    const liveModeSameSurface = document.querySelector('[data-touch-surface]') === surface &&
      surface.getAttribute('data-touch-mode') === 'live' &&
      surface.querySelectorAll('[data-touch-control]').length === initialControlCount;
    const safetyDeckInLive = document.querySelector('.touchSafetyDeck') === safetyDeck;
    const activePage = surface.querySelector('.touchSurfaceGrid')?.getAttribute('data-touch-page') ?? '';
    const defaultPresetMatches = surface.getAttribute('data-touch-default-preset') === String(expectedDefaultPreset);
    const palettePassed = JSON.stringify(paletteLabels) === JSON.stringify(expectedPalette);
    const allKindsPresent = JSON.stringify(initialKinds) === JSON.stringify([
      'Button', 'ColorWheel', 'Dial', 'Fader', 'Image', 'IncrementalWheel', 'Label', 'XyGrid'
    ]);

    window.__syndocalTouchSurfaceCheck = {
      passed:
        activePage === expectedPage &&
        defaultPresetMatches &&
        palettePassed &&
        allKindsPresent &&
        liveOperationsPassed &&
        editModeSameSurface &&
        liveModeSameSurface &&
        editHandlesPresent &&
        pageManagementPresent &&
        safetyDeckInEdit &&
        safetyDeckInLive,
      found: true,
      activePage,
      expectedPage,
      defaultPresetMatches,
      paletteLabels,
      palettePassed,
      initialKinds,
      allKindsPresent,
      operatedKinds,
      liveActivationKinds,
      liveOperationsPassed,
      editModeSameSurface,
      liveModeSameSurface,
      editHandlesPresent,
      pageManagementPresent,
      safetyDeckInEdit,
      safetyDeckInLive,
    };
    return window.__syndocalTouchSurfaceCheck.passed;
  })()`);
}

function clickLiveAudioControlInPage(kind) {
  const rail = document.querySelector(".videoMixerClipPane > .liveAudioInputBar");
  if (!rail) return false;
  const button = rail.querySelector(
    `[data-live-audio-action="${kind === "refresh" ? "refresh" : "transport"}"]`,
  );
  if (!button || button.disabled) return false;
  button.click();
  return true;
}

async function clickLiveAudioControl(client, kind) {
  return client.evaluate(
    "(" + clickLiveAudioControlInPage.toString() + ")(" + JSON.stringify(kind) + ")",
  );
}

function setLiveAudioSelectInPage(kind, value) {
  const rail = document.querySelector(".videoMixerClipPane > .liveAudioInputBar");
  if (!rail) return false;
  const select = rail.querySelector(`[data-live-audio-control="${kind}"]`);
  if (!select || select.disabled || ![...select.options].some((option) => option.value === value)) {
    return false;
  }
  select.value = value;
  select.dispatchEvent(new InputEvent("input", { bubbles: true, inputType: "insertReplacementText" }));
  return true;
}

async function setLiveAudioSelect(client, kind, value) {
  return client.evaluate(
    "(" + setLiveAudioSelectInPage.toString() + ")(" +
      JSON.stringify(kind) + "," + JSON.stringify(value) + ")",
  );
}

async function runLiveAudioAcceptance(client, locale, label) {
  await installLiveAudioInvokeMock(client);
  await sleep(60);
  // T6: the live audio rail sits behind the collapsed "Audio In" drawer by
  // default; open it so every existing rail assertion measures the open state.
  await openMixerDrawer(client, "audio-in", ".videoMixerClipPane > .liveAudioInputBar");
  const initial = await readLiveAudioRailState(client);
  const refreshClicked = await clickLiveAudioControl(client, "refresh");
  await sleep(45);
  const refreshBusy = await readLiveAudioRailState(client);
  await waitForClientCondition(
    client,
    "(() => { const action = [...document.querySelectorAll('.videoMixerClipPane > .liveAudioInputBar .liveAudioInputControls > button')].at(-1); return Boolean(action && !action.disabled && document.querySelector('.videoMixerClipPane > .liveAudioInputBar .liveAudioConfigFormat')?.textContent?.includes('WASAPI') && window.__syndocalLiveAudioMock?.calls.some((call) => call.command === 'get_live_audio_input_capabilities')); })()",
    "Live audio device refresh and default capability resolution",
  );
  const refreshed = await readLiveAudioRailState(client);

  const deviceSelected = await setLiveAudioSelect(client, "device", "viewport-wasapi-studio-g1");
  await waitForClientCondition(
    client,
    "(() => { const calls = window.__syndocalLiveAudioMock?.calls ?? []; const latest = calls.filter((call) => call.command === 'get_live_audio_input_capabilities').at(-1); const action = [...document.querySelectorAll('.videoMixerClipPane > .liveAudioInputBar .liveAudioInputControls > button')].at(-1); return latest?.args?.deviceId === 'viewport-wasapi-studio-g1' && Boolean(action && !action.disabled); })()",
    "Explicit live audio input capability resolution",
  );

  const uniqueRefreshClicked = await clickLiveAudioControl(client, "refresh");
  await waitForClientCondition(
    client,
    "(() => { const calls = window.__syndocalLiveAudioMock?.calls ?? []; const latest = calls.filter((call) => call.command === 'get_live_audio_input_capabilities').at(-1); const device = document.querySelector('.videoMixerClipPane > .liveAudioInputBar .liveAudioInputControls select'); const action = [...document.querySelectorAll('.videoMixerClipPane > .liveAudioInputBar .liveAudioInputControls > button')].at(-1); return device?.value === 'viewport-wasapi-studio-g2' && latest?.args?.deviceId === 'viewport-wasapi-studio-g2' && Boolean(action && !action.disabled); })()",
    "Unique generation ID remap after live audio Refresh",
  );
  const uniquelyRemapped = await readLiveAudioRailState(client);

  const ambiguousRefreshClicked = await clickLiveAudioControl(client, "refresh");
  await waitForClientCondition(
    client,
    "window.__syndocalLiveAudioMock?.deviceGeneration === 3",
    "Ambiguous live audio identity fail-closed state",
  );
  await sleep(45);
  const ambiguousIdentity = await readLiveAudioRailState(client);

  const finalDeviceSelected = await setLiveAudioSelect(client, "device", "viewport-wasapi-studio-g3a");
  await waitForClientCondition(
    client,
    "(() => { const calls = window.__syndocalLiveAudioMock?.calls ?? []; const latest = calls.filter((call) => call.command === 'get_live_audio_input_capabilities').at(-1); const action = [...document.querySelectorAll('.videoMixerClipPane > .liveAudioInputBar .liveAudioInputControls > button')].at(-1); return latest?.args?.deviceId === 'viewport-wasapi-studio-g3a' && Boolean(action && !action.disabled); })()",
    "Explicit live audio reselection after ambiguous Refresh",
  );
  const rateSelected = await setLiveAudioSelect(client, "rate", "192000");
  await waitForClientCondition(
    client,
    "(() => { const calls = window.__syndocalLiveAudioMock?.calls ?? []; const latest = calls.filter((call) => call.command === 'get_live_audio_input_capabilities').at(-1); const action = [...document.querySelectorAll('.videoMixerClipPane > .liveAudioInputBar .liveAudioInputControls > button')].at(-1); return latest?.args?.sampleRate === 192000 && Boolean(action && !action.disabled); })()",
    "Selected live audio sample-rate capability resolution",
  );
  const bufferSelected = await setLiveAudioSelect(client, "buffer", "8192");
  const mixSelected = await setLiveAudioSelect(client, "mix", "stereo_pair:0:1");
  await sleep(45);
  const configured = await readLiveAudioRailState(client);

  const asioBackendSelected = await setLiveAudioSelect(client, "backend", "asio");
  await waitForClientCondition(
    client,
    "window.__syndocalLiveAudioMock?.asioGeneration === 1 && document.querySelector('.videoMixerClipPane > .liveAudioInputBar')?.getAttribute('data-live-audio-backend-state') === 'select_device'",
    "ASIO explicit driver selection gate",
  );
  const asioUnselected = await readLiveAudioRailState(client);
  const asioDriverSelected = await setLiveAudioSelect(client, "device", "viewport-asio-studio-g1");
  await waitForClientCondition(
    client,
    "(() => { const calls = window.__syndocalLiveAudioMock?.calls ?? []; const latest = calls.filter((call) => call.command === 'get_live_audio_input_capabilities').at(-1); const rate = document.querySelector('[data-live-audio-control=\"rate\"]'); return latest?.args?.backend === 'asio' && latest?.args?.deviceId === 'viewport-asio-studio-g1' && Boolean(rate && !rate.disabled); })()",
    "ASIO driver capability resolution",
  );
  await sleep(30);
  const asioDriverOnly = await readLiveAudioRailState(client);
  const asioRateSelected = await setLiveAudioSelect(client, "rate", "48000");
  await waitForClientCondition(
    client,
    "(() => { const calls = window.__syndocalLiveAudioMock?.calls ?? []; const latest = calls.filter((call) => call.command === 'get_live_audio_input_capabilities').at(-1); const buffer = document.querySelector('[data-live-audio-control=\"buffer\"]'); return latest?.args?.backend === 'asio' && latest?.args?.sampleRate === 48000 && Boolean(buffer && !buffer.disabled); })()",
    "ASIO explicit sample-rate gate",
  );
  await sleep(30);
  const asioRateOnly = await readLiveAudioRailState(client);
  const asioBufferSelected = await setLiveAudioSelect(client, "buffer", "128");
  await sleep(30);
  const asioReady = await readLiveAudioRailState(client);
  const asioRefreshClicked = await clickLiveAudioControl(client, "refresh");
  await waitForClientCondition(
    client,
    "window.__syndocalLiveAudioMock?.asioGeneration === 2",
    "ASIO generation refresh",
  );
  await sleep(45);
  const asioAfterRefresh = await readLiveAudioRailState(client);

  const wasapiBackendRestored = await setLiveAudioSelect(client, "backend", "wasapi_shared");
  await waitForClientCondition(
    client,
    "window.__syndocalLiveAudioMock?.deviceGeneration === 4 && document.querySelector('.videoMixerClipPane > .liveAudioInputBar')?.getAttribute('data-live-audio-backend-state') === 'ready'",
    "WASAPI backend restoration",
  );
  const restoredDeviceSelected = await setLiveAudioSelect(client, "device", "viewport-wasapi-studio-g3a");
  await waitForClientCondition(
    client,
    "(() => { const calls = window.__syndocalLiveAudioMock?.calls ?? []; const latest = calls.filter((call) => call.command === 'get_live_audio_input_capabilities').at(-1); const rate = document.querySelector('[data-live-audio-control=\"rate\"]'); return latest?.args?.backend === 'wasapi_shared' && latest?.args?.deviceId === 'viewport-wasapi-studio-g3a' && Boolean(rate && !rate.disabled); })()",
    "Restored WASAPI device capability resolution",
  );
  const restoredRateSelected = await setLiveAudioSelect(client, "rate", "192000");
  await waitForClientCondition(
    client,
    "(() => { const calls = window.__syndocalLiveAudioMock?.calls ?? []; const latest = calls.filter((call) => call.command === 'get_live_audio_input_capabilities').at(-1); const buffer = document.querySelector('[data-live-audio-control=\"buffer\"]'); return latest?.args?.backend === 'wasapi_shared' && latest?.args?.sampleRate === 192000 && Boolean(buffer && !buffer.disabled); })()",
    "Restored WASAPI sample rate",
  );
  const restoredBufferSelected = await setLiveAudioSelect(client, "buffer", "8192");
  const restoredMixSelected = await setLiveAudioSelect(client, "mix", "stereo_pair:0:1");
  await sleep(30);
  const restoredWasapi = await readLiveAudioRailState(client);

  const startClicked = await clickLiveAudioControl(client, "action");
  await sleep(45);
  const startBusy = await readLiveAudioRailState(client);
  await waitForClientCondition(
    client,
    "document.querySelector('.videoMixerClipPane > .liveAudioInputBar')?.getAttribute('data-health') === 'live'",
    "Live audio Start response",
  );
  await sleep(60);
  const live = await readLiveAudioRailState(client);
  const liveContainment = await measure(client, label);

  const stopClicked = await clickLiveAudioControl(client, "action");
  await sleep(45);
  const stopBusy = await readLiveAudioRailState(client);
  await waitForClientCondition(
    client,
    "document.querySelector('.videoMixerClipPane > .liveAudioInputBar')?.getAttribute('data-health') === 'clearing'",
    "Live audio Stop safety-clear response",
  );
  await sleep(60);
  const clearing = await readLiveAudioRailState(client);
  const calls = await client.evaluate(
    "JSON.parse(JSON.stringify(window.__syndocalLiveAudioMock?.calls ?? []))",
  );
  const startCall = calls.find((call) => call.command === "start_live_audio_input");
  const expected = locale === "ja"
    ? {
        systemDefaultTitle: "システム既定の音声入力",
        systemDefaultOption: "システム既定",
        reselectTitle: "音声入力を再選択",
        reselectFormat: "入力を再選択",
        checking: "確認中",
        liveAnnouncement: "ライブ音声入力は動作中です。",
        clearingAction: "クリア待ち",
        clearingAnnouncement: "ライブ音声の停止はエンジンの安全クリア待ちです。開始はロックされています。",
        selectAsioDriver: "ASIOドライバを選択",
        meterLabels: ["低域レベル", "中域レベル", "高域レベル"],
        meterValues: ["42パーセント", "58パーセント", "76パーセント"],
        meterTitles: ["低域 42%", "中域 58%", "高域 76%"],
      }
    : {
        systemDefaultTitle: "System default audio input",
        systemDefaultOption: "System default",
        reselectTitle: "Reselect audio input",
        reselectFormat: "Reselect input",
        checking: "Checking",
        liveAnnouncement: "Live audio input active.",
        clearingAction: "Clear Pending",
        clearingAnnouncement: "Live audio Stop is waiting for the engine safety clear. Start is locked.",
        selectAsioDriver: "Select ASIO driver",
        meterLabels: ["Bass level", "Mid level", "High level"],
        meterValues: ["42 percent", "58 percent", "76 percent"],
        meterTitles: ["Bass 42%", "Mid 58%", "High 76%"],
      };
  const request = startCall?.args?.request;
  const checks = {
    initialStopped: initial?.health === "stopped",
    localizedSystemDefaultTitle: initial?.deviceTitle === expected.systemDefaultTitle,
    refreshClicked,
    refreshShowsChecking:
      refreshBusy?.actionText === expected.checking &&
      refreshBusy?.actionDisabled === true &&
      refreshBusy?.refreshDisabled === true,
    deviceChoicesResolved:
      refreshed?.deviceOptions?.includes("viewport-wasapi-studio-g1") &&
      refreshed?.deviceOptions?.includes("viewport-wasapi-line-g1"),
    uniqueGenerationRemap:
      deviceSelected &&
      uniqueRefreshClicked &&
      uniquelyRemapped?.selectedDevice === "viewport-wasapi-studio-g2" &&
      uniquelyRemapped?.deviceOptions?.includes("viewport-wasapi-studio-g2") &&
      !uniquelyRemapped?.deviceOptions?.includes("viewport-wasapi-studio-g1") &&
      uniquelyRemapped?.deviceInvalid === "" &&
      uniquelyRemapped?.actionDisabled === false,
    ambiguousIdentityFailsClosed:
      ambiguousRefreshClicked &&
      ambiguousIdentity?.selectedDevice === "viewport-wasapi-studio-g2" &&
      ambiguousIdentity?.deviceOptions?.includes("viewport-wasapi-studio-g2") &&
      ambiguousIdentity?.deviceOptions?.includes("viewport-wasapi-studio-g3a") &&
      ambiguousIdentity?.deviceOptions?.includes("viewport-wasapi-studio-g3b") &&
      ambiguousIdentity?.deviceInvalid === "true" &&
      ambiguousIdentity?.deviceTitle === expected.reselectTitle &&
      ambiguousIdentity?.configFormat?.endsWith(expected.reselectFormat) &&
      ambiguousIdentity?.actionDisabled === true,
    explicitReselectionUnlocksStart:
      finalDeviceSelected &&
      configured?.selectedDevice === "viewport-wasapi-studio-g3a" &&
      configured?.deviceInvalid === "" &&
      configured?.deviceTitle === "Viewport Studio Microphone · WASAPI (#1)" &&
      configured?.actionDisabled === false,
    rateOptionsResolved:
      rateSelected &&
      ["44100", "48000", "88200", "96000", "176400", "192000"]
        .every((value) => configured?.sampleRateOptions?.includes(value)),
    bufferOptionsResolved:
      bufferSelected &&
      ["64", "128", "256", "512", "1024", "2048", "4096", "8192"]
        .every((value) => configured?.bufferOptions?.includes(value)),
    mixOptionsResolved:
      mixSelected &&
      ["average_all", "single:0", "single:1", "stereo_pair:0:1"]
        .every((value) => configured?.mixOptions?.includes(value)),
    exactResolvedFormat: configured?.configFormat?.endsWith("WASAPI · f32 · 2ch"),
    startClicked,
    startShowsChecking:
      startBusy?.actionText === expected.checking && startBusy?.actionDisabled === true,
    exactStartRequest:
      request?.backend === "wasapi_shared" &&
      request?.device_id === "viewport-wasapi-studio-g3a" &&
      request?.sample_rate === 192_000 &&
      request?.stream_channels === 2 &&
      request?.sample_format === "f32" &&
      request?.buffer_frames === 8_192 &&
      request?.channel_mix?.mode === "stereo_pair" &&
      request?.channel_mix?.left_channel_index === 0 &&
      request?.channel_mix?.right_channel_index === 1,
    liveState:
      live?.health === "live" &&
      live?.backend === "wasapi_shared" &&
      live?.backendState === "active" &&
      live?.announcement === expected.liveAnnouncement &&
      live?.telemetry?.length === 6 &&
      live.telemetry[0] === "ACTIVE" &&
      live.telemetry[1] === "OVR 9999/999999f" &&
      live.telemetry[2] === "XRUN 7" &&
      live.xrunWarning === true &&
      live.deviceDisabled === true,
    sixteenBandRhythmPresentation:
      live?.meters?.length === 3 &&
      live?.visualBands?.length === 16 &&
      live.visualBands.every((band) => band.role === "") &&
      live.visualBands[0]?.transform === "scaleY(0.05)" &&
      live.visualBands[15]?.transform === "scaleY(0.8)" &&
      live.onset === "true" &&
      live.rhythm === "BPM 128.0 · 87%",
    localizedNonzeroMeters:
      JSON.stringify(live?.meters?.map((meter) => meter.label)) === JSON.stringify(expected.meterLabels) &&
      JSON.stringify(live?.meters?.map((meter) => meter.now)) === JSON.stringify([42, 58, 76]) &&
      JSON.stringify(live?.meters?.map((meter) => meter.valueText)) === JSON.stringify(expected.meterValues) &&
      JSON.stringify(live?.meters?.map((meter) => meter.title)) === JSON.stringify(expected.meterTitles),
    stopClicked,
    stopShowsChecking:
      stopBusy?.actionText === expected.checking && stopBusy?.actionDisabled === true,
    safetyClearGatesRestart:
      clearing?.health === "clearing" &&
      clearing?.actionText === expected.clearingAction &&
      clearing?.actionDisabled === true &&
      clearing?.deviceDisabled === true &&
      clearing?.announcement === expected.clearingAnnouncement &&
      clearing?.safetyMessage?.length > 0 &&
      clearing?.meters?.length === 3 &&
      clearing.meters.every((meter) => meter.now === 0) &&
      clearing?.visualBands?.length === 16 &&
      clearing.visualBands.every((band) => band.transform === "scaleY(0)") &&
      clearing.onset === "false",
    invokeSequence:
      calls.filter((call) => call.command === "live_audio_input_backends").length === 1 &&
      calls.filter(
        (call) => call.command === "list_audio_input_devices" && call.args?.backend === "wasapi_shared",
      ).length === 4 &&
      calls.filter(
        (call) => call.command === "list_audio_input_devices" && call.args?.backend === "asio",
      ).length === 2 &&
      calls.filter((call) => call.command === "get_live_audio_input_capabilities").length >= 10 &&
      calls.filter((call) => call.command === "start_live_audio_input").length === 1 &&
      calls.filter((call) => call.command === "stop_live_audio_input").length === 1,
    backendContractVisible:
      configured?.backend === "wasapi_shared" &&
      configured?.backendBuilt === "true" &&
      configured?.backendOptions?.includes("wasapi_shared") &&
      configured?.backendOptions?.includes("asio"),
    asioRequiresExplicitDriver:
      asioBackendSelected &&
      asioUnselected?.backend === "asio" &&
      asioUnselected?.backendState === "select_device" &&
      asioUnselected?.selectedDevice === "" &&
      asioUnselected?.deviceOptionLabels?.[0] === expected.selectAsioDriver &&
      !asioUnselected?.deviceOptionLabels?.includes(expected.systemDefaultOption) &&
      asioUnselected?.actionDisabled === true,
    asioRequiresExplicitRateAndFixedBuffer:
      asioDriverSelected &&
      asioDriverOnly?.backendState === "configure" &&
      asioDriverOnly?.selectedSampleRate === "" &&
      asioDriverOnly?.selectedBuffer === "" &&
      asioDriverOnly?.actionDisabled === true &&
      asioRateSelected &&
      asioRateOnly?.selectedSampleRate === "48000" &&
      asioRateOnly?.selectedBuffer === "" &&
      asioRateOnly?.actionDisabled === true &&
      asioBufferSelected &&
      asioReady?.selectedBuffer === "128" &&
      asioReady?.backendState === "ready" &&
      asioReady?.actionDisabled === false,
    asioGenerationNeverNameRemaps:
      asioRefreshClicked &&
      asioAfterRefresh?.selectedDevice === "viewport-asio-studio-g1" &&
      asioAfterRefresh?.deviceOptions?.includes("viewport-asio-studio-g1") &&
      asioAfterRefresh?.deviceOptions?.includes("viewport-asio-studio-g2") &&
      asioAfterRefresh?.deviceInvalid === "true" &&
      asioAfterRefresh?.actionDisabled === true,
    wasapiRestoresWithoutLosingAutoContract:
      wasapiBackendRestored &&
      restoredDeviceSelected &&
      restoredRateSelected &&
      restoredBufferSelected &&
      restoredMixSelected &&
      restoredWasapi?.backend === "wasapi_shared" &&
      restoredWasapi?.backendState === "ready" &&
      restoredWasapi?.selectedDevice === "viewport-wasapi-studio-g3a" &&
      restoredWasapi?.actionDisabled === false,
  };
  const failedChecks = Object.entries(checks)
    .filter(([, passed]) => !passed)
    .map(([name]) => name);
  const acceptance = {
    passed: failedChecks.length === 0,
    locale,
    checks,
    failedChecks,
    initial,
    refreshBusy,
    refreshed,
    uniquelyRemapped,
    ambiguousIdentity,
    configured,
    asioUnselected,
    asioDriverOnly,
    asioRateOnly,
    asioReady,
    asioAfterRefresh,
    restoredWasapi,
    startBusy,
    live,
    stopBusy,
    clearing,
    calls,
  };
  if (!acceptance.passed) {
    throw new Error("Live audio UI acceptance failed: " + JSON.stringify(acceptance));
  }
  // T6: close the audio-in drawer again so its localStorage-persisted open
  // state cannot leak into later scenarios that assert the collapsed default.
  await client.evaluate(
    "(() => { const bar = document.querySelector('[data-mixer-drawer-toggle=\"audio-in\"]'); " +
    "if (bar && bar.getAttribute('aria-expanded') === 'true') bar.click(); })()",
  );
  await sleep(60);
  return {
    ...acceptance,
    liveContainment: {
      ...liveContainment,
      liveAudioAcceptancePassed: true,
      liveAudioAcceptance: acceptance,
    },
  };
}

async function prepareLiveAudioAcceptanceViewport(client, viewport, locale, fullscreen = false) {
  await client.send("Emulation.setDeviceMetricsOverride", {
    width: viewport.width,
    height: viewport.height,
    deviceScaleFactor: 1,
    mobile: false,
  });
  await client.send("Page.navigate", { url: appUrl });
  await waitForApp(client);
  // Reset persisted UI state before applying the locale. A prior iteration's
  // acceptance run persists controlMode "mixer"; booting straight into the
  // full VJ-desk layout replaces the shared workspace band whose tab is
  // labeled "Mixer"/"ミキサー" with tabs labeled "VJ Desk"/"VJデスク", so the
  // scripted tab click below would not find its target.
  await client.evaluate(
    "window.localStorage.clear();" +
      "window.localStorage.setItem('syndocal.uiLocale.v1'," + JSON.stringify(locale) + ")",
  );
  await client.send("Page.navigate", { url: appUrl });
  await waitForApp(client);
  await pressKey(client, "F2");
  await sleep(120);
  await clickByText(client, locale === "ja" ? "ミキサー" : "Mixer");
  await sleep(120);
  if (fullscreen) {
    await client.evaluate("document.documentElement.setAttribute('data-window-mode','fullscreen')");
    await sleep(60);
  }
}

// T6: open one of the mixer drawers (audio-in / auto-vj / reactive) and wait
// until the strip that follows its bar is actually visible.
async function openMixerDrawer(client, drawerId, stripSelector) {
  await client.evaluate(
    `(() => { const bar = document.querySelector('[data-mixer-drawer-toggle="${drawerId}"]'); ` +
    `if (bar && bar.getAttribute('aria-expanded') !== 'true') bar.click(); })()`,
  );
  await waitForClientCondition(
    client,
    `(() => { const strip = document.querySelector('${stripSelector}'); ` +
    `return Boolean(strip && strip.getBoundingClientRect().height > 0); })()`,
    `Mixer drawer ${drawerId} open`,
  );
  await sleep(60);
}

async function runLiveAudioAcceptanceViewport(client, viewport, locale) {
  await prepareLiveAudioAcceptanceViewport(client, viewport, locale);
  return runLiveAudioAcceptance(
    client,
    locale,
    "live-audio-" + locale + "-" + viewport.width + "x" + viewport.height,
  );
}

async function runFullscreenVjAcceptanceViewport(client, viewport) {
  await prepareLiveAudioAcceptanceViewport(client, viewport, "en", true);
  await openMixerDrawer(client, "audio-in", ".videoMixerClipPane > .liveAudioInputBar");
  const stopped = await measure(
    client,
    "fullscreen-vj-stopped-" + viewport.width + "x" + viewport.height,
  );
  const acceptance = await runLiveAudioAcceptance(
    client,
    "en",
    "fullscreen-vj-live-" + viewport.width + "x" + viewport.height,
  );
  const live = acceptance.liveContainment;
  const focusViewport = viewport.width >= 1_600 && viewport.height >= 900;
  const monitorRatio = live.videoMonitorPreviewWidth > 0
    ? live.videoMonitorProgramWidth / live.videoMonitorPreviewWidth
    : 0;
  const requiredTelemetryTokens = ["OVR ", "XRUN ", "C→W ", "I/O ", "BUF ", "CB ", "Q "];
  const checks = focusViewport
    ? {
        statefulAcceptance: acceptance.passed,
        containedStopped: isContained(stopped),
        containedLive: isContained(live),
        fullscreenModeApplied: stopped.windowMode === "fullscreen" && live.windowMode === "fullscreen",
        duplicateHeaderRemoved:
          stopped.visibleVideoMixerOuterHeaderCount === 0 &&
          stopped.videoMixerOuterHeaderHeight === 0 &&
          stopped.videoMixerBodyTopGap <= 1,
        focusColumnWidths:
          live.videoMixerClipPaneWidth >= 680 &&
          live.videoMixerProgramPaneWidth >= 760 &&
          live.videoMixerLayerPaneWidth >= 300,
        programMonitorPriority: monitorRatio >= 1.45 && monitorRatio <= 1.58,
        audioDockHeight: live.liveAudioRailHeight >= 84 && live.liveAudioRailHeight <= 96,
        primaryControlTargets: stopped.liveAudioRailPrimaryControlMinHeight >= 32,
        configurationTargets: stopped.liveAudioRailConfigControlMinHeight >= 28,
        configurationLabelsVisible: stopped.visibleLiveAudioConfigLabelCount >= 4,
        telemetryPresent: live.liveAudioRailTelemetryBadgeCount === 6,
        telemetryUnabridged:
          live.liveAudioRailTelemetryTruncatedCount === 0 &&
          live.liveAudioRailFullscreenCriticalOverflowCount === 0 &&
          requiredTelemetryTokens.every((token) => live.liveAudioRailTelemetryText.includes(token)),
        telemetryContained: live.liveAudioRailTelemetryOutsideCount === 0,
      }
    : {
        statefulAcceptance: acceptance.passed,
        containedStopped: isContained(stopped),
        containedLive: isContained(live),
        fullscreenModeApplied: stopped.windowMode === "fullscreen" && live.windowMode === "fullscreen",
        compactHeaderPreserved:
          stopped.visibleVideoMixerOuterHeaderCount === 1 &&
          stopped.videoMixerOuterHeaderHeight >= 35 &&
          stopped.videoMixerBodyTopGap >= 35,
        compactAudioDockPreserved: live.liveAudioRailHeight >= 56 && live.liveAudioRailHeight <= 64,
        compactMonitorBalance: Math.abs(live.videoMonitorProgramWidth - live.videoMonitorPreviewWidth) <= 2,
        telemetryPresent: live.liveAudioRailTelemetryBadgeCount === 6,
        telemetryContained:
          live.liveAudioRailCriticalTelemetryOverflowCount === 0 &&
          live.liveAudioRailTelemetryOutsideCount === 0,
      };
  const failedChecks = Object.entries(checks)
    .filter(([, passed]) => !passed)
    .map(([name]) => name);
  return {
    viewport,
    focusViewport,
    passed: failedChecks.length === 0,
    checks,
    failedChecks,
    stopped,
    live,
    monitorRatio: Number(monitorRatio.toFixed(3)),
    acceptance,
  };
}

async function runAudioReactiveAcceptanceViewport(client, viewport) {
  await client.send("Emulation.setDeviceMetricsOverride", {
    width: viewport.width,
    height: viewport.height,
    deviceScaleFactor: 1,
    mobile: false,
  });
  const url = fixtureUrl("audio-reactive");
  await client.send("Page.navigate", { url });
  await waitForApp(client);
  await client.evaluate("window.localStorage.setItem('syndocal.uiLocale.v1','en')");
  await client.send("Page.navigate", { url });
  await waitForApp(client);
  await waitForClientCondition(
    client,
    "Boolean(document.querySelector('.videoMixerClipPane > .audioReactiveVjStrip'))",
    "Audio Reactive VJ strip",
  );
  await openMixerDrawer(client, "reactive", ".videoMixerClipPane > .audioReactiveVjStrip");
  const mixer = await measure(client, `audio-reactive-mixer-${viewport.width}x${viewport.height}`);
  const mixerContract = await client.evaluate(`(() => {
    const strip = document.querySelector('.audioReactiveVjStrip');
    const clipPane = document.querySelector('.videoMixerClipPane');
    const programPane = document.querySelector('.videoMixerProgramPane');
    const clipGrid = document.querySelector('.videoClipGridPanel');
    if (!strip || !clipPane || !programPane || !clipGrid) return null;
    const stripRect = strip.getBoundingClientRect();
    const paneRect = clipPane.getBoundingClientRect();
    const gridRect = clipGrid.getBoundingClientRect();
    return {
      stripCount: document.querySelectorAll('.audioReactiveVjStrip').length,
      rowCount: document.querySelectorAll('.audioReactiveVjRow').length,
      meterCount: document.querySelectorAll('.audioReactiveVjRow meter').length,
      toggleCount: document.querySelectorAll('.audioReactiveVjRow button[aria-pressed]').length,
      identityText: (document.querySelector('.audioReactiveVjIdentity strong')?.textContent || '').trim(),
      rowState: document.querySelector('.audioReactiveVjRow')?.getAttribute('data-runtime-state') || '',
      rowStatusText: (document.querySelector('.audioReactiveVjRow b')?.textContent || '').trim(),
      outputValue: Number(document.querySelector('.audioReactiveVjRow meter')?.value ?? -1),
      openRackText: (document.querySelector('.audioReactiveVjActions button')?.textContent || '').trim(),
      openRackAriaLabel: document.querySelector('.audioReactiveVjActions button')?.getAttribute('aria-label') || '',
      stripInsideClipPane: stripRect.left >= paneRect.left - 1 && stripRect.right <= paneRect.right + 1,
      stripAboveClipGrid: stripRect.bottom <= gridRect.top + 1,
      stripHeight: Math.round(stripRect.height),
      programWidth: Math.round(programPane.getBoundingClientRect().width),
      clipGridHeight: Math.round(gridRect.height),
      overflowX: Math.max(0, strip.scrollWidth - strip.clientWidth),
      overflowY: Math.max(0, strip.scrollHeight - strip.clientHeight),
    };
  })()`);

  await clickByText(client, "Live Edit");
  await sleep(120);
  await clickVisibleByText(client, ".editDeskTabs button", "Effects");
  await sleep(120);
  await clickVisibleByText(client, ".effectRackTabs button", "Graphs");
  await waitForClientCondition(client, "Boolean(document.querySelector('.audioReactiveRack'))", "Audio Reactive editor rack");
  const editor = await measure(client, `audio-reactive-editor-${viewport.width}x${viewport.height}`);
  const editorContract = await client.evaluate(`(() => {
    const rack = document.querySelector('.audioReactiveRack');
    const panel = document.querySelector('.nodeGraphPanel');
    if (!rack || !panel) return null;
    const panelRect = panel.getBoundingClientRect();
    const rackRect = rack.getBoundingClientRect();
    const fields = [...rack.querySelectorAll('input, select, button')];
    const meters = [...rack.querySelectorAll('meter')];
    const runtimeMeters = rack.querySelector('.audioReactiveMeters');
    const runtimeStatus = rack.querySelector('.audioReactiveMeters > b');
    const monitor = rack.querySelector('[data-audio-runtime-monitor]');
    return {
      rackCount: document.querySelectorAll('.audioReactiveRack').length,
      canvasCount: document.querySelectorAll('.audioReactiveMode .nodeGraphCanvas').length,
      sectionCount: rack.querySelectorAll('.audioReactiveSection').length,
      meterCount: meters.length,
      featureOptionCount: rack.querySelectorAll('select option').length,
      signalPathCount: rack.querySelectorAll('.audioReactiveSignalPath').length,
      runtimeAuthoritative: runtimeMeters?.getAttribute('data-runtime-authoritative') || '',
      runtimeNodeId: runtimeMeters?.getAttribute('data-runtime-node-id') || '',
      runtimeStatusText: (runtimeStatus?.textContent || '').trim(),
      runtimeInputValue: Number(meters[0]?.value ?? -1),
      runtimeOutputValue: Number(meters[1]?.value ?? -1),
      monitorValue: monitor?.value || '',
      monitorOptionCount: monitor?.querySelectorAll('option').length ?? 0,
      panelHorizontalOverflow: Math.max(0, panel.scrollWidth - panel.clientWidth),
      rackInsidePanelHorizontally: rackRect.left >= panelRect.left - 1 && rackRect.right <= panelRect.right + 1,
      firstFieldReachable: fields.length > 0 && fields[0].getBoundingClientRect().width > 0,
      lastFieldReachable: fields.length > 0 && fields.at(-1).getBoundingClientRect().width > 0,
    };
  })()`);
  const passed = Boolean(
    isContained(mixer) &&
    isContained(editor) &&
    mixerContract &&
    mixerContract.stripCount === 1 &&
    mixerContract.rowCount === 1 &&
    mixerContract.meterCount === 1 &&
    mixerContract.toggleCount === 1 &&
    mixerContract.identityText === "SAFE ZERO" &&
    mixerContract.rowState === "safe" &&
    mixerContract.rowStatusText === "SAFE ZERO" &&
    mixerContract.outputValue === 0 &&
    mixerContract.openRackText === "Open Rack" &&
    mixerContract.openRackAriaLabel === "Open Audio Reactive Rack" &&
    mixerContract.stripInsideClipPane &&
    mixerContract.stripAboveClipGrid &&
    mixerContract.stripHeight <= 56 &&
    mixerContract.programWidth >= 360 &&
    mixerContract.clipGridHeight >= 90 &&
    mixerContract.overflowX <= 1 &&
    mixerContract.overflowY <= 1 &&
    editorContract &&
    editorContract.rackCount === 1 &&
    editorContract.canvasCount === 0 &&
    editorContract.sectionCount === 2 &&
    editorContract.meterCount === 2 &&
    editorContract.featureOptionCount >= 18 &&
    editorContract.signalPathCount === 1 &&
    editorContract.runtimeAuthoritative === "true" &&
    editorContract.runtimeNodeId === "1" &&
    editorContract.runtimeStatusText === "SAFE ZERO" &&
    editorContract.runtimeInputValue === 0.78 &&
    editorContract.runtimeOutputValue === 0 &&
    editorContract.monitorValue === "301" &&
    editorContract.monitorOptionCount === 1 &&
    editorContract.panelHorizontalOverflow <= 1 &&
    editorContract.rackInsidePanelHorizontally &&
    editorContract.firstFieldReachable &&
    editorContract.lastFieldReachable
  );
  return { viewport, passed, mixer, mixerContract, editor, editorContract };
}

async function prepareOperatorVjAcceptanceViewport(client, viewport, locale) {
  await client.send("Emulation.setDeviceMetricsOverride", {
    width: viewport.width,
    height: viewport.height,
    deviceScaleFactor: 1,
    mobile: false,
  });
  const url = fixtureUrl("operator-vj");
  await client.send("Page.navigate", { url });
  await waitForApp(client);
  await client.evaluate(
    "window.localStorage.setItem('syndocal.uiLocale.v1'," + JSON.stringify(locale) + ")",
  );
  await client.send("Page.navigate", { url });
  await waitForApp(client);
  await waitForClientCondition(
    client,
    `(() =>
      typeof window.__syndocalReadOperatorVjFixtureSnapshot === 'function' &&
      document.querySelectorAll('[data-video-isf-layer-id]').length === 6 &&
      document.querySelectorAll('.videoOutputRailButton[data-video-output-id]').length === 3
    )()`,
    "Operator VJ fixture surface",
  );
  await client.evaluate("document.documentElement.setAttribute('data-window-mode','fullscreen')");
  await installOperatorVjInvokeMock(client);
  await sleep(60);
}

async function runOperatorVjAcceptanceViewport(client, viewport, locale) {
  await prepareOperatorVjAcceptanceViewport(client, viewport, locale);
  await client.evaluate(`document.querySelector('[data-video-isf-layer-id="1"]')?.scrollIntoView({ block: 'nearest' })`);
  await sleep(40);
  const initial = await readOperatorVjState(client);
  const initialLayout = await measureOperatorVjLayout(client, 1);
  if (screenshotDir && shouldCaptureViewport(viewport)) {
    mkdirSync(screenshotDir, { recursive: true });
    const screenshot = await client.send("Page.captureScreenshot", { format: "png", fromSurface: true });
    writeFileSync(
      join(screenshotDir, `vj-operator-${locale}-${viewport.width}x${viewport.height}-initial.png`),
      screenshot.data,
      "base64",
    );
  }

  await clickVisibleSelector(client, '[data-video-isf-layer-id="1"] [data-video-isf-action="advanced"]');
  await waitForClientCondition(
    client,
    `(() => {
      const root = document.querySelector('[data-video-isf-layer-id="1"]');
      return document.querySelectorAll('.videoIsfAdvanced').length === 1 &&
        root?.querySelectorAll('.videoIsfAdvanced .videoIsfControl').length === 6 &&
        root?.querySelectorAll('.videoIsfAdvanced .videoIsfStackRow').length === 8 &&
        root?.querySelector('[data-video-isf-action="advanced"]')?.getAttribute('aria-expanded') === 'true';
    })()`,
    "Operator VJ layer 1 Advanced controls",
  );
  await client.evaluate(`document.querySelector('[data-video-isf-layer-id="1"] .videoIsfAdvanced')?.scrollIntoView({ block: 'nearest' })`);
  await sleep(500);
  const expanded = await readOperatorVjState(client);
  const expandedLayout = await measureOperatorVjLayout(client, 1);
  if (screenshotDir && shouldCaptureViewport(viewport)) {
    const screenshot = await client.send("Page.captureScreenshot", { format: "png", fromSurface: true });
    writeFileSync(
      join(screenshotDir, `vj-operator-${locale}-${viewport.width}x${viewport.height}-expanded.png`),
      screenshot.data,
      "base64",
    );
  }

  await clickVisibleSelector(
    client,
    '[data-video-isf-layer-id="1"] [data-video-isf-action="trigger-event"]',
  );
  await client.evaluate("document.querySelector('.appMenuButton')?.click()");
  await waitForClientCondition(
    client,
    `(() => {
      const root = document.querySelector('[data-video-isf-layer-id="1"]');
      const undo = document.querySelector('[aria-keyshortcuts^="Control+Z"]');
      const redo = document.querySelector('[aria-keyshortcuts^="Control+Y"]');
      return root?.getAttribute('aria-busy') === 'true' &&
        root.querySelector('[data-video-isf-action="trigger-event"]')?.disabled === true &&
        root.querySelector('[data-video-isf-stage-index="0"] [data-video-isf-action="move-down"]')?.disabled === true &&
        undo?.disabled === true && redo?.disabled === true &&
        window.__syndocalOperatorVjMock?.eventPulses?.[0]?.values?.join(',') === '1';
    })()`,
    "Operator VJ Event pulse busy interlock",
  );
  await client.evaluate(`(() => {
    document.querySelector('[data-video-isf-layer-id="1"] [data-video-isf-stage-index="0"] [data-video-isf-action="move-down"]')?.click();
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'z', ctrlKey: true, bubbles: true }));
  })()`);
  const pulseBusy = await readOperatorVjState(client);
  await waitForClientCondition(
    client,
    `(() => {
      const root = document.querySelector('[data-video-isf-layer-id="1"]');
      const mock = window.__syndocalOperatorVjMock;
      return root?.getAttribute('aria-busy') === 'false' &&
        root.querySelector('[data-video-isf-action="trigger-event"]')?.disabled === false &&
        document.querySelector('[aria-keyshortcuts^="Control+Z"]')?.disabled === false &&
        document.querySelector('[aria-keyshortcuts^="Control+Y"]')?.disabled === false &&
        mock?.eventPulses?.[0]?.values?.join(',') === '1,0' &&
        mock?.effectsByLayerId?.[1]?.controls?.find((control) => control.name === 'pulse')?.value?.[0] === 0;
    })()`,
    "Operator VJ Event pulse completion",
  );
  const pulseCompleted = await readOperatorVjState(client);

  await clickVisibleSelector(
    client,
    '[data-video-isf-layer-id="1"] [data-video-isf-stage-index="1"] [data-video-isf-action="select-stage"]',
  );
  await waitForClientCondition(
    client,
    `(() => {
      const root = document.querySelector('[data-video-isf-layer-id="1"]');
      return root?.querySelector('.videoIsfStackRow.selected')?.getAttribute('data-video-isf-stage-index') === '1' &&
        root?.querySelector('.videoIsfSelectedEditor')?.getAttribute('data-video-isf-editor-stage') === '1' &&
        root?.querySelectorAll('.videoIsfAdvanced .videoIsfControl').length === 1 &&
        (root?.querySelector('.videoIsfQuickStatus [data-no-localize]')?.textContent || '').trim() === 'Monochrome' &&
        root?.querySelector('[data-video-isf-action="bypass"]')?.getAttribute('aria-pressed') === 'false';
    })()`,
    "Operator VJ selected tail stage editor",
  );
  const selectedTail = await readOperatorVjState(client);

  await clickVisibleSelector(
    client,
    '[data-video-isf-layer-id="1"] [data-video-isf-stage-index="1"] [data-video-isf-action="toggle-stage"]',
  );
  await waitForClientCondition(
    client,
    `(() => {
      const root = document.querySelector('[data-video-isf-layer-id="1"]');
      const toggle = root?.querySelector('[data-video-isf-stage-index="1"] [data-video-isf-action="toggle-stage"]');
      return root?.querySelector('[data-video-isf-action="advanced"]')?.getAttribute('aria-expanded') === 'true' &&
        root.querySelector('.videoIsfStackRow.selected')?.getAttribute('data-video-isf-stage-index') === '1' &&
        toggle?.getAttribute('aria-pressed') === 'true' &&
        window.__syndocalOperatorVjMock?.snapshotReadCount >= 1 &&
        window.__syndocalOperatorVjMock?.diagnosticsReadCount >= 2;
    })()`,
    "Operator VJ Advanced mutation preserves state and publishes GPU error",
  );
  const enabledTail = await readOperatorVjState(client);

  await clickVisibleSelector(
    client,
    '[data-video-isf-layer-id="1"] [data-video-isf-stage-index="1"] [data-video-isf-action="toggle-stage"]',
  );
  await waitForClientCondition(
    client,
    `(() => {
      const root = document.querySelector('[data-video-isf-layer-id="1"]');
      const toggle = root?.querySelector('[data-video-isf-stage-index="1"] [data-video-isf-action="toggle-stage"]');
      return root?.querySelector('[data-video-isf-action="advanced"]')?.getAttribute('aria-expanded') === 'true' &&
        root.querySelector('.videoIsfStackRow.selected')?.getAttribute('data-video-isf-stage-index') === '1' &&
        toggle?.getAttribute('aria-pressed') === 'false' &&
        window.__syndocalOperatorVjMock?.snapshotReadCount >= 2 &&
        window.__syndocalOperatorVjMock?.diagnosticsReadCount >= 3;
    })()`,
    "Operator VJ Advanced bypass clears GPU error without remount",
  );
  const disabledTail = await readOperatorVjState(client);

  await clickVisibleSelector(
    client,
    '[data-video-isf-layer-id="1"] [data-video-isf-stage-index="2"] [data-video-isf-action="move-up"]',
  );
  await waitForClientCondition(
    client,
    `(() => {
      const root = document.querySelector('[data-video-isf-layer-id="1"]');
      const focused = document.activeElement;
      return [...root.querySelectorAll('.videoIsfStackRow .videoIsfStageSelect strong')].map((label) => label.textContent?.trim()).join(',') === 'Threshold,Invert,Monochrome,RGB Split,Mirror,Scanlines,Vignette,Posterize' &&
        root.querySelector('.videoIsfStackRow.selected')?.getAttribute('data-video-isf-stage-index') === '2' &&
        (root.querySelector('.videoIsfQuickStatus [data-no-localize]')?.textContent || '').trim() === 'Monochrome' &&
        focused?.getAttribute('data-video-isf-action') === 'move-up' &&
        focused?.closest('[data-video-isf-stage-index]')?.getAttribute('data-video-isf-stage-index') === '1' &&
        window.__syndocalOperatorVjMock?.snapshotReadCount === 3 &&
        window.__syndocalOperatorVjMock?.diagnosticsReadCount === 4;
    })()`,
    "Operator VJ crossing move preserves selected FX identity",
  );
  const crossedTail = await readOperatorVjState(client);

  await clickVisibleSelector(
    client,
    '[data-video-isf-layer-id="1"] [data-video-isf-stage-index="2"] [data-video-isf-action="move-up"]',
  );
  await waitForClientCondition(
    client,
    `(() => {
      const root = document.querySelector('[data-video-isf-layer-id="1"]');
      const focused = document.activeElement;
      return [...root.querySelectorAll('.videoIsfStackRow .videoIsfStageSelect strong')].map((label) => label.textContent?.trim()).join(',') === 'Threshold,Monochrome,Invert,RGB Split,Mirror,Scanlines,Vignette,Posterize' &&
        root.querySelector('.videoIsfStackRow.selected')?.getAttribute('data-video-isf-stage-index') === '1' &&
        (root.querySelector('.videoIsfQuickStatus [data-no-localize]')?.textContent || '').trim() === 'Monochrome' &&
        focused?.getAttribute('data-video-isf-action') === 'move-up' &&
        focused?.closest('[data-video-isf-stage-index]')?.getAttribute('data-video-isf-stage-index') === '1' &&
        window.__syndocalOperatorVjMock?.snapshotReadCount === 4 &&
        window.__syndocalOperatorVjMock?.diagnosticsReadCount === 5;
    })()`,
    "Operator VJ selected move preserves FX identity",
  );
  const movedTailRestored = await readOperatorVjState(client);

  await clickVisibleSelector(
    client,
    '[data-video-isf-layer-id="1"] [data-video-isf-stage-index="1"] [data-video-isf-action="remove"]',
  );
  await waitForClientCondition(
    client,
    `(() => {
      const root = document.querySelector('[data-video-isf-layer-id="1"]');
      return [...root.querySelectorAll('.videoIsfStackRow .videoIsfStageSelect strong')].map((label) => label.textContent?.trim()).join(',') === 'Threshold,Invert,RGB Split,Mirror,Scanlines,Vignette,Posterize' &&
        root.querySelector('.videoIsfStackRow.selected')?.getAttribute('data-video-isf-stage-index') === '1' &&
        (root.querySelector('.videoIsfQuickStatus [data-no-localize]')?.textContent || '').trim() === 'Invert' &&
        window.__syndocalOperatorVjMock?.snapshotReadCount === 5 &&
        window.__syndocalOperatorVjMock?.diagnosticsReadCount === 6;
    })()`,
    "Operator VJ selected removal chooses safe adjacent FX",
  );
  const removedTail = await readOperatorVjState(client);

  await clickVisibleSelector(
    client,
    '[data-video-isf-layer-id="1"] [data-video-isf-stage-index="0"] [data-video-isf-action="select-stage"]',
  );
  await waitForClientCondition(
    client,
    `(() => {
      const root = document.querySelector('[data-video-isf-layer-id="1"]');
      return root?.querySelector('.videoIsfStackRow.selected')?.getAttribute('data-video-isf-stage-index') === '0' &&
        root?.querySelector('.videoIsfSelectedEditor')?.getAttribute('data-video-isf-editor-stage') === '0' &&
        root?.querySelectorAll('.videoIsfAdvanced .videoIsfControl').length === 6 &&
        (root?.querySelector('.videoIsfQuickStatus [data-no-localize]')?.textContent || '').trim() === 'Threshold';
    })()`,
    "Operator VJ restored root stage editor",
  );
  const selectedRootRestored = await readOperatorVjState(client);

  await clickVisibleSelector(client, '[data-video-isf-layer-id="1"] [data-video-isf-action="advanced"]');
  await waitForClientCondition(
    client,
    `document.querySelectorAll('.videoIsfAdvanced').length === 0 &&
      document.querySelector('[data-video-isf-layer-id="1"] [data-video-isf-action="advanced"]')?.getAttribute('aria-expanded') === 'false'`,
    "Operator VJ Advanced collapse",
  );
  const collapsed = await readOperatorVjState(client);

  await clickVisibleSelector(client, '[data-video-isf-layer-id="1"] [data-video-isf-action="bypass"]');
  await waitForClientCondition(
    client,
    `(() => {
      const mock = window.__syndocalOperatorVjMock;
      return document.querySelector('[data-video-isf-layer-id="1"] [data-video-isf-action="bypass"]')?.getAttribute('aria-pressed') === 'false' &&
        mock?.snapshotReadCount === 6 && mock?.diagnosticsReadCount === 7 && mock?.effectsByLayerId?.[1]?.enabled === false;
    })()`,
    "Operator VJ transactional bypass snapshot",
  );
  const bypassed = await readOperatorVjState(client);

  const bankChanged = await client.evaluate(`(() => {
    const pager = document.querySelector('.videoLayerList.compact > .deckPager');
    const next = pager?.querySelector('button:last-of-type');
    if (!next || next.disabled) return false;
    next.click();
    return true;
  })()`);
  await waitForClientCondition(
    client,
    `JSON.stringify([...document.querySelectorAll('[data-video-isf-layer-id]')]
      .map((node) => Number(node.getAttribute('data-video-isf-layer-id')))) === '[7]'`,
    "Operator VJ terminal layer bank",
  );
  await client.evaluate(`document.querySelector('[data-video-isf-layer-id="7"]')?.scrollIntoView({ block: 'nearest' })`);
  await sleep(40);
  const terminalBank = await readOperatorVjState(client);
  const terminalBankLayout = await measureOperatorVjLayout(client, 7);

  await clickVisibleSelector(client, '.videoOutputRailButton[data-video-output-id="3"]');
  await waitForClientCondition(
    client,
    `(() => {
      const selected = document.querySelector('.videoOutputRailButton[data-video-output-id="3"]')?.getAttribute('aria-current') === 'true';
      const detail = document.querySelector('[data-video-output-detail-id="3"]');
      const detailLabel = (detail?.querySelector(':scope > div:first-child > strong')?.textContent || '').trim();
      const programLabel = (document.querySelector('[data-live-video-monitor="program"] header span')?.textContent || '').trim();
      const monitorSynced = window.__syndocalOperatorVjMock?.calls?.some((call) =>
        call.command === 'get_live_video_monitor_frame' && call.args?.monitorKind === 'program' && call.args?.outputId === 3
      );
      return selected && detailLabel === 'Stream Fill' && programLabel === 'Stream Fill' && monitorSynced;
    })()`,
    "Operator VJ output rail/detail/Program monitor synchronization",
  );
  const outputSelected = await readOperatorVjState(client);
  const outputLayout = await measureOperatorVjLayout(client, 7);
  const outerContainment = await measure(client, `operator-vj-${locale}-${viewport.width}x${viewport.height}`);

  const calls = bypassed.mock?.calls ?? [];
  const mutationIndex = calls.findIndex((call) =>
    call.command === "set_video_layer_isf_effect_enabled" && call.args?.stageIndex === 0
  );
  const beginIndex = calls.reduce((latest, call, index) =>
    call.command === "begin_project_transaction" && index < mutationIndex ? index : latest
  , -1);
  const commitIndex = calls.findIndex((call, index) =>
    index > mutationIndex && call.command === "commit_project_transaction"
  );
  const snapshotIndex = calls.findIndex((call, index) => index > commitIndex && call.command === "get_snapshot");
  const expectedHeading = locale === "ja" ? "レイヤー" : "Layers";
  const expectedAdvanced = locale === "ja" ? "詳細" : "Advanced";
  const expectedBypassed = locale === "ja" ? "バイパス" : "Bypassed";
  const maximumStackLabels = "Threshold,Monochrome,Invert,RGB Split,Mirror,Scanlines,Vignette,Posterize";
  const crossedStackLabels = "Threshold,Invert,Monochrome,RGB Split,Mirror,Scanlines,Vignette,Posterize";
  const removedStackLabels = "Threshold,Invert,RGB Split,Mirror,Scanlines,Vignette,Posterize";
  const expectedQuickBypassLabel = locale === "ja"
    ? "Threshold Pulse（レイヤー1）のThreshold FX有効状態"
    : "Threshold FX enabled for Threshold Pulse (layer 1)";
  const expectedTailBypassLabel = locale === "ja"
    ? "Threshold Pulse（レイヤー1）のMonochrome FX有効状態"
    : "Monochrome FX enabled for Threshold Pulse (layer 1)";
  const expectedControlLabels = locale === "ja"
    ? [
        "Threshold Pulse（レイヤー1）のlevel 成分1",
        "Threshold Pulse（レイヤー1）のThreshold FXのuseSourceAlpha",
        "Threshold Pulse（レイヤー1）のtoneCount 成分1",
        "Threshold Pulse（レイヤー1）のcenter 成分1",
        "Threshold Pulse（レイヤー1）のcenter 成分2",
        "Threshold Pulse（レイヤー1）のtint 成分1",
        "Threshold Pulse（レイヤー1）のtint 成分2",
        "Threshold Pulse（レイヤー1）のtint 成分3",
        "Threshold Pulse（レイヤー1）のtint 成分4",
        "Threshold Pulse（レイヤー1）のpulseをトリガー",
      ]
    : [
        "level component 1 for Threshold Pulse (layer 1)",
        "useSourceAlpha for Threshold FX on Threshold Pulse (layer 1)",
        "toneCount component 1 for Threshold Pulse (layer 1)",
        "center component 1 for Threshold Pulse (layer 1)",
        "center component 2 for Threshold Pulse (layer 1)",
        "tint component 1 for Threshold Pulse (layer 1)",
        "tint component 2 for Threshold Pulse (layer 1)",
        "tint component 3 for Threshold Pulse (layer 1)",
        "tint component 4 for Threshold Pulse (layer 1)",
        "Trigger pulse for Threshold Pulse (layer 1)",
      ];
  const expandedControlRows = expanded.layerOne?.controlRows ?? [];
  const expandedControlLabels = expandedControlRows.flatMap((row) => row.controls.map((control) => control.ariaLabel));
  const layoutPhases = [initialLayout, expandedLayout, terminalBankLayout, outputLayout];
  const checks = {
    fixtureAndLocale:
      initial.lang === locale && initial.windowMode === "fullscreen" &&
      initial.layerOne?.label === "Threshold" &&
      initial.outputButtons.map((output) => output.id).join(",") === "1,2,3",
    initialAdvancedUnmounted:
      initial.advancedDomCount === 0 && initial.layerOne?.advancedExpanded === "false",
    initialFxQuickRack:
      initial.layerOne?.bypassPressed === "true" && initial.layerOne?.bypassActive === true &&
      initial.layerOne?.bypassAriaLabel === expectedQuickBypassLabel &&
      initial.layerOne?.advancedText === expectedAdvanced &&
      initial.layerOne?.advancedAriaLabel.includes("Threshold Pulse") &&
      initial.layerOne?.advancedAriaLabel.includes("1") &&
      initial.layerOne?.builtinAriaLabel.includes("Threshold Pulse") &&
      initial.layerOne?.builtinAriaLabel.includes("1"),
    advancedLazyMount:
      expanded.advancedDomCount === 1 && expanded.layerOne?.controlCount === 6 &&
      expanded.layerOne?.advancedExpanded === "true" && expanded.layerOne?.stackRowCount === 8 &&
      expanded.layerOne?.stackLabels.join(",") === maximumStackLabels &&
      expanded.layerOne?.selectedStackIndex === 0,
    maximumStackAndControlKinds:
      expanded.layerOne?.builtinDisabled === true && expanded.layerOne?.addIsfDisabled === true &&
      expanded.layerOne?.stackCountText === "8/8 FX" &&
      expandedControlRows.map((row) => row.kind).join(",") === "Float,Bool,Long,Point2d,Color,Event" &&
      expandedControlRows.map((row) => row.name).join(",") === "level,useSourceAlpha,toneCount,center,tint,pulse" &&
      expandedControlRows.map((row) => row.controls.length).join(",") === "1,1,1,2,4,1" &&
      expandedControlRows.find((row) => row.kind === "Bool")?.controls[0]?.type === "checkbox" &&
      expandedControlRows.find((row) => row.kind === "Long")?.controls[0]?.step === "1" &&
      JSON.stringify(expandedControlLabels) === JSON.stringify(expectedControlLabels),
    eventPulseNonHistoricalInterlock:
      pulseBusy.mock?.eventPulses?.[0]?.values?.join(",") === "1" &&
      pulseBusy.mock?.historyStatus?.undo_depth === 4 && pulseBusy.mock?.historyStatus?.redo_depth === 2 &&
      pulseCompleted.mock?.eventPulses?.[0]?.values?.join(",") === "1,0" &&
      pulseCompleted.mock?.storedEventValue === 0 &&
      pulseCompleted.mock?.historyStatus?.undo_depth === 4 && pulseCompleted.mock?.historyStatus?.redo_depth === 2 &&
      pulseCompleted.mock?.calls.filter((call) => call.command === "pulse_video_layer_isf_event").length === 1 &&
      pulseCompleted.mock?.calls.filter((call) => call.command === "begin_project_transaction").length === 0 &&
      pulseCompleted.mock?.calls.filter((call) => call.command === "commit_project_transaction").length === 0 &&
      pulseCompleted.mock?.calls.filter((call) => call.command === "get_snapshot").length === 0 &&
      pulseCompleted.mock?.calls.filter((call) => call.command === "move_video_layer_isf_effect").length === 0,
    selectedStageEditorLazySwitch:
      selectedTail.layerOne?.label === "Monochrome" && selectedTail.layerOne?.controlCount === 1 &&
      selectedTail.layerOne?.selectedStackIndex === 1 && selectedTail.layerOne?.bypassPressed === "false" &&
      selectedTail.layerOne?.bypassAriaLabel === expectedTailBypassLabel &&
      selectedRootRestored.layerOne?.label === "Threshold" &&
      selectedRootRestored.layerOne?.controlCount === 6 &&
      selectedRootRestored.layerOne?.selectedStackIndex === 0 &&
      selectedRootRestored.layerOne?.bypassPressed === "true",
    advancedMutationStateFocusAndDiagnostics:
      enabledTail.layerOne?.advancedExpanded === "true" && enabledTail.layerOne?.selectedStackIndex === 1 &&
      enabledTail.layerOne?.label === "Monochrome" && enabledTail.layerOne?.focusedAction === "toggle-stage" &&
      enabledTail.layerOne?.focusedStageIndex === 1 && enabledTail.layerOne?.runtimeErrorCount === 1 &&
      disabledTail.layerOne?.advancedExpanded === "true" && disabledTail.layerOne?.selectedStackIndex === 1 &&
      disabledTail.layerOne?.label === "Monochrome" && disabledTail.layerOne?.focusedAction === "toggle-stage" &&
      disabledTail.layerOne?.focusedStageIndex === 1 && disabledTail.layerOne?.runtimeErrorCount === 0,
    moveRemoveSelectionIdentity:
      crossedTail.layerOne?.stackLabels.join(",") === crossedStackLabels &&
      crossedTail.layerOne?.selectedStackIndex === 2 && crossedTail.layerOne?.label === "Monochrome" &&
      movedTailRestored.layerOne?.stackLabels.join(",") === maximumStackLabels &&
      movedTailRestored.layerOne?.selectedStackIndex === 1 && movedTailRestored.layerOne?.label === "Monochrome" &&
      removedTail.layerOne?.stackLabels.join(",") === removedStackLabels &&
      removedTail.layerOne?.selectedStackIndex === 1 && removedTail.layerOne?.label === "Invert" &&
      removedTail.layerOne?.builtinDisabled === false && removedTail.layerOne?.addIsfDisabled === false &&
      removedTail.layerOne?.stackCountText === "7/8 FX",
    advancedUnmountsOnClose:
      collapsed.advancedDomCount === 0 && collapsed.layerOne?.advancedExpanded === "false",
    bypassTransactionSnapshotAndAria:
      beginIndex >= 0 && mutationIndex > beginIndex && commitIndex > mutationIndex && snapshotIndex > commitIndex &&
      calls.filter((call) => call.command === "begin_project_transaction").length === 6 &&
      calls.filter((call) => call.command === "set_video_layer_isf_effect_enabled").length === 3 &&
      calls.filter((call) => call.command === "move_video_layer_isf_effect").length === 2 &&
      calls.filter((call) => call.command === "remove_video_layer_isf_effect").length === 1 &&
      calls.filter((call) => call.command === "commit_project_transaction").length === 6 &&
      calls.filter((call) => call.command === "cancel_project_transaction").length === 0 &&
      calls.filter((call) => call.command === "get_snapshot").length === 6 &&
      calls[mutationIndex]?.args?.layerId === 1 && calls[mutationIndex]?.args?.stageIndex === 0 &&
      calls[mutationIndex]?.args?.enabled === false &&
      bypassed.mock?.snapshotReadCount === 6 && bypassed.mock?.diagnosticsReadCount === 7 &&
      bypassed.mock?.historyStatus?.undo_depth === 10 && bypassed.mock?.historyStatus?.redo_depth === 0 &&
      bypassed.mock?.storedLayerOneEnabled === false &&
      bypassed.mock?.lastLayerOneEnabled === false && bypassed.layerOne?.bypassPressed === "false" &&
      bypassed.layerOne?.bypassActive === false && bypassed.layerOne?.bypassText === expectedBypassed &&
      bypassed.layerOne?.statusText.includes(expectedBypassed),
    recordingTelemetryVisible:
      initialLayout.lowHeightClipGrid.recordingTelemetryVisible &&
      initialLayout.lowHeightClipGrid.recordingTelemetryText.includes("2 dropped"),
    lowHeightClipGridPlacement:
      viewport.height > 740 || (
        initialLayout.lowHeightClipGrid.audioSyncRow === "3" &&
        initialLayout.lowHeightClipGrid.clipGridRow === "4"
      ),
    sixPlusOneLayerBanks:
      initial.layerIds.join(",") === "1,2,3,4,5,6" && initial.pagerHeading === expectedHeading &&
      initial.pagerText === "1-6 / 7" && bankChanged && terminalBank.layerIds.join(",") === "7" &&
      terminalBank.pagerText === "7-7 / 7" && terminalBank.advancedDomCount === 0,
    outputRailStates:
      JSON.stringify(initial.outputButtons.map((output) => [output.id, output.state])) ===
        JSON.stringify([[1, "state-live"], [2, "state-off"], [3, "state-blackout"]]) &&
      initial.selectedOutputId === 1 && initial.visibleOutputDetails.length === 1 &&
      initial.visibleOutputDetails[0]?.id === 1 && initial.visibleOutputDetails[0]?.label === "Main LED" &&
      initial.programLabel === "Main LED",
    outputRailDetailProgramMonitorSync:
      outputSelected.selectedOutputId === 3 && outputSelected.visibleOutputDetails.length === 1 &&
      outputSelected.visibleOutputDetails[0]?.id === 3 && outputSelected.visibleOutputDetails[0]?.label === "Stream Fill" &&
      outputSelected.programLabel === "Stream Fill" && outputSelected.mock?.calls.some((call) =>
        call.command === "get_live_video_monitor_frame" && call.args?.monitorKind === "program" && call.args?.outputId === 3
      ),
    internalOverflowAndRectContainment:
      layoutPhases.every((layout) => layout.unsafeOverflowCount === 0 && layout.outsideRectCount === 0) &&
      isContained(outerContainment),
  };
  const failedChecks = Object.entries(checks).filter(([, passed]) => !passed).map(([name]) => name);
  return {
    passed: failedChecks.length === 0,
    viewport,
    locale,
    checks,
    failedChecks,
    initial,
    expanded,
    pulseBusy,
    pulseCompleted,
    selectedTail,
    enabledTail,
    disabledTail,
    crossedTail,
    movedTailRestored,
    removedTail,
    selectedRootRestored,
    bypassed,
    terminalBank,
    outputSelected,
    layoutPhases,
    outerContainment,
  };
}

async function prepareAutoVjAcceptanceViewport(client, viewport, locale) {
  await client.send("Emulation.setDeviceMetricsOverride", {
    width: viewport.width,
    height: viewport.height,
    deviceScaleFactor: 1,
    mobile: false,
  });
  const url = fixtureUrl("auto-vj");
  await client.send("Page.navigate", { url });
  await waitForApp(client);
  await client.evaluate(
    "window.localStorage.setItem('syndocal.uiLocale.v1'," + JSON.stringify(locale) + ")",
  );
  await client.send("Page.navigate", { url });
  await waitForApp(client);
  await waitForClientCondition(
    client,
    "Boolean(document.querySelector('.videoMixerClipPane > .autoVjStrip'))",
    "Auto VJ fixture strip",
  );
  await openMixerDrawer(client, "auto-vj", ".videoMixerClipPane > .autoVjStrip");
  await installAutoVjInvokeMock(client);
}

async function runAutoVjAcceptanceViewport(client, viewport, locale) {
  await prepareAutoVjAcceptanceViewport(client, viewport, locale);
  const initial = await readAutoVjState(client);

  const programAudioVolumeChanged = await setProgramAudioMonitorVolume(client, 0.65);
  await waitForClientCondition(
    client,
    `(() => {
      const calls = window.__syndocalAutoVjMock?.calls?.filter(
        (call) => call.command === 'set_program_audio_handoff_config',
      ) ?? [];
      return calls.length === 1 && JSON.stringify(calls[0].args) === JSON.stringify({
        enabled: true,
        volume: 0.65,
        deviceName: null,
      });
    })()`,
    "Program audio default-device config registration",
  );
  const programAudioOutputRefreshClicked = await refreshProgramAudioOutputDevices(client);
  await waitForClientCondition(
    client,
    `(() => [...document.querySelectorAll('.videoAudioDeviceField select option')]
      .some((option) => option.value === 'Viewport ASIO Program Output'))()`,
    "Program audio output-device refresh",
  );
  const programAudioOutputChanged = await setProgramAudioOutputDevice(
    client,
    "Viewport ASIO Program Output",
  );
  await waitForClientCondition(
    client,
    `(() => {
      const calls = window.__syndocalAutoVjMock?.calls?.filter(
        (call) => call.command === 'set_program_audio_handoff_config',
      ) ?? [];
      return calls.length === 2 && JSON.stringify(calls[1].args) === JSON.stringify({
        enabled: true,
        volume: 0.65,
        deviceName: 'Viewport ASIO Program Output',
      });
    })()`,
    "Program audio explicit-device config registration",
  );
  const programAudioDisabled = await setProgramAudioEnabled(client, false);
  await waitForClientCondition(
    client,
    `(() => {
      const calls = window.__syndocalAutoVjMock?.calls?.filter(
        (call) => call.command === 'set_program_audio_handoff_config',
      ) ?? [];
      return calls.length === 3 && calls[2].args?.enabled === false;
    })()`,
    "Program audio disabled config registration",
  );
  const programAudioEnabled = await setProgramAudioEnabled(client, true);
  await waitForClientCondition(
    client,
    `(() => {
      const mock = window.__syndocalAutoVjMock;
      const calls = mock?.calls?.filter(
        (call) => call.command === 'set_program_audio_handoff_config',
      ) ?? [];
      return calls.length === 4 && calls[3].args?.enabled === true &&
        mock?.programAudioHandoffConfig?.enabled === true;
    })()`,
    "Program audio enabled config registration",
  );

  const rhythmChanged = await setAutoVjRhythmSource(client, "LiveAudio");
  await waitForClientCondition(
    client,
    "(() => { const strip = document.querySelector('.videoMixerClipPane > .autoVjStrip'); return strip?.getAttribute('data-rhythm-source') === 'LiveAudio' && strip.querySelectorAll('.autoVjEligiblePicker input[type=checkbox]:checked').length === 3; })()",
    "Auto VJ candidate defaulting and Live Input configuration",
  );
  await sleep(50);
  const configured = await readAutoVjState(client);

  const armClicked = await clickAutoVjButton(client, 0);
  await waitForClientCondition(
    client,
    "document.querySelector('.videoMixerClipPane > .autoVjStrip')?.getAttribute('data-mode') === 'Armed'",
    "Auto VJ Arm state",
  );
  await sleep(50);
  const armed = await readAutoVjState(client);

  const holdClicked = await clickAutoVjButton(client, 1);
  await waitForClientCondition(
    client,
    "document.querySelector('.videoMixerClipPane > .autoVjStrip')?.getAttribute('data-mode') === 'Hold'",
    "Auto VJ Hold state",
  );
  await sleep(50);
  const held = await readAutoVjState(client);

  const resumeClicked = await clickAutoVjButton(client, 1);
  await waitForClientCondition(
    client,
    "(() => { const mock = window.__syndocalAutoVjMock; return document.querySelector('.videoMixerClipPane > .autoVjStrip')?.getAttribute('data-mode') === 'Running' && mock?.programAudioHandoffConfig?.enabled === true; })()",
    "Auto VJ Resume action with backend-owned Program audio handoff",
  );
  await sleep(50);
  const running = await readAutoVjState(client);
  const runningContainment = await measure(
    client,
    `auto-vj-${locale}-${viewport.width}x${viewport.height}`,
  );

  const disarmClicked = await clickAutoVjButton(client, 0);
  await waitForClientCondition(
    client,
    "document.querySelector('.videoMixerClipPane > .autoVjStrip')?.getAttribute('data-mode') === 'Off'",
    "Auto VJ Disarm state",
  );
  await sleep(50);
  const disarmed = await readAutoVjState(client);
  const calls = await client.evaluate(
    "JSON.parse(JSON.stringify(window.__syndocalAutoVjMock?.calls ?? []))",
  );
  const configCallIndex = calls.findIndex((call) => call.command === "set_auto_vj_config");
  const armCallIndex = calls.findIndex(
    (call) => call.command === "set_auto_vj_armed" && call.args?.armed === true,
  );
  const configCall = calls[configCallIndex];
  const expected = locale === "ja"
    ? {
        initialMode: "OFF",
        armedMode: "待機",
        holdMode: "ホールド",
        runningMode: "実行中",
        initialArm: "有効化",
        disarm: "解除",
        hold: "ホールド",
        resume: "再開",
        emptyClips: "クリップ 未設定",
        configuredClips: "クリップ 3/3",
        lockedTitle: "設定を編集するにはAuto VJを解除してください",
        action: "#1 · Output · 入力パルス 4",
        progress: "入力パルス 4 · ステップ 0/4 · FSEQ 77",
      }
    : {
        initialMode: "Off",
        armedMode: "Armed",
        holdMode: "Hold",
        runningMode: "Running",
        initialArm: "Arm",
        disarm: "Disarm",
        hold: "Hold",
        resume: "Resume",
        emptyClips: "Clips Not set",
        configuredClips: "Clips 3/3",
        lockedTitle: "Disarm Auto VJ to edit configuration",
        action: "#1 · Output · INPUT PULSE 4",
        progress: "INPUT PULSE 4 · STEP 0/4 · FSEQ 77",
      };
  const stopAudioCalls = calls.filter((call) => call.command === "stop_video_layer_audio_monitor");
  const playAudioCalls = calls.filter((call) => call.command === "play_video_layer_audio_monitor");
  const programAudioHandoffCalls = calls.filter(
    (call) => call.command === "set_program_audio_handoff_config",
  );
  const expectedProgramAudioHandoffConfigs = [
    { enabled: true, volume: 0.65, deviceName: null },
    { enabled: true, volume: 0.65, deviceName: "Viewport ASIO Program Output" },
    { enabled: false, volume: 0.65, deviceName: "Viewport ASIO Program Output" },
    { enabled: true, volume: 0.65, deviceName: "Viewport ASIO Program Output" },
  ];
  const checks = {
    emptyCandidateState:
      initial?.mode === "Off" &&
      initial?.modeText === expected.initialMode &&
      initial?.armText === expected.initialArm &&
      initial?.armDisabled === false &&
      initial?.holdDisabled === true &&
      initial?.clipSummary === expected.emptyClips &&
      initial?.selectedClipCount === 0 &&
      initial?.disabledConfigControlCount === 0,
    rhythmSourcesExplicit:
      rhythmChanged &&
      JSON.stringify(initial?.rhythmOptions) === JSON.stringify(["Clock", "LiveAudio"]) &&
      initial?.rhythmValue === "Clock" &&
      configured?.rhythmValue === "LiveAudio" &&
      configured?.rhythmSource === "LiveAudio",
    emptyCandidatesDefaultBeforeConfig:
      configCallIndex >= 0 &&
      armCallIndex > configCallIndex &&
      JSON.stringify(configCall?.args?.config?.eligible_layer_ids) === JSON.stringify([1, 2, 3]) &&
      configCall?.args?.config?.rhythm_source === "LiveAudio" &&
      configured?.clipSummary === expected.configuredClips &&
      configured?.selectedClipCount === 3,
    armedStateLocksConfiguration:
      armClicked &&
      armed?.mode === "Armed" &&
      armed?.modeText === expected.armedMode &&
      armed?.armText === expected.disarm &&
      armed?.armPressed === "true" &&
      armed?.holdText === expected.hold &&
      armed?.holdDisabled === false &&
      armed?.configControlCount > 0 &&
      armed?.disabledConfigControlCount === armed?.configControlCount &&
      armed?.clipSummaryTitle === expected.lockedTitle,
    holdState:
      holdClicked &&
      held?.mode === "Hold" &&
      held?.modeText === expected.holdMode &&
      held?.holdText === expected.resume &&
      held?.holdPressed === "true",
    resumeRunsLiveInputAction:
      resumeClicked &&
      running?.mode === "Running" &&
      running?.modeText === expected.runningMode &&
      running?.holdText === expected.hold &&
      running?.holdPressed === "false" &&
      running?.readout === expected.action &&
      running?.secondaryReadout === expected.progress,
    userLayerLabelBoundary:
      running?.readout?.includes("Output") &&
      !running?.readout?.includes("出力"),
    programAudioSettingsRegisteredExactlyOnce:
      programAudioVolumeChanged &&
      programAudioOutputRefreshClicked &&
      programAudioOutputChanged &&
      programAudioDisabled &&
      programAudioEnabled &&
      JSON.stringify(programAudioHandoffCalls.map((call) => call.args)) ===
        JSON.stringify(expectedProgramAudioHandoffConfigs),
    programAudioPlaybackOwnedByBackend:
      stopAudioCalls.length === 0 &&
      playAudioCalls.length === 0,
    disarmedStateUnlocksConfiguration:
      disarmClicked &&
      disarmed?.mode === "Off" &&
      disarmed?.modeText === expected.initialMode &&
      disarmed?.armText === expected.initialArm &&
      disarmed?.holdDisabled === true &&
      disarmed?.disabledConfigControlCount === 0,
    stableAccessibleMetersAndBands:
      running?.accessibleMeterCount === 3 &&
      running?.visualBandCount === 16 &&
      running?.statusCount === 1,
    fullWindowContainment:
      running?.horizontalOverflowPx === 0 &&
      running?.outsideViewport === false &&
      isContained(runningContainment),
    commandCounts:
      calls.filter((call) => call.command === "set_auto_vj_config").length === 1 &&
      calls.filter((call) => call.command === "set_auto_vj_armed").length === 2 &&
      calls.filter((call) => call.command === "set_auto_vj_hold").length === 2 &&
      calls.filter((call) => call.command === "list_audio_output_devices").length === 1 &&
      programAudioHandoffCalls.length === expectedProgramAudioHandoffConfigs.length,
  };
  const failedChecks = Object.entries(checks)
    .filter(([, passed]) => !passed)
    .map(([name]) => name);
  return {
    passed: failedChecks.length === 0,
    locale,
    viewport,
    checks,
    failedChecks,
    initial,
    configured,
    armed,
    held,
    running,
    disarmed,
    calls,
    runningContainment,
  };
}

async function runViewport(client, viewport) {
  traceViewport(`start ${viewport.width}x${viewport.height}`);
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
  traceViewport(`keyboard control measured ${viewport.width}x${viewport.height}`);
  await pressKey(client, "F1");
  await sleep(80);
  await clickByText(client, "Mapping");
  await clickByText(client, "Stage Map");
  await sleep(120);
  results.push(await measure(client, `setup-mapping-keyboard-${viewport.width}x${viewport.height}`));
  await client.evaluate(`document.querySelector('button[aria-label="Keyboard shortcut help"]')?.click()`);
  await sleep(120);
  results.push(await measure(client, `mapping-hotkey-help-${viewport.width}x${viewport.height}`));
  traceViewport(`mapping help measured ${viewport.width}x${viewport.height}`);
  await pressKey(client, "Escape", "Escape");
  await sleep(80);
  await pressKey(client, "F3");
  await sleep(120);
  await checkEditableTouchSurface(client, "Default Desk", true);
  results.push(await measure(client, `touch-keyboard-${viewport.width}x${viewport.height}`));
  traceViewport(`touch keyboard measured ${viewport.width}x${viewport.height}`);
  await pressKey(client, "F1");
  await sleep(80);
  await clickVisibleByText(client, ".appMenuButton", "...");
  await sleep(120);
  results.push(await measure(client, `project-menu-${viewport.width}x${viewport.height}`));
  traceViewport(`project menu measured ${viewport.width}x${viewport.height}`);
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
    await clickVisibleByText(client, ".appMenuButton", "...");
    await pressKey(client, "F2");
    await sleep(120);
    await clickByText(client, "ミキサー");
    await sleep(120);
    const japaneseLiveAudioAcceptance = await runLiveAudioAcceptance(
      client,
      "ja",
      `localization-ja-live-audio-${viewport.width}x${viewport.height}`,
    );
    results.push(japaneseLiveAudioAcceptance.liveContainment);
    await client.evaluate(`window.localStorage.setItem('syndocal.uiLocale.v1', 'en')`);
    await client.send("Page.navigate", { url: appUrl });
    await waitForApp(client);
  }
  await clickByText(client, "Setup");
  let setupMappingContainment = null;
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
    const setupContainment = await measure(client, `setup-${setupTab.id}-${viewport.width}x${viewport.height}`);
    results.push(setupContainment);
    if (setupTab.id === "mapping") setupMappingContainment = setupContainment;
    traceViewport(`setup ${setupTab.id} measured ${viewport.width}x${viewport.height}`);
  }
  await clickByText(client, "Setup");
  await clickByText(client, "Mapping");
  await clickByText(client, "Stage Map");
  await sleep(120);
  const persistentSetupBefore = await measurePersistentBand(client, `persistent-band-setup-before-${viewport.width}x${viewport.height}`);
  await clickByText(client, "Control");
  await clickByText(client, "Live Edit");
  await sleep(120);
  const persistentControl = await measurePersistentBand(client, `persistent-band-control-${viewport.width}x${viewport.height}`);
  await clickByText(client, "Setup");
  await clickByText(client, "Mapping");
  await clickByText(client, "Stage Map");
  await sleep(120);
  const persistentSetupAfter = await measurePersistentBand(client, `persistent-band-setup-after-${viewport.width}x${viewport.height}`);
  const persistentBandComparison = comparePersistentBandMeasurements(
    persistentSetupBefore,
    persistentControl,
    persistentSetupAfter,
  );
  const timelinePaneExpansion = await runTimelinePaneExpansionCheck(client, viewport);
  const layeredTimelineDesk = await runLayeredTimelineDeskCheck(client, viewport);
  results.push({
    ...(setupMappingContainment ?? {}),
    ...persistentSetupAfter,
    label: `persistent-band-invariance-${viewport.width}x${viewport.height}`,
    persistentBandInvariant: persistentBandComparison.invariant,
    persistentBandRectsByWorkspace: persistentBandComparison.rectsByWorkspace,
    persistentBandRectDeltas: persistentBandComparison.deltas,
    timelinePaneExpansion,
    layeredTimelineDesk,
  });
  traceViewport(`persistent band compared ${viewport.width}x${viewport.height}`);
  await clickByText(client, "Setup");
  await clickByText(client, "Mapping");
  await clickByText(client, "Stage Map");
  await sleep(120);
  await clickByText(client, "Pick Visible");
  await sleep(120);
  await clickByText(client, "Wave Draft");
  await sleep(180);
  results.push(await measure(client, `mapping-wave-draft-${viewport.width}x${viewport.height}`));
  traceViewport(`mapping wave draft measured ${viewport.width}x${viewport.height}`);
  await clickByText(client, "Control");
  for (const controlTab of controlTabs) {
    await clickByText(client, controlTab.label);
    await sleep(320);
    if (shouldCaptureViewport(viewport)) {
      mkdirSync(screenshotDir, { recursive: true });
      const screenshot = await client.send("Page.captureScreenshot", { format: "png", fromSurface: true });
      writeFileSync(join(screenshotDir, `control-${controlTab.id}-${viewport.width}x${viewport.height}.png`), screenshot.data, "base64");
    }
    if (controlTab.id === "live") {
      // T3: the Finder+inspector is the default editor view; these control-live
      // expectations describe the legacy paged list, so switch to it first.
      await client.evaluate(`document.querySelector('[data-scene-block-view-toggle="list"]')?.click()`);
      await sleep(120);
    }
    results.push(await measure(client, `control-${controlTab.id}-${viewport.width}x${viewport.height}`));
    traceViewport(`control ${controlTab.id} measured ${viewport.width}x${viewport.height}`);
    if (controlTab.id === "mixer") {
      const liveAudioAcceptance = await runLiveAudioAcceptance(
        client,
        "en",
        `control-mixer-live-${viewport.width}x${viewport.height}`,
      );
      results.push(liveAudioAcceptance.liveContainment);
    }
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
      await clickVisibleByText(client, ".effectFamilyChooser button", "COLOR FX");
      await sleep(80);
      results.push(await measure(client, `control-edit-effects-color-family-${viewport.width}x${viewport.height}`));
      await clickVisibleByText(client, ".effectFamilyChooser button", "VALUE FX");
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
      await clickVisibleByText(client, ".liveDeskViewToggle button", "Matrix");
      await clickVisibleSelector(client, "[data-scene-matrix-edit-cue]");
      await sleep(120);
      results.push(await measure(client, `control-live-cues-${viewport.width}x${viewport.height}`));
      await client.evaluate(`(() => {
        const toggle = document.querySelector('.cuePanelEditToggle');
        if (toggle instanceof HTMLButtonElement && toggle.getAttribute('aria-expanded') !== 'true') toggle.click();
      })()`);
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
  await checkEditableTouchSurface(client, "Default Desk", true);
  if (shouldCaptureViewport(viewport)) {
    mkdirSync(screenshotDir, { recursive: true });
    const screenshot = await client.send("Page.captureScreenshot", { format: "png", fromSurface: true });
    writeFileSync(join(screenshotDir, `touch-${viewport.width}x${viewport.height}.png`), screenshot.data, "base64");
  }
  results.push(await measure(client, `touch-${viewport.width}x${viewport.height}`));
  traceViewport(`complete ${viewport.width}x${viewport.height}`);
  return results;
}

async function runComposedTouchViewport(client, viewport) {
  await client.send("Emulation.setDeviceMetricsOverride", {
    width: viewport.width,
    height: viewport.height,
    deviceScaleFactor: 1,
    mobile: false,
  });
  await client.send("Page.navigate", { url: fixtureUrl("touch-composed") });
  await waitForApp(client);
  await clickByText(client, "Touch");
  await sleep(180);
  await checkEditableTouchSurface(client, "Viewport Touch", false);
  return measure(client, `touch-composed-${viewport.width}x${viewport.height}`);
}

async function checkPatchHighAddressAction(client) {
  return await client.evaluate(`(async () => {
    const grid = document.querySelector('.dmxAddressGrid');
    const cell497 = grid?.querySelector('[data-dmx-address="497"]');
    const cell25 = grid?.querySelector('[data-dmx-address="25"]');
    const addressInput = [...document.querySelectorAll('label')]
      .find((label) => (label.childNodes[0]?.textContent || '').trim() === 'Address')
      ?.querySelector('input');
    if (!(grid instanceof HTMLElement) || !(cell497 instanceof HTMLButtonElement) || !(addressInput instanceof HTMLInputElement)) {
      return { passed: false, reason: 'missing grid, A497, or Address input' };
    }
    grid.scrollTop = grid.scrollHeight;
    await new Promise((resolveFrame) => requestAnimationFrame(resolveFrame));
    const gridRect = grid.getBoundingClientRect();
    const cellRect = cell497.getBoundingClientRect();
    const highCellReachable =
      cellRect.left >= gridRect.left - 1 && cellRect.right <= gridRect.right + 1 &&
      cellRect.top >= gridRect.top - 1 && cellRect.bottom <= gridRect.bottom + 1;
    cell497.click();
    await new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame)));
    const plannedHighAddresses = [...grid.querySelectorAll('.dmxAddressCell.planned')]
      .map((cell) => Number(cell.getAttribute('data-dmx-address')))
      .filter((address) => Number.isFinite(address) && address >= 497)
      .sort((left, right) => left - right);
    const app = document.querySelector('.app');
    const layout = document.querySelector('.layout');
    const desk = grid.closest('.setupPatchAddressDesk');
    const outerScrollZero =
      window.scrollX === 0 && window.scrollY === 0 &&
      document.documentElement.scrollLeft === 0 && document.documentElement.scrollTop === 0 &&
      document.body.scrollLeft === 0 && document.body.scrollTop === 0 &&
      (!app || (app.scrollLeft === 0 && app.scrollTop === 0)) &&
      (!layout || (layout.scrollLeft === 0 && layout.scrollTop === 0)) &&
      (!desk || (desk.scrollLeft === 0 && desk.scrollTop === 0));
    const addressValue = addressInput.value;
    const readout = (document.querySelector('.dmxPatchAddressReadout')?.textContent || '').replace(/\s+/g, ' ').trim();
    cell25?.click();
    grid.scrollTop = 0;
    await new Promise((resolveFrame) => requestAnimationFrame(resolveFrame));
    const passed =
      highCellReachable &&
      addressValue === '497' &&
      plannedHighAddresses.length === 16 &&
      plannedHighAddresses.every((address, index) => address === 497 + index) &&
      readout.endsWith('A497') &&
      outerScrollZero;
    return {
      passed,
      highCellReachable,
      addressValue,
      plannedHighAddresses,
      readout,
      outerScrollZero,
    };
  })()`);
}

async function runPatchViewport(client, viewport) {
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
  await clickByText(client, "Setup");
  await clickByText(client, "Lighting");
  await clickByText(client, "Patch");
  await sleep(180);
  if (shouldCaptureViewport(viewport)) {
    mkdirSync(screenshotDir, { recursive: true });
    const screenshot = await client.send("Page.captureScreenshot", { format: "png", fromSurface: true });
    writeFileSync(join(screenshotDir, `patch-continuous-${viewport.width}x${viewport.height}.png`), screenshot.data, "base64");
  }
  const containment = await measure(client, `setup-patch-${viewport.width}x${viewport.height}`);
  const highAddressAction = await checkPatchHighAddressAction(client);
  const checks = {
    contained: isContained(containment),
    setupSurface: hasExpectedSetupSurface(containment),
    continuousGrid: hasExpectedContinuousPatchGrid(containment),
    highAddressAction: highAddressAction.passed === true,
  };
  const failedChecks = Object.entries(checks).filter(([, passed]) => !passed).map(([name]) => name);
  return {
    passed: failedChecks.length === 0,
    label: `patch-continuous-${viewport.width}x${viewport.height}`,
    checks,
    failedChecks,
    containment,
    highAddressAction,
  };
}

const workspaceSplitDefaults = {
  top: 0.58,
  lower: 0.44,
};

const workspaceSplitMinimums = {
  upper: 280,
  lower: 310,
  lowerLeft: 430,
  lowerRight: 480,
};

async function readWorkspaceSplitState(client) {
  return evaluatePageFunction(client, () => {
    const visibleElement = (selector) => [...document.querySelectorAll(selector)].find((element) => {
      const box = element.getBoundingClientRect();
      const style = getComputedStyle(element);
      return box.width > 0 && box.height > 0 && style.display !== "none" && style.visibility !== "hidden";
    }) ?? null;
    const rect = (selector) => {
      const element = visibleElement(selector);
      if (!element) return null;
      const box = element.getBoundingClientRect();
      const precision = (value) => Math.round(value * 100) / 100;
      return {
        x: precision(box.x),
        y: precision(box.y),
        right: precision(box.right),
        bottom: precision(box.bottom),
        width: precision(box.width),
        height: precision(box.height),
      };
    };
    const splitterInfo = (name) => {
      const selector = `[data-workspace-splitter="${name}"]`;
      const element = visibleElement(selector);
      if (!element) return null;
      return {
        rect: rect(selector),
        role: element.getAttribute("role") ?? "",
        orientation: element.getAttribute("aria-orientation") ?? "",
        label: element.getAttribute("aria-label") ?? "",
        minimum: Number(element.getAttribute("aria-valuemin")),
        maximum: Number(element.getAttribute("aria-valuemax")),
        current: Number(element.getAttribute("aria-valuenow")),
        tabIndex: element.tabIndex,
        dragging: element.getAttribute("data-dragging") === "true",
      };
    };
    const renderedRatio = (root, splitter, axis) => {
      if (!root || !splitter?.rect) return Number.NaN;
      const rootBox = root.getBoundingClientRect();
      const splitterSize = axis === "horizontal" ? splitter.rect.height : splitter.rect.width;
      const usable = (axis === "horizontal" ? rootBox.height : rootBox.width) - splitterSize;
      const first = axis === "horizontal"
        ? splitter.rect.y - rootBox.y
        : splitter.rect.x - rootBox.x;
      return usable > 0 ? first / usable : Number.NaN;
    };
    const app = document.querySelector(".app");
    const layout = document.querySelector('[data-workspace-split-root="true"]');
    const rootRect = rect('[data-workspace-split-root="true"]');
    const lowerRect = rect('[data-workspace-pane="lower"]');
    const leftRect = rect('[data-workspace-pane="lower-left"]');
    const rightRect = rect('[data-workspace-pane="lower-right"]');
    const groupRect = rect('[data-persistent-band-part="groups"]');
    const drawer = document.querySelector('[data-workspace-selection-drawer]');
    const drawerRect = rect('[data-workspace-selection-drawer][open]');
    const drawerToggle = document.querySelector('[data-workspace-selection-drawer-toggle]');
    const topSplitter = splitterInfo("upper-lower");
    const lowerSplitter = splitterInfo("lower-left-right");
    const livePanel = document.querySelector('.liveControlPanel');
    const liveStatusToggle = document.querySelector('[data-live-status-toggle]');
    const liveStatusRect = rect('.liveControlPanel > .liveStatusGrid');
    const sceneMatrixRect = rect('.liveControlPanel > .sceneMatrixPanel');
    const liveFadeRect = rect('.liveControlPanel > .liveFadeMeter');
    const liveMasterRect = rect('.liveControlPanel > .liveMasterGrid');
    const visibleStatusItems = [...document.querySelectorAll('.liveControlPanel > .liveStatusGrid > .liveStatusItem')]
      .filter((element) => {
        const box = element.getBoundingClientRect();
        const style = getComputedStyle(element);
        return box.width > 0 && box.height > 0 && style.display !== 'none' && style.visibility !== 'hidden';
      });
    const visibleTransportButtons = [...document.querySelectorAll('.liveControlPanel > .liveTransportGrid > button')]
      .filter((element) => {
        const box = element.getBoundingClientRect();
        const style = getComputedStyle(element);
        return box.width > 0 && box.height > 0 && style.display !== 'none' && style.visibility !== 'hidden';
      });
    const transportRows = new Set(visibleTransportButtons.map((element) => Math.round(element.getBoundingClientRect().top)));
    let storage = null;
    const storageRaw = window.localStorage.getItem("syndocal.workspaceLayout.v1") ?? "";
    try { storage = storageRaw ? JSON.parse(storageRaw) : null; } catch { storage = null; }
    const documentElement = document.documentElement;
    const body = document.body;
    return {
      rootRect,
      lowerRect,
      leftRect,
      rightRect,
      groupRect,
      drawerRect,
      topSplitter,
      lowerSplitter,
      visibleSplitterCount: [topSplitter, lowerSplitter].filter(Boolean).length,
      upperHeight: rootRect && topSplitter?.rect ? topSplitter.rect.y - rootRect.y : 0,
      lowerHeight: rootRect && topSplitter?.rect ? rootRect.bottom - topSplitter.rect.bottom : 0,
      renderedTopRatio: renderedRatio(layout, topSplitter, "horizontal"),
      renderedLowerRatio: renderedRatio(visibleElement('[data-workspace-pane="lower"]'), lowerSplitter, "vertical"),
      drawerOpen: drawer instanceof HTMLDetailsElement && drawer.open,
      drawerToggleNamed: Boolean((drawerToggle?.getAttribute("aria-label") || drawerToggle?.textContent || "").trim()),
      drawerExpandedMatches:
        drawer instanceof HTMLDetailsElement &&
        drawerToggle?.getAttribute("aria-expanded") === String(drawer.open),
      drawerContainedInLeft:
        !drawerRect || !leftRect || (
          drawerRect.x >= leftRect.x - 1 &&
          drawerRect.right <= leftRect.right + 1 &&
          drawerRect.y >= leftRect.y - 1 &&
          drawerRect.bottom <= leftRect.bottom + 1
        ),
      groupsAlignedWithLeft:
        Boolean(groupRect && leftRect && Math.abs(groupRect.x - leftRect.x) <= 1 && Math.abs(groupRect.width - leftRect.width) <= 1),
      topRatio: Number(layout?.getAttribute("data-upper-lower-ratio") ?? NaN),
      lowerRatio: Number(layout?.getAttribute("data-lower-left-right-ratio") ?? NaN),
      workspace: (document.querySelector(".workspaceTabs button.active")?.textContent || "").trim(),
      controlMode: (document.querySelector(".controlModeTabs button.active")?.textContent || "").trim(),
      timelineExpanded: document.querySelector('.mappingPersistentWorkspaceBand')?.getAttribute('data-timeline-pane-expanded') === 'true',
      liveStatus: {
        expanded: livePanel?.getAttribute('data-live-status-expanded') === 'true',
        inspectorRole: document.querySelector('.liveControlPanel > .liveStatusGrid')?.getAttribute('role') ?? '',
        inspectorName: document.querySelector('.liveControlPanel > .liveStatusGrid')?.getAttribute('aria-label') ?? '',
        toggleNamed: Boolean(liveStatusToggle?.getAttribute('aria-label')),
        toggleControlsInspector: liveStatusToggle?.getAttribute('aria-controls') === 'live-status-inspector',
        toggleExpanded: liveStatusToggle?.getAttribute('aria-expanded') === 'true',
        inspectorWidth: liveStatusRect?.width ?? 0,
        visibleItemCount: visibleStatusItems.length,
        totalItemCount: document.querySelectorAll('.liveControlPanel > .liveStatusGrid > .liveStatusItem').length,
        matrixStatusDoNotOverlap: Boolean(
          sceneMatrixRect && liveStatusRect &&
          sceneMatrixRect.right <= liveStatusRect.x + 1
        ),
        matrixRow: getComputedStyle(document.querySelector('.liveControlPanel > .sceneMatrixPanel') ?? document.body).gridRowStart,
        fadeRow: getComputedStyle(document.querySelector('.liveControlPanel > .liveFadeMeter') ?? document.body).gridRowStart,
        masterRow: getComputedStyle(document.querySelector('.liveControlPanel > .liveMasterGrid') ?? document.body).gridRowStart,
        matrixFadeMasterDoNotOverlap: Boolean(
          sceneMatrixRect && liveFadeRect && liveMasterRect &&
          sceneMatrixRect.bottom <= liveFadeRect.y + 1 &&
          liveFadeRect.bottom <= liveMasterRect.y + 1
        ),
        transportButtonCount: visibleTransportButtons.length,
        transportRowCount: transportRows.size,
        sceneMatrixHeaderCount: document.querySelectorAll('.liveControlPanel .sceneMatrixHeader').length,
      },
      storage,
      storageRaw,
      splitDebug: {
        rootInlineStyle: layout?.getAttribute("style") ?? "",
        rootComputedTracks: layout ? getComputedStyle(layout).gridTemplateRows : "",
        rootClientHeight: layout?.clientHeight ?? 0,
        lowerInlineStyle: visibleElement('[data-workspace-pane="lower"]')?.getAttribute("style") ?? "",
        lowerComputedTracks: (() => {
          const lowerPane = visibleElement('[data-workspace-pane="lower"]');
          return lowerPane ? getComputedStyle(lowerPane).gridTemplateColumns : "";
        })(),
      },
      outerScrollZero:
        window.scrollX === 0 && window.scrollY === 0 &&
        documentElement.scrollWidth === documentElement.clientWidth &&
        documentElement.scrollHeight === documentElement.clientHeight &&
        body.scrollWidth === documentElement.clientWidth &&
        body.scrollHeight === documentElement.clientHeight &&
        (!app || (app.scrollWidth === app.clientWidth && app.scrollHeight === app.clientHeight)) &&
        (!layout || (layout.scrollWidth === layout.clientWidth && layout.scrollHeight === layout.clientHeight)),
    };
  });
}

async function measureWorkspaceDrawerReachability(client) {
  return evaluatePageFunction(client, () => {
    const drawer = document.querySelector('[data-workspace-selection-drawer][open]');
    const drawerBody = drawer?.querySelector('.mappingSelectionsDrawerBody') ?? null;
    const scroller = drawerBody?.querySelector('.mappingSelectionsColumn') ?? null;
    const leftPane = document.querySelector('[data-workspace-pane="lower-left"]');
    const documentElement = document.documentElement;
    const app = document.querySelector('.app');
    const rectSnapshot = (element) => {
      if (!element) return null;
      const rect = element.getBoundingClientRect();
      return {
        x: rect.x,
        y: rect.y,
        right: rect.right,
        bottom: rect.bottom,
        width: rect.width,
        height: rect.height,
      };
    };
    const isRendered = (element) => {
      if (!element) return false;
      const rect = element.getBoundingClientRect();
      const style = getComputedStyle(element);
      return rect.width > 0 && rect.height > 0 && style.display !== 'none' && style.visibility !== 'hidden';
    };
    const reachableAfterInternalScroll = (control) => {
      if (!drawerBody || !scroller || !control) return false;
      const ancestors = [];
      for (let ancestor = control.parentElement; ancestor; ancestor = ancestor.parentElement) {
        ancestors.push(ancestor);
        if (ancestor === drawerBody) break;
      }
      const scrollState = ancestors.map((ancestor) => ({
        element: ancestor,
        left: ancestor.scrollLeft,
        top: ancestor.scrollTop,
      }));
      const scrollableAncestors = ancestors.filter((ancestor) => {
        const style = getComputedStyle(ancestor);
        return (
          (/(auto|scroll)/.test(style.overflowY) && ancestor.scrollHeight > ancestor.clientHeight + 1) ||
          (/(auto|scroll)/.test(style.overflowX) && ancestor.scrollWidth > ancestor.clientWidth + 1)
        );
      });
      for (let pass = 0; pass < 4; pass += 1) {
        for (const ancestor of scrollableAncestors) {
          const ancestorRect = ancestor.getBoundingClientRect();
          const controlRect = control.getBoundingClientRect();
          if (controlRect.bottom > ancestorRect.bottom - 1) {
            ancestor.scrollTop += controlRect.bottom - ancestorRect.bottom + 1;
          } else if (controlRect.top < ancestorRect.top + 1) {
            ancestor.scrollTop -= ancestorRect.top - controlRect.top + 1;
          }
          if (controlRect.right > ancestorRect.right - 1) {
            ancestor.scrollLeft += controlRect.right - ancestorRect.right + 1;
          } else if (controlRect.left < ancestorRect.left + 1) {
            ancestor.scrollLeft -= ancestorRect.left - controlRect.left + 1;
          }
        }
      }
      const bodyRect = drawerBody.getBoundingClientRect();
      const controlRect = control.getBoundingClientRect();
      const reachable =
        isRendered(control) &&
        controlRect.left >= bodyRect.left - 1 &&
        controlRect.right <= bodyRect.right + 1 &&
        controlRect.top >= bodyRect.top - 1 &&
        controlRect.bottom <= bodyRect.bottom + 1 &&
        controlRect.left >= -1 &&
        controlRect.right <= innerWidth + 1 &&
        controlRect.top >= -1 &&
        controlRect.bottom <= innerHeight + 1;
      scrollState.forEach(({ element, left, top }) => {
        element.scrollLeft = left;
        element.scrollTop = top;
      });
      return reachable;
    };
    const buttons = drawerBody ? [...drawerBody.querySelectorAll('button')].filter(isRendered) : [];
    const inputs = drawerBody ? [...drawerBody.querySelectorAll('input')].filter(isRendered) : [];
    const interactives = drawerBody
      ? [...drawerBody.querySelectorAll('button, input')].filter(isRendered)
      : [];
    const drawerRect = rectSnapshot(drawer);
    const bodyRect = rectSnapshot(drawerBody);
    const leftPaneRect = rectSnapshot(leftPane);
    const summary = drawer?.querySelector(':scope > summary') ?? null;
    const summaryRect = rectSnapshot(summary);
    const drawerStyle = drawer ? getComputedStyle(drawer) : null;
    const bodyStyle = drawerBody ? getComputedStyle(drawerBody) : null;
    const scrollerStyle = scroller ? getComputedStyle(scroller) : null;
    const offsetParentSnapshot = (element) => {
      const offsetParent = element?.offsetParent ?? null;
      return offsetParent ? {
        tag: offsetParent.tagName,
        className: offsetParent.getAttribute('class') ?? '',
        rect: rectSnapshot(offsetParent),
      } : null;
    };
    return {
      drawerHeight: drawerRect?.height ?? 0,
      expectedDrawerHeight: leftPaneRect ? Math.min(359, Math.max(0, leftPaneRect.height - 8)) : 0,
      drawerBodyHeight: bodyRect?.height ?? 0,
      scrollerClientHeight: scroller?.clientHeight ?? 0,
      scrollerScrollHeight: scroller?.scrollHeight ?? 0,
      scrollerOverflowY: scrollerStyle?.overflowY ?? '',
      internallyScrollable: Boolean(
        scroller &&
        /(auto|scroll)/.test(scrollerStyle?.overflowY ?? '') &&
        scroller.scrollHeight > scroller.clientHeight + 1
      ),
      lastInteractiveReachable: reachableAfterInternalScroll(interactives.at(-1)),
      lastButtonReachable: reachableAfterInternalScroll(buttons.at(-1)),
      lastInputReachable: reachableAfterInternalScroll(inputs.at(-1)),
      controlCounts: [interactives.length, buttons.length, inputs.length],
      bodyInsideViewport: Boolean(
        bodyRect &&
        bodyRect.x >= -1 &&
        bodyRect.right <= innerWidth + 1 &&
        bodyRect.y >= -1 &&
        bodyRect.bottom <= innerHeight + 1
      ),
      outerScrollZero:
        window.scrollX === 0 && window.scrollY === 0 &&
        documentElement.scrollWidth === documentElement.clientWidth &&
        documentElement.scrollHeight === documentElement.clientHeight &&
        (!app || (app.scrollWidth === app.clientWidth && app.scrollHeight === app.clientHeight)),
      computed: {
        drawer: {
          display: drawerStyle?.display ?? '',
          gridTemplateRows: drawerStyle?.gridTemplateRows ?? '',
          height: drawerStyle?.height ?? '',
          minHeight: drawerStyle?.minHeight ?? '',
          overflow: drawerStyle?.overflow ?? '',
          overflowX: drawerStyle?.overflowX ?? '',
          overflowY: drawerStyle?.overflowY ?? '',
          offsetParent: offsetParentSnapshot(drawer),
        },
        summaryRect,
        body: {
          height: bodyStyle?.height ?? '',
          minHeight: bodyStyle?.minHeight ?? '',
          overflow: bodyStyle?.overflow ?? '',
          overflowX: bodyStyle?.overflowX ?? '',
          overflowY: bodyStyle?.overflowY ?? '',
          gridRow: bodyStyle?.gridRow ?? '',
          gridRowStart: bodyStyle?.gridRowStart ?? '',
          gridRowEnd: bodyStyle?.gridRowEnd ?? '',
          offsetParent: offsetParentSnapshot(drawerBody),
        },
        child: {
          height: scrollerStyle?.height ?? '',
          minHeight: scrollerStyle?.minHeight ?? '',
          overflow: scrollerStyle?.overflow ?? '',
          overflowX: scrollerStyle?.overflowX ?? '',
          overflowY: scrollerStyle?.overflowY ?? '',
          offsetParent: offsetParentSnapshot(scroller),
        },
      },
    };
  });
}

async function armWorkspacePointerCaptureProbe(client, splitter) {
  return evaluatePageFunction(client, (name) => {
    const element = document.querySelector(`[data-workspace-splitter="${name}"]`);
    if (!(element instanceof HTMLElement)) return false;
    window.__syndocalWorkspacePointerCaptureProbe = { got: 0, lost: 0 };
    element.addEventListener("gotpointercapture", () => {
      window.__syndocalWorkspacePointerCaptureProbe.got += 1;
    }, { once: true });
    element.addEventListener("lostpointercapture", () => {
      window.__syndocalWorkspacePointerCaptureProbe.lost += 1;
    }, { once: true });
    return true;
  }, splitter);
}

async function workspaceSplitterPoint(client, splitter) {
  return evaluatePageFunction(client, (name) => {
    const element = document.querySelector(`[data-workspace-splitter="${name}"]`);
    if (!element) return null;
    const rect = element.getBoundingClientRect();
    const style = getComputedStyle(element);
    if (rect.width <= 0 || rect.height <= 0 || style.display === "none" || style.visibility === "hidden") return null;
    return {
      x: rect.left + rect.width / 2,
      y: rect.top + rect.height / 2,
      viewportWidth: window.innerWidth,
      viewportHeight: window.innerHeight,
    };
  }, splitter);
}

async function dragWorkspaceSplitter(client, splitter, operation = {}) {
  const point = await workspaceSplitterPoint(client, splitter);
  if (!point) return { dragged: false, probe: { got: 0, lost: 0 } };
  await armWorkspacePointerCaptureProbe(client, splitter);
  const targetX = operation.edge === "start"
    ? 1
    : operation.edge === "end"
      ? point.viewportWidth - 1
      : point.x + (operation.deltaX ?? 0);
  const targetY = operation.edge === "start"
    ? 1
    : operation.edge === "end"
      ? point.viewportHeight - 1
      : point.y + (operation.deltaY ?? 0);
  await client.send("Input.dispatchMouseEvent", {
    type: "mousePressed",
    x: point.x,
    y: point.y,
    button: "left",
    buttons: 1,
    clickCount: 1,
  });
  await client.send("Input.dispatchMouseEvent", {
    type: "mouseMoved",
    x: targetX,
    y: targetY,
    button: "left",
    buttons: 1,
  });
  await sleep(48);
  if (operation.escape) {
    await pressKey(client, "Escape", "Escape");
    await sleep(32);
  }
  await client.send("Input.dispatchMouseEvent", {
    type: "mouseReleased",
    x: targetX,
    y: targetY,
    button: "left",
    buttons: 0,
    clickCount: 1,
  });
  await sleep(140);
  const probe = await client.evaluate(`window.__syndocalWorkspacePointerCaptureProbe ?? ({ got: 0, lost: 0 })`);
  return { dragged: true, probe };
}

async function doubleClickWorkspaceSplitter(client, splitter) {
  const point = await workspaceSplitterPoint(client, splitter);
  if (!point) return false;
  for (const clickCount of [1, 2]) {
    await client.send("Input.dispatchMouseEvent", {
      type: "mousePressed",
      x: point.x,
      y: point.y,
      button: "left",
      buttons: 1,
      clickCount,
    });
    await client.send("Input.dispatchMouseEvent", {
      type: "mouseReleased",
      x: point.x,
      y: point.y,
      button: "left",
      buttons: 0,
      clickCount,
    });
    await sleep(24);
  }
  await sleep(140);
  return true;
}

async function clickWorkspaceSelector(client, selector) {
  return evaluatePageFunction(client, (wanted) => {
    const element = document.querySelector(wanted);
    if (!(element instanceof HTMLElement)) return false;
    const rect = element.getBoundingClientRect();
    const style = getComputedStyle(element);
    if (rect.width <= 0 || rect.height <= 0 || style.display === "none" || style.visibility === "hidden") return false;
    element.click();
    return true;
  }, selector);
}

const workspaceNumberClose = (left, right, tolerance = 0.001) =>
  Number.isFinite(left) && Number.isFinite(right) && Math.abs(left - right) <= tolerance;

const workspaceRectClose = (left, right, tolerance = 1) =>
  Boolean(
    left && right &&
    Math.abs(left.x - right.x) <= tolerance &&
    Math.abs(left.y - right.y) <= tolerance &&
    Math.abs(left.width - right.width) <= tolerance &&
    Math.abs(left.height - right.height) <= tolerance
  );

async function runWorkspaceSplitViewport(client, viewport) {
  await client.send("Emulation.setDeviceMetricsOverride", {
    width: viewport.width,
    height: viewport.height,
    deviceScaleFactor: 1,
    mobile: false,
  });
  await client.send("Page.navigate", { url: appUrl });
  await waitForApp(client);
  // Deliberately seed the pre-T15-P schema. A successful load must migrate the
  // missing split fields to defaults without collapsing any pane.
  await seedViewportLocalStorage(client);
  await client.send("Page.navigate", { url: appUrl });
  await waitForApp(client);
  await clickVisibleByText(client, ".workspaceTabs button", "Control");
  await clickVisibleByText(client, ".controlModeTabs button", "Timeline");
  await sleep(160);

  const initial = await readWorkspaceSplitState(client);
  const checks = {
    normalShellHasOneUpperAndTwoLowerPaneHosts:
      Boolean(initial.rootRect && initial.lowerRect && initial.leftRect && initial.rightRect),
    normalShellHasTwoAccessibleSplitters:
      initial.visibleSplitterCount === 2 &&
      initial.topSplitter?.role === "separator" &&
      initial.topSplitter?.orientation === "horizontal" &&
      initial.topSplitter?.label.length > 0 &&
      initial.topSplitter?.tabIndex === 0 &&
      initial.lowerSplitter?.role === "separator" &&
      initial.lowerSplitter?.orientation === "vertical" &&
      initial.lowerSplitter?.label.length > 0 &&
      initial.lowerSplitter?.tabIndex === 0,
    splitterAriaValuesTrackRenderedRatios:
      initial.topSplitter?.minimum <= initial.topSplitter?.current &&
      initial.topSplitter?.current <= initial.topSplitter?.maximum &&
      initial.lowerSplitter?.minimum <= initial.lowerSplitter?.current &&
      initial.lowerSplitter?.current <= initial.lowerSplitter?.maximum &&
      Math.abs(initial.topSplitter?.current - Math.round(initial.renderedTopRatio * 100)) <= 1 &&
      Math.abs(initial.lowerSplitter?.current - Math.round(initial.renderedLowerRatio * 100)) <= 1,
    legacyWorkspaceStorageUsesDefaultSplitRatios:
      workspaceNumberClose(initial.topRatio, workspaceSplitDefaults.top) &&
      workspaceNumberClose(initial.lowerRatio, workspaceSplitDefaults.lower) &&
      workspaceNumberClose(initial.storage?.top_split_ratio, workspaceSplitDefaults.top) &&
      workspaceNumberClose(initial.storage?.lower_split_ratio, workspaceSplitDefaults.lower),
    normalPaneMinimumsSatisfied:
      initial.upperHeight >= workspaceSplitMinimums.upper - 2 &&
      initial.lowerHeight >= workspaceSplitMinimums.lower - 2 &&
      initial.leftRect?.width >= workspaceSplitMinimums.lowerLeft - 2 &&
      initial.rightRect?.width >= workspaceSplitMinimums.lowerRight - 2,
    selectionDrawerBelongsToLowerLeftPane:
      initial.drawerToggleNamed && initial.drawerExpandedMatches && initial.groupsAlignedWithLeft,
    normalShellOuterScrollZero: initial.outerScrollZero,
  };

  const expectedClosedStatusWidth = viewport.width <= 1320 ? 168 : 180;
  const expectedExpandedStatusWidth = viewport.width <= 1320 ? 238 : 248;
  checks.liveStatusToggleNamedAndControlsInspector =
    initial.liveStatus.toggleNamed &&
    initial.liveStatus.toggleControlsInspector &&
    initial.liveStatus.toggleExpanded === false;
  checks.liveStatusInspectorIsNamedRegion =
    initial.liveStatus.inspectorRole === 'region' && initial.liveStatus.inspectorName.trim().length > 0;
  checks.liveStatusClosedShowsTwoCellsAtExpectedWidth =
    initial.liveStatus.expanded === false &&
    initial.liveStatus.visibleItemCount === 2 &&
    initial.liveStatus.totalItemCount === 11 &&
    Math.abs(initial.liveStatus.inspectorWidth - expectedClosedStatusWidth) <= 2;
  checks.liveTransportIsNineButtonsInOneRow =
    initial.liveStatus.transportButtonCount === 9 && initial.liveStatus.transportRowCount === 1;
  checks.liveMatrixFadeMasterUseRowsThreeFourFive =
    initial.liveStatus.matrixRow === "3" &&
    initial.liveStatus.fadeRow === "4" &&
    initial.liveStatus.masterRow === "5" &&
    initial.liveStatus.matrixFadeMasterDoNotOverlap;
  checks.sceneMatrixHeaderRemoved = initial.liveStatus.sceneMatrixHeaderCount === 0;
  checks.liveStatusClosedDoesNotOverlapMatrix = initial.liveStatus.matrixStatusDoNotOverlap;
  const liveStatusOpened = await clickWorkspaceSelector(client, '[data-live-status-toggle]');
  await sleep(100);
  const expandedLiveStatus = await readWorkspaceSplitState(client);
  checks.liveStatusExpandedShowsElevenCellsAtExpectedWidth =
    liveStatusOpened &&
    expandedLiveStatus.liveStatus.expanded &&
    expandedLiveStatus.liveStatus.toggleExpanded &&
    expandedLiveStatus.liveStatus.visibleItemCount === 11 &&
    Math.abs(expandedLiveStatus.liveStatus.inspectorWidth - expectedExpandedStatusWidth) <= 2;
  checks.liveStatusExpansionHasExpectedWidthRatio =
    expandedLiveStatus.liveStatus.inspectorWidth / Math.max(1, initial.liveStatus.inspectorWidth) >= 1.3;
  checks.liveStatusExpandedDoesNotOverlapMatrixOrOuterScroll =
    expandedLiveStatus.liveStatus.matrixStatusDoNotOverlap && expandedLiveStatus.outerScrollZero;
  const liveStatusClosed = await clickWorkspaceSelector(client, '[data-live-status-toggle]');
  await sleep(100);
  const restoredLiveStatus = await readWorkspaceSplitState(client);
  checks.liveStatusToggleRestoresClosedContract =
    liveStatusClosed &&
    !restoredLiveStatus.liveStatus.expanded &&
    restoredLiveStatus.liveStatus.visibleItemCount === 2 &&
    Math.abs(restoredLiveStatus.liveStatus.inspectorWidth - expectedClosedStatusWidth) <= 2 &&
    restoredLiveStatus.outerScrollZero;

  const topDrag = await dragWorkspaceSplitter(client, "upper-lower", { deltaY: -64 });
  const topCustom = await readWorkspaceSplitState(client);
  const topHasVisualRange =
    (initial.topSplitter?.maximum ?? 0) - (initial.topSplitter?.minimum ?? 0) >= 2;
  checks.upperLowerSplitterCapturedAndReleasedPointer =
    topDrag.dragged && topDrag.probe.got >= 1 && topDrag.probe.lost >= 1;
  checks.upperLowerDragChangesOnlyUpperLowerRatio =
    (topHasVisualRange
      ? Math.abs(topCustom.topRatio - initial.topRatio) >= 0.01
      : workspaceNumberClose(topCustom.topRatio, initial.topRatio) &&
        workspaceNumberClose(topCustom.renderedTopRatio, initial.renderedTopRatio, 0.002)) &&
    workspaceNumberClose(topCustom.lowerRatio, initial.lowerRatio);
  checks.upperLowerDragKeepsOuterScrollZero = topCustom.outerScrollZero;

  const topEscapeBefore = await readWorkspaceSplitState(client);
  const topEscape = await dragWorkspaceSplitter(client, "upper-lower", { deltaY: 56, escape: true });
  const topEscapeAfter = await readWorkspaceSplitState(client);
  checks.upperLowerEscapeRestoresPreGestureGeometry =
    topEscape.dragged && topEscape.probe.got >= 1 && topEscape.probe.lost >= 1 &&
    workspaceNumberClose(topEscapeAfter.topRatio, topEscapeBefore.topRatio) &&
    workspaceRectClose(topEscapeAfter.topSplitter?.rect, topEscapeBefore.topSplitter?.rect) &&
    topEscapeAfter.storageRaw === topEscapeBefore.storageRaw;

  await dragWorkspaceSplitter(client, "upper-lower", { edge: "start" });
  const topMinimum = await readWorkspaceSplitState(client);
  await dragWorkspaceSplitter(client, "upper-lower", { edge: "end" });
  const bottomMinimum = await readWorkspaceSplitState(client);
  checks.upperLowerDragClampsBothPaneMinimums =
    topMinimum.upperHeight >= workspaceSplitMinimums.upper - 2 &&
    topMinimum.upperHeight <= workspaceSplitMinimums.upper + 3 &&
    bottomMinimum.lowerHeight >= workspaceSplitMinimums.lower - 2 &&
    bottomMinimum.lowerHeight <= workspaceSplitMinimums.lower + 3 &&
    topMinimum.outerScrollZero && bottomMinimum.outerScrollZero;
  checks.upperLowerDoubleClickTriggered = await doubleClickWorkspaceSplitter(client, "upper-lower");
  const topReset = await readWorkspaceSplitState(client);
  checks.upperLowerDoubleClickRestoresDefaultRatio = workspaceNumberClose(topReset.topRatio, workspaceSplitDefaults.top, 0.002);

  const lowerDrag = await dragWorkspaceSplitter(client, "lower-left-right", { deltaX: 72 });
  const lowerCustom = await readWorkspaceSplitState(client);
  checks.lowerLeftRightSplitterCapturedAndReleasedPointer =
    lowerDrag.dragged && lowerDrag.probe.got >= 1 && lowerDrag.probe.lost >= 1;
  checks.lowerLeftRightDragChangesOnlyLowerRatio =
    Math.abs(lowerCustom.lowerRatio - topReset.lowerRatio) >= 0.01 &&
    workspaceNumberClose(lowerCustom.topRatio, topReset.topRatio);
  checks.lowerLeftRightDragKeepsOuterScrollZero = lowerCustom.outerScrollZero;

  const lowerEscapeBefore = await readWorkspaceSplitState(client);
  const lowerEscape = await dragWorkspaceSplitter(client, "lower-left-right", { deltaX: -56, escape: true });
  const lowerEscapeAfter = await readWorkspaceSplitState(client);
  checks.lowerLeftRightEscapeRestoresPreGestureGeometry =
    lowerEscape.dragged && lowerEscape.probe.got >= 1 && lowerEscape.probe.lost >= 1 &&
    workspaceNumberClose(lowerEscapeAfter.lowerRatio, lowerEscapeBefore.lowerRatio) &&
    workspaceRectClose(lowerEscapeAfter.lowerSplitter?.rect, lowerEscapeBefore.lowerSplitter?.rect) &&
    lowerEscapeAfter.storageRaw === lowerEscapeBefore.storageRaw;

  await dragWorkspaceSplitter(client, "lower-left-right", { edge: "start" });
  const leftMinimum = await readWorkspaceSplitState(client);
  await dragWorkspaceSplitter(client, "lower-left-right", { edge: "end" });
  const rightMinimum = await readWorkspaceSplitState(client);
  checks.lowerLeftRightDragClampsBothPaneMinimums =
    leftMinimum.leftRect?.width >= workspaceSplitMinimums.lowerLeft - 2 &&
    leftMinimum.leftRect?.width <= workspaceSplitMinimums.lowerLeft + 3 &&
    rightMinimum.rightRect?.width >= workspaceSplitMinimums.lowerRight - 2 &&
    rightMinimum.rightRect?.width <= workspaceSplitMinimums.lowerRight + 3 &&
    leftMinimum.outerScrollZero && rightMinimum.outerScrollZero;
  checks.lowerLeftRightDoubleClickTriggered = await doubleClickWorkspaceSplitter(client, "lower-left-right");
  const lowerReset = await readWorkspaceSplitState(client);
  checks.lowerLeftRightDoubleClickRestoresDefaultRatio = workspaceNumberClose(lowerReset.lowerRatio, workspaceSplitDefaults.lower, 0.002);

  // Commit non-default ratios for reload, workspace-switch and T8 restoration.
  await dragWorkspaceSplitter(client, "upper-lower", { deltaY: -42 });
  await dragWorkspaceSplitter(client, "lower-left-right", { deltaX: 54 });
  const customBeforeDrawer = await readWorkspaceSplitState(client);
  const drawerClickedOpen = await clickWorkspaceSelector(client, '[data-workspace-selection-drawer-toggle]');
  await sleep(100);
  const drawerOpen = await readWorkspaceSplitState(client);
  const drawerOpenMetrics = await measureWorkspaceDrawerReachability(client);
  const drawerClickedClosed = await clickWorkspaceSelector(client, '[data-workspace-selection-drawer-toggle]');
  await sleep(100);
  const drawerClosed = await readWorkspaceSplitState(client);
  checks.selectionDrawerToggleNamed = initial.drawerToggleNamed;
  checks.selectionDrawerDoesNotMoveLowerSplitBoundary =
    drawerClickedOpen && drawerClickedClosed && drawerOpen.drawerOpen && !drawerClosed.drawerOpen &&
    drawerOpen.drawerContainedInLeft &&
    workspaceRectClose(customBeforeDrawer.lowerSplitter?.rect, drawerOpen.lowerSplitter?.rect) &&
    workspaceRectClose(customBeforeDrawer.lowerSplitter?.rect, drawerClosed.lowerSplitter?.rect);
  checks.selectionDrawerStatePersistedLocally = drawerClosed.storage?.selections_drawer_open === false;
  const compactMainViewport = viewport.width <= 1366 && viewport.height <= 768;
  checks.compactSelectionDrawerHasDeterministicHeightAndInternalScroll =
    !compactMainViewport || (
      Math.abs(drawerOpenMetrics.drawerHeight - drawerOpenMetrics.expectedDrawerHeight) <= 1 &&
      drawerOpenMetrics.drawerBodyHeight > 0 &&
      drawerOpenMetrics.internallyScrollable
    );
  checks.compactSelectionDrawerLastControlsReachBodyAndViewport =
    !compactMainViewport || (
      drawerOpenMetrics.controlCounts[0] > 0 &&
      drawerOpenMetrics.controlCounts[1] > 0 &&
      drawerOpenMetrics.controlCounts[2] > 0 &&
      drawerOpenMetrics.lastInteractiveReachable &&
      drawerOpenMetrics.lastButtonReachable &&
      drawerOpenMetrics.lastInputReachable &&
      drawerOpenMetrics.bodyInsideViewport &&
      drawerOpenMetrics.outerScrollZero
    );

  const beforeReload = await readWorkspaceSplitState(client);
  await client.send("Page.navigate", { url: appUrl });
  await waitForApp(client);
  await sleep(160);
  const afterReload = await readWorkspaceSplitState(client);
  checks.customSplitRatiosPersistAfterReload =
    workspaceNumberClose(afterReload.topRatio, beforeReload.topRatio) &&
    workspaceNumberClose(afterReload.lowerRatio, beforeReload.lowerRatio) &&
    workspaceRectClose(afterReload.topSplitter?.rect, beforeReload.topSplitter?.rect) &&
    workspaceRectClose(afterReload.lowerSplitter?.rect, beforeReload.lowerSplitter?.rect);
  checks.reloadKeepsOuterScrollZero = afterReload.outerScrollZero;

  await clickVisibleByText(client, ".workspaceTabs button", "Setup");
  await clickByText(client, "Mapping");
  await clickByText(client, "Stage Map");
  await sleep(120);
  const setupBefore = await readWorkspaceSplitState(client);
  await clickVisibleByText(client, ".workspaceTabs button", "Control");
  await clickVisibleByText(client, ".controlModeTabs button", "Timeline");
  await sleep(120);
  const control = await readWorkspaceSplitState(client);
  await clickVisibleByText(client, ".workspaceTabs button", "Setup");
  await clickByText(client, "Mapping");
  await clickByText(client, "Stage Map");
  await sleep(120);
  const setupAfter = await readWorkspaceSplitState(client);
  checks.setupControlSwitchPreservesSplitRatios =
    workspaceNumberClose(setupBefore.topRatio, control.topRatio) &&
    workspaceNumberClose(setupBefore.lowerRatio, control.lowerRatio) &&
    workspaceNumberClose(control.topRatio, setupAfter.topRatio) &&
    workspaceNumberClose(control.lowerRatio, setupAfter.lowerRatio);
  checks.setupControlSwitchPreservesSplitBoundariesWithinOnePixel =
    workspaceRectClose(setupBefore.topSplitter?.rect, control.topSplitter?.rect) &&
    workspaceRectClose(control.topSplitter?.rect, setupAfter.topSplitter?.rect) &&
    workspaceRectClose(setupBefore.lowerSplitter?.rect, control.lowerSplitter?.rect) &&
    workspaceRectClose(control.lowerSplitter?.rect, setupAfter.lowerSplitter?.rect);
  checks.workspaceSwitchKeepsDocumentAndAppScrollZero =
    setupBefore.outerScrollZero && control.outerScrollZero && setupAfter.outerScrollZero;

  const timelinePaneExpansion = await runTimelinePaneExpansionCheck(client, viewport);
  checks.timelineFocusContract = timelinePaneExpansion.passed === true;
  const paneWindow = await runPaneWindowViewport(client, viewport);
  checks.paneWindowContract = paneWindow.passed === true;

  await client.send("Page.navigate", { url: appUrl });
  await waitForApp(client);
  await clickVisibleByText(client, ".appMenuButton", "...");
  await clickVisibleByText(client, ".appProjectMenu button", "Reset Layout");
  await sleep(120);
  const reset = await readWorkspaceSplitState(client);
  checks.resetLayoutRestoresTabsDrawerAndDefaultRatios =
    reset.workspace === "Setup" &&
    workspaceNumberClose(reset.topRatio, workspaceSplitDefaults.top) &&
    workspaceNumberClose(reset.lowerRatio, workspaceSplitDefaults.lower) &&
    reset.drawerOpen === false &&
    workspaceNumberClose(reset.storage?.top_split_ratio, workspaceSplitDefaults.top) &&
    workspaceNumberClose(reset.storage?.lower_split_ratio, workspaceSplitDefaults.lower) &&
    reset.storage?.selections_drawer_open === false;
  checks.resetLayoutKeepsOuterScrollZero = reset.outerScrollZero;

  const failedChecks = Object.entries(checks).filter(([, passed]) => !passed).map(([name]) => name);
  return {
    viewport,
    label: `workspace-split-${viewport.width}x${viewport.height}`,
    passed: failedChecks.length === 0,
    checks,
    failedChecks,
    initial: {
      ratios: [initial.topRatio, initial.lowerRatio],
      renderedRatios: [initial.renderedTopRatio, initial.renderedLowerRatio],
      panes: [initial.upperHeight, initial.lowerHeight, initial.leftRect?.width ?? 0, initial.rightRect?.width ?? 0],
      splitterAria: [initial.topSplitter, initial.lowerSplitter].map((splitter) => splitter ? [
        splitter.minimum,
        splitter.current,
        splitter.maximum,
      ] : null),
      debug: initial.splitDebug,
    },
    persisted: {
      before: [beforeReload.topRatio, beforeReload.lowerRatio],
      after: [afterReload.topRatio, afterReload.lowerRatio],
    },
    drawerOpenMetrics,
    timelinePaneExpansion,
    paneWindow,
  };
}

async function runPersistentBandInvarianceViewport(client, viewport) {
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

  await clickByText(client, "Setup");
  await clickByText(client, "Mapping");
  await clickByText(client, "Stage Map");
  await sleep(120);
  const setupBefore = await measurePersistentBand(client, `persistent-band-setup-before-${viewport.width}x${viewport.height}`);
  await clickByText(client, "Control");
  await clickByText(client, "Live Edit");
  await sleep(120);
  const control = await measurePersistentBand(client, `persistent-band-control-${viewport.width}x${viewport.height}`);
  await clickByText(client, "Setup");
  await clickByText(client, "Mapping");
  await clickByText(client, "Stage Map");
  await sleep(120);
  const setupAfter = await measurePersistentBand(client, `persistent-band-setup-after-${viewport.width}x${viewport.height}`);
  const comparison = comparePersistentBandMeasurements(setupBefore, control, setupAfter);
  const timelinePaneExpansion = await runTimelinePaneExpansionCheck(client, viewport);
  const layeredTimelineDesk = await runLayeredTimelineDeskCheck(client, viewport);
  const containment = {
    ...setupAfter,
    label: `persistent-band-invariance-${viewport.width}x${viewport.height}`,
    persistentBandInvariant: comparison.invariant,
    persistentBandRectsByWorkspace: comparison.rectsByWorkspace,
    persistentBandRectDeltas: comparison.deltas,
    timelinePaneExpansion,
    layeredTimelineDesk,
  };
  return {
    passed:
      setupBefore.outerContained &&
      control.outerContained &&
      setupAfter.outerContained &&
      hasExpectedPersistentWorkspaceBand(setupBefore) &&
      hasExpectedPersistentWorkspaceBand(control) &&
      hasExpectedPersistentWorkspaceBand(containment) &&
      hasExpectedPersistentBandInvariance(containment) &&
      hasExpectedTimelinePaneExpansion(containment) &&
      hasExpectedLayeredTimelineDesk(containment),
    containment,
  };
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
  await clickVisibleByText(client, ".liveDeskViewToggle button", "Matrix");
  await clickVisibleSelector(
    client,
    "[data-scene-matrix-edit-cue], [data-scene-matrix-open-cue-editor]",
  );
  await sleep(120);
  await client.evaluate(`(() => {
    const toggle = document.querySelector('.cuePanelEditToggle');
    if (toggle instanceof HTMLButtonElement && toggle.getAttribute('aria-expanded') !== 'true') toggle.click();
  })()`);
  await sleep(120);
}

async function measureSceneMatrixPane(client) {
  return await client.evaluate(`(async () => {
    await new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame)));
    const isVisible = (element) => {
      if (!element) return false;
      const rect = element.getBoundingClientRect();
      const style = getComputedStyle(element);
      return rect.width > 0 && rect.height > 0 && style.display !== 'none' && style.visibility !== 'hidden';
    };
    const pane = document.querySelector('.liveControlPanel > .sceneMatrixPanel');
    const scroller = document.querySelector('.sceneMatrixScroller');
    const columns = [...document.querySelectorAll('[data-scene-matrix-column]')].filter(isVisible);
    const headers = [...document.querySelectorAll('[data-scene-matrix-group-hue]')].filter(isVisible);
    const cards = [...document.querySelectorAll('[data-scene-matrix-cue-id]')].filter(isVisible);
    const triggers = cards.map((card) => card.querySelector('.sceneMatrixTrigger')).filter(isVisible);
    const kindBadges = cards.map((card) => card.querySelector('[data-scene-matrix-kind]')).filter(isVisible);
    const progressBars = cards.map((card) => card.querySelector('[data-scene-matrix-progress] progress')).filter(isVisible);
    const bankStrips = columns.map((column) => column.querySelector('.sceneMatrixBankStrip')).filter(isVisible);
    const cardScrollers = columns.map((column) => column.querySelector('.sceneMatrixCards')).filter(isVisible);
    const dragHandles = cards.map((card) => card.querySelector('.cueTimelineDragHandle')).filter(isVisible);
    const editCueButtons = cards.map((card) => {
      const button = card.querySelector('[data-scene-matrix-edit-cue]');
      const label = (card.querySelector('.sceneMatrixTrigger strong')?.textContent || '').trim();
      return {
        cueId: card.getAttribute('data-scene-matrix-cue-id') || '',
        label,
        name: (button?.getAttribute('aria-label') || '').trim(),
        rendered: isVisible(button),
      };
    });
    const dragSources = cards.filter((card) => card.hasAttribute('data-timeline-cue-drag-source'));
    const timelineLanes = [...document.querySelectorAll('.timelineShowSurface [data-timeline-layer-id]')].filter(isVisible);
    const liveViewButtons = [...document.querySelectorAll('.liveDeskViewToggle button')].filter(isVisible);
    const matrixTextNodes = pane
      ? [...pane.querySelectorAll('h2, p, span, strong, small, button')]
          .filter(isVisible)
          .filter((element) => (element.textContent || '').trim().length > 0)
      : [];
    const replaceCards = cards.filter((card) => isVisible(card.querySelector('.sceneMatrixReplaceBadge')));
    const groupColorInputs = [...document.querySelectorAll('[data-group-color-input]')].filter(isVisible);
    const backHeader = headers.find((header) =>
      header.closest('[data-scene-matrix-column]')?.getAttribute('data-scene-matrix-column') === 'back');
    const documentElement = document.documentElement;
    const body = document.body;
    const app = document.querySelector('.app');
    const livePanel = pane?.parentElement ?? null;
    const status = document.querySelector('.liveControlPanel > .liveStatusGrid');
    const paneRect = pane?.getBoundingClientRect() ?? null;
    const liveRect = livePanel?.getBoundingClientRect() ?? null;
    const statusRect = status?.getBoundingClientRect() ?? null;
    const overlapArea = paneRect && statusRect
      ? Math.max(0, Math.min(paneRect.right, statusRect.right) - Math.max(paneRect.left, statusRect.left)) *
        Math.max(0, Math.min(paneRect.bottom, statusRect.bottom) - Math.max(paneRect.top, statusRect.top))
      : Number.POSITIVE_INFINITY;
    const scrollerStyle = scroller ? getComputedStyle(scroller) : null;
    return {
      paneVisible: isVisible(pane),
      paneInPrimaryLiveDesk: pane?.parentElement?.classList.contains('liveControlPanel') === true,
      paneHeightCoverage: paneRect && liveRect && liveRect.height > 0 ? paneRect.height / liveRect.height : 0,
      paneWidthCoverage: paneRect && liveRect && liveRect.width > 0 ? paneRect.width / liveRect.width : 0,
      paneRect: paneRect ? {
        left: paneRect.left,
        top: paneRect.top,
        right: paneRect.right,
        bottom: paneRect.bottom,
        width: paneRect.width,
        height: paneRect.height,
      } : null,
      statusVisible: isVisible(status),
      statusRect: statusRect ? {
        left: statusRect.left,
        top: statusRect.top,
        right: statusRect.right,
        bottom: statusRect.bottom,
        width: statusRect.width,
        height: statusRect.height,
      } : null,
      paneStatusOverlap: overlapArea > 1,
      matrixMinFontPx: matrixTextNodes.length > 0
        ? Math.min(...matrixTextNodes.map((element) => parseFloat(getComputedStyle(element).fontSize) || 0))
        : 0,
      columns: columns.map((column) => column.getAttribute('data-scene-matrix-column')),
      headerHues: headers.map((header) => header.getAttribute('data-scene-matrix-group-hue')),
      cardHues: cards.map((card) => card.getAttribute('data-scene-matrix-cue-hue')),
      cardIds: cards.map((card) => card.getAttribute('data-scene-matrix-cue-id')),
      groupColorInputCount: groupColorInputs.length,
      cueColorInputCount: [...document.querySelectorAll('[data-cue-color-input]')].filter(isVisible).length,
      coloredCardIdentity: document.querySelector('[data-scene-matrix-cue-id="301"]')?.style.getPropertyValue('--cue-identity') ?? '',
      backHeaderIdentity: backHeader?.closest('[data-scene-matrix-column]')?.style.getPropertyValue('--group-identity') ?? '',
      replaceCardIds: replaceCards.map((card) => card.getAttribute('data-scene-matrix-cue-id')),
      kindBadgeCounts: Object.fromEntries(['STATIC', 'FX'].map((kind) => [
        kind,
        kindBadges.filter((badge) => badge.getAttribute('data-scene-matrix-kind') === kind).length,
      ])),
      bankStripCount: bankStrips.length,
      bankStripColors: bankStrips.map((strip) => getComputedStyle(strip).backgroundColor),
      progressBarCount: progressBars.length,
      progressValues: progressBars.map((progress) => Number(progress.value)),
      minCellHitSize: triggers.length > 0
        ? Math.min(...triggers.map((trigger) => {
            const rect = trigger.getBoundingClientRect();
            return Math.min(rect.width, rect.height);
          }))
        : 0,
      dragHandleCount: dragHandles.length,
      editCueButtons,
      minDragHandleHitSize: dragHandles.length > 0
        ? Math.min(...dragHandles.map((handle) => {
            const rect = handle.getBoundingClientRect();
            return Math.min(rect.width, rect.height);
          }))
        : 0,
      dragHandleTouchActions: dragHandles.map((handle) => getComputedStyle(handle).touchAction),
      dragSourceCount: dragSources.length,
      visibleTimelineLaneCount: timelineLanes.length,
      timelineShowSurfaceVisible: isVisible(document.querySelector('.timelineShowSurface')),
      liveViewButtonCount: liveViewButtons.length,
      activeLiveView: (liveViewButtons.find((button) => button.getAttribute('aria-pressed') === 'true')?.textContent || '').trim(),
      activeCardIds: cards
        .filter((card) => card.getAttribute('data-scene-matrix-active') === 'true' && card.classList.contains('active'))
        .map((card) => card.getAttribute('data-scene-matrix-cue-id')),
      internalScrollport: Boolean(
        scroller && scrollerStyle && /(auto|scroll)/.test(scrollerStyle.overflowX) &&
        cardScrollers.length === columns.length &&
        cardScrollers.every((cardScroller) => /(auto|scroll)/.test(getComputedStyle(cardScroller).overflowY))
      ),
      documentAndAppScrollZero:
        window.scrollX === 0 && window.scrollY === 0 &&
        documentElement.scrollWidth === documentElement.clientWidth &&
        documentElement.scrollHeight === documentElement.clientHeight &&
        body.scrollWidth === documentElement.clientWidth &&
        body.scrollHeight === documentElement.clientHeight &&
        (!app || (app.scrollWidth === app.clientWidth && app.scrollHeight === app.clientHeight)),
      scrollMetrics: {
        document: [
          documentElement.scrollWidth,
          documentElement.clientWidth,
          documentElement.scrollHeight,
          documentElement.clientHeight,
        ],
        app: app ? [app.scrollWidth, app.clientWidth, app.scrollHeight, app.clientHeight] : null,
        matrix: scroller ? [scroller.scrollWidth, scroller.clientWidth, scroller.scrollHeight, scroller.clientHeight] : null,
      },
    };
  })()`);
}

async function runGroupStrobeViewport(client, viewport) {
  await client.send("Emulation.setDeviceMetricsOverride", {
    width: viewport.width,
    height: viewport.height,
    deviceScaleFactor: 1,
    mobile: false,
  });
  await client.send("Page.navigate", { url: fixtureUrl("scene-matrix") });
  await waitForApp(client);
  await clickVisibleByText(client, ".workspaceTabs button", "Control");
  await clickVisibleByText(client, ".controlModeTabs button", "Timeline");
  await waitForClientCondition(
    client,
    "document.querySelector('.groupLiveMixerStrip input[aria-label=\"Group strobe rate\"]')",
    "group Live Mixer strobe",
  );
  const readState = async (scope) => await client.evaluate(`(() => {
    const strip = document.querySelector('.groupLiveMixerStrip');
    const slider = strip?.querySelector('input[aria-label="Group strobe rate"]');
    const solo = strip?.querySelector('button');
    const coverage = strip?.querySelector('[data-strobe-compatible-count]');
    const touchSlider = document.querySelector('.touchGroupStrobeControl input');
    const app = document.querySelector('.app');
    const documentElement = document.documentElement;
    const body = document.body;
    const rect = strip?.getBoundingClientRect();
    const hostRect = strip?.parentElement?.getBoundingClientRect();
    return {
      scope: ${JSON.stringify(scope)},
      stripCount: document.querySelectorAll('.groupLiveMixerStrip').length,
      strobeValue: slider instanceof HTMLInputElement ? Number(slider.value) : -1,
      strobeDisabled: slider instanceof HTMLInputElement ? slider.disabled : true,
      compatibleCount: Number(coverage?.getAttribute('data-strobe-compatible-count') ?? -1),
      soloPressed: solo?.getAttribute('aria-pressed') ?? '',
      touchStrobeValue: touchSlider instanceof HTMLInputElement ? Number(touchSlider.value) : -1,
      stripContained: Boolean(rect && hostRect && rect.left >= hostRect.left - 1 && rect.right <= hostRect.right + 1),
      documentAndAppScrollZero:
        window.scrollX === 0 && window.scrollY === 0 &&
        documentElement.scrollWidth === documentElement.clientWidth &&
        documentElement.scrollHeight === documentElement.clientHeight &&
        body.scrollWidth === documentElement.clientWidth &&
        body.scrollHeight === documentElement.clientHeight &&
        (!app || (app.scrollWidth === app.clientWidth && app.scrollHeight === app.clientHeight)),
    };
  })()`);
  const setRange = async (selector, value) => {
    const changed = await client.evaluate(`(() => {
      const input = document.querySelector(${JSON.stringify(selector)});
      if (!(input instanceof HTMLInputElement) || input.disabled) return false;
      const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value')?.set;
      setter?.call(input, ${JSON.stringify(value)});
      input.dispatchEvent(new Event('input', { bubbles: true }));
      input.dispatchEvent(new Event('change', { bubbles: true }));
      return true;
    })()`);
    if (!changed) throw new Error(`Could not set range ${selector}`);
    await sleep(80);
  };

  const initial = await readState("desktop-initial");
  await setRange('.groupLiveMixerStrip input[aria-label="Group strobe rate"]', 12);
  await client.evaluate(`document.querySelector('.groupLiveMixerStrip button')?.click()`);
  await sleep(80);
  const latched = await readState("desktop-latched");
  await setRange('.groupLiveMixerStrip input[aria-label="Group strobe rate"]', 0);
  await client.evaluate(`document.querySelector('.groupLiveMixerStrip button')?.click()`);
  await sleep(80);
  const cleared = await readState("desktop-cleared");

  await clickVisibleByText(client, ".workspaceTabs button", "Touch");
  await waitForClientCondition(
    client,
    "document.querySelector('.touchGroupStrobeControl input')",
    "Touch group strobe",
  );
  await setRange('.touchGroupStrobeControl input', 18);
  const touchLatched = await readState("touch-latched");
  await setRange('.touchGroupStrobeControl input', 0);
  const touchCleared = await readState("touch-cleared");

  const conditions = [
    ["desktopLiveMixerVisible", () => initial.stripCount === 1 && initial.stripContained],
    ["desktopGdtfCoverage", () => !initial.strobeDisabled && initial.compatibleCount === 2],
    ["desktopStrobeLatches", () => latched.strobeValue === 12],
    ["desktopSoloDirect", () => latched.soloPressed === "true"],
    ["desktopReset", () => cleared.strobeValue === 0 && cleared.soloPressed === "false"],
    ["touchStrobeLatches", () => touchLatched.touchStrobeValue === 18],
    ["touchStrobeReset", () => touchCleared.touchStrobeValue === 0],
    ["viewportContained", () => [initial, latched, cleared, touchLatched, touchCleared]
      .every((state) => state.documentAndAppScrollZero)],
  ];
  const failedChecks = conditions.filter(([, check]) => !check()).map(([name]) => name);
  return {
    viewport,
    label: `group-strobe-${viewport.width}x${viewport.height}`,
    passed: failedChecks.length === 0,
    failedChecks,
    initial,
    latched,
    cleared,
    touchLatched,
    touchCleared,
  };
}

async function exerciseSceneMatrixHorizontalScroll(client) {
  return await client.evaluate(`(async () => {
    const settle = () => new Promise((resolveFrame) =>
      requestAnimationFrame(() => requestAnimationFrame(resolveFrame)));
    const scroller = document.querySelector('.sceneMatrixScroller');
    const showColumn = document.querySelector('[data-scene-matrix-column="Show"]');
    const showHeader = showColumn?.querySelector('.sceneMatrixColumnHeader') ?? showColumn;
    if (!(scroller instanceof HTMLElement) || !(showColumn instanceof HTMLElement) || !(showHeader instanceof HTMLElement)) {
      return {
        available: false,
        initialScrollLeft: -1,
        maxScrollLeft: 0,
        reachedScrollLeft: -1,
        reachedMax: false,
        showVisibleAtMax: false,
        showHitColumnAtMax: '',
        resetScrollLeft: -1,
        resetAtOrigin: false,
      };
    }

    scroller.scrollLeft = 0;
    await settle();
    const initialScrollLeft = scroller.scrollLeft;
    const maxScrollLeft = Math.max(0, scroller.scrollWidth - scroller.clientWidth);
    scroller.scrollLeft = maxScrollLeft;
    await settle();
    const reachedScrollLeft = scroller.scrollLeft;
    const scrollerRect = scroller.getBoundingClientRect();
    const showRect = showHeader.getBoundingClientRect();
    const visibleLeft = Math.max(scrollerRect.left, showRect.left);
    const visibleRight = Math.min(scrollerRect.right, showRect.right);
    const visibleTop = Math.max(scrollerRect.top, showRect.top);
    const visibleBottom = Math.min(scrollerRect.bottom, showRect.bottom);
    const showVisibleAtMax = visibleRight - visibleLeft >= 2 && visibleBottom - visibleTop >= 2;
    const showHitColumnAtMax = showVisibleAtMax
      ? document.elementFromPoint(
          visibleLeft + (visibleRight - visibleLeft) / 2,
          visibleTop + (visibleBottom - visibleTop) / 2,
        )?.closest('[data-scene-matrix-column]')?.getAttribute('data-scene-matrix-column') ?? ''
      : '';

    scroller.scrollLeft = 0;
    await settle();
    const resetScrollLeft = scroller.scrollLeft;
    return {
      available: true,
      initialScrollLeft,
      maxScrollLeft,
      reachedScrollLeft,
      reachedMax: maxScrollLeft > 1 && Math.abs(reachedScrollLeft - maxScrollLeft) <= 2,
      showVisibleAtMax,
      showHitColumnAtMax,
      resetScrollLeft,
      resetAtOrigin: Math.abs(resetScrollLeft) <= 1,
    };
  })()`);
}

async function measureSceneMatrixInteractionState(client, cueId) {
  return await client.evaluate(`(() => {
    const markers = [...document.querySelectorAll('.timelineOverview .timelineMarker.sceneBlock[data-timeline-event-id]')]
      .map((marker) => ({
        eventId: marker.getAttribute('data-timeline-event-id') ?? '',
        layerId: Number(marker.getAttribute('data-timeline-layer-id')),
        layerKind: marker.getAttribute('data-timeline-layer-kind') ?? '',
        label: marker.getAttribute('aria-label') ?? '',
      }));
    const cueItem = document.querySelector('.cueItem[data-cue-id="${cueId}"]');
    return {
      cueId: ${cueId},
      markerCount: markers.length,
      markers,
      cuePlacementCount: cueItem?.querySelectorAll('.cueTimelinePlacementChip').length ?? -1,
      activeCardIds: [...document.querySelectorAll('[data-scene-matrix-cue-id]')]
        .filter((card) => card.getAttribute('data-scene-matrix-active') === 'true' && card.classList.contains('active'))
        .map((card) => card.getAttribute('data-scene-matrix-cue-id')),
      ghostPresent: Boolean(document.querySelector('[data-timeline-cue-drag-ghost]')),
    };
  })()`);
}

// T6: the vj-bank fixture carries 14 clips so bank 1 must show all 12 pads
// without scrolling, monitors must own >=65% of the center column, at least
// six layer decks stay visible, and the three settings drawers default closed.
async function runVjBankViewport(client, viewport) {
  await client.send("Emulation.setDeviceMetricsOverride", {
    width: viewport.width,
    height: viewport.height,
    deviceScaleFactor: 1,
    mobile: false,
  });
  await client.send("Page.navigate", { url: fixtureUrl("vj-bank") });
  await waitForApp(client);
  await waitForClientCondition(
    client,
    "document.querySelectorAll('.videoClipPad').length === 12",
    "VJ bank clip pads",
  );
  await sleep(120);
  const containment = await measure(client, `vj-bank-${viewport.width}x${viewport.height}`);
  const conditions = [
    ["vjBankTwelvePadsFullyVisibleWithoutScroll", () =>
      containment.visibleVideoClipPadCount === 12 &&
      containment.fullyVisibleVideoClipPadCount === 12 &&
      containment.videoClipGridScrollDelta <= 1],
    ["vjBankDrawersDefaultClosed", () =>
      containment.mixerDrawerBarCount === 3 && containment.mixerDrawerOpenCount === 0],
    ["vjBankMonitorsDominateCenterColumn", () => containment.videoMixerMonitorHeightRatio >= 0.65],
    ["vjBankSixLayerRowsVisible", () => containment.fullyVisibleVideoLayerItemCount >= 6],
    ["vjBankKillVocabularyPresent", () => containment.visibleMixerKillButtonCount >= 1],
    ["vjBankPagerPresentForSecondBank", () => containment.visibleVideoDeckPagerCount >= 1],
    ["vjBankContained", () => isContained(containment)],
  ];
  const failedChecks = conditions.filter(([, check]) => !check()).map(([name]) => name);
  return {
    viewport,
    label: `vj-bank-${viewport.width}x${viewport.height}`,
    passed: failedChecks.length === 0,
    failedChecks,
    pads: [containment.visibleVideoClipPadCount, containment.fullyVisibleVideoClipPadCount, containment.videoClipGridScrollDelta],
    drawers: [containment.mixerDrawerBarCount, containment.mixerDrawerOpenCount],
    monitorRatio: containment.videoMixerMonitorHeightRatio,
    layerRows: containment.fullyVisibleVideoLayerItemCount,
    killButtons: containment.visibleMixerKillButtonCount,
  };
}

// T12: pane windows and popped-main compaction. Each scenario is a separate
// navigation; the pane-window route collapses the shell to one pane and the
// popped param simulates a pane living in another window.
async function runPaneWindowViewport(client, viewport) {
  const paneWindowCapability = JSON.parse(readFileSync(
    resolve(appRoot, "src-tauri", "capabilities", "main.json"),
    "utf8",
  ));
  const paneWindowDestroyAllowed =
    Array.isArray(paneWindowCapability.windows) &&
    paneWindowCapability.windows.includes("*") &&
    Array.isArray(paneWindowCapability.permissions) &&
    paneWindowCapability.permissions.includes("core:window:allow-destroy");
  await client.send("Emulation.setDeviceMetricsOverride", {
    width: viewport.width,
    height: viewport.height,
    deviceScaleFactor: 1,
    mobile: false,
  });
  const paneWindowStorageSentinel = {
    workspace_tab: "setup",
    setup_sub_tab: "mapping",
    control_mode: "edit",
    timeline_desk_surface: "playback",
    edit_desk_surface: "effects",
    control_category: "color",
    top_split_ratio: 0.47,
    lower_split_ratio: 0.52,
    selections_drawer_open: true,
  };
  const paneWindowStorageRaw = JSON.stringify(paneWindowStorageSentinel);
  await client.evaluate(`window.localStorage.setItem('syndocal.workspaceLayout.v1', ${JSON.stringify(paneWindowStorageRaw)})`);
  const readState = async () => await client.evaluate(`(() => {
      const rect = (sel) => {
        const el = document.querySelector(sel);
        if (!el) return { x: 0, y: 0, w: 0, h: 0, right: 0, bottom: 0 };
        const b = el.getBoundingClientRect();
        return {
          x: Math.round(b.x),
          y: Math.round(b.y),
          w: Math.round(b.width),
          h: Math.round(b.height),
          right: Math.round(b.right),
          bottom: Math.round(b.bottom),
        };
      };
      const doc = document.documentElement;
      const body = document.body;
      const app = document.querySelector('.app');
      const visible = (sel) => {
        const el = document.querySelector(sel);
        if (!el) return false;
        const box = el.getBoundingClientRect();
        const style = getComputedStyle(el);
        return box.width > 0 && box.height > 0 && style.display !== 'none' && style.visibility !== 'hidden';
      };
      return {
        stage: rect('[data-workspace-pane="lower-left"]'),
        context: rect('[data-workspace-pane="lower-right"]'),
        timelineFaders: rect('.controlContextPane > .faders'),
        groups: rect('[data-persistent-band-part="groups"]'),
        topbar: rect('.topbar'),
        band: rect('.mappingPersistentWorkspaceBand'),
        upperSplitter: rect('[data-workspace-splitter="upper-lower"]'),
        lowerSplitter: rect('[data-workspace-splitter="lower-left-right"]'),
        visibleSplitterCount: [...document.querySelectorAll('[data-workspace-splitter]')].filter((element) => visible('[data-workspace-splitter="' + element.getAttribute('data-workspace-splitter') + '"]')).length,
        stageToggle: document.querySelector('[data-pane-popout-toggle="stage"]')?.getAttribute('aria-pressed') ?? '',
        timelineToggle: document.querySelector('[data-pane-popout-toggle="timeline"]')?.getAttribute('aria-pressed') ?? '',
        popoutToggleCount: document.querySelectorAll('[data-pane-popout-toggle]').length,
        contextModeTabCount: document.querySelectorAll('.contextModeTabs').length,
        visibleContextModeTabCount: [...document.querySelectorAll('.contextModeTabs')].filter((element) => {
          const box = element.getBoundingClientRect();
          const style = getComputedStyle(element);
          return box.width > 0 && box.height > 0 && style.display !== 'none' && style.visibility !== 'hidden';
        }).length,
        visiblePanePopoutToggleCount: [...document.querySelectorAll('.panePopoutToggle')].filter((element) => {
          const box = element.getBoundingClientRect();
          const style = getComputedStyle(element);
          return box.width > 0 && box.height > 0 && style.display !== 'none' && style.visibility !== 'hidden';
        }).length,
        controlLiveFixed: Boolean(
          document.querySelector('.layout.layoutControl.controlModeLive') &&
          document.querySelector('.controlContextPane')
        ),
        timelineRejoinToggleCount: document.querySelectorAll('[data-pane-rejoin-toggle="timeline"]').length,
        timelineRejoinToggleVisible: visible('[data-pane-rejoin-toggle="timeline"]'),
        timelineRejoinToggleName:
          document.querySelector('[data-pane-rejoin-toggle="timeline"]')?.getAttribute('aria-label') ?? '',
        paneWindowMode: app?.getAttribute('data-pane-window-mode') ?? '',
        storageRaw: window.localStorage.getItem('syndocal.workspaceLayout.v1') ?? '',
        paneWindowStorageRaw: window.localStorage.getItem('syndocal.paneWindows.v1') ?? '',
        scrollZero:
          window.scrollX === 0 && window.scrollY === 0 &&
          doc.scrollWidth - doc.clientWidth === 0 &&
          doc.scrollHeight - doc.clientHeight === 0 &&
          body.scrollWidth === doc.clientWidth &&
          body.scrollHeight === doc.clientHeight &&
          (!app || (app.scrollWidth === app.clientWidth && app.scrollHeight === app.clientHeight)),
        vw: window.innerWidth,
        vh: window.innerHeight,
      };
    })()`);
  const measureState = async (query, setup) => {
    await client.send("Page.navigate", { url: `${fixtureUrl("timeline")}&${query}` });
    await waitForApp(client);
    if (setup) await setup();
    await sleep(320);
    return await readState();
  };
  const stageWin = await measureState("syndocalPaneWindow=stage");
  const timelineWin = await measureState("syndocalPaneWindow=timeline");
  await client.evaluate(`document.activeElement instanceof HTMLElement && document.activeElement.blur()`);
  const timelineHotkeyStates = [];
  for (const [code, key] of [["KeyE", "e"], ["KeyM", "m"], ["F1", "F1"]]) {
    await pressKey(client, code, key);
    await sleep(60);
    timelineHotkeyStates.push(await readState());
  }
  const timelineEditablePrepared = await client.evaluate(`(() => {
    const visible = (element) => {
      if (!element) return false;
      const rect = element.getBoundingClientRect();
      const style = getComputedStyle(element);
      return rect.width > 0 && rect.height > 0 && style.display !== 'none' && style.visibility !== 'hidden';
    };
    let input = [...document.querySelectorAll('.controlContextPane input[type="search"], .controlContextPane input[type="text"], .controlContextPane input:not([type])')]
      .find(visible);
    if (!input) {
      const sourcePicker = document.querySelector('.controlContextPane .sceneBlockComposerSourceButton');
      if (visible(sourcePicker) && !sourcePicker.disabled) sourcePicker.click();
    }
    return Boolean(input || document.querySelector('.controlContextPane .sceneBlockComposerSourceButton'));
  })()`);
  await sleep(100);
  const timelineEditableFocused = await client.evaluate(`(() => {
    const visible = (element) => {
      if (!element) return false;
      const rect = element.getBoundingClientRect();
      const style = getComputedStyle(element);
      return rect.width > 0 && rect.height > 0 && style.display !== 'none' && style.visibility !== 'hidden';
    };
    const input = [...document.querySelectorAll('.controlContextPane input[type="search"], .controlContextPane input[type="text"], .controlContextPane input:not([type])')]
      .find(visible);
    if (!(input instanceof HTMLInputElement)) return false;
    input.setAttribute('data-viewport-pane-input-probe', 'true');
    const valueSetter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value')?.set;
    valueSetter?.call(input, '');
    input.dispatchEvent(new InputEvent('input', { bubbles: true, inputType: 'deleteContentBackward' }));
    input.focus({ preventScroll: true });
    return document.activeElement === input;
  })()`);
  for (const character of "example") {
    const code = `Key${character.toUpperCase()}`;
    await client.send("Input.dispatchKeyEvent", { type: "rawKeyDown", code, key: character });
    await client.send("Input.dispatchKeyEvent", {
      type: "char",
      code,
      key: character,
      text: character,
      unmodifiedText: character,
    });
    await client.send("Input.dispatchKeyEvent", { type: "keyUp", code, key: character });
  }
  await sleep(100);
  const timelineEditableResult = await client.evaluate(`(() => {
    const input = document.querySelector('[data-viewport-pane-input-probe="true"]');
    return {
      value: input instanceof HTMLInputElement ? input.value : '',
      focused: document.activeElement === input,
      controlLiveFixed: Boolean(document.querySelector('.layout.layoutControl.controlModeLive')),
    };
  })()`);
  const timelineWinAfterInteractions = await readState();
  const poppedMain = await measureState("syndocalPoppedPanes=stage", async () => {
    await clickVisibleByText(client, ".workspaceTabs button", "Control");
    await clickVisibleByText(client, ".controlModeTabs button", "Timeline");
  });
  const poppedTimelineMain = await measureState("syndocalPoppedPanes=timeline", async () => {
    await clickVisibleByText(client, ".workspaceTabs button", "Control");
    // The entire Timeline context pane is intentionally hidden in this state,
    // so select its exact (hidden) mode tab without the global text helper;
    // the new visible rejoin button carries the same "Timeline" text.
    const selectedTimelineMode = await client.evaluate(`(() => {
      const button = [...document.querySelectorAll('.controlModeTabs button')]
        .find((candidate) => (candidate.textContent || '').trim() === 'Timeline');
      button?.click();
      return Boolean(button);
    })()`);
    if (!selectedTimelineMode) throw new Error('Could not select hidden Timeline control mode');
  });
  const timelineRejoinClicked = await clickWorkspaceSelector(client, '[data-pane-rejoin-toggle="timeline"]');
  await sleep(160);
  const rejoinedTimelineMain = await readState();
  const conditions = [
    ["paneWindowStageShowsOnlyLowerLeftUnit", () =>
      stageWin.stage.w >= stageWin.band.w - 4 && stageWin.context.w === 0 && stageWin.groups.w > 0],
    ["paneWindowStageFillsWidthWithoutChrome", () =>
      stageWin.band.w >= stageWin.vw - 20 && stageWin.band.h >= stageWin.vh * 0.75 && stageWin.topbar.w === 0],
    ["paneWindowStageHasNoSplitters", () => stageWin.visibleSplitterCount === 0],
    ["paneWindowStageModeNamed", () => stageWin.paneWindowMode === "stage"],
    ["paneWindowStageScrollZero", () => stageWin.scrollZero === true],
    ["paneWindowTimelineShowsOnlyLowerRightUnit", () =>
      timelineWin.context.w >= timelineWin.band.w - 4 && timelineWin.stage.w === 0 && timelineWin.groups.w === 0],
    ["paneWindowTimelineFillsViewportWithoutChrome", () =>
      timelineWin.band.w >= timelineWin.vw - 20 && timelineWin.band.h >= timelineWin.vh * 0.75 && timelineWin.topbar.w === 0],
    ["paneWindowTimelineDeskFillsContextFromTop", () =>
      timelineWin.timelineFaders.h >= timelineWin.context.h - 4 &&
      Math.abs(timelineWin.timelineFaders.y - timelineWin.context.y) <= 2],
    ["paneWindowNativeDestroyCapability", () => paneWindowDestroyAllowed],
    ["paneWindowTimelineHasNoSplitters", () => timelineWin.visibleSplitterCount === 0],
    ["paneWindowTimelineModeNamed", () => timelineWin.paneWindowMode === "timeline"],
    ["paneWindowTimelineHidesModeTabsAndPopoutToggles", () =>
      timelineWin.visibleContextModeTabCount === 0 && timelineWin.visiblePanePopoutToggleCount === 0],
    ["paneWindowTimelineNonEditableHotkeysStayControlLive", () =>
      timelineWin.controlLiveFixed &&
      timelineHotkeyStates.length === 3 &&
      timelineHotkeyStates.every((state) => state.controlLiveFixed && state.storageRaw === paneWindowStorageRaw)],
    ["paneWindowTimelineEditableInputAcceptsModeLetters", () =>
      timelineEditablePrepared &&
      timelineEditableFocused &&
      timelineEditableResult.focused &&
      timelineEditableResult.value === "example" &&
      timelineEditableResult.controlLiveFixed &&
      timelineWinAfterInteractions.controlLiveFixed &&
      timelineWinAfterInteractions.storageRaw === paneWindowStorageRaw &&
      timelineWinAfterInteractions.scrollZero],
    ["paneWindowRoutesHideTimelineRejoin", () =>
      !stageWin.timelineRejoinToggleVisible && !timelineWin.timelineRejoinToggleVisible],
    ["paneWindowTimelineScrollZero", () => timelineWin.scrollZero === true],
    ["paneWindowDoesNotOverwriteMainLayoutStorage", () =>
      stageWin.storageRaw === paneWindowStorageRaw && timelineWin.storageRaw === paneWindowStorageRaw],
    ["stagePoppedMainHidesLowerLeftAndSplitter", () =>
      poppedMain.stage.w === 0 &&
      poppedMain.groups.w === 0 &&
      poppedMain.lowerSplitter.w === 0],
    ["stagePoppedMainTimelineRefillsLowerBand", () => poppedMain.context.w >= poppedMain.band.w - 4],
    ["timelinePoppedMainHidesLowerRightAndSplitter", () =>
      poppedTimelineMain.context.w === 0 && poppedTimelineMain.lowerSplitter.w === 0],
    ["timelinePoppedMainStageRefillsLowerBand", () =>
      poppedTimelineMain.stage.w >= poppedTimelineMain.band.w - 4 && poppedTimelineMain.groups.w > 0],
    ["timelinePoppedMainRejoinVisibleAndNamed", () =>
      poppedTimelineMain.timelineRejoinToggleCount === 1 &&
      poppedTimelineMain.timelineRejoinToggleVisible === true &&
      poppedTimelineMain.timelineRejoinToggleName.trim().length > 0],
    ["timelinePoppedMainRejoinClickRestoresPane", () =>
      timelineRejoinClicked &&
      rejoinedTimelineMain.context.w > 0 &&
      rejoinedTimelineMain.stage.w > 0 &&
      rejoinedTimelineMain.groups.w > 0 &&
      rejoinedTimelineMain.lowerSplitter.w > 0 &&
      rejoinedTimelineMain.timelineToggle === "false" &&
      rejoinedTimelineMain.timelineRejoinToggleVisible === false],
    ["timelinePoppedMainRejoinClearsChildWindowState", () =>
      rejoinedTimelineMain.paneWindowStorageRaw === "[]"],
    ["timelinePoppedMainRejoinKeepsOuterScrollZero", () => rejoinedTimelineMain.scrollZero === true],
    ["poppedMainToggleStatesPressed", () =>
      poppedMain.stageToggle === "true" &&
      poppedTimelineMain.timelineToggle === "true" &&
      poppedMain.popoutToggleCount === 2 &&
      poppedTimelineMain.popoutToggleCount === 2],
    ["poppedMainKeepsOnlyUpperSplitter", () =>
      poppedMain.visibleSplitterCount === 1 && poppedTimelineMain.visibleSplitterCount === 1],
    ["paneWindowAndPoppedMainKeepOuterScrollZero", () =>
      stageWin.scrollZero && timelineWin.scrollZero && poppedMain.scrollZero && poppedTimelineMain.scrollZero && rejoinedTimelineMain.scrollZero],
  ];
  const failedChecks = conditions.filter(([, check]) => {
    try { return !check(); } catch { return true; }
  }).map(([name]) => name);
  return {
    viewport,
    label: `pane-window-${viewport.width}x${viewport.height}`,
    passed: failedChecks.length === 0,
    failedChecks,
    stageWin: [stageWin.stage.w, stageWin.stage.h, stageWin.topbar.w],
    timelineWin: [
      timelineWin.context.w,
      timelineWin.context.h,
      timelineWin.timelineFaders.y - timelineWin.context.y,
      timelineWin.timelineFaders.h,
    ],
    timelineLock: {
      modeControls: [timelineWin.contextModeTabCount, timelineWin.visibleContextModeTabCount],
      popoutControls: [timelineWin.popoutToggleCount, timelineWin.visiblePanePopoutToggleCount],
      hotkeys: timelineHotkeyStates.map((state) => state.controlLiveFixed),
      editable: timelineEditableResult,
    },
    poppedMain: [poppedMain.stage.w, poppedMain.context.w, poppedMain.lowerSplitter.w],
    poppedTimelineMain: [poppedTimelineMain.stage.w, poppedTimelineMain.context.w, poppedTimelineMain.lowerSplitter.w],
    rejoinedTimelineMain: [
      rejoinedTimelineMain.stage.w,
      rejoinedTimelineMain.context.w,
      rejoinedTimelineMain.lowerSplitter.w,
      rejoinedTimelineMain.paneWindowStorageRaw,
    ],
    checks: Object.fromEntries(conditions.map(([name, check]) => {
      try { return [name, Boolean(check())]; } catch { return [name, false]; }
    })),
  };
}

async function runSceneMatrixPaneCheck(client, viewport) {
  await client.send("Emulation.setDeviceMetricsOverride", {
    width: viewport.width,
    height: viewport.height,
    deviceScaleFactor: 1,
    mobile: false,
  });
  await client.send("Page.navigate", { url: fixtureUrl("scene-matrix") });
  await waitForApp(client);
  await clickVisibleByText(client, ".workspaceTabs button", "Control");
  await clickVisibleByText(client, ".controlModeTabs button", "Timeline");
  await clickVisibleByText(client, ".timelineDeskTabs button", "Show");
  await sleep(120);
  const before = await measureSceneMatrixPane(client);
  const horizontalScroll = await exerciseSceneMatrixHorizontalScroll(client);
  await clickVisibleByText(client, ".liveDeskViewToggle button", "Cue Pads");
  await sleep(80);
  const alternateView = await client.evaluate(`(() => {
    const visible = (selector) => [...document.querySelectorAll(selector)].filter((element) => {
      const rect = element.getBoundingClientRect();
      const style = getComputedStyle(element);
      return rect.width > 0 && rect.height > 0 && style.display !== 'none' && style.visibility !== 'hidden';
    }).length;
    return {
      matrixCount: visible('.liveControlPanel > .sceneMatrixPanel'),
      cuePadSurfaceCount: visible('.liveControlPanel > .liveCuePadSurface'),
      activeView: ([...document.querySelectorAll('.liveDeskViewToggle button')]
        .find((button) => button.getAttribute('aria-pressed') === 'true')?.textContent || '').trim(),
    };
  })()`);
  await clickVisibleByText(client, ".liveDeskViewToggle button", "Matrix");
  await sleep(80);
  const interactionGeometry = await client.evaluate(`(() => {
    const clickSource = document.querySelector('[data-scene-matrix-cue-id="302"] .sceneMatrixTrigger');
    const dragSource = document.querySelector('[data-scene-matrix-cue-id="303"] .cueTimelineDragHandle');
    clickSource?.scrollIntoView({ block: 'nearest', inline: 'nearest' });
    dragSource?.scrollIntoView({ block: 'nearest', inline: 'nearest' });
    const lane = [...document.querySelectorAll('.timelineOverview [data-timeline-layer-kind="Lighting"][data-timeline-layer-id]')]
      .filter((element) => {
        const rect = element.getBoundingClientRect();
        return rect.width > 80 && rect.height > 10;
      })
      .sort((left, right) => right.getBoundingClientRect().width - left.getBoundingClientRect().width)[0];
    if (!clickSource || !dragSource || !lane) return null;
    const visibleHitPoint = (source, cueId, requireHandle = false) => {
      const rect = source.getBoundingClientRect();
      for (const yRatio of [0.5, 0.25, 0.75]) {
        for (const xRatio of [0.45, 0.25, 0.7]) {
          const x = rect.left + rect.width * xRatio;
          const y = rect.top + rect.height * yRatio;
          const hitElement = document.elementFromPoint(x, y);
          const hitCueId = hitElement
            ?.closest('[data-scene-matrix-cue-id]')
            ?.getAttribute('data-scene-matrix-cue-id');
          const handleHit = hitElement?.closest('.cueTimelineDragHandle') === source;
          if (hitCueId === cueId && (!requireHandle || handleHit)) return { x, y, hitCueId, handleHit };
        }
      }
      return null;
    };
    const clickPoint = visibleHitPoint(clickSource, '302');
    const dragPoint = visibleHitPoint(dragSource, '303', true);
    if (!clickPoint || !dragPoint) return null;
    const laneRect = lane.getBoundingClientRect();
    return {
      clickSourceX: clickPoint.x,
      clickSourceY: clickPoint.y,
      clickHitCueId: clickPoint.hitCueId,
      dragSourceX: dragPoint.x,
      dragSourceY: dragPoint.y,
      dragHitCueId: dragPoint.hitCueId,
      dragHandleHit: dragPoint.handleHit,
      laneX: laneRect.left + laneRect.width * 0.6,
      laneY: laneRect.top + laneRect.height / 2,
      laneId: Number(lane.getAttribute('data-timeline-layer-id')),
      laneKind: lane.getAttribute('data-timeline-layer-kind') ?? '',
    };
  })()`);
  let subThresholdClick = null;
  if (interactionGeometry) {
    const stateBefore = await measureSceneMatrixInteractionState(client, 302);
    await client.evaluate(`(() => {
      const eventTypes = ['pointerdown', 'pointermove', 'pointerup', 'pointercancel', 'click'];
      window.__sceneMatrixTinyTrace = [];
      window.__sceneMatrixTinyTraceHandlers = Object.fromEntries(eventTypes.map((type) => {
        const handler = (event) => {
          const target = event.target instanceof Element ? event.target.closest('[data-scene-matrix-cue-id]') : null;
          if (target?.getAttribute('data-scene-matrix-cue-id') !== '302') return;
          window.__sceneMatrixTinyTrace.push({
            type,
            target: event.target instanceof Element ? event.target.className : '',
            defaultPrevented: event.defaultPrevented,
          });
        };
        document.addEventListener(type, handler, true);
        return [type, handler];
      }));
    })()`);
    await client.send("Input.dispatchMouseEvent", {
      type: "mousePressed",
      x: interactionGeometry.clickSourceX,
      y: interactionGeometry.clickSourceY,
      button: "left",
      buttons: 1,
      clickCount: 1,
    });
    await client.send("Input.dispatchMouseEvent", {
      type: "mouseMoved",
      x: interactionGeometry.clickSourceX + 3,
      y: interactionGeometry.clickSourceY,
      button: "left",
      buttons: 1,
    });
    await sleep(30);
    const ghostDuringMove = await client.evaluate(`Boolean(document.querySelector('[data-timeline-cue-drag-ghost]'))`);
    await client.send("Input.dispatchMouseEvent", {
      type: "mouseReleased",
      x: interactionGeometry.clickSourceX + 3,
      y: interactionGeometry.clickSourceY,
      button: "left",
      buttons: 0,
      clickCount: 1,
    });
    await sleep(120);
    const eventTrace = await client.evaluate(`(() => {
      const trace = window.__sceneMatrixTinyTrace ?? [];
      for (const [type, handler] of Object.entries(window.__sceneMatrixTinyTraceHandlers ?? {})) {
        document.removeEventListener(type, handler, true);
      }
      delete window.__sceneMatrixTinyTrace;
      delete window.__sceneMatrixTinyTraceHandlers;
      return trace;
    })()`);
    subThresholdClick = {
      movedPx: 3,
      hitCueId: interactionGeometry.clickHitCueId,
      ghostDuringMove,
      eventTrace,
      before: stateBefore,
      after: await measureSceneMatrixInteractionState(client, 302),
    };
  }
  let oneGestureDrag = null;
  if (interactionGeometry) {
    const stateBefore = await measureSceneMatrixInteractionState(client, 303);
    await client.send("Input.dispatchMouseEvent", {
      type: "mousePressed",
      x: interactionGeometry.dragSourceX,
      y: interactionGeometry.dragSourceY,
      button: "left",
      buttons: 1,
      clickCount: 1,
    });
    await client.send("Input.dispatchMouseEvent", {
      type: "mouseMoved",
      x: interactionGeometry.dragSourceX + 6,
      y: interactionGeometry.dragSourceY,
      button: "left",
      buttons: 1,
    });
    await client.send("Input.dispatchMouseEvent", {
      type: "mouseMoved",
      x: interactionGeometry.laneX,
      y: interactionGeometry.laneY,
      button: "left",
      buttons: 1,
    });
    await sleep(50);
    const during = await client.evaluate(`(() => ({
      dropState: document.querySelector('[data-timeline-cue-drag-ghost]')?.getAttribute('data-timeline-drop-state') ?? '',
      sourceStillVisible: (() => {
        const source = document.querySelector('[data-scene-matrix-cue-id="303"]');
        const rect = source?.getBoundingClientRect();
        return Boolean(rect && rect.width > 0 && rect.height > 0);
      })(),
      matrixStillVisible: Boolean(document.querySelector('.liveControlPanel > .sceneMatrixPanel')),
    }))()`);
    await client.send("Input.dispatchMouseEvent", {
      type: "mouseReleased",
      x: interactionGeometry.laneX,
      y: interactionGeometry.laneY,
      button: "left",
      buttons: 0,
      clickCount: 1,
    });
    await sleep(180);
    const stateAfter = await measureSceneMatrixInteractionState(client, 303);
    const beforeEventIds = new Set(stateBefore.markers.map((marker) => marker.eventId));
    oneGestureDrag = {
      ...during,
      sourceCueId: 303,
      sourceCueLabel: "Back Sweep",
      hitCueId: interactionGeometry.dragHitCueId,
      sourceHandleHit: interactionGeometry.dragHandleHit,
      targetLayerId: interactionGeometry.laneId,
      targetLayerKind: interactionGeometry.laneKind,
      before: stateBefore,
      after: stateAfter,
      addedMarkers: stateAfter.markers.filter((marker) => !beforeEventIds.has(marker.eventId)),
      ghostCleared: !stateAfter.ghostPresent,
    };
  }
  const after = await measureSceneMatrixPane(client);
  const expectedColumns = [
    "front",
    "back",
    ...Array.from({ length: 10 }, (_, index) => `bank-${String(index + 3).padStart(2, "0")}`),
    "Show",
  ];
  // T17 added the flash-mode cue 320 to the "back" column of the fixture.
  const expectedCardCount = 15;
  const expectedReplaceCards = ["302", "303"];
  const validHueAttributes = (values, expectedCount) =>
    values.length === expectedCount && values.every((value) => {
      if (typeof value !== "string" || value.trim() === "") return false;
      const hue = Number(value);
      return Number.isFinite(hue) && hue >= 0 && hue <= 359;
    });
  const conditions = [
    ["matrixPrimarySurfaceVisibleByDefault", () =>
      before.paneVisible &&
      after.paneVisible &&
      before.paneInPrimaryLiveDesk &&
      before.paneHeightCoverage >= 0.35 &&
      before.activeLiveView === "Matrix"],
    ["matrixTimelineLiveOwnsPrimaryWidth", () =>
      before.paneWidthCoverage >= 0.6 &&
      before.paneRect?.width > before.statusRect?.width],
    ["matrixTimelineLiveDoesNotOverlapStatus", () => before.statusVisible && !before.paneStatusOverlap],
    ["matrixCuePadsRemainAlternateView", () =>
      before.liveViewButtonCount === 2 &&
      alternateView.matrixCount === 0 &&
      alternateView.cuePadSurfaceCount === 1 &&
      alternateView.activeView === "Cue Pads"],
    ["matrixExpectedGroupColumns", () =>
      JSON.stringify(before.columns) === JSON.stringify(expectedColumns) && before.cardIds.length === expectedCardCount],
    ["matrixHeaderHueAttributesPresent", () => validHueAttributes(before.headerHues, expectedColumns.length)],
    ["matrixCardHueAttributesPresent", () => validHueAttributes(before.cardHues, expectedCardCount)],
    ["matrixBankIdentityStripsVisible", () =>
      before.bankStripCount === expectedColumns.length && before.bankStripColors.every((color) => color !== "rgba(0, 0, 0, 0)")],
    ["matrixStaticAndFxBadgesPresent", () =>
      before.kindBadgeCounts.STATIC === 14 && before.kindBadgeCounts.FX === 1],
    ["matrixReplaceGroupBadgesPresent", () => JSON.stringify(before.replaceCardIds.sort()) === JSON.stringify(expectedReplaceCards)],
    ["matrixInitialActiveCueVisible", () => JSON.stringify(before.activeCardIds) === JSON.stringify(["301"])],
    ["matrixActiveCueProgressVisible", () =>
      before.progressBarCount === 1 && before.progressValues.length === 1 && Math.abs(before.progressValues[0] - 0.42) <= 0.001],
    ["matrixCellsMeetFortyPixelHitArea", () => before.minCellHitSize >= 40],
    ["matrixDragHandlesMeetFortyPixelTouchContract", () =>
      before.dragHandleCount === expectedCardCount &&
      before.minDragHandleHitSize >= 40 &&
      before.dragHandleTouchActions.every((touchAction) => touchAction === "none")],
    ["matrixEditSourceButtonsNameTheirCue", () =>
      before.editCueButtons.length === expectedCardCount &&
      before.editCueButtons.every((entry) =>
        entry.rendered && entry.label.length > 0 && entry.name === `Edit Source for Cue ${entry.label}`) &&
      new Set(before.editCueButtons.map((entry) => entry.name)).size === expectedCardCount],
    ["matrixVisibleTextMeetsElevenPixelFloor", () => before.matrixMinFontPx >= 11],
    ["matrixSubThresholdMoveRecallsWithoutTimelineMutation", () =>
      subThresholdClick?.movedPx < 4 &&
      subThresholdClick.hitCueId === "302" &&
      !subThresholdClick.ghostDuringMove &&
      JSON.stringify(subThresholdClick.before.activeCardIds) === JSON.stringify(["301"]) &&
      JSON.stringify(subThresholdClick.after.activeCardIds) === JSON.stringify(["302"]) &&
      subThresholdClick.after.markerCount === subThresholdClick.before.markerCount &&
      (subThresholdClick.before.cuePlacementCount < 0
        ? subThresholdClick.after.cuePlacementCount < 0
        : subThresholdClick.after.cuePlacementCount === subThresholdClick.before.cuePlacementCount)],
    ["matrixAndTimelineLaneCoexistForOneGestureDrag", () =>
      before.dragSourceCount === expectedCardCount && before.timelineShowSurfaceVisible && before.visibleTimelineLaneCount > 0],
    ["matrixOneGestureDragUsesTimelineDropPath", () =>
      oneGestureDrag?.dropState === "valid" &&
      oneGestureDrag.sourceStillVisible &&
      oneGestureDrag.matrixStillVisible &&
      oneGestureDrag.ghostCleared &&
      oneGestureDrag.sourceCueId === 303 &&
      oneGestureDrag.hitCueId === "303" &&
      oneGestureDrag.sourceHandleHit === true &&
      oneGestureDrag.targetLayerKind === "Lighting" &&
      oneGestureDrag.after.markerCount === oneGestureDrag.before.markerCount + 1 &&
      (oneGestureDrag.before.cuePlacementCount < 0
        ? oneGestureDrag.after.cuePlacementCount < 0
        : oneGestureDrag.after.cuePlacementCount === oneGestureDrag.before.cuePlacementCount + 1) &&
      oneGestureDrag.addedMarkers.length === 1 &&
      oneGestureDrag.addedMarkers[0].layerId === oneGestureDrag.targetLayerId &&
      oneGestureDrag.addedMarkers[0].layerKind === "Lighting" &&
      oneGestureDrag.addedMarkers[0].label.includes(oneGestureDrag.sourceCueLabel)],
    ["matrixDragDoesNotRecallCue", () =>
      JSON.stringify(oneGestureDrag?.before.activeCardIds) === JSON.stringify(["302"]) &&
      JSON.stringify(oneGestureDrag?.after.activeCardIds) === JSON.stringify(["302"])],
    ["matrixActiveCueHighlightFollowedSubThresholdClick", () => JSON.stringify(after.activeCardIds) === JSON.stringify(["302"])],
    ["matrixColumnsUseInternalScrollport", () =>
      before.internalScrollport &&
      after.internalScrollport &&
      horizontalScroll.available &&
      Math.abs(horizontalScroll.initialScrollLeft) <= 1 &&
      horizontalScroll.maxScrollLeft > 1 &&
      horizontalScroll.reachedMax &&
      horizontalScroll.showVisibleAtMax &&
      horizontalScroll.showHitColumnAtMax === "Show" &&
      horizontalScroll.resetAtOrigin],
    ["matrixDocumentAndAppScrollZero", () => before.documentAndAppScrollZero && after.documentAndAppScrollZero],
    // T7: the fixture persists #ff3366 on cue 301 (hue 345) and #22aa88 on the
    // back group (hue 165); persisted colors must win over the hash palette
    // while the header hue ATTRIBUTE keeps the deterministic hash value.
    ["matrixPersistedCueColorWinsHashHue", () =>
      (before.coloredCardIdentity || "").startsWith("hsl(345")],
    ["matrixPersistedGroupColorWinsHashHue", () =>
      (before.backHeaderIdentity || "").startsWith("hsl(165")],
    ["matrixGroupColorPickerPerGroupColumn", () => before.groupColorInputCount === expectedColumns.length - 1],
  ];
  const checks = Object.fromEntries(conditions.map(([name, check]) => {
    try {
      return [name, Boolean(check())];
    } catch {
      return [name, false];
    }
  }));
  const failedChecks = Object.entries(checks).filter(([, passed]) => !passed).map(([name]) => name);
  return {
    label: `scene-matrix-${viewport.width}x${viewport.height}`,
    passed: failedChecks.length === 0,
    checks,
    failedChecks,
    before,
    after,
    horizontalScroll,
    alternateView,
    subThresholdClick,
    oneGestureDrag,
  };
}

// T15: the Timeline slim pass deliberately reuses the T14 Scene Matrix
// interaction fixture. That keeps the 4px click/drag boundary on the exact
// production path while the visual assertions inspect the resulting Show
// surface at every supported viewport.
async function measureTimelineSlimVisual(client) {
  return await client.evaluate(`(() => {
    const isRendered = (element) => {
      if (!(element instanceof Element)) return false;
      const rect = element.getBoundingClientRect();
      const style = getComputedStyle(element);
      return rect.width > 0 && rect.height > 0 && style.display !== 'none' && style.visibility !== 'hidden';
    };
    const semanticText = (root) => {
      const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
      const chunks = [];
      for (let node = walker.nextNode(); node; node = walker.nextNode()) {
        const parent = node.parentElement;
        if (!parent || parent.closest('svg, [data-icon], .icon')) continue;
        const style = getComputedStyle(parent);
        if (style.display === 'none' || style.visibility === 'hidden') continue;
        const value = node.textContent?.trim();
        if (parent.closest('[aria-hidden="true"]') && value && value.length <= 3) continue;
        if (value) chunks.push(value);
      }
      return chunks.join(' ').trim();
    };
    const hasIcon = (button) => {
      if (button.querySelector('svg, [data-icon], .icon')) return true;
      return [...button.querySelectorAll('[aria-hidden="true"]')].some((element) => {
        const glyph = (element.textContent || '').trim();
        return glyph.length > 0 && glyph.length <= 3;
      });
    };
    const accessibleName = (element) => (
      element.getAttribute('aria-label') || element.getAttribute('title') || ''
    ).trim();
    const parseColor = (value) => {
      const match = String(value || '').match(/rgba?\\(([^)]+)\\)/i);
      if (!match) return null;
      const parts = match[1].trim().split(/[\\s,\\/]+/).filter(Boolean).map(Number);
      if (parts.length < 3 || parts.slice(0, 3).some((part) => !Number.isFinite(part))) return null;
      return {
        r: parts[0],
        g: parts[1],
        b: parts[2],
        a: Number.isFinite(parts[3]) ? parts[3] : 1,
      };
    };
    const normalizeColor = (value) => {
      if (!String(value || '').trim()) return null;
      const probe = document.createElement('span');
      probe.style.position = 'fixed';
      probe.style.pointerEvents = 'none';
      probe.style.color = String(value);
      document.body.append(probe);
      const normalized = parseColor(getComputedStyle(probe).color);
      probe.remove();
      return normalized;
    };
    const sameRgb = (left, right, tolerance = 2) => Boolean(
      left && right &&
      Math.abs(left.r - right.r) <= tolerance &&
      Math.abs(left.g - right.g) <= tolerance &&
      Math.abs(left.b - right.b) <= tolerance
    );
    const relativeLuminance = (color) => {
      if (!color) return Number.POSITIVE_INFINITY;
      const linear = [color.r, color.g, color.b].map((channel) => {
        const value = channel / 255;
        return value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4;
      });
      return linear[0] * 0.2126 + linear[1] * 0.7152 + linear[2] * 0.0722;
    };
    const lineInk = (line) => {
      const style = getComputedStyle(line);
      const color = parseColor(style.stroke);
      if (!color) return Number.POSITIVE_INFINITY;
      const opacity = (Number.parseFloat(style.opacity) || 1) *
        (Number.parseFloat(style.strokeOpacity) || 1) * color.a;
      return relativeLuminance(color) * opacity;
    };
    const isPaintedLine = (line) => {
      if (!(line instanceof SVGLineElement)) return false;
      const style = getComputedStyle(line);
      return style.display !== 'none' && style.visibility !== 'hidden' &&
        (line.x1.baseVal.value !== line.x2.baseVal.value || line.y1.baseVal.value !== line.y2.baseVal.value);
    };
    const maxOrMissing = (values) => values.length > 0 ? Math.max(...values) : -1;

    const deskTabs = [...document.querySelectorAll('.timelineDeskTabs button')].filter(isRendered);
    const cuesDeskTabs = deskTabs.filter((button) => (button.textContent || '').trim().toLowerCase() === 'cues');

    const toolbarRoots = [...document.querySelectorAll(
      '.timelineViewportToolbar, .timelineDirectToolbar, [data-timeline-slim-toolbar]'
    )].filter(isRendered);
    const toolbarButtons = [...new Set(toolbarRoots.flatMap((toolbar) =>
      [...toolbar.querySelectorAll('button')].filter(isRendered)))];
    const toolbarAriaButtonCount = toolbarButtons.filter((button) => accessibleName(button).length > 0).length;
    const toolbarIconOnlyButtonCount = toolbarButtons.filter((button) =>
      hasIcon(button) && semanticText(button) === '').length;
    const toolbarOverflowCount = toolbarButtons.filter((button) => {
      const toolbar = button.closest('.timelineViewportToolbar, .timelineDirectToolbar, [data-timeline-slim-toolbar]');
      if (!toolbar) return true;
      const buttonRect = button.getBoundingClientRect();
      const toolbarRect = toolbar.getBoundingClientRect();
      return buttonRect.left < toolbarRect.left - 1 || buttonRect.right > toolbarRect.right + 1;
    }).length;

    const gutters = [...document.querySelectorAll('.timelineUserLaneGutter')].filter(isRendered);
    const gutterRows = gutters.map((gutter) => {
      const buttons = [...gutter.querySelectorAll('button')].filter(isRendered);
      const names = [...gutter.querySelectorAll('.timelineLaneGutterName')].filter(isRendered);
      const numberElements = [...gutter.querySelectorAll('[data-timeline-lane-number], .timelineLaneGutterCount')]
        .filter(isRendered)
        .filter((element) => /^\\d+$/.test((element.textContent || '').trim()));
      const numberElement = numberElements[0] ?? null;
      const numberHasLaneSemantics = Boolean(numberElement && (
        numberElement.hasAttribute('data-timeline-lane-number') ||
        /(?:lane|track)\\s*#?\\d+/i.test(accessibleName(numberElement))
      ));
      const controlButtons = buttons.filter((button) => button !== numberElement);
      const labels = controlButtons.map(accessibleName);
      const expandCount = controlButtons.filter((button, index) =>
        /(?:expand|collapse|details|open|close)/i.test(labels[index]) ||
        button.hasAttribute('data-timeline-lane-expand-toggle') ||
        button.hasAttribute('data-timeline-layer-expand-toggle')).length;
      const forbiddenPlayCount = labels.filter((label) => /(?:play|run|go|trigger)/i.test(label)).length;
      const visibilityCount = labels.filter((label) => /(?:visibility|visible|show|hide|mute|unmute)/i.test(label)).length;
      const lockCount = labels.filter((label) => /(?:lock|unlock)/i.test(label)).length;
      const soloCount = labels.filter((label) => /solo/i.test(label)).length;
      const iconOnlyButtonCount = controlButtons.filter((button) =>
        hasIcon(button) && semanticText(button) === '').length;
      const section = gutter.closest('.timelineLayerSection');
      const sectionHeader = section?.querySelector('.timelineLayerSectionHeader');
      const gutterStyle = getComputedStyle(gutter);
      const sectionHeaderStyle = sectionHeader ? getComputedStyle(sectionHeader) : null;
      const typeAccent = (
        gutterStyle.getPropertyValue('--timeline-lane-kind-color') ||
        gutterStyle.getPropertyValue('--timeline-layer-kind-color') ||
        sectionHeaderStyle?.borderLeftColor ||
        gutterStyle.borderLeftColor ||
        gutterStyle.backgroundColor
      ).trim();
      return {
        kind: gutter.getAttribute('data-timeline-layer-kind') || '',
        id: gutter.getAttribute('data-timeline-layer-id') || '',
        accessibleName: accessibleName(gutter),
        buttonCount: buttons.length,
        controlButtonCount: controlButtons.length,
        numberCount: numberElements.length,
        numberHasLaneSemantics,
        expandCount,
        forbiddenPlayCount,
        visibilityCount,
        lockCount,
        soloCount,
        visibleNameCount: names.length,
        iconOnlyButtonCount,
        typeAccent,
        exactFourElements:
          numberElements.length === 1 &&
          numberHasLaneSemantics &&
          controlButtons.length === 3 &&
          expandCount === 1 &&
          forbiddenPlayCount === 0 &&
          visibilityCount === 1 &&
          lockCount === 1 &&
          soloCount === 0 &&
          iconOnlyButtonCount === controlButtons.length,
      };
    });
    const gutterKinds = new Map();
    for (const row of gutterRows) {
      if (!row.kind || !row.typeAccent || row.typeAccent === 'rgba(0, 0, 0, 0)') continue;
      if (!gutterKinds.has(row.kind)) gutterKinds.set(row.kind, row.typeAccent);
    }
    const sectionKindSummaries = [...document.querySelectorAll('.timelineLayerSectionKindIcon[role="img"]')]
      .filter(isRendered)
      .map((icon) => ({
        kind: (icon.getAttribute('title') || '').trim(),
        name: accessibleName(icon),
      }));

    const markers = [...document.querySelectorAll('.timelineOverview .timelineMarker.sceneBlock')]
      .filter((marker) => isRendered(marker.querySelector('.timelineSceneBlockBody')));
    const baseStyleMarkers = markers.filter((marker) =>
      !marker.classList.contains('selected') &&
      !marker.classList.contains('underPlayhead') &&
      !marker.classList.contains('dragging'));
    const styledMarkers = baseStyleMarkers.length > 0 ? baseStyleMarkers : markers;
    const blockRows = styledMarkers.map((marker) => {
      const body = marker.querySelector('.timelineSceneBlockBody');
      const band = marker.querySelector('.timelineSceneBlockIdentityBand');
      const label = marker.querySelector('.timelineSceneBlockLabel');
      const duration = marker.querySelector('.timelineSceneBlockDuration');
      const bodyRect = body.getBoundingClientRect();
      const bodyStyle = getComputedStyle(body);
      const bodyFill = normalizeColor(bodyStyle.fill);
      const identity = normalizeColor(getComputedStyle(marker).getPropertyValue('--identity'));
      const bandFill = band ? normalizeColor(getComputedStyle(band).fill) : bodyFill;
      const stroke = parseColor(bodyStyle.stroke);
      const labelRect = label?.getBoundingClientRect() ?? null;
      const durationRect = duration?.getBoundingClientRect() ?? null;
      const twoLineEligible = bodyRect.width >= 80;
      const lineFontSizes = [label, duration]
        .filter(Boolean)
        .map((element) => Number.parseFloat(getComputedStyle(element).fontSize) || 0);
      const twoLine = Boolean(
        twoLineEligible &&
        label && duration && isRendered(label) && isRendered(duration) &&
        (label.textContent || '').trim().length > 0 &&
        (duration.textContent || '').trim().length > 0 &&
        labelRect && durationRect &&
        labelRect.bottom <= durationRect.top + 1 &&
        labelRect.top >= bodyRect.top - 1 && durationRect.bottom <= bodyRect.bottom + 1
      );
      return {
        height: bodyRect.height,
        width: bodyRect.width,
        identityFill: sameRgb(bodyFill, identity),
        solidIdentityBand: sameRgb(bodyFill, bandFill),
        fillOpacity: Number.parseFloat(bodyStyle.fillOpacity) || 1,
        opaqueBlackBorder: Boolean(
          stroke && stroke.r <= 36 && stroke.g <= 36 && stroke.b <= 36 && stroke.a >= 0.85 &&
          (Number.parseFloat(bodyStyle.strokeWidth) || 0) >= 1
        ),
        twoLineEligible,
        twoLine,
        minLineFontPx: lineFontSizes.length > 0 ? Math.min(...lineFontSizes) : 0,
      };
    });
    const eligibleTwoLineRows = blockRows.filter((row) => row.twoLineEligible);
    const minBlockHeight = blockRows.length > 0 ? Math.min(...blockRows.map((row) => row.height)) : 0;
    const minTwoLineFontPx = eligibleTwoLineRows.length > 0
      ? Math.min(...eligibleTwoLineRows.map((row) => row.minLineFontPx))
      : 0;

    const minorGridLines = [...document.querySelectorAll('.timelineRuler line:not(.major)')].filter(isPaintedLine);
    const majorGridLines = [...document.querySelectorAll('.timelineRuler line.major')].filter(isPaintedLine);
    const dividerLines = [...document.querySelectorAll('.timelineLaneDivider, .timelineSectionDivider')].filter(isPaintedLine);
    const documentElement = document.documentElement;
    const body = document.body;
    const app = document.querySelector('.app');
    const documentAndAppScrollZero =
      window.scrollX === 0 && window.scrollY === 0 &&
      documentElement.scrollWidth === documentElement.clientWidth &&
      documentElement.scrollHeight === documentElement.clientHeight &&
      body.scrollWidth === documentElement.clientWidth &&
      body.scrollHeight === documentElement.clientHeight &&
      (!app || (app.scrollWidth === app.clientWidth && app.scrollHeight === app.clientHeight));

    return {
      deskTabLabels: deskTabs.map((button) => (button.textContent || '').trim()),
      deskTabSurfaceIds: deskTabs.map((button) => button.getAttribute('data-timeline-desk-surface') || ''),
      deskTabPressedStates: deskTabs.map((button) => button.getAttribute('aria-pressed') || ''),
      deskTabActiveStates: deskTabs.map((button) => button.classList.contains('active')),
      cuesDeskTabCount: cuesDeskTabs.length,
      toolbarRootCount: toolbarRoots.length,
      toolbarButtonCount: toolbarButtons.length,
      toolbarAriaButtonCount,
      toolbarIconOnlyButtonCount,
      toolbarOverflowCount,
      toolbarButtonNames: toolbarButtons.map((button) => ({
        name: accessibleName(button),
        text: semanticText(button),
        icon: hasIcon(button),
      })),
      toolbarToolIds: toolbarButtons.map((button) => button.getAttribute('data-timeline-tool') || ''),
      gutterCount: gutterRows.length,
      gutterRows,
      visibleGutterNameCount: gutterRows.reduce((sum, row) => sum + row.visibleNameCount, 0),
      gutterKindCount: gutterKinds.size,
      gutterKindAccents: Object.fromEntries(gutterKinds),
      sectionKindSummaries,
      blockCount: blockRows.length,
      minBlockHeight,
      thickBlockCount: blockRows.filter((row) => row.height >= 26).length,
      identityFillCount: blockRows.filter((row) => row.identityFill && row.fillOpacity >= 0.9).length,
      solidIdentityBandCount: blockRows.filter((row) => row.solidIdentityBand).length,
      opaqueBlackBorderCount: blockRows.filter((row) => row.opaqueBlackBorder).length,
      twoLineEligibleCount: eligibleTwoLineRows.length,
      twoLineBlockCount: eligibleTwoLineRows.filter((row) => row.twoLine).length,
      minTwoLineFontPx,
      minorGridLineCount: minorGridLines.length,
      majorGridLineCount: majorGridLines.length,
      dividerLineCount: dividerLines.length,
      maxMinorGridInk: maxOrMissing(minorGridLines.map(lineInk)),
      maxMajorGridInk: maxOrMissing(majorGridLines.map(lineInk)),
      maxDividerInk: maxOrMissing(dividerLines.map(lineInk)),
      documentAndAppScrollZero,
      scrollMetrics: {
        document: [documentElement.scrollWidth, documentElement.clientWidth, documentElement.scrollHeight, documentElement.clientHeight],
        app: app ? [app.scrollWidth, app.clientWidth, app.scrollHeight, app.clientHeight] : null,
      },
    };
  })()`);
}

async function exerciseTimelineDeskSurfaceAria(client) {
  return await evaluatePageFunction(client, async () => {
    const expected = ['show', 'automation', 'playback'];
    const read = () => {
      const buttons = [...document.querySelectorAll('.timelineDeskTabs button')];
      return buttons.map((button) => ({
        id: button.getAttribute('data-timeline-desk-surface') || '',
        pressed: button.getAttribute('aria-pressed') === 'true',
        active: button.classList.contains('active'),
      }));
    };
    const transitions = [];
    for (const id of expected) {
      const button = document.querySelector(`[data-timeline-desk-surface="${id}"]`);
      if (!(button instanceof HTMLButtonElement)) {
        transitions.push({ id, state: read(), missing: true });
        continue;
      }
      button.click();
      await new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame)));
      transitions.push({ id, state: read(), missing: false });
    }
    document.querySelector('[data-timeline-desk-surface="show"]')?.click();
    await new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame)));
    const restored = read();
    const transitionStatesValid = transitions.every((transition) =>
      !transition.missing &&
      transition.state.length === expected.length &&
      transition.state.filter((entry) => entry.pressed).length === 1 &&
      transition.state.filter((entry) => entry.active).length === 1 &&
      transition.state.some((entry) => entry.id === transition.id && entry.pressed && entry.active)
    );
    return {
      transitions,
      restored,
      passed: transitionStatesValid &&
        restored.filter((entry) => entry.pressed).length === 1 &&
        restored.some((entry) => entry.id === 'show' && entry.pressed && entry.active),
    };
  });
}

async function exerciseTimelineBlockPropertiesDrawerLayout(client) {
  const markerPoint = await evaluatePageFunction(client, () => {
    const bodies = [...document.querySelectorAll('.timelineMarker.sceneBlock .timelineSceneBlockBody')];
    const body = bodies.find((candidate) => {
      const rect = candidate.getBoundingClientRect();
      return rect.width >= 32 && rect.height >= 20 &&
        rect.right > 0 && rect.left < innerWidth && rect.bottom > 0 && rect.top < innerHeight;
    });
    const rect = body?.getBoundingClientRect();
    return rect ? { x: rect.left + rect.width / 2, y: rect.top + rect.height / 2 } : null;
  });
  if (!markerPoint) return { passed: false, reason: 'missing-rendered-scene-block' };

  await client.send('Input.dispatchMouseEvent', {
    type: 'mousePressed',
    x: markerPoint.x,
    y: markerPoint.y,
    button: 'left',
    buttons: 1,
    clickCount: 1,
  });
  await client.send('Input.dispatchMouseEvent', {
    type: 'mouseReleased',
    x: markerPoint.x,
    y: markerPoint.y,
    button: 'left',
    buttons: 0,
    clickCount: 1,
  });
  await sleep(100);

  const layout = await evaluatePageFunction(client, async () => {
    await new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame)));
    const surface = document.querySelector('.timelineShowSurface');
    const frame = surface?.querySelector(':scope > .timelineOverviewFrame');
    const drawer = surface?.querySelector(
      ':scope > [data-timeline-context-drawer-panel="block"]:not(.timelineBlockBrowserDrawer)',
    );
    const body = drawer?.querySelector('.timelineContextDrawerBody');
    const inspector = drawer?.querySelector('[data-scene-block-inspector-only="true"]');
    const surfaceRect = surface?.getBoundingClientRect();
    const frameRect = frame?.getBoundingClientRect();
    const drawerRect = drawer?.getBoundingClientRect();
    const bodyRect = body?.getBoundingClientRect();
    const contained = (inner, outer) => Boolean(
      inner && outer &&
      inner.left >= outer.left - 1 && inner.top >= outer.top - 1 &&
      inner.right <= outer.right + 1 && inner.bottom <= outer.bottom + 1
    );
    const horizontalIntersection = frameRect && drawerRect
      ? Math.max(0, Math.min(frameRect.right, drawerRect.right) - Math.max(frameRect.left, drawerRect.left))
      : Number.POSITIVE_INFINITY;
    const verticalIntersection = frameRect && drawerRect
      ? Math.max(0, Math.min(frameRect.bottom, drawerRect.bottom) - Math.max(frameRect.top, drawerRect.top))
      : Number.POSITIVE_INFINITY;
    const bodyStyle = body ? getComputedStyle(body) : null;
    const selectedBlockBody = document.querySelector('.timelineMarker.sceneBlock.selected .timelineSceneBlockBody');
    const selectedBlockStyle = selectedBlockBody ? getComputedStyle(selectedBlockBody) : null;
    const selectedStrokeChannels = selectedBlockStyle?.stroke.match(/[0-9.]+/g)?.slice(0, 3).map(Number) ?? [];
    const focusables = body
      ? [...body.querySelectorAll('button:not([disabled]), input:not([disabled]), select:not([disabled])')]
      : [];
    const lastControl = focusables.at(-1) ?? null;
    if (body && lastControl instanceof HTMLElement) {
      body.scrollTop = body.scrollHeight;
      lastControl.scrollIntoView({ block: 'nearest', inline: 'nearest' });
      await new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame)));
    }
    const finalBodyRect = body?.getBoundingClientRect();
    const lastControlRect = lastControl?.getBoundingClientRect();
    const lastControlReachable = Boolean(
      finalBodyRect && lastControlRect &&
      lastControlRect.top >= finalBodyRect.top - 1 &&
      lastControlRect.bottom <= finalBodyRect.bottom + 1 &&
      lastControlRect.left >= finalBodyRect.left - 1 &&
      lastControlRect.right <= finalBodyRect.right + 1
    );
    const documentElement = document.documentElement;
    const app = document.querySelector('.app');
    return {
      drawerOpen: Boolean(drawer),
      browserMode: drawer?.classList.contains('timelineBlockBrowserDrawer') ?? null,
      inspectorOnly: Boolean(inspector),
      surfaceRect: surfaceRect ? { left: surfaceRect.left, top: surfaceRect.top, right: surfaceRect.right, bottom: surfaceRect.bottom } : null,
      frameRect: frameRect ? { left: frameRect.left, top: frameRect.top, right: frameRect.right, bottom: frameRect.bottom, width: frameRect.width } : null,
      drawerRect: drawerRect ? { left: drawerRect.left, top: drawerRect.top, right: drawerRect.right, bottom: drawerRect.bottom, width: drawerRect.width } : null,
      frameContained: contained(frameRect, surfaceRect),
      drawerContained: contained(drawerRect, surfaceRect),
      adjacentWithoutOverlap: horizontalIntersection <= 1 || verticalIntersection <= 1,
      drawerRightOfFrame: Boolean(frameRect && drawerRect && drawerRect.left >= frameRect.right - 1),
      drawerBodyHorizontalOverflow: body ? body.scrollWidth - body.clientWidth : Number.POSITIVE_INFINITY,
      drawerBodyVerticalOverflowSafe: Boolean(
        body && (body.scrollHeight <= body.clientHeight + 1 || /(auto|scroll)/.test(bodyStyle?.overflowY || ''))
      ),
      focusableCount: focusables.length,
      lastControlReachable,
      selectedBlockClass: selectedBlockBody?.closest('.timelineMarker.sceneBlock')?.getAttribute('class') ?? '',
      selectedBlockStroke: selectedBlockStyle?.stroke ?? '',
      selectedBlockStrokeWidth: selectedBlockStyle?.strokeWidth ?? '',
      selectedBlockHasVisibleStroke:
        selectedStrokeChannels.length === 3 &&
        selectedStrokeChannels.every((channel) => channel >= 240) &&
        (Number.parseFloat(selectedBlockStyle?.strokeWidth || '0') || 0) >= 2.4,
      documentAndAppScrollZero:
        window.scrollX === 0 && window.scrollY === 0 &&
        documentElement.scrollWidth === documentElement.clientWidth &&
        documentElement.scrollHeight === documentElement.clientHeight &&
        (!app || (app.scrollWidth === app.clientWidth && app.scrollHeight === app.clientHeight)),
    };
  });

  const closeButton = await evaluatePageFunction(client, () => {
    const button = document.querySelector(
      '[data-timeline-context-drawer-panel="block"] .timelineContextDrawerClose',
    );
    if (!(button instanceof HTMLButtonElement)) return false;
    button.click();
    return true;
  });
  await sleep(60);
  const closed = await evaluatePageFunction(client, () =>
    !document.querySelector('[data-timeline-context-drawer-panel="block"]'),
  );
  return {
    markerPoint,
    layout,
    closeButton,
    closed,
    passed: Boolean(
      layout.drawerOpen &&
      layout.browserMode === false &&
      layout.inspectorOnly &&
      layout.frameContained &&
      layout.drawerContained &&
      layout.adjacentWithoutOverlap &&
      layout.drawerRightOfFrame &&
      layout.frameRect?.width >= 180 &&
      layout.drawerRect?.width >= 240 &&
      layout.drawerBodyHorizontalOverflow <= 1 &&
      layout.drawerBodyVerticalOverflowSafe &&
      layout.focusableCount > 0 &&
      layout.lastControlReachable &&
      layout.selectedBlockHasVisibleStroke &&
      layout.documentAndAppScrollZero &&
      closeButton && closed
    ),
  };
}

async function readTimelineLaneCountState(client) {
  return await client.evaluate(`(async () => {
    await new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame)));
    const overview = document.querySelector('.timelineOverview');
    return {
      laneNames: [...document.querySelectorAll('.timelineUserLaneGutter')]
        .map((gutter) => ({
          id: gutter.getAttribute('data-timeline-layer-id') || '',
          name: (gutter.getAttribute('aria-label') || '').trim(),
        }))
        .sort((left, right) => left.id.localeCompare(right.id)),
      markerIds: [...document.querySelectorAll('.timelineMarker.sceneBlock')]
        .filter((marker) => marker.getBoundingClientRect().width > 0)
        .map((marker) => marker.getAttribute('data-timeline-event-id') || '')
        .sort(),
      visibleStartMs: Number(overview?.getAttribute('data-visible-start-ms') || 0),
      visibleEndMs: Number(overview?.getAttribute('data-visible-end-ms') || 0),
    };
  })()`);
}

async function exerciseTimelineLaneCountStability(client) {
  const before = await readTimelineLaneCountState(client);
  let after = before;
  for (let attempt = 0; attempt < 8; attempt += 1) {
    const zoomed = await client.evaluate(`(() => {
      const button = document.querySelector('[data-timeline-tool="zoom-in"]');
      if (!(button instanceof HTMLButtonElement) || button.disabled) return false;
      button.click();
      return true;
    })()`);
    await sleep(50);
    const panned = await client.evaluate(`(() => {
      const button = document.querySelector('[data-timeline-tool="pan-next"]');
      if (!(button instanceof HTMLButtonElement) || button.disabled) return false;
      button.click();
      return true;
    })()`);
    await sleep(70);
    after = await readTimelineLaneCountState(client);
    if (JSON.stringify(after.markerIds) !== JSON.stringify(before.markerIds)) break;
    if (!zoomed && !panned) break;
  }
  const restored = await client.evaluate(`(() => {
    const button = document.querySelector('[data-timeline-tool="fit-all"]');
    if (!(button instanceof HTMLButtonElement) || button.disabled) return false;
    button.click();
    return true;
  })()`);
  await sleep(80);
  const finalState = await readTimelineLaneCountState(client);
  return {
    before,
    after,
    finalState,
    restored,
    passed:
      before.laneNames.length > 0 &&
      before.laneNames.every((entry) => /, \d+ items$/.test(entry.name)) &&
      (after.visibleStartMs !== before.visibleStartMs || after.visibleEndMs !== before.visibleEndMs) &&
      JSON.stringify(after.markerIds) !== JSON.stringify(before.markerIds) &&
      JSON.stringify(after.laneNames) === JSON.stringify(before.laneNames) &&
      restored &&
      JSON.stringify(finalState.laneNames) === JSON.stringify(before.laneNames),
  };
}

async function exerciseTimelineBlockKeyboardFocusVisual(client) {
  const prepared = await client.evaluate(`(() => {
    const markers = [...document.querySelectorAll('.timelineMarker')]
      .filter((marker) => {
        const rect = marker.getBoundingClientRect();
        return rect.width > 0 && rect.height > 0;
      });
    const marker = markers.find((candidate) => candidate.tabIndex === 0);
    const sceneBlockCount = markers.filter((candidate) => candidate.classList.contains('sceneBlock')).length;
    if (!(marker instanceof SVGGElement) || sceneBlockCount < 2) {
      return {
        startId: '',
        markerCount: markers.length,
        sceneBlockCount,
        selected: false,
        markerTabStops: markers.map((candidate) => ({
          id: candidate.getAttribute('data-timeline-event-id') || '',
          attribute: candidate.getAttribute('tabindex'),
          property: candidate.tabIndex,
          selected: candidate.classList.contains('selected'),
        })),
      };
    }
    marker.setAttribute('data-timeline-keyboard-focus-probe', 'true');
    return {
      startId: marker.getAttribute('data-timeline-event-id') || '',
      markerCount: markers.length,
      sceneBlockCount,
      startIsSceneBlock: marker.classList.contains('sceneBlock'),
      selected: marker.classList.contains('selected'),
    };
  })()`);
  if (!prepared?.startId) return { passed: false, prepared, result: null };
  const anchorFocused = await client.evaluate(`(() => {
    const anchor = document.querySelector('[data-timeline-tool="reveal-playhead"]')
      ?? document.querySelector('[data-timeline-desk-surface="show"]');
    if (!(anchor instanceof HTMLButtonElement) || anchor.disabled) return false;
    anchor.focus();
    return document.activeElement === anchor;
  })()`);
  let focused = null;
  let tabsToMarker = 0;
  for (let tab = 1; tab <= 96; tab += 1) {
    await pressKey(client, 'Tab');
    focused = await client.evaluate(`(() => {
      const marker = document.activeElement?.closest?.('.timelineMarker');
      const body = marker?.querySelector('.timelineSceneBlockBody');
      const style = body ? getComputedStyle(body) : null;
      const channels = style?.stroke.match(/[0-9.]+/g)?.slice(0, 3).map(Number) ?? [];
      return {
        activeTag: document.activeElement?.tagName ?? '',
        activeId: marker?.getAttribute('data-timeline-event-id') ?? '',
        sceneBlock: Boolean(marker?.classList.contains('sceneBlock')),
        focusVisible: Boolean(marker?.matches(':focus-visible')),
        whiteStroke: channels.length === 3 && channels.every((channel) => channel >= 240),
        strokeWidth: Number.parseFloat(style?.strokeWidth || '0') || 0,
        stroke: style?.stroke ?? '',
      };
    })()`);
    if (focused.activeId) {
      tabsToMarker = tab;
      break;
    }
  }
  if (!focused?.activeId) {
    await client.evaluate(`document.querySelector('[data-timeline-keyboard-focus-probe="true"]')
      ?.removeAttribute('data-timeline-keyboard-focus-probe')`);
    return { passed: false, prepared, anchorFocused, tabsToMarker, focused, result: null };
  }
  const focusedProbe = await client.evaluate(`(() => {
    const probe = document.querySelector('[data-timeline-keyboard-focus-probe="true"]');
    return {
      activeTag: document.activeElement?.tagName ?? '',
      activeId: document.activeElement?.getAttribute?.('data-timeline-event-id') ?? '',
      focusVisible: Boolean(probe?.matches(':focus-visible')),
      activeIsProbe: document.activeElement === probe,
    };
  })()`);
  let result = null;
  let arrowMoves = 0;
  for (let move = 1; move <= prepared.markerCount; move += 1) {
    await pressKey(client, 'ArrowRight');
    await sleep(80);
    result = await client.evaluate(`(() => {
      const marker = document.activeElement?.closest?.('.timelineMarker');
      const body = marker?.querySelector('.timelineSceneBlockBody');
      const style = body ? getComputedStyle(body) : null;
      const channels = style?.stroke.match(/[0-9.]+/g)?.slice(0, 3).map(Number) ?? [];
      return {
        activeId: marker?.getAttribute('data-timeline-event-id') || '',
        activeTag: document.activeElement?.tagName ?? '',
        activeClass: document.activeElement?.getAttribute?.('class') ?? '',
        activeTabIndex: document.activeElement?.getAttribute?.('tabindex') ?? '',
        sceneBlock: Boolean(marker?.classList.contains('sceneBlock')),
        focusVisible: Boolean(marker?.matches(':focus-visible')),
        selected: Boolean(marker?.classList.contains('selected')),
        whiteStroke: channels.length === 3 && channels.every((channel) => channel >= 240),
        strokeWidth: Number.parseFloat(style?.strokeWidth || '0') || 0,
        stroke: style?.stroke ?? '',
      };
    })()`);
    arrowMoves = move;
    if (!result.activeId || result.sceneBlock) break;
  }
  await client.evaluate(`document.querySelector('[data-timeline-keyboard-focus-probe="true"]')
    ?.removeAttribute('data-timeline-keyboard-focus-probe')`);
  return {
    prepared,
    anchorFocused,
    tabsToMarker,
    focused,
    focusedProbe,
    arrowMoves,
    result,
    passed:
      anchorFocused &&
      focusedProbe.activeIsProbe &&
      focusedProbe.activeId === prepared.startId &&
      focused.focusVisible &&
      result?.activeId.length > 0 &&
      result.activeId !== prepared.startId &&
      result.sceneBlock &&
      result.focusVisible &&
      result.selected &&
      result.whiteStroke &&
      result.strokeWidth >= 2.8,
  };
}

async function exerciseTimelineDialogEscapePriority(client) {
  const prepared = await client.evaluate(`(() => {
    const drawerToggle = document.querySelector('[data-timeline-block-properties-toggle]');
    if (!(drawerToggle instanceof HTMLButtonElement)) return false;
    if (drawerToggle.getAttribute('aria-pressed') !== 'true') drawerToggle.click();
    const trigger = document.querySelector('[data-timeline-layer-menu-trigger]');
    if (!(trigger instanceof HTMLButtonElement)) return false;
    trigger.setAttribute('data-timeline-dialog-return-focus-probe', 'true');
    trigger.focus();
    return true;
  })()`);
  if (!prepared) return { passed: false, prepared: false };
  await pressKey(client, 'Enter');
  const opened = await client.evaluate(`(() => {
    const menu = document.querySelector('.timelineLayerContextMenu');
    const addButton = [...(menu?.querySelectorAll('button') ?? [])]
      .find((button) => (button.textContent || '').trim() === 'Add Layer');
    if (!(addButton instanceof HTMLButtonElement)) return false;
    addButton.click();
    return true;
  })()`);
  await sleep(80);
  const beforeEscape = await client.evaluate(`(() => {
    const dialog = document.querySelector('[data-timeline-layer-add-dialog]');
    return {
      dialogOpen: dialog instanceof HTMLDialogElement && dialog.open,
      focusInsideDialog: Boolean(dialog?.contains(document.activeElement)),
      drawerOpen: Boolean(document.querySelector('[data-timeline-context-drawer-panel="block"]')),
    };
  })()`);
  await client.send('Input.dispatchKeyEvent', {
    type: 'keyDown',
    code: 'Escape',
    key: 'Escape',
    windowsVirtualKeyCode: 27,
    nativeVirtualKeyCode: 27,
  });
  await client.send('Input.dispatchKeyEvent', {
    type: 'keyUp',
    code: 'Escape',
    key: 'Escape',
    windowsVirtualKeyCode: 27,
    nativeVirtualKeyCode: 27,
  });
  await sleep(80);
  const afterFirstEscape = await client.evaluate(`(() => {
    const dialog = document.querySelector('[data-timeline-layer-add-dialog]');
    const trigger = document.querySelector('[data-timeline-dialog-return-focus-probe="true"]');
    return {
      dialogClosed: !(dialog instanceof HTMLDialogElement) || !dialog.open,
      drawerOpen: Boolean(document.querySelector('[data-timeline-context-drawer-panel="block"]')),
      focusReturned: document.activeElement === trigger,
    };
  })()`);
  await pressKey(client, 'Escape');
  await sleep(80);
  const afterSecondEscape = await client.evaluate(`(() => {
    const trigger = document.querySelector('[data-timeline-dialog-return-focus-probe="true"]');
    const result = {
      drawerClosed: !document.querySelector('[data-timeline-context-drawer-panel="block"]'),
      dialogClosed: !document.querySelector('[data-timeline-layer-add-dialog][open]'),
    };
    trigger?.removeAttribute('data-timeline-dialog-return-focus-probe');
    return result;
  })()`);
  return {
    prepared,
    opened,
    beforeEscape,
    afterFirstEscape,
    afterSecondEscape,
    passed:
      opened &&
      beforeEscape.dialogOpen && beforeEscape.focusInsideDialog && beforeEscape.drawerOpen &&
      afterFirstEscape.dialogClosed && afterFirstEscape.drawerOpen && afterFirstEscape.focusReturned &&
      afterSecondEscape.dialogClosed && afterSecondEscape.drawerClosed,
  };
}

async function exerciseTimelineSlimEscapePriority(client) {
  const opened = await client.evaluate(`(() => {
    const isRendered = (element) => {
      if (!(element instanceof Element)) return false;
      const rect = element.getBoundingClientRect();
      const style = getComputedStyle(element);
      return rect.width > 0 && rect.height > 0 && style.display !== 'none' && style.visibility !== 'hidden';
    };
    const toggle = [...document.querySelectorAll(
      '.timelineViewportToolbar button[aria-haspopup], .timelineDirectToolbar button[aria-haspopup], [data-timeline-slim-toolbar] button[aria-haspopup]'
    )].find((button) => isRendered(button) && !button.disabled);
    if (!(toggle instanceof HTMLButtonElement)) return { required: false, opened: false };
    toggle.setAttribute('data-timeline-slim-escape-probe', 'true');
    toggle.click();
    const controls = toggle.getAttribute('aria-controls');
    const controlled = controls ? document.getElementById(controls) : null;
    const popover = (() => {
      try { return document.querySelector(':popover-open'); } catch { return null; }
    })();
    return {
      required: true,
      opened:
        toggle.getAttribute('aria-expanded') === 'true' ||
        isRendered(controlled) ||
        isRendered(popover) ||
        [...document.querySelectorAll('.timelinePanel [role="menu"]')].some(isRendered),
    };
  })()`);
  if (!opened.required || !opened.opened) return { ...opened, passed: true, closed: true };
  await sleep(60);
  await client.send('Input.dispatchKeyEvent', { type: 'keyDown', key: 'Escape', code: 'Escape' });
  await client.send('Input.dispatchKeyEvent', { type: 'keyUp', key: 'Escape', code: 'Escape' });
  await sleep(80);
  const closed = await client.evaluate(`(() => {
    const isRendered = (element) => {
      if (!(element instanceof Element)) return false;
      const rect = element.getBoundingClientRect();
      const style = getComputedStyle(element);
      return rect.width > 0 && rect.height > 0 && style.display !== 'none' && style.visibility !== 'hidden';
    };
    const toggle = document.querySelector('[data-timeline-slim-escape-probe="true"]');
    const controls = toggle?.getAttribute('aria-controls');
    const controlled = controls ? document.getElementById(controls) : null;
    const popover = (() => {
      try { return document.querySelector(':popover-open'); } catch { return null; }
    })();
    const menuVisible = [...document.querySelectorAll('.timelinePanel [role="menu"]')].some(isRendered);
    const result = {
      closed:
        toggle?.getAttribute('aria-expanded') !== 'true' &&
        !isRendered(controlled) &&
        !isRendered(popover) &&
        !menuVisible,
      timelineStillVisible: isRendered(document.querySelector('.timelineShowSurface')),
      outerScrollZero: window.scrollX === 0 && window.scrollY === 0,
    };
    toggle?.removeAttribute('data-timeline-slim-escape-probe');
    return result;
  })()`);
  return {
    ...opened,
    ...closed,
    passed: closed.closed && closed.timelineStillVisible && closed.outerScrollZero,
  };
}

async function exerciseTimelineLaneMenuKeyboard(client) {
  const prepared = await client.evaluate(`(() => {
    const trigger = document.querySelector('[data-timeline-layer-menu-trigger]');
    if (!(trigger instanceof HTMLButtonElement)) return null;
    trigger.setAttribute('data-timeline-lane-menu-keyboard-probe', 'true');
    trigger.focus();
    return {
      tagName: trigger.tagName,
      hasPopup: trigger.getAttribute('aria-haspopup'),
      focused: document.activeElement === trigger,
    };
  })()`);
  if (!prepared) return { passed: false, prepared: null, opened: null, closed: null };
  await pressKey(client, 'Enter');
  const opened = await client.evaluate(`(async () => {
    await new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame)));
    const menu = document.querySelector('.timelineLayerContextMenu[role="dialog"][aria-modal="false"]');
    return {
      visible: Boolean(menu && menu.getBoundingClientRect().width > 0),
      focusInside: Boolean(menu && menu.contains(document.activeElement)),
      role: menu?.getAttribute('role') ?? '',
    };
  })()`);
  await pressKey(client, 'Escape');
  const closed = await client.evaluate(`(async () => {
    await new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame)));
    const trigger = document.querySelector('[data-timeline-lane-menu-keyboard-probe="true"]');
    const result = {
      closed: !document.querySelector('.timelineLayerContextMenu'),
      focusReturned: document.activeElement === trigger,
    };
    trigger?.removeAttribute('data-timeline-lane-menu-keyboard-probe');
    return result;
  })()`);
  return {
    prepared,
    opened,
    closed,
    passed: prepared.tagName === 'BUTTON' &&
      prepared.hasPopup === 'dialog' &&
      prepared.focused &&
      opened.visible &&
      opened.focusInside &&
      opened.role === 'dialog' &&
      closed.closed &&
      closed.focusReturned,
  };
}

async function runTimelineSlimViewport(client, viewport) {
  const sceneMatrix = await runSceneMatrixPaneCheck(client, viewport);
  // Cover both persisted authored layers and the implicit two-layer projection
  // used by new / legacy .sdc projects. T15 must present the same operator
  // contract on both data shapes.
  await client.send('Page.navigate', { url: fixtureUrl('timeline-layered') });
  await waitForApp(client);
  await clickVisibleByText(client, '.workspaceTabs button', 'Control');
  await clickVisibleByText(client, '.controlModeTabs button', 'Timeline');
  await clickVisibleByText(client, '.timelineDeskTabs button', 'Show');
  await sleep(120);
  const visual = await measureTimelineSlimVisual(client);
  const deskSurfaceAria = await exerciseTimelineDeskSurfaceAria(client);
  const laneCountStability = await exerciseTimelineLaneCountStability(client);
  const blockKeyboardFocus = await exerciseTimelineBlockKeyboardFocusVisual(client);
  const blockPropertiesDrawer = await exerciseTimelineBlockPropertiesDrawerLayout(client);
  const laneMenuKeyboard = await exerciseTimelineLaneMenuKeyboard(client);
  const dialogEscapePriority = await exerciseTimelineDialogEscapePriority(client);
  const escapePriority = await exerciseTimelineSlimEscapePriority(client);
  await client.send('Page.navigate', { url: fixtureUrl('timeline') });
  await waitForApp(client);
  await clickVisibleByText(client, '.workspaceTabs button', 'Control');
  await clickVisibleByText(client, '.controlModeTabs button', 'Timeline');
  await clickVisibleByText(client, '.timelineDeskTabs button', 'Show');
  await sleep(120);
  const implicitVisual = await measureTimelineSlimVisual(client);
  const expectedDeskSurfaceIds = ['automation', 'playback', 'show'];
  const expectedTimelineToolIds = [
    'arm-cue',
    'block-properties',
    'fit-all',
    'magnet',
    'pan-next',
    'pan-prev',
    'reveal-playhead',
    'reveal-selected',
    'stretch-rate',
    'stretch-window',
    'super-scene',
    'zoom-in',
    'zoom-out',
  ];
  const conditions = [
    ['timelineSlimRemovesDuplicateCuesDeskTab', () => visual.cuesDeskTabCount === 0],
    ['timelineSlimUsesExactDeskSurfaceSetAndAriaState', () =>
      JSON.stringify([...visual.deskTabSurfaceIds].sort()) === JSON.stringify(expectedDeskSurfaceIds) &&
      visual.deskTabPressedStates.filter((value) => value === 'true').length === 1 &&
      visual.deskTabActiveStates.filter(Boolean).length === 1 &&
      deskSurfaceAria.passed],
    ['timelineSlimToolbarsUseNamedIconToggles', () =>
      visual.toolbarRootCount >= 1 &&
      visual.toolbarButtonCount === expectedTimelineToolIds.length &&
      visual.toolbarAriaButtonCount === visual.toolbarButtonCount &&
      visual.toolbarIconOnlyButtonCount === visual.toolbarButtonCount &&
      visual.toolbarOverflowCount === 0],
    ['timelineSlimPreservesExactToolInventory', () =>
      visual.toolbarToolIds.length === expectedTimelineToolIds.length &&
      new Set(visual.toolbarToolIds).size === expectedTimelineToolIds.length &&
      JSON.stringify([...visual.toolbarToolIds].sort()) === JSON.stringify(expectedTimelineToolIds)],
    ['timelineSlimBlockPropertiesDrawerIsDockedReachableAndContained', () => blockPropertiesDrawer.passed],
    ['timelineSlimLaneCountsStayStableAcrossZoomAndPan', () => laneCountStability.passed],
    ['timelineSlimKeyboardFocusKeepsVisibleWhiteStroke', () => blockKeyboardFocus.passed],
    ['timelineSlimGuttersUseExactlyNumberExpandEyeLock', () =>
      visual.gutterCount > 0 && visual.gutterRows.every((row) => row.exactFourElements)],
    ['timelineSlimLaneMenuSupportsKeyboardAndReturnsFocus', () => laneMenuKeyboard.passed],
    ['timelineSlimDialogEscapePrecedesDrawerAndReturnsFocus', () => dialogEscapePriority.passed],
    ['timelineSlimGutterTypeUsesColorOrIconNotText', () =>
      visual.visibleGutterNameCount === 0 && visual.gutterKindCount >= 2 &&
      visual.sectionKindSummaries.length >= 2 &&
      new Set(visual.sectionKindSummaries.map((entry) => entry.name)).size === visual.sectionKindSummaries.length &&
      visual.sectionKindSummaries.every((entry) =>
        entry.kind.length > 0 && entry.name.toLowerCase().includes(entry.kind.toLowerCase()))],
    ['timelineSlimImplicitLayersUseSameGutterContract', () =>
      implicitVisual.gutterCount === 2 &&
      implicitVisual.gutterRows.every((row) => row.exactFourElements) &&
      implicitVisual.visibleGutterNameCount === 0 &&
      implicitVisual.gutterKindCount === 2],
    ['timelineSlimBlocksAreThick', () =>
      visual.blockCount > 0 && visual.thickBlockCount === visual.blockCount && visual.minBlockHeight >= 26],
    ['timelineSlimBlocksUseSolidIdentityFill', () =>
      visual.blockCount > 0 &&
      visual.identityFillCount === visual.blockCount &&
      visual.solidIdentityBandCount === visual.blockCount],
    ['timelineSlimBlocksUseOpaqueBlackBorder', () =>
      visual.blockCount > 0 && visual.opaqueBlackBorderCount === visual.blockCount],
    ['timelineSlimBlocksShowNameAndDurationOnTwoLines', () =>
      visual.twoLineEligibleCount > 0 &&
      visual.twoLineBlockCount === visual.twoLineEligibleCount &&
      visual.minTwoLineFontPx >= 10],
    ['timelineSlimImplicitLayersUseSameBlockContract', () =>
      implicitVisual.blockCount > 0 &&
      implicitVisual.thickBlockCount === implicitVisual.blockCount &&
      implicitVisual.identityFillCount === implicitVisual.blockCount &&
      implicitVisual.opaqueBlackBorderCount === implicitVisual.blockCount &&
      implicitVisual.twoLineBlockCount === implicitVisual.twoLineEligibleCount],
    ['timelineSlimGridLinesStayDimmed', () =>
      visual.minorGridLineCount > 0 &&
      visual.majorGridLineCount > 0 &&
      visual.dividerLineCount > 0 &&
      visual.maxMinorGridInk <= 0.085 &&
      visual.maxMajorGridInk <= 0.14 &&
      visual.maxDividerInk <= 0.09],
    ['timelineSlimPreservesT14SubFourPixelClick', () =>
      sceneMatrix.subThresholdClick?.movedPx === 3 &&
      sceneMatrix.checks.matrixSubThresholdMoveRecallsWithoutTimelineMutation === true],
    ['timelineSlimPreservesT14OneGestureDrag', () =>
      sceneMatrix.checks.matrixAndTimelineLaneCoexistForOneGestureDrag === true &&
      sceneMatrix.checks.matrixOneGestureDragUsesTimelineDropPath === true &&
      sceneMatrix.checks.matrixDragDoesNotRecallCue === true],
    ['timelineSlimKeepsDocumentAndAppScrollZero', () =>
      visual.documentAndAppScrollZero &&
      implicitVisual.documentAndAppScrollZero &&
      sceneMatrix.before.documentAndAppScrollZero &&
      sceneMatrix.after.documentAndAppScrollZero],
    ['timelineSlimEscapeClosesToolMenuBeforeLeavingTimeline', () => escapePriority.passed],
  ];
  const checks = Object.fromEntries(conditions.map(([name, check]) => {
    try { return [name, Boolean(check())]; } catch { return [name, false]; }
  }));
  const failedChecks = Object.entries(checks)
    .filter(([, passed]) => !passed)
    .map(([name]) => name);
  return {
    viewport,
    label: `timeline-slim-${viewport.width}x${viewport.height}`,
    passed: failedChecks.length === 0,
    checks,
    failedChecks,
    visual,
    implicitVisual,
    deskSurfaceAria,
    blockPropertiesDrawer,
    laneCountStability,
    blockKeyboardFocus,
    laneMenuKeyboard,
    dialogEscapePriority,
    t14: {
      subThresholdClick: sceneMatrix.subThresholdClick,
      oneGestureDrag: sceneMatrix.oneGestureDrag,
      failedChecks: sceneMatrix.failedChecks.filter((name) =>
        name.includes('SubThreshold') || name.includes('OneGestureDrag') || name.includes('DragDoesNotRecall')),
    },
    escapePriority,
  };
}

async function runCueRecallViewport(client, viewport) {
  await openCueFixture(client, viewport, "cue-recall");
  await selectVisibleOption(client, "#cue-store-form select", "effects");
  await sleep(120);
  await clickVisibleByText(client, ".cueItem .cueEffectRecallEditor > summary", "Effect Recall");
  await sleep(80);
  await clickVisibleByText(client, ".cueItem .cueEffectRecallToolbar .buttonRow button", "Clear");
  await sleep(120);
  const stepExercise = await client.evaluate(`(async () => {
    const raf2 = () => new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame)));
    const editor = document.querySelector('.cueItem [data-cue-step-editor]');
    if (!(editor instanceof HTMLDetailsElement)) return null;
    editor.open = true;
    editor.scrollIntoView({ block: 'center', inline: 'nearest' });
    await raf2();
    const initialRows = [...editor.querySelectorAll('[data-cue-step-index]')];
    const initialFadeValues = initialRows.map((row) => Number(row.querySelector('[data-cue-step-fade]')?.value));
    const initialHoldValues = initialRows.map((row) => Number(row.querySelector('[data-cue-step-hold]')?.value));
    const initialTotalLabel = (editor.querySelector('summary small')?.textContent || '').trim();
    editor.querySelectorAll('[data-cue-step-duplicate]')[0]?.click();
    await raf2();
    const firstFade = editor.querySelectorAll('[data-cue-step-fade]')[0];
    if (firstFade instanceof HTMLInputElement) {
      firstFade.value = '375';
      firstFade.dispatchEvent(new Event('input', { bubbles: true }));
    }
    await raf2();
    editor.querySelectorAll('[data-cue-step-down]')[0]?.click();
    await raf2();
    editor.querySelector('[data-cue-step-add]')?.click();
    await raf2();
    editor.querySelector('[data-cue-step-use-beats]')?.click();
    await raf2();
    return {
      initialCount: initialRows.length,
      initialFadeValues,
      initialHoldValues,
      initialTotalLabel,
    };
  })()`);
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
    const stepEditor = document.querySelector('.cueItem [data-cue-step-editor]');
    const stepList = stepEditor?.querySelector('.cueStepList');
    const stepRows = [...(stepEditor?.querySelectorAll('[data-cue-step-index]') ?? [])];
    const authoredBeats = document.querySelector('.cueItem [data-cue-authored-beats]');
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
      stepEditorOpen: Boolean(stepEditor?.open),
      stepCount: stepRows.length,
      stepFadeValues: stepRows.map((row) => Number(row.querySelector('[data-cue-step-fade]')?.value)),
      stepHoldValues: stepRows.map((row) => Number(row.querySelector('[data-cue-step-hold]')?.value)),
      stepTotalLabel: (stepEditor?.querySelector('summary small')?.textContent || '').trim(),
      stepDerivedLabel: (stepEditor?.querySelector('.cueStepToolbar span')?.textContent || '').trim(),
      stepListOverflowY: stepList ? getComputedStyle(stepList).overflowY : '',
      addButtonCount: stepEditor?.querySelectorAll('[data-cue-step-add]').length ?? 0,
      duplicateButtonCount: stepEditor?.querySelectorAll('[data-cue-step-duplicate]').length ?? 0,
      upButtonCount: stepEditor?.querySelectorAll('[data-cue-step-up]').length ?? 0,
      downButtonCount: stepEditor?.querySelectorAll('[data-cue-step-down]').length ?? 0,
      fadeInputCount: stepEditor?.querySelectorAll('[data-cue-step-fade]').length ?? 0,
      holdInputCount: stepEditor?.querySelectorAll('[data-cue-step-hold]').length ?? 0,
      saveStepsEnabled: !Boolean(stepEditor?.querySelector('[data-cue-step-save]')?.disabled),
      authoredBeatsValue: authoredBeats instanceof HTMLInputElement ? Number(authoredBeats.value) : null,
      // T7: the cue edit row carries the identity color picker.
      cueColorInputCount: document.querySelectorAll('.cueItem [data-cue-color-input]').length,
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
  const conditions = [
    ["cueActionReached", () => actionReached],
    ["cueNoOuterOverflow", () => hasNoOuterOverflow(containment)],
    ["cueEffectsScopeSelected", () => stats.scope === "effects"],
    ["cueEffectRecallOpen", () => stats.recallDetailsOpen],
    ["cueEffectRecallCleared", () => stats.recallCount === "0 / 1"],
    ["cueSaveDetailsLabel", () => stats.saveDetailsLabel === "Save Details"],
    ["cueSaveRecallLabel", () => stats.saveRecallLabel === "Save Recall"],
    ["cueUpdateLookLabel", () => stats.updateLookLabel === "Update Look"],
    ["cueSaveDetailsEnabled", () => !stats.saveDetailsDisabled],
    ["cueSaveRecallEnabled", () => !stats.saveRecallDisabled],
    ["cueUpdateLookEnabled", () => !stats.updateLookDisabled],
    ["cueThreeActionsRendered", () => stats.actionCount === 3],
    ["cueThreeActionsVisible", () => stats.fullyVisibleActionCount === 3],
    ["cueThreeStepFixtureLoaded", () => stepExercise?.initialCount === 3],
    ["cueStepFixtureFadeValues", () => JSON.stringify(stepExercise?.initialFadeValues) === JSON.stringify([250, 500, 1000])],
    ["cueStepFixtureHoldValues", () => JSON.stringify(stepExercise?.initialHoldValues) === JSON.stringify([750, 500, 250])],
    ["cueStepFixtureTotal", () => stepExercise?.initialTotalLabel === "3250 ms total"],
    ["cueStepEditorOpen", () => stats.stepEditorOpen],
    ["cueStepDuplicateAndAddApplied", () => stats.stepCount === 5],
    ["cueStepFadeEditAndReorderApplied", () => JSON.stringify(stats.stepFadeValues) === JSON.stringify([250, 375, 500, 1000, 1000])],
    ["cueStepHoldsPreserved", () => JSON.stringify(stats.stepHoldValues) === JSON.stringify([750, 750, 500, 250, 0])],
    ["cueStepEditedTotal", () => stats.stepTotalLabel === "5375 ms total"],
    ["cueStepDerivedBeatsVisible", () => stats.stepDerivedLabel === "10.750 beats at 120.0 BPM"],
    ["cueStepAuthoredBeatsApplied", () => stats.authoredBeatsValue === 10.75],
    ["cueStepUsesInternalScrollport", () => stats.stepListOverflowY === "auto" || stats.stepListOverflowY === "scroll"],
    ["cueStepAddControlRendered", () => stats.addButtonCount === 1],
    ["cueStepDuplicateControlsRendered", () => stats.duplicateButtonCount === 5],
    ["cueStepUpControlsRendered", () => stats.upButtonCount === 5],
    ["cueStepDownControlsRendered", () => stats.downButtonCount === 5],
    ["cueStepFadeInputsRendered", () => stats.fadeInputCount === 5],
    ["cueStepHoldInputsRendered", () => stats.holdInputCount === 5],
    ["cueStepDirtySaveEnabled", () => stats.saveStepsEnabled],
    // T7: the identity color picker lives in the cue edit row.
    ["cueColorPickerPresentInCueEditRow", () => stats.cueColorInputCount >= 1],
  ];
  const checks = Object.fromEntries(conditions.map(([name, check]) => {
    try {
      return [name, Boolean(check())];
    } catch {
      return [name, false];
    }
  }));
  const failedChecks = Object.entries(checks).filter(([, passed]) => !passed).map(([name]) => name);
  return {
    label: `cue-recall-${viewport.width}x${viewport.height}`,
    passed: failedChecks.length === 0,
    checks,
    failedChecks,
    containment,
    stepExercise,
    stats,
  };
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

async function openSceneBlockBrowser(client) {
  const drawerOpen = await client.evaluate(`Boolean(document.querySelector(
    '[data-timeline-context-drawer-panel="block"]'
  ))`);
  if (!drawerOpen) {
    await clickVisibleByText(client, "[data-timeline-block-properties-toggle]", "Block Properties");
    await sleep(100);
  }
  const browserMode = await client.evaluate(`document.querySelector(
    '[data-timeline-block-browser-toggle]'
  )?.getAttribute('aria-pressed') === 'true'`);
  if (!browserMode) {
    await clickVisibleSelector(client, "[data-timeline-block-browser-toggle]");
    await sleep(140);
  }
}

async function closeSceneBlockDrawer(client) {
  const closeButtonVisible = await client.evaluate(`(() => {
    const button = document.querySelector(
      '[data-timeline-context-drawer-panel="block"] .timelineContextDrawerClose'
    );
    if (!(button instanceof HTMLElement)) return false;
    const rect = button.getBoundingClientRect();
    return rect.width > 0 && rect.height > 0;
  })()`);
  if (closeButtonVisible) {
    await clickVisibleSelector(
      client,
      '[data-timeline-context-drawer-panel="block"] .timelineContextDrawerClose',
    );
    await sleep(100);
  }
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

  await openSceneBlockBrowser(client);
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
  await closeSceneBlockDrawer(client);

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
      // T15 keeps T4's upper move band inside the thicker two-line block.
      y: bodyRect.top + 5,
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
      .find((button) => [
        (button.textContent || "").trim(),
        button.getAttribute("aria-label") || "",
        button.getAttribute("title") || "",
      ].includes("Zoom Out"));
    const zoomOutClicked = zoomOut instanceof HTMLButtonElement && !zoomOut.disabled;
    if (zoomOutClicked) zoomOut.click();
    await new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame)));
    return { zoomOutClicked, before, after: read() };
  });

  const navigationStats = await evaluatePageFunction(client, async () => {
    const button = (label) => [...document.querySelectorAll(".timelineViewportToolbar button")]
      .find((candidate) => [
        (candidate.textContent || "").trim(),
        candidate.getAttribute("aria-label") || "",
        candidate.getAttribute("title") || "",
      ].includes(label));
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
  await openSceneBlockBrowser(client);
  await evaluatePageFunction(client, async () => {
    const search = document.querySelector(".sceneBlockRowSearch");
    if (!(search instanceof HTMLInputElement)) return;
    search.value = "500";
    search.dispatchEvent(new InputEvent("input", { bubbles: true, inputType: "insertText", data: "500" }));
    await new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame)));
    document.querySelector('.sceneBlockRow[data-scene-block-id="500"]')?.click();
    await new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame)));
  });
  await closeSceneBlockDrawer(client);
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
      const blockDrawerOpen = Boolean(document.querySelector(
        '[data-timeline-context-drawer-panel="block"]',
      ));
      const blockBrowserModeActive = document.querySelector(
        '[data-timeline-block-browser-toggle]',
      )?.getAttribute('aria-pressed') === 'true';
      return {
        stateTrack,
        stateHandlerElapsedMs: Number.isFinite(stateHandlerElapsedMs) ? stateHandlerElapsedMs : null,
        stateMicrotaskElapsedMs: Number.isFinite(stateMicrotaskElapsedMs) ? stateMicrotaskElapsedMs : null,
        blockDrawerOpen,
        blockBrowserModeActive,
        committedFilterVisible: stateTrack === wantedTrack && blockDrawerOpen && blockBrowserModeActive &&
          Boolean(document.querySelector(".sceneBlockOverlapFilter")),
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
    const readLaneGeometry = () => [...document.querySelectorAll(".timelineLayerRowBackground")]
      .map((row) => ({
        layerId: row.getAttribute("data-timeline-layer-id") ?? "",
        top: Number(row.getAttribute("y")),
        height: Number(row.getAttribute("height")),
      }));
    const laneGeometryBeforePan = readLaneGeometry();
    const namedToolbarButton = (label) => [...document.querySelectorAll(".timelineViewportToolbar button")]
      .find((button) => [
        (button.textContent || "").trim(),
        button.getAttribute("aria-label") || "",
        button.getAttribute("title") || "",
      ].includes(label));
    const panNext = namedToolbarButton("Pan Next");
    panNext?.click();
    await new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame)));
    const afterPanStartMs = Number(range?.getAttribute("data-visible-start-ms"));
    const laneGeometryAfterPan = readLaneGeometry();
    const badgeAfterPan = [...document.querySelectorAll(".timelineOverlapCluster")]
      .find((candidate) => candidate.getAttribute("data-overlap-track") === wantedTrack);
    const membersAfterPan = (badgeAfterPan?.getAttribute("data-overlap-members") || "")
      .split(",")
      .filter(Boolean)
      .map(Number);
    const filterPersistedAfterPan = Boolean(document.querySelector(".sceneBlockOverlapFilter"));
    const fitAll = namedToolbarButton("Fit All");
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
      laneGeometryBeforePan,
      laneGeometryAfterPan,
      laneGeometryStableAfterPan:
        JSON.stringify(laneGeometryAfterPan) === JSON.stringify(laneGeometryBeforePan),
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
    const listRect = document.querySelector(".sceneBlockFinder, .sceneBlockList")?.getBoundingClientRect();
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
    const listView = document.querySelector('[data-scene-block-view-toggle="list"]');
    if (listView instanceof HTMLButtonElement) listView.click();
    await new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame)));
    result.listViewReached = Boolean(document.querySelector(".sceneBlockList"));
    const dirtyRow = document.querySelector(".sceneBlockList .sceneBlockRow");
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
  await closeSceneBlockDrawer(client);
  const containment = await measure(client, "scene-block-overlap-" + track.toLowerCase() + "-" + viewport.width + "x" + viewport.height);
  const checks = [
    ["initial-overlap-contract", Boolean(
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
    initialStats.rulerTickCount <= 64
    )],
    ["activation-timing", Boolean(
    activationStats !== null &&
    activationStats.stateHandlerElapsedMs < 100 &&
    activationStats.stateMicrotaskElapsedMs < 100 &&
    activationStats.stateTrack === track &&
    activationStats.blockDrawerOpen &&
    activationStats.blockBrowserModeActive &&
    activationStats.committedFilterVisible
    )],
    ["keyboard-access", Boolean(
    activationStats !== null &&
    activationStats.keyboardAccessStats.active?.track === track &&
    activationStats.keyboardAccessStats.tabsToCluster > 0 &&
    // T15: six four-control lane gutters and the icon tool strip precede the
    // canvas. Keep the bound finite without tabbing through 500 block markers.
    activationStats.keyboardAccessStats.tabsToCluster <= 40 &&
    activationStats.keyboardAccessStats.axButtonFound &&
    activationStats.keyboardAccessStats.markerTabStopCount === 1 &&
    activationStats.keyboardAccessStats.clusterTabStopCount === 2 &&
    activationStats.keyboardAccessStats.overviewRole === "group"
    )],
    ["filtered-navigation", Boolean(
    activationStats !== null &&
    activationStats.filterText.includes("250") &&
    activationStats.pagerLabel.includes("/ 250") &&
    activationStats.rowCount > 0 &&
    activationStats.rowCount <= 12 &&
    activationStats.rowsBelongToCluster &&
    activationStats.afterPanStartMs > activationStats.beforePanStartMs &&
    activationStats.laneGeometryBeforePan.length > 0 &&
    activationStats.laneGeometryStableAfterPan &&
    activationStats.filterPersistedAfterPan &&
    activationStats.badgePersistedAfterPan &&
    activationStats.membershipPersistedAfterPan &&
    activationStats.pagerAfterLiveFollow === activationStats.pagerBeforeLiveFollow &&
    activationStats.filterPersistedDuringLiveFollow
    )],
    ["search-selection", Boolean(
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
    searchStats.overviewNodeCountAfterClear < 3_500
    )],
    ["search-dirty-guards", Boolean(
    searchStats !== null &&
    searchStats.searchGuardPassed &&
    searchStats.dirtyGuardPassed
    )],
    ["outer-containment", hasNoOuterOverflow(containment)],
  ];
  const failedChecks = checks.filter(([, passed]) => !passed).map(([label]) => label);
  const passed = failedChecks.length === 0;
  return {
    track,
    passed,
    initialStats,
    activationStats,
    searchStats,
    containment,
    failedChecks,
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
    const y = bodyRect.top + 5;
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
  await closeSceneBlockDrawer(client);
  const clippedDragStats = await runPartiallyClippedMarkerDrag(
    client,
    Number(switchStats.selectedAfterVideo),
  );
  await openSceneBlockBrowser(client);
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
  await closeSceneBlockDrawer(client);
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
  await openSceneBlockBrowser(client);
  // T3: the Finder + fixed inspector is the default editor view. Verify it,
  // exercise a row-select and an Enter-commit edit, then switch to the legacy
  // List view so every pre-T3 expectation below runs against the paged list.
  const finderStats = await client.evaluate(`(async () => {
    const raf2 = () => new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame)));
    const split = document.querySelector('[data-scene-block-finder-split]');
    const inspector = document.querySelector('[data-scene-block-inspector]');
    const rows = [...document.querySelectorAll('.sceneBlockFinderRow')];
    const firstRow = rows[0] ?? null;
    const splitVisible = Boolean(split && split.getBoundingClientRect().width > 0);
    const inspectorVisible = Boolean(inspector && inspector.getBoundingClientRect().width > 0);
    let rowSelected = false;
    let inspectorShowsRow = false;
    let inspectorStepCountLabel = '';
    let startCommitApplied = false;
    let dirtyChipAfterCommit = true;
    if (firstRow) {
      firstRow.click();
      await raf2(); await raf2();
      rowSelected = firstRow.classList.contains('selected');
      const rowId = firstRow.getAttribute('data-scene-block-id') ?? '';
      inspectorShowsRow = Boolean(inspector?.textContent?.includes('#' + rowId));
      inspectorStepCountLabel = (inspector?.querySelector('[data-scene-block-step-count]')?.textContent || '').trim();
      const startInput = inspector?.querySelector('[data-scene-block-inspector-start]');
      const marker = document.querySelector('.timelineMarker.sceneBlock[data-timeline-event-id="' + rowId + '"]');
      const beforeStartMs = Number(marker?.getAttribute('data-timeline-start-ms'));
      if (startInput instanceof HTMLInputElement && Number.isFinite(beforeStartMs)) {
        const targetMs = beforeStartMs + 250;
        startInput.focus();
        startInput.value = String(targetMs);
        startInput.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
        startInput.blur();
        startInput.dispatchEvent(new Event('blur'));
        await raf2(); await raf2();
        const afterStartMs = Number(document
          .querySelector('.timelineMarker.sceneBlock[data-timeline-event-id="' + rowId + '"]')
          ?.getAttribute('data-timeline-start-ms'));
        startCommitApplied = afterStartMs === targetMs;
        dirtyChipAfterCommit = !document.querySelector('[data-scene-block-inspector-dirty]');
        // Restore the original start so the untouched-list expectations hold.
        startInput.focus();
        startInput.value = String(beforeStartMs);
        startInput.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
        startInput.blur();
        startInput.dispatchEvent(new Event('blur'));
        await raf2(); await raf2();
      }
    }
    return {
      splitVisible,
      inspectorVisible,
      finderRowCount: rows.length,
      rowSelected,
      inspectorShowsRow,
      inspectorStepCountLabel,
      startCommitApplied,
      dirtyChipAfterCommit,
    };
  })()`);
  await client.evaluate(`document.querySelector('[data-scene-block-view-toggle="list"]')?.click()`);
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
  await closeSceneBlockDrawer(client);
  const dragGeometry = await client.evaluate(`(() => {
    const marker = document.querySelector('.timelineMarker.sceneBlock[data-timeline-event-id="500"]');
    const body = marker?.querySelector('.timelineSceneBlockBody');
    const svg = marker?.ownerSVGElement;
    if (!marker || !body || !svg) return null;
    const rect = body.getBoundingClientRect();
    const svgRect = svg.getBoundingClientRect();
    return {
      x: rect.left + rect.width / 2,
      // T4 zone contract: upper band (0-14px of the 28px block) center = Move;
      // the vertical center sits exactly on the move/select band boundary, so
      // aim a quarter of the height down to land inside the move zone.
      y: rect.top + 5,
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
      // T4 zone contract: aim the upper move band, same as the mouse drag.
      return { x: rect.left + rect.width / 2, y: rect.top + 5 };
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
  const markerBlockPropertiesStats = await client.evaluate(`(async () => {
    const marker = document.querySelector('.timelineMarker.sceneBlock[data-timeline-event-id="493"]');
    marker?.dispatchEvent(new MouseEvent('click', { bubbles: true }));
    await new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame)));
    const drawer = document.querySelector('[data-timeline-context-drawer-panel="block"]');
    const inspector = drawer?.querySelector('[data-scene-block-inspector-only="true"]');
    return {
      selectedEventId: Number(document.querySelector('.timelineMarker.sceneBlock.selected')?.getAttribute('data-timeline-event-id')),
      drawerOpen: Boolean(drawer),
      browserMode: drawer?.classList.contains('timelineBlockBrowserDrawer') ?? null,
      inspectorOnly: Boolean(inspector),
      inspectorShows493: (inspector?.textContent || '').includes('#493'),
    };
  })()`);
  await openSceneBlockBrowser(client);
  await clickVisibleSelector(client, '[data-scene-block-view-toggle="list"]');
  await sleep(140);
  const showStats = await client.evaluate(`(async () => {
    const rows = () => [...document.querySelectorAll('.sceneBlockRow')];
    const beforeRows = rows();
    const sourceOptionCount = document.querySelectorAll('.sceneBlockRow .sceneBlockSourceField option').length;
    const sourceChangeButtonCount = document.querySelectorAll('.sceneBlockRow .sceneBlockSourceButton').length;
    const composerSourceOptionCount = document.querySelectorAll('.sceneBlockComposer .sceneBlockSourceField option').length;
    const rowJumpOptionCount = document.querySelectorAll('.sceneBlockRow .sceneBlockAfterField option').length;
    const totalOptionCount = document.querySelectorAll('.sceneBlockWorkspace option').length;
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
    const search = document.querySelector('.sceneBlockRowSearch');
    if (search instanceof HTMLInputElement) {
      search.value = 'L1 cue 493';
      search.dispatchEvent(new Event('input', { bubbles: true }));
    }
    await new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame)));
    document.querySelector('.sceneBlockRow[data-scene-block-id="493"]')?.dispatchEvent(
      new MouseEvent('click', { bubbles: true }),
    );
    await new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame)));
    const repeatedSelectionVisible = rowVisible(
      document.querySelector('.sceneBlockRow.selected[data-scene-block-id="493"]'),
    );
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
    const globalSaveBlocked = document.querySelector('.appStatusLine')
      ?.getAttribute('data-status-key') === 'timeline-drafts-block-save';
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
    // Pixel-space canvas (T2): the overview viewBox now mirrors the measured
    // client box 1:1, so the resolved CSS font-size is already the rendered px.
    const renderedBlockLabelFontSize = longLabel
      ? parseFloat(getComputedStyle(longLabel).fontSize)
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
      fieldLabelFontSize: (() => {
        const element = document.querySelector('.sceneBlockRowFields label');
        return element ? parseFloat(getComputedStyle(element).fontSize) : 0;
      })(),
      semanticHintFontSize: (() => {
        const element = document.querySelector('.sceneBlockFieldLabel small');
        return element ? parseFloat(getComputedStyle(element).fontSize) : 0;
      })(),
      columnGuideFontSize: (() => {
        const element = document.querySelector('.sceneBlockColumnGuide');
        return element ? parseFloat(getComputedStyle(element).fontSize) : 0;
      })(),
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
    cue500?.scrollIntoView({ block: 'center', inline: 'nearest' });
    await new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame)));
    const cuePanel = document.querySelector('.cuePanel');
    const cueDrawer = document.querySelector('[data-timeline-context-drawer-panel="cue"]');
    const cueHeader = cue500?.querySelector('.cueMetaLine');
    const headerRect = cueHeader?.getBoundingClientRect();
    const panelRect = cuePanel?.getBoundingClientRect();
    return {
      cue500Visible: Boolean(headerRect && panelRect && headerRect.width > 0 && headerRect.height > 0 &&
        headerRect.top >= Math.max(0, panelRect.top) - 1 && headerRect.bottom <= Math.min(window.innerHeight, panelRect.bottom) + 1),
      cuePanelEditing: cuePanel?.classList.contains('cuePanelEditing') ?? false,
      cueDrawerActive: Boolean(cueDrawer && cueDrawer.getBoundingClientRect().width > 0),
      cuesDeskTabRemoved: ![...document.querySelectorAll('.timelineDeskTabs button')]
        .some((button) => (button.textContent || '').trim() === 'Cues'),
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
  // T2: the two-band scene block adds ~1000 SVG nodes at the 500-block stress
  // fixture (per-block identity band + duration <text>), lengthening the
  // getComputedStyle-forced reflow during the live transition. The maximized
  // desk roles now share the 2000ms budget already used by the similarly sized
  // extended-ceiling viewport (which reflows more pixels and stays green).
  const activeTransitionBudgetMs = role === "compact-fallback" ? 4_000 : 2_000;
  const __sblConditions = [
    ['showStats.overviewBlockCount === 500', () => Boolean(showStats.overviewBlockCount === 500)],
    ['showStats.overviewLoopLineCount <= 400', () => Boolean(showStats.overviewLoopLineCount <= 400)],
    ['showStats.overviewNodeCount < 3_500', () => Boolean(showStats.overviewNodeCount < 3_500)],
    ['idleMutationStats.records === 0', () => Boolean(idleMutationStats.records === 0)],
    ['idleMutationStats.addedNodes === 0', () => Boolean(idleMutationStats.addedNodes === 0)],
    ['idleMutationStats.removedNodes === 0', () => Boolean(idleMutationStats.removedNodes === 0)],
    ['idleMutationStats.classAttributeMutations === 0', () => Boolean(idleMutationStats.classAttributeMutations === 0)],
    ['activeTransitionStats !== null', () => Boolean(activeTransitionStats !== null)],
    ['activeTransitionStats.activeCount === 500', () => Boolean(activeTransitionStats.activeCount === 500)],
    ['activeTransitionStats.liveRowCount > 0', () => Boolean(activeTransitionStats.liveRowCount > 0)],
    ['activeTransitionStats.liveMarkerVisual.overviewExecutingLive', () => Boolean(activeTransitionStats.liveMarkerVisual.overviewExecutingLive)],
    ['activeTransitionStats.liveMarkerVisual.headerExecutingLive', () => Boolean(activeTransitionStats.liveMarkerVisual.headerExecutingLive)],
    ['activeTransitionStats.liveMarkerVisual.perMarkerActiveClassCount === 0', () => Boolean(activeTransitionStats.liveMarkerVisual.perMarkerActiveClassCount === 0)],
    ['activeTransitionStats.liveMarkerVisual.markerId !== ""', () => Boolean(activeTransitionStats.liveMarkerVisual.markerId !== '')],
    ['!activeTransitionStats.stoppedMarkerVisual.overviewExecutingLive', () => Boolean(!activeTransitionStats.stoppedMarkerVisual.overviewExecutingLive)],
    ['!activeTransitionStats.stoppedMarkerVisual.headerExecutingLive', () => Boolean(!activeTransitionStats.stoppedMarkerVisual.headerExecutingLive)],
    ['activeTransitionStats.stoppedMarkerVisual.perMarkerActiveClassCount === 0', () => Boolean(activeTransitionStats.stoppedMarkerVisual.perMarkerActiveClassCount === 0)],
    ['activeTransitionStats.liveMarkerVisual.stroke !== activeTransitionStats.stoppedMarkerVisua', () => Boolean(activeTransitionStats.liveMarkerVisual.stroke !== activeTransitionStats.stoppedMarkerVisual.stroke)],
    ['activeTransitionStats.liveMarkerVisual.strokeDasharray !== activeTransitionStats.stoppedMa', () => Boolean(activeTransitionStats.liveMarkerVisual.strokeDasharray !== activeTransitionStats.stoppedMarkerVisual.strokeDasharray)],
    ['activeTransitionStats.stoppedMarkerVisual.strokeDasharray !== "none"', () => Boolean(activeTransitionStats.stoppedMarkerVisual.strokeDasharray !== 'none')],
    ['activeTransitionStats.stoppedActiveCount === 0', () => Boolean(activeTransitionStats.stoppedActiveCount === 0)],
    ['activeTransitionStats.stoppedUnderPlayheadCount === 500', () => Boolean(activeTransitionStats.stoppedUnderPlayheadCount === 500)],
    ['activeTransitionStats.stoppedLiveRowCount === 0', () => Boolean(activeTransitionStats.stoppedLiveRowCount === 0)],
    ['activeTransitionStats.childListMutations === 0', () => Boolean(activeTransitionStats.childListMutations === 0)],
    ['activeTransitionStats.addedNodes === 0', () => Boolean(activeTransitionStats.addedNodes === 0)],
    ['activeTransitionStats.removedNodes === 0', () => Boolean(activeTransitionStats.removedNodes === 0)],
    ['activeTransitionStats.classAttributeMutations <= 4', () => Boolean(activeTransitionStats.classAttributeMutations <= 4)],
    ['activeTransitionStats.markerIdentityStable', () => Boolean(activeTransitionStats.markerIdentityStable)],
    ['activeTransitionStats.elapsedMs < activeTransitionBudgetMs', () => Boolean(activeTransitionStats.elapsedMs < activeTransitionBudgetMs)],
    ['finderStats.splitVisible', () => Boolean(finderStats?.splitVisible)],
    ['finderStats.inspectorVisible', () => Boolean(finderStats?.inspectorVisible)],
    ['finderStats.finderRowCount > 0', () => Boolean((finderStats?.finderRowCount ?? 0) > 0)],
    ['finderStats.rowSelected', () => Boolean(finderStats?.rowSelected)],
    ['finderStats.inspectorShowsRow', () => Boolean(finderStats?.inspectorShowsRow)],
    ['finderStats.inspectorStepCountLabel === "3 Static step(s)"', () => Boolean(finderStats?.inspectorStepCountLabel === '3 Static step(s)')],
    ['finderStats.startCommitApplied', () => Boolean(finderStats?.startCommitApplied)],
    ['finderStats.dirtyChipAfterCommit', () => Boolean(finderStats?.dirtyChipAfterCommit)],
    ['markerDragStats !== null', () => Boolean(markerDragStats !== null)],
    ['Math.abs(markerDragStats.tinyMoveX - markerDragStats.initialX) < 1', () => Boolean(Math.abs(markerDragStats.tinyMoveX - markerDragStats.initialX) < 1)],
    ['Math.abs(markerDragStats.committedMoveX - markerDragStats.expectedCommittedX) < 2', () => Boolean(Math.abs(markerDragStats.committedMoveX - markerDragStats.expectedCommittedX) < 2)],
    ['Math.abs(markerDragStats.settledMouseX - markerDragStats.committedMoveX) < 2', () => Boolean(Math.abs(markerDragStats.settledMouseX - markerDragStats.committedMoveX) < 2)],
    ['markerDragStats.markerTouchAction === "none"', () => Boolean(markerDragStats.markerTouchAction === 'none')],
    ['markerDragStats.nonMousePointerType === "pen"', () => Boolean(markerDragStats.nonMousePointerType === 'pen')],
    ['Math.abs(markerDragStats.nonMouseTinyMoveX - markerDragStats.nonMouseInitialX) < 1', () => Boolean(Math.abs(markerDragStats.nonMouseTinyMoveX - markerDragStats.nonMouseInitialX) < 1)],
    ['Math.abs(markerDragStats.nonMouseCommittedMoveX - markerDragStats.expectedNonMouseCommitte', () => Boolean(Math.abs(markerDragStats.nonMouseCommittedMoveX - markerDragStats.expectedNonMouseCommittedX) < 2)],
    ['Math.abs(markerDragStats.settledNonMouseX - markerDragStats.nonMouseCommittedMoveX) < 2', () => Boolean(Math.abs(markerDragStats.settledNonMouseX - markerDragStats.nonMouseCommittedMoveX) < 2)],
    ['markerBlockPropertiesStats.selectedEventId === 493', () => Boolean(markerBlockPropertiesStats.selectedEventId === 493)],
    ['markerBlockPropertiesStats.drawerOpen', () => Boolean(markerBlockPropertiesStats.drawerOpen)],
    ['markerBlockPropertiesStats.browserMode === false', () => Boolean(markerBlockPropertiesStats.browserMode === false)],
    ['markerBlockPropertiesStats.inspectorOnly', () => Boolean(markerBlockPropertiesStats.inspectorOnly)],
    ['markerBlockPropertiesStats.inspectorShows493', () => Boolean(markerBlockPropertiesStats.inspectorShows493)],
    ['showStats.beforeRowCount <= 12', () => Boolean(showStats.beforeRowCount <= 12)],
    ['showStats.afterRowCount <= 12', () => Boolean(showStats.afterRowCount <= 12)],
    ['showStats.sourceOptionCount === 0', () => Boolean(showStats.sourceOptionCount === 0)],
    ['showStats.sourceChangeButtonCount === showStats.beforeRowCount', () => Boolean(showStats.sourceChangeButtonCount === showStats.beforeRowCount)],
    ['showStats.composerSourceOptionCount === 0', () => Boolean(showStats.composerSourceOptionCount === 0)],
    ['showStats.rowJumpOptionCount <= 48', () => Boolean(showStats.rowJumpOptionCount <= 48)],
    ['showStats.totalOptionCount < 200', () => Boolean(showStats.totalOptionCount < 200)],
    ['showStats.pagerCount === 1', () => Boolean(showStats.pagerCount === 1)],
    ['showStats.pagerLabel.includes("493-500 / 500")', () => Boolean(showStats.pagerLabel.includes('493-500 / 500'))],
    ['showStats.reachedLastBlock', () => Boolean(showStats.reachedLastBlock)],
    ['(!requiresFullScaleVisualSignoff || showStats.repeatedSelectionVisible)', () => Boolean((!requiresFullScaleVisualSignoff || showStats.repeatedSelectionVisible))],
    ['showStats.cueIdentitySearchReached500', () => Boolean(showStats.cueIdentitySearchReached500)],
    ['showStats.selectedMarkerRenderedLast', () => Boolean(showStats.selectedMarkerRenderedLast)],
    ['showStats.cleanSaveDisabled', () => Boolean(showStats.cleanSaveDisabled)],
    ['showStats.dirtyClassVisible', () => Boolean(showStats.dirtyClassVisible)],
    ['showStats.dirtyBadgeVisible', () => Boolean(showStats.dirtyBadgeVisible)],
    ['showStats.dirtySaveEnabled', () => Boolean(showStats.dirtySaveEnabled)],
    ['showStats.topbarDirtyVisible', () => Boolean(showStats.topbarDirtyVisible)],
    ['showStats.projectLabelDirtyVisible', () => Boolean(showStats.projectLabelDirtyVisible)],
    ['showStats.globalSaveBlocked', () => Boolean(showStats.globalSaveBlocked)],
    ['showStats.cleanAfterRevert', () => Boolean(showStats.cleanAfterRevert)],
    ['showStats.overviewBackgroundFits', () => Boolean(showStats.overviewBackgroundFits)],
    ['(!requiresFullScaleVisualSignoff || showStats.renderedBlockLabelFontSize >= 9)', () => Boolean((!requiresFullScaleVisualSignoff || showStats.renderedBlockLabelFontSize >= 9))],
    ['showStats.longLabelWithinBody', () => Boolean(showStats.longLabelWithinBody)],
    ['showStats.sharedJumpPickerCount === 0', () => Boolean(showStats.sharedJumpPickerCount === 0)],
    ['showStats.fieldLabelFontSize >= 10', () => Boolean(showStats.fieldLabelFontSize >= 10)],
    ['showStats.semanticHintFontSize >= 9', () => Boolean(showStats.semanticHintFontSize >= 9)],
    ['showStats.columnGuideFontSize >= 9', () => Boolean(showStats.columnGuideFontSize >= 9)],
    ['showStats.timeLabel.includes(" / ")', () => Boolean(showStats.timeLabel.includes(' / '))],
    ['showStats.timeTitle.startsWith("1000 / ")', () => Boolean(showStats.timeTitle.startsWith('1000 / '))],
    ['showStats.timeTitle.endsWith(" ms")', () => Boolean(showStats.timeTitle.endsWith(' ms'))],
    ['!showStats.timeStatClipped', () => Boolean(!showStats.timeStatClipped)],
    ['sourcePickerStats !== null', () => Boolean(sourcePickerStats !== null)],
    ['sourcePickerStats.pickerCount === 1', () => Boolean(sourcePickerStats.pickerCount === 1)],
    ['sourcePickerStats.initialOptionCount <= 80', () => Boolean(sourcePickerStats.initialOptionCount <= 80)],
    ['sourcePickerStats.initialSelectedCueId === sourcePickerStats.expectedCueId', () => Boolean(sourcePickerStats.initialSelectedCueId === sourcePickerStats.expectedCueId)],
    ['sourcePickerStats.initialSelectedLabel.includes("Scale Cue 500")', () => Boolean(sourcePickerStats.initialSelectedLabel.includes('Scale Cue 500'))],
    ['sourcePickerStats.noMatchOptionCount === 0', () => Boolean(sourcePickerStats.noMatchOptionCount === 0)],
    ['sourcePickerStats.applyDisabledWithNoMatches', () => Boolean(sourcePickerStats.applyDisabledWithNoMatches)],
    ['sourcePickerStats.restoredCueId === sourcePickerStats.expectedCueId', () => Boolean(sourcePickerStats.restoredCueId === sourcePickerStats.expectedCueId)],
    ['sourcePickerStats.preservedAfterApply', () => Boolean(sourcePickerStats.preservedAfterApply)],
    ['sourcePickerStats.closedAfterApply', () => Boolean(sourcePickerStats.closedAfterApply)],
    ['sourcePickerStats.filteredOptionCount === 1', () => Boolean(sourcePickerStats.filteredOptionCount === 1)],
    ['sourcePickerStats.filteredLabel.includes("Scale Cue 500")', () => Boolean(sourcePickerStats.filteredLabel.includes('Scale Cue 500'))],
    ['sourcePickerStats.reachable', () => Boolean(sourcePickerStats.reachable)],
    ['sourcePickerStats.controlCount > 0', () => Boolean(sourcePickerStats.controlCount > 0)],
    ['sourcePickerStats.fullyVisibleControlCount === sourcePickerStats.controlCount', () => Boolean(sourcePickerStats.fullyVisibleControlCount === sourcePickerStats.controlCount)],
    ['sourcePickerStats.closedAfterCancel', () => Boolean(sourcePickerStats.closedAfterCancel)],
    ['pickerStats.pickerCount === 1', () => Boolean(pickerStats.pickerCount === 1)],
    ['pickerStats.sourcePickerCountBeforeJump === 1', () => Boolean(pickerStats.sourcePickerCountBeforeJump === 1)],
    ['pickerStats.sourcePickerCountAfterJump === 0', () => Boolean(pickerStats.sourcePickerCountAfterJump === 0)],
    ['pickerStats.pickerOptionCount <= 80', () => Boolean(pickerStats.pickerOptionCount <= 80)],
    ['pickerStats.pagerCount === 1', () => Boolean(pickerStats.pagerCount === 1)],
    ['pickerStats.rowCount <= 12', () => Boolean(pickerStats.rowCount <= 12)],
    ['pickerStats.rowActionCount > 0', () => Boolean(pickerStats.rowActionCount > 0)],
    ['pickerStats.listHeight > 0', () => Boolean(pickerStats.listHeight > 0)],
    ['pickerStats.pickerReachable', () => Boolean(pickerStats.pickerReachable)],
    ['pickerStats.pickerControlCount > 0', () => Boolean(pickerStats.pickerControlCount > 0)],
    ['pickerStats.fullyVisiblePickerControlCount === pickerStats.pickerControlCount', () => Boolean(pickerStats.fullyVisiblePickerControlCount === pickerStats.pickerControlCount)],
    ['pickerStats.pagerReachable', () => Boolean(pickerStats.pagerReachable)],
    ['pickerStats.lastActionReachable', () => Boolean(pickerStats.lastActionReachable)],
    ['composerPickerStats.sourceInitialOptionCount <= 80', () => Boolean(composerPickerStats.sourceInitialOptionCount <= 80)],
    ['composerPickerStats.sourceFilteredOptionCount === 1', () => Boolean(composerPickerStats.sourceFilteredOptionCount === 1)],
    ['composerPickerStats.sourceClosedAfterApply', () => Boolean(composerPickerStats.sourceClosedAfterApply)],
    ['composerPickerStats.jumpInitialOptionCount <= 80', () => Boolean(composerPickerStats.jumpInitialOptionCount <= 80)],
    ['composerPickerStats.jumpFilteredOptionCount >= 1', () => Boolean(composerPickerStats.jumpFilteredOptionCount >= 1)],
    ['composerPickerStats.jumpFilteredOptionCount <= 80', () => Boolean(composerPickerStats.jumpFilteredOptionCount <= 80)],
    ['composerPickerStats.jumpApplyEnabledAfterChoice', () => Boolean(composerPickerStats.jumpApplyEnabledAfterChoice)],
    ['composerPickerStats.jumpClosedAfterApply', () => Boolean(composerPickerStats.jumpClosedAfterApply)],
    ['composerPickerStats.composerEagerOptionCount === 0', () => Boolean(composerPickerStats.composerEagerOptionCount === 0)],
    ['sourceOpenStats.cue500Visible', () => Boolean(sourceOpenStats.cue500Visible)],
    ['sourceOpenStats.cuePanelEditing', () => Boolean(sourceOpenStats.cuePanelEditing)],
    ['sourceOpenStats.cueDrawerActive', () => Boolean(sourceOpenStats.cueDrawerActive)],
    ['sourceOpenStats.cuesDeskTabRemoved', () => Boolean(sourceOpenStats.cuesDeskTabRemoved)],
    ['sourceOpenStats.pagerLabel.includes("42 / 42")', () => Boolean(sourceOpenStats.pagerLabel.includes('42 / 42'))],
    ['cueStats.cueRowCount === 12', () => Boolean(cueStats.cueRowCount === 12)],
    ['cueStats.placementChipCount === 8', () => Boolean(cueStats.placementChipCount === 8)],
    ['cueStats.placementChipButtonCount === 24', () => Boolean(cueStats.placementChipButtonCount === 24)],
    ['cueStats.placementMoreButtonCount === 1', () => Boolean(cueStats.placementMoreButtonCount === 1)],
    ['cueStats.placementMoreLabel.includes("491 more")', () => Boolean(cueStats.placementMoreLabel.includes('491 more'))],
    ['(!shouldCaptureViewport(viewport) || screenshotVerification?.verified === true)', () => Boolean((!shouldCaptureViewport(viewport) || screenshotVerification?.verified === true))],
    ['hasNoOuterOverflow(showContainment)', () => Boolean(hasNoOuterOverflow(showContainment))],
    ['hasNoOuterOverflow(pickerContainment)', () => Boolean(hasNoOuterOverflow(pickerContainment))],
    ['hasNoOuterOverflow(cueContainment)', () => Boolean(hasNoOuterOverflow(cueContainment))],
  ];
  const failedConditions = __sblConditions.filter(([, check]) => { try { return !check(); } catch { return true; } }).map(([label]) => label);
  const passed = failedConditions.length === 0;
  if (!passed) {
    console.log(`scene-block-large-${viewport.width}x${viewport.height} FAILED CONDITIONS: ${JSON.stringify(failedConditions)}`);
    if (failedConditions.some((label) => label.includes('markerDragStats'))) {
      console.log(`scene-block-large-${viewport.width}x${viewport.height} DRAG STATS: ${JSON.stringify(markerDragStats)}`);
    }
  }
  return {
    label: `scene-block-large-${viewport.width}x${viewport.height}`,
    passed,
    showStats,
    idleMutationStats,
    activeTransitionStats,
    markerDragStats,
    markerBlockPropertiesStats,
    finderStats,
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
  await clickByText(client, "Mixer");
  await sleep(180);
  if (shouldCaptureViewport(viewport)) {
    mkdirSync(screenshotDir, { recursive: true });
    const screenshot = await client.send("Page.captureScreenshot", { format: "png", fromSurface: true });
    writeFileSync(join(screenshotDir, `vj-empty-${viewport.width}x${viewport.height}.png`), screenshot.data, "base64");
  }
  return measure(client, `vj-empty-${viewport.width}x${viewport.height}`);
}

const fxVisualFamilyOrder = [
  "STEPS",
  "COLOR FX",
  "CHASER FX",
  "MOVE FX",
  "VALUE FX",
  "CURVE FX",
  "MAPPINGS",
  "COLOR MAPPINGS",
  "SUPER SCENE",
];

const fxVisualRecipeFamilies = [
  ["COLOR FX", "Color", 1],
  ["CHASER FX", "Chaser", 1],
  ["MOVE FX", "Move", 1],
  ["VALUE FX", "Value", 5],
  ["CURVE FX", "Curve", 1],
  ["MAPPINGS", "Mapping", 3],
  ["COLOR MAPPINGS", "ColorMapping", 1],
];

async function readFxVisualSurface(client) {
  return evaluatePageFunction(client, async (expectedFamilies) => {
    const visible = (element) => {
      if (!element) return false;
      const rectangle = element.getBoundingClientRect();
      const style = getComputedStyle(element);
      return rectangle.width > 0 && rectangle.height > 0 && style.display !== "none" && style.visibility !== "hidden";
    };
    const within = (inner, outer) => Boolean(
      inner && outer &&
      inner.left >= outer.left - 1 && inner.right <= outer.right + 1 &&
      inner.top >= outer.top - 1 && inner.bottom <= outer.bottom + 1
    );
    const chooser = document.querySelector(".effectFamilyChooser");
    const chooserRect = chooser?.getBoundingClientRect() ?? null;
    const familyButtons = [...document.querySelectorAll(".effectFamilyChooser button")].map((button) => {
      const rectangle = button.getBoundingClientRect();
      const family = button.getAttribute("data-effect-family") || "";
      return {
        family,
        label: (button.querySelector("strong")?.textContent || "").trim(),
        order: Number(button.getAttribute("data-family-order")),
        active: button.classList.contains("active") && button.getAttribute("aria-pressed") === "true",
        visible: visible(button),
        contained: within(rectangle, chooserRect) && rectangle.left >= -1 && rectangle.right <= innerWidth + 1,
        width: Math.round(rectangle.width * 100) / 100,
        height: Math.round(rectangle.height * 100) / 100,
      };
    });
    const rack = document.querySelector(".effectRackPane");
    const effectList = document.querySelector(".effectList");
    const layoutRect = (selector) => {
      const rectangle = document.querySelector(selector)?.getBoundingClientRect();
      return rectangle ? {
        width: Math.round(rectangle.width * 100) / 100,
        height: Math.round(rectangle.height * 100) / 100,
      } : { width: 0, height: 0 };
    };
    const previews = [...document.querySelectorAll(".effectListRows .effectGraphicalPreview")].map((preview) => {
      const rectangle = preview.getBoundingClientRect();
      const summaryRect = preview.closest(".effectItemSummary")?.getBoundingClientRect() ?? null;
      const curve = preview.querySelector(".effectGraphicalCurve");
      const gradient = preview.querySelector(".effectGraphicalGradient");
      const move = preview.querySelector(".effectGraphicalMove");
      const chaser = preview.querySelector(".effectGraphicalChaser");
      const colorMapping = preview.querySelector(".effectGraphicalColorMap");
      return {
        kind: preview.getAttribute("data-preview-kind") || "",
        label: preview.getAttribute("data-effect-label") || "",
        width: Math.round(rectangle.width * 100) / 100,
        height: Math.round(rectangle.height * 100) / 100,
        containedInItem: within(rectangle, summaryRect),
        horizontalOverflowPx: Math.max(0, preview.scrollWidth - preview.clientWidth),
        shape: curve?.getAttribute("data-lfo-shape") ?? "",
        low: curve?.getAttribute("data-lfo-low") ?? "",
        high: curve?.getAttribute("data-lfo-high") ?? "",
        phase: curve?.getAttribute("data-lfo-phase") ?? "",
        curvePath: curve?.querySelector("path")?.getAttribute("d") ?? "",
        stopCount: gradient?.getAttribute("data-stop-count") ?? "",
        stopPositions: gradient?.getAttribute("data-stop-positions") ?? "",
        stopColors: gradient?.getAttribute("data-stop-colors") ?? "",
        colorInterpolation: gradient?.getAttribute("data-color-interpolation") ?? "",
        gradientBackground: gradient ? getComputedStyle(gradient).backgroundImage : "",
        movePointCount: move?.getAttribute("data-point-count") ?? "",
        movePoints: move?.getAttribute("data-move-points") ?? "",
        moveCenter: move?.getAttribute("data-move-center") ?? "",
        moveSize: move?.getAttribute("data-move-size") ?? "",
        moveRotation: move?.getAttribute("data-move-rotation") ?? "",
        moveCoordinateMode: move?.getAttribute("data-move-coordinate-mode") ?? "",
        movePreviewBase: move?.getAttribute("data-move-preview-base") ?? "",
        moveControlPath: move?.querySelector("path.control")?.getAttribute("d") ?? "",
        moveOutputPath: move?.querySelector("path.output")?.getAttribute("d") ?? "",
        moveOutputInViewBox: (() => {
          const values = (move?.querySelector("path.output")?.getAttribute("d") ?? "")
            .match(/-?\d+(?:\.\d+)?/g)
            ?.map(Number) ?? [];
          return values.length > 0 && values.every((value) => value >= 0 && value <= 100);
        })(),
        valuePoints: curve?.getAttribute("data-value-points") ?? "",
        valueInterpolation: curve?.getAttribute("data-value-interpolation") ?? "",
        curvePoints: curve?.getAttribute("data-curve-points") ?? "",
        mappingDirection: curve?.getAttribute("data-mapping-direction") ?? "",
        mappingRepetitions: curve?.getAttribute("data-mapping-repetitions") ?? "",
        mappingFixtureOrder: curve?.getAttribute("data-mapping-fixture-order") ?? "",
        colorMappingRaster: colorMapping?.getAttribute("data-raster") ?? "",
        colorMappingFrameCount: colorMapping?.getAttribute("data-frame-count") ?? "",
        colorMappingCellCount: colorMapping?.getAttribute("data-cell-count") ?? "",
        colorMappingSourceKind: colorMapping?.getAttribute("data-source-kind") ?? "",
        colorMappingDirection: colorMapping?.getAttribute("data-playback-direction") ?? "",
        colorMappingWrapMode: colorMapping?.getAttribute("data-wrap-mode") ?? "",
        colorMappingSampling: colorMapping?.getAttribute("data-sampling") ?? "",
        colorMappingPixelCount: colorMapping?.querySelectorAll("i").length ?? 0,
        chaserStepCount: chaser?.getAttribute("data-step-count") ?? "",
        chaserActiveStepCount: chaser?.getAttribute("data-active-step-count") ?? "",
      };
    });
    const effectListInitialScrollTop = effectList?.scrollTop ?? 0;
    const lastPreview = [...document.querySelectorAll(".effectListRows .effectGraphicalPreview")].at(-1) ?? null;
    if (effectList && lastPreview) {
      const listRectangle = effectList.getBoundingClientRect();
      const previewRectangle = lastPreview.getBoundingClientRect();
      if (previewRectangle.bottom > listRectangle.bottom - 1) {
        effectList.scrollTop += previewRectangle.bottom - listRectangle.bottom + 1;
      } else if (previewRectangle.top < listRectangle.top + 1) {
        effectList.scrollTop -= listRectangle.top - previewRectangle.top + 1;
      }
    }
    await new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame)));
    const lastPreviewRectAtEnd = lastPreview?.getBoundingClientRect() ?? null;
    const effectListRectAtEnd = effectList?.getBoundingClientRect() ?? null;
    const lastPreviewReachable = Boolean(
      lastPreviewRectAtEnd && effectListRectAtEnd &&
      lastPreviewRectAtEnd.left >= effectListRectAtEnd.left - 1 &&
      lastPreviewRectAtEnd.right <= effectListRectAtEnd.right + 1 &&
      lastPreviewRectAtEnd.top >= effectListRectAtEnd.top - 1 &&
      lastPreviewRectAtEnd.bottom <= effectListRectAtEnd.bottom + 1
    );
    const effectListScrollMetrics = {
      client: [effectList?.clientWidth ?? 0, effectList?.clientHeight ?? 0],
      scroll: [effectList?.scrollWidth ?? 0, effectList?.scrollHeight ?? 0],
      scrollTopAtEnd: effectList?.scrollTop ?? 0,
      listRect: effectListRectAtEnd ? [
        effectListRectAtEnd.left,
        effectListRectAtEnd.top,
        effectListRectAtEnd.right,
        effectListRectAtEnd.bottom,
      ] : null,
      lastPreviewRect: lastPreviewRectAtEnd ? [
        lastPreviewRectAtEnd.left,
        lastPreviewRectAtEnd.top,
        lastPreviewRectAtEnd.right,
        lastPreviewRectAtEnd.bottom,
      ] : null,
    };
    if (effectList) effectList.scrollTop = effectListInitialScrollTop;
    await new Promise((resolveFrame) => requestAnimationFrame(resolveFrame));
    const documentAndAppScrollZero =
      window.scrollX === 0 && window.scrollY === 0 &&
      document.documentElement.scrollLeft === 0 && document.documentElement.scrollTop === 0 &&
      document.body.scrollLeft === 0 && document.body.scrollTop === 0 &&
      (document.querySelector(".app")?.scrollLeft ?? 0) === 0 &&
      (document.querySelector(".app")?.scrollTop ?? 0) === 0;
    return {
      expectedFamilies,
      familyButtons,
      familyNames: familyButtons.map((button) => button.family),
      familyLabels: familyButtons.map((button) => button.label),
      activeFamilies: familyButtons.filter((button) => button.active).map((button) => button.family),
      chooserHorizontalOverflowPx: chooser ? Math.max(0, chooser.scrollWidth - chooser.clientWidth) : -1,
      chooserVerticalOverflowPx: chooser ? Math.max(0, chooser.scrollHeight - chooser.clientHeight) : -1,
      rackHorizontalOverflowPx: rack ? Math.max(0, rack.scrollWidth - rack.clientWidth) : -1,
      rackVerticalOverflowPx: rack ? Math.max(0, rack.scrollHeight - rack.clientHeight) : -1,
      rackOverflowY: rack ? getComputedStyle(rack).overflowY : "",
      effectListHorizontalOverflowPx: effectList ? Math.max(0, effectList.scrollWidth - effectList.clientWidth) : -1,
      effectListVerticalOverflowPx: effectList ? Math.max(0, effectList.scrollHeight - effectList.clientHeight) : -1,
      effectListOverflowY: effectList ? getComputedStyle(effectList).overflowY : "",
      lastPreviewReachable,
      effectListScrollMetrics,
      workbenchRect: layoutRect(".effectWorkbench"),
      libraryPaneRect: layoutRect(".effectLibraryPane"),
      inspectorPaneRect: layoutRect(".effectInspectorPane"),
      rackPaneRect: layoutRect(".effectRackPane"),
      previewKinds: previews.map((preview) => preview.kind),
      previews,
      documentAndAppScrollZero,
      appSize: (() => {
        const rectangle = document.querySelector(".app")?.getBoundingClientRect();
        return rectangle ? [Math.round(rectangle.width), Math.round(rectangle.height)] : [0, 0];
      })(),
    };
  }, fxVisualFamilyOrder);
}

async function readMoveFxPointState(client, requestedPointIndex = null) {
  return evaluatePageFunction(client, (pointIndex) => {
    const handles = [...document.querySelectorAll(".moveEffectPathCanvas .moveEffectPointHandle")];
    const handleCandidates = handles.map((candidate, index) => {
      const rectangle = candidate.getBoundingClientRect();
      const center = { x: rectangle.left + rectangle.width / 2, y: rectangle.top + rectangle.height / 2 };
      const hit = center.x >= 0 && center.x < innerWidth && center.y >= 0 && center.y < innerHeight
        ? document.elementFromPoint(center.x, center.y)
        : null;
      return {
        candidate,
        index,
        rectangle,
        center,
        hit,
        hitTestable: Boolean(hit && (hit === candidate || hit.closest(".moveEffectPointHandle") === candidate)),
      };
    });
    const selectedHandle = (
      Number.isInteger(pointIndex) ? handleCandidates.find((entry) => entry.index === pointIndex) : null
    ) ?? handleCandidates.find((entry) => entry.hitTestable) ?? handleCandidates[0] ?? null;
    const handle = selectedHandle?.candidate ?? null;
    const canvas = document.querySelector(".moveEffectPathCanvas");
    const rectangle = selectedHandle?.rectangle ?? null;
    const canvasRect = canvas?.getBoundingClientRect() ?? null;
    const canvasMatrix = canvas instanceof SVGGraphicsElement ? canvas.getScreenCTM() : null;
    const label = handle?.getAttribute("aria-label") ?? "";
    const match = label.match(/X\s+(-?[0-9.]+),\s+Y\s+(-?[0-9.]+)/);
    const outputValues = (document.querySelector(".moveEffectPathOutput")?.getAttribute("d") ?? "")
      .match(/-?\d+(?:\.\d+)?/g)
      ?.map(Number) ?? [];
    return {
      exists: Boolean(handle && rectangle && rectangle.width > 0 && rectangle.height > 0),
      pointIndex: selectedHandle?.index ?? -1,
      hitTestable: selectedHandle?.hitTestable ?? false,
      hitClass: selectedHandle?.hit?.getAttribute("class") ?? "",
      label,
      x: match ? Number(match[1]) : null,
      y: match ? Number(match[2]) : null,
      transform: handle?.getAttribute("transform") ?? "",
      center: rectangle ? { x: rectangle.left + rectangle.width / 2, y: rectangle.top + rectangle.height / 2 } : null,
      canvas: canvasRect ? {
        left: canvasRect.left,
        top: canvasRect.top,
        right: canvasRect.right,
        bottom: canvasRect.bottom,
        width: canvasRect.width,
        height: canvasRect.height,
      } : null,
      canvasScale: canvasMatrix ? {
        x: Math.hypot(canvasMatrix.a, canvasMatrix.b),
        y: Math.hypot(canvasMatrix.c, canvasMatrix.d),
      } : null,
      ghostCount: document.querySelectorAll("[data-move-point-ghost]").length,
      readoutCount: document.querySelectorAll("[data-move-point-drag-readout]").length,
      readout: (document.querySelector("[data-move-point-drag-readout]")?.textContent ?? "").replace(/\s+/g, " ").trim(),
      outputPathInViewBox: outputValues.length > 0 && outputValues.every((value) => value >= 0 && value <= 100),
      documentAndAppScrollZero:
        window.scrollX === 0 && window.scrollY === 0 &&
        document.documentElement.scrollLeft === 0 && document.documentElement.scrollTop === 0 &&
        document.body.scrollLeft === 0 && document.body.scrollTop === 0 &&
        (document.querySelector(".app")?.scrollLeft ?? 0) === 0 &&
        (document.querySelector(".app")?.scrollTop ?? 0) === 0,
    };
  }, requestedPointIndex);
}

async function exerciseMoveFxDrag(client, deltaX, deltaY, escape = false) {
  const before = await readMoveFxPointState(client);
  if (!before.center) return { before, during: null, after: null };
  const target = { x: before.center.x + deltaX, y: before.center.y + deltaY };
  await client.send("Input.dispatchMouseEvent", {
    type: "mousePressed",
    x: before.center.x,
    y: before.center.y,
    button: "left",
    buttons: 1,
    clickCount: 1,
  });
  await client.send("Input.dispatchMouseEvent", {
    type: "mouseMoved",
    x: target.x,
    y: target.y,
    button: "left",
    buttons: 1,
  });
  await sleep(64);
  const during = await readMoveFxPointState(client, before.pointIndex);
  if (escape) {
    await pressKey(client, "Escape", "Escape");
    await sleep(48);
  }
  await client.send("Input.dispatchMouseEvent", {
    type: "mouseReleased",
    x: target.x,
    y: target.y,
    button: "left",
    buttons: 0,
    clickCount: 1,
  });
  await sleep(96);
  const after = await readMoveFxPointState(client, before.pointIndex);
  return { before, during, after, deltaX, deltaY, escape };
}

async function readSceneLiveModifierState(client, cueId) {
  return evaluatePageFunction(client, (cueId) => {
    const strip = document.querySelector(`[data-cue-live-modifier="${cueId}"]`);
    const app = document.querySelector(".app");
    return {
      stripCueId: strip?.getAttribute("data-cue-live-modifier") ?? null,
      override: strip?.getAttribute("data-live-override") ?? null,
      readouts: strip
        ? [...strip.querySelectorAll(".cueLiveModifierRow b")].map((element) =>
            (element.textContent ?? "").trim(),
          )
        : [],
      direction: strip?.querySelector(`[data-cue-live-modifier-direction="${cueId}"]`)?.value ?? null,
      segment: strip?.querySelector(`[data-cue-live-modifier-segment="${cueId}"]`)?.value ?? null,
      resetDisabled: strip?.querySelector(".cueLiveModifierReset")?.disabled ?? null,
      activeCardIds: [...document.querySelectorAll('[data-scene-matrix-active="true"]')].map(
        (element) => element.getAttribute("data-scene-matrix-cue-id"),
      ),
      matrixFlashCells: document.querySelectorAll("[data-scene-flash-cue]").length,
      touchFlashPads: document.querySelectorAll(".touchPlacedButton[data-touch-flash-cue]").length,
      // The Touch surface proves activation through the same runtime truth
      // the operator sees: the active scene's live strip.
      touchActivePadIds: [
        ...document.querySelectorAll(".cueLiveModifierStrip.touch[data-cue-live-modifier]"),
      ].map((element) => element.getAttribute("data-cue-live-modifier")),
      containmentZero:
        window.scrollX === 0 &&
        window.scrollY === 0 &&
        document.documentElement.scrollLeft === 0 &&
        document.documentElement.scrollTop === 0 &&
        document.body.scrollLeft === 0 &&
        document.body.scrollTop === 0 &&
        (app?.scrollLeft ?? 0) === 0 &&
        (app?.scrollTop ?? 0) === 0,
    };
  }, cueId);
}

async function setSceneLiveModifierSlider(client, cueId, control, value) {
  return evaluatePageFunction(
    client,
    (cueId, control, value) => {
      const input = document.querySelector(`[data-cue-live-modifier-${control}="${cueId}"]`);
      if (!input) return false;
      input.value = String(value);
      input.dispatchEvent(new Event("input", { bubbles: true }));
      return true;
    },
    cueId,
    control,
    value,
  );
}

async function setSceneLiveModifierSelect(client, cueId, control, value) {
  return evaluatePageFunction(
    client,
    (cueId, control, value) => {
      const input = document.querySelector(`[data-cue-live-modifier-${control}="${cueId}"]`);
      if (!input) return false;
      input.value = String(value);
      input.dispatchEvent(new Event("input", { bubbles: true }));
      return true;
    },
    cueId,
    control,
    value,
  );
}

async function dispatchFlashPointer(client, selector, press) {
  const center = await evaluatePageFunction(
    client,
    (selector) => {
      const cell = document.querySelector(selector);
      if (!cell) return null;
      // Cells can sit below the internal panel fold on compact viewports;
      // panel-internal scrolling is allowed, off-viewport clicks are not.
      cell.scrollIntoView({ block: "nearest", inline: "nearest" });
      const rectangle = cell.getBoundingClientRect();
      const x = rectangle.left + rectangle.width / 2;
      const y = rectangle.top + rectangle.height / 2;
      const hit = document.elementFromPoint(x, y);
      return {
        x,
        y,
        hitSelf: Boolean(hit && (hit === cell || cell.contains(hit))),
        hitClass: hit?.getAttribute("class") ?? "",
        rect: [rectangle.left, rectangle.top, rectangle.width, rectangle.height].map(Math.round),
      };
    },
    selector,
  );
  if (!center) return false;
  if (viewportTraceEnabled && !center.hitSelf) {
    console.log(
      `flash pointer target occluded: ${selector} hit=${center.hitClass} rect=${center.rect.join(",")}`,
    );
  }
  await client.send("Input.dispatchMouseEvent", {
    type: press ? "mousePressed" : "mouseReleased",
    x: center.x,
    y: center.y,
    button: "left",
    buttons: press ? 1 : 0,
    clickCount: 1,
  });
  return true;
}

// T17 acceptance: the Scene Matrix cell and the Touch pad drive the same
// latched live-modifier path, flash pads are strictly momentary, and the
// latch resets on reset/retrigger while document/.app scroll stays zero.
async function runSceneLiveModifierViewport(client, viewport) {
  await client.send("Emulation.setDeviceMetricsOverride", {
    width: viewport.width,
    height: viewport.height,
    deviceScaleFactor: 1,
    mobile: false,
  });
  await client.send("Page.navigate", { url: fixtureUrl("scene-matrix") });
  await waitForApp(client);
  await clickVisibleByText(client, ".workspaceTabs button", "Control");
  await sleep(64);
  const failed = [];

  await evaluatePageFunction(client, () => {
    document.querySelector('[data-scene-matrix-cue-id="303"] .sceneMatrixTrigger')?.click();
  });
  await sleep(48);
  const authored = await readSceneLiveModifierState(client, 303);
  if (authored.stripCueId !== "303") failed.push(`strip=${authored.stripCueId}`);
  if (authored.override !== "false") failed.push(`authored-override=${authored.override}`);
  if (authored.readouts.join("|") !== "x2|50%|25%") {
    failed.push(`authored-readouts=${authored.readouts.join("|")}`);
  }
  if (authored.direction !== "Authored" || authored.segment !== "0") {
    failed.push(`authored-playback=${authored.direction}/${authored.segment}`);
  }
  if (authored.resetDisabled !== true) failed.push("reset-enabled-at-authored");
  if (authored.matrixFlashCells < 1) failed.push("no-matrix-flash-cell");

  await setSceneLiveModifierSlider(client, 303, "speed", 3);
  await sleep(48);
  const latched = await readSceneLiveModifierState(client, 303);
  if (latched.override !== "true") failed.push(`latch-override=${latched.override}`);
  if (latched.readouts[0] !== "x3") failed.push(`latch-speed=${latched.readouts[0]}`);
  if (latched.resetDisabled !== false) failed.push("reset-disabled-while-live");

  await evaluatePageFunction(client, () => {
    document.querySelector('[data-cue-live-modifier-reset="303"]')?.click();
  });
  await sleep(48);
  const reset = await readSceneLiveModifierState(client, 303);
  if (reset.override !== "false" || reset.readouts[0] !== "x2") {
    failed.push(`reset=${reset.override}/${reset.readouts[0]}`);
  }

  await setSceneLiveModifierSelect(client, 303, "direction", "Reverse");
  await setSceneLiveModifierSelect(client, 303, "segment", 2);
  await sleep(48);
  const playback = await readSceneLiveModifierState(client, 303);
  if (
    playback.override !== "true" ||
    playback.direction !== "Reverse" ||
    playback.segment !== "2"
  ) {
    failed.push(`playback=${playback.override}/${playback.direction}/${playback.segment}`);
  }
  await evaluatePageFunction(client, () => {
    document.querySelector('[data-cue-live-modifier-reset="303"]')?.click();
  });
  await sleep(48);

  await setSceneLiveModifierSlider(client, 303, "speed", 3);
  await sleep(32);
  await evaluatePageFunction(client, () => {
    document.querySelector('[data-scene-matrix-cue-id="303"] .sceneMatrixTrigger')?.click();
  });
  await sleep(48);
  const retriggered = await readSceneLiveModifierState(client, 303);
  if (retriggered.override !== "false") failed.push(`retrigger-override=${retriggered.override}`);

  await dispatchFlashPointer(client, '[data-scene-flash-cue="320"]', true);
  await sleep(48);
  const flashDown = await readSceneLiveModifierState(client, 303);
  if (!flashDown.activeCardIds.includes("320")) failed.push("flash-not-active-on-down");
  await dispatchFlashPointer(client, '[data-scene-flash-cue="320"]', false);
  await sleep(48);
  const flashUp = await readSceneLiveModifierState(client, 303);
  if (flashUp.activeCardIds.includes("320")) failed.push("flash-still-active-on-up");
  if (!flashUp.containmentZero) failed.push("matrix-containment");

  await clickVisibleByText(client, ".workspaceTabs button", "Touch");
  await sleep(96);
  // The matrix flash release cleared the active cue; bring the latched-strip
  // scene back through the Touch pad so both surfaces prove the same path.
  await evaluatePageFunction(client, () => {
    const pad = document.querySelector('[data-touch-cue-pad="303"]');
    pad?.scrollIntoView({ block: "nearest" });
    pad?.click();
  });
  await sleep(64);
  const touch = await readSceneLiveModifierState(client, 303);
  if (touch.stripCueId !== "303") failed.push(`touch-strip=${touch.stripCueId}`);
  if (touch.touchFlashPads < 1) failed.push("no-touch-flash-pad");
  await dispatchFlashPointer(client, '[data-touch-flash-cue="320"]', true);
  await sleep(48);
  const touchDown = await readSceneLiveModifierState(client, 303);
  if (!touchDown.touchActivePadIds.includes("320")) failed.push("touch-flash-not-active-on-down");
  await dispatchFlashPointer(client, '[data-touch-flash-cue="320"]', false);
  await sleep(48);
  const touchUp = await readSceneLiveModifierState(client, 303);
  if (touchUp.touchActivePadIds.includes("320")) failed.push("touch-flash-still-active-on-up");
  if (!touchUp.containmentZero) failed.push("touch-containment");

  return {
    label: `scene-live-modifier-${viewport.width}x${viewport.height}`,
    passed: failed.length === 0,
    failedChecks: failed,
    authored,
    latched,
    playback,
    retriggered,
    flash: { down: flashDown.activeCardIds, up: flashUp.activeCardIds },
    touchFlash: { down: touchDown.touchActivePadIds, up: touchUp.touchActivePadIds },
  };
}

async function runFxVisualViewport(client, viewport) {
  await client.send("Emulation.setDeviceMetricsOverride", {
    width: viewport.width,
    height: viewport.height,
    deviceScaleFactor: 1,
    mobile: false,
  });
  await client.send("Page.navigate", { url: fixtureUrl("fx-visual") });
  await waitForApp(client);
  await waitForClientCondition(
    client,
    "document.querySelectorAll('.effectFamilyChooser button').length === 9 && document.querySelectorAll('.effectListRows .effectGraphicalPreview').length === 8",
    "T19 FX visualization fixture",
  );
  await sleep(120);
  const initial = await readFxVisualSurface(client);

  const recipeFamilies = [];
  for (const [family, expectedType, expectedCards] of fxVisualRecipeFamilies) {
    await clickVisibleByText(client, ".effectFamilyChooser button", family);
    await sleep(64);
    recipeFamilies.push(await evaluatePageFunction(client, (name, type, cards) => {
      const isVisible = (element) => {
        if (!element) return false;
        const rectangle = element.getBoundingClientRect();
        const style = getComputedStyle(element);
        return rectangle.width > 0 && rectangle.height > 0 && style.display !== "none" && style.visibility !== "hidden";
      };
      const typeSelect = [...document.querySelectorAll(".effectEditor select")]
        .find((candidate) => [...candidate.options].some((option) => option.value === "Mapping"));
      const active = document.querySelector('.effectFamilyChooser button.active[aria-pressed="true"]');
      return {
        family: name,
        expectedType: type,
        expectedCards: cards,
        activeFamily: active?.getAttribute("data-effect-family") ?? "",
        effectType: typeSelect?.value ?? "",
        cardCount: [...document.querySelectorAll(".sampleEffectPresetCard")].filter((card) => {
          const rectangle = card.getBoundingClientRect();
          const style = getComputedStyle(card);
          return rectangle.width > 0 && rectangle.height > 0 && style.display !== "none" && style.visibility !== "hidden";
        }).length,
        visibleCurveEditorCount: [...document.querySelectorAll(".curveEffectEditor")].filter(isVisible).length,
        visibleCurvePointRowCount: [...document.querySelectorAll(".curveEffectPointRow")].filter(isVisible).length,
        visibleCurveTangentInputCount: [...document.querySelectorAll('.curveEffectPointRow input[aria-label^="In"], .curveEffectPointRow input[aria-label^="Out"]')].filter(isVisible).length,
        curveEditorHorizontalOverflowPx: (() => {
          const editor = document.querySelector(".curveEffectEditor");
          return editor ? Math.max(0, editor.scrollWidth - editor.clientWidth) : 0;
        })(),
        visibleMappingEditorCount: [...document.querySelectorAll(".mappingEffectEditor")].filter(isVisible).length,
        visibleMappingOrderRowCount: [...document.querySelectorAll(".mappingEffectOrderList li")].filter(isVisible).length,
        visibleMappingRepetitionInputCount: [...document.querySelectorAll('.mappingEffectEditor input[aria-label^="Mapping repetitions"]')].filter(isVisible).length,
        mappingEditorHorizontalOverflowPx: (() => {
          const editor = document.querySelector(".mappingEffectEditor");
          return editor ? Math.max(0, editor.scrollWidth - editor.clientWidth) : 0;
        })(),
        visibleColorMappingEditorCount: [...document.querySelectorAll(".colorMappingEffectEditor")].filter(isVisible).length,
        visibleColorMappingPreviewCount: [...document.querySelectorAll(".colorMappingPreview")].filter(isVisible).length,
        visibleColorMappingControlCount: [...document.querySelectorAll(".colorMappingControlGrid input, .colorMappingControlGrid select")].filter(isVisible).length,
        colorMappingEditorHorizontalOverflowPx: (() => {
          const editor = document.querySelector(".colorMappingEffectEditor");
          return editor ? Math.max(0, editor.scrollWidth - editor.clientWidth) : 0;
        })(),
      };
    }, family, expectedType, expectedCards));
  }

  await clickVisibleByText(client, ".effectFamilyChooser button", "STEPS");
  await sleep(160);
  const cueEditOpened = await evaluatePageFunction(client, () => {
    const toggle = document.querySelector(".cuePanelEditToggle");
    if (toggle instanceof HTMLButtonElement && toggle.getAttribute("aria-expanded") !== "true") toggle.click();
    return Boolean(toggle);
  });
  await sleep(96);
  const cueRecallOpened = await evaluatePageFunction(client, () => {
    const details = document.querySelector(".cueItem .cueEffectRecallEditor");
    if (!(details instanceof HTMLDetailsElement)) return false;
    details.open = true;
    details.dispatchEvent(new Event("toggle"));
    return true;
  });
  await sleep(96);
  const stepsNavigation = await evaluatePageFunction(client, () => ({
    cueDrawerVisible: (() => {
      const drawer = document.querySelector('[data-timeline-context-drawer-panel="cue"]');
      const rectangle = drawer?.getBoundingClientRect();
      return Boolean(rectangle && rectangle.width > 0 && rectangle.height > 0);
    })(),
    cueLabelVisible: [...document.querySelectorAll(".cueItem")].some((item) =>
      (item.textContent || "").includes("T16 Owned FX Cue")),
    stepEditorCount: document.querySelectorAll('[data-cue-step-editor][data-cue-step-count="3"]').length,
    ownedParamsChipCount: document.querySelectorAll(".cueItem .cueEffectParamsChip").length,
    ownedPreviewKinds: [...document.querySelectorAll(".cueItem .cueEffectRecallRow .effectGraphicalPreview")]
      .map((preview) => preview.getAttribute("data-preview-kind")),
    ownedTransitionControls: [...document.querySelectorAll(".cueItem [data-cue-effect-transition]")]
      .map((input) => {
        const rectangle = input.getBoundingClientRect();
        const row = input.closest(".cueEffectRecallRow")?.getBoundingClientRect();
        return {
          effectId: input.getAttribute("data-cue-effect-transition") ?? "",
          value: input.value,
          disabled: input.disabled,
          visible: rectangle.width > 0 && rectangle.height > 0,
          contained: Boolean(row && rectangle.left >= row.left - 1 && rectangle.right <= row.right + 1),
        };
      }),
    ownedLfo: (() => {
      const preview = [...document.querySelectorAll(".cueItem .cueEffectRecallRow .effectGraphicalPreview")]
        .find((candidate) => candidate.getAttribute("data-preview-kind") === "Lfo");
      const curve = preview?.querySelector(".effectGraphicalCurve");
      return {
        label: preview?.getAttribute("data-effect-label") ?? "",
        low: curve?.getAttribute("data-lfo-low") ?? "",
        high: curve?.getAttribute("data-lfo-high") ?? "",
        phase: curve?.getAttribute("data-lfo-phase") ?? "",
      };
    })(),
  }));
  stepsNavigation.cueEditOpened = cueEditOpened;
  stepsNavigation.cueRecallOpened = cueRecallOpened;

  await client.send("Page.navigate", { url: fixtureUrl("fx-visual") });
  await waitForApp(client);
  await waitForClientCondition(client, "document.querySelectorAll('.effectFamilyChooser button').length === 9", "T16 family chooser reload");
  await clickVisibleByText(client, ".effectFamilyChooser button", "SUPER SCENE");
  await sleep(180);
  const superSceneNavigation = await evaluatePageFunction(client, () => ({
    breadcrumbVisible: (() => {
      const breadcrumb = document.querySelector("[data-timeline-breadcrumb]");
      const rectangle = breadcrumb?.getBoundingClientRect();
      return Boolean(rectangle && rectangle.width > 0 && rectangle.height > 0);
    })(),
    childLabel: (document.querySelector("[data-child-timeline-label]")?.textContent || "").trim(),
    childLayerCount: document.querySelectorAll(".timelineUserLaneGutter").length,
    showExitButton: (document.querySelector("[data-timeline-breadcrumb] button")?.textContent || "").trim(),
  }));

  await client.send("Page.navigate", { url: fixtureUrl("fx-visual") });
  await waitForApp(client);
  await waitForClientCondition(client, "document.querySelectorAll('.effectListRows .effectGraphicalPreview').length === 8", "T19 rack reload");
  await clickVisibleByText(client, ".effectItemSummary", "T16 Diamond Move");
  await waitForClientCondition(client, "Boolean(document.querySelector('.moveEffectPathCanvas .moveEffectPointHandle'))", "T16 Move editor");
  await evaluatePageFunction(client, async () => {
    const canvas = document.querySelector(".moveEffectPathCanvas");
    canvas?.scrollIntoView({ block: "center", inline: "nearest" });
    window.scrollTo(0, 0);
    document.documentElement.scrollTo(0, 0);
    document.body.scrollTo(0, 0);
    document.querySelector(".app")?.scrollTo(0, 0);
    await new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame)));
    return true;
  });
  await sleep(96);
  const subThreshold = await exerciseMoveFxDrag(client, 3, 0);
  const escapeRollback = await exerciseMoveFxDrag(client, 6, 0, true);
  const committed = await exerciseMoveFxDrag(client, 6, 0);
  const clampStart = await readMoveFxPointState(client);
  const clamped = clampStart.canvas
    ? await exerciseMoveFxDrag(client, clampStart.canvas.width, -clampStart.canvas.height)
    : { before: clampStart, during: null, after: null };

  const close = (left, right, tolerance = 0.000_1) =>
    typeof left === "number" && typeof right === "number" && Math.abs(left - right) <= tolerance;
  const previewByKind = Object.fromEntries(initial.previews.map((preview) => [preview.kind, preview]));
  const conditions = [
    ["familyExactOrder", () => JSON.stringify(initial.familyNames) === JSON.stringify(fxVisualFamilyOrder)],
    ["familyExactLabels", () => JSON.stringify(initial.familyLabels) === JSON.stringify(fxVisualFamilyOrder)],
    ["familyAllVisibleAndContained", () => initial.familyButtons.length === 9 && initial.familyButtons.every((button) => button.visible && button.contained && button.width >= 40 && button.height >= 40)],
    ["familyOneActive", () => JSON.stringify(initial.activeFamilies) === JSON.stringify(["VALUE FX"])],
    ["familyNoOverflow", () => initial.chooserHorizontalOverflowPx <= 1 && initial.chooserVerticalOverflowPx <= 1],
    ["sevenRecipeFamiliesReachable", () => recipeFamilies.length === 7 && recipeFamilies.every((entry) => entry.activeFamily === entry.family && entry.effectType === entry.expectedType && entry.cardCount === entry.expectedCards)],
    ["independentCurveEditorReachable", () => {
      const curve = recipeFamilies.find((entry) => entry.family === "CURVE FX");
      return curve?.effectType === "Curve" && curve.visibleCurveEditorCount === 1 && curve.visibleCurvePointRowCount === 2 && curve.visibleCurveTangentInputCount === 4 && curve.curveEditorHorizontalOverflowPx <= 1;
    }],
    ["independentMappingEditorReachable", () => {
      const mapping = recipeFamilies.find((entry) => entry.family === "MAPPINGS");
      return mapping?.effectType === "Mapping" && mapping.visibleMappingEditorCount === 1 && mapping.visibleMappingOrderRowCount >= 1 && mapping.visibleMappingRepetitionInputCount === 2 && mapping.mappingEditorHorizontalOverflowPx <= 1;
    }],
    ["independentColorMappingEditorReachable", () => {
      const mapping = recipeFamilies.find((entry) => entry.family === "COLOR MAPPINGS");
      return mapping?.effectType === "ColorMapping" && mapping.visibleColorMappingEditorCount === 1 && mapping.visibleColorMappingPreviewCount === 1 && mapping.visibleColorMappingControlCount === 10 && mapping.colorMappingEditorHorizontalOverflowPx <= 1;
    }],
    ["rackEightActualPreviews", () => JSON.stringify(initial.previewKinds) === JSON.stringify(["Lfo", "Color", "Move", "Value", "Curve", "Mapping", "ColorMapping", "Chaser"])],
    ["rackPreviewDimensions", () => initial.previews.every((preview) => preview.width >= 72 && preview.height >= 34 && preview.containedInItem && preview.horizontalOverflowPx <= 1)],
    ["rackNoHorizontalOverflow", () => initial.rackHorizontalOverflowPx <= 1 && initial.effectListHorizontalOverflowPx <= 1],
    ["rackVerticalScrollReachesLastPreview", () => initial.effectListVerticalOverflowPx <= 1 || (["auto", "scroll"].includes(initial.effectListOverflowY) && initial.lastPreviewReachable)],
    ["lfoPreviewUsesAuthoredValues", () => previewByKind.Lfo?.shape === "Sine" && previewByKind.Lfo?.low === "4096" && previewByKind.Lfo?.high === "57344" && previewByKind.Lfo?.phase === "0.125" && previewByKind.Lfo?.curvePath.length > 20],
    ["colorPreviewUsesAuthoredPalette", () => previewByKind.Color?.stopCount === "4" && previewByKind.Color?.stopPositions === "0,0.18,0.54,1" && previewByKind.Color?.stopColors === "65535:2048:0,65535:41000:0,0:50000:65535,25000:0:65535" && previewByKind.Color?.colorInterpolation === "HsvShortest" && previewByKind.Color?.gradientBackground.includes("gradient")],
    ["movePreviewUsesAuthoredPath", () => previewByKind.Move?.movePointCount === "4" && previewByKind.Move?.movePoints === "0.5:0.08,0.92:0.5,0.5:0.92,0.08:0.5" && previewByKind.Move?.moveCenter === "0.46:0.56" && previewByKind.Move?.moveSize === "0.78:0.62" && previewByKind.Move?.moveRotation === "18" && previewByKind.Move?.moveCoordinateMode === "Absolute" && previewByKind.Move?.movePreviewBase === "absolute" && previewByKind.Move?.moveControlPath.length > 20 && previewByKind.Move?.moveOutputPath.length > 20 && previewByKind.Move?.moveControlPath !== previewByKind.Move?.moveOutputPath && previewByKind.Move?.moveOutputInViewBox],
    ["valuePreviewUsesAuthoredEnvelope", () => previewByKind.Value?.valuePoints === "0:0.12,0.22:0.88,0.58:0.42,1:0.76" && previewByKind.Value?.valueInterpolation === "Smooth" && previewByKind.Value?.curvePath.length > 20],
    ["curvePreviewUsesAuthoredTangents", () => previewByKind.Curve?.curvePoints === "0:0.08:0:2.4,0.42:0.92:0.2:-0.8,1:0.22:-1.6:0" && previewByKind.Curve?.curvePath.length > 20],
    ["mappingPreviewUsesAuthoredOrder", () => previewByKind.Mapping?.shape === "Triangle" && previewByKind.Mapping?.low === "2048" && previewByKind.Mapping?.high === "63000" && previewByKind.Mapping?.phase === "0.2" && previewByKind.Mapping?.mappingDirection === "Bounce" && previewByKind.Mapping?.mappingRepetitions === "1.5" && previewByKind.Mapping?.mappingFixtureOrder === "3,1,2" && previewByKind.Mapping?.curvePath.length > 20],
    ["colorMappingPreviewUsesAuthoredMedia", () => previewByKind.ColorMapping?.colorMappingRaster === "4x2" && previewByKind.ColorMapping?.colorMappingFrameCount === "2" && previewByKind.ColorMapping?.colorMappingCellCount === "3" && previewByKind.ColorMapping?.colorMappingSourceKind === "Video" && previewByKind.ColorMapping?.colorMappingDirection === "Bounce" && previewByKind.ColorMapping?.colorMappingWrapMode === "Repeat" && previewByKind.ColorMapping?.colorMappingSampling === "Bilinear" && previewByKind.ColorMapping?.colorMappingPixelCount === 8],
    ["chaserPreviewUsesAuthoredSteps", () => previewByKind.Chaser?.chaserStepCount === "4" && previewByKind.Chaser?.chaserActiveStepCount === "2"],
    ["stepsNavigation", () => stepsNavigation.cueEditOpened && stepsNavigation.cueRecallOpened && stepsNavigation.cueDrawerVisible && stepsNavigation.cueLabelVisible && stepsNavigation.stepEditorCount === 1],
    ["cueOwnedParamsRendered", () => stepsNavigation.ownedParamsChipCount === 8 && JSON.stringify(stepsNavigation.ownedPreviewKinds) === JSON.stringify(["Lfo", "Color", "Move", "Value", "Curve", "Mapping", "ColorMapping", "Chaser"]) && stepsNavigation.ownedLfo.label === "Cue-owned Sine Curve" && stepsNavigation.ownedLfo.low === "8192" && stepsNavigation.ownedLfo.high === "61440" && stepsNavigation.ownedLfo.phase === "0.25"],
    ["cueOwnedTransitionsRendered", () => stepsNavigation.ownedTransitionControls.length === 8 && stepsNavigation.ownedTransitionControls.every((control) => control.visible && control.contained && !control.disabled) && stepsNavigation.ownedTransitionControls.filter((control) => control.value === "750").length === 1],
    ["superSceneNavigation", () => superSceneNavigation.breadcrumbVisible && superSceneNavigation.childLabel === "T16 Owned FX Cue" && superSceneNavigation.childLayerCount === 3 && superSceneNavigation.showExitButton === "Show"],
    ["moveThreePxIsClick", () => close(subThreshold.before?.x, subThreshold.during?.x) && close(subThreshold.before?.y, subThreshold.during?.y) && subThreshold.during?.ghostCount === 0 && subThreshold.during?.readoutCount === 0 && close(subThreshold.before?.x, subThreshold.after?.x) && close(subThreshold.before?.y, subThreshold.after?.y)],
    ["moveSixPxShowsGhostAndReadout", () => escapeRollback.during?.ghostCount === 1 && escapeRollback.during?.readoutCount === 1 && /X\s+[0-9.]+\s+·\s+Y\s+[0-9.]+/.test(escapeRollback.during?.readout ?? "") && !close(escapeRollback.before?.x, escapeRollback.during?.x)],
    ["moveEscapeRollsBack", () => close(escapeRollback.before?.x, escapeRollback.after?.x) && close(escapeRollback.before?.y, escapeRollback.after?.y) && escapeRollback.after?.ghostCount === 0 && escapeRollback.after?.readoutCount === 0],
    ["moveSixPxCommits", () => committed.during?.ghostCount === 1 && committed.during?.readoutCount === 1 && !close(committed.before?.x, committed.after?.x) && close(committed.during?.x, committed.after?.x) && committed.after?.ghostCount === 0 && committed.after?.readoutCount === 0],
    ["moveDragUsesRenderedSvgCoordinates", () => {
      const scale = committed.before?.canvasScale?.x;
      const actualDelta = (committed.after?.x ?? Number.NaN) - (committed.before?.x ?? Number.NaN);
      const expectedDelta = typeof scale === "number" && scale > 0 ? committed.deltaX / (scale * 100) : Number.NaN;
      return Number.isFinite(expectedDelta) && close(actualDelta, expectedDelta, 0.000_15);
    }],
    ["moveEditorOutputUsesRuntimeBounds", () => [subThreshold, escapeRollback, committed, clamped].every((result) => result.before?.outputPathInViewBox && result.during?.outputPathInViewBox && result.after?.outputPathInViewBox)],
    ["moveClampCommitsUnitBounds", () => clamped.during?.ghostCount === 1 && close(clamped.after?.x, 1) && close(clamped.after?.y, 1) && clamped.after?.documentAndAppScrollZero === true],
    ["documentAndAppScrollZero", () => initial.documentAndAppScrollZero && subThreshold.after?.documentAndAppScrollZero && escapeRollback.after?.documentAndAppScrollZero && committed.after?.documentAndAppScrollZero && clamped.after?.documentAndAppScrollZero],
  ];
  const checks = Object.fromEntries(conditions.map(([name, check]) => {
    try {
      return [name, Boolean(check())];
    } catch {
      return [name, false];
    }
  }));
  const failedChecks = Object.entries(checks).filter(([, passed]) => !passed).map(([name]) => name);
  return {
    label: `fx-visual-${viewport.width}x${viewport.height}`,
    passed: failedChecks.length === 0,
    checks,
    failedChecks,
    initial,
    recipeFamilies,
    stepsNavigation,
    superSceneNavigation,
    move: { subThreshold, escapeRollback, committed, clamped },
  };
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
  const extraProfileDirs = [];

  try {
    if (shouldStartVite) {
      await failIfPortOccupied(vitePort, "Syndocal dev server");
      viteProcess = startProcess(
        process.execPath,
        ["node_modules/vite/bin/vite.js", "--configLoader", "runner", "--host", "127.0.0.1", "--port", String(vitePort), "--strictPort"],
        { cwd: appRoot },
      );
    }
    await failIfPortOccupied(cdpPort, "Chrome DevTools Protocol");
    await waitForHttp(appUrl, "Syndocal dev server");

    const browserArgs = [
      "--headless=new",
      "--no-sandbox",
      "--disable-gpu",
      "--disable-background-timer-throttling",
      "--disable-backgrounding-occluded-windows",
      "--disable-renderer-backgrounding",
      "--hide-scrollbars",
      "--disable-crash-reporter",
      "--disable-crashpad",
      `--remote-debugging-port=${cdpPort}`,
      "--remote-debugging-address=127.0.0.1",
      `--user-data-dir=${profileDir}`,
      `--window-size=${primaryOperationalViewport.width},${primaryOperationalViewport.height}`,
      "about:blank",
    ];
    browserProcess = startProcess(browser, browserArgs);
    await waitForHttp(`http://127.0.0.1:${cdpPort}/json/version`, "Chrome DevTools Protocol");

    client = await createCdpClient();
    // Long-lived headless sessions freeze their renderer main thread after
    // dozens of heavy fixture navigations (zero-CPU block, browser-side CDP
    // fine, Runtime.evaluate dead - reproduced 3x on this machine, isolated
    // fresh-browser runs of the same scenarios pass 6/6). Recycling the
    // browser between phases isolates each phase from that failure class.
    const recycleBrowser = async () => {
      try {
        client?.close();
      } catch {
        // Ignore close races on a possibly-wedged client.
      }
      await stopProcess(browserProcess);
      // A force-killed Chrome leaves singleton locks in its profile; reusing
      // the directory makes the next instance recover state unpredictably.
      // Each recycled browser gets a fresh profile instead.
      const recycledProfileDir = mkdtempSync(join(tmpdir(), "syndocal-cdp-"));
      extraProfileDirs.push(recycledProfileDir);
      browserProcess = startProcess(
        browser,
        browserArgs.map((arg) => (arg.startsWith("--user-data-dir=") ? `--user-data-dir=${recycledProfileDir}` : arg)),
      );
      await waitForHttp(`http://127.0.0.1:${cdpPort}/json/version`, "Chrome DevTools Protocol");
      client = await createCdpClient();
      await waitForApp(client);
      traceViewport("browser recycled for next phase");
    };
    console.log(
      `viewport contract primary-browser=${primaryOperationalViewport.width}x${primaryOperationalViewport.height} measured-client-size-browser=${measuredClientSizeViewport.width}x${measuredClientSizeViewport.height} extended-browser=${extendedCeilingViewport.width}x${extendedCeilingViewport.height} fallback-browsers=${compactFallbackViewports.map((viewport) => `${viewport.width}x${viewport.height}`).join(",")} screenshots=${captureAllViewportScreenshots ? "all" : "large-browser-fixtures"}`,
    );
    if (groupStrobeOnlyMode) {
      const groupStrobeResults = [];
      for (const viewport of viewports) {
        const result = await runGroupStrobeViewport(client, viewport);
        groupStrobeResults.push(result);
        console.log(
          `${result.passed ? "pass" : "fail"} ${result.label} ` +
            `coverage=${result.initial.compatibleCount} ` +
            `desktop=${result.initial.strobeValue}->${result.latched.strobeValue}->${result.cleared.strobeValue} ` +
            `solo=${result.latched.soloPressed}->${result.cleared.soloPressed} ` +
            `touch=${result.touchLatched.touchStrobeValue}->${result.touchCleared.touchStrobeValue} ` +
            `failed=${JSON.stringify(result.failedChecks)}`,
        );
      }
      const failures = groupStrobeResults.filter((result) => !result.passed);
      if (failures.length > 0) {
        throw new Error(`Group strobe viewport failed: ${JSON.stringify(failures)}`);
      }
      return;
    }
    if (sceneLiveModifierOnlyMode) {
      const liveModifierResults = [];
      for (const viewport of viewports) {
        const result = await runSceneLiveModifierViewport(client, viewport);
        liveModifierResults.push(result);
        console.log(
          `${result.passed ? "pass" : "fail"} ${result.label} ` +
            `authored=${result.authored.readouts.join("/")} latch=${result.latched.readouts[0]} ` +
            `retrigger=${result.retriggered.override} ` +
            `flash=${result.flash.down.join("+") || "none"}->${result.flash.up.join("+") || "none"} ` +
            `touchFlash=${result.touchFlash.down.join("+") || "none"}->${result.touchFlash.up.join("+") || "none"} ` +
            `failed=${JSON.stringify(result.failedChecks)}`,
        );
      }
      const failures = liveModifierResults.filter((result) => !result.passed);
      if (failures.length > 0) {
        throw new Error(`Scene live modifier viewport failed: ${JSON.stringify(failures)}`);
      }
      return;
    }
    if (fxVisualOnlyMode) {
      const fxVisualResults = [];
      for (const viewport of viewports) {
        const result = await runFxVisualViewport(client, viewport);
        fxVisualResults.push(result);
        console.log(
          `${result.passed ? "pass" : "fail"} ${result.label} ` +
            `families=${result.initial.familyNames.join("/")} active=${result.initial.activeFamilies.join("+") || "none"} ` +
            `recipes=${result.recipeFamilies.map((entry) => `${entry.family}:${entry.cardCount}:${entry.effectType}`).join(",")} ` +
            `colourMap=${(() => { const entry = result.recipeFamilies.find((candidate) => candidate.family === "COLOR MAPPINGS"); return entry ? `${entry.visibleColorMappingEditorCount}/${entry.visibleColorMappingPreviewCount}/${entry.visibleColorMappingControlCount}/overflow:${entry.colorMappingEditorHorizontalOverflowPx}` : "missing"; })()} ` +
            `panes=${Math.round(result.initial.workbenchRect.width)}:` +
              `${Math.round(result.initial.libraryPaneRect.width)}/${Math.round(result.initial.inspectorPaneRect.width)}/${Math.round(result.initial.rackPaneRect.width)} ` +
            `previews=${result.initial.previews.map((entry) => `${entry.kind}:${Math.round(entry.width)}x${Math.round(entry.height)}`).join(",")} ` +
            `owned=${result.stepsNavigation.ownedParamsChipCount}/${result.stepsNavigation.ownedPreviewKinds.join("+")}/fade:${result.stepsNavigation.ownedTransitionControls.filter((control) => control.value !== "").map((control) => control.value).join("+") || "snap"} ` +
            `nav=${result.stepsNavigation.stepEditorCount}/${result.superSceneNavigation.childLabel || "none"} ` +
            `move=3:${result.move.subThreshold.before?.x}->${result.move.subThreshold.after?.x} ` +
              `esc:${result.move.escapeRollback.before?.x}->${result.move.escapeRollback.during?.x}->${result.move.escapeRollback.after?.x} ` +
              `commit:${result.move.committed.before?.x}->${result.move.committed.after?.x} ` +
              `clamp:${result.move.clamped.after?.x},${result.move.clamped.after?.y} ` +
            `scroll=${result.initial.documentAndAppScrollZero ? "zero" : "overflow"} ` +
            `failed=${JSON.stringify(result.failedChecks)}`,
        );
      }
      const failures = fxVisualResults.filter((result) => !result.passed);
      if (failures.length > 0) {
        throw new Error(`FX visual viewport failed: ${JSON.stringify(failures.map((result) => ({
          label: result.label,
          failedChecks: result.failedChecks,
          panes: result.initial ? {
            workbench: result.initial.workbenchRect,
            library: result.initial.libraryPaneRect,
            inspector: result.initial.inspectorPaneRect,
            rack: result.initial.rackPaneRect,
          } : null,
          previews: result.initial?.previews.map((preview) => ({
            kind: preview.kind,
            size: [preview.width, preview.height],
            overflow: preview.horizontalOverflowPx,
          })) ?? [],
          rackScroll: result.initial?.effectListScrollMetrics ?? null,
          colorMappingEditor: result.recipeFamilies?.find((entry) => entry.family === "COLOR MAPPINGS") ?? null,
        })))}`);
      }
      return;
    }
    if (patchOnlyMode) {
      const patchResults = [];
      for (const viewport of viewports) {
        const result = await runPatchViewport(client, viewport);
        patchResults.push(result);
        const metrics = result.containment.patchGridMetrics;
        console.log(
          `${result.passed ? "pass" : "fail"} ${result.label} ` +
            `cells=${metrics?.addressCount ?? 0} range=${metrics?.firstAddress ?? "?"}..${metrics?.lastAddress ?? "?"} ` +
            `pages=${metrics?.pageSelectorCount ?? "?"} geometry=${metrics?.columnCount ?? "?"}x${metrics?.rowCount ?? "?"} ` +
            `cell=${metrics?.minCellWidth ?? "?"}x${metrics?.minCellHeight ?? "?"} ` +
            `scroll=${metrics?.clientWidth ?? "?"}x${metrics?.clientHeight ?? "?"}->${metrics?.scrollWidth ?? "?"}x${metrics?.scrollHeight ?? "?"} ` +
            `end=${metrics?.endReachable ? 1 : 0} outer=${metrics?.outerScrollUnchangedAtEnd ? 0 : 1} ` +
            `keys=${metrics?.arrowRightAddress ?? "?"}/${metrics?.arrowDownAddress ?? "?"}/${metrics?.controlEndAddress ?? "?"} ` +
            `keyVisible=${metrics?.controlEndFullyVisible ? 1 : 0} ` +
            `high=${result.highAddressAction.addressValue ?? "?"}:${result.highAddressAction.plannedHighAddresses?.join(",") ?? "?"} ` +
            `failed=${JSON.stringify(result.failedChecks)}`,
        );
      }
      const failures = patchResults.filter((result) => !result.passed);
      if (failures.length > 0) {
        throw new Error(`Continuous PATCH viewport failed: ${JSON.stringify(failures)}`);
      }
      return;
    }
    if (workspaceSplitOnlyMode) {
      const workspaceSplitResults = [];
      for (const viewport of viewports) {
        const result = await runWorkspaceSplitViewport(client, viewport);
        workspaceSplitResults.push(result);
        console.log(
          `${result.passed ? "pass" : "fail"} ${result.label} ` +
            `initial=${result.initial.ratios.join("/")}:${result.initial.panes.map((value) => Math.round(value)).join("/")} ` +
            `persisted=${result.persisted.before.join("/")}->${result.persisted.after.join("/")} ` +
            `drawer=${Math.round(result.drawerOpenMetrics.drawerHeight)}/${Math.round(result.drawerOpenMetrics.expectedDrawerHeight)}:${result.drawerOpenMetrics.scrollerClientHeight}->${result.drawerOpenMetrics.scrollerScrollHeight}:${result.drawerOpenMetrics.lastInteractiveReachable ? 1 : 0}/${result.drawerOpenMetrics.lastButtonReachable ? 1 : 0}/${result.drawerOpenMetrics.lastInputReachable ? 1 : 0} ` +
            `timelineFocus=${result.timelinePaneExpansion.passed ? "pass" : "fail"} ` +
            `paneWindow=${result.paneWindow.passed ? "pass" : "fail"} ` +
            `failed=${JSON.stringify(result.failedChecks)} ` +
            `timelineFailed=${JSON.stringify(result.timelinePaneExpansion.failedChecks)} ` +
            `paneFailed=${JSON.stringify(result.paneWindow.failedChecks)}`,
        );
      }
      const failures = workspaceSplitResults.filter((result) => !result.passed);
      if (failures.length > 0) {
        throw new Error(`Workspace split viewport failed: ${JSON.stringify(failures)}`);
      }
      return;
    }
    if (persistentBandOnlyMode) {
      const persistentBandResults = [];
      for (const viewport of viewports) {
        const result = await runPersistentBandInvarianceViewport(client, viewport);
        persistentBandResults.push(result);
        console.log(
          `${result.passed ? "pass" : "fail"} ${result.containment.label} ` +
            `rects=${JSON.stringify(result.containment.persistentBandRectsByWorkspace)} ` +
            `deltas=${JSON.stringify(result.containment.persistentBandRectDeltas)} ` +
            `timelineExpansion=${result.containment.timelinePaneExpansion?.passed ? "pass" : "fail"} ` +
            `failedChecks=${JSON.stringify(result.containment.timelinePaneExpansion?.failedChecks ?? [])} ` +
            `layeredDesk=${result.containment.layeredTimelineDesk?.passed ? "pass" : "fail"} ` +
            `layeredDeskFailed=${JSON.stringify(result.containment.layeredTimelineDesk?.failedChecks ?? [])}`,
        );
      }
      const failures = persistentBandResults.filter((result) => !result.passed);
      if (failures.length > 0) {
        throw new Error(`Persistent band invariance failed: ${JSON.stringify(failures)}`);
      }
      return;
    }
    if (timelineSlimOnlyMode) {
      const timelineSlimResults = [];
      for (const viewport of viewports) {
        const result = await runTimelineSlimViewport(client, viewport);
        timelineSlimResults.push(result);
        console.log(
          `${result.passed ? "pass" : "fail"} ${result.label} ` +
            `tabs=${JSON.stringify(result.visual.deskTabLabels)} ` +
            `tools=${result.visual.toolbarIconOnlyButtonCount}/${result.visual.toolbarAriaButtonCount}/${result.visual.toolbarButtonCount} ` +
            `gutters=${result.visual.gutterRows.filter((row) => row.exactFourElements).length}/${result.visual.gutterCount} ` +
            `implicit=${result.implicitVisual.gutterRows.filter((row) => row.exactFourElements).length}/${result.implicitVisual.gutterCount} ` +
            `blocks=${result.visual.thickBlockCount}/${result.visual.identityFillCount}/${result.visual.opaqueBlackBorderCount}/${result.visual.blockCount} ` +
            `lines=${result.visual.twoLineBlockCount}/${result.visual.twoLineEligibleCount}@${Math.round(result.visual.minTwoLineFontPx * 100) / 100}px ` +
            `grid=${result.visual.maxMinorGridInk.toFixed(3)}/${result.visual.maxMajorGridInk.toFixed(3)}/${result.visual.maxDividerInk.toFixed(3)} ` +
            `t14=${result.t14.subThresholdClick?.movedPx ?? "?"}/${result.t14.oneGestureDrag?.sourceCueId ?? "?"}@${result.t14.oneGestureDrag?.targetLayerId ?? "?"} ` +
            `t14Failed=${JSON.stringify(result.t14.failedChecks)} ` +
            `scroll=${result.visual.documentAndAppScrollZero ? "zero" : "overflow"} ` +
            `failed=${JSON.stringify(result.failedChecks)}`,
        );
      }
      const failures = timelineSlimResults.filter((result) => !result.passed);
      if (failures.length > 0) {
        throw new Error(`Timeline slim viewport failed: ${JSON.stringify(failures)}`);
      }
      return;
    }
    if (sceneMatrixOnlyMode) {
      const sceneMatrixResults = [];
      for (const viewport of viewports) {
        const result = await runSceneMatrixPaneCheck(client, viewport);
        sceneMatrixResults.push(result);
        console.log(
          `${result.passed ? "pass" : "fail"} ${result.label} ` +
            `columns=${JSON.stringify(result.before.columns)} ` +
            `cell=${Math.round(result.before.minCellHitSize * 100) / 100}px ` +
            `handle=${Math.round(result.before.minDragHandleHitSize * 100) / 100}px ` +
            `width=${Math.round(result.before.paneWidthCoverage * 1000) / 1000} ` +
            `scroll=${result.before.documentAndAppScrollZero ? "zero" : "overflow"} ` +
            `click=302 active=${result.subThresholdClick?.before.activeCardIds.join("+") || "?"}->` +
              `${result.subThresholdClick?.after.activeCardIds.join("+") || "?"} ` +
            `drag=${result.oneGestureDrag?.sourceCueId ?? "?"}@${result.oneGestureDrag?.targetLayerId ?? "?"} ` +
              `events=${result.oneGestureDrag?.before.markerCount ?? "?"}->${result.oneGestureDrag?.after.markerCount ?? "?"} ` +
              `placements=${result.oneGestureDrag?.before.cuePlacementCount ?? "?"}->${result.oneGestureDrag?.after.cuePlacementCount ?? "?"} ` +
            `failed=${JSON.stringify(result.failedChecks)}`,
        );
      }
      const failures = sceneMatrixResults.filter((result) => !result.passed);
      if (failures.length > 0) {
        throw new Error(`Scene Matrix viewport failed: ${JSON.stringify(failures)}`);
      }
      return;
    }
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
    if (operatorVjOnlyMode) {
      const operatorVjResults = [];
      for (const viewport of viewports) {
        for (const locale of ["en", "ja"]) {
          const result = await runOperatorVjAcceptanceViewport(client, viewport, locale);
          operatorVjResults.push(result);
          console.log(
            `${result.passed ? "pass" : "fail"} VJ operator acceptance ${locale} ${viewport.width}x${viewport.height} ` +
              JSON.stringify({
                failedChecks: result.failedChecks,
                layers: [result.initial.layerIds, result.terminalBank.layerIds],
                advanced: [result.initial.advancedDomCount, result.expanded.layerOne?.controlCount, result.terminalBank.advancedDomCount],
                advancedViewport: result.layoutPhases[1]?.advancedViewport,
                bypass: {
                  pressed: result.bypassed.layerOne?.bypassPressed,
                  snapshotReads: result.bypassed.mock?.snapshotReadCount,
                  snapshotEnabled: result.bypassed.mock?.lastLayerOneEnabled,
                },
                output: {
                  initial: result.initial.selectedOutputId,
                  selected: result.outputSelected.selectedOutputId,
                  detail: result.outputSelected.visibleOutputDetails,
                  program: result.outputSelected.programLabel,
                },
                layout: result.layoutPhases.map((phase) => ({
                  overflow: phase.unsafeOverflowCount,
                  outside: phase.outsideRectCount,
                  outsideNames: phase.rectContainment
                    .filter((entry) => !entry.contained)
                    .map((entry) => entry.name),
                })),
              }),
          );
        }
      }
      const failures = operatorVjResults.filter((result) => !result.passed);
      if (failures.length > 0) {
        throw new Error(`VJ operator viewport acceptance failed: ${JSON.stringify(failures)}`);
      }
      return;
    }
    if (audioReactiveOnlyMode) {
      const audioReactiveResults = [];
      for (const viewport of viewports) {
        const result = await runAudioReactiveAcceptanceViewport(client, viewport);
        audioReactiveResults.push(result);
        console.log(
          `${result.passed ? "pass" : "fail"} Audio Reactive Rack ${viewport.width}x${viewport.height} ` +
            JSON.stringify({ mixer: result.mixerContract, editor: result.editorContract }),
        );
      }
      const failures = audioReactiveResults.filter((result) => !result.passed);
      if (failures.length > 0) {
        throw new Error(`Audio Reactive viewport acceptance failed: ${JSON.stringify(failures)}`);
      }
      return;
    }
    if (autoVjOnlyMode) {
      const autoVjResults = [];
      for (const viewport of viewports) {
        for (const locale of ["en", "ja"]) {
          const result = await runAutoVjAcceptanceViewport(client, viewport, locale);
          autoVjResults.push(result);
          console.log(
            `${result.passed ? "pass" : "fail"} Auto VJ stateful acceptance ${locale} ${viewport.width}x${viewport.height} ` +
              JSON.stringify({
                failedChecks: result.failedChecks,
                initial: result.initial,
                running: result.running,
                audioCalls: result.calls.filter((call) =>
                  call.command === "play_video_layer_audio_monitor" ||
                  call.command === "stop_video_layer_audio_monitor"
                ),
              }),
          );
        }
      }
      const failures = autoVjResults.filter((result) => !result.passed);
      if (failures.length > 0) {
        throw new Error(`Auto VJ stateful viewport acceptance failed: ${JSON.stringify(failures)}`);
      }
      return;
    }
    if (fullscreenVjOnlyMode) {
      const fullscreenVjResults = [];
      for (const viewport of viewports) {
        const result = await runFullscreenVjAcceptanceViewport(client, viewport);
        fullscreenVjResults.push(result);
        console.log(
          `${result.passed ? "pass" : "fail"} fullscreen VJ focus ${viewport.width}x${viewport.height} ` +
            JSON.stringify({
              failedChecks: result.failedChecks,
              columns: [
                result.live.videoMixerClipPaneWidth,
                result.live.videoMixerProgramPaneWidth,
                result.live.videoMixerLayerPaneWidth,
              ],
              monitorWidths: [result.live.videoMonitorPreviewWidth, result.live.videoMonitorProgramWidth],
              monitorRatio: result.monitorRatio,
              outerHeader: [
                result.stopped.visibleVideoMixerOuterHeaderCount,
                result.stopped.videoMixerOuterHeaderHeight,
                result.stopped.videoMixerBodyTopGap,
              ],
              audioDock: {
                height: result.live.liveAudioRailHeight,
                primaryControl: result.stopped.liveAudioRailPrimaryControlMinHeight,
                configControl: result.stopped.liveAudioRailConfigControlMinHeight,
                labels: result.stopped.visibleLiveAudioConfigLabelCount,
              },
              telemetry: {
                badges: result.live.liveAudioRailTelemetryBadgeCount,
                truncated: result.live.liveAudioRailTelemetryTruncatedCount,
                criticalOverflow: result.live.liveAudioRailFullscreenCriticalOverflowCount,
                outside: result.live.liveAudioRailTelemetryOutsideCount,
                text: result.live.liveAudioRailTelemetryText,
              },
            }),
        );
      }
      const failures = fullscreenVjResults.filter((result) => !result.passed);
      if (failures.length > 0) {
        throw new Error(`Fullscreen VJ focus acceptance failed: ${JSON.stringify(failures)}`);
      }
      return;
    }
    if (liveAudioOnlyMode) {
      const liveAudioResults = [];
      for (const viewport of viewports) {
        for (const locale of ["en", "ja"]) {
          const result = await runLiveAudioAcceptanceViewport(client, viewport, locale);
          const passed =
            result.passed &&
            isContained(result.liveContainment) &&
            result.liveContainment.liveAudioRailTelemetryBadgeCount === 6 &&
            result.liveContainment.liveAudioRailCriticalTelemetryOverflowCount === 0 &&
            result.liveContainment.liveAudioRailTelemetryOutsideCount === 0;
          liveAudioResults.push({ viewport, locale, passed, result });
          console.log(
            `${passed ? "pass" : "fail"} live audio stateful acceptance ${locale} ${viewport.width}x${viewport.height} ` +
              JSON.stringify({
                failedChecks: result.failedChecks,
                startRequest: result.calls.find((call) => call.command === "start_live_audio_input")?.args?.request,
                liveMeters: result.live.meters,
                clearing: {
                  health: result.clearing.health,
                  action: result.clearing.actionText,
                  announcement: result.clearing.announcement,
                },
                telemetryLayout: {
                  badges: result.liveContainment.liveAudioRailTelemetryBadgeCount,
                  criticalOverflow: result.liveContainment.liveAudioRailCriticalTelemetryOverflowCount,
                  outside: result.liveContainment.liveAudioRailTelemetryOutsideCount,
                  railOverflow: [
                    result.liveContainment.liveAudioRailOverflowX,
                    result.liveContainment.liveAudioRailOverflowY,
                  ],
                },
              }),
          );
        }
      }
      const failures = liveAudioResults.filter((entry) => !entry.passed);
      if (failures.length > 0) {
        throw new Error(`Live audio stateful viewport acceptance failed: ${JSON.stringify(failures)}`);
      }
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
      results.push(await runComposedTouchViewport(client, viewport));
    }
    const cueRecallResults = [];
    const cueRecallLargeResults = [];
    const effectStackLargeResults = [];
    const sceneBlockLargeResults = [];
    const cueNodeGraphResults = [];
    const sceneMatrixResults = [];
    const vjBankResults = [];
    const paneWindowResults = [];
    if (!workspaceShellOnlyMode) {
      for (const viewport of viewports) {
        await recycleBrowser();
        cueRecallResults.push(await runCueRecallViewport(client, viewport));
        cueRecallLargeResults.push(await runCueRecallLargeViewport(client, viewport));
        effectStackLargeResults.push(await runEffectStackLargeViewport(client, viewport));
        cueNodeGraphResults.push(await runCueNodeGraphViewport(client, viewport));
        sceneMatrixResults.push(await runSceneMatrixPaneCheck(client, viewport));
        vjBankResults.push(await runVjBankViewport(client, viewport));
        paneWindowResults.push(await runPaneWindowViewport(client, viewport));
      }
    }
    const sceneBlockScaleViewports = viewports.filter((viewport) =>
      isPrimaryOperationalViewport(viewport) ||
      isMeasuredClientSizeViewport(viewport) ||
      matchesViewport(viewport, compactFallbackViewports[0]),
    );
    if (!workspaceShellOnlyMode) {
      for (const viewport of sceneBlockScaleViewports.length > 0 ? sceneBlockScaleViewports : viewports.slice(0, 1)) {
        await recycleBrowser();
        sceneBlockLargeResults.push(await runSceneBlockLargeViewport(client, viewport));
      }
    }

    const failures = results.filter((result) => !isContained(result));
    const cueRecallFailures = cueRecallResults.filter((result) => !result.passed);
    const cueRecallLargeFailures = cueRecallLargeResults.filter((result) => !result.passed);
    const effectStackLargeFailures = effectStackLargeResults.filter((result) => !result.passed);
    const sceneBlockLargeFailures = sceneBlockLargeResults.filter((result) => !result.passed);
    const cueNodeGraphFailures = cueNodeGraphResults.filter((result) => !result.passed);
    const sceneMatrixFailures = sceneMatrixResults.filter((result) => !result.passed);
    const vjBankFailures = vjBankResults.filter((result) => !result.passed);
    const paneWindowFailures = paneWindowResults.filter((result) => !result.passed);
    const setupSurfaceFailures = results.filter((result) => !hasExpectedSetupSurface(result));
    const persistentBandFailures = results.filter((result) => !hasExpectedPersistentWorkspaceBand(result));
    const persistentBandInvarianceFailures = results.filter((result) => !hasExpectedPersistentBandInvariance(result));
    const timelinePaneExpansionFailures = results.filter((result) => !hasExpectedTimelinePaneExpansion(result));
    const layeredTimelineDeskFailures = results.filter((result) => !hasExpectedLayeredTimelineDesk(result));
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
    const killZoneFailures = shouldCheckTimelineAutomation
      ? results.filter((result) => !hasExpectedKillZone(result))
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
        ? ` touch=${result.visibleTouchSurfaceCount}/${result.visibleTouchModeToggleCount}/${result.visibleTouchPageTabCount}/${result.visibleTouchPlacedControlCount} page=${JSON.stringify(result.touchActivePageLabel)} kinds=${JSON.stringify(result.touchPlacedKinds)} palette=${JSON.stringify(result.touchComposedCheckResult?.paletteLabels ?? [])} live=${result.touchComposedCheckResult?.liveOperationsPassed ? "pass" : "fail"} safety=${result.visibleTouchSafetyDeckCount}/${result.visibleTouchSafetyGuardButtonCount} targets=${result.touchPlacedUndersizedCount}/${result.touchUndersizedTargetCount} scroll=${result.touchDocumentAndAppScrollZero ? 0 : 1}`
        : "";
      const controlStageGlyphSuffix = /^control-live-\d+x\d+$/.test(result.label)
        ? ` controlStageGlyphs=${JSON.stringify(result.controlStageFixtureGlyphMetrics)} hitMin=${result.controlStageFixtureMinSize}`
        : "";
      const mappingSuffix = result.label.startsWith("setup-mapping-")
        ? ` mapping=${result.mappingUseInEffectsButtonCount}/${result.mappingWaveDraftButtonCount}/${result.visibleMappingProjectorButtonCount}/${result.visibleMappingProjectorControlsCount}/${result.visibleMappingProjectorWarpGridCount}/${result.visibleMappingProjectorActionButtonCount}/${result.visibleMappingProjectorResetPoseButtonCount}/${result.visibleStageVideoSurfaceCount} stage=${result.mappingStageHeight} clip=${result.mappingFilterVerticalClipCount}/${result.mappingViewportChildOverlapCount}/${result.mappingSidebarUnsafeOverflowCount}`
        : "";
      const mappingHotkeyHelpSuffix = result.label.startsWith("mapping-hotkey-help-")
        ? ` mappingHelp=${result.visibleMappingHotkeyHelpCount}/${result.mappingHotkeyHelpKeyCount}`
        : "";
      const patchSuffix = result.label.startsWith("setup-patch-")
        ? ` patch=${result.visiblePatchActionRowCount}/${result.visiblePatchAutoButtonCount}/${result.visiblePatchPrimaryButtonCount}/${result.visiblePatchNextFreeButtonCount}/${result.visiblePatchFootprintCount}/${result.visibleDmxAddressGridCount}/${result.dmxAddressCellCount}/${result.dmxAddressOccupiedCellCount}/${result.dmxAddressPlannedCellCount}/${result.visibleDmxFixtureBlockCount}/${result.compactMappingStageWidth}x${result.compactMappingStageHeight}/${result.visibleDmxGridSummaryCount}/${result.visibleFixtureSetupEditorCount}/${result.visibleUseProfileForPatchButtonCount}/${result.visibleDuplicateFixtureButtonCount} continuous=${result.patchGridMetrics?.addressCount ?? 0}:${result.patchGridMetrics?.firstAddress ?? "?"}..${result.patchGridMetrics?.lastAddress ?? "?"} pages=${result.patchGridMetrics?.pageSelectorCount ?? "?"} geometry=${result.patchGridMetrics?.columnCount ?? "?"}x${result.patchGridMetrics?.rowCount ?? "?"} cell=${result.patchGridMetrics?.minCellWidth ?? "?"}x${result.patchGridMetrics?.minCellHeight ?? "?"} scroll=${result.patchGridMetrics?.clientWidth ?? "?"}x${result.patchGridMetrics?.clientHeight ?? "?"}->${result.patchGridMetrics?.scrollWidth ?? "?"}x${result.patchGridMetrics?.scrollHeight ?? "?"} end=${result.patchGridMetrics?.endReachable ? 1 : 0} outer=${result.patchGridMetrics?.outerScrollUnchangedAtEnd ? 0 : 1} keys=${result.patchGridMetrics?.arrowRightAddress ?? "?"}/${result.patchGridMetrics?.arrowDownAddress ?? "?"}/${result.patchGridMetrics?.controlEndAddress ?? "?"}`
        : "";
      const outputSetupSuffix = result.label.startsWith("setup-video-")
        ? ` outputSetup=${result.visibleSetupVideoPanelCount}/${result.visibleSetupVideoOutputDeckCount}/${result.visibleSetupVideoOutputActiveDeckCount}/${result.visibleSetupVideoOutputDetailPaneCount}/${result.visibleVideoOutputMappingPanelCount}/${result.visibleVideoOutputBlendControlsCount}/${result.visibleProjectorMapEditorCount}/${result.visibleProjectorMapHandleCount}/${result.visibleProjectorKeystoneHandleCount}/${result.visibleProjectorScaleHandleCount}/${result.visibleProjectorRotateHandleCount}/${result.visibleProjectorAspectModeButtonCount}/${result.visibleProjectorAspectPresetButtonCount}/${result.visibleProjectorResetPoseButtonCount}/${result.videoSetupSidebarWidth}w/${result.videoSetupOutputDeskWidth}w panes=${result.videoSetupRoutingPaneWidth}/${result.videoSetupMapPaneWidth}/${result.videoSetupInspectorPaneWidth} mapH=${result.videoSetupMapPaneHeight} overflow=${result.videoSetupMapPaneOverflowPx} reachable=${result.videoSetupMappingLastControlReachable ? 1 : 0}/${result.videoSetupPreviewContained ? 1 : 0}/${result.videoSetupActionDockLastActionReachable ? 1 : 0} dock=${result.visibleVideoSetupActionDockCount}/${result.videoSetupActionDockHeight} actions=${result.videoSetupCriticalActionInViewportCount}`
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
        ? ` mixer=${result.visibleVideoOutputItemCount}/${result.visibleVideoOutputSelectedItemCount}/${result.visibleVideoMixerOutputDeckCount}/${result.visibleVideoMixerOutputFaderCount}/${result.visibleVideoMixerOutputSelectButtonCount}/${result.visibleVideoLayerItemCount}/${result.visibleBuiltinVideoFxSelectCount}/${result.visibleVideoMixerLayerDeckCount}/${result.visibleVideoMixerLayerFaderCount}/${result.visibleVideoMixerLayerButtonCount} clip=${result.videoClipGridClientHeight}/${result.fullyVisibleVideoClipPadCount} contract=${result.controlModeFailedChecks?.join(",") || "ok"}`
        : "";
      const persistentBandSuffix = result.label.startsWith("persistent-band-invariance-")
        ? ` persistentBand=${result.persistentBandInvariant ? "stable" : "moved"} rects=${JSON.stringify(result.persistentBandRectsByWorkspace)} deltas=${JSON.stringify(result.persistentBandRectDeltas)} timelineExpansion=${result.timelinePaneExpansion?.passed ? "pass" : "fail"} timelineExpansionFailed=${JSON.stringify(result.timelinePaneExpansion?.failedChecks ?? [])} layeredDesk=${result.layeredTimelineDesk?.passed ? "pass" : "fail"} layeredDeskFailed=${JSON.stringify(result.layeredTimelineDesk?.failedChecks ?? [])}`
        : "";
      console.log(
        `${status} [${viewportRole({ width: result.innerWidth, height: result.innerHeight })}] ${result.label} document=${result.documentScrollWidth}x${result.documentScrollHeight} app=${result.appScrollWidth}x${result.appScrollHeight} moved=${result.movedX},${result.movedY}${keyboardSuffix}${timelineSuffix}${sceneBlockSuffix}${touchSuffix}${controlStageGlyphSuffix}${projectMenuSuffix}${mappingSuffix}${mappingHotkeyHelpSuffix}${patchSuffix}${outputSetupSuffix}${waveDraftSuffix}${editVisualSuffix}${positionVisualSuffix}${colorEffectSuffix}${chaserEffectSuffix}${moveEffectSuffix}${mixerSuffix}${persistentBandSuffix}`,
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
    for (const result of sceneMatrixResults) {
      console.log(
        `${result.passed ? "pass" : "fail"} ${result.label} columns=${JSON.stringify(result.before.columns)} active=${JSON.stringify(result.before.activeCardIds)}->${JSON.stringify(result.after.activeCardIds)} width=${Math.round(result.before.paneWidthCoverage * 1000) / 1000} handle=${Math.round(result.before.minDragHandleHitSize * 100) / 100}px click=302:${result.subThresholdClick?.before.activeCardIds.join("+") || "?"}->${result.subThresholdClick?.after.activeCardIds.join("+") || "?"} drag=${result.oneGestureDrag?.sourceCueId ?? "?"}@${result.oneGestureDrag?.targetLayerId ?? "?"} events=${result.oneGestureDrag?.before.markerCount ?? "?"}->${result.oneGestureDrag?.after.markerCount ?? "?"} placements=${result.oneGestureDrag?.before.cuePlacementCount ?? "?"}->${result.oneGestureDrag?.after.cuePlacementCount ?? "?"} failed=${JSON.stringify(result.failedChecks)}`,
      );
    }
    for (const result of paneWindowResults) {
      console.log(
        `${result.passed ? "pass" : "fail"} ${result.label} stageWin=${result.stageWin.join("/")} timelineWin=${result.timelineWin.join("/")} poppedMain=${result.poppedMain.join("/")} failed=${JSON.stringify(result.failedChecks)}`,
      );
    }
    for (const result of vjBankResults) {
      console.log(
        `${result.passed ? "pass" : "fail"} ${result.label} pads=${result.pads.join("/")} drawers=${result.drawers.join("/")} monitorRatio=${result.monitorRatio} layerRows=${result.layerRows} kill=${result.killButtons} failed=${JSON.stringify(result.failedChecks)}`,
      );
    }
    if (
      failures.length > 0 ||
      cueRecallFailures.length > 0 ||
      cueRecallLargeFailures.length > 0 ||
      effectStackLargeFailures.length > 0 ||
      sceneBlockLargeFailures.length > 0 ||
      cueNodeGraphFailures.length > 0 ||
      sceneMatrixFailures.length > 0 ||
      vjBankFailures.length > 0 ||
      paneWindowFailures.length > 0 ||
      keyboardNavigationFailures.length > 0 ||
      setupSurfaceFailures.length > 0 ||
      persistentBandFailures.length > 0 ||
      persistentBandInvarianceFailures.length > 0 ||
      timelinePaneExpansionFailures.length > 0 ||
      layeredTimelineDeskFailures.length > 0 ||
      projectMenuFailures.length > 0 ||
      localizationFailures.length > 0 ||
      mappingHotkeyHelpFailures.length > 0 ||
      mappingWaveDraftFailures.length > 0 ||
      controlModeFailures.length > 0 ||
      touchSurfaceFailures.length > 0 ||
      timelineAutomationFailures.length > 0 ||
      sceneBlockFailures.length > 0 ||
      killZoneFailures.length > 0 ||
      statusLineFailures.length > 0
    ) {
      if (workspaceShellOnlyMode) {
        console.error(JSON.stringify({
          viewport: failures.map((result) => result.label),
          setupSurface: setupSurfaceFailures.map((result) => result.label),
          persistentBand: persistentBandFailures.map((result) => result.label),
          persistentBandInvariance: persistentBandInvarianceFailures.map((result) => result.label),
          timelinePaneExpansion: timelinePaneExpansionFailures.map((result) => ({
            label: result.timelinePaneExpansion?.label ?? result.label,
            failedChecks: result.timelinePaneExpansion?.failedChecks ?? [],
          })),
          layeredTimelineDesk: layeredTimelineDeskFailures.map((result) => ({
            label: result.layeredTimelineDesk?.label ?? result.label,
            failedChecks: result.layeredTimelineDesk?.failedChecks ?? [],
          })),
          localization: localizationFailures.map((result) => ({
            label: result.label,
            language: result.documentLanguage,
            projectMenu: result.visibleProjectMenuCount,
            japaneseLanguage: result.visibleJapaneseLanguageLabelCount,
            japaneseSave: result.visibleJapaneseSaveButtonCount,
            preservedFixtures: result.preservedUserFixtureLabelCount,
            translatedFixtures: result.translatedUserFixtureCollisionCount,
          })),
          controlMode: controlModeFailures.map((result) => ({
            label: result.label,
            liveHeight: result.liveControlPanelHeight,
            upperPaneHeight: result.workspacePaneRects?.upper?.height ?? 0,
            position: [result.positionConsoleWidth, result.positionConsoleHeight, result.positionPadWidth, result.positionPadHeight],
            move: [result.moveEffectEditorWidth, result.moveEffectEditorHeight, result.moveEffectLastControlReachable, result.moveEffectLastPointReachable],
            timeline: [result.visibleTimelinePanelCount, result.visibleTimelineShowSurfaceCount, result.visibleTimelineDeskTabCount],
            cue: [
              result.visibleCueContextDrawerCount,
              result.visibleCuePanelCount,
              result.visibleCueLivePanelCount,
              result.visibleCueFormCount,
              result.visibleCueEffectRecallEditorCount,
              result.visibleCueEditOnlyCount,
            ],
            cueToggle: [
              result.visibleCueEditToggleCount,
              result.cueEditToggleExpanded,
              result.cueEditToggleControls,
              result.cueEditToggleMinTargetSize,
            ],
            cueScroll: [
              result.cuePanelOverflowY,
              result.cuePanelHorizontalOverflowPx,
              result.cuePanelVerticalOverflowPx,
              result.cueContextDrawerBodyOverflowY,
              result.cueContextDrawerBodyHorizontalOverflowPx,
              result.cueContextDrawerBodyVerticalOverflowPx,
              result.cueContextDrawerContained,
              result.cueContextDrawerBodyContained,
              result.cueContextDrawerLastControlReachable,
              result.cueHostHorizontalOverflowPx,
              result.cueHostVerticalOverflowPx,
            ],
            cueScope: [
              result.cueCaptureScopeValue,
              result.cuePreviewScopeLabel,
              result.cuePreviewStatValues,
              result.cueStoreButtonDisabled,
              result.visibleCueScopeErrorCount,
            ],
          })),
          timelineAutomation: timelineAutomationFailures.map((result) => result.label),
          sceneBlocks: sceneBlockFailures.map((result) => ({
            label: result.label,
            workspace: [result.sceneBlockWorkspaceWidth, result.sceneBlockWorkspaceHeight],
            overflow: [result.sceneBlockWorkspaceHorizontalOverflowPx, result.sceneBlockWorkspaceVerticalOverflowPx, result.sceneBlockListHorizontalOverflowPx, result.sceneBlockListVerticalOverflowPx, result.sceneBlockComposerHorizontalOverflowPx],
            reachability: [result.sceneBlockLastComposerControlReachable, result.sceneBlockLastRowActionReachable],
          })),
          killZone: killZoneFailures.map((result) => ({
            label: result.label,
            killButtonCount: result.killButtonCount,
            killClearCount: result.killClearCount,
            killButtonBorderIsRed: result.killButtonBorderIsRed,
            liveStatusMinFontPx: result.liveStatusMinFontPx,
            liveCueIdentityChipCount: result.liveCueIdentityChipCount,
          })),
        }, null, 2));
      } else console.error(
        JSON.stringify(
          {
            viewport: failures,
            cueRecall: cueRecallFailures,
            cueRecallLarge: cueRecallLargeFailures,
            effectStackLarge: effectStackLargeFailures,
            sceneBlockLarge: sceneBlockLargeFailures,
            cueNodeGraph: cueNodeGraphFailures,
            sceneMatrix: sceneMatrixFailures,
            vjBank: vjBankFailures,
            paneWindow: paneWindowFailures,
            keyboardNavigation: keyboardNavigationFailures,
            setupSurface: setupSurfaceFailures,
            persistentBand: persistentBandFailures,
            persistentBandInvariance: persistentBandInvarianceFailures,
            timelinePaneExpansion: timelinePaneExpansionFailures.map((result) => result.timelinePaneExpansion),
            layeredTimelineDesk: layeredTimelineDeskFailures.map((result) => result.layeredTimelineDesk),
            projectMenu: projectMenuFailures,
            localization: localizationFailures,
            mappingHotkeyHelp: mappingHotkeyHelpFailures,
            mappingWaveDraft: mappingWaveDraftFailures,
            controlMode: controlModeFailures,
            touchSurface: touchSurfaceFailures,
            timelineAutomation: timelineAutomationFailures,
            sceneBlocks: sceneBlockFailures,
            killZone: killZoneFailures.map((result) => ({
              label: result.label,
              killButtonCount: result.killButtonCount,
              killClearCount: result.killClearCount,
              killButtonBorderIsRed: result.killButtonBorderIsRed,
              liveStatusMinFontPx: result.liveStatusMinFontPx,
              liveCueIdentityChipCount: result.liveCueIdentityChipCount,
            })),
            statusLine: statusLineFailures,
          },
          null,
          2,
        ),
      );
      throw new Error(
        `${failures.length} viewport containment check(s), ${cueRecallFailures.length} Cue Recall fixture check(s), ${cueRecallLargeFailures.length} large Cue Recall DOM-budget check(s), ${effectStackLargeFailures.length} large live-effect DOM-budget check(s), ${sceneBlockLargeFailures.length} large Scene Block DOM-budget/layout check(s), ${cueNodeGraphFailures.length} Cue Node Graph fixture check(s), ${sceneMatrixFailures.length} Scene Matrix fixture check(s), ${keyboardNavigationFailures.length} keyboard navigation check(s), ${setupSurfaceFailures.length} setup surface check(s), ${persistentBandFailures.length} persistent band check(s), ${persistentBandInvarianceFailures.length} persistent band invariance check(s), ${timelinePaneExpansionFailures.length} timeline pane expansion check(s), ${layeredTimelineDeskFailures.length} layered timeline desk check(s), ${projectMenuFailures.length} project menu check(s), ${localizationFailures.length} localization check(s), ${mappingHotkeyHelpFailures.length} mapping hotkey help check(s), ${mappingWaveDraftFailures.length} mapping wave draft check(s), ${controlModeFailures.length} control mode surface check(s), ${touchSurfaceFailures.length} touch surface check(s), ${timelineAutomationFailures.length} timeline automation visual check(s), ${sceneBlockFailures.length} Scene Block context-pane check(s), ${killZoneFailures.length} kill zone check(s), ${statusLineFailures.length} status line check(s) failed.`,
      );
    }
  } finally {
    client?.close();
    await stopProcess(browserProcess);
    await stopProcess(viteProcess);
    for (const dir of [profileDir, ...extraProfileDirs]) {
      try {
        rmSync(dir, { recursive: true, force: true });
      } catch (error) {
        console.warn(`Viewport check passed, but cleanup could not remove ${dir}: ${error.message}`);
      }
    }
  }
}

// Second net behind the CDP close rejection: if every child process dies the
// event loop can drain with main() still unsettled, and Node would exit 0
// without having run a single assertion. Fail closed instead.
let mainSettled = false;
process.on("beforeExit", () => {
  if (!mainSettled) {
    console.error("viewport harness: event loop drained before checks completed - failing closed.");
    process.exitCode = 1;
  }
});
main()
  .catch((error) => {
    console.error(error.message);
    process.exitCode = 1;
  })
  .finally(() => {
    mainSettled = true;
  });
