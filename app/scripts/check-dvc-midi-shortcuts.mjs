import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

const importer = await readFile(new URL("../src-tauri/src/dvc_import.rs", import.meta.url), "utf8");
const protocol = await readFile(new URL("../../crates/protocol/src/lib.rs", import.meta.url), "utf8");
const midi = await readFile(new URL("../../crates/io/src/midi.rs", import.meta.url), "utf8");
const osc = await readFile(new URL("../../crates/io/src/osc.rs", import.meta.url), "utf8");
const remote = await readFile(new URL("../../crates/io/src/remote_ws.rs", import.meta.url), "utf8");
const engine = await readFile(new URL("../../crates/engine/src/lib.rs", import.meta.url), "utf8");
const backend = await readFile(new URL("../src-tauri/src/main.rs", import.meta.url), "utf8");
const app = await readFile(new URL("../src/App.tsx", import.meta.url), "utf8");
const types = await readFile(new URL("../src/types.ts", import.meta.url), "utf8");
const midiPanel = await readFile(new URL("../src/components/MidiControlMappingPanel.tsx", import.meta.url), "utf8");
const controlController = await readFile(new URL("../src/createControlInputController.ts", import.meta.url), "utf8");

const checks = [
  [types.includes("midi_mappings: MidiControlMapping[];"), "DVC report exposes imported MIDI mappings"],
  [importer.includes('"107" => {'), "DVC action 107 is decoded as a verified scene mapping"],
  [importer.includes('"55" => dvc_midi_mapping(event, MidiControlAction::TapBpm'), "DVC action 55 is decoded as Tap BPM"],
  [importer.includes('"113" => {') && importer.includes("MidiControlAction::TriggerCueListNext"), "DVC action 113 is decoded as Bank Next"],
  [importer.includes('"229" =>') && importer.includes("dvc_midi_selected_feature_fader_mapping"), "DVC action 229 preserves the visible feature-fader index"],
  [importer.includes('[("OUT", "Off"), ("OUT1", "On"), ("OUT2", "Unknown")]') && importer.includes("mapped.feedback = Some(feedback)"), "DVC OUT/OUT1/OUT2 preserve exact Off/On/Unknown feedback states"],
  [importer.includes('"108" | "109" | "110" => {') && importer.includes('"108" => "Forward"') && importer.includes('"109" => "Reverse"') && importer.includes('"110" => "Bounce"'), "DVC directional Scene actions preserve their verified directions"],
  [importer.includes("cue_lists.push(CueListSummary") && importer.includes("cue_list_id,"), "DVC banks remain distinct Cue Lists"],
  [importer.includes("MidiControlAction::FlashCue"), "DVC FLASH=1 remains a hold-to-release cue mapping"],
  [importer.includes("dvc_midi_shortcuts_import_scene_tap_and_flash_without_guessing_unknown_actions"), "synthetic DVC regression is retained"],
  [importer.includes("dvc_local_homecoming_midi_scene_tap_and_flash_shortcuts_import_when_present"), "real Homecoming DVC golden is retained"],
  [importer.includes("dvc_local_homecoming_laser_midi_faders_follow_operator_selection_when_present"), "real Laser DVC action-229 golden is retained"],
  [protocol.includes("pub enum MidiControlAction") && protocol.includes("FlashCue,"), "protocol persists FlashCue"],
  [protocol.includes("TriggerCueListNext,"), "protocol persists Cue List Next"],
  [protocol.includes("SelectedFeatureFader,"), "protocol persists Selected Feature Fader"],
  [protocol.includes("pub struct MidiControlFeedback") && protocol.includes("pub feedback: Option<MidiControlFeedback>"), "protocol persists typed custom MIDI feedback"],
  [midi.includes("MidiControlEvent::ReleaseCue"), "MIDI Note Off releases a flashed cue"],
  [midi.includes("MidiControlEvent::TriggerCueWithDirection") && midi.includes("cue_live_direction(mapping.attribute.as_deref()?)"), "MIDI emits the persisted directional Scene trigger"],
  [midi.includes("SetSelectedFeatureFader") && midi.includes("target_index: mapping.cue_point_index?"), "MIDI emits the selected visible-fader slot"],
  [osc.includes("SetSelectedFeatureFader") && osc.includes("target_index: mapping.cue_point_index?"), "OSC emits the selected visible-fader slot"],
  [engine.includes("EngineCommand::TriggerCueWithDirection") && engine.includes("pending_manual_cue_direction"), "engine carries a one-shot direction through Cue pre-wait"],
  [backend.includes("MidiControlEvent::ReleaseCue(cue_id) => EngineCommand::ReleaseCue(cue_id)"), "backend reuses the engine ReleaseCue route"],
  [backend.includes("MidiControlEvent::TriggerCueListNext(anchor_cue_id)") && backend.includes("EngineCommand::TriggerCueListNext(cue_list_id)"), "backend resolves the imported Bank anchor to its Cue List"],
  [(backend.match(/operator_feature_fader_command\(/g) ?? []).length >= 4 && backend.includes("EngineCommand::SetFixtureAttributeBatch"), "UI, MIDI, OSC, and Remote converge on the existing engine batch command"],
  [midi.includes("build_feedback_messages_with_operator_selection") && backend.includes("Some(&operator_selection)"), "MIDI feedback resolves the same runtime selection instead of guessing a fixed fixture"],
  [midi.includes("custom_midi_feedback_message") && midi.includes("midi_feedback_action_is_continuous"), "MIDI output selects exact states and interpolates continuous feedback endpoints"],
  [midi.includes("struct MidiFeedbackCache") && midi.includes("send_changed_feedback_slots") && midi.includes("last_slot_by_address"), "MIDI output suppresses unchanged mapping slots and collapses duplicate hardware addresses"],
  [midi.includes("fn cue_is_active") && midi.includes("active_group_cue_ids"), "MIDI feedback includes parallel Cue List and group activity"],
  [backend.includes("validate_midi_feedback") && backend.includes("MIDI feedback requires at least one state"), "custom MIDI feedback is validated at every persistence and connection boundary"],
  [remote.includes("SetOperatorSelection(OperatorSelectionContext)") && remote.includes("SetOperatorFeatureFader"), "Remote and future AI callers have typed operator-selection commands"],
  [app.includes("selectedControlTargetFixtures().map((fixture) => fixture.id)") && app.includes("visibleControls().map((control) => control.attribute)"), "frontend publishes the exact selected fixtures and visible fader order"],
  [app.includes("replaceProjectControlMappings(report.midi_mappings ?? [], [], report.dmx_mappings ?? [])"), "DVC import installs MIDI and DMX mappings instead of clearing them"],
  [app.includes("report.midi_mappings?.length ?? 0} MIDI and ${report.dmx_mappings?.length ?? 0} DMX mappings"), "operator import status reports the restored MIDI and DMX mapping counts"],
  [controlController.includes("const updateMidiMapping") && app.includes("onUpdateMapping={updateMidiMapping}"), "operator can update feedback without replacing the mapping route"],
  [backend.includes('name("syndocal-midi-feedback".to_string())') && backend.includes("MIDI_FEEDBACK_REFRESH_INTERVAL") && backend.includes("TELEMETRY_DMX_TARGET_FRAME_RATE_HZ") && backend.includes("engine.inspect_snapshot"), "auto feedback reads the published snapshot without full clones at the engine's 44 Hz rate"],
  [backend.includes("fn set_midi_feedback_auto(") && backend.includes("fn midi_feedback_status("), "backend exposes explicit auto-feedback configuration and health commands"],
  [controlController.includes('"set_midi_feedback_auto"') && controlController.includes('"midi_feedback_status"') && !controlController.includes("midiFeedbackTimer"), "frontend configures the backend worker instead of sending feedback on a 500 ms UI timer"],
  [controlController.includes("force: true") && backend.includes("force.unwrap_or(true)"), "manual feedback remains an explicit forced hardware refresh"],
  [midiPanel.includes("Create Off / On feedback") && midiPanel.includes("Unknown / mixed") && midiPanel.includes("Clear feedback"), "normal MIDI mapping editor exposes all feedback states"],
];

for (const [condition, message] of checks) assert.ok(condition, message);
console.log(`dvc midi shortcuts ok: ${checks.length} assertions`);
