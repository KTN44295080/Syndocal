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
  resetVideoOutputBlend,
  resetVideoOutputLensKeystone,
  resetVideoOutputMask,
  resetVideoOutputPose,
  resetVideoOutputWarp,
  videoOutputAspectPresets,
  videoOutputAspectModes,
} from "../videoOutputMapping";
import { ProjectorMapEditor } from "./ProjectorMapEditor";

type MaybePromise = void | Promise<unknown>;

type VideoOutputMappingPanelProps = {
  output: VideoOutputSummary;
  mappingPresetLabel: string;
  selectedMappingPresetLabel: string;
  mappingPresets: VideoOutputMappingPresetSummary[];
  onMappingPresetLabel: (value: string) => void;
  onSelectedMappingPresetLabel: (value: string) => void;
  onSavePreset: (mapping: VideoOutputMapping) => MaybePromise;
  onExportPreset: (mapping: VideoOutputMapping) => MaybePromise;
  onImportPreset: () => MaybePromise;
  onApplyPreset: (outputId: number, label: string) => MaybePromise;
  onRemovePreset: (label: string) => MaybePromise;
  onSetMapping: (outputId: number, mapping: VideoOutputMapping) => MaybePromise;
  onImportBitmapMask: (output: VideoOutputSummary) => MaybePromise;
  onClearBitmapMask: (output: VideoOutputSummary) => MaybePromise;
};

export function VideoOutputMappingPanel(props: VideoOutputMappingPanelProps) {
  const patchMapping = (patch: Partial<VideoOutputMapping>) =>
    props.onSetMapping(props.output.id, { ...props.output.mapping, ...patch });

  const setAspectPreset = (aspectRatio: number) =>
    patchMapping({
      aspect_ratio: aspectRatio,
      aspect_mode: "Fit",
    });

  const nativeAspectLabel = () => aspectRatioLabel(outputAspectRatio(props.output.width, props.output.height));

  const regularMaskPoints = (count: number) => Array.from({ length: 8 }, (_, index) => {
    if (index >= count) return { x: 0, y: 0 };
    const angle = -Math.PI / 2 + (index / count) * Math.PI * 2;
    return { x: 0.5 + Math.cos(angle) * 0.46, y: 0.5 + Math.sin(angle) * 0.46 };
  });

  const setMaskPointCount = (count: number) => patchMapping({
    mask_point_count: count,
    mask_points: count >= 3 ? regularMaskPoints(count) : regularMaskPoints(0),
  });

  const patchMaskPoint = (index: number, coordinate: "x" | "y", value: number) => {
    const points = props.output.mapping.mask_points.map((point) => ({ ...point }));
    points[index] = { ...points[index], [coordinate]: Math.max(0, Math.min(1, value)) };
    return patchMapping({ mask_points: points });
  };

  return (
    <div class="videoOutputMapping">
      <h3>Projection Mapping</h3>
      <div class="mappingReadout">
        <span>
          {props.output.width}x{props.output.height} / native {nativeAspectLabel()}
        </span>
        <strong>{mappingCorrectionReadout(props.output.mapping)}</strong>
        <span>{mappingReadout(props.output.mapping)}</span>
      </div>
      <ProjectorMapEditor
        mapping={props.output.mapping}
        outputId={props.output.id}
        label={props.output.label}
        onPatch={(patch) => void patchMapping(patch)}
      />
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
              {(preset) => <option data-no-localize value={preset.label}>{preset.label}</option>}
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
        <button onClick={() => void props.onSetMapping(props.output.id, resetVideoOutputPose(props.output.mapping))}>
          Reset Pose
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
        <button onClick={() => void props.onSetMapping(props.output.id, resetVideoOutputBlend(props.output.mapping))}>
          Clear Blend
        </button>
        <button onClick={() => void props.onSetMapping(props.output.id, resetVideoOutputMask(props.output.mapping))}>
          Clear Mask
        </button>
        <button onClick={() => void setAspectPreset(outputAspectRatio(props.output.width, props.output.height))}>
          Output Ratio
        </button>
        <For each={videoOutputAspectPresets}>
          {(preset) => (
            <button onClick={() => void setAspectPreset(preset.ratio)}>
              <span data-no-localize>{preset.label}</span>
            </button>
          )}
        </For>
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
      <div class="videoOutputMaskControls">
        <div class="sectionHeader">
          <h4>Output Mask</h4>
          <span>Polygon + embedded luma bitmap · CPU/GPU parity</span>
        </div>
        <div class="presetRow">
          <button type="button" onClick={() => void props.onImportBitmapMask(props.output)}>
            Import Luma Mask
          </button>
          <button
            type="button"
            disabled={props.output.mapping.bitmap_mask_width === 0}
            onClick={() => void props.onClearBitmapMask(props.output)}
          >
            Clear Bitmap
          </button>
          <span class="videoBitmapMaskStatus" role="status">
            {props.output.mapping.bitmap_mask_width > 0
              ? `${props.output.mapping.bitmap_mask_width}x${props.output.mapping.bitmap_mask_height} embedded`
              : "No bitmap mask"}
          </span>
        </div>
        <div class="triple">
          <label>
            Shape
            <select
              value={props.output.mapping.mask_point_count}
              onInput={(event) => void setMaskPointCount(Number(event.currentTarget.value))}
            >
              <option value="0">Disabled</option>
              <For each={[3, 4, 5, 6, 7, 8]}>{(count) => <option value={count}>{count} points</option>}</For>
            </select>
          </label>
          <label>
            Feather
            <input
              type="number"
              min="0"
              max="0.5"
              step="0.005"
              value={props.output.mapping.mask_softness}
              onChange={(event) => void patchMapping({ mask_softness: Number(event.currentTarget.value) })}
            />
          </label>
          <label class="videoMaskInvertToggle">
            <input
              type="checkbox"
              checked={props.output.mapping.mask_invert}
              onChange={(event) => void patchMapping({ mask_invert: event.currentTarget.checked })}
            />
            Invert mask
          </label>
        </div>
        <Show when={props.output.mapping.mask_point_count >= 3}>
          <div class="videoMaskPointGrid tabularNums">
            <For each={props.output.mapping.mask_points.slice(0, props.output.mapping.mask_point_count)}>
              {(point, index) => (
                <fieldset>
                  <legend>Point {index() + 1}</legend>
                  <label>X<input type="number" min="0" max="1" step="0.005" value={point.x} onChange={(event) => void patchMaskPoint(index(), "x", Number(event.currentTarget.value))} /></label>
                  <label>Y<input type="number" min="0" max="1" step="0.005" value={point.y} onChange={(event) => void patchMaskPoint(index(), "y", Number(event.currentTarget.value))} /></label>
                </fieldset>
              )}
            </For>
          </div>
        </Show>
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
      <div class="videoOutputBlendControls">
        <div class="sectionHeader">
          <h4>Edge Blend / Black Level</h4>
          <span>Normalized projector edge width</span>
        </div>
        <div class="quad">
          <label>
            Left
            <input type="number" min="0" max="1" step="0.01" value={props.output.mapping.edge_blend_left} onChange={(event) => void patchMapping({ edge_blend_left: Number(event.currentTarget.value) })} />
          </label>
          <label>
            Right
            <input type="number" min="0" max="1" step="0.01" value={props.output.mapping.edge_blend_right} onChange={(event) => void patchMapping({ edge_blend_right: Number(event.currentTarget.value) })} />
          </label>
          <label>
            Top
            <input type="number" min="0" max="1" step="0.01" value={props.output.mapping.edge_blend_top} onChange={(event) => void patchMapping({ edge_blend_top: Number(event.currentTarget.value) })} />
          </label>
          <label>
            Bottom
            <input type="number" min="0" max="1" step="0.01" value={props.output.mapping.edge_blend_bottom} onChange={(event) => void patchMapping({ edge_blend_bottom: Number(event.currentTarget.value) })} />
          </label>
        </div>
        <div class="split">
          <label>
            Blend Gamma
            <input type="number" min="0.1" max="8" step="0.1" value={props.output.mapping.edge_blend_gamma} onChange={(event) => void patchMapping({ edge_blend_gamma: Number(event.currentTarget.value) })} />
          </label>
          <label>
            Black Level
            <input type="number" min="0" max="1" step="0.01" value={props.output.mapping.black_level} onChange={(event) => void patchMapping({ black_level: Number(event.currentTarget.value) })} />
          </label>
        </div>
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
