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
            const audioNodes = () => graph.nodes.filter((node) => node.kind === "Audio" && node.audio);
            const hasAudioNode = () => audioNodes().length > 0;
            const audioNode = () => audioNodes()[0]?.audio ?? null;
            const audioRuntime = () => graph.audio_runtime?.[0] ?? null;
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
                    <span class={`nodeGraphMetaChip ${hasAudioNode() ? "audio" : hasWaveNode() ? "wave" : "lfo"}`}>
                      {hasAudioNode() ? "Audio" : hasWaveNode() ? "Wave" : "LFO"}
                    </span>
                    <Show when={hasAudioNode()}>
                      <span class="nodeGraphMetaChip audioFeature">
                        {audioNode()?.source ?? "Timeline"} · {audioNode()?.feature ?? "LegacyBand"}
                      </span>
                    </Show>
                    <Show when={clockSyncBeats()}>
                      {(beats) => <span class="nodeGraphMetaChip sync">sync {beats()} beat</span>}
                    </Show>
                    <span class="nodeGraphMetaChip">{graph.nodes.length} node(s)</span>
                    <span class="nodeGraphMetaChip target">{graph.edges.length} edge(s)</span>
                  </div>
                  <Show when={hasAudioNode()}>
                    <div class="nodeGraphAudioRuntime" role="status">
                      <meter
                        min="0"
                        max="1"
                        value={audioRuntime()?.output_value ?? 0}
                        aria-label={`${graph.label} audio reactive output`}
                      />
                      <span class="tabularReadout">{(audioRuntime()?.output_value ?? 0).toFixed(2)}</span>
                      <b class={audioRuntime()?.safety_zeroed || !audioRuntime()?.source_available ? "safe" : audioRuntime()?.held ? "held" : "ready"}>
                        {!graph.enabled
                          ? "DISABLED"
                          : audioRuntime()?.safety_zeroed || !audioRuntime()?.source_available
                            ? "SAFE ZERO"
                            : audioRuntime()?.held
                              ? "HELD"
                              : audioRuntime()?.feature_sequence == null
                                ? "LIVE"
                                : `LIVE · ${audioRuntime()?.feature_sequence}`}
                      </b>
                    </div>
                  </Show>
                </div>
                <button aria-label={`${graph.enabled ? "Disable" : "Enable"} ${graph.label}`} onClick={() => void props.onSetEnabled(graph.id, !graph.enabled)}>
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
