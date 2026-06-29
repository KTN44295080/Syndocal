import { For, Show, type JSX } from "solid-js";
import type { MappingStageTool } from "../mappingViewPresets";
import { stageViewBoxSize } from "../stageGeometry";

type MaybePromise = void | Promise<void>;

type StagePoint = {
  x: number;
  z: number;
};

type SvgPointerEvent = PointerEvent & { currentTarget: SVGSVGElement };
type SvgWheelEvent = WheelEvent & { currentTarget: SVGSVGElement };

export type MappingSnapLine = {
  axis: "x" | "z";
  svg: number;
};

export type MappingMarqueeBox = {
  x: number;
  z: number;
  width: number;
  height: number;
};

type MappingEditableStageShellProps = {
  svgRef: (element: SVGSVGElement) => void;
  dragging: boolean;
  stageTool: MappingStageTool;
  viewBox: string;
  stageOrigin: StagePoint;
  cursorPoint: StagePoint | null;
  cursorLabel: string;
  snapEnabled: boolean;
  snapLines: MappingSnapLine[];
  marqueeBox: MappingMarqueeBox | null;
  onPointerDown: (event: SvgPointerEvent) => MaybePromise;
  onPointerMove: (event: SvgPointerEvent) => void;
  onPointerUp: (event: SvgPointerEvent) => MaybePromise;
  onPointerLeave: () => void;
  onWheel: (event: SvgWheelEvent) => void;
  children: JSX.Element;
};

export function MappingEditableStageShell(props: MappingEditableStageShellProps) {
  const className = () => [
    "visualizerStage",
    "editableStage",
    props.dragging ? "dragging" : "",
    props.stageTool === "place"
      ? "placeMode"
      : props.stageTool === "rotate"
        ? "rotateMode"
        : props.stageTool === "pan"
          ? "panMode"
          : "selectMode",
  ].filter(Boolean).join(" ");

  return (
    <svg
      ref={props.svgRef}
      class={className()}
      viewBox={props.viewBox}
      role="img"
      aria-label="2D fixture and projector mapping stage"
      onPointerDown={(event) => void props.onPointerDown(event)}
      onPointerMove={(event) => props.onPointerMove(event)}
      onPointerUp={(event) => void props.onPointerUp(event)}
      onPointerCancel={(event) => void props.onPointerUp(event)}
      onPointerLeave={() => props.onPointerLeave()}
      onWheel={(event) => props.onWheel(event)}
    >
      <defs>
        <pattern id="stage-grid" width="10" height="10" patternUnits="userSpaceOnUse">
          <path d="M 10 0 L 0 0 0 10" />
        </pattern>
      </defs>
      <rect class="stageFloor" x="0" y="0" width={stageViewBoxSize} height={stageViewBoxSize} />
      <rect class="stageGrid" x="0" y="0" width={stageViewBoxSize} height={stageViewBoxSize} />
      <line class="stageAxis2d" x1={props.stageOrigin.x} y1="0" x2={props.stageOrigin.x} y2={stageViewBoxSize} />
      <line class="stageAxis2d" x1="0" y1={props.stageOrigin.z} x2={stageViewBoxSize} y2={props.stageOrigin.z} />
      <Show when={props.cursorPoint}>
        {(cursor) => (
          <g class="stageCursorGuide">
            <line x1={cursor().x} y1="0" x2={cursor().x} y2={stageViewBoxSize} />
            <line x1="0" y1={cursor().z} x2={stageViewBoxSize} y2={cursor().z} />
            <circle cx={cursor().x} cy={cursor().z} r="1.6">
              <title>{props.cursorLabel}</title>
            </circle>
          </g>
        )}
      </Show>
      <Show when={props.snapEnabled}>
        <g>
          <For each={props.snapLines}>
            {(line) => (
              <line
                class={`stageSnapLine ${line.axis}`}
                x1={line.axis === "x" ? line.svg : 0}
                y1={line.axis === "z" ? line.svg : 0}
                x2={line.axis === "x" ? line.svg : stageViewBoxSize}
                y2={line.axis === "z" ? line.svg : stageViewBoxSize}
              />
            )}
          </For>
        </g>
      </Show>
      <Show when={props.marqueeBox}>
        {(box) => (
          <rect
            class="mappingMarquee"
            x={box().x}
            y={box().z}
            width={box().width}
            height={box().height}
          />
        )}
      </Show>
      {props.children}
    </svg>
  );
}
