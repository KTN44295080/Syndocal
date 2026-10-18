/** UI-only preference: never changes show state or video-output colour. */
export const uiThemeOptions = [
  { id: "tally", label: "Tally" },
  { id: "graphite", label: "Graphite" },
  { id: "ice", label: "Ice" },
  { id: "cobalt", label: "Cobalt" },
  { id: "forest", label: "Forest" },
  { id: "ember", label: "Ember" },
  { id: "neon", label: "Neon" },
  { id: "safe", label: "Safe" },
] as const;

export type UiTheme = (typeof uiThemeOptions)[number]["id"];
export const uiThemeStorageKey = "syndocal.uiTheme.v1";

export function uiThemeFromUnknown(value: unknown): UiTheme {
  return uiThemeOptions.find((theme) => theme.id === value)?.id ?? "tally";
}

export function loadUiTheme(): UiTheme {
  try {
    return uiThemeFromUnknown(window.localStorage.getItem(uiThemeStorageKey));
  } catch {
    return "tally";
  }
}

export function applyUiTheme(theme: UiTheme): void {
  document.documentElement.dataset.theme = theme;
}

export function saveUiTheme(theme: UiTheme): boolean {
  try {
    window.localStorage.setItem(uiThemeStorageKey, theme);
    return true;
  } catch {
    // Keep the current session usable when local storage is unavailable.
    return false;
  }
}
