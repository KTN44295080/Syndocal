import type {
  EffectBlendMode,
  MoveCoordinateMode,
  MovePathPoint,
} from "./types";

export const movePathRecipes = ["Circle", "Line", "Triangle", "Square", "Figure Eight"] as const;
export type MovePathPreset = (typeof movePathRecipes)[number];
export type MovePathRecipe = MovePathPreset | "Custom";

const roundPathCoordinate = (value: number) => Number(value.toFixed(4));

export const clampMoveUnit = (value: number) =>
  Math.min(1, Math.max(0, Number.isFinite(value) ? value : 0));

export const clampMoveSize = (value: number) =>
  Math.min(2, Math.max(0.01, Number.isFinite(value) ? value : 1));

const circlePoints = (count: number): MovePathPoint[] =>
  Array.from({ length: count }, (_, index) => {
    const angle = -Math.PI / 2 + (index / count) * Math.PI * 2;
    return {
      x: roundPathCoordinate(0.5 + Math.cos(angle) * 0.36),
      y: roundPathCoordinate(0.5 + Math.sin(angle) * 0.36),
    };
  });

export const movePathRecipePoints = (recipe: MovePathPreset): MovePathPoint[] => {
  switch (recipe) {
    case "Line":
      return [
        { x: 0.15, y: 0.5 },
        { x: 0.85, y: 0.5 },
      ];
    case "Triangle":
      return [
        { x: 0.5, y: 0.12 },
        { x: 0.88, y: 0.84 },
        { x: 0.12, y: 0.84 },
      ];
    case "Square":
      return [
        { x: 0.15, y: 0.15 },
        { x: 0.85, y: 0.15 },
        { x: 0.85, y: 0.85 },
        { x: 0.15, y: 0.85 },
      ];
    case "Figure Eight":
      return Array.from({ length: 16 }, (_, index) => {
        const angle = (index / 16) * Math.PI * 2;
        return {
          x: roundPathCoordinate(0.5 + Math.sin(angle) * 0.38),
          y: roundPathCoordinate(0.5 + Math.sin(angle * 2) * 0.22),
        };
      });
    case "Circle":
    default:
      return circlePoints(12);
  }
};

export const defaultMovePathPoints = () => movePathRecipePoints("Circle");

interface MoveDraftValidationInput {
  fixtureIds: number[];
  targetGroupIds: string[];
  points: MovePathPoint[];
  centerX: number;
  centerY: number;
  sizeX: number;
  sizeY: number;
  rotationDegrees: number;
  periodMs: number;
  clockSyncBeats: number | null;
  phase: number;
  fixtureSpread: number;
  coordinateMode: MoveCoordinateMode;
  blendMode: EffectBlendMode;
}

export const moveEffectDraftError = (input: MoveDraftValidationInput) => {
  if (input.fixtureIds.length === 0 && input.targetGroupIds.length === 0) {
    return "Move effects require at least one fixture or group target.";
  }
  if (input.points.length < 2 || input.points.length > 256) {
    return "Move paths require between 2 and 256 points.";
  }
  if (input.points.some((point) =>
    !Number.isFinite(point.x)
    || !Number.isFinite(point.y)
    || point.x < 0
    || point.x > 1
    || point.y < 0
    || point.y > 1
  )) {
    return "Move path points must stay within the normalized 0..1 canvas.";
  }
  const hasDistinctPoint = input.points.some((point) => {
    const first = input.points[0];
    return Math.hypot(point.x - first.x, point.y - first.y) > 0.000001;
  });
  if (!hasDistinctPoint) {
    return "Move paths require at least two distinct points.";
  }
  if (![input.centerX, input.centerY].every((value) => Number.isFinite(value) && value >= 0 && value <= 1)) {
    return "Move path center must stay within 0..1.";
  }
  if (![input.sizeX, input.sizeY].every((value) => Number.isFinite(value) && value >= 0.01 && value <= 2)) {
    return "Move path size must stay within 0.01..2.";
  }
  if (!Number.isFinite(input.rotationDegrees)) {
    return "Move path rotation must be finite.";
  }
  if (!Number.isFinite(input.periodMs) || input.periodMs < 10) {
    return "Move free-running period must be at least 10 ms.";
  }
  if (input.clockSyncBeats !== null && (!Number.isFinite(input.clockSyncBeats) || input.clockSyncBeats <= 0)) {
    return "Move beat division must be greater than 0.";
  }
  if (!Number.isFinite(input.phase) || input.phase < 0 || input.phase > 1) {
    return "Move phase must stay within 0..1.";
  }
  if (!Number.isFinite(input.fixtureSpread) || input.fixtureSpread < 0 || input.fixtureSpread > 1) {
    return "Move fixture spread must stay within 0..1.";
  }
  if (input.blendMode !== "Override") {
    return "Move effects control paired Pan/Tilt output and require Override blend.";
  }
  return "";
};
