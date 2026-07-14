import { For, Show, createMemo, createSignal } from "solid-js";
import type {
  AudioReactiveCurve,
  AudioReactiveFeature,
  AudioSpectrumBand,
  AudioSpectrumSource,
  EffectKind,
  NodeGraphSummary,
  NodeGraphTransformOp,
} from "../types";
import { NodeGraphListPanel } from "./NodeGraphListPanel";

const audioFeatures: Array<{ value: AudioReactiveFeature; label: string }> = [
  { value: "LegacyBand", label: "3-band FFT" },
  { value: "Band", label: "Spectrum band 1–16" },
  { value: "Rms", label: "RMS loudness" },
  { value: "Peak", label: "Peak level" },
  { value: "SpectralFlux", label: "Spectral flux" },
  { value: "Onset", label: "Onset trigger" },
  { value: "OnsetStrength", label: "Onset strength" },
  { value: "BeatPhase", label: "Beat phase" },
  { value: "Bpm", label: "BPM (60–200)" },
  { value: "BpmConfidence", label: "BPM confidence" },
  { value: "SpectralCentroid", label: "Spectral centroid" },
  { value: "SpectralDensitySlow", label: "Spectral density · slow" },
  { value: "SpectralDensityFast", label: "Spectral density · fast" },
  { value: "Kick", label: "Kick trigger" },
  { value: "KickStrength", label: "Kick strength" },
  { value: "Snare", label: "Snare trigger" },
  { value: "SnareStrength", label: "Snare strength" },
];

interface NodeGraphEditorPanelProps {
  graphCount: number;
  label: string;
  transformOp: NodeGraphTransformOp;
  transformAmount: number;
  transformMin: number;
  transformMax: number;
  effectType: Extract<EffectKind, "Lfo" | "PositionWave">;
  sourceMode: "Effect" | "Audio";
  audioBand: AudioSpectrumBand;
  audioSource: AudioSpectrumSource;
  audioFeature: AudioReactiveFeature;
  audioBandIndex: number;
  audioGain: number;
  audioBias: number;
  audioGate: number;
  audioAttackMs: number;
  audioReleaseMs: number;
  audioHoldMs: number;
  audioCurve: AudioReactiveCurve;
  audioInvert: boolean;
  sourceLabel: string;
  sourceDetail: string;
  transformLabel: string;
  targetMode: "fixture" | "selection" | "group" | "video";
  canSave: boolean;
  saveError?: string | null;
  graphs: NodeGraphSummary[];
  targetLabel: (graph: NodeGraphSummary) => string;
  onLoadPreset: () => void | Promise<void>;
  onLabel: (label: string) => void;
  onSourceMode: (mode: "Effect" | "Audio") => void;
  onAudioBand: (band: AudioSpectrumBand) => void;
  onAudioSource: (source: AudioSpectrumSource) => void;
  onAudioFeature: (feature: AudioReactiveFeature) => void;
  onAudioBandIndex: (bandIndex: number) => void;
  onAudioGain: (gain: number) => void;
  onAudioBias: (bias: number) => void;
  onAudioGate: (gate: number) => void;
  onAudioAttackMs: (attackMs: number) => void;
  onAudioReleaseMs: (releaseMs: number) => void;
  onAudioHoldMs: (holdMs: number) => void;
  onAudioCurve: (curve: AudioReactiveCurve) => void;
  onAudioInvert: (invert: boolean) => void;
  onTransformOp: (op: NodeGraphTransformOp) => void;
  onTransformAmount: (amount: number) => void;
  onTransformMin: (min: number) => void;
  onTransformMax: (max: number) => void;
  onSaveGraph: () => void | Promise<void>;
  onResetTransform: () => void;
  onSetGraphEnabled: (graphId: number, enabled: boolean) => void | Promise<void>;
  onSaveGraphPreset: (graphId: number) => void | Promise<void>;
  onRemoveGraph: (graphId: number) => void | Promise<void>;
}

const clampUnit = (value: number) => Math.max(0, Math.min(1, Number.isFinite(value) ? value : 0));

export function NodeGraphEditorPanel(props: NodeGraphEditorPanelProps) {
  const [monitoredGraphId, setMonitoredGraphId] = createSignal<number | null>(null);
  const audioGraphs = createMemo(() => props.graphs.filter((graph) => graph.nodes.some((node) => node.kind === "Audio")));
  const monitoredGraph = () => {
    const requestedId = monitoredGraphId();
    return audioGraphs().find((graph) => graph.id === requestedId) ?? audioGraphs()[0] ?? null;
  };
  const monitoredAudioNode = () => monitoredGraph()?.nodes.find((node) => node.kind === "Audio") ?? null;
  const monitoredRuntime = () => {
    const graph = monitoredGraph();
    const audioNode = monitoredAudioNode();
    if (!graph || !audioNode) return null;
    return graph.audio_runtime?.find((runtime) => runtime.node_id === audioNode.id) ?? null;
  };
  const monitorState = () => {
    const graph = monitoredGraph();
    const runtime = monitoredRuntime();
    if (!graph) return { label: "SAVE TO MONITOR", className: "ready" };
    if (!graph.enabled) return { label: "DISABLED", className: "ready" };
    if (!runtime) return { label: "READY", className: "ready" };
    if (runtime.safety_zeroed || !runtime.source_available) return { label: "SAFE ZERO", className: "safe" };
    if (runtime.held) return { label: "HELD", className: "held" };
    return {
      label: runtime.feature_sequence == null ? "LIVE" : `LIVE · ${runtime.feature_sequence}`,
      className: "ready",
    };
  };

  return (
    <div class={`nodeGraphPanel ${props.sourceMode === "Audio" ? "audioReactiveMode" : ""}`}>
      <div class="panelHeader">
        <h3>{props.sourceMode === "Audio" ? "Audio Reactive Rack" : "Node Graph"}</h3>
        <div class="panelHeaderActions">
          <span class="tabularReadout">{props.graphCount}</span>
          <button onClick={() => void props.onLoadPreset()}>Load Graph</button>
        </div>
      </div>
      <div class="split">
        <label>
          Label
          <input value={props.label} onInput={(event) => props.onLabel(event.currentTarget.value)} />
        </label>
        <label>
          Source
          <select value={props.sourceMode} onInput={(event) => props.onSourceMode(event.currentTarget.value as "Effect" | "Audio")}>
            <option value="Effect">Current Effect</option>
            <option value="Audio">Audio Reactive</option>
          </select>
        </label>
      </div>

      <Show
        when={props.sourceMode === "Audio"}
        fallback={
          <>
            <div class="quad">
              <label>
                Transform
                <select value={props.transformOp} onInput={(event) => props.onTransformOp(event.currentTarget.value as NodeGraphTransformOp)}>
                  <option value="Scale">Scale</option>
                  <option value="Offset">Offset</option>
                  <option value="Clamp">Clamp</option>
                  <option value="Invert">Invert</option>
                  <option value="Abs">Abs</option>
                </select>
              </label>
            </div>
            <div class="triple">
              <label>
                Amount
                <input
                  type="number"
                  step="0.01"
                  value={props.transformAmount}
                  disabled={props.transformOp === "Invert" || props.transformOp === "Abs"}
                  onInput={(event) => props.onTransformAmount(Number(event.currentTarget.value))}
                />
              </label>
              <label>
                Min
                <input
                  type="number"
                  step="0.01"
                  value={props.transformMin}
                  disabled={props.transformOp !== "Clamp"}
                  onInput={(event) => props.onTransformMin(Number(event.currentTarget.value))}
                />
              </label>
              <label>
                Max
                <input
                  type="number"
                  step="0.01"
                  value={props.transformMax}
                  disabled={props.transformOp !== "Clamp"}
                  onInput={(event) => props.onTransformMax(Number(event.currentTarget.value))}
                />
              </label>
            </div>
            <svg class="nodeGraphCanvas" viewBox="0 0 100 44" aria-label="Node graph preview">
              <defs>
                <pattern id="node-graph-grid" width="5" height="5" patternUnits="userSpaceOnUse">
                  <path d="M 5 0 L 0 0 0 5" />
                </pattern>
                <marker id="node-graph-arrow" viewBox="0 0 8 8" refX="7" refY="4" markerWidth="4" markerHeight="4" orient="auto">
                  <path d="M 0 0 L 8 4 L 0 8 z" />
                </marker>
              </defs>
              <rect class="nodeGraphCanvasBg" x="0" y="0" width="100" height="44" />
              <line class="nodeGraphEdge" x1="31" y1="22" x2="39" y2="22" />
              <line class="nodeGraphEdge" x1="63" y1="22" x2="71" y2="22" />
              <g class={props.effectType === "PositionWave" ? "nodeGraphNode wave" : "nodeGraphNode lfo"} transform="translate(6 10)">
                <rect x="0" y="0" width="25" height="24" rx="2" />
                <text class="nodeGraphNodeLabel" x="12.5" y="10">{props.sourceLabel}</text>
                <text class="nodeGraphNodeDetail" x="12.5" y="17">{props.sourceDetail}</text>
              </g>
              <g class="nodeGraphNode transform" transform="translate(38 10)">
                <rect x="0" y="0" width="25" height="24" rx="2" />
                <text class="nodeGraphNodeLabel" x="12.5" y="10">Transform</text>
                <text class="nodeGraphNodeDetail" x="12.5" y="17">{props.transformLabel}</text>
              </g>
              <g class="nodeGraphNode output" transform="translate(70 10)">
                <rect x="0" y="0" width="25" height="24" rx="2" />
                <text class="nodeGraphNodeLabel" x="12.5" y="10">Output</text>
                <text class="nodeGraphNodeDetail" x="12.5" y="17">{props.targetMode}</text>
              </g>
            </svg>
          </>
        }
      >
        <section class="audioReactiveRack" aria-label="Audio reactive mapping controls">
          <div class="audioReactiveSection">
            <header><strong>Input</strong><span>{props.sourceDetail}</span></header>
            <div class="audioReactiveInputGrid">
              <label>
                Feed
                <select
                  value={props.audioSource}
                  onInput={(event) => {
                    const source = event.currentTarget.value as AudioSpectrumSource;
                    props.onAudioSource(source);
                    if (source === "Timeline") props.onAudioFeature("LegacyBand");
                  }}
                >
                  <option value="Live">Live Input</option>
                  <option value="Timeline">Timeline Analysis</option>
                </select>
              </label>
              <label>
                Feature
                <select
                  value={props.audioFeature}
                  disabled={props.audioSource === "Timeline"}
                  onInput={(event) => props.onAudioFeature(event.currentTarget.value as AudioReactiveFeature)}
                >
                  <For each={audioFeatures}>{(feature) => <option value={feature.value}>{feature.label}</option>}</For>
                </select>
              </label>
              <Show when={props.audioFeature === "LegacyBand"}>
                <label>
                  FFT band
                  <select value={props.audioBand} onInput={(event) => props.onAudioBand(event.currentTarget.value as AudioSpectrumBand)}>
                    <option value="Bass">Bass</option>
                    <option value="Mid">Mid</option>
                    <option value="High">High</option>
                  </select>
                </label>
              </Show>
              <Show when={props.audioFeature === "Band"}>
                <label>
                  Band 1–16
                  <input
                    type="number"
                    min="1"
                    max="16"
                    step="1"
                    value={props.audioBandIndex + 1}
                    onInput={(event) => props.onAudioBandIndex(Math.max(0, Math.min(15, Math.round(Number(event.currentTarget.value)) - 1)))}
                  />
                </label>
              </Show>
            </div>
            <div class="audioReactiveMonitorRow">
              <label>
                Monitor mapping
                <select
                  data-audio-runtime-monitor
                  disabled={audioGraphs().length === 0}
                  value={monitoredGraph()?.id ?? ""}
                  onInput={(event) => setMonitoredGraphId(Number(event.currentTarget.value))}
                >
                  <Show when={audioGraphs().length > 0} fallback={<option value="">No saved mapping</option>}>
                    <For each={audioGraphs()}>{(graph) => <option value={graph.id}>{graph.label}</option>}</For>
                  </Show>
                </select>
              </label>
              <span>Engine runtime · node {monitoredAudioNode()?.id ?? "—"}</span>
            </div>
            <div
              class="audioReactiveMeters"
              role="status"
              aria-live="polite"
              data-runtime-authoritative={monitoredRuntime() ? "true" : "false"}
              data-runtime-node-id={monitoredRuntime()?.node_id ?? ""}
            >
              <div>
                <span>IN</span>
                <meter aria-label="Engine audio feature input level" min="0" max="1" value={clampUnit(monitoredRuntime()?.input_value ?? 0)} />
                <strong>{clampUnit(monitoredRuntime()?.input_value ?? 0).toFixed(2)}</strong>
              </div>
              <div>
                <span>OUT</span>
                <meter aria-label="Engine audio mapping output" min="0" max="1" value={clampUnit(monitoredRuntime()?.output_value ?? 0)} />
                <strong>{clampUnit(monitoredRuntime()?.output_value ?? 0).toFixed(2)}</strong>
              </div>
              <b class={monitorState().className}>{monitorState().label}</b>
            </div>
          </div>

          <div class="audioReactiveSection">
            <header><strong>Dynamics</strong><span>0.00–1.00 normalized</span></header>
            <div class="audioReactiveControlGrid">
              <label>Gain<input type="number" min="0" step="0.1" value={props.audioGain} onInput={(event) => props.onAudioGain(Number(event.currentTarget.value))} /></label>
              <label>Bias<input type="number" min="-1" max="1" step="0.05" value={props.audioBias} onInput={(event) => props.onAudioBias(Number(event.currentTarget.value))} /></label>
              <label>Gate<input type="number" min="0" max="1" step="0.01" value={props.audioGate} onInput={(event) => props.onAudioGate(Number(event.currentTarget.value))} /></label>
              <label>Curve<select value={props.audioCurve} onInput={(event) => props.onAudioCurve(event.currentTarget.value as AudioReactiveCurve)}><option value="Linear">Linear</option><option value="Smoothstep">Smoothstep</option><option value="Exponential">Exponential</option><option value="Logarithmic">Logarithmic</option></select></label>
              <label>Attack ms<input class="tabularReadout" type="number" min="0" max="60000" step="5" value={props.audioAttackMs} onInput={(event) => props.onAudioAttackMs(Number(event.currentTarget.value))} /></label>
              <label>Release ms<input class="tabularReadout" type="number" min="0" max="60000" step="5" value={props.audioReleaseMs} onInput={(event) => props.onAudioReleaseMs(Number(event.currentTarget.value))} /></label>
              <label>Hold ms<input class="tabularReadout" type="number" min="0" max="60000" step="5" value={props.audioHoldMs} onInput={(event) => props.onAudioHoldMs(Number(event.currentTarget.value))} /></label>
              <label class="audioReactiveCheck"><input type="checkbox" checked={props.audioInvert} onInput={(event) => props.onAudioInvert(event.currentTarget.checked)} /><span>Invert output</span></label>
            </div>
          </div>

          <div class="audioReactiveSignalPath" aria-label="Audio mapping signal path">
            <span>{props.audioSource === "Live" ? "LIVE INPUT" : "TIMELINE"}</span>
            <b aria-hidden="true">→</b>
            <span>{audioFeatures.find((feature) => feature.value === props.audioFeature)?.label ?? props.audioFeature}</span>
            <b aria-hidden="true">→</b>
            <span>{props.targetMode.toUpperCase()}</span>
          </div>
        </section>
      </Show>

      <Show when={props.saveError}><p class="nodeGraphInlineError" role="alert">{props.saveError}</p></Show>
      <div class="buttonRow">
        <button class="primary" onClick={() => void props.onSaveGraph()} disabled={!props.canSave}>Save Graph</button>
        <button onClick={props.onResetTransform}>{props.sourceMode === "Audio" ? "Reset Response" : "Reset Transform"}</button>
      </div>
      <NodeGraphListPanel
        graphs={props.graphs}
        targetLabel={props.targetLabel}
        onSetEnabled={props.onSetGraphEnabled}
        onSavePreset={props.onSaveGraphPreset}
        onRemoveGraph={props.onRemoveGraph}
      />
    </div>
  );
}
