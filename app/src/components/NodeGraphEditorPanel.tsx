import type { EffectKind, NodeGraphSummary, NodeGraphTransformOp } from "../types";
import { NodeGraphListPanel } from "./NodeGraphListPanel";

interface NodeGraphEditorPanelProps {
  graphCount: number;
  label: string;
  transformOp: NodeGraphTransformOp;
  transformAmount: number;
  transformMin: number;
  transformMax: number;
  effectType: EffectKind;
  sourceLabel: string;
  sourceDetail: string;
  transformLabel: string;
  targetMode: "fixture" | "group" | "video";
  canSave: boolean;
  graphs: NodeGraphSummary[];
  targetLabel: (graph: NodeGraphSummary) => string;
  onLoadPreset: () => void | Promise<void>;
  onLabel: (label: string) => void;
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

export function NodeGraphEditorPanel(props: NodeGraphEditorPanelProps) {
  return (
    <div class="nodeGraphPanel">
      <div class="panelHeader">
        <h3>Node Graph</h3>
        <div class="panelHeaderActions">
          <span>{props.graphCount}</span>
          <button onClick={() => void props.onLoadPreset()}>Load Graph</button>
        </div>
      </div>
      <div class="split">
        <label>
          Label
          <input value={props.label} onInput={(event) => props.onLabel(event.currentTarget.value)} />
        </label>
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
          <text class="nodeGraphNodeLabel" x="12.5" y="10">
            {props.sourceLabel}
          </text>
          <text class="nodeGraphNodeDetail" x="12.5" y="17">
            {props.sourceDetail}
          </text>
        </g>
        <g class="nodeGraphNode transform" transform="translate(38 10)">
          <rect x="0" y="0" width="25" height="24" rx="2" />
          <text class="nodeGraphNodeLabel" x="12.5" y="10">
            Transform
          </text>
          <text class="nodeGraphNodeDetail" x="12.5" y="17">
            {props.transformLabel}
          </text>
        </g>
        <g class="nodeGraphNode output" transform="translate(70 10)">
          <rect x="0" y="0" width="25" height="24" rx="2" />
          <text class="nodeGraphNodeLabel" x="12.5" y="10">
            Output
          </text>
          <text class="nodeGraphNodeDetail" x="12.5" y="17">
            {props.targetMode}
          </text>
        </g>
      </svg>
      <div class="buttonRow">
        <button class="primary" onClick={() => void props.onSaveGraph()} disabled={!props.canSave}>
          Save Graph
        </button>
        <button onClick={props.onResetTransform}>Reset Transform</button>
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
