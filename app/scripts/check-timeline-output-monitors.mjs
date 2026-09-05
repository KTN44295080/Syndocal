import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { createRequire } from "node:module";
import { pathToFileURL } from "node:url";
import ts from "typescript";

await import("./check-live-video-monitor-packet.mjs");
const require = createRequire(import.meta.url);
const backend = await readFile(new URL("../src-tauri/src/main.rs", import.meta.url), "utf8");
const commandStart = backend.indexOf("async fn get_live_video_monitor_frame(");
const workerStart = backend.indexOf("fn render_live_video_monitor_frame(", commandStart);
assert(commandStart >= 0 && workerStart > commandStart, "monitor command must have an asynchronous worker boundary");
const monitorCommand = backend.slice(commandStart, workerStart);
assert.match(monitorCommand, /spawn_blocking\(move \|\| \{\s*let state = app\.state::<AppState>\(\);\s*render_live_video_monitor_frame\(&state, args\)\s*\}\)\s*\.await/s,
  "capture, decode, composition and encoding run in the worker with owned app/request lifetime");
assert.doesNotMatch(monitorCommand, /warm_output_decode_queue|render_output_preview_with_effects|encode_live_video_monitor_jpeg/,
  "heavy monitor work cannot return to the WebView callback thread");
const monitorWorker = backend.slice(workerStart, backend.indexOf("#[cfg(test)]", workerStart));
const programWorker = monitorWorker.slice(monitorWorker.indexOf("LiveVideoMonitorKind::Program =>"), monitorWorker.indexOf("LiveVideoMonitorKind::Preview =>"));
assert.match(programWorker, /capture_video_monitor_snapshot\(&state\.engine\)\?/,
  "Program captures only its owned video render inputs");
assert.doesNotMatch(programWorker, /capture_video_output_preview_effect_snapshot|warm_output_decode_queue|engine\.snapshot\(/,
  "Program monitor neither clones the entire engine nor fetches each decoded frame twice");
assert.doesNotMatch(monitorWorker, /decodeBudget|decode_budget/);
assert.match(monitorWorker, /encode_live_video_monitor_jpeg[\s\S]*?if program_render_epoch\s*\.is_some_and\(\|epoch\| state\.engine\.output_ownership_status\(\)\.epoch != epoch\)[\s\S]*?return Err\("Output changed while rendering; waiting for a current frame\."\.to_string\(\)\);[\s\S]*?if let Some\(rendered\)/,
  "after encoding, a retired Program epoch yields an error that clears the old frame before image publication");
const solidUrl = pathToFileURL(require.resolve("solid-js/dist/solid.js")).href;
const { createRoot, createSignal } = await import(solidUrl);
const compiled = new Map();
async function compile(url) {
  if (compiled.has(url.href)) return compiled.get(url.href);
  let code = ts.transpileModule(await readFile(url, "utf8"), {
    compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 },
  }).outputText.replaceAll('from "solid-js"', `from "${solidUrl}"`);
  for (const match of [...code.matchAll(/from\s+"(\.[^"]+)"/g)]) {
    code = code.replace(match[0], `from "${await compile(new URL(`${match[1]}.ts`, url))}"`);
  }
  const result = `data:text/javascript;base64,${Buffer.from(code).toString("base64")}`;
  compiled.set(url.href, result);
  return result;
}
const { createTimelineOutputMonitorController, timelineOutputMonitorSize, TIMELINE_OUTPUT_FRAME_MAX_AGE_MS } = await import(
  await compile(new URL("../src/createTimelineOutputMonitorController.ts", import.meta.url)),
);
assert.deepEqual(timelineOutputMonitorSize(1920, 1080), { width: 320, height: 180 });
assert.deepEqual(timelineOutputMonitorSize(1080, 1920), { width: 101, height: 180 });
assert.deepEqual(timelineOutputMonitorSize(2048, 512), { width: 320, height: 80 });
assert.equal(timelineOutputMonitorSize(0, 100), null);

let now = 0, timerId = 0;
let timerLateness = () => 0;
const timers = new Map();
Object.defineProperty(globalThis, "performance", { value: { now: () => now }, configurable: true });
globalThis.window = {
  setTimeout(fn, delay) { const id = ++timerId; timers.set(id, { fn, at: now + delay + timerLateness() }); return id; },
  clearTimeout(id) { timers.delete(id); },
};
globalThis.document = Object.assign(new EventTarget(), { hidden: false });
URL.createObjectURL = () => assert.fail("Timeline raw frames must not create Blob URLs");
URL.revokeObjectURL = () => assert.fail("Timeline raw frames must not use Blob URL lifecycle");
const flush = async () => { await Promise.resolve(); await Promise.resolve(); };
async function advance(ms) {
  const until = now + ms;
  let iterations = 0;
  while (true) {
    const next = [...timers].sort((a, b) => a[1].at - b[1].at)[0];
    if (!next || next[1].at > until) break;
    assert(++iterations < 200, "scheduler must not busy loop");
    timers.delete(next[0]); now = next[1].at; next[1].fn(); await flush();
  }
  now = until; await flush();
}
const output = (id, width = 1920, height = 1080) => ({ id, width, height, label: `Output ${id}`,
  enabled: true, kind: "Display", composition_id: id, fullscreen: false, opacity: 1, blackout: false, mapping: {} });
const packet = (request, busy = false) => {
  const length = busy ? 0 : request.args.width * request.args.height * 4;
  const bytes = new Uint8Array(40 + length), view = new DataView(bytes.buffer);
  bytes.set([0x53, 0x59, 0x4c, 0x56, 2, busy ? 1 : 0, 0, 1]);
  view.setUint16(32, request.args.width, true); view.setUint16(34, request.args.height, true);
  view.setUint32(36, length, true);
  if (!busy) for (let index = 40; index < bytes.length; index += 4) bytes.set([198, 67, 114, 255], index);
  return bytes;
};
function fixture(initial = [output(1), output(2, 1080, 1920)]) {
  let dispose, setOutputs, setActive, setProjectEpoch, controller, outstanding = 0, maxConcurrent = 0;
  const requests = [];
  createRoot((cleanup) => {
    dispose = cleanup;
    const [outputs, updateOutputs] = createSignal(initial);
    const [active, updateActive] = createSignal(true);
    const [projectEpoch, updateProjectEpoch] = createSignal(1);
    setProjectEpoch = updateProjectEpoch;
    setOutputs = updateOutputs; setActive = updateActive;
    controller = createTimelineOutputMonitorController({ outputs, active, projectEpoch, backendAvailable: () => true,
      invoke(command, args) {
        outstanding += 1; maxConcurrent = Math.max(maxConcurrent, outstanding);
        return new Promise((resolve, reject) => requests.push({ command, args, at: now,
          resolve(value) { outstanding -= 1; resolve(value); }, reject(error) { outstanding -= 1; reject(error); },
        }));
      },
    });
  });
  return { controller, dispose, setOutputs, setActive, setProjectEpoch, requests, maxConcurrent: () => maxConcurrent };
}

const f = fixture();
await advance(0);
assert.equal(f.requests.length, 1);
assert.equal(f.requests[0].command, "get_live_video_monitor_frame");
assert.equal(f.requests[0].args.monitorKind, "program");
assert.equal(f.requests[0].args.pixelFormat, "rgba");
await advance(1_000);
assert.equal(f.requests.length, 1, "slow decoder cannot overlap another output");
for (let i = 0; i < 20; i += 1) f.setOutputs([output(1), output(2, 1080, 1920)]);
const initialPacket=packet(f.requests[0]);
f.requests[0].resolve(initialPacket); await flush();
assert.equal(f.controller.states().get(1).frame.data.buffer,initialPacket.buffer,"RGBA state retains the IPC buffer without a second pixel copy");
assert.equal(f.controller.states().get(1).status, "live", "equivalent snapshots cannot starve responses");
await advance(0);
assert.equal(f.requests[1].args.outputId, 2, "all outputs get round-robin turns");
assert.equal(f.requests[1].args.width, 101);
f.requests[1].resolve(packet(f.requests[1])); await flush();
await advance(16);
assert.equal(f.requests.length, 2, "aggregate start rate is bounded");
await advance(1);
assert.equal(f.requests[2].args.outputId, 1);
f.setOutputs([output(1, 2048, 512), output(2, 1080, 1920)]);
assert.equal(f.controller.states().get(1).frame, null, "resolution changes clear previous image immediately");
f.requests[2].resolve(packet(f.requests[2])); await flush();
assert.equal(f.controller.states().get(1).frame, null, "late old-resolution frame is rejected");
await advance(100);
const removedRequest = f.requests.at(-1);
assert.equal(removedRequest.args.outputId, 2);
f.setOutputs([output(1, 2048, 512)]);
removedRequest.resolve(packet(removedRequest)); await flush();
assert(!f.controller.states().has(2), "removed output cannot reappear from delayed response");
await advance(100);
const hiddenRequest = f.requests.at(-1);
document.hidden = true; document.dispatchEvent(new Event("visibilitychange"));
hiddenRequest.resolve(packet(hiddenRequest)); await flush();
const hiddenCount = f.requests.length;
await advance(1_000);
assert.equal(f.requests.length, hiddenCount, "hidden page stops requests");
assert.equal(f.controller.states().get(1).frame, null);
document.hidden = false; document.dispatchEvent(new Event("visibilitychange"));
await advance(0);
const failing = f.requests.at(-1);
failing.reject(new Error("decoder unavailable")); await flush();
assert.equal(f.controller.states().get(1).status, "error");
await advance(249);
assert.equal(f.requests.at(-1), failing, "error retry respects backoff");
await advance(1);
const recovered = f.requests.at(-1);
recovered.resolve(packet(recovered)); await flush();
assert.equal(f.controller.states().get(1).status, "live");
await advance(100);
const closing = f.requests.at(-1);
const beforeBusyFrame = f.controller.states().get(1).frame;
closing.resolve(packet(closing, true)); await flush();
assert.equal(f.controller.states().get(1).frame, beforeBusyFrame, "transient busy retains the current unexpired RGBA frame");
assert.equal(f.controller.states().get(1).status, "live");
assert.equal(f.controller.states().get(1).busyDrops, 1);
await advance(100);
const disposingRequest = f.requests.at(-1);
f.dispose(); disposingRequest.resolve(packet(disposingRequest)); await flush();
assert.equal(timers.size, 0);
assert.equal(f.controller.states().size, 0, "dispose releases all frame state and rejects late frames");
assert.equal(f.maxConcurrent(), 1);

// BUSY is contention, not a new frame: its deadline never extends. A hung
// successor must also expire the last frame without waiting for a response.
const busy = fixture([output(50)]);
await advance(0);
busy.requests[0].resolve(packet(busy.requests[0], true)); await flush();
assert.equal(busy.controller.states().get(50).status, "starting");
assert.equal(busy.controller.states().get(50).frame, null, "initial BUSY cannot invent an image");
await advance(34);
busy.requests.at(-1).resolve(packet(busy.requests.at(-1))); await flush();
const firstFrame = busy.controller.states().get(50).frame;
await advance(34);
busy.requests.at(-1).resolve(packet(busy.requests.at(-1), true)); await flush();
assert.equal(busy.controller.states().get(50).frame, firstFrame);
assert.deepEqual([...firstFrame.data.subarray(0,4)], [198,67,114,255], "BUSY retains exact RGBA pixels");
await advance(34);
busy.requests.at(-1).resolve(packet(busy.requests.at(-1))); await flush();
const secondFrame = busy.controller.states().get(50).frame;
assert.notEqual(secondFrame, firstFrame, "successor frame replaces the retained RGBA frame");
assert.notEqual(secondFrame.data.buffer, firstFrame.data.buffer, "successor owns its new IPC buffer");
const secondAt = now;
await advance(34);
busy.requests.at(-1).resolve(packet(busy.requests.at(-1), true)); await flush();
await advance(TIMELINE_OUTPUT_FRAME_MAX_AGE_MS - 35);
assert.equal(now, secondAt + TIMELINE_OUTPUT_FRAME_MAX_AGE_MS - 1);
assert.equal(busy.controller.states().get(50).frame, secondFrame, "frame retained until exact bounded deadline");
await advance(1);
assert.equal(busy.controller.states().get(50).frame, null, "expiry is independent of hung in-flight request");
assert.equal(busy.controller.states().get(50).status, "error");
assert.match(busy.controller.states().get(50).error, /expired/);
assert.equal(busy.controller.states().get(50).frame, null, "expired pixel buffer is no longer retained by state");
busy.requests.at(-1).resolve(packet(busy.requests.at(-1), true)); await flush();
assert.equal(busy.controller.states().get(50).status, "error", "continued BUSY preserves the visible expiry error");
await advance(34);
busy.requests.at(-1).resolve(packet(busy.requests.at(-1))); await flush();
assert.equal(busy.controller.states().get(50).status, "live");
assert.equal(busy.controller.states().get(50).error, null);
await advance(34);
const oldProjectRequest = busy.requests.at(-1);
busy.setProjectEpoch(2);
assert.equal(busy.controller.states().get(50).frame, null, "project change clears even identically configured output");
oldProjectRequest.resolve(packet(oldProjectRequest)); await flush();
assert.equal(busy.controller.states().get(50).frame, null, "retired project response cannot return");
await advance(34);
busy.requests.at(-1).resolve(packet(busy.requests.at(-1), true)); await flush();
assert.equal(busy.controller.states().get(50).frame, null, "new project BUSY cannot retain old project image");
await advance(34);
busy.requests.at(-1).resolve(packet(busy.requests.at(-1))); await flush();
busy.setOutputs([output(50, 2048, 512)]);
assert.equal(busy.controller.states().get(50).frame, null);
await advance(34);
busy.requests.at(-1).resolve(packet(busy.requests.at(-1), true)); await flush();
assert.equal(busy.controller.states().get(50).frame, null, "changed configuration BUSY cannot retain old config image");
await advance(34);
busy.requests.at(-1).resolve(packet(busy.requests.at(-1))); await flush();
await advance(34);
busy.requests.at(-1).reject(new Error("Output changed while rendering; waiting for a current frame.")); await flush();
assert.equal(busy.controller.states().get(50).frame, null, "retired Program epoch error immediately clears valid image");
assert.equal(busy.controller.states().get(50).status, "error");
busy.dispose();
assert.equal(busy.controller.states().size, 0);
assert.equal(timers.size, 0);

const settledDispose = fixture([output(60)]);
await advance(0);
settledDispose.requests[0].resolve(packet(settledDispose.requests[0])); await flush();
assert(settledDispose.controller.states().get(60).frame);
settledDispose.dispose();
assert.equal(timers.size, 0, "settled frame disposal clears both readback and expiry timers without relying on an IPC finally");
assert.equal(settledDispose.controller.states().size, 0);
const invalid = fixture([output(3, 0, 100)]);
await advance(500);
assert.equal(invalid.requests.length, 0);
assert.equal(invalid.controller.states().get(3).status, "error");
invalid.dispose();
const slotOf = (request, origin) => Math.floor((request.at - origin + 1e-7) * 60 / 1_000);
const assertUniqueSlots = (requests, origin) => {
  const slots = requests.map(request => slotOf(request, origin));
  assert.equal(new Set(slots).size, slots.length, "one request at most in each fixed 60 Hz slot");
  for (let index = 1; index < slots.length; index++) assert(slots[index] > slots[index - 1]);
};
for (const outputCount of [1, 2, 4]) {
  const cadence = fixture(Array.from({ length: outputCount }, (_, index) => output(index + 10)));
  const startedAt = now;
  let resolved = 0;
  for (let elapsed = 0; elapsed < 1_000; elapsed++) {
    await advance(1);
    while (resolved < cadence.requests.length) {
      const request = cadence.requests[resolved++];
      request.resolve(packet(request));
      await flush();
    }
  }
  const measured = cadence.requests.filter(request => request.at < startedAt + 1_000);
  assert(measured.length <= 60, "all outputs share 60 slots in a half-open one-second window");
  assertUniqueSlots(cadence.requests, startedAt);
  for (let id = 10; id < 10 + outputCount; id++) {
    const starts = measured.filter((request) => request.args.outputId === id);
    const minimum = outputCount <= 2 ? 28 : 14;
    assert(starts.length >= minimum && starts.length <= 30,
      `${outputCount} outputs: each gets its fair video cadence, got ${starts.length}`);
    for (let i = 1; i < starts.length; i++) {
      assert(slotOf(starts[i], startedAt) - slotOf(starts[i - 1], startedAt) >= 2,
        "each output skips at least one shared slot; timer lateness does not move the phase");
    }
  }
  assert.equal(cadence.maxConcurrent(), 1, "higher cadence never queues concurrent readbacks");
  assert.equal(now - startedAt, 1_000);
  cadence.dispose();
}

// Persistent timer lateness must not accumulate into a slower clock. Mixed
// lateness also crosses slot boundaries: obsolete slots are lost, never queued.
for (const latePattern of [[3], [0, 4, 1, 8, 2, 5], [0, 41, 2, 75, 1]]) {
  let timerIndex = 0;
  timerLateness = () => latePattern[timerIndex++ % latePattern.length];
  const origin = now;
  const cadence = fixture();
  let resolved = 0;
  for (let elapsed = 0; elapsed < 5_000; elapsed++) {
    await advance(1);
    while (resolved < cadence.requests.length) {
      const request = cadence.requests[resolved++];
      request.resolve(packet(request)); await flush();
    }
  }
  assertUniqueSlots(cadence.requests, origin);
  for (const id of [1, 2]) {
    const starts = cadence.requests.filter(request => request.args.outputId === id && request.at < origin + 5_000);
    if (Math.max(...latePattern) < 10) {
      assert(starts.length >= 149 && starts.length <= 150,
        `timer lag ${latePattern} must preserve 30 Hz phase over five seconds, got ${starts.length}`);
    }
    for (let index = 1; index < starts.length; index++) {
      assert(slotOf(starts[index], origin) - slotOf(starts[index - 1], origin) >= 2);
    }
  }
  assert.equal(cadence.maxConcurrent(), 1);
  cadence.dispose();
  timerLateness = () => 0;
}

{
  const origin = now, cadence = fixture();
  await advance(0);
  const first = cadence.requests[0];
  await advance(10_000);
  assert.equal(cadence.requests.length, 1, "hung readback does not accumulate requests");
  first.resolve(packet(first)); await flush(); await advance(0);
  assert.equal(cadence.requests.length, 2, "completion consumes only the current slot, not missed slots");
  cadence.requests[1].resolve(packet(cadence.requests[1])); await flush();
  await advance(0);
  assert.equal(cadence.requests.length, 2, "no immediate catch-up after a late completion");
  cadence.controller.retry();
  cadence.setProjectEpoch(2);
  cadence.setActive(false); cadence.setActive(true);
  await advance(0);
  assert.equal(cadence.requests.length, 2, "retry/project/pause cannot mint another start in a consumed slot");
  document.hidden = true; document.dispatchEvent(new Event("visibilitychange"));
  await advance(20_000);
  assert.equal(cadence.requests.length, 2);
  document.hidden = false; document.dispatchEvent(new Event("visibilitychange"));
  await advance(0);
  assert.equal(cadence.requests.length, 3, "visibility resume skips directly to the current slot");
  cadence.requests[2].resolve(packet(cadence.requests[2])); await flush(); await advance(0);
  assert.equal(cadence.requests.length, 3);
  assertUniqueSlots(cadence.requests, origin);
  assert.equal(cadence.maxConcurrent(), 1);
  cadence.dispose();
  assert.equal(timers.size, 0);
}

// The existing VJ bus still consumes JPEG. A valid RGBA packet must not become
// an image/jpeg Blob merely because the shared parser now accepts both formats.
const { createLiveVideoMonitorController } = await import(
  await compile(new URL("../src/createLiveVideoMonitorController.ts", import.meta.url)),
);
let disposeVj, vj;
const vjRequests = [];
createRoot((cleanup) => {
  disposeVj = cleanup;
  vj = createLiveVideoMonitorController({
    backendAvailable: () => true, active: () => true, layerCount: () => 1,
    previewLayerId: () => null, programOutputId: () => 70, width: 2, height: 1,
    invoke(command, args) { return new Promise((resolve) => vjRequests.push({ command, args, resolve })); },
  });
});
await advance(0);
assert.equal(vjRequests.length, 1);
assert.equal(vjRequests[0].args.decodeBudget, undefined);
vjRequests[0].resolve(packet(vjRequests[0])); await flush();
assert.equal(vj.program().status, "error");
assert.match(vj.program().error, /unexpected pixel format/);
assert.equal(vj.program().frameUrl, null);
let created = 0, revoked = 0;
URL.createObjectURL = (blob) => { assert.equal(blob.type, "image/jpeg"); created++; return "blob:vj-jpeg"; };
URL.revokeObjectURL = (url) => { assert.equal(url, "blob:vj-jpeg"); revoked++; };
vj.retry(); await advance(0);
const jpegRequest = vjRequests.at(-1);
const jpeg = new Uint8Array(44), jpegView = new DataView(jpeg.buffer);
jpeg.set([83, 89, 76, 86, 2, 0, 0, 0]);
jpegView.setUint16(32, 2, true); jpegView.setUint16(34, 1, true); jpegView.setUint32(36, 4, true);
jpeg.set([255, 216, 255, 217], 40);
jpegRequest.resolve(jpeg); await flush();
assert.equal(vj.program().status, "live");
assert.equal(vj.program().frameUrl, "blob:vj-jpeg");
assert.equal(created, 1);
disposeVj();
assert.equal(revoked, 1);
assert.equal(timers.size, 0);

console.log("Timeline output monitor checks passed: aspect-fit, fair fixed-slot cadence under late/jittered timers, missed-slot dropping, single-flight, snapshot stability, bounded BUSY retention, hung-read expiry/recovery, project/config/removal fences, visibility, error recovery, invalid dimensions, disposal and VJ JPEG-only compatibility.");
