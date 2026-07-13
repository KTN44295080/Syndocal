import { Show } from "solid-js";
import type { EffectBlendMode } from "../types";

interface EffectActionControlsPanelProps {
  showLightRange: boolean;
  showPhase?: boolean;
  lockBlendMode?: boolean;
  low: number;
  high: number;
  phase: number;
  blendMode: EffectBlendMode;
  addDisabled: boolean;
  submitLabel: string;
  editing: boolean;
  editingLabel: string | null;
  onLow: (low: number) => void;
  onHigh: (high: number) => void;
  onPhase: (phase: number) => void;
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
      <div class="effectSubmitActions">
        <Show when={props.editingLabel}>
          {(label) => (
            <span class="effectEditBadge" title={`Editing ${label()}`}>
              Editing {label()}
            </span>
          )}
        </Show>
        <button class="primary" onClick={() => void props.onSubmitEffect()} disabled={props.addDisabled}>
          {props.submitLabel}
        </button>
        <Show when={props.editing}>
          <button onClick={props.onCancelEdit}>Cancel Edit</button>
        </Show>
      </div>
    </div>
  );
}
