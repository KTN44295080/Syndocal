import { onCleanup, type Accessor, type Setter } from "solid-js";
import {
  isFixtureFlagMappingAction,
  isGroupFlagMappingAction,
  isVideoLayerMappingAction,
  isVideoOutputMappingAction,
  type FixtureFlagClearKind,
} from "./controlMappingActions";
import type {
  LearnedMidiControl,
  LearnedOscControl,
  MidiControlAction,
  MidiControlMapping,
  MidiControlMessage,
  MidiInputSummary,
  MidiOutputSummary,
  OscControlAction,
  OscControlMapping,
  OscInputConfig,
  PatchedFixtureSummary,
  VideoParam,
} from "./types";

type Invoke = <T>(command: string, args?: Record<string, unknown>) => Promise<T>;

interface ControlInputControllerOptions {
  invoke: Invoke;
  setMessage: (message: string) => unknown;
  selectedFixture: Accessor<PatchedFixtureSummary | undefined>;
  selectedEffectAttribute: Accessor<string>;
  setMidiInputs: Setter<MidiInputSummary[]>;
  setMidiOutputs: Setter<MidiOutputSummary[]>;
  selectedMidiInput: Accessor<number | null>;
  setSelectedMidiInput: Setter<number | null>;
  selectedMidiOutput: Accessor<number | null>;
  setSelectedMidiOutput: Setter<number | null>;
  setMidiConnected: Setter<boolean>;
  setMidiControlConnected: Setter<boolean>;
  midiFeedbackConnected: Accessor<boolean>;
  setMidiFeedbackConnected: Setter<boolean>;
  midiFeedbackEnabled: Accessor<boolean>;
  setMidiFeedbackEnabled: Setter<boolean>;
  midiMappings: Accessor<MidiControlMapping[]>;
  setMidiMappings: Setter<MidiControlMapping[]>;
  midiMapMessage: Accessor<MidiControlMessage>;
  setMidiMapMessage: Setter<MidiControlMessage>;
  midiMapChannel: Accessor<number>;
  setMidiMapChannel: Setter<number>;
  midiMapNumber: Accessor<number>;
  setMidiMapNumber: Setter<number>;
  midiMapAction: Accessor<MidiControlAction>;
  midiMapAttribute: Accessor<string>;
  midiMapGroupId: Accessor<string>;
  midiClearFixtureFlagKind: Accessor<FixtureFlagClearKind>;
  selectedMidiCueId: Accessor<number | null>;
  selectedMidiEffectId: Accessor<number | null>;
  selectedMidiNodeGraphId: Accessor<number | null>;
  selectedMidiLayerId: Accessor<number | null>;
  selectedMidiVideoOutputId: Accessor<number | null>;
  selectedMidiVideoOutputMappingField: Accessor<string>;
  selectedMidiVideoOutputMappingPresetLabel: Accessor<string>;
  midiMapVideoParam: Accessor<VideoParam>;
  midiMapCuePointIndex: Accessor<number>;
  midiMapDurationMs: Accessor<number>;
  midiMapLow: Accessor<number>;
  midiMapHigh: Accessor<number>;
  oscRunning: Accessor<boolean>;
  setOscRunning: Setter<boolean>;
  oscMappings: Accessor<OscControlMapping[]>;
  setOscMappings: Setter<OscControlMapping[]>;
  oscBindIp: Accessor<string>;
  oscPort: Accessor<number>;
  oscMapAddress: Accessor<string>;
  setOscMapAddress: Setter<string>;
  oscMapAction: Accessor<OscControlAction>;
  oscMapAttribute: Accessor<string>;
  oscMapGroupId: Accessor<string>;
  oscClearFixtureFlagKind: Accessor<FixtureFlagClearKind>;
  selectedOscCueId: Accessor<number | null>;
  selectedOscEffectId: Accessor<number | null>;
  selectedOscNodeGraphId: Accessor<number | null>;
  selectedOscLayerId: Accessor<number | null>;
  selectedOscVideoOutputId: Accessor<number | null>;
  selectedOscVideoOutputMappingField: Accessor<string>;
  selectedOscVideoOutputMappingPresetLabel: Accessor<string>;
  oscMapVideoParam: Accessor<VideoParam>;
  oscMapCuePointIndex: Accessor<number>;
  oscMapDurationMs: Accessor<number>;
  oscMapLow: Accessor<number>;
  oscMapHigh: Accessor<number>;
}

export function createControlInputController(options: ControlInputControllerOptions) {
  const reportMessage = (message: string): void => {
    options.setMessage(message);
  };
  const refreshMidiInputs = async () => {
    try {
      const inputs = await options.invoke<MidiInputSummary[]>("list_midi_inputs");
      options.setMidiInputs(inputs);
      if (options.selectedMidiInput() === null && inputs.length > 0) options.setSelectedMidiInput(inputs[0].index);
    } catch (error) {
      options.setMessage(String(error));
    }
  };

  const refreshMidiOutputs = async () => {
    try {
      const outputs = await options.invoke<MidiOutputSummary[]>("list_midi_outputs");
      options.setMidiOutputs(outputs);
      if (options.selectedMidiOutput() === null && outputs.length > 0) options.setSelectedMidiOutput(outputs[0].index);
    } catch (error) {
      options.setMessage(String(error));
    }
  };

  const connectMidiClock = async () => {
    const inputIndex = options.selectedMidiInput();
    if (inputIndex === null) {
      options.setMessage("No MIDI input selected.");
      return;
    }
    try {
      await options.invoke("connect_midi_clock", { inputIndex });
      options.setMidiConnected(true);
      options.setMessage("MIDI Clock connected.");
    } catch (error) {
      options.setMessage(String(error));
    }
  };

  const disconnectMidiClock = async () => {
    try {
      await options.invoke("disconnect_midi_clock");
      options.setMidiConnected(false);
      options.setMessage("MIDI Clock disconnected.");
    } catch (error) {
      options.setMessage(String(error));
    }
  };

  const addMidiMapping = () => {
    const action = options.midiMapAction();
    const fixture = options.selectedFixture();
    const cueId = options.selectedMidiCueId();
    const effectId = options.selectedMidiEffectId();
    const nodeGraphId = options.selectedMidiNodeGraphId();
    const layerId = options.selectedMidiLayerId();
    const outputId = options.selectedMidiVideoOutputId();
    const outputMappingField = options.selectedMidiVideoOutputMappingField();
    const outputMappingPresetLabel = options.selectedMidiVideoOutputMappingPresetLabel();
    const attribute = options.midiMapAttribute() || options.selectedEffectAttribute();
    if (action === "FixtureAttribute" && (!fixture || !attribute)) return reportMessage("Select a fixture and MIDI attribute target first.");
    if (isFixtureFlagMappingAction(action) && !fixture) return reportMessage("Select a fixture before mapping MIDI to a fixture flag.");
    if (action === "TriggerCue" && cueId === null) return reportMessage("Create a cue before mapping MIDI to cues.");
    if (action === "EffectEnabled" && effectId === null) return reportMessage("Add an effect before mapping MIDI to effect enable.");
    if (action === "NodeGraphEnabled" && nodeGraphId === null) return reportMessage("Add a node graph before mapping MIDI to node graph enable.");
    if ((action === "GroupSubmaster" || isGroupFlagMappingAction(action)) && !options.midiMapGroupId().trim()) {
      return reportMessage("Enter a group ID before mapping MIDI to a group.");
    }
    if (isVideoLayerMappingAction(action) && layerId === null) return reportMessage("Add a video layer before mapping MIDI to video.");
    if (isVideoOutputMappingAction(action) && outputId === null) return reportMessage("Add a video output before mapping MIDI to video output.");
    if (action === "VideoOutputMappingPreset" && !outputMappingPresetLabel) {
      return reportMessage("Save a video output mapping preset before mapping MIDI to a preset.");
    }
    const mapping: MidiControlMapping = {
      channel: options.midiMapChannel() >= 0 ? options.midiMapChannel() : null,
      message: options.midiMapMessage(),
      number: Math.max(0, Math.min(127, Math.round(options.midiMapNumber()))),
      action,
      fixture_id: action === "FixtureAttribute" || isFixtureFlagMappingAction(action) ? fixture?.id ?? null : null,
      attribute: action === "FixtureAttribute" ? attribute
        : action === "ClearFixtureFlags" ? options.midiClearFixtureFlagKind()
        : action === "VideoOutputMappingField" ? outputMappingField
        : action === "VideoOutputMappingPreset" ? outputMappingPresetLabel : null,
      group_id: action === "GroupSubmaster" || isGroupFlagMappingAction(action) ? options.midiMapGroupId().trim() : null,
      cue_id: action === "TriggerCue" ? cueId : action === "EffectEnabled" ? effectId : action === "NodeGraphEnabled" ? nodeGraphId : null,
      layer_id: isVideoLayerMappingAction(action) ? layerId : null,
      output_id: isVideoOutputMappingAction(action) ? outputId : null,
      video_param: action === "VideoParam" ? options.midiMapVideoParam() : null,
      cue_point_index: action === "VideoCuePointJump" ? Math.max(0, Math.round(options.midiMapCuePointIndex())) : null,
      duration_ms: action === "VideoOutputFade" || action === "VideoLayerFade" || action === "VideoCuePointAdd" || action === "VideoCuePointRemove"
        ? Math.max(0, Math.round(options.midiMapDurationMs())) : null,
      low: options.midiMapLow(),
      high: options.midiMapHigh(),
    };
    options.setMidiMappings((current) => [...current, mapping]);
    options.setMessage(`Added MIDI mapping ${mapping.message} ${mapping.number}`);
  };

  const removeMidiMapping = (index: number) => options.setMidiMappings((current) => current.filter((_, candidate) => candidate !== index));

  const applyLearnedMidiControl = (learned: LearnedMidiControl) => {
    options.setMidiMapChannel(learned.channel);
    options.setMidiMapMessage(learned.message);
    options.setMidiMapNumber(learned.number);
    options.setMessage(`Learned ${learned.message} ch ${learned.channel} #${learned.number}`);
  };

  const learnMidiControl = async () => {
    const inputIndex = options.selectedMidiInput();
    if (inputIndex === null) return reportMessage("No MIDI input selected.");
    try {
      options.setMessage("Waiting for MIDI input...");
      const learned = await options.invoke<LearnedMidiControl | null>("learn_midi_control", { inputIndex });
      if (learned) applyLearnedMidiControl(learned);
      else options.setMessage("MIDI learn timed out.");
    } catch (error) {
      options.setMessage(String(error));
    }
  };

  const saveMidiMappings = async () => {
    try {
      const path = await options.invoke<string | null>("save_midi_mappings", { mappings: options.midiMappings() });
      if (path) options.setMessage(`Saved MIDI mappings to ${path}`);
    } catch (error) { options.setMessage(String(error)); }
  };
  const loadMidiMappings = async () => {
    try {
      const loaded = await options.invoke<MidiControlMapping[] | null>("load_midi_mappings");
      if (loaded) {
        options.setMidiMappings(loaded);
        options.setMessage(`Loaded ${loaded.length} MIDI mapping(s).`);
      }
    } catch (error) { options.setMessage(String(error)); }
  };

  const connectMidiControl = async () => {
    const inputIndex = options.selectedMidiInput();
    if (inputIndex === null) return reportMessage("No MIDI input selected.");
    if (options.midiMappings().length === 0) return reportMessage("Add at least one MIDI mapping first.");
    try {
      await options.invoke("connect_midi_control", { inputIndex, mappings: options.midiMappings() });
      options.setMidiControlConnected(true);
      options.setMessage(`MIDI control connected with ${options.midiMappings().length} mapping(s).`);
    } catch (error) { options.setMessage(String(error)); }
  };
  const disconnectMidiControl = async () => {
    try {
      await options.invoke("disconnect_midi_control");
      options.setMidiControlConnected(false);
      options.setMessage("MIDI control disconnected.");
    } catch (error) { options.setMessage(String(error)); }
  };
  const connectMidiFeedback = async () => {
    const outputIndex = options.selectedMidiOutput();
    if (outputIndex === null) return reportMessage("No MIDI output selected.");
    try {
      await options.invoke("connect_midi_feedback", { outputIndex });
      options.setMidiFeedbackConnected(true);
      options.setMessage("MIDI feedback connected.");
    } catch (error) { options.setMessage(String(error)); }
  };
  const disconnectMidiFeedback = async () => {
    try {
      await options.invoke("disconnect_midi_feedback");
      options.setMidiFeedbackConnected(false);
      options.setMidiFeedbackEnabled(false);
      options.setMessage("MIDI feedback disconnected.");
    } catch (error) { options.setMessage(String(error)); }
  };
  const sendMidiFeedback = async (report = true) => {
    if (!options.midiFeedbackConnected() || options.midiMappings().length === 0) {
      if (report) options.setMessage("Connect MIDI feedback and add mappings first.");
      return;
    }
    try {
      const sent = await options.invoke<number>("send_midi_feedback", { mappings: options.midiMappings() });
      if (report) options.setMessage(`Sent ${sent} MIDI feedback message(s).`);
    } catch (error) {
      options.setMidiFeedbackEnabled(false);
      options.setMessage(String(error));
    }
  };
  const midiFeedbackTimer = window.setInterval(() => {
    if (options.midiFeedbackEnabled() && options.midiFeedbackConnected() && options.midiMappings().length > 0) {
      void sendMidiFeedback(false);
    }
  }, 500);
  onCleanup(() => window.clearInterval(midiFeedbackTimer));

  const addOscMapping = () => {
    const action = options.oscMapAction();
    const fixture = options.selectedFixture();
    const cueId = options.selectedOscCueId();
    const effectId = options.selectedOscEffectId();
    const nodeGraphId = options.selectedOscNodeGraphId();
    const layerId = options.selectedOscLayerId();
    const outputId = options.selectedOscVideoOutputId();
    const outputMappingField = options.selectedOscVideoOutputMappingField();
    const outputMappingPresetLabel = options.selectedOscVideoOutputMappingPresetLabel();
    const attribute = options.oscMapAttribute() || options.selectedEffectAttribute();
    if (!options.oscMapAddress().trim()) return reportMessage("Enter an OSC address.");
    if (action === "FixtureAttribute" && (!fixture || !attribute)) return reportMessage("Select a fixture and OSC attribute target first.");
    if (isFixtureFlagMappingAction(action) && !fixture) return reportMessage("Select a fixture before mapping OSC to a fixture flag.");
    if (action === "TriggerCue" && cueId === null) return reportMessage("Create a cue before mapping OSC to cues.");
    if (action === "EffectEnabled" && effectId === null) return reportMessage("Add an effect before mapping OSC to effect enable.");
    if (action === "NodeGraphEnabled" && nodeGraphId === null) return reportMessage("Add a node graph before mapping OSC to node graph enable.");
    if ((action === "GroupSubmaster" || isGroupFlagMappingAction(action)) && !options.oscMapGroupId().trim()) {
      return reportMessage("Enter a group ID before mapping OSC to a group.");
    }
    if (isVideoLayerMappingAction(action) && layerId === null) return reportMessage("Add a video layer before mapping OSC to video.");
    if (isVideoOutputMappingAction(action) && outputId === null) return reportMessage("Add a video output before mapping OSC to video output.");
    if (action === "VideoOutputMappingPreset" && !outputMappingPresetLabel) {
      return reportMessage("Save a video output mapping preset before mapping OSC to a preset.");
    }
    const mapping: OscControlMapping = {
      address: options.oscMapAddress().startsWith("/") ? options.oscMapAddress() : `/${options.oscMapAddress()}`,
      action,
      fixture_id: action === "FixtureAttribute" || isFixtureFlagMappingAction(action) ? fixture?.id ?? null : null,
      attribute: action === "FixtureAttribute" ? attribute
        : action === "ClearFixtureFlags" ? options.oscClearFixtureFlagKind()
        : action === "VideoOutputMappingField" ? outputMappingField
        : action === "VideoOutputMappingPreset" ? outputMappingPresetLabel : null,
      group_id: action === "GroupSubmaster" || isGroupFlagMappingAction(action) ? options.oscMapGroupId().trim() : null,
      cue_id: action === "TriggerCue" ? cueId : action === "EffectEnabled" ? effectId : action === "NodeGraphEnabled" ? nodeGraphId : null,
      layer_id: isVideoLayerMappingAction(action) ? layerId : null,
      output_id: isVideoOutputMappingAction(action) ? outputId : null,
      video_param: action === "VideoParam" ? options.oscMapVideoParam() : null,
      cue_point_index: action === "VideoCuePointJump" ? Math.max(0, Math.round(options.oscMapCuePointIndex())) : null,
      duration_ms: action === "VideoOutputFade" || action === "VideoLayerFade" || action === "VideoCuePointAdd" || action === "VideoCuePointRemove"
        ? Math.max(0, Math.round(options.oscMapDurationMs())) : null,
      low: options.oscMapLow(),
      high: options.oscMapHigh(),
    };
    options.setOscMappings((current) => [...current, mapping]);
    options.setMessage(`Added OSC mapping ${mapping.address}`);
  };
  const removeOscMapping = (index: number) => options.setOscMappings((current) => current.filter((_, candidate) => candidate !== index));
  const saveOscMappings = async () => {
    try {
      const path = await options.invoke<string | null>("save_osc_mappings", { mappings: options.oscMappings() });
      if (path) options.setMessage(`Saved OSC mappings to ${path}`);
    } catch (error) { options.setMessage(String(error)); }
  };
  const loadOscMappings = async () => {
    try {
      const loaded = await options.invoke<OscControlMapping[] | null>("load_osc_mappings");
      if (loaded) {
        options.setOscMappings(loaded);
        options.setMessage(`Loaded ${loaded.length} OSC mapping(s).`);
      }
    } catch (error) { options.setMessage(String(error)); }
  };
  const applyLearnedOscControl = (learned: LearnedOscControl) => {
    options.setOscMapAddress(learned.address);
    const valueLabel = learned.value === null || learned.value === undefined ? "no numeric value" : `value ${learned.value}`;
    options.setMessage(`Learned OSC ${learned.address} (${learned.argument_count} arg(s), ${valueLabel}).`);
  };
  const learnOscControl = async () => {
    if (options.oscRunning()) return reportMessage("Stop OSC input before OSC learn.");
    const config: OscInputConfig = { bind_ip: options.oscBindIp(), port: options.oscPort() };
    try {
      options.setMessage("Waiting for OSC input...");
      const learned = await options.invoke<LearnedOscControl | null>("learn_osc_control", { config });
      if (learned) applyLearnedOscControl(learned);
      else options.setMessage("OSC learn timed out.");
    } catch (error) { options.setMessage(String(error)); }
  };
  const startOscInput = async () => {
    const config: OscInputConfig = { bind_ip: options.oscBindIp(), port: options.oscPort() };
    try {
      await options.invoke("start_osc_input", { config, mappings: options.oscMappings() });
      options.setOscRunning(true);
      options.setMessage(`OSC input listening on ${config.bind_ip}:${config.port} with ${options.oscMappings().length} mapping(s)`);
    } catch (error) { options.setMessage(String(error)); }
  };
  const stopOscInput = async () => {
    try {
      await options.invoke("stop_osc_input");
      options.setOscRunning(false);
      options.setMessage("OSC input stopped.");
    } catch (error) { options.setMessage(String(error)); }
  };

  return {
    refreshMidiInputs,
    refreshMidiOutputs,
    connectMidiClock,
    disconnectMidiClock,
    addMidiMapping,
    removeMidiMapping,
    learnMidiControl,
    saveMidiMappings,
    loadMidiMappings,
    connectMidiControl,
    disconnectMidiControl,
    connectMidiFeedback,
    disconnectMidiFeedback,
    sendMidiFeedback,
    addOscMapping,
    removeOscMapping,
    saveOscMappings,
    loadOscMappings,
    learnOscControl,
    startOscInput,
    stopOscInput,
  };
}
