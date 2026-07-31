import { Show } from "solid-js";
import type { FixtureLiveColorSegment } from "../fixtureLiveColor";
import {
  mappingFixtureCellGap,
  mappingFixtureGridUnit,
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
  const segmentPitch = () => {
    const count = segments()?.length ?? 1;
    return props.width / count;
  };
  const segmentGap = () => Math.min(mappingFixtureCellGap, segmentPitch() * 0.12);
  const segmentX = (index: number) => {
    return -props.width / 2 + segmentGap() / 2 + index * segmentPitch();
  };
  const segmentWidth = () => Math.max(0.1, segmentPitch() - segmentGap());
  const segmentHeight = () => Math.max(0.1, props.height - segmentGap());
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
          <>
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
                class="stageFixtureSegment"
                x={segmentX(index)}
                y={-segmentHeight() / 2}
                width={segmentWidth()}
                height={segmentHeight()}
                rx={fixtureCornerRadius()}
                ry={fixtureCornerRadius()}
                fill={segment.color}
                stroke="none"
              />
            ))}
          </>
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

export function StageFixtureLabel(props: StageFixtureLabelProps) {
  return (
    <text
      data-no-localize
      data-stage-fixture-label-id={props.layout.fixtureId}
      data-stage-label-screen-font-size={stageFixtureLabelScreenFontSizePx}
      class={props.class ?? "stageLabel"}
      x={props.layout.x}
      y={props.layout.z}
      style={{ "font-size": `${stageFixtureLabelScreenFontSizePx * props.worldPerCssPixel}px` }}
      ref={(element) => element.setAttribute("title", props.layout.fullLabel)}
    >
      {props.layout.displayLabel}
    </text>
  );
}

type StageProjectionSurfaceGlyphProps = {
  width: number;
  label: string;
  showLabel?: boolean;
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
        <text data-no-localize x={-halfWidth() + 0.6} y={2.6}>
          {props.label}
        </text>
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
        <text data-no-localize x={-props.width / 2 + 1} y={-renderDepth() / 2 - 1}>
          {props.label}
        </text>
      </Show>
    </>
  );
}
