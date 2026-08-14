import type { TimelineItemGroupSummary, TimelineItemRef } from "./types";

export const timelineItemKey = (item: TimelineItemRef): string => {
  switch (item.kind) {
    case "video_clip": return `video:${item.clip_id}`;
    case "audio_clip": return `audio:${item.clip_id}`;
    case "lighting_event": return `event:${item.event_id}`;
    case "lighting_automation": return `lighting-automation:${item.automation_id}`;
    case "video_automation": return `video-automation:${item.automation_id}`;
  }
};

export const expandTimelineItemGroupSelection = (
  item: TimelineItemRef,
  groups: TimelineItemGroupSummary[],
  existing: TimelineItemRef[] = [],
): TimelineItemRef[] => {
  const group = groups.find((candidate) => candidate.members.some((member) =>
    timelineItemKey(member) === timelineItemKey(item)));
  const next = [...existing];
  const keys = new Set(next.map(timelineItemKey));
  for (const member of group?.members ?? [item]) {
    const key = timelineItemKey(member);
    if (!keys.has(key)) {
      keys.add(key);
      next.push(member);
    }
  }
  return next;
};
