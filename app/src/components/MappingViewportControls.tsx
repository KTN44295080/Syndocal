import { For } from "solid-js";
import type { MappingStageTool } from "../mappingViewPresets";
import { mappingSnapPresets } from "../uiPresets";

type MappingViewportControlsProps = {
  stageTool: MappingStageTool;
  fixtureCount: number;
  outputCount: number;
  objectCount: number;
  cursorReadout: string | null;
  canFitVisible: boolean;
  canFitSelection: boolean;
  zoomLabel: string;
  canZoomOut: boolean;
  canZoomIn: boolean;
  canResetZoom: boolean;
  snapEnabled: boolean;
  snapSize: number;
  showLabels: boolean;
  showBeams: boolean;
  showGeometry: boolean;
  showProjectors: boolean;
  showStageObjects: boolean;
  showLevels: boolean;
  onFitVisible: () => void;
  onFitSelection: () => void;
  onZoomOut: () => void;
  onZoomIn: () => void;
  onResetZoom: () => void;
  onSnapOff: () => void;
  onSnapPreset: (size: number) => void;
  onSnapSizeChange: (size: number) => void;
  onShowLabels: (enabled: boolean) => void;
  onShowBeams: (enabled: boolean) => void;
  onShowGeometry: (enabled: boolean) => void;
  onShowProjectors: (enabled: boolean) => void;
  onShowStageObjects: (enabled: boolean) => void;
  onShowLevels: (enabled: boolean) => void;
};

export function MappingViewportControls(props: MappingViewportControlsProps) {
  return (
    <>
      <div class="mappingViewportControls">
        <span>Tool</span>
        <strong>{props.stageTool.toUpperCase()}</strong>
        <span>Fixtures</span>
        <strong>{props.fixtureCount}</strong>
        <span>Outputs</span>
        <strong>{props.outputCount}</strong>
        <span>Objects</span>
        <strong>{props.objectCount}</strong>
      </div>
      <div class={props.cursorReadout ? "mappingCursorReadout active" : "mappingCursorReadout"}>
        {props.cursorReadout ?? "Move over the stage"}
      </div>
      <div class="mappingViewControls" aria-label="2D mapping viewport">
        <span>View</span>
        <button onClick={props.onFitVisible} disabled={!props.canFitVisible} title="Fit visible stage items (F)">
          Fit All
        </button>
        <button onClick={props.onFitSelection} disabled={!props.canFitSelection} title="Fit selected items (Shift+F)">
          Fit Sel
        </button>
        <button onClick={props.onZoomOut} disabled={!props.canZoomOut} title="Zoom out (-)">
          Zoom -
        </button>
        <strong>{props.zoomLabel}</strong>
        <button onClick={props.onZoomIn} disabled={!props.canZoomIn} title="Zoom in (+)">
          Zoom +
        </button>
        <button onClick={props.onResetZoom} disabled={!props.canResetZoom} title="Reset viewport (0)">
          Reset
        </button>
      </div>
      <div class="mappingStageSnapControls">
        <button class={!props.snapEnabled ? "active" : ""} onClick={props.onSnapOff}>
          Snap Off
        </button>
        <For each={mappingSnapPresets}>
          {(preset) => (
            <button
              class={props.snapEnabled && Math.abs(props.snapSize - preset) < 0.001 ? "active" : ""}
              onClick={() => props.onSnapPreset(preset)}
            >
              {preset}m
            </button>
          )}
        </For>
        <label>
          Grid
          <input
            type="number"
            min="0.05"
            max="20"
            step="0.05"
            value={props.snapSize}
            onChange={(event) => props.onSnapSizeChange(Number(event.currentTarget.value))}
          />
        </label>
      </div>
      <div class="mappingLayerToggles">
        <label>
          <input
            type="checkbox"
            checked={props.showLabels}
            onChange={(event) => props.onShowLabels(event.currentTarget.checked)}
          />
          Labels
        </label>
        <label>
          <input
            type="checkbox"
            checked={props.showBeams}
            onChange={(event) => props.onShowBeams(event.currentTarget.checked)}
          />
          Beams
        </label>
        <label>
          <input
            type="checkbox"
            checked={props.showGeometry}
            onChange={(event) => props.onShowGeometry(event.currentTarget.checked)}
          />
          Geometry
        </label>
        <label>
          <input
            type="checkbox"
            checked={props.showProjectors}
            onChange={(event) => props.onShowProjectors(event.currentTarget.checked)}
          />
          Projectors
        </label>
        <label>
          <input
            type="checkbox"
            checked={props.showStageObjects}
            onChange={(event) => props.onShowStageObjects(event.currentTarget.checked)}
          />
          Objects
        </label>
        <label>
          <input
            type="checkbox"
            checked={props.showLevels}
            onChange={(event) => props.onShowLevels(event.currentTarget.checked)}
          />
          Levels
        </label>
      </div>
    </>
  );
}
