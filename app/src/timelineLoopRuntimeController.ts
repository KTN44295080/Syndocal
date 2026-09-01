/**
 * Strict renderer-side lane for root Timeline loop runtime commands. This is
 * deliberately a separate wire family from Timeline Play/Pause: it obtains a
 * `timeline.loop` capability, sends exactly one typed action, then accepts
 * success only after the caller applies an exact canonical runtime snapshot.
 */

import type { FrontendTauriInvoke } from "./tauriInvokeCommands";

export const timelineLoopRuntimeCommitOperationId =
  "syndocal.runtime.timeline.loop.commit.v1";

export type TimelineLoopRuntimeErrorCode =
  | "invalid_request"
  | "forbidden"
  | "stale_fence"
  | "conflict"
  | "busy"
  | "overloaded"
  | "publication_failed"
  | "internal";

export type TimelineLoopRuntimeAction =
  | { kind: "set_enabled"; enabled: boolean }
  | { kind: "scale"; scale: "half" | "double" };

export type TimelineLoopProjectFence = {
  process_incarnation: number;
  session_incarnation: number;
  project_epoch: number;
  project_revision: number;
  project_checkpoint_hash: string;
  project_publication_generation: number;
};

export type TimelineLoopRuntimeFence = {
  project: TimelineLoopProjectFence;
  domain: "timeline.loop";
  source_runtime_epoch: number;
  source_runtime_generation: number;
  /** Exact canonical loop watermark captured with the authority. */
  source_loop_generation: number;
  /** Exact canonical Follow watermark captured with the authority. */
  source_follow_generation: number;
};

/** A renderer enqueue is bound to one exact E/R/H image and local read. */
export type TimelineLoopRuntimeScope = {
  project_epoch: number;
  project_revision: number;
  checkpoint_hash: string;
  project_read_generation: number;
};

type RuntimeAuthorityBundle = {
  operation_id: string;
  authority_id: string;
  fence: TimelineLoopRuntimeFence;
};

export type TimelineLoopRuntimeRequest = {
  operation_id: string;
  authority_id: string;
  request_id: number;
  expected_fence: TimelineLoopRuntimeFence;
  action: TimelineLoopRuntimeAction;
};

/** A receipt remains pending until `refreshCanonicalSnapshot` proves it. */
export type TimelineLoopRuntimeAcknowledgement = {
  requestedAction: TimelineLoopRuntimeAction;
  fenceBefore: TimelineLoopRuntimeFence;
  scope: TimelineLoopRuntimeScope;
  epochAfter: number;
  generationAfter: number;
  loopGenerationAfter: number;
  followGenerationAfter: number;
  outcome: "applied" | "no_op";
};

type RuntimeCommandResponse = {
  kind: "receipt" | "rejected";
  result: Record<string, unknown>;
};

const maxSafeInteger = Number.MAX_SAFE_INTEGER;
const errorCodes = new Set<TimelineLoopRuntimeErrorCode>([
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
// character carries two bits, so only AQgw is canonical.
const isAuthorityId = (value: unknown): value is string =>
  typeof value === "string" && /^[A-Za-z0-9_-]{21}[AQgw]$/.test(value);

const isProjectFence = (value: unknown): value is TimelineLoopProjectFence => {
  if (!isRecord(value)) return false;
  return isNonZeroSafeInteger(value.process_incarnation)
    && isNonZeroSafeInteger(value.session_incarnation)
    && isCounter(value.project_epoch)
    && isCounter(value.project_revision)
    && isCounter(value.project_publication_generation)
    && typeof value.project_checkpoint_hash === "string"
    && /^[0-9a-f]{64}$/.test(value.project_checkpoint_hash);
};

const isScope = (value: unknown): value is TimelineLoopRuntimeScope => {
  if (!isRecord(value)) return false;
  return isCounter(value.project_epoch)
    && isCounter(value.project_revision)
    && typeof value.checkpoint_hash === "string"
    && /^[0-9a-f]{64}$/.test(value.checkpoint_hash)
    && isCounter(value.project_read_generation);
};

export const isTimelineLoopRuntimeFence = (
  value: unknown,
): value is TimelineLoopRuntimeFence => {
  if (!isRecord(value)) return false;
  return isProjectFence(value.project)
    && value.domain === "timeline.loop"
    && isNonZeroSafeInteger(value.source_runtime_epoch)
    && isNonZeroSafeInteger(value.source_runtime_generation)
    && isCounter(value.source_loop_generation)
    && isCounter(value.source_follow_generation);
};

const isAction = (value: unknown): value is TimelineLoopRuntimeAction => isRecord(value)
  && ((value.kind === "set_enabled" && typeof value.enabled === "boolean")
    || (value.kind === "scale" && (value.scale === "half" || value.scale === "double")));

const actionsEqual = (left: TimelineLoopRuntimeAction, right: TimelineLoopRuntimeAction) =>
  left.kind === right.kind
  && (left.kind === "set_enabled"
    ? left.enabled === (right as Extract<TimelineLoopRuntimeAction, { kind: "set_enabled" }>).enabled
    : left.scale === (right as Extract<TimelineLoopRuntimeAction, { kind: "scale" }>).scale);

const fencesEqual = (left: TimelineLoopRuntimeFence, right: TimelineLoopRuntimeFence) =>
  left.project.process_incarnation === right.project.process_incarnation
  && left.project.session_incarnation === right.project.session_incarnation
  && left.project.project_epoch === right.project.project_epoch
  && left.project.project_revision === right.project.project_revision
  && left.project.project_checkpoint_hash === right.project.project_checkpoint_hash
  && left.project.project_publication_generation === right.project.project_publication_generation
  && left.domain === right.domain
  && left.source_runtime_epoch === right.source_runtime_epoch
  && left.source_runtime_generation === right.source_runtime_generation
  && left.source_loop_generation === right.source_loop_generation
  && left.source_follow_generation === right.source_follow_generation;

const scopesEqual = (left: TimelineLoopRuntimeScope, right: TimelineLoopRuntimeScope) =>
  left.project_epoch === right.project_epoch
  && left.project_revision === right.project_revision
  && left.checkpoint_hash === right.checkpoint_hash
  && left.project_read_generation === right.project_read_generation;

const fenceProjectMatchesScope = (
  fence: TimelineLoopRuntimeFence,
  scope: TimelineLoopRuntimeScope,
) => fence.project.project_epoch === scope.project_epoch
  && fence.project.project_revision === scope.project_revision
  && fence.project.project_checkpoint_hash === scope.checkpoint_hash;

const nextAuthorityPair = (fence: TimelineLoopRuntimeFence) => {
  if (fence.source_runtime_generation < maxSafeInteger) {
    return { epoch: fence.source_runtime_epoch, generation: fence.source_runtime_generation + 1 };
  }
  if (fence.source_runtime_epoch < maxSafeInteger) {
    return { epoch: fence.source_runtime_epoch + 1, generation: 1 };
  }
  return null;
};

export class TimelineLoopRuntimeCommandError extends Error {
  readonly code: TimelineLoopRuntimeErrorCode;

  constructor(code: TimelineLoopRuntimeErrorCode) {
    super(`Timeline loop command was rejected (${code}).`);
    this.name = "TimelineLoopRuntimeCommandError";
    this.code = code;
  }
}

export class TimelineLoopRuntimeProtocolError extends Error {
  constructor(part: string) {
    super(`Timeline loop runtime returned an invalid ${part}.`);
    this.name = "TimelineLoopRuntimeProtocolError";
  }
}

export class TimelineLoopRuntimeScopeError extends Error {
  constructor() {
    super("Timeline loop project scope changed; the original action was discarded.");
    this.name = "TimelineLoopRuntimeScopeError";
  }
}

const invalidResponse = (part: string) => new TimelineLoopRuntimeProtocolError(part);

const validateAuthorityBundle = (value: unknown): RuntimeAuthorityBundle => {
  if (!isRecord(value)
    || value.operation_id !== timelineLoopRuntimeCommitOperationId
    || !isAuthorityId(value.authority_id)
    || !isTimelineLoopRuntimeFence(value.fence)) {
    throw invalidResponse("authority bundle");
  }
  return { operation_id: value.operation_id, authority_id: value.authority_id, fence: value.fence };
};

const validateResponse = (
  value: unknown,
  request: TimelineLoopRuntimeRequest,
  scope: TimelineLoopRuntimeScope,
): TimelineLoopRuntimeAcknowledgement => {
  if (!isRecord(value)
    || (value.kind !== "receipt" && value.kind !== "rejected")
    || !isRecord(value.result)
    || !isNonZeroSafeInteger(request.request_id)
    || !isAuthorityId(request.authority_id)
    || !isAction(request.action)) {
    throw invalidResponse("response");
  }
  const response = value as RuntimeCommandResponse;
  if (response.kind === "rejected") {
    const error = isRecord(response.result.error) ? response.result.error : null;
    if (response.result.operation_id !== timelineLoopRuntimeCommitOperationId
      || response.result.request_id !== request.request_id
      || !isTimelineLoopRuntimeFence(response.result.fence_before)
      || !fencesEqual(response.result.fence_before, request.expected_fence)
      || !error
      || typeof error.code !== "string"
      || !errorCodes.has(error.code as TimelineLoopRuntimeErrorCode)) {
      throw invalidResponse("rejection");
    }
    throw new TimelineLoopRuntimeCommandError(error.code as TimelineLoopRuntimeErrorCode);
  }
  if (response.result.operation_id !== timelineLoopRuntimeCommitOperationId
    || response.result.request_id !== request.request_id
    || !isTimelineLoopRuntimeFence(response.result.fence_before)
    || !fencesEqual(response.result.fence_before, request.expected_fence)
    || !isAction(response.result.requested_action)
    || !actionsEqual(response.result.requested_action, request.action)
    || !isNonZeroSafeInteger(response.result.epoch_after)
    || !isNonZeroSafeInteger(response.result.generation_after)
    || !isCounter(response.result.loop_generation_after)
    || !isCounter(response.result.follow_generation_after)
    || typeof response.result.shape_sha256 !== "string"
    || !/^[0-9a-f]{64}$/.test(response.result.shape_sha256)
    || (response.result.outcome !== "applied" && response.result.outcome !== "no_op")) {
    throw invalidResponse("receipt");
  }
  if (response.result.outcome === "no_op") {
    if (response.result.epoch_after !== request.expected_fence.source_runtime_epoch
      || response.result.generation_after !== request.expected_fence.source_runtime_generation
      || response.result.loop_generation_after !== request.expected_fence.source_loop_generation
      || response.result.follow_generation_after !== request.expected_fence.source_follow_generation) {
      throw invalidResponse("NoOp receipt");
    }
  } else {
    const expected = nextAuthorityPair(request.expected_fence);
    if (!expected
      || response.result.epoch_after !== expected.epoch
      || response.result.generation_after !== expected.generation
      || response.result.loop_generation_after < request.expected_fence.source_loop_generation
      || response.result.follow_generation_after < request.expected_fence.source_follow_generation) {
      throw invalidResponse("Applied receipt");
    }
  }
  return {
    requestedAction: request.action,
    fenceBefore: request.expected_fence,
    scope,
    epochAfter: response.result.epoch_after,
    generationAfter: response.result.generation_after,
    loopGenerationAfter: response.result.loop_generation_after,
    followGenerationAfter: response.result.follow_generation_after,
    outcome: response.result.outcome,
  };
};

let fallbackRequestCounter = 0;

export const nextTimelineLoopRuntimeRequestId = (): number => {
  const values = new Uint32Array(1);
  if (globalThis.crypto?.getRandomValues) globalThis.crypto.getRandomValues(values);
  else {
    fallbackRequestCounter = (fallbackRequestCounter + 1) >>> 0;
    values[0] = fallbackRequestCounter;
  }
  const high = (Date.now() % 2_097_151) + 1;
  return high * 4_294_967_296 + values[0];
};

const nextDistinctRequestId = (previous: number | undefined): number => {
  const candidate = nextTimelineLoopRuntimeRequestId();
  if (candidate !== previous) return candidate;
  return candidate < maxSafeInteger ? candidate + 1 : candidate - 1;
};

type Deferred = { resolve: () => void; reject: (error: unknown) => void };

export type TimelineLoopRuntimeControllerOptions = {
  invoke: FrontendTauriInvoke;
  captureScope: () => TimelineLoopRuntimeScope;
  /** Must apply the exact authority-bound snapshot that proves this receipt. */
  refreshCanonicalSnapshot: (acknowledgement: TimelineLoopRuntimeAcknowledgement) => Promise<void>;
  /** Defaults to two fresh authority attempts, solely after stale_fence. */
  maxAttempts?: number;
  /** Defaults to one exact request resend after an untyped IPC reply loss. */
  maxReplyLossRetries?: number;
};

/**
 * Serialize every root-loop action in FIFO order. Unlike a boolean transport
 * intent, Scale is relative to the admitted canonical state and cannot be
 * safely coalesced with a preceding SetEnabled or Scale action.
 */
export function createTimelineLoopRuntimeController(options: TimelineLoopRuntimeControllerOptions) {
  const maxAttempts = Math.max(1, Math.floor(options.maxAttempts ?? 2));
  const maxReplyLossRetries = Math.max(0, Math.floor(options.maxReplyLossRetries ?? 1));
  let draining = false;
  type Intent = { action: TimelineLoopRuntimeAction; scope: TimelineLoopRuntimeScope; deferred: Deferred };
  const queued: Intent[] = [];

  const captureScope = (): TimelineLoopRuntimeScope => {
    const scope = options.captureScope();
    if (!isScope(scope)) throw invalidResponse("project scope");
    return scope;
  };
  const requireCurrentScope = (scope: TimelineLoopRuntimeScope) => {
    if (!scopesEqual(scope, captureScope())) throw new TimelineLoopRuntimeScopeError();
  };
  const sendExactRequest = async (
    request: TimelineLoopRuntimeRequest,
    scope: TimelineLoopRuntimeScope,
  ): Promise<TimelineLoopRuntimeAcknowledgement> => {
    for (let replyAttempt = 0; replyAttempt <= maxReplyLossRetries; replyAttempt += 1) {
      try {
        requireCurrentScope(scope);
        const response = await options.invoke<unknown>("commit_timeline_loop_runtime_v1", { request });
        return validateResponse(response, request, scope);
      } catch (error) {
        // Only an untyped reply loss can have applied the exact command while
        // withholding its receipt. Typed/protocol/scope failures never replay.
        if (error instanceof TimelineLoopRuntimeCommandError
          || error instanceof TimelineLoopRuntimeProtocolError
          || error instanceof TimelineLoopRuntimeScopeError
          || replyAttempt >= maxReplyLossRetries) {
          throw error;
        }
      }
    }
    throw new Error("Timeline loop runtime reply retry exhausted.");
  };
  const dispatch = async (intent: Intent) => {
    let finalError: unknown = null;
    let previousRequestId: number | undefined;
    for (let attempt = 0; attempt < maxAttempts; attempt += 1) {
      requireCurrentScope(intent.scope);
      const authority = validateAuthorityBundle(
        await options.invoke<unknown>("query_timeline_loop_runtime_authority_v1"),
      );
      if (!fenceProjectMatchesScope(authority.fence, intent.scope)) {
        throw new TimelineLoopRuntimeScopeError();
      }
      requireCurrentScope(intent.scope);
      const request: TimelineLoopRuntimeRequest = {
        operation_id: timelineLoopRuntimeCommitOperationId,
        authority_id: authority.authority_id,
        request_id: nextDistinctRequestId(previousRequestId),
        expected_fence: authority.fence,
        action: intent.action,
      };
      previousRequestId = request.request_id;
      try {
        const acknowledgement = await sendExactRequest(request, intent.scope);
        requireCurrentScope(intent.scope);
        // A receipt admits native B but does not itself change renderer state.
        // A failed canonical read stays visible and never triggers a mutation.
        await options.refreshCanonicalSnapshot(acknowledgement);
        requireCurrentScope(intent.scope);
        return;
      } catch (error) {
        finalError = error;
        if (!(error instanceof TimelineLoopRuntimeCommandError)
          || error.code !== "stale_fence"
          || attempt + 1 >= maxAttempts) {
          throw error;
        }
      }
    }
    throw finalError ?? new Error("Timeline loop runtime retry exhausted.");
  };
  const drain = async () => {
    while (queued.length > 0) {
      const intent = queued.shift();
      if (!intent) continue;
      try {
        requireCurrentScope(intent.scope);
        await dispatch(intent);
        intent.deferred.resolve();
      } catch (error) {
        intent.deferred.reject(error);
      }
    }
    draining = false;
  };
  const enqueue = (action: TimelineLoopRuntimeAction): Promise<void> => {
    let scope: TimelineLoopRuntimeScope;
    try {
      scope = captureScope();
    } catch (error) {
      return Promise.reject(error);
    }
    return new Promise((resolve, reject) => {
      queued.push({ action, scope, deferred: { resolve, reject } });
      if (!draining) {
        draining = true;
        void drain();
      }
    });
  };
  return {
    setEnabled: (enabled: boolean) => enqueue({ kind: "set_enabled", enabled }),
    scale: (scale: "half" | "double") => enqueue({ kind: "scale", scale }),
  };
}
