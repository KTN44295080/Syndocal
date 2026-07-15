import type { StageObjectSummary, VideoOutputMapping, VideoOutputSummary } from "../types";
import { mappingCorrectionReadout } from "../videoOutputMapping";

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
  onEditProjection: (outputId: number) => void;
};

// Mapping keeps only floor-plan-legitimate output properties: stage X/Z/rotation and
// enable/blackout/window state. Keystone, corner-pin, lens, and aspect editing lives in
// Setup > Video's Projection Map; this panel jumps there instead of duplicating it.
export function MappingProjectorControlsPanel(props: MappingProjectorControlsPanelProps) {
  const patchMapping = (patch: Partial<VideoOutputMapping>) =>
    props.onSetMapping(props.output.id, { ...props.output.mapping, ...patch });

  return (
    <div class="mappingProjectorControls">
      <div class="mappingProjectorSummary">
        <strong data-no-localize>{props.output.label}</strong>
        <span>{props.output.width}x{props.output.height} / {mappingCorrectionReadout(props.output.mapping)}</span>
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
          Rot
          <input
            type="number"
            step="1"
            value={props.output.mapping.rotation_deg}
            onChange={(event) => void patchMapping({ rotation_deg: Number(event.currentTarget.value) })}
          />
        </label>
      </div>
      <button
        class="mappingProjectorEditProjection"
        onClick={() => props.onEditProjection(props.output.id)}
      >
        Edit Projection in Video Setup
      </button>
    </div>
  );
}
