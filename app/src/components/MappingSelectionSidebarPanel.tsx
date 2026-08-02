import { For, Show, type ComponentProps } from "solid-js";
import type {
  FixtureGroupSummary,
  PatchedFixtureSummary,
  StageObjectKind,
  StageObjectSummary,
  VideoOutputMapping,
  VideoOutputSummary,
} from "../types";
import { MappingFixtureTypeStrip } from "./MappingFilterStrips";
import { FixtureLimitsPanel } from "./FixtureLimitsPanel";
import { MappingFixtureInspectorPanel, type MappingFixtureGeometryRow } from "./MappingFixtureInspectorPanel";
import { MappingFixtureListPanel, MappingFixtureSelectionToolsPanel } from "./MappingFixtureSelectionPanel";
import { MappingProjectorSelectionPanel } from "./MappingProjectorSelectionPanel";
import {
  MappingSelectionActionsPanel,
  type MappingSelectionFlagState,
} from "./MappingSelectionActionsPanel";
import { MappingStageObjectPanel } from "./MappingStageObjectPanel";

type MaybePromise = void | Promise<unknown>;
type MappingBulkGroupMode = "add" | "remove" | "set";
type MappingFixtureFlag = "highlight" | "solo" | "park";
type MappingAxis = "x" | "z";
type MappingFixtureLayoutMode = "line" | "grid" | "circle";
type StageObjectLayoutMode = "line" | "grid";
type StageObjectPickMode = "replace" | "add";
type MappingSelectionEvent = Pick<MouseEvent, "ctrlKey" | "metaKey" | "shiftKey">;

export interface MappingSelectionPanelProps {
  selectedFixtureCount: number;
  selectedFixtures: PatchedFixtureSummary[];
  filteredFixtureCount: number;
  fixtureSearch: string;
  groupText: string;
  groupTokenCount: number;
  availableGroups: FixtureGroupSummary[];
  stageObjects: StageObjectSummary[];
  stageObjectFixtureCounts: Record<number, number>;
  selectedStageObject: StageObjectSummary | null;
  selectedStageObjectId: number | null;
  stageObjectDraftLabel: string;
  stageObjectDraftKind: StageObjectKind;
  stageObjectDraftWidth: number;
  stageObjectDraftDepth: number;
  stageObjectDraftRotation: number;
  stageObjectDraftColor: string;
  flagState: MappingSelectionFlagState;
  snapSize: number;
  selectedFixture: PatchedFixtureSummary | null;
  selectedGeometryRows: MappingFixtureGeometryRow[];
  unresolvedGeometryReferences: string[];
  fixtureLimitsEditor: ComponentProps<typeof FixtureLimitsPanel> | null;
  filteredFixtures: PatchedFixtureSummary[];
  selectedFixtureIds: Set<number>;
  outputs: VideoOutputSummary[];
  selectedOutput: VideoOutputSummary | null;
  selectedOutputId: number | null;
  onSearch: (value: string) => void;
  onGroupText: (value: string) => void;
  onPickVisible: () => MaybePromise;
  onDuplicateSelected: () => MaybePromise;
  onRemoveSelected: () => MaybePromise;
  onClearSelection: () => MaybePromise;
  onApplyGroups: (mode: MappingBulkGroupMode) => MaybePromise;
  onStageObjectDraftLabel: (value: string) => void;
  onStageObjectDraftKind: (value: StageObjectKind) => void;
  onStageObjectDraftWidth: (value: number) => void;
  onStageObjectDraftDepth: (value: number) => void;
  onStageObjectDraftRotation: (value: number) => void;
  onStageObjectDraftColor: (value: string) => void;
  onAddStageObjectCenter: () => MaybePromise;
  onSelectStageObject: (objectId: number) => void;
  onSetStageObject: (object: StageObjectSummary, updates: Partial<StageObjectSummary>) => MaybePromise;
  onRemoveStageObject: (objectId: number) => MaybePromise;
  onPickInsideStageObject: (mode: StageObjectPickMode) => void;
  onLayoutOnStageObject: (mode: StageObjectLayoutMode) => MaybePromise;
  onSetSelectionFlag: (flag: MappingFixtureFlag, enabled: boolean) => MaybePromise;
  onNudgeSelection: (dx: number, dz: number) => MaybePromise;
  onLayoutSelection: (mode: MappingFixtureLayoutMode) => MaybePromise;
  onAlignSelection: (axis: MappingAxis) => MaybePromise;
  onDistributeSelection: (axis: MappingAxis) => MaybePromise;
  onMirrorSelection: (axis: MappingAxis) => MaybePromise;
  onRotateSelection: (degrees: number) => MaybePromise;
  onLayoutFixtures: (mode: MappingFixtureLayoutMode) => MaybePromise;
  onControlActive: () => void;
  onOpenSceneFx: () => void;
  onSetFixtureTransform: (
    fixture: PatchedFixtureSummary,
    updates: Partial<Pick<PatchedFixtureSummary, "position" | "rotation">>,
  ) => MaybePromise;
  onControlFixture: () => void;
  onPatchFixture: () => void;
  onSelectFixture: (fixture: PatchedFixtureSummary, event: MappingSelectionEvent) => void;
  onSelectOutput: (outputId: number) => void;
  onSetOutputEnabled: (outputId: number, enabled: boolean) => MaybePromise;
  onSetOutputBlackout: (outputId: number, blackout: boolean) => MaybePromise;
  onOpenOutputWindow: (outputId: number, testPattern?: boolean) => MaybePromise;
  onSyncOutputWindow: (outputId: number) => MaybePromise;
  onFitOutputToStageObject: (output: VideoOutputSummary, object: StageObjectSummary) => MaybePromise;
  onSetOutputMapping: (outputId: number, mapping: VideoOutputMapping) => MaybePromise;
  onEditOutputProjection: (outputId: number) => void;
}

type MappingSelectionsColumnProps = MappingSelectionPanelProps & {
  typeFilters: ComponentProps<typeof MappingFixtureTypeStrip>;
};

export function MappingSelectionsColumn(props: MappingSelectionsColumnProps) {
  return (
    <aside
      class="mappingSelectionPanel mappingSelectionsColumn"
      data-persistent-band-part="selections"
      aria-label="Fixture selections"
    >
      <MappingFixtureTypeStrip {...props.typeFilters} />
      <MappingFixtureSelectionToolsPanel
        selectedCount={props.selectedFixtureCount}
        filteredCount={props.filteredFixtureCount}
        search={props.fixtureSearch}
        groupText={props.groupText}
        groupTokenCount={props.groupTokenCount}
        availableGroups={props.availableGroups}
        onSearch={props.onSearch}
        onGroupText={props.onGroupText}
        onPickVisible={props.onPickVisible}
        onDuplicateSelected={props.onDuplicateSelected}
        onRemoveSelected={props.onRemoveSelected}
        onClearSelection={props.onClearSelection}
        onApplyGroups={props.onApplyGroups}
      />
      <MappingFixtureListPanel
        fixtures={props.filteredFixtures}
        selectedFixtureIds={props.selectedFixtureIds}
        onSelectFixture={props.onSelectFixture}
      />
    </aside>
  );
}

export function MappingControlSelections(props: Pick<
  MappingSelectionPanelProps,
  "selectedFixtureCount" | "selectedFixtures"
>) {
  return (
    <section
      class="controlStageSelections"
      data-control-stage-chrome-operation="selections"
      data-control-stage-selection-list
      data-persistent-band-part="selections"
      aria-label="Fixture selections"
    >
      <div class="controlStageSelectionsHeader">
        <strong>Selections</strong>
        <span>{props.selectedFixtureCount}</span>
      </div>
      <div class="controlStageSelectionNames" role="list">
        <Show
          when={props.selectedFixtures.length > 0}
          fallback={<span class="empty">No fixture selected</span>}
        >
          <For each={props.selectedFixtures.slice(0, 6)}>
            {(fixture) => (
              <span role="listitem" title={fixture.label} data-no-localize>
                {fixture.label}
              </span>
            )}
          </For>
          <Show when={props.selectedFixtures.length > 6}>
            <span class="overflowCount" data-no-localize>
              +{props.selectedFixtures.length - 6}
            </span>
          </Show>
        </Show>
      </div>
    </section>
  );
}

export function MappingSetupContextPanel(props: MappingSelectionPanelProps) {
  return (
    <div class="mappingSelectionPanel mappingSetupContextContent">
      <MappingStageObjectPanel
        stageObjects={props.stageObjects}
        stageObjectFixtureCounts={props.stageObjectFixtureCounts}
        selectedObject={props.selectedStageObject}
        selectedObjectId={props.selectedStageObjectId}
        selectedFixtureCount={props.selectedFixtureCount}
        draftLabel={props.stageObjectDraftLabel}
        draftKind={props.stageObjectDraftKind}
        draftWidth={props.stageObjectDraftWidth}
        draftDepth={props.stageObjectDraftDepth}
        draftRotation={props.stageObjectDraftRotation}
        draftColor={props.stageObjectDraftColor}
        onDraftLabel={props.onStageObjectDraftLabel}
        onDraftKind={props.onStageObjectDraftKind}
        onDraftWidth={props.onStageObjectDraftWidth}
        onDraftDepth={props.onStageObjectDraftDepth}
        onDraftRotation={props.onStageObjectDraftRotation}
        onDraftColor={props.onStageObjectDraftColor}
        onAddCenter={props.onAddStageObjectCenter}
        onSelectObject={props.onSelectStageObject}
        onSetObject={props.onSetStageObject}
        onRemoveObject={props.onRemoveStageObject}
        onPickInside={props.onPickInsideStageObject}
        onLayoutOnObject={props.onLayoutOnStageObject}
      />
      <Show when={props.fixtureLimitsEditor}>
        {(fixtureLimitsEditor) => <FixtureLimitsPanel {...fixtureLimitsEditor()} />}
      </Show>
      <MappingSelectionActionsPanel
        flagState={props.flagState}
        selectedCount={props.selectedFixtureCount}
        snapSize={props.snapSize}
        hasSelectedStageObject={Boolean(props.selectedStageObject)}
        onSetFlag={props.onSetSelectionFlag}
        onNudge={props.onNudgeSelection}
        onLayoutSelection={props.onLayoutSelection}
        onLayoutOnStageObject={props.onLayoutOnStageObject}
        onAlign={props.onAlignSelection}
        onDistribute={props.onDistributeSelection}
        onMirror={props.onMirrorSelection}
        onRotate={props.onRotateSelection}
        onDuplicate={props.onDuplicateSelected}
        onRemove={props.onRemoveSelected}
        onControlActive={props.onControlActive}
        onOpenSceneFx={props.onOpenSceneFx}
        onClearSelection={props.onClearSelection}
      />
      <MappingFixtureInspectorPanel
        fixture={props.selectedFixture}
        geometryRows={props.selectedGeometryRows}
        unresolvedGeometryReferences={props.unresolvedGeometryReferences}
        filteredFixtureCount={props.filteredFixtureCount}
        onSetTransform={props.onSetFixtureTransform}
        onLayoutFixtures={props.onLayoutFixtures}
        onControl={props.onControlFixture}
        onPatch={props.onPatchFixture}
        onRemove={props.onRemoveSelected}
      />
      <MappingProjectorSelectionPanel
        outputs={props.outputs}
        selectedOutput={props.selectedOutput}
        selectedOutputId={props.selectedOutputId}
        selectedStageObject={props.selectedStageObject}
        onSelectOutput={props.onSelectOutput}
        onSetEnabled={props.onSetOutputEnabled}
        onSetBlackout={props.onSetOutputBlackout}
        onOpenWindow={props.onOpenOutputWindow}
        onSyncWindow={props.onSyncOutputWindow}
        onFitStageObject={props.onFitOutputToStageObject}
        onSetMapping={props.onSetOutputMapping}
        onEditProjection={props.onEditOutputProjection}
      />
    </div>
  );
}
