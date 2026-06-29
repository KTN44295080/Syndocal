import { For, Show } from "solid-js";
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
  onSelectFixture: (fixtureId: number) => void;
  onSelectVideoOutput: (outputId: number) => void;
};

export function StagePreview2D(props: StagePreview2DProps) {
  return (
    <svg class={`visualizerStage ${props.className}`} viewBox={`0 0 ${stageViewBoxSize} ${stageViewBoxSize}`}>
      <defs>
        <pattern id={props.patternId} width="10" height="10" patternUnits="userSpaceOnUse">
          <path d="M 10 0 L 0 0 0 10" />
        </pattern>
      </defs>
      <rect class="stageFloor" x="0" y="0" width={stageViewBoxSize} height={stageViewBoxSize} />
      <rect class="stageGrid" x="0" y="0" width={stageViewBoxSize} height={stageViewBoxSize} />
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
            <text x={-object.width / 2 + 1} y={-object.depth / 2 - 1}>
              {object.label}
            </text>
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
            <text x={-surface.width / 2 + 1.2} y={-surface.height / 2 - 1.6}>
              {surface.label}
            </text>
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
              <line class="stageFixtureCenterLine" x1="0" y1="0" x2="0" y2="-7" />
              <circle class="stageFixtureLaserMark" cx="0" cy="-7" r="0.9" />
              <title>{`${fixture.label} / ${fixture.dmxLabel} / ${fixture.groupLabel}`}</title>
            </g>
          );
        }}
      </For>
    </svg>
  );
}
