import type {
  FrontendTauriInvoke,
  FrontendTauriInvokeCommand,
} from "./tauriInvokeCommands";

/**
 * The MCP bridge may execute only this reviewed, finite projection of the
 * backend control-plane registry. The source inventory remains discovery-only
 * unless a command is added here and reviewed with its typed Tauri handler.
 */
export const CANONICAL_TAURI_COMMANDS = {
  "syndocal.query.control_plane.registry.v1": "get_control_plane_operation_registry",
  "syndocal.query.control_plane.canonical_registry.v3": "get_control_plane_canonical_registry",
  "syndocal.query.control_plane.schemas.v1": "get_control_plane_query_schema_catalog",
  "syndocal.query.control_plane.capabilities.v1": "get_control_plane_query_capabilities",
  "syndocal.query.project.authority.v1": "query_control_plane_project_authority",
  "syndocal.query.runtime.generations.v1": "query_control_plane_runtime_generations",
  "syndocal.query.runtime.timeline.transport.authority.v1": "query_timeline_transport_authority_v1",
  "syndocal.query.runtime.timeline.loop.authority.v1": "query_timeline_loop_runtime_authority_v1",
  "syndocal.query.runtime.timeline.follow.abort.authority.v1": "query_timeline_follow_abort_authority_v1",
  "syndocal.query.output.control.authority.v1": "query_output_control_authority_v1",
  "syndocal.query.output.dsf2026_artnet_acceptance_probe.status.v1": "query_dsf2026_artnet_acceptance_probe_status_v1",
  "syndocal.query.output.display.add.authority.v1": "query_display_add_lease_authority_v1",
  "syndocal.query.output.ownership.v1": "query_control_plane_output_ownership",
  "syndocal.query.video.display_monitors.v1": "list_video_display_monitors",
  "syndocal.query.video.camera_profiles.v1": "list_video_camera_profiles",
  "syndocal.query.video.camera_profile_probe.v1": "probe_video_camera_profile",
  "syndocal.query.video.output_window_observation.v1": "get_video_output_window_observation_v1",
  "syndocal.query.events.observations.v1": "poll_control_plane_observation_events",
  "syndocal.effects.set_enabled.v1": "set_effect_enabled",
  "syndocal.runtime.timeline.transport.set_playing.v1": "set_timeline_transport_playing_runtime_v1",
  "syndocal.runtime.timeline.loop.commit.v1": "commit_timeline_loop_runtime_v1",
  "syndocal.runtime.timeline.follow.abort.v1": "abort_timeline_follow_runtime_v1",
  "syndocal.safety.blackout.engage.v1": "safety_blackout_engage_v1",
  "syndocal.output.blackout.release.v2": "release_blackout_output_control_v2",
  "syndocal.output.blackout.set.v2": "set_blackout_output_control_v2",
  "syndocal.output.ownership.arm.v2": "arm_output_control_v2",
  "syndocal.output.standby.takeover.v2": "take_over_output_control_v2",
  "syndocal.output.display.add.v2": "add_display_output_v2",
  "syndocal.output.display.window.set_open.v2": "set_display_output_window_open_v2",
  "syndocal.output.video.composition.assign.v2": "assign_video_output_composition_v2",
  "syndocal.output.show_artnet_loopback_route.enable.v1": "enable_show_art_net_loopback_route_v1",
  "syndocal.output.show_serial_dmx_s0_route.enable.v1": "enable_show_serial_dmx_safety_blackout_route_v1",
  "syndocal.output.show_serial_dmx_s0_route.stop.v1": "stop_show_serial_dmx_safety_blackout_route_v1",
  "syndocal.output.show_spout_outputs.enable.v2": "enable_show_spout_outputs_v2",
  "syndocal.output.show_spout_outputs.reset.v1": "reset_show_spout_outputs_v1",
  "syndocal.output.dsf2026_artnet_acceptance_probe.send.v1": "send_dsf2026_artnet_acceptance_probe_v1",
  "syndocal.output.dsf2026_artnet_acceptance_probe.reconcile.v1": "acknowledge_dsf2026_artnet_acceptance_probe_in_doubt_v1",
  "syndocal.output.enable.v2": "enable_output_control_v2",
  "syndocal.output.lease.acquire.v2": "acquire_output_lease_v2",
  "syndocal.output.lease.renew.v2": "renew_output_lease_v2",
  "syndocal.output.lease.recover.v2": "recover_output_lease_v2",
  "syndocal.output.lease.relinquish.v2": "relinquish_output_lease_v2",
  "syndocal.output.lease.force_transfer.v2": "force_transfer_output_lease_v2",
  "syndocal.cue_lists.reorder.v1": "reorder_cue_lists",
  "syndocal.cue_lists.rename.v1": "rename_cue_list",
  "syndocal.cue_lists.delete.v1": "delete_cue_list",
  "syndocal.scenes.create.v1": "create_scene_authoritative_v1",
} as const;

const CANONICAL_QUERY_OPERATION_IDS = new Set(
  Object.keys(CANONICAL_TAURI_COMMANDS).filter((operationId) => operationId.startsWith("syndocal.query.")),
);

export const canonicalOperationIsMutation = (operationId: string) =>
  !CANONICAL_QUERY_OPERATION_IDS.has(operationId);

export async function executeAgentBridgeCanonicalOperation(
  invoke: FrontendTauriInvoke,
  params: Record<string, unknown>,
  onMutationStarted?: () => void,
) {
  const operationId = params.operationId;
  const request = params.request;
  if (typeof operationId !== "string" || operationId.length === 0 || operationId.length > 512
    || request === null || typeof request !== "object" || Array.isArray(request)) {
    throw new Error("Canonical operation id and typed request object are required.");
  }
  const command = CANONICAL_TAURI_COMMANDS[
    operationId as keyof typeof CANONICAL_TAURI_COMMANDS
  ];
  if (!command) throw new Error("Canonical operation is not executable through the reviewed adapter set.");
  if (canonicalOperationIsMutation(operationId)) onMutationStarted?.();
  // The generated frontend invoke tuple tracks commands used directly by the
  // UI. These canonical adapters are a second, reviewed invocation surface;
  // the finite map above is the authority for this bridge projection.
  const result = await invoke(command as FrontendTauriInvokeCommand, request as Record<string, unknown>);
  return { ok: true, operation_id: operationId, result };
}

const MAX_CANONICAL_OPERATIONS = 128;
const MAX_SOURCE_INVENTORY = 2048;
const MAX_ADAPTERS_PER_OPERATION = 16;

type RecordValue = Record<string, unknown>;

const isRecord = (value: unknown): value is RecordValue =>
  value !== null && typeof value === "object" && !Array.isArray(value);

const requiredString = (value: unknown, name: string, maxLength = 1024): string => {
  if (typeof value !== "string" || value.length === 0 || value.length > maxLength) {
    throw new Error(`Control-plane registry field is invalid: ${name}`);
  }
  return value;
};

const requiredSchema = (value: unknown, name: string): RecordValue => {
  if (!isRecord(value)) throw new Error(`Control-plane registry schema is invalid: ${name}`);
  const version = value.version;
  if (typeof version !== "number" || !Number.isSafeInteger(version) || version < 0 || version > 65_535) {
    throw new Error(`Control-plane registry schema version is invalid: ${name}`);
  }
  return {
    name: requiredString(value.name, `${name}.name`, 768),
    version,
  };
};

const requiredArray = (value: unknown, name: string, maxLength: number): unknown[] => {
  if (!Array.isArray(value) || value.length > maxLength) {
    throw new Error(`Control-plane registry array is invalid: ${name}`);
  }
  return value;
};

const enumString = (value: unknown, name: string): string => requiredString(value, name, 128);

const compactAdapter = (value: unknown, index: number): RecordValue => {
  if (!isRecord(value) || !isRecord(value.source_key)) {
    throw new Error(`Control-plane registry adapter is invalid: ${index}`);
  }
  return {
    adapter: enumString(value.adapter, `derived_adapters[${index}].adapter`),
    binding_id: requiredString(value.binding_id, `derived_adapters[${index}].binding_id`, 576),
    source_key: {
      family: enumString(value.source_key.family, `derived_adapters[${index}].source_key.family`),
      source_id: requiredString(value.source_key.source_id, `derived_adapters[${index}].source_key.source_id`, 512),
    },
  };
};

const compactOperation = (value: unknown, index: number): RecordValue => {
  if (!isRecord(value)) throw new Error(`Control-plane registry operation is invalid: ${index}`);
  const capabilities = requiredArray(value.capabilities, `canonical_operations[${index}].capabilities`, 32)
    .map((capability, capabilityIndex) => enumString(capability, `capabilities[${capabilityIndex}]`));
  const adapters = requiredArray(value.derived_adapters, `canonical_operations[${index}].derived_adapters`, MAX_ADAPTERS_PER_OPERATION)
    .map(compactAdapter);
  return {
    operation_id: requiredString(value.operation_id, `canonical_operations[${index}].operation_id`, 512),
    class: enumString(value.class, `canonical_operations[${index}].class`),
    risk: enumString(value.risk, `canonical_operations[${index}].risk`),
    capabilities,
    request_schema: requiredSchema(value.request_schema, `canonical_operations[${index}].request_schema`),
    response_schema: requiredSchema(value.response_schema, `canonical_operations[${index}].response_schema`),
    idempotency: enumString(value.idempotency, `canonical_operations[${index}].idempotency`),
    audit: enumString(value.audit, `canonical_operations[${index}].audit`),
    adapter_policy: enumString(value.adapter_policy, `canonical_operations[${index}].adapter_policy`),
    receipt_policy: enumString(value.receipt_policy, `canonical_operations[${index}].receipt_policy`),
    rate_policy: enumString(value.rate_policy, `canonical_operations[${index}].rate_policy`),
    payload_policy: enumString(value.payload_policy, `canonical_operations[${index}].payload_policy`),
    consent_policy: enumString(value.consent_policy, `canonical_operations[${index}].consent_policy`),
    derived_adapters: adapters,
  };
};

/**
 * Read the backend-owned registry through the trusted renderer and return a
 * bounded projection. The MCP sidecar receives metadata and exact adapter
 * policy, never an unbounded source dump or an implicit dynamic invoke path.
 */
export async function executeAgentBridgeControlPlane(invoke: FrontendTauriInvoke) {
  const raw = await invoke<unknown>("get_control_plane_canonical_registry");
  if (!isRecord(raw)) throw new Error("Control-plane registry response is invalid.");
  const operations = requiredArray(raw.canonical_operations, "canonical_operations", MAX_CANONICAL_OPERATIONS)
    .map(compactOperation);
  const sourceInventory = requiredArray(raw.source_inventory, "source_inventory", MAX_SOURCE_INVENTORY);
  const sourceFamilies: Record<string, number> = {};
  const dispositions: Record<string, number> = {};
  for (const [index, source] of sourceInventory.entries()) {
    if (!isRecord(source) || !isRecord(source.source_key) || !isRecord(source.disposition)) {
      throw new Error(`Control-plane source inventory entry is invalid: ${index}`);
    }
    const family = enumString(source.source_key.family, `source_inventory[${index}].source_key.family`);
    const disposition = enumString(source.disposition.kind, `source_inventory[${index}].disposition.kind`);
    sourceFamilies[family] = (sourceFamilies[family] ?? 0) + 1;
    dispositions[disposition] = (dispositions[disposition] ?? 0) + 1;
  }
  return {
    ok: true,
    control_plane: {
      schema: requiredSchema(raw.schema, "schema"),
      canonical_operation_count: operations.length,
      canonical_operations: operations,
      source_inventory_count: sourceInventory.length,
      source_inventory_by_family: sourceFamilies,
      source_inventory_by_disposition: dispositions,
      execution_boundary: "The 47 reviewed canonical operations are executable through static typed local-window adapters; FailClosed entries remain discovery-only.",
    },
    agent_bridge: {
      adapter: "local_window_mcp_sidecar",
      operations: [
        "fixtures.list",
        "fixtures.get",
        "fixtures.set_transform",
        "output.set_video_blackout",
        "request.status",
        "runtime.get",
        "control_plane.get_capabilities",
        "recording.get_status",
        "control_plane.execute",
      ],
    },
  };
}
