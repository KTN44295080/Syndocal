import { For, Show } from "solid-js";
import type { TimelineEventDraft } from "../editorDrafts";
import type { AudioAnalysisSummary, TimelineCueEventSummary, TimelineTrackKind } from "../types";
import { TimelineOverview, type TimelineOverviewEvent } from "./TimelineOverview";

export type TimelineSnapMode = "Off" | "Beat" | "Bar" | "Grid";

export interface TimelineCueOption {
  id: number;
  label: string;
}

export interface TimelineEventRow extends TimelineCueEventSummary {
  cue_label: string;
}

interface AudioBeatMarker {
  time_ms: number;
  x: number;
}

interface TimelineCueEventsPanelProps {
  positionMs: number;
  durationMs: number;
  playing: boolean;
  cuesCount: number;
  overviewEvents: TimelineOverviewEvent[];
  overviewPlayheadX: number;
  audioAnalysis: AudioAnalysisSummary | null;
  audioWaveformPoints: string;
  audioBeatMarkers: AudioBeatMarker[];
  snapMode: TimelineSnapMode;
  gridMs: number;
  selectedCueId: number | null;
  eventTimeMs: number;
  track: TimelineTrackKind;
  cueOptions: TimelineCueOption[];
  eventRows: TimelineEventRow[];
  timelineEventDraft: (event: TimelineCueEventSummary) => TimelineEventDraft;
  onSeek: (timeMs: number) => void | Promise<void>;
  onPause: () => void | Promise<void>;
  onPlay: () => void | Promise<void>;
  onSeekRatio: (ratio: number) => void;
  onMoveEventRatio: (eventId: number, ratio: number) => void | Promise<void>;
  onAnalyzeAudio: () => void | Promise<void>;
  onClearAudio: () => void | Promise<void>;
  onApplyAudioBpm: () => void | Promise<void>;
  onSnapMode: (mode: TimelineSnapMode) => void;
  onGridMs: (value: number) => void;
  onSnapDrafts: () => void;
  onSelectedCueId: (cueId: number) => void;
  onEventTimeMs: (timeMs: number) => void;
  onTrack: (track: TimelineTrackKind) => void;
  onAddEvent: () => void | Promise<void>;
  onAddEventAtPlayhead: () => void | Promise<void>;
  onUpdateEventDraft: (event: TimelineCueEventSummary, patch: Partial<TimelineEventDraft>) => void;
  onSaveEvent: (event: TimelineCueEventSummary) => void | Promise<void>;
  onRemoveEvent: (eventId: number) => void | Promise<void>;
}

export function TimelineCueEventsPanel(props: TimelineCueEventsPanelProps) {
  return (
    <>
      <div class="panelHeader">
        <h2>Timeline</h2>
        <span>
          {props.positionMs} / {props.durationMs} ms
        </span>
      </div>
      <div class="timelineTransport">
        <button onClick={() => void props.onSeek(0)}>|&lt;</button>
        <button onClick={() => void props.onPause()} disabled={!props.playing}>
          Pause
        </button>
        <button class="primary" onClick={() => void props.onPlay()} disabled={props.durationMs === 0 || props.playing}>
          Play
        </button>
      </div>
      <input
        type="range"
        min="0"
        max={Math.max(props.durationMs, 1)}
        value={props.positionMs}
        onInput={(event) => void props.onSeek(Number(event.currentTarget.value))}
      />
      <TimelineOverview
        events={props.overviewEvents}
        playheadX={props.overviewPlayheadX}
        onSeekRatio={props.onSeekRatio}
        onSeekTime={(timeMs) => void props.onSeek(timeMs)}
        onMoveEventRatio={(eventId, ratio) => void props.onMoveEventRatio(eventId, ratio)}
      />
      <div class="audioAnalysisPanel">
        <div class="panelHeader">
          <h3>Audio</h3>
          <div class="buttonRow">
            <button onClick={() => void props.onAnalyzeAudio()}>Analyze WAV</button>
            <button onClick={() => void props.onClearAudio()} disabled={!props.audioAnalysis}>
              Clear
            </button>
          </div>
        </div>
        <Show when={props.audioAnalysis}>
          {(analysis) => (
            <>
              <div class="audioStats">
                <span>{Math.round(analysis().duration_ms / 1000)}s</span>
                <span>{analysis().sample_rate} Hz</span>
                <span>{analysis().channels} ch</span>
                <span>{analysis().estimated_bpm ? `${analysis().estimated_bpm!.toFixed(1)} BPM` : "BPM n/a"}</span>
                <button onClick={() => void props.onApplyAudioBpm()} disabled={!analysis().estimated_bpm}>
                  Apply BPM
                </button>
              </div>
              <svg class="waveformView" viewBox="0 0 100 36" role="img">
                <rect x="0" y="0" width="100" height="36" />
                <polyline points={props.audioWaveformPoints} />
                <For each={props.audioBeatMarkers}>
                  {(beat) => <line class="beatMarker" x1={beat.x} x2={beat.x} y1="0" y2="36" />}
                </For>
              </svg>
              <small>{analysis().path}</small>
            </>
          )}
        </Show>
      </div>
      <div class="timelineSnapControls">
        <label>
          Snap
          <select value={props.snapMode} onInput={(event) => props.onSnapMode(event.currentTarget.value as TimelineSnapMode)}>
            <option value="Off">Off</option>
            <option value="Beat">Beat</option>
            <option value="Bar">Bar</option>
            <option value="Grid">Grid</option>
          </select>
        </label>
        <label>
          Grid ms
          <input
            type="number"
            min="1"
            value={props.gridMs}
            disabled={props.snapMode !== "Grid"}
            onInput={(event) => props.onGridMs(Number(event.currentTarget.value))}
          />
        </label>
        <button onClick={props.onSnapDrafts} disabled={props.snapMode === "Off"}>
          Snap Times
        </button>
      </div>
      <div class="timelineForm">
        <label>
          Cue
          <select value={props.selectedCueId ?? ""} onInput={(event) => props.onSelectedCueId(Number(event.currentTarget.value))} disabled={props.cuesCount === 0}>
            <For each={props.cueOptions}>{(cue) => <option value={cue.id}>{cue.label}</option>}</For>
          </select>
        </label>
        <label>
          Time ms
          <input type="number" min="0" value={props.eventTimeMs} onInput={(event) => props.onEventTimeMs(Number(event.currentTarget.value))} />
        </label>
        <label>
          Track
          <select value={props.track} onInput={(event) => props.onTrack(event.currentTarget.value as TimelineTrackKind)}>
            <option value="Lighting">Lighting</option>
            <option value="Video">Video</option>
          </select>
        </label>
        <button class="primary" onClick={() => void props.onAddEvent()} disabled={props.cuesCount === 0}>
          Add Event
        </button>
        <button onClick={() => void props.onAddEventAtPlayhead()} disabled={props.cuesCount === 0}>
          At Playhead
        </button>
      </div>
      <div class="timelineList">
        <For each={props.eventRows}>
          {(event) => {
            const draft = () => props.timelineEventDraft(event);
            return (
              <div class="timelineItem timelineEventItem">
                <div>
                  <strong>{event.time_ms} ms</strong>
                  <span>{event.cue_label} / {event.track}</span>
                </div>
                <label>
                  Cue
                  <select
                    value={draft().cue_id}
                    onInput={(inputEvent) =>
                      props.onUpdateEventDraft(event, {
                        cue_id: Number(inputEvent.currentTarget.value),
                      })
                    }
                  >
                    <For each={props.cueOptions}>{(cue) => <option value={cue.id}>{cue.label}</option>}</For>
                  </select>
                </label>
                <label>
                  Time
                  <input
                    type="number"
                    min="0"
                    value={draft().time_ms}
                    onInput={(inputEvent) =>
                      props.onUpdateEventDraft(event, {
                        time_ms: Number(inputEvent.currentTarget.value),
                      })
                    }
                  />
                </label>
                <label>
                  Track
                  <select
                    value={draft().track}
                    onInput={(inputEvent) =>
                      props.onUpdateEventDraft(event, {
                        track: inputEvent.currentTarget.value as TimelineTrackKind,
                      })
                    }
                  >
                    <option value="Lighting">Lighting</option>
                    <option value="Video">Video</option>
                  </select>
                </label>
                <div class="buttonRow">
                  <button onClick={() => void props.onSaveEvent(event)}>Save</button>
                  <button onClick={() => void props.onRemoveEvent(event.id)}>Remove</button>
                </div>
              </div>
            );
          }}
        </For>
      </div>
    </>
  );
}
