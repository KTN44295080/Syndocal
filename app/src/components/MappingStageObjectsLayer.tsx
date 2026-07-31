import { For, Show } from "solid-js";
import { stageObjectClass } from "../stageObjects";
import {
  stageOverlayHandleMinimumHitSizePx,
  stageOverlayHandleScreenSizePx,
  stageOverlayHandleWorldOffsetFromEdge,
  stageOverlayHandleWorldRadius,
  stageOverlayHandleWorldSize,
} from "../stageOverlayLayout";
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
};

type MappingStageObjectHandlesLayerProps = {
  objects: MappingStageObject2D[];
  readOnly?: boolean;
  worldPerCssPixel: number;
  onBeginRotate: (event: PointerEvent, objectId: number) => void;
  onBeginResize: (event: PointerEvent, objectId: number, axis: "width" | "depth" | "both") => void;
};

export function MappingStageObjectsLayer(props: MappingStageObjectsLayerProps) {
  return (
    <g class="stageObjectLayer">
      <For each={props.objects}>
        {(object) => {
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
            </g>
          );
        }}
      </For>
    </g>
  );
}

export function MappingStageObjectHandlesLayer(props: MappingStageObjectHandlesLayerProps) {
  return (
    <g class="stageObjectHandlesLayer">
      <For each={props.objects}>
        {(object) => {
          const isScreen = () => object.kind === "Screen";
          const renderDepth = () => stageObjectRenderDepth(object.kind, object.depth);
          const handleRadius = () => stageOverlayHandleWorldRadius(props.worldPerCssPixel);
          const handleSize = () => stageOverlayHandleWorldSize(props.worldPerCssPixel);
          const handleOffset = () => stageOverlayHandleWorldOffsetFromEdge(props.worldPerCssPixel);
          const rotateHandleY = () => -renderDepth() / 2 - handleOffset();
          const widthHandleX = () => object.width / 2 + handleOffset();
          const depthHandleY = () => renderDepth() / 2 + handleOffset();
          return (
            <Show when={object.selected && !props.readOnly}>
              <g transform={`translate(${object.x} ${object.z}) rotate(${object.rotationDeg})`}>
                <line
                  class="stageObjectHandleLine"
                  x1="0"
                  y1={-renderDepth() / 2}
                  x2="0"
                  y2={rotateHandleY()}
                />
                <circle
                  data-stage-overlay-handle="stage-object-rotate"
                  data-stage-overlay-handle-screen-size={stageOverlayHandleScreenSizePx}
                  data-stage-overlay-handle-min-hit-size={stageOverlayHandleMinimumHitSizePx}
                  class="stageObjectRotateHandle"
                  cx="0"
                  cy={rotateHandleY()}
                  r={handleRadius()}
                  onPointerDown={(event) => props.onBeginRotate(event, object.id)}
                >
                  <title data-no-localize>Rotate {object.label}</title>
                </circle>
                <circle
                  data-stage-overlay-handle="stage-object-resize-width"
                  data-stage-overlay-handle-screen-size={stageOverlayHandleScreenSizePx}
                  data-stage-overlay-handle-min-hit-size={stageOverlayHandleMinimumHitSizePx}
                  class="stageObjectResizeHandle width"
                  cx={widthHandleX()}
                  cy="0"
                  r={handleRadius()}
                  onPointerDown={(event) => props.onBeginResize(event, object.id, "width")}
                >
                  <title data-no-localize>Resize width of {object.label}</title>
                </circle>
                <Show when={!isScreen()}>
                  <circle
                    data-stage-overlay-handle="stage-object-resize-depth"
                    data-stage-overlay-handle-screen-size={stageOverlayHandleScreenSizePx}
                    data-stage-overlay-handle-min-hit-size={stageOverlayHandleMinimumHitSizePx}
                    class="stageObjectResizeHandle depth"
                    cx="0"
                    cy={depthHandleY()}
                    r={handleRadius()}
                    onPointerDown={(event) => props.onBeginResize(event, object.id, "depth")}
                  >
                    <title data-no-localize>Resize depth of {object.label}</title>
                  </circle>
                  <rect
                    data-stage-overlay-handle="stage-object-resize-both"
                    data-stage-overlay-handle-screen-size={stageOverlayHandleScreenSizePx}
                    data-stage-overlay-handle-min-hit-size={stageOverlayHandleMinimumHitSizePx}
                    class="stageObjectResizeHandle both"
                    x={widthHandleX() - handleSize() / 2}
                    y={depthHandleY() - handleSize() / 2}
                    width={handleSize()}
                    height={handleSize()}
                    onPointerDown={(event) => props.onBeginResize(event, object.id, "both")}
                  >
                    <title data-no-localize>Resize {object.label}</title>
                  </rect>
                </Show>
              </g>
            </Show>
          );
        }}
      </For>
    </g>
  );
}
