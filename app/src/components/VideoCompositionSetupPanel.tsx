import { For, Show } from "solid-js";
import type {
  CompositionSummary,
  TimelineLayerSummary,
  TimelineVideoLayerRef,
  VideoLayerSummary,
} from "../types";

type MaybePromise = void | Promise<unknown>;

type VideoCompositionSetupPanelProps = {
  compositions: CompositionSummary[];
  layers: VideoLayerSummary[];
  timelineId: number;
  timelineLayers: TimelineLayerSummary[];
  draftLabel: string;
  draftLayerIds: number[];
  onDraftLabel: (value: string) => void;
  onToggleDraftLayer: (layerId: number, checked: boolean) => void;
  onAddComposition: () => MaybePromise;
  onRemoveComposition: (compositionId: number) => MaybePromise;
  onSetCompositionLayers: (compositionId: number, layerIds: number[]) => MaybePromise;
  onSetCompositionTimelineLayers: (
    compositionId: number,
    timelineLayerIds: TimelineVideoLayerRef[],
  ) => MaybePromise;
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
  const timelineVideoLayers = () => props.timelineLayers.filter((layer) => layer.kind === "Video");
  const timelineLayerIds = (composition: CompositionSummary) => composition.timeline_layer_ids ?? [];
  const isCurrentTimelineLayer = (timelineLayerIds: TimelineVideoLayerRef[], layerId: number) =>
    timelineLayerIds.some(
      (candidate) => candidate.timeline_id === props.timelineId && candidate.layer_id === layerId,
    );

  return (
    <>
      <div class="compositionList">
        <For each={props.compositions}>
          {(composition) => (
            <div class="compositionItem">
              <div class="compositionHeader">
                <strong data-no-localize>{composition.label}</strong>
                <span>{timelineLayerIds(composition).length} Timeline Video lane(s) + {composition.layer_ids.length} fixed video layer(s) / {composition.output_ids.length} video output(s)</span>
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
                  <strong>Timeline Video — rendered below fixed video layers</strong>
                  <For each={timelineVideoLayers()}>
                    {(timelineLayer) => (
                      <label class="checkbox">
                        <input
                          type="checkbox"
                          checked={isCurrentTimelineLayer(timelineLayerIds(composition), timelineLayer.id)}
                          onChange={(event) => {
                            const nextTimelineLayerIds = event.currentTarget.checked
                              ? [
                                  ...timelineLayerIds(composition),
                                  { timeline_id: props.timelineId, layer_id: timelineLayer.id },
                                ]
                              : timelineLayerIds(composition).filter(
                                  (candidate) =>
                                    candidate.timeline_id !== props.timelineId
                                      || candidate.layer_id !== timelineLayer.id,
                                );
                            void props.onSetCompositionTimelineLayers(composition.id, nextTimelineLayerIds);
                          }}
                        />
                        <span data-no-localize>{timelineLayer.label}</span>
                      </label>
                    )}
                  </For>
                </div>
                <div class="compositionLayerPicker">
                  <strong>Fixed video layers — rendered above Timeline Video</strong>
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
                        <span data-no-localize>{layer.label}</span>
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
      <details class="videoSetupDisclosure">
        <summary>New Composition</summary>
        <div class="videoCompositionForm">
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
                <span data-no-localize>{layer.label}</span>
              </label>
            )}
          </For>
        </div>
        <button class="primary" onClick={() => void props.onAddComposition()}>
          Add Composition
        </button>
        </div>
      </details>
    </>
  );
}
