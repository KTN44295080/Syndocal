import { getCurrentWindow } from "@tauri-apps/api/window";
import { Show, createSignal, onCleanup, onMount, type ParentComponent } from "solid-js";
import { desktopWindowShortcutAction } from "../desktopWindowMode";
import { isEditableShortcutTarget } from "../hotkeyHelpers";
import "../desktopWindowMode.css";

type DesktopWindowMode = "unknown" | "windowed" | "maximized" | "fullscreen" | "error";

const NOTICE_DURATION_MS = 4_000;
const RESIZE_SETTLE_MS = 120;
const DESKTOP_FULLSCREEN_SHORTCUT_EVENT = "desktop-window-toggle-fullscreen";
const DESKTOP_ESCAPE_SHORTCUT_EVENT = "desktop-window-forward-escape";

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
      return { label: "WINDOW MODE ERROR", shortcut: "F11 RETRY" };
    default:
      return null;
  }
};
export const DesktopWindowModeController: ParentComponent = (props) => {
  const [mode, setMode] = createSignal<DesktopWindowMode>("unknown");
  const [noticeVisible, setNoticeVisible] = createSignal(false);
  let noticeTimer: number | undefined;
  let resizeTimer: number | undefined;
  let unlistenNativeFullscreen: (() => void) | undefined;
  let unlistenNativeEscape: (() => void) | undefined;
  let transitionInFlight = false;
  let disposed = false;

  const showNotice = () => {
    window.clearTimeout(noticeTimer);
    setNoticeVisible(true);
    noticeTimer = window.setTimeout(() => setNoticeVisible(false), NOTICE_DURATION_MS);
  };

  onMount(() => {
    if (!isTauriRuntime()) {
      return;
    }

    const appWindow = getCurrentWindow();

    const readMode = async (): Promise<DesktopWindowMode> => {
      if (await appWindow.isFullscreen()) {
        return "fullscreen";
      }
      return (await appWindow.isMaximized()) ? "maximized" : "windowed";
    };

    const syncMode = async (announceChange: boolean) => {
      try {
        const nextMode = await readMode();
        if (disposed) return;
        const changed = mode() !== nextMode;
        setMode(nextMode);
        if (announceChange && changed) showNotice();
      } catch {
        if (disposed) return;
        setMode("error");
        showNotice();
      }
    };

    const enterOperationalWindowMode = async () => {
      try {
        if (!(await appWindow.isFullscreen()) && !(await appWindow.isMaximized())) {
          await appWindow.maximize();
        }
        await syncMode(false);
      } catch {
        if (disposed) return;
        setMode("error");
        showNotice();
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
        }
        if (disposed) return;
        setMode(await readMode());
        showNotice();
      } catch {
        if (disposed) return;
        setMode("error");
        showNotice();
      } finally {
        transitionInFlight = false;
      }
    };

    const handleKeyDown = (event: KeyboardEvent) => {
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

    const handleResize = () => {
      window.clearTimeout(resizeTimer);
      resizeTimer = window.setTimeout(() => void syncMode(true), RESIZE_SETTLE_MS);
    };

    const forwardNativeEscape = () => {
      const target = document.activeElement ?? window;
      target.dispatchEvent(
        new KeyboardEvent("keydown", {
          key: "Escape",
          code: "Escape",
          bubbles: true,
          cancelable: true,
        }),
      );
    };

    void appWindow
      .listen(DESKTOP_FULLSCREEN_SHORTCUT_EVENT, () => void setFullscreen())
      .then((unlisten) => {
        if (disposed) {
          unlisten();
          return;
        }
        unlistenNativeFullscreen = unlisten;
      })
      .catch(() => {
        if (disposed) return;
        setMode("error");
        showNotice();
      });
    void appWindow
      .listen(DESKTOP_ESCAPE_SHORTCUT_EVENT, forwardNativeEscape)
      .then((unlisten) => {
        if (disposed) {
          unlisten();
          return;
        }
        unlistenNativeEscape = unlisten;
      })
      .catch(() => {
        if (disposed) return;
        setMode("error");
        showNotice();
      });
    window.addEventListener("keydown", handleKeyDown);
    window.addEventListener("resize", handleResize, { passive: true });
    void enterOperationalWindowMode();

    onCleanup(() => {
      disposed = true;
      window.clearTimeout(noticeTimer);
      window.clearTimeout(resizeTimer);
      unlistenNativeFullscreen?.();
      unlistenNativeEscape?.();
      window.removeEventListener("keydown", handleKeyDown);
      window.removeEventListener("resize", handleResize);
    });
  });

  const notice = () => noticeForMode(mode());

  return (
    <>
      {props.children}
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
