import type { Accessor, Setter } from "solid-js";
import type { MappingViewPreset } from "./mappingViewPresets";
import { stageObjectDefaultColor } from "./stageObjects";
import type { StageWorldBounds } from "./stageGeometry";
import type {
  EngineSnapshot,
  StageMapConfig,
  StageMapPresetSummary,
  StageObjectKind,
  StageObjectSummary,
} from "./types";

type Invoke = <T>(command: string, args?: Record<string, unknown>) => Promise<T>;

interface StageMapControllerOptions {
  invoke: Invoke;
  snapshot: Accessor<EngineSnapshot>;
  refreshSnapshot: () => Promise<EngineSnapshot | null>;
  setMessage: (message: string) => unknown;
  stageObjectLabel: Accessor<string>;
  stageObjectKind: Accessor<StageObjectKind>;
  stageObjectWidth: Accessor<number>;
  stageObjectDepth: Accessor<number>;
  stageObjectRotation: Accessor<number>;
  stageObjectColor: Accessor<string>;
  stageWorldBounds: Accessor<StageWorldBounds>;
  autoStageWorldBounds: Accessor<StageWorldBounds>;
  setSelectedStageObjectId: Setter<number | null>;
  selectedStageObjectId: Accessor<number | null>;
  setMappingShowStageObjects: Setter<boolean>;
  stageMapPresetLabel: Accessor<string>;
  setStageMapPresetLabel: Setter<string>;
  selectedStageMapPresetLabel: Accessor<string>;
  setSelectedStageMapPresetLabel: Setter<string>;
}

export function createStageMapController(options: StageMapControllerOptions) {
  const addStageObjectAtCenter = async () => {
    const kind = options.stageObjectKind();
    try {
      const bounds = options.stageWorldBounds();
      const objectId = await options.invoke<number>("add_stage_object", {
        label: options.stageObjectLabel(),
        kind,
        x: Number(((bounds.minX + bounds.maxX) / 2).toFixed(2)),
        z: Number(((bounds.minZ + bounds.maxZ) / 2).toFixed(2)),
        width: options.stageObjectWidth(),
        depth: options.stageObjectDepth(),
        rotationDeg: options.stageObjectRotation(),
        color: options.stageObjectColor() || stageObjectDefaultColor(kind),
      });
      options.setSelectedStageObjectId(objectId);
      options.setMappingShowStageObjects(true);
      await options.refreshSnapshot();
      options.setMessage(`Added stage object ${options.stageObjectLabel()}.`);
    } catch (error) { options.setMessage(String(error)); }
  };

  const setStageObject = async (object: StageObjectSummary, updates: Partial<StageObjectSummary>) => {
    try {
      await options.invoke("set_stage_object", { object: { ...object, ...updates } });
      await options.refreshSnapshot();
    } catch (error) { options.setMessage(String(error)); }
  };

  const removeStageObject = async (objectId: number) => {
    try {
      await options.invoke("remove_stage_object", { objectId });
      if (options.selectedStageObjectId() === objectId) options.setSelectedStageObjectId(null);
      await options.refreshSnapshot();
      options.setMessage(`Removed stage object ${objectId}.`);
    } catch (error) { options.setMessage(String(error)); }
  };

  const setStageMapConfig = async (
    updates: Partial<StageMapConfig>,
    successMessage = "Updated 2D stage map.",
  ) => {
    try {
      await options.invoke("set_stage_map_config", { config: { ...options.snapshot().stage_map, ...updates } });
      options.setMessage(successMessage);
      await options.refreshSnapshot();
    } catch (error) { options.setMessage(String(error)); }
  };

  const lockStageMapToCurrentBounds = async () => {
    const bounds = options.autoStageWorldBounds();
    await setStageMapConfig({
      locked: true,
      min_x: Number(bounds.minX.toFixed(2)),
      max_x: Number(bounds.maxX.toFixed(2)),
      min_z: Number(bounds.minZ.toFixed(2)),
      max_z: Number(bounds.maxZ.toFixed(2)),
    }, "Locked 2D stage map to current bounds.");
  };

  const stageMapPresetObjectCountLabel = (preset: StageMapPresetSummary) => {
    const count = preset.stage_objects?.length;
    return count === undefined ? "bounds only" : `${count} object${count === 1 ? "" : "s"}`;
  };
  const mappingViewPresetObjectLabel = (preset: MappingViewPreset) =>
    preset.showStageObjects ? "objects shown" : "objects hidden";

  const saveStageMapPreset = async () => {
    try {
      const stageObjectCount = options.snapshot().stage_objects.length;
      const label = await options.invoke<string>("save_stage_map_preset", {
        label: options.stageMapPresetLabel(),
        config: options.snapshot().stage_map,
      });
      options.setStageMapPresetLabel(label);
      options.setSelectedStageMapPresetLabel(label);
      options.setMessage(`Saved stage map preset ${label} with ${stageObjectCount} stage object${stageObjectCount === 1 ? "" : "s"}.`);
      await options.refreshSnapshot();
    } catch (error) { options.setMessage(String(error)); }
  };

  const applyStageMapPreset = async (label: string) => {
    try {
      const preset = options.snapshot().stage_map_presets.find((candidate) => candidate.label === label);
      const stageObjectLabel = preset ? stageMapPresetObjectCountLabel(preset) : "selected layout";
      await options.invoke("apply_stage_map_preset", { label });
      options.setSelectedStageMapPresetLabel(label);
      options.setMessage(`Applied stage map preset ${label} (${stageObjectLabel}).`);
      await options.refreshSnapshot();
    } catch (error) { options.setMessage(String(error)); }
  };

  const removeStageMapPreset = async (label: string) => {
    try {
      await options.invoke("remove_stage_map_preset", { label });
      if (options.selectedStageMapPresetLabel() === label) options.setSelectedStageMapPresetLabel("");
      options.setMessage(`Removed stage map preset ${label}.`);
      await options.refreshSnapshot();
    } catch (error) { options.setMessage(String(error)); }
  };

  const exportStageMapPreset = async () => {
    try {
      const stageObjectCount = options.snapshot().stage_objects.length;
      const path = await options.invoke<string | null>("save_stage_map_preset_file", {
        label: options.stageMapPresetLabel(),
        config: options.snapshot().stage_map,
      });
      options.setMessage(path
        ? `Exported stage map preset ${path} with ${stageObjectCount} stage object${stageObjectCount === 1 ? "" : "s"}.`
        : "Stage map preset export canceled.");
    } catch (error) { options.setMessage(String(error)); }
  };

  const importStageMapPreset = async () => {
    try {
      const label = await options.invoke<string | null>("load_stage_map_preset_file");
      if (label === null) {
        options.setMessage("Stage map preset import canceled.");
        return;
      }
      options.setStageMapPresetLabel(label);
      options.setSelectedStageMapPresetLabel(label);
      options.setMessage(`Imported stage map preset ${label}.`);
      await options.refreshSnapshot();
    } catch (error) { options.setMessage(String(error)); }
  };

  return {
    addStageObjectAtCenter,
    setStageObject,
    removeStageObject,
    setStageMapConfig,
    lockStageMapToCurrentBounds,
    stageMapPresetObjectCountLabel,
    mappingViewPresetObjectLabel,
    saveStageMapPreset,
    applyStageMapPreset,
    removeStageMapPreset,
    exportStageMapPreset,
    importStageMapPreset,
  };
}
