import { createSignal } from "solid-js";

/**
 * In-app confirmation dialogs. The WebView2 `window.confirm` UI ("tauri.localhost
 * says...") is never used: every question goes through this service and is drawn
 * by `InAppDialogHost`, one host per window, so pop-out pane windows show their
 * own modal.
 *
 * Text is English source text. The host runs it through `translateUiText`, so a
 * caller supplies the English wording and the Japanese lives in uiLocalization.
 */
export type InAppDialogTone = "default" | "danger";

export interface InAppDialogCopy {
  /** The question or the headline. */
  title: string;
  /** The consequence, one paragraph per entry. */
  message: string | readonly string[];
}

export interface ConfirmDialogOptions extends InAppDialogCopy {
  confirmLabel?: string;
  cancelLabel?: string;
  /** `danger` puts the initial focus on Cancel and uses the hazard treatment. */
  tone?: InAppDialogTone;
}

export interface AlertDialogOptions extends InAppDialogCopy {
  okLabel?: string;
  tone?: InAppDialogTone;
}

export interface ActiveInAppDialog {
  readonly id: number;
  readonly kind: "confirm" | "alert";
  readonly title: string;
  readonly message: readonly string[];
  readonly confirmLabel: string;
  /** `null` for an alert, which has no way to decline. */
  readonly cancelLabel: string | null;
  readonly tone: InAppDialogTone;
  /** Settles the pending promise exactly once and closes the dialog. */
  readonly settle: (confirmed: boolean) => void;
}

const [activeDialog, setActiveDialog] = createSignal<ActiveInAppDialog | null>(null);
let nextDialogId = 0;
// Alerts wait for the dialog in front of them; a confirmation never queues
// (see confirmDialog) so a stale question cannot appear after the operator moved on.
let alertQueue: Promise<void> = Promise.resolve();

/** The dialog the host should draw, or null. Reactive. */
export const activeInAppDialog = activeDialog;
/** True while a dialog is open in this window. Reactive. */
export const isInAppDialogOpen = (): boolean => activeDialog() !== null;

const paragraphs = (message: string | readonly string[]): readonly string[] =>
  typeof message === "string" ? [message] : message;

const present = (
  kind: ActiveInAppDialog["kind"],
  copy: InAppDialogCopy,
  labels: { confirm: string; cancel: string | null },
  tone: InAppDialogTone,
): Promise<boolean> => new Promise<boolean>((resolve) => {
  const id = ++nextDialogId;
  setActiveDialog({
    id,
    kind,
    title: copy.title,
    message: paragraphs(copy.message),
    confirmLabel: labels.confirm,
    cancelLabel: labels.cancel,
    tone,
    settle: (confirmed) => {
      // A dialog settles once; a late second click must not touch its successor.
      if (activeDialog()?.id !== id) return;
      setActiveDialog(null);
      resolve(confirmed);
    },
  });
});

/**
 * Ask the operator a yes/no question. Resolves true only for an explicit confirm.
 * Escape, Cancel, a backdrop dismissal and "another dialog is already open" all
 * resolve false, so a caller that treats false as "stop before any work" can
 * never act on an unanswered or duplicated question.
 */
export function confirmDialog(options: ConfirmDialogOptions): Promise<boolean> {
  if (activeDialog() !== null) return Promise.resolve(false);
  return present(
    "confirm",
    options,
    { confirm: options.confirmLabel ?? "OK", cancel: options.cancelLabel ?? "Cancel" },
    options.tone ?? "default",
  );
}

/** Tell the operator something that needs acknowledging. Waits for any open dialog. */
export function alertDialog(options: AlertDialogOptions): Promise<void> {
  const shown = alertQueue.then(async () => {
    while (activeDialog() !== null) await new Promise<void>((resolve) => setTimeout(resolve, 50));
    await present("alert", options, { confirm: options.okLabel ?? "OK", cancel: null }, options.tone ?? "default");
  });
  alertQueue = shown.catch(() => undefined);
  return shown;
}
