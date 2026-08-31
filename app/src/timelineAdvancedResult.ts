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

/**
 * Apply an acknowledged Timeline bank without promoting its authored-only
 * entries into live transport state.
 *
 * `timeline_bank` is deliberately runtime-free on the authoritative wire
 * result.  A same-identity browser fixture may still need the fresh authored
 * IDs immediately, so merge only the already-canonical runtime fields from
 * the current active Timeline.  If the acknowledged active ID differs, keep
 * the current active Timeline until a canonical `get_snapshot` installs the
 * new runtime image; showing a default-disabled bank entry as live state
 * would falsely report an enabled loop as OFF.
 */
export const engineSnapshotWithTimelineAdvancedResult = (
  current: EngineSnapshot,
  result: TimelineAdvancedAuthoritativeResult,
): EngineSnapshot => {
  const acknowledged = timelineAdvancedActiveTimeline(result);
  const currentTimeline = current.timeline;
  const timeline = currentTimeline.id === acknowledged.id
    ? {
        ...acknowledged,
        playing: currentTimeline.playing,
        position_ms: currentTimeline.position_ms,
        count_in_remaining_ms: currentTimeline.count_in_remaining_ms,
        transport_epoch: currentTimeline.transport_epoch,
        transport_generation: currentTimeline.transport_generation,
        loop_runtime: currentTimeline.loop_runtime,
        follow_runtime: currentTimeline.follow_runtime,
        guide_cues: currentTimeline.guide_cues,
      }
    : currentTimeline;
  return {
    ...current,
    timeline,
    timeline_bank: result.timeline_bank,
  };
};

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
