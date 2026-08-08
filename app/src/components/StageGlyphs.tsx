import { Show } from "solid-js";
import type { FixtureLiveColorSegment } from "../fixtureLiveColor";
import {
  mappingFixtureCellGap,
  mappingFixtureGridUnit,
  mappingFixtureSegmentCell,
  type MappingFixtureSegmentOrder,
  type MappingFixtureVisualKind,
} from "../fixtureVisuals";
import type { StageFixtureLabelLayout } from "../stageLabelLayout";
import { stageWorldPerCssPixel } from "../stageOverlayLayout";
import type { StageObjectKind } from "../types";

type StageFixtureGlyphProps = {
  visualKind: MappingFixtureVisualKind;
  width: number;
  height: number;
  color: string;
  shapeClass?: string;
  facingLineClass?: string;
  facingMarkClass?: string;
  hitTargetClass?: string;
  hitTargetRadius?: number;
  showFacingMark?: boolean;
  segments?: FixtureLiveColorSegment[];
  segmentColumns?: number;
  segmentRows?: number;
  segmentOrder?: MappingFixtureSegmentOrder;
  liveSegmentScreenScale?: number;
  title?: string;
};

export const stageFixtureHasFacingTick = (visualKind: MappingFixtureVisualKind) =>
  visualKind === "moving" || visualKind === "laser";

export const stageFixtureFacingTickLength = (width: number, height: number) =>
  Math.max(width, height) / 2 + 1.3;

export function StageFixtureGlyph(props: StageFixtureGlyphProps) {
  const shapeClass = () => props.shapeClass ?? "stageFixtureShape";
  const tickLength = () => stageFixtureFacingTickLength(props.width, props.height);
  const segments = () => props.segments?.length && props.segments.length > 1 ? props.segments : null;
  const segmentColumns = () => {
    const count = segments()?.length ?? 1;
    return Math.max(1, Math.min(count, Math.floor(props.segmentColumns ?? count)));
  };
  const segmentRows = () => {
    const count = segments()?.length ?? 1;
    return Math.max(Math.ceil(count / segmentColumns()), Math.floor(props.segmentRows ?? 1));
  };
  const segmentPitchX = () => props.width / segmentColumns();
  const segmentPitchY = () => props.height / segmentRows();
  const segmentGap = () => Math.min(
    mappingFixtureCellGap,
    Math.min(segmentPitchX(), segmentPitchY()) * 0.12,
  );
  const segmentCell = (index: number) => mappingFixtureSegmentCell(
    { columns: segmentColumns(), rows: segmentRows() },
    index,
    props.segmentOrder,
  );
  const segmentX = (index: number) => {
    const cell = segmentCell(index);
    return -props.width / 2 + segmentGap() / 2 + cell.column * segmentPitchX();
  };
  const segmentY = (index: number) => {
    const cell = segmentCell(index);
    return -props.height / 2 + segmentGap() / 2 + cell.row * segmentPitchY();
  };
  const segmentWidth = () => Math.max(0.1, segmentPitchX() - segmentGap());
  const segmentHeight = () => Math.max(0.1, segmentPitchY() - segmentGap());
  const liveSegmentScreenScale = () => props.liveSegmentScreenScale ?? 1;
  const fixtureCornerRadius = () => Math.min(0.8, props.height * 0.2);

  return (
    <>
      <Show when={props.hitTargetRadius}>
        {(radius) => (
          <rect
            class={props.hitTargetClass ?? "stageFixtureHitTarget"}
            x={-Math.max(radius(), props.width / 2 + 1.5)}
            y={-Math.max(radius(), props.height / 2 + 1.5)}
            width={Math.max(radius(), props.width / 2 + 1.5) * 2}
            height={Math.max(radius(), props.height / 2 + 1.5) * 2}
            rx="1"
            ry="1"
          />
        )}
      </Show>
      <Show
        when={segments()}
        fallback={
          <rect
            data-stage-fixture-shape
            data-stage-fixture-grid-world-size={mappingFixtureGridUnit}
            class={shapeClass()}
            x={-props.width / 2}
            y={-props.height / 2}
            width={props.width}
            height={props.height}
            rx={fixtureCornerRadius()}
            ry={fixtureCornerRadius()}
            fill={props.color}
          />
        }
      >
        {(liveSegments) => (
          <g
            data-stage-live-segment-screen-scale={liveSegmentScreenScale()}
            transform={`scale(${liveSegmentScreenScale()})`}
          >
            <rect
              data-stage-fixture-shape
              data-stage-fixture-outline
              data-stage-fixture-grid-world-size={mappingFixtureGridUnit}
              class={`${shapeClass()} stageFixtureSegmentOutline`}
              x={-props.width / 2}
              y={-props.height / 2}
              width={props.width}
              height={props.height}
              rx={fixtureCornerRadius()}
              ry={fixtureCornerRadius()}
              fill="none"
            />
            {liveSegments().map((segment, index) => (
              <rect
                data-stage-fixture-segment={index + 1}
                data-stage-fixture-segment-column={segmentCell(index).column + 1}
                data-stage-fixture-segment-row={segmentCell(index).row + 1}
                class="stageFixtureSegment"
                x={segmentX(index)}
                y={segmentY(index)}
                width={segmentWidth()}
                height={segmentHeight()}
                rx={fixtureCornerRadius()}
                ry={fixtureCornerRadius()}
                fill={segment.color}
                stroke="none"
              />
            ))}
          </g>
        )}
      </Show>
      <Show when={stageFixtureHasFacingTick(props.visualKind)}>
        <line
          class={props.facingLineClass ?? "stageFixtureCenterLine"}
          x1="0"
          y1="0"
          x2="0"
          y2={-tickLength()}
        />
        <Show when={props.showFacingMark !== false}>
          <circle
            class={props.facingMarkClass ?? "stageFixtureLaserMark"}
            cx="0"
            cy={-tickLength()}
            r="0.6"
          />
        </Show>
      </Show>
      <Show when={props.title}>
        {(title) => <title>{title()}</title>}
      </Show>
    </>
  );
}

type StageFixtureLabelProps = {
  layout: StageFixtureLabelLayout;
  worldPerCssPixel: number;
  class?: string;
};

export const stageFixtureLabelScreenFontSizePx = 11;

export const stageFixtureLabelWorldPerCssPixel = stageWorldPerCssPixel;

type StageScreenFixedLabelProps = {
  x: number;
  y: number;
  label: string;
  fullLabel?: string;
  worldPerCssPixel?: number;
  kind: "fixture" | "projection-surface" | "stage-object";
  fixtureId?: number;
  class?: string;
};

function StageScreenFixedLabel(props: StageScreenFixedLabelProps) {
  const screenFixed = () => props.worldPerCssPixel !== undefined;
  const className = () => [
    props.class,
    screenFixed() ? "stageScreenFixedLabel" : "",
  ].filter(Boolean).join(" ");
  return (
    <text
      data-no-localize
      data-stage-screen-fixed-label={screenFixed() ? props.kind : undefined}
      data-stage-fixture-label-id={props.fixtureId}
      data-stage-label-screen-font-size={screenFixed() ? stageFixtureLabelScreenFontSizePx : undefined}
      class={className() || undefined}
      x={props.x}
      y={props.y}
      style={screenFixed()
        ? { "font-size": `${stageFixtureLabelScreenFontSizePx * props.worldPerCssPixel!}px` }
        : undefined}
      ref={(element) => element.setAttribute("title", props.fullLabel ?? props.label)}
    >
      {props.label}
    </text>
  );
}

export function StageFixtureLabel(props: StageFixtureLabelProps) {
  return (
    <StageScreenFixedLabel
      x={props.layout.x}
      y={props.layout.z}
      label={props.layout.displayLabel}
      fullLabel={props.layout.fullLabel}
      worldPerCssPixel={props.worldPerCssPixel}
      kind="fixture"
      fixtureId={props.layout.fixtureId}
      class={props.class ?? "stageLabel"}
    />
  );
}

type StageProjectionSurfaceGlyphProps = {
  width: number;
  label: string;
  showLabel?: boolean;
  worldPerCssPixel?: number;
};

export const stageProjectionSurfaceHalfWidth = (width: number) => Math.max(1.2, width / 2);

export const stageProjectionSurfaceFacingLength = (width: number) =>
  Math.max(2, Math.min(3.2, width / 3));

export function StageProjectionSurfaceGlyph(props: StageProjectionSurfaceGlyphProps) {
  const halfWidth = () => stageProjectionSurfaceHalfWidth(props.width);
  const facingLength = () => stageProjectionSurfaceFacingLength(props.width);

  return (
    <>
      <line class="stageVideoSurfaceHit" x1={-halfWidth()} y1="0" x2={halfWidth()} y2="0" />
      <line class="stageVideoSurfaceLine" x1={-halfWidth()} y1="0" x2={halfWidth()} y2="0" />
      <line class="stageVideoSurfaceEndCap" x1={-halfWidth()} y1={-0.9} x2={-halfWidth()} y2={0.9} />
      <line class="stageVideoSurfaceEndCap" x1={halfWidth()} y1={-0.9} x2={halfWidth()} y2={0.9} />
      <line class="stageVideoSurfaceFacing" x1="0" y1="0" x2="0" y2={-facingLength()} />
      <circle class="stageVideoSurfaceFacingMark" cx="0" cy={-facingLength()} r="0.6" />
      <Show when={props.showLabel !== false}>
        <StageScreenFixedLabel
          x={-halfWidth() + 0.6}
          y={2.6}
          label={props.label}
          worldPerCssPixel={props.worldPerCssPixel}
          kind="projection-surface"
        />
      </Show>
      <title data-no-localize>{props.label}</title>
    </>
  );
}

type StageObjectGlyphProps = {
  kind: StageObjectKind;
  width: number;
  depth: number;
  color: string;
  label: string;
  showLabel?: boolean;
  worldPerCssPixel?: number;
};

export const stageObjectRenderDepth = (kind: StageObjectKind, depth: number) =>
  kind === "Screen" ? Math.min(depth, 1) : depth;

export function StageObjectGlyph(props: StageObjectGlyphProps) {
  const renderDepth = () => stageObjectRenderDepth(props.kind, props.depth);

  return (
    <>
      <rect
        x={-props.width / 2}
        y={-renderDepth() / 2}
        width={props.width}
        height={renderDepth()}
        fill={props.color}
      >
        <title data-no-localize>{props.label} / {props.kind}</title>
      </rect>
      <line x1={-props.width / 2} y1="0" x2={props.width / 2} y2="0" />
      <Show when={props.kind !== "Screen"}>
        <line x1="0" y1={-renderDepth() / 2} x2="0" y2={renderDepth() / 2} />
      </Show>
      <Show when={props.showLabel !== false}>
        <StageScreenFixedLabel
          x={-props.width / 2 + 1}
          y={-renderDepth() / 2 - 1}
          label={props.label}
          worldPerCssPixel={props.worldPerCssPixel}
          kind="stage-object"
        />
      </Show>
    </>
  );
}
