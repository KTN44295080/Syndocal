import { Show } from "solid-js";
import type { EffectBlendMode } from "../types";

interface EffectActionControlsPanelProps {
  showLightRange: boolean;
  showPhase?: boolean;
  showFixtureSpread?: boolean;
  lockBlendMode?: boolean;
  low: number;
  high: number;
  phase: number;
  fixtureSpread?: number;
  blendMode: EffectBlendMode;
  addDisabled: boolean;
  submitLabel: string;
  editing: boolean;
  /** Scene FX: the form differs from the saved FX (undefined = not tracked). */
  dirty?: boolean;
  /** Scene FX: whether settled edits reach the output ("live") or not ("blind"). */
  outputMode?: "live" | "blind";
  editingLabel: string | null;
  onLow: (low: number) => void;
  onHigh: (high: number) => void;
  onPhase: (phase: number) => void;
  onFixtureSpread?: (spread: number) => void;
  onBlendMode: (blendMode: EffectBlendMode) => void;
  onSubmitEffect: () => void | Promise<void>;
  onCancelEdit: () => void;
}

export function EffectActionControlsPanel(props: EffectActionControlsPanelProps) {
  return (
    <div class="effectActionDock">
      <Show when={props.showLightRange}>
        <div class="split">
          <label>
            Low
            <input type="number" min="0" max="65535" value={props.low} onInput={(event) => props.onLow(Number(event.currentTarget.value))} />
          </label>
          <label>
            High
            <input type="number" min="0" max="65535" value={props.high} onInput={(event) => props.onHigh(Number(event.currentTarget.value))} />
          </label>
        </div>
      </Show>
      <Show when={props.showPhase !== false}>
      <label>
        Phase
        <input
          type="number"
          min="0"
          max="1"
          step="0.01"
          value={props.phase}
          onInput={(event) => props.onPhase(Number(event.currentTarget.value))}
        />
      </label>
      </Show>
      <Show when={props.showFixtureSpread && props.onFixtureSpread}>
        <label>
          Fixture phasing %
          <input
            type="number"
            min="0"
            max="100"
            step="1"
            value={Math.round((props.fixtureSpread ?? 0) * 100)}
            aria-label="LFO fixture phasing percent"
            onInput={(event) => props.onFixtureSpread?.(Math.min(1, Math.max(0, Number(event.currentTarget.value) / 100)))}
          />
        </label>
      </Show>
      <label>
        Blend
        <select
          value={props.blendMode}
          disabled={props.lockBlendMode}
          title={props.lockBlendMode ? "Move effects use Override blend" : undefined}
          onInput={(event) => props.onBlendMode(event.currentTarget.value as EffectBlendMode)}
        >
          <option value="Override">Override</option>
          <option value="Add">Add</option>
          <option value="Multiply">Multiply</option>
        </select>
      </label>
      <div class="effectSubmitActions" data-effect-dirty={props.dirty ? "true" : undefined}>
        <Show when={props.outputMode}>
          {(mode) => (
            <span
              class={`effectOutputModeBadge ${mode()}`}
              data-effect-output-mode={mode()}
              title={mode() === "live"
                ? "Edits are saved automatically and a playing scene shows them at once. Turn on Blind to edit without changing the output."
                : "Blind: edits are saved but the output does not change until the scene is triggered again."}
            >
              {mode() === "live" ? "LIVE" : "BLIND"}
            </span>
          )}
        </Show>
        <Show when={props.editingLabel}>
          {(label) => (
            <span class="effectEditBadge" title={`Editing ${label()}`}>
              <Show when={props.dirty}>
                <span class="effectUnsavedMark" data-effect-unsaved title="Unsaved changes">Unsaved</span>{" "}
              </Show>
              Editing {label()}
            </span>
          )}
        </Show>
        <button class="primary" onClick={() => void props.onSubmitEffect()} disabled={props.addDisabled}>
          {props.submitLabel}
        </button>
        <Show when={props.editing}>
          <button onClick={props.onCancelEdit} disabled={props.dirty === false} title={props.dirty === false ? "No unsaved changes" : undefined}>Cancel Edit</button>
        </Show>
      </div>
    </div>
  );
}
