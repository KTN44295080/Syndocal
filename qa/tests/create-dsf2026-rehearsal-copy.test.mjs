import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { copyFile, mkdtemp, readFile, rm, stat, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { basename, dirname, join } from "node:path";

import {
  APPROVED_PRODUCTION_IDENTITIES,
  DEFAULT_OUTPUT_PATH,
  PRODUCTION_SOURCE_PATH,
  REHEARSAL_MUTED_PATHS,
  assertRehearsalDiff,
  createRehearsalCopy,
  parseCliArgs,
} from "../../tools/create-dsf2026-rehearsal-copy.mjs";
import { normalizeIdentityPath } from "../../tools/dsf2026/io.mjs";

const productionDir = dirname(PRODUCTION_SOURCE_PATH);
const productionSource = await readFile(PRODUCTION_SOURCE_PATH);
const productionProject = JSON.parse(productionSource);
const productionSidecars = [
  join(productionDir, "dsf2026-reference-jinsei-over.mp3"),
  join(productionDir, "dsf2026-reference-madow-hoshi.mp3"),
];

assert.deepEqual(APPROVED_PRODUCTION_IDENTITIES.source, {
  byteSize: 1_095_864,
  sha256: "93e71d8ac3889968c2aad5b0a8ca194b88cb1c7b51bf897c7741c969d9a05094",
});

function sha256(bytes) {
  return createHash("sha256").update(bytes).digest("hex");
}

async function makeFixture({ mutateProject, mutateManifest } = {}) {
  const directory = await mkdtemp(join(tmpdir(), "syndocal-rehearsal-copy-"));
  const sourcePath = join(directory, basename(PRODUCTION_SOURCE_PATH));
  const sidecarPaths = productionSidecars.map((path) => join(directory, basename(path)));
  const project = structuredClone(productionProject);
  for (const [index, asset] of (project.snapshot.video.media_assets ?? []).entries()) {
    if (asset?.source?.path === productionSidecars[index]) asset.source.path = sidecarPaths[index];
  }
  mutateProject?.(project);
  const sourceBytes = Buffer.from(`${JSON.stringify(project, null, 2)}\n`, "utf8");
  await writeFile(sourcePath, sourceBytes, { flag: "wx" });
  await Promise.all(sidecarPaths.map((path, index) => copyFile(productionSidecars[index], path)));
  const sourceIdentity = {
    path: sourcePath,
    byteSize: sourceBytes.byteLength,
    sha256: sha256(sourceBytes),
  };
  const sidecarIdentities = await Promise.all(sidecarPaths.map(async (path) => {
    const bytes = await readFile(path);
    return { path, byteSize: bytes.byteLength, sha256: sha256(bytes) };
  }));
  const manifestObject = {
    version: 1,
    purpose: "dsf2026-rehearsal-reference-audio",
    source: sourceIdentity,
    sidecars: [
      { name: "dsf2026-reference-jinsei-over.mp3", ...sidecarIdentities[0] },
      { name: "dsf2026-reference-madow-hoshi.mp3", ...sidecarIdentities[1] },
    ],
  };
  mutateManifest?.(manifestObject);
  const manifestPath = join(directory, "approved-identity.json");
  await writeFile(manifestPath, `${JSON.stringify(manifestObject, null, 2)}\n`, { flag: "wx" });
  return { directory, sourcePath, sidecarPaths, manifestPath, sourceBytes, sourceProject: project };
}

async function cleanup(fixture) {
  await rm(fixture.directory, { recursive: true, force: true });
}

async function assertRejected(action, pattern) {
  await assert.rejects(action, (error) => {
    assert.match(String(error?.message ?? error), pattern);
    return true;
  });
}

async function assertAbsent(path) {
  await assert.rejects(stat(path), (error) => error?.code === "ENOENT");
}

// Success changes exactly three synchronized layer flags and leaves source and
// both referenced MP3s byte-identical.
{
  const fixture = await makeFixture();
  try {
    const outputPath = join(fixture.directory, "rehearsal.sdc");
    const sidecarsBefore = await Promise.all(fixture.sidecarPaths.map((path) => readFile(path)));
    const result = await createRehearsalCopy({
      sourcePath: fixture.sourcePath,
      manifestPath: fixture.manifestPath,
      outputPath,
      allowTestSource: true,
    });
    assert.equal(normalizeIdentityPath(result.outputPath), normalizeIdentityPath(outputPath));
    assert.deepEqual(result.changedPaths, REHEARSAL_MUTED_PATHS);
    const outputProject = JSON.parse(await readFile(outputPath, "utf8"));
    assertRehearsalDiff(fixture.sourceProject, outputProject);
    assert.deepEqual(await readFile(fixture.sourcePath), fixture.sourceBytes, "source must remain unchanged after publication");
    assert.deepEqual(await Promise.all(fixture.sidecarPaths.map((path) => readFile(path))), sidecarsBefore, "sidecars must remain unchanged after publication");
  } finally {
    await cleanup(fixture);
  }
}

// Exclusive creation must not overwrite an existing target.
{
  const fixture = await makeFixture();
  try {
    const outputPath = join(fixture.directory, "existing-rehearsal.sdc");
    const sentinel = Buffer.from("operator-owned target\n", "utf8");
    await writeFile(outputPath, sentinel, { flag: "wx" });
    await assertRejected(() => createRehearsalCopy({
      sourcePath: fixture.sourcePath,
      manifestPath: fixture.manifestPath,
      outputPath,
      allowTestSource: true,
    }), /already exists|overwrite/u);
    assert.deepEqual(await readFile(outputPath), sentinel, "existing output must not be overwritten");
  } finally {
    await cleanup(fixture);
  }
}

// Approval identities are mandatory and source/output are untouched if the
// source hash is stale.
{
  const fixture = await makeFixture({ mutateManifest: (manifest) => { manifest.source.sha256 = "0".repeat(64); } });
  try {
    const outputPath = join(fixture.directory, "bad-source-hash.sdc");
    await assertRejected(() => createRehearsalCopy({ sourcePath: fixture.sourcePath, manifestPath: fixture.manifestPath, outputPath, allowTestSource: true }), /approved identity|changed/u);
    assert.deepEqual(await readFile(fixture.sourcePath), fixture.sourceBytes);
    await assertAbsent(outputPath);
  } finally {
    await cleanup(fixture);
  }
}

// Sidecar paths are fixed siblings of the source; a manifest path substitution
// is rejected before any target can be created.
{
  const fixture = await makeFixture({ mutateManifest: (manifest) => { manifest.sidecars[0].path = join(fixtureDirectoryPlaceholder(), "wrong.mp3"); } });
  try {
    const outputPath = join(fixture.directory, "bad-sidecar-path.sdc");
    await assertRejected(() => createRehearsalCopy({ sourcePath: fixture.sourcePath, manifestPath: fixture.manifestPath, outputPath, allowTestSource: true }), /exact sibling path|cannot be read|path/u);
    assert.deepEqual(await readFile(fixture.sourcePath), fixture.sourceBytes);
    await assertAbsent(outputPath);
  } finally {
    await cleanup(fixture);
  }
}

function fixtureDirectoryPlaceholder() {
  // The fixture callback cannot access the returned directory.  Any path not
  // equal to the source parent exercises the exact-sibling rejection first.
  return join(tmpdir(), "not-the-source-parent");
}

// An extra ordinary Reference Audio target is ambiguous and must fail closed.
{
  const fixture = await makeFixture({ mutateProject: (project) => {
    project.snapshot.timeline_bank[1].layers.push(structuredClone(project.snapshot.timeline_bank[1].layers[0]));
  } });
  try {
    const outputPath = join(fixture.directory, "extra-reference-layer.sdc");
    await assertRejected(() => createRehearsalCopy({ sourcePath: fixture.sourcePath, manifestPath: fixture.manifestPath, outputPath, allowTestSource: true }), /exactly one ordinary Reference Audio/u);
    assert.deepEqual(await readFile(fixture.sourcePath), fixture.sourceBytes);
    await assertAbsent(outputPath);
  } finally {
    await cleanup(fixture);
  }
}

// The source boundary requires all three pre-publication occurrences to start
// muted; an already-unmuted flag is not silently normalized.
{
  const fixture = await makeFixture({ mutateProject: (project) => {
    project.snapshot.timeline.layers[0].muted = false;
    project.snapshot.timeline_bank[0].layers[0].muted = false;
  } });
  try {
    const outputPath = join(fixture.directory, "wrong-mute-count.sdc");
    await assertRejected(() => createRehearsalCopy({ sourcePath: fixture.sourcePath, manifestPath: fixture.manifestPath, outputPath, allowTestSource: true }), /muted must be true/u);
    assert.deepEqual(await readFile(fixture.sourcePath), fixture.sourceBytes);
    await assertAbsent(outputPath);
  } finally {
    await cleanup(fixture);
  }
}

// A sidecar replacement between ordinary preflight and the publication fence
// is rejected without creating an output.
{
  const fixture = await makeFixture();
  try {
    const outputPath = join(fixture.directory, "sidecar-race.sdc");
    await assertRejected(() => createRehearsalCopy({
      sourcePath: fixture.sourcePath,
      manifestPath: fixture.manifestPath,
      outputPath,
      allowTestSource: true,
      testHooks: {
        beforePublication: async ({ sidecars }) => {
          await writeFile(sidecars[0].path, Buffer.from("changed during publication\n", "utf8"));
        },
      },
    }), /publication fence|approved identity|changed/u);
    assert.deepEqual(await readFile(fixture.sourcePath), fixture.sourceBytes);
    await assertAbsent(outputPath);
  } finally {
    await cleanup(fixture);
  }
}

// The normal boundary rejects a self-consistent fixture manifest because the
// source identity is not the fixed production candidate. Only the explicit
// test boundary permits that fixture.
{
  const fixture = await makeFixture();
  try {
    const outputPath = join(fixture.directory, "normal-boundary-reject.sdc");
    await assertRejected(() => createRehearsalCopy({
      sourcePath: fixture.sourcePath,
      manifestPath: fixture.manifestPath,
      outputPath,
    }), /fixed to the exact authored production candidate/u);
    assert.deepEqual(await readFile(fixture.sourcePath), fixture.sourceBytes);
    await assertAbsent(outputPath);
  } finally {
    await cleanup(fixture);
  }
}

// Duplicate flags are rejected even when the first explicit output equals the
// default output path.
assert.throws(() => parseCliArgs([
  "--manifest", "approved.json",
  "--output", "default.sdc",
  "--output", "second.sdc",
]), /usage:/iu);

// A manifest can be internally coherent yet still be rejected by the normal
// code-side authority when its identity is not the approved production one.
{
  const fixture = await makeFixture();
  try {
    const nonCanonical = JSON.parse(await readFile(fixture.manifestPath, "utf8"));
    nonCanonical.source = {
      path: PRODUCTION_SOURCE_PATH,
      byteSize: APPROVED_PRODUCTION_IDENTITIES.source.byteSize + 1,
      sha256: "1".repeat(64),
    };
    nonCanonical.sidecars = nonCanonical.sidecars.map((sidecar) => ({
      ...sidecar,
      path: join(dirname(PRODUCTION_SOURCE_PATH), sidecar.name),
      byteSize: APPROVED_PRODUCTION_IDENTITIES.sidecars[sidecar.name].byteSize + 1,
      sha256: "2".repeat(64),
    }));
    await writeFile(fixture.manifestPath, `${JSON.stringify(nonCanonical, null, 2)}\n`, { flag: "w" });
    await assertRejected(() => createRehearsalCopy({
      manifestPath: fixture.manifestPath,
      outputPath: DEFAULT_OUTPUT_PATH,
    }), /exact approved production identity/u);
  } finally {
    await cleanup(fixture);
  }
}

// Rehearsal publication is an .sdc-only operation.
{
  const fixture = await makeFixture();
  try {
    const outputPath = join(fixture.directory, "rehearsal.json");
    await assertRejected(() => createRehearsalCopy({
      sourcePath: fixture.sourcePath,
      manifestPath: fixture.manifestPath,
      outputPath,
      allowTestSource: true,
    }), /exact .sdc extension/u);
    assert.deepEqual(await readFile(fixture.sourcePath), fixture.sourceBytes);
    await assertAbsent(outputPath);
  } finally {
    await cleanup(fixture);
  }
}

// A source replacement after the initial transformation but before the final
// fence must not produce an output. The simulated external change is restored
// only so the temporary fixture can be removed cleanly.
{
  const fixture = await makeFixture();
  try {
    const outputPath = join(fixture.directory, "source-race.sdc");
    await assertRejected(() => createRehearsalCopy({
      sourcePath: fixture.sourcePath,
      manifestPath: fixture.manifestPath,
      outputPath,
      allowTestSource: true,
      testHooks: {
        beforePublication: async ({ sourcePath }) => {
          await writeFile(sourcePath, Buffer.from("changed source during publication\n", "utf8"));
        },
      },
    }), /approved identity|changed/u);
    await writeFile(fixture.sourcePath, fixture.sourceBytes, { flag: "w" });
    await assertAbsent(outputPath);
  } finally {
    await cleanup(fixture);
  }
}

// An output sentinel created after prepareOutputPath must remain untouched when
// exclusive publication observes the race.
{
  const fixture = await makeFixture();
  try {
    const outputPath = join(fixture.directory, "output-race.sdc");
    const sentinel = Buffer.from("external output sentinel\n", "utf8");
    await assertRejected(() => createRehearsalCopy({
      sourcePath: fixture.sourcePath,
      manifestPath: fixture.manifestPath,
      outputPath,
      allowTestSource: true,
      testHooks: {
        beforePublication: async () => {
          await writeFile(outputPath, sentinel, { flag: "wx" });
        },
      },
    }), /already exists|overwrite|native exclusive/u);
    assert.deepEqual(await readFile(outputPath), sentinel, "output sentinel must not be overwritten");
    assert.deepEqual(await readFile(fixture.sourcePath), fixture.sourceBytes);
  } finally {
    await cleanup(fixture);
  }
}

console.log("DSF2026 rehearsal-copy: exact mute whitelist, identity, exclusive-create, and race rejection tests passed");
