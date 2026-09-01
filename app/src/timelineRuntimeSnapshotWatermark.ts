import type { EngineSnapshot } from "./types";

/**
 * Renderer-side provenance for an engine snapshot ingress. The backend does
 * not attach E/R/H to ordinary `get_snapshot` responses, so the caller must
 * carry the read guard that admitted the request to this single comparison
 * seam. Authority-bound bundles use their exact E/R/H image instead.
 */
export type TimelineRuntimeSnapshotScope = Readonly<{
  project_epoch: number;
  project_revision: number;
  checkpoint_hash: string;
  project_read_generation: number;
}>;

export type TimelineRuntimeSnapshotWatermark = Readonly<{
  transport_epoch: number;
  transport_generation: number;
  loop_generation: number;
  follow_generation: number;
}>;

const isCounter = (value: unknown): value is number =>
  typeof value === "number" && Number.isSafeInteger(value) && value >= 0;

const isTransportCounter = (value: unknown): value is number =>
  typeof value === "number" && Number.isSafeInteger(value) && value > 0;

const scopesEqual = (
  left: TimelineRuntimeSnapshotScope,
  right: TimelineRuntimeSnapshotScope,
) => left.project_epoch === right.project_epoch
  && left.project_revision === right.project_revision
  && left.checkpoint_hash === right.checkpoint_hash
  && left.project_read_generation === right.project_read_generation;

const isScope = (value: TimelineRuntimeSnapshotScope): boolean =>
  isCounter(value.project_epoch)
  && isCounter(value.project_revision)
  && typeof value.checkpoint_hash === "string"
  && isCounter(value.project_read_generation);

const compareTransportPair = (
  left: TimelineRuntimeSnapshotWatermark,
  right: TimelineRuntimeSnapshotWatermark,
) => left.transport_epoch !== right.transport_epoch
  ? left.transport_epoch - right.transport_epoch
  : left.transport_generation - right.transport_generation;

/**
 * Extract only the runtime fence that is authoritative for renderer snapshot
 * ordering. Missing or malformed runtime fields deliberately produce null:
 * callers must keep the last accepted image rather than synthesize a lower
 * generation from an incomplete IPC payload.
 */
export const timelineRuntimeSnapshotWatermarkFromEngineSnapshot = (
  snapshot: EngineSnapshot,
): TimelineRuntimeSnapshotWatermark | null => {
  const timeline = snapshot.timeline;
  const transportEpoch = timeline.transport_epoch;
  const transportGeneration = timeline.transport_generation;
  const loopGeneration = timeline.loop_runtime?.generation;
  const followGeneration = timeline.follow_runtime?.generation;
  if (!isTransportCounter(transportEpoch)
    || !isTransportCounter(transportGeneration)
    || !isCounter(loopGeneration)
    || !isCounter(followGeneration)) {
    return null;
  }
  return {
    transport_epoch: transportEpoch,
    transport_generation: transportGeneration,
    loop_generation: loopGeneration,
    follow_generation: followGeneration,
  };
};

/**
 * One shared monotonic admission gate for full, delta, polling, and explicit
 * canonical renderer snapshots. The transport epoch/generation pair is the
 * primary order. Loop and Follow are independently monotonic only within one
 * exact transport pair, allowing a later same-pair delta without permitting a
 * late A image to rewind either runtime generation.
 */
export const createTimelineRuntimeSnapshotWatermark = () => {
  let activeScope: TimelineRuntimeSnapshotScope | null = null;
  let accepted: TimelineRuntimeSnapshotWatermark | null = null;

  const resetForProjectScope = (scope: TimelineRuntimeSnapshotScope): boolean => {
    if (!isScope(scope)) return false;
    activeScope = { ...scope };
    accepted = null;
    return true;
  };

  const canAccept = (
    scope: TimelineRuntimeSnapshotScope,
    candidate: TimelineRuntimeSnapshotWatermark,
  ): boolean => {
    if (!isScope(scope)
      || !isTransportCounter(candidate.transport_epoch)
      || !isTransportCounter(candidate.transport_generation)
      || !isCounter(candidate.loop_generation)
      || !isCounter(candidate.follow_generation)) {
      return false;
    }
    if (activeScope === null) {
      activeScope = { ...scope };
      accepted = { ...candidate };
      return true;
    }
    if (!scopesEqual(activeScope, scope)) return false;
    if (accepted === null) {
      accepted = { ...candidate };
      return true;
    }
    const transportOrder = compareTransportPair(candidate, accepted);
    if (transportOrder < 0) return false;
    if (transportOrder === 0
      && (candidate.loop_generation < accepted.loop_generation
        || candidate.follow_generation < accepted.follow_generation)) {
      return false;
    }
    accepted = { ...candidate };
    return true;
  };

  return {
    resetForProjectScope,
    canAccept,
    current: () => activeScope === null || accepted === null
      ? null
      : { scope: { ...activeScope }, watermark: { ...accepted } },
  };
};
