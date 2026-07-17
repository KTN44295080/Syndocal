import { createEffect, createSignal, For, Show } from "solid-js";
import type { TimelineEventDraft } from "../editorDrafts";
import type {
  AudioAnalysisSummary,
  TimelineAudioClipSummary,
  TimelineCueEventSummary,
  TimelineLayerKind,
  TimelineLayerSummary,
  TimelineTrackKind,
} from "../types";
import type { TimelineCueDragState } from "../timelineCueDrag";
import { timelineLayerIdForEvent } from "../timelineLayers";
import {
  TIMELINE_MIN_VISIBLE_WINDOW_MS,
  timelineVisibleWindowSpanMs,
  type TimelineVisibleWindow,
} from "../timelineViewport";
import {
  TimelineOverview,
  type TimelineOverviewAutomationRange,
  type TimelineOverviewEvent,
  type TimelineOverviewOverlapCluster,
} from "./TimelineOverview";
import {
  TimelineSceneBlocksEditor,
  type TimelineSceneBlockCueOption,
  type TimelineSceneBlockRow,
} from "./TimelineSceneBlocksEditor";
import { TimelineLayerToolbar } from "./TimelineLayerToolbar";
import {
  naturalTimelineBlockDurationMs,
  type TimelineStretchMode,
} from "../timelineBlockGestures";

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
  childTimelineLabel: string | null;
  cueColors?: Record<number, string>;
  positionMs: number;
  bpm: number;
  durationMs: number;
  playing: boolean;
  executingLive: boolean;
  cuesCount: number;
  lightingAutomationCount: number;
  videoAutomationCount: number;
  overviewEvents: TimelineOverviewEvent[];
  timelineLayers: TimelineLayerSummary[];
  legacyTimelineLayers: boolean;
  timelineCueDrag: TimelineCueDragState | null;
  overviewMarkerAriaLabel: (event: TimelineOverviewEvent) => string;
  overviewAutomationRanges: TimelineOverviewAutomationRange[];
  overviewOverlapClusters: TimelineOverviewOverlapCluster[];
  overlapClusterMemberships: { id: string; member_ids: number[] }[];
  selectedAutomationRangeId: string | null;
  overviewPlayheadX: number;
  visibleWindow: TimelineVisibleWindow;
  overviewShowDurationMs: number;
  overviewEditExtentMs: number;
  selectedEventId: number | null;
  selectionRevision: number;
  audioAnalysis: AudioAnalysisSummary | null;
  audioClips: TimelineAudioClipSummary[];
  audioOffsetMs: number;
  audioMuted: boolean;
  audioWaveformPoints: string;
  audioSpectrumPaths: { bass: string; mid: string; high: string };
  audioBeatMarkers: AudioBeatMarker[];
  snapMode: TimelineSnapMode;
  gridMs: number;
  selectedCueId: number | null;
  selectedCueIsSuperScene: boolean;
  eventTimeMs: number;
  blockDurationMs: number;
  blockLoopCount: number;
  blockJumpToEventId: number | null;
  track: TimelineTrackKind;
  cueOptions: TimelineSceneBlockCueOption[];
  eventRows: TimelineSceneBlockRow[];
  timelineEventDraft: (event: TimelineCueEventSummary) => TimelineEventDraft;
  onSeek: (timeMs: number) => void | Promise<void>;
  onExitChildTimeline: () => void;
  onOpenSuperScene: (cueId: number) => void;
  onPause: () => void | Promise<void>;
  onPlay: () => void | Promise<void>;
  onSeekOverviewTime: (timeMs: number) => void;
  onMoveEventPlacement: (
    eventId: number,
    timeMs: number,
    layerId: number,
    snapEnabled: boolean,
  ) => void | Promise<void>;
  onResizeEventTime: (
    eventId: number,
    edge: "start" | "end",
    timeMs: number,
    mode: TimelineStretchMode,
    snapEnabled: boolean,
  ) => void | Promise<void>;
  onSetEventFade: (eventId: number, edge: "in" | "out", fadeMs: number, snapEnabled: boolean) => void | Promise<void>;
  onSelectAutomationRange: (range: TimelineOverviewAutomationRange) => void;
  onMoveAutomationRangeTime: (range: TimelineOverviewAutomationRange, timeMs: number) => void | Promise<void>;
  onResizeAutomationRangeTime: (
    range: TimelineOverviewAutomationRange,
    edge: "start" | "end",
    timeMs: number,
  ) => void | Promise<void>;
  onMoveAutomationKeyframeTime: (
    range: TimelineOverviewAutomationRange,
    keyframeIndex: number,
    timeMs: number,
  ) => void | Promise<void>;
  onSelectEvent: (eventId: number, reveal: boolean) => void;
  onFitOverview: () => void;
  onZoomOverview: (scale: number) => void;
  onPanOverview: (direction: -1 | 1) => void;
  onSetVisibleWindow: (window: TimelineVisibleWindow) => void;
  onZoomOverviewAt: (anchorMs: number, scale: number) => void;
  onRevealSelected: () => void;
  onRevealPlayhead: () => void;
  onAnalyzeAudio: () => void | Promise<void>;
  onClearAudio: () => void | Promise<void>;
  onApplyAudioBpm: () => void | Promise<void>;
  onAddAudioClip: (layerId: number) => void | Promise<void>;
  onUpdateAudioClip: (clip: TimelineAudioClipSummary) => void | Promise<void>;
  onRemoveAudioClip: (clipId: number) => void | Promise<void>;
  onSetAudioMaster: (offsetMs: number, muted: boolean) => void | Promise<void>;
  onSnapMode: (mode: TimelineSnapMode) => void;
  onGridMs: (value: number) => void;
  onSnapDrafts: () => void;
  onSnapItems: () => void | Promise<void>;
  snapTimeMs: (timeMs: number) => number;
  onPlaceArmedCue: (
    cueId: number,
    timeMs: number,
    layerId: number,
    durationMs: number,
    mode: TimelineStretchMode,
    snapEnabled: boolean,
  ) => void | Promise<void>;
  onSelectedCueId: (cueId: number) => void;
  onOpenOrCreateSuperScene: (cueId: number) => void | Promise<void>;
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
  onAddTimelineLayer: (label: string, kind: TimelineLayerKind) => void | Promise<void>;
  onUpdateTimelineLayer: (layer: TimelineLayerSummary) => void | Promise<void>;
  onRemoveTimelineLayer: (layerId: number, reassignToLayerId: number | null) => void | Promise<void>;
  onReorderTimelineLayer: (layerId: number, direction: -1 | 1) => void | Promise<void>;
  onTimelineStatus: (message: string) => void;
}

export function TimelineCueEventsPanel(props: TimelineCueEventsPanelProps) {
  let audioClipRemoveDialog: HTMLDialogElement | undefined;
  const [stretchMode, setStretchMode] = createSignal<TimelineStretchMode>("RATE");
  const [magnetEnabled, setMagnetEnabled] = createSignal(true);
  const [armedCueId, setArmedCueId] = createSignal<number | null>(null);
  const [selectedAudioClipId, setSelectedAudioClipId] = createSignal<number | null>(null);
  const [pendingRemoveAudioClipId, setPendingRemoveAudioClipId] = createSignal<number | null>(null);
  const selectedAudioClip = () => props.audioClips.find((clip) => clip.id === selectedAudioClipId()) ?? null;
  const pendingRemoveAudioClip = () =>
    props.audioClips.find((clip) => clip.id === pendingRemoveAudioClipId()) ?? null;
  createEffect(() => {
    if (selectedAudioClipId() !== null && !selectedAudioClip()) setSelectedAudioClipId(null);
  });
  const armedCue = () => {
    const cue = props.cueOptions.find((candidate) => candidate.id === armedCueId());
    return cue ? {
      id: cue.id,
      label: cue.label,
      authored_beats: cue.authored_beats,
      natural_duration_ms: naturalTimelineBlockDurationMs(cue.authored_beats, props.bpm, props.blockDurationMs),
    } : null;
  };
  const toggleArmedCue = (cueId: number | null) => {
    setArmedCueId((current) => current === cueId ? null : cueId);
    if (cueId !== null) props.onTimelineStatus(
      armedCueId() === cueId
        ? `Cue ${cueId} armed: double-click or sweep a Lighting lane to place it.`
        : `Cue ${cueId} disarmed.`,
    );
  };
  const [overlapFilter, setOverlapFilter] = createSignal<{
    eventIds: number[];
    label: string;
    clusterIds: string[];
  } | null>(null);
  const overlapFilterEventIds = () => overlapFilter()?.eventIds ?? null;
  const overlapFilterLabel = () => overlapFilter()?.label ?? null;
  const overlapFilterClusterIds = () => overlapFilter()?.clusterIds ?? null;
  const inspectOverlapCluster = (cluster: TimelineOverviewOverlapCluster) => {
    setOverlapFilter({
      eventIds: cluster.member_ids,
      label: `${cluster.track} overlap ×${cluster.count}`,
      clusterIds: cluster.source_cluster_ids ?? [cluster.id],
    });
    if (
      cluster.member_ids.length > 0 &&
      (props.selectedEventId === null || !cluster.member_ids.includes(props.selectedEventId))
    ) {
      props.onSelectEvent(cluster.member_ids[0], false);
    }
  };
  const clearOverlapFilter = () => {
    setOverlapFilter(null);
  };
  createEffect(() => {
    const clusterIds = overlapFilterClusterIds();
    const eventIds = overlapFilterEventIds();
    if (clusterIds === null || eventIds === null) return;
    const liveClusters = clusterIds.map((clusterId) =>
      props.overlapClusterMemberships.find((cluster) => cluster.id === clusterId));
    const liveMemberIds = new Set(liveClusters.flatMap((cluster) => cluster?.member_ids ?? []));
    if (
      liveClusters.some((cluster) => !cluster) ||
      liveMemberIds.size !== eventIds.length ||
      eventIds.some((eventId) => !liveMemberIds.has(eventId))
    ) {
      clearOverlapFilter();
    }
  });
  return (
    <>
      <div class="panelHeader">
        <div class="timelineTitleGroup">
          <h2>Timeline</h2>
          <Show when={props.childTimelineLabel}>
            {(label) => (
              <nav class="timelineBreadcrumb" aria-label="Timeline breadcrumb" data-timeline-breadcrumb>
                <button type="button" onClick={props.onExitChildTimeline}>Show</button>
                <span aria-hidden="true" data-no-localize>›</span>
                <strong data-child-timeline-label data-no-localize>{label()}</strong>
              </nav>
            )}
          </Show>
        </div>
        <div
          class={`timelineHeaderMeta ${props.executingLive ? "executingLive" : ""}`}
          aria-label="Timeline summary"
        >
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
        <TimelineLayerToolbar
          layers={props.timelineLayers}
          legacyMode={props.legacyTimelineLayers}
          eventCountForLayer={(layerId) => props.eventRows.filter((event) =>
            timelineLayerIdForEvent(props.timelineLayers, event) === layerId).length}
          onAddLayer={props.onAddTimelineLayer}
          onUpdateLayer={props.onUpdateTimelineLayer}
          onRemoveLayer={props.onRemoveTimelineLayer}
          onReorderLayer={props.onReorderTimelineLayer}
        />
      </div>
      <div class="timelineTransport">
        <button onClick={() => void props.onSeek(0)} disabled={props.childTimelineLabel !== null}>|&lt;</button>
        <button onClick={() => void props.onPause()} disabled={props.childTimelineLabel !== null || !props.playing}>
          Pause
        </button>
        <button class="primary" onClick={() => void props.onPlay()} disabled={props.childTimelineLabel !== null || props.durationMs === 0 || props.playing}>
          Play
        </button>
      </div>
      <input
        type="range"
        min="0"
        max={Math.max(props.durationMs, 1)}
        value={props.positionMs}
        disabled={props.childTimelineLabel !== null}
        onInput={(event) => void props.onSeek(Number(event.currentTarget.value))}
      />
      <nav class="timelineViewportToolbar" aria-label="Timeline visible range controls">
        <button
          type="button"
          onClick={() => props.onPanOverview(-1)}
          disabled={props.visibleWindow.start_ms <= 0}
        >
          Pan Prev
        </button>
        <button
          type="button"
          onClick={() => props.onZoomOverview(2)}
          disabled={timelineVisibleWindowSpanMs(props.visibleWindow) >= props.overviewEditExtentMs}
        >
          Zoom Out
        </button>
        <button type="button" onClick={props.onFitOverview}>Fit All</button>
        <button
          type="button"
          onClick={() => props.onZoomOverview(0.5)}
          disabled={timelineVisibleWindowSpanMs(props.visibleWindow) <= TIMELINE_MIN_VISIBLE_WINDOW_MS}
        >
          Zoom In
        </button>
        <button
          type="button"
          onClick={() => props.onPanOverview(1)}
          disabled={props.visibleWindow.end_ms >= props.overviewEditExtentMs}
        >
          Pan Next
        </button>
        <button type="button" onClick={props.onRevealSelected} disabled={props.selectedEventId === null}>
          Reveal Selected
        </button>
        <button type="button" onClick={props.onRevealPlayhead}>Reveal Playhead</button>
        <output
          class="timelineVisibleRange tabularNums"
          aria-label="Visible timeline range"
          data-visible-start-ms={props.visibleWindow.start_ms}
          data-visible-end-ms={props.visibleWindow.end_ms}
          data-show-duration-ms={props.overviewShowDurationMs}
          data-edit-extent-ms={props.overviewEditExtentMs}
        >
          {formatTimelineHeaderTime(props.visibleWindow.start_ms)} – {formatTimelineHeaderTime(props.visibleWindow.end_ms)}
        </output>
      </nav>
      <div class="timelineDirectToolbar" aria-label="Timeline block direct manipulation">
        <span class="timelineDirectToolbarLabel">Stretch mode</span>
        <div class="timelineStretchModeToggle" role="group" aria-label="Stretch mode" data-timeline-stretch-mode-toggle>
          <button
            type="button"
            classList={{ active: stretchMode() === "RATE" }}
            aria-pressed={stretchMode() === "RATE"}
            data-timeline-stretch-mode="RATE"
            onClick={() => setStretchMode("RATE")}
          >
            RATE
          </button>
          <button
            type="button"
            classList={{ active: stretchMode() === "WINDOW" }}
            aria-pressed={stretchMode() === "WINDOW"}
            data-timeline-stretch-mode="WINDOW"
            onClick={() => setStretchMode("WINDOW")}
          >
            WINDOW
          </button>
        </div>
        <button
          type="button"
          classList={{ active: magnetEnabled() }}
          aria-pressed={magnetEnabled()}
          aria-label={magnetEnabled() ? "Disable magnet snap" : "Enable magnet snap"}
          data-timeline-magnet-toggle
          onClick={() => setMagnetEnabled((enabled) => !enabled)}
        >
          Magnet
        </button>
        <button
          type="button"
          classList={{ active: armedCueId() === props.selectedCueId }}
          aria-pressed={props.selectedCueId !== null && armedCueId() === props.selectedCueId}
          aria-label={armedCueId() === props.selectedCueId ? "Disarm Cue" : "Arm Cue"}
          data-timeline-arm-cue={props.selectedCueId ?? undefined}
          disabled={props.selectedCueId === null}
          onClick={() => toggleArmedCue(props.selectedCueId)}
        >
          {armedCueId() === props.selectedCueId ? "Disarm Cue" : "Arm Cue"}
        </button>
        <button
          type="button"
          data-open-super-scene={props.selectedCueId ?? undefined}
          disabled={props.selectedCueId === null || props.childTimelineLabel !== null}
          onClick={() => {
            if (props.selectedCueId !== null) void props.onOpenOrCreateSuperScene(props.selectedCueId);
          }}
        >
          {props.selectedCueIsSuperScene ? "Edit Super Scene" : "Create Super Scene"}
        </button>
        <Show when={armedCue()}>
          {(cue) => <output class="timelineArmedCue" data-timeline-armed-cue={cue().id}>Armed: {cue().label}</output>}
        </Show>
      </div>
      <TimelineOverview
        events={props.overviewEvents}
        cueColors={props.cueColors}
        audioClips={props.audioClips}
        audioAnalysis={props.audioAnalysis}
        layers={props.timelineLayers}
        legacyMode={props.legacyTimelineLayers}
        cueDrag={props.timelineCueDrag}
        executionLive={props.executingLive}
        markerAriaLabel={props.overviewMarkerAriaLabel}
        automationRanges={props.overviewAutomationRanges}
        overlapClusters={props.overviewOverlapClusters}
        selectedRangeId={props.selectedAutomationRangeId}
        selectedEventId={props.selectedEventId}
        selectedAudioClipId={selectedAudioClipId()}
        playheadX={props.overviewPlayheadX}
        visibleWindow={props.visibleWindow}
        bpm={props.bpm}
        stretchMode={stretchMode()}
        magnetEnabled={magnetEnabled()}
        armedCue={armedCue()}
        snapTimeMs={props.snapTimeMs}
        onSeekTime={props.onSeekOverviewTime}
        onSelectAutomationRange={props.onSelectAutomationRange}
        onSelectEvent={(eventId) => props.onSelectEvent(eventId, false)}
        onOpenSuperScene={props.onOpenSuperScene}
        onSelectAudioClip={setSelectedAudioClipId}
        onInspectOverlapCluster={inspectOverlapCluster}
        onUpdateLayer={(layer) => void props.onUpdateTimelineLayer(layer)}
        onAddAudioClip={(layerId) => void props.onAddAudioClip(layerId)}
        onUpdateAudioClip={(clip) => void props.onUpdateAudioClip(clip)}
        onStatus={props.onTimelineStatus}
        onMoveEventPlacement={(eventId, timeMs, layerId, snapEnabled) =>
          void props.onMoveEventPlacement(eventId, timeMs, layerId, snapEnabled)
        }
        onResizeEventTime={(eventId, edge, timeMs, mode, snapEnabled) =>
          void props.onResizeEventTime(eventId, edge, timeMs, mode, snapEnabled)
        }
        onSetEventFade={(eventId, edge, fadeMs, snapEnabled) =>
          void props.onSetEventFade(eventId, edge, fadeMs, snapEnabled)
        }
        onSetVisibleWindow={props.onSetVisibleWindow}
        onZoomAt={props.onZoomOverviewAt}
        onPlaceArmedCue={(cueId, timeMs, layerId, durationMs, mode, snapEnabled) =>
          void props.onPlaceArmedCue(cueId, timeMs, layerId, durationMs, mode, snapEnabled)
        }
        onMoveAutomationRangeTime={(range, timeMs) => void props.onMoveAutomationRangeTime(range, timeMs)}
        onResizeAutomationRangeTime={(range, edge, timeMs) => void props.onResizeAutomationRangeTime(range, edge, timeMs)}
        onMoveAutomationKeyframeTime={(range, keyframeIndex, timeMs) =>
          void props.onMoveAutomationKeyframeTime(range, keyframeIndex, timeMs)
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
        <div class="timelineAudioMasterControls">
          <button
            type="button"
            data-timeline-audio-master-mute
            aria-pressed={props.audioMuted}
            onClick={() => void props.onSetAudioMaster(props.audioOffsetMs, !props.audioMuted)}
          >
            {props.audioMuted ? "Unmute Timeline Audio" : "Mute Timeline Audio"}
          </button>
          <label>
            Master Offset (ms)
            <input
              class="tabularNums"
              type="number"
              step="1"
              value={props.audioOffsetMs}
              onChange={(event) => void props.onSetAudioMaster(
                Math.round(Number(event.currentTarget.value) || 0),
                props.audioMuted,
              )}
            />
          </label>
          <span class="tabularNums">{`${props.audioClips.length} audio clips`}</span>
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
        <Show when={selectedAudioClip()}>
          {(clip) => {
            const update = (patch: Partial<TimelineAudioClipSummary>) =>
              void props.onUpdateAudioClip({ ...clip(), ...patch });
            return (
              <div class="timelineAudioClipProperties" data-timeline-audio-clip-properties={clip().id}>
                <strong data-no-localize>{clip().path.replaceAll("\\", "/").split("/").pop()}</strong>
                <label>
                  Start (ms)
                  <input class="tabularNums" type="number" min="0" step="1" value={clip().start_ms}
                    onChange={(event) => update({ start_ms: Math.max(0, Math.round(Number(event.currentTarget.value) || 0)) })} />
                </label>
                <label>
                  Source Offset (ms)
                  <input class="tabularNums" type="number" min="0" step="1" value={clip().offset_ms}
                    onChange={(event) => update({ offset_ms: Math.max(0, Math.round(Number(event.currentTarget.value) || 0)) })} />
                </label>
                <label>
                  Duration (ms)
                  <input class="tabularNums" type="number" min="1" step="1" value={clip().duration_ms}
                    onChange={(event) => update({ duration_ms: Math.max(1, Math.round(Number(event.currentTarget.value) || 1)) })} />
                </label>
                <label>
                  Gain
                  <input class="tabularNums" type="number" min="0" max="2" step="0.01" value={clip().gain}
                    onChange={(event) => update({ gain: Math.min(2, Math.max(0, Number(event.currentTarget.value) || 0)) })} />
                </label>
                <label>
                  Fade In (ms)
                  <input class="tabularNums" type="number" min="0" step="1" value={clip().fade_in_ms}
                    onChange={(event) => update({ fade_in_ms: Math.max(0, Math.round(Number(event.currentTarget.value) || 0)) })} />
                </label>
                <label>
                  Fade Out (ms)
                  <input class="tabularNums" type="number" min="0" step="1" value={clip().fade_out_ms}
                    onChange={(event) => update({ fade_out_ms: Math.max(0, Math.round(Number(event.currentTarget.value) || 0)) })} />
                </label>
                <button
                  type="button"
                  class="danger"
                  onClick={() => {
                    setPendingRemoveAudioClipId(clip().id);
                    if (!audioClipRemoveDialog?.open) audioClipRemoveDialog?.showModal();
                  }}
                >
                  Remove Audio Clip
                </button>
              </div>
            );
          }}
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
        bpm={props.bpm}
        cueColors={props.cueColors}
        executionLive={props.executingLive}
        selectedCueId={props.selectedCueId}
        startMs={props.eventTimeMs}
        durationMs={props.blockDurationMs}
        loopCount={props.blockLoopCount}
        track={props.track}
        jumpToEventId={props.blockJumpToEventId}
        cueOptions={props.cueOptions}
        eventRows={props.eventRows}
        filterEventIds={overlapFilterEventIds()}
        filterLabel={overlapFilterLabel()}
        selectedEventId={props.selectedEventId}
        selectionRevision={props.selectionRevision}
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
        onSelectEvent={(eventId) => props.onSelectEvent(eventId, true)}
        onClearEventFilter={clearOverlapFilter}
        onOpenSourceCue={props.onOpenSourceCue}
        armedCueId={armedCueId()}
        onArmCue={toggleArmedCue}
      />
      <dialog
        ref={(element) => { audioClipRemoveDialog = element; }}
        class="timelineLayerRemoveDialog timelineAudioClipRemoveDialog"
        data-timeline-audio-clip-remove-dialog
        role="alertdialog"
        aria-labelledby="timeline-audio-clip-remove-title"
        aria-describedby="timeline-audio-clip-remove-description"
        onClose={() => setPendingRemoveAudioClipId(null)}
      >
        <form method="dialog">
          <h2 id="timeline-audio-clip-remove-title" class="textBalance">Remove Audio Clip?</h2>
          <div id="timeline-audio-clip-remove-description" class="textPretty">
            <p>Remove this clip from the timeline?</p>
            <strong data-no-localize>
              {pendingRemoveAudioClip()?.path.replaceAll("\\", "/").split("/").pop() ?? ""}
            </strong>
          </div>
          <div class="buttonRow">
            <button value="cancel">Cancel</button>
            <button
              type="button"
              class="danger"
              data-timeline-audio-clip-remove-confirm
              disabled={!pendingRemoveAudioClip()}
              onClick={() => {
                const clipId = pendingRemoveAudioClipId();
                if (clipId === null) return;
                audioClipRemoveDialog?.close();
                void props.onRemoveAudioClip(clipId);
                setSelectedAudioClipId(null);
              }}
            >
              Remove Audio Clip
            </button>
          </div>
        </form>
      </dialog>
    </>
  );
}
