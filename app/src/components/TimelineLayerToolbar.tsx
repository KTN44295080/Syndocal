import { createEffect, createMemo, createSignal, createUniqueId, For, Show } from "solid-js";
import type { TimelineLayerKind, TimelineLayerSummary } from "../types";

const timelineLayerKinds: TimelineLayerKind[] = ["Audio", "Lighting", "Video"];

export interface TimelineLayerToolbarProps {
  layers: TimelineLayerSummary[];
  legacyMode: boolean;
  eventCountForLayer: (layerId: number) => number;
  onAddLayer: (label: string, kind: TimelineLayerKind) => void | Promise<void>;
  onUpdateLayer: (layer: TimelineLayerSummary) => void | Promise<void>;
  onRemoveLayer: (layerId: number, reassignToLayerId: number | null) => void | Promise<void>;
  onReorderLayer: (layerId: number, direction: -1 | 1) => void | Promise<void>;
}

export function TimelineLayerToolbar(props: TimelineLayerToolbarProps) {
  const [newLayerKind, setNewLayerKind] = createSignal<TimelineLayerKind>("Lighting");
  const [newLayerLabel, setNewLayerLabel] = createSignal("");
  const [selectedLayerId, setSelectedLayerId] = createSignal<number | null>(null);
  const [pendingRemoveLayerId, setPendingRemoveLayerId] = createSignal<number | null>(null);
  const [reassignTargetLayerId, setReassignTargetLayerId] = createSignal<number | null>(null);
  const removeTitleId = `${createUniqueId()}-timeline-layer-remove-title`;
  const removeDescriptionId = `${createUniqueId()}-timeline-layer-remove-description`;
  let removeDialog: HTMLDialogElement | undefined;

  const orderedLayers = createMemo(() => [...props.layers].sort(
    (left, right) => left.order - right.order || left.id - right.id,
  ));
  const selectedLayer = createMemo(() => {
    const layerId = selectedLayerId();
    return layerId === null ? undefined : orderedLayers().find((layer) => layer.id === layerId);
  });
  const selectedKindLayers = createMemo(() => {
    const layer = selectedLayer();
    return layer ? orderedLayers().filter((candidate) => candidate.kind === layer.kind) : [];
  });
  const selectedKindIndex = createMemo(() => {
    const layer = selectedLayer();
    return layer ? selectedKindLayers().findIndex((candidate) => candidate.id === layer.id) : -1;
  });
  const canMoveSelectedUp = createMemo(() => selectedKindIndex() > 0);
  const canMoveSelectedDown = createMemo(() => {
    const index = selectedKindIndex();
    return index >= 0 && index < selectedKindLayers().length - 1;
  });

  const pendingRemoveLayer = createMemo(() => {
    const layerId = pendingRemoveLayerId();
    return layerId === null ? undefined : orderedLayers().find((layer) => layer.id === layerId);
  });
  const pendingRemoveEventCount = createMemo(() => {
    const layer = pendingRemoveLayer();
    return layer ? Math.max(0, props.eventCountForLayer(layer.id)) : 0;
  });
  const reassignCandidates = createMemo(() => {
    const layer = pendingRemoveLayer();
    if (!layer) return [];
    return orderedLayers().filter((candidate) =>
      candidate.id !== layer.id && candidate.kind !== "Audio" && !candidate.locked);
  });
  const canConfirmRemoval = createMemo(() => {
    const layer = pendingRemoveLayer();
    if (!layer || layer.locked || orderedLayers().length <= 1) return false;
    return pendingRemoveEventCount() === 0 || reassignTargetLayerId() !== null;
  });

  createEffect(() => {
    const layers = orderedLayers();
    const currentLayerId = selectedLayerId();
    if (props.legacyMode || layers.length === 0) {
      if (currentLayerId !== null) setSelectedLayerId(null);
      return;
    }
    if (currentLayerId === null || !layers.some((layer) => layer.id === currentLayerId)) {
      setSelectedLayerId(layers[0].id);
    }
  });

  createEffect(() => {
    const pendingLayer = pendingRemoveLayer();
    if (pendingRemoveLayerId() !== null && !pendingLayer) {
      if (removeDialog?.open) removeDialog.close();
      setPendingRemoveLayerId(null);
      setReassignTargetLayerId(null);
      return;
    }
    if (!pendingLayer || pendingRemoveEventCount() === 0) {
      if (reassignTargetLayerId() !== null) setReassignTargetLayerId(null);
      return;
    }
    const candidates = reassignCandidates();
    if (!candidates.some((candidate) => candidate.id === reassignTargetLayerId())) {
      setReassignTargetLayerId(candidates[0]?.id ?? null);
    }
  });

  const suggestedLayerLabel = () => {
    const kind = newLayerKind();
    const nextNumber = orderedLayers().filter((layer) => layer.kind === kind).length + 1;
    return `${kind} ${nextNumber}`;
  };

  const addLayer = async () => {
    const label = newLayerLabel().trim() || suggestedLayerLabel();
    await props.onAddLayer(label, newLayerKind());
    setNewLayerLabel("");
  };

  const updateSelectedLock = (locked: boolean) => {
    const layer = selectedLayer();
    if (layer) void props.onUpdateLayer({ ...layer, locked });
  };

  const reorderSelected = (direction: -1 | 1) => {
    const layer = selectedLayer();
    if (!layer) return;
    if (direction === -1 && !canMoveSelectedUp()) return;
    if (direction === 1 && !canMoveSelectedDown()) return;
    void props.onReorderLayer(layer.id, direction);
  };

  const openRemoveDialog = () => {
    const layer = selectedLayer();
    if (!layer || layer.locked || orderedLayers().length <= 1) return;
    setPendingRemoveLayerId(layer.id);
    const firstTarget = orderedLayers().find((candidate) =>
      candidate.id !== layer.id && candidate.kind === layer.kind && !candidate.locked);
    setReassignTargetLayerId(firstTarget?.id ?? null);
    if (!removeDialog?.open) removeDialog?.showModal();
  };

  const confirmRemoval = () => {
    const layer = pendingRemoveLayer();
    if (!layer || !canConfirmRemoval()) return;
    const reassignToLayerId = pendingRemoveEventCount() > 0 ? reassignTargetLayerId() : null;
    removeDialog?.close();
    void props.onRemoveLayer(layer.id, reassignToLayerId);
  };

  const removeButtonLabel = () => {
    const layer = selectedLayer();
    if (orderedLayers().length <= 1) return "Timeline must keep at least one layer";
    return layer?.locked ? "Unlock selected layer before removing" : "Remove selected layer";
  };

  return (
    <div
      class="timelineLayerToolbar"
      data-timeline-layer-toolbar
      data-timeline-layer-legacy={props.legacyMode ? "true" : "false"}
      aria-label="Timeline layer controls"
    >
      <div class="timelineLayerAddControls" data-timeline-layer-add-controls role="group" aria-label="Add timeline layer">
        <label class="timelineLayerKindField">
          <span>Kind</span>
          <select
            data-timeline-layer-add-kind
            value={newLayerKind()}
            onInput={(event) => setNewLayerKind(event.currentTarget.value as TimelineLayerKind)}
          >
            <For each={timelineLayerKinds}>{(kind) => <option value={kind}>{kind}</option>}</For>
          </select>
        </label>
        <label class="timelineLayerNameField">
          <span>Name (optional)</span>
          <input
            data-timeline-layer-add-name
            maxlength="64"
            value={newLayerLabel()}
            placeholder="Optional layer name"
            onInput={(event) => setNewLayerLabel(event.currentTarget.value)}
            onKeyDown={(event) => {
              if (event.key !== "Enter") return;
              event.preventDefault();
              void addLayer();
            }}
          />
        </label>
        <button type="button" data-timeline-layer-add onClick={() => void addLayer()}>
          Add Layer
        </button>
      </div>

      <Show when={!props.legacyMode}>
        <div
          class="timelineLayerSelectionControls"
          data-timeline-layer-selection-controls
          role="group"
          aria-label="Selected timeline layer controls"
        >
          <label class="timelineLayerSelectionField">
            <span>Layer</span>
            <select
              data-timeline-selected-layer
              value={selectedLayerId() ?? ""}
              disabled={orderedLayers().length === 0}
              onInput={(event) => setSelectedLayerId(Number(event.currentTarget.value))}
            >
              <For each={orderedLayers()}>
                {(layer) => (
                  <option value={layer.id} data-no-localize>
                    {layer.label}
                  </option>
                )}
              </For>
            </select>
          </label>
          <Show when={selectedLayer()}>
            {(layer) => (
              <label class="timelineLayerToolbarLock">
                <input
                  type="checkbox"
                  data-timeline-layer-toolbar-lock-toggle
                  checked={layer().locked}
                  onChange={(event) => updateSelectedLock(event.currentTarget.checked)}
                />
                Layer locked
              </label>
            )}
          </Show>
          <button
            type="button"
            data-timeline-layer-reorder="up"
            disabled={!canMoveSelectedUp()}
            title="Move selected layer up within its section"
            aria-label="Move selected layer up within its section"
            onClick={() => reorderSelected(-1)}
          >
            <span aria-hidden="true" data-no-localize>↑</span>
          </button>
          <button
            type="button"
            data-timeline-layer-reorder="down"
            disabled={!canMoveSelectedDown()}
            title="Move selected layer down within its section"
            aria-label="Move selected layer down within its section"
            onClick={() => reorderSelected(1)}
          >
            <span aria-hidden="true" data-no-localize>↓</span>
          </button>
          <button
            type="button"
            class="danger"
            data-timeline-layer-remove
            disabled={!selectedLayer() || selectedLayer()!.locked || orderedLayers().length <= 1}
            title={removeButtonLabel()}
            aria-label={removeButtonLabel()}
            onClick={openRemoveDialog}
          >
            <span aria-hidden="true" data-no-localize>−</span>
          </button>
        </div>
      </Show>

      <dialog
        ref={(element) => { removeDialog = element; }}
        class="timelineLayerRemoveDialog"
        data-timeline-layer-remove-dialog
        aria-labelledby={removeTitleId}
        aria-describedby={removeDescriptionId}
        onClose={() => {
          setPendingRemoveLayerId(null);
          setReassignTargetLayerId(null);
        }}
      >
        <form method="dialog">
          <h2 id={removeTitleId} class="textBalance">Remove Timeline Layer?</h2>
          <div id={removeDescriptionId} class="textPretty">
            <Show when={pendingRemoveLayer()}>
              {(layer) => (
                <>
                  <dl class="timelineLayerRemoveSummary">
                    <div>
                      <dt>Layer</dt>
                      <dd data-no-localize>{layer().label}</dd>
                    </div>
                    <div>
                      <dt>Timeline blocks</dt>
                      <dd class="tabularNums" data-no-localize>{pendingRemoveEventCount()}</dd>
                    </div>
                  </dl>
                  <Show
                    when={!layer().locked}
                    fallback={<p class="validationError">Unlock this layer before removing it.</p>}
                  >
                    <Show
                      when={pendingRemoveEventCount() > 0}
                      fallback={<p>This layer is empty and can be removed directly.</p>}
                    >
                      <Show
                        when={reassignCandidates().length > 0}
                        fallback={
                          <p class="validationError">
                            Add another unlocked Lighting or Video layer, or remove this layer's blocks first.
                          </p>
                        }
                      >
                        <p>This layer is not empty. Reassign its blocks before removal.</p>
                        <label class="timelineLayerReassignField">
                          <span>Reassign blocks to</span>
                          <select
                            data-timeline-layer-reassign-target
                            value={reassignTargetLayerId() ?? ""}
                            onInput={(event) => setReassignTargetLayerId(Number(event.currentTarget.value))}
                          >
                            <For each={reassignCandidates()}>
                              {(candidate) => <option value={candidate.id} data-no-localize>{candidate.label}</option>}
                            </For>
                          </select>
                        </label>
                      </Show>
                    </Show>
                  </Show>
                </>
              )}
            </Show>
          </div>
          <div class="buttonRow">
            <button value="cancel">Cancel</button>
            <button
              type="button"
              class="danger"
              data-timeline-layer-remove-confirm
              disabled={!canConfirmRemoval()}
              onClick={confirmRemoval}
            >
              Remove Layer
            </button>
          </div>
        </form>
      </dialog>
    </div>
  );
}
