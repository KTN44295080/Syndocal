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
  zoomValue: number;
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
  onZoomLevel: (zoom: number) => void;
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

type ControlStageToolbarProps = Pick<
  MappingViewportControlsProps,
  "stageTool" | "canFitVisible" | "zoomValue" | "onFitVisible" | "onZoomLevel"
> & {
  onStageTool: (tool: MappingStageTool) => void;
  onOpenMapping: () => void;
};

export function ControlStageToolbar(props: ControlStageToolbarProps) {
  return (
    <div class="controlStageToolbar" data-control-stage-tool-row aria-label="2D mapping viewport">
      <button
        type="button"
        class={props.stageTool === "select" ? "active" : ""}
        data-control-stage-chrome-operation
        data-control-stage-tool-item
        data-control-stage-tool-icon="select"
        title="Select fixture or projection surface (S)"
        aria-label="Select fixture or projection surface"
        aria-pressed={props.stageTool === "select"}
        onClick={() => props.onStageTool("select")}
      >
        <svg viewBox="0 0 16 16" aria-hidden="true">
          <path d="M3 2.5 12.5 8 8.4 9.2 6.2 13.5z" />
        </svg>
      </button>
      <button
        type="button"
        class={props.stageTool === "pan" ? "active" : ""}
        data-control-stage-chrome-operation
        data-control-stage-tool-item
        data-control-stage-tool-icon="pan"
        title="Pan stage view (H)"
        aria-label="Pan stage view"
        aria-pressed={props.stageTool === "pan"}
        onClick={() => props.onStageTool("pan")}
      >
        <svg viewBox="0 0 16 16" aria-hidden="true">
          <path d="M5 8V4.5M7.5 7V3.5M10 7V4.5M12.5 8V6M5 7.5 3.5 7a1 1 0 0 0-1.2 1.4l2.5 4.2c.4.6 1 1 1.8 1h3.7c1.6 0 2.7-1.1 2.7-2.7V8" />
        </svg>
      </button>
      <button
        type="button"
        data-control-stage-chrome-operation
        data-control-stage-tool-item
        data-control-stage-tool-icon="fit"
        title="Fit visible stage items (F)"
        aria-label="Fit visible stage items (F)"
        disabled={!props.canFitVisible}
        onClick={props.onFitVisible}
      >
        <svg viewBox="0 0 16 16" aria-hidden="true">
          <path d="M2.5 6V2.5H6M10 2.5h3.5V6M13.5 10v3.5H10M6 13.5H2.5V10" />
        </svg>
      </button>
      <label
        class="controlStageZoom"
        data-control-stage-chrome-operation
        data-control-stage-tool-item
        data-control-stage-tool-icon="zoom"
        title="Zoom level"
      >
        <svg viewBox="0 0 16 16" aria-hidden="true">
          <circle cx="7" cy="7" r="4" />
          <path d="m10 10 3.5 3.5" />
        </svg>
        <input
          type="range"
          min="1"
          max="4"
          step="0.05"
          value={props.zoomValue}
          aria-label="Zoom level"
          onInput={(event) => props.onZoomLevel(Number(event.currentTarget.value))}
        />
      </label>
      <button
        type="button"
        class="controlStageMappingLink"
        data-control-stage-chrome-operation
        data-control-stage-mapping-link
        onClick={props.onOpenMapping}
      >
        Edit in Mapping
      </button>
    </div>
  );
}

export function MappingViewportControls(props: MappingViewportControlsProps) {
  return (
    <>
      <div class="mappingViewportControls" data-mapping-viewport-readout>
        <span>Tool</span>
        <strong>{props.stageTool.toUpperCase()}</strong>
        <span>Fixtures</span>
        <strong>{props.fixtureCount}</strong>
        <span>Surfaces</span>
        <strong>{props.outputCount}</strong>
        <span>Objects</span>
        <strong>{props.objectCount}</strong>
      </div>
      <div class={props.cursorReadout ? "mappingCursorReadout active" : "mappingCursorReadout"}>
        {props.cursorReadout ?? "Move over the stage"}
      </div>
      <div class="mappingViewControls" data-mapping-view-controls aria-label="2D mapping viewport">
        <span>View</span>
        <button
          data-mapping-viewport-action="fit-visible"
          onClick={props.onFitVisible}
          disabled={!props.canFitVisible}
          title="Fit visible stage items (F)"
        >
          Fit All
        </button>
        <button
          data-mapping-viewport-action="fit-selection"
          onClick={props.onFitSelection}
          disabled={!props.canFitSelection}
          title="Fit selected items (Shift+F)"
        >
          Fit Sel
        </button>
        <button
          data-mapping-viewport-action="zoom-out"
          onClick={props.onZoomOut}
          disabled={!props.canZoomOut}
          title="Zoom out (-)"
        >
          Zoom -
        </button>
        <input
          class="mappingZoomSlider"
          data-mapping-viewport-action="zoom-slider"
          type="range"
          min="1"
          max="4"
          step="0.05"
          value={props.zoomValue}
          aria-label="Zoom level"
          title="Zoom level"
          onInput={(event) => props.onZoomLevel(Number(event.currentTarget.value))}
        />
        <strong data-mapping-zoom-readout>{props.zoomLabel}</strong>
        <button
          data-mapping-viewport-action="zoom-in"
          onClick={props.onZoomIn}
          disabled={!props.canZoomIn}
          title="Zoom in (+)"
        >
          Zoom +
        </button>
        <button
          data-mapping-viewport-action="reset"
          onClick={props.onResetZoom}
          disabled={!props.canResetZoom}
          title="Reset viewport (0)"
        >
          Reset
        </button>
      </div>
      <div class="mappingStageSnapControls" data-mapping-snap-controls>
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
      <div class="mappingLayerToggles" data-mapping-layer-toggles>
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
          Surfaces
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
