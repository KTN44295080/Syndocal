import {
  evaluateLfoShape,
  sampleColorStops,
  sampleMovePath,
  transformMovePreview,
  movePointToPreview,
} from "./effectVisualization";
import { chaserDefaultSeed, chaserStepsFromTargets } from "./chaserDraft";
import { defaultMovePathPoints } from "./moveEffect";
import type {
  CueSummary,
  EffectParamsSnapshot,
  PatchedFixtureSummary,
} from "./types";

export const quickSceneFxFamilies = ["COLOR FX", "CHASER FX", "VALUE FX", "MOVE FX"] as const;
export type QuickSceneFxFamily = (typeof quickSceneFxFamilies)[number];

const normalizedAttribute = (attribute: string) =>
  attribute.toLowerCase().replace(/[^a-z0-9]/g, "");

const paramsFixtureIds = (params: EffectParamsSnapshot | null | undefined) => {
  if (!params) return [];
  if ("Chaser" in params) {
    return params.Chaser.steps.flatMap((step) => step.fixture_ids);
  }
  const request = Object.values(params)[0];
  return "fixture_ids" in request ? request.fixture_ids : [];
};

const paramsTargetGroupIds = (params: EffectParamsSnapshot | null | undefined) => {
  if (!params) return [];
  if ("Chaser" in params) {
    return params.Chaser.steps.flatMap((step) => step.target_group_ids);
  }
  const request = Object.values(params)[0];
  return "target_group_ids" in request ? request.target_group_ids : [];
};

export const sceneFxTargetFixtures = (
  cue: CueSummary,
  fixtures: PatchedFixtureSummary[],
) => {
  const fixtureIds = new Set(cue.targets.map((target) => target.fixture_id));
  const groupIds = new Set<string>();
  if (cue.group_id?.trim()) groupIds.add(cue.group_id.trim());
  for (const target of cue.effect_targets) {
    for (const fixtureId of paramsFixtureIds(target.params)) fixtureIds.add(fixtureId);
    for (const groupId of paramsTargetGroupIds(target.params)) groupIds.add(groupId);
  }
  for (const fixture of fixtures) {
    if (fixture.group_ids.some((groupId) => groupIds.has(groupId))) fixtureIds.add(fixture.id);
  }
  return fixtures.filter((fixture) => fixtureIds.has(fixture.id));
};

const pairedPanTilt = (fixture: PatchedFixtureSummary) => {
  const axes = fixture.controls.reduce((counts, control) => {
    const attribute = normalizedAttribute(control.attribute);
    if (attribute.includes("tilt")) counts.tilt += 1;
    else if (attribute.includes("pan")) counts.pan += 1;
    return counts;
  }, { pan: 0, tilt: 0 });
  return axes.pan === 1 && axes.tilt === 1;
};

export const sceneMoveFxAvailable = (
  cue: CueSummary,
  fixtures: PatchedFixtureSummary[],
) => sceneFxTargetFixtures(cue, fixtures).some(pairedPanTilt);

const sceneScalarAttribute = (fixtures: PatchedFixtureSummary[]) => {
  for (const fixture of fixtures) {
    const dimmer = fixture.controls.find((control) => {
      const attribute = normalizedAttribute(control.attribute);
      return attribute === "dimmer" || attribute === "intensity" || attribute === "masterintensity";
    });
    if (dimmer) return dimmer.attribute;
  }
  return fixtures[0]?.controls[0]?.attribute ?? "";
};

const rainbowStops = [
  [65_535, 0, 0],
  [0, 65_535, 0],
  [0, 0, 65_535],
].map(([red, green, blue], index, stops) => ({
  position: index / (stops.length - 1),
  color: { red, green, blue },
}));

export const defaultSceneFxParams = (
  family: QuickSceneFxFamily,
  cue: CueSummary,
  fixtures: PatchedFixtureSummary[],
): EffectParamsSnapshot | null => {
  const targets = sceneFxTargetFixtures(cue, fixtures);
  const fixtureIds = targets.map((fixture) => fixture.id);
  if (fixtureIds.length === 0) return null;
  const attribute = sceneScalarAttribute(targets);
  if (!attribute && family !== "COLOR FX" && family !== "MOVE FX") return null;
  if (family === "COLOR FX") {
    return {
      Color: {
        label: "Rainbow",
        fixture_ids: fixtureIds,
        target_group_ids: [],
        stops: rainbowStops,
        algorithm: "Cycle",
        interpolation: "Rgb",
        period_ms: 2_000,
        clock_sync: null,
        phase: 0,
        fixture_spread: fixtureIds.length > 1 ? 1 : 0,
        blend_mode: "Override",
        spatial_pattern: {
          recipe: {
            ColorRainbow: {
              grayscale: false,
              vertical_symmetry: false,
              color_width: 0,
              angle_degrees: 0,
              gradient: 100,
            },
          },
          beam_targets: [],
        },
      },
    };
  }
  if (family === "CHASER FX") {
    return {
      Chaser: {
        label: "Dimmer Chaser",
        steps: chaserStepsFromTargets(fixtureIds, [], true),
        features: [{ attribute, low: 0, high: 65_535 }],
        step_duration_ms: 250,
        clock_sync: null,
        direction: "Forward",
        wings: 1,
        active_step_count: 1,
        duty_cycle: 1,
        overlap: 0,
        phase: 0,
        fixture_spread: fixtureIds.length > 1 ? 1 : 0,
        random_seed: chaserDefaultSeed,
        blend_mode: "Override",
      },
    };
  }
  if (family === "VALUE FX") {
    return {
      Lfo: {
        label: "Dimmer Pulse",
        fixture_ids: fixtureIds,
        target_group_ids: [],
        attribute,
        video_targets: [],
        shape: "Square",
        period_ms: 1_000,
        clock_sync: null,
        low: 0,
        high: 65_535,
        phase: 0,
        fixture_spread: fixtureIds.length > 1 ? 1 : 0,
        blend_mode: "Override",
      },
    };
  }
  const moveFixtureIds = targets.filter(pairedPanTilt).map((fixture) => fixture.id);
  if (moveFixtureIds.length === 0) return null;
  return {
    Move: {
      label: "Pan/Tilt Circle",
      fixture_ids: moveFixtureIds,
      target_group_ids: [],
      points: defaultMovePathPoints(),
      closed: true,
      interpolation: "Smooth",
      coordinate_mode: "Absolute",
      center_x: 0.5,
      center_y: 0.5,
      size_x: 1,
      size_y: 1,
      rotation_degrees: 0,
      period_ms: 2_000,
      clock_sync: null,
      direction: "Forward",
      phase: 0,
      fixture_spread: moveFixtureIds.length > 1 ? 1 : 0,
      symmetry: false,
      blend_mode: "Override",
    },
  };
};

const setFixtureAttributes = (
  fixture: PatchedFixtureSummary,
  values: Record<string, number>,
) => ({
  ...fixture,
  attribute_values: fixture.controls.map((control) => ({
    attribute: control.attribute,
    value: Math.round(Math.max(0, Math.min(65_535,
      values[control.attribute]
      ?? fixture.attribute_values.find((entry) => entry.attribute === control.attribute)?.value
      ?? control.default_value,
    ))),
  })),
});

const cycleProgress = (elapsedMs: number, periodMs: number, phase = 0) =>
  ((elapsedMs / Math.max(10, periodMs) + phase) % 1 + 1) % 1;

export const previewSceneFxFixtures = (
  fixtures: PatchedFixtureSummary[],
  params: EffectParamsSnapshot,
  elapsedMs: number,
) => {
  if ("Lfo" in params) {
    const request = params.Lfo;
    const targetIds = new Set(request.fixture_ids);
    const progress = cycleProgress(elapsedMs, request.period_ms, request.phase);
    const amount = evaluateLfoShape(request.shape, progress);
    const value = request.low + (request.high - request.low) * amount;
    return fixtures.map((fixture) => targetIds.has(fixture.id)
      ? setFixtureAttributes(fixture, { [request.attribute]: value })
      : fixture);
  }
  if ("Color" in params) {
    const request = params.Color;
    const indexById = new Map(request.fixture_ids.map((fixtureId, index) => [fixtureId, index]));
    return fixtures.map((fixture) => {
      const index = indexById.get(fixture.id);
      if (index === undefined) return fixture;
      const spread = request.fixture_ids.length > 1
        ? (index / (request.fixture_ids.length - 1)) * request.fixture_spread
        : 0;
      const color = sampleColorStops(
        request.stops,
        request.interpolation,
        cycleProgress(elapsedMs, request.period_ms, request.phase + spread),
      );
      if (!color) return fixture;
      const values: Record<string, number> = {};
      for (const control of fixture.controls) {
        const attribute = normalizedAttribute(control.attribute);
        if (attribute.includes("red")) values[control.attribute] = color.red;
        else if (attribute.includes("green")) values[control.attribute] = color.green;
        else if (attribute.includes("blue")) values[control.attribute] = color.blue;
      }
      return setFixtureAttributes(fixture, values);
    });
  }
  if ("Chaser" in params) {
    const request = params.Chaser;
    const stepIndex = Math.floor(elapsedMs / Math.max(10, request.step_duration_ms))
      % Math.max(1, request.steps.length);
    const activeFixtureIds = new Set(request.steps[stepIndex]?.fixture_ids ?? []);
    const targetFixtureIds = new Set(request.steps.flatMap((step) => step.fixture_ids));
    return fixtures.map((fixture) => {
      if (!targetFixtureIds.has(fixture.id)) return fixture;
      const values = Object.fromEntries(request.features.map((feature) => [
        feature.attribute,
        activeFixtureIds.has(fixture.id) ? feature.high : feature.low,
      ]));
      return setFixtureAttributes(fixture, values);
    });
  }
  if ("Move" in params) {
    const request = params.Move;
    const sampled = transformMovePreview(
      sampleMovePath(request.points.map(movePointToPreview), request.interpolation, request.closed),
      request,
    );
    if (sampled.length === 0) return fixtures;
    const indexById = new Map(request.fixture_ids.map((fixtureId, index) => [fixtureId, index]));
    const targetCount = Math.max(1, request.fixture_ids.length);
    const symmetrySplit = Math.ceil(request.fixture_ids.length / 2);
    const anchorX = request.coordinate_mode === "Relative" ? 50 : request.center_x * 100;
    return fixtures.map((fixture) => {
      const index = indexById.get(fixture.id);
      if (index === undefined) return fixture;
      const spread = index / targetCount * request.fixture_spread;
      const baseProgress = cycleProgress(elapsedMs, request.period_ms, request.phase + spread);
      const progress = request.direction === "Reverse"
        ? 1 - baseProgress
        : request.direction === "Bounce"
          ? baseProgress * 2 <= 1 ? baseProgress * 2 : 2 - baseProgress * 2
          : baseProgress;
      const sampledIndex = Math.floor(progress * sampled.length) % sampled.length;
      const sourcePoint = sampled[sampledIndex];
      if (!sourcePoint) return fixture;
      const point = request.symmetry && index >= symmetrySplit
        ? { ...sourcePoint, x: Math.max(0, Math.min(100, anchorX * 2 - sourcePoint.x)) }
        : sourcePoint;
      const values: Record<string, number> = {};
      for (const control of fixture.controls) {
        const attribute = normalizedAttribute(control.attribute);
        if (attribute.includes("tilt")) values[control.attribute] = (1 - point.y / 100) * 65_535;
        else if (attribute.includes("pan")) values[control.attribute] = (point.x / 100) * 65_535;
      }
      return setFixtureAttributes(fixture, values);
    });
  }
  return fixtures;
};
