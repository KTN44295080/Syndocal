import type { VideoIsfEffectStageSummary, VideoIsfEffectSummary } from "./types";
import { boundedFxValue, cloneFxStage, type FxValue } from "./fxParameterModel";

/** Native limit for one layer chain (VIDEO_ISF_EFFECT_STACK_MAX_STAGES). */
export const FX_AUDITION_MAX_STAGES = 8;

export type FxChain = VideoIsfEffectSummary | null;

const plain = (stage: VideoIsfEffectStageSummary): VideoIsfEffectStageSummary => cloneFxStage(stage);

/** The ordered stages of a layer chain; the root stage is stage zero. */
export function fxChainStages(chain: FxChain | undefined): VideoIsfEffectStageSummary[] {
  return chain ? [chain, ...(chain.stack ?? [])].map(plain) : [];
}

export function fxChainFromStages(stages: readonly VideoIsfEffectStageSummary[]): FxChain {
  const [root, ...stack] = stages.map(plain);
  return root ? { ...root, stack } : null;
}

// Control values are f32 in the engine. A snapshot returns the shortest f32 decimal, so a
// double typed or randomized in the browser only matches it after rounding to f32.
const f32Image = (key: string, value: unknown) => typeof value === "number" && key !== "layer_id" ? Math.fround(value) : value;

export const fxChainsEqual = (left: FxChain | undefined, right: FxChain | undefined): boolean =>
  JSON.stringify(fxChainStages(left), f32Image) === JSON.stringify(fxChainStages(right), f32Image);

export const cloneFxChain = (chain: FxChain | undefined): FxChain => fxChainFromStages(fxChainStages(chain));

export function auditionAppendStage(chain: FxChain, stage: VideoIsfEffectStageSummary): FxChain {
  const stages = fxChainStages(chain);
  if (stages.length >= FX_AUDITION_MAX_STAGES) return chain;
  return fxChainFromStages([...stages, { ...plain(stage), enabled: true }]);
}

export function auditionSetEnabled(chain: FxChain, index: number, enabled: boolean): FxChain {
  const stages = fxChainStages(chain);
  if (!stages[index]) return chain;
  stages[index].enabled = enabled;
  return fxChainFromStages(stages);
}

export function auditionMoveStage(chain: FxChain, index: number, delta: number): FxChain {
  const stages = fxChainStages(chain);
  const target = index + delta;
  if (!stages[index] || target < 0 || target >= stages.length) return chain;
  [stages[index], stages[target]] = [stages[target], stages[index]];
  return fxChainFromStages(stages);
}

/** Moves one stage to a new position (insert, not swap), as a drag-and-drop reorder does. */
export function moveFxStageTo(chain: FxChain, from: number, to: number): FxChain {
  const stages = fxChainStages(chain);
  if (!stages[from] || to < 0 || to >= stages.length || from === to) return chain;
  const [moved] = stages.splice(from, 1);
  stages.splice(to, 0, moved);
  return fxChainFromStages(stages);
}

export function auditionRemoveStage(chain: FxChain, index: number): FxChain {
  const stages = fxChainStages(chain);
  if (!stages[index]) return chain;
  stages.splice(index, 1);
  return fxChainFromStages(stages);
}

/** Replaces one control value after applying the same range rules as the editor. */
export function auditionSetControl(chain: FxChain, index: number, name: string, value: readonly number[]): FxChain {
  const stages = fxChainStages(chain);
  const control = stages[index]?.controls.find((candidate) => candidate.name === name);
  if (!control) return chain;
  control.value = boundedFxValue(control, value) as FxValue;
  return fxChainFromStages(stages);
}

/** Restores every control of one stage to its declared default. */
export function auditionResetStage(chain: FxChain, index: number): FxChain {
  const stages = fxChainStages(chain);
  if (!stages[index]) return chain;
  // The declared default is a raw value, so it also ends any tempo sync (like the live Reset).
  stages[index].controls = stages[index].controls.map((control) => {
    const reset = { ...control, value: [...control.default] as FxValue };
    delete reset.tempo_sync;
    return reset;
  });
  return fxChainFromStages(stages);
}

export function auditionReplaceChain(next: FxChain): FxChain {
  return cloneFxChain(next);
}
