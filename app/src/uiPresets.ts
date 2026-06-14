// Static UI preset data (color palette/quick-looks, pan-tilt nudge steps and target grid,
// mapping snap increments) extracted from App.tsx. Pure data, no type/state deps.

export const mappingSnapPresets = [0.25, 0.5, 1, 2];

export const defaultColorPalette = [
  "#ff0000",
  "#00ff00",
  "#0000ff",
  "#ffff00",
  "#ff00ff",
  "#00ffff",
  "#ffffff",
  "#ff7a00",
  "#7a2cff",
  "#1ee6a8",
  "#ff4fa3",
  "#b5ff2f",
];

export const colorQuickLooks = [
  { label: "White", color: "#ffffff" },
  { label: "Red", color: "#ff0000" },
  { label: "Green", color: "#00ff00" },
  { label: "Blue", color: "#0000ff" },
  { label: "Amber", color: "#ff7a00" },
  { label: "Cyan", color: "#00ffff" },
];

export const panTiltNudgeSteps = [
  { label: "Fine", value: 256 },
  { label: "Small", value: 1024 },
  { label: "Medium", value: 2048 },
  { label: "Coarse", value: 8192 },
];

export const panTiltTargetPoints = [
  { label: "TL", pan: 0.2, tilt: 0.8 },
  { label: "TC", pan: 0.5, tilt: 0.8 },
  { label: "TR", pan: 0.8, tilt: 0.8 },
  { label: "L", pan: 0.2, tilt: 0.5 },
  { label: "C", pan: 0.5, tilt: 0.5 },
  { label: "R", pan: 0.8, tilt: 0.5 },
  { label: "BL", pan: 0.2, tilt: 0.2 },
  { label: "BC", pan: 0.5, tilt: 0.2 },
  { label: "BR", pan: 0.8, tilt: 0.2 },
];
