import type {
  CueEffectTarget,
  EffectBlendMode,
  EffectKind,
  EffectSummary,
  PatchedFixtureSummary,
} from "./types";

export type CueEffectCaptureScope = "all" | "lighting" | "effects" | "selectedFixture" | "selectedGroup" | "video";

export type CueEffectRecallChange =
  | { kind: "captureCurrent" }
  | { kind: "refreshStates" }
  | { kind: "selectAll" }
  | { kind: "clear" }
  | { kind: "include"; effectId: number; included: boolean }
  | { kind: "state"; effectId: number }
  | { kind: "transition"; effectId: number };

export interface CueEffectScopeContext {
  fixtures: Pick<PatchedFixtureSummary, "id" | "group_ids">[];
  selectedFixtureId: number | null;
  selectedGroupId: string | null;
}

const normalizedGroupId = (value: string) => {
  const segments = value.trim().split("/").map((segment) => segment.trim());
  return segments.some((segment) => segment.length === 0) ? null : segments.join("/");
};

export const groupMatches = (candidate: string, requested: string) => {
  const candidateId = normalizedGroupId(candidate);
  const requestedId = normalizedGroupId(requested);
  if (candidateId === null || requestedId === null) return false;
  return candidateId === requestedId || candidateId.startsWith(`${requestedId}/`);
};

const effectLightingTargets = (effect: EffectSummary) =>
  effect.effect_type === "Color" && effect.color
    ? effect.color
    : effect;

const effectTargetsFixture = (
  effect: EffectSummary,
  fixture: Pick<PatchedFixtureSummary, "id" | "group_ids">,
) => {
  const targets = effectLightingTargets(effect);
  return targets.fixture_ids.includes(fixture.id)
    || targets.target_group_ids.some((targetGroup) => fixture.group_ids.some((group) => groupMatches(group, targetGroup)));
};

export const eligibleCueEffects = (
  effects: EffectSummary[],
  scope: CueEffectCaptureScope,
  context: CueEffectScopeContext,
) => {
  if (scope === "all") return effects;
  if (scope === "effects") return effects;
  if (scope === "lighting") {
    return effects.filter((effect) => {
      const targets = effectLightingTargets(effect);
      return targets.fixture_ids.length > 0 || targets.target_group_ids.length > 0;
    });
  }
  if (scope === "video") {
    return effects.filter((effect) => effect.video_targets.length > 0);
  }
  if (scope === "selectedFixture") {
    const fixture = context.fixtures.find((candidate) => candidate.id === context.selectedFixtureId);
    return fixture ? effects.filter((effect) => effectTargetsFixture(effect, fixture)) : [];
  }
  if (!context.selectedGroupId) return [];
  const fixtures = context.fixtures.filter((fixture) =>
    fixture.group_ids.some((group) => groupMatches(group, context.selectedGroupId!))
  );
  return effects.filter((effect) => fixtures.some((fixture) => effectTargetsFixture(effect, fixture)));
};

export const currentCueEffectTargets = (effects: EffectSummary[]): CueEffectTarget[] =>
  effects.map((effect) => ({ effect_id: effect.id, enabled: effect.enabled }));

export const normalizedCueEffectTargets = (
  effects: EffectSummary[],
  targets: CueEffectTarget[],
): CueEffectTarget[] => {
  const targetById = new Map<number, CueEffectTarget>();
  const effectIds = new Set(effects.map((effect) => effect.id));
  for (const target of targets) {
    if (!targetById.has(target.effect_id)) targetById.set(target.effect_id, target);
  }
  return effects.flatMap((effect) => {
    const target = targetById.get(effect.id);
    return target ? [{ ...target, effect_id: effect.id, enabled: target.enabled }] : [];
  }).concat(
    [...targetById.values()]
      .filter((target) => target.params != null && !effectIds.has(target.effect_id))
      .map((target) => ({ ...target })),
  );
};

export const selectAllCueEffects = (
  effects: EffectSummary[],
  targets: CueEffectTarget[],
): CueEffectTarget[] => {
  const existingIds = new Set(targets.map((target) => target.effect_id));
  return normalizedCueEffectTargets(effects, [
    ...targets,
    ...effects
      .filter((effect) => !existingIds.has(effect.id))
      .map((effect) => ({ effect_id: effect.id, enabled: effect.enabled })),
  ]);
};

export const syncCueEffectCaptureTargets = (
  effects: EffectSummary[],
  targets: CueEffectTarget[],
  stateOverrideIds: Iterable<number>,
  includeMissing: boolean,
): CueEffectTarget[] => {
  const currentById = new Map(targets.map((target) => [target.effect_id, target]));
  const overrides = new Set(stateOverrideIds);
  return effects.flatMap((effect) => {
    const current = currentById.get(effect.id);
    if (!current && !includeMissing) return [];
    return [{
      ...current,
      effect_id: effect.id,
      enabled: current && overrides.has(effect.id) ? current.enabled : effect.enabled,
    }];
  });
};

export const refreshListedCueEffectStates = (
  effects: EffectSummary[],
  targets: CueEffectTarget[],
): CueEffectTarget[] => {
  const current = new Map(effects.map((effect) => [effect.id, effect.enabled]));
  return normalizedCueEffectTargets(
    effects,
    targets.map((target) => ({
      ...target,
      effect_id: target.effect_id,
      enabled: current.get(target.effect_id) ?? target.enabled,
    })),
  );
};

export const setCueEffectIncluded = (
  effects: EffectSummary[],
  targets: CueEffectTarget[],
  effectId: number,
  included: boolean,
): CueEffectTarget[] => {
  const withoutTarget = targets.filter((target) => target.effect_id !== effectId);
  if (!included) return normalizedCueEffectTargets(effects, withoutTarget);
  const existing = targets.find((target) => target.effect_id === effectId);
  if (existing) return normalizedCueEffectTargets(effects, [...withoutTarget, existing]);
  const effect = effects.find((candidate) => candidate.id === effectId);
  return effect
    ? normalizedCueEffectTargets(effects, [...withoutTarget, { effect_id: effectId, enabled: effect.enabled }])
    : normalizedCueEffectTargets(effects, withoutTarget);
};

export const setCueEffectTargetEnabled = (
  effects: EffectSummary[],
  targets: CueEffectTarget[],
  effectId: number,
  enabled: boolean,
): CueEffectTarget[] => normalizedCueEffectTargets(
  effects,
  targets.map((target) => target.effect_id === effectId ? { ...target, enabled } : target),
);

export const setCueEffectTargetTransition = (
  effects: EffectSummary[],
  targets: CueEffectTarget[],
  effectId: number,
  transitionMs: number | null,
): CueEffectTarget[] => normalizedCueEffectTargets(
  effects,
  targets.map((target) => {
    if (target.effect_id !== effectId) return target;
    if (transitionMs === null || !Number.isFinite(transitionMs)) {
      const { transition_ms: _transitionMs, ...legacyShape } = target;
      return legacyShape;
    }
    return {
      ...target,
      transition_ms: Math.max(0, Math.min(600_000, Math.round(transitionMs))),
    };
  }),
);

export const chaserTraversalStepCount = (effect: EffectSummary) => {
  const chaser = effect.chaser;
  if (!chaser) return 0;
  const stepCount = chaser.steps.length;
  if (stepCount <= 1) return Math.max(1, stepCount);
  if (chaser.direction === "Bounce") return stepCount * 2 - 2;
  if (chaser.direction === "Random") return stepCount * (chaser.random_cycle_count ?? 1);
  return stepCount;
};

export const authoredBeatsForEffectClock = (effect: EffectSummary) => {
  switch (effect.effect_type) {
    case "Color":
      return effect.color?.clock_sync?.beats ?? effect.clock_sync?.beats ?? null;
    case "Chaser":
      return effect.chaser?.clock_sync?.beats ?? effect.clock_sync?.beats ?? null;
    case "Move":
      return effect.move_effect?.clock_sync?.beats ?? effect.clock_sync?.beats ?? null;
    case "Value":
      return effect.value?.clock_sync?.beats ?? effect.clock_sync?.beats ?? null;
    case "Curve":
      return effect.curve?.clock_sync?.beats ?? effect.clock_sync?.beats ?? null;
    case "Mapping":
      return effect.mapping?.clock_sync?.beats ?? effect.clock_sync?.beats ?? null;
    case "ColorMapping":
      return effect.color_mapping?.clock_sync?.beats ?? effect.clock_sync?.beats ?? null;
    default:
      return effect.clock_sync?.beats ?? null;
  }
};

export const inferredCueAuthoredBeats = (effects: EffectSummary[], targets: CueEffectTarget[]) => {
  const effectsById = new Map(effects.map((effect) => [effect.id, effect]));
  const candidates = targets.filter((target) => target.enabled).flatMap((target) => {
    const effect = effectsById.get(target.effect_id);
    if (!effect) return [];
    if (effect.effect_type === "Chaser") {
      const traversalSteps = chaserTraversalStepCount(effect);
      if (traversalSteps <= 0) return [];
      const syncBeats = authoredBeatsForEffectClock(effect);
      return [(syncBeats ?? 1) * traversalSteps];
    }
    const syncBeats = authoredBeatsForEffectClock(effect);
    return syncBeats === null ? [] : [syncBeats];
  });
  if (candidates.length === 0) return null;
  if (candidates.some((candidate) => !Number.isFinite(candidate) || candidate < 0.25 || candidate > 1024)) {
    return null;
  }
  const first = candidates[0];
  return candidates.every((candidate) => Math.abs(candidate - first) <= 1e-6) ? first : null;
};

export const cueOwnedEffectSummary = (
  target: CueEffectTarget,
  source: EffectSummary | null,
): EffectSummary | null => {
  const params = target.params;
  if (!params) return source;
  const shell = (
    effectType: EffectKind,
    label: string,
    fixtureIds: number[],
    targetGroupIds: string[],
    attribute: string,
    periodMs: number | null,
    clockSync: EffectSummary["clock_sync"],
    low: number,
    high: number,
    phase: number,
    blendMode: EffectBlendMode,
  ): EffectSummary => ({
    id: target.effect_id,
    label,
    effect_type: effectType,
    fixture_ids: fixtureIds,
    target_group_ids: targetGroupIds,
    attribute,
    video_targets: [],
    shape: source?.shape ?? "Sine",
    period_ms: periodMs,
    clock_sync: clockSync,
    low,
    high,
    phase,
    fixture_spread: 0,
    blend_mode: blendMode,
    enabled: target.enabled,
    params,
    color: null,
    chaser: null,
    move_effect: null,
    value: null,
    curve: null,
    mapping: null,
    color_mapping: null,
  });
  if ("Lfo" in params) {
    const request = params.Lfo;
    return {
      ...shell(
        "Lfo",
        request.label,
        request.fixture_ids,
        request.target_group_ids,
        request.attribute,
        request.period_ms,
        request.clock_sync,
        request.low,
        request.high,
        request.phase,
        request.blend_mode,
      ),
      video_targets: request.video_targets,
      shape: request.shape,
      fixture_spread: request.fixture_spread ?? 0,
      lfo: request,
    };
  }
  if ("PositionWave" in params) {
    const request = params.PositionWave;
    return {
      ...shell(
        "PositionWave",
        request.label,
        request.fixture_ids,
        request.target_group_ids,
        request.attribute,
        source?.period_ms ?? null,
        request.clock_sync,
        request.low,
        request.high,
        request.phase,
        request.blend_mode,
      ),
      video_targets: request.video_targets,
      shape: request.shape,
      origin: request.origin,
      direction: request.direction,
      speed: request.speed,
      wavelength: request.wavelength,
    };
  }
  if ("Color" in params) {
    const request = params.Color;
    return {
      ...shell(
        "Color",
        request.label,
        request.fixture_ids,
        request.target_group_ids,
        "",
        request.period_ms,
        request.clock_sync,
        0,
        65_535,
        request.phase,
        request.blend_mode,
      ),
      fixture_spread: request.fixture_spread,
      color: request,
    };
  }
  if ("Chaser" in params) {
    const request = params.Chaser;
    const fixtureIds = [...new Set(request.steps.flatMap((step) => step.fixture_ids))];
    const groupIds = [...new Set(request.steps.flatMap((step) => step.target_group_ids))];
    const feature = request.features[0];
    return {
      ...shell(
        "Chaser",
        request.label,
        fixtureIds,
        groupIds,
        feature?.attribute ?? "",
        request.step_duration_ms,
        request.clock_sync,
        feature?.low ?? 0,
        feature?.high ?? 65_535,
        request.phase,
        request.blend_mode,
      ),
      fixture_spread: request.fixture_spread,
      chaser: request,
    };
  }
  if ("Move" in params) {
    const request = params.Move;
    return {
      ...shell(
        "Move",
        request.label,
        request.fixture_ids,
        request.target_group_ids,
        "Pan/Tilt",
        request.period_ms,
        request.clock_sync,
        0,
        65_535,
        request.phase,
        request.blend_mode,
      ),
      fixture_spread: request.fixture_spread,
      move_effect: request,
    };
  }
  if ("Value" in params) {
    const request = params.Value;
    return {
      ...shell(
        "Value",
        request.label,
        request.fixture_ids,
        request.target_group_ids,
        request.attribute,
        request.period_ms,
        request.clock_sync,
        request.low,
        request.high,
        request.phase,
        request.blend_mode,
      ),
      fixture_spread: request.fixture_spread,
      value: request,
    };
  }
  if ("Curve" in params) {
    const request = params.Curve;
    return {
      ...shell(
        "Curve",
        request.label,
        request.fixture_ids,
        request.target_group_ids,
        request.attribute,
        request.period_ms,
        request.clock_sync,
        request.low,
        request.high,
        request.phase,
        request.blend_mode,
      ),
      fixture_spread: request.fixture_spread,
      curve: request,
    };
  }
  if ("Mapping" in params) {
    const request = params.Mapping;
    return {
      ...shell(
        "Mapping",
        request.label,
        request.fixture_ids,
        request.target_group_ids,
        request.attribute,
        request.period_ms,
        request.clock_sync,
        request.low,
        request.high,
        request.phase,
        request.blend_mode,
      ),
      shape: request.shape,
      fixture_spread: request.fixture_spread,
      mapping: request,
    };
  }
  const request = params.ColorMapping;
  return {
    ...shell(
      "ColorMapping",
      request.label,
      request.fixture_ids,
      request.target_group_ids,
      "Colour Mapping",
      request.period_ms,
      request.clock_sync,
      0,
      65_535,
      request.phase,
      request.blend_mode,
    ),
    color_mapping: request,
  };
};
