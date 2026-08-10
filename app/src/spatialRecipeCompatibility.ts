import type { ColorEffectSpatialRecipe } from "./types";

/** Shared authored defaults for the COLOR and VALUE beam-space editors. */

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

/** Authored starting body for a beam-space recipe kind. */
export const defaultSpatialRecipe = (kind: SpatialRecipeKind): ColorEffectSpatialRecipe => {
  switch (kind) {
    case "KnightRider":
      return { KnightRider: { grayscale: false, vertical_symmetry: false, size: 8, one_way: false, fading: true, go_outside: false, gradient: 50 } };
    case "Burst":
      return { Burst: { color_width: 50, gradient: 100 } };
    case "Sweep":
      return { Sweep: { grayscale: false, vertical_symmetry: false, direction_change: false } };
    case "RandomFill":
      return { RandomFill: { grayscale: false, vertical_symmetry: false, rng_seed: 1, point_width: 10, source_point_height: null } };
    case "Sparkle":
      return { Sparkle: { grayscale: false, vertical_symmetry: false, rng_seed: 1, number: 5, lifetime_ms: 250, source_lifespan: null, width: 10 } };
    case "Plasma":
      return { Plasma: { grayscale: false, vertical_symmetry: false, size_x: 1, param_x: 2, size_y: 1, param_y: 2, speed_x: -1, param_sx: 2, speed_y: 1, param_sy: -1 } };
    case "ColorRainbow":
      return { ColorRainbow: { grayscale: false, vertical_symmetry: false, color_width: 0, angle_degrees: 0, gradient: 100 } };
    case "Rainbow":
      return { Rainbow: { grayscale: false, vertical_symmetry: false, horizontal_symmetry: false, rotation_degrees: 0, color_width: 0, angle_degrees: 0, gradient: 100 } };
    case "Perlin":
      return { Perlin: { grayscale: false, vertical_symmetry: false, horizontal_symmetry: false, rotation_degrees: 0, octaves: 5, zoom: 0.5, direction_degrees: 0, speed: 1, amplitude: 100 } };
  }
};
