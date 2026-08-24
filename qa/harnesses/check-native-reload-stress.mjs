import { writeFile } from "node:fs/promises";

const optionValue = (name, fallback = null) => {
  const index = process.argv.indexOf(name);
  return index >= 0 ? process.argv[index + 1] ?? fallback : fallback;
};

const parsePositiveInteger = (name, fallback) => {
  const raw = optionValue(name, String(fallback));
  const value = Number(raw);
  if (!Number.isSafeInteger(value) || value <= 0) {
    throw new Error(`${name} must be a positive safe integer; received ${String(raw)}`);
  }
  return value;
};

const port = parsePositiveInteger("--port", 9333);
if (port > 65_535) throw new Error(`--port must be at most 65535; received ${port}`);
const count = parsePositiveInteger("--count", 100);
const checkpointEvery = parsePositiveInteger("--checkpoint-every", 10);
const settleMs = parsePositiveInteger("--settle-ms", 25);
const transportTimeoutMs = parsePositiveInteger("--transport-timeout-ms", 15_000);
const expectedTitle = optionValue("--title", "Syndocal");
const expectedUrl = optionValue("--url", "http://tauri.localhost/");
const outputPath = optionValue("--output");
const cdpBaseUrl = `http://127.0.0.1:${port}`;
const sleep = (milliseconds) => new Promise((resolve) => setTimeout(resolve, milliseconds));

const targetResponse = await fetch(`${cdpBaseUrl}/json/list`, {
  signal: AbortSignal.timeout(transportTimeoutMs),
});
if (!targetResponse.ok) {
  throw new Error(`CDP target discovery failed with HTTP ${targetResponse.status}`);
}
const targets = await targetResponse.json();
const pages = targets.filter((target) =>
  target.type === "page"
  && target.title === expectedTitle
  && target.url === expectedUrl
  && typeof target.webSocketDebuggerUrl === "string"
);
if (pages.length !== 1) {
  throw new Error(
    `Expected exactly one ${expectedTitle} CDP page at ${expectedUrl}; found ${pages.length}`,
  );
}

const page = pages[0];
const socket = new WebSocket(page.webSocketDebuggerUrl);
await new Promise((resolve, reject) => {
  const timer = setTimeout(
    () => reject(new Error(`Timed out opening the CDP WebSocket after ${transportTimeoutMs}ms`)),
    transportTimeoutMs,
  );
  socket.addEventListener("open", () => {
    clearTimeout(timer);
    resolve();
  }, { once: true });
  socket.addEventListener(
    "error",
    () => {
      clearTimeout(timer);
      reject(new Error("The CDP WebSocket could not be opened"));
    },
    { once: true },
  );
});

let nextMessageId = 1;
const pending = new Map();
const eventWaiters = new Map();
const eventCounts = new Map();
const exceptionCounts = new Map();
const consoleCounts = new Map();
const logCounts = new Map();

const valueOfRemoteObject = (value) =>
  value?.value ?? value?.unserializableValue ?? value?.description ?? value?.type ?? "unknown";

const normalizeConsoleIssue = (type, args) => {
  const normalizedArgs = args.map((value) => {
    const firstLine = String(value).split(/\r?\n/, 1)[0];
    return firstLine
      .replace(/callback id \d+/g, "callback id <id>")
      .replace(/\s+/g, " ")
      .trim();
  });
  return `${type} | ${normalizedArgs.join(" | ")}`.slice(0, 2_000);
};

const exceptionKey = (params) => {
  const details = params?.exceptionDetails;
  const exception = details?.exception;
  return [
    exception?.className ?? details?.text ?? "UnknownException",
    exception?.description ?? details?.text ?? "No description",
    details?.url ?? "",
    details?.lineNumber ?? "",
    details?.columnNumber ?? "",
  ].join(" | ");
};

const normalizeLogIssue = (entry) => [
  entry?.level ?? "unknown",
  String(entry?.text ?? "")
    .replace(/callback id \d+/g, "callback id <id>")
    .replace(/\s+/g, " ")
    .trim(),
  entry?.url ?? "",
  entry?.lineNumber ?? "",
].join(" | ").slice(0, 2_000);

socket.addEventListener("message", (event) => {
  const message = JSON.parse(String(event.data));
  if (message.id != null) {
    const slot = pending.get(message.id);
    if (!slot) return;
    pending.delete(message.id);
    if (message.error) slot.reject(new Error(JSON.stringify(message.error)));
    else slot.resolve(message.result);
    return;
  }

  eventCounts.set(message.method, (eventCounts.get(message.method) ?? 0) + 1);
  if (message.method === "Runtime.exceptionThrown") {
    const key = exceptionKey(message.params);
    exceptionCounts.set(key, (exceptionCounts.get(key) ?? 0) + 1);
  }
  if (message.method === "Runtime.consoleAPICalled") {
    const type = message.params?.type ?? "unknown";
    if (["warning", "error", "assert"].includes(type)) {
      const args = (message.params?.args ?? []).map(valueOfRemoteObject).map(String);
      const key = normalizeConsoleIssue(type, args);
      consoleCounts.set(key, (consoleCounts.get(key) ?? 0) + 1);
    }
  }
  if (message.method === "Log.entryAdded") {
    const level = message.params?.entry?.level ?? "unknown";
    if (["warning", "error"].includes(level)) {
      const key = normalizeLogIssue(message.params.entry);
      logCounts.set(key, (logCounts.get(key) ?? 0) + 1);
    }
  }

  const queue = eventWaiters.get(message.method);
  if (queue?.length) queue.shift()(message.params ?? {});
});

socket.addEventListener("close", () => {
  for (const { reject, timer } of pending.values()) {
    clearTimeout(timer);
    reject(new Error("The CDP WebSocket closed with a command pending"));
  }
  pending.clear();
});

const send = (method, params = {}, timeoutMs = transportTimeoutMs) => new Promise((resolve, reject) => {
  if (socket.readyState !== WebSocket.OPEN) {
    reject(new Error(`Cannot send ${method}: CDP WebSocket state is ${socket.readyState}`));
    return;
  }
  const id = nextMessageId++;
  const timer = setTimeout(() => {
    pending.delete(id);
    reject(new Error(`Timed out waiting for CDP command ${method} after ${timeoutMs}ms`));
  }, timeoutMs);
  pending.set(id, {
    resolve: (value) => {
      clearTimeout(timer);
      resolve(value);
    },
    reject: (error) => {
      clearTimeout(timer);
      reject(error);
    },
    timer,
  });
  try {
    socket.send(JSON.stringify({ id, method, params }));
  } catch (error) {
    pending.delete(id);
    clearTimeout(timer);
    reject(error);
  }
});

const waitForEvent = (method, timeoutMs = 15_000) => new Promise((resolve, reject) => {
  const queue = eventWaiters.get(method) ?? [];
  eventWaiters.set(method, queue);
  let settled = false;
  const done = (params) => {
    if (settled) return;
    settled = true;
    clearTimeout(timer);
    resolve(params);
  };
  queue.push(done);
  const timer = setTimeout(() => {
    if (settled) return;
    settled = true;
    const index = queue.indexOf(done);
    if (index >= 0) queue.splice(index, 1);
    reject(new Error(`Timed out waiting for ${method}`));
  }, timeoutMs);
});

const evaluate = async (expression) => {
  const result = await send("Runtime.evaluate", {
    expression,
    returnByValue: true,
    awaitPromise: true,
  });
  if (result.exceptionDetails) {
    throw new Error(`Runtime.evaluate failed: ${JSON.stringify(result.exceptionDetails)}`);
  }
  return result.result.value;
};

const probeExpression = `({
  timeOrigin: performance.timeOrigin,
  readyState: document.readyState,
  title: document.title,
  href: location.href,
  hasTauri: Boolean(window.__TAURI_INTERNALS__),
  rootHasBrand: (document.querySelector("#root")?.innerText || "").includes(${JSON.stringify(expectedTitle)}),
  statusReady: /準備完了|Ready/.test(document.body?.innerText || ""),
  navigationType: performance.getEntriesByType("navigation")[0]?.type || null
})`;

const waitForProbe = async ({ requireStatusReady, timeoutMs }) => {
  const deadline = Date.now() + timeoutMs;
  let probe = null;
  while (Date.now() < deadline) {
    try {
      probe = await evaluate(probeExpression);
      if (
        probe.readyState === "complete"
        && probe.title === expectedTitle
        && probe.href === expectedUrl
        && probe.hasTauri
        && probe.rootHasBrand
        && (!requireStatusReady || probe.statusReady)
      ) {
        return probe;
      }
    } catch {
      // The old JavaScript context may disappear between reload and probe.
    }
    await sleep(50);
  }
  throw new Error(`The reloaded document did not become ready: ${JSON.stringify(probe)}`);
};

const percentile = (sortedValues, ratio) =>
  sortedValues[Math.min(sortedValues.length - 1, Math.floor((sortedValues.length - 1) * ratio))];

const startedAt = new Date().toISOString();
await send("Page.enable");
await send("Runtime.enable");
await send("Log.enable");
await sleep(100);
const initial = await waitForProbe({ requireStatusReady: true, timeoutMs: 10_000 });
let previousTimeOrigin = initial.timeOrigin;
const timeOrigins = [];
const durationsMs = [];
const checkpoints = [];

for (let cycle = 1; cycle <= count; cycle += 1) {
  const started = performance.now();
  const loaded = waitForEvent("Page.loadEventFired");
  await send("Page.reload", { ignoreCache: true });
  await loaded;

  const checkpoint = cycle % checkpointEvery === 0 || cycle === count;
  const probe = await waitForProbe({
    requireStatusReady: checkpoint,
    timeoutMs: checkpoint ? 10_000 : 5_000,
  });
  if (!(probe.timeOrigin > previousTimeOrigin)) {
    throw new Error(
      `Document timeOrigin did not advance at cycle ${cycle}: ${probe.timeOrigin} <= ${previousTimeOrigin}`,
    );
  }
  if (probe.navigationType !== "reload") {
    throw new Error(`Navigation type was not reload at cycle ${cycle}: ${probe.navigationType}`);
  }

  previousTimeOrigin = probe.timeOrigin;
  timeOrigins.push(probe.timeOrigin);
  const durationMs = performance.now() - started;
  durationsMs.push(durationMs);
  if (checkpoint) {
    const snapshot = {
      cycle,
      statusReady: probe.statusReady,
      timeOrigin: probe.timeOrigin,
      durationMs: Number(durationMs.toFixed(1)),
    };
    checkpoints.push(snapshot);
    console.log(`CHECKPOINT ${JSON.stringify(snapshot)}`);
  }
  await sleep(settleMs);
}

const final = await waitForProbe({ requireStatusReady: true, timeoutMs: 10_000 });
await sleep(500);
const sortedDurations = [...durationsMs].sort((left, right) => left - right);
const exceptions = [...exceptionCounts.entries()].map(([message, occurrences]) => ({
  occurrences,
  message,
}));
const consoleIssues = [...consoleCounts.entries()].map(([message, occurrences]) => ({
  occurrences,
  message,
}));
const consoleErrors = consoleIssues.filter(({ message }) =>
  message.startsWith("error |") || message.startsWith("assert |")
);
const logIssues = [...logCounts.entries()].map(([message, occurrences]) => ({
  occurrences,
  message,
}));
const logErrors = logIssues.filter(({ message }) => message.startsWith("error |"));
const result = {
  startedAt,
  completedAt: new Date().toISOString(),
  cdp: {
    port,
    pageId: page.id,
    title: page.title,
    url: page.url,
  },
  count,
  checkpointEvery,
  settleMs,
  transportTimeoutMs,
  uniqueTimeOrigins: new Set(timeOrigins).size,
  final,
  durationMs: {
    min: Number(sortedDurations[0].toFixed(1)),
    p50: Number(percentile(sortedDurations, 0.5).toFixed(1)),
    p95: Number(percentile(sortedDurations, 0.95).toFixed(1)),
    max: Number(sortedDurations.at(-1).toFixed(1)),
    total: Number(durationsMs.reduce((sum, value) => sum + value, 0).toFixed(1)),
  },
  eventCounts: Object.fromEntries(
    [...eventCounts.entries()]
      .filter(([name]) => name.startsWith("Page.") || name.startsWith("Runtime."))
      .sort(([left], [right]) => left.localeCompare(right)),
  ),
  runtimeExceptions: exceptions,
  consoleIssues,
  logIssues,
  checkpoints,
  pass:
    timeOrigins.length === count
    && new Set(timeOrigins).size === count
    && exceptions.length === 0
    && consoleErrors.length === 0
    && logErrors.length === 0,
};

const serialized = `${JSON.stringify(result, null, 2)}\n`;
if (outputPath) await writeFile(outputPath, serialized, { encoding: "utf8", flag: "wx" });
console.log(`FINAL ${JSON.stringify(result)}`);
socket.close();
if (!result.pass) process.exitCode = 1;
