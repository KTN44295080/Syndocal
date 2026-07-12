import type {
  ControlCategory,
  ControlMode,
  EditDeskSurface,
  SetupSubTab,
  TimelineDeskSurface,
  WorkspaceTab,
} from "./uiModes";

export type WorkspaceLayout = {
  workspace_tab: WorkspaceTab;
  setup_sub_tab: SetupSubTab;
  control_mode: ControlMode;
  timeline_desk_surface: TimelineDeskSurface;
  edit_desk_surface: EditDeskSurface;
  control_category: ControlCategory;
};

export const defaultWorkspaceLayout: WorkspaceLayout = {
  workspace_tab: "setup",
  setup_sub_tab: "patch",
  control_mode: "edit",
  timeline_desk_surface: "show",
  edit_desk_surface: "attributes",
  control_category: "position",
};

export const workspaceLayoutStorageKey = "syndocal.workspaceLayout.v1";

const allowedWorkspaceTabs: WorkspaceTab[] = ["setup", "control", "touch"];
const allowedSetupSubTabs: SetupSubTab[] = [
  "library",
  "profiles",
  "patch",
  "video",
  "mapping",
  "dmx",
  "midi",
  "osc",
  "remote",
];
const allowedControlModes: ControlMode[] = ["edit", "live", "mixer"];
const allowedTimelineDeskSurfaces: TimelineDeskSurface[] = ["show", "cues", "automation", "playback"];
const allowedEditDeskSurfaces: EditDeskSurface[] = ["attributes", "effects", "dmx"];
const allowedControlCategories: ControlCategory[] = [
  "dimmer",
  "color",
  "position",
  "gobo",
  "beam",
  "focus",
  "other",
  "fader",
];

const enumValue = <T extends string>(candidate: unknown, allowed: readonly T[], fallback: T): T =>
  typeof candidate === "string" && allowed.includes(candidate as T) ? (candidate as T) : fallback;

export const workspaceLayoutFromUnknown = (candidate: unknown): WorkspaceLayout => {
  if (!candidate || typeof candidate !== "object" || Array.isArray(candidate)) {
    return { ...defaultWorkspaceLayout };
  }
  const value = candidate as Record<string, unknown>;
  return {
    workspace_tab: enumValue(value.workspace_tab, allowedWorkspaceTabs, defaultWorkspaceLayout.workspace_tab),
    setup_sub_tab: enumValue(value.setup_sub_tab, allowedSetupSubTabs, defaultWorkspaceLayout.setup_sub_tab),
    control_mode: enumValue(value.control_mode, allowedControlModes, defaultWorkspaceLayout.control_mode),
    timeline_desk_surface: enumValue(
      value.timeline_desk_surface,
      allowedTimelineDeskSurfaces,
      defaultWorkspaceLayout.timeline_desk_surface,
    ),
    edit_desk_surface: enumValue(
      value.edit_desk_surface,
      allowedEditDeskSurfaces,
      defaultWorkspaceLayout.edit_desk_surface,
    ),
    control_category: enumValue(
      value.control_category,
      allowedControlCategories,
      defaultWorkspaceLayout.control_category,
    ),
  };
};

export const loadWorkspaceLayout = (): WorkspaceLayout => {
  if (typeof window === "undefined") {
    return { ...defaultWorkspaceLayout };
  }
  try {
    const raw = window.localStorage.getItem(workspaceLayoutStorageKey);
    return raw ? workspaceLayoutFromUnknown(JSON.parse(raw)) : { ...defaultWorkspaceLayout };
  } catch {
    try {
      window.localStorage.removeItem(workspaceLayoutStorageKey);
    } catch {
      // A locked-down WebView can deny both reads and cleanup.
    }
    return { ...defaultWorkspaceLayout };
  }
};

export const saveWorkspaceLayout = (layout: WorkspaceLayout): boolean => {
  if (typeof window === "undefined") {
    return false;
  }
  try {
    window.localStorage.setItem(workspaceLayoutStorageKey, JSON.stringify(workspaceLayoutFromUnknown(layout)));
    return true;
  } catch {
    return false;
  }
};
