import type { VideoGenerator } from "./types";

export const defaultVideoGenerators = {
  Solid: (): VideoGenerator => ({ Solid: { color: [255, 255, 255, 255] } }),
  LinearGradient: (): VideoGenerator => ({ LinearGradient: { color_a: [0, 0, 0, 255], color_b: [255, 255, 255, 255], angle_degrees: 0 } }),
  Noise: (): VideoGenerator => ({ Noise: { seed: 1, scale: 32, speed: 100, color_a: [0, 0, 0, 255], color_b: [255, 255, 255, 255] } }),
};
