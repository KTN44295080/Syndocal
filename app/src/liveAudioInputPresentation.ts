import type { LiveAudioInputStatus } from "./types";

export type LiveAudioInputHealth = "unknown" | "stopped" | "live" | "clearing" | "stale";

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
    case "live":
      return `OVR ${status.dropped_chunks}/${status.dropped_frames}f · ${status.backend ?? "Audio"} shared · ${status.sample_format ?? "unknown"} ${(status.sample_rate / 1_000).toFixed(1)}kHz · ${liveAudioChannelMixLabel(status)} · REQ BUF ${status.configured_buffer_frames === null || status.configured_buffer_frames === undefined ? "default" : `${status.configured_buffer_frames}f`} · CB ${status.last_callback_frames}/${status.min_callback_frames}/${status.max_callback_frames}f · FFT ${status.analyzed_windows} · C→W EST ${(status.capture_to_worker_us / 1_000).toFixed(1)}/${(status.max_capture_to_worker_us / 1_000).toFixed(1)}ms · Q ${status.queue_depth}/${status.queue_depth_high_water}/${status.queue_capacity}`;
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
