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

/**
 * A renderer enqueue belongs to exactly one project identity and one local
 * read generation.  The latter matters because a same E/R/H image can still
 * be superseded while the coordinator applies an asynchronous replacement.
 */
export type TimelineTransportRuntimeScope = {
  project_epoch: number;
  project_revision: number;
  checkpoint_hash: string;
  project_read_generation: number;
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

/**
 * The receipt is not a renderer-success boundary on its own.  The caller
 * must apply a canonical snapshot that proves this exact acknowledged state
 * before the originating Play/Pause action is allowed to report success.
 */
export type TimelineTransportRuntimeAcknowledgement = {
  requestedPlaying: boolean;
  fenceBefore: TimelineTransportRuntimeFence;
  scope: TimelineTransportRuntimeScope;
  epochAfter: number;
  generationAfter: number;
  outcome: "applied" | "no_op";
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

const isScope = (value: unknown): value is TimelineTransportRuntimeScope => {
  if (!isRecord(value)) return false;
  return isCounter(value.project_epoch)
    && isCounter(value.project_revision)
    && typeof value.checkpoint_hash === "string"
    && /^[0-9a-f]{64}$/.test(value.checkpoint_hash)
    && isCounter(value.project_read_generation);
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

const scopesEqual = (
  left: TimelineTransportRuntimeScope,
  right: TimelineTransportRuntimeScope,
) => left.project_epoch === right.project_epoch
  && left.project_revision === right.project_revision
  && left.checkpoint_hash === right.checkpoint_hash
  && left.project_read_generation === right.project_read_generation;

const fenceProjectMatchesScope = (
  fence: TimelineTransportRuntimeFence,
  scope: TimelineTransportRuntimeScope,
) => fence.project.project_epoch === scope.project_epoch
  && fence.project.project_revision === scope.project_revision
  && fence.project.project_checkpoint_hash === scope.checkpoint_hash;

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

export class TimelineTransportRuntimeScopeError extends Error {
  constructor() {
    super("Timeline transport project scope changed; the original action was discarded.");
    this.name = "TimelineTransportRuntimeScopeError";
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
  scope: TimelineTransportRuntimeScope,
): TimelineTransportRuntimeAcknowledgement => {
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
    return {
      requestedPlaying: request.payload.playing,
      fenceBefore: request.expected_fence,
      scope,
      epochAfter: response.result.epoch_after,
      generationAfter: response.result.generation_after,
      outcome: "no_op",
    };
  }
  const expected = nextAuthorityPair(request.expected_fence);
  if (!expected
    || response.result.epoch_after !== expected.epoch
    || response.result.generation_after !== expected.generation) {
    throw invalidResponse("Applied receipt");
  }
  return {
    requestedPlaying: request.payload.playing,
    fenceBefore: request.expected_fence,
    scope,
    epochAfter: response.result.epoch_after,
    generationAfter: response.result.generation_after,
    outcome: "applied",
  };
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
  /** Captured synchronously for every enqueue; no origin is inferred later. */
  captureScope: () => TimelineTransportRuntimeScope;
  /**
   * Applies only an authority-bound snapshot which proves the exact receipt.
   * A rejected/invalid/stale canonical read rejects the originating action;
   * the controller never synthesizes a renderer-side playing value.
   */
  refreshCanonicalSnapshot: (
    acknowledgement: TimelineTransportRuntimeAcknowledgement,
  ) => Promise<void>;
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
  let draining = false;
  type IntentGroup = {
    playing: boolean;
    scope: TimelineTransportRuntimeScope;
    deferreds: Deferred[];
  };
  const queued: IntentGroup[] = [];

  const captureScope = (): TimelineTransportRuntimeScope => {
    const scope = options.captureScope();
    if (!isScope(scope)) throw invalidResponse("project scope");
    return scope;
  };
  const scopeIsCurrent = (scope: TimelineTransportRuntimeScope) =>
    scopesEqual(scope, captureScope());
  const requireCurrentScope = (scope: TimelineTransportRuntimeScope) => {
    if (!scopeIsCurrent(scope)) throw new TimelineTransportRuntimeScopeError();
  };

  const sendExactRequest = async (
    request: TimelineTransportRuntimeRequest,
    scope: TimelineTransportRuntimeScope,
  ): Promise<TimelineTransportRuntimeAcknowledgement> => {
    for (let replyAttempt = 0; replyAttempt <= maxReplyLossRetries; replyAttempt += 1) {
      try {
        // This also fences an exact reply-loss resend: it is the same A
        // request, but may not be delivered to B after a project switch.
        requireCurrentScope(scope);
        const response = await options.invoke<unknown>(
          "set_timeline_transport_playing_runtime_v1",
          { request },
        );
        return validateResponse(response, request, scope);
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

  const dispatch = async (group: IntentGroup) => {
    let finalError: unknown = null;
    let previousRequestId: number | undefined;
    for (let attempt = 0; attempt < maxAttempts; attempt += 1) {
      // Replacement B must never receive a request intended for A. Recheck
      // before every query/retry rather than treating a cached authority as a
      // current project capability.
      requireCurrentScope(group.scope);
      const authority = validateAuthorityBundle(
        await options.invoke<unknown>("query_timeline_transport_authority_v1"),
      );
      if (!fenceProjectMatchesScope(authority.fence, group.scope)) {
        throw new TimelineTransportRuntimeScopeError();
      }
      // The authority read crossed an async boundary. Validate A again at
      // the last possible renderer boundary before sending its mutation.
      requireCurrentScope(group.scope);
      const request: TimelineTransportRuntimeRequest = {
        operation_id: timelineTransportSetPlayingOperationId,
        authority_id: authority.authority_id,
        request_id: nextDistinctRequestId(previousRequestId),
        expected_fence: authority.fence,
        payload: { playing: group.playing },
      };
      previousRequestId = request.request_id;
      try {
        const acknowledgement = await sendExactRequest(request, group.scope);
        // If the project moved while the native worker completed, reject the
        // original group. Do not let canonical convergence read/apply B for A.
        requireCurrentScope(group.scope);
        // Keep the receipt retry policy separate from canonical convergence.
        // A receipt has already admitted B, so a failed/stale snapshot read
        // must stay visible and must never issue another mutation implicitly.
        await options.refreshCanonicalSnapshot(acknowledgement);
        // Canonical application can itself await the authority-bound image.
        // Do not report A as successful if B became current during that wait.
        requireCurrentScope(group.scope);
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
    while (queued.length > 0) {
      const group = queued.shift();
      if (!group) continue;
      try {
        // This independently rejects obsolete A groups while allowing an
        // already-enqueued B group to progress immediately afterward.
        requireCurrentScope(group.scope);
        await dispatch(group);
        group.deferreds.forEach(({ resolve }) => resolve());
      } catch (error) {
        group.deferreds.forEach(({ reject }) => reject(error));
      }
    }
    draining = false;
  };

  const setPlaying = (playing: boolean): Promise<void> => {
    let scope: TimelineTransportRuntimeScope;
    try {
      scope = captureScope();
    } catch (error) {
      return Promise.reject(error);
    }
    return new Promise((resolve, reject) => {
      const tail = queued.at(-1);
      // Coalesce only queued intents from the exact same project/read scope.
      // An in-flight group has already left `queued` and always settles on
      // its own canonical result.
      if (tail && scopesEqual(tail.scope, scope)) {
        tail.playing = playing;
        tail.deferreds.push({ resolve, reject });
      } else {
        queued.push({ playing, scope, deferreds: [{ resolve, reject }] });
      }
      if (!draining) {
        draining = true;
        void drain();
      }
    });
  };

  return { setPlaying };
}
