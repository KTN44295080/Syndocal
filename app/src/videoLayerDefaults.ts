import type { Transform2D, VideoColorAdjust, VideoFxAdjust } from "./types";

export const defaultTransform: Transform2D = {
  x: 0,
  y: 0,
  scale_x: 1,
  scale_y: 1,
  rotation_deg: 0,
  crop_left: 0,
  crop_top: 0,
  crop_right: 0,
  crop_bottom: 0,
};

export const defaultColorAdjust: VideoColorAdjust = {
  brightness: 0,
  contrast: 1,
  hue_deg: 0,
  saturation: 1,
  gamma: 1,
};

export const defaultFxAdjust: VideoFxAdjust = {
  pixelate: 1,
  blur: 0,
  glow: 0,
  edge: 0,
  key_red: 0,
  key_green: 1,
  key_blue: 0,
  key_threshold: 0,
};

export const defaultVideoCuePointColors = [
  "#4aa8ff",
  "#74d99f",
  "#f2c14e",
  "#ff6b6b",
  "#b16cff",
  "#22d3ee",
];
