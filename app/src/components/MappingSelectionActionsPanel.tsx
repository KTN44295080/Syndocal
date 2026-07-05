import { Show } from "solid-js";

type MaybePromise = void | Promise<unknown>;
type MappingFixtureFlag = "highlight" | "solo" | "park";
type FixtureLayoutMode = "line" | "grid" | "circle";
type MappingAxis = "x" | "z";

export type MappingSelectionFlagState = {
  count: number;
  allHighlighted: boolean;
  allSoloed: boolean;
  allParked: boolean;
};

type MappingSelectionActionsPanelProps = {
  flagState: MappingSelectionFlagState;
  selectedCount: number;
  snapSize: number;
  hasSelectedStageObject: boolean;
  onSetFlag: (flag: MappingFixtureFlag, enabled: boolean) => MaybePromise;
  onNudge: (dx: number, dz: number) => MaybePromise;
  onLayoutSelection: (mode: FixtureLayoutMode) => MaybePromise;
  onLayoutOnStageObject: (mode: Exclude<FixtureLayoutMode, "circle">) => MaybePromise;
  onAlign: (axis: MappingAxis) => MaybePromise;
  onDistribute: (axis: MappingAxis) => MaybePromise;
  onMirror: (axis: MappingAxis) => MaybePromise;
  onRotate: (degrees: number) => MaybePromise;
  onDuplicate: () => MaybePromise;
  onRemove: () => MaybePromise;
  onControlActive: () => void;
  onUseSelectionAsEffectTarget: () => void;
  onUseSelectionAsWaveEffectTarget: () => void;
  onClearSelection: () => MaybePromise;
};

export function MappingSelectionActionsPanel(props: MappingSelectionActionsPanelProps) {
  const hasSelection = () => props.flagState.count > 0;

  return (
    <>
      <div class="mappingTransformInspector">
        <div class="mappingTransformTitle">
          <strong>Selection Flags</strong>
          <span>
            {hasSelection()
              ? `${props.flagState.count} fixture(s) picked`
              : "Pick fixtures on the map or list."}
          </span>
        </div>
        <div class="mappingFlagActions">
          <button
            class={props.flagState.allHighlighted ? "active" : ""}
            onClick={() => void props.onSetFlag("highlight", !props.flagState.allHighlighted)}
            disabled={!hasSelection()}
            title="Toggle highlight for the picked fixtures (Q)"
          >
            {props.flagState.allHighlighted ? "Clear High" : "Highlight"}
          </button>
          <button
            class={props.flagState.allSoloed ? "active" : ""}
            onClick={() => void props.onSetFlag("solo", !props.flagState.allSoloed)}
            disabled={!hasSelection()}
            title="Toggle solo for the picked fixtures (W)"
          >
            {props.flagState.allSoloed ? "Clear Solo" : "Solo"}
          </button>
          <button
            class={props.flagState.allParked ? "active" : ""}
            onClick={() => void props.onSetFlag("park", !props.flagState.allParked)}
            disabled={!hasSelection()}
            title="Toggle park for the picked fixtures (E)"
          >
            {props.flagState.allParked ? "Clear Park" : "Park"}
          </button>
        </div>
        <div class="mappingEffectActions">
          <button onClick={() => props.onUseSelectionAsEffectTarget()} disabled={!hasSelection()} title="Use picked fixtures as the Control/Edit effect target">
            Use in Effects
          </button>
          <button
            class="primary"
            onClick={() => props.onUseSelectionAsWaveEffectTarget()}
            disabled={!hasSelection()}
            title="Open Control/Edit with a Position Wave draft for the picked fixtures"
          >
            Wave Draft
          </button>
        </div>
        <div class="mappingNudgePanel">
          <span>Nudge {props.snapSize}m</span>
          <div class="mappingNudgeGrid">
            <button onClick={() => void props.onNudge(0, -props.snapSize)} disabled={!hasSelection()}>
              Up
            </button>
            <button onClick={() => void props.onNudge(-props.snapSize, 0)} disabled={!hasSelection()}>
              Left
            </button>
            <button onClick={() => void props.onNudge(props.snapSize, 0)} disabled={!hasSelection()}>
              Right
            </button>
            <button onClick={() => void props.onNudge(0, props.snapSize)} disabled={!hasSelection()}>
              Down
            </button>
          </div>
        </div>
      </div>
      <Show when={props.selectedCount > 1}>
        <div class="mappingTransformInspector">
          <div class="mappingTransformTitle">
            <strong>{props.selectedCount} fixtures selected</strong>
            <span>Drag any selected fixture to move the group.</span>
          </div>
          <div class="mappingTransformActions">
            <button onClick={() => void props.onLayoutSelection("line")} title="Layout selected fixtures in a line (1)">Line</button>
            <button onClick={() => void props.onLayoutSelection("grid")} title="Layout selected fixtures in a grid (2)">Grid</button>
            <button onClick={() => void props.onLayoutSelection("circle")} title="Layout selected fixtures in a circle (3)">Circle</button>
            <button
              onClick={() => void props.onLayoutOnStageObject("line")}
              disabled={!props.hasSelectedStageObject}
              title="Layout selected fixtures across the selected object (Shift+1)"
            >
              Object Line
            </button>
            <button
              onClick={() => void props.onLayoutOnStageObject("grid")}
              disabled={!props.hasSelectedStageObject}
              title="Layout selected fixtures in the selected object (Shift+2)"
            >
              Object Grid
            </button>
            <button onClick={() => void props.onAlign("x")} title="Align selected fixtures on X (X)">Align X</button>
            <button onClick={() => void props.onAlign("z")} title="Align selected fixtures on Z (Z)">Align Z</button>
            <button onClick={() => void props.onDistribute("x")} disabled={props.selectedCount < 3} title="Distribute selected fixtures on X (Shift+X)">
              Distribute X
            </button>
            <button onClick={() => void props.onDistribute("z")} disabled={props.selectedCount < 3} title="Distribute selected fixtures on Z (Shift+Z)">
              Distribute Z
            </button>
            <button onClick={() => void props.onMirror("x")} title="Mirror selected fixtures across X (Shift+[)">Mirror X</button>
            <button onClick={() => void props.onMirror("z")} title="Mirror selected fixtures across Z (Shift+])">Mirror Z</button>
            <button onClick={() => void props.onRotate(-15)} title="Rotate selected fixtures -15 degrees ([)">Rot -15</button>
            <button onClick={() => void props.onRotate(15)} title="Rotate selected fixtures +15 degrees (])">Rot +15</button>
            <button onClick={() => void props.onRotate(180)} title="Rotate selected fixtures 180 degrees (4)">Flip 180</button>
            <button onClick={() => void props.onDuplicate()}>Duplicate</button>
            <button onClick={() => void props.onRemove()}>Remove</button>
            <button onClick={() => props.onControlActive()}>Control Active</button>
            <button onClick={() => void props.onClearSelection()}>Clear</button>
          </div>
        </div>
      </Show>
    </>
  );
}
