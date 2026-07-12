import { For, Show } from "solid-js";
import type { MappingFixtureVisualKind } from "../fixtureVisuals";

export interface MappingFixture2D {
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
  yawHandleX: number;
  yawHandleZ: number;
  intensity: number;
  color: string;
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
  fixtures: MappingFixture2D[];
  selectedFixtureIds: Set<number>;
  selectedFixtureId: number | null;
  selectedGroupId: string | null;
  selectedTypeKey: string | null;
  showLabels: boolean;
  showLevels: boolean;
  placePreview: MappingPlacePreview2D | null;
  isDragging: (fixtureId: number) => boolean;
  isYawDragging: (fixtureId: number) => boolean;
  onFixturePointerDown: (event: PointerEvent, fixtureId: number) => void;
  onBeginYawDrag: (event: PointerEvent, fixtureId: number) => void;
};

export function MappingFixturesLayer(props: MappingFixturesLayerProps) {
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
          const yawDragging = () => props.isYawDragging(fixture.id);
          const showYawHandle = () => selected() || props.selectedFixtureId === fixture.id || yawDragging();
          return (
            <>
              <g
                class={className()}
                transform={`translate(${fixture.x} ${fixture.z}) rotate(${fixture.yaw})`}
                onPointerDown={(event) => props.onFixturePointerDown(event, fixture.id)}
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
                          r={Math.max(fixture.width, fixture.height) / 2 + fixture.intensity * 1.5}
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
              <Show when={showYawHandle()}>
                <line
                  class={yawDragging() ? "stageYawLine dragging" : "stageYawLine"}
                  x1={fixture.x}
                  y1={fixture.z}
                  x2={fixture.yawHandleX}
                  y2={fixture.yawHandleZ}
                />
                <circle
                  class={yawDragging() ? "stageYawHandle dragging" : "stageYawHandle"}
                  cx={fixture.yawHandleX}
                  cy={fixture.yawHandleZ}
                  r="2.3"
                  onPointerDown={(event) => props.onBeginYawDrag(event, fixture.id)}
                >
                  <title data-no-localize>{fixture.label} yaw {fixture.yaw} deg</title>
                </circle>
              </Show>
              <Show when={props.showLabels}>
                <text data-no-localize class="stageLabel" x={fixture.x + 3.5} y={fixture.z - 3.5}>
                  {fixture.label}
                </text>
              </Show>
              <Show when={props.showLevels}>
                <text class={fixture.inGroupFilter ? "stageLevelLabel" : "stageLevelLabel muted"} x={fixture.x + 3.5} y={fixture.z + 5}>
                  {Math.round(fixture.intensity * 100)}%
                </text>
              </Show>
            </>
          );
        }}
      </For>
      <Show when={props.placePreview}>
        {(preview) => (
          <g
            class={`stagePlacePreview kind-${preview().visualKind}`}
            transform={`translate(${preview().x} ${preview().z}) rotate(${preview().yaw})`}
          >
            <Show
              when={preview().visualKind === "bar" || preview().visualKind === "panel"}
              fallback={
                <Show
                  when={preview().visualKind === "laser"}
                  fallback={
                    <circle
                      r={Math.max(preview().width, preview().height) / 2}
                      fill={preview().color}
                    >
                      <title>Place {preview().label} / {preview().dmxLabel} / {preview().groupLabel}</title>
                    </circle>
                  }
                >
                  <polygon
                    class="stagePlacePreviewShape"
                    points={`0,${-preview().height / 2} ${preview().width / 2},${preview().height / 2} ${-preview().width / 2},${preview().height / 2}`}
                    fill={preview().color}
                  >
                    <title>Place {preview().label} / {preview().dmxLabel} / {preview().groupLabel}</title>
                  </polygon>
                </Show>
              }
            >
              <rect
                class="stagePlacePreviewShape"
                x={-preview().width / 2}
                y={-preview().height / 2}
                width={preview().width}
                height={preview().height}
                fill={preview().color}
              >
                <title>Place {preview().label} / {preview().dmxLabel} / {preview().groupLabel}</title>
              </rect>
              <line
                class="stagePlacePreviewCenterLine"
                x1="0"
                y1="0"
                x2="0"
                y2={-Math.max(5, preview().height / 2 + 2)}
              />
            </Show>
            <text x={preview().width / 2 + 1.6} y={-preview().height / 2 - 1}>
              Place {preview().label}
            </text>
          </g>
        )}
      </Show>
    </>
  );
}
