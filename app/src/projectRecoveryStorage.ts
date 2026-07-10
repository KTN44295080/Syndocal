import type { ProjectFile } from "./types";

const projectRecoveryStorageKey = "rayard.projectRecovery.v1";

export interface ProjectRecoveryCheckpoint {
  version: 1;
  app: "Rayard";
  saved_at: string;
  source_path: string | null;
  signature: string;
  project: ProjectFile;
}

const isProjectFile = (candidate: unknown): candidate is ProjectFile => {
  if (!candidate || typeof candidate !== "object") {
    return false;
  }
  const source = candidate as Partial<ProjectFile>;
  const snapshot = source.snapshot as Partial<ProjectFile["snapshot"]> | undefined;
  return (
    source.version === 1 &&
    source.app === "Rayard" &&
    Boolean(snapshot) &&
    Array.isArray(snapshot?.fixtures) &&
    Array.isArray(snapshot?.cues)
  );
};

export const projectRecoverySourceLabel = (checkpoint: ProjectRecoveryCheckpoint) => {
  if (!checkpoint.source_path) {
    return "Untitled.ry";
  }
  const normalized = checkpoint.source_path.replaceAll("\\", "/");
  return normalized.split("/").pop()?.trim() || checkpoint.source_path;
};

export const projectRecoveryTimeLabel = (checkpoint: ProjectRecoveryCheckpoint) => {
  const date = new Date(checkpoint.saved_at);
  if (Number.isNaN(date.getTime())) {
    return "Recovery";
  }
  return date.toLocaleString(undefined, {
    month: "short",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  });
};

export const createProjectRecoveryCheckpoint = (
  project: ProjectFile,
  sourcePath: string | null,
  signature: string,
): ProjectRecoveryCheckpoint => ({
  version: 1,
  app: "Rayard",
  saved_at: new Date().toISOString(),
  source_path: sourcePath,
  signature,
  project,
});

export const recoveryCheckpointFromUnknown = (candidate: unknown): ProjectRecoveryCheckpoint | null => {
  if (!candidate || typeof candidate !== "object") {
    return null;
  }
  const source = candidate as Partial<ProjectRecoveryCheckpoint>;
  if (
    source.version !== 1 ||
    source.app !== "Rayard" ||
    typeof source.saved_at !== "string" ||
    typeof source.signature !== "string" ||
    !isProjectFile(source.project)
  ) {
    return null;
  }
  return {
    version: 1,
    app: "Rayard",
    saved_at: source.saved_at,
    source_path: typeof source.source_path === "string" && source.source_path.trim() ? source.source_path.trim() : null,
    signature: source.signature,
    project: source.project,
  };
};

export const loadProjectRecoveryCheckpoint = () => {
  if (typeof window === "undefined") {
    return null;
  }
  try {
    const raw = window.localStorage.getItem(projectRecoveryStorageKey);
    return raw ? recoveryCheckpointFromUnknown(JSON.parse(raw)) : null;
  } catch {
    return null;
  }
};

export const saveProjectRecoveryCheckpoint = (checkpoint: ProjectRecoveryCheckpoint) => {
  if (typeof window === "undefined") {
    return;
  }
  try {
    window.localStorage.setItem(projectRecoveryStorageKey, JSON.stringify(checkpoint));
  } catch {
    // localStorage can be unavailable or full; the explicit project-save path remains authoritative.
  }
};

export const clearProjectRecoveryCheckpoint = () => {
  if (typeof window === "undefined") {
    return;
  }
  try {
    window.localStorage.removeItem(projectRecoveryStorageKey);
  } catch {
    // Ignore storage cleanup failures.
  }
};
