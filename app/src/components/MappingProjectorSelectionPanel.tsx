import { For, Show } from "solid-js";
import type { StageObjectSummary, VideoOutputMapping, VideoOutputSummary } from "../types";
import { MappingProjectorControlsPanel } from "./MappingProjectorControlsPanel";

type MaybePromise = void | Promise<unknown>;

type MappingProjectorSelectionPanelProps = {
  outputs: VideoOutputSummary[];
  selectedOutput: VideoOutputSummary | null;
  selectedOutputId: number | null;
  selectedStageObject: StageObjectSummary | null;
  onSelectOutput: (outputId: number) => void;
  onSetEnabled: (outputId: number, enabled: boolean) => MaybePromise;
  onSetBlackout: (outputId: number, blackout: boolean) => MaybePromise;
  onOpenWindow: (outputId: number, testPattern?: boolean) => MaybePromise;
  onSyncWindow: (outputId: number) => MaybePromise;
  onFitStageObject: (output: VideoOutputSummary, object: StageObjectSummary) => MaybePromise;
  onSetMapping: (outputId: number, mapping: VideoOutputMapping) => MaybePromise;
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
            onSetEnabled={props.onSetEnabled}
            onSetBlackout={props.onSetBlackout}
            onOpenWindow={props.onOpenWindow}
            onSyncWindow={props.onSyncWindow}
            onFitStageObject={props.onFitStageObject}
            onSetMapping={props.onSetMapping}
            onEditProjection={props.onEditProjection}
          />
        )}
      </Show>
    </>
  );
}
