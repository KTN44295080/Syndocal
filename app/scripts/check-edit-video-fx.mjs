import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { mkdir, writeFile } from "node:fs/promises";
import { createRequire } from "node:module";
import { homedir } from "node:os";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

// Use the project's Playwright when installed, or the desktop's bundled runtime.
// PLAYWRIGHT_MODULE_PATH makes the same gate usable from other runtime layouts.
const require = createRequire(import.meta.url);
const { chromium } = (() => {
  try { return require("playwright"); } catch {
    return require(process.env.PLAYWRIGHT_MODULE_PATH ?? resolve(homedir(),
      ".cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright"));
  }
})();
const appRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const artifacts = resolve(appRoot, "../target/qa/edit-video-fx-20260905");
const port = Number(process.env.EDIT_VIDEO_FX_PORT ?? 5197);
const origin = `http://127.0.0.1:${port}`;
const sleep = (ms) => new Promise((done) => setTimeout(done, ms));
const chromePath = [process.env.CHROME_PATH,
  "C:/Program Files/Google/Chrome/Application/chrome.exe",
  "C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe",
].find((path) => path && existsSync(path));
const vite = spawn(process.execPath, [resolve(appRoot, "node_modules/vite/bin/vite.js"),
  "--host", "127.0.0.1", "--port", String(port), "--strictPort"],
{ cwd: appRoot, windowsHide: true, stdio: ["ignore", "pipe", "pipe"] });
let viteLog = "";
vite.stdout.on("data", (data) => { viteLog += data; });
vite.stderr.on("data", (data) => { viteLog += data; });
let browser;
try {
  for (let i = 0; ; i++) {
    assert.equal(vite.exitCode, null, `Owned Vite process exited: ${viteLog}`);
    try { if ((await fetch(origin)).ok) break; } catch { /* starting */ }
    assert.ok(i < 150, `Vite did not start: ${viteLog}`);
    await sleep(100);
  }
  await mkdir(artifacts, { recursive: true });
  browser = await chromium.launch({ headless: true, ...(chromePath ? { executablePath: chromePath } : {}) });
  for (const viewport of [{ width: 1920, height: 1080 }, { width: 1280, height: 720 }]) {
    const page = await browser.newPage({ viewport });
    const pageErrors = [];
    page.on("pageerror", (error) => pageErrors.push(String(error)));
    await page.addInitScript(() => localStorage.setItem("syndocal.uiLocale.v1", "en"));
    await page.goto(`${origin}/?syndocalViewportFixture=operator-vj`);
    await page.waitForFunction(() => typeof window.__syndocalReadOperatorVjFixtureSnapshot === "function");
    const inspector = page.locator("[data-edit-video-inspector]");
    await inspector.waitFor({ state: "visible" });
    assert.match(await inspector.innerText(), /Select a Media Library item/,
      "Fixture has no selected media asset; layer controls must still be reachable");
    assert.equal(await page.locator("[data-video-isf-layer-id]").count(), 0, "Closed inspector mounts no FX consumers");
    const legacy = ".videoControlPanelLibrary :is(.videoMixerContextPane,.videoClipLegacyTransport,.videoClipGridPanel,.videoMixerTopPane,.videoMixerDiagnostics,.videoMixerPaneHeader)";
    assert.equal(await page.locator(legacy).count(), 0, "Library-only mode does not mount hidden legacy consumer subtrees");
    assert.equal(await page.locator('.videoControlPanelLibrary [data-video-clip-slot-bank="edit"]').count(), 1,
      "Library-only mode keeps the Edit Video Clip Bank reachable");
    await inspector.locator(".editVideoAdvancedDisclosure > summary").click();
    await page.locator("[data-video-isf-layer-id]").waitFor();
    assert.equal(await page.locator("[data-video-isf-layer-id]").count(), 1);
    const layer = page.locator("[data-edit-video-fx-layer]");
    assert.equal(await layer.inputValue(), "1");
    await page.locator('[data-video-isf-action="advanced"]').click();
    await page.locator(".videoIsfAdvanced").waitFor();
    assert.equal(await page.locator(".videoIsfStackRow").count(), 8, "Existing eight-stage layer remains editable");
    await layer.selectOption("2");
    await page.locator('[data-video-isf-layer-id="2"]').waitFor();
    assert.equal(await page.locator(".videoIsfAdvanced").count(), 0, "Layer switch resets the nested editor selection/disclosure");
    await page.locator('[data-video-isf-action="advanced"]').click();
    await page.locator(".videoIsfAdvanced").waitFor();

    // Only replace the native boundary. The actual App callback, command facade,
    // refresh path and inspector all execute; this is not a GPU rendering test.
    await page.evaluate(() => {
      const mock = { calls: [], effects: {}, snapshotReads: 0 };
      window.__editVideoFxMock = mock;
      Object.defineProperty(window, "__TAURI_INTERNALS__", { configurable: true, value: {
        invoke: async (command, args = {}) => {
          mock.calls.push({ command, args: structuredClone(args) });
          if (command === "register_project_transaction_owner") return null;
          if (command === "add_builtin_video_isf_effect") {
            const effect = { ...structuredClone(window.__syndocalReadOperatorVjFixtureSnapshot().video.layers[0].isf_effect),
              label: "Invert", stack: [], enabled: true };
            mock.effects[args.layerId] = effect;
            return effect;
          }
          if (command === "get_snapshot") {
            mock.snapshotReads++;
            const snapshot = structuredClone(window.__syndocalReadOperatorVjFixtureSnapshot());
            for (const candidate of snapshot.video.layers) {
              if (mock.effects[candidate.id]) candidate.isf_effect = structuredClone(mock.effects[candidate.id]);
            }
            return { snapshot, timeline_runtime: {
              transport_epoch: 1, transport_generation: 1,
              loop_runtime: { generation: 0, status: "disabled", a_ms: null, b_ms: null, wrap_count: 0 },
              follow_runtime: { epoch: 1, generation: 0, status: "idle", admission_reason: null, outcome: null,
                source_timeline_id: null, target_timeline_id: null, elapsed_ms: 0, duration_ms: 0,
                progress_millis: 0, fault: null, transition_hold_active: false, waiting_for_pedal_start: false },
            } };
          }
          if (command === "get_video_preview_diagnostics") return { isf_stage_errors: [], layer_queues: [], output_decode_previews: [], decoder_diagnostics: {} };
          throw new Error(`Unexpected Edit Video fixture command: ${command}`);
        },
      } });
    });
    const builtin = page.locator('[data-video-isf-action="builtin"]');
    const invert = await builtin.locator("option").evaluateAll((options) => options.find((option) => option.textContent === "Invert")?.value);
    assert.ok(invert, "Existing Invert preset is selectable");
    await builtin.selectOption(invert);
    await page.waitForFunction(() => window.__editVideoFxMock.calls.some((call) => call.command === "add_builtin_video_isf_effect"), null, { timeout: 5000 }).catch(async (error) => {
      console.error(await page.evaluate(() => ({ mock: window.__editVideoFxMock, text: document.body.innerText.slice(-6500) })));
      throw error;
    });
    await page.waitForFunction(() => window.__editVideoFxMock.snapshotReads > 0);
    await page.waitForFunction(() => document.querySelector('[data-video-isf-layer-id="2"] .videoIsfQuickStatus [data-no-localize]')?.textContent === "Invert", null, { timeout: 5000 }).catch(async (error) => {
      console.error(JSON.stringify(await page.evaluate(() => ({ mock: window.__editVideoFxMock, timeline: window.__syndocalReadOperatorVjFixtureSnapshot().timeline, text: document.body.innerText.slice(-1500) }))));
      throw error;
    });
    assert.equal(await page.locator(".videoIsfAdvanced").count(), 1, "Same-layer snapshot refresh preserves the open nested editor");
    const calls = await page.evaluate(() => window.__editVideoFxMock.calls.filter((call) => call.command === "add_builtin_video_isf_effect"));
    assert.deepEqual(calls.map((call) => [call.args.layerId, call.args.presetId]), [[2, invert]], "Builtin invokes exactly the selected layer once");
    await layer.selectOption("1");
    await page.locator('[data-video-isf-layer-id="1"]').waitFor();
    assert.equal(await page.locator("[data-video-isf-layer-id]").count(), 1);
    await page.locator('[data-video-isf-action="advanced"]').click();
    await page.locator(".videoIsfAdvanced").waitFor();
    await page.locator('.videoIsfAdvanced').scrollIntoViewIfNeeded();
    const geometry = await inspector.evaluate((root) => {
      const bounds = root.getBoundingClientRect();
      const controls = [...root.querySelectorAll("button,select,input")].map((control) => {
        const target = control.matches('input[type="checkbox"]') ? control.closest('label') : control;
        const rect = target.getBoundingClientRect();
        const container = control.closest('.videoIsfStackRow,.videoIsfControl,.videoIsfQuickRack') ?? root;
        const local = container.getBoundingClientRect();
        // Existing operator quick actions are 28px, generic stack actions26px,
        // value fields28px (30px at the larger viewport) and checkbox visual18px.
        const minimumHeight = control.matches('input[type="checkbox"]') ? 18
          : control.closest('.videoIsfStackRow') ? 26
            : 28;
        return { tag: control.tagName, label: control.getAttribute('aria-label') ?? control.textContent?.slice(0, 35), height: rect.height, width: rect.width,
          minimumHeight,
          outsideX: rect.left < bounds.left - 1 || rect.right > bounds.right + 1 || rect.left < local.left - 1 || rect.right > local.right + 1 };
      }).filter((rect) => rect.height > 0 && rect.width > 0);
      return { inspectorContained: bounds.left >= -1 && bounds.right <= innerWidth + 1 && bounds.bottom <= innerHeight + 1,
        documentOverflow: [document.documentElement.scrollWidth - innerWidth, document.documentElement.scrollHeight - innerHeight],
        shortControls: controls.filter((rect) => rect.height < rect.minimumHeight - 0.5), clippedControls: controls.filter((rect) => rect.outsideX), controlCount: controls.length };
    });
    assert.equal(geometry.inspectorContained, true);
    assert.deepEqual(geometry.documentOverflow, [0, 0]);
    await page.screenshot({ path: resolve(artifacts, `expanded-${viewport.width}x${viewport.height}.png`) });
    assert.deepEqual(geometry.shortControls, [], "Existing FX control sizing is preserved");
    assert.deepEqual(geometry.clippedControls, [], "Every FX action/input remains horizontally inside the inspector scrollport");
    const lastAction = inspector.getByRole("button", { name: "Clear stack", exact: true });
    await lastAction.scrollIntoViewIfNeeded();
    await lastAction.click({ trial: true });
    await page.screenshot({ path: resolve(artifacts, `controls-${viewport.width}x${viewport.height}.png`) });
    await inspector.locator('.videoIsfStackRow').first().scrollIntoViewIfNeeded();
    await page.screenshot({ path: resolve(artifacts, `stack-${viewport.width}x${viewport.height}.png`) });
    await inspector.locator('.editVideoAdvancedDisclosure > summary').scrollIntoViewIfNeeded();
    await page.screenshot({ path: resolve(artifacts, `overview-${viewport.width}x${viewport.height}.png`) });
    await inspector.locator(".editVideoAdvancedDisclosure > summary").click();
    await page.waitForFunction(() => document.querySelectorAll("[data-video-isf-layer-id]").length === 0);
    assert.equal(await page.locator("[data-edit-video-layer-fx]").count(), 0);
    assert.deepEqual(pageErrors, [], "No uncaught browser runtime failures");
    await writeFile(resolve(artifacts, `result-${viewport.width}x${viewport.height}.json`), JSON.stringify({ viewport, geometry, calls, pageErrors }, null, 2));
    console.log(`PASS edit-video-fx ${viewport.width}x${viewport.height}: mounted=0/1/0, selected-layer callback=2, controls=${geometry.controlCount}, outer-overflow=0`);
    await page.close();
  }
} finally {
  await browser?.close();
  if (vite.exitCode === null) {
    const exited = new Promise((done) => vite.once("exit", done));
    vite.kill();
    await Promise.race([exited, sleep(2000)]);
  }
}
