import { createHash } from "node:crypto";
import { spawnSync } from "node:child_process";
import { dirname, extname, join, resolve } from "node:path";

import { fail } from "./common.mjs";
import {
  normalizeIdentityPath,
  readRegularFileBytesNoReparse,
  writeExclusiveBytes,
} from "./io.mjs";

/**
 * Reference audio is an authoring-time convenience, not a show transport.
 * The generated files are deterministic ASCII sidecars in the candidate's
 * already-verified parent. MediaAsset file paths are deliberately absolute:
 * another PC must explicitly relink and reverify the matching content rather
 * than treating a copied project directory as a portable transport package.
 * Supplied originals are opened read-only and are never opened for writing.
 */
export const MAX_REFERENCE_AUDIO_BYTES = 64 * 1024 * 1024;
export const REFERENCE_AUDIO_SIDECAR_NAMES = Object.freeze({
  jinseiOverPath: "dsf2026-reference-jinsei-over.mp3",
  madowHoshiPath: "dsf2026-reference-madow-hoshi.mp3",
});

const REFERENCE_AUDIO_INPUTS = Object.freeze([
  Object.freeze({
    key: "jinseiOverPath",
    timelineId: 1,
    managedName: REFERENCE_AUDIO_SIDECAR_NAMES.jinseiOverPath,
    label: "Jinsei Over Reference Audio",
  }),
  Object.freeze({
    key: "madowHoshiPath",
    timelineId: 2,
    managedName: REFERENCE_AUDIO_SIDECAR_NAMES.madowHoshiPath,
    label: "Madow Hoshi Reference Audio",
  }),
]);

function assertMp3Path(path, label) {
  if (extname(path).toLowerCase() !== ".mp3") {
    fail(`${label} must be an MP3 file; reference authoring does not transcode inputs`);
  }
}

function sha256(bytes) {
  return createHash("sha256").update(bytes).digest("hex");
}

function measureDurationMs(bytes, label) {
  const result = spawnSync(
    process.platform === "win32" ? "ffprobe.exe" : "ffprobe",
    [
      "-v",
      "error",
      "-show_entries",
      "packet=pts_time,duration_time",
      "-of",
      "json",
      "-show_packets",
      "-select_streams",
      "a:0",
      "-i",
      "pipe:0",
    ],
    {
      input: bytes,
      encoding: "utf8",
      windowsHide: true,
      maxBuffer: MAX_REFERENCE_AUDIO_BYTES,
    },
  );
  if (result.error || result.status !== 0) {
    const detail = result.error?.message ?? result.stderr?.trim() ?? `exit ${result.status}`;
    fail(`${label} duration could not be measured by ffprobe: ${detail}`);
  }
  let packets;
  try {
    packets = JSON.parse(result.stdout).packets;
  } catch (error) {
    fail(`${label} duration packet report from ffprobe is not valid JSON: ${String(error?.message ?? error)}`);
  }
  if (!Array.isArray(packets) || packets.length === 0) {
    fail(`${label} duration could not be measured from audio packets`);
  }
  const seconds = packets.reduce((maximum, packet) => {
    const start = Number(packet?.pts_time);
    const duration = Number(packet?.duration_time);
    const end = start + duration;
    return Number.isFinite(end) ? Math.max(maximum, end) : maximum;
  }, Number.NEGATIVE_INFINITY);
  const durationMs = Math.round(seconds * 1_000);
  if (!Number.isFinite(seconds) || seconds <= 0 || !Number.isSafeInteger(durationMs) || durationMs <= 0) {
    fail(`${label} duration is not a finite positive ffprobe measurement`);
  }
  return durationMs;
}

async function readReferenceInput(path, label, testHooks = null) {
  assertMp3Path(path, label);
  let read;
  try {
    read = await readRegularFileBytesNoReparse(path, label, MAX_REFERENCE_AUDIO_BYTES, {
      beforeNativeOpen: testHooks?.beforeNativeOpen
        ? (context) => testHooks.beforeNativeOpen({ ...context, label })
        : undefined,
    });
    const bytes = read.bytes;
    if (bytes.byteLength === 0) fail(`${label} must not be empty`);
    await testHooks?.afterRead?.({ path: read.absolute, label, bytes: Buffer.from(bytes) });
    if (testHooks?.afterRead) {
      const reread = await readRegularFileBytesNoReparse(read.absolute, label, MAX_REFERENCE_AUDIO_BYTES);
      if (!reread.bytes.equals(bytes)) fail(`${label} changed while it was being read; refusing an unstable reference`);
    }
  } catch (error) {
    if (error?.message?.includes("refusing an unstable reference") || error?.message?.includes("must not be empty") || error?.message?.includes("bounded size")) throw error;
    fail(`${label} cannot be read: ${String(error?.message ?? error)}`);
  }
  return {
    durationMs: measureDurationMs(read.bytes, label),
    byteSize: read.bytes.byteLength,
    sha256: sha256(read.bytes),
    bytes: read.bytes,
  };
}

function assertPublication(publication) {
  if (!publication || typeof publication !== "object" || typeof publication.outputPath !== "string" || !publication.outputGuard) {
    fail("reference audio requires the prepared candidate output and its parent identity guard");
  }
  return publication;
}

async function checkedSidecarPath(publication, spec) {
  const checkedPublication = assertPublication(publication);
  const outputPath = resolve(checkedPublication.outputPath);
  const outputParent = dirname(outputPath);
  if (normalizeIdentityPath(outputParent) !== normalizeIdentityPath(checkedPublication.outputGuard.parentPath)) {
    fail("reference audio candidate parent differs from its guarded publication");
  }
  // Do not inspect this leaf by path.  Existing sidecars are opened only by
  // readRegularFileBytesNoReparse under the held candidate parent; absent
  // sidecars are created by the same native parent-handle primitive.
  return join(checkedPublication.outputGuard.parentPath, spec.managedName);
}

async function readExistingSidecar(destination, input, publication) {
  const existing = await readRegularFileBytesNoReparse(destination, "reference audio sidecar", MAX_REFERENCE_AUDIO_BYTES, {
    guard: assertPublication(publication).outputGuard,
    allowMissing: true,
  });
  if (existing === null) return false;
  if (existing.bytes.byteLength !== input.byteSize || sha256(existing.bytes) !== input.sha256) {
    fail(`reference audio sidecar already exists with different bytes: ${destination}`);
  }
  return true;
}

/**
 * Read and copy both explicit reference inputs for a generated candidate.
 * The returned descriptor is intentionally plain data so `authorDsf2026Show`
 * remains pure and deterministic once the files have been measured.
 */
export async function planReferenceAudio(options, publication, testHooks = null) {
  if (options == null) return null;
  if (typeof options !== "object") fail("reference audio options must be an object");
  assertPublication(publication);
  const requested = REFERENCE_AUDIO_INPUTS.map((spec) => {
    const path = options[spec.key];
    if (typeof path !== "string" || path.trim().length === 0) {
      fail(`both reference audio inputs are required; missing ${spec.key}`);
    }
    return { spec, path: path.trim() };
  });
  const inputs = await Promise.all(requested.map(({ spec, path }) => readReferenceInput(path, spec.label, testHooks)));
  const descriptors = {};
  const entries = [];
  for (const [{ spec }, input] of requested.map((item, index) => [item, inputs[index]])) {
    const managedPath = await checkedSidecarPath(publication, spec);
    descriptors[spec.key] = {
      path: managedPath,
      durationMs: input.durationMs,
      byteSize: input.byteSize,
      sha256: input.sha256,
    };
    entries.push({ spec, input, destination: managedPath });
  }
  return { publication, descriptors, entries };
}

/**
 * Publish only the durable sidecars planned for a preflight-valid candidate.
 * There is intentionally no deletion on a later .sdc write failure: each
 * deterministic filename is a byte-identity stage. Retrying with the exact
 * inputs reuses it; divergent bytes fail closed; no incomplete .sdc is ever
 * published by this routine.
 */
export async function materializeReferenceAudio(plan) {
  if (!plan || typeof plan !== "object" || !Array.isArray(plan.entries) || !plan.descriptors) {
    fail("reference audio materialization requires a prepared plan");
  }
  assertPublication(plan.publication);
  for (const { spec, input, destination } of plan.entries) {
    const checked = await checkedSidecarPath(plan.publication, spec);
    if (checked !== destination) fail("reference audio sidecar path changed before materialization");
    if (!await readExistingSidecar(checked, input, plan.publication)) {
      await writeExclusiveBytes(checked, input.bytes, plan.publication.outputGuard);
      if (!await readExistingSidecar(checked, input, plan.publication)) {
        fail(`reference audio sidecar disappeared after creation: ${checked}`);
      }
    }
  }
  return plan.descriptors;
}

/**
 * The .sdc writer passes these exact byte identities into the same held-parent
 * native publication call.  Each sidecar remains open without write/delete
 * sharing until the candidate .sdc has been flushed, so an equal-hash reparse
 * or leaf replacement cannot race the completed project into existence.
 */
export function referenceAudioPublicationFences(plan) {
  if (!plan || typeof plan !== "object" || !Array.isArray(plan.entries)) {
    fail("reference audio publication fences require a prepared plan");
  }
  return plan.entries.map(({ input, destination }) => ({
    path: destination,
    byteSize: input.byteSize,
    sha256: input.sha256,
  }));
}

export async function prepareReferenceAudio(options, publication) {
  const plan = await planReferenceAudio(options, publication);
  return plan ? materializeReferenceAudio(plan) : null;
}

/** Test-only deterministic source-replacement seam; production callers use planReferenceAudio. */
export async function planReferenceAudioForTest(options, publication, testHooks) {
  return planReferenceAudio(options, publication, testHooks);
}

export function referenceAudioInputKeys() {
  return REFERENCE_AUDIO_INPUTS.map(({ key }) => key);
}
