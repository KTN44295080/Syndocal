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

export const CURRENT_MEDIA_SOURCE_PATH = resolve(
  REPO_ROOT,
  "target/qa/DSF2026-show-alpha52-three-display-current-media.sdc",
);
export const CURRENT_MEDIA_SOURCE_BYTE_SIZE = 1_112_316;
export const CURRENT_MEDIA_SOURCE_SHA256 =
  "27484E18DE3FFBB19829D19A90459AB4D847209BF272E629CAE37AF3DD38EA11";

export const CANONICAL_AUTHORITY_PATH = resolve(
  REPO_ROOT,
  "target/qa/dsf2026-show-authored-20260901-canonical/DSF2026-show-alpha10-reference-audio.sdc",
);
export const CANONICAL_AUTHORITY_BYTE_SIZE = 1_092_638;
export const CANONICAL_AUTHORITY_SHA256 =
  "FBA13D2234D493FE9CBBDFB17B479CA79336F00B4AB0CB7CB21EB229E9EF4CF7";

export const OUTPUT_PATH = resolve(
  REPO_ROOT,
  "target/qa/DSF2026-show-alpha53-complete-show.sdc",
);

// Fixed identity produced by the approved exclusive derivation.
export const OUTPUT_BYTE_SIZE = 1_112_332;
export const OUTPUT_SHA256 =
  "72B5120740580A16907B1C1C3A2CD14F995829C6CB837F332F2473501A789C53";

export const EXPECTED_DURATION_MS = 81_153;
export const EXPECTED_TIMELINE_ID = 1;
export const EXPECTED_TIMELINE_LABEL = "人生オーバー";
export const EXPECTED_FOLLOW_DESTINATION_START_MODE = "wait_for_pedal";
export const EXPECTED_EVENT_IDENTITIES = Object.freeze([
  { id: 1, cue_id: 1, time_ms: 138_353, layer_id: 1 },
  { id: 2, cue_id: 2, time_ms: 138_353, layer_id: 1 },
]);

export const ALLOWED_CHANGED_PATHS = Object.freeze([
  "snapshot.timeline.events[0].duration_ms",
  "snapshot.timeline.events[1].duration_ms",
  "snapshot.timeline_bank[0].events[0].duration_ms",
  "snapshot.timeline_bank[0].events[1].duration_ms",
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

function eventIdentity(event, label) {
  if (!event || typeof event !== "object" || Array.isArray(event)) {
    fail(`${label} must be an event object`);
  }
  return {
    id: event.id,
    cue_id: event.cue_id,
    time_ms: event.time_ms,
    layer_id: event.layer_id,
  };
}

function assertExactEventIdentities(events, label, expectedDuration) {
  if (!Array.isArray(events) || events.length !== EXPECTED_EVENT_IDENTITIES.length) {
    fail(`${label} must contain exactly the two approved authored events`);
  }
  for (const [index, expected] of EXPECTED_EVENT_IDENTITIES.entries()) {
    const actual = eventIdentity(events[index], `${label}[${index}]`);
    if (!equalJson(actual, expected)) {
      fail(`${label}[${index}] identity differs from the approved id/cue/time/layer mapping`);
    }
    if (events[index].duration_ms !== expectedDuration) {
      fail(`${label}[${index}].duration_ms must be exactly ${expectedDuration}`);
    }
  }
}

function assertEventOnlyDurationDifference(currentEvents, authorityEvents, label) {
  if (!Array.isArray(currentEvents) || !Array.isArray(authorityEvents)) {
    fail(`${label} must be present in both current and canonical projects`);
  }
  if (currentEvents.length !== authorityEvents.length) {
    fail(`${label} has an ambiguous future event count; refusing to derive`);
  }
  for (const [index, currentEvent] of currentEvents.entries()) {
    const authorityEvent = authorityEvents[index];
    if (!authorityEvent || typeof authorityEvent !== "object") {
      fail(`${label}[${index}] canonical event is missing`);
    }
    const currentKeys = Object.keys(currentEvent).sort();
    const authorityKeys = Object.keys(authorityEvent).sort();
    if (!equalJson(currentKeys, authorityKeys)) {
      fail(`${label}[${index}] event schema differs; future/ambiguous event fields are rejected`);
    }
    for (const key of currentKeys) {
      if (key !== "duration_ms" && !equalJson(currentEvent[key], authorityEvent[key])) {
        fail(`${label}[${index}] has an unsupported difference outside duration_ms: ${key}`);
      }
    }
    if (currentEvent.duration_ms !== 0 || authorityEvent.duration_ms !== EXPECTED_DURATION_MS) {
      fail(`${label}[${index}] requires current duration 0 and canonical duration ${EXPECTED_DURATION_MS}`);
    }
  }
}

function assertCurrentShowShape(project) {
  const snapshot = project?.snapshot;
  const video = snapshot?.video;
  if (!snapshot || !video || !Array.isArray(video.outputs) || !Array.isArray(video.compositions)) {
    fail("current alpha52 project must contain the expected snapshot/video model");
  }
  if (video.outputs.length !== 2 || video.compositions.length !== 3) {
    fail("current alpha52 project must retain exactly two Display outputs and three compositions");
  }
  const displayContract = video.outputs.map((output) => ({
    id: output.id,
    kind: output.kind,
    composition_id: output.composition_id,
    width: output.width,
    height: output.height,
  }));
  const expectedDisplays = [
    { id: 3, kind: "Display", composition_id: 2, width: 1920, height: 1080 },
    { id: 4, kind: "Display", composition_id: 3, width: 3840, height: 2160 },
  ];
  if (!equalJson(displayContract, expectedDisplays)) {
    fail("current Display output contract changed");
  }
  if (!Array.isArray(snapshot.timeline_bank) || snapshot.timeline_bank.length < 2) {
    fail("current project must retain the complete timeline bank");
  }
  if (snapshot.timeline?.id !== EXPECTED_TIMELINE_ID || snapshot.timeline?.label !== EXPECTED_TIMELINE_LABEL) {
    fail("current primary Timeline identity is not the approved 人生オーバー Timeline");
  }
  if (snapshot.timeline?.follow?.destination_start_mode !== EXPECTED_FOLLOW_DESTINATION_START_MODE) {
    fail("current primary Timeline must preserve wait_for_pedal Follow intent");
  }
  const bankIds = snapshot.timeline_bank.map((timeline) => timeline?.id);
  if (bankIds[0] !== 1 || bankIds[1] !== 2 || snapshot.timeline_bank[1]?.label !== "惑う星") {
    fail("current Timeline bank source/destination identity changed");
  }
  assertExactEventIdentities(snapshot.timeline.events, "current snapshot.timeline.events", 0);
  assertExactEventIdentities(snapshot.timeline_bank[0]?.events, "current snapshot.timeline_bank[0].events", 0);
}

function assertCanonicalAuthorityShape(project) {
  const snapshot = project?.snapshot;
  if (!snapshot || !Array.isArray(snapshot.timeline_bank) || snapshot.timeline_bank.length !== 2) {
    fail("canonical alpha10 authority must contain exactly the two authored Timelines");
  }
  if (snapshot.timeline?.id !== EXPECTED_TIMELINE_ID || snapshot.timeline?.label !== EXPECTED_TIMELINE_LABEL) {
    fail("canonical authority primary Timeline identity is ambiguous");
  }
  if (snapshot.timeline?.follow?.destination_start_mode !== EXPECTED_FOLLOW_DESTINATION_START_MODE) {
    fail("canonical authority must preserve wait_for_pedal Follow intent");
  }
  if (snapshot.timeline_bank[0]?.id !== 1 || snapshot.timeline_bank[0]?.label !== EXPECTED_TIMELINE_LABEL) {
    fail("canonical authority source Timeline identity changed");
  }
  if (snapshot.timeline_bank[1]?.id !== 2 || snapshot.timeline_bank[1]?.label !== "惑う星") {
    fail("canonical authority destination Timeline identity changed");
  }
  assertExactEventIdentities(snapshot.timeline.events, "canonical snapshot.timeline.events", EXPECTED_DURATION_MS);
  assertExactEventIdentities(snapshot.timeline_bank[0]?.events, "canonical snapshot.timeline_bank[0].events", EXPECTED_DURATION_MS);
}

function assertCanonicalEventAgreement(currentProject, authorityProject) {
  const currentTimeline = currentProject.snapshot.timeline;
  const currentBankTimeline = currentProject.snapshot.timeline_bank[0];
  const authorityTimeline = authorityProject.snapshot.timeline;
  const authorityBankTimeline = authorityProject.snapshot.timeline_bank[0];
  assertEventOnlyDurationDifference(
    currentTimeline.events,
    authorityTimeline.events,
    "snapshot.timeline.events",
  );
  assertEventOnlyDurationDifference(
    currentBankTimeline.events,
    authorityBankTimeline.events,
    "snapshot.timeline_bank[0].events",
  );
}

export function deriveCompleteShowProject(currentProject, authorityProject) {
  assertCurrentShowShape(currentProject);
  assertCanonicalAuthorityShape(authorityProject);
  assertCanonicalEventAgreement(currentProject, authorityProject);

  const output = structuredClone(currentProject);
  const outputEvents = output.snapshot.timeline.events;
  const outputBankEvents = output.snapshot.timeline_bank[0].events;
  for (const [index, expected] of EXPECTED_EVENT_IDENTITIES.entries()) {
    outputEvents[index].duration_ms = EXPECTED_DURATION_MS;
    outputBankEvents[index].duration_ms = EXPECTED_DURATION_MS;
    if (outputEvents[index].id !== expected.id || outputBankEvents[index].id !== expected.id) {
      fail("derived event identity changed while applying canonical durations");
    }
  }

  const actualChangedPaths = changedPaths(currentProject, output).sort();
  const expectedChangedPaths = [...ALLOWED_CHANGED_PATHS].sort();
  if (!equalJson(actualChangedPaths, expectedChangedPaths)) {
    fail(`complete-show derivation changed unexpected paths: ${actualChangedPaths.join(", ")}`);
  }
  assertExactEventIdentities(output.snapshot.timeline.events, "derived snapshot.timeline.events", EXPECTED_DURATION_MS);
  assertExactEventIdentities(output.snapshot.timeline_bank[0].events, "derived snapshot.timeline_bank[0].events", EXPECTED_DURATION_MS);
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

function assertFixedPath(actual, expected, label) {
  if (normalizeIdentityPath(actual) !== normalizeIdentityPath(expected)) {
    fail(`${label} is fixed to ${expected}`);
  }
}

export async function createCompleteShowProject({
  currentSourcePath = CURRENT_MEDIA_SOURCE_PATH,
  canonicalAuthorityPath = CANONICAL_AUTHORITY_PATH,
  outputPath = OUTPUT_PATH,
} = {}) {
  assertFixedPath(currentSourcePath, CURRENT_MEDIA_SOURCE_PATH, "current alpha52 source path");
  assertFixedPath(canonicalAuthorityPath, CANONICAL_AUTHORITY_PATH, "canonical alpha10 authority path");
  assertFixedPath(outputPath, OUTPUT_PATH, "complete-show output path");
  if (extname(outputPath).toLowerCase() !== ".sdc" || normalizeIdentityPath(dirname(outputPath)) === normalizeIdentityPath(REPO_ROOT)) {
    fail("complete-show output must be the fixed target/qa .sdc child");
  }
  if (normalizeIdentityPath(outputPath) === normalizeIdentityPath(currentSourcePath) || normalizeIdentityPath(outputPath) === normalizeIdentityPath(canonicalAuthorityPath)) {
    fail("complete-show output must not overwrite either approved source");
  }

  const currentSource = await readExactIdentity(
    currentSourcePath,
    "approved alpha52 current-media source",
    CURRENT_MEDIA_SOURCE_BYTE_SIZE,
    CURRENT_MEDIA_SOURCE_SHA256,
  );
  const canonicalAuthority = await readExactIdentity(
    canonicalAuthorityPath,
    "approved alpha10 canonical authority",
    CANONICAL_AUTHORITY_BYTE_SIZE,
    CANONICAL_AUTHORITY_SHA256,
  );
  const currentProject = parseJsonText(
    decodeUtf8Strict(currentSource.bytes, "approved alpha52 current-media source"),
    "approved alpha52 current-media source",
  );
  const authorityProject = parseJsonText(
    decodeUtf8Strict(canonicalAuthority.bytes, "approved alpha10 canonical authority"),
    "approved alpha10 canonical authority",
  );
  const derived = deriveCompleteShowProject(currentProject, authorityProject);
  const outputBytes = Buffer.from(`${JSON.stringify(derived.output, null, 2)}\n`, "utf8");
  if (outputBytes.byteLength !== OUTPUT_BYTE_SIZE || sha256(outputBytes) !== OUTPUT_SHA256) {
    fail("complete-show output identity changed before publication");
  }

  const outputGuard = {};
  const prepared = await prepareOutputPath(outputPath, { guard: outputGuard });
  await readExactIdentity(currentSourcePath, "alpha52 current-media publication fence", CURRENT_MEDIA_SOURCE_BYTE_SIZE, CURRENT_MEDIA_SOURCE_SHA256);
  await readExactIdentity(canonicalAuthorityPath, "alpha10 canonical authority publication fence", CANONICAL_AUTHORITY_BYTE_SIZE, CANONICAL_AUTHORITY_SHA256);
  let published = false;
  try {
    // The current-media source is a sibling of the output, so the native
    // exclusive writer can hold it as a same-parent publication fence. The
    // canonical authority is in a sibling folder and is re-read at each
    // post-publication fence below.
    await writeExclusiveBytes(prepared, outputBytes, outputGuard, {
      fences: [{
        path: currentSourcePath,
        byteSize: CURRENT_MEDIA_SOURCE_BYTE_SIZE,
        sha256: CURRENT_MEDIA_SOURCE_SHA256,
      }],
    });
    published = true;
    const written = await readRegularFileBytesNoReparse(
      prepared,
      "created alpha53 complete-show project",
      MAX_BASE_BYTES,
    );
    if (!Buffer.from(written.bytes).equals(outputBytes)) {
      fail(`created complete-show output failed byte-for-byte verification; quarantine ${prepared}`);
    }
    await readExactIdentity(currentSourcePath, "alpha52 current-media post-publication fence", CURRENT_MEDIA_SOURCE_BYTE_SIZE, CURRENT_MEDIA_SOURCE_SHA256);
    await readExactIdentity(canonicalAuthorityPath, "alpha10 canonical authority post-publication fence", CANONICAL_AUTHORITY_BYTE_SIZE, CANONICAL_AUTHORITY_SHA256);
    const finalWritten = await readRegularFileBytesNoReparse(
      prepared,
      "final alpha53 complete-show project",
      MAX_BASE_BYTES,
    );
    if (!Buffer.from(finalWritten.bytes).equals(outputBytes)) {
      fail(`created complete-show output changed after post-publication fences; quarantine ${prepared}`);
    }
    const reparsed = parseJsonText(
      decodeUtf8Strict(finalWritten.bytes, "created alpha53 complete-show project"),
      "created alpha53 complete-show project",
    );
    if (!equalJson(reparsed, derived.output)) fail("created complete-show project differs from the exact derivation");
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
    status: "PASS",
    currentSourcePath,
    currentSourceByteSize: CURRENT_MEDIA_SOURCE_BYTE_SIZE,
    currentSourceSha256: CURRENT_MEDIA_SOURCE_SHA256,
    canonicalAuthorityPath,
    canonicalAuthorityByteSize: CANONICAL_AUTHORITY_BYTE_SIZE,
    canonicalAuthoritySha256: CANONICAL_AUTHORITY_SHA256,
    outputPath: prepared,
    outputByteSize: OUTPUT_BYTE_SIZE,
    outputSha256: OUTPUT_SHA256,
    changedPaths: derived.changedPaths,
    routesPreserved: true,
    physicalOutputOpened: false,
  };
}

export async function verifyCompleteShowProject() {
  const currentSource = await readExactIdentity(
    CURRENT_MEDIA_SOURCE_PATH,
    "approved alpha52 current-media source",
    CURRENT_MEDIA_SOURCE_BYTE_SIZE,
    CURRENT_MEDIA_SOURCE_SHA256,
  );
  const canonicalAuthority = await readExactIdentity(
    CANONICAL_AUTHORITY_PATH,
    "approved alpha10 canonical authority",
    CANONICAL_AUTHORITY_BYTE_SIZE,
    CANONICAL_AUTHORITY_SHA256,
  );
  const output = await readExactIdentity(
    OUTPUT_PATH,
    "alpha53 complete-show output",
    OUTPUT_BYTE_SIZE,
    OUTPUT_SHA256,
  );
  const currentProject = parseJsonText(
    decodeUtf8Strict(currentSource.bytes, "approved alpha52 current-media source"),
    "approved alpha52 current-media source",
  );
  const authorityProject = parseJsonText(
    decodeUtf8Strict(canonicalAuthority.bytes, "approved alpha10 canonical authority"),
    "approved alpha10 canonical authority",
  );
  const outputProject = parseJsonText(
    decodeUtf8Strict(output.bytes, "alpha53 complete-show output"),
    "alpha53 complete-show output",
  );
  const derived = deriveCompleteShowProject(currentProject, authorityProject);
  if (!equalJson(outputProject, derived.output)) fail("alpha53 complete-show output differs from the exact derivation");
  return {
    status: "PASS",
    currentSourcePath: CURRENT_MEDIA_SOURCE_PATH,
    currentSourceByteSize: CURRENT_MEDIA_SOURCE_BYTE_SIZE,
    currentSourceSha256: CURRENT_MEDIA_SOURCE_SHA256,
    canonicalAuthorityPath: CANONICAL_AUTHORITY_PATH,
    canonicalAuthorityByteSize: CANONICAL_AUTHORITY_BYTE_SIZE,
    canonicalAuthoritySha256: CANONICAL_AUTHORITY_SHA256,
    outputPath: OUTPUT_PATH,
    outputByteSize: OUTPUT_BYTE_SIZE,
    outputSha256: OUTPUT_SHA256,
    changedPaths: derived.changedPaths,
  };
}

function parseCliArgs(argv) {
  if (argv.length === 0) return { verifyOnly: false };
  if (argv.length === 1 && argv[0] === "--verify-only") return { verifyOnly: true };
  fail("usage: node qa/harnesses/derive-dsf2026-alpha53-complete-show.mjs [--verify-only]");
}

async function main() {
  const options = parseCliArgs(process.argv.slice(2));
  console.log(JSON.stringify(options.verifyOnly ? await verifyCompleteShowProject() : await createCompleteShowProject(), null, 2));
}

const invokedDirectly = process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href;
if (invokedDirectly) {
  main().catch((error) => {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  });
}
