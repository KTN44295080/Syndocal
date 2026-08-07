import type { CueSummary } from "./types";

export type SceneCueKind = "TIMELINE" | "FX" | "STATIC";

export function sceneCueKind(
  cue: Pick<CueSummary, "child_timeline" | "effect_targets">,
): SceneCueKind {
  if (cue.child_timeline) return "TIMELINE";
  return cue.effect_targets.length > 0 ? "FX" : "STATIC";
}
