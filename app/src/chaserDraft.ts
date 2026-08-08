import type { ChaserDirection, ChaserFeature, ChaserStep } from "./types";

export const chaserMinimumSteps = 2;
export const chaserMaximumSteps = 256;
export const chaserDefaultLevel = 65_535;
export const chaserDefaultSeed = 1_337;

const clamp = (value: number, minimum: number, maximum: number) =>
  Math.min(maximum, Math.max(minimum, Number.isFinite(value) ? value : minimum));

export const clampChaserLevel = (value: number) => Math.round(clamp(value, 0, 65_535));
export const clampChaserUnit = (value: number) => clamp(value, 0, 1);
export const clampChaserWings = (value: number) => Math.round(clamp(value, 1, 16));
export const clampChaserSeed = (value: number) => Math.round(clamp(value, 1, 0xffff_ffff));

const uniqueFixtureIds = (fixtureIds: number[]) =>
  [...new Set(fixtureIds.filter((fixtureId) => Number.isInteger(fixtureId) && fixtureId > 0))];

const uniqueGroupIds = (groupIds: string[]) => {
  const seen = new Set<string>();
  return groupIds
    .map((groupId) => groupId.trim())
    .filter((groupId) => groupId.length > 0 && !seen.has(groupId) && Boolean(seen.add(groupId)));
};

export const canonicalChaserAttribute = (attribute: string) =>
  [...attribute]
    .filter((character) => /[a-z0-9]/i.test(character))
    .join("")
    .toLocaleLowerCase();

export const normalizedChaserStep = (step: ChaserStep): ChaserStep => ({
  fixture_ids: uniqueFixtureIds(step.fixture_ids),
  target_group_ids: uniqueGroupIds(step.target_group_ids),
  level: clampChaserLevel(step.level),
});

export const chaserStepsFromTargets = (
  fixtureIds: number[],
  groupIds: string[],
  includeSingleTargetGap = true,
  level = chaserDefaultLevel,
): ChaserStep[] => {
  const steps: ChaserStep[] = [
    ...uniqueFixtureIds(fixtureIds).map((fixtureId) => ({
      fixture_ids: [fixtureId],
      target_group_ids: [],
      level: clampChaserLevel(level),
    })),
    ...uniqueGroupIds(groupIds).map((groupId) => ({
      fixture_ids: [],
      target_group_ids: [groupId],
      level: clampChaserLevel(level),
    })),
  ];
  if (includeSingleTargetGap && steps.length === 1) {
    steps.push({ fixture_ids: [], target_group_ids: [], level: 0 });
  }
  return steps;
};

export interface ChaserDraftValidationInput {
  steps: ChaserStep[];
  features: ChaserFeature[];
  direction: ChaserDirection;
  stepDurationMs: number;
  clockSyncBeats: number | null;
  wings: number;
  activeStepCount: number;
  dutyCycle: number;
  overlap: number;
  phase: number;
  fixtureSpread: number;
  randomSeed: number;
}

export const chaserDraftError = (input: ChaserDraftValidationInput) => {
  const minimumSteps = input.direction === "BuildUpDown" ? 1 : chaserMinimumSteps;
  if (input.steps.length < minimumSteps || input.steps.length > chaserMaximumSteps) {
    return `A ${input.direction === "BuildUpDown" ? "Build / clear Chaser" : "Chaser"} requires ${minimumSteps} to ${chaserMaximumSteps} steps.`;
  }
  if (!input.steps.some((step) => step.fixture_ids.length > 0 || step.target_group_ids.length > 0)) {
    return "At least one Chaser step must target a fixture or group.";
  }
  if (input.steps.some((step) => !Number.isFinite(step.level) || step.level < 0 || step.level > 65_535)) {
    return "Chaser step levels must be within 0 to 65535.";
  }
  if (input.features.length < 1 || input.features.length > 16) {
    return "A Chaser requires 1 to 16 feature ranges.";
  }
  if (input.features.some((feature) => canonicalChaserAttribute(feature.attribute).length === 0)) {
    return "Every Chaser feature requires an attribute containing a letter or number.";
  }
  if (
    input.features.some((feature) =>
      !Number.isFinite(feature.low)
      || !Number.isFinite(feature.high)
      || feature.low < 0
      || feature.low > 65_535
      || feature.high < 0
      || feature.high > 65_535)
  ) {
    return "Chaser feature ranges must be within 0 to 65535.";
  }
  const featureKeys = input.features.map((feature) => canonicalChaserAttribute(feature.attribute));
  if (new Set(featureKeys).size !== featureKeys.length) {
    return "Each Chaser feature attribute can only be added once.";
  }
  if (!Number.isFinite(input.stepDurationMs) || input.stepDurationMs < 10) {
    return "Chaser step duration must be at least 10 ms.";
  }
  if (input.clockSyncBeats !== null && (!Number.isFinite(input.clockSyncBeats) || input.clockSyncBeats <= 0)) {
    return "Chaser clock sync must be greater than zero beats per step.";
  }
  if (!Number.isInteger(input.wings) || input.wings < 1 || input.wings > Math.min(input.steps.length, 16)) {
    return "Chaser wings must be an integer from 1 to the step count, up to 16.";
  }
  if (
    !Number.isInteger(input.activeStepCount)
    || input.activeStepCount < 1
    || input.activeStepCount > Math.min(input.steps.length, 64)
  ) {
    return "Chaser Pixels on must be an integer from 1 to the step count, up to 64.";
  }
  if (input.direction === "BuildUpDown" && input.activeStepCount !== 1) {
    return "Build / clear uses exactly one transition frontier; Pixels on must be 1.";
  }
  if (!Number.isFinite(input.dutyCycle) || input.dutyCycle <= 0 || input.dutyCycle > 1) {
    return "Chaser size must be greater than 0% and at most 100%.";
  }
  if (!Number.isFinite(input.overlap) || input.overlap < 0 || input.overlap > 1) {
    return "Chaser overlap must be within 0% to 100%.";
  }
  if (!Number.isFinite(input.phase) || input.phase < 0 || input.phase > 1) {
    return "Chaser phase must be within 0% to 100%.";
  }
  if (!Number.isFinite(input.fixtureSpread) || input.fixtureSpread < 0 || input.fixtureSpread > 1) {
    return "Chaser fixture spread must be within 0% to 100%.";
  }
  if (!Number.isSafeInteger(input.randomSeed) || input.randomSeed < 1 || input.randomSeed > 0xffff_ffff) {
    return "Chaser random seed must be an integer from 1 to 4294967295.";
  }
  return "";
};

const seededStepOrder = (stepCount: number, seed: number) => {
  const order = Array.from({ length: stepCount }, (_, index) => index);
  let state = clampChaserSeed(seed) >>> 0;
  for (let index = order.length - 1; index > 0; index -= 1) {
    state = (Math.imul(state, 1_664_525) + 1_013_904_223) >>> 0;
    const target = state % (index + 1);
    [order[index], order[target]] = [order[target], order[index]];
  }
  return order;
};

export const chaserPreviewOrder = (stepCount: number, direction: ChaserDirection, randomSeed: number) => {
  const forward = Array.from({ length: Math.max(0, stepCount) }, (_, index) => index);
  if (direction === "Reverse") return forward.reverse();
  if (direction === "Bounce" && forward.length > 1) {
    return [...forward, ...forward.slice(1, -1).reverse()];
  }
  if (direction === "Random") return seededStepOrder(forward.length, randomSeed);
  if (direction === "BuildUpDown") return [...forward, ...forward];
  return forward;
};

export const chaserPreviewIndex = (
  stepCount: number,
  direction: ChaserDirection,
  randomSeed: number,
  phase: number,
) => {
  const order = chaserPreviewOrder(stepCount, direction, randomSeed);
  if (order.length === 0) return null;
  const offset = Math.floor(clampChaserUnit(phase) * order.length) % order.length;
  return order[offset];
};

export const chaserActivePreviewIndices = (
  order: number[],
  activeOrderPosition: number,
  activeStepCount: number,
  direction: ChaserDirection = "Forward",
) => {
  if (direction === "BuildUpDown") {
    const stepCount = order.length / 2;
    if (!Number.isInteger(stepCount) || stepCount <= 0) return [];
    const position = ((activeOrderPosition % order.length) + order.length) % order.length;
    if (position < stepCount) {
      return order.slice(0, position + 1);
    }
    return order.slice(position - stepCount + 1, stepCount);
  }
  const active = new Set<number>();
  const targetCount = Math.min(Math.max(0, Math.round(activeStepCount)), new Set(order).size);
  if (order.length === 0 || targetCount === 0) return [];
  for (let offset = 0; offset < order.length && active.size < targetCount; offset += 1) {
    const position = (activeOrderPosition - offset + order.length) % order.length;
    active.add(order[position]);
  }
  return [...active];
};
