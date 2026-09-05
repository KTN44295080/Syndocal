// Stage 2D coordinate helpers extracted from App.tsx.
// Pure conversions between stage-world coordinates and the SVG view-box used by the
// Setup mapping view. No SolidJS/state deps.
import { clamp01 } from "./numericHelpers";

export interface StageWorldBounds {
  minX: number;
  maxX: number;
  minZ: number;
  maxZ: number;
}

export const stageViewBoxSize = 100;
export const stagePadding = 10;

export interface MappingStageSvgFrame {
  originX: number;
  originZ: number;
  worldToSvgScale: number;
}

// The editable Mapping surface must preserve one physical scale for both world
// axes. The viewport may be rectangular, but a 5 m X step and a 5 m Z step must
// occupy the same SVG distance so fixtures, grid cells, and drag deltas cannot
// drift apart or change proportions as the viewport zooms.
export const mappingStageSvgFrame = (bounds: StageWorldBounds): MappingStageSvgFrame => {
  const drawableSize = stageViewBoxSize - stagePadding * 2;
  const rangeX = Math.max(Number.EPSILON, bounds.maxX - bounds.minX);
  const rangeZ = Math.max(Number.EPSILON, bounds.maxZ - bounds.minZ);
  return {
    originX: stageViewBoxSize / 2 - ((bounds.minX + bounds.maxX) / 2) * Math.min(drawableSize / rangeX, drawableSize / rangeZ),
    originZ: stageViewBoxSize / 2 - ((bounds.minZ + bounds.maxZ) / 2) * Math.min(drawableSize / rangeX, drawableSize / rangeZ),
    worldToSvgScale: Math.min(drawableSize / rangeX, drawableSize / rangeZ),
  };
};

export const mappingStageWorldToSvgPoint = (x: number, z: number, bounds: StageWorldBounds) => {
  const frame = mappingStageSvgFrame(bounds);
  return {
    x: frame.originX + x * frame.worldToSvgScale,
    z: frame.originZ + z * frame.worldToSvgScale,
  };
};

export const mappingStageSvgPointToWorld = (x: number, z: number, bounds: StageWorldBounds) => {
  const frame = mappingStageSvgFrame(bounds);
  return {
    x: (x - frame.originX) / frame.worldToSvgScale,
    z: (z - frame.originZ) / frame.worldToSvgScale,
  };
};

export const mappingStageSvgDeltaToWorld = (x: number, z: number, bounds: StageWorldBounds) => {
  const scale = mappingStageSvgFrame(bounds).worldToSvgScale;
  return { x: x / scale, z: z / scale };
};

export const stageWorldToSvgPoint = (x: number, z: number, bounds: StageWorldBounds) => {
  const drawableSize = stageViewBoxSize - stagePadding * 2;
  const rangeX = Math.max(Number.EPSILON, bounds.maxX - bounds.minX);
  const rangeZ = Math.max(Number.EPSILON, bounds.maxZ - bounds.minZ);
  return {
    x: stagePadding + ((x - bounds.minX) / rangeX) * drawableSize,
    z: stagePadding + ((z - bounds.minZ) / rangeZ) * drawableSize,
  };
};

export const svgPointToStageWorld = (x: number, z: number, bounds: StageWorldBounds) => {
  const drawableSize = stageViewBoxSize - stagePadding * 2;
  const normalizedX = clamp01((x - stagePadding) / drawableSize);
  const normalizedZ = clamp01((z - stagePadding) / drawableSize);
  return {
    x: bounds.minX + normalizedX * (bounds.maxX - bounds.minX),
    z: bounds.minZ + normalizedZ * (bounds.maxZ - bounds.minZ),
  };
};

export const svgDeltaToStageWorld = (x: number, z: number, bounds: StageWorldBounds) => {
  const drawableSize = stageViewBoxSize - stagePadding * 2;
  return {
    x: (x / drawableSize) * (bounds.maxX - bounds.minX),
    z: (z / drawableSize) * (bounds.maxZ - bounds.minZ),
  };
};
