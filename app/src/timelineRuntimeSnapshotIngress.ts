import type { EngineSnapshot } from "./types";
import {
  createTimelineRuntimeSnapshotWatermark,
  timelineRuntimeSnapshotWatermarkFromEngineSnapshot,
} from "./timelineRuntimeSnapshotWatermark";
import { hydrateTimelineRuntimeSnapshot } from "./timelineRuntimeSnapshotWire";
import type { TimelineSnapshotProjectReadGuard } from "./timelineSnapshotRefreshController";

export type TimelineRuntimeSnapshotIngress = {
  projectReadGuard?: TimelineSnapshotProjectReadGuard;
  resetForProjectScope?: boolean;
  /** Required for every native snapshot ingress; never inferred from authored data. */
  timelineRuntime?: unknown;
};

export type TimelineRuntimeSnapshotIngressOptions = {
  captureProjectReadGuard: () => TimelineSnapshotProjectReadGuard;
  projectReadGuardIsCurrent: (guard: TimelineSnapshotProjectReadGuard) => boolean;
  normalizeEngineSnapshot: (snapshot: EngineSnapshot) => EngineSnapshot;
  isTauriRuntime: () => boolean;
};

/**
 * Shared renderer ingress seam for full, delta, polling, and canonical
 * snapshots. This keeps the E/R/H guard and monotonic runtime watermark in
 * one focused module while leaving UI application to App's existing callback.
 */
export const createTimelineRuntimeSnapshotIngress = (
  options: TimelineRuntimeSnapshotIngressOptions,
) => {
  const timelineRuntimeSnapshotWatermark = createTimelineRuntimeSnapshotWatermark();

  const scopeFromReadGuard = (guard: TimelineSnapshotProjectReadGuard) => ({
    project_epoch: guard.authority.project_epoch,
    project_revision: guard.authority.project_revision,
    checkpoint_hash: guard.authority.checkpoint_hash,
    project_read_generation: guard.generation,
  });

  const prepare = (
    incoming: EngineSnapshot,
    ingress: TimelineRuntimeSnapshotIngress = {},
  ): EngineSnapshot | null => {
    const readGuard = ingress.projectReadGuard ?? options.captureProjectReadGuard();
    if (!options.projectReadGuardIsCurrent(readGuard)) return null;
    const hydrated = options.isTauriRuntime()
      ? hydrateTimelineRuntimeSnapshot(incoming, ingress.timelineRuntime)
      : incoming;
    if (hydrated === null) return null;
    const next = options.normalizeEngineSnapshot(hydrated);
    // Browser viewport fixtures intentionally model renderer-only state and
    // do not expose the native runtime fence. Every desktop ingress must
    // instead carry a complete backend-owned watermark or remain unapplied.
    if (!options.isTauriRuntime()) return next;
    const scope = scopeFromReadGuard(readGuard);
    if (ingress.resetForProjectScope
      && !timelineRuntimeSnapshotWatermark.resetForProjectScope(scope)) {
      return null;
    }
    const watermark = timelineRuntimeSnapshotWatermarkFromEngineSnapshot(next);
    if (watermark === null || !timelineRuntimeSnapshotWatermark.canAccept(scope, watermark)) {
      return null;
    }
    return next;
  };

  return {
    watermark: timelineRuntimeSnapshotWatermark,
    prepare,
  };
};
