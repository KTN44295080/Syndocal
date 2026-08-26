import type { CueEffectTarget, EffectSummary, PatchedFixtureSummary } from "./types";

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
