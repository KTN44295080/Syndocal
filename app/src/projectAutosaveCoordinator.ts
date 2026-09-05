/** Per-window scheduling only; durable publication identity remains in its journal. */
export const createProjectAutosaveCoordinator = (
  isMainWindow: boolean,
  now: () => number = () => performance.now(),
) => {
  let inFlight = false;
  let lastBackupAttemptAt: number | null = null;
  let lastError: string | null = null;

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
    recoverySucceeded(): void {
      lastError = null;
    },
  };
};
