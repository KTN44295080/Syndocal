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
// Daslight reference counts come from the maximized real-application sessions
// recorded in qa/DASLIGHT_OPERATOR_COUNT_AUDIT_2026-08-08.md. Keep the
// comparison honest: a Syndocal PASS here plus the recorded Daslight count for
// the same task is evidence; a missing Daslight observation means "no claim
// yet", never an assumed win.
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
  const url = `${appBase}/?syndocalViewportFixture=${fixture}`;
  const navigation = await send("Page.navigate", { url });
  if (navigation.errorText) {
    throw new Error(`Fixture navigation failed for ${url}: ${navigation.errorText}`);
  }
  const expectedOrigin = new URL(appBase).origin;
  const deadline = Date.now() + 15_000;
  let lastLocation = "";
  while (Date.now() < deadline) {
    lastLocation = await evalJs("location.href");
    const ready = await evalJs("document.readyState");
    if (lastLocation.startsWith(expectedOrigin) && ready !== "loading") {
      await sleep(1000);
      return;
    }
    await sleep(200);
  }
  throw new Error(`Fixture navigation did not reach ${expectedOrigin}; last location was ${lastLocation}`);
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
      const card = [...document.querySelectorAll('.sceneMatrixCard[data-timeline-cue-drag-source]')]
        .find(e =>
          e.getBoundingClientRect().width > 0 &&
          e.getAttribute('data-scene-matrix-active') !== 'true' &&
          !e.querySelector('[data-scene-flash-cue]'));
      const el = card?.querySelector('[data-scene-matrix-edit-strip]');
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
    record("trigger-scene-from-matrix", 1, 1, active === "true" && authored === "false", "1 click");

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
      "1 drag",
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
      "no equivalent one-action dial reset",
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
      "1 click",
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
    record("open-touch-workspace", 1, 1, touchVisible === true, "1 click");

    const cuePad = JSON.parse(await evalJs(`(() => {
      const el = document.querySelector('[data-touch-cue-pad="303"]');
      if (!el) return 'null';
      const r = el.getBoundingClientRect();
      return JSON.stringify({ x: r.x + r.width / 2, y: r.y + r.height / 2 });
    })()`));
    await opClick(cuePad.x, cuePad.y); // op 1
    const touchActive = await evalJs(`Boolean(document.querySelector('.cueLiveModifierStrip.touch[data-cue-live-modifier="303"]'))`);
    record("trigger-scene-from-touch", 1, 1, touchActive === true, "1 click");

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
    record("momentary-flash-from-touch", 1, 1, touchFlashDown === true && touchFlashUp === false, "1 press/release gesture");

    const edit = JSON.parse(await evalJs(`(() => {
      const el = [...document.querySelectorAll('.touchSurfaceModeToggle button')]
        .find((button) => button.textContent.trim() === 'EDIT');
      if (!el) return 'null';
      const r = el.getBoundingClientRect();
      return JSON.stringify({ x: r.x + r.width / 2, y: r.y + r.height / 2 });
    })()`));
    await opClick(edit.x, edit.y); // op 1
    const editMode = await evalJs(`document.querySelector('[data-touch-surface]')?.getAttribute('data-touch-mode')`);
    record("enter-touch-edit-mode", 1, 1, editMode === "edit", "1 click");

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
      "2 clicks (palette + placement)",
      `controls ${controlsBefore} -> ${controlsAfter}`,
    );
  }

  // ---- Task 13: create a cue-owned Mapping FX from its family. The current
  // Scene Settings workflow creates and selects the default family draft in
  // the same click; the retired sample-recipe cards are not part of this UI.
  // Mapping and Colour Mapping intentionally share the 2D MAPPING family, so
  // the production chooser has eight families rather than Daslight's nine.
  {
    await openFixture("fx-visual");
    const fxSetup = JSON.parse(await evalJs(`(async () => {
      const raf2 = () => new Promise(r => requestAnimationFrame(() => requestAnimationFrame(r)));
      const clickVisible = (selector, predicate) => {
        const el = [...document.querySelectorAll(selector)]
          .find(candidate => predicate(candidate) && candidate.getBoundingClientRect().width > 0);
        el?.click();
        return Boolean(el);
      };
      const show = clickVisible('.timelineDeskTabs button', el => el.textContent.trim() === 'Show');
      await raf2();
      const scene = clickVisible('[data-scene-matrix-edit-strip="401"]', () => true);
      await raf2();
      const fx = clickVisible('[data-scene-settings-surface-control="fx"]', () => true);
      await raf2(); await raf2();
      return JSON.stringify({
        show,
        scene,
        fx,
        surface: document.querySelector('[data-scene-settings]')?.getAttribute('data-scene-settings-surface') || '',
        families: document.querySelectorAll('[data-scene-fx-chooser] [data-effect-family]').length,
      });
    })()`));
    if (!fxSetup.scene || !fxSetup.fx || fxSetup.surface !== "fx" || fxSetup.families !== 8) {
      throw new Error(`FX operation-count starting state unavailable: ${JSON.stringify(fxSetup)}`);
    }
    const effectsBefore = Number(await evalJs("document.querySelectorAll('[data-scene-owned-effect]').length"));
    const mappings = JSON.parse(await evalJs(`(() => {
      const el = document.querySelector('[data-effect-family="2D MAPPING"]');
      if (!el) return 'null';
      const r = el.getBoundingClientRect();
      return JSON.stringify({ x: r.x + r.width / 2, y: r.y + r.height / 2 });
    })()`));
    if (!mappings) throw new Error("2D MAPPING family control unavailable after FX setup");
    await opClick(mappings.x, mappings.y); // op 1
    const mappingState = JSON.parse(await evalJs(`(() => {
      const active = document.querySelector('.effectFamilyChooser button.active[aria-pressed="true"]');
      const effects = [...document.querySelectorAll('[data-scene-owned-effect]')];
      const selected = effects.find((row) => row.querySelector('.sceneOwnedFxSelect[aria-pressed="true"]'));
      const editor = document.querySelector('[data-scene-settings-effect-editor]');
      const targetEditor = document.querySelector('[data-scene-effect-target-mode]');
      const activeFixture = document.querySelector('.sceneEffectTargetFixtureSelect select');
      return JSON.stringify({
        family: active?.getAttribute('data-effect-family') || '',
        effects: effects.length,
        selectedType: selected?.querySelector('small')?.textContent?.trim() || '',
        editorType: editor?.getAttribute('data-scene-settings-effect-editor') || '',
        targetMode: targetEditor?.getAttribute('data-scene-effect-target-mode') || '',
        activeFixtureId: activeFixture instanceof HTMLSelectElement ? Number(activeFixture.value) : null,
      });
    })()`));
    record(
      "create-mapping-fx-from-family",
      1,
      1,
      mappingState.family === "2D MAPPING"
        && mappingState.effects === effectsBefore + 1
        && mappingState.selectedType === "ColorMapping"
        && mappingState.editorType === "ColorMapping"
        && mappingState.targetMode === "fixture"
        && mappingState.activeFixtureId === 1,
      "1 click from open FX menu; Selected beams inherited",
      `${mappingState.family}; effects ${effectsBefore} -> ${mappingState.effects}; selected=${mappingState.selectedType}; editor=${mappingState.editorType}; target=${mappingState.targetMode}:fixture${mappingState.activeFixtureId}`,
    );
  }

  // ---- Task 14: patch a prepared profile at a prepared free address.
  // The profile choice, address and count are starting-state preparation.
  // Completion is one PATCH click in both applications. Daslight was observed
  // maximized with Strongpoint 13ch prepared at A400; one PATCH click created
  // the A400-412 block. Syndocal must retain the same one-operation finish.
  {
    await navigateFixture("patch");
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
    const patchSetup = JSON.parse(await evalJs(`(async () => {
      const raf2 = () => new Promise(r => requestAnimationFrame(() => requestAnimationFrame(r)));
      const clickVisibleText = (text) => {
        const el = [...document.querySelectorAll('button')]
          .find(candidate => candidate.textContent.trim() === text && candidate.getBoundingClientRect().width > 0);
        el?.click();
        return Boolean(el);
      };
      clickVisibleText('Setup'); await raf2();
      clickVisibleText('Lighting'); await raf2();
      clickVisibleText('Patch'); await raf2(); await raf2();
      const row = document.querySelector('[data-patch-profile-row][data-profile-source="session"]');
      const address = document.querySelector('[data-patch-field="address"] input');
      const count = document.querySelector('[data-patch-field="count"] input');
      if (!(row instanceof HTMLButtonElement) || !(address instanceof HTMLInputElement) || !(count instanceof HTMLInputElement)) {
        return JSON.stringify({ ready: false, reason: 'missing prepared profile row, address, or count input' });
      }
      row.click(); await raf2();
      address.value = '65';
      address.dispatchEvent(new Event('input', { bubbles: true }));
      count.value = '1';
      count.dispatchEvent(new Event('input', { bubbles: true }));
      await raf2(); await raf2();
      const button = [...document.querySelectorAll('[data-patch-minimal-form] button.primary')]
        .find(candidate => candidate.textContent.trim().toUpperCase() === 'PATCH' && candidate.getBoundingClientRect().width > 0);
      const target = document.querySelector('[data-dmx-address="65"]');
      if (!(button instanceof HTMLButtonElement) || !(target instanceof HTMLButtonElement)) {
        return JSON.stringify({ ready: false, reason: 'missing visible PATCH button or A65 target' });
      }
      const rect = button.getBoundingClientRect();
      return JSON.stringify({
        ready: !button.disabled && !target.classList.contains('occupied'),
        buttonDisabled: button.disabled,
        targetOccupied: target.classList.contains('occupied'),
        x: rect.x + rect.width / 2,
        y: rect.y + rect.height / 2,
        before: document.querySelectorAll('.dmxPatchFixtureBlock').length,
      });
    })()`));
    if (!patchSetup.ready) throw new Error(`PATCH operation-count starting state unavailable: ${JSON.stringify(patchSetup)}`);
    await opClick(patchSetup.x, patchSetup.y); // op 1
    await sleep(700);
    const patchResult = JSON.parse(await evalJs(`(() => JSON.stringify({
      after: document.querySelectorAll('.dmxPatchFixtureBlock').length,
      targetOccupied: document.querySelector('[data-dmx-address="65"]')?.classList.contains('occupied') === true,
      selectedStart: document.querySelector('.dmxPatchFixtureBlock.selected small')?.textContent?.trim() || '',
    }))()`));
    record(
      "patch-prepared-profile-to-address",
      1,
      1,
      patchResult.after === patchSetup.before + 1 && patchResult.targetOccupied,
      "1 click on PATCH (prepared profile/address)",
      `blocks ${patchSetup.before} -> ${patchResult.after}; A65=${patchResult.targetOccupied}; ${patchResult.selectedStart}`,
    );
  }

  // ---- Task 15: write a static Dimmer Full value into a prepared EDIT
  // scene. Scene selection, fixture selection and EDIT mode are starting-state
  // preparation. Daslight was observed maximized: one click on the selected
  // Strongpoint Dimmer control changed it from OFF to 100.0% in scene Red.
  {
    await navigateFixture("edit-live");
    const staticSetup = JSON.parse(await evalJs(`(async () => {
      const raf2 = () => new Promise(r => requestAnimationFrame(() => requestAnimationFrame(r)));
      const clickVisible = (selector, predicate) => {
        const el = [...document.querySelectorAll(selector)]
          .find(candidate => predicate(candidate) && candidate.getBoundingClientRect().width > 0);
        el?.click();
        return Boolean(el);
      };
      const control = clickVisible('.workspaceTabs button', el => el.textContent.trim() === 'Control');
      await raf2();
      const liveEdit = clickVisible('.controlModeTabs button', el => el.textContent.trim() === 'Live Edit');
      await raf2();
      const dimmer = clickVisible('.attributeCategoryRail button', el => el.textContent.trim().toUpperCase() === 'DIMMER');
      window.__syndocalSetControlFixtureSelection?.([1], 1, '');
      await raf2(); await raf2();
      const scene = clickVisible('[data-scene-matrix-edit-strip="301"]', () => true);
      await raf2();
      const edit = clickVisible('[data-control-fader-write-mode-option="edit"]', () => true);
      await raf2(); await raf2();
      const quickButtons = [...document.querySelectorAll('.dimmerQuickRow button')]
        .filter(candidate => candidate.getBoundingClientRect().width > 0);
      const outButton = quickButtons.find(candidate => candidate.textContent.trim() === 'Out');
      const button = quickButtons
        .find(candidate => candidate.textContent.trim() === 'Full' && candidate.getBoundingClientRect().width > 0);
      if (!(outButton instanceof HTMLButtonElement) || !(button instanceof HTMLButtonElement)) {
        return JSON.stringify({ ready: false, control, liveEdit, dimmer, scene, edit, reason: 'visible Dimmer Out/Full buttons missing' });
      }
      outButton.click();
      await new Promise(resolve => setTimeout(resolve, 420));
      const snapshot = window.__syndocalReadEditLiveFixtureSnapshot?.();
      const cue = snapshot?.cues?.find(candidate => candidate.id === 301);
      const before = cue?.targets
        ?.find(target => target.fixture_id === 1)
        ?.values?.find(value => value.attribute === 'Dimmer')?.value ?? null;
      const rect = button.getBoundingClientRect();
      return JSON.stringify({
        ready: control && liveEdit && scene && edit && before === 0 && !button.disabled,
        control,
        liveEdit,
        dimmer,
        scene,
        edit,
        before,
        selectedScene: document.querySelector('[data-scene-matrix-selected="true"]')
          ?.getAttribute('data-scene-matrix-cue-id') || '',
        writeMode: document.querySelector('[data-control-fader-write-header]')
          ?.getAttribute('data-control-fader-write-mode') || '',
        x: rect.x + rect.width / 2,
        y: rect.y + rect.height / 2,
      });
    })()`));
    if (!staticSetup.ready) throw new Error(`Static programming starting state unavailable: ${JSON.stringify(staticSetup)}`);
    await opClick(staticSetup.x, staticSetup.y); // op 1
    await sleep(600);
    const staticResult = JSON.parse(await evalJs(`(() => {
      const snapshot = window.__syndocalReadEditLiveFixtureSnapshot?.();
      const cue = snapshot?.cues?.find(candidate => candidate.id === 301);
      const otherCue = snapshot?.cues?.find(candidate => candidate.id === 302);
      return JSON.stringify({
        after: cue?.targets
          ?.find(target => target.fixture_id === 1)
          ?.values?.find(value => value.attribute === 'Dimmer')?.value ?? null,
        other: otherCue?.targets
          ?.find(target => target.fixture_id === 1)
          ?.values?.find(value => value.attribute === 'Dimmer')?.value ?? null,
        selectedScene: document.querySelector('[data-scene-matrix-selected="true"]')
          ?.getAttribute('data-scene-matrix-cue-id') || '',
        writeMode: document.querySelector('[data-control-fader-write-header]')
          ?.getAttribute('data-control-fader-write-mode') || '',
      });
    })()`));
    record(
      "program-static-dimmer-full",
      1,
      1,
      staticSetup.selectedScene === "301"
        && staticSetup.writeMode === "edit"
        && staticResult.after === 65_535
        && staticResult.selectedScene === "301"
        && staticResult.writeMode === "edit",
      "1 click on Dimmer Full (prepared EDIT scene)",
      `cue301 ${staticSetup.before} -> ${staticResult.after}; cue302=${staticResult.other}`,
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
