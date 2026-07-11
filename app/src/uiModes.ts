export type WorkspaceTab = "setup" | "control" | "touch";
export type SetupSubTab = "library" | "profiles" | "patch" | "mapping" | "output";
export type ControlMode = "edit" | "live" | "mixer";
export type ControlCategory = "dimmer" | "color" | "position" | "gobo" | "beam" | "focus" | "other" | "fader";

export const setupSubTabs: { id: SetupSubTab; label: string; description: string }[] = [
  { id: "library", label: "Library", description: "GDTF import and share lookup" },
  { id: "profiles", label: "Profiles", description: "Fixture profile authoring" },
  { id: "patch", label: "Patch", description: "DMX addressing and fixture assignment" },
  { id: "mapping", label: "Mapping", description: "2D fixture and projection surface mapping" },
  { id: "output", label: "Output", description: "DMX and video output setup" },
];

export const controlModes: { id: ControlMode; label: string; description: string }[] = [
  { id: "edit", label: "Live Edit", description: "Fixture attributes, visual controls, and presets" },
  { id: "live", label: "Timeline", description: "Cue playback, timeline, and blackout controls" },
  { id: "mixer", label: "Mixer", description: "Video layers, compositions, and outputs" },
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
  const digitMatch = code.match(/^(Digit|Numpad)([1-5])$/);
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
