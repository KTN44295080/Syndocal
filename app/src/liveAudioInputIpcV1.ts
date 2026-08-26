import type { LiveAudioInputBackendId, LiveAudioInputStartRequest } from "./types";

/** The only renderer/native request schema currently admitted by this module. */
export const LIVE_AUDIO_INPUT_IPC_V1_SCHEMA_VERSION = 1 as const;

const MIN_SAMPLE_RATE_HZ = 8_000;
const MAX_SAMPLE_RATE_HZ = 768_000;
const MAX_STREAM_CHANNELS = 1_024;
const MAX_BUFFER_FRAMES = 8_192;
const MAX_BUFFER_AGE_MS = 200;
const MAX_TEXT_LENGTH = 512;

const WASAPI_SAMPLE_FORMATS = new Set([
  "f32",
  "i16",
  "u16",
  "f64",
  "i32",
  "i24",
  "i8",
  "u8",
  "i64",
  "u32",
  "u64",
]);

const ASIO_SAMPLE_FORMATS = new Set(["f32", "i16", "i24", "i32", "f64"]);

export type LiveAudioInputChannelMixWireV1 =
  | { mode: "averageAll" }
  | { mode: "single"; channelIndex: number }
  | { mode: "stereoPair"; leftChannelIndex: number; rightChannelIndex: number };

/** Rust's camelCase enum spelling; the persisted/frontend id stays snake_case. */
export type LiveAudioInputBackendWireV1 = "wasapiShared" | "asio";

export interface LiveAudioInputBackendRequestV1 {
  schemaVersion: typeof LIVE_AUDIO_INPUT_IPC_V1_SCHEMA_VERSION;
  backend: LiveAudioInputBackendWireV1;
}

export interface LiveAudioInputCapabilitiesRequestV1 extends LiveAudioInputBackendRequestV1 {
  deviceId: string | null;
  sampleRate: number | null;
}

export interface LiveAudioInputStartRequestV1 {
  schemaVersion: typeof LIVE_AUDIO_INPUT_IPC_V1_SCHEMA_VERSION;
  backend: LiveAudioInputBackendWireV1;
  deviceId: string | null;
  sampleRate: number | null;
  streamChannels: number | null;
  sampleFormat: string | null;
  bufferFrames: number | null;
  channelMix: LiveAudioInputChannelMixWireV1;
}

export interface LiveAudioInputBackendArgsV1 extends Record<string, unknown> {
  request: LiveAudioInputBackendRequestV1;
}

export interface LiveAudioInputCapabilitiesArgsV1 extends Record<string, unknown> {
  request: LiveAudioInputCapabilitiesRequestV1;
}

export interface LiveAudioInputStartArgsV1 extends Record<string, unknown> {
  request: LiveAudioInputStartRequestV1;
}

export type LiveAudioInputIpcV1RejectionCode =
  | "INVALID_SHAPE"
  | "INVALID_BACKEND"
  | "INVALID_DEVICE_ID"
  | "INVALID_SAMPLE_RATE"
  | "INVALID_STREAM_CHANNELS"
  | "INVALID_SAMPLE_FORMAT"
  | "INVALID_BUFFER_FRAMES"
  | "INVALID_CHANNEL_MIX";

/** Explicit failure instead of silently defaulting a missing/ambiguous field. */
export class LiveAudioInputIpcV1Error extends Error {
  readonly code: LiveAudioInputIpcV1RejectionCode;

  constructor(code: LiveAudioInputIpcV1RejectionCode, message: string) {
    super(`Live audio IPC v1 ${code}: ${message}`);
    this.name = "LiveAudioInputIpcV1Error";
    this.code = code;
  }
}

type UnknownRecord = Record<string, unknown>;

const isRecord = (value: unknown): value is UnknownRecord =>
  typeof value === "object" && value !== null && !Array.isArray(value);

const own = (value: UnknownRecord, key: string): boolean =>
  Object.prototype.hasOwnProperty.call(value, key);

const exactKeys = (value: unknown, keys: readonly string[], label: string): UnknownRecord => {
  if (!isRecord(value)) {
    throw new LiveAudioInputIpcV1Error("INVALID_SHAPE", `${label} must be an object.`);
  }
  const allowed = new Set(keys);
  const missing = keys.find((key) => !own(value, key));
  const unknown = Object.keys(value).find((key) => !allowed.has(key));
  if (missing || unknown || Object.keys(value).length !== keys.length) {
    throw new LiveAudioInputIpcV1Error(
      "INVALID_SHAPE",
      `${label} must contain exactly ${keys.join(", ")}; ${missing ? `missing ${missing}` : `unknown ${unknown ?? "field"}`}.`,
    );
  }
  return value;
};

const invalid = (code: LiveAudioInputIpcV1RejectionCode, label: string, detail: string): never => {
  throw new LiveAudioInputIpcV1Error(code, `${label} ${detail}.`);
};

const backend = (value: unknown): LiveAudioInputBackendId => {
  if (value === "wasapi_shared" || value === "asio") return value;
  return invalid("INVALID_BACKEND", "backend", "must be wasapi_shared or asio");
};

const backendWire = (value: LiveAudioInputBackendId): LiveAudioInputBackendWireV1 =>
  value === "wasapi_shared" ? "wasapiShared" : "asio";

const safeInteger = (value: unknown, minimum: number, maximum: number): value is number =>
  typeof value === "number" && Number.isSafeInteger(value) && value >= minimum && value <= maximum;

const text = (value: unknown, label: string, allowEmpty = false): string => {
  if (
    typeof value !== "string" ||
    (!allowEmpty && value.length === 0) ||
    value.length > MAX_TEXT_LENGTH ||
    value !== value.trim() ||
    /[\u0000-\u001f\u007f]/.test(value)
  ) {
    return invalid("INVALID_SHAPE", label, "must be an exact printable string");
  }
  return value;
};

const nullableInteger = (
  value: unknown,
  label: string,
  minimum: number,
  maximum: number,
  code: LiveAudioInputIpcV1RejectionCode,
): number | null => {
  if (value === null) return null;
  if (!safeInteger(value, minimum, maximum)) return invalid(code, label, `must be null or an integer in ${minimum}..${maximum}`);
  return value;
};

const nullableSampleRate = (value: unknown, label: string): number | null =>
  nullableInteger(value, label, MIN_SAMPLE_RATE_HZ, MAX_SAMPLE_RATE_HZ, "INVALID_SAMPLE_RATE");

const nullableStreamChannels = (value: unknown, label: string): number | null =>
  nullableInteger(value, label, 1, MAX_STREAM_CHANNELS, "INVALID_STREAM_CHANNELS");

const nullableBufferFrames = (value: unknown, label: string): number | null =>
  nullableInteger(value, label, 1, MAX_BUFFER_FRAMES, "INVALID_BUFFER_FRAMES");

const validatedSampleFormat = (
  value: unknown,
  selectedBackend: LiveAudioInputBackendId,
  label: string,
): string | null => {
  if (value === null) return null;
  const format = text(value, label);
  const supported = selectedBackend === "asio" ? ASIO_SAMPLE_FORMATS : WASAPI_SAMPLE_FORMATS;
  if (!supported.has(format)) return invalid("INVALID_SAMPLE_FORMAT", label, `is not supported by ${selectedBackend}`);
  return format;
};

/** Convert the persisted/internal snake_case mix into the only admitted wire spelling. */
export const mapLiveAudioChannelMixToWireV1 = (
  value: unknown,
  streamChannels: number | null,
): LiveAudioInputChannelMixWireV1 => {
  if (streamChannels !== null && !safeInteger(streamChannels, 1, MAX_STREAM_CHANNELS)) {
    return invalid("INVALID_STREAM_CHANNELS", "streamChannels", `must be null or an integer in 1..${MAX_STREAM_CHANNELS}`);
  }
  const mix = exactKeys(value, ["mode", ...(isRecord(value) && value.mode === "single" ? ["channel_index"] : []), ...(isRecord(value) && value.mode === "stereo_pair" ? ["left_channel_index", "right_channel_index"] : [])], "channel mix");
  if (mix.mode === "average_all") {
    exactKeys(mix, ["mode"], "average_all channel mix");
    return { mode: "averageAll" };
  }
  if (mix.mode === "single") {
    exactKeys(mix, ["mode", "channel_index"], "single channel mix");
    if (streamChannels === null || !safeInteger(mix.channel_index, 0, streamChannels - 1)) {
      return invalid("INVALID_CHANNEL_MIX", "single channel mix", "must name one current channel");
    }
    return { mode: "single", channelIndex: mix.channel_index };
  }
  if (mix.mode === "stereo_pair") {
    exactKeys(mix, ["mode", "left_channel_index", "right_channel_index"], "stereo_pair channel mix");
    if (
      streamChannels === null ||
      !safeInteger(mix.left_channel_index, 0, streamChannels - 1) ||
      !safeInteger(mix.right_channel_index, 0, streamChannels - 1) ||
      mix.left_channel_index === mix.right_channel_index
    ) {
      return invalid("INVALID_CHANNEL_MIX", "stereo_pair channel mix", "must name two distinct current channels");
    }
    return {
      mode: "stereoPair",
      leftChannelIndex: mix.left_channel_index,
      rightChannelIndex: mix.right_channel_index,
    };
  }
  return invalid("INVALID_CHANNEL_MIX", "channel mix mode", "must be average_all, single, or stereo_pair");
};

/** Build the versioned backend/device-query outer args. */
export const buildLiveAudioInputBackendArgsV1 = (
  value: LiveAudioInputBackendId,
): LiveAudioInputBackendArgsV1 => ({
  request: {
    schemaVersion: LIVE_AUDIO_INPUT_IPC_V1_SCHEMA_VERSION,
    backend: backendWire(backend(value)),
  },
});

/** Build capabilities args with explicit nullable deviceId/sampleRate keys. */
export const buildLiveAudioInputCapabilitiesArgsV1 = (
  value: { backend: LiveAudioInputBackendId; deviceId: string | null; sampleRate: number | null },
): LiveAudioInputCapabilitiesArgsV1 => {
  const request = exactKeys(value, ["backend", "deviceId", "sampleRate"], "capabilities request");
  const selectedBackend = backend(request.backend);
  const deviceId = request.deviceId === null
    ? null
    : text(request.deviceId, "deviceId");
  const sampleRate = nullableSampleRate(request.sampleRate, "sampleRate");
  return {
    request: {
      schemaVersion: LIVE_AUDIO_INPUT_IPC_V1_SCHEMA_VERSION,
      backend: backendWire(selectedBackend),
      deviceId,
      sampleRate,
    },
  };
};

/** Build start args from the existing internal snake_case request shape. */
export const buildLiveAudioInputStartArgsV1 = (
  value: LiveAudioInputStartRequest,
): LiveAudioInputStartArgsV1 => {
  const request = exactKeys(
    value,
    ["backend", "device_id", "sample_rate", "stream_channels", "sample_format", "buffer_frames", "channel_mix"],
    "start request",
  );
  const selectedBackend = backend(request.backend);
  const deviceId = request.device_id === null ? null : text(request.device_id, "device_id");
  const sampleRate = nullableSampleRate(request.sample_rate, "sample_rate");
  const streamChannels = nullableStreamChannels(request.stream_channels, "stream_channels");
  const sampleFormat = validatedSampleFormat(request.sample_format, selectedBackend, "sample_format");
  const bufferFrames = nullableBufferFrames(request.buffer_frames, "buffer_frames");
  if (
    sampleRate !== null &&
    bufferFrames !== null &&
    bufferFrames * 1_000 > sampleRate * MAX_BUFFER_AGE_MS
  ) {
    return invalid(
      "INVALID_BUFFER_FRAMES",
      "buffer_frames",
      `would exceed the ${MAX_BUFFER_AGE_MS} ms watchdog-safe age at sample_rate`,
    );
  }
  return {
    request: {
      schemaVersion: LIVE_AUDIO_INPUT_IPC_V1_SCHEMA_VERSION,
      backend: backendWire(selectedBackend),
      deviceId,
      sampleRate,
      streamChannels,
      sampleFormat,
      bufferFrames,
      channelMix: mapLiveAudioChannelMixToWireV1(request.channel_mix, streamChannels),
    },
  };
};
