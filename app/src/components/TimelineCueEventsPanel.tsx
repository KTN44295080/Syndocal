import { createEffect, createMemo, createSignal, createUniqueId, For, onCleanup, onMount, Show } from "solid-js";
import type { TimelineEventDraft } from "../editorDrafts";
import type {
  AudioAnalysisSummary,
  MediaAssetSummary,
  TimelineAudioClipSummary,
  TimelineCueEventSummary,
  TimelineLayerKind,
  TimelineLayerSummary,
  TimelineLoopRegionSummary,
  TimelineLoopRuntimeSummary,
  TimelineFollowRuntimeSummary,
  TimelineFollowSummary,
  TimelineItemGroupSummary,
  TimelineItemRef,
  TimelinePhaseRole,
  TimelinePhaseSummary,
  TimelineSnapshot,
  TimelineTrackKind,
  TimelineVideoClipSummary,
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
import { TimelineBankPanel } from "./TimelineBankPanel";
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
  embeddedControls: boolean;
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
  phases: TimelinePhaseSummary[];
  guideEnabled: boolean;
  loopRegion: TimelineLoopRegionSummary | null;
  loopRuntime: TimelineLoopRuntimeSummary;
  timelineBank: TimelineSnapshot[];
  activeTimelineId: number;
  followRuntime: TimelineFollowRuntimeSummary;
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
  videoClips: TimelineVideoClipSummary[];
  mediaAssets: MediaAssetSummary[];
  itemGroups: TimelineItemGroupSummary[];
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
  onSetGuideEnabled: (enabled: boolean) => void | Promise<void>;
  onSetPhases: (phases: TimelinePhaseSummary[]) => void | Promise<void>;
  onSetLoopRegion: (region: TimelineLoopRegionSummary | null) => void | Promise<void>;
  onSetLoopEnabled: (enabled: boolean) => void | Promise<void>;
  onScaleLoop: (scale: "half" | "double") => void | Promise<void>;
  onCreateTimeline: (label: string) => void | Promise<void>;
  onDuplicateTimeline: (timelineId: number) => void | Promise<void>;
  onRemoveTimeline: (timelineId: number) => void | Promise<void>;
  onReorderTimelines: (timelineIds: number[]) => void | Promise<void>;
  onSelectTimeline: (timelineId: number, play: boolean) => void | Promise<void>;
  onSetFollow: (follow: TimelineFollowSummary | null) => void | Promise<void>;
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
  onUpdateVideoClip: (clip: TimelineVideoClipSummary) => void | Promise<void>;
  onGroupItems: (items: TimelineItemRef[]) => void | Promise<void>;
  onUngroupItem: (item: TimelineItemRef) => void | Promise<void>;
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
  const [selectedVideoClipId, setSelectedVideoClipId] = createSignal<number | null>(null);
  const [selectedTimelineItems, setSelectedTimelineItems] = createSignal<TimelineItemRef[]>([]);
  const [itemContextMenu, setItemContextMenu] = createSignal<{ x: number; y: number } | null>(null);
  const [pendingRemoveAudioClipId, setPendingRemoveAudioClipId] = createSignal<number | null>(null);
  const [layerMenu, setLayerMenu] = createSignal<{ layerId: number; x: number; y: number } | null>(null);
  const [layerRename, setLayerRename] = createSignal("");
  const [newLayerKind, setNewLayerKind] = createSignal<TimelineLayerKind>("Lighting");
  const [newLayerLabel, setNewLayerLabel] = createSignal("");
  const [pendingRemoveLayerId, setPendingRemoveLayerId] = createSignal<number | null>(null);
  const [reassignTargetLayerId, setReassignTargetLayerId] = createSignal<number | null>(null);
  const [blockDrawerBrowserMode, setBlockDrawerBrowserMode] = createSignal(false);
  const loopRuntimeEnabled = () => props.loopRuntime.status !== "disabled";
  const defaultLoopLengthMs = () => Math.max(1, Math.round(240_000 / Math.max(20, props.bpm)));
  const setLoopAAtPlayhead = () => {
    const aMs = Math.max(0, Math.round(props.positionMs));
    const existingB = props.loopRegion?.b_ms ?? 0;
    const maximumEnd = Math.max(aMs + 1, props.durationMs || aMs + defaultLoopLengthMs());
    const bMs = existingB > aMs
      ? existingB
      : Math.min(maximumEnd, aMs + defaultLoopLengthMs());
    void props.onSetLoopRegion({
      a_ms: aMs,
      b_ms: bMs,
      enabled: props.loopRegion?.enabled ?? true,
      musical_length_beats: 4,
    });
  };
  const setLoopBAtPlayhead = () => {
    const bMs = Math.max(1, Math.round(props.positionMs));
    const aMs = props.loopRegion?.a_ms ?? Math.max(0, bMs - defaultLoopLengthMs());
    if (bMs <= aMs) {
      props.onTimelineStatus("Loop B must be after Loop A.");
      return;
    }
    void props.onSetLoopRegion({
      a_ms: aMs,
      b_ms: bMs,
      enabled: props.loopRegion?.enabled ?? true,
      musical_length_beats: props.loopRegion?.musical_length_beats ?? 4,
    });
  };
  const updatePhase = (phaseId: number, patch: Partial<TimelinePhaseSummary>) => {
    void props.onSetPhases(props.phases.map((phase) => phase.id === phaseId
      ? { ...phase, ...patch }
      : phase));
  };
  const addPhase = () => {
    const startMs = Math.max(0, Math.round(props.positionMs));
    const nextStart = props.phases
      .filter((phase) => phase.start_ms > startMs)
      .map((phase) => phase.start_ms)
      .sort((left, right) => left - right)[0];
    const endMs = Math.max(
      startMs + 1,
      Math.min(props.durationMs || startMs + defaultLoopLengthMs() * 4, nextStart ?? startMs + defaultLoopLengthMs() * 4),
    );
    void props.onSetPhases([
      ...props.phases,
      {
        id: 0,
        label: `Phase ${props.phases.length + 1}`,
        role: "custom" as TimelinePhaseRole,
        start_ms: startMs,
        end_ms: endMs,
      },
    ].sort((left, right) => left.start_ms - right.start_ms));
  };
  const layerAddTitleId = `${createUniqueId()}-timeline-layer-add-title`;
  const layerRemoveTitleId = `${createUniqueId()}-timeline-layer-remove-title`;
  const layerRemoveDescriptionId = `${createUniqueId()}-timeline-layer-remove-description`;
  const selectedAudioClip = () => props.audioClips.find((clip) => clip.id === selectedAudioClipId()) ?? null;
  const selectedVideoClip = () => props.videoClips.find((clip) => clip.id === selectedVideoClipId()) ?? null;
  const timelineItemKey = (item: TimelineItemRef) => {
    switch (item.kind) {
      case "video_clip": return `video:${item.clip_id}`;
      case "audio_clip": return `audio:${item.clip_id}`;
      case "lighting_event": return `event:${item.event_id}`;
      case "lighting_automation": return `lighting-automation:${item.automation_id}`;
      case "video_automation": return `video-automation:${item.automation_id}`;
    }
  };
  const selectedTimelineItemRefs = createMemo<TimelineItemRef[]>(() => {
    const videoIds = new Set(props.videoClips.map((clip) => clip.id));
    const audioIds = new Set(props.audioClips.map((clip) => clip.id));
    return selectedTimelineItems().filter((item) => item.kind === "video_clip"
      ? videoIds.has(item.clip_id)
      : item.kind === "audio_clip" ? audioIds.has(item.clip_id) : true);
  });
  const selectedAudioClipIds = createMemo(() => selectedTimelineItemRefs()
    .filter((item): item is Extract<TimelineItemRef, { kind: "audio_clip" }> => item.kind === "audio_clip")
    .map((item) => item.clip_id));
  const selectedVideoClipIds = createMemo(() => selectedTimelineItemRefs()
    .filter((item): item is Extract<TimelineItemRef, { kind: "video_clip" }> => item.kind === "video_clip")
    .map((item) => item.clip_id));
  const itemGroupForSelection = createMemo(() => props.itemGroups.find((group) =>
    selectedTimelineItemRefs().some((selected) => group.members.some((member) =>
      JSON.stringify(member) === JSON.stringify(selected)))) ?? null);
  const expandLinkedSelection = (item: TimelineItemRef, additive = false) => {
    const group = props.itemGroups.find((candidate) => candidate.members.some((member) =>
      JSON.stringify(member) === JSON.stringify(item)));
    const members = group?.members ?? [item];
    const next = additive ? [...selectedTimelineItems()] : [];
    const keys = new Set(next.map(timelineItemKey));
    for (const member of members) {
      if (!keys.has(timelineItemKey(member))) next.push(member);
    }
    setSelectedTimelineItems(next);
    if (!additive) {
      setSelectedAudioClipId(null);
      setSelectedVideoClipId(null);
    }
    if (item.kind === "audio_clip") setSelectedAudioClipId(item.clip_id);
    if (item.kind === "video_clip") setSelectedVideoClipId(item.clip_id);
  };
  const selectAudioClip = (clipId: number, additive = false) =>
    expandLinkedSelection({ kind: "audio_clip", clip_id: clipId }, additive);
  const selectVideoClip = (clipId: number, additive = false) =>
    expandLinkedSelection({ kind: "video_clip", clip_id: clipId }, additive);
  const pendingRemoveAudioClip = () =>
    props.audioClips.find((clip) => clip.id === pendingRemoveAudioClipId()) ?? null;
  createEffect(() => {
    if (selectedAudioClipId() !== null && !selectedAudioClip()) setSelectedAudioClipId(null);
    if (selectedVideoClipId() !== null && !selectedVideoClip()) setSelectedVideoClipId(null);
    const retained = selectedTimelineItemRefs();
    if (retained.length !== selectedTimelineItems().length) setSelectedTimelineItems(retained);
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
    for (const clip of props.videoClips) {
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
    && props.videoClips.length === 0
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
      <Show when={props.embeddedControls}>
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
      <Show when={props.childTimelineLabel === null && props.timelineBank.length > 0}>
        <TimelineBankPanel
          timelines={props.timelineBank}
          activeTimelineId={props.activeTimelineId}
          bpm={props.bpm}
          followRuntime={props.followRuntime}
          onCreate={props.onCreateTimeline}
          onDuplicate={props.onDuplicateTimeline}
          onRemove={props.onRemoveTimeline}
          onReorder={props.onReorderTimelines}
          onSelect={props.onSelectTimeline}
          onSetFollow={props.onSetFollow}
        />
      </Show>
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
        <button
          type="button"
          classList={{ active: props.guideEnabled }}
          aria-pressed={props.guideEnabled}
          data-timeline-guide
          title="Announce phases and loop transitions"
          onClick={() => void props.onSetGuideEnabled(!props.guideEnabled)}
        >
          Guide
        </button>
      </div>
      <div class="timelineLoopControls" role="group" aria-label="Timeline A-B loop controls">
        <button type="button" title="Set loop A at playhead" onClick={setLoopAAtPlayhead}>A</button>
        <button type="button" title="Set loop B at playhead" onClick={setLoopBAtPlayhead}>B</button>
        <button
          type="button"
          classList={{ active: loopRuntimeEnabled() }}
          aria-pressed={loopRuntimeEnabled()}
          disabled={props.loopRegion === null}
          title={loopRuntimeEnabled() ? "Disable loop (Break)" : "Enable or arm loop"}
          data-timeline-loop-toggle
          onClick={() => void props.onSetLoopEnabled(!loopRuntimeEnabled())}
        >
          Loop
        </button>
        <button
          type="button"
          title="Halve loop length"
          disabled={props.loopRegion === null}
          onClick={() => void props.onScaleLoop("half")}
        >
          1/2
        </button>
        <button
          type="button"
          title="Double loop length"
          disabled={props.loopRegion === null}
          onClick={() => void props.onScaleLoop("double")}
        >
          ×2
        </button>
        <output class={`timelineLoopState ${props.loopRuntime.status}`} aria-live="polite" data-no-localize>
          {props.loopRuntime.status === "armed"
            ? "ARMED"
            : props.loopRuntime.status === "looping"
              ? `LOOP ×${props.loopRuntime.wrap_count}`
              : "OFF"}
        </output>
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
      <details class="timelinePhaseEditor" data-timeline-phase-editor>
        <summary>
          <span>Phases</span>
          <strong class="tabularNums" data-no-localize>{props.phases.length}</strong>
        </summary>
        <div class="timelinePhaseEditorBody">
          <div class="timelinePhaseRail" aria-label="Timeline phases">
            <For each={props.phases.filter((phase) => (
              phase.end_ms > props.visibleWindow.start_ms && phase.start_ms < props.visibleWindow.end_ms
            ))}>
              {(phase) => {
                const span = Math.max(1, timelineVisibleWindowSpanMs(props.visibleWindow));
                const left = Math.max(0, phase.start_ms - props.visibleWindow.start_ms) / span * 100;
                const right = Math.min(props.visibleWindow.end_ms, phase.end_ms) - props.visibleWindow.start_ms;
                const width = Math.max(0.5, right / span * 100 - left);
                return (
                  <button
                    type="button"
                    class={`timelinePhaseBand ${phase.role}`}
                    style={{ left: `${left}%`, width: `${width}%` }}
                    title={`${phase.label}: ${formatTimelineHeaderTime(phase.start_ms)}–${formatTimelineHeaderTime(phase.end_ms)}`}
                    onClick={() => void props.onSeek(phase.start_ms)}
                  >
                    <span data-no-localize>{phase.label}</span>
                  </button>
                );
              }}
            </For>
          </div>
          <div class="timelinePhaseRows">
            <For each={props.phases}>
              {(phase) => (
                <div class="timelinePhaseRow" data-phase-id={phase.id}>
                  <input
                    aria-label="Phase label"
                    value={phase.label}
                    maxlength="64"
                    onChange={(event) => updatePhase(phase.id, { label: event.currentTarget.value.trim() || phase.label })}
                  />
                  <select
                    aria-label="Phase role"
                    value={phase.role}
                    onChange={(event) => updatePhase(phase.id, { role: event.currentTarget.value as TimelinePhaseRole })}
                  >
                    <option value="intro">Intro</option>
                    <option value="verse">Verse</option>
                    <option value="pre_chorus">Pre-Chorus</option>
                    <option value="chorus">Chorus</option>
                    <option value="bridge">Bridge</option>
                    <option value="outro">Outro</option>
                    <option value="custom">Custom</option>
                  </select>
                  <label>
                    <span>Start</span>
                    <input
                      class="tabularNums"
                      type="number"
                      min="0"
                      value={phase.start_ms}
                      onChange={(event) => updatePhase(phase.id, { start_ms: Math.max(0, Number(event.currentTarget.value)) })}
                    />
                  </label>
                  <label>
                    <span>End</span>
                    <input
                      class="tabularNums"
                      type="number"
                      min={phase.start_ms + 1}
                      value={phase.end_ms}
                      onChange={(event) => updatePhase(phase.id, { end_ms: Math.max(phase.start_ms + 1, Number(event.currentTarget.value)) })}
                    />
                  </label>
                  <button
                    type="button"
                    class="danger ghost"
                    aria-label={`Remove phase ${phase.label}`}
                    onClick={() => void props.onSetPhases(props.phases.filter((candidate) => candidate.id !== phase.id))}
                  >
                    Remove
                  </button>
                </div>
              )}
            </For>
          </div>
          <button type="button" class="secondary" onClick={addPhase}>Add Phase at Playhead</button>
        </div>
      </details>
      </>
      </Show>
      <TimelineOverview
        events={props.overviewEvents}
        layerItemCounts={timelineLayerItemCounts()}
        cueIdentities={props.cueIdentities}
        audioClips={props.audioClips}
        videoClips={props.videoClips}
        mediaAssets={props.mediaAssets}
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
        selectedVideoClipId={selectedVideoClipId()}
        selectedAudioClipIds={selectedAudioClipIds()}
        selectedVideoClipIds={selectedVideoClipIds()}
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
        onSelectAudioClip={selectAudioClip}
        onSelectVideoClip={selectVideoClip}
        onOpenItemContextMenu={setItemContextMenu}
        onInspectOverlapCluster={inspectOverlapCluster}
        onUpdateLayer={(layer) => void props.onUpdateTimelineLayer(layer)}
        onAddAudioClip={(layerId) => void props.onAddAudioClip(layerId)}
        onUpdateAudioClip={(clip) => void props.onUpdateAudioClip(clip)}
        onUpdateVideoClip={(clip) => void props.onUpdateVideoClip(clip)}
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
      <Show when={itemContextMenu()}>
        {(menu) => (
          <div
            class="timelineItemContextMenu"
            role="menu"
            aria-label="Timeline item group actions"
            style={{ left: `${menu().x}px`, top: `${menu().y}px` }}
            onContextMenu={(event) => event.preventDefault()}
          >
            <button
              type="button"
              role="menuitem"
              disabled={selectedTimelineItemRefs().length < 2 || itemGroupForSelection() !== null}
              onClick={() => {
                void props.onGroupItems(selectedTimelineItemRefs());
                setItemContextMenu(null);
              }}
            >
              Group selected
            </button>
            <button
              type="button"
              role="menuitem"
              disabled={itemGroupForSelection() === null}
              onClick={() => {
                const item = selectedTimelineItemRefs()[0];
                if (item) void props.onUngroupItem(item);
                setItemContextMenu(null);
              }}
            >
              Ungroup
            </button>
            <button type="button" role="menuitem" onClick={() => setItemContextMenu(null)}>Close</button>
          </div>
        )}
      </Show>
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
                setSelectedTimelineItems((items) => items.filter((item) =>
                  !(item.kind === "audio_clip" && item.clip_id === clipId)));
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
