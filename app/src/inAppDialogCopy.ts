import type { ConfirmDialogOptions } from "./inAppDialog";

/**
 * Wording for the in-app confirmation dialogs. English source text only: each
 * string has a Japanese entry in uiLocalization.ts, applied by the dialog host.
 * Every dialog names its consequence ("Program goes black immediately."), and
 * every dangerous one is `tone: "danger"` so initial focus lands on Cancel.
 */

const undoFromMenu = "You can undo this from the Project menu or with Ctrl/Cmd+Z.";
const undoFromHistory = "This can be undone from Project History.";

export const blackoutDialog = (enabled: boolean): ConfirmDialogOptions => enabled
  ? {
    title: "Blackout video output?",
    message: "Program goes black immediately.",
    confirmLabel: "Blackout",
    tone: "danger",
  }
  : {
    title: "Recover video from blackout?",
    message: "Program returns to the live picture immediately.",
    confirmLabel: "Recover video",
    tone: "danger",
  };

export const newProjectDuringShowDialog = (): ConfirmDialogOptions => ({
  title: "Create a new project and stop this show?",
  message: "The running show stops and the current project is replaced.",
  confirmLabel: "Create new project",
  tone: "danger",
});

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

export const removeFixturesDialog = (count: number, label: string): ConfirmDialogOptions => ({
  title: count === 1 ? `Remove fixture "${label}"?` : `Remove ${count} selected fixtures?`,
  message: undoFromMenu,
  confirmLabel: "Remove",
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

export const disarmDjLinkDialog = (): ConfirmDialogOptions => ({
  title: "Disarm DJ Link?",
  message: "The machine credential is removed and automatic start is disabled.",
  confirmLabel: "Disarm",
  tone: "danger",
});

export const rotateDjLinkTokenDialog = (): ConfirmDialogOptions => ({
  title: "Rotate the DJ Link token?",
  message: "The current token stops working. The new token is shown only once, so copy it right away.",
  confirmLabel: "Rotate token",
  tone: "danger",
});

export const returnDjControlDialog = (): ConfirmDialogOptions => ({
  title: "Return DJ control authority?",
  message: "The current Timeline keeps running while the Agent announces a fresh track candidate.",
  confirmLabel: "Return control",
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

export const removeTimelineDialog = (label: string): ConfirmDialogOptions => ({
  title: `Remove Timeline "${label}"?`,
  message: undoFromHistory,
  confirmLabel: "Remove",
  tone: "danger",
});

export const deleteTouchPageDialog = (label: string): ConfirmDialogOptions => ({
  title: `Delete Touch page "${label}"?`,
  message: "The page and every control on it are removed from the Touch surface.",
  confirmLabel: "Delete page",
  tone: "danger",
});

export const removeTouchControlDialog = (label: string): ConfirmDialogOptions => ({
  title: `Remove Touch control "${label}"?`,
  message: "The control is removed from this page.",
  confirmLabel: "Remove",
  tone: "danger",
});

export const removeDjLinkMappingDialog = (): ConfirmDialogOptions => ({
  title: "Remove this DJ Link mapping?",
  message: "The track it matches no longer starts its Timeline.",
  confirmLabel: "Remove",
  tone: "danger",
});

export const removeClipSlotDialog = (slotId: number | string): ConfirmDialogOptions => ({
  title: `Remove Clip Slot ${slotId}?`,
  message: undoFromHistory,
  confirmLabel: "Remove",
  tone: "danger",
});

export const removeFxPresetDialog = (label: string): ConfirmDialogOptions => ({
  title: `Remove FX preset "${label}"?`,
  message: "The preset is removed from the catalog. Layers that already use it keep their effects.",
  confirmLabel: "Remove",
  tone: "danger",
});

export const removeLayerGroupDialog = (label: string): ConfirmDialogOptions => ({
  title: `Remove layer group "${label}"?`,
  message: "The group, its FX and transitions that require it are removed. Its layers remain in the composition.",
  confirmLabel: "Remove",
  tone: "danger",
});

export const removeFxStageDialog = (label: string): ConfirmDialogOptions => ({
  title: `Remove FX "${label}"?`,
  message: "The effect and its settings are removed from the chain.",
  confirmLabel: "Remove",
  tone: "danger",
});

export const removeFxChainDialog = (scopeLabel: string): ConfirmDialogOptions => ({
  title: `Remove the ${scopeLabel} effect chain?`,
  message: "Every effect in the chain and its settings are removed.",
  confirmLabel: "Remove chain",
  tone: "danger",
});

export const removeTransitionBusDialog = (label: string): ConfirmDialogOptions => ({
  title: `Remove transition bus "${label}"?`,
  message: "The bus and its Bus FX chain are removed.",
  confirmLabel: "Remove",
  tone: "danger",
});

export const addEffectToLiveLayerDialog = (layerTarget: string, effectLabel: string): ConfirmDialogOptions => ({
  title: `Add effect to Layer ${layerTarget}?`,
  message: [effectLabel, "LIVE: this changes Program right away."],
  confirmLabel: "Add effect",
});
