type CurrentBatch = () => boolean;
interface ThumbnailBatch {
  generation: number;
  run: (isCurrent: CurrentBatch, signal: AbortSignal) => Promise<void>;
  onError: (error: unknown) => void;
}

/** One running batch and one replaceable successor; no detached native reads. */
export const createLatestThumbnailBatch = (onBusyChange: (busy: boolean) => void = () => {}) => {
  let generation = 0;
  let running = false;
  let disposed = false;
  let pending: ThumbnailBatch | undefined;
  let activeController: AbortController | undefined;
  const clear = () => {
    generation += 1;
    pending = undefined;
    activeController?.abort();
  };
  const drain = async () => {
    running = true;
    onBusyChange(true);
    try {
      while (!disposed && pending) {
        const batch = pending;
        pending = undefined;
        const isCurrent = () => !disposed && generation === batch.generation;
        const controller = new AbortController();
        activeController = controller;
        try {
          await batch.run(isCurrent, controller.signal);
        } catch (error) {
          if (isCurrent()) batch.onError(error);
        } finally {
          if (activeController === controller) activeController = undefined;
        }
      }
    } finally {
      running = false;
      onBusyChange(false);
    }
  };
  return {
    replace(run: ThumbnailBatch["run"], onError: ThumbnailBatch["onError"]) {
      if (disposed) return;
      pending = { generation: ++generation, run, onError };
      activeController?.abort();
      if (!running) void drain();
    },
    clear,
    dispose() {
      disposed = true;
      clear();
    },
  };
};
