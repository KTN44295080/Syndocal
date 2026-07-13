import { createEffect, createSignal, For, Show } from "solid-js";
import type { TimelineEventDraft } from "../editorDrafts";
import type { AudioAnalysisSummary, TimelineCueEventSummary, TimelineTrackKind } from "../types";
import { TimelineOverview, type TimelineOverviewAutomationRange, type TimelineOverviewEvent } from "./TimelineOverview";
import {
  TimelineSceneBlocksEditor,
  type TimelineSceneBlockCueOption,
  type TimelineSceneBlockRow,
} from "./TimelineSceneBlocksEditor";

export type TimelineSnapMode = "Off" | "Beat" | "Bar" | "Grid";

const formatTimelineHeaderTime = (timeMs: number) => {
  const safeMs = Math.max(0, Math.round(timeMs));
  const milliseconds = String(safeMs % 1000).padStart(3, "0");
  const totalSeconds = Math.floor(safeMs / 1000);
  const seconds = String(totalSeconds % 60).padStart(2, "0");
  const totalMinutes = Math.floor(totalSeconds / 60);
  if (totalMinutes === 0) return `${Math.floor(totalSeconds)}.${milliseconds}s`;
  const minutes = String(totalMinutes % 60).padStart(2, "0");
  const hours = Math.floor(totalMinutes / 60);
  return hours > 0
    ? `${hours}:${minutes}:${seconds}.${milliseconds}`
    : `${totalMinutes}:${seconds}.${milliseconds}`;
};

interface AudioBeatMarker {
  time_ms: number;
  x: number;
}

interface TimelineCueEventsPanelProps {
  positionMs: number;
  durationMs: number;
  playing: boolean;
  executingLive: boolean;
  cuesCount: number;
  lightingAutomationCount: number;
  videoAutomationCount: number;
  overviewEvents: TimelineOverviewEvent[];
  overviewAutomationRanges: TimelineOverviewAutomationRange[];
  selectedAutomationRangeId: string | null;
  overviewPlayheadX: number;
  audioAnalysis: AudioAnalysisSummary | null;
  audioWaveformPoints: string;
  audioSpectrumPaths: { bass: string; mid: string; high: string };
  audioBeatMarkers: AudioBeatMarker[];
  snapMode: TimelineSnapMode;
  gridMs: number;
  selectedCueId: number | null;
  eventTimeMs: number;
  blockDurationMs: number;
  blockLoopCount: number;
  blockJumpToEventId: number | null;
  track: TimelineTrackKind;
  cueOptions: TimelineSceneBlockCueOption[];
  eventRows: TimelineSceneBlockRow[];
  timelineEventDraft: (event: TimelineCueEventSummary) => TimelineEventDraft;
  onSeek: (timeMs: number) => void | Promise<void>;
  onPause: () => void | Promise<void>;
  onPlay: () => void | Promise<void>;
  onSeekRatio: (ratio: number) => void;
  onMoveEventRatio: (eventId: number, ratio: number) => void | Promise<void>;
  onSelectAutomationRange: (range: TimelineOverviewAutomationRange) => void;
  onMoveAutomationRangeRatio: (range: TimelineOverviewAutomationRange, ratio: number) => void | Promise<void>;
  onResizeAutomationRangeRatio: (
    range: TimelineOverviewAutomationRange,
    edge: "start" | "end",
    ratio: number,
  ) => void | Promise<void>;
  onMoveAutomationKeyframeRatio: (
    range: TimelineOverviewAutomationRange,
    keyframeIndex: number,
    ratio: number,
  ) => void | Promise<void>;
  onAnalyzeAudio: () => void | Promise<void>;
  onClearAudio: () => void | Promise<void>;
  onApplyAudioBpm: () => void | Promise<void>;
  onSnapMode: (mode: TimelineSnapMode) => void;
  onGridMs: (value: number) => void;
  onSnapDrafts: () => void;
  onSnapItems: () => void | Promise<void>;
  onSelectedCueId: (cueId: number) => void;
  onEventTimeMs: (timeMs: number) => void;
  onBlockDurationMs: (durationMs: number) => void;
  onBlockLoopCount: (loopCount: number) => void;
  onBlockJumpToEventId: (eventId: number | null) => void;
  onTrack: (track: TimelineTrackKind) => void;
  onAddEvent: () => void | Promise<void>;
  onAddEventAtPlayhead: () => void | Promise<void>;
  onUpdateEventDraft: (event: TimelineCueEventSummary, patch: Partial<TimelineEventDraft>) => void;
  onSaveEvent: (event: TimelineCueEventSummary) => void | Promise<void>;
  onRemoveEvent: (event: TimelineCueEventSummary) => void | Promise<void>;
  onOpenSourceCue: (cueId: number) => void;
}

export function TimelineCueEventsPanel(props: TimelineCueEventsPanelProps) {
  const [selectedSceneBlockEventId, setSelectedSceneBlockEventId] = createSignal<number | null>(null);
  const [sceneBlockSelectionRevision, setSceneBlockSelectionRevision] = createSignal(0);
  const selectSceneBlockEvent = (eventId: number) => {
    setSelectedSceneBlockEventId(eventId);
    setSceneBlockSelectionRevision((revision) => revision + 1);
  };
  createEffect(() => {
    const selectedEventId = selectedSceneBlockEventId();
    if (selectedEventId !== null && !props.eventRows.some((event) => event.id === selectedEventId)) {
      setSelectedSceneBlockEventId(null);
    }
  });
  return (
    <>
      <div class="panelHeader">
        <h2>Timeline</h2>
        <div class="timelineHeaderMeta" aria-label="Timeline summary">
          <span>
            <small>Blocks</small>
            <strong>{props.eventRows.filter((event) => event.duration_ms > 0).length}</strong>
          </span>
          <span>
            <small>Points</small>
            <strong>{props.eventRows.filter((event) => event.duration_ms === 0).length}</strong>
          </span>
          <span>
            <small>Light</small>
            <strong>{props.lightingAutomationCount}</strong>
          </span>
          <span>
            <small>Video</small>
            <strong>{props.videoAutomationCount}</strong>
          </span>
          <span class="timelineTimeStat" title={`${props.positionMs} / ${props.durationMs} ms`}>
            <small>Time</small>
            <strong data-no-localize>{formatTimelineHeaderTime(props.positionMs)} / {formatTimelineHeaderTime(props.durationMs)}</strong>
          </span>
        </div>
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
        automationRanges={props.overviewAutomationRanges}
        selectedRangeId={props.selectedAutomationRangeId}
        selectedEventId={selectedSceneBlockEventId()}
        playheadX={props.overviewPlayheadX}
        onSeekRatio={props.onSeekRatio}
        onSeekTime={(timeMs) => void props.onSeek(timeMs)}
        onSelectAutomationRange={props.onSelectAutomationRange}
        onSelectEvent={selectSceneBlockEvent}
        onMoveEventRatio={(eventId, ratio) => void props.onMoveEventRatio(eventId, ratio)}
        onMoveAutomationRangeRatio={(range, ratio) => void props.onMoveAutomationRangeRatio(range, ratio)}
        onResizeAutomationRangeRatio={(range, edge, ratio) => void props.onResizeAutomationRangeRatio(range, edge, ratio)}
        onMoveAutomationKeyframeRatio={(range, keyframeIndex, ratio) =>
          void props.onMoveAutomationKeyframeRatio(range, keyframeIndex, ratio)
        }
      />
      <div class="audioAnalysisPanel">
        <div class="panelHeader">
          <h3>Audio</h3>
          <div class="buttonRow">
            <button onClick={() => void props.onAnalyzeAudio()}>Analyze Audio</button>
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
              <Show when={analysis().spectrum.length > 0}>
                <svg class="audioSpectrumView" viewBox="0 0 100 36" role="img" aria-label="Audio FFT bass mid high over time">
                  <rect x="0" y="0" width="100" height="36" />
                  <line x1="0" x2="100" y1="12" y2="12" />
                  <line x1="0" x2="100" y1="24" y2="24" />
                  <polyline class="bass" points={props.audioSpectrumPaths.bass} />
                  <polyline class="mid" points={props.audioSpectrumPaths.mid} />
                  <polyline class="high" points={props.audioSpectrumPaths.high} />
                  <text x="1" y="5">BASS</text>
                  <text x="1" y="17">MID</text>
                  <text x="1" y="29">HIGH</text>
                </svg>
              </Show>
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
        <button
          onClick={() => void props.onSnapItems()}
          disabled={
            props.snapMode === "Off" ||
            (props.eventRows.length + props.lightingAutomationCount + props.videoAutomationCount === 0)
          }
        >
          Snap Items
        </button>
      </div>
      <TimelineSceneBlocksEditor
        positionMs={props.positionMs}
        executionLive={props.executingLive}
        selectedCueId={props.selectedCueId}
        startMs={props.eventTimeMs}
        durationMs={props.blockDurationMs}
        loopCount={props.blockLoopCount}
        track={props.track}
        jumpToEventId={props.blockJumpToEventId}
        cueOptions={props.cueOptions}
        eventRows={props.eventRows}
        selectedEventId={selectedSceneBlockEventId()}
        selectionRevision={sceneBlockSelectionRevision()}
        timelineEventDraft={props.timelineEventDraft}
        onSelectedCueId={props.onSelectedCueId}
        onStartMs={props.onEventTimeMs}
        onDurationMs={props.onBlockDurationMs}
        onLoopCount={props.onBlockLoopCount}
        onTrack={props.onTrack}
        onJumpToEventId={props.onBlockJumpToEventId}
        onAddBlock={props.onAddEvent}
        onAddBlockAtPlayhead={props.onAddEventAtPlayhead}
        onUpdateEventDraft={props.onUpdateEventDraft}
        onSaveEvent={props.onSaveEvent}
        onRemoveEvent={props.onRemoveEvent}
        onSelectEvent={selectSceneBlockEvent}
        onOpenSourceCue={props.onOpenSourceCue}
      />
    </>
  );
}
