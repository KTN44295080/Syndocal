import type {
  ProjectTransactionCommandResult,
  ProjectTransactionStageCommandResult,
} from "./types";

/**
 * Published-command routes sharing one retained receipt recovery query: the
 * PATCH/Repair pair plus every D4 Stage renderer-ticketed route.
 */
export type PatchRepairTransactionCommand =
  | "patch_fixtures"
  | "repair_fixture_profile"
  | StageRendererTicketedCommand;

/** The nine Stage routes ticketed on the same backend project transaction lane. */
export type StageRendererTicketedCommand =
  | "set_fixture_transform"
  | "set_stage_map_config"
  | "save_stage_map_preset"
  | "apply_stage_map_preset"
  | "remove_stage_map_preset"
  | "import_stage_map_preset"
  | "add_stage_object"
  | "set_stage_object"
  | "remove_stage_object";

const stageRendererTicketedCommands: readonly string[] = [
  "set_fixture_transform",
  "set_stage_map_config",
  "save_stage_map_preset",
  "apply_stage_map_preset",
  "remove_stage_map_preset",
  "import_stage_map_preset",
  "add_stage_object",
  "set_stage_object",
  "remove_stage_object",
];

export const isStageRendererTicketedCommand = (command: string): command is StageRendererTicketedCommand =>
  stageRendererTicketedCommands.includes(command);

type StageRoutePayloadFamily = "unit" | "stage_object_id" | "label";

const stageRoutePayloadFamily = (
  command: PatchRepairTransactionCommand,
): StageRoutePayloadFamily | null => {
  switch (command) {
    case "set_fixture_transform":
    case "set_stage_map_config":
    case "apply_stage_map_preset":
    case "remove_stage_map_preset":
    case "set_stage_object":
    case "remove_stage_object":
      return "unit";
    case "add_stage_object":
      return "stage_object_id";
    case "save_stage_map_preset":
    case "import_stage_map_preset":
      return "label";
    default:
      return null;
  }
};

export const fixturePatchIdsAreExact = (value: unknown, expectedCount: number): value is number[] =>
  Array.isArray(value)
  && Number.isSafeInteger(expectedCount)
  && expectedCount > 0
  && value.length === expectedCount
  && value.every((fixtureId) => Number.isSafeInteger(fixtureId) && fixtureId > 0)
  && new Set(value).size === value.length;

const hasExactOwnKeys = (value: object, keys: readonly string[]) => {
  const actual = Object.keys(value).sort();
  const expected = [...keys].sort();
  return actual.length === expected.length && actual.every((key, index) => key === expected[index]);
};

const isCanonicalRequestDigest = (value: unknown): value is string =>
  typeof value === "string" && /^[a-f0-9]{64}$/.test(value);

const expectedStagePresetLabel = (
  command: StageRendererTicketedCommand,
  args: Record<string, unknown>,
): string | null => {
  const rawLabel = command === "save_stage_map_preset"
    ? args.label
    : command === "import_stage_map_preset"
      && args.preset
      && typeof args.preset === "object"
      && !Array.isArray(args.preset)
      ? (args.preset as Record<string, unknown>).label
      : null;
  if (typeof rawLabel !== "string") return null;
  const normalized = rawLabel.trim();
  return normalized.length > 0 ? normalized : null;
};

export const projectTransactionCommandResultIsWellFormed = (
  value: unknown,
  command: PatchRepairTransactionCommand,
  args: Record<string, unknown>,
): value is ProjectTransactionCommandResult => {
  if (!value || typeof value !== "object" || Array.isArray(value)) return false;
  const candidate = value as Record<string, unknown>;
  if (command === "patch_fixtures") {
    const expectedCount = Array.isArray(args.requests) ? args.requests.length : 0;
    return candidate.kind === "patch_fixtures"
      && hasExactOwnKeys(candidate, ["kind", "request_digest", "fixture_ids"])
      && isCanonicalRequestDigest(candidate.request_digest)
      && fixturePatchIdsAreExact(candidate.fixture_ids, expectedCount);
  }
  if (command === "repair_fixture_profile") {
    return candidate.kind === "repair_fixture_profile"
      && hasExactOwnKeys(candidate, ["kind", "request_digest", "fixture_id"])
      && isCanonicalRequestDigest(candidate.request_digest)
      && typeof candidate.fixture_id === "number"
      && Number.isSafeInteger(candidate.fixture_id)
      && candidate.fixture_id > 0
      && candidate.fixture_id === args.fixtureId;
  }
  if (!isStageRendererTicketedCommand(command)) return false;
  // A Stage receipt mirrors Rust's serde shape exactly: exact own keys, a
  // stored command_name equal to the requested route, a canonical digest,
  // an Applied/Unchanged outcome, and the route's payload invariant. Any
  // future/extra field stays malformed so recovery holds instead of Cancel.
  if (candidate.kind !== "stage_project_mutation") return false;
  if (!hasExactOwnKeys(candidate, [
    "kind",
    "command_name",
    "request_digest",
    "outcome",
    "stage_object_id",
    "label",
  ])) {
    return false;
  }
  if (candidate.command_name !== command) return false;
  if (!isCanonicalRequestDigest(candidate.request_digest)) return false;
  if (candidate.outcome !== "applied" && candidate.outcome !== "unchanged") return false;
  switch (stageRoutePayloadFamily(command)) {
    case "unit":
      return candidate.stage_object_id === null && candidate.label === null;
    case "stage_object_id":
      return typeof candidate.stage_object_id === "number"
        && Number.isSafeInteger(candidate.stage_object_id)
        && candidate.stage_object_id > 0
        && candidate.label === null;
    case "label":
      const expectedLabel = expectedStagePresetLabel(command, args);
      return candidate.stage_object_id === null
        && typeof candidate.label === "string"
        && expectedLabel !== null
        && candidate.label === expectedLabel;
    default:
      return false;
  }
};

/**
 * Converts one recovered receipt back into the legacy reply shape its route's
 * callers consume: PATCH/Repair keep exposing their full durable receipt,
 * while Stage routes are narrowed to their pre-D4 shapes (`void` for unit
 * routes, the allocated object ID for `add_stage_object`, and the preset
 * label for save/import). Callers validate receipts first; a malformed
 * result still fails loudly here so a future caller cannot adopt a silent
 * `undefined` as a published legacy reply.
 */
export const publishedCommandLegacyReplyFromRecoveredResult = (
  command: PatchRepairTransactionCommand,
  result: ProjectTransactionCommandResult,
  args: Record<string, unknown>,
): unknown => {
  if (!isStageRendererTicketedCommand(command)) return result;
  if (!projectTransactionCommandResultIsWellFormed(result, command, args)) {
    throw new Error(`Recovered ${command} result is malformed.`);
  }
  const receipt = result as ProjectTransactionStageCommandResult;
  switch (stageRoutePayloadFamily(command)) {
    case "stage_object_id":
      return receipt.stage_object_id;
    case "label":
      return receipt.label;
    default:
      return null;
  }
};

export type PublishedCommandRecoveryDecision =
  | { kind: "published"; result: ProjectTransactionCommandResult }
  | { kind: "not_published" }
  | { kind: "indeterminate"; error: string }
  | { kind: "unconfirmed" };

export const publishedCommandRecoveryDisposition = (
  decision: PublishedCommandRecoveryDecision,
): "commit" | "cancel" | "hold" =>
  decision.kind === "published" ? "commit" : decision.kind === "not_published" ? "cancel" : "hold";

export const publishedCommandRecoveryBackoffMs = (attempt: number) =>
  Math.min(1_000, 25 * (2 ** Math.min(5, Math.max(0, attempt))));

/**
 * Retains the exact transaction identity while a command is genuinely still
 * in flight. The caller gets a terminal disposition only after publication or
 * a backend-confirmed non-publication; no renderer replay is possible here.
 */
export const waitForPublishedCommandRecovery = async (
  query: () => Promise<{
    status: string;
    command_result?: unknown;
    command_in_flight?: unknown;
    command_indeterminate_error?: unknown;
  } | null>,
  command: PatchRepairTransactionCommand,
  args: Record<string, unknown>,
  wait: (milliseconds: number) => Promise<void>,
): Promise<PublishedCommandRecoveryDecision> => {
  let attempt = 0;
  for (;;) {
    try {
      const decision = publishedCommandRecoveryDecision(await query(), command, args);
      if (decision.kind !== "unconfirmed") return decision;
    } catch {
      // A lost recovery reply is indistinguishable from a slow publication;
      // preserve the same ticket and retry instead of cancelling or replaying.
    }
    await wait(publishedCommandRecoveryBackoffMs(attempt));
    attempt += 1;
  }
};

/**
 * A pending receipt without a result is deliberately not a rejection: the
 * engine publication may still complete after the lost Tauri reply.
 */
export const publishedCommandRecoveryDecision = (
  recovery: {
    status: string;
    command_result?: unknown;
    command_in_flight?: unknown;
    command_indeterminate_error?: unknown;
  } | null,
  command: PatchRepairTransactionCommand,
  args: Record<string, unknown>,
): PublishedCommandRecoveryDecision => {
  if (!recovery) return { kind: "unconfirmed" };
  if (typeof recovery.command_indeterminate_error === "string"
    && recovery.command_indeterminate_error.trim() !== "") {
    return { kind: "indeterminate", error: recovery.command_indeterminate_error };
  }
  if (recovery.command_indeterminate_error !== undefined
    && recovery.command_indeterminate_error !== null) {
    return { kind: "indeterminate", error: "The command fault latch was malformed; restart is required before retrying." };
  }
  if (projectTransactionCommandResultIsWellFormed(recovery.command_result, command, args)) {
    return { kind: "published", result: recovery.command_result };
  }
  if (recovery.command_result !== undefined && recovery.command_result !== null) {
    // A non-null result with the wrong schema may represent a post-engine
    // publication. Do not reinterpret it as a safe Cancel.
    return { kind: "indeterminate", error: "The command receipt was malformed; restart is required before retrying." };
  }
  if (recovery.status === "cancelled") return { kind: "not_published" };
  // An acknowledged receipt deliberately has no payload. It cannot prove
  // that the command did not reach B, so never turn it into a Cancel.
  if (recovery.status === "acknowledged") return { kind: "unconfirmed" };
  if (recovery.status === "pending") {
    // A missing or malformed admission bit is not proof that the engine call
    // stopped. Keep the exact ticket open instead of manufacturing a Cancel.
    if (recovery.command_indeterminate_error !== null || typeof recovery.command_in_flight !== "boolean") {
      return { kind: "unconfirmed" };
    }
    // The backend records an ACK-disconnect as command_indeterminate_error
    // before this lane guard can become false. A null latch plus false is
    // therefore its definitive durable non-publication fence.
    return recovery.command_in_flight ? { kind: "unconfirmed" } : { kind: "not_published" };
  }
  return { kind: "unconfirmed" };
};

/** One UI lane covers all PATCH and profile Repair affordances. */
export function createPatchRepairSingleflight(onBusy: (busy: boolean) => void) {
  let active = false;
  return {
    begin: () => {
      if (active) return false;
      active = true;
      onBusy(true);
      return true;
    },
    finish: () => {
      if (!active) return;
      active = false;
      onBusy(false);
    },
    active: () => active,
  };
}
