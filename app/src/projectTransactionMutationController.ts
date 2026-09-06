import type { FrontendTauriInvoke, FrontendTauriInvokeCommand } from "./tauriInvokeCommands";
import {
  isStageRendererTicketedCommand,
  publishedCommandLegacyReplyFromRecoveredResult,
  publishedCommandRecoveryDisposition,
  projectTransactionCommandResultIsWellFormed,
  type PatchRepairTransactionCommand,
  type PublishedCommandRecoveryDecision,
} from "./patchTransactionD2";
import {
  ProjectTransactionTerminalAcknowledgementUnresolvedError,
  ProjectTransactionTerminalMalformedMutationError,
  ProjectTransactionTerminalRecoveryHoldError,
  ProjectTransactionTerminalRecoveryUnresolvedError,
  type ProjectTransactionIdentity,
} from "./projectTransactionRecovery";
import type {
  ProjectHistoryMutationResult,
  ProjectTransactionTicket,
} from "./types";

export class ProjectTransactionPublicationUnconfirmedError extends Error {
  constructor(command: PatchRepairTransactionCommand) {
    super(`${command} may still be publishing; its terminal receipt was not confirmed. Do not retry yet.`);
    this.name = "ProjectTransactionPublicationUnconfirmedError";
  }
}

export class ProjectTransactionPublicationIndeterminateError extends Error {
  constructor(command: PatchRepairTransactionCommand, detail: string) {
    super(`${command} publication is indeterminate; restart is required before retrying. ${detail}`);
    this.name = "ProjectTransactionPublicationIndeterminateError";
  }
}

type ProjectTransactionTerminalSettlement = (
  mutation: ProjectHistoryMutationResult,
) => Promise<void>;

export type ProjectTransactionMutationControllerPorts = Readonly<{
  /** Raw Tauri transport; this module must never receive the App facade. */
  invoke: FrontendTauriInvoke;
  ownerId: string;
  cancelProjectTransactionWithRecovery: (
    transaction: ProjectTransactionTicket,
    identity: ProjectTransactionIdentity,
  ) => Promise<ProjectHistoryMutationResult | null>;
  commitProjectTransactionWithRecovery: (
    transaction: ProjectTransactionTicket,
    identity: ProjectTransactionIdentity,
    settlement: ProjectTransactionTerminalSettlement,
  ) => Promise<ProjectHistoryMutationResult>;
  createAppProjectTransactionTerminalSettlement: (
    identity: ProjectTransactionIdentity,
  ) => ProjectTransactionTerminalSettlement;
  recoverPublishedProjectTransactionCommandResult: (
    identity: ProjectTransactionIdentity,
    command: PatchRepairTransactionCommand,
    commandArgs: Record<string, unknown>,
  ) => Promise<PublishedCommandRecoveryDecision>;
}>;

export type ProjectTransactionMutationControllerInput = Readonly<{
  command: FrontendTauriInvokeCommand;
  commandArgs: Record<string, unknown>;
  transaction: ProjectTransactionTicket;
  identity: ProjectTransactionIdentity;
  onOpened: (() => void) | null;
  shouldAbort: (() => boolean) | null;
  awaitAbort: (() => Promise<void> | void) | null;
}>;

const shouldCancelOpenedTransaction = (error: unknown): boolean =>
  !(error instanceof ProjectTransactionPublicationUnconfirmedError)
  && !(error instanceof ProjectTransactionPublicationIndeterminateError)
  && !(error instanceof ProjectTransactionTerminalAcknowledgementUnresolvedError)
  && !(error instanceof ProjectTransactionTerminalMalformedMutationError)
  && !(error instanceof ProjectTransactionTerminalRecoveryHoldError)
  && !(error instanceof ProjectTransactionTerminalRecoveryUnresolvedError);

const publishedCommandFor = (command: FrontendTauriInvokeCommand): PatchRepairTransactionCommand | null =>
  command === "patch_fixtures"
    ? command
    : command === "repair_fixture_profile"
      ? command
      : isStageRendererTicketedCommand(command)
        ? command
        : null;

/**
 * Completes one already-open renderer project transaction. Begin/preflight,
 * command classification, operator admission, and authority capture stay in
 * App; this controller owns only the ticket's dispatch and terminal cleanup.
 */
export const createProjectTransactionMutationController = (
  ports: ProjectTransactionMutationControllerPorts,
) => {
  const executeProjectTransactionMutation = async <T,>(
    input: ProjectTransactionMutationControllerInput,
  ): Promise<T> => {
    const {
      command,
      commandArgs,
      transaction,
      identity,
      onOpened,
      shouldAbort,
      awaitAbort,
    } = input;
    let openedTransactionCancellation: Promise<ProjectHistoryMutationResult | null> | null = null;
    const cancelOpenedProjectTransaction = () => {
      if (!openedTransactionCancellation) {
        openedTransactionCancellation = ports.cancelProjectTransactionWithRecovery(transaction, identity);
      }
      return openedTransactionCancellation;
    };

    try {
      // A signal can arrive while Begin itself is awaiting. Release this
      // ticket before sending the staged mutation, then await the helper's
      // exact media-operation cancel request when available.
      if (shouldAbort?.()) {
        const mediaAbort = Promise.resolve(awaitAbort?.()).catch(() => undefined);
        await cancelOpenedProjectTransaction();
        await mediaAbort;
        throw new DOMException("Project mutation was cancelled after Begin.", "AbortError");
      }
      onOpened?.();

      // Backend mutation commands may opt into server-authoritative ownership.
      // Newly hardened routes require this exact ticket envelope; legacy
      // routes continue to receive their existing top-level shape.
      const ticketedRequest = {
        ...commandArgs,
        projectTransactionId: transaction.transaction_id,
        expectedEpoch: transaction.project_epoch,
        ownerId: ports.ownerId,
      };
      const strictTicketedRequestMutation = command === "set_fixture_transform"
        || command === "set_fixture_transforms"
        || command === "move_cue_between_scene_banks_batch";
      const ticketedArgs = strictTicketedRequestMutation
        ? { request: ticketedRequest }
        : ticketedRequest;
      const publishedCommand = publishedCommandFor(command);
      let result: T;
      try {
        const replied = await ports.invoke<T>(command, ticketedArgs);
        // Stage routes keep their legacy reply shapes on the happy path; only
        // PATCH/Repair expose and validate their receipt there.
        if (publishedCommand && !isStageRendererTicketedCommand(publishedCommand)
          && !projectTransactionCommandResultIsWellFormed(replied, publishedCommand, commandArgs)) {
          throw new Error(`${publishedCommand} returned a malformed published-command receipt.`);
        }
        result = replied;
      } catch (commandError) {
        if (!publishedCommand) throw commandError;
        const recovered = await ports.recoverPublishedProjectTransactionCommandResult(
          identity,
          publishedCommand,
          commandArgs,
        );
        const disposition = publishedCommandRecoveryDisposition(recovered);
        if (disposition === "commit" && recovered.kind === "published") {
          // Only a recovered Stage receipt is converted back to its legacy
          // shape; PATCH/Repair retain their durable receipt.
          result = publishedCommandLegacyReplyFromRecoveredResult(
            publishedCommand,
            recovered.result,
            commandArgs,
          ) as T;
        } else if (disposition === "hold") {
          // A command can finish publishing after its Tauri reply is lost. No
          // cancel/replay is safe until the backend receipt says otherwise.
          throw recovered.kind === "indeterminate"
            ? new ProjectTransactionPublicationIndeterminateError(publishedCommand, recovered.error)
            : new ProjectTransactionPublicationUnconfirmedError(publishedCommand);
        } else {
          throw commandError;
        }
      }
      const settleTerminal = ports.createAppProjectTransactionTerminalSettlement(identity);
      await ports.commitProjectTransactionWithRecovery(transaction, identity, settleTerminal);
      return result;
    } catch (error) {
      // A failed cleanup is deliberately retained and rethrown to the
      // foreground; a second Cancel would obscure the exact unresolved receipt.
      if (shouldCancelOpenedTransaction(error)) {
        await cancelOpenedProjectTransaction();
      }
      throw error;
    }
  };

  return { executeProjectTransactionMutation };
};
