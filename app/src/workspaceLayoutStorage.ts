import type {
  ControlCategory,
  ControlMode,
  EditDeskSurface,
  SetupSubTab,
  TimelineContextDrawer,
  TimelineDeskSurface,
  WorkspaceTab,
} from "./uiModes";

export type WorkspaceLayout = {
  workspace_tab: WorkspaceTab;
  setup_sub_tab: SetupSubTab;
  control_mode: ControlMode;
  timeline_desk_surface: TimelineDeskSurface;
  timeline_context_drawer: TimelineContextDrawer;
  edit_desk_surface: EditDeskSurface;
  control_category: ControlCategory;
  top_split_ratio: number;
  lower_split_ratio: number;
  selections_drawer_open: boolean;
};

export const defaultWorkspaceLayout: WorkspaceLayout = {
  workspace_tab: "setup",
  setup_sub_tab: "patch",
  control_mode: "edit",
  timeline_desk_surface: "show",
  timeline_context_drawer: "none",
  edit_desk_surface: "attributes",
  control_category: "position",
  top_split_ratio: 0.58,
  lower_split_ratio: 0.44,
  selections_drawer_open: false,
};

export const workspaceLayoutStorageKey = "syndocal.workspaceLayout.v1";

const allowedWorkspaceTabs: WorkspaceTab[] = ["setup", "control", "touch"];
const allowedSetupSubTabs: SetupSubTab[] = [
  "patch",
  "video",
  "io",
];
const allowedControlModes: ControlMode[] = ["edit", "live", "mixer"];
const allowedTimelineDeskSurfaces: TimelineDeskSurface[] = ["show", "automation", "playback"];
// Block properties are tied to the current selection, so reopening that drawer
// from a previous process would restore stale editing context. Only stable
// drawers participate in workspace persistence.
const allowedPersistedTimelineContextDrawers: TimelineContextDrawer[] = ["none", "cue"];
const allowedEditDeskSurfaces: EditDeskSurface[] = ["faders", "attributes"];
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

const ratioValue = (candidate: unknown, fallback: number): number =>
  typeof candidate === "number" && Number.isFinite(candidate)
    ? Math.min(0.85, Math.max(0.15, candidate))
    : fallback;

const normalizedSetupSubTab = (candidate: unknown): unknown => {
  if (typeof candidate !== "string") return candidate;
  // #51: the four I/O sub-tabs merged into one surface.
  if (["dmx", "midi", "osc", "remote"].includes(candidate)) return "io";
  // #63: Library and Profiles merged into the self-contained Patch surface.
  if (["library", "profiles"].includes(candidate)) return "patch";
  return candidate;
};

export const workspaceLayoutFromUnknown = (candidate: unknown): WorkspaceLayout => {
  if (!candidate || typeof candidate !== "object" || Array.isArray(candidate)) {
    return { ...defaultWorkspaceLayout };
  }
  const value = candidate as Record<string, unknown>;
  const legacyCueSurface = value.timeline_desk_surface === "cues";
  return {
    workspace_tab: enumValue(value.workspace_tab, allowedWorkspaceTabs, defaultWorkspaceLayout.workspace_tab),
    setup_sub_tab: enumValue(
      normalizedSetupSubTab(value.setup_sub_tab),
      allowedSetupSubTabs,
      defaultWorkspaceLayout.setup_sub_tab,
    ),
    control_mode: enumValue(value.control_mode, allowedControlModes, defaultWorkspaceLayout.control_mode),
    timeline_desk_surface: enumValue(
      legacyCueSurface ? "show" : value.timeline_desk_surface,
      allowedTimelineDeskSurfaces,
      defaultWorkspaceLayout.timeline_desk_surface,
    ),
    timeline_context_drawer: enumValue(
      legacyCueSurface ? "cue" : value.timeline_context_drawer,
      allowedPersistedTimelineContextDrawers,
      defaultWorkspaceLayout.timeline_context_drawer,
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
    top_split_ratio: ratioValue(value.top_split_ratio, defaultWorkspaceLayout.top_split_ratio),
    lower_split_ratio: ratioValue(value.lower_split_ratio, defaultWorkspaceLayout.lower_split_ratio),
    selections_drawer_open:
      typeof value.selections_drawer_open === "boolean"
        ? value.selections_drawer_open
        : defaultWorkspaceLayout.selections_drawer_open,
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
