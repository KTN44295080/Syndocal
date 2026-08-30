import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { access, link, mkdir, mkdtemp, readFile, readdir, rename, rm, symlink, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { basename, join } from "node:path";
import { spawn, spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

import {
  CANONICAL_MANIFEST_PATH,
  CANONICAL_MANIFEST_SHA256,
  APPROVED_BASE_FILENAME,
  APPROVED_BASE_SHA256,
  authorDsf2026Show,
  materializeReferenceAudio,
  planReferenceAudio,
  preflightShowContract,
  prepareAuthoredShowPublication,
  prepareOutputPath,
  validateGeneratedTimelineLayerReferences,
  validateBaseProject,
  validateCanonicalManifest,
  validateDmxOutputStaging,
  writeAuthoredShow,
} from "../../tools/author-dsf2026-show.mjs";
import { writeExclusive } from "../../tools/dsf2026/io.mjs";
import {
  planReferenceAudioForTest,
  REFERENCE_AUDIO_SIDECAR_NAMES,
} from "../../tools/dsf2026/reference-audio.mjs";

const toolPath = new URL("../../tools/author-dsf2026-show.mjs", import.meta.url);
const ioPath = fileURLToPath(new URL("../../tools/dsf2026/io.mjs", import.meta.url));
const safeWriteHelperPath = fileURLToPath(new URL("../../tools/dsf2026/safe-write-win32.ps1", import.meta.url));
const TEST_POWERSHELL_PATH = "C:\\Windows\\System32\\WindowsPowerShell\\v1.0\\powershell.exe";
const REPO_ROOT = fileURLToPath(new URL("../../", import.meta.url));
const CONTENT_ARTIFACT_PATH = join(REPO_ROOT, "target", "qa", "dsf2026-show-authored-20260828", "DSF2026-show-alpha9-reference-audio.sdc");
const CONTENT_ARTIFACT_BYTES = 1_095_864;
const CONTENT_ARTIFACT_SHA256 = "93E71D8AC3889968C2AAD5B0A8CA194B88CB1C7B51BF897C7741C969D9A05094";
const REQUIRE_CONTENT_ARTIFACT_FLAG = "--require-content-artifact";
const TEST_ONLY_MISSING_CONTENT_ARTIFACT = "--test-only-missing-content-artifact";
const RETIRED_ARTIFACT_FLAGS = ["--require-final-artifact", "--test-only-missing-final-artifact"];
const TEST_ARGS = process.argv.slice(2);
const UNKNOWN_TEST_ARGS = TEST_ARGS.filter((arg) => ![REQUIRE_CONTENT_ARTIFACT_FLAG, TEST_ONLY_MISSING_CONTENT_ARTIFACT].includes(arg));
assert.deepEqual(UNKNOWN_TEST_ARGS, [], "unknown author-dsf2026-show test arguments must fail closed");
const RUN_TEST_ONLY_MISSING_CONTENT_ARTIFACT = TEST_ARGS.includes(TEST_ONLY_MISSING_CONTENT_ARTIFACT);
const EXPLICIT_REQUIRE_CONTENT_ARTIFACT = TEST_ARGS.includes(REQUIRE_CONTENT_ARTIFACT_FLAG);
assert.ok(!(RUN_TEST_ONLY_MISSING_CONTENT_ARTIFACT && EXPLICIT_REQUIRE_CONTENT_ARTIFACT), "test-only missing-artifact behavior cannot be combined with the pinned authored content gate");
const REQUIRE_CONTENT_ARTIFACT = EXPLICIT_REQUIRE_CONTENT_ARTIFACT || !RUN_TEST_ONLY_MISSING_CONTENT_ARTIFACT;

function emptyTimeline() {
  return {
    id: 1,
    label: "Timeline 1",
    layers: [],
    events: [],
    automations: [],
    video_automations: [],
    audio: null,
    audio_clips: [],
    video_clips: [],
    phases: [],
    item_groups: [],
    loop_region: null,
    follow: null,
    guide_enabled: false,
    audio_offset_ms: 0,
    audio_muted: false,
    metronome_enabled: false,
    count_in_beats: 4,
    tempo_meter_map_version: 1,
    tempo_meter_map: [],
    playing: false,
    position_ms: 0,
    duration_ms: 0,
  };
}

function makeBase() {
  const timeline = emptyTimeline();
  return {
    version: 1,
    app: "Syndocal",
    custom_profiles: [{ source_path: "memory://fixture/keep", name: "Keep Profile" }],
    fixture_groups: [{ id: "keep", label: "Keep Group", color: "#123456" }],
    midi_mappings: [{ id: "midi-keep", action: "keep" }],
    osc_mappings: [{ id: "osc-keep", address: "/keep" }],
    dmx_mappings: [{ id: "dmx-keep", universe: 0, address: 1 }],
    dj_track_triggers: [],
    snapshot: {
      fixtures: [{ id: 7, label: "Keep Fixture" }],
      cue_lists: [{ id: 1, label: "color" }, { id: 2, label: "dimmer" }],
      cues: [
        { id: 1, label: "all_white", cue_list_id: 1, group_id: "color", recall_mode: "Coexist" },
        { id: 2, label: "all_max", cue_list_id: 2, group_id: "dimmer", recall_mode: "Coexist" },
        { id: 8, label: "Keep Cue" },
      ],
      active_cue_id: null,
      output: {
        enabled: false,
        port: 6454,
        protocol: "ArtNet",
        serial_baud_rate: 57600,
        serial_port: "",
        target_ip: "192.0.2.7",
        universe: 0,
      },
      dmx_outputs: [{
        enabled: false,
        port: 6454,
        protocol: "ArtNet",
        serial_baud_rate: 57600,
        serial_port: "",
        target_ip: "192.0.2.7",
        universe: 0,
      }],
      video: { layers: [{ id: 9, label: "Keep Layer" }] },
      timeline,
      timeline_bank: [structuredClone(timeline)],
    },
  };
}

function assertThrows(action, pattern, label) {
  assert.throws(action, (error) => {
    assert.match(String(error?.message ?? error), pattern, `${label} must explain the failure`);
    return true;
  }, label);
}

async function withEnvironment(values, action) {
  const previous = new Map(Object.keys(values).map((key) => [key, process.env[key]]));
  try {
    for (const [key, value] of Object.entries(values)) {
      if (value === undefined) delete process.env[key];
      else process.env[key] = value;
    }
    return await action();
  } finally {
    for (const [key, value] of previous) {
      if (value === undefined) delete process.env[key];
      else process.env[key] = value;
    }
  }
}

function collectChild(child) {
  let stdout = "";
  let stderr = "";
  child.stdout?.on("data", (chunk) => { stdout += chunk.toString(); });
  child.stderr?.on("data", (chunk) => { stderr += chunk.toString(); });
  return new Promise((resolve, reject) => {
    child.once("error", reject);
    child.once("close", (code, signal) => resolve({ code, signal, stdout, stderr }));
  });
}

async function waitForPath(path, timeoutMs = 10_000) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    try {
      await access(path);
      return;
    } catch {
      await new Promise((resolve) => setTimeout(resolve, 25));
    }
  }
  throw new Error(`timed out waiting for ${path}`);
}

const RACE_MOVER_SOURCE = String.raw`param(
  [Parameter(Mandatory = $true)] [string] $SourcePath,
  [Parameter(Mandatory = $true)] [string] $DestinationPath,
  [Parameter(Mandatory = $true)] [string] $TriggerPath,
  [Parameter(Mandatory = $true)] [string] $ReadyPath,
  [Parameter(Mandatory = $true)] [int] $TimeoutMs
)

$source = @'
using System;
using System.Runtime.InteropServices;

public static class SyndocalDsf2026RaceMove
{
    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern bool MoveFileEx(string existingFileName, string newFileName, uint flags);

    public static bool TryMove(string source, string destination)
    {
        return MoveFileEx(source, destination, 0);
    }
}
'@

try {
  Add-Type -TypeDefinition $source -Language CSharp -ErrorAction Stop
  [System.IO.File]::WriteAllText($ReadyPath, "ready")
  $deadline = (Get-Date).AddMilliseconds($TimeoutMs)
  while ((Get-Date) -lt $deadline) {
    if (Test-Path -LiteralPath $TriggerPath -PathType Any) {
      if ([SyndocalDsf2026RaceMove]::TryMove($SourcePath, $DestinationPath)) {
        [Console]::Out.WriteLine("MOVED")
        exit 3
      }
    }
    Start-Sleep -Milliseconds 1
  }
  [Console]::Out.WriteLine("DENIED")
  exit 0
} catch {
  [Console]::Error.WriteLine($_.Exception.Message)
  exit 2
}`;

function startPowerShell(scriptPath, args, input) {
  const child = spawn(TEST_POWERSHELL_PATH, [
    "-NoProfile",
    "-NonInteractive",
    "-ExecutionPolicy",
    "Bypass",
    "-File",
    scriptPath,
    ...args,
  ], { windowsHide: true, stdio: ["pipe", "pipe", "pipe"] });
  child.stdin.end(Buffer.from(input, "utf8"));
  return { child, result: collectChild(child) };
}

function helperArguments(scriptPath, parentPath, leafName, guard) {
  return [
    "-ParentPath",
    parentPath,
    "-LeafName",
    leafName,
    "-ExpectedVolumeSerial",
    guard.parentVolumeSerial,
    "-ExpectedFileIndex",
    guard.parentFileIndex,
  ];
}

function parseUtf8Json(bytes) {
  return JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(bytes));
}

function sha256(bytes) {
  return createHash("sha256").update(bytes).digest("hex").toUpperCase();
}

function wavBytes(durationMs, sampleRate = 8_000) {
  const frames = Math.max(1, Math.floor((durationMs * sampleRate) / 1_000));
  const bytesPerSample = 2;
  const dataBytes = frames * bytesPerSample;
  const bytes = Buffer.alloc(44 + dataBytes);
  bytes.write("RIFF", 0, "ascii");
  bytes.writeUInt32LE(36 + dataBytes, 4);
  bytes.write("WAVEfmt ", 8, "ascii");
  bytes.writeUInt32LE(16, 16);
  bytes.writeUInt16LE(1, 20);
  bytes.writeUInt16LE(1, 22);
  bytes.writeUInt32LE(sampleRate, 24);
  bytes.writeUInt32LE(sampleRate * bytesPerSample, 28);
  bytes.writeUInt16LE(bytesPerSample, 32);
  bytes.writeUInt16LE(16, 34);
  bytes.write("data", 36, "ascii");
  bytes.writeUInt32LE(dataBytes, 40);
  return bytes;
}

function referenceSidecarPaths(parent) {
  return Object.values(REFERENCE_AUDIO_SIDECAR_NAMES).map((name) => join(parent, name));
}

function checkPass(report, label) {
  assert.equal(report.status, "PASS", `${label} preflight must PASS`);
  assert.ok(report.checks.length > 0, `${label} preflight must report checks`);
  assert.ok(report.checks.every((check) => check.status === "PASS"), `${label} every preflight check must PASS`);
}

function assertGeneratedTimelineLayerReferences(timeline, label) {
  assert.doesNotThrow(
    () => validateGeneratedTimelineLayerReferences(timeline, label),
    `${label} must satisfy the native typed timeline-lane contract`,
  );
  const layersById = new Map((timeline.layers ?? []).map((layer) => [layer.id, layer]));
  for (const event of timeline.events ?? []) {
    assert.ok(Number.isSafeInteger(event.layer_id) && event.layer_id > 0, `${label} event ${event.id} must have an explicit positive layer_id`);
    assert.equal(layersById.get(event.layer_id)?.kind, event.track, `${label} event ${event.id} must target a compatible ${event.track} layer`);
  }
  for (const clip of timeline.audio_clips ?? []) {
    assert.equal(layersById.get(clip.layer_id)?.kind, "Audio", `${label} audio clip ${clip.id} must target an Audio layer`);
  }
}

function assertCanonicalTimelineLayers(timeline, expected, label) {
  assert.deepEqual(
    (timeline.layers ?? []).map(({ id, kind, order }) => ({ id, kind, order })),
    expected,
    `${label} layers must use native display_section_rank order Audio -> Lighting -> Video with contiguous order values`,
  );
  assert.deepEqual(
    (timeline.layers ?? []).map(({ order }) => order),
    (timeline.layers ?? []).map((_, index) => index),
    `${label} layer order must be contiguous from zero`,
  );
}

function assertStageLayoutShape(fixtures, expectedCount, expectedCells, expectedSegments, label) {
  assert.equal(fixtures.length, expectedCount, `${label} fixture count must be exact`);
  for (const [index, fixture] of fixtures.entries()) {
    const layout = fixture.stage_layout;
    assert.ok(layout && typeof layout === "object", `${label}[${index}] must have a stage_layout`);
    assert.ok(Array.isArray(layout.cells), `${label}[${index}] stage_layout.cells must be an array`);
    assert.ok(Array.isArray(layout.logical_segments), `${label}[${index}] stage_layout.logical_segments must be an array`);
    assert.equal(layout.cells.length, expectedCells, `${label}[${index}] must have exactly ${expectedCells} cells`);
    assert.equal(layout.logical_segments.length, expectedSegments, `${label}[${index}] must have exactly ${expectedSegments} logical segments`);
  }
}

function assertApprovedFixtureStageLayouts(project, label) {
  const fixtures = project?.snapshot?.fixtures;
  assert.ok(Array.isArray(fixtures), `${label} must contain a fixture array`);
  assert.equal(fixtures.length, 46, `${label} must contain exactly 46 fixtures`);
  assert.equal(fixtures.filter(({ stage_layout }) => stage_layout !== undefined).length, 9, `${label} must retain exactly all 9 imported stage layouts`);
  const megaBars = fixtures.filter(({ profile_name }) => profile_name === "Mega Bar RGBA");
  const strongpoints = fixtures.filter(({ profile_name }) => profile_name === "960 sound waves strongpoint");
  assertStageLayoutShape(
    megaBars,
    6,
    8,
    8,
    `${label} Mega Bar layouts`,
  );
  assertStageLayoutShape(
    strongpoints,
    3,
    4,
    4,
    `${label} Strongpoint layouts`,
  );
}

function assertContentArtifactInvariants(project, label) {
  assert.deepEqual(project.dj_track_triggers, [{
    id: "jinsei-over-production",
    selector: {
      artist: null,
      contentId: null,
      fallbackDeck: 1,
      title: null,
      titleContains: "人生オーバー",
    },
    timelineId: 1,
    retrigger: "once_per_play_session",
  }], `${label} authored title selector must be exact`);

  const snapshot = project?.snapshot;
  assert.ok(snapshot && typeof snapshot === "object", `${label} must contain a snapshot`);
  const source = snapshot.timeline;
  const destination = snapshot.timeline_bank?.find(({ id }) => id === 2);
  assert.ok(source && typeof source === "object", `${label} must contain Timeline 1`);
  assert.equal(snapshot.timeline_bank.length, 2, `${label} must contain exactly two Timelines`);
  assert.deepEqual(snapshot.timeline, snapshot.timeline_bank[0], `${label} active Timeline must equal bank Timeline 1`);
  assert.equal(source.id, 1, `${label} source Timeline ID must be 1`);
  assert.equal(source.label, "人生オーバー", `${label} source Timeline label must be exact`);
  assertGeneratedTimelineLayerReferences(source, `${label} source Timeline`);
  assertCanonicalTimelineLayers(source, [
    { id: 2, kind: "Audio", order: 0 },
    { id: 1, kind: "Lighting", order: 1 },
  ], `${label} source Timeline`);
  assert.deepEqual(source.loop_region, {
    a_ms: 136_941,
    b_ms: 138_353,
    enabled: true,
    musical_length_beats: 4,
  }, `${label} Timeline 1 must retain the indefinite four-beat loop [136941, 138353)`);
  assert.deepEqual(source.follow, {
    enabled: true,
    next_timeline_id: 2,
    duration: { unit: "Bars", value_milliunits: 1000 },
    curve: "Linear",
    video_kind: "Crossfade",
    lighting_policy: "hold_then_cut",
    destination_bpm: null,
    preroll_ms: 0,
    trans_cadence_bars: 2,
    trans_target_measures: [149, 151, 153, 155],
    hold_first_destination_measure: true,
    fault_policy: "hold",
  }, `${label} Follow must be one bar with first destination-measure hold`);
  assert.deepEqual(source.events.map(({ id, cue_id, time_ms, track, layer_id, duration_ms }) => ({ id, cue_id, time_ms, track, layer_id, duration_ms })), [
    { id: 1, cue_id: 1, time_ms: 138_353, track: "Lighting", layer_id: 1, duration_ms: 0 },
    { id: 2, cue_id: 2, time_ms: 138_353, track: "Lighting", layer_id: 1, duration_ms: 0 },
  ], `${label} all_white/all_max lighting events must both be at 138353 ms`);
  const cueLabels = new Map((snapshot.cues ?? []).map(({ id, label: cueLabel }) => [id, cueLabel]));
  assert.deepEqual(source.events.map(({ cue_id }) => cueLabels.get(cue_id)), ["all_white", "all_max"], `${label} lighting events must target all_white then all_max`);
  assert.ok(destination && typeof destination === "object", `${label} must contain destination Timeline 2`);
  assertGeneratedTimelineLayerReferences(destination, `${label} destination Timeline`);
  assertCanonicalTimelineLayers(destination, [
    { id: 3, kind: "Audio", order: 0 },
  ], `${label} destination Timeline`);
  assert.equal(destination.label, "惑う星", `${label} destination Timeline label must be exact`);
  assert.equal(Object.hasOwn(destination, "loop_region"), false, `${label} destination Timeline must not carry the source loop`);
  assert.equal(Object.hasOwn(destination, "follow"), false, `${label} destination Timeline must not carry a second Follow`);
  assert.deepEqual(
    (snapshot.video?.media_assets ?? []).map(({ id, source, byte_size, content_hash }) => ({
      id,
      sidecar: basename(source?.path ?? ""),
      duration_ms: source?.metadata?.duration_ms,
      byte_size,
      sha256: content_hash?.hex,
    })),
    [
      {
        id: 1,
        sidecar: REFERENCE_AUDIO_SIDECAR_NAMES.jinseiOverPath,
        duration_ms: 214_032,
        byte_size: 8_561_392,
        sha256: "de7a0e78d269643035823df57ad5dd98fe1294583f2a072b91aea9c66a66c7ca",
      },
      {
        id: 2,
        sidecar: REFERENCE_AUDIO_SIDECAR_NAMES.madowHoshiPath,
        duration_ms: 273_432,
        byte_size: 9_738_524,
        sha256: "e82ae9e8641ccd798fc5e90dc40603286d6965d9e726989f018871c2446c3c3b",
      },
    ],
    `${label} must retain the two exact managed reference MediaAssets`,
  );
  assert.deepEqual(
    [source, destination].map((timeline) => timeline.audio_clips.map(({ media_asset_id, path }) => ({ media_asset_id, path }))),
    [[{ media_asset_id: 1, path: "" }], [{ media_asset_id: 2, path: "" }]],
    `${label} reference assets must remain ordinary Media Library-backed Timeline Audio Clips`,
  );

  for (const [routeLabel, route] of [
    ["snapshot.output", snapshot.output],
    ["snapshot.dmx_outputs[0]", snapshot.dmx_outputs?.[0]],
  ]) {
    assert.deepEqual({
      enabled: route?.enabled,
      protocol: route?.protocol,
      serial_port: route?.serial_port,
      serial_baud_rate: route?.serial_baud_rate,
    }, {
      enabled: false,
      protocol: "EnttecOpenDmx",
      serial_port: "",
      serial_baud_rate: 250_000,
    }, `${label} ${routeLabel} must be staged disabled on EnttecOpenDmx at 250000 with an empty machine-local serial_port`);
  }
  assert.equal(snapshot.dmx_outputs.length, 1, `${label} must contain one staged DMX output`);
  assertApprovedFixtureStageLayouts(project, label);
}

async function checkOptionalContentArtifactForTest(path, required) {
  let bytes;
  try {
    bytes = await readFile(path);
  } catch (error) {
    if (error?.code !== "ENOENT") throw error;
    if (required) assert.fail(`required pinned authored content artifact is missing: ${path}`);
    console.log(`test-only missing-artifact behavior: optional pinned authored content artifact is intentionally absent: ${path}`);
    return false;
  }
  assert.equal(bytes.length, CONTENT_ARTIFACT_BYTES, "pinned authored content artifact must retain its exact byte length");
  assert.equal(sha256(bytes), CONTENT_ARTIFACT_SHA256, "pinned authored content artifact must retain its exact SHA-256");
  const project = parseUtf8Json(bytes);
  checkPass(preflightShowContract(project), "pinned authored content artifact");
  assertContentArtifactInvariants(project, "pinned authored content artifact");
  return true;
}

async function runPinnedContentArtifactGate(path) {
  await checkOptionalContentArtifactForTest(path, true);
  console.log(`PINNED AUTHORED CONTENT-ARTIFACT GATE passed: ${path} (${CONTENT_ARTIFACT_BYTES} bytes, SHA-256 ${CONTENT_ARTIFACT_SHA256})`);
}

async function runTestOnlyMissingContentArtifactBehavior() {
  const work = await mkdtemp(join(tmpdir(), "syndocal-author-dsf2026-missing-artifact-test-"));
  try {
    assert.equal(
      await checkOptionalContentArtifactForTest(join(work, "missing-content.sdc"), false),
      false,
      "an absent optional pinned authored content artifact must be an intentional test-only skip",
    );
    await assert.rejects(
      () => runPinnedContentArtifactGate(join(work, "missing-required-content.sdc")),
      /required pinned authored content artifact is missing/i,
      "the pinned authored content-artifact gate must fail clearly when its artifact is absent",
    );
  } finally {
    await rm(work, { recursive: true, force: true });
  }
}

function assertRetiredArtifactFlagsFailClosed() {
  for (const flag of RETIRED_ARTIFACT_FLAGS) {
    const rejected = spawnSync(process.execPath, [fileURLToPath(import.meta.url), flag], { encoding: "utf8" });
    assert.notEqual(rejected.status, 0, `${flag} must remain retired and fail closed`);
    assert.match(
      `${rejected.stdout}\n${rejected.stderr}`,
      /unknown author-dsf2026-show test arguments must fail closed/,
      `${flag} must be rejected as an unknown argument without a compatibility shim`,
    );
  }
}

const ioSource = await readFile(ioPath, "utf8");
const safeWriteHelperSource = await readFile(safeWriteHelperPath, "utf8");
assert.match(ioSource, /C:\\\\Windows\\\\System32\\\\WindowsPowerShell\\\\v1\.0\\\\powershell\.exe/, "Windows writer must use the pinned trusted PowerShell path");
assert.doesNotMatch(ioSource, /process\.env\.SystemRoot|process\.env\.windir|process\.env\.WINDIR/i, "Windows writer must not derive the executable from caller-controlled environment");
assert.match(ioSource, /spawnSync\(powershellPath/, "Windows writer must spawn only the verified absolute PowerShell path");
assert.doesNotMatch(ioSource, /spawnSync\(\s*[\"']powershell\.exe[\"']/, "Windows writer must not use a bare PowerShell executable name");
assert.doesNotMatch(ioSource, /adversarialMovePath|AdversarialMovePath/, "the production writer must not expose a parent-move argument");
assert.doesNotMatch(ioSource, /afterCanonicalize/, "the production path-preparation API must not expose a destructive race callback");
assert.doesNotMatch(safeWriteHelperSource, /AdversarialMovePath|adversarialMovePath|MoveFileEx|CreateDirectory|TEST_MODE|TEST_NONCE/, "the production native helper must not expose a destructive race operation");
assert.match(safeWriteHelperSource, /DeletePending/, "native failures must mark the created handle DeletePending");
assert.match(safeWriteHelperSource, /FILE_SHARE_READ,\s*$/m, "native child opens must retain only read sharing");
assert.match(safeWriteHelperSource, /AssertLexicalChild/, "native publication must verify the held child identity at its lexical path");

const manifestBytes = await readFile(CANONICAL_MANIFEST_PATH);
assert.equal(sha256(manifestBytes), CANONICAL_MANIFEST_SHA256, "test manifest must be the pinned canonical bytes");
const manifest = parseUtf8Json(manifestBytes);
validateCanonicalManifest(manifest);

const originalBase = makeBase();
const baseBefore = structuredClone(originalBase);
const authored = authorDsf2026Show(originalBase, manifest);
const authoredAgain = authorDsf2026Show(originalBase, manifest);
assert.deepEqual(authored, authoredAgain, "repeated pure authoring must be deterministic");
assertGeneratedTimelineLayerReferences(authored.snapshot.timeline, "pure authored source Timeline");
assertGeneratedTimelineLayerReferences(authored.snapshot.timeline_bank[1], "pure authored destination Timeline");
assertCanonicalTimelineLayers(authored.snapshot.timeline, [
  { id: 2, kind: "Audio", order: 0 },
  { id: 1, kind: "Lighting", order: 1 },
], "pure authored source Timeline");
assertCanonicalTimelineLayers(authored.snapshot.timeline_bank[1], [
  { id: 3, kind: "Audio", order: 0 },
], "pure authored destination Timeline");
assert.deepEqual(
  authored.snapshot.timeline_bank.map(({ audio_clips }) => audio_clips),
  [[], []],
  "authoring without reference audio must not add Timeline Audio Clips",
);
assert.deepEqual(
  authored.snapshot.video.media_assets ?? [],
  [],
  "authoring without reference audio must not add Media Library assets",
);
assert.deepEqual(authored.snapshot.timeline, authored.snapshot.timeline_bank[0], "pure authored active Timeline must equal its bank projection");
assert.deepEqual(originalBase, baseBefore, "pure authoring must not mutate the base object");
assert.notStrictEqual(authored, originalBase, "authoring must return a new project object");
assert.deepEqual(authored.custom_profiles, originalBase.custom_profiles, "custom profiles must remain byte-equivalent JSON values");
assert.deepEqual(authored.fixture_groups, originalBase.fixture_groups, "fixture groups must remain byte-equivalent JSON values");
assert.deepEqual(authored.midi_mappings, originalBase.midi_mappings, "MIDI mappings must remain unchanged");
assert.deepEqual(authored.osc_mappings, originalBase.osc_mappings, "OSC mappings must remain unchanged");
assert.deepEqual(authored.dmx_mappings, originalBase.dmx_mappings, "DMX mappings must remain unchanged");
assert.deepEqual(authored.snapshot.fixtures, originalBase.snapshot.fixtures, "fixtures must remain unchanged");
assert.deepEqual(authored.snapshot.cues, originalBase.snapshot.cues, "cues must remain unchanged");
assert.deepEqual(authored.snapshot.output, {
  ...originalBase.snapshot.output,
  enabled: false,
  protocol: "EnttecOpenDmx",
  serial_port: "",
  serial_baud_rate: 250_000,
}, "output config must retain unrelated fields while staging the exact disabled logical route with an empty machine-local serial_port");
assert.deepEqual(authored.snapshot.dmx_outputs, [{
  ...originalBase.snapshot.dmx_outputs[0],
  enabled: false,
  protocol: "EnttecOpenDmx",
  serial_port: "",
  serial_baud_rate: 250_000,
}], "persisted DMX route must be staged identically without pinning a machine-local physical port");
assert.deepEqual(
  validateDmxOutputStaging(authored),
  { detail: "primary and persisted DMX routes are staged as disabled EnttecOpenDmx at 250000 baud; physical port is machine-local; project route serial_port is empty" },
  "valid staged DMX output must explain the machine-local physical-port boundary",
);
{
  const stalePhysicalPort = structuredClone(authored);
  stalePhysicalPort.snapshot.output.serial_port = "COM9";
  assert.deepEqual(
    validateDmxOutputStaging(stalePhysicalPort),
    { error: "primary and persisted DMX routes must be disabled EnttecOpenDmx at 250000 baud; physical port is machine-local; project route serial_port must be empty" },
    "a project-persisted physical port must fail closed with the exact machine-local boundary",
  );
}
{
  const stalePersistedPhysicalPort = structuredClone(authored);
  stalePersistedPhysicalPort.snapshot.dmx_outputs[0].serial_port = "COM9";
  assert.deepEqual(
    validateDmxOutputStaging(stalePersistedPhysicalPort),
    { error: "primary and persisted DMX routes must be disabled EnttecOpenDmx at 250000 baud; physical port is machine-local; project route serial_port must be empty" },
    "a physical port embedded in the persisted DMX route must fail closed independently of the primary route",
  );
}
assert.deepEqual(authored.snapshot.video, originalBase.snapshot.video, "video state outside the authored Timeline must remain unchanged");
assert.deepEqual(authored.dj_track_triggers, [{
  id: "jinsei-over-production",
  selector: { titleContains: "人生オーバー", fallbackDeck: 1 },
  timelineId: 1,
  retrigger: "once_per_play_session",
}], "production mapping must be exact");

const referenceAudio = {
  jinseiOverPath: {
    path: "C:/show/reference-audio/jinsei-over-reference.mp3",
    durationMs: 214_032,
    byteSize: 8_561_392,
    sha256: "de7a0e78d269643035823df57ad5dd98fe1294583f2a072b91aea9c66a66c7ca",
  },
  madowHoshiPath: {
    path: "C:/show/reference-audio/madow-hoshi-reference.mp3",
    durationMs: 273_432,
    byteSize: 9_738_524,
    sha256: "e82ae9e8641ccd798fc5e90dc40603286d6965d9e726989f018871c2446c3c3b",
  },
};
const authoredWithReferenceAudio = authorDsf2026Show(originalBase, manifest, { referenceAudio });
assert.deepEqual(originalBase, baseBefore, "reference audio authoring must not mutate the base object");
assert.equal(authoredWithReferenceAudio.snapshot.timeline_bank.length, 2, "reference audio authoring must retain both Timelines");
assert.deepEqual(authoredWithReferenceAudio.snapshot.timeline, authoredWithReferenceAudio.snapshot.timeline_bank[0], "reference audio active Timeline must equal its bank projection");
assert.equal(authoredWithReferenceAudio.snapshot.timeline.audio_muted, false, "reference audio authoring must not mute the Timeline master");
assertGeneratedTimelineLayerReferences(authoredWithReferenceAudio.snapshot.timeline, "reference authored source Timeline");
assertGeneratedTimelineLayerReferences(authoredWithReferenceAudio.snapshot.timeline_bank[1], "reference authored destination Timeline");
assertCanonicalTimelineLayers(authoredWithReferenceAudio.snapshot.timeline, [
  { id: 2, kind: "Audio", order: 0 },
  { id: 1, kind: "Lighting", order: 1 },
], "reference authored source Timeline");
assertCanonicalTimelineLayers(authoredWithReferenceAudio.snapshot.timeline_bank[1], [
  { id: 3, kind: "Audio", order: 0 },
], "reference authored destination Timeline");
for (const [timeline, expectedDuration, expectedAssetId] of [
  [authoredWithReferenceAudio.snapshot.timeline, 214_032, 1],
  [authoredWithReferenceAudio.snapshot.timeline_bank[1], 273_432, 2],
]) {
  const audioLayer = timeline.layers.find(({ kind }) => kind === "Audio");
  assert.ok(audioLayer, "each reference Timeline must receive one Audio lane");
  assert.deepEqual({ ...audioLayer }, {
    id: expectedAssetId + 1,
    label: "Reference Audio",
    order: 0,
    muted: true,
    locked: false,
    solo: false,
    expanded: true,
    kind: "Audio",
  }, "reference Audio lanes must be separate and muted by default");
  if (timeline.id === 1) {
    assert.deepEqual(timeline.layers.filter(({ kind }) => kind === "Lighting"), [{
      id: 1,
      label: "Lighting",
      order: 1,
      muted: false,
      locked: false,
      solo: false,
      expanded: true,
      kind: "Lighting",
    }], "source boundary events must retain a dedicated Lighting lane");
  }
  assert.equal(timeline.audio_clips.length, 1, "each reference Timeline must receive one ordinary Audio Clip");
  assert.deepEqual(timeline.audio_clips[0], {
    id: expectedAssetId,
    layer_id: audioLayer.id,
    media_asset_id: expectedAssetId,
    path: "",
    start_ms: 0,
    offset_ms: 0,
    duration_ms: expectedDuration,
    gain: 1,
    fade_in_ms: 0,
    fade_out_ms: 0,
  }, "reference clips must use the existing MediaAsset-backed clip shape");
}
assert.equal(authoredWithReferenceAudio.snapshot.video.media_assets.length, 2, "reference audio must be adopted into the existing Media Library catalog");
assert.deepEqual(authoredWithReferenceAudio.snapshot.video.media_assets.map(({ id, label, source, content_hash, byte_size }) => ({ id, label, source, content_hash, byte_size })), [
  {
    id: 1,
    label: "Jinsei Over Reference Audio",
    source: {
      kind: "File",
      path: "C:/show/reference-audio/jinsei-over-reference.mp3",
      name: null,
      codec: "mp3",
      metadata: { duration_ms: 214_032, width: null, height: null, frame_rate: null, has_audio: true },
    },
    content_hash: { algorithm: "Sha256", hex: referenceAudio.jinseiOverPath.sha256 },
    byte_size: 8_561_392,
  },
  {
    id: 2,
    label: "Madow Hoshi Reference Audio",
    source: {
      kind: "File",
      path: "C:/show/reference-audio/madow-hoshi-reference.mp3",
      name: null,
      codec: "mp3",
      metadata: { duration_ms: 273_432, width: null, height: null, frame_rate: null, has_audio: true },
    },
    content_hash: { algorithm: "Sha256", hex: referenceAudio.madowHoshiPath.sha256 },
    byte_size: 9_738_524,
  },
], "reference MediaAssets must preserve measured duration and byte identity");
checkPass(preflightShowContract(authoredWithReferenceAudio), "reference audio authored output");
assertThrows(
  () => authorDsf2026Show(originalBase, manifest, { referenceAudio: { ...referenceAudio, madowHoshiPath: { ...referenceAudio.madowHoshiPath, sha256: "bad" } } }),
  /SHA-256/i,
  "invalid reference MediaAsset identity",
);

const source = authored.snapshot.timeline;
const destination = authored.snapshot.timeline_bank[1];
assert.deepEqual(authored.snapshot.timeline, authored.snapshot.timeline_bank[0], "active Timeline must equal its authored bank projection");
assert.equal(authored.snapshot.timeline_bank.length, 2, "authored Timeline bank must contain exactly two Timelines");
assert.equal(source.id, 1);
assert.equal(source.label, "人生オーバー");
assert.equal(source.duration_ms, Math.round(10_536_286 * 1_000 / 48_000));
assert.equal(destination.id, 2);
assert.equal(destination.label, "惑う星");
assert.equal(destination.duration_ms, Math.round(12_470_103 * 1_000 / 48_000));
assert.equal(source.guide_enabled, true);
assert.equal(destination.guide_enabled, true);
assert.equal(source.metronome_enabled, false);
assert.equal(destination.metronome_enabled, false);
assert.equal(source.count_in_beats, 4);
assert.equal(destination.count_in_beats, 4);
assert.equal(source.playing, false);
assert.equal(source.position_ms, 0);
assert.deepEqual(source.tempo_meter_map, [
  { position_sixteenth_steps: 0, bpm: 170, numerator: 4, denominator: 4, interpolation: "Step", measure_number: 1 },
  { position_sixteenth_steps: 2368, bpm: 170, numerator: 4, denominator: 4, interpolation: "Linear", measure_number: 149 },
  { position_sixteenth_steps: 2496, bpm: 194, numerator: 4, denominator: 4, interpolation: "Step", measure_number: 157 },
]);
assert.deepEqual(destination.tempo_meter_map.map(({ position_sixteenth_steps, bpm, numerator, denominator, interpolation, measure_number }) => ({ position_sixteenth_steps, bpm, numerator, denominator, interpolation, measure_number })), [
  { position_sixteenth_steps: 0, bpm: 194, numerator: 4, denominator: 4, interpolation: "Step", measure_number: 1 },
  { position_sixteenth_steps: 272, bpm: 194, numerator: 6, denominator: 4, interpolation: "Step", measure_number: 18 },
  { position_sixteenth_steps: 296, bpm: 194, numerator: 4, denominator: 4, interpolation: "Step", measure_number: 19 },
  { position_sixteenth_steps: 1864, bpm: 194, numerator: 5, denominator: 4, interpolation: "Step", measure_number: 117 },
  { position_sixteenth_steps: 1924, bpm: 194, numerator: 6, denominator: 4, interpolation: "Step", measure_number: 120 },
  { position_sixteenth_steps: 1948, bpm: 194, numerator: 5, denominator: 4, interpolation: "Step", measure_number: 121 },
  { position_sixteenth_steps: 2008, bpm: 194, numerator: 6, denominator: 4, interpolation: "Step", measure_number: 124 },
  { position_sixteenth_steps: 2032, bpm: 194, numerator: 4, denominator: 4, interpolation: "Step", measure_number: 125 },
]);
assert.deepEqual(source.loop_region, {
  a_ms: 136_941,
  b_ms: 138_353,
  enabled: true,
  musical_length_beats: 4,
}, "source loop must use exact rounded m98/m99 first-click positions");
assert.deepEqual(source.events.map(({ id, cue_id, time_ms, track, layer_id, duration_ms }) => ({ id, cue_id, time_ms, track, layer_id, duration_ms })), [
  { id: 1, cue_id: 1, time_ms: 138_353, track: "Lighting", layer_id: 1, duration_ms: 0 },
  { id: 2, cue_id: 2, time_ms: 138_353, track: "Lighting", layer_id: 1, duration_ms: 0 },
], "existing all_white/all_max Cues must coexist exactly at m99 outside the loop");
assert.deepEqual(originalBase.snapshot.cues.filter(({ label }) => ["all_white", "all_max"].includes(label)).map(({ id, label, cue_list_id, group_id, recall_mode }) => ({ id, label, cue_list_id, group_id, recall_mode })), [
  { id: 1, label: "all_white", cue_list_id: 1, group_id: "color", recall_mode: "Coexist" },
  { id: 2, label: "all_max", cue_list_id: 2, group_id: "dimmer", recall_mode: "Coexist" },
], "boundary Cue IDs, labels, lists, and coexist mode must come from base");
assert.equal(source.events[0].time_ms, source.loop_region.b_ms, "all_white event must be at the first point outside the loop");
assert.ok(source.events.every((event) => event.time_ms >= source.loop_region.b_ms), "boundary events must not be inside the indefinite loop");
assert.deepEqual(source.follow, {
  enabled: true,
  next_timeline_id: 2,
  duration: { unit: "Bars", value_milliunits: 1000 },
  curve: "Linear",
  video_kind: "Crossfade",
  lighting_policy: "hold_then_cut",
  destination_bpm: null,
  preroll_ms: 0,
  trans_cadence_bars: 2,
  trans_target_measures: [149, 151, 153, 155],
  hold_first_destination_measure: true,
  fault_policy: "hold",
});
assert.deepEqual(source.phases.map(({ id, label, role, start_ms, end_ms }) => ({ id, label, role, start_ms, end_ms })), [
  { id: 101, label: "Intro", role: "intro", start_ms: 0, end_ms: 24_000 },
  { id: 102, label: "Verse", role: "verse", start_ms: 24_000, end_ms: 35_294 },
  { id: 103, label: "Pre Chorus", role: "pre_chorus", start_ms: 35_294, end_ms: 46_588 },
  { id: 104, label: "Chorus", role: "chorus", start_ms: 46_588, end_ms: 69_176 },
  { id: 105, label: "Interlude", role: "interlude", start_ms: 69_176, end_ms: 80_471 },
  { id: 106, label: "Verse", role: "verse", start_ms: 80_471, end_ms: 91_765 },
  { id: 107, label: "Pre Chorus", role: "pre_chorus", start_ms: 91_765, end_ms: 114_353 },
  { id: 108, label: "Chorus", role: "chorus", start_ms: 114_353, end_ms: 159_529 },
  { id: 109, label: "Breakdown", role: "breakdown", start_ms: 159_529, end_ms: 163_765 },
  { id: 110, label: "Chorus", role: "chorus", start_ms: 163_765, end_ms: 199_059 },
  { id: 111, label: "Outro", role: "outro", start_ms: 199_059, end_ms: 219_506 },
]);
assert.deepEqual(destination.phases.map(({ label, role, start_ms, end_ms }) => ({ label, role, start_ms, end_ms })), [
  { label: "Verse", role: "verse", start_ms: 22_887, end_ms: 52_577 },
  { label: "Pre Chorus", role: "pre_chorus", start_ms: 52_577, end_ms: 94_639 },
  { label: "Chorus", role: "chorus", start_ms: 94_639, end_ms: 124_330 },
  { label: "Interlude", role: "interlude", start_ms: 124_330, end_ms: 157_113 },
  { label: "Verse", role: "verse", start_ms: 157_113, end_ms: 184_330 },
  { label: "Pre Chorus", role: "pre_chorus", start_ms: 184_330, end_ms: 205_361 },
  { label: "Chorus", role: "chorus", start_ms: 205_361, end_ms: 240_000 },
  { label: "Outro", role: "outro", start_ms: 240_000, end_ms: 259_794 },
]);
const authoredReport = preflightShowContract(authored);
checkPass(authoredReport, "pure authored output");
assert.deepEqual(authoredReport.checks.filter(({ id }) => ["lighting_boundary", "dmx_staging"].includes(id)).map(({ id, status }) => ({ id, status })), [
  { id: "lighting_boundary", status: "PASS" },
  { id: "dmx_staging", status: "PASS" },
], "show-specific boundary and disabled DMX staging checks must PASS");
if (RUN_TEST_ONLY_MISSING_CONTENT_ARTIFACT) {
  await runTestOnlyMissingContentArtifactBehavior();
  console.log(`PINNED AUTHORED CONTENT-ARTIFACT GATE intentionally isolated by ${TEST_ONLY_MISSING_CONTENT_ARTIFACT}; generated-contract tests continue without the pinned authored content artifact`);
} else if (REQUIRE_CONTENT_ARTIFACT) {
  await runPinnedContentArtifactGate(CONTENT_ARTIFACT_PATH);
}
assertRetiredArtifactFlagsFailClosed();

assertThrows(() => authorDsf2026Show({ ...originalBase, dj_track_triggers: [{ id: "already-authored" }] }, manifest), /dj_track_triggers.*absent or empty|existing mapping/i, "existing mapping");
{
  const dirty = structuredClone(originalBase);
  dirty.snapshot.timeline.events.push({ id: 1 });
  assertThrows(() => authorDsf2026Show(dirty, manifest), /events.*empty|default/i, "nonempty authored timeline");
}
{
  const badHashShape = structuredClone(manifest);
  badHashShape.clickEvents[0].localFrame = 1;
  assertThrows(() => authorDsf2026Show(originalBase, badHashShape), /clickEvents.*canonical chart/i, "bad chart value");
}
{
  const badManifest = structuredClone(manifest);
  badManifest.audioFormat.channels = 2;
  assertThrows(() => validateCanonicalManifest(badManifest), /audioFormat.*canonical/i, "bad manifest shape");
}
assertThrows(() => validateBaseProject({ ...originalBase, version: 2 }), /version.*exactly 1/i, "dirty base version");
{
  const missingCue = structuredClone(originalBase);
  missingCue.snapshot.cues = missingCue.snapshot.cues.filter(({ label }) => label !== "all_white");
  assertThrows(() => authorDsf2026Show(missingCue, manifest), /all_white/i, "missing existing all_white Cue");
}
{
  const unsafeCue = structuredClone(originalBase);
  unsafeCue.snapshot.cues.find(({ label }) => label === "all_max").recall_mode = "Replace";
  assertThrows(() => authorDsf2026Show(unsafeCue, manifest), /all_max.*Coexist/i, "non-coexist all_max Cue");
}
{
  const nullLayerId = emptyTimeline();
  nullLayerId.layers = [{ id: 1, kind: "Lighting" }];
  nullLayerId.events = [{ id: 1, track: "Lighting", layer_id: null }];
  assertThrows(
    () => validateGeneratedTimelineLayerReferences(nullLayerId, "null Lighting event layer_id"),
    /^BLOCKED: null Lighting event layer_id event 1 requires a positive timeline layer ID$/,
    "null Lighting event layer_id",
  );
}
{
  const missingLayerId = emptyTimeline();
  missingLayerId.layers = [{ id: 1, kind: "Lighting" }];
  missingLayerId.events = [{ id: 1, track: "Lighting" }];
  assertThrows(
    () => validateGeneratedTimelineLayerReferences(missingLayerId, "missing Lighting event layer_id"),
    /^BLOCKED: missing Lighting event layer_id event 1 requires a positive timeline layer ID$/,
    "missing Lighting event layer_id",
  );
}
{
  const wrongKind = emptyTimeline();
  wrongKind.layers = [{ id: 1, kind: "Video" }];
  wrongKind.events = [{ id: 1, track: "Lighting", layer_id: 1 }];
  assertThrows(
    () => validateGeneratedTimelineLayerReferences(wrongKind, "wrong-kind event layer"),
    /^BLOCKED: wrong-kind event layer event 1 uses Lighting track with incompatible Video timeline layer 1$/,
    "wrong-kind event layer",
  );
}
{
  const duplicateLayerId = emptyTimeline();
  duplicateLayerId.layers = [{ id: 1, kind: "Lighting" }, { id: 1, kind: "Lighting" }];
  assertThrows(
    () => validateGeneratedTimelineLayerReferences(duplicateLayerId, "duplicate layer ID"),
    /^BLOCKED: duplicate layer ID contains duplicate timeline layer ID 1$/,
    "duplicate layer ID",
  );
}
{
  const audioClipOnLighting = emptyTimeline();
  audioClipOnLighting.layers = [{ id: 1, kind: "Lighting" }];
  audioClipOnLighting.audio_clips = [{ id: 1, layer_id: 1 }];
  assertThrows(
    () => validateGeneratedTimelineLayerReferences(audioClipOnLighting, "Audio clip Lighting layer"),
    /^BLOCKED: Audio clip Lighting layer audio clip 1 references non-Audio timeline layer 1$/,
    "Audio clip Lighting layer",
  );
}

const work = await mkdtemp(join(tmpdir(), "syndocal-author-dsf2026-"));
try {
  const approvedArtifactPath = join(REPO_ROOT, "target", "qa", "dsf2026-native-alpha27", APPROVED_BASE_FILENAME);
  let approvedBytes;
  try {
    approvedBytes = await readFile(approvedArtifactPath);
  } catch (error) {
    if (error?.code !== "ENOENT") throw error;
    console.log(`approved alpha27 base preservation check skipped (artifact absent by design): ${approvedArtifactPath}`);
  }
  const hasApprovedArtifact = approvedBytes !== undefined;
  if (hasApprovedArtifact) {
    assert.equal(approvedBytes.length, 1_079_564, "the local approved alpha27 artifact must retain its pinned byte length");
    assert.equal(sha256(approvedBytes), APPROVED_BASE_SHA256, "the local approved alpha27 artifact must retain its pinned bytes");
    const approvedProject = parseUtf8Json(approvedBytes);
    assertApprovedFixtureStageLayouts(approvedProject, "approved alpha27 base");
    const authoredFromApproved = authorDsf2026Show(approvedProject, manifest);
    assertApprovedFixtureStageLayouts(authoredFromApproved, "authored alpha27 output");
    assert.deepEqual(authoredFromApproved.snapshot.fixtures, approvedProject.snapshot.fixtures, "authored output must deep-preserve every approved fixture");
    assert.deepEqual(
      authoredFromApproved.snapshot.fixtures.map(({ stage_layout }) => stage_layout),
      approvedProject.snapshot.fixtures.map(({ stage_layout }) => stage_layout),
      "authored output must deep-preserve every approved fixture stage_layout",
    );
  }
  const basePath = join(work, hasApprovedArtifact ? APPROVED_BASE_FILENAME : "base.sdc");
  const manifestPath = join(work, "manifest.json");
  const outputPath = join(work, "authored.sdc");
  const arbitraryBasePath = join(work, "base.sdc");
  await writeFile(arbitraryBasePath, Buffer.from(`${JSON.stringify(originalBase, null, 2)}\n`, "utf8"), { flag: "wx" });
  await writeFile(manifestPath, manifestBytes, { flag: "wx" });
  const arbitrary = spawnSync(process.execPath, [fileURLToPath(toolPath), "--base", arbitraryBasePath, "--manifest", manifestPath, "--output", join(work, "arbitrary-output.sdc")], { encoding: "utf8" });
  assert.notEqual(arbitrary.status, 0, "an arbitrary structurally-valid base must fail closed");
  assert.match(`${arbitrary.stdout}\n${arbitrary.stderr}`, /approved|SHA-256/i);
  await assert.rejects(() => readFile(join(work, "arbitrary-output.sdc")), { code: "ENOENT" }, "base identity rejection must happen before any output write");
  await rm(arbitraryBasePath, { force: true });
  await writeFile(basePath, hasApprovedArtifact ? approvedBytes : Buffer.from(`${JSON.stringify(originalBase, null, 2)}\n`, "utf8"), { flag: "wx" });
  const baseHashBefore = sha256(await readFile(basePath));

  const referenceInputs = join(work, "reference-inputs");
  await mkdir(referenceInputs, { recursive: true });
  const jinseiReferenceSource = join(referenceInputs, "jinsei-test.mp3");
  const madowReferenceSource = join(referenceInputs, "madow-test.mp3");
  const jinseiReferenceBytes = wavBytes(250);
  const madowReferenceBytes = wavBytes(500);
  await writeFile(jinseiReferenceSource, jinseiReferenceBytes, { flag: "wx" });
  await writeFile(madowReferenceSource, madowReferenceBytes, { flag: "wx" });
  const referenceOptions = {
    jinseiOverPath: jinseiReferenceSource,
    madowHoshiPath: madowReferenceSource,
  };

  const referenceOutput = join(work, "reference-authoring.sdc");
  const referencePublication = await prepareAuthoredShowPublication(referenceOutput);
  const referencePlan = await planReferenceAudio(referenceOptions, referencePublication);
  const referenceProject = authorDsf2026Show(originalBase, manifest, {
    referenceAudio: referencePlan.descriptors,
  });
  checkPass(preflightShowContract(referenceProject), "planned reference-audio output");
  await materializeReferenceAudio(referencePlan);
  await assert.rejects(
    () => writeAuthoredShow(referenceProject, referencePublication.outputPath, referencePublication),
    /reference MediaAssets.*publication plan/i,
    "managed reference MediaAssets must not bypass their held-parent publication fence",
  );
  await assert.rejects(() => access(referencePublication.outputPath), { code: "ENOENT" }, "reference-plan bypass rejection must not publish an .sdc");
  await writeAuthoredShow(referenceProject, referencePublication.outputPath, referencePublication, referencePlan);
  assert.deepEqual(await readFile(jinseiReferenceSource), jinseiReferenceBytes, "reference authoring must never mutate the Jinsei source bytes");
  assert.deepEqual(await readFile(madowReferenceSource), madowReferenceBytes, "reference authoring must never mutate the Madow source bytes");
  assert.equal(referenceProject.snapshot.video.media_assets.length, 2, "reference authoring must add exactly two Media Library assets");
  for (const [index, expected] of [
    [0, { source: jinseiReferenceBytes, timelineId: 1, sourcePath: jinseiReferenceSource }],
    [1, { source: madowReferenceBytes, timelineId: 2, sourcePath: madowReferenceSource }],
  ]) {
    const asset = referenceProject.snapshot.video.media_assets[index];
    const clip = referenceProject.snapshot.timeline_bank.find(({ id }) => id === expected.timelineId).audio_clips[0];
    assert.equal(asset.source.path.toLowerCase(), join(work, Object.values(REFERENCE_AUDIO_SIDECAR_NAMES)[index]).toLowerCase(), "reference MediaAsset must point only at its deterministic managed sidecar");
    assert.equal(asset.source.metadata.duration_ms, index === 0 ? 250 : 500, "reference MediaAsset duration must be measured from the read bytes");
    assert.equal(asset.content_hash.hex, sha256(expected.source).toLowerCase(), "reference MediaAsset hash must describe the exact original bytes");
    assert.equal(asset.byte_size, expected.source.length, "reference MediaAsset byte size must describe the exact original bytes");
    assert.equal(clip.media_asset_id, asset.id, "reference Timeline clip must use the ordinary MediaAsset path");
    assert.equal(clip.path, "", "reference Timeline clip must not duplicate a legacy path");
    const referenceTimeline = referenceProject.snapshot.timeline_bank.find(({ id }) => id === expected.timelineId);
    assert.equal(referenceTimeline.layers.find(({ kind }) => kind === "Audio")?.muted, true, "reference Audio lane must remain explicitly muted by default");
    assert.deepEqual(await readFile(asset.source.path), expected.source, "managed sidecar bytes must be exact");
    assert.equal(referenceProject.snapshot.timeline_bank.find(({ id }) => id === expected.timelineId).audio_clips.length, 1, "reference audio must not become a Click or Guide asset");
    assert.ok(!JSON.stringify(referenceProject).includes(expected.sourcePath), "authored project must not leak an operator source path");
  }

  const badExtensionPublication = await prepareAuthoredShowPublication(join(work, "bad-reference-extension.sdc"));
  const badExtension = join(referenceInputs, "not-an-mp3.wav");
  await writeFile(badExtension, wavBytes(100), { flag: "wx" });
  await assert.rejects(
    () => planReferenceAudio({ jinseiOverPath: badExtension, madowHoshiPath: madowReferenceSource }, badExtensionPublication),
    /MP3/i,
    "reference authoring must reject a non-MP3 extension before staging sidecars",
  );
  for (const path of referenceSidecarPaths(work)) {
    // The positive plan owns these exact paths, so use a clean parent for the negative checks below.
    assert.ok(await readFile(path), "positive reference sidecar must remain readable");
  }

  const invalidReferenceParent = join(work, "invalid-reference");
  await mkdir(invalidReferenceParent, { recursive: true });
  const invalidReferencePublication = await prepareAuthoredShowPublication(join(invalidReferenceParent, "invalid-reference.sdc"));
  const invalidReference = join(invalidReferenceParent, "invalid.mp3");
  await writeFile(invalidReference, "not-decodable", { flag: "wx" });
  await assert.rejects(
    () => planReferenceAudio({ jinseiOverPath: invalidReference, madowHoshiPath: madowReferenceSource }, invalidReferencePublication),
    /ffprobe|duration/i,
    "reference authoring must fail closed when ffprobe cannot measure the read bytes",
  );
  for (const path of referenceSidecarPaths(invalidReferenceParent)) {
    await assert.rejects(() => access(path), { code: "ENOENT" }, "invalid reference input must not create a sidecar");
  }

  const swapParent = join(work, "source-swap");
  await mkdir(swapParent, { recursive: true });
  const swapSource = join(swapParent, "swap-source.mp3");
  const swapReplacement = join(swapParent, "swap-replacement.mp3");
  await writeFile(swapSource, wavBytes(250), { flag: "wx" });
  await writeFile(swapReplacement, wavBytes(750), { flag: "wx" });
  const swapPublication = await prepareAuthoredShowPublication(join(swapParent, "swap.sdc"));
  await assert.rejects(
    () => planReferenceAudioForTest(
      { jinseiOverPath: swapSource, madowHoshiPath: madowReferenceSource },
      swapPublication,
      {
        afterRead: async ({ path }) => {
          if (path === swapSource) {
            const displaced = join(swapParent, "swap-displaced.mp3");
            await rename(swapSource, displaced);
            await rename(swapReplacement, swapSource);
          }
        },
      },
    ),
    /changed|identity|unstable/i,
    "source replacement after a read must fail the handle/lexical identity fence",
  );
  for (const path of referenceSidecarPaths(swapParent)) {
    await assert.rejects(() => access(path), { code: "ENOENT" }, "source replacement must not create a sidecar");
  }

  const sourceParentRaceRoot = join(work, "source-parent-open-race");
  const sourceParentRaceLive = join(sourceParentRaceRoot, "live");
  const sourceParentRaceMoved = join(sourceParentRaceRoot, "moved");
  await mkdir(sourceParentRaceLive, { recursive: true });
  const sourceParentRacePath = join(sourceParentRaceLive, "race.mp3");
  await writeFile(sourceParentRacePath, wavBytes(250), { flag: "wx" });
  const sourceParentRacePublication = await prepareAuthoredShowPublication(join(sourceParentRaceRoot, "race.sdc"));
  let sourceParentRaceMovedOnce = false;
  await assert.rejects(
    () => planReferenceAudioForTest(
      { jinseiOverPath: sourceParentRacePath, madowHoshiPath: madowReferenceSource },
      sourceParentRacePublication,
      {
        beforeNativeOpen: async ({ absolute }) => {
          if (absolute !== sourceParentRacePath || sourceParentRaceMovedOnce) return;
          sourceParentRaceMovedOnce = true;
          await rename(sourceParentRaceLive, sourceParentRaceMoved);
          await mkdir(sourceParentRaceLive, { recursive: true });
          await writeFile(sourceParentRacePath, wavBytes(500), { flag: "wx" });
        },
      },
    ),
    /identity|changed|native no-reparse|parent/i,
    "source parent replacement between lexical preflight and native open must fail closed",
  );
  assert.equal(sourceParentRaceMovedOnce, true, "source parent race seam must run after the lexical inspection");
  for (const path of referenceSidecarPaths(sourceParentRaceRoot)) {
    await assert.rejects(() => access(path), { code: "ENOENT" }, "source parent replacement must not stage a sidecar");
  }

  const sidecarLeafSwapParent = join(work, "sidecar-leaf-swap");
  await mkdir(sidecarLeafSwapParent, { recursive: true });
  const sidecarLeafSwapPublication = await prepareAuthoredShowPublication(join(sidecarLeafSwapParent, "leaf-swap.sdc"));
  const sidecarLeafSwapPlan = await planReferenceAudio(referenceOptions, sidecarLeafSwapPublication);
  const swappedLeaf = sidecarLeafSwapPlan.entries[0].destination;
  const swappedLeafBytes = Buffer.from("external divergent sidecar leaf", "utf8");
  await writeFile(swappedLeaf, swappedLeafBytes, { flag: "wx" });
  await assert.rejects(
    () => materializeReferenceAudio(sidecarLeafSwapPlan),
    /different bytes/i,
    "a sidecar leaf introduced after planning must be reopened under the held parent and rejected by byte identity",
  );
  assert.deepEqual(await readFile(swappedLeaf), swappedLeafBytes, "sidecar leaf-swap rejection must not overwrite the introduced leaf");
  await assert.rejects(() => access(join(sidecarLeafSwapParent, "leaf-swap.sdc")), { code: "ENOENT" }, "sidecar leaf-swap rejection must not publish a candidate");

  const redirectParent = join(work, "sidecar-hash-matching-redirect");
  const redirectTargetParent = join(work, "sidecar-hash-matching-target");
  await mkdir(redirectParent, { recursive: true });
  await mkdir(redirectTargetParent, { recursive: true });
  const redirectPublication = await prepareAuthoredShowPublication(join(redirectParent, "redirect.sdc"));
  const redirectPlan = await planReferenceAudio(referenceOptions, redirectPublication);
  const redirectTarget = join(redirectTargetParent, "same-hash.mp3");
  await writeFile(redirectTarget, jinseiReferenceBytes, { flag: "wx" });
  const redirectLeaf = redirectPlan.entries[0].destination;
  try {
    await symlink(redirectTarget, redirectLeaf, "file");
    await assert.rejects(
      () => materializeReferenceAudio(redirectPlan),
      /reparse|native no-reparse|NtCreateFile/i,
      "a hash-matching external symlink sidecar must fail before reuse",
    );
    assert.deepEqual(await readFile(redirectTarget), jinseiReferenceBytes, "hash-matching external redirect must remain untouched");
    await assert.rejects(() => access(join(redirectParent, "redirect.sdc")), { code: "ENOENT" }, "external redirect rejection must not publish a candidate");
  } catch (error) {
    if (!["EPERM", "EACCES", "UNKNOWN"].includes(error?.code)) throw error;
    console.log("hash-matching external sidecar redirect test skipped: file symlink privilege unavailable");
  }

  const sidecarRaceRoot = join(work, "sidecar-parent-race");
  const sidecarRaceParent = join(sidecarRaceRoot, "parent");
  const sidecarRaceMoved = join(sidecarRaceRoot, "parent-moved");
  await mkdir(sidecarRaceParent, { recursive: true });
  const sidecarRacePublication = await prepareAuthoredShowPublication(join(sidecarRaceParent, "race.sdc"));
  const sidecarRacePlan = await planReferenceAudio(referenceOptions, sidecarRacePublication);
  await rename(sidecarRaceParent, sidecarRaceMoved);
  await assert.rejects(
    () => materializeReferenceAudio(sidecarRacePlan),
    /missing|changed|reparse|identity/i,
    "sidecar materialization must fail closed when the guarded parent changes after planning",
  );
  for (const path of referenceSidecarPaths(sidecarRaceMoved)) {
    await assert.rejects(() => access(path), { code: "ENOENT" }, "a redirected parent must not receive a sidecar write");
  }

  const durableParent = join(work, "durable-reference-stage");
  await mkdir(durableParent, { recursive: true });
  const durableOutput = join(durableParent, "durable.sdc");
  const durablePublication = await prepareAuthoredShowPublication(durableOutput);
  const durablePlan = await planReferenceAudio(referenceOptions, durablePublication);
  await materializeReferenceAudio(durablePlan);
  await assert.rejects(() => access(durableOutput), { code: "ENOENT" }, "sidecar staging alone must never publish an incomplete .sdc");
  const durableRetryPublication = await prepareAuthoredShowPublication(durableOutput);
  const durableRetryPlan = await planReferenceAudio(referenceOptions, durableRetryPublication);
  assert.deepEqual(await materializeReferenceAudio(durableRetryPlan), durablePlan.descriptors, "same-byte retry must reuse the durable reference stage");
  const divergentJinsei = join(referenceInputs, "jinsei-divergent.mp3");
  await writeFile(divergentJinsei, wavBytes(1_000), { flag: "wx" });
  const divergentPlan = await planReferenceAudio({ jinseiOverPath: divergentJinsei, madowHoshiPath: madowReferenceSource }, durableRetryPublication);
  await assert.rejects(
    () => materializeReferenceAudio(divergentPlan),
    /already exists with different bytes/i,
    "divergent bytes must not replace a durable reference stage",
  );
  await assert.rejects(() => access(durableOutput), { code: "ENOENT" }, "divergent stage rejection must not publish an incomplete .sdc");
  const durableProject = authorDsf2026Show(originalBase, manifest, { referenceAudio: durableRetryPlan.descriptors });
  await writeAuthoredShow(durableProject, durableRetryPublication.outputPath, durableRetryPublication, durableRetryPlan);
  checkPass(preflightShowContract(parseUtf8Json(await readFile(durableOutput))), "same-byte durable stage retry output");

  const publicationFenceParent = join(work, "publication-fence");
  await mkdir(publicationFenceParent, { recursive: true });
  const publicationFenceOutput = join(publicationFenceParent, "fenced.sdc");
  const publicationFencePublication = await prepareAuthoredShowPublication(publicationFenceOutput);
  const publicationFencePlan = await planReferenceAudio(referenceOptions, publicationFencePublication);
  await materializeReferenceAudio(publicationFencePlan);
  const publicationFenceProject = authorDsf2026Show(originalBase, manifest, { referenceAudio: publicationFencePlan.descriptors });
  const publicationFenceLeaf = publicationFencePlan.entries[0].destination;
  const publicationFenceMoved = join(publicationFenceParent, "fence-displaced.mp3");
  await rename(publicationFenceLeaf, publicationFenceMoved);
  await writeFile(publicationFenceLeaf, Buffer.from("post-materialization leaf replacement", "utf8"), { flag: "wx" });
  await assert.rejects(
    () => writeAuthoredShow(publicationFenceProject, publicationFencePublication.outputPath, publicationFencePublication, publicationFencePlan),
    /publication fence|different bytes|native exclusive/i,
    "candidate publication must re-open exact sidecars and reject a post-materialization leaf replacement",
  );
  await assert.rejects(() => access(publicationFenceOutput), { code: "ENOENT" }, "a failed publication fence must leave no candidate .sdc");

  if (hasApprovedArtifact) {
    const workEntriesBeforePositive = (await readdir(work)).sort();
    const positive = spawnSync(process.execPath, [fileURLToPath(toolPath), "--base", basePath, "--manifest", manifestPath, "--output", outputPath], { encoding: "utf8" });
    assert.equal(positive.status, 0, `positive CLI must pass: ${positive.stderr}`);
    assert.equal(sha256(await readFile(basePath)), baseHashBefore, "CLI must not mutate base bytes");
    const outputValue = parseUtf8Json(await readFile(outputPath));
    checkPass(preflightShowContract(outputValue), "CLI authored output");
    assert.deepEqual(outputValue.snapshot.fixtures, parseUtf8Json(approvedBytes).snapshot.fixtures, "CLI must preserve fixtures");
    assert.deepEqual((await readdir(work)).sort(), [...workEntriesBeforePositive, "authored.sdc"].sort(), "ordinary CLI must write only the requested output");

    const cliReferenceParent = join(work, "cli-reference");
    await mkdir(cliReferenceParent, { recursive: true });
    const cliReferenceOutput = join(cliReferenceParent, "reference.sdc");
    const cliReference = spawnSync(process.execPath, [
      fileURLToPath(toolPath),
      "--base", basePath,
      "--manifest", manifestPath,
      "--output", cliReferenceOutput,
      "--reference-audio-jinsei-over", jinseiReferenceSource,
      "--reference-audio-madow-hoshi", madowReferenceSource,
    ], { encoding: "utf8" });
    assert.equal(cliReference.status, 0, `reference CLI must pass: ${cliReference.stderr}`);
    const cliReferenceProject = parseUtf8Json(await readFile(cliReferenceOutput));
    checkPass(preflightShowContract(cliReferenceProject), "reference CLI output");
    assert.equal(cliReferenceProject.snapshot.video.media_assets.length, 2, "reference CLI must persist exactly two Media Library assets");
    assert.equal(cliReferenceProject.snapshot.timeline.audio_clips.length, 1, "reference CLI must persist a normal Timeline Audio Clip");
    assert.deepEqual(await readFile(jinseiReferenceSource), jinseiReferenceBytes, "reference CLI must not mutate the Jinsei original");
    assert.deepEqual(await readFile(madowReferenceSource), madowReferenceBytes, "reference CLI must not mutate the Madow original");
    for (const [index, sidecarPath] of referenceSidecarPaths(cliReferenceParent).entries()) {
      assert.deepEqual(await readFile(sidecarPath), index === 0 ? jinseiReferenceBytes : madowReferenceBytes, "reference CLI sidecars must remain byte-exact");
    }

    const unpairedReferenceOutput = join(cliReferenceParent, "unpaired.sdc");
    const unpairedReference = spawnSync(process.execPath, [
      fileURLToPath(toolPath),
      "--base", basePath,
      "--manifest", manifestPath,
      "--output", unpairedReferenceOutput,
      "--reference-audio-jinsei-over", jinseiReferenceSource,
    ], { encoding: "utf8" });
    assert.notEqual(unpairedReference.status, 0, "unpaired reference CLI arguments must fail closed");
    await assert.rejects(() => access(unpairedReferenceOutput), { code: "ENOENT" }, "unpaired reference CLI arguments must not publish an .sdc");

    const cliCollisionParent = join(work, "cli-reference-collision");
    await mkdir(cliCollisionParent, { recursive: true });
    const cliCollisionSidecar = join(cliCollisionParent, REFERENCE_AUDIO_SIDECAR_NAMES.jinseiOverPath);
    const collisionBytes = Buffer.from("divergent-sidecar", "utf8");
    await writeFile(cliCollisionSidecar, collisionBytes, { flag: "wx" });
    const cliCollisionOutput = join(cliCollisionParent, "collision.sdc");
    const cliCollision = spawnSync(process.execPath, [
      fileURLToPath(toolPath),
      "--base", basePath,
      "--manifest", manifestPath,
      "--output", cliCollisionOutput,
      "--reference-audio-jinsei-over", jinseiReferenceSource,
      "--reference-audio-madow-hoshi", madowReferenceSource,
    ], { encoding: "utf8" });
    assert.notEqual(cliCollision.status, 0, "divergent deterministic sidecar collision must fail closed");
    assert.match(`${cliCollision.stdout}\n${cliCollision.stderr}`, /sidecar.*different bytes/i, "sidecar collision must explain its byte-identity rejection");
    assert.deepEqual(await readFile(cliCollisionSidecar), collisionBytes, "collision rejection must not overwrite the existing sidecar");
    await assert.rejects(() => access(cliCollisionOutput), { code: "ENOENT" }, "collision rejection must not publish an .sdc");
  }

  const existingOutput = join(work, "existing.sdc");
  const existingBytes = Buffer.from("existing-bytes\n", "utf8");
  await writeFile(existingOutput, existingBytes, { flag: "wx" });
  const existing = spawnSync(process.execPath, [fileURLToPath(toolPath), "--base", basePath, "--manifest", manifestPath, "--output", existingOutput], { encoding: "utf8" });
  assert.notEqual(existing.status, 0, "existing output must fail closed");
  assert.deepEqual(await readFile(existingOutput), existingBytes, "existing output must remain unchanged");

  const same = spawnSync(process.execPath, [fileURLToPath(toolPath), "--base", basePath, "--manifest", manifestPath, "--output", basePath], { encoding: "utf8" });
  assert.notEqual(same.status, 0, "base/output same path must fail closed");
  assert.equal(sha256(await readFile(basePath)), baseHashBefore, "same-path rejection must not mutate base");

  if (hasApprovedArtifact) {
    const badManifestPath = join(work, "bad-manifest.json");
    const badManifest = structuredClone(manifest);
    badManifest.clickEvents[0].localFrame = 1;
    await writeFile(badManifestPath, `${JSON.stringify(badManifest)}\n`, { flag: "wx" });
    const badHash = spawnSync(process.execPath, [fileURLToPath(toolPath), "--base", basePath, "--manifest", badManifestPath, "--output", join(work, "bad-hash.sdc")], { encoding: "utf8" });
    assert.notEqual(badHash.status, 0, "bad manifest hash must fail closed");
    assert.match(`${badHash.stdout}\n${badHash.stderr}`, /SHA-256.*canonical/i);

    const dirtyBasePath = join(work, "dirty", APPROVED_BASE_FILENAME);
    await mkdir(join(work, "dirty"), { recursive: true });
    const dirtyBytes = Buffer.from(`${JSON.stringify({ ...parseUtf8Json(approvedBytes), dirty: true })}\n`, "utf8");
    await writeFile(dirtyBasePath, dirtyBytes, { flag: "wx" });
    const dirtyRun = spawnSync(process.execPath, [fileURLToPath(toolPath), "--base", dirtyBasePath, "--manifest", manifestPath, "--output", join(work, "dirty-output.sdc")], { encoding: "utf8" });
    assert.notEqual(dirtyRun.status, 0, "dirty authored base must fail closed");
    assert.match(`${dirtyRun.stdout}\n${dirtyRun.stderr}`, /approved|SHA-256/i);
    await assert.rejects(() => readFile(join(work, "dirty-output.sdc")), { code: "ENOENT" }, "approved-name wrong-hash rejection must happen before any output write");
  }

  const linkBase = join(work, "link", APPROVED_BASE_FILENAME);
  try {
    await mkdir(join(work, "link"), { recursive: true });
    await symlink(basePath, linkBase, "file");
    const linked = spawnSync(process.execPath, [fileURLToPath(toolPath), "--base", linkBase, "--manifest", manifestPath, "--output", join(work, "linked-output.sdc")], { encoding: "utf8" });
    assert.notEqual(linked.status, 0, "symlink base must fail closed");
    assert.match(`${linked.stdout}\n${linked.stderr}`, /symlink|reparse/i);
  } catch (error) {
    if (!['EPERM', 'EACCES', 'UNKNOWN'].includes(error?.code)) throw error;
  }

  const junctionTarget = join(work, "junction-target");
  const junctionParent = join(work, "junction-parent");
  const junctionOutput = join(junctionParent, "junction-output.sdc");
  try {
    await mkdir(junctionTarget, { recursive: true });
    await symlink(junctionTarget, junctionParent, process.platform === "win32" ? "junction" : "dir");
    const junctionRun = spawnSync(process.execPath, [fileURLToPath(toolPath), "--base", basePath, "--manifest", manifestPath, "--output", junctionOutput], { encoding: "utf8" });
    assert.notEqual(junctionRun.status, 0, "reparse-backed output parent must fail closed");
    assert.match(`${junctionRun.stdout}\n${junctionRun.stderr}`, /symlink|reparse|parent/i);
    await assert.rejects(() => readFile(join(junctionTarget, "junction-output.sdc")), { code: "ENOENT" }, "reparse-backed output parent must not receive output");
  } catch (error) {
    if (!['EPERM', 'EACCES', 'UNKNOWN'].includes(error?.code)) throw error;
  }

  const blockedOutput = join(work, "preflight-blocked.sdc");
  const malformedOutput = structuredClone(authored);
  malformedOutput.snapshot.timeline.events = [{ id: 999 }];
  await assert.rejects(() => writeAuthoredShow(malformedOutput, blockedOutput), /preflight|BLOCKED|boundary/i, "generated preflight failure must stop before output write");
  await assert.rejects(() => readFile(blockedOutput), { code: "ENOENT" }, "generated preflight failure must leave no output");

  if (process.platform === "win32") {
    for (const leafName of ["stream:bad.sdc", "bad-tail. ", "bad-tail.", "CON.txt", "PRN.sdc", "AUX.dat", "NUL.bin", "COM1.log", "LPT9.tmp"]) {
      await assert.rejects(
        () => prepareOutputPath(join(work, leafName), { guard: {} }),
        /leaf|alternate data stream|dot|space|device|component/i,
        `unsafe Windows leaf ${JSON.stringify(leafName)} must fail closed`,
      );
    }
    await assert.rejects(
      () => prepareOutputPath("\\\\server\\share\\unsafe.sdc", { guard: {} }),
      /local|UNC|device|NTFS/i,
      "UNC output paths must fail closed",
    );
    await assert.rejects(
      () => prepareOutputPath(join(work, `${"x".repeat(300)}.sdc`), { guard: {} }),
      /length|boundary|local/i,
      "long output paths must fail closed",
    );
    const caseExisting = join(work, "CaseCollision.sdc");
    await writeFile(caseExisting, "existing", { flag: "wx" });
    await assert.rejects(
      () => prepareOutputPath(join(work, "casecollision.sdc"), { guard: {} }),
      /already exists|overwrite/i,
      "case-insensitive existing output must fail closed",
    );
    const hardlinkTarget = join(work, "hardlink-target.sdc");
    const hardlinkOutput = join(work, "hardlink-output.sdc");
    await writeFile(hardlinkTarget, "hardlink", { flag: "wx" });
    await link(hardlinkTarget, hardlinkOutput);
    await assert.rejects(
      () => prepareOutputPath(hardlinkOutput, { guard: {} }),
      /already exists|overwrite/i,
      "existing hardlink output must fail closed",
    );

    await access(TEST_POWERSHELL_PATH);
    const redirectedSystemRoot = join(work, "caller-controlled-system-root");
    const redirectedParent = join(redirectedSystemRoot, "System32", "WindowsPowerShell", "v1.0");
    await mkdir(redirectedParent, { recursive: true });
    await writeFile(join(redirectedParent, "powershell.exe"), "not-an-executable", { flag: "wx" });
    const redirectedOutput = join(work, "redirected-env.sdc");
    const redirectedGuard = {};
    const redirectedCheckedPath = await prepareOutputPath(redirectedOutput, { guard: redirectedGuard });
    await withEnvironment({ SystemRoot: redirectedSystemRoot }, async () => {
      await writeExclusive(redirectedCheckedPath, "literal-path-only", redirectedGuard);
    });
    assert.equal((await readFile(redirectedOutput)).toString(), "literal-path-only", "caller-controlled SystemRoot must not redirect the trusted executable");
    await rm(redirectedOutput, { force: true });

    const writeFailureMarker = 'WriteExact(child, bytes, bytes.Length, "output write");';
    const flushFailureMarker = 'if (!FlushFileBuffers(child))';
    assert.equal(safeWriteHelperSource.split(writeFailureMarker).length, 2, "write-failure test copy marker must remain unique");
    assert.equal(safeWriteHelperSource.split(flushFailureMarker).length, 2, "flush-failure test copy marker must remain unique");
    const writeFailureSource = safeWriteHelperSource.replace(
      writeFailureMarker,
      'WriteExact(child, bytes, Math.Max(1, bytes.Length / 2), "injected write failure");\n                        throw new InvalidOperationException("injected write failure after a partial native write");',
    );
    const flushFailureSource = safeWriteHelperSource.replace(
      flushFailureMarker,
      'throw new InvalidOperationException("injected flush failure after a complete native write");\n                        if (!FlushFileBuffers(child))',
    );
    for (const [mode, source] of [["write-failure", writeFailureSource], ["flush-failure", flushFailureSource]]) {
      const failureParent = join(work, mode);
      await mkdir(failureParent, { recursive: true });
      const failurePath = join(failureParent, `${mode}.sdc`);
      const failureGuard = {};
      const checkedFailurePath = await prepareOutputPath(failurePath, { guard: failureGuard });
      const testCopyPath = join(work, `${mode}-safe-write-copy.ps1`);
      await writeFile(testCopyPath, source, { flag: "wx" });
      const { result } = startPowerShell(testCopyPath, helperArguments(testCopyPath, failureParent, `${mode}.sdc`, failureGuard), "partial-output-injection");
      const failureResult = await result;
      assert.notEqual(failureResult.code, 0, `${mode} must fail through the isolated native test copy`);
      assert.match(`${failureResult.stdout}\n${failureResult.stderr}`, /injected|failed|cleanup/i, `${mode} must explain the injected failure`);
      await assert.rejects(() => readFile(checkedFailurePath), { code: "ENOENT" }, `${mode} must leave no partial output`);
      assert.deepEqual(await readdir(failureParent), [], `${mode} must leave no directory residue`);
    }

    const delayedSourceMarker = 'AssertChildRegular(child, "after native create");';
    assert.equal(safeWriteHelperSource.split(delayedSourceMarker).length, 2, "race test copy marker must remain unique");
    const delayedSource = safeWriteHelperSource.replace(
      delayedSourceMarker,
      'System.Threading.Thread.Sleep(3000);\n                        AssertChildRegular(child, "after native create");',
    );
    const moverPath = join(work, "safe-write-race-mover.ps1");
    await writeFile(moverPath, RACE_MOVER_SOURCE, { flag: "wx" });
    const delayedCopyPath = join(work, "safe-write-race-copy.ps1");
    await writeFile(delayedCopyPath, delayedSource, { flag: "wx" });

    async function runRenameRace(label, parentPath, leafName, sourcePath, destinationPath) {
      await mkdir(parentPath, { recursive: true });
      const outputPath = join(parentPath, leafName);
      const guard = {};
      const checkedOutputPath = await prepareOutputPath(outputPath, { guard });
      const triggerPath = outputPath;
      const readyPath = join(work, `${label}-mover-ready`);
      const mover = startPowerShell(moverPath, [
        "-SourcePath", sourcePath,
        "-DestinationPath", destinationPath,
        "-TriggerPath", triggerPath,
        "-ReadyPath", readyPath,
        "-TimeoutMs", "2000",
      ], "");
      await waitForPath(readyPath);
      const writer = startPowerShell(delayedCopyPath, helperArguments(delayedCopyPath, parentPath, leafName, guard), "rename-race-output");
      const [writerResult, moverResult] = await Promise.all([writer.result, mover.result]);
      assert.equal(writerResult.code, 0, `${label} safe-write test copy must publish successfully: ${writerResult.stderr}`);
      assert.equal(moverResult.code, 0, `${label} external rename must be denied: ${moverResult.stdout}\n${moverResult.stderr}`);
      assert.match(moverResult.stdout, /DENIED/, `${label} external rename helper must report denial`);
      assert.equal((await readFile(checkedOutputPath)).toString(), "rename-race-output", `${label} output must remain at its lexical path`);
      await assert.rejects(() => access(destinationPath), { code: "ENOENT" }, `${label} attacker destination must remain absent`);
      await rm(checkedOutputPath, { force: true });
    }

    const raceRoot = join(work, "race");
    await runRenameRace("parent", join(raceRoot, "parent"), "race.sdc", join(raceRoot, "parent"), join(raceRoot, "parent-moved"));
    const leafRaceParent = join(raceRoot, "leaf-parent");
    await runRenameRace("leaf", leafRaceParent, "leaf.sdc", join(leafRaceParent, "leaf.sdc"), join(leafRaceParent, "leaf-moved.sdc"));
  } else {
    const raceRoot = join(work, "race");
    const raceParent = join(raceRoot, "parent");
    const movedParent = join(raceRoot, "parent-moved");
    await mkdir(raceParent, { recursive: true });
    const raceGuard = {};
    const racePath = await prepareOutputPath(join(raceParent, "race.sdc"), { guard: raceGuard });
    await rename(raceParent, movedParent);
    await assert.rejects(() => writeExclusive(racePath, "must not redirect", raceGuard), /missing|changed|reparse|identity/i, "parent replacement between preparation and write must fail closed");
    await assert.rejects(() => readFile(join(movedParent, "race.sdc")), { code: "ENOENT" }, "the redirected race path must not receive output");
  }
} finally {
  await rm(work, { recursive: true, force: true });
}

console.log("author-dsf2026-show tests passed: warning0; Rust/native/hardware not claimed");
