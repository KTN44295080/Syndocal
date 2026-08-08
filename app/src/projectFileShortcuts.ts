type MaybePromise = void | Promise<unknown>;

export interface ProjectFileShortcutActions {
  saveProject: () => MaybePromise;
  saveProjectAs: () => MaybePromise;
  loadProject: () => MaybePromise;
}

export type ProjectFileShortcutEvent = Pick<
  KeyboardEvent,
  "repeat" | "altKey" | "ctrlKey" | "metaKey" | "shiftKey" | "code" | "preventDefault"
>;

export function dispatchProjectFileShortcut(
  event: ProjectFileShortcutEvent,
  actions: ProjectFileShortcutActions,
): boolean {
  if (event.repeat || event.altKey || (!event.ctrlKey && !event.metaKey)) return false;

  if (event.code === "KeyS") {
    event.preventDefault();
    void (event.shiftKey ? actions.saveProjectAs() : actions.saveProject());
    return true;
  }
  if (!event.shiftKey && event.code === "KeyO") {
    event.preventDefault();
    void actions.loadProject();
    return true;
  }
  return false;
}
