// Strict Timeline authoritative-route browser gate.
//
// PROOF BOUNDARY: this gate proves only the renderer's browser-side IPC
// contract on the timeline-layered fixture: exact command identity, exact
// payload shapes, trusted-input-gated owner registration rearm, fail-closed
// rejection of unknown commands, and zero mutation requests from invalid
// targets. The bridge declines every validated mutation because it owns no
// backend projection; nothing here evidences native Tauri, transaction
// coordinator, engine, persistence, or physical-output completion.
//
// Allowed operations (exact): register_project_transaction_owner,
// set_program_audio_handoff_config, and the two Timeline operations
// split_timeline_items / move_timeline_items_to_lanes which the production
// renderer emits as apply_timeline_advanced_authoritative requests with
// request.kind "split_items" / "move_items_to_lanes". No fixture callback is
// installed and there is no catch-all success path.
import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { createServer } from "node:net";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const appRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const host = "127.0.0.1";
// 2026-08-26: fixed ports 5196 (HTTP) / 9246 (CDP) let a stale headless
// listener from an earlier run answer for this run's children, producing false
// passes or leaked processes. Ports are now allocated free per run and every
// listener is proven gate-owned before use; the legacy ports are additionally
// swept for listeners this gate verifiably owns via its unique profile marker,
// so prior-run leaks are terminated while unrelated processes are preserved.
const legacyGatePorts = [5196, 9246];
const profileMarker = "syndocal-timeline-authority-";
let vitePort;
let cdpPort;
let baseUrl;
const screenshotDir = process.env.SYNDOCAL_TIMELINE_AUTHORITY_SCREENSHOT_DIR || process.env.SYNDOCAL_CONTROL_SCREENSHOT_DIR
  ? resolve(process.env.SYNDOCAL_TIMELINE_AUTHORITY_SCREENSHOT_DIR || process.env.SYNDOCAL_CONTROL_SCREENSHOT_DIR)
  : null;
const viewport = { width: 1280, height: 720 };
const browsers = [
  process.env.CHROME_PATH,
  "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe",
  "C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe",
  "C:\\Program Files\\Microsoft\\Edge\\Application\\msedge.exe",
].filter(Boolean);
const sleep = (ms) => new Promise((resolveSleep) => setTimeout(resolveSleep, ms));
// Raised for definite gate-owned failures (early child exit, foreign port
// listener); waitFor must surface these immediately instead of burning the
// full timeout on a condition that can never become true.
class FatalGateSignal extends Error {}
const waitFor = async (check, label, timeoutMs = 30_000) => {
  const deadline = Date.now() + timeoutMs;
  let lastError;
  while (Date.now() < deadline) {
    try {
      const result = await check();
      if (result) return result;
    } catch (error) {
      if (error instanceof FatalGateSignal) throw error;
      lastError = error;
    }
    await sleep(100);
  }
  throw new Error(`Timed out waiting for ${label}${lastError ? `: ${lastError}` : ""}`);
};
const runCaptured = (file, args, timeoutMs = 15_000) => new Promise((resolveRun, rejectRun) => {
  const child = spawn(file, args, { stdio: ["ignore", "pipe", "pipe"], windowsHide: true });
  let stdout = "";
  let stderr = "";
  let settled = false;
  const finish = (settle) => {
    if (settled) return;
    settled = true;
    clearTimeout(timer);
    settle();
  };
  const timer = setTimeout(() => {
    child.kill();
    finish(() => rejectRun(new Error(`${file} timed out after ${timeoutMs}ms`)));
  }, timeoutMs);
  child.stdout.setEncoding("utf8");
  child.stderr.setEncoding("utf8");
  child.stdout.on("data", (chunk) => { stdout += chunk; });
  child.stderr.on("data", (chunk) => { stderr += chunk; });
  child.once("error", (error) => finish(() => rejectRun(error)));
  child.once("exit", (code) => finish(() => (
    code === 0 ? resolveRun(stdout) : rejectRun(new Error(`${file} exited ${code}: ${stderr.trim()}`))
  )));
});
const getListenOwners = async (port) => {
  // Availability of Get-NetTCPConnection is verified explicitly and fails
  // closed; a silent query failure must never be read as "port is free".
  const script = `$ErrorActionPreference='Stop';`
    + `if (-not (Get-Command Get-NetTCPConnection -ErrorAction SilentlyContinue)) { `
    + `[Console]::Error.WriteLine('Get-NetTCPConnection unavailable'); exit 87 }; `
    + `(Get-NetTCPConnection -LocalPort ${port} -State Listen -ErrorAction SilentlyContinue | `
    + `Select-Object -ExpandProperty OwningProcess -Unique) -join ','`;
  const stdout = (await runCaptured("powershell.exe", ["-NoProfile", "-NonInteractive", "-Command", script])).trim();
  return stdout === "" ? [] : stdout.split(",").map(Number).filter((pid) => Number.isSafeInteger(pid) && pid > 0);
};
const getProcessCommandLine = async (pid) => {
  try {
    return (await runCaptured("powershell.exe", [
      "-NoProfile",
      "-NonInteractive",
      "-Command",
      `(Get-CimInstance Win32_Process -Filter "ProcessId=${pid}" -ErrorAction Stop).CommandLine`,
    ])).trim();
  } catch {
    return null;
  }
};
const forceKillTree = (pid, reason) => {
  console.error(`Timeline authority gate: force-terminating owned process tree PID ${pid} (${reason})`);
  return new Promise((resolveKill) => {
    const killer = spawn("taskkill.exe", ["/PID", String(pid), "/T", "/F"], { stdio: "ignore", windowsHide: true });
    killer.once("error", () => resolveKill());
    killer.once("exit", () => resolveKill());
  });
};
const stopChild = async (child, label) => {
  if (!child || typeof child.pid !== "number" || child.exitCode !== null) return;
  const exited = new Promise((resolveExit) => child.once("exit", resolveExit));
  try { child.kill(); } catch { /* already exiting */ }
  await Promise.race([exited, sleep(2_000)]);
  if (child.exitCode !== null) return;
  await forceKillTree(child.pid, `${label} ignored graceful termination`);
  await Promise.race([exited, sleep(2_000)]);
};
const allocateFreePort = async (label) => new Promise((resolvePort, rejectPort) => {
  const server = createServer();
  server.once("error", (error) => rejectPort(new Error(`${label}: free-port allocation failed: ${error.message}`)));
  server.listen(0, host, () => {
    const address = server.address();
    const port = address && typeof address === "object" ? address.port : -1;
    server.close(() => (Number.isInteger(port) && port > 0
      ? resolvePort(port)
      : rejectPort(new Error(`${label}: OS refused to allocate a usable listen port`))));
  });
});
const assertListenPortFree = async (port, label) => {
  try {
    await new Promise((resolveProbe, rejectProbe) => {
      const probe = createServer();
      probe.once("error", (error) => rejectProbe(Object.assign(new Error(
        `${label}: freshly allocated port 127.0.0.1:${port} is already occupied`,
      ), { cause: error })));
      probe.listen(port, host, () => probe.close(() => resolveProbe()));
    });
  } catch (error) {
    const owners = await getListenOwners(port).catch(() => []);
    throw Object.assign(new Error(`${error.message}; owning PID(s): ${owners.join(",") || "unknown"} -- refusing to start against a stale or foreign listener`), { cause: error });
  }
};
const assertListenerOwnership = async (port, ownedPid, marker, label) => {
  const owners = await getListenOwners(port);
  if (owners.length === 0) {
    throw new FatalGateSignal(`${label}: no listener is bound on 127.0.0.1:${port}`);
  }
  for (const pid of owners) {
    if (typeof ownedPid === "number" && pid === ownedPid) continue;
    const commandLine = marker === null ? null : await getProcessCommandLine(pid);
    if (marker === null || commandLine === null || !commandLine.includes(marker)) {
      throw new FatalGateSignal(
        `${label}: listener PID ${pid} on 127.0.0.1:${port} is not owned by this gate `
        + `(expected gate PID ${ownedPid ?? "none"}${marker ? ` or a command line containing "${marker}"` : ""}); `
        + `refusing to continue against a foreign or stale listener.`,
      );
    }
  }
};
// Chrome --headless=new relaunches itself: the spawned chrome.exe exits while a
// re-parented browser owns the CDP port (see check-viewport-containment.mjs
// 2026-08-04 note). Ownership therefore cannot be proven by child.pid alone;
// every CDP-listener command line must carry this run's unique profile dir.
const terminateVerifiedGateListeners = async () => {
  if (process.platform !== "win32") return;
  for (const [port, child] of [[vitePort, vite], [cdpPort, browser]]) {
    if (typeof port !== "number") continue;
    for (const pid of await getListenOwners(port).catch(() => [])) {
      if (typeof child?.pid === "number" && pid === child.pid) {
        await forceKillTree(pid, `still listening on 127.0.0.1:${port} after graceful stop`);
        continue;
      }
      const commandLine = await getProcessCommandLine(pid);
      if (typeof profileDir === "string" && commandLine !== null && commandLine.includes(profileDir)) {
        await forceKillTree(pid, `verified gate-owned listener on 127.0.0.1:${port}`);
      } else {
        console.error(`Timeline authority gate: leaving unverified listener PID ${pid} on 127.0.0.1:${port} untouched`);
      }
    }
  }
};
const sweepOwnedLegacyListeners = async () => {
  if (process.platform !== "win32") return;
  for (const port of legacyGatePorts) {
    for (const pid of await getListenOwners(port).catch(() => [])) {
      const commandLine = await getProcessCommandLine(pid);
      if (commandLine !== null && commandLine.includes(profileMarker)) {
        await forceKillTree(pid, `legacy gate-owned listener from a previous run on reserved port 127.0.0.1:${port}`);
      } else {
        console.error(`Timeline authority gate: preserving unverified listener PID ${pid} on reserved port 127.0.0.1:${port}`);
      }
    }
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
      this.socket.onclose = () => reject(new Error("CDP socket closed"));
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
const statusText = (client) => evaluate(client, "document.querySelector('.appStatusText')?.textContent?.trim() ?? ''");
const bridgeState = (client) => evaluate(client, `(() => {
  const bridge = window.__syndocalStrictAuthorityBridge;
  if (!bridge) return null;
  return {
    calls: structuredClone(bridge.calls),
    rejected: structuredClone(bridge.rejected),
    faults: structuredClone(bridge.faults),
  };
})()`);
const contextMenuAction = async (client, selector, clientX, clientY) => evaluate(client, `(() => {
  const target = document.querySelector(${JSON.stringify(selector)});
  if (!(target instanceof Element)) return false;
  target.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true, clientX: ${clientX}, clientY: ${clientY} }));
  return true;
})()`);
const clickSplitAction = (client) => evaluate(client, `(() => {
  const button = document.querySelector('[data-timeline-split-action]');
  if (!(button instanceof HTMLButtonElement) || button.disabled) return false;
  button.click();
  return true;
})()`);

const altClickSelect = (client, selector) => evaluate(client, `(() => {
  const element = document.querySelector(${JSON.stringify(selector)});
  if (!(element instanceof Element)) return false;
  element.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true, altKey: true }));
  return true;
})()`);
const dragTimelineItemToLane = async (client, selector, targetLayerId) => {
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
    const sourceYs = item.matches('.timelineMarker.sceneBlock') ? [moveBandY, centerY] : [centerY, moveBandY];
    const excluded = '.timelineAutomationHandle, .timelineAutomationKeyframeGroup, .timelineSceneBlockResizeHandle, .timelineSceneBlockFadeHandle, .timelineVideoClipResizeHandle, .timelineVideoClipFadeHandle, .timelineAudioClipResizeHandle, .timelineAudioClipFadeHandle';
    const sourceXs = [
      itemRect.left + itemRect.width * 0.5,
      itemRect.left + itemRect.width * 0.25,
      itemRect.left + itemRect.width * 0.75,
      itemRect.left + Math.min(safeInset, Math.max(1, itemRect.width / 2)),
    ];
    const sourcePoint = sourceYs.flatMap((candidateY) => sourceXs.map((candidateX) => ({ x: candidateX, y: candidateY }))).find((candidate) => {
      const hit = document.elementFromPoint(candidate.x, candidate.y);
      return hit && item.contains(hit) && !hit.closest(excluded);
    });
    const resolvedSource = sourcePoint ?? { x: itemRect.left + itemRect.width / 2, y: itemRect.top + itemRect.height / 2 };
    return { source: resolvedSource, sourceSize: [itemRect.width, itemRect.height] };
  })()`);
  assert.ok(points && points.sourceSize[0] > 0 && points.sourceSize[1] > 0, `visible lane drag source geometry for ${selector}`);
  await client.send("Input.dispatchMouseEvent", { type: "mouseMoved", ...points.source });
  await client.send("Input.dispatchMouseEvent", { type: "mousePressed", ...points.source, button: "left", buttons: 1, clickCount: 1 });
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
  assert.ok(target && target.size[0] > 0 && target.size[1] > 0, `visible lane drag target geometry for lane ${targetLayerId}`);
  assert.equal(target.layerAtPoint, String(targetLayerId), `pointer target resolves lane ${targetLayerId} for ${selector}`);
  await client.send("Input.dispatchMouseEvent", { type: "mouseMoved", ...target.point, button: "left", buttons: 1 });
  await client.send("Input.dispatchMouseEvent", { type: "mouseReleased", ...target.point, button: "left", buttons: 0, clickCount: 1 });
  await sleep(100);
};
const hitVerifiedPointerClick = async (client, selector, label, clickCount = 1) => {
  const target = await evaluate(client, `(() => {
    const elements = [...document.querySelectorAll(${JSON.stringify(selector)})]
      .filter((element) => element instanceof Element);
    for (const element of elements) {
      element.scrollIntoView({ block: "center", inline: "nearest" });
      const rect = element.getBoundingClientRect();
      if (!(rect.width > 0 && rect.height > 0)) continue;
      const candidates = [
        { x: rect.left + rect.width / 2, y: rect.top + rect.height / 2 },
        { x: rect.left + Math.min(rect.width - 2, Math.max(2, rect.width * 0.25)), y: rect.top + rect.height / 2 },
        { x: rect.left + rect.width / 2, y: rect.top + Math.min(rect.height - 2, Math.max(2, rect.height * 0.25)) },
      ];
      for (const point of candidates) {
        const hit = document.elementFromPoint(point.x, point.y);
        if (hit && (hit === element || element.contains(hit))) {
          return {
            point,
            hitTag: hit.tagName,
            targetTag: element.tagName,
            visible: true,
          };
        }
      }
    }
    return {
      point: null,
      hitTag: null,
      targetTag: elements[0]?.tagName ?? null,
      visible: elements.length > 0,
      candidates: elements.map((element) => {
        const rect = element.getBoundingClientRect();
        const point = { x: rect.left + rect.width / 2, y: rect.top + rect.height / 2 };
        return {
          tag: element.tagName,
          rect: [rect.left, rect.top, rect.width, rect.height],
          hit: document.elementFromPoint(point.x, point.y)?.tagName ?? null,
          hitClass: document.elementFromPoint(point.x, point.y)?.getAttribute('class') ?? null,
        };
      }),
    };
  })()`);
  assert.ok(target?.visible && target.point, `${label}: a visible, hit-verified pointer target is required (${JSON.stringify(target)})`);
  await client.send("Input.dispatchMouseEvent", { type: "mouseMoved", ...target.point });
  await client.send("Input.dispatchMouseEvent", {
    type: "mousePressed",
    ...target.point,
    button: "left",
    buttons: 1,
    clickCount,
  });
  await client.send("Input.dispatchMouseEvent", {
    type: "mouseReleased",
    ...target.point,
    button: "left",
    buttons: 0,
    clickCount,
  });
  await sleep(50);
  return target;
};
const selectTimelineShelfLightingLaneWithPointerKeyboard = async (client, expectedLayerId) => {
  const target = await evaluate(client, `(() => {
    const label = [...document.querySelectorAll("label.timelineExternalSourceTarget")]
      .find((candidate) => candidate.querySelector("span")?.textContent?.trim() === "Lighting lane");
    const select = label?.querySelector("select");
    if (!(select instanceof HTMLSelectElement)) return null;
    const expectedIndex = [...select.options].findIndex((option) => option.value === ${JSON.stringify(String(expectedLayerId))});
    if (expectedIndex < 0) return { expectedIndex, point: null, initialValue: select.value };
    select.scrollIntoView({ block: "center", inline: "nearest" });
    const rect = select.getBoundingClientRect();
    if (!(rect.width > 0 && rect.height > 0)) return { expectedIndex, point: null, initialValue: select.value };
    const point = { x: rect.left + rect.width / 2, y: rect.top + rect.height / 2 };
    const hit = document.elementFromPoint(point.x, point.y);
    return {
      expectedIndex,
      initialValue: select.value,
      point: hit === select || Boolean(hit && select.contains(hit)) ? point : null,
      hitTag: hit?.tagName ?? null,
    };
  })()`);
  assert.ok(target && target.expectedIndex >= 0 && target.point, "child Timeline exposes Lighting lane 51 as a visible hit-verified select option");
  await client.send("Input.dispatchMouseEvent", { type: "mouseMoved", ...target.point });
  await client.send("Input.dispatchMouseEvent", {
    type: "mousePressed",
    ...target.point,
    button: "left",
    buttons: 1,
    clickCount: 1,
  });
  await client.send("Input.dispatchMouseEvent", {
    type: "mouseReleased",
    ...target.point,
    button: "left",
    buttons: 0,
    clickCount: 1,
  });
  const focused = await evaluate(client, `(() => {
    const label = [...document.querySelectorAll("label.timelineExternalSourceTarget")]
      .find((candidate) => candidate.querySelector("span")?.textContent?.trim() === "Lighting lane");
    return document.activeElement === label?.querySelector("select");
  })()`);
  assert.equal(focused, true, "real pointer focus stays on the child Timeline Lighting lane select before keyboard selection");
  const sendKey = async (key, code, windowsVirtualKeyCode) => {
    await client.send("Input.dispatchKeyEvent", { type: "keyDown", key, code, windowsVirtualKeyCode });
    await client.send("Input.dispatchKeyEvent", { type: "keyUp", key, code, windowsVirtualKeyCode });
  };
  await sendKey("Home", "Home", 36);
  for (let index = 0; index < target.expectedIndex; index += 1) {
    await sendKey("ArrowDown", "ArrowDown", 40);
  }
  await sendKey("Enter", "Enter", 13);
  await waitFor(() => evaluate(client, `(() => {
    const label = [...document.querySelectorAll("label.timelineExternalSourceTarget")]
      .find((candidate) => candidate.querySelector("span")?.textContent?.trim() === "Lighting lane");
    return label?.querySelector("select")?.value === ${JSON.stringify(String(expectedLayerId))};
  })()`), `real pointer/keyboard selection of child Lighting lane ${expectedLayerId}`);
};
const closeItemMenuIfNeeded = async (client) => {
  const closed = await evaluate(client, `(() => {
    const menu = document.querySelector('.timelineItemContextMenu');
    if (!(menu instanceof HTMLElement)) return true;
    const close = [...menu.querySelectorAll('button')].find((button) => button.textContent?.trim() === 'Close');
    if (close instanceof HTMLButtonElement) { close.click(); return true; }
    return false;
  })()`);
  assert.equal(closed, true, "Timeline item context menu exposes a Close action");
};
const saveScreenshot = async (client, name) => {
  if (!screenshotDir) return;
  const screenshot = await client.send("Page.captureScreenshot", { format: "png", captureBeyondViewport: false, fromSurface: true });
  await writeFile(join(screenshotDir, name), Buffer.from(screenshot.data, "base64"));
};

const installStrictBridge = () => `(() => {
  const clone = (value) => structuredClone(value);
  const exactKeys = (value, keys) => {
    if (!value || typeof value !== "object" || Array.isArray(value)) return false;
    const actual = Object.keys(value).sort();
    const expected = [...keys].sort();
    return actual.length === expected.length && actual.every((key, index) => key === expected[index]);
  };
  const isSafeInt = (value) => Number.isSafeInteger(value);
  const RENDERER_OWNER = /^renderer:.+/;
  const ITEM_KINDS = ["lighting_event", "video_clip", "audio_clip", "lighting_automation", "video_automation"];
  const itemIdKey = (kind) => kind === "lighting_event" ? "event_id"
    : kind === "lighting_automation" || kind === "video_automation" ? "automation_id" : "clip_id";
  const validItemRef = (item) => {
    if (!item || typeof item !== "object") return false;
    if (!ITEM_KINDS.includes(item.kind)) return false;
    const idKey = itemIdKey(item.kind);
    return exactKeys(item, ["kind", idKey]) && isSafeInt(item[idKey]) && item[idKey] > 0;
  };
  const jsonEqual = (a, b) => JSON.stringify(a) === JSON.stringify(b);
  const declineError = (message) => Object.assign(new Error(message), { __strictAuthorityDecline: true });

  const validateOwnerRegistration = (args) => {
    if (!exactKeys(args, ["ownerId"])) throw new Error("owner registration payload keys differ from the exact { ownerId } contract");
    if (typeof args.ownerId !== "string" || !RENDERER_OWNER.test(args.ownerId)) throw new Error("owner registration ownerId must be a renderer:* identity string");
    return null;
  };
  const validateProgramAudioHandoffConfig = (args) => {
    if (!exactKeys(args, ["enabled", "volume", "deviceName"])) throw new Error("program audio handoff payload keys differ from the exact three-field contract");
    if (typeof args.enabled !== "boolean") throw new Error("program audio handoff enabled must be boolean");
    if (typeof args.volume !== "number" || !Number.isFinite(args.volume) || args.volume < 0 || args.volume > 1) throw new Error("program audio handoff volume must be a finite number in [0,1]");
    if (args.deviceName !== null && (typeof args.deviceName !== "string" || args.deviceName.trim() === "")) throw new Error("program audio handoff deviceName must be null or a nonempty string");
    return null;
  };
  const validateTimelineEnvelope = (args) => {
    if (!exactKeys(args, ["request", "requestId", "expectedRevision", "expectedCheckpointHash", "expectedEpoch", "ownerId"])) {
      throw new Error("timeline advanced envelope keys differ from the exact server-authoritative contract");
    }
    if (!isSafeInt(args.requestId) || args.requestId <= 0) throw new Error("timeline advanced requestId must be a positive safe integer");
    if (!isSafeInt(args.expectedEpoch) || args.expectedEpoch < 0) throw new Error("timeline advanced expectedEpoch must be a non-negative safe integer");
    if (!isSafeInt(args.expectedRevision) || args.expectedRevision < 0) throw new Error("timeline advanced expectedRevision must be a non-negative safe integer");
    if (typeof args.expectedCheckpointHash !== "string") throw new Error("timeline advanced expectedCheckpointHash must be a string");
    if (typeof args.ownerId !== "string" || !RENDERER_OWNER.test(args.ownerId)) throw new Error("timeline advanced ownerId must be a renderer:* identity string");
    return args.request;
  };
  const validateSplitRequest = (request) => {
    if (request.kind !== "split_items" || !exactKeys(request, ["kind", "items", "primary", "boundary_ms", "isolate"])) {
      throw new Error("split_timeline_items request shape differs from the exact contract");
    }
    if (!Array.isArray(request.items) || request.items.length < 1 || !request.items.every(validItemRef)) throw new Error("split_timeline_items items must be exact Timeline item refs");
    if (!validItemRef(request.primary) || !request.items.some((item) => jsonEqual(item, request.primary))) throw new Error("split_timeline_items primary must be one of its items");
    if (!isSafeInt(request.boundary_ms) || request.boundary_ms < 0) throw new Error("split_timeline_items boundary_ms must be a non-negative safe integer");
    if (typeof request.isolate !== "boolean") throw new Error("split_timeline_items isolate must be boolean");
  };
  const validateMoveRequest = (request) => {
    if (request.kind !== "move_items_to_lanes" || !exactKeys(request, ["kind", "items", "primary", "lane_targets", "delta_ms", "isolate"])) {
      throw new Error("move_timeline_items_to_lanes request shape differs from the exact contract");
    }
    if (!Array.isArray(request.items) || request.items.length < 1 || !request.items.every(validItemRef)) throw new Error("move_timeline_items_to_lanes items must be exact Timeline item refs");
    if (!validItemRef(request.primary) || !request.items.some((item) => jsonEqual(item, request.primary))) throw new Error("move_timeline_items_to_lanes primary must be one of its items");
    if (!isSafeInt(request.delta_ms)) throw new Error("move_timeline_items_to_lanes delta_ms must be a safe integer");
    if (typeof request.isolate !== "boolean") throw new Error("move_timeline_items_to_lanes isolate must be boolean");
    if (!Array.isArray(request.lane_targets) || request.lane_targets.length !== request.items.length) throw new Error("move_timeline_items_to_lanes lane_targets must cover every item exactly once");
    for (const laneTarget of request.lane_targets) {
      if (!exactKeys(laneTarget, ["item", "target_layer_id"]) || !validItemRef(laneTarget.item)) throw new Error("move_timeline_items_to_lanes lane target item must be an exact Timeline item ref");
      if (!isSafeInt(laneTarget.target_layer_id) || laneTarget.target_layer_id <= 0) throw new Error("move_timeline_items_to_lanes target_layer_id must be a positive safe integer");
      if (!request.items.some((item) => jsonEqual(item, laneTarget.item))) throw new Error("move_timeline_items_to_lanes lane target does not match any requested item");
    }
  };

  const bridge = {
    calls: [],
    rejected: [],
    faults: [],
    registrationFaultArmed: true,
    releaseRegistrationFault() {
      this.registrationFaultArmed = false;
    },
    async invoke(command, args) {
      const received = { command, args: clone(args ?? {}) };
      try {
        if (command === "register_project_transaction_owner") {
          if (this.registrationFaultArmed) {
            this.registrationFaultArmed = false;
            this.faults.push({ ...received });
            throw declineError("injected one-time owner registration fault for the trusted-rearm proof");
          }
          const reply = validateOwnerRegistration(received.args);
          this.calls.push({ ...received, operation: "register_project_transaction_owner" });
          return reply;
        }
        if (command === "set_program_audio_handoff_config") {
          const reply = validateProgramAudioHandoffConfig(received.args);
          this.calls.push({ ...received, operation: "set_program_audio_handoff_config" });
          return reply;
        }
        if (command === "apply_timeline_advanced_authoritative") {
          const request = validateTimelineEnvelope(received.args);
          if (request && request.kind === "split_items") validateSplitRequest(request);
          else if (request && request.kind === "move_items_to_lanes") validateMoveRequest(request);
          else throw new Error("apply_timeline_advanced_authoritative request kind " + JSON.stringify(request && request.kind) + " is outside this strict gate's two Timeline operations");
          this.calls.push({ ...received, operation: request.kind });
          throw declineError("validated " + request.kind + " (" + (request.kind === "split_items" ? "split_timeline_items" : "move_timeline_items_to_lanes") + ") request intentionally declined: the browser authority bridge owns no backend project projection");
        }
        throw new Error("strict authority bridge rejected unregistered command: " + command);
      } catch (error) {
        if (!error || error.__strictAuthorityDecline !== true) {
          this.rejected.push({ ...received, reason: String(error && error.message ? error.message : error) });
        }
        throw error;
      }
    },
  };
  window.__syndocalStrictAuthorityBridge = bridge;
  window.__TAURI_INTERNALS__ = { invoke: (command, args) => bridge.invoke(command, args) };
})()`;

let vite;
let browser;
let client;
let profileDir;
try {
  const browserPath = browsers.find((candidate) => existsSync(candidate));
  assert.ok(browserPath, "Chrome or Edge is required for the Timeline authority browser gate");
  await sweepOwnedLegacyListeners();
  vitePort = await allocateFreePort("Vite HTTP");
  cdpPort = await allocateFreePort("Chrome DevTools");
  while (cdpPort === vitePort) cdpPort = await allocateFreePort("Chrome DevTools");
  baseUrl = `http://${host}:${vitePort}/?syndocalViewportFixture=timeline-layered`;
  await assertListenPortFree(vitePort, "Vite fixture server");
  vite = spawn(process.execPath, [resolve(appRoot, "node_modules/vite/bin/vite.js"), "--host", host, "--port", String(vitePort), "--strictPort"], { cwd: appRoot, stdio: "ignore" });
  await waitFor(async () => {
    if (vite.exitCode !== null) {
      throw new FatalGateSignal(`Vite exited early with code ${vite.exitCode} before serving ${baseUrl} (port conflict or crash; --strictPort forbids fallback)`);
    }
    return (await fetch(baseUrl)).ok;
  }, "Vite fixture server");
  await assertListenerOwnership(vitePort, vite.pid, null, "Vite fixture server");
  profileDir = await mkdtemp(join(tmpdir(), profileMarker));
  await assertListenPortFree(cdpPort, "browser CDP endpoint");
  browser = spawn(browserPath, ["--headless=new", "--disable-gpu", "--no-first-run", "--no-default-browser-check", `--remote-debugging-port=${cdpPort}`, `--user-data-dir=${profileDir}`, "about:blank"], { stdio: "ignore" });
  const target = await waitFor(async () => {
    // Liveness is proven only by the CDP endpoint answering AND every listener
    // carrying this run's unique profile dir; the spawned PID may already be
    // gone because --headless=new relaunches itself.
    const response = await fetch(`http://${host}:${cdpPort}/json/new?${encodeURIComponent(baseUrl)}`, { method: "PUT" });
    if (!response.ok) return null;
    await assertListenerOwnership(cdpPort, browser?.pid ?? null, profileDir, "browser CDP endpoint");
    return response.json();
  }, "browser target");
  client = new CdpClient(target.webSocketDebuggerUrl);
  await client.ready();
  await client.send("Page.enable");
  await client.send("Runtime.enable");
  await client.send("Emulation.setDeviceMetricsOverride", { ...viewport, deviceScaleFactor: 1, mobile: false });
  if (screenshotDir) await mkdir(screenshotDir, { recursive: true });
  await waitFor(() => evaluate(client, "document.querySelector('.app') && document.readyState === 'complete'"), "app mount");

  assert.equal(await click(client, '[data-workspace-option="control"]'), true);
  assert.equal(await click(client, '[data-edit-domain-navigation] [data-control-mode-option="live"]'), true);
  assert.equal(await waitFor(() => click(client, '[data-timeline-desk-surface="show"]'), "Timeline Show tab"), true);
  await waitFor(() => evaluate(client, "document.querySelectorAll('.timelineVideoClip').length === 1 && document.querySelectorAll('.timelineAudioClip').length === 2"), "authored Timeline media clips");

  assert.equal(await evaluate(client, "window.__TAURI_INTERNALS__ === undefined"), true, "no Tauri internals exist before the post-mount install");
  const preInstallStatus = await statusText(client);
  assert.match(preInstallStatus, /not connected in this browser preview|registration failed/i, "mount without Tauri internals leaves the visible fail-closed backend/registration status");

  await evaluate(client, installStrictBridge());

  await saveScreenshot(client, "timeline-authority-before-rearm.png");

  // Phase 1: the bridge injects exactly one registration fault so the app
  // latches its real fail-closed hasFailure state through the production
  // attempt path. A synthetic-only split gesture triggers that attempt.
  const runSplitGesture = async (label) => {
    await altClickSelect(client, '[data-timeline-audio-clip-id="700"]');
    await sleep(50);
    assert.equal(await contextMenuAction(client, `[data-timeline-audio-clip-id="700"]`, 240, 180), true, `${label}: Alt-isolated Audio clip opens the production Timeline item menu`);
    await sleep(50);
    assert.equal(await clickSplitAction(client), true, `${label}: Split action is clickable`);
  };
  await runSplitGesture("faulting phase");
  const faulted = await waitFor(async () => {
    const state = await bridgeState(client);
    return state.faults.length === 1 ? state : null;
  }, "one-time injected owner-registration fault");
  const faultedRegistrationArgs = faulted.faults[0].args;
  assert.deepEqual(Object.keys(faultedRegistrationArgs).sort(), ["ownerId"], "the faulted registration attempt carries exactly the single key ownerId");
  assert.equal(typeof faultedRegistrationArgs.ownerId, "string", "faulted registration ownerId is a string value");
  assert.match(faultedRegistrationArgs.ownerId, /^renderer:.+/, "faulted registration ownerId is a renderer-scoped identity");
  assert.equal(faulted.calls.length, 0, "no allowlisted operation is recorded while registration is faulted");
  assert.equal(faulted.rejected.length, 0, "no command reaches the bridge behind a faulted registration barrier");
  assert.match(await statusText(client), /Project transaction owner registration failed/i, "the app latches its visible keyed owner-registration failure status");
  await closeItemMenuIfNeeded(client);

  // Phase 2: make the bridge willing while the app-side retry stays unarmed.
  await evaluate(client, "window.__syndocalStrictAuthorityBridge.releaseRegistrationFault()");
  const beforeUnarmed = await bridgeState(client);
  await runSplitGesture("unarmed phase");
  await sleep(900);
  const unarmed = await bridgeState(client);
  assert.deepEqual(
    { calls: unarmed.calls.length, rejected: unarmed.rejected.length, faults: unarmed.faults.length },
    { calls: beforeUnarmed.calls.length, rejected: beforeUnarmed.rejected.length, faults: beforeUnarmed.faults.length },
    "with a willing bridge but no trusted input, synthetic-only interaction emits zero IPC and rearms nothing",
  );
  assert.match(await statusText(client), /Project transaction owner registration failed/i, "the keyed registration failure remains latched against untrusted retries");
  await closeItemMenuIfNeeded(client);

  // Phase 3: a real CDP pointer gesture arms the app's trusted-input retry.
  // The target is checked with elementFromPoint first; this may never become a
  // blind coordinate gesture which could silently miss the rendered Timeline.
  await hitVerifiedPointerClick(
    client,
    '[data-timeline-desk-surface="show"]',
    "trusted owner-registration rearm",
  );

  // Phase 4: the same synthetic gesture now registers and dispatches the
  // exact production split request.
  await runSplitGesture("rearmed phase");
  const splitProof = await waitFor(async () => {
    const state = await bridgeState(client);
    const split = state.calls.filter((call) => call.operation === "split_items");
    const register = state.calls.filter((call) => call.operation === "register_project_transaction_owner");
    return split.length === 1 && register.length === 1 ? state : null;
  }, "trusted-pointer-rearmed registration and fixture-free production split dispatch");
  const registerCall = splitProof.calls.find((call) => call.operation === "register_project_transaction_owner");
  const splitCall = splitProof.calls.find((call) => call.operation === "split_items");
  assert.ok(splitProof.calls.indexOf(registerCall) < splitProof.calls.indexOf(splitCall), "owner registration strictly precedes the first server-authoritative mutation (ownership barrier)");
  assert.deepEqual(Object.keys(registerCall.args).sort(), ["ownerId"], "registration payload carries exactly the single key ownerId");
  assert.equal(typeof registerCall.args.ownerId, "string", "registration ownerId is a string value");
  assert.match(registerCall.args.ownerId, /^renderer:.+/, "registration ownerId is a renderer-scoped identity");
  assert.deepEqual(splitCall.args.request, {
    kind: "split_items",
    items: [{ kind: "audio_clip", clip_id: 700 }],
    primary: { kind: "audio_clip", clip_id: 700 },
    boundary_ms: 1_000,
    isolate: true,
  }, "split_timeline_items emits the exact Alt-isolated request through apply_timeline_advanced_authoritative");
  assert.equal(Number.isSafeInteger(splitCall.args.requestId) && splitCall.args.requestId > 0, true, "split requestId is a positive safe integer");
  assert.equal(splitCall.args.ownerId, registerCall.args.ownerId, "mutation envelope reuses the registered renderer owner identity");
  await closeItemMenuIfNeeded(client);

  await waitFor(async () => {
    const state = await bridgeState(client);
    return state.calls.some((call) => call.operation === "set_program_audio_handoff_config");
  }, "post-registration Program Audio handoff startup sync");
  const handoffState = await bridgeState(client);
  const handoffCalls = handoffState.calls.filter((call) => call.operation === "set_program_audio_handoff_config");
  assert.equal(handoffCalls.length, 1, "Program Audio handoff syncs exactly once per registration revision");
  assert.deepEqual(Object.keys(handoffCalls[0].args).sort(), ["deviceName", "enabled", "volume"], "handoff payload carries exactly the three-field Program Audio config");

  const organicRejections = () => bridgeState(client).then((state) => state.rejected.filter((entry) => entry.command === "get_timeline_advanced_operation_terminal_result"));
  assert.equal((await organicRejections()).length, 1, "the declined split triggers exactly one typed terminal-result recovery read, itself rejected by the strict allowlist");
  assert.match(await statusText(client), /Timeline selection split failed.*declined/s, "declined split surfaces a visible fail-closed message");

  const sceneSelector = '.timelineMarker[data-timeline-event-id]';
  const invalidTargets = [
    { selector: sceneSelector, layer: 10, label: "Scene pointer drop on Audio Bed lane" },
    { selector: '[data-timeline-automation-kind="video"][data-timeline-automation-id="2"]', layer: 10, label: "wrong-kind Video automation drop on Audio lane" },
  ];
  for (const invalidCase of invalidTargets) {
    const before = await bridgeState(client);
    await dragTimelineItemToLane(client, invalidCase.selector, invalidCase.layer);
    await sleep(150);
    const after = await bridgeState(client);
    assert.deepEqual(
      { calls: after.calls.length, rejected: after.rejected.length },
      { calls: before.calls.length, rejected: before.rejected.length },
      `${invalidCase.label} emits no mutation request (fixture or native)`,
    );
  }

  const beforeMove = await bridgeState(client);
  await dragTimelineItemToLane(client, '[data-timeline-video-clip-id="800"]', 15);
  const moveProof = await waitFor(async () => {
    const state = await bridgeState(client);
    const moves = state.calls.slice(beforeMove.calls.length).filter((call) => call.operation === "move_items_to_lanes");
    return moves.length === 1 ? state : null;
  }, "linked lane-move production dispatch through apply_timeline_advanced_authoritative");
  const moveCall = moveProof.calls.slice(beforeMove.calls.length).find((call) => call.operation === "move_items_to_lanes");
  assert.equal(Number.isSafeInteger(moveCall.args.request.delta_ms) && moveCall.args.request.delta_ms !== 0, true, "lane move carries a nonzero signed pointer delta");
  assert.deepEqual(
    { ...moveCall.args.request, delta_ms: "<nonzero signed pointer delta>" },
    {
      kind: "move_items_to_lanes",
      items: [{ kind: "video_clip", clip_id: 800 }, { kind: "audio_clip", clip_id: 700 }],
      primary: { kind: "video_clip", clip_id: 800 },
      lane_targets: [
        { item: { kind: "video_clip", clip_id: 800 }, target_layer_id: 15 },
        { item: { kind: "audio_clip", clip_id: 700 }, target_layer_id: 11 },
      ],
      delta_ms: "<nonzero signed pointer delta>",
      isolate: false,
    },
    "move_timeline_items_to_lanes emits the exact linked group plan through apply_timeline_advanced_authoritative",
  );
  assert.equal(moveCall.args.ownerId, registerCall.args.ownerId, "lane-move envelope reuses the registered renderer owner identity");
  assert.equal((await organicRejections()).length, 2, "the declined lane move triggers exactly one further typed terminal-result recovery read, also rejected");
  assert.match(await statusText(client), /lane move failed.*declined|rejected unregistered command/s, "the declined lane move (or its rejected runtime side-command) leaves a visible fail-closed message");

  // A child Timeline remains an editable acyclic descendant, but its owner
  // Scene (and any recursive target) must be absent from the source shelf.
  // Exercise the real Super Scene pointer path and the terminal App placement
  // route rather than a synthetic component callback.
  const rootMarkersBeforeChild = await evaluate(client, `(() =>
    [...document.querySelectorAll('.timelineMarker[data-timeline-event-id]')]
      .map((marker) => ({
        id: marker.getAttribute('data-timeline-event-id'),
        cueId: marker.getAttribute('data-timeline-cue-id'),
        layer: marker.getAttribute('data-timeline-layer-id'),
        label: marker.querySelector('.timelineSceneBlockLabel')?.getAttribute('data-full-label') ?? null,
      }))
      .sort((left, right) => Number(left.id) - Number(right.id))
  )()`);
  assert.ok(rootMarkersBeforeChild.some((marker) => marker.id === "8500" && marker.layer === "13" && marker.label === "Shin"), "root fixture exposes the direct Super Scene on its authored Lighting lane");
  await hitVerifiedPointerClick(client, '[data-timeline-event-id="8500"]', "child Timeline Super Scene first pointer click", 1);
  await hitVerifiedPointerClick(client, '[data-timeline-event-id="8500"]', "child Timeline Super Scene double-click", 2);
  await waitFor(() => evaluate(client, "document.querySelector('[data-child-timeline-label]')?.textContent?.trim() === 'Shin'"), "real pointer opening child Super Scene Shin");
  const childBeforePlacement = await evaluate(client, `(() => ({
    markers: [...document.querySelectorAll('.timelineMarker[data-timeline-event-id]')]
      .map((marker) => ({
        id: marker.getAttribute('data-timeline-event-id'),
        cueId: marker.getAttribute('data-timeline-cue-id'),
        layer: marker.getAttribute('data-timeline-layer-id'),
        label: marker.querySelector('.timelineSceneBlockLabel')?.getAttribute('data-full-label') ?? null,
      }))
      .sort((left, right) => Number(left.id) - Number(right.id)),
    ownerPresent: document.querySelector('[data-timeline-source-cue-id="350"]') !== null,
    valid: (() => {
      const source = document.querySelector('[data-timeline-source-cue-id="301"]');
      return {
        present: source !== null,
        label: source?.querySelector('strong')?.textContent?.trim() ?? null,
      };
    })(),
  }))()`);
  assert.deepEqual(childBeforePlacement.markers.map((marker) => marker.id), ["8501", "8502"], "child Timeline starts with its two authored Scene events only");
  assert.equal(childBeforePlacement.ownerPresent, false, "child Timeline owner Cue 350 is excluded from the source shelf");
  assert.equal(childBeforePlacement.valid.present, true, "acyclic descendant Cue 301 remains available in the child Timeline source shelf");
  assert.ok(childBeforePlacement.valid.label, "available Cue 301 exposes an operator-visible source label");
  const validSceneEnabled = await evaluate(client, `(() => {
    const source = document.querySelector('[data-timeline-source-cue-id="301"]');
    return source instanceof HTMLButtonElement && !source.disabled && source.getAttribute('data-timeline-source-shelf-scene-placeable') === 'true';
  })()`);
  assert.equal(validSceneEnabled, true, "valid Cue 301 remains an enabled child Timeline placement source");
  await selectTimelineShelfLightingLaneWithPointerKeyboard(client, 51);
  const bridgeBeforeChildPlacement = await bridgeState(client);
  await hitVerifiedPointerClick(client, '[data-timeline-source-cue-id="301"]', "child Timeline Cue 301 source placement");
  const expectedChildSourceLabel = JSON.stringify(childBeforePlacement.valid.label);
  let childAfterPlacement;
  try {
    childAfterPlacement = await waitFor(() => evaluate(client, `(() => {
    const markers = [...document.querySelectorAll('.timelineMarker[data-timeline-event-id]')]
      .map((marker) => ({
        id: marker.getAttribute('data-timeline-event-id'),
        cueId: marker.getAttribute('data-timeline-cue-id'),
        layer: marker.getAttribute('data-timeline-layer-id'),
        label: marker.querySelector('.timelineSceneBlockLabel')?.getAttribute('data-full-label') ?? null,
      }));
    const added = markers.filter((marker) => !["8501", "8502"].includes(marker.id));
    return markers.length === 3 && added.length === 1 && added[0].cueId === "301" && added[0].layer === "51" && added[0].label === ${expectedChildSourceLabel}
      ? { markers, added: added[0] }
      : null;
  })()`), "child Timeline Cue 301 placement on exact Lighting lane 51");
  } catch (error) {
    const diagnostic = await evaluate(client, `(() => ({
      markers: [...document.querySelectorAll('.timelineMarker[data-timeline-event-id]')].map((marker) => ({
        id: marker.getAttribute('data-timeline-event-id'),
        cueId: marker.getAttribute('data-timeline-cue-id'),
        layer: marker.getAttribute('data-timeline-layer-id'),
        label: marker.querySelector('.timelineSceneBlockLabel')?.getAttribute('data-full-label') ?? null,
      })),
      status: document.querySelector('[data-app-status]')?.textContent ?? null,
      shelf: [...document.querySelectorAll('[data-timeline-source-cue-id]')].map((source) => ({
        id: source.getAttribute('data-timeline-source-cue-id'),
        disabled: source.getAttribute('data-timeline-source-disabled'),
      })),
    }))()`);
    throw new Error(`${String(error)}; diagnostic=${JSON.stringify(diagnostic)}`);
  }
  assert.equal(childAfterPlacement.added.layer, "51", "child placement preserves the exact selected Lighting lane");
  assert.equal(childAfterPlacement.added.cueId, "301", "child placement preserves the exact source Cue identity");
  assert.equal(childAfterPlacement.added.label, childBeforePlacement.valid.label, "child placement preserves the exact operator-visible source label");
  assert.match(await statusText(client), /Scene 301 placed on the Timeline\./, "successful child placement is visible to the operator");
  const bridgeAfterChildPlacement = await bridgeState(client);
  assert.deepEqual(
    { calls: bridgeAfterChildPlacement.calls.length, rejected: bridgeAfterChildPlacement.rejected.length },
    { calls: bridgeBeforeChildPlacement.calls.length, rejected: bridgeBeforeChildPlacement.rejected.length },
    "valid child fixture placement emits no legacy Tauri IPC or rejected side-channel request",
  );
  await hitVerifiedPointerClick(client, 'details.timelineToolsDisclosure > summary', "reveal child Timeline return control");
  await waitFor(() => evaluate(client, `(() => {
    const exit = document.querySelector('details.timelineToolsDisclosure[open] [data-timeline-breadcrumb] button');
    const rect = exit?.getBoundingClientRect();
    return exit instanceof HTMLButtonElement && rect.width > 0 && rect.height > 0;
  })()`), "visible child Timeline return control after opening Timeline tools");
  await hitVerifiedPointerClick(client, '[data-timeline-breadcrumb] button', "return from child Timeline Shin");
  await waitFor(() => evaluate(client, "document.querySelector('[data-child-timeline-label]') === null"), "returning to the root Timeline");
  const rootMarkersAfterChild = await evaluate(client, `(() =>
    [...document.querySelectorAll('.timelineMarker[data-timeline-event-id]')]
      .map((marker) => ({
        id: marker.getAttribute('data-timeline-event-id'),
        cueId: marker.getAttribute('data-timeline-cue-id'),
        layer: marker.getAttribute('data-timeline-layer-id'),
        label: marker.querySelector('.timelineSceneBlockLabel')?.getAttribute('data-full-label') ?? null,
      }))
      .sort((left, right) => Number(left.id) - Number(right.id))
  )()`);
  assert.deepEqual(rootMarkersAfterChild, rootMarkersBeforeChild, "child placement changes no root Timeline event");

  const probe = await evaluate(client, `(async () => {
    try {
      await window.__TAURI_INTERNALS__.invoke('begin_project_transaction', {});
      return { rejected: false };
    } catch (error) {
      return { rejected: true, reason: String(error && error.message ? error.message : error) };
    }
  })()`);
  assert.equal(probe.rejected, true, "unknown command begin_project_transaction is rejected by the strict bridge");
  assert.match(probe.reason, /unregistered command/, "unknown-command rejection names the strict allowlist failure");

  const finalState = await bridgeState(client);
  const operationCounts = finalState.calls.reduce((counts, call) => {
    counts[call.operation] = (counts[call.operation] ?? 0) + 1;
    return counts;
  }, {});
  assert.deepEqual(operationCounts, {
    register_project_transaction_owner: 1,
    set_program_audio_handoff_config: 1,
    split_items: 1,
    move_items_to_lanes: 1,
  }, "observed IPC surface is exactly the four allowed operations, each once, with no extra commands");
  const callOrder = finalState.calls.map((call) => call.operation);
  assert.ok(callOrder.indexOf("register_project_transaction_owner") < callOrder.indexOf("split_items")
    && callOrder.indexOf("split_items") < callOrder.indexOf("move_items_to_lanes"), "registration precedes both acknowledged-order mutations");
  const allowedCommands = new Set(["register_project_transaction_owner", "set_program_audio_handoff_config", "apply_timeline_advanced_authoritative"]);
  assert.ok(finalState.rejected.every((entry) => !allowedCommands.has(entry.command)), "every rejection was genuinely outside the four-operation allowlist");
  assert.equal(finalState.rejected.filter((entry) => entry.command === "get_timeline_advanced_operation_terminal_result").length, 2, "both declined mutations attempted exactly one typed recovery read each");
  assert.equal(finalState.rejected.some((entry) => entry.command === "begin_project_transaction"), true, "the direct unknown-command probe was recorded");
  for (const seek of finalState.rejected.filter((entry) => entry.command === "seek_timeline")) {
    assert.equal(Object.keys(seek.args).sort().join(","), "positionMs", "rejected runtime seeks carry exactly { positionMs }");
    assert.equal(Number.isSafeInteger(seek.args.positionMs) && seek.args.positionMs >= 0, true, "rejected runtime seeks carry non-negative integer positions");
  }

  console.log(`Timeline authority gate ${viewport.width}x${viewport.height}: injected-fault latch, trusted-input-only rearm, exact registration/handoff/split/move payloads, invalid-target silence, and unknown-command rejection passed`);
  console.log("PROOF BOUNDARY: browser IPC request-contract evidence only -- no native Tauri, backend coordinator, persistence, or hardware completion is claimed.");
} finally {
  client?.close();
  await stopChild(browser, "headless browser");
  await stopChild(vite, "Vite fixture server");
  await terminateVerifiedGateListeners();
  if (profileDir) await rm(profileDir, { recursive: true, force: true });
}
