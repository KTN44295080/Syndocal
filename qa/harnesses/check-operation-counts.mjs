// Executable operation-count contract for core lighting-desk tasks.
//
// The project goal forbids claiming operability superiority over Daslight 5
// without same-task operation-count measurements. This harness makes the
// Syndocal side of that claim executable: each task below is performed with
// EXACTLY its budgeted number of user operations (1 drag = 1 op, 1 click =
// 1 op, 1 key = 1 op) via CDP input events, and the outcome is asserted.
// If a UI change adds a required gesture, the task fails - operation-count
// regressions become visible instead of anecdotal.
//
// Daslight reference counts come from the observed sessions recorded in
// target/qa/ui-comparison/PRIMARY_OBSERVATIONS.md (manual counts on the real
// application; not automatable). Keep the comparison honest: a Syndocal PASS
// here plus the recorded Daslight count for the same task is evidence; a
// missing Daslight observation means "no claim yet", never an assumed win.
//
// Usage: node qa/harnesses/check-operation-counts.mjs [appUrl]
//   appUrl defaults to http://127.0.0.1:5173 (a running dev server is
//   required; the fixture query parameter is appended per task).
import { spawn } from "node:child_process";
import { mkdtempSync, existsSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const appBase = process.argv[2] ?? "http://127.0.0.1:5173";
const cdpPort = 9339;
const chromeCandidates = [
  process.env.CHROME_PATH,
  "C:/Program Files/Google/Chrome/Application/chrome.exe",
  "C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe",
].filter(Boolean);
const chrome = chromeCandidates.find((candidate) => existsSync(candidate));
if (!chrome) throw new Error("No Chrome/Edge found; set CHROME_PATH");

const child = spawn(chrome, [
  `--remote-debugging-port=${cdpPort}`,
  `--user-data-dir=${mkdtempSync(join(tmpdir(), "syndocal-opcount-"))}`,
  "--headless=new", "--disable-gpu", "--no-first-run",
  "--window-size=1920,1080", "about:blank",
], { stdio: "ignore" });

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
let ws; let messageId = 0;
const pending = new Map();
function send(method, params = {}) {
  const id = ++messageId;
  return new Promise((resolve, reject) => {
    pending.set(id, { resolve, reject });
    ws.send(JSON.stringify({ id, method, params }));
  });
}
const evalJs = async (expr) => {
  const r = await send("Runtime.evaluate", { expression: expr, awaitPromise: true, returnByValue: true });
  if (r.exceptionDetails) throw new Error(r.exceptionDetails.exception?.description ?? "eval failed");
  return r.result?.value;
};
const mouse = (type, x, y, extra = {}) =>
  send("Input.dispatchMouseEvent", { type, x, y, button: "left", clickCount: 1, ...extra });

// One user operation = one gesture. A drag is pointer-down -> move(s) -> up:
// intermediate moves are part of the same gesture, not extra operations.
async function opClick(x, y) {
  await mouse("mousePressed", x, y, { buttons: 1 });
  await mouse("mouseReleased", x, y, { buttons: 0 });
  await sleep(250);
}
async function opDrag(from, to) {
  await mouse("mousePressed", from.x, from.y, { buttons: 1 });
  const steps = 6;
  for (let i = 1; i <= steps; i += 1) {
    await mouse("mouseMoved", from.x + ((to.x - from.x) * i) / steps, from.y + ((to.y - from.y) * i) / steps, { buttons: 1 });
    await sleep(60);
  }
  await mouse("mouseReleased", to.x, to.y, { buttons: 0 });
  await sleep(400);
}
async function opKey(key, code) {
  await send("Input.dispatchKeyEvent", { type: "keyDown", key, code, windowsVirtualKeyCode: key === "Escape" ? 27 : 0 });
  await send("Input.dispatchKeyEvent", { type: "keyUp", key, code, windowsVirtualKeyCode: key === "Escape" ? 27 : 0 });
  await sleep(250);
}

async function openFixture(fixture) {
  await send("Page.navigate", { url: `${appBase}/?syndocalViewportFixture=${fixture}` });
  await sleep(5000);
  await evalJs(`(async () => {
    const raf2 = () => new Promise(r => requestAnimationFrame(() => requestAnimationFrame(r)));
    const clickVis = (sel, text) => { const el = [...document.querySelectorAll(sel)].find(b => b.textContent.trim() === text && b.getBoundingClientRect().width > 0); el?.click(); return !!el; };
    clickVis('.workspaceTabs button', 'Control'); await raf2();
    clickVis('.controlModeTabs button', 'Timeline'); await raf2(); await raf2();
    return true;
  })()`);
  await sleep(300);
}

const results = [];
function record(task, budget, used, passed, daslight, note = "") {
  results.push({ task, budget, used, passed, daslight, note });
  console.log(`${passed ? "pass" : "FAIL"} ${task}: ops=${used}/${budget} (Daslight: ${daslight}) ${note}`);
}

try {
  await sleep(1500);
  const targets = await (await fetch(`http://127.0.0.1:${cdpPort}/json/list`)).json();
  const page = targets.find((t) => t.type === "page");
  ws = new WebSocket(page.webSocketDebuggerUrl);
  await new Promise((resolve, reject) => { ws.onopen = resolve; ws.onerror = () => reject(new Error("WS failed")); });
  ws.onmessage = (event) => {
    const m = JSON.parse(event.data);
    if (m.id && pending.has(m.id)) {
      const { resolve, reject } = pending.get(m.id);
      pending.delete(m.id);
      m.error ? reject(new Error(m.error.message)) : resolve(m.result);
    }
  };
  await send("Page.enable");
  await send("Runtime.enable");
  await send("Emulation.setDeviceMetricsOverride", { width: 1920, height: 1080, deviceScaleFactor: 1, mobile: false });

  // ---- Task 1: place a scene on a timeline lane. Budget: 2 ops
  // (1 click to open the Cues surface where the pool lives, 1 grip drag).
  // Daslight: 1 drag from its always-visible scene list (observed) - so the
  // honest comparison is 2 vs 1 until the cue pool is reachable without a
  // surface switch; the assertion protects the drag itself staying single.
  {
    await openFixture("timeline-layered");
    const before = Number(await evalJs("document.querySelectorAll('.timelineMarker.sceneBlock').length"));
    const cuesBtn = JSON.parse(await evalJs(`(() => {
      const el = [...document.querySelectorAll('.timelineDeskTabs button')].find(b => b.textContent.trim() === 'Cues' && b.getBoundingClientRect().width > 0);
      if (!el) return 'null';
      const r = el.getBoundingClientRect();
      return JSON.stringify({ x: r.x + r.width / 2, y: r.y + r.height / 2 });
    })()`));
    await opClick(cuesBtn.x, cuesBtn.y); // op 1
    const grip = JSON.parse(await evalJs(`(() => {
      const el = [...document.querySelectorAll('[data-timeline-cue-drag-source]')].find(e => e.getBoundingClientRect().width > 0);
      if (!el) return 'null';
      const r = el.getBoundingClientRect();
      return JSON.stringify({ x: r.x + r.width / 2, y: r.y + r.height / 2 });
    })()`));
    // Drag toward the lighting lane; the surface switches to Show mid-drag,
    // so the target is resolved when the pointer arrives (drop coordinates
    // measured after the switch would be a second gesture - instead aim at
    // the known lane band region revealed during the drag).
    await mouse("mousePressed", grip.x, grip.y, { buttons: 1 }); // op 2 begins
    await mouse("mouseMoved", grip.x + 30, grip.y - 30, { buttons: 1 });
    await sleep(400);
    const lane = JSON.parse(await evalJs(`(() => {
      const all = [...document.querySelectorAll('[data-timeline-layer-kind]')].filter(e => {
        const r = e.getBoundingClientRect();
        return r.width > 300 && r.height > 5 && !e.hasAttribute('data-timeline-layer-gutter');
      });
      const light = all.filter(e => (e.getAttribute('data-timeline-layer-kind') ?? '').toLowerCase() === 'lighting')[0];
      if (!light) return 'null';
      const r = light.getBoundingClientRect();
      return JSON.stringify({ x: r.x + r.width * 0.55, y: r.y + r.height / 2 });
    })()`));
    await mouse("mouseMoved", lane.x, lane.y, { buttons: 1 });
    await sleep(250);
    await mouse("mouseReleased", lane.x, lane.y, { buttons: 0 }); // op 2 ends
    await sleep(500);
    const after = Number(await evalJs("document.querySelectorAll('.timelineMarker.sceneBlock').length"));
    record("place-scene-on-lane", 2, 2, after === before + 1, "1 drag (always-visible pool)",
      after === before + 1 ? "" : `blocks ${before} -> ${after}`);
  }

  // ---- Task 2: toggle a layer's mute from the desk. Budget: 1 op.
  // Daslight: 1 click on the lane's toggle (observed).
  {
    const eye = JSON.parse(await evalJs(`(() => {
      const gutter = document.querySelector('[data-timeline-layer-id="12"][data-timeline-layer-gutter]');
      const el = gutter?.querySelector('[data-timeline-layer-mute-toggle]');
      if (!el) return 'null';
      const r = el.getBoundingClientRect();
      return JSON.stringify({ x: r.x + r.width / 2, y: r.y + r.height / 2 });
    })()`));
    const mutedBefore = await evalJs(`document.querySelector('[data-timeline-layer-id="12"][data-timeline-layer-gutter]')?.getAttribute('data-timeline-layer-muted')`);
    await opClick(eye.x, eye.y); // op 1
    await sleep(400);
    const mutedAfter = await evalJs(`document.querySelector('[data-timeline-layer-id="12"][data-timeline-layer-gutter]')?.getAttribute('data-timeline-layer-muted')`);
    record("toggle-layer-mute", 1, 1, mutedBefore !== mutedAfter, "1 click", `muted ${mutedBefore} -> ${mutedAfter}`);
  }

  // ---- Task 3: expand the timeline pane to full width and restore. Budget:
  // 2 ops (1 click expand, 1 Esc restore). Daslight: 1 click + 1 click
  // (pane expand button, observed).
  {
    const toggle = JSON.parse(await evalJs(`(() => {
      const el = document.querySelector('[data-timeline-pane-expand-toggle]');
      if (!el) return 'null';
      const r = el.getBoundingClientRect();
      return JSON.stringify({ x: r.x + r.width / 2, y: r.y + r.height / 2 });
    })()`));
    await opClick(toggle.x, toggle.y); // op 1
    const expanded = await evalJs(`document.querySelector('.mappingPersistentWorkspaceBand')?.dataset?.timelinePaneExpanded`);
    await opKey("Escape", "Escape"); // op 2
    const restored = await evalJs(`document.querySelector('.mappingPersistentWorkspaceBand')?.dataset?.timelinePaneExpanded`);
    record("expand-and-restore-timeline-pane", 2, 2, expanded === "true" && restored === "false", "2 clicks");
  }

  const failed = results.filter((r) => !r.passed);
  console.log("\n== operation-count summary ==");
  for (const r of results) {
    console.log(`${r.passed ? "PASS" : "FAIL"}  ${r.task}  syndocal=${r.used} ops  daslight=${r.daslight}`);
  }
  if (failed.length > 0) {
    console.log(`${failed.length} task(s) exceeded their operation budget or failed their outcome.`);
    process.exit(1);
  }
} finally {
  try { ws?.close(); } catch { /* closing race */ }
  child.kill();
}
