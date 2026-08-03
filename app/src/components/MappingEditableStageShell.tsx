import { For, Show, type JSX } from "solid-js";
import {
  mappingFixtureGridStageSize,
  mappingFixtureGridUnit,
  mappingFixtureWorldToSvgScale,
} from "../fixtureVisuals";
import type { MappingStageTool } from "../mappingViewPresets";
import { stageViewBoxSize, type StageWorldBounds } from "../stageGeometry";

type MaybePromise = void | Promise<void>;

type StagePoint = {
  x: number;
  z: number;
};

type SvgPointerEvent = PointerEvent & { currentTarget: SVGSVGElement };
type SvgWheelEvent = WheelEvent & { currentTarget: SVGSVGElement };
type SvgMouseEvent = MouseEvent & { currentTarget: SVGSVGElement };

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
  stageWorldBounds: StageWorldBounds;
  stageOrigin: StagePoint;
  snapEnabled: boolean;
  snapLines: MappingSnapLine[];
  marqueeBox: MappingMarqueeBox | null;
  onPointerDown: (event: SvgPointerEvent) => MaybePromise;
  onAuxClick: (event: SvgMouseEvent) => void;
  onPointerMove: (event: SvgPointerEvent) => void;
  onPointerUp: (event: SvgPointerEvent) => MaybePromise;
  onPointerLeave: () => void;
  onWheel: (event: SvgWheelEvent) => void;
  children: JSX.Element;
};

export function MappingEditableStageShell(props: MappingEditableStageShellProps) {
  const minorGridSize = () => mappingFixtureGridStageSize(props.stageWorldBounds);
  const majorGridSize = () => minorGridSize() * 5;
  const gridWorldToSvgScale = () => mappingFixtureWorldToSvgScale(props.stageWorldBounds);
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
      data-persistent-band-part="stage"
      viewBox={props.viewBox}
      role="img"
      aria-label="2D fixture and projection surface mapping stage"
      onPointerDown={(event) => void props.onPointerDown(event)}
      onAuxClick={(event) => props.onAuxClick(event)}
      onPointerMove={(event) => props.onPointerMove(event)}
      onPointerUp={(event) => void props.onPointerUp(event)}
      onPointerCancel={(event) => void props.onPointerUp(event)}
      onPointerLeave={() => props.onPointerLeave()}
      onWheel={(event) => props.onWheel(event)}
    >
      <defs>
        <pattern
          id="stage-grid-minor"
          data-mapping-grid-pattern="minor"
          data-grid-world-size={mappingFixtureGridUnit}
          data-world-to-svg-scale={gridWorldToSvgScale()}
          x={props.stageOrigin.x}
          y={props.stageOrigin.z}
          width={minorGridSize()}
          height={minorGridSize()}
          patternUnits="userSpaceOnUse"
        >
          <path
            class="stageGridMinor"
            d={`M ${minorGridSize()} 0 L 0 0 0 ${minorGridSize()}`}
          />
        </pattern>
        <pattern
          id="stage-grid"
          data-mapping-grid-pattern="major"
          data-grid-world-size={mappingFixtureGridUnit * 5}
          data-world-to-svg-scale={gridWorldToSvgScale()}
          x={props.stageOrigin.x}
          y={props.stageOrigin.z}
          width={majorGridSize()}
          height={majorGridSize()}
          patternUnits="userSpaceOnUse"
        >
          <rect width={majorGridSize()} height={majorGridSize()} fill="url(#stage-grid-minor)" />
          <path
            class="stageGridMajor"
            d={`M ${majorGridSize()} 0 L 0 0 0 ${majorGridSize()}`}
          />
        </pattern>
      </defs>
      <rect class="stageFloor" x="0" y="0" width={stageViewBoxSize} height={stageViewBoxSize} />
      <rect class="stageGrid" x="0" y="0" width={stageViewBoxSize} height={stageViewBoxSize} />
      <line
        class="stageAxis2d"
        data-mapping-origin-axis="x"
        x1={props.stageOrigin.x}
        y1="0"
        x2={props.stageOrigin.x}
        y2={stageViewBoxSize}
      />
      <line
        class="stageAxis2d"
        data-mapping-origin-axis="z"
        x1="0"
        y1={props.stageOrigin.z}
        x2={stageViewBoxSize}
        y2={props.stageOrigin.z}
      />
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
