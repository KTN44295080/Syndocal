export interface TimelineVisibleWindow {
  start_ms: number;
  end_ms: number;
}

export type TimelineViewportMode = "fit" | "manual";

export interface TimelineViewportState {
  show_duration_ms: number;
  edit_extent_ms: number;
  visible_window: TimelineVisibleWindow;
  mode: TimelineViewportMode;
}

export interface TimelineViewportReconcileOptions {
  project_replaced?: boolean;
}

export interface TimelineRulerTick {
  time_ms: number;
  ratio: number;
  label: string;
  major: boolean;
}

export type TimelineGridLineKind = "major" | "minor";

export interface TimelineGridLine {
  time_ms: number;
  ratio: number;
  /** Pixel position in the measured timeline canvas. */
  x_px: number;
  kind: TimelineGridLineKind;
  major: boolean;
  /** Major lines carry labels; minor lines intentionally do not. */
  label: string | null;
}

export interface TimelineGridModel {
  visible_window: TimelineVisibleWindow;
  pixel_width: number;
  major_step_ms: number;
  minor_step_ms: number;
  subdivision_count: number;
  lines: TimelineGridLine[];
}

export interface TimelineGridModelOptions {
  /** Approximate minimum distance between adjacent labelled major lines. */
  minimum_major_spacing_px?: number;
  /** Approximate minimum distance between minor lines. */
  minimum_minor_spacing_px?: number;
  /** Hard upper bound for generated SVG/CSS grid lines. */
  maximum_lines?: number;
}

export const TIMELINE_GRID_MIN_MAJOR_SPACING_PX = 72;
export const TIMELINE_GRID_MIN_MINOR_SPACING_PX = 14;
export const TIMELINE_GRID_MAX_LINES = 256;

export interface TimelineAbsoluteDragProjection {
  original_time_ms: number;
  time_ms: number;
  start_client_x: number;
  moved: boolean;
}

export const TIMELINE_MIN_VISIBLE_WINDOW_MS = 5_000;
export const TIMELINE_PAN_FRACTION = 0.8;
export const TIMELINE_MAX_RULER_TICKS = 64;

const finiteOr = (value: number, fallback: number) => Number.isFinite(value) ? value : fallback;
const clamp = (value: number, minimum: number, maximum: number) =>
  Math.min(maximum, Math.max(minimum, value));

const normalizedExtentMs = (extentMs: number, minimumSpanMs: number) =>
  Math.max(1, finiteOr(extentMs, minimumSpanMs), minimumSpanMs);

const normalizedShowDurationMs = (showDurationMs: number) =>
  Math.max(1, finiteOr(showDurationMs, 1));

export const timelineVisibleWindowSpanMs = (window: TimelineVisibleWindow) =>
  Math.max(1, finiteOr(window.end_ms, 1) - finiteOr(window.start_ms, 0));

export const beginTimelineAbsoluteDragProjection = (
  originalTimeMs: number,
  clientX: number,
): TimelineAbsoluteDragProjection => ({
  original_time_ms: Math.max(0, finiteOr(originalTimeMs, 0)),
  time_ms: Math.max(0, finiteOr(originalTimeMs, 0)),
  start_client_x: finiteOr(clientX, 0),
  moved: false,
});

export const updateTimelineAbsoluteDragProjection = (
  drag: TimelineAbsoluteDragProjection,
  clientX: number,
  pixelWidth: number,
  visibleSpanMs: number,
  minimumTimeMs = 0,
  maximumTimeMs = Number.POSITIVE_INFINITY,
  thresholdPx = 4,
): TimelineAbsoluteDragProjection => {
  const safeClientX = finiteOr(clientX, drag.start_client_x);
  const deltaPx = safeClientX - drag.start_client_x;
  const moved = drag.moved || Math.abs(deltaPx) >= Math.max(0, finiteOr(thresholdPx, 4));
  if (!moved) return { ...drag, moved: false, time_ms: drag.original_time_ms };
  const safePixelWidth = Math.max(1, finiteOr(pixelWidth, 1));
  const safeVisibleSpanMs = Math.max(1, finiteOr(visibleSpanMs, 1));
  const safeMinimumTimeMs = finiteOr(minimumTimeMs, 0);
  const safeMaximumTimeMs = Math.max(
    safeMinimumTimeMs,
    Number.isFinite(maximumTimeMs) ? maximumTimeMs : Number.POSITIVE_INFINITY,
  );
  const requestedTimeMs = drag.original_time_ms + (deltaPx / safePixelWidth) * safeVisibleSpanMs;
  return {
    ...drag,
    moved,
    time_ms: Math.min(safeMaximumTimeMs, Math.max(safeMinimumTimeMs, requestedTimeMs)),
  };
};

export const normalizeTimelineVisibleWindow = (
  window: TimelineVisibleWindow,
  extentMs: number,
  minimumSpanMs = TIMELINE_MIN_VISIBLE_WINDOW_MS,
): TimelineVisibleWindow => {
  const safeMinimumSpanMs = Math.max(1, finiteOr(minimumSpanMs, TIMELINE_MIN_VISIBLE_WINDOW_MS));
  const safeExtentMs = normalizedExtentMs(extentMs, safeMinimumSpanMs);
  const requestedStartMs = finiteOr(window.start_ms, 0);
  const requestedEndMs = finiteOr(window.end_ms, requestedStartMs + safeMinimumSpanMs);
  const requestedSpanMs = Math.max(1, requestedEndMs - requestedStartMs);
  const spanMs = clamp(requestedSpanMs, safeMinimumSpanMs, safeExtentMs);
  const startMs = clamp(requestedStartMs, 0, Math.max(0, safeExtentMs - spanMs));
  return {
    start_ms: startMs,
    end_ms: startMs + spanMs,
  };
};

export const fitTimelineVisibleWindow = (
  extentMs: number,
  minimumSpanMs = TIMELINE_MIN_VISIBLE_WINDOW_MS,
): TimelineVisibleWindow => {
  const safeMinimumSpanMs = Math.max(1, finiteOr(minimumSpanMs, TIMELINE_MIN_VISIBLE_WINDOW_MS));
  return {
    start_ms: 0,
    end_ms: normalizedExtentMs(extentMs, safeMinimumSpanMs),
  };
};

export const createTimelineViewportState = (
  showDurationMs: number,
  minimumSpanMs = TIMELINE_MIN_VISIBLE_WINDOW_MS,
): TimelineViewportState => {
  const safeShowDurationMs = normalizedShowDurationMs(showDurationMs);
  const visibleWindow = fitTimelineVisibleWindow(safeShowDurationMs, minimumSpanMs);
  return {
    show_duration_ms: safeShowDurationMs,
    edit_extent_ms: visibleWindow.end_ms + Math.max(1, finiteOr(minimumSpanMs, TIMELINE_MIN_VISIBLE_WINDOW_MS)),
    visible_window: visibleWindow,
    mode: "fit",
  };
};

export const reconcileTimelineViewportState = (
  current: TimelineViewportState,
  showDurationMs: number,
  options: TimelineViewportReconcileOptions = {},
  minimumSpanMs = TIMELINE_MIN_VISIBLE_WINDOW_MS,
): TimelineViewportState => {
  const safeShowDurationMs = normalizedShowDurationMs(showDurationMs);
  const fitWindow = fitTimelineVisibleWindow(safeShowDurationMs, minimumSpanMs);
  const editExtentMs = fitWindow.end_ms + Math.max(
    1,
    finiteOr(minimumSpanMs, TIMELINE_MIN_VISIBLE_WINDOW_MS),
  );
  const showDurationShrank = safeShowDurationMs < current.show_duration_ms;
  const mode = options.project_replaced || showDurationShrank ? "fit" : current.mode;
  const visibleWindow = mode === "fit"
    ? fitWindow
    : normalizeTimelineVisibleWindow(current.visible_window, editExtentMs, minimumSpanMs);
  if (
    current.show_duration_ms === safeShowDurationMs &&
    current.edit_extent_ms === editExtentMs &&
    current.mode === mode &&
    current.visible_window.start_ms === visibleWindow.start_ms &&
    current.visible_window.end_ms === visibleWindow.end_ms
  ) {
    return current;
  }
  return {
    show_duration_ms: safeShowDurationMs,
    edit_extent_ms: editExtentMs,
    visible_window: visibleWindow,
    mode,
  };
};

export const zoomTimelineVisibleWindow = (
  window: TimelineVisibleWindow,
  extentMs: number,
  scale: number,
  anchorTimeMs: number,
  minimumSpanMs = TIMELINE_MIN_VISIBLE_WINDOW_MS,
): TimelineVisibleWindow => {
  const normalizedWindow = normalizeTimelineVisibleWindow(window, extentMs, minimumSpanMs);
  const safeExtentMs = normalizedExtentMs(extentMs, Math.max(1, minimumSpanMs));
  const currentSpanMs = timelineVisibleWindowSpanMs(normalizedWindow);
  const safeScale = finiteOr(scale, 1) > 0 ? scale : 1;
  const nextSpanMs = clamp(currentSpanMs * safeScale, Math.max(1, minimumSpanMs), safeExtentMs);
  const fallbackAnchorMs = normalizedWindow.start_ms + currentSpanMs / 2;
  const safeAnchorMs = clamp(
    finiteOr(anchorTimeMs, fallbackAnchorMs),
    normalizedWindow.start_ms,
    normalizedWindow.end_ms,
  );
  const anchorRatio = (safeAnchorMs - normalizedWindow.start_ms) / currentSpanMs;
  const nextStartMs = safeAnchorMs - anchorRatio * nextSpanMs;
  return normalizeTimelineVisibleWindow(
    { start_ms: nextStartMs, end_ms: nextStartMs + nextSpanMs },
    safeExtentMs,
    minimumSpanMs,
  );
};

export const panTimelineVisibleWindow = (
  window: TimelineVisibleWindow,
  extentMs: number,
  direction: -1 | 1,
  panFraction = TIMELINE_PAN_FRACTION,
  minimumSpanMs = TIMELINE_MIN_VISIBLE_WINDOW_MS,
): TimelineVisibleWindow => {
  const normalizedWindow = normalizeTimelineVisibleWindow(window, extentMs, minimumSpanMs);
  const spanMs = timelineVisibleWindowSpanMs(normalizedWindow);
  const safeDirection = direction < 0 ? -1 : 1;
  const safePanFraction = Math.max(0, finiteOr(panFraction, TIMELINE_PAN_FRACTION));
  const offsetMs = safeDirection * spanMs * safePanFraction;
  return normalizeTimelineVisibleWindow(
    {
      start_ms: normalizedWindow.start_ms + offsetMs,
      end_ms: normalizedWindow.end_ms + offsetMs,
    },
    extentMs,
    minimumSpanMs,
  );
};

export const revealTimelineVisibleRange = (
  window: TimelineVisibleWindow,
  extentMs: number,
  rangeStartMs: number,
  rangeEndMs: number,
  minimumSpanMs = TIMELINE_MIN_VISIBLE_WINDOW_MS,
): TimelineVisibleWindow => {
  const normalizedWindow = normalizeTimelineVisibleWindow(window, extentMs, minimumSpanMs);
  const safeExtentMs = normalizedExtentMs(extentMs, Math.max(1, minimumSpanMs));
  const startMs = Math.max(0, finiteOr(rangeStartMs, 0));
  const endMs = Math.max(startMs, finiteOr(rangeEndMs, startMs));
  const rangeSpanMs = endMs - startMs;

  if (rangeSpanMs <= 0) {
    const currentSpanMs = timelineVisibleWindowSpanMs(normalizedWindow);
    return normalizeTimelineVisibleWindow(
      {
        start_ms: startMs - currentSpanMs / 2,
        end_ms: startMs + currentSpanMs / 2,
      },
      safeExtentMs,
      minimumSpanMs,
    );
  }

  if (rangeSpanMs >= safeExtentMs) return fitTimelineVisibleWindow(safeExtentMs, minimumSpanMs);

  const desiredSpanMs = Math.min(
    safeExtentMs,
    Math.max(Math.max(1, minimumSpanMs), rangeSpanMs * 4),
  );
  const centerMs = startMs + rangeSpanMs / 2;
  return normalizeTimelineVisibleWindow(
    {
      start_ms: centerMs - desiredSpanMs / 2,
      end_ms: centerMs + desiredSpanMs / 2,
    },
    safeExtentMs,
    minimumSpanMs,
  );
};

export const timelineVisibleRatioToTimeMs = (
  ratio: number,
  window: TimelineVisibleWindow,
) => finiteOr(window.start_ms, 0) + finiteOr(ratio, 0) * timelineVisibleWindowSpanMs(window);

export const timelineTimeToVisibleRawRatio = (
  timeMs: number,
  window: TimelineVisibleWindow,
) => (finiteOr(timeMs, window.start_ms) - finiteOr(window.start_ms, 0)) / timelineVisibleWindowSpanMs(window);

// Alternate word order kept explicit for call sites that read from the time axis inward.
export const timelineRatioToVisibleTimeMs = timelineVisibleRatioToTimeMs;
export const timelineTimeToVisibleRatio = timelineTimeToVisibleRawRatio;

export const timelineRangeIntersectsVisibleWindow = (
  rangeStartMs: number,
  rangeEndMs: number,
  window: TimelineVisibleWindow,
) => {
  const normalizedWindowStartMs = Math.min(window.start_ms, window.end_ms);
  const normalizedWindowEndMs = Math.max(window.start_ms, window.end_ms);
  const safeRangeStartMs = finiteOr(rangeStartMs, normalizedWindowStartMs);
  const safeRangeEndMs = finiteOr(rangeEndMs, normalizedWindowStartMs);
  const normalizedRangeStartMs = Math.min(
    safeRangeStartMs,
    safeRangeEndMs,
  );
  const normalizedRangeEndMs = Math.max(
    safeRangeStartMs,
    safeRangeEndMs,
  );
  if (normalizedRangeStartMs === normalizedRangeEndMs) {
    return normalizedRangeStartMs >= normalizedWindowStartMs &&
      normalizedRangeStartMs <= normalizedWindowEndMs;
  }
  return normalizedRangeEndMs > normalizedWindowStartMs &&
    normalizedRangeStartMs < normalizedWindowEndMs;
};

/**
 * Closed-range visibility for point collections such as automation keyframes.
 * Unlike duration blocks, both endpoint keyframes remain visible when they sit
 * exactly on either edge of the viewport.
 */
export const timelineClosedRangeIntersectsVisibleWindow = (
  rangeStartMs: number,
  rangeEndMs: number,
  window: TimelineVisibleWindow,
) => {
  const normalizedWindowStartMs = Math.min(window.start_ms, window.end_ms);
  const normalizedWindowEndMs = Math.max(window.start_ms, window.end_ms);
  const safeRangeStartMs = finiteOr(rangeStartMs, normalizedWindowStartMs);
  const safeRangeEndMs = finiteOr(rangeEndMs, normalizedWindowStartMs);
  const normalizedRangeStartMs = Math.min(safeRangeStartMs, safeRangeEndMs);
  const normalizedRangeEndMs = Math.max(safeRangeStartMs, safeRangeEndMs);
  return normalizedRangeEndMs >= normalizedWindowStartMs &&
    normalizedRangeStartMs <= normalizedWindowEndMs;
};

export const niceTimelineRulerStepMs = (minimumStepMs: number) => {
  const safeMinimumStepMs = Math.max(Number.EPSILON, finiteOr(minimumStepMs, 1));
  const magnitude = 10 ** Math.floor(Math.log10(safeMinimumStepMs));
  const normalized = safeMinimumStepMs / magnitude;
  const multiplier = normalized <= 1 ? 1 : normalized <= 2 ? 2 : normalized <= 5 ? 5 : 10;
  return multiplier * magnitude;
};

export const formatTimelineRulerLabel = (timeMs: number, stepMs = 1_000) => {
  const safeTimeMs = Math.max(0, Math.round(finiteOr(timeMs, 0)));
  const milliseconds = safeTimeMs % 1_000;
  const totalSeconds = Math.floor(safeTimeMs / 1_000);
  const seconds = totalSeconds % 60;
  const totalMinutes = Math.floor(totalSeconds / 60);
  const minutes = totalMinutes % 60;
  const hours = Math.floor(totalMinutes / 60);
  const base = hours > 0
    ? `${hours}:${String(minutes).padStart(2, "0")}:${String(seconds).padStart(2, "0")}`
    : `${totalMinutes}:${String(seconds).padStart(2, "0")}`;
  return stepMs < 1_000 ? `${base}.${String(milliseconds).padStart(3, "0")}` : base;
};

export const buildTimelineRulerTicks = (
  window: TimelineVisibleWindow,
  pixelWidth: number,
  minimumSpacingPx = 80,
): TimelineRulerTick[] => {
  const startMs = Math.min(window.start_ms, window.end_ms);
  const endMs = Math.max(window.start_ms, window.end_ms);
  const spanMs = Math.max(1, endMs - startMs);
  const safePixelWidth = Math.max(1, finiteOr(pixelWidth, 1));
  const safeMinimumSpacingPx = Math.max(1, finiteOr(minimumSpacingPx, 80));
  const desiredIntervalCount = clamp(
    Math.floor(safePixelWidth / safeMinimumSpacingPx),
    1,
    TIMELINE_MAX_RULER_TICKS - 1,
  );
  const stepMs = niceTimelineRulerStepMs(spanMs / desiredIntervalCount);
  const epsilonMs = stepMs * 1e-9;
  const firstTickMs = Math.ceil((startMs - epsilonMs) / stepMs) * stepMs;
  const ticks: TimelineRulerTick[] = [];
  const majorStepMs = stepMs * 5;

  for (
    let timeMs = firstTickMs;
    timeMs <= endMs + epsilonMs && ticks.length < TIMELINE_MAX_RULER_TICKS;
    timeMs += stepMs
  ) {
    const normalizedTimeMs = Math.round(timeMs * 1_000) / 1_000;
    const majorRemainderMs = Math.abs(normalizedTimeMs % majorStepMs);
    ticks.push({
      time_ms: normalizedTimeMs,
      ratio: (normalizedTimeMs - startMs) / spanMs,
      label: formatTimelineRulerLabel(normalizedTimeMs, stepMs),
      major: majorRemainderMs < epsilonMs || Math.abs(majorRemainderMs - majorStepMs) < epsilonMs,
    });
  }

  return ticks;
};

const timelineGridSubdivisionCount = (
  majorStepMs: number,
  spanMs: number,
  pixelWidth: number,
  minimumMinorSpacingPx: number,
  maximumLines: number,
) => {
  const majorSpacingPx = pixelWidth * majorStepMs / spanMs;
  // Prefer the finest 1/2/5/10 subdivision that remains legible. When an
  // unusually wide canvas would otherwise exceed the line budget, step down
  // the subdivisions before coarsening the major unit.
  for (const subdivision of [10, 5, 2, 1]) {
    if (majorSpacingPx / subdivision + 1e-7 < minimumMinorSpacingPx) continue;
    if (spanMs / (majorStepMs / subdivision) + 2 <= maximumLines) return subdivision;
  }
  return 1;
};

/**
 * Build a bounded, zoom-stable timeline grid. Major units use the existing
 * 1-2-5 duration policy; minor lines are unlabelled and are only subdivided
 * while the measured canvas can keep them around the requested 14px apart.
 */
export const buildAdaptiveTimelineGrid = (
  window: TimelineVisibleWindow,
  pixelWidth: number,
  options: TimelineGridModelOptions = {},
): TimelineGridModel => {
  const startMs = Math.min(finiteOr(window.start_ms, 0), finiteOr(window.end_ms, 0));
  const endMs = Math.max(finiteOr(window.start_ms, 0), finiteOr(window.end_ms, 0));
  const spanMs = Math.max(1, endMs - startMs);
  const safePixelWidth = Math.max(1, finiteOr(pixelWidth, 1));
  const minimumMajorSpacingPx = Math.max(
    1,
    finiteOr(options.minimum_major_spacing_px ?? TIMELINE_GRID_MIN_MAJOR_SPACING_PX, TIMELINE_GRID_MIN_MAJOR_SPACING_PX),
  );
  const minimumMinorSpacingPx = Math.max(
    1,
    finiteOr(options.minimum_minor_spacing_px ?? TIMELINE_GRID_MIN_MINOR_SPACING_PX, TIMELINE_GRID_MIN_MINOR_SPACING_PX),
  );
  const maximumLines = Math.max(
    2,
    Math.floor(finiteOr(options.maximum_lines ?? TIMELINE_GRID_MAX_LINES, TIMELINE_GRID_MAX_LINES)),
  );
  const desiredMajorCount = Math.max(1, Math.floor(safePixelWidth / minimumMajorSpacingPx));
  let majorStepMs = niceTimelineRulerStepMs(spanMs / desiredMajorCount);
  let subdivisionCount = timelineGridSubdivisionCount(
    majorStepMs,
    spanMs,
    safePixelWidth,
    minimumMinorSpacingPx,
    maximumLines,
  );
  // The normal path is below the cap. For very large canvases, coarsen the
  // major unit deterministically until even one line per major is bounded.
  let guard = 0;
  while (spanMs / (majorStepMs / subdivisionCount) + 2 > maximumLines && guard < 32) {
    majorStepMs = niceTimelineRulerStepMs(majorStepMs * 2);
    subdivisionCount = timelineGridSubdivisionCount(
      majorStepMs,
      spanMs,
      safePixelWidth,
      minimumMinorSpacingPx,
      maximumLines,
    );
    guard += 1;
  }
  const minorStepMs = majorStepMs / subdivisionCount;
  // Only absorb floating-point representation error at a real endpoint. A
  // fixed percentage epsilon would incorrectly emit a visible line just
  // outside a fractional window (for example time 0 for a window starting at
  // 1e-9). The tolerance is deliberately many ulps, not a fraction of the
  // display unit.
  const endpointTolerance = Number.EPSILON * Math.max(
    1,
    Math.abs(startMs),
    Math.abs(endMs),
    Math.abs(minorStepMs),
  ) * 64;
  const firstIndex = Math.ceil(startMs / minorStepMs - endpointTolerance / minorStepMs);
  const lines: TimelineGridLine[] = [];
  for (
    let index = firstIndex;
    index * minorStepMs <= endMs + endpointTolerance && lines.length < maximumLines;
    index += 1
  ) {
    const rawTimeMs = index * minorStepMs;
    if (rawTimeMs < startMs - endpointTolerance) continue;
    if (rawTimeMs > endMs + endpointTolerance) break;
    // Rounding keeps ordinary 1-2-5 values stable in serialized snapshots;
    // clamping prevents that presentation rounding from violating the strict
    // visible-window contract at either edge.
    const roundedTimeMs = Math.round(rawTimeMs * 1e6) / 1e6;
    const timeMs = Math.abs(rawTimeMs - startMs) <= endpointTolerance
      ? startMs
      : Math.abs(rawTimeMs - endMs) <= endpointTolerance
        ? endMs
        : Math.min(endMs, Math.max(startMs, roundedTimeMs));
    const isMajor = Math.abs(index % subdivisionCount) === 0;
    const ratio = Math.min(1, Math.max(0, (timeMs - startMs) / spanMs));
    lines.push({
      time_ms: timeMs,
      ratio,
      x_px: Math.min(safePixelWidth, Math.max(0, ratio * safePixelWidth)),
      kind: isMajor ? "major" : "minor",
      major: isMajor,
      label: isMajor ? formatTimelineRulerLabel(timeMs, majorStepMs) : null,
    });
  }
  return {
    visible_window: { start_ms: startMs, end_ms: endMs },
    pixel_width: safePixelWidth,
    major_step_ms: majorStepMs,
    minor_step_ms: minorStepMs,
    subdivision_count: subdivisionCount,
    lines,
  };
};

/** Descriptive alias for call sites that refer to the model rather than grid. */
export const buildTimelineGridModel = buildAdaptiveTimelineGrid;
