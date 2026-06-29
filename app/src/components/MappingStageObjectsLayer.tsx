import { For, Show } from "solid-js";
import { stageObjectClass } from "../stageObjects";
import type { StageObjectKind } from "../types";

export interface MappingStageObject2D {
  id: number;
  label: string;
  kind: StageObjectKind;
  x: number;
  z: number;
  width: number;
  depth: number;
  rotationDeg: number;
  color: string;
  selected: boolean;
}

type MappingStageObjectsLayerProps = {
  objects: MappingStageObject2D[];
  isDragging: (objectId: number) => boolean;
  onBeginDrag: (event: PointerEvent, objectId: number) => void;
  onBeginRotate: (event: PointerEvent, objectId: number) => void;
  onBeginResize: (event: PointerEvent, objectId: number, axis: "width" | "depth" | "both") => void;
};

export function MappingStageObjectsLayer(props: MappingStageObjectsLayerProps) {
  return (
    <g class="stageObjectLayer">
      <For each={props.objects}>
        {(object) => (
          <g
            class={`${stageObjectClass(object)} ${object.selected ? "selected" : ""} ${props.isDragging(object.id) ? "dragging" : ""}`}
            transform={`translate(${object.x} ${object.z}) rotate(${object.rotationDeg})`}
            onPointerDown={(event) => props.onBeginDrag(event, object.id)}
          >
            <rect
              x={-object.width / 2}
              y={-object.depth / 2}
              width={object.width}
              height={object.depth}
              fill={object.color}
            >
              <title>{object.label} / {object.kind}</title>
            </rect>
            <line x1={-object.width / 2} y1="0" x2={object.width / 2} y2="0" />
            <line x1="0" y1={-object.depth / 2} x2="0" y2={object.depth / 2} />
            <text x={-object.width / 2 + 1} y={-object.depth / 2 - 1}>
              {object.label}
            </text>
            <Show when={object.selected}>
              <line
                class="stageObjectHandleLine"
                x1="0"
                y1={-object.depth / 2}
                x2="0"
                y2={-object.depth / 2 - 5}
              />
              <circle
                class="stageObjectRotateHandle"
                cx="0"
                cy={-object.depth / 2 - 5}
                r="1.8"
                onPointerDown={(event) => props.onBeginRotate(event, object.id)}
              >
                <title>Rotate {object.label}</title>
              </circle>
              <circle
                class="stageObjectResizeHandle width"
                cx={object.width / 2 + 2.3}
                cy="0"
                r="1.7"
                onPointerDown={(event) => props.onBeginResize(event, object.id, "width")}
              >
                <title>Resize width of {object.label}</title>
              </circle>
              <circle
                class="stageObjectResizeHandle depth"
                cx="0"
                cy={object.depth / 2 + 2.3}
                r="1.7"
                onPointerDown={(event) => props.onBeginResize(event, object.id, "depth")}
              >
                <title>Resize depth of {object.label}</title>
              </circle>
              <rect
                class="stageObjectResizeHandle both"
                x={object.width / 2 + 0.9}
                y={object.depth / 2 + 0.9}
                width="3"
                height="3"
                onPointerDown={(event) => props.onBeginResize(event, object.id, "both")}
              >
                <title>Resize {object.label}</title>
              </rect>
            </Show>
          </g>
        )}
      </For>
    </g>
  );
}
