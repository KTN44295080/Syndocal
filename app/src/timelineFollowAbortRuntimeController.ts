/**
 * Strict local runtime-safety lane for Timeline Follow Abort. The native
 * adapter owns principal/window identity and issues one opaque capability for
 * the exact project/output/Follow fence; the renderer can only echo it.
 */

import type { FrontendTauriInvoke } from "./tauriInvokeCommands";

export const timelineFollowAbortOperationId =
  "syndocal.runtime.timeline.follow.abort.v1";

export type TimelineFollowAbortRuntimeErrorCode =
  | "invalid_request"
  | "forbidden"
  | "stale_fence"
  | "conflict"
  | "busy"
  | "overloaded"
  | "publication_failed"
  | "internal";

type ProjectFence = {
  process_incarnation: number;
  session_incarnation: number;
  project_epoch: number;
  project_revision: number;
  project_checkpoint_hash: string;
  project_publication_generation: number;
};

export type TimelineFollowAbortRuntimeFence = {
  project: ProjectFence;
  domain: "timeline.follow.abort";
  output_ownership_epoch: number;
  follow_generation: number;
};

type TimelineFollowAbortAuthorityBundle = {
  operation_id: typeof timelineFollowAbortOperationId;
  authority_id: string;
  fence: TimelineFollowAbortRuntimeFence;
};

export type TimelineFollowAbortRuntimeRequest = {
  operation_id: typeof timelineFollowAbortOperationId;
  authority_id: string;
  request_id: number;
  expected_fence: TimelineFollowAbortRuntimeFence;
};

const errorCodes = new Set<TimelineFollowAbortRuntimeErrorCode>([
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
const isCounter = (value: unknown): value is number =>
  typeof value === "number" && Number.isSafeInteger(value) && value >= 0;
const isAuthorityId = (value: unknown): value is string =>
  typeof value === "string" && /^[A-Za-z0-9_-]{21}[AQgw]$/.test(value);

let fallbackRequestCounter = 0;
const nextFollowAbortRequestId = (): number => {
  const values = new Uint32Array(1);
  if (globalThis.crypto?.getRandomValues) globalThis.crypto.getRandomValues(values);
  else {
    fallbackRequestCounter = (fallbackRequestCounter + 1) >>> 0;
    values[0] = fallbackRequestCounter;
  }
  const high = (Date.now() % 2_097_151) + 1;
  return high * 4_294_967_296 + values[0];
};

const isProjectFence = (value: unknown): value is ProjectFence =>
  isRecord(value)
  && isNonZeroSafeInteger(value.process_incarnation)
  && isNonZeroSafeInteger(value.session_incarnation)
  && isCounter(value.project_epoch)
  && isCounter(value.project_revision)
  && isCounter(value.project_publication_generation)
  && typeof value.project_checkpoint_hash === "string"
  && /^[0-9a-f]{64}$/.test(value.project_checkpoint_hash);

export const isTimelineFollowAbortRuntimeFence = (
  value: unknown,
): value is TimelineFollowAbortRuntimeFence =>
  isRecord(value)
  && isProjectFence(value.project)
  && value.domain === "timeline.follow.abort"
  && isNonZeroSafeInteger(value.output_ownership_epoch)
  && isNonZeroSafeInteger(value.follow_generation);

const fencesEqual = (
  left: TimelineFollowAbortRuntimeFence,
  right: TimelineFollowAbortRuntimeFence,
) => left.project.process_incarnation === right.project.process_incarnation
  && left.project.session_incarnation === right.project.session_incarnation
  && left.project.project_epoch === right.project.project_epoch
  && left.project.project_revision === right.project.project_revision
  && left.project.project_checkpoint_hash === right.project.project_checkpoint_hash
  && left.project.project_publication_generation === right.project.project_publication_generation
  && left.domain === right.domain
  && left.output_ownership_epoch === right.output_ownership_epoch
  && left.follow_generation === right.follow_generation;

export class TimelineFollowAbortRuntimeCommandError extends Error {
  readonly code: TimelineFollowAbortRuntimeErrorCode;

  constructor(code: TimelineFollowAbortRuntimeErrorCode) {
    super(`Timeline Follow abort was rejected (${code}).`);
    this.name = "TimelineFollowAbortRuntimeCommandError";
    this.code = code;
  }
}

export class TimelineFollowAbortRuntimeProtocolError extends Error {
  constructor(part: string) {
    super(`Timeline Follow abort returned an invalid ${part}.`);
    this.name = "TimelineFollowAbortRuntimeProtocolError";
  }
}

const validateAuthority = (value: unknown): TimelineFollowAbortAuthorityBundle => {
  if (!isRecord(value)
    || value.operation_id !== timelineFollowAbortOperationId
    || !isAuthorityId(value.authority_id)
    || !isTimelineFollowAbortRuntimeFence(value.fence)) {
    throw new TimelineFollowAbortRuntimeProtocolError("authority bundle");
  }
  return {
    operation_id: timelineFollowAbortOperationId,
    authority_id: value.authority_id,
    fence: value.fence,
  };
};

const validateResponse = (
  value: unknown,
  request: TimelineFollowAbortRuntimeRequest,
): void => {
  if (!isRecord(value)
    || (value.kind !== "receipt" && value.kind !== "rejected")
    || !isRecord(value.result)) {
    throw new TimelineFollowAbortRuntimeProtocolError("response");
  }
  if (value.result.operation_id !== timelineFollowAbortOperationId
    || value.result.request_id !== request.request_id
    || !isTimelineFollowAbortRuntimeFence(value.result.fence_before)
    || !fencesEqual(value.result.fence_before, request.expected_fence)) {
    throw new TimelineFollowAbortRuntimeProtocolError("response identity");
  }
  if (value.kind === "rejected") {
    const error = isRecord(value.result.error) ? value.result.error : null;
    if (!error || typeof error.code !== "string"
      || !errorCodes.has(error.code as TimelineFollowAbortRuntimeErrorCode)) {
      throw new TimelineFollowAbortRuntimeProtocolError("rejection");
    }
    throw new TimelineFollowAbortRuntimeCommandError(
      error.code as TimelineFollowAbortRuntimeErrorCode,
    );
  }
  if (!isNonZeroSafeInteger(value.result.output_ownership_epoch_after)
    || !isNonZeroSafeInteger(value.result.follow_generation_after)
    || typeof value.result.shape_sha256 !== "string"
    || !/^[0-9a-f]{64}$/.test(value.result.shape_sha256)
    || (value.result.outcome !== "applied" && value.result.outcome !== "no_op")) {
    throw new TimelineFollowAbortRuntimeProtocolError("receipt");
  }
  if (value.result.outcome === "no_op") {
    if (value.result.output_ownership_epoch_after
        !== request.expected_fence.output_ownership_epoch
      || value.result.follow_generation_after !== request.expected_fence.follow_generation) {
      throw new TimelineFollowAbortRuntimeProtocolError("NoOp receipt");
    }
    return;
  }
  if (request.expected_fence.follow_generation >= Number.MAX_SAFE_INTEGER
    || value.result.output_ownership_epoch_after
      !== request.expected_fence.output_ownership_epoch
    || value.result.follow_generation_after
      !== request.expected_fence.follow_generation + 1) {
    throw new TimelineFollowAbortRuntimeProtocolError("Applied receipt");
  }
};

export type TimelineFollowAbortRuntimeControllerOptions = {
  invoke: FrontendTauriInvoke;
  maxAttempts?: number;
  maxReplyLossRetries?: number;
};

export function createTimelineFollowAbortRuntimeController(
  options: TimelineFollowAbortRuntimeControllerOptions,
) {
  const maxAttempts = Math.max(1, Math.floor(options.maxAttempts ?? 2));
  const maxReplyLossRetries = Math.max(0, Math.floor(options.maxReplyLossRetries ?? 1));

  const sendExact = async (request: TimelineFollowAbortRuntimeRequest) => {
    for (let replyAttempt = 0; replyAttempt <= maxReplyLossRetries; replyAttempt += 1) {
      try {
        const response = await options.invoke<unknown>("abort_timeline_follow_runtime_v1", {
          request,
        });
        validateResponse(response, request);
        return;
      } catch (error) {
        if (error instanceof TimelineFollowAbortRuntimeCommandError
          || error instanceof TimelineFollowAbortRuntimeProtocolError
          || replyAttempt >= maxReplyLossRetries) {
          throw error;
        }
      }
    }
  };

  const abort = async (): Promise<void> => {
    let finalError: unknown = null;
    let previousRequestId: number | undefined;
    for (let attempt = 0; attempt < maxAttempts; attempt += 1) {
      const authority = validateAuthority(
        await options.invoke<unknown>("query_timeline_follow_abort_authority_v1"),
      );
      let requestId = nextFollowAbortRequestId();
      if (requestId === previousRequestId) {
        requestId = requestId < Number.MAX_SAFE_INTEGER ? requestId + 1 : requestId - 1;
      }
      previousRequestId = requestId;
      const request: TimelineFollowAbortRuntimeRequest = {
        operation_id: timelineFollowAbortOperationId,
        authority_id: authority.authority_id,
        request_id: requestId,
        expected_fence: authority.fence,
      };
      try {
        await sendExact(request);
        return;
      } catch (error) {
        finalError = error;
        if (!(error instanceof TimelineFollowAbortRuntimeCommandError)
          || error.code !== "stale_fence"
          || attempt + 1 >= maxAttempts) {
          throw error;
        }
      }
    }
    throw finalError ?? new Error("Timeline Follow abort retry exhausted.");
  };

  return { abort };
}
