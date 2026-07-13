import { createMemo, createSignal, For, onCleanup, onMount, Show } from "solid-js";
import {
  beginTimelineMarkerDragProjection,
  shouldCommitTimelineMarkerDrag,
  timelineSceneBlockLoopDivisionPositions,
  updateTimelineMarkerDragProjection,
} from "../timelineSceneBlocks";
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
  active: boolean;
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

interface TimelineOverviewProps {
  events: TimelineOverviewEvent[];
  automationRanges: TimelineOverviewAutomationRange[];
  selectedRangeId: string | null;
  selectedEventId: number | null;
  playheadX: number;
  onSeekRatio: (ratio: number) => void;
  onSeekTime: (timeMs: number) => void;
  onSelectAutomationRange: (range: TimelineOverviewAutomationRange) => void;
  onSelectEvent: (eventId: number) => void;
  onMoveEventRatio: (eventId: number, ratio: number) => void;
  onMoveAutomationRangeRatio: (range: TimelineOverviewAutomationRange, ratio: number) => void;
  onResizeAutomationRangeRatio: (
    range: TimelineOverviewAutomationRange,
    edge: "start" | "end",
    ratio: number,
  ) => void;
  onMoveAutomationKeyframeRatio: (
    range: TimelineOverviewAutomationRange,
    keyframeIndex: number,
    ratio: number,
  ) => void;
}

interface TimelineMarkerDrag {
  eventId: number;
  pointerId: number;
  x: number;
  maxX: number;
  grabOffsetX: number;
  startClientX: number;
  moved: boolean;
}

interface TimelineAutomationRangeDrag {
  rangeId: string;
  pointerId: number;
  mode: "move" | "resize-start" | "resize-end";
  grabOffsetX: number;
  startClientX: number;
  moved: boolean;
  x: number;
  width: number;
}

interface TimelineAutomationKeyframeDrag {
  rangeId: string;
  keyframeIndex: number;
  pointerId: number;
  startClientX: number;
  moved: boolean;
  grabOffsetX: number;
  x: number;
}

const clampRatio = (value: number) => Math.min(1, Math.max(0, value));
const clampPercent = (value: number, max = 100) => Math.min(max, Math.max(0, value));
const minAutomationRangeWidth = 0.75;
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
  const [viewBoxWidth, setViewBoxWidth] = createSignal(100);
  let overviewElement: SVGSVGElement | undefined;
  onMount(() => {
    const updateViewBox = () => {
      if (!overviewElement) return;
      const width = overviewElement.clientWidth;
      const height = overviewElement.clientHeight;
      if (width > 0 && height > 0) setViewBoxWidth(44 * (width / height));
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
  const activeEventIds = createMemo(
    () => new Set(props.events.filter((event) => event.active).map((event) => event.id)),
    new Set<number>(),
    { equals: sameNumberSet },
  );
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

  const ratioFromPointer = (event: PointerEvent | MouseEvent, svg: SVGSVGElement) => {
    const rect = svg.getBoundingClientRect();
    return clampRatio((event.clientX - rect.left) / Math.max(1, rect.width));
  };

  const seekFromPointer = (event: MouseEvent & { currentTarget: SVGSVGElement }) => {
    props.onSeekRatio(ratioFromPointer(event, event.currentTarget));
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
    const projection = beginTimelineMarkerDragProjection(
      overviewEvent.x,
      overviewEvent.width,
      ratioFromPointer(event, svg) * 100,
      event.clientX,
    );
    setMarkerDrag({
      eventId: overviewEvent.id,
      pointerId: event.pointerId,
      x: projection.x,
      maxX: projection.max_x,
      grabOffsetX: projection.grab_offset_x,
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
    const projection = updateTimelineMarkerDragProjection(
      {
        x: drag.x,
        max_x: drag.maxX,
        grab_offset_x: drag.grabOffsetX,
        start_client_x: drag.startClientX,
        moved: drag.moved,
      },
      ratioFromPointer(event, svg) * 100,
      event.clientX,
    );
    setMarkerDrag({
      ...drag,
      x: projection.x,
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
    props.onMoveEventRatio(drag.eventId, drag.x / 100);
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
    const pointerX = ratioFromPointer(event, svg) * 100;
    event.preventDefault();
    event.stopPropagation();
    props.onSelectAutomationRange(range);
    event.currentTarget.setPointerCapture(event.pointerId);
    setRangeDrag({
      rangeId: range.id,
      pointerId: event.pointerId,
      mode: "move",
      grabOffsetX: pointerX - range.x,
      startClientX: event.clientX,
      moved: false,
      x: range.x,
      width: range.width,
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
      grabOffsetX: 0,
      startClientX: event.clientX,
      moved: false,
      x: range.x,
      width: range.width,
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
    const pointerX = ratioFromPointer(event, svg) * 100;
    const moved = drag.moved || Math.abs(event.clientX - drag.startClientX) >= 4;
    if (!moved) {
      return;
    }
    if (drag.mode === "resize-start") {
      const nextX = clampPercent(pointerX, range.x + range.width - minAutomationRangeWidth);
      setRangeDrag({
        ...drag,
        moved,
        x: nextX,
        width: Math.max(minAutomationRangeWidth, range.x + range.width - nextX),
      });
      return;
    }
    if (drag.mode === "resize-end") {
      const nextEndX = Math.max(range.x + minAutomationRangeWidth, clampPercent(pointerX));
      setRangeDrag({
        ...drag,
        moved,
        x: range.x,
        width: Math.max(minAutomationRangeWidth, nextEndX - range.x),
      });
      return;
    }
    setRangeDrag({
      ...drag,
      moved,
      x: clampPercent(pointerX - drag.grabOffsetX, 100 - range.width),
      width: range.width,
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
        props.onResizeAutomationRangeRatio(range, "start", drag.x / 100);
      } else if (drag.mode === "resize-end") {
        props.onResizeAutomationRangeRatio(range, "end", (drag.x + drag.width) / 100);
      } else {
        props.onMoveAutomationRangeRatio(range, drag.x / 100);
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
    const spanMs = Math.max(1, range.end_ms - range.start_ms);
    const localRatio = clampRatio(
      ((range.keyframes.find((keyframe) => keyframe.keyframe_index === keyframeIndex)?.time_ms ?? range.start_ms) - range.start_ms) / spanMs,
    );
    const keyframeX = range.x + localRatio * range.width;
    const pointerX = ratioFromPointer(event, svg) * 100;
    setKeyframeDrag({
      rangeId: range.id,
      keyframeIndex,
      pointerId: event.pointerId,
      startClientX: event.clientX,
      moved: false,
      grabOffsetX: pointerX - keyframeX,
      x: keyframeX,
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
    const moved = drag.moved || Math.abs(event.clientX - drag.startClientX) >= 4;
    if (!moved) {
      return;
    }
    setKeyframeDrag({
      ...drag,
      moved,
      x: clampPercent(ratioFromPointer(event, svg) * 100 - drag.grabOffsetX),
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
      props.onMoveAutomationKeyframeRatio(range, drag.keyframeIndex, drag.x / 100);
    }
  };
  const endKeyframeDrag = (event: PointerEvent & { currentTarget: SVGGElement }) =>
    finishKeyframeDrag(event, false);
  const cancelKeyframeDrag = (event: PointerEvent & { currentTarget: SVGGElement }) =>
    finishKeyframeDrag(event, true);

  const automationRangeX = (range: TimelineOverviewAutomationRange) =>
    rangeDrag()?.rangeId === range.id ? rangeDrag()!.x : range.x;
  const automationRangeWidth = (range: TimelineOverviewAutomationRange) =>
    rangeDrag()?.rangeId === range.id ? rangeDrag()!.width : range.width;
  const automationKeyframeX = (range: TimelineOverviewAutomationRange, keyframeIndex: number, timeMs: number) => {
    const drag = keyframeDrag();
    if (drag?.rangeId === range.id && drag.keyframeIndex === keyframeIndex) {
      return drag.x;
    }
    const spanMs = Math.max(1, range.end_ms - range.start_ms);
    const localRatio = clampRatio((timeMs - range.start_ms) / spanMs);
    return automationRangeX(range) + localRatio * automationRangeWidth(range);
  };
  const viewBoxX = (percent: number) => (percent / 100) * viewBoxWidth();
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
    const label = event.loop_count > 1 ? `${event.cue_label} x${event.loop_count}` : event.cue_label;
    const availableWidth = Math.max(0, viewBoxX(event.width) - 2.4);
    const maxCharacters = Math.floor(availableWidth / 5.5);
    if (maxCharacters < 3) return "";
    return label.length <= maxCharacters
      ? label
      : `${label.slice(0, Math.max(1, maxCharacters - 1))}…`;
  };

  return (
    <svg
      class="timelineOverview"
      ref={(element) => { overviewElement = element; }}
      viewBox={`0 0 ${viewBoxWidth()} 44`}
      data-viewbox-width={viewBoxWidth()}
      role="img"
      aria-label="Timeline overview"
      onClick={seekFromPointer}
    >
      <rect class="timelineOverviewBg" x="0" y="0" width={viewBoxWidth()} height="44" />
      <line class="timelineLaneDivider" x1="0" y1="22" x2={viewBoxWidth()} y2="22" />
      <text class="timelineLaneLabel" x="1.2" y="12">Light</text>
      <text class="timelineLaneLabel" x="1.2" y="30">Video</text>
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
              activeEventIds().has(event.id) ? "active" : "",
              props.selectedEventId === event.id ? "selected" : "",
              markerDrag()?.eventId === event.id ? "dragging" : "",
            ].filter(Boolean).join(" ")}
            data-timeline-event-id={event.id}
            transform={`translate(${viewBoxX(markerDrag()?.eventId === event.id ? markerDrag()!.x : event.x)} ${event.y})`}
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
                  data-full-label={event.loop_count > 1 ? `${event.cue_label} x${event.loop_count}` : event.cue_label}
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
      <line class="timelinePlayhead" x1={viewBoxX(props.playheadX)} y1="2" x2={viewBoxX(props.playheadX)} y2="42" />
    </svg>
  );
}
