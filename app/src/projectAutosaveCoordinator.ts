/** Consecutive timer captures superseded by a newer project revision before the
 * operator is told recovery is lagging: about 30 s, as the timer runs every
 * 10 s. Quick retries of a superseded capture are not counted. */
export const SUPERSEDED_RECOVERY_REPORT_AFTER = 3;

/**
 * A capture the backend refused only because the project moved on (a live edit
 * landed between reading the revision and capturing it) or another capture held
 * the lock. Nothing failed: the next capture sees the newer revision.
 */
export const isSupersededRecoveryCapture = (error: unknown): boolean => {
  const text = String(error);
  return text.includes("Project changed before recovery capture")
    || text.includes("Project recovery capture is busy; retry");
};

/** Per-window scheduling only; durable publication identity remains in its journal. */
export const createProjectAutosaveCoordinator = (
  isMainWindow: boolean,
  now: () => number = () => performance.now(),
) => {
  let inFlight = false;
  let lastBackupAttemptAt: number | null = null;
  let lastError: string | null = null;
  let superseded = 0;

  return {
    beginRecovery(): boolean {
      if (inFlight) return false;
      inFlight = true;
      return true;
    },
    finishRecovery(): void {
      inFlight = false;
    },
    beginDesktopBackup(changed: boolean): boolean {
      if (!isMainWindow || !changed) return false;
      const attemptedAt = now();
      if (lastBackupAttemptAt !== null && attemptedAt - lastBackupAttemptAt < 60_000) return false;
      // Reserve the attempt before awaiting IPC, including attempts that fail.
      lastBackupAttemptAt = attemptedAt;
      return true;
    },
    shouldReportError(message: string): boolean {
      if (lastError === message) return false;
      lastError = message;
      return true;
    },
    /** Counts a superseded capture; true once enough have run in a row that
     * recovery is lagging and the operator should know. */
    recoverySuperseded(): boolean {
      superseded += 1;
      return superseded >= SUPERSEDED_RECOVERY_REPORT_AFTER;
    },
    recoverySucceeded(): void {
      lastError = null;
      superseded = 0;
    },
  };
};
