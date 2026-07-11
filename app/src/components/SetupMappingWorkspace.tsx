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
  compact?: boolean;
  onOpenMapping?: () => void;
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
      <div class={props.compact ? "mappingVisualizer visualizer compact" : "mappingVisualizer visualizer"}>
        <div class="panelHeader">
          <h2>2D Mapping</h2>
          <div class="panelHeaderActions">
            <span>{props.fixtureCount} fixture(s) / {props.projectorCount} projection surface(s)</span>
            <Show when={props.compact && props.onOpenMapping}>
              <button onClick={() => props.onOpenMapping?.()}>Open Stage Map</button>
            </Show>
          </div>
        </div>
        <Show when={!props.compact}>
          <MappingFilterStrips {...props.filters} />
        </Show>
        <div class={props.compact ? "mappingStageShell compact" : "mappingStageShell"}>
          <Show when={!props.compact}>
            <MappingToolRail {...props.toolRail} />
          </Show>
          <div class="mappingStageViewport">
            <Show when={!props.compact}>
              <MappingViewportControls {...props.viewportControls} />
              <MappingStageConfigPanel {...props.stageConfig} />
            </Show>
            <MappingEditableStageShell {...props.editableStage}>
              <MappingStageLayersPanel {...props.stageLayers} />
            </MappingEditableStageShell>
          </div>
          <Show when={!props.compact}>
            <MappingSelectionSidebarPanel {...props.selectionSidebar} />
          </Show>
          <Show when={!props.compact && props.hotkeyHelpOpen}>
            <MappingHotkeyHelp onClose={props.onCloseHotkeyHelp} />
          </Show>
        </div>
      </div>
    </section>
  );
}
