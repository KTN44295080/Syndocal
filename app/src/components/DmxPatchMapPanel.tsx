import { For, Show } from "solid-js";
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
  onNextFreeAddress: () => void;
  onUniverse: (universe: number) => void;
  onViewMode: (mode: DmxPatchViewMode) => void;
  onSelectFixture: (fixture: PatchedFixtureSummary) => void;
  onAddressCell: (cell: DmxAddressCell) => void;
}

export function DmxPatchMapPanel(props: DmxPatchMapPanelProps) {
  const gridFragments = () => segmentGridFragments(props.activeMap.segments);

  return (
    <>
      <div class="dmxPatchMap">
        <div class="panelHeader">
          <h3>DMX Patch Grid</h3>
          <div class="panelHeaderActions">
            <button onClick={props.onNextFreeAddress} disabled={props.nextFreeAddress === null}>
              Next Free
            </button>
            <select
              value={props.activeUniverse}
              onInput={(event) => props.onUniverse(Number(event.currentTarget.value))}
            >
              <For each={props.universeOptions}>
                {(universeId) => <option value={universeId}>Universe {universeId}</option>}
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
                    <strong>{segment.fixture.label}</strong>
                    <span>
                      U{props.activeUniverse} A{segment.start}-{segment.end} / {segment.end - segment.start + 1}ch
                    </span>
                    <small>{segment.fixture.manufacturer} {segment.fixture.profile_name}</small>
                  </button>
                )}
              </For>
              <Show when={props.activeMap.segments.length === 0}>
                <p class="empty">No fixtures in this universe. Use Setup &gt; Patch to assign one.</p>
              </Show>
            </div>
          }
        >
          <div class="dmxAddressGrid" role="grid" aria-label={`Universe ${props.activeUniverse} DMX addresses`}>
            <For each={props.addressCells}>
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
                    "grid-column": `${((cell.channel - 1) % dmxGridColumnCount) + 1}`,
                    "grid-row": `${Math.floor((cell.channel - 1) / dmxGridColumnCount) + 1}`,
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
                  <strong>{fragment.continuation ? `> ${fragment.segment.fixture.label}` : fragment.segment.fixture.label}</strong>
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
                      A{segment.start}-{segment.end} {segment.fixture.label}
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
