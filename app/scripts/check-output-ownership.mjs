import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

const read = (path) =>
  readFile(new URL(path, import.meta.url), "utf8").then((source) =>
    source.replace(/\r\n?/g, "\n"),
  );
const panel = await read("../src/components/StandbySyncPanel.tsx");
const videoPanel = await read("../src/components/VideoControlOutputsPanel.tsx");
const app = await read("../src/App.tsx");
const types = await read("../src/types.ts");
const localization = await read("../src/uiLocalization.ts");
const appBackend = await read("../../app/src-tauri/src/main.rs");
const ndiTransport = await read("../../app/src-tauri/src/ndi_transport.rs");
const ndiIo = await read("../../crates/io/src/ndi.rs");
const spoutTransport = await read("../../app/src-tauri/src/spout_transport.rs");
const engine = await read("../../crates/engine/src/lib.rs");
const video = await read("../../crates/video/src/lib.rs");
const protocol = await read("../../crates/protocol/src/lib.rs");
const qa = await read("../../qa/SYNDOCAL_2PC_TRANCHE1_OUTPUT_OWNERSHIP.md");

const indexOfOrFail = (source, marker, label, fromIndex = 0) => {
  const index = source.indexOf(marker, fromIndex);
  assert(index >= 0, `${label}: missing marker ${marker}`);
  return index;
};

const lastIndexOfOrFail = (source, marker, label) => {
  const index = source.lastIndexOf(marker);
  assert(index >= 0, `${label}: missing marker ${marker}`);
  return index;
};

const sliceBetween = (source, start, end, label, fromIndex = 0) => {
  const startIndex = indexOfOrFail(source, start, label, fromIndex);
  const endIndex = indexOfOrFail(source, end, label, startIndex + start.length);
  return source.slice(startIndex, endIndex);
};

const assertOrdered = (source, before, after, message) => {
  const beforeIndex = indexOfOrFail(source, before, message);
  const afterIndex = indexOfOrFail(source, after, message);
  assert(beforeIndex < afterIndex, `${message}: ${before} must precede ${after}`);
};

const assertOrderedMarkers = (source, markers, message) => {
  const indices = markers.map((marker) => indexOfOrFail(source, marker, message));
  for (let index = 1; index < indices.length; index += 1) {
    assert(
      indices[index - 1] < indices[index],
      `${message}: ${markers[index - 1]} must precede ${markers[index]}`,
    );
  }
  return indices;
};

const tauriCommandSegments = (source) => {
  const matches = [...source.matchAll(/#\[tauri::command\]\s*(?:async\s+)?fn\s+([A-Za-z0-9_]+)\s*\(/g)];
  return matches.map((match) => {
    // rustfmt leaves a top-level function close at column zero. Limiting the
    // segment there avoids attributing helper code between adjacent Tauri
    // commands to the preceding command.
    const end = source.indexOf("\n}\n", match.index);
    assert(end >= 0, `${match[1]}: missing rustfmt top-level function close`);
    return { name: match[1], source: source.slice(match.index, end + 3) };
  });
};

const protocolProjectSwapConstructor = sliceBetween(
  protocol,
  "pub fn project_swap_disarmed(",
  "pub fn failed(",
  "protocol project-swap disarmed constructor",
);
assertOrderedMarkers(
  protocolProjectSwapConstructor,
  [
    "role: MachineOutputRole::Standby",
    "effective_role: MachineOutputRole::Standby",
    "desired_role,",
    "persisted_role,",
    "state: OutputOwnershipState::Ready",
    "lighting_allowed: false",
    "video_allowed: false",
    "lighting_reason: OutputOwnershipReason::ProjectSwapDisarmed",
    "video_reason: OutputOwnershipReason::ProjectSwapDisarmed",
    "error: None",
  ],
  "protocol project-swap disarmed constructor",
);

const ownershipReasonUnion = sliceBetween(
  types,
  "export type OutputOwnershipReason =",
  "export type OutputOwnershipState =",
  "frontend output-ownership reason union",
);
indexOfOrFail(
  ownershipReasonUnion,
  '"ProjectSwapDisarmed"',
  "frontend output-ownership reason union",
);

const engineProjectSwapGate = sliceBetween(
  engine,
  "fn complete_project_swap_disarmed(",
  "fn fail_transition(",
  "engine project-swap disarmed gate completion",
);
assertOrderedMarkers(
  engineProjectSwapGate,
  [
    "if !state.transition_active",
    "if !matches!(",
    "if state.in_flight != 0",
    "OutputOwnershipStatus::project_swap_disarmed(",
    "state.transition_active = false",
    "self.inner.changed.notify_all()",
  ],
  "engine project-swap disarmed gate completion",
);

const engineProjectSwapTransition = sliceBetween(
  engine,
  "pub fn complete_project_swap_disarmed(",
  "pub fn fail(",
  "engine project-swap disarmed transition completion",
);
assertOrderedMarkers(
  engineProjectSwapTransition,
  [
    "self.gate.complete_project_swap_disarmed(",
    "self.finished = true",
  ],
  "engine project-swap disarmed transition completion",
);

const projectSwapTransitionTest = sliceBetween(
  engine,
  "fn project_swap_disarmed_preserves_desired_role_for_explicit_rearm()",
  "fn fail_closed_engine_start_emits_no_artnet_before_initialization()",
  "engine project-swap disarmed transition test",
);
assertOrderedMarkers(
  projectSwapTransitionTest,
  [
    "let status = transition.complete_project_swap_disarmed().unwrap();",
    "assert!(gate.acquire(OutputCapability::Lighting).is_err());",
    "assert!(gate.acquire(OutputCapability::Video).is_err());",
    "let rearm = gate.begin_transition(status.desired_role).unwrap();",
    "let rearmed = rearm.complete().unwrap();",
  ],
  "engine project-swap disarmed transition test",
);

const appExternalTransportReason = sliceBetween(
  app,
  "const externalVideoOwnershipReasonLabel",
  "const externalVideoTransportSummary",
  "App external transport ownership reason",
);
assertOrderedMarkers(
  appExternalTransportReason,
  [
    'case "ProjectSwapDisarmed":',
    'return "Project changed — outputs disarmed";',
  ],
  "App external transport ownership reason",
);

const standbyOwnershipReason = sliceBetween(
  panel,
  "function ownershipReasonLabel",
  "function ownershipStateLabel",
  "Standby ownership presentation",
);
assertOrderedMarkers(
  standbyOwnershipReason,
  [
    'case "ProjectSwapDisarmed":',
    'return "Project changed — outputs disarmed";',
  ],
  "Standby ownership presentation",
);

const videoOwnershipReason = sliceBetween(
  videoPanel,
  "const videoOwnershipMessage",
  "interface VideoMasterControlsPanelProps",
  "Video output blocked reason",
);
assertOrderedMarkers(
  videoOwnershipReason,
  [
    'case "ProjectSwapDisarmed":',
    'return "Project changed — outputs disarmed";',
  ],
  "Video output blocked reason",
);
indexOfOrFail(
  localization,
  '"Project changed — outputs disarmed": "プロジェクト変更 — 出力停止中"',
  "ProjectSwapDisarmed Japanese localization",
);

const explicitArmPath = sliceBetween(
  appBackend,
  "fn arm_output_ownership_role(",
  "fn sync_external_video_transports(",
  "preserved desired-role arm behavior",
);
assertOrderedMarkers(
  explicitArmPath,
  [
    "let preferred_role = state.engine.output_ownership_status().desired_role;",
    "apply_output_ownership_role(&app, &state, preferred_role)",
  ],
  "preserved desired-role arm behavior",
);

assert.match(
  types,
  /export type MachineOutputRole = "Lighting" \| "Video" \| "Both" \| "Standby";/,
  "frontend must expose exactly the four machine output roles",
);
for (const role of ["Lighting", "Video", "Both", "Standby"]) {
  assert.match(panel, new RegExp(`<option value="${role}">${role}</option>`));
}
assert.match(panel, /data-io-control="machine-output-role"/);
assert.match(panel, /data-io-control="arm-machine-output-role"/);
assert.match(panel, /aria-label="Machine output role"/);
assert.match(panel, /aria-label="Arm selected machine output role"/);
assert.match(panel, /get_output_ownership_status/);
assert.match(panel, /set_output_ownership_role/);
assert.match(panel, /lighting_allowed/);
assert.match(panel, /video_allowed/);
assert.match(panel, /lighting_reason/);
assert.match(panel, /video_reason/);
assert.match(panel, /effective_role/);
assert.match(panel, /desired_role/);
assert.match(panel, /persisted_role/);
assert.match(panel, /state/);
assert.match(panel, /generation/);
assert.match(panel, /epoch/);
assert.match(panel, /status\(\)\.running && status\(\)\.role === "standby"/);

assert.match(appBackend, /fn get_output_ownership_status/);
assert.match(appBackend, /fn set_output_ownership_role/);
assert.match(appBackend, /apply_output_ownership_role/);
assert.match(appBackend, /initialize_output_ownership/);
assert.match(appBackend, /load_output_ownership_role_from_path/);
assert.match(appBackend, /resolve_output_ownership_role_from_path/);
assert.match(appBackend, /persist_output_ownership_role_to_path/);
const startupOwnership = sliceBetween(
  appBackend,
  "fn initialize_output_ownership",
  "fn apply_output_ownership_role",
  "output ownership startup path",
);
assert.match(startupOwnership, /load_or_initialize_output_ownership_preference/);
assert.match(startupOwnership, /publish_output_ownership_startup_preference/);
assert.doesNotMatch(
  startupOwnership,
  /apply_output_ownership_role\(/,
  "startup may remember a preferred role but must not auto-arm it",
);
assert.match(appBackend, /fn arm_output_ownership_role/);
assert.match(panel, /arm_output_ownership_role/);
const ownershipTransition = sliceBetween(
  appBackend,
  "fn apply_output_ownership_role_locked",
  "fn get_output_ownership_status",
  "output ownership durable transition path",
);
assertOrdered(
  ownershipTransition,
  "persist_output_ownership_role_to_path(&path, MachineOutputRole::Standby)",
  "begin_output_ownership_transition(role)",
  "the durable Standby marker must precede runtime transition publication",
);
assert.match(
  appBackend,
  /persist_output_ownership_target_after_preparation\(path, role/,
  "the target role must persist only after preparation succeeds",
);
const preparedTransition = sliceBetween(
  appBackend,
  "fn apply_output_ownership_transition_after_reservation",
  "fn reserve_standby_output_ownership_transition",
  "prepared output ownership transition path",
);
assertOrdered(
  preparedTransition,
  "let prepared",
  "persist_output_ownership_target_after_preparation",
  "target persistence must follow the preparation closure",
);
assert.match(
  appBackend,
  /mark_output_ownership_durable_standby_failure/,
  "transition failures must publish durable Standby status",
);
assert.match(
  appBackend,
  /reserve_standby_output_ownership_transition/,
  "Standby startup must reserve the ownership fence before publication",
);
const standbyStart = sliceBetween(
  appBackend,
  "fn start_standby_sync",
  "fn stop_standby_sync(state",
  "Standby sync start path",
);
assert.match(standbyStart, /running: role != StandbySyncRole::Standby/);
assert.match(standbyStart, /standby_sync_may_publish_running/);
assertOrdered(
  standbyStart,
  "reserve_standby_output_ownership_transition",
  "status.running = true",
  "Standby running=true must publish only after the ownership transition",
);
assert.match(appBackend, /begin_output_ownership_transition/);
assert.match(appBackend, /fence_output_ownership/);
assert.match(appBackend, /prepare_output_ownership_role/);
assert.match(appBackend, /NATIVE_VIDEO_OUTPUT_WINDOW_PREFIX/);
assert.match(appBackend, /retire_native_video_output_windows\(/);
assert.match(appBackend, /native_video_output_window_labels_from/);
assert.match(appBackend, /External video output blocked by machine output role/);
assert.match(appBackend, /ownership_allowed: ownership\.video_allowed/);
assert.match(appBackend, /ownership_state: ownership\.state/);
assert.match(appBackend, /ownership_reason: ownership\.video_reason/);
assert.match(videoPanel, /outputOwnershipBlocked/);
assert.match(videoPanel, /videoOwnershipMessage/);
assert.match(appBackend, /struct ExternalVideoTransportStatusResponse/);
assert.match(appBackend, /ownership_allowed: ownership\.video_allowed/);
assert.match(types, /ownership_state: OutputOwnershipState;/);
assert.match(ndiTransport, /acquire_video_output/);
assert.match(spoutTransport, /acquire_video_output/);
assert.doesNotMatch(ndiTransport, /output_ownership_status\(\)\.video_allowed/);
assert.doesNotMatch(spoutTransport, /output_ownership_status\(\)\.video_allowed/);
const recordingStart = sliceBetween(
  appBackend,
  "fn start_video_output_recording",
  "fn recording_audio_inputs",
  "local video recording start path",
);
assert.doesNotMatch(recordingStart, /ensure_video_output_allowed/);
assert.match(engine, /SetOutputOwnershipRole/);
assert.match(engine, /OutputOwnershipGate/);
assert.match(engine, /state\.transition_active = false/);
assert.match(engine, /state\.transition_active = true/);
assert.match(engine, /OutputOwnershipActivation/);
assert.match(engine, /begin_activation/);
assert.match(engine, /validate_output_activation/);
assert.match(engine, /OutputCapability/);
assert.match(engine, /FenceOutputOwnership/);
assert.match(engine, /PrepareOutputOwnershipRole/);
assert.match(engine, /send_output_frame_with_recovery\([\s\S]*lighting_allowed/);
assert.match(engine, /output_ownership_gate_timeout_keeps_fence_until_permit_drains/);
assert.match(video, /sync_routes_with_driver_and_role/);
for (const command of [
  "send_art_rdm_request",
  "send_usb_rdm_request",
  "discover_usb_rdm_devices",
  "discover_art_rdm_devices",
  "start_art_rdm_full_discovery",
]) {
  const commandSource = sliceBetween(
    appBackend,
    "async fn " + command,
    "\n}",
    `${command} command path`,
  );
  assert.match(
    commandSource,
    /spawn_blocking\(move \|\| \{[\s\S]*let _lighting_permit = lighting_permit/,
    command + " must keep its lighting permit inside the blocking hardware worker",
  );
}
assert.match(ndiTransport, /begin_close_with_lease/);
assert.match(ndiTransport, /pending_output_teardowns/);
assert.match(ndiTransport, /begin_output_ownership_teardown/);
assert.match(ndiTransport, /begin_output_ownership_failure_fence/);
assert.match(ndiTransport, /failed_output_routes/);
assert.match(ndiTransport, /failure_snapshot/);
assert.match(ndiTransport, /failure_lease = Some/);
assert.match(ndiTransport, /JoinHandle<Result<\(\), NdiOutputWorkerStopError>>/);
assert.match(ndiTransport, /awaiting teardown acknowledgement/);
const ndiWorkerConstruction = sliceBetween(
  ndiTransport,
  "fn spawn_ndi_output_worker",
  "fn retire_ndi_output_after_creation",
  "NDI output worker spawn helper",
);
assertOrdered(
  ndiWorkerConstruction,
  "std::thread::Builder::new()",
  ".spawn(worker)",
  "NDI output worker spawn helper must create the thread through its worker argument",
);
const ndiWorkerStart = sliceBetween(
  ndiTransport,
  "#[cfg(feature = \"ndi\")]\nimpl NdiOutputWorker {\n    fn start(",
  "    fn publish(&mut self)",
  "NDI output worker start path",
);
const ndiWorkerClosure = sliceBetween(
  ndiWorkerStart,
  "let worker = spawn_ndi_output_worker(output_id, move || {",
  "\n        })\n        .map_err",
  "NDI spawned output worker closure",
);
const ndiOutputProduction = sliceBetween(
  ndiTransport,
  "#[cfg(feature = \"ndi\")]\nimpl NdiOutputWorker {\n",
  "#[cfg(test)]",
  "NDI output production lifecycle",
);
for (const marker of [
  "activation.admit_resource_creation()",
  "lease.publish()",
  "creation_lease.retire()",
  "NdiOutputStartDecision::Retire(lease)",
]) {
  indexOfOrFail(ndiOutputProduction, marker, "NDI output production lifecycle");
}
const ndiAdmitIndex = indexOfOrFail(
  ndiWorkerClosure,
  "activation.admit_resource_creation()",
  "NDI spawned output worker closure",
);
const ndiStartConstructorIndex = indexOfOrFail(
  ndiWorkerClosure,
  "io::ndi::NdiOutput::new",
  "NDI spawned output worker closure",
);
const ndiStartRetireIndex = indexOfOrFail(
  ndiWorkerClosure,
  "creation_lease.retire()",
  "NDI spawned output worker closure",
);
assert(ndiAdmitIndex < ndiStartConstructorIndex, "NDI admission must precede construction");
assert(
  ndiStartConstructorIndex < ndiStartRetireIndex,
  "NDI constructor failure must retire its creation lease",
);
assertOrdered(
  ndiWorkerClosure,
  "spawn_ndi_output_worker(output_id, move || {",
  "activation.admit_resource_creation()",
  "NDI sender constructor must execute inside the spawned worker closure",
);
assert.match(ndiWorkerStart, /startup_receiver\.recv_timeout/);
assert.match(ndiWorkerStart, /begin_output_ownership_failure_fence/);
const ndiPublishPath = sliceBetween(
  ndiTransport,
  "    fn publish(&mut self)",
  "    fn failure_snapshot",
  "NDI output publication path",
);
assert.match(ndiPublishPath, /lease\.publish\(\)/);
assert.match(ndiPublishPath, /self\.creation_lease = Some\(lease\)/);
const ndiRetirementPath = sliceBetween(
  ndiTransport,
  "    fn signal_retirement(&mut self)",
  "    fn stop(mut self,",
  "NDI output retirement path",
);
assert.match(ndiRetirementPath, /NdiOutputStartDecision::Retire\(lease\)/);
const ndiCreationRetirement = sliceBetween(
  ndiTransport,
  "fn retire_ndi_output_after_creation",
  "#[cfg(feature = \"ndi\")]\nimpl NdiOutputWorker",
  "NDI creation retirement helper",
);
assert.match(ndiCreationRetirement, /begin_close_with_lease\(lease\)/);
const ndiFailureFenceIndex = indexOfOrFail(
  ndiWorkerStart,
  "begin_output_ownership_failure_fence",
  "NDI output worker failure path",
);
const ndiTeardownIndex = indexOfOrFail(
  ndiWorkerStart,
  "begin_close_with_lease",
  "NDI output worker failure path",
);
assert(
  ndiFailureFenceIndex < ndiTeardownIndex,
  "NDI worker failure must fence ownership before sender teardown",
);
assert.match(ndiWorkerStart, /worker_failure/);
assert.match(ndiTransport, /injected_worker_spawn_failure_constructs_no_sender/);
assert.match(ndiTransport, /failure_fence_before_ndi_creation_admission_never_constructs_sender/);
assert.match(engine, /output_worker_failure_fence_blocks_rearm_until_teardown_ack/);
assert.match(ndiIo, /lease_released/);
assert.match(ndiIo, /drop\(lease\)/);
assert.match(ndiIo, /injected_slow_cleanup_remains_retryable_until_acknowledged/);

const spoutOutputStart = sliceBetween(
  spoutTransport,
  "    fn start_output(",
  "    fn publish(&mut self)",
  "Spout output worker start path",
);
const spoutAdmitIndex = indexOfOrFail(
  spoutOutputStart,
  "activation.admit_resource_creation()",
  "Spout output worker start path",
);
const spoutConstructorIndex = indexOfOrFail(
  spoutOutputStart,
  "spout2::dx::Sender::new",
  "Spout output worker start path",
);
const spoutRetireIndex = indexOfOrFail(
  spoutOutputStart,
  "creation_lease.retire()",
  "Spout output worker start path",
);
assert(spoutAdmitIndex < spoutConstructorIndex, "Spout admission must precede construction");
assert(
  spoutConstructorIndex < spoutRetireIndex,
  "Spout constructor failure must retire its creation lease",
);
const spoutPublishPath = sliceBetween(
  spoutTransport,
  "    fn publish(&mut self)",
  "    fn failure_snapshot",
  "Spout output publication path",
);
assert.match(spoutPublishPath, /lease\.publish\(\)/);
assert.match(spoutPublishPath, /self\.creation_lease = Some\(lease\)/);
const spoutRetirementPath = sliceBetween(
  spoutTransport,
  "    fn signal_retirement(&mut self)",
  "fn wait_until_ready(",
  "Spout output retirement path",
);
assert.match(spoutRetirementPath, /SpoutOutputStartDecision::Retire\(lease\)/);
const spoutOutputSource = sliceBetween(
  spoutTransport,
  "struct SpoutRouteWorker",
  "#[cfg(test)]\nmod tests",
  "Spout output worker ownership path",
);
for (const marker of [
  "activation.admit_resource_creation()",
  "lease.publish()",
  "creation_lease.retire()",
  "SpoutOutputStartDecision::Retire(lease)",
]) {
  indexOfOrFail(spoutOutputSource, marker, "Spout output production lifecycle");
}
const spoutJoinHandleIndex = indexOfOrFail(
  spoutOutputSource,
  "JoinHandle<Result<(), SpoutOutputWorkerStopError>>",
  "Spout output worker ownership path",
);
const spoutTeardownLeaseIndex = indexOfOrFail(
  spoutOutputSource,
  "teardown_lease: Arc<Mutex<Option<OutputOwnershipTeardownLease>>>",
  "Spout output worker ownership path",
);
const spoutFailureFenceIndex = indexOfOrFail(
  spoutOutputSource,
  "begin_output_ownership_failure_fence",
  "Spout output worker ownership path",
);
const spoutFinishPath = sliceBetween(
  spoutTransport,
  "fn finish_spout_output_worker",
  "fn run_spout_output_worker",
  "Spout output teardown path",
);
const spoutFinishFenceIndex = indexOfOrFail(
  spoutFinishPath,
  "begin_output_ownership_failure_fence",
  "Spout output teardown path",
);
const spoutDropSenderIndex = indexOfOrFail(
  spoutFinishPath,
  "drop(sender)",
  "Spout output teardown path",
);
const spoutStoreTeardownIndex = indexOfOrFail(
  spoutFinishPath,
  "*slot = Some(teardown_lease)",
  "Spout output teardown path",
);
assert(
  spoutFinishFenceIndex < spoutDropSenderIndex &&
    spoutDropSenderIndex < spoutStoreTeardownIndex,
  "Spout failure fence must precede sender teardown and lease retention",
);
const spoutStopPath = sliceBetween(
  spoutTransport,
  "    fn stop(mut self)",
  "    fn signal_retirement",
  "Spout output worker stop path",
);
const spoutStopJoinIndex = indexOfOrFail(
  spoutStopPath,
  "worker.join()",
  "Spout output worker stop path",
);
const spoutReleaseTeardownIndex = indexOfOrFail(
  spoutStopPath,
  "self.release_teardown_lease()",
  "Spout output worker stop path",
);
assert(
  spoutStopJoinIndex < spoutReleaseTeardownIndex,
  "Spout worker join must acknowledge teardown before lease release",
);
assert.match(spoutTransport, /failure_fence_before_spout_creation_admission_never_constructs_sender/);
assert.match(spoutTransport, /injected_spout_render_failure_fences_until_sender_teardown_acknowledged/);
assert.match(spoutTransport, /injected_spout_send_failure_fences_while_frame_permit_is_held/);

const displayCreationHelper = sliceBetween(
  appBackend,
  "fn with_output_resource_creation_lease",
  "\nfn stop_video_output_recording_runtime",
  "Display output creation helper",
);
for (const marker of [
  "activation.admit_resource_creation()?",
  "match lease.publish()",
  "let retire_result = retire(&resource)",
  "lease.retire()",
]) {
  indexOfOrFail(displayCreationHelper, marker, "Display output production lifecycle");
}
const displayAdmitIndex = indexOfOrFail(
  displayCreationHelper,
  "let lease = activation.admit_resource_creation()?",
  "Display output creation helper",
);
const displayCreateIndex = indexOfOrFail(
  displayCreationHelper,
  "let resource = match create()",
  "Display output creation helper",
);
const displayPublishIndex = indexOfOrFail(
  displayCreationHelper,
  "match lease.publish()",
  "Display output creation helper",
);
const displayRetireResourceIndex = indexOfOrFail(
  displayCreationHelper,
  "let retire_result = retire(&resource)",
  "Display output creation helper",
);
const displayRetireIndex = lastIndexOfOrFail(
  displayCreationHelper,
  "lease.retire()",
  "Display output creation helper",
);
assert(displayAdmitIndex < displayCreateIndex, "Display admission must precede native creation");
assert(displayCreateIndex < displayPublishIndex, "Display creation must precede publication");
assert(
  displayPublishIndex < displayRetireResourceIndex &&
    displayRetireResourceIndex < displayRetireIndex,
  "Display invalidation must retire the resource before releasing its creation lease",
);
const displayWindowOpen = sliceBetween(
  appBackend,
  "async fn open_video_output_window",
  "fn video_output_window_label",
  "Display output window path",
);
const displayLeaseUseIndex = indexOfOrFail(
  displayWindowOpen,
  "with_output_resource_creation_lease_with_cleanup(",
  "Display output window path",
);
const displayBuilderIndex = indexOfOrFail(
  displayWindowOpen,
  "builder.build()",
  "Display output window path",
);
assert(displayLeaseUseIndex < displayBuilderIndex, "Display builder must be inside the lease helper");

const engineCreationFenceTest = sliceBetween(
  engine,
  "fn output_resource_creation_admission_rechecks_activation_before_constructor",
  "fn output_resource_creation_lease_blocks_retry_until_resource_retirement",
  "engine creation-fence race test",
);
assert.match(engineCreationFenceTest, /begin_failure_fence/);
assert.match(engineCreationFenceTest, /let mut constructor_attempts = 0/);
assert.match(engineCreationFenceTest, /assert_eq!\(constructor_attempts, 0\)/);
assertOrdered(
  engineCreationFenceTest,
  "begin_failure_fence",
  "admit_resource_creation",
  "engine fence-wins test",
);
const engineCreationLeaseTest = sliceBetween(
  engine,
  "fn output_resource_creation_lease_blocks_retry_until_resource_retirement",
  "fn output_resource_creation_lease_rejects_stale_epoch_and_role",
  "engine creation-lease race test",
);
assert.match(engineCreationLeaseTest, /creation[\s\S]*\.publish\(\)/);
assert.match(engineCreationLeaseTest, /expect_err/);
assert.match(engineCreationLeaseTest, /creation\.retire\(\)/);
assertOrdered(
  engineCreationLeaseTest,
  "drop(failure)",
  "creation.retire()",
  "engine creation-lease retirement acknowledgement test",
);
assert.match(appBackend, /fenced_resource_creation_rejects_ndi_spout_and_display_constructor_seams/);
assert.doesNotMatch(
  ndiTransport,
  /begin_output_ownership_transition\(transition_target\)/,
  "NDI route stop must not complete an independent maintenance transition",
);
assert.match(appBackend, /admit_output_activation/);
assert.match(appBackend, /validate_external_video_activation/);
assert.match(appBackend, /finish_external_video_transport_maintenance_transition/);
const serializedSync = sliceBetween(
  appBackend,
  "fn sync_external_video_transports(",
  "fn get_video_runtime_status",
  "serialized external video sync path",
);
assertOrdered(
  serializedSync,
  "reject_legacy_output_control_route::<ExternalVideoTransportSyncResponse>",
  "begin_output_ownership_transition",
  "legacy Tauri external video sync must fail closed before transition admission",
);
assert.match(serializedSync, /begin_output_ownership_transition/);
assert.match(serializedSync, /fence_output_ownership/);
assert.match(serializedSync, /Some\(&mut transition\)/);
assert.match(video, /sync_routes_with_driver_and_role_with_start_admission/);
const routeSync = sliceBetween(
  video,
  "pub fn sync_routes_with_driver_and_role_with_start_admission",
  "pub fn clear(&mut self)",
  "external video route synchronization path",
);
assertOrderedMarkers(
  routeSync,
  ["for route in &stopped", "admit_start(driver)", "driver.start_route(route)"],
  "external resource admission must be reacquired between route stop and start",
);
assert.match(routeSync, /if stop_failed\.is_empty\(\)/);
assert.match(qa, /outer sync owns the stop-to-start fence and re-admits the current epoch/);
assert.match(qa, /worker send\/render failure fences ownership before sender teardown/i);
assert.match(qa, /worker spawn failure creates no sender before the worker exists/i);
assert.match(qa, /A creation lease is admitted after activation validation/);
assert.match(qa, /Spout has no asynchronous close acknowledgement/);
const externalDriver = sliceBetween(
  appBackend,
  "impl video::ExternalVideoTransportDriver for AppExternalVideoTransportDriver",
  "fn external_video_transport_direction_label",
  "production external video driver",
);
const ndiDriverBranch = sliceBetween(
  externalDriver,
  'if route.backend_id == "ndi"',
  "#[cfg(all(feature = \"spout\"",
  "NDI external video driver branch",
);
assertOrdered(
  ndiDriverBranch,
  "validate_external_video_activation(route)?",
  "self.ndi.start_route(route",
  "NDI sender creation must follow activation admission",
);
const spoutDriverBranch = sliceBetween(
  externalDriver,
  'if route.backend_id == "spout"',
  "self.recording.start_route(route)",
  "Spout external video driver branch",
);
assertOrdered(
  spoutDriverBranch,
  "validate_external_video_activation(route)?",
  "self.spout.start_route(route",
  "Spout sender creation must follow activation admission",
);
const nativeWindowOpen = sliceBetween(
  appBackend,
  "async fn open_video_output_window",
  "fn video_output_window_label",
  "native Display output window path",
);
assertOrdered(
  nativeWindowOpen,
  "admit_output_activation",
  "WindowBuilder",
  "native Display window creation/show must follow activation admission",
);
const remoteSyncFailClosed = sliceBetween(
  appBackend,
  "fn external_video_transport_sync_fail_closed()",
  "fn start_remote_control(",
  "remote external video synchronization fail-closed provider",
);
assert.match(remoteSyncFailClosed, /"report": null/);
assert.match(remoteSyncFailClosed, /"events": \[\]/);
assert.match(remoteSyncFailClosed, /OutputControl R4/);
assert.doesNotMatch(remoteSyncFailClosed, /sync_external_video_transports_from_snapshot/);
const remoteSyncAdmission = sliceBetween(
  appBackend,
  "fn start_remote_control(",
  "fn remote_control_status",
  "remote external video synchronization path",
);
assertOrdered(
  remoteSyncAdmission,
  "RemoteWsServer::start_with_snapshot_and_video_status_providers_and_dj_link",
  "external_video_transport_sync_fail_closed",
  "remote external sync must install the fail-closed provider",
);
assert.doesNotMatch(remoteSyncAdmission, /sync_external_video_transports_from_snapshot/);

// Generated legacy-ingress inventory.  The explicit list makes removals and
// additions review-visible; candidate discovery independently scans every
// Tauri command for an R4-classified EngineCommand or a native output sink so
// a new wrapper cannot silently bypass the list.
const legacyTauriOutputRoutes = [
  "add_video_output",
  "apply_video_output_mapping_preset",
  "close_open_video_output_windows",
  "close_video_output_window",
  "fade_video_output_opacity",
  "open_video_output_window",
  "remove_video_output",
  "send_dmx_routes_test_frame",
  "send_dmx_test_frame",
  "set_all_blackout",
  "set_blackout",
  "set_dmx_outputs",
  "set_group_submaster",
  "set_lighting_master",
  "set_output_config",
  "set_video_blackout",
  "set_video_master_opacity",
  "set_video_output_blackout",
  "set_video_output_config",
  "set_video_output_enabled",
  "set_video_output_mapping",
  "set_video_output_mapping_field",
  "set_video_output_opacity",
  "set_video_output_routing",
  "sync_external_video_transports",
  "sync_open_video_output_windows",
  "sync_video_output_window",
];
assert.deepEqual(
  legacyTauriOutputRoutes,
  [...legacyTauriOutputRoutes].sort(),
  "legacy Tauri output-route inventory must remain bytewise sorted",
);

const classifier = sliceBetween(
  appBackend,
  "fn external_output_command_requires_local_r4(",
  "fn external_control_source_requires_local_r4(",
  "external output-command classifier",
);
assert.doesNotMatch(classifier, /_\s*=>/, "external output classifier must remain exhaustive");
const forbiddenClassifierArm = classifier.slice(0, indexOfOrFail(
  classifier,
  'Some("legacy or output-affecting command; use OutputControl R4")',
  "external output-command classifier",
));
const forbiddenVariants = new Set(
  [...forbiddenClassifierArm.matchAll(/EngineCommand::([A-Za-z0-9_]+)/g)].map((match) => match[1]),
);
assert(forbiddenVariants.size >= 20, "R4 classifier unexpectedly lost output variants");

const commandSegments = tauriCommandSegments(appBackend);
const directNativeOutputNames = /^(?:sync_external_video_transports|sync_open_video_output_windows|close_open_video_output_windows|close_video_output_window|sync_video_output_window|open_video_output_window)$/;
const externalAdapterRoutes = [
  "connect_midi_control",
  "start_dmx_input",
  "start_osc_input",
  "start_remote_control",
];
const usesForbiddenVariant = (source) => [...forbiddenVariants].some((variant) =>
  new RegExp(`EngineCommand::${variant}(?:\\s|\\(|\\{)`).test(source));
const discoveredLegacyRoutes = commandSegments
  .filter(({ name, source }) => source.includes("reject_legacy_output_control_route")
    || directNativeOutputNames.test(name)
    || usesForbiddenVariant(source))
  .filter(({ name }) => !externalAdapterRoutes.includes(name))
  .map(({ name }) => name)
  .sort();
assert.deepEqual(
  discoveredLegacyRoutes,
  legacyTauriOutputRoutes,
  "generated legacy Tauri output-route inventory changed; classify and fail-close every new ingress",
);
const discoveredExternalAdapters = commandSegments
  .filter(({ source }) => usesForbiddenVariant(source))
  .map(({ name }) => name)
  .filter((name) => externalAdapterRoutes.includes(name))
  .sort();
assert.deepEqual(
  discoveredExternalAdapters,
  externalAdapterRoutes,
  "MIDI/OSC/DMX/Web Remote output-ingress inventory changed",
);

for (const route of legacyTauriOutputRoutes) {
  const matches = commandSegments.filter(({ name }) => name === route);
  assert.equal(matches.length, 1, `${route}: expected one Tauri command`);
  const body = matches[0].source;
  const rejectIndex = indexOfOrFail(
    body,
    "reject_legacy_output_control_route",
    `${route} fail-closed route`,
  );
  const sideEffectMarkers = [
    "state.engine",
    "begin_output_ownership_transition",
    "sync_external_video_transports_from_snapshot",
    "WindowBuilder",
    "admit_output_activation",
    "output_ownership_transition.lock",
  ];
  const sideEffectIndices = sideEffectMarkers
    .map((marker) => body.indexOf(marker))
    .filter((index) => index >= 0);
  if (sideEffectIndices.length > 0) {
    assert(
      rejectIndex < Math.min(...sideEffectIndices),
      `${route}: R4 rejection must precede every engine/native side effect`,
    );
  }
}

for (const route of externalAdapterRoutes) {
  const matches = commandSegments.filter(({ name }) => name === route);
  assert.equal(matches.length, 1, `${route}: expected one external adapter command`);
  const body = matches[0].source;
  assert(
    body.includes("send_engine_command_if_callback_epoch")
      || body.includes("external_output_command_requires_local_r4(&command)"),
  `${route}: every produced command must enter an exhaustive R4 classifier before engine send`,
  );
}

// Keep the physical-sink scan deliberately narrow and source-backed.  These
// markers are only applied to production Tauri-command segments from main.rs;
// strings in this checker and Rust tests are never scanned.  A new native
// sink therefore cannot hide behind a new command name without either being
// added to the explicit legacy/R4 inventory or failing this gate.
const nativePhysicalSinkMarkers = [
  "EngineCommand::AddVideoOutput(",
  "EngineCommand::ApplyVideoOutputMappingPreset {",
  "EngineCommand::Blackout(",
  "EngineCommand::ClearDmxInput(",
  "EngineCommand::FadeVideoOutputOpacity {",
  "EngineCommand::SafetyBlackoutEngagePublished {",
  "EngineCommand::SafetyBlackoutReleasePublished {",
  "EngineCommand::SetAllBlackout(",
  "EngineCommand::SetDmxInputFrame {",
  "EngineCommand::SetDmxOutputs(",
  "EngineCommand::SetGroupSubmaster {",
  "EngineCommand::SetLightingMaster(",
  "EngineCommand::SetOutput(",
  "EngineCommand::SetOutputOwnershipRole {",
  "EngineCommand::SetVideoBlackout(",
  "EngineCommand::SetVideoMasterOpacity(",
  "EngineCommand::SetVideoOutputBlackout {",
  "EngineCommand::SetVideoOutputConfig {",
  "EngineCommand::SetVideoOutputEnabled {",
  "EngineCommand::SetVideoOutputMapping {",
  "EngineCommand::SetVideoOutputMappingField {",
  "EngineCommand::SetVideoOutputOpacity {",
  "EngineCommand::SetVideoOutputRouting {",
  "admit_output_activation(",
  "apply_output_ownership_role(",
  "begin_output_ownership_transition(",
  "control_plane_runtime::engage_safety_blackout(",
  "engine.acquire_lighting_output(",
  "fence_output_ownership(",
  "ndi_transport",
  "retire_native_video_output_window(",
  "retire_native_video_output_windows(",
  "send_dmx_config_test_frame(",
  "send_dmx_route_test_frames(",
  "spout_transport",
  "state.output_ownership_transition.lock(",
  "sync_external_video_transports_from_snapshot(",
  "tauri::window::WindowBuilder::new(",
  "with_output_resource_creation_lease_with_cleanup(",
];
assert.deepEqual(
  nativePhysicalSinkMarkers,
  [...nativePhysicalSinkMarkers].sort(),
  "native physical sink marker inventory must remain bytewise sorted",
);

const canonicalR4TauriOutputRoutes = [
  "arm_output_control_v1",
  "arm_output_ownership_role",
  "force_transfer_output_lease_v1",
  "recover_output_lease_v1",
  "release_blackout_output_control_v1",
  "relinquish_output_lease_v1",
  "renew_output_lease_v1",
  "safety_blackout_engage_v1",
  "set_output_ownership_role",
  "start_standby_sync",
  "stop_standby_sync",
  "take_over_output_control_v1",
  "take_over_standby",
];
assert.deepEqual(
  canonicalR4TauriOutputRoutes,
  [...canonicalR4TauriOutputRoutes].sort(),
  "canonical R4/S0 output-route inventory must remain bytewise sorted",
);

const nativeSinkHits = (source, markers = nativePhysicalSinkMarkers) =>
  markers.filter((marker) => source.includes(marker));

const assertNativeSinkInventory = (source, {
  legacyRoutes,
  canonicalRoutes,
  protectedAdapterRoutes,
}) => {
  const segments = tauriCommandSegments(source);
  const allowed = new Set([
    ...legacyRoutes,
    ...canonicalRoutes,
  ]);
  const detected = segments
    .map(({ name, source: segment }) => ({
      name,
      // MIDI/OSC/DMX/Web Remote adapters are checked independently through
      // the exhaustive R4 classifier above.  Their EngineCommand values are
      // intent, not a native sink; a direct native marker in one of these
      // adapters remains visible and fails the exact ingress inventory.
      markers: nativeSinkHits(segment).filter((marker) =>
        !protectedAdapterRoutes.includes(name) || !marker.startsWith("EngineCommand::")),
    }))
    .filter(({ markers }) => markers.length > 0);
  for (const route of detected) {
    assert(
      allowed.has(route.name),
      `${route.name}: native physical sink is outside the exact legacy/R4 ingress inventory (${route.markers.join(", ")})`,
    );
  }

  const legacySet = new Set(legacyRoutes);
  for (const route of detected.filter(({ name }) => legacySet.has(name))) {
    const segment = segments.find(({ name }) => name === route.name).source;
    const rejectIndex = indexOfOrFail(
      segment,
      "reject_legacy_output_control_route",
      `${route.name} native sink rejection`,
    );
    for (const marker of route.markers) {
      const markerIndex = segment.indexOf(marker);
      assert(
        rejectIndex < markerIndex,
        `${route.name}: reject_legacy_output_control_route must precede ${marker}`,
      );
    }
  }

  const conditionalRoleRoutes = new Set([
    "arm_output_ownership_role",
    "set_output_ownership_role",
  ]);
  for (const route of detected.filter(({ name }) => conditionalRoleRoutes.has(name))) {
    const segment = segments.find(({ name }) => name === route.name).source;
    const guardIndex = indexOfOrFail(
      segment,
      "Active output arming is fail-closed",
      `${route.name} conditional R4 guard`,
    );
    const applyIndex = indexOfOrFail(
      segment,
      "apply_output_ownership_role(",
      `${route.name} conditional R4 guard`,
    );
    assert(
      guardIndex < applyIndex,
      `${route.name}: active role must be rejected before ownership transition`,
    );
  }

  const s0 = segments.find(({ name }) => name === "safety_blackout_engage_v1");
  assert(s0, "dedicated safety_blackout_engage_v1 must remain a Tauri command");
  assert.match(
    s0.source,
    /control_plane_runtime::engage_safety_blackout\(/,
    "dedicated S0 must use the target-less safety runtime",
  );
  assert.doesNotMatch(
    s0.source,
    /reject_legacy_output_control_route/,
    "dedicated S0 must not be misclassified as a legacy route",
  );

  return detected;
};

const nativeSinkDetected = assertNativeSinkInventory(appBackend, {
  legacyRoutes: legacyTauriOutputRoutes,
  canonicalRoutes: canonicalR4TauriOutputRoutes,
  protectedAdapterRoutes: externalAdapterRoutes,
});
assert(
  nativeSinkDetected.some(({ name }) => name === "open_video_output_window"),
  "native Display window ingress must be covered by the native-sink inventory",
);

// Negative fixtures prove that this is an executable detector rather than a
// documentation-only list.  They are intentionally temporary source strings;
// no fixture text is read by the production scan above.
assert.throws(
  () => assertNativeSinkInventory(appBackend, {
    legacyRoutes: legacyTauriOutputRoutes.filter((name) => name !== "set_output_config"),
    canonicalRoutes: canonicalR4TauriOutputRoutes,
    protectedAdapterRoutes: externalAdapterRoutes,
  }),
  /set_output_config|outside the exact legacy\/R4 ingress inventory/,
  "removing an ingress from the explicit inventory must fail",
);
const unknownNativeCommand = `${appBackend}\n\n#[tauri::command]\nfn future_video_output_sink(state: State<'_, AppState>) -> Result<(), String> {\n    state.engine.send(EngineCommand::SetVideoOutputEnabled { output_id: 1, enabled: true }).map_err(|error| error.to_string())\n}\n`;
assert.throws(
  () => assertNativeSinkInventory(unknownNativeCommand, {
    legacyRoutes: legacyTauriOutputRoutes,
    canonicalRoutes: canonicalR4TauriOutputRoutes,
    protectedAdapterRoutes: externalAdapterRoutes,
  }),
  /future_video_output_sink|outside the exact legacy\/R4 ingress inventory/,
  "a new command with a native sink must fail closed until classified",
);
const setOutputCommand = commandSegments.find(({ name }) => name === "set_output_config");
assert(setOutputCommand, "set_output_config fixture source must exist");
const movedRejectBody = setOutputCommand.source
  .replace('    reject_legacy_output_control_route::<()>("DMX output configuration")?;\n', "")
  .replace(
    "        .map_err(|error| error.to_string())\n}",
    '        .map_err(|error| error.to_string())\n    reject_legacy_output_control_route::<()>("DMX output configuration")?;\n}',
  );
assert.throws(
  () => assertNativeSinkInventory(
    appBackend.replace(setOutputCommand.source, movedRejectBody),
    {
      legacyRoutes: legacyTauriOutputRoutes,
      canonicalRoutes: canonicalR4TauriOutputRoutes,
      protectedAdapterRoutes: externalAdapterRoutes,
    },
  ),
  /set_output_config: reject_legacy_output_control_route must precede/,
  "moving rejection after a physical sink must fail closed",
);

const sharedExternalCallback = sliceBetween(
  appBackend,
  "fn send_engine_command_if_callback_epoch(",
  "fn callback_epoch_allows_send(",
  "MIDI/OSC shared external callback seam",
);
assertOrdered(
  sharedExternalCallback,
  "external_output_command_requires_local_r4(&command)",
  "engine.send(command)",
  "MIDI/OSC shared callback must classify before the engine queue",
);
const remoteExternalCallback = sliceBetween(
  appBackend,
  "RemoteInputEvent::SetVideoOutputBlackout",
  "move || snapshot_engine.snapshot()",
  "Web Remote external callback seam",
);
assertOrdered(
  remoteExternalCallback,
  "external_output_command_requires_local_r4(&command)",
  "command_engine.send(command)",
  "Web Remote callback must classify before the engine queue",
);
assert.match(
  appBackend,
  /fn safety_blackout_engage_v1\([\s\S]*?control_plane_runtime::engage_safety_blackout/,
  "dedicated target-less S0 path must remain separate from legacy callback routing",
);
assert.match(appBackend, /output_ownership_target_is_not_durable_until_preparation_succeeds/);
assert.match(appBackend, /standby_sync_running_publication_requires_completed_all_deny_status/);
assert.match(protocol, /pub enum MachineOutputRole/);
assert.match(protocol, /pub struct OutputOwnershipStatus/);
assert.match(protocol, /pub enum OutputOwnershipState/);
assert.match(protocol, /Activating/);
assert.match(protocol, /TransitionFailed/);
assert.doesNotMatch(protocol, /pub machine_output_role:/);
assert.doesNotMatch(protocol, /pub output_ownership:/);

console.log("output ownership static contract: PASS");
