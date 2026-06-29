import { For, Show } from "solid-js";
import type { EffectKind, VideoLayerSummary, VideoParam } from "../types";

const videoEffectParamOptions: { value: VideoParam; label: string }[] = [
  { value: "Opacity", label: "Opacity" },
  { value: "Speed", label: "Speed" },
  { value: "BpmSyncEnabled", label: "BPM Sync" },
  { value: "BpmSyncRatio", label: "BPM Sync Ratio" },
  { value: "BpmSyncLoopBars", label: "BPM Loop Bars" },
  { value: "TransformX", label: "Transform X" },
  { value: "TransformY", label: "Transform Y" },
  { value: "TransformScaleX", label: "Scale X" },
  { value: "TransformScaleY", label: "Scale Y" },
  { value: "TransformRotationDeg", label: "Rotation" },
  { value: "TransformCropLeft", label: "Crop L" },
  { value: "TransformCropTop", label: "Crop T" },
  { value: "TransformCropRight", label: "Crop R" },
  { value: "TransformCropBottom", label: "Crop B" },
  { value: "ColorBrightness", label: "Brightness" },
  { value: "ColorContrast", label: "Contrast" },
  { value: "ColorHueDeg", label: "Hue" },
  { value: "ColorSaturation", label: "Saturation" },
  { value: "ColorGamma", label: "Gamma" },
  { value: "FxPixelate", label: "Pixelate" },
  { value: "FxBlur", label: "Blur" },
  { value: "FxGlow", label: "Glow" },
  { value: "FxEdge", label: "Edge" },
  { value: "FxKeyRed", label: "Key R" },
  { value: "FxKeyGreen", label: "Key G" },
  { value: "FxKeyBlue", label: "Key B" },
  { value: "FxKeyThreshold", label: "Key Threshold" },
];

interface VideoEffectTargetPanelProps {
  layers: VideoLayerSummary[];
  outputsCount: number;
  selectedLayerId: number | null;
  param: VideoParam;
  low: number;
  high: number;
  effectType: EffectKind;
  positionX: number;
  positionY: number;
  positionZ: number;
  hasSelectedStageObject: boolean;
  onSetLayerId: (layerId: number) => void;
  onSetParam: (param: VideoParam) => void;
  onSetLow: (low: number) => void;
  onSetHigh: (high: number) => void;
  onUseSelectedOutput: () => void;
  onUseWaveOrigin: () => void;
  onUseStageCenter: () => void;
  onUseSelectedStageObject: () => void;
  onSetPositionX: (x: number) => void;
  onSetPositionY: (y: number) => void;
  onSetPositionZ: (z: number) => void;
}

export function VideoEffectTargetPanel(props: VideoEffectTargetPanelProps) {
  return (
    <div class="videoEffectTarget">
      <label>
        Layer
        <select
          value={props.selectedLayerId ?? ""}
          disabled={props.layers.length === 0}
          onInput={(event) => props.onSetLayerId(Number(event.currentTarget.value))}
        >
          <For each={props.layers}>{(layer) => <option value={layer.id}>{layer.label}</option>}</For>
        </select>
      </label>
      <label>
        Param
        <select value={props.param} onInput={(event) => props.onSetParam(event.currentTarget.value as VideoParam)}>
          <For each={videoEffectParamOptions}>{(option) => <option value={option.value}>{option.label}</option>}</For>
        </select>
      </label>
      <div class="split">
        <label>
          Low
          <input type="number" step="0.01" value={props.low} onInput={(event) => props.onSetLow(Number(event.currentTarget.value))} />
        </label>
        <label>
          High
          <input type="number" step="0.01" value={props.high} onInput={(event) => props.onSetHigh(Number(event.currentTarget.value))} />
        </label>
      </div>
      <Show when={props.effectType === "PositionWave"}>
        <div class="videoTargetPositionPanel">
          <div class="waveStageHeader">
            <div>
              <strong>Video Position</strong>
              <span>
                X {props.positionX.toFixed(1)}, Y {props.positionY.toFixed(1)}, Z {props.positionZ.toFixed(1)}
              </span>
            </div>
          </div>
          <div class="buttonRow">
            <button onClick={props.onUseSelectedOutput} disabled={props.outputsCount === 0}>
              Output Surface
            </button>
            <button onClick={props.onUseWaveOrigin}>Wave Origin</button>
            <button onClick={props.onUseStageCenter}>Stage Center</button>
            <button onClick={props.onUseSelectedStageObject} disabled={!props.hasSelectedStageObject}>
              Stage Object
            </button>
          </div>
          <div class="triple">
            <label>
              X
              <input type="number" step="0.1" value={props.positionX} onInput={(event) => props.onSetPositionX(Number(event.currentTarget.value))} />
            </label>
            <label>
              Y
              <input type="number" step="0.1" value={props.positionY} onInput={(event) => props.onSetPositionY(Number(event.currentTarget.value))} />
            </label>
            <label>
              Z
              <input type="number" step="0.1" value={props.positionZ} onInput={(event) => props.onSetPositionZ(Number(event.currentTarget.value))} />
            </label>
          </div>
        </div>
      </Show>
    </div>
  );
}
