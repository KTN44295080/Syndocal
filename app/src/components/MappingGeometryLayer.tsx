import { For, Show } from "solid-js";

export interface MappingGeometryNode2D {
  className: string;
  fixtureLabel: string;
  name: string;
  kind: string;
  x: number;
  z: number;
  radius: number;
  footprintWidth: number;
  footprintHeight: number;
  mappedChannelCount: number;
}

type MappingGeometryLayerProps = {
  nodes: MappingGeometryNode2D[];
  showLabels: boolean;
};

export function MappingGeometryLayer(props: MappingGeometryLayerProps) {
  return (
    <For each={props.nodes}>
      {(geometry) => (
        <g class={geometry.className}>
          <ellipse
            class="stageGeometryFootprint"
            cx={geometry.x}
            cy={geometry.z}
            rx={geometry.footprintWidth / 2}
            ry={geometry.footprintHeight / 2}
          >
            <title>
              {geometry.fixtureLabel} / {geometry.name} / {geometry.kind}
              {geometry.mappedChannelCount > 0 ? ` / ${geometry.mappedChannelCount} channel(s)` : ""}
            </title>
          </ellipse>
          <circle cx={geometry.x} cy={geometry.z} r={geometry.radius}>
            <title>
              {geometry.fixtureLabel} / {geometry.name} / {geometry.kind}
              {geometry.mappedChannelCount > 0 ? ` / ${geometry.mappedChannelCount} channel(s)` : ""}
            </title>
          </circle>
          <Show when={props.showLabels}>
            <text x={geometry.x + 1.8} y={geometry.z - 1.6}>
              {geometry.name}
            </text>
          </Show>
        </g>
      )}
    </For>
  );
}
