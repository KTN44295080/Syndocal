export interface SnapshotRequestGuard {
  beginFull: () => void;
  finishFull: () => void;
  beginDelta: () => number | null;
  canApplyDelta: (generation: number) => boolean;
}

export const createSnapshotRequestGuard = (): SnapshotRequestGuard => {
  let deltaGeneration = 0;
  let pendingFullRequests = 0;

  return {
    beginFull: () => {
      pendingFullRequests += 1;
      deltaGeneration += 1;
    },
    finishFull: () => {
      pendingFullRequests = Math.max(0, pendingFullRequests - 1);
    },
    beginDelta: () => {
      if (pendingFullRequests > 0) return null;
      deltaGeneration += 1;
      return deltaGeneration;
    },
    canApplyDelta: (generation) => pendingFullRequests === 0 && generation === deltaGeneration,
  };
};
