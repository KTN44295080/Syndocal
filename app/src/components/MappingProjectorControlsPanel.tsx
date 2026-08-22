import type { StageObjectSummary, VideoOutputSummary } from "../types";
import { mappingCorrectionReadout } from "../videoOutputMapping";

type MappingProjectorControlsPanelProps = {
  output: VideoOutputSummary;
  selectedStageObject: StageObjectSummary | null;
  onEditProjection: (outputId: number) => void;
};

// Mapping keeps a read-only projection summary. Physical window control and any
// authored mutation live in Setup > Video's canonical v2 action lane.
export function MappingProjectorControlsPanel(props: MappingProjectorControlsPanelProps) {
  return (
    <div class="mappingProjectorControls">
      <div class="mappingProjectorSummary">
        <strong data-no-localize>{props.output.label}</strong>
        <span>{props.output.width}x{props.output.height} / {mappingCorrectionReadout(props.output.mapping)}</span>
        <small>
          Authored {props.output.enabled ? "enabled" : "disabled"} / {props.output.blackout ? "Blackout" : `${Math.round(props.output.opacity * 100)}% opacity`}
        </small>
      </div>
      <p class="inlineUnavailable" role="status">
        Projection mapping/configuration mutation is unavailable until a canonical path exists. This view is read-only.
      </p>
      <button
        class="mappingProjectorEditProjection"
        onClick={() => props.onEditProjection(props.output.id)}
      >
        Edit Projection in Video Setup
      </button>
    </div>
  );
}
