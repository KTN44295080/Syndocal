const clamp = (value: number, minimum: number, maximum: number) =>
  Math.min(maximum, Math.max(minimum, value));

/** Missing legacy recipes deserialize to the runtime seed default: zero. */
export const materializedRandomEffectSeed = (value: unknown): number =>
  typeof value === "number" && Number.isFinite(value)
    ? clamp(Math.round(value), 0, 0xffff_ffff)
    : 0;

/**
 * Legacy native Sparkle stored lifespan as a percentage of the authored effect
 * period. Corrected Sparkle stores an absolute millisecond lifetime, so the
 * visual-preserving conversion resolves that percentage against the same
 * authored period and clamps it to the corrected 100..1000 ms domain. Missing
 * period or lifespan state means the 100 ms clamp floor.
 */
export const correctedSparkleLifetimeMsFromLegacyPercent = (
  periodMs: unknown,
  lifespanPercent: unknown,
): number => {
  if (typeof periodMs !== "number" || !Number.isFinite(periodMs)) return 100;
  if (typeof lifespanPercent !== "number" || !Number.isFinite(lifespanPercent)) return 100;
  return clamp(Math.round((periodMs * lifespanPercent) / 100), 100, 1_000);
};
