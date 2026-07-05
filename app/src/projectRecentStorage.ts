const recentProjectsStorageKey = "rayard.recentProjects.v1";
const recentProjectLimit = 8;

export const recentProjectFileName = (path: string) => {
  const normalized = path.replaceAll("\\", "/");
  return normalized.split("/").pop()?.trim() || path;
};

const normalizeRecentProjectPath = (path: unknown) => {
  if (typeof path !== "string") {
    return null;
  }
  const trimmed = path.trim();
  return trimmed.toLowerCase().endsWith(".ry") ? trimmed : null;
};

export const recentProjectPathsFromUnknown = (candidate: unknown) => {
  if (!Array.isArray(candidate)) {
    return [];
  }
  const seen = new Set<string>();
  const paths: string[] = [];
  for (const entry of candidate) {
    const path = normalizeRecentProjectPath(entry);
    if (!path) {
      continue;
    }
    const key = path.toLowerCase();
    if (seen.has(key)) {
      continue;
    }
    seen.add(key);
    paths.push(path);
    if (paths.length >= recentProjectLimit) {
      break;
    }
  }
  return paths;
};

export const loadRecentProjectPaths = () => {
  if (typeof window === "undefined") {
    return [];
  }
  try {
    const raw = window.localStorage.getItem(recentProjectsStorageKey);
    return raw ? recentProjectPathsFromUnknown(JSON.parse(raw)) : [];
  } catch {
    return [];
  }
};

export const saveRecentProjectPaths = (paths: string[]) => {
  if (typeof window === "undefined") {
    return;
  }
  try {
    window.localStorage.setItem(recentProjectsStorageKey, JSON.stringify(recentProjectPathsFromUnknown(paths)));
  } catch {
    // Hardened WebViews can block localStorage; the in-memory recent list still works for this session.
  }
};

export const touchRecentProjectPath = (paths: string[], path: string) => {
  const normalized = normalizeRecentProjectPath(path);
  if (!normalized) {
    return recentProjectPathsFromUnknown(paths);
  }
  return recentProjectPathsFromUnknown([normalized, ...paths]);
};
