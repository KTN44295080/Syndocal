import type { MappingStageTool } from "../mappingViewPresets";

type MappingToolRailProps = {
  stageTool: MappingStageTool;
  helpOpen: boolean;
  onStageTool: (tool: MappingStageTool) => void;
  onToggleHelp: () => void;
};

export function MappingToolRail(props: MappingToolRailProps) {
  return (
    <div class="mappingToolRail" data-mapping-tool-rail aria-label="2D mapping tools">
      <button
        data-mapping-tool="select"
        class={props.stageTool === "select" ? "active" : ""}
        onClick={() => props.onStageTool("select")}
        title="Select fixture or projection surface (S)"
        aria-label="Select fixture or projection surface"
      >
        S
      </button>
      <button
        data-mapping-tool="place"
        class={props.stageTool === "place" ? "active" : ""}
        onClick={() => props.onStageTool("place")}
        title="Place selected fixture (P)"
        aria-label="Place selected fixture"
      >
        P
      </button>
      <button
        data-mapping-tool="rotate"
        class={props.stageTool === "rotate" ? "active" : ""}
        onClick={() => props.onStageTool("rotate")}
        title="Set selected fixture yaw (R)"
        aria-label="Set selected fixture yaw"
      >
        R
      </button>
      <button
        data-mapping-tool="pan"
        class={props.stageTool === "pan" ? "active" : ""}
        onClick={() => props.onStageTool("pan")}
        title="Pan stage view (H)"
        aria-label="Pan stage view"
      >
        H
      </button>
      <div class="mappingToolDivider" />
      <button
        data-mapping-tool="help"
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
