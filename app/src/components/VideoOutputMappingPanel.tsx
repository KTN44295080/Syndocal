import { For, Show } from "solid-js";
import type {
  VideoOutputAspectMode,
  VideoOutputMapping,
  VideoOutputMappingPresetSummary,
  VideoOutputSummary,
} from "../types";
import {
  defaultVideoOutputMapping,
  aspectRatioLabel,
  mappingCorrectionReadout,
  mappingReadout,
  outputAspectRatio,
  resetVideoOutputCornerOffsets,
  resetVideoOutputLensKeystone,
  resetVideoOutputWarp,
  videoOutputAspectPresets,
  videoOutputAspectModes,
} from "../videoOutputMapping";
import { ProjectorMapEditor } from "./ProjectorMapEditor";

type MaybePromise = void | Promise<unknown>;
type VideoOutputPreviewMode = "output" | "test";

type VideoOutputMappingPanelProps = {
  output: VideoOutputSummary;
  mappingPresetLabel: string;
  selectedMappingPresetLabel: string;
  mappingPresets: VideoOutputMappingPresetSummary[];
  previewOutputId: number | null;
  previewMode: VideoOutputPreviewMode;
  previewInfo: string;
  previewUrl: string;
  onMappingPresetLabel: (value: string) => void;
  onSelectedMappingPresetLabel: (value: string) => void;
  onSavePreset: (mapping: VideoOutputMapping) => MaybePromise;
  onExportPreset: (mapping: VideoOutputMapping) => MaybePromise;
  onImportPreset: () => MaybePromise;
  onApplyPreset: (outputId: number, label: string) => MaybePromise;
  onRemovePreset: (label: string) => MaybePromise;
  onSetMapping: (outputId: number, mapping: VideoOutputMapping) => MaybePromise;
};

export function VideoOutputMappingPanel(props: VideoOutputMappingPanelProps) {
  const patchMapping = (patch: Partial<VideoOutputMapping>) =>
    props.onSetMapping(props.output.id, { ...props.output.mapping, ...patch });

  const setAspectPreset = (aspectRatio: number) =>
    patchMapping({
      aspect_ratio: aspectRatio,
      aspect_mode: "Fit",
    });

  const previewLabel = () =>
    props.previewOutputId === props.output.id
      ? `${props.previewMode === "test" ? "Pattern" : "Output"} / ${props.previewInfo}`
      : "No preview";

  const nativeAspectLabel = () => aspectRatioLabel(outputAspectRatio(props.output.width, props.output.height));

  return (
    <div class="videoOutputMapping">
      <h3>Projector Map</h3>
      <div class="mappingReadout">
        <span>
          {props.output.width}x{props.output.height} / native {nativeAspectLabel()}
        </span>
        <strong>{mappingCorrectionReadout(props.output.mapping)}</strong>
        <span>{mappingReadout(props.output.mapping)}</span>
      </div>
      <div class="split">
        <label>
          Preset Label
          <input
            value={props.mappingPresetLabel}
            onInput={(event) => props.onMappingPresetLabel(event.currentTarget.value)}
          />
        </label>
        <label>
          Saved Preset
          <select
            value={props.selectedMappingPresetLabel}
            disabled={props.mappingPresets.length === 0}
            onInput={(event) => props.onSelectedMappingPresetLabel(event.currentTarget.value)}
          >
            <option value="">Select preset</option>
            <For each={props.mappingPresets}>
              {(preset) => <option value={preset.label}>{preset.label}</option>}
            </For>
          </select>
        </label>
      </div>
      <div class="presetRow">
        <button class="primary" onClick={() => void props.onSavePreset(props.output.mapping)}>
          Save Preset
        </button>
        <button onClick={() => void props.onExportPreset(props.output.mapping)}>
          Export
        </button>
        <button onClick={() => void props.onImportPreset()}>
          Import
        </button>
        <button
          disabled={!props.selectedMappingPresetLabel}
          onClick={() => void props.onApplyPreset(props.output.id, props.selectedMappingPresetLabel)}
        >
          Apply Preset
        </button>
        <button
          disabled={!props.selectedMappingPresetLabel}
          onClick={() => void props.onRemovePreset(props.selectedMappingPresetLabel)}
        >
          Delete Preset
        </button>
        <button onClick={() => void props.onSetMapping(props.output.id, defaultVideoOutputMapping)}>
          Reset
        </button>
        <button onClick={() => void props.onSetMapping(props.output.id, resetVideoOutputWarp(props.output.mapping))}>
          Clear Warp
        </button>
        <button onClick={() => void props.onSetMapping(props.output.id, resetVideoOutputLensKeystone(props.output.mapping))}>
          Clear Lens/Key
        </button>
        <button onClick={() => void props.onSetMapping(props.output.id, resetVideoOutputCornerOffsets(props.output.mapping))}>
          Clear Corners
        </button>
        <button onClick={() => void setAspectPreset(outputAspectRatio(props.output.width, props.output.height))}>
          Output Ratio
        </button>
        <For each={videoOutputAspectPresets}>
          {(preset) => (
            <button onClick={() => void setAspectPreset(preset.ratio)}>
              {preset.label}
            </button>
          )}
        </For>
      </div>
      <ProjectorMapEditor
        mapping={props.output.mapping}
        outputId={props.output.id}
        label={props.output.label}
        onPatch={(patch) => void patchMapping(patch)}
      />
      <div class="videoOutputPreviewCard">
        <div class="sectionHeader">
          <h4>Output Preview</h4>
          <span>{previewLabel()}</span>
        </div>
        <Show
          when={props.previewOutputId === props.output.id && props.previewUrl}
          fallback={<span class="emptyState">No preview</span>}
        >
          {(url) => <img src={url()} alt={`${props.output.label} preview`} />}
        </Show>
      </div>
      <div class="triple">
        <label>
          X
          <input
            type="number"
            step="0.01"
            value={props.output.mapping.offset_x}
            onChange={(event) => void patchMapping({ offset_x: Number(event.currentTarget.value) })}
          />
        </label>
        <label>
          Y
          <input
            type="number"
            step="0.01"
            value={props.output.mapping.offset_y}
            onChange={(event) => void patchMapping({ offset_y: Number(event.currentTarget.value) })}
          />
        </label>
        <label>
          Rotate
          <input
            type="number"
            step="0.1"
            value={props.output.mapping.rotation_deg}
            onChange={(event) => void patchMapping({ rotation_deg: Number(event.currentTarget.value) })}
          />
        </label>
      </div>
      <div class="triple">
        <label>
          Scale X
          <input
            type="number"
            min="0.01"
            step="0.01"
            value={props.output.mapping.scale_x}
            onChange={(event) => void patchMapping({ scale_x: Number(event.currentTarget.value) })}
          />
        </label>
        <label>
          Scale Y
          <input
            type="number"
            min="0.01"
            step="0.01"
            value={props.output.mapping.scale_y}
            onChange={(event) => void patchMapping({ scale_y: Number(event.currentTarget.value) })}
          />
        </label>
        <label>
          Aspect
          <input
            type="number"
            min="0.1"
            step="0.01"
            value={props.output.mapping.aspect_ratio}
            onChange={(event) => void patchMapping({ aspect_ratio: Number(event.currentTarget.value) })}
          />
        </label>
      </div>
      <div class="split">
        <label>
          Aspect Mode
          <select
            value={props.output.mapping.aspect_mode}
            onInput={(event) =>
              void patchMapping({ aspect_mode: event.currentTarget.value as VideoOutputAspectMode })
            }
          >
            <For each={videoOutputAspectModes}>{(mode) => <option value={mode}>{mode}</option>}</For>
          </select>
        </label>
        <label>
          Lens Distortion
          <input
            type="number"
            min="-1"
            max="1"
            step="0.01"
            value={props.output.mapping.lens_distortion}
            onChange={(event) => void patchMapping({ lens_distortion: Number(event.currentTarget.value) })}
          />
        </label>
      </div>
      <div class="split">
        <label>
          Keystone H
          <input
            type="number"
            min="-1"
            max="1"
            step="0.01"
            value={props.output.mapping.keystone_x}
            onChange={(event) => void patchMapping({ keystone_x: Number(event.currentTarget.value) })}
          />
        </label>
        <label>
          Keystone V
          <input
            type="number"
            min="-1"
            max="1"
            step="0.01"
            value={props.output.mapping.keystone_y}
            onChange={(event) => void patchMapping({ keystone_y: Number(event.currentTarget.value) })}
          />
        </label>
      </div>
      <div class="quad">
        <label>
          TL X
          <input
            type="number"
            min="-1"
            max="1"
            step="0.01"
            value={props.output.mapping.corner_top_left_x}
            onChange={(event) => void patchMapping({ corner_top_left_x: Number(event.currentTarget.value) })}
          />
        </label>
        <label>
          TL Y
          <input
            type="number"
            min="-1"
            max="1"
            step="0.01"
            value={props.output.mapping.corner_top_left_y}
            onChange={(event) => void patchMapping({ corner_top_left_y: Number(event.currentTarget.value) })}
          />
        </label>
        <label>
          TR X
          <input
            type="number"
            min="-1"
            max="1"
            step="0.01"
            value={props.output.mapping.corner_top_right_x}
            onChange={(event) => void patchMapping({ corner_top_right_x: Number(event.currentTarget.value) })}
          />
        </label>
        <label>
          TR Y
          <input
            type="number"
            min="-1"
            max="1"
            step="0.01"
            value={props.output.mapping.corner_top_right_y}
            onChange={(event) => void patchMapping({ corner_top_right_y: Number(event.currentTarget.value) })}
          />
        </label>
      </div>
      <div class="quad">
        <label>
          BR X
          <input
            type="number"
            min="-1"
            max="1"
            step="0.01"
            value={props.output.mapping.corner_bottom_right_x}
            onChange={(event) => void patchMapping({ corner_bottom_right_x: Number(event.currentTarget.value) })}
          />
        </label>
        <label>
          BR Y
          <input
            type="number"
            min="-1"
            max="1"
            step="0.01"
            value={props.output.mapping.corner_bottom_right_y}
            onChange={(event) => void patchMapping({ corner_bottom_right_y: Number(event.currentTarget.value) })}
          />
        </label>
        <label>
          BL X
          <input
            type="number"
            min="-1"
            max="1"
            step="0.01"
            value={props.output.mapping.corner_bottom_left_x}
            onChange={(event) => void patchMapping({ corner_bottom_left_x: Number(event.currentTarget.value) })}
          />
        </label>
        <label>
          BL Y
          <input
            type="number"
            min="-1"
            max="1"
            step="0.01"
            value={props.output.mapping.corner_bottom_left_y}
            onChange={(event) => void patchMapping({ corner_bottom_left_y: Number(event.currentTarget.value) })}
          />
        </label>
      </div>
    </div>
  );
}
