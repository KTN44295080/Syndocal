import type { FrontendTauriInvoke } from "./tauriInvokeCommands";

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
      execution_boundary: "Only explicitly typed local-window adapters are executable; FailClosed entries remain discovery-only.",
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
      ],
    },
  };
}
