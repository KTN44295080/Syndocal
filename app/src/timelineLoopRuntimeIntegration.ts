import type { FrontendTauriInvoke, FrontendTauriInvokeCommand } from "./tauriInvokeCommands";
import type {
  EngineSnapshot,
  ProjectAuthorityBundle,
} from "./types";
import type {
  TimelineLoopRuntimeAcknowledgement,
  TimelineLoopRuntimeScope,
} from "./timelineLoopRuntimeController";
import {
  hydrateTimelineRuntimeSnapshot,
  projectAuthorityBundleTimelineRuntimeFromUnknown,
} from "./timelineRuntimeSnapshotWire";

export type TimelineLoopRuntimeProjectReadGuard = Readonly<{
  generation: number;
  authority: Readonly<{
    project_epoch: number;
    project_revision: number;
    checkpoint_hash: string;
  }>;
}>;

export type TimelineLoopRuntimeAuthorityIdentity = Readonly<{
  project_epoch: number;
  project_revision: number;
  checkpoint_hash: string;
}>;

export type TimelineLoopRuntimeSnapshotRequestGuard = Readonly<{
  beginFull: () => void;
  finishFull: () => void;
}>;

export type TimelineLoopRuntimeIntegrationOptions = {
  captureAuthority: () => TimelineLoopRuntimeAuthorityIdentity;
  projectReadGeneration: () => number;
  isProjectAuthorityIdentityCurrent: (authority: TimelineLoopRuntimeAuthorityIdentity) => boolean;
  captureProjectReadGuard: () => TimelineLoopRuntimeProjectReadGuard;
  projectReadGuardIsCurrent: (guard: TimelineLoopRuntimeProjectReadGuard) => boolean;
  isTauriRuntime: () => boolean;
  tauriBackendUnavailableMessage: string;
  timelineAuthorityReady: (operation: string) => boolean;
  tauriInvoke: FrontendTauriInvoke;
  beginTimelineTransportCanonicalSnapshotConvergence: () => number;
  timelineTransportCanonicalSnapshotGeneration: () => number;
  snapshotRequestGuard: TimelineLoopRuntimeSnapshotRequestGuard;
  projectAuthorityTokenIsCurrent: (
    expected: TimelineLoopRuntimeAuthorityIdentity,
    current: TimelineLoopRuntimeAuthorityIdentity,
  ) => boolean;
  authorityToken: (bundle: ProjectAuthorityBundle) => TimelineLoopRuntimeAuthorityIdentity;
  applyEngineSnapshot: (
    snapshot: EngineSnapshot,
    timelineRuntime: unknown,
    projectReadGuard: TimelineLoopRuntimeProjectReadGuard,
  ) => boolean;
  setSnapshotRevision: (revision: null) => void;
};

export const createTimelineLoopRuntimeIntegration = (
  options: TimelineLoopRuntimeIntegrationOptions,
) => {
  const captureScope = (): TimelineLoopRuntimeScope => {
    const authority = options.captureAuthority();
    return {
      project_epoch: authority.project_epoch,
      project_revision: authority.project_revision,
      checkpoint_hash: authority.checkpoint_hash,
      project_read_generation: options.projectReadGeneration(),
    };
  };

  const scopeIsCurrent = (scope: TimelineLoopRuntimeScope) =>
    scope.project_read_generation === options.projectReadGeneration()
    && options.isProjectAuthorityIdentityCurrent({
      project_epoch: scope.project_epoch,
      project_revision: scope.project_revision,
      checkpoint_hash: scope.checkpoint_hash,
    });

  const invoke = async <T,>(
    command: FrontendTauriInvokeCommand,
    args?: Record<string, unknown>,
  ): Promise<T> => {
    if (command !== "query_timeline_loop_runtime_authority_v1"
      && command !== "commit_timeline_loop_runtime_v1") {
      throw new Error("Timeline loop dispatcher rejected an unknown command.");
    }
    if (!options.isTauriRuntime()) throw new Error(options.tauriBackendUnavailableMessage);
    if (command === "commit_timeline_loop_runtime_v1"
      && !options.timelineAuthorityReady("Timeline loop change")) {
      throw new Error("Timeline authority is not ready; command was not sent.");
    }
    return options.tauriInvoke<T>(command, args);
  };

  const refreshCanonicalSnapshot = async (
    acknowledgement: TimelineLoopRuntimeAcknowledgement,
  ) => {
    const expectedAuthority = {
      project_epoch: acknowledgement.scope.project_epoch,
      project_revision: acknowledgement.scope.project_revision,
      checkpoint_hash: acknowledgement.scope.checkpoint_hash,
    };
    if (!scopeIsCurrent(acknowledgement.scope)) {
      throw new Error("Timeline loop canonical snapshot was superseded before it could be read.");
    }
    const readGuard = options.captureProjectReadGuard();
    // Both root Timeline runtime lanes share the full-read convergence barrier
    // so a delayed generic image cannot overwrite an acknowledged receipt.
    const convergenceGeneration = options.beginTimelineTransportCanonicalSnapshotConvergence();
    options.snapshotRequestGuard.beginFull();
    try {
      const canonical = await options.tauriInvoke<ProjectAuthorityBundle>("get_project_authority_bundle", {
        expectedEpoch: expectedAuthority.project_epoch,
        expectedRevision: expectedAuthority.project_revision,
        expectedCheckpointHash: expectedAuthority.checkpoint_hash,
      });
      if (!options.projectReadGuardIsCurrent(readGuard)
        || convergenceGeneration !== options.timelineTransportCanonicalSnapshotGeneration()
        || !scopeIsCurrent(acknowledgement.scope)
        || !options.isProjectAuthorityIdentityCurrent(expectedAuthority)
        || !options.projectAuthorityTokenIsCurrent(expectedAuthority, options.authorityToken(canonical))
        || canonical.publication_generation
          !== acknowledgement.fenceBefore.project.project_publication_generation) {
        throw new Error("Timeline loop canonical snapshot was superseded before it could be applied.");
      }
      const timelineRuntime = projectAuthorityBundleTimelineRuntimeFromUnknown(canonical);
      const hydrated = timelineRuntime === null
        ? null
        : hydrateTimelineRuntimeSnapshot(canonical.snapshot, timelineRuntime);
      if (hydrated === null || timelineRuntime === null) {
        throw new Error("Timeline loop canonical snapshot omitted or malformed its runtime projection.");
      }
      const timeline = hydrated.timeline;
      const loopGeneration = timeline.loop_runtime?.generation ?? 0;
      const followGeneration = timeline.follow_runtime?.generation ?? 0;
      if (!Number.isSafeInteger(loopGeneration)
        || loopGeneration < 0
        || !Number.isSafeInteger(followGeneration)
        || followGeneration < 0
        || timelineRuntime.transport_epoch !== acknowledgement.epochAfter
        || timelineRuntime.transport_generation !== acknowledgement.generationAfter
        || loopGeneration !== acknowledgement.loopGenerationAfter
        || followGeneration !== acknowledgement.followGenerationAfter
        || (acknowledgement.requestedAction.kind === "set_enabled"
          && ((timeline.loop_runtime?.status ?? "disabled") !== "disabled")
            !== acknowledgement.requestedAction.enabled)) {
        throw new Error(
          "Timeline loop canonical snapshot did not converge to the acknowledged runtime state.",
        );
      }
      // Capture the guard before the asynchronous read and carry that exact
      // identity across the accepted snapshot seam. Capturing again at apply
      // time would allow a newer project identity to bless this older read.
      const applied = options.applyEngineSnapshot(canonical.snapshot, timelineRuntime, readGuard);
      if (!applied) {
        throw new Error("Timeline loop canonical snapshot had a stale or malformed runtime watermark.");
      }
      options.setSnapshotRevision(null);
    } finally {
      options.snapshotRequestGuard.finishFull();
    }
  };

  return {
    captureScope,
    scopeIsCurrent,
    invoke: <T,>(command: FrontendTauriInvokeCommand, args?: Record<string, unknown>) =>
      invoke<T>(command, args),
    refreshCanonicalSnapshot,
  };
};
