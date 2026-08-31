export const SERIAL_DMX_STATUS_QUERY_TIMEOUT_MS = 1_500;

export interface SerialDmxStatusSnapshot<TBinding, TRoute> {
  binding: TBinding | null;
  route: TRoute | null;
}

type TimeoutHandle = ReturnType<typeof globalThis.setTimeout>;
type ScheduleTimeout = (callback: () => void, delayMs: number) => TimeoutHandle;
type CancelTimeout = (handle: TimeoutHandle) => void;

const isCanonicalPositiveDecimal = (value: string) => /^(?:[1-9]\d*)$/.test(value);

const compareCanonicalPositiveDecimals = (left: string, right: string) =>
  left.length === right.length
    ? left.localeCompare(right)
    : left.length - right.length;

interface SerialDmxStatusPollerOptions<TBinding, TRoute> {
  queryBinding: () => Promise<TBinding>;
  queryRoute: () => Promise<TRoute>;
  commit: (snapshot: SerialDmxStatusSnapshot<TBinding, TRoute>) => void;
  /** Return a redacted field-level reason, never raw device data. */
  validateSnapshot?: (binding: TBinding, route: TRoute) => string | null;
  /**
   * Return one canonical decimal revision only when binding and route came
   * from the same native fence. `null` is fail-closed.
   */
  coherentRouteStatusRevision?: (binding: TBinding, route: TRoute) => string | null;
  /** Keep status Unknown until the native spontaneous-fault event fence exists. */
  requireRouteStatusEventFence?: boolean;
  onInvalidSnapshot?: (reason: string) => void;
  timeoutMs?: number;
  scheduleTimeout?: ScheduleTimeout;
  cancelTimeout?: CancelTimeout;
}

export interface SerialDmxStatusPoller {
  refresh: () => Promise<void>;
  invalidate: () => void;
  /** Fence a native route-transition event before the next full status read. */
  invalidateAtOrAfterRouteStatusRevision: (revision: string) => boolean;
  /** A failed/uninstalled event subscription cannot permit an Active commit. */
  setRouteStatusEventFenceAvailable: (available: boolean) => void;
  dispose: () => void;
}

interface RawStatusFlight<TBinding, TRoute> {
  /** Resolves/rejects as soon as either endpoint determines the query result. */
  result: Promise<[TBinding, TRoute]>;
  /** Resolves only after both uncancellable native invokes have settled. */
  settled: Promise<void>;
}

/**
 * Bounded, generation-fenced status retrieval for the machine-local USB-DMX
 * worker. Tauri invokes cannot be cancelled after dispatch, so a timeout or
 * explicit invalidation advances the generation before a late response can
 * mutate the visible state.
 */
export function createSerialDmxStatusPoller<TBinding, TRoute>(
  options: SerialDmxStatusPollerOptions<TBinding, TRoute>,
): SerialDmxStatusPoller {
  const timeoutMs = options.timeoutMs ?? SERIAL_DMX_STATUS_QUERY_TIMEOUT_MS;
  const scheduleTimeout = options.scheduleTimeout ?? globalThis.setTimeout;
  const cancelTimeout = options.cancelTimeout ?? globalThis.clearTimeout;
  let disposed = false;
  let generation = 0;
  let minimumRouteStatusRevision: string | null = null;
  let lastCommittedRouteStatusRevision: string | null = null;
  let routeStatusEventFenceAvailable = options.requireRouteStatusEventFence !== true;
  let refreshFlight: Promise<void> | null = null;
  // Tauri invokes cannot be cancelled. Keep this slot until both native
  // requests settle even when the bounded UI waiter has timed out; otherwise
  // the one-second cadence can build unbounded overlapping COM-status calls.
  let rawFlight: RawStatusFlight<TBinding, TRoute> | null = null;

  const isCurrent = (token: number) => !disposed && generation === token;
  const commitUnknown = () => options.commit({ binding: null, route: null });
  const invalidate = () => {
    generation += 1;
    refreshFlight = null;
    if (!disposed) commitUnknown();
  };
  const invalidateAtOrAfterRouteStatusRevision = (revision: string): boolean => {
    if (!isCanonicalPositiveDecimal(revision)) {
      invalidate();
      return false;
    }
    if (
      minimumRouteStatusRevision === null
      || compareCanonicalPositiveDecimals(revision, minimumRouteStatusRevision) > 0
    ) {
      minimumRouteStatusRevision = revision;
    }
    invalidate();
    return true;
  };
  const setRouteStatusEventFenceAvailable = (available: boolean) => {
    routeStatusEventFenceAvailable = available;
    invalidate();
  };
  const awaitBounded = <T>(query: Promise<T>) => new Promise<T>((resolve, reject) => {
    let settled = false;
    const timeout = scheduleTimeout(() => {
      if (settled) return;
      settled = true;
      reject(new Error(`USB-DMX status query timed out after ${timeoutMs}ms`));
    }, timeoutMs);
    query.then(
      (value) => {
        if (settled) return;
        settled = true;
        cancelTimeout(timeout);
        resolve(value);
      },
      (error) => {
        if (settled) return;
        settled = true;
        cancelTimeout(timeout);
        reject(error);
      },
    );
  });
  const beginRawFlight = (): RawStatusFlight<TBinding, TRoute> => {
    const binding = Promise.resolve().then(options.queryBinding);
    const route = Promise.resolve().then(options.queryRoute);
    const result = Promise.all([binding, route]) as Promise<[TBinding, TRoute]>;
    const flight: RawStatusFlight<TBinding, TRoute> = {
      result,
      settled: Promise.allSettled([binding, route]).then(() => undefined),
    };
    rawFlight = flight;
    void flight.settled.then(() => {
      if (rawFlight === flight) rawFlight = null;
    });
    return flight;
  };

  const refresh = (): Promise<void> => {
    if (disposed) return Promise.resolve();
    if (!routeStatusEventFenceAvailable) {
      // A status query cannot observe a spontaneous worker fault between the
      // native read and frontend commit without its revision event. Keep the
      // UI Unknown rather than re-accepting a plausible but stale Active.
      commitUnknown();
      return Promise.resolve();
    }
    if (refreshFlight) return refreshFlight;
    // An old invoke may be indefinitely stuck below Tauri. Its result belongs
    // to a pre-timeout/pre-invalidation generation and must never be reused as
    // a fresh authority read; stay Unknown and wait for both raw calls to end.
    if (rawFlight) return Promise.resolve();

    const token = ++generation;
    const raw = beginRawFlight();
    const flight = awaitBounded(raw.result).then(
      ([binding, route]) => {
        if (!routeStatusEventFenceAvailable) {
          if (isCurrent(token)) {
            generation += 1;
            commitUnknown();
          }
          return;
        }
        let validationError: string | null = null;
        try {
          validationError = options.validateSnapshot?.(binding, route) ?? null;
        } catch {
          validationError = "USB-DMX status validation threw";
        }
        if (validationError) {
          if (!isCurrent(token)) return;
          // Treat a syntactically successful but semantically malformed IPC
          // payload exactly like a query failure. Advance first so it cannot
          // be followed by a late Active commit for this generation.
          generation += 1;
          try {
            options.onInvalidSnapshot?.(validationError);
          } catch {
            // Diagnostics must not prevent the fail-closed Unknown commit.
          }
          commitUnknown();
          return;
        }
        if (options.coherentRouteStatusRevision) {
          let revision: string | null = null;
          try {
            revision = options.coherentRouteStatusRevision(binding, route);
          } catch {
            revision = null;
          }
          const newerThanInvalidation = minimumRouteStatusRevision === null
            || (revision !== null
              && isCanonicalPositiveDecimal(revision)
              && compareCanonicalPositiveDecimals(revision, minimumRouteStatusRevision) >= 0);
          const notOlderThanCommitted = lastCommittedRouteStatusRevision === null
            || (revision !== null
              && isCanonicalPositiveDecimal(revision)
              && compareCanonicalPositiveDecimals(revision, lastCommittedRouteStatusRevision) >= 0);
          if (!revision || !isCanonicalPositiveDecimal(revision) || !newerThanInvalidation || !notOlderThanCommitted) {
            if (!isCurrent(token)) return;
            generation += 1;
            try {
              options.onInvalidSnapshot?.("USB-DMX route revision fence is malformed, incoherent, or stale");
            } catch {
              // Diagnostics must not prevent the fail-closed Unknown commit.
            }
            commitUnknown();
            return;
          }
          if (isCurrent(token)) lastCommittedRouteStatusRevision = revision;
        }
        if (isCurrent(token)) options.commit({ binding, route });
      },
      () => {
        if (!isCurrent(token)) return;
        // Make the timed-out/error generation stale before exposing Unknown,
        // so a late Active result cannot revive a stopped or faulted worker.
        generation += 1;
        commitUnknown();
      },
    ).finally(() => {
      if (refreshFlight === flight) refreshFlight = null;
    });
    refreshFlight = flight;
    return flight;
  };

  return {
    refresh,
    invalidate,
    invalidateAtOrAfterRouteStatusRevision,
    setRouteStatusEventFenceAvailable,
    dispose: () => {
      disposed = true;
      generation += 1;
      refreshFlight = null;
    },
  };
}
