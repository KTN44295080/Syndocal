import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const appRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const repoRoot = path.resolve(appRoot, "..");
const normalize = (source) => source.replace(/\r\n/g, "\n");
const readText = async (relativePath) => normalize(await readFile(path.join(repoRoot, relativePath), "utf8"));

function assertNeedles(source, relativePath, needles, label) {
  for (const needle of needles) {
    assert(source.includes(needle), `${label} is missing ${relativePath}:${needle}`);
  }
}

function runGate(relativePath) {
  try {
    execFileSync(process.execPath, [path.join(repoRoot, relativePath)], {
      cwd: repoRoot,
      encoding: "utf8",
      stdio: ["ignore", "pipe", "pipe"],
    });
  } catch (error) {
    throw new Error(`AI1 query/event gate failed: ${relativePath}\n${error.stdout ?? ""}${error.stderr ?? ""}`);
  }
}

function runSelfTest() {
  const source = "const MAX = 100; const Gap = 'event_gap'; const resnapshot = true;";
  assertNeedles(source, "fixture", ["MAX = 100", "event_gap", "resnapshot = true"], "AI1 parser fixture");
  assert.throws(
    () => assertNeedles(source, "fixture", ["future_schema"], "AI1 parser fixture"),
    /future_schema/u,
  );
  console.log("AI1 query/event self-tests ok; 2 contract-presence cases");
}

async function run() {
  const protocol = await readText("crates/protocol/src/control_plane_query.rs");
  const appQuery = await readText("app/src-tauri/src/control_plane_query.rs");
  const appControlPlane = await readText("app/src-tauri/src/control_plane.rs");
  const frontendWatermark = await readText("app/src/timelineRuntimeSnapshotWatermark.ts");
  const runtimeWire = await readText("app/src/timelineRuntimeSnapshotWire.ts");

  assertNeedles(protocol, "crates/protocol/src/control_plane_query.rs", [
    "QUERY_PROTOCOL_MAJOR: u16 = 1",
    "QUERY_PROTOCOL_MINOR: u16 = 0",
    "PAGE_MAX_LIMIT: u16 = 100",
    "MAX_RUNTIME_DOMAIN_GENERATIONS: usize = 32",
    "MAX_SCHEMA_CATALOG_ENTRIES: usize = 512",
    "MAX_CAPABILITY_DISCOVERY_ENTRIES: usize = 512",
    "MAX_EVENT_PAGE_EVENTS: usize = 100",
    "pub enum QueryErrorCode",
    "EventGap",
    "SnapshotRequired",
    "pub struct SchemaCatalog",
    "pub struct CapabilityDiscovery",
    "pub struct ControlPlaneEvent",
    "pub struct EventPageRequest",
    "pub struct OpaqueCursorToken",
  ], "AI1 protocol contract");
  assertNeedles(appQuery, "app/src-tauri/src/control_plane_query.rs", [
    "MAX_CURSORS_PER_WINDOW: usize = 32",
    "MAX_CURSORS_PER_PROCESS: usize = 256",
    "MAX_EVENT_RING: usize = 2_048",
    "event_stream_epoch",
    "event_stream_generation",
    "query_error(QueryErrorCode::SnapshotRequired)",
    "EventSlice::Gap",
    "gap.resnapshot_required",
    "fn schema_catalog",
    "fn capability_discovery",
  ], "AI1 local query adapter");
  assertNeedles(appControlPlane, "app/src-tauri/src/control_plane.rs", [
    "get_control_plane_query_schema_catalog",
    "get_control_plane_query_capabilities",
    "poll_control_plane_observation_events",
  ], "AI1 query discovery admission");
  assertNeedles(frontendWatermark, "app/src/timelineRuntimeSnapshotWatermark.ts", [
    "canAccept",
    "resetForProjectScope",
    "transport_generation",
    "loop_generation",
    "follow_generation",
  ], "AI1 frontend runtime watermark");
  assertNeedles(runtimeWire, "app/src/timelineRuntimeSnapshotWire.ts", [
    "engineSnapshotRuntimeWireResponseFromUnknown",
    "generation",
    "transport_epoch",
  ], "AI1 runtime wire validation");

  const gates = [
    "app/scripts/check-snapshot-live-publication.mjs",
    "app/scripts/check-snapshot-runtime-watermark.mjs",
  ];
  for (const gate of gates) runGate(gate);
  console.log(`AI1 query/event contract ok; protocol bounds and schemas verified; focused frontend gates ${gates.length}`);
}

if (process.argv.includes("--self-test")) runSelfTest();
else await run();
