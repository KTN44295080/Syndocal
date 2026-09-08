import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { createRequire } from "node:module";
import { pathToFileURL } from "node:url";
import ts from "typescript";
const require = createRequire(import.meta.url);
const solidUrl = pathToFileURL(require.resolve("solid-js/dist/solid.js")).href;
const { createRoot, createSignal } = await import(solidUrl);
const source = await readFile(new URL("../src/createMediaThumbnailController.ts", import.meta.url), "utf8");
const laneSource = await readFile(new URL("../src/createLatestThumbnailBatch.ts", import.meta.url), "utf8");
const laneCode = ts.transpileModule(laneSource, {
  compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 },
}).outputText;
const laneUrl = `data:text/javascript;base64,${Buffer.from(laneCode).toString("base64")}`;
const { createLatestThumbnailBatch } = await import(laneUrl);
const retrySource = await readFile(new URL("../src/thumbnailReadRetry.ts", import.meta.url), "utf8");
const retryCode = ts.transpileModule(retrySource, {
  compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 },
}).outputText;
const retryUrl = `data:text/javascript;base64,${Buffer.from(retryCode).toString("base64")}`;
const { readThumbnailWithRetry } = await import(retryUrl);
const code = ts.transpileModule(source, {
  compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 },
}).outputText.replaceAll('from "solid-js"', `from "${solidUrl}"`)
  .replaceAll('from "./createLatestThumbnailBatch"', `from "${laneUrl}"`)
  .replaceAll('from "./thumbnailReadRetry"', `from "${retryUrl}"`);
const { createMediaThumbnailController } = await import(`data:text/javascript;base64,${Buffer.from(code).toString("base64")}`);
const flush = async () => { for (let i = 0; i < 16; i++) await Promise.resolve(); };
const authorityA = { project_epoch: 1, project_revision: 0, checkpoint_hash: "A" };
const authorityB = { project_epoch: 2, project_revision: 0, checkpoint_hash: "B" };
const layer = { id: 1, source: { kind: "File", path: "a.mp4" } };
const asset = { id: 2, source: { kind: "File", path: "a.mp4" }, content_hash: { algorithm: "Sha256", hex: "a".repeat(64) }, byte_size: 42 };
let layerScans = 0, assetScans = 0, authorityChecks = 0;
const measured = (values, count) => new Proxy(values, {
  get(target, key, receiver) {
    if (key === "map") return (...args) => { count(); return target.map(...args); };
    return Reflect.get(target, key, receiver);
  },
});
let controller, setSnapshot, setAuthority, dispose;
const layerRequests = [], assetRequests = [];
const deferredLoad = (requests) => (id) => new Promise((resolve, reject) => requests.push({ id, resolve, reject }));
const layers = measured([layer], () => layerScans++);
const assets = measured([asset], () => assetScans++);
createRoot((cleanup) => {
  dispose = cleanup;
  const [snapshot, writeSnapshot] = createSignal({ video: { layers, media_assets: assets } });
  const [authority, writeAuthority] = createSignal(authorityA);
  setSnapshot = writeSnapshot; setAuthority = writeAuthority;
  controller = createMediaThumbnailController({
    layers: () => snapshot().video.layers,
    assets: () => snapshot().video.media_assets,
    projectMappingsAuthority: authority,
    isProjectAuthorityIdentityCurrent: (captured) => {
      authorityChecks++;
      return JSON.stringify(captured) === JSON.stringify(authority());
    },
    isTauriRuntime: () => true,
    loadVideoLayerThumbnail: deferredLoad(layerRequests),
    loadMediaAssetThumbnail: deferredLoad(assetRequests),
  });
});
await flush();
assert.deepEqual([layerRequests.length, assetRequests.length], [0, 0], "hydration cannot read files without authorization");
assert.deepEqual([layerScans, assetScans], [1, 1]);
for (let i = 0; i < 100; i++) setSnapshot((previous) => ({ ...previous, clock: { phase: i } }));
assert.deepEqual([layerScans, assetScans], [1, 1], "100 unrelated snapshot updates do not rescan either source array");
controller.authorizeVideoThumbnailAccess();
await flush();
assert.deepEqual([layerRequests.length, assetRequests.length], [1, 1]);
setAuthority({ ...authorityA });
await flush();
assert.equal(assetRequests.length, 1, "equivalent authority publication does not restart pending batch");
setAuthority(authorityB);
await flush();
assert.equal(assetRequests.length, 1, "new authority queues one successor without overlapping the active read");
assetRequests[0].resolve("retired");
layerRequests[0].resolve("layer-current");
await flush();
assert.deepEqual(controller.mediaAssetThumbnails(), {}, "retired authority completion is rejected");
assert.equal(controller.videoClipThumbnails()[1], "layer-current", "asset authority changes do not retire independent layer batch");
assert.equal(assetRequests.length, 2, "successor starts only when the retired read settles");
assetRequests[1].resolve("asset-current");
await flush();
assert.equal(controller.mediaAssetThumbnails()[2], "asset-current");
const cachedAuthority = { ...authorityB, project_revision: 1 };
const beforeChangedAuthority = authorityChecks;
setAuthority(cachedAuthority);
await flush();
assert.ok(authorityChecks > beforeChangedAuthority, "a genuine authority change still validates the cached batch");
const beforeEquivalentAuthorities = authorityChecks;
for (let i = 0; i < 100; i++) setAuthority({ ...cachedAuthority });
await flush();
assert.equal(authorityChecks, beforeEquivalentAuthorities, "100 equivalent authority publications cannot rerun the cached batch");
assert.equal(assetRequests.length, 2, "cached source remains reusable after current-authority validation");
setSnapshot((previous) => ({ ...previous, video: { ...previous.video, layers: measured([{ ...layer, source: { ...layer.source, path: "b.mp4" } }], () => layerScans++) } }));
await flush();
assert.deepEqual([layerScans, assetScans], [2, 1], "changed layer array only rescans layers");
assert.equal(layerRequests.length, 2);
controller.reset();
layerRequests[1].resolve("after-reset");
await flush();
assert.equal(controller.videoThumbnailAccessAuthorized(), false);
assert.deepEqual(controller.videoClipThumbnails(), {});
assert.deepEqual(controller.mediaAssetThumbnails(), {});
controller.authorizeVideoThumbnailAccess();
await flush();
assert.deepEqual([layerRequests.length, assetRequests.length], [3, 3], "replacement reset retires cache and both generations");
dispose();
layerRequests[2].resolve("after-dispose-layer");
assetRequests[2].resolve("after-dispose-asset");
await flush();
assert.deepEqual(controller.videoClipThumbnails(), {});
assert.deepEqual(controller.mediaAssetThumbnails(), {});
console.log("PASS media thumbnail controller: 100 unchanged-array and 100 equivalent cached-authority publications, independent source scans, authorization, authority, reset and disposal");

// Controlled slow readers exercise production code without opening media/devices.
function boundedFixture(initialAssets = [asset]) {
  const requests = { layers: [], assets: [] };
  const active = { layers: 0, assets: 0 }, peak = { layers: 0, assets: 0 };
  const load = (kind) => (id) => {
    active[kind]++;
    peak[kind] = Math.max(peak[kind], active[kind]);
    return new Promise((resolve, reject) => requests[kind].push({ id, resolve, reject }))
      .finally(() => { active[kind]--; });
  };
  const result = { requests, active, peak };
  createRoot((dispose) => {
    const [layers, setLayers] = createSignal([layer]);
    const [assets, setAssets] = createSignal(initialAssets);
    const [authority, setAuthority] = createSignal(authorityA);
    Object.assign(result, { dispose, setLayers, setAssets, setAuthority,
      controller: createMediaThumbnailController({ layers, assets,
        projectMappingsAuthority: authority,
        isProjectAuthorityIdentityCurrent: captured => JSON.stringify(captured) === JSON.stringify(authority()),
        isTauriRuntime: () => true,
        loadVideoLayerThumbnail: load('layers'), loadMediaAssetThumbnail: load('assets'),
      }),
    });
  });
  return result;
}
{
  const f = boundedFixture();
  f.controller.authorizeVideoThumbnailAccess(); await flush();
  for (let i = 1; i <= 50; i++) {
    f.setAuthority({ ...authorityA, project_revision: i });
    f.setLayers([{ ...layer, source: { ...layer.source, path: `clip-${i}.mp4` } }]);
  }
  await flush();
  assert.deepEqual(f.peak, { layers: 1, assets: 1 }, 'burst cannot multiply in-flight native reads');
  assert.deepEqual([f.requests.layers.length, f.requests.assets.length], [1, 1]);
  f.requests.layers[0].resolve('retired-layer');
  f.requests.assets[0].reject(new Error('retired-error')); await flush();
  assert.deepEqual(f.controller.videoClipThumbnails(), {});
  assert.deepEqual(f.controller.mediaAssetThumbnails(), {});
  assert.deepEqual([f.requests.layers.length, f.requests.assets.length], [2, 2], 'only the latest successor runs');
  f.requests.layers[1].resolve('latest-layer');
  f.requests.assets[1].resolve('latest-asset'); await flush();
  assert.equal(f.controller.videoClipThumbnails()[1], 'latest-layer');
  assert.equal(f.controller.mediaAssetThumbnails()[2], 'latest-asset');
  assert.deepEqual(f.active, { layers: 0, assets: 0 });
  assert.deepEqual(f.peak, { layers: 1, assets: 1 });
  console.log('PASS 50-change burst: peak native reads 1 per lane; 2 calls total per lane including latest successor');
  f.dispose();
}
{
  const catalog = Array.from({ length: 13 }, (_, i) => ({ ...asset, id: i + 2 }));
  const f = boundedFixture(catalog);
  f.controller.authorizeVideoThumbnailAccess(); await flush();
  f.setAuthority(authorityB);
  f.requests.assets[0].resolve('retired'); await flush();
  for (let i = 0; i < catalog.length; i++) {
    assert.equal(f.requests.assets[i + 1].id, catalog[i].id);
    f.requests.assets[i + 1].resolve(`current-${catalog[i].id}`); await flush();
  }
  assert.equal(Object.keys(f.controller.mediaAssetThumbnails()).length, 13);
  assert.ok(Object.values(f.controller.mediaAssetThumbnails()).every(v => v.startsWith('current-')));
  assert.equal(f.requests.assets.length, 14, 'retired batch never continues to a second asset');
  assert.equal(f.peak.assets, 1);
  f.dispose(); f.requests.layers[0].resolve('disposed'); await flush();
  console.log('PASS 13-asset successor completes with one active read and no retired batch continuation');
}
{
  const f = boundedFixture(); f.controller.authorizeVideoThumbnailAccess(); await flush();
  for (let i = 0; i < 20; i++) { f.controller.reset(); f.controller.authorizeVideoThumbnailAccess(); }
  assert.deepEqual([f.requests.layers.length, f.requests.assets.length], [1, 1]);
  f.controller.reset();
  f.requests.layers[0].resolve('old'); f.requests.assets[0].resolve('old'); await flush();
  assert.deepEqual(f.controller.mediaAssetThumbnails(), {});
  assert.deepEqual(f.controller.videoClipThumbnails(), {});
  assert.deepEqual([f.requests.layers.length, f.requests.assets.length], [1, 1], 'reset removes the pending successor');
  f.dispose();
}

// Retry policy is bounded and never treats authority/decode failures as transient.
const busy = 'Thumbnail work is already queued or running; retry after it completes';
const timeout = 'Thumbnail renderer wait timed out; retry after preview work completes';
for (const error of [busy, new Error(timeout)]) {
  let calls = 0, waits = 0;
  const value = await readThumbnailWithRetry(async () => {
    if (++calls === 1) throw error; return 'retried';
  }, () => true, async () => { waits++; });
  assert.equal(value, 'retried'); assert.equal(calls, 2); assert.equal(waits, 1);
}
{
  let calls = 0, waits = 0;
  await assert.rejects(readThumbnailWithRetry(async () => { calls++; throw new Error(busy); },
    () => true, async () => { waits++; }), /already queued/);
  assert.equal(calls, 2); assert.equal(waits, 1, 'no recursive retry');
}
for (const error of [new Error('decode failed'), new Error('project authority changed'), null]) {
  let calls = 0, waits = 0;
  try { await readThumbnailWithRetry(async () => { calls++; throw error; }, () => true,
    async () => { waits++; }); assert.fail('expected rejection'); }
  catch (actual) { assert.equal(actual, error); }
  assert.equal(calls, 1); assert.equal(waits, 0);
}
{
  let calls = 0, current = true, release;
  const waiting = new Promise(resolve => { release = resolve; });
  const result = readThumbnailWithRetry(async () => { calls++; throw busy; }, () => current, () => waiting);
  await flush(); current = false; release();
  await assert.rejects(result, /retired before retry/); assert.equal(calls, 1);
  await assert.rejects(readThumbnailWithRetry(async () => { calls++; return 'wrong'; }, () => false), /retired before read/);
  assert.equal(calls, 1);
}
{
  const f = boundedFixture(); f.controller.authorizeVideoThumbnailAccess(); await flush();
  f.requests.assets[0].reject(busy);
  await new Promise(resolve => setTimeout(resolve, 130)); await flush();
  assert.equal(f.requests.assets.length, 2, 'one transient retry is performed');
  f.requests.assets[1].reject(busy);
  await new Promise(resolve => setTimeout(resolve, 130)); await flush();
  assert.equal(f.requests.assets.length, 2, 'persistent busy does not loop');
  f.controller.authorizeVideoThumbnailAccess(); await flush();
  assert.equal(f.requests.assets.length, 3, 'explicit load retries an absent unchanged-source entry');
  f.requests.assets[2].resolve('manual-recovery'); await flush();
  assert.equal(f.controller.mediaAssetThumbnails()[2], 'manual-recovery');
  assert.deepEqual(f.peak, { layers: 1, assets: 1 });
  f.dispose(); f.requests.layers[0].resolve('retired'); await flush();
}
{
  const f = boundedFixture(); f.controller.authorizeVideoThumbnailAccess(); await flush();
  f.requests.layers[0].resolve('cached-layer'); f.requests.assets[0].resolve('cached-asset'); await flush();
  f.controller.authorizeVideoThumbnailAccess(); await flush();
  assert.deepEqual([f.requests.layers.length, f.requests.assets.length], [1, 1], 'explicit load retains valid cached successes');
  f.dispose();
}
{
  const f = boundedFixture(); f.controller.authorizeVideoThumbnailAccess(); await flush();
  f.requests.assets[0].reject(timeout); await flush();
  f.controller.reset();
  await new Promise(resolve => setTimeout(resolve, 130)); await flush();
  assert.equal(f.requests.assets.length, 1, 'reset retires the scheduled retry before native work');
  f.dispose(); f.requests.layers[0].resolve('retired'); await flush();
}
console.log('PASS bounded transient retry, terminal error rejection, stale retry retirement, explicit recovery and cache reuse');

// Busy is worker lifetime, not permission: clear must not claim an outstanding read ended.
{
  const observed = []; let release;
  const gate = createLatestThumbnailBatch(value => observed.push(value));
  const held = new Promise(resolve => { release = resolve; });
  gate.replace(async () => { await held; }, error => { throw error; });
  gate.clear();
  assert.deepEqual(observed, [true]);
  let successors = 0;
  for (let i = 0; i < 20; i++) gate.replace(async () => { successors++; }, () => {});
  release(); await flush();
  assert.equal(successors, 1); assert.deepEqual(observed, [true, false]);
  gate.dispose();
}
{
  const f = boundedFixture();
  assert.equal(f.controller.videoThumbnailBusy(), false);
  assert.equal(f.controller.mediaAssetThumbnailBusy(), false);
  f.controller.authorizeVideoThumbnailAccess(); await flush();
  for (let i = 0; i < 30; i++) f.controller.authorizeVideoThumbnailAccess();
  assert.deepEqual([f.requests.layers.length, f.requests.assets.length], [1, 1]);
  assert.equal(f.controller.videoThumbnailBusy(), true);
  assert.equal(f.controller.mediaAssetThumbnailBusy(), true);
  f.requests.assets[0].reject(new Error('decode unavailable')); await flush();
  assert.equal(f.controller.mediaAssetThumbnailBusy(), false);
  assert.equal(f.controller.videoThumbnailBusy(), true);
  f.controller.authorizeVideoThumbnailAccess(); await flush();
  assert.deepEqual([f.requests.layers.length, f.requests.assets.length], [1, 2]);
  f.requests.layers[0].resolve('not-retired-by-click');
  f.requests.assets[1].resolve('retry-recovered'); await flush();
  assert.equal(f.controller.videoClipThumbnails()[1], 'not-retired-by-click');
  assert.equal(f.controller.mediaAssetThumbnails()[2], 'retry-recovered');
  assert.equal(f.controller.videoThumbnailBusy(), false);
  assert.equal(f.controller.mediaAssetThumbnailBusy(), false);
  f.controller.authorizeVideoThumbnailAccess(); await flush();
  assert.deepEqual([f.requests.layers.length, f.requests.assets.length], [1, 2], 'successful caches do not reload');
  f.dispose();
}
{
  const f = boundedFixture(); f.controller.authorizeVideoThumbnailAccess(); await flush();
  f.controller.reset();
  assert.equal(f.controller.videoThumbnailAccessAuthorized(), false);
  assert.equal(f.controller.videoThumbnailBusy(), true);
  assert.equal(f.controller.mediaAssetThumbnailBusy(), true);
  f.dispose(); f.controller.authorizeVideoThumbnailAccess();
  assert.equal(f.controller.videoThumbnailAccessAuthorized(), false);
  f.requests.layers[0].resolve('old'); f.requests.assets[0].resolve('old'); await flush();
  assert.equal(f.controller.videoThumbnailBusy(), false);
  assert.equal(f.controller.mediaAssetThumbnailBusy(), false);
  assert.deepEqual(f.controller.videoClipThumbnails(), {});
  assert.deepEqual(f.controller.mediaAssetThumbnails(), {});
  assert.deepEqual([f.requests.layers.length, f.requests.assets.length], [1, 1]);
}
console.log('PASS independent busy lifetimes, repeated-click admission, visible retry eligibility and retired-worker ownership');

{
  const { mergeProps } = await import(solidUrl);
  const f = boundedFixture();
  const layerView = mergeProps(f.controller.layerThumbnailView, { label: 'unrelated' });
  const assetView = mergeProps(f.controller.assetThumbnailView, { label: 'unrelated' });
  assert.equal(layerView.thumbnailsAuthorized, false);
  f.controller.authorizeVideoThumbnailAccess(); await flush();
  assert.equal(layerView.thumbnailsAuthorized, true);
  assert.equal(layerView.thumbnailsBusy, true); assert.equal(assetView.thumbnailsBusy, true);
  f.requests.layers[0].resolve('view-layer'); f.requests.assets[0].resolve('view-asset'); await flush();
  assert.equal(layerView.thumbnailsBusy, false); assert.equal(assetView.thumbnailsBusy, false);
  assert.equal(layerView.thumbnails[1], 'view-layer'); assert.equal(assetView.thumbnails[2], 'view-asset');
  f.controller.reset();
  assert.equal(layerView.thumbnailsAuthorized, false); assert.deepEqual(assetView.thumbnails, {});
  f.dispose();
}
console.log('PASS reactive thumbnail view bindings across loading, completion and reset');
