import type {
  LiveAudioChannelMix,
  LiveAudioInputBackendId,
  LiveAudioInputCapabilities,
  LiveAudioInputDeviceSummary,
  LiveAudioInputStartRequest,
} from "./types";

/**
 * Machine-local only. This module deliberately imports no platform storage API:
 * the application supplies a storage port, and this module owns the strict
 * read/write contract so malformed data is preserved for inspection, write
 * failures are reported instead of claimed as saved, and nothing is ever
 * deleted behind the operator's back.
 */
export const LIVE_AUDIO_INPUT_SELECTION_STORAGE_KEY =
  "syndocal.live-audio-input-selection.v1";
export const LIVE_AUDIO_INPUT_SELECTION_STORAGE_SCHEMA_VERSION = 1;

const MAX_SAMPLE_RATE_HZ = 768_000;
const MIN_SAMPLE_RATE_HZ = 8_000;
const MAX_STREAM_CHANNELS = 1_024;
const MAX_BUFFER_FRAMES = 8_192;
const MAX_STABLE_IDENTITY_TEXT_LENGTH = 512;

const BACKEND_LABELS: Record<LiveAudioInputBackendId, string> = {
  wasapi_shared: "WASAPI",
  asio: "ASIO",
};

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

export type LiveAudioInputSelectionStorageReasonCode =
  | "EMPTY"
  | "INVALID_JSON"
  | "DUPLICATE_KEY"
  | "ROOT_NOT_OBJECT"
  | "MISSING_REQUIRED_FIELD"
  | "UNEXPECTED_FIELD"
  | "SCHEMA_VERSION_MISSING"
  | "SCHEMA_VERSION_INVALID"
  | "SCHEMA_VERSION_FUTURE"
  | "SCHEMA_VERSION_UNSUPPORTED"
  | "UNKNOWN_BACKEND"
  | "INVALID_DEVICE_IDENTITY"
  | "INVALID_SAMPLE_RATE"
  | "UNKNOWN_SAMPLE_FORMAT"
  | "INVALID_STREAM_CHANNELS"
  | "INVALID_BUFFER_FRAMES"
  | "INVALID_CHANNEL_MIX"
  | "REVALIDATION_REQUIRED"
  | "DEVICE_MISSING"
  | "DEVICE_AMBIGUOUS"
  | "BACKEND_MISMATCH"
  | "CAPABILITIES_UNAVAILABLE"
  | "CAPABILITY_DEVICE_MISMATCH"
  | "CAPABILITY_BACKEND_MISMATCH"
  | "CATALOGUE_INVALID"
  | "CONFIGURATION_DRIFT";

export interface LiveAudioInputStableDeviceIdentity {
  /** The catalogue backend label, never the generation-scoped device id. */
  backend: string;
  name: string;
  label: string;
}

/** The exact, versioned on-disk shape. Do not add optional compatibility keys. */
export interface PersistedLiveAudioInputSelectionV1 {
  schema_version: typeof LIVE_AUDIO_INPUT_SELECTION_STORAGE_SCHEMA_VERSION;
  backend: LiveAudioInputBackendId;
  device_identity: LiveAudioInputStableDeviceIdentity;
  sample_rate: number;
  sample_format: string;
  stream_channels: number;
  buffer_frames: number;
  channel_mix: LiveAudioChannelMix;
}

export interface LiveAudioInputSelectionIntent {
  backend: LiveAudioInputBackendId;
  device: LiveAudioInputDeviceSummary;
  sample_rate: number;
  sample_format: string;
  stream_channels: number;
  buffer_frames: number;
  channel_mix: LiveAudioChannelMix;
}

export interface LiveAudioInputSelectionFailure {
  ok: false;
  reason_code: LiveAudioInputSelectionStorageReasonCode;
  message: string;
}

export interface LiveAudioInputSelectionSuccess {
  ok: true;
  selection: PersistedLiveAudioInputSelectionV1;
}

export type LiveAudioInputSelectionParseResult =
  | LiveAudioInputSelectionSuccess
  | LiveAudioInputSelectionFailure;

export type LiveAudioInputSelectionSerializeResult =
  | (LiveAudioInputSelectionSuccess & { serialized: string })
  | LiveAudioInputSelectionFailure;

export interface LiveAudioInputSelectionStaleCandidate {
  state: "stale";
  start_locked: true;
  storage_action: "preserve";
  reason_code: "REVALIDATION_REQUIRED";
  message: string;
  selection: PersistedLiveAudioInputSelectionV1;
}

export interface LiveAudioInputSelectionInvalidCandidate {
  state: "invalid";
  start_locked: true;
  storage_action: "preserve";
  reason_code: LiveAudioInputSelectionStorageReasonCode;
  message: string;
}

export type LiveAudioInputSelectionRestoreResult =
  | LiveAudioInputSelectionStaleCandidate
  | LiveAudioInputSelectionInvalidCandidate;

/** A current catalogue device joined with the capabilities fetched for that exact id. */
export interface LiveAudioInputSelectionCatalogueEntry {
  device: LiveAudioInputDeviceSummary;
  capabilities: LiveAudioInputCapabilities | null;
}

export interface LiveAudioInputSelectionRevalidated {
  state: "ready";
  start_locked: false;
  reason_code: "REVALIDATED";
  message: string;
  selection: PersistedLiveAudioInputSelectionV1;
  /** The freshly enumerated id. It is never persisted. */
  current_device_id: string;
  start_request: LiveAudioInputStartRequest;
}

export interface LiveAudioInputSelectionStillLocked {
  state: "stale";
  start_locked: true;
  storage_action: "preserve";
  reason_code: Exclude<
    LiveAudioInputSelectionStorageReasonCode,
    | "EMPTY"
    | "INVALID_JSON"
    | "DUPLICATE_KEY"
    | "ROOT_NOT_OBJECT"
    | "MISSING_REQUIRED_FIELD"
    | "UNEXPECTED_FIELD"
    | "SCHEMA_VERSION_MISSING"
    | "SCHEMA_VERSION_INVALID"
    | "SCHEMA_VERSION_FUTURE"
    | "SCHEMA_VERSION_UNSUPPORTED"
    | "UNKNOWN_BACKEND"
    | "INVALID_DEVICE_IDENTITY"
    | "INVALID_SAMPLE_RATE"
    | "UNKNOWN_SAMPLE_FORMAT"
    | "INVALID_STREAM_CHANNELS"
    | "INVALID_BUFFER_FRAMES"
    | "INVALID_CHANNEL_MIX"
    | "REVALIDATION_REQUIRED"
  >;
  message: string;
  selection: PersistedLiveAudioInputSelectionV1;
}

export type LiveAudioInputSelectionRevalidationResult =
  | LiveAudioInputSelectionRevalidated
  | LiveAudioInputSelectionStillLocked;

const failure = (
  reason_code: LiveAudioInputSelectionStorageReasonCode,
  message: string,
): LiveAudioInputSelectionFailure => ({ ok: false, reason_code, message });

const isRecord = (value: unknown): value is Record<string, unknown> =>
  typeof value === "object" && value !== null && !Array.isArray(value);

const hasExactKeys = (
  value: Record<string, unknown>,
  expected: readonly string[],
): { ok: true } | { ok: false; missing?: string; unexpected?: string } => {
  const present = Object.keys(value);
  const missing = expected.find((key) => !Object.hasOwn(value, key));
  if (missing) return { ok: false, missing };
  const unexpected = present.find((key) => !expected.includes(key));
  if (unexpected) return { ok: false, unexpected };
  return present.length === expected.length ? { ok: true } : { ok: false };
};

const isExactText = (value: unknown): value is string =>
  typeof value === "string" &&
  value.length > 0 &&
  value.length <= MAX_STABLE_IDENTITY_TEXT_LENGTH &&
  value === value.trim();

const isIntegerInRange = (value: unknown, minimum: number, maximum: number): value is number =>
  typeof value === "number" &&
  Number.isSafeInteger(value) &&
  value >= minimum &&
  value <= maximum;

const isBackend = (value: unknown): value is LiveAudioInputBackendId =>
  value === "wasapi_shared" || value === "asio";

const sampleFormatIsSupported = (backend: LiveAudioInputBackendId, value: string): boolean =>
  (backend === "asio" ? ASIO_SAMPLE_FORMATS : WASAPI_SAMPLE_FORMATS).has(value);

const MAX_CURRENT_CATALOGUE_ENTRIES = 4_096;
const MAX_CURRENT_SUPPORTED_CONFIGS = 4_096;

/** Current IPC catalogue strings are validated without trimming or coercion. */
const isCatalogueText = (value: unknown): value is string =>
  isExactText(value) && !/[\u0000-\u001f\u007f]/.test(value);

const isBackendLabel = (value: unknown): value is "WASAPI" | "ASIO" =>
  value === "WASAPI" || value === "ASIO";

const backendIdForLabel = (value: "WASAPI" | "ASIO"): LiveAudioInputBackendId =>
  value === "ASIO" ? "asio" : "wasapi_shared";

const parseExactChannelMix = (
  value: unknown,
  streamChannels: number,
): LiveAudioChannelMix | LiveAudioInputSelectionFailure => {
  if (!isRecord(value) || !isExactText(value.mode)) {
    return failure("INVALID_CHANNEL_MIX", "Live audio channel mix must be an exact tagged object.");
  }
  if (value.mode === "average_all") {
    const keys = hasExactKeys(value, ["mode"]);
    return keys.ok
      ? { mode: "average_all" }
      : failure(
          keys.missing ? "MISSING_REQUIRED_FIELD" : "UNEXPECTED_FIELD",
          "Average-all channel mix contains unsupported fields.",
        );
  }
  if (value.mode === "single") {
    const keys = hasExactKeys(value, ["mode", "channel_index"]);
    if (!keys.ok || !isIntegerInRange(value.channel_index, 0, streamChannels - 1)) {
      return failure("INVALID_CHANNEL_MIX", "Single channel mix does not name a current channel.");
    }
    return { mode: "single", channel_index: value.channel_index };
  }
  if (value.mode === "stereo_pair") {
    const keys = hasExactKeys(value, ["mode", "left_channel_index", "right_channel_index"]);
    if (
      !keys.ok ||
      !isIntegerInRange(value.left_channel_index, 0, streamChannels - 1) ||
      !isIntegerInRange(value.right_channel_index, 0, streamChannels - 1) ||
      value.left_channel_index === value.right_channel_index
    ) {
      return failure("INVALID_CHANNEL_MIX", "Stereo-pair channel mix must name two distinct current channels.");
    }
    return {
      mode: "stereo_pair",
      left_channel_index: value.left_channel_index,
      right_channel_index: value.right_channel_index,
    };
  }
  return failure("INVALID_CHANNEL_MIX", "Live audio channel mix mode is unsupported.");
};

const currentBufferCapabilityIsValid = (value: unknown): boolean => {
  if (!isRecord(value) || typeof value.kind !== "string") return false;
  if (value.kind === "unknown") return hasExactKeys(value, ["kind"]).ok;
  if (value.kind !== "range" || !hasExactKeys(value, ["kind", "min_frames", "max_frames"]).ok) {
    return false;
  }
  return (
    isIntegerInRange(value.min_frames, 1, MAX_BUFFER_FRAMES) &&
    isIntegerInRange(value.max_frames, 1, MAX_BUFFER_FRAMES) &&
    value.min_frames <= value.max_frames
  );
};

const currentConfigIsValid = (
  value: unknown,
  backend: LiveAudioInputBackendId,
  resolved: boolean,
): boolean => {
  if (
    !isRecord(value) ||
    !hasExactKeys(
      value,
      resolved ? ["channels", "sample_rate", "sample_format", "buffer_size"] : ["channels", "sample_rate", "sample_format"],
    ).ok
  ) {
    return false;
  }
  return (
    isIntegerInRange(value.channels, 1, MAX_STREAM_CHANNELS) &&
    isIntegerInRange(value.sample_rate, MIN_SAMPLE_RATE_HZ, MAX_SAMPLE_RATE_HZ) &&
    isCatalogueText(value.sample_format) &&
    sampleFormatIsSupported(backend, value.sample_format) &&
    (!resolved || currentBufferCapabilityIsValid(value.buffer_size))
  );
};

const currentConfigRangeIsValid = (value: unknown, backend: LiveAudioInputBackendId): boolean => {
  if (
    !isRecord(value) ||
    !hasExactKeys(value, [
      "channels",
      "min_sample_rate",
      "max_sample_rate",
      "sample_format",
      "buffer_size",
    ]).ok
  ) {
    return false;
  }
  return (
    isIntegerInRange(value.channels, 1, MAX_STREAM_CHANNELS) &&
    isIntegerInRange(value.min_sample_rate, MIN_SAMPLE_RATE_HZ, MAX_SAMPLE_RATE_HZ) &&
    isIntegerInRange(value.max_sample_rate, MIN_SAMPLE_RATE_HZ, MAX_SAMPLE_RATE_HZ) &&
    value.min_sample_rate <= value.max_sample_rate &&
    isCatalogueText(value.sample_format) &&
    sampleFormatIsSupported(backend, value.sample_format) &&
    currentBufferCapabilityIsValid(value.buffer_size)
  );
};

const currentCapabilitiesAreValid = (value: unknown): boolean => {
  if (
    !isRecord(value) ||
    !hasExactKeys(value, [
      "device_id",
      "device_name",
      "backend",
      "default_config",
      "supported_configs",
      "resolved_config",
      "max_capture_frames",
    ]).ok ||
    !isBackendLabel(value.backend)
  ) {
    return false;
  }
  const backend = backendIdForLabel(value.backend);
  if (
    !(value.device_id === null || isCatalogueText(value.device_id)) ||
    !isCatalogueText(value.device_name) ||
    !currentConfigIsValid(value.default_config, backend, false) ||
    !currentConfigIsValid(value.resolved_config, backend, true) ||
    !isIntegerInRange(value.max_capture_frames, 1, MAX_BUFFER_FRAMES) ||
    !Array.isArray(value.supported_configs) ||
    value.supported_configs.length === 0 ||
    value.supported_configs.length > MAX_CURRENT_SUPPORTED_CONFIGS
  ) {
    return false;
  }
  const seen = new Set<string>();
  for (const config of value.supported_configs) {
    if (!currentConfigRangeIsValid(config, backend)) return false;
    const identity = JSON.stringify(config);
    if (seen.has(identity)) return false;
    seen.add(identity);
  }
  return true;
};

const currentCatalogueIsValid = (value: unknown): value is readonly LiveAudioInputSelectionCatalogueEntry[] => {
  if (!Array.isArray(value) || value.length > MAX_CURRENT_CATALOGUE_ENTRIES) return false;
  const seenDeviceIds = new Set<string>();
  for (const rawEntry of value) {
    if (!isRecord(rawEntry) || !hasExactKeys(rawEntry, ["device", "capabilities"]).ok) return false;
    const rawDevice = rawEntry.device;
    if (
      !isRecord(rawDevice) ||
      !hasExactKeys(rawDevice, ["id", "name", "label", "backend"]).ok ||
      !isCatalogueText(rawDevice.id) ||
      !isCatalogueText(rawDevice.name) ||
      !isCatalogueText(rawDevice.label) ||
      !isBackendLabel(rawDevice.backend)
    ) {
      return false;
    }
    if (seenDeviceIds.has(rawDevice.id)) return false;
    seenDeviceIds.add(rawDevice.id);
    if (rawEntry.capabilities !== null && !currentCapabilitiesAreValid(rawEntry.capabilities)) {
      return false;
    }
  }
  return true;
};

/**
 * JSON.parse intentionally accepts duplicate object keys. Storage is a strict
 * format, so scan the already-valid JSON text and reject any duplicate before
 * validating the decoded object.
 */
const hasDuplicateJsonObjectKey = (raw: string): boolean => {
  const skipWhitespace = (index: number): number => {
    while (/\s/.test(raw[index] ?? "")) index += 1;
    return index;
  };
  const scanString = (index: number): number => {
    let cursor = index + 1;
    while (cursor < raw.length) {
      if (raw[cursor] === "\\") {
        cursor += 2;
      } else if (raw[cursor] === '"') {
        return cursor + 1;
      } else {
        cursor += 1;
      }
    }
    return cursor;
  };
  const scanValue = (start: number): { index: number; duplicate: boolean } => {
    let index = skipWhitespace(start);
    if (raw[index] === "{") {
      index = skipWhitespace(index + 1);
      const keys = new Set<string>();
      if (raw[index] === "}") return { index: index + 1, duplicate: false };
      while (index < raw.length) {
        if (raw[index] !== '"') return { index: raw.length, duplicate: false };
        const keyEnd = scanString(index);
        const key = JSON.parse(raw.slice(index, keyEnd)) as string;
        if (keys.has(key)) return { index: keyEnd, duplicate: true };
        keys.add(key);
        index = skipWhitespace(keyEnd);
        if (raw[index] !== ":") return { index: raw.length, duplicate: false };
        const child = scanValue(index + 1);
        if (child.duplicate) return child;
        index = skipWhitespace(child.index);
        if (raw[index] === "}") return { index: index + 1, duplicate: false };
        if (raw[index] !== ",") return { index: raw.length, duplicate: false };
        index = skipWhitespace(index + 1);
      }
      return { index, duplicate: false };
    }
    if (raw[index] === "[") {
      index = skipWhitespace(index + 1);
      if (raw[index] === "]") return { index: index + 1, duplicate: false };
      while (index < raw.length) {
        const child = scanValue(index);
        if (child.duplicate) return child;
        index = skipWhitespace(child.index);
        if (raw[index] === "]") return { index: index + 1, duplicate: false };
        if (raw[index] !== ",") return { index: raw.length, duplicate: false };
        index = skipWhitespace(index + 1);
      }
      return { index, duplicate: false };
    }
    if (raw[index] === '"') return { index: scanString(index), duplicate: false };
    while (index < raw.length && !/[\s,}\]]/.test(raw[index])) index += 1;
    return { index, duplicate: false };
  };
  return scanValue(0).duplicate;
};

const validateSelection = (value: unknown): LiveAudioInputSelectionParseResult => {
  if (!isRecord(value)) {
    return failure("ROOT_NOT_OBJECT", "Live audio selection storage must be an object.");
  }
  if (!Object.hasOwn(value, "schema_version")) {
    return failure("SCHEMA_VERSION_MISSING", "Live audio selection storage has no schema version.");
  }
  const rootKeys = hasExactKeys(value, [
    "schema_version",
    "backend",
    "device_identity",
    "sample_rate",
    "sample_format",
    "stream_channels",
    "buffer_frames",
    "channel_mix",
  ]);
  if (!rootKeys.ok) {
    return failure(
      rootKeys.missing ? "MISSING_REQUIRED_FIELD" : "UNEXPECTED_FIELD",
      rootKeys.missing
        ? `Live audio selection storage is missing ${rootKeys.missing}.`
        : `Live audio selection storage has an unsupported field ${rootKeys.unexpected ?? ""}.`,
    );
  }
  if (typeof value.schema_version !== "number" || !Number.isSafeInteger(value.schema_version)) {
    return failure("SCHEMA_VERSION_INVALID", "Live audio selection schema version must be an integer.");
  }
  if (value.schema_version > LIVE_AUDIO_INPUT_SELECTION_STORAGE_SCHEMA_VERSION) {
    return failure("SCHEMA_VERSION_FUTURE", "Live audio selection was written by a newer Syndocal version.");
  }
  if (value.schema_version !== LIVE_AUDIO_INPUT_SELECTION_STORAGE_SCHEMA_VERSION) {
    return failure("SCHEMA_VERSION_UNSUPPORTED", "Live audio selection schema version is unsupported.");
  }
  if (!isBackend(value.backend)) {
    return failure("UNKNOWN_BACKEND", "Live audio selection names an unsupported backend.");
  }
  if (!isRecord(value.device_identity)) {
    return failure("INVALID_DEVICE_IDENTITY", "Live audio selection device identity must be an object.");
  }
  const deviceKeys = hasExactKeys(value.device_identity, ["backend", "name", "label"]);
  if (!deviceKeys.ok) {
    return failure(
      deviceKeys.missing ? "MISSING_REQUIRED_FIELD" : "UNEXPECTED_FIELD",
      "Live audio device identity must have only backend, name, and label.",
    );
  }
  const identity = value.device_identity;
  if (
    !isExactText(identity.backend) ||
    !isExactText(identity.name) ||
    !isExactText(identity.label) ||
    identity.backend !== BACKEND_LABELS[value.backend]
  ) {
    return failure("INVALID_DEVICE_IDENTITY", "Live audio selection device identity is invalid or backend-mismatched.");
  }
  if (!isIntegerInRange(value.sample_rate, MIN_SAMPLE_RATE_HZ, MAX_SAMPLE_RATE_HZ)) {
    return failure("INVALID_SAMPLE_RATE", "Live audio sample rate is outside the supported persisted range.");
  }
  if (!isExactText(value.sample_format) || !sampleFormatIsSupported(value.backend, value.sample_format)) {
    return failure("UNKNOWN_SAMPLE_FORMAT", "Live audio selection names an unsupported sample format.");
  }
  if (!isIntegerInRange(value.stream_channels, 1, MAX_STREAM_CHANNELS)) {
    return failure("INVALID_STREAM_CHANNELS", "Live audio stream channel count is outside the supported persisted range.");
  }
  if (!isIntegerInRange(value.buffer_frames, 1, MAX_BUFFER_FRAMES)) {
    return failure("INVALID_BUFFER_FRAMES", "Live audio buffer size is outside the capture safety range.");
  }
  if (value.buffer_frames * 1_000 > value.sample_rate * 200) {
    return failure("INVALID_BUFFER_FRAMES", "Live audio buffer age exceeds the watchdog-safe limit.");
  }
  const channel_mix = parseExactChannelMix(value.channel_mix, value.stream_channels);
  if ("ok" in channel_mix) return channel_mix;
  return {
    ok: true,
    selection: {
      schema_version: LIVE_AUDIO_INPUT_SELECTION_STORAGE_SCHEMA_VERSION,
      backend: value.backend,
      device_identity: {
        backend: identity.backend,
        name: identity.name,
        label: identity.label,
      },
      sample_rate: value.sample_rate,
      sample_format: value.sample_format,
      stream_channels: value.stream_channels,
      buffer_frames: value.buffer_frames,
      channel_mix,
    },
  };
};

export const parseLiveAudioInputSelectionStorage = (
  raw: string | null | undefined,
): LiveAudioInputSelectionParseResult => {
  if (typeof raw !== "string" || raw.length === 0) {
    return failure("EMPTY", "No machine-local live audio selection has been saved.");
  }
  let value: unknown;
  try {
    value = JSON.parse(raw) as unknown;
  } catch {
    return failure("INVALID_JSON", "Machine-local live audio selection is not valid JSON and was preserved.");
  }
  if (hasDuplicateJsonObjectKey(raw)) {
    return failure("DUPLICATE_KEY", "Machine-local live audio selection has duplicate JSON keys and was preserved.");
  }
  return validateSelection(value);
};

export const serializeLiveAudioInputSelection = (
  intent: LiveAudioInputSelectionIntent,
): LiveAudioInputSelectionSerializeResult => {
  const parsed = validateSelection({
    schema_version: LIVE_AUDIO_INPUT_SELECTION_STORAGE_SCHEMA_VERSION,
    backend: intent.backend,
    device_identity: {
      backend: intent.device.backend,
      name: intent.device.name,
      label: intent.device.label,
    },
    sample_rate: intent.sample_rate,
    sample_format: intent.sample_format,
    stream_channels: intent.stream_channels,
    buffer_frames: intent.buffer_frames,
    channel_mix: intent.channel_mix,
  });
  return parsed.ok
    ? { ...parsed, serialized: JSON.stringify(parsed.selection) }
    : parsed;
};

export const restoreLiveAudioInputSelection = (
  raw: string | null | undefined,
): LiveAudioInputSelectionRestoreResult => {
  const parsed = parseLiveAudioInputSelectionStorage(raw);
  if (!parsed.ok) {
    return {
      state: "invalid",
      start_locked: true,
      storage_action: "preserve",
      reason_code: parsed.reason_code,
      message: parsed.message,
    };
  }
  return {
    state: "stale",
    start_locked: true,
    storage_action: "preserve",
    reason_code: "REVALIDATION_REQUIRED",
    message: "Saved live audio selection requires fresh device and capability revalidation before Start.",
    selection: parsed.selection,
  };
};

const bufferCapabilityIncludes = (
  capability: LiveAudioInputCapabilities["supported_configs"][number]["buffer_size"],
  frames: number,
): boolean =>
  capability.kind === "range" &&
  Number.isSafeInteger(capability.min_frames) &&
  Number.isSafeInteger(capability.max_frames) &&
  capability.min_frames > 0 &&
  capability.min_frames <= capability.max_frames &&
  capability.min_frames <= frames &&
  frames <= capability.max_frames;

const capabilityExactlyMatchesSelection = (
  capabilities: LiveAudioInputCapabilities,
  device: LiveAudioInputDeviceSummary,
  selection: PersistedLiveAudioInputSelectionV1,
): LiveAudioInputSelectionStorageReasonCode | null => {
  if (
    capabilities.device_id !== device.id ||
    capabilities.device_name !== selection.device_identity.name
  ) {
    return "CAPABILITY_DEVICE_MISMATCH";
  }
  if (capabilities.backend !== BACKEND_LABELS[selection.backend]) {
    return "CAPABILITY_BACKEND_MISMATCH";
  }
  if (
    capabilities.resolved_config.channels !== selection.stream_channels ||
    capabilities.resolved_config.sample_rate !== selection.sample_rate ||
    capabilities.resolved_config.sample_format !== selection.sample_format ||
    !isIntegerInRange(capabilities.max_capture_frames, 1, MAX_BUFFER_FRAMES) ||
    selection.buffer_frames > capabilities.max_capture_frames
  ) {
    return "CONFIGURATION_DRIFT";
  }
  const exactSupportedConfiguration = capabilities.supported_configs.some(
    (config) =>
      config.channels === selection.stream_channels &&
      config.sample_format === selection.sample_format &&
      config.min_sample_rate <= selection.sample_rate &&
      selection.sample_rate <= config.max_sample_rate &&
      bufferCapabilityIncludes(config.buffer_size, selection.buffer_frames),
  );
  return exactSupportedConfiguration ? null : "CONFIGURATION_DRIFT";
};

const stale = (
  reason_code: LiveAudioInputSelectionStillLocked["reason_code"],
  message: string,
  selection: PersistedLiveAudioInputSelectionV1,
): LiveAudioInputSelectionStillLocked => ({
  state: "stale",
  start_locked: true,
  storage_action: "preserve",
  reason_code,
  message,
  selection,
});

/**
 * Revalidation is the only path that can expose a current catalogue id. It
 * intentionally never chooses a different backend, a first device, or a best
 * matching configuration.
 */
export const revalidateLiveAudioInputSelection = (
  candidate: LiveAudioInputSelectionStaleCandidate,
  catalogue: readonly LiveAudioInputSelectionCatalogueEntry[],
): LiveAudioInputSelectionRevalidationResult => {
  const selection = candidate.selection;
  if (!currentCatalogueIsValid(catalogue)) {
    return stale(
      "CATALOGUE_INVALID",
      "Current live audio device or capability data is malformed; refresh the catalogue before Start.",
      selection,
    );
  }
  const sameNameAndLabel = catalogue.filter(
    (entry) =>
      entry.device.name === selection.device_identity.name &&
      entry.device.label === selection.device_identity.label,
  );
  const exactIdentity = sameNameAndLabel.filter(
    (entry) => entry.device.backend === selection.device_identity.backend,
  );
  if (exactIdentity.length === 0) {
    return sameNameAndLabel.length > 0
      ? stale("BACKEND_MISMATCH", "Saved live audio backend is not available for the selected device.", selection)
      : stale("DEVICE_MISSING", "Saved live audio device is not present in the refreshed catalogue.", selection);
  }
  if (exactIdentity.length !== 1) {
    return stale("DEVICE_AMBIGUOUS", "Saved live audio device identity is ambiguous in the refreshed catalogue.", selection);
  }
  const [entry] = exactIdentity;
  if (!entry.capabilities) {
    return stale("CAPABILITIES_UNAVAILABLE", "Saved live audio device has no fresh capability result.", selection);
  }
  const mismatch = capabilityExactlyMatchesSelection(entry.capabilities, entry.device, selection);
  if (mismatch === "CAPABILITY_DEVICE_MISMATCH") {
    return stale(mismatch, "Refreshed capabilities belong to a different audio device.", selection);
  }
  if (mismatch === "CAPABILITY_BACKEND_MISMATCH") {
    return stale(mismatch, "Refreshed capabilities belong to a different audio backend.", selection);
  }
  if (mismatch === "CONFIGURATION_DRIFT") {
    return stale(mismatch, "Saved live audio configuration is no longer exactly supported.", selection);
  }
  return {
    state: "ready",
    start_locked: false,
    reason_code: "REVALIDATED",
    message: "Saved live audio selection was revalidated against the current catalogue.",
    selection,
    current_device_id: entry.device.id,
    start_request: {
      backend: selection.backend,
      device_id: entry.device.id,
      sample_rate: selection.sample_rate,
      stream_channels: selection.stream_channels,
      sample_format: selection.sample_format,
      buffer_frames: selection.buffer_frames,
      channel_mix: selection.channel_mix,
    },
  };
};

export interface LiveAudioInputSelectionStoragePort {
  getItem(key: string): string | null;
  setItem(key: string, value: string): void;
}

export type LiveAudioInputSelectionStorageReadResult =
  | { ok: true; raw: string | null }
  | { ok: false; error: string };

export type LiveAudioInputSelectionStorageWriteResult =
  | { ok: true }
  | { ok: false; error: string };

export const readLiveAudioInputSelectionStorage = (
  openPort: () => LiveAudioInputSelectionStoragePort,
): LiveAudioInputSelectionStorageReadResult => {
  try {
    const port = openPort();
    const raw = port.getItem(LIVE_AUDIO_INPUT_SELECTION_STORAGE_KEY);
    if (raw !== null && typeof raw !== "string") {
      return {
        ok: false,
        error: "Live audio selection storage returned a non-string payload.",
      };
    }
    return { ok: true, raw };
  } catch (error) {
    return {
      ok: false,
      error: error instanceof Error ? error.message : String(error),
    };
  }
};

export const writeLiveAudioInputSelectionStorage = (
  openPort: () => LiveAudioInputSelectionStoragePort,
  serialized: string,
): LiveAudioInputSelectionStorageWriteResult => {
  try {
    const port = openPort();
    port.setItem(LIVE_AUDIO_INPUT_SELECTION_STORAGE_KEY, serialized);
    return { ok: true };
  } catch (error) {
    return {
      ok: false,
      error: error instanceof Error ? error.message : String(error),
    };
  }
};
