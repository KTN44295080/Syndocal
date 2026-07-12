import { For, Show } from "solid-js";
import type { MappingStageTool } from "../mappingViewPresets";
import type { VideoOutputMapping } from "../types";
import type { MappingVideoOutputCornerKey } from "../videoOutputMapping";

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

export interface MappingVideoSurfaceCorner {
  key: MappingVideoOutputCornerKey;
  label: string;
  x: number;
  z: number;
}

type MappingVideoSurfacesLayerProps = {
  surfaces: MappingVideoSurface2D[];
  selectedVideoOutputId: number | null;
  stageTool: MappingStageTool;
  isDragging: (outputId: number) => boolean;
  surfaceMapping: (surface: MappingVideoSurface2D) => VideoOutputMapping;
  cornerLocals: (surface: MappingVideoSurface2D, mapping: VideoOutputMapping) => MappingVideoSurfaceCorner[];
  cornerPointList: (surface: MappingVideoSurface2D, mapping: VideoOutputMapping) => string;
  onSelectOutput: (outputId: number) => void;
  onBeginDrag: (event: PointerEvent, outputId: number) => void;
  onBeginCornerDrag: (event: PointerEvent, outputId: number, corner: MappingVideoOutputCornerKey) => void;
  onBeginRotate: (event: PointerEvent, outputId: number) => void;
  onBeginScale: (event: PointerEvent, outputId: number) => void;
};

export function MappingVideoSurfacesLayer(props: MappingVideoSurfacesLayerProps) {
  return (
    <For each={props.surfaces}>
      {(surface) => {
        const surfaceMapping = () => props.surfaceMapping(surface);
        const corners = () => props.cornerLocals(surface, surfaceMapping());
        return (
          <g
            class={[
              "stageVideoSurface2d",
              props.selectedVideoOutputId === surface.id ? "selected" : "",
              props.isDragging(surface.id) ? "dragging" : "",
              surface.active ? "" : "inactive",
            ].filter(Boolean).join(" ")}
            transform={`translate(${surface.x} ${surface.z}) rotate(${surface.rotationDeg})`}
            opacity={Math.max(0.22, surface.opacity)}
            onPointerDown={(event) => {
              if (props.stageTool === "pan") {
                return;
              }
              event.stopPropagation();
              props.onSelectOutput(surface.id);
              props.onBeginDrag(event, surface.id);
            }}
          >
            <polygon
              class="stageVideoSurfaceShape"
              points={props.cornerPointList(surface, surfaceMapping())}
            >
              <title data-no-localize>{surface.label} / projection surface warp</title>
            </polygon>
            <line x1={-surface.width / 2} y1="0" x2={surface.width / 2} y2="0" />
            <line x1="0" y1={-surface.height / 2} x2="0" y2={surface.height / 2} />
            <text data-no-localize x={-surface.width / 2 + 1.2} y={-surface.height / 2 - 1.6}>
              {surface.label}
            </text>
            <Show when={props.selectedVideoOutputId === surface.id}>
              <For each={corners()}>
                {(corner) => (
                  <rect
                    class="stageVideoSurfaceCornerHandle"
                    x={corner.x - 1.25}
                    y={corner.z - 1.25}
                    width="2.5"
                    height="2.5"
                    onPointerDown={(event) => props.onBeginCornerDrag(event, surface.id, corner.key)}
                  >
                    <title>{corner.label} corner pin</title>
                  </rect>
                )}
              </For>
              <line
                class="stageVideoSurfaceHandleLine"
                x1="0"
                y1={-surface.height / 2}
                x2="0"
                y2={-surface.height / 2 - 5}
              />
              <circle
                class="stageVideoSurfaceRotateHandle"
                cx="0"
                cy={-surface.height / 2 - 5}
                r="1.8"
                onPointerDown={(event) => {
                  if (props.stageTool === "pan") {
                    return;
                  }
                  event.preventDefault();
                  event.stopPropagation();
                  props.onSelectOutput(surface.id);
                  props.onBeginRotate(event, surface.id);
                }}
              />
              <circle
                class="stageVideoSurfaceScaleHandle"
                cx={surface.width / 2 + 2.4}
                cy={surface.height / 2 + 2.4}
                r="1.8"
                onPointerDown={(event) => {
                  if (props.stageTool === "pan") {
                    return;
                  }
                  event.preventDefault();
                  event.stopPropagation();
                  props.onSelectOutput(surface.id);
                  props.onBeginScale(event, surface.id);
                }}
              />
            </Show>
          </g>
        );
      }}
    </For>
  );
}
