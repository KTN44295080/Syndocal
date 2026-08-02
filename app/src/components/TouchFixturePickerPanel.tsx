import { For, Show } from "solid-js";
import type { PatchedFixtureSummary } from "../types";
import { groupIdentityCss } from "../identityColor";

export interface TouchFixtureGroupRow {
  groupId: string;
  label: string;
  count: number;
}

interface TouchFixturePickerPanelProps {
  groups: TouchFixtureGroupRow[];
  fixtures: PatchedFixtureSummary[];
  groupColors?: Record<string, string>;
  totalFixtureCount: number;
  selectedGroupId?: string | null;
  selectedFixtureId?: number | null;
  onSelectGroup: (groupId: string | null) => void;
  onSelectFixture: (fixture: PatchedFixtureSummary) => void;
}

export function TouchFixturePickerPanel(props: TouchFixturePickerPanelProps) {
  return (
    <>
      <div class="touchGroupScroller">
        <button
          class={!props.selectedGroupId ? "groupChip active" : "groupChip"}
          onClick={() => props.onSelectGroup(null)}
        >
          All
          <span>{props.totalFixtureCount}</span>
        </button>
        <For each={props.groups}>
          {(group) => (
            <button
              class={props.selectedGroupId === group.groupId ? "groupChip active" : "groupChip"}
              style={{ "--identity": groupIdentityCss(group.groupId, props.groupColors, "fill") }}
              onClick={() => props.onSelectGroup(group.groupId)}
            >
              <span data-no-localize>{group.label}</span>
              <span>{group.count}</span>
            </button>
          )}
        </For>
      </div>
      <div class="touchFixtureScroller">
        <For each={props.fixtures}>
          {(fixture) => (
            <button
              class={fixture.id === props.selectedFixtureId ? "fixture selected" : "fixture"}
              onClick={() => props.onSelectFixture(fixture)}
            >
              <strong data-no-localize>{fixture.label}</strong>
              <span>
                U{fixture.universe} A{fixture.address}
              </span>
              <small data-no-localize>{fixture.group_ids.join(", ") || fixture.mode_name}</small>
            </button>
          )}
        </For>
        <Show when={props.fixtures.length === 0}>
          <p class="empty">No fixtures. Patch one in Setup &gt; Patch.</p>
        </Show>
      </div>
    </>
  );
}
