import { createEffect, createMemo, createSignal, For, onCleanup, onMount, Show, type ComponentProps } from "solid-js";
import { createStore, reconcile } from "solid-js/store";
import { Portal } from "solid-js/web";
import { controlMappingTargetData } from "../controlMappingLearn";
import type { MediaAssetAvailability, VideoBlendMode, VideoClipLayerRuntimeSummary, VideoClipSlotId, VideoLayerSummary, VideoLayerGroupSummary, CompositionSummary } from "../types";
import { videoClipSlotBankCells } from "../videoClipSlotBankModel";
import { VideoClipSlotBankPanel } from "./VideoClipSlotBankPanel";
import { defaultVideoGroupMix, videoGroupBlends } from "../videoLayerGroups";
import { VideoLayerListPanel } from "./VideoLayerListPanel";
import { confirmDialog } from "../inAppDialog";
import { addEffectToLiveLayerDialog } from "../inAppDialogCopy";
import "./VideoPerformMatrix.css";
import { FX_DRAG_MIME, readFxDragPayload, type FxCatalogEntry } from "../fxCatalogModel";

export interface VideoPerformMatrixProps {
  groups?: VideoLayerGroupSummary[];
  compositions?: CompositionSummary[];
  selectedGroupId?: number | null;
  onSelectGroup?: (groupId: number | null) => void;
  onPatchGroup?: (id: number, patch: Partial<VideoLayerGroupSummary>) => unknown;
  groupBusy?: boolean;
  effectEntries?: FxCatalogEntry[];
  onAddEffect?: (entry: FxCatalogEntry, layerId: number) => void | Promise<void>;
  bank: ComponentProps<typeof VideoClipSlotBankPanel>;
  layerList: ComponentProps<typeof VideoLayerListPanel>;
  runtimes: VideoClipLayerRuntimeSummary[];
  availabilityById: Record<number, MediaAssetAvailability>;
  previewLayerId: number | null;
  previewSlotId?: VideoClipSlotId | null;
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
  const orderedLayers = createMemo(() => {
    const ids = [...new Set((props.compositions ?? []).flatMap(composition => composition.layer_ids))];
    return [...ids.map(id => props.bank.layers.find(layer => layer.id === id)).filter((layer): layer is VideoLayerSummary => Boolean(layer)), ...props.bank.layers.filter(layer => !ids.includes(layer.id))];
  });
  createEffect(() => setRows(reconcile(orderedLayers().map((layer) => ({ id: layer.id, layer })), { key: "id", merge: true })));
  const [collapsed, setCollapsed] = createSignal<number[]>([]);
  const groupFor = (id: number) => props.groups?.find(group => group.layer_ids.includes(id));
  const visibleRows = () => rows.filter(row => !collapsed().includes(groupFor(row.id)?.id ?? -1));
  const selectLayer = (id: number) => { props.onSelectGroup?.(null); props.bank.onSelectLayer(id); };
  const [focusedColumn, setFocusedColumn] = createSignal(0);
  let root: HTMLDivElement | undefined;
  const selectedLayerId = () => props.bank.layer?.id ?? rows[0]?.id ?? null;

  const focusCell = (layerId: number, column: number) => {
    setFocusedColumn(column);
    selectLayer(layerId);
    const layer = rows.find((row) => row.id === layerId)?.layer;
    props.bank.onSelect(layer?.clip_slots?.[column]?.id ?? null);
    queueMicrotask(() => {
      const pad = root?.querySelector<HTMLButtonElement>(`[data-video-perform-layer-id="${layerId}"][data-video-perform-cell-index="${column}"]`);
      if (!pad || !root) return;
      pad.focus({ preventScroll: true });
      const bounds = root.getBoundingClientRect();
      const target = pad.getBoundingClientRect();
      const stickyWidth = root.querySelector<HTMLElement>(".videoPerformMatrixCorner")?.offsetWidth ?? 336;
      const stickyHeight = root.querySelector<HTMLElement>(".videoPerformMatrixHead")?.offsetHeight ?? 44;
      if (target.left < bounds.left + stickyWidth) root.scrollLeft -= bounds.left + stickyWidth - target.left;
      else if (target.right > bounds.right - 2) root.scrollLeft += target.right - bounds.right + 2;
      const row = pad.closest<HTMLElement>("[data-video-perform-row]")?.getBoundingClientRect();
      if (row && row.top < bounds.top + stickyHeight) root.scrollTop -= bounds.top + stickyHeight - row.top;
      else if (row && row.bottom > bounds.bottom - 2) root.scrollTop += row.bottom - bounds.bottom + 2;
    });
  };

  const queue = (layerId: number, column: number) => {
    const slot = rows.find((row) => row.id === layerId)?.layer.clip_slots?.[column];
    selectLayer(layerId);
    props.bank.onSelect(slot?.id ?? null);
    setFocusedColumn(column);
    if (slot) void props.bank.onQueue(slot.id);
  };

  const audition = (layerId: number, column: number) => {
    const slot = rows.find((row) => row.id === layerId)?.layer.clip_slots?.[column];
    if (!slot) return;
    selectLayer(layerId);
    props.bank.onSelect(slot.id);
    setFocusedColumn(column);
    void props.bank.onPreview(slot.id);
  };

  type ClipMenu = { layerId: number; slotId: VideoClipSlotId | null; trigger: HTMLButtonElement; x: number; y: number };
  const [clipMenu, setClipMenu] = createSignal<ClipMenu | null>(null);
  let clipMenuElement: HTMLDivElement | undefined;
  const closeClipMenu = (restoreFocus = false) => {
    const trigger = clipMenu()?.trigger;
    setClipMenu(null);
    if (restoreFocus && trigger?.isConnected) trigger.focus({ preventScroll: true });
  };
  const menuSlotExists = () => {
    const menu = clipMenu();
    return menu?.slotId != null && rows.some(row => row.id === menu.layerId && row.layer.clip_slots?.some(slot => slot.id === menu.slotId));
  };
  const openClipMenu = (event: MouseEvent, layerId: number, slotId: VideoClipSlotId | null) => {
    event.preventDefault();
    const trigger = event.currentTarget as HTMLButtonElement;
    selectLayer(layerId);
    props.bank.onSelect(slotId);
    const bounds = trigger.getBoundingClientRect();
    setClipMenu({ layerId, slotId, trigger, x: event.clientX || bounds.left, y: event.clientY || bounds.bottom });
    queueMicrotask(() => {
      if (!clipMenuElement || clipMenu()?.trigger !== trigger) return;
      const menuBounds = clipMenuElement.getBoundingClientRect();
      setClipMenu(menu => menu && ({
        ...menu,
        x: Math.max(8, Math.min(menu.x, window.innerWidth - menuBounds.width - 8)),
        y: Math.max(8, Math.min(menu.y, window.innerHeight - menuBounds.height - 8)),
      }));
      clipMenuElement.querySelector<HTMLButtonElement>("button:not(:disabled)")?.focus();
    });
  };
  const auditionFromMenu = () => {
    const menu = clipMenu();
    if (!menu || !menuSlotExists()) return;
    const column = rows.find(row => row.id === menu.layerId)!.layer.clip_slots!.findIndex(slot => slot.id === menu.slotId);
    closeClipMenu(true);
    audition(menu.layerId, column);
  };
  const optionsFromMenu = () => {
    const menu = clipMenu();
    if (!menu) return;
    closeClipMenu(true);
    selectLayer(menu.layerId);
    props.bank.onSelect(menu.slotId);
    props.bank.onMore(menu.slotId, menu.trigger);
  };
  const menuKeyDown = (event: KeyboardEvent) => {
    event.stopPropagation();
    if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      closeClipMenu(true);
    } else if (event.key === "Tab") {
      closeClipMenu(true);
    } else if (["ArrowUp", "ArrowDown", "Home", "End"].includes(event.key)) {
      event.preventDefault();
      event.stopPropagation();
      const buttons = [...(clipMenuElement?.querySelectorAll<HTMLButtonElement>("button:not(:disabled)") ?? [])];
      const index = buttons.indexOf(document.activeElement as HTMLButtonElement);
      const next = event.key === "Home" ? 0 : event.key === "End" ? buttons.length - 1 : (index + (event.key === "ArrowDown" ? 1 : -1) + buttons.length) % buttons.length;
      buttons[next]?.focus();
    }
  };
  const outsideClipMenu = (event: PointerEvent) => {
    if (clipMenu() && event.target instanceof Node && !clipMenuElement?.contains(event.target)) closeClipMenu();
  };
  createEffect(() => {
    const menu = clipMenu();
    if (menu && (!rows.some(row => row.id === menu.layerId) || (menu.slotId !== null && !menuSlotExists()))) closeClipMenu();
  });
  onMount(() => document.addEventListener("pointerdown", outsideClipMenu, true));
  onCleanup(() => document.removeEventListener("pointerdown", outsideClipMenu, true));

  const keyDown = (event: KeyboardEvent) => {
    if (props.keyboardEnabled === false || clipMenu() || event.defaultPrevented || event.repeat || event.isComposing
      || event.ctrlKey || event.metaKey || event.altKey) return;
    const target = event.target instanceof Element ? event.target : null;
    // Domain tabs own their roving focus keys; row keys still launch from the
    // persistent chrome just as they do elsewhere outside editable controls.
    if (target?.closest("[data-control-domain]") &&
      ["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown", "Home", "End"].includes(event.key)) return;
    // A focused pane divider owns its Arrow/Home/End resize keys.
    if (target?.closest("input, textarea, select, [role='separator'], [contenteditable]:not([contenteditable='false']), [data-video-perform-pane='inspector'], [data-effect-browser]")
      || document.querySelector("[role='dialog'], dialog[open], [aria-modal='true'], .app[data-control-learn-mode]")) return;
    const layerId = selectedLayerId();
    if (layerId === null) return;
    if (event.shiftKey) {
      // Physical codes work when Shift changes the character on US/JA keyboards.
      if (!/^Digit[0-9]$/.test(event.code)) return;
      const column = event.code === "Digit0" ? 9 : Number(event.code.slice(-1)) - 1;
      event.preventDefault();
      event.stopImmediatePropagation();
      focusCell(layerId, column);
      audition(layerId, column);
      return;
    }
    const rowIndex = Math.max(0, visibleRows().findIndex((row) => row.id === layerId));
    const targetPad = target?.closest<HTMLElement>("[data-video-perform-cell-index]");
    const column = targetPad ? Number(targetPad.dataset.videoPerformCellIndex) : focusedColumn();
    let nextColumn = column;
    let nextRow = rowIndex;
    if (event.key === "ArrowLeft") nextColumn = Math.max(0, column - 1);
    else if (event.key === "ArrowRight") nextColumn = Math.min(31, column + 1);
    else if (event.key === "ArrowUp") nextRow = Math.max(0, rowIndex - 1);
    else if (event.key === "ArrowDown") nextRow = Math.min(visibleRows().length - 1, rowIndex + 1);
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
    if (visibleRows()[nextRow]) focusCell(visibleRows()[nextRow].id, nextColumn);
  };
  onMount(() => window.addEventListener("keydown", keyDown, true));
  onCleanup(() => window.removeEventListener("keydown", keyDown, true));

  const selectedSlot = () => {
    const slots = rows.find((row) => row.id === selectedLayerId())?.layer.clip_slots;
    return slots?.find((slot) => slot.id === props.bank.selectedSlotId) ?? null;
  };
  // The corner buttons carry the per-clip actions that used to repeat in every cell.
  const inspectSelected = () => {
    const slot = selectedSlot();
    const layerId = selectedLayerId();
    if (!slot || layerId === null) return;
    selectLayer(layerId);
    props.bank.onSelect(slot.id);
    void props.bank.onPreview(slot.id);
  };
  const clipOptions = (trigger: HTMLButtonElement) => {
    const layerId = selectedLayerId();
    if (layerId === null) return;
    const slot = selectedSlot();
    selectLayer(layerId);
    props.bank.onSelect(slot?.id ?? null);
    props.bank.onMore(slot?.id ?? null, trigger);
  };

  const Row = (rowProps: { layer: VideoLayerSummary }) => {
    const layer = () => rowProps.layer;
    const runtime = createMemo(() => props.runtimes.find((item) => item.layer_id === layer().id) ?? null);
    const [cells, setCells] = createStore(videoClipSlotBankCells(layer().clip_slots, runtime()));
    createEffect(() => setCells(reconcile(videoClipSlotBankCells(layer().clip_slots, runtime()), { key: "index" })));
    const selected = () => selectedLayerId() === layer().id;
    const live = () => runtime()?.active_slot_id != null && runtime()?.playing === true;
    return <div class="videoPerformMatrixRow" classList={{ selected: selected(), grouped: Boolean(groupFor(layer().id)) }} data-video-perform-group-child={groupFor(layer().id)?.id} data-video-perform-row={layer().id} data-video-clip-slot-layer-id={layer().id} role="row" aria-selected={selected()}
      onDragOver={event => { if (event.dataTransfer?.types.includes(FX_DRAG_MIME)) { event.preventDefault(); event.dataTransfer.dropEffect = "copy"; } }}
      onDrop={async event => {
        if (!event.dataTransfer) return;
        const entry = readFxDragPayload(event.dataTransfer, props.effectEntries ?? []);
        if (!entry) return;
        event.preventDefault(); event.stopPropagation();
        selectLayer(layer().id);
        // A row may differ from the browser's selected target. Review the exact
        // destination before changing its live chain.
        const target = layer();
        if (await confirmDialog(addEffectToLiveLayerDialog(`${target.id} · ${target.label}`, entry.label))) void props.onAddEffect?.(entry, target.id);
      }}>
      <div class="videoPerformLayerStrip" role="rowheader">
        <div class="videoPerformLayerTop">
          <button type="button" class="videoPerformLayerName" aria-pressed={selected()} title={layer().label} data-video-perform-select-layer={layer().id} onClick={() => selectLayer(layer().id)} data-no-localize>{layer().label}</button>
          <button type="button" aria-pressed={layer().state.solo} classList={{ active: layer().state.solo }} data-video-layer-solo={layer().id} {...controlMappingTargetData({ action: "VideoLayerSolo", label: `${layer().label} solo`, layer_id: layer().id })} onClick={() => void props.layerList.onSetLayerState(layer().id, { ...layer().state, solo: !layer().state.solo })}>Solo</button>
          <button type="button" aria-pressed={!layer().state.enabled} classList={{ active: !layer().state.enabled }} data-video-layer-bypass={layer().id} {...controlMappingTargetData({ action: "VideoLayerEnabled", label: `${layer().label} enabled`, layer_id: layer().id })} onClick={() => void props.layerList.onSetLayerState(layer().id, { ...layer().state, enabled: !layer().state.enabled })}>Bypass</button>
          <button type="button" data-video-layer-stop={layer().id} {...controlMappingTargetData({ action: "VideoLayerStop", label: `${layer().label} stop`, layer_id: layer().id })} onClick={() => void props.onStop(layer().id)}>Stop</button>
        </div>
        <div class="videoPerformLayerMix">
          <span class="videoPerformLayerLive" classList={{ live: live() }} data-video-perform-layer-live={live() ? "true" : "false"} title={live() ? "LIVE" : runtime()?.active_slot_id != null ? "Paused" : "Stopped"}><span aria-hidden="true">{live() ? "▶" : runtime()?.active_slot_id != null ? "❚❚" : "■"}</span><span>{live() ? "LIVE" : runtime()?.active_slot_id != null ? "Paused" : "Stopped"}</span></span>
          <label class="videoPerformLayerOpacity" title="Opacity"><input type="range" min="0" max="1" step="0.01" value={layer().state.opacity} aria-label="Opacity" data-video-layer-opacity={layer().id} {...controlMappingTargetData({ action: "VideoParam", label: `${layer().label} opacity`, layer_id: layer().id, video_param: "Opacity" })} onInput={(event) => void props.layerList.onSetLayerState(layer().id, { ...layer().state, opacity: Number(event.currentTarget.value) })} /><strong data-no-localize>{Math.round(layer().state.opacity * 100)}%</strong></label>
          <label class="videoPerformLayerBlend" title="Blend"><select value={layer().blend_mode} aria-label="Blend" data-video-layer-blend={layer().id} {...controlMappingTargetData({ action: "VideoLayerBlendMode", label: `${layer().label} blend`, layer_id: layer().id })} onInput={(event) => void props.layerList.onSetBlendMode(layer().id, event.currentTarget.value as VideoBlendMode)}><For each={blends}>{(blend) => <option value={blend}>{blend === "SoftLight" ? "Soft Light" : blend}</option>}</For></select></label>
        </div>
      </div>
      <For each={cells}>{(cell) => {
        const asset = () => props.bank.assets.find((item) => item.id === cell.slot?.media_asset_id);
        const availability = () => cell.slot ? props.availabilityById[cell.slot.media_asset_id] : undefined;
        const missing = () => cell.slot !== null && (!asset() || availability()?.kind === "missing");
        const failed = () => ["unreadable", "hash_mismatch"].includes(availability()?.kind ?? "");
        // Accepted native identity is independent of selection and Program.
        // Legacy layer P has no slot identity and still marks the active source.
        const previewing = () => props.previewLayerId === layer().id
          && (props.previewSlotId != null ? cell.slot?.id === props.previewSlotId : cell.active);
        const state = () => !cell.slot ? "Empty" : missing() ? "Missing" : failed() ? "Error" : cell.active ? (runtime()?.playing ? "LIVE" : previewing() ? "Preview" : "Paused") : cell.queued || cell.pending ? "Queued" : previewing() ? "Preview" : "Loaded";
        const selectedCell = () => selected() && (cell.slot ? cell.slot.id === props.bank.selectedSlotId : focusedColumn() === cell.index && props.bank.selectedSlotId === null);
        return <div class="videoPerformMatrixCell" classList={{ occupied: cell.slot !== null, live: cell.active && runtime()?.playing === true, queued: cell.queued || cell.pending, preview: previewing(), selected: selectedCell(), error: missing() || failed() }} role="gridcell" data-video-clip-slot-id={cell.slot?.id} data-video-clip-slot-index={cell.index} data-video-clip-slot-layer-id={layer().id} data-video-perform-cell-state={state()} data-video-perform-preview={previewing() ? "true" : undefined}>
          <button type="button" class="videoPerformClipTrigger" data-video-perform-layer-id={layer().id} data-video-perform-cell-index={cell.index} data-video-clip-slot-pad-index={cell.index} {...controlMappingTargetData(cell.slot ? { action: "VideoClipSlotTrigger", label: `${layer().label} clip ${cell.index + 1}`, layer_id: layer().id, clip_slot_id: cell.slot.id } : null)} aria-label={`Queue clip ${cell.index + 1}: ${asset()?.label ?? "Empty"}`} aria-pressed={selectedCell()} tabIndex={selectedCell() || selected() && props.bank.selectedSlotId == null && cell.index === focusedColumn() ? 0 : -1} onFocus={() => setFocusedColumn(cell.index)} onClick={() => queue(layer().id, cell.index)} aria-haspopup="menu" onContextMenu={(event) => openClipMenu(event, layer().id, cell.slot?.id ?? null)}>
            <span class="videoPerformClipImage"><Show when={props.thumbnailsAuthorized && cell.slot && props.bank.thumbnails[cell.slot.media_asset_id]} fallback={<span class="videoPerformClipPlaceholder" aria-hidden="true">{cell.slot ? "▧" : "+"}</span>}>{(thumbnail) => <img src={thumbnail()} alt="" loading="lazy" />}</Show><kbd data-no-localize>{videoPerformClipKeys[cell.index].toUpperCase()}</kbd><Show when={previewing() && state() !== "Preview"}><span class="videoPerformPreviewBadge"><span aria-hidden="true">◉</span><span>Preview</span></span></Show></span>
            <span class="videoPerformClipTitle" title={asset()?.label} data-no-localize>{cell.slot ? asset()?.label ?? `#${cell.slot.id}` : "—"}</span>
            <span class="videoPerformClipState"><span aria-hidden="true">{state() === "LIVE" ? "▶ " : state() === "Queued" ? "◷ " : state() === "Preview" ? "◉" : missing() || failed() ? "! " : ""}</span><span>{state()}</span></span>
          </button>
        </div>;
      }}</For>
    </div>;
  };

  const GroupHeader = (header: { id: number }) => {
    const group = () => ({ ...defaultVideoGroupMix, ...props.groups!.find(item => item.id === header.id)! });
    const patch = (value: Partial<VideoLayerGroupSummary>) => props.onPatchGroup?.(header.id, value);
    // The current learn model cannot represent a video group. Do not claim a
    // child-layer mapping is group control; preserve explicit target metadata.
    const mapping = (property: string) => ({ ...controlMappingTargetData(null), "data-control-map-group-id": header.id, "data-control-map-group-property": property, "data-control-map-unsupported": "video-group" });
    return <div class="videoPerformGroupRow" data-video-perform-group={header.id} role="row">
      <div class="videoPerformGroupStrip" role="rowheader">
        <button type="button" data-video-group-collapse={header.id} aria-label={collapsed().includes(header.id) ? "Expand group" : "Collapse group"} aria-expanded={!collapsed().includes(header.id)} onClick={() => setCollapsed(ids => ids.includes(header.id) ? ids.filter(id => id !== header.id) : [...ids, header.id])}>{collapsed().includes(header.id) ? "\u25b8" : "\u25be"}</button>
        <button type="button" class="videoPerformGroupName" aria-pressed={props.selectedGroupId === header.id} data-video-perform-select-group={header.id} title={group().label} onClick={() => props.onSelectGroup?.(header.id)} data-no-localize>{group().label}</button>
        <label title="Group opacity"><input type="range" min="0" max="1" step="0.01" value={group().opacity} aria-label="Group opacity" data-video-group-opacity={header.id} disabled={props.groupBusy} {...mapping("opacity")} onChange={event => patch({ opacity: Number(event.currentTarget.value) })} /><span data-no-localize>{Math.round(group().opacity * 100)}%</span></label>
        <select value={group().blend_mode} aria-label="Group blend" data-video-group-blend={header.id} disabled={props.groupBusy} onChange={event => patch({ blend_mode: event.currentTarget.value as VideoBlendMode })}><For each={videoGroupBlends}>{blend => <option value={blend}>{blend === "SoftLight" ? "Soft Light" : blend}</option>}</For></select>
        <button type="button" aria-pressed={group().solo} data-video-group-solo={header.id} disabled={props.groupBusy} {...mapping("solo")} onClick={() => patch({ solo: !group().solo })}>Solo</button>
        <button type="button" aria-pressed={group().bypassed} data-video-group-bypass={header.id} disabled={props.groupBusy} {...mapping("bypassed")} onClick={() => patch({ bypassed: !group().bypassed })}>Bypass</button>
      </div>
    </div>;
  };

  return <section class="videoPerformMatrix" data-video-perform-matrix data-video-perform-region="matrix" aria-label="Clip matrix">
    <Show when={rows.length > 0} fallback={<div class="videoPerformMatrixEmpty"><span>No video layers. Add a file, still, or input above.</span><button type="button" onClick={() => void props.bank.onImport()}>Import</button></div>}>
      <div ref={root} class="videoPerformMatrixScroll" role="grid" aria-label="Video layers and clips" aria-rowcount={rows.length + 1} aria-colcount="33" tabIndex={-1}>
        <div class="videoPerformMatrixHead" role="row"><div class="videoPerformMatrixCorner" role="columnheader"><strong title={`${rows.length} × 32`}>Layers / Clips</strong><button type="button" data-video-perform-inspect disabled={!selectedSlot()} title="Audition selected clip in Preview" onClick={inspectSelected}>Inspect</button><button type="button" data-video-perform-clip-options aria-label="Options for the selected clip" title="Options for the selected clip" onClick={(event) => clipOptions(event.currentTarget)}>Clip…</button></div><For each={Array.from({ length: 32 }, (_, index) => index)}>{(index) => <div class="videoPerformMatrixColumn" role="columnheader"><span data-no-localize>{index + 1}</span><kbd data-no-localize>{videoPerformClipKeys[index].toUpperCase()}</kbd></div>}</For></div>
        <For each={rows}>{(row) => <>
          <Show when={groupFor(row.id)?.layer_ids[0] === row.id ? groupFor(row.id) : undefined}>{group => <GroupHeader id={group().id} />}</Show>
          <Show when={!collapsed().includes(groupFor(row.id)?.id ?? -1)}><Row layer={row.layer} /></Show>
        </>}</For>
      </div>
    </Show>
    <Show when={clipMenu()}>{menu => <Portal>
      <div ref={clipMenuElement} class="videoPerformClipMenu" role="menu" aria-label="Clip actions"
        data-video-perform-context-menu style={{ left: `${menu().x}px`, top: `${menu().y}px` }} onKeyDown={menuKeyDown}>
        <button type="button" role="menuitem" data-video-perform-audition disabled={!menuSlotExists()} onClick={auditionFromMenu}>Audition in Preview</button>
        <button type="button" role="menuitem" data-video-perform-context-options onClick={optionsFromMenu}>Clip options</button>
      </div>
    </Portal>}</Show>
  </section>;
}
