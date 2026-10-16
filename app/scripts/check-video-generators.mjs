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
const artifacts = resolve(appRoot, "../target/qa/generators");
const port = Number(process.env.VIDEO_GENERATORS_PORT ?? 5199);
const origin = `http://127.0.0.1:${port}`;
const sleep = (ms) => new Promise((done) => setTimeout(done, ms));
const chromePath = [process.env.CHROME_PATH,
  "C:/Program Files/Google/Chrome/Application/chrome.exe",
  "C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe",
].find((path) => path && existsSync(path));
const generators = ["Solid", "LinearGradient", "Noise"];
const labels = {
  en: ["Solid colour", "Linear gradient", "Animated noise"],
  ja: ["単色", "線形グラデーション", "アニメーションノイズ"],
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
      const create = page.locator("[data-video-generator-create]");
      await create.waitFor({ state: "visible" });
      await create.locator("summary").click();
      for (let index = 0; index < generators.length; index++) {
        await page.waitForFunction(({ kind, expected }) => document.querySelector(`[data-video-generator-add="${kind}"]`)?.textContent === expected,
          { kind: generators[index], expected: labels[locale][index] });
      }
      assert.ok((await page.locator(".videoLayerList").innerText()).length > 50, "Meaningful layer content rendered");

      // Mock only native IPC. The App callback, transaction facade, refresh and
      // Solid control execute unchanged; this does not test CPU/GPU rendering.
      await page.evaluate(() => {
        const mock = { calls: [], created: [], edits: {}, modes: {}, snapshotReads: 0, transactionId: 0, unexpected: [] };
        const readSnapshot = () => {
          const snapshot = structuredClone(window.__syndocalReadOperatorVjFixtureSnapshot());
          for (const layer of snapshot.video.layers) {
            if (mock.modes[layer.id]) layer.blend_mode = mock.modes[layer.id];
          }
          snapshot.video.layers = [...mock.created, ...snapshot.video.layers.filter(layer => layer.id < 100)];
          for (const layer of snapshot.video.layers) {
            if (mock.modes[layer.id]) layer.blend_mode = mock.modes[layer.id];
            if (mock.edits[layer.id]) layer.source.kind = { Generator: mock.edits[layer.id] };
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
                undo_label: "Edit Video Generator", redo_label: null, project_epoch: 0, project_revision: 0,
                checkpoint_hash: "", history_generation: mock.transactionId, undo_entry_id: null, undo_checkpoint_hash: null,
                redo_entry_id: null, redo_checkpoint_hash: null };
              return { authority: { project_epoch: 0, project_revision: 0, checkpoint_hash: "", publication_generation: 0,
                publication_kind: "Initial", mapping_replacement_generation: 0, authority_disposition_generation: 0,
                authority_disposition: "Clean", recovery_authority_serial: 0, recovery_authority_last_transition: { kind: "legacy_unknown" },
                path_generation: 0, history_generation: mock.transactionId, current_project_path: null,
                snapshot: readSnapshot(), profiles: [], fixture_groups: [], operator_policy: null, midi_mappings: [],
                osc_mappings: [], dmx_mappings: [], dj_track_triggers: [], history, input_runtime: {} }, history_status: history };
            }
            if (command === "add_video_generator_layer") {
              const layer = structuredClone(window.__syndocalReadOperatorVjFixtureSnapshot().video.layers[0]);
              layer.id = 100 + mock.created.length;
              layer.label = args.label;
              layer.source = { kind: { Generator: args.generator }, name: null, path: null, codec: null, metadata: null };
              mock.created.unshift(layer);
              return layer.id;
            }
            if (command === "set_video_layer_generator") {
              if (mock.holdGenerator) await new Promise((done) => { mock.releaseGenerator = done; });
              mock.edits[args.layerId] = args.generator;
              return null;
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
            throw new Error(`Unexpected generator fixture command: ${command}`);
          },
        } });
      });
      const geometry = [];
      const commandCount = () => page.evaluate(() => window.__videoBlendMock.calls.filter(call => call.command === "set_video_layer_generator").length);
      for (const [index, kind] of generators.entries()) {
        const layerId = 100 + index;
        await create.locator(`[data-video-generator-add="${kind}"]`).click();
        await page.waitForFunction(({ layerId, kind }) => {
          const layer = window.__syndocalReadOperatorVjFixtureSnapshot().video.layers.find(layer => layer.id === layerId);
          return layer && kind in layer.source.kind.Generator;
        }, { layerId, kind });
        const editor = page.locator(`[data-video-generator-editor="${layerId}"]`);
        await editor.locator("summary").click();
        const edits = kind === "Solid" ? [["color", "#123456"], ["color-alpha", "201"]]
          : [["color_a", "#234567"], ["color_a-alpha", "202"], ["color_b", "#abcdef"], ["color_b-alpha", "203"],
            ...(kind === "LinearGradient" ? [["angle_degrees", "123"]] : [["seed", "4294967295"], ["scale", "73"], ["speed", "317"]])];
        const bounds = [];
        for (const [field, value] of edits) {
          const input = editor.locator(`[data-generator-field="${field}"]`);
          await input.scrollIntoViewIfNeeded();
          await input.click({ trial: true });
          const rect = await input.evaluate(element => {
            const pane = element.closest(".videoMixerLayerPane").getBoundingClientRect();
            const box = element.getBoundingClientRect();
            return { width: box.width, height: box.height, fontSize: getComputedStyle(element).fontSize,
              contained: box.left >= pane.left - 1 && box.right <= pane.right + 1,
              visible: box.top >= pane.top - 1 && box.bottom <= pane.bottom + 1 };
          });
          assert.equal(rect.contained, true, "Parameters remain inside layer scrollport");
          assert.equal(rect.visible, true, "Each parameter is visible within the scrollport when edited");
          assert.ok(rect.height >= 28, "Existing control target preserved");
          assert.ok(rect.width >= 70, "Parameter control has usable width");
          bounds.push({ field, ...rect });
          const before = await commandCount();
          const delayed = kind === "Solid" && field === "color";
          if (delayed) await page.evaluate(() => { window.__videoBlendMock.holdGenerator = true; });
          await input.fill(value);
          if (!value.startsWith("#")) await input.blur();
          if (delayed) {
            await page.waitForFunction(() => typeof window.__videoBlendMock.releaseGenerator === "function");
            for (const control of await editor.locator("input").all()) {
              assert.equal(await control.isDisabled(), true, "Pending publication prevents stale full-parameter edits");
            }
            assert.equal(await commandCount(), before + 1);
            await page.evaluate(() => {
              window.__videoBlendMock.holdGenerator = false;
              window.__videoBlendMock.releaseGenerator();
            });
          }
          await page.waitForFunction(({ layerId, kind, field, value, before }) => {
            const mock = window.__videoBlendMock;
            const actual = window.__syndocalReadOperatorVjFixtureSnapshot().video.layers.find(layer => layer.id === layerId)?.source.kind.Generator[kind];
            const key = field.replace(/-alpha$/, "");
            const expected = field.endsWith("-alpha") ? Number(value) : value.startsWith("#")
              ? [parseInt(value.slice(1, 3), 16), parseInt(value.slice(3, 5), 16), parseInt(value.slice(5, 7), 16)] : Number(value);
            const observed = field.endsWith("-alpha") ? actual?.[key]?.[3] : value.startsWith("#") ? actual?.[key]?.slice(0, 3) : actual?.[key];
            return mock.calls.filter(call => call.command === "set_video_layer_generator").length === before + 1
              && JSON.stringify(observed) === JSON.stringify(expected);
          }, { layerId, kind, field, value, before }).catch(async error => {
            console.error(JSON.stringify(await page.evaluate(() => ({ calls: window.__videoBlendMock.calls.filter(call => ["set_video_layer_generator", "add_video_generator_layer"].includes(call.command)), snapshot: window.__syndocalReadOperatorVjFixtureSnapshot().video.layers.slice(0, 3).map(layer => ({id:layer.id,kind:layer.source.kind})) }))));
            throw error;
          });
          assert.equal(await input.inputValue(), value, "Published parameters are reflected by the control");
          await page.waitForFunction(layerId => !document.querySelector(`[data-video-generator-editor="${layerId}"] fieldset`).disabled, layerId);
        }
        geometry.push({ kind, bounds });
        await page.screenshot({ path: resolve(artifacts, `browser-${locale}-${viewport.width}x${viewport.height}-${kind}.png`) });
        // Generator layers use the same blend control and authored callback.
        const blend = page.locator(`[data-video-layer-blend="${layerId}"]`);
        await blend.selectOption("Overlay");
        await page.waitForFunction(layerId => window.__syndocalReadOperatorVjFixtureSnapshot().video.layers.find(layer => layer.id === layerId)?.blend_mode === "Overlay", layerId);
        await editor.locator("summary").click();
      }
      const overflow = await page.evaluate(() => [document.documentElement.scrollWidth - innerWidth, document.documentElement.scrollHeight - innerHeight]);
      assert.deepEqual(overflow, [0, 0]);
      const calls = await page.evaluate(() => window.__videoBlendMock.calls.filter(call => ["add_video_generator_layer", "set_video_layer_generator", "set_video_layer_blend_mode"].includes(call.command)));
      assert.equal(calls.filter(call => call.command === "add_video_generator_layer").length, 3);
      assert.equal(calls.filter(call => call.command === "set_video_layer_generator").length, 14);
      assert.equal(calls.filter(call => call.command === "set_video_layer_blend_mode").length, 3);
      assert.equal(await page.locator("vite-error-overlay").count(), 0);
      assert.deepEqual(await page.evaluate(() => window.__videoBlendMock.unexpected), [], "Native fixture admits only known commands");
      assert.deepEqual(pageErrors, [], "No uncaught browser errors");
      assert.deepEqual(consoleErrors, [], "No browser console errors or warnings");
      await writeFile(resolve(artifacts, `browser-${locale}-${viewport.width}x${viewport.height}.json`), JSON.stringify({
        url: page.url(), title: await page.title(), viewport, locale, geometry, overflow, calls, pageErrors, consoleErrors, faviconErrors,
        browser: "Playwright; Browser plugin not available", boundary: "Native IPC mocked; compositor and native Display not exercised",
      }, null, 2));
      console.log(`PASS video-generators ${locale} ${viewport.width}x${viewport.height}: 3 creates, 14 parameter edits, 3 blends + authoritative refresh; contained controls, no app errors, favicon404=${faviconErrors.length}`);
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
