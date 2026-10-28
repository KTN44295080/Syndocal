import { batch, createEffect, createMemo, createSignal, For, onCleanup, onMount, Show, type ComponentProps } from "solid-js";
import { createStore, reconcile } from "solid-js/store";
import { Portal } from "solid-js/web";
import { controlMappingTargetData, type ControlMappingTarget } from "../controlMappingLearn";
import type { MediaAssetAvailability, VideoBlendMode, VideoClipLayerRuntimeSummary, VideoClipSlotId, VideoLayerSummary, VideoLayerGroupSummary, CompositionSummary } from "../types";
import { videoClipSlotBankCells } from "../videoClipSlotBankModel";
import { VideoClipSlotBankPanel } from "./VideoClipSlotBankPanel";
import { defaultVideoGroupMix, videoGroupBlends } from "../videoLayerGroups";
import { VideoLayerListPanel } from "./VideoLayerListPanel";
import "./VideoPerformMatrix.css";
import { FX_DRAG_MIME, readFxDragPayload, type FxCatalogEntry } from "../fxCatalogModel";
import { dragCarriesFiles, droppedFilePaths } from "../videoPerformFirstShow";

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
  /** Empty Perform starts the first show with the existing first-run flow. */
  firstShow?: VideoPerformFirstShow;
}

export interface VideoPerformFirstShow {
  available: boolean;
  busy: boolean;
  backendAvailable: boolean;
  /** Without paths the first-run flow opens its file picker. */
  onStart: (paths?: readonly string[]) => void | Promise<unknown>;
  /** Where layers are added when the show is no longer empty (media already imported). */
  onOpenEditor?: () => void;
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
  const [pageSize, setPageSize] = createSignal(1);
  const [pageStart, setPageStart] = createSignal(0);
  const pageEnd = () => Math.min(32, pageStart() + pageSize());
  const columnOnPage = (column: number) => column >= pageStart() && column < pageEnd();
  let root: HTMLDivElement | undefined;
  let resizeObserver: ResizeObserver | undefined;
  const selectedLayerId = () => props.bank.layer?.id ?? rows[0]?.id ?? null;
  const tabLayerId = () => visibleRows().find(row => row.id === selectedLayerId())?.id ?? visibleRows()[0]?.id;
  const tabColumn = () => {
    const selected = rows.find(row => row.id === tabLayerId())?.layer.clip_slots?.findIndex(slot => slot.id === props.bank.selectedSlotId) ?? -1;
    return columnOnPage(selected) ? selected : columnOnPage(focusedColumn()) ? focusedColumn() : pageStart();
  };
  const attachScroll = (element: HTMLDivElement) => {
    root = element;
    resizeObserver?.disconnect();
    const measure = () => {
      const stripWidth = element.querySelector<HTMLElement>(".videoPerformMatrixCorner")?.offsetWidth ?? 336;
      const nextSize = Math.max(1, Math.min(32, Math.floor((element.clientWidth - stripWidth) / 100)));
      if (nextSize === pageSize()) return;
      const active = document.activeElement instanceof HTMLElement && element.contains(document.activeElement)
        ? document.activeElement.closest<HTMLElement>("[data-video-perform-cell-index]") : null;
      const anchor = active ? Number(active.dataset.videoPerformCellIndex) : pageStart();
      // Keep the focused absolute slot in the page when the pane is resized.
      // Reconcile the same cell objects so an overlapping pad keeps its DOM.
      batch(() => {
        setPageSize(nextSize);
        setPageStart(Math.floor(anchor / nextSize) * nextSize);
      });
    };
    resizeObserver = new ResizeObserver(measure);
    resizeObserver.observe(element);
    measure();
  };
  onCleanup(() => resizeObserver?.disconnect());
  const showColumn = (column: number) => {
    if (!columnOnPage(column)) setPageStart(Math.floor(column / pageSize()) * pageSize());
  };
  const changePage = (direction: -1 | 1) => {
    // Paging is presentation only: it never queues, auditions, or takes a clip.
    closeClipMenu();
    closeOffpage();
    setPageStart(start => Math.max(0, Math.min(Math.floor(31 / pageSize()) * pageSize(), start + direction * pageSize())));
  };

  const focusCell = (layerId: number, column: number) => {
    showColumn(column);
    setFocusedColumn(column);
    selectLayer(layerId);
    const layer = rows.find((row) => row.id === layerId)?.layer;
    props.bank.onSelect(layer?.clip_slots?.[column]?.id ?? null);
    queueMicrotask(() => {
      const pad = root?.querySelector<HTMLButtonElement>(`[data-video-perform-layer-id="${layerId}"][data-video-perform-cell-index="${column}"]`);
      if (!pad || !root) return;
      pad.focus({ preventScroll: true });
      const bounds = root.getBoundingClientRect();
      const stickyHeight = root.querySelector<HTMLElement>(".videoPerformMatrixHead")?.offsetHeight ?? 44;
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

  type OffpageClip = { id: string; layerId: number; layerLabel: string; slotId: VideoClipSlotId; index: number; label: string; state: "live" | "queued" };
  const [offpageClips, setOffpageClips] = createStore<OffpageClip[]>([]);
  const [offpageOpen, setOffpageOpen] = createSignal(false);
  const [offpagePosition, setOffpagePosition] = createSignal({ x: 8, y: 8 });
  let offpageTrigger: HTMLButtonElement | undefined;
  let offpageElement: HTMLDivElement | undefined;
  const returnFromOffpage = () => {
    const candidates = [offpageTrigger,
      root?.querySelector<HTMLElement>('[data-video-perform-cell-index][tabindex="0"]'),
      root?.querySelector<HTMLElement>("[data-video-perform-page]:not(:disabled)"), root];
    const target = candidates.find(element => {
      if (!element?.isConnected || !element.getClientRects().length) return false;
      const bounds = element.getBoundingClientRect();
      return bounds.width > 0 && bounds.height > 0;
    });
    target?.focus({ preventScroll: true });
    if (root && target?.matches("[data-video-perform-cell-index]")) {
      const viewport = root.getBoundingClientRect();
      const headHeight = root.querySelector<HTMLElement>(".videoPerformMatrixHead")?.offsetHeight ?? 44;
      const row = target.closest<HTMLElement>("[data-video-perform-row]")?.getBoundingClientRect();
      if (row && row.top < viewport.top + headHeight) root.scrollTop -= viewport.top + headHeight - row.top;
      else if (row && row.bottom > viewport.bottom - 2) root.scrollTop += row.bottom - viewport.bottom + 2;
    }
  };
  const closeOffpage = (restoreFocus = false) => {
    setOffpageOpen(false);
    if (restoreFocus) returnFromOffpage();
  };
  createEffect(() => {
    const entries = rows.flatMap(row => {
      const runtime = props.runtimes.find(item => item.layer_id === row.id);
      const states: Array<{ slotId: VideoClipSlotId; state: "live" | "queued" }> = [];
      if (runtime?.playing && runtime.active_slot_id != null) states.push({ slotId: runtime.active_slot_id, state: "live" });
      for (const slotId of new Set([runtime?.queued_slot_id, runtime?.pending_launch?.slot_id])) {
        if (slotId != null) states.push({ slotId, state: "queued" });
      }
      return states.flatMap(({ slotId, state }) => {
        const index = row.layer.clip_slots?.findIndex(slot => slot.id === slotId) ?? -1;
        if (index < 0 || columnOnPage(index)) return [];
        const slot = row.layer.clip_slots![index];
        return [{ id: `${row.id}:${slotId}:${state}`, layerId: row.id, layerLabel: row.layer.label, slotId, index,
          label: props.bank.assets.find(asset => asset.id === slot.media_asset_id)?.label ?? `#${slotId}`, state }];
      });
    });
    const dialog = offpageOpen() ? offpageElement : undefined;
    const active = document.activeElement;
    const focused = active instanceof HTMLElement && dialog?.contains(active) ? active : null;
    setOffpageClips(reconcile(entries, { key: "id" }));
    if (dialog) queueMicrotask(() => {
      if (!offpageOpen() || offpageElement !== dialog) return;
      const current = document.activeElement;
      // A removed focused entry can leave focus on body without a focusout
      // event. Restore only that lost focus, never a user's new external focus.
      const ownsFocus = focused !== null && (current === focused || (!focused.isConnected
        && (current === document.body || current === document.documentElement || current === null)));
      if (offpageClips.length === 0) closeOffpage(ownsFocus);
      else if (focused && !ownsFocus && !dialog.contains(current)) closeOffpage();
      else if (ownsFocus && focused && !focused.isConnected) {
        dialog.scrollTop = 0;
        dialog.querySelector<HTMLButtonElement>("button")?.focus({ preventScroll: true });
      }
    });
  });
  const openOffpage = () => {
    if (offpageOpen()) { closeOffpage(true); return; }
    closeClipMenu();
    const anchor = offpageTrigger?.getBoundingClientRect();
    setOffpagePosition({ x: anchor?.left ?? 8, y: anchor?.bottom ?? 8 });
    setOffpageOpen(true);
    queueMicrotask(() => {
      if (!offpageElement) return;
      const bounds = offpageElement.getBoundingClientRect();
      setOffpagePosition({ x: Math.max(8, Math.min(anchor?.left ?? 8, innerWidth - bounds.width - 8)),
        y: Math.max(8, Math.min(anchor?.bottom ?? 8, innerHeight - bounds.height - 8)) });
      offpageElement.querySelector<HTMLButtonElement>("button")?.focus();
    });
  };
  const outsideOffpage = (event: PointerEvent) => {
    if (offpageOpen() && event.target instanceof Node && !offpageElement?.contains(event.target) && !offpageTrigger?.contains(event.target)) closeOffpage();
  };
  onMount(() => document.addEventListener("pointerdown", outsideOffpage, true));
  onCleanup(() => document.removeEventListener("pointerdown", outsideOffpage, true));
  const jumpToClip = (entry: OffpageClip) => {
    closeOffpage();
    const group = groupFor(entry.layerId);
    if (group) setCollapsed(ids => ids.filter(id => id !== group.id));
    focusCell(entry.layerId, entry.index);
  };

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
  const selectedClipName = () => {
    const slot = selectedSlot();
    return slot ? props.bank.assets.find(asset => asset.id === slot.media_asset_id)?.label ?? `#${slot.id}` : "";
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
        // A row may differ from the browser's selected target; add to this row.
        void props.onAddEffect?.(entry, layer().id);
      }}>
      <div class="videoPerformLayerStrip" role="rowheader" aria-colindex={1}>
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
      <For each={cells.slice(pageStart(), pageEnd())}>{(cell) => {
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
        return <div class="videoPerformMatrixCell" classList={{ occupied: cell.slot !== null, live: cell.active && runtime()?.playing === true, queued: cell.queued || cell.pending, preview: previewing(), selected: selectedCell(), error: missing() || failed() }} role="gridcell" aria-colindex={cell.index + 2} data-video-clip-slot-id={cell.slot?.id} data-video-clip-slot-index={cell.index} data-video-clip-slot-layer-id={layer().id} data-video-perform-cell-state={state()} data-video-perform-preview={previewing() ? "true" : undefined}>
          <button type="button" class="videoPerformClipTrigger" data-video-perform-layer-id={layer().id} data-video-perform-cell-index={cell.index} data-video-clip-slot-pad-index={cell.index} {...controlMappingTargetData(cell.slot ? { action: "VideoClipSlotTrigger", label: `${layer().label} clip ${cell.index + 1}`, layer_id: layer().id, clip_slot_id: cell.slot.id } : null)} aria-label={`Queue clip ${cell.index + 1}: ${asset()?.label ?? "Empty"}`} aria-pressed={selectedCell()} tabIndex={tabLayerId() === layer().id && cell.index === tabColumn() ? 0 : -1} onFocus={() => setFocusedColumn(cell.index)} onClick={() => queue(layer().id, cell.index)} aria-haspopup="menu" onContextMenu={(event) => openClipMenu(event, layer().id, cell.slot?.id ?? null)}>
            <span class="videoPerformClipImage"><Show when={props.thumbnailsAuthorized && cell.slot && props.bank.thumbnails[cell.slot.media_asset_id]} fallback={<span class="videoPerformClipPlaceholder" aria-hidden="true">{cell.slot ? "▧" : "+"}</span>}>{(thumbnail) => <img src={thumbnail()} alt="" loading="lazy" />}</Show><kbd data-no-localize>{videoPerformClipKeys[cell.index].toUpperCase()}</kbd><Show when={previewing() && state() !== "Preview"}><span class="videoPerformPreviewBadge"><span aria-hidden="true">◉</span><span>Preview</span></span></Show></span>
            <span class="videoPerformClipTitle" title={asset()?.label} data-no-localize>{cell.slot ? asset()?.label ?? `#${cell.slot.id}` : "—"}</span>
            <span class="videoPerformClipState" title={state()} aria-label={state()} role="status">
              <span aria-hidden="true">{state() === "LIVE" ? "▶" : state() === "Queued" ? "◷" : state() === "Preview" ? "◉" : missing() || failed() ? "!" : state() === "Paused" ? "Ⅱ" : state() === "Loaded" ? "●" : "○"}</span>
              <span class="srOnly">{state()}</span>
            </span>
          </button>
        </div>;
      }}</For>
    </div>;
  };

  const GroupHeader = (header: { id: number }) => {
    const group = () => ({ ...defaultVideoGroupMix, ...props.groups!.find(item => item.id === header.id)! });
    const patch = (value: Partial<VideoLayerGroupSummary>) => props.onPatchGroup?.(header.id, value);
    const mapping = (action: ControlMappingTarget["action"], property: string) => ({
      ...controlMappingTargetData({ action, label: `Video Group ${header.id} ${property}`, video_group_id: header.id }),
      "data-control-map-group-id": header.id, "data-control-map-group-property": property,
    });
    return <div class="videoPerformGroupRow" data-video-perform-group={header.id} role="row">
      <div class="videoPerformGroupStrip" role="rowheader">
        <button type="button" data-video-group-collapse={header.id} aria-label={collapsed().includes(header.id) ? "Expand group" : "Collapse group"} aria-expanded={!collapsed().includes(header.id)} onClick={() => setCollapsed(ids => ids.includes(header.id) ? ids.filter(id => id !== header.id) : [...ids, header.id])}>{collapsed().includes(header.id) ? "\u25b8" : "\u25be"}</button>
        <button type="button" class="videoPerformGroupName" aria-pressed={props.selectedGroupId === header.id} data-video-perform-select-group={header.id} title={group().label} onClick={() => props.onSelectGroup?.(header.id)} data-no-localize>{group().label}</button>
        <label title="Group opacity"><input type="range" min="0" max="1" step="0.01" value={group().opacity} aria-label="Group opacity" data-video-group-opacity={header.id} disabled={props.groupBusy} {...mapping("VideoGroupOpacity", "opacity")} onChange={event => patch({ opacity: Number(event.currentTarget.value) })} /><span data-no-localize>{Math.round(group().opacity * 100)}%</span></label>
        <select value={group().blend_mode} aria-label="Group blend" data-video-group-blend={header.id} {...mapping("VideoGroupBlendMode", "blend")} disabled={props.groupBusy} onChange={event => patch({ blend_mode: event.currentTarget.value as VideoBlendMode })}><For each={videoGroupBlends}>{blend => <option value={blend}>{blend === "SoftLight" ? "Soft Light" : blend}</option>}</For></select>
        <button type="button" aria-pressed={group().solo} data-video-group-solo={header.id} disabled={props.groupBusy} {...mapping("VideoGroupSolo", "solo")} onClick={() => patch({ solo: !group().solo })}>Solo</button>
        <button type="button" aria-pressed={group().bypassed} data-video-group-bypass={header.id} disabled={props.groupBusy} {...mapping("VideoGroupBypass", "bypass")} onClick={() => patch({ bypassed: !group().bypassed })}>Bypass</button>
        <button type="button" aria-pressed={group().enabled} aria-label="Group enabled" title="Group enabled" data-video-group-enabled={header.id} disabled={props.groupBusy} {...mapping("VideoGroupEnabled", "enabled")} onClick={() => patch({ enabled: !group().enabled })}>Enabled</button>
      </div>
    </div>;
  };

  const [fileDragOver, setFileDragOver] = createSignal(false);
  // The whole empty matrix is the drop target. Native drops reach App's drop
  // router through the data attribute; a page-level drop carries no paths in
  // WebView2, so it opens the same picker instead of doing nothing.
  const EmptyState = () => {
    const show = () => props.firstShow;
    const startable = () => Boolean(show()?.available || show()?.busy);
    const canStart = () => Boolean(show()?.available && show()?.backendAvailable && !show()?.busy);
    return <div class="videoPerformMatrixEmpty" data-video-perform-empty
      data-video-perform-first-show-drop={startable() ? "" : undefined}
      data-file-drag-over={fileDragOver() ? "true" : undefined}
      onDragOver={event => {
        if (!canStart() || !dragCarriesFiles(event.dataTransfer)) return;
        event.preventDefault();
        if (event.dataTransfer) event.dataTransfer.dropEffect = "copy";
        setFileDragOver(true);
      }}
      onDragLeave={event => { if (!(event.relatedTarget instanceof Node && event.currentTarget.contains(event.relatedTarget))) setFileDragOver(false); }}
      onDrop={event => {
        setFileDragOver(false);
        if (!canStart() || !dragCarriesFiles(event.dataTransfer)) return;
        event.preventDefault();
        const paths = droppedFilePaths(event.dataTransfer);
        void show()?.onStart(paths.length > 0 ? paths : undefined);
      }}>
      <Show when={startable()} fallback={<>
        <span>{"No video layers. Add one with Import Media in Edit > Video."}</span>
        <Show when={show()?.onOpenEditor}>{open => <button type="button" data-video-perform-open-editor onClick={() => open()()}>{"Open Edit > Video"}</button>}</Show>
      </>}>
        <span>No video layers. Pick a file or drop it here.</span>
        <button type="button" class="primary" data-video-perform-first-show aria-busy={show()?.busy ? "true" : undefined}
          disabled={!canStart()} title={show()?.backendAvailable === false ? "Desktop required" : undefined}
          onClick={() => void show()?.onStart()}>
          <span role={show()?.busy ? "status" : undefined}>{show()?.busy ? "Setting Up…" : "Pick a file"}</span>
        </button>
      </Show>
    </div>;
  };

  return <section class="videoPerformMatrix" data-video-perform-matrix data-video-perform-region="matrix" aria-label="Clip matrix"
    data-page-start={pageStart()} data-page-size={pageSize()} data-page-end={pageEnd()} data-slot-count="32"
    style={{ "--perform-visible-columns": pageEnd() - pageStart() }}>
    <Show when={rows.length > 0} fallback={<EmptyState />}>
      <div ref={attachScroll} class="videoPerformMatrixScroll" role="grid" aria-label="Video layers and clips" aria-rowcount={rows.length + 1} aria-colcount="33" tabIndex={-1}>
        <div class="videoPerformMatrixHead" role="row">
          <div class="videoPerformMatrixCorner" role="columnheader" aria-colindex={1}>
            <button type="button" data-video-perform-page="previous" aria-label="Previous clip slots" title="Previous clip slots" disabled={pageStart() === 0} onClick={() => changePage(-1)}>‹</button>
            <span data-video-perform-page-range data-no-localize aria-label="Visible clips">{pageStart() + 1}–{pageEnd()}/32</span>
            <button type="button" data-video-perform-page="next" aria-label="Next clip slots" title="Next clip slots" disabled={pageEnd() === 32} onClick={() => changePage(1)}>›</button>
            <button type="button" data-video-perform-inspect disabled={!selectedSlot()} title="Audition selected clip in Preview" onClick={inspectSelected}>Inspect</button>
            <button type="button" data-video-perform-clip-options aria-label="Options for the selected clip" title="Options for the selected clip" onClick={(event) => clipOptions(event.currentTarget)}>Clip</button>
          </div>
          <div class="videoPerformMatrixSelection" role="columnheader" aria-colindex={pageStart() + 2} aria-colspan={pageEnd() - pageStart()}>
            <div class="videoPerformSelectedClip"><Show when={selectedSlot()} fallback={<span>No clip selected</span>}>
              <span>Selected clip</span><strong data-video-perform-selected-clip-name data-no-localize>{selectedClipName()}</strong>
            </Show></div>
            <Show when={offpageClips.length > 0}>
              <button ref={offpageTrigger} type="button" data-video-perform-offpage-status aria-label="Off-page clips" aria-haspopup="dialog" aria-expanded={offpageOpen()} onClick={openOffpage}>
                <span>Off-page clips</span>
                <Show when={offpageClips.some(entry => entry.state === "live")}><span class="videoPerformOffpageLive">LIVE <span data-no-localize>{offpageClips.filter(entry => entry.state === "live").length}</span></span></Show>
                <Show when={offpageClips.some(entry => entry.state === "queued")}><span class="videoPerformOffpageQueued"><span>Queued</span> <span data-no-localize>{offpageClips.filter(entry => entry.state === "queued").length}</span></span></Show>
              </button>
            </Show>
          </div>
        </div>
        <For each={rows}>{(row) => <>
          <Show when={groupFor(row.id)?.layer_ids[0] === row.id ? groupFor(row.id) : undefined}>{group => <GroupHeader id={group().id} />}</Show>
          <Show when={!collapsed().includes(groupFor(row.id)?.id ?? -1)}><Row layer={row.layer} /></Show>
        </>}</For>
      </div>
    </Show>
    <Show when={offpageOpen()}><Portal>
      <div ref={offpageElement} class="videoPerformOffpageList" data-video-perform-offpage-list role="dialog" aria-label="Off-page clips"
        style={{ left: `${offpagePosition().x}px`, top: `${offpagePosition().y}px` }}
        onFocusOut={event => {
          const dialog = event.currentTarget;
          // Do not defer an internal focus move: its destination may be removed
          // by the next runtime snapshot before the microtask is delivered.
          if (event.relatedTarget instanceof Node && dialog.contains(event.relatedTarget)) return;
          const departed = event.target;
          queueMicrotask(() => {
            if (offpageElement !== dialog) return;
            const active = document.activeElement;
            const removedEntry = departed instanceof Node && !departed.isConnected
              && (active === document.body || active === document.documentElement || active === null);
            if (offpageOpen() && !offpageElement?.contains(active) && !removedEntry) closeOffpage();
          });
        }}
        onKeyDown={event => { if (event.key === "Escape") { event.preventDefault(); event.stopPropagation(); closeOffpage(true); } }}>
        <header><strong>Off-page clips</strong><button type="button" onClick={() => closeOffpage(true)}>Close</button></header>
        <For each={offpageClips}>{entry => <button type="button" data-video-perform-status-jump={entry.state}
          data-layer-id={entry.layerId} data-slot-id={entry.slotId} data-clip-index={entry.index} title="Jump to clip" onClick={() => jumpToClip(entry)}>
          <strong class={entry.state === "live" ? "videoPerformOffpageLive" : "videoPerformOffpageQueued"}>{entry.state === "live" ? "LIVE" : "Queued"}</strong>
          <span data-no-localize>{entry.layerLabel} · {entry.index + 1} · {entry.label}</span><span aria-hidden="true">↗</span>
        </button>}</For>
      </div>
    </Portal></Show>
    <Show when={clipMenu()}>{menu => <Portal>
      <div ref={clipMenuElement} class="videoPerformClipMenu" role="menu" aria-label="Clip actions"
        data-video-perform-context-menu style={{ left: `${menu().x}px`, top: `${menu().y}px` }} onKeyDown={menuKeyDown}>
        <button type="button" role="menuitem" data-video-perform-audition disabled={!menuSlotExists()} onClick={auditionFromMenu}>Audition in Preview</button>
        <button type="button" role="menuitem" data-video-perform-context-options onClick={optionsFromMenu}>Clip options</button>
      </div>
    </Portal>}</Show>
  </section>;
}
