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

// ---- T7: persistent identity colors -------------------------------------
//
// A cue or group may carry a user-chosen #rrggbb color that wins over the
// hash-derived hue. The user's hue and saturation are honoured, but per-role
// lightness (and a saturation ceiling) stay clamped so the same readability
// contract as the hash palette holds on graphite: near-black or neon picks
// can never produce unreadable pads, bands, or text tints.

/** Parse #rgb/#rrggbb into HSL. Returns null for anything else. */
export const parseHexColorHsl = (
  value: string,
): { h: number; s: number; l: number } | null => {
  const raw = value.trim();
  const match = /^#(?:[0-9a-fA-F]{3}|[0-9a-fA-F]{6})$/.exec(raw);
  if (!match) return null;
  const hex = raw.slice(1);
  const full = hex.length === 3 ? hex.split("").map((c) => c + c).join("") : hex;
  const r = parseInt(full.slice(0, 2), 16) / 255;
  const g = parseInt(full.slice(2, 4), 16) / 255;
  const b = parseInt(full.slice(4, 6), 16) / 255;
  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  const delta = max - min;
  const l = (max + min) / 2;
  if (delta === 0) return { h: 0, s: 0, l };
  const s = delta / (1 - Math.abs(2 * l - 1));
  let h: number;
  if (max === r) h = ((g - b) / delta) % 6;
  else if (max === g) h = (b - r) / delta + 2;
  else h = (r - g) / delta + 4;
  h = ((h * 60) % 360 + 360) % 360;
  return { h, s, l };
};

const clamp = (value: number, low: number, high: number): number =>
  Math.min(high, Math.max(low, value));

/** Role-clamped css color for a persisted identity hex. */
export const identityCssFromPersistent = (
  hex: string,
  role: IdentityColorRole,
): string | null => {
  const hsl = parseHexColorHsl(hex);
  if (!hsl) return null;
  const hue = Math.round(hsl.h);
  const saturation = Math.round(clamp(hsl.s * 100, 20, 85));
  if (role === "band") return `hsl(${hue}, ${saturation}%, ${Math.round(clamp(hsl.l * 100, 24, 36))}%)`;
  if (role === "text") return `hsl(${hue}, ${Math.max(saturation, 55)}%, ${Math.round(clamp(hsl.l * 100, 68, 80))}%)`;
  return `hsl(${hue}, ${saturation}%, ${Math.round(clamp(hsl.l * 100, 40, 65))}%)`;
};

/** Cue identity css: persisted color wins, hash hue is the fallback. */
export const cueIdentityCss = (
  cueId: number,
  color: string | null | undefined,
  role: IdentityColorRole,
): string =>
  (color ? identityCssFromPersistent(color, role) : null) ??
  identityCssColor(cueIdentityHue(cueId), role);

/** Group identity css: persisted map entry wins, hash hue is the fallback. */
export const groupIdentityCss = (
  groupId: string,
  groupColors: Readonly<Record<string, string>> | null | undefined,
  role: IdentityColorRole,
): string => {
  const persisted = groupColors?.[groupId];
  return (
    (persisted ? identityCssFromPersistent(persisted, role) : null) ??
    identityCssColor(groupIdentityHue(groupId), role)
  );
};
