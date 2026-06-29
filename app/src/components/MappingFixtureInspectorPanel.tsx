import { For, Show } from "solid-js";
import type { GeometrySummary, PatchedFixtureSummary } from "../types";

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
  onSetTransform: (
    fixture: PatchedFixtureSummary,
    updates: Partial<Pick<PatchedFixtureSummary, "position" | "rotation">>,
  ) => MaybePromise;
  onSetHighlight: (fixtureId: number, enabled: boolean) => MaybePromise;
  onSetSolo: (fixtureId: number, enabled: boolean) => MaybePromise;
  onSetPark: (fixtureId: number, enabled: boolean) => MaybePromise;
  onControl: () => void;
  onPatch: () => void;
  onDuplicate: () => MaybePromise;
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
            <span>
              {fixture().manufacturer} {fixture().profile_name} / U{fixture().universe} A{fixture().address}
            </span>
          </div>
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
          <div class="mappingGeometrySummary">
            <div class="mappingTransformTitle">
              <strong>GDTF Geometry</strong>
              <span>{props.geometryRows.length} node(s)</span>
            </div>
            <Show when={props.geometryRows.length > 0} fallback={<p class="empty">No geometry nodes on this fixture.</p>}>
              <div class="mappingGeometryList">
                <For each={props.geometryRows}>
                  {(row) => (
                    <div class="mappingGeometryRow">
                      <strong title={row.geometry.name}>{row.geometry.name}</strong>
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
          <div class="mappingFlagActions">
            <button
              class={fixture().highlighted ? "active" : ""}
              onClick={() => void props.onSetHighlight(fixture().id, !fixture().highlighted)}
            >
              Highlight
            </button>
            <button
              class={fixture().soloed ? "active" : ""}
              onClick={() => void props.onSetSolo(fixture().id, !fixture().soloed)}
            >
              Solo
            </button>
            <button
              class={fixture().parked ? "active" : ""}
              onClick={() => void props.onSetPark(fixture().id, !fixture().parked)}
            >
              Park
            </button>
          </div>
          <div class="mappingControlShortcuts">
            <button onClick={() => props.onControl()}>Control</button>
            <button onClick={() => props.onPatch()}>Patch</button>
            <button onClick={() => void props.onDuplicate()}>Duplicate</button>
            <button onClick={() => void props.onRemove()}>Remove</button>
          </div>
        </div>
      )}
    </Show>
  );
}
