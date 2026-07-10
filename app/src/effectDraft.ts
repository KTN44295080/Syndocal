import type { EffectSummary, PatchedFixtureSummary, VideoEffectTarget } from "./types";

export type EffectDraftTargetMode = "video" | "group" | "selection" | "fixture" | "source";

export interface EffectDraftTargetPlan {
  mode: EffectDraftTargetMode;
  videoTarget: VideoEffectTarget | null;
  videoTargetLinked: boolean;
  fixtureIds: number[];
  activeFixtureId: number | null;
  targetGroups: string;
  attribute: string;
  omittedVideoBindingCount: number;
  message: string;
}

const videoBindingCount = (effect: EffectSummary) =>
  effect.video_targets.reduce((count, target) => count + target.layer_ids.length, 0);

const draftMessage = (effectId: number, description: string, omittedVideoBindingCount: number) =>
  `Loaded effect ${effectId} into ${description}.${
    omittedVideoBindingCount > 0
      ? ` ${omittedVideoBindingCount} additional video binding${omittedVideoBindingCount === 1 ? "" : "s"} remain active but are not represented by the single-target editor.`
      : ""
  }`;

export const effectDraftTargetPlan = (
  effect: EffectSummary,
  fixtures: Pick<PatchedFixtureSummary, "id">[],
): EffectDraftTargetPlan => {
  const videoTarget = effect.video_targets[0] ?? null;
  const representedVideoBindingCount = videoTarget?.layer_ids[0] === undefined ? 0 : 1;
  const omittedVideoBindingCount = Math.max(0, videoBindingCount(effect) - representedVideoBindingCount);
  const hasLightTarget = effect.fixture_ids.length > 0 || effect.target_group_ids.length > 0;
  const base = {
    videoTarget,
    fixtureIds: [] as number[],
    activeFixtureId: null as number | null,
    targetGroups: "",
    attribute: effect.attribute,
    omittedVideoBindingCount,
  };

  if (!hasLightTarget && videoTarget) {
    return {
      ...base,
      mode: "video",
      videoTargetLinked: false,
      message: draftMessage(effect.id, "the video effect draft", omittedVideoBindingCount),
    };
  }

  if (effect.target_group_ids.length > 0) {
    return {
      ...base,
      mode: "group",
      videoTargetLinked: Boolean(videoTarget),
      targetGroups: effect.target_group_ids.join(", "),
      message: draftMessage(
        effect.id,
        videoTarget ? "the mixed group + video draft" : "the group effect draft",
        omittedVideoBindingCount,
      ),
    };
  }

  const availableFixtureIds = new Set(fixtures.map((fixture) => fixture.id));
  const fixtureIds = effect.fixture_ids.filter((fixtureId) => availableFixtureIds.has(fixtureId));
  if (fixtureIds.length > 1) {
    return {
      ...base,
      mode: "selection",
      videoTargetLinked: Boolean(videoTarget),
      fixtureIds,
      activeFixtureId: fixtureIds[0],
      message: draftMessage(
        effect.id,
        videoTarget ? "the mixed map selection + video draft" : "the map selection draft",
        omittedVideoBindingCount,
      ),
    };
  }

  if (fixtureIds.length === 1) {
    return {
      ...base,
      mode: "fixture",
      videoTargetLinked: Boolean(videoTarget),
      fixtureIds,
      activeFixtureId: fixtureIds[0],
      message: draftMessage(
        effect.id,
        videoTarget ? "the mixed fixture + video draft" : "the fixture effect draft",
        omittedVideoBindingCount,
      ),
    };
  }

  return {
    ...base,
    mode: "source",
    videoTargetLinked: false,
    message: draftMessage(effect.id, "the source settings draft", omittedVideoBindingCount),
  };
};
