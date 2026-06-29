import { Show, type JSX } from "solid-js";
import { AttributeCategoryRail, type AttributeCategoryRow } from "./AttributeCategoryRail";
import { TouchFixtureActionsPanel, type TouchFixtureGroupFlagState } from "./TouchFixtureActionsPanel";
import { TouchFixturePickerPanel, type TouchFixtureGroupRow } from "./TouchFixturePickerPanel";
import type { PatchedFixtureSummary } from "../types";
import type { ControlCategory } from "../uiModes";

interface TouchFixturePanelProps {
  selectedGroupId?: string | null;
  selectedFixture?: PatchedFixtureSummary | null;
  groups: TouchFixtureGroupRow[];
  fixtures: PatchedFixtureSummary[];
  totalFixtureCount: number;
  selectedFixtureId?: number | null;
  groupFlagState: TouchFixtureGroupFlagState;
  globalAnyFlagged: boolean;
  categories: AttributeCategoryRow[];
  activeCategory: ControlCategory;
  activeCategoryLabel: string;
  visibleControlCount: number;
  onSelectGroup: (groupId: string | null) => void;
  onSelectFixture: (fixture: PatchedFixtureSummary) => void;
  onSetFixtureHighlight: (fixtureId: number, enabled: boolean) => void | Promise<void>;
  onSetFixtureSolo: (fixtureId: number, enabled: boolean) => void | Promise<void>;
  onSetFixturePark: (fixtureId: number, enabled: boolean) => void | Promise<void>;
  onSetGroupHighlight: (groupId: string, enabled: boolean) => void | Promise<void>;
  onSetGroupSolo: (groupId: string, enabled: boolean) => void | Promise<void>;
  onSetGroupPark: (groupId: string, enabled: boolean) => void | Promise<void>;
  onClearFixtureFlags: () => void | Promise<void>;
  onCategory: (category: ControlCategory) => void;
  children: JSX.Element;
}

export function TouchFixturePanel(props: TouchFixturePanelProps) {
  return (
    <section class="panel touchPanel touchFixturePanel">
      <div class="panelHeader">
        <h2>Touch Fixtures</h2>
        <span>{props.selectedGroupId ? `Group ${props.selectedGroupId}` : props.selectedFixture?.label ?? "No selection"}</span>
      </div>
      <TouchFixturePickerPanel
        groups={props.groups}
        fixtures={props.fixtures}
        totalFixtureCount={props.totalFixtureCount}
        selectedGroupId={props.selectedGroupId}
        selectedFixtureId={props.selectedFixtureId}
        onSelectGroup={props.onSelectGroup}
        onSelectFixture={props.onSelectFixture}
      />
      <Show when={props.selectedFixture}>
        {(fixture) => (
          <div class="touchFixtureActions">
            <TouchFixtureActionsPanel
              fixture={fixture()}
              selectedGroupId={props.selectedGroupId}
              groupFlagState={props.groupFlagState}
              globalAnyFlagged={props.globalAnyFlagged}
              onSetFixtureHighlight={props.onSetFixtureHighlight}
              onSetFixtureSolo={props.onSetFixtureSolo}
              onSetFixturePark={props.onSetFixturePark}
              onSetGroupHighlight={props.onSetGroupHighlight}
              onSetGroupSolo={props.onSetGroupSolo}
              onSetGroupPark={props.onSetGroupPark}
              onClearFixtureFlags={props.onClearFixtureFlags}
            />
            <AttributeCategoryRail
              categories={props.categories}
              activeCategory={props.activeCategory}
              className="touchAttributeCategoryRail"
              ariaLabel="Touch attribute category"
              onCategory={props.onCategory}
            />
            {props.children}
            <Show when={props.visibleControlCount === 0}>
              <p class="empty">No {props.activeCategoryLabel.toLowerCase()} controls.</p>
            </Show>
          </div>
        )}
      </Show>
    </section>
  );
}
