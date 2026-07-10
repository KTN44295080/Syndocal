import { For, Show } from "solid-js";
import type { CompositionSummary, VideoLayerSummary } from "../types";

type MaybePromise = void | Promise<unknown>;

type VideoCompositionSetupPanelProps = {
  compositions: CompositionSummary[];
  layers: VideoLayerSummary[];
  draftLabel: string;
  draftLayerIds: number[];
  onDraftLabel: (value: string) => void;
  onToggleDraftLayer: (layerId: number, checked: boolean) => void;
  onAddComposition: () => MaybePromise;
  onRemoveComposition: (compositionId: number) => MaybePromise;
  onSetCompositionLayers: (compositionId: number, layerIds: number[]) => MaybePromise;
  onMoveCompositionLayer: (
    compositionId: number,
    layerIds: number[],
    layerId: number,
    direction: -1 | 1,
  ) => MaybePromise;
};

export function VideoCompositionSetupPanel(props: VideoCompositionSetupPanelProps) {
  const layerLabel = (layerId: number) =>
    props.layers.find((candidate) => candidate.id === layerId)?.label ?? `Video Layer ${layerId}`;

  return (
    <>
      <div class="compositionList">
        <For each={props.compositions}>
          {(composition) => (
            <div class="compositionItem">
              <div class="compositionHeader">
                <strong>{composition.label}</strong>
                <span>{composition.layer_ids.length} video layer(s) / {composition.output_ids.length} video output(s)</span>
              </div>
              <Show when={composition.id !== 1}>
                <Show when={composition.layer_ids.length > 0}>
                  <div class="compositionLayerOrder">
                    <For each={composition.layer_ids}>
                      {(layerId, orderIndex) => (
                        <div class="compositionOrderRow">
                          <span>{orderIndex() + 1}. {layerLabel(layerId)}</span>
                          <div class="buttonRow">
                            <button
                              onClick={() =>
                                void props.onMoveCompositionLayer(composition.id, composition.layer_ids, layerId, -1)
                              }
                              disabled={orderIndex() === 0}
                            >
                              Up
                            </button>
                            <button
                              onClick={() =>
                                void props.onMoveCompositionLayer(composition.id, composition.layer_ids, layerId, 1)
                              }
                              disabled={orderIndex() === composition.layer_ids.length - 1}
                            >
                              Down
                            </button>
                          </div>
                        </div>
                      )}
                    </For>
                  </div>
                </Show>
                <div class="compositionLayerPicker">
                  <For each={props.layers}>
                    {(layer) => (
                      <label class="checkbox">
                        <input
                          type="checkbox"
                          checked={composition.layer_ids.includes(layer.id)}
                          onChange={(event) => {
                            const layerIds = event.currentTarget.checked
                              ? [...composition.layer_ids, layer.id]
                              : composition.layer_ids.filter((candidate) => candidate !== layer.id);
                            void props.onSetCompositionLayers(composition.id, layerIds);
                          }}
                        />
                        {layer.label}
                      </label>
                    )}
                  </For>
                </div>
                <button onClick={() => void props.onRemoveComposition(composition.id)}>Remove Composition</button>
              </Show>
            </div>
          )}
        </For>
      </div>
      <div class="videoCompositionForm">
        <h3>Composition</h3>
        <label>
          Name
          <input value={props.draftLabel} onInput={(event) => props.onDraftLabel(event.currentTarget.value)} />
        </label>
        <div class="compositionLayerPicker">
          <For each={props.layers}>
            {(layer) => (
              <label class="checkbox">
                <input
                  type="checkbox"
                  checked={props.draftLayerIds.includes(layer.id)}
                  onChange={(event) => props.onToggleDraftLayer(layer.id, event.currentTarget.checked)}
                />
                {layer.label}
              </label>
            )}
          </For>
        </div>
        <button class="primary" onClick={() => void props.onAddComposition()}>
          Add Composition
        </button>
      </div>
    </>
  );
}
