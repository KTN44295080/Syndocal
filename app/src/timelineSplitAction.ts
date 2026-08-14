import type { TimelineItemRef } from "./types";

export interface TimelineSplitAtPlayheadAction {
  items: TimelineItemRef[];
  primary: TimelineItemRef;
  boundary_ms: number;
  isolate: boolean;
}

export const applyTimelineSplitAtPlayhead = async (
  splitItems: (
    items: TimelineItemRef[],
    primary: TimelineItemRef,
    boundaryMs: number,
    isolate: boolean,
  ) => Promise<TimelineItemRef[]>,
  restoreSelection: (items: TimelineItemRef[]) => void,
  restoreFocus: (item: TimelineItemRef) => void,
  action: TimelineSplitAtPlayheadAction,
) => {
  const selected = await splitItems(
    action.items,
    action.primary,
    Math.max(0, Math.round(action.boundary_ms)),
    action.isolate,
  );
  if (selected.length > 0) {
    restoreSelection(selected);
    restoreFocus(selected[0]);
  }
  return selected;
};
