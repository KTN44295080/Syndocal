import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { createRequire } from "node:module";
import { pathToFileURL } from "node:url";
import ts from "typescript";
const require = createRequire(import.meta.url);
const solidUrl = pathToFileURL(require.resolve("solid-js/dist/solid.js")).href;
const { createRoot, createSignal } = await import(solidUrl);
const source = await readFile(new URL("../src/createMediaThumbnailController.ts", import.meta.url), "utf8");
const code = ts.transpileModule(source, {
  compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 },
}).outputText.replaceAll('from "solid-js"', `from "${solidUrl}"`);
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
assert.equal(assetRequests.length, 2, "new authority starts exactly one successor asset batch");
assetRequests[0].resolve("retired");
layerRequests[0].resolve("layer-current");
await flush();
assert.deepEqual(controller.mediaAssetThumbnails(), {}, "retired authority completion is rejected");
assert.equal(controller.videoClipThumbnails()[1], "layer-current", "asset authority changes do not retire independent layer batch");
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
