import { For, Show, createEffect, createMemo, createSignal, type JSX } from "solid-js";
import { createStore, reconcile } from "solid-js/store";
import type {
  MediaAssetSummary,
  VideoClipLaunchQuantization,
  VideoClipLayerRuntimeSummary,
  VideoClipSlotId,
  VideoClipTakeKind,
  VideoClipTakeDurationUnit,
  VideoLayerSummary,
} from "../types";
import {
  videoClipSlotBankActionLabels,
  videoClipSlotBankCells,
  type VideoClipSlotBankMode,
} from "../videoClipSlotBankModel";

export interface VideoClipSlotBankPanelProps {
  mode: VideoClipSlotBankMode;
  performance?: boolean;
  layers: VideoLayerSummary[];
  layer: VideoLayerSummary | null;
  runtime: VideoClipLayerRuntimeSummary | null;
  assets: MediaAssetSummary[];
  thumbnails: Record<number, string>;
  selectedSlotId: VideoClipSlotId | null;
  onSelectLayer: (layerId: number) => void;
  onSelect: (slotId: VideoClipSlotId | null) => void;
  onQueue: (slotId: VideoClipSlotId) => void | Promise<unknown>;
  onPreview: (slotId: VideoClipSlotId) => void | Promise<unknown>;
  onMore: (slotId: VideoClipSlotId | null, trigger: HTMLButtonElement) => void;
  onImport: () => void | Promise<unknown>;
  onCancelQueue: () => void | Promise<unknown>;
  onSetQuantization: (slotId: VideoClipSlotId, quantization: VideoClipLaunchQuantization) => void | Promise<unknown>;
  transitionKind: VideoClipTakeKind;
  transitionDurationUnit: VideoClipTakeDurationUnit;
  transitionDurationMilliunits: number;
  onSetTransitionKind: (kind: VideoClipTakeKind) => void;
  onSetTransitionDurationUnit: (unit: VideoClipTakeDurationUnit) => void;
  onSetTransitionDurationMilliunits: (value: number) => void;
}

const slotLabel = (index: number, slotId: VideoClipSlotId | undefined) =>
  slotId === undefined ? `Slot ${index + 1} empty` : `Slot ${index + 1}, ID ${slotId}`;

export function VideoClipSlotBankPanel(props: VideoClipSlotBankPanelProps) {
  // Runtime playheads refresh during keyboard navigation. Keep each pad's
  // identity so polling updates badges without replacing the focused button.
  const [cellRows, setCellRows] = createStore(videoClipSlotBankCells(props.layer?.clip_slots, props.runtime));
  createEffect(() => setCellRows(reconcile(videoClipSlotBankCells(props.layer?.clip_slots, props.runtime), { key: "index" })));
  const cells = () => cellRows;
  const [page, setPage] = createSignal(0);
  const pageSize = () => props.performance ? 4 : cells().length;
  const pageCount = () => Math.max(1, Math.ceil(cells().length / pageSize()));
  const visibleCells = () => cells().slice(page() * pageSize(), (page() + 1) * pageSize());
  // Compare stable identities before navigating: runtime snapshots replace the
  // layer summary frequently and must not reset the operator's visible page.
  const layerId = createMemo(() => props.layer?.id ?? null);
  const selectedSlotIndex = createMemo(() =>
    props.layer?.clip_slots?.findIndex((slot) => slot.id === props.selectedSlotId) ?? -1);
  createEffect(() => {
    layerId();
    setPage(0);
  });
  createEffect(() => {
    layerId();
    const selectedIndex = selectedSlotIndex();
    if (selectedIndex >= 0) setPage(Math.floor(selectedIndex / pageSize()));
  });
  const actions = createMemo(() => videoClipSlotBankActionLabels(props.mode));
  const selectedSlot = createMemo(() => props.layer?.clip_slots?.find((slot) => slot.id === props.selectedSlotId) ?? null);
  let bankRoot: HTMLElement | undefined;
  let bankGrid: HTMLDivElement | undefined;

  const moveFocus = (index: number, delta: number) => {
    const next = (index + delta + cells().length) % cells().length;
    setPage(Math.floor(next / pageSize()));
    queueMicrotask(() => {
      if (bankRoot?.isConnected) {
        bankRoot.querySelector<HTMLButtonElement>(`[data-video-clip-slot-pad-index="${next}"]`)?.focus();
      }
    });
  };

  const keyDown = (event: KeyboardEvent, index: number) => {
    const columns = Math.max(1, bankGrid
      ? getComputedStyle(bankGrid).gridTemplateColumns.split(" ").filter(Boolean).length
      : 1);
    if (event.key === "ArrowRight") { event.preventDefault(); moveFocus(index, 1); }
    if (event.key === "ArrowLeft") { event.preventDefault(); moveFocus(index, -1); }
    if (event.key === "ArrowDown") { event.preventDefault(); moveFocus(index, columns); }
    if (event.key === "ArrowUp") { event.preventDefault(); moveFocus(index, -columns); }
    if (event.key === "Home") { event.preventDefault(); moveFocus(index, -index); }
    if (event.key === "End") { event.preventDefault(); moveFocus(index, cells().length - 1 - index); }
    if (event.key === "Escape") { event.preventDefault(); props.onSelect(null); }
  };

  const primary = (slotId: VideoClipSlotId | null) => {
    if (slotId === null) {
      props.onSelect(null);
      return;
    }
    if (props.mode === "edit") props.onSelect(slotId);
    else {
      props.onSelect(slotId);
      void props.onQueue(slotId);
    }
  };

  return (
    <section
      ref={bankRoot}
      class="videoClipSlotBankPanel"
      aria-label={props.mode === "edit" ? "Video clip bank editor" : "Video clip bank control"}
      data-video-clip-slot-bank={props.mode}
      data-video-clip-slot-layer-id={props.layer?.id}
      data-video-clip-slot-has-layers={props.layers.length > 0 ? "true" : "false"}
    >
      <header class="videoClipSlotBankHeader">
        <div>
          <h3>Clip Bank</h3>
          <span>{props.layer ? `${props.layer.label} · ${props.layer.clip_slots?.length ?? 0}/32` : "Select a layer"}</span>
        </div>
        <span class="videoClipSlotRuntimeReadout" role="status">
          <Show when={props.runtime?.transition} fallback={
            <Show when={props.runtime?.active_slot_id !== null && props.runtime?.active_slot_id !== undefined} fallback="No active slot">
              Active #{props.runtime?.active_slot_id} · {props.runtime?.playhead_ms ?? 0} ms
            </Show>
          }>
            {(transition) => <>
              {transition().kind} #{transition().outgoing_slot_id} → #{transition().incoming_slot_id} · {Math.round(transition().progress_millis / 10)}%
              <progress
                max="1000"
                value={transition().progress_millis}
                aria-label={`${transition().kind} progress`}
              />
            </>}
          </Show>
        </span>
      </header>
      <div class="videoClipSlotBankToolbar">
        <Show when={props.mode !== "control" || props.layers.length > 0}>
        <label>
          Layer
          <select
            value={props.layer?.id ?? ""}
            disabled={props.layers.length === 0}
            onInput={(event) => props.onSelectLayer(Number(event.currentTarget.value))}
          >
            <For each={props.layers}>{(layer) => <option value={layer.id} data-no-localize>{layer.label}</option>}</For>
          </select>
        </label>
        </Show>
        <Show when={!props.performance}>
        <button type="button" onClick={() => void props.onImport()}>Import</button>
        </Show>
        <Show when={!props.performance && props.mode === "control" && props.layer != null}>
          <label>
            Launch quantization
            <select
              value={selectedSlot()?.launch_quantization ?? "Immediate"}
              disabled={!selectedSlot()}
              onInput={(event) => {
                const slot = selectedSlot();
                if (slot) void props.onSetQuantization(slot.id, event.currentTarget.value as VideoClipLaunchQuantization);
              }}
            >
              <option value="Immediate">Immediate</option>
              <option value="NextBeat">Next Beat</option>
              <option value="NextBar">Next Bar</option>
            </select>
          </label>
          <label>
            Transition
            <select
              value={props.transitionKind}
              disabled={props.runtime?.transition != null}
              onInput={(event) => props.onSetTransitionKind(event.currentTarget.value as VideoClipTakeKind)}
            >
              <option value="Cut">Cut</option>
              <option value="Crossfade">Crossfade</option>
              <option value="Dip">Dip</option>
              <option value="Wipe">Wipe</option>
              <option value="Luma">Luma</option>
              <option value="Displacement">Displacement</option>
              <option value="Blur">Blur</option>
              <option value="Glitch">Glitch</option>
              <option value="Custom">Custom FX</option>
            </select>
          </label>
          <label>
            Duration unit
            <select
              value={props.transitionDurationUnit}
              disabled={props.transitionKind === "Cut" || props.runtime?.transition != null}
              onInput={(event) => props.onSetTransitionDurationUnit(event.currentTarget.value as VideoClipTakeDurationUnit)}
            >
              <option value="Milliseconds">Milliseconds</option>
              <option value="Beats">Beats</option>
              <option value="Bars">Bars</option>
            </select>
          </label>
          <label>
            Transition duration
            <input
              type="number"
              min={props.transitionDurationUnit === "Milliseconds" ? "1" : "0.001"}
              max={props.transitionDurationUnit === "Milliseconds" ? "600000" : "600"}
              step={props.transitionDurationUnit === "Milliseconds" ? "50" : "0.25"}
              value={props.transitionDurationUnit === "Milliseconds"
                ? props.transitionDurationMilliunits
                : props.transitionDurationMilliunits / 1000}
              disabled={props.transitionKind === "Cut" || props.runtime?.transition != null}
              onInput={(event) => props.onSetTransitionDurationMilliunits(
                props.transitionDurationUnit === "Milliseconds"
                  ? Number(event.currentTarget.value)
                  : Number(event.currentTarget.value) * 1000,
              )}
            />
          </label>
          <button type="button" disabled={props.runtime?.queued_slot_id == null || props.runtime?.transition != null} onClick={() => void props.onCancelQueue()}>
            Cancel Queue
          </button>
        </Show>
      </div>
      <Show when={props.performance && props.layer != null}>
        <nav class="deckPager videoPerformSlotPager" aria-label="Clip slot pages">
          <span>{page() * pageSize() + 1}–{Math.min((page() + 1) * pageSize(), cells().length)} / {cells().length}</span>
          <button type="button" aria-label="Previous clip slots" disabled={page() === 0} onClick={() => setPage(page() - 1)}>Prev</button>
          <button type="button" aria-label="Next clip slots" disabled={page() >= pageCount() - 1} onClick={() => setPage(page() + 1)}>Next</button>
        </nav>
      </Show>
      <Show when={props.mode === "edit" || props.layer != null} fallback={
        <p class="videoClipSlotEmptyState">
          {props.layers.length > 0 ? "Choose a video layer to prepare clips." : "No video layer yet. Import media to prepare clips."}
        </p>
      }>
      <div ref={bankGrid} class="videoClipSlotBankGrid" role="grid" aria-label="32 video clip slots">
        <For each={visibleCells()}>{(cell) => {
          const slotId = () => cell.slot?.id ?? null;
          const selected = () => slotId() !== null && slotId() === props.selectedSlotId;
          const classes = () => [
            "videoClipSlotPad",
            cell.slot ? "occupied" : "empty",
            selected() ? "selected" : "",
            cell.active ? "active" : "",
            cell.queued ? "queued" : "",
            cell.pending ? "pending" : "",
          ].filter(Boolean).join(" ");
          return (
            <article
              class={classes()}
              role="gridcell"
              data-video-clip-slot-layer-id={props.layer?.id}
              data-video-clip-slot-id={slotId() ?? undefined}
              data-video-clip-slot-index={cell.index}
              aria-label={slotLabel(cell.index, slotId() ?? undefined)}
            >
              <button
                type="button"
                class="videoClipSlotPrimary"
                data-video-clip-slot-pad-index={cell.index}
                aria-label={`${actions()[0]} ${slotLabel(cell.index, slotId() ?? undefined)}`}
                aria-pressed={selected()}
                tabIndex={selected() || (!visibleCells().some((candidate) => candidate.slot?.id === props.selectedSlotId) && cell.index === page() * pageSize()) ? 0 : -1}
                onKeyDown={(event) => keyDown(event, cell.index)}
                onClick={() => primary(slotId())}
              >
                <strong>{cell.index + 1}</strong>
                <Show when={cell.slot} fallback={<span>Empty</span>}>
                  {(slot) => {
                    const asset = () => props.assets.find((candidate) => candidate.id === slot().media_asset_id);
                    return <>
                      <Show when={props.thumbnails[slot().media_asset_id]}>
                        {(thumbnail) => <img src={thumbnail()} alt="" aria-hidden="true" />}
                      </Show>
                      <span class="videoClipSlotPadTitle" title={asset()?.label ?? `Slot ${slot().id}`} data-no-localize>
                        {asset()?.label ?? `#${slot().id}`}
                      </span>
                    </>;
                  }}
                </Show>
                <Show when={cell.active}><em>LIVE</em></Show>
                <Show when={cell.queued}><em>QUEUED</em></Show>
              </button>
              <Show when={cell.slot}>
                {(slot) => (
                  <div class="videoClipSlotPadActions" role="group" aria-label={`Slot ${cell.index + 1} actions`}>
                    <button type="button" onClick={() => void props.onPreview(slot().id)} aria-label={`Audition slot ${slot().id} in Preview`}>Preview</button>
                    <button type="button" onClick={(event) => props.onMore(slot().id, event.currentTarget)} aria-label={`More actions for slot ${slot().id}`}>More</button>
                  </div>
                )}
              </Show>
              <Show when={!cell.slot}>
                <button type="button" onClick={(event) => props.onMore(null, event.currentTarget)} aria-label={`Create slot ${cell.index + 1}`}>More</button>
              </Show>
            </article>
          );
        }}</For>
      </div>
      </Show>
    </section>
  );
}
