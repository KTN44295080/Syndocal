import type { PaneWindowPlacement } from "./workspaceProfiles";

export const PANE_WINDOW_TERMINAL_EVENT = "syndocal://pane-window-terminal";

export type PaneWindowStatusReport = PaneWindowPlacement & {
  instance_id: string | null;
  pending_close_request_id: string | null;
};

export type PaneWindowOpenResult = {
  placement_applied: boolean;
  warning: string | null;
};

export type PaneWindowPendingCloseContext = {
  pane: string;
  instance_id: string;
  request_id: string;
};

export type PaneWindowTerminalKind = "destroyed" | "canceled";

export type PaneWindowTerminalEvent = {
  pane: unknown;
  instance_id: unknown;
  request_id: unknown;
  terminal: unknown;
};

const PANE_OPAQUE_ID_MAX_LENGTH = 128;

export const isValidPaneOpaqueId = (value: unknown): value is string =>
  typeof value === "string"
  && value.length > 0
  && value.length <= PANE_OPAQUE_ID_MAX_LENGTH
  && /^[A-Za-z0-9_-]+$/.test(value);

export const createOpaquePaneId = (): string => {
  const raw = typeof crypto !== "undefined" && typeof crypto.randomUUID === "function"
    ? crypto.randomUUID()
    : `${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 12)}-${Math.random().toString(36).slice(2, 8)}`;
  const bounded = raw.replace(/[^A-Za-z0-9_-]/g, "");
  return bounded.length > 0 ? bounded : `pane-id-${Date.now().toString(36)}`;
};

export const paneWindowTerminalEventFromUnknown = (
  value: unknown,
): { pane: string; instanceId: string | null; requestId: string | null; terminal: PaneWindowTerminalKind } | null => {
  if (typeof value !== "object" || value === null) return null;
  const candidate = value as Partial<Record<"pane" | "instance_id" | "request_id" | "terminal", unknown>>;
  if (typeof candidate.pane !== "string" || candidate.pane.length === 0) return null;
  if (candidate.terminal !== "destroyed" && candidate.terminal !== "canceled") return null;
  const instanceId = isValidPaneOpaqueId(candidate.instance_id) ? candidate.instance_id : null;
  const requestId = isValidPaneOpaqueId(candidate.request_id) ? candidate.request_id : null;
  return { pane: candidate.pane, instanceId, requestId, terminal: candidate.terminal };
};

export type PaneLifecyclePhase = "opening" | "open" | "closing";

export type PaneTransitionView = {
  phase: PaneLifecyclePhase;
  instanceId: string;
  requestId: string | null;
};

export type PaneCloseWaiterRejection =
  | { reason: "canceled" }
  | { reason: "aborted"; message: string };

type PaneTransition = {
  phase: PaneLifecyclePhase;
  instanceId: string;
  requestId: string | null;
};

type PaneWaiter = {
  pane: string;
  instanceId: string;
  requestId: string;
  resolveDestroyed: () => void;
  reject: (rejection: PaneCloseWaiterRejection) => void;
  settled: boolean;
};

export type PaneWindowLifecycleController = {
  beginOpen: (pane: string, requestedInstanceId?: string) => { pane: string; instanceId: string } | null;
  adoptOpen: (pane: string, instanceId: string) => boolean;
  confirmOpen: (pane: string, instanceId: string) => void;
  failOpen: (pane: string, instanceId: string) => void;
  armClose: (pane: string) => {
    pane: string;
    instanceId: string;
    requestId: string;
    terminalPromise: Promise<"destroyed">;
  } | null;
  settleArmInvokeFailure: (pane: string, requestId: string, message: string) => void;
  applyTerminal: (event: unknown) => boolean;
  restoreOpenAfterFailedReposition: (pane: string, instanceId: string) => boolean;
  tryBeginWorkspaceApply: () => boolean;
  endWorkspaceApply: () => void;
  isWorkspaceApplyActive: () => boolean;
  transitionOf: (pane: string) => PaneTransitionView | null;
  hasAnyTransition: () => boolean;
  view: () => Partial<Record<string, PaneTransitionView>>;
  subscribe: (listener: () => void) => () => void;
  dispose: (message: string) => void;
  isDisposed: () => boolean;
};

export const createPaneWindowLifecycleController = (): PaneWindowLifecycleController => {
  let disposed = false;
  let workspaceApplyActive = false;
  let notifyScheduled = false;
  const transitions = new Map<string, PaneTransition>();
  const waiters = new Map<string, PaneWaiter>();
  const listeners = new Set<() => void>();

  const notify = () => {
    if (notifyScheduled || listeners.size === 0) return;
    notifyScheduled = true;
    queueMicrotask(() => {
      notifyScheduled = false;
      for (const listener of [...listeners]) listener();
    });
  };

  const settleWaiter = (waiter: PaneWaiter, outcome: "destroyed" | PaneCloseWaiterRejection) => {
    if (waiter.settled) return;
    waiter.settled = true;
    if (outcome === "destroyed") waiter.resolveDestroyed();
    else waiter.reject(outcome);
  };

  return {
    beginOpen: (pane, requestedInstanceId) => {
      if (disposed) return null;
      const instanceId = isValidPaneOpaqueId(requestedInstanceId)
        ? requestedInstanceId
        : createOpaquePaneId();
      if (transitions.has(pane)) return null;
      transitions.set(pane, { phase: "opening", instanceId, requestId: null });
      notify();
      return { pane, instanceId };
    },
    adoptOpen: (pane, instanceId) => {
      if (disposed || !isValidPaneOpaqueId(instanceId)) return false;
      const transition = transitions.get(pane);
      if (transition) {
        return transition.phase === "open" && transition.instanceId === instanceId;
      }
      transitions.set(pane, { phase: "open", instanceId, requestId: null });
      notify();
      return true;
    },
    confirmOpen: (pane, instanceId) => {
      if (disposed) return;
      const transition = transitions.get(pane);
      if (!transition || transition.instanceId !== instanceId || transition.phase !== "opening") return;
      transition.phase = "open";
      notify();
    },
    failOpen: (pane, instanceId) => {
      if (disposed) return;
      const transition = transitions.get(pane);
      if (!transition || transition.instanceId !== instanceId) return;
      transitions.delete(pane);
      notify();
    },
    armClose: (pane) => {
      if (disposed) return null;
      const transition = transitions.get(pane);
      if (!transition || transition.phase !== "open") return null;
      const requestId = createOpaquePaneId();
      // The terminal waiter must exist before the native close invoke so a
      // synchronous missing-window terminal can never be lost.
      let resolveDestroyed!: () => void;
      let reject!: (rejection: PaneCloseWaiterRejection) => void;
      const terminalPromise = new Promise<"destroyed">((resolve, rejectPromise) => {
        resolveDestroyed = () => resolve("destroyed");
        reject = rejectPromise;
      });
      waiters.set(pane, {
        pane,
        instanceId: transition.instanceId,
        requestId,
        resolveDestroyed,
        reject,
        settled: false,
      });
      transition.requestId = requestId;
      transition.phase = "closing";
      notify();
      return { pane, instanceId: transition.instanceId, requestId, terminalPromise };
    },
    settleArmInvokeFailure: (pane, requestId, message) => {
      const waiter = waiters.get(pane);
      if (waiter && waiter.requestId === requestId) {
        waiters.delete(pane);
        settleWaiter(waiter, { reason: "aborted", message });
      }
      const transition = transitions.get(pane);
      if (transition && transition.requestId === requestId && transition.phase === "closing") {
        transition.requestId = null;
        transition.phase = "open";
      }
      notify();
    },
    applyTerminal: (event) => {
      if (disposed) return false;
      const terminal = paneWindowTerminalEventFromUnknown(event);
      if (!terminal) return false;
      const transition = transitions.get(terminal.pane);
      // Stale/duplicate/wrong-instance terminals never mutate newer state.
      if (!transition) return false;
      if (!terminal.instanceId || terminal.instanceId !== transition.instanceId) return false;
      if (terminal.terminal === "destroyed") {
        const waiter = waiters.get(terminal.pane);
        if (waiter && waiter.instanceId === terminal.instanceId) {
          waiters.delete(terminal.pane);
          settleWaiter(waiter, "destroyed");
        }
        transitions.delete(terminal.pane);
        notify();
        return true;
      }
      // A canceled terminal only matters for the exact armed close request.
      if (
        transition.phase !== "closing"
        || !transition.requestId
        || !terminal.requestId
        || terminal.requestId !== transition.requestId
      ) {
        return false;
      }
      const waiter = waiters.get(terminal.pane);
      if (waiter && waiter.requestId === terminal.requestId) {
        waiters.delete(terminal.pane);
        settleWaiter(waiter, { reason: "canceled" });
      }
      transition.requestId = null;
      transition.phase = "open";
      notify();
      return true;
    },
    restoreOpenAfterFailedReposition: (pane, instanceId) => {
      if (disposed) return false;
      const transition = transitions.get(pane);
      // Only the exact incarnation's transitional residue is restored; a
      // newer identity and an already stable open pane are left untouched.
      if (!transition || transition.instanceId !== instanceId || transition.phase === "open") {
        return false;
      }
      const staleRequestId = transition.requestId;
      transition.requestId = null;
      transition.phase = "open";
      // Any waiter armed for the exact restored request must still settle.
      if (staleRequestId) {
        const waiter = waiters.get(pane);
        if (waiter && waiter.requestId === staleRequestId) {
          waiters.delete(pane);
          settleWaiter(
            waiter,
            { reason: "aborted", message: "Pane window close wait returned to open state." },
          );
        }
      }
      notify();
      return true;
    },
    tryBeginWorkspaceApply: () => {
      if (disposed || workspaceApplyActive) return false;
      if ([...transitions.values()].some((transition) => transition.phase !== "open")) return false;
      workspaceApplyActive = true;
      notify();
      return true;
    },
    endWorkspaceApply: () => {
      workspaceApplyActive = false;
      notify();
    },
    isWorkspaceApplyActive: () => workspaceApplyActive,
    transitionOf: (pane) => {
      const transition = transitions.get(pane);
      return transition ? { ...transition } : null;
    },
    hasAnyTransition: () =>
      [...transitions.values()].some((transition) => transition.phase !== "open"),
    view: () => Object.fromEntries([...transitions].map(([pane, transition]) => [pane, { ...transition }])),
    subscribe: (listener) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    dispose: (message) => {
      if (disposed) return;
      disposed = true;
      for (const waiter of [...waiters.values()]) {
        settleWaiter(waiter, { reason: "aborted", message });
      }
      waiters.clear();
      transitions.clear();
      workspaceApplyActive = false;
      notify();
    },
    isDisposed: () => disposed,
  };
};

/** Injectable timer seam so probe pacing is deterministically testable. */
export type PendingCloseReconciliationProbeTimers = {
  setTimeout: (callback: () => void, ms: number) => unknown;
  clearTimeout: (handle: unknown) => void;
};

export type PendingCloseReconciliationProbeOptions = {
  /** True only while the exact armed close request is still unsettled. */
  stillWaiting: () => boolean;
  /**
   * One non-authoritative reconciliation probe (for example the main-only
   * placement capture whose backend sweep retries lost terminals). Failures
   * never settle, fail, or fake the pending close.
   */
  probe: () => Promise<void>;
  intervalMs: number;
  /** Defaults to the real global timers; tests inject a manual clock. */
  schedule?: PendingCloseReconciliationProbeTimers;
};

/**
 * Continuous bounded-rate reconciliation for a parent close that is
 * legitimately waiting on its correlated terminal. One sequential loop runs:
 * at most one scheduled timer and at most one in-flight probe exist at any
 * moment, and the next probe is scheduled only after the previous one settles,
 * so probes can never overlap. The loop persists at a fixed rate for as long
 * as the exact wait is pending; it is never an attempt cap on the wait, which
 * stays unbounded and terminal-authoritative. Probes are advisory only: they
 * never resolve success, and the returned canceller is the single cleanup path
 * (terminal arrival, cancellation, disposal, or teardown all funnel through
 * the caller's finally block). The canceller latches immediately: it clears
 * any scheduled timer and stops an already-in-flight probe from scheduling
 * another one.
 */
export const startPendingCloseReconciliationProbes = (
  options: PendingCloseReconciliationProbeOptions,
): (() => void) => {
  const timers: PendingCloseReconciliationProbeTimers = options.schedule ?? {
    setTimeout: (callback, ms) => setTimeout(callback, ms),
    clearTimeout: (handle) => clearTimeout(handle as ReturnType<typeof setTimeout>),
  };
  let canceled = false;
  let probeInFlight = false;
  let timer: unknown = null;
  const cancel = () => {
    canceled = true;
    if (timer !== null) {
      timers.clearTimeout(timer);
      timer = null;
    }
  };
  const tick = (): void => {
    // The fired handle is consumed immediately: between this point and the
    // post-settle reschedule there is deliberately no scheduled timer, so the
    // single-slot invariant stays truthful and a cancel arriving mid-probe
    // finds exactly nothing to clear.
    timer = null;
    if (canceled || !options.stillWaiting()) return;
    if (probeInFlight) return;
    probeInFlight = true;
    void Promise.resolve()
      .then(() => options.probe())
      .catch(() => undefined)
      .then(() => {
        probeInFlight = false;
        if (canceled || !options.stillWaiting()) return;
        timer = timers.setTimeout(tick, options.intervalMs);
      });
  };
  // Delayed start keeps the rate fixed end to end: the first probe waits one
  // full interval exactly like every subsequent one.
  timer = timers.setTimeout(tick, options.intervalMs);
  return cancel;
};
