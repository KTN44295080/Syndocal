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

export const SOURCE_PATH = resolve(REPO_ROOT, "DSF2026-show-alpha51-usb-final.sdc");
export const SOURCE_BYTE_SIZE = 1_116_223;
export const SOURCE_SHA256 = "7031196A6527431FB8D625F420FE5D3E442DD2AA35D2ED86D57AF9EDF69890A7";

export const REPLACEMENT_MEDIA_PATH = resolve("C:/Users/kouty/Downloads/logo-anim-dark.mp4");
export const REPLACEMENT_MEDIA_BYTE_SIZE = 630_538;
export const REPLACEMENT_MEDIA_SHA256 = "BB2E52ABF4199716DDB8F9E6180F8343A0AFFA3474F22C363CA05207B19B89B9";
export const REPLACEMENT_MEDIA_METADATA = Object.freeze({
  duration_ms: 3_008,
  frame_rate: 30,
  has_audio: true,
  height: 1080,
  width: 1280,
});

export const OUTPUT_PATH = resolve(
  REPO_ROOT,
  "target/qa/DSF2026-show-alpha52-three-display-current-media.sdc",
);
export const OUTPUT_BYTE_SIZE = 1_112_316;
export const OUTPUT_SHA256 = "27484E18DE3FFBB19829D19A90459AB4D847209BF272E629CAE37AF3DD38EA11";

export const ALLOWED_CHANGED_PATHS = Object.freeze([
  "snapshot.video.layers[2].label",
  "snapshot.video.layers[2].source.metadata.duration_ms",
  "snapshot.video.layers[2].source.metadata.frame_rate",
  "snapshot.video.layers[2].source.metadata.height",
  "snapshot.video.layers[2].source.metadata.width",
  "snapshot.video.layers[2].source.path",
  "snapshot.video.media_assets[4].byte_size",
  "snapshot.video.media_assets[4].content_hash.hex",
  "snapshot.video.media_assets[4].label",
  "snapshot.video.media_assets[4].source.metadata.duration_ms",
  "snapshot.video.media_assets[4].source.metadata.frame_rate",
  "snapshot.video.media_assets[4].source.metadata.height",
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

function assertSourceShape(project) {
  const video = project?.snapshot?.video;
  if (!video || !Array.isArray(video.media_assets) || !Array.isArray(video.layers)) {
    fail("source project must contain the expected video model");
  }
  const asset = video.media_assets[4];
  const layer = video.layers[2];
  if (asset?.id !== 5 || layer?.id !== 3 || layer?.media_asset_id !== 5) {
    fail("source foreground asset/layer identity must remain exactly asset 5 / layer 3");
  }
  if (video.outputs?.length !== 2 || video.compositions?.length !== 3) {
    fail("source project must retain exactly two Display outputs and three compositions");
  }
  const displayContract = video.outputs.map((output) => ({
    id: output.id,
    kind: output.kind,
    composition_id: output.composition_id,
    width: output.width,
    height: output.height,
  }));
  const expected = [
    { id: 3, kind: "Display", composition_id: 2, width: 1920, height: 1080 },
    { id: 4, kind: "Display", composition_id: 3, width: 3840, height: 2160 },
  ];
  if (!equalJson(displayContract, expected)) {
    fail("source Display output contract changed");
  }
}

export function deriveCurrentMediaProject(sourceProject) {
  assertSourceShape(sourceProject);
  const output = structuredClone(sourceProject);
  const asset = output.snapshot.video.media_assets[4];
  const layer = output.snapshot.video.layers[2];
  const label = "logo-anim-dark";

  asset.byte_size = REPLACEMENT_MEDIA_BYTE_SIZE;
  asset.content_hash.hex = REPLACEMENT_MEDIA_SHA256.toLowerCase();
  asset.label = label;
  asset.source.path = REPLACEMENT_MEDIA_PATH;
  Object.assign(asset.source.metadata, REPLACEMENT_MEDIA_METADATA);

  layer.label = label;
  layer.source.path = REPLACEMENT_MEDIA_PATH;
  Object.assign(layer.source.metadata, REPLACEMENT_MEDIA_METADATA);

  const actualChangedPaths = changedPaths(sourceProject, output).sort();
  const expectedChangedPaths = [...ALLOWED_CHANGED_PATHS].sort();
  if (!equalJson(actualChangedPaths, expectedChangedPaths)) {
    fail(`derived project changed unexpected paths: ${actualChangedPaths.join(", ")}`);
  }
  assertSourceShape(output);
  return { output, changedPaths: actualChangedPaths };
}

async function readExactIdentity(path, label, byteSize, expectedSha256) {
  const file = await readRegularFileBytesNoReparse(path, label, MAX_BASE_BYTES);
  if (file.bytes.byteLength !== byteSize) {
    fail(`${label} byte length must be exactly ${byteSize}; found ${file.bytes.byteLength}`);
  }
  const actualSha256 = sha256(file.bytes);
  if (actualSha256 !== expectedSha256) {
    fail(`${label} SHA-256 ${actualSha256} does not match the approved identity`);
  }
  return { ...file, sha256: actualSha256 };
}

export async function createCurrentMediaProject({
  sourcePath = SOURCE_PATH,
  replacementMediaPath = REPLACEMENT_MEDIA_PATH,
  outputPath = OUTPUT_PATH,
} = {}) {
  if (normalizeIdentityPath(sourcePath) !== normalizeIdentityPath(SOURCE_PATH)) {
    fail(`source path is fixed to ${SOURCE_PATH}`);
  }
  if (normalizeIdentityPath(replacementMediaPath) !== normalizeIdentityPath(REPLACEMENT_MEDIA_PATH)) {
    fail(`replacement media path is fixed to ${REPLACEMENT_MEDIA_PATH}`);
  }
  if (normalizeIdentityPath(outputPath) !== normalizeIdentityPath(OUTPUT_PATH)) {
    fail(`output path is fixed to ${OUTPUT_PATH}`);
  }
  if (extname(outputPath).toLowerCase() !== ".sdc" || normalizeIdentityPath(dirname(outputPath)) === normalizeIdentityPath(REPO_ROOT)) {
    fail("output must be the fixed target/qa .sdc child");
  }

  const source = await readExactIdentity(sourcePath, "protected alpha51 source", SOURCE_BYTE_SIZE, SOURCE_SHA256);
  await readExactIdentity(
    replacementMediaPath,
    "replacement foreground media",
    REPLACEMENT_MEDIA_BYTE_SIZE,
    REPLACEMENT_MEDIA_SHA256,
  );
  const sourceProject = parseJsonText(decodeUtf8Strict(source.bytes, "protected alpha51 source"), "protected alpha51 source");
  const derived = deriveCurrentMediaProject(sourceProject);
  const outputBytes = Buffer.from(`${JSON.stringify(derived.output, null, 2)}\n`, "utf8");
  if (outputBytes.byteLength !== OUTPUT_BYTE_SIZE || sha256(outputBytes) !== OUTPUT_SHA256) {
    fail("derived alpha52 three-display output identity changed before publication");
  }

  const outputGuard = {};
  const prepared = await prepareOutputPath(outputPath, { guard: outputGuard });
  await readExactIdentity(sourcePath, "protected alpha51 publication fence", SOURCE_BYTE_SIZE, SOURCE_SHA256);
  await readExactIdentity(
    replacementMediaPath,
    "replacement foreground media publication fence",
    REPLACEMENT_MEDIA_BYTE_SIZE,
    REPLACEMENT_MEDIA_SHA256,
  );
  let published = false;
  try {
    await writeExclusiveBytes(prepared, outputBytes, outputGuard);
    published = true;
    // Establish the exact created bytes before the post-publication source
    // fences. If any later check fails, the path is an in-doubt local QA
    // artifact. Node has no handle-bound unlink/rename primitive, so automatic
    // cleanup would introduce a target-substitution deletion race.
    const written = await readRegularFileBytesNoReparse(
      prepared,
      "created alpha52 three-display project",
      MAX_BASE_BYTES,
    );
    if (!Buffer.from(written.bytes).equals(outputBytes)) {
      fail(`created output failed byte-for-byte verification; quarantine ${prepared}`);
    }
    await readExactIdentity(sourcePath, "protected alpha51 post-publication fence", SOURCE_BYTE_SIZE, SOURCE_SHA256);
    await readExactIdentity(
      replacementMediaPath,
      "replacement foreground media post-publication fence",
      REPLACEMENT_MEDIA_BYTE_SIZE,
      REPLACEMENT_MEDIA_SHA256,
    );
    const finalWritten = await readRegularFileBytesNoReparse(
      prepared,
      "final alpha52 three-display project",
      MAX_BASE_BYTES,
    );
    if (!Buffer.from(finalWritten.bytes).equals(outputBytes)) {
      fail(`created output changed after post-publication fences; quarantine ${prepared}`);
    }
    const reparsed = parseJsonText(
      decodeUtf8Strict(finalWritten.bytes, "created alpha52 project"),
      "created alpha52 project",
    );
    const verified = deriveCurrentMediaProject(sourceProject);
    if (!equalJson(reparsed, verified.output)) fail("created alpha52 project differs from the derived model");
  } catch (error) {
    if (published) {
      throw new Error(
        `${String(error?.message ?? error)}; created local QA output is in-doubt and must not be accepted or regenerated in place; manually quarantine/remove ${prepared} only after verifying its exact path and identity`,
        { cause: error },
      );
    }
    throw error;
  }
  return {
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
    routesEnabled: true,
    physicalOutputOpened: false,
  };
}

export async function verifyCurrentMediaProject() {
  const source = await readExactIdentity(SOURCE_PATH, "protected alpha51 source", SOURCE_BYTE_SIZE, SOURCE_SHA256);
  await readExactIdentity(
    REPLACEMENT_MEDIA_PATH,
    "replacement foreground media",
    REPLACEMENT_MEDIA_BYTE_SIZE,
    REPLACEMENT_MEDIA_SHA256,
  );
  const output = await readExactIdentity(OUTPUT_PATH, "alpha52 three-display output", OUTPUT_BYTE_SIZE, OUTPUT_SHA256);
  const sourceProject = parseJsonText(decodeUtf8Strict(source.bytes, "protected alpha51 source"), "protected alpha51 source");
  const outputProject = parseJsonText(decodeUtf8Strict(output.bytes, "alpha52 three-display output"), "alpha52 three-display output");
  const derived = deriveCurrentMediaProject(sourceProject);
  if (!equalJson(outputProject, derived.output)) fail("alpha52 three-display output differs from the exact derivation");
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
  fail("usage: node qa/harnesses/derive-dsf2026-alpha52-three-display-current-media.mjs [--verify-only]");
}

async function main() {
  const options = parseCliArgs(process.argv.slice(2));
  console.log(JSON.stringify(options.verifyOnly ? await verifyCurrentMediaProject() : await createCurrentMediaProject(), null, 2));
}

const invokedDirectly = process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href;
if (invokedDirectly) {
  main().catch((error) => {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  });
}
