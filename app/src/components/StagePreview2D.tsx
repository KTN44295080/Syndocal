import { createMemo, createSignal, For, onCleanup, onMount, Show } from "solid-js";
import type { MappingFixtureVisualKind } from "../fixtureVisuals";
import { stageViewBoxSize } from "../stageGeometry";
import { stageObjectClass } from "../stageObjects";
import type { StageObjectKind } from "../types";

export interface StagePreviewFixture {
  id: number;
  label: string;
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
  fixtureRadiusIntensityScale: number;
  compact?: boolean;
  viewAspectRatio?: number;
  onSelectFixture: (fixtureId: number) => void;
  onSelectVideoOutput: (outputId: number) => void;
};

export function StagePreview2D(props: StagePreview2DProps) {
  let stageElement: SVGSVGElement | undefined;
  const [measuredAspectRatio, setMeasuredAspectRatio] = createSignal<number | null>(null);

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
            <rect
              x={-object.width / 2}
              y={-object.depth / 2}
              width={object.width}
              height={object.depth}
              fill={object.color}
            />
            <line x1={-object.width / 2} y1="0" x2={object.width / 2} y2="0" />
            <line x1="0" y1={-object.depth / 2} x2="0" y2={object.depth / 2} />
            <Show when={!props.compact}>
              <text data-no-localize x={-object.width / 2 + 1} y={-object.depth / 2 - 1}>{object.label}</text>
            </Show>
          </g>
        )}
      </For>
      <For each={props.videoSurfaces}>
        {(surface) => (
          <g
            class={[
              "stageVideoSurface2d",
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
            <rect
              class="stageVideoSurfaceShape"
              x={-surface.width / 2}
              y={-surface.height / 2}
              width={surface.width}
              height={surface.height}
            />
            <line x1={-surface.width / 2} y1="0" x2={surface.width / 2} y2="0" />
            <line x1="0" y1={-surface.height / 2} x2="0" y2={surface.height / 2} />
            <Show when={!props.compact}>
              <text data-no-localize x={-surface.width / 2 + 1.2} y={-surface.height / 2 - 1.6}>{surface.label}</text>
            </Show>
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
              transform={`translate(${fixture.x} ${fixture.z}) rotate(${fixture.yaw})`}
              onPointerDown={(event) => {
                event.stopPropagation();
                props.onSelectFixture(fixture.id);
              }}
            >
              <circle
                class="stageFixtureHitTarget"
                cx="0"
                cy="0"
                r={Math.max(props.compact ? 3.2 : 3.8, fixture.width / 2 + 1.5, fixture.height / 2 + 1.5)}
              />
              <circle
                class="stageFixtureSelectionRing"
                cx="0"
                cy="0"
                r={Math.max(props.compact ? 2.35 : 3.1, fixture.width / 2 + 0.8, fixture.height / 2 + 0.8)}
              />
              <Show
                when={fixture.visualKind === "bar" || fixture.visualKind === "panel"}
                fallback={
                  <Show
                    when={fixture.visualKind === "laser"}
                    fallback={
                      <circle
                        class="stageFixtureShape"
                        cx="0"
                        cy="0"
                        r={Math.max(fixture.width, fixture.height) / 2 + fixture.intensity * props.fixtureRadiusIntensityScale}
                        fill={fixture.color}
                      />
                    }
                  >
                    <polygon
                      class="stageFixtureShape"
                      points={`0,${-fixture.height / 2} ${fixture.width / 2},${fixture.height / 2} ${-fixture.width / 2},${fixture.height / 2}`}
                      fill={fixture.color}
                    />
                  </Show>
                }
              >
                <rect
                  class="stageFixtureShape"
                  x={-fixture.width / 2}
                  y={-fixture.height / 2}
                  width={fixture.width}
                  height={fixture.height}
                  fill={fixture.color}
                />
              </Show>
              <line class="stageFixtureCenterLine" x1="0" y1="0" x2="0" y2={props.compact ? -3.6 : -7} />
              <circle class="stageFixtureLaserMark" cx="0" cy={props.compact ? -3.6 : -7} r={props.compact ? 0.62 : 0.9} />
              <title>{`${fixture.label} / ${fixture.dmxLabel} / ${fixture.groupLabel}`}</title>
            </g>
          );
        }}
      </For>
    </svg>
  );
}
