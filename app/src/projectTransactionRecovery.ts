import type {
  ProjectHistoryMutationResult,
  ProjectTransactionRecovery,
  ProjectTransactionTicket,
} from "./types";

/**
 * The renderer must retain this exact tuple for an operation's complete
 * lifetime. Recovery may query/settle the tuple, but it may never create a
 * second raw project-mutation dispatch from it.
 */
export type ProjectTransactionIdentity = Readonly<{
  clientOperationId: string;
  shapeFingerprint: string;
  commandName: string;
  schemaVersion: number;
  ownerId: string;
}>;

export type ProjectTransactionTerminalAction = "commit" | "cancel";

export type ProjectTransactionTerminalArgs = Readonly<{
  transactionId: number;
  expectedEpoch: number;
  clientOperationId: string;
  shapeFingerprint: string;
  commandName: string;
  schemaVersion: number;
  ownerId: string;
}>;

export class ProjectTransactionTerminalAcknowledgementError extends Error {
  constructor(cause: unknown) {
    super(`Project transaction terminal acknowledgement failed: ${String(cause)}`);
    this.name = "ProjectTransactionTerminalAcknowledgementError";
  }
}

export class ProjectTransactionTerminalAcknowledgementUnresolvedError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "ProjectTransactionTerminalAcknowledgementUnresolvedError";
  }
}

export class ProjectTransactionTerminalMalformedMutationError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "ProjectTransactionTerminalMalformedMutationError";
  }
}

const isRecord = (value: unknown): value is Record<string, unknown> =>
  typeof value === "object" && value !== null && !Array.isArray(value);

const isNonNegativeSafeInteger = (value: unknown): value is number =>
  typeof value === "number" && Number.isSafeInteger(value) && value >= 0;

const isHistoryStatus = (value: unknown): boolean => {
  if (!isRecord(value)) return false;
  return typeof value.can_undo === "boolean"
    && typeof value.can_redo === "boolean"
    && isNonNegativeSafeInteger(value.undo_depth)
    && isNonNegativeSafeInteger(value.redo_depth)
    && isNonNegativeSafeInteger(value.project_epoch)
    && isNonNegativeSafeInteger(value.project_revision)
    && typeof value.checkpoint_hash === "string"
    && isNonNegativeSafeInteger(value.history_generation);
};

const isPresentValue = (value: unknown): boolean => value !== undefined && value !== null;

/**
 * The backend derives each capability flag from its stack depth
 * (`can_undo == undo_depth > 0`) and each entry descriptor from that same
 * stack top, so a status whose flags contradict its depths — or whose
 * optional descriptors outlive their stack — is stitched or corrupted
 * rather than authoritative.
 */
const historyStatusCapabilitiesMatchDepths = (status: Record<string, unknown>): boolean => {
  const canUndo = status.can_undo === true;
  const canRedo = status.can_redo === true;
  const undoDepth = status.undo_depth;
  const redoDepth = status.redo_depth;
  if (typeof undoDepth !== "number" || typeof redoDepth !== "number") return false;
  if (canUndo !== (undoDepth > 0)) return false;
  if (canRedo !== (redoDepth > 0)) return false;
  // Optional fields may be truthfully absent under this schema; absence is
  // never repaired or synthesized, but any present non-null descriptor must
  // still agree with its capability flag instead of silently elevating the
  // malformed state.
  const descriptorImpliesCapability = (descriptor: unknown, capability: boolean): boolean =>
    !isPresentValue(descriptor) || capability;
  return descriptorImpliesCapability(status.undo_label, canUndo)
    && descriptorImpliesCapability(status.undo_entry_id, canUndo)
    && descriptorImpliesCapability(status.undo_checkpoint_hash, canUndo)
    && descriptorImpliesCapability(status.redo_label, canRedo)
    && descriptorImpliesCapability(status.redo_entry_id, canRedo)
    && descriptorImpliesCapability(status.redo_checkpoint_hash, canRedo);
};

/**
 * One coordinator capture publishes the project identity three ways: the
 * terminal `history_status`, the authority token fields, and the authority's
 * embedded history image. A receipt whose epoch/revision/checkpoint-hash/
 * history-generation copies disagree cannot identify one authoritative state,
 * so applying or ACKing it would corrupt or discard the retained receipt.
 */
const terminalHistoryIdentityFieldsAgree = (
  status: Record<string, unknown>,
  authority: Record<string, unknown>,
): boolean => {
  const authorityHistory = authority.history;
  if (!isRecord(authorityHistory)) return false;
  for (const field of [
    "project_epoch",
    "project_revision",
    "checkpoint_hash",
    "history_generation",
  ] as const) {
    if (status[field] !== authority[field]) return false;
    if (authority[field] !== authorityHistory[field]) return false;
  }
  return true;
};

export const projectTransactionTerminalMutationIsInternallyConsistent = (
  value: unknown,
): boolean => {
  if (!isRecord(value) || !isRecord(value.history_status) || !isRecord(value.authority)) return false;
  const { history_status: status, authority } = value;
  if (!isRecord(authority.history)) return false;
  return historyStatusCapabilitiesMatchDepths(status)
    && historyStatusCapabilitiesMatchDepths(authority.history)
    && terminalHistoryIdentityFieldsAgree(status, authority);
};

/**
 * A terminal reply may cross the IPC boundary as unknown JSON. Do not ACK a
 * partial or self-contradictory result: applying it can corrupt the rendered
 * history, while ACKing it would discard the only receipt needed to repair
 * that rendering.
 */
export const projectTransactionTerminalMutationIsWellFormed = (
  value: unknown,
): value is ProjectHistoryMutationResult =>
  projectTransactionTerminalMutationHasWellFormedShape(value)
  && projectTransactionTerminalMutationIsInternallyConsistent(value);

const projectTransactionTerminalMutationHasWellFormedShape = (
  value: unknown,
): value is ProjectHistoryMutationResult => {
  if (!isRecord(value) || !isHistoryStatus(value.history_status) || !isRecord(value.authority)) return false;
  const authority = value.authority;
  return isNonNegativeSafeInteger(authority.project_epoch)
    && isNonNegativeSafeInteger(authority.project_revision)
    && typeof authority.checkpoint_hash === "string"
    && isNonNegativeSafeInteger(authority.publication_generation)
    && typeof authority.publication_kind === "string"
    && isNonNegativeSafeInteger(authority.mapping_replacement_generation)
    && isNonNegativeSafeInteger(authority.authority_disposition_generation)
    && typeof authority.authority_disposition === "string"
    && isNonNegativeSafeInteger(authority.recovery_authority_serial)
    && isRecord(authority.recovery_authority_last_transition)
    && isNonNegativeSafeInteger(authority.path_generation)
    && isNonNegativeSafeInteger(authority.history_generation)
    && (typeof authority.current_project_path === "string" || authority.current_project_path === null)
    && isRecord(authority.snapshot)
    && Array.isArray(authority.profiles)
    && Array.isArray(authority.fixture_groups)
    && (isRecord(authority.operator_policy) || authority.operator_policy === null)
    && Array.isArray(authority.midi_mappings)
    && Array.isArray(authority.osc_mappings)
    && Array.isArray(authority.dmx_mappings)
    && Array.isArray(authority.dj_track_triggers)
    && isHistoryStatus(authority.history)
    && isRecord(authority.input_runtime);
};

export const requireProjectTransactionTerminalMutation = (
  value: unknown,
): ProjectHistoryMutationResult => {
  if (!projectTransactionTerminalMutationIsWellFormed(value)) {
    throw new ProjectTransactionTerminalMalformedMutationError(
      "Project transaction terminal reply was malformed; the receipt was retained without applying history or acknowledging it.",
    );
  }
  return value;
};

export const projectTransactionTerminalArgs = (
  transaction: ProjectTransactionTicket,
  identity: ProjectTransactionIdentity,
): ProjectTransactionTerminalArgs => Object.freeze({
  transactionId: transaction.transaction_id,
  expectedEpoch: transaction.project_epoch,
  clientOperationId: transaction.client_operation_id,
  shapeFingerprint: transaction.shape_fingerprint,
  commandName: identity.commandName,
  schemaVersion: transaction.schema_version,
  ownerId: identity.ownerId,
});

/**
 * Keep history delivery and the retained-receipt acknowledgement idempotent
 * across a caller's direct-reply/recovery branches. A failed ACK remains
 * retryable without applying the same authoritative mutation twice.
 */
export const createProjectTransactionTerminalSettlement = (
  apply: (mutation: ProjectHistoryMutationResult) => void,
  acknowledge: () => Promise<void>,
) => {
  let applied = false;
  let acknowledged = false;
  return async (mutation: ProjectHistoryMutationResult): Promise<void> => {
    requireProjectTransactionTerminalMutation(mutation);
    if (!applied) {
      apply(mutation);
      applied = true;
    }
    if (!acknowledged) {
      try {
        await acknowledge();
      } catch (error) {
        throw new ProjectTransactionTerminalAcknowledgementError(error);
      }
      acknowledged = true;
    }
  };
};

export type ProjectTransactionTerminalRecoveryResult =
  | { kind: "operation"; mutation: ProjectHistoryMutationResult }
  | {
    kind: "terminal";
    recovery: Extract<ProjectTransactionRecovery, { status: "committed" | "cancelled" }>;
  };

export class ProjectTransactionTerminalRecoveryHoldError extends Error {
  readonly action: ProjectTransactionTerminalAction;

  constructor(action: ProjectTransactionTerminalAction, message: string) {
    super(message);
    this.name = "ProjectTransactionTerminalRecoveryHoldError";
    this.action = action;
  }
}

export class ProjectTransactionTerminalRecoveryUnresolvedError extends Error {
  readonly action: ProjectTransactionTerminalAction;

  constructor(action: ProjectTransactionTerminalAction, message: string) {
    super(message);
    this.name = "ProjectTransactionTerminalRecoveryUnresolvedError";
    this.action = action;
  }
}

type ProjectTransactionTerminalRecoveryOptions = Readonly<{
  identity: ProjectTransactionIdentity;
  terminalArgs: ProjectTransactionTerminalArgs;
  action: ProjectTransactionTerminalAction;
  query: (identity: ProjectTransactionIdentity) => Promise<ProjectTransactionRecovery | null>;
  invokeTerminal: (args: ProjectTransactionTerminalArgs) => Promise<unknown>;
  wait: (milliseconds: number) => Promise<void>;
  reportUnresolved: (message: string) => void;
}>;

const terminalRecoveryMaxAttempts = 6;
const terminalRecoveryInitialDelayMs = 25;
const terminalRecoveryMaximumDelayMs = 400;

export const projectTransactionTerminalRecoveryDelayMs = (attempt: number): number =>
  Math.min(
    terminalRecoveryInitialDelayMs * 2 ** Math.max(0, attempt),
    terminalRecoveryMaximumDelayMs,
  );

/**
 * Once Commit/Cancel reached a terminal, ACK is the only permitted retry.
 * Reusing the same settlement retains its applied latch, so an ACK reply-loss
 * retry cannot apply the history mutation twice or fall through to Cancel.
 */
export const settleProjectTransactionTerminalAcknowledgement = async (
  settlement: (mutation: ProjectHistoryMutationResult) => Promise<void>,
  mutation: ProjectHistoryMutationResult,
  wait: (milliseconds: number) => Promise<void>,
  reportUnresolved: (message: string) => void,
): Promise<void> => {
  let lastError: ProjectTransactionTerminalAcknowledgementError | null = null;
  for (let attempt = 0; attempt < terminalRecoveryMaxAttempts; attempt += 1) {
    try {
      await settlement(mutation);
      return;
    } catch (error) {
      if (!(error instanceof ProjectTransactionTerminalAcknowledgementError)) throw error;
      lastError = error;
      if (attempt + 1 < terminalRecoveryMaxAttempts) {
        await wait(projectTransactionTerminalRecoveryDelayMs(attempt));
      }
    }
  }
  const message = `Project transaction terminal acknowledgement remains pending after ${terminalRecoveryMaxAttempts} bounded attempts: ${String(lastError ?? "no acknowledgement reply")}`;
  reportUnresolved(message);
  throw new ProjectTransactionTerminalAcknowledgementUnresolvedError(message);
};

const pendingRecoveryHoldReason = (
  action: ProjectTransactionTerminalAction,
  recovery: Extract<ProjectTransactionRecovery, { status: "pending" }>,
): string | null => {
  if (typeof recovery.command_indeterminate_error === "string"
    && recovery.command_indeterminate_error.trim() !== "") {
    return `Project transaction ${action} recovery is held: command publication is indeterminate (${recovery.command_indeterminate_error}).`;
  }
  if (recovery.command_indeterminate_error !== null) {
    return `Project transaction ${action} recovery is held: command indeterminacy state is malformed.`;
  }
  if (typeof recovery.command_in_flight !== "boolean") {
    return `Project transaction ${action} recovery is held: command admission state is malformed.`;
  }
  // A definitive published receipt cannot be discarded. Commit's own exact
  // retry will settle it; a Cancel request remains visibly held instead of
  // changing user intent or manufacturing a rollback.
  if (action === "cancel" && recovery.command_result != null) {
    return "Project transaction Cancel recovery is held: a published command result requires the original Commit path.";
  }
  return null;
};

/**
 * Recover only an already-issued terminal action. Every probe receives the
 * original identity object and every retry receives the same frozen ticket
 * shape. No raw engine/project mutation command is available to this helper.
 */
export const recoverProjectTransactionTerminalAction = async (
  options: ProjectTransactionTerminalRecoveryOptions,
): Promise<ProjectTransactionTerminalRecoveryResult> => {
  let lastError: unknown = null;
  for (let attempt = 0; attempt < terminalRecoveryMaxAttempts; attempt += 1) {
    let recovery: ProjectTransactionRecovery | null;
    try {
      recovery = await options.query(options.identity);
    } catch (error) {
      lastError = error;
      if (attempt + 1 < terminalRecoveryMaxAttempts) {
        await options.wait(projectTransactionTerminalRecoveryDelayMs(attempt));
        continue;
      }
      break;
    }
    if (recovery?.status === "committed" || recovery?.status === "cancelled") {
      try {
        return {
          kind: "terminal",
          recovery: {
            ...recovery,
            mutation: requireProjectTransactionTerminalMutation(recovery.mutation),
          },
        };
      } catch (error) {
        lastError = error;
        if (attempt + 1 < terminalRecoveryMaxAttempts) {
          await options.wait(projectTransactionTerminalRecoveryDelayMs(attempt));
          continue;
        }
        break;
      }
    }
    if (recovery?.status === "acknowledged") {
      const message = `Project transaction ${options.action} recovery cannot apply an already acknowledged terminal receipt.`;
      options.reportUnresolved(message);
      throw new ProjectTransactionTerminalRecoveryUnresolvedError(options.action, message);
    }
    if (!recovery) {
      const message = `Project transaction ${options.action} recovery could not find its exact receipt; no further action was sent.`;
      options.reportUnresolved(message);
      throw new ProjectTransactionTerminalRecoveryUnresolvedError(options.action, message);
    }
    const holdReason = pendingRecoveryHoldReason(options.action, recovery);
    if (holdReason) {
      options.reportUnresolved(holdReason);
      throw new ProjectTransactionTerminalRecoveryHoldError(options.action, holdReason);
    }
    if (recovery.command_in_flight) {
      lastError = new Error("the exact project command is still in flight");
      if (attempt + 1 < terminalRecoveryMaxAttempts) {
        await options.wait(projectTransactionTerminalRecoveryDelayMs(attempt));
        continue;
      }
      break;
    }
    // The backend's close fence may have changed from busy to idle after the
    // terminal reply was lost. Retry only this idempotent terminal command
    // against exactly the same ticket; never replay the raw mutation.
    try {
      return {
        kind: "operation",
        mutation: requireProjectTransactionTerminalMutation(
          await options.invokeTerminal(options.terminalArgs),
        ),
      };
    } catch (error) {
      lastError = error;
      if (attempt + 1 < terminalRecoveryMaxAttempts) {
        await options.wait(projectTransactionTerminalRecoveryDelayMs(attempt));
      }
    }
  }
  const message = `Project transaction ${options.action} recovery remains pending after ${terminalRecoveryMaxAttempts} bounded attempts: ${String(lastError ?? "no terminal reply")}`;
  options.reportUnresolved(message);
  throw new ProjectTransactionTerminalRecoveryUnresolvedError(options.action, message);
};
