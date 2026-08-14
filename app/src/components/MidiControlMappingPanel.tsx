import { For, Show } from "solid-js";
import {
  fixtureFlagClearKinds,
  isGroupFlagMappingAction,
  isVideoLayerMappingAction,
  isVideoOutputMappingAction,
  videoOutputMappingFieldOptions,
  type FixtureFlagClearKind,
} from "../controlMappingActions";
import type {
  EngineSnapshot,
  MidiControlAction,
  MidiControlFeedback,
  MidiControlMapping,
  MidiControlMessage,
  MidiFeedbackMessage,
  MidiInputSummary,
  MidiOutputSummary,
  VideoParam,
} from "../types";

interface MidiControlMappingPanelProps {
  snapshot: EngineSnapshot;
  midiInputs: MidiInputSummary[];
  selectedMidiInput: number | null;
  clockConnected: boolean;
  midiOutputs: MidiOutputSummary[];
  selectedMidiOutput: number | null;
  feedbackConnected: boolean;
  feedbackEnabled: boolean;
  midiInputsCount: number;
  controlConnected: boolean;
  mappings: MidiControlMapping[];
  mapMessage: MidiControlMessage;
  mapNumber: number;
  mapChannel: number;
  mapAction: MidiControlAction;
  mapAttribute: string;
  clearFixtureFlagKind: FixtureFlagClearKind;
  selectedCueId: number | null;
  selectedEffectId: number | null;
  selectedNodeGraphId: number | null;
  mapGroupId: string;
  selectedLayerId: number | null;
  selectedOutputId: number | null;
  selectedMappingField: string;
  selectedMappingPresetLabel: string;
  mapVideoParam: VideoParam;
  mapCuePointIndex: number;
  mapDurationMs: number;
  mapLow: number;
  mapHigh: number;
  mappingTargetLabel: (mapping: MidiControlMapping) => string;
  onRefreshMidi: () => void | Promise<void>;
  onSelectedMidiInput: (index: number) => void;
  onConnectClock: () => void | Promise<void>;
  onDisconnectClock: () => void | Promise<void>;
  onSelectedMidiOutput: (index: number) => void;
  onConnectFeedback: () => void | Promise<void>;
  onDisconnectFeedback: () => void | Promise<void>;
  onSendFeedback: () => void | Promise<void>;
  onFeedbackEnabled: (enabled: boolean) => void;
  onMapMessage: (message: MidiControlMessage) => void;
  onMapNumber: (value: number) => void;
  onMapChannel: (value: number) => void;
  onMapAction: (action: MidiControlAction) => void;
  onMapAttribute: (value: string) => void;
  onMapCueId: (id: number) => void;
  onMapEffectId: (id: number) => void;
  onMapNodeGraphId: (id: number) => void;
  onMapGroupId: (value: string) => void;
  onMapLayerId: (id: number) => void;
  onMapOutputId: (id: number) => void;
  onMappingField: (field: string) => void;
  onMappingPresetLabel: (label: string) => void;
  onMapVideoParam: (param: VideoParam) => void;
  onMapCuePointIndex: (value: number) => void;
  onMapDurationMs: (value: number) => void;
  onMapLow: (value: number) => void;
  onMapHigh: (value: number) => void;
  onLearn: () => void | Promise<void>;
  onAddMapping: () => void;
  onConnectControl: () => void | Promise<void>;
  onDisconnectControl: () => void | Promise<void>;
  onLoadMappings: () => void | Promise<void>;
  onSaveMappings: () => void | Promise<void>;
  onRemoveMapping: (index: number) => void;
  onUpdateMapping: (index: number, mapping: MidiControlMapping) => void;
}

type MidiFeedbackState = keyof MidiControlFeedback;

const feedbackStateLabels: Record<MidiFeedbackState, string> = {
  off: "Off",
  on: "On",
  unknown: "Unknown / mixed",
};

const defaultFeedbackMessage = (mapping: MidiControlMapping, value: number): MidiFeedbackMessage => ({
  message: mapping.message,
  channel: mapping.channel ?? 0,
  number: mapping.number,
  value,
});

const feedbackMessageLabel = (message: MidiFeedbackMessage) => (
  `${message.message === "ControlChange" ? "CC" : message.message === "NoteOn" ? "Note On" : message.message === "NoteOff" ? "Note Off" : "Program"} · Ch ${message.channel + 1} · #${message.number} · ${message.value}`
);

export function MidiControlMappingPanel(props: MidiControlMappingPanelProps) {
  return (
    <div class="midiClock ioOperatorSurface">
      <section class="ioConnectionDesk" data-io-default-surface="midi">
        <header class="ioDeskHeader">
          <div>
            <h2>MIDI Connections</h2>
            <span>Clock, MTC, and mapped control</span>
          </div>
          <div class="ioConnectionStates">
            <span class={`ioConnectionState ${props.clockConnected ? "ok" : "idle"}`}><i aria-hidden="true" />Clock {props.clockConnected ? "Connected" : "Stopped"}</span>
            <span class={`ioConnectionState ${props.controlConnected ? "ok" : "idle"}`}><i aria-hidden="true" />Control {props.controlConnected ? "Connected" : "Stopped"}</span>
          </div>
        </header>
        <div class="ioConnectionControls">
          <label>
            MIDI input
            <select
              data-io-control="midi-input"
              value={props.selectedMidiInput ?? ""}
              onInput={(event) => props.onSelectedMidiInput(Number(event.currentTarget.value))}
            >
              <option value="">Select input</option>
              <For each={props.midiInputs}>{(input) => <option data-no-localize value={input.index}>{input.name}</option>}</For>
            </select>
          </label>
          <button data-io-control="midi-refresh" onClick={() => void props.onRefreshMidi()}>Scan MIDI</button>
          <Show when={props.clockConnected} fallback={
            <button data-io-control="midi-clock-connect" class="primary" disabled={props.midiInputs.length === 0} onClick={() => void props.onConnectClock()}>Connect MIDI Clock / MTC</button>
          }>
            <button data-io-control="midi-clock-disconnect" onClick={() => void props.onDisconnectClock()}>Disconnect MIDI Clock / MTC</button>
          </Show>
          <Show when={props.controlConnected} fallback={
            <button
              data-io-control="midi-control-connect"
              class="primary"
              onClick={() => void props.onConnectControl()}
              disabled={props.midiInputs.length === 0 || props.mappings.length === 0}
            >Connect MIDI Control</button>
          }>
            <button data-io-control="midi-control-disconnect" onClick={() => void props.onDisconnectControl()}>Disconnect Control</button>
          </Show>
        </div>
      </section>

      <div class="ioDisclosureStack">
        <details class="ioDisclosure" data-io-disclosure="midi-feedback">
          <summary>MIDI feedback</summary>
          <div class="ioDisclosureBody mappingEditorDesk" data-io-disclosure-body>
            <h3>MIDI Feedback</h3>
            <select data-io-control="midi-feedback-output" value={props.selectedMidiOutput ?? ""} onInput={(event) => props.onSelectedMidiOutput(Number(event.currentTarget.value))}>
              <For each={props.midiOutputs}>{(output) => <option data-no-localize value={output.index}>{output.name}</option>}</For>
            </select>
            <div class="buttonRow">
              <button
                class="primary"
                onClick={() => void props.onConnectFeedback()}
                disabled={props.midiOutputs.length === 0 || props.feedbackConnected}
              >Connect Feedback</button>
              <button onClick={() => void props.onDisconnectFeedback()} disabled={!props.feedbackConnected}>Disconnect Feedback</button>
              <button onClick={() => void props.onSendFeedback()} disabled={!props.feedbackConnected || props.mappings.length === 0}>Send Feedback</button>
            </div>
            <label class="checkbox">
              <input
                type="checkbox"
                checked={props.feedbackEnabled}
                disabled={!props.feedbackConnected}
                onChange={(event) => props.onFeedbackEnabled(event.currentTarget.checked)}
              />
              Auto feedback
            </label>
          </div>
        </details>

        <details class="ioDisclosure" data-io-disclosure="midi-mapping">
          <summary>Control mapping and learn</summary>
          <div class="ioDisclosureBody ioMappingWorkbench" data-io-disclosure-body>
          <div class="mappingEditorDesk">
      <h3>MIDI Control</h3>
      <div class="split">
        <label>
          Message
          <select data-io-control="midi-map-message" value={props.mapMessage} onInput={(event) => props.onMapMessage(event.currentTarget.value as MidiControlMessage)}>
            <option value="ControlChange">CC</option>
            <option value="NoteOn">Note On</option>
            <option value="NoteOff">Note Off</option>
            <option value="ProgramChange">Program</option>
          </select>
        </label>
        <label>
          Number
          <input type="number" min="0" max="127" value={props.mapNumber} onInput={(event) => props.onMapNumber(Number(event.currentTarget.value))} />
        </label>
      </div>
      <div class="split">
        <label>
          Channel
          <input type="number" min="-1" max="15" value={props.mapChannel} onInput={(event) => props.onMapChannel(Number(event.currentTarget.value))} />
        </label>
        <label>
          Action
          <select value={props.mapAction} onInput={(event) => {
            const action = event.currentTarget.value as MidiControlAction;
            props.onMapAction(action);
            if ((action === "TriggerCueDirection" || action === "FlashCueDirection") && !["Forward", "Reverse", "Bounce"].includes(props.mapAttribute)) {
              props.onMapAttribute("Forward");
            }
          }}>
            <option value="FixtureAttribute">Fixture Attribute</option>
            <option value="SelectedFeatureFader">Selected Feature Fader</option>
            <option value="FixtureHighlight">Fixture Highlight</option>
            <option value="FixtureSolo">Fixture Solo</option>
            <option value="FixturePark">Fixture Park</option>
            <option value="GroupHighlight">Group Highlight</option>
            <option value="GroupSolo">Group Solo</option>
            <option value="GroupPark">Group Park</option>
            <option value="TriggerCue">Trigger Cue</option>
            <option value="FlashCue">Flash Cue (hold)</option>
            <option value="TriggerCueDirection">Directional Cue</option>
            <option value="FlashCueDirection">Directional Cue (hold)</option>
            <option value="TriggerCueListNext">Cue List Next</option>
            <option value="TriggerNextCue">Cue Next</option>
            <option value="TriggerPreviousCue">Cue Previous</option>
            <option value="EffectEnabled">Effect Enable</option>
            <option value="NodeGraphEnabled">Node Graph Enable</option>
            <option value="VideoParam">Video Param</option>
            <option value="VideoCuePointAdd">Video Cue Point Add</option>
            <option value="VideoCuePointRemove">Video Cue Point Remove</option>
            <option value="VideoCuePointJump">Video Cue Point Jump</option>
            <option value="VideoCuePointPrevious">Video Cue Point Previous</option>
            <option value="VideoCuePointNext">Video Cue Point Next</option>
            <option value="VideoLayerEnabled">Video Layer Enable</option>
            <option value="VideoLayerSolo">Video Layer Solo</option>
            <option value="VideoPlay">Video Play</option>
            <option value="VideoLoop">Video A-B Loop</option>
            <option value="VideoLayerFade">Video Layer Fade</option>
            <option value="VideoOutputEnabled">Video Output Enable</option>
            <option value="VideoOutputOpacity">Video Output Opacity</option>
            <option value="VideoOutputFade">Video Output Fade</option>
            <option value="VideoOutputMappingField">Video Output Mapping Field</option>
            <option value="VideoOutputMappingPreset">Video Output Mapping Preset</option>
            <option value="VideoOutputBlackout">Video Output Blackout</option>
            <option value="TimelinePlay">Timeline Play</option>
            <option value="TimelineLoopToggle">Timeline Loop Toggle</option>
            <option value="TimelineLoopHalf">Timeline Loop 1/2</option>
            <option value="TimelineLoopDouble">Timeline Loop ×2</option>
            <option value="TimelineSeek">Timeline Seek</option>
            <option value="TimelineBeatPrevious">Beat Previous</option>
            <option value="TimelineBeatNext">Beat Next</option>
            <option value="LightingMaster">Lighting Master</option>
            <option value="VideoMaster">Video Master</option>
            <option value="GroupSubmaster">Group Submaster</option>
            <option value="SetBpm">Set BPM</option>
            <option value="TapBpm">Tap BPM</option>
            <option value="CueFadePause">Cue Fade Pause</option>
            <option value="Blackout">DMX Blackout</option>
            <option value="AllBlackout">All Blackout</option>
            <option value="VideoBlackout">Video Blackout</option>
            <option value="ClearFixtureFlags">Clear Fixture Flags</option>
          </select>
        </label>
      </div>

      <Show when={props.mapAction === "FixtureAttribute"}>
        <label>
          Attribute
          <input value={props.mapAttribute} onInput={(event) => props.onMapAttribute(event.currentTarget.value)} />
        </label>
      </Show>
      <Show when={props.mapAction === "SelectedFeatureFader"}>
        <label>
          Visible fader number
          <input
            type="number"
            min="1"
            max="512"
            value={props.mapCuePointIndex + 1}
            onInput={(event) => props.onMapCuePointIndex(Math.max(0, Number(event.currentTarget.value) - 1))}
          />
        </label>
      </Show>
      <Show when={props.mapAction === "ClearFixtureFlags"}>
        <label>
          Clear flags
          <select value={props.clearFixtureFlagKind} onInput={(event) => props.onMapAttribute(event.currentTarget.value)}>
            <For each={fixtureFlagClearKinds}>{(kind) => <option value={kind}>{kind}</option>}</For>
          </select>
        </label>
      </Show>
      <Show when={props.mapAction === "TriggerCue" || props.mapAction === "FlashCue" || props.mapAction === "TriggerCueDirection" || props.mapAction === "FlashCueDirection" || props.mapAction === "TriggerCueListNext"}>
        <label>
          Cue
          <select value={props.selectedCueId ?? ""} onInput={(event) => props.onMapCueId(Number(event.currentTarget.value))}>
            <For each={props.snapshot.cues}>{(cue) => <option data-no-localize value={cue.id}>{cue.id}: {cue.label}</option>}</For>
          </select>
        </label>
      </Show>
      <Show when={props.mapAction === "TriggerCueDirection" || props.mapAction === "FlashCueDirection"}>
        <label>
          Direction
          <select value={props.mapAttribute} onInput={(event) => props.onMapAttribute(event.currentTarget.value)}>
            <option value="Forward">Forward</option>
            <option value="Reverse">Reverse</option>
            <option value="Bounce">Back &amp; Forth</option>
          </select>
        </label>
      </Show>
      <Show when={props.mapAction === "EffectEnabled"}>
        <label>
          Effect
          <select value={props.selectedEffectId ?? ""} onInput={(event) => props.onMapEffectId(Number(event.currentTarget.value))}>
            <For each={props.snapshot.effects}>{(effect) => <option value={effect.id}>{effect.id}: {effect.label}</option>}</For>
          </select>
        </label>
      </Show>
      <Show when={props.mapAction === "NodeGraphEnabled"}>
        <label>
          Node Graph
          <select value={props.selectedNodeGraphId ?? ""} onInput={(event) => props.onMapNodeGraphId(Number(event.currentTarget.value))}>
            <For each={props.snapshot.node_graphs}>{(graph) => <option value={graph.id}>{graph.id}: {graph.label}</option>}</For>
          </select>
        </label>
      </Show>
      <Show when={props.mapAction === "GroupSubmaster" || isGroupFlagMappingAction(props.mapAction)}>
        <label>
          Group ID
          <input value={props.mapGroupId} onInput={(event) => props.onMapGroupId(event.currentTarget.value)} />
        </label>
      </Show>
      <Show when={isVideoLayerMappingAction(props.mapAction)}>
        <label>
          Layer
          <select value={props.selectedLayerId ?? ""} onInput={(event) => props.onMapLayerId(Number(event.currentTarget.value))}>
            <For each={props.snapshot.video.layers}>{(layer) => <option data-no-localize value={layer.id}>{layer.id}: {layer.label}</option>}</For>
          </select>
        </label>
      </Show>
      <Show when={isVideoOutputMappingAction(props.mapAction)}>
        <label>
          Output
          <select value={props.selectedOutputId ?? ""} onInput={(event) => props.onMapOutputId(Number(event.currentTarget.value))}>
            <For each={props.snapshot.video.outputs}>{(output) => <option data-no-localize value={output.id}>{output.id}: {output.label}</option>}</For>
          </select>
        </label>
      </Show>
      <Show when={props.mapAction === "VideoOutputMappingField"}>
        <label>
          Mapping Field
          <select value={props.selectedMappingField} onInput={(event) => props.onMappingField(event.currentTarget.value)}>
            <For each={videoOutputMappingFieldOptions}>{(field) => <option value={field.value}>{field.label}</option>}</For>
          </select>
        </label>
      </Show>
      <Show when={props.mapAction === "VideoOutputMappingPreset"}>
        <label>
          Mapping Preset
          <select
            value={props.selectedMappingPresetLabel}
            disabled={props.snapshot.video.mapping_presets.length === 0}
            onInput={(event) => props.onMappingPresetLabel(event.currentTarget.value)}
          >
            <Show when={props.snapshot.video.mapping_presets.length === 0}>
              <option value="">No presets</option>
            </Show>
            <For each={props.snapshot.video.mapping_presets}>{(preset) => <option data-no-localize value={preset.label}>{preset.label}</option>}</For>
          </select>
        </label>
      </Show>
      <Show when={props.mapAction === "VideoParam"}>
        <label>
          Video Param
          <select value={props.mapVideoParam} onInput={(event) => props.onMapVideoParam(event.currentTarget.value as VideoParam)}>
            <option value="Opacity">Opacity</option>
            <option value="Speed">Speed</option>
            <option value="PositionMs">Position</option>
            <option value="BpmSyncEnabled">BPM Sync</option>
            <option value="BpmSyncRatio">BPM Sync Ratio</option>
            <option value="BpmSyncLoopBars">BPM Loop Bars</option>
            <option value="TransformX">X</option>
            <option value="TransformY">Y</option>
            <option value="TransformScaleX">Scale X</option>
            <option value="TransformScaleY">Scale Y</option>
            <option value="TransformRotationDeg">Rotation</option>
            <option value="ColorBrightness">Brightness</option>
            <option value="ColorContrast">Contrast</option>
            <option value="ColorHueDeg">Hue</option>
            <option value="ColorSaturation">Saturation</option>
            <option value="ColorGamma">Gamma</option>
            <option value="FxPixelate">Pixelate</option>
            <option value="FxBlur">Blur</option>
            <option value="FxGlow">Glow</option>
            <option value="FxEdge">Edge</option>
            <option value="FxKeyRed">Key R</option>
            <option value="FxKeyGreen">Key G</option>
            <option value="FxKeyBlue">Key B</option>
            <option value="FxKeyThreshold">Key Threshold</option>
          </select>
        </label>
      </Show>
      <Show when={props.mapAction === "VideoCuePointJump"}>
        <label>
          Cue point index
          <input type="number" min="0" value={props.mapCuePointIndex} onInput={(event) => props.onMapCuePointIndex(Number(event.currentTarget.value))} />
        </label>
      </Show>
      <Show
        when={
          props.mapAction === "VideoOutputFade" ||
          props.mapAction === "VideoLayerFade" ||
          props.mapAction === "VideoCuePointAdd" ||
          props.mapAction === "VideoCuePointRemove"
        }
      >
        <label>
          {props.mapAction === "VideoOutputFade" || props.mapAction === "VideoLayerFade" ? "Fade ms" : "Cue point ms"}
          <input type="number" min="0" step="10" value={props.mapDurationMs} onInput={(event) => props.onMapDurationMs(Number(event.currentTarget.value))} />
        </label>
      </Show>
      <div class="split">
        <label>
          {props.mapAction === "VideoLoop"
            ? "Loop In ms"
            : props.mapAction === "SetBpm"
              ? "BPM Low"
              : props.mapAction === "VideoOutputMappingField"
                ? "Field Low"
                : "Low"}
          <input type="number" value={props.mapLow} onInput={(event) => props.onMapLow(Number(event.currentTarget.value))} />
        </label>
        <label>
          {props.mapAction === "VideoLoop"
            ? "Loop Out ms"
            : props.mapAction === "SetBpm"
              ? "BPM High"
              : props.mapAction === "VideoOutputMappingField"
                ? "Field High"
                : "High"}
          <input type="number" value={props.mapHigh} onInput={(event) => props.onMapHigh(Number(event.currentTarget.value))} />
        </label>
      </div>
      <div class="buttonRow">
        <button onClick={() => void props.onLearn()} disabled={props.midiInputsCount === 0}>
          Learn
        </button>
        <button onClick={props.onAddMapping}>Add Mapping</button>
      </div>
      <div class="buttonRow">
        <button onClick={() => void props.onLoadMappings()}>Load Mapping</button>
        <button onClick={() => void props.onSaveMappings()} disabled={props.mappings.length === 0}>
          Save Mapping
        </button>
      </div>
      </div>
      <section class="mappingListDesk">
        <header class="ioDeskHeader">
          <h2>Mappings</h2>
          <span>{props.mappings.length}</span>
        </header>
      <div class="timelineList">
        <For each={props.mappings}>
          {(mapping, index) => (
            <div class="timelineItem">
              <strong>{mapping.message} {mapping.number}</strong>
              <span>{mapping.channel === null || mapping.channel === undefined ? "Any ch" : `Ch ${mapping.channel}`} / {props.mappingTargetLabel(mapping)}</span>
              <Show when={mapping.feedback}>
                {(feedback) => (
                  <span>
                    <span>Feedback:</span>{" "}
                    <span data-no-localize>{[feedback().off, feedback().on, feedback().unknown]
                      .filter((message): message is MidiFeedbackMessage => Boolean(message))
                      .map(feedbackMessageLabel)
                      .join(" / ")}</span>
                  </span>
                )}
              </Show>
              <details class="mappingFeedbackEditor">
                <summary>{mapping.feedback ? "Edit feedback" : "Add feedback"}</summary>
                <div class="mappingFeedbackEditorBody">
                  <Show when={mapping.feedback} fallback={
                    <button
                      onClick={() => props.onUpdateMapping(index(), {
                        ...mapping,
                        feedback: {
                          off: defaultFeedbackMessage(mapping, 0),
                          on: defaultFeedbackMessage(mapping, 127),
                          unknown: null,
                        },
                      })}
                    >Create Off / On feedback</button>
                  }>
                    <For each={(["off", "on", "unknown"] as MidiFeedbackState[])}>
                      {(state) => {
                        const message = () => mapping.feedback?.[state] ?? null;
                        const updateMessage = (patch: Partial<MidiFeedbackMessage>) => {
                          const current = message();
                          if (!current) return;
                          props.onUpdateMapping(index(), {
                            ...mapping,
                            feedback: {
                              ...mapping.feedback,
                              [state]: { ...current, ...patch },
                            },
                          });
                        };
                        return (
                          <fieldset>
                            <legend>{feedbackStateLabels[state]}</legend>
                            <Show when={message()} fallback={
                              <button
                                onClick={() => props.onUpdateMapping(index(), {
                                  ...mapping,
                                  feedback: {
                                    ...mapping.feedback,
                                    [state]: defaultFeedbackMessage(mapping, state === "on" ? 127 : 0),
                                  },
                                })}
                              >Add state</button>
                            }>
                              {(current) => (
                                <>
                                  <label>
                                    Message
                                    <select
                                      value={current().message}
                                      onInput={(event) => updateMessage({ message: event.currentTarget.value as MidiControlMessage })}
                                    >
                                      <option value="ControlChange">CC</option>
                                      <option value="NoteOn">Note On</option>
                                      <option value="NoteOff">Note Off</option>
                                      <option value="ProgramChange">Program</option>
                                    </select>
                                  </label>
                                  <label>
                                    Channel
                                    <input
                                      type="number"
                                      min="1"
                                      max="16"
                                      value={current().channel + 1}
                                      onInput={(event) => updateMessage({ channel: Math.max(0, Math.min(15, Number(event.currentTarget.value) - 1)) })}
                                    />
                                  </label>
                                  <label>
                                    Number
                                    <input
                                      type="number"
                                      min="0"
                                      max="127"
                                      value={current().number}
                                      onInput={(event) => updateMessage({ number: Math.max(0, Math.min(127, Number(event.currentTarget.value))) })}
                                    />
                                  </label>
                                  <label>
                                    Velocity / value
                                    <input
                                      type="number"
                                      min="0"
                                      max="127"
                                      value={current().value}
                                      onInput={(event) => updateMessage({ value: Math.max(0, Math.min(127, Number(event.currentTarget.value))) })}
                                    />
                                  </label>
                                  <button
                                    onClick={() => props.onUpdateMapping(index(), {
                                      ...mapping,
                                      feedback: { ...mapping.feedback, [state]: null },
                                    })}
                                  >Remove state</button>
                                </>
                              )}
                            </Show>
                          </fieldset>
                        );
                      }}
                    </For>
                    <button onClick={() => props.onUpdateMapping(index(), { ...mapping, feedback: null })}>
                      Clear feedback
                    </button>
                  </Show>
                </div>
              </details>
              <button onClick={() => props.onRemoveMapping(index())}>Remove</button>
            </div>
          )}
        </For>
      </div>
      </section>
          </div>
        </details>
      </div>
    </div>
  );
}
