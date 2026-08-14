import type { TimelineAdvancedAuthoringSummary } from "./types";

export const removeTimelineAudioClipFromAdvancedAuthoring = (
  authoring: TimelineAdvancedAuthoringSummary,
  clipId: number,
): TimelineAdvancedAuthoringSummary => {
  const dissolvedGroupIds = new Set(authoring.item_groups
    .filter((group) => group.members.some((member) => member.kind === "audio_clip" && member.clip_id === clipId))
    .map((group) => group.id));
  return {
    ...authoring,
    audio_clips: authoring.audio_clips.filter((clip) => clip.id !== clipId),
    item_groups: authoring.item_groups.filter((group) => !dissolvedGroupIds.has(group.id)),
  };
};
