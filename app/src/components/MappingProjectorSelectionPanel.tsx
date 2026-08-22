import { For, Show } from "solid-js";
import type { StageObjectSummary, VideoOutputSummary } from "../types";
import { MappingProjectorControlsPanel } from "./MappingProjectorControlsPanel";

type MappingProjectorSelectionPanelProps = {
  outputs: VideoOutputSummary[];
  selectedOutput: VideoOutputSummary | null;
  selectedOutputId: number | null;
  selectedStageObject: StageObjectSummary | null;
  onSelectOutput: (outputId: number) => void;
  onEditProjection: (outputId: number) => void;
};

export function MappingProjectorSelectionPanel(props: MappingProjectorSelectionPanelProps) {
  return (
    <>
      <div class="mappingSelectionHeader secondary">
        <strong>Projection Surfaces</strong>
        <span>{props.outputs.length}</span>
      </div>
      <For each={props.outputs}>
        {(output) => (
          <button
            class={`projector ${props.selectedOutputId === output.id ? "active" : ""}`}
            onClick={() => props.onSelectOutput(output.id)}
          >
            <strong data-no-localize>{output.label}</strong>
            <span>{output.width}x{output.height}</span>
            <small>{output.kind} / {output.enabled ? "Enabled" : "Disabled"} / {output.blackout ? "Blackout" : "Live"}</small>
          </button>
        )}
      </For>
      <Show when={props.selectedOutput}>
        {(output) => (
          <MappingProjectorControlsPanel
            output={output()}
            selectedStageObject={props.selectedStageObject}
            onEditProjection={props.onEditProjection}
          />
        )}
      </Show>
    </>
  );
}
