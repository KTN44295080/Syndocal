import { createEffect, createMemo, createSignal, createUniqueId, For, Index, onCleanup, onMount, Show, untrack } from "solid-js";
import type { PatchedFixtureSummary } from "../types";

export type DmxPatchViewMode = "grid" | "list";
export type PatchProfileDndStatus = "valid" | "conflict" | "rejected" | null;

export interface DmxPatchSegment {
  fixture: PatchedFixtureSummary;
  start: number;
  end: number;
  left: number;
  width: number;
}

export interface DmxUniverseMap {
  universe: number;
  used: number;
  free: number;
  largestFree: number;
  segments: DmxPatchSegment[];
}

export interface DmxAddressCell {
  channel: number;
  segment: DmxPatchSegment | null;
  isStart: boolean;
  isSelected: boolean;
  plannedIndex: number | null;
  plannedStart: boolean;
  plannedConflict: boolean;
}

interface DmxPatchGridFragment {
  segment: DmxPatchSegment;
  row: number;
  column: number;
  span: number;
  continuation: boolean;
}

interface FixturePointerDragState {
  fixture: PatchedFixtureSummary;
  pointerId: number;
  startX: number;
  startY: number;
  active: boolean;
  grabOffset: number;
}

const dmxGridColumnCount = 32;
const dmxGridRowCount = 16;
const dmxGridMinimumCellSize = 16;
const dmxGridMaximumCellSize = 44;

const cssPixelValue = (value: string): number => Number.parseFloat(value) || 0;

const segmentGridFragments = (segments: DmxPatchSegment[]): DmxPatchGridFragment[] =>
  segments.flatMap((segment) => {
    const fragments: DmxPatchGridFragment[] = [];
    let channel = segment.start;
    while (channel <= segment.end) {
      const row = Math.floor((channel - 1) / dmxGridColumnCount) + 1;
      const column = ((channel - 1) % dmxGridColumnCount) + 1;
      const rowEnd = Math.min(segment.end, row * dmxGridColumnCount);
      fragments.push({
        segment,
        row,
        column,
        span: rowEnd - channel + 1,
        continuation: channel !== segment.start,
      });
      channel = rowEnd + 1;
    }
    return fragments;
  });

interface DmxPatchMapPanelProps {
  activeUniverse: number;
  universeOptions: number[];
  viewMode: DmxPatchViewMode;
  nextFreeAddress: number | null;
  activeMap: DmxUniverseMap;
  addressCells: DmxAddressCell[];
  universeMaps: DmxUniverseMap[];
  fixtureCount: number;
  selectedFixtureId: number | null;
  plannedAddressSummary: string;
  profileDragActive: boolean;
  profileDndStatus: PatchProfileDndStatus;
  fixtureDragActive: boolean;
  fixtureDndStatus: PatchProfileDndStatus;
  onNextFreeAddress: () => void;
  onUniverse: (universe: number) => void;
  onViewMode: (mode: DmxPatchViewMode) => void;
  onSelectFixture: (fixture: PatchedFixtureSummary) => void;
  onAddressCell: (cell: DmxAddressCell) => void;
  onProfileDragHover: (channel: number) => void;
  onProfileDragLeave: () => void;
  onProfileDrop: (channel: number) => void | Promise<void>;
  onFixtureDragStart: (fixture: PatchedFixtureSummary, selection?: PatchedFixtureSummary[]) => void;
  onFixtureDragEnd: () => void;
  onFixtureDragHover: (channel: number) => void;
  onFixtureDragLeave: () => void;
  onFixtureDrop: (channel: number) => void | Promise<void>;
}

export function DmxPatchMapPanel(props: DmxPatchMapPanelProps) {
  const [inspectedAddress, setInspectedAddress] = createSignal(1);
  const [activeAddress, setActiveAddress] = createSignal(1);
  const [selectedPatchIds, setSelectedPatchIds] = createSignal(new Set<number>());
  const [moveAddress, setMoveAddress] = createSignal(1);
  const [selectionExpanded, setSelectionExpanded] = createSignal(false);
  const selectionControlsId = createUniqueId();
  let selectionAnchor: number | null = null;
  const selectedPatches = () => props.activeMap.segments.map(segment => segment.fixture).filter(fixture => selectedPatchIds().has(fixture.id));
  const patchSelected = (id: number) => selectedPatchIds().has(id);
  const cellSelected = (cell: DmxAddressCell) => Boolean(cell.segment && patchSelected(cell.segment.fixture.id));
  createEffect(() => {
    const id = props.selectedFixtureId;
    if (id === null) setSelectedPatchIds(new Set<number>());
    else if (!untrack(selectedPatchIds).has(id)) setSelectedPatchIds(new Set([id]));
  });
  const selectPatch = (fixture: PatchedFixtureSummary, event: MouseEvent) => {
    const ordered = props.activeMap.segments.map(segment => segment.fixture.id);
    if (event.shiftKey && selectionAnchor !== null && ordered.includes(selectionAnchor)) {
      const a = ordered.indexOf(selectionAnchor), b = ordered.indexOf(fixture.id);
      setSelectedPatchIds(new Set(ordered.slice(Math.min(a,b),Math.max(a,b)+1)));
    } else if (event.ctrlKey || event.metaKey) {
      const next = new Set(selectedPatchIds());
      if (next.has(fixture.id)) next.delete(fixture.id); else next.add(fixture.id);
      setSelectedPatchIds(next); selectionAnchor = fixture.id;
    } else { setSelectedPatchIds(new Set([fixture.id])); selectionAnchor = fixture.id; }
    const primary = selectedPatches().find(f => f.id === fixture.id) ?? selectedPatches()[0];
    if (primary) props.onSelectFixture(primary);
  };
  const gridFragments = createMemo(() => segmentGridFragments(props.activeMap.segments));
  const addressRows = createMemo(() =>
    Array.from({ length: dmxGridRowCount }, (_, row) =>
      props.addressCells.slice(row * dmxGridColumnCount, (row + 1) * dmxGridColumnCount),
    ),
  );
  const fragmentByCell = createMemo(() =>
    new Map(gridFragments().map((fragment) => [`${fragment.row}:${fragment.column}`, fragment] as const)),
  );
  let patchMap: HTMLDivElement | undefined;
  let addressGrid: HTMLDivElement | undefined;
  let universeOverview: HTMLDetailsElement | undefined;
  let patchLayoutResizeObserver: ResizeObserver | undefined;
  let patchLayoutResizeFrame = 0;
  let selectionFocusTarget: HTMLElement | undefined;
  let lastRevealedSelectionKey = "";
  let fixturePointerDrag: FixturePointerDragState | null = null;
  let suppressNextFixtureClick = false;

  const updatePatchLayout = () => {
    const pane = patchMap;
    const header = pane?.querySelector<HTMLElement>(":scope > .panelHeader");
    if (!pane || !header || !pane.isConnected) return;

    const paneStyle = getComputedStyle(pane);
    const paneRect = pane.getBoundingClientRect();
    const headerRect = header.getBoundingClientRect();
    const zoom = pane.offsetWidth > 0 ? paneRect.width / pane.offsetWidth : 1;
    const paneTop = paneRect.top + cssPixelValue(paneStyle.borderTopWidth) * zoom;
    const popupTop = Math.max(0, paneTop + cssPixelValue(paneStyle.paddingTop) * zoom);
    const popupBottom = Math.min(window.innerHeight, paneRect.bottom
      - (cssPixelValue(paneStyle.borderBottomWidth) + cssPixelValue(paneStyle.paddingBottom)) * zoom);
    pane.style.setProperty("--dmx-overview-max-height", `${Math.max(0, (popupBottom - headerRect.bottom) / zoom - 4)}px`);

    const selectionControls = pane.querySelector<HTMLElement>(":scope > .dmxPatchSelectionControls");
    const selectionToggle = pane.querySelector<HTMLButtonElement>("[data-patch-selection-toggle]");
    if (selectionToggle?.offsetWidth) {
      if (!selectionExpanded() && (selectionControls?.contains(document.activeElement)
        || (selectionFocusTarget && selectionControls?.contains(selectionFocusTarget)))) {
        selectionToggle.focus({ preventScroll: true });
      }
    } else if (selectionToggle && selectionFocusTarget === selectionToggle) {
      selectionControls?.querySelector<HTMLButtonElement>("button")?.focus({ preventScroll: true });
    }
    if (selectionExpanded() && selectionControls && selectionToggle?.offsetWidth) {
      const toggleRect = selectionToggle.getBoundingClientRect();
      const above = Math.max(0, (toggleRect.top - popupTop) / zoom - 4);
      const below = Math.max(0, (popupBottom - toggleRect.bottom) / zoom - 4);
      const selectionStyle = getComputedStyle(selectionControls);
      const naturalHeight = selectionControls.scrollHeight
        + cssPixelValue(selectionStyle.borderTopWidth) + cssPixelValue(selectionStyle.borderBottomWidth);
      const opensAbove = below < naturalHeight && above > below;
      const height = Math.min(360, naturalHeight, opensAbove ? above : below);
      const top = opensAbove
        ? (toggleRect.top - paneTop) / zoom - 4 - height
        : (toggleRect.bottom - paneTop) / zoom + 4;
      pane.style.setProperty("--dmx-selection-top", `${top}px`);
      pane.style.setProperty("--dmx-selection-max-height", `${height}px`);
    }

    const grid = addressGrid;
    if (!grid?.isConnected) return;

    const gridStyle = getComputedStyle(grid);
    const paneInnerWidth = pane.clientWidth
      - cssPixelValue(paneStyle.paddingLeft)
      - cssPixelValue(paneStyle.paddingRight);
    const paneInnerHeight = pane.clientHeight
      - cssPixelValue(paneStyle.paddingTop)
      - cssPixelValue(paneStyle.paddingBottom);
    const gridGap = cssPixelValue(gridStyle.getPropertyValue("--dmx-grid-gap"));
    const rowHeaderWidth = cssPixelValue(gridStyle.getPropertyValue("--dmx-row-label-width"));
    const paneContentWidth = Math.max(0, paneInnerWidth - dmxGridColumnCount * gridGap);
    const widthLimitedCellSize = Math.floor(
      (paneContentWidth - rowHeaderWidth) / dmxGridColumnCount,
    );

    const inlineSelectionHeight = selectionControls && getComputedStyle(selectionControls).position !== "absolute"
      ? selectionControls.offsetHeight : 0;
    const availableGridHeight = paneInnerHeight
      - header.offsetHeight
      - inlineSelectionHeight
      - (inlineSelectionHeight > 0 ? 2 : 1) * cssPixelValue(paneStyle.rowGap);
    const gridBlockChrome = cssPixelValue(gridStyle.borderTopWidth)
      + cssPixelValue(gridStyle.borderBottomWidth)
      + cssPixelValue(gridStyle.paddingTop)
      + cssPixelValue(gridStyle.paddingBottom)
      + (dmxGridRowCount - 1) * gridGap;
    const heightLimitedCellSize = Math.floor(
      (availableGridHeight - gridBlockChrome) / dmxGridRowCount,
    );
    const paneLimitedCellSize = heightLimitedCellSize >= dmxGridMinimumCellSize
      ? Math.min(widthLimitedCellSize, heightLimitedCellSize)
      : widthLimitedCellSize;
    const cellSize = Math.min(
      dmxGridMaximumCellSize,
      Math.max(dmxGridMinimumCellSize, paneLimitedCellSize),
    );
    grid.style.setProperty("--dmx-cell-size", `${cellSize}px`);
    // Status overlays the grid without moving a live drag target when it appears.
    pane.style.setProperty("--dmx-dnd-top", `${grid.offsetTop + 4}px`);
  };

  const schedulePatchLayout = () => {
    cancelAnimationFrame(patchLayoutResizeFrame);
    patchLayoutResizeFrame = requestAnimationFrame(updatePatchLayout);
  };

  const attachAddressGrid = (element: HTMLDivElement) => {
    addressGrid = element;
    schedulePatchLayout();
  };

  onMount(() => {
    patchLayoutResizeObserver = new ResizeObserver(schedulePatchLayout);
    const pane = patchMap;
    if (pane) patchLayoutResizeObserver.observe(pane);
    const header = pane?.querySelector<HTMLElement>(":scope > .panelHeader");
    if (header) patchLayoutResizeObserver.observe(header);
    const selectionControls = pane?.querySelector<HTMLElement>(":scope > .dmxPatchSelectionControls");
    if (selectionControls) patchLayoutResizeObserver.observe(selectionControls);
    schedulePatchLayout();
  });

  onCleanup(() => {
    patchLayoutResizeObserver?.disconnect();
    cancelAnimationFrame(patchLayoutResizeFrame);
    if (fixturePointerDrag) props.onFixtureDragEnd();
    fixturePointerDrag = null;
  });

  const scrollAddressIntoView = (channel: number): boolean => {
    const grid = addressGrid;
    const cell = grid?.querySelector<HTMLElement>(`[data-dmx-address="${channel}"]`);
    if (!grid || !cell) return false;

    const gridRect = grid.getBoundingClientRect();
    const cellRect = cell.getBoundingClientRect();
    const zoom = grid.offsetWidth > 0 ? gridRect.width / grid.offsetWidth : 1;
    const inset = 4 * zoom;
    const visibleLeft = gridRect.left + grid.clientLeft * zoom;
    const visibleTop = gridRect.top + grid.clientTop * zoom;
    const visibleRight = visibleLeft + grid.clientWidth * zoom;
    const visibleBottom = visibleTop + grid.clientHeight * zoom;
    if (cellRect.top < visibleTop + inset) {
      grid.scrollTop += (cellRect.top - visibleTop - inset) / zoom;
    } else if (cellRect.bottom > visibleBottom - inset) {
      grid.scrollTop += (cellRect.bottom - visibleBottom + inset) / zoom;
    }
    if (cellRect.left < visibleLeft + inset) {
      grid.scrollLeft += (cellRect.left - visibleLeft - inset) / zoom;
    } else if (cellRect.right > visibleRight - inset) {
      grid.scrollLeft += (cellRect.right - visibleRight + inset) / zoom;
    }
    return true;
  };

  const revealAddress = (channel: number) => {
    setInspectedAddress(channel);
    scrollAddressIntoView(channel);
    requestAnimationFrame(() => requestAnimationFrame(() => scrollAddressIntoView(channel)));
  };

  const focusAddress = (channel: number) => {
    const nextAddress = Math.min(512, Math.max(1, channel));
    setActiveAddress(nextAddress);
    addressGrid
      ?.querySelector<HTMLButtonElement>(`[data-dmx-address="${nextAddress}"]`)
      ?.focus({ preventScroll: true });
    revealAddress(nextAddress);
  };

  const handleAddressKeyDown = (event: KeyboardEvent, channel: number) => {
    if (event.key === "Escape") { event.preventDefault(); cancelFixturePointerDrag(); return; }
    const column = (channel - 1) % dmxGridColumnCount;
    const rowStart = channel - column;
    let nextAddress: number | null = null;
    switch (event.key) {
      case "ArrowLeft":
        if (column > 0) nextAddress = channel - 1;
        break;
      case "ArrowRight":
        if (column < dmxGridColumnCount - 1) nextAddress = channel + 1;
        break;
      case "ArrowUp":
        if (channel > dmxGridColumnCount) nextAddress = channel - dmxGridColumnCount;
        break;
      case "ArrowDown":
        if (channel <= 512 - dmxGridColumnCount) nextAddress = channel + dmxGridColumnCount;
        break;
      case "Home":
        nextAddress = event.ctrlKey || event.metaKey ? 1 : rowStart;
        break;
      case "End":
        nextAddress = event.ctrlKey || event.metaKey ? 512 : rowStart + dmxGridColumnCount - 1;
        break;
      default:
        break;
    }
    if (nextAddress === null || nextAddress === channel) return;
    event.preventDefault();
    focusAddress(nextAddress);
  };

  createEffect(() => {
    const selectedId = props.selectedFixtureId;
    const selectedSegment = props.activeMap.segments.find((segment) => segment.fixture.id === selectedId);
    const selectionKey = `${props.activeUniverse}:${selectedId ?? "none"}:${selectedSegment?.start ?? "none"}`;
    if (selectionKey === lastRevealedSelectionKey) return;
    lastRevealedSelectionKey = selectionKey;
    if (selectedSegment) {
      setActiveAddress(selectedSegment.start);
      revealAddress(selectedSegment.start);
    }
  });

  const useNextFreeAddress = () => {
    const nextFreeAddress = props.nextFreeAddress;
    props.onNextFreeAddress();
    if (nextFreeAddress !== null) revealAddress(nextFreeAddress);
  };

  const channelAtPoint = (clientX: number, clientY: number): number | null => {
    const target = document.elementFromPoint(clientX, clientY);
    const addressCell = target instanceof HTMLElement
      ? target.closest<HTMLElement>("[data-dmx-address]")
      : null;
    const channel = Number(addressCell?.dataset.dmxAddress);
    return Number.isInteger(channel) && channel >= 1 && channel <= 512 ? channel : null;
  };

  const handleFixturePointerDown = (
    event: PointerEvent & { currentTarget: HTMLButtonElement },
    fixture: PatchedFixtureSummary,
  ) => {
    if (event.button !== 0 || fixturePointerDrag) return;
    fixturePointerDrag = {
      fixture,
      pointerId: event.pointerId,
      startX: event.clientX,
      startY: event.clientY,
      active: false,
      grabOffset: Math.max(0, Number(event.currentTarget.dataset.dmxAddress) - fixture.address),
    };
    event.currentTarget.setPointerCapture?.(event.pointerId);
  };

  const handleFixturePointerMove = (event: PointerEvent) => {
    const drag = fixturePointerDrag;
    if (!drag || drag.pointerId !== event.pointerId) return;
    const distance = Math.hypot(event.clientX - drag.startX, event.clientY - drag.startY);
    if (!drag.active && distance < 5) return;
    if (!drag.active) {
      drag.active = true;
      suppressNextFixtureClick = true;
      if (!patchSelected(drag.fixture.id)) {
        setSelectedPatchIds(new Set([drag.fixture.id]));
        props.onSelectFixture(drag.fixture);
      }
      props.onFixtureDragStart(drag.fixture, selectedPatches());
    }
    event.preventDefault();
    const channel = channelAtPoint(event.clientX, event.clientY);
    if (channel === null) {
      props.onFixtureDragLeave();
    } else {
      props.onFixtureDragHover(channel - drag.grabOffset);
    }
  };

  const handleFixturePointerEnd = (event: PointerEvent) => {
    const drag = fixturePointerDrag;
    if (!drag || drag.pointerId !== event.pointerId) return;
    fixturePointerDrag = null;
    if (!drag.active) return;
    event.preventDefault();
    const channel = channelAtPoint(event.clientX, event.clientY);
    if (channel === null) {
      props.onFixtureDragEnd();
      return;
    }
    void props.onFixtureDrop(channel - drag.grabOffset);
  };

  const cancelFixturePointerDrag = () => {
    fixturePointerDrag = null;
    suppressNextFixtureClick = false;
    props.onFixtureDragEnd();
  };

  const handlePatchDragOver = (
    event: DragEvent & { currentTarget: HTMLButtonElement },
    channel: number,
  ) => {
    if (props.fixtureDragActive) {
      event.preventDefault();
      if (event.dataTransfer) event.dataTransfer.dropEffect = "move";
      props.onFixtureDragHover(channel);
      return;
    }
    if (!props.profileDragActive) return;
    event.preventDefault();
    if (event.dataTransfer) event.dataTransfer.dropEffect = "copy";
    props.onProfileDragHover(channel);
  };

  const handlePatchDragLeave = (event: DragEvent & { currentTarget: HTMLDivElement }) => {
    const nextTarget = event.relatedTarget;
    if (nextTarget instanceof Node && event.currentTarget.contains(nextTarget)) return;
    if (props.fixtureDragActive) {
      props.onFixtureDragLeave();
    } else if (props.profileDragActive) {
      props.onProfileDragLeave();
    }
  };

  const activeDndStatus = () => props.fixtureDragActive ? props.fixtureDndStatus : props.profileDndStatus;

  return (
    <>
      <div class="dmxPatchMap withFixtureSelection" ref={(element) => { patchMap = element; }}
        onFocusIn={(event) => {
          const target = event.target;
          selectionFocusTarget = target instanceof HTMLElement
            && (target.matches("[data-patch-selection-toggle]") || target.closest(".dmxPatchSelectionControls"))
            ? target : undefined;
        }}
        onFocusOut={(event) => {
          // A responsive display:none may blur before ResizeObserver runs.
          // Retain that target until its visible counterpart receives focus.
          if (event.target === selectionFocusTarget && selectionFocusTarget.offsetHeight > 0) {
            selectionFocusTarget = undefined;
          }
        }}
        onKeyDown={(event) => {
          if (event.key === "Escape" && selectionExpanded()) {
            event.preventDefault();
            event.stopPropagation();
            setSelectionExpanded(false);
            patchMap?.querySelector<HTMLButtonElement>("[data-patch-selection-toggle]")?.focus({ preventScroll: true });
          }
        }}>
        <div class="panelHeader">
          <h3>DMX Patch Grid</h3>
          <output class="dmxPatchAddressReadout" aria-label="Current DMX address">
            U{props.activeUniverse} A{inspectedAddress()}
          </output>
          <output class="dmxGridUsage" data-dmx-grid-usage>
            {props.activeMap.used}/512 used / {props.activeMap.largestFree}ch max free
          </output>
          <div class="panelHeaderActions">
            <button onClick={useNextFreeAddress} disabled={props.nextFreeAddress === null}>
              Next Free
            </button>
            <select
              value={props.activeUniverse}
              aria-label="DMX universe"
              onInput={(event) => {
                setInspectedAddress(1);
                setActiveAddress(1);
                addressGrid?.scrollTo({ left: 0, top: 0 });
                props.onUniverse(Number(event.currentTarget.value));
              }}
            >
              <For each={props.universeOptions}>
                {(universeId) => <option value={universeId}>Universe {universeId}</option>}
              </For>
            </select>
            <div class="viewToggle" role="group" aria-label="DMX map view">
              <button
                class={props.viewMode === "grid" ? "active" : ""}
                onClick={() => props.onViewMode("grid")}
                aria-pressed={props.viewMode === "grid"}
              >
                Grid
              </button>
              <button
                class={props.viewMode === "list" ? "active" : ""}
                onClick={() => props.onViewMode("list")}
                aria-pressed={props.viewMode === "list"}
              >
                List
              </button>
            </div>
            <details class="dmxUniverseOverviewDisclosure" data-dmx-universe-disclosure
              ref={(element) => { universeOverview = element; }}
              onToggle={(event) => {
                if (event.currentTarget.open) setSelectionExpanded(false);
                schedulePatchLayout();
              }}>
              <summary>Universe Overview</summary>
              <div class="dmxUniverseOverviewContent">
                <div class="dmxUniverseOverviewMeta">
                  <span>{props.universeMaps.length} universe(s)</span>
                  <span>{props.fixtureCount} fixture(s)</span>
                  <span>{props.plannedAddressSummary}</span>
                </div>
                <For each={props.universeMaps}>
                  {(map) => (
                    <div class="dmxUniverseMap">
                      <div class="dmxUniverseHeader">
                        <strong>U{map.universe}</strong>
                        <span>{map.used}/512 used</span>
                        <span>{map.largestFree}ch max free</span>
                      </div>
                      <div class="dmxUniverseTrack">
                        <For each={map.segments}>
                          {(segment) => (
                            <button
                              class={patchSelected(segment.fixture.id) ? "dmxPatchSegment selected" : "dmxPatchSegment"}
                              style={{
                                left: `${segment.left}%`,
                                width: `${Math.max(segment.width, 0.7)}%`,
                              }}
                              title={`${segment.fixture.label} A${segment.start}-${segment.end}`}
                              aria-label={`Select ${segment.fixture.label}, Universe ${map.universe}, addresses ${segment.start} to ${segment.end}`}
                              onClick={() => {
                                props.onUniverse(map.universe);
                                props.onSelectFixture(segment.fixture);
                              }}
                            />
                          )}
                        </For>
                      </div>
                      <div class="dmxUniverseLegend">
                        <For each={map.segments}>
                          {(segment) => (
                            <button
                              onClick={() => {
                                props.onUniverse(map.universe);
                                props.onSelectFixture(segment.fixture);
                              }}
                            >
                              <span data-no-localize>A{segment.start}-{segment.end} {segment.fixture.label}</span>
                            </button>
                          )}
                        </For>
                        <Show when={map.segments.length === 0}>
                          <span>Empty</span>
                        </Show>
                      </div>
                    </div>
                  )}
                </For>
              </div>
            </details>
            <button class="dmxPatchSelectionToggle" data-patch-selection-toggle
              aria-expanded={selectionExpanded()} aria-controls={selectionControlsId}
              onClick={() => {
                if (universeOverview) universeOverview.open = false;
                setSelectionExpanded(!selectionExpanded());
                schedulePatchLayout();
              }}>
              Selection and move ({selectedPatches().length})
            </button>
          </div>
        </div>
        <div class="dmxPatchSelectionControls" classList={{ expanded: selectionExpanded() }}
          id={selectionControlsId} aria-label="Patch selection and move">
          <output>{selectedPatches().length} selected</output>
          <button onClick={() => setSelectedPatchIds(new Set(props.activeMap.segments.map(segment => segment.fixture.id)))}>Select All</button>
          <button onClick={() => setSelectedPatchIds(new Set<number>())} disabled={selectedPatches().length === 0}>Clear Selection</button>
          <label>Move to address <input type="number" min="1" max="512" value={moveAddress()} aria-label="Move selected fixtures to address" onInput={(event) => setMoveAddress(Number(event.currentTarget.value))} /></label>
          <button disabled={selectedPatches().length === 0} onClick={() => {
            const fixtures = selectedPatches();
            const anchor = fixtures.reduce((first, fixture) => fixture.address < first.address ? fixture : first);
            props.onFixtureDragStart(anchor, fixtures); void props.onFixtureDrop(moveAddress());
          }}>Move Selected</button>
          <span>Ctrl / Cmd: toggle · Shift: range · Drag: move</span>
        </div>
        <Show when={activeDndStatus()}>
          {(status) => (
            <output
              class={`patchProfileDndStatus ${status()}`}
              data-patch-dnd-status={status()}
              role={status() === "conflict" || status() === "rejected" ? "alert" : "status"}
            >
              {status() === "valid"
                ? props.fixtureDragActive ? "Drop to move" : "Drop to patch"
                : props.fixtureDragActive ? "Conflict: move rejected" : "Conflict: drop rejected"}
            </output>
          )}
        </Show>
        <Show
          when={props.viewMode === "grid"}
          fallback={
            <div class="dmxPatchList">
              <For each={props.activeMap.segments}>
                {(segment) => (
                  <button
                    class={patchSelected(segment.fixture.id) ? "dmxPatchListItem selected" : "dmxPatchListItem"}
                    onClick={(event) => selectPatch(segment.fixture, event)}
                    aria-pressed={patchSelected(segment.fixture.id)}
                  >
                    <strong data-no-localize>{segment.fixture.label}</strong>
                    <span>
                      U{props.activeUniverse} A{segment.start}-{segment.end} / {segment.end - segment.start + 1}ch
                    </span>
                    <small data-no-localize>{segment.fixture.manufacturer} {segment.fixture.profile_name}</small>
                  </button>
                )}
              </For>
              <Show when={props.activeMap.segments.length === 0}>
                <p class="empty">No fixtures in this universe. Use Setup &gt; Patch to assign one.</p>
              </Show>
            </div>
          }
        >
          <div
            class="dmxAddressGrid"
            ref={attachAddressGrid}
            role="grid"
            aria-label={`Universe ${props.activeUniverse} DMX addresses 1 to 512`}
            aria-rowcount={dmxGridRowCount}
            aria-colcount={dmxGridColumnCount}
            onDragLeave={handlePatchDragLeave}
            onPointerMove={handleFixturePointerMove}
            onPointerUp={handleFixturePointerEnd}
            onPointerCancel={cancelFixturePointerDrag}
          >
            <Index each={addressRows()}>
              {(cells, rowIndex) => (
                <div class="dmxAddressRow" role="row" aria-rowindex={rowIndex + 1}>
                  <span class="dmxAddressRowLabel" aria-hidden="true">
                    {rowIndex * dmxGridColumnCount + 1}–{(rowIndex + 1) * dmxGridColumnCount}
                  </span>
                  <Index each={cells()}>
                    {(cell, columnIndex) => {
                      const fragment = () => fragmentByCell().get(`${rowIndex + 1}:${columnIndex + 1}`);
                      return (
                        <div
                          class="dmxAddressGridCell"
                          role="gridcell"
                          aria-colindex={columnIndex + 1}
                          aria-selected={cellSelected(cell())}
                        >
                          <button
                            class={`dmxAddressCell ${cell().segment ? "occupied" : ""} ${cell().isStart ? "start" : ""} ${
                              cell().plannedIndex !== null ? "planned" : ""
                            } ${cell().plannedStart ? "plannedStart" : ""} ${
                              cell().plannedConflict ? "plannedConflict" : ""
                            } ${cellSelected(cell()) ? "selected" : ""}`}
                            title={
                              cell().segment || cell().plannedIndex !== null
                                ? [
                                    `U${props.activeUniverse} A${cell().channel}`,
                                    cell().segment
                                      ? `${cell().segment!.fixture.label} (${cell().segment!.start}-${cell().segment!.end})`
                                      : "",
                                    cell().plannedIndex !== null ? `Pending fixture ${cell().plannedIndex! + 1}` : "",
                                  ]
                                    .filter(Boolean)
                                    .join(" / ")
                                : `U${props.activeUniverse} A${cell().channel}`
                            }
                            data-dmx-address={cell().channel}
                            draggable={false}
                            tabIndex={activeAddress() === cell().channel ? 0 : -1}
                            onFocus={() => {
                              setActiveAddress(cell().channel);
                              setInspectedAddress(cell().channel);
                            }}
                            onPointerDown={(event) => {
                              const fixture = cell().segment?.fixture;
                              if (fixture) handleFixturePointerDown(event, fixture);
                            }}
                            onPointerEnter={() => setInspectedAddress(cell().channel)}
                            onDragStart={(event) => event.preventDefault()}
                            onDragEnd={props.onFixtureDragEnd}
                            onDragEnter={(event) => handlePatchDragOver(event, cell().channel)}
                            onDragOver={(event) => handlePatchDragOver(event, cell().channel)}
                            onDrop={(event) => {
                              if (props.fixtureDragActive) {
                                event.preventDefault();
                                void props.onFixtureDrop(cell().channel);
                                return;
                              }
                              if (!props.profileDragActive) return;
                              event.preventDefault();
                              void props.onProfileDrop(cell().channel);
                            }}
                            onKeyDown={(event) => handleAddressKeyDown(event, cell().channel)}
                            onClick={(event) => {
                              if (suppressNextFixtureClick) {
                                suppressNextFixtureClick = false;
                                return;
                              }
                              setActiveAddress(cell().channel);
                              setInspectedAddress(cell().channel);
                              if (cell().segment) {
                                selectPatch(cell().segment!.fixture, event);
                                if (!event.ctrlKey && !event.metaKey && !event.shiftKey) props.onAddressCell(cell());
                              } else props.onAddressCell(cell());
                            }}
                            aria-label={
                              [
                                `Address ${cell().channel}`,
                                cell().segment ? cell().segment!.fixture.label : "empty",
                                cell().plannedIndex !== null ? `pending fixture ${cell().plannedIndex! + 1}` : "",
                                cell().plannedConflict ? "conflict" : "",
                              ]
                                .filter(Boolean)
                                .join(", ")
                            }
                          />
                          <Show when={fragment()} keyed>
                            {(rowFragment) => (
                              <span
                                class={`dmxPatchFixtureBlock ${
                                  patchSelected(rowFragment.segment.fixture.id) ? "selected" : ""
                                } ${rowFragment.span <= 2 ? "tiny" : rowFragment.span <= 5 ? "narrow" : ""}`}
                                style={{
                                  left: `calc(var(--dmx-row-label-width) + var(--dmx-grid-gap) + ${
                                    rowFragment.column - 1
                                  } * (var(--dmx-cell-size) + var(--dmx-grid-gap)))`,
                                  width: `calc(${rowFragment.span} * var(--dmx-cell-size) + ${
                                    Math.max(0, rowFragment.span - 1)
                                  } * var(--dmx-grid-gap))`,
                                }}
                                title={`${rowFragment.segment.fixture.label} / A${rowFragment.segment.start} / ${
                                  rowFragment.segment.end - rowFragment.segment.start + 1
                                }ch`}
                                aria-hidden="true"
                              >
                                <strong data-no-localize>
                                  {rowFragment.continuation
                                    ? `> ${rowFragment.segment.fixture.label}`
                                    : rowFragment.segment.fixture.label}
                                </strong>
                                <small>A{rowFragment.segment.start} / {rowFragment.segment.end - rowFragment.segment.start + 1}ch</small>
                              </span>
                            )}
                          </Show>
                        </div>
                      );
                    }}
                  </Index>
                </div>
              )}
            </Index>
          </div>
        </Show>
      </div>
    </>
  );
}
