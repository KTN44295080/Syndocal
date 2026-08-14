import { createEffect, onCleanup, type Accessor, type Setter } from "solid-js";
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
import {
  midiMappingsFromLearnedControl,
  oscMappingsFromLearnedControl,
  sameMidiSource,
  sameOscSource,
  type ControlMappingTarget,
} from "./controlMappingLearn";
import {
  runAfterProjectAuthorityFlush,
  type ProjectAuthorityToken,
} from "./projectAuthority";

type Invoke = <T>(command: string, args?: Record<string, unknown>) => Promise<T>;

interface MidiFeedbackRuntimeStatus {
  enabled: boolean;
  last_error: string | null;
}

interface ControlInputControllerOptions {
  invoke: Invoke;
  // Mapping edits are debounced in App.  A mapping-driven worker must never be
  // constructed from the pre-ack clone, including the reconnect branches in
  // Learn, so every Connect/Start route shares this one barrier.
  flushProjectControlMappingsAuthority: () => Promise<void>;
  /** Captured before a Learn await; exact token equality is required before
   * the learned result can edit signals or revive an old runtime. */
  captureProjectAuthorityIdentity: () => ProjectAuthorityToken;
  isProjectAuthorityIdentityCurrent: (captured: ProjectAuthorityToken) => boolean;
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
  midiControlConnected: Accessor<boolean>;
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
  const flushProjectControlMappingsAuthority = () => runAfterProjectAuthorityFlush(
    options.flushProjectControlMappingsAuthority,
    async () => undefined,
  );
  const learnAuthorityIsCurrent = (captured: ProjectAuthorityToken): boolean => {
    if (options.isProjectAuthorityIdentityCurrent(captured)) return true;
    options.setMessage("Project changed while Learn was waiting; the older learned input was discarded.");
    return false;
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
      await flushProjectControlMappingsAuthority();
      const authority = options.captureProjectAuthorityIdentity();
      options.setMidiConnected(false);
      await options.invoke("connect_midi_clock", {
        inputIndex,
        expectedEpoch: authority.project_epoch,
      });
      if (!options.isProjectAuthorityIdentityCurrent(authority)) return;
      options.setMidiConnected(true);
      options.setMessage("MIDI Clock connected.");
    } catch (error) {
      options.setMidiConnected(false);
      options.setMessage(String(error));
    }
  };

  const disconnectMidiClock = async () => {
    const authority = options.captureProjectAuthorityIdentity();
    try {
      await options.invoke("disconnect_midi_clock", { expectedEpoch: authority.project_epoch });
      if (!options.isProjectAuthorityIdentityCurrent(authority)) return;
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
    if ((action === "TriggerCue" || action === "FlashCue" || action === "TriggerCueDirection" || action === "FlashCueDirection" || action === "TriggerCueListNext") && cueId === null) return reportMessage("Create a cue before mapping MIDI to cues.");
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
        : action === "TriggerCueDirection" || action === "FlashCueDirection" ? (["Forward", "Reverse", "Bounce"].includes(options.midiMapAttribute()) ? options.midiMapAttribute() : "Forward")
        : action === "ClearFixtureFlags" ? options.midiClearFixtureFlagKind()
        : action === "VideoOutputMappingField" ? outputMappingField
        : action === "VideoOutputMappingPreset" ? outputMappingPresetLabel : null,
      group_id: action === "GroupSubmaster" || isGroupFlagMappingAction(action) ? options.midiMapGroupId().trim() : null,
      cue_id: action === "TriggerCue" || action === "FlashCue" || action === "TriggerCueDirection" || action === "FlashCueDirection" || action === "TriggerCueListNext" ? cueId : action === "EffectEnabled" ? effectId : action === "NodeGraphEnabled" ? nodeGraphId : null,
      layer_id: isVideoLayerMappingAction(action) ? layerId : null,
      output_id: isVideoOutputMappingAction(action) ? outputId : null,
      video_param: action === "VideoParam" ? options.midiMapVideoParam() : null,
      cue_point_index: action === "VideoCuePointJump" || action === "SelectedFeatureFader"
        ? Math.max(0, Math.round(options.midiMapCuePointIndex())) : null,
      duration_ms: action === "VideoOutputFade" || action === "VideoLayerFade" || action === "VideoCuePointAdd" || action === "VideoCuePointRemove"
        ? Math.max(0, Math.round(options.midiMapDurationMs())) : null,
      low: options.midiMapLow(),
      high: options.midiMapHigh(),
    };
    options.setMidiMappings((current) => [...current, mapping]);
    options.setMessage(`Added MIDI mapping ${mapping.message} ${mapping.number}`);
  };

  const removeMidiMapping = (index: number) => options.setMidiMappings((current) => current.filter((_, candidate) => candidate !== index));
  const updateMidiMapping = (index: number, mapping: MidiControlMapping) => {
    options.setMidiMappings((current) => current.map((candidate, candidateIndex) => (
      candidateIndex === index ? mapping : candidate
    )));
  };

  const applyLearnedMidiControl = (learned: LearnedMidiControl) => {
    options.setMidiMapChannel(learned.channel);
    options.setMidiMapMessage(learned.message);
    options.setMidiMapNumber(learned.number);
    options.setMessage(`Learned ${learned.message} ch ${learned.channel} #${learned.number}`);
  };

  const learnMidiControl = async () => {
    const inputIndex = options.selectedMidiInput();
    if (inputIndex === null) return reportMessage("No MIDI input selected.");
    const authority = options.captureProjectAuthorityIdentity();
    try {
      options.setMessage("Waiting for MIDI input...");
      const learned = await options.invoke<LearnedMidiControl | null>("learn_midi_control", {
        inputIndex,
        expectedEpoch: authority.project_epoch,
      });
      if (!learnAuthorityIsCurrent(authority)) return;
      if (learned) applyLearnedMidiControl(learned);
      else options.setMessage("MIDI learn timed out.");
    } catch (error) {
      options.setMessage(String(error));
    }
  };

  const learnMidiControlForTargets = async (targets: readonly ControlMappingTarget[]) => {
    const inputIndex = options.selectedMidiInput();
    if (inputIndex === null) {
      reportMessage("Select a MIDI input in Setup > I/O before MIDI Learn.");
      return false;
    }
    if (targets.length === 0) {
      reportMessage("This control does not expose a MIDI mapping target.");
      return false;
    }
    const authority = options.captureProjectAuthorityIdentity();
    const previousMappings = options.midiMappings();
    const wasConnected = options.midiControlConnected();
    let disconnectedForLearn = false;
    try {
      if (!learnAuthorityIsCurrent(authority)) return false;
      if (wasConnected) {
        await options.invoke("disconnect_midi_control", { expectedEpoch: authority.project_epoch });
        if (!learnAuthorityIsCurrent(authority)) return false;
        options.setMidiControlConnected(false);
        disconnectedForLearn = true;
      }
      options.setMessage(`MIDI Learn: move a control for ${targets[0].label}...`);
      const learned = await options.invoke<LearnedMidiControl | null>("learn_midi_control", {
        inputIndex,
        expectedEpoch: authority.project_epoch,
      });
      if (!learnAuthorityIsCurrent(authority)) return false;
      if (!learned) {
        if (wasConnected && learnAuthorityIsCurrent(authority)) {
          await flushProjectControlMappingsAuthority();
          if (!learnAuthorityIsCurrent(authority)) return false;
          const mappings = options.midiMappings();
          if (mappings.length > 0) {
            options.setMidiControlConnected(false);
            await options.invoke("connect_midi_control", {
              inputIndex,
              mappings,
              expectedEpoch: authority.project_epoch,
            });
            if (!learnAuthorityIsCurrent(authority)) return false;
            options.setMidiControlConnected(true);
          }
        }
        options.setMessage("MIDI Learn timed out; the previous mapping connection was restored.");
        return false;
      }
      applyLearnedMidiControl(learned);
      if (!learnAuthorityIsCurrent(authority)) return false;
      const replaced = previousMappings.filter((mapping) => sameMidiSource(mapping, learned));
      const nextMappings = [
        ...previousMappings.filter((mapping) => !sameMidiSource(mapping, learned)),
        ...midiMappingsFromLearnedControl(targets, learned),
      ];
      options.setMidiMappings(nextMappings);
      await flushProjectControlMappingsAuthority();
      if (!learnAuthorityIsCurrent(authority)) return false;
      const mappings = options.midiMappings();
      if (mappings.length === 0) {
        options.setMidiControlConnected(false);
        return false;
      }
      options.setMidiControlConnected(false);
      await options.invoke("connect_midi_control", {
        inputIndex,
        mappings,
        expectedEpoch: authority.project_epoch,
      });
      if (!learnAuthorityIsCurrent(authority)) return false;
      options.setMidiControlConnected(true);
      options.setMessage(
        `MIDI mapped ${learned.message} ch ${learned.channel + 1} #${learned.number} to ${targets[0].label}`
        + (targets.length > 1 ? ` and ${targets.length - 1} linked target(s)` : "")
        + (replaced.length > 0 ? `; replaced ${replaced.length} previous binding(s)` : ""),
      );
      return true;
    } catch (error) {
      let restored = false;
      if (wasConnected && disconnectedForLearn && learnAuthorityIsCurrent(authority)) {
        try {
          await flushProjectControlMappingsAuthority();
          if (!learnAuthorityIsCurrent(authority)) return false;
          const mappings = options.midiMappings();
          if (mappings.length > 0) {
            options.setMidiControlConnected(false);
            await options.invoke("connect_midi_control", {
              inputIndex,
              mappings,
              expectedEpoch: authority.project_epoch,
            });
            if (!learnAuthorityIsCurrent(authority)) return false;
            options.setMidiControlConnected(true);
            restored = true;
          }
        } catch {
          options.setMidiControlConnected(false);
        }
      }
      options.setMessage(
        `MIDI Learn failed: ${String(error)}`
        + (restored ? "; the previous mapping connection was restored." : ""),
      );
      return false;
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
      await flushProjectControlMappingsAuthority();
      const authority = options.captureProjectAuthorityIdentity();
      const mappings = options.midiMappings();
      if (mappings.length === 0) return reportMessage("Add at least one MIDI mapping first.");
      options.setMidiControlConnected(false);
      await options.invoke("connect_midi_control", {
        inputIndex,
        mappings,
        expectedEpoch: authority.project_epoch,
      });
      if (!options.isProjectAuthorityIdentityCurrent(authority)) return;
      options.setMidiControlConnected(true);
      options.setMessage(`MIDI control connected with ${mappings.length} mapping(s).`);
    } catch (error) {
      options.setMidiControlConnected(false);
      options.setMessage(String(error));
    }
  };
  const disconnectMidiControl = async () => {
    const authority = options.captureProjectAuthorityIdentity();
    try {
      await options.invoke("disconnect_midi_control", { expectedEpoch: authority.project_epoch });
      if (!options.isProjectAuthorityIdentityCurrent(authority)) return;
      options.setMidiControlConnected(false);
      options.setMessage("MIDI control disconnected.");
    } catch (error) { options.setMessage(String(error)); }
  };
  const connectMidiFeedback = async () => {
    const outputIndex = options.selectedMidiOutput();
    if (outputIndex === null) return reportMessage("No MIDI output selected.");
    try {
      await flushProjectControlMappingsAuthority();
      const authority = options.captureProjectAuthorityIdentity();
      options.setMidiFeedbackConnected(false);
      options.setMidiFeedbackEnabled(false);
      await options.invoke("connect_midi_feedback", {
        outputIndex,
        expectedEpoch: authority.project_epoch,
      });
      if (!options.isProjectAuthorityIdentityCurrent(authority)) return;
      options.setMidiFeedbackConnected(true);
      options.setMessage("MIDI feedback connected.");
    } catch (error) {
      options.setMidiFeedbackConnected(false);
      options.setMidiFeedbackEnabled(false);
      options.setMessage(String(error));
    }
  };
  const disconnectMidiFeedback = async () => {
    const authority = options.captureProjectAuthorityIdentity();
    try {
      await options.invoke("disconnect_midi_feedback", { expectedEpoch: authority.project_epoch });
      if (!options.isProjectAuthorityIdentityCurrent(authority)) return;
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
      const sent = await options.invoke<number>("send_midi_feedback", {
        mappings: options.midiMappings(),
        force: true,
      });
      if (report) options.setMessage(`Sent ${sent} MIDI feedback message(s).`);
    } catch (error) {
      options.setMidiFeedbackEnabled(false);
      options.setMessage(String(error));
    }
  };
  let midiFeedbackConfigurationGeneration = 0;
  let midiFeedbackConfigurationQueue = Promise.resolve();
  createEffect(() => {
    const connected = options.midiFeedbackConnected();
    const enabled = options.midiFeedbackEnabled();
    const mappings = options.midiMappings();
    if (!connected) return;
    if (enabled && mappings.length === 0) {
      options.setMidiFeedbackEnabled(false);
      options.setMessage("Add at least one MIDI mapping before enabling auto feedback.");
      return;
    }
    const generation = ++midiFeedbackConfigurationGeneration;
    midiFeedbackConfigurationQueue = midiFeedbackConfigurationQueue.then(async () => {
      const authority = options.captureProjectAuthorityIdentity();
      try {
        await options.invoke<MidiFeedbackRuntimeStatus>("set_midi_feedback_auto", {
          enabled,
          mappings,
          expectedEpoch: authority.project_epoch,
        });
        if (!options.isProjectAuthorityIdentityCurrent(authority)) return;
      } catch (error) {
        if (generation !== midiFeedbackConfigurationGeneration) return;
        if (!options.isProjectAuthorityIdentityCurrent(authority)) return;
        options.setMidiFeedbackEnabled(false);
        options.setMessage(String(error));
      }
    });
  });
  createEffect(() => {
    if (!options.midiFeedbackEnabled() || !options.midiFeedbackConnected()) return;
    let disposed = false;
    const statusTimer = window.setInterval(async () => {
      try {
        const status = await options.invoke<MidiFeedbackRuntimeStatus>("midi_feedback_status");
        if (disposed || status.enabled) return;
        options.setMidiFeedbackEnabled(false);
        options.setMessage(status.last_error ?? "MIDI auto feedback stopped unexpectedly.");
      } catch (error) {
        if (disposed) return;
        options.setMidiFeedbackEnabled(false);
        options.setMessage(String(error));
      }
    }, 1_000);
    onCleanup(() => {
      disposed = true;
      window.clearInterval(statusTimer);
    });
  });

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
      cue_point_index: action === "VideoCuePointJump" || action === "SelectedFeatureFader"
        ? Math.max(0, Math.round(options.oscMapCuePointIndex())) : null,
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
    const authority = options.captureProjectAuthorityIdentity();
    const config: OscInputConfig = { bind_ip: options.oscBindIp(), port: options.oscPort() };
    try {
      options.setMessage("Waiting for OSC input...");
      const learned = await options.invoke<LearnedOscControl | null>("learn_osc_control", {
        config,
        expectedEpoch: authority.project_epoch,
      });
      if (!learnAuthorityIsCurrent(authority)) return;
      if (learned) applyLearnedOscControl(learned);
      else options.setMessage("OSC learn timed out.");
    } catch (error) { options.setMessage(String(error)); }
  };
  const learnOscControlForTargets = async (targets: readonly ControlMappingTarget[]) => {
    if (targets.length === 0) {
      reportMessage("This control does not expose an OSC mapping target.");
      return false;
    }
    const authority = options.captureProjectAuthorityIdentity();
    const config: OscInputConfig = { bind_ip: options.oscBindIp(), port: options.oscPort() };
    const previousMappings = options.oscMappings();
    const wasRunning = options.oscRunning();
    let stoppedForLearn = false;
    try {
      if (!learnAuthorityIsCurrent(authority)) return false;
      if (wasRunning) {
        await options.invoke("stop_osc_input", { expectedEpoch: authority.project_epoch });
        if (!learnAuthorityIsCurrent(authority)) return false;
        options.setOscRunning(false);
        stoppedForLearn = true;
      }
      options.setMessage(`OSC Learn: send a message for ${targets[0].label}...`);
      const learned = await options.invoke<LearnedOscControl | null>("learn_osc_control", {
        config,
        expectedEpoch: authority.project_epoch,
      });
      if (!learnAuthorityIsCurrent(authority)) return false;
      if (!learned) {
        if (wasRunning && learnAuthorityIsCurrent(authority)) {
          await flushProjectControlMappingsAuthority();
          if (!learnAuthorityIsCurrent(authority)) return false;
          const mappings = options.oscMappings();
          options.setOscRunning(false);
          await options.invoke("start_osc_input", {
            config,
            mappings,
            expectedEpoch: authority.project_epoch,
          });
          if (!learnAuthorityIsCurrent(authority)) return false;
          options.setOscRunning(true);
        }
        options.setMessage("OSC Learn timed out; the previous listener was restored.");
        return false;
      }
      applyLearnedOscControl(learned);
      if (!learnAuthorityIsCurrent(authority)) return false;
      const replaced = previousMappings.filter((mapping) => sameOscSource(mapping, learned));
      const nextMappings = [
        ...previousMappings.filter((mapping) => !sameOscSource(mapping, learned)),
        ...oscMappingsFromLearnedControl(targets, learned),
      ];
      options.setOscMappings(nextMappings);
      await flushProjectControlMappingsAuthority();
      if (!learnAuthorityIsCurrent(authority)) return false;
      options.setOscRunning(false);
      await options.invoke("start_osc_input", {
        config,
        mappings: options.oscMappings(),
        expectedEpoch: authority.project_epoch,
      });
      if (!learnAuthorityIsCurrent(authority)) return false;
      options.setOscRunning(true);
      options.setMessage(
        `OSC mapped ${learned.address} to ${targets[0].label}`
        + (targets.length > 1 ? ` and ${targets.length - 1} linked target(s)` : "")
        + (replaced.length > 0 ? `; replaced ${replaced.length} previous binding(s)` : ""),
      );
      return true;
    } catch (error) {
      let restored = false;
      if (wasRunning && stoppedForLearn && learnAuthorityIsCurrent(authority)) {
        try {
          await flushProjectControlMappingsAuthority();
          if (!learnAuthorityIsCurrent(authority)) return false;
          options.setOscRunning(false);
          await options.invoke("start_osc_input", {
            config,
            mappings: options.oscMappings(),
            expectedEpoch: authority.project_epoch,
          });
          if (!learnAuthorityIsCurrent(authority)) return false;
          options.setOscRunning(true);
          restored = true;
        } catch {
          options.setOscRunning(false);
        }
      }
      options.setMessage(
        `OSC Learn failed: ${String(error)}`
        + (restored ? "; the previous listener was restored." : ""),
      );
      return false;
    }
  };
  const startOscInput = async () => {
    const config: OscInputConfig = { bind_ip: options.oscBindIp(), port: options.oscPort() };
    try {
      await flushProjectControlMappingsAuthority();
      const authority = options.captureProjectAuthorityIdentity();
      options.setOscRunning(false);
      await options.invoke("start_osc_input", {
        config,
        mappings: options.oscMappings(),
        expectedEpoch: authority.project_epoch,
      });
      if (!options.isProjectAuthorityIdentityCurrent(authority)) return;
      options.setOscRunning(true);
      options.setMessage(`OSC input listening on ${config.bind_ip}:${config.port} with ${options.oscMappings().length} mapping(s)`);
    } catch (error) {
      options.setOscRunning(false);
      options.setMessage(String(error));
    }
  };
  const stopOscInput = async () => {
    const authority = options.captureProjectAuthorityIdentity();
    try {
      await options.invoke("stop_osc_input", { expectedEpoch: authority.project_epoch });
      if (!options.isProjectAuthorityIdentityCurrent(authority)) return;
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
    updateMidiMapping,
    learnMidiControl,
    learnMidiControlForTargets,
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
    learnOscControlForTargets,
    startOscInput,
    stopOscInput,
  };
}
