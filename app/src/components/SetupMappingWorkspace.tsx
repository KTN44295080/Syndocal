import { Show, type ComponentProps } from "solid-js";
import { DmxPatchMapPanel } from "./DmxPatchMapPanel";
import { MappingEditableStageShell } from "./MappingEditableStageShell";
import { MappingFilterStrips } from "./MappingFilterStrips";
import { MappingHotkeyHelp } from "./MappingHotkeyHelp";
import { MappingSelectionSidebarPanel } from "./MappingSelectionSidebarPanel";
import { MappingStageConfigPanel } from "./MappingStageConfigPanel";
import { MappingStageLayersPanel } from "./MappingStageLayersPanel";
import { MappingToolRail } from "./MappingToolRail";
import { MappingViewportControls } from "./MappingViewportControls";
import { SetupFixtureEditorPanel } from "./SetupFixtureEditorPanel";
import { SetupFixtureListPanel } from "./SetupFixtureListPanel";

type WithoutChildren<T> = Omit<T, "children">;

interface SetupMappingWorkspaceProps {
  className: string;
  panelRef: (element: HTMLElement) => void;
  fixtureList: WithoutChildren<ComponentProps<typeof SetupFixtureListPanel>>;
  patchMap: ComponentProps<typeof DmxPatchMapPanel>;
  fixtureEditor: ComponentProps<typeof SetupFixtureEditorPanel> | null;
  fixtureCount: number;
  projectorCount: number;
  filters: ComponentProps<typeof MappingFilterStrips>;
  toolRail: ComponentProps<typeof MappingToolRail>;
  viewportControls: ComponentProps<typeof MappingViewportControls>;
  stageConfig: ComponentProps<typeof MappingStageConfigPanel>;
  editableStage: WithoutChildren<ComponentProps<typeof MappingEditableStageShell>>;
  stageLayers: ComponentProps<typeof MappingStageLayersPanel>;
  selectionSidebar: ComponentProps<typeof MappingSelectionSidebarPanel>;
  hotkeyHelpOpen: boolean;
  onCloseHotkeyHelp: () => void;
}

export function SetupMappingWorkspace(props: SetupMappingWorkspaceProps) {
  return (
    <section class={props.className} ref={props.panelRef} tabIndex={-1}>
      <SetupFixtureListPanel {...props.fixtureList}>
        <DmxPatchMapPanel {...props.patchMap} />
      </SetupFixtureListPanel>
      <Show when={props.fixtureEditor}>
        {(fixtureEditor) => <SetupFixtureEditorPanel {...fixtureEditor()} />}
      </Show>
      <div class="mappingVisualizer visualizer">
        <div class="panelHeader">
          <h2>2D Mapping</h2>
          <span>{props.fixtureCount} fixture(s) / {props.projectorCount} projection surface(s)</span>
        </div>
        <MappingFilterStrips {...props.filters} />
        <div class="mappingStageShell">
          <MappingToolRail {...props.toolRail} />
          <div class="mappingStageViewport">
            <MappingViewportControls {...props.viewportControls} />
            <MappingStageConfigPanel {...props.stageConfig} />
            <MappingEditableStageShell {...props.editableStage}>
              <MappingStageLayersPanel {...props.stageLayers} />
            </MappingEditableStageShell>
          </div>
          <MappingSelectionSidebarPanel {...props.selectionSidebar} />
          <Show when={props.hotkeyHelpOpen}>
            <MappingHotkeyHelp onClose={props.onCloseHotkeyHelp} />
          </Show>
        </div>
      </div>
    </section>
  );
}
