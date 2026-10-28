import { videoSourceKindLabel } from "../videoHelpers";
import { createEffect, createMemo, createSignal, For, on, onMount, Show, type ComponentProps } from "solid-js";
import type { MediaAssetAvailability, MediaAssetSummary } from "../types";
import { VideoIsfEffectPanel } from "./VideoIsfEffectPanel";
import type { VideoLayerListPanel } from "./VideoLayerListPanel";
import "./EditVideoInspector.css";
import { VideoLayerGroupsPanel } from "./VideoLayerGroupsPanel";
import type { VideoControlPanelProps } from "./VideoControlPanel";

type LayerFxProps = Pick<ComponentProps<typeof VideoLayerListPanel>,
  "layers" | "bpm" | "locale" | "projectEpoch" | "effectCatalog" | "isfRuntimeErrors" | "isfEventPulseBusy" | "onImportIsf" |
  "onApplyBuiltinIsf" | "onSetIsfEffect" | "onMoveIsfEffect" |
  "onRemoveIsfEffect" | "onSetIsfEffectEnabled" | "onResetIsfEffect" |
  "onSetIsfControl" | "onTriggerIsfEvent">;

interface EditVideoInspectorProps {
  asset: MediaAssetSummary | null | undefined;
  availabilityById: Record<number, MediaAssetAvailability>;
  layerFx: LayerFxProps;
  groupEffects?: VideoControlPanelProps["effectScopes"];
  /** The application's selected video layer (the one Perform and the clip bank act on). */
  selectedLayerId?: number | null;
  /** Perform's "Full FX editor" asked for the layer effects to be shown open. */
  openFxEditor?: boolean;
  onFxEditorOpened?: () => void;
}

export function EditVideoInspector(props: EditVideoInspectorProps) {
  const [advancedOpen, setAdvancedOpen] = createSignal(false);
  // A local pick in the FX layer menu lasts until the application selects
  // another layer; otherwise the editor targets the application's selection.
  const [selectedLayerId, setSelectedLayerId] = createSignal<number | null>(null);
  // Memoised so a snapshot refresh that keeps the same layer id keeps the pick.
  const applicationLayerId = createMemo(() => props.selectedLayerId ?? null);
  createEffect(on(applicationLayerId, () => setSelectedLayerId(null), { defer: true }));
  const layerById = (id: number | null | undefined) =>
    id === null || id === undefined ? undefined : props.layerFx.layers.find((layer) => layer.id === id);
  const selectedLayer = createMemo(() =>
    layerById(selectedLayerId()) ?? layerById(applicationLayerId()) ?? props.layerFx.layers[0]);
  let advanced: HTMLDetailsElement | undefined;
  onMount(() => {
    if (!props.openFxEditor || !advanced) return;
    advanced.open = true;
    setAdvancedOpen(true);
    props.onFxEditorOpened?.();
    queueMicrotask(() => advanced?.querySelector<HTMLElement>("[data-edit-video-layer-fx]")?.scrollIntoView({ block: "nearest" }));
  });

  return (
    <section class="editVideoInspectorPane" data-edit-video-inspector aria-label="Media properties and layer effects">
      <header class="panelHeader"><h2>Media Properties</h2></header>
      <div class="editVideoInspectorContent">
        <Show when={props.asset} fallback={<div class="emptyState">Select a Media Library item to inspect its properties.</div>}>
          {(asset) => <>
            <strong data-no-localize>{asset().label}</strong>
            <dl>
              <div><dt>Source</dt><dd data-no-localize>{videoSourceKindLabel(asset().source.kind)}</dd></div>
              <div><dt>Availability</dt><dd data-no-localize>{props.availabilityById[asset().id]?.kind ?? "Not checked"}</dd></div>
            </dl>
          </>}
        </Show>
        <Show when={props.groupEffects}>{effects => <VideoLayerGroupsPanel effects={effects()} layers={props.layerFx.layers} />}</Show>
        <details ref={advanced} class="editVideoAdvancedDisclosure" onToggle={(event) => setAdvancedOpen(event.currentTarget.open)}>
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
                    availableLayers={props.layerFx.layers} locale={props.layerFx.locale}
                    projectEpoch={props.layerFx.projectEpoch} effectCatalog={props.layerFx.effectCatalog}
                    layerId={layerId}
                    layerLabel={selectedLayer()?.label}
                    compact
                    bpm={props.layerFx.bpm}
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
