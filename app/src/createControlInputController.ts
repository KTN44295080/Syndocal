import { midiFaderConnection, type MidiFaderConnection } from "./midiFaderInput";
import { batch, createEffect, createSignal, onCleanup, untrack, type Accessor, type Setter } from "solid-js";
import type { FrontendTauriInvoke } from "./tauriInvokeCommands";
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
  beatDivisionMappingsFromTargets,
  midiMappingDrivesTarget,
  midiMappingsFromLearnedControl,
  oscMappingsFromLearnedControl,
  sameMidiSource,
  sameOscSource,
  type ControlMappingTarget,
  midiContinuousActions,
} from "./controlMappingLearn";
import { beatDivisionLabel } from "./controlMappingPerform";
import { audioMappingsFromTargets, audioTargetIsEligible, isAudioMapping, maxAudioMappings, type AudioAssignmentSettings } from "./audioControlMapping";
import {
  projectAuthorityLearnContinuation,
  runAfterProjectAuthorityFlush,
  type ProjectAuthorityMappingFlushResult,
  type ProjectAuthorityToken,
} from "./projectAuthority";

import { learnedFeedbackOutput, type MidiFeedbackOutputStatus } from "./midiLearnFeedback";

type Invoke = FrontendTauriInvoke;

interface MidiFeedbackRuntimeStatus {
  enabled: boolean;
  last_error: string | null;
  output: MidiFeedbackOutputStatus | null;
}

interface ControlInputControllerOptions {
  invoke: Invoke;
  // Mapping edits are debounced in App.  A mapping-driven worker must never be
  // constructed from the pre-ack clone, including the reconnect branches in
  // Learn, so every Connect/Start route shares this one barrier.
  flushProjectControlMappingsAuthority: () => Promise<ProjectAuthorityMappingFlushResult>;
  /** Captured before a Learn await; exact token equality is required before
   * the learned result can edit signals or revive an old runtime. A trusted
   * flush may explicitly return this flow's own replacement token. */
  captureProjectAuthorityIdentity: () => ProjectAuthorityToken;
  isProjectAuthorityIdentityCurrent: (captured: ProjectAuthorityToken) => boolean;
  setMessage: (message: string) => unknown;
  onMidiControlConnection?: (connection: MidiFaderConnection) => void;
  onOscControlConnection?: (connection: MidiFaderConnection) => void;
  onOscControlStarting?: () => void;
  onMidiLearnBusy?: (busy: boolean) => void;
  listenMidiLearnReady?: (handler: (ready: { learn_request_id: string; project_epoch: number }) => void) => Promise<() => void>;
  selectedFixture: Accessor<PatchedFixtureSummary | undefined>;
  selectedEffectAttribute: Accessor<string>;
  midiInputs?: Accessor<MidiInputSummary[]>;
  midiOutputs?: Accessor<MidiOutputSummary[]>;
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
  const connectInput = async (args: Record<string, unknown>) => {
    const identity = options.captureProjectAuthorityIdentity();
    const status = await options.invoke<unknown>("connect_midi_control", args);
    if (options.onMidiControlConnection
      && options.isProjectAuthorityIdentityCurrent(identity)
      && args.expectedEpoch === identity.project_epoch) {
      options.onMidiControlConnection(midiFaderConnection(status));
    }
  };
  let midiInputRefreshGeneration = 0;
  let midiOutputRefreshGeneration = 0;
  const reportMessage = (message: string): void => {
    options.setMessage(message);
  };
  const flushProjectControlMappingsAuthority = () => runAfterProjectAuthorityFlush(
    options.flushProjectControlMappingsAuthority,
    async (flushed) => flushed,
  );
  const flushForInputAction = async (): Promise<void> => {
    const flushed = await flushProjectControlMappingsAuthority();
    if (!flushed.trusted) {
      throw new Error("Project control mappings could not be durably synchronized; input start was canceled.");
    }
  };
  const learnAuthorityIsCurrent = (captured: ProjectAuthorityToken): boolean => {
    if (options.isProjectAuthorityIdentityCurrent(captured)) return true;
    options.setMessage("Project changed while Learn was waiting; the older learned input was discarded.");
    return false;
  };
  const learnAuthorityAfterFlush = (
    captured: ProjectAuthorityToken,
    flushed: ProjectAuthorityMappingFlushResult,
  ): ProjectAuthorityToken | null => {
    const continued = projectAuthorityLearnContinuation(
      captured,
      options.captureProjectAuthorityIdentity(),
      flushed,
    );
    if (continued !== null) return continued;
    options.setMessage("Project changed or control mappings were not durably synchronized while Learn was waiting; the older learned input was discarded.");
    return null;
  };
  const refreshMidiInputs = async () => {
    const requestGeneration = ++midiInputRefreshGeneration;
    try {
      const inputs = await options.invoke<MidiInputSummary[]>("list_midi_inputs");
      if (requestGeneration !== midiInputRefreshGeneration) return;
      options.setMidiInputs(inputs);
      if (options.selectedMidiInput() === null && inputs.length > 0) options.setSelectedMidiInput(inputs[0].index);
    } catch (error) {
      if (requestGeneration === midiInputRefreshGeneration) options.setMessage(String(error));
    }
  };

  const refreshMidiOutputs = async () => {
    const requestGeneration = ++midiOutputRefreshGeneration;
    try {
      const outputs = await options.invoke<MidiOutputSummary[]>("list_midi_outputs");
      if (requestGeneration !== midiOutputRefreshGeneration) return;
      options.setMidiOutputs(outputs);
      if (options.selectedMidiOutput() === null && outputs.length > 0) options.setSelectedMidiOutput(outputs[0].index);
    } catch (error) {
      if (requestGeneration === midiOutputRefreshGeneration) options.setMessage(String(error));
    }
  };

  const connectMidiClock = async () => {
    const inputIndex = options.selectedMidiInput();
    if (inputIndex === null) {
      options.setMessage("No MIDI input selected.");
      return;
    }
    let authority: ProjectAuthorityToken | null = null;
    try {
      await flushForInputAction();
      const capturedAuthority = options.captureProjectAuthorityIdentity();
      authority = capturedAuthority;
      options.setMidiConnected(false);
      await options.invoke("connect_midi_clock", {
        inputIndex,
        expectedEpoch: capturedAuthority.project_epoch,
      });
      if (!options.isProjectAuthorityIdentityCurrent(capturedAuthority)) return;
      options.setMidiConnected(true);
      options.setMessage("MIDI Clock connected.");
    } catch (error) {
      if (authority !== null && !options.isProjectAuthorityIdentityCurrent(authority)) return;
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
      if (!options.isProjectAuthorityIdentityCurrent(authority)) return;
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
    if ((action === "ToggleCue" || action === "TriggerCue" || action === "FlashCue" || action === "TriggerCueDirection" || action === "FlashCueDirection" || action === "TriggerCueListNext") && cueId === null) return reportMessage("Create a cue before mapping MIDI to cues.");
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
      number: options.midiMapMessage() === "PitchBend" ? 0 : Math.max(0, Math.min(127, Math.round(options.midiMapNumber()))),
      action,
      fixture_id: action === "FixtureAttribute" || isFixtureFlagMappingAction(action) ? fixture?.id ?? null : null,
      attribute: action === "FixtureAttribute" ? attribute
        : action === "TriggerCueDirection" || action === "FlashCueDirection" ? (["Forward", "Reverse", "Bounce"].includes(options.midiMapAttribute()) ? options.midiMapAttribute() : "Forward")
        : action === "ClearFixtureFlags" ? options.midiClearFixtureFlagKind()
        : action === "VideoOutputMappingField" ? outputMappingField
        : action === "VideoOutputMappingPreset" ? outputMappingPresetLabel : null,
      group_id: action === "GroupSubmaster" || isGroupFlagMappingAction(action) ? options.midiMapGroupId().trim() : null,
      cue_id: action === "ToggleCue" || action === "TriggerCue" || action === "FlashCue" || action === "TriggerCueDirection" || action === "FlashCueDirection" || action === "TriggerCueListNext" ? cueId : action === "EffectEnabled" ? effectId : action === "NodeGraphEnabled" ? nodeGraphId : null,
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
    options.setMessage(`Learned ${learned.message} ch ${learned.channel + 1} #${learned.number}`);
  };

  const waitForMidiLearn = async (inputIndex: number, authority: ProjectAuthorityToken, waitingMessage: string) => {
    const learnRequestId = globalThis.crypto.randomUUID();
    options.setMessage("Preparing MIDI input…");
    const unlisten = await options.listenMidiLearnReady?.((ready) => {
      if (ready.learn_request_id === learnRequestId && ready.project_epoch === authority.project_epoch
        && options.isProjectAuthorityIdentityCurrent(authority)) options.setMessage(waitingMessage);
    });
    try {
      return await options.invoke<LearnedMidiControl | null>("learn_midi_control", {
        inputIndex, expectedEpoch: authority.project_epoch, learnRequestId,
      });
    } finally { unlisten?.(); }
  };

  let midiLearnInFlight = false;
  const learnMidiControl = async () => {
    if (midiLearnInFlight) return;
    const inputIndex = options.selectedMidiInput();
    if (inputIndex === null) return reportMessage("No MIDI input selected.");
    let authority = options.captureProjectAuthorityIdentity();
    const wasConnected = options.midiControlConnected();
    let disconnectedForLearn = false;
    midiLearnInFlight = true;
    options.onMidiLearnBusy?.(true);
    try {
      const beforeLearn = await flushProjectControlMappingsAuthority();
      const beforeLearnAuthority = learnAuthorityAfterFlush(authority, beforeLearn);
      if (beforeLearnAuthority === null) return;
      authority = beforeLearnAuthority;
      if (wasConnected) {
        await options.invoke("disconnect_midi_control", { expectedEpoch: authority.project_epoch });
        if (!learnAuthorityIsCurrent(authority)) return;
        options.setMidiControlConnected(false);
        disconnectedForLearn = true;
      }
      const learned = await waitForMidiLearn(inputIndex, authority, "MIDI input ready: move or press the control now.");
      if (!learnAuthorityIsCurrent(authority)) return;
      if (learned) applyLearnedMidiControl(learned);
      else options.setMessage("MIDI learn timed out.");
    } catch (error) {
      if (!options.isProjectAuthorityIdentityCurrent(authority)) return;
      options.setMessage(String(error));
    } finally {
      try {
        if (disconnectedForLearn && learnAuthorityIsCurrent(authority)) {
          const flushed = await flushProjectControlMappingsAuthority();
          const continued = learnAuthorityAfterFlush(authority, flushed);
          if (continued !== null && options.midiMappings().length > 0) {
            authority = continued;
            await connectInput( { inputIndex, mappings: options.midiMappings(), expectedEpoch: authority.project_epoch });
            if (learnAuthorityIsCurrent(authority)) options.setMidiControlConnected(true);
          }
        }
      } catch (error) { if (options.isProjectAuthorityIdentityCurrent(authority)) options.setMessage(`MIDI connection could not be restored after Learn: ${String(error)}`); }
      finally { midiLearnInFlight = false; options.onMidiLearnBusy?.(false); }
    }
  };

  const learnMidiControlForTargets = async (targets: readonly ControlMappingTarget[]) => {
    if (midiLearnInFlight) return false;
    const inputIndex = options.selectedMidiInput();
    if (inputIndex === null) {
      reportMessage("Select a MIDI input in Setup > I/O before MIDI Learn.");
      return false;
    }
    if (targets.length === 0) {
      reportMessage("This control does not expose a MIDI mapping target.");
      return false;
    }
    let authority = options.captureProjectAuthorityIdentity();
    let previousMappings = options.midiMappings();
    const wasConnected = options.midiControlConnected();
    const inputName = options.midiInputs?.().find(input => input.index === inputIndex)?.name ?? null;
    const wasFeedbackConnected = options.midiFeedbackConnected();
    const wasFeedbackEnabled = options.midiFeedbackEnabled();
    const preservedOutputName = wasFeedbackConnected
      ? options.midiOutputs?.().find(output => output.index === options.selectedMidiOutput())?.name ?? null
      : null;
    let disconnectedForLearn = false;
    midiLearnInFlight = true;
    options.onMidiLearnBusy?.(true);
    try {
      const beforeLearn = await flushProjectControlMappingsAuthority();
      const beforeLearnAuthority = learnAuthorityAfterFlush(authority, beforeLearn);
      if (beforeLearnAuthority === null) return false;
      authority = beforeLearnAuthority;
      previousMappings = options.midiMappings();
      if (!learnAuthorityIsCurrent(authority)) return false;
      if (wasConnected) {
        await options.invoke("disconnect_midi_control", { expectedEpoch: authority.project_epoch });
        if (!learnAuthorityIsCurrent(authority)) return false;
        options.setMidiControlConnected(false);
        disconnectedForLearn = true;
      }
      const learned = await waitForMidiLearn(inputIndex, authority, `MIDI input ready: move or press the control for ${targets[0].label}.`);
      if (!learnAuthorityIsCurrent(authority)) return false;
      if (!learned) {
        if (wasConnected && learnAuthorityIsCurrent(authority)) {
          const flushed = await flushProjectControlMappingsAuthority();
          const continued = learnAuthorityAfterFlush(authority, flushed);
          if (continued === null) return false;
          authority = continued;
          const mappings = options.midiMappings();
          if (mappings.length > 0) {
            options.setMidiControlConnected(false);
            await connectInput( {
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
      // Re-learning a knob already switched to Relative encoder keeps it relative.
      const keepRelative = learned.message === "ControlChange"
        && replaced.some((mapping) => mapping.message === "RelativeControlChange");
      const learnedMappings = midiMappingsFromLearnedControl(targets, learned).map((mapping) =>
        keepRelative && midiContinuousActions.has(mapping.action)
          ? { ...mapping, message: "RelativeControlChange" as const }
          : mapping);
      const nextMappings = [
        ...previousMappings.filter((mapping) => !sameMidiSource(mapping, learned)),
        ...learnedMappings,
      ];
      options.setMidiMappings(nextMappings);
      const flushed = await flushProjectControlMappingsAuthority();
      const continued = learnAuthorityAfterFlush(authority, flushed);
      if (continued === null) return false;
      authority = continued;
      const mappings = options.midiMappings();
      if (mappings.length === 0) {
        options.setMidiControlConnected(false);
        return false;
      }
      options.setMidiControlConnected(false);
      await connectInput( {
        inputIndex,
        mappings,
        expectedEpoch: authority.project_epoch,
      });
      if (!learnAuthorityIsCurrent(authority)) return false;
      options.setMidiControlConnected(true);
      let feedbackNote = "";
      const learnedSceneFeedback = learnedMappings.some(mapping => mapping.feedback);
      if (learnedSceneFeedback || wasFeedbackConnected) {
        try {
          const outputs = await options.invoke<MidiOutputSummary[]>("list_midi_outputs");
          if (!learnAuthorityIsCurrent(authority)) return false;
          options.setMidiOutputs(outputs);
          const output = learnedFeedbackOutput(inputName, outputs, preservedOutputName);
          if (output) {
            options.setMidiFeedbackConnected(false);
            options.setMidiFeedbackEnabled(false);
            await options.invoke("connect_midi_feedback", { outputIndex: output.index, expectedOutputName: output.name, expectedEpoch: authority.project_epoch });
            if (!learnAuthorityIsCurrent(authority)) return false;
            const enableFeedback = learnedSceneFeedback || wasFeedbackEnabled;
            await configureFeedback(enableFeedback, mappings, authority);
            if (!learnAuthorityIsCurrent(authority)) return false;
            batch(() => { options.setSelectedMidiOutput(output.index); options.setMidiFeedbackConnected(true); options.setMidiFeedbackEnabled(enableFeedback); });
            feedbackNote = `; auto feedback connected to ${output.name}`;
          } else {
            feedbackNote = "; select a matching MIDI output in Setup > I/O > MIDI to enable feedback";
          }
        } catch (error) {
          if (!learnAuthorityIsCurrent(authority)) return false;
          options.setMidiFeedbackConnected(false);
          options.setMidiFeedbackEnabled(false);
          feedbackNote = `; feedback could not connect: ${String(error)}`;
        }
      }
      // A single detent of an endless encoder (Mackie V-Pot) reads 1-15 up or
      // 65-79 down; point at the Relative encoder switch rather than guessing.
      const encoderHint = learned.message === "ControlChange"
        && !keepRelative
        && learnedMappings.some((mapping) => midiContinuousActions.has(mapping.action))
        && ((learned.value >= 1 && learned.value <= 15) || (learned.value >= 65 && learned.value <= 79))
        ? "; if this is an endless knob, turn on Relative encoder for it in Setup > I/O > MIDI"
        : "";
      options.setMessage(
        `MIDI mapped ${learned.message} ch ${learned.channel + 1} #${learned.number} to ${targets[0].label}`
        + (targets.length > 1 ? ` and ${targets.length - 1} linked target(s)` : "")
        + (replaced.length > 0 ? `; replaced ${replaced.length} previous binding(s)` : "") + feedbackNote + encoderHint,
      );
      return true;
    } catch (error) {
      let restored = false;
      if (wasConnected && disconnectedForLearn && learnAuthorityIsCurrent(authority)) {
        try {
          const flushed = await flushProjectControlMappingsAuthority();
          const continued = learnAuthorityAfterFlush(authority, flushed);
          if (continued === null) return false;
          authority = continued;
          const mappings = options.midiMappings();
          if (mappings.length > 0) {
            options.setMidiControlConnected(false);
            await connectInput( {
              inputIndex,
              mappings,
              expectedEpoch: authority.project_epoch,
            });
            if (!learnAuthorityIsCurrent(authority)) return false;
            options.setMidiControlConnected(true);
            restored = true;
          }
        } catch {
          if (!options.isProjectAuthorityIdentityCurrent(authority)) return false;
          options.setMidiControlConnected(false);
        }
      }
      if (!options.isProjectAuthorityIdentityCurrent(authority)) return false;
      options.setMessage(
        `MIDI Learn failed: ${String(error)}`
        + (restored ? "; the previous mapping connection was restored." : ""),
      );
      return false;
    } finally { midiLearnInFlight = false; options.onMidiLearnBusy?.(false); }
  };

  /** Edit the mapping list outside Learn (a beat-division assignment or removal).
   * The native input worker is fenced to the exact saved list, so a connected
   * worker is re-seated on the new one; the renderer then keeps receiving
   * routed Perform input. */
  let midiMappingEditQueue: Promise<unknown> = Promise.resolve();
  const editMidiMappingsAndReseat = (
    edit: (current: MidiControlMapping[]) => MidiControlMapping[],
    message: string,
  ): Promise<boolean> => {
    // Serialize assign/remove, persist once, then re-seat against that exact ACK.
    const entryEpoch = options.captureProjectAuthorityIdentity().project_epoch;
    const canceled = () => {
      reportMessage("Project changed while editing control assignments; the edit was canceled.");
      return false;
    };
    const run = async (): Promise<boolean> => {
      if (midiLearnInFlight) return false;
      const authority = options.captureProjectAuthorityIdentity();
      if (authority.project_epoch !== entryEpoch) return canceled();
      const wasConnected = options.midiControlConnected();
      try {
        options.setMidiMappings((current) => {
          if (options.captureProjectAuthorityIdentity().project_epoch !== entryEpoch) {
            throw new Error("Project changed while editing control assignments; the edit was canceled.");
          }
          return edit(current);
        });
        const flushed = await flushProjectControlMappingsAuthority();
        if (!flushed.trusted) {
          reportMessage("Control assignments could not be saved.");
          return false;
        }
        const continued = projectAuthorityLearnContinuation(
          authority, options.captureProjectAuthorityIdentity(), flushed,
        );
        if (continued === null || continued.project_epoch !== entryEpoch) return canceled();
        if (wasConnected) {
          if (midiLearnInFlight) return false;
          const mappings = options.midiMappings();
          options.setMidiControlConnected(false);
          if (mappings.length === 0) {
            await options.invoke("disconnect_midi_control", { expectedEpoch: continued.project_epoch });
          } else {
            const inputIndex = options.selectedMidiInput();
            if (inputIndex === null) { reportMessage("No MIDI input selected."); return false; }
            await connectInput({ inputIndex, mappings, expectedEpoch: continued.project_epoch });
          }
          if (!options.isProjectAuthorityIdentityCurrent(continued)) return canceled();
          options.setMidiControlConnected(mappings.length > 0);
        }
        reportMessage(message);
        return true;
      } catch (error) {
        if (options.captureProjectAuthorityIdentity().project_epoch !== entryEpoch) return canceled();
        reportMessage(String(error));
        return false;
      }
    };
    const next = midiMappingEditQueue.then(run, run);
    midiMappingEditQueue = next.catch(() => false);
    return next;
  };

  /** Fire the target on a tempo division instead of a controller message. */
  const assignBeatDivisionForTargets = async (targets: readonly ControlMappingTarget[], divisionNumber: number) => {
    const assigned = beatDivisionMappingsFromTargets(targets, divisionNumber);
    if (assigned.length === 0) {
      reportMessage("A beat division can only trigger a clip, Take, Stop or an effect stage.");
      return false;
    }
    // One tempo source per target: choosing another division replaces it.
    return editMidiMappingsAndReseat(
      (current) => [
        ...current.filter((mapping) => !(mapping.message === "BeatDivision"
          && targets.some((target) => midiMappingDrivesTarget(mapping, target)))),
        ...assigned,
      ],
      `Assigned ${targets[0].label} to every ${beatDivisionLabel(divisionNumber)}.`,
    );
  };

  /** Audio settings share the mapping edit queue and its durable authority barrier. */
  const assignAudioForTargets = async (targets: readonly ControlMappingTarget[], settings: AudioAssignmentSettings) => {
    const eligible = targets.filter(audioTargetIsEligible);
    const assigned = audioMappingsFromTargets(eligible, settings);
    if (assigned.length === 0) return false;
    return editMidiMappingsAndReseat((current) => {
      const replaced = current.filter(mapping => isAudioMapping(mapping)
        && targets.some(target => midiMappingDrivesTarget(mapping, target)));
      const retained = current.filter(mapping => !replaced.includes(mapping));
      if (retained.filter(isAudioMapping).length + assigned.length > maxAudioMappings) {
        throw new Error("Audio assignment limit reached.");
      }
      return [...retained, ...assigned.map((mapping, index) => {
        const previous = replaced.find(candidate => midiMappingDrivesTarget(candidate, eligible[index]));
        return previous?.audio ? { ...mapping, audio: { ...mapping.audio, id: previous.audio.id } } : mapping;
      })];
    }, "Audio mapping assigned.");
  };

  /** Remove stored assignments (a MIDI controller or a beat division) by identity. */
  const removeMidiAssignments = async (removed: readonly MidiControlMapping[], label: string) => {
    if (removed.length === 0) return false;
    return editMidiMappingsAndReseat(
      (current) => current.filter((mapping) => !removed.includes(mapping)),
      `Removed ${removed.length} assignment(s) from ${label}.`,
    );
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

  /** Resolves true only when the input is connected on the current project. */
  const connectMidiControl = async (): Promise<boolean> => {
    const inputIndex = options.selectedMidiInput();
    if (inputIndex === null) { reportMessage("No MIDI input selected."); return false; }
    if (options.midiMappings().length === 0) { reportMessage("Add at least one MIDI mapping first."); return false; }
    let authority: ProjectAuthorityToken | null = null;
    try {
      await flushForInputAction();
      const capturedAuthority = options.captureProjectAuthorityIdentity();
      authority = capturedAuthority;
      const mappings = options.midiMappings();
      if (mappings.length === 0) { reportMessage("Add at least one MIDI mapping first."); return false; }
      options.setMidiControlConnected(false);
      await connectInput( {
        inputIndex,
        mappings,
        expectedEpoch: capturedAuthority.project_epoch,
      });
      if (!options.isProjectAuthorityIdentityCurrent(capturedAuthority)) return false;
      options.setMidiControlConnected(true);
      options.setMessage(`MIDI control connected with ${mappings.length} mapping(s).`);
      return true;
    } catch (error) {
      if (authority !== null && !options.isProjectAuthorityIdentityCurrent(authority)) return false;
      options.setMidiControlConnected(false);
      options.setMessage(String(error));
      return false;
    }
  };
  /** Resolves true only when the input was disconnected on the current project. */
  const disconnectMidiControl = async (): Promise<boolean> => {
    const authority = options.captureProjectAuthorityIdentity();
    try {
      await options.invoke("disconnect_midi_control", { expectedEpoch: authority.project_epoch });
      if (!options.isProjectAuthorityIdentityCurrent(authority)) return false;
      options.setMidiControlConnected(false);
      options.setMessage("MIDI control disconnected.");
      return true;
    } catch (error) {
      if (!options.isProjectAuthorityIdentityCurrent(authority)) return false;
      options.setMessage(String(error));
      return false;
    }
  };
  const connectMidiFeedback = async () => {
    const outputIndex = options.selectedMidiOutput();
    if (outputIndex === null) return reportMessage("No MIDI output selected.");
    let authority: ProjectAuthorityToken | null = null;
    try {
      await flushForInputAction();
      const capturedAuthority = options.captureProjectAuthorityIdentity();
      authority = capturedAuthority;
      options.setMidiFeedbackConnected(false);
      options.setMidiFeedbackEnabled(false);
      await options.invoke("connect_midi_feedback", {
        outputIndex,
        expectedEpoch: capturedAuthority.project_epoch,
      });
      if (!options.isProjectAuthorityIdentityCurrent(capturedAuthority)) return;
      options.setMidiFeedbackConnected(true);
      options.setMessage("MIDI feedback connected.");
    } catch (error) {
      if (authority !== null && !options.isProjectAuthorityIdentityCurrent(authority)) return;
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
    } catch (error) {
      if (!options.isProjectAuthorityIdentityCurrent(authority)) return;
      options.setMessage(String(error));
    }
  };
  const sendMidiFeedback = async (report = true) => {
    if (!options.midiFeedbackConnected() || options.midiMappings().length === 0) {
      if (report) options.setMessage("Connect MIDI feedback and add mappings first.");
      return;
    }
    const authority = options.captureProjectAuthorityIdentity();
    const mappings = options.midiMappings();
    try {
      const sent = await options.invoke<number>("send_midi_feedback", {
        mappings,
        force: true,
        expectedEpoch: authority.project_epoch,
      });
      if (!options.isProjectAuthorityIdentityCurrent(authority)) return;
      if (report) options.setMessage(`Sent ${sent} MIDI feedback message(s).`);
    } catch (error) {
      if (!options.isProjectAuthorityIdentityCurrent(authority)) return;
      options.setMidiFeedbackEnabled(false);
      options.setMessage(String(error));
    }
  };
  let appliedFeedbackConfiguration: string | null = null;
  const configureFeedback = async (enabled: boolean, mappings: MidiControlMapping[], authority: ProjectAuthorityToken) => {
    const key = JSON.stringify([enabled, mappings, authority]);
    if (appliedFeedbackConfiguration === key) return;
    await options.invoke<MidiFeedbackRuntimeStatus>("set_midi_feedback_auto", { enabled, mappings, expectedEpoch: authority.project_epoch });
    if (options.isProjectAuthorityIdentityCurrent(authority)) appliedFeedbackConfiguration = key;
  };
  let midiFeedbackConfigurationGeneration = 0;
  let midiFeedbackConfigurationQueue = Promise.resolve();
  createEffect(() => {
    const connected = options.midiFeedbackConnected();
    const enabled = options.midiFeedbackEnabled();
    const mappings = options.midiMappings();
    if (!connected) { appliedFeedbackConfiguration = null; ++midiFeedbackConfigurationGeneration; return; }
    if (enabled && mappings.length === 0) {
      options.setMidiFeedbackEnabled(false);
      options.setMessage("Add at least one MIDI mapping before enabling auto feedback.");
      return;
    }
    const generation = ++midiFeedbackConfigurationGeneration;
    const requestedEpoch = untrack(() => options.captureProjectAuthorityIdentity().project_epoch);
    midiFeedbackConfigurationQueue = midiFeedbackConfigurationQueue.then(async () => {
      const isCurrent = () => generation === midiFeedbackConfigurationGeneration
        && options.midiFeedbackConnected()
        && options.captureProjectAuthorityIdentity().project_epoch === requestedEpoch;
      if (!isCurrent()) return;
      let authority = options.captureProjectAuthorityIdentity();
      try {
        // Audio shares this list. Configure feedback only after the exact
        // persisted mapping ACK, including edits made while a flush was pending.
        const flushed = await flushProjectControlMappingsAuthority();
        if (!isCurrent()) return;
        const continued = projectAuthorityLearnContinuation(
          authority, options.captureProjectAuthorityIdentity(), flushed,
        );
        if (continued === null) {
          if (!flushed.trusted) reportMessage("Control assignments could not be saved.");
          return;
        }
        authority = continued;
        await configureFeedback(options.midiFeedbackEnabled(), options.midiMappings(), authority);
        if (!isCurrent() || !options.isProjectAuthorityIdentityCurrent(authority)) return;
      } catch (error) {
        if (!isCurrent() || !options.isProjectAuthorityIdentityCurrent(authority)) return;
        options.setMidiFeedbackEnabled(false);
        options.setMessage(String(error));
      }
    });
  });
  // While feedback is connected, poll the backend so the operator can see what
  // was actually sent; a running auto worker that stops is reported once.
  const [midiFeedbackOutputStatus, setMidiFeedbackOutputStatus] = createSignal<MidiFeedbackOutputStatus | null>(null);
  createEffect(() => {
    if (!options.midiFeedbackConnected()) { setMidiFeedbackOutputStatus(null); return; }
    let disposed = false;
    const statusTimer = window.setInterval(async () => {
      try {
        const status = await options.invoke<MidiFeedbackRuntimeStatus>("midi_feedback_status");
        if (disposed) return;
        setMidiFeedbackOutputStatus(status.output ?? null);
        if (!options.midiFeedbackEnabled() || status.enabled) return;
        options.setMidiFeedbackEnabled(false);
        options.setMessage(status.last_error ?? "MIDI auto feedback stopped unexpectedly.");
      } catch (error) {
        if (disposed || !options.midiFeedbackEnabled()) return;
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
    } catch (error) {
      if (!options.isProjectAuthorityIdentityCurrent(authority)) return;
      options.setMessage(String(error));
    }
  };
  const learnOscControlForTargets = async (targets: readonly ControlMappingTarget[]) => {
    if (targets.length === 0) {
      reportMessage("This control does not expose an OSC mapping target.");
      return false;
    }
    let authority = options.captureProjectAuthorityIdentity();
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
          const flushed = await flushProjectControlMappingsAuthority();
          const continued = learnAuthorityAfterFlush(authority, flushed);
          if (continued === null) return false;
          authority = continued;
          const mappings = options.oscMappings();
          options.setOscRunning(false);
          options.onOscControlStarting?.();
          const connection = await options.invoke<unknown>("start_osc_input", {
            config,
            mappings,
            expectedEpoch: authority.project_epoch,
          });
          if (!learnAuthorityIsCurrent(authority)) return false;
          options.setOscRunning(true);
          options.onOscControlConnection?.(midiFaderConnection(connection, "OSC"));
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
      const flushed = await flushProjectControlMappingsAuthority();
      const continued = learnAuthorityAfterFlush(authority, flushed);
      if (continued === null) return false;
      authority = continued;
      options.setOscRunning(false);
      options.onOscControlStarting?.();
      const connection = await options.invoke<unknown>("start_osc_input", {
        config,
        mappings: options.oscMappings(),
        expectedEpoch: authority.project_epoch,
      });
      if (!learnAuthorityIsCurrent(authority)) return false;
      options.setOscRunning(true);
      options.onOscControlConnection?.(midiFaderConnection(connection, "OSC"));
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
          const flushed = await flushProjectControlMappingsAuthority();
          const continued = learnAuthorityAfterFlush(authority, flushed);
          if (continued === null) return false;
          authority = continued;
          options.setOscRunning(false);
          options.onOscControlStarting?.();
          const connection = await options.invoke<unknown>("start_osc_input", {
            config,
            mappings: options.oscMappings(),
            expectedEpoch: authority.project_epoch,
          });
          if (!learnAuthorityIsCurrent(authority)) return false;
          options.setOscRunning(true);
          options.onOscControlConnection?.(midiFaderConnection(connection, "OSC"));
          restored = true;
        } catch {
          if (!options.isProjectAuthorityIdentityCurrent(authority)) return false;
          options.setOscRunning(false);
        }
      }
      if (!options.isProjectAuthorityIdentityCurrent(authority)) return false;
      options.setMessage(
        `OSC Learn failed: ${String(error)}`
        + (restored ? "; the previous listener was restored." : ""),
      );
      return false;
    }
  };
  const startOscInput = async () => {
    const config: OscInputConfig = { bind_ip: options.oscBindIp(), port: options.oscPort() };
    let authority: ProjectAuthorityToken | null = null;
    try {
      await flushForInputAction();
      const capturedAuthority = options.captureProjectAuthorityIdentity();
      authority = capturedAuthority;
      options.setOscRunning(false);
      options.onOscControlStarting?.();
      const connection = await options.invoke<unknown>("start_osc_input", {
        config,
        mappings: options.oscMappings(),
        expectedEpoch: capturedAuthority.project_epoch,
      });
      if (!options.isProjectAuthorityIdentityCurrent(capturedAuthority)) return;
      options.setOscRunning(true);
      options.onOscControlConnection?.(midiFaderConnection(connection, "OSC"));
      options.setMessage(`OSC input listening on ${config.bind_ip}:${config.port} with ${options.oscMappings().length} mapping(s)`);
    } catch (error) {
      if (authority !== null && !options.isProjectAuthorityIdentityCurrent(authority)) return;
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
    } catch (error) {
      if (!options.isProjectAuthorityIdentityCurrent(authority)) return;
      options.setMessage(String(error));
    }
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
    assignBeatDivisionForTargets,
    assignAudioForTargets,
    removeMidiAssignments,
    saveMidiMappings,
    loadMidiMappings,
    connectMidiControl,
    disconnectMidiControl,
    connectMidiFeedback,
    disconnectMidiFeedback,
    sendMidiFeedback,
    midiFeedbackOutputStatus,
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
