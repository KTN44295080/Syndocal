import type {
  LiveAudioInputAsioSelection,
  LiveAudioInputAsioSelectionState,
  LiveAudioInputBackendAvailability,
  LiveAudioInputBackendId,
  LiveAudioInputBackendSummary,
  LiveAudioInputCapabilities,
  LiveAudioInputStatus,
} from "./types";

export type LiveAudioInputHealth = "unknown" | "stopped" | "live" | "clearing" | "stale";

export type LiveAudioInputBackendState =
  | "checking"
  | "not_packaged"
  | "unsupported"
  | "contract_invalid"
  | "empty"
  | "select_device"
  | "configure"
  | "fault"
  | "ready"
  | "open"
  | "active";

export type LiveAudioInputBackendContractFailureReason =
  | "SUMMARY_NOT_OBJECT"
  | "SUMMARY_LIST_INVALID"
  | "SUMMARY_UNEXPECTED_FIELD"
  | "BACKEND_ID_UNKNOWN"
  | "BACKEND_ID_DUPLICATE"
  | "LABEL_BLANK"
  | "BUILT_INVALID"
  | "REQUIRES_EXPLICIT_DEVICE_INVALID"
  | "DISTRIBUTION_BLANK"
  | "AVAILABILITY_MISSING"
  | "AVAILABILITY_UNKNOWN"
  | "AVAILABILITY_READY_CONTRADICTS_BUILT"
  | "AVAILABILITY_DETAIL_INVALID"
  | "AVAILABILITY_DETAIL_BLANK";

const LIVE_AUDIO_INPUT_BACKEND_AVAILABILITIES: readonly string[] = [
  "ready",
  "not_packaged",
  "fault",
  "unsupported",
];

const LIVE_AUDIO_INPUT_BACKEND_SUMMARY_KEYS: readonly string[] = [
  "id",
  "label",
  "built",
  "requires_explicit_device",
  "distribution",
  "availability",
  "availability_detail",
];

export type ParsedLiveAudioInputBackendSummary =
  | { ok: true; summary: LiveAudioInputBackendSummary }
  | { ok: false; reason_code: LiveAudioInputBackendContractFailureReason };

export type ParsedLiveAudioInputBackendSummaries =
  | { ok: true; summaries: LiveAudioInputBackendSummary[] }
  | {
    ok: false;
    index: number;
    reason_code: LiveAudioInputBackendContractFailureReason;
  };

/**
 * Runtime contract gate for backend summaries: unknown or missing
 * `availability` fails closed instead of falling back to a `built`-derived
 * guess. The parsed summary is an exact defensive copy with exactly the
 * contracted fields.
 */
export const parseLiveAudioInputBackendSummary = (
  raw: unknown,
): ParsedLiveAudioInputBackendSummary => {
  if (typeof raw !== "object" || raw === null || Array.isArray(raw)) {
    return { ok: false, reason_code: "SUMMARY_NOT_OBJECT" };
  }
  const record = raw as Record<string, unknown>;
  for (const key of Object.keys(record)) {
    if (!LIVE_AUDIO_INPUT_BACKEND_SUMMARY_KEYS.includes(key)) {
      return { ok: false, reason_code: "SUMMARY_UNEXPECTED_FIELD" };
    }
  }
  if (record.id !== "wasapi_shared" && record.id !== "asio") {
    return { ok: false, reason_code: "BACKEND_ID_UNKNOWN" };
  }
  if (typeof record.label !== "string" || !record.label.trim()) {
    return { ok: false, reason_code: "LABEL_BLANK" };
  }
  if (typeof record.built !== "boolean") {
    return { ok: false, reason_code: "BUILT_INVALID" };
  }
  if (typeof record.requires_explicit_device !== "boolean") {
    return { ok: false, reason_code: "REQUIRES_EXPLICIT_DEVICE_INVALID" };
  }
  if (typeof record.distribution !== "string" || !record.distribution.trim()) {
    return { ok: false, reason_code: "DISTRIBUTION_BLANK" };
  }
  if (
    !("availability" in record) ||
    record.availability === undefined ||
    record.availability === null
  ) {
    return { ok: false, reason_code: "AVAILABILITY_MISSING" };
  }
  if (!LIVE_AUDIO_INPUT_BACKEND_AVAILABILITIES.includes(record.availability as string)) {
    return { ok: false, reason_code: "AVAILABILITY_UNKNOWN" };
  }
  if (record.availability_detail !== null) {
    if (typeof record.availability_detail !== "string") {
      return { ok: false, reason_code: "AVAILABILITY_DETAIL_INVALID" };
    }
    if (!record.availability_detail.trim()) {
      return { ok: false, reason_code: "AVAILABILITY_DETAIL_BLANK" };
    }
  }
  if (record.availability === "ready" && record.built !== true) {
    return { ok: false, reason_code: "AVAILABILITY_READY_CONTRADICTS_BUILT" };
  }
  return {
    ok: true,
    summary: {
      id: record.id,
      label: record.label,
      built: record.built,
      requires_explicit_device: record.requires_explicit_device,
      distribution: record.distribution,
      availability: record.availability as LiveAudioInputBackendAvailability,
      availability_detail: record.availability_detail as string | null,
    },
  };
};

/**
 * All-or-nothing list gate: one invalid summary or one duplicate backend ID
 * fails the entire catalogue at the offending index.
 */
export const parseLiveAudioInputBackendSummaries = (
  raw: unknown,
): ParsedLiveAudioInputBackendSummaries => {
  if (!Array.isArray(raw)) {
    return { ok: false, index: -1, reason_code: "SUMMARY_LIST_INVALID" };
  }
  const summaries: LiveAudioInputBackendSummary[] = [];
  const seenIds = new Set<LiveAudioInputBackendId>();
  for (let index = 0; index < raw.length; index += 1) {
    const parsed = parseLiveAudioInputBackendSummary(raw[index]);
    if (!parsed.ok) return { ok: false, index, reason_code: parsed.reason_code };
    if (seenIds.has(parsed.summary.id)) {
      return { ok: false, index, reason_code: "BACKEND_ID_DUPLICATE" };
    }
    seenIds.add(parsed.summary.id);
    summaries.push(parsed.summary);
  }
  return { ok: true, summaries };
};

export const liveAudioInputBackendCanDispatch = (
  raw: unknown,
): boolean => {
  const parsed = parseLiveAudioInputBackendSummary(raw);
  return parsed.ok && parsed.summary.built && parsed.summary.availability === "ready";
};

export const liveAudioInputBackendAvailabilityLabel = (
  availability: LiveAudioInputBackendAvailability,
): string => {
  switch (availability) {
    case "ready":
      return "READY";
    case "not_packaged":
      return "NOT PACKAGED";
    case "fault":
      return "FAULT";
    case "unsupported":
      return "UNSUPPORTED";
  }
};

export const liveAudioInputBackendScopeLabel = (
  backend: LiveAudioInputBackendId,
): string => (backend === "asio" ? "ASIO" : "WASAPI");

export const liveAudioInputBackendOptionLabel = (
  backend: LiveAudioInputBackendSummary,
): string => {
  const parsed = parseLiveAudioInputBackendSummary(backend);
  if (!parsed.ok) return `${backend.label} · CONTRACT INVALID`;
  if (!liveAudioInputBackendCanDispatch(backend)) {
    return `${backend.label} · ${liveAudioInputBackendAvailabilityLabel(parsed.summary.availability)}`;
  }
  return backend.label;
};

export type LiveAudioInputBackendSelectOption =
  | { kind: "checking"; value: LiveAudioInputBackendId; label: string; noLocalize: false }
  | { kind: "selection_unavailable"; value: LiveAudioInputBackendId; label: string; noLocalize: true }
  | {
    kind: "catalogue";
    value: LiveAudioInputBackendId;
    label: string;
    noLocalize: true;
    duplicate_catalogue_entry: boolean;
  };

export const liveAudioInputBackendSelectOptions = (input: {
  selectedBackend: LiveAudioInputBackendId;
  backends: readonly LiveAudioInputBackendSummary[];
  backendsKnown: boolean;
  savedBackend: LiveAudioInputBackendId | null;
}): LiveAudioInputBackendSelectOption[] => {
  const { selectedBackend, backends, backendsKnown, savedBackend } = input;
  if (backends.length === 0) {
    return [{
      kind: "checking",
      value: selectedBackend,
      label: backendsKnown ? "Backend unavailable" : "Checking backends",
      noLocalize: false,
    }];
  }
  const firstIndexById = new Map<LiveAudioInputBackendId, number>();
  const duplicateIds = new Set<LiveAudioInputBackendId>();
  for (let index = 0; index < backends.length; index += 1) {
    const id = backends[index].id;
    if (firstIndexById.has(id)) duplicateIds.add(id);
    else firstIndexById.set(id, index);
  }
  const options: LiveAudioInputBackendSelectOption[] = [];
  for (let index = 0; index < backends.length; index += 1) {
    const backend = backends[index];
    if (firstIndexById.get(backend.id) !== index) continue;
    const baseLabel = liveAudioInputBackendOptionLabel(backend);
    options.push({
      kind: "catalogue",
      value: backend.id,
      label: duplicateIds.has(backend.id)
        ? `${baseLabel} · DUPLICATE BACKEND CATALOGUE · CONTRACT INVALID`
        : baseLabel,
      noLocalize: true,
      duplicate_catalogue_entry: duplicateIds.has(backend.id),
    });
  }
  if (!firstIndexById.has(selectedBackend)) {
    options.unshift({
      kind: "selection_unavailable",
      value: selectedBackend,
      label:
        savedBackend === selectedBackend
          ? `${liveAudioInputBackendScopeLabel(selectedBackend)} · saved selection missing`
          : `${liveAudioInputBackendScopeLabel(selectedBackend)} · selection unavailable`,
      noLocalize: true,
    });
  }
  return options;
};

export const liveAudioInputBackendVisibleValue = (
  options: readonly { value: string }[],
  target: string,
): string | null =>
  options.some((option) => option.value === target) ? target : null;

export type LiveAudioInputAsioSelectionContractFailureReason =
  | "ASIO_SELECTION_NOT_OBJECT"
  | "ASIO_SELECTION_UNEXPECTED_FIELD"
  | "ASIO_SELECTION_STATE_INVALID"
  | "ASIO_SELECTION_DRIVER_ID_INVALID"
  | "ASIO_SELECTION_DRIVER_NAME_INVALID"
  | "ASIO_SELECTION_SAMPLE_RATE_INVALID"
  | "ASIO_SELECTION_INPUT_CHANNELS_INVALID"
  | "ASIO_SELECTION_SAMPLE_FORMAT_INVALID"
  | "ASIO_SELECTION_FIXED_BUFFER_FRAMES_INVALID"
  | "ASIO_SELECTION_REASON_INVALID"
  | "ASIO_SELECTION_MESSAGE_INVALID";

export type ParsedLiveAudioInputAsioSelection =
  | { ok: true; selection: LiveAudioInputAsioSelection }
  | { ok: false; reason_code: LiveAudioInputAsioSelectionContractFailureReason };

const LIVE_AUDIO_INPUT_ASIO_SELECTION_KEYS: readonly string[] = [
  "state",
  "driver_id",
  "driver_name",
  "sample_rate_hz",
  "input_channels",
  "sample_format",
  "fixed_buffer_frames",
  "reason",
  "message",
];

const liveAudioAsioIsNonblankString = (value: unknown): value is string =>
  typeof value === "string" && value.trim().length > 0;

const liveAudioAsioIsPositiveSafeInteger = (value: unknown): value is number =>
  typeof value === "number" && Number.isSafeInteger(value) && value > 0;

const liveAudioAsioInvalidStateLeakedFieldReason = (
  record: Record<string, unknown>,
): LiveAudioInputAsioSelectionContractFailureReason => {
  if (record.driver_id !== null) return "ASIO_SELECTION_DRIVER_ID_INVALID";
  if (record.driver_name !== null) return "ASIO_SELECTION_DRIVER_NAME_INVALID";
  if (record.sample_rate_hz !== null) return "ASIO_SELECTION_SAMPLE_RATE_INVALID";
  if (record.input_channels !== null) return "ASIO_SELECTION_INPUT_CHANNELS_INVALID";
  if (record.sample_format !== null) return "ASIO_SELECTION_SAMPLE_FORMAT_INVALID";
  return "ASIO_SELECTION_FIXED_BUFFER_FRAMES_INVALID";
};

/**
 * Exact fail-closed contract gate for the backend-native ASIO persisted
 * selection verdict attached to LiveAudioInputStatus. restored/revalidated
 * require full nonblank device identity plus positive safe integers and a
 * null reason; invalid requires every device/config field to be exactly
 * null with a nonblank reason. Anything unknown, missing, or malformed
 * fails with an explicit reason code.
 */
export const parseLiveAudioInputAsioSelection = (
  raw: unknown,
): ParsedLiveAudioInputAsioSelection => {
  if (typeof raw !== "object" || raw === null || Array.isArray(raw)) {
    return { ok: false, reason_code: "ASIO_SELECTION_NOT_OBJECT" };
  }
  const record = raw as Record<string, unknown>;
  for (const key of Object.keys(record)) {
    if (!LIVE_AUDIO_INPUT_ASIO_SELECTION_KEYS.includes(key)) {
      return { ok: false, reason_code: "ASIO_SELECTION_UNEXPECTED_FIELD" };
    }
  }
  const state = record.state;
  if (state !== "restored" && state !== "revalidated" && state !== "invalid") {
    return { ok: false, reason_code: "ASIO_SELECTION_STATE_INVALID" };
  }
  if (!liveAudioAsioIsNonblankString(record.message)) {
    return { ok: false, reason_code: "ASIO_SELECTION_MESSAGE_INVALID" };
  }
  const message = record.message;
  if (state === "invalid") {
    if (
      record.driver_id !== null ||
      record.driver_name !== null ||
      record.sample_rate_hz !== null ||
      record.input_channels !== null ||
      record.sample_format !== null ||
      record.fixed_buffer_frames !== null
    ) {
      return { ok: false, reason_code: liveAudioAsioInvalidStateLeakedFieldReason(record) };
    }
    if (!liveAudioAsioIsNonblankString(record.reason)) {
      return { ok: false, reason_code: "ASIO_SELECTION_REASON_INVALID" };
    }
    return {
      ok: true,
      selection: {
        state,
        driver_id: null,
        driver_name: null,
        sample_rate_hz: null,
        input_channels: null,
        sample_format: null,
        fixed_buffer_frames: null,
        reason: record.reason,
        message,
      },
    };
  }
  if (!liveAudioAsioIsNonblankString(record.driver_id)) {
    return { ok: false, reason_code: "ASIO_SELECTION_DRIVER_ID_INVALID" };
  }
  if (!liveAudioAsioIsNonblankString(record.driver_name)) {
    return { ok: false, reason_code: "ASIO_SELECTION_DRIVER_NAME_INVALID" };
  }
  if (!liveAudioAsioIsPositiveSafeInteger(record.sample_rate_hz)) {
    return { ok: false, reason_code: "ASIO_SELECTION_SAMPLE_RATE_INVALID" };
  }
  if (!liveAudioAsioIsPositiveSafeInteger(record.input_channels)) {
    return { ok: false, reason_code: "ASIO_SELECTION_INPUT_CHANNELS_INVALID" };
  }
  if (!liveAudioAsioIsNonblankString(record.sample_format)) {
    return { ok: false, reason_code: "ASIO_SELECTION_SAMPLE_FORMAT_INVALID" };
  }
  if (!liveAudioAsioIsPositiveSafeInteger(record.fixed_buffer_frames)) {
    return { ok: false, reason_code: "ASIO_SELECTION_FIXED_BUFFER_FRAMES_INVALID" };
  }
  if (record.reason !== null) {
    return { ok: false, reason_code: "ASIO_SELECTION_REASON_INVALID" };
  }
  return {
    ok: true,
    selection: {
      state,
      driver_id: record.driver_id,
      driver_name: record.driver_name,
      sample_rate_hz: record.sample_rate_hz,
      input_channels: record.input_channels,
      sample_format: record.sample_format,
      fixed_buffer_frames: record.fixed_buffer_frames,
      reason: null,
      message,
    },
  };
};

export type LiveAudioInputAsioVerdictState =
  | LiveAudioInputAsioSelectionState
  | "contract_invalid";

export interface LiveAudioInputAsioVerdict {
  state: LiveAudioInputAsioVerdictState;
  contract_valid: boolean;
  start_locked: boolean;
  state_label: string;
  reason: string | null;
  message: string;
}

export const liveAudioInputAsioSelectionVerdict = (
  raw: unknown,
): LiveAudioInputAsioVerdict => {
  const parsed = parseLiveAudioInputAsioSelection(raw);
  if (!parsed.ok) {
    return {
      state: "contract_invalid",
      contract_valid: false,
      start_locked: true,
      state_label: "ASIO VERDICT CONTRACT INVALID",
      reason: parsed.reason_code,
      message: "The native ASIO selection verdict violated its wire contract; Start stays locked.",
    };
  }
  switch (parsed.selection.state) {
    case "restored":
      return {
        state: "restored",
        contract_valid: true,
        start_locked: true,
        state_label: "ASIO RESTORED",
        reason: null,
        message: parsed.selection.message,
      };
    case "revalidated":
      return {
        state: "revalidated",
        contract_valid: true,
        start_locked: false,
        state_label: "ASIO REVALIDATED",
        reason: null,
        message: parsed.selection.message,
      };
    case "invalid":
      return {
        state: "invalid",
        contract_valid: true,
        start_locked: true,
        state_label: "ASIO INVALID",
        reason: parsed.selection.reason,
        message: parsed.selection.message,
      };
  }
};

const statusMatchesBackend = (
  status: LiveAudioInputStatus,
  backendId: LiveAudioInputBackendId,
): boolean => {
  const backend = status.backend?.trim().toLowerCase();
  if (!backend) return false;
  return backendId === "asio" ? backend.includes("asio") : backend.includes("wasapi");
};

export const liveAudioInputBackendState = (input: {
  backend?: LiveAudioInputBackendSummary;
  backendKnown: boolean;
  backendBusy: boolean;
  backendError?: string | null;
  devicesAvailable: number;
  selectedDeviceId: string;
  sampleRate: number | null;
  bufferFrames: number | null;
  capabilities: LiveAudioInputCapabilities | null;
  status: LiveAudioInputStatus;
  statusKnown: boolean;
}): LiveAudioInputBackendState => {
  if (!input.backendKnown || input.backendBusy) return "checking";
  if (!input.backend || input.backendError?.trim()) return "fault";
  const parsed = parseLiveAudioInputBackendSummary(input.backend);
  if (!parsed.ok) return "contract_invalid";
  switch (parsed.summary.availability) {
    case "not_packaged":
      return "not_packaged";
    case "fault":
      return "fault";
    case "unsupported":
      return "unsupported";
    case "ready":
      if (!liveAudioInputBackendCanDispatch(parsed.summary)) return "contract_invalid";
      break;
  }
  if (statusMatchesBackend(input.status, input.backend.id)) {
    if (!input.statusKnown) return "checking";
    if (input.status.stale || input.status.last_error?.trim()) return "fault";
    if (input.status.running) {
      return input.status.callback_count > 0 ? "active" : "open";
    }
  }
  if (input.devicesAvailable === 0) return "empty";
  if (input.backend.requires_explicit_device && !input.selectedDeviceId.trim()) {
    return "select_device";
  }
  if (
    input.backend.requires_explicit_device &&
    (input.sampleRate === null || input.bufferFrames === null)
  ) {
    return "configure";
  }
  return input.capabilities ? "ready" : "select_device";
};

export const liveAudioInputBackendStateLabel = (
  state: LiveAudioInputBackendState,
): string => {
  switch (state) {
    case "checking":
      return "CHECKING";
    case "not_packaged":
      return "NOT PACKAGED";
    case "unsupported":
      return "UNSUPPORTED";
    case "contract_invalid":
      return "CONTRACT INVALID";
    case "empty":
      return "EMPTY";
    case "select_device":
      return "SELECT DRIVER";
    case "configure":
      return "CONFIGURE";
    case "fault":
      return "FAULT";
    case "ready":
      return "READY";
    case "open":
      return "OPEN";
    case "active":
      return "ACTIVE";
  }
};

export const liveAudioChannelMixLabel = (status: LiveAudioInputStatus): string => {
  switch (status.channel_mix.mode) {
    case "average_all":
      return `${status.channels}→M`;
    case "single":
      return `CH${status.channel_mix.channel_index + 1}→M`;
    case "stereo_pair":
      return `CH${status.channel_mix.left_channel_index + 1}+${status.channel_mix.right_channel_index + 1}→M`;
  }
};

const liveAudioRuntimeBackendLabel = (status: LiveAudioInputStatus): string => {
  const backend = status.backend?.trim() || "Audio";
  return backend.toLowerCase().includes("asio") ? backend : `${backend} shared`;
};

export const liveAudioInputHealth = (
  status: LiveAudioInputStatus,
  statusKnown = true,
): LiveAudioInputHealth => {
  if (!statusKnown) return "unknown";
  if (status.safety_clear_pending) return "clearing";
  if (!status.running) return "stopped";
  if (!status.stale) return "live";
  return "stale";
};

export const liveAudioInputDetail = (
  status: LiveAudioInputStatus,
  statusKnown = true,
): string => {
  switch (liveAudioInputHealth(status, statusKnown)) {
    case "unknown":
      return status.running
        ? "LIVE AUDIO STATUS UNKNOWN · Stop remains available as the safe escape."
        : "LIVE AUDIO STATUS UNKNOWN · Start is locked until the backend responds.";
    case "clearing":
      return status.running
        ? "SAFETY CLEAR PENDING · The engine has not accepted the clear yet; Stop remains available."
        : "SAFETY CLEAR PENDING · Stop is draining the final clear; Start stays locked.";
    case "stale":
      return status.last_error?.trim()
        ? `SAFETY CLEAR ACCEPTED · ${status.last_error} Stop then Start to reconnect.`
        : "SAFETY CLEAR ACCEPTED · Live audio is stale; the engine is draining the zero-source request.";
    case "live": {
      const requestedBuffer = status.configured_buffer_frames === null || status.configured_buffer_frames === undefined
        ? "default"
        : `${status.configured_buffer_frames}f`;
      const bufferDetail = status.applied_buffer_frames === null || status.applied_buffer_frames === undefined
        ? `REQ BUF ${requestedBuffer} · APPLIED pending`
        : `BUF ${status.applied_buffer_frames}f · REQ ${requestedBuffer}`;
      return `OVR ${status.dropped_chunks}/${status.dropped_frames}f · XRUN ${status.backend_xruns ?? 0} · ${liveAudioRuntimeBackendLabel(status)} · ${status.sample_format ?? "unknown"} ${(status.sample_rate / 1_000).toFixed(1)}kHz · ${liveAudioChannelMixLabel(status)} · ${bufferDetail} · CB ${status.last_callback_frames}/${status.min_callback_frames}/${status.max_callback_frames}f · FFT ${status.analyzed_windows} · C→W EST ${(status.capture_to_worker_us / 1_000).toFixed(1)}/${(status.max_capture_to_worker_us / 1_000).toFixed(1)}ms · Q ${status.queue_depth}/${status.queue_depth_high_water}/${status.queue_capacity}`;
    }
    case "stopped":
      return status.last_error?.trim() || "Live bands can drive Node Graph Audio sources.";
  }
};

export const liveAudioNodeAvailability = (
  status: LiveAudioInputStatus,
  statusKnown = true,
): string => {
  switch (liveAudioInputHealth(status, statusKnown)) {
    case "unknown":
      return "Live status unknown";
    case "clearing":
      return "Live safety clear pending";
    case "stale":
      return "Live stale · clear accepted";
    case "live":
      return "Live input";
    case "stopped":
      return "Live stopped";
  }
};

export const liveAudioInputAnnouncement = (
  status: LiveAudioInputStatus,
  statusKnown = true,
): string => {
  switch (liveAudioInputHealth(status, statusKnown)) {
    case "unknown":
      return status.running
        ? "Live audio status is unavailable. Stop remains available."
        : "Live audio status is unavailable. Start is locked.";
    case "stopped":
      return "Live audio input stopped.";
    case "live":
      return "Live audio input active.";
    case "clearing":
      return status.running
        ? "Live audio safety clear pending. Stop remains available."
        : "Live audio Stop is waiting for the engine safety clear. Start is locked.";
    case "stale":
      return "Live audio input stale. The engine accepted its zero-source clear request.";
  }
};
