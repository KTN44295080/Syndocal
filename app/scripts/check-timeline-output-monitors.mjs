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
const { createTimelineOutputMonitorController, timelineOutputMonitorSize } = await import(
  await compile(new URL("../src/createTimelineOutputMonitorController.ts", import.meta.url)),
);
assert.deepEqual(timelineOutputMonitorSize(1920, 1080), { width: 320, height: 180 });
assert.deepEqual(timelineOutputMonitorSize(1080, 1920), { width: 101, height: 180 });
assert.deepEqual(timelineOutputMonitorSize(2048, 512), { width: 320, height: 80 });
assert.equal(timelineOutputMonitorSize(0, 100), null);

let now = 0, timerId = 0, urlId = 0;
const timers = new Map(), liveUrls = new Set();
Object.defineProperty(globalThis, "performance", { value: { now: () => now }, configurable: true });
globalThis.window = {
  setTimeout(fn, delay) { const id = ++timerId; timers.set(id, { fn, at: now + delay }); return id; },
  clearTimeout(id) { timers.delete(id); },
};
globalThis.document = Object.assign(new EventTarget(), { hidden: false });
URL.createObjectURL = () => { const url = `blob:test-${++urlId}`; liveUrls.add(url); return url; };
URL.revokeObjectURL = (url) => { assert(liveUrls.delete(url), `URL revoked exactly once: ${url}`); };
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
  const bytes = new Uint8Array(busy ? 40 : 44), view = new DataView(bytes.buffer);
  bytes.set([0x53, 0x59, 0x4c, 0x56, 1, busy ? 1 : 0, 0, 0]);
  view.setUint16(32, request.args.width, true); view.setUint16(34, request.args.height, true);
  view.setUint32(36, busy ? 0 : 4, true);
  if (!busy) bytes.set([0xff, 0xd8, 0xff, 0xd9], 40);
  return bytes;
};
function fixture(initial = [output(1), output(2, 1080, 1920)]) {
  let dispose, setOutputs, setActive, controller, outstanding = 0, maxConcurrent = 0;
  const requests = [];
  createRoot((cleanup) => {
    dispose = cleanup;
    const [outputs, updateOutputs] = createSignal(initial);
    const [active, updateActive] = createSignal(true);
    setOutputs = updateOutputs; setActive = updateActive;
    controller = createTimelineOutputMonitorController({ outputs, active, backendAvailable: () => true,
      invoke(command, args) {
        outstanding += 1; maxConcurrent = Math.max(maxConcurrent, outstanding);
        return new Promise((resolve, reject) => requests.push({ command, args, at: now,
          resolve(value) { outstanding -= 1; resolve(value); }, reject(error) { outstanding -= 1; reject(error); },
        }));
      },
    });
  });
  return { controller, dispose, setOutputs, setActive, requests, maxConcurrent: () => maxConcurrent };
}

const f = fixture();
await advance(0);
assert.equal(f.requests.length, 1);
assert.equal(f.requests[0].command, "get_live_video_monitor_frame");
assert.equal(f.requests[0].args.monitorKind, "program");
await advance(1_000);
assert.equal(f.requests.length, 1, "slow decoder cannot overlap another output");
for (let i = 0; i < 20; i += 1) f.setOutputs([output(1), output(2, 1080, 1920)]);
f.requests[0].resolve(packet(f.requests[0])); await flush();
assert.equal(f.controller.states().get(1).status, "live", "equivalent snapshots cannot starve responses");
await advance(0);
assert.equal(f.requests[1].args.outputId, 2, "all outputs get round-robin turns");
assert.equal(f.requests[1].args.width, 101);
f.requests[1].resolve(packet(f.requests[1])); await flush();
await advance(99);
assert.equal(f.requests.length, 2, "aggregate start rate is bounded");
await advance(1);
assert.equal(f.requests[2].args.outputId, 1);
f.setOutputs([output(1, 2048, 512), output(2, 1080, 1920)]);
assert.equal(f.controller.states().get(1).frameUrl, null, "resolution changes clear previous image immediately");
f.requests[2].resolve(packet(f.requests[2])); await flush();
assert.equal(f.controller.states().get(1).frameUrl, null, "late old-resolution frame is rejected");
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
assert.equal(f.controller.states().get(1).frameUrl, null);
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
closing.resolve(packet(closing, true)); await flush();
assert.equal(f.controller.states().get(1).frameUrl, null, "busy response cannot preserve a stale live image");
assert.equal(f.controller.states().get(1).status, "starting");
assert.equal(f.controller.states().get(1).busyDrops, 1);
await advance(100);
const disposingRequest = f.requests.at(-1);
f.dispose(); disposingRequest.resolve(packet(disposingRequest)); await flush();
assert.equal(timers.size, 0);
assert.equal(liveUrls.size, 0, "dispose revokes all frame URLs and rejects late frames");
assert.equal(f.maxConcurrent(), 1);

const invalid = fixture([output(3, 0, 100)]);
await advance(500);
assert.equal(invalid.requests.length, 0);
assert.equal(invalid.controller.states().get(3).status, "error");
invalid.dispose();
console.log("Timeline output monitor checks passed: aspect-fit, all-output fairness, aggregate throttling, single-flight, snapshot stability, config/removal fences, visibility, error recovery, invalid dimensions and disposal.");
