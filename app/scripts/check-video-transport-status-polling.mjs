import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

const root = new URL("../", import.meta.url);
const read = (path) => readFile(new URL(path, root), "utf8");

const [app, controller, panel, types, localization] = await Promise.all([
  read("src/App.tsx"),
  read("src/createVideoRuntimeController.ts"),
  read("src/components/VideoRuntimeStatusPanels.tsx"),
  read("src/types.ts"),
  read("src/uiLocalization.ts"),
]);

assert.match(
  types,
  /interface ExternalVideoCaptureFaultStatus\s*\{\s*route_id: number;\s*backend_id: string;\s*label: string;\s*message: string;/s,
  "capture fault type must preserve the flat backend contract",
);
assert.match(
  types,
  /interface ExternalVideoTransportStatus[\s\S]*?capture_faults: ExternalVideoCaptureFaultStatus\[\];/s,
  "transport status must carry structured capture faults",
);

assert.match(
  controller,
  /const refreshExternalVideoTransportStatus = async \(silent = false\) => \{/,
  "transport status refresh must be a dedicated controller operation",
);
assert.match(
  controller,
  /options\.invoke<ExternalVideoTransportStatus>\("get_external_video_transport_status"\)/,
  "dedicated refresh must use the exact transport status command",
);
assert.match(
  controller,
  /externalVideoTransportStatusRefreshInFlight/,
  "dedicated refresh must fence overlapping polls",
);
assert.match(
  controller,
  /options\.setExternalVideoTransportStatus\(status\);[\s\S]*?options\.setExternalVideoTransportReport\(null\);/s,
  "fresh status must supersede a stale sync report",
);
assert.match(
  controller,
  /catch \(error\) \{[\s\S]*?clearExternalVideoTransportStatus\(\);[\s\S]*?return null;/s,
  "status errors must clear stale transport state",
);
assert.match(
  controller,
  /refreshVideoRuntimeStatus, refreshExternalVideoTransportStatus, refreshExternalVideoIoPlans,/,
  "dedicated refresh must be exposed to App",
);
assert.match(
  controller,
  /const \[plans, status, transportStatus\] = await Promise\.all\(/,
  "the existing Check I/O path remains intact",
);
assert.match(
  controller,
  /clearExternalVideoTransportStatus\(\);\s*options\.setMessage\(String\(error\)\);/s,
  "Check I/O errors must also clear stale transport state",
);

assert.match(
  app,
  /captureFaults = status\?\.capture_faults \?\? \[\]/,
  "App must consume additive capture_faults safely",
);
assert.match(
  app,
  /fault\.route_id === route\.route_id && fault\.backend_id === route\.backend_id/s,
  "faulted routes must be identified by route and backend identity",
);
assert.match(
  app,
  /status\.active_routes\.filter\(\(route\) =>[\s\S]*?capture_faults/s,
  "faulted routes must be removed from Active rows",
);
assert.match(
  app,
  /report\.kept\.filter\(\(route\) => !hasCaptureFault\(route\)\)/,
  "faulted routes must also be removed from a stale sync report",
);
assert.match(app, /captureFaultText = captureFaults\.length > 0/, "transport summary must count capture faults");
assert.match(
  app,
  /const externalVideoTransportStatusTimer = isTauriRuntime\(\)[\s\S]*?refreshExternalVideoTransportStatus\(true\), 1000/s,
  "App must run a bounded dedicated transport status poll",
);
assert.match(
  app,
  /clearInterval\(externalVideoTransportStatusTimer\)/,
  "dedicated transport status poll must be cleaned up",
);
assert.match(
  app,
  /get captureFaultRows\(\) \{ return externalVideoTransportCaptureFaultRows\(\); \}/,
  "structured fault rows must be wired to the status panel",
);

assert.match(panel, /captureFaultRows: ExternalVideoStatusRow\[\];/, "status panel must accept capture faults");
assert.match(panel, /data-video-capture-fault-row/, "status panel must render a distinct fault row");
assert.match(
  panel,
  /Action: Check the camera or capture device connection, then disable and re-enable this source\./,
  "fault row must provide an actionable recovery instruction",
);

assert.match(localization, /"Capture fault":\s*"キャプチャ異常"/, "fault state must be localized");
assert.match(
  localization,
  /"Action: Check the camera or capture device connection, then disable and re-enable this source\."/,
  "fault recovery action must be localized",
);
assert.match(
  localization,
  /External video transport:.*capture fault/,
  "dedicated refresh message must have a Japanese dynamic mapping",
);

console.log("video transport status polling contract passed");
