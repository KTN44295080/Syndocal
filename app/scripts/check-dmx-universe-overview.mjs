// The DMX universe overview lists every output universe once, with a state
// derived from the same native status the editing sections use.
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const source = await readFile(new URL("../src/dmxUniverseOverview.ts", import.meta.url), "utf8");
const transpiled = ts.transpileModule(source, {
  compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 },
  fileName: "dmxUniverseOverview.ts",
});
const { dmxUniverseOverviewRows } = await import(
  `data:text/javascript;base64,${Buffer.from(transpiled.outputText).toString("base64")}`
);

const route = (protocol, universe, enabled = true, target_ip = "10.0.0.5", port = 6454) => ({
  enabled, protocol, target_ip, port, universe, serial_port: "", serial_baud_rate: 57600,
});
const identity = (port_name) => ({
  port_name, port_type: "usb", usb_vid: 1027, usb_pid: 24577, serial_number: `SN-${port_name}`,
  manufacturer: "FTDI", product: "FT232R",
});
const base = {
  routes: [route("ArtNet", 0, true, "127.0.0.1")],
  routeTelemetry: [],
  showDmxBinding: { state: "selected_and_present", selected: identity("COM3"), detail: "", routeStatusRevision: "1" },
  showDmxWorker: null,
  usbCables: null,
  safetyBlackoutEngaged: false,
};

// Universe 0 alone, before Prepare.
let rows = dmxUniverseOverviewRows({ ...base, routes: [route("ArtNet", 0, false, "127.0.0.1")] });
assert.equal(rows.length, 1);
assert.deepEqual(
  { universe: rows[0].universe, transport: rows[0].transport, state: rows[0].state, tone: rows[0].tone },
  { universe: 0, transport: "USB-DMX + Art-Net", state: "Not prepared", tone: "idle" },
);
assert.match(rows[0].target, /COM3 · FTDI \/ FT232R · 127\.0\.0\.1:6454/);

// Prepared and the USB worker live; S0 shows as armed.
const worker = (overrides) => ({
  routeStatusRevision: "1", active: true, zeroFrameQueued: false, zeroFramePhysicalWriteCompleted: true,
  liveFrameQueued: true, workerShutdownCompleted: true, faulted: false, artnetMirrorLive: true,
  artnetMirrorDetail: "", detail: "", ...overrides,
});
assert.equal(dmxUniverseOverviewRows({ ...base, showDmxWorker: worker({}) })[0].state, "Sending");
assert.equal(
  dmxUniverseOverviewRows({ ...base, showDmxWorker: worker({}), safetyBlackoutEngaged: true })[0].state,
  "Armed · S0",
);
assert.equal(dmxUniverseOverviewRows({ ...base, showDmxWorker: worker({ faulted: true }) })[0].tone, "error");

// Cables, network routes and their telemetry, in universe order.
const cable = (universe, overrides = {}) => ({
  slot: universe, universe, routeStatusRevision: "1", active: false, zeroFrameQueued: false,
  zeroFramePhysicalWriteCompleted: false, liveFrameQueued: false, workerShutdownCompleted: true,
  faulted: false, detail: "", ...overrides,
});
rows = dmxUniverseOverviewRows({
  ...base,
  routes: [route("ArtNet", 0, true, "127.0.0.1"), route("Sacn", 3, true, "multicast", 5568), route("ArtNet", 2, false)],
  routeTelemetry: [{ index: 1, universe: 3, attempted: true, success: true, bytes: 638, consecutive_failures: 0, reconnect_attempts: 0, reconnecting: false }],
  usbCables: {
    bindings: [
      { universe: 1, state: "selected_and_present", identity: identity("COM5"), detail: "" },
      { universe: 4, state: "stale_or_missing", identity: identity("COM6"), detail: "" },
    ],
    // A running cable without a readable binding keeps its row.
    cables: [cable(1, { active: true, liveFrameQueued: true }), cable(5, { faulted: true })],
    persistenceError: null,
  },
});
assert.deepEqual(rows.map((row) => [row.universe, row.transport, row.state]), [
  [0, "USB-DMX + Art-Net", "Art-Net only · USB stopped"],
  [1, "USB-DMX", "Sending"],
  [2, "Art-Net", "Disabled"],
  [3, "sACN", "Sending"],
  [4, "USB-DMX", "Cable missing"],
  [5, "USB-DMX", "Fault · S0 latched"],
]);
assert.equal(rows.find((row) => row.universe === 3).target, "multicast:5568");

// A failing network route is an error with its reason.
rows = dmxUniverseOverviewRows({
  ...base,
  routes: [route("ArtNet", 0, true, "127.0.0.1"), route("ArtNet", 7)],
  routeTelemetry: [{ index: 1, universe: 7, attempted: true, success: false, bytes: 0, error: "host unreachable", consecutive_failures: 3, reconnect_attempts: 1, reconnecting: false }],
});
assert.deepEqual([rows[1].state, rows[1].tone], ["Send failed · host unreachable", "error"]);

const app = await readFile(new URL("../src/App.tsx", import.meta.url), "utf8");
assert.match(app, /<DmxUniverseOverview[\s\S]*?<DmxOutputConfigPanel/, "the overview leads the DMX zone");
console.log("DMX universe overview: PASS (Universe 0, USB cables, network routes, telemetry, order)");
