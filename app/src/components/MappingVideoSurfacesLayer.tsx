import { For } from "solid-js";
import type { MappingStageTool } from "../mappingViewPresets";
import { StageProjectionSurfaceGlyph } from "./StageGlyphs";

export interface MappingVideoSurface2D {
  id: number;
  label: string;
  x: number;
  z: number;
  width: number;
  height: number;
  rotationDeg: number;
  opacity: number;
  active: boolean;
}

type MappingVideoSurfacesLayerProps = {
  surfaces: MappingVideoSurface2D[];
  selectedVideoOutputId: number | null;
  stageTool: MappingStageTool;
  worldPerCssPixel: number;
  onSelectOutput: (outputId: number) => void;
};

// Reference-only floor-plan representation of a projection surface. A vertical screen
// seen from above is a line segment with a facing tick, NOT a warped rectangle. All
// keystone/corner-pin/aspect/lens editing lives in Setup > Video's Projection Map.
export function MappingVideoSurfacesLayer(props: MappingVideoSurfacesLayerProps) {
  return (
      <For each={props.surfaces}>
        {(surface) => (
          <g
            class={[
              "stageVideoSurface2d",
              "reference",
              props.selectedVideoOutputId === surface.id ? "selected" : "",
              surface.active ? "" : "inactive",
            ].filter(Boolean).join(" ")}
            transform={`translate(${surface.x} ${surface.z}) rotate(${surface.rotationDeg})`}
            onPointerDown={(event) => {
              if (event.button === 1 || props.stageTool === "pan") {
                return;
              }
              event.stopPropagation();
              props.onSelectOutput(surface.id);
            }}
          >
            <StageProjectionSurfaceGlyph
              width={surface.width}
              label={surface.label}
              worldPerCssPixel={props.worldPerCssPixel}
            />
          </g>
        )}
      </For>
  );
}
