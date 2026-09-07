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
const code = ts.transpileModule(source, {
  compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 },
}).outputText.replaceAll('from "solid-js"', `from "${solidUrl}"`)
  .replaceAll('from "./createLatestThumbnailBatch"', `from "${laneUrl}"`);
const { createMediaThumbnailController } = await import(`data:text/javascript;base64,${Buffer.from(code).toString("base64")}`);
const flush = async () => { for (let i = 0; i < 6; i++) await Promise.resolve(); };
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
