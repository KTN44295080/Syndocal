import type { FrontendTauriInvoke } from "./tauriInvokeCommands";

export const OUTPUT_CONTROL_AUTHORITY_QUERY_OPERATION_ID =
  "syndocal.query.output.control.authority.v1";
export const OUTPUT_CONSENT_PREPARE_OPERATION_ID = "syndocal.output.consent.prepare.v1";
export const OUTPUT_CONSENT_STATUS_QUERY_OPERATION_ID =
  "syndocal.query.output.consent.status.v1";
export const OUTPUT_OWNERSHIP_ARM_OPERATION_ID = "syndocal.output.ownership.arm.v1";
export const OUTPUT_BLACKOUT_RELEASE_OPERATION_ID = "syndocal.output.blackout.release.v1";
export const OUTPUT_STANDBY_TAKEOVER_OPERATION_ID = "syndocal.output.standby.takeover.v1";

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

export type OutputControlAction =
  | { kind: "arm"; role: OutputControlTargetRole }
  | { kind: "release_blackout" }
  | {
      kind: "take_over_standby";
      force: boolean;
      standby_session_id: string;
      standby_generation: number;
    };

interface OutputControlAuthority {
  operation_id: typeof OUTPUT_CONTROL_AUTHORITY_QUERY_OPERATION_ID;
  fence: OutputControlFence;
}

interface OutputConsentChallenge {
  operation_id: typeof OUTPUT_CONSENT_PREPARE_OPERATION_ID;
  request_id: number;
  target_operation_id:
    | typeof OUTPUT_OWNERSHIP_ARM_OPERATION_ID
    | typeof OUTPUT_BLACKOUT_RELEASE_OPERATION_ID
    | typeof OUTPUT_STANDBY_TAKEOVER_OPERATION_ID;
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

interface OutputControlReceipt {
  operation_id: string;
  request_id: number;
  shape_sha256: string;
  argument_fingerprint: string;
  audit_sequence: number;
  fence_before: OutputControlFence;
  fence_after: OutputControlFence;
  outcome: "applied" | "no_op";
}

export interface OutputControlChallengeNotice {
  action: OutputControlAction;
  displayCode: string;
  expiresAtUnixMs: number;
}

let nextOutputControlRequestId = 1;

const allocateRequestId = (): number => {
  const requestId = nextOutputControlRequestId;
  nextOutputControlRequestId = nextOutputControlRequestId >= Number.MAX_SAFE_INTEGER
    ? 1
    : nextOutputControlRequestId + 1;
  return requestId;
};

const operationIdForAction = (action: OutputControlAction): string => {
  switch (action.kind) {
    case "arm":
      return OUTPUT_OWNERSHIP_ARM_OPERATION_ID;
    case "release_blackout":
      return OUTPUT_BLACKOUT_RELEASE_OPERATION_ID;
    case "take_over_standby":
      return OUTPUT_STANDBY_TAKEOVER_OPERATION_ID;
  }
};

const wait = (milliseconds: number): Promise<void> => new Promise((resolve) => {
  globalThis.setTimeout(resolve, milliseconds);
});

const isObject = (value: unknown): value is Record<string, unknown> =>
  Boolean(value) && typeof value === "object";

const isPositiveSafeInteger = (value: unknown): value is number =>
  typeof value === "number" && Number.isSafeInteger(value) && value > 0;

const isLowerHexSha256 = (value: unknown): value is string =>
  typeof value === "string" && /^[0-9a-f]{64}$/.test(value);

const isCanonicalOpaqueToken = (value: unknown): value is string =>
  typeof value === "string" && /^[A-Za-z0-9_-]{22}$/.test(value);

const hasExactKeys = (value: Record<string, unknown>, expected: readonly string[]): boolean => {
  const actual = Object.keys(value).sort();
  const required = [...expected].sort();
  return actual.length === required.length
    && actual.every((key, index) => key === required[index]);
};

const outputControlFenceKeys = [
  "process_incarnation",
  "session_incarnation",
  "project_epoch",
  "project_revision",
  "project_checkpoint_hash",
  "project_publication_generation",
  "output_epoch",
  "output_generation",
  "safety_blackout_epoch",
  "safety_blackout_generation",
] as const;

const assertFence = (value: unknown, errorMessage: string): OutputControlFence => {
  if (!isObject(value)
    || !hasExactKeys(value, outputControlFenceKeys)
    || !isPositiveSafeInteger(value.process_incarnation)
    || !isPositiveSafeInteger(value.session_incarnation)
    || !isPositiveSafeInteger(value.project_epoch)
    || !isPositiveSafeInteger(value.project_revision)
    || !isLowerHexSha256(value.project_checkpoint_hash)
    || !isPositiveSafeInteger(value.project_publication_generation)
    || !isPositiveSafeInteger(value.output_epoch)
    || !isPositiveSafeInteger(value.output_generation)
    || !isPositiveSafeInteger(value.safety_blackout_epoch)
    || !isPositiveSafeInteger(value.safety_blackout_generation)) {
    throw new Error(errorMessage);
  }
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

const assertAuthority = (value: unknown): OutputControlAuthority => {
  if (!isObject(value)
    || value.operation_id !== OUTPUT_CONTROL_AUTHORITY_QUERY_OPERATION_ID
    || !hasExactKeys(value, ["operation_id", "fence"])) {
    throw new Error("OutputControl authority response was invalid; nothing was applied.");
  }
  return {
    operation_id: OUTPUT_CONTROL_AUTHORITY_QUERY_OPERATION_ID,
    fence: assertFence(
      value.fence,
      "OutputControl authority response was invalid; nothing was applied.",
    ),
  };
};

const assertChallenge = (
  value: unknown,
  operationId: string,
  requestId: number,
): OutputConsentChallenge => {
  if (!isObject(value)
    || !hasExactKeys(value, [
      "operation_id",
      "request_id",
      "target_operation_id",
      "challenge_id",
      "consent_token",
      "display_code",
      "argument_fingerprint",
      "expires_at_unix_ms",
    ])
    || value.operation_id !== OUTPUT_CONSENT_PREPARE_OPERATION_ID
    || value.request_id !== requestId
    || value.target_operation_id !== operationId
    || !isCanonicalOpaqueToken(value.challenge_id)
    || !isCanonicalOpaqueToken(value.consent_token)
    || typeof value.display_code !== "string" || !/^\d{6}$/.test(value.display_code)
    || !isLowerHexSha256(value.argument_fingerprint)
    || !isPositiveSafeInteger(value.expires_at_unix_ms)) {
    throw new Error("OutputControl physical-confirmation challenge was invalid; nothing was applied.");
  }
  return value as unknown as OutputConsentChallenge;
};

const assertStatus = (
  value: unknown,
  challenge: OutputConsentChallenge,
  requestId: number,
): OutputConsentStatus => {
  if (!isObject(value)
    || !hasExactKeys(value, [
      "operation_id",
      "request_id",
      "challenge_id",
      "state",
      "expires_at_unix_ms",
    ])
    || value.operation_id !== OUTPUT_CONSENT_STATUS_QUERY_OPERATION_ID
    || value.request_id !== requestId
    || value.challenge_id !== challenge.challenge_id
    || (value.state !== "pending_physical_input" && value.state !== "ready")
    || !isPositiveSafeInteger(value.expires_at_unix_ms)
    || value.expires_at_unix_ms !== challenge.expires_at_unix_ms) {
    throw new Error("OutputControl physical-confirmation status was invalid; nothing was applied.");
  }
  return value as unknown as OutputConsentStatus;
};

const knownPreActionRejections = new Set([
  "invalid_request",
  "forbidden",
  "stale_fence",
  "consent_missing",
  "consent_expired",
  "consent_pending",
  "consent_replayed",
  "consent_wrong_binding",
  "consent_device_removed",
  "busy",
  "overloaded",
]);

const assertResponse = (
  value: unknown,
  operationId: string,
  requestId: number,
  expectedFence: OutputControlFence,
  expectedArgumentFingerprint: string,
): OutputControlReceipt => {
  if (!isObject(value) || (value.type !== "receipt" && value.type !== "rejected")) {
    throw new Error("OutputControl terminal response was invalid; physical output state is unknown.");
  }
  if (!hasExactKeys(value, value.type === "receipt"
    ? ["type", "receipt"]
    : ["type", "rejection"])) {
    throw new Error("OutputControl terminal response was invalid; physical output state is unknown.");
  }
  const result = (value.type === "receipt" ? value.receipt : value.rejection) as Record<string, unknown> | undefined;
  if (!isObject(result)) {
    throw new Error("OutputControl terminal response omitted its result; physical output state is unknown.");
  }
  if (result.operation_id !== operationId || result.request_id !== requestId) {
    throw new Error("OutputControl terminal response identity did not match the request; physical output state is unknown.");
  }
  if (value.type === "rejected") {
    if (!hasExactKeys(result, ["operation_id", "request_id", "error"])
      || typeof result.error !== "string") {
      throw new Error("OutputControl rejection was invalid; physical output state is unknown.");
    }
    if (knownPreActionRejections.has(result.error)) {
      throw new Error(`OutputControl rejected (${result.error}); nothing was applied.`);
    }
    throw new Error(`OutputControl rejected (${result.error}); physical output state is unknown.`);
  }
  if (!hasExactKeys(result, [
    "operation_id",
    "request_id",
    "shape_sha256",
    "argument_fingerprint",
    "audit_sequence",
    "fence_before",
    "fence_after",
    "outcome",
  ])
    || !isLowerHexSha256(result.shape_sha256)
    || !isLowerHexSha256(result.argument_fingerprint)
    || result.argument_fingerprint !== expectedArgumentFingerprint
    || !isPositiveSafeInteger(result.audit_sequence)
    || (result.outcome !== "applied" && result.outcome !== "no_op")) {
    throw new Error("OutputControl receipt was invalid; physical output state is unknown.");
  }
  const fenceBefore = assertFence(
    result.fence_before,
    "OutputControl receipt was invalid; physical output state is unknown.",
  );
  const fenceAfter = assertFence(
    result.fence_after,
    "OutputControl receipt was invalid; physical output state is unknown.",
  );
  if (!fencesEqual(fenceBefore, expectedFence)
    || (result.outcome === "no_op" && !fencesEqual(fenceBefore, fenceAfter))
    || (result.outcome === "applied" && fencesEqual(fenceBefore, fenceAfter))) {
    throw new Error("OutputControl receipt was inconsistent; physical output state is unknown.");
  }
  return {
    operation_id: operationId,
    request_id: requestId,
    shape_sha256: result.shape_sha256,
    argument_fingerprint: result.argument_fingerprint,
    audit_sequence: result.audit_sequence,
    fence_before: fenceBefore,
    fence_after: fenceAfter,
    outcome: result.outcome,
  };
};

/**
 * Execute one local R4 action through the backend-issued fence, a fresh
 * physical-input challenge, and its action-specific canonical Tauri ingress.
 * The browser/DOM never supplies consent: it only displays the challenge and
 * polls the backend's Raw Input result.
 */
export async function executeOutputControl(
  invoke: FrontendTauriInvoke,
  action: OutputControlAction,
  onChallenge?: (notice: OutputControlChallengeNotice) => void,
): Promise<OutputControlReceipt> {
  if (!onChallenge) {
    throw new Error(
      "OutputControl requires a visible, nonblocking physical-confirmation challenge surface; nothing was applied.",
    );
  }
  const operationId = operationIdForAction(action);
  const authority = assertAuthority(await invoke<unknown>("query_output_control_authority_v1"));
  const requestId = allocateRequestId();
  const challenge = assertChallenge(
    await invoke<unknown>("prepare_output_consent_v1", {
      request: {
        operation_id: OUTPUT_CONSENT_PREPARE_OPERATION_ID,
        request_id: requestId,
        expected_fence: authority.fence,
        action,
      },
    }),
    operationId,
    requestId,
  );
  const notice = {
    action,
    displayCode: challenge.display_code,
    expiresAtUnixMs: challenge.expires_at_unix_ms,
  } satisfies OutputControlChallengeNotice;
  onChallenge(notice);

  const deadline = Math.min(challenge.expires_at_unix_ms, Date.now() + 15_000);
  let statusRequestId = allocateRequestId();
  while (Date.now() < deadline) {
    const status = assertStatus(
      await invoke<unknown>("query_output_consent_status_v1", {
        request: {
          operation_id: OUTPUT_CONSENT_STATUS_QUERY_OPERATION_ID,
          request_id: statusRequestId,
          challenge_id: challenge.challenge_id,
        },
      }),
      challenge,
      statusRequestId,
    );
    if (status.state === "ready") break;
    await wait(100);
    statusRequestId = allocateRequestId();
  }
  if (Date.now() >= deadline) {
    throw new Error("Physical confirmation expired or was not received; nothing was applied.");
  }

  const command = action.kind === "arm"
    ? "arm_output_control_v1"
    : action.kind === "release_blackout"
      ? "release_blackout_output_control_v1"
      : "take_over_output_control_v1";
  const executeArgs = {
    request: {
      operation_id: operationId,
      request_id: requestId,
      expected_fence: authority.fence,
      consent_token: challenge.consent_token,
      action,
    },
  };
  let terminal: unknown;
  try {
    terminal = await invoke<unknown>(command, executeArgs);
  } catch (firstError) {
    // Reply loss is safe to retry exactly once with the same frozen request;
    // the backend receipt lane returns the original terminal result without
    // repeating a physical action. Typed rejection never reaches this path.
    try {
      terminal = await invoke<unknown>(command, executeArgs);
    } catch {
      throw new Error(
        `OutputControl execution reply was lost (${String(firstError)}); physical output state is unknown.`,
      );
    }
  }
  return assertResponse(
    terminal,
    operationId,
    requestId,
    authority.fence,
    challenge.argument_fingerprint,
  );
}
