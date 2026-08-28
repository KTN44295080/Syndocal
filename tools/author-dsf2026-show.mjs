import { createHash } from "node:crypto";
import { fileURLToPath } from "node:url";
import { basename, resolve } from "node:path";

import { preflightShowContract as structuralPreflightShowContract } from "./show-structural-preflight.mjs";
import {
  ShowAuthorError,
  decodeUtf8Strict,
  fail,
  parseJsonText,
} from "./dsf2026/common.mjs";
import {
  CANONICAL_MANIFEST_PATH,
  CANONICAL_MANIFEST_SHA256,
  validateCanonicalManifest as validateManifestContract,
} from "./dsf2026/manifest.mjs";
import {
  authoredTimelineProjection,
  resolveExistingLightingCues,
  stageDmxOutput,
  SHOW_DMX_PROTOCOL,
  SHOW_DMX_SERIAL_PORT,
  SHOW_DMX_SERIAL_BAUD_RATE,
  validateDmxOutputStaging,
  validateLightingBoundary,
  validateBaseProject,
  validateEmptyTimeline,
} from "./dsf2026/base.mjs";
import {
  buildAuthoredTimelines,
  buildBoundaryLightingEvents,
  buildPhases,
  buildTempoMapOne,
  buildTempoMapTwo,
  emptyTimelineBase,
  findClick,
  roundFrameToMs,
  applyReferenceAudioToTimelines,
  validateGeneratedTimelineLayerReferences,
} from "./dsf2026/timeline.mjs";
import {
  assertSafePath,
  MAX_BASE_BYTES,
  MAX_MANIFEST_BYTES,
  normalizeIdentityPath,
  parseCliArgs,
  prepareOutputPath,
  readStrictJsonFile,
  requireRegularFile,
  writeExclusive,
} from "./dsf2026/io.mjs";
import {
  materializeReferenceAudio,
  planReferenceAudio,
  prepareReferenceAudio,
  REFERENCE_AUDIO_SIDECAR_NAMES,
  referenceAudioPublicationFences,
} from "./dsf2026/reference-audio.mjs";

export const APPROVED_BASE_FILENAME = "DSF2026-imported-alpha27.sdc";
export const APPROVED_BASE_SHA256 = "B21165A70A41A4036153359E579E1433C2739EC1C3EDCC0C46B94F513238DFB1";

export {
  CANONICAL_MANIFEST_PATH,
  CANONICAL_MANIFEST_SHA256,
  MAX_BASE_BYTES,
  MAX_MANIFEST_BYTES,
  ShowAuthorError,
  authoredTimelineProjection,
  assertSafePath,
  buildAuthoredTimelines,
  buildBoundaryLightingEvents,
  buildPhases,
  buildTempoMapOne,
  buildTempoMapTwo,
  emptyTimelineBase,
  findClick,
  normalizeIdentityPath,
  parseCliArgs,
  prepareOutputPath,
  readStrictJsonFile,
  requireRegularFile,
  roundFrameToMs,
  validateGeneratedTimelineLayerReferences,
  validateBaseProject,
  resolveExistingLightingCues,
  stageDmxOutput,
  validateDmxOutputStaging,
  validateLightingBoundary,
  validateEmptyTimeline,
  applyReferenceAudioToTimelines,
  prepareReferenceAudio,
  planReferenceAudio,
  materializeReferenceAudio,
  referenceAudioPublicationFences,
  SHOW_DMX_PROTOCOL,
  SHOW_DMX_SERIAL_PORT,
  SHOW_DMX_SERIAL_BAUD_RATE,
};

export const validateCanonicalManifest = validateManifestContract;
export const validateManifest = validateManifestContract;

export function verifyApprovedBaseIdentity(path, bytes) {
  if (basename(path).toLowerCase() !== APPROVED_BASE_FILENAME.toLowerCase()) {
    fail(`base .sdc must be the approved ${APPROVED_BASE_FILENAME}; arbitrary structurally-valid bases are not accepted`);
  }
  const actualHash = createHash("sha256").update(bytes).digest("hex").toUpperCase();
  if (actualHash !== APPROVED_BASE_SHA256) {
    fail(`base .sdc SHA-256 ${actualHash} does not match the approved ${APPROVED_BASE_FILENAME}`);
  }
  return actualHash;
}

export function preflightShowContract(input, options = {}) {
  const report = structuralPreflightShowContract(input, options);
  if (report.status !== "PASS") return report;
  const project = typeof input === "string" ? parseJsonText(input, "preflight project") : input;
  const lighting = validateLightingBoundary(project);
  const dmx = validateDmxOutputStaging(project);
  const extraChecks = [
    lighting.error
      ? { id: "lighting_boundary", status: "BLOCKED", detail: lighting.error }
      : { id: "lighting_boundary", status: "PASS", detail: lighting.detail },
    dmx.error
      ? { id: "dmx_staging", status: "BLOCKED", detail: dmx.error }
      : { id: "dmx_staging", status: "PASS", detail: dmx.detail },
  ];
  const blocked = extraChecks.some((check) => check.status === "BLOCKED");
  return {
    ...report,
    status: blocked ? "BLOCKED" : "PASS",
    checks: [...report.checks, ...extraChecks],
  };
}

const PRODUCTION_MAPPING = Object.freeze({
  id: "jinsei-over-production",
  selector: Object.freeze({ titleContains: "人生オーバー", fallbackDeck: 1 }),
  timelineId: 1,
  retrigger: "once_per_play_session",
});

export function authorDsf2026Show(baseInput, manifestInput, options = {}) {
  const base = validateBaseProject(baseInput);
  const manifest = validateManifestContract(manifestInput);
  const lightingCues = resolveExistingLightingCues(base);
  const { source, destination } = buildAuthoredTimelines(manifest, lightingCues);
  const output = structuredClone(base);
  const references = options?.referenceAudio ?? null;
  if (references !== null) {
    const mediaAssets = output.snapshot.video?.media_assets ?? [];
    const applied = applyReferenceAudioToTimelines(source, destination, references, mediaAssets);
    if (!output.snapshot.video || typeof output.snapshot.video !== "object") fail("base snapshot.video is required for Media Library reference audio");
    output.snapshot.video.media_assets = applied.mediaAssets;
  }
  validateGeneratedTimelineLayerReferences(source, "authored source Timeline");
  validateGeneratedTimelineLayerReferences(destination, "authored destination Timeline");
  output.dj_track_triggers = [structuredClone(PRODUCTION_MAPPING)];
  output.snapshot.timeline = source;
  output.snapshot.timeline_bank = [structuredClone(source), destination];
  const stagedDmx = stageDmxOutput(base);
  output.snapshot.output = stagedDmx.output;
  output.snapshot.dmx_outputs = stagedDmx.dmx_outputs;
  return output;
}

export const authorShowProject = authorDsf2026Show;
export const buildAuthoredShow = authorDsf2026Show;
export const authorShow = authorDsf2026Show;
export const buildShow = authorDsf2026Show;

function assertAuthoredOutputPreflight(output) {
  const outputPreflight = preflightShowContract(output);
  if (outputPreflight.status !== "PASS") {
    const blocked = outputPreflight.checks.filter(({ status }) => status !== "PASS").map(({ id, detail }) => `${id}: ${detail}`).join("; ");
    fail(`generated show preflight failed before write${blocked ? `: ${blocked}` : ""}`);
  }
}

function outputUsesManagedReferenceAudio(output) {
  const managedNames = new Set(Object.values(REFERENCE_AUDIO_SIDECAR_NAMES).map((name) => name.toLowerCase()));
  return (output?.snapshot?.video?.media_assets ?? []).some((asset) => {
    const path = asset?.source?.path;
    return typeof path === "string" && managedNames.has(basename(path).toLowerCase());
  });
}

export async function prepareAuthoredShowPublication(outputPath) {
  const outputGuard = {};
  const outputPathChecked = await prepareOutputPath(outputPath, { guard: outputGuard });
  return { outputPath: outputPathChecked, outputGuard };
}

export async function writeAuthoredShow(output, outputPath, publication = null, referenceAudioPlan = null) {
  assertAuthoredOutputPreflight(output);
  const prepared = publication ?? await prepareAuthoredShowPublication(outputPath);
  if (!prepared || typeof prepared !== "object" || typeof prepared.outputPath !== "string" || !prepared.outputGuard) {
    fail("generated show write requires a prepared output publication");
  }
  if (normalizeIdentityPath(prepared.outputPath) !== normalizeIdentityPath(outputPath)) {
    fail("generated show write output path differs from its prepared publication");
  }
  if (outputUsesManagedReferenceAudio(output)) {
    if (!referenceAudioPlan?.publication?.outputPath || normalizeIdentityPath(referenceAudioPlan.publication.outputPath) !== normalizeIdentityPath(prepared.outputPath)) {
      fail("reference MediaAssets require their exact prepared sidecar publication plan");
    }
  } else if (referenceAudioPlan !== null) {
    fail("a reference audio publication plan was supplied for output without managed reference MediaAssets");
  }
  const outputText = `${JSON.stringify(output, null, 2)}\n`;
  await writeExclusive(prepared.outputPath, outputText, prepared.outputGuard, {
    fences: referenceAudioPlan ? referenceAudioPublicationFences(referenceAudioPlan) : [],
  });
  parseJsonText(decodeUtf8Strict(Buffer.from(outputText, "utf8"), "generated output"), "generated output");
  return prepared.outputPath;
}

async function main(argv = process.argv.slice(2)) {
  const args = parseCliArgs(argv);
  const basePath = resolve(args.base);
  const manifestPath = resolve(args.manifest);
  const outputPath = resolve(args.output);
  if (normalizeIdentityPath(basePath) === normalizeIdentityPath(outputPath)) fail("base and output must be different paths");
  const baseFile = await readStrictJsonFile(basePath, "base .sdc", MAX_BASE_BYTES);
  verifyApprovedBaseIdentity(baseFile.absolute, baseFile.bytes);
  const manifestFile = await readStrictJsonFile(manifestPath, "manifest", MAX_MANIFEST_BYTES);
  const manifestHash = createHash("sha256").update(manifestFile.bytes).digest("hex").toUpperCase();
  if (manifestHash !== CANONICAL_MANIFEST_SHA256) fail(`manifest SHA-256 ${manifestHash} does not match the canonical DSF2026 export`);
  const publication = await prepareAuthoredShowPublication(outputPath);
  const referenceAudioPlan = args.referenceAudio
    ? await planReferenceAudio(args.referenceAudio, publication)
    : null;
  const output = authorDsf2026Show(baseFile.value, manifestFile.value, {
    referenceAudio: referenceAudioPlan?.descriptors ?? null,
  });
  assertAuthoredOutputPreflight(output);
  if (referenceAudioPlan) await materializeReferenceAudio(referenceAudioPlan);
  const outputPathChecked = await writeAuthoredShow(output, publication.outputPath, publication, referenceAudioPlan);
  console.log(`authored DSF2026 show: ${outputPathChecked}`);
}

if (resolve(process.argv[1] ?? "") === fileURLToPath(import.meta.url)) {
  main().catch((error) => {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 2;
  });
}
