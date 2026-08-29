import { createHash } from "node:crypto";
import { lstat, realpath } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { dirname, extname, resolve } from "node:path";

import {
  decodeUtf8Strict,
  equalJson,
  exactKeys,
  fail,
  isObject,
  parseJsonText,
} from "./dsf2026/common.mjs";
import {
  MAX_BASE_BYTES,
  normalizeIdentityPath,
  prepareOutputPath,
  readRegularFileBytesNoReparse,
  readStrictJsonFile,
  writeExclusive,
} from "./dsf2026/io.mjs";

export const SAME_PC_SOURCE_FILENAME = "DSF2026-show-alpha9-reference-audio.sdc";
export const SAME_PC_SOURCE_PATH = resolve(fileURLToPath(new URL(`../target/qa/dsf2026-show-authored-20260828/${SAME_PC_SOURCE_FILENAME}`, import.meta.url)));
export const SAME_PC_SOURCE_BYTE_SIZE = 1_095_864;
export const SAME_PC_SOURCE_SHA256 = "93e71d8ac3889968c2aad5b0a8ca194b88cb1c7b51bf897c7741c969d9a05094";
export const SAME_PC_OUTPUT_FILENAME = "DSF2026-show-alpha10-same-pc-output.sdc";
export const SAME_PC_OUTPUT_PATH = resolve(dirname(SAME_PC_SOURCE_PATH), SAME_PC_OUTPUT_FILENAME);

export const SAME_PC_ALLOWED_CHANGED_PATHS = Object.freeze([
  "snapshot.dmx_outputs[0].protocol",
  "snapshot.output.protocol",
  "snapshot.video.compositions[0].output_ids",
  "snapshot.video.outputs",
]);

const PROJECT_KEYS = [
  "app",
  "custom_profiles",
  "dj_track_triggers",
  "fixture_groups",
  "midi_mappings",
  "snapshot",
  "version",
];
const SNAPSHOT_KEYS = [
  "active_cue_id",
  "active_fade",
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
];
const VIDEO_KEYS = [
  "auto_vj",
  "blackout",
  "compositions",
  "layers",
  "mapping_presets",
  "master_opacity",
  "media_assets",
  "outputs",
];
const DMX_ROUTE_KEYS = [
  "enabled",
  "port",
  "protocol",
  "serial_baud_rate",
  "serial_port",
  "target_ip",
  "universe",
];
const COMPOSITION_KEYS = ["id", "label", "layer_ids", "output_ids"];
const VIDEO_OUTPUT_KEYS = [
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
];
const VIDEO_MAPPING_KEYS = [
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
];

function sha256(bytes) {
  return createHash("sha256").update(bytes).digest("hex");
}

function assertArray(value, label) {
  if (!Array.isArray(value)) fail(`${label} must be an array`);
  return value;
}

function assertObject(value, label) {
  if (!isObject(value)) fail(`${label} must be an object`);
  return value;
}

function assertFiniteNumber(value, label) {
  if (typeof value !== "number" || !Number.isFinite(value)) fail(`${label} must be a finite number`);
}

function assertExactString(value, expected, label) {
  if (value !== expected) fail(`${label} must be exactly ${JSON.stringify(expected)}`);
}

function assertProjectShape(project, label) {
  assertObject(project, label);
  exactKeys(project, PROJECT_KEYS, label);
  assertExactString(project.app, "Syndocal", `${label}.app`);
  if (project.version !== 1) fail(`${label}.version must be exactly 1`);
  const snapshot = assertObject(project.snapshot, `${label}.snapshot`);
  exactKeys(snapshot, SNAPSHOT_KEYS, `${label}.snapshot`);
  const video = assertObject(snapshot.video, `${label}.snapshot.video`);
  exactKeys(video, VIDEO_KEYS, `${label}.snapshot.video`);
  return { snapshot, video };
}

function assertDmxRouteShape(route, label) {
  assertObject(route, label);
  exactKeys(route, DMX_ROUTE_KEYS, label);
  if (typeof route.enabled !== "boolean") fail(`${label}.enabled must be boolean`);
  if (!Number.isSafeInteger(route.port) || !Number.isSafeInteger(route.universe) || !Number.isSafeInteger(route.serial_baud_rate)) {
    fail(`${label} numeric route fields must be safe integers`);
  }
  assertExactString(route.target_ip, "127.0.0.1", `${label}.target_ip`);
  assertExactString(route.serial_port, "", `${label}.serial_port`);
}

function expectedDisabledSerialRoute() {
  return {
    enabled: false,
    port: 6454,
    protocol: "EnttecOpenDmx",
    serial_baud_rate: 250000,
    serial_port: "",
    target_ip: "127.0.0.1",
    universe: 0,
  };
}

function expectedDisabledArtNetRoute() {
  return { ...expectedDisabledSerialRoute(), protocol: "ArtNet" };
}

function assertSourceDmxContract(snapshot) {
  assertDmxRouteShape(snapshot.output, "source snapshot.output");
  const routes = assertArray(snapshot.dmx_outputs, "source snapshot.dmx_outputs");
  if (routes.length !== 1) fail("source snapshot.dmx_outputs must contain exactly one route");
  assertDmxRouteShape(routes[0], "source snapshot.dmx_outputs[0]");
  const expected = expectedDisabledSerialRoute();
  if (!equalJson(snapshot.output, expected) || !equalJson(routes[0], expected) || !equalJson(snapshot.output, routes[0])) {
    fail("source DMX routes must be the exact disabled alpha9 EnttecOpenDmx/127.0.0.1:6454/U0 route");
  }
}

function assertSourceVideoContract(video) {
  const compositions = assertArray(video.compositions, "source snapshot.video.compositions");
  if (compositions.length !== 1) fail("source snapshot.video.compositions must contain exactly one existing composition");
  exactKeys(compositions[0], COMPOSITION_KEYS, "source snapshot.video.compositions[0]");
  if (compositions[0].id !== 1 || compositions[0].label !== "Main" || !equalJson(compositions[0].layer_ids, []) || !equalJson(compositions[0].output_ids, [])) {
    fail("source Main composition must be the existing id 1 composition with no outputs");
  }
  const outputs = assertArray(video.outputs, "source snapshot.video.outputs");
  if (outputs.length !== 0) fail("source snapshot.video.outputs must be empty; existing outputs are ambiguous");
}

function assertSourceProjectContract(project) {
  const { snapshot, video } = assertProjectShape(project, "source project");
  assertSourceDmxContract(snapshot);
  assertSourceVideoContract(video);
  if (!Array.isArray(snapshot.fixtures) || snapshot.fixtures.length !== 46) fail("source snapshot.fixtures must contain exactly 46 fixtures");
  if (!Array.isArray(snapshot.timeline_bank) || snapshot.timeline_bank.length !== 2) fail("source snapshot.timeline_bank must contain the two authored Timelines");
  if (!Array.isArray(video.media_assets) || video.media_assets.length !== 2) fail("source snapshot.video.media_assets must contain the two reference assets");
}

function defaultVideoOutputMapping() {
  return {
    stage_x: 0.0,
    stage_y: 0.0,
    stage_z: 0.0,
    offset_x: 0.0,
    offset_y: 0.0,
    scale_x: 1.0,
    scale_y: 1.0,
    rotation_deg: 0.0,
    aspect_ratio: 1.0,
    aspect_mode: "Stretch",
    lens_distortion: 0.0,
    edge_blend_left: 0.0,
    edge_blend_right: 0.0,
    edge_blend_top: 0.0,
    edge_blend_bottom: 0.0,
    edge_blend_gamma: 2.2,
    black_level: 0.0,
    mask_point_count: 0,
    mask_invert: false,
    mask_softness: 0.0,
    mask_points: Array.from({ length: 8 }, () => ({ x: 0.0, y: 0.0 })),
    bitmap_mask_width: 0,
    bitmap_mask_height: 0,
    bitmap_mask_luma_words: Array(32).fill(0),
    keystone_x: 0.0,
    keystone_y: 0.0,
    corner_top_left_x: 0.0,
    corner_top_left_y: 0.0,
    corner_top_right_x: 0.0,
    corner_top_right_y: 0.0,
    corner_bottom_right_x: 0.0,
    corner_bottom_right_y: 0.0,
    corner_bottom_left_x: 0.0,
    corner_bottom_left_y: 0.0,
  };
}

function buildSpoutOutput(id, name) {
  return {
    id,
    label: name,
    kind: "SpoutSender",
    enabled: true,
    composition_id: 1,
    fullscreen: false,
    monitor_id: null,
    monitor_identity: null,
    width: 1920,
    height: 1080,
    endpoint_name: name,
    opacity: 1.0,
    blackout: false,
    mapping: defaultVideoOutputMapping(),
  };
}

function assertMapping(mapping, label) {
  assertObject(mapping, label);
  exactKeys(mapping, VIDEO_MAPPING_KEYS, label);
  assertExactString(mapping.aspect_mode, "Stretch", `${label}.aspect_mode`);
  if (mapping.scale_x !== 1 || mapping.scale_y !== 1 || mapping.aspect_ratio !== 1 || mapping.edge_blend_gamma !== 2.2) {
    fail(`${label} must be the exact VideoOutputMapping default scale/aspect/gamma`);
  }
  for (const field of ["stage_x", "stage_y", "stage_z", "offset_x", "offset_y", "rotation_deg", "lens_distortion", "edge_blend_left", "edge_blend_right", "edge_blend_top", "edge_blend_bottom", "black_level", "mask_softness", "keystone_x", "keystone_y", "corner_top_left_x", "corner_top_left_y", "corner_top_right_x", "corner_top_right_y", "corner_bottom_right_x", "corner_bottom_right_y", "corner_bottom_left_x", "corner_bottom_left_y"]) {
    assertFiniteNumber(mapping[field], `${label}.${field}`);
  }
  if (mapping.mask_point_count !== 0 || mapping.mask_invert !== false || mapping.bitmap_mask_width !== 0 || mapping.bitmap_mask_height !== 0 || !equalJson(mapping.mask_points, defaultVideoOutputMapping().mask_points) || !equalJson(mapping.bitmap_mask_luma_words, Array(32).fill(0))) {
    fail(`${label} must contain the disabled default masks`);
  }
}

function expectedSpoutOutput(id, name) {
  return buildSpoutOutput(id, name);
}

function assertOutputVideoContract(video) {
  const compositions = assertArray(video.compositions, "output snapshot.video.compositions");
  if (compositions.length !== 1) fail("output snapshot.video.compositions must contain exactly one composition");
  exactKeys(compositions[0], COMPOSITION_KEYS, "output snapshot.video.compositions[0]");
  if (compositions[0].id !== 1 || compositions[0].label !== "Main" || !equalJson(compositions[0].layer_ids, []) || !equalJson(compositions[0].output_ids, [1, 2])) {
    fail("output Main composition must reference exactly output IDs [1,2]");
  }
  const outputs = assertArray(video.outputs, "output snapshot.video.outputs");
  if (outputs.length !== 2) fail("output snapshot.video.outputs must contain exactly two outputs");
  for (const [index, output] of outputs.entries()) {
    const id = index + 1;
    const name = index === 0 ? "Syndocal Background" : "Syndocal Foreground";
    exactKeys(output, VIDEO_OUTPUT_KEYS, `output snapshot.video.outputs[${index}]`);
    if (!equalJson(output, expectedSpoutOutput(id, name))) {
      fail(`output snapshot.video.outputs[${index}] is not the exact ${name} SpoutSender contract`);
    }
    assertMapping(output.mapping, `output snapshot.video.outputs[${index}].mapping`);
  }
}

function assertOutputDmxContract(snapshot) {
  assertDmxRouteShape(snapshot.output, "output snapshot.output");
  const routes = assertArray(snapshot.dmx_outputs, "output snapshot.dmx_outputs");
  if (routes.length !== 1) fail("output snapshot.dmx_outputs must contain exactly one route");
  assertDmxRouteShape(routes[0], "output snapshot.dmx_outputs[0]");
  const expected = expectedDisabledArtNetRoute();
  if (!equalJson(snapshot.output, expected) || !equalJson(routes[0], expected) || !equalJson(snapshot.output, routes[0])) {
    fail("output DMX routes must be the exact disabled ArtNet/127.0.0.1:6454/U0 route");
  }
}

function changedPaths(left, right, path = "") {
  if (equalJson(left, right)) return [];
  if (Array.isArray(left) || Array.isArray(right)) {
    if (!Array.isArray(left) || !Array.isArray(right) || left.length !== right.length) return [path || "$"];
    return left.flatMap((entry, index) => changedPaths(entry, right[index], `${path}[${index}]`));
  }
  if (!isObject(left) || !isObject(right)) return [path || "$"].filter(Boolean);
  const keys = [...new Set([...Object.keys(left), ...Object.keys(right)])].sort();
  return keys.flatMap((key) => changedPaths(left[key], right[key], path ? `${path}.${key}` : key));
}

export function getChangedPaths(sourceProject, outputProject) {
  return changedPaths(sourceProject, outputProject).sort();
}

function assertOnlyAllowedChanges(sourceProject, outputProject) {
  const actual = getChangedPaths(sourceProject, outputProject);
  const unexpected = actual.filter((path) => !SAME_PC_ALLOWED_CHANGED_PATHS.includes(path));
  if (unexpected.length > 0) fail(`output changes non-allowlisted paths: ${unexpected.join(", ")}`);
  const missing = SAME_PC_ALLOWED_CHANGED_PATHS.filter((path) => !actual.includes(path));
  if (missing.length > 0) fail(`output is missing required allowlisted changes: ${missing.join(", ")}`);
  return actual;
}

export function validateSamePcOutputProject(outputInput, sourceInput) {
  const source = sourceInput;
  if (!source) fail("source project is required for same-PC output validation");
  assertSourceProjectContract(source);
  const output = assertObject(outputInput, "same-PC output project");
  assertProjectShape(output, "same-PC output project");
  assertOutputDmxContract(output.snapshot);
  assertOutputVideoContract(output.snapshot.video);
  const actual = assertOnlyAllowedChanges(source, output);
  return { status: "PASS", changedPaths: actual };
}

export function assertApprovedSamePcSource(sourcePath, sourceBytes, sourceProject) {
  if (normalizeIdentityPath(sourcePath) !== normalizeIdentityPath(SAME_PC_SOURCE_PATH)) {
    fail(`source must be the exact approved alpha9 path ${SAME_PC_SOURCE_PATH}`);
  }
  if (!(sourceBytes instanceof Uint8Array)) fail("source bytes must be a Uint8Array");
  if (sourceBytes.byteLength !== SAME_PC_SOURCE_BYTE_SIZE) fail(`source byte length must be exactly ${SAME_PC_SOURCE_BYTE_SIZE}`);
  const actualHash = sha256(sourceBytes);
  if (actualHash !== SAME_PC_SOURCE_SHA256) fail(`source SHA-256 ${actualHash} does not match the approved alpha9 identity`);
  const parsed = parseJsonText(decodeUtf8Strict(sourceBytes, "approved alpha9 source"), "approved alpha9 source");
  assertSourceProjectContract(parsed);
  if (sourceProject !== undefined && !equalJson(sourceProject, parsed)) fail("parsed source project differs from the approved source bytes");
  return { byteSize: sourceBytes.byteLength, sha256: actualHash };
}

export function authorSamePcOutputProject(sourceProject, options = {}) {
  const sourcePath = options.sourcePath ?? SAME_PC_SOURCE_PATH;
  const sourceBytes = options.sourceBytes;
  assertApprovedSamePcSource(sourcePath, sourceBytes, sourceProject);
  const output = structuredClone(sourceProject);
  output.snapshot.output.protocol = "ArtNet";
  output.snapshot.dmx_outputs[0].protocol = "ArtNet";
  output.snapshot.video.outputs = [
    buildSpoutOutput(1, "Syndocal Background"),
    buildSpoutOutput(2, "Syndocal Foreground"),
  ];
  output.snapshot.video.compositions[0].output_ids = [1, 2];
  validateSamePcOutputProject(output, sourceProject);
  return output;
}

function targetIdentity(info) {
  return {
    device: String(info.dev),
    fileIndex: String(info.ino),
    byteSize: String(info.size),
  };
}

function sameTargetIdentity(left, right) {
  return left.device === right.device && left.fileIndex === right.fileIndex && left.byteSize === right.byteSize;
}

async function captureCreatedTarget(path, guard, expectedBytes) {
  const parent = dirname(path);
  const canonicalParent = await realpath(parent).catch((error) => fail(`created output parent could not be revalidated: ${String(error?.message ?? error)}`));
  if (normalizeIdentityPath(canonicalParent) !== normalizeIdentityPath(parent) || normalizeIdentityPath(canonicalParent) !== normalizeIdentityPath(guard.parentPath)) {
    fail(`created output parent identity changed; target quarantined at ${path}`);
  }
  const observed = await readRegularFileBytesNoReparse(path, "created same-PC output", MAX_BASE_BYTES, { guard });
  if (!Buffer.from(observed.bytes).equals(Buffer.from(expectedBytes))) fail(`created output bytes differ from the exact expected publication; target quarantined at ${path}`);
  const info = await lstat(path, { bigint: true }).catch((error) => fail(`created output identity could not be revalidated: ${String(error?.message ?? error)}`));
  if (!info.isFile() || info.isSymbolicLink()) fail(`created output is not a regular non-reparse file; target quarantined at ${path}`);
  const canonicalPath = await realpath(path).catch((error) => fail(`created output path could not be revalidated: ${String(error?.message ?? error)}`));
  if (normalizeIdentityPath(canonicalPath) !== normalizeIdentityPath(path)) fail(`created output path is a reparse or symlink; target quarantined at ${path}`);
  return { identity: targetIdentity(info), bytes: Buffer.from(observed.bytes) };
}

async function cleanupCreatedTarget(path, guard, expectedBytes, createdIdentity, testHooks = null) {
  if (!createdIdentity) fail(`created output identity was never established; manual removal required for ${path}`);
  await testHooks?.beforeCleanupVerification?.({ outputPath: path });
  const observed = await captureCreatedTarget(path, guard, expectedBytes);
  if (!sameTargetIdentity(observed.identity, createdIdentity)) {
    fail(`created output identity changed; refusing to delete substituted target ${path}`);
  }
  // A second exact identity/bytes check is intentional: the first check proves
  // the publication result, while this one is the deletion admission fence.
  const finalObserved = await captureCreatedTarget(path, guard, expectedBytes);
  if (!sameTargetIdentity(finalObserved.identity, createdIdentity)) {
    fail(`created output identity changed before cleanup; refusing to delete substituted target ${path}`);
  }
  await testHooks?.beforeCleanupDelete?.({ outputPath: path, identity: finalObserved.identity });
  const afterDeleteFence = await captureCreatedTarget(path, guard, expectedBytes);
  if (!sameTargetIdentity(afterDeleteFence.identity, createdIdentity)) {
    fail(`created output identity changed during cleanup; refusing to delete substituted target ${path}`);
  }
  // Node's path-based unlink has no identity-bound delete primitive.  Never
  // turn the final check into a deletion race: retain the exact target and
  // require an operator to quarantine/remove it explicitly.
  fail(`automatic cleanup is unavailable without an identity-bound delete; manual removal required for ${path}`);
}

export async function createSamePcOutputCopy({ sourcePath = SAME_PC_SOURCE_PATH, outputPath = SAME_PC_OUTPUT_PATH, testHooks = null } = {}) {
  if (extname(outputPath).toLowerCase() !== ".sdc") fail("same-PC output must use the .sdc extension");
  if (normalizeIdentityPath(sourcePath) === normalizeIdentityPath(outputPath)) fail("same-PC output must not overwrite alpha9 source");
  const sourceFile = await readStrictJsonFile(sourcePath, "approved alpha9 source", MAX_BASE_BYTES);
  const output = authorSamePcOutputProject(sourceFile.value, { sourcePath: sourceFile.absolute, sourceBytes: sourceFile.bytes });
  const outputText = `${JSON.stringify(output, null, 2)}\n`;
  const outputGuard = {};
  const preparedOutputPath = await prepareOutputPath(outputPath, { guard: outputGuard });
  const expectedBytes = Buffer.from(outputText, "utf8");
  let published = false;
  let createdIdentity = null;
  try {
    await writeExclusive(preparedOutputPath, outputText, outputGuard);
    published = true;
    const created = await captureCreatedTarget(preparedOutputPath, outputGuard, expectedBytes);
    createdIdentity = created.identity;
    await testHooks?.afterWrite?.({ outputPath: preparedOutputPath, expectedBytes: Buffer.from(expectedBytes) });
    const current = await captureCreatedTarget(preparedOutputPath, outputGuard, expectedBytes);
    if (!sameTargetIdentity(current.identity, createdIdentity)) fail(`created output identity changed after write; target quarantined at ${preparedOutputPath}`);
    const observedText = decodeUtf8Strict(current.bytes, "written same-PC output");
    const postWriteText = await testHooks?.postWriteText?.({ outputPath: preparedOutputPath, text: observedText }) ?? observedText;
    const reparsed = parseJsonText(postWriteText, "written same-PC output");
    const postWriteProject = await testHooks?.postWriteProject?.({ outputPath: preparedOutputPath, project: reparsed }) ?? reparsed;
    validateSamePcOutputProject(postWriteProject, sourceFile.value);
  } catch (error) {
    if (published) {
      try {
        await cleanupCreatedTarget(preparedOutputPath, outputGuard, expectedBytes, createdIdentity, testHooks);
      } catch (cleanupError) {
        throw new Error(`${String(error?.message ?? error)}; created output cleanup failed; quarantine/manual removal required for ${preparedOutputPath}: ${String(cleanupError?.message ?? cleanupError)}`, { cause: error });
      }
    }
    throw error;
  }
  return {
    sourcePath: sourceFile.absolute,
    outputPath: preparedOutputPath,
    outputByteSize: expectedBytes.byteLength,
    changedPaths: SAME_PC_ALLOWED_CHANGED_PATHS,
  };
}

export function parseCliArgs(argv = process.argv.slice(2)) {
  const usage = "usage: node tools/create-dsf2026-same-pc-output-copy.mjs --output <new .sdc> [--source <approved alpha9 .sdc>]";
  if (argv.length < 2 || argv.length % 2 !== 0) fail(usage);
  const options = {};
  for (let index = 0; index < argv.length; index += 2) {
    const flag = argv[index];
    const value = argv[index + 1];
    if (!["--source", "--output"].includes(flag) || !value || value.startsWith("--") || options[flag]) fail(usage);
    options[flag] = value;
  }
  if (!options["--output"]) fail(usage);
  return { source: options["--source"] ?? SAME_PC_SOURCE_PATH, output: resolve(options["--output"]) };
}

if (resolve(process.argv[1] ?? "") === fileURLToPath(import.meta.url)) {
  createSamePcOutputCopy(parseCliArgs()).then((result) => {
    console.log(`created same-PC output copy: ${result.outputPath}`);
  }).catch((error) => {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  });
}
