import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

import {
  ALLOWED_CHANGED_PATHS,
  OUTPUT_BYTE_SIZE,
  OUTPUT_PATH,
  OUTPUT_SHA256,
  REPLACEMENT_MEDIA_BYTE_SIZE,
  REPLACEMENT_MEDIA_METADATA,
  REPLACEMENT_MEDIA_PATH,
  REPLACEMENT_MEDIA_SHA256,
  SOURCE_BYTE_SIZE,
  SOURCE_PATH,
  SOURCE_SHA256,
  createDualFileDisplayProject,
  deriveDualFileDisplayProject,
} from "./derive-dsf2026-alpha55-dual-file-display.mjs";

function sha256(bytes) {
  return createHash("sha256").update(bytes).digest("hex").toUpperCase();
}

function readProject(path) {
  return JSON.parse(readFileSync(path, "utf8"));
}

const sourceBytes = readFileSync(SOURCE_PATH);
assert.equal(sourceBytes.byteLength, SOURCE_BYTE_SIZE);
assert.equal(sha256(sourceBytes), SOURCE_SHA256);
const replacementBytes = readFileSync(REPLACEMENT_MEDIA_PATH);
assert.equal(replacementBytes.byteLength, REPLACEMENT_MEDIA_BYTE_SIZE);
assert.equal(sha256(replacementBytes), REPLACEMENT_MEDIA_SHA256);

const sourceProject = readProject(SOURCE_PATH);
const derived = deriveDualFileDisplayProject(sourceProject);
assert.deepEqual(derived.changedPaths, [...ALLOWED_CHANGED_PATHS].sort());
assert.deepEqual(derived.output.snapshot.video.outputs, sourceProject.snapshot.video.outputs);
assert.deepEqual(derived.output.snapshot.video.compositions, sourceProject.snapshot.video.compositions);
assert.deepEqual(derived.output.snapshot.video.media_assets.slice(0, 3), sourceProject.snapshot.video.media_assets.slice(0, 3));
assert.deepEqual(derived.output.snapshot.video.media_assets[4], sourceProject.snapshot.video.media_assets[4]);
assert.deepEqual(derived.output.snapshot.video.layers[0], sourceProject.snapshot.video.layers[0]);
assert.deepEqual(derived.output.snapshot.video.layers[2], sourceProject.snapshot.video.layers[2]);

const asset = derived.output.snapshot.video.media_assets[3];
const layer = derived.output.snapshot.video.layers[1];
const slot = layer.clip_slots[0];
assert.equal(asset.id, 4);
assert.equal(layer.id, 2);
assert.equal(layer.media_asset_id, 4);
assert.equal(asset.byte_size, REPLACEMENT_MEDIA_BYTE_SIZE);
assert.equal(asset.content_hash.hex, REPLACEMENT_MEDIA_SHA256.toLowerCase());
assert.deepEqual(asset.source.metadata, REPLACEMENT_MEDIA_METADATA);
assert.deepEqual(layer.source.metadata, REPLACEMENT_MEDIA_METADATA);
assert.equal(asset.source.path, REPLACEMENT_MEDIA_PATH);
assert.equal(layer.source.path, REPLACEMENT_MEDIA_PATH);
assert.equal(layer.state.enabled, true);
assert.equal(layer.state.opacity, 1);
assert.equal(layer.state.playing, true);
assert.equal(layer.state.loop_enabled, true);
assert.equal(layer.state.loop_end_ms, 8_000);
assert.equal(slot.loop_mode, "Loop");
assert.equal(slot.out_point_ms, 8_000);

const sourceIdentityMutation = structuredClone(sourceProject);
sourceIdentityMutation.snapshot.video.layers[1].source.kind = "File";
assert.throws(
  () => deriveDualFileDisplayProject(sourceIdentityMutation),
  /camera-backed/,
  "an already-mutated source route must fail closed",
);

const topologyMutation = structuredClone(sourceProject);
topologyMutation.snapshot.video.outputs[1].composition_id = 2;
assert.throws(
  () => deriveDualFileDisplayProject(topologyMutation),
  /Display output topology changed/,
  "output routing mutations must fail closed",
);

const harnessText = readFileSync(
  fileURLToPath(new URL("./derive-dsf2026-alpha55-dual-file-display.mjs", import.meta.url)),
  "utf8",
);
for (const required of [
  "writeExclusiveBytes",
  "fences:",
  "published = true",
  "finalWritten",
  "in-doubt",
  "manually quarantine/remove",
  "--verify-only",
]) {
  assert.ok(harnessText.includes(required), `harness must retain ${required} safety boundary`);
}

const harnessPath = fileURLToPath(
  new URL("./derive-dsf2026-alpha55-dual-file-display.mjs", import.meta.url),
);
const create = spawnSync(process.execPath, [harnessPath], {
  cwd: fileURLToPath(new URL("../../", import.meta.url)),
  encoding: "utf8",
});
if (create.status !== 0 && !/output already exists; refusing to overwrite/.test(create.stderr)) {
  assert.equal(create.status, 0, create.stderr || create.stdout);
}

const verify = spawnSync(process.execPath, [harnessPath, "--verify-only"], {
  cwd: fileURLToPath(new URL("../../", import.meta.url)),
  encoding: "utf8",
});
assert.equal(verify.status, 0, verify.stderr || verify.stdout);
const verifyResult = JSON.parse(verify.stdout);
assert.equal(verifyResult.status, "PASS");
assert.equal(verifyResult.outputPath, OUTPUT_PATH);
assert.equal(verifyResult.outputByteSize, OUTPUT_BYTE_SIZE);
assert.equal(verifyResult.outputSha256, OUTPUT_SHA256);
assert.deepEqual(verifyResult.changedPaths, [...ALLOWED_CHANGED_PATHS].sort());

await assert.rejects(
  () => createDualFileDisplayProject({ sourcePath: OUTPUT_PATH }),
  /alpha54 source path is fixed/,
  "an unexpected source path must be rejected before any read or write",
);
await assert.rejects(
  () => createDualFileDisplayProject({ replacementMediaPath: SOURCE_PATH }),
  /background replacement media path is fixed/,
  "an unexpected replacement path must be rejected before any read or write",
);
await assert.rejects(
  () => createDualFileDisplayProject({ outputPath: SOURCE_PATH }),
  /alpha55 output path is fixed/,
  "an unexpected output path must be rejected before any read or write",
);
await assert.rejects(
  () => createDualFileDisplayProject(),
  /output already exists; refusing to overwrite/,
  "a second create must reject the existing output without overwrite",
);

console.log("alpha55 dual-file Display derivation checks passed");
