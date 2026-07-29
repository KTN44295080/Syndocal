import { For, Show } from "solid-js";
import { stageObjectClass } from "../stageObjects";
import type { StageObjectKind } from "../types";
import { StageObjectGlyph, stageObjectRenderDepth } from "./StageGlyphs";

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
  readOnly?: boolean;
  isDragging: (objectId: number) => boolean;
  onBeginDrag: (event: PointerEvent, objectId: number) => void;
  onBeginRotate: (event: PointerEvent, objectId: number) => void;
  onBeginResize: (event: PointerEvent, objectId: number, axis: "width" | "depth" | "both") => void;
};

export function MappingStageObjectsLayer(props: MappingStageObjectsLayerProps) {
  return (
    <g class="stageObjectLayer">
      <For each={props.objects}>
        {(object) => {
          const isScreen = () => object.kind === "Screen";
          const renderDepth = () => stageObjectRenderDepth(object.kind, object.depth);
          return (
            <g
              class={`${stageObjectClass(object)} ${object.selected ? "selected" : ""} ${props.isDragging(object.id) ? "dragging" : ""}`}
              transform={`translate(${object.x} ${object.z}) rotate(${object.rotationDeg})`}
              onPointerDown={(event) => {
                if (!props.readOnly) props.onBeginDrag(event, object.id);
              }}
            >
              <StageObjectGlyph
                kind={object.kind}
                width={object.width}
                depth={object.depth}
                color={object.color}
                label={object.label}
              />
              <Show when={object.selected && !props.readOnly}>
                <line
                  class="stageObjectHandleLine"
                  x1="0"
                  y1={-renderDepth() / 2}
                  x2="0"
                  y2={-renderDepth() / 2 - 5}
                />
                <circle
                  class="stageObjectRotateHandle"
                  cx="0"
                  cy={-renderDepth() / 2 - 5}
                  r="1.8"
                  onPointerDown={(event) => props.onBeginRotate(event, object.id)}
                >
                  <title data-no-localize>Rotate {object.label}</title>
                </circle>
                <circle
                  class="stageObjectResizeHandle width"
                  cx={object.width / 2 + 2.3}
                  cy="0"
                  r="1.7"
                  onPointerDown={(event) => props.onBeginResize(event, object.id, "width")}
                >
                  <title data-no-localize>Resize width of {object.label}</title>
                </circle>
                <Show when={!isScreen()}>
                  <circle
                    class="stageObjectResizeHandle depth"
                    cx="0"
                    cy={renderDepth() / 2 + 2.3}
                    r="1.7"
                    onPointerDown={(event) => props.onBeginResize(event, object.id, "depth")}
                  >
                    <title data-no-localize>Resize depth of {object.label}</title>
                  </circle>
                  <rect
                    class="stageObjectResizeHandle both"
                    x={object.width / 2 + 0.9}
                    y={renderDepth() / 2 + 0.9}
                    width="3"
                    height="3"
                    onPointerDown={(event) => props.onBeginResize(event, object.id, "both")}
                  >
                    <title data-no-localize>Resize {object.label}</title>
                  </rect>
                </Show>
              </Show>
            </g>
          );
        }}
      </For>
    </g>
  );
}
