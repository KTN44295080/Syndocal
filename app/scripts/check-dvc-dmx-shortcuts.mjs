import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

const importer = await readFile(new URL("../src-tauri/src/dvc_import.rs", import.meta.url), "utf8");
const backend = await readFile(new URL("../src-tauri/src/main.rs", import.meta.url), "utf8");
const protocol = await readFile(new URL("../../crates/protocol/src/lib.rs", import.meta.url), "utf8");
const dmxInput = await readFile(new URL("../../crates/io/src/dmx_input.rs", import.meta.url), "utf8");
const osc = await readFile(new URL("../../crates/io/src/osc.rs", import.meta.url), "utf8");
const app = (await readFile(new URL("../src/App.tsx", import.meta.url), "utf8")).replace(/\r\n/g, "\n");
const dvcImportController = await readFile(new URL("../src/dvcImportController.ts", import.meta.url), "utf8");
const types = await readFile(new URL("../src/types.ts", import.meta.url), "utf8");
const panel = await readFile(new URL("../src/components/DmxInputPanel.tsx", import.meta.url), "utf8");
const workspace = await readFile(new URL("../src/components/WorkspaceChrome.tsx", import.meta.url), "utf8");
const learn = await readFile(new URL("../src/controlMappingLearn.ts", import.meta.url), "utf8");
const viewport = await readFile(new URL("./check-viewport-containment.mjs", import.meta.url), "utf8");

function sectionBetween(source, startMarker, endMarker) {
  const start = source.indexOf(startMarker);
  if (start < 0) return "";
  const end = source.indexOf(endMarker, start + startMarker.length);
  if (end < 0) return "";
  return source.slice(start, end);
}

const externalControlDispatcher = sectionBetween(
  backend,
  "type ExternalControlDispatchState<'a>",
  "fn start_osc_input(",
);
const dmxInputCommand = sectionBetween(
  backend,
  "fn start_dmx_input(",
  "fn learn_dmx_control(",
);
const dmxStatusRefresh = sectionBetween(
  app,
  "let dmxInputStatusRequestGeneration = 0;",
  "const startDmxInput = async () =>",
);
const dmxStartRoute = sectionBetween(
  app,
  "const startDmxInput = async () =>",
  "const stopDmxInput = async () =>",
);
const dmxStopRoute = sectionBetween(
  app,
  "const stopDmxInput = async () =>",
  "const sendArtRdmRequest =",
);
const dmxLearnRoute = sectionBetween(
  app,
  "const learnDmxControlForTargets = async",
  "const setControlLearnMode =",
);

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
  [externalControlDispatcher.includes("fn dispatch_external_control_event(")
    && /type ExternalControlDispatchState<'a>\s*=\s*\(\s*&'a Arc<AtomicU64>,\s*&'a Arc<AtomicBool>,\s*&'a Arc<ProjectExternalCommandAdmission>,\s*&'a Arc<AtomicBool>,\s*u64,\s*\);/.test(externalControlDispatcher)
    && /dispatch_state:\s*ExternalControlDispatchState<'_>/.test(externalControlDispatcher)
    && /let \(\s*callback_epoch,\s*project_transaction_active,\s*project_external_command_admission,\s*callback_installed,\s*captured_callback_epoch,\s*\) = dispatch_state;/.test(externalControlDispatcher),
  "the shared external-control dispatcher carries and names every callback epoch and external-admission fence"],
  [/send_engine_command_if_callback_epoch\(\s*engine,\s*callback_epoch,\s*project_transaction_active,\s*project_external_command_admission,\s*callback_installed,\s*captured_callback_epoch,\s*command,\s*\);/.test(externalControlDispatcher),
  "the shared external-control dispatcher applies every captured fence to its final engine send"],
  [/dispatch_external_control_event\(\s*&engine,\s*&operator_selection,\s*\(\s*&callback_epoch_for_worker,\s*&project_transaction_active_for_worker,\s*&project_external_command_admission_for_worker,\s*&callback_installed_for_worker,\s*captured_callback_epoch,\s*\),\s*event,\s*"DMX",\s*\);/.test(dmxInputCommand),
  "DMX controls pass the complete captured fence state into the shared backend command dispatcher"],
  [backend.includes("if merge_enabled {") && backend.includes("} else {\n                let previous = previous_control_frames"), "raw merge and control mapping modes are mutually exclusive"],
  [backend.includes("control_events_from_changed_frame") && backend.includes("previous_control_frames"), "streaming DMX dispatch is edge-aware"],
  [backend.includes("fn learn_dmx_control") && backend.includes("learn_dmx_control,"), "DMX Learn is exposed through the desktop backend"],
  [dmxInput.includes("previous.is_none_or") && dmxInput.includes("unchanged_streaming_dmx_does_not_retrigger_control_actions"), "unchanged streaming input cannot retrigger discrete controls"],
  [dmxInput.includes("artnet_packet_drives_a_mapped_control_once_until_the_channel_changes"), "real UDP Art-Net input reaches a mapping exactly once per change"],
  [dmxInput.includes("fn control_mapping_universe") && dmxInput.includes("dmx_control_universe_normalizes_artnet_and_sacn_numbering"), "Art-Net and sACN wire universes share the persisted zero-based mapping contract"],
  [dmxInput.includes("crate::osc::event_from_control_value") && osc.includes("pub(crate) fn event_from_control_value"), "DMX and OSC share value-to-action semantics"],
  [dmxInput.includes("fn update_dmx_learning") && dmxInput.includes("max_by_key"), "DMX Learn selects the strongest changed channel after a baseline"],
  [backend.includes("dmx_mappings: Vec<DmxControlMapping>") && backend.includes("project_control_mappings_roundtrip_and_legacy_absence_stays_empty"), "project persistence and legacy absence are regression tested"],
  [backend.includes("project_backup_preserves_control_mappings") && backend.includes("user_templates_round_trip_shared_mappings_without_rewriting_authored_outputs"), "backup and template persistence retain DMX mappings without rewriting authored output state"],
  [app.includes("dmx: dmxMappings()")
    && app.includes("projectControlMappingsSignature(prepared.midi, prepared.osc, prepared.dmx, prepared.dj)")
    && app.includes("dmxMappings: sentMappings.dmx"),
  "dirty tracking and the authority-fenced persistence boundary include DMX mappings"],
  [backend.includes("let mappings = project_control_mappings_from_daslight_import_report(&report);")
    && dvcImportController.includes("applyLoadedProjectResult(imported.load, null)")
    && app.includes("if (prepared.dmx.length > 0)")
    && app.includes("setDmxInputConfig((current) => ({ ...current, merge_enabled: false }))"),
  "the paired DVC project result hydrates DMX mappings and activates control semantics without raw merge"],
  [learn.includes("dmxMappingsFromLearnedControl") && learn.includes("sameDmxSource"), "visual Learn creates mappings and replaces a conflicting DMX source"],
  [workspace.includes('data-control-learn-toggle="dmx"')
    && app.includes('invoke<LearnedDmxControl | null>("learn_dmx_control", {')
    && app.includes("expectedEpoch: authority.project_epoch"),
  "global DMX Learn uses the same direct visual workflow as MIDI and OSC under the captured project authority"],
  [app.includes('await invoke("stop_dmx_input", { expectedEpoch: authority.project_epoch })')
    && /await invoke\("start_dmx_input", \{\s*config: controlConfig,\s*mappings: dmxMappings\(\),\s*expectedEpoch: authority\.project_epoch,\s*\}\);/.test(app),
  "learned mappings are persisted and applied to the active input under one captured project authority"],
  [/const\s+requestGeneration\s*=\s*\+\+dmxInputStatusRequestGeneration/.test(dmxStatusRefresh)
    && /if\s*\(dmxInputStatusDisposed\s*\|\|\s*requestGeneration\s*!==\s*dmxInputStatusRequestGeneration\)\s*return;/.test(dmxStatusRefresh)
    && /if\s*\(!dmxInputStatusDisposed\s*&&\s*requestGeneration\s*===\s*dmxInputStatusRequestGeneration\)\s*\{/.test(dmxStatusRefresh),
  "DMX input status polling rejects stale success and failure responses"],
  [app.includes("dmxInputStatusDisposed = true;") && app.includes("dmxInputStatusRequestGeneration += 1;"),
  "DMX input status polling retires an in-flight response on App cleanup"],
  [/catch\s*\(error\)[\s\S]*?if\s*\(!isProjectAuthorityIdentityCurrent\(authority\)\)\s*return;[\s\S]*?setDmxInputStatus/.test(dmxStartRoute),
  "DMX input Start failure cannot write after project authority replacement"],
  [/catch\s*\(error\)[\s\S]*?if\s*\(!isProjectAuthorityIdentityCurrent\(authority\)\)\s*return;[\s\S]*?setMessage/.test(dmxStopRoute),
  "DMX input Stop failure cannot write after project authority replacement"],
  [/if\s*\(!authorityIsCurrent\(\)\)\s*return;\s*setDmxMappings\(nextMappings\);/.test(dmxLearnRoute),
  "DMX Learn rejects a stale authority before publishing learned mappings"],
  [dmxLearnRoute.includes("if (!isProjectAuthorityIdentityCurrent(authority)) return;\n        setMessage(`DMX Learn could not start input")
    && dmxLearnRoute.includes("if (!isProjectAuthorityIdentityCurrent(authority)) return;\n      setMessage(`DMX Learn failed"),
  "DMX Learn failure paths reject stale project errors before messaging"],
  [panel.includes('data-io-control="dmx-input-use"') && panel.includes('value="control">Control mappings'), "Setup I/O exposes an explicit Control mappings mode"],
  [panel.includes("props.mappings.map") && panel.includes("onRemoveMapping(index)"), "operators can inspect and remove imported or learned mappings"],
  [viewport.includes("command === 'learn_dmx_control'") && viewport.includes("dmxSelected.controlLearnMockCalls"), "real browser pointer flow covers DMX Learn"],
  [viewport.includes("call.args?.config?.merge_enabled === false") && viewport.includes("mapping.action === 'TriggerCue'"), "browser regression fixes the learned action and control-only mode"],
];

for (const [condition, message] of checks) assert.ok(condition, message);
console.log(`dvc dmx shortcuts ok: ${checks.length} assertions`);
