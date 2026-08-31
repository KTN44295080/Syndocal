import type { TimelineVisibleWindow } from "./timelineViewport";

/** Identifiers are deliberately opaque: the snap layer must not invent IDs. */
export type TimelineSnapId = string | number;

export type TimelineSnapKind =
  | "grid"
  | "beat"
  | "bar"
  | "item-start"
  | "item-end"
  | "none";

export type TimelineQuantizerKind = "grid" | "beat" | "bar";

export interface TimelineSnapQuantizerPoint {
  time_ms: number;
  target_id?: TimelineSnapId | null;
  valid?: boolean;
}

export type TimelineSnapQuantizerInput = number | TimelineSnapQuantizerPoint;

/** Quantizer points are supplied by the caller so the snap layer stays tempo/grid agnostic. */
export interface TimelineSnapQuantizers {
  grid?: readonly TimelineSnapQuantizerInput[];
  beat?: readonly TimelineSnapQuantizerInput[];
  bar?: readonly TimelineSnapQuantizerInput[];
}

/** A visible block used only as an anchor. Locked blocks remain valid anchors. */
export interface TimelineSnapSceneBlock {
  id: TimelineSnapId;
  lane_id: TimelineSnapId;
  kind: string;
  start_ms: number;
  end_ms: number;
  /** Explicit invalid markers fail closed; omitted/true valid values are accepted. */
  valid?: boolean;
  invalid?: boolean;
  locked?: boolean;
}

export interface TimelineEdgeSnapInput {
  raw_time_ms: number;
  moving_item_id?: TimelineSnapId | null;
  moving_item_ids?: readonly TimelineSnapId[];
  target_lane_id: TimelineSnapId;
  target_kind: string;
  visible_window: TimelineVisibleWindow;
  canvas_width_px: number;
  quantizers?: TimelineSnapQuantizers;
  visible_scene_blocks?: readonly TimelineSnapSceneBlock[];
  threshold_px?: number;
}

export interface TimelineSnapResolution {
  time_ms: number;
  kind: TimelineSnapKind;
  target_id: TimelineSnapId | null;
  distance_ms: number;
  distance_px: number;
  threshold_ms: number;
}

export interface TimelineFadeBoundarySnapInput {
  item_start_ms: number;
  item_end_ms: number;
  edge: "in" | "out";
  raw_fade_ms: number;
  visible_window: TimelineVisibleWindow;
  canvas_width_px: number;
  quantizers?: TimelineSnapQuantizers;
  threshold_px?: number;
}

export interface TimelineFadeBoundarySnapResolution {
  boundary_ms: number;
  fade_ms: number;
  kind: Exclude<TimelineSnapKind, "item-start" | "item-end" | "none"> | "none";
  target_id: null;
  distance_ms: number;
  distance_px: number;
  threshold_ms: number;
}

const DEFAULT_THRESHOLD_PX = 8;
const EPSILON = 1e-7;

const finiteOr = (value: number, fallback: number) => Number.isFinite(value) ? value : fallback;
const clamp = (value: number, minimum: number, maximum: number) =>
  Math.min(maximum, Math.max(minimum, value));

const idKey = (id: TimelineSnapId) => `${typeof id}:${String(id)}`;
const sameId = (left: TimelineSnapId | null | undefined, right: TimelineSnapId | null | undefined) =>
  left != null && right != null && idKey(left) === idKey(right);

const normalizedWindow = (window: TimelineVisibleWindow) => {
  const start_ms = finiteOr(window.start_ms, 0);
  const end_ms = Math.max(start_ms, finiteOr(window.end_ms, start_ms));
  return { start_ms, end_ms, span_ms: Math.max(1, end_ms - start_ms) };
};

const snapThreshold = (window: TimelineVisibleWindow, canvasWidthPx: number, thresholdPx: number) => {
  const { span_ms } = normalizedWindow(window);
  const width = finiteOr(canvasWidthPx, 0);
  const px = Math.max(0, finiteOr(thresholdPx, DEFAULT_THRESHOLD_PX));
  return width > 0 ? (span_ms / width) * px : 0;
};

const pointInput = (point: TimelineSnapQuantizerInput): TimelineSnapQuantizerPoint =>
  typeof point === "number"
    ? { time_ms: point }
    : point && typeof point === "object"
      ? point
      : { time_ms: Number.NaN, valid: false };

interface InternalQuantizerCandidate {
  kind: TimelineQuantizerKind;
  time_ms: number;
  target_id: TimelineSnapId | null;
}

const quantizerPriority = (kind: TimelineQuantizerKind) => kind === "bar" ? 0 : kind === "beat" ? 1 : 2;

const validQuantizerCandidates = (
  quantizers: TimelineSnapQuantizers | undefined,
  window: TimelineVisibleWindow,
  minimumMs = Number.NEGATIVE_INFINITY,
  maximumMs = Number.POSITIVE_INFINITY,
): InternalQuantizerCandidate[] => {
  const { start_ms: windowStart, end_ms: windowEnd } = normalizedWindow(window);
  const lower = Math.max(windowStart, finiteOr(minimumMs, windowStart));
  const upper = Math.min(windowEnd, finiteOr(maximumMs, windowEnd));
  const result: InternalQuantizerCandidate[] = [];
  for (const kind of ["grid", "beat", "bar"] as const) {
    for (const rawPoint of quantizers?.[kind] ?? []) {
      const point = pointInput(rawPoint);
      if (point.valid === false || !Number.isFinite(point.time_ms)) continue;
      if (point.time_ms < lower - EPSILON || point.time_ms > upper + EPSILON) continue;
      result.push({ kind, time_ms: point.time_ms, target_id: point.target_id ?? null });
    }
  }
  return result;
};

const quantizerCandidateCompare = (left: InternalQuantizerCandidate, right: InternalQuantizerCandidate) =>
  quantizerPriority(left.kind) - quantizerPriority(right.kind) ||
  left.time_ms - right.time_ms ||
  idKey(left.target_id ?? "").localeCompare(idKey(right.target_id ?? ""));

const nearestQuantizer = (
  rawTimeMs: number,
  candidates: readonly InternalQuantizerCandidate[],
  thresholdMs: number,
) => {
  const candidatesInRange = candidates
    .map((candidate) => ({ candidate, distanceMs: Math.abs(candidate.time_ms - rawTimeMs) }))
    .filter(({ distanceMs }) => distanceMs <= thresholdMs + EPSILON)
    .sort((left, right) => left.distanceMs - right.distanceMs || quantizerCandidateCompare(left.candidate, right.candidate));
  return candidatesInRange[0] ?? null;
};

const itemCandidatePriority = (kind: "item-start" | "item-end") => kind === "item-start" ? 0 : 1;

/**
 * Resolve one absolute timeline edge. Item edges are magnetic anchors: once an
 * eligible same-lane/same-kind edge is inside the threshold it wins over a
 * quantizer point, and its exact time is returned without a second quantize.
 */
export const resolveTimelineEdgeSnap = (input: TimelineEdgeSnapInput): TimelineSnapResolution => {
  const window = normalizedWindow(input.visible_window);
  const rawTimeMs = finiteOr(input.raw_time_ms, 0);
  const thresholdMs = snapThreshold(input.visible_window, input.canvas_width_px, input.threshold_px ?? DEFAULT_THRESHOLD_PX);
  const movingIds = new Set(
    [input.moving_item_id ?? null, ...(input.moving_item_ids ?? [])]
      .filter((id): id is TimelineSnapId => id != null)
      .map(idKey),
  );
  const itemCandidates: Array<{
    kind: "item-start" | "item-end";
    timeMs: number;
    targetId: TimelineSnapId;
    distanceMs: number;
  }> = [];
  for (const block of input.visible_scene_blocks ?? []) {
    if (movingIds.has(idKey(block.id))) continue;
    if (block.valid === false || block.invalid === true) continue;
    if (block.kind !== input.target_kind || !sameId(block.lane_id, input.target_lane_id)) continue;
    if (!Number.isFinite(block.start_ms) || !Number.isFinite(block.end_ms) || block.end_ms < block.start_ms) continue;
    for (const [kind, timeMs] of [["item-start", block.start_ms], ["item-end", block.end_ms]] as const) {
      if (timeMs < window.start_ms - EPSILON || timeMs > window.end_ms + EPSILON) continue;
      const distanceMs = Math.abs(timeMs - rawTimeMs);
      if (distanceMs <= thresholdMs + EPSILON) {
        itemCandidates.push({ kind, timeMs, targetId: block.id, distanceMs });
      }
    }
  }
  itemCandidates.sort((left, right) =>
    left.distanceMs - right.distanceMs ||
    itemCandidatePriority(left.kind) - itemCandidatePriority(right.kind) ||
    left.timeMs - right.timeMs ||
    idKey(left.targetId).localeCompare(idKey(right.targetId)),
  );
  const item = itemCandidates[0];
  if (item) {
    return {
      time_ms: item.timeMs,
      kind: item.kind,
      target_id: item.targetId,
      distance_ms: item.distanceMs,
      distance_px: input.canvas_width_px > 0 ? item.distanceMs / window.span_ms * input.canvas_width_px : 0,
      threshold_ms: thresholdMs,
    };
  }

  const quantizer = nearestQuantizer(
    rawTimeMs,
    validQuantizerCandidates(input.quantizers, input.visible_window),
    thresholdMs,
  );
  if (quantizer) {
    const { candidate, distanceMs } = quantizer;
    return {
      time_ms: candidate.time_ms,
      kind: candidate.kind,
      target_id: candidate.target_id,
      distance_ms: distanceMs,
      distance_px: input.canvas_width_px > 0 ? distanceMs / window.span_ms * input.canvas_width_px : 0,
      threshold_ms: thresholdMs,
    };
  }
  return {
    time_ms: rawTimeMs,
    kind: "none",
    target_id: null,
    distance_ms: 0,
    distance_px: 0,
    threshold_ms: thresholdMs,
  };
};

/** Short alias for integrations that call all time snapping simply "snap". */
export const resolveTimelineSnap = resolveTimelineEdgeSnap;

/**
 * Snap a fade boundary in absolute time and convert it back to a bounded local
 * fade duration. Fade boundaries intentionally never inspect scene blocks, so
 * a neighbouring item's edge cannot unexpectedly change a fade gesture.
 */
export const resolveTimelineFadeBoundarySnap = (
  input: TimelineFadeBoundarySnapInput,
): TimelineFadeBoundarySnapResolution => {
  const rawStartMs = finiteOr(input.item_start_ms, 0);
  const rawEndMs = finiteOr(input.item_end_ms, rawStartMs);
  const startMs = Math.min(rawStartMs, rawEndMs);
  const endMs = Math.max(rawStartMs, rawEndMs);
  const durationMs = Math.max(0, endMs - startMs);
  const rawFadeMs = clamp(finiteOr(input.raw_fade_ms, 0), 0, durationMs);
  const rawBoundaryMs = input.edge === "in" ? startMs + rawFadeMs : endMs - rawFadeMs;
  const thresholdMs = snapThreshold(input.visible_window, input.canvas_width_px, input.threshold_px ?? DEFAULT_THRESHOLD_PX);
  const quantizer = nearestQuantizer(
    rawBoundaryMs,
    validQuantizerCandidates(input.quantizers, input.visible_window, startMs, endMs),
    thresholdMs,
  );
  const boundaryMs = quantizer?.candidate.time_ms ?? rawBoundaryMs;
  const fadeMs = clamp(input.edge === "in" ? boundaryMs - startMs : endMs - boundaryMs, 0, durationMs);
  const distanceMs = quantizer?.distanceMs ?? 0;
  return {
    boundary_ms: boundaryMs,
    fade_ms: fadeMs,
    kind: quantizer?.candidate.kind ?? "none",
    target_id: null,
    distance_ms: distanceMs,
    distance_px: input.canvas_width_px > 0
      ? distanceMs / normalizedWindow(input.visible_window).span_ms * input.canvas_width_px
      : 0,
    threshold_ms: thresholdMs,
  };
};

export const snapTimelineFadeBoundary = resolveTimelineFadeBoundarySnap;
