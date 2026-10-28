/**
 * Main-window close confirmation policy.
 *
 * Owner policy (2026-10-28, under the 2026-10-20 "confirm only irreversible
 * loss" rule): closing Syndocal asks only when unsaved project or Timeline
 * work would be discarded. Playing layers or an armed output alone never ask;
 * the backend main-window close shutdown stops outputs before the process
 * exits. When unsaved work and live output coincide, the dialog still names
 * both consequences.
 */
export type MainProtectedCloseReason = "dirty-only" | "dirty-and-runtime" | "output-state-unknown";

export type MainProtectedCloseRequest = {
  pane: "main";
  reason: MainProtectedCloseReason;
  projectDirty: boolean;
  timelineDirty: boolean;
  /** `null` when the bounded close-time snapshot refresh failed. */
  runtimeActive: boolean | null;
};

export const mainCloseHasUnsavedWork = (projectDirty: boolean, timelineDirty: boolean): boolean =>
  projectDirty || timelineDirty;

/** `null` means the close proceeds without any dialog. */
export const mainProtectedCloseRequest = (
  projectDirty: boolean,
  timelineDirty: boolean,
  runtimeActive: boolean | null,
): MainProtectedCloseRequest | null => {
  if (!mainCloseHasUnsavedWork(projectDirty, timelineDirty)) return null;
  const reason: MainProtectedCloseReason = runtimeActive === null
    ? "output-state-unknown"
    : runtimeActive ? "dirty-and-runtime" : "dirty-only";
  return { pane: "main", reason, projectDirty, timelineDirty, runtimeActive };
};
