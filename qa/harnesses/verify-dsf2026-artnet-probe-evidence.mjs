#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { execFileSync } from "node:child_process";
import { pathToFileURL } from "node:url";

import {
  ARTNET_MONITOR_EVIDENCE_SCHEMA,
  buildArtDmxForSelfTest,
  createArtNetMonitorState,
  finishArtNetMonitorCapture,
  recordArtNetMonitorDatagram,
  serializeArtNetMonitorEvidence,
} from "./artnet-monitor.mjs";

const MONITOR_PRODUCT = "Syndocal external Art-Net monitor";
const ARTNET_PROTOCOL_VERSION = 14;
const ARTNET_UNIVERSE = 0;
const ARTNET_PORT = 6454;
const ART_DMX_OPCODE = 0x5000;
const ART_DMX_PACKET_LENGTH = 530;
const DMX_SLOT_COUNT = 512;
const DMX_CHANNEL_1_INDEX = 0;
const DMX_CHANNEL_5_INDEX = 4;
const DMX_CHANNEL_500_INDEX = 499;

const ROOT_KEYS = [
  "schema", "product", "startedAt", "startedAtEpochMs", "endedAt", "endedAtEpochMs",
  "listen", "monitorUrl", "capture", "totalDatagrams", "artDmxFrames", "rejectedDatagrams",
  "changedFrames", "sequenceDiscontinuities", "maxGapMs", "firstFrameAt", "lastFrameAt",
  "sources", "streams", "universes", "transitions", "lastFrames", "lastFrame", "rawArtDmx", "reason",
];
const LISTEN_KEYS = ["address", "port"];
const CAPTURE_KEYS = ["transitionData", "maxTransitions"];
const FRAME_KEYS = [
  "at", "source", "universe", "sequence", "digest", "data", "nonZeroChannels", "maxValue", "firstNonZero",
];
const TRANSITION_KEYS = [
  "at", "universe", "sequence", "digest", "nonZeroChannels", "maxValue", "firstNonZero", "data",
];
const UNIVERSE_KEYS = [
  "frames", "changedFrames", "protocolVersion", "physical", "dataLength", "lastSequence", "lastDigest",
  "lastFrameAt", "lastFrameEpochMs", "nonZeroChannels", "maxValue", "firstNonZero",
];
const STREAM_KEYS = ["frames", "lastSequence", "lastFrameAt", "lastFrameEpochMs"];
const RAW_ART_DMX_KEYS = ["at", "receivedAtEpochMs", "source", "bytes"];
const FIRST_NON_ZERO_KEYS = ["channel", "value"];

function fail(detail) {
  throw new Error(`Fail closed: DSF2026 Art-Net probe evidence ${detail}`);
}

function requireRecord(value, label) {
  if (value === null || typeof value !== "object" || Array.isArray(value)) fail(`${label} must be an object`);
  return value;
}

function requireExactObjectKeys(record, label, expectedKeys) {
  const actual = Object.keys(record).sort();
  const expected = [...expectedKeys].sort();
  if (actual.length !== expected.length || actual.some((key, index) => key !== expected[index])) {
    fail(`${label} keys must equal ${expected.map((key) => JSON.stringify(key)).join(", ")}`);
  }
}

function requireInteger(value, label, minimum, maximum) {
  if (!Number.isInteger(value) || value < minimum || value > maximum) {
    fail(`${label} must be an integer from ${minimum} to ${maximum}`);
  }
  return value;
}

function requireExactInteger(value, label, expected) {
  if (value !== expected) fail(`${label} must equal ${expected}`);
  return value;
}

function requireSame(value, label, expected) {
  if (value !== expected) fail(`${label} does not match the raw ArtDmx frame`);
}

function requireIsoTime(value, label) {
  if (typeof value !== "string" || Number.isNaN(Date.parse(value)) || new Date(value).toISOString() !== value) {
    fail(`${label} must be a canonical ISO-8601 timestamp`);
  }
  return value;
}

function requireTimestampEpoch(isoTime, epochMs, label) {
  const time = requireIsoTime(isoTime, label);
  if (!Number.isSafeInteger(epochMs) || epochMs !== Date.parse(time)) {
    fail(`${label} epoch must equal its ISO-8601 timestamp`);
  }
  return epochMs;
}

function requireWindowsUdpDynamicSourcePortRange(value, label) {
  const range = requireRecord(value, label);
  requireExactObjectKeys(range, label, ["start", "end"]);
  const start = requireInteger(range.start, `${label}.start`, 1, 65535);
  const end = requireInteger(range.end, `${label}.end`, start, 65535);
  return { start, end };
}

export function parseWindowsUdpDynamicPortRange(netshOutput) {
  if (typeof netshOutput !== "string") fail("Windows UDP dynamic port range output must be text");
  // `netsh` localizes the two labels (for example Japanese emits
  // `開始ポート` and `ポート数`), but the command's two numeric colon lines
  // preserve their start/count order. Refuse any unexpected shape instead of
  // guessing an English label or a static Windows default.
  const reportedValues = netshOutput
    .split(/\r?\n/)
    .map((line) => /:\s*(\d+)\s*$/.exec(line)?.[1])
    .filter((value) => value !== undefined);
  if (reportedValues.length !== 2) fail("Windows UDP dynamic port range could not be read from netsh");
  const start = requireInteger(Number(reportedValues[0]), "Windows UDP dynamic source range start", 1, 65535);
  const count = requireInteger(Number(reportedValues[1]), "Windows UDP dynamic source range count", 1, 65535);
  const end = start + count - 1;
  if (!Number.isSafeInteger(end) || end > 65535) {
    fail("Windows UDP dynamic source range exceeds the UDP port space");
  }
  return { start, end };
}

export function readWindowsUdpDynamicPortRange() {
  if (process.platform !== "win32") {
    fail("requires the current Windows UDP dynamic port range; no static default is assumed");
  }
  let output;
  try {
    output = execFileSync("netsh.exe", ["int", "ipv4", "show", "dynamicport", "udp"], {
      encoding: "utf8",
      stdio: ["ignore", "pipe", "pipe"],
      windowsHide: true,
    });
  } catch (error) {
    fail(`could not read the current Windows UDP dynamic port range: ${String(error?.message ?? error)}`);
  }
  return parseWindowsUdpDynamicPortRange(output);
}

function requireLoopbackSource(value, label, dynamicSourcePortRange) {
  if (typeof value !== "string") fail(`${label} must be a loopback IPv4 source`);
  const match = /^127\.0\.0\.1:([0-9]+)$/.exec(value);
  if (!match) fail(`${label} must equal 127.0.0.1:<port>`);
  const port = requireInteger(Number(match[1]), `${label} port`, 1, 65535);
  if (port < dynamicSourcePortRange.start || port > dynamicSourcePortRange.end) {
    fail(`${label} port must fall within the current Windows UDP dynamic source range ${dynamicSourcePortRange.start}-${dynamicSourcePortRange.end}`);
  }
  return value;
}

function fnv1aDigest(data) {
  let digest = 2166136261;
  for (const value of data) {
    digest ^= value;
    digest = Math.imul(digest, 16777619) >>> 0;
  }
  return digest.toString(16).padStart(8, "0");
}

function requireByteArray(value, label, expectedLength) {
  if (!Array.isArray(value) || value.length !== expectedLength) {
    fail(`${label} must contain exactly ${expectedLength} bytes`);
  }
  for (let index = 0; index < value.length; index += 1) {
    requireInteger(value[index], `${label}[${index}]`, 0, 255);
  }
  return value;
}

function readUInt16LE(bytes, offset) {
  return bytes[offset] | (bytes[offset + 1] << 8);
}

function readUInt16BE(bytes, offset) {
  return (bytes[offset] << 8) | bytes[offset + 1];
}

function requireProbePayload(payload, label) {
  requireByteArray(payload, label, DMX_SLOT_COUNT);
  for (let index = 0; index < payload.length; index += 1) {
    const expected = index === DMX_CHANNEL_1_INDEX || index === DMX_CHANNEL_5_INDEX ? 255 : 0;
    if (payload[index] !== expected) fail(`${label}[${index}] must equal ${expected}`);
  }
  requireExactInteger(payload[DMX_CHANNEL_500_INDEX], "U0 payload channel 500", 0);
}

function requireSummary(data, summary, label) {
  const nonZeroChannels = data.reduce((count, value) => count + (value === 0 ? 0 : 1), 0);
  const maxValue = data.reduce((maximum, value) => Math.max(maximum, value), 0);
  const firstIndex = data.findIndex((value) => value !== 0);
  requireExactInteger(summary.nonZeroChannels, `${label}.nonZeroChannels`, nonZeroChannels);
  requireExactInteger(summary.maxValue, `${label}.maxValue`, maxValue);
  const firstNonZero = requireRecord(summary.firstNonZero, `${label}.firstNonZero`);
  requireExactObjectKeys(firstNonZero, `${label}.firstNonZero`, FIRST_NON_ZERO_KEYS);
  requireExactInteger(firstNonZero.channel, `${label}.firstNonZero.channel`, firstIndex + 1);
  requireExactInteger(firstNonZero.value, `${label}.firstNonZero.value`, data[firstIndex]);
}

function requireFrame(frame, label, expected, dynamicSourcePortRange) {
  const record = requireRecord(frame, label);
  requireExactObjectKeys(record, label, FRAME_KEYS);
  const at = requireIsoTime(record.at, `${label}.at`);
  const source = requireLoopbackSource(record.source, `${label}.source`, dynamicSourcePortRange);
  requireExactInteger(record.universe, `${label}.universe`, ARTNET_UNIVERSE);
  const sequence = requireInteger(record.sequence, `${label}.sequence`, 0, 255);
  requireByteArray(record.data, `${label}.data`, DMX_SLOT_COUNT);
  requireProbePayload(record.data, `${label}.data`);
  const digest = fnv1aDigest(record.data);
  requireSame(record.digest, `${label}.digest`, digest);
  requireSummary(record.data, record, label);
  if (expected) {
    requireSame(at, `${label}.at`, expected.at);
    requireSame(source, `${label}.source`, expected.source);
    requireSame(sequence, `${label}.sequence`, expected.sequence);
    requireSame(record.digest, `${label}.digest`, expected.digest);
    for (let index = 0; index < DMX_SLOT_COUNT; index += 1) {
      requireSame(record.data[index], `${label}.data[${index}]`, expected.payload[index]);
    }
  }
  return { at, source, sequence, digest, payload: record.data };
}

function requireTransition(transition, expected) {
  const record = requireRecord(transition, "root.transitions[0]");
  requireExactObjectKeys(record, "root.transitions[0]", TRANSITION_KEYS);
  requireSame(requireIsoTime(record.at, "root.transitions[0].at"), "root.transitions[0].at", expected.at);
  requireExactInteger(record.universe, "root.transitions[0].universe", ARTNET_UNIVERSE);
  requireSame(requireInteger(record.sequence, "root.transitions[0].sequence", 0, 255), "root.transitions[0].sequence", expected.sequence);
  requireSame(record.digest, "root.transitions[0].digest", expected.digest);
  requireByteArray(record.data, "root.transitions[0].data", DMX_SLOT_COUNT);
  requireProbePayload(record.data, "root.transitions[0].data");
  for (let index = 0; index < DMX_SLOT_COUNT; index += 1) {
    requireSame(record.data[index], `root.transitions[0].data[${index}]`, expected.payload[index]);
  }
  requireSummary(record.data, record, "root.transitions[0]");
}

function requireRawArtDmx(rawArtDmx, dynamicSourcePortRange) {
  const record = requireRecord(rawArtDmx, "root.rawArtDmx");
  requireExactObjectKeys(record, "root.rawArtDmx", RAW_ART_DMX_KEYS);
  const at = requireIsoTime(record.at, "root.rawArtDmx.at");
  const receivedAtEpochMs = requireTimestampEpoch(at, record.receivedAtEpochMs, "root.rawArtDmx.at");
  const source = requireLoopbackSource(record.source, "root.rawArtDmx.source", dynamicSourcePortRange);
  const bytes = requireByteArray(record.bytes, "root.rawArtDmx.bytes", ART_DMX_PACKET_LENGTH);
  const artNetId = [65, 114, 116, 45, 78, 101, 116, 0];
  for (let index = 0; index < artNetId.length; index += 1) {
    requireExactInteger(bytes[index], `root.rawArtDmx.bytes[${index}]`, artNetId[index]);
  }
  requireExactInteger(readUInt16LE(bytes, 8), "root.rawArtDmx.OpCode", ART_DMX_OPCODE);
  requireExactInteger(readUInt16BE(bytes, 10), "root.rawArtDmx.protocolVersion", ARTNET_PROTOCOL_VERSION);
  const sequence = requireInteger(bytes[12], "root.rawArtDmx.sequence", 0, 255);
  requireExactInteger(bytes[13], "root.rawArtDmx.physical", 0);
  requireExactInteger(readUInt16LE(bytes, 14), "root.rawArtDmx.universe", ARTNET_UNIVERSE);
  requireExactInteger(readUInt16BE(bytes, 16), "root.rawArtDmx.length", DMX_SLOT_COUNT);
  const payload = bytes.slice(18);
  requireProbePayload(payload, "root.rawArtDmx.payload");
  return { at, receivedAtEpochMs, source, sequence, payload, digest: fnv1aDigest(payload) };
}

/**
 * Verify only monitor evidence schema v2. Schema v1 intentionally fails: it
 * lacked one raw datagram and cannot prove the encoded ArtDmx header.
 */
function verifyDsf2026ArtNetProbeEvidenceWithDynamicSourceRange(evidence, dynamicSourcePortRange) {
  const verifiedDynamicSourcePortRange = requireWindowsUdpDynamicSourcePortRange(
    dynamicSourcePortRange,
    "Windows UDP dynamic source range",
  );
  const record = requireRecord(evidence, "root");
  requireExactObjectKeys(record, "root", ROOT_KEYS);
  requireExactInteger(record.schema, "root.schema", ARTNET_MONITOR_EVIDENCE_SCHEMA);
  if (record.product !== MONITOR_PRODUCT) fail("root.product must be the Art-Net monitor product");
  const startedAtEpochMs = requireTimestampEpoch(record.startedAt, record.startedAtEpochMs, "root.startedAt");
  const endedAtEpochMs = requireTimestampEpoch(record.endedAt, record.endedAtEpochMs, "root.endedAt");
  if (startedAtEpochMs > endedAtEpochMs) fail("root capture end must not precede start");
  if (!new Set(["duration", "SIGINT", "SIGTERM"]).has(record.reason)) {
    fail("root.reason must be duration, SIGINT, or SIGTERM");
  }

  const listen = requireRecord(record.listen, "root.listen");
  requireExactObjectKeys(listen, "root.listen", LISTEN_KEYS);
  if (listen.address !== "0.0.0.0") fail("root.listen.address must equal 0.0.0.0");
  requireExactInteger(listen.port, "root.listen.port", ARTNET_PORT);
  const monitorUrl = typeof record.monitorUrl === "string" ? record.monitorUrl : "";
  let parsedMonitorUrl;
  try {
    parsedMonitorUrl = new URL(monitorUrl);
  } catch {
    fail("root.monitorUrl must be a URL");
  }
  if (parsedMonitorUrl.protocol !== "http:" || parsedMonitorUrl.hostname !== "127.0.0.1" || parsedMonitorUrl.pathname !== "/") {
    fail("root.monitorUrl must be a loopback HTTP root URL");
  }
  requireInteger(Number(parsedMonitorUrl.port), "root.monitorUrl port", 1, 65535);

  const capture = requireRecord(record.capture, "root.capture");
  requireExactObjectKeys(capture, "root.capture", CAPTURE_KEYS);
  if (capture.transitionData !== true) fail("root.capture.transitionData must be true");
  requireInteger(capture.maxTransitions, "root.capture.maxTransitions", 1, Number.MAX_SAFE_INTEGER);

  requireExactInteger(record.totalDatagrams, "root.totalDatagrams", 1);
  requireExactInteger(record.artDmxFrames, "root.artDmxFrames", 1);
  requireExactInteger(record.rejectedDatagrams, "root.rejectedDatagrams", 0);
  requireExactInteger(record.changedFrames, "root.changedFrames", 1);
  requireExactInteger(record.sequenceDiscontinuities, "root.sequenceDiscontinuities", 0);
  requireExactInteger(record.maxGapMs, "root.maxGapMs", 0);

  const raw = requireRawArtDmx(record.rawArtDmx, verifiedDynamicSourcePortRange);
  if (raw.receivedAtEpochMs < startedAtEpochMs || raw.receivedAtEpochMs > endedAtEpochMs) {
    fail("raw ArtDmx receipt must be within the monitor capture window");
  }
  requireFrame(record.lastFrame, "root.lastFrame", raw, verifiedDynamicSourcePortRange);
  requireSame(requireIsoTime(record.firstFrameAt, "root.firstFrameAt"), "root.firstFrameAt", raw.at);
  requireSame(requireIsoTime(record.lastFrameAt, "root.lastFrameAt"), "root.lastFrameAt", raw.at);

  const sources = requireRecord(record.sources, "root.sources");
  requireExactObjectKeys(sources, "root.sources", [raw.source]);
  requireExactInteger(sources[raw.source], `root.sources[${JSON.stringify(raw.source)}]`, 1);

  const universeKey = String(ARTNET_UNIVERSE);
  const universes = requireRecord(record.universes, "root.universes");
  requireExactObjectKeys(universes, "root.universes", [universeKey]);
  const universe = requireRecord(universes[universeKey], "root.universes.U0");
  requireExactObjectKeys(universe, "root.universes.U0", UNIVERSE_KEYS);
  requireExactInteger(universe.frames, "root.universes.U0.frames", 1);
  requireExactInteger(universe.changedFrames, "root.universes.U0.changedFrames", 1);
  requireExactInteger(universe.protocolVersion, "root.universes.U0.protocolVersion", ARTNET_PROTOCOL_VERSION);
  requireExactInteger(universe.physical, "root.universes.U0.physical", 0);
  requireExactInteger(universe.dataLength, "root.universes.U0.dataLength", DMX_SLOT_COUNT);
  requireSame(universe.lastSequence, "root.universes.U0.lastSequence", raw.sequence);
  requireSame(universe.lastDigest, "root.universes.U0.lastDigest", raw.digest);
  requireSame(requireIsoTime(universe.lastFrameAt, "root.universes.U0.lastFrameAt"), "root.universes.U0.lastFrameAt", raw.at);
  requireTimestampEpoch(universe.lastFrameAt, universe.lastFrameEpochMs, "root.universes.U0.lastFrameAt");
  requireSame(universe.lastFrameEpochMs, "root.universes.U0.lastFrameEpochMs", raw.receivedAtEpochMs);
  requireSummary(raw.payload, universe, "root.universes.U0");

  const lastFrames = requireRecord(record.lastFrames, "root.lastFrames");
  requireExactObjectKeys(lastFrames, "root.lastFrames", [universeKey]);
  requireFrame(lastFrames[universeKey], "root.lastFrames.U0", raw, verifiedDynamicSourcePortRange);

  const streamKey = `${raw.source}/u${ARTNET_UNIVERSE}`;
  const streams = requireRecord(record.streams, "root.streams");
  requireExactObjectKeys(streams, "root.streams", [streamKey]);
  const stream = requireRecord(streams[streamKey], "root.streams.U0");
  requireExactObjectKeys(stream, "root.streams.U0", STREAM_KEYS);
  requireExactInteger(stream.frames, "root.streams.U0.frames", 1);
  requireSame(stream.lastSequence, "root.streams.U0.lastSequence", raw.sequence);
  requireSame(requireIsoTime(stream.lastFrameAt, "root.streams.U0.lastFrameAt"), "root.streams.U0.lastFrameAt", raw.at);
  requireTimestampEpoch(stream.lastFrameAt, stream.lastFrameEpochMs, "root.streams.U0.lastFrameAt");
  requireSame(stream.lastFrameEpochMs, "root.streams.U0.lastFrameEpochMs", raw.receivedAtEpochMs);

  if (!Array.isArray(record.transitions) || record.transitions.length !== 1) {
    fail("root.transitions must contain exactly one captured transition");
  }
  requireTransition(record.transitions[0], raw);

  return {
    schema: ARTNET_MONITOR_EVIDENCE_SCHEMA,
    contract: "dsf2026-artnet-acceptance-probe.v1",
    verified: true,
    observed: {
      source: raw.source,
      sequence: raw.sequence,
      physical: 0,
      protocolVersion: ARTNET_PROTOCOL_VERSION,
      universe: ARTNET_UNIVERSE,
      datagramBytes: ART_DMX_PACKET_LENGTH,
      payloadLength: DMX_SLOT_COUNT,
      nonZeroChannels: [1, 5],
      receivedAt: raw.at,
      windowsUdpDynamicSourcePortRange: verifiedDynamicSourcePortRange,
    },
    senderProcessProvenance: "not-proven-by-udp-evidence",
    limitation: "This UDP receipt proves only a loopback datagram matching the contract; it does not identify or prove the sender process was Syndocal.",
  };
}

/**
 * Validate evidence against the machine's current configured IPv4 UDP dynamic
 * source-port range. The evidence schema deliberately records only the
 * observed source; it does not claim a portable Windows default range.
 */
export function verifyDsf2026ArtNetProbeEvidence(evidence) {
  return verifyDsf2026ArtNetProbeEvidenceWithDynamicSourceRange(
    evidence,
    readWindowsUdpDynamicPortRange(),
  );
}

function createIntegratedValidEvidence() {
  const startedAtEpochMs = Date.parse("2026-08-30T00:00:00.000Z");
  const receivedAtEpochMs = Date.parse("2026-08-30T00:00:01.000Z");
  const endedAtEpochMs = Date.parse("2026-08-30T00:00:02.000Z");
  const state = createArtNetMonitorState({
    artnetPort: ARTNET_PORT,
    httpPort: 6455,
    captureChanges: true,
    maxTransitions: 1,
  }, startedAtEpochMs);
  const payload = Buffer.alloc(DMX_SLOT_COUNT);
  payload[DMX_CHANNEL_1_INDEX] = 255;
  payload[DMX_CHANNEL_5_INDEX] = 255;
  recordArtNetMonitorDatagram(
    state,
    buildArtDmxForSelfTest({ universe: 0, sequence: 1, values: payload }),
    { address: "127.0.0.1", port: 50000 },
    receivedAtEpochMs,
  );
  finishArtNetMonitorCapture(state, "duration", endedAtEpochMs);
  return serializeArtNetMonitorEvidence(state);
}

function expectRejected(action, expectedText) {
  try {
    action();
  } catch (error) {
    if (String(error?.message ?? error).includes(expectedText)) return;
    throw error;
  }
  throw new Error(`Expected verifier to reject: ${expectedText}`);
}

export function runSelfTest() {
  let assertions = 0;
  // Keep the verifier self-test deterministic: these are injected test
  // bounds, not a claim about Windows defaults. Real `--evidence` runs read
  // the current machine configuration through netsh above.
  const selfTestDynamicSourceRange = Object.freeze({ start: 50000, end: 50000 });
  const verifyDsf2026ArtNetProbeEvidence = (evidence) =>
    verifyDsf2026ArtNetProbeEvidenceWithDynamicSourceRange(
      evidence,
      selfTestDynamicSourceRange,
    );
  const valid = createIntegratedValidEvidence();
  const verified = verifyDsf2026ArtNetProbeEvidence(valid);
  if (!verified.verified || verified.senderProcessProvenance !== "not-proven-by-udp-evidence") {
    throw new Error("Integrated monitor serializer evidence self-test failed");
  }
  assertions += 1;

  const parsedDynamicRange = parseWindowsUdpDynamicPortRange(
    "Protocol udp Dynamic Port Range\r\nStart Port      : 53000\r\nNumber of Ports : 100\r\n",
  );
  if (parsedDynamicRange.start !== 53000 || parsedDynamicRange.end !== 53099) {
    throw new Error("Windows UDP dynamic-range parser self-test failed");
  }
  assertions += 1;

  const unsupportedSchema = structuredClone(valid);
  unsupportedSchema.schema = ARTNET_MONITOR_EVIDENCE_SCHEMA - 1;
  expectRejected(() => verifyDsf2026ArtNetProbeEvidence(unsupportedSchema), "root.schema");
  assertions += 1;

  const extraRootKey = structuredClone(valid);
  extraRootKey.unreviewed = true;
  expectRejected(() => verifyDsf2026ArtNetProbeEvidence(extraRootKey), "root keys");
  assertions += 1;

  const wrongListenPort = structuredClone(valid);
  wrongListenPort.listen.port = 6455;
  expectRejected(() => verifyDsf2026ArtNetProbeEvidence(wrongListenPort), "root.listen.port");
  assertions += 1;

  const extraListenKey = structuredClone(valid);
  extraListenKey.listen.scope = "all";
  expectRejected(() => verifyDsf2026ArtNetProbeEvidence(extraListenKey), "root.listen keys");
  assertions += 1;

  const extraRawKey = structuredClone(valid);
  extraRawKey.rawArtDmx.extra = true;
  expectRejected(() => verifyDsf2026ArtNetProbeEvidence(extraRawKey), "root.rawArtDmx keys");
  assertions += 1;

  const extraLastFrameKey = structuredClone(valid);
  extraLastFrameKey.lastFrame.unreviewed = true;
  expectRejected(() => verifyDsf2026ArtNetProbeEvidence(extraLastFrameKey), "root.lastFrame keys");
  assertions += 1;

  const extraUniverseKey = structuredClone(valid);
  extraUniverseKey.universes["0"].unreviewed = true;
  expectRejected(() => verifyDsf2026ArtNetProbeEvidence(extraUniverseKey), "root.universes.U0 keys");
  assertions += 1;

  const extraStreamKey = structuredClone(valid);
  extraStreamKey.streams["127.0.0.1:50000/u0"].unreviewed = true;
  expectRejected(() => verifyDsf2026ArtNetProbeEvidence(extraStreamKey), "root.streams.U0 keys");
  assertions += 1;

  const extraTransitionKey = structuredClone(valid);
  extraTransitionKey.transitions[0].unreviewed = true;
  expectRejected(() => verifyDsf2026ArtNetProbeEvidence(extraTransitionKey), "root.transitions[0] keys");
  assertions += 1;

  const nonLoopbackSource = structuredClone(valid);
  nonLoopbackSource.rawArtDmx.source = "192.168.1.20:50000";
  expectRejected(() => verifyDsf2026ArtNetProbeEvidence(nonLoopbackSource), "root.rawArtDmx.source");
  assertions += 1;

  const outOfRangeSourcePort = structuredClone(valid);
  outOfRangeSourcePort.rawArtDmx.source = "127.0.0.1:65536";
  expectRejected(() => verifyDsf2026ArtNetProbeEvidence(outOfRangeSourcePort), "root.rawArtDmx.source port");
  assertions += 1;

  const lowSourcePort = structuredClone(valid);
  lowSourcePort.rawArtDmx.source = "127.0.0.1:1024";
  expectRejected(
    () => verifyDsf2026ArtNetProbeEvidence(lowSourcePort),
    "current Windows UDP dynamic source range",
  );
  assertions += 1;

  const rawArtNetId = structuredClone(valid);
  rawArtNetId.rawArtDmx.bytes[0] = 0;
  expectRejected(() => verifyDsf2026ArtNetProbeEvidence(rawArtNetId), "root.rawArtDmx.bytes[0]");
  assertions += 1;

  const rawOpcode = structuredClone(valid);
  rawOpcode.rawArtDmx.bytes[9] = 0;
  expectRejected(() => verifyDsf2026ArtNetProbeEvidence(rawOpcode), "root.rawArtDmx.OpCode");
  assertions += 1;

  const rawProtocol = structuredClone(valid);
  rawProtocol.rawArtDmx.bytes[11] = 13;
  expectRejected(() => verifyDsf2026ArtNetProbeEvidence(rawProtocol), "root.rawArtDmx.protocolVersion");
  assertions += 1;

  const rawUniverse = structuredClone(valid);
  rawUniverse.rawArtDmx.bytes[14] = 1;
  expectRejected(() => verifyDsf2026ArtNetProbeEvidence(rawUniverse), "root.rawArtDmx.universe");
  assertions += 1;

  const rawLength = structuredClone(valid);
  rawLength.rawArtDmx.bytes[17] = 2;
  expectRejected(() => verifyDsf2026ArtNetProbeEvidence(rawLength), "root.rawArtDmx.length");
  assertions += 1;

  const rawPacketLength = structuredClone(valid);
  rawPacketLength.rawArtDmx.bytes.pop();
  expectRejected(() => verifyDsf2026ArtNetProbeEvidence(rawPacketLength), "root.rawArtDmx.bytes must contain exactly 530 bytes");
  assertions += 1;

  const rawPhysical = structuredClone(valid);
  rawPhysical.rawArtDmx.bytes[13] = 1;
  expectRejected(() => verifyDsf2026ArtNetProbeEvidence(rawPhysical), "root.rawArtDmx.physical");
  assertions += 1;

  const rawChannel1 = structuredClone(valid);
  rawChannel1.rawArtDmx.bytes[18 + DMX_CHANNEL_1_INDEX] = 0;
  expectRejected(() => verifyDsf2026ArtNetProbeEvidence(rawChannel1), "root.rawArtDmx.payload[0]");
  assertions += 1;

  const rawChannel5 = structuredClone(valid);
  rawChannel5.rawArtDmx.bytes[18 + DMX_CHANNEL_5_INDEX] = 0;
  expectRejected(() => verifyDsf2026ArtNetProbeEvidence(rawChannel5), "root.rawArtDmx.payload[4]");
  assertions += 1;

  const rawChannel500 = structuredClone(valid);
  rawChannel500.rawArtDmx.bytes[18 + DMX_CHANNEL_500_INDEX] = 1;
  expectRejected(() => verifyDsf2026ArtNetProbeEvidence(rawChannel500), "root.rawArtDmx.payload[499]");
  assertions += 1;

  const rawStrayChannel = structuredClone(valid);
  rawStrayChannel.rawArtDmx.bytes[18 + 7] = 1;
  expectRejected(() => verifyDsf2026ArtNetProbeEvidence(rawStrayChannel), "root.rawArtDmx.payload[7]");
  assertions += 1;

  const rawEpochMismatch = structuredClone(valid);
  rawEpochMismatch.rawArtDmx.receivedAtEpochMs += 1;
  expectRejected(() => verifyDsf2026ArtNetProbeEvidence(rawEpochMismatch), "root.rawArtDmx.at epoch");
  assertions += 1;

  const endedEpochMismatch = structuredClone(valid);
  endedEpochMismatch.endedAtEpochMs += 1;
  expectRejected(() => verifyDsf2026ArtNetProbeEvidence(endedEpochMismatch), "root.endedAt epoch");
  assertions += 1;

  const summarySequenceMismatch = structuredClone(valid);
  summarySequenceMismatch.lastFrame.sequence = 2;
  expectRejected(() => verifyDsf2026ArtNetProbeEvidence(summarySequenceMismatch), "root.lastFrame.sequence");
  assertions += 1;

  const summarySourceMismatch = structuredClone(valid);
  summarySourceMismatch.lastFrame = structuredClone(summarySourceMismatch.lastFrame);
  summarySourceMismatch.lastFrame.source = "127.0.0.1:50001";
  expectRejected(() => verifyDsf2026ArtNetProbeEvidence(summarySourceMismatch), "root.lastFrame.source");
  assertions += 1;

  const summaryDigestMismatch = structuredClone(valid);
  summaryDigestMismatch.lastFrame = structuredClone(summaryDigestMismatch.lastFrame);
  summaryDigestMismatch.lastFrame.digest = "00000000";
  expectRejected(() => verifyDsf2026ArtNetProbeEvidence(summaryDigestMismatch), "root.lastFrame.digest");
  assertions += 1;

  const universeDigestMismatch = structuredClone(valid);
  universeDigestMismatch.universes["0"].lastDigest = "00000000";
  expectRejected(() => verifyDsf2026ArtNetProbeEvidence(universeDigestMismatch), "root.universes.U0.lastDigest");
  assertions += 1;

  const summaryPayloadMismatch = structuredClone(valid);
  summaryPayloadMismatch.lastFrames["0"] = structuredClone(summaryPayloadMismatch.lastFrames["0"]);
  summaryPayloadMismatch.lastFrames["0"].data[7] = 1;
  expectRejected(() => verifyDsf2026ArtNetProbeEvidence(summaryPayloadMismatch), "root.lastFrames.U0.data[7]");
  assertions += 1;

  const epochMismatch = structuredClone(valid);
  epochMismatch.streams["127.0.0.1:50000/u0"].lastFrameEpochMs += 1;
  expectRejected(() => verifyDsf2026ArtNetProbeEvidence(epochMismatch), "root.streams.U0.lastFrameAt epoch");
  assertions += 1;

  const transitionTimestampMismatch = structuredClone(valid);
  transitionTimestampMismatch.transitions[0].at = "2026-08-30T00:00:01.001Z";
  expectRejected(() => verifyDsf2026ArtNetProbeEvidence(transitionTimestampMismatch), "root.transitions[0].at");
  assertions += 1;

  const multipleDatagrams = structuredClone(valid);
  multipleDatagrams.totalDatagrams = 2;
  expectRejected(() => verifyDsf2026ArtNetProbeEvidence(multipleDatagrams), "root.totalDatagrams");
  assertions += 1;

  const multipleArtDmxFrames = structuredClone(valid);
  multipleArtDmxFrames.artDmxFrames = 2;
  expectRejected(() => verifyDsf2026ArtNetProbeEvidence(multipleArtDmxFrames), "root.artDmxFrames");
  assertions += 1;

  const rejectedDatagrams = structuredClone(valid);
  rejectedDatagrams.rejectedDatagrams = 1;
  expectRejected(() => verifyDsf2026ArtNetProbeEvidence(rejectedDatagrams), "root.rejectedDatagrams");
  assertions += 1;

  const changedFrames = structuredClone(valid);
  changedFrames.changedFrames = 0;
  expectRejected(() => verifyDsf2026ArtNetProbeEvidence(changedFrames), "root.changedFrames");
  assertions += 1;

  const discontinuity = structuredClone(valid);
  discontinuity.sequenceDiscontinuities = 1;
  expectRejected(() => verifyDsf2026ArtNetProbeEvidence(discontinuity), "root.sequenceDiscontinuities");
  assertions += 1;

  const gap = structuredClone(valid);
  gap.maxGapMs = 1;
  expectRejected(() => verifyDsf2026ArtNetProbeEvidence(gap), "root.maxGapMs");
  assertions += 1;

  const missingRawFrame = structuredClone(valid);
  missingRawFrame.rawArtDmx = null;
  expectRejected(() => verifyDsf2026ArtNetProbeEvidence(missingRawFrame), "root.rawArtDmx must be an object");
  assertions += 1;

  console.log(`DSF2026 Art-Net probe evidence verifier self-test: ${assertions} assertions passed (39 negative, 2 positive)`);
}

function parseArgs(argv) {
  const options = { evidencePath: "", selfTest: false };
  for (let index = 0; index < argv.length; index += 1) {
    const argument = argv[index];
    if (argument === "--self-test") {
      options.selfTest = true;
      continue;
    }
    if (argument === "--evidence") {
      index += 1;
      if (index >= argv.length || argv[index].startsWith("-")) throw new Error("Missing evidence path after --evidence");
      options.evidencePath = argv[index];
      continue;
    }
    throw new Error(`Unknown argument: ${argument}`);
  }
  if (options.selfTest === (options.evidencePath.length > 0)) {
    throw new Error("Usage: node qa/harnesses/verify-dsf2026-artnet-probe-evidence.mjs --self-test | --evidence <artnet-monitor-v2.json>");
  }
  return options;
}

function main() {
  const options = parseArgs(process.argv.slice(2));
  if (options.selfTest) return runSelfTest();
  const evidencePath = path.resolve(options.evidencePath);
  let evidence;
  try {
    evidence = JSON.parse(fs.readFileSync(evidencePath, "utf8"));
  } catch (error) {
    throw new Error(`Could not read evidence JSON '${evidencePath}': ${error.message}`);
  }
  console.log(JSON.stringify({ evidence: evidencePath, ...verifyDsf2026ArtNetProbeEvidence(evidence) }));
}

const invokedDirectly = process.argv[1] && import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href;
if (invokedDirectly) {
  try {
    main();
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  }
}
