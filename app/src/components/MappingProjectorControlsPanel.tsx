import { For } from "solid-js";
import type {
  StageObjectSummary,
  VideoOutputAspectMode,
  VideoOutputMapping,
  VideoOutputSummary,
} from "../types";
import {
  defaultVideoOutputMapping,
  mappingCorrectionReadout,
  outputAspectRatio,
  resetVideoOutputCornerOffsets,
  resetVideoOutputLensKeystone,
  resetVideoOutputPose,
  resetVideoOutputStagePosition,
  resetVideoOutputWarp,
  videoOutputAspectPresets,
  videoOutputAspectModes,
} from "../videoOutputMapping";
import { ProjectorMapEditor, ProjectorMapPreview } from "./ProjectorMapEditor";

type MaybePromise = void | Promise<unknown>;

type MappingProjectorControlsPanelProps = {
  output: VideoOutputSummary;
  selectedStageObject: StageObjectSummary | null;
  onSetEnabled: (outputId: number, enabled: boolean) => MaybePromise;
  onSetBlackout: (outputId: number, blackout: boolean) => MaybePromise;
  onOpenWindow: (outputId: number, testPattern?: boolean) => MaybePromise;
  onSyncWindow: (outputId: number) => MaybePromise;
  onFitStageObject: (output: VideoOutputSummary, object: StageObjectSummary) => MaybePromise;
  onSetMapping: (outputId: number, mapping: VideoOutputMapping) => MaybePromise;
};

export function MappingProjectorControlsPanel(props: MappingProjectorControlsPanelProps) {
  const patchMapping = (patch: Partial<VideoOutputMapping>) =>
    props.onSetMapping(props.output.id, { ...props.output.mapping, ...patch });

  const setAspectPreset = (aspectRatio: number) =>
    patchMapping({
      aspect_ratio: aspectRatio,
      aspect_mode: "Fit",
    });

  return (
    <div class="mappingProjectorControls">
      <div class="mappingProjectorPreview">
        <ProjectorMapPreview
          mapping={props.output.mapping}
          outputId={props.output.id}
          class="projectorMapSurface outputMappingMiniSurface"
        />
        <div>
          <strong data-no-localize>{props.output.label}</strong>
          <span>{mappingCorrectionReadout(props.output.mapping)}</span>
        </div>
      </div>
      <div class="mappingProjectorActionRow">
        <button onClick={() => void props.onSetEnabled(props.output.id, !props.output.enabled)}>
          {props.output.enabled ? "Disable" : "Enable"}
        </button>
        <button onClick={() => void props.onSetBlackout(props.output.id, !props.output.blackout)}>
          {props.output.blackout ? "Clear BO" : "Blackout"}
        </button>
        <button onClick={() => void props.onOpenWindow(props.output.id)}>Window</button>
        <button onClick={() => void props.onOpenWindow(props.output.id, true)}>Pattern</button>
        <button onClick={() => void props.onSyncWindow(props.output.id)}>Sync</button>
        <button
          disabled={!props.selectedStageObject}
          onClick={() => {
            if (props.selectedStageObject) {
              void props.onFitStageObject(props.output, props.selectedStageObject);
            }
          }}
        >
          Fit Object
        </button>
      </div>
      <ProjectorMapEditor
        mapping={props.output.mapping}
        outputId={props.output.id}
        label={props.output.label}
        onPatch={(patch) => void patchMapping(patch)}
      />
      <div class="mappingProjectorFieldGrid">
        <label>
          X
          <input
            type="number"
            step="0.1"
            value={props.output.mapping.stage_x}
            onChange={(event) => void patchMapping({ stage_x: Number(event.currentTarget.value) })}
          />
        </label>
        <label>
          Z
          <input
            type="number"
            step="0.1"
            value={props.output.mapping.stage_z}
            onChange={(event) => void patchMapping({ stage_z: Number(event.currentTarget.value) })}
          />
        </label>
        <label>
          Y
          <input
            type="number"
            step="0.1"
            value={props.output.mapping.stage_y}
            onChange={(event) => void patchMapping({ stage_y: Number(event.currentTarget.value) })}
          />
        </label>
        <label>
          Rot
          <input
            type="number"
            step="1"
            value={props.output.mapping.rotation_deg}
            onChange={(event) => void patchMapping({ rotation_deg: Number(event.currentTarget.value) })}
          />
        </label>
      </div>
      <div class="mappingProjectorWarpGrid">
        <label>
          Aspect
          <input
            type="number"
            step="0.01"
            min="0.1"
            value={props.output.mapping.aspect_ratio}
            onChange={(event) => void patchMapping({ aspect_ratio: Number(event.currentTarget.value) })}
          />
        </label>
        <label>
          Mode
          <select
            value={props.output.mapping.aspect_mode}
            onChange={(event) =>
              void patchMapping({ aspect_mode: event.currentTarget.value as VideoOutputAspectMode })
            }
          >
            <For each={videoOutputAspectModes}>
              {(mode) => <option value={mode}>{mode}</option>}
            </For>
          </select>
        </label>
        <label>
          Lens
          <input
            type="number"
            step="0.01"
            min="-1"
            max="1"
            value={props.output.mapping.lens_distortion}
            onChange={(event) => void patchMapping({ lens_distortion: Number(event.currentTarget.value) })}
          />
        </label>
        <label>
          Keystone X
          <input
            type="number"
            step="0.01"
            min="-1"
            max="1"
            value={props.output.mapping.keystone_x}
            onChange={(event) => void patchMapping({ keystone_x: Number(event.currentTarget.value) })}
          />
        </label>
        <label>
          Keystone Y
          <input
            type="number"
            step="0.01"
            min="-1"
            max="1"
            value={props.output.mapping.keystone_y}
            onChange={(event) => void patchMapping({ keystone_y: Number(event.currentTarget.value) })}
          />
        </label>
        <div class="mappingProjectorPresetGrid">
          <button onClick={() => void props.onSetMapping(props.output.id, defaultVideoOutputMapping)}>
            Reset
          </button>
          <button onClick={() => void props.onSetMapping(props.output.id, resetVideoOutputStagePosition(props.output.mapping))}>
            Reset Stage
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
      </div>
    </div>
  );
}
