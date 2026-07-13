import { createMemo, createSignal, For, onCleanup, onMount, Show } from "solid-js";
import {
  shouldCommitTimelineMarkerDrag,
  timelineSceneBlockLoopDivisionPositions,
} from "../timelineSceneBlocks";
import {
  beginTimelineAbsoluteDragProjection,
  buildTimelineRulerTicks,
  timelineTimeToVisibleRawRatio,
  timelineVisibleRatioToTimeMs,
  timelineVisibleWindowSpanMs,
  updateTimelineAbsoluteDragProjection,
  type TimelineVisibleWindow,
} from "../timelineViewport";
import { packTimelineOverlapClusterBadges } from "../timelineOverlapClusters";
import type { TimelineTrackKind } from "../types";

export interface TimelineOverviewEvent {
  id: number;
  cue_id: number;
  cue_label: string;
  track: TimelineTrackKind;
  time_ms: number;
  duration_ms: number;
  loop_count: number;
  total_duration_ms: number;
  x: number;
  width: number;
  y: number;
  under_playhead: boolean;
}

export interface TimelineOverviewAutomationRange {
  id: string;
  kind: "lighting" | "video";
  automation_id: number;
  target_id: number;
  label: string;
  track: TimelineTrackKind;
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
  events: TimelineOverviewEvent[];
  executionLive: boolean;
  markerAriaLabel: (event: TimelineOverviewEvent) => string;
  automationRanges: TimelineOverviewAutomationRange[];
  overlapClusters: TimelineOverviewOverlapCluster[];
  selectedRangeId: string | null;
  selectedEventId: number | null;
  playheadX: number;
  visibleWindow: TimelineVisibleWindow;
  onSeekTime: (timeMs: number) => void;
  onSelectAutomationRange: (range: TimelineOverviewAutomationRange) => void;
  onSelectEvent: (eventId: number) => void;
  onInspectOverlapCluster: (cluster: TimelineOverviewOverlapCluster) => void;
  onMoveEventTime: (eventId: number, timeMs: number) => void;
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
  moved: boolean;
}

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
const maxSceneBlockLoopLines = 400;
const sameNumberSet = (left: Set<number>, right: Set<number>) =>
  left.size === right.size && [...left].every((value) => right.has(value));

const sameOverviewEvent = (left: TimelineOverviewEvent, right: TimelineOverviewEvent) =>
  left.id === right.id &&
  left.cue_id === right.cue_id &&
  left.cue_label === right.cue_label &&
  left.track === right.track &&
  left.time_ms === right.time_ms &&
  left.duration_ms === right.duration_ms &&
  left.loop_count === right.loop_count &&
  left.total_duration_ms === right.total_duration_ms &&
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

export function TimelineOverview(props: TimelineOverviewProps) {
  const [markerDrag, setMarkerDrag] = createSignal<TimelineMarkerDrag | null>(null);
  const [rangeDrag, setRangeDrag] = createSignal<TimelineAutomationRangeDrag | null>(null);
  const [keyframeDrag, setKeyframeDrag] = createSignal<TimelineAutomationKeyframeDrag | null>(null);
  const [suppressClickEventId, setSuppressClickEventId] = createSignal<number | null>(null);
  const [suppressClickRangeId, setSuppressClickRangeId] = createSignal<string | null>(null);
  const [keyboardMarkerEventId, setKeyboardMarkerEventId] = createSignal<number | null>(null);
  const [viewBoxWidth, setViewBoxWidth] = createSignal(100);
  const [overviewPixelWidth, setOverviewPixelWidth] = createSignal(1);
  let overviewElement: SVGSVGElement | undefined;
  onMount(() => {
    const updateViewBox = () => {
      if (!overviewElement) return;
      const width = overviewElement.clientWidth;
      const height = overviewElement.clientHeight;
      if (width > 0 && height > 0) {
        setViewBoxWidth(44 * (width / height));
        setOverviewPixelWidth(width);
      }
    };
    updateViewBox();
    const observer = new ResizeObserver(updateViewBox);
    if (overviewElement) observer.observe(overviewElement);
    onCleanup(() => observer.disconnect());
  });
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
  const markerTabStopId = createMemo(() => {
    const events = orderedEvents();
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
    const events = stableEvents();
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
    event.currentTarget.setPointerCapture(event.pointerId);
    const projection = beginTimelineAbsoluteDragProjection(overviewEvent.time_ms, event.clientX);
    setMarkerDrag({
      eventId: overviewEvent.id,
      pointerId: event.pointerId,
      originalTimeMs: projection.original_time_ms,
      timeMs: projection.time_ms,
      startClientX: projection.start_client_x,
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
    setMarkerDrag({
      ...drag,
      timeMs: projection.time_ms,
      moved: projection.moved,
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
    props.onMoveEventTime(drag.eventId, drag.timeMs);
  };
  const endMarkerDrag = (event: PointerEvent & { currentTarget: SVGGElement }) =>
    finishMarkerDrag(event, false);
  const cancelMarkerDrag = (event: PointerEvent & { currentTarget: SVGGElement }) =>
    finishMarkerDrag(event, true);

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
  const viewBoxX = (percent: number) => (percent / 100) * viewBoxWidth();
  const packedOverlapClusters = createMemo(() => packTimelineOverlapClusterBadges(
    props.overlapClusters.map((cluster) => ({
      ...cluster,
      x: viewBoxX(cluster.x),
      width: viewBoxX(cluster.width),
    })),
    viewBoxWidth(),
  ));
  const rulerTicks = createMemo(() => buildTimelineRulerTicks(props.visibleWindow, overviewPixelWidth()));
  const rulerLabelX = (ratio: number) => Math.min(
    Math.max(1.2, ratio * viewBoxWidth()),
    Math.max(1.2, viewBoxWidth() - 1.2),
  );
  const rulerLabelAnchor = (ratio: number) => ratio <= 0.04 ? "start" : ratio >= 0.96 ? "end" : "middle";
  const desiredSceneBlockLoopDivisions = (event: TimelineOverviewEvent) => {
    const divisions = Math.max(0, event.loop_count - 1);
    const divisionsAllowedByWidth = Math.max(0, Math.min(8, Math.floor(event.width / 2.5) - 1));
    return Math.min(divisions, divisionsAllowedByWidth);
  };
  const sceneBlockLoopDivisionCounts = createMemo(() => {
    const eligible = stableEvents()
      .map((event) => ({ event, desired: desiredSceneBlockLoopDivisions(event) }))
      .filter(({ desired }) => desired > 0);
    const counts = new Map<number, number>();
    if (eligible.length > maxSceneBlockLoopLines) {
      for (let sample = 0; sample < maxSceneBlockLoopLines; sample += 1) {
        const index = Math.floor((sample * eligible.length) / maxSceneBlockLoopLines);
        counts.set(eligible[index].event.id, 1);
      }
      return counts;
    }
    let remaining = maxSceneBlockLoopLines;
    for (let level = 0; remaining > 0; level += 1) {
      let allocated = false;
      for (const { event, desired } of eligible) {
        if (desired <= level || remaining === 0) continue;
        counts.set(event.id, level + 1);
        remaining -= 1;
        allocated = true;
      }
      if (!allocated) break;
    }
    return counts;
  });
  const sceneBlockLoopDivisions = (event: TimelineOverviewEvent) => {
    const visibleDivisions = sceneBlockLoopDivisionCounts().get(event.id) ?? 0;
    return timelineSceneBlockLoopDivisionPositions(event.width, event.loop_count, visibleDivisions);
  };
  const sceneBlockLabel = (event: TimelineOverviewEvent) => {
    // Keep the dense overview label focused on identity. Loop semantics remain
    // available through the marker title and data attributes; repeating a nearby
    // `x256` beside an aggregate `×250` badge made the two counts ambiguous.
    const label = event.cue_label;
    const availableWidth = Math.max(0, viewBoxX(event.width) - 2.4);
    const maxCharacters = Math.floor(availableWidth / 5.5);
    if (maxCharacters < 3) return "";
    return label.length <= maxCharacters
      ? label
      : `${label.slice(0, Math.max(1, maxCharacters - 1))}…`;
  };

  return (
    <svg
      class={`timelineOverview ${props.executionLive ? "executingLive" : ""}`}
      ref={(element) => { overviewElement = element; }}
      viewBox={`0 0 ${viewBoxWidth()} 44`}
      data-viewbox-width={viewBoxWidth()}
      role="group"
      aria-label="Timeline overview"
      onClick={seekFromPointer}
    >
      <rect class="timelineOverviewBg" x="0" y="0" width={viewBoxWidth()} height="44" />
      <line class="timelineLaneDivider" x1="0" y1="22" x2={viewBoxWidth()} y2="22" />
      <g class="timelineRuler" aria-hidden="true">
        <For each={rulerTicks()}>
          {(tick) => (
            <g data-timeline-ruler-ms={tick.time_ms}>
              <line
                class={tick.major ? "major" : ""}
                x1={tick.ratio * viewBoxWidth()}
                x2={tick.ratio * viewBoxWidth()}
                y1="6"
                y2="42"
              />
              <text
                x={rulerLabelX(tick.ratio)}
                y="5"
                text-anchor={rulerLabelAnchor(tick.ratio)}
              >
                {tick.label}
              </text>
            </g>
          )}
        </For>
      </g>
      <For each={stableAutomationRanges()}>
        {(range) => (
          <g
            class={[
              "timelineAutomationRange",
              range.track === "Lighting" ? "lighting" : "video",
              range.enabled ? "" : "disabled",
              props.selectedRangeId === range.id ? "selected" : "",
              rangeDrag()?.rangeId === range.id ? "dragging" : "",
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
              y={range.y}
              width={viewBoxX(automationRangeWidth(range))}
              height="3.2"
              rx="1.1"
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
                    cy={range.y + 1.6}
                    r="2.1"
                  />
                  <circle
                    class="timelineAutomationKeyframe"
                    cx={viewBoxX(automationKeyframeX(range, keyframe.keyframe_index, keyframe.time_ms))}
                    cy={range.y + 1.6}
                    r="0.6"
                  />
                </g>
              )}
            </For>
            <rect
              class="timelineAutomationHandle start"
              x={viewBoxX(automationRangeX(range))}
              y={range.y - 0.65}
              width="1.05"
              height="4.5"
              rx="0.3"
              onPointerDown={(pointerEvent) => beginRangeResize(pointerEvent, range, "start")}
            />
            <rect
              class="timelineAutomationHandle end"
              x={Math.max(0, viewBoxX(automationRangeX(range) + automationRangeWidth(range)) - 1.05)}
              y={range.y - 0.65}
              width="1.05"
              height="4.5"
              rx="0.3"
              onPointerDown={(pointerEvent) => beginRangeResize(pointerEvent, range, "end")}
            />
            <title>
              {range.label} / {range.track} / {range.start_ms}-{range.end_ms} ms
            </title>
          </g>
        )}
      </For>
      <For each={orderedEvents()}>
        {(event) => (
          <g
            class={[
              "timelineMarker",
              event.duration_ms > 0 ? "sceneBlock" : "pointEvent",
              event.track === "Lighting" ? "lighting" : "video",
              underPlayheadEventIds().has(event.id) ? "underPlayhead" : "",
              props.selectedEventId === event.id ? "selected" : "",
              markerDrag()?.eventId === event.id ? "dragging" : "",
            ].filter(Boolean).join(" ")}
            data-timeline-event-id={event.id}
            data-timeline-start-ms={event.time_ms}
            data-timeline-loop-count={event.loop_count}
            data-timeline-preview-start-ms={markerDrag()?.eventId === event.id
              ? markerDrag()!.timeMs
              : event.time_ms}
            role="button"
            tabIndex={markerTabStopId() === event.id ? 0 : -1}
            aria-label={props.markerAriaLabel(event)}
            transform={`translate(${viewBoxX(markerDrag()?.eventId === event.id
              ? timelineTimeToVisibleRawRatio(markerDrag()!.timeMs, props.visibleWindow) * 100
              : event.x)} ${event.y})`}
            onPointerDown={(pointerEvent) => beginMarkerDrag(pointerEvent, event)}
            onPointerMove={moveMarkerDrag}
            onPointerUp={endMarkerDrag}
            onPointerCancel={cancelMarkerDrag}
            onClick={(pointerEvent) => {
              pointerEvent.stopPropagation();
              if (suppressClickEventId() === event.id) {
                setSuppressClickEventId(null);
                return;
              }
              props.onSelectEvent(event.id);
              props.onSeekTime(event.time_ms);
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
              props.onSelectEvent(event.id);
              props.onSeekTime(event.time_ms);
            }}
          >
            <Show
              when={event.duration_ms > 0}
              fallback={
                <>
                  <line x1="0" y1="-6" x2="0" y2="6" />
                  <circle cx="0" cy="0" r="1.8" />
                </>
              }
            >
              <rect class="timelineSceneBlockHit" x="0" y="-5.5" width={Math.max(viewBoxX(event.width), 0.8)} height="11" rx="0.8" />
              <rect class="timelineSceneBlockBody" x="0" y="-4" width={Math.max(viewBoxX(event.width), 0.8)} height="8" rx="0.8" />
              <line class="timelineSceneBlockStart" x1="0" y1="-5" x2="0" y2="5" />
              <For each={sceneBlockLoopDivisions(event)}>
                {(divisionX) => (
                  <line class="timelineSceneBlockLoop" x1={viewBoxX(divisionX)} y1="-3.4" x2={viewBoxX(divisionX)} y2="3.4" />
                )}
              </For>
              <Show when={event.width >= 7 && sceneBlockLabel(event)}>
                <text
                  class="timelineSceneBlockLabel"
                  x="1.1"
                  y="1.25"
                  data-full-label={event.cue_label}
                >
                  {sceneBlockLabel(event)}
                </text>
              </Show>
            </Show>
            <title>
              {event.duration_ms > 0
                ? `${event.cue_label} / ${event.track} / ${event.time_ms} ms / ${event.duration_ms} ms x ${event.loop_count}`
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
              tabIndex={0}
              aria-label={accessibleLabel}
              data-overlap-track={cluster.track}
              data-overlap-count={cluster.count}
              data-overlap-start-ms={cluster.start_ms}
              data-overlap-end-ms={cluster.end_ms}
              data-overlap-members={cluster.member_ids.join(",")}
              data-overlap-cluster-ids={cluster.source_cluster_ids.join(",")}
              data-overlap-group-count={cluster.group_count}
              data-overlap-aggregated={cluster.aggregated ? "true" : "false"}
              transform={`translate(${cluster.x} ${cluster.track === "Lighting" ? 6 : 24})`}
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
              <rect width="18" height="6" rx="1" />
              <text x="1.2" y="4.6">×{cluster.count}</text>
              <title>{accessibleLabel}</title>
            </g>
          );
        }}
      </For>
      <g class="timelineLaneLabelPlate" aria-hidden="true">
        <rect x="0" y="6" width="26" height="13" rx="1" />
        <rect x="0" y="24" width="26" height="13" rx="1" />
        <text class="timelineLaneLabel" x="1.2" y="14">Light</text>
        <text class="timelineLaneLabel" x="1.2" y="32">Video</text>
      </g>
      <Show when={props.playheadX >= 0 && props.playheadX <= 100}>
        <line class="timelinePlayhead" x1={viewBoxX(props.playheadX)} y1="2" x2={viewBoxX(props.playheadX)} y2="42" />
      </Show>
    </svg>
  );
}
