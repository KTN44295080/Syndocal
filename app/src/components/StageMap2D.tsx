import { For, Show } from "solid-js";
import { stageViewBoxSize } from "../stageGeometry";
import type { GeometryModelMeshKind, VideoOutputMapping } from "../types";

const clampRange = (value: number, min: number, max: number) => Math.min(max, Math.max(min, value));

const visualizerGeometryKindClass = (kind: string) => {
  const normalized = kind.toLowerCase();
  if (normalized.includes("beam")) return "beam";
  if (normalized.includes("axis")) return "axis";
  return "body";
};

type MappingFixtureVisualKind = "point" | "moving" | "bar" | "panel" | "laser" | "par";
export type StageVideoSurfaceCornerKey = "topLeft" | "topRight" | "bottomRight" | "bottomLeft";

export interface VisualizerFixture {
  id: number;
  label: string;
  dmxLabel: string;
  groupLabel: string;
  mapLabel: string;
  typeKey: string;
  visualKind: MappingFixtureVisualKind;
  x: number;
  z: number;
  width: number;
  height: number;
  inGroupFilter: boolean;
  yaw: number;
  yawHandleX: number;
  yawHandleZ: number;
  beamPoints: string;
  intensity: number;
  color: string;
  highlighted: boolean;
  soloed: boolean;
  parked: boolean;
}

export type MappingPlacePreview = Pick<
  VisualizerFixture,
  "label" | "dmxLabel" | "groupLabel" | "visualKind" | "width" | "height" | "yaw" | "color"
> & { x: number; z: number };

interface MappingSelectionBounds {
  x: number;
  z: number;
  width: number;
  height: number;
  label: string;
}

export interface VisualizerFixtureGeometry2d {
  key: string;
  fixtureId: number;
  name: string;
  kind: string;
  meshKind: GeometryModelMeshKind;
  modelLabel: string;
  mappedChannelCount: number;
  x: number;
  z: number;
  inGroupFilter: boolean;
  selected: boolean;
}

export interface VisualizerFixtureModel2d {
  key: string;
  fixtureId: number;
  geometryName: string;
  meshKind: GeometryModelMeshKind;
  modelLabel: string;
  footprintPoints: string | null;
  footprintRadius: number;
  x: number;
  z: number;
  inGroupFilter: boolean;
  selected: boolean;
}

export interface VisualizerVideoSurface2d {
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

interface StageSnapGridLine {
  key: string;
  axis: "x" | "z";
  position: number;
}

interface StageAxisTick {
  key: string;
  axis: "x" | "z";
  value: number;
  position: number;
  label: string;
}

export interface StageSurfaceCornerLocal {
  key: StageVideoSurfaceCornerKey;
  label: string;
  x: number;
  z: number;
}

interface StageFixtureDragRef {
  fixture_id: number;
  mode: string;
  fixture_ids?: number[];
}

interface StageMap2DProps {
  mappingStageViewBox: () => string;
  mappingStageTool: () => string;
  stageFixtureDrag: () => StageFixtureDragRef | null;
  stageVideoSurfaceDrag: () => { output_id: number } | null;
  mappingStagePanDrag: () => object | null;
  mappingStageCursorGuide: () => { x: number; z: number } | null;
  mappingStageCursorLabel: () => string;
  stageSnapGridLines: () => StageSnapGridLine[];
  stageOrigin2d: () => { x: number; z: number };
  stageAxisTicks: () => StageAxisTick[];
  mappingSelectionBounds: () => MappingSelectionBounds | null;
  mappingMarqueeBox: () => { x: number; z: number; width: number; height: number } | null;
  mappingShowProjectors: () => boolean;
  visualizerVideoSurfaces2d: () => VisualizerVideoSurface2d[];
  stageVideoSurfaceMapping: (id: number) => VideoOutputMapping;
  stageVideoSurfaceCornerLocals: (surface: VisualizerVideoSurface2d, mapping: VideoOutputMapping) => StageSurfaceCornerLocal[];
  stageVideoSurfaceCornerPointList: (surface: VisualizerVideoSurface2d, mapping: VideoOutputMapping) => string;
  selectedVideoOutputId: () => number | null;
  beginStageVideoSurfaceDrag: (event: PointerEvent, id: number) => void;
  dragStageVideoSurface: (event: PointerEvent, id: number) => void;
  endStageVideoSurfaceDrag: (event: PointerEvent, id: number) => Promise<void>;
  beginStageVideoSurfaceCornerDrag: (event: PointerEvent, id: number, key: StageVideoSurfaceCornerKey) => void;
  dragStageVideoSurfaceCorner: (event: PointerEvent, id: number, key: StageVideoSurfaceCornerKey) => void;
  endStageVideoSurfaceCornerDrag: (event: PointerEvent, id: number, key: StageVideoSurfaceCornerKey) => Promise<void>;
  beginStageVideoSurfaceRotationDrag: (event: PointerEvent, id: number) => void;
  dragStageVideoSurfaceRotation: (event: PointerEvent, id: number) => void;
  endStageVideoSurfaceRotationDrag: (event: PointerEvent, id: number) => Promise<void>;
  beginStageVideoSurfaceScaleDrag: (event: PointerEvent, id: number) => void;
  dragStageVideoSurfaceScale: (event: PointerEvent, id: number) => void;
  endStageVideoSurfaceScaleDrag: (event: PointerEvent, id: number) => Promise<void>;
  mappingShowLabels: () => boolean;
  mappingShowGeometry: () => boolean;
  visualizerFixtureModels2d: () => VisualizerFixtureModel2d[];
  visualizerFixtureGeometries2d: () => VisualizerFixtureGeometry2d[];
  mappingShowFixtures: () => boolean;
  visualizerFixtures: () => VisualizerFixture[];
  mappingShowBeams: () => boolean;
  selectedFixtureId: () => number | null;
  selectedFixtureGroupFilter: () => string | null;
  selectedFixtureTypeFilter: () => string | null;
  mappingFixtureSearch: () => string;
  selectedMappingFixtureIdSet: () => Set<number>;
  beginStageFixtureYawDrag: (event: PointerEvent, id: number) => void;
  dragStageFixtureYaw: (event: PointerEvent, id: number) => void;
  endStageFixtureYawDrag: (event: PointerEvent, id: number) => Promise<void>;
  beginStageFixtureDrag: (event: PointerEvent, id: number) => void;
  dragStageFixture: (event: PointerEvent, id: number) => void;
  endStageFixtureDrag: (event: PointerEvent, id: number) => Promise<void>;
  openControlForFixtureId: (id: number) => void;
  mappingShowLevels: () => boolean;
  mappingPlacePreview: () => MappingPlacePreview | null;
  handleMappingStagePointerDown: (event: PointerEvent) => void;
  handleMappingStagePointerMove: (event: PointerEvent) => void;
  handleMappingStagePointerUp: (event: PointerEvent) => void;
  handleMappingStagePointerLeave: (event: PointerEvent) => void;
}

export const StageMap2D = (props: StageMap2DProps) => (
  <svg
    class={[
      "visualizerStage",
      "editableStage",
      props.mappingStageTool() === "place"
        ? "placeMode"
        : props.mappingStageTool() === "rotate"
          ? "rotateMode"
          : props.mappingStageTool() === "pan"
            ? "panMode"
            : "selectMode",
      props.stageFixtureDrag() || props.stageVideoSurfaceDrag() || props.mappingStagePanDrag() ? "dragging" : "",
    ].filter(Boolean).join(" ")}
    viewBox={props.mappingStageViewBox()}
    role="img"
    aria-label="2D fixture and projection surface mapping stage"
    onPointerDown={props.handleMappingStagePointerDown}
    onPointerMove={props.handleMappingStagePointerMove}
    onPointerUp={props.handleMappingStagePointerUp}
    onPointerCancel={props.handleMappingStagePointerUp}
    onPointerLeave={props.handleMappingStagePointerLeave}
  >
    <defs>
      <pattern id="stage-grid" width="10" height="10" patternUnits="userSpaceOnUse">
        <path d="M 10 0 L 0 0 0 10" />
      </pattern>
    </defs>
    <rect class="stageFloor" x="0" y="0" width={stageViewBoxSize} height={stageViewBoxSize} />
    <rect class="stageGrid" x="0" y="0" width={stageViewBoxSize} height={stageViewBoxSize} />
    <Show when={props.mappingStageCursorGuide()}>
      {(cursor) => (
        <g class="stageCursorGuide">
          <line x1={cursor().x} y1="0" x2={cursor().x} y2={stageViewBoxSize} />
          <line x1="0" y1={cursor().z} x2={stageViewBoxSize} y2={cursor().z} />
          <circle cx={cursor().x} cy={cursor().z} r="1.6" />
          <title>{props.mappingStageCursorLabel()}</title>
        </g>
      )}
    </Show>
    <For each={props.stageSnapGridLines()}>
      {(line) => (
        <line
          class={`stageSnapLine ${line.axis}`}
          x1={line.axis === "x" ? line.position : 0}
          y1={line.axis === "x" ? 0 : line.position}
          x2={line.axis === "x" ? line.position : stageViewBoxSize}
          y2={line.axis === "x" ? stageViewBoxSize : line.position}
        />
      )}
    </For>
    <line class="stageAxis2d" x1={props.stageOrigin2d().x} y1="0" x2={props.stageOrigin2d().x} y2={stageViewBoxSize} />
    <line class="stageAxis2d" x1="0" y1={props.stageOrigin2d().z} x2={stageViewBoxSize} y2={props.stageOrigin2d().z} />
    <For each={props.stageAxisTicks()}>
      {(tick) => (
        <g class={`stageAxisTick ${tick.axis}`}>
          <Show
            when={tick.axis === "x"}
            fallback={
              <>
                <line
                  x1={Math.max(0, props.stageOrigin2d().x - 0.8)}
                  y1={tick.position}
                  x2={Math.min(stageViewBoxSize, props.stageOrigin2d().x + 0.8)}
                  y2={tick.position}
                />
                <text
                  x={Math.min(stageViewBoxSize - 2, props.stageOrigin2d().x + 1.5)}
                  y={clampRange(tick.position + 0.8, 3, stageViewBoxSize - 1.5)}
                >
                  {tick.label}
                </text>
              </>
            }
          >
            <line
              x1={tick.position}
              y1={Math.max(0, props.stageOrigin2d().z - 0.8)}
              x2={tick.position}
              y2={Math.min(stageViewBoxSize, props.stageOrigin2d().z + 0.8)}
            />
            <text
              x={clampRange(tick.position - 2.6, 1.5, stageViewBoxSize - 7)}
              y={Math.max(3, props.stageOrigin2d().z - 1.8)}
            >
              {tick.label}
            </text>
          </Show>
        </g>
      )}
    </For>
    <text class="stageAxisLabel x" x={stageViewBoxSize - 3.5} y={Math.max(3, props.stageOrigin2d().z - 2.2)}>X</text>
    <text class="stageAxisLabel z" x={Math.min(stageViewBoxSize - 4, props.stageOrigin2d().x + 1.8)} y="4">Z</text>
    <Show when={props.mappingSelectionBounds()}>
      {(bounds) => (
        <g class="stageSelectionBounds">
          <rect
            x={bounds().x}
            y={bounds().z}
            width={bounds().width}
            height={bounds().height}
          />
          <text
            x={clampRange(bounds().x + 0.8, 1.5, stageViewBoxSize - 24)}
            y={clampRange(bounds().z - 1.3, 3, stageViewBoxSize - 1.5)}
          >
            {bounds().label}
          </text>
        </g>
      )}
    </Show>
    <Show when={props.mappingMarqueeBox()}>
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
    <Show when={props.mappingShowProjectors()}>
      <For each={props.visualizerVideoSurfaces2d()}>
        {(surface) => {
          const mapping = () => props.stageVideoSurfaceMapping(surface.id);
          const corners = () => props.stageVideoSurfaceCornerLocals(surface, mapping());
          return (
            <g
              class={[
                "stageVideoSurface2d",
                surface.active ? "" : "inactive",
                surface.id === props.selectedVideoOutputId() ? "selected" : "",
                props.stageVideoSurfaceDrag()?.output_id === surface.id ? "dragging" : "",
              ].filter(Boolean).join(" ")}
              transform={`translate(${surface.x} ${surface.z}) rotate(${surface.rotationDeg})`}
              opacity={Math.max(0.18, surface.opacity * 0.72)}
              onPointerDown={(event) => props.beginStageVideoSurfaceDrag(event, surface.id)}
              onPointerMove={(event) => props.dragStageVideoSurface(event, surface.id)}
              onPointerUp={(event) => void props.endStageVideoSurfaceDrag(event, surface.id)}
              onPointerCancel={(event) => void props.endStageVideoSurfaceDrag(event, surface.id)}
            >
              <polygon class="stageVideoSurfaceShape" points={props.stageVideoSurfaceCornerPointList(surface, mapping())}>
                <title data-no-localize>{surface.label} / Projection surface</title>
              </polygon>
              <line x1={0} y1={-surface.height / 2} x2={0} y2={surface.height / 2} />
              <Show when={surface.id === props.selectedVideoOutputId()}>
                <For each={corners()}>
                  {(corner) => (
                    <rect
                      class="stageVideoSurfaceCornerHandle"
                      x={corner.x - 1.2}
                      y={corner.z - 1.2}
                      width="2.4"
                      height="2.4"
                      onPointerDown={(event) => props.beginStageVideoSurfaceCornerDrag(event, surface.id, corner.key)}
                      onPointerMove={(event) => props.dragStageVideoSurfaceCorner(event, surface.id, corner.key)}
                      onPointerUp={(event) => void props.endStageVideoSurfaceCornerDrag(event, surface.id, corner.key)}
                      onPointerCancel={(event) => void props.endStageVideoSurfaceCornerDrag(event, surface.id, corner.key)}
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
                  r="1.45"
                  onPointerDown={(event) => props.beginStageVideoSurfaceRotationDrag(event, surface.id)}
                  onPointerMove={(event) => props.dragStageVideoSurfaceRotation(event, surface.id)}
                  onPointerUp={(event) => void props.endStageVideoSurfaceRotationDrag(event, surface.id)}
                  onPointerCancel={(event) => void props.endStageVideoSurfaceRotationDrag(event, surface.id)}
                >
                  <title data-no-localize>Rotate {surface.label}</title>
                </circle>
                <rect
                  class="stageVideoSurfaceScaleHandle"
                  x={surface.width / 2 - 1.35}
                  y={surface.height / 2 - 1.35}
                  width="2.7"
                  height="2.7"
                  onPointerDown={(event) => props.beginStageVideoSurfaceScaleDrag(event, surface.id)}
                  onPointerMove={(event) => props.dragStageVideoSurfaceScale(event, surface.id)}
                  onPointerUp={(event) => void props.endStageVideoSurfaceScaleDrag(event, surface.id)}
                  onPointerCancel={(event) => void props.endStageVideoSurfaceScaleDrag(event, surface.id)}
                >
                  <title data-no-localize>Resize {surface.label}</title>
                </rect>
              </Show>
              <Show when={props.mappingShowLabels()}>
                <text x={surface.width / 2 + 1.5} y={-surface.height / 2 - 1}>
                  <tspan data-no-localize>{surface.label}</tspan>
                </text>
              </Show>
            </g>
          );
        }}
      </For>
    </Show>
    <Show when={props.mappingShowGeometry()}>
      <For each={props.visualizerFixtureModels2d()}>
        {(model) => (
          <g
            class={[
              "stageGeometryNode",
              "stageGeometryModel",
              `mesh-${model.meshKind.toLowerCase()}`,
              model.inGroupFilter ? "" : "muted",
              model.selected ? "selected" : "",
            ].filter(Boolean).join(" ")}
          >
            <Show when={model.footprintPoints}>
              {(points) => (
                <polygon class="stageGeometryFootprint" points={points()}>
                  <title>
                    {model.geometryName} / {model.meshKind}
                    {model.modelLabel ? ` / ${model.modelLabel}` : ""}
                  </title>
                </polygon>
              )}
            </Show>
            <Show when={model.footprintRadius > 0}>
              <circle
                class="stageGeometryFootprint"
                cx={model.x}
                cy={model.z}
                r={model.footprintRadius}
              >
                <title>
                  {model.geometryName} / {model.meshKind}
                  {model.modelLabel ? ` / ${model.modelLabel}` : ""}
                </title>
              </circle>
            </Show>
          </g>
        )}
      </For>
      <For each={props.visualizerFixtureGeometries2d()}>
        {(geometry) => (
          <g
            class={[
              "stageGeometryNode",
              "stageGeometryNode2d",
              visualizerGeometryKindClass(geometry.kind),
              `mesh-${geometry.meshKind.toLowerCase()}`,
              geometry.inGroupFilter ? "" : "muted",
              geometry.selected ? "selected" : "",
              geometry.mappedChannelCount > 0 ? "mapped" : "",
            ].filter(Boolean).join(" ")}
          >
            <circle cx={geometry.x} cy={geometry.z} r={visualizerGeometryKindClass(geometry.kind) === "beam" ? 1.35 : 1.05}>
              <title>
                {geometry.name} / {geometry.kind} / {geometry.mappedChannelCount} mapped channel(s)
                {geometry.modelLabel ? ` / ${geometry.modelLabel}` : ""}
              </title>
            </circle>
          </g>
        )}
      </For>
    </Show>
    <Show when={props.mappingShowFixtures()}>
      <For each={props.visualizerFixtures()}>
        {(fixture) => (
          <>
            <Show when={props.mappingShowBeams()}>
              <polygon
                class={fixture.inGroupFilter ? "stageBeam" : "stageBeam muted"}
                points={fixture.beamPoints}
                fill={fixture.color}
                opacity={Math.max(0.08, fixture.intensity * 0.55)}
              />
            </Show>
            <Show when={fixture.id === props.selectedFixtureId() || props.stageFixtureDrag()?.fixture_id === fixture.id}>
              <line
                class={props.stageFixtureDrag()?.mode === "yaw" && props.stageFixtureDrag()?.fixture_id === fixture.id
                  ? "stageYawLine dragging"
                  : "stageYawLine"}
                x1={fixture.x}
                y1={fixture.z}
                x2={fixture.yawHandleX}
                y2={fixture.yawHandleZ}
              />
              <circle
                class={props.stageFixtureDrag()?.mode === "yaw" && props.stageFixtureDrag()?.fixture_id === fixture.id
                  ? "stageYawHandle dragging"
                  : "stageYawHandle"}
                cx={fixture.yawHandleX}
                cy={fixture.yawHandleZ}
                r="2.3"
                onPointerDown={(event) => props.beginStageFixtureYawDrag(event, fixture.id)}
                onPointerMove={(event) => props.dragStageFixtureYaw(event, fixture.id)}
                onPointerUp={(event) => void props.endStageFixtureYawDrag(event, fixture.id)}
                onPointerCancel={(event) => void props.endStageFixtureYawDrag(event, fixture.id)}
              >
                <title data-no-localize>{fixture.label} yaw {fixture.yaw} deg</title>
              </circle>
            </Show>
            <Show
              when={fixture.visualKind === "bar" || fixture.visualKind === "panel" || fixture.visualKind === "laser"}
              fallback={
                <circle
                  class={[
                    "stageFixture",
                    `kind-${fixture.visualKind}`,
                    fixture.inGroupFilter ? "" : "muted",
                    (props.selectedFixtureGroupFilter() || props.selectedFixtureTypeFilter() || props.mappingFixtureSearch().trim()) && fixture.inGroupFilter ? "groupMatch" : "",
                    props.selectedFixtureTypeFilter() && fixture.typeKey === props.selectedFixtureTypeFilter() ? "typeMatch" : "",
                    props.selectedMappingFixtureIdSet().has(fixture.id) ? "picked" : "",
                    fixture.id === props.selectedFixtureId() ? "selected" : "",
                    fixture.highlighted ? "highlighted" : "",
                    fixture.soloed ? "soloed" : "",
                    fixture.parked ? "parked" : "",
                    (props.stageFixtureDrag()?.fixture_ids?.includes(fixture.id) || props.stageFixtureDrag()?.fixture_id === fixture.id)
                      ? "dragging"
                      : "",
                  ].filter(Boolean).join(" ")}
                  cx={fixture.x}
                  cy={fixture.z}
                  r={Math.max(fixture.width, fixture.height) / 2 + fixture.intensity * 1.8}
                  fill={fixture.color}
                  onPointerDown={(event) => props.beginStageFixtureDrag(event, fixture.id)}
                  onPointerMove={(event) => props.dragStageFixture(event, fixture.id)}
                  onPointerUp={(event) => void props.endStageFixtureDrag(event, fixture.id)}
                  onPointerCancel={(event) => void props.endStageFixtureDrag(event, fixture.id)}
                  onDblClick={(event) => {
                    event.stopPropagation();
                    props.openControlForFixtureId(fixture.id);
                  }}
                >
                  <title data-no-localize>{fixture.label} / {fixture.dmxLabel} / {fixture.groupLabel} / {fixture.visualKind}</title>
                </circle>
              }
            >
              <g
                class={[
                  "stageFixture",
                  "stageFixtureBlock",
                  `kind-${fixture.visualKind}`,
                  fixture.inGroupFilter ? "" : "muted",
                  (props.selectedFixtureGroupFilter() || props.selectedFixtureTypeFilter() || props.mappingFixtureSearch().trim()) && fixture.inGroupFilter ? "groupMatch" : "",
                  props.selectedFixtureTypeFilter() && fixture.typeKey === props.selectedFixtureTypeFilter() ? "typeMatch" : "",
                  props.selectedMappingFixtureIdSet().has(fixture.id) ? "picked" : "",
                  fixture.id === props.selectedFixtureId() ? "selected" : "",
                  fixture.highlighted ? "highlighted" : "",
                  fixture.soloed ? "soloed" : "",
                  fixture.parked ? "parked" : "",
                  (props.stageFixtureDrag()?.fixture_ids?.includes(fixture.id) || props.stageFixtureDrag()?.fixture_id === fixture.id)
                    ? "dragging"
                    : "",
                ].filter(Boolean).join(" ")}
                transform={`translate(${fixture.x} ${fixture.z}) rotate(${fixture.yaw})`}
                onPointerDown={(event) => props.beginStageFixtureDrag(event, fixture.id)}
                onPointerMove={(event) => props.dragStageFixture(event, fixture.id)}
                onPointerUp={(event) => void props.endStageFixtureDrag(event, fixture.id)}
                onPointerCancel={(event) => void props.endStageFixtureDrag(event, fixture.id)}
                onDblClick={(event) => {
                  event.stopPropagation();
                  props.openControlForFixtureId(fixture.id);
                }}
              >
                <rect
                  class="stageFixtureShape"
                  x={-fixture.width / 2}
                  y={-fixture.height / 2}
                  width={fixture.width}
                  height={fixture.height}
                  rx={fixture.visualKind === "panel" ? 0.7 : 1.1}
                  fill={fixture.color}
                  opacity={Math.max(0.78, 0.46 + fixture.intensity * 0.54)}
                >
                  <title data-no-localize>{fixture.label} / {fixture.dmxLabel} / {fixture.groupLabel} / {fixture.visualKind}</title>
                </rect>
                <Show when={fixture.visualKind === "bar"}>
                  <line
                    class="stageFixtureCenterLine"
                    x1={-fixture.width / 2 + 1.2}
                    y1="0"
                    x2={fixture.width / 2 - 1.2}
                    y2="0"
                  />
                </Show>
                <Show when={fixture.visualKind === "panel"}>
                  <line class="stageFixtureCenterLine" x1={-fixture.width / 2} y1="0" x2={fixture.width / 2} y2="0" />
                  <line class="stageFixtureCenterLine" x1="0" y1={-fixture.height / 2} x2="0" y2={fixture.height / 2} />
                </Show>
                <Show when={fixture.visualKind === "laser"}>
                  <polygon
                    class="stageFixtureLaserMark"
                    points={`0,${-fixture.height / 2 - 1.4} 1.4,${-fixture.height / 2 + 1.2} -1.4,${-fixture.height / 2 + 1.2}`}
                  />
                </Show>
              </g>
            </Show>
            <Show when={props.mappingShowLabels()}>
              <text class={fixture.inGroupFilter ? "stageLabel" : "stageLabel muted"} x={fixture.x + 4} y={fixture.z - 4}>
                {fixture.mapLabel}
              </text>
            </Show>
            <Show when={props.mappingShowLevels()}>
              <text
                class={fixture.inGroupFilter ? "stageLevelLabel" : "stageLevelLabel muted"}
                x={fixture.x + 4}
                y={fixture.z + 5}
              >
                {Math.round(fixture.intensity * 100)}%
              </text>
            </Show>
          </>
        )}
      </For>
      <Show when={props.mappingPlacePreview()}>
        {(preview) => (
          <g
            class={`stagePlacePreview kind-${preview().visualKind}`}
            transform={`translate(${preview().x} ${preview().z}) rotate(${preview().yaw})`}
          >
            <Show
              when={
                preview().visualKind === "bar" ||
                preview().visualKind === "panel" ||
                preview().visualKind === "laser"
              }
              fallback={
                <circle
                  r={Math.max(preview().width, preview().height) / 2}
                  fill={preview().color}
                >
                  <title>Place {preview().label} / {preview().dmxLabel} / {preview().groupLabel}</title>
                </circle>
              }
            >
              <rect
                class="stagePlacePreviewShape"
                x={-preview().width / 2}
                y={-preview().height / 2}
                width={preview().width}
                height={preview().height}
                rx={preview().visualKind === "panel" ? 0.7 : 1.1}
                fill={preview().color}
              >
                <title>Place {preview().label} / {preview().dmxLabel} / {preview().groupLabel}</title>
              </rect>
              <Show when={preview().visualKind === "bar"}>
                <line
                  class="stagePlacePreviewCenterLine"
                  x1={-preview().width / 2 + 1.2}
                  y1="0"
                  x2={preview().width / 2 - 1.2}
                  y2="0"
                />
              </Show>
              <Show when={preview().visualKind === "panel"}>
                <line class="stagePlacePreviewCenterLine" x1={-preview().width / 2} y1="0" x2={preview().width / 2} y2="0" />
                <line class="stagePlacePreviewCenterLine" x1="0" y1={-preview().height / 2} x2="0" y2={preview().height / 2} />
              </Show>
              <Show when={preview().visualKind === "laser"}>
                <polygon
                  class="stageFixtureLaserMark"
                  points={`0,${-preview().height / 2 - 1.4} 1.4,${-preview().height / 2 + 1.2} -1.4,${-preview().height / 2 + 1.2}`}
                />
              </Show>
            </Show>
            <text x={preview().width / 2 + 1.6} y={-preview().height / 2 - 1}>
              Place {preview().label}
            </text>
          </g>
        )}
      </Show>
    </Show>
  </svg>
);
