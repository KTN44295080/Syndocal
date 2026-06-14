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

// Triangle points (in SVG view-box units) for a fixture's emitted beam, given its
// stage position, yaw, and 0..1 intensity.
export const beamPoints = (x: number, z: number, yawDegrees: number, intensity: number) => {
  const yaw = (yawDegrees * Math.PI) / 180;
  const angle = -Math.PI / 2 + yaw;
  const beamLength = 18 + intensity * 34;
  const beamWidth = 5 + intensity * 15;
  const tipX = x + Math.cos(angle) * beamLength;
  const tipZ = z + Math.sin(angle) * beamLength;
  const leftX = tipX + Math.cos(angle + Math.PI / 2) * beamWidth;
  const leftZ = tipZ + Math.sin(angle + Math.PI / 2) * beamWidth;
  const rightX = tipX + Math.cos(angle - Math.PI / 2) * beamWidth;
  const rightZ = tipZ + Math.sin(angle - Math.PI / 2) * beamWidth;
  return `${x},${z} ${leftX},${leftZ} ${rightX},${rightZ}`;
};
