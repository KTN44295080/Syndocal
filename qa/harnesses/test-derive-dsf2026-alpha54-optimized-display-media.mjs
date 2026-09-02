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
  deriveOptimizedDisplayMediaProject,
  createOptimizedDisplayMediaProject,
} from "./derive-dsf2026-alpha54-optimized-display-media.mjs";

function readProject(path) {
  return JSON.parse(readFileSync(path, "utf8"));
}

function sha256(bytes) {
  return createHash("sha256").update(bytes).digest("hex").toUpperCase();
}

const sourceBytes = readFileSync(SOURCE_PATH);
assert.equal(sourceBytes.byteLength, SOURCE_BYTE_SIZE);
assert.equal(sha256(sourceBytes), SOURCE_SHA256);
const replacementBytes = readFileSync(REPLACEMENT_MEDIA_PATH);
assert.equal(replacementBytes.byteLength, REPLACEMENT_MEDIA_BYTE_SIZE);
assert.equal(sha256(replacementBytes), REPLACEMENT_MEDIA_SHA256);

const sourceProject = readProject(SOURCE_PATH);
const derived = deriveOptimizedDisplayMediaProject(sourceProject);
assert.deepEqual(derived.changedPaths, [...ALLOWED_CHANGED_PATHS].sort());
assert.deepEqual(derived.output.snapshot.video.outputs, sourceProject.snapshot.video.outputs);
assert.deepEqual(derived.output.snapshot.video.compositions, sourceProject.snapshot.video.compositions);
assert.deepEqual(derived.output.snapshot.video.media_assets.slice(0, 4), sourceProject.snapshot.video.media_assets.slice(0, 4));
assert.deepEqual(derived.output.snapshot.video.layers.slice(0, 2), sourceProject.snapshot.video.layers.slice(0, 2));

const asset = derived.output.snapshot.video.media_assets[4];
const layer = derived.output.snapshot.video.layers[2];
const slot = layer.clip_slots[0];
assert.equal(asset.id, 5);
assert.equal(layer.id, 3);
assert.equal(layer.media_asset_id, 5);
assert.equal(asset.byte_size, REPLACEMENT_MEDIA_BYTE_SIZE);
assert.equal(asset.content_hash.hex, REPLACEMENT_MEDIA_SHA256.toLowerCase());
assert.deepEqual(asset.source.metadata, REPLACEMENT_MEDIA_METADATA);
assert.deepEqual(layer.source.metadata, REPLACEMENT_MEDIA_METADATA);
assert.equal(asset.source.path, REPLACEMENT_MEDIA_PATH);
assert.equal(layer.source.path, REPLACEMENT_MEDIA_PATH);
assert.equal(layer.state.loop_end_ms, 3_000);
assert.equal(slot.out_point_ms, 3_000);
assert.equal(layer.state.loop_enabled, true);
assert.equal(slot.loop_mode, "Loop");

const preservedNonTargetSourceChange = structuredClone(sourceProject);
preservedNonTargetSourceChange.snapshot.video.layers[0].label = "operator-authored label";
const preservedNonTargetDerived = deriveOptimizedDisplayMediaProject(preservedNonTargetSourceChange);
assert.equal(
  preservedNonTargetDerived.output.snapshot.video.layers[0].label,
  "operator-authored label",
  "non-target source fields must be preserved rather than rewritten",
);
assert.deepEqual(
  preservedNonTargetDerived.changedPaths,
  [...ALLOWED_CHANGED_PATHS].sort(),
  "non-target source fields must not become derived changes",
);

const sourceIdentityMutation = structuredClone(sourceProject);
sourceIdentityMutation.snapshot.video.media_assets[4].content_hash.hex = "0".repeat(64);
assert.throws(
  () => deriveOptimizedDisplayMediaProject(sourceIdentityMutation),
  /asset identity changed/,
  "ambiguous source media identity must fail closed",
);

const topologyMutation = structuredClone(sourceProject);
topologyMutation.snapshot.video.outputs[0].composition_id = 3;
assert.throws(
  () => deriveOptimizedDisplayMediaProject(topologyMutation),
  /Display output topology changed/,
  "output routing mutations must fail closed",
);

const loopMutation = structuredClone(sourceProject);
loopMutation.snapshot.video.layers[2].state.loop_end_ms = 2_999;
assert.throws(
  () => deriveOptimizedDisplayMediaProject(loopMutation),
  /loop state changed/,
  "source loop-end mutations must fail closed",
);

const harnessText = readFileSync(
  fileURLToPath(new URL("./derive-dsf2026-alpha54-optimized-display-media.mjs", import.meta.url)),
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

const verify = spawnSync(
  process.execPath,
  [fileURLToPath(new URL("./derive-dsf2026-alpha54-optimized-display-media.mjs", import.meta.url)), "--verify-only"],
  { cwd: fileURLToPath(new URL("../../", import.meta.url)), encoding: "utf8" },
);
assert.equal(verify.status, 0, verify.stderr || verify.stdout);
const verifyResult = JSON.parse(verify.stdout);
assert.equal(verifyResult.status, "PASS");
assert.equal(verifyResult.outputPath, OUTPUT_PATH);
assert.equal(verifyResult.outputByteSize, OUTPUT_BYTE_SIZE);
assert.equal(verifyResult.outputSha256, OUTPUT_SHA256);
assert.deepEqual(verifyResult.changedPaths, [...ALLOWED_CHANGED_PATHS].sort());

await assert.rejects(
  () => createOptimizedDisplayMediaProject({ sourcePath: OUTPUT_PATH }),
  /alpha53 source path is fixed/,
  "an unexpected source path must be rejected before any read or write",
);
await assert.rejects(
  () => createOptimizedDisplayMediaProject({ replacementMediaPath: SOURCE_PATH }),
  /optimized replacement media path is fixed/,
  "an unexpected replacement path must be rejected before any read or write",
);
await assert.rejects(
  () => createOptimizedDisplayMediaProject({ outputPath: SOURCE_PATH }),
  /alpha54 output path is fixed/,
  "an unexpected output path must be rejected before any read or write",
);

await assert.rejects(
  () => createOptimizedDisplayMediaProject(),
  /output already exists; refusing to overwrite/,
  "a second create must reject the existing output without overwrite",
);

console.log("alpha54 optimized display media derivation checks passed");
