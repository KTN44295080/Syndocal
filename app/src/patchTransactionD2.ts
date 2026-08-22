import type {
  ProjectTransactionCommandResult,
} from "./types";

export type PatchRepairTransactionCommand = "patch_fixtures" | "repair_fixture_profile";

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
  return candidate.kind === "repair_fixture_profile"
    && hasExactOwnKeys(candidate, ["kind", "request_digest", "fixture_id"])
    && isCanonicalRequestDigest(candidate.request_digest)
    && typeof candidate.fixture_id === "number"
    && Number.isSafeInteger(candidate.fixture_id)
    && candidate.fixture_id > 0
    && candidate.fixture_id === args.fixtureId;
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
