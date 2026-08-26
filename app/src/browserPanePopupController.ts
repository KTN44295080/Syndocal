import type { PaneWindowKind } from "./workspaceProfiles";

export type PoppedPanesPersistenceResult =
  | { ok: true }
  | { ok: false; error: string };

export type BrowserPanePopupController = {
  open: (pane: PaneWindowKind) => Promise<boolean>;
  close: (pane: PaneWindowKind) => Promise<boolean>;
  isOpening: (pane: PaneWindowKind) => boolean;
  dispose: () => void;
};

export type BrowserPanePopupControllerOptions = {
  browserWindow: Window;
  browserLocation: Location;
  getPoppedPanes: () => PaneWindowKind[];
  setPoppedPanes: (panes: PaneWindowKind[]) => void;
  persistPoppedPanes: (panes: PaneWindowKind[]) => PoppedPanesPersistenceResult;
  setMessage: (text: string) => void;
};

type BrowserPanePopupRecord = {
  popup: Window;
  instanceId: string;
  target: string;
  closedPollId: number | null;
};

type BrowserPanePopupRetirement =
  | { kind: "closed" }
  | { kind: "already_closed" }
  | { kind: "missing" }
  | { kind: "failed"; error: string };

type BrowserPanePopupReadiness =
  | { ok: true }
  | { ok: false; reason: "closed" | "timeout" | "inspection" | "disposed"; error?: string };

const BROWSER_PANE_POPUP_READY_POLL_MS = 50;
const BROWSER_PANE_POPUP_READY_TIMEOUT_MS = 5_000;
const BROWSER_PANE_POPUP_CLOSED_POLL_MS = 250;

export function createBrowserPanePopupController({
  browserWindow,
  browserLocation,
  getPoppedPanes,
  setPoppedPanes,
  persistPoppedPanes,
  setMessage,
}: BrowserPanePopupControllerOptions): BrowserPanePopupController {
  let disposed = false;
  let browserPanePopupInstanceSequence = 0;
  const browserPanePopups = new Map<PaneWindowKind, BrowserPanePopupRecord>();
  const browserPanePopupOpenings = new Set<PaneWindowKind>();
  const browserPanePopupRetirements = new Set<PaneWindowKind>();
  const browserPanePopupPending = new Map<PaneWindowKind, BrowserPanePopupRecord>();
  const cancelReadinessWaiters = new Set<() => void>();

  const nextBrowserPanePopupInstanceId = (): string => {
    browserPanePopupInstanceSequence += 1;
    const random = globalThis.crypto?.randomUUID?.().replaceAll("-", "")
      ?? Math.random().toString(36).slice(2);
    return `browser-${Date.now().toString(36)}-${browserPanePopupInstanceSequence.toString(36)}-${random}`;
  };

  const clearBrowserPanePopupClosedPoll = (record: BrowserPanePopupRecord) => {
    if (record.closedPollId === null) return;
    browserWindow.clearInterval(record.closedPollId);
    record.closedPollId = null;
  };

  const detachBrowserPanePopupRecord = (pane: PaneWindowKind, record: BrowserPanePopupRecord) => {
    if (browserPanePopups.get(pane) === record) {
      clearBrowserPanePopupClosedPoll(record);
      browserPanePopups.delete(pane);
    }
    if (browserPanePopupPending.get(pane) === record) {
      browserPanePopupPending.delete(pane);
    }
  };

  const retireBrowserPanePopupRecord = (
    pane: PaneWindowKind,
    record: BrowserPanePopupRecord,
  ): BrowserPanePopupRetirement => {
    detachBrowserPanePopupRecord(pane, record);
    let closed: boolean;
    try {
      closed = record.popup.closed;
    } catch (error) {
      return { kind: "failed", error: `the browser popup close state could not be read: ${String(error)}` };
    }
    if (closed) return { kind: "already_closed" };
    browserPanePopupRetirements.add(pane);
    try {
      record.popup.close();
    } catch (error) {
      return { kind: "failed", error: String(error) };
    } finally {
      browserPanePopupRetirements.delete(pane);
    }
    try {
      if (!record.popup.closed) {
        return { kind: "failed", error: "the browser reported the popup still open after close" };
      }
    } catch (error) {
      return { kind: "failed", error: `the browser popup close state could not be read: ${String(error)}` };
    }
    return { kind: "closed" };
  };

  const retireBrowserPanePopup = (pane: PaneWindowKind): BrowserPanePopupRetirement => {
    const record = browserPanePopups.get(pane);
    return record ? retireBrowserPanePopupRecord(pane, record) : { kind: "missing" };
  };

  const reintegrateClosedBrowserPanePopup = (pane: PaneWindowKind, record: BrowserPanePopupRecord) => {
    if (disposed || browserPanePopupRetirements.has(pane) || browserPanePopups.get(pane) !== record) return;
    let closed: boolean;
    try {
      closed = record.popup.closed;
    } catch (error) {
      // An unreadable popup is not evidence of a close. In particular, a
      // reload/navigation must never be treated as a rejoin.
      setMessage(`Pane window ${pane} popup close state could not be inspected: ${String(error)}`);
      return;
    }
    if (!closed) return;
    clearBrowserPanePopupClosedPoll(record);
    browserPanePopups.delete(pane);
    const next = getPoppedPanes().filter((candidate) => candidate !== pane);
    const persisted = persistPoppedPanes(next);
    // A real browser close can never leave its only main-window copy hidden,
    // even when clearing the restore record fails.
    setPoppedPanes(next);
    setMessage(
      persisted.ok
        ? `Pane window ${pane} was closed in the browser and rejoined the main window.`
        : `Pane window ${pane} was closed in the browser and rejoined the main window, but clearing its browser window state failed: ${persisted.error}`,
    );
  };

  const trackBrowserPanePopup = (pane: PaneWindowKind, record: BrowserPanePopupRecord) => {
    browserPanePopupPending.delete(pane);
    browserPanePopups.set(pane, record);
    record.closedPollId = browserWindow.setInterval(
      () => reintegrateClosedBrowserPanePopup(pane, record),
      BROWSER_PANE_POPUP_CLOSED_POLL_MS,
    );
  };

  const waitForBrowserPanePopupReady = (
    pane: PaneWindowKind,
    instanceId: string,
    popup: Window,
  ): Promise<BrowserPanePopupReadiness> => new Promise((resolve) => {
    const deadline = Date.now() + BROWSER_PANE_POPUP_READY_TIMEOUT_MS;
    let timerId: number | null = null;
    let settled = false;
    const finish = (result: BrowserPanePopupReadiness) => {
      if (settled) return;
      settled = true;
      if (timerId !== null) browserWindow.clearTimeout(timerId);
      cancelReadinessWaiters.delete(cancel);
      resolve(result);
    };
    const cancel = () => finish({ ok: false, reason: "disposed" });
    cancelReadinessWaiters.add(cancel);
    const inspect = (): BrowserPanePopupReadiness | null => {
      try {
        if (popup.closed) return { ok: false, reason: "closed" };
        const location = new URL(popup.location.href, browserLocation.href);
        if (location.href === "about:blank") return null;
        if (
          location.origin !== browserLocation.origin ||
          location.searchParams.get("syndocalPaneWindow") !== pane ||
          location.searchParams.get("syndocalPaneWindowInstance") !== instanceId
        ) {
          return {
            ok: false,
            reason: "inspection",
            error: "the popup did not navigate to its exact same-origin pane and instance identity",
          };
        }
        if (popup.document.readyState === "loading") return null;
        const root = popup.document.querySelector(`.app[data-pane-window-mode="${pane}"]`);
        return root
          ? { ok: true }
          : {
              ok: false,
              reason: "inspection",
              error: "the popup document did not mount its exact pane root",
            };
      } catch (error) {
        return { ok: false, reason: "inspection", error: String(error) };
      }
    };
    const poll = () => {
      const result = inspect();
      if (result !== null) {
        finish(result);
        return;
      }
      if (Date.now() >= deadline) {
        finish({ ok: false, reason: "timeout" });
        return;
      }
      timerId = browserWindow.setTimeout(poll, BROWSER_PANE_POPUP_READY_POLL_MS);
    };
    poll();
  });

  const open = async (pane: PaneWindowKind): Promise<boolean> => {
    if (disposed) return false;
    if (browserPanePopupOpenings.has(pane)) {
      setMessage(
        `Pane window ${pane} is already opening in the browser; wait for its exact pane content to become ready.`,
      );
      return false;
    }
    // Set this before window.open and retain it through readiness, durable
    // state, main-content hiding, and handle tracking. A second direct call
    // must never mint an untracked sibling popup for the same pane.
    browserPanePopupOpenings.add(pane);
    let popupRecord: BrowserPanePopupRecord | null = null;
    try {
      const previousPopup = browserPanePopups.get(pane);
      if (previousPopup) {
        let previousClosed: boolean;
        try {
          previousClosed = previousPopup.popup.closed;
        } catch (error) {
          setMessage(`Pane window ${pane} popup close state could not be inspected: ${String(error)}`);
          return false;
        }
        if (!previousClosed) {
          setMessage(
            `Pane window ${pane} remains integrated because its previous browser popup is still open; close or rejoin that window before opening another.`,
          );
          return false;
        }
        detachBrowserPanePopupRecord(pane, previousPopup);
      }
      const instanceId = nextBrowserPanePopupInstanceId();
      const target = `syndocal-pane-${pane}-${instanceId}`;
      const params = new URLSearchParams(browserLocation.search);
      params.set("syndocalPaneWindow", pane);
      params.set("syndocalPaneWindowInstance", instanceId);
      params.delete("syndocalPoppedPanes");
      let popup: Window | null;
      try {
        popup = browserWindow.open(`${browserLocation.pathname}?${params}`, target);
      } catch (error) {
        setMessage(
          `Pane window ${pane} could not open: ${String(error)}. ${pane} remains integrated in the main window.`,
        );
        return false;
      }
      if (popup === null) {
        setMessage(
          `Pane window ${pane} was blocked by the browser. Allow pop-ups for this site, then try again; ${pane} remains integrated in the main window.`,
        );
        return false;
      }
      popupRecord = { popup, instanceId, target, closedPollId: null };
      browserPanePopupPending.set(pane, popupRecord);
      const readiness = await waitForBrowserPanePopupReady(pane, instanceId, popup);
      browserPanePopupPending.delete(pane);
      if (disposed) return false;
      if (!readiness.ok) {
        const retired = retireBrowserPanePopupRecord(pane, popupRecord);
        const closeDetail = retired.kind === "failed"
          ? ` Closing the just-opened pane window also failed: ${retired.error}.`
          : "";
        const readinessDetail = readiness.reason === "closed"
          ? "closed before its exact pane content became ready"
          : readiness.reason === "timeout"
            ? `did not become ready within ${BROWSER_PANE_POPUP_READY_TIMEOUT_MS} ms`
            : `could not be inspected safely: ${readiness.error ?? "unknown browser inspection error"}`;
        setMessage(
          `Pane window ${pane} remains integrated because its browser popup ${readinessDetail}.${closeDetail}`,
        );
        return false;
      }
      const current = getPoppedPanes();
      const next = current.includes(pane) ? current : [...current, pane];
      const persisted = persistPoppedPanes(next);
      if (!persisted.ok) {
        const retired = retireBrowserPanePopupRecord(pane, popupRecord);
        const popupCloseError = retired.kind === "failed"
          ? ` Closing the just-opened pane window also failed: ${retired.error}.`
          : retired.kind === "missing"
            ? " The just-opened pane window handle was missing during retirement."
            : "";
        setMessage(
          `Pane window ${pane} remains integrated because saving its browser window state failed: ${persisted.error}.${popupCloseError}`,
        );
        return false;
      }
      setPoppedPanes(next);
      trackBrowserPanePopup(pane, popupRecord);
      return true;
    } finally {
      if (popupRecord && browserPanePopupPending.get(pane) === popupRecord) {
        browserPanePopupPending.delete(pane);
        if (disposed) {
          try {
            if (!popupRecord.popup.closed) popupRecord.popup.close();
          } catch {
            // Disposal cannot surface a useful status after the app unmounts.
          }
        }
      }
      browserPanePopupOpenings.delete(pane);
    }
  };

  const close = async (pane: PaneWindowKind): Promise<boolean> => {
    if (disposed) return false;
    if (browserPanePopupOpenings.has(pane)) {
      setMessage(
        `Pane window ${pane} is already opening in the browser; wait for its exact pane content to become ready.`,
      );
      return false;
    }
    const retired = retireBrowserPanePopup(pane);
    const next = getPoppedPanes().filter((candidate) => candidate !== pane);
    const persisted = persistPoppedPanes(next);
    // Rejoining must never leave the only main-window copy hidden merely
    // because browser storage is unavailable.
    setPoppedPanes(next);
    const popupRetirementMessage = retired.kind === "failed"
      ? ` The browser popup could not close: ${retired.error}.`
      : retired.kind === "missing"
        ? " The browser popup handle was missing."
        : retired.kind === "already_closed"
          ? " The browser popup had already closed."
          : "";
    if (!persisted.ok || retired.kind === "failed" || retired.kind === "missing") {
      setMessage(
        !persisted.ok
          ? `Pane window ${pane} rejoined the main window, but clearing its browser window state failed: ${persisted.error}.${popupRetirementMessage}`
          : `Pane window ${pane} rejoined the main window.${popupRetirementMessage}`,
      );
      return false;
    }
    if (popupRetirementMessage) setMessage(`Pane window ${pane} rejoined the main window.${popupRetirementMessage}`);
    return true;
  };

  const isOpening = (pane: PaneWindowKind) => browserPanePopupOpenings.has(pane);

  const dispose = () => {
    if (disposed) return;
    disposed = true;
    for (const cancel of cancelReadinessWaiters) cancel();
    for (const record of browserPanePopups.values()) {
      clearBrowserPanePopupClosedPoll(record);
      try {
        if (!record.popup.closed) record.popup.close();
      } catch {
        // The parent is unmounting; there is no safe status surface left.
      }
    }
    for (const record of browserPanePopupPending.values()) {
      try {
        if (!record.popup.closed) record.popup.close();
      } catch {
        // The parent is unmounting; there is no safe status surface left.
      }
    }
    browserPanePopups.clear();
    browserPanePopupPending.clear();
    browserPanePopupOpenings.clear();
    browserPanePopupRetirements.clear();
  };

  return { open, close, isOpening, dispose };
}
