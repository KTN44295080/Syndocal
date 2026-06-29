import { For, Show, type JSX } from "solid-js";
import type { PatchedFixtureSummary } from "../types";

export interface SetupFixtureGroupRow {
  groupId: string;
  count: number;
}

interface SetupFixtureListPanelProps {
  fixtures: PatchedFixtureSummary[];
  totalFixtureCount: number;
  selectedGroupId: string | null;
  groupRows: SetupFixtureGroupRow[];
  selectedFixtureId: number | null;
  children?: JSX.Element;
  onSelectGroup: (groupId: string | null) => void;
  onSelectFixture: (fixture: PatchedFixtureSummary) => void;
}

export function SetupFixtureListPanel(props: SetupFixtureListPanelProps) {
  return (
    <>
      <div class="panelHeader">
        <h2>Fixtures</h2>
        <span>{props.fixtures.length} / {props.totalFixtureCount}</span>
      </div>
      <div class="groupOverview">
        <button
          class={!props.selectedGroupId ? "groupChip active" : "groupChip"}
          onClick={() => props.onSelectGroup(null)}
        >
          All
          <span>{props.totalFixtureCount}</span>
        </button>
        <For each={props.groupRows}>
          {(group) => (
            <button
              class={props.selectedGroupId === group.groupId ? "groupChip active" : "groupChip"}
              onClick={() => props.onSelectGroup(group.groupId)}
            >
              {group.groupId}
              <span>{group.count}</span>
            </button>
          )}
        </For>
      </div>
      {props.children}
      <div class="fixtureList">
        <For each={props.fixtures}>
          {(fixture) => (
            <button
              class={fixture.id === props.selectedFixtureId ? "fixture selected" : "fixture"}
              onClick={() => props.onSelectFixture(fixture)}
            >
              <strong>{fixture.label}</strong>
              <span>{fixture.manufacturer} {fixture.profile_name}</span>
              <small>U{fixture.universe} A{fixture.address} / {fixture.mode_name}</small>
              <Show when={fixture.highlighted || fixture.soloed || fixture.parked}>
                <small>
                  {[fixture.highlighted ? "Highlight" : "", fixture.soloed ? "Solo" : "", fixture.parked ? "Park" : ""]
                    .filter(Boolean)
                    .join(" / ")}
                </small>
              </Show>
              <Show when={fixture.group_ids.length > 0}>
                <small>{fixture.group_ids.join(", ")}</small>
              </Show>
            </button>
          )}
        </For>
        <Show when={props.totalFixtureCount === 0}>
          <p class="empty">No fixtures patched.</p>
        </Show>
        <Show when={props.totalFixtureCount > 0 && props.fixtures.length === 0}>
          <p class="empty">No fixtures in group.</p>
        </Show>
      </div>
    </>
  );
}
