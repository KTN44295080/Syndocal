import { For, Show } from "solid-js";
import type { NodeGraphSummary } from "../types";

interface NodeGraphListPanelProps {
  graphs: NodeGraphSummary[];
  targetLabel: (graph: NodeGraphSummary) => string;
  onSetEnabled: (graphId: number, enabled: boolean) => void | Promise<void>;
  onSavePreset: (graphId: number) => void | Promise<void>;
  onRemoveGraph: (graphId: number) => void | Promise<void>;
}

export function NodeGraphListPanel(props: NodeGraphListPanelProps) {
  return (
    <Show when={props.graphs.length > 0}>
      <div class="effectList nodeGraphList">
        <For each={props.graphs}>
          {(graph) => {
            const hasWaveNode = () => graph.nodes.some((node) => node.kind === "PositionWave");
            const clockSyncBeats = () => {
              const sourceNode = graph.nodes.find((node) => node.lfo?.clock_sync || node.position_wave?.clock_sync);
              return sourceNode?.lfo?.clock_sync?.beats ?? sourceNode?.position_wave?.clock_sync?.beats ?? null;
            };
            return (
              <div class={graph.enabled ? "effectItem" : "effectItem editing"}>
                <div class="effectItemSummary">
                  <strong>{graph.label}</strong>
                  <span>{props.targetLabel(graph)}</span>
                  <div class="nodeGraphMetaRow">
                    <span class={`nodeGraphMetaChip ${hasWaveNode() ? "wave" : "lfo"}`}>
                      {hasWaveNode() ? "Wave" : "LFO"}
                    </span>
                    <Show when={clockSyncBeats()}>
                      {(beats) => <span class="nodeGraphMetaChip sync">sync {beats()} beat</span>}
                    </Show>
                    <span class="nodeGraphMetaChip">{graph.nodes.length} node(s)</span>
                    <span class="nodeGraphMetaChip target">{graph.edges.length} edge(s)</span>
                  </div>
                </div>
                <button onClick={() => void props.onSetEnabled(graph.id, !graph.enabled)}>
                  {graph.enabled ? "Disable" : "Enable"}
                </button>
                <button onClick={() => void props.onSavePreset(graph.id)}>Save</button>
                <button onClick={() => void props.onRemoveGraph(graph.id)}>Remove</button>
              </div>
            );
          }}
        </For>
      </div>
    </Show>
  );
}
