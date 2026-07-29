import { createEffect, createMemo, createSignal, For, onCleanup, onMount, Show } from "solid-js";
import type { PatchedFixtureSummary } from "../types";
import { virtualListRange } from "../virtualList";

type MaybePromise = void | Promise<unknown>;
type MappingBulkGroupMode = "add" | "remove" | "set";
type MappingSelectionEvent = Pick<MouseEvent, "ctrlKey" | "metaKey" | "shiftKey">;

type MappingFixtureSelectionToolsPanelProps = {
  selectedCount: number;
  filteredCount: number;
  search: string;
  groupText: string;
  groupTokenCount: number;
  onSearch: (value: string) => void;
  onGroupText: (value: string) => void;
  onPickVisible: () => MaybePromise;
  onDuplicateSelected: () => MaybePromise;
  onRemoveSelected: () => MaybePromise;
  onClearSelection: () => MaybePromise;
  onApplyGroups: (mode: MappingBulkGroupMode) => MaybePromise;
};

type MappingFixtureListPanelProps = {
  fixtures: PatchedFixtureSummary[];
  selectedFixtureIds: Set<number>;
  onSelectFixture: (fixture: PatchedFixtureSummary, event: MappingSelectionEvent) => void;
};

const mappingFixtureVirtualizationThreshold = 60;
const mappingFixtureVirtualRowHeight = 64;
const mappingFixtureVirtualOverscan = 4;

export function MappingFixtureSelectionToolsPanel(props: MappingFixtureSelectionToolsPanelProps) {
  const hasSelection = () => props.selectedCount > 0;
  const hasGroupTokens = () => props.groupTokenCount > 0;

  return (
    <>
      <div class="mappingSelectionHeader">
        <strong>Selections</strong>
        <span>
          {hasSelection()
            ? `${props.selectedCount} picked / ${props.filteredCount}`
            : `${props.filteredCount} fixture(s)`}
        </span>
      </div>
      <div class="mappingSearchPanel">
        <label>
          Search
          <input
            data-mapping-selection-search
            type="search"
            value={props.search}
            onInput={(event) => props.onSearch(event.currentTarget.value)}
            placeholder="label, U1 A24, group"
          />
        </label>
        <button onClick={() => props.onSearch("")} disabled={props.search.trim().length === 0}>
          Clear
        </button>
        <div class="mappingSearchActions">
          <button
            onClick={() => void props.onPickVisible()}
            disabled={props.filteredCount === 0}
            title="Pick all fixtures currently visible in the mapping filters (Ctrl+A)"
          >
            Pick Visible
            <span>{props.filteredCount}</span>
          </button>
          <button
            data-mapping-selection-action="duplicate"
            onClick={() => void props.onDuplicateSelected()}
            disabled={!hasSelection()}
          >
            Duplicate
            <span>{props.selectedCount}</span>
          </button>
          <button
            data-mapping-selection-action="remove"
            onClick={() => void props.onRemoveSelected()}
            disabled={!hasSelection()}
          >
            Remove
            <span>{props.selectedCount}</span>
          </button>
          <button
            data-mapping-selection-action="clear"
            onClick={() => void props.onClearSelection()}
            disabled={!hasSelection()}
            title="Clear the current 2D mapping fixture pick (Ctrl+Shift+A)"
          >
            Clear Pick
            <span>{props.selectedCount}</span>
          </button>
        </div>
      </div>
      <div class="mappingSelectionAssign">
        <label>
          Selected Groups
          <input
            data-mapping-group-editor
            type="text"
            value={props.groupText}
            onInput={(event) => props.onGroupText(event.currentTarget.value)}
            placeholder="front, movers, floor"
          />
        </label>
        <div class="mappingSelectionQuickActions">
          <button
            data-mapping-group-action="add"
            onClick={() => void props.onApplyGroups("add")}
            disabled={!hasSelection() || !hasGroupTokens()}
          >
            Add
          </button>
          <button
            data-mapping-group-action="remove"
            onClick={() => void props.onApplyGroups("remove")}
            disabled={!hasSelection() || !hasGroupTokens()}
          >
            Remove
          </button>
          <button
            data-mapping-group-action="set"
            onClick={() => void props.onApplyGroups("set")}
            disabled={!hasSelection() || !hasGroupTokens()}
          >
            Set
          </button>
        </div>
      </div>
    </>
  );
}

export function MappingFixtureListPanel(props: MappingFixtureListPanelProps) {
  let fixtureListElement: HTMLDivElement | undefined;
  const [scrollTop, setScrollTop] = createSignal(0);
  const [viewportHeight, setViewportHeight] = createSignal(320);
  const virtualized = createMemo(() => props.fixtures.length > mappingFixtureVirtualizationThreshold);
  const virtualRange = createMemo(() => virtualListRange({
    itemCount: props.fixtures.length,
    itemHeight: mappingFixtureVirtualRowHeight,
    scrollTop: scrollTop(),
    viewportHeight: viewportHeight(),
    overscan: mappingFixtureVirtualOverscan,
  }));
  const visibleFixtures = createMemo(() => {
    const range = virtualRange();
    return props.fixtures.slice(range.startIndex, range.endIndex);
  });

  onMount(() => {
    const measure = () => setViewportHeight(fixtureListElement?.clientHeight || 320);
    measure();
    if (typeof ResizeObserver === "undefined" || !fixtureListElement) return;
    const observer = new ResizeObserver(measure);
    observer.observe(fixtureListElement);
    onCleanup(() => observer.disconnect());
  });

  createEffect(() => {
    if (!virtualized() || props.selectedFixtureIds.size !== 1 || !fixtureListElement) return;
    const selectedId = props.selectedFixtureIds.values().next().value as number | undefined;
    const selectedIndex = props.fixtures.findIndex((fixture) => fixture.id === selectedId);
    if (selectedIndex < 0) return;
    const rowTop = selectedIndex * mappingFixtureVirtualRowHeight;
    const rowBottom = rowTop + mappingFixtureVirtualRowHeight;
    if (rowTop < fixtureListElement.scrollTop) fixtureListElement.scrollTop = rowTop;
    else if (rowBottom > fixtureListElement.scrollTop + fixtureListElement.clientHeight) {
      fixtureListElement.scrollTop = rowBottom - fixtureListElement.clientHeight;
    }
  });

  const fixtureButton = (fixture: PatchedFixtureSummary) => (
    <button
      class={`mappingFixtureRowButton ${props.selectedFixtureIds.has(fixture.id) ? "active" : ""}`}
      aria-current={props.selectedFixtureIds.has(fixture.id) ? "true" : undefined}
      onClick={(event) => props.onSelectFixture(fixture, event)}
    >
      <strong data-no-localize>{fixture.label}</strong>
      <span>U{fixture.universe} A{fixture.address}</span>
      <small>{fixture.manufacturer} {fixture.profile_name} / {fixture.group_ids.join(", ") || "No group"}</small>
    </button>
  );

  return (
    <>
      <div class="mappingSelectionHeader secondary">
        <strong>Fixtures</strong>
        <span>{props.fixtures.length}</span>
      </div>
      <div
        ref={fixtureListElement}
        class={`mappingFixtureList ${virtualized() ? "virtualizedMappingFixtureList" : ""}`}
        role="list"
        aria-label="Mapping fixtures"
        aria-rowcount={props.fixtures.length}
        onScroll={(event) => setScrollTop(event.currentTarget.scrollTop)}
      >
        <Show
          when={virtualized()}
          fallback={<For each={props.fixtures}>{(fixture) => <div role="listitem">{fixtureButton(fixture)}</div>}</For>}
        >
          <div class="virtualMappingFixtureSpacer" style={{ height: `${virtualRange().totalHeightPx}px` }}>
            <div class="virtualMappingFixtureWindow" style={{ transform: `translateY(${virtualRange().offsetPx}px)` }}>
              <For each={visibleFixtures()}>{(fixture) => <div class="virtualMappingFixtureRow" role="listitem">{fixtureButton(fixture)}</div>}</For>
            </div>
          </div>
        </Show>
      </div>
      <Show when={props.fixtures.length === 0}>
        <div class="mappingTransformInspector">
          <div class="mappingTransformTitle">
            <strong>No fixtures</strong>
            <span>Patch fixtures or adjust filters.</span>
          </div>
        </div>
      </Show>
    </>
  );
}
