import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdtemp, readFile, rm, stat, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { basename, join } from "node:path";

import {
  SAME_PC_ALLOWED_CHANGED_PATHS,
  SAME_PC_OUTPUT_PATH,
  SAME_PC_SOURCE_BYTE_SIZE,
  SAME_PC_SOURCE_FILENAME,
  SAME_PC_SOURCE_PATH,
  SAME_PC_SOURCE_SHA256,
  assertApprovedSamePcSource,
  authorSamePcOutputProject,
  createSamePcOutputCopy,
  getChangedPaths,
  validateSamePcOutputProject,
} from "../../tools/create-dsf2026-same-pc-output-copy.mjs";
import { equalJson } from "../../tools/dsf2026/common.mjs";
import { normalizeIdentityPath } from "../../tools/dsf2026/io.mjs";

const sourceBytes = await readFile(SAME_PC_SOURCE_PATH);
const sourceProject = JSON.parse(sourceBytes);
const sourceHash = createHash("sha256").update(sourceBytes).digest("hex");
assert.equal(basename(SAME_PC_SOURCE_PATH), SAME_PC_SOURCE_FILENAME);
assert.equal(sourceBytes.byteLength, SAME_PC_SOURCE_BYTE_SIZE);
assert.equal(sourceHash, SAME_PC_SOURCE_SHA256);
assert.equal(SAME_PC_OUTPUT_PATH.endsWith("DSF2026-show-alpha10-same-pc-output.sdc"), true);

function assertRejected(action, pattern) {
  return assert.rejects(action, (error) => {
    assert.match(String(error?.message ?? error), pattern);
    return true;
  });
}

function assertThrows(action, pattern) {
  assert.throws(action, (error) => {
    assert.match(String(error?.message ?? error), pattern);
    return true;
  });
}

function makeOutput() {
  return authorSamePcOutputProject(sourceProject, {
    sourcePath: SAME_PC_SOURCE_PATH,
    sourceBytes,
  });
}

// The approved source identity, strict schema, and exact allowlist are fixed.
assert.deepEqual(assertApprovedSamePcSource(SAME_PC_SOURCE_PATH, sourceBytes, sourceProject), {
  byteSize: SAME_PC_SOURCE_BYTE_SIZE,
  sha256: SAME_PC_SOURCE_SHA256,
});

// Authoring changes only the four synchronized output paths.
{
  const output = makeOutput();
  const report = validateSamePcOutputProject(output, sourceProject);
  assert.equal(report.status, "PASS");
  assert.deepEqual(report.changedPaths, [...SAME_PC_ALLOWED_CHANGED_PATHS].sort());
  assert.deepEqual(getChangedPaths(sourceProject, output), [...SAME_PC_ALLOWED_CHANGED_PATHS].sort());
  assert.equal(output.snapshot.output.protocol, "ArtNet");
  assert.equal(output.snapshot.dmx_outputs[0].protocol, "ArtNet");
  assert.deepEqual(output.snapshot.video.compositions, [{ id: 1, label: "Main", layer_ids: [], output_ids: [1, 2] }]);
  assert.equal(output.snapshot.video.outputs.length, 2);
  assert.deepEqual(output.snapshot.video.outputs.map(({ id, label, kind, enabled, composition_id, width, height, endpoint_name, opacity, blackout }) => ({ id, label, kind, enabled, composition_id, width, height, endpoint_name, opacity, blackout })), [
    { id: 1, label: "Syndocal Background", kind: "SpoutSender", enabled: true, composition_id: 1, width: 1920, height: 1080, endpoint_name: "Syndocal Background", opacity: 1, blackout: false },
    { id: 2, label: "Syndocal Foreground", kind: "SpoutSender", enabled: true, composition_id: 1, width: 1920, height: 1080, endpoint_name: "Syndocal Foreground", opacity: 1, blackout: false },
  ]);
  for (const outputEntry of output.snapshot.video.outputs) {
    assert.equal(outputEntry.mapping.aspect_mode, "Stretch");
    assert.equal(outputEntry.mapping.mask_points.length, 8);
    assert.equal(outputEntry.mapping.bitmap_mask_luma_words.length, 32);
  }
  assert.equal(output.snapshot.fixtures.length, 46);
  assert.deepEqual(output.snapshot.dmx_preview, sourceProject.snapshot.dmx_preview);
  assert.deepEqual(output.snapshot.dmx_previews, sourceProject.snapshot.dmx_previews);
}

// The author never mutates the source object or source bytes.
{
  const beforeProject = structuredClone(sourceProject);
  const beforeBytes = Buffer.from(sourceBytes);
  makeOutput();
  assert.ok(equalJson(sourceProject, beforeProject));
  assert.deepEqual(sourceBytes, beforeBytes);
}

// Source identity is path-, byte-size-, hash-, and schema-bound.
{
  assertThrows(() => authorSamePcOutputProject(sourceProject, { sourcePath: "C:\\elsewhere\\alpha9.sdc", sourceBytes }), /exact approved alpha9 path/u);
  const alteredBytes = Buffer.from(sourceBytes);
  alteredBytes[alteredBytes.length - 2] ^= 1;
  assertThrows(() => authorSamePcOutputProject(sourceProject, { sourcePath: SAME_PC_SOURCE_PATH, sourceBytes: alteredBytes }), /SHA-256|byte length/u);
  const alteredProject = structuredClone(sourceProject);
  alteredProject.snapshot.fixtures[0].label = "future mutation";
  assertThrows(() => authorSamePcOutputProject(alteredProject, { sourcePath: SAME_PC_SOURCE_PATH, sourceBytes }), /differs from the approved source bytes/u);
  const malformedProject = structuredClone(sourceProject);
  malformedProject.snapshot.video.future_field = true;
  assertThrows(() => authorSamePcOutputProject(malformedProject, { sourcePath: SAME_PC_SOURCE_PATH, sourceBytes }), /differs from the approved source bytes/u);
}

// Result validation fails closed for route, composition, output, mapping, and future-field drift.
{
  const cases = [
    ["DMX target", (project) => { project.snapshot.output.target_ip = "192.168.1.2"; }, /output DMX routes|target_ip/u],
    ["DMX enabled", (project) => { project.snapshot.dmx_outputs[0].enabled = true; }, /output DMX routes|enabled/u],
    ["composition reference", (project) => { project.snapshot.video.compositions[0].output_ids = [1]; }, /output Main composition/u],
    ["output dimensions", (project) => { project.snapshot.video.outputs[0].width = 1280; }, /exact Syndocal Background/u],
    ["output duplicate", (project) => { project.snapshot.video.outputs[1].id = 1; }, /exact Syndocal Foreground/u],
    ["output kind", (project) => { project.snapshot.video.outputs[0].kind = "Display"; }, /exact Syndocal Background/u],
    ["mapping future field", (project) => { project.snapshot.video.outputs[0].mapping.future = 1; }, /mapping shape differs|exact Syndocal Background/u],
    ["unexpected source path", (project) => { project.snapshot.timeline_bank[0].label = "changed"; }, /non-allowlisted/u],
  ];
  for (const [label, mutate, pattern] of cases) {
    const output = makeOutput();
    mutate(output);
    assertThrows(() => validateSamePcOutputProject(output, sourceProject), pattern, label);
  }
}

// The real writer creates only a temporary sibling, preserves source and media sidecars,
// and reparses/validates the written JSON. The production alpha10 path is never touched.
{
  const directory = await mkdtemp(join(tmpdir(), "syndocal-same-pc-output-"));
  const outputPath = join(directory, "same-pc-output.sdc");
  const sidecarPaths = sourceProject.snapshot.video.media_assets.map((asset) => asset.source.path);
  const beforeSidecars = await Promise.all(sidecarPaths.map((path) => readFile(path)));
  try {
    const result = await createSamePcOutputCopy({ sourcePath: SAME_PC_SOURCE_PATH, outputPath });
    assert.equal(normalizeIdentityPath(result.outputPath), normalizeIdentityPath(outputPath));
    assert.deepEqual(result.changedPaths, SAME_PC_ALLOWED_CHANGED_PATHS);
    const written = JSON.parse(await readFile(outputPath, "utf8"));
    assert.equal(validateSamePcOutputProject(written, sourceProject).status, "PASS");
    assert.deepEqual(await readFile(SAME_PC_SOURCE_PATH), sourceBytes);
    assert.deepEqual(await Promise.all(sidecarPaths.map((path) => readFile(path))), beforeSidecars);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
}

// Existing and divergent targets are never overwritten.
{
  const directory = await mkdtemp(join(tmpdir(), "syndocal-same-pc-output-existing-"));
  const outputPath = join(directory, "existing.sdc");
  const sentinel = Buffer.from("operator-owned output\n", "utf8");
  await writeFile(outputPath, sentinel, { flag: "wx" });
  try {
    await assertRejected(() => createSamePcOutputCopy({ sourcePath: SAME_PC_SOURCE_PATH, outputPath }), /already exists|overwrite/u);
    assert.deepEqual(await readFile(outputPath), sentinel);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
}

// A post-write parse failure retains the run-created target for explicit quarantine/manual removal.
{
  const directory = await mkdtemp(join(tmpdir(), "syndocal-same-pc-output-parse-failure-"));
  const outputPath = join(directory, "parse-failure.sdc");
  const beforeSource = await readFile(SAME_PC_SOURCE_PATH);
  const beforeSidecars = await Promise.all(sourceProject.snapshot.video.media_assets.map((asset) => readFile(asset.source.path)));
  const expectedBytes = Buffer.from(`${JSON.stringify(makeOutput(), null, 2)}\n`, "utf8");
  try {
    await assertRejected(() => createSamePcOutputCopy({
      sourcePath: SAME_PC_SOURCE_PATH,
      outputPath,
      testHooks: { postWriteText: () => "{ malformed" },
    }), /JSON|parse|invalid|cleanup/u);
    assert.deepEqual(await readFile(outputPath), expectedBytes);
    assert.deepEqual(await readFile(SAME_PC_SOURCE_PATH), beforeSource);
    assert.deepEqual(await Promise.all(sourceProject.snapshot.video.media_assets.map((asset) => readFile(asset.source.path))), beforeSidecars);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
}

// A post-write validation failure also retains the intact target for explicit quarantine/manual removal.
{
  const directory = await mkdtemp(join(tmpdir(), "syndocal-same-pc-output-validation-failure-"));
  const outputPath = join(directory, "validation-failure.sdc");
  const expectedBytes = Buffer.from(`${JSON.stringify(makeOutput(), null, 2)}\n`, "utf8");
  try {
    await assertRejected(() => createSamePcOutputCopy({
      sourcePath: SAME_PC_SOURCE_PATH,
      outputPath,
      testHooks: {
        postWriteText: ({ text }) => {
          const value = JSON.parse(text);
          value.snapshot.output.target_ip = "192.0.2.1";
          return JSON.stringify(value);
        },
      },
    }), /output DMX routes|target_ip|cleanup/u);
    assert.deepEqual(await readFile(outputPath), expectedBytes);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
}

// If the target is substituted after publication, identity/bytes checks fail
// closed and the substitute is retained for quarantine/manual inspection.
{
  const directory = await mkdtemp(join(tmpdir(), "syndocal-same-pc-output-substitute-"));
  const outputPath = join(directory, "substituted.sdc");
  const substitute = Buffer.from("operator substitute\n", "utf8");
  try {
    await assertRejected(() => createSamePcOutputCopy({
      sourcePath: SAME_PC_SOURCE_PATH,
      outputPath,
      testHooks: {
        afterWrite: async ({ outputPath: publishedPath }) => {
          await rm(publishedPath);
          await writeFile(publishedPath, substitute, { flag: "wx" });
        },
      },
    }), /identity|substituted|quarantine|manual removal|cleanup/u);
    assert.deepEqual(await readFile(outputPath), substitute);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
}

// A cleanup failure is explicit and leaves the exact target instead of making
// a second unverified deletion attempt.
{
  const directory = await mkdtemp(join(tmpdir(), "syndocal-same-pc-output-cleanup-failure-"));
  const outputPath = join(directory, "cleanup-failure.sdc");
  try {
    await assertRejected(() => createSamePcOutputCopy({
      sourcePath: SAME_PC_SOURCE_PATH,
      outputPath,
      testHooks: { postWriteText: () => "{ malformed", beforeCleanupDelete: () => { throw new Error("injected cleanup failure"); } },
    }), /cleanup failed|quarantine|manual removal|injected cleanup failure/u);
    assert.ok((await stat(outputPath)).isFile());
    assert.deepEqual(await readFile(outputPath), Buffer.from(`${JSON.stringify(makeOutput(), null, 2)}\n`, "utf8"));
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
}

// A replacement in the final cleanup window is detected and retained; the
// path-based cleanup never deletes the substituted content.
{
  const directory = await mkdtemp(join(tmpdir(), "syndocal-same-pc-output-cleanup-substitute-"));
  const outputPath = join(directory, "cleanup-substituted.sdc");
  const substitute = Buffer.from("cleanup-window substitute\n", "utf8");
  try {
    await assertRejected(() => createSamePcOutputCopy({
      sourcePath: SAME_PC_SOURCE_PATH,
      outputPath,
      testHooks: {
        postWriteText: () => "{ malformed",
        beforeCleanupDelete: async ({ outputPath: publishedPath }) => {
          await rm(publishedPath);
          await writeFile(publishedPath, substitute, { flag: "wx" });
        },
      },
    }), /identity|substituted|quarantine|manual removal|automatic cleanup/u);
    assert.deepEqual(await readFile(outputPath), substitute);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
}

// Non-SDC output names are rejected before any target is created.
{
  const directory = await mkdtemp(join(tmpdir(), "syndocal-same-pc-output-extension-"));
  try {
    await assertRejected(() => createSamePcOutputCopy({ sourcePath: SAME_PC_SOURCE_PATH, outputPath: join(directory, "wrong.json") }), /\.sdc extension/u);
    await assert.rejects(stat(join(directory, "wrong.json")), (error) => error?.code === "ENOENT");
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
}

console.log("DSF2026 same-PC output copy: strict alpha9 identity, ArtNet route, exact Spout pair, allowlist, preservation, and exclusive-write tests passed");
