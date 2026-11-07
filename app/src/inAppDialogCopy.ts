import type { ConfirmDialogOptions } from "./inAppDialog";

/**
 * Wording for the in-app confirmation dialogs. English source text only: each
 * string has a Japanese entry in uiLocalization.ts, applied by the dialog host.
 * Only irreversible data loss asks for confirmation. Every dialog names its consequence, and
 * every dangerous one is `tone: "danger"` so initial focus lands on Cancel.
 */

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

export const restoreRdmFactoryDefaultsDialog = (): ConfirmDialogOptions => ({
  title: "Restore the selected fixture's factory defaults?",
  message: "The fixture's saved configuration will be erased. This cannot be undone.",
  confirmLabel: "Restore factory defaults",
  tone: "danger",
});
