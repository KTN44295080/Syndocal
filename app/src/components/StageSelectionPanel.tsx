import { For, Show } from "solid-js";
import type { VideoOutputSummary } from "../types";
import type { ControlCategory, ControlMode } from "../uiModes";
import { ProjectorMapPreview } from "./ProjectorMapEditor";
import { mappingReadout } from "../videoOutputMapping";

type GroupBulkMode = "add" | "remove" | "set";

interface MappingFixtureTypeRow {
  key: string;
  label: string;
  manufacturer: string;
  mode: string;
  visualKind: string;
  count: number;
}

interface MappingTransformSummary {
  count: number;
  label: string;
  mode: string;
  x: number;
  y: number;
  z: number;
  yaw: number;
}

interface MappingTransformFlagState {
  count: number;
  highlightedCount: number;
  soloedCount: number;
  parkedCount: number;
  anyHighlighted: boolean;
  anySoloed: boolean;
  anyParked: boolean;
  allHighlighted: boolean;
  allSoloed: boolean;
  allParked: boolean;
}

interface GlobalFlagState {
  anyHighlighted: boolean;
  anySoloed: boolean;
  anyParked: boolean;
  highlightedCount: number;
  soloedCount: number;
  parkedCount: number;
}

interface ControlCoverageRow {
  attribute: string;
  targetCount: number;
}

interface SelectedGroupControlCoverage {
  complete: boolean;
  rows: ControlCoverageRow[];
  fixtureCount: number;
  more: number | null;
}

interface MappingVideoOutputRow {
  output: VideoOutputSummary;
  stageLabel: string;
  stateLabel: string;
  compositionLabel: string;
}

type FixtureFlag = "highlight" | "solo" | "park";

interface StageSelectionPanelProps {
  selectedMappingFixtures: () => { length: number };
  mappingFilteredFixtures: () => { length: number };
  mappingFixtureSearch: () => string;
  setMappingFixtureSearch: (value: string) => void;
  clearMappingSearchKeepPicked: () => void;
  pickVisibleMappingFixtures: () => void;
  selectedMappingFixtureIds: () => number[];
  globalFlagState: () => GlobalFlagState;
  mappingTransformSummary: () => MappingTransformSummary | null;
  selectedGroupVisibleControlCoverage: () => SelectedGroupControlCoverage | null;
  setMappingTransformValue: (field: "x" | "y" | "z" | "yaw", value: number) => Promise<void>;
  openPatchForCurrentTarget: () => void;
  openControlForCurrentTarget: (category?: ControlCategory, mode?: ControlMode) => void;
  selectedFixtureGroupFilter: () => string | null;
  snapMappingTransformTargets: () => Promise<void>;
  stageSnapEnabled: () => boolean;
  stageYawSnapEnabled: () => boolean;
  mappingTransformFlagState: () => MappingTransformFlagState;
  setMappingTargetFlag: (flag: FixtureFlag, value: boolean) => Promise<void>;
  clearFixtureFlags: (flag: FixtureFlag) => Promise<void>;
  mappingControlQuickCategories: () => ControlCategory[];
  controlCategory: () => ControlCategory;
  controlCategoryLabel: (category: ControlCategory) => string;
  invertVisibleMappingFixtureSelection: () => void;
  clearMappingFixtureSelection: () => void;
  bulkGroupText: () => string;
  setBulkGroupText: (value: string) => void;
  bulkGroupMode: () => GroupBulkMode;
  setBulkGroupMode: (value: GroupBulkMode) => void;
  applyBulkGroupsToMappingSelection: () => Promise<void>;
  mappingOperationFixtureCount: () => number;
  filteredFixtures: () => { length: number };
  selectedFixtureTypeFilter: () => string | null;
  selectFixtureTypeFilter: (key: string | null) => void;
  fixtureTypeRows: () => MappingFixtureTypeRow[];
  fixtureTypePickedCounts: () => Map<string, number>;
  mappingVideoOutputRows: () => MappingVideoOutputRow[];
  selectedVideoOutputId: () => number | null;
  setSelectedVideoOutputId: (id: number) => void;
  selectedMappingVideoOutput: () => VideoOutputSummary | null;
}

export const StageSelectionPanel = (props: StageSelectionPanelProps) => (
  <aside class="mappingSelectionPanel">
    <div class="mappingSelectionHeader">
      <strong>Selections</strong>
      <span>
        {props.selectedMappingFixtures().length > 0
          ? `${props.selectedMappingFixtures().length} picked / ${props.mappingFilteredFixtures().length}`
          : `${props.mappingFilteredFixtures().length} fixture(s)`}
      </span>
    </div>
    <div class="mappingSearchPanel">
      <label>
        Search
        <input
          type="search"
          value={props.mappingFixtureSearch()}
          onInput={(event) => props.setMappingFixtureSearch(event.currentTarget.value)}
          placeholder="label, U1 A24, group"
        />
      </label>
      <button
        onClick={props.clearMappingSearchKeepPicked}
        disabled={props.mappingFixtureSearch().trim().length === 0}
      >
        Clear
      </button>
      <div class="mappingSearchActions" aria-label="Search result selection">
        <button onClick={props.pickVisibleMappingFixtures} disabled={props.mappingFilteredFixtures().length === 0}>
          Pick Results
          <span>{props.mappingFilteredFixtures().length}</span>
        </button>
        <button
          onClick={props.clearMappingSearchKeepPicked}
          disabled={props.mappingFixtureSearch().trim().length === 0}
        >
          Keep Picked
          <span>{props.selectedMappingFixtureIds().length}</span>
        </button>
      </div>
      <div class="mappingSearchQuickFilters" aria-label="Fixture flag quick filters">
        <button
          class={props.mappingFixtureSearch().trim().toLowerCase() === "highlight" ? "active" : ""}
          onClick={() => props.setMappingFixtureSearch("highlight")}
          disabled={!props.globalFlagState().anyHighlighted}
        >
          Hi {props.globalFlagState().highlightedCount}
        </button>
        <button
          class={props.mappingFixtureSearch().trim().toLowerCase() === "solo" ? "active" : ""}
          onClick={() => props.setMappingFixtureSearch("solo")}
          disabled={!props.globalFlagState().anySoloed}
        >
          Solo {props.globalFlagState().soloedCount}
        </button>
        <button
          class={props.mappingFixtureSearch().trim().toLowerCase() === "park" ? "active" : ""}
          onClick={() => props.setMappingFixtureSearch("park")}
          disabled={!props.globalFlagState().anyParked}
        >
          Park {props.globalFlagState().parkedCount}
        </button>
      </div>
    </div>
    <Show when={props.mappingTransformSummary()}>
      {(transform) => (
        <div class="mappingTransformInspector">
          <div class="mappingTransformTitle">
            <strong>{transform().label}</strong>
            <span>{transform().mode}</span>
          </div>
          <Show when={props.selectedGroupVisibleControlCoverage()}>
            {(coverage) => (
              <div class={coverage().complete ? "mappingCoverage ok" : "mappingCoverage warn"}>
                <strong>{coverage().complete ? "Control coverage" : "Mixed controls"}</strong>
                <span>
                  <For each={coverage().rows}>
                    {(row, index) => (
                      <>
                        {index() > 0 ? " / " : ""}
                        {row.attribute} {row.targetCount}/{coverage().fixtureCount}
                      </>
                    )}
                  </For>
                  {coverage().more ? ` / +${coverage().more}` : ""}
                </span>
              </div>
            )}
          </Show>
          <div class="mappingTransformGrid">
            <label>
              {transform().count > 1 ? "Center X" : "X"}
              <input
                type="number"
                step="0.1"
                value={transform().x}
                onChange={(event) => void props.setMappingTransformValue("x", Number(event.currentTarget.value))}
              />
            </label>
            <label>
              {transform().count > 1 ? "Center Z" : "Z"}
              <input
                type="number"
                step="0.1"
                value={transform().z}
                onChange={(event) => void props.setMappingTransformValue("z", Number(event.currentTarget.value))}
              />
            </label>
            <label>
              {transform().count > 1 ? "Center Y" : "Y"}
              <input
                type="number"
                step="0.1"
                value={transform().y}
                onChange={(event) => void props.setMappingTransformValue("y", Number(event.currentTarget.value))}
              />
            </label>
            <label>
              Yaw
              <input
                type="number"
                min="-180"
                max="180"
                step="1"
                value={transform().yaw}
                onChange={(event) => void props.setMappingTransformValue("yaw", Number(event.currentTarget.value))}
              />
            </label>
          </div>
          <div class="mappingTransformActions">
            <button
              onClick={() => props.openPatchForCurrentTarget()}
              disabled={transform().count !== 1}
              title={transform().count !== 1 ? "Pick one fixture before opening Patch" : "Open Patch setup for this fixture"}
            >
              Patch
            </button>
            <button
              onClick={() => props.openControlForCurrentTarget()}
              disabled={transform().count > 1 && !props.selectedFixtureGroupFilter()}
              title={
                transform().count > 1 && !props.selectedFixtureGroupFilter()
                  ? "Pick a fixture or select a group before opening Control"
                  : "Open Control Edit for this target"
              }
            >
              Control
            </button>
            <button
              onClick={() => props.openControlForCurrentTarget(undefined, "live")}
              disabled={transform().count > 1 && !props.selectedFixtureGroupFilter()}
              title={
                transform().count > 1 && !props.selectedFixtureGroupFilter()
                  ? "Pick a fixture or select a group before opening Live Control"
                  : "Open Live Control for this target"
              }
            >
              Live
            </button>
            <button
              onClick={() => props.openControlForCurrentTarget("position")}
              disabled={transform().count > 1 && !props.selectedFixtureGroupFilter()}
            >
              Position
            </button>
            <button
              onClick={() => void props.snapMappingTransformTargets()}
              disabled={transform().count === 0 || (!props.stageSnapEnabled() && !props.stageYawSnapEnabled())}
              title={
                !props.stageSnapEnabled() && !props.stageYawSnapEnabled()
                  ? "Enable Grid Snap or Yaw Snap before applying snap"
                  : "Apply current snap settings to this map target"
              }
            >
              Snap
            </button>
          </div>
          <div class="mappingFlagActions" aria-label="2D map fixture flags">
            <button
              class={props.mappingTransformFlagState().anyHighlighted ? "active" : ""}
              onClick={() => void props.setMappingTargetFlag("highlight", !props.mappingTransformFlagState().allHighlighted)}
              disabled={props.mappingTransformFlagState().count === 0}
            >
              {props.mappingTransformFlagState().allHighlighted ? "Clear Hi" : "Highlight"}
            </button>
            <button
              class={props.mappingTransformFlagState().anySoloed ? "active" : ""}
              onClick={() => void props.setMappingTargetFlag("solo", !props.mappingTransformFlagState().allSoloed)}
              disabled={props.mappingTransformFlagState().count === 0}
            >
              {props.mappingTransformFlagState().allSoloed ? "Clear Solo" : "Solo"}
            </button>
            <button
              class={props.mappingTransformFlagState().anyParked ? "active" : ""}
              onClick={() => void props.setMappingTargetFlag("park", !props.mappingTransformFlagState().allParked)}
              disabled={props.mappingTransformFlagState().count === 0}
            >
              {props.mappingTransformFlagState().allParked ? "Clear Park" : "Park"}
            </button>
          </div>
          <div class="mappingFlagStatus">
            <span>
              Target H {props.mappingTransformFlagState().highlightedCount}/{props.mappingTransformFlagState().count}
              {" / "}S {props.mappingTransformFlagState().soloedCount}/{props.mappingTransformFlagState().count}
              {" / "}P {props.mappingTransformFlagState().parkedCount}/{props.mappingTransformFlagState().count}
            </span>
            <div class="mappingFlagClearActions" aria-label="Clear fixture flags globally">
              <button disabled={!props.globalFlagState().anyHighlighted} onClick={() => void props.clearFixtureFlags("highlight")}>
                Clear H {props.globalFlagState().highlightedCount}
              </button>
              <button disabled={!props.globalFlagState().anySoloed} onClick={() => void props.clearFixtureFlags("solo")}>
                Clear S {props.globalFlagState().soloedCount}
              </button>
              <button disabled={!props.globalFlagState().anyParked} onClick={() => void props.clearFixtureFlags("park")}>
                Clear P {props.globalFlagState().parkedCount}
              </button>
            </div>
          </div>
          <Show when={props.mappingControlQuickCategories().length > 0}>
            <div class="mappingControlShortcuts" aria-label="Open Control category">
              <For each={props.mappingControlQuickCategories()}>
                {(category) => (
                  <button
                    class={props.controlCategory() === category ? "active" : ""}
                    onClick={() => props.openControlForCurrentTarget(category)}
                    disabled={transform().count > 1 && !props.selectedFixtureGroupFilter()}
                  >
                    {props.controlCategoryLabel(category)}
                  </button>
                )}
              </For>
            </div>
          </Show>
        </div>
      )}
    </Show>
    <div class="mappingSelectionAssign">
      <div class="mappingSelectionQuickActions">
        <button onClick={props.pickVisibleMappingFixtures} disabled={props.mappingFilteredFixtures().length === 0}>
          Pick Visible
        </button>
        <button onClick={props.invertVisibleMappingFixtureSelection} disabled={props.mappingFilteredFixtures().length === 0}>
          Invert
        </button>
        <button onClick={props.clearMappingFixtureSelection} disabled={props.selectedMappingFixtures().length === 0}>
          Clear Picked
        </button>
      </div>
      <label>
        Group
        <input
          value={props.bulkGroupText()}
          onInput={(event) => props.setBulkGroupText(event.currentTarget.value)}
          placeholder="front, bars"
        />
      </label>
      <div>
        <select
          value={props.bulkGroupMode()}
          onInput={(event) => props.setBulkGroupMode(event.currentTarget.value as GroupBulkMode)}
        >
          <option value="add">Add</option>
          <option value="remove">Remove</option>
          <option value="set">Set</option>
        </select>
        <button
          class="primary"
          onClick={() => void props.applyBulkGroupsToMappingSelection()}
          disabled={props.mappingOperationFixtureCount() === 0}
        >
          Apply
        </button>
      </div>
    </div>
    <button
      type="button"
      class={!props.selectedFixtureTypeFilter() ? "mappingTypeButton mappingTypeAll active" : "mappingTypeButton mappingTypeAll"}
      aria-label="All fixture types"
      title="Show all fixture types"
      onClick={() => props.selectFixtureTypeFilter(null)}
    >
      <span class="mappingTypeCopy">
        <strong>All Types</strong>
        <small>{props.selectedFixtureGroupFilter() ?? "All groups"}</small>
      </span>
      <span class="mappingTypeCount">
        <b>{props.filteredFixtures().length}</b>
        <Show when={props.selectedMappingFixtures().length > 0}>
          <small>{props.selectedMappingFixtures().length} picked</small>
        </Show>
      </span>
    </button>
    <For each={props.fixtureTypeRows()}>
      {(row) => (
        <button
          type="button"
          class={props.selectedFixtureTypeFilter() === row.key ? "mappingTypeButton active" : "mappingTypeButton"}
          aria-label={`Filter by fixture type: ${row.label}`}
          title={`${row.manufacturer} / ${row.label} / ${row.mode}`}
          onClick={() => props.selectFixtureTypeFilter(row.key)}
        >
          <span class={`mappingTypeGlyph kind-${row.visualKind}`} aria-hidden="true" />
          <span class="mappingTypeCopy">
            <strong data-no-localize>{row.label}</strong>
            <small>{row.manufacturer}</small>
            <small>{row.mode}</small>
          </span>
          <span class="mappingTypeCount">
            <b>{row.count}</b>
            <Show when={props.fixtureTypePickedCounts().get(row.key)}>
              {(count) => <small>{count()} picked</small>}
            </Show>
          </span>
        </button>
      )}
    </For>
    <Show when={props.fixtureTypeRows().length === 0}>
      <p class="empty">No fixture types.</p>
    </Show>
    <div class="mappingSelectionHeader secondary">
      <strong>Projection Surfaces</strong>
      <span>{props.mappingVideoOutputRows().length} surface(s)</span>
    </div>
    <For each={props.mappingVideoOutputRows()}>
      {(row) => (
        <button
          class={props.selectedVideoOutputId() === row.output.id ? "active projector" : "projector"}
          title={`${row.output.label} / ${row.stageLabel} / ${row.compositionLabel}`}
          onClick={() => props.setSelectedVideoOutputId(row.output.id)}
        >
          <strong data-no-localize>{row.output.label}</strong>
          <span>{row.stateLabel}</span>
          <small>{row.compositionLabel}</small>
          <small>Stage {row.stageLabel}</small>
        </button>
      )}
    </For>
    <Show when={props.mappingVideoOutputRows().length === 0}>
      <p class="empty">No projection surfaces. Add a Video Output in Setup &gt; Output.</p>
    </Show>
    <Show when={props.selectedMappingVideoOutput()}>
      {(output) => (
        <div class="mappingProjectorControls">
          <div class="mappingProjectorPreview">
            <ProjectorMapPreview
              mapping={output().mapping}
              outputId={output().id}
              class="projectorMapSurface outputMappingMiniSurface"
            />
            <div>
              <strong>{output().label}</strong>
              <span>{mappingReadout(output().mapping)}</span>
              <small>
                {output().enabled ? "Authored enabled" : "Authored disabled"} / {output().blackout ? "Blackout" : `${Math.round(output().opacity * 100)}%`}
              </small>
            </div>
          </div>
          <p class="inlineUnavailable" role="status">
            Projection mapping/configuration mutation is unavailable until a canonical path exists. This view is read-only.
          </p>
        </div>
      )}
    </Show>
  </aside>
);
