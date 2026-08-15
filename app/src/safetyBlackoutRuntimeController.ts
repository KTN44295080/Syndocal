/**
 * Strict safer-direction renderer lane for the S0 DMX safety latch.
 * The wire request deliberately contains no target/toggle/release field.
 */

import type { FrontendTauriInvoke } from "./tauriInvokeCommands";

export const safetyBlackoutEngageOperationId =
  "syndocal.safety.blackout.engage.v1" as const;

export type SafetyBlackoutRuntimeErrorCode =
  | "invalid_request"
  | "forbidden"
  | "stale_fence"
  | "conflict"
  | "busy"
  | "overloaded"
  | "publication_failed"
  | "internal";

export type SafetyBlackoutEngageRequest = {
  operation_id: typeof safetyBlackoutEngageOperationId;
  request_id: number;
};

const errorCodes = new Set<SafetyBlackoutRuntimeErrorCode>([
  "invalid_request",
  "forbidden",
  "stale_fence",
  "conflict",
  "busy",
  "overloaded",
  "publication_failed",
  "internal",
]);

const isRecord = (value: unknown): value is Record<string, unknown> =>
  Boolean(value) && typeof value === "object" && !Array.isArray(value);
const isNonZeroSafeInteger = (value: unknown): value is number =>
  typeof value === "number" && Number.isSafeInteger(value) && value > 0;

let fallbackRequestCounter = 0;
export const nextSafetyBlackoutRequestId = (): number => {
  const values = new Uint32Array(1);
  if (globalThis.crypto?.getRandomValues) globalThis.crypto.getRandomValues(values);
  else {
    fallbackRequestCounter = (fallbackRequestCounter + 1) >>> 0;
    values[0] = fallbackRequestCounter;
  }
  const high = (Date.now() % 2_097_151) + 1;
  return high * 4_294_967_296 + values[0];
};

export class SafetyBlackoutCommandError extends Error {
  readonly code: SafetyBlackoutRuntimeErrorCode;

  constructor(code: SafetyBlackoutRuntimeErrorCode) {
    super(`Safety blackout engage was rejected (${code}).`);
    this.name = "SafetyBlackoutCommandError";
    this.code = code;
  }
}

export class SafetyBlackoutProtocolError extends Error {
  constructor(part: string) {
    super(`Safety blackout engage returned an invalid ${part}.`);
    this.name = "SafetyBlackoutProtocolError";
  }
}

const validateResponse = (
  value: unknown,
  request: SafetyBlackoutEngageRequest,
): "applied" | "no_op" => {
  if (!isRecord(value)
    || (value.kind !== "receipt" && value.kind !== "rejected")
    || !isRecord(value.result)
    || value.result.operation_id !== safetyBlackoutEngageOperationId
    || value.result.request_id !== request.request_id) {
    throw new SafetyBlackoutProtocolError("response identity");
  }
  if (value.kind === "rejected") {
    const error = isRecord(value.result.error) ? value.result.error : null;
    if (!error || typeof error.code !== "string"
      || !errorCodes.has(error.code as SafetyBlackoutRuntimeErrorCode)) {
      throw new SafetyBlackoutProtocolError("rejection");
    }
    throw new SafetyBlackoutCommandError(error.code as SafetyBlackoutRuntimeErrorCode);
  }
  if (!isNonZeroSafeInteger(value.result.audit_sequence)
    || typeof value.result.shape_sha256 !== "string"
    || !/^[0-9a-f]{64}$/.test(value.result.shape_sha256)
    || (value.result.outcome !== "applied" && value.result.outcome !== "no_op")) {
    throw new SafetyBlackoutProtocolError("receipt");
  }
  return value.result.outcome;
};

export type SafetyBlackoutRuntimeControllerOptions = {
  invoke: FrontendTauriInvoke;
  /** One exact resend is the default reply-loss recovery budget. */
  maxReplyLossRetries?: number;
};

export function createSafetyBlackoutRuntimeController(
  options: SafetyBlackoutRuntimeControllerOptions,
) {
  const maxReplyLossRetries = Math.max(0, Math.floor(options.maxReplyLossRetries ?? 1));

  const engage = async (): Promise<"applied" | "no_op"> => {
    const request: SafetyBlackoutEngageRequest = {
      operation_id: safetyBlackoutEngageOperationId,
      request_id: nextSafetyBlackoutRequestId(),
    };
    for (let attempt = 0; attempt <= maxReplyLossRetries; attempt += 1) {
      try {
        const response = await options.invoke<unknown>("safety_blackout_engage_v1", {
          request,
        });
        return validateResponse(response, request);
      } catch (error) {
        if (error instanceof SafetyBlackoutCommandError
          || error instanceof SafetyBlackoutProtocolError
          || attempt >= maxReplyLossRetries) {
          throw error;
        }
      }
    }
    throw new SafetyBlackoutProtocolError("reply-loss retry state");
  };

  return { engage };
}
