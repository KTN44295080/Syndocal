import { createMemo, createSignal, For, onCleanup, onMount } from "solid-js";
import type { FixtureLiveColorSegment } from "../fixtureLiveColor";
import type { MappingFixtureVisualKind } from "../fixtureVisuals";
import { planStageFixtureLabels } from "../stageLabelLayout";
import { stageViewBoxSize } from "../stageGeometry";
import { stageObjectClass } from "../stageObjects";
import type { StageObjectKind } from "../types";
import { StageFixtureGlyph, StageFixtureLabel, StageObjectGlyph, StageProjectionSurfaceGlyph } from "./StageGlyphs";

export interface StagePreviewFixture {
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
  yaw: number;
  beamPoints: string;
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

export interface StagePreviewVideoSurface {
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

export interface StagePreviewObject {
  id: number;
  label: string;
  kind: StageObjectKind;
  x: number;
  z: number;
  width: number;
  depth: number;
  rotationDeg: number;
  color: string;
}

type StagePreview2DProps = {
  className: string;
  patternId: string;
  stageOrigin: { x: number; z: number };
  fixtures: StagePreviewFixture[];
  videoSurfaces: StagePreviewVideoSurface[];
  stageObjects: StagePreviewObject[];
  selectedFixtureId: number | null;
  selectedVideoOutputId: number | null;
  selectedFixtureGroupFilter: string | null;
  selectedFixtureTypeFilter?: string | null;
  stageObjectClassName: string;
  surfaceMinOpacity: number;
  beamMinOpacity: number;
  beamIntensityScale: number;
  compact?: boolean;
  showLabels?: boolean;
  viewAspectRatio?: number;
  onSelectFixture: (fixtureId: number) => void;
  onSelectVideoOutput: (outputId: number) => void;
};

export function StagePreview2D(props: StagePreview2DProps) {
  let stageElement: SVGSVGElement | undefined;
  const [measuredAspectRatio, setMeasuredAspectRatio] = createSignal<number | null>(null);
  const [hoveredFixtureId, setHoveredFixtureId] = createSignal<number | null>(null);

  onMount(() => {
    if (!stageElement || typeof ResizeObserver === "undefined") return;
    const observer = new ResizeObserver(([entry]) => {
      const { width, height } = entry.contentRect;
      if (width > 0 && height > 0) setMeasuredAspectRatio(width / height);
    });
    observer.observe(stageElement);
    onCleanup(() => observer.disconnect());
  });

  const viewBox = createMemo(() => {
    const aspect = measuredAspectRatio() ?? props.viewAspectRatio;
    if (!aspect || aspect <= 0) {
      return { x: 0, z: 0, width: stageViewBoxSize, height: stageViewBoxSize };
    }

    const bounds: Array<[number, number, number, number]> = [
      ...props.fixtures.map((fixture) => {
        const radius = Math.max(4, fixture.width / 2, fixture.height / 2);
        return [fixture.x - radius, fixture.z - radius, fixture.x + radius, fixture.z + radius] as [number, number, number, number];
      }),
      ...props.videoSurfaces.map((surface) => [
        surface.x - surface.width / 2,
        surface.z - surface.height / 2,
        surface.x + surface.width / 2,
        surface.z + surface.height / 2,
      ] as [number, number, number, number]),
      ...props.stageObjects.map((object) => [
        object.x - object.width / 2,
        object.z - object.depth / 2,
        object.x + object.width / 2,
        object.z + object.depth / 2,
      ] as [number, number, number, number]),
    ];
    if (bounds.length === 0) {
      return { x: 0, z: 0, width: stageViewBoxSize, height: stageViewBoxSize / aspect };
    }

    let minX = Math.min(...bounds.map((row) => row[0])) - 6;
    let minZ = Math.min(...bounds.map((row) => row[1])) - 6;
    let maxX = Math.max(...bounds.map((row) => row[2])) + 6;
    let maxZ = Math.max(...bounds.map((row) => row[3])) + 6;
    const minimumHeight = 34;
    if (maxZ - minZ < minimumHeight) {
      const center = (minZ + maxZ) / 2;
      minZ = center - minimumHeight / 2;
      maxZ = center + minimumHeight / 2;
    }
    if ((maxX - minX) / (maxZ - minZ) < aspect) {
      const width = (maxZ - minZ) * aspect;
      const center = (minX + maxX) / 2;
      minX = center - width / 2;
      maxX = center + width / 2;
    } else {
      const height = (maxX - minX) / aspect;
      const center = (minZ + maxZ) / 2;
      minZ = center - height / 2;
      maxZ = center + height / 2;
    }
    if (maxX - minX > stageViewBoxSize) {
      minX = 0;
      maxX = stageViewBoxSize;
      const height = stageViewBoxSize / aspect;
      const center = (minZ + maxZ) / 2;
      minZ = center - height / 2;
      maxZ = center + height / 2;
    }
    if (maxZ - minZ > stageViewBoxSize) {
      minZ = 0;
      maxZ = stageViewBoxSize;
      const width = stageViewBoxSize * aspect;
      const center = (minX + maxX) / 2;
      minX = center - width / 2;
      maxX = center + width / 2;
    }
    const shiftIntoStage = (min: number, max: number) => {
      if (min < 0) return [0, max - min] as const;
      if (max > stageViewBoxSize) return [min - (max - stageViewBoxSize), stageViewBoxSize] as const;
      return [min, max] as const;
    };
    [minX, maxX] = shiftIntoStage(minX, maxX);
    [minZ, maxZ] = shiftIntoStage(minZ, maxZ);
    return { x: minX, z: minZ, width: maxX - minX, height: maxZ - minZ };
  });

  const viewBoxAttribute = createMemo(() => {
    const view = viewBox();
    return `${view.x} ${view.z} ${view.width} ${view.height}`;
  });
  const labelLayout = createMemo(() => {
    const view = viewBox();
    return planStageFixtureLabels({
      fixtures: props.fixtures,
      viewport: { x: view.x, z: view.z, width: view.width, height: view.height },
      zoom: stageViewBoxSize / Math.max(view.width, view.height),
      showLabels: props.showLabels !== false,
      pickedFixtureId: props.selectedFixtureId,
      hoveredFixtureId: hoveredFixtureId(),
    });
  });

  return (
    <svg ref={stageElement} class={`visualizerStage ${props.className}`} viewBox={viewBoxAttribute()}>
      <defs>
        <pattern id={`${props.patternId}-minor`} width="5" height="5" patternUnits="userSpaceOnUse">
          <path class="stageGridMinor" d="M 5 0 L 0 0 0 5" />
        </pattern>
        <pattern id={props.patternId} width="20" height="20" patternUnits="userSpaceOnUse">
          <rect width="20" height="20" fill={`url(#${props.patternId}-minor)`} />
          <path class="stageGridMajor" d="M 20 0 L 0 0 0 20" />
        </pattern>
      </defs>
      <rect
        class="stageFloor"
        x={viewBox().x}
        y={viewBox().z}
        width={viewBox().width}
        height={viewBox().height}
      />
      <rect
        class="stageGrid"
        x={viewBox().x}
        y={viewBox().z}
        width={viewBox().width}
        height={viewBox().height}
      />
      <line class="stageAxis2d" x1={props.stageOrigin.x} y1="0" x2={props.stageOrigin.x} y2={stageViewBoxSize} />
      <line class="stageAxis2d" x1="0" y1={props.stageOrigin.z} x2={stageViewBoxSize} y2={props.stageOrigin.z} />
      <For each={props.stageObjects}>
        {(object) => (
          <g
            class={`${stageObjectClass(object)} ${props.stageObjectClassName}`}
            transform={`translate(${object.x} ${object.z}) rotate(${object.rotationDeg})`}
          >
            <StageObjectGlyph
              kind={object.kind}
              width={object.width}
              depth={object.depth}
              color={object.color}
              label={object.label}
              showLabel={object.kind === "Screen" || !props.compact}
            />
          </g>
        )}
      </For>
      <For each={props.videoSurfaces}>
        {(surface) => (
          <g
            class={[
              "stageVideoSurface2d",
              "reference",
              props.selectedVideoOutputId === surface.id ? "selected" : "",
              surface.active ? "" : "inactive",
            ].filter(Boolean).join(" ")}
            transform={`translate(${surface.x} ${surface.z}) rotate(${surface.rotationDeg})`}
            opacity={Math.max(props.surfaceMinOpacity, surface.opacity)}
            onPointerDown={(event) => {
              event.stopPropagation();
              props.onSelectVideoOutput(surface.id);
            }}
          >
            <StageProjectionSurfaceGlyph width={surface.width} label={surface.label} />
          </g>
        )}
      </For>
      <For each={props.fixtures}>
        {(fixture) => (
          <polygon
            class="stageBeam"
            points={fixture.beamPoints}
            fill={fixture.color}
            opacity={Math.max(props.beamMinOpacity, fixture.intensity * props.beamIntensityScale)}
          />
        )}
      </For>
      <For each={props.fixtures}>
        {(fixture) => {
          const className = () => [
            "stageFixture",
            "stageFixtureBlock",
            `kind-${fixture.visualKind}`,
            props.selectedFixtureId === fixture.id ? "selected picked" : "",
            props.selectedFixtureGroupFilter && fixture.inGroupFilter ? "groupMatch" : "",
            props.selectedFixtureTypeFilter && fixture.typeKey === props.selectedFixtureTypeFilter ? "typeMatch" : "",
            fixture.highlighted ? "highlighted" : "",
            fixture.soloed ? "soloed" : "",
            fixture.parked ? "parked" : "",
          ].filter(Boolean).join(" ");
          return (
            <g
              class={className()}
              data-stage-fixture-id={fixture.id}
              data-live-color-applied={fixture.liveColorApplied ? "true" : "false"}
              data-live-color-source={fixture.liveColorValueSource ?? "none"}
              data-live-segment-count={fixture.liveSegments?.length ?? 1}
              transform={`translate(${fixture.x} ${fixture.z}) rotate(${fixture.yaw})`}
              onPointerDown={(event) => {
                event.stopPropagation();
                props.onSelectFixture(fixture.id);
              }}
              onPointerEnter={() => setHoveredFixtureId(fixture.id)}
              onPointerLeave={() =>
                setHoveredFixtureId((current) => current === fixture.id ? null : current)
              }
            >
              <StageFixtureGlyph
                visualKind={fixture.visualKind}
                width={fixture.width}
                height={fixture.height}
                color={fixture.color}
                segments={fixture.liveSegments}
                hitTargetRadius={6}
                title={`${fixture.label} / ${fixture.dmxLabel} / ${fixture.groupLabel}`}
              />
            </g>
          );
        }}
      </For>
      <For each={labelLayout().labels}>
        {(layout) => <StageFixtureLabel layout={layout} />}
      </For>
    </svg>
  );
}
