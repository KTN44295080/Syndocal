import { For } from "solid-js";
import type { VisualizerFixtureBeam } from "../mappingRuntime";

export interface MappingBeamFixture {
  id: number;
  beamPoints: string;
  beams?: VisualizerFixtureBeam[];
  intensity: number;
  color: string;
}

type MappingBeamsLayerProps = {
  fixtures: MappingBeamFixture[];
};

export function MappingBeamsLayer(props: MappingBeamsLayerProps) {
  return (
    <For each={props.fixtures.filter((fixture) => fixture.intensity > 0)}>
      {(fixture) => {
        const beams = () => fixture.beams ?? [{
          cellIndex: 1,
          points: fixture.beamPoints,
          intensity: fixture.intensity,
          color: fixture.color,
        }];
        return (
          <For each={beams()}>
            {(beam) => (
              <polygon
                class="stageBeam"
                data-stage-beam-fixture-id={fixture.id}
                data-stage-beam-cell={beam.cellIndex}
                points={beam.points}
                fill={beam.color}
                opacity={Math.max(0.08, beam.intensity * 0.55)}
              />
            )}
          </For>
        );
      }}
    </For>
  );
}
