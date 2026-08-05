export type WorkspaceTab = "setup" | "control" | "touch";
export type SetupArea = "lighting" | "video" | "mapping" | "io";
export type SetupSubTab = "patch" | "video" | "mapping" | "io";
export type ControlMode = "edit" | "live" | "mixer";
export type ControlCategory = "dimmer" | "color" | "position" | "gobo" | "beam" | "focus" | "other" | "fader";
export type TimelineDeskSurface = "show" | "automation" | "playback";
export type TimelineContextDrawer = "none" | "cue" | "block";
export type EditDeskSurface = "faders" | "attributes";

export const setupAreas: { id: SetupArea; label: string; description: string; defaultTab: SetupSubTab }[] = [
  { id: "lighting", label: "Lighting", description: "Fixture library, profiles, and DMX patch", defaultTab: "patch" },
  { id: "video", label: "Video", description: "Compositions, displays, and projector calibration", defaultTab: "video" },
  { id: "io", label: "I/O", description: "DMX, MIDI, OSC, and remote connections", defaultTab: "io" },
];

export const setupSubTabs: { id: SetupSubTab; area: SetupArea; label: string; description: string }[] = [
  { id: "patch", area: "lighting", label: "Patch", description: "Fixture profiles, DMX addressing, and assignment" },
  { id: "video", area: "video", label: "Outputs", description: "Compositions, output routing, resolution, and projection calibration" },
  { id: "io", area: "io", label: "I/O", description: "DMX, MIDI, OSC, and remote connections" },
];

export const setupAreaForSubTab = (tab: SetupSubTab): SetupArea =>
  setupSubTabs.find((candidate) => candidate.id === tab)?.area ?? "lighting";

export const setupSubTabsForArea = (area: SetupArea) => setupSubTabs.filter((tab) => tab.area === area);

export const controlModes: { id: ControlMode; label: string; description: string }[] = [
  { id: "edit", label: "Live Edit", description: "Fixture attributes, visual controls, and presets" },
  { id: "live", label: "Timeline", description: "Cue playback, timeline, and blackout controls" },
  { id: "mixer", label: "VJ Desk", description: "Clip launch, preview/program mixing, layers, and outputs" },
];

export const controlCategories: { id: ControlCategory; label: string }[] = [
  { id: "dimmer", label: "Dimmer" },
  { id: "color", label: "Color" },
  { id: "position", label: "Position" },
  { id: "gobo", label: "Gobo" },
  { id: "beam", label: "Beam" },
  { id: "focus", label: "Focus" },
  { id: "other", label: "Other" },
  { id: "fader", label: "Fader" },
];

export const controlModeForShortcut = (code: string): ControlMode | null => {
  switch (code) {
    case "KeyE":
      return "edit";
    case "KeyL":
      return "live";
    case "KeyM":
      return "mixer";
    default:
      return null;
  }
};

export const workspaceTabForShortcut = (code: string): WorkspaceTab | null => {
  switch (code) {
    case "F1":
      return "setup";
    case "F2":
      return "control";
    case "F3":
      return "touch";
    default:
      return null;
  }
};

export const setupSubTabForShortcut = (code: string): SetupSubTab | null => {
  const digitMatch = code.match(/^(Digit|Numpad)([1-9])$/);
  if (!digitMatch) {
    return null;
  }
  return setupSubTabs[Number(digitMatch[2]) - 1]?.id ?? null;
};

export const controlCategoryForAttribute = (attribute: string): Exclude<ControlCategory, "fader"> => {
  const normalized = attribute.toLowerCase().replace(/[^a-z0-9]/g, "");
  if (/(dimmer|intensity)/.test(normalized)) {
    return "dimmer";
  }
  if (/(pan|tilt|position|move|movement)/.test(normalized)) {
    return "position";
  }
  if (
    /(color|colour|red|green|blue|cyan|magenta|yellow|amber|white|warmwhite|coldwhite|uv|hue|saturation|cto|ctc|ctb)/.test(
      normalized,
    )
  ) {
    return "color";
  }
  if (/(gobo|animationwheel)/.test(normalized)) {
    return "gobo";
  }
  if (/(focus|focal)/.test(normalized)) {
    return "focus";
  }
  if (/(zoom|iris|prism|frost|beam|shutter|strobe|blade|framing|wash|spot)/.test(normalized)) {
    return "beam";
  }
  return "other";
};
