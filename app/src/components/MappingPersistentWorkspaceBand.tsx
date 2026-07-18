import { createEffect, createSignal, For, onCleanup, Show, type ComponentProps, type JSX } from "solid-js";
import type { ControlMode } from "../uiModes";
import { controlModes } from "../uiModes";
import { MappingEditableStageShell } from "./MappingEditableStageShell";
import { MappingGroupRibbon, type MappingFilterStripsProps } from "./MappingFilterStrips";
import { MappingHotkeyHelp } from "./MappingHotkeyHelp";
import {
  MappingSelectionsColumn,
  MappingSetupContextPanel,
  type MappingSelectionPanelProps,
} from "./MappingSelectionSidebarPanel";
import { MappingStageLayersPanel } from "./MappingStageLayersPanel";
import { MappingToolRail } from "./MappingToolRail";
import { MappingViewportControls } from "./MappingViewportControls";

type WithoutChildren<T> = Omit<T, "children">;

type MappingPersistentWorkspaceBandProps = {
  workspace: "setup" | "control";
  controlMode: ControlMode;
  filters: MappingFilterStripsProps;
  toolRail: ComponentProps<typeof MappingToolRail>;
  viewportControls: ComponentProps<typeof MappingViewportControls>;
  editableStage: WithoutChildren<ComponentProps<typeof MappingEditableStageShell>>;
  stageLayers: ComponentProps<typeof MappingStageLayersPanel>;
  selection: MappingSelectionPanelProps;
  hotkeyHelpOpen: boolean;
  poppedPanes: string[];
  onTogglePaneWindow: (pane: "stage" | "timeline") => void;
  onControlMode: (mode: ControlMode) => void;
  onCloseHotkeyHelp: () => void;
  children?: JSX.Element;
};

const controlContextModes = controlModes.map((mode) => ({
  ...mode,
  label: mode.id === "mixer" ? "Mixer" : mode.label,
}));

export function MappingPersistentWorkspaceBand(props: MappingPersistentWorkspaceBandProps) {
  const [timelinePaneExpanded, setTimelinePaneExpanded] = createSignal(false);
  const timelinePaneActionLabel = () =>
    timelinePaneExpanded() ? "Restore Timeline pane (Esc)" : "Expand Timeline pane";
  const contextClass = () =>
    props.workspace === "setup"
      ? "workspaceContextPane setupContextPane"
      : `workspaceContextPane controlContextPane controlMode${props.controlMode[0].toUpperCase()}${props.controlMode.slice(1)}`;

  const handleWindowKeyDown = (event: KeyboardEvent) => {
    if (event.key === "Escape" && timelinePaneExpanded()) {
      setTimelinePaneExpanded(false);
    }
  };

  window.addEventListener("keydown", handleWindowKeyDown);
  onCleanup(() => window.removeEventListener("keydown", handleWindowKeyDown));

  createEffect(() => {
    if (props.workspace !== "control" || props.controlMode !== "live") {
      setTimelinePaneExpanded(false);
    }
  });

  const selectControlMode = (mode: ControlMode) => {
    if (mode !== "live") {
      setTimelinePaneExpanded(false);
    }
    props.onControlMode(mode);
  };

  return (
    <section
      class={`mappingPersistentWorkspaceBand${timelinePaneExpanded() ? " timelinePaneExpanded" : ""}${props.poppedPanes.includes("stage") ? " stagePanePopped" : ""}`}
      data-timeline-pane-expanded={timelinePaneExpanded() ? "true" : "false"}
      aria-label="Persistent workspace band"
    >
      <MappingGroupRibbon
        fixtureCount={props.filters.fixtureCount}
        selectedGroupId={props.filters.selectedGroupId}
        groupRows={props.filters.groupRows}
        onSelectGroup={props.filters.onSelectGroup}
      />
      <div class="mappingPersistentWorkspaceGrid">
        <section class="mappingPersistentStage" aria-label="Editable 2D stage map">
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
        </section>
        <MappingSelectionsColumn
          {...props.selection}
          typeFilters={{
            filteredFixtureCount: props.filters.filteredFixtureCount,
            selectedTypeKey: props.filters.selectedTypeKey,
            fixtureTypeRows: props.filters.fixtureTypeRows,
            onSelectType: props.filters.onSelectType,
          }}
        />
        <aside
          class={contextClass()}
          data-persistent-band-part="context"
          aria-label="Workspace context pane"
        >
          <Show
            when={props.workspace === "setup"}
            fallback={
              <>
                <nav class="controlModeTabs contextModeTabs" aria-label="Control mode">
                  <For each={controlContextModes}>
                    {(mode) => (
                      <button
                        class={props.controlMode === mode.id ? "active" : ""}
                        title={mode.description}
                        aria-keyshortcuts={mode.label[0]}
                        onClick={() => selectControlMode(mode.id)}
                        aria-pressed={props.controlMode === mode.id}
                      >
                        {mode.label}
                      </button>
                    )}
                  </For>
                  <Show when={props.controlMode === "live"}>
                    <button
                      type="button"
                      class={`timelinePaneExpandToggle${timelinePaneExpanded() ? " expanded" : ""}`}
                      data-timeline-pane-expand-toggle
                      title={timelinePaneActionLabel()}
                      aria-label={timelinePaneActionLabel()}
                      aria-expanded={timelinePaneExpanded()}
                      onClick={() => setTimelinePaneExpanded((expanded) => !expanded)}
                    >
                      <svg viewBox="0 0 16 16" aria-hidden="true">
                        <path d={timelinePaneExpanded() ? "M3 6h3V3M13 6h-3V3M13 10h-3v3M3 10h3v3" : "M6 3H3v3M10 3h3v3M13 10v3h-3M3 10v3h3"} />
                      </svg>
                    </button>
                  </Show>
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
                </nav>
                {props.children}
              </>
            }
          >
            <MappingSetupContextPanel {...props.selection} />
          </Show>
        </aside>
      </div>
    </section>
  );
}
