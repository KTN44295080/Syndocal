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

export type MappingFilterStripsProps = {
  fixtureCount: number;
  filteredFixtureCount: number;
  selectedGroupId: string | null;
  groupRows: MappingGroupStripRow[];
  selectedTypeKey: string | null;
  fixtureTypeRows: MappingFixtureTypeStripRow[];
  onSelectGroup: (groupId: string | null) => void;
  onSelectType: (typeKey: string | null) => void;
};

export type MappingGroupRibbonProps = Pick<
  MappingFilterStripsProps,
  "fixtureCount" | "selectedGroupId" | "groupRows" | "onSelectGroup"
>;

export type MappingFixtureTypeStripProps = Pick<
  MappingFilterStripsProps,
  "filteredFixtureCount" | "selectedTypeKey" | "fixtureTypeRows" | "onSelectType"
>;

export function MappingGroupRibbon(props: MappingGroupRibbonProps) {
  return (
    <div class="mappingGroupStrip" data-persistent-band-part="groups">
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
            <span data-no-localize>{group.groupId}</span>
            <small>{group.count}</small>
          </button>
        )}
      </For>
    </div>
  );
}

export function MappingFixtureTypeStrip(props: MappingFixtureTypeStripProps) {
  return (
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
            <span data-no-localize>{row.label}</span>
            <small>{row.count}</small>
          </button>
        )}
      </For>
    </div>
  );
}

export function MappingFilterStrips(props: MappingFilterStripsProps) {
  return (
    <>
      <MappingGroupRibbon
        fixtureCount={props.fixtureCount}
        selectedGroupId={props.selectedGroupId}
        groupRows={props.groupRows}
        onSelectGroup={props.onSelectGroup}
      />
      <MappingFixtureTypeStrip
        filteredFixtureCount={props.filteredFixtureCount}
        selectedTypeKey={props.selectedTypeKey}
        fixtureTypeRows={props.fixtureTypeRows}
        onSelectType={props.onSelectType}
      />
    </>
  );
}
