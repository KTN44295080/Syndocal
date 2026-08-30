import { getCurrentWindow } from "@tauri-apps/api/window";
import { Show, createSignal, onCleanup, onMount, type ParentComponent } from "solid-js";
import {
  DESKTOP_WINDOW_MODE_ATTRIBUTE,
  desktopWindowModeFromWindowState,
  desktopWindowShortcutAction,
  type DesktopWindowMode,
} from "../desktopWindowMode";
import { isEditableShortcutTarget } from "../hotkeyHelpers";
import "../desktopWindowMode.css";

const NOTICE_DURATION_MS = 4_000;
const RESIZE_SETTLE_MS = 120;
const DESKTOP_ESCAPE_SHORTCUT_EVENT = "desktop-window-forward-escape";
const DESKTOP_RESIZE_DIRECTIONS = [
  "North",
  "NorthEast",
  "East",
  "SouthEast",
  "South",
  "SouthWest",
  "West",
  "NorthWest",
] as const;

const isTauriRuntime = () =>
  typeof window !== "undefined" &&
  Boolean((window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__);

const noticeForMode = (mode: DesktopWindowMode) => {
  switch (mode) {
    case "fullscreen":
      return { label: "FULL SCREEN", shortcut: "ESC EXIT" };
    case "maximized":
      return { label: "MAXIMIZED", shortcut: "F11 FULL SCREEN" };
    case "windowed":
      return { label: "WINDOW", shortcut: "F11 FULL SCREEN" };
    case "error":
      return { label: "WINDOW MODE ERROR", shortcut: "RESTART APP" };
    default:
      return null;
  }
};

const DesktopWindowResizeZones = () => {
  const startResize = async (
    direction: (typeof DESKTOP_RESIZE_DIRECTIONS)[number],
    event: PointerEvent,
  ) => {
    if (event.button !== 0 || !isTauriRuntime()) return;
    const appWindow = getCurrentWindow();
    if (await appWindow.isFullscreen() || await appWindow.isMaximized()) return;
    event.preventDefault();
    await appWindow.startResizeDragging(direction);
  };

  return (
    <div class="desktopResizeZones" aria-hidden="true">
      {DESKTOP_RESIZE_DIRECTIONS.map((direction) => (
        <div
          class={`desktopResizeZone desktopResizeZone${direction}`}
          data-window-resize-direction={direction}
          onPointerDown={(event) => void startResize(direction, event)}
        />
      ))}
    </div>
  );
};

export const DesktopWindowModeController: ParentComponent = (props) => {
  const [mode, setMode] = createSignal<DesktopWindowMode>("unknown");
  const [noticeVisible, setNoticeVisible] = createSignal(false);
  let noticeTimer: number | undefined;
  let resizeTimer: number | undefined;
  let handleNativeEscape: ((event: Event) => void) | undefined;
  let handleKeyDown: ((event: KeyboardEvent) => void) | undefined;
  let handleResize: (() => void) | undefined;
  let forwardingNativeEscape = false;
  let transitionInFlight = false;
  let disposed = false;

  const showNotice = () => {
    window.clearTimeout(noticeTimer);
    setNoticeVisible(true);
    noticeTimer = mode() === "error"
      ? undefined
      : window.setTimeout(() => setNoticeVisible(false), NOTICE_DURATION_MS);
  };

  onMount(() => {
    const documentRoot = document.documentElement;
    const hadPreviousWindowMode = documentRoot.hasAttribute(DESKTOP_WINDOW_MODE_ATTRIBUTE);
    const previousWindowMode = documentRoot.getAttribute(DESKTOP_WINDOW_MODE_ATTRIBUTE);
    const updateMode = (nextMode: DesktopWindowMode) => {
      setMode(nextMode);
      documentRoot.setAttribute(DESKTOP_WINDOW_MODE_ATTRIBUTE, nextMode);
    };

    updateMode("unknown");

    onCleanup(() => {
      disposed = true;
      window.clearTimeout(noticeTimer);
      window.clearTimeout(resizeTimer);
      if (handleNativeEscape) {
        window.removeEventListener(DESKTOP_ESCAPE_SHORTCUT_EVENT, handleNativeEscape);
      }
      if (handleKeyDown) window.removeEventListener("keydown", handleKeyDown);
      if (handleResize) window.removeEventListener("resize", handleResize);
      if (hadPreviousWindowMode) {
        documentRoot.setAttribute(DESKTOP_WINDOW_MODE_ATTRIBUTE, previousWindowMode ?? "");
      } else {
        documentRoot.removeAttribute(DESKTOP_WINDOW_MODE_ATTRIBUTE);
      }
    });

    if (!isTauriRuntime()) {
      return;
    }

    const appWindow = getCurrentWindow();

    const readMode = async (): Promise<DesktopWindowMode> => {
      const fullscreen = await appWindow.isFullscreen();
      return desktopWindowModeFromWindowState(
        fullscreen,
        fullscreen ? false : await appWindow.isMaximized(),
      );
    };

    const syncMode = async (announceChange: boolean) => {
      try {
        const nextMode = await readMode();
        if (disposed) return;
        const changed = mode() !== nextMode;
        updateMode(nextMode);
        if (announceChange && changed) showNotice();
      } catch {
        if (disposed) return;
        updateMode("error");
        showNotice();
      }
    };

    const enterOperationalWindowMode = async () => {
      if (transitionInFlight) return;
      transitionInFlight = true;
      try {
        // Keep the native config windowed until WebView2 has created its
        // controller.  Starting already maximized can strand the controller
        // on mixed-DPI multi-monitor desktops; the mounted DOM is the safe
        // boundary for entering the operational maximized workspace.
        if (!(await appWindow.isFullscreen()) && !(await appWindow.isMaximized())) {
          await appWindow.maximize();
        }
        await syncMode(false);
      } catch {
        if (disposed) return;
        updateMode("error");
        showNotice();
      } finally {
        transitionInFlight = false;
      }
    };

    const setFullscreen = async (nextFullscreen?: boolean) => {
      if (transitionInFlight) return;
      transitionInFlight = true;
      try {
        const currentFullscreen = await appWindow.isFullscreen();
        const next = nextFullscreen ?? !currentFullscreen;
        if (currentFullscreen !== next) {
          await appWindow.setFullscreen(next);
          if (!next) await appWindow.maximize();
        }
        if (disposed) return;
        updateMode(await readMode());
        showNotice();
      } catch (error) {
        console.error("Unable to change desktop fullscreen mode", error);
        if (disposed) return;
        updateMode("error");
        showNotice();
      } finally {
        transitionInFlight = false;
      }
    };

    handleKeyDown = (event: KeyboardEvent) => {
      if (forwardingNativeEscape && event.code === "Escape") return;
      const action = desktopWindowShortcutAction(
        {
          code: event.code,
          repeat: event.repeat,
          altKey: event.altKey,
          ctrlKey: event.ctrlKey,
          metaKey: event.metaKey,
          shiftKey: event.shiftKey,
          isComposing: event.isComposing,
          defaultPrevented: event.defaultPrevented,
          editableTarget: isEditableShortcutTarget(event.target),
        },
        mode() === "fullscreen",
      );
      if (!action) return;

      event.preventDefault();
      void setFullscreen(action === "exitFullscreen" ? false : undefined);
    };

    handleResize = () => {
      window.clearTimeout(resizeTimer);
      resizeTimer = window.setTimeout(() => void syncMode(true), RESIZE_SETTLE_MS);
    };

    const closeOpenDialogForNativeEscape = () => {
      const openDialogs = document.querySelectorAll<HTMLDialogElement>("dialog[open]");
      const dialog = openDialogs.item(openDialogs.length - 1);
      if (!dialog) return false;
      const cancelEvent = new Event("cancel", { cancelable: true });
      if (dialog.dispatchEvent(cancelEvent)) dialog.close();
      return true;
    };

    handleNativeEscape = (event) => {
      const detail = (event as CustomEvent<{ consumed: boolean }>).detail;
      if (closeOpenDialogForNativeEscape()) {
        detail.consumed = true;
        return;
      }
      const target = document.activeElement ?? window;
      const keyEvent = new KeyboardEvent("keydown", {
        key: "Escape",
        code: "Escape",
        bubbles: true,
        cancelable: true,
      });
      forwardingNativeEscape = true;
      try {
        target.dispatchEvent(keyEvent);
      } finally {
        forwardingNativeEscape = false;
      }
      detail.consumed = keyEvent.defaultPrevented;
    };

    window.addEventListener(DESKTOP_ESCAPE_SHORTCUT_EVENT, handleNativeEscape);
    window.addEventListener("keydown", handleKeyDown);
    window.addEventListener("resize", handleResize, { passive: true });
    void enterOperationalWindowMode();
  });

  const notice = () => noticeForMode(mode());

  return (
    <>
      {props.children}
      <Show when={isTauriRuntime()}>
        <DesktopWindowResizeZones />
      </Show>
      <Show when={noticeVisible() && notice()} keyed>
        {(visibleNotice) => (
          <output
            class="desktopWindowModeNotice"
            data-mode={mode()}
            aria-live="polite"
            aria-atomic="true"
          >
            <strong>{visibleNotice.label}</strong>
            <span aria-hidden="true">{visibleNotice.shortcut}</span>
          </output>
        )}
      </Show>
    </>
  );
};
