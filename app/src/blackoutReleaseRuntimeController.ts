/**
 * R4 renderer lane for releasing the runtime safety blackout.
 *
 * The finite command union below is included in the generated frontend invoke
 * authority only after the canonical R4 registry row is present.
 *
 * The controller never mints or fakes human presence. `prepareRelease` returns
 * the backend challenge so the desktop can display it. Only physical Raw Input
 * accepted by the backend can move that challenge to `ready`.
 */

export const outputControlAuthorityQueryOperationId =
  "syndocal.query.output.control.authority.v1" as const;
export const outputConsentPrepareOperationId =
  "syndocal.output.consent.prepare.v1" as const;
export const outputConsentStatusQueryOperationId =
  "syndocal.query.output.consent.status.v1" as const;
export const blackoutReleaseOperationId =
  "syndocal.output.blackout.release.v1" as const;

export const blackoutReleaseInvokeCommands = [
  "query_output_control_authority_v1",
  "prepare_output_consent_v1",
  "query_output_consent_status_v1",
  "execute_output_control_v1",
] as const;
export type BlackoutReleaseInvokeCommand = typeof blackoutReleaseInvokeCommands[number];
export type BlackoutReleaseInvoke = <T>(
  command: BlackoutReleaseInvokeCommand,
  args?: Record<string, unknown>,
) => Promise<T>;

export type BlackoutReleaseErrorCode =
  | "invalid_request"
  | "forbidden"
  | "stale_fence"
  | "consent_missing"
  | "consent_expired"
  | "consent_pending"
  | "consent_replayed"
  | "consent_wrong_binding"
  | "consent_device_removed"
  | "busy"
  | "overloaded"
  | "publication_failed"
  | "internal";

export type OutputControlFence = {
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
};

export type BlackoutReleasePreparedConsent = {
  authority: {
    operation_id: typeof outputControlAuthorityQueryOperationId;
    fence: OutputControlFence;
  };
  challenge: {
    operation_id: typeof outputConsentPrepareOperationId;
    request_id: number;
    target_operation_id: typeof blackoutReleaseOperationId;
    challenge_id: string;
    consent_token: string;
    display_code: string;
    argument_fingerprint: string;
    expires_at_unix_ms: number;
  };
  /** Stable across the one exact reply-loss retry. */
  execution_request_id: number;
};

export type BlackoutReleaseConsentState = "pending_physical_input" | "ready";
export type BlackoutReleaseOutcome = "applied" | "no_op";

export type BlackoutReleaseReceipt = {
  operation_id: typeof blackoutReleaseOperationId;
  request_id: number;
  shape_sha256: string;
  argument_fingerprint: string;
  audit_sequence: number;
  fence_before: OutputControlFence;
  fence_after: OutputControlFence;
  outcome: BlackoutReleaseOutcome;
};

const errorCodes = new Set<BlackoutReleaseErrorCode>([
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
  "publication_failed",
  "internal",
]);

const lowerHexSha256 = /^[0-9a-f]{64}$/;
// A 16-byte unpadded base64url value is 22 characters. Its final sextet has
// only two payload bits, so canonical encoding limits the final character to
// indices 0, 16, 32, or 48 instead of accepting alternate textual encodings.
const canonicalOpaqueId = /^[A-Za-z0-9_-]{21}[AQgw]$/;
const sixDigits = /^\d{6}$/;

const isRecord = (value: unknown): value is Record<string, unknown> =>
  Boolean(value) && typeof value === "object" && !Array.isArray(value);
const isNonNegativeSafeInteger = (value: unknown): value is number =>
  typeof value === "number" && Number.isSafeInteger(value) && value >= 0;
const isNonZeroSafeInteger = (value: unknown): value is number =>
  isNonNegativeSafeInteger(value) && value > 0;
const isHash = (value: unknown): value is string =>
  typeof value === "string" && lowerHexSha256.test(value);
const isOpaqueId = (value: unknown): value is string =>
  typeof value === "string" && canonicalOpaqueId.test(value);

const isFence = (value: unknown): value is OutputControlFence => {
  if (!isRecord(value)) return false;
  return isNonZeroSafeInteger(value.process_incarnation)
    && isNonZeroSafeInteger(value.session_incarnation)
    && isNonNegativeSafeInteger(value.project_epoch)
    && isNonNegativeSafeInteger(value.project_revision)
    && isHash(value.project_checkpoint_hash)
    && isNonNegativeSafeInteger(value.project_publication_generation)
    && isNonZeroSafeInteger(value.output_epoch)
    && isNonZeroSafeInteger(value.output_generation)
    && isNonZeroSafeInteger(value.safety_blackout_epoch)
    && isNonZeroSafeInteger(value.safety_blackout_generation);
};

const sameFence = (left: OutputControlFence, right: OutputControlFence): boolean =>
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

let fallbackRequestCounter = 0;
export const nextBlackoutReleaseRequestId = (): number => {
  const values = new Uint32Array(1);
  if (globalThis.crypto?.getRandomValues) globalThis.crypto.getRandomValues(values);
  else {
    fallbackRequestCounter = (fallbackRequestCounter + 1) >>> 0;
    values[0] = fallbackRequestCounter;
  }
  const high = (Date.now() % 2_097_151) + 1;
  return high * 4_294_967_296 + values[0];
};

export class BlackoutReleaseCommandError extends Error {
  readonly code: BlackoutReleaseErrorCode;

  constructor(code: BlackoutReleaseErrorCode) {
    super(`Blackout release was rejected (${code}).`);
    this.name = "BlackoutReleaseCommandError";
    this.code = code;
  }
}

export class BlackoutReleaseProtocolError extends Error {
  constructor(part: string) {
    super(`Blackout release returned an invalid ${part}.`);
    this.name = "BlackoutReleaseProtocolError";
  }
}

const validateAuthority = (value: unknown): BlackoutReleasePreparedConsent["authority"] => {
  if (!isRecord(value)
    || value.operation_id !== outputControlAuthorityQueryOperationId
    || !isFence(value.fence)) {
    throw new BlackoutReleaseProtocolError("authority");
  }
  return value as BlackoutReleasePreparedConsent["authority"];
};

const validateChallenge = (
  value: unknown,
  requestId: number,
): BlackoutReleasePreparedConsent["challenge"] => {
  if (!isRecord(value)
    || value.operation_id !== outputConsentPrepareOperationId
    || value.request_id !== requestId
    || value.target_operation_id !== blackoutReleaseOperationId
    || !isOpaqueId(value.challenge_id)
    || !isOpaqueId(value.consent_token)
    || typeof value.display_code !== "string"
    || !sixDigits.test(value.display_code)
    || !isHash(value.argument_fingerprint)
    || !isNonZeroSafeInteger(value.expires_at_unix_ms)) {
    throw new BlackoutReleaseProtocolError("prepared consent challenge");
  }
  return value as BlackoutReleasePreparedConsent["challenge"];
};

const validateStatus = (
  value: unknown,
  requestId: number,
  challengeId: string,
): BlackoutReleaseConsentState => {
  if (!isRecord(value)
    || value.operation_id !== outputConsentStatusQueryOperationId
    || value.request_id !== requestId
    || value.challenge_id !== challengeId
    || (value.state !== "pending_physical_input" && value.state !== "ready")
    || !isNonZeroSafeInteger(value.expires_at_unix_ms)) {
    throw new BlackoutReleaseProtocolError("consent status");
  }
  return value.state;
};

const validateExecutionResponse = (
  value: unknown,
  prepared: BlackoutReleasePreparedConsent,
): BlackoutReleaseReceipt => {
  const requestId = prepared.execution_request_id;
  if (!isRecord(value) || (value.type !== "receipt" && value.type !== "rejected")) {
    throw new BlackoutReleaseProtocolError("terminal response");
  }
  if (value.type === "rejected") {
    const rejection = isRecord(value.rejection) ? value.rejection : null;
    if (!rejection
      || rejection.operation_id !== blackoutReleaseOperationId
      || rejection.request_id !== requestId
      || typeof rejection.error !== "string"
      || !errorCodes.has(rejection.error as BlackoutReleaseErrorCode)) {
      throw new BlackoutReleaseProtocolError("rejection");
    }
    throw new BlackoutReleaseCommandError(rejection.error as BlackoutReleaseErrorCode);
  }
  const receipt = isRecord(value.receipt) ? value.receipt : null;
  if (!receipt
    || receipt.operation_id !== blackoutReleaseOperationId
    || receipt.request_id !== requestId
    || !isHash(receipt.shape_sha256)
    || receipt.argument_fingerprint !== prepared.challenge.argument_fingerprint
    || !isNonZeroSafeInteger(receipt.audit_sequence)
    || !isFence(receipt.fence_before)
    || !sameFence(receipt.fence_before, prepared.authority.fence)
    || !isFence(receipt.fence_after)
    || (receipt.outcome !== "applied" && receipt.outcome !== "no_op")) {
    throw new BlackoutReleaseProtocolError("terminal receipt");
  }
  if ((receipt.outcome === "no_op" && !sameFence(receipt.fence_before, receipt.fence_after))
    || (receipt.outcome === "applied" && sameFence(receipt.fence_before, receipt.fence_after))) {
    throw new BlackoutReleaseProtocolError("receipt fence transition");
  }
  return receipt as BlackoutReleaseReceipt;
};

const validatePrepared = (prepared: BlackoutReleasePreparedConsent): void => {
  validateAuthority(prepared.authority);
  validateChallenge(prepared.challenge, prepared.challenge.request_id);
  if (!isNonZeroSafeInteger(prepared.execution_request_id)) {
    throw new BlackoutReleaseProtocolError("execution request identity");
  }
};

export type BlackoutReleaseRuntimeControllerOptions = {
  invoke: BlackoutReleaseInvoke;
  /** One exact resend is the default reply-loss recovery budget. */
  maxReplyLossRetries?: number;
};

export function createBlackoutReleaseRuntimeController(
  options: BlackoutReleaseRuntimeControllerOptions,
) {
  const maxReplyLossRetries = Math.max(0, Math.floor(options.maxReplyLossRetries ?? 1));

  const prepareRelease = async (): Promise<BlackoutReleasePreparedConsent> => {
    const authority = validateAuthority(
      await options.invoke<unknown>("query_output_control_authority_v1"),
    );
    const requestId = nextBlackoutReleaseRequestId();
    const challenge = validateChallenge(
      await options.invoke<unknown>("prepare_output_consent_v1", {
        request: {
          operation_id: outputConsentPrepareOperationId,
          request_id: requestId,
          expected_fence: authority.fence,
          action: { kind: "release_blackout" },
        },
      }),
      requestId,
    );
    return {
      authority,
      challenge,
      execution_request_id: nextBlackoutReleaseRequestId(),
    };
  };

  const consentStatus = async (
    prepared: BlackoutReleasePreparedConsent,
  ): Promise<BlackoutReleaseConsentState> => {
    validatePrepared(prepared);
    const requestId = nextBlackoutReleaseRequestId();
    return validateStatus(
      await options.invoke<unknown>("query_output_consent_status_v1", {
        request: {
          operation_id: outputConsentStatusQueryOperationId,
          request_id: requestId,
          challenge_id: prepared.challenge.challenge_id,
        },
      }),
      requestId,
      prepared.challenge.challenge_id,
    );
  };

  const release = async (
    prepared: BlackoutReleasePreparedConsent,
  ): Promise<BlackoutReleaseReceipt> => {
    validatePrepared(prepared);
    const request = {
      operation_id: blackoutReleaseOperationId,
      request_id: prepared.execution_request_id,
      expected_fence: prepared.authority.fence,
      consent_token: prepared.challenge.consent_token,
      action: { kind: "release_blackout" as const },
    };
    for (let attempt = 0; attempt <= maxReplyLossRetries; attempt += 1) {
      try {
        const response = await options.invoke<unknown>("execute_output_control_v1", { request });
        return validateExecutionResponse(response, prepared);
      } catch (error) {
        if (error instanceof BlackoutReleaseCommandError
          || error instanceof BlackoutReleaseProtocolError
          || attempt >= maxReplyLossRetries) {
          throw error;
        }
      }
    }
    throw new BlackoutReleaseProtocolError("reply-loss retry state");
  };

  return {
    prepareRelease,
    consentStatus,
    release,
  };
}
