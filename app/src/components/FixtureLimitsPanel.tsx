import type { JSX } from "solid-js";
import { handleMovementLimitKey } from "../fixtureLimitKeyboard";
import type { FixtureLimits, PatchedFixtureSummary } from "../types";
import { DimmerLimitMeter } from "./FixtureLimitVisuals";

type MaybePromise = void | Promise<unknown>;
type NumericFixtureLimitField = "dimmer_min" | "dimmer_max" | "pan_min" | "pan_max" | "tilt_min" | "tilt_max";
type ToggleFixtureLimitField = "invert_pan" | "invert_tilt" | "swap_pan_tilt";
type MovementLimitPointerEvent = PointerEvent & { currentTarget: HTMLElement };

export interface FixtureLimitsPanelProps {
  fixture: PatchedFixtureSummary;
  limitsDraft: FixtureLimits;
  normalizedLimits: FixtureLimits;
  limitWindowStyle: JSX.CSSProperties;
  movementLimitDragging: boolean;
  movementLimitNudgeAmount: number;
  formatDmxPercent: (value: number) => string;
  onUpdateNumericLimit: (field: NumericFixtureLimitField, value: number) => void;
  onUpdateToggleLimit: (field: ToggleFixtureLimitField, value: boolean) => void;
  onResetLimits: () => void;
  onResetMovementLimits: () => void;
  onApplyLimits: (fixture: PatchedFixtureSummary) => MaybePromise;
  onMovementLimitPointerDown: (event: MovementLimitPointerEvent) => void;
  onMovementLimitPointerMove: (event: MovementLimitPointerEvent) => void;
  onMovementLimitPointerEnd: (event: MovementLimitPointerEvent) => void;
}

export function FixtureLimitsPanel(props: FixtureLimitsPanelProps) {
  const handleMovementLimitKeyDown = (event: KeyboardEvent) => {
    handleMovementLimitKey(event, {
      limits: props.normalizedLimits,
      baseAmount: props.movementLimitNudgeAmount,
      onUpdate: props.onUpdateNumericLimit,
      onReset: props.onResetMovementLimits,
    });
  };

  return (
    <div class="limitEditor fixtureLimitsEditor" data-fixture-limits-editor>
      <div class="limitEditorHeader">
        <strong>Dimmer Limits</strong>
        <span>
          {props.formatDmxPercent(props.normalizedLimits.dimmer_min)} -{" "}
          {props.formatDmxPercent(props.normalizedLimits.dimmer_max)}
        </span>
      </div>
      <div class="dimmerLimitEditor">
        <DimmerLimitMeter
          minimum={props.normalizedLimits.dimmer_min}
          maximum={props.normalizedLimits.dimmer_max}
        />
        <div class="split">
          <label>
            Min
            <input
              type="number"
              min="0"
              max="65535"
              value={props.limitsDraft.dimmer_min}
              onInput={(event) => props.onUpdateNumericLimit("dimmer_min", Number(event.currentTarget.value))}
            />
          </label>
          <label>
            Max
            <input
              type="number"
              min="0"
              max="65535"
              value={props.limitsDraft.dimmer_max}
              onInput={(event) => props.onUpdateNumericLimit("dimmer_max", Number(event.currentTarget.value))}
            />
          </label>
        </div>
      </div>
      <div class="limitEditorHeader">
        <strong>Movement Limits</strong>
        <span>
          Pan {props.formatDmxPercent(props.normalizedLimits.pan_min)} -{" "}
          {props.formatDmxPercent(props.normalizedLimits.pan_max)}
        </span>
      </div>
      <div class="movementLimitEditor">
        <div
          class={props.movementLimitDragging ? "movementLimitMap dragging" : "movementLimitMap"}
          data-movement-limit-map
          aria-label="Pan tilt movement limits"
          role="group"
          tabIndex={0}
          title="Arrow keys move limits, PageUp/+ expands, PageDown/- shrinks, Shift coarse, Alt fine, Home reset."
          onPointerDown={props.onMovementLimitPointerDown}
          onPointerMove={props.onMovementLimitPointerMove}
          onPointerUp={props.onMovementLimitPointerEnd}
          onPointerCancel={props.onMovementLimitPointerEnd}
          onKeyDown={handleMovementLimitKeyDown}
        >
          <i style={props.limitWindowStyle} />
        </div>
        <div class="movementLimitFields">
          <div class="split">
            <label>
              Pan Min
              <input
                type="number"
                min="0"
                max="65535"
                value={props.limitsDraft.pan_min}
                onInput={(event) => props.onUpdateNumericLimit("pan_min", Number(event.currentTarget.value))}
              />
            </label>
            <label>
              Pan Max
              <input
                type="number"
                min="0"
                max="65535"
                value={props.limitsDraft.pan_max}
                onInput={(event) => props.onUpdateNumericLimit("pan_max", Number(event.currentTarget.value))}
              />
            </label>
          </div>
          <div class="split">
            <label>
              Tilt Min
              <input
                type="number"
                min="0"
                max="65535"
                value={props.limitsDraft.tilt_min}
                onInput={(event) => props.onUpdateNumericLimit("tilt_min", Number(event.currentTarget.value))}
              />
            </label>
            <label>
              Tilt Max
              <input
                type="number"
                min="0"
                max="65535"
                value={props.limitsDraft.tilt_max}
                onInput={(event) => props.onUpdateNumericLimit("tilt_max", Number(event.currentTarget.value))}
              />
            </label>
          </div>
          <div class="limitToggleRow">
            <label>
              <input
                type="checkbox"
                checked={props.limitsDraft.invert_pan}
                onChange={(event) => props.onUpdateToggleLimit("invert_pan", event.currentTarget.checked)}
              />
              Invert Pan
            </label>
            <label>
              <input
                type="checkbox"
                checked={props.limitsDraft.invert_tilt}
                onChange={(event) => props.onUpdateToggleLimit("invert_tilt", event.currentTarget.checked)}
              />
              Invert Tilt
            </label>
            <label>
              <input
                type="checkbox"
                checked={props.limitsDraft.swap_pan_tilt}
                onChange={(event) => props.onUpdateToggleLimit("swap_pan_tilt", event.currentTarget.checked)}
              />
              Swap
            </label>
          </div>
        </div>
      </div>
      <div class="presetRow">
        <button onClick={props.onResetLimits}>Reset</button>
        <button class="primary" onClick={() => void props.onApplyLimits(props.fixture)}>
          Apply Limits
        </button>
      </div>
    </div>
  );
}
