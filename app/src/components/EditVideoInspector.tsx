import { createMemo, createSignal, For, Show, type ComponentProps } from "solid-js";
import type { MediaAssetAvailability, MediaAssetSummary } from "../types";
import { VideoIsfEffectPanel } from "./VideoIsfEffectPanel";
import type { VideoLayerListPanel } from "./VideoLayerListPanel";
import "./EditVideoInspector.css";

type LayerFxProps = Pick<ComponentProps<typeof VideoLayerListPanel>,
  "layers" | "isfRuntimeErrors" | "isfEventPulseBusy" | "onImportIsf" |
  "onApplyBuiltinIsf" | "onSetIsfEffect" | "onMoveIsfEffect" |
  "onRemoveIsfEffect" | "onSetIsfEffectEnabled" | "onResetIsfEffect" |
  "onSetIsfControl" | "onTriggerIsfEvent">;

interface EditVideoInspectorProps {
  asset: MediaAssetSummary | null | undefined;
  availabilityById: Record<number, MediaAssetAvailability>;
  layerFx: LayerFxProps;
}

export function EditVideoInspector(props: EditVideoInspectorProps) {
  const [advancedOpen, setAdvancedOpen] = createSignal(false);
  const [selectedLayerId, setSelectedLayerId] = createSignal<number | null>(null);
  const selectedLayer = createMemo(() => props.layerFx.layers.find(
    (layer) => layer.id === selectedLayerId(),
  ) ?? props.layerFx.layers[0]);

  return (
    <section class="editVideoInspectorPane" data-edit-video-inspector aria-label="Media properties and layer effects">
      <header class="panelHeader"><h2>Media Properties</h2></header>
      <div class="editVideoInspectorContent">
        <Show when={props.asset} fallback={<div class="emptyState">Select a Media Library item to inspect its properties.</div>}>
          {(asset) => <>
            <strong data-no-localize>{asset().label}</strong>
            <dl>
              <div><dt>Source</dt><dd data-no-localize>{asset().source.kind}</dd></div>
              <div><dt>Availability</dt><dd data-no-localize>{props.availabilityById[asset().id]?.kind ?? "Not checked"}</dd></div>
            </dl>
          </>}
        </Show>
        <details class="editVideoAdvancedDisclosure" onToggle={(event) => setAdvancedOpen(event.currentTarget.open)}>
          <summary>Advanced Video Controls</summary>
          <Show when={advancedOpen()}>
            <div class="editVideoLayerFx" data-edit-video-layer-fx>
              <p>Layer effects apply to the selected layer, independently of the selected media asset.</p>
              <Show when={selectedLayer()} fallback={<div class="emptyState">Add a layer in Setup Video to edit its effects.</div>}>
                <label>
                  Video FX layer
                  <select data-edit-video-fx-layer aria-label="Video FX layer" value={selectedLayer()?.id ?? ""}
                    onChange={(event) => setSelectedLayerId(Number(event.currentTarget.value))}>
                    <For each={props.layerFx.layers}>{(layer) => <option value={layer.id} data-no-localize>{layer.label}</option>}</For>
                  </select>
                </label>
                <Show when={selectedLayer()?.id} keyed>
                  {(layerId) => <VideoIsfEffectPanel
                    layerId={layerId}
                    layerLabel={selectedLayer()?.label}
                    compact
                    effect={selectedLayer()?.isf_effect}
                    runtimeErrors={props.layerFx.isfRuntimeErrors?.filter((error) => error.layer_id === layerId) ?? []}
                    eventPulseBusy={props.layerFx.isfEventPulseBusy}
                    onImport={props.layerFx.onImportIsf}
                    onApplyBuiltin={props.layerFx.onApplyBuiltinIsf}
                    onSetEffect={props.layerFx.onSetIsfEffect}
                    onMoveEffect={props.layerFx.onMoveIsfEffect}
                    onRemoveEffect={props.layerFx.onRemoveIsfEffect}
                    onSetEffectEnabled={props.layerFx.onSetIsfEffectEnabled}
                    onResetEffect={props.layerFx.onResetIsfEffect}
                    onSetControl={props.layerFx.onSetIsfControl}
                    onTriggerEvent={props.layerFx.onTriggerIsfEvent}
                  />}
                </Show>
              </Show>
            </div>
          </Show>
        </details>
      </div>
    </section>
  );
}
