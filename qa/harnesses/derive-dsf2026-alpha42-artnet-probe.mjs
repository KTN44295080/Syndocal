import { createHash } from "node:crypto";
import { dirname, extname, join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

import {
  decodeUtf8Strict,
  exactKeys,
  fail,
  isObject,
  parseJsonText,
  equalJson,
} from "../../tools/dsf2026/common.mjs";
import {
  MAX_BASE_BYTES,
  normalizeIdentityPath,
  prepareOutputPath,
  readRegularFileBytesNoReparse,
  writeExclusiveBytes,
} from "../../tools/dsf2026/io.mjs";

const REPO_ROOT = resolve(fileURLToPath(new URL("../../", import.meta.url)));
const SOURCE_DIRECTORY = join("target", "qa", "dsf2026-show-authored-20260828");

export const SOURCE_FILENAME = "DSF2026-show-alpha42-three-display-acceptance.sdc";
export const OUTPUT_FILENAME = "DSF2026-show-alpha42-artnet-probe-acceptance.sdc";
export const SOURCE_PATH = resolve(REPO_ROOT, SOURCE_DIRECTORY, SOURCE_FILENAME);
export const OUTPUT_PATH = resolve(REPO_ROOT, SOURCE_DIRECTORY, OUTPUT_FILENAME);
export const SOURCE_BYTE_SIZE = 1_120_320;
export const SOURCE_SHA256 = "2D8B4D760E51009344D5A3195A39A61D0674F993027CD440A49F6F7D8F1F355C";
export const OUTPUT_BYTE_SIZE = 1_120_306;
export const OUTPUT_SHA256 = "4130599CEB73F2D97C7BB22DBCAD6A9187BD02F53DAC3946491BEB32E33776D9";
export const ALLOWED_CHANGED_PATHS = Object.freeze([
  "snapshot.dmx_outputs[0].protocol",
  "snapshot.output.protocol",
]);

const SOURCE_PROTOCOL = "EnttecOpenDmx";
const OUTPUT_PROTOCOL = "ArtNet";
const SOURCE_PROTOCOL_LITERAL = '"protocol": "EnttecOpenDmx"';
const OUTPUT_PROTOCOL_LITERAL = '"protocol": "ArtNet"';

const PROJECT_KEYS = Object.freeze([
  "app",
  "custom_profiles",
  "dj_track_triggers",
  "fixture_groups",
  "midi_mappings",
  "snapshot",
  "version",
]);
const SNAPSHOT_KEYS = Object.freeze([
  "active_cue_id",
  "active_fade",
  "active_group_cue_ids",
  "blackout",
  "clock",
  "cue_lists",
  "cues",
  "dmx_outputs",
  "dmx_preview",
  "dmx_previews",
  "effects",
  "fixtures",
  "group_colors",
  "lighting_master",
  "node_graphs",
  "output",
  "palettes",
  "playback_executors",
  "playback_master",
  "programmer",
  "stage_map",
  "stage_map_presets",
  "stage_objects",
  "submasters",
  "telemetry",
  "timeline",
  "timeline_bank",
  "touch_surface",
  "video",
]);
const VIDEO_KEYS = Object.freeze([
  "auto_vj",
  "blackout",
  "compositions",
  "layers",
  "mapping_presets",
  "master_opacity",
  "media_assets",
  "outputs",
]);
const DMX_ROUTE_KEYS = Object.freeze([
  "enabled",
  "port",
  "protocol",
  "serial_baud_rate",
  "serial_port",
  "target_ip",
  "universe",
]);
const COMPOSITION_KEYS = Object.freeze([
  "id",
  "label",
  "layer_ids",
  "output_ids",
  "timeline_layer_ids",
]);
const VIDEO_OUTPUT_KEYS = Object.freeze([
  "blackout",
  "composition_id",
  "enabled",
  "endpoint_name",
  "fullscreen",
  "height",
  "id",
  "kind",
  "label",
  "mapping",
  "monitor_id",
  "monitor_identity",
  "opacity",
  "width",
]);
const VIDEO_MAPPING_KEYS = Object.freeze([
  "aspect_mode",
  "aspect_ratio",
  "bitmap_mask_height",
  "bitmap_mask_luma_words",
  "bitmap_mask_width",
  "black_level",
  "corner_bottom_left_x",
  "corner_bottom_left_y",
  "corner_bottom_right_x",
  "corner_bottom_right_y",
  "corner_top_left_x",
  "corner_top_left_y",
  "corner_top_right_x",
  "corner_top_right_y",
  "edge_blend_bottom",
  "edge_blend_gamma",
  "edge_blend_left",
  "edge_blend_right",
  "edge_blend_top",
  "keystone_x",
  "keystone_y",
  "lens_distortion",
  "mask_invert",
  "mask_point_count",
  "mask_points",
  "mask_softness",
  "offset_x",
  "offset_y",
  "rotation_deg",
  "scale_x",
  "scale_y",
  "stage_x",
  "stage_y",
  "stage_z",
]);

const SOURCE_ARRAY_COUNTS = Object.freeze({
  custom_profiles: 12,
  fixture_groups: 16,
  midi_mappings: 2,
  dj_track_triggers: 1,
  fixtures: 46,
  timeline_bank: 3,
  video_layers: 3,
  video_media_assets: 5,
  video_compositions: 3,
  video_outputs: 4,
});

function sha256(bytes) {
  return createHash("sha256").update(bytes).digest("hex").toUpperCase();
}

function assertObject(value, label) {
  if (!isObject(value)) fail(`${label} must be an object`);
  return value;
}

function assertArray(value, label, expectedLength) {
  if (!Array.isArray(value)) fail(`${label} must be an array`);
  if (expectedLength !== undefined && value.length !== expectedLength) {
    fail(`${label} must contain exactly ${expectedLength} entries; found ${value.length}`);
  }
  return value;
}

function assertExact(value, expected, label) {
  if (!equalJson(value, expected)) fail(`${label} must remain exactly ${JSON.stringify(expected)}`);
}

function assertRoute(route, label, protocol) {
  assertObject(route, label);
  exactKeys(route, DMX_ROUTE_KEYS, label);
  assertExact(route.enabled, false, `${label}.enabled`);
  assertExact(route.port, 6454, `${label}.port`);
  assertExact(route.protocol, protocol, `${label}.protocol`);
  assertExact(route.serial_baud_rate, 250000, `${label}.serial_baud_rate`);
  assertExact(route.serial_port, "", `${label}.serial_port`);
  assertExact(route.target_ip, "127.0.0.1", `${label}.target_ip`);
  assertExact(route.universe, 0, `${label}.universe`);
}

function assertCount(value, expected, label) {
  if (!Array.isArray(value) || value.length !== expected) {
    fail(`${label} count must be exactly ${expected}; found ${Array.isArray(value) ? value.length : "non-array"}`);
  }
}

function assertSourceCompositionContract(compositions) {
  const expected = [
    { id: 1, label: "Main", layer_ids: [1, 2, 3], output_ids: [1, 2], timeline_layer_ids: [] },
    { id: 2, label: "Foreground Video 1", layer_ids: [3], output_ids: [3], timeline_layer_ids: [] },
    { id: 3, label: "Background Video2 Camera", layer_ids: [2], output_ids: [4], timeline_layer_ids: [{ layer_id: 3, timeline_id: 209 }] },
  ];
  assertArray(compositions, "snapshot.video.compositions", expected.length);
  compositions.forEach((composition, index) => {
    assertObject(composition, `snapshot.video.compositions[${index}]`);
    exactKeys(composition, COMPOSITION_KEYS, `snapshot.video.compositions[${index}]`);
    assertExact(composition, expected[index], `snapshot.video.compositions[${index}]`);
  });
}

function assertSourceVideoOutputContract(outputs, protocol) {
  const expected = [
    { id: 1, label: "Syndocal Background", kind: "SpoutSender", composition_id: 1, width: 1920, height: 1080 },
    { id: 2, label: "Syndocal Foreground", kind: "SpoutSender", composition_id: 1, width: 1920, height: 1080 },
    { id: 3, label: "Display 1", kind: "Display", composition_id: 2, width: 1920, height: 1080 },
    { id: 4, label: "Display 5", kind: "Display", composition_id: 3, width: 3840, height: 2160 },
  ];
  assertArray(outputs, "snapshot.video.outputs", expected.length);
  outputs.forEach((output, index) => {
    assertObject(output, `snapshot.video.outputs[${index}]`);
    exactKeys(output, VIDEO_OUTPUT_KEYS, `snapshot.video.outputs[${index}]`);
    exactKeys(output.mapping, VIDEO_MAPPING_KEYS, `snapshot.video.outputs[${index}].mapping`);
    for (const [key, value] of Object.entries(expected[index])) assertExact(output[key], value, `snapshot.video.outputs[${index}].${key}`);
  });
  if (protocol !== SOURCE_PROTOCOL && protocol !== OUTPUT_PROTOCOL) fail(`unsupported probe protocol ${JSON.stringify(protocol)}`);
}

function assertArrayCounts(project) {
  assertCount(project.custom_profiles, SOURCE_ARRAY_COUNTS.custom_profiles, "custom_profiles");
  assertCount(project.fixture_groups, SOURCE_ARRAY_COUNTS.fixture_groups, "fixture_groups");
  assertCount(project.midi_mappings, SOURCE_ARRAY_COUNTS.midi_mappings, "midi_mappings");
  assertCount(project.dj_track_triggers, SOURCE_ARRAY_COUNTS.dj_track_triggers, "dj_track_triggers");
  const snapshot = project.snapshot;
  assertCount(snapshot.fixtures, SOURCE_ARRAY_COUNTS.fixtures, "snapshot.fixtures");
  assertCount(snapshot.timeline_bank, SOURCE_ARRAY_COUNTS.timeline_bank, "snapshot.timeline_bank");
  assertCount(snapshot.video.layers, SOURCE_ARRAY_COUNTS.video_layers, "snapshot.video.layers");
  assertCount(snapshot.video.media_assets, SOURCE_ARRAY_COUNTS.video_media_assets, "snapshot.video.media_assets");
  assertCount(snapshot.video.compositions, SOURCE_ARRAY_COUNTS.video_compositions, "snapshot.video.compositions");
  assertCount(snapshot.video.outputs, SOURCE_ARRAY_COUNTS.video_outputs, "snapshot.video.outputs");
}

/**
 * Validate the approved alpha42 three-display project shape.  The hash check
 * in assertApprovedSourceIdentity is authoritative for production; this
 * explicit schema/count contract keeps malformed or future-shaped fixtures
 * fail-closed in focused tests and before the byte patch is admitted.
 */
export function validateAlpha42SourceProject(project, { protocol = SOURCE_PROTOCOL } = {}) {
  assertObject(project, "alpha42 project");
  exactKeys(project, PROJECT_KEYS, "alpha42 project");
  assertExact(project.app, "Syndocal", "alpha42 project.app");
  assertExact(project.version, 1, "alpha42 project.version");
  const snapshot = assertObject(project.snapshot, "alpha42 project.snapshot");
  exactKeys(snapshot, SNAPSHOT_KEYS, "alpha42 project.snapshot");
  const video = assertObject(snapshot.video, "alpha42 project.snapshot.video");
  exactKeys(video, VIDEO_KEYS, "alpha42 project.snapshot.video");
  assertArrayCounts(project);

  assertRoute(snapshot.output, "alpha42 snapshot.output", protocol);
  const dmxOutputs = assertArray(snapshot.dmx_outputs, "alpha42 snapshot.dmx_outputs", 1);
  assertRoute(dmxOutputs[0], "alpha42 snapshot.dmx_outputs[0]", protocol);
  assertSourceCompositionContract(video.compositions);
  assertSourceVideoOutputContract(video.outputs, protocol);

  const timelineBankIds = snapshot.timeline_bank.map((timeline) => timeline?.id);
  assertExact(timelineBankIds, [1, 2, 209], "alpha42 snapshot.timeline_bank IDs");
  assertExact(snapshot.timeline?.id, 209, "alpha42 snapshot.timeline.id");
  return { status: "PASS" };
}

function countLiteral(text, literal) {
  let count = 0;
  let offset = 0;
  while (true) {
    const index = text.indexOf(literal, offset);
    if (index === -1) return count;
    count += 1;
    offset = index + literal.length;
  }
}

function changedPaths(left, right, path = "") {
  if (Object.is(left, right)) return [];
  if (Array.isArray(left) || Array.isArray(right)) {
    if (!Array.isArray(left) || !Array.isArray(right) || left.length !== right.length) return [path || "$"].filter(Boolean);
    return left.flatMap((entry, index) => changedPaths(entry, right[index], `${path}[${index}]`));
  }
  if (!isObject(left) || !isObject(right)) return [path || "$"].filter(Boolean);
  const keys = [...new Set([...Object.keys(left), ...Object.keys(right)])].sort();
  return keys.flatMap((key) => changedPaths(left[key], right[key], path ? `${path}.${key}` : key));
}

export function getChangedPaths(sourceProject, outputProject) {
  return changedPaths(sourceProject, outputProject).sort();
}

export function validateAlpha42ArtNetProbeProject(outputProject, sourceProject) {
  if (!sourceProject) fail("alpha42 source project is required for probe validation");
  validateAlpha42SourceProject(sourceProject, { protocol: SOURCE_PROTOCOL });
  validateAlpha42SourceProject(outputProject, { protocol: OUTPUT_PROTOCOL });
  const actual = getChangedPaths(sourceProject, outputProject);
  const expected = [...ALLOWED_CHANGED_PATHS].sort();
  if (!equalJson(actual, expected)) {
    fail(`alpha42 Art-Net probe changed paths differ; expected exactly ${expected.join(", ")}, found ${actual.join(", ")}`);
  }
  return { status: "PASS", changedPaths: actual };
}

function assertFixedPath(path, expected, label, allowTestPaths) {
  if (typeof path !== "string" || path.trim().length === 0) fail(`${label} must be a non-empty path`);
  const absolute = resolve(path);
  if (!allowTestPaths && normalizeIdentityPath(absolute) !== normalizeIdentityPath(expected)) {
    fail(`${label} is fixed to ${expected}; arbitrary paths are not permitted`);
  }
  return absolute;
}

export function assertApprovedSourceIdentity(sourcePath, sourceBytes, sourceProject, { allowTestPath = false } = {}) {
  const absoluteSource = assertFixedPath(sourcePath, SOURCE_PATH, "alpha42 source path", allowTestPath);
  if (!(sourceBytes instanceof Uint8Array)) fail("alpha42 source bytes must be a Uint8Array");
  if (sourceBytes.byteLength !== SOURCE_BYTE_SIZE) {
    fail(`alpha42 source byte length must be exactly ${SOURCE_BYTE_SIZE}; found ${sourceBytes.byteLength}`);
  }
  const actualHash = sha256(sourceBytes);
  if (actualHash !== SOURCE_SHA256) fail(`alpha42 source SHA-256 ${actualHash} does not match the approved source identity`);
  const sourceText = decodeUtf8Strict(sourceBytes, "alpha42 source");
  const reparsed = parseJsonText(sourceText, "alpha42 source");
  validateAlpha42SourceProject(reparsed, { protocol: SOURCE_PROTOCOL });
  if (sourceProject !== undefined && !equalJson(sourceProject, reparsed)) {
    fail("alpha42 parsed source project differs from the approved source bytes");
  }
  return { absoluteSource, sourceText, sourceProject: reparsed, byteSize: sourceBytes.byteLength, sha256: actualHash };
}

/**
 * Build the probe in memory using one exact byte-preserving textual patch.
 * No JSON reserialization is used: this keeps every source byte except the
 * two approved protocol literals unchanged.
 */
export function deriveAlpha42ArtNetProbeBytes(sourceBytes, { sourcePath = SOURCE_PATH, sourceProject, allowTestPath = false } = {}) {
  const identity = assertApprovedSourceIdentity(sourcePath, sourceBytes, sourceProject, { allowTestPath });
  const sourceLiteralCount = countLiteral(identity.sourceText, SOURCE_PROTOCOL_LITERAL);
  if (sourceLiteralCount !== 2) {
    fail(`alpha42 source must contain exactly two ${SOURCE_PROTOCOL_LITERAL} literals; found ${sourceLiteralCount}`);
  }
  if (countLiteral(identity.sourceText, OUTPUT_PROTOCOL_LITERAL) !== 0) {
    fail(`alpha42 source must not contain a pre-existing ${OUTPUT_PROTOCOL_LITERAL} literal`);
  }
  const outputText = identity.sourceText.replaceAll(SOURCE_PROTOCOL_LITERAL, OUTPUT_PROTOCOL_LITERAL);
  const outputBytes = Buffer.from(outputText, "utf8");
  if (outputBytes.byteLength !== OUTPUT_BYTE_SIZE) {
    fail(`alpha42 derived output length is not the expected ${OUTPUT_BYTE_SIZE} bytes`);
  }
  const outputProject = parseJsonText(decodeUtf8Strict(outputBytes, "derived alpha42 Art-Net probe"), "derived alpha42 Art-Net probe");
  const validation = validateAlpha42ArtNetProbeProject(outputProject, identity.sourceProject);
  const expectedText = identity.sourceText.replaceAll(SOURCE_PROTOCOL_LITERAL, OUTPUT_PROTOCOL_LITERAL);
  if (!Buffer.from(expectedText, "utf8").equals(outputBytes)) fail("alpha42 probe byte patch is not deterministic");
  return {
    sourcePath: identity.absoluteSource,
    sourceProject: identity.sourceProject,
    outputProject,
    outputText,
    outputBytes,
    sourceByteSize: sourceBytes.byteLength,
    sourceSha256: identity.sha256,
    outputByteSize: outputBytes.byteLength,
    outputSha256: sha256(outputBytes),
    changedPaths: validation.changedPaths,
    literalReplacementCount: sourceLiteralCount,
  };
}

async function readIdentity(path, label) {
  const file = await readRegularFileBytesNoReparse(path, label, MAX_BASE_BYTES);
  return { ...file, byteSize: file.bytes.byteLength, sha256: sha256(file.bytes) };
}

/**
 * Publish the fixed ignored QA derivative.  Production calls use no path
 * arguments; allowTestPaths exists solely for deterministic temp-fixture
 * tests and still requires source/output to be siblings and .sdc files.
 */
export async function createAlpha42ArtNetProbe({
  sourcePath = SOURCE_PATH,
  outputPath = OUTPUT_PATH,
  allowTestPaths = false,
  testHooks = null,
} = {}) {
  const exactSourcePath = assertFixedPath(sourcePath, SOURCE_PATH, "alpha42 source path", allowTestPaths);
  const exactOutputPath = assertFixedPath(outputPath, OUTPUT_PATH, "alpha42 output path", allowTestPaths);
  if (extname(exactSourcePath).toLowerCase() !== ".sdc" || extname(exactOutputPath).toLowerCase() !== ".sdc") {
    fail("alpha42 probe source and output must both use the .sdc extension");
  }
  if (normalizeIdentityPath(exactSourcePath) === normalizeIdentityPath(exactOutputPath)) {
    fail("alpha42 probe output must not overwrite its source");
  }
  if (normalizeIdentityPath(dirname(exactSourcePath)) !== normalizeIdentityPath(dirname(exactOutputPath))) {
    fail("alpha42 probe output must be a sibling of the exact source");
  }

  const source = await readIdentity(exactSourcePath, "exact alpha42 three-display source");
  const derived = deriveAlpha42ArtNetProbeBytes(source.bytes, {
    sourcePath: source.absolute,
    allowTestPath: allowTestPaths,
  });
  if (derived.sourceByteSize !== SOURCE_BYTE_SIZE || derived.sourceSha256 !== SOURCE_SHA256) {
    fail("alpha42 source identity changed before publication");
  }
  if (derived.outputByteSize !== OUTPUT_BYTE_SIZE || derived.outputSha256 !== OUTPUT_SHA256) {
    fail(`alpha42 derived output identity differs from the approved ${OUTPUT_BYTE_SIZE}-byte probe`);
  }

  const outputGuard = {};
  const preparedOutputPath = await prepareOutputPath(exactOutputPath, { guard: outputGuard });
  await testHooks?.beforePublication?.({ sourcePath: exactSourcePath, outputPath: preparedOutputPath });
  const finalSource = await readIdentity(exactSourcePath, "alpha42 source publication fence");
  if (finalSource.byteSize !== SOURCE_BYTE_SIZE || finalSource.sha256 !== SOURCE_SHA256 || !Buffer.from(finalSource.bytes).equals(Buffer.from(source.bytes))) {
    fail("alpha42 source changed before exclusive publication");
  }
  await writeExclusiveBytes(preparedOutputPath, derived.outputBytes, outputGuard, {
    fences: [{ path: exactSourcePath, byteSize: SOURCE_BYTE_SIZE, sha256: SOURCE_SHA256 }],
  });

  const written = await readIdentity(preparedOutputPath, "created alpha42 Art-Net probe");
  if (written.byteSize !== OUTPUT_BYTE_SIZE || written.sha256 !== OUTPUT_SHA256 || !Buffer.from(written.bytes).equals(derived.outputBytes)) {
    fail(`created alpha42 Art-Net probe failed exact identity verification; quarantine ${preparedOutputPath}`);
  }
  const reparsed = parseJsonText(decodeUtf8Strict(written.bytes, "created alpha42 Art-Net probe"), "created alpha42 Art-Net probe");
  const validation = validateAlpha42ArtNetProbeProject(reparsed, derived.sourceProject);
  await testHooks?.afterPublication?.({ sourcePath: exactSourcePath, outputPath: preparedOutputPath, bytes: Buffer.from(written.bytes) });
  return {
    sourcePath: exactSourcePath,
    outputPath: preparedOutputPath,
    sourceByteSize: SOURCE_BYTE_SIZE,
    sourceSha256: SOURCE_SHA256,
    outputByteSize: written.byteSize,
    outputSha256: written.sha256,
    changedPaths: validation.changedPaths,
    literalReplacementCount: 2,
  };
}

export function parseCliArgs(argv = process.argv.slice(2)) {
  if (argv.length !== 0) {
    fail("usage: node qa/harnesses/derive-dsf2026-alpha42-artnet-probe.mjs (source and output paths are fixed)");
  }
  return {};
}

async function main() {
  parseCliArgs();
  const result = await createAlpha42ArtNetProbe();
  console.log(JSON.stringify({
    ...result,
    purpose: "probe-only current alpha42 derivative; not a production replacement or physical acceptance",
    transport: "ArtNet",
    target: "127.0.0.1:6454",
    universe: 0,
    routesEnabled: false,
    udpSent: false,
  }, null, 2));
}

const invokedDirectly = process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href;
if (invokedDirectly) {
  main().catch((error) => {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  });
}
