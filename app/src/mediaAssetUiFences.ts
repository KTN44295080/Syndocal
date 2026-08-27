/**
 * Availability is machine-local UI state, but overlapping inspections still
 * need a per-asset latest-request-wins fence. A later Verify for asset 1 must
 * not discard a non-overlapping asset 2 result from an earlier Verify All.
 * Reset advances the generation and clears ownership so every prior project
 * operation is rejected, even if a new project reuses the same asset ID.
 */
export function createMediaAssetAvailabilityApplyFence() {
  let nextGeneration = 0;
  let latestStatusGeneration = 0;
  const latestGenerationByAsset = new Map<number, number>();
  const allocateGeneration = () => {
    if (nextGeneration >= Number.MAX_SAFE_INTEGER) {
      throw new Error("Media availability request generations are exhausted; restart Syndocal before verifying again.");
    }
    nextGeneration += 1;
    return nextGeneration;
  };
  const reserveAssetsAtGeneration = (assetIds: readonly number[], generation: number) => {
    const reservation = new Map<number, number>();
    for (const assetId of assetIds) {
      latestGenerationByAsset.set(assetId, generation);
      reservation.set(assetId, generation);
    }
    return reservation;
  };
  return {
    // A verification owns both per-asset application and the aggregate status
    // line. Relink deliberately has no reservation here: a successful Relink
    // decides whether to clear its row from the row's terminal E/R/H, rather
    // than cancelling a concurrently completing Verify request by invocation
    // order alone.
    reserveVerification: (assetIds: readonly number[]) => {
      const generation = allocateGeneration();
      // A verification reports one aggregate status line as well as individual
      // asset values.  The aggregate has stricter ownership than the entries:
      // a later Verify [1] may leave A's asset 2 applicable, but A must never
      // replace B's status line after B has begun.
      latestStatusGeneration = generation;
      return {
        assets: reserveAssetsAtGeneration(assetIds, generation),
        statusGeneration: generation,
      };
    },
    canApply: (reservation: ReadonlyMap<number, number>, assetId: number) => {
      const generation = reservation.get(assetId);
      return generation !== undefined && latestGenerationByAsset.get(assetId) === generation;
    },
    canPublishStatus: (reservation: { statusGeneration: number }) => {
      return latestStatusGeneration === reservation.statusGeneration;
    },
    reset: () => {
      latestStatusGeneration = allocateGeneration();
      latestGenerationByAsset.clear();
    },
  };
}

/**
 * Verify and Relink share the Media Library's single aggregate status line.
 * Row availability remains deliberately per-asset above, but this lease makes
 * the visible status line latest-invocation-wins across both operations. A
 * reset revokes every prior project operation before an ID can be reused.
 */
export function createMediaLibraryStatusLease() {
  let generation = 0;
  const advance = () => {
    if (generation >= Number.MAX_SAFE_INTEGER) {
      throw new Error("Media Library status generations are exhausted; restart Syndocal before trying again.");
    }
    generation += 1;
    return generation;
  };
  return {
    begin: () => advance(),
    reset: () => advance(),
    isCurrent: (lease: number) => generation === lease,
  };
}

type MediaAssetAvailabilityAuthority = {
  project_epoch: number;
  project_revision: number;
  checkpoint_hash: string;
};

/**
 * A machine-local availability row has its own authoritative terminal result.
 * Keep that E/R/H beside, rather than inside, the serializable availability
 * payload so Relink can clear only a row that did not already verify the exact
 * same post-relink project image.
 */
export function createMediaAssetAvailabilityRowAuthority() {
  const terminalAuthorityByAsset = new Map<number, MediaAssetAvailabilityAuthority>();
  const matches = (left: MediaAssetAvailabilityAuthority, right: MediaAssetAvailabilityAuthority) =>
    left.project_epoch === right.project_epoch
    && left.project_revision === right.project_revision
    && left.checkpoint_hash === right.checkpoint_hash;
  return {
    record: (assetId: number, authority: MediaAssetAvailabilityAuthority) => {
      terminalAuthorityByAsset.set(assetId, { ...authority });
    },
    shouldClearAfterRelink: (assetId: number, relinkAuthority: MediaAssetAvailabilityAuthority) => {
      const recorded = terminalAuthorityByAsset.get(assetId);
      if (recorded && matches(recorded, relinkAuthority)) return false;
      terminalAuthorityByAsset.delete(assetId);
      return true;
    },
    reset: () => terminalAuthorityByAsset.clear(),
  };
}

/**
 * Bootstrap busy state is a local lease, not an authority-derived boolean.
 * Project replacement revokes the old lease before making its UI visible, so a
 * delayed A finally cannot clear a newer C setup in the same empty show.
 */
export function createVjFirstRunOperationLease() {
  let generation = 0;
  const advance = () => {
    if (!Number.isSafeInteger(generation) || generation >= Number.MAX_SAFE_INTEGER) {
      throw new Error("First-run VJ operation generations are exhausted; restart Syndocal before trying again.");
    }
    generation += 1;
    return generation;
  };
  return {
    begin: () => advance(),
    reset: () => advance(),
    isCurrent: (lease: number) => generation === lease,
    applyIfCurrent: (lease: number, apply: () => void): boolean => {
      if (generation !== lease) return false;
      apply();
      return true;
    },
  };
}

/**
 * Mapping preflight never guesses that a same-epoch revision is ours. The
 * result is admissible only when it is the original exact E/R/H or the exact
 * terminal E/R/H returned by this renderer's successful mapping CAS ACK.
 */
export function mediaAssetMappingPreflightProvenance(
  expected: MediaAssetAvailabilityAuthority,
  current: MediaAssetAvailabilityAuthority,
  ownAcknowledgements: readonly MediaAssetAvailabilityAuthority[],
): "unchanged" | "own_mapping_ack" | null {
  const matches = (left: MediaAssetAvailabilityAuthority, right: MediaAssetAvailabilityAuthority) =>
    left.project_epoch === right.project_epoch
    && left.project_revision === right.project_revision
    && left.checkpoint_hash === right.checkpoint_hash;
  if (matches(expected, current)) return "unchanged";
  return ownAcknowledgements.some((acknowledgement) => matches(acknowledgement, current))
    ? "own_mapping_ack"
    : null;
}
