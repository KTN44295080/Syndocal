import { createMemo, createSignal, For, onCleanup, onMount, Show } from "solid-js";
import { shouldCommitTimelineMarkerDrag, timelineConformRateBadge } from "../timelineSceneBlocks";
import {
  beginTimelineAbsoluteDragProjection,
  buildTimelineRulerTicks,
  timelineTimeToVisibleRawRatio,
  timelineVisibleRatioToTimeMs,
  timelineVisibleWindowSpanMs,
  updateTimelineAbsoluteDragProjection,
  type TimelineVisibleWindow,
} from "../timelineViewport";
import {
  packTimelineOverlapClusterBadges,
  TIMELINE_OVERLAP_BADGE_WIDTH_PX,
} from "../timelineOverlapClusters";
import {
  cueIdentityCss,
  cueIdentityHue,
  identityCssColor,
  type CueIdentitySource,
} from "../identityColor";
import { formatCompactClock } from "../clockDisplay";
import type {
  AudioAnalysisSummary,
  TimelineAudioClipSummary,
  TimelineLayerKind,
  TimelineLayerSummary,
  TimelineTrackKind,
} from "../types";
import type { TimelineCueDragState } from "../timelineCueDrag";
import {
  TIMELINE_BLOCK_FADE_EDGE_PX,
  TIMELINE_BLOCK_STRETCH_EDGE_PX,
  TIMELINE_BLOCK_UPPER_BAND_PX,
  projectTimelineBlockFadeMs,
  projectTimelineBlockStretch,
  timelineBlockGestureZone,
  type TimelineStretchMode,
} from "../timelineBlockGestures";

export interface TimelineOverviewEvent {
  id: number;
  cue_id: number;
  cue_label: string;
  track: TimelineTrackKind;
  layer_id: number;
  time_ms: number;
  duration_ms: number;
  loop_count: number;
  conform_to_tempo: boolean;
  loop_fill: boolean;
  authored_beats: number | null;
  rate: number | null;
  total_duration_ms: number;
  fade_in_ms: number;
  fade_out_ms: number;
  x: number;
  width: number;
  y: number;
  under_playhead: boolean;
  is_super_scene: boolean;
}

export interface TimelineOverviewAutomationRange {
  id: string;
  kind: "lighting" | "video";
  automation_id: number;
  target_id: number;
  label: string;
  track: TimelineTrackKind;
  layer_id: number;
  start_ms: number;
  end_ms: number;
  keyframes: { keyframe_index: number; time_ms: number }[];
  x: number;
  width: number;
  y: number;
  enabled: boolean;
}

export interface TimelineOverviewOverlapCluster {
  id: string;
  label: string;
  track: TimelineTrackKind;
  layer_id: number;
  start_ms: number;
  end_ms: number;
  count: number;
  member_ids: number[];
  x: number;
  width: number;
  source_cluster_ids?: string[];
  group_count?: number;
  aggregated?: boolean;
}

interface TimelineOverviewProps {
  layers: TimelineLayerSummary[];
  legacyMode: boolean;
  cueIdentities?: Record<number, CueIdentitySource>;
  cueDrag: TimelineCueDragState | null;
  events: TimelineOverviewEvent[];
  layerItemCounts: ReadonlyMap<number, number>;
  audioClips: TimelineAudioClipSummary[];
  audioAnalysis: AudioAnalysisSummary | null;
  executionLive: boolean;
  markerAriaLabel: (event: TimelineOverviewEvent) => string;
  automationRanges: TimelineOverviewAutomationRange[];
  superSceneEmptyHintCount: number;
  overlapClusters: TimelineOverviewOverlapCluster[];
  overlapLayerIds: number[];
  selectedRangeId: string | null;
  selectedEventId: number | null;
  selectedAudioClipId: number | null;
  playheadX: number;
  visibleWindow: TimelineVisibleWindow;
  bpm: number;
  stretchMode: TimelineStretchMode;
  magnetEnabled: boolean;
  armedCue: {
    id: number;
    label: string;
    authored_beats: number | null;
    natural_duration_ms: number;
  } | null;
  snapTimeMs: (timeMs: number) => number;
  onSeekTime: (timeMs: number) => void;
  onSelectAutomationRange: (range: TimelineOverviewAutomationRange) => void;
  onSelectEvent: (eventId: number, openProperties?: boolean) => void;
  onOpenSuperScene: (cueId: number) => void;
  onSelectAudioClip: (clipId: number) => void;
  onInspectOverlapCluster: (cluster: TimelineOverviewOverlapCluster) => void;
  onUpdateLayer: (layer: TimelineLayerSummary) => void | Promise<void>;
  onOpenLayerMenu: (layer: TimelineLayerSummary, point: { x: number; y: number }) => void;
  onAddAudioClip: (layerId: number) => void | Promise<void>;
  onUpdateAudioClip: (clip: TimelineAudioClipSummary) => void | Promise<void>;
  onStatus: (message: string) => void;
  onMoveEventPlacement: (eventId: number, timeMs: number, layerId: number, snapEnabled: boolean) => void;
  onResizeEventTime: (
    eventId: number,
    edge: "start" | "end",
    timeMs: number,
    mode: TimelineStretchMode,
    snapEnabled: boolean,
  ) => void;
  onSetEventFade: (eventId: number, edge: "in" | "out", fadeMs: number, snapEnabled: boolean) => void;
  onSetVisibleWindow: (window: TimelineVisibleWindow) => void;
  onZoomAt: (anchorMs: number, scale: number) => void;
  onPlaceArmedCue: (
    cueId: number,
    timeMs: number,
    layerId: number,
    durationMs: number,
    mode: TimelineStretchMode,
    snapEnabled: boolean,
  ) => void;
  onMoveAutomationRangeTime: (range: TimelineOverviewAutomationRange, timeMs: number) => void;
  onResizeAutomationRangeTime: (
    range: TimelineOverviewAutomationRange,
    edge: "start" | "end",
    timeMs: number,
  ) => void;
  onMoveAutomationKeyframeTime: (
    range: TimelineOverviewAutomationRange,
    keyframeIndex: number,
    timeMs: number,
  ) => void;
}

interface TimelineMarkerDrag {
  eventId: number;
  pointerId: number;
  originalTimeMs: number;
  timeMs: number;
  startClientX: number;
  startClientY: number;
  originalLayerId: number;
  layerId: number;
  hoverLayerId: number | null;
  rejection: "locked" | "kind" | "audio" | null;
  moved: boolean;
}

interface TimelineEventResizeDrag {
  eventId: number;
  pointerId: number;
  edge: "start" | "end";
  startClientX: number;
  moved: boolean;
  originalStartMs: number;
  originalEndMs: number;
  startMs: number;
  endMs: number;
}

interface TimelineEventFadeDrag {
  eventId: number;
  pointerId: number;
  edge: "in" | "out";
  startClientX: number;
  originalFadeMs: number;
  fadeMs: number;
  moved: boolean;
}

interface TimelineCanvasPanDrag {
  pointerId: number;
  startClientX: number;
  originalWindow: TimelineVisibleWindow;
  moved: boolean;
}

interface TimelinePlacementDrag {
  pointerId: number;
  cueId: number;
  layerId: number;
  startMs: number;
  endMs: number;
  moved: boolean;
}

interface TimelineAudioClipDrag {
  clipId: number;
  pointerId: number;
  mode: "move" | "resize-start" | "resize-end" | "fade-in" | "fade-out";
  startClientX: number;
  startClientY: number;
  original: TimelineAudioClipSummary;
  preview: TimelineAudioClipSummary;
  hoverLayerId: number | null;
  rejection: "locked" | "kind" | null;
  moved: boolean;
}

interface TimelineLayerRowLayout {
  layer: TimelineLayerSummary;
  top: number;
  height: number;
}

interface TimelineSectionRowLayout {
  kind: TimelineLayerKind;
  top: number;
  height: number;
  totalHeight: number;
  collapsed: boolean;
  layers: TimelineLayerSummary[];
}

const timelineSectionKinds: TimelineLayerKind[] = ["Audio", "Lighting", "Video"];
const timelineSectionHeaderHeightPx = 14;
const timelineUserLaneHeightPx = 36;
const timelineExpandedLaneHeightPx = 54;
const legacyTimelineLayers: TimelineLayerSummary[] = [
  { id: 0, label: "Lighting", order: 0, muted: false, locked: false, solo: false, kind: "Lighting" },
  { id: 1, label: "Video", order: 1, muted: false, locked: false, solo: false, kind: "Video" },
];

interface TimelineAutomationRangeDrag {
  rangeId: string;
  pointerId: number;
  mode: "move" | "resize-start" | "resize-end";
  startClientX: number;
  moved: boolean;
  originalStartMs: number;
  originalEndMs: number;
  startMs: number;
  endMs: number;
}

interface TimelineAutomationKeyframeDrag {
  rangeId: string;
  keyframeIndex: number;
  pointerId: number;
  startClientX: number;
  moved: boolean;
  originalTimeMs: number;
  timeMs: number;
}

const clampRatio = (value: number) => Math.min(1, Math.max(0, value));
const sameNumberSet = (left: Set<number>, right: Set<number>) =>
  left.size === right.size && [...left].every((value) => right.has(value));

const sameOverviewEvent = (left: TimelineOverviewEvent, right: TimelineOverviewEvent) =>
  left.id === right.id &&
  left.cue_id === right.cue_id &&
  left.cue_label === right.cue_label &&
  left.track === right.track &&
  left.layer_id === right.layer_id &&
  left.time_ms === right.time_ms &&
  left.duration_ms === right.duration_ms &&
  left.loop_count === right.loop_count &&
  left.conform_to_tempo === right.conform_to_tempo &&
  left.loop_fill === right.loop_fill &&
  left.authored_beats === right.authored_beats &&
  left.rate === right.rate &&
  left.total_duration_ms === right.total_duration_ms &&
  left.fade_in_ms === right.fade_in_ms &&
  left.fade_out_ms === right.fade_out_ms &&
  left.is_super_scene === right.is_super_scene &&
  left.x === right.x &&
  left.width === right.width &&
  left.y === right.y;

const sameOverviewAutomationRange = (
  left: TimelineOverviewAutomationRange,
  right: TimelineOverviewAutomationRange,
) => left.id === right.id &&
  left.kind === right.kind &&
  left.automation_id === right.automation_id &&
  left.target_id === right.target_id &&
  left.label === right.label &&
  left.track === right.track &&
  left.layer_id === right.layer_id &&
  left.start_ms === right.start_ms &&
  left.end_ms === right.end_ms &&
  left.x === right.x &&
  left.width === right.width &&
  left.y === right.y &&
  left.enabled === right.enabled &&
  left.keyframes.length === right.keyframes.length &&
  left.keyframes.every((keyframe, index) => {
    const candidate = right.keyframes[index];
    return keyframe.keyframe_index === candidate.keyframe_index && keyframe.time_ms === candidate.time_ms;
  });

const stableTimelineAudioPathHash = (path: string) => {
  let hash = 2166136261;
  for (let index = 0; index < path.length; index += 1) {
    hash ^= path.charCodeAt(index);
    hash = Math.imul(hash, 16777619);
  }
  return hash >>> 0;
};

const timelineAudioClipName = (path: string) => {
  const normalized = path.replaceAll("\\", "/");
  return normalized.split("/").pop() || path || "Audio Clip";
};

export function TimelineOverview(props: TimelineOverviewProps) {
  const [markerDrag, setMarkerDrag] = createSignal<TimelineMarkerDrag | null>(null);
  const [eventResizeDrag, setEventResizeDrag] = createSignal<TimelineEventResizeDrag | null>(null);
  const [eventFadeDrag, setEventFadeDrag] = createSignal<TimelineEventFadeDrag | null>(null);
  const [canvasPanDrag, setCanvasPanDrag] = createSignal<TimelineCanvasPanDrag | null>(null);
  const [placementDrag, setPlacementDrag] = createSignal<TimelinePlacementDrag | null>(null);
  const [audioClipDrag, setAudioClipDrag] = createSignal<TimelineAudioClipDrag | null>(null);
  const [rangeDrag, setRangeDrag] = createSignal<TimelineAutomationRangeDrag | null>(null);
  const [keyframeDrag, setKeyframeDrag] = createSignal<TimelineAutomationKeyframeDrag | null>(null);
  const [suppressClickEventId, setSuppressClickEventId] = createSignal<number | null>(null);
  const [suppressClickRangeId, setSuppressClickRangeId] = createSignal<string | null>(null);
  const [keyboardMarkerEventId, setKeyboardMarkerEventId] = createSignal<number | null>(null);
  const [suppressCanvasClick, setSuppressCanvasClick] = createSignal(false);
  // True pixel-space canvas: the SVG viewBox mirrors the measured client box
  // (0 0 width height), so every user unit equals one rendered pixel and text
  // renders undistorted at any aspect ratio (mirrors ValueEffectEditorPanel).
  const [overviewPixelWidth, setOverviewPixelWidth] = createSignal(1);
  const [overviewPixelHeight, setOverviewPixelHeight] = createSignal(78);
  const [lightingLaneVisible, setLightingLaneVisible] = createSignal(true);
  const [videoLaneVisible, setVideoLaneVisible] = createSignal(true);
  const [collapsedSections, setCollapsedSections] = createSignal<Set<TimelineLayerKind>>(new Set());
  const [expandedLayerIds, setExpandedLayerIds] = createSignal<Set<number>>(new Set());
  let overviewElement: SVGSVGElement | undefined;
  onMount(() => {
    const updateViewBox = () => {
      if (!overviewElement) return;
      const width = overviewElement.clientWidth;
      const height = overviewElement.clientHeight;
      if (width > 0 && height > 0) {
        setOverviewPixelWidth(width);
        setOverviewPixelHeight(height);
      }
    };
    updateViewBox();
    const observer = new ResizeObserver(updateViewBox);
    if (overviewElement) observer.observe(overviewElement);
    const handleWheel = (event: WheelEvent) => {
      if (!overviewElement) return;
      event.preventDefault();
      const ratio = ratioFromPointer(event, overviewElement);
      const anchorMs = timelineVisibleRatioToTimeMs(ratio, props.visibleWindow);
      props.onZoomAt(anchorMs, event.deltaY > 0 ? 1.2 : 0.8);
    };
    const handleEscape = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return;
      const pan = canvasPanDrag();
      if (
        !markerDrag() &&
        !eventResizeDrag() &&
        !eventFadeDrag() &&
        !placementDrag() &&
        !audioClipDrag() &&
        !rangeDrag() &&
        !keyframeDrag() &&
        !pan
      ) return;
      event.preventDefault();
      // Gesture rollback owns Escape ahead of drawers, menus, and the T8 pane
      // expansion listener. Other Escape handlers run only when no gesture is
      // active, so the preview geometry is always restored first.
      event.stopImmediatePropagation();
      if (pan) props.onSetVisibleWindow(pan.originalWindow);
      setMarkerDrag(null);
      setEventResizeDrag(null);
      setEventFadeDrag(null);
      setPlacementDrag(null);
      setAudioClipDrag(null);
      setRangeDrag(null);
      setKeyframeDrag(null);
      setCanvasPanDrag(null);
      setSuppressCanvasClick(true);
      props.onStatus("Timeline drag canceled; the original geometry was restored.");
    };
    overviewElement?.addEventListener("wheel", handleWheel, { passive: false });
    window.addEventListener("keydown", handleEscape, { capture: true });
    onCleanup(() => {
      observer.disconnect();
      overviewElement?.removeEventListener("wheel", handleWheel);
      window.removeEventListener("keydown", handleEscape, { capture: true });
    });
  });

  const overviewW = () => Math.max(1, overviewPixelWidth());
  const overviewH = () => Math.max(1, overviewPixelHeight());
  const laneVisible = (track: TimelineTrackKind) =>
    track === "Lighting" ? lightingLaneVisible() : videoLaneVisible();
  const orderedLayers = createMemo(() => [...props.layers].sort(
    (left, right) => left.order - right.order || left.id - right.id,
  ));
  const effectiveLayers = createMemo(() => props.legacyMode ? legacyTimelineLayers : orderedLayers());
  const layerById = createMemo(() => new Map(effectiveLayers().map((layer) => [layer.id, layer])));
  const overlapLayerIds = createMemo(
    () => new Set(props.overlapLayerIds),
    new Set<number>(),
    { equals: sameNumberSet },
  );
  const timelineLayerHeightPx = (layerId: number) =>
    expandedLayerIds().has(layerId) || overlapLayerIds().has(layerId)
      ? timelineExpandedLaneHeightPx
      : timelineUserLaneHeightPx;
  const sectionLayout = createMemo(() => {
    const collapsed = collapsedSections();
    const sections: TimelineSectionRowLayout[] = [];
    const laneRows: TimelineLayerRowLayout[] = [];
    let top = 0;
    for (const kind of timelineSectionKinds) {
      const layers = orderedLayers().filter((layer) => layer.kind === kind);
      const isCollapsed = collapsed.has(kind);
      const sectionTop = top;
      top += timelineSectionHeaderHeightPx;
      if (!isCollapsed) {
        for (const layer of layers) {
          const height = timelineLayerHeightPx(layer.id);
          laneRows.push({ layer, top, height });
          top += height;
        }
      }
      sections.push({
        kind,
        top: sectionTop,
        height: timelineSectionHeaderHeightPx,
        totalHeight: top - sectionTop,
        collapsed: isCollapsed,
        layers,
      });
    }
    return { sections, laneRows, contentHeight: Math.max(timelineSectionHeaderHeightPx * 3, top) };
  });
  const layerRowById = createMemo(() => new Map(
    sectionLayout().laneRows.map((row) => [row.layer.id, row]),
  ));
  const canvasH = () => props.legacyMode ? overviewH() : sectionLayout().contentHeight;
  const toggleSection = (kind: TimelineLayerKind) => {
    setCollapsedSections((current) => {
      const next = new Set(current);
      if (next.has(kind)) next.delete(kind);
      else next.add(kind);
      return next;
    });
  };
  const toggleLayerExpanded = (layerId: number) => {
    setExpandedLayerIds((current) => {
      const next = new Set(current);
      if (next.has(layerId)) next.delete(layerId);
      else next.add(layerId);
      return next;
    });
  };
  const layerIdFromPoint = (clientX: number, clientY: number) => {
    if (typeof document === "undefined") return null;
    const target = document.elementFromPoint(clientX, clientY)?.closest<Element>("[data-timeline-layer-id]");
    if (!target || !target.closest(".timelineOverviewFrame")) return null;
    const layerId = Number(target.getAttribute("data-timeline-layer-id"));
    return Number.isFinite(layerId) && layerById().has(layerId) ? layerId : null;
  };
  const cueDragTargetLayerId = createMemo(() => {
    const drag = props.cueDrag;
    return drag?.moved ? layerIdFromPoint(drag.client_x, drag.client_y) : null;
  });
  const cueDropStateForLayer = (layerId: number) => {
    if (cueDragTargetLayerId() !== layerId) return "none" as const;
    const layer = layerById().get(layerId);
    return layer?.kind === "Lighting" && !layer.locked ? "valid" as const : "rejected" as const;
  };
  const markerDropStateForLayer = (layerId: number) => {
    const drag = markerDrag();
    if (!drag || drag.hoverLayerId !== layerId) return "none" as const;
    return drag.rejection === null ? "valid" as const : "rejected" as const;
  };
  const safeCueDrag = createMemo(() => {
    const drag = props.cueDrag;
    return drag?.moved && Number.isFinite(drag.client_x) && Number.isFinite(drag.client_y) ? drag : null;
  });
  const matrixCueDropState = () => {
    const drag = safeCueDrag();
    if (!drag || drag.source_surface !== "scene-matrix" || typeof document === "undefined") return null;
    const sourceCard = document.querySelector<HTMLElement>(
      `[data-scene-matrix-cue-id="${drag.cue_id}"]`,
    );
    const hitElement = document.elementFromPoint(drag.client_x, drag.client_y);
    const targetCard = hitElement?.closest<HTMLElement>("[data-scene-matrix-cue-id]");
    if (!sourceCard) return null;
    const sourceColumn = sourceCard.closest<HTMLElement>("[data-scene-matrix-column]");
    const targetColumn = hitElement?.closest<HTMLElement>("[data-scene-matrix-column]");
    if (!targetColumn) return null;
    const sameColumn =
      sourceColumn?.dataset.sceneMatrixColumn === targetColumn?.dataset.sceneMatrixColumn;
    const sameCueList =
      !targetCard ||
      sourceCard.dataset.sceneMatrixCueListId === targetCard.dataset.sceneMatrixCueListId;
    return sameCueList &&
      (!sameColumn || (
        targetCard &&
        targetCard.dataset.sceneMatrixCueId !== String(drag.cue_id)
      ))
      ? "matrix" as const
      : "rejected" as const;
  };
  let cachedEventsById = new Map<number, TimelineOverviewEvent>();
  const stableEvents = createMemo(() => {
    const nextCache = new Map<number, TimelineOverviewEvent>();
    const events = props.events.map((event) => {
      const cached = cachedEventsById.get(event.id);
      const stable = cached && sameOverviewEvent(cached, event) ? cached : event;
      nextCache.set(event.id, stable);
      return stable;
    });
    cachedEventsById = nextCache;
    return events;
  });
  let cachedAutomationRangesById = new Map<string, TimelineOverviewAutomationRange>();
  const stableAutomationRanges = createMemo(() => {
    const nextCache = new Map<string, TimelineOverviewAutomationRange>();
    const ranges = props.automationRanges.map((range) => {
      const cached = cachedAutomationRangesById.get(range.id);
      const stable = cached && sameOverviewAutomationRange(cached, range) ? cached : range;
      nextCache.set(range.id, stable);
      return stable;
    });
    cachedAutomationRangesById = nextCache;
    return ranges;
  });
  const underPlayheadEventIds = createMemo(
    () => new Set(props.events.filter((event) => event.under_playhead).map((event) => event.id)),
    new Set<number>(),
    { equals: sameNumberSet },
  );
  const orderedEvents = createMemo(() => {
    const events = stableEvents();
    const selectedEventId = props.selectedEventId;
    if (selectedEventId === null) return events;
    const selectedEvent = events.find((event) => event.id === selectedEventId);
    return selectedEvent
      ? [...events.filter((event) => event.id !== selectedEventId), selectedEvent]
      : events;
  });
  const renderedEvents = createMemo(() => props.legacyMode
    ? orderedEvents()
    : orderedEvents().filter((event) => layerRowById().has(event.layer_id)));
  const renderedAudioClips = createMemo(() => props.legacyMode
    ? []
    : props.audioClips.filter((clip) => layerRowById().get(clip.layer_id)?.layer.kind === "Audio"));
  const superSceneEmptyHint = createMemo(() => props.superSceneEmptyHintCount === 1
    ? "The timeline is empty. 1 Super Scene has a child timeline (open it from the SS badge in Scene Matrix)."
    : `The timeline is empty. ${props.superSceneEmptyHintCount} Super Scenes have child timelines (open them from the SS badges in Scene Matrix).`);
  const audioClipTabStopId = createMemo(() => {
    const clips = renderedAudioClips();
    return clips.some((clip) => clip.id === props.selectedAudioClipId)
      ? props.selectedAudioClipId
      : clips[0]?.id ?? null;
  });
  const markerTabStopId = createMemo(() => {
    const events = renderedEvents();
    if (events.some((event) => event.id === props.selectedEventId)) return props.selectedEventId;
    const keyboardId = keyboardMarkerEventId();
    return events.some((event) => event.id === keyboardId) ? keyboardId : (events[0]?.id ?? null);
  });
  const focusMarkerById = (eventId: number) => {
    setKeyboardMarkerEventId(eventId);
    requestAnimationFrame(() => {
      overviewElement
        ?.querySelector<SVGGElement>(`.timelineMarker[data-timeline-event-id="${eventId}"]`)
        ?.focus();
    });
  };
  const moveMarkerKeyboardFocus = (eventId: number, direction: -1 | 1 | "first" | "last") => {
    const events = renderedEvents();
    if (events.length === 0) return;
    const currentIndex = Math.max(0, events.findIndex((event) => event.id === eventId));
    const nextIndex = direction === "first"
      ? 0
      : direction === "last"
        ? events.length - 1
        : (currentIndex + direction + events.length) % events.length;
    const next = events[nextIndex];
    props.onSelectEvent(next.id);
    focusMarkerById(next.id);
  };

  const ratioFromPointer = (event: PointerEvent | MouseEvent, svg: SVGSVGElement) => {
    const rect = svg.getBoundingClientRect();
    return clampRatio((event.clientX - rect.left) / Math.max(1, rect.width));
  };

  const seekFromPointer = (event: MouseEvent & { currentTarget: SVGSVGElement }) => {
    if (suppressCanvasClick()) {
      setSuppressCanvasClick(false);
      return;
    }
    props.onSeekTime(timelineVisibleRatioToTimeMs(
      ratioFromPointer(event, event.currentTarget),
      props.visibleWindow,
    ));
  };

  const beginMarkerDrag = (event: PointerEvent & { currentTarget: SVGGElement }, overviewEvent: TimelineOverviewEvent) => {
    const svg = event.currentTarget.ownerSVGElement;
    if (!svg) {
      return;
    }
    event.preventDefault();
    event.stopPropagation();
    props.onSelectEvent(overviewEvent.id);
    const sourceLayer = layerById().get(overviewEvent.layer_id);
    if (!sourceLayer) {
      props.onStatus(`Timeline layer ${overviewEvent.layer_id} was not found.`);
      return;
    }
    if (sourceLayer.locked) {
      props.onStatus(`Timeline layer ${sourceLayer.label} is locked. Unlock it before moving blocks.`);
      return;
    }
    if (sourceLayer.kind === "Audio") {
      props.onStatus("Scene Blocks cannot be moved from Audio layers.");
      return;
    }
    event.currentTarget.setPointerCapture(event.pointerId);
    const projection = beginTimelineAbsoluteDragProjection(overviewEvent.time_ms, event.clientX);
    setMarkerDrag({
      eventId: overviewEvent.id,
      pointerId: event.pointerId,
      originalTimeMs: projection.original_time_ms,
      timeMs: projection.time_ms,
      startClientX: projection.start_client_x,
      startClientY: event.clientY,
      originalLayerId: overviewEvent.layer_id,
      layerId: overviewEvent.layer_id,
      hoverLayerId: overviewEvent.layer_id,
      rejection: null,
      moved: projection.moved,
    });
  };

  const moveMarkerDrag = (event: PointerEvent & { currentTarget: SVGGElement }) => {
    const drag = markerDrag();
    const svg = event.currentTarget.ownerSVGElement;
    if (!drag || drag.pointerId !== event.pointerId || !svg) {
      return;
    }
    event.preventDefault();
    event.stopPropagation();
    const projection = updateTimelineAbsoluteDragProjection(
      {
        original_time_ms: drag.originalTimeMs,
        time_ms: drag.timeMs,
        start_client_x: drag.startClientX,
        moved: drag.moved,
      },
      event.clientX,
      svg.getBoundingClientRect().width,
      timelineVisibleWindowSpanMs(props.visibleWindow),
    );
    const hoverLayerId = layerIdFromPoint(event.clientX, event.clientY);
    const sourceLayer = layerById().get(drag.originalLayerId);
    const targetLayer = hoverLayerId === null ? undefined : layerById().get(hoverLayerId);
    const rejection = !targetLayer || !sourceLayer
      ? null
      : targetLayer.kind === "Audio"
        ? "audio" as const
        : targetLayer.kind !== sourceLayer.kind
          ? "kind" as const
          : targetLayer.locked
            ? "locked" as const
            : null;
    const nextLayerId = targetLayer && rejection === null ? targetLayer.id : drag.originalLayerId;
    setMarkerDrag({
      ...drag,
      timeMs: props.magnetEnabled ? props.snapTimeMs(projection.time_ms) : projection.time_ms,
      layerId: nextLayerId,
      hoverLayerId,
      rejection,
      moved: projection.moved || Math.abs(event.clientY - drag.startClientY) >= 4,
    });
  };

  const finishMarkerDrag = (
    event: PointerEvent & { currentTarget: SVGGElement },
    canceled: boolean,
  ) => {
    const drag = markerDrag();
    if (!drag || drag.pointerId !== event.pointerId) {
      return;
    }
    event.preventDefault();
    event.stopPropagation();
    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }
    setMarkerDrag(null);
    if (!shouldCommitTimelineMarkerDrag(drag, canceled)) {
      return;
    }
    setSuppressClickEventId(drag.eventId);
    if (drag.rejection !== null) {
      const targetLayer = drag.hoverLayerId === null ? undefined : layerById().get(drag.hoverLayerId);
      if (drag.rejection === "locked") {
        props.onStatus(`Timeline layer ${targetLayer?.label ?? drag.hoverLayerId ?? "target"} is locked. No changes were made.`);
      } else if (drag.rejection === "audio") {
        props.onStatus("Scene Blocks cannot be moved to Audio layers. No changes were made.");
      } else {
        props.onStatus("Scene Blocks can only move within the same timeline section. No changes were made.");
      }
      return;
    }
    props.onMoveEventPlacement(drag.eventId, drag.timeMs, drag.layerId, props.magnetEnabled);
  };
  const endMarkerDrag = (event: PointerEvent & { currentTarget: SVGGElement }) =>
    finishMarkerDrag(event, false);
  const cancelMarkerDrag = (event: PointerEvent & { currentTarget: SVGGElement }) =>
    finishMarkerDrag(event, true);

  const beginEventResize = (
    event: PointerEvent & { currentTarget: SVGGraphicsElement },
    overviewEvent: TimelineOverviewEvent,
    edge: "start" | "end",
  ) => {
    const svg = event.currentTarget.ownerSVGElement;
    if (!svg || overviewEvent.duration_ms <= 0) return;
    event.preventDefault();
    event.stopPropagation();
    const layer = layerById().get(overviewEvent.layer_id);
    if (layer?.locked) {
      props.onStatus(`Timeline layer ${layer.label} is locked. Unlock it before resizing blocks.`);
      return;
    }
    props.onSelectEvent(overviewEvent.id);
    event.currentTarget.setPointerCapture(event.pointerId);
    setEventResizeDrag({
      eventId: overviewEvent.id,
      pointerId: event.pointerId,
      edge,
      startClientX: event.clientX,
      moved: false,
      originalStartMs: overviewEvent.time_ms,
      originalEndMs: overviewEvent.time_ms + overviewEvent.total_duration_ms,
      startMs: overviewEvent.time_ms,
      endMs: overviewEvent.time_ms + overviewEvent.total_duration_ms,
    });
  };

  const moveEventResize = (event: PointerEvent & { currentTarget: SVGGraphicsElement }) => {
    const drag = eventResizeDrag();
    const svg = event.currentTarget.ownerSVGElement;
    if (!drag || drag.pointerId !== event.pointerId || !svg) return;
    event.preventDefault();
    event.stopPropagation();
    const originalTimeMs = drag.edge === "end" ? drag.originalEndMs : drag.originalStartMs;
    const projection = updateTimelineAbsoluteDragProjection(
      {
        original_time_ms: originalTimeMs,
        time_ms: drag.edge === "end" ? drag.endMs : drag.startMs,
        start_client_x: drag.startClientX,
        moved: drag.moved,
      },
      event.clientX,
      svg.getBoundingClientRect().width,
      timelineVisibleWindowSpanMs(props.visibleWindow),
      drag.edge === "end" ? drag.originalStartMs + 1 : 0,
      drag.edge === "start" ? drag.originalEndMs - 1 : Number.POSITIVE_INFINITY,
    );
    if (!projection.moved) return;
    const projectedTimeMs = props.magnetEnabled ? props.snapTimeMs(projection.time_ms) : projection.time_ms;
    setEventResizeDrag(drag.edge === "start"
      ? { ...drag, moved: true, startMs: Math.min(projectedTimeMs, drag.originalEndMs - 1) }
      : { ...drag, moved: true, endMs: Math.max(projectedTimeMs, drag.originalStartMs + 1) });
  };

  const finishEventResize = (
    event: PointerEvent & { currentTarget: SVGGraphicsElement },
    canceled: boolean,
  ) => {
    const drag = eventResizeDrag();
    if (!drag || drag.pointerId !== event.pointerId) return;
    event.preventDefault();
    event.stopPropagation();
    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }
    setEventResizeDrag(null);
    if (!shouldCommitTimelineMarkerDrag(drag, canceled)) return;
    setSuppressClickEventId(drag.eventId);
    props.onResizeEventTime(
      drag.eventId,
      drag.edge,
      drag.edge === "start" ? drag.startMs : drag.endMs,
      props.stretchMode,
      props.magnetEnabled,
    );
  };
  const endEventResize = (event: PointerEvent & { currentTarget: SVGGraphicsElement }) =>
    finishEventResize(event, false);
  const cancelEventResize = (event: PointerEvent & { currentTarget: SVGGraphicsElement }) =>
    finishEventResize(event, true);

  const beginEventFade = (
    event: PointerEvent & { currentTarget: SVGGraphicsElement },
    overviewEvent: TimelineOverviewEvent,
    edge: "in" | "out",
  ) => {
    if (overviewEvent.duration_ms <= 0) return;
    event.preventDefault();
    event.stopPropagation();
    const layer = layerById().get(overviewEvent.layer_id);
    if (layer?.locked) {
      props.onStatus(`Timeline layer ${layer.label} is locked. Unlock it before editing fades.`);
      return;
    }
    props.onSelectEvent(overviewEvent.id);
    event.currentTarget.setPointerCapture(event.pointerId);
    const originalFadeMs = edge === "in" ? overviewEvent.fade_in_ms : overviewEvent.fade_out_ms;
    setEventFadeDrag({
      eventId: overviewEvent.id,
      pointerId: event.pointerId,
      edge,
      startClientX: event.clientX,
      originalFadeMs,
      fadeMs: originalFadeMs,
      moved: false,
    });
  };

  const moveEventFade = (event: PointerEvent & { currentTarget: SVGGraphicsElement }) => {
    const drag = eventFadeDrag();
    const svg = event.currentTarget.ownerSVGElement;
    if (!drag || drag.pointerId !== event.pointerId || !svg) return;
    event.preventDefault();
    event.stopPropagation();
    const overviewEvent = stableEvents().find((candidate) => candidate.id === drag.eventId);
    if (!overviewEvent) return;
    const deltaMs = (event.clientX - drag.startClientX)
      / Math.max(1, svg.getBoundingClientRect().width)
      * timelineVisibleWindowSpanMs(props.visibleWindow);
    const blockStartMs = overviewEvent.time_ms;
    const blockEndMs = blockStartMs + overviewEvent.total_duration_ms;
    const pointerTimeMs = drag.edge === "in"
      ? blockStartMs + drag.originalFadeMs + deltaMs
      : blockEndMs - drag.originalFadeMs + deltaMs;
    const bounded = Math.min(
      overviewEvent.duration_ms,
      projectTimelineBlockFadeMs(drag.edge, blockStartMs, blockEndMs, pointerTimeMs),
    );
    const fadeMs = props.magnetEnabled
      ? Math.min(overviewEvent.duration_ms, Math.max(0, props.snapTimeMs(bounded)))
      : bounded;
    setEventFadeDrag({
      ...drag,
      fadeMs,
      moved: drag.moved || Math.abs(event.clientX - drag.startClientX) >= 4,
    });
  };

  const finishEventFade = (
    event: PointerEvent & { currentTarget: SVGGraphicsElement },
    canceled: boolean,
  ) => {
    const drag = eventFadeDrag();
    if (!drag || drag.pointerId !== event.pointerId) return;
    event.preventDefault();
    event.stopPropagation();
    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }
    setEventFadeDrag(null);
    if (canceled || !drag.moved) return;
    setSuppressClickEventId(drag.eventId);
    props.onSetEventFade(drag.eventId, drag.edge, drag.fadeMs, props.magnetEnabled);
  };
  const endEventFade = (event: PointerEvent & { currentTarget: SVGGraphicsElement }) =>
    finishEventFade(event, false);
  const cancelEventFade = (event: PointerEvent & { currentTarget: SVGGraphicsElement }) =>
    finishEventFade(event, true);

  const beginBlockGesture = (
    event: PointerEvent & { currentTarget: SVGGElement },
    overviewEvent: TimelineOverviewEvent,
  ) => {
    if (overviewEvent.duration_ms <= 0) {
      beginMarkerDrag(event, overviewEvent);
      return;
    }
    const rect = event.currentTarget.getBoundingClientRect();
    const zone = timelineBlockGestureZone(
      event.clientX - rect.left,
      event.clientY - rect.top,
      sceneBlockPixelWidth(overviewEvent),
      props.selectedEventId === overviewEvent.id,
      blockHeightPx(),
    );
    if (zone === "stretch-start" || zone === "stretch-end") {
      beginEventResize(event, overviewEvent, zone === "stretch-start" ? "start" : "end");
      return;
    }
    if (zone === "fade-in" || zone === "fade-out") {
      beginEventFade(event, overviewEvent, zone === "fade-in" ? "in" : "out");
      return;
    }
    if (zone === "select") {
      event.preventDefault();
      event.stopPropagation();
      props.onSelectEvent(overviewEvent.id, true);
      return;
    }
    beginMarkerDrag(event, overviewEvent);
  };
  const moveBlockGesture = (event: PointerEvent & { currentTarget: SVGGElement }) => {
    if (eventResizeDrag()) moveEventResize(event);
    else if (eventFadeDrag()) moveEventFade(event);
    else moveMarkerDrag(event);
  };
  const finishBlockGesture = (
    event: PointerEvent & { currentTarget: SVGGElement },
    canceled: boolean,
  ) => {
    if (eventResizeDrag()) finishEventResize(event, canceled);
    else if (eventFadeDrag()) finishEventFade(event, canceled);
    else finishMarkerDrag(event, canceled);
  };

  const beginAudioClipGesture = (
    event: PointerEvent & { currentTarget: SVGGElement },
    clip: TimelineAudioClipSummary,
  ) => {
    const layer = layerById().get(clip.layer_id);
    if (layer?.locked) {
      props.onStatus(`Timeline layer ${layer.label} is locked. Unlock it before editing Audio Clips.`);
      return;
    }
    event.preventDefault();
    event.stopPropagation();
    props.onSelectAudioClip(clip.id);
    const rect = event.currentTarget.getBoundingClientRect();
    const zone = timelineBlockGestureZone(
      event.clientX - rect.left,
      event.clientY - rect.top,
      Math.max(1, rect.width),
      props.selectedAudioClipId === clip.id,
      blockHeightPx(),
    );
    const mode = zone === "stretch-start"
      ? "resize-start"
      : zone === "stretch-end"
        ? "resize-end"
        : zone === "fade-in"
          ? "fade-in"
          : zone === "fade-out"
            ? "fade-out"
            : "move";
    event.currentTarget.setPointerCapture(event.pointerId);
    setAudioClipDrag({
      clipId: clip.id,
      pointerId: event.pointerId,
      mode,
      startClientX: event.clientX,
      startClientY: event.clientY,
      original: { ...clip },
      preview: { ...clip },
      hoverLayerId: clip.layer_id,
      rejection: null,
      moved: false,
    });
  };

  const moveAudioClipGesture = (event: PointerEvent & { currentTarget: SVGGElement }) => {
    const drag = audioClipDrag();
    const svg = event.currentTarget.ownerSVGElement;
    if (!drag || drag.pointerId !== event.pointerId || !svg) return;
    event.preventDefault();
    event.stopPropagation();
    const deltaMs = (event.clientX - drag.startClientX)
      / Math.max(1, svg.getBoundingClientRect().width)
      * timelineVisibleWindowSpanMs(props.visibleWindow);
    const originalEndMs = drag.original.start_ms + drag.original.duration_ms;
    let preview = { ...drag.preview };
    let hoverLayerId = drag.hoverLayerId;
    let rejection = drag.rejection;
    if (drag.mode === "move") {
      const rawStartMs = Math.max(0, Math.round(drag.original.start_ms + deltaMs));
      preview.start_ms = props.magnetEnabled ? props.snapTimeMs(rawStartMs) : rawStartMs;
      hoverLayerId = layerIdFromPoint(event.clientX, event.clientY);
      const target = hoverLayerId === null ? undefined : layerById().get(hoverLayerId);
      rejection = !target
        ? null
        : target.kind !== "Audio"
          ? "kind"
          : target.locked
            ? "locked"
            : null;
      preview.layer_id = target && rejection === null ? target.id : drag.original.layer_id;
    } else if (drag.mode === "resize-start") {
      const earliestStartMs = Math.max(0, drag.original.start_ms - drag.original.offset_ms);
      const rawStartMs = Math.min(
        originalEndMs - 1,
        Math.max(earliestStartMs, Math.round(drag.original.start_ms + deltaMs)),
      );
      const startMs = props.magnetEnabled ? props.snapTimeMs(rawStartMs) : rawStartMs;
      const boundedStartMs = Math.min(originalEndMs - 1, Math.max(earliestStartMs, startMs));
      const trimDeltaMs = boundedStartMs - drag.original.start_ms;
      preview.start_ms = boundedStartMs;
      preview.offset_ms = Math.max(0, drag.original.offset_ms + trimDeltaMs);
      preview.duration_ms = Math.max(1, originalEndMs - boundedStartMs);
      preview.fade_in_ms = Math.min(preview.fade_in_ms, preview.duration_ms);
      preview.fade_out_ms = Math.min(preview.fade_out_ms, preview.duration_ms - preview.fade_in_ms);
    } else if (drag.mode === "resize-end") {
      const rawEndMs = Math.max(drag.original.start_ms + 1, Math.round(originalEndMs + deltaMs));
      const endMs = props.magnetEnabled ? props.snapTimeMs(rawEndMs) : rawEndMs;
      preview.duration_ms = Math.max(1, endMs - drag.original.start_ms);
      preview.fade_in_ms = Math.min(preview.fade_in_ms, preview.duration_ms);
      preview.fade_out_ms = Math.min(preview.fade_out_ms, preview.duration_ms - preview.fade_in_ms);
    } else {
      const edge = drag.mode === "fade-in" ? "in" : "out";
      const pointerTimeMs = edge === "in"
        ? drag.original.start_ms + drag.original.fade_in_ms + deltaMs
        : originalEndMs - drag.original.fade_out_ms + deltaMs;
      const projected = projectTimelineBlockFadeMs(
        edge,
        drag.original.start_ms,
        originalEndMs,
        pointerTimeMs,
      );
      const fadeMs = props.magnetEnabled ? props.snapTimeMs(projected) : projected;
      if (edge === "in") {
        preview.fade_in_ms = Math.min(
          drag.original.duration_ms - drag.original.fade_out_ms,
          Math.max(0, fadeMs),
        );
      } else {
        preview.fade_out_ms = Math.min(
          drag.original.duration_ms - drag.original.fade_in_ms,
          Math.max(0, fadeMs),
        );
      }
    }
    setAudioClipDrag({
      ...drag,
      preview,
      hoverLayerId,
      rejection,
      moved: drag.moved
        || Math.abs(event.clientX - drag.startClientX) >= 4
        || Math.abs(event.clientY - drag.startClientY) >= 4,
    });
  };

  const finishAudioClipGesture = (
    event: PointerEvent & { currentTarget: SVGGElement },
    canceled: boolean,
  ) => {
    const drag = audioClipDrag();
    if (!drag || drag.pointerId !== event.pointerId) return;
    event.preventDefault();
    event.stopPropagation();
    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }
    setAudioClipDrag(null);
    setSuppressCanvasClick(true);
    if (canceled || !drag.moved) return;
    if (drag.rejection) {
      props.onStatus(drag.rejection === "locked"
        ? "The target Audio lane is locked. No changes were made."
        : "Audio Clips can only move within Audio lanes. No changes were made.");
      return;
    }
    void props.onUpdateAudioClip(drag.preview);
  };

  const isCanvasTarget = (target: EventTarget | null) => {
    const element = target instanceof Element ? target : null;
    return !element?.closest(".timelineMarker, .timelineAudioClip, .timelineAutomationRange, .timelineOverlapCluster, .timelinePlayheadGroup");
  };

  const validatePlacementLayer = (layerId: number | null) => {
    const layer = layerId === null ? undefined : layerById().get(layerId);
    if (!layer) {
      props.onStatus("Choose a timeline lane before placing the armed Cue.");
      return null;
    }
    if (layer.locked) {
      props.onStatus(`Timeline layer ${layer.label} is locked. Unlock it before placing blocks.`);
      return null;
    }
    if (layer.kind !== "Lighting") {
      props.onStatus("Scene Blocks can only be placed on Lighting lanes.");
      return null;
    }
    return layer;
  };

  const beginCanvasPointer = (event: PointerEvent & { currentTarget: SVGSVGElement }) => {
    if (!isCanvasTarget(event.target) || event.button !== 0) return;
    const armedCue = props.armedCue;
    const layerId = layerIdFromPoint(event.clientX, event.clientY);
    if (armedCue) {
      const layer = validatePlacementLayer(layerId);
      if (!layer) return;
      event.preventDefault();
      event.currentTarget.setPointerCapture(event.pointerId);
      const rawTimeMs = timelineVisibleRatioToTimeMs(ratioFromPointer(event, event.currentTarget), props.visibleWindow);
      const timeMs = props.magnetEnabled ? props.snapTimeMs(rawTimeMs) : Math.round(rawTimeMs);
      setPlacementDrag({
        pointerId: event.pointerId,
        cueId: armedCue.id,
        layerId: layer.id,
        startMs: timeMs,
        endMs: timeMs,
        moved: false,
      });
      return;
    }
    event.preventDefault();
    event.currentTarget.setPointerCapture(event.pointerId);
    setCanvasPanDrag({
      pointerId: event.pointerId,
      startClientX: event.clientX,
      originalWindow: { ...props.visibleWindow },
      moved: false,
    });
  };

  const moveCanvasPointer = (event: PointerEvent & { currentTarget: SVGSVGElement }) => {
    const placement = placementDrag();
    if (placement?.pointerId === event.pointerId) {
      event.preventDefault();
      const rawTimeMs = timelineVisibleRatioToTimeMs(ratioFromPointer(event, event.currentTarget), props.visibleWindow);
      const endMs = props.magnetEnabled ? props.snapTimeMs(rawTimeMs) : Math.round(rawTimeMs);
      setPlacementDrag({
        ...placement,
        endMs,
        moved: placement.moved || Math.abs(endMs - placement.startMs) >= 1,
      });
      return;
    }
    const pan = canvasPanDrag();
    if (!pan || pan.pointerId !== event.pointerId) return;
    event.preventDefault();
    const deltaPx = event.clientX - pan.startClientX;
    const deltaMs = -deltaPx / Math.max(1, event.currentTarget.getBoundingClientRect().width)
      * timelineVisibleWindowSpanMs(pan.originalWindow);
    props.onSetVisibleWindow({
      start_ms: pan.originalWindow.start_ms + deltaMs,
      end_ms: pan.originalWindow.end_ms + deltaMs,
    });
    setCanvasPanDrag({ ...pan, moved: pan.moved || Math.abs(deltaPx) >= 4 });
  };

  const finishCanvasPointer = (
    event: PointerEvent & { currentTarget: SVGSVGElement },
    canceled: boolean,
  ) => {
    const placement = placementDrag();
    if (placement?.pointerId === event.pointerId) {
      if (event.currentTarget.hasPointerCapture(event.pointerId)) event.currentTarget.releasePointerCapture(event.pointerId);
      setPlacementDrag(null);
      if (!canceled && placement.moved) {
        const startMs = Math.min(placement.startMs, placement.endMs);
        const durationMs = Math.max(1, Math.abs(placement.endMs - placement.startMs));
        props.onPlaceArmedCue(
          placement.cueId,
          startMs,
          placement.layerId,
          durationMs,
          props.stretchMode,
          props.magnetEnabled,
        );
      }
      setSuppressCanvasClick(true);
      return;
    }
    const pan = canvasPanDrag();
    if (!pan || pan.pointerId !== event.pointerId) return;
    if (event.currentTarget.hasPointerCapture(event.pointerId)) event.currentTarget.releasePointerCapture(event.pointerId);
    if (canceled) props.onSetVisibleWindow(pan.originalWindow);
    setCanvasPanDrag(null);
    if (pan.moved || canceled) setSuppressCanvasClick(true);
  };

  const placeArmedCueAtDoubleClick = (event: MouseEvent & { currentTarget: SVGSVGElement }) => {
    const armedCue = props.armedCue;
    if (!armedCue || !isCanvasTarget(event.target)) return;
    event.preventDefault();
    event.stopPropagation();
    const layer = validatePlacementLayer(layerIdFromPoint(event.clientX, event.clientY));
    if (!layer) return;
    const rawTimeMs = timelineVisibleRatioToTimeMs(ratioFromPointer(event, event.currentTarget), props.visibleWindow);
    const timeMs = props.magnetEnabled ? props.snapTimeMs(rawTimeMs) : Math.round(rawTimeMs);
    props.onPlaceArmedCue(
      armedCue.id,
      timeMs,
      layer.id,
      armedCue.natural_duration_ms,
      props.stretchMode,
      props.magnetEnabled,
    );
    setSuppressCanvasClick(true);
  };

  const beginRangeDrag = (event: PointerEvent & { currentTarget: SVGGElement }, range: TimelineOverviewAutomationRange) => {
    const svg = event.currentTarget.ownerSVGElement;
    if (!svg) {
      return;
    }
    event.preventDefault();
    event.stopPropagation();
    props.onSelectAutomationRange(range);
    event.currentTarget.setPointerCapture(event.pointerId);
    setRangeDrag({
      rangeId: range.id,
      pointerId: event.pointerId,
      mode: "move",
      startClientX: event.clientX,
      moved: false,
      originalStartMs: range.start_ms,
      originalEndMs: range.end_ms,
      startMs: range.start_ms,
      endMs: range.end_ms,
    });
  };

  const beginRangeResize = (
    event: PointerEvent & { currentTarget: SVGRectElement },
    range: TimelineOverviewAutomationRange,
    edge: "start" | "end",
  ) => {
    const svg = event.currentTarget.ownerSVGElement;
    if (!svg) {
      return;
    }
    event.preventDefault();
    event.stopPropagation();
    props.onSelectAutomationRange(range);
    event.currentTarget.setPointerCapture(event.pointerId);
    setRangeDrag({
      rangeId: range.id,
      pointerId: event.pointerId,
      mode: edge === "start" ? "resize-start" : "resize-end",
      startClientX: event.clientX,
      moved: false,
      originalStartMs: range.start_ms,
      originalEndMs: range.end_ms,
      startMs: range.start_ms,
      endMs: range.end_ms,
    });
  };

  const moveRangeDrag = (event: PointerEvent & { currentTarget: SVGGElement }) => {
    const drag = rangeDrag();
    const svg = event.currentTarget.ownerSVGElement;
    const range = props.automationRanges.find((candidate) => candidate.id === drag?.rangeId);
    if (!drag || drag.pointerId !== event.pointerId || !svg || !range) {
      return;
    }
    event.preventDefault();
    event.stopPropagation();
    const originalTimeMs = drag.mode === "resize-end" ? drag.originalEndMs : drag.originalStartMs;
    const projection = updateTimelineAbsoluteDragProjection(
      {
        original_time_ms: originalTimeMs,
        time_ms: drag.mode === "resize-end" ? drag.endMs : drag.startMs,
        start_client_x: drag.startClientX,
        moved: drag.moved,
      },
      event.clientX,
      svg.getBoundingClientRect().width,
      timelineVisibleWindowSpanMs(props.visibleWindow),
      drag.mode === "resize-end" ? drag.originalStartMs + 1 : 0,
      drag.mode === "resize-start" ? drag.originalEndMs - 1 : Number.POSITIVE_INFINITY,
    );
    if (!projection.moved) return;
    if (drag.mode === "resize-start") {
      setRangeDrag({
        ...drag,
        moved: projection.moved,
        startMs: projection.time_ms,
      });
      return;
    }
    if (drag.mode === "resize-end") {
      setRangeDrag({
        ...drag,
        moved: projection.moved,
        endMs: projection.time_ms,
      });
      return;
    }
    const durationMs = Math.max(1, drag.originalEndMs - drag.originalStartMs);
    setRangeDrag({
      ...drag,
      moved: projection.moved,
      startMs: projection.time_ms,
      endMs: projection.time_ms + durationMs,
    });
  };

  const finishRangeDrag = (
    event: PointerEvent & { currentTarget: SVGGElement },
    canceled: boolean,
  ) => {
    const drag = rangeDrag();
    if (!drag || drag.pointerId !== event.pointerId) {
      return;
    }
    event.preventDefault();
    event.stopPropagation();
    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }
    setRangeDrag(null);
    if (!shouldCommitTimelineMarkerDrag(drag, canceled)) {
      return;
    }
    setSuppressClickRangeId(drag.rangeId);
    const range = props.automationRanges.find((candidate) => candidate.id === drag.rangeId);
    if (range) {
      if (drag.mode === "resize-start") {
        props.onResizeAutomationRangeTime(
          range,
          "start",
          drag.startMs,
        );
      } else if (drag.mode === "resize-end") {
        props.onResizeAutomationRangeTime(
          range,
          "end",
          drag.endMs,
        );
      } else {
        props.onMoveAutomationRangeTime(
          range,
          drag.startMs,
        );
      }
    }
  };
  const endRangeDrag = (event: PointerEvent & { currentTarget: SVGGElement }) =>
    finishRangeDrag(event, false);
  const cancelRangeDrag = (event: PointerEvent & { currentTarget: SVGGElement }) =>
    finishRangeDrag(event, true);

  const beginKeyframeDrag = (
    event: PointerEvent & { currentTarget: SVGGElement },
    range: TimelineOverviewAutomationRange,
    keyframeIndex: number,
  ) => {
    const svg = event.currentTarget.ownerSVGElement;
    if (!svg) {
      return;
    }
    event.preventDefault();
    event.stopPropagation();
    event.currentTarget.setPointerCapture(event.pointerId);
    const keyframeTimeMs = range.keyframes.find(
      (keyframe) => keyframe.keyframe_index === keyframeIndex,
    )?.time_ms ?? range.start_ms;
    setKeyframeDrag({
      rangeId: range.id,
      keyframeIndex,
      pointerId: event.pointerId,
      startClientX: event.clientX,
      moved: false,
      originalTimeMs: keyframeTimeMs,
      timeMs: keyframeTimeMs,
    });
  };

  const moveKeyframeDrag = (event: PointerEvent & { currentTarget: SVGGElement }) => {
    const drag = keyframeDrag();
    const svg = event.currentTarget.ownerSVGElement;
    if (!drag || drag.pointerId !== event.pointerId || !svg) {
      return;
    }
    event.preventDefault();
    event.stopPropagation();
    const projection = updateTimelineAbsoluteDragProjection(
      {
        original_time_ms: drag.originalTimeMs,
        time_ms: drag.timeMs,
        start_client_x: drag.startClientX,
        moved: drag.moved,
      },
      event.clientX,
      svg.getBoundingClientRect().width,
      timelineVisibleWindowSpanMs(props.visibleWindow),
    );
    if (!projection.moved) return;
    setKeyframeDrag({
      ...drag,
      moved: projection.moved,
      timeMs: projection.time_ms,
    });
  };

  const finishKeyframeDrag = (
    event: PointerEvent & { currentTarget: SVGGElement },
    canceled: boolean,
  ) => {
    const drag = keyframeDrag();
    if (!drag || drag.pointerId !== event.pointerId) {
      return;
    }
    event.preventDefault();
    event.stopPropagation();
    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }
    setKeyframeDrag(null);
    if (!shouldCommitTimelineMarkerDrag(drag, canceled)) {
      return;
    }
    setSuppressClickRangeId(drag.rangeId);
    const range = props.automationRanges.find((candidate) => candidate.id === drag.rangeId);
    if (range) {
      props.onMoveAutomationKeyframeTime(
        range,
        drag.keyframeIndex,
        drag.timeMs,
      );
    }
  };
  const endKeyframeDrag = (event: PointerEvent & { currentTarget: SVGGElement }) =>
    finishKeyframeDrag(event, false);
  const cancelKeyframeDrag = (event: PointerEvent & { currentTarget: SVGGElement }) =>
    finishKeyframeDrag(event, true);

  const automationRangeX = (range: TimelineOverviewAutomationRange) => {
    const drag = rangeDrag();
    return drag?.rangeId === range.id
      ? timelineTimeToVisibleRawRatio(drag.startMs, props.visibleWindow) * 100
      : range.x;
  };
  const automationRangeWidth = (range: TimelineOverviewAutomationRange) => {
    const drag = rangeDrag();
    return drag?.rangeId === range.id
      ? ((drag.endMs - drag.startMs) / timelineVisibleWindowSpanMs(props.visibleWindow)) * 100
      : range.width;
  };
  const automationKeyframeX = (range: TimelineOverviewAutomationRange, keyframeIndex: number, timeMs: number) => {
    const drag = keyframeDrag();
    if (drag?.rangeId === range.id && drag.keyframeIndex === keyframeIndex) {
      return timelineTimeToVisibleRawRatio(drag.timeMs, props.visibleWindow) * 100;
    }
    const spanMs = Math.max(1, range.end_ms - range.start_ms);
    const localRatio = clampRatio((timeMs - range.start_ms) / spanMs);
    return automationRangeX(range) + localRatio * automationRangeWidth(range);
  };
  // ---- Pixel-space layout ------------------------------------------------
  // Legacy keeps the measured two-lane geometry. Authored layers use a compact
  // 14px section separator with 36px lanes (54px when details are expanded)
  // inside the scrollport; content height stays out of the frame's intrinsic size.
  const viewBoxX = (percent: number) => (percent / 100) * overviewW();
  const legacyLaneHeightPx = () => overviewH() / 2;
  const legacyLaneTopPx = (track: TimelineTrackKind) => track === "Lighting" ? 0 : legacyLaneHeightPx();
  const layerRow = (layerId: number, track: TimelineTrackKind) => props.legacyMode
    ? { top: legacyLaneTopPx(track), height: legacyLaneHeightPx() }
    : layerRowById().get(layerId) ?? { top: 0, height: timelineUserLaneHeightPx };
  const blockHeightPx = () => props.legacyMode ? Math.max(9, legacyLaneHeightPx() * 0.52) : 28;
  // The Lighting lane clears the ruler labels at the very top of the canvas.
  const blockTopPx = (event: Pick<TimelineOverviewEvent, "layer_id" | "track">) => {
    const row = layerRow(event.layer_id, event.track);
    return props.legacyMode
      ? row.top + row.height * (event.track === "Lighting" ? 0.30 : 0.18)
      : row.top + 4;
  };
  const blockCenterYPx = (event: Pick<TimelineOverviewEvent, "layer_id" | "track">) =>
    blockTopPx(event) + blockHeightPx() / 2;
  const blockUpperBandHeightPx = () => blockHeightPx() / 2;
  const automationLayerRow = (
    range: TimelineOverviewAutomationRange,
  ): { top: number; height: number; layerId: number } | undefined => {
    if (props.legacyMode) {
      const layerId = range.track === "Lighting" ? 0 : 1;
      return { ...layerRow(layerId, range.track), layerId };
    }
    const kind: TimelineLayerKind = range.kind === "lighting" ? "Lighting" : "Video";
    const row = layerRowById().get(range.layer_id)
      ?? sectionLayout().laneRows.find((candidate) => candidate.layer.kind === kind);
    return row ? { top: row.top, height: row.height, layerId: row.layer.id } : undefined;
  };
  const automationBarHeightPx = (range: TimelineOverviewAutomationRange) => {
    const row = automationLayerRow(range);
    if (!row) return 3;
    const pseudoEvent = {
      layer_id: row.layerId,
      track: range.track,
    };
    return Math.max(3, Math.min(9, (row.top + row.height - (blockTopPx(pseudoEvent) + blockHeightPx())) * 0.72));
  };
  const automationBarTopPx = (range: TimelineOverviewAutomationRange) => {
    const row = automationLayerRow(range);
    const pseudoEvent = {
      layer_id: row?.layerId ?? (range.track === "Lighting" ? 0 : 1),
      track: range.track,
    };
    return blockTopPx(pseudoEvent) + blockHeightPx() - automationBarHeightPx(range) * 0.3;
  };
  const rulerLabelBaselineY = () => props.legacyMode
    ? Math.max(9, Math.min(14, overviewH() * 0.12))
    : 14;
  const rulerLineTopPx = () => rulerLabelBaselineY() + 2;
  const rulerLineBottomPx = () => canvasH() - 1;
  const clusterBadgeWidthPx = TIMELINE_OVERLAP_BADGE_WIDTH_PX;
  // Overlap lanes reserve a compact rail below the 28px block. Keeping the ×N
  // badge in that rail preserves both the upper move band and lower fade band.
  const clusterBadgeTopPx = (cluster: TimelineOverviewOverlapCluster) => {
    const row = layerRow(cluster.layer_id, cluster.track);
    return props.legacyMode ? row.top + 2 : row.top + row.height - 18;
  };
  const clusterBadgeHeightPx = () => 15;

  const nameInsetPx = 4;
  const nameCharWidthPx = 6.6;
  const eventPreviewStartMs = (event: TimelineOverviewEvent) => {
    const resize = eventResizeDrag();
    if (resize?.eventId === event.id) return resize.startMs;
    const move = markerDrag();
    return move?.eventId === event.id ? move.timeMs : event.time_ms;
  };
  const eventPreviewLayerId = (event: TimelineOverviewEvent) => {
    const move = markerDrag();
    return move?.eventId === event.id ? move.layerId : event.layer_id;
  };
  const eventPreviewEndMs = (event: TimelineOverviewEvent) => {
    const resize = eventResizeDrag();
    return resize?.eventId === event.id ? resize.endMs : eventPreviewStartMs(event) + event.total_duration_ms;
  };
  const eventPreviewSpanMs = (event: TimelineOverviewEvent) => Math.max(0, eventPreviewEndMs(event) - eventPreviewStartMs(event));
  const eventStretchProjection = (event: TimelineOverviewEvent) => {
    const resize = eventResizeDrag();
    if (resize?.eventId !== event.id) return null;
    return projectTimelineBlockStretch({
      mode: props.stretchMode,
      edge: resize.edge,
      originalStartMs: resize.originalStartMs,
      originalEndMs: resize.originalEndMs,
      requestedEdgeMs: resize.edge === "start" ? resize.startMs : resize.endMs,
      authoredBeats: event.authored_beats,
      bpm: props.bpm,
    });
  };
  const eventPreviewRate = (event: TimelineOverviewEvent) => {
    const projection = eventStretchProjection(event);
    return projection?.rate ?? event.rate;
  };
  const eventPreviewFadeMs = (event: TimelineOverviewEvent, edge: "in" | "out") => {
    const drag = eventFadeDrag();
    if (drag?.eventId === event.id && drag.edge === edge) return drag.fadeMs;
    return edge === "in" ? event.fade_in_ms : event.fade_out_ms;
  };
  const sceneBlockPixelWidth = (event: TimelineOverviewEvent) => Math.max(
    eventResizeDrag()?.eventId === event.id
      ? (eventPreviewSpanMs(event) / timelineVisibleWindowSpanMs(props.visibleWindow)) * overviewW()
      : viewBoxX(event.width),
    0.8,
  );
  const sceneBlockHasReadableBody = (event: TimelineOverviewEvent) => sceneBlockPixelWidth(event) >= 16;
  const sceneBlockShowsDuration = (event: TimelineOverviewEvent) => sceneBlockPixelWidth(event) >= 48;
  const sceneBlockName = (event: TimelineOverviewEvent) => {
    // Identity name only; loop/× semantics stay in the marker title and the
    // aggregate overlap badge so the two counts never read ambiguously.
    const label = event.cue_label;
    const previewRate = eventPreviewRate(event);
    const rateBadge = eventStretchProjection(event)?.fallback_to_window
      ? null
      : previewRate !== null && previewRate !== undefined && previewRate > 0
        ? `[${previewRate.toFixed(2)}x]`
        : null;
    const available = Math.max(0, sceneBlockPixelWidth(event) - nameInsetPx * 2);
    const maxCharacters = Math.floor(available / nameCharWidthPx);
    if (maxCharacters < 1) return "";
    if (rateBadge && maxCharacters >= rateBadge.length) {
      const labelCharacters = maxCharacters - rateBadge.length - 1;
      if (labelCharacters <= 0) return rateBadge;
      const visibleLabel = label.length <= labelCharacters
        ? label
        : `${label.slice(0, Math.max(1, labelCharacters - 1))}…`;
      return `${visibleLabel} ${rateBadge}`;
    }
    return label.length <= maxCharacters ? label : `${label.slice(0, Math.max(1, maxCharacters - 1))}…`;
  };
  const sceneBlockDurationStamp = (event: TimelineOverviewEvent) =>
    formatCompactClock(eventPreviewSpanMs(event));
  const sceneBlockFadeRampPoints = (event: TimelineOverviewEvent, edge: "in" | "out") => {
    const widthPx = sceneBlockPixelWidth(event);
    const previewSpanMs = eventPreviewSpanMs(event);
    if (previewSpanMs <= 0) return null;
    const fadePx = Math.min(widthPx, (eventPreviewFadeMs(event, edge) / previewSpanMs) * widthPx);
    if (fadePx < 1) return null;
    const halfHeight = blockHeightPx() / 2;
    return edge === "in"
      ? `0,${halfHeight} 0,0 ${fadePx},0`
      : `${widthPx},${halfHeight} ${widthPx},0 ${widthPx - fadePx},0`;
  };
  const audioClipPreview = (clip: TimelineAudioClipSummary) =>
    audioClipDrag()?.clipId === clip.id ? audioClipDrag()!.preview : clip;
  const audioClipPixelWidth = (clip: TimelineAudioClipSummary) => Math.max(
    0.8,
    audioClipPreview(clip).duration_ms
      / timelineVisibleWindowSpanMs(props.visibleWindow)
      * overviewW(),
  );
  const audioClipCenterYPx = (clip: TimelineAudioClipSummary) => {
    const row = layerRowById().get(audioClipPreview(clip).layer_id);
    return (row?.top ?? 0) + 4 + blockHeightPx() / 2;
  };
  const audioClipVisibleName = (clip: TimelineAudioClipSummary) => {
    const label = timelineAudioClipName(clip.path);
    const maxCharacters = Math.floor(Math.max(0, audioClipPixelWidth(clip) - nameInsetPx * 2) / nameCharWidthPx);
    if (maxCharacters < 1) return "";
    return label.length <= maxCharacters ? label : `${label.slice(0, Math.max(1, maxCharacters - 1))}…`;
  };
  const audioClipShowsDuration = (clip: TimelineAudioClipSummary) => audioClipPixelWidth(clip) >= 80;
  const audioClipWaveformPoints = (clip: TimelineAudioClipSummary) => {
    const preview = audioClipPreview(clip);
    const width = audioClipPixelWidth(clip);
    const stripTop = -blockHeightPx() / 2 + blockUpperBandHeightPx() + 1;
    const stripHeight = Math.max(2, blockHeightPx() - blockUpperBandHeightPx() - 2);
    const analysis = props.audioAnalysis;
    if (!analysis || analysis.path !== clip.path || analysis.waveform.length === 0) {
      return `0,${stripTop + stripHeight / 2} ${width},${stripTop + stripHeight / 2}`;
    }
    const sourceStartMs = preview.offset_ms;
    const sourceEndMs = sourceStartMs + preview.duration_ms;
    const candidates = analysis.waveform.filter((point) =>
      point.time_ms >= sourceStartMs && point.time_ms <= sourceEndMs);
    if (candidates.length === 0) {
      return `0,${stripTop + stripHeight / 2} ${width},${stripTop + stripHeight / 2}`;
    }
    const nodeBudget = 64;
    const stride = Math.max(1, Math.ceil(candidates.length / nodeBudget));
    const sampled = candidates.filter((_, index) => index % stride === 0).slice(0, nodeBudget);
    return sampled.map((point) => {
      const ratio = clampRatio((point.time_ms - sourceStartMs) / Math.max(1, preview.duration_ms));
      const peak = Math.min(1, Math.max(0, Math.abs(point.peak)));
      return `${(ratio * width).toFixed(2)},${(stripTop + (1 - peak) * stripHeight).toFixed(2)}`;
    }).join(" ");
  };
  const audioClipFadeRampPoints = (clip: TimelineAudioClipSummary, edge: "in" | "out") => {
    const preview = audioClipPreview(clip);
    const fadeMs = edge === "in" ? preview.fade_in_ms : preview.fade_out_ms;
    if (fadeMs <= 0 || preview.duration_ms <= 0) return null;
    const width = audioClipPixelWidth(clip);
    const fadePx = Math.min(width, fadeMs / preview.duration_ms * width);
    if (fadePx < 1) return null;
    const halfHeight = blockHeightPx() / 2;
    return edge === "in"
      ? `0,${halfHeight} 0,0 ${fadePx},0`
      : `${width},${halfHeight} ${width},0 ${width - fadePx},0`;
  };
  const audioClipLiveStamp = (clip: TimelineAudioClipSummary) => {
    const drag = audioClipDrag();
    if (drag?.clipId !== clip.id) return null;
    if (drag.mode === "fade-in") return `Fade In ${drag.preview.fade_in_ms} ms`;
    if (drag.mode === "fade-out") return `Fade Out ${drag.preview.fade_out_ms} ms`;
    return `${formatCompactClock(drag.preview.start_ms)} · ${formatCompactClock(drag.preview.duration_ms)}`;
  };
  const activeDragStamp = (event: TimelineOverviewEvent) => {
    const fade = eventFadeDrag();
    if (fade?.eventId === event.id) return `Fade ${fade.edge === "in" ? "In" : "Out"} ${fade.fadeMs} ms`;
    const resize = eventResizeDrag();
    if (resize?.eventId === event.id) {
      const projection = eventStretchProjection(event);
      const badge = projection?.rate ? ` [${projection.rate.toFixed(2)}x]` : "";
      return `${Math.round(eventPreviewSpanMs(event))} ms${badge}`;
    }
    const move = markerDrag();
    return move?.eventId === event.id ? `${Math.round(move.timeMs)} ms` : null;
  };
  const placementPreview = createMemo(() => {
    const placement = placementDrag();
    if (!placement) return null;
    const startMs = Math.min(placement.startMs, placement.endMs);
    const endMs = Math.max(placement.startMs, placement.endMs);
    return {
      ...placement,
      startMs,
      endMs,
      durationMs: Math.max(1, endMs - startMs),
    };
  });
  const laneCounts = createMemo(() => {
    let lighting = 0;
    let video = 0;
    for (const event of props.events) {
      if (event.track === "Lighting") lighting += 1;
      else video += 1;
    }
    return { lighting, video };
  });

  const packedOverlapClusters = createMemo(() => {
    const visible = props.overlapClusters.filter((cluster) =>
      props.legacyMode || layerRowById().has(cluster.layer_id));
    const layerIds = [...new Set(visible.map((cluster) => cluster.layer_id))];
    return layerIds.flatMap((layerId) => packTimelineOverlapClusterBadges(
      visible.filter((cluster) => cluster.layer_id === layerId).map((cluster) => ({
        ...cluster,
        x: viewBoxX(cluster.x),
        width: viewBoxX(cluster.width),
      })),
      overviewW(),
    ).map((cluster) => ({ ...cluster, layer_id: layerId })));
  });
  const rulerTicks = createMemo(() => buildTimelineRulerTicks(props.visibleWindow, overviewPixelWidth()));
  const rulerLabelInsetPx = () => Math.max(2, overviewW() * 0.004);
  const rulerLabelX = (ratio: number) => Math.min(
    Math.max(rulerLabelInsetPx(), ratio * overviewW()),
    Math.max(rulerLabelInsetPx(), overviewW() - rulerLabelInsetPx()),
  );
  const rulerLabelAnchor = (ratio: number) => ratio <= 0.04 ? "start" : ratio >= 0.96 ? "end" : "middle";

  const toggleLightingLane = () => setLightingLaneVisible((visible) => !visible);
  const toggleVideoLane = () => setVideoLaneVisible((visible) => !visible);
  const layerSequenceById = createMemo(() => new Map(
    orderedLayers().map((layer, index) => [layer.id, index + 1]),
  ));
  const updateLayer = (layer: TimelineLayerSummary, patch: Partial<TimelineLayerSummary>) => {
    void props.onUpdateLayer({ ...layer, ...patch });
  };
  const layerAccessibleSummary = (layer: TimelineLayerSummary) =>
    `Lane ${layerSequenceById().get(layer.id) ?? 0}: ${layer.label}, ${props.layerItemCounts.get(layer.id) ?? 0} items`;
  const layerActionAccessibleLabel = (layer: TimelineLayerSummary, action: string) =>
    `${action} for lane ${layerSequenceById().get(layer.id) ?? 0}: ${layer.label}`;
  const openLayerMenuFromEvent = (
    event: MouseEvent & { currentTarget: HTMLElement },
    layer: TimelineLayerSummary,
  ) => {
    event.preventDefault();
    event.stopPropagation();
    const bounds = event.currentTarget.getBoundingClientRect();
    const hasPointerCoordinates = event.clientX !== 0 || event.clientY !== 0;
    props.onOpenLayerMenu(layer, hasPointerCoordinates
      ? { x: event.clientX, y: event.clientY }
      : { x: bounds.left, y: bounds.bottom });
  };

  const renderLegacyGutter = () => (
      <div class="timelineOverviewGutter timelineOverviewGutterLegacy" aria-hidden="false">
        <div
          class="timelineLaneGutter"
          data-lane="lighting"
          data-timeline-layer-gutter
          data-timeline-layer-id="0"
          data-timeline-layer-kind="Lighting"
          data-timeline-layer-muted={!lightingLaneVisible() ? "true" : "false"}
          classList={{ laneHidden: !lightingLaneVisible() }}
        >
          <span class="timelineLaneGutterName">Light</span>
          <span class="timelineLaneGutterCount" data-no-localize>{laneCounts().lighting}</span>
          <button
            type="button"
            class="timelineLaneEye"
            aria-pressed={lightingLaneVisible()}
            aria-label="Light lane visibility"
            title="Light lane visibility"
            onClick={toggleLightingLane}
          >
            <svg viewBox="0 0 16 16" aria-hidden="true">
              <path class="timelineLaneEyeShape" d="M1 8 C 3.5 3.5, 12.5 3.5, 15 8 C 12.5 12.5, 3.5 12.5, 1 8 Z" />
              <circle cx="8" cy="8" r="2.4" />
              <Show when={!lightingLaneVisible()}>
                <line class="timelineLaneEyeSlash" x1="2.5" y1="13.5" x2="13.5" y2="2.5" />
              </Show>
            </svg>
          </button>
        </div>
        <div
          class="timelineLaneGutter"
          data-lane="video"
          data-timeline-layer-gutter
          data-timeline-layer-id="1"
          data-timeline-layer-kind="Video"
          data-timeline-layer-muted={!videoLaneVisible() ? "true" : "false"}
          classList={{ laneHidden: !videoLaneVisible() }}
        >
          <span class="timelineLaneGutterName">Video</span>
          <span class="timelineLaneGutterCount" data-no-localize>{laneCounts().video}</span>
          <button
            type="button"
            class="timelineLaneEye"
            aria-pressed={videoLaneVisible()}
            aria-label="Video lane visibility"
            title="Video lane visibility"
            onClick={toggleVideoLane}
          >
            <svg viewBox="0 0 16 16" aria-hidden="true">
              <path class="timelineLaneEyeShape" d="M1 8 C 3.5 3.5, 12.5 3.5, 15 8 C 12.5 12.5, 3.5 12.5, 1 8 Z" />
              <circle cx="8" cy="8" r="2.4" />
              <Show when={!videoLaneVisible()}>
                <line class="timelineLaneEyeSlash" x1="2.5" y1="13.5" x2="13.5" y2="2.5" />
              </Show>
            </svg>
          </button>
        </div>
      </div>
  );

  const renderLayeredGutter = () => (
    <div
      class="timelineOverviewGutter timelineOverviewGutterLayered"
      aria-label="Timeline layer gutters"
      style={{ height: `${sectionLayout().contentHeight}px` }}
    >
      <For each={sectionLayout().sections}>
        {(section) => (
          <section
            class={`timelineLayerSection ${section.kind.toLowerCase()} ${section.collapsed ? "collapsed" : ""}`}
            data-timeline-section-kind={section.kind}
            data-timeline-section-count={section.layers.length}
            style={{
              top: `${section.top}px`,
              height: `${section.totalHeight}px`,
            }}
          >
            <div
              class="timelineLayerSectionHeader"
              data-timeline-section-separator
              style={{ height: `${timelineSectionHeaderHeightPx}px` }}
            >
              <button
                type="button"
                class="timelineLayerSectionCollapse"
                aria-expanded={!section.collapsed}
                aria-label={`${section.collapsed ? "Expand" : "Collapse"} ${section.kind} timeline section`}
                title={`${section.collapsed ? "Expand" : "Collapse"} ${section.kind} timeline section`}
                onClick={() => toggleSection(section.kind)}
              >
                <span aria-hidden="true" data-no-localize>{section.collapsed ? "▸" : "▾"}</span>
              </button>
              <span
                class={`timelineLayerSectionKindIcon ${section.kind.toLowerCase()}`}
                role="img"
                aria-label={`${section.kind} timeline layer kind`}
                title={section.kind}
              >
                <svg viewBox="0 0 16 16" aria-hidden="true">
                  <Show when={section.kind === "Audio"}>
                    <path d="M10.5 2v7.2a2.7 2.7 0 1 1-1.4-2.4V3.4l4.4-1.1v2.1z" />
                  </Show>
                  <Show when={section.kind === "Lighting"}>
                    <path d="M9.2 1 3.7 9h3.5L6.6 15l5.7-8.2H8.8z" />
                  </Show>
                  <Show when={section.kind === "Video"}>
                    <path d="M2 3h9v3l3-2v8l-3-2v3H2z" />
                  </Show>
                </svg>
              </span>
              <Show when={section.kind === "Audio" && section.layers.length > 0}>
                <button
                  type="button"
                  class="timelineAudioClipAdd"
                  data-timeline-add-audio-clip
                  data-timeline-audio-layer-id={(section.layers.find((layer) => !layer.locked) ?? section.layers[0]).id}
                  aria-label="Add Audio Clip"
                  title="Add Audio Clip"
                  onClick={() => {
                    const target = section.layers.find((layer) => !layer.locked) ?? section.layers[0];
                    if (target.locked) {
                      props.onStatus("The Audio section has no unlocked lane for a new clip.");
                      return;
                    }
                    void props.onAddAudioClip(target.id);
                  }}
                >
                  <svg viewBox="0 0 16 16" aria-hidden="true">
                    <path d="M7 2h2v5h5v2H9v5H7V9H2V7h5z" />
                  </svg>
                </button>
              </Show>
            </div>
            <Show when={!section.collapsed}>
              <For each={section.layers}>
                {(layer) => {
                  const cueDropState = () => cueDropStateForLayer(layer.id);
                  const markerDropState = () => markerDropStateForLayer(layer.id);
                  const row = () => layerRowById().get(layer.id);
                  const expanded = () => expandedLayerIds().has(layer.id);
                  return (
                    <div
                      class="timelineLaneGutter timelineUserLaneGutter"
                      classList={{
                        layerMuted: layer.muted,
                        layerLocked: layer.locked,
                        layerSolo: layer.solo,
                        dropValid: cueDropState() === "valid" || markerDropState() === "valid",
                        dropRejected: cueDropState() === "rejected" || markerDropState() === "rejected",
                      }}
                      data-timeline-layer-gutter
                      data-timeline-layer-id={layer.id}
                      data-timeline-layer-kind={layer.kind}
                      data-timeline-layer-muted={layer.muted ? "true" : "false"}
                      data-timeline-layer-locked={layer.locked ? "true" : "false"}
                      data-timeline-layer-solo={layer.solo ? "true" : "false"}
                      data-timeline-layer-expanded={expanded() ? "true" : "false"}
                      aria-label={layerAccessibleSummary(layer)}
                      title={layerAccessibleSummary(layer)}
                      onContextMenu={(event) => openLayerMenuFromEvent(event, layer)}
                      style={{
                        top: `${(row()?.top ?? section.top + timelineSectionHeaderHeightPx) - section.top}px`,
                        height: `${row()?.height ?? timelineUserLaneHeightPx}px`,
                      }}
                    >
                      <button
                        type="button"
                        class="timelineLaneNumber timelineLaneMenuTrigger tabularNums"
                        data-timeline-lane-number
                        data-timeline-layer-menu-trigger
                        aria-haspopup="dialog"
                        aria-label={layerActionAccessibleLabel(layer, "Open actions")}
                        title={layerAccessibleSummary(layer)}
                        onClick={(event) => openLayerMenuFromEvent(event, layer)}
                        onKeyDown={(event) => {
                          const opensPrimary = event.key === "Enter" || event.key === " ";
                          const opensContext = event.key === "ContextMenu" || (event.shiftKey && event.key === "F10");
                          if (!opensPrimary && !opensContext) return;
                          event.preventDefault();
                          event.stopPropagation();
                          const bounds = event.currentTarget.getBoundingClientRect();
                          props.onOpenLayerMenu(layer, { x: bounds.left, y: bounds.bottom });
                        }}
                        data-no-localize
                      >
                        {layerSequenceById().get(layer.id) ?? 0}
                      </button>
                      <button
                        type="button"
                        class="timelineLaneDetailsToggle"
                        data-timeline-layer-details-toggle
                        data-timeline-layer-expand-toggle
                        aria-expanded={expanded()}
                        aria-label={layerActionAccessibleLabel(layer, expanded() ? "Collapse details" : "Expand details")}
                        title={layerActionAccessibleLabel(layer, expanded() ? "Collapse details" : "Expand details")}
                        onClick={() => toggleLayerExpanded(layer.id)}
                      >
                        <svg viewBox="0 0 16 16" aria-hidden="true">
                          <path d={expanded() ? "M3 5.5 8 10.5l5-5" : "M5.5 3 10.5 8l-5 5"} />
                        </svg>
                      </button>
                      <button
                        type="button"
                        class="timelineLaneEye"
                        data-timeline-layer-mute-toggle
                        aria-pressed={!layer.muted}
                        aria-label={layerActionAccessibleLabel(layer, layer.muted ? "Unmute" : "Mute")}
                        title={layerActionAccessibleLabel(layer, layer.muted ? "Unmute" : "Mute")}
                        onClick={() => updateLayer(layer, { muted: !layer.muted })}
                      >
                        <svg viewBox="0 0 16 16" aria-hidden="true">
                          <path class="timelineLaneEyeShape" d="M1 8 C 3.5 3.5, 12.5 3.5, 15 8 C 12.5 12.5, 3.5 12.5, 1 8 Z" />
                          <circle cx="8" cy="8" r="2.4" />
                          <Show when={layer.muted}>
                            <line class="timelineLaneEyeSlash" x1="2.5" y1="13.5" x2="13.5" y2="2.5" />
                          </Show>
                        </svg>
                      </button>
                      <button
                        type="button"
                        class="timelineLaneLock"
                        data-timeline-layer-lock-toggle
                        aria-pressed={layer.locked}
                        aria-label={layerActionAccessibleLabel(layer, layer.locked ? "Unlock" : "Lock")}
                        title={layerActionAccessibleLabel(layer, layer.locked ? "Unlock" : "Lock")}
                        onClick={() => updateLayer(layer, { locked: !layer.locked })}
                      >
                        <svg viewBox="0 0 16 16" aria-hidden="true">
                          <Show
                            when={layer.locked}
                            fallback={<path d="M5 7V5a3 3 0 0 1 5.7-1.3L9.2 4.4A1.4 1.4 0 0 0 6.5 5v2H13v7H3V7z" />}
                          >
                            <path d="M5 7V5a3 3 0 0 1 6 0v2h2v7H3V7zm1.5 0h3V5a1.5 1.5 0 0 0-3 0z" />
                          </Show>
                        </svg>
                      </button>
                      <Show when={expanded()}>
                        <div class="timelineLaneExpandedDetails" data-timeline-layer-details>
                          <span class="timelineLaneExpandedName" data-no-localize>{layer.label}</span>
                          <span class="timelineLaneExpandedCount tabularNums" data-no-localize>
                            {props.layerItemCounts.get(layer.id) ?? 0}
                          </span>
                        </div>
                      </Show>
                    </div>
                  );
                }}
              </For>
            </Show>
          </section>
        )}
      </For>
    </div>
  );

  const renderOverviewCanvas = () => (
      <svg
      class={`timelineOverview ${props.executionLive ? "executingLive" : ""}`}
      ref={(element) => { overviewElement = element; }}
      viewBox={`0 0 ${overviewW()} ${canvasH()}`}
      data-viewbox-width={overviewW()}
      data-viewbox-height={canvasH()}
      data-visible-start-ms={props.visibleWindow.start_ms}
      data-visible-end-ms={props.visibleWindow.end_ms}
      data-timeline-gesture-active={
        markerDrag() || eventResizeDrag() || eventFadeDrag() || placementDrag() || audioClipDrag() ||
        rangeDrag() || keyframeDrag() || canvasPanDrag()
          ? "true"
          : "false"
      }
      style={!props.legacyMode ? { height: `${sectionLayout().contentHeight}px` } : undefined}
      preserveAspectRatio="none"
      role="group"
      aria-label="Timeline overview"
      onClick={seekFromPointer}
      onDblClick={placeArmedCueAtDoubleClick}
      onPointerDown={beginCanvasPointer}
      onPointerMove={moveCanvasPointer}
      onPointerUp={(event) => finishCanvasPointer(event, false)}
      onPointerCancel={(event) => finishCanvasPointer(event, true)}
    >
      <rect class="timelineOverviewBg" x="0" y="0" width={overviewW()} height={canvasH()} />
      <Show
        when={!props.legacyMode}
        fallback={
          <>
            <rect
              class="timelineLayerRowBackground lighting"
              x="0"
              y="0"
              width={overviewW()}
              height={legacyLaneHeightPx()}
              data-timeline-layer-id="0"
              data-timeline-layer-kind="Lighting"
            />
            <rect
              class="timelineLayerRowBackground video"
              x="0"
              y={legacyLaneHeightPx()}
              width={overviewW()}
              height={legacyLaneHeightPx()}
              data-timeline-layer-id="1"
              data-timeline-layer-kind="Video"
            />
            <line class="timelineLaneDivider" x1="0" y1={legacyLaneHeightPx()} x2={overviewW()} y2={legacyLaneHeightPx()} />
          </>
        }
      >
        <For each={sectionLayout().sections}>
          {(section) => (
            <rect
              class={`timelineLayerSectionBackground ${section.kind.toLowerCase()}`}
              x="0"
              y={section.top}
              width={overviewW()}
              height={section.height}
            />
          )}
        </For>
        <For each={sectionLayout().laneRows}>
          {(row) => {
            const cueDropState = () => cueDropStateForLayer(row.layer.id);
            const markerDropState = () => markerDropStateForLayer(row.layer.id);
            return (
              <rect
                class={`timelineLayerRowBackground ${row.layer.kind.toLowerCase()}`}
                classList={{
                  layerMuted: row.layer.muted,
                  layerLocked: row.layer.locked,
                  layerSolo: row.layer.solo,
                  dropValid: cueDropState() === "valid" || markerDropState() === "valid",
                  dropRejected: cueDropState() === "rejected" || markerDropState() === "rejected",
                }}
                x="0"
                y={row.top}
                width={overviewW()}
                height={row.height}
                data-timeline-layer-id={row.layer.id}
                data-timeline-layer-kind={row.layer.kind}
                data-timeline-layer-muted={row.layer.muted ? "true" : "false"}
                data-timeline-layer-locked={row.layer.locked ? "true" : "false"}
                data-timeline-layer-expanded={expandedLayerIds().has(row.layer.id) ? "true" : "false"}
              />
            );
          }}
        </For>
        <For each={sectionLayout().sections.slice(1)}>
          {(section) => (
            <line class="timelineSectionDivider" x1="0" y1={section.top} x2={overviewW()} y2={section.top} />
          )}
        </For>
        <For each={sectionLayout().laneRows}>
          {(row) => (
            <line class="timelineLaneDivider" x1="0" y1={row.top + row.height} x2={overviewW()} y2={row.top + row.height} />
          )}
        </For>
      </Show>
      <g class="timelineRuler" aria-hidden="true">
        <For each={rulerTicks()}>
          {(tick) => (
            <g data-timeline-ruler-ms={tick.time_ms}>
              <line
                class={tick.major ? "major" : ""}
                x1={tick.ratio * overviewW()}
                x2={tick.ratio * overviewW()}
                y1={rulerLineTopPx()}
                y2={rulerLineBottomPx()}
              />
              <text
                x={rulerLabelX(tick.ratio)}
                y={rulerLabelBaselineY()}
                text-anchor={rulerLabelAnchor(tick.ratio)}
              >
                {tick.label}
              </text>
            </g>
          )}
        </For>
      </g>
      <For each={stableAutomationRanges().filter((range) => props.legacyMode || Boolean(automationLayerRow(range)))}>
        {(range) => (
          <g
            class={[
              "timelineAutomationRange",
              range.track === "Lighting" ? "lighting" : "video",
              range.enabled ? "" : "disabled",
              props.selectedRangeId === range.id ? "selected" : "",
              rangeDrag()?.rangeId === range.id ? "dragging" : "",
              props.legacyMode && !laneVisible(range.track) ? "laneDimmed" : "",
            ].filter(Boolean).join(" ")}
            onPointerDown={(pointerEvent) => beginRangeDrag(pointerEvent, range)}
            onPointerMove={moveRangeDrag}
            onPointerUp={endRangeDrag}
            onPointerCancel={cancelRangeDrag}
            onClick={(pointerEvent) => {
              pointerEvent.stopPropagation();
              if (suppressClickRangeId() === range.id) {
                setSuppressClickRangeId(null);
                return;
              }
              props.onSelectAutomationRange(range);
              props.onSeekTime(range.start_ms);
            }}
          >
            <rect
              x={viewBoxX(automationRangeX(range))}
              y={automationBarTopPx(range)}
              width={viewBoxX(automationRangeWidth(range))}
              height={automationBarHeightPx(range)}
              rx="1.6"
            />
            <For each={range.keyframes}>
              {(keyframe) => (
                <g
                  class={[
                    "timelineAutomationKeyframeGroup",
                    keyframeDrag()?.rangeId === range.id && keyframeDrag()?.keyframeIndex === keyframe.keyframe_index
                      ? "dragging"
                      : "",
                  ].filter(Boolean).join(" ")}
                  onPointerDown={(pointerEvent) => beginKeyframeDrag(pointerEvent, range, keyframe.keyframe_index)}
                  onPointerMove={moveKeyframeDrag}
                  onPointerUp={endKeyframeDrag}
                  onPointerCancel={cancelKeyframeDrag}
                  onClick={(pointerEvent) => {
                    pointerEvent.stopPropagation();
                    if (suppressClickRangeId() === range.id) {
                      setSuppressClickRangeId(null);
                      return;
                    }
                    props.onSelectAutomationRange(range);
                    props.onSeekTime(keyframe.time_ms);
                  }}
                >
                  <circle
                    class="timelineAutomationKeyframeHit"
                    cx={viewBoxX(automationKeyframeX(range, keyframe.keyframe_index, keyframe.time_ms))}
                    cy={automationBarTopPx(range) + automationBarHeightPx(range) / 2}
                    r="3.2"
                  />
                  <circle
                    class="timelineAutomationKeyframe"
                    cx={viewBoxX(automationKeyframeX(range, keyframe.keyframe_index, keyframe.time_ms))}
                    cy={automationBarTopPx(range) + automationBarHeightPx(range) / 2}
                    r="1.1"
                  />
                </g>
              )}
            </For>
            <rect
              class="timelineAutomationHandle start"
              x={viewBoxX(automationRangeX(range))}
              y={automationBarTopPx(range) - 0.8}
              width="2.1"
              height={automationBarHeightPx(range) + 1.6}
              rx="0.6"
              onPointerDown={(pointerEvent) => beginRangeResize(pointerEvent, range, "start")}
            />
            <rect
              class="timelineAutomationHandle end"
              x={Math.max(0, viewBoxX(automationRangeX(range) + automationRangeWidth(range)) - 2.1)}
              y={automationBarTopPx(range) - 0.8}
              width="2.1"
              height={automationBarHeightPx(range) + 1.6}
              rx="0.6"
              onPointerDown={(pointerEvent) => beginRangeResize(pointerEvent, range, "end")}
            />
            <title>
              {range.label} / {range.track} / {range.start_ms}-{range.end_ms} ms
            </title>
          </g>
        )}
      </For>
      <For each={renderedAudioClips()}>
        {(clip) => {
          const preview = () => audioClipPreview(clip);
          const selected = () => props.selectedAudioClipId === clip.id;
          return (
            <g
              class="timelineAudioClip"
              classList={{
                selected: selected(),
                dragging: audioClipDrag()?.clipId === clip.id,
                layerMuted: layerById().get(clip.layer_id)?.muted ?? false,
              }}
              data-timeline-audio-clip-id={clip.id}
              data-timeline-layer-id={preview().layer_id}
              data-timeline-layer-kind="Audio"
              data-timeline-audio-path={clip.path}
              data-timeline-audio-start-ms={preview().start_ms}
              data-timeline-audio-offset-ms={preview().offset_ms}
              data-timeline-audio-duration-ms={preview().duration_ms}
              data-timeline-audio-gain={preview().gain}
              data-timeline-audio-fade-in-ms={preview().fade_in_ms}
              data-timeline-audio-fade-out-ms={preview().fade_out_ms}
              style={{
                "--identity": identityCssColor(cueIdentityHue(stableTimelineAudioPathHash(clip.path)), "fill"),
                "--identity-band": identityCssColor(cueIdentityHue(stableTimelineAudioPathHash(clip.path)), "band"),
              }}
              role="button"
              tabindex={audioClipTabStopId() === clip.id ? 0 : -1}
              aria-label={`Audio Clip ${timelineAudioClipName(clip.path)}, starts ${preview().start_ms} milliseconds, duration ${preview().duration_ms} milliseconds`}
              transform={`translate(${viewBoxX(timelineTimeToVisibleRawRatio(preview().start_ms, props.visibleWindow) * 100)} ${audioClipCenterYPx(clip)})`}
              onPointerDown={(pointerEvent) => beginAudioClipGesture(pointerEvent, clip)}
              onPointerMove={moveAudioClipGesture}
              onPointerUp={(pointerEvent) => finishAudioClipGesture(pointerEvent, false)}
              onPointerCancel={(pointerEvent) => finishAudioClipGesture(pointerEvent, true)}
              onClick={(pointerEvent) => {
                pointerEvent.preventDefault();
                pointerEvent.stopPropagation();
                props.onSelectAudioClip(clip.id);
                props.onSeekTime(preview().start_ms);
              }}
              onKeyDown={(keyboardEvent) => {
                if (keyboardEvent.key !== "Enter" && keyboardEvent.key !== " ") return;
                keyboardEvent.preventDefault();
                props.onSelectAudioClip(clip.id);
                props.onSeekTime(preview().start_ms);
              }}
            >
              <Show when={audioClipDrag()?.clipId === clip.id}>
                <rect
                  class="timelineAudioClipGhost"
                  data-timeline-audio-drag-ghost
                  x={(clip.start_ms - preview().start_ms) / timelineVisibleWindowSpanMs(props.visibleWindow) * overviewW()}
                  y={-blockHeightPx() / 2}
                  width={Math.max(0.8, clip.duration_ms / timelineVisibleWindowSpanMs(props.visibleWindow) * overviewW())}
                  height={blockHeightPx()}
                  rx="1.6"
                />
              </Show>
              <rect
                class="timelineAudioClipBody"
                x="0"
                y={-blockHeightPx() / 2}
                width={audioClipPixelWidth(clip)}
                height={blockHeightPx()}
                rx="1.6"
              />
              <rect
                class="timelineAudioClipIdentityBand"
                x="0"
                y={-blockHeightPx() / 2}
                width={audioClipPixelWidth(clip)}
                height={blockUpperBandHeightPx()}
              />
              <Show when={audioClipFadeRampPoints(clip, "in")}>
                {(points) => (
                  <polygon
                    class="timelineAudioClipFade in"
                    data-timeline-audio-fade-ramp="in"
                    points={points()}
                  />
                )}
              </Show>
              <Show when={audioClipFadeRampPoints(clip, "out")}>
                {(points) => (
                  <polygon
                    class="timelineAudioClipFade out"
                    data-timeline-audio-fade-ramp="out"
                    points={points()}
                  />
                )}
              </Show>
              <polyline
                class="timelineAudioClipWaveform"
                data-timeline-audio-waveform
                data-timeline-audio-waveform-node-count="1"
                points={audioClipWaveformPoints(clip)}
              />
              <text
                class="timelineAudioClipLabel"
                x={nameInsetPx}
                y={-blockHeightPx() / 2 + blockUpperBandHeightPx() / 2}
                dominant-baseline="central"
                data-full-label={timelineAudioClipName(clip.path)}
              >
                {audioClipVisibleName(clip)}
              </text>
              <Show when={audioClipShowsDuration(clip)}>
                <text
                  class="timelineAudioClipDuration"
                  x={nameInsetPx}
                  y={-blockHeightPx() / 2 + blockUpperBandHeightPx() + (blockHeightPx() - blockUpperBandHeightPx()) / 2}
                  dominant-baseline="central"
                  data-no-localize
                >
                  {formatCompactClock(preview().duration_ms)}
                </text>
              </Show>
              <Show when={audioClipLiveStamp(clip)}>
                {(stamp) => (
                  <text class="timelineSceneBlockLiveStamp" data-timeline-audio-live-stamp x="4" y={-blockHeightPx() / 2 - 4} data-no-localize>
                    {stamp()}
                  </text>
                )}
              </Show>
              <Show when={selected() || audioClipDrag()?.clipId === clip.id}>
                <rect
                  class="timelineAudioClipResizeHandle start"
                  data-timeline-audio-resize="start"
                  data-timeline-zone-band-px={TIMELINE_BLOCK_UPPER_BAND_PX}
                  data-timeline-zone-edge-px={TIMELINE_BLOCK_STRETCH_EDGE_PX}
                  x="0"
                  y={-blockHeightPx() / 2}
                  width={TIMELINE_BLOCK_STRETCH_EDGE_PX}
                  height={TIMELINE_BLOCK_UPPER_BAND_PX}
                  aria-label="Resize Audio Clip start"
                />
                <rect
                  class="timelineAudioClipResizeHandle end"
                  data-timeline-audio-resize="end"
                  data-timeline-zone-band-px={TIMELINE_BLOCK_UPPER_BAND_PX}
                  data-timeline-zone-edge-px={TIMELINE_BLOCK_STRETCH_EDGE_PX}
                  x={Math.max(0, audioClipPixelWidth(clip) - TIMELINE_BLOCK_STRETCH_EDGE_PX)}
                  y={-blockHeightPx() / 2}
                  width={TIMELINE_BLOCK_STRETCH_EDGE_PX}
                  height={TIMELINE_BLOCK_UPPER_BAND_PX}
                  aria-label="Resize Audio Clip end"
                />
                <rect
                  class="timelineAudioClipFadeHandle in"
                  data-timeline-audio-fade="in"
                  data-timeline-zone-band-px={TIMELINE_BLOCK_UPPER_BAND_PX}
                  data-timeline-zone-edge-px={TIMELINE_BLOCK_FADE_EDGE_PX}
                  x="0"
                  y="0"
                  width={TIMELINE_BLOCK_FADE_EDGE_PX}
                  height={TIMELINE_BLOCK_UPPER_BAND_PX}
                  aria-label="Adjust Audio Clip Fade In"
                />
                <rect
                  class="timelineAudioClipFadeHandle out"
                  data-timeline-audio-fade="out"
                  data-timeline-zone-band-px={TIMELINE_BLOCK_UPPER_BAND_PX}
                  data-timeline-zone-edge-px={TIMELINE_BLOCK_FADE_EDGE_PX}
                  x={Math.max(0, audioClipPixelWidth(clip) - TIMELINE_BLOCK_FADE_EDGE_PX)}
                  y="0"
                  width={TIMELINE_BLOCK_FADE_EDGE_PX}
                  height={TIMELINE_BLOCK_UPPER_BAND_PX}
                  aria-label="Adjust Audio Clip Fade Out"
                />
              </Show>
              <title>{`${timelineAudioClipName(clip.path)} / Audio / ${preview().start_ms} ms / ${preview().duration_ms} ms / gain ${preview().gain.toFixed(2)}`}</title>
            </g>
          );
        }}
      </For>
      <For each={renderedEvents()}>
        {(event) => (
          <g
            class={[
              "timelineMarker",
              event.duration_ms > 0 ? "sceneBlock" : "pointEvent",
              event.track === "Lighting" ? "lighting" : "video",
              underPlayheadEventIds().has(event.id) ? "underPlayhead" : "",
              props.selectedEventId === event.id ? "selected" : "",
              markerDrag()?.eventId === event.id ? "dragging" : "",
              eventResizeDrag()?.eventId === event.id ? "resizing" : "",
              eventFadeDrag()?.eventId === event.id ? "fading" : "",
              props.legacyMode && !laneVisible(event.track) ? "laneDimmed" : "",
              !props.legacyMode && layerById().get(event.layer_id)?.muted ? "layerMuted" : "",
            ].filter(Boolean).join(" ")}
            data-timeline-event-id={event.id}
            data-super-scene={event.is_super_scene ? "true" : undefined}
            data-timeline-layer-id={eventPreviewLayerId(event)}
            data-timeline-layer-kind={layerById().get(eventPreviewLayerId(event))?.kind ?? event.track}
            data-timeline-layer-muted={layerById().get(event.layer_id)?.muted ? "true" : "false"}
            data-timeline-start-ms={event.time_ms}
            data-timeline-loop-count={event.loop_count}
            data-timeline-conform={event.conform_to_tempo ? "true" : "false"}
            data-timeline-rate={event.rate ?? undefined}
            data-timeline-block-layout={event.duration_ms > 0 ? "solid-two-line" : undefined}
            data-timeline-fade-in-ms={eventPreviewFadeMs(event, "in")}
            data-timeline-fade-out-ms={eventPreviewFadeMs(event, "out")}
            data-timeline-preview-rate={eventPreviewRate(event) ?? undefined}
            style={{
              "--identity": cueIdentityCss(
                event.cue_id,
                props.cueIdentities?.[event.cue_id]?.color,
                "fill",
                props.cueIdentities?.[event.cue_id]?.groupId,
                props.cueIdentities?.[event.cue_id]?.groupColor,
              ),
              "--identity-band": cueIdentityCss(
                event.cue_id,
                props.cueIdentities?.[event.cue_id]?.color,
                "band",
                props.cueIdentities?.[event.cue_id]?.groupId,
                props.cueIdentities?.[event.cue_id]?.groupColor,
              ),
              "--identity-text": cueIdentityCss(
                event.cue_id,
                props.cueIdentities?.[event.cue_id]?.color,
                "text",
                props.cueIdentities?.[event.cue_id]?.groupId,
                props.cueIdentities?.[event.cue_id]?.groupColor,
              ),
            }}
            data-timeline-preview-start-ms={eventPreviewStartMs(event)}
            data-timeline-preview-end-ms={eventPreviewEndMs(event)}
            role="button"
            tabindex={markerTabStopId() === event.id ? 0 : -1}
            aria-label={props.markerAriaLabel(event)}
            transform={`translate(${viewBoxX(timelineTimeToVisibleRawRatio(eventPreviewStartMs(event), props.visibleWindow) * 100)} ${blockCenterYPx({
              layer_id: eventPreviewLayerId(event),
              track: event.track,
            })})`}
            onPointerDown={(pointerEvent) => beginBlockGesture(pointerEvent, event)}
            onPointerMove={moveBlockGesture}
            onPointerUp={(pointerEvent) => finishBlockGesture(pointerEvent, false)}
            onPointerCancel={(pointerEvent) => finishBlockGesture(pointerEvent, true)}
            onClick={(pointerEvent) => {
              pointerEvent.stopPropagation();
              if (suppressClickEventId() === event.id) {
                setSuppressClickEventId(null);
                return;
              }
              props.onSelectEvent(event.id, true);
              props.onSeekTime(event.time_ms);
            }}
            onDblClick={(pointerEvent) => {
              if (!event.is_super_scene) return;
              pointerEvent.preventDefault();
              pointerEvent.stopPropagation();
              props.onOpenSuperScene(event.cue_id);
            }}
            onKeyDown={(keyboardEvent) => {
              if (keyboardEvent.key === "ArrowRight" || keyboardEvent.key === "ArrowDown") {
                keyboardEvent.preventDefault();
                keyboardEvent.stopPropagation();
                moveMarkerKeyboardFocus(event.id, 1);
                return;
              }
              if (keyboardEvent.key === "ArrowLeft" || keyboardEvent.key === "ArrowUp") {
                keyboardEvent.preventDefault();
                keyboardEvent.stopPropagation();
                moveMarkerKeyboardFocus(event.id, -1);
                return;
              }
              if (keyboardEvent.key === "Home" || keyboardEvent.key === "End") {
                keyboardEvent.preventDefault();
                keyboardEvent.stopPropagation();
                moveMarkerKeyboardFocus(event.id, keyboardEvent.key === "Home" ? "first" : "last");
                return;
              }
              if (keyboardEvent.key !== "Enter" && keyboardEvent.key !== " ") return;
              keyboardEvent.preventDefault();
              keyboardEvent.stopPropagation();
              props.onSelectEvent(event.id, true);
              props.onSeekTime(event.time_ms);
            }}
          >
            <Show
              when={event.duration_ms > 0}
              fallback={
                <>
                  <line
                    class="timelinePointStem"
                    x1="0"
                    y1={-blockHeightPx() / 2}
                    x2="0"
                    y2={blockHeightPx() / 2}
                  />
                  <polygon
                    class="timelinePointFlag"
                    points={`0,${-blockHeightPx() / 2} ${Math.max(5, blockHeightPx() * 0.5)},${-blockHeightPx() / 2 + blockHeightPx() * 0.22} 0,${-blockHeightPx() / 2 + blockHeightPx() * 0.44}`}
                  />
                </>
              }
            >
              <Show when={activeDragStamp(event)}>
                <rect
                  class="timelineSceneBlockGhost"
                  data-timeline-drag-ghost
                  x={(event.time_ms - eventPreviewStartMs(event)) / timelineVisibleWindowSpanMs(props.visibleWindow) * overviewW()}
                  y={-blockHeightPx() / 2}
                  width={Math.max(0.8, event.total_duration_ms / timelineVisibleWindowSpanMs(props.visibleWindow) * overviewW())}
                  height={blockHeightPx()}
                  rx="1.6"
                />
              </Show>
              <Show
                when={sceneBlockHasReadableBody(event)}
                fallback={
                  <>
                    <rect
                      class="timelineSceneBlockHit"
                      x="0"
                      y={-blockHeightPx() / 2}
                      width={Math.max(sceneBlockPixelWidth(event), 6)}
                      height={blockHeightPx()}
                      rx="1"
                    />
                    <rect
                      class="timelineSceneBlockBody"
                      x="0"
                      y={-blockHeightPx() / 2}
                      width={sceneBlockPixelWidth(event)}
                      height={blockHeightPx()}
                      rx="1"
                    />
                  </>
                }
              >
                <rect
                  class="timelineSceneBlockBody"
                  x="0"
                  y={-blockHeightPx() / 2}
                  width={sceneBlockPixelWidth(event)}
                  height={blockHeightPx()}
                  rx="1"
                />
                <Show when={sceneBlockFadeRampPoints(event, "in")}>
                  {(points) => (
                    <polygon
                      class="timelineSceneBlockFade in"
                      data-timeline-block-fade-ramp="in"
                      points={points()}
                    />
                  )}
                </Show>
                <Show when={sceneBlockFadeRampPoints(event, "out")}>
                  {(points) => (
                    <polygon
                      class="timelineSceneBlockFade out"
                      data-timeline-block-fade-ramp="out"
                      points={points()}
                    />
                  )}
                </Show>
                <text
                  class="timelineSceneBlockLabel"
                  x={nameInsetPx}
                  y={-blockHeightPx() / 2 + blockUpperBandHeightPx() / 2}
                  dominant-baseline="central"
                  data-full-label={event.cue_label}
                >
                  {sceneBlockName(event)}
                </text>
                <Show when={event.is_super_scene}>
                  <text
                    class="timelineSuperSceneLink"
                    data-super-scene-source-link
                    x={Math.max(nameInsetPx, sceneBlockPixelWidth(event) - 12)}
                    y={-blockHeightPx() / 2 + blockUpperBandHeightPx() / 2}
                    dominant-baseline="central"
                    data-no-localize
                  >
                    ↳
                  </text>
                </Show>
                <Show when={sceneBlockShowsDuration(event)}>
                  <text
                    class="timelineSceneBlockDuration"
                    x={nameInsetPx}
                    y={-blockHeightPx() / 2 + blockUpperBandHeightPx() + (blockHeightPx() - blockUpperBandHeightPx()) / 2}
                    dominant-baseline="central"
                    data-no-localize
                  >
                    {sceneBlockDurationStamp(event)}
                  </text>
                </Show>
              </Show>
              <Show when={activeDragStamp(event)}>
                {(stamp) => (
                  <text
                    class="timelineSceneBlockLiveStamp"
                    data-timeline-live-stamp
                    x="4"
                    y={-blockHeightPx() / 2 - 4}
                    data-no-localize
                  >
                    {stamp()}
                  </text>
                )}
              </Show>
              <Show when={props.selectedEventId === event.id || eventResizeDrag()?.eventId === event.id || eventFadeDrag()?.eventId === event.id}>
                <rect
                  class="timelineSceneBlockResizeHandle start"
                  data-timeline-scene-block-resize="start"
                  data-timeline-zone-band-px={TIMELINE_BLOCK_UPPER_BAND_PX}
                  data-timeline-zone-edge-px={TIMELINE_BLOCK_STRETCH_EDGE_PX}
                  x="0"
                  y={-blockHeightPx() / 2}
                  width={TIMELINE_BLOCK_STRETCH_EDGE_PX}
                  height={TIMELINE_BLOCK_UPPER_BAND_PX}
                  aria-label="Resize Scene Block start"
                  onPointerDown={(pointerEvent) => beginEventResize(pointerEvent, event, "start")}
                  onPointerMove={moveEventResize}
                  onPointerUp={endEventResize}
                  onPointerCancel={cancelEventResize}
                />
                <rect
                  class="timelineSceneBlockResizeHandle end"
                  data-timeline-scene-block-resize="end"
                  data-timeline-zone-band-px={TIMELINE_BLOCK_UPPER_BAND_PX}
                  data-timeline-zone-edge-px={TIMELINE_BLOCK_STRETCH_EDGE_PX}
                  x={Math.max(0, sceneBlockPixelWidth(event) - TIMELINE_BLOCK_STRETCH_EDGE_PX)}
                  y={-blockHeightPx() / 2}
                  width={TIMELINE_BLOCK_STRETCH_EDGE_PX}
                  height={TIMELINE_BLOCK_UPPER_BAND_PX}
                  aria-label="Resize Scene Block end"
                  onPointerDown={(pointerEvent) => beginEventResize(pointerEvent, event, "end")}
                  onPointerMove={moveEventResize}
                  onPointerUp={endEventResize}
                  onPointerCancel={cancelEventResize}
                />
                <rect
                  class="timelineSceneBlockFadeHandle in"
                  data-timeline-scene-block-fade="in"
                  data-timeline-zone-band-px={TIMELINE_BLOCK_UPPER_BAND_PX}
                  data-timeline-zone-edge-px={TIMELINE_BLOCK_FADE_EDGE_PX}
                  x="0"
                  y="0"
                  width={TIMELINE_BLOCK_FADE_EDGE_PX}
                  height={TIMELINE_BLOCK_UPPER_BAND_PX}
                  aria-label="Adjust Scene Block Fade In"
                  onPointerDown={(pointerEvent) => beginEventFade(pointerEvent, event, "in")}
                  onPointerMove={moveEventFade}
                  onPointerUp={endEventFade}
                  onPointerCancel={cancelEventFade}
                />
                <rect
                  class="timelineSceneBlockFadeHandle out"
                  data-timeline-scene-block-fade="out"
                  data-timeline-zone-band-px={TIMELINE_BLOCK_UPPER_BAND_PX}
                  data-timeline-zone-edge-px={TIMELINE_BLOCK_FADE_EDGE_PX}
                  x={Math.max(0, sceneBlockPixelWidth(event) - TIMELINE_BLOCK_FADE_EDGE_PX)}
                  y="0"
                  width={TIMELINE_BLOCK_FADE_EDGE_PX}
                  height={TIMELINE_BLOCK_UPPER_BAND_PX}
                  aria-label="Adjust Scene Block Fade Out"
                  onPointerDown={(pointerEvent) => beginEventFade(pointerEvent, event, "out")}
                  onPointerMove={moveEventFade}
                  onPointerUp={endEventFade}
                  onPointerCancel={cancelEventFade}
                />
              </Show>
            </Show>
            <title>
              {event.duration_ms > 0
                ? event.conform_to_tempo
                  ? `${event.cue_label}${timelineConformRateBadge(event) ? ` ${timelineConformRateBadge(event)}` : ""} / ${event.track} / ${event.time_ms} ms / ${event.duration_ms} ms window / ${event.loop_count} ${event.loop_fill ? "fill loops" : "tempo iterations"}`
                  : `${event.cue_label} / ${event.track} / ${event.time_ms} ms / ${event.duration_ms} ms x ${event.loop_count}`
                : `${event.cue_label} / ${event.track} / ${event.time_ms} ms / legacy point`}
            </title>
          </g>
        )}
      </For>
      <For each={packedOverlapClusters()}>
        {(cluster) => {
          const timeSpanLabel = `${cluster.start_ms} to ${cluster.end_ms} ms`;
          const accessibleLabel = cluster.aggregated
            ? `${cluster.label}; ${timeSpanLabel}; inspect ${cluster.count} blocks across ${cluster.group_count} overlap groups`
            : `${cluster.label}; ${timeSpanLabel}; inspect ${cluster.count} overlapping blocks`;
          const activate = () => {
            const startedAt = performance.now();
            props.onInspectOverlapCluster(cluster);
            // Diagnostic only: synchronous handler and microtask state activation.
            // These are state timing metrics, not frame timing metrics.
            overviewElement?.setAttribute(
              "data-overlap-state-handler-ms",
              String(performance.now() - startedAt),
            );
            overviewElement?.setAttribute("data-overlap-state-track", cluster.track);
            queueMicrotask(() => {
              overviewElement?.setAttribute(
                "data-overlap-state-microtask-ms",
                String(performance.now() - startedAt),
              );
            });
          };
          return (
            <g
              class={`timelineOverlapCluster ${cluster.track === "Lighting" ? "lighting" : "video"}`}
              role="button"
              tabindex={0}
              aria-label={accessibleLabel}
              data-overlap-track={cluster.track}
              data-overlap-count={cluster.count}
              data-overlap-start-ms={cluster.start_ms}
              data-overlap-end-ms={cluster.end_ms}
              data-overlap-members={cluster.member_ids.join(",")}
              data-overlap-cluster-ids={cluster.source_cluster_ids.join(",")}
              data-overlap-group-count={cluster.group_count}
              data-overlap-aggregated={cluster.aggregated ? "true" : "false"}
              data-timeline-layer-id={cluster.layer_id}
              transform={`translate(${cluster.x} ${clusterBadgeTopPx(cluster)})`}
              onClick={(event) => {
                event.stopPropagation();
                activate();
              }}
              onKeyDown={(event) => {
                if (event.key !== "Enter" && event.key !== " ") return;
                event.preventDefault();
                event.stopPropagation();
                activate();
              }}
            >
              <rect width={clusterBadgeWidthPx} height={clusterBadgeHeightPx()} rx="2" />
              <text x={clusterBadgeWidthPx / 2} y={clusterBadgeHeightPx() / 2} text-anchor="middle" dominant-baseline="central">×{cluster.count}</text>
              <title>{accessibleLabel}</title>
            </g>
          );
        }}
      </For>
      <Show when={placementPreview()}>
        {(placement) => (
          <g
            class="timelinePlacementPreview"
            data-timeline-placement-preview
            data-timeline-start-ms={placement().startMs}
            data-timeline-duration-ms={placement().durationMs}
            transform={`translate(${timelineTimeToVisibleRawRatio(placement().startMs, props.visibleWindow) * overviewW()} ${blockCenterYPx({
              layer_id: placement().layerId,
              track: "Lighting",
            })})`}
            aria-hidden="true"
          >
            <rect
              x="0"
              y={-blockHeightPx() / 2}
              width={Math.max(1, placement().durationMs / timelineVisibleWindowSpanMs(props.visibleWindow) * overviewW())}
              height={blockHeightPx()}
              rx="1.6"
            />
            <text x="4" y="0" dominant-baseline="central" data-no-localize>
              {placement().durationMs} ms
            </text>
          </g>
        )}
      </Show>
      <Show when={props.playheadX >= 0 && props.playheadX <= 100}>
        <g class="timelinePlayheadGroup" aria-hidden="true">
          <line
            class="timelinePlayhead"
            x1={viewBoxX(props.playheadX)}
            y1={rulerLineTopPx()}
            x2={viewBoxX(props.playheadX)}
            y2={canvasH() - 1}
          />
          <polygon
            class="timelinePlayheadHandle"
            points={`${viewBoxX(props.playheadX) - 4},${rulerLineTopPx()} ${viewBoxX(props.playheadX) + 4},${rulerLineTopPx()} ${viewBoxX(props.playheadX)},${rulerLineTopPx() + 5}`}
          />
        </g>
      </Show>
    </svg>
  );

  return (
    <>
      <div
        class="timelineOverviewFrame"
        classList={{ timelineOverviewLayered: !props.legacyMode }}
      >
        <Show
          when={!props.legacyMode}
          fallback={
            <>
              {renderLegacyGutter()}
              {renderOverviewCanvas()}
            </>
          }
        >
          <div class="timelineLayerScrollport" data-timeline-layer-scrollport>
            <div
              class="timelineLayerScrollContent"
              style={{ height: `${sectionLayout().contentHeight}px` }}
            >
              {renderLayeredGutter()}
              {renderOverviewCanvas()}
            </div>
          </div>
        </Show>
        <Show when={props.superSceneEmptyHintCount > 0}>
          <p
            class="timelineSuperSceneEmptyHint textPretty"
            data-timeline-super-scene-empty-hint={props.superSceneEmptyHintCount}
          >
            {superSceneEmptyHint()}
          </p>
        </Show>
      </div>
      <Show when={safeCueDrag()}>
        {(drag) => {
          const targetLayerId = () => cueDragTargetLayerId();
          const dropState = () => matrixCueDropState() ?? (
            targetLayerId() === null
              ? "outside"
              : cueDropStateForLayer(targetLayerId()!)
          );
          return (
            <div
              class={`timelineCueDragGhost ${dropState()}`}
              data-timeline-cue-drag-ghost
              data-timeline-drop-state={dropState()}
              style={{
                position: "fixed",
                left: `${drag().client_x + 12}px`,
                top: `${drag().client_y + 12}px`,
              }}
              aria-hidden="true"
            >
              <span data-no-localize>{drag().cue_label}</span>
            </div>
          );
        }}
      </Show>
    </>
  );
}
