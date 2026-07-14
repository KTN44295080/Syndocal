import type {
  LiveAudioInputBackendId,
  LiveAudioInputBackendSummary,
  LiveAudioInputCapabilities,
  LiveAudioInputStatus,
} from "./types";

export type LiveAudioInputHealth = "unknown" | "stopped" | "live" | "clearing" | "stale";

export type LiveAudioInputBackendState =
  | "checking"
  | "not_built"
  | "empty"
  | "select_device"
  | "configure"
  | "fault"
  | "ready"
  | "open"
  | "active";

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
  if (!input.backend.built) return "not_built";
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
    case "not_built":
      return "NOT BUILT";
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
