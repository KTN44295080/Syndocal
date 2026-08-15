/**
 * Strict renderer-side lane for the canonical Timeline Play/Pause runtime
 * command. It never accepts an owner/window/principal, never opens a generic
 * project transaction, and refreshes the server-minted capability only after
 * a typed stale-fence rejection.
 */

import type { FrontendTauriInvoke } from "./tauriInvokeCommands";

export const timelineTransportSetPlayingOperationId =
  "syndocal.runtime.timeline.transport.set_playing.v1";

export type TimelineTransportRuntimeErrorCode =
  | "invalid_request"
  | "forbidden"
  | "stale_fence"
  | "conflict"
  | "busy"
  | "overloaded"
  | "publication_failed"
  | "internal";

export type TimelineTransportProjectFence = {
  process_incarnation: number;
  session_incarnation: number;
  project_epoch: number;
  project_revision: number;
  project_checkpoint_hash: string;
  project_publication_generation: number;
};

export type TimelineTransportRuntimeFence = {
  project: TimelineTransportProjectFence;
  domain: "timeline.transport";
  source_runtime_epoch: number;
  source_runtime_generation: number;
};

type RuntimeAuthorityBundle = {
  operation_id: string;
  authority_id: string;
  fence: TimelineTransportRuntimeFence;
};

export type TimelineTransportRuntimeRequest = {
  operation_id: string;
  authority_id: string;
  request_id: number;
  expected_fence: TimelineTransportRuntimeFence;
  payload: { playing: boolean };
};

type RuntimeCommandResponse = {
  kind: "receipt" | "rejected";
  result: Record<string, unknown>;
};

const maxSafeInteger = Number.MAX_SAFE_INTEGER;
const errorCodes = new Set<TimelineTransportRuntimeErrorCode>([
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

// Sixteen bytes encode to 22 unpadded base64url characters. The final
// character carries only two data bits, so canonical encodings end in AQgw.
const isAuthorityId = (value: unknown): value is string =>
  typeof value === "string"
  && /^[A-Za-z0-9_-]{21}[AQgw]$/.test(value);

const isProjectFence = (value: unknown): value is TimelineTransportProjectFence => {
  if (!isRecord(value)) return false;
  return isNonZeroSafeInteger(value.process_incarnation)
    && isNonZeroSafeInteger(value.session_incarnation)
    && isCounter(value.project_epoch)
    && isCounter(value.project_revision)
    && isCounter(value.project_publication_generation)
    && typeof value.project_checkpoint_hash === "string"
    && /^[0-9a-f]{64}$/.test(value.project_checkpoint_hash);
};

export const isTimelineTransportRuntimeFence = (
  value: unknown,
): value is TimelineTransportRuntimeFence => {
  if (!isRecord(value)) return false;
  return isProjectFence(value.project)
    && value.domain === "timeline.transport"
    && isNonZeroSafeInteger(value.source_runtime_epoch)
    && isNonZeroSafeInteger(value.source_runtime_generation);
};

const fencesEqual = (left: TimelineTransportRuntimeFence, right: TimelineTransportRuntimeFence) =>
  left.project.process_incarnation === right.project.process_incarnation
  && left.project.session_incarnation === right.project.session_incarnation
  && left.project.project_epoch === right.project.project_epoch
  && left.project.project_revision === right.project.project_revision
  && left.project.project_checkpoint_hash === right.project.project_checkpoint_hash
  && left.project.project_publication_generation === right.project.project_publication_generation
  && left.domain === right.domain
  && left.source_runtime_epoch === right.source_runtime_epoch
  && left.source_runtime_generation === right.source_runtime_generation;

const nextAuthorityPair = (fence: TimelineTransportRuntimeFence) => {
  if (fence.source_runtime_generation < maxSafeInteger) {
    return {
      epoch: fence.source_runtime_epoch,
      generation: fence.source_runtime_generation + 1,
    };
  }
  if (fence.source_runtime_epoch < maxSafeInteger) {
    return { epoch: fence.source_runtime_epoch + 1, generation: 1 };
  }
  return null;
};

export class TimelineTransportRuntimeCommandError extends Error {
  readonly code: TimelineTransportRuntimeErrorCode;

  constructor(code: TimelineTransportRuntimeErrorCode) {
    super(`Timeline transport command was rejected (${code}).`);
    this.name = "TimelineTransportRuntimeCommandError";
    this.code = code;
  }
}

export class TimelineTransportRuntimeProtocolError extends Error {
  constructor(part: string) {
    super(`Timeline transport runtime returned an invalid ${part}.`);
    this.name = "TimelineTransportRuntimeProtocolError";
  }
}

const invalidResponse = (part: string) => new TimelineTransportRuntimeProtocolError(part);

const validateAuthorityBundle = (value: unknown): RuntimeAuthorityBundle => {
  if (!isRecord(value)
    || value.operation_id !== timelineTransportSetPlayingOperationId
    || !isAuthorityId(value.authority_id)
    || !isTimelineTransportRuntimeFence(value.fence)) {
    throw invalidResponse("authority bundle");
  }
  return {
    operation_id: value.operation_id,
    authority_id: value.authority_id,
    fence: value.fence,
  };
};

const validateResponse = (
  value: unknown,
  request: TimelineTransportRuntimeRequest,
): void => {
  if (!isRecord(value)
    || (value.kind !== "receipt" && value.kind !== "rejected")
    || !isRecord(value.result)
    || !isNonZeroSafeInteger(request.request_id)
    || !isAuthorityId(request.authority_id)
    || typeof request.payload.playing !== "boolean") {
    throw invalidResponse("response");
  }
  const response = value as RuntimeCommandResponse;
  if (response.kind === "rejected") {
    const error = isRecord(response.result.error) ? response.result.error : null;
    if (response.result.operation_id !== timelineTransportSetPlayingOperationId
      || response.result.request_id !== request.request_id
      || !isTimelineTransportRuntimeFence(response.result.fence_before)
      || !fencesEqual(response.result.fence_before, request.expected_fence)
      || !error
      || typeof error.code !== "string"
      || !errorCodes.has(error.code as TimelineTransportRuntimeErrorCode)) {
      throw invalidResponse("rejection");
    }
    throw new TimelineTransportRuntimeCommandError(error.code as TimelineTransportRuntimeErrorCode);
  }
  if (response.result.operation_id !== timelineTransportSetPlayingOperationId
    || response.result.request_id !== request.request_id
    || !isTimelineTransportRuntimeFence(response.result.fence_before)
    || !fencesEqual(response.result.fence_before, request.expected_fence)
    || response.result.requested_playing !== request.payload.playing
    || !isNonZeroSafeInteger(response.result.epoch_after)
    || !isNonZeroSafeInteger(response.result.generation_after)
    || typeof response.result.shape_sha256 !== "string"
    || !/^[0-9a-f]{64}$/.test(response.result.shape_sha256)
    || (response.result.outcome !== "applied" && response.result.outcome !== "no_op")) {
    throw invalidResponse("receipt");
  }
  if (response.result.outcome === "no_op") {
    if (response.result.epoch_after !== request.expected_fence.source_runtime_epoch
      || response.result.generation_after !== request.expected_fence.source_runtime_generation) {
      throw invalidResponse("NoOp receipt");
    }
    return;
  }
  const expected = nextAuthorityPair(request.expected_fence);
  if (!expected
    || response.result.epoch_after !== expected.epoch
    || response.result.generation_after !== expected.generation) {
    throw invalidResponse("Applied receipt");
  }
};

let fallbackRequestCounter = 0;

export const nextTimelineTransportRequestId = (): number => {
  const values = new Uint32Array(1);
  if (globalThis.crypto?.getRandomValues) globalThis.crypto.getRandomValues(values);
  else {
    fallbackRequestCounter = (fallbackRequestCounter + 1) >>> 0;
    values[0] = fallbackRequestCounter;
  }
  // 21 high bits plus 32 random/monotonic low bits remains exact in a JS
  // number and leaves zero outside the generated range.
  const high = (Date.now() % 2_097_151) + 1;
  return high * 4_294_967_296 + values[0];
};

const nextDistinctRequestId = (previous: number | undefined): number => {
  const candidate = nextTimelineTransportRequestId();
  if (candidate !== previous) return candidate;
  return candidate < maxSafeInteger ? candidate + 1 : candidate - 1;
};

type Deferred = { resolve: () => void; reject: (error: unknown) => void };

export type TimelineTransportRuntimeControllerOptions = {
  invoke: FrontendTauriInvoke;
  /** Defaults to two fresh authority attempts, only for typed stale fences. */
  maxAttempts?: number;
  /** Defaults to one exact request resend after a generic IPC reply loss. */
  maxReplyLossRetries?: number;
};

/**
 * A single latest-intent lane deliberately coalesces rapid opposite button or
 * shortcut input. If Play is already on the worker, a subsequent Pause waits
 * behind it; if it is merely queued, the final desired value replaces it.
 */
export function createTimelineTransportRuntimeController(
  options: TimelineTransportRuntimeControllerOptions,
) {
  const maxAttempts = Math.max(1, Math.floor(options.maxAttempts ?? 2));
  const maxReplyLossRetries = Math.max(0, Math.floor(options.maxReplyLossRetries ?? 1));
  let desired: boolean | null = null;
  let draining = false;
  let deferreds: Deferred[] = [];

  const sendExactRequest = async (request: TimelineTransportRuntimeRequest): Promise<void> => {
    for (let replyAttempt = 0; replyAttempt <= maxReplyLossRetries; replyAttempt += 1) {
      try {
        const response = await options.invoke<unknown>(
          "set_timeline_transport_playing_runtime_v1",
          { request },
        );
        validateResponse(response, request);
        return;
      } catch (error) {
        // Typed server rejections and malformed/unknown protocol replies must
        // never be replayed. Only an untyped IPC failure can have applied B
        // while losing its reply, so it gets this exact request object once.
        if (error instanceof TimelineTransportRuntimeCommandError
          || error instanceof TimelineTransportRuntimeProtocolError
          || replyAttempt >= maxReplyLossRetries) {
          throw error;
        }
      }
    }
    throw new Error("Timeline transport runtime reply retry exhausted.");
  };

  const dispatch = async (playing: boolean) => {
    let finalError: unknown = null;
    let previousRequestId: number | undefined;
    for (let attempt = 0; attempt < maxAttempts; attempt += 1) {
      const authority = validateAuthorityBundle(
        await options.invoke<unknown>("query_timeline_transport_authority_v1"),
      );
      const request: TimelineTransportRuntimeRequest = {
        operation_id: timelineTransportSetPlayingOperationId,
        authority_id: authority.authority_id,
        request_id: nextDistinctRequestId(previousRequestId),
        expected_fence: authority.fence,
        payload: { playing },
      };
      previousRequestId = request.request_id;
      try {
        await sendExactRequest(request);
        return;
      } catch (error) {
        finalError = error;
        if (!(error instanceof TimelineTransportRuntimeCommandError)
          || error.code !== "stale_fence"
          || attempt + 1 >= maxAttempts) {
          throw error;
        }
      }
    }
    throw finalError ?? new Error("Timeline transport runtime retry exhausted.");
  };

  const drain = async () => {
    let finalError: unknown = null;
    while (desired !== null) {
      const playing = desired;
      desired = null;
      try {
        await dispatch(playing);
        finalError = null;
      } catch (error) {
        finalError = error;
      }
    }
    const settled = deferreds;
    deferreds = [];
    draining = false;
    if (finalError) {
      settled.forEach(({ reject }) => reject(finalError));
    } else {
      settled.forEach(({ resolve }) => resolve());
    }
  };

  const setPlaying = (playing: boolean): Promise<void> => new Promise((resolve, reject) => {
    desired = playing;
    deferreds.push({ resolve, reject });
    if (!draining) {
      draining = true;
      void drain();
    }
  });

  return { setPlaying };
}
