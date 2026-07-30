import { Show } from "solid-js";
import type { JSX } from "solid-js";

export interface HsvColor {
  hue: number;
  saturation: number;
  value: number;
}

interface HsvColorPickerProps {
  color: string;
  hsv: HsvColor;
  approximationLabel?: string;
  sideContent?: JSX.Element;
  onPointerColor: (event: PointerEvent) => void;
  onSetColor: (hexColor: string) => void;
  onSetHsv: (updates: Partial<HsvColor>) => void;
}

interface HsvColorAdjustmentsProps {
  hsv: HsvColor;
  saturationRamp: string;
  onSetHsv: (updates: Partial<HsvColor>) => void;
}

const clampPercent = (value: number) => Math.min(100, Math.max(0, Number.isFinite(value) ? value : 0));
const clampUnit = (value: number) => Math.min(1, Math.max(0, Number.isFinite(value) ? value : 0));
const wrapHue = (value: number) => ((value % 360) + 360) % 360;

const hueKeyboardStep = (event: KeyboardEvent) => {
  if (event.shiftKey) {
    return 10;
  }
  if (event.altKey) {
    return 0.25;
  }
  return 1;
};

const valueKeyboardStep = (event: KeyboardEvent) => {
  if (event.shiftKey) {
    return 0.05;
  }
  if (event.altKey) {
    return 0.005;
  }
  return 0.01;
};

export function HsvColorPicker(props: HsvColorPickerProps) {
  const handleColorPadKeyDown = (event: KeyboardEvent) => {
    if (event.key === "ArrowLeft") {
      event.preventDefault();
      props.onSetHsv({ hue: wrapHue(props.hsv.hue - hueKeyboardStep(event)) });
      return;
    }
    if (event.key === "ArrowRight") {
      event.preventDefault();
      props.onSetHsv({ hue: wrapHue(props.hsv.hue + hueKeyboardStep(event)) });
      return;
    }
    if (event.key === "ArrowUp") {
      event.preventDefault();
      props.onSetHsv({ value: clampUnit(props.hsv.value + valueKeyboardStep(event)) });
      return;
    }
    if (event.key === "ArrowDown") {
      event.preventDefault();
      props.onSetHsv({ value: clampUnit(props.hsv.value - valueKeyboardStep(event)) });
      return;
    }
    if (event.key === "Home") {
      event.preventDefault();
      props.onSetHsv({ hue: 0, value: 1 });
    }
  };

  return (
    <div class="colorPickerRow">
      <div
        class="colorPlane"
        role="slider"
        aria-label="Hue and brightness pad"
        aria-valuetext={props.color}
        aria-valuemin="0"
        aria-valuemax="360"
        aria-valuenow={Math.round(props.hsv.hue)}
        tabIndex={0}
        onPointerDown={(event) => {
          event.currentTarget.setPointerCapture(event.pointerId);
          props.onPointerColor(event);
        }}
        onPointerMove={(event) => {
          if (event.buttons === 1) {
            props.onPointerColor(event);
          }
        }}
        onPointerUp={(event) => event.currentTarget.releasePointerCapture(event.pointerId)}
        onPointerCancel={(event) => event.currentTarget.releasePointerCapture(event.pointerId)}
        onKeyDown={handleColorPadKeyDown}
      >
        <i
          style={{
            left: `${(props.hsv.hue / 360) * 100}%`,
            top: `${(1 - props.hsv.value) * 100}%`,
          }}
        />
      </div>
      <div class="colorPickerSide">
        <label>
          Color
          <input
            type="color"
            value={props.color}
            aria-label="Picker color"
            onInput={(event) => props.onSetColor(event.currentTarget.value)}
          />
        </label>
        <Show when={props.approximationLabel}>
          {(label) => (
            <output class="wheelColorApproximation" data-color-wheel-approximation aria-live="polite">
              <span>Nearest wheel slot</span>
              <strong>{label()}</strong>
            </output>
          )}
        </Show>
        {props.sideContent}
      </div>
    </div>
  );
}

export function HsvColorAdjustments(props: HsvColorAdjustmentsProps) {
  return (
    <>
      <label class="colorSaturationStrip">
        <span>Saturation</span>
        <input
          type="range"
          min="0"
          max="100"
          step="1"
          value={Math.round(props.hsv.saturation * 100)}
          style={{ background: props.saturationRamp }}
          onInput={(event) => props.onSetHsv({ saturation: clampPercent(Number(event.currentTarget.value)) / 100 })}
        />
        <strong>{Math.round(props.hsv.saturation * 100)}%</strong>
      </label>
      <div class="visualNumberGrid hsvDirectGrid">
        <label>
          Hue
          <input
            type="number"
            min="0"
            max="360"
            step="1"
            value={Math.round(props.hsv.hue)}
            onInput={(event) => props.onSetHsv({ hue: Number(event.currentTarget.value) })}
          />
        </label>
        <label>
          Sat %
          <input
            type="number"
            min="0"
            max="100"
            step="1"
            value={Math.round(props.hsv.saturation * 100)}
            onInput={(event) => props.onSetHsv({ saturation: clampPercent(Number(event.currentTarget.value)) / 100 })}
          />
        </label>
        <label>
          Val %
          <input
            type="number"
            min="0"
            max="100"
            step="1"
            value={Math.round(props.hsv.value * 100)}
            onInput={(event) => props.onSetHsv({ value: clampPercent(Number(event.currentTarget.value)) / 100 })}
          />
        </label>
      </div>
    </>
  );
}
