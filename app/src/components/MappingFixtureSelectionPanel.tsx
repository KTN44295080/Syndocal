import { For, Show } from "solid-js";
import type { PatchedFixtureSummary } from "../types";

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
          <button onClick={() => void props.onDuplicateSelected()} disabled={!hasSelection()}>
            Duplicate
            <span>{props.selectedCount}</span>
          </button>
          <button onClick={() => void props.onRemoveSelected()} disabled={!hasSelection()}>
            Remove
            <span>{props.selectedCount}</span>
          </button>
          <button
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
            type="text"
            value={props.groupText}
            onInput={(event) => props.onGroupText(event.currentTarget.value)}
            placeholder="front, movers, floor"
          />
        </label>
        <div class="mappingSelectionQuickActions">
          <button
            onClick={() => void props.onApplyGroups("add")}
            disabled={!hasSelection() || !hasGroupTokens()}
          >
            Add
          </button>
          <button
            onClick={() => void props.onApplyGroups("remove")}
            disabled={!hasSelection() || !hasGroupTokens()}
          >
            Remove
          </button>
          <button
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
  return (
    <>
      <div class="mappingSelectionHeader secondary">
        <strong>Fixtures</strong>
        <span>{props.fixtures.length}</span>
      </div>
      <For each={props.fixtures}>
        {(fixture) => (
          <button
            class={props.selectedFixtureIds.has(fixture.id) ? "active" : ""}
            onClick={(event) => props.onSelectFixture(fixture, event)}
          >
            <strong>{fixture.label}</strong>
            <span>U{fixture.universe} A{fixture.address}</span>
            <small>{fixture.manufacturer} {fixture.profile_name} / {fixture.group_ids.join(", ") || "No group"}</small>
          </button>
        )}
      </For>
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
