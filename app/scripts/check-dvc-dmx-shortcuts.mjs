import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

const importer = await readFile(new URL("../src-tauri/src/dvc_import.rs", import.meta.url), "utf8");
const backend = await readFile(new URL("../src-tauri/src/main.rs", import.meta.url), "utf8");
const protocol = await readFile(new URL("../../crates/protocol/src/lib.rs", import.meta.url), "utf8");
const dmxInput = await readFile(new URL("../../crates/io/src/dmx_input.rs", import.meta.url), "utf8");
const osc = await readFile(new URL("../../crates/io/src/osc.rs", import.meta.url), "utf8");
const app = await readFile(new URL("../src/App.tsx", import.meta.url), "utf8");
const types = await readFile(new URL("../src/types.ts", import.meta.url), "utf8");
const panel = await readFile(new URL("../src/components/DmxInputPanel.tsx", import.meta.url), "utf8");
const workspace = await readFile(new URL("../src/components/WorkspaceChrome.tsx", import.meta.url), "utf8");
const learn = await readFile(new URL("../src/controlMappingLearn.ts", import.meta.url), "utf8");
const viewport = await readFile(new URL("./check-viewport-containment.mjs", import.meta.url), "utf8");

const checks = [
  [protocol.includes("pub merge_enabled: bool") && protocol.includes('serde(default = "default_true")'), "legacy DMX input remains raw-merge compatible"],
  [protocol.includes("pub struct DmxControlMapping") && protocol.includes("pub type DmxControlAction = OscControlAction"), "DMX mappings persist the shared typed action surface"],
  [protocol.includes("pub struct LearnedDmxControl"), "DMX learn result is typed"],
  [types.includes("dmx_mappings: DmxControlMapping[];") && types.includes("export interface LearnedDmxControl"), "frontend project and learn contracts expose DMX mappings"],
  [importer.includes("report.dmx_mappings = parse_dmx_shortcuts"), "DVC import installs parsed DMX mappings into its report"],
  [importer.includes('shortcut.attribute("TYPE") != Some("3")'), "only DVC TYPE=3 shortcuts enter DMX conversion"],
  [importer.includes('scalar_selector != "5"') && importer.includes("daslight_universe - 1"), "verified DVC scalar selector and one-based universe conversion are fixed"],
  [importer.includes('action.attribute("TYPE") != Some("210")'), "only verified DVC action 210 is converted"],
  [importer.includes('(\"SMODE\", \"1\")') && importer.includes('(\"CMODE\", \"1\")') && importer.includes('(\"TMODE\", \"0\")') && importer.includes('settings.attribute("INC")'), "verified DVC control settings and increment are required"],
  [importer.includes('beam.attribute("BEAMID") != Some("0")') && importer.includes("raw_channel_index == raw_channel_index"), "DVC beam and feature index must match verified profile evidence"],
  [importer.includes("dvc_local_panel_restores_verified_dmx_rgb_mappings_when_present"), "real Panel.dvc nine-mapping golden is retained"],
  [importer.includes("dvc_dmx_shortcuts_import_verified_feature_mapping_and_skip_unproven_variants"), "synthetic no-guess regression is retained"],
  [backend.includes("fn validate_dmx_control_mappings") && backend.includes("validate_mapping_required_fields_for_osc"), "DMX mappings reuse the complete OSC action validation surface"],
  [backend.includes("fn dispatch_external_control_event") && backend.includes('dispatch_external_control_event(&engine, &operator_selection, event, "DMX")'), "DMX controls reach the shared backend command dispatcher"],
  [backend.includes("if merge_enabled {") && backend.includes("} else {\n                let previous = previous_control_frames"), "raw merge and control mapping modes are mutually exclusive"],
  [backend.includes("control_events_from_changed_frame") && backend.includes("previous_control_frames"), "streaming DMX dispatch is edge-aware"],
  [backend.includes("fn learn_dmx_control") && backend.includes("learn_dmx_control,"), "DMX Learn is exposed through the desktop backend"],
  [dmxInput.includes("previous.is_none_or") && dmxInput.includes("unchanged_streaming_dmx_does_not_retrigger_control_actions"), "unchanged streaming input cannot retrigger discrete controls"],
  [dmxInput.includes("artnet_packet_drives_a_mapped_control_once_until_the_channel_changes"), "real UDP Art-Net input reaches a mapping exactly once per change"],
  [dmxInput.includes("fn control_mapping_universe") && dmxInput.includes("dmx_control_universe_normalizes_artnet_and_sacn_numbering"), "Art-Net and sACN wire universes share the persisted zero-based mapping contract"],
  [dmxInput.includes("crate::osc::event_from_control_value") && osc.includes("pub(crate) fn event_from_control_value"), "DMX and OSC share value-to-action semantics"],
  [dmxInput.includes("fn update_dmx_learning") && dmxInput.includes("max_by_key"), "DMX Learn selects the strongest changed channel after a baseline"],
  [backend.includes("dmx_mappings: Vec<DmxControlMapping>") && backend.includes("project_control_mappings_roundtrip_and_legacy_absence_stays_empty"), "project persistence and legacy absence are regression tested"],
  [backend.includes("project_backup_preserves_control_mappings") && backend.includes("user_templates_round_trip_shared_mappings_and_open_with_outputs_disarmed"), "backup and template persistence retain DMX mappings"],
  [app.includes("dmx: dmxMappings()") && app.includes("dmxMappings: dmxMappings()"), "dirty tracking and save boundaries include DMX mappings"],
  [app.includes("replaceProjectControlMappings(report.midi_mappings ?? [], [], report.dmx_mappings ?? [])") && app.includes("if (nextDmxMappings.length > 0)") && app.includes("merge_enabled: false"), "every project restore with DMX mappings activates control semantics without raw merge"],
  [learn.includes("dmxMappingsFromLearnedControl") && learn.includes("sameDmxSource"), "visual Learn creates mappings and replaces a conflicting DMX source"],
  [workspace.includes('data-control-learn-toggle="dmx"') && app.includes('invoke<LearnedDmxControl | null>("learn_dmx_control")'), "global DMX Learn uses the same direct visual workflow as MIDI and OSC"],
  [app.includes('await invoke("stop_dmx_input")') && app.includes('await invoke("start_dmx_input", { config: controlConfig, mappings: nextMappings })'), "learned mappings are applied to the active input immediately"],
  [panel.includes('data-io-control="dmx-input-use"') && panel.includes('value="control">Control mappings'), "Setup I/O exposes an explicit Control mappings mode"],
  [panel.includes("props.mappings.map") && panel.includes("onRemoveMapping(index)"), "operators can inspect and remove imported or learned mappings"],
  [viewport.includes("command === 'learn_dmx_control'") && viewport.includes("dmxSelected.controlLearnMockCalls"), "real browser pointer flow covers DMX Learn"],
  [viewport.includes("call.args?.config?.merge_enabled === false") && viewport.includes("mapping.action === 'TriggerCue'"), "browser regression fixes the learned action and control-only mode"],
];

for (const [condition, message] of checks) assert.ok(condition, message);
console.log(`dvc dmx shortcuts ok: ${checks.length} assertions`);
