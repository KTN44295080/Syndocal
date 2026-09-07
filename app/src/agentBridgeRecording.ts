import type { FrontendTauriInvoke } from "./tauriInvokeCommands";

const MAX_PATH_LENGTH = 2048;
const MAX_ERROR_LENGTH = 1024;

const optionalText = (value: unknown, name: string, maxLength: number): string | null => {
  if (value === null || value === undefined) return null;
  if (typeof value !== "string" || value.length > maxLength) {
    throw new Error(`Recording status field is invalid: ${name}`);
  }
  return value;
};

const finiteNonNegative = (value: unknown, name: string): number => {
  if (typeof value !== "number" || !Number.isSafeInteger(value) || value < 0) {
    throw new Error(`Recording status field is invalid: ${name}`);
  }
  return value;
};

/** Read-only recording observation for MCP; it never starts or stops a sink. */
export async function executeAgentBridgeRecordingStatus(invoke: FrontendTauriInvoke) {
  const raw = await invoke<Record<string, unknown>>("video_output_recording_status");
  if (raw === null || typeof raw !== "object" || Array.isArray(raw)) {
    throw new Error("Recording status response is invalid.");
  }
  if (typeof raw.active !== "boolean" || typeof raw.audio_requested !== "boolean"
    || typeof raw.audio_included !== "boolean") {
    throw new Error("Recording status response is invalid.");
  }
  const outputId = raw.output_id === null || raw.output_id === undefined
    ? null : finiteNonNegative(raw.output_id, "output_id");
  const startedUnixMs = raw.started_unix_ms === null || raw.started_unix_ms === undefined
    ? null : finiteNonNegative(raw.started_unix_ms, "started_unix_ms");
  return {
    ok: true,
    recording: {
      active: raw.active,
      output_id: outputId,
      path: optionalText(raw.path, "path", MAX_PATH_LENGTH),
      width: finiteNonNegative(raw.width, "width"),
      height: finiteNonNegative(raw.height, "height"),
      frame_rate: finiteNonNegative(raw.frame_rate, "frame_rate"),
      frames_written: finiteNonNegative(raw.frames_written, "frames_written"),
      dropped_frames: finiteNonNegative(raw.dropped_frames, "dropped_frames"),
      audio_requested: raw.audio_requested,
      audio_included: raw.audio_included,
      audio_track_count: finiteNonNegative(raw.audio_track_count, "audio_track_count"),
      started_unix_ms: startedUnixMs,
      last_error: optionalText(raw.last_error, "last_error", MAX_ERROR_LENGTH),
    },
    control_boundary: "Read-only observation; recording start/stop/finalize remains a separately fenced operation.",
  };
}
