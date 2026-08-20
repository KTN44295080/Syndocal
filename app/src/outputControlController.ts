import type { FrontendTauriInvoke } from "./tauriInvokeCommands";

export const OUTPUT_CONTROL_AUTHORITY_QUERY_OPERATION_ID =
  "syndocal.query.output.control.authority.v1";
export const OUTPUT_LEASE_AUTHORITY_QUERY_OPERATION_ID =
  "syndocal.output.lease.authority.query.v1";
export const OUTPUT_CONSENT_PREPARE_OPERATION_ID = "syndocal.output.consent.prepare.v1";
export const OUTPUT_CONSENT_STATUS_QUERY_OPERATION_ID =
  "syndocal.query.output.consent.status.v1";
export const OUTPUT_OWNERSHIP_ARM_OPERATION_ID = "syndocal.output.ownership.arm.v1";
export const OUTPUT_BLACKOUT_RELEASE_OPERATION_ID = "syndocal.output.blackout.release.v1";
export const OUTPUT_STANDBY_TAKEOVER_OPERATION_ID = "syndocal.output.standby.takeover.v1";
export const OUTPUT_LEASE_ACQUIRE_OPERATION_ID = "syndocal.output.lease.acquire.v1";
export const OUTPUT_LEASE_RENEW_OPERATION_ID = "syndocal.output.lease.renew.v1";
export const OUTPUT_LEASE_RECOVER_OPERATION_ID = "syndocal.output.lease.recover.v1";
export const OUTPUT_LEASE_RELINQUISH_OPERATION_ID = "syndocal.output.lease.relinquish.v1";
export const OUTPUT_LEASE_FORCE_TRANSFER_OPERATION_ID =
  "syndocal.output.lease.force_transfer.v1";

const MAX_LEASE_QUERY_STATUSES = 64;
const MAX_SAFE_REQUEST_ID = Number.MAX_SAFE_INTEGER;

export interface OutputControlFence {
  process_incarnation: number;
  session_incarnation: number;
  project_epoch: number;
  project_revision: number;
  project_checkpoint_hash: string;
  project_publication_generation: number;
  output_epoch: number;
  output_generation: number;
  safety_blackout_epoch: number;
  safety_blackout_generation: number;
}

export type OutputControlTargetRole = "lighting" | "video" | "both";
export type OutputLeaseResource = "lighting" | "video";
export type OutputLeaseResources = readonly OutputLeaseResource[];

export interface OutputLeaseAuthority {
  lease_id: string;
  generation: number;
}

export interface OutputLeaseAuthorityQueryUnavailable {
  status: "unavailable";
}

export interface OutputLeaseAuthorityQueryHeld {
  status: "held_active" | "held_orphaned";
  authority: OutputLeaseAuthority;
  resources: OutputLeaseResource[];
}

export type OutputLeaseAuthorityQueryStatus =
  | OutputLeaseAuthorityQueryUnavailable
  | OutputLeaseAuthorityQueryHeld;

export interface OutputLeaseAuthorityQuery {
  operation_id: typeof OUTPUT_LEASE_AUTHORITY_QUERY_OPERATION_ID;
  statuses: OutputLeaseAuthorityQueryStatus[];
}

export type OutputControlAction =
  | { kind: "arm"; role: OutputControlTargetRole; lease: OutputLeaseAuthority }
  | { kind: "release_blackout"; lease: OutputLeaseAuthority }
  | {
      kind: "take_over_standby";
      force: boolean;
      standby_session_id: string;
      standby_generation: number;
      lease: OutputLeaseAuthority;
    };

export type OutputLeaseLifecycleAction =
  | { kind: "acquire_lease"; role: OutputControlTargetRole }
  | { kind: "renew_lease"; lease: OutputLeaseAuthority }
  | { kind: "recover_lease"; lease: OutputLeaseAuthority }
  | { kind: "relinquish_output_lease"; lease: OutputLeaseAuthority }
  | { kind: "force_transfer_lease"; lease: OutputLeaseAuthority };

export type OutputControlOperationAction = OutputControlAction | OutputLeaseLifecycleAction;

export interface OutputControlChallengeNotice {
  action: OutputControlOperationAction;
  displayCode: string;
  expiresAtUnixMs: number;
}

export type OutputLeaseReceiptPhase = "unclaimed" | "held_active" | "held_orphaned";
export type OutputLeaseReceiptOutcome =
  | "acquired"
  | "renewed"
  | "expiry_observed"
  | "recovered"
  | "relinquished"
  | "transferred"
  | "owner_retired"
  | "project_orphaned"
  | "authorized";

export interface OutputLeaseReceiptChange {
  lease_id: string;
  before_generation: number | null;
  after_generation: number | null;
  before_resources: OutputLeaseResource[];
  after_resources: OutputLeaseResource[];
  before_phase: OutputLeaseReceiptPhase | null;
  after_phase: OutputLeaseReceiptPhase | null;
}

export interface OutputControlLeaseResult {
  authority: OutputLeaseAuthority;
  resources: OutputLeaseResource[];
  phase: OutputLeaseReceiptPhase;
  outcome: OutputLeaseReceiptOutcome;
  audit_sequence: number;
  changes: OutputLeaseReceiptChange[];
}

export interface OutputControlReceipt {
  operation_id: string;
  request_id: number;
  shape_sha256: string;
  argument_fingerprint: string;
  audit_sequence: number;
  fence_before: OutputControlFence;
  fence_after: OutputControlFence;
  outcome: "applied" | "no_op";
  lease_result: OutputControlLeaseResult;
}

let nextOutputControlRequestId = 1;

const allocateRequestId = (): number => {
  if (nextOutputControlRequestId <= 0 || nextOutputControlRequestId > MAX_SAFE_REQUEST_ID) {
    throw new Error(
      "OutputControl request identity is exhausted; refresh the renderer session before retrying.",
    );
  }
  const requestId = nextOutputControlRequestId;
  nextOutputControlRequestId = requestId === MAX_SAFE_REQUEST_ID
    ? MAX_SAFE_REQUEST_ID + 1
    : requestId + 1;
  return requestId;
};

const operationIdForAction = (action: OutputControlOperationAction): string => {
  switch (action.kind) {
    case "arm": return OUTPUT_OWNERSHIP_ARM_OPERATION_ID;
    case "release_blackout": return OUTPUT_BLACKOUT_RELEASE_OPERATION_ID;
    case "take_over_standby": return OUTPUT_STANDBY_TAKEOVER_OPERATION_ID;
    case "acquire_lease": return OUTPUT_LEASE_ACQUIRE_OPERATION_ID;
    case "renew_lease": return OUTPUT_LEASE_RENEW_OPERATION_ID;
    case "recover_lease": return OUTPUT_LEASE_RECOVER_OPERATION_ID;
    case "relinquish_output_lease": return OUTPUT_LEASE_RELINQUISH_OPERATION_ID;
    case "force_transfer_lease": return OUTPUT_LEASE_FORCE_TRANSFER_OPERATION_ID;
  }
};

const commandForAction = (action: OutputControlOperationAction): string => {
  switch (action.kind) {
    case "arm": return "arm_output_control_v1";
    case "release_blackout": return "release_blackout_output_control_v1";
    case "take_over_standby": return "take_over_output_control_v1";
    case "acquire_lease": return "acquire_output_lease_v1";
    case "renew_lease": return "renew_output_lease_v1";
    case "recover_lease": return "recover_output_lease_v1";
    case "relinquish_output_lease": return "relinquish_output_lease_v1";
    case "force_transfer_lease": return "force_transfer_output_lease_v1";
  }
};

const wait = (milliseconds: number): Promise<void> => new Promise((resolve) => {
  globalThis.setTimeout(resolve, milliseconds);
});

const isObject = (value: unknown): value is Record<string, unknown> =>
  Boolean(value) && typeof value === "object";
const isPositiveSafeInteger = (value: unknown): value is number =>
  typeof value === "number" && Number.isSafeInteger(value) && value > 0;
const isNonnegativeSafeInteger = (value: unknown): value is number =>
  typeof value === "number" && Number.isSafeInteger(value) && value >= 0;
const isLowerHexSha256 = (value: unknown): value is string =>
  typeof value === "string" && /^[0-9a-f]{64}$/.test(value);
const isLeaseId = (value: unknown): value is string =>
  typeof value === "string" && /^lease-[0-9a-f]{16}$/.test(value)
    && !/^lease-0{16}$/.test(value);
const isCanonicalChallengeToken = (value: unknown): value is string =>
  typeof value === "string" && /^[A-Za-z0-9_-]{22}$/.test(value);

const hasExactKeys = (value: Record<string, unknown>, expected: readonly string[]): boolean => {
  const actual = Object.keys(value).sort();
  const required = [...expected].sort();
  return actual.length === required.length && actual.every((key, index) => key === required[index]);
};

const outputControlFenceKeys = [
  "process_incarnation", "session_incarnation", "project_epoch", "project_revision",
  "project_checkpoint_hash", "project_publication_generation", "output_epoch",
  "output_generation", "safety_blackout_epoch", "safety_blackout_generation",
] as const;

const assertFence = (value: unknown, errorMessage: string): OutputControlFence => {
  if (!isObject(value) || !hasExactKeys(value, outputControlFenceKeys)
    || !isPositiveSafeInteger(value.process_incarnation)
    || !isPositiveSafeInteger(value.session_incarnation)
    || !isNonnegativeSafeInteger(value.project_epoch)
    || !isNonnegativeSafeInteger(value.project_revision)
    || !isLowerHexSha256(value.project_checkpoint_hash)
    || !isNonnegativeSafeInteger(value.project_publication_generation)
    || !isPositiveSafeInteger(value.output_epoch)
    || !isPositiveSafeInteger(value.output_generation)
    || !isPositiveSafeInteger(value.safety_blackout_epoch)
    || !isPositiveSafeInteger(value.safety_blackout_generation)) throw new Error(errorMessage);
  return value as unknown as OutputControlFence;
};

const fencesEqual = (left: OutputControlFence, right: OutputControlFence): boolean =>
  left.process_incarnation === right.process_incarnation
  && left.session_incarnation === right.session_incarnation
  && left.project_epoch === right.project_epoch
  && left.project_revision === right.project_revision
  && left.project_checkpoint_hash === right.project_checkpoint_hash
  && left.project_publication_generation === right.project_publication_generation
  && left.output_epoch === right.output_epoch
  && left.output_generation === right.output_generation
  && left.safety_blackout_epoch === right.safety_blackout_epoch
  && left.safety_blackout_generation === right.safety_blackout_generation;

const assertLeaseAuthority = (value: unknown, errorMessage: string): OutputLeaseAuthority => {
  if (!isObject(value) || !hasExactKeys(value, ["lease_id", "generation"])
    || !isLeaseId(value.lease_id) || !isPositiveSafeInteger(value.generation)) throw new Error(errorMessage);
  return value as unknown as OutputLeaseAuthority;
};

const assertResources = (value: unknown, errorMessage: string): OutputLeaseResource[] => {
  if (!Array.isArray(value) || value.length === 0 || value.length > 2
    || value.some((resource) => resource !== "lighting" && resource !== "video")
    || value.some((resource, index) => value.indexOf(resource) !== index)
    || value.length === 2 && (value[0] !== "lighting" || value[1] !== "video")) throw new Error(errorMessage);
  return [...value] as OutputLeaseResource[];
};

const sameResources = (left: readonly string[], right: readonly string[]): boolean =>
  left.length === right.length && left.every((resource, index) => resource === right[index]);

const resourcesForRole = (role: OutputControlTargetRole): OutputLeaseResource[] => {
  switch (role) {
    case "lighting": return ["lighting"];
    case "video": return ["video"];
    case "both": return ["lighting", "video"];
  }
};

const assertOutputLeaseAuthorityQuery = (value: unknown): OutputLeaseAuthorityQuery => {
  const errorMessage = "Output lease authority query was invalid; refresh lease state before retrying.";
  if (!isObject(value) || !hasExactKeys(value, ["operation_id", "statuses"])
    || value.operation_id !== OUTPUT_LEASE_AUTHORITY_QUERY_OPERATION_ID
    || !Array.isArray(value.statuses) || value.statuses.length === 0
    || value.statuses.length > MAX_LEASE_QUERY_STATUSES) throw new Error(errorMessage);
  const statuses = value.statuses.map((candidate): OutputLeaseAuthorityQueryStatus => {
    if (!isObject(candidate)) throw new Error(errorMessage);
    if (candidate.status === "unavailable") {
      if (!hasExactKeys(candidate, ["status"])) throw new Error(errorMessage);
      return { status: "unavailable" };
    }
    if ((candidate.status !== "held_active" && candidate.status !== "held_orphaned")
      || !hasExactKeys(candidate, ["status", "authority", "resources"])) throw new Error(errorMessage);
    return {
      status: candidate.status,
      authority: assertLeaseAuthority(candidate.authority, errorMessage),
      resources: assertResources(candidate.resources, errorMessage),
    };
  });
  if (statuses.some((status) => status.status === "unavailable")) {
    if (statuses.length !== 1) throw new Error(errorMessage);
  } else {
    for (let index = 1; index < statuses.length; index += 1) {
      const previous = statuses[index - 1];
      const current = statuses[index];
      if (previous.status === "unavailable" || current.status === "unavailable"
        || previous.authority.lease_id >= current.authority.lease_id) throw new Error(errorMessage);
    }
  }
  return { operation_id: OUTPUT_LEASE_AUTHORITY_QUERY_OPERATION_ID, statuses };
};

export async function queryOutputLeaseAuthority(
  invoke: FrontendTauriInvoke,
): Promise<OutputLeaseAuthorityQuery> {
  return assertOutputLeaseAuthorityQuery(await invoke<unknown>("query_output_lease_authority_v1"));
}

const activeStatuses = (query: OutputLeaseAuthorityQuery): OutputLeaseAuthorityQueryHeld[] =>
  query.statuses.filter((status): status is OutputLeaseAuthorityQueryHeld => status.status === "held_active");

export function selectOutputLeaseAuthority(
  query: OutputLeaseAuthorityQuery,
  leaseId: string | null,
  expectedResources?: OutputLeaseResources,
): OutputLeaseAuthority {
  if (query.statuses.length === 1 && query.statuses[0].status === "unavailable") {
    throw new Error("Output lease is unavailable; output was not applied. Refresh lease state.");
  }
  if (!leaseId || !isLeaseId(leaseId)) throw new Error("Select one exact active output lease before continuing; nothing was applied.");
  const matches = activeStatuses(query).filter((status) => status.authority.lease_id === leaseId);
  if (matches.length !== 1 || expectedResources && !sameResources(matches[0].resources, expectedResources)) {
    throw new Error("Selected output lease is stale, orphaned, or has the wrong resources; nothing was applied.");
  }
  return { ...matches[0].authority };
}

export function selectOnlyActiveOutputLease(
  query: OutputLeaseAuthorityQuery,
  expectedResources: OutputLeaseResources,
): OutputLeaseAuthority {
  const matches = activeStatuses(query).filter((status) => sameResources(status.resources, expectedResources));
  if (matches.length !== 1) throw new Error("Exactly one active output lease with the requested resources is required; nothing was applied.");
  return { ...matches[0].authority };
}

const assertAction = (action: OutputControlOperationAction): void => {
  const record = action as unknown as Record<string, unknown>;
  if (action.kind === "acquire_lease") {
    if (!hasExactKeys(record, ["kind", "role"]) || !["lighting", "video", "both"].includes(action.role)) {
      throw new Error("Output lease acquire action was invalid; nothing was applied.");
    }
    return;
  }
  if (action.kind === "arm") {
    if (!hasExactKeys(record, ["kind", "role", "lease"]) || !["lighting", "video", "both"].includes(action.role)) {
      throw new Error("OutputControl arm action was invalid; nothing was applied.");
    }
  } else if (action.kind === "release_blackout") {
    if (!hasExactKeys(record, ["kind", "lease"])) throw new Error("OutputControl release action was invalid; nothing was applied.");
  } else if (action.kind === "take_over_standby") {
    if (!hasExactKeys(record, ["kind", "force", "standby_session_id", "standby_generation", "lease"])
      || typeof action.force !== "boolean" || !action.standby_session_id
      || !isPositiveSafeInteger(action.standby_generation)) throw new Error("OutputControl Take Over action was invalid; nothing was applied.");
  } else if (!hasExactKeys(record, ["kind", "lease"])) {
    throw new Error("Output lease lifecycle action was invalid; nothing was applied.");
  }
  assertLeaseAuthority(action.lease, "Output lease authority was invalid; nothing was applied.");
};

const assertAuthority = (value: unknown): { operation_id: string; fence: OutputControlFence } => {
  if (!isObject(value) || !hasExactKeys(value, ["operation_id", "fence"])
    || value.operation_id !== OUTPUT_CONTROL_AUTHORITY_QUERY_OPERATION_ID) throw new Error("OutputControl authority response was invalid; nothing was applied.");
  return {
    operation_id: OUTPUT_CONTROL_AUTHORITY_QUERY_OPERATION_ID,
    fence: assertFence(value.fence, "OutputControl authority response was invalid; nothing was applied."),
  };
};

interface OutputConsentChallenge {
  operation_id: typeof OUTPUT_CONSENT_PREPARE_OPERATION_ID;
  request_id: number;
  target_operation_id: string;
  challenge_id: string;
  consent_token: string;
  display_code: string;
  argument_fingerprint: string;
  expires_at_unix_ms: number;
}

interface OutputConsentStatus {
  operation_id: typeof OUTPUT_CONSENT_STATUS_QUERY_OPERATION_ID;
  request_id: number;
  challenge_id: string;
  state: "pending_physical_input" | "ready";
  expires_at_unix_ms: number;
}

const OUTPUT_CONSENT_READY_HANDOFF_MS = 5_000;

const assertChallenge = (value: unknown, operationId: string, requestId: number): OutputConsentChallenge => {
  if (!isObject(value) || !hasExactKeys(value, [
    "operation_id", "request_id", "target_operation_id", "challenge_id", "consent_token",
    "display_code", "argument_fingerprint", "expires_at_unix_ms",
  ]) || value.operation_id !== OUTPUT_CONSENT_PREPARE_OPERATION_ID || value.request_id !== requestId
    || value.target_operation_id !== operationId || !isCanonicalChallengeToken(value.challenge_id)
    || !isCanonicalChallengeToken(value.consent_token) || typeof value.display_code !== "string"
    || !/^\d{6}$/.test(value.display_code) || !isLowerHexSha256(value.argument_fingerprint)
    || !isPositiveSafeInteger(value.expires_at_unix_ms)) throw new Error("OutputControl physical-confirmation challenge was invalid; nothing was applied.");
  return value as unknown as OutputConsentChallenge;
};

const assertStatus = (value: unknown, challenge: OutputConsentChallenge, requestId: number): OutputConsentStatus => {
  if (!isObject(value) || !hasExactKeys(value, ["operation_id", "request_id", "challenge_id", "state", "expires_at_unix_ms"])
    || value.operation_id !== OUTPUT_CONSENT_STATUS_QUERY_OPERATION_ID || value.request_id !== requestId
    || value.challenge_id !== challenge.challenge_id || value.state !== "pending_physical_input" && value.state !== "ready"
    || !isPositiveSafeInteger(value.expires_at_unix_ms)
    || value.state === "pending_physical_input" && value.expires_at_unix_ms !== challenge.expires_at_unix_ms
    || value.state === "ready" && (value.expires_at_unix_ms < challenge.expires_at_unix_ms
      || value.expires_at_unix_ms > challenge.expires_at_unix_ms + OUTPUT_CONSENT_READY_HANDOFF_MS)) {
    throw new Error("OutputControl physical-confirmation status was invalid; nothing was applied.");
  }
  return value as unknown as OutputConsentStatus;
};

const assertPhase = (value: unknown, errorMessage: string): OutputLeaseReceiptPhase => {
  if (value !== "unclaimed" && value !== "held_active" && value !== "held_orphaned") throw new Error(errorMessage);
  return value;
};
const assertOutcome = (value: unknown, errorMessage: string): OutputLeaseReceiptOutcome => {
  if (!["acquired", "renewed", "expiry_observed", "recovered", "relinquished", "transferred", "owner_retired", "project_orphaned", "authorized"].includes(value as string)) throw new Error(errorMessage);
  return value as OutputLeaseReceiptOutcome;
};

const assertLeaseResult = (value: unknown, action: OutputControlOperationAction): OutputControlLeaseResult => {
  const errorMessage = "OutputControl lease result was invalid; physical output state is unknown.";
  if (!isObject(value) || !hasExactKeys(value, ["authority", "resources", "phase", "outcome", "audit_sequence", "changes"])
    || !Array.isArray(value.changes) || value.changes.length !== 1 || !isPositiveSafeInteger(value.audit_sequence)) throw new Error(errorMessage);
  const authority = assertLeaseAuthority(value.authority, errorMessage);
  const resources = assertResources(value.resources, errorMessage);
  const phase = assertPhase(value.phase, errorMessage);
  const outcome = assertOutcome(value.outcome, errorMessage);
  const rawChange = value.changes[0];
  if (!isObject(rawChange) || !hasExactKeys(rawChange, [
    "lease_id", "before_generation", "after_generation", "before_resources", "after_resources", "before_phase", "after_phase",
  ]) || rawChange.lease_id !== authority.lease_id) throw new Error(errorMessage);
  const beforeGeneration = rawChange.before_generation === null ? null : isPositiveSafeInteger(rawChange.before_generation) ? rawChange.before_generation : undefined;
  const afterGeneration = rawChange.after_generation === null ? null : isPositiveSafeInteger(rawChange.after_generation) ? rawChange.after_generation : undefined;
  if (beforeGeneration === undefined || afterGeneration === undefined || beforeGeneration === null && afterGeneration === null) throw new Error(errorMessage);
  const beforePhase = rawChange.before_phase === null ? null : assertPhase(rawChange.before_phase, errorMessage);
  const afterPhase = rawChange.after_phase === null ? null : assertPhase(rawChange.after_phase, errorMessage);
  const beforeResources = beforeGeneration === null
    ? (Array.isArray(rawChange.before_resources) && rawChange.before_resources.length === 0 ? [] : (() => { throw new Error(errorMessage); })())
    : beforePhase === "unclaimed"
      ? (Array.isArray(rawChange.before_resources) && rawChange.before_resources.length === 0 ? [] : (() => { throw new Error(errorMessage); })())
      : assertResources(rawChange.before_resources, errorMessage);
  const afterResources = afterGeneration === null
    ? (Array.isArray(rawChange.after_resources) && rawChange.after_resources.length === 0 ? [] : (() => { throw new Error(errorMessage); })())
    : afterPhase === "unclaimed"
      ? (Array.isArray(rawChange.after_resources) && rawChange.after_resources.length === 0 ? [] : (() => { throw new Error(errorMessage); })())
      : assertResources(rawChange.after_resources, errorMessage);
  if (beforeGeneration !== null && beforePhase === null || beforeGeneration === null && beforePhase !== null
    || afterGeneration !== null && afterPhase === null || afterGeneration === null && afterPhase !== null) throw new Error(errorMessage);
  const terminalGeneration = afterGeneration ?? beforeGeneration;
  const terminalPhase = afterPhase ?? beforePhase;
  const terminalResources = outcome === "relinquished" ? beforeResources : afterResources;
  if (terminalGeneration !== authority.generation || terminalPhase !== phase || !sameResources(resources, terminalResources)) throw new Error(errorMessage);
  if (action.kind !== "acquire_lease" && authority.lease_id !== action.lease.lease_id) throw new Error(errorMessage);
  switch (action.kind) {
    case "arm":
      if (outcome !== "authorized" || phase !== "held_active" || !sameResources(resources, resourcesForRole(action.role))) throw new Error(errorMessage);
      break;
    case "release_blackout":
      if (outcome !== "authorized" || phase !== "held_active" || !sameResources(resources, ["lighting", "video"])) throw new Error(errorMessage);
      break;
    case "take_over_standby":
      if (!(outcome === "authorized" && phase === "held_active" || outcome === "project_orphaned" && phase === "held_orphaned") || !sameResources(resources, ["lighting", "video"])) throw new Error(errorMessage);
      break;
    case "acquire_lease":
      if (outcome !== "acquired" || phase !== "held_active" || !sameResources(resources, resourcesForRole(action.role))) throw new Error(errorMessage);
      break;
    case "renew_lease":
      if (outcome !== "renewed" || phase !== "held_active") throw new Error(errorMessage);
      break;
    case "recover_lease":
      if (outcome !== "recovered" || phase !== "held_active") throw new Error(errorMessage);
      break;
    case "relinquish_output_lease":
      if (outcome !== "relinquished" || phase !== "unclaimed") throw new Error(errorMessage);
      break;
    case "force_transfer_lease":
      if (outcome !== "transferred" || phase !== "held_active") throw new Error(errorMessage);
      break;
  }
  return {
    authority,
    resources,
    phase,
    outcome,
    audit_sequence: value.audit_sequence,
    changes: [{
      lease_id: rawChange.lease_id,
      before_generation: beforeGeneration,
      after_generation: afterGeneration,
      before_resources: beforeResources,
      after_resources: afterResources,
      before_phase: beforePhase,
      after_phase: afterPhase,
    }],
  };
};

const knownPreActionRejections = new Set([
  "invalid_request", "forbidden", "stale_fence", "consent_missing", "consent_expired", "consent_pending",
  "consent_replayed", "consent_wrong_binding", "consent_device_removed", "busy", "overloaded",
]);

const deepFreeze = <T>(value: T): T => {
  if (value && typeof value === "object" && !Object.isFrozen(value)) {
    Object.freeze(value);
    for (const child of Object.values(value as Record<string, unknown>)) deepFreeze(child);
  }
  return value;
};

const assertResponse = (
  value: unknown,
  action: OutputControlOperationAction,
  operationId: string,
  requestId: number,
  expectedFence: OutputControlFence,
  expectedArgumentFingerprint: string,
): OutputControlReceipt => {
  if (!isObject(value) || value.type !== "receipt" && value.type !== "rejected") throw new Error("OutputControl terminal response was invalid; physical output state is unknown.");
  if (!hasExactKeys(value, value.type === "receipt" ? ["type", "receipt"] : ["type", "rejection"])) throw new Error("OutputControl terminal response was invalid; physical output state is unknown.");
  const result = (value.type === "receipt" ? value.receipt : value.rejection) as Record<string, unknown> | undefined;
  if (!isObject(result) || result.operation_id !== operationId || result.request_id !== requestId) throw new Error("OutputControl terminal response identity did not match the request; physical output state is unknown.");
  if (value.type === "rejected") {
    if (!hasExactKeys(result, ["operation_id", "request_id", "error"]) || typeof result.error !== "string") throw new Error("OutputControl rejection was invalid; physical output state is unknown.");
    if (result.error === "forbidden" || result.error === "consent_expired") throw new Error(`OutputControl rejected (${result.error}); output was not applied; refresh lease state.`);
    if (knownPreActionRejections.has(result.error)) throw new Error(`OutputControl rejected (${result.error}); nothing was applied.`);
    throw new Error(`OutputControl rejected (${result.error}); physical output state is unknown.`);
  }
  if (!hasExactKeys(result, ["operation_id", "request_id", "shape_sha256", "argument_fingerprint", "audit_sequence", "fence_before", "fence_after", "outcome", "lease_result"])
    || !isLowerHexSha256(result.shape_sha256) || !isLowerHexSha256(result.argument_fingerprint)
    || result.argument_fingerprint !== expectedArgumentFingerprint || !isPositiveSafeInteger(result.audit_sequence)
    || result.outcome !== "applied" && result.outcome !== "no_op") throw new Error("OutputControl receipt was invalid; physical output state is unknown.");
  const fenceBefore = assertFence(result.fence_before, "OutputControl receipt was invalid; physical output state is unknown.");
  const fenceAfter = assertFence(result.fence_after, "OutputControl receipt was invalid; physical output state is unknown.");
  if (!fencesEqual(fenceBefore, expectedFence) || result.outcome === "no_op" && !fencesEqual(fenceBefore, fenceAfter) || result.outcome === "applied" && fencesEqual(fenceBefore, fenceAfter)) throw new Error("OutputControl receipt was inconsistent; physical output state is unknown.");
  return {
    operation_id: operationId,
    request_id: requestId,
    shape_sha256: result.shape_sha256,
    argument_fingerprint: result.argument_fingerprint,
    audit_sequence: result.audit_sequence,
    fence_before: fenceBefore,
    fence_after: fenceAfter,
    outcome: result.outcome,
    lease_result: assertLeaseResult(result.lease_result, action),
  };
};

const assertSelectedLeaseIsUsable = (query: OutputLeaseAuthorityQuery, action: OutputControlOperationAction): void => {
  if (action.kind === "acquire_lease") return;
  const selected = query.statuses.filter((status): status is OutputLeaseAuthorityQueryHeld =>
    status.status !== "unavailable" && status.authority.lease_id === action.lease.lease_id);
  if (selected.length !== 1 || selected[0].authority.generation !== action.lease.generation
    || action.kind === "arm" && selected[0].status !== "held_active"
    || action.kind === "release_blackout" && selected[0].status !== "held_active"
    || action.kind === "take_over_standby" && selected[0].status !== "held_active"
    || action.kind === "renew_lease" && selected[0].status !== "held_active"
    || action.kind === "recover_lease" && selected[0].status !== "held_orphaned") {
    throw new Error("Selected output lease is unavailable, orphaned, stale, or has the wrong resources; nothing was applied.");
  }
  if (action.kind === "arm" || action.kind === "release_blackout" || action.kind === "take_over_standby") {
    const expectedResources = action.kind === "arm" ? resourcesForRole(action.role) : ["lighting", "video"] as const;
    if (!sameResources(selected[0].resources, expectedResources)) throw new Error("Selected output lease is unavailable, orphaned, stale, or has the wrong resources; nothing was applied.");
  }
};

const executeOutputControlOperation = async (
  invoke: FrontendTauriInvoke,
  action: OutputControlOperationAction,
  onChallenge?: (notice: OutputControlChallengeNotice) => void,
): Promise<OutputControlReceipt> => {
  if (!onChallenge) throw new Error("OutputControl requires a visible, nonblocking physical-confirmation challenge surface; nothing was applied.");
  const requestId = allocateRequestId();
  const operationId = operationIdForAction(action);
  assertAction(action);
  const authority = assertAuthority(await invoke<unknown>("query_output_control_authority_v1"));
  const leaseQuery = await queryOutputLeaseAuthority(invoke);
  assertSelectedLeaseIsUsable(leaseQuery, action);
  const prepareArgs = deepFreeze({
    request: { operation_id: OUTPUT_CONSENT_PREPARE_OPERATION_ID, request_id: requestId, expected_fence: authority.fence, action },
  });
  const challenge = assertChallenge(await invoke<unknown>("prepare_output_consent_v1", prepareArgs), operationId, requestId);
  onChallenge({ action, displayCode: challenge.display_code, expiresAtUnixMs: challenge.expires_at_unix_ms });
  // The backend accepts digits only during the original challenge window.
  // One final status query may cross that boundary when the sixth physical
  // digit completed immediately before expiry; Ready then carries a single,
  // bounded five-second handoff to the one-shot consume path.  Pending input
  // still expires in the backend at the original deadline.
  const deadline = Math.min(
    challenge.expires_at_unix_ms + OUTPUT_CONSENT_READY_HANDOFF_MS,
    Date.now() + 20_000,
  );
  let statusRequestId = allocateRequestId();
  let ready = false;
  while (!ready) {
    if (Date.now() >= deadline) throw new Error("Physical confirmation expired or was not received; nothing was applied.");
    const status = assertStatus(await invoke<unknown>("query_output_consent_status_v1", {
      request: { operation_id: OUTPUT_CONSENT_STATUS_QUERY_OPERATION_ID, request_id: statusRequestId, challenge_id: challenge.challenge_id },
    }), challenge, statusRequestId);
    ready = status.state === "ready";
    if (!ready) {
      await wait(100);
      statusRequestId = allocateRequestId();
    }
  }
  const executeArgs = deepFreeze({
    request: { operation_id: operationId, request_id: requestId, expected_fence: authority.fence, consent_token: challenge.consent_token, action },
  });
  let terminal: unknown;
  const command = commandForAction(action) as Parameters<FrontendTauriInvoke>[0];
  try {
    terminal = await invoke<unknown>(command, executeArgs);
  } catch (firstError) {
    try {
      terminal = await invoke<unknown>(command, executeArgs);
    } catch {
      throw new Error(`OutputControl execution reply was lost (${String(firstError)}); physical output state is unknown.`);
    }
  }
  return assertResponse(terminal, action, operationId, requestId, authority.fence, challenge.argument_fingerprint);
};

/** Execute an ordinary action with a caller-selected exact active lease. */
export async function executeOutputControl(
  invoke: FrontendTauriInvoke,
  action: OutputControlAction,
  onChallenge?: (notice: OutputControlChallengeNotice) => void,
): Promise<OutputControlReceipt> {
  return executeOutputControlOperation(invoke, action, onChallenge);
}

/** Execute a lease lifecycle action through the same consent and receipt lane. */
export async function executeOutputLeaseLifecycle(
  invoke: FrontendTauriInvoke,
  action: OutputLeaseLifecycleAction,
  onChallenge?: (notice: OutputControlChallengeNotice) => void,
): Promise<OutputControlReceipt> {
  return executeOutputControlOperation(invoke, action, onChallenge);
}
