import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { createRequire } from "node:module";
import { pathToFileURL } from "node:url";
import ts from "typescript";
const require = createRequire(import.meta.url);
const solidUrl = pathToFileURL(require.resolve("solid-js/dist/solid.js")).href;
const { createRoot, createSignal } = await import(solidUrl);
const compiled = new Map();
async function compile(url) {
  if (compiled.has(url.href)) return compiled.get(url.href);
  let code = ts.transpileModule(await readFile(url, "utf8"), { compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 } }).outputText
    .replaceAll('from "solid-js"', `from "${solidUrl}"`);
  for (const match of [...code.matchAll(/from\s+"(\.[^"]+)"/g)]) code = code.replace(match[0], `from "${await compile(new URL(`${match[1]}.ts`, url))}"`);
  const result = `data:text/javascript;base64,${Buffer.from(code).toString("base64")}`;
  compiled.set(url.href, result); return result;
}
const { createLiveVideoMonitorController } = await import(await compile(new URL("../src/createLiveVideoMonitorController.ts", import.meta.url)));
const { createLiveVideoOutputHealthController } = await import(await compile(new URL("../src/createLiveVideoOutputHealthController.ts", import.meta.url)));
const { createMonitorPressurePolicy, createRecentMissedTracker, outputMissedWarningThreshold, monitorTargetFps, monitorQualityFromUnknown } = await import(await compile(new URL("../src/liveVideoMonitorPolicy.ts", import.meta.url)));
assert.deepEqual(monitorTargetFps("smooth", true), { program: 30, preview: 30 });
assert.deepEqual(monitorTargetFps("smooth", false), { program: 30, preview: 15 });
assert.deepEqual(monitorTargetFps("balanced", true), { program: 15, preview: 15 });
assert.deepEqual(monitorTargetFps("low-cpu", true), { program: 10, preview: 5 });
assert.equal(monitorQualityFromUnknown("retired"), "smooth");
let now = 1, timerId = 0, closed = 0, decoded = 0;
const timers = new Map();
Object.defineProperty(globalThis, "performance", { value: { now: () => now }, configurable: true });
globalThis.window = { setTimeout(fn, delay) { const id = ++timerId; timers.set(id, { fn, at: now + delay }); return id; }, clearTimeout(id) { timers.delete(id); } };
globalThis.document = Object.assign(new EventTarget(), { hidden: false });
globalThis.createImageBitmap = async blob => { assert.equal(blob.type, "image/jpeg"); decoded++; return { width: 2, height: 1, close() { closed++; } }; };
URL.createObjectURL = () => assert.fail("Canvas presentation cannot allocate Blob URLs");
URL.revokeObjectURL = () => assert.fail("Canvas presentation cannot revoke Blob URLs");
const flush = async () => { for (let i = 0; i < 12; i++) await Promise.resolve(); };
async function advance(ms) {
  const until = now + ms; let iterations = 0;
  while (true) {
    const next = [...timers].sort((a, b) => a[1].at - b[1].at)[0];
    if (!next || next[1].at > until + 0.00001) break;
    assert(++iterations < 2_000, "no timer spin or catch-up burst");
    timers.delete(next[0]); now = next[1].at; next[1].fn(); await flush();
  }
  now = until; await flush();
}
function packet(kind, { busy = false, renderUs = 100, encodeUs = 100 } = {}) {
  const bytes = new Uint8Array(busy ? 40 : 44), view = new DataView(bytes.buffer);
  bytes.set([83, 89, 76, 86, 2, busy ? 1 : 0, kind === "program" ? 0 : 1, 0]);
  view.setUint16(32, 2, true); view.setUint16(34, 1, true); view.setUint32(36, busy ? 0 : 4, true);
  view.setUint32(24, renderUs, true); view.setUint32(28, encodeUs, true);
  if (!busy) bytes.set([255, 216, 255, 217], 40); return bytes;
}
function fixture({ auto = true, backendAvailable = () => true, respond = request => packet(request.args.monitorKind) } = {}) {
  let controller, dispose, setActive, setPreviewId, setQuality, setPreviewActive, setOutputs, setSnapshot, setHealthError, setPreviewTransport;
  const requests = [], outstanding = { program: 0, preview: 0 }, max = { program: 0, preview: 0 };
  createRoot(cleanup => {
    dispose = cleanup;
    const [active, updateActive] = createSignal(true); setActive = updateActive;
    const [previewId, updatePreviewId] = createSignal(9); setPreviewId = updatePreviewId;
    const [previewTransport, updatePreviewTransport] = createSignal({ generation: 1, position_ms: 0 }); setPreviewTransport = updatePreviewTransport;
    const [quality, updateQuality] = createSignal("smooth"); setQuality = updateQuality;
    const [previewActive, updatePreviewActive] = createSignal(true); setPreviewActive = updatePreviewActive;
    const [outputs, updateOutputs] = createSignal([]); setOutputs = updateOutputs;
    const [snapshot, updateSnapshot] = createSignal({ layerCount: 3 }); setSnapshot = updateSnapshot;
    const [healthError, updateHealthError] = createSignal(null); setHealthError = updateHealthError;
    controller = createLiveVideoMonitorController({ backendAvailable, active, layerCount: () => snapshot().layerCount, previewLayerId: previewId,
      previewTransportGeneration: () => previewTransport().generation,
      programOutputId: () => 7, quality, previewActive, outputHealth: outputs, outputHealthError: healthError,
      invoke(command, args) {
        assert.equal(command, "get_live_video_monitor_frame"); assert.equal(args.pixelFormat, "jpeg");
        const kind = args.monitorKind; outstanding[kind]++; max[kind] = Math.max(max[kind], outstanding[kind]);
        return new Promise((resolve, reject) => {
          const request = { args, at: now, resolve(value) { outstanding[kind]--; resolve(value); }, reject(error) { outstanding[kind]--; reject(error); } };
          requests.push(request); if (auto) request.resolve(respond(request));
        });
      },
    });
  });
  return { controller, dispose, setActive, setPreviewId, setQuality, setPreviewActive, setOutputs, setSnapshot, setHealthError, setPreviewTransport, requests, max };
}
const count = (fixture, kind, since) => fixture.requests.filter(request => request.args.monitorKind === kind && request.at >= since).length;
{
  let backend = false;
  const f = fixture({ backendAvailable: () => backend });
  await advance(100); assert.equal(f.requests.length, 0);
  backend = true;
  f.setActive(false); f.setActive(true);
  await advance(100);
  assert(f.requests.some(request => request.args.monitorKind === "program"), "a post-mount native bridge is rechecked on desk activation");
  assert.equal(f.controller.program().status, "live");
  f.dispose();
}
{
  const f = fixture(); const origin = now; await advance(5_000);
  assert(count(f, "program", origin) >= 149 && count(f, "program", origin) <= 151);
  assert(count(f, "preview", origin) >= 149 && count(f, "preview", origin) <= 151);
  assert(f.controller.program().frame && f.controller.preview().frame, "both buses decoded onto owned canvas frames");
  assert(f.controller.program().fps > 29.9 && f.controller.program().fps < 30.1, "reported monitor FPS measures the rolling delivered count");
  assert.equal(f.controller.program().frameUrl, null);
  assert.deepEqual(f.max, { program: 1, preview: 1 });
  f.setPreviewActive(false); const passive = now; await advance(2_000);
  assert(count(f, "preview", passive) >= 29 && count(f, "preview", passive) <= 31);
  f.setQuality("low-cpu"); const low = now; await advance(2_000);
  assert(count(f, "program", low) >= 19 && count(f, "program", low) <= 21);
  assert(count(f, "preview", low) >= 9 && count(f, "preview", low) <= 11);
  document.hidden = true; document.dispatchEvent(new Event("visibilitychange")); const paused = f.requests.length;
  await advance(5_000); assert.equal(f.requests.length, paused, "hidden document stops both monitor buses");
  document.hidden = false; document.dispatchEvent(new Event("visibilitychange")); await advance(40);
  assert(f.requests.length > paused); f.dispose(); assert.equal(timers.size, 0);
}
{
  const f = fixture({ auto: false }); await advance(20);
  const existing = f.requests.length;
  for (let index = 0; index < 20; index++) f.setSnapshot({ layerCount: 3 });
  for (let index = 0; index < 20; index++) f.setPreviewTransport({ generation: 1, position_ms: index * 10 });
  await advance(1); assert.equal(f.requests.length, existing, "unchanged snapshot values cannot mint request slots");
  f.requests[0].resolve(packet("program")); f.requests[1].resolve(packet("preview")); await flush();
  assert.equal(f.controller.program().status, "live", "snapshot object churn cannot retire a valid same-input frame");
  assert.equal(f.controller.preview().status, "live", "position polling does not invalidate same-generation Preview frames");
  f.dispose();
}
{
  let holdPreviewDecode, calls = 0;
  const previousDecode = globalThis.createImageBitmap;
  globalThis.createImageBitmap = blob => ++calls === 2
    ? new Promise(done => { holdPreviewDecode = done; }) : previousDecode(blob);
  const f = fixture(); await advance(20);
  assert(holdPreviewDecode, "Preview bitmap decode remains in flight");
  f.setPreviewTransport({ generation: 2, position_ms: 5_000 });
  decoded++; holdPreviewDecode({ width: 2, height: 1, close() { closed++; } }); await flush();
  assert.equal(f.controller.preview().frame, null, "same-layer seek/restage fences the pre-seek bitmap after asynchronous decoding");
  f.dispose(); globalThis.createImageBitmap = previousDecode;
}
{
  let holdDecode;
  const previousDecode = globalThis.createImageBitmap;
  globalThis.createImageBitmap = () => new Promise(done => { holdDecode = done; });
  const f = fixture(); await advance(0);
  assert(holdDecode); f.setActive(false);
  decoded++; holdDecode({ width: 2, height: 1, close() { closed++; } }); await flush();
  assert.equal(f.controller.program().frame, null, "generation fence also covers asynchronous browser decoding");
  f.dispose(); globalThis.createImageBitmap = previousDecode;
}
{
  const f = fixture({ auto: false }); await advance(20);
  assert.equal(f.requests.length, 2, "one request per bus may be in flight");
  await advance(10_000); assert.equal(f.requests.length, 2, "hung requests cannot overlap their own bus or queue work");
  f.setPreviewId(10); f.requests[1].resolve(packet("preview")); await flush();
  assert.equal(f.controller.preview().frame, null, "old selection reply is fenced");
  f.requests[0].resolve(packet("program")); await flush(); await advance(40);
  assert(f.requests.length <= 4, "elapsed slots are dropped, never replayed");
  assert.deepEqual(f.max, { program: 1, preview: 1 });
  f.dispose(); for (const request of f.requests.slice(2)) request.resolve(packet(request.args.monitorKind)); await flush();
  assert.equal(timers.size, 0);
}
{
  const f = fixture({ respond: request => packet(request.args.monitorKind, { busy: true }) });
  await advance(2_000);
  assert(f.controller.program().throttled || f.controller.preview().throttled, "repeated native busy replies protect output");
  assert(f.requests.length < 110, "busy pressure lowers monitor request rate");
  f.dispose(); assert.equal(timers.size, 0);
}
{
  const f = fixture({ respond: request => packet(request.args.monitorKind, { renderUs: 28_000, encodeUs: 2_000 }) });
  await advance(300); assert(f.controller.program().throttled || f.controller.preview().throttled, "render/encode cost lowers monitor rate"); f.dispose();
}
{
  const f = fixture({ respond: request => packet(request.args.monitorKind, request.args.monitorKind === "program"
    ? { renderUs: 28_000, encodeUs: 2_000 } : { renderUs: 100, encodeUs: 100 }) });
  await advance(300); assert(f.controller.program().throttled || f.controller.preview().throttled,
    "fast Preview cannot reset consecutive slow Program observations"); f.dispose();
}
{
  const policy = createMonitorPressurePolicy();
  const output = Object.freeze({ outputId: 1, kind: "display", targetFps: 30, deliveredFps: 30, sampleCount: 60, frameTimeP95Ms: 34, droppedFrames: 0, lastFrameAgeMs: 3 });
  policy.observeOutputs(0, [output]); assert.equal(policy.intervalMs(0, 30), 1_000 / 30);
  policy.observeOutputs(1_000, [{ ...output, frameTimeP95Ms: 42 }]);
  assert(policy.intervalMs(1_000, 30) > 1_000 / 30, "rising output p95 protects output even below absolute cutoff");
  policy.observeOutputs(2_000, [output]); assert(policy.throttled());
  policy.intervalMs(6_000, 30); assert(!policy.throttled(), "recovery requires five healthy seconds");
  policy.observeOutputs(7_000, [{ ...output, lastFrameAgeMs: 500 }]);
  assert(policy.intervalMs(7_000, 30) > 1_000 / 30, "stalled send triggers protection before a new completed percentile");
  assert.equal(policy.outputPauseMs(7_000), 1_000, "severe output stall pauses monitor requests for a bounded second");
  assert.equal(policy.outputPauseMs(8_000), 0, "pause expires and permits a recovery probe");
  policy.observeOutputs(8_000, [{ ...output, droppedFrames: 1 }]);
  policy.intervalMs(14_000, 30); assert(!policy.throttled(), "one missed slot per poll is jitter, not contention");
  policy.observeOutputs(15_000, [{ ...output, droppedFrames: 4 }]); assert(policy.throttled(), "a burst of missed slots triggers protection");
}
{
  for (const sampleCount of [0, 1]) {
    const policy = createMonitorPressurePolicy();
    policy.observeOutputs(0, [{ outputId: 1, kind: "display", targetFps: 60,
      deliveredFps: 0, sampleCount, frameTimeP95Ms: 0, droppedFrames: 0, lastFrameAgeMs: 500 }]);
    assert(policy.throttled(), "a startup stall protects output before two completed intervals");
    assert.equal(policy.outputPauseMs(0), 1_000, "startup stalls pause monitor IPC");
  }
}
{
  const f = fixture(); await advance(1_100);
  assert(f.controller.program().fps > 29);
  f.setOutputs([{ outputId: 7, kind: "display", targetFps: 30, deliveredFps: 1, sampleCount: 60, frameTimeP95Ms: 100, droppedFrames: 90, lastFrameAgeMs: 500 }]);
  const before = f.requests.length; await advance(900);
  assert.equal(f.requests.length, before, "severely unhealthy output yields no monitor IPC during the pause");
  f.setOutputs([{ outputId: 7, kind: "display", targetFps: 30, deliveredFps: 1, sampleCount: 60, frameTimeP95Ms: 100, droppedFrames: 90, lastFrameAgeMs: 1_400 }]);
  await advance(200); f.setOutputs([]);
  assert.equal(f.controller.program().fps, 0, "held last frame never reports a frozen stale high FPS");
  f.setHealthError("Output health unavailable"); await advance(2_000);
  assert(f.controller.program().throttled, "telemetry failure protects output instead of restoring full monitor load");
  f.dispose();
}
{
  let controller, dispose;
  const pending = [];
  createRoot(cleanup => {
    dispose = cleanup;
    controller = createLiveVideoOutputHealthController({ backendAvailable: () => true, invoke(command) {
      assert.equal(command, "get_live_video_output_health"); return new Promise((resolve, reject) => pending.push({ resolve, reject }));
    } });
  });
  await advance(0); await advance(10_000); assert.equal(pending.length, 1, "slow output telemetry stays single-flight");
  document.hidden = true; document.dispatchEvent(new Event("visibilitychange"));
  pending[0].resolve([{ outputId: 1 }]); await flush(); assert.deepEqual(controller.outputs(), [], "hidden-generation telemetry reply is fenced");
  document.hidden = false; document.dispatchEvent(new Event("visibilitychange")); await advance(0);
  assert.equal(pending.length, 2); pending[1].reject(new Error("read failed")); await flush(); assert.equal(controller.error(), "Output health unavailable");
  await advance(1_000); assert.equal(pending.length, 3); pending[2].resolve([]); await flush(); assert.equal(controller.error(), null, "telemetry recovers after a read failure");
  dispose(); assert.equal(timers.size, 0);
}
{
  const track = createRecentMissedTracker();
  const out = (droppedFrames, outputId = 1) => ({ outputId, kind: "display", deliveredFps: 60, frameTimeP50Ms: 16, frameTimeP95Ms: 17, frameTimeP99Ms: 18, droppedFrames, targetFps: 60, deliveredFrames: 100, sampleCount: 60, measurementBoundary: "present", lastFrameAgeMs: 2 });
  assert.equal(track(0, [out(69)])[0].recentMissedFrames, 0, "deficit present at first sight (startup) is the baseline, not a recent miss");
  assert.equal(track(1_000, [out(69)])[0].recentMissedFrames, 0);
  assert.equal(track(2_000, [out(74)])[0].recentMissedFrames, 5);
  assert(track(3_000, [out(74)])[0].recentMissedFrames <= outputMissedWarningThreshold, "five missed intervals stay below the warning threshold");
  assert.equal(track(9_000, [out(74)])[0].recentMissedFrames, 5, "inside the 10 s window");
  assert.equal(track(12_500, [out(74)])[0].recentMissedFrames, 0, "misses age out of the window while the cumulative total stays");
  assert.equal(track(12_500, [out(74)])[0].droppedFrames, 74, "cumulative total is preserved for the tooltip");
  assert.equal(track(13_000, [out(3)])[0].recentMissedFrames, 0, "a restarted output (smaller counter) forgets its history");
  assert.equal(track(14_000, [out(12)])[0].recentMissedFrames, 9);
  assert.equal(track(15_000, [])[0], undefined);
  assert.equal(track(16_000, [out(12)])[0].recentMissedFrames, 0, "a removed output is forgotten and re-baselined");
}
assert.equal(decoded, closed, "every replaced/stale/disposed ImageBitmap is closed exactly once");
console.log("PASS smooth/balanced/low-CPU rates; per-bus pipelining/single-flight; no catch-up; generation, visibility and disposal fences; canvas bitmap ownership; busy/render/encode/output-p95/missed-slot/stall adaptive protection and hysteresis.");
