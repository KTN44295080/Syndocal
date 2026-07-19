export interface MoveEffectDragPoint {
  x: number;
  y: number;
}

export interface MoveEffectPointDragState {
  index: number;
  pointerId: number;
  startClientX: number;
  startClientY: number;
  surfaceWidth: number;
  surfaceHeight: number;
  originalPoint: MoveEffectDragPoint;
  draftPoint: MoveEffectDragPoint;
  moved: boolean;
}

export interface MoveEffectPointDragStart {
  index: number;
  pointerId: number;
  clientX: number;
  clientY: number;
  surfaceWidth: number;
  surfaceHeight: number;
  point: MoveEffectDragPoint;
}

export interface MoveEffectPointDragPosition {
  clientX: number;
  clientY: number;
}

export const moveEffectDragThresholdPx = 4;
export const moveEffectMinimumPoints = 2;
export const moveEffectMaximumPoints = 256;

const finiteOr = (value: number, fallback: number) => Number.isFinite(value) ? value : fallback;
const clampUnit = (value: number) => Math.min(1, Math.max(0, finiteOr(value, 0)));

export const normalizeMoveEffectDragPoint = (point: MoveEffectDragPoint): MoveEffectDragPoint => ({
  x: Number(clampUnit(point.x).toFixed(4)),
  y: Number(clampUnit(point.y).toFixed(4)),
});

export const moveEffectDragSurfaceSizeFromCtm = (
  matrix: Pick<DOMMatrix, "a" | "b" | "c" | "d"> | null,
  fallbackWidth: number,
  fallbackHeight: number,
  viewBoxWidth = 100,
  viewBoxHeight = 100,
) => {
  if (!matrix) {
    return {
      width: Math.max(0, finiteOr(fallbackWidth, 0)),
      height: Math.max(0, finiteOr(fallbackHeight, 0)),
    };
  }
  const scaleX = Math.hypot(finiteOr(matrix.a, 0), finiteOr(matrix.b, 0));
  const scaleY = Math.hypot(finiteOr(matrix.c, 0), finiteOr(matrix.d, 0));
  return {
    width: Math.max(0, scaleX * Math.max(0, finiteOr(viewBoxWidth, 100))),
    height: Math.max(0, scaleY * Math.max(0, finiteOr(viewBoxHeight, 100))),
  };
};

export const beginMoveEffectPointDrag = (
  start: MoveEffectPointDragStart,
): MoveEffectPointDragState => {
  const originalPoint = normalizeMoveEffectDragPoint(start.point);
  return {
    index: Math.max(0, Math.trunc(finiteOr(start.index, 0))),
    pointerId: start.pointerId,
    startClientX: finiteOr(start.clientX, 0),
    startClientY: finiteOr(start.clientY, 0),
    surfaceWidth: Math.max(0, finiteOr(start.surfaceWidth, 0)),
    surfaceHeight: Math.max(0, finiteOr(start.surfaceHeight, 0)),
    originalPoint,
    draftPoint: { ...originalPoint },
    moved: false,
  };
};

export const updateMoveEffectPointDrag = (
  drag: MoveEffectPointDragState,
  position: MoveEffectPointDragPosition,
  thresholdPx = moveEffectDragThresholdPx,
): MoveEffectPointDragState => {
  const clientX = finiteOr(position.clientX, drag.startClientX);
  const clientY = finiteOr(position.clientY, drag.startClientY);
  const deltaX = clientX - drag.startClientX;
  const deltaY = clientY - drag.startClientY;
  const moved = drag.moved || Math.hypot(deltaX, deltaY) > Math.max(0, finiteOr(thresholdPx, moveEffectDragThresholdPx));
  if (!moved) return { ...drag, moved: false };
  return {
    ...drag,
    moved: true,
    draftPoint: normalizeMoveEffectDragPoint({
      x: drag.originalPoint.x + (drag.surfaceWidth > 0 ? deltaX / drag.surfaceWidth : 0),
      y: drag.originalPoint.y - (drag.surfaceHeight > 0 ? deltaY / drag.surfaceHeight : 0),
    }),
  };
};

export const commitMoveEffectPointDrag = <Point extends MoveEffectDragPoint>(
  points: readonly Point[],
  drag: MoveEffectPointDragState,
): Point[] | null => {
  if (!drag.moved || !points[drag.index]) return null;
  return points.map((point, index) => index === drag.index
    ? { ...point, ...drag.draftPoint }
    : { ...point });
};
