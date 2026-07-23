import { workspaceLayoutFromUnknown, type WorkspaceLayout } from "./workspaceLayoutStorage";

export const paneWindowKinds = ["stage", "timeline", "programmer", "setup", "live", "mixer", "touch"] as const;
export type PaneWindowKind = (typeof paneWindowKinds)[number];

export type PaneWindowPlacement = {
  pane: PaneWindowKind;
  x: number;
  y: number;
  width: number;
  height: number;
  maximized: boolean;
};

export type NamedWorkspaceProfile = {
  id: string;
  name: string;
  layout: WorkspaceLayout;
  pane_windows: PaneWindowPlacement[];
};

export const namedWorkspaceStorageKey = "syndocal.namedWorkspaces.v1";
const namedWorkspaceLimit = 16;

const isRecord = (value: unknown): value is Record<string, unknown> =>
  typeof value === "object" && value !== null && !Array.isArray(value);

export const isPaneWindowKind = (value: unknown): value is PaneWindowKind =>
  typeof value === "string" && paneWindowKinds.includes(value as PaneWindowKind);

const placementFromUnknown = (value: unknown): PaneWindowPlacement | null => {
  if (!isRecord(value) || !isPaneWindowKind(value.pane)) return null;
  const x = value.x;
  const y = value.y;
  const width = value.width;
  const height = value.height;
  if (
    typeof x !== "number" || !Number.isInteger(x) || Math.abs(x) > 100_000 ||
    typeof y !== "number" || !Number.isInteger(y) || Math.abs(y) > 100_000 ||
    typeof width !== "number" || !Number.isInteger(width) || width < 320 || width > 8_192 ||
    typeof height !== "number" || !Number.isInteger(height) || height < 240 || height > 8_192 ||
    typeof value.maximized !== "boolean"
  ) {
    return null;
  }
  return { pane: value.pane, x, y, width, height, maximized: value.maximized };
};

const profileFromUnknown = (value: unknown): NamedWorkspaceProfile | null => {
  if (!isRecord(value) || typeof value.id !== "string" || typeof value.name !== "string") return null;
  const id = value.id.trim();
  const name = value.name.trim();
  if (!/^[a-zA-Z0-9-]{8,80}$/.test(id) || name.length === 0 || name.length > 48) return null;
  const rawPlacements = Array.isArray(value.pane_windows) ? value.pane_windows : [];
  const placements = rawPlacements.map(placementFromUnknown).filter((item): item is PaneWindowPlacement => item !== null);
  if (placements.length !== rawPlacements.length || new Set(placements.map((item) => item.pane)).size !== placements.length) {
    return null;
  }
  return {
    id,
    name,
    layout: workspaceLayoutFromUnknown(value.layout),
    pane_windows: placements,
  };
};

export const namedWorkspacesFromUnknown = (value: unknown): NamedWorkspaceProfile[] => {
  if (!Array.isArray(value)) return [];
  const profiles: NamedWorkspaceProfile[] = [];
  const ids = new Set<string>();
  for (const candidate of value.slice(0, namedWorkspaceLimit)) {
    const profile = profileFromUnknown(candidate);
    if (!profile || ids.has(profile.id)) continue;
    ids.add(profile.id);
    profiles.push(profile);
  }
  return profiles;
};

export const loadNamedWorkspaces = (): NamedWorkspaceProfile[] => {
  if (typeof window === "undefined") return [];
  try {
    const raw = window.localStorage.getItem(namedWorkspaceStorageKey);
    return raw ? namedWorkspacesFromUnknown(JSON.parse(raw)) : [];
  } catch {
    return [];
  }
};

export const saveNamedWorkspaces = (profiles: NamedWorkspaceProfile[]) => {
  if (typeof window === "undefined") return false;
  try {
    window.localStorage.setItem(namedWorkspaceStorageKey, JSON.stringify(namedWorkspacesFromUnknown(profiles)));
    return true;
  } catch {
    return false;
  }
};

const newWorkspaceId = () =>
  typeof crypto !== "undefined" && typeof crypto.randomUUID === "function"
    ? crypto.randomUUID()
    : `workspace-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 10)}`;

export const upsertNamedWorkspace = (
  profiles: NamedWorkspaceProfile[],
  name: string,
  layout: WorkspaceLayout,
  paneWindows: PaneWindowPlacement[],
) => {
  const normalizedName = name.trim();
  if (normalizedName.length === 0 || normalizedName.length > 48) {
    throw new Error("Workspace name must contain 1 to 48 characters.");
  }
  const existing = profiles.find((profile) => profile.name.localeCompare(normalizedName, undefined, { sensitivity: "accent" }) === 0);
  const profile: NamedWorkspaceProfile = {
    id: existing?.id ?? newWorkspaceId(),
    name: normalizedName,
    layout: workspaceLayoutFromUnknown(layout),
    pane_windows: paneWindows.map((placement) => ({ ...placement })),
  };
  const next = existing
    ? profiles.map((candidate) => candidate.id === existing.id ? profile : candidate)
    : [...profiles, profile];
  if (next.length > namedWorkspaceLimit) throw new Error(`A maximum of ${namedWorkspaceLimit} named workspaces can be saved.`);
  return next;
};
