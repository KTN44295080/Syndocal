import type { CueFixtureTarget, CueStepSummary } from "./types";

const cloneTargets = (targets: CueFixtureTarget[]): CueFixtureTarget[] => targets.map((target) => ({
  fixture_id: target.fixture_id,
  values: target.values.map((value) => ({ ...value })),
}));

export const cloneCueSteps = (steps: CueStepSummary[] | null | undefined): CueStepSummary[] =>
  (steps ?? []).map((step) => ({
    values: cloneTargets(step.values),
    fade_ms: step.fade_ms,
    hold_ms: step.hold_ms,
  }));

export const cueStepsTotalDurationMs = (steps: CueStepSummary[] | null | undefined) =>
  (steps ?? []).reduce(
    (total, step) => total + Math.max(0, step.fade_ms) + Math.max(0, step.hold_ms),
    0,
  );

export const cueStepsAuthoredBeats = (
  steps: CueStepSummary[] | null | undefined,
  bpm: number,
) => {
  const totalMs = cueStepsTotalDurationMs(steps);
  if (totalMs <= 0 || !Number.isFinite(bpm) || bpm <= 0) return null;
  const beats = totalMs * bpm / 60_000;
  if (!Number.isFinite(beats) || beats < 0.25 || beats > 1024) return null;
  return Math.round(beats * 1_000) / 1_000;
};

export const appendCueStep = (
  steps: CueStepSummary[],
  cueTargets: CueFixtureTarget[],
  cueFadeMs: number,
) => {
  const sourceValues = steps.at(-1)?.values ?? cueTargets;
  return [
    ...cloneCueSteps(steps),
    {
      values: cloneTargets(sourceValues),
      fade_ms: Math.max(0, Math.round(cueFadeMs)),
      hold_ms: 0,
    },
  ];
};

export const duplicateCueStep = (steps: CueStepSummary[], index: number) => {
  const source = steps[index];
  if (!source) return cloneCueSteps(steps);
  const next = cloneCueSteps(steps);
  next.splice(index + 1, 0, cloneCueSteps([source])[0]);
  return next;
};

export const moveCueStep = (steps: CueStepSummary[], index: number, delta: -1 | 1) => {
  const target = index + delta;
  const next = cloneCueSteps(steps);
  if (!next[index] || !next[target]) return next;
  [next[index], next[target]] = [next[target], next[index]];
  return next;
};

export const sanitizeCueStepDuration = (value: number) =>
  Math.max(0, Math.round(Number.isFinite(value) ? value : 0));
