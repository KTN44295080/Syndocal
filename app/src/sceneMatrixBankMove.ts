import type { CueSummary } from "./types";

export const sceneMatrixCueMetadataArgs = (
  cue: CueSummary,
  groupId: string | null,
) => ({
  cueId: cue.id,
  cueNumber: cue.cue_number || String(cue.id),
  label: cue.label,
  groupId,
  recallMode: cue.recall_mode ?? "Coexist",
  fadeMs: cue.fade_ms,
  authoredBeats: cue.authored_beats ?? null,
  preWaitMs: cue.pre_wait_ms ?? 0,
  followMs: cue.follow_ms ?? null,
  ifcbTiming: {
    intensity_fade_ms: cue.ifcb_timing?.intensity_fade_ms ?? null,
    intensity_delay_ms: cue.ifcb_timing?.intensity_delay_ms ?? 0,
    focus_fade_ms: cue.ifcb_timing?.focus_fade_ms ?? null,
    focus_delay_ms: cue.ifcb_timing?.focus_delay_ms ?? 0,
    color_fade_ms: cue.ifcb_timing?.color_fade_ms ?? null,
    color_delay_ms: cue.ifcb_timing?.color_delay_ms ?? 0,
    beam_fade_ms: cue.ifcb_timing?.beam_fade_ms ?? null,
    beam_delay_ms: cue.ifcb_timing?.beam_delay_ms ?? 0,
  },
  parts: (cue.parts ?? []).map((part) => ({
    ...part,
    fixture_ids: [...part.fixture_ids],
    video_layer_ids: [...(part.video_layer_ids ?? [])],
    video_output_ids: [...(part.video_output_ids ?? [])],
  })),
  mark: cue.mark ?? false,
  mibFixtureIds: [...(cue.mib_fixture_ids ?? [])],
  tracking: cue.tracking ?? true,
  notes: cue.notes ?? "",
});

export const applySceneMatrixCueMove = (
  currentCues: CueSummary[],
  sourceCueId: number,
  targetGroupId: string | null,
  crossesBank: boolean,
  delta: -1 | 1,
  stepCount: number,
): CueSummary[] => {
  const next = currentCues.map((cue) =>
    cue.id === sourceCueId && crossesBank
      ? { ...cue, group_id: targetGroupId }
      : cue
  );
  for (let step = 0; step < stepCount; step += 1) {
    const cueIndex = next.findIndex((cue) => cue.id === sourceCueId);
    if (cueIndex < 0) break;
    const cueListId = next[cueIndex].cue_list_id;
    let adjacentIndex = cueIndex;
    if (delta < 0) {
      for (let index = cueIndex - 1; index >= 0; index -= 1) {
        if (next[index].cue_list_id === cueListId) {
          adjacentIndex = index;
          break;
        }
      }
    } else {
      for (let index = cueIndex + 1; index < next.length; index += 1) {
        if (next[index].cue_list_id === cueListId) {
          adjacentIndex = index;
          break;
        }
      }
    }
    if (adjacentIndex === cueIndex) break;
    [next[cueIndex], next[adjacentIndex]] = [next[adjacentIndex], next[cueIndex]];
  }
  return next;
};

/**
 * Apply the Scene Matrix's outer-bank move in the browser fixture. The older
 * helper above intentionally models the legacy group-column move; keep it
 * intact for that surface and make Cue List ownership explicit here.
 */
export const applySceneMatrixCueListMove = (
  currentCues: CueSummary[],
  sourceCueId: number,
  targetCueId: number | null,
  targetCueListId: number,
  targetGroupId: string | null,
  position: "before" | "after",
): CueSummary[] => {
  const source = currentCues.find((cue) => cue.id === sourceCueId);
  if (!source) return currentCues;
  const next = currentCues.map((cue) => cue.id === sourceCueId
    ? {
        ...cue,
        cue_list_id: targetCueListId,
        group_id: targetGroupId,
      }
    : cue);
  const sourceIndex = next.findIndex((cue) => cue.id === sourceCueId);
  if (sourceIndex < 0) return next;
  const [moved] = next.splice(sourceIndex, 1);
  if (!moved) return next;
  const targetIndex = targetCueId === null
    ? next.reduce((last, cue, index) => cue.cue_list_id === targetCueListId ? index : last, -1)
    : next.findIndex((cue) => cue.id === targetCueId);
  if (targetIndex < 0) {
    next.push(moved);
    return next;
  }
  next.splice(position === "after" ? targetIndex + 1 : targetIndex, 0, moved);
  return next;
};
