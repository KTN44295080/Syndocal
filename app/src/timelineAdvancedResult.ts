import type {
  EngineSnapshot,
  TimelineAdvancedAuthoritativeResult,
  TimelineItemRef,
  TimelineSnapshot,
} from "./types";

export const timelineAdvancedActiveTimeline = (
  result: TimelineAdvancedAuthoritativeResult,
): TimelineSnapshot => {
  const active = result.timeline_bank.find((timeline) => timeline.id === result.active_timeline_id);
  if (!active) throw new Error("Timeline mutation result omitted its active Timeline.");
  return active;
};

/** Apply an acknowledged Timeline bank without disturbing unrelated live state. */
export const engineSnapshotWithTimelineAdvancedResult = (
  current: EngineSnapshot,
  result: TimelineAdvancedAuthoritativeResult,
): EngineSnapshot => ({
  ...current,
  timeline: timelineAdvancedActiveTimeline(result),
  timeline_bank: result.timeline_bank,
});

export const specializedTimelineSelectionFromItems = (items: TimelineItemRef[]) => {
  const event = items.find((item) => item.kind === "lighting_event");
  const automation = items.find((item) => item.kind === "lighting_automation" || item.kind === "video_automation");
  return {
    event_id: event?.kind === "lighting_event" ? event.event_id : null,
    automation: automation?.kind === "lighting_automation"
      ? { kind: "lighting" as const, automationId: automation.automation_id }
      : automation?.kind === "video_automation"
        ? { kind: "video" as const, automationId: automation.automation_id }
        : null,
  };
};
