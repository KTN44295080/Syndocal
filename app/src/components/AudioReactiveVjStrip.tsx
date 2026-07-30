import { For, Show, createMemo } from "solid-js";
import type { NodeGraphAudioRuntimeStatus, NodeGraphSummary } from "../types";

interface AudioReactiveVjStripProps {
  graphs: NodeGraphSummary[];
  onSetEnabled: (graphId: number, enabled: boolean) => void | Promise<void>;
}

type AudioReactiveGraphState = "off" | "ready" | "safe" | "held" | "live";

const runtimeForGraph = (graph: NodeGraphSummary): NodeGraphAudioRuntimeStatus | null => {
  const audioNode = graph.nodes.find((node) => node.kind === "Audio");
  if (!audioNode) return null;
  return graph.audio_runtime?.find((runtime) => runtime.node_id === audioNode.id) ?? null;
};

const graphState = (graph: NodeGraphSummary): AudioReactiveGraphState => {
  if (!graph.enabled) return "off";
  const runtime = runtimeForGraph(graph);
  if (!runtime) return "ready";
  if (runtime.safety_zeroed || !runtime.source_available) return "safe";
  if (runtime.held) return "held";
  return "live";
};

export function AudioReactiveVjStrip(props: AudioReactiveVjStripProps) {
  const graphs = createMemo(() => props.graphs.filter((graph) => graph.nodes.some((node) => node.kind === "Audio")));
  const identityState = () => {
    const enabledGraphs = graphs().filter((graph) => graph.enabled);
    const liveCount = enabledGraphs.filter((graph) => {
      const state = graphState(graph);
      return state === "live" || state === "held";
    }).length;
    if (liveCount > 0) return `${liveCount}/${enabledGraphs.length} LIVE`;
    const safeCount = enabledGraphs.filter((graph) => graphState(graph) === "safe").length;
    if (safeCount > 0) return safeCount === enabledGraphs.length ? "SAFE ZERO" : `${safeCount}/${enabledGraphs.length} SAFE ZERO`;
    if (graphs().length > 0 && enabledGraphs.length === 0) return `0/${graphs().length} OFF`;
    return `${enabledGraphs.length}/${graphs().length} READY`;
  };

  return (
    <section class="audioReactiveVjStrip" aria-label="Audio reactive live rack">
      <div class="audioReactiveVjIdentity">
        <small>REACTIVE</small>
        <strong>{identityState()}</strong>
      </div>
      <Show
        when={graphs().length > 0}
        fallback={
          <div class="audioReactiveVjEmpty textPretty">
            <span>No audio mappings</span>
            <small>Load a project containing an audio Node Graph mapping.</small>
          </div>
        }
      >
        <div class="audioReactiveVjRows">
          <For each={graphs().slice(0, 2)}>
            {(graph) => {
              const runtime = () => runtimeForGraph(graph);
              const state = () => graphState(graph);
              return (
                <div class="audioReactiveVjRow" data-runtime-state={state()}>
                  <span title={graph.label}>{graph.label}</span>
                  <meter min="0" max="1" value={runtime()?.output_value ?? 0} aria-label={`${graph.label} live output`} />
                  <b class={state() === "safe" ? "safe" : state() === "held" ? "held" : "ready"}>
                    {state() === "off"
                      ? "OFF"
                      : state() === "safe"
                        ? "SAFE ZERO"
                        : state() === "ready"
                          ? "READY"
                          : state() === "held"
                            ? "HELD"
                            : (runtime()?.output_value ?? 0).toFixed(2)}
                  </b>
                  <button
                    aria-label={`${graph.enabled ? "Disable" : "Enable"} ${graph.label}`}
                    aria-pressed={graph.enabled}
                    onClick={() => void props.onSetEnabled(graph.id, !graph.enabled)}
                  >
                    {graph.enabled ? "ON" : "OFF"}
                  </button>
                </div>
              );
            }}
          </For>
        </div>
        <div class="audioReactiveVjActions">
          <Show when={graphs().length > 2}><span>+{graphs().length - 2}</span></Show>
        </div>
      </Show>
    </section>
  );
}
