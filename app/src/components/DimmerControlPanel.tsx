import { Show } from "solid-js";
import type { FixtureLimits } from "../types";

export interface DimmerControlSet {
  attribute: string;
  value: number;
}

type DimmerLimitField = "dimmer_min" | "dimmer_max";

interface DimmerControlPanelProps {
  control: DimmerControlSet;
  sliderMin: number;
  sliderMax: number;
  canEditLimits: boolean;
  limitsDraft: FixtureLimits;
  normalizedLimits: FixtureLimits;
  formatDmxPercent: (value: number) => string;
  applyLimitsLabel: string;
  onSetValue: (value: number) => void;
  onUpdateLimit: (field: DimmerLimitField, value: number) => void;
  onResetLimits: () => void;
  onApplyLimits: () => void;
}

export function DimmerControlPanel(props: DimmerControlPanelProps) {
  let bumpRestoreValue: number | null = null;
  const clampDmxValue = (value: number) => Math.min(65_535, Math.max(0, Math.round(Number.isFinite(value) ? value : 0)));
  const clampToSliderRange = (value: number) =>
    Math.min(Math.max(props.sliderMin, props.sliderMax), Math.max(Math.min(props.sliderMin, props.sliderMax), clampDmxValue(value)));
  const percentToDmxValue = (value: number) => clampDmxValue((Math.min(100, Math.max(0, Number.isFinite(value) ? value : 0)) / 100) * 65_535);
  const dmxValueToPercent = (value: number) => Math.round((clampDmxValue(value) / 65_535) * 1000) / 10;
  const sliderRatio = () => {
    const min = Math.min(props.sliderMin, props.sliderMax);
    const max = Math.max(props.sliderMin, props.sliderMax);
    if (max <= min) {
      return 0;
    }
    return (clampToSliderRange(props.control.value) - min) / (max - min);
  };
  const setLevelFromPointer = (event: PointerEvent & { currentTarget: HTMLElement }) => {
    const rect = event.currentTarget.getBoundingClientRect();
    const min = Math.min(props.sliderMin, props.sliderMax);
    const max = Math.max(props.sliderMin, props.sliderMax);
    const ratio = Math.min(1, Math.max(0, 1 - (event.clientY - rect.top) / Math.max(1, rect.height)));
    props.onSetValue(clampDmxValue(min + ratio * (max - min)));
  };
  const handleLevelKeyDown = (event: KeyboardEvent) => {
    const step = event.shiftKey ? 4096 : event.altKey ? 64 : 512;
    if (event.key === "ArrowUp" || event.key === "ArrowRight") {
      event.preventDefault();
      props.onSetValue(clampToSliderRange(props.control.value + step));
      return;
    }
    if (event.key === "ArrowDown" || event.key === "ArrowLeft") {
      event.preventDefault();
      props.onSetValue(clampToSliderRange(props.control.value - step));
      return;
    }
    if (event.key === "Home") {
      event.preventDefault();
      props.onSetValue(clampToSliderRange(props.sliderMin));
      return;
    }
    if (event.key === "End") {
      event.preventDefault();
      props.onSetValue(clampToSliderRange(props.sliderMax));
    }
  };
  const startBump = () => {
    if (bumpRestoreValue !== null) {
      return;
    }
    bumpRestoreValue = props.control.value;
    props.onSetValue(65_535);
  };
  const endBump = () => {
    if (bumpRestoreValue === null) {
      return;
    }
    const restoreValue = bumpRestoreValue;
    bumpRestoreValue = null;
    props.onSetValue(restoreValue);
  };

  return (
    <div class="visualControlPanel dimmerControlPanel">
      <div class="visualControlHeader">
        <div>
          <strong>Dimmer</strong>
          <span>{Math.round((props.control.value / 65535) * 1000) / 10}%</span>
        </div>
        <span>{props.control.attribute}</span>
      </div>
      <div class="dimmerQuickRow">
        <button onClick={() => props.onSetValue(0)}>Out</button>
        <button onClick={() => props.onSetValue(32768)}>Half</button>
        <button class="primary" onClick={() => props.onSetValue(65535)}>Full</button>
        <button
          class="momentary"
          onPointerDown={startBump}
          onPointerUp={endBump}
          onPointerCancel={endBump}
          onPointerLeave={endBump}
          onBlur={endBump}
          onKeyDown={(event) => {
            if (event.key === " " || event.key === "Enter") {
              event.preventDefault();
              startBump();
            }
          }}
          onKeyUp={(event) => {
            if (event.key === " " || event.key === "Enter") {
              event.preventDefault();
              endBump();
            }
          }}
      >
        Bump
      </button>
    </div>
      <div class="dimmerVisualRow">
        <div
          class="dimmerLevelPad"
          role="slider"
          aria-label="Dimmer visual level"
          aria-valuemin={Math.min(props.sliderMin, props.sliderMax)}
          aria-valuemax={Math.max(props.sliderMin, props.sliderMax)}
          aria-valuenow={clampToSliderRange(props.control.value)}
          aria-valuetext={`${dmxValueToPercent(props.control.value)}%`}
          tabIndex={0}
          onPointerDown={(event) => {
            event.currentTarget.setPointerCapture(event.pointerId);
            setLevelFromPointer(event);
          }}
          onPointerMove={(event) => {
            if (event.buttons === 1) {
              setLevelFromPointer(event);
            }
          }}
          onPointerUp={(event) => event.currentTarget.releasePointerCapture(event.pointerId)}
          onPointerCancel={(event) => event.currentTarget.releasePointerCapture(event.pointerId)}
          onKeyDown={handleLevelKeyDown}
        >
          <b style={{ height: `${sliderRatio() * 100}%` }} />
          <i style={{ bottom: `${sliderRatio() * 100}%` }} />
          <span class="dimmerLevelMax">{props.formatDmxPercent(props.sliderMax)}</span>
          <span class="dimmerLevelMin">{props.formatDmxPercent(props.sliderMin)}</span>
        </div>
        <div class="visualNumberGrid dimmerDirectGrid">
          <label>
            Level %
            <input
              type="number"
              min="0"
              max="100"
              step="0.1"
              value={dmxValueToPercent(props.control.value)}
              onInput={(event) => props.onSetValue(clampToSliderRange(percentToDmxValue(Number(event.currentTarget.value))))}
            />
          </label>
          <label>
            DMX
            <input
              type="number"
              min="0"
              max="65535"
              value={clampDmxValue(props.control.value)}
              onInput={(event) => props.onSetValue(clampToSliderRange(Number(event.currentTarget.value)))}
            />
          </label>
        </div>
      </div>
      <label class="dimmerVisualSlider">
        Level
        <input
          type="range"
          min={props.sliderMin}
          max={props.sliderMax}
          value={props.control.value}
          onInput={(event) => props.onSetValue(Number(event.currentTarget.value))}
        />
        <span>{props.control.value}</span>
      </label>
      <Show when={props.canEditLimits}>
        <div class="controlLimitPanel">
          <div class="limitEditorHeader">
            <strong>Dimmer Limits</strong>
            <span>
              {props.formatDmxPercent(props.normalizedLimits.dimmer_min)} -{" "}
              {props.formatDmxPercent(props.normalizedLimits.dimmer_max)}
            </span>
          </div>
          <div class="controlLimitRow">
            <label>
              Min
              <input
                type="number"
                min="0"
                max="65535"
                value={props.limitsDraft.dimmer_min}
                onInput={(event) => props.onUpdateLimit("dimmer_min", Number(event.currentTarget.value))}
              />
            </label>
            <label>
              Max
              <input
                type="number"
                min="0"
                max="65535"
                value={props.limitsDraft.dimmer_max}
                onInput={(event) => props.onUpdateLimit("dimmer_max", Number(event.currentTarget.value))}
              />
            </label>
            <button onClick={props.onResetLimits}>Full Range</button>
            <button class="primary" onClick={props.onApplyLimits}>
              {props.applyLimitsLabel}
            </button>
          </div>
        </div>
      </Show>
    </div>
  );
}
