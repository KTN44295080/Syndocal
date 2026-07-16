import { For, Show, type ComponentProps, type JSX } from "solid-js";
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
  onControlMode: (mode: ControlMode) => void;
  onCloseHotkeyHelp: () => void;
  children?: JSX.Element;
};

const controlContextModes = controlModes.map((mode) => ({
  ...mode,
  label: mode.id === "mixer" ? "Mixer" : mode.label,
}));

export function MappingPersistentWorkspaceBand(props: MappingPersistentWorkspaceBandProps) {
  const contextClass = () =>
    props.workspace === "setup"
      ? "workspaceContextPane setupContextPane"
      : `workspaceContextPane controlContextPane controlMode${props.controlMode[0].toUpperCase()}${props.controlMode.slice(1)}`;

  return (
    <section class="mappingPersistentWorkspaceBand" aria-label="Persistent workspace band">
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
                        onClick={() => props.onControlMode(mode.id)}
                        aria-pressed={props.controlMode === mode.id}
                      >
                        {mode.label}
                      </button>
                    )}
                  </For>
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
