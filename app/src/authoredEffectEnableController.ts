/**
 * Strict renderer-side contract and latest-intent lane for the first authored
 * control-plane command. This module deliberately accepts no caller owner,
 * principal, window label, path, or generic project-transaction ticket.
 */

export const MAX_SAFE_JAVASCRIPT_INTEGER = Number.MAX_SAFE_INTEGER;
export const authoredSetEffectEnabledOperationId = "syndocal.effects.set_enabled.v1";

export type AuthoredProjectMutationFence = {
  process_incarnation: number;
  session_incarnation: number;
  project_epoch: number;
  project_revision: number;
  project_checkpoint_hash: string;
  project_publication_generation: number;
};

export type AuthoredSetEffectEnabledErrorCode =
  | "invalid_request"
  | "forbidden"
  | "stale_fence"
  | "conflict"
  | "busy"
  | "overloaded"
  | "not_found"
  | "publication_failed"
  | "internal";

const authoredSetEffectEnabledErrorCodes = new Set<AuthoredSetEffectEnabledErrorCode>([
  "invalid_request",
  "forbidden",
  "stale_fence",
  "conflict",
  "busy",
  "overloaded",
  "not_found",
  "publication_failed",
  "internal",
]);

const isRecord = (value: unknown): value is Record<string, unknown> =>
  Boolean(value) && typeof value === "object" && !Array.isArray(value);

const isNonZeroSafeInteger = (value: unknown): value is number =>
  Number.isSafeInteger(value) && (value as number) > 0;

const isCounter = (value: unknown): value is number =>
  Number.isSafeInteger(value) && (value as number) >= 0;

export const isAuthoredSetEffectEnabledErrorCode = (
  value: unknown,
): value is AuthoredSetEffectEnabledErrorCode =>
  typeof value === "string" && authoredSetEffectEnabledErrorCodes.has(value as AuthoredSetEffectEnabledErrorCode);

export const isAuthoredProjectMutationFence = (
  value: unknown,
): value is AuthoredProjectMutationFence => {
  if (!isRecord(value)) return false;
  return isNonZeroSafeInteger(value.process_incarnation)
    && isNonZeroSafeInteger(value.session_incarnation)
    && isCounter(value.project_epoch)
    && isCounter(value.project_revision)
    && isCounter(value.project_publication_generation)
    && typeof value.project_checkpoint_hash === "string"
    && /^[0-9a-f]{64}$/.test(value.project_checkpoint_hash);
};

export const authoredProjectMutationFencesEqual = (
  left: AuthoredProjectMutationFence,
  right: AuthoredProjectMutationFence,
) => left.process_incarnation === right.process_incarnation
  && left.session_incarnation === right.session_incarnation
  && left.project_epoch === right.project_epoch
  && left.project_revision === right.project_revision
  && left.project_checkpoint_hash === right.project_checkpoint_hash
  && left.project_publication_generation === right.project_publication_generation;

/** A fixed-code terminal/rejection response from the strict Rust DTO. */
export class AuthoredSetEffectEnabledCommandError extends Error {
  readonly code: AuthoredSetEffectEnabledErrorCode;

  constructor(code: AuthoredSetEffectEnabledErrorCode, terminal: boolean) {
    super(`Effect enable command ${terminal ? "failed" : "was rejected"} (${code}).`);
    this.name = "AuthoredSetEffectEnabledCommandError";
    this.code = code;
  }
}

const invalidReceipt = (kind: "rejection" | "terminal" | "failure" | "outcome") =>
  new Error(`Effect enable request returned an invalid ${kind} receipt.`);

/**
 * Fail-closed validation of every renderer-visible response discriminator and
 * numeric identity before it changes UI state. The response is deliberately
 * `unknown`: a TypeScript cast at the IPC boundary is not validation.
 */
export const validateAuthoredSetEffectEnabledResponse = (
  response: unknown,
  requestId: number,
  fence: AuthoredProjectMutationFence,
  effectId: number,
  enabled: boolean,
): void => {
  if (!isNonZeroSafeInteger(requestId)
    || !isNonZeroSafeInteger(effectId)
    || typeof enabled !== "boolean"
    || !isAuthoredProjectMutationFence(fence)
    || !isRecord(response)) {
    throw invalidReceipt("terminal");
  }

  if (response.kind === "rejected") {
    const rejected = isRecord(response.result) ? response.result : null;
    const error = rejected && isRecord(rejected.error) ? rejected.error : null;
    if (!rejected
      || rejected.operation_id !== authoredSetEffectEnabledOperationId
      || rejected.request_id !== requestId
      || !isNonZeroSafeInteger(rejected.request_id)
      || !isAuthoredProjectMutationFence(rejected.start_fence)
      || !authoredProjectMutationFencesEqual(rejected.start_fence, fence)
      || !error
      || !isAuthoredSetEffectEnabledErrorCode(error.code)) {
      throw invalidReceipt("rejection");
    }
    throw new AuthoredSetEffectEnabledCommandError(error.code, false);
  }

  if (response.kind !== "terminal_receipt") {
    throw invalidReceipt("terminal");
  }
  const receipt = isRecord(response.result) ? response.result : null;
  if (!receipt
    || receipt.operation_id !== authoredSetEffectEnabledOperationId
    || receipt.request_id !== requestId
    || !isNonZeroSafeInteger(receipt.request_id)
    || !isAuthoredProjectMutationFence(receipt.start_fence)
    || !authoredProjectMutationFencesEqual(receipt.start_fence, fence)
    || typeof receipt.shape_sha256 !== "string"
    || !/^[0-9a-f]{64}$/.test(receipt.shape_sha256)
    || !isRecord(receipt.outcome)) {
    throw invalidReceipt("terminal");
  }

  const outcome = receipt.outcome;
  if (outcome.kind === "failed") {
    const failure = isRecord(outcome.result) ? outcome.result : null;
    if (!failure || !isAuthoredSetEffectEnabledErrorCode(failure.code)) {
      throw invalidReceipt("failure");
    }
    throw new AuthoredSetEffectEnabledCommandError(failure.code, true);
  }
  if (outcome.kind !== "applied" && outcome.kind !== "no_op") {
    throw invalidReceipt("outcome");
  }
  const applied = isRecord(outcome.result) ? outcome.result : null;
  if (!applied
    || applied.effect_id !== effectId
    || !isNonZeroSafeInteger(applied.effect_id)
    || applied.enabled !== enabled
    || typeof applied.enabled !== "boolean"
    || !isAuthoredProjectMutationFence(applied.post_fence)) {
    throw invalidReceipt("outcome");
  }
  if (outcome.kind === "no_op"
    && !authoredProjectMutationFencesEqual(applied.post_fence, fence)) {
    throw new Error("Effect enable NoOp receipt changed project authority.");
  }
};

export type AuthoredEffectEnableIntent = Readonly<{
  effectId: number;
  enabled: boolean;
}>;

/**
 * Lifecycle observers receive the controller-owned generation rather than
 * inventing a second renderer counter.  That lets optimistic UI state roll
 * back only the intent which is still current for this effect lane.
 */
export type AuthoredEffectEnableIntentLifecycle = AuthoredEffectEnableIntent & Readonly<{
  generation: number;
}>;

type VersionedIntent = AuthoredEffectEnableIntentLifecycle;

type EffectEnableLane = {
  latest: VersionedIntent | null;
  running: boolean;
  drain: Promise<void> | null;
  /** Last server-settled enabled value, never an optimistic draft value. */
  settledEnabled: boolean;
};

export type AuthoredEffectEnableIntentControllerOptions = {
  /**
   * Reads the authoritative enabled value only when an effect lane is idle.
   * A newly queued intent must not treat an earlier optimistic Store Recall
   * mirror as its rollback baseline.
   */
  seedSettledBaseline: (effectId: number) => boolean;
  /** Captures a current authoritative fence on every bounded attempt. */
  dispatch: (intent: AuthoredEffectEnableIntent, attempt: number) => Promise<void>;
  isStaleFence: (error: unknown) => boolean;
  /** Runs synchronously when a newest intent becomes the lane's current intent. */
  onQueued?: (intent: AuthoredEffectEnableIntentLifecycle) => void;
  onApplied?: (intent: AuthoredEffectEnableIntentLifecycle) => void;
  /**
   * `settledEnabled` is the last successful server value for this lane. A
   * terminal failure must roll an optimistic mirror back to it, not to the
   * value visible when the failed intent was enqueued.
   */
  onFailed?: (
    intent: AuthoredEffectEnableIntentLifecycle,
    error: unknown,
    settledEnabled: boolean,
  ) => void;
  /** Includes the first request; default 2 gives one current-intent refresh/retry. */
  maxAttempts?: number;
};

/**
 * One serialized lane per global effect. A later intent replaces the queued
 * desired value. Completion from an older intent is never surfaced and a
 * stale-fence retry is allowed only while that exact desired value remains
 * current, for at most `maxAttempts` total dispatches.
 */
export function createAuthoredEffectEnableIntentController(
  options: AuthoredEffectEnableIntentControllerOptions,
) {
  const lanes = new Map<number, EffectEnableLane>();
  const maxAttempts = Math.max(1, Math.floor(options.maxAttempts ?? 2));
  let nextGeneration = 0;

  const allocateGeneration = () => {
    if (!Number.isSafeInteger(nextGeneration) || nextGeneration >= MAX_SAFE_JAVASCRIPT_INTEGER) {
      throw new Error("Effect enable intent generations are exhausted; restart Syndocal before trying again.");
    }
    nextGeneration += 1;
    return nextGeneration;
  };

  const runLane = async (effectId: number, lane: EffectEnableLane): Promise<void> => {
    for (;;) {
      const intent = lane.latest;
      if (!intent) {
        lane.running = false;
        lane.drain = null;
        if (lanes.get(effectId) === lane) lanes.delete(effectId);
        return;
      }
      let attempt = 0;
      for (;;) {
        try {
          await options.dispatch(
            { effectId: intent.effectId, enabled: intent.enabled },
            attempt,
          );
          // A reply can belong to a superseded intent, but it is still a real
          // server settlement. Update the lane baseline before deciding
          // whether it should surface as the current UI completion.
          lane.settledEnabled = intent.enabled;
          const current = lane.latest;
          if (current?.generation === intent.generation) {
            options.onApplied?.(intent);
            if (lane.latest?.generation === intent.generation) lane.latest = null;
          } else if (current?.enabled === intent.enabled) {
            // A rapid A/B/A settled A on the wire. The newest desired intent
            // is already satisfied, so coalesce it rather than publishing A
            // a second time.
            options.onApplied?.(current);
            if (lane.latest?.generation === current.generation) lane.latest = null;
          }
          break;
        } catch (error) {
          if (lane.latest?.generation !== intent.generation) {
            // A newer desired value arrived while the old call was pending.
            // Never retry or report that superseded result.
            break;
          }
          attempt += 1;
          if (options.isStaleFence(error) && attempt < maxAttempts) {
            // The next dispatch obtains a fresh server-issued fence. No old
            // stale call can overwrite a newer desired value because the
            // generation was checked immediately above.
            continue;
          }
          // Notify while this exact generation remains current. An optimistic
          // Store Recall rollback can therefore reject a newer A/B intent
          // even if an observer itself synchronously queues another change.
          options.onFailed?.(intent, error, lane.settledEnabled);
          if (lane.latest?.generation === intent.generation) lane.latest = null;
          break;
        }
      }
    }
  };

  return {
    enqueue: (effectId: number, enabled: boolean): Promise<void> => {
      if (!isNonZeroSafeInteger(effectId) || typeof enabled !== "boolean") {
        return Promise.reject(new Error("Effect enable intent has an invalid effect ID or enabled value."));
      }
      let lane = lanes.get(effectId);
      if (!lane) {
        const settledEnabled = options.seedSettledBaseline(effectId);
        if (typeof settledEnabled !== "boolean") {
          return Promise.reject(new Error("Effect enable baseline must be an authoritative boolean."));
        }
        lane = { latest: null, running: false, drain: null, settledEnabled };
        lanes.set(effectId, lane);
      }
      lane.latest = { effectId, enabled, generation: allocateGeneration() };
      // This intentionally precedes starting the async lane. Consumers that
      // mirror live state into a Store Recall draft must expose the newest
      // intent while the strict IPC request is still pending.
      options.onQueued?.(lane.latest);
      if (!lane.running) {
        lane.running = true;
        lane.drain = runLane(effectId, lane);
      }
      return lane.drain!;
    },
    isPending: (effectId: number) => lanes.get(effectId)?.running ?? false,
  };
}
