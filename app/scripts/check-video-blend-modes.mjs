import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { mkdir, writeFile } from "node:fs/promises";
import { createRequire } from "node:module";
import { homedir } from "node:os";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

// Browser plugin not available. Reuse the existing local Playwright/Vite gate.
const require = createRequire(import.meta.url);
const { chromium } = (() => {
  try { return require("playwright"); } catch {
    return require(process.env.PLAYWRIGHT_MODULE_PATH ?? resolve(homedir(),
      ".cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright"));
  }
})();
const appRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const artifacts = resolve(appRoot, "../target/qa/blend-modes");
const port = Number(process.env.VIDEO_BLEND_MODES_PORT ?? 5198);
const origin = `http://127.0.0.1:${port}`;
const sleep = (ms) => new Promise((done) => setTimeout(done, ms));
const chromePath = [process.env.CHROME_PATH,
  "C:/Program Files/Google/Chrome/Application/chrome.exe",
  "C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe",
].find((path) => path && existsSync(path));
const modes = ["Normal", "Add", "Multiply", "Screen", "Overlay", "Subtract", "Difference", "Lighten", "Darken", "SoftLight"];
const labels = {
  en: ["Normal", "Add", "Multiply", "Screen", "Overlay", "Subtract", "Difference", "Lighten", "Darken", "Soft Light"],
  ja: ["通常", "追加", "乗算", "画面", "オーバーレイ", "減算", "差の絶対値", "比較（明）", "比較（暗）", "ソフトライト"],
};
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
    for (const locale of ["en", "ja"]) {
      const page = await browser.newPage({ viewport });
      page.setDefaultTimeout(15_000);
      const pageErrors = [];
      const consoleErrors = [];
      const faviconErrors = [];
      page.on("pageerror", (error) => pageErrors.push(String(error)));
      page.on("console", (message) => {
        if (message.type() !== "error" && message.type() !== "warning") return;
        const diagnostic = `${message.type()}: ${message.text()} (${message.location().url})`;
        // Vite's development page has no favicon; keep this exact unrelated
        // resource error in evidence instead of hiding application failures.
        if (message.type() === "error" && message.location().url === `${origin}/favicon.ico`
          && message.text() === "Failed to load resource: the server responded with a status of 404 (Not Found)") faviconErrors.push(diagnostic);
        else consoleErrors.push(diagnostic);
      });
      await page.addInitScript((value) => localStorage.setItem("syndocal.uiLocale.v1", value), locale);
      await page.goto(`${origin}/?syndocalViewportFixture=operator-vj`);
      await page.waitForFunction(() => typeof window.__syndocalReadOperatorVjFixtureSnapshot === "function");
      assert.equal(new URL(page.url()).origin, origin);
      assert.match(await page.title(), /Syndocal/i);
      assert.equal(await page.locator("vite-error-overlay").count(), 0);
      const blend = page.locator('[data-video-layer-blend="1"]');
      await blend.waitFor({ state: "visible" });
      await blend.scrollIntoViewIfNeeded();
      await blend.click({ trial: true }).catch(async (error) => {
        await page.screenshot({ path: resolve(artifacts, `browser-blocked-${locale}-${viewport.width}x${viewport.height}.png`) });
        console.error(JSON.stringify(await blend.evaluate((select) => {
          const rect = select.getBoundingClientRect();
          const layout = (element) => {
            const style = getComputedStyle(element);
            return { rect: element.getBoundingClientRect().toJSON(), display: style.display, height: style.height,
              minHeight: style.minHeight, maxHeight: style.maxHeight, rows: style.gridTemplateRows, autoRows: style.gridAutoRows };
          };
          return { rect: rect.toJSON(), parent: layout(select.closest(".videoLayerItem")), list: layout(select.closest(".videoLayerList")),
            hit: document.elementFromPoint(rect.x + rect.width / 2, rect.y + rect.height / 2)?.outerHTML.slice(0, 500) };
        })));
        throw error;
      });
      await page.waitForFunction(({ expected }) => {
        const select = document.querySelector('[data-video-layer-blend="1"]');
        return JSON.stringify([...select.options].map((option) => option.textContent)) === JSON.stringify(expected);
      }, { expected: labels[locale] });
      assert.deepEqual(await blend.locator("option").evaluateAll((options) => options.map((option) => option.value)), modes);
      assert.ok((await page.locator(".videoLayerList").innerText()).length > 50, "Meaningful layer content rendered");

      // Mock only native IPC. The App callback, transaction facade, refresh and
      // Solid control execute unchanged; this does not test CPU/GPU rendering.
      await page.evaluate(() => {
        const mock = { calls: [], modes: {}, snapshotReads: 0, transactionId: 0, unexpected: [] };
        const readSnapshot = () => {
          const snapshot = structuredClone(window.__syndocalReadOperatorVjFixtureSnapshot());
          for (const layer of snapshot.video.layers) {
            if (mock.modes[layer.id]) layer.blend_mode = mock.modes[layer.id];
          }
          return snapshot;
        };
        window.__videoBlendMock = mock;
        Object.defineProperty(window, "__TAURI_INTERNALS__", { configurable: true, value: {
          invoke: async (command, args = {}) => {
            mock.calls.push({ command, args: structuredClone(args) });
            if (command === "register_project_transaction_owner") return null;
            if (command === "set_program_audio_handoff_config" || command === "acknowledge_project_transaction") return null;
            if (command === "get_live_video_monitor_frame") {
              // A valid busy packet supplies no rendered image and makes no
              // visual-compositor claim while this menu gate owns native IPC.
              const packet = Array.from({ length: 40 }, () => 0);
              packet.splice(0, 7, 0x53, 0x59, 0x4c, 0x56, 2, 1, args.monitorKind === "preview" ? 1 : 0);
              return packet;
            }
            if (command === "remote_control_status") return {
              running: false, active_connections: 0, rejected_connections: 0, clients: [], web_remote_enabled: false, dj_link_enabled: false,
            };
            if (command === "get_dj_link_machine_status") return {
              configured: false, credentialReady: false, autoStartArmed: false, bindIp: null, bindPort: null,
              adapterGuid: null, networkGuid: null, credentialGeneration: null, credentialCleanupPending: false, blockReason: null,
            };
            if (command === "list_dj_link_wired_candidates") return [];
            if (command === "begin_project_transaction") return {
              transaction_id: ++mock.transactionId, project_epoch: args.expectedEpoch, project_revision: args.expectedRevision,
              project_checkpoint_hash: args.expectedCheckpointHash, client_operation_id: args.clientOperationId,
              shape_fingerprint: args.shapeFingerprint, schema_version: args.schemaVersion, owner_id: args.ownerId,
              window_label: "main", owner_incarnation: 1, label: args.label, coalesce_key: args.coalesceKey,
            };
            if (command === "commit_project_transaction") {
              const history = { can_undo: true, can_redo: false, undo_depth: mock.transactionId, redo_depth: 0,
                undo_label: "Set Video Layer Blend Mode", redo_label: null, project_epoch: 0, project_revision: 0,
                checkpoint_hash: "", history_generation: mock.transactionId, undo_entry_id: null, undo_checkpoint_hash: null,
                redo_entry_id: null, redo_checkpoint_hash: null };
              return { authority: { project_epoch: 0, project_revision: 0, checkpoint_hash: "", publication_generation: 0,
                publication_kind: "Initial", mapping_replacement_generation: 0, authority_disposition_generation: 0,
                authority_disposition: "Clean", recovery_authority_serial: 0, recovery_authority_last_transition: { kind: "legacy_unknown" },
                path_generation: 0, history_generation: mock.transactionId, current_project_path: null,
                snapshot: readSnapshot(), profiles: [], fixture_groups: [], operator_policy: null, midi_mappings: [],
                osc_mappings: [], dmx_mappings: [], dj_track_triggers: [], history, input_runtime: {} }, history_status: history };
            }
            if (command === "set_video_layer_blend_mode") {
              mock.modes[args.layerId] = args.blendMode;
              return null;
            }
            if (command === "get_snapshot") {
              mock.snapshotReads++;
              const snapshot = readSnapshot();
              return { snapshot, timeline_runtime: {
                transport_epoch: 1, transport_generation: 1,
                loop_runtime: { generation: 0, status: "disabled", a_ms: null, b_ms: null, wrap_count: 0 },
                follow_runtime: { epoch: 1, generation: 0, status: "idle", admission_reason: null, outcome: null,
                  source_timeline_id: null, target_timeline_id: null, elapsed_ms: 0, duration_ms: 0,
                  progress_millis: 0, fault: null, transition_hold_active: false, waiting_for_pedal_start: false },
              } };
            }
            if (command === "get_video_preview_diagnostics") return { isf_stage_errors: [], layer_queues: [], output_decode_previews: [], decoder_diagnostics: {} };
            mock.unexpected.push(command);
            throw new Error(`Unexpected blend-mode fixture command: ${command}`);
          },
        } });
      });
      const geometry = await blend.evaluate((select) => {
        const rect = select.getBoundingClientRect();
        const row = select.closest(".videoLayerItem").getBoundingClientRect();
        const pane = select.closest(".videoMixerLayerPane").getBoundingClientRect();
        return { width: rect.width, height: rect.height, fontSize: getComputedStyle(select).fontSize,
          contained: rect.left >= row.left - 1 && rect.right <= row.right + 1 && rect.left >= pane.left - 1 && rect.right <= pane.right + 1,
          visible: rect.top >= 0 && rect.bottom <= innerHeight,
          documentOverflow: [document.documentElement.scrollWidth - innerWidth, document.documentElement.scrollHeight - innerHeight] };
      });
      assert.equal(geometry.contained, true, "Blend control remains inside its layer and scrollport");
      assert.equal(geometry.visible, true);
      assert.ok(geometry.height >= 28, `Existing 28px control target preserved: ${JSON.stringify(geometry)}`);
      assert.ok(geometry.width >= 80, "Blend text has a usable control width");
      assert.deepEqual(geometry.documentOverflow, [0, 0]);
      for (const [index, mode] of modes.slice(4).entries()) {
        await blend.selectOption(mode);
        await page.waitForFunction(({ mode, count }) => {
          const mock = window.__videoBlendMock;
          return mock.snapshotReads >= count
            && window.__syndocalReadOperatorVjFixtureSnapshot().video.layers.find((layer) => layer.id === 1)?.blend_mode === mode
            && document.querySelector('[data-video-layer-blend="1"]').value === mode;
        }, { mode, count: index + 1 }).catch(async (error) => {
          console.error(JSON.stringify(await page.evaluate(() => ({ mock: window.__videoBlendMock,
            layer: window.__syndocalReadOperatorVjFixtureSnapshot().video.layers.find((layer) => layer.id === 1),
            body: document.body.innerText.slice(-1500) }))));
          throw error;
        });
        assert.equal(await blend.inputValue(), mode);
        assert.equal(await blend.locator("option:checked").textContent(), labels[locale][index + 4]);
        await page.screenshot({ path: resolve(artifacts, `browser-${locale}-${viewport.width}x${viewport.height}-${mode}.png`) });
      }
      const calls = await page.evaluate(() => window.__videoBlendMock.calls.filter((call) => call.command === "set_video_layer_blend_mode"));
      assert.deepEqual(calls.map((call) => [call.args.layerId, call.args.blendMode]), modes.slice(4).map((mode) => [1, mode]), "All six modes invoke exactly once for the selected layer");
      assert.equal(await page.locator("vite-error-overlay").count(), 0);
      assert.deepEqual(await page.evaluate(() => window.__videoBlendMock.unexpected), [], "Native fixture admits only known commands");
      assert.equal((await page.locator("body").innerText()).includes("Unexpected blend-mode fixture command"), false);
      assert.deepEqual(pageErrors, [], "No uncaught browser errors");
      assert.deepEqual(consoleErrors, [], "No browser console errors or warnings");
      await writeFile(resolve(artifacts, `browser-${locale}-${viewport.width}x${viewport.height}.json`), JSON.stringify({
        url: page.url(), title: await page.title(), viewport, locale, geometry, calls, pageErrors, consoleErrors, faviconErrors,
        browser: "Playwright; Browser plugin not available", boundary: "Native IPC mocked; compositor and native Display not exercised",
      }, null, 2));
      console.log(`PASS video-blend-modes ${locale} ${viewport.width}x${viewport.height}: 10 labels, 6 commands + authoritative refresh, contained ${geometry.width}x${geometry.height}, no app errors, favicon404=${faviconErrors.length}`);
      await page.close();
    }
  }
} finally {
  await browser?.close();
  if (vite.exitCode === null) {
    const exited = new Promise((done) => vite.once("exit", done));
    vite.kill();
    await Promise.race([exited, sleep(2000)]);
  }
}
