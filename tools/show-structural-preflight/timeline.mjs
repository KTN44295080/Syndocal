import {
  FINITE_LOOP_KEYS,
  FOLLOW_CURVES,
  FOLLOW_FAULT_POLICIES,
  FOLLOW_LIGHTING_POLICIES,
  FOLLOW_VIDEO_KINDS,
  SHOW_FOLLOW_BAR_MILLIUNITS,
  SHOW_RELEASE_TRIGGER,
  TIMELINE_FOLLOW_MAX_DURATION_BAR_MILLIUNITS,
  TIMELINE_FOLLOW_MAX_DURATION_BEAT_MILLIUNITS,
  TIMELINE_FOLLOW_MAX_PREROLL_MS,
  TIMELINE_FOLLOW_MAX_RESOLVED_DURATION_MS,
  TIMELINE_FOLLOW_MAX_TRANS_CADENCE_BARS,
  TIMELINE_FOLLOW_MAX_TRANS_TARGETS,
  TIMELINE_TEMPO_METER_MAX_MEASURE,
  TIMELINE_TEMPO_METER_MAX_POINTS,
  TIMELINE_TEMPO_METER_MAX_SIXTEENTH_STEPS,
  TIMELINE_TEMPO_METER_MAP_VERSION,
  hasOwn,
  isFinitePositiveNumber,
  isObject,
  isSafeNonNegativeInteger,
  isSafePositiveInteger,
  readOwn,
} from "./primitives.mjs";

function tempoPointShape(point, index) {
  if (!isObject(point)) return `tempo_meter_map[${index}] must be an object`;
  for (const field of ["position_sixteenth_steps", "bpm", "numerator", "denominator"]) {
    if (!hasOwn(point, field)) return `tempo_meter_map[${index}] is missing explicit ${field} evidence`;
  }
  if (!isSafeNonNegativeInteger(point.position_sixteenth_steps)) {
    return `tempo_meter_map[${index}] position_sixteenth_steps must be a safe non-negative integer`;
  }
  if (BigInt(point.position_sixteenth_steps) > TIMELINE_TEMPO_METER_MAX_SIXTEENTH_STEPS) {
    return `tempo_meter_map[${index}] exceeds the checked sixteenth-step range`;
  }
  if (typeof point.bpm !== "number" || !Number.isFinite(point.bpm) || point.bpm < 20 || point.bpm > 300) {
    return `tempo_meter_map[${index}] bpm must be finite and within 20..=300`;
  }
  if (!Number.isInteger(point.numerator) || point.numerator < 1 || point.numerator > 16) {
    return `tempo_meter_map[${index}] numerator must be within 1..=16`;
  }
  if (![1, 2, 4, 8, 16].includes(point.denominator)) {
    return `tempo_meter_map[${index}] denominator must be one of 1,2,4,8,16`;
  }
  if (hasOwn(point, "measure_number") && point.measure_number !== null
      && !isSafePositiveInteger(point.measure_number)) {
    return `tempo_meter_map[${index}] measure_number must be a positive safe integer when present`;
  }
  if (hasOwn(point, "interpolation") && !["Step", "Linear"].includes(point.interpolation)) {
    return `tempo_meter_map[${index}] interpolation is unsupported`;
  }
  return null;
}

function measureSpanSixteenthSteps(point) {
  const numerator = point.numerator * 16;
  if (numerator % point.denominator !== 0) return null;
  return numerator / point.denominator;
}

function timelinePointQuarterBeats(point) {
  return point.position_sixteenth_steps / 4;
}

function integrateTempoSegment(startBpm, endBpm, quarterBeats, interpolation) {
  if (!Number.isFinite(startBpm) || !Number.isFinite(endBpm)
      || !Number.isFinite(quarterBeats) || quarterBeats < 0) return null;
  if (quarterBeats === 0) return 0;
  if (interpolation !== "Linear" || Math.abs(endBpm - startBpm) < Number.EPSILON) {
    return 60 * quarterBeats / startBpm;
  }
  const slope = (endBpm - startBpm) / quarterBeats;
  const ratio = endBpm / startBpm;
  if (slope === 0 || ratio <= 0) return null;
  return 60 * Math.log(ratio) / slope;
}

function exactSecondsToQuarterBeat(points, targetQuarterBeats) {
  let elapsed = 0;
  for (const [index, point] of points.entries()) {
    const start = timelinePointQuarterBeats(point);
    if (targetQuarterBeats <= start) break;
    const next = points[index + 1];
    const end = next ? timelinePointQuarterBeats(next) : targetQuarterBeats;
    const segmentEnd = Math.min(targetQuarterBeats, end);
    const span = segmentEnd - start;
    const endBpm = next && point.interpolation === "Linear"
      ? point.bpm + (next.bpm - point.bpm) * (span / Math.max(end - start, Number.EPSILON))
      : point.bpm;
    const seconds = integrateTempoSegment(
      point.bpm,
      endBpm,
      span,
      point.interpolation ?? "Step",
    );
    if (seconds === null || !Number.isFinite(seconds)) return null;
    elapsed += seconds;
    if (segmentEnd >= targetQuarterBeats) break;
  }
  return Number.isFinite(elapsed) ? elapsed : null;
}

/**
 * Validate the explicit authored map and return the first complete measure.
 * No fallback to snapshot.clock or an assumed 4/4 is allowed here: a missing
 * map is evidence failure for this show contract.
 */
export function firstMeasureEvidence(timeline, label) {
  if (!isObject(timeline)) {
    return { error: `${label} Timeline is missing` };
  }
  const points = timeline.tempo_meter_map;
  if (!Array.isArray(points) || points.length === 0) {
    return { error: `${label} tempo_meter_map is missing; measure/tempo evidence is BLOCKED` };
  }
  const version = timeline.tempo_meter_map_version === undefined
    ? TIMELINE_TEMPO_METER_MAP_VERSION
    : timeline.tempo_meter_map_version;
  if (version !== TIMELINE_TEMPO_METER_MAP_VERSION) {
    return { error: `${label} tempo_meter_map_version ${String(version)} is unsupported` };
  }
  if (points.length > TIMELINE_TEMPO_METER_MAX_POINTS) {
    return {
      error: `${label} tempo_meter_map contains ${points.length} points; the limit is ${TIMELINE_TEMPO_METER_MAX_POINTS}`,
    };
  }
  const errors = [];
  let previousPosition = null;
  let previousMeasureNumber = null;
  let currentMeasureNumber = 1;
  let lastMeasureBoundary = 0;
  let currentNumerator = 4;
  let currentDenominator = 4;
  for (const [index, point] of points.entries()) {
    const shapeError = tempoPointShape(point, index);
    if (shapeError) errors.push(`${label} ${shapeError}`);
    if (shapeError) continue;
    if (previousPosition !== null && point.position_sixteenth_steps <= previousPosition) {
      errors.push(`${label} tempo_meter_map positions must increase strictly`);
    }
    previousPosition = point.position_sixteenth_steps;

    const hasMeasureNumber = hasOwn(point, "measure_number")
      && point.measure_number !== null
      && point.measure_number !== undefined;
    const measureSteps = currentNumerator * 16 / currentDenominator;
    const delta = point.position_sixteenth_steps - lastMeasureBoundary;
    const atMeasureBoundary = delta % measureSteps === 0;
    const meterChanges = point.numerator !== currentNumerator
      || point.denominator !== currentDenominator;
    if ((meterChanges || hasMeasureNumber) && !atMeasureBoundary) {
      errors.push(`${label} tempo/meter map changes meter/measure number away from an exact measure boundary`);
    }
    if (atMeasureBoundary) {
      const elapsedMeasures = delta / measureSteps;
      const expectedMeasure = currentMeasureNumber + elapsedMeasures;
      if (hasMeasureNumber) {
        if (previousMeasureNumber !== null && point.measure_number <= previousMeasureNumber) {
          errors.push(`${label} tempo/meter measure numbers must increase strictly at point ${index}`);
        }
        if (index > 0 && point.measure_number !== expectedMeasure) {
          errors.push(`${label} tempo/meter point ${index} measure number ${point.measure_number} does not match expected boundary ${expectedMeasure}`);
        }
        currentMeasureNumber = point.measure_number;
        previousMeasureNumber = point.measure_number;
      } else {
        currentMeasureNumber = expectedMeasure;
      }
      lastMeasureBoundary = point.position_sixteenth_steps;
    } else if (hasMeasureNumber) {
      errors.push(`${label} tempo/meter point ${index} has an ambiguous measure number`);
    }
    if (meterChanges) {
      currentNumerator = point.numerator;
      currentDenominator = point.denominator;
      lastMeasureBoundary = point.position_sixteenth_steps;
    }
    if (point.interpolation === "Linear" && index === points.length - 1) {
      errors.push(`${label} final tempo/meter point cannot request a linear slew without an end point`);
    }
  }
  if (errors.length > 0) return { error: errors.join("; ") };
  if (points[0].position_sixteenth_steps !== 0) {
    return { error: `${label} tempo_meter_map must explicitly start at position_sixteenth_steps=0` };
  }

  const first = points[0];
  const measureSpan = measureSpanSixteenthSteps(first);
  if (!measureSpan || !isSafePositiveInteger(measureSpan)) {
    return { error: `${label} first meter does not resolve to a finite complete measure` };
  }
  const firstMeasureEndSteps = measureSpan;
  const quarterBeats = first.numerator * 4 / first.denominator;
  const seconds = exactSecondsToQuarterBeat(points, quarterBeats);
  const durationMs = seconds === null ? Number.NaN : seconds * 1_000;
  if (!Number.isFinite(durationMs) || durationMs <= 0 || !Number.isSafeInteger(Math.round(durationMs))) {
    return { error: `${label} first meter-aware measure duration is not finite/valid` };
  }
  return {
    point: first,
    measureSpan,
    firstMeasureEndSteps,
    quarterBeats,
    durationMs: Math.round(durationMs),
  };
}

function forbiddenFiniteLoopFields(value) {
  if (!isObject(value)) return [];
  return FINITE_LOOP_KEYS.filter((key) => hasOwn(value, key));
}

export function validateLoopIntent(timeline) {
  const loop = timeline?.loop_region;
  if (!isObject(loop)) return { error: "source loop_region is missing" };
  if (loop.enabled !== true) return { error: "source loop_region.enabled must be true" };
  if (!isSafeNonNegativeInteger(loop.a_ms) || !isSafeNonNegativeInteger(loop.b_ms)) {
    return { error: "source loop_region A/B positions must be finite safe integers" };
  }
  if (loop.b_ms <= loop.a_ms) return { error: "source C-melody loop_region must be non-empty (b_ms > a_ms)" };
  if (!isSafePositiveInteger(timeline.duration_ms)) {
    return { error: "source Timeline bank duration_ms must be finite and positive" };
  }
  if (loop.b_ms > timeline.duration_ms) {
    return { error: "source C-melody loop_region must fit within the finite authored Timeline duration" };
  }
  if (!isFinitePositiveNumber(loop.musical_length_beats)) {
    return { error: "source loop_region.musical_length_beats must be finite and positive" };
  }
  const finiteKeys = forbiddenFiniteLoopFields(loop);
  if (finiteKeys.length > 0) {
    return { error: `source loop_region contains finite repeat-count field(s): ${finiteKeys.join(", ")}` };
  }
  const repeatMode = readOwn(loop, "repeatMode", "repeat_mode");
  if (repeatMode !== undefined && repeatMode !== "indefinite") {
    return { error: "source C-melody loop repeatMode must be indefinite" };
  }
  const releaseTrigger = readOwn(loop, "releaseTrigger", "release_trigger");
  if (releaseTrigger !== undefined && releaseTrigger !== SHOW_RELEASE_TRIGGER) {
    return { error: "source C-melody loop releaseTrigger must be F13" };
  }
  const automaticRelease = readOwn(loop, "automaticRelease", "automatic_release");
  if (automaticRelease !== undefined && automaticRelease !== false) {
    return { error: "source C-melody loop automaticRelease must be false" };
  }
  return {
    durationMs: loop.b_ms - loop.a_ms,
    detail: "non-empty authored A-B loop has no finite repeat-count semantics; runtime release control is outside this persisted project evidence",
  };
}

export function followEnumError(follow, label) {
  if (!isObject(follow)) return null;
  if (hasOwn(follow, "curve") && !FOLLOW_CURVES.includes(follow.curve)) {
    return `${label} curve must be an exact Rust serde value: Linear, EaseIn, EaseOut, or EaseInOut`;
  }
  if (hasOwn(follow, "video_kind") && !FOLLOW_VIDEO_KINDS.includes(follow.video_kind)) {
    return `${label} video_kind must be an exact Rust serde value: Cut, Crossfade, Dip, Wipe, Luma, Displacement, Blur, Glitch, or Custom`;
  }
  if (hasOwn(follow, "lighting_policy")
      && !FOLLOW_LIGHTING_POLICIES.includes(follow.lighting_policy)) {
    return `${label} lighting_policy must be a Rust serde value: hold_then_cut or linear_merge`;
  }
  if (hasOwn(follow, "fault_policy")
      && !FOLLOW_FAULT_POLICIES.includes(follow.fault_policy)) {
    return `${label} fault_policy must be a Rust serde value: hold, cut, or fault`;
  }
  return null;
}

export function followTimingError(follow, timeline, label) {
  if (!isObject(follow)) return null;
  const nextTimelineId = readOwn(follow, "next_timeline_id");
  if (!isSafePositiveInteger(nextTimelineId) || nextTimelineId === timeline?.id) {
    return `${label} Follow requires a different non-zero next Timeline ID`;
  }

  const duration = follow.duration === undefined
    ? { unit: "Milliseconds", value_milliunits: 0 }
    : follow.duration;
  const durationLimit = duration?.unit === "Milliseconds"
    ? TIMELINE_FOLLOW_MAX_RESOLVED_DURATION_MS
    : duration?.unit === "Beats"
      ? TIMELINE_FOLLOW_MAX_DURATION_BEAT_MILLIUNITS
      : duration?.unit === "Bars"
        ? TIMELINE_FOLLOW_MAX_DURATION_BAR_MILLIUNITS
        : null;
  const durationValue = duration?.value_milliunits;
  const videoKind = follow.video_kind ?? "Cut";
  const cut = videoKind === "Cut";
  const cadence = follow.trans_cadence_bars === undefined ? 4 : follow.trans_cadence_bars;
  const destinationBpm = follow.destination_bpm;
  const prerollMs = follow.preroll_ms === undefined ? 0 : follow.preroll_ms;
  if (!Number.isSafeInteger(cadence)
      || cadence < 1
      || cadence > TIMELINE_FOLLOW_MAX_TRANS_CADENCE_BARS
      || (destinationBpm !== undefined
        && destinationBpm !== null
        && (typeof destinationBpm !== "number"
          || !Number.isFinite(destinationBpm)
          || destinationBpm < 20
          || destinationBpm > 300))
      || !isSafeNonNegativeInteger(durationValue)
      || durationLimit === null
      || durationValue > durationLimit
      || !isSafeNonNegativeInteger(prerollMs)
      || prerollMs > TIMELINE_FOLLOW_MAX_PREROLL_MS
      || (timeline?.duration_ms > 0 && prerollMs > timeline.duration_ms)
      || (cut && (durationValue !== 0 || prerollMs !== 0))
      || (!cut && durationValue === 0)) {
    if (cut && (durationValue !== 0 || prerollMs !== 0)) {
      return `${label} Follow must be non-Cut when it declares a transition duration`;
    }
    return `${label} Follow has invalid timing, preroll, BPM, or Guide cadence`;
  }

  const targets = follow.trans_target_measures === undefined
    ? []
    : follow.trans_target_measures;
  if (!Array.isArray(targets)
      || targets.length > TIMELINE_FOLLOW_MAX_TRANS_TARGETS
      || targets.some((measure) =>
        !isSafePositiveInteger(measure)
        || BigInt(measure) > TIMELINE_TEMPO_METER_MAX_MEASURE,
      )
      || targets.some((measure, index) => index > 0 && measure <= targets[index - 1])) {
    return `${label} Follow Trans target measures must be bounded, non-zero, sorted, and unique`;
  }
  if (targets.length > 0) {
    const authoredMeasures = (Array.isArray(timeline?.tempo_meter_map)
      ? timeline.tempo_meter_map
      : [])
      .filter((point) => hasOwn(point, "measure_number") && point.measure_number !== null)
      .map((point) => point.measure_number);
    if (authoredMeasures.length === 0) {
      return `${label} Follow exact Trans targets require authored measure anchors`;
    }
    const minimum = Math.min(...authoredMeasures);
    const maximum = Math.max(...authoredMeasures);
    if (targets.some((measure) => measure < minimum || measure > maximum)) {
      return `${label} Follow exact Trans target is outside the authored measure range`;
    }
  }
  return null;
}

export function followIntent(source, destinationId) {
  const follow = source?.follow;
  if (!isObject(follow)) return { error: "source Timeline follow is missing" };
  if (follow.enabled !== true) return { error: "source Timeline Follow must be enabled" };
  if (readOwn(follow, "next_timeline_id") !== destinationId) {
    return { error: "source Follow destination ID does not match the explicit destination ID" };
  }
  if (follow.video_kind === "Cut") return { error: "source Follow must be non-Cut" };
  if (!FOLLOW_VIDEO_KINDS.includes(follow.video_kind)) {
    return {
      error: "source Follow video_kind must be an exact Rust serde value: Crossfade, Dip, Wipe, Luma, Displacement, Blur, Glitch, Custom (Cut is not allowed)",
    };
  }
  const enumError = followEnumError(follow, "source Follow");
  if (enumError) return { error: enumError };
  if (!hasOwn(follow, "destination_bpm")
      || (follow.destination_bpm !== null
        && (typeof follow.destination_bpm !== "number"
          || !Number.isFinite(follow.destination_bpm)
          || follow.destination_bpm < 20
          || follow.destination_bpm > 300))) {
    return { error: "source Follow destination_bpm must be explicitly authored as null or a value within 20..=300" };
  }
  if (!hasOwn(follow, "preroll_ms") || !isSafeNonNegativeInteger(follow.preroll_ms)) {
    return { error: "source Follow preroll_ms must be explicitly authored as a non-negative safe integer" };
  }
  if (!hasOwn(follow, "trans_cadence_bars")
      || !Number.isSafeInteger(follow.trans_cadence_bars)
      || follow.trans_cadence_bars < 1
      || follow.trans_cadence_bars > TIMELINE_FOLLOW_MAX_TRANS_CADENCE_BARS) {
    return { error: "source Follow trans_cadence_bars must be an authored integer within 1..=256" };
  }
  const timingError = followTimingError(follow, source, "source");
  if (timingError) return { error: timingError };
  const duration = follow.duration;
  if (!isObject(duration) || duration.unit !== "Bars" || duration.value_milliunits !== SHOW_FOLLOW_BAR_MILLIUNITS) {
    return { error: "source Follow must declare exactly one meter-aware bar (duration unit Bars, value_milliunits 1000)" };
  }
  if (follow.hold_first_destination_measure !== true) {
    return { error: "source Follow hold_first_destination_measure must be true" };
  }
  const finiteKeys = forbiddenFiniteLoopFields(follow);
  if (finiteKeys.length > 0) {
    return { error: `destination hold Follow contains finite repeat-count field(s): ${finiteKeys.join(", ")}` };
  }
  const repeatMode = readOwn(follow, "repeatMode", "repeat_mode");
  if (repeatMode !== undefined && repeatMode !== "indefinite") {
    return { error: "destination first-measure hold repeatMode must be indefinite" };
  }
  const releaseTrigger = readOwn(follow, "releaseTrigger", "release_trigger");
  if (releaseTrigger !== undefined && releaseTrigger !== SHOW_RELEASE_TRIGGER) {
    return { error: "destination first-measure hold releaseTrigger must be F13" };
  }
  const automaticRelease = readOwn(follow, "automaticRelease", "automatic_release");
  if (automaticRelease !== undefined && automaticRelease !== false) {
    return { error: "destination first-measure hold automaticRelease must be false" };
  }
  return { follow, detail: "enabled non-Cut Follow declares one source meter-aware bar and the destination hold flag" };
}
