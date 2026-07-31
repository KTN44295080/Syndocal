// Setup-mapping stage-tool and saved view-preset model + localStorage helpers,
// extracted from App.tsx. Storage degrades to in-memory defaults in hardened WebViews.
import { clampRange, finiteOr } from "./numericHelpers";
import { stageViewBoxSize } from "./stageGeometry";

export type MappingStageTool = "select" | "place" | "rotate" | "pan";
const mappingStageTools: MappingStageTool[] = ["select", "place", "rotate", "pan"];

export interface MappingViewPreset {
  id: string;
  label: string;
  viewportZoom: number;
  viewportCenterX: number;
  viewportCenterZ: number;
  snapEnabled: boolean;
  snapSize: number;
  showLabels: boolean;
  showBeams: boolean;
  showGeometry: boolean;
  showProjectors: boolean;
  showStageObjects: boolean;
  showLevels: boolean;
  stageTool: MappingStageTool;
}

const mappingViewPresetStorageKey = "syndocal.mappingViewPresets.v1";

export const normalizeMappingStageTool = (value: unknown): MappingStageTool =>
  typeof value === "string" && mappingStageTools.includes(value as MappingStageTool)
    ? (value as MappingStageTool)
    : "select";

export const defaultMappingViewPresets = (): MappingViewPreset[] => [
  {
    id: "overview",
    label: "Overview",
    viewportZoom: 1,
    viewportCenterX: stageViewBoxSize / 2,
    viewportCenterZ: stageViewBoxSize / 2,
    snapEnabled: false,
    snapSize: 0.5,
    showLabels: true,
    showBeams: true,
    showGeometry: false,
    showProjectors: true,
    showStageObjects: true,
    showLevels: false,
    stageTool: "select",
  },
  {
    id: "fixture-focus",
    label: "Fixture Focus",
    viewportZoom: 1.8,
    viewportCenterX: stageViewBoxSize / 2,
    viewportCenterZ: stageViewBoxSize / 2,
    snapEnabled: true,
    snapSize: 0.5,
    showLabels: true,
    showBeams: true,
    showGeometry: true,
    showProjectors: false,
    showStageObjects: true,
    showLevels: true,
    stageTool: "select",
  },
  {
    id: "projection",
    label: "Projection",
    viewportZoom: 1.4,
    viewportCenterX: stageViewBoxSize / 2,
    viewportCenterZ: stageViewBoxSize / 2,
    snapEnabled: false,
    snapSize: 1,
    showLabels: false,
    showBeams: false,
    showGeometry: false,
    showProjectors: true,
    showStageObjects: true,
    showLevels: false,
    stageTool: "select",
  },
];

export const mappingViewPresetFromUnknown = (candidate: unknown): MappingViewPreset | null => {
  if (!candidate || typeof candidate !== "object") {
    return null;
  }
  const source = candidate as Partial<MappingViewPreset>;
  const label = typeof source.label === "string" ? source.label.trim().slice(0, 28) : "";
  if (!label) {
    return null;
  }
  const idSource = typeof source.id === "string" ? source.id.trim() : "";
  return {
    id: idSource || `view-${label.toLowerCase().replace(/[^a-z0-9]+/g, "-")}`,
    label,
    viewportZoom: Math.max(1, finiteOr(Number(source.viewportZoom), 1)),
    viewportCenterX: clampRange(finiteOr(Number(source.viewportCenterX), stageViewBoxSize / 2), 0, stageViewBoxSize),
    viewportCenterZ: clampRange(finiteOr(Number(source.viewportCenterZ), stageViewBoxSize / 2), 0, stageViewBoxSize),
    snapEnabled: Boolean(source.snapEnabled),
    snapSize: clampRange(Math.abs(finiteOr(Number(source.snapSize), 0.5)), 0.05, 20),
    showLabels: source.showLabels !== false,
    showBeams: source.showBeams !== false,
    showGeometry: Boolean(source.showGeometry),
    showProjectors: source.showProjectors !== false,
    showStageObjects: source.showStageObjects !== false,
    showLevels: Boolean(source.showLevels),
    stageTool: normalizeMappingStageTool(source.stageTool),
  };
};

export const loadMappingViewPresets = () => {
  const defaultPresets = defaultMappingViewPresets();
  const defaultIds = new Set(defaultPresets.map((preset) => preset.id));
  if (typeof window === "undefined") {
    return defaultPresets;
  }
  try {
    const raw = window.localStorage.getItem(mappingViewPresetStorageKey);
    if (!raw) {
      return defaultPresets;
    }
    const parsed = JSON.parse(raw);
    if (!Array.isArray(parsed)) {
      return defaultPresets;
    }
    const customPresets = parsed
      .map(mappingViewPresetFromUnknown)
      .filter((preset): preset is MappingViewPreset => Boolean(preset))
      .filter((preset) => !defaultIds.has(preset.id))
      .slice(0, 18);
    return [...defaultPresets, ...customPresets].slice(0, 18);
  } catch {
    return defaultPresets;
  }
};

export const saveMappingViewPresets = (presets: MappingViewPreset[]) => {
  if (typeof window === "undefined") {
    return;
  }
  try {
    window.localStorage.setItem(mappingViewPresetStorageKey, JSON.stringify(presets.slice(0, 18)));
  } catch {
    // Local storage can be unavailable in hardened WebViews; view presets still work in memory.
  }
};
