import { createMemo } from "solid-js";
import type { VideoRecordingStatus } from "../types";

interface VideoRecordingStatusBarProps {
  recordingStatus: VideoRecordingStatus;
  programAudioEnabled: boolean;
  selectedOutputId: number | null;
  onStartRecording: (outputId: number, includeAudio: boolean) => void | Promise<void>;
  onStopRecording: () => void | Promise<void>;
}

export function VideoRecordingStatusBar(props: VideoRecordingStatusBarProps) {
  const recordingState = createMemo(() => props.recordingStatus.state
    ?? (props.recordingStatus.active ? "Recording" : props.recordingStatus.last_error ? "Fault" : "Idle"));
  const recordingInProgress = createMemo(() => ["Preparing", "Recording", "Finalizing"].includes(recordingState()));
  // Only the video output owner records (DEC-RECORD-OWN-001); the backend
  // refuses a start anyway, this only keeps the button truthful.
  const recordingBlocked = createMemo(() => !recordingInProgress()
    && props.recordingStatus.admission_allowed === false);
  const takeoverWaiting = createMemo(() => props.recordingStatus.takeover?.state === "pending"
    || props.recordingStatus.takeover?.state === "blocked");
  const recordingStatusText = createMemo(() => recordingInProgress()
    ? `${recordingState()} · ${props.recordingStatus.dropped_frames} dropped · ${props.recordingStatus.frames_written} frames · ${props.recordingStatus.width}x${props.recordingStatus.height} @ ${props.recordingStatus.frame_rate}fps · ${props.recordingStatus.audio_included ? `${props.recordingStatus.audio_track_count} audio` : "silent"}`
    // A waiting takeover take matters most: it explains why nothing records yet.
    : takeoverWaiting()
      ? `Takeover recording ${props.recordingStatus.takeover?.state} · ${props.recordingStatus.takeover?.reason ?? ""}`
      : recordingState() === "Complete"
        ? `${props.recordingStatus.end_reason === "ownership_revoked" ? `Ended: ${props.recordingStatus.end_detail ?? "video output ownership changed"}` : "Complete"} · ${props.recordingStatus.frames_written} frames · ${props.recordingStatus.path ?? "artifact published"}${props.recordingStatus.last_error ? ` · ${props.recordingStatus.last_error}` : ""}`
        : props.recordingStatus.last_error
          ?? (recordingBlocked()
            ? props.recordingStatus.admission_reason ?? "Recording needs this PC to own video output."
            : `Ready · records H.264 MP4${props.programAudioEnabled ? " and active Program audio" : " without audio"}.`));

  return (
    <div class={`videoRecordingBar ${recordingInProgress() ? "active" : ""}`} aria-live="polite">
      <div>
        <small>{recordingInProgress() ? `● ${recordingState().toUpperCase()}` : "OUTPUT RECORD"}</small>
        <span
          class={props.recordingStatus.last_error ? "videoRecordingError" : ""}
          role={props.recordingStatus.last_error ? "alert" : undefined}
          title={recordingStatusText()}
        >
          {recordingStatusText()}
        </span>
      </div>
      <button
        class={recordingInProgress() ? "danger" : ""}
        disabled={recordingState() === "Finalizing"
          || (!recordingInProgress() && (props.selectedOutputId === null || recordingBlocked()))}
        title={recordingBlocked() ? props.recordingStatus.admission_reason ?? undefined : undefined}
        onClick={() => recordingInProgress()
          ? recordingState() !== "Finalizing" && void props.onStopRecording()
          : props.selectedOutputId !== null && void props.onStartRecording(props.selectedOutputId, props.programAudioEnabled)}
      >
        {recordingState() === "Finalizing" ? "Finalizing…" : recordingInProgress() ? "Stop Recording" : "Record Output"}
      </button>
    </div>
  );
}
