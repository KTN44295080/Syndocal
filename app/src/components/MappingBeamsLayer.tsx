import { For } from "solid-js";

export interface MappingBeamFixture {
  beamPoints: string;
  intensity: number;
  color: string;
}

type MappingBeamsLayerProps = {
  fixtures: MappingBeamFixture[];
};

export function MappingBeamsLayer(props: MappingBeamsLayerProps) {
  return (
    <For each={props.fixtures}>
      {(fixture) => (
        <polygon
          class="stageBeam"
          points={fixture.beamPoints}
          fill={fixture.color}
          opacity={Math.max(0.08, fixture.intensity * 0.55)}
        />
      )}
    </For>
  );
}
