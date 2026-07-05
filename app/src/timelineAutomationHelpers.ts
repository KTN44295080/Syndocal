import type { AutomationInterpolation, VideoLayerState, VideoParam } from "./types";

export interface TimelineKeyframeLike {
  time_ms: number;
}

export interface TimelineValueKeyframeLike extends TimelineKeyframeLike {
  value: number;
  interpolation: AutomationInterpolation;
}

const clampRange = (value: number, min: number, max: number) => Math.min(max, Math.max(min, value));

export const shiftedTimelineKeyframes = <T extends TimelineKeyframeLike>(keyframes: T[], nextStartMs: number): T[] | null => {
  if (keyframes.length === 0) {
    return null;
  }
  const currentStartMs = Math.min(...keyframes.map((keyframe) => keyframe.time_ms));
  const deltaMs = nextStartMs - currentStartMs;
  return keyframes.map((keyframe) => ({
    ...keyframe,
    time_ms: Math.max(0, Math.round(keyframe.time_ms + deltaMs)),
  }));
};

export const resizedTimelineKeyframes = <T extends TimelineKeyframeLike>(
  keyframes: T[],
  nextStartMs: number,
  nextEndMs: number,
): T[] | null => {
  if (keyframes.length === 0) {
    return null;
  }
  const currentStartMs = Math.min(...keyframes.map((keyframe) => keyframe.time_ms));
  const currentEndMs = Math.max(...keyframes.map((keyframe) => keyframe.time_ms));
  const nextDurationMs = Math.max(1, nextEndMs - nextStartMs);
  if (currentEndMs <= currentStartMs) {
    return keyframes.map((keyframe, index) => ({
      ...keyframe,
      time_ms: index === keyframes.length - 1 ? nextStartMs + nextDurationMs : nextStartMs,
    }));
  }
  const currentDurationMs = currentEndMs - currentStartMs;
  return keyframes.map((keyframe) => ({
    ...keyframe,
    time_ms: Math.max(
      0,
      Math.round(nextStartMs + ((keyframe.time_ms - currentStartMs) / currentDurationMs) * nextDurationMs),
    ),
  }));
};

export const sortedTimelineKeyframes = <T extends TimelineKeyframeLike>(keyframes: T[]): T[] =>
  [...keyframes].sort((left, right) => left.time_ms - right.time_ms);

export const timelineSegmentProgress = (
  startMs: number,
  endMs: number,
  positionMs: number,
  interpolation: AutomationInterpolation,
) => {
  if (interpolation === "Step") {
    return 0;
  }
  const spanMs = Math.max(0, endMs - startMs);
  const linear = spanMs === 0 ? 1 : clampRange((positionMs - startMs) / spanMs, 0, 1);
  return interpolation === "Bezier" ? linear * linear * (3 - 2 * linear) : linear;
};

export const evaluateTimelineKeyframes = <T extends TimelineValueKeyframeLike>(keyframes: T[], positionMs: number) => {
  const sorted = sortedTimelineKeyframes(keyframes);
  const first = sorted[0];
  if (!first) {
    return null;
  }
  if (positionMs <= first.time_ms) {
    return first.value;
  }
  for (let index = 0; index < sorted.length - 1; index += 1) {
    const current = sorted[index];
    const next = sorted[index + 1];
    if (positionMs < current.time_ms || positionMs > next.time_ms) {
      continue;
    }
    const progress = timelineSegmentProgress(current.time_ms, next.time_ms, positionMs, current.interpolation);
    return current.value + (next.value - current.value) * progress;
  }
  return sorted[sorted.length - 1]?.value ?? null;
};

export const interpolationForInsertedKeyframe = <T extends TimelineKeyframeLike & { interpolation: AutomationInterpolation }>(
  keyframes: T[],
  positionMs: number,
): AutomationInterpolation => {
  const sorted = sortedTimelineKeyframes(keyframes);
  const previous = [...sorted].reverse().find((keyframe) => keyframe.time_ms <= positionMs);
  return previous?.interpolation ?? sorted[0]?.interpolation ?? "Linear";
};

export const upsertTimelineKeyframe = <T extends TimelineKeyframeLike>(keyframes: T[], keyframe: T): T[] =>
  sortedTimelineKeyframes([...keyframes.filter((candidate) => candidate.time_ms !== keyframe.time_ms), keyframe]);

export const timelineKeyframeMoveTime = <T extends TimelineKeyframeLike>(
  keyframes: T[],
  keyframeIndex: number,
  nextTimeMs: number,
): number | null => {
  if (keyframeIndex < 0 || keyframeIndex >= keyframes.length) {
    return null;
  }
  const previousTimeMs = keyframes[keyframeIndex - 1]?.time_ms;
  const nextNeighborTimeMs = keyframes[keyframeIndex + 1]?.time_ms;
  const lowerBoundMs = previousTimeMs === undefined ? 0 : previousTimeMs + 1;
  const upperBoundMs = nextNeighborTimeMs === undefined ? Number.MAX_SAFE_INTEGER : Math.max(lowerBoundMs, nextNeighborTimeMs - 1);
  return Math.round(clampRange(nextTimeMs, lowerBoundMs, upperBoundMs));
};

export const movedTimelineKeyframe = <T extends TimelineKeyframeLike>(
  keyframes: T[],
  keyframeIndex: number,
  nextTimeMs: number,
): T[] | null => {
  const timeMs = timelineKeyframeMoveTime(keyframes, keyframeIndex, nextTimeMs);
  if (timeMs === null) {
    return null;
  }
  return sortedTimelineKeyframes(
    keyframes.map((keyframe, index) => (index === keyframeIndex ? { ...keyframe, time_ms: timeMs } : keyframe)),
  );
};

export const snappedTimelineKeyframes = <T extends TimelineKeyframeLike>(
  keyframes: T[],
  snapTimeMs: (timeMs: number) => number,
): T[] => {
  let previousTimeMs: number | null = null;
  return sortedTimelineKeyframes(keyframes).map((keyframe) => {
    const snappedTimeMs = Math.max(0, Math.round(snapTimeMs(keyframe.time_ms)));
    const timeMs = previousTimeMs === null ? snappedTimeMs : Math.max(snappedTimeMs, previousTimeMs + 1);
    previousTimeMs = timeMs;
    return {
      ...keyframe,
      time_ms: timeMs,
    };
  });
};

export const keyframesWithoutIndex = <T extends TimelineKeyframeLike>(
  keyframes: T[],
  keyframeIndex: number,
): { keyframes: T[]; keyframeIndex: number; timeMs: number } | null => {
  if (keyframes.length <= 2 || keyframeIndex < 0 || keyframeIndex >= keyframes.length) {
    return null;
  }
  const removed = keyframes[keyframeIndex];
  return {
    keyframes: sortedTimelineKeyframes(keyframes.filter((_, index) => index !== keyframeIndex)),
    keyframeIndex,
    timeMs: removed.time_ms,
  };
};

export const keyframesWithInterpolationAtIndex = <T extends TimelineValueKeyframeLike>(
  keyframes: T[],
  keyframeIndex: number,
  interpolation: AutomationInterpolation,
): T[] | null => {
  if (keyframeIndex < 0 || keyframeIndex >= keyframes.length) {
    return null;
  }
  return sortedTimelineKeyframes(
    keyframes.map((keyframe, index) => (index === keyframeIndex ? { ...keyframe, interpolation } : keyframe)),
  );
};

export const keyframesWithValueAtIndex = <T extends TimelineValueKeyframeLike>(
  keyframes: T[],
  keyframeIndex: number,
  value: number,
): T[] | null => {
  if (keyframeIndex < 0 || keyframeIndex >= keyframes.length || !Number.isFinite(value)) {
    return null;
  }
  return sortedTimelineKeyframes(keyframes.map((keyframe, index) => (index === keyframeIndex ? { ...keyframe, value } : keyframe)));
};

export const keyframesWithoutPlayheadKeyframe = <T extends TimelineKeyframeLike>(
  keyframes: T[],
  playheadMs: number,
  toleranceMs: number,
): { keyframes: T[]; keyframeIndex: number; timeMs: number } | null => {
  if (keyframes.length <= 2) {
    return null;
  }
  let nearestIndex = -1;
  let nearestDeltaMs = Number.POSITIVE_INFINITY;
  keyframes.forEach((keyframe, index) => {
    const deltaMs = Math.abs(keyframe.time_ms - playheadMs);
    if (deltaMs < nearestDeltaMs) {
      nearestDeltaMs = deltaMs;
      nearestIndex = index;
    }
  });
  if (nearestIndex < 0 || nearestDeltaMs > toleranceMs) {
    return null;
  }
  return keyframesWithoutIndex(keyframes, nearestIndex);
};

export const keyframesWithDraftEndpoints = <T extends TimelineValueKeyframeLike>(
  keyframes: T[],
  startMs: number,
  endMs: number,
  startValue: number,
  endValue: number,
  interpolation: AutomationInterpolation,
): T[] => {
  const source =
    keyframes.length >= 2
      ? keyframes
      : ([
          { time_ms: startMs, value: startValue, interpolation },
          { time_ms: endMs, value: endValue, interpolation: "Step" },
        ] as T[]);
  const resized = sortedTimelineKeyframes(resizedTimelineKeyframes(source, startMs, endMs) ?? source);
  const lastIndex = resized.length - 1;
  return resized.map((keyframe, index) => {
    if (index === 0) {
      return {
        ...keyframe,
        time_ms: startMs,
        value: startValue,
        interpolation,
      };
    }
    if (index === lastIndex) {
      return {
        ...keyframe,
        time_ms: endMs,
        value: endValue,
        interpolation: "Step",
      };
    }
    return keyframe;
  });
};

export const draftRangeFromKeyframes = (keyframes: TimelineKeyframeLike[]) => {
  const times = keyframes.map((keyframe) => keyframe.time_ms).sort((left, right) => left - right);
  return {
    start_ms: times[0] ?? 0,
    end_ms: times[times.length - 1] ?? times[0] ?? 0,
  };
};

export const videoAutomationValueFromState = (state: VideoLayerState, param: VideoParam) => {
  switch (param) {
    case "Opacity":
      return state.opacity;
    case "Speed":
      return state.speed;
    case "PositionMs":
      return state.position_ms;
    case "BpmSyncEnabled":
      return state.bpm_sync.enabled ? 1 : 0;
    case "BpmSyncRatio":
      return state.bpm_sync.ratio;
    case "BpmSyncLoopBars":
      return state.bpm_sync.loop_bars;
    case "TransformX":
      return state.transform.x;
    case "TransformY":
      return state.transform.y;
    case "TransformScaleX":
      return state.transform.scale_x;
    case "TransformScaleY":
      return state.transform.scale_y;
    case "TransformRotationDeg":
      return state.transform.rotation_deg;
    case "TransformCropLeft":
      return state.transform.crop_left;
    case "TransformCropTop":
      return state.transform.crop_top;
    case "TransformCropRight":
      return state.transform.crop_right;
    case "TransformCropBottom":
      return state.transform.crop_bottom;
    case "ColorBrightness":
      return state.color.brightness;
    case "ColorContrast":
      return state.color.contrast;
    case "ColorHueDeg":
      return state.color.hue_deg;
    case "ColorSaturation":
      return state.color.saturation;
    case "ColorGamma":
      return state.color.gamma;
    case "FxPixelate":
      return state.fx.pixelate;
    case "FxBlur":
      return state.fx.blur;
    case "FxGlow":
      return state.fx.glow;
    case "FxEdge":
      return state.fx.edge;
    case "FxKeyRed":
      return state.fx.key_red;
    case "FxKeyGreen":
      return state.fx.key_green;
    case "FxKeyBlue":
      return state.fx.key_blue;
    case "FxKeyThreshold":
      return state.fx.key_threshold;
  }
};
