import { createEffect, createMemo, createSignal, createUniqueId, For, onCleanup, onMount, Show } from "solid-js";
import type { TimelineEventDraft } from "../editorDrafts";
import type {
  AudioAnalysisSummary,
  TimelineAudioClipSummary,
  TimelineCueEventSummary,
  TimelineLayerKind,
  TimelineLayerSummary,
  TimelineTrackKind,
} from "../types";
import type { CueIdentitySource } from "../identityColor";
import type { TimelineCueDragState } from "../timelineCueDrag";
import type { TimelineContextDrawer } from "../uiModes";
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
import {
  type TimelineStretchMode,
} from "../timelineBlockGestures";

export type TimelineSnapMode = "Off" | "Beat" | "Bar" | "Grid";

const timelineLayerKinds: TimelineLayerKind[] = ["Audio", "Lighting", "Video"];

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
  contextDrawer: TimelineContextDrawer;
  childTimelineLabel: string | null;
  cueIdentities?: Record<number, CueIdentitySource>;
  positionMs: number;
  bpm: number;
  durationMs: number;
  playing: boolean;
  metronomeEnabled: boolean;
  countInBeats: number;
  countInRemainingMs: number;
  executingLive: boolean;
  cuesCount: number;
  superSceneCueCount: number;
  lightingAutomationCount: number;
  videoAutomationCount: number;
  overviewEvents: TimelineOverviewEvent[];
  timelineLayers: TimelineLayerSummary[];
  timelineCueDrag: TimelineCueDragState | null;
  overviewMarkerAriaLabel: (event: TimelineOverviewEvent) => string;
  overviewAutomationRanges: TimelineOverviewAutomationRange[];
  overviewOverlapClusters: TimelineOverviewOverlapCluster[];
  overviewOverlapLayerIds: number[];
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
  stretchMode: TimelineStretchMode;
  magnetEnabled: boolean;
  armedCueId: number | null;
  armedCue: {
    id: number;
    label: string;
    authored_beats: number | null;
    natural_duration_ms: number;
  } | null;
  timelineEventDraft: (event: TimelineCueEventSummary) => TimelineEventDraft;
  onSeek: (timeMs: number) => void | Promise<void>;
  onExitChildTimeline: () => void;
  onOpenSuperScene: (cueId: number) => void;
  onPause: () => void | Promise<void>;
  onPlay: () => void | Promise<void>;
  onSetMetronome: (enabled: boolean, countInBeats: number) => void | Promise<void>;
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
  onStretchMode: (mode: TimelineStretchMode) => void;
  onMagnetEnabled: (enabled: boolean) => void;
  onArmCue: (cueId: number | null) => void;
  onContextDrawer: (drawer: TimelineContextDrawer) => void;
}

export function TimelineCueEventsPanel(props: TimelineCueEventsPanelProps) {
  let audioClipRemoveDialog: HTMLDialogElement | undefined;
  let layerAddDialog: HTMLDialogElement | undefined;
  let layerRemoveDialog: HTMLDialogElement | undefined;
  let layerMenuElement: HTMLDivElement | undefined;
  let layerMenuReturnFocus: HTMLElement | null = null;
  let layerDialogReturnFocus: HTMLElement | null = null;
  const [selectedAudioClipId, setSelectedAudioClipId] = createSignal<number | null>(null);
  const [pendingRemoveAudioClipId, setPendingRemoveAudioClipId] = createSignal<number | null>(null);
  const [layerMenu, setLayerMenu] = createSignal<{ layerId: number; x: number; y: number } | null>(null);
  const [layerRename, setLayerRename] = createSignal("");
  const [newLayerKind, setNewLayerKind] = createSignal<TimelineLayerKind>("Lighting");
  const [newLayerLabel, setNewLayerLabel] = createSignal("");
  const [pendingRemoveLayerId, setPendingRemoveLayerId] = createSignal<number | null>(null);
  const [reassignTargetLayerId, setReassignTargetLayerId] = createSignal<number | null>(null);
  const [blockDrawerBrowserMode, setBlockDrawerBrowserMode] = createSignal(false);
  const layerAddTitleId = `${createUniqueId()}-timeline-layer-add-title`;
  const layerRemoveTitleId = `${createUniqueId()}-timeline-layer-remove-title`;
  const layerRemoveDescriptionId = `${createUniqueId()}-timeline-layer-remove-description`;
  const selectedAudioClip = () => props.audioClips.find((clip) => clip.id === selectedAudioClipId()) ?? null;
  const pendingRemoveAudioClip = () =>
    props.audioClips.find((clip) => clip.id === pendingRemoveAudioClipId()) ?? null;
  createEffect(() => {
    if (selectedAudioClipId() !== null && !selectedAudioClip()) setSelectedAudioClipId(null);
  });
  createEffect(() => {
    if (props.contextDrawer !== "block" && blockDrawerBrowserMode()) {
      setBlockDrawerBrowserMode(false);
    }
  });
  const menuLayer = () => {
    const menu = layerMenu();
    return menu ? props.timelineLayers.find((layer) => layer.id === menu.layerId) ?? null : null;
  };
  const orderedTimelineLayers = () => [...props.timelineLayers].sort(
    (left, right) => left.order - right.order || left.id - right.id,
  );
  const timelineLayerSequence = (layerId: number) =>
    orderedTimelineLayers().findIndex((layer) => layer.id === layerId) + 1;
  const timelineEventCountForLayer = (layerId: number) => props.eventRows.filter((event) =>
    timelineLayerIdForEvent(props.timelineLayers, event) === layerId).length;
  const timelineAudioClipCountForLayer = (layerId: number) => props.audioClips.filter((clip) =>
    clip.layer_id === layerId).length;
  const timelineLayerItemCounts = createMemo(() => {
    const counts = new Map<number, number>();
    for (const event of props.eventRows) {
      const layerId = timelineLayerIdForEvent(props.timelineLayers, event);
      counts.set(layerId, (counts.get(layerId) ?? 0) + 1);
    }
    for (const clip of props.audioClips) {
      counts.set(clip.layer_id, (counts.get(clip.layer_id) ?? 0) + 1);
    }
    return counts;
  });
  const timelineSuperSceneEmptyHintCount = createMemo(() =>
    props.childTimelineLabel === null
    && props.eventRows.length === 0
    && props.lightingAutomationCount === 0
    && props.videoAutomationCount === 0
    && props.audioClips.length === 0
      ? props.superSceneCueCount
      : 0);
  const pendingRemoveLayer = () => {
    const layerId = pendingRemoveLayerId();
    return layerId === null
      ? null
      : props.timelineLayers.find((layer) => layer.id === layerId) ?? null;
  };
  const pendingRemoveEventCount = () => {
    const layer = pendingRemoveLayer();
    return layer ? timelineEventCountForLayer(layer.id) : 0;
  };
  const pendingRemoveAudioClipCount = () => {
    const layer = pendingRemoveLayer();
    return layer ? timelineAudioClipCountForLayer(layer.id) : 0;
  };
  const reassignCandidatesFor = (layer: TimelineLayerSummary) => {
    const eventCount = timelineEventCountForLayer(layer.id);
    const audioClipCount = timelineAudioClipCountForLayer(layer.id);
    return orderedTimelineLayers().filter((candidate) =>
      candidate.id !== layer.id
      && !candidate.locked
      && (eventCount === 0 || candidate.kind !== "Audio")
      && (audioClipCount === 0 || candidate.kind === "Audio"));
  };
  const reassignCandidates = () => {
    const layer = pendingRemoveLayer();
    return layer ? reassignCandidatesFor(layer) : [];
  };
  const canConfirmLayerRemoval = () => {
    const layer = pendingRemoveLayer();
    if (!layer || layer.locked || props.timelineLayers.length <= 1) return false;
    const hasContents = pendingRemoveEventCount() > 0 || pendingRemoveAudioClipCount() > 0;
    return !hasContents || reassignCandidates().some((candidate) => candidate.id === reassignTargetLayerId());
  };
  const openLayerMenu = (layer: TimelineLayerSummary, point: { x: number; y: number }) => {
    const activeElement = document.activeElement;
    const fallbackTrigger = document.querySelector<HTMLElement>(
      `[data-timeline-layer-gutter][data-timeline-layer-id="${layer.id}"] [data-timeline-layer-menu-trigger]`,
    );
    layerMenuReturnFocus = activeElement instanceof HTMLElement &&
      activeElement.matches("[data-timeline-layer-menu-trigger]")
      ? activeElement
      : fallbackTrigger;
    setLayerRename(layer.label);
    setLayerMenu({
      layerId: layer.id,
      x: Math.max(8, Math.min(point.x, window.innerWidth - 276)),
      y: Math.max(8, Math.min(point.y, window.innerHeight - 286)),
    });
    queueMicrotask(() => {
      layerMenuElement
        ?.querySelector<HTMLElement>('input, button:not(:disabled), [tabindex="0"]')
        ?.focus();
    });
  };
  const closeLayerMenu = (restoreFocus = true) => {
    const returnFocus = restoreFocus ? layerMenuReturnFocus : null;
    layerMenuReturnFocus = null;
    setLayerMenu(null);
    if (returnFocus) queueMicrotask(() => {
      if (returnFocus.isConnected) returnFocus.focus();
    });
  };
  const openLayerAddDialog = (initialKind: TimelineLayerKind) => {
    setNewLayerKind(initialKind);
    setNewLayerLabel("");
    layerDialogReturnFocus = layerMenuReturnFocus;
    closeLayerMenu(false);
    if (!layerAddDialog?.open) layerAddDialog?.showModal();
  };
  const suggestedLayerLabel = () => {
    const kind = newLayerKind();
    const nextNumber = props.timelineLayers.filter((layer) => layer.kind === kind).length + 1;
    return `${kind} ${nextNumber}`;
  };
  const addTimelineLayer = () => {
    const label = newLayerLabel().trim() || suggestedLayerLabel();
    layerAddDialog?.close();
    void props.onAddTimelineLayer(label, newLayerKind());
  };
  const openLayerRemoveDialog = (layer: TimelineLayerSummary) => {
    if (layer.locked || props.timelineLayers.length <= 1) return;
    const candidates = reassignCandidatesFor(layer);
    setPendingRemoveLayerId(layer.id);
    setReassignTargetLayerId(candidates[0]?.id ?? null);
    layerDialogReturnFocus = layerMenuReturnFocus;
    closeLayerMenu(false);
    if (!layerRemoveDialog?.open) layerRemoveDialog?.showModal();
  };
  const restoreLayerDialogFocus = () => {
    const returnFocus = layerDialogReturnFocus;
    layerDialogReturnFocus = null;
    if (returnFocus) queueMicrotask(() => {
      if (returnFocus.isConnected) returnFocus.focus();
    });
  };
  const confirmLayerRemoval = () => {
    const layer = pendingRemoveLayer();
    if (!layer || !canConfirmLayerRemoval()) return;
    const hasContents = pendingRemoveEventCount() > 0 || pendingRemoveAudioClipCount() > 0;
    const reassignToLayerId = hasContents ? reassignTargetLayerId() : null;
    layerRemoveDialog?.close();
    void props.onRemoveTimelineLayer(layer.id, reassignToLayerId);
  };
  createEffect(() => {
    const layer = pendingRemoveLayer();
    if (pendingRemoveLayerId() !== null && !layer) {
      if (layerRemoveDialog?.open) layerRemoveDialog.close();
      setPendingRemoveLayerId(null);
      setReassignTargetLayerId(null);
      return;
    }
    if (!layer || (pendingRemoveEventCount() === 0 && pendingRemoveAudioClipCount() === 0)) {
      if (reassignTargetLayerId() !== null) setReassignTargetLayerId(null);
      return;
    }
    const candidates = reassignCandidates();
    if (!candidates.some((candidate) => candidate.id === reassignTargetLayerId())) {
      setReassignTargetLayerId(candidates[0]?.id ?? null);
    }
  });
  onMount(() => {
    const closeFromPointer = (event: PointerEvent) => {
      if (!layerMenu()) return;
      const target = event.target;
      if (target instanceof Element && target.closest(".timelineLayerContextMenu")) return;
      closeLayerMenu(false);
    };
    const closeFromEscape = (event: KeyboardEvent) => {
      if (event.key !== "Escape" || event.defaultPrevented || !layerMenu()) return;
      if (document.querySelector('.timelineOverview[data-timeline-gesture-active="true"]')) return;
      event.preventDefault();
      event.stopImmediatePropagation();
      closeLayerMenu();
    };
    window.addEventListener("pointerdown", closeFromPointer, true);
    window.addEventListener("keydown", closeFromEscape, { capture: true });
    onCleanup(() => {
      window.removeEventListener("pointerdown", closeFromPointer, true);
      window.removeEventListener("keydown", closeFromEscape, { capture: true });
    });
  });
  const armedCue = () => props.armedCue;
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
    props.onContextDrawer("block");
    setBlockDrawerBrowserMode(true);
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
      </div>
      <div class="timelineToolStrip" role="toolbar" aria-label="Timeline tools">
      <div class="timelineTransport" role="group" aria-label="Timeline transport">
        <button
          type="button"
          title="Go to timeline start"
          aria-label="Go to timeline start"
          onClick={() => void props.onSeek(0)}
        >
          <span class="timelineToolIcon" aria-hidden="true" data-no-localize>|◀</span>
        </button>
        <button
          type="button"
          title="Pause timeline"
          aria-label="Pause timeline"
          onClick={() => void props.onPause()}
          disabled={!props.playing}
        >
          <span class="timelineToolIcon" aria-hidden="true" data-no-localize>Ⅱ</span>
        </button>
        <button
          type="button"
          class="primary"
          title="Play timeline"
          aria-label="Play timeline"
          onClick={() => void props.onPlay()}
          disabled={props.durationMs === 0 || props.playing}
        >
          <span class="timelineToolIcon" aria-hidden="true" data-no-localize>▶</span>
        </button>
      </div>
      <div class="timelineMetronomeControls" role="group" aria-label="Timeline click controls">
        <button
          type="button"
          classList={{ active: props.metronomeEnabled }}
          aria-pressed={props.metronomeEnabled}
          data-timeline-metronome
          onClick={() => void props.onSetMetronome(!props.metronomeEnabled, props.countInBeats || 4)}
        >
          クリック
        </button>
        <button
          type="button"
          classList={{ active: props.countInBeats > 0 }}
          aria-pressed={props.countInBeats > 0}
          data-timeline-count-in
          title="One bar count-in (4 beats)"
          onClick={() => void props.onSetMetronome(props.metronomeEnabled, props.countInBeats > 0 ? 0 : 4)}
        >
          1小節
        </button>
        <Show when={props.countInRemainingMs > 0}>
          <output class="timelineCountInReadout tabularNums" aria-label="Count-in remaining">
            {`${Math.ceil(props.countInRemainingMs / Math.max(1, 60_000 / props.bpm))}`}
          </output>
        </Show>
      </div>
      <input
        class="timelineScrubber"
        type="range"
        aria-label="Timeline position"
        min="0"
        max={Math.max(props.durationMs, 1)}
        value={props.positionMs}
        onInput={(event) => void props.onSeek(Number(event.currentTarget.value))}
      />
      <nav class="timelineViewportToolbar" aria-label="Timeline visible range controls">
        <button
          type="button"
          title="Pan Prev"
          aria-label="Pan Prev"
          data-timeline-tool="pan-prev"
          onClick={() => props.onPanOverview(-1)}
          disabled={props.visibleWindow.start_ms <= 0}
        >
          <span class="timelineToolIcon" aria-hidden="true" data-no-localize>←</span>
        </button>
        <button
          type="button"
          title="Zoom Out"
          aria-label="Zoom Out"
          data-timeline-tool="zoom-out"
          onClick={() => props.onZoomOverview(2)}
          disabled={timelineVisibleWindowSpanMs(props.visibleWindow) >= props.overviewEditExtentMs}
        >
          <span class="timelineToolIcon" aria-hidden="true" data-no-localize>−</span>
        </button>
        <button type="button" title="Fit All" aria-label="Fit All" data-timeline-tool="fit-all" onClick={props.onFitOverview}>
          <span class="timelineToolIcon" aria-hidden="true" data-no-localize>↔</span>
        </button>
        <button
          type="button"
          title="Zoom In"
          aria-label="Zoom In"
          data-timeline-tool="zoom-in"
          onClick={() => props.onZoomOverview(0.5)}
          disabled={timelineVisibleWindowSpanMs(props.visibleWindow) <= TIMELINE_MIN_VISIBLE_WINDOW_MS}
        >
          <span class="timelineToolIcon" aria-hidden="true" data-no-localize>+</span>
        </button>
        <button
          type="button"
          title="Pan Next"
          aria-label="Pan Next"
          data-timeline-tool="pan-next"
          onClick={() => props.onPanOverview(1)}
          disabled={props.visibleWindow.end_ms >= props.overviewEditExtentMs}
        >
          <span class="timelineToolIcon" aria-hidden="true" data-no-localize>→</span>
        </button>
        <button
          type="button"
          title="Reveal Selected"
          aria-label="Reveal Selected"
          data-timeline-tool="reveal-selected"
          onClick={props.onRevealSelected}
          disabled={props.selectedEventId === null}
        >
          <span class="timelineToolIcon" aria-hidden="true" data-no-localize>◎</span>
        </button>
        <button type="button" title="Reveal Playhead" aria-label="Reveal Playhead" data-timeline-tool="reveal-playhead" onClick={props.onRevealPlayhead}>
          <span class="timelineToolIcon" aria-hidden="true" data-no-localize>⌖</span>
        </button>
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
        <div class="timelineStretchModeToggle" role="group" aria-label="Stretch mode" data-timeline-stretch-mode-toggle>
          <button
            type="button"
            title="Stretch by rate"
            aria-label="Stretch by rate"
            classList={{ active: props.stretchMode === "RATE" }}
            aria-pressed={props.stretchMode === "RATE"}
            data-timeline-stretch-mode="RATE"
            data-timeline-tool="stretch-rate"
            onClick={() => props.onStretchMode("RATE")}
          >
            <span class="timelineToolIcon" aria-hidden="true" data-no-localize>↯</span>
          </button>
          <button
            type="button"
            title="Stretch block window"
            aria-label="Stretch block window"
            classList={{ active: props.stretchMode === "WINDOW" }}
            aria-pressed={props.stretchMode === "WINDOW"}
            data-timeline-stretch-mode="WINDOW"
            data-timeline-tool="stretch-window"
            onClick={() => props.onStretchMode("WINDOW")}
          >
            <span class="timelineToolIcon" aria-hidden="true" data-no-localize>↔</span>
          </button>
        </div>
        <button
          type="button"
          classList={{ active: props.magnetEnabled }}
          aria-pressed={props.magnetEnabled}
          aria-label={props.magnetEnabled ? "Disable magnet snap" : "Enable magnet snap"}
          title={props.magnetEnabled ? "Disable magnet snap" : "Enable magnet snap"}
          data-timeline-magnet-toggle
          data-timeline-tool="magnet"
          onClick={() => props.onMagnetEnabled(!props.magnetEnabled)}
        >
          <span class="timelineToolIcon" aria-hidden="true" data-no-localize>∩</span>
        </button>
        <button
          type="button"
          classList={{ active: props.armedCueId === props.selectedCueId }}
          aria-pressed={props.selectedCueId !== null && props.armedCueId === props.selectedCueId}
          aria-label={props.armedCueId === props.selectedCueId ? "Disarm Cue" : "Arm Cue"}
          title={props.armedCueId === props.selectedCueId ? "Disarm Cue" : "Arm Cue"}
          data-timeline-arm-cue={props.selectedCueId ?? undefined}
          data-timeline-tool="arm-cue"
          disabled={props.selectedCueId === null}
          onClick={() => props.onArmCue(props.selectedCueId)}
        >
          <span class="timelineToolIcon" aria-hidden="true" data-no-localize>{props.armedCueId === props.selectedCueId ? "●" : "○"}</span>
        </button>
        <button
          type="button"
          title={props.selectedCueIsSuperScene ? "Edit Timeline" : "Create Timeline"}
          aria-label={props.selectedCueIsSuperScene ? "Edit Timeline" : "Create Timeline"}
          data-open-super-scene={props.selectedCueId ?? undefined}
          data-timeline-tool="super-scene"
          disabled={props.selectedCueId === null || props.childTimelineLabel !== null}
          onClick={() => {
            if (props.selectedCueId !== null) void props.onOpenOrCreateSuperScene(props.selectedCueId);
          }}
        >
          <span class="timelineToolIcon" aria-hidden="true" data-no-localize>◇</span>
        </button>
        <button
          type="button"
          title="Block Properties"
          aria-label="Block Properties"
          classList={{ active: props.contextDrawer === "block" }}
          aria-pressed={props.contextDrawer === "block"}
          data-timeline-block-properties-toggle
          data-timeline-tool="block-properties"
          onClick={() => {
            if (props.contextDrawer === "block") {
              props.onContextDrawer("none");
              return;
            }
            props.onContextDrawer("block");
            setBlockDrawerBrowserMode(props.selectedEventId === null);
          }}
        >
          <span class="timelineToolIcon" aria-hidden="true" data-no-localize>▤</span>
        </button>
        <Show when={armedCue()}>
          {(cue) => <output class="timelineArmedCue" data-timeline-armed-cue={cue().id}>Armed: {cue().label}</output>}
        </Show>
      </div>
      </div>
      <TimelineOverview
        events={props.overviewEvents}
        layerItemCounts={timelineLayerItemCounts()}
        cueIdentities={props.cueIdentities}
        audioClips={props.audioClips}
        audioAnalysis={props.audioAnalysis}
        layers={props.timelineLayers}
        legacyMode={false}
        cueDrag={props.timelineCueDrag}
        executionLive={props.executingLive}
        markerAriaLabel={props.overviewMarkerAriaLabel}
        automationRanges={props.overviewAutomationRanges}
        superSceneEmptyHintCount={timelineSuperSceneEmptyHintCount()}
        overlapClusters={props.overviewOverlapClusters}
        overlapLayerIds={props.overviewOverlapLayerIds}
        selectedRangeId={props.selectedAutomationRangeId}
        selectedEventId={props.selectedEventId}
        selectedAudioClipId={selectedAudioClipId()}
        playheadX={props.overviewPlayheadX}
        visibleWindow={props.visibleWindow}
        bpm={props.bpm}
        stretchMode={props.stretchMode}
        magnetEnabled={props.magnetEnabled}
        armedCue={armedCue()}
        snapTimeMs={props.snapTimeMs}
        onSeekTime={props.onSeekOverviewTime}
        onSelectAutomationRange={props.onSelectAutomationRange}
        onSelectEvent={(eventId, openProperties = false) => {
          props.onSelectEvent(eventId, false);
          if (openProperties) {
            props.onContextDrawer("block");
            setBlockDrawerBrowserMode(false);
          }
        }}
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
        onOpenLayerMenu={openLayerMenu}
      />
      <Show when={menuLayer()}>
        {(layer) => (
          <div
            class="timelineLayerContextMenu"
            ref={(element) => { layerMenuElement = element; }}
            data-timeline-layer-context-menu={layer().id}
            role="dialog"
            aria-modal="false"
            aria-label={`Timeline layer actions for lane ${timelineLayerSequence(layer().id)}: ${layer().label}`}
            style={{ left: `${layerMenu()?.x ?? 8}px`, top: `${layerMenu()?.y ?? 8}px` }}
            onContextMenu={(event) => event.preventDefault()}
          >
            <header>
              <span
                class={`timelineLayerKindIcon ${layer().kind.toLowerCase()}`}
                aria-hidden="true"
                data-timeline-layer-kind-icon={layer().kind}
              />
              <strong class="tabularNums" data-no-localize>#{timelineLayerSequence(layer().id)}</strong>
            </header>
            <label>
              <span>Layer name</span>
              <input
                value={layerRename()}
                maxlength="64"
                aria-label="Layer name"
                onInput={(event) => setLayerRename(event.currentTarget.value)}
                onKeyDown={(event) => {
                  if (event.key !== "Enter") return;
                  event.preventDefault();
                  const label = layerRename().trim();
                  if (label) void props.onUpdateTimelineLayer({ ...layer(), label });
                  closeLayerMenu();
                }}
              />
            </label>
            <button
              type="button"
              disabled={!layerRename().trim() || layerRename().trim() === layer().label}
              onClick={() => {
                const label = layerRename().trim();
                if (label) void props.onUpdateTimelineLayer({ ...layer(), label });
                closeLayerMenu();
              }}
            >
              Rename Layer
            </button>
            <button
              type="button"
              aria-pressed={layer().solo}
              onClick={() => {
                void props.onUpdateTimelineLayer({ ...layer(), solo: !layer().solo });
                closeLayerMenu();
              }}
            >
              {layer().solo ? "Clear timeline layer solo" : "Solo timeline layer"}
            </button>
            <button
              type="button"
              onClick={() => openLayerAddDialog(layer().kind)}
            >
              Add Layer
            </button>
            <div class="timelineLayerContextMove" role="group" aria-label="Reorder timeline layer">
              <button
                type="button"
                title="Move selected layer up within its section"
                aria-label="Move selected layer up within its section"
                onClick={() => {
                  void props.onReorderTimelineLayer(layer().id, -1);
                  closeLayerMenu();
                }}
              >
                <span aria-hidden="true" data-no-localize>↑</span>
              </button>
              <button
                type="button"
                title="Move selected layer down within its section"
                aria-label="Move selected layer down within its section"
                onClick={() => {
                  void props.onReorderTimelineLayer(layer().id, 1);
                  closeLayerMenu();
                }}
              >
                <span aria-hidden="true" data-no-localize>↓</span>
              </button>
            </div>
            <button
              type="button"
              class="danger"
              disabled={layer().locked || props.timelineLayers.length <= 1}
              onClick={() => openLayerRemoveDialog(layer())}
            >
              Remove Layer
            </button>
          </div>
        )}
      </Show>
      <dialog
        ref={(element) => { layerAddDialog = element; }}
        class="timelineLayerRemoveDialog timelineLayerAddDialog"
        data-timeline-layer-add-dialog
        aria-labelledby={layerAddTitleId}
        onClose={() => {
          setNewLayerLabel("");
          restoreLayerDialogFocus();
        }}
      >
        <form method="dialog">
          <h2 id={layerAddTitleId} class="textBalance">Add Timeline Layer</h2>
          <label class="timelineLayerReassignField">
            <span>Layer kind</span>
            <select
              data-timeline-layer-add-kind
              value={newLayerKind()}
              onInput={(event) => setNewLayerKind(event.currentTarget.value as TimelineLayerKind)}
            >
              <For each={timelineLayerKinds}>{(kind) => <option value={kind}>{kind}</option>}</For>
            </select>
          </label>
          <label class="timelineLayerReassignField">
            <span>Name (optional)</span>
            <input
              data-timeline-layer-add-name
              maxlength="64"
              value={newLayerLabel()}
              placeholder="Optional layer name"
              onInput={(event) => setNewLayerLabel(event.currentTarget.value)}
              onKeyDown={(event) => {
                if (event.key !== "Enter") return;
                event.preventDefault();
                addTimelineLayer();
              }}
            />
          </label>
          <div class="buttonRow">
            <button value="cancel">Cancel</button>
            <button type="button" class="primary" data-timeline-layer-add onClick={addTimelineLayer}>
              Add Layer
            </button>
          </div>
        </form>
      </dialog>
      <dialog
        ref={(element) => { layerRemoveDialog = element; }}
        class="timelineLayerRemoveDialog"
        data-timeline-layer-remove-dialog
        role="alertdialog"
        aria-labelledby={layerRemoveTitleId}
        aria-describedby={layerRemoveDescriptionId}
        onClose={() => {
          setPendingRemoveLayerId(null);
          setReassignTargetLayerId(null);
          restoreLayerDialogFocus();
        }}
      >
        <form method="dialog">
          <h2 id={layerRemoveTitleId} class="textBalance">Remove Timeline Layer?</h2>
          <div id={layerRemoveDescriptionId} class="textPretty">
            <Show when={pendingRemoveLayer()}>
              {(layer) => (
                <>
                  <dl class="timelineLayerRemoveSummary">
                    <div>
                      <dt>Layer</dt>
                      <dd data-no-localize>{layer().label}</dd>
                    </div>
                    <div>
                      <dt>Timeline blocks</dt>
                      <dd class="tabularNums" data-no-localize>{pendingRemoveEventCount()}</dd>
                    </div>
                    <div>
                      <dt>Audio clips</dt>
                      <dd class="tabularNums" data-no-localize>{pendingRemoveAudioClipCount()}</dd>
                    </div>
                  </dl>
                  <Show
                    when={!layer().locked}
                    fallback={<p class="validationError">Unlock this layer before removing it.</p>}
                  >
                    <Show
                      when={pendingRemoveEventCount() > 0 || pendingRemoveAudioClipCount() > 0}
                      fallback={<p>This layer is empty and can be removed directly.</p>}
                    >
                      <Show
                        when={reassignCandidates().length > 0}
                        fallback={
                          <Show
                            when={pendingRemoveEventCount() > 0 && pendingRemoveAudioClipCount() > 0}
                            fallback={
                              <Show
                                when={pendingRemoveAudioClipCount() > 0}
                                fallback={
                                  <p class="validationError">
                                    Add another unlocked Lighting or Video layer, or remove this layer's blocks first.
                                  </p>
                                }
                              >
                                <p class="validationError">
                                  Add another unlocked Audio layer, or remove this layer's audio clips first.
                                </p>
                              </Show>
                            }
                          >
                            <p class="validationError">
                              This layer contains timeline blocks and audio clips. Move one content type before removing it.
                            </p>
                          </Show>
                        }
                      >
                        <p>
                          {pendingRemoveAudioClipCount() > 0
                            ? "This layer is not empty. Reassign its audio clips before removal."
                            : "This layer is not empty. Reassign its blocks before removal."}
                        </p>
                        <label class="timelineLayerReassignField">
                          <span>
                            {pendingRemoveAudioClipCount() > 0 ? "Reassign audio clips to" : "Reassign blocks to"}
                          </span>
                          <select
                            data-timeline-layer-reassign-target
                            value={reassignTargetLayerId() ?? ""}
                            onInput={(event) => setReassignTargetLayerId(Number(event.currentTarget.value))}
                          >
                            <For each={reassignCandidates()}>
                              {(candidate) => <option value={candidate.id} data-no-localize>{candidate.label}</option>}
                            </For>
                          </select>
                        </label>
                      </Show>
                    </Show>
                  </Show>
                </>
              )}
            </Show>
          </div>
          <div class="buttonRow">
            <button value="cancel">Cancel</button>
            <button
              type="button"
              class="danger"
              data-timeline-layer-remove-confirm
              disabled={!canConfirmLayerRemoval()}
              onClick={confirmLayerRemoval}
            >
              Remove Layer
            </button>
          </div>
        </form>
      </dialog>
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
      <Show when={props.contextDrawer === "block"}>
      <aside
        class="timelineContextDrawer timelineBlockPropertiesDrawer"
        classList={{ timelineBlockBrowserDrawer: blockDrawerBrowserMode() }}
        data-timeline-context-drawer-panel="block"
        aria-label="Block Properties drawer"
      >
        <header class="timelineContextDrawerHeader">
          <div>
            <strong>{blockDrawerBrowserMode() ? "Scene Blocks" : "Block Properties"}</strong>
            <Show when={props.selectedEventId !== null}>
              <span class="tabularNums" data-no-localize>#{props.selectedEventId}</span>
            </Show>
          </div>
          <button
            type="button"
            class="timelineContextDrawerMode"
            data-timeline-block-browser-toggle
            aria-pressed={blockDrawerBrowserMode()}
            title={blockDrawerBrowserMode() ? "Show selected Block Properties" : "Browse Scene Blocks"}
            aria-label={blockDrawerBrowserMode() ? "Show selected Block Properties" : "Browse Scene Blocks"}
            onClick={() => setBlockDrawerBrowserMode((current) => !current)}
          >
            <span aria-hidden="true" data-no-localize>{blockDrawerBrowserMode() ? "▤" : "⌗"}</span>
          </button>
          <button
            type="button"
            class="timelineContextDrawerClose"
            title="Close Block Properties"
            aria-label="Close Block Properties"
            onClick={() => props.onContextDrawer("none")}
          >
            <span aria-hidden="true" data-no-localize>×</span>
          </button>
        </header>
        <div class="timelineContextDrawerBody">
      <TimelineSceneBlocksEditor
        inspectorOnly={!blockDrawerBrowserMode()}
        positionMs={props.positionMs}
        bpm={props.bpm}
        cueIdentities={props.cueIdentities}
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
        armedCueId={props.armedCueId}
        onArmCue={props.onArmCue}
      />
        </div>
      </aside>
      </Show>
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
