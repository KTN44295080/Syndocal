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
  "target/qa/DSF2026-show-alpha54-optimized-display-media.sdc",
);
export const SOURCE_BYTE_SIZE = 1_112_448;
export const SOURCE_SHA256 =
  "D20A7891C1D2DBB6E1E41BA1291C4F1D9C13E348237ABD16F20FF059CA3D1347";

export const REPLACEMENT_MEDIA_PATH = resolve(
  REPO_ROOT,
  "target/qa/syndocal-near-show-projector-3840x2160.mp4",
);
export const REPLACEMENT_MEDIA_BYTE_SIZE = 43_807_726;
export const REPLACEMENT_MEDIA_SHA256 =
  "D51DCD2B4F55CB7A6FAE3F34E9B4D93E6BAC24B0B29A6BE500AB4B848D323E29";
export const REPLACEMENT_MEDIA_METADATA = Object.freeze({
  duration_ms: 8_000,
  frame_rate: 30,
  has_audio: false,
  height: 2_160,
  width: 3_840,
});

export const OUTPUT_PATH = resolve(
  REPO_ROOT,
  "target/qa/DSF2026-show-alpha55-dual-file-display.sdc",
);
export const OUTPUT_BYTE_SIZE = 1_112_438;
export const OUTPUT_SHA256 =
  "F57A98966CD58FE040D2AC3C2A5B41D7CA53F15BE2A6C5A9C117EC918A574459";

export const BACKGROUND_LAYER_ID = 2;
export const BACKGROUND_MEDIA_ASSET_ID = 4;
export const REPLACEMENT_DURATION_MS = 8_000;

export const ALLOWED_CHANGED_PATHS = Object.freeze([
  "snapshot.video.layers[1].clip_slots[0].loop_mode",
  "snapshot.video.layers[1].clip_slots[0].out_point_ms",
  "snapshot.video.layers[1].source.kind",
  "snapshot.video.layers[1].source.metadata",
  "snapshot.video.layers[1].source.name",
  "snapshot.video.layers[1].source.path",
  "snapshot.video.layers[1].state.loop_enabled",
  "snapshot.video.layers[1].state.loop_end_ms",
  "snapshot.video.layers[1].state.opacity",
  "snapshot.video.layers[1].state.playing",
  "snapshot.video.media_assets[3].byte_size",
  "snapshot.video.media_assets[3].content_hash",
  "snapshot.video.media_assets[3].source.kind",
  "snapshot.video.media_assets[3].source.metadata",
  "snapshot.video.media_assets[3].source.name",
  "snapshot.video.media_assets[3].source.path",
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
  const displays = video.outputs?.map((output) => ({
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
  const compositions = video.compositions?.map((composition) => ({
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

function backgroundParts(project, label) {
  assertDisplayTopology(project, label);
  const video = project.snapshot.video;
  const asset = video.media_assets[BACKGROUND_MEDIA_ASSET_ID - 1];
  const layer = video.layers[BACKGROUND_LAYER_ID - 1];
  if (
    asset?.id !== BACKGROUND_MEDIA_ASSET_ID
    || layer?.id !== BACKGROUND_LAYER_ID
    || layer?.media_asset_id !== BACKGROUND_MEDIA_ASSET_ID
  ) {
    fail(`${label} background identity must remain exactly asset 4 / layer 2`);
  }
  if (!Array.isArray(layer.clip_slots) || layer.clip_slots.length !== 1 || layer.default_clip_slot_id !== 2) {
    fail(`${label} background must retain its single default clip slot`);
  }
  return { asset, layer, slot: layer.clip_slots[0], state: layer.state };
}

export function assertAlpha54SourceShape(project) {
  const { asset, layer, slot, state } = backgroundParts(project, "alpha54 source");
  if (asset.label !== "Layer 3" || layer.label !== "Layer 3") {
    fail("alpha54 source background labels changed");
  }
  if (asset.source?.kind !== "Camera" || layer.source?.kind !== "Camera") {
    fail("alpha54 source background must remain camera-backed");
  }
  if (asset.byte_size !== undefined || asset.content_hash !== undefined) {
    fail("alpha54 source camera must not synthesize a file identity");
  }
  if (!state || state.enabled !== true || state.opacity !== 0 || state.playing !== false || state.loop_enabled !== false || state.loop_end_ms !== 0) {
    fail("alpha54 source camera state changed");
  }
  if (slot.in_point_ms !== 0 || slot.loop_mode !== "Once" || slot.out_point_ms !== undefined) {
    fail("alpha54 source camera clip slot changed");
  }
}

function expectedFileSource() {
  return {
    codec: null,
    kind: "File",
    metadata: structuredClone(REPLACEMENT_MEDIA_METADATA),
    name: null,
    path: REPLACEMENT_MEDIA_PATH,
  };
}

function assertDualFileShape(project) {
  const { asset, layer, slot, state } = backgroundParts(project, "derived alpha55 project");
  const fileSource = expectedFileSource();
  if (!equalJson(asset.source, fileSource) || !equalJson(layer.source, fileSource)) {
    fail("derived alpha55 background source is not the exact 3840x2160 file");
  }
  if (asset.byte_size !== REPLACEMENT_MEDIA_BYTE_SIZE || asset.content_hash?.algorithm !== "Sha256" || asset.content_hash?.hex !== REPLACEMENT_MEDIA_SHA256.toLowerCase()) {
    fail("derived alpha55 background asset bytes do not match the fixed media");
  }
  if (!state || state.enabled !== true || state.opacity !== 1 || state.playing !== true || state.loop_enabled !== true || state.loop_start_ms !== 0 || state.loop_end_ms !== REPLACEMENT_DURATION_MS || state.position_ms !== 0) {
    fail("derived alpha55 background playback state is not the exact looped file state");
  }
  if (slot.in_point_ms !== 0 || slot.loop_mode !== "Loop" || slot.out_point_ms !== REPLACEMENT_DURATION_MS) {
    fail("derived alpha55 background clip slot is not the exact full-duration loop");
  }
}

export function deriveDualFileDisplayProject(sourceProject) {
  assertAlpha54SourceShape(sourceProject);
  const output = structuredClone(sourceProject);
  const { asset, layer, slot, state } = backgroundParts(output, "derived alpha55 project");
  asset.byte_size = REPLACEMENT_MEDIA_BYTE_SIZE;
  asset.content_hash = { algorithm: "Sha256", hex: REPLACEMENT_MEDIA_SHA256.toLowerCase() };
  asset.source = expectedFileSource();
  layer.source = expectedFileSource();
  state.loop_enabled = true;
  state.loop_end_ms = REPLACEMENT_DURATION_MS;
  state.opacity = 1;
  state.playing = true;
  slot.loop_mode = "Loop";
  slot.out_point_ms = REPLACEMENT_DURATION_MS;

  const actualChangedPaths = changedPaths(sourceProject, output).sort();
  const expectedChangedPaths = [...ALLOWED_CHANGED_PATHS].sort();
  if (!equalJson(actualChangedPaths, expectedChangedPaths)) {
    fail(`derived alpha55 project changed unexpected paths: ${actualChangedPaths.join(", ")}`);
  }
  assertDualFileShape(output);
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
  const derived = deriveDualFileDisplayProject(sourceProject);
  const bytes = Buffer.from(`${JSON.stringify(derived.output, null, 2)}\n`, "utf8");
  return { ...derived, bytes };
}

export async function createDualFileDisplayProject({
  sourcePath = SOURCE_PATH,
  replacementMediaPath = REPLACEMENT_MEDIA_PATH,
  outputPath = OUTPUT_PATH,
} = {}) {
  assertFixedPath(sourcePath, SOURCE_PATH, "alpha54 source path");
  assertFixedPath(replacementMediaPath, REPLACEMENT_MEDIA_PATH, "background replacement media path");
  assertFixedPath(outputPath, OUTPUT_PATH, "alpha55 output path");
  if (extname(outputPath).toLowerCase() !== ".sdc" || normalizeIdentityPath(dirname(outputPath)) === normalizeIdentityPath(REPO_ROOT)) {
    fail("alpha55 output must be the fixed target/qa .sdc child");
  }
  if (normalizeIdentityPath(outputPath) === normalizeIdentityPath(sourcePath) || normalizeIdentityPath(outputPath) === normalizeIdentityPath(replacementMediaPath)) {
    fail("alpha55 output must not overwrite an approved source or media file");
  }

  const source = await readExactIdentity(sourcePath, "protected alpha54 source", SOURCE_BYTE_SIZE, SOURCE_SHA256);
  await readExactIdentity(replacementMediaPath, "background replacement media", REPLACEMENT_MEDIA_BYTE_SIZE, REPLACEMENT_MEDIA_SHA256);
  const sourceProject = parseJsonText(decodeUtf8Strict(source.bytes, "protected alpha54 source"), "protected alpha54 source");
  const derived = outputBytesFor(sourceProject);
  if (derived.bytes.byteLength !== OUTPUT_BYTE_SIZE || sha256(derived.bytes) !== OUTPUT_SHA256) {
    fail("derived alpha55 output identity changed before publication");
  }

  const outputGuard = {};
  const prepared = await prepareOutputPath(outputPath, { guard: outputGuard });
  await readExactIdentity(sourcePath, "alpha54 source publication fence", SOURCE_BYTE_SIZE, SOURCE_SHA256);
  await readExactIdentity(replacementMediaPath, "background media publication fence", REPLACEMENT_MEDIA_BYTE_SIZE, REPLACEMENT_MEDIA_SHA256);
  let published = false;
  try {
    await writeExclusiveBytes(prepared, derived.bytes, outputGuard, {
      fences: [
        { path: sourcePath, byteSize: SOURCE_BYTE_SIZE, sha256: SOURCE_SHA256 },
        { path: replacementMediaPath, byteSize: REPLACEMENT_MEDIA_BYTE_SIZE, sha256: REPLACEMENT_MEDIA_SHA256 },
      ],
    });
    published = true;
    const written = await readRegularFileBytesNoReparse(prepared, "created alpha55 dual-file project", MAX_BASE_BYTES);
    if (!Buffer.from(written.bytes).equals(derived.bytes)) fail(`created alpha55 output failed byte-for-byte verification; quarantine ${prepared}`);
    await readExactIdentity(sourcePath, "alpha54 source post-publication fence", SOURCE_BYTE_SIZE, SOURCE_SHA256);
    await readExactIdentity(replacementMediaPath, "background media post-publication fence", REPLACEMENT_MEDIA_BYTE_SIZE, REPLACEMENT_MEDIA_SHA256);
    const finalWritten = await readRegularFileBytesNoReparse(prepared, "final alpha55 dual-file project", MAX_BASE_BYTES);
    if (!Buffer.from(finalWritten.bytes).equals(derived.bytes)) fail(`created alpha55 output changed after post-publication fences; quarantine ${prepared}`);
    const reparsed = parseJsonText(decodeUtf8Strict(finalWritten.bytes, "created alpha55 dual-file project"), "created alpha55 dual-file project");
    if (!equalJson(reparsed, derived.output)) fail("created alpha55 project differs from the exact derivation");
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

export async function verifyDualFileDisplayProject() {
  const source = await readExactIdentity(SOURCE_PATH, "protected alpha54 source", SOURCE_BYTE_SIZE, SOURCE_SHA256);
  await readExactIdentity(REPLACEMENT_MEDIA_PATH, "background replacement media", REPLACEMENT_MEDIA_BYTE_SIZE, REPLACEMENT_MEDIA_SHA256);
  const output = await readExactIdentity(OUTPUT_PATH, "alpha55 dual-file output", OUTPUT_BYTE_SIZE, OUTPUT_SHA256);
  const sourceProject = parseJsonText(decodeUtf8Strict(source.bytes, "protected alpha54 source"), "protected alpha54 source");
  const outputProject = parseJsonText(decodeUtf8Strict(output.bytes, "alpha55 dual-file output"), "alpha55 dual-file output");
  const derived = outputBytesFor(sourceProject);
  if (!equalJson(outputProject, derived.output)) fail("alpha55 output differs from the exact derivation");
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
  fail("usage: node qa/harnesses/derive-dsf2026-alpha55-dual-file-display.mjs [--verify-only]");
}

async function main() {
  const options = parseCliArgs(process.argv.slice(2));
  console.log(JSON.stringify(options.verifyOnly ? await verifyDualFileDisplayProject() : await createDualFileDisplayProject(), null, 2));
}

const invokedDirectly = process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href;
if (invokedDirectly) {
  main().catch((error) => {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  });
}
