export type DesktopWindowShortcutAction = "toggleFullscreen" | "exitFullscreen";

export interface DesktopWindowShortcutEvent {
  code: string;
  repeat: boolean;
  altKey: boolean;
  ctrlKey: boolean;
  metaKey: boolean;
  shiftKey: boolean;
  isComposing: boolean;
  defaultPrevented: boolean;
  editableTarget: boolean;
}

export const desktopWindowShortcutAction = (
  event: DesktopWindowShortcutEvent,
  fullscreen: boolean,
): DesktopWindowShortcutAction | null => {
  if (
    event.repeat ||
    event.isComposing ||
    event.altKey ||
    event.ctrlKey ||
    event.metaKey ||
    event.shiftKey
  ) {
    return null;
  }

  if (event.code === "F11") {
    return event.defaultPrevented ? null : "toggleFullscreen";
  }

  if (event.code !== "Escape" || !fullscreen) {
    return null;
  }

  // Let an editor keep Escape when it explicitly consumes the key. Unhandled
  // Escape still exits fullscreen without blurring or mutating the field.
  return event.editableTarget && event.defaultPrevented ? null : "exitFullscreen";
};
