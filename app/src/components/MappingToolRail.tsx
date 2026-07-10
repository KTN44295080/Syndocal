import type { MappingStageTool } from "../mappingViewPresets";

type MappingToolRailProps = {
  stageTool: MappingStageTool;
  showLabels: boolean;
  showBeams: boolean;
  showGeometry: boolean;
  showProjectors: boolean;
  showStageObjects: boolean;
  showLevels: boolean;
  helpOpen: boolean;
  onStageTool: (tool: MappingStageTool) => void;
  onToggleLabels: () => void;
  onToggleBeams: () => void;
  onToggleGeometry: () => void;
  onToggleProjectors: () => void;
  onToggleStageObjects: () => void;
  onToggleLevels: () => void;
  onToggleHelp: () => void;
};

export function MappingToolRail(props: MappingToolRailProps) {
  return (
    <div class="mappingToolRail" aria-label="2D mapping tools">
      <button
        class={props.stageTool === "select" ? "active" : ""}
        onClick={() => props.onStageTool("select")}
        title="Select fixture or projection surface (S)"
        aria-label="Select fixture or projection surface"
      >
        S
      </button>
      <button
        class={props.stageTool === "place" ? "active" : ""}
        onClick={() => props.onStageTool("place")}
        title="Place selected fixture (P)"
        aria-label="Place selected fixture"
      >
        P
      </button>
      <button
        class={props.stageTool === "rotate" ? "active" : ""}
        onClick={() => props.onStageTool("rotate")}
        title="Set selected fixture yaw (R)"
        aria-label="Set selected fixture yaw"
      >
        R
      </button>
      <button
        class={props.stageTool === "pan" ? "active" : ""}
        onClick={() => props.onStageTool("pan")}
        title="Pan stage view (H)"
        aria-label="Pan stage view"
      >
        H
      </button>
      <div class="mappingToolDivider" />
      <button
        class={props.showLabels ? "active" : ""}
        onClick={props.onToggleLabels}
        title="Toggle labels (L)"
        aria-label="Toggle labels"
      >
        L
      </button>
      <button
        class={props.showBeams ? "active" : ""}
        onClick={props.onToggleBeams}
        title="Toggle beams (B)"
        aria-label="Toggle beams"
      >
        B
      </button>
      <button
        class={props.showGeometry ? "active" : ""}
        onClick={props.onToggleGeometry}
        title="Toggle GDTF geometry nodes (G)"
        aria-label="Toggle GDTF geometry nodes"
      >
        G
      </button>
      <button
        class={props.showProjectors ? "active" : ""}
        onClick={props.onToggleProjectors}
        title="Toggle projection surfaces (V)"
        aria-label="Toggle projection surfaces"
      >
        V
      </button>
      <button
        class={props.showStageObjects ? "active" : ""}
        onClick={props.onToggleStageObjects}
        title="Toggle stage reference objects (O)"
        aria-label="Toggle stage reference objects"
      >
        O
      </button>
      <button
        class={props.showLevels ? "active" : ""}
        onClick={props.onToggleLevels}
        title="Toggle fixture level readouts (Shift+5)"
        aria-label="Toggle fixture level readouts"
      >
        %
      </button>
      <div class="mappingToolDivider" />
      <button
        class={props.helpOpen ? "active" : ""}
        onClick={props.onToggleHelp}
        title="Keyboard shortcut help (?)"
        aria-label="Keyboard shortcut help"
        aria-expanded={props.helpOpen}
      >
        ?
      </button>
    </div>
  );
}
