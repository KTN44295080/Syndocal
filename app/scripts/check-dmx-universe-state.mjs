// The DMX universes table: one table for Show DMX (U0), the USB-DMX cables
// and the Art-Net / sACN universes. Row states come from the same native
// status the actions are gated on.
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const source = await readFile(new URL("../src/dmxUniverseState.ts", import.meta.url), "utf8");
const transpiled = ts.transpileModule(source, {
  compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 },
  fileName: "dmxUniverseState.ts",
});
const { showDmxUniverseState, networkDmxRouteState, sameNetworkDmxRoute } = await import(
  `data:text/javascript;base64,${Buffer.from(transpiled.outputText).toString("base64")}`
);

const route = (protocol, universe, enabled = true, target_ip = "10.0.0.5", port = 6454) => ({
  enabled, protocol, target_ip, port, universe, serial_port: "", serial_baud_rate: 57600,
});
const identity = (port_name) => ({
  port_name, port_type: "usb", usb_vid: 1027, usb_pid: 24577, serial_number: `SN-${port_name}`,
  manufacturer: "FTDI", product: "FT232R",
});
const worker = (overrides) => ({
  routeStatusRevision: "1", active: true, zeroFrameQueued: false, zeroFramePhysicalWriteCompleted: true,
  liveFrameQueued: true, workerShutdownCompleted: true, faulted: false, artnetMirrorLive: true,
  artnetMirrorDetail: "", detail: "", ...overrides,
});
const show = {
  route: route("ArtNet", 0, true, "127.0.0.1"),
  binding: { state: "selected_and_present", selected: identity("COM3"), detail: "", routeStatusRevision: "1" },
  worker: null,
  safetyBlackoutEngaged: false,
};

// Universe 0 before Prepare, Art-Net only, live, armed under S0 and faulted.
assert.deepEqual(showDmxUniverseState({ ...show, route: route("ArtNet", 0, false, "127.0.0.1") }),
  { label: "Not prepared", tone: "idle" });
assert.deepEqual(showDmxUniverseState(show), { label: "Art-Net only · USB stopped", tone: "idle" });
assert.deepEqual(showDmxUniverseState({ ...show, binding: null }), { label: "Art-Net only", tone: "idle" });
assert.deepEqual(showDmxUniverseState({ ...show, worker: worker({}) }), { label: "Sending", tone: "ready" });
assert.deepEqual(showDmxUniverseState({ ...show, worker: worker({ liveFrameQueued: false }) }),
  { label: "Started", tone: "ready" });
assert.deepEqual(showDmxUniverseState({ ...show, worker: worker({}), safetyBlackoutEngaged: true }),
  { label: "Armed · S0", tone: "ready" });
assert.deepEqual(showDmxUniverseState({ ...show, worker: worker({ faulted: true }) }),
  { label: "Fault · S0 latched", tone: "error" });

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

// One panel owns every DMX output universe; the former separate editors are
// row groups of its table.
const app = await readFile(new URL("../src/App.tsx", import.meta.url), "utf8");
const panel = await readFile(new URL("../src/components/DmxOutputConfigPanel.tsx", import.meta.url), "utf8");
const usbRows = await readFile(new URL("../src/components/UsbDmxCablesPanel.tsx", import.meta.url), "utf8");
const networkRows = await readFile(new URL("../src/components/NetworkDmxRoutesPanel.tsx", import.meta.url), "utf8");
assert.doesNotMatch(app, /<UsbDmxCablesPanel|<NetworkDmxRoutesPanel|<DmxUniverseOverview/,
  "the DMX zone must not mount separate cable, network or overview sections");
assert.equal([...app.matchAll(/<DmxOutputConfigPanel\b/g)].length, 1);
assert.match(app, /<DmxOutputConfigPanel[\s\S]*?onApplyNetworkRoutes=\{applyDmxOutputRoutes\}[\s\S]*?onStopUsbDmxUniverse=\{stopUsbDmxUniverse\}/);
assert.match(usbRows, /<tbody class="dmxUniverseGroup" data-usb-dmx-cables>/);
assert.match(networkRows, /<tbody class="dmxUniverseGroup" data-network-dmx-routes>/);
assert.match(panel, /<tfoot>[\s\S]*data-dmx-universe-add[\s\S]*<option value="Usb" disabled=\{!props\.usb\.canAddCable\(\)\}>USB-DMX<\/option>/,
  "the one Add row must offer USB-DMX only while another cable may be added");
assert.match(panel, /data-network-dmx-save\s+disabled=\{!network\.canApply\(\)\}/,
  "Apply must stay disabled without edits, while busy, on a conflict or before U0 is prepared");
assert.match(networkRows, /const canApply = \(\) => dirty\(\) && !busy\(\) && !conflict\(\) && showRoutePrepared\(\);/);
console.log("DMX universes table: PASS (U0, network row states, draft vs applied, one panel, one Add row, gated Apply)");
