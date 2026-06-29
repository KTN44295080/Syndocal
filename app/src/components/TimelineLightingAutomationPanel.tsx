import { For } from "solid-js";
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
  startMs: number;
  endMs: number;
  startValue: number;
  endValue: number;
  interpolation: AutomationInterpolation;
  rows: TimelineLightingAutomationRow[];
  fixtureOptions: FixtureOption[];
  timelineAutomationDraft: (automation: TimelineAutomationSummary) => TimelineAutomationDraft;
  fixtureAttributeOptions: (fixtureId: number) => string[];
  onAttribute: (attribute: string) => void;
  onStartMs: (value: number) => void;
  onEndMs: (value: number) => void;
  onStartValue: (value: number) => void;
  onEndValue: (value: number) => void;
  onInterpolation: (interpolation: AutomationInterpolation) => void;
  onAddAutomation: () => void | Promise<void>;
  onUpdateDraft: (automation: TimelineAutomationSummary, patch: Partial<TimelineAutomationDraft>) => void;
  onSaveAutomation: (automation: TimelineAutomationSummary) => void | Promise<void>;
  onRemoveAutomation: (automationId: number) => void | Promise<void>;
}

export function TimelineLightingAutomationPanel(props: TimelineLightingAutomationPanelProps) {
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
        <button class="primary" onClick={() => void props.onAddAutomation()} disabled={!props.canAddAutomation}>
          Add Automation
        </button>
      </div>
      <div class="timelineList">
        <For each={props.rows}>
          {(automation) => {
            const draft = () => props.timelineAutomationDraft(automation);
            const attributes = () => props.fixtureAttributeOptions(draft().fixture_id);
            return (
              <div class="timelineItem timelineAutomationItem">
                <div>
                  <strong>{automation.fixture_label} / {automation.attribute}</strong>
                  <span>
                    {automation.keyframes[0]?.time_ms ?? 0}-
                    {automation.keyframes[automation.keyframes.length - 1]?.time_ms ?? 0} ms / {automation.keyframes.length} keys
                  </span>
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
                      <For each={props.fixtureOptions}>{(fixture) => <option value={fixture.id}>{fixture.label}</option>}</For>
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
