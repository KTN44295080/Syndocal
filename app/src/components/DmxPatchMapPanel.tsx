import { createEffect, createMemo, createSignal, For, onCleanup, Show } from "solid-js";
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
  onNextFreeAddress: () => void;
  onUniverse: (universe: number) => void;
  onViewMode: (mode: DmxPatchViewMode) => void;
  onSelectFixture: (fixture: PatchedFixtureSummary) => void;
  onAddressCell: (cell: DmxAddressCell) => void;
  onProfileDragHover: (channel: number) => void;
  onProfileDragLeave: () => void;
  onProfileDrop: (channel: number) => void | Promise<void>;
}

export function DmxPatchMapPanel(props: DmxPatchMapPanelProps) {
  const [inspectedAddress, setInspectedAddress] = createSignal(1);
  const [activeAddress, setActiveAddress] = createSignal(1);
  const gridFragments = createMemo(() => segmentGridFragments(props.activeMap.segments));
  const addressRows = createMemo(() =>
    Array.from({ length: dmxGridRowCount }, (_, row) =>
      props.addressCells.slice(row * dmxGridColumnCount, (row + 1) * dmxGridColumnCount),
    ),
  );
  const fragmentByCell = createMemo(() =>
    new Map(gridFragments().map((fragment) => [`${fragment.row}:${fragment.column}`, fragment] as const)),
  );
  let addressGrid: HTMLDivElement | undefined;
  let addressGridResizeObserver: ResizeObserver | undefined;
  let addressGridResizeFrame = 0;
  let lastRevealedSelectionKey = "";

  const updateAddressGridCellSize = () => {
    const grid = addressGrid;
    const pane = grid?.parentElement;
    const header = pane?.querySelector<HTMLElement>(":scope > .panelHeader");
    if (!grid || !pane || !header || !grid.isConnected) return;

    const paneStyle = getComputedStyle(pane);
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

    const availableGridHeight = paneInnerHeight
      - header.getBoundingClientRect().height
      - cssPixelValue(paneStyle.rowGap);
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
  };

  const scheduleAddressGridCellSize = () => {
    cancelAnimationFrame(addressGridResizeFrame);
    addressGridResizeFrame = requestAnimationFrame(updateAddressGridCellSize);
  };

  const attachAddressGrid = (element: HTMLDivElement) => {
    addressGrid = element;
    addressGridResizeObserver?.disconnect();
    addressGridResizeObserver = new ResizeObserver(scheduleAddressGridCellSize);
    const pane = element.parentElement;
    if (pane) addressGridResizeObserver.observe(pane);
    const header = pane?.querySelector<HTMLElement>(":scope > .panelHeader");
    if (header) addressGridResizeObserver.observe(header);
    scheduleAddressGridCellSize();
  };

  onCleanup(() => {
    addressGridResizeObserver?.disconnect();
    cancelAnimationFrame(addressGridResizeFrame);
  });

  const scrollAddressIntoView = (channel: number): boolean => {
    const grid = addressGrid;
    const cell = grid?.querySelector<HTMLElement>(`[data-dmx-address="${channel}"]`);
    if (!grid || !cell) return false;

    const gridRect = grid.getBoundingClientRect();
    const cellRect = cell.getBoundingClientRect();
    const inset = 4;
    const visibleLeft = gridRect.left + grid.clientLeft;
    const visibleTop = gridRect.top + grid.clientTop;
    const visibleRight = visibleLeft + grid.clientWidth;
    const visibleBottom = visibleTop + grid.clientHeight;
    if (cellRect.top < visibleTop + inset) {
      grid.scrollTop += cellRect.top - visibleTop - inset;
    } else if (cellRect.bottom > visibleBottom - inset) {
      grid.scrollTop += cellRect.bottom - visibleBottom + inset;
    }
    if (cellRect.left < visibleLeft + inset) {
      grid.scrollLeft += cellRect.left - visibleLeft - inset;
    } else if (cellRect.right > visibleRight - inset) {
      grid.scrollLeft += cellRect.right - visibleRight + inset;
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

  const handleProfileDragOver = (
    event: DragEvent & { currentTarget: HTMLButtonElement },
    channel: number,
  ) => {
    if (!props.profileDragActive) return;
    event.preventDefault();
    if (event.dataTransfer) event.dataTransfer.dropEffect = "copy";
    props.onProfileDragHover(channel);
  };

  const handleProfileDragLeave = (event: DragEvent & { currentTarget: HTMLDivElement }) => {
    const nextTarget = event.relatedTarget;
    if (nextTarget instanceof Node && event.currentTarget.contains(nextTarget)) return;
    props.onProfileDragLeave();
  };

  return (
    <>
      <div class="dmxPatchMap">
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
            <details class="dmxUniverseOverviewDisclosure" data-dmx-universe-disclosure>
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
                              class={segment.fixture.id === props.selectedFixtureId ? "dmxPatchSegment selected" : "dmxPatchSegment"}
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
          </div>
        </div>
        <Show when={props.profileDndStatus}>
          {(status) => (
            <output
              class={`patchProfileDndStatus ${status()}`}
              data-patch-dnd-status={status()}
              role={status() === "conflict" || status() === "rejected" ? "alert" : "status"}
            >
              {status() === "valid" ? "Drop to patch" : "Conflict: drop rejected"}
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
                    class={segment.fixture.id === props.selectedFixtureId ? "dmxPatchListItem selected" : "dmxPatchListItem"}
                    onClick={() => props.onSelectFixture(segment.fixture)}
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
            onDragLeave={handleProfileDragLeave}
          >
            <For each={addressRows()}>
              {(cells, rowIndex) => (
                <div class="dmxAddressRow" role="row" aria-rowindex={rowIndex() + 1}>
                  <span class="dmxAddressRowLabel" aria-hidden="true">
                    {rowIndex() * dmxGridColumnCount + 1}–{(rowIndex() + 1) * dmxGridColumnCount}
                  </span>
                  <For each={cells}>
                    {(cell, columnIndex) => {
                      const fragment = () => fragmentByCell().get(`${rowIndex() + 1}:${columnIndex() + 1}`);
                      return (
                        <div
                          class="dmxAddressGridCell"
                          role="gridcell"
                          aria-colindex={columnIndex() + 1}
                          aria-selected={cell.isSelected}
                        >
                          <button
                            class={`dmxAddressCell ${cell.segment ? "occupied" : ""} ${cell.isStart ? "start" : ""} ${
                              cell.plannedIndex !== null ? "planned" : ""
                            } ${cell.plannedStart ? "plannedStart" : ""} ${
                              cell.plannedConflict ? "plannedConflict" : ""
                            } ${cell.isSelected ? "selected" : ""}`}
                            title={
                              cell.segment || cell.plannedIndex !== null
                                ? [
                                    `U${props.activeUniverse} A${cell.channel}`,
                                    cell.segment
                                      ? `${cell.segment.fixture.label} (${cell.segment.start}-${cell.segment.end})`
                                      : "",
                                    cell.plannedIndex !== null ? `Pending fixture ${cell.plannedIndex + 1}` : "",
                                  ]
                                    .filter(Boolean)
                                    .join(" / ")
                                : `U${props.activeUniverse} A${cell.channel}`
                            }
                            data-dmx-address={cell.channel}
                            tabIndex={activeAddress() === cell.channel ? 0 : -1}
                            onFocus={() => {
                              setActiveAddress(cell.channel);
                              setInspectedAddress(cell.channel);
                            }}
                            onPointerEnter={() => setInspectedAddress(cell.channel)}
                            onDragEnter={(event) => handleProfileDragOver(event, cell.channel)}
                            onDragOver={(event) => handleProfileDragOver(event, cell.channel)}
                            onDrop={(event) => {
                              if (!props.profileDragActive) return;
                              event.preventDefault();
                              void props.onProfileDrop(cell.channel);
                            }}
                            onKeyDown={(event) => handleAddressKeyDown(event, cell.channel)}
                            onClick={() => {
                              setActiveAddress(cell.channel);
                              setInspectedAddress(cell.channel);
                              props.onAddressCell(cell);
                            }}
                            aria-label={
                              [
                                `Address ${cell.channel}`,
                                cell.segment ? cell.segment.fixture.label : "empty",
                                cell.plannedIndex !== null ? `pending fixture ${cell.plannedIndex + 1}` : "",
                                cell.plannedConflict ? "conflict" : "",
                              ]
                                .filter(Boolean)
                                .join(", ")
                            }
                          />
                          <Show when={fragment()} keyed>
                            {(rowFragment) => (
                              <span
                                class={`dmxPatchFixtureBlock ${
                                  rowFragment.segment.fixture.id === props.selectedFixtureId ? "selected" : ""
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
                  </For>
                </div>
              )}
            </For>
          </div>
        </Show>
      </div>
    </>
  );
}
