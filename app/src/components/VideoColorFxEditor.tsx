import type { VideoColorAdjust, VideoFxAdjust } from "../types";

const colorPadWidth = 100;
const colorPadHeight = 62;

const finiteOr = (value: number, fallback: number) => (Number.isFinite(value) ? value : fallback);
const clampRange = (value: number, min: number, max: number) => Math.min(max, Math.max(min, value));
const roundedRangeValue = (value: number, min: number, max: number, digits = 3) =>
  Number(clampRange(value, min, max).toFixed(digits));
const wrapHue = (value: number) => ((value % 360) + 360) % 360;

const colorPointerPoint = (event: PointerEvent, element: HTMLElement | SVGElement | null) => {
  if (!element) {
    return null;
  }
  const rect = element.getBoundingClientRect();
  if (rect.width <= 0 || rect.height <= 0) {
    return null;
  }
  return {
    x: clampRange((event.clientX - rect.left) / rect.width, 0, 1),
    y: clampRange((event.clientY - rect.top) / rect.height, 0, 1),
  };
};

const hueStep = (event: KeyboardEvent) => {
  if (event.shiftKey) {
    return 10;
  }
  if (event.altKey) {
    return 0.25;
  }
  return 1;
};

const unitStep = (event: KeyboardEvent) => {
  if (event.shiftKey) {
    return 0.1;
  }
  if (event.altKey) {
    return 0.005;
  }
  return 0.02;
};

const rgbComponentToHex = (value: number) =>
  Math.round(clampRange(finiteOr(value, 0), 0, 1) * 255)
    .toString(16)
    .padStart(2, "0");

const keyColorHex = (fx: VideoFxAdjust) =>
  `#${rgbComponentToHex(fx.key_red)}${rgbComponentToHex(fx.key_green)}${rgbComponentToHex(fx.key_blue)}`;

const hexToKeyPatch = (value: string): Pick<VideoFxAdjust, "key_red" | "key_green" | "key_blue"> | null => {
  const normalized = value.trim().toLowerCase();
  if (!/^#[0-9a-f]{6}$/.test(normalized)) {
    return null;
  }
  return {
    key_red: Number((Number.parseInt(normalized.slice(1, 3), 16) / 255).toFixed(3)),
    key_green: Number((Number.parseInt(normalized.slice(3, 5), 16) / 255).toFixed(3)),
    key_blue: Number((Number.parseInt(normalized.slice(5, 7), 16) / 255).toFixed(3)),
  };
};

interface VideoColorFxEditorProps {
  label: string;
  color: VideoColorAdjust;
  fx: VideoFxAdjust;
  onColorPatch: (patch: Partial<VideoColorAdjust>) => void;
  onFxPatch: (patch: Partial<VideoFxAdjust>) => void;
}

export function VideoColorFxEditor(props: VideoColorFxEditorProps) {
  const hue = () => wrapHue(finiteOr(props.color.hue_deg, 0));
  const brightness = () => clampRange(finiteOr(props.color.brightness, 0), -1, 1);
  const saturation = () => clampRange(finiteOr(props.color.saturation, 1), 0, 4);
  const contrast = () => clampRange(finiteOr(props.color.contrast, 1), 0, 4);
  const gamma = () => clampRange(finiteOr(props.color.gamma, 1), 0.1, 4);
  const threshold = () => clampRange(finiteOr(props.fx.key_threshold, 0), 0, 1);

  const setHueBrightnessFromPointer = (event: PointerEvent & { currentTarget: HTMLDivElement }) => {
    event.preventDefault();
    const point = colorPointerPoint(event, event.currentTarget);
    if (!point) {
      return;
    }
    props.onColorPatch({
      hue_deg: Number((point.x * 360).toFixed(1)),
      brightness: roundedRangeValue(1 - point.y * 2, -1, 1),
    });
  };

  const nudgeHueBrightness = (event: KeyboardEvent) => {
    if (event.key === "Home") {
      event.preventDefault();
      props.onColorPatch({ hue_deg: 0, brightness: 0 });
      return;
    }
    if (event.key === "ArrowLeft" || event.key === "ArrowRight") {
      event.preventDefault();
      props.onColorPatch({ hue_deg: Number(wrapHue(hue() + (event.key === "ArrowRight" ? hueStep(event) : -hueStep(event))).toFixed(1)) });
      return;
    }
    if (event.key === "ArrowUp" || event.key === "ArrowDown") {
      event.preventDefault();
      props.onColorPatch({
        brightness: roundedRangeValue(brightness() + (event.key === "ArrowUp" ? unitStep(event) : -unitStep(event)), -1, 1),
      });
    }
  };

  const setKeyColor = (value: string) => {
    const patch = hexToKeyPatch(value);
    if (patch) {
      props.onFxPatch(patch);
    }
  };

  return (
    <div class="videoColorFxEditor">
      <div class="videoColorFxPadRow">
        <div
          class="videoHueBrightnessPad"
          role="slider"
          aria-label={`${props.label} hue and brightness`}
          aria-valuetext={`Hue ${Math.round(hue())} degrees, brightness ${brightness().toFixed(2)}`}
          aria-valuemin="0"
          aria-valuemax="360"
          aria-valuenow={Math.round(hue())}
          tabIndex={0}
          onPointerDown={(event) => {
            event.currentTarget.setPointerCapture(event.pointerId);
            setHueBrightnessFromPointer(event);
          }}
          onPointerMove={(event) => {
            if (event.buttons === 1) {
              setHueBrightnessFromPointer(event);
            }
          }}
          onPointerUp={(event) => event.currentTarget.releasePointerCapture(event.pointerId)}
          onPointerCancel={(event) => event.currentTarget.releasePointerCapture(event.pointerId)}
          onKeyDown={nudgeHueBrightness}
        >
          <i
            style={{
              left: `${(hue() / 360) * 100}%`,
              top: `${((1 - brightness()) / 2) * 100}%`,
            }}
          />
        </div>
        <div class="videoKeyColorPanel">
          <label>
            Key
            <input type="color" value={keyColorHex(props.fx)} onInput={(event) => setKeyColor(event.currentTarget.value)} />
          </label>
          <label>
            Threshold
            <input
              type="range"
              min="0"
              max="1"
              step="0.01"
              value={threshold()}
              onInput={(event) => props.onFxPatch({ key_threshold: roundedRangeValue(Number(event.currentTarget.value), 0, 1) })}
            />
            <span>{threshold().toFixed(2)}</span>
          </label>
        </div>
      </div>
      <div class="videoColorFxSliderGrid">
        <label>
          Saturation
          <input
            type="range"
            min="0"
            max="4"
            step="0.01"
            value={saturation()}
            onInput={(event) => props.onColorPatch({ saturation: roundedRangeValue(Number(event.currentTarget.value), 0, 4) })}
          />
          <span>{saturation().toFixed(2)}</span>
        </label>
        <label>
          Contrast
          <input
            type="range"
            min="0"
            max="4"
            step="0.01"
            value={contrast()}
            onInput={(event) => props.onColorPatch({ contrast: roundedRangeValue(Number(event.currentTarget.value), 0, 4) })}
          />
          <span>{contrast().toFixed(2)}</span>
        </label>
        <label>
          Gamma
          <input
            type="range"
            min="0.1"
            max="4"
            step="0.01"
            value={gamma()}
            onInput={(event) => props.onColorPatch({ gamma: roundedRangeValue(Number(event.currentTarget.value), 0.1, 4) })}
          />
          <span>{gamma().toFixed(2)}</span>
        </label>
      </div>
      <div class="videoColorFxPresetRow">
        <button onClick={() => props.onColorPatch({ brightness: 0, contrast: 1, hue_deg: 0, saturation: 1, gamma: 1 })}>
          Neutral
        </button>
        <button onClick={() => props.onColorPatch({ hue_deg: 180 })}>Invert Hue</button>
        <button onClick={() => props.onFxPatch({ key_red: 0, key_green: 1, key_blue: 0, key_threshold: 0.1 })}>
          Green Key
        </button>
      </div>
      <svg class="videoColorFxPreview" viewBox={`0 0 ${colorPadWidth} ${colorPadHeight}`} role="img">
        <rect x="0" y="0" width="50" height={colorPadHeight} />
        <rect x="50" y="0" width="50" height={colorPadHeight} />
        <line x1="50" y1="0" x2="50" y2={colorPadHeight} />
      </svg>
    </div>
  );
}
