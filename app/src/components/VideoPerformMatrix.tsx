import { createEffect, createMemo, createSignal, For, onCleanup, onMount, Show, type ComponentProps } from "solid-js";
import { createStore, reconcile } from "solid-js/store";
import { controlMappingTargetData } from "../controlMappingLearn";
import type { MediaAssetAvailability, VideoBlendMode, VideoClipLayerRuntimeSummary, VideoLayerSummary } from "../types";
import { videoClipSlotBankCells } from "../videoClipSlotBankModel";
import { VideoClipSlotBankPanel } from "./VideoClipSlotBankPanel";
import { VideoLayerListPanel } from "./VideoLayerListPanel";
import "./VideoPerformMatrix.css";

export interface VideoPerformMatrixProps {
  bank: ComponentProps<typeof VideoClipSlotBankPanel>;
  layerList: ComponentProps<typeof VideoLayerListPanel>;
  runtimes: VideoClipLayerRuntimeSummary[];
  availabilityById: Record<number, MediaAssetAvailability>;
  previewLayerId: number | null;
  thumbnailsAuthorized: boolean;
  onStop: (layerId: number) => void | Promise<unknown>;
  keyboardEnabled?: boolean;
  onTake: () => void | Promise<unknown>;
}

export const videoPerformClipKeys = "1234567890qwertyuiopasdfghjklzxc";
const blends: VideoBlendMode[] = ["Normal", "Add", "Multiply", "Screen", "Overlay", "Subtract", "Difference", "Lighten", "Darken", "SoftLight"];

/** A presentation of the canonical clip bank and existing layer commands. Runtime
 * IDs remain the sole source of queued/live state. Reconciliation retains the
 * focused row and pad while the runtime publishes replacement snapshots. */
export function VideoPerformMatrix(props: VideoPerformMatrixProps) {
  const [rows, setRows] = createStore<{ id: number; layer: VideoLayerSummary }[]>([]);
  createEffect(() => setRows(reconcile(props.bank.layers.map((layer) => ({ id: layer.id, layer })), { key: "id", merge: true })));
  const [focusedColumn, setFocusedColumn] = createSignal(0);
  // Existing Preview authorizes a thumbnail audition, not an isolated clip
  // transport. Keep that explicit identity separate from queue selection.
  const [inspectedSlot, setInspectedSlot] = createSignal<{ layerId: number; slotId: number } | null>(null);
  let root: HTMLDivElement | undefined;
  const selectedLayerId = () => props.bank.layer?.id ?? rows[0]?.id ?? null;

  const focusCell = (layerId: number, column: number) => {
    setFocusedColumn(column);
    props.bank.onSelectLayer(layerId);
    const layer = rows.find((row) => row.id === layerId)?.layer;
    props.bank.onSelect(layer?.clip_slots?.[column]?.id ?? null);
    queueMicrotask(() => {
      const pad = root?.querySelector<HTMLButtonElement>(`[data-video-perform-layer-id="${layerId}"][data-video-perform-cell-index="${column}"]`);
      if (!pad || !root) return;
      pad.focus({ preventScroll: true });
      const bounds = root.getBoundingClientRect();
      const target = pad.getBoundingClientRect();
      const stickyWidth = root.querySelector<HTMLElement>(".videoPerformMatrixCorner")?.offsetWidth ?? 300;
      const stickyHeight = root.querySelector<HTMLElement>(".videoPerformMatrixHead")?.offsetHeight ?? 36;
      if (target.left < bounds.left + stickyWidth) root.scrollLeft -= bounds.left + stickyWidth - target.left;
      else if (target.right > bounds.right - 2) root.scrollLeft += target.right - bounds.right + 2;
      const row = pad.closest<HTMLElement>("[data-video-perform-row]")?.getBoundingClientRect();
      if (row && row.top < bounds.top + stickyHeight) root.scrollTop -= bounds.top + stickyHeight - row.top;
      else if (row && row.bottom > bounds.bottom - 2) root.scrollTop += row.bottom - bounds.bottom + 2;
    });
  };

  const queue = (layerId: number, column: number) => {
    const slot = rows.find((row) => row.id === layerId)?.layer.clip_slots?.[column];
    props.bank.onSelectLayer(layerId);
    props.bank.onSelect(slot?.id ?? null);
    setFocusedColumn(column);
    if (slot) void props.bank.onQueue(slot.id);
  };

  const keyDown = (event: KeyboardEvent) => {
    if (props.keyboardEnabled === false || event.defaultPrevented || event.repeat || event.isComposing
      || event.ctrlKey || event.metaKey || event.altKey || event.shiftKey) return;
    const target = event.target instanceof Element ? event.target : null;
    if (target?.closest("input, textarea, select, [contenteditable]:not([contenteditable='false'])")
      || document.querySelector("[role='dialog'], dialog[open], [aria-modal='true'], .app[data-control-learn-mode]")) return;
    const layerId = selectedLayerId();
    if (layerId === null) return;
    const rowIndex = Math.max(0, rows.findIndex((row) => row.id === layerId));
    const targetPad = target?.closest<HTMLElement>("[data-video-perform-cell-index]");
    const column = targetPad ? Number(targetPad.dataset.videoPerformCellIndex) : focusedColumn();
    let nextColumn = column;
    let nextRow = rowIndex;
    if (event.key === "ArrowLeft") nextColumn = Math.max(0, column - 1);
    else if (event.key === "ArrowRight") nextColumn = Math.min(31, column + 1);
    else if (event.key === "ArrowUp") nextRow = Math.max(0, rowIndex - 1);
    else if (event.key === "ArrowDown") nextRow = Math.min(rows.length - 1, rowIndex + 1);
    else if (event.key === "Home") nextColumn = 0;
    else if (event.key === "End") nextColumn = 31;
    else if (event.key === " " || event.key === "Enter") {
      // Native button activation still owns explicit Preview/Solo/Stop/etc.
      if (target?.closest("button, summary, a") && !targetPad) return;
      event.preventDefault();
      event.stopImmediatePropagation();
      void props.onTake();
      return;
    } else {
      const keyIndex = event.key.length === 1 ? videoPerformClipKeys.indexOf(event.key.toLowerCase()) : -1;
      if (keyIndex < 0) return;
      event.preventDefault();
      event.stopImmediatePropagation();
      focusCell(layerId, keyIndex);
      queue(layerId, keyIndex);
      return;
    }
    event.preventDefault();
    event.stopImmediatePropagation();
    focusCell(rows[nextRow].id, nextColumn);
  };
  onMount(() => window.addEventListener("keydown", keyDown, true));
  onCleanup(() => window.removeEventListener("keydown", keyDown, true));

  const Row = (rowProps: { layer: VideoLayerSummary }) => {
    const layer = () => rowProps.layer;
    const runtime = createMemo(() => props.runtimes.find((item) => item.layer_id === layer().id) ?? null);
    const [cells, setCells] = createStore(videoClipSlotBankCells(layer().clip_slots, runtime()));
    createEffect(() => setCells(reconcile(videoClipSlotBankCells(layer().clip_slots, runtime()), { key: "index" })));
    const selected = () => selectedLayerId() === layer().id;
    const live = () => runtime()?.active_slot_id != null && runtime()?.playing === true;
    return <div class="videoPerformMatrixRow" classList={{ selected: selected() }} data-video-perform-row={layer().id} data-video-clip-slot-layer-id={layer().id} role="row" aria-selected={selected()}>
      <div class="videoPerformLayerStrip" role="rowheader">
        <div class="videoPerformLayerHeading">
          <button type="button" class="videoPerformLayerName" aria-pressed={selected()} title={layer().label} data-video-perform-select-layer={layer().id} onClick={() => props.bank.onSelectLayer(layer().id)} data-no-localize>{layer().label}</button>
          <span class="videoPerformLayerLive" classList={{ live: live() }} data-video-perform-layer-live={live() ? "true" : "false"}><span aria-hidden="true">{live() ? "▶" : "■"}</span><span>{live() ? "LIVE" : runtime()?.active_slot_id != null ? "Paused" : "Stopped"}</span></span>
        </div>
        <div class="videoPerformLayerMix">
          <label class="videoPerformLayerOpacity"><span>Opacity <strong data-no-localize>{Math.round(layer().state.opacity * 100)}%</strong></span><input type="range" min="0" max="1" step="0.01" value={layer().state.opacity} aria-label="Opacity" data-video-layer-opacity={layer().id} {...controlMappingTargetData({ action: "VideoParam", label: `${layer().label} opacity`, layer_id: layer().id, video_param: "Opacity" })} onInput={(event) => void props.layerList.onSetLayerState(layer().id, { ...layer().state, opacity: Number(event.currentTarget.value) })} /></label>
          <label class="videoPerformLayerBlend"><span>Blend</span><select value={layer().blend_mode} aria-label="Blend" data-video-layer-blend={layer().id} onInput={(event) => void props.layerList.onSetBlendMode(layer().id, event.currentTarget.value as VideoBlendMode)}><For each={blends}>{(blend) => <option value={blend}>{blend === "SoftLight" ? "Soft Light" : blend}</option>}</For></select></label>
        </div>
        <div class="videoPerformLayerButtons">
          <button type="button" aria-pressed={layer().state.solo} classList={{ active: layer().state.solo }} data-video-layer-solo={layer().id} {...controlMappingTargetData({ action: "VideoLayerSolo", label: `${layer().label} solo`, layer_id: layer().id })} onClick={() => void props.layerList.onSetLayerState(layer().id, { ...layer().state, solo: !layer().state.solo })}>Solo</button>
          <button type="button" aria-pressed={!layer().state.enabled} classList={{ active: !layer().state.enabled }} data-video-layer-bypass={layer().id} {...controlMappingTargetData({ action: "VideoLayerEnabled", label: `${layer().label} enabled`, layer_id: layer().id })} onClick={() => void props.layerList.onSetLayerState(layer().id, { ...layer().state, enabled: !layer().state.enabled })}>Bypass</button>
          <button type="button" data-video-layer-stop={layer().id} onClick={() => void props.onStop(layer().id)}>Stop</button>
        </div>
      </div>
      <For each={cells}>{(cell) => {
        const asset = () => props.bank.assets.find((item) => item.id === cell.slot?.media_asset_id);
        const availability = () => cell.slot ? props.availabilityById[cell.slot.media_asset_id] : undefined;
        const missing = () => cell.slot !== null && (!asset() || availability()?.kind === "missing");
        const failed = () => ["unreadable", "hash_mismatch"].includes(availability()?.kind ?? "");
        const inspected = () => cell.slot != null && inspectedSlot()?.layerId === layer().id && inspectedSlot()?.slotId === cell.slot.id;
        const state = () => !cell.slot ? "Empty" : missing() ? "Missing" : failed() ? "Error" : cell.active ? (runtime()?.playing ? "LIVE" : "Paused") : cell.queued || cell.pending ? "Queued" : inspected() ? "Preview" : "Loaded";
        const selectedCell = () => selected() && (cell.slot ? cell.slot.id === props.bank.selectedSlotId : focusedColumn() === cell.index && props.bank.selectedSlotId === null);
        return <div class="videoPerformMatrixCell" classList={{ occupied: cell.slot !== null, live: cell.active && runtime()?.playing === true, queued: cell.queued || cell.pending, preview: inspected(), selected: selectedCell(), error: missing() || failed() }} role="gridcell" data-video-clip-slot-id={cell.slot?.id} data-video-clip-slot-index={cell.index} data-video-clip-slot-layer-id={layer().id} data-video-perform-cell-state={state()}>
          <button type="button" class="videoPerformClipTrigger" data-video-perform-layer-id={layer().id} data-video-perform-cell-index={cell.index} data-video-clip-slot-pad-index={cell.index} aria-label={`Queue clip ${cell.index + 1}: ${asset()?.label ?? "Empty"}`} aria-pressed={selectedCell()} tabIndex={selectedCell() || selected() && props.bank.selectedSlotId == null && cell.index === focusedColumn() ? 0 : -1} onFocus={() => setFocusedColumn(cell.index)} onClick={() => queue(layer().id, cell.index)}>
            <span class="videoPerformClipImage"><Show when={props.thumbnailsAuthorized && cell.slot && props.bank.thumbnails[cell.slot.media_asset_id]} fallback={<span class="videoPerformClipPlaceholder" aria-hidden="true">{cell.slot ? "▧" : "+"}</span>}>{(thumbnail) => <img src={thumbnail()} alt="" loading="lazy" />}</Show><kbd data-no-localize>{videoPerformClipKeys[cell.index].toUpperCase()}</kbd><span class="videoPerformClipNumber" data-no-localize>{cell.index + 1}</span></span>
            <span class="videoPerformClipTitle" title={asset()?.label} data-no-localize>{cell.slot ? asset()?.label ?? `#${cell.slot.id}` : "—"}</span>
            <span class="videoPerformClipState"><span aria-hidden="true">{state() === "LIVE" ? "▶ " : state() === "Queued" ? "◷ " : missing() || failed() ? "! " : ""}</span><span>{state()}</span></span>
          </button>
          <div class="videoPerformClipActions">
            <button type="button" disabled={!cell.slot} title="Preview thumbnail" onClick={() => { const slot = cell.slot; if (!slot) return; props.bank.onSelectLayer(layer().id); props.bank.onSelect(slot.id); setInspectedSlot({ layerId: layer().id, slotId: slot.id }); void props.bank.onPreview(slot.id); }}>Preview</button>
            <button type="button" aria-label={`More actions for clip ${cell.index + 1}`} onClick={(event) => { props.bank.onSelectLayer(layer().id); props.bank.onSelect(cell.slot?.id ?? null); props.bank.onMore(cell.slot?.id ?? null, event.currentTarget); }}>More</button>
          </div>
        </div>;
      }}</For>
    </div>;
  };

  return <section class="videoPerformMatrix" data-video-perform-matrix data-video-perform-region="matrix" aria-label="Clip matrix">
    <Show when={rows.length > 0} fallback={<div class="videoPerformMatrixEmpty"><span>No video layers. Add a file, still, or input above.</span><button type="button" onClick={() => void props.bank.onImport()}>Import</button></div>}>
      <div ref={root} class="videoPerformMatrixScroll" role="grid" aria-label="Video layers and clips" aria-rowcount={rows.length + 1} aria-colcount="33" tabIndex={-1}>
        <div class="videoPerformMatrixHead" role="row"><div class="videoPerformMatrixCorner" role="columnheader"><strong>Layers / Clips</strong><span data-no-localize>{rows.length} × 32</span></div><For each={Array.from({ length: 32 }, (_, index) => index)}>{(index) => <div class="videoPerformMatrixColumn" role="columnheader"><span data-no-localize>{index + 1}</span><kbd data-no-localize>{videoPerformClipKeys[index].toUpperCase()}</kbd></div>}</For></div>
        <For each={rows}>{(row) => <Row layer={row.layer} />}</For>
      </div>
    </Show>
  </section>;
}
