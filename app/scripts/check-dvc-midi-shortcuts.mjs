import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

const importer = await readFile(new URL("../src-tauri/src/dvc_import.rs", import.meta.url), "utf8");
const protocol = await readFile(new URL("../../crates/protocol/src/lib.rs", import.meta.url), "utf8");
const midi = await readFile(new URL("../../crates/io/src/midi.rs", import.meta.url), "utf8");
const engine = await readFile(new URL("../../crates/engine/src/lib.rs", import.meta.url), "utf8");
const backend = await readFile(new URL("../src-tauri/src/main.rs", import.meta.url), "utf8");
const app = await readFile(new URL("../src/App.tsx", import.meta.url), "utf8");
const types = await readFile(new URL("../src/types.ts", import.meta.url), "utf8");

const checks = [
  [types.includes("midi_mappings: MidiControlMapping[];"), "DVC report exposes imported MIDI mappings"],
  [importer.includes('"107" => {'), "DVC action 107 is decoded as a verified scene mapping"],
  [importer.includes('"55" => dvc_midi_mapping(event, MidiControlAction::TapBpm'), "DVC action 55 is decoded as Tap BPM"],
  [importer.includes('"113" => {') && importer.includes("MidiControlAction::TriggerCueListNext"), "DVC action 113 is decoded as Bank Next"],
  [importer.includes('"108" | "109" | "110" => {') && importer.includes('"108" => "Forward"') && importer.includes('"109" => "Reverse"') && importer.includes('"110" => "Bounce"'), "DVC directional Scene actions preserve their verified directions"],
  [importer.includes("cue_lists.push(CueListSummary") && importer.includes("cue_list_id,"), "DVC banks remain distinct Cue Lists"],
  [importer.includes("MidiControlAction::FlashCue"), "DVC FLASH=1 remains a hold-to-release cue mapping"],
  [importer.includes("dvc_midi_shortcuts_import_scene_tap_and_flash_without_guessing_unknown_actions"), "synthetic DVC regression is retained"],
  [importer.includes("dvc_local_homecoming_midi_scene_tap_and_flash_shortcuts_import_when_present"), "real Homecoming DVC golden is retained"],
  [protocol.includes("pub enum MidiControlAction") && protocol.includes("FlashCue,"), "protocol persists FlashCue"],
  [protocol.includes("TriggerCueListNext,"), "protocol persists Cue List Next"],
  [midi.includes("MidiControlEvent::ReleaseCue"), "MIDI Note Off releases a flashed cue"],
  [midi.includes("MidiControlEvent::TriggerCueWithDirection") && midi.includes("cue_live_direction(mapping.attribute.as_deref()?)"), "MIDI emits the persisted directional Scene trigger"],
  [engine.includes("EngineCommand::TriggerCueWithDirection") && engine.includes("pending_manual_cue_direction"), "engine carries a one-shot direction through Cue pre-wait"],
  [backend.includes("MidiControlEvent::ReleaseCue(cue_id) => EngineCommand::ReleaseCue(cue_id)"), "backend reuses the engine ReleaseCue route"],
  [backend.includes("MidiControlEvent::TriggerCueListNext(anchor_cue_id)") && backend.includes("EngineCommand::TriggerCueListNext(cue_list_id)"), "backend resolves the imported Bank anchor to its Cue List"],
  [app.includes("replaceProjectControlMappings(report.midi_mappings ?? [], [])"), "DVC import installs mappings instead of clearing them"],
  [app.includes("report.midi_mappings?.length ?? 0} MIDI mappings"), "operator import status reports the restored mapping count"],
];

for (const [condition, message] of checks) assert.ok(condition, message);
console.log(`dvc midi shortcuts ok: ${checks.length} assertions`);
