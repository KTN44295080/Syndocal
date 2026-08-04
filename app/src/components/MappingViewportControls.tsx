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
  zoomMax: number;
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
  | "stageTool"
  | "canFitVisible"
  | "canFitSelection"
  | "zoomValue"
  | "zoomMax"
  | "canZoomOut"
  | "canZoomIn"
  | "showLabels"
  | "showBeams"
  | "showProjectors"
  | "showStageObjects"
  | "onFitVisible"
  | "onFitSelection"
  | "onZoomOut"
  | "onZoomIn"
  | "onZoomLevel"
  | "onShowLabels"
  | "onShowBeams"
  | "onShowProjectors"
  | "onShowStageObjects"
> & {
  selectedFixtureCount: number;
  canPickVisible: boolean;
  onStageTool: (tool: MappingStageTool) => void;
  onPickVisible: () => void;
  onClearSelection: () => void;
  onOpenMapping: () => void;
};

type ControlStageIconButtonProps = {
  icon: string;
  path: string;
  title: string;
  active?: boolean;
  pressed?: boolean;
  disabled?: boolean;
  layer?: "beams" | "labels" | "projectors" | "objects";
  onClick: () => void;
};

type MappingLayerToggleButtonProps = {
  layer: "labels" | "beams" | "geometry" | "projectors" | "objects" | "levels";
  label: string;
  title: string;
  pressed: boolean;
  onClick: () => void;
};

function ControlStageIconButton(props: ControlStageIconButtonProps) {
  return (
    <button
      type="button"
      class={props.active ? "active" : ""}
      data-control-stage-chrome-operation
      data-control-stage-tool-item
      data-control-stage-tool-icon={props.icon}
      data-control-stage-layer-toggle={props.layer}
      title={props.title}
      aria-label={props.title}
      aria-pressed={props.pressed}
      disabled={props.disabled}
      onClick={props.onClick}
    >
      <svg viewBox="0 0 16 16" aria-hidden="true">
        <path d={props.path} />
      </svg>
    </button>
  );
}

function MappingLayerToggleButton(props: MappingLayerToggleButtonProps) {
  return (
    <button
      type="button"
      class={props.pressed ? "active" : ""}
      data-mapping-layer-toggle={props.layer}
      title={props.title}
      aria-label={props.title}
      aria-pressed={props.pressed}
      onClick={props.onClick}
    >
      {props.label}
    </button>
  );
}

export function ControlStageToolbar(props: ControlStageToolbarProps) {
  return (
    <div class="controlStageToolbar" data-control-stage-tool-row aria-label="2D mapping viewport">
      <ControlStageIconButton
        icon="select"
        path="M3 2.5 12.5 8 8.4 9.2 6.2 13.5z"
        title="Select fixture or projection surface (S)"
        active={props.stageTool === "select"}
        pressed={props.stageTool === "select"}
        onClick={() => props.onStageTool("select")}
      />
      <ControlStageIconButton
        icon="pan"
        path="M5 8V4.5M7.5 7V3.5M10 7V4.5M12.5 8V6M5 7.5 3.5 7a1 1 0 0 0-1.2 1.4l2.5 4.2c.4.6 1 1 1.8 1h3.7c1.6 0 2.7-1.1 2.7-2.7V8"
        title="Pan stage view (H)"
        active={props.stageTool === "pan"}
        pressed={props.stageTool === "pan"}
        onClick={() => props.onStageTool("pan")}
      />
      <ControlStageIconButton
        icon="fit"
        path="M2.5 6V2.5H6M10 2.5h3.5V6M13.5 10v3.5H10M6 13.5H2.5V10"
        title="Fit visible stage items (F)"
        disabled={!props.canFitVisible}
        onClick={props.onFitVisible}
      />
      <ControlStageIconButton
        icon="fit-selection"
        path="M2.5 6V2.5H6M10 2.5h3.5V6M13.5 10v3.5H10M6 13.5H2.5V10M8 6.3a1.7 1.7 0 1 0 0 3.4 1.7 1.7 0 0 0 0-3.4Z"
        title="Fit selected items (Shift+F)"
        disabled={!props.canFitSelection}
        onClick={props.onFitSelection}
      />
      <ControlStageIconButton
        icon="pick-visible"
        path="M3 5V3h2M11 3h2v2M13 11v2h-2M5 13H3v-2M5.5 8h5M8 5.5v5"
        title="Pick Visible"
        disabled={!props.canPickVisible}
        onClick={props.onPickVisible}
      />
      <ControlStageIconButton
        icon="clear-pick"
        path="m4 4 8 8M12 4l-8 8"
        title="Clear Pick"
        disabled={props.selectedFixtureCount === 0}
        onClick={props.onClearSelection}
      />
      <ControlStageIconButton
        icon="zoom-out"
        path="M4 8h8"
        title="Zoom out (-)"
        disabled={!props.canZoomOut}
        onClick={props.onZoomOut}
      />
      <label
        class="controlStageZoom"
        data-control-stage-chrome-operation
        data-control-stage-tool-item
        title="Zoom level"
      >
        <input
          type="range"
          min="1"
          max={props.zoomMax}
          step="0.05"
          value={props.zoomValue}
          aria-label="Zoom level"
          onInput={(event) => props.onZoomLevel(Number(event.currentTarget.value))}
        />
      </label>
      <ControlStageIconButton
        icon="zoom-in"
        path="M4 8h8M8 4v8"
        title="Zoom in (+)"
        disabled={!props.canZoomIn}
        onClick={props.onZoomIn}
      />
      <ControlStageIconButton
        icon="beams"
        path="M8 2.2 3.2 13h9.6L8 2.2Zm0 3.2v5.4"
        title="Toggle beams (B)"
        active={props.showBeams}
        pressed={props.showBeams}
        layer="beams"
        onClick={() => props.onShowBeams(!props.showBeams)}
      />
      <ControlStageIconButton
        icon="labels"
        path="M3 3.2h10M8 3.2v9.6M5.5 12.8h5"
        title="Toggle labels (L)"
        active={props.showLabels}
        pressed={props.showLabels}
        layer="labels"
        onClick={() => props.onShowLabels(!props.showLabels)}
      />
      <ControlStageIconButton
        icon="projectors"
        path="M2.5 4.2h7.7v7.6H2.5zM10.2 6.2l3.3-1.5v6.6l-3.3-1.5"
        title="Toggle projection surfaces (V)"
        active={props.showProjectors}
        pressed={props.showProjectors}
        layer="projectors"
        onClick={() => props.onShowProjectors(!props.showProjectors)}
      />
      <ControlStageIconButton
        icon="objects"
        path="m8 2.5 5 2.7v5.6L8 13.5l-5-2.7V5.2L8 2.5Zm0 0v5.6m5-2.9L8 8.1 3 5.2"
        title="Toggle stage reference objects (O)"
        active={props.showStageObjects}
        pressed={props.showStageObjects}
        layer="objects"
        onClick={() => props.onShowStageObjects(!props.showStageObjects)}
      />
      <button
        type="button"
        class="controlStageMappingLink"
        data-control-stage-chrome-operation
        data-control-stage-mapping-link
        title="Edit in Stage"
        aria-label="Edit in Stage"
        onClick={props.onOpenMapping}
      >
        <svg viewBox="0 0 16 16" aria-hidden="true">
          <path d="M2.5 3.5h6v6h-6zM7.5 8.5l5.7-5.7M9.5 2.8h3.7v3.7" />
        </svg>
        <span>Stage</span>
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
          type="button"
          class="mappingViewportCompactButton mappingViewportIconButton"
          data-mapping-viewport-action="fit-visible"
          onClick={props.onFitVisible}
          disabled={!props.canFitVisible}
          title="Fit visible stage items (F)"
          aria-label="Fit visible stage items (F)"
        >
          <svg viewBox="0 0 16 16" aria-hidden="true">
            <path d="M2.5 6V2.5H6M10 2.5h3.5V6M13.5 10v3.5H10M6 13.5H2.5V10" />
          </svg>
        </button>
        <button
          type="button"
          class="mappingViewportCompactButton mappingViewportIconButton"
          data-mapping-viewport-action="fit-selection"
          onClick={props.onFitSelection}
          disabled={!props.canFitSelection}
          title="Fit selected items (Shift+F)"
          aria-label="Fit selected items (Shift+F)"
        >
          <svg viewBox="0 0 16 16" aria-hidden="true">
            <path d="M2.5 6V2.5H6M10 2.5h3.5V6M13.5 10v3.5H10M6 13.5H2.5V10M8 6.3a1.7 1.7 0 1 0 0 3.4 1.7 1.7 0 0 0 0-3.4Z" />
          </svg>
        </button>
        <button
          type="button"
          class="mappingViewportCompactButton"
          data-mapping-viewport-action="zoom-out"
          onClick={props.onZoomOut}
          disabled={!props.canZoomOut}
          title="Zoom out (-)"
          aria-label="Zoom out (-)"
        >
          -
        </button>
        <input
          class="mappingZoomSlider"
          data-mapping-viewport-action="zoom-slider"
          type="range"
          min="1"
          max={props.zoomMax}
          step="0.05"
          value={props.zoomValue}
          aria-label="Zoom level"
          title="Zoom level"
          onInput={(event) => props.onZoomLevel(Number(event.currentTarget.value))}
        />
        <strong data-mapping-zoom-readout>{props.zoomLabel}</strong>
        <button
          type="button"
          class="mappingViewportCompactButton"
          data-mapping-viewport-action="zoom-in"
          onClick={props.onZoomIn}
          disabled={!props.canZoomIn}
          title="Zoom in (+)"
          aria-label="Zoom in (+)"
        >
          +
        </button>
        <button
          type="button"
          class="mappingViewportCompactButton"
          data-mapping-viewport-action="reset"
          onClick={props.onResetZoom}
          disabled={!props.canResetZoom}
          title="Reset viewport (0)"
          aria-label="Reset viewport (0)"
        >
          0
        </button>
      </div>
      <details class="mappingStageSnapControls" data-mapping-snap-controls data-mapping-snap-control>
        <summary>
          <span>{props.snapEnabled ? "Snap" : "Snap Off"}</span>
          {" "}
          <strong data-no-localize>{props.snapEnabled ? `${props.snapSize}m` : ""}</strong>
        </summary>
        <div class="mappingSnapMenu">
          <button
            type="button"
            data-mapping-snap-option="off"
            class={!props.snapEnabled ? "active" : ""}
            onClick={props.onSnapOff}
          >
            Snap Off
          </button>
          <For each={mappingSnapPresets}>
            {(preset) => (
              <button
                type="button"
                data-mapping-snap-option={String(preset)}
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
              data-mapping-snap-size
              type="number"
              min="0.05"
              max="20"
              step="0.05"
              value={props.snapSize}
              onChange={(event) => props.onSnapSizeChange(Number(event.currentTarget.value))}
            />
          </label>
        </div>
      </details>
      <div class="mappingLayerToggles" data-mapping-layer-toggles>
        <MappingLayerToggleButton
          layer="labels"
          label="L"
          title="Toggle labels (L)"
          pressed={props.showLabels}
          onClick={() => props.onShowLabels(!props.showLabels)}
        />
        <MappingLayerToggleButton
          layer="beams"
          label="B"
          title="Toggle beams (B)"
          pressed={props.showBeams}
          onClick={() => props.onShowBeams(!props.showBeams)}
        />
        <MappingLayerToggleButton
          layer="geometry"
          label="G"
          title="Toggle GDTF geometry nodes (G)"
          pressed={props.showGeometry}
          onClick={() => props.onShowGeometry(!props.showGeometry)}
        />
        <MappingLayerToggleButton
          layer="projectors"
          label="V"
          title="Toggle projection surfaces (V)"
          pressed={props.showProjectors}
          onClick={() => props.onShowProjectors(!props.showProjectors)}
        />
        <MappingLayerToggleButton
          layer="objects"
          label="O"
          title="Toggle stage reference objects (O)"
          pressed={props.showStageObjects}
          onClick={() => props.onShowStageObjects(!props.showStageObjects)}
        />
        <MappingLayerToggleButton
          layer="levels"
          label="%"
          title="Toggle fixture level readouts (Shift+5)"
          pressed={props.showLevels}
          onClick={() => props.onShowLevels(!props.showLevels)}
        />
      </div>
    </>
  );
}
