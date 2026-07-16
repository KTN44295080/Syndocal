import { Show, type ComponentProps } from "solid-js";
import { DmxPatchMapPanel } from "./DmxPatchMapPanel";
import { MappingStageConfigPanel } from "./MappingStageConfigPanel";
import { SetupFixtureEditorPanel } from "./SetupFixtureEditorPanel";

interface SetupMappingWorkspaceProps {
  className: string;
  panelRef: (element: HTMLElement) => void;
  compact: boolean;
  patchMap: ComponentProps<typeof DmxPatchMapPanel>;
  fixtureEditor: ComponentProps<typeof SetupFixtureEditorPanel> | null;
  stageConfig: ComponentProps<typeof MappingStageConfigPanel>;
}

export function SetupMappingWorkspace(props: SetupMappingWorkspaceProps) {
  return (
    <section
      class={`${props.className} ${props.compact ? "setupPatchTopPanel" : "mappingSetupTopPanel"}`}
      ref={props.panelRef}
      tabIndex={-1}
    >
      <Show
        when={props.compact}
        fallback={<MappingStageConfigPanel {...props.stageConfig} />}
      >
        <div class="setupPatchAddressDesk">
          <DmxPatchMapPanel {...props.patchMap} />
        </div>
        <Show when={props.fixtureEditor}>
          {(fixtureEditor) => <SetupFixtureEditorPanel {...fixtureEditor()} />}
        </Show>
      </Show>
    </section>
  );
}
