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

interface TimelineOverviewProps {
  events: TimelineOverviewEvent[];
  playheadX: number;
  onSeekRatio: (ratio: number) => void;
  onSeekTime: (timeMs: number) => void;
  onMoveEventRatio: (eventId: number, ratio: number) => void;
}

interface TimelineMarkerDrag {
  eventId: number;
  pointerId: number;
  x: number;
}

const clampRatio = (value: number) => Math.min(1, Math.max(0, value));

export function TimelineOverview(props: TimelineOverviewProps) {
  const [markerDrag, setMarkerDrag] = createSignal<TimelineMarkerDrag | null>(null);
  const [suppressClickEventId, setSuppressClickEventId] = createSignal<number | null>(null);

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
