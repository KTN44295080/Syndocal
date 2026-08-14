import type { TimelineItemRef } from "./types";

export interface TimelineDirectTrimGesture {
  item: TimelineItemRef;
  edge: "start" | "end";
  boundary_ms: number;
  isolate: boolean;
}

export const timelineDirectTrimGesture = (
  item: TimelineItemRef,
  linked: boolean,
  edge: "start" | "end",
  boundaryMs: number,
  isolate: boolean,
): TimelineDirectTrimGesture | null => linked
  ? {
      item,
      edge,
      boundary_ms: Math.max(0, Math.round(boundaryMs)),
      isolate,
    }
  : null;

export const applyTimelineDirectTrim = async (
  trimItems: (
    items: TimelineItemRef[],
    primary: TimelineItemRef,
    edge: "start" | "end",
    boundaryMs: number,
    isolate: boolean,
  ) => Promise<TimelineItemRef[]>,
  restoreSelection: (items: TimelineItemRef[]) => void,
  gesture: TimelineDirectTrimGesture,
) => {
  const selected = await trimItems(
    [gesture.item],
    gesture.item,
    gesture.edge,
    gesture.boundary_ms,
    gesture.isolate,
  );
  if (selected.length > 0) restoreSelection(selected);
  return selected;
};
