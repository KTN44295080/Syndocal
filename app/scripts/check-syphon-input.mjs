import assert from "node:assert/strict";
import { typescriptFixtureModuleUrl } from "./typescript-fixture-module.mjs";
const discovery = await import(await typescriptFixtureModuleUrl(new URL("../src/videoSyphonSources.ts", import.meta.url)));
const server = { uuid: "id-a", name: "RGB corners", app_name: "Simple Server" };
const catalog = discovery.parseVideoSyphonServers([server, { ...server, uuid: "id-b" }]);
const endpoint = discovery.syphonServerEndpoint(server);
assert.deepEqual(catalog[0], server);
assert.equal(discovery.syphonServerAllowsAdd("", catalog, false), false);
assert.equal(discovery.syphonServerAllowsAdd("RGB corners", catalog, false), false);
assert.equal(discovery.syphonServerAllowsAdd(endpoint, catalog, false), true);
assert.equal(discovery.syphonServerAllowsAdd(endpoint, catalog, true), false);
assert.equal(discovery.syphonServerAllowsAdd(endpoint, null, false), false);
for (const value of [null, {}, [server, server], Array(65).fill(server), [{ ...server, uuid: "" }], [{ ...server, uuid: " id" }], [{ ...server, app_name: "" }], [{ ...server, name: "a\u0000" }], [{ ...server, name: "映".repeat(342) }]]) {
  assert.throws(() => discovery.parseVideoSyphonServers(value), /invalid response/);
}
assert.equal(discovery.syphonEndpointLabel(endpoint), "Simple Server / RGB corners");
assert.equal(discovery.syphonEndpointLabel("legacy-name"), null);
assert.equal(discovery.syphonServerLabel({ ...server, name: "" }), "Simple Server");
assert.throws(() => discovery.parseVideoSyphonServers([{ uuid: "\\".repeat(1024), name: "\\".repeat(1024), app_name: "\\".repeat(1024) }]), /invalid response/, "JSON escaping cannot exceed the endpoint byte budget");
console.log("Syphon input UI contracts passed: bounded directory, exact explicit selection, refresh invalidation, malformed and ambiguous rejection, readable persisted identity.");
