import { For } from "solid-js";
import type {
  AutomationInterpolation,
  TimelineVideoAutomationSummary,
  VideoLayerSummary,
  VideoParam,
} from "../types";
import type { TimelineVideoAutomationDraft } from "../editorDrafts";

export type VideoTimelineAutomationRow = TimelineVideoAutomationSummary & {
  layer_label: string;
};

const videoParamOptions: { value: VideoParam; label: string }[] = [
  { value: "Opacity", label: "Opacity" },
  { value: "Speed", label: "Speed" },
  { value: "PositionMs", label: "Position" },
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

const interpolationOptions: AutomationInterpolation[] = ["Linear", "Step", "Bezier"];

interface VideoTimelineAutomationPanelProps {
  layers: VideoLayerSummary[];
  selectedLayerId: number | null;
  param: VideoParam;
  interpolation: AutomationInterpolation;
  startMs: number;
  endMs: number;
  startValue: number;
  endValue: number;
  automations: VideoTimelineAutomationRow[];
  draftForAutomation: (automation: VideoTimelineAutomationRow) => TimelineVideoAutomationDraft;
  onSetLayerId: (layerId: number) => void;
  onSetParam: (param: VideoParam) => void;
  onSetInterpolation: (interpolation: AutomationInterpolation) => void;
  onSetStartMs: (startMs: number) => void;
  onSetEndMs: (endMs: number) => void;
  onSetStartValue: (startValue: number) => void;
  onSetEndValue: (endValue: number) => void;
  onAddAutomation: () => void | Promise<void>;
  onUpdateAutomationDraft: (
    automation: VideoTimelineAutomationRow,
    patch: Partial<TimelineVideoAutomationDraft>,
  ) => void;
  onSaveAutomation: (automation: VideoTimelineAutomationRow) => void | Promise<void>;
  onRemoveAutomation: (automationId: number) => void | Promise<void>;
}

export function VideoTimelineAutomationPanel(props: VideoTimelineAutomationPanelProps) {
  return (
    <div class="videoAutomationForm">
      <h3>Timeline Automation</h3>
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
      <div class="split">
        <label>
          Param
          <select value={props.param} onInput={(event) => props.onSetParam(event.currentTarget.value as VideoParam)}>
            <For each={videoParamOptions}>{(option) => <option value={option.value}>{option.label}</option>}</For>
          </select>
        </label>
        <label>
          Curve
          <select
            value={props.interpolation}
            onInput={(event) => props.onSetInterpolation(event.currentTarget.value as AutomationInterpolation)}
          >
            <For each={interpolationOptions}>{(option) => <option value={option}>{option}</option>}</For>
          </select>
        </label>
      </div>
      <div class="split">
        <label>
          Start
          <input type="number" min="0" value={props.startMs} onInput={(event) => props.onSetStartMs(Number(event.currentTarget.value))} />
        </label>
        <label>
          End
          <input type="number" min="0" value={props.endMs} onInput={(event) => props.onSetEndMs(Number(event.currentTarget.value))} />
        </label>
      </div>
      <div class="split">
        <label>
          From
          <input
            type="number"
            step="0.01"
            value={props.startValue}
            onInput={(event) => props.onSetStartValue(Number(event.currentTarget.value))}
          />
        </label>
        <label>
          To
          <input
            type="number"
            step="0.01"
            value={props.endValue}
            onInput={(event) => props.onSetEndValue(Number(event.currentTarget.value))}
          />
        </label>
      </div>
      <button class="primary" onClick={() => void props.onAddAutomation()} disabled={props.layers.length === 0}>
        Add Video Automation
      </button>
      <div class="timelineList">
        <For each={props.automations}>
          {(automation) => {
            const draft = () => props.draftForAutomation(automation);
            return (
              <div class="timelineItem timelineAutomationItem">
                <div>
                  <strong>
                    {automation.layer_label} / {automation.param}
                  </strong>
                  <span>
                    {automation.keyframes[0]?.time_ms ?? 0}-
                    {automation.keyframes[automation.keyframes.length - 1]?.time_ms ?? 0} ms / {automation.keyframes.length} keys
                  </span>
                </div>
                <div class="automationEditGrid">
                  <label>
                    Layer
                    <select
                      value={draft().layer_id}
                      onInput={(event) =>
                        props.onUpdateAutomationDraft(automation, {
                          layer_id: Number(event.currentTarget.value),
                        })
                      }
                    >
                      <For each={props.layers}>{(layer) => <option value={layer.id}>{layer.label}</option>}</For>
                    </select>
                  </label>
                  <label>
                    Param
                    <select
                      value={draft().param}
                      onInput={(event) =>
                        props.onUpdateAutomationDraft(automation, {
                          param: event.currentTarget.value as VideoParam,
                        })
                      }
                    >
                      <For each={videoParamOptions}>{(option) => <option value={option.value}>{option.label}</option>}</For>
                    </select>
                  </label>
                  <label>
                    Start
                    <input
                      type="number"
                      min="0"
                      value={draft().start_ms}
                      onInput={(event) =>
                        props.onUpdateAutomationDraft(automation, {
                          start_ms: Number(event.currentTarget.value),
                        })
                      }
                    />
                  </label>
                  <label>
                    End
                    <input
                      type="number"
                      min="0"
                      value={draft().end_ms}
                      onInput={(event) =>
                        props.onUpdateAutomationDraft(automation, {
                          end_ms: Number(event.currentTarget.value),
                        })
                      }
                    />
                  </label>
                  <label>
                    From
                    <input
                      type="number"
                      step="0.01"
                      value={draft().start_value}
                      onInput={(event) =>
                        props.onUpdateAutomationDraft(automation, {
                          start_value: Number(event.currentTarget.value),
                        })
                      }
                    />
                  </label>
                  <label>
                    To
                    <input
                      type="number"
                      step="0.01"
                      value={draft().end_value}
                      onInput={(event) =>
                        props.onUpdateAutomationDraft(automation, {
                          end_value: Number(event.currentTarget.value),
                        })
                      }
                    />
                  </label>
                  <label>
                    Curve
                    <select
                      value={draft().interpolation}
                      onInput={(event) =>
                        props.onUpdateAutomationDraft(automation, {
                          interpolation: event.currentTarget.value as AutomationInterpolation,
                        })
                      }
                    >
                      <For each={interpolationOptions}>{(option) => <option value={option}>{option}</option>}</For>
                    </select>
                  </label>
                </div>
                <div class="buttonRow">
                  <button onClick={() => void props.onSaveAutomation(automation)}>Save</button>
                  <button onClick={() => void props.onRemoveAutomation(automation.id)}>Remove</button>
                </div>
              </div>
            );
          }}
        </For>
      </div>
    </div>
  );
}
