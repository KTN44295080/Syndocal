import { waitForPublishedCommandRecovery, type PatchRepairTransactionCommand } from "./patchTransactionD2";
import {
  ProjectTransactionTerminalMalformedMutationError,
  createProjectTransactionTerminalSettlement,
  projectTransactionTerminalArgs,
  projectTransactionTerminalMutationIsWellFormed,
  recoverProjectTransactionTerminalAction,
  settleProjectTransactionTerminalAcknowledgement,
  type ProjectTransactionIdentity,
  type ProjectTransactionTerminalAction,
  type ProjectTransactionTerminalArgs,
  type ProjectTransactionTerminalRecoveryResult,
} from "./projectTransactionRecovery";
import { projectTransactionRecoveryCanAdopt } from "./types";
import type {
  ProjectHistoryMutationResult,
  ProjectTransactionCommandResult,
  ProjectTransactionRecovery,
  ProjectTransactionTicket,
} from "./types";

type ProjectTransactionRecoveryCommand =
  | "begin_project_transaction"
  | "query_project_transaction"
  | "adopt_project_transaction"
  | "commit_project_transaction"
  | "cancel_project_transaction"
  | "acknowledge_project_transaction";

export type ProjectTransactionRecoveryPorts = Readonly<{
  invoke: <T>(command: ProjectTransactionRecoveryCommand, args?: Record<string, unknown>) => Promise<T>;
  dispatchHistoryMutation: (mutation: ProjectHistoryMutationResult) => void;
  reportRecovery: (message: string) => void;
  wait: (milliseconds: number) => Promise<void>;
}>;

/** Owns one renderer's retained terminal receipt; never retains or replays a raw mutation.
 * Registration, operator locks, authority capture, and mapping barriers belong to the caller.
 */
export const createProjectTransactionRecoveryController = (ports: ProjectTransactionRecoveryPorts) => {
  const queryProjectTransactionRecovery = async (
    identity: ProjectTransactionIdentity,
  ): Promise<ProjectTransactionRecovery | null> => ports.invoke<ProjectTransactionRecovery | null>(
    "query_project_transaction",
    identity,
  );

  const acknowledgeProjectTransaction = async (
    identity: ProjectTransactionIdentity,
  ) => ports.invoke<void>("acknowledge_project_transaction", identity);

  const reportProjectTransactionForegroundRecovery = ports.reportRecovery;

  const reportProjectTransactionForegroundRecoveryWithRetry = (message: string) =>
    reportProjectTransactionForegroundRecovery(
      `${message} Retry a project mutation to continue this exact terminal recovery.`,
    );

  const waitForProjectTransactionTerminalRecovery = ports.wait;

  type ProjectTransactionForegroundTerminalRecovery = Readonly<{
    phase: "terminal" | "acknowledgement";
    action: ProjectTransactionTerminalAction;
    identity: ProjectTransactionIdentity;
    terminalArgs: ProjectTransactionTerminalArgs;
    retry: () => Promise<unknown>;
  }>;

  // This is intentionally structured rather than a status string: a later
  // explicit project mutation resumes the exact action/ticket after its bounded
  // batch expires. The raw project mutation is never retained here, so no retry
  // path can replay it.
  let foregroundProjectTransactionTerminalRecovery: ProjectTransactionForegroundTerminalRecovery | null = null;

  const clearForegroundProjectTransactionTerminalRecovery = (
    recovery: ProjectTransactionForegroundTerminalRecovery,
  ) => {
    if (foregroundProjectTransactionTerminalRecovery === recovery) {
      foregroundProjectTransactionTerminalRecovery = null;
    }
  };

  const installForegroundProjectTransactionTerminalRecovery = (
    phase: ProjectTransactionForegroundTerminalRecovery["phase"],
    action: ProjectTransactionTerminalAction,
    identity: ProjectTransactionIdentity,
    terminalArgs: ProjectTransactionTerminalArgs,
    buildRetry: (
      clear: () => void,
    ) => () => Promise<unknown>,
  ): ProjectTransactionForegroundTerminalRecovery => {
    const existing = foregroundProjectTransactionTerminalRecovery;
    if (existing) {
      throw new Error(
        `Project transaction ${existing.action} recovery remains pending; retry that exact ${existing.phase} before starting another mutation.`,
      );
    }
    let recovery: ProjectTransactionForegroundTerminalRecovery;
    const clear = () => clearForegroundProjectTransactionTerminalRecovery(recovery);
    recovery = Object.freeze({
      phase,
      action,
      identity,
      terminalArgs,
      retry: buildRetry(clear),
    });
    foregroundProjectTransactionTerminalRecovery = recovery;
    return recovery;
  };

  const resumeForegroundProjectTransactionTerminalRecoveryBeforeMutation = async (): Promise<void> => {
    const recovery = foregroundProjectTransactionTerminalRecovery;
    if (!recovery) return;
    reportProjectTransactionForegroundRecovery(
      `Retrying pending project transaction ${recovery.action} ${recovery.phase} recovery before the next mutation.`,
    );
    await recovery.retry();
  };

  const createAppProjectTransactionTerminalSettlement = (identity: ProjectTransactionIdentity) =>
    createProjectTransactionTerminalSettlement(
      ports.dispatchHistoryMutation,
      () => acknowledgeProjectTransaction(identity),
    );

  const settleAppProjectTransactionTerminal = (
    settlement: ReturnType<typeof createAppProjectTransactionTerminalSettlement>,
    mutation: ProjectHistoryMutationResult,
  ) => settleProjectTransactionTerminalAcknowledgement(
    settlement,
    mutation,
    waitForProjectTransactionTerminalRecovery,
    reportProjectTransactionForegroundRecoveryWithRetry,
  );

  const settleProjectTransactionTerminalInForeground = async (
    action: ProjectTransactionTerminalAction,
    identity: ProjectTransactionIdentity,
    terminalArgs: ProjectTransactionTerminalArgs,
    settlement: ReturnType<typeof createAppProjectTransactionTerminalSettlement>,
    mutation: ProjectHistoryMutationResult,
  ): Promise<void> => {
    const recovery = installForegroundProjectTransactionTerminalRecovery(
      "acknowledgement",
      action,
      identity,
      terminalArgs,
      (clear) => async () => {
        await settleAppProjectTransactionTerminal(settlement, mutation);
        clear();
      },
    );
    await recovery.retry();
  };

  const recoverProjectTransactionTerminalInForeground = async (
    action: ProjectTransactionTerminalAction,
    terminalArgs: ProjectTransactionTerminalArgs,
    identity: ProjectTransactionIdentity,
    settlement: ReturnType<typeof createAppProjectTransactionTerminalSettlement>,
  ): Promise<ProjectTransactionTerminalRecoveryResult> => {
    const recovery = installForegroundProjectTransactionTerminalRecovery(
      "terminal",
      action,
      identity,
      terminalArgs,
      (clear) => async () => {
        const result = await recoverProjectTransactionTerminal(action, terminalArgs, identity);
        const mutation = result.kind === "operation" ? result.mutation : result.recovery.mutation;
        await settleAppProjectTransactionTerminal(settlement, mutation);
        clear();
        return result;
      },
    );
    return await recovery.retry() as ProjectTransactionTerminalRecoveryResult;
  };

  const recoverProjectTransactionTerminal = async (
    action: ProjectTransactionTerminalAction,
    terminalArgs: ProjectTransactionTerminalArgs,
    identity: ProjectTransactionIdentity,
  ) => recoverProjectTransactionTerminalAction({
    identity,
    terminalArgs,
    action,
    query: queryProjectTransactionRecovery,
    invokeTerminal: (args) => ports.invoke<ProjectHistoryMutationResult>(
      action === "commit" ? "commit_project_transaction" : "cancel_project_transaction",
      args,
    ),
    wait: waitForProjectTransactionTerminalRecovery,
    reportUnresolved: reportProjectTransactionForegroundRecoveryWithRetry,
  });

  type PublishedProjectTransactionCommandRecovery =
    | { kind: "published"; result: ProjectTransactionCommandResult }
    | { kind: "not_published" }
    | { kind: "indeterminate"; error: string }
    | { kind: "unconfirmed" };

  const recoverPublishedProjectTransactionCommandResult = async (
    identity: ProjectTransactionIdentity,
    command: PatchRepairTransactionCommand,
    commandArgs: Record<string, unknown>,
  ): Promise<PublishedProjectTransactionCommandRecovery> => {
    const decision = await waitForPublishedCommandRecovery(
      () => queryProjectTransactionRecovery(identity),
      command,
      commandArgs,
      ports.wait,
    );
    return decision;
  };

  const recoverProjectTransactionBegin = async (
    identity: ProjectTransactionIdentity,
  ): Promise<ProjectTransactionRecovery | null> => {
    let lastError: unknown = null;
    for (let attempt = 0; attempt < 3; attempt += 1) {
      try {
        const queried = await queryProjectTransactionRecovery(identity);
        if (!queried) return null;
        if (!projectTransactionRecoveryCanAdopt(queried.status)) return queried;
        return await ports.invoke<ProjectTransactionRecovery>("adopt_project_transaction", identity);
      } catch (error) {
        lastError = error;
      }
    }
    if (lastError) throw lastError;
    return null;
  };

  const beginProjectTransactionWithRecovery = async (
    beginArgs: Record<string, unknown>,
    identity: ProjectTransactionIdentity,
  ): Promise<ProjectTransactionTicket> => {
    try {
      return await ports.invoke<ProjectTransactionTicket>("begin_project_transaction", beginArgs);
    } catch (beginError) {
      const recovered = await recoverProjectTransactionBegin(identity).catch(() => null);
      if (!recovered) throw beginError;
      if (recovered.status === "pending") return recovered.ticket;
      if (recovered.status === "committed") {
        const settlement = createAppProjectTransactionTerminalSettlement(identity);
        await settleAppProjectTransactionTerminal(settlement, recovered.mutation);
        throw new Error(
          "Project transaction committed while the Begin reply was lost; command result was not delivered. Refresh before retrying.",
        );
      }
      if (recovered.status === "cancelled") {
        const settlement = createAppProjectTransactionTerminalSettlement(identity);
        await settleAppProjectTransactionTerminal(settlement, recovered.mutation);
        throw new Error("Project transaction was cancelled while the Begin reply was lost.");
      }
      throw new Error("Project transaction receipt was already acknowledged; use a new operation ID.");
    }
  };

  const cancelProjectTransactionWithRecovery = async (
    transaction: ProjectTransactionTicket,
    identity: ProjectTransactionIdentity,
  ): Promise<ProjectHistoryMutationResult | null> => {
    const cancelArgs = projectTransactionTerminalArgs(transaction, identity);
    const settleTerminal = createAppProjectTransactionTerminalSettlement(identity);
    let cancellation: ProjectHistoryMutationResult | null = null;
    try {
      cancellation = await ports.invoke<ProjectHistoryMutationResult>(
        "cancel_project_transaction",
        cancelArgs,
      );
      if (!projectTransactionTerminalMutationIsWellFormed(cancellation)) {
        throw new ProjectTransactionTerminalMalformedMutationError(
          "Project transaction Cancel reply was malformed; querying the exact terminal receipt.",
        );
      }
    } catch {
      const recovered = await recoverProjectTransactionTerminalInForeground(
        "cancel",
        cancelArgs,
        identity,
        settleTerminal,
      );
      if (recovered.kind === "operation") {
        return recovered.mutation;
      }
      if (recovered.recovery.status === "cancelled") {
        return recovered.recovery.mutation;
      }
      if (recovered.recovery.status === "committed") {
        throw new Error(
          "Project transaction committed while Cancel was in flight; inspect the recovered history before retrying.",
        );
      }
    }
    if (cancellation === null) {
      throw new Error("Project transaction Cancel recovery returned no terminal mutation.");
    }
    await settleProjectTransactionTerminalInForeground(
      "cancel",
      identity,
      cancelArgs,
      settleTerminal,
      cancellation,
    );
    return cancellation;
  };

  const commitProjectTransactionWithRecovery = async (
    transaction: ProjectTransactionTicket,
    identity: ProjectTransactionIdentity,
    settleTerminal: ReturnType<typeof createAppProjectTransactionTerminalSettlement>,
  ): Promise<ProjectHistoryMutationResult> => {
    const commitArgs = projectTransactionTerminalArgs(transaction, identity);
    let committed: ProjectHistoryMutationResult | null = null;
    try {
      committed = await ports.invoke<ProjectHistoryMutationResult>(
        "commit_project_transaction",
        commitArgs,
      );
      if (!projectTransactionTerminalMutationIsWellFormed(committed)) {
        throw new ProjectTransactionTerminalMalformedMutationError(
          "Project transaction Commit reply was malformed; querying the exact terminal receipt.",
        );
      }
    } catch {
      const recovered = await recoverProjectTransactionTerminalInForeground(
        "commit",
        commitArgs,
        identity,
        settleTerminal,
      );
      if (recovered.kind === "operation") return recovered.mutation;
      if (recovered.recovery.status === "committed") return recovered.recovery.mutation;
      if (recovered.recovery.status === "cancelled") {
        throw new Error("Project transaction was cancelled while Commit was in flight.");
      }
    }
    if (committed === null) {
      throw new Error("Project transaction Commit recovery returned no terminal mutation.");
    }
    await settleProjectTransactionTerminalInForeground(
      "commit",
      identity,
      commitArgs,
      settleTerminal,
      committed,
    );
    return committed;
  };

  return {
    resumeForegroundProjectTransactionTerminalRecoveryBeforeMutation,
    createAppProjectTransactionTerminalSettlement,
    beginProjectTransactionWithRecovery,
    cancelProjectTransactionWithRecovery,
    commitProjectTransactionWithRecovery,
    recoverPublishedProjectTransactionCommandResult,
  };
};
