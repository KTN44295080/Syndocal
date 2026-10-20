import type { ConfirmDialogOptions } from "./inAppDialog";

/**
 * Wording for the in-app confirmation dialogs. English source text only: each
 * string has a Japanese entry in uiLocalization.ts, applied by the dialog host.
 * Every dialog names its consequence ("Program goes black immediately."), and
 * every dangerous one is `tone: "danger"` so initial focus lands on Cancel.
 */

const undoFromMenu = "You can undo this from the Project menu or with Ctrl/Cmd+Z.";
const undoFromHistory = "This can be undone from Project History.";

export const discardTimelineEditsDialog = (): ConfirmDialogOptions => ({
  title: "Discard unsaved Timeline edits?",
  message: "Your unsaved changes to this Timeline are lost.",
  confirmLabel: "Discard Changes",
  cancelLabel: "Keep Editing",
  tone: "danger",
});

export const discardFxDraftDialog = (): ConfirmDialogOptions => ({
  title: "Discard unsaved changes to this FX?",
  message: "These FX edits are incomplete and cannot be saved. The last saved FX is kept.",
  confirmLabel: "Discard Changes",
  cancelLabel: "Keep Editing",
  tone: "danger",
});

export const installUpdateDialog = (version: string): ConfirmDialogOptions => ({
  title: `Install Syndocal ${version}?`,
  message: [
    "A verified project backup is created before the download.",
    "The signed installer may close and restart Syndocal.",
  ],
  confirmLabel: "Install",
});

export const abandonPublicationDialog = (description: string): ConfirmDialogOptions => ({
  title: `Abandon the unfinished ${description} request?`,
  message: [
    "The earlier request has not finished, so its result is unknown.",
    "Abandoning it cannot be undone. You can then start a new request.",
  ],
  confirmLabel: "Abandon request",
  tone: "danger",
});

export const deleteBackupDialog = (createdAt: string): ConfirmDialogOptions => ({
  title: `Delete the backup from ${createdAt}?`,
  message: "This backup is deleted permanently.",
  confirmLabel: "Delete backup",
  tone: "danger",
});

export const rotateDjLinkTokenDialog = (): ConfirmDialogOptions => ({
  title: "Rotate the DJ Link token?",
  message: "The current token stops working. The new token is shown only once, so copy it right away.",
  confirmLabel: "Rotate token",
  tone: "danger",
});

export const adoptMediaReplacementDialog = (assetId: number | string, replacementPath: string): ConfirmDialogOptions => ({
  title: `Adopt replacement content for media asset ${assetId}?`,
  message: [
    replacementPath,
    "This file's content differs from what the project recorded for this asset (the hash does not match).",
    "This changes the saved content identity and updates every reference to this asset.",
  ],
  confirmLabel: "Adopt replacement",
  tone: "danger",
});

export const revokeAllExternalAccessDialog = (): ConfirmDialogOptions => ({
  title: "Revoke all external access?",
  message: "Every external principal is revoked and all authority grants are closed.",
  confirmLabel: "Revoke all",
  tone: "danger",
});
