import { createEffect, For } from "solid-js";
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
  selectedAutomationId: number | null;
  rowScope: "all" | "layer";
  allRowsCount: number;
  draftForAutomation: (automation: VideoTimelineAutomationRow) => TimelineVideoAutomationDraft;
  onSetLayerId: (layerId: number) => void;
  onSetParam: (param: VideoParam) => void;
  onSetInterpolation: (interpolation: AutomationInterpolation) => void;
  onSetStartMs: (startMs: number) => void;
  onSetEndMs: (endMs: number) => void;
  onSetStartValue: (startValue: number) => void;
  onSetEndValue: (endValue: number) => void;
  onRowScope: (scope: "all" | "layer") => void;
  onUsePlayheadRange: () => void;
  onAddAutomation: () => void | Promise<void>;
  onUpdateAutomationDraft: (
    automation: VideoTimelineAutomationRow,
    patch: Partial<TimelineVideoAutomationDraft>,
  ) => void;
  onAlignDraftToPlayhead: (automation: VideoTimelineAutomationRow) => void;
  onAddKeyframeAtPlayhead: (automation: VideoTimelineAutomationRow) => void | Promise<void>;
  onRemoveKeyframeAtPlayhead: (automation: VideoTimelineAutomationRow) => void | Promise<void>;
  onRemoveKeyframe: (automation: VideoTimelineAutomationRow, keyframeIndex: number) => void | Promise<void>;
  onSetKeyframeInterpolation: (
    automation: VideoTimelineAutomationRow,
    keyframeIndex: number,
    interpolation: AutomationInterpolation,
  ) => void | Promise<void>;
  onSetKeyframeValue: (
    automation: VideoTimelineAutomationRow,
    keyframeIndex: number,
    value: number,
  ) => void | Promise<void>;
  onSetAutomationEnabled: (automation: VideoTimelineAutomationRow, enabled: boolean) => void | Promise<void>;
  onSetRowsEnabled: (automations: VideoTimelineAutomationRow[], enabled: boolean) => void | Promise<void>;
  onSeekKeyframe: (timeMs: number) => void | Promise<void>;
  onSaveAutomation: (automation: VideoTimelineAutomationRow) => void | Promise<void>;
  onRemoveAutomation: (automationId: number) => void | Promise<void>;
}

const formatVideoKeyValue = (value: number) => {
  if (Math.abs(value) >= 100 || Number.isInteger(value)) {
    return value.toFixed(0);
  }
  return value.toFixed(2);
};

export function VideoTimelineAutomationPanel(props: VideoTimelineAutomationPanelProps) {
  let automationListElement: HTMLDivElement | undefined;
  const enabledRowCount = () => props.automations.filter((automation) => automation.enabled).length;
  const disabledRowCount = () => props.automations.length - enabledRowCount();

  createEffect(() => {
    const selectedAutomationId = props.selectedAutomationId;
    if (selectedAutomationId === null) {
      return;
    }
    automationListElement
      ?.querySelector(`[data-automation-id="${selectedAutomationId}"]`)
      ?.scrollIntoView({ block: "nearest" });
  });

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
      <div class="buttonRow timelineAutomationActions">
        <button onClick={props.onUsePlayheadRange}>Playhead</button>
        <button class="primary" onClick={() => void props.onAddAutomation()} disabled={props.layers.length === 0}>
          Add Video
        </button>
      </div>
      <div class="timelineAutomationBatch">
        <span>
          {enabledRowCount()}/{props.automations.length} on, {props.automations.length}/{props.allRowsCount} shown
        </span>
        <select
          class="timelineAutomationScope"
          aria-label="Video automation rows"
          value={props.rowScope}
          onInput={(event) => props.onRowScope(event.currentTarget.value as "all" | "layer")}
        >
          <option value="all">All</option>
          <option value="layer">Video Layer</option>
        </select>
        <div class="buttonRow timelineAutomationBatchActions">
          <button
            title="Enable every video automation row"
            disabled={props.automations.length === 0 || disabledRowCount() === 0}
            onClick={() => void props.onSetRowsEnabled(props.automations, true)}
          >
            All On
          </button>
          <button
            title="Disable every video automation row"
            disabled={props.automations.length === 0 || enabledRowCount() === 0}
            onClick={() => void props.onSetRowsEnabled(props.automations, false)}
          >
            Mute
          </button>
        </div>
      </div>
      <div class="timelineList" ref={(element) => { automationListElement = element; }}>
        <For each={props.automations}>
          {(automation) => {
            const draft = () => props.draftForAutomation(automation);
            return (
              <div
                class="timelineItem timelineAutomationItem"
                data-automation-id={automation.id}
                classList={{
                  disabled: !automation.enabled,
                  selected: props.selectedAutomationId === automation.id,
                }}
              >
                <div class="timelineAutomationTitle">
                  <div>
                    <strong>
                      {automation.layer_label} / {automation.param}
                    </strong>
                    <span>
                      {automation.keyframes[0]?.time_ms ?? 0}-
                      {automation.keyframes[automation.keyframes.length - 1]?.time_ms ?? 0} ms / {automation.keyframes.length} keys
                    </span>
                  </div>
                  <label class="timelineAutomationEnabled">
                    <input
                      type="checkbox"
                      checked={automation.enabled}
                      onChange={(event) => void props.onSetAutomationEnabled(automation, event.currentTarget.checked)}
                    />
                    Enabled
                  </label>
                </div>
                <div class="timelineKeyframeStrip" aria-label={`${automation.layer_label} ${automation.param} keyframes`}>
                  <For each={automation.keyframes}>
                    {(keyframe, index) => (
                      <div class="timelineKeyframeChip">
                        <button
                          class="timelineKeyframeSeek"
                          title={`Seek to key ${index() + 1}`}
                          onClick={() => void props.onSeekKeyframe(keyframe.time_ms)}
                        >
                          <strong>{keyframe.time_ms}ms</strong>
                        </button>
                        <input
                          class="timelineKeyframeValue"
                          type="number"
                          step="0.01"
                          aria-label={`Key ${index() + 1} value`}
                          value={formatVideoKeyValue(keyframe.value)}
                          onChange={(event) => void props.onSetKeyframeValue(automation, index(), Number(event.currentTarget.value))}
                        />
                        <select
                          class="timelineKeyframeCurve"
                          aria-label={`Key ${index() + 1} interpolation`}
                          value={keyframe.interpolation}
                          onInput={(event) =>
                            void props.onSetKeyframeInterpolation(
                              automation,
                              index(),
                              event.currentTarget.value as AutomationInterpolation,
                            )
                          }
                        >
                          <For each={interpolationOptions}>{(option) => <option value={option}>{option}</option>}</For>
                        </select>
                        <button
                          class="timelineKeyframeDelete"
                          type="button"
                          aria-label={`Remove key ${index() + 1}`}
                          title={`Remove key ${index() + 1}`}
                          disabled={automation.keyframes.length <= 2}
                          onClick={() => void props.onRemoveKeyframe(automation, index())}
                        >
                          x
                        </button>
                      </div>
                    )}
                  </For>
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
                  <button title="Add a keyframe at the current playhead" onClick={() => void props.onAddKeyframeAtPlayhead(automation)}>
                    Key
                  </button>
                  <button title="Remove the keyframe nearest to the current playhead" onClick={() => void props.onRemoveKeyframeAtPlayhead(automation)}>
                    Del Key
                  </button>
                  <button title="Move draft start to the current playhead" onClick={() => props.onAlignDraftToPlayhead(automation)}>
                    Align
                  </button>
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
