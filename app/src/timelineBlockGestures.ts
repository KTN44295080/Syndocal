export type TimelineStretchMode = "RATE" | "WINDOW";
export type TimelineBlockGestureZone =
  | "move"
  | "stretch-start"
  | "stretch-end"
  | "fade-in"
  | "fade-out"
  | "select";

export const TIMELINE_BLOCK_HEIGHT_PX = 28;
export const TIMELINE_BLOCK_UPPER_BAND_PX = 14;
export const TIMELINE_BLOCK_STRETCH_EDGE_PX = 6;
export const TIMELINE_BLOCK_FADE_EDGE_PX = 10;

const finiteOr = (value: number, fallback: number) => Number.isFinite(value) ? value : fallback;
const clamp = (value: number, minimum: number, maximum: number) =>
  Math.min(maximum, Math.max(minimum, value));

const nearestEdge = (x: number, width: number) => x <= width / 2 ? "start" : "end";

export const timelineBlockGestureZone = (
  localX: number,
  localY: number,
  widthPx: number,
  selected: boolean,
  heightPx = TIMELINE_BLOCK_HEIGHT_PX,
): TimelineBlockGestureZone => {
  const width = Math.max(1, finiteOr(widthPx, 1));
  const x = clamp(finiteOr(localX, 0), 0, width);
  const y = clamp(finiteOr(localY, 0), 0, Math.max(1, finiteOr(heightPx, TIMELINE_BLOCK_HEIGHT_PX)));
  const edge = nearestEdge(x, width);
  const edgeDistance = edge === "start" ? x : width - x;
  if (selected && y >= Math.min(TIMELINE_BLOCK_UPPER_BAND_PX, heightPx / 2) && edgeDistance <= TIMELINE_BLOCK_FADE_EDGE_PX) {
    return edge === "start" ? "fade-in" : "fade-out";
  }
  if (selected && edgeDistance <= TIMELINE_BLOCK_STRETCH_EDGE_PX) {
    return edge === "start" ? "stretch-start" : "stretch-end";
  }
  return y < Math.min(TIMELINE_BLOCK_UPPER_BAND_PX, heightPx / 2) ? "move" : "select";
};

export interface TimelineStretchProjectionInput {
  mode: TimelineStretchMode;
  edge: "start" | "end";
  originalStartMs: number;
  originalEndMs: number;
  requestedEdgeMs: number;
  authoredBeats: number | null;
  bpm: number;
}

export interface TimelineStretchProjection {
  start_ms: number;
  end_ms: number;
  duration_ms: number;
  duration_beats: number | null;
  conform_to_tempo: boolean;
  loop_fill: boolean;
  loop_count: number;
  rate: number | null;
  fallback_to_window: boolean;
}

export const projectTimelineBlockStretch = (
  input: TimelineStretchProjectionInput,
): TimelineStretchProjection => {
  const originalStartMs = Math.max(0, finiteOr(input.originalStartMs, 0));
  const originalEndMs = Math.max(originalStartMs + 1, finiteOr(input.originalEndMs, originalStartMs + 1));
  const requestedEdgeMs = Math.max(0, Math.round(finiteOr(
    input.requestedEdgeMs,
    input.edge === "start" ? originalStartMs : originalEndMs,
  )));
  const startMs = input.edge === "start"
    ? Math.min(requestedEdgeMs, originalEndMs - 1)
    : originalStartMs;
  const endMs = input.edge === "end"
    ? Math.max(originalStartMs + 1, requestedEdgeMs)
    : originalEndMs;
  const durationMs = Math.max(1, endMs - startMs);
  const bpm = finiteOr(input.bpm, 0);
  const authoredBeats = input.authoredBeats === null ? null : finiteOr(input.authoredBeats, 0);
  const canConform = authoredBeats !== null && authoredBeats > 0 && bpm > 0;
  const fallbackToWindow = input.mode === "RATE" && !canConform;
  if (!canConform) {
    return {
      start_ms: startMs,
      end_ms: endMs,
      duration_ms: durationMs,
      duration_beats: null,
      conform_to_tempo: false,
      loop_fill: false,
      loop_count: 1,
      rate: null,
      fallback_to_window: fallbackToWindow,
    };
  }
  const durationBeats = durationMs * bpm / 60_000;
  const authoredPeriodMs = authoredBeats * 60_000 / bpm;
  const rate = authoredPeriodMs / durationMs;
  return {
    start_ms: startMs,
    end_ms: endMs,
    duration_ms: durationMs,
    duration_beats: input.mode === "RATE" ? durationBeats : null,
    conform_to_tempo: true,
    loop_fill: input.mode === "WINDOW",
    loop_count: 1,
    rate: input.mode === "RATE" && Number.isFinite(rate) && rate > 0 ? rate : null,
    fallback_to_window: fallbackToWindow,
  };
};

export const projectTimelineBlockFadeMs = (
  edge: "in" | "out",
  blockStartMs: number,
  blockEndMs: number,
  pointerTimeMs: number,
) => {
  const startMs = Math.max(0, finiteOr(blockStartMs, 0));
  const endMs = Math.max(startMs + 1, finiteOr(blockEndMs, startMs + 1));
  const pointerMs = clamp(finiteOr(pointerTimeMs, edge === "in" ? startMs : endMs), startMs, endMs);
  return Math.round(edge === "in" ? pointerMs - startMs : endMs - pointerMs);
};

export const naturalTimelineBlockDurationMs = (
  authoredBeats: number | null,
  bpm: number,
  composerDefaultMs: number,
) => authoredBeats !== null && authoredBeats > 0 && Number.isFinite(bpm) && bpm > 0
  ? Math.max(1, Math.round(authoredBeats * 60_000 / bpm))
  : Math.max(1, Math.round(finiteOr(composerDefaultMs, 1_000)));
