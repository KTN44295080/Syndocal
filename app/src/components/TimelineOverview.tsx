import { createSignal, For } from "solid-js";
import type { TimelineTrackKind } from "../types";

export interface TimelineOverviewEvent {
  id: number;
  cue_id: number;
  cue_label: string;
  track: TimelineTrackKind;
  time_ms: number;
  x: number;
  y: number;
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
  playheadX: number;
  onSeekRatio: (ratio: number) => void;
  onSeekTime: (timeMs: number) => void;
  onSelectAutomationRange: (range: TimelineOverviewAutomationRange) => void;
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
}

interface TimelineAutomationRangeDrag {
  rangeId: string;
  pointerId: number;
  mode: "move" | "resize-start" | "resize-end";
  grabOffsetX: number;
  x: number;
  width: number;
}

interface TimelineAutomationKeyframeDrag {
  rangeId: string;
  keyframeIndex: number;
  pointerId: number;
  x: number;
}

const clampRatio = (value: number) => Math.min(1, Math.max(0, value));
const clampPercent = (value: number, max = 100) => Math.min(max, Math.max(0, value));
const minAutomationRangeWidth = 0.75;

export function TimelineOverview(props: TimelineOverviewProps) {
  const [markerDrag, setMarkerDrag] = createSignal<TimelineMarkerDrag | null>(null);
  const [rangeDrag, setRangeDrag] = createSignal<TimelineAutomationRangeDrag | null>(null);
  const [keyframeDrag, setKeyframeDrag] = createSignal<TimelineAutomationKeyframeDrag | null>(null);
  const [suppressClickEventId, setSuppressClickEventId] = createSignal<number | null>(null);
  const [suppressClickRangeId, setSuppressClickRangeId] = createSignal<string | null>(null);

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
    event.currentTarget.setPointerCapture(event.pointerId);
    setMarkerDrag({
      eventId: overviewEvent.id,
      pointerId: event.pointerId,
      x: ratioFromPointer(event, svg) * 100,
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
    setMarkerDrag({
      ...drag,
      x: ratioFromPointer(event, svg) * 100,
    });
  };

  const endMarkerDrag = (event: PointerEvent & { currentTarget: SVGGElement }) => {
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
    setSuppressClickEventId(drag.eventId);
    props.onMoveEventRatio(drag.eventId, drag.x / 100);
  };

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
    if (drag.mode === "resize-start") {
      const nextX = clampPercent(pointerX, range.x + range.width - minAutomationRangeWidth);
      setRangeDrag({
        ...drag,
        x: nextX,
        width: Math.max(minAutomationRangeWidth, range.x + range.width - nextX),
      });
      return;
    }
    if (drag.mode === "resize-end") {
      const nextEndX = Math.max(range.x + minAutomationRangeWidth, clampPercent(pointerX));
      setRangeDrag({
        ...drag,
        x: range.x,
        width: Math.max(minAutomationRangeWidth, nextEndX - range.x),
      });
      return;
    }
    setRangeDrag({
      ...drag,
      x: clampPercent(pointerX - drag.grabOffsetX, 100 - range.width),
      width: range.width,
    });
  };

  const endRangeDrag = (event: PointerEvent & { currentTarget: SVGGElement }) => {
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
    setKeyframeDrag({
      rangeId: range.id,
      keyframeIndex,
      pointerId: event.pointerId,
      x: ratioFromPointer(event, svg) * 100,
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
    setKeyframeDrag({
      ...drag,
      x: ratioFromPointer(event, svg) * 100,
    });
  };

  const endKeyframeDrag = (event: PointerEvent & { currentTarget: SVGGElement }) => {
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
    setSuppressClickRangeId(drag.rangeId);
    const range = props.automationRanges.find((candidate) => candidate.id === drag.rangeId);
    if (range) {
      props.onMoveAutomationKeyframeRatio(range, drag.keyframeIndex, drag.x / 100);
    }
  };

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

  return (
    <svg
      class="timelineOverview"
      viewBox="0 0 100 44"
      role="img"
      aria-label="Timeline overview"
      onClick={seekFromPointer}
    >
      <rect class="timelineOverviewBg" x="0" y="0" width="100" height="44" />
      <line class="timelineLaneDivider" x1="0" y1="22" x2="100" y2="22" />
      <text class="timelineLaneLabel" x="1.2" y="12">Light</text>
      <text class="timelineLaneLabel" x="1.2" y="30">Video</text>
      <For each={props.automationRanges}>
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
            onPointerCancel={endRangeDrag}
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
              x={automationRangeX(range)}
              y={range.y}
              width={automationRangeWidth(range)}
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
                  onPointerCancel={endKeyframeDrag}
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
                    cx={automationKeyframeX(range, keyframe.keyframe_index, keyframe.time_ms)}
                    cy={range.y + 1.6}
                    r="2.1"
                  />
                  <circle
                    class="timelineAutomationKeyframe"
                    cx={automationKeyframeX(range, keyframe.keyframe_index, keyframe.time_ms)}
                    cy={range.y + 1.6}
                    r="0.6"
                  />
                </g>
              )}
            </For>
            <rect
              class="timelineAutomationHandle start"
              x={automationRangeX(range)}
              y={range.y - 0.65}
              width="1.05"
              height="4.5"
              rx="0.3"
              onPointerDown={(pointerEvent) => beginRangeResize(pointerEvent, range, "start")}
            />
            <rect
              class="timelineAutomationHandle end"
              x={automationRangeX(range) + Math.max(0, automationRangeWidth(range) - 1.05)}
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
      <For each={props.events}>
        {(event) => (
          <g
            class={[
              "timelineMarker",
              event.track === "Lighting" ? "lighting" : "video",
              event.active ? "active" : "",
              markerDrag()?.eventId === event.id ? "dragging" : "",
            ].filter(Boolean).join(" ")}
            transform={`translate(${markerDrag()?.eventId === event.id ? markerDrag()!.x : event.x} ${event.y})`}
            onPointerDown={(pointerEvent) => beginMarkerDrag(pointerEvent, event)}
            onPointerMove={moveMarkerDrag}
            onPointerUp={endMarkerDrag}
            onPointerCancel={endMarkerDrag}
            onClick={(pointerEvent) => {
              pointerEvent.stopPropagation();
              if (suppressClickEventId() === event.id) {
                setSuppressClickEventId(null);
                return;
              }
              props.onSeekTime(event.time_ms);
            }}
          >
            <line x1="0" y1="-6" x2="0" y2="6" />
            <circle cx="0" cy="0" r="1.8" />
            <title>
              {event.cue_label} / {event.track} / {event.time_ms} ms
            </title>
          </g>
        )}
      </For>
      <line class="timelinePlayhead" x1={props.playheadX} y1="2" x2={props.playheadX} y2="42" />
    </svg>
  );
}
