// Deterministic show-identity hue for cross-surface readability.
// One stable hue per cue id and per group path, repeated across cue pads,
// timeline scene blocks, cue rows, and group chips so the same show entity
// reads as the same colour everywhere (the Daslight-style identity device).
//
// Pure module: no SolidJS or component-state dependencies. Keep side-effect
// free, mirroring the style of the other pure helpers (see numericHelpers.ts).

// FNV-1a 32-bit string hash. Deterministic across runs and platforms.
const fnv1a = (input: string): number => {
  let hash = 0x811c9dc5;
  for (let index = 0; index < input.length; index += 1) {
    hash ^= input.charCodeAt(index);
    // Multiply by the FNV prime (16777619) with 32-bit overflow via Math.imul.
    hash = Math.imul(hash, 0x01000193);
  }
  return hash >>> 0;
};

// Accent orange used by the GO button (--ray-accent: #ed8435 ≈ 25.8°). Identity
// hues must never sit near it, so identity can never be confused with the GO
// accent. Derived once here; the palette guard excludes the ±band around it.
const ACCENT_HUE = 26;
const ACCENT_GUARD_DEG = 20;

// Quantize the wheel into 15° slots and drop every slot within ±20° of the
// accent. This leaves 21 guard-safe hue slots.
const HUE_STEP = 15;

const hueDistance = (left: number, right: number): number => {
  const delta = Math.abs(((left - right) % 360 + 360) % 360);
  return Math.min(delta, 360 - delta);
};

const IDENTITY_HUES: readonly number[] = Array.from(
  { length: 360 / HUE_STEP },
  (_, index) => index * HUE_STEP,
).filter((hue) => hueDistance(hue, ACCENT_HUE) >= ACCENT_GUARD_DEG);

// Golden-ratio-ish stride over the 21-slot palette. Coprime with the palette
// length, so cue ids that increment by one land far apart on the wheel
// (>=120° in practice) instead of drifting through neighbouring hues.
const CUE_HUE_STRIDE = 13;

const paletteHue = (slot: number): number => {
  const length = IDENTITY_HUES.length;
  const index = ((slot % length) + length) % length;
  return IDENTITY_HUES[index];
};

/** Stable identity hue (degrees) for a cue id. Adjacent ids stay far apart. */
export const cueIdentityHue = (cueId: number): number => {
  const id = Number.isFinite(cueId) ? Math.trunc(Math.abs(cueId)) : 0;
  return paletteHue(id * CUE_HUE_STRIDE);
};

/** Stable identity hue (degrees) for a group path string. */
export const groupIdentityHue = (groupId: string): number =>
  paletteHue(fnv1a(`group:${groupId}`));

export type IdentityColorRole = "fill" | "band" | "text";

/**
 * hsl() string for an identity hue. `fill` is the mid-saturation base block
 * colour, `band` is a darker variant for lower bands / bars, and `text` is a
 * high-lightness on-hue tint that stays readable on graphite (#181c1f).
 */
export const identityCssColor = (hue: number, role: IdentityColorRole): string => {
  const normalized = Math.round(((hue % 360) + 360) % 360);
  if (role === "band") return `hsl(${normalized}, 52%, 30%)`;
  if (role === "text") return `hsl(${normalized}, 70%, 74%)`;
  return `hsl(${normalized}, 58%, 55%)`;
};
