import { createUniqueId, For, onCleanup, onMount, Show } from "solid-js";
import { activeInAppDialog, type ActiveInAppDialog } from "../inAppDialog";
import { translateUiText, type UiLocale } from "../uiLocalization";
import "./InAppDialogHost.css";

const focusableSelector = "button:not(:disabled), [href], input:not(:disabled), select:not(:disabled), textarea:not(:disabled), [tabindex]:not([tabindex='-1'])";

const eyebrow = (dialog: ActiveInAppDialog) =>
  dialog.kind === "alert" ? "NOTICE" : dialog.tone === "danger" ? "CAUTION" : "CONFIRM";

function InAppDialogView(props: { dialog: ActiveInAppDialog; locale: UiLocale }) {
  const id = createUniqueId();
  const titleId = `in-app-dialog-title-${id}`;
  const detailId = `in-app-dialog-detail-${id}`;
  // The control that had focus when the question was asked gets it back afterwards.
  const opener = typeof document !== "undefined" && document.activeElement instanceof HTMLElement
    ? document.activeElement : null;
  let element!: HTMLDialogElement;
  const text = (value: string) => translateUiText(value, props.locale);
  const isAlert = () => props.dialog.kind === "alert";
  const answer = (confirmed: boolean) => props.dialog.settle(confirmed);

  onMount(() => {
    if (!element.open) element.showModal();
    // A dangerous question starts on Cancel, so a stray Enter or Space can never confirm it.
    const initial = props.dialog.tone === "danger" && !isAlert()
      ? element.querySelector<HTMLElement>("[data-in-app-dialog-cancel]")
      : element.querySelector<HTMLElement>("[data-in-app-dialog-confirm]");
    initial?.focus();
  });
  onCleanup(() => {
    // Leave the modal state first: the page behind a modal dialog is inert and
    // cannot take focus back.
    if (element?.open) element.close();
    if (opener?.isConnected) opener.focus({ preventScroll: true });
  });

  const onKeyDown = (event: KeyboardEvent) => {
    // The dialog owns every key while it is open; nothing reaches global or Perform shortcuts.
    event.stopPropagation();
    if (event.key === "Tab") {
      const focusable = [...element.querySelectorAll<HTMLElement>(focusableSelector)];
      if (focusable.length === 0) { event.preventDefault(); return; }
      const first = focusable[0];
      const last = focusable[focusable.length - 1];
      const current = document.activeElement;
      if (event.shiftKey && (current === first || !element.contains(current))) { event.preventDefault(); last.focus(); }
      else if (!event.shiftKey && (current === last || !element.contains(current))) { event.preventDefault(); first.focus(); }
      return;
    }
    // A held Enter/Space must not repeat-activate a button (a held key could confirm).
    if (event.repeat && (event.key === "Enter" || event.key === " ") && event.target instanceof HTMLButtonElement) {
      event.preventDefault();
    }
  };

  return (
    <dialog
      ref={element}
      class="inAppDialog"
      data-in-app-dialog
      data-no-localize
      data-in-app-dialog-kind={props.dialog.kind}
      data-tone={props.dialog.tone}
      role="alertdialog"
      aria-modal="true"
      aria-labelledby={titleId}
      aria-describedby={detailId}
      onKeyDown={onKeyDown}
      onCancel={(event) => {
        // Escape cancels (an alert simply acknowledges). The dialog is closed by the service.
        event.preventDefault();
        answer(false);
      }}
    >
      <section class="inAppDialogFrame">
        <Show when={props.dialog.tone === "danger"}>
          <div class="inAppDialogHazard" aria-hidden="true" />
        </Show>
        <header class="inAppDialogHeader">
          <span class="inAppDialogBadge">{text(eyebrow(props.dialog))}</span>
          <h2 id={titleId} class="inAppDialogTitle">{text(props.dialog.title)}</h2>
        </header>
        <div id={detailId} class="inAppDialogBody">
          <For each={props.dialog.message}>{(paragraph) => <p>{text(paragraph)}</p>}</For>
        </div>
        <footer class="inAppDialogActions">
          <Show when={!isAlert()}>
            <button type="button" class="inAppDialogCancel" data-in-app-dialog-cancel onClick={() => answer(false)}>
              {text(props.dialog.cancelLabel ?? "Cancel")}
            </button>
          </Show>
          <button
            type="button"
            class="inAppDialogConfirm"
            classList={{ danger: props.dialog.tone === "danger" }}
            data-in-app-dialog-confirm
            onClick={() => answer(true)}
          >
            {text(props.dialog.confirmLabel)}
          </button>
        </footer>
      </section>
    </dialog>
  );
}

/** Draws the one in-app question the window is currently asking. Mount once per
 * window root (main window and every pop-out pane window). */
export function InAppDialogHost(props: { locale: () => UiLocale }) {
  return (
    <Show when={activeInAppDialog()} keyed>
      {(dialog) => <InAppDialogView dialog={dialog} locale={props.locale()} />}
    </Show>
  );
}
