import { For, Show } from "solid-js";
import type { VideoCuePointSummary, VideoLayerState, VideoSourceSummary } from "../types";

const timelineViewBoxWidth = 100;
const timelineViewBoxHeight = 34;
const timelineFrame = {
  x: 4,
  y: 12,
  width: 92,
  height: 8,
};

const finiteOr = (value: number, fallback: number) => (Number.isFinite(value) ? value : fallback);
const clampRange = (value: number, min: number, max: number) => Math.min(max, Math.max(min, value));

const durationFor = (source: VideoSourceSummary, state: VideoLayerState, cuePoints: VideoCuePointSummary[]) =>
  Math.max(
    1,
    Math.round(source.metadata?.duration_ms ?? 0),
    Math.round(state.position_ms ?? 0),
    Math.round(state.loop_end_ms ?? 0),
    ...cuePoints.map((cuePoint) => Math.round(cuePoint.position_ms)),
    1000,
  );

const formatTimelineTime = (positionMs: number) => {
  const totalSeconds = Math.max(0, Math.round(positionMs / 1000));
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  return `${minutes}:${seconds.toString().padStart(2, "0")}`;
};

const pointerTimeMs = (event: PointerEvent, svg: SVGSVGElement | null, durationMs: number) => {
  if (!svg) {
    return null;
  }
  const rect = svg.getBoundingClientRect();
  if (rect.width <= 0) {
    return null;
  }
  const x = ((event.clientX - rect.left) / rect.width) * timelineViewBoxWidth;
  const ratio = clampRange((x - timelineFrame.x) / timelineFrame.width, 0, 1);
  return Math.round(ratio * durationMs);
};

const keyboardTimeStep = (event: KeyboardEvent, durationMs: number) => {
  if (event.shiftKey) {
    return Math.max(1, Math.round(durationMs / 16));
  }
  if (event.altKey) {
    return 1;
  }
  return Math.max(1, Math.round(durationMs / 100));
};

interface VideoPlaybackTimelineProps {
  compact?: boolean;
  label: string;
  source: VideoSourceSummary;
  state: VideoLayerState;
  cuePoints: VideoCuePointSummary[];
  onStatePatch: (patch: Partial<VideoLayerState>) => void;
  onAddCuePoint: (positionMs?: number) => void;
  onJumpCuePoint: (index: number) => void;
  onSetCuePoint: (index: number, cuePoint: VideoCuePointSummary) => void;
}

export function VideoPlaybackTimeline(props: VideoPlaybackTimelineProps) {
  const durationMs = () => durationFor(props.source, props.state, props.cuePoints);
  const xForTime = (positionMs: number) =>
    timelineFrame.x + clampRange(finiteOr(positionMs, 0) / durationMs(), 0, 1) * timelineFrame.width;
  const loopStart = () => clampRange(Math.round(props.state.loop_start_ms), 0, durationMs());
  const loopEnd = () => clampRange(Math.max(loopStart() + 1, Math.round(props.state.loop_end_ms)), 1, durationMs());
  const position = () => clampRange(Math.round(props.state.position_ms), 0, durationMs());

  const setPositionFromPointer = (event: PointerEvent & { currentTarget: SVGRectElement | SVGLineElement }) => {
    event.preventDefault();
    const timeMs = pointerTimeMs(event, event.currentTarget.ownerSVGElement, durationMs());
    if (timeMs === null) {
      return;
    }
    props.onStatePatch({ position_ms: timeMs });
  };

  const setLoopBoundaryFromPointer = (
    event: PointerEvent & { currentTarget: SVGPolygonElement },
    boundary: "start" | "end",
  ) => {
    event.preventDefault();
    const timeMs = pointerTimeMs(event, event.currentTarget.ownerSVGElement, durationMs());
    if (timeMs === null) {
      return;
    }
    props.onStatePatch({
      loop_enabled: true,
      loop_start_ms: boundary === "start" ? Math.min(timeMs, loopEnd() - 1) : loopStart(),
      loop_end_ms: boundary === "end" ? Math.max(timeMs, loopStart() + 1) : loopEnd(),
    });
  };

  const setCuePointFromPointer = (
    event: PointerEvent & { currentTarget: SVGPolygonElement },
    cuePoint: VideoCuePointSummary,
    index: number,
  ) => {
    event.preventDefault();
    const timeMs = pointerTimeMs(event, event.currentTarget.ownerSVGElement, durationMs());
    if (timeMs === null) {
      return;
    }
    props.onSetCuePoint(index, { ...cuePoint, position_ms: timeMs });
  };

  const nudgePosition = (event: KeyboardEvent) => {
    if (event.key === "Home") {
      event.preventDefault();
      props.onStatePatch({ position_ms: 0 });
      return;
    }
    if (event.key === "End") {
      event.preventDefault();
      props.onStatePatch({ position_ms: durationMs() });
      return;
    }
    if (event.key !== "ArrowLeft" && event.key !== "ArrowRight") {
      return;
    }
    event.preventDefault();
    const direction = event.key === "ArrowRight" ? 1 : -1;
    props.onStatePatch({ position_ms: clampRange(position() + keyboardTimeStep(event, durationMs()) * direction, 0, durationMs()) });
  };

  const nudgeLoopBoundary = (event: KeyboardEvent, boundary: "start" | "end") => {
    if (event.key === "Home") {
      event.preventDefault();
      props.onStatePatch({ loop_enabled: true, [boundary === "start" ? "loop_start_ms" : "loop_end_ms"]: boundary === "start" ? 0 : durationMs() });
      return;
    }
    if (event.key !== "ArrowLeft" && event.key !== "ArrowRight") {
      return;
    }
    event.preventDefault();
    const direction = event.key === "ArrowRight" ? 1 : -1;
    const step = keyboardTimeStep(event, durationMs()) * direction;
    props.onStatePatch({
      loop_enabled: true,
      loop_start_ms: boundary === "start" ? clampRange(loopStart() + step, 0, loopEnd() - 1) : loopStart(),
      loop_end_ms: boundary === "end" ? clampRange(loopEnd() + step, loopStart() + 1, durationMs()) : loopEnd(),
    });
  };

  const loopPoints = () => {
    const startX = xForTime(loopStart());
    const endX = xForTime(loopEnd());
    return {
      startX,
      endX,
      y: timelineFrame.y - 4,
      bottomY: timelineFrame.y + timelineFrame.height + 4,
    };
  };

  const handleTriangle = (x: number, top: boolean) =>
    top
      ? `${x},${timelineFrame.y - 6} ${x - 3},${timelineFrame.y - 1} ${x + 3},${timelineFrame.y - 1}`
      : `${x},${timelineFrame.y + timelineFrame.height + 6} ${x - 3},${timelineFrame.y + timelineFrame.height + 1} ${x + 3},${timelineFrame.y + timelineFrame.height + 1}`;

  return (
    <div class={props.compact ? "videoPlaybackTimeline compact" : "videoPlaybackTimeline"}>
      <svg class="videoPlaybackSurface" viewBox={`0 0 ${timelineViewBoxWidth} ${timelineViewBoxHeight}`} role="img">
        <rect
          class="videoPlaybackHitbox"
          x="0"
          y="0"
          width={timelineViewBoxWidth}
          height={timelineViewBoxHeight}
          tabIndex={0}
          onPointerDown={(event) => {
            event.currentTarget.setPointerCapture(event.pointerId);
            setPositionFromPointer(event);
          }}
          onPointerMove={(event) => {
            if (event.buttons === 1) {
              setPositionFromPointer(event);
            }
          }}
          onPointerUp={(event) => event.currentTarget.releasePointerCapture(event.pointerId)}
          onPointerCancel={(event) => event.currentTarget.releasePointerCapture(event.pointerId)}
          onKeyDown={nudgePosition}
        >
          <title>Seek {props.label}. Arrow keys nudge, Shift coarse, Alt fine, Home/End jump.</title>
        </rect>
        <rect class="videoPlaybackTrack" x={timelineFrame.x} y={timelineFrame.y} width={timelineFrame.width} height={timelineFrame.height} />
        <rect
          class={props.state.loop_enabled ? "videoPlaybackLoop" : "videoPlaybackLoop inactive"}
          x={loopPoints().startX}
          y={timelineFrame.y}
          width={Math.max(0.5, loopPoints().endX - loopPoints().startX)}
          height={timelineFrame.height}
        />
        <For each={props.cuePoints}>
          {(cuePoint, index) => {
            const x = () => xForTime(cuePoint.position_ms);
            return (
              <>
                <line
                  class="videoPlaybackCueLine"
                  x1={x()}
                  x2={x()}
                  y1={timelineFrame.y - 3}
                  y2={timelineFrame.y + timelineFrame.height + 3}
                  style={{ "--cue-color": cuePoint.color ?? "#4aa8ff" }}
                />
                <polygon
                  class="videoPlaybackCueHandle"
                  points={handleTriangle(x(), false)}
                  style={{ "--cue-color": cuePoint.color ?? "#4aa8ff" }}
                  tabIndex={0}
                  onPointerDown={(event) => {
                    event.currentTarget.setPointerCapture(event.pointerId);
                    setCuePointFromPointer(event, cuePoint, index());
                  }}
                  onPointerMove={(event) => {
                    if (event.buttons === 1) {
                      setCuePointFromPointer(event, cuePoint, index());
                    }
                  }}
                  onPointerUp={(event) => event.currentTarget.releasePointerCapture(event.pointerId)}
                  onPointerCancel={(event) => event.currentTarget.releasePointerCapture(event.pointerId)}
                  onDblClick={() => props.onJumpCuePoint(index())}
                >
                  <title data-no-localize>{cuePoint.label} {formatTimelineTime(cuePoint.position_ms)}</title>
                </polygon>
              </>
            );
          }}
        </For>
        <line class="videoPlaybackPlayhead" x1={xForTime(position())} x2={xForTime(position())} y1="4" y2="30" />
        <polygon
          class="videoPlaybackLoopHandle start"
          points={handleTriangle(loopPoints().startX, true)}
          tabIndex={0}
          onPointerDown={(event) => {
            event.currentTarget.setPointerCapture(event.pointerId);
            setLoopBoundaryFromPointer(event, "start");
          }}
          onPointerMove={(event) => {
            if (event.buttons === 1) {
              setLoopBoundaryFromPointer(event, "start");
            }
          }}
          onPointerUp={(event) => event.currentTarget.releasePointerCapture(event.pointerId)}
          onPointerCancel={(event) => event.currentTarget.releasePointerCapture(event.pointerId)}
          onKeyDown={(event) => nudgeLoopBoundary(event, "start")}
        >
          <title>Loop in {formatTimelineTime(loopStart())}</title>
        </polygon>
        <polygon
          class="videoPlaybackLoopHandle end"
          points={handleTriangle(loopPoints().endX, true)}
          tabIndex={0}
          onPointerDown={(event) => {
            event.currentTarget.setPointerCapture(event.pointerId);
            setLoopBoundaryFromPointer(event, "end");
          }}
          onPointerMove={(event) => {
            if (event.buttons === 1) {
              setLoopBoundaryFromPointer(event, "end");
            }
          }}
          onPointerUp={(event) => event.currentTarget.releasePointerCapture(event.pointerId)}
          onPointerCancel={(event) => event.currentTarget.releasePointerCapture(event.pointerId)}
          onKeyDown={(event) => nudgeLoopBoundary(event, "end")}
        >
          <title>Loop out {formatTimelineTime(loopEnd())}</title>
        </polygon>
      </svg>
      <div class="videoPlaybackReadout">
        <span>{formatTimelineTime(position())}</span>
        <span>{formatTimelineTime(durationMs())}</span>
        <span>{props.state.loop_enabled ? `${formatTimelineTime(loopStart())} - ${formatTimelineTime(loopEnd())}` : "Loop off"}</span>
      </div>
      <Show when={!props.compact}>
        <div class="videoPlaybackToolbar">
          <button onClick={() => props.onAddCuePoint(position())}>+ Cue Point</button>
          <button
            class={props.state.loop_enabled ? "active" : ""}
            onClick={() => props.onStatePatch({ loop_enabled: !props.state.loop_enabled })}
          >
            {props.state.loop_enabled ? "Loop On" : "Loop Off"}
          </button>
          <button onClick={() => props.onStatePatch({ loop_enabled: true, loop_start_ms: 0, loop_end_ms: durationMs() })}>
            Full
          </button>
        </div>
      </Show>
    </div>
  );
}
