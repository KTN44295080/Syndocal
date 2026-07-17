export interface TimelineCueDragState {
  cue_id: number;
  cue_label: string;
  pointer_id: number;
  start_client_x: number;
  start_client_y: number;
  client_x: number;
  client_y: number;
  moved: boolean;
}

export interface TimelineCueDragPoint {
  pointerId: number;
  clientX: number;
  clientY: number;
}

export const updateTimelineCueDrag = (
  drag: TimelineCueDragState,
  point: TimelineCueDragPoint,
  thresholdPx = 4,
): TimelineCueDragState => ({
  ...drag,
  client_x: point.clientX,
  client_y: point.clientY,
  moved: drag.moved
    || Math.hypot(point.clientX - drag.start_client_x, point.clientY - drag.start_client_y) >= thresholdPx,
});
