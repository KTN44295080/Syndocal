import { createMemo, createSignal, For, Show } from "solid-js";
import type { FixtureLiveColorSegment } from "../fixtureLiveColor";
import type {
  MappingFixtureSegmentOrder,
  MappingFixtureVisualKind,
} from "../fixtureVisuals";
import { MAPPING_FIXTURE_DRAG_THRESHOLD_PX } from "../createMappingInteractionController";
import type { MappingStageTool } from "../mappingViewPresets";
import { planStageFixtureLabels, type StageLabelViewport } from "../stageLabelLayout";
import {
  stageFixtureYawHandleScreenSizePx,
  stageFixtureYawHandlePoint,
  stageFixtureYawHandleWorldRadius,
  stageOverlayHandleMinimumHitSizePx,
} from "../stageOverlayLayout";
import {
  StageFixtureGlyph,
  StageFixtureLabel,
} from "./StageGlyphs";

export interface MappingFixture2D {
  id: number;
  label: string;
  addressOrder: number;
  dmxLabel: string;
  groupLabel: string;
  typeKey: string;
  visualKind: MappingFixtureVisualKind;
  x: number;
  z: number;
  width: number;
  height: number;
  segmentColumns: number;
  segmentRows: number;
  segmentOrder: MappingFixtureSegmentOrder;
  yaw: number;
  intensity: number;
  color: string;
  liveColorApplied?: boolean;
  liveColorValueSource?: "preview" | "attribute";
  liveSegments?: FixtureLiveColorSegment[];
  inGroupFilter: boolean;
  highlighted: boolean;
  soloed: boolean;
  parked: boolean;
}

export type MappingPlacePreview2D = Pick<
  MappingFixture2D,
  "label" | "dmxLabel" | "groupLabel" | "visualKind" | "width" | "height" | "yaw" | "color"
> & { x: number; z: number };

type MappingFixturesLayerProps = {
  readOnly?: boolean;
  fixtureTransformsEditable?: boolean;
  stageTool: MappingStageTool;
  fixtures: MappingFixture2D[];
  labelFixtures?: MappingFixture2D[];
  selectedFixtureIds: Set<number>;
  selectedFixtureId: number | null;
  selectedGroupId: string | null;
  selectedTypeKey: string | null;
  showLabels: boolean;
  labelViewport: StageLabelViewport;
  worldPerCssPixel: number;
  showLevels: boolean;
  placePreview: MappingPlacePreview2D | null;
  isDragging: (fixtureId: number) => boolean;
  isYawDragging: (fixtureId: number) => boolean;
  onFixturePointerDown: (event: PointerEvent, fixtureId: number) => void;
  onBeginYawDrag: (event: PointerEvent, fixtureId: number) => void;
};

export function MappingFixturesLayer(props: MappingFixturesLayerProps) {
  const [hoveredFixtureId, setHoveredFixtureId] = createSignal<number | null>(null);
  const labelLayout = createMemo(() =>
    planStageFixtureLabels({
      fixtures: props.labelFixtures ?? props.fixtures,
      viewport: props.labelViewport,
      showLabels: props.showLabels,
      pickedFixtureIds: props.selectedFixtureIds,
      pickedFixtureId: props.selectedFixtureId,
      hoveredFixtureId: hoveredFixtureId(),
    }),
  );
  return (
    <>
      <For each={props.fixtures}>
        {(fixture) => {
          const selected = () => props.selectedFixtureIds.has(fixture.id);
          const className = () => [
            "stageFixture",
            "stageFixtureBlock",
            `kind-${fixture.visualKind}`,
            selected() ? "selected picked" : "",
            props.isDragging(fixture.id) ? "dragging" : "",
            props.selectedGroupId && fixture.inGroupFilter ? "groupMatch" : "",
            props.selectedTypeKey && fixture.typeKey === props.selectedTypeKey ? "typeMatch" : "",
            fixture.highlighted ? "highlighted" : "",
            fixture.soloed ? "soloed" : "",
            fixture.parked ? "parked" : "",
          ].filter(Boolean).join(" ");
          return (
            <>
              <g
                class={className()}
                data-stage-fixture-id={fixture.id}
                data-stage-fixture-hovered={hoveredFixtureId() === fixture.id ? "true" : "false"}
                data-live-color-applied={fixture.liveColorApplied ? "true" : "false"}
                data-live-color-source={fixture.liveColorValueSource ?? "none"}
                data-live-segment-count={fixture.liveSegments?.length ?? 1}
                data-live-segment-columns={fixture.segmentColumns}
                data-live-segment-rows={fixture.segmentRows}
                data-live-segment-order={fixture.segmentOrder}
                data-stage-fixture-drag-threshold={MAPPING_FIXTURE_DRAG_THRESHOLD_PX}
                transform={`translate(${fixture.x} ${fixture.z}) rotate(${fixture.yaw})`}
                onPointerDown={(event) => props.onFixturePointerDown(event, fixture.id)}
                onMouseOver={() => setHoveredFixtureId(fixture.id)}
                onMouseOut={(event) => {
                  const nextTarget = event.relatedTarget;
                  if (nextTarget instanceof Node && event.currentTarget.contains(nextTarget)) {
                    return;
                  }
                  setHoveredFixtureId((current) => current === fixture.id ? null : current);
                }}
              >
                <StageFixtureGlyph
                  visualKind={fixture.visualKind}
                  width={fixture.width}
                  height={fixture.height}
                  color={fixture.color}
                  segments={fixture.liveSegments}
                  segmentColumns={fixture.segmentColumns}
                  segmentRows={fixture.segmentRows}
                  segmentOrder={fixture.segmentOrder}
                  hitTargetRadius={6}
                  title={`${fixture.label} / ${fixture.dmxLabel} / ${fixture.groupLabel}`}
                />
              </g>
              <Show when={props.showLevels}>
                <text class={fixture.inGroupFilter ? "stageLevelLabel" : "stageLevelLabel muted"} x={fixture.x + 3.5} y={fixture.z + 5}>
                  {Math.round(fixture.intensity * 100)}%
                </text>
              </Show>
            </>
          );
        }}
      </For>
      <For each={props.fixtures}>
        {(fixture) => {
          const yawDragging = () => props.isYawDragging(fixture.id);
          const showYawHandle = () =>
            (props.fixtureTransformsEditable ?? !props.readOnly) &&
            (yawDragging() || (props.stageTool === "rotate" && props.selectedFixtureId === fixture.id));
          const yawHandlePoint = () => stageFixtureYawHandlePoint(fixture, props.worldPerCssPixel);
          const yawHandleRadius = () => stageFixtureYawHandleWorldRadius(props.worldPerCssPixel);
          return (
            <Show when={showYawHandle()}>
              <line
                class={yawDragging() ? "stageYawLine dragging" : "stageYawLine"}
                x1={fixture.x}
                y1={fixture.z}
                x2={yawHandlePoint().x}
                y2={yawHandlePoint().z}
              />
              <circle
                data-stage-overlay-handle="fixture-yaw"
                data-stage-overlay-handle-screen-size={stageFixtureYawHandleScreenSizePx}
                data-stage-overlay-handle-min-hit-size={stageOverlayHandleMinimumHitSizePx}
                class={yawDragging() ? "stageYawHandle dragging" : "stageYawHandle"}
                cx={yawHandlePoint().x}
                cy={yawHandlePoint().z}
                r={yawHandleRadius()}
                onPointerDown={(event) => props.onBeginYawDrag(event, fixture.id)}
              >
                <title data-no-localize>{fixture.label} yaw {fixture.yaw} deg</title>
              </circle>
            </Show>
          );
        }}
      </For>
      <For each={labelLayout().labels}>
        {(layout) => (
          <StageFixtureLabel
            layout={layout}
            worldPerCssPixel={props.worldPerCssPixel}
          />
        )}
      </For>
      <Show when={!props.readOnly && props.placePreview}>
        {(preview) => (
          <g
            class={`stagePlacePreview kind-${preview().visualKind}`}
            transform={`translate(${preview().x} ${preview().z}) rotate(${preview().yaw})`}
          >
            <StageFixtureGlyph
              visualKind={preview().visualKind}
              width={preview().width}
              height={preview().height}
              color={preview().color}
              shapeClass="stagePlacePreviewShape"
              facingLineClass="stagePlacePreviewCenterLine"
              showFacingMark={false}
              title={`Place ${preview().label} / ${preview().dmxLabel} / ${preview().groupLabel}`}
            />
            <text x={preview().width / 2 + 1.6} y={-preview().height / 2 - 1}>
              Place {preview().label}
            </text>
          </g>
        )}
      </Show>
    </>
  );
}
