import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import { readFile, readdir } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const appRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const repoRoot = path.resolve(appRoot, "..");
const normalize = (source) => source.replace(/\r\n/g, "\n");
const readText = async (relativePath) => normalize(await readFile(path.join(repoRoot, relativePath), "utf8"));

function parseGenerateHandlerInventory(source) {
  const marker = "tauri::generate_handler![";
  const start = source.indexOf(marker);
  assert(start >= 0, "production generate_handler inventory is missing");
  const end = source.indexOf(".build(tauri::generate_context!())", start + marker.length);
  assert(end > start, "production generate_handler inventory has no build boundary");
  const block = source.slice(start, end);
  const names = [...block.matchAll(/^\s+([a-z][a-z0-9_]+),?\s*$/gm)].map((match) => match[1]);
  assert(names.length > 0, "production generate_handler inventory is empty");
  assert.equal(new Set(names).size, names.length, "production generate_handler inventory contains a duplicate");
  return names.sort();
}

function parseQuotedArray(source, marker) {
  const start = source.indexOf(marker);
  assert(start >= 0, `missing generated inventory marker ${marker}`);
  const end = source.indexOf("];", start + marker.length);
  assert(end > start, `missing generated inventory end for ${marker}`);
  return [...source.slice(start, end).matchAll(/"([A-Za-z][A-Za-z0-9_]*)"/g)].map((match) => match[1]);
}

function countMacroVariants(source, marker, arrow) {
  const start = source.indexOf(marker);
  assert(start >= 0, `missing declarative source ${marker}`);
  const end = source.indexOf(arrow ? "\n);" : "\n}", start + marker.length);
  assert(end > start, `missing declarative source end for ${marker}`);
  const block = source.slice(start, end);
  const pattern = arrow
    ? /^\s*([A-Z][A-Za-z0-9_]*)\s*=>/gm
    : /^ {4}([A-Z][A-Za-z0-9_]*)/gm;
  return [...block.matchAll(pattern)].map((match) => match[1]);
}

function assertNeedles(source, relativePath, needles, label) {
  for (const needle of needles) {
    assert(source.includes(needle), `${label} is missing ${relativePath}:${needle}`);
  }
}

function runStaticGate(relativePath) {
  try {
    return execFileSync(process.execPath, [path.join(repoRoot, relativePath)], {
      cwd: repoRoot,
      encoding: "utf8",
      stdio: ["ignore", "pipe", "pipe"],
    });
  } catch (error) {
    const output = `${error.stdout ?? ""}${error.stderr ?? ""}`;
    throw new Error(`AI0 delegated gate failed: ${relativePath}\n${output}`);
  }
}

async function collectFrontendSources(directory) {
  const entries = await readdir(directory, { withFileTypes: true });
  const nested = await Promise.all(entries.map(async (entry) => {
    const child = path.join(directory, entry.name);
    if (entry.isDirectory()) return collectFrontendSources(child);
    return /\.(?:ts|tsx)$/.test(entry.name) ? [child] : [];
  }));
  return nested.flat();
}

function runSelfTest() {
  assert.deepEqual(
    parseGenerateHandlerInventory("tauri::generate_handler![\n                get_snapshot,\n                set_bpm,\n            ]\n.build(tauri::generate_context!())"),
    ["get_snapshot", "set_bpm"],
  );
  assert.throws(
    () => parseGenerateHandlerInventory("tauri::generate_handler![\n                get_snapshot,\n                get_snapshot,\n            ]\n.build(tauri::generate_context!())"),
    /duplicate/,
  );
  assert.deepEqual(
    countMacroVariants("control_plane_variant_inventory!(\n  Names,\n  Name,\n  Kind,\n  Type;\n  SetBpm => Type::SetBpm,\n  TapBpm => Type::TapBpm,\n);", "control_plane_variant_inventory!", true),
    ["SetBpm", "TapBpm"],
  );
  console.log("AI0 source-coverage self-tests ok; 3 parser/duplicate cases");
}

async function run() {
  const main = await readText("app/src-tauri/src/main.rs");
  const controlPlane = await readText("app/src-tauri/src/control_plane.rs");
  const engine = await readText("crates/engine/src/lib.rs");
  const engineInventory = await readText("crates/engine/src/control_plane.rs");
  const remote = await readText("crates/io/src/remote_ws.rs");
  const midi = await readText("crates/io/src/midi.rs");
  const osc = await readText("crates/io/src/osc.rs");
  const dmx = await readText("crates/io/src/dmx_input.rs");
  const ioInventory = await readText("crates/io/src/control_plane.rs");
  const keyboardManifest = JSON.parse(await readText("app/src/keyboard-shortcut-source-manifest.json"));
  const invokeManifest = JSON.parse(await readText("app/src/tauri-invoke-manifest.json"));
  const frontendSources = await Promise.all(
    (await collectFrontendSources(path.join(appRoot, "src"))).map(async (filePath) => ({
      filePath,
      source: normalize(await readFile(filePath, "utf8")),
    })),
  );

  const tauriCommands = parseGenerateHandlerInventory(main);
  const tauriFingerprint = createHash("sha256").update(tauriCommands.join("\n"), "utf8").digest("hex");
  const frozenCount = Number(controlPlane.match(/FROZEN_TAURI_ROUTE_ADMISSION_COUNT: usize = (\d+)/)?.[1]);
  const frozenFingerprint = controlPlane.match(/FROZEN_TAURI_ROUTE_ADMISSION_SHA256: &str =\s*"([a-f0-9]{64})"/)?.[1];
  assert.equal(frozenCount, tauriCommands.length, "Tauri admission count is not tied to the production source");
  assert.equal(frozenFingerprint, tauriFingerprint, "Tauri admission fingerprint is not tied to the production source");

  assert.equal(invokeManifest.length, 464, "frontend invoke inventory count drifted; classify the delta");
  assert(invokeManifest.every((command) => tauriCommands.includes(command)), "frontend invoke inventory contains an unregistered Tauri command");
  assert.equal(new Set(invokeManifest).size, invokeManifest.length, "frontend invoke inventory contains a duplicate");

  assert.equal(keyboardManifest.schema_version, 1, "keyboard source manifest schema drifted");
  assert.equal(keyboardManifest.keyboard_app.length, 30, "keyboard app source inventory count drifted");
  assert.equal(keyboardManifest.keyboard_project_file.length, 3, "keyboard project-file source inventory count drifted");
  for (const sources of [keyboardManifest.keyboard_app, keyboardManifest.keyboard_project_file]) {
    assert.equal(new Set(sources).size, sources.length, "keyboard source inventory contains a duplicate");
    assert(sources.every((source) => /^[a-z][a-z0-9_]*_v1$/.test(source)), "keyboard source inventory has an unstable source id");
  }

  const engineSource = countMacroVariants(engine, "define_engine_command!", false);
  assert.equal(engineSource.length, 280, "EngineCommand declaration inventory count drifted");
  assertNeedles(engineInventory, "crates/engine/src/control_plane.rs", [
    "EngineCommand::CONTROL_PLANE_VARIANT_NAMES",
    "unavailable_engine_command_descriptor",
    "engine_command_variant_count",
  ], "Engine inventory");

  const remoteInput = countMacroVariants(remote, "define_remote_input_event!", false);
  const remoteClient = countMacroVariants(remote, "define_remote_client_request!", false);
  assert.equal(remoteInput.length, 51, "RemoteInputEvent declaration inventory count drifted");
  assert.equal(remoteClient.length, 7, "RemoteClientRequest declaration inventory count drifted");
  assertNeedles(ioInventory, "crates/io/src/control_plane.rs", [
    "RemoteInputEvent::CONTROL_PLANE_VARIANT_NAMES",
    "RemoteClientRequest::CONTROL_PLANE_VARIANT_NAMES",
    "RemoteWireOperation::CONTROL_PLANE_OPERATIONS",
  ], "Remote inventory");

  const ioFamilies = [
    [midi, "MIDI_CONTROL_MESSAGE_VARIANT_NAMES", 4],
    [midi, "MIDI_CONTROL_ACTION_VARIANT_NAMES", 51],
    [midi, "MIDI_CLOCK_EVENT_VARIANT_NAMES", 6],
    [midi, "MIDI_CONTROL_EVENT_VARIANT_NAMES", 47],
    [osc, "OSC_CONTROL_ACTION_VARIANT_NAMES", 47],
    [osc, "OSC_INPUT_EVENT_VARIANT_NAMES", 47],
    [dmx, "DMX_INPUT_PROTOCOL_VARIANT_NAMES", 2],
    [dmx, "DMX_INPUT_EVENT_VARIANT_NAMES", 2],
  ];
  for (const [source, marker, expected] of ioFamilies) {
    const names = countMacroVariants(source, `control_plane_variant_inventory!(\n    ${marker}`, true);
    assert.equal(names.length, expected, `${marker} declaration inventory count drifted`);
  }
  assertNeedles(ioInventory, "crates/io/src/control_plane.rs", [
    "control_plane_midi_osc_dmx_descriptors",
    "MIDI_CONTROL_MESSAGE_VARIANT_NAMES",
    "OSC_INPUT_EVENT_VARIANT_NAMES",
    "DMX_INPUT_EVENT_VARIANT_NAMES",
  ], "MIDI/OSC/DMX inventory");

  const frontendLiteralInvokes = new Set(
    frontendSources.flatMap(({ source }) => [...source.matchAll(/\b(?:invoke|tauriInvoke)(?:<[^;()]*?>)?\(\s*"([a-z0-9_]+)"/g)].map((match) => match[1])),
  );
  assert(frontendLiteralInvokes.size > 0, "frontend UI mutation inventory found no literal command source");
  assert([...frontendLiteralInvokes].every((command) => tauriCommands.includes(command)), "frontend UI mutation source bypasses Tauri admission");
  assertNeedles(controlPlane, "app/src-tauri/src/control_plane.rs", [
    "canonical_source_inventory_descriptor",
    "SourceDisposition::Unclassified",
    "verify_canonical_registry_exact_sources",
    "LocalWindowAuthoritativeMutation",
  ], "Canonical fail-closed UI/adapter inventory");

  const audioBpmMarkers = [
    "EngineCommand::SetBpm",
    "EngineCommand::TapBpm",
    "fn analyze_timeline_audio_path",
    "publish_live_audio_analysis_with",
  ];
  assertNeedles(main, "app/src-tauri/src/main.rs", audioBpmMarkers, "Audio-analysis/BPM inventory");
  const nativeOutputMarkers = [
    "fn open_video_output_window(",
    "fn close_video_output_window(",
    "fn sync_video_output_window(",
    "fn retire_native_video_output_window(",
    "fn video_output_window_label(",
  ];
  assertNeedles(main, "app/src-tauri/src/main.rs", nativeOutputMarkers, "Native output-window inventory");
  assertNeedles(controlPlane, "app/src-tauri/src/control_plane.rs", [
    "OUTPUT_DISPLAY_ADD_OPERATION_ID",
    "OUTPUT_DISPLAY_WINDOW_SET_OPEN_OPERATION_ID",
    "OUTPUT_VIDEO_COMPOSITION_ASSIGN_OPERATION_ID",
  ], "Native output control inventory");

  const delegatedGates = [
    "app/scripts/check-tauri-admission-inventory.mjs",
    "app/scripts/check-frontend-command-routing.mjs",
    "app/scripts/check-frontend-tauri-invokes.mjs",
    "app/scripts/check-backend-operator-contract.mjs",
    "app/scripts/check-project-shortcuts.mjs",
    "app/scripts/check-output-ownership.mjs",
  ];
  for (const gate of delegatedGates) runStaticGate(gate);

  const familySummary = [
    `Tauri ${tauriCommands.length}`,
    `Engine ${engineSource.length}`,
    `Remote ${remoteInput.length + remoteClient.length + 58}`,
    `MIDI/OSC/DMX ${ioFamilies.reduce((total, [, , count]) => total + count, 0)}`,
    `Frontend ${invokeManifest.length}`,
    `Keyboard ${keyboardManifest.keyboard_app.length + keyboardManifest.keyboard_project_file.length}`,
    `UI literal invokes ${frontendLiteralInvokes.size}`,
  ].join(", ");
  console.log(`AI0 source coverage ok; ${familySummary}; fail-closed authority gates ${delegatedGates.length}`);
}

if (process.argv.includes("--self-test")) runSelfTest();
else await run();
