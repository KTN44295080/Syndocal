import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { connect as createTcpConnection } from "node:net";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { createServer as createViteServer } from "vite";

const appRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const host = "127.0.0.1";
let vitePort;
let cdpPort;
let baseUrl;
let faviconUrl;
const screenshotDir = resolve(
  process.env.SYNDOCAL_CONTROL_SCREENSHOT_DIR ?? "C:\\TEMP\\syndocal-control-ui-checkpoints",
);
const productMinimumWindow = { width: 960, height: 640 };
// The show-core uses these four physical desktop classes. Smaller browser and
// detached-pane tests remain supplemental and do not define this acceptance.
const defaultViewports = [
  { width: 3840, height: 2160 },
  { width: 2560, height: 1440 },
  { width: 1920, height: 1080 },
  { width: 1280, height: 720 },
];
const selectedViewport = process.env.SYNDOCAL_CONTROL_VIEWPORT;
const viewports = (() => {
  if (selectedViewport === undefined) return defaultViewports;
  const match = defaultViewports.find(({ width, height }) => `${width}x${height}` === selectedViewport);
  if (!match) {
    throw new Error(`SYNDOCAL_CONTROL_VIEWPORT must be one exact supported viewport; received ${JSON.stringify(selectedViewport)}`);
  }
  return [match];
})();
const browserCandidates = [
  process.env.CHROME_PATH,
  "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe",
  "C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe",
  "C:\\Program Files\\Microsoft\\Edge\\Application\\msedge.exe",
].filter(Boolean);

// Cleanup is deliberately bounded independently from the browser assertions.
// The browser is spawned detached, so a hung teardown must fail closed with an
// attributable diagnostic instead of leaving the parent waiting forever.
const cleanupTimeoutsMs = Object.freeze({
  cdpClose: 2_000,
  // taskkill /T /F can take tens of seconds while Chromium drains its
  // renderer tree. Keep the deadline finite, but long enough to complete the
  // exact tree termination observed on the supported Windows runner.
  taskkill: 30_000,
  childExit: 10_000,
  browserTree: 90_000,
  viteClose: 5_000,
  endpointRelease: 10_000,
  processQuery: 15_000,
  profileAttempt: 2_000,
  profile: 15_000,
});
const monotonicDeadlineExceededCode = "ERR_CONTROL_UPPER_WORKSPACES_MONOTONIC_DEADLINE";
const cleanupSelfTestOnly = process.env.SYNDOCAL_CONTROL_UPPER_WORKSPACES_CLEANUP_SELF_TEST === "1";
const cleanupSelfTestComplete = Symbol("cleanup-self-test-complete");
const runToken = process.env.SYNDOCAL_CONTROL_UPPER_WORKSPACES_RUN_TOKEN;
if (runToken !== undefined && !/^[A-Za-z0-9_-]{1,128}$/.test(runToken)) {
  throw new Error(`SYNDOCAL_CONTROL_UPPER_WORKSPACES_RUN_TOKEN is invalid: ${JSON.stringify(runToken)}`);
}
const browserProfilePrefix = runToken === undefined
  ? "syndocal-control-upper-workspaces-"
  : `syndocal-control-upper-workspaces-${runToken}-`;
const maxOwnedBrowserProcesses = 64;
const cdpTimeoutsMs = Object.freeze({
  request: 15_000,
  ready: 15_000,
  navigate: 30_000,
  evaluate: 15_000,
  screenshot: 15_000,
});

const monotonicNowMs = () => Number(process.hrtime.bigint()) / 1_000_000;
const cleanupStartedAtMs = monotonicNowMs();
let lastCleanupCheckpointMs = cleanupStartedAtMs;
let cleanupCheckpointOrdinal = 0;
const cleanupCheckpoint = (label) => {
  const observedNowMs = monotonicNowMs();
  // process.hrtime.bigint() is monotonic, but clamping keeps the diagnostic
  // sequence monotonic even if a test double supplies a regressing clock.
  const nowMs = Math.max(lastCleanupCheckpointMs, observedNowMs);
  lastCleanupCheckpointMs = nowMs;
  cleanupCheckpointOrdinal += 1;
  console.error(
    `[control-upper cleanup checkpoint ${cleanupCheckpointOrdinal}] ${label} `
    + `(elapsed=${Math.round(nowMs - cleanupStartedAtMs)}ms)`,
  );
};

// Promise.race alone leaves the losing operation alive. That is intentional:
// the operation still owns its resource, while the caller records a bounded
// failure and decides whether to continue its exact cleanup sequence.
// Optional onDeadline work is synchronous and must itself never wait.
const runWithMonotonicDeadline = async (label, operation, timeoutMs, onDeadline) => {
  assert.ok(Number.isFinite(timeoutMs) && timeoutMs > 0, `${label} timeout must be positive`);
  const deadlineMs = monotonicNowMs() + timeoutMs;
  let timer;
  let timedOut = false;
  const operationPromise = Promise.resolve().then(operation);
  // A timed-out operation may settle later; consume that settlement so a
  // late rejection cannot become an unhandled process failure.
  operationPromise.catch(() => {});
  const deadlinePromise = new Promise((_, rejectDeadline) => {
    const delayMs = Math.max(1, deadlineMs - monotonicNowMs());
    timer = setTimeout(() => {
      timedOut = true;
      try {
        onDeadline?.();
      } catch {
        // The bounded failure below is the authoritative diagnostic.
      }
      rejectDeadline(new Error(`${label} exceeded monotonic cleanup deadline of ${timeoutMs} ms`));
    }, delayMs);
  });
  try {
    return await Promise.race([operationPromise, deadlinePromise]);
  } catch (error) {
    if (timedOut) {
      const deadlineError = new Error(`${label} exceeded monotonic cleanup deadline of ${timeoutMs} ms`, { cause: error });
      deadlineError.code = monotonicDeadlineExceededCode;
      throw deadlineError;
    }
    throw error;
  } finally {
    clearTimeout(timer);
  }
};

const runBoundedCleanup = async (label, operation, timeoutMs, onDeadline) => {
  assert.ok(Number.isFinite(timeoutMs) && timeoutMs > 0, `${label} cleanup timeout must be positive`);
  cleanupCheckpoint(`${label}: start`);
  let timedOut = false;
  try {
    return await runWithMonotonicDeadline(label, operation, timeoutMs, () => {
      timedOut = true;
      onDeadline?.();
    });
  } finally {
    cleanupCheckpoint(`${label}: ${timedOut ? "deadline exceeded" : "settled"}`);
  }
};

const sleep = (milliseconds) => new Promise((resolveSleep) => setTimeout(resolveSleep, milliseconds));
const waitFor = async (check, label, timeoutMs = 30_000, checkTimeoutMs = Math.min(timeoutMs, cdpTimeoutsMs.request)) => {
  const deadline = monotonicNowMs() + timeoutMs;
  let lastError;
  while (monotonicNowMs() < deadline) {
    try {
      const remainingMs = deadline - monotonicNowMs();
      if (remainingMs <= 0) break;
      const value = await runWithMonotonicDeadline(
        `${label} check`,
        check,
        Math.max(1, Math.min(checkTimeoutMs, remainingMs)),
      );
      if (value) return value;
    } catch (error) {
      lastError = error;
    }
    const remainingAfterCheckMs = deadline - monotonicNowMs();
    if (remainingAfterCheckMs <= 0) break;
    await sleep(Math.min(100, remainingAfterCheckMs));
  }
  throw new Error(`Timed out waiting for ${label}${lastError ? `: ${lastError}` : ""}`);
};

const runCaptured = (file, args, label, timeoutMs) => {
  let child;
  let onError;
  let onExit;
  return runWithMonotonicDeadline(label, () => new Promise((resolveCaptured, rejectCaptured) => {
    child = spawn(file, args, {
      stdio: ["ignore", "pipe", "pipe"],
      windowsHide: true,
    });
    let stdout = "";
    let stderr = "";
    child.stdout.setEncoding("utf8");
    child.stderr.setEncoding("utf8");
    child.stdout.on("data", (chunk) => { stdout += chunk; });
    child.stderr.on("data", (chunk) => { stderr += chunk; });
    onError = (error) => {
      child.removeListener("exit", onExit);
      rejectCaptured(error);
    };
    onExit = (status, signal) => {
      child.removeListener("error", onError);
      if (status === 0 && signal === null) {
        resolveCaptured(stdout);
        return;
      }
      rejectCaptured(new Error(`${label} exited with status ${status ?? "null"} signal ${signal ?? "none"}: ${stderr.trim()}`));
    };
    child.once("error", onError);
    child.once("exit", onExit);
  }), timeoutMs, () => {
    if (child) {
      if (onError) child.removeListener("error", onError);
      if (onExit) child.removeListener("exit", onExit);
    }
    if (child && !childHasExited(child)) {
      try {
        child.kill();
      } catch {
        // The helper may have exited between the check and kill.
      }
    }
  });
};

const fetchWithDeadline = (url, init = {}, timeoutMs = cdpTimeoutsMs.request) => {
  const controller = new AbortController();
  return runWithMonotonicDeadline(
    `fetch ${url}`,
    () => fetch(url, { ...init, signal: controller.signal }),
    timeoutMs,
    () => controller.abort(),
  );
};

const childHasExited = (child) => !child || child.exitCode !== null || child.signalCode !== null;
const remainingCleanupTimeout = (label, deadlineMs, maximumMs) => {
  const remainingMs = deadlineMs - monotonicNowMs();
  if (remainingMs <= 0) throw new Error(`${label} cleanup deadline exhausted before the next operation`);
  return Math.max(1, Math.min(maximumMs, remainingMs));
};
const assertChildAlive = (child, label) => {
  if (!child) throw new Error(`${label} was not spawned`);
  if (child.spawnError) throw new Error(`${label} failed to spawn: ${child.spawnError.message}`);
  if (childHasExited(child)) {
    throw new Error(`${label} exited before readiness (status=${child.exitCode}, signal=${child.signalCode})`);
  }
};

const waitForChildExit = async (child, label, timeoutMs = 5_000) => {
  if (!child || childHasExited(child)) return;
  let onExit;
  await runBoundedCleanup(`${label} child exit`, () => new Promise((resolveExit) => {
    onExit = () => {
      child.removeListener("exit", onExit);
      resolveExit();
    };
    child.once("exit", onExit);
    if (childHasExited(child)) {
      child.removeListener("exit", onExit);
      resolveExit();
    }
  }), timeoutMs, () => {
    if (onExit) child.removeListener("exit", onExit);
  });
  if (!childHasExited(child)) throw new Error(`${label} exit could not be verified`);
};

const runTaskkillTree = (pid, timeoutMs = cleanupTimeoutsMs.taskkill) => {
  let killer;
  let onError;
  let onExit;
  return runBoundedCleanup(`taskkill tree PID ${pid}`, () => new Promise((resolveKill, rejectKill) => {
    killer = spawn("taskkill.exe", ["/PID", String(pid), "/T", "/F"], {
      stdio: "ignore",
      windowsHide: true,
    });
    onError = rejectKill;
    onExit = (status, signal) => resolveKill({ status, signal });
    killer.once("error", onError);
    killer.once("exit", onExit);
  }), timeoutMs, () => {
    if (killer) {
      if (onError) killer.removeListener("error", onError);
      if (onExit) killer.removeListener("exit", onExit);
    }
    if (killer && !childHasExited(killer)) {
      try {
        killer.kill();
      } catch {
        // The taskkill helper may have exited between the check and kill.
      }
    }
  });
};

const getOwnedBrowserProcesses = async (timeoutMs = cleanupTimeoutsMs.processQuery) => {
  if (process.platform !== "win32" || !profileDir) return [];
  const script = "$ErrorActionPreference='Stop';"
    + `$needle=${JSON.stringify(profileDir)};`
    + "Get-CimInstance Win32_Process -Filter \"Name='chrome.exe' OR Name='msedge.exe'\" -ErrorAction Stop | "
    + "Where-Object { $_.CommandLine "
    + "-and $_.CommandLine.ToLowerInvariant().Contains($needle.ToLowerInvariant()) } | "
    + "Select-Object ProcessId,ParentProcessId,Name,CommandLine | ConvertTo-Json -Compress";
  const output = (await runCaptured(
    "powershell.exe",
    ["-NoProfile", "-NonInteractive", "-Command", script],
    "owned browser process query",
    timeoutMs,
  )).trim();
  if (!output) return [];
  const records = JSON.parse(output);
  return (Array.isArray(records) ? records : [records])
    .filter((record) => Number.isSafeInteger(Number(record?.ProcessId)) && Number(record.ProcessId) > 0)
    .map((record) => ({
      pid: Number(record.ProcessId),
      parentPid: Number(record.ParentProcessId),
      name: String(record.Name ?? ""),
      commandLine: String(record.CommandLine ?? ""),
    }));
};

const terminateOwnedBrowserProcesses = async (label, deadlineMs = monotonicNowMs() + cleanupTimeoutsMs.browserTree) => {
  if (process.platform !== "win32" || !profileDir) return;
  let records = await getOwnedBrowserProcesses(
    remainingCleanupTimeout(`${label} owned browser process query`, deadlineMs, cleanupTimeoutsMs.processQuery),
  );
  if (records.length > maxOwnedBrowserProcesses) {
    throw new Error(`${label} owned browser process query returned ${records.length} processes; refusing an unbounded cleanup sweep`);
  }
  const attemptedPids = new Set();
  while (records.length > 0) {
    if (attemptedPids.size >= maxOwnedBrowserProcesses) {
      throw new Error(`${label} owned browser process cleanup reached its ${maxOwnedBrowserProcesses}-PID bound: ${records.map(({ pid }) => pid).join(",")}`);
    }
    // The parent browser owns the renderer tree. Prefer it so one exact
    // taskkill /T /F removes all descendants; if the parent already exited,
    // fall through to the remaining profile-matched processes.
    records.sort((left, right) => {
      const leftWorker = left.commandLine.includes("--type=") ? 1 : 0;
      const rightWorker = right.commandLine.includes("--type=") ? 1 : 0;
      return leftWorker - rightWorker || left.pid - right.pid;
    });
    const candidate = records.find((record) => !attemptedPids.has(record.pid));
    if (!candidate) {
      throw new Error(`${label} owned browser processes could not be drained: ${records.map(({ pid }) => pid).join(",")}`);
    }
    const result = await runTaskkillTree(
      candidate.pid,
      remainingCleanupTimeout(`${label} taskkill tree PID ${candidate.pid}`, deadlineMs, cleanupTimeoutsMs.taskkill),
    );
    attemptedPids.add(candidate.pid);
    if (result.status !== 0 || result.signal !== null) {
      const stillOwned = await getOwnedBrowserProcesses(
        remainingCleanupTimeout(`${label} owned browser post-kill query`, deadlineMs, cleanupTimeoutsMs.processQuery),
      );
      if (stillOwned.some(({ pid }) => pid === candidate.pid)) {
        throw new Error(`${label} owned PID ${candidate.pid} termination failed (status=${result.status}, signal=${result.signal})`);
      }
    }
    records = await getOwnedBrowserProcesses(
      remainingCleanupTimeout(`${label} owned browser drain query`, deadlineMs, cleanupTimeoutsMs.processQuery),
    );
  }
};

const stopChild = async (child, label, { inspectOwnedBrowser = false, timeoutMs = cleanupTimeoutsMs.browserTree } = {}) => {
  if (!child) return;
  const deadlineMs = monotonicNowMs() + timeoutMs;
  const childExited = childHasExited(child);
  if (!child.pid) {
    if (childExited && !inspectOwnedBrowser) return;
    throw new Error(`${label} has no spawned PID`);
  }
  const failures = [];
  if (process.platform === "win32") {
    // A detached Chromium parent may have exited while its profile-matched
    // descendants remain. The exact spawned PID is always safe while alive;
    // when it has exited, profile ownership is verified before taskkill /T.
    let shouldTaskkillSpawnedPid = !childExited;
    if (childExited && inspectOwnedBrowser) {
      shouldTaskkillSpawnedPid = (await getOwnedBrowserProcesses(
        remainingCleanupTimeout(`${label} owned browser identity query`, deadlineMs, cleanupTimeoutsMs.processQuery),
      )).some(({ pid }) => pid === child.pid);
    }
    if (shouldTaskkillSpawnedPid) {
      try {
        const result = await runTaskkillTree(
          child.pid,
          remainingCleanupTimeout(`${label} taskkill tree PID ${child.pid}`, deadlineMs, cleanupTimeoutsMs.taskkill),
        );
        if ((result.status !== 0 || result.signal !== null) && !childHasExited(child) && !inspectOwnedBrowser) {
          failures.push(`${label} tree termination failed (status=${result.status}, signal=${result.signal})`);
        }
      } catch (error) {
        failures.push(error instanceof Error ? error.message : String(error));
      }
    }
  } else {
    if (childExited) return;
    try {
      await runBoundedCleanup(`${label} process-group termination`, () => {
        try {
          process.kill(-child.pid, "SIGTERM");
        } catch (error) {
          if (error?.code !== "ESRCH") throw error;
        }
      }, cleanupTimeoutsMs.taskkill);
    } catch (error) {
      failures.push(error instanceof Error ? error.message : String(error));
    }
  }
  if (!childExited) {
    try {
      await waitForChildExit(
        child,
        label,
        remainingCleanupTimeout(`${label} child exit`, deadlineMs, cleanupTimeoutsMs.childExit),
      );
    } catch (error) {
      failures.push(error instanceof Error ? error.message : String(error));
    }
  }
  if (inspectOwnedBrowser) {
    try {
      await terminateOwnedBrowserProcesses(label, deadlineMs);
    } catch (error) {
      failures.push(error instanceof Error ? error.message : String(error));
    }
  }
  if (failures.length > 0) throw new Error(`${label} cleanup failed: ${failures.join("; ")}`);
};

const tcpPortIsHeld = (port) => new Promise((resolveHeld) => {
  const socket = createTcpConnection({ host, port });
  let settled = false;
  const finish = (held) => {
    if (settled) return;
    settled = true;
    socket.destroy();
    resolveHeld(held);
  };
  socket.once("connect", () => finish(true));
  socket.once("error", (error) => {
    if (["ECONNREFUSED", "EHOSTUNREACH", "ENETUNREACH"].includes(error?.code)) {
      finish(false);
      return;
    }
    finish(true);
  });
  socket.setTimeout(500, () => finish(true));
});

const waitForEndpointRelease = async (port, label, required = false) => {
  if (!Number.isInteger(port)) {
    if (required) throw new Error(`${label} endpoint identity was never established`);
    return;
  }
  await waitFor(async () => !(await tcpPortIsHeld(port)), `${label} TCP endpoint release`, 5_000);
};

class CdpClient {
  constructor(url) {
    this.socket = new WebSocket(url);
    this.nextId = 0;
    this.closePromise = null;
    this.pending = new Map();
    this.runtimeExceptions = [];
    this.runtimeConsoleErrors = [];
    this.runtimeConsoleWarnings = [];
    this.logErrors = [];
    this.logWarnings = [];
    this.harnessErrors = [];
    this.socket.addEventListener("close", () => {
      this.rejectPending(new Error("CDP socket closed before a response was received"));
    });
    this.socket.onmessage = (event) => {
      const message = JSON.parse(event.data);
      if (!message.id) {
        this.recordDiagnosticEvent(message);
        return;
      }
      const pending = this.pending.get(message.id);
      if (!pending) return;
      this.pending.delete(message.id);
      if (message.error) pending.reject(new Error(message.error.message));
      else pending.resolve(message.result);
    };
  }

  recordDiagnosticEvent(message) {
    const stackFrames = (stackTrace) => (stackTrace?.callFrames ?? []).map((frame) => ({
      functionName: frame.functionName ?? "",
      url: frame.url ?? "",
      lineNumber: frame.lineNumber ?? null,
      columnNumber: frame.columnNumber ?? null,
    }));
    if (message.method === "Fetch.requestPaused") {
      const requestId = message.params?.requestId;
      const requestUrl = message.params?.request?.url ?? "";
      const faviconRequest = requestUrl === faviconUrl;
      const command = faviconRequest ? "Fetch.fulfillRequest" : "Fetch.continueRequest";
      const params = faviconRequest ? { requestId, responseCode: 204, responsePhrase: "No Content" } : { requestId };
      if (!faviconRequest) {
        this.harnessErrors.push({ command, requestUrl, message: "Unexpected Fetch.requestPaused URL" });
      }
      void this.send(command, params).catch((error) => {
        this.harnessErrors.push({ command, requestUrl, message: error instanceof Error ? error.message : String(error) });
      });
      return;
    }
    if (message.method === "Runtime.exceptionThrown") {
      const details = message.params?.exceptionDetails;
      this.runtimeExceptions.push({
        timestamp: message.params?.timestamp ?? null,
        text: details?.text ?? "",
        description: details?.exception?.description ?? "",
        url: details?.url ?? "",
        lineNumber: details?.lineNumber ?? null,
        columnNumber: details?.columnNumber ?? null,
        stack: stackFrames(details?.stackTrace),
      });
      return;
    }
    if (message.method === "Runtime.consoleAPICalled") {
      const type = message.params?.type ?? "";
      if (type !== "error" && type !== "warning") return;
      const entry = {
        timestamp: message.params?.timestamp ?? null,
        type,
        context: message.params?.context ?? "",
        stack: stackFrames(message.params?.stackTrace),
        args: (message.params?.args ?? []).map((argument) => (
          Object.hasOwn(argument ?? {}, "value")
            ? argument.value
            : argument?.unserializableValue ?? argument?.description ?? argument?.type ?? ""
        )),
      };
      if (type === "error") this.runtimeConsoleErrors.push(entry);
      else this.runtimeConsoleWarnings.push(entry);
      return;
    }
    if (message.method === "Log.entryAdded") {
      const source = message.params?.entry;
      if (source?.level !== "error" && source?.level !== "warning") return;
      const entry = {
        timestamp: source.timestamp ?? null,
        level: source.level,
        source: source.source ?? "",
        text: source.text ?? "",
        url: source.url ?? "",
        lineNumber: source.lineNumber ?? null,
        stack: stackFrames(source.stackTrace),
      };
      if (source.level === "error") this.logErrors.push(entry);
      else this.logWarnings.push(entry);
    }
  }

  diagnosticCursor() {
    return {
      runtimeExceptions: this.runtimeExceptions.length,
      runtimeConsoleErrors: this.runtimeConsoleErrors.length,
      runtimeConsoleWarnings: this.runtimeConsoleWarnings.length,
      logErrors: this.logErrors.length,
      logWarnings: this.logWarnings.length,
      harnessErrors: this.harnessErrors.length,
    };
  }

  diagnosticsSince(cursor) {
    return {
      runtimeExceptions: this.runtimeExceptions.slice(cursor.runtimeExceptions),
      runtimeConsoleErrors: this.runtimeConsoleErrors.slice(cursor.runtimeConsoleErrors),
      runtimeConsoleWarnings: this.runtimeConsoleWarnings.slice(cursor.runtimeConsoleWarnings),
      logErrors: this.logErrors.slice(cursor.logErrors),
      logWarnings: this.logWarnings.slice(cursor.logWarnings),
      harnessErrors: this.harnessErrors.slice(cursor.harnessErrors),
    };
  }

  async ready(timeoutMs = cdpTimeoutsMs.ready) {
    if (this.socket.readyState === WebSocket.OPEN) return;
    let cancelReady = () => {};
    await runWithMonotonicDeadline("CDP socket ready", () => new Promise((resolveReady, rejectReady) => {
      let settled = false;
      const finish = (error) => {
        if (settled) return;
        settled = true;
        this.socket.removeEventListener("open", onOpen);
        this.socket.removeEventListener("error", onError);
        this.socket.removeEventListener("close", onClose);
        if (error) rejectReady(error);
        else resolveReady();
      };
      const onOpen = () => finish();
      const onError = () => finish(new Error("CDP socket failed while connecting"));
      const onClose = () => finish(new Error("CDP socket closed while connecting"));
      cancelReady = () => finish(new Error("CDP socket readiness wait canceled at its deadline"));
      this.socket.addEventListener("open", onOpen, { once: true });
      this.socket.addEventListener("error", onError, { once: true });
      this.socket.addEventListener("close", onClose, { once: true });
      if (this.socket.readyState === WebSocket.OPEN) finish();
      else if (this.socket.readyState === WebSocket.CLOSED) finish(new Error("CDP socket is already closed"));
    }), timeoutMs, () => cancelReady());
  }

  rejectPending(error) {
    for (const [id, pending] of this.pending) {
      this.pending.delete(id);
      pending.reject(error);
    }
  }

  send(method, params = {}, timeoutMs = cdpTimeoutsMs.request) {
    assert.ok(Number.isFinite(timeoutMs) && timeoutMs > 0, `CDP ${method} request timeout must be positive`);
    const id = ++this.nextId;
    let rejectTimedOutRequest = () => {};
    const requestPromise = new Promise((resolveSend, rejectSend) => {
      rejectTimedOutRequest = (error) => {
        const pending = this.pending.get(id);
        if (!pending) return;
        this.pending.delete(id);
        pending.reject(error);
      };
      if (this.socket.readyState !== WebSocket.OPEN) {
        rejectSend(new Error(`CDP ${method} request ${id} cannot send while socket state is ${this.socket.readyState}`));
        return;
      }
      this.pending.set(id, { resolve: resolveSend, reject: rejectSend });
      try {
        this.socket.send(JSON.stringify({ id, method, params }));
      } catch (error) {
        rejectTimedOutRequest(error);
      }
    });
    return runWithMonotonicDeadline(
      `CDP ${method} request ${id}`,
      () => requestPromise,
      timeoutMs,
      () => rejectTimedOutRequest(new Error(`CDP ${method} request ${id} timed out`)),
    );
  }

  close() {
    if (this.closePromise) return this.closePromise;
    if (this.socket.readyState === WebSocket.CLOSED) return Promise.resolve();
    this.closePromise = new Promise((resolveClose, rejectClose) => {
      let settled = false;
      const finish = (error) => {
        if (settled) return;
        settled = true;
        this.socket.removeEventListener("close", onClose);
        if (error) rejectClose(error);
        else resolveClose();
      };
      const onClose = () => finish();
      this.socket.addEventListener("close", onClose, { once: true });
      try {
        if (this.socket.readyState !== WebSocket.CLOSING) this.socket.close();
        if (this.socket.readyState === WebSocket.CLOSED) finish();
      } catch (error) {
        finish(error);
      }
    });
    return this.closePromise;
  }
}

const evaluate = async (client, expression, timeoutMs = cdpTimeoutsMs.evaluate) => {
  const result = await client.send("Runtime.evaluate", {
    expression,
    awaitPromise: true,
    returnByValue: true,
  }, timeoutMs);
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
  await client.send("Input.dispatchKeyEvent", { type: "keyDown", key: "Escape", code: "Escape" }, cdpTimeoutsMs.request);
  await client.send("Input.dispatchKeyEvent", { type: "keyUp", key: "Escape", code: "Escape" }, cdpTimeoutsMs.request);
};

const pressTab = async (client) => {
  await client.send("Input.dispatchKeyEvent", { type: "keyDown", key: "Tab", code: "Tab" }, cdpTimeoutsMs.request);
  await client.send("Input.dispatchKeyEvent", { type: "keyUp", key: "Tab", code: "Tab" }, cdpTimeoutsMs.request);
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
  const directPerformanceEditor = popup.querySelector(':scope > .timelinePerformanceEditor');
  const directPerformanceEditorRect = directPerformanceEditor instanceof HTMLElement
    ? directPerformanceEditor.getBoundingClientRect()
    : null;
  const nestedHorizontalRegions = [
    ['bank', '.timelineBankBody'],
    ['cueAudio', '.timelineCueAudioEditorBody'],
    ['phases', '.timelinePhaseEditorBody'],
  ].map(([name, regionSelector]) => {
    const region = directPerformanceEditor?.querySelector(regionSelector);
    const regionRect = region instanceof HTMLElement ? region.getBoundingClientRect() : null;
    return {
      name,
      rect: regionRect ? rectArray(regionRect) : null,
      clientWidth: region instanceof HTMLElement ? region.clientWidth : 0,
      scrollWidth: region instanceof HTMLElement ? region.scrollWidth : 0,
    };
  });
  const layout = popup.closest('.layout') ?? document.querySelector('.layout');
  const layoutRect = layout?.getBoundingClientRect() ?? null;
  const within = (inner, outer) => Boolean(
    outer && inner.left >= outer.left - 1 && inner.top >= outer.top - 1 &&
    inner.right <= outer.right + 1 && inner.bottom <= outer.bottom + 1
  );
  const horizontallyWithin = (inner, outer) => Boolean(
    inner && outer && inner.left >= outer.left - 1 && inner.right <= outer.right + 1
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
    popupClientWidth: popup.clientWidth,
    popupScrollWidth: popup.scrollWidth,
    layout: layoutRect ? rectArray(layoutRect) : null,
    target: rectArray(targetRect),
    targetLabel: target.getAttribute('aria-label') ?? target.textContent?.trim().slice(0, 64) ?? target.tagName,
    targetHeight: targetRect.height,
    popupInsideLayout: layoutRect ? within(popupRect, layoutRect) : false,
    popupInsideViewport: popupRect.left >= -1 && popupRect.top >= -1 && popupRect.right <= innerWidth + 1 && popupRect.bottom <= innerHeight + 1,
    targetInsidePopup: within(targetRect, popupRect),
    directPerformanceEditor: directPerformanceEditorRect ? rectArray(directPerformanceEditorRect) : null,
    directPerformanceEditorWidth: directPerformanceEditorRect?.width ?? 0,
    directPerformanceEditorInsidePopupHorizontally: directPerformanceEditorRect
      ? horizontallyWithin(directPerformanceEditorRect, popupRect)
      : null,
    nestedHorizontalRegions,
    hit: hit === target || target.contains(hit),
    point,
    scrollTop: popup.scrollTop,
    scrollHeight: popup.scrollHeight,
    clientHeight: popup.clientHeight,
  };
})()`);

const exerciseTimelineNestedRegionTargets = async (client) => evaluate(client, `(async () => {
  const popup = document.querySelector('.timelineToolsDisclosure[open] .timelineToolsDisclosurePanel');
  const editor = popup?.querySelector(':scope > [data-timeline-performance-editor]');
  if (!(popup instanceof HTMLElement) || !(editor instanceof HTMLElement)) return null;
  const rectArray = (rect) => [rect.x, rect.y, rect.width, rect.height, rect.right, rect.bottom];
  const within = (inner, outer) => Boolean(
    inner && outer && inner.left >= outer.left - 1 && inner.top >= outer.top - 1 &&
    inner.right <= outer.right + 1 && inner.bottom <= outer.bottom + 1
  );
  const enabledVisibleFocusable = (element) => {
    if (!(element instanceof HTMLElement)) return false;
    const elementRect = element.getBoundingClientRect();
    const style = getComputedStyle(element);
    return elementRect.width > 0 && elementRect.height > 0 &&
      style.display !== 'none' && style.visibility !== 'hidden' &&
      !element.matches(':disabled') && element.getAttribute('aria-disabled') !== 'true' &&
      element.tabIndex >= 0 && !element.closest('details:not([open]), [inert], [aria-hidden="true"]');
  };
  const specs = [
    ['bank', '[data-timeline-bank]', '.timelineBankBody'],
    ['cueAudio', '[data-timeline-cue-audio-editor]', '.timelineCueAudioEditorBody'],
    ['phases', '[data-timeline-phase-editor]', '.timelinePhaseEditorBody'],
  ];
  for (const [, detailsSelector] of specs) {
    const details = editor.querySelector(detailsSelector);
    if (details instanceof HTMLDetailsElement) details.open = true;
  }
  await new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame)));
  const proveTarget = async (region, target, role) => {
    target.scrollIntoView({ block: 'nearest', inline: 'nearest' });
    await new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame)));
    const popupRect = popup.getBoundingClientRect();
    const regionRect = region.getBoundingClientRect();
    const targetRect = target.getBoundingClientRect();
    const point = { x: targetRect.left + targetRect.width / 2, y: targetRect.top + targetRect.height / 2 };
    const hit = document.elementFromPoint(point.x, point.y);
    return {
      role,
      label: target.getAttribute('aria-label') ?? target.textContent?.trim().slice(0, 64) ?? target.tagName,
      target: rectArray(targetRect),
      region: rectArray(regionRect),
      popup: rectArray(popupRect),
      popupInsideViewport: popupRect.left >= -1 && popupRect.top >= -1 && popupRect.right <= innerWidth + 1 && popupRect.bottom <= innerHeight + 1,
      targetInsideRegion: within(targetRect, regionRect),
      targetInsidePopup: within(targetRect, popupRect),
      targetInsideViewport: targetRect.left >= -1 && targetRect.top >= -1 && targetRect.right <= innerWidth + 1 && targetRect.bottom <= innerHeight + 1,
      hit: hit === target || target.contains(hit),
      point,
    };
  };
  const regions = [];
  for (const [name, detailsSelector, regionSelector] of specs) {
    const details = editor.querySelector(detailsSelector);
    const region = details?.querySelector(regionSelector);
    const controls = region instanceof HTMLElement
      ? [...new Set(region.querySelectorAll('button, input, select, textarea, summary, [role="button"], [tabindex]'))].filter(enabledVisibleFocusable)
      : [];
    const targets = [];
    if (region instanceof HTMLElement && controls[0]) {
      targets.push(await proveTarget(region, controls[0], 'representative'));
    }
    if (region instanceof HTMLElement && controls.at(-1)) {
      targets.push(await proveTarget(region, controls.at(-1), 'terminal'));
    }
    const regionRect = region instanceof HTMLElement ? region.getBoundingClientRect() : null;
    regions.push({
      name,
      detailsOpen: details instanceof HTMLDetailsElement && details.open,
      region: regionRect ? rectArray(regionRect) : null,
      enabledVisibleFocusableCount: controls.length,
      targets,
    });
  }
  return { regions };
})()`);

const rectArray = (rect) => [rect.x, rect.y, rect.width, rect.height, rect.right, rect.bottom];

const capture = async (client, name) => {
  const result = await client.send("Page.captureScreenshot", {
    format: "png",
    captureBeyondViewport: false,
  }, cdpTimeoutsMs.screenshot);
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
  const clickPlacementDetails = shelf?.querySelector('[data-timeline-source-click-placement]');
  const toolsToolbarGroups = [
    '.timelineToolsScrubGroup',
    '.timelineViewportToolbar',
    '.timelineDirectToolbar',
  ].map((selector) => {
    const element = toolsPanel?.querySelector(':scope > ' + selector);
    return { selector, rect: visible(element) ? rect(element) : null };
  });
  const toolsPanelRect = rect(toolsPanel);
  const toolsToolbarGroupsInsidePanel = Boolean(
    toolsPanelRect
      && toolsToolbarGroups.every(({ rect: groupRect }) => (
        groupRect
          && groupRect[0] >= toolsPanelRect[0] - 1
          && groupRect[4] <= toolsPanelRect[4] + 1
      )),
  );
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
    shelfCompactTopAligned: Boolean(
      shelf && context &&
      shelf.getBoundingClientRect().top >= context.getBoundingClientRect().top - 1 &&
      shelf.getBoundingClientRect().bottom <= context.getBoundingClientRect().bottom + 1 &&
      shelf.getBoundingClientRect().height < context.getBoundingClientRect().height - 4,
    ),
    clickPlacementDisclosure: {
      present: clickPlacementDetails instanceof HTMLDetailsElement,
      closed: clickPlacementDetails instanceof HTMLDetailsElement && !clickPlacementDetails.open,
      summaryHeight: rect(clickPlacementDetails?.querySelector(':scope > summary'))?.[3] ?? 0,
    },
    toolsOpen: tools instanceof HTMLDetailsElement && tools.open,
    toolsPanel: toolsPanelRect,
    toolsPanelClientWidth: toolsPanel instanceof HTMLElement ? toolsPanel.clientWidth : 0,
    toolsPanelScrollWidth: toolsPanel instanceof HTMLElement ? toolsPanel.scrollWidth : 0,
    toolsToolbarGroups,
    toolsToolbarGroupsInsidePanel,
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

const diagnosticCounts = (diagnostics) => ({
  runtimeExceptions: diagnostics.runtimeExceptions.length,
  runtimeConsoleErrors: diagnostics.runtimeConsoleErrors.length,
  logErrors: diagnostics.logErrors.length,
  runtimeConsoleWarnings: diagnostics.runtimeConsoleWarnings.length,
  logWarnings: diagnostics.logWarnings.length,
  harnessErrors: diagnostics.harnessErrors.length,
});

const assertNoCdpErrors = (diagnostics, label) => {
  assert.deepEqual(
    {
      runtimeExceptions: diagnostics.runtimeExceptions,
      runtimeConsoleErrors: diagnostics.runtimeConsoleErrors,
      logErrors: diagnostics.logErrors,
      harnessErrors: diagnostics.harnessErrors,
    },
    { runtimeExceptions: [], runtimeConsoleErrors: [], logErrors: [], harnessErrors: [] },
    `${label} has zero CDP runtime exceptions, console errors, Log.entryAdded errors, and harness errors`,
  );
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
let gateFailure;
let browserSpawned = false;
try {
  // Keep the deadline contract itself deterministic and exercised before any
  // browser or Vite resource is acquired. This catches regressions in the
  // teardown guard without requiring a viewport or external process.
  const cleanupSelfTestResult = await runBoundedCleanup(
    "cleanup helper resolve self-test",
    () => Promise.resolve("resolved"),
    100,
  );
  assert.equal(cleanupSelfTestResult, "resolved", "bounded cleanup resolves a completed operation");
  await assert.rejects(
    runBoundedCleanup("cleanup helper timeout self-test", () => new Promise(() => {}), 25),
    /exceeded monotonic cleanup deadline of 25 ms/,
    "bounded cleanup rejects an operation that exceeds its deadline",
  );
  await assert.rejects(
    waitFor(() => new Promise(() => {}), "waitFor check timeout self-test", 25, 10),
    /Timed out waiting for waitFor check timeout self-test/,
    "waitFor bounds an unresponsive check as well as its polling window",
  );
  if (cleanupSelfTestOnly) throw cleanupSelfTestComplete;

  const browserPath = browserCandidates.find((candidate) => existsSync(candidate));
  assert.ok(browserPath, "Chrome or Edge is required for the Control upper workspace browser gate");
  await assertMainWindowMinimum();
  await mkdir(screenshotDir, { recursive: true });
  vite = await createViteServer({
    root: appRoot,
    clearScreen: false,
    logLevel: "silent",
    server: {
      host,
      port: 0,
      // Port 0 requests an ephemeral fixture port. Keep fallback enabled if
      // the app config's default 5173 is occupied by an unrelated dev server.
      strictPort: false,
    },
  });
  await vite.listen();
  const viteAddress = vite.httpServer?.address();
  assert.ok(viteAddress && typeof viteAddress === "object", "Vite fixture server did not expose its owned TCP address");
  vitePort = viteAddress.port;
  assert.ok(Number.isInteger(vitePort) && vitePort >= 1 && vitePort <= 65_535, "Vite fixture server exposed an invalid owned TCP port");
  baseUrl = `http://${host}:${vitePort}/?syndocalViewportFixture=timeline-layered`;
  faviconUrl = `http://${host}:${vitePort}/favicon.ico`;
  await waitFor(async () => {
    try {
      return (await fetchWithDeadline(baseUrl)).ok;
    } catch {
      return false;
    }
  }, "Vite fixture server");
  profileDir = await mkdtemp(join(tmpdir(), browserProfilePrefix));
  browser = spawn(browserPath, [
    "--headless=new",
    "--disable-gpu",
    "--no-first-run",
    "--no-default-browser-check",
    "--remote-debugging-port=0",
    `--user-data-dir=${profileDir}`,
    "about:blank",
  ], { stdio: "ignore", windowsHide: true, detached: true });
  browserSpawned = Boolean(browser.pid);
  browser.once("error", (error) => { browser.spawnError = error; });
  const activePortPath = join(profileDir, "DevToolsActivePort");
  const devToolsEndpoint = await waitFor(async () => {
    assertChildAlive(browser, "headless browser");
    if (!existsSync(activePortPath)) return null;
    const [portText, browserPath] = (await readFile(activePortPath, "utf8")).trim().split(/\r?\n/);
    const port = Number(portText);
    if (!Number.isInteger(port) || port < 1 || port > 65_535) {
      throw new Error(`Invalid DevToolsActivePort value: ${JSON.stringify(portText)}`);
    }
    if (!/^\/devtools\/browser\/[0-9a-f-]+$/i.test(browserPath ?? "")) {
      throw new Error(`Invalid DevToolsActivePort browser identity: ${JSON.stringify(browserPath)}`);
    }
    return { port, browserPath };
  }, "headless browser DevToolsActivePort");
  cdpPort = devToolsEndpoint.port;
  const browserVersion = await waitFor(async () => {
    assertChildAlive(browser, "headless browser");
    try {
      const response = await fetchWithDeadline(`http://${host}:${cdpPort}/json/version`);
      return response.ok ? response.json() : null;
    } catch {
      return null;
    }
  }, "headless browser CDP identity");
  const browserWebSocketUrl = new URL(browserVersion.webSocketDebuggerUrl);
  assert.equal(browserWebSocketUrl.protocol, "ws:", "CDP browser endpoint must use ws");
  assert.equal(browserWebSocketUrl.hostname, host, "CDP browser endpoint must remain on the checker loopback host");
  assert.equal(Number(browserWebSocketUrl.port), cdpPort, "CDP browser endpoint port must match DevToolsActivePort");
  assert.equal(browserWebSocketUrl.pathname, devToolsEndpoint.browserPath, "CDP browser endpoint identity must match the exact spawned profile");
  assert.equal(browserWebSocketUrl.username, "", "CDP browser endpoint must not contain credentials");
  assert.equal(browserWebSocketUrl.password, "", "CDP browser endpoint must not contain credentials");
  assert.equal(browserWebSocketUrl.search, "", "CDP browser endpoint must not contain a query");
  assert.equal(browserWebSocketUrl.hash, "", "CDP browser endpoint must not contain a fragment");
  const target = await waitFor(async () => {
    assertChildAlive(browser, "headless browser");
    try {
      const response = await fetchWithDeadline(
        `http://${host}:${cdpPort}/json/new?${encodeURIComponent("about:blank")}`,
        { method: "PUT" },
      );
      return response.ok ? response.json() : null;
    } catch {
      return null;
    }
  }, "headless browser CDP target");
  const targetWebSocketUrl = new URL(target.webSocketDebuggerUrl);
  assert.equal(targetWebSocketUrl.protocol, "ws:", "CDP page endpoint must use ws");
  assert.equal(targetWebSocketUrl.hostname, host, "CDP page endpoint must remain on the checker loopback host");
  assert.equal(Number(targetWebSocketUrl.port), cdpPort, "CDP page endpoint port must match the exact spawned browser");
  assert.match(targetWebSocketUrl.pathname, /^\/devtools\/page\/[0-9a-f-]+$/i, "CDP page endpoint must expose an exact page identity");
  assert.equal(targetWebSocketUrl.username, "", "CDP page endpoint must not contain credentials");
  assert.equal(targetWebSocketUrl.password, "", "CDP page endpoint must not contain credentials");
  assert.equal(targetWebSocketUrl.search, "", "CDP page endpoint must not contain a query");
  assert.equal(targetWebSocketUrl.hash, "", "CDP page endpoint must not contain a fragment");
  const pinnedTargetWebSocketUrl = `ws://${host}:${cdpPort}${targetWebSocketUrl.pathname}`;
  client = new CdpClient(pinnedTargetWebSocketUrl);
  await client.ready(cdpTimeoutsMs.ready);
  const diagnosticsOrigin = client.diagnosticCursor();
  await client.send("Page.enable", {}, cdpTimeoutsMs.request);
  await client.send("Runtime.enable", {}, cdpTimeoutsMs.request);
  await client.send("Log.enable", {}, cdpTimeoutsMs.request);
  await client.send("Fetch.enable", { patterns: [{ urlPattern: faviconUrl, requestStage: "Request" }] }, cdpTimeoutsMs.request);

  for (const viewport of viewports) {
    const viewportDiagnosticsCursor = client.diagnosticCursor();
    await client.send("Emulation.setDeviceMetricsOverride", { ...viewport, deviceScaleFactor: 1, mobile: false }, cdpTimeoutsMs.request);
    await client.send("Page.navigate", { url: baseUrl }, cdpTimeoutsMs.navigate);
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
    assert.equal(timeline.shelfCompactTopAligned, true, "Sources shelf is compact and top-aligned in the lower context pane");
    assert.ok(rectHeight(timeline.shelfHeader) >= 32 && timeline.shelfHeading?.[3] > 0, "Sources header is readable and not clipped");
    assert.ok(timeline.sourceCardCount > 0 && (timeline.shelfBodyOverflowY === "auto" || timeline.shelfBodyOverflowY === "scroll"), "Sources body owns internal scrolling and mounts source cards");
    assert.equal(timeline.shelfOverflowY, "hidden", "Sources shelf has no outer scrollport");
    assert.equal(timeline.shelfOuterScroll, 0, "Sources shelf outer scroll is zero");
    assert.equal(timeline.clickPlacementDisclosure.present, true, "Sources keep click-placement lane selectors in an accessible disclosure");
    assert.equal(timeline.clickPlacementDisclosure.closed, true, "Click-placement lane selectors are collapsed by default so DnD stays primary");
    assert.ok(timeline.clickPlacementDisclosure.summaryHeight >= 24, "Click-placement disclosure keeps a usable summary target");
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
    assert.ok(toolsOpen.toolsPanelScrollWidth <= toolsOpen.toolsPanelClientWidth + 1, `Timeline Tools popup has no horizontal overflow at ${viewport.width}x${viewport.height}: ${JSON.stringify({ clientWidth: toolsOpen.toolsPanelClientWidth, scrollWidth: toolsOpen.toolsPanelScrollWidth })}`);
    assert.equal(toolsOpen.toolsToolbarGroupsInsidePanel, true, `Timeline Tools immediate toolbar groups remain inside the popup at ${viewport.width}x${viewport.height}: ${JSON.stringify({ panel: toolsOpen.toolsPanel, groups: toolsOpen.toolsToolbarGroups })}`);
    const nestedRegionTargets = await exerciseTimelineNestedRegionTargets(client);
    assert.deepEqual(nestedRegionTargets?.regions?.map(({ name }) => name), ['bank', 'cueAudio', 'phases'], `Timeline Tools exercises every nested target region at ${viewport.width}x${viewport.height}`);
    for (const region of nestedRegionTargets?.regions ?? []) {
      assert.equal(region.detailsOpen, true, `Timeline ${region.name} details are open before reachability checks at ${viewport.width}x${viewport.height}: ${JSON.stringify(region)}`);
      assert.ok(region.region && region.enabledVisibleFocusableCount > 0, `Timeline ${region.name} has an enabled visible focusable control at ${viewport.width}x${viewport.height}: ${JSON.stringify(region)}`);
      assert.deepEqual(region.targets.map(({ role }) => role), ['representative', 'terminal'], `Timeline ${region.name} exercises representative and terminal controls at ${viewport.width}x${viewport.height}: ${JSON.stringify(region)}`);
      assert.equal(region.targets.every((target) => target.popupInsideViewport && target.targetInsideRegion && target.targetInsidePopup && target.targetInsideViewport && target.hit), true, `Timeline ${region.name} representative/terminal controls remain contained and hit-testable at ${viewport.width}x${viewport.height}: ${JSON.stringify(region)}`);
    }
    const toolsLast = await exercisePopupLastTarget(client, '.timelineToolsDisclosure[open] .timelineToolsDisclosurePanel');
    assert.ok(toolsLast?.popupInsideLayout && toolsLast.popupInsideViewport, `Timeline Tools popup remains inside its layout and viewport: ${JSON.stringify(toolsLast)}`);
    assert.ok(toolsLast?.targetInsidePopup && toolsLast.hit, `Timeline Tools last action remains scroll-reachable and hit-testable: ${JSON.stringify(toolsLast)}`);
    assert.ok((toolsLast?.targetHeight ?? 0) >= 24, `Timeline Tools last action keeps a usable control height: ${JSON.stringify(toolsLast)}`);
    assert.ok((toolsLast?.popupScrollWidth ?? Infinity) <= (toolsLast?.popupClientWidth ?? 0) + 1, `Timeline Tools popup has no horizontal overflow after nested Bank/Cue Audio/Phases open at ${viewport.width}x${viewport.height}: ${JSON.stringify(toolsLast)}`);
    assert.ok((toolsLast?.directPerformanceEditorWidth ?? 0) > 0, `Timeline Performance editor keeps a useful positive width after nested details open at ${viewport.width}x${viewport.height}: ${JSON.stringify(toolsLast)}`);
    assert.equal(toolsLast?.directPerformanceEditorInsidePopupHorizontally, true, `Timeline Performance editor remains horizontally inside the popup after nested details open at ${viewport.width}x${viewport.height}: ${JSON.stringify(toolsLast)}`);
    assert.deepEqual(toolsLast?.nestedHorizontalRegions?.map(({ name }) => name), ['bank', 'cueAudio', 'phases'], `Timeline Tools measures every nested horizontal region at ${viewport.width}x${viewport.height}`);
    assert.equal(toolsLast?.nestedHorizontalRegions?.every(({ clientWidth, scrollWidth }) => clientWidth > 0 && scrollWidth <= clientWidth + 1), true, `Timeline Bank/Cue Audio/Phases regions have no horizontal overflow after nesting at ${viewport.width}x${viewport.height}: ${JSON.stringify(toolsLast?.nestedHorizontalRegions)}`);
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
    assert.equal(await clickVisible(client, '.timelineToolsDisclosure > summary'), true, "open Timeline Tools from Live Mixer");
    const toolsReopened = await waitFor(async () => {
      const value = await measureTimeline(client);
      return value?.toolsOpen ? value : false;
    }, "Timeline Tools disclosure reopened from Live Mixer");
    assert.deepEqual({ toolsOpen: toolsReopened.toolsOpen, mixerOpen: toolsReopened.mixerOpen }, { toolsOpen: true, mixerOpen: false }, "Tools and Live Mixer disclosures are mutually exclusive in the reverse gate sequence");
    await evaluate(client, `(() => {
      document.querySelector('.groupLiveMixerDisclosure')?.removeAttribute('open');
      document.querySelector('.timelineToolsDisclosure')?.removeAttribute('open');
    })()`);
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
    const expandedDeepestTarget = await evaluate(client, `(async () => {
      const panel = document.querySelector('.timelineToolsDisclosure[open] .timelineToolsDisclosurePanel');
      if (!(panel instanceof HTMLElement)) return null;
      for (const details of panel.querySelectorAll('details')) details.open = true;
      await new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame)));
      const visible = (element) => {
        if (!(element instanceof HTMLElement)) return false;
        const rect = element.getBoundingClientRect();
        const style = getComputedStyle(element);
        return rect.width > 0 && rect.height > 0 && style.display !== 'none' && style.visibility !== 'hidden' && !element.matches(':disabled') && element.getAttribute('aria-disabled') !== 'true' && element.tabIndex >= 0 && !element.closest('details:not([open]), [inert], [aria-hidden="true"]');
      };
      const target = [...panel.querySelectorAll('button, input, select, textarea, summary, [role="button"], [tabindex]')].filter(visible).at(-1);
      if (!(target instanceof HTMLElement)) return null;
      const nestedEditor = target.closest('.timelineBankPanel, .timelineCueAudioEditor, .timelinePhaseEditor');
      const nestedRegion = target.closest('.timelineBankBody, .timelineCueAudioEditorBody, .timelinePhaseEditorBody');
      if (!(nestedEditor instanceof HTMLDetailsElement) || !(nestedRegion instanceof HTMLElement) || !panel.contains(nestedEditor)) return null;
      target.scrollIntoView({ block: 'nearest', inline: 'nearest' });
      await new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame)));
      const panelRect = panel.getBoundingClientRect();
      const nestedRegionRect = nestedRegion.getBoundingClientRect();
      const targetRect = target.getBoundingClientRect();
      const within = (inner, outer) => inner.left >= outer.left - 1 && inner.top >= outer.top - 1 && inner.right <= outer.right + 1 && inner.bottom <= outer.bottom + 1;
      const point = { x: targetRect.left + targetRect.width / 2, y: targetRect.top + targetRect.height / 2 };
      const hit = document.elementFromPoint(point.x, point.y);
      target.focus();
      const rectArray = (rect) => [rect.x, rect.y, rect.width, rect.height, rect.right, rect.bottom];
      return {
        nestedEditorOpen: nestedEditor.open,
        targetLabel: target.getAttribute('aria-label') ?? target.textContent?.trim().slice(0, 64) ?? target.tagName,
        target: rectArray(targetRect),
        nestedRegion: rectArray(nestedRegionRect),
        popup: rectArray(panelRect),
        popupInsideViewport: panelRect.left >= -1 && panelRect.top >= -1 && panelRect.right <= innerWidth + 1 && panelRect.bottom <= innerHeight + 1,
        targetInsideNestedRegion: within(targetRect, nestedRegionRect),
        targetInsidePopup: within(targetRect, panelRect),
        targetInsideViewport: targetRect.left >= -1 && targetRect.top >= -1 && targetRect.right <= innerWidth + 1 && targetRect.bottom <= innerHeight + 1,
        hit: hit === target || target.contains(hit),
        focused: document.activeElement === target,
        point,
      };
    })()`);
    assert.equal(expandedDeepestTarget?.focused, true, `Expanded Timeline Tools accepts focus on its deepest visible target inside the open popup: ${JSON.stringify(expandedDeepestTarget)}`);
    assert.equal(expandedDeepestTarget?.nestedEditorOpen, true, `Expanded Timeline deepest target belongs to an open nested editor: ${JSON.stringify(expandedDeepestTarget)}`);
    assert.ok(expandedDeepestTarget?.popupInsideViewport && expandedDeepestTarget.targetInsideNestedRegion && expandedDeepestTarget.targetInsidePopup && expandedDeepestTarget.targetInsideViewport, `Expanded Timeline deepest target remains inside its nested region, popup, and viewport after scrolling: ${JSON.stringify(expandedDeepestTarget)}`);
    assert.equal(expandedDeepestTarget?.hit, true, `Expanded Timeline deepest target remains reachable by elementFromPoint after scrolling: ${JSON.stringify(expandedDeepestTarget)}`);
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
    await evaluate(client, "new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame)))");
    await sleep(20);
    const viewportDiagnostics = client.diagnosticsSince(viewportDiagnosticsCursor);
    console.log(`${viewport.width}x${viewport.height} CDP diagnostics: ${JSON.stringify(diagnosticCounts(viewportDiagnostics))}`);
    assertNoCdpErrors(viewportDiagnostics, `${viewport.width}x${viewport.height}`);
  }
  await evaluate(client, "new Promise((resolveFrame) => requestAnimationFrame(() => requestAnimationFrame(resolveFrame)))");
  await sleep(20);
  const finalDiagnostics = client.diagnosticsSince(diagnosticsOrigin);
  console.log(`Final CDP diagnostics: ${JSON.stringify(diagnosticCounts(finalDiagnostics))}`);
  assertNoCdpErrors(finalDiagnostics, "Final cumulative gate");
} catch (error) {
  if (error !== cleanupSelfTestComplete) gateFailure = error;
} finally {
  const cleanupFailures = [];
  const recordCleanupFailure = (label, error) => {
    cleanupFailures.push(`${label}: ${error instanceof Error ? error.message : String(error)}`);
  };

  // Each step owns an exact resource and is attempted even when an earlier
  // step failed. runBoundedCleanup supplies a monotonic per-step deadline and
  // a checkpoint pair, preventing a failed close from hiding later teardown.
  const runCleanupStep = async (label, operation, timeoutMs) => {
    try {
      await runBoundedCleanup(label, operation, timeoutMs);
    } catch (error) {
      recordCleanupFailure(label, error);
    }
  };

  await runCleanupStep("CDP client close", () => client?.close(), cleanupTimeoutsMs.cdpClose);
  await runCleanupStep(
    "headless browser tree cleanup",
    () => stopChild(browser, "headless browser", { inspectOwnedBrowser: true }),
    cleanupTimeoutsMs.browserTree,
  );
  await runCleanupStep("Vite fixture server handle cleanup", () => vite?.close(), cleanupTimeoutsMs.viteClose);
  await runCleanupStep(
    "headless browser endpoint cleanup",
    () => waitForEndpointRelease(cdpPort, "headless browser", browserSpawned),
    cleanupTimeoutsMs.endpointRelease,
  );
  await runCleanupStep(
    "Vite fixture server endpoint cleanup",
    () => waitForEndpointRelease(vitePort, "Vite fixture server", Boolean(vite)),
    cleanupTimeoutsMs.endpointRelease,
  );
  if (profileDir) {
    await runCleanupStep("browser profile cleanup", async () => {
      let removed = false;
      for (let attempt = 0; attempt < 5; attempt += 1) {
        try {
          await runBoundedCleanup(
            `browser profile removal attempt ${attempt + 1}`,
            () => rm(profileDir, { recursive: true, force: true }),
            cleanupTimeoutsMs.profileAttempt,
          );
          removed = !existsSync(profileDir);
          if (removed) break;
        } catch (error) {
          // rm() has no cancellation primitive. A deadline means the original
          // promise may still own the profile, so never issue a second rm while
          // that first operation could still be running.
          if (error?.code === monotonicDeadlineExceededCode) throw error;
          if (attempt === 4) throw error;
          await sleep(250);
        }
      }
      if (!removed) throw new Error(`profile directory still exists: ${profileDir}`);
    }, cleanupTimeoutsMs.profile);
  }
  if (cleanupFailures.length > 0) {
    const cleanupError = new Error(`Test cleanup failed: ${cleanupFailures.join("; ")}`);
    if (gateFailure) {
      console.error(`${cleanupError.message}; preserving primary gate failure: ${gateFailure instanceof Error ? gateFailure.message : String(gateFailure)}`);
    } else {
      gateFailure = cleanupError;
    }
  }
}

if (gateFailure) throw gateFailure;
if (cleanupSelfTestOnly) {
  console.log("Control upper workspace cleanup deadline self-test passed");
} else {
  console.log(`Control upper workspace browser gate passed; screenshots=${screenshotDir}`);
}
