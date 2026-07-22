import { For, Show } from "solid-js";
import type { CueLiveDirection, CueLiveModifierState, CueSummary } from "../types";
import {
  CUE_LIVE_MODIFIER_SIZE_MAX,
  cueLiveModifierIsOverridden,
  effectiveCueLiveModifier,
  formatLiveModifierPhase,
  formatLiveModifierSize,
  formatLiveModifierSpeed,
} from "../cueLiveModifier";

type MaybePromise = void | Promise<void>;

interface CueLiveModifierStripProps {
  cue: CueSummary;
  liveStates?: CueLiveModifierState[];
  touch?: boolean;
  onSetCueLiveModifier: (
    cueId: number,
    speed: number,
    size: number,
    phase: number,
    direction: CueLiveDirection,
    segment: number,
  ) => MaybePromise;
  onClearCueLiveModifier: (cueId: number) => MaybePromise;
}

/**
 * T17 latched live-modifier dials for one active scene. The Scene Matrix cell
 * and the Touch cue deck render this same strip so both surfaces drive the
 * identical runtime override path. The speed slider covers the musical
 * 0.05..4x span; wider engine-clamped values keep showing in the readout.
 */
export function CueLiveModifierStrip(props: CueLiveModifierStripProps) {
  const effective = () => effectiveCueLiveModifier(props.cue, props.liveStates);
  const overridden = () => cueLiveModifierIsOverridden(props.cue, props.liveStates);
  const apply = (part: Partial<{
    speed: number;
    size: number;
    phase: number;
    direction: CueLiveDirection;
    segment: number;
  }>) => {
    const current = effective();
    void props.onSetCueLiveModifier(
      props.cue.id,
      part.speed ?? current.speed,
      part.size ?? current.size,
      part.phase ?? current.phase,
      part.direction ?? current.direction ?? "Authored",
      part.segment ?? current.segment ?? 0,
    );
  };
  const stop = (event: Event) => event.stopPropagation();
  return (
    <div
      class="cueLiveModifierStrip"
      classList={{ touch: props.touch, overridden: overridden() }}
      data-cue-live-modifier={props.cue.id}
      data-live-override={overridden() ? "true" : "false"}
      onPointerDown={stop}
      onPointerMove={stop}
      onPointerUp={stop}
      onPointerCancel={stop}
    >
      <label class="cueLiveModifierRow">
        <span>Speed</span>
        <input
          type="range"
          min="0.05"
          max="4"
          step="0.05"
          value={Math.min(effective().speed, 4)}
          data-cue-live-modifier-speed={props.cue.id}
          aria-label={`Live speed for Cue ${props.cue.label}`}
          onInput={(event) => apply({ speed: Number(event.currentTarget.value) })}
        />
        <b data-no-localize>{formatLiveModifierSpeed(effective().speed)}</b>
      </label>
      <label class="cueLiveModifierRow">
        <span>Size</span>
        <input
          type="range"
          min="0"
          max={CUE_LIVE_MODIFIER_SIZE_MAX}
          step="0.05"
          value={effective().size}
          data-cue-live-modifier-size={props.cue.id}
          aria-label={`Live size for Cue ${props.cue.label}`}
          onInput={(event) => apply({ size: Number(event.currentTarget.value) })}
        />
        <b data-no-localize>{formatLiveModifierSize(effective().size)}</b>
      </label>
      <label class="cueLiveModifierRow">
        <span>Phase</span>
        <input
          type="range"
          min="0"
          max="1"
          step="0.01"
          value={effective().phase}
          data-cue-live-modifier-phase={props.cue.id}
          aria-label={`Live phase for Cue ${props.cue.label}`}
          onInput={(event) => apply({ phase: Number(event.currentTarget.value) })}
        />
        <b data-no-localize>{formatLiveModifierPhase(effective().phase)}</b>
      </label>
      <div class="cueLiveModifierRow cueLiveModifierModeRow">
        <span>Play</span>
        <select
          value={effective().direction ?? "Authored"}
          data-cue-live-modifier-direction={props.cue.id}
          aria-label={`Live direction for Cue ${props.cue.label}`}
          onInput={(event) => apply({ direction: event.currentTarget.value as CueLiveDirection })}
        >
          <option value="Authored">Authored</option>
          <option value="Forward">Forward</option>
          <option value="Reverse">Reverse</option>
          <option value="Bounce">Bounce</option>
        </select>
        <select
          value={effective().segment ?? 0}
          disabled={(props.cue.steps?.length ?? 0) < 2}
          data-cue-live-modifier-segment={props.cue.id}
          aria-label={`Live segment for Cue ${props.cue.label}`}
          onInput={(event) => apply({ segment: Number(event.currentTarget.value) })}
        >
          <option value="0">Auto</option>
          <For each={props.cue.steps ?? []}>
            {(_, index) => <option value={index() + 1}>{index() + 1}</option>}
          </For>
        </select>
      </div>
      <div class="cueLiveModifierFooter">
        <Show when={overridden()}>
          <span class="cueLiveModifierLiveBadge" data-no-localize>
            LIVE
          </span>
        </Show>
        <button
          type="button"
          class="cueLiveModifierReset"
          data-cue-live-modifier-reset={props.cue.id}
          disabled={!overridden()}
          aria-label={`Reset live modifier for Cue ${props.cue.label}`}
          onClick={() => void props.onClearCueLiveModifier(props.cue.id)}
        >
          Reset
        </button>
      </div>
    </div>
  );
}
