// Focused CDP profiler: measures what burns the main thread when a timeline
// scene-block marker is clicked under the scene-block-large fixture.
// Usage: node scripts/profile-marker-click.mjs [appUrl]
// Prints the top functions by self time from the V8 sampling profile.
import { spawn } from "node:child_process";
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const appUrl = process.argv[2] ?? "http://127.0.0.1:5178/?syndocalViewportFixture=scene-block-large";

const chromeCandidates = [
  process.env.CHROME_PATH,
  "C:/Program Files/Google/Chrome/Application/chrome.exe",
  "C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe",
].filter(Boolean);

import { existsSync } from "node:fs";
const chrome = chromeCandidates.find((candidate) => existsSync(candidate));
if (!chrome) throw new Error("No Chrome/Edge found; set CHROME_PATH");

const profileDir = mkdtempSync(join(tmpdir(), "syndocal-prof-"));
const port = 9333;
const child = spawn(chrome, [
  `--remote-debugging-port=${port}`,
  `--user-data-dir=${profileDir}`,
  "--headless=new",
  "--disable-gpu",
  "--no-first-run",
  "--window-size=1920,1080",
  "about:blank",
], { stdio: "ignore" });

const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

async function fetchJson(path) {
  const response = await fetch(`http://127.0.0.1:${port}${path}`);
  return response.json();
}

let ws;
let messageId = 0;
const pending = new Map();
function send(method, params = {}) {
  const id = ++messageId;
  return new Promise((resolve, reject) => {
    pending.set(id, { resolve, reject });
    ws.send(JSON.stringify({ id, method, params }));
  });
}

try {
  await sleep(1500);
  const targets = await fetchJson("/json/list");
  const page = targets.find((t) => t.type === "page");
  ws = new WebSocket(page.webSocketDebuggerUrl);
  await new Promise((resolve) => { ws.onopen = resolve; });
  ws.onmessage = (event) => {
    const message = JSON.parse(event.data);
    if (message.id && pending.has(message.id)) {
      const { resolve, reject } = pending.get(message.id);
      pending.delete(message.id);
      if (message.error) reject(new Error(message.error.message));
      else resolve(message.result);
    }
  };

  await send("Page.enable");
  await send("Runtime.enable");
  await send("Emulation.setDeviceMetricsOverride", { width: 1920, height: 1080, deviceScaleFactor: 1, mobile: false });
  await send("Page.navigate", { url: appUrl });
  await sleep(6000);

  // open Control > Timeline
  await send("Runtime.evaluate", { expression: `[...document.querySelectorAll('button')].find(b => b.textContent.trim() === 'Timeline')?.click()`, awaitPromise: false });
  await sleep(1500);
  const markerCheck = await send("Runtime.evaluate", { expression: `document.querySelectorAll('.timelineMarker').length` });
  console.log("markers:", markerCheck.result.value);

  await send("Profiler.enable");
  await send("Profiler.setSamplingInterval", { interval: 200 });
  await send("Profiler.start");
  const t0 = Date.now();
  await send("Runtime.evaluate", {
    expression: `document.querySelector('.timelineMarker.sceneBlock[data-timeline-event-id="493"]').dispatchEvent(new MouseEvent('click', { bubbles: true })); 'clicked'`,
    awaitPromise: false,
  });
  // wait until the main thread responds again (probe with cheap evals)
  let recovered = false;
  for (let attempt = 0; attempt < 120; attempt += 1) {
    const started = Date.now();
    try {
      await Promise.race([
        send("Runtime.evaluate", { expression: "1" }),
        sleep(2000).then(() => { throw new Error("probe-timeout"); }),
      ]);
      if (Date.now() - started < 500 && Date.now() - t0 > 1500) { recovered = true; break; }
    } catch { /* still busy */ }
  }
  const elapsedMs = Date.now() - t0;
  const { profile } = await send("Profiler.stop");
  console.log(`recovered=${recovered} elapsedMs=${elapsedMs}`);

  // aggregate self time per function
  const nodesById = new Map(profile.nodes.map((node) => [node.id, node]));
  const selfMicros = new Map();
  const samples = profile.samples ?? [];
  const deltas = profile.timeDeltas ?? [];
  for (let index = 0; index < samples.length; index += 1) {
    const node = nodesById.get(samples[index]);
    if (!node) continue;
    const key = `${node.callFrame.functionName || "(anonymous)"} @ ${node.callFrame.url.split("/").pop()}:${node.callFrame.lineNumber}`;
    selfMicros.set(key, (selfMicros.get(key) ?? 0) + (deltas[index] ?? 0));
  }
  const top = [...selfMicros.entries()].sort((a, b) => b[1] - a[1]).slice(0, 20);
  console.log("--- top self-time (ms) ---");
  for (const [key, micros] of top) {
    console.log(`${(micros / 1000).toFixed(0).padStart(7)} ms  ${key}`);
  }
} finally {
  try { ws?.close(); } catch {}
  child.kill();
  await sleep(500);
  try { rmSync(profileDir, { recursive: true, force: true }); } catch {}
}
