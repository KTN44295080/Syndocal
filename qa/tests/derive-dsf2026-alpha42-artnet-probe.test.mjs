import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import {
  lstat,
  mkdir,
  mkdtemp,
  readFile,
  rm,
  stat,
  symlink,
  writeFile,
} from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";

import {
  ALLOWED_CHANGED_PATHS,
  OUTPUT_BYTE_SIZE,
  OUTPUT_FILENAME,
  OUTPUT_PATH,
  OUTPUT_SHA256,
  SOURCE_BYTE_SIZE,
  SOURCE_FILENAME,
  SOURCE_PATH,
  SOURCE_SHA256,
  createAlpha42ArtNetProbe,
  deriveAlpha42ArtNetProbeBytes,
  getChangedPaths,
  parseCliArgs,
  validateAlpha42ArtNetProbeProject,
  validateAlpha42SourceProject,
} from "../harnesses/derive-dsf2026-alpha42-artnet-probe.mjs";

const sourceBytes = await readFile(SOURCE_PATH);
const sourceProject = JSON.parse(sourceBytes);
const sourceText = sourceBytes.toString("utf8");

function sha256(bytes) {
  return createHash("sha256").update(bytes).digest("hex").toUpperCase();
}

function assertThrows(action, pattern, label) {
  assert.throws(action, (error) => {
    assert.match(String(error?.message ?? error), pattern, `${label} must explain the rejection`);
    return true;
  }, label);
}

async function assertRejected(action, pattern, label) {
  await assert.rejects(action, (error) => {
    assert.match(String(error?.message ?? error), pattern, `${label} must explain the rejection`);
    return true;
  }, label);
}

async function assertAbsent(path, label) {
  await assert.rejects(stat(path), (error) => error?.code === "ENOENT", label);
}

async function makeFixture(bytes = sourceBytes) {
  const directory = await mkdtemp(join(tmpdir(), "syndocal-alpha42-artnet-probe-"));
  const sourcePath = join(directory, SOURCE_FILENAME);
  const outputPath = join(directory, OUTPUT_FILENAME);
  await writeFile(sourcePath, bytes, { flag: "wx" });
  return { directory, sourcePath, outputPath };
}

async function cleanup(fixture) {
  await rm(fixture.directory, { recursive: true, force: true });
}

assert.equal(sourceBytes.byteLength, SOURCE_BYTE_SIZE);
assert.equal(sha256(sourceBytes), SOURCE_SHA256);
assert.equal(dirname(SOURCE_PATH), dirname(OUTPUT_PATH));
assert.equal(OUTPUT_PATH.endsWith(OUTPUT_FILENAME), true);

// The pure authoring path performs only the two literal protocol replacements
// and proves the complete parsed-project diff is exactly the two route fields.
{
  const derived = deriveAlpha42ArtNetProbeBytes(sourceBytes);
  assert.equal(derived.outputByteSize, OUTPUT_BYTE_SIZE);
  assert.equal(derived.outputSha256, OUTPUT_SHA256);
  assert.equal(sha256(derived.outputBytes), OUTPUT_SHA256);
  assert.equal(derived.literalReplacementCount, 2);
  assert.deepEqual(derived.changedPaths, [...ALLOWED_CHANGED_PATHS].sort());
  assert.equal((sourceText.match(/"protocol": "EnttecOpenDmx"/g) ?? []).length, 2);
  assert.equal((derived.outputText.match(/"protocol": "ArtNet"/g) ?? []).length, 2);
  assert.equal(derived.outputProject.snapshot.output.protocol, "ArtNet");
  assert.equal(derived.outputProject.snapshot.dmx_outputs[0].protocol, "ArtNet");
  assert.equal(derived.outputProject.snapshot.output.enabled, false);
  assert.equal(derived.outputProject.snapshot.dmx_outputs[0].enabled, false);
  assert.equal(derived.outputProject.snapshot.output.target_ip, "127.0.0.1");
  assert.equal(derived.outputProject.snapshot.output.port, 6454);
  assert.equal(derived.outputProject.snapshot.output.universe, 0);
  assert.equal(derived.outputProject.snapshot.output.serial_port, "");
  assert.equal(derived.outputProject.snapshot.output.serial_baud_rate, 250000);
  assert.deepEqual(getChangedPaths(derived.sourceProject, derived.outputProject), [...ALLOWED_CHANGED_PATHS].sort());
}

// The source schema and counts are explicit, including the strict three
// timeline/four-output/46-fixture topology used by the alpha42 acceptance file.
{
  assert.deepEqual(validateAlpha42SourceProject(sourceProject), { status: "PASS" });
  const extraField = structuredClone(sourceProject);
  extraField.snapshot.video.unreviewed = true;
  assertThrows(() => validateAlpha42SourceProject(extraField), /shape differs|expected keys/u, "schema drift");

  const wrongFixtureCount = structuredClone(sourceProject);
  wrongFixtureCount.snapshot.fixtures.pop();
  assertThrows(() => validateAlpha42SourceProject(wrongFixtureCount), /snapshot\.fixtures count|exactly 46/u, "fixture count drift");

  const wrongRouteCount = structuredClone(sourceProject);
  wrongRouteCount.snapshot.dmx_outputs.push(structuredClone(wrongRouteCount.snapshot.dmx_outputs[0]));
  assertThrows(() => validateAlpha42SourceProject(wrongRouteCount), /dmx_outputs.*exactly 1|count/u, "DMX route count drift");

  const wrongTimelineCount = structuredClone(sourceProject);
  wrongTimelineCount.snapshot.timeline_bank.pop();
  assertThrows(() => validateAlpha42SourceProject(wrongTimelineCount), /timeline_bank count|exactly 3/u, "timeline count drift");
}

// Publication succeeds in a deterministic temp fixture while preserving the
// approved source bytes and using an exclusive, non-reparse output write.
{
  const fixture = await makeFixture();
  try {
    const before = await readFile(fixture.sourcePath);
    const result = await createAlpha42ArtNetProbe({
      sourcePath: fixture.sourcePath,
      outputPath: fixture.outputPath,
      allowTestPaths: true,
    });
    assert.equal(result.sourceByteSize, SOURCE_BYTE_SIZE);
    assert.equal(result.sourceSha256, SOURCE_SHA256);
    assert.equal(result.outputByteSize, OUTPUT_BYTE_SIZE);
    assert.equal(result.outputSha256, OUTPUT_SHA256);
    assert.deepEqual(result.changedPaths, [...ALLOWED_CHANGED_PATHS].sort());
    assert.deepEqual(await readFile(fixture.sourcePath), before);
    const written = await readFile(fixture.outputPath);
    assert.equal(written.byteLength, OUTPUT_BYTE_SIZE);
    assert.equal(sha256(written), OUTPUT_SHA256);
    assert.equal((await lstat(fixture.outputPath)).isFile(), true);
    assert.equal((await lstat(fixture.outputPath)).isSymbolicLink(), false);
    assert.deepEqual(getChangedPaths(sourceProject, JSON.parse(written)), [...ALLOWED_CHANGED_PATHS].sort());
  } finally {
    await cleanup(fixture);
  }
}

// A stale source hash is rejected before publication and cannot leave output.
{
  const altered = Buffer.from(sourceBytes);
  altered[altered.length - 2] ^= 1;
  const fixture = await makeFixture(altered);
  try {
    await assertRejected(() => createAlpha42ArtNetProbe({
      sourcePath: fixture.sourcePath,
      outputPath: fixture.outputPath,
      allowTestPaths: true,
    }), /SHA-256|byte length/u, "source hash mismatch");
    await assertAbsent(fixture.outputPath, "hash rejection must not create output");
  } finally {
    await cleanup(fixture);
  }
}

// Production use is fixed-path only: arbitrary CLI/path overrides fail closed.
assertThrows(() => parseCliArgs(["--output", "operator-owned.sdc"]), /usage|fixed|arbitrary/u, "arbitrary CLI output");
{
  const fixture = await makeFixture();
  try {
    await assertRejected(() => createAlpha42ArtNetProbe({
      sourcePath: fixture.sourcePath,
      outputPath: fixture.outputPath,
    }), /fixed|arbitrary/u, "arbitrary API output");
    await assertAbsent(fixture.outputPath, "fixed-path rejection must not create output");
  } finally {
    await cleanup(fixture);
  }
}

// Existing files, directories, and reparse-backed leaves are never replaced.
{
  const fixture = await makeFixture();
  const sentinel = Buffer.from("operator-owned output\n", "utf8");
  try {
    await writeFile(fixture.outputPath, sentinel, { flag: "wx" });
    await assertRejected(() => createAlpha42ArtNetProbe({
      sourcePath: fixture.sourcePath,
      outputPath: fixture.outputPath,
      allowTestPaths: true,
    }), /already exists|overwrite/u, "existing output");
    assert.deepEqual(await readFile(fixture.outputPath), sentinel);
  } finally {
    await cleanup(fixture);
  }
}

{
  const fixture = await makeFixture();
  try {
    await rm(fixture.outputPath, { force: true });
    await mkdir(fixture.outputPath);
    await assertRejected(() => createAlpha42ArtNetProbe({
      sourcePath: fixture.sourcePath,
      outputPath: fixture.outputPath,
      allowTestPaths: true,
    }), /already exists|regular|directory/u, "non-regular output");
    assert.equal((await lstat(fixture.outputPath)).isDirectory(), true);
  } finally {
    await cleanup(fixture);
  }
}

{
  const fixture = await makeFixture();
  const targetPath = join(fixture.directory, "redirect-target.sdc");
  try {
    await writeFile(targetPath, Buffer.from("redirect target\n", "utf8"), { flag: "wx" });
    await symlink(targetPath, fixture.outputPath, "file");
    await assertRejected(() => createAlpha42ArtNetProbe({
      sourcePath: fixture.sourcePath,
      outputPath: fixture.outputPath,
      allowTestPaths: true,
    }), /symlink|reparse|redirect/u, "reparse-backed output");
    assert.deepEqual(await readFile(targetPath), Buffer.from("redirect target\n", "utf8"));
  } finally {
    await cleanup(fixture);
  }
}

// The output validator rejects any content or path beyond the two protocols.
{
  const derived = deriveAlpha42ArtNetProbeBytes(sourceBytes);
  const changed = structuredClone(derived.outputProject);
  changed.snapshot.timeline_bank[0].label = "operator mutation";
  assertThrows(() => validateAlpha42ArtNetProbeProject(changed, derived.sourceProject), /changed paths differ|timeline|allow/u, "non-protocol content drift");
}

console.log("DSF2026 alpha42 Art-Net probe: fixed identity, strict schema/counts, exact two-byte-literal patch, and exclusive-write rejection tests passed");
