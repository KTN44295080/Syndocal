import { For, Show } from "solid-js";
import type { StageObjectKind, StageObjectSummary } from "../types";
import { stageObjectDefaultColor, stageObjectKinds } from "../stageObjects";

type MaybePromise = void | Promise<unknown>;
type StageObjectLayoutMode = "line" | "grid";
type StageObjectPickMode = "replace" | "add";

type MappingStageObjectPanelProps = {
  stageObjects: StageObjectSummary[];
  stageObjectFixtureCounts: Record<number, number>;
  selectedObject: StageObjectSummary | null;
  selectedObjectId: number | null;
  selectedFixtureCount: number;
  draftLabel: string;
  draftKind: StageObjectKind;
  draftWidth: number;
  draftDepth: number;
  draftRotation: number;
  draftColor: string;
  onDraftLabel: (value: string) => void;
  onDraftKind: (value: StageObjectKind) => void;
  onDraftWidth: (value: number) => void;
  onDraftDepth: (value: number) => void;
  onDraftRotation: (value: number) => void;
  onDraftColor: (value: string) => void;
  onAddCenter: () => MaybePromise;
  onSelectObject: (objectId: number) => void;
  onSetObject: (object: StageObjectSummary, updates: Partial<StageObjectSummary>) => MaybePromise;
  onRemoveObject: (objectId: number) => MaybePromise;
  onPickInside: (mode: StageObjectPickMode) => void;
  onLayoutOnObject: (mode: StageObjectLayoutMode) => MaybePromise;
};

export function MappingStageObjectPanel(props: MappingStageObjectPanelProps) {
  const fixtureCountForObject = (objectId: number) => props.stageObjectFixtureCounts[objectId] ?? 0;

  const setDraftKind = (kind: StageObjectKind) => {
    props.onDraftKind(kind);
    props.onDraftColor(stageObjectDefaultColor(kind));
  };

  return (
    <>
      <div class="mappingStageObjectPanel">
        <div class="mappingTransformTitle">
          <strong>Stage Objects</strong>
          <span>{props.stageObjects.length} reference(s)</span>
        </div>
        <label>
          Label
          <input
            type="text"
            value={props.draftLabel}
            onInput={(event) => props.onDraftLabel(event.currentTarget.value)}
          />
        </label>
        <div class="mappingProjectorFieldGrid">
          <label>
            Kind
            <select
              value={props.draftKind}
              onChange={(event) => setDraftKind(event.currentTarget.value as StageObjectKind)}
            >
              <For each={stageObjectKinds}>{(kind) => <option value={kind}>{kind}</option>}</For>
            </select>
          </label>
          <label>
            Width
            <input
              type="number"
              min="0.05"
              step="0.1"
              value={props.draftWidth}
              onInput={(event) => props.onDraftWidth(Number(event.currentTarget.value))}
            />
          </label>
          <label>
            Depth
            <input
              type="number"
              min="0.05"
              step="0.1"
              value={props.draftDepth}
              onInput={(event) => props.onDraftDepth(Number(event.currentTarget.value))}
            />
          </label>
          <label>
            Rot
            <input
              type="number"
              step="1"
              value={props.draftRotation}
              onInput={(event) => props.onDraftRotation(Number(event.currentTarget.value))}
            />
          </label>
        </div>
        <div class="mappingStageObjectColorRow">
          <label>
            Color
            <input
              type="color"
              value={props.draftColor}
              onInput={(event) => props.onDraftColor(event.currentTarget.value)}
            />
          </label>
          <button class="primary" onClick={() => void props.onAddCenter()}>
            Add Center
          </button>
        </div>
        <For each={props.stageObjects}>
          {(object) => (
            <button
              class={props.selectedObjectId === object.id ? "stageObjectListItem active" : "stageObjectListItem"}
              onClick={() => props.onSelectObject(object.id)}
            >
              <strong data-no-localize>{object.label}</strong>
              <span>
                {object.kind} / {fixtureCountForObject(object.id)} fixture(s)
              </span>
              <small>X {object.x.toFixed(1)} / Z {object.z.toFixed(1)}</small>
            </button>
          )}
        </For>
      </div>
      <Show when={props.selectedObject}>
        {(object) => (
          <div class="mappingStageObjectPanel selectedObject">
            <div class="mappingTransformTitle">
              <strong>{object().label}</strong>
              <span>
                {object().kind} / {object().width.toFixed(1)}x{object().depth.toFixed(1)}m /{" "}
                {fixtureCountForObject(object().id)} fixture(s)
              </span>
            </div>
            <div class="mappingTransformGrid">
              <label>
                X
                <input
                  type="number"
                  step="0.1"
                  value={object().x}
                  onChange={(event) => void props.onSetObject(object(), { x: Number(event.currentTarget.value) })}
                />
              </label>
              <label>
                Z
                <input
                  type="number"
                  step="0.1"
                  value={object().z}
                  onChange={(event) => void props.onSetObject(object(), { z: Number(event.currentTarget.value) })}
                />
              </label>
              <label>
                Width
                <input
                  type="number"
                  min="0.05"
                  step="0.1"
                  value={object().width}
                  onChange={(event) => void props.onSetObject(object(), { width: Number(event.currentTarget.value) })}
                />
              </label>
              <label>
                Depth
                <input
                  type="number"
                  min="0.05"
                  step="0.1"
                  value={object().depth}
                  onChange={(event) => void props.onSetObject(object(), { depth: Number(event.currentTarget.value) })}
                />
              </label>
              <label>
                Rot
                <input
                  type="number"
                  step="1"
                  value={object().rotation_deg}
                  onChange={(event) =>
                    void props.onSetObject(object(), { rotation_deg: Number(event.currentTarget.value) })
                  }
                />
              </label>
              <label>
                Kind
                <select
                  value={object().kind}
                  onChange={(event) => {
                    const kind = event.currentTarget.value as StageObjectKind;
                    void props.onSetObject(object(), {
                      kind,
                      color: object().color ?? stageObjectDefaultColor(kind),
                    });
                  }}
                >
                  <For each={stageObjectKinds}>{(kind) => <option value={kind}>{kind}</option>}</For>
                </select>
              </label>
            </div>
            <label>
              Label
              <input
                type="text"
                value={object().label}
                onChange={(event) => void props.onSetObject(object(), { label: event.currentTarget.value })}
              />
            </label>
            <div class="mappingStageObjectColorRow">
              <label>
                Color
                <input
                  type="color"
                  value={object().color ?? stageObjectDefaultColor(object().kind)}
                  onInput={(event) => void props.onSetObject(object(), { color: event.currentTarget.value })}
                />
              </label>
              <button onClick={() => void props.onRemoveObject(object().id)}>
                Delete Object
              </button>
            </div>
            <div class="mappingTransformActions">
              <button onClick={() => props.onPickInside("replace")} title="Pick fixtures inside the selected stage object (I)">
                Pick Inside
              </button>
              <button onClick={() => props.onPickInside("add")} title="Add fixtures inside the selected stage object to the current pick (Shift+I)">
                Add Inside
              </button>
              <button
                onClick={() => void props.onLayoutOnObject("line")}
                disabled={props.selectedFixtureCount === 0}
              >
                Line Here
              </button>
              <button
                onClick={() => void props.onLayoutOnObject("grid")}
                disabled={props.selectedFixtureCount === 0}
              >
                Grid Here
              </button>
            </div>
          </div>
        )}
      </Show>
    </>
  );
}
