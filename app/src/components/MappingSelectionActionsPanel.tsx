import { createSignal, Show } from "solid-js";

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
  onMatchOrientations: () => MaybePromise;
  onAimInstallationAxes: (target: { x: number; y: number; z: number }) => MaybePromise;
  onDuplicate: () => MaybePromise;
  onRemove: () => MaybePromise;
  onControlActive: () => void;
  onOpenSceneFx: () => void;
  onClearSelection: () => MaybePromise;
};

export function MappingSelectionActionsPanel(props: MappingSelectionActionsPanelProps) {
  const hasSelection = () => props.flagState.count > 0;
  const [targetX, setTargetX] = createSignal("0");
  const [targetY, setTargetY] = createSignal("0");
  const [targetZ, setTargetZ] = createSignal("0");
  const [orientationBusy, setOrientationBusy] = createSignal(false);
  const validTarget = () => [targetX(), targetY(), targetZ()].every(value => value.trim() !== "" && Number.isFinite(Number(value)));
  const orient = async (action: () => MaybePromise) => {
    if (orientationBusy()) return;
    setOrientationBusy(true);
    try { await action(); } finally { setOrientationBusy(false); }
  };

  return (
    <>
      <Show when={hasSelection()}>
        <div class="mappingTransformInspector" data-mapping-installation-orientation>
          <div class="mappingTransformTitle"><strong>設置姿勢</strong></div>
          <div class="mappingTransformActions" style={{ "grid-template-columns": "minmax(0, 1fr)" }}>
            <button type="button" disabled={orientationBusy() || props.selectedCount < 2}
              onClick={() => void orient(props.onMatchOrientations)}>基準灯体と同じ姿勢に揃える</button>
          </div>
          <p>選択中の基準灯体のYaw・Pitch・Rollを揃えます。</p>
          <div class="mappingTransformGrid">
            <label>指定点 X<input type="number" step="0.1" value={targetX()} onInput={event => setTargetX(event.currentTarget.value)} /></label>
            <label>指定点 Y<input type="number" step="0.1" value={targetY()} onInput={event => setTargetY(event.currentTarget.value)} /></label>
            <label>指定点 Z<input type="number" step="0.1" value={targetZ()} onInput={event => setTargetZ(event.currentTarget.value)} /></label>
          </div>
          <div class="mappingTransformActions" style={{ "grid-template-columns": "minmax(0, 1fr)" }}><button type="button" disabled={orientationBusy() || !validTarget()}
            onClick={() => void orient(() => props.onAimInstallationAxes({ x: Number(targetX()), y: Number(targetY()), z: Number(targetZ()) }))}>
            設置基準軸（+Z）を指定点へ向ける
          </button></div>
          <p>Rollは0、DMX値は変更しません。プロファイルの光軸やPan・Tiltが加わるため、各ビームの照射点とは異なる場合があります。</p>
        </div>
      </Show>
      <Show when={hasSelection()}>
        <div class="mappingTransformInspector" data-mapping-selection-flags>
          <div class="mappingTransformTitle">
            <strong>Selection Flags</strong>
            <span>{props.flagState.count} fixture(s) picked</span>
          </div>
          <div class="mappingFlagActions" data-mapping-fixture-flag-row>
            <button
              class={props.flagState.allHighlighted ? "active" : ""}
              onClick={() => void props.onSetFlag("highlight", !props.flagState.allHighlighted)}
              title="Toggle highlight for the picked fixtures (Q)"
            >
              {props.flagState.allHighlighted ? "Clear High" : "Highlight"}
            </button>
            <button
              class={props.flagState.allSoloed ? "active" : ""}
              onClick={() => void props.onSetFlag("solo", !props.flagState.allSoloed)}
              title="Toggle solo for the picked fixtures (W)"
            >
              {props.flagState.allSoloed ? "Clear Solo" : "Solo"}
            </button>
            <button
              class={props.flagState.allParked ? "active" : ""}
              onClick={() => void props.onSetFlag("park", !props.flagState.allParked)}
              title="Toggle park for the picked fixtures (E)"
            >
              {props.flagState.allParked ? "Clear Park" : "Park"}
            </button>
          </div>
          <div class="mappingEffectActions">
            <button
              class="primary"
              onClick={() => props.onOpenSceneFx()}
              title="Open the selected scene's FX surface"
            >
              Open Scene FX
            </button>
            <button
              data-mapping-selection-action="clear"
              onClick={() => void props.onClearSelection()}
            >
              Clear selection
            </button>
          </div>
          <div class="mappingNudgePanel">
            <span>Nudge {props.snapSize}m</span>
            <div class="mappingNudgeGrid">
              <button onClick={() => void props.onNudge(0, -props.snapSize)}>
                Up
              </button>
              <button onClick={() => void props.onNudge(-props.snapSize, 0)}>
                Left
              </button>
              <button onClick={() => void props.onNudge(props.snapSize, 0)}>
                Right
              </button>
              <button onClick={() => void props.onNudge(0, props.snapSize)}>
                Down
              </button>
            </div>
          </div>
        </div>
      </Show>
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
          </div>
        </div>
      </Show>
    </>
  );
}
