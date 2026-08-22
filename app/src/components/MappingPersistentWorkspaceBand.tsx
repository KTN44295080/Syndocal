import { createEffect, Show, type ComponentProps, type JSX } from "solid-js";
import type { ControlMode } from "../uiModes";
import { defaultWorkspaceLayout } from "../workspaceLayoutStorage";
import { MappingEditableStageShell } from "./MappingEditableStageShell";
import { MappingGroupRibbon, type MappingFilterStripsProps } from "./MappingFilterStrips";
import { MappingHotkeyHelp } from "./MappingHotkeyHelp";
import {
  MappingControlSelections,
  MappingSelectionsColumn,
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
  lowerSplitRatio: number;
  timelinePaneExpanded: boolean;
  selectionsDrawerOpen: boolean;
  onTogglePaneWindow: (pane: "stage" | "timeline") => void;
  onLowerSplitRatio: (ratio: number) => void;
  onSelectionsDrawerOpen: (open: boolean) => void;
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

  return (
    <section
      class={`mappingPersistentWorkspaceBand${props.timelinePaneExpanded ? " timelinePaneExpanded" : ""}${props.poppedPanes.includes("stage") ? " stagePanePopped" : ""}${props.poppedPanes.includes("timeline") ? " timelinePanePopped" : ""}`}
      data-timeline-pane-expanded={props.timelinePaneExpanded ? "true" : "false"}
      data-control-stage-chrome={props.workspace !== "setup" ? "true" : undefined}
      data-workspace-pane="lower"
      aria-label="Persistent workspace band"
      aria-hidden={props.timelinePaneExpanded ? "true" : undefined}
      inert={props.timelinePaneExpanded ? true : undefined}
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
        <Show when={props.poppedPanes.includes("timeline")}>
          <button
            type="button"
            class="paneRejoinToggle"
            data-pane-rejoin-toggle="timeline"
            title="Close Timeline window"
            aria-label="Close Timeline window"
            onClick={() => props.onTogglePaneWindow("timeline")}
          >
            <svg viewBox="0 0 16 16" aria-hidden="true">
              <path d="M2 4h8v8H2zM6 2h8v8M6 10h4" />
            </svg>
            <span>Timeline</span>
          </button>
        </Show>
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
          <Show when={props.workspace === "setup"}>
            <details
              class="mappingSelectionsDrawer"
              data-workspace-selection-drawer
              open={props.selectionsDrawerOpen}
              onToggle={(event) => props.onSelectionsDrawerOpen(event.currentTarget.open)}
            >
              <summary
                data-persistent-band-part="selections-drawer"
                data-workspace-selection-drawer-toggle
                aria-expanded={props.selectionsDrawerOpen}
              >
                <span>Selections</span>
                <strong>{props.selection.selectedFixtureCount} / {props.filters.filteredFixtureCount}</strong>
              </summary>
              <div class="mappingSelectionsDrawerBody">
                <MappingSelectionsColumn
                  {...props.selection}
                  typeFilters={{
                    filteredFixtureCount: props.filters.filteredFixtureCount,
                    selectedTypeKey: props.filters.selectedTypeKey,
                    fixtureTypeRows: props.filters.fixtureTypeRows,
                    onSelectType: props.filters.onSelectType,
                  }}
                />
              </div>
            </details>
          </Show>
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
          label={props.workspace === "touch" ? "Resize Mapping and Faders panes" : "Resize Stage and Timeline panes"}
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
                        onClick={() => props.onTogglePaneWindow("stage")}
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
                        onClick={() => props.onTogglePaneWindow("timeline")}
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
              <div class="mappingSetupContextMain">
                <MappingSetupContextPanel {...props.selection} />
              </div>
            </div>
          </Show>
        </aside>
      </div>
    </section>
  );
}
