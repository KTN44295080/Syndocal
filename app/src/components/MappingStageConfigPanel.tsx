import { For } from "solid-js";
import type { MappingViewPreset } from "../mappingViewPresets";
import type { StageWorldBounds } from "../stageGeometry";
import type { StageMapConfig, StageMapPresetSummary } from "../types";

type MappingStageConfigPanelProps = {
  stageMap: StageMapConfig;
  stageWorldBounds: StageWorldBounds;
  stageMapPresetLabel: string;
  selectedStageMapPresetLabel: string;
  stageMapPresets: StageMapPresetSummary[];
  mappingViewPresetLabel: string;
  selectedMappingViewPresetId: string;
  mappingViewPresets: MappingViewPreset[];
  stageMapPresetLabelFor: (preset: StageMapPresetSummary) => string;
  mappingViewPresetLabelFor: (preset: MappingViewPreset) => string;
  onLockToCurrentBounds: () => void | Promise<void>;
  onSetStageMapConfig: (updates: Partial<StageMapConfig>, successMessage?: string) => void | Promise<void>;
  onStageMapPresetLabel: (label: string) => void;
  onSelectedStageMapPresetLabel: (label: string) => void;
  onSaveStageMapPreset: () => void | Promise<void>;
  onExportStageMapPreset: () => void | Promise<void>;
  onImportStageMapPreset: () => void | Promise<void>;
  onApplyStageMapPreset: (label: string) => void | Promise<void>;
  onRemoveStageMapPreset: (label: string) => void | Promise<void>;
  onMappingViewPresetLabel: (label: string) => void;
  onSelectedMappingViewPresetId: (id: string) => void;
  onSaveMappingViewPreset: () => void;
  onApplyMappingViewPreset: (id: string) => void;
  onRemoveMappingViewPreset: (id: string) => void;
  onExportMappingStageSvg: () => void;
  onExportVisualizerRenderPayload: () => void | Promise<void>;
};

export function MappingStageConfigPanel(props: MappingStageConfigPanelProps) {
  return (
    <div class="mappingStageBoundsPanel">
      <div class="mappingStageBoundsStatus">
        <strong>Stage Bounds</strong>
        <span>
          {props.stageMap.locked ? "Locked" : "Auto fit"} / X {props.stageWorldBounds.minX.toFixed(1)} to{" "}
          {props.stageWorldBounds.maxX.toFixed(1)} / Z {props.stageWorldBounds.minZ.toFixed(1)} to{" "}
          {props.stageWorldBounds.maxZ.toFixed(1)}
        </span>
      </div>
      <div class="mappingStageBoundsActions">
        <button
          class={props.stageMap.locked ? "active" : ""}
          onClick={() => void props.onLockToCurrentBounds()}
        >
          Fit Current
        </button>
        <button
          class={!props.stageMap.locked ? "active" : ""}
          onClick={() => void props.onSetStageMapConfig({ locked: false }, "Unlocked 2D stage map auto-fit.")}
        >
          Auto Fit
        </button>
      </div>
      <div class="mappingStageBoundsGrid">
        <label>
          Min X
          <input
            type="number"
            step="0.1"
            value={props.stageMap.min_x}
            onChange={(event) =>
              void props.onSetStageMapConfig({ locked: true, min_x: Number(event.currentTarget.value) })
            }
          />
        </label>
        <label>
          Max X
          <input
            type="number"
            step="0.1"
            value={props.stageMap.max_x}
            onChange={(event) =>
              void props.onSetStageMapConfig({ locked: true, max_x: Number(event.currentTarget.value) })
            }
          />
        </label>
        <label>
          Min Z
          <input
            type="number"
            step="0.1"
            value={props.stageMap.min_z}
            onChange={(event) =>
              void props.onSetStageMapConfig({ locked: true, min_z: Number(event.currentTarget.value) })
            }
          />
        </label>
        <label>
          Max Z
          <input
            type="number"
            step="0.1"
            value={props.stageMap.max_z}
            onChange={(event) =>
              void props.onSetStageMapConfig({ locked: true, max_z: Number(event.currentTarget.value) })
            }
          />
        </label>
      </div>
      <div class="mappingStagePresetPanel">
        <label>
          Preset
          <input
            type="text"
            value={props.stageMapPresetLabel}
            onInput={(event) => props.onStageMapPresetLabel(event.currentTarget.value)}
          />
        </label>
        <label>
          Stored
          <select
            value={props.selectedStageMapPresetLabel}
            disabled={props.stageMapPresets.length === 0}
            onInput={(event) => props.onSelectedStageMapPresetLabel(event.currentTarget.value)}
          >
            <option value="">Select preset</option>
            <For each={props.stageMapPresets}>
              {(preset) => (
                <option value={preset.label}>
                  {preset.label} ({props.stageMapPresetLabelFor(preset)})
                </option>
              )}
            </For>
          </select>
        </label>
        <div class="mappingStagePresetActions">
          <button onClick={() => void props.onSaveStageMapPreset()}>
            Save
          </button>
          <button onClick={() => void props.onExportStageMapPreset()}>
            Export
          </button>
          <button onClick={() => void props.onImportStageMapPreset()}>
            Import
          </button>
          <button
            disabled={!props.selectedStageMapPresetLabel}
            onClick={() => void props.onApplyStageMapPreset(props.selectedStageMapPresetLabel)}
          >
            Apply
          </button>
          <button
            disabled={!props.selectedStageMapPresetLabel}
            onClick={() => void props.onRemoveStageMapPreset(props.selectedStageMapPresetLabel)}
          >
            Delete
          </button>
        </div>
      </div>
      <div class="mappingStagePresetPanel mappingViewPresetPanel">
        <label>
          View
          <input
            type="text"
            value={props.mappingViewPresetLabel}
            onInput={(event) => props.onMappingViewPresetLabel(event.currentTarget.value)}
          />
        </label>
        <label>
          Stored View
          <select
            value={props.selectedMappingViewPresetId}
            disabled={props.mappingViewPresets.length === 0}
            onChange={(event) => props.onSelectedMappingViewPresetId(event.currentTarget.value)}
          >
            <option value="">Select view</option>
            <For each={props.mappingViewPresets}>
              {(preset) => (
                <option value={preset.id}>
                  {preset.label} ({props.mappingViewPresetLabelFor(preset)})
                </option>
              )}
            </For>
          </select>
        </label>
        <div class="mappingStagePresetActions viewPresetActions">
          <button onClick={props.onSaveMappingViewPreset}>
            Save View
          </button>
          <button
            disabled={!props.selectedMappingViewPresetId}
            onClick={() => props.onApplyMappingViewPreset(props.selectedMappingViewPresetId)}
          >
            Apply View
          </button>
          <button
            disabled={!props.selectedMappingViewPresetId}
            onClick={() => props.onRemoveMappingViewPreset(props.selectedMappingViewPresetId)}
          >
            Delete View
          </button>
          <button onClick={props.onExportMappingStageSvg}>
            Export SVG
          </button>
          <button onClick={() => void props.onExportVisualizerRenderPayload()}>
            Export JSON
          </button>
        </div>
      </div>
    </div>
  );
}
