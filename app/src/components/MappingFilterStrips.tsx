import { For } from "solid-js";
import { mappingTypeGlyphClass, type MappingFixtureVisualKind } from "../fixtureVisuals";

export interface MappingGroupStripRow {
  groupId: string;
  count: number;
}

export interface MappingFixtureTypeStripRow {
  key: string;
  label: string;
  manufacturer: string;
  visualKind: MappingFixtureVisualKind;
  count: number;
}

type MappingFilterStripsProps = {
  fixtureCount: number;
  filteredFixtureCount: number;
  selectedGroupId: string | null;
  groupRows: MappingGroupStripRow[];
  selectedTypeKey: string | null;
  fixtureTypeRows: MappingFixtureTypeStripRow[];
  onSelectGroup: (groupId: string | null) => void;
  onSelectType: (typeKey: string | null) => void;
};

export function MappingFilterStrips(props: MappingFilterStripsProps) {
  return (
    <>
      <div class="mappingGroupStrip">
        <span>Groups</span>
        <button
          class={!props.selectedGroupId ? "active" : ""}
          onClick={() => props.onSelectGroup(null)}
        >
          All
          <small>{props.fixtureCount}</small>
        </button>
        <For each={props.groupRows}>
          {(group) => (
            <button
              class={props.selectedGroupId === group.groupId ? "active" : ""}
              onClick={() => props.onSelectGroup(group.groupId)}
            >
              {group.groupId}
              <small>{group.count}</small>
            </button>
          )}
        </For>
      </div>
      <div class="mappingTypeStrip">
        <span>Types</span>
        <button
          class={!props.selectedTypeKey ? "active" : ""}
          onClick={() => props.onSelectType(null)}
        >
          <span class="mappingTypeGlyph kind-all" />
          All
          <small>{props.filteredFixtureCount}</small>
        </button>
        <For each={props.fixtureTypeRows}>
          {(row) => (
            <button
              class={props.selectedTypeKey === row.key ? "active" : ""}
              onClick={() => props.onSelectType(row.key)}
              title={`${row.manufacturer} ${row.label}`}
            >
              <span class={mappingTypeGlyphClass(row.visualKind)} />
              {row.label}
              <small>{row.count}</small>
            </button>
          )}
        </For>
      </div>
    </>
  );
}
