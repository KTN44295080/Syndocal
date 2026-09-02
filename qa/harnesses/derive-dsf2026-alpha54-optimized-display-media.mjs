import { createHash } from "node:crypto";
import { dirname, extname, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

import {
  decodeUtf8Strict,
  equalJson,
  fail,
  parseJsonText,
} from "../../tools/dsf2026/common.mjs";
import {
  MAX_BASE_BYTES,
  normalizeIdentityPath,
  prepareOutputPath,
  readRegularFileBytesNoReparse,
  writeExclusiveBytes,
} from "../../tools/dsf2026/io.mjs";

const REPO_ROOT = resolve(fileURLToPath(new URL("../../", import.meta.url)));

export const SOURCE_PATH = resolve(
  REPO_ROOT,
  "target/qa/DSF2026-show-alpha53-complete-show.sdc",
);
export const SOURCE_BYTE_SIZE = 1_112_369;
export const SOURCE_SHA256 =
  "46D79EEA2A0562D4CB385F4BA9AA6E721FC741D081533D95DA073EB2099F016E";

export const REPLACEMENT_MEDIA_PATH = resolve(
  REPO_ROOT,
  "target/qa/logo-anim-dark-1920x1080-yuv420p30.mp4",
);
export const REPLACEMENT_MEDIA_BYTE_SIZE = 1_092_472;
export const REPLACEMENT_MEDIA_SHA256 =
  "20B7187D87D4B2EDF1B306C160FA37788BB895781F8A35E6913E779BA6F35F2D";
export const REPLACEMENT_MEDIA_METADATA = Object.freeze({
  duration_ms: 3_000,
  frame_rate: 30,
  has_audio: false,
  height: 1_080,
  width: 1_920,
});

export const OUTPUT_PATH = resolve(
  REPO_ROOT,
  "target/qa/DSF2026-show-alpha54-optimized-display-media.sdc",
);
export const OUTPUT_BYTE_SIZE = 1_112_448;
export const OUTPUT_SHA256 =
  "D20A7891C1D2DBB6E1E41BA1291C4F1D9C13E348237ABD16F20FF059CA3D1347";

export const FIXED_FOREGROUND_LAYER_ID = 3;
export const FIXED_FOREGROUND_MEDIA_ASSET_ID = 5;
export const ORIGINAL_FOREGROUND_DURATION_MS = 3_008;
export const OPTIMIZED_FOREGROUND_DURATION_MS = 3_000;

export const ALLOWED_CHANGED_PATHS = Object.freeze([
  "snapshot.video.layers[2].clip_slots[0].out_point_ms",
  "snapshot.video.layers[2].source.metadata.duration_ms",
  "snapshot.video.layers[2].source.metadata.has_audio",
  "snapshot.video.layers[2].source.metadata.width",
  "snapshot.video.layers[2].source.path",
  "snapshot.video.layers[2].state.loop_end_ms",
  "snapshot.video.media_assets[4].byte_size",
  "snapshot.video.media_assets[4].content_hash.hex",
  "snapshot.video.media_assets[4].source.metadata.duration_ms",
  "snapshot.video.media_assets[4].source.metadata.has_audio",
  "snapshot.video.media_assets[4].source.metadata.width",
  "snapshot.video.media_assets[4].source.path",
]);

function sha256(bytes) {
  return createHash("sha256").update(bytes).digest("hex").toUpperCase();
}

function changedPaths(left, right, path = "") {
  if (Object.is(left, right)) return [];
  if (Array.isArray(left) || Array.isArray(right)) {
    if (!Array.isArray(left) || !Array.isArray(right) || left.length !== right.length) {
      return [path || "$"].filter(Boolean);
    }
    return left.flatMap((entry, index) => changedPaths(entry, right[index], `${path}[${index}]`));
  }
  const leftObject = left !== null && typeof left === "object";
  const rightObject = right !== null && typeof right === "object";
  if (!leftObject || !rightObject) return [path || "$"].filter(Boolean);
  const keys = [...new Set([...Object.keys(left), ...Object.keys(right)])].sort();
  return keys.flatMap((key) => changedPaths(left[key], right[key], path ? `${path}.${key}` : key));
}

function assertDisplayTopology(project, label) {
  const video = project?.snapshot?.video;
  if (!video || !Array.isArray(video.media_assets) || !Array.isArray(video.layers)) {
    fail(`${label} must contain the expected video model`);
  }
  if (video.media_assets.length !== 5 || video.layers.length !== 3) {
    fail(`${label} must retain exactly five media assets and three video layers`);
  }
  if (!Array.isArray(video.outputs) || video.outputs.length !== 2 || !Array.isArray(video.compositions) || video.compositions.length !== 3) {
    fail(`${label} must retain exactly two Display outputs and three compositions`);
  }
  const displays = video.outputs.map((output) => ({
    id: output.id,
    kind: output.kind,
    composition_id: output.composition_id,
    width: output.width,
    height: output.height,
  }));
  const expectedDisplays = [
    { id: 3, kind: "Display", composition_id: 2, width: 1_920, height: 1_080 },
    { id: 4, kind: "Display", composition_id: 3, width: 3_840, height: 2_160 },
  ];
  if (!equalJson(displays, expectedDisplays)) fail(`${label} Display output topology changed`);
  const compositions = video.compositions.map((composition) => ({
    id: composition.id,
    label: composition.label,
    layer_ids: composition.layer_ids,
    output_ids: composition.output_ids,
    timeline_layer_ids: composition.timeline_layer_ids,
  }));
  const expectedCompositions = [
    { id: 1, label: "Main", layer_ids: [1, 2, 3], output_ids: [], timeline_layer_ids: [] },
    { id: 2, label: "Foreground Video 1", layer_ids: [3], output_ids: [3], timeline_layer_ids: [] },
    { id: 3, label: "Background Video2 Camera", layer_ids: [2], output_ids: [4], timeline_layer_ids: [{ layer_id: 3, timeline_id: 209 }] },
  ];
  if (!equalJson(compositions, expectedCompositions)) fail(`${label} composition topology changed`);
}

function foregroundParts(project, label) {
  assertDisplayTopology(project, label);
  const video = project.snapshot.video;
  const asset = video.media_assets[FIXED_FOREGROUND_MEDIA_ASSET_ID - 1];
  const layer = video.layers[FIXED_FOREGROUND_LAYER_ID - 1];
  if (
    asset?.id !== FIXED_FOREGROUND_MEDIA_ASSET_ID
    || layer?.id !== FIXED_FOREGROUND_LAYER_ID
    || layer?.media_asset_id !== FIXED_FOREGROUND_MEDIA_ASSET_ID
  ) {
    fail(`${label} foreground asset/layer identity must remain exactly asset 5 / layer 3`);
  }
  if (asset.label !== "logo-anim-dark" || layer.label !== "logo-anim-dark") {
    fail(`${label} foreground asset/layer label changed`);
  }
  if (asset.source?.kind !== "File" || layer.source?.kind !== "File") {
    fail(`${label} foreground source must remain file-backed`);
  }
  if (!Array.isArray(layer.clip_slots) || layer.clip_slots.length !== 1 || layer.default_clip_slot_id !== 3) {
    fail(`${label} foreground must retain its single default clip slot`);
  }
  const slot = layer.clip_slots[0];
  return { asset, layer, slot, state: layer.state };
}

export function assertAlpha53SourceShape(project) {
  const { asset, layer, slot, state } = foregroundParts(project, "alpha53 source");
  const oldMetadata = {
    duration_ms: ORIGINAL_FOREGROUND_DURATION_MS,
    frame_rate: 30,
    has_audio: true,
    height: 1_080,
    width: 1_280,
  };
  if (normalizeIdentityPath(asset.source.path) !== normalizeIdentityPath(resolve("C:/Users/kouty/Downloads/logo-anim-dark.mp4")) || normalizeIdentityPath(layer.source.path) !== normalizeIdentityPath(resolve("C:/Users/kouty/Downloads/logo-anim-dark.mp4"))) {
    fail("alpha53 source foreground media path changed");
  }
  if (!equalJson(asset.source.metadata, oldMetadata) || !equalJson(layer.source.metadata, oldMetadata)) {
    fail("alpha53 source foreground metadata changed");
  }
  if (asset.byte_size !== 630_538 || asset.content_hash?.algorithm !== "Sha256" || asset.content_hash?.hex !== "bb2e52abf4199716ddb8f9e6180f8343a0affa3474f22c363ca05207b19b89b9") {
    fail("alpha53 source foreground asset identity changed");
  }
  if (!state || state.playing !== true || state.loop_enabled !== true || state.loop_start_ms !== 0 || state.loop_end_ms !== ORIGINAL_FOREGROUND_DURATION_MS || state.position_ms !== 0) {
    fail("alpha53 source foreground loop state changed");
  }
  if (slot.in_point_ms !== 0 || slot.loop_mode !== "Loop" || slot.out_point_ms !== ORIGINAL_FOREGROUND_DURATION_MS) {
    fail("alpha53 source foreground clip slot changed");
  }
  return { asset, layer, slot, state };
}

function assertOptimizedForegroundShape(project) {
  const { asset, layer, slot, state } = foregroundParts(project, "derived alpha54 project");
  if (normalizeIdentityPath(asset.source.path) !== normalizeIdentityPath(REPLACEMENT_MEDIA_PATH) || normalizeIdentityPath(layer.source.path) !== normalizeIdentityPath(REPLACEMENT_MEDIA_PATH)) {
    fail("derived alpha54 foreground media path is not the fixed optimized asset");
  }
  if (!equalJson(asset.source.metadata, REPLACEMENT_MEDIA_METADATA) || !equalJson(layer.source.metadata, REPLACEMENT_MEDIA_METADATA)) {
    fail("derived alpha54 foreground metadata is not the exact optimized 1920x1080 asset");
  }
  if (asset.byte_size !== REPLACEMENT_MEDIA_BYTE_SIZE || asset.content_hash?.algorithm !== "Sha256" || asset.content_hash?.hex !== REPLACEMENT_MEDIA_SHA256.toLowerCase()) {
    fail("derived alpha54 foreground asset bytes do not match the fixed optimized media");
  }
  if (!state || state.playing !== true || state.loop_enabled !== true || state.loop_start_ms !== 0 || state.loop_end_ms !== OPTIMIZED_FOREGROUND_DURATION_MS || state.position_ms !== 0) {
    fail("derived alpha54 foreground loop_end state is not exactly 3000 ms");
  }
  if (slot.in_point_ms !== 0 || slot.loop_mode !== "Loop" || slot.out_point_ms !== OPTIMIZED_FOREGROUND_DURATION_MS) {
    fail("derived alpha54 foreground slot out_point is not exactly 3000 ms");
  }
}

export function deriveOptimizedDisplayMediaProject(sourceProject) {
  assertAlpha53SourceShape(sourceProject);
  const output = structuredClone(sourceProject);
  const { asset, layer, slot, state } = foregroundParts(output, "derived alpha54 project");
  asset.byte_size = REPLACEMENT_MEDIA_BYTE_SIZE;
  asset.content_hash.hex = REPLACEMENT_MEDIA_SHA256.toLowerCase();
  asset.source.path = REPLACEMENT_MEDIA_PATH;
  asset.source.metadata = structuredClone(REPLACEMENT_MEDIA_METADATA);
  layer.source.path = REPLACEMENT_MEDIA_PATH;
  layer.source.metadata = structuredClone(REPLACEMENT_MEDIA_METADATA);
  state.loop_end_ms = OPTIMIZED_FOREGROUND_DURATION_MS;
  slot.out_point_ms = OPTIMIZED_FOREGROUND_DURATION_MS;

  const actualChangedPaths = changedPaths(sourceProject, output).sort();
  const expectedChangedPaths = [...ALLOWED_CHANGED_PATHS].sort();
  if (!equalJson(actualChangedPaths, expectedChangedPaths)) {
    fail(`derived alpha54 project changed unexpected paths: ${actualChangedPaths.join(", ")}`);
  }
  assertOptimizedForegroundShape(output);
  return { output, changedPaths: actualChangedPaths };
}

async function readExactIdentity(path, label, byteSize, expectedSha256) {
  const file = await readRegularFileBytesNoReparse(path, label, MAX_BASE_BYTES);
  if (file.bytes.byteLength !== byteSize) fail(`${label} byte length must be exactly ${byteSize}; found ${file.bytes.byteLength}`);
  const actualSha256 = sha256(file.bytes);
  if (actualSha256 !== expectedSha256) fail(`${label} SHA-256 ${actualSha256} does not match the approved identity`);
  return { ...file, sha256: actualSha256 };
}

function assertFixedPath(actual, expected, label) {
  if (normalizeIdentityPath(actual) !== normalizeIdentityPath(expected)) fail(`${label} is fixed to ${expected}`);
}

function outputBytesFor(sourceProject) {
  const derived = deriveOptimizedDisplayMediaProject(sourceProject);
  const bytes = Buffer.from(`${JSON.stringify(derived.output, null, 2)}\n`, "utf8");
  return { ...derived, bytes };
}

export async function createOptimizedDisplayMediaProject({
  sourcePath = SOURCE_PATH,
  replacementMediaPath = REPLACEMENT_MEDIA_PATH,
  outputPath = OUTPUT_PATH,
} = {}) {
  assertFixedPath(sourcePath, SOURCE_PATH, "alpha53 source path");
  assertFixedPath(replacementMediaPath, REPLACEMENT_MEDIA_PATH, "optimized replacement media path");
  assertFixedPath(outputPath, OUTPUT_PATH, "alpha54 output path");
  if (extname(outputPath).toLowerCase() !== ".sdc" || normalizeIdentityPath(dirname(outputPath)) === normalizeIdentityPath(REPO_ROOT)) {
    fail("alpha54 output must be the fixed target/qa .sdc child");
  }
  if (normalizeIdentityPath(outputPath) === normalizeIdentityPath(sourcePath) || normalizeIdentityPath(outputPath) === normalizeIdentityPath(replacementMediaPath)) {
    fail("alpha54 output must not overwrite an approved source or media file");
  }

  const source = await readExactIdentity(sourcePath, "protected alpha53 source", SOURCE_BYTE_SIZE, SOURCE_SHA256);
  await readExactIdentity(replacementMediaPath, "optimized replacement media", REPLACEMENT_MEDIA_BYTE_SIZE, REPLACEMENT_MEDIA_SHA256);
  const sourceProject = parseJsonText(decodeUtf8Strict(source.bytes, "protected alpha53 source"), "protected alpha53 source");
  const derived = outputBytesFor(sourceProject);
  if (derived.bytes.byteLength !== OUTPUT_BYTE_SIZE || sha256(derived.bytes) !== OUTPUT_SHA256) {
    fail("derived alpha54 output identity changed before publication");
  }

  const outputGuard = {};
  const prepared = await prepareOutputPath(outputPath, { guard: outputGuard });
  await readExactIdentity(sourcePath, "alpha53 source publication fence", SOURCE_BYTE_SIZE, SOURCE_SHA256);
  await readExactIdentity(replacementMediaPath, "optimized media publication fence", REPLACEMENT_MEDIA_BYTE_SIZE, REPLACEMENT_MEDIA_SHA256);
  let published = false;
  try {
    await writeExclusiveBytes(prepared, derived.bytes, outputGuard, {
      fences: [
        { path: sourcePath, byteSize: SOURCE_BYTE_SIZE, sha256: SOURCE_SHA256 },
        { path: replacementMediaPath, byteSize: REPLACEMENT_MEDIA_BYTE_SIZE, sha256: REPLACEMENT_MEDIA_SHA256 },
      ],
    });
    published = true;
    const written = await readRegularFileBytesNoReparse(prepared, "created alpha54 optimized display project", MAX_BASE_BYTES);
    if (!Buffer.from(written.bytes).equals(derived.bytes)) fail(`created alpha54 output failed byte-for-byte verification; quarantine ${prepared}`);
    await readExactIdentity(sourcePath, "alpha53 source post-publication fence", SOURCE_BYTE_SIZE, SOURCE_SHA256);
    await readExactIdentity(replacementMediaPath, "optimized media post-publication fence", REPLACEMENT_MEDIA_BYTE_SIZE, REPLACEMENT_MEDIA_SHA256);
    const finalWritten = await readRegularFileBytesNoReparse(prepared, "final alpha54 optimized display project", MAX_BASE_BYTES);
    if (!Buffer.from(finalWritten.bytes).equals(derived.bytes)) fail(`created alpha54 output changed after post-publication fences; quarantine ${prepared}`);
    const reparsed = parseJsonText(decodeUtf8Strict(finalWritten.bytes, "created alpha54 optimized display project"), "created alpha54 optimized display project");
    const verified = outputBytesFor(sourceProject);
    if (!equalJson(reparsed, verified.output)) fail("created alpha54 project differs from the exact derivation");
  } catch (error) {
    if (published) {
      throw new Error(
        `${String(error?.message ?? error)}; created local QA output is in-doubt and must not be accepted or regenerated in place; manually quarantine/remove ${prepared} only after verifying its exact path and identity (${OUTPUT_BYTE_SIZE} bytes, SHA-256 ${OUTPUT_SHA256})`,
        { cause: error },
      );
    }
    throw error;
  }
  return {
    status: "PASS",
    sourcePath,
    sourceByteSize: SOURCE_BYTE_SIZE,
    sourceSha256: SOURCE_SHA256,
    replacementMediaPath,
    replacementMediaByteSize: REPLACEMENT_MEDIA_BYTE_SIZE,
    replacementMediaSha256: REPLACEMENT_MEDIA_SHA256,
    outputPath: prepared,
    outputByteSize: OUTPUT_BYTE_SIZE,
    outputSha256: OUTPUT_SHA256,
    changedPaths: derived.changedPaths,
    routesPreserved: true,
    physicalOutputOpened: false,
  };
}

export async function verifyOptimizedDisplayMediaProject() {
  const source = await readExactIdentity(SOURCE_PATH, "protected alpha53 source", SOURCE_BYTE_SIZE, SOURCE_SHA256);
  await readExactIdentity(REPLACEMENT_MEDIA_PATH, "optimized replacement media", REPLACEMENT_MEDIA_BYTE_SIZE, REPLACEMENT_MEDIA_SHA256);
  const output = await readExactIdentity(OUTPUT_PATH, "alpha54 optimized display output", OUTPUT_BYTE_SIZE, OUTPUT_SHA256);
  const sourceProject = parseJsonText(decodeUtf8Strict(source.bytes, "protected alpha53 source"), "protected alpha53 source");
  const outputProject = parseJsonText(decodeUtf8Strict(output.bytes, "alpha54 optimized display output"), "alpha54 optimized display output");
  const derived = outputBytesFor(sourceProject);
  if (!equalJson(outputProject, derived.output)) fail("alpha54 output differs from the exact derivation");
  return {
    status: "PASS",
    sourcePath: SOURCE_PATH,
    sourceByteSize: SOURCE_BYTE_SIZE,
    sourceSha256: SOURCE_SHA256,
    replacementMediaPath: REPLACEMENT_MEDIA_PATH,
    replacementMediaByteSize: REPLACEMENT_MEDIA_BYTE_SIZE,
    replacementMediaSha256: REPLACEMENT_MEDIA_SHA256,
    outputPath: OUTPUT_PATH,
    outputByteSize: OUTPUT_BYTE_SIZE,
    outputSha256: OUTPUT_SHA256,
    changedPaths: derived.changedPaths,
  };
}

function parseCliArgs(argv) {
  if (argv.length === 0) return { verifyOnly: false };
  if (argv.length === 1 && argv[0] === "--verify-only") return { verifyOnly: true };
  fail("usage: node qa/harnesses/derive-dsf2026-alpha54-optimized-display-media.mjs [--verify-only]");
}

async function main() {
  const options = parseCliArgs(process.argv.slice(2));
  console.log(JSON.stringify(options.verifyOnly ? await verifyOptimizedDisplayMediaProject() : await createOptimizedDisplayMediaProject(), null, 2));
}

const invokedDirectly = process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href;
if (invokedDirectly) {
  main().catch((error) => {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  });
}
