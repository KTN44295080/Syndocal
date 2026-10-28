// The DMX universes table: one table of independent routes (USB-DMX, Art-Net,
// sACN), one Apply. Row states come from the same native status the actions
// are gated on. There is no mandatory U0 loopback and no USB mirror state.
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const source = await readFile(new URL("../src/dmxUniverseState.ts", import.meta.url), "utf8");
const transpiled = ts.transpileModule(source, {
  compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 },
  fileName: "dmxUniverseState.ts",
});
const dmxUniverseState = await import(
  `data:text/javascript;base64,${Buffer.from(transpiled.outputText).toString("base64")}`
);
const { networkDmxRouteState, sameNetworkDmxRoute } = dmxUniverseState;
assert.equal("showDmxUniverseState" in dmxUniverseState, false,
  "the retired U0 loopback/USB-mirror state (Not prepared, Armed · S0) must not return");

const route = (protocol, universe, enabled = true, target_ip = "10.0.0.5", port = 6454) => ({
  enabled, protocol, target_ip, port, universe, serial_port: "", serial_baud_rate: 57600,
});

// Network universes and their last send result.
const telemetry = (overrides) => ({
  index: 1, universe: 3, attempted: true, success: true, bytes: 638, consecutive_failures: 0,
  reconnect_attempts: 0, reconnecting: false, ...overrides,
});
assert.deepEqual(networkDmxRouteState(route("ArtNet", 2, false), undefined), { label: "Disabled", tone: "idle" });
assert.deepEqual(networkDmxRouteState(route("ArtNet", 2), undefined), { label: "Enabled", tone: "idle" });
assert.deepEqual(networkDmxRouteState(route("Sacn", 3, true, "multicast", 5568), telemetry({})),
  { label: "Sending", tone: "ready" });
assert.deepEqual(
  networkDmxRouteState(route("ArtNet", 7), telemetry({ success: false, error: "host unreachable" })),
  { label: "Send failed · host unreachable", tone: "error" },
);
assert.deepEqual(networkDmxRouteState(route("ArtNet", 7), telemetry({ success: false, reconnecting: true })),
  { label: "Send failed", tone: "error" });

// A draft row shows the applied state only while it matches the applied route.
assert.equal(sameNetworkDmxRoute(route("ArtNet", 2), { ...route("ArtNet", 2), serial_baud_rate: 1 }), true);
for (const changed of [
  route("ArtNet", 2, false), route("Sacn", 2), route("ArtNet", 3), route("ArtNet", 2, true, "10.0.0.6"),
  route("ArtNet", 2, true, "10.0.0.5", 6455),
]) {
  assert.equal(sameNetworkDmxRoute(route("ArtNet", 2), changed), false);
}
assert.equal(sameNetworkDmxRoute(undefined, route("ArtNet", 2)), false);

// One panel owns every DMX output universe as row groups of one table.
const app = await readFile(new URL("../src/App.tsx", import.meta.url), "utf8");
const setupIoView = await readFile(new URL("../src/components/SetupIoWorkspaceView.tsx", import.meta.url), "utf8");
const panel = await readFile(new URL("../src/components/DmxOutputConfigPanel.tsx", import.meta.url), "utf8");
for (const [name, mount] of [["App", app], ["SetupIoWorkspaceView", setupIoView]]) {
  assert.doesNotMatch(mount, /<UsbDmxCablesPanel|<NetworkDmxRoutesPanel|<DmxUniverseOverview/,
    `${name}: the DMX zone must not mount separate cable, network or overview sections`);
}
assert.equal([...app.matchAll(/<DmxOutputConfigPanel\b/g)].length, 0, "App must not mount a second DMX output panel");
assert.equal([...setupIoView.matchAll(/<DmxOutputConfigPanel\b/g)].length, 1);
assert.match(setupIoView, /<DmxOutputConfigPanel[\s\S]*?onApplyUniverseTable=\{applyDmxOutputRoutes\}/);
assert.doesNotMatch(setupIoView, /<DmxOutputConfigPanel[^>]*onPrepareShowDmx|onApplyNetworkRoutes=|onStartUsbDmxUniverse=/,
  "the separate Prepare / network Apply / per-cable start steps are not part of the operator table");
assert.equal([...panel.matchAll(/<table\b/g)].length, 1, "one table");
assert.match(panel, /<tbody\s+data-usb-dmx-cables>/);
assert.match(panel, /<tbody\s+data-network-dmx-routes>/);
assert.match(panel, /<tfoot>[\s\S]*data-dmx-universe-add[\s\S]*<option\s+value="Usb">USB-DMX/,
  "the one Add row offers USB-DMX as an ordinary route kind");
assert.equal([...panel.matchAll(/data-dmx-universe-apply/g)].length, 1, "exactly one commit control");
assert.match(panel, /data-dmx-universe-apply\s+disabled=\{!dirty\(\) \|\| busy\(\) \|\| conflict\(\) \|\| Boolean\(validation\(\)\)\}/,
  "Apply stays disabled without edits, while busy, on a conflict or while the plan is invalid");
assert.match(panel, /when=\{props\.usbDmxUniverses\?\.legacyRouteActive\}>[\s\S]*?data-dmx-legacy-usb-active[\s\S]*?disabled=\{busy\(\) \|\| !props\.safetyBlackoutEngaged\}[\s\S]*?onStopLegacyUsbDmx\(\)/,
  "a pre-table USB-DMX worker started elsewhere has one recovery stop, only under safety blackout");
assert.match(setupIoView, /onStopLegacyUsbDmx=\{async \(\) => \{\s*await stopShowSerialDmxSafetyBlackoutRoute\(\);\s*await refreshUsbDmxUniverses\(\);/);
assert.doesNotMatch(panel, /Prepare|S0 engaged|output role|Armed · S0|Not prepared/,
  "developer wording and the Prepare step stay out of the operator table");
console.log("DMX universes table: PASS (independent route rows, draft vs applied, one panel, one Add row, one gated Apply)");

await import("./check-dmx-universe-table.mjs");
await import("./check-dmx-universe-apply.mjs");
