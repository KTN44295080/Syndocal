import assert from "node:assert/strict";
import { typescriptFixtureModuleUrl } from "./typescript-fixture-module.mjs";

const discovery = await import(await typescriptFixtureModuleUrl(new URL("../src/videoNdiSources.ts", import.meta.url)));
const health = await import(await typescriptFixtureModuleUrl(new URL("../src/ndiInputHealth.ts", import.meta.url)));
const catalog = { state: "available", sources: [{ name: "Camera A", endpoint_name: "Studio (Camera A)" }, { name: "Camera A", endpoint_name: "Backup (Camera A)" }], message: null };
const parsed = discovery.parseVideoNdiDiscovery(catalog);
assert.deepEqual(parsed, catalog);
assert.equal(discovery.ndiSourceAllowsAdd("", parsed, false), false, "no automatic first-source admission");
assert.equal(discovery.ndiSourceAllowsAdd("Camera A", parsed, false), false, "display names are not identities");
assert.equal(discovery.ndiSourceAllowsAdd("Studio (Camera A)", parsed, false), true);
assert.equal(discovery.ndiSourceAllowsAdd("Studio (Camera A)", parsed, true), false, "refresh invalidates admission");
assert.equal(discovery.ndiSourceAllowsAdd("Studio (Camera A)", null, false), false);
for (const state of ["not_built", "runtime_missing", "error"]) {
  const unavailable = discovery.parseVideoNdiDiscovery({ state, sources: [], message: "diagnostic" });
  assert.equal(discovery.ndiSourceAllowsAdd("Studio (Camera A)", unavailable, false), false);
  assert.throws(() => discovery.parseVideoNdiDiscovery({ ...catalog, state }), /invalid response/);
}
for (const invalid of [null, [], {}, { ...catalog, state: "unknown" }, { ...catalog, state: ["available"] }, { ...catalog, message: 42 }, { ...catalog, sources: [{ name: "", endpoint_name: "x" }] }, { ...catalog, sources: [{ name: "x", endpoint_name: "" }] }, { ...catalog, sources: [catalog.sources[0], catalog.sources[0]] }]) {
  assert.throws(() => discovery.parseVideoNdiDiscovery(invalid), /invalid response/);
}
for (const endpoint of [" Studio (Camera A)", "Studio (Camera A) ", "PC\u0000(A)", "映".repeat(1366)]) {
  assert.throws(() => discovery.parseVideoNdiDiscovery({ ...catalog, sources: [{ name: "Camera", endpoint_name: endpoint }] }), /invalid response/);
}
assert.match(discovery.ndiDiscoveryStateMessage({ state: "runtime_missing", sources: [], message: null }), /NDI runtime not installed/);
assert.equal(health.ndiInputSourceIdentity("収録:カメラ"), "v1:3:ndi:16:収録:カメラ", "UTF-8 byte length, not JavaScript character length");

const layer = { id: 7, label: "NDI camera", source: { kind: "Ndi", name: "Stage NDI" }, state: { enabled: true } };
const source = { route_id: 7, identity: "v1:3:ndi:9:Stage NDI", generation: 1, state: "Live", issue: null };
const route = { direction: "Input", route_id: 7, label: "NDI camera", backend_id: "ndi", endpoint_name: "Stage NDI" };
const status = { active_routes: [route], active_count: 1, live_sources: [source], live_source_faults: [], capture_faults: [], ownership_allowed: true };
const stateOf = (next, layers = [layer]) => health.buildNdiInputHealthRows(layers, next)[0].state;
assert.equal(stateOf(status), "connected");
assert.equal(stateOf(null), "unavailable", "failed status reads cannot retain Connected");
assert.equal(stateOf({ ...status, live_sources: [] }), "unavailable", "active route alone is not frame proof");
assert.equal(stateOf({ ...status, active_routes: [] }), "unavailable", "Live without exact active route is insufficient");
assert.equal(stateOf({ ...status, live_sources: [{ ...source, state: "Starting" }] }), "no_signal");
assert.equal(stateOf({ ...status, active_routes: [{ ...route, endpoint_name: "Other NDI" }] }), "unavailable");
assert.equal(stateOf({ ...status, live_sources: [{ ...source, identity: "v1:3:ndi:9:Other NDI" }] }), "unavailable", "previous endpoint status must not attach to replacement source");
assert.equal(stateOf({ ...status, ownership_allowed: false }), "connected", "receiving an input is independent of physical output ownership");
assert.equal(stateOf(status, [{ ...layer, state: { enabled: false } }]), "disabled");
const fault = (message) => ({ route_id: 7, backend_id: "ndi", label: layer.label, message });
assert.equal(stateOf({ ...status, live_source_faults: [fault("NDI input has no signal for 'Stage NDI'. NDI source identity is exact.")] }), "no_signal", "initial receiver state dominates active route");
assert.equal(stateOf({ ...status, live_source_faults: [fault("NDI input signal lost for 'Stage NDI'. Reconnecting.")] }), "lost");
assert.equal(stateOf({ ...status, live_source_faults: [fault("NDI receiver failed")] }), "unavailable");
assert.equal(stateOf({ ...status, live_source_faults: [{ ...fault("NDI input signal lost for another layer"), route_id: 8 }] }), "connected", "other layer fault does not overwrite this source");
assert.equal(health.buildNdiInputHealthRows([{ ...layer, source: { kind: "File", path: "a.mp4" } }], status).length, 0);
console.log("NDI input checks passed: strict discovery, explicit exact-source admission, UTF-8 identity and connected/no-signal/lost health.");
