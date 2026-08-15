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

export type ProjectFileShortcutAction =
  | { kind: "saveProject" }
  | { kind: "saveProjectAs" }
  | { kind: "loadProject" };

const projectFileShortcutActionKindManifest = {
  saveProject: true,
  saveProjectAs: true,
  loadProject: true,
} as const satisfies Record<ProjectFileShortcutAction["kind"], true>;

export const PROJECT_FILE_SHORTCUT_ACTION_KINDS = Object.freeze(
  Object.keys(projectFileShortcutActionKindManifest) as ProjectFileShortcutAction["kind"][],
);

export function resolveProjectFileShortcut(event: ProjectFileShortcutEvent): ProjectFileShortcutAction | null {
  if (event.repeat || event.altKey || (!event.ctrlKey && !event.metaKey)) return null;
  if (event.code === "KeyS") return { kind: event.shiftKey ? "saveProjectAs" : "saveProject" };
  if (!event.shiftKey && event.code === "KeyO") return { kind: "loadProject" };
  return null;
}

export function dispatchProjectFileShortcut(
  event: ProjectFileShortcutEvent,
  actions: ProjectFileShortcutActions,
): boolean {
  const action = resolveProjectFileShortcut(event);
  if (!action) return false;
  event.preventDefault();
  switch (action.kind) {
    case "saveProject":
      void actions.saveProject();
      break;
    case "saveProjectAs":
      void actions.saveProjectAs();
      break;
    case "loadProject":
      void actions.loadProject();
      break;
    default: {
      const unreachable: never = action;
      return unreachable;
    }
  }
  return true;
}
