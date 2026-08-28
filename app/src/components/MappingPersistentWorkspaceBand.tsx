import { createEffect, Show, type ComponentProps, type JSX } from "solid-js";
import type { ControlMode } from "../uiModes";
import { defaultWorkspaceLayout } from "../workspaceLayoutStorage";
import { MappingEditableStageShell } from "./MappingEditableStageShell";
import { MappingGroupRibbon, type MappingFilterStripsProps } from "./MappingFilterStrips";
import { MappingHotkeyHelp } from "./MappingHotkeyHelp";
import {
  MappingControlSelections,
  MappingSetupContextPanel,
  type MappingSelectionPanelProps,
} from "./MappingSelectionSidebarPanel";
import { MappingStageConfigPanel } from "./MappingStageConfigPanel";
import { MappingStageLayersPanel } from "./MappingStageLayersPanel";
import { MappingToolRail } from "./MappingToolRail";
import { ControlStageToolbar, MappingViewportControls } from "./MappingViewportControls";
import { WorkspaceSplitHandle } from "./WorkspaceSplitHandle";

type WithoutChildren<T> = Omit<T, "children">;

type MappingPersistentWorkspaceBandProps = {
  workspace: "setup" | "control" | "touch";
  controlMode: ControlMode;
  controlHeaderTitle: JSX.Element;
  controlHeaderTools?: JSX.Element;
  filters: MappingFilterStripsProps;
  toolRail: ComponentProps<typeof MappingToolRail>;
  viewportControls: ComponentProps<typeof MappingViewportControls>;
  editableStage: WithoutChildren<ComponentProps<typeof MappingEditableStageShell>>;
  stageLayers: ComponentProps<typeof MappingStageLayersPanel>;
  stageConfig: ComponentProps<typeof MappingStageConfigPanel>;
  selection: MappingSelectionPanelProps;
  hotkeyHelpOpen: boolean;
  poppedPanes: string[];
  paneOperationPending: (pane: "stage" | "timeline") => boolean;
  lowerSplitRatio: number;
  timelinePaneExpanded: boolean;
  onTogglePaneWindow: (pane: "stage" | "timeline") => void;
  onLowerSplitRatio: (ratio: number) => void;
  onCloseHotkeyHelp: () => void;
  onOpenMapping: () => void;
  lowerLeftContent?: JSX.Element;
  contextContent?: JSX.Element;
  keepChildrenMounted?: boolean;
  children?: JSX.Element;
};

export function MappingPersistentWorkspaceBand(props: MappingPersistentWorkspaceBandProps) {
  const contextClass = () =>
    props.workspace === "setup"
      ? "workspaceContextPane setupContextPane"
      : `workspaceContextPane controlContextPane controlMode${props.controlMode[0].toUpperCase()}${props.controlMode.slice(1)}`;

  createEffect(() => {
    if (
      props.workspace !== "setup" &&
      props.toolRail.stageTool !== "select" &&
      props.toolRail.stageTool !== "pan"
    ) {
      props.toolRail.onStageTool("select");
    }
  });

  const stagePanePopped = () => props.poppedPanes.includes("stage");
  const timelinePanePopped = () => props.poppedPanes.includes("timeline");
  // Expansion only owns the shell while the real Timeline still lives in the
  // upper arranger; a detached Timeline child must never suppress the band.
  const timelinePaneEffectivelyExpanded = () =>
    props.timelinePaneExpanded && !timelinePanePopped();

  return (
    <section
      class={`mappingPersistentWorkspaceBand${timelinePaneEffectivelyExpanded() ? " timelinePaneExpanded" : ""}${stagePanePopped() ? " stagePanePopped" : ""}${timelinePanePopped() ? " timelinePanePopped" : ""}${stagePanePopped() && timelinePanePopped() ? " stageAndTimelinePanesPopped" : ""}`}
      data-timeline-pane-expanded={timelinePaneEffectivelyExpanded() ? "true" : "false"}
      data-control-stage-chrome={props.workspace !== "setup" ? "true" : undefined}
      data-workspace-pane="lower"
      aria-label="Persistent workspace band"
      aria-hidden={timelinePaneEffectivelyExpanded() ? "true" : undefined}
      inert={timelinePaneEffectivelyExpanded() ? true : undefined}
    >
      <div
        class="mappingPersistentWorkspaceGrid"
        data-lower-split-ratio={props.lowerSplitRatio}
      >
        <MappingGroupRibbon
          fixtureCount={props.filters.fixtureCount}
          selectedGroupId={props.filters.selectedGroupId}
          groupRows={props.filters.groupRows}
          onSelectGroup={props.filters.onSelectGroup}
          onCreateGroup={props.filters.onCreateGroup}
          onRenameGroup={props.filters.onRenameGroup}
          onDeleteGroup={props.filters.onDeleteGroup}
          onRecolorGroup={props.filters.onRecolorGroup}
          controlChrome={props.workspace !== "setup"}
        />
        <section class="mappingWorkspaceLeftPane" data-workspace-pane="lower-left" aria-label="Groups and Stage pane">
          <Show when={props.lowerLeftContent} fallback={<>
          <section
            class={`mappingPersistentStage${props.workspace === "setup" ? " setupStageContext" : " controlStageContext"}`}
            aria-label={props.workspace === "setup" ? "Editable 2D stage map" : "2D fixture and projection surface mapping stage"}
          >
            <Show
              when={props.workspace === "setup"}
              fallback={
                <div class="mappingStageViewport controlStageViewport">
                  <ControlStageToolbar
                    stageTool={props.toolRail.stageTool}
                    canFitVisible={props.viewportControls.canFitVisible}
                    canFitSelection={props.viewportControls.canFitSelection}
                    zoomValue={props.viewportControls.zoomValue}
                    zoomMax={props.viewportControls.zoomMax}
                    canZoomOut={props.viewportControls.canZoomOut}
                    canZoomIn={props.viewportControls.canZoomIn}
                    showLabels={props.viewportControls.showLabels}
                    showBeams={props.viewportControls.showBeams}
                    showProjectors={props.viewportControls.showProjectors}
                    showStageObjects={props.viewportControls.showStageObjects}
                    selectedFixtureCount={props.selection.selectedFixtureCount}
                    canPickVisible={props.selection.filteredFixtureCount > 0}
                    onStageTool={props.toolRail.onStageTool}
                    onFitVisible={props.viewportControls.onFitVisible}
                    onFitSelection={props.viewportControls.onFitSelection}
                    onZoomOut={props.viewportControls.onZoomOut}
                    onZoomIn={props.viewportControls.onZoomIn}
                    onZoomLevel={props.viewportControls.onZoomLevel}
                    onShowLabels={props.viewportControls.onShowLabels}
                    onShowBeams={props.viewportControls.onShowBeams}
                    onShowProjectors={props.viewportControls.onShowProjectors}
                    onShowStageObjects={props.viewportControls.onShowStageObjects}
                    onPickVisible={props.selection.onPickVisible}
                    onClearSelection={props.selection.onClearSelection}
                    onOpenMapping={props.onOpenMapping}
                  />
                  <MappingControlSelections
                    selectedFixtureCount={props.selection.selectedFixtureCount}
                    selectedFixtures={props.selection.selectedFixtures}
                  />
                  <MappingEditableStageShell {...props.editableStage}>
                    <MappingStageLayersPanel
                      {...props.stageLayers}
                      readOnly
                      fixtureTransformsEditable
                    />
                  </MappingEditableStageShell>
                </div>
              }
            >
              <MappingToolRail {...props.toolRail} />
              <div class="mappingStageViewport">
                <div class="mappingViewportToolbar">
                  <MappingViewportControls {...props.viewportControls} />
                </div>
                <MappingEditableStageShell {...props.editableStage}>
                  <MappingStageLayersPanel {...props.stageLayers} />
                </MappingEditableStageShell>
              </div>
              <Show when={props.hotkeyHelpOpen}>
                <MappingHotkeyHelp onClose={props.onCloseHotkeyHelp} />
              </Show>
            </Show>
          </section>
          </>}>
            {props.lowerLeftContent}
          </Show>
        </section>
        <WorkspaceSplitHandle
          axis="vertical"
          ratio={props.lowerSplitRatio}
          defaultRatio={defaultWorkspaceLayout.lower_split_ratio}
          minFirstPx={430}
          minSecondPx={480}
          label={props.workspace === "touch" ? "Resize Mapping and Faders panes" : "Resize Stage and Source panes"}
          splitter="lower-left-right"
          onCommit={props.onLowerSplitRatio}
        />
        <aside
          class={contextClass()}
          data-persistent-band-part="context"
          data-workspace-pane="lower-right"
          aria-label="Workspace context pane"
        >
          <Show
            when={props.workspace === "setup"}
            fallback={
              <>
                <Show when={props.workspace !== "control" || props.controlMode !== "live"}>
                <header class="panelHeader controlContextHeader" data-control-context-header>
                  <h2>{props.controlHeaderTitle}</h2>
                  <div class="controlContextHeaderTools">
                    {props.controlHeaderTools}
                  </div>
                  <Show when={props.workspace === "control"}>
                    <div class="controlContextHeaderTools controlContextHeaderPaneActions" aria-label="Workspace pane controls">
                      <button
                        type="button"
                        class={`timelinePaneExpandToggle panePopoutToggle${props.poppedPanes.includes("stage") ? " expanded" : ""}`}
                        data-pane-popout-toggle="stage"
                        title={props.poppedPanes.includes("stage") ? "Close Stage window" : "Open Stage in a window"}
                        aria-label={props.poppedPanes.includes("stage") ? "Close Stage window" : "Open Stage in a window"}
                        aria-pressed={props.poppedPanes.includes("stage")}
                        aria-busy={props.paneOperationPending("stage") ? "true" : undefined}
                        disabled={props.paneOperationPending("stage")}
                        onClick={() => {
                          if (!props.paneOperationPending("stage")) props.onTogglePaneWindow("stage");
                        }}
                      >
                        <svg viewBox="0 0 16 16" aria-hidden="true">
                          <path d="M3 5h7v8H3zM6 5V3h7v8h-2" />
                        </svg>
                      </button>
                      <button
                        type="button"
                        class={`timelinePaneExpandToggle panePopoutToggle${props.poppedPanes.includes("timeline") ? " expanded" : ""}`}
                        data-pane-popout-toggle="timeline"
                        title={props.poppedPanes.includes("timeline") ? "Close Timeline window" : "Open Timeline in a window"}
                        aria-label={props.poppedPanes.includes("timeline") ? "Close Timeline window" : "Open Timeline in a window"}
                        aria-pressed={props.poppedPanes.includes("timeline")}
                        aria-busy={props.paneOperationPending("timeline") ? "true" : undefined}
                        disabled={props.paneOperationPending("timeline")}
                        onClick={() => {
                          if (!props.paneOperationPending("timeline")) props.onTogglePaneWindow("timeline");
                        }}
                      >
                        <svg viewBox="0 0 16 16" aria-hidden="true">
                          <path d="M2 6h12M2 6v6h12V6M6 3h7v3" />
                      </svg>
                        </button>
                    </div>
                  </Show>
                </header>
                </Show>
                {props.contextContent ?? props.children}
                <Show when={props.contextContent && props.keepChildrenMounted}>
                  <div hidden data-persistent-band-background-content>
                    {props.children}
                  </div>
                </Show>
              </>
            }
          >
            <div class="mappingSetupContextStack">
              <details class="stageSettingsDisclosure" data-stage-settings-disclosure>
                <summary data-stage-settings-disclosure-toggle>Stage Settings</summary>
                <div class="stageSettingsDisclosureBody" data-stage-settings-panel>
                  <MappingStageConfigPanel {...props.stageConfig} />
                </div>
              </details>
              <div
                class="mappingSetupContextMain"
                data-persistent-band-part="selections"
              >
                <MappingSetupContextPanel
                  {...props.selection}
                  typeFilters={{
                    filteredFixtureCount: props.filters.filteredFixtureCount,
                    selectedTypeKey: props.filters.selectedTypeKey,
                    fixtureTypeRows: props.filters.fixtureTypeRows,
                    onSelectType: props.filters.onSelectType,
                  }}
                />
              </div>
            </div>
          </Show>
        </aside>
      </div>
    </section>
  );
}
