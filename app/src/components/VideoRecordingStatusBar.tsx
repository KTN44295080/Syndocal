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
  const recordingStatusText = createMemo(() => recordingInProgress()
    ? `${recordingState()} · ${props.recordingStatus.dropped_frames} dropped · ${props.recordingStatus.frames_written} frames · ${props.recordingStatus.width}x${props.recordingStatus.height} @ ${props.recordingStatus.frame_rate}fps · ${props.recordingStatus.audio_included ? `${props.recordingStatus.audio_track_count} audio` : "silent"}`
    : recordingState() === "Complete"
      ? `Complete · ${props.recordingStatus.frames_written} frames · ${props.recordingStatus.path ?? "artifact published"}`
      : props.recordingStatus.last_error ?? `Ready · records H.264 MP4${props.programAudioEnabled ? " and active Program audio" : " without audio"}.`);

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
        disabled={recordingState() === "Finalizing" || (!recordingInProgress() && props.selectedOutputId === null)}
        onClick={() => recordingInProgress()
          ? recordingState() !== "Finalizing" && void props.onStopRecording()
          : props.selectedOutputId !== null && void props.onStartRecording(props.selectedOutputId, props.programAudioEnabled)}
      >
        {recordingState() === "Finalizing" ? "Finalizing…" : recordingInProgress() ? "Stop Recording" : "Record Output"}
      </button>
    </div>
  );
}
