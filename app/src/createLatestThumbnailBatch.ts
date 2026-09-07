type CurrentBatch = () => boolean;
interface ThumbnailBatch {
  generation: number;
  run: (isCurrent: CurrentBatch) => Promise<void>;
  onError: (error: unknown) => void;
}

/** One running batch and one replaceable successor; no detached native reads. */
export const createLatestThumbnailBatch = () => {
  let generation = 0;
  let running = false;
  let disposed = false;
  let pending: ThumbnailBatch | undefined;
  const clear = () => {
    generation += 1;
    pending = undefined;
  };
  const drain = async () => {
    running = true;
    try {
      while (!disposed && pending) {
        const batch = pending;
        pending = undefined;
        const isCurrent = () => !disposed && generation === batch.generation;
        try {
          await batch.run(isCurrent);
        } catch (error) {
          if (isCurrent()) batch.onError(error);
        }
      }
    } finally {
      running = false;
    }
  };
  return {
    replace(run: ThumbnailBatch["run"], onError: ThumbnailBatch["onError"]) {
      if (disposed) return;
      pending = { generation: ++generation, run, onError };
      if (!running) void drain();
    },
    clear,
    dispose() {
      disposed = true;
      clear();
    },
  };
};
