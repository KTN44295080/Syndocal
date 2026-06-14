// localStorage-backed color and position favorites, extracted from App.tsx.
// Hardened WebViews may block localStorage; every accessor degrades to in-memory defaults.
import type { PositionFavorite } from "./components/PositionControlPanel";
import { clampDmxValue, defaultColorFavorites, normalizeHexColor } from "./numericHelpers";

const colorFavoritesStorageKey = "rayard.colorFavorites.v1";
const positionFavoritesStorageKey = "rayard.positionFavorites.v1";

export const loadColorFavorites = () => {
  if (typeof window === "undefined") {
    return defaultColorFavorites();
  }
  try {
    const raw = window.localStorage.getItem(colorFavoritesStorageKey);
    if (!raw) {
      return defaultColorFavorites();
    }
    const parsed = JSON.parse(raw);
    if (!Array.isArray(parsed)) {
      return defaultColorFavorites();
    }
    const unique = new Set<string>();
    for (const candidate of parsed) {
      const color = normalizeHexColor(candidate);
      if (color) {
        unique.add(color);
      }
    }
    const favorites = [...unique].slice(0, 12);
    return favorites.length > 0 ? favorites : defaultColorFavorites();
  } catch {
    return defaultColorFavorites();
  }
};

export const saveColorFavorites = (favorites: string[]) => {
  if (typeof window === "undefined") {
    return;
  }
  try {
    const colors = favorites
      .map(normalizeHexColor)
      .filter((color): color is string => Boolean(color))
      .slice(0, 12);
    window.localStorage.setItem(colorFavoritesStorageKey, JSON.stringify(colors));
  } catch {
    // Local storage can be unavailable in hardened WebViews; the live palette still works in memory.
  }
};

export const defaultPositionFavorites = (): PositionFavorite[] => [
  { id: "home", label: "Home", pan: 32768, tilt: 32768 },
  { id: "down", label: "Down", pan: 32768, tilt: 0 },
  { id: "up", label: "Up", pan: 32768, tilt: 65535 },
];

export const positionFavoriteFromUnknown = (candidate: unknown): PositionFavorite | null => {
  if (!candidate || typeof candidate !== "object") {
    return null;
  }
  const source = candidate as Partial<PositionFavorite>;
  if (typeof source.id !== "string" || typeof source.label !== "string") {
    return null;
  }
  return {
    id: source.id.trim() || `position-${Date.now().toString(36)}`,
    label: source.label.trim().slice(0, 16) || "Position",
    pan: clampDmxValue(Number(source.pan)),
    tilt: clampDmxValue(Number(source.tilt)),
  };
};

export const loadPositionFavorites = () => {
  if (typeof window === "undefined") {
    return defaultPositionFavorites();
  }
  try {
    const raw = window.localStorage.getItem(positionFavoritesStorageKey);
    if (!raw) {
      return defaultPositionFavorites();
    }
    const parsed = JSON.parse(raw);
    if (!Array.isArray(parsed)) {
      return defaultPositionFavorites();
    }
    const favorites = parsed
      .map(positionFavoriteFromUnknown)
      .filter((favorite): favorite is PositionFavorite => Boolean(favorite))
      .slice(0, 24);
    return favorites.length > 0 ? favorites : defaultPositionFavorites();
  } catch {
    return defaultPositionFavorites();
  }
};

export const savePositionFavorites = (favorites: PositionFavorite[]) => {
  if (typeof window === "undefined") {
    return;
  }
  try {
    window.localStorage.setItem(positionFavoritesStorageKey, JSON.stringify(favorites.slice(0, 24)));
  } catch {
    // Local storage can be unavailable in hardened WebViews; the live palette still works in memory.
  }
};
