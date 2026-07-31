import { For, Show } from "solid-js";
import type { GeometrySummary, PatchedFixtureSummary } from "../types";
import type { FixtureLayoutMode } from "./PatchFixtureFormPanel";

type MaybePromise = void | Promise<unknown>;

export type MappingFixtureGeometryRow = {
  geometry: GeometrySummary;
  positionLabel: string;
  controlCount: number;
};

type MappingFixtureInspectorPanelProps = {
  fixture: PatchedFixtureSummary | null;
  geometryRows: MappingFixtureGeometryRow[];
  unresolvedGeometryReferences: string[];
  filteredFixtureCount: number;
  onSetTransform: (
    fixture: PatchedFixtureSummary,
    updates: Partial<Pick<PatchedFixtureSummary, "position" | "rotation">>,
  ) => MaybePromise;
  onLayoutFixtures: (mode: FixtureLayoutMode) => MaybePromise;
  onControl: () => void;
  onPatch: () => void;
  onRemove: () => MaybePromise;
};

export function MappingFixtureInspectorPanel(props: MappingFixtureInspectorPanelProps) {
  return (
    <Show
      when={props.fixture}
      fallback={
        <div class="mappingTransformInspector">
          <div class="mappingTransformTitle">
            <strong>No fixture selected</strong>
            <span>Select a fixture on the map or patch list.</span>
          </div>
        </div>
      }
    >
      {(fixture) => (
        <div class="mappingTransformInspector">
          <div class="mappingTransformTitle">
            <strong>{fixture().label}</strong>
            <span
              data-fixture-profile-summary
              data-fixture-profile-family={`${fixture().manufacturer}|${fixture().profile_name}|${fixture().mode_name}`}
            >
              {fixture().manufacturer} {fixture().profile_name} / U{fixture().universe} A{fixture().address}
            </span>
          </div>
          <div class="mappingFixtureCoordinateEditor" data-fixture-coordinate-editor>
            <strong>2D Mapping</strong>
            <div class="mappingTransformGrid">
              <label>
                X
                <input
                  type="number"
                  step="0.1"
                  value={fixture().position.x}
                  onChange={(event) =>
                    void props.onSetTransform(fixture(), {
                      position: { ...fixture().position, x: Number(event.currentTarget.value) },
                    })
                  }
                />
              </label>
              <label>
                Z
                <input
                  type="number"
                  step="0.1"
                  value={fixture().position.z}
                  onChange={(event) =>
                    void props.onSetTransform(fixture(), {
                      position: { ...fixture().position, z: Number(event.currentTarget.value) },
                    })
                  }
                />
              </label>
              <label>
                Y
                <input
                  type="number"
                  step="0.1"
                  value={fixture().position.y}
                  onChange={(event) =>
                    void props.onSetTransform(fixture(), {
                      position: { ...fixture().position, y: Number(event.currentTarget.value) },
                    })
                  }
                />
              </label>
              <label>
                Yaw
                <input
                  type="number"
                  step="1"
                  value={fixture().rotation.yaw}
                  onChange={(event) =>
                    void props.onSetTransform(fixture(), {
                      rotation: { ...fixture().rotation, yaw: Number(event.currentTarget.value) },
                    })
                  }
                />
              </label>
            </div>
            <details class="mappingCoordinateAdvanced" data-fixture-coordinate-advanced>
              <summary>Advanced</summary>
              <div class="mappingTransformGrid">
                <label>
                  Pitch
                  <input
                    type="number"
                    step="1"
                    value={fixture().rotation.pitch}
                    onChange={(event) =>
                      void props.onSetTransform(fixture(), {
                        rotation: { ...fixture().rotation, pitch: Number(event.currentTarget.value) },
                      })
                    }
                  />
                </label>
                <label>
                  Roll
                  <input
                    type="number"
                    step="1"
                    value={fixture().rotation.roll}
                    onChange={(event) =>
                      void props.onSetTransform(fixture(), {
                        rotation: { ...fixture().rotation, roll: Number(event.currentTarget.value) },
                      })
                    }
                  />
                </label>
              </div>
              <div class="presetRow">
                <button onClick={() => void props.onLayoutFixtures("line")} disabled={props.filteredFixtureCount === 0}>
                  Line X
                </button>
                <button onClick={() => void props.onLayoutFixtures("grid")} disabled={props.filteredFixtureCount === 0}>
                  Grid
                </button>
                <button onClick={() => void props.onLayoutFixtures("circle")} disabled={props.filteredFixtureCount === 0}>
                  Circle
                </button>
              </div>
            </details>
          </div>
          <details class="mappingGeometrySummary" data-gdtf-geometry-disclosure>
            <summary class="mappingTransformTitle">
              <strong>GDTF Geometry</strong>
              <span>{props.geometryRows.length} node(s)</span>
            </summary>
            <div class="mappingGeometryDisclosureBody">
              <Show when={props.geometryRows.length > 0} fallback={<p class="empty">No geometry nodes on this fixture.</p>}>
                <div class="mappingGeometryList">
                  <For each={props.geometryRows}>
                    {(row) => (
                      <div class="mappingGeometryRow">
                        <strong data-no-localize title={row.geometry.name}>{row.geometry.name}</strong>
                        <span>{row.geometry.kind}</span>
                        <small title={row.positionLabel}>{row.positionLabel}</small>
                        <b>{row.controlCount} ctl</b>
                        <Show when={row.geometry.model_file || row.geometry.model_name || row.geometry.model_primitive}>
                          <em title={row.geometry.model_file ?? row.geometry.model_name ?? row.geometry.model_primitive ?? ""}>
                            {row.geometry.model_file ? "mesh" : row.geometry.model_primitive ? row.geometry.model_primitive : "model"}
                          </em>
                        </Show>
                      </div>
                    )}
                  </For>
                </div>
              </Show>
              <Show when={props.unresolvedGeometryReferences.length > 0}>
                <ul class="mappingGeometryWarnings">
                  <For each={props.unresolvedGeometryReferences}>
                    {(reference) => <li>Missing geometry reference: {reference}</li>}
                  </For>
                </ul>
              </Show>
            </div>
          </details>
          <div class="mappingControlShortcuts">
            <button onClick={() => props.onControl()}>Control</button>
            <button onClick={() => props.onPatch()}>Patch</button>
            <button onClick={() => void props.onRemove()}>Remove</button>
          </div>
        </div>
      )}
    </Show>
  );
}
