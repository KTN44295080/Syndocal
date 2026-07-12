import { createEffect, createSignal, For, Show } from "solid-js";
import type { PatchedFixtureSummary } from "../types";

export type DmxPatchViewMode = "grid" | "list";

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
const dmxAddressPageSize = 128;
const dmxAddressPageCount = 512 / dmxAddressPageSize;

const segmentGridFragments = (segments: DmxPatchSegment[], page: number): DmxPatchGridFragment[] => {
  const pageStart = page * dmxAddressPageSize + 1;
  const pageEnd = pageStart + dmxAddressPageSize - 1;
  return (
  segments.flatMap((segment) => {
    const fragments: DmxPatchGridFragment[] = [];
    let channel = Math.max(segment.start, pageStart);
    const clippedEnd = Math.min(segment.end, pageEnd);
    while (channel <= clippedEnd) {
      const localChannel = channel - pageStart;
      const row = Math.floor(localChannel / dmxGridColumnCount) + 1;
      const column = (localChannel % dmxGridColumnCount) + 1;
      const rowEnd = Math.min(clippedEnd, pageStart + row * dmxGridColumnCount - 1);
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
  })
  );
};

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
  onNextFreeAddress: () => void;
  onUniverse: (universe: number) => void;
  onViewMode: (mode: DmxPatchViewMode) => void;
  onSelectFixture: (fixture: PatchedFixtureSummary) => void;
  onAddressCell: (cell: DmxAddressCell) => void;
}

export function DmxPatchMapPanel(props: DmxPatchMapPanelProps) {
  const [addressPage, setAddressPage] = createSignal(0);
  const pageStart = () => addressPage() * dmxAddressPageSize + 1;
  const pageEnd = () => pageStart() + dmxAddressPageSize - 1;
  const visibleAddressCells = () => props.addressCells.slice(addressPage() * dmxAddressPageSize, (addressPage() + 1) * dmxAddressPageSize);
  const gridFragments = () => segmentGridFragments(props.activeMap.segments, addressPage());

  createEffect(() => {
    const selectedId = props.selectedFixtureId;
    const selectedSegment = props.activeMap.segments.find((segment) => segment.fixture.id === selectedId);
    if (selectedSegment) {
      setAddressPage(Math.floor((selectedSegment.start - 1) / dmxAddressPageSize));
    }
  });

  const useNextFreeAddress = () => {
    if (props.nextFreeAddress !== null) {
      setAddressPage(Math.floor((props.nextFreeAddress - 1) / dmxAddressPageSize));
    }
    props.onNextFreeAddress();
  };

  return (
    <>
      <div class="dmxPatchMap">
        <div class="panelHeader">
          <h3>DMX Patch Grid</h3>
          <div class="panelHeaderActions">
            <button onClick={useNextFreeAddress} disabled={props.nextFreeAddress === null}>
              Next Free
            </button>
            <select
              value={props.activeUniverse}
              onInput={(event) => {
                setAddressPage(0);
                props.onUniverse(Number(event.currentTarget.value));
              }}
            >
              <For each={props.universeOptions}>
                {(universeId) => <option value={universeId}>Universe {universeId}</option>}
              </For>
            </select>
            <select
              aria-label="DMX address page"
              value={addressPage()}
              onInput={(event) => setAddressPage(Number(event.currentTarget.value))}
            >
              <For each={Array.from({ length: dmxAddressPageCount }, (_, page) => page)}>
                {(page) => (
                  <option value={page}>
                    A{page * dmxAddressPageSize + 1}-{(page + 1) * dmxAddressPageSize}
                  </option>
                )}
              </For>
            </select>
            <div class="viewToggle" aria-label="DMX map view">
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
          </div>
        </div>
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
          <div class="dmxAddressGrid" role="grid" aria-label={`Universe ${props.activeUniverse} DMX addresses ${pageStart()} to ${pageEnd()}`}>
            <For each={visibleAddressCells()}>
              {(cell) => (
                <button
                  class={`dmxAddressCell ${cell.segment ? "occupied" : ""} ${cell.isStart ? "start" : ""} ${
                    cell.plannedIndex !== null ? "planned" : ""
                  } ${cell.plannedStart ? "plannedStart" : ""} ${cell.plannedConflict ? "plannedConflict" : ""} ${
                    cell.isSelected ? "selected" : ""
                  }`}
                  title={
                    cell.segment || cell.plannedIndex !== null
                      ? [
                          `U${props.activeUniverse} A${cell.channel}`,
                          cell.segment ? `${cell.segment.fixture.label} (${cell.segment.start}-${cell.segment.end})` : "",
                          cell.plannedIndex !== null ? `Pending fixture ${cell.plannedIndex + 1}` : "",
                        ]
                          .filter(Boolean)
                          .join(" / ")
                      : `U${props.activeUniverse} A${cell.channel}`
                  }
                  style={{
                    "grid-column": `${((cell.channel - pageStart()) % dmxGridColumnCount) + 1}`,
                    "grid-row": `${Math.floor((cell.channel - pageStart()) / dmxGridColumnCount) + 1}`,
                  }}
                  onClick={() => props.onAddressCell(cell)}
                  aria-label={
                    cell.segment
                      ? `Address ${cell.channel}, ${cell.segment.fixture.label}`
                      : `Address ${cell.channel}, empty`
                  }
                >
                  {cell.channel}
                </button>
              )}
            </For>
            <For each={gridFragments()}>
              {(fragment) => (
                <button
                  class={`dmxPatchFixtureBlock ${fragment.segment.fixture.id === props.selectedFixtureId ? "selected" : ""} ${
                    fragment.span <= 2 ? "tiny" : fragment.span <= 5 ? "narrow" : ""
                  }`}
                  style={{
                    "grid-column": `${fragment.column} / span ${fragment.span}`,
                    "grid-row": `${fragment.row}`,
                  }}
                  title={`${fragment.segment.fixture.label} / A${fragment.segment.start} / ${
                    fragment.segment.end - fragment.segment.start + 1
                  }ch`}
                  onClick={() => props.onSelectFixture(fragment.segment.fixture)}
                >
                  <strong data-no-localize>{fragment.continuation ? `> ${fragment.segment.fixture.label}` : fragment.segment.fixture.label}</strong>
                  <small>A{fragment.segment.start} / {fragment.segment.end - fragment.segment.start + 1}ch</small>
                </button>
              )}
            </For>
          </div>
        </Show>
        <div class="dmxGridSummary">
          <span>{props.activeMap.used}/512 used</span>
          <span>{props.activeMap.largestFree}ch max free</span>
          <span>{props.plannedAddressSummary}</span>
          <span>{props.fixtureCount} fixture(s)</span>
        </div>
      </div>
      <div class="dmxPatchMap compact">
        <div class="panelHeader">
          <h3>Universe Overview</h3>
          <span>{props.universeMaps.length} universe(s)</span>
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
                      onClick={() => props.onSelectFixture(segment.fixture)}
                    />
                  )}
                </For>
              </div>
              <div class="dmxUniverseLegend">
                <For each={map.segments.slice(0, 6)}>
                  {(segment) => (
                    <button onClick={() => props.onSelectFixture(segment.fixture)}>
                      <span data-no-localize>A{segment.start}-{segment.end} {segment.fixture.label}</span>
                    </button>
                  )}
                </For>
                <Show when={map.segments.length === 0}>
                  <span>Empty</span>
                </Show>
                <Show when={map.segments.length > 6}>
                  <span>+{map.segments.length - 6}</span>
                </Show>
              </div>
            </div>
          )}
        </For>
      </div>
    </>
  );
}
