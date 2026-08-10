import type { ColorEffectSpatialRecipe } from "./types";

/**
 * Shared beam-space recipe compatibility layer for the COLOR and VALUE FX
 * editors. Both panels author the same `ColorEffectSpatialRecipe` bodies and
 * both have to convert between Syndocal's Enhanced parameter domains and the
 * corrected DVC domains, so the authored defaults and the Enhanced<->DVC
 * conversions live here instead of drifting apart in two component copies.
 */

/** Every recipe kind that has an authored default in the FX editors. */
export type SpatialRecipeKind =
  | "KnightRider"
  | "Burst"
  | "Sweep"
  | "RandomFill"
  | "Sparkle"
  | "Plasma"
  | "ColorRainbow"
  | "Rainbow"
  | "Perlin";

/** Partial recipe body written back through the panel's patch helper. */
export type SpatialRecipePatch = Record<string, number | boolean>;

/** Reader over the currently authored recipe body. */
export interface SpatialRecipeReadout {
  /** Finite number for `key`, or `fallback` when the field is absent. */
  number: (key: string, fallback?: number) => number;
  /** True only when `key` is literally `true` in the current body. */
  boolean: (key: string) => boolean;
}

const clamp = (value: number, minimum: number, maximum: number) =>
  Math.min(maximum, Math.max(minimum, Number.isFinite(value) ? value : minimum));

/** Sparkle and Random fill are the seeded generators with a corrected evaluator. */
export const matchesRandomSpatialKind = (kind: string) =>
  kind === "Sparkle" || kind === "RandomFill";

/** Authored starting body for a beam-space recipe kind. */
export const defaultSpatialRecipe = (kind: SpatialRecipeKind): ColorEffectSpatialRecipe => {
  switch (kind) {
    case "KnightRider":
      return { KnightRider: { grayscale: false, vertical_symmetry: false, size: 8, one_way: false, fading: true, go_outside: false, gradient: 50 } };
    case "Burst":
      return { Burst: { color_width: 50, gradient: 100 } };
    case "Sweep":
      return { Sweep: { daslight_exact: false, grayscale: false, vertical_symmetry: false, direction_change: false } };
    case "RandomFill":
      return { RandomFill: { syndocal_corrected: true, grayscale: false, vertical_symmetry: false, rng_seed: 1, point_width: 1, source_point_height: null } };
    case "Sparkle":
      return { Sparkle: { syndocal_corrected: true, grayscale: false, vertical_symmetry: false, rng_seed: 1, number: 5, lifespan: 0, lifetime_ms: 250, source_lifespan: null, width: 1 } };
    case "Plasma":
      return { Plasma: { grayscale: false, vertical_symmetry: false, size_x: 1, param_x: 2, size_y: 1, param_y: 2, speed_x: -1, param_sx: 2, speed_y: 1, param_sy: -1 } };
    case "ColorRainbow":
      return { ColorRainbow: { grayscale: false, vertical_symmetry: false, color_width: 0, angle_degrees: 0, gradient: 100 } };
    case "Rainbow":
      return { Rainbow: { grayscale: false, vertical_symmetry: false, horizontal_symmetry: false, rotation_degrees: 0, color_width: 0, angle_degrees: 0, gradient: 100 } };
    case "Perlin":
      return { Perlin: { daslight_exact: false, grayscale: false, vertical_symmetry: false, horizontal_symmetry: false, rotation_degrees: 0, octaves: 5, zoom: 20, direction_degrees: 0, speed: 1, amplitude: 100 } };
  }
};

/**
 * Burst switches between the Enhanced percent domain (`color_width` 0..100,
 * `gradient` 0..100) and the corrected DVC pixel domain (`color_width` 10..900,
 * `gradient` 0..1). The corrected route deliberately preserves the operator's
 * qGray/Transform state; only the Enhanced route clears those DVC-only fields.
 */
export const burstEvaluatorPatch = (
  daslightExact: boolean,
  values: SpatialRecipeReadout,
): SpatialRecipePatch => {
  const currentWidth = values.number("color_width", 50);
  const currentGradient = values.number("gradient", values.boolean("daslight_exact") ? 1 : 100);
  return daslightExact
    ? {
        daslight_exact: true,
        color_width: clamp(Math.round(currentWidth), 10, 900),
        gradient: clamp(currentGradient > 1 ? currentGradient / 100 : currentGradient, 0, 1),
      }
    : {
        daslight_exact: false,
        grayscale: false,
        vertical_symmetry: false,
        color_width: clamp(currentWidth, 0, 100),
        gradient: clamp(currentGradient <= 1 ? currentGradient * 100 : currentGradient, 0, 100),
      };
};

/** Sweep shares one parameter domain, so only the evaluator route changes. */
export const sweepEvaluatorPatch = (daslightExact: boolean): SpatialRecipePatch => ({
  daslight_exact: daslightExact,
});

/**
 * Perlin switches between the Enhanced open domains and the corrected DVC
 * domains (`octaves` 2..10, `zoom` 1..100, `direction_degrees` 1..100,
 * `speed` 1..10, `amplitude` 5..100). Horizontal symmetry and rotation only
 * exist for placed 2D mapping patterns, so they are preserved when the pattern
 * carries a placement and reset to the neutral value when it does not.
 */
export const perlinEvaluatorPatch = (
  daslightExact: boolean,
  values: SpatialRecipeReadout,
  hasMappingPlacement: boolean,
): SpatialRecipePatch => {
  if (daslightExact) {
    return {
      daslight_exact: true,
      grayscale: values.boolean("grayscale"),
      vertical_symmetry: values.boolean("vertical_symmetry"),
      horizontal_symmetry: hasMappingPlacement && values.boolean("horizontal_symmetry"),
      rotation_degrees: hasMappingPlacement
        ? clamp(Math.round(values.number("rotation_degrees")), 0, 360)
        : 0,
      octaves: clamp(Math.round(values.number("octaves", 5)), 2, 10),
      zoom: clamp(Math.round(values.number("zoom", 20)), 1, 100),
      direction_degrees: clamp(Math.round(values.number("direction_degrees", 1)), 1, 100),
      speed: clamp(Math.round(values.number("speed", 1)), 1, 10),
      amplitude: clamp(Math.round(values.number("amplitude", 100)), 5, 100),
    };
  }
  return {
    daslight_exact: false,
    grayscale: false,
    vertical_symmetry: false,
    horizontal_symmetry: false,
    rotation_degrees: 0,
    octaves: clamp(Math.round(values.number("octaves", 5)), 1, 16),
    zoom: Math.max(0.01, values.number("zoom", 20)),
    amplitude: clamp(values.number("amplitude", 100), 0, 100),
  };
};
