import type { FrontendTauriInvoke } from "./tauriInvokeCommands";
import type { EngineSnapshot, EngineSnapshotRuntimeWireResponse } from "./types";
import { engineSnapshotRuntimeWireResponseFromUnknown } from "./timelineRuntimeSnapshotWire";

/** The E/R/H identity captured when a full snapshot request is queued. */
export type TimelineSnapshotProjectReadGuard = Readonly<{
  generation: number;
  authority: Readonly<{
    project_epoch: number;
    project_revision: number;
    checkpoint_hash: string;
  }>;
}>;

export type TimelineSnapshotRefreshRequestGuard = Readonly<{
  beginFull: () => void;
  finishFull: () => void;
}>;

export type TimelineSnapshotRefreshControllerOptions = {
  captureProjectReadGuard: () => TimelineSnapshotProjectReadGuard;
  projectReadGuardIsCurrent: (guard: TimelineSnapshotProjectReadGuard) => boolean;
  invoke: FrontendTauriInvoke;
  isTauriRuntime: () => boolean;
  snapshotRequestGuard: TimelineSnapshotRefreshRequestGuard;
  timelineTransportCanonicalSnapshotGeneration: () => number;
  refreshOperatorPolicy: (force: boolean) => Promise<unknown>;
  refreshFixtureGroups: () => Promise<unknown>;
  setFixtureGroupDeleteUndoAvailable: (available: boolean) => void;
  setViewportFixtureGroupDeleteUndo: (value: null) => void;
  setMessage: (message: string) => void;
};

export type TimelineSnapshotApply = (
  incoming: EngineSnapshot,
  timelineRuntime: unknown,
  syncProjectState: boolean,
  resetEditorDrafts: boolean,
  projectReadGuard: TimelineSnapshotProjectReadGuard,
) => boolean;

type SnapshotRefreshWaiter = {
  syncProjectState: boolean;
  resetEditorDrafts: boolean;
  projectReadGuard: TimelineSnapshotProjectReadGuard;
  resolve: (snapshot: EngineSnapshot | null) => void;
};

/**
 * Serializes full snapshot reads and batches only requests from the same
 * project-read generation. The caller owns the guarded apply seam; this
 * module owns queueing, async revalidation, and waiter resolution.
 */
export const createTimelineSnapshotRefreshController = (
  options: TimelineSnapshotRefreshControllerOptions,
) => {
  const pendingFullSnapshotRefreshes: SnapshotRefreshWaiter[] = [];
  let fullSnapshotRefreshRunning = false;

  const run = async (applySnapshot: TimelineSnapshotApply) => {
    if (fullSnapshotRefreshRunning) return;
    fullSnapshotRefreshRunning = true;
    try {
      while (pendingFullSnapshotRefreshes.length > 0) {
        const requestedReadGuard = pendingFullSnapshotRefreshes[0].projectReadGuard;
        const batch: SnapshotRefreshWaiter[] = [];
        for (let index = pendingFullSnapshotRefreshes.length - 1; index >= 0; index -= 1) {
          if (pendingFullSnapshotRefreshes[index].projectReadGuard.generation
            === requestedReadGuard.generation) {
            const [waiter] = pendingFullSnapshotRefreshes.splice(index, 1);
            batch.unshift(waiter);
          }
        }
        const timelineTransportGenerationAtRequest =
          options.timelineTransportCanonicalSnapshotGeneration();
        options.snapshotRequestGuard.beginFull();
        let next: EngineSnapshot | null = null;
        try {
          const response = await options.invoke<unknown>("get_snapshot");
          const candidate = engineSnapshotRuntimeWireResponseFromUnknown(response)
            ?? (!options.isTauriRuntime() && response !== null && typeof response === "object"
              ? { snapshot: response as EngineSnapshot, timeline_runtime: null as unknown as EngineSnapshotRuntimeWireResponse["timeline_runtime"] }
              : null);
          if (candidate === null) throw new Error("Full snapshot omitted or malformed its Timeline runtime projection.");
          if (options.projectReadGuardIsCurrent(requestedReadGuard)
            && timelineTransportGenerationAtRequest
              === options.timelineTransportCanonicalSnapshotGeneration()) {
            next = candidate.snapshot;
            if (batch.some((waiter) => waiter.resetEditorDrafts)) {
              await options.refreshOperatorPolicy(true);
              await options.refreshFixtureGroups();
              if (!options.projectReadGuardIsCurrent(requestedReadGuard)
                || timelineTransportGenerationAtRequest
                  !== options.timelineTransportCanonicalSnapshotGeneration()) {
                next = null;
              } else {
                options.setFixtureGroupDeleteUndoAvailable(false);
                options.setViewportFixtureGroupDeleteUndo(null);
              }
            }
            // The policy/group refresh above is async. A canonical runtime
            // snapshot can win while it settles, so repeat the same fence
            // immediately before the only full-image apply.
            if (next !== null
              && options.projectReadGuardIsCurrent(requestedReadGuard)
              && timelineTransportGenerationAtRequest
                === options.timelineTransportCanonicalSnapshotGeneration()) {
              if (!applySnapshot(
                next,
                candidate.timeline_runtime,
                batch.some((waiter) => waiter.syncProjectState),
                batch.some((waiter) => waiter.resetEditorDrafts),
                requestedReadGuard,
              )) {
                next = null;
              }
            }
          }
        } catch (error) {
          // Report only errors from the still-current project. A superseded
          // read is intentionally silent and resolves as stale.
          if (options.projectReadGuardIsCurrent(requestedReadGuard)) {
            options.setMessage(String(error));
          }
        } finally {
          options.snapshotRequestGuard.finishFull();
        }
        for (const waiter of batch) waiter.resolve(next);
      }
    } finally {
      fullSnapshotRefreshRunning = false;
    }
  };

  const refresh = (
    syncProjectState: boolean,
    resetEditorDrafts: boolean,
    runRefreshes: () => void,
  ): Promise<EngineSnapshot | null> => new Promise((resolve) => {
    pendingFullSnapshotRefreshes.push({
      syncProjectState,
      resetEditorDrafts,
      projectReadGuard: options.captureProjectReadGuard(),
      resolve,
    });
    runRefreshes();
  });

  return { refresh, run };
};
