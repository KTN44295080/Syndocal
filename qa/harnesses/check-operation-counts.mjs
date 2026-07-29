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
  "--headless=new",
  "--no-sandbox",
  "--disable-gpu",
  "--disable-background-timer-throttling",
  "--disable-backgrounding-occluded-windows",
  "--disable-renderer-backgrounding",
  "--disable-crash-reporter",
  "--disable-crashpad",
  `--remote-debugging-port=${cdpPort}`,
  "--remote-debugging-address=127.0.0.1",
  `--user-data-dir=${mkdtempSync(join(tmpdir(), "syndocal-opcount-"))}`,
  "--no-first-run",
  "--window-size=1920,1080", "about:blank",
], { stdio: "ignore" });

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
async function waitForCdpTargets(timeoutMs = 10_000) {
  const deadline = Date.now() + timeoutMs;
  let lastError = null;
  while (Date.now() < deadline) {
    if (child.exitCode !== null) {
      throw new Error(`Browser exited before CDP became ready (exit ${child.exitCode})`);
    }
    try {
      const response = await fetch(`http://127.0.0.1:${cdpPort}/json/list`);
      if (response.ok) {
        const targets = await response.json();
        if (targets.some((target) => target.type === "page")) return targets;
      } else {
        lastError = new Error(`CDP target list returned HTTP ${response.status}`);
      }
    } catch (error) {
      lastError = error;
    }
    await sleep(200);
  }
  throw new Error(`CDP did not become ready within ${timeoutMs} ms`, { cause: lastError });
}
let ws; let messageId = 0;
const pending = new Map();
function send(method, params = {}) {
  const id = ++messageId;
  return new Promise((resolve, reject) => {
    const requestTimeoutMs = method === "Page.navigate" ? 30_000 : 10_000;
    const timeout = setTimeout(() => {
      pending.delete(id);
      reject(new Error(`CDP request timed out: ${method}`));
    }, requestTimeoutMs);
    pending.set(id, {
      resolve: (value) => {
        clearTimeout(timeout);
        resolve(value);
      },
      reject: (error) => {
        clearTimeout(timeout);
        reject(error);
      },
    });
    ws.send(JSON.stringify({ id, method, params }));
  });
}
function rejectPending(error) {
  for (const { reject } of pending.values()) reject(error);
  pending.clear();
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
  const deltaX = to.x - from.x;
  const deltaY = to.y - from.y;
  const distance = Math.max(1, Math.hypot(deltaX, deltaY));
  const captured = {
    x: from.x + (deltaX / distance) * 6,
    y: from.y + (deltaY / distance) * 6,
  };
  // Cross the production 4 px threshold while still inside the source so its
  // pointermove handler can establish pointer capture before the long move.
  await mouse("mouseMoved", captured.x, captured.y, { buttons: 1 });
  await sleep(100);
  const steps = 6;
  for (let i = 1; i <= steps; i += 1) {
    await mouse(
      "mouseMoved",
      captured.x + ((to.x - captured.x) * i) / steps,
      captured.y + ((to.y - captured.y) * i) / steps,
      { buttons: 1 },
    );
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

async function navigateFixture(fixture) {
  await send("Page.navigate", { url: `${appBase}/?syndocalViewportFixture=${fixture}` });
  await sleep(5000);
}

async function openFixture(fixture) {
  await navigateFixture(fixture);
  const localeChanged = await evalJs(`(() => {
    const key = 'syndocal.uiLocale.v1';
    const changed = window.localStorage.getItem(key) !== 'en';
    window.localStorage.setItem(key, 'en');
    return changed;
  })()`);
  if (localeChanged) {
    await send("Page.reload", { ignoreCache: true });
    await sleep(5000);
  }
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
  const targets = await waitForCdpTargets();
  const page = targets.find((t) => t.type === "page");
  ws = new WebSocket(page.webSocketDebuggerUrl);
  await new Promise((resolve, reject) => { ws.onopen = resolve; ws.onerror = () => reject(new Error("WS failed")); });
  ws.onclose = () => rejectPending(new Error("CDP WebSocket closed"));
  ws.onerror = () => rejectPending(new Error("CDP WebSocket failed"));
  ws.onmessage = (event) => {
    const m = JSON.parse(event.data);
    if (m.method === "Page.javascriptDialogOpening") {
      // Fixture mutations intentionally exercise dirty-state UI. Accept only
      // the browser's synthetic beforeunload dialog so the next fixture can
      // load; native Save/Load acceptance remains a separate Tauri gate.
      void send("Page.handleJavaScriptDialog", { accept: true }).catch((error) => {
        rejectPending(error);
      });
      return;
    }
    if (m.id && pending.has(m.id)) {
      const { resolve, reject } = pending.get(m.id);
      pending.delete(m.id);
      m.error ? reject(new Error(m.error.message)) : resolve(m.result);
    }
  };
  await send("Page.enable");
  await send("Runtime.enable");
  await send("Emulation.setDeviceMetricsOverride", { width: 1920, height: 1080, deviceScaleFactor: 1, mobile: false });

  // ---- Task 1: place a scene on a timeline lane. Budget: 1 op.
  // T14/T15 keep Scene Matrix and Timeline visible together, so the current
  // production task is one direct drag, matching Daslight's observed count.
  {
    await openFixture("timeline-layered");
    const before = Number(await evalJs("document.querySelectorAll('.timelineMarker.sceneBlock').length"));
    const grip = JSON.parse(await evalJs(`(() => {
      const el = [...document.querySelectorAll('.sceneMatrixCard[data-timeline-cue-drag-source]')]
        .find(e =>
          e.getBoundingClientRect().width > 0 &&
          e.getAttribute('data-scene-matrix-active') !== 'true' &&
          !e.querySelector('[data-scene-flash-cue]'));
      if (!el) return 'null';
      const r = el.getBoundingClientRect();
      return JSON.stringify({ x: r.x + r.width / 2, y: r.y + r.height / 2 });
    })()`));
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
    if (!grip || !lane) {
      const diagnostic = JSON.parse(await evalJs(`(() => JSON.stringify({
        lang: document.documentElement.lang,
        workspace: document.querySelector('.workspaceTabs')?.textContent?.trim() || '',
        controlMode: document.querySelector('.controlModeTabs')?.textContent?.trim() || '',
        sceneCards: document.querySelectorAll('.sceneMatrixCard[data-timeline-cue-drag-source]').length,
        timelineLayers: document.querySelectorAll('[data-timeline-layer-kind]').length,
      }))()`));
      throw new Error(`Scene-to-timeline drag target unavailable: ${JSON.stringify(diagnostic)}`);
    }
    await opDrag(grip, lane); // op 1
    const after = Number(await evalJs("document.querySelectorAll('.timelineMarker.sceneBlock').length"));
    const dropDiagnostic = JSON.parse(await evalJs(`(() => {
      const element = document.elementFromPoint(${lane.x}, ${lane.y});
      const target = element?.closest('[data-timeline-layer-id]');
      return JSON.stringify({
        status: document.querySelector('.appStatusText')?.textContent?.trim() || '',
        element: element?.tagName || '',
        className: typeof element?.className === 'string' ? element.className : element?.className?.baseVal || '',
        layerId: target?.getAttribute('data-timeline-layer-id') || '',
        layerKind: target?.getAttribute('data-timeline-layer-kind') || '',
      });
    })()`));
    record("place-scene-on-lane", 1, 1, after === before + 1, "1 drag (always-visible Scene Matrix)",
      after === before + 1 ? "" : `blocks ${before} -> ${after}; drop=${JSON.stringify(dropDiagnostic)}`);
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

  // ---- Tasks 4-7: live scene operation from the always-visible matrix.
  // The Daslight side has not been counted for these exact fixture/task
  // starting states, so these are Syndocal regression budgets only.
  {
    await openFixture("scene-matrix");
    const trigger = JSON.parse(await evalJs(`(() => {
      const el = document.querySelector('[data-scene-matrix-cue-id="303"] .sceneMatrixTrigger');
      if (!el) return 'null';
      const r = el.getBoundingClientRect();
      return JSON.stringify({ x: r.x + r.width / 2, y: r.y + r.height / 2 });
    })()`));
    await opClick(trigger.x, trigger.y); // op 1
    const active = await evalJs(`document.querySelector('[data-scene-matrix-cue-id="303"]')?.getAttribute('data-scene-matrix-active')`);
    const authored = await evalJs(`document.querySelector('[data-cue-live-modifier="303"]')?.getAttribute('data-live-override')`);
    record("trigger-scene-from-matrix", 1, 1, active === "true" && authored === "false", "未計測");

    const speed = JSON.parse(await evalJs(`(() => {
      const el = document.querySelector('[data-cue-live-modifier-speed="303"]');
      if (!el) return 'null';
      const r = el.getBoundingClientRect();
      const min = Number(el.min);
      const max = Number(el.max);
      const target = 3;
      return JSON.stringify({
        x: r.x + 8 + (r.width - 16) * ((target - min) / (max - min)),
        y: r.y + r.height / 2,
      });
    })()`));
    await opClick(speed.x, speed.y); // op 1; one range-pointer gesture
    const latched = JSON.parse(await evalJs(`(() => {
      const strip = document.querySelector('[data-cue-live-modifier="303"]');
      return JSON.stringify({
        override: strip?.getAttribute('data-live-override') || '',
        speed: strip?.querySelector('.cueLiveModifierRow b')?.textContent?.trim() || '',
        resetDisabled: strip?.querySelector('[data-cue-live-modifier-reset="303"]')?.disabled ?? true,
      });
    })()`));
    record(
      "latch-scene-live-speed",
      1,
      1,
      latched.override === "true" && latched.speed !== "x2" && latched.resetDisabled === false,
      "未計測",
      `speed ${latched.speed}`,
    );

    const reset = JSON.parse(await evalJs(`(() => {
      const el = document.querySelector('[data-cue-live-modifier-reset="303"]');
      if (!el) return 'null';
      const r = el.getBoundingClientRect();
      return JSON.stringify({ x: r.x + r.width / 2, y: r.y + r.height / 2 });
    })()`));
    await opClick(reset.x, reset.y); // op 1
    const resetState = JSON.parse(await evalJs(`(() => {
      const strip = document.querySelector('[data-cue-live-modifier="303"]');
      return JSON.stringify({
        override: strip?.getAttribute('data-live-override') || '',
        speed: strip?.querySelector('.cueLiveModifierRow b')?.textContent?.trim() || '',
      });
    })()`));
    record(
      "reset-scene-live-modifier",
      1,
      1,
      resetState.override === "false" && resetState.speed === "x2",
      "未計測",
      `speed ${resetState.speed}`,
    );

    await opClick(trigger.x, trigger.y); // op 1
    const released = JSON.parse(await evalJs(`(() => JSON.stringify({
      active: document.querySelector('[data-scene-matrix-cue-id="303"]')?.getAttribute('data-scene-matrix-active') || '',
      stripPresent: Boolean(document.querySelector('[data-cue-live-modifier="303"]')),
    }))()`));
    record(
      "release-active-scene-from-matrix",
      1,
      1,
      released.active === "false" && released.stripPresent === false,
      "未計測",
    );

  }

  // ---- Tasks 8-12: switch to the project-saved Touch surface, operate it,
  // then create one control in Edit mode. Fixture preparation is not counted;
  // every task action below is a real CDP pointer gesture.
  {
    const touchTab = JSON.parse(await evalJs(`(() => {
      const el = [...document.querySelectorAll('.workspaceTabs button')]
        .find((button) => button.title === 'Touch workspace');
      if (!el) return 'null';
      const r = el.getBoundingClientRect();
      return JSON.stringify({ x: r.x + r.width / 2, y: r.y + r.height / 2 });
    })()`));
    await opClick(touchTab.x, touchTab.y); // op 1
    const touchVisible = await evalJs(`(() => {
      const el = document.querySelector('[data-touch-surface]');
      return Boolean(el && el.getBoundingClientRect().width > 0);
    })()`);
    record("open-touch-workspace", 1, 1, touchVisible === true, "未計測");

    const cuePad = JSON.parse(await evalJs(`(() => {
      const el = document.querySelector('[data-touch-cue-pad="303"]');
      if (!el) return 'null';
      const r = el.getBoundingClientRect();
      return JSON.stringify({ x: r.x + r.width / 2, y: r.y + r.height / 2 });
    })()`));
    await opClick(cuePad.x, cuePad.y); // op 1
    const touchActive = await evalJs(`Boolean(document.querySelector('.cueLiveModifierStrip.touch[data-cue-live-modifier="303"]'))`);
    record("trigger-scene-from-touch", 1, 1, touchActive === true, "未計測");

    const touchFlash = JSON.parse(await evalJs(`(() => {
      const el = document.querySelector('[data-touch-flash-cue="320"]');
      if (!el) return 'null';
      const r = el.getBoundingClientRect();
      return JSON.stringify({ x: r.x + r.width / 2, y: r.y + r.height / 2 });
    })()`));
    await mouse("mousePressed", touchFlash.x, touchFlash.y, { buttons: 1 }); // one gesture begins
    await sleep(150);
    const touchFlashDown = await evalJs(`Boolean(document.querySelector('.cueLiveModifierStrip.touch[data-cue-live-modifier="320"]'))`);
    await mouse("mouseReleased", touchFlash.x, touchFlash.y, { buttons: 0 });
    await sleep(250);
    const touchFlashUp = await evalJs(`Boolean(document.querySelector('.cueLiveModifierStrip.touch[data-cue-live-modifier="320"]'))`);
    record("momentary-flash-from-touch", 1, 1, touchFlashDown === true && touchFlashUp === false, "未計測");

    const edit = JSON.parse(await evalJs(`(() => {
      const el = [...document.querySelectorAll('.touchSurfaceModeToggle button')]
        .find((button) => button.textContent.trim() === 'EDIT');
      if (!el) return 'null';
      const r = el.getBoundingClientRect();
      return JSON.stringify({ x: r.x + r.width / 2, y: r.y + r.height / 2 });
    })()`));
    await opClick(edit.x, edit.y); // op 1
    const editMode = await evalJs(`document.querySelector('[data-touch-surface]')?.getAttribute('data-touch-mode')`);
    record("enter-touch-edit-mode", 1, 1, editMode === "edit", "未計測");

    const controlsBefore = Number(await evalJs(`document.querySelectorAll('[data-touch-control]').length`));
    const addButton = JSON.parse(await evalJs(`(() => {
      const el = document.querySelector('.touchControlPalette button');
      if (!el) return 'null';
      const r = el.getBoundingClientRect();
      return JSON.stringify({ x: r.x + r.width / 2, y: r.y + r.height / 2 });
    })()`));
    await opClick(addButton.x, addButton.y); // op 1
    const controlsAfter = Number(await evalJs(`document.querySelectorAll('[data-touch-control]').length`));
    record(
      "add-touch-control",
      1,
      1,
      controlsAfter === controlsBefore + 1,
      "未計測",
      `controls ${controlsBefore} -> ${controlsAfter}`,
    );
  }

  // ---- Tasks 13-14: effect-family and recipe selection. These assert the
  // visible programming state, while Daslight same-task counts stay unmeasured.
  {
    await navigateFixture("fx-visual");
    const mappings = JSON.parse(await evalJs(`(() => {
      const el = document.querySelector('[data-effect-family="MAPPINGS"]');
      if (!el) return 'null';
      const r = el.getBoundingClientRect();
      return JSON.stringify({ x: r.x + r.width / 2, y: r.y + r.height / 2 });
    })()`));
    await opClick(mappings.x, mappings.y); // op 1
    const mappingState = JSON.parse(await evalJs(`(() => {
      const active = document.querySelector('.effectFamilyChooser button.active[aria-pressed="true"]');
      const cards = [...document.querySelectorAll('.sampleEffectPresetCard')]
        .filter((card) => card.getBoundingClientRect().width > 0);
      return JSON.stringify({ family: active?.getAttribute('data-effect-family') || '', cards: cards.length });
    })()`));
    record(
      "select-effect-family",
      1,
      1,
      mappingState.family === "MAPPINGS" && mappingState.cards === 3,
      "未計測",
      `${mappingState.family}/${mappingState.cards} recipes`,
    );

    const recipe = JSON.parse(await evalJs(`(() => {
      const cards = [...document.querySelectorAll('.sampleEffectPresetCard')]
        .filter((card) => card.getBoundingClientRect().width > 0);
      const el = cards[1];
      if (!el) return 'null';
      el.scrollIntoView({ block: 'nearest', inline: 'nearest' });
      const r = el.getBoundingClientRect();
      const x = r.x + r.width / 2;
      const y = r.y + r.height / 2;
      const hit = document.elementFromPoint(x, y);
      return JSON.stringify({ x, y, hit: Boolean(hit && (hit === el || el.contains(hit))) });
    })()`));
    await opClick(recipe.x, recipe.y); // op 1
    const selectedRecipe = await evalJs(`(() => {
      const selected = document.querySelector('.sampleEffectPresetCard[aria-pressed="true"]');
      return selected?.querySelector('strong')?.textContent?.trim() || '';
    })()`);
    record(
      "select-effect-recipe",
      1,
      1,
      recipe.hit === true && selectedRecipe === "Ball",
      "未計測",
      `hit=${recipe.hit} selected=${selectedRecipe}`,
    );
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
