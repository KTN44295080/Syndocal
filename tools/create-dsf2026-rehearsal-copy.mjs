import { createHash } from "node:crypto";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { fail, isObject, parseJsonText } from "./dsf2026/common.mjs";
import {
  normalizeIdentityPath,
  prepareOutputPath,
  readRegularFileBytesNoReparse,
  readStrictJsonFile,
  writeExclusiveBytes,
} from "./dsf2026/io.mjs";
import { preflightShowContract } from "./show-structural-preflight.mjs";
import { stableJson } from "./show-structural-preflight/primitives.mjs";

const REPO_ROOT = resolve(fileURLToPath(new URL("../", import.meta.url)));
const SOURCE_RELATIVE_PATH = join(
  "target",
  "qa",
  "dsf2026-show-authored-20260828",
  "DSF2026-show-alpha9-reference-audio.sdc",
);

export const PRODUCTION_SOURCE_PATH = resolve(REPO_ROOT, SOURCE_RELATIVE_PATH);
export const DEFAULT_OUTPUT_PATH = join(
  dirname(PRODUCTION_SOURCE_PATH),
  "DSF2026-show-alpha9-rehearsal-reference-audio.sdc",
);
export const MAX_PROJECT_BYTES = 64 * 1024 * 1024;
export const MAX_MANIFEST_BYTES = 1024 * 1024;
export const APPROVED_PRODUCTION_IDENTITIES = Object.freeze({
  source: Object.freeze({
    byteSize: 1_095_864,
    sha256: "93e71d8ac3889968c2aad5b0a8ca194b88cb1c7b51bf897c7741c969d9a05094",
  }),
  sidecars: Object.freeze({
    "dsf2026-reference-jinsei-over.mp3": Object.freeze({
      byteSize: 8_561_392,
      sha256: "de7a0e78d269643035823df57ad5dd98fe1294583f2a072b91aea9c66a66c7ca",
    }),
    "dsf2026-reference-madow-hoshi.mp3": Object.freeze({
      byteSize: 9_738_524,
      sha256: "e82ae9e8641ccd798fc5e90dc40603286d6965d9e726989f018871c2446c3c3b",
    }),
  }),
});

const EXPECTED_SOURCE_TIMELINE_ID = 1;
const EXPECTED_DESTINATION_TIMELINE_ID = 2;
const EXPECTED_REFERENCE_LAYER_LABEL = "Reference Audio";
const EXPECTED_REFERENCE_AUDIO = Object.freeze([
  Object.freeze({
    name: "dsf2026-reference-jinsei-over.mp3",
    label: "Jinsei Over Reference Audio",
    timelineId: EXPECTED_SOURCE_TIMELINE_ID,
  }),
  Object.freeze({
    name: "dsf2026-reference-madow-hoshi.mp3",
    label: "Madow Hoshi Reference Audio",
    timelineId: EXPECTED_DESTINATION_TIMELINE_ID,
  }),
]);

export const REHEARSAL_MUTED_PATHS = Object.freeze([
  "/snapshot/timeline/layers/0/muted",
  "/snapshot/timeline_bank/0/layers/0/muted",
  "/snapshot/timeline_bank/1/layers/0/muted",
]);

function sha256(bytes) {
  return createHash("sha256").update(bytes).digest("hex");
}

function failUsage() {
  fail("usage: node tools/create-dsf2026-rehearsal-copy.mjs --manifest <approved-identity.json> [--output <rehearsal.sdc>]");
}

function assertObject(value, label) {
  if (!isObject(value)) fail(`${label} must be an object`);
  return value;
}

function assertExactKeys(value, keys, label) {
  const actual = Object.keys(value).sort();
  const expected = [...keys].sort();
  if (actual.length !== expected.length || actual.some((key, index) => key !== expected[index])) {
    fail(`${label} has unexpected keys; expected exactly ${expected.join(", ")}`);
  }
}

function assertIdentity(value, label) {
  assertObject(value, label);
  assertExactKeys(value, ["path", "byteSize", "sha256"], label);
  if (typeof value.path !== "string" || value.path.trim().length === 0) fail(`${label}.path must be a non-empty path`);
  if (!Number.isSafeInteger(value.byteSize) || value.byteSize <= 0) fail(`${label}.byteSize must be a positive safe integer`);
  if (typeof value.sha256 !== "string" || !/^[a-f0-9]{64}$/iu.test(value.sha256)) fail(`${label}.sha256 must be a 64-character SHA-256 value`);
  return {
    path: resolve(value.path),
    byteSize: value.byteSize,
    sha256: value.sha256.toLowerCase(),
  };
}

function parseManifestValue(value) {
  assertObject(value, "approval manifest");
  assertExactKeys(value, ["version", "purpose", "source", "sidecars"], "approval manifest");
  if (value.version !== 1) fail("approval manifest version must be exactly 1");
  if (value.purpose !== "dsf2026-rehearsal-reference-audio") fail("approval manifest purpose is not the exact rehearsal-copy purpose");
  const source = assertIdentity(value.source, "approval manifest source");
  if (!Array.isArray(value.sidecars) || value.sidecars.length !== EXPECTED_REFERENCE_AUDIO.length) {
    fail(`approval manifest must contain exactly ${EXPECTED_REFERENCE_AUDIO.length} sidecars`);
  }
  const sidecars = value.sidecars.map((entry, index) => {
    assertObject(entry, `approval manifest sidecars[${index}]`);
    assertExactKeys(entry, ["name", "path", "byteSize", "sha256"], `approval manifest sidecars[${index}]`);
    if (typeof entry.name !== "string" || entry.name.length === 0) fail(`approval manifest sidecars[${index}].name must be non-empty`);
    const expected = EXPECTED_REFERENCE_AUDIO[index];
    if (entry.name !== expected?.name) fail(`approval manifest sidecars[${index}] must be the exact ordered sidecar ${expected?.name ?? "reference-audio"}`);
    const identity = assertIdentity({ path: entry.path, byteSize: entry.byteSize, sha256: entry.sha256 }, `approval manifest sidecars[${index}]`);
    return { ...expected, ...identity };
  });
  const names = sidecars.map(({ name }) => name);
  if (new Set(names).size !== names.length || EXPECTED_REFERENCE_AUDIO.some(({ name }, index) => names[index] !== name)) {
    fail("approval manifest sidecars must contain each exact reference-audio name once in canonical order");
  }
  return { version: value.version, purpose: value.purpose, source, sidecars };
}

function assertProductionManifest(manifest, sourcePath, sourceParent) {
  if (normalizeIdentityPath(sourcePath) !== normalizeIdentityPath(PRODUCTION_SOURCE_PATH)) {
    fail("normal rehearsal publication source is not the fixed production candidate");
  }
  if (normalizeIdentityPath(manifest.source.path) !== normalizeIdentityPath(PRODUCTION_SOURCE_PATH)) {
    fail("approval manifest source.path is not the fixed production candidate path");
  }
  if (manifest.source.byteSize !== APPROVED_PRODUCTION_IDENTITIES.source.byteSize || manifest.source.sha256 !== APPROVED_PRODUCTION_IDENTITIES.source.sha256) {
    fail("approval manifest source identity is not the exact approved production identity");
  }
  for (const sidecar of manifest.sidecars) {
    const expectedPath = resolve(sourceParent, sidecar.name);
    const expectedIdentity = APPROVED_PRODUCTION_IDENTITIES.sidecars[sidecar.name];
    if (normalizeIdentityPath(sidecar.path) !== normalizeIdentityPath(expectedPath)) {
      fail(`approval manifest sidecar path is not the fixed production sibling path for ${sidecar.name}`);
    }
    if (!expectedIdentity || sidecar.byteSize !== expectedIdentity.byteSize || sidecar.sha256 !== expectedIdentity.sha256) {
      fail(`approval manifest sidecar identity is not the exact approved production identity for ${sidecar.name}`);
    }
  }
}

function assertPathInParent(path, parent, label) {
  if (normalizeIdentityPath(dirname(path)) !== normalizeIdentityPath(parent)) {
    fail(`${label} must be a sibling in the exact source parent`);
  }
}

function findTimelineById(bank, id, label) {
  const matches = bank.filter((timeline) => timeline?.id === id);
  if (matches.length !== 1) fail(`${label} must contain exactly one Timeline ${id}`);
  return matches[0];
}

function findReferenceLayer(timeline, pathLabel, expectedMuted) {
  if (!isObject(timeline) || !Array.isArray(timeline.layers)) fail(`${pathLabel}.layers must be an array`);
  const matches = timeline.layers
    .map((layer, index) => ({ layer, index }))
    .filter(({ layer }) => layer?.kind === "Audio" && layer?.label === EXPECTED_REFERENCE_LAYER_LABEL);
  if (matches.length !== 1) fail(`${pathLabel} must contain exactly one ordinary ${EXPECTED_REFERENCE_LAYER_LABEL} Audio layer`);
  const match = matches[0];
  if (match.layer.muted !== expectedMuted) {
    fail(`${pathLabel}.layers[${match.index}].muted must be ${String(expectedMuted)} for the rehearsal-copy boundary`);
  }
  return match;
}

function assertReferenceMediaAssets(project, sidecars, sourceParent) {
  const assets = project.snapshot?.video?.media_assets;
  if (!Array.isArray(assets)) fail("snapshot.video.media_assets is missing; reference audio identity cannot be verified");
  for (const sidecar of sidecars) {
    const matches = assets.filter((asset) => asset?.label === sidecar.label);
    if (matches.length !== 1) fail(`MediaAsset ${sidecar.label} must occur exactly once`);
    const asset = matches[0];
    const actualPath = typeof asset.source?.path === "string" ? resolve(asset.source.path) : "";
    const expectedPath = resolve(sourceParent, sidecar.name);
    if (normalizeIdentityPath(actualPath) !== normalizeIdentityPath(expectedPath)) {
      fail(`MediaAsset ${sidecar.label} path is not the exact sibling ${expectedPath}`);
    }
    if (asset.source?.codec !== "mp3" || asset.source?.kind !== "File") fail(`MediaAsset ${sidecar.label} must remain an ordinary File/mp3 asset`);
    if (asset.byte_size !== sidecar.byteSize) fail(`MediaAsset ${sidecar.label} byte_size differs from the approved sidecar identity`);
    if (asset.content_hash?.algorithm !== "Sha256" || String(asset.content_hash?.hex).toLowerCase() !== sidecar.sha256) {
      fail(`MediaAsset ${sidecar.label} content_hash differs from the approved sidecar identity`);
    }
  }
}

function assertReferenceClipLinks(project, sourceTimeline, destinationTimeline, sourceLayer, destinationLayer, sidecars) {
  const assets = project.snapshot.video.media_assets;
  const timelines = [sourceTimeline, destinationTimeline];
  const layers = [sourceLayer.layer, destinationLayer.layer];
  for (const [index, timeline] of timelines.entries()) {
    const expectedSidecar = sidecars.find(({ timelineId }) => timelineId === timeline.id);
    const asset = assets.find(({ label }) => label === expectedSidecar.label);
    const clips = (timeline.audio_clips ?? []).filter((clip) => clip?.layer_id === layers[index].id && clip?.media_asset_id === asset.id);
    if (clips.length !== 1) fail(`Timeline ${timeline.id} must retain exactly one ordinary Reference Audio clip link`);
  }
}

function assertProjectInvariants(project, sourceText, sidecars, expectedMuted) {
  const preflight = preflightShowContract(sourceText);
  if (preflight.status !== "PASS") {
    const blocked = preflight.checks.filter(({ status }) => status === "BLOCKED").map(({ id, detail }) => `${id}: ${detail}`).join("; ");
    fail(`source authored structural preflight failed: ${blocked}`);
  }
  assertObject(project, "Syndocal project");
  assertObject(project.snapshot, "Syndocal project snapshot");
  const active = project.snapshot.timeline;
  const bank = project.snapshot.timeline_bank;
  if (!Array.isArray(bank) || bank.length !== 2) fail("exact rehearsal source must contain exactly two authored Timeline bank entries");
  if (active?.id !== EXPECTED_SOURCE_TIMELINE_ID) fail("active authored Timeline must be exactly Timeline 1");
  const sourceTimeline = findTimelineById(bank, EXPECTED_SOURCE_TIMELINE_ID, "Timeline bank");
  const destinationTimeline = findTimelineById(bank, EXPECTED_DESTINATION_TIMELINE_ID, "Timeline bank");
  if (sourceTimeline.label !== "人生オーバー" || destinationTimeline.label !== "惑う星") {
    fail("the exact rehearsal source must retain the two authored Timeline labels");
  }
  if (active.audio_muted !== false || sourceTimeline.audio_muted !== false || destinationTimeline.audio_muted !== false) {
    fail("Timeline master audio_muted must remain false; only the two Reference Audio layers may change");
  }
  const activeLayer = findReferenceLayer(active, "snapshot.timeline", expectedMuted);
  const sourceLayer = findReferenceLayer(sourceTimeline, "snapshot.timeline_bank[0]", expectedMuted);
  const destinationLayer = findReferenceLayer(destinationTimeline, "snapshot.timeline_bank[1]", expectedMuted);
  if (activeLayer.index !== 0 || sourceLayer.index !== 0 || destinationLayer.index !== 0) {
    fail("Reference Audio layers must remain at the exact synchronized layer index 0");
  }
  if (stableJson(activeLayer.layer) !== stableJson(sourceLayer.layer)) {
    fail("active Timeline 1 and authored bank Timeline 1 Reference Audio layers are not synchronized");
  }
  const referenceLayerCount = bank.flatMap((timeline) => timeline.layers.filter((layer) => layer?.kind === "Audio" && layer?.label === EXPECTED_REFERENCE_LAYER_LABEL)).length + 1;
  if (referenceLayerCount !== REHEARSAL_MUTED_PATHS.length) fail(`expected exactly ${REHEARSAL_MUTED_PATHS.length} synchronized Reference Audio muted occurrences; found ${referenceLayerCount}`);
  assertReferenceMediaAssets(project, sidecars, dirname(sidecars[0].path));
  assertReferenceClipLinks(project, sourceTimeline, destinationTimeline, sourceLayer, destinationLayer, sidecars);
  return {
    targets: [
      ["snapshot", "timeline", "layers", 0, "muted"],
      ["snapshot", "timeline_bank", 0, "layers", 0, "muted"],
      ["snapshot", "timeline_bank", 1, "layers", 0, "muted"],
    ],
  };
}

function pathString(path) {
  return `/${path.map((part) => String(part).replaceAll("~", "~0").replaceAll("/", "~1")).join("/")}`;
}

function diffJson(left, right, path = [], differences = []) {
  if (Object.is(left, right)) return differences;
  if (Array.isArray(left) && Array.isArray(right)) {
    if (left.length !== right.length) differences.push({ path: pathString(path), kind: "array-length", before: left.length, after: right.length });
    for (let index = 0; index < Math.max(left.length, right.length); index += 1) diffJson(left[index], right[index], path.concat(index), differences);
    return differences;
  }
  if (isObject(left) && isObject(right)) {
    const keys = new Set([...Object.keys(left), ...Object.keys(right)]);
    for (const key of [...keys].sort()) diffJson(left[key], right[key], path.concat(key), differences);
    return differences;
  }
  differences.push({ path: pathString(path), kind: "value", before: left, after: right });
  return differences;
}

export function assertRehearsalDiff(before, after) {
  const differences = diffJson(before, after);
  if (differences.length !== REHEARSAL_MUTED_PATHS.length) fail(`rehearsal copy changed ${differences.length} JSON values; exactly the three Reference Audio muted flags are allowed`);
  const actualPaths = differences.map(({ path }) => path).sort();
  const expectedPaths = [...REHEARSAL_MUTED_PATHS].sort();
  if (actualPaths.some((path, index) => path !== expectedPaths[index])) fail(`rehearsal copy changed paths outside the exact mute whitelist: ${actualPaths.join(", ")}`);
  for (const difference of differences) {
    if (difference.kind !== "value" || difference.before !== true || difference.after !== false) {
      fail(`rehearsal copy mute whitelist contains an invalid change at ${difference.path}`);
    }
  }
  return differences;
}

async function readIdentity(path, label) {
  const file = await readRegularFileBytesNoReparse(path, label, MAX_PROJECT_BYTES);
  return { ...file, byteSize: file.bytes.byteLength, sha256: sha256(file.bytes) };
}

function assertObservedIdentity(observed, expected, label) {
  if (observed.byteSize !== expected.byteSize || observed.sha256 !== expected.sha256) {
    fail(`${label} changed from its approved identity; refusing rehearsal publication`);
  }
}

async function verifyManifestFiles(manifest, sourcePath, sourceParent) {
  if (normalizeIdentityPath(manifest.source.path) !== normalizeIdentityPath(sourcePath)) {
    fail("approval manifest source.path is not the exact source candidate path");
  }
  const source = await readIdentity(sourcePath, "exact rehearsal source");
  assertObservedIdentity(source, manifest.source, "exact rehearsal source");
  const sidecars = [];
  for (const entry of manifest.sidecars) {
    const expectedPath = resolve(sourceParent, entry.name);
    if (normalizeIdentityPath(entry.path) !== normalizeIdentityPath(expectedPath)) {
      fail(`approval manifest sidecar path is not the exact sibling path for ${entry.name}`);
    }
    const observed = await readIdentity(entry.path, `reference sidecar ${entry.name}`);
    assertObservedIdentity(observed, entry, `reference sidecar ${entry.name}`);
    sidecars.push(entry);
  }
  return { source, sidecars };
}

function buildOutputText(sourceBytes, sourcePath, sidecars) {
  let sourceText;
  try {
    sourceText = new TextDecoder("utf-8", { fatal: true, ignoreBOM: true }).decode(sourceBytes);
  } catch (error) {
    fail(`exact rehearsal source is not strict UTF-8: ${String(error?.message ?? error)}`);
  }
  const sourceProject = parseJsonText(sourceText, "exact rehearsal source");
  const invariant = assertProjectInvariants(sourceProject, sourceText, sidecars, true);
  const transformed = structuredClone(sourceProject);
  for (const path of invariant.targets) {
    let cursor = transformed;
    for (const part of path.slice(0, -1)) cursor = cursor[part];
    cursor[path.at(-1)] = false;
  }
  assertProjectInvariants(transformed, sourceText, sidecars, false);
  assertRehearsalDiff(sourceProject, transformed);
  // The parsed current-format JSON is serialized with stable indentation so
  // the rehearsal artifact is deterministic for one approved source identity.
  return { text: `${JSON.stringify(transformed, null, 2)}\n`, sourceProject, transformed };
}

export function parseCliArgs(argv) {
  const args = [...argv];
  let manifest;
  let output = DEFAULT_OUTPUT_PATH;
  const seen = new Set();
  while (args.length > 0) {
    const flag = args.shift();
    const value = args.shift();
    if (!value || value.startsWith("--") || (flag !== "--manifest" && flag !== "--output") || seen.has(flag)) {
      failUsage();
    }
    seen.add(flag);
    if (flag === "--manifest") manifest = resolve(value);
    else output = resolve(value);
  }
  if (!manifest) failUsage();
  return { manifest, output };
}

export async function createRehearsalCopy({ manifestPath, outputPath = DEFAULT_OUTPUT_PATH, sourcePath = PRODUCTION_SOURCE_PATH, allowTestSource = false, testHooks = null }) {
  const exactSource = resolve(sourcePath);
  if (!allowTestSource && normalizeIdentityPath(exactSource) !== normalizeIdentityPath(PRODUCTION_SOURCE_PATH)) {
    fail("rehearsal copy source is fixed to the exact authored production candidate");
  }
  const absoluteOutput = resolve(outputPath);
  const sourceParent = dirname(exactSource);
  assertPathInParent(absoluteOutput, sourceParent, "rehearsal output");
  if (!absoluteOutput.toLowerCase().endsWith(".sdc")) fail("rehearsal output must use the exact .sdc extension");
  if (normalizeIdentityPath(absoluteOutput) === normalizeIdentityPath(exactSource)) fail("rehearsal output must not overwrite the exact source candidate");
  if (typeof manifestPath !== "string" || manifestPath.trim().length === 0) fail("an approved source/sidecar identity manifest is required");
  const manifestFile = await readStrictJsonFile(resolve(manifestPath), "rehearsal approval manifest", MAX_MANIFEST_BYTES);
  const manifest = parseManifestValue(manifestFile.value);
  if (!allowTestSource) assertProductionManifest(manifest, exactSource, sourceParent);
  const observed = await verifyManifestFiles(manifest, exactSource, sourceParent);
  const output = buildOutputText(observed.source.bytes, exactSource, observed.sidecars);
  const outputGuard = {};
  const publication = await prepareOutputPath(absoluteOutput, { guard: outputGuard });
  await testHooks?.beforePublication?.({ sourcePath: exactSource, sidecars: observed.sidecars, outputPath: publication });
  // Re-read immediately before publication.  The native writer receives all
  // three identities as held-parent fences and repeats these checks at its
  // exclusive-create boundary, so a source/sidecar race cannot publish.
  const finalSource = await readIdentity(exactSource, "exact rehearsal source publication fence");
  assertObservedIdentity(finalSource, manifest.source, "exact rehearsal source publication fence");
  const fences = [{ path: exactSource, byteSize: manifest.source.byteSize, sha256: manifest.source.sha256 }, ...observed.sidecars.map(({ path, byteSize, sha256 }) => ({ path, byteSize, sha256 }))];
  for (const fence of fences.slice(1)) {
    const finalSidecar = await readIdentity(fence.path, "reference sidecar publication fence");
    assertObservedIdentity(finalSidecar, fence, "reference sidecar publication fence");
  }
  await writeExclusiveBytes(publication, Buffer.from(output.text, "utf8"), outputGuard, { fences });
  return {
    sourcePath: exactSource,
    outputPath: publication,
    sidecarPaths: observed.sidecars.map(({ path }) => path),
    changedPaths: [...REHEARSAL_MUTED_PATHS],
    outputByteSize: Buffer.byteLength(output.text, "utf8"),
  };
}

async function main(argv) {
  const { manifest, output } = parseCliArgs(argv);
  const result = await createRehearsalCopy({ manifestPath: manifest, outputPath: output });
  console.log("DSF2026 rehearsal copy created with the exact three-flag whitelist.");
  console.log(`source: ${result.sourcePath}`);
  console.log(`output: ${result.outputPath}`);
  console.log(`changed: ${result.changedPaths.join(", ")}`);
}

if (normalizeIdentityPath(resolve(process.argv[1] ?? "")) === normalizeIdentityPath(fileURLToPath(import.meta.url))) {
  main(process.argv.slice(2)).catch((error) => {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 2;
  });
}
