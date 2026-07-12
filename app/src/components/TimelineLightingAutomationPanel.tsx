import { createEffect, For } from "solid-js";
import type { TimelineAutomationDraft } from "../editorDrafts";
import type { AttributeControl, AutomationInterpolation, TimelineAutomationSummary } from "../types";

interface FixtureOption {
  id: number;
  label: string;
}

export interface TimelineLightingAutomationRow extends TimelineAutomationSummary {
  fixture_label: string;
}

interface TimelineLightingAutomationPanelProps {
  activeControls: AttributeControl[];
  selectedAttribute: string;
  canAddAutomation: boolean;
  selectedGroupId: string | null;
  canAddGroupAutomation: boolean;
  rowScope: "all" | "current";
  allRowsCount: number;
  startMs: number;
  endMs: number;
  startValue: number;
  endValue: number;
  interpolation: AutomationInterpolation;
  rows: TimelineLightingAutomationRow[];
  selectedAutomationId: number | null;
  fixtureOptions: FixtureOption[];
  timelineAutomationDraft: (automation: TimelineAutomationSummary) => TimelineAutomationDraft;
  fixtureAttributeOptions: (fixtureId: number) => string[];
  onAttribute: (attribute: string) => void;
  onStartMs: (value: number) => void;
  onEndMs: (value: number) => void;
  onStartValue: (value: number) => void;
  onEndValue: (value: number) => void;
  onInterpolation: (interpolation: AutomationInterpolation) => void;
  onRowScope: (scope: "all" | "current") => void;
  onUsePlayheadRange: () => void;
  onAddAutomation: () => void | Promise<void>;
  onAddGroupAutomation: () => void | Promise<void>;
  onUpdateDraft: (automation: TimelineAutomationSummary, patch: Partial<TimelineAutomationDraft>) => void;
  onAlignDraftToPlayhead: (automation: TimelineAutomationSummary) => void;
  onAddKeyframeAtPlayhead: (automation: TimelineAutomationSummary) => void | Promise<void>;
  onRemoveKeyframeAtPlayhead: (automation: TimelineAutomationSummary) => void | Promise<void>;
  onRemoveKeyframe: (automation: TimelineAutomationSummary, keyframeIndex: number) => void | Promise<void>;
  onSetKeyframeInterpolation: (
    automation: TimelineAutomationSummary,
    keyframeIndex: number,
    interpolation: AutomationInterpolation,
  ) => void | Promise<void>;
  onSetKeyframeValue: (
    automation: TimelineAutomationSummary,
    keyframeIndex: number,
    value: number,
  ) => void | Promise<void>;
  onSetAutomationEnabled: (automation: TimelineAutomationSummary, enabled: boolean) => void | Promise<void>;
  onSetRowsEnabled: (automations: TimelineAutomationSummary[], enabled: boolean) => void | Promise<void>;
  onSeekKeyframe: (timeMs: number) => void | Promise<void>;
  onSaveAutomation: (automation: TimelineAutomationSummary) => void | Promise<void>;
  onRemoveAutomation: (automationId: number) => void | Promise<void>;
}

const interpolationOptions: AutomationInterpolation[] = ["Linear", "Step", "Bezier"];
const formatLightingKeyValue = (value: number) => Math.round(value).toString();

export function TimelineLightingAutomationPanel(props: TimelineLightingAutomationPanelProps) {
  let automationListElement: HTMLDivElement | undefined;
  const enabledRowCount = () => props.rows.filter((automation) => automation.enabled).length;
  const disabledRowCount = () => props.rows.length - enabledRowCount();

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
    <>
      <div class="automationForm">
        <label>
          Automation
          <select
            value={props.selectedAttribute}
            disabled={!props.canAddAutomation}
            onInput={(event) => props.onAttribute(event.currentTarget.value)}
          >
            <For each={props.activeControls}>{(control) => <option value={control.attribute}>{control.attribute}</option>}</For>
          </select>
        </label>
        <label>
          Start
          <input type="number" min="0" value={props.startMs} onInput={(event) => props.onStartMs(Number(event.currentTarget.value))} />
        </label>
        <label>
          End
          <input type="number" min="0" value={props.endMs} onInput={(event) => props.onEndMs(Number(event.currentTarget.value))} />
        </label>
        <label>
          From
          <input
            type="number"
            min="0"
            max="65535"
            value={props.startValue}
            onInput={(event) => props.onStartValue(Number(event.currentTarget.value))}
          />
        </label>
        <label>
          To
          <input
            type="number"
            min="0"
            max="65535"
            value={props.endValue}
            onInput={(event) => props.onEndValue(Number(event.currentTarget.value))}
          />
        </label>
        <label>
          Curve
          <select value={props.interpolation} onInput={(event) => props.onInterpolation(event.currentTarget.value as AutomationInterpolation)}>
            <option value="Linear">Linear</option>
            <option value="Step">Step</option>
            <option value="Bezier">Bezier</option>
          </select>
        </label>
        <div class="buttonRow timelineAutomationActions">
          <button onClick={props.onUsePlayheadRange}>Playhead</button>
          <button class="primary" onClick={() => void props.onAddAutomation()} disabled={!props.canAddAutomation}>
            Add
          </button>
          <button
            title={props.selectedGroupId ? `Add to group ${props.selectedGroupId}` : "Select a group before adding group automation"}
            onClick={() => void props.onAddGroupAutomation()}
            disabled={!props.canAddGroupAutomation}
          >
            Group
          </button>
        </div>
      </div>
      <div class="timelineAutomationBatch">
        <span>
          {enabledRowCount()}/{props.rows.length} on, {props.rows.length}/{props.allRowsCount} shown
        </span>
        <select
          class="timelineAutomationScope"
          aria-label="Lighting automation rows"
          value={props.rowScope}
          onInput={(event) => props.onRowScope(event.currentTarget.value as "all" | "current")}
        >
          <option value="all">All</option>
          <option value="current">Current</option>
        </select>
        <div class="buttonRow timelineAutomationBatchActions">
          <button
            title="Enable every lighting automation row"
            disabled={props.rows.length === 0 || disabledRowCount() === 0}
            onClick={() => void props.onSetRowsEnabled(props.rows, true)}
          >
            All On
          </button>
          <button
            title="Disable every lighting automation row"
            disabled={props.rows.length === 0 || enabledRowCount() === 0}
            onClick={() => void props.onSetRowsEnabled(props.rows, false)}
          >
            Mute
          </button>
        </div>
      </div>
      <div class="timelineList" ref={(element) => { automationListElement = element; }}>
        <For each={props.rows}>
          {(automation) => {
            const draft = () => props.timelineAutomationDraft(automation);
            const attributes = () => props.fixtureAttributeOptions(draft().fixture_id);
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
                    <strong>{automation.fixture_label} / {automation.attribute}</strong>
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
                <div class="timelineKeyframeStrip" aria-label={`${automation.fixture_label} ${automation.attribute} keyframes`}>
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
                          min="0"
                          max="65535"
                          aria-label={`Key ${index() + 1} value`}
                          value={formatLightingKeyValue(keyframe.value)}
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
                    Fixture
                    <select
                      value={draft().fixture_id}
                      onInput={(event) =>
                        props.onUpdateDraft(automation, {
                          fixture_id: Number(event.currentTarget.value),
                        })
                      }
                    >
                      <For each={props.fixtureOptions}>{(fixture) => <option data-no-localize value={fixture.id}>{fixture.label}</option>}</For>
                    </select>
                  </label>
                  <label>
                    Attribute
                    <select
                      value={draft().attribute}
                      onInput={(event) =>
                        props.onUpdateDraft(automation, {
                          attribute: event.currentTarget.value,
                        })
                      }
                    >
                      <For each={attributes()}>{(attribute) => <option value={attribute}>{attribute}</option>}</For>
                    </select>
                  </label>
                  <label>
                    Start
                    <input
                      type="number"
                      min="0"
                      value={draft().start_ms}
                      onInput={(event) =>
                        props.onUpdateDraft(automation, {
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
                        props.onUpdateDraft(automation, {
                          end_ms: Number(event.currentTarget.value),
                        })
                      }
                    />
                  </label>
                  <label>
                    From
                    <input
                      type="number"
                      min="0"
                      max="65535"
                      value={draft().start_value}
                      onInput={(event) =>
                        props.onUpdateDraft(automation, {
                          start_value: Number(event.currentTarget.value),
                        })
                      }
                    />
                  </label>
                  <label>
                    To
                    <input
                      type="number"
                      min="0"
                      max="65535"
                      value={draft().end_value}
                      onInput={(event) =>
                        props.onUpdateDraft(automation, {
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
                        props.onUpdateDraft(automation, {
                          interpolation: event.currentTarget.value as AutomationInterpolation,
                        })
                      }
                    >
                      <option value="Linear">Linear</option>
                      <option value="Step">Step</option>
                      <option value="Bezier">Bezier</option>
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
    </>
  );
}
