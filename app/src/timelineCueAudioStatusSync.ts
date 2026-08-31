export interface TimelineCueAudioStatusFence {
  runtimeIncarnation: number;
  statusRevision: number;
}

export interface TimelineCueAudioStatusRequestGate {
  beginMutation: () => number;
  endMutation: (requestEpoch: number) => void;
  beginPoll: () => number | null;
  canBeginPoll: () => boolean;
  endPoll: () => void;
  acceptsRequest: (requestEpoch: number) => boolean;
  acceptsStatus: (status: TimelineCueAudioStatusFence) => boolean;
  commitStatus: (status: TimelineCueAudioStatusFence) => boolean;
  invalidate: () => void;
}

const isFiniteNonNegativeSafeInteger = (value: number) => Number.isSafeInteger(value)
  && value >= 0;

const isValidFence = (status: TimelineCueAudioStatusFence) =>
  isFiniteNonNegativeSafeInteger(status.runtimeIncarnation)
  && status.runtimeIncarnation > 0
  && isFiniteNonNegativeSafeInteger(status.statusRevision);

/**
 * The backend exposes no speculative state. Keep a monotonic frontend receipt
 * fence. Runtime incarnation plus the checked status revision are the sole
 * ordering domain; output frame and topology values are diagnostic snapshots
 * that may legitimately reset as the runtime detaches or restarts.
 */
export const timelineCueAudioStatusCanApply = (
  current: TimelineCueAudioStatusFence | null,
  candidate: TimelineCueAudioStatusFence,
) => {
  if (!isValidFence(candidate)) return false;
  if (!current) return true;
  if (!isValidFence(current)) return false;
  return candidate.runtimeIncarnation !== current.runtimeIncarnation
    || candidate.statusRevision >= current.statusRevision;
};

export const createTimelineCueAudioStatusRequestGate = (): TimelineCueAudioStatusRequestGate => {
  let requestEpoch = 0;
  let pollInFlight = false;
  let mutationInFlight = false;
  let applied: TimelineCueAudioStatusFence | null = null;
  const retiredRuntimeIncarnations = new Set<number>();

  return {
    beginMutation: () => {
      requestEpoch += 1;
      mutationInFlight = true;
      return requestEpoch;
    },
    endMutation: (mutationEpoch) => {
      if (mutationEpoch === requestEpoch) mutationInFlight = false;
    },
    beginPoll: () => {
      if (pollInFlight || mutationInFlight) return null;
      pollInFlight = true;
      return requestEpoch;
    },
    canBeginPoll: () => !pollInFlight && !mutationInFlight,
    endPoll: () => {
      pollInFlight = false;
    },
    acceptsRequest: (candidateEpoch) => candidateEpoch === requestEpoch,
    acceptsStatus: (status) => {
      if (!timelineCueAudioStatusCanApply(applied, status)) return false;
      return !applied
        || status.runtimeIncarnation === applied.runtimeIncarnation
        || !retiredRuntimeIncarnations.has(status.runtimeIncarnation);
    },
    commitStatus: (status) => {
      if (!timelineCueAudioStatusCanApply(applied, status)) return false;
      if (
        applied
        && status.runtimeIncarnation !== applied.runtimeIncarnation
        && retiredRuntimeIncarnations.has(status.runtimeIncarnation)
      ) return false;
      if (applied && status.runtimeIncarnation !== applied.runtimeIncarnation) {
        retiredRuntimeIncarnations.add(applied.runtimeIncarnation);
      }
      applied = status;
      return true;
    },
    invalidate: () => {
      requestEpoch += 1;
      mutationInFlight = false;
      applied = null;
      retiredRuntimeIncarnations.clear();
    },
  };
};

export interface TimelineCueAudioRefreshQueue<T> {
  request: () => Promise<T | null>;
  notifyAvailable: () => void;
}

export interface TimelineCueAudioRefreshQueueOptions<T> {
  canRun: () => boolean;
  run: () => Promise<T | null>;
}

/**
 * Coalesces explicit topology refreshes and waits for an in-flight status
 * poll or settings mutation to settle.  The queue never starts two list/status
 * transactions at once; callers receive the same eventual result.
 */
export const createTimelineCueAudioRefreshQueue = <T>(
  options: TimelineCueAudioRefreshQueueOptions<T>,
): TimelineCueAudioRefreshQueue<T> => {
  let pending: Array<{ resolve: (value: T | null) => void; reject: (reason: unknown) => void }> = [];
  let running = false;

  const drain = async () => {
    if (running || pending.length === 0 || !options.canRun()) return;
    running = true;
    const waiters = pending;
    pending = [];
    try {
      const result = await options.run();
      waiters.forEach(({ resolve }) => resolve(result));
    } catch (error) {
      waiters.forEach(({ reject }) => reject(error));
    } finally {
      running = false;
      if (pending.length > 0) queueMicrotask(() => void drain());
    }
  };

  return {
    request: () => new Promise<T | null>((resolve, reject) => {
      pending.push({ resolve, reject });
      void drain();
    }),
    notifyAvailable: () => void drain(),
  };
};

/**
 * Serializes the Setup Audio mount refresh with the operator's route/device
 * write. The native list/status operation and settings mutation use separate
 * commands, so the UI owns one small transaction boundary around both. A
 * queued operation re-checks mutation settlement immediately before entering
 * the other direction; it never starts a refresh midway through a selection,
 * nor publishes a selection over an incomplete topology receipt.
 */
export interface TimelineCueAudioSetupOperationGate {
  refresh: () => Promise<boolean>;
  configure: (write: () => void | Promise<void>) => Promise<boolean>;
  dispose: () => void;
}

export interface TimelineCueAudioSetupOperationGateOptions {
  waitForMutationIdle: () => Promise<void>;
  waitForMutationTerminal: () => Promise<void>;
  refresh: () => void | Promise<void>;
  onBusy: (busy: boolean) => void;
}

export const createTimelineCueAudioSetupOperationGate = (
  options: TimelineCueAudioSetupOperationGateOptions,
): TimelineCueAudioSetupOperationGate => {
  let tail: Promise<void> = Promise.resolve();
  let queued = 0;
  let busy = false;
  let disposed = false;

  const setBusy = (next: boolean) => {
    if (busy === next) return;
    busy = next;
    options.onBusy(next);
  };

  const enqueue = (operation: () => Promise<void>): Promise<boolean> => {
    if (disposed) return Promise.resolve(false);
    queued += 1;
    setBusy(true);
    const completion = tail.then(async () => {
      if (disposed) return false;
      await operation();
      return !disposed;
    });
    tail = completion.then(() => undefined, () => undefined);
    return completion.finally(() => {
      queued -= 1;
      if (!disposed && queued === 0) setBusy(false);
    });
  };

  return {
    refresh: () => enqueue(async () => {
      await options.waitForMutationIdle();
      if (disposed) return;
      await options.refresh();
    }),
    configure: (write) => enqueue(async () => {
      await options.waitForMutationIdle();
      if (disposed) return;
      await write();
      if (disposed) return;
      await options.waitForMutationTerminal();
    }),
    dispose: () => {
      disposed = true;
      setBusy(false);
    },
  };
};

export interface TimelineCueAudioSettingsQueueOptions<Settings, Status> {
  send: (settings: Settings) => Promise<Status>;
  fenceOf: (status: Status) => TimelineCueAudioStatusFence;
  gate: TimelineCueAudioStatusRequestGate;
  onStatus: (status: Status) => void;
  onError: (error: unknown) => void;
  onBusy: (busy: boolean) => void;
}

export interface TimelineCueAudioSettingsQueue<Settings> {
  submit: (settings: Settings) => void;
  dispose: () => void;
}

/** Serializes backend writes while retaining only the newest requested settings. */
export const createTimelineCueAudioSettingsQueue = <Settings, Status>(
  options: TimelineCueAudioSettingsQueueOptions<Settings, Status>,
): TimelineCueAudioSettingsQueue<Settings> => {
  let pending: { settings: Settings; desiredEpoch: number } | null = null;
  let desiredEpoch = 0;
  let running = false;
  let disposed = false;

  const drain = async () => {
    if (running || disposed) return;
    running = true;
    options.onBusy(true);
    try {
      while (!disposed && pending) {
        const next = pending;
        pending = null;
        const requestEpoch = options.gate.beginMutation();
        try {
          const status = await options.send(next.settings);
          if (
            !disposed
            && next.desiredEpoch === desiredEpoch
            && options.gate.acceptsRequest(requestEpoch)
            && options.gate.commitStatus(options.fenceOf(status))
          ) options.onStatus(status);
        } catch (error) {
          if (!disposed && next.desiredEpoch === desiredEpoch && options.gate.acceptsRequest(requestEpoch)) {
            options.onError(error);
          }
        } finally {
          options.gate.endMutation(requestEpoch);
        }
      }
    } finally {
      running = false;
      if (!disposed) options.onBusy(false);
      if (!disposed && pending) void drain();
    }
  };

  return {
    submit: (settings) => {
      if (disposed) return;
      desiredEpoch += 1;
      pending = { settings, desiredEpoch };
      void drain();
    },
    dispose: () => {
      disposed = true;
      pending = null;
      options.gate.invalidate();
    },
  };
};
