// Chrome trace around a single marker class flip: attributes the long task to
// style recalc vs layout vs other. Usage: node scripts/trace-marker-flip.mjs
import { spawn } from "node:child_process";
import { existsSync, mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const appUrl = process.argv[2] ?? "http://127.0.0.1:5178/?syndocalViewportFixture=scene-block-large";
const chrome = [
  process.env.CHROME_PATH,
  "C:/Program Files/Google/Chrome/Application/chrome.exe",
  "C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe",
].filter(Boolean).find((candidate) => existsSync(candidate));
if (!chrome) throw new Error("no chrome");

const dir = mkdtempSync(join(tmpdir(), "syndocal-trace-"));
const port = 9334;
const child = spawn(chrome, [
  `--remote-debugging-port=${port}`, `--user-data-dir=${dir}`,
  "--headless=new", "--disable-gpu", "--no-first-run", "--window-size=1920,1080", "about:blank",
], { stdio: "ignore" });
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

let ws; let id = 0; const pending = new Map(); const traceEvents = [];
function send(method, params = {}) {
  const mid = ++id;
  return new Promise((resolve, reject) => {
    pending.set(mid, { resolve, reject });
    ws.send(JSON.stringify({ id: mid, method, params }));
  });
}

try {
  await sleep(1500);
  const targets = await (await fetch(`http://127.0.0.1:${port}/json/list`)).json();
  const page = targets.find((t) => t.type === "page");
  ws = new WebSocket(page.webSocketDebuggerUrl);
  await new Promise((resolve) => { ws.onopen = resolve; });
  let collectDone;
  const collected = new Promise((resolve) => { collectDone = resolve; });
  ws.onmessage = (event) => {
    const message = JSON.parse(event.data);
    if (message.id && pending.has(message.id)) {
      const { resolve, reject } = pending.get(message.id);
      pending.delete(message.id);
      if (message.error) reject(new Error(message.error.message)); else resolve(message.result);
    } else if (message.method === "Tracing.dataCollected") {
      traceEvents.push(...message.params.value);
    } else if (message.method === "Tracing.tracingComplete") {
      collectDone();
    }
  };

  await send("Page.enable");
  await send("Emulation.setDeviceMetricsOverride", { width: 1920, height: 1080, deviceScaleFactor: 1, mobile: false });
  await send("Page.navigate", { url: appUrl });
  await sleep(6000);
  await send("Runtime.evaluate", { expression: `[...document.querySelectorAll('button')].find(b => b.textContent.trim() === 'Timeline')?.click()` });
  await sleep(1500);

  await send("Tracing.start", { categories: "blink,devtools.timeline,disabled-by-default-devtools.timeline", options: "" });
  const t0 = Date.now();
  send("Runtime.evaluate", {
    expression: `const m = document.querySelector('.timelineMarker.sceneBlock[data-timeline-event-id="10"]'); m.classList.add('probeX'); void document.body.offsetHeight; m.classList.remove('probeX'); 'done'`,
  }).then(() => console.log(`probe finished after ${Date.now() - t0} ms`)).catch(() => {});
  // give it up to 3 minutes
  for (let waited = 0; waited < 180_000; waited += 5000) {
    await sleep(5000);
    try {
      await Promise.race([send("Runtime.evaluate", { expression: "1" }), sleep(2500).then(() => { throw new Error("busy"); })]);
      break;
    } catch { /* busy */ }
  }
  await send("Tracing.end");
  await collected;

  const byName = new Map();
  for (const ev of traceEvents) {
    if (ev.ph === "X" && typeof ev.dur === "number") {
      byName.set(ev.name, (byName.get(ev.name) ?? 0) + ev.dur);
    }
  }
  const top = [...byName.entries()].sort((a, b) => b[1] - a[1]).slice(0, 15);
  console.log("--- top trace slices (total ms) ---");
  for (const [name, dur] of top) console.log(`${(dur / 1000).toFixed(0).padStart(8)} ms  ${name}`);
} finally {
  try { ws?.close(); } catch {}
  child.kill();
  await sleep(400);
  try { rmSync(dir, { recursive: true, force: true }); } catch {}
}
