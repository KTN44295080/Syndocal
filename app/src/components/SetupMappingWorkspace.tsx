import { Show, type ComponentProps } from "solid-js";
import { DmxPatchMapPanel } from "./DmxPatchMapPanel";
import { SetupFixtureEditorPanel } from "./SetupFixtureEditorPanel";

interface SetupMappingWorkspaceProps {
  className: string;
  panelRef: (element: HTMLElement) => void;
  patchMap: ComponentProps<typeof DmxPatchMapPanel>;
  fixtureEditor: ComponentProps<typeof SetupFixtureEditorPanel> | null;
}

export function SetupMappingWorkspace(props: SetupMappingWorkspaceProps) {
  return (
    <section
      class={`${props.className} setupPatchTopPanel`}
      ref={props.panelRef}
      tabIndex={-1}
    >
      <div class="setupPatchAddressDesk">
        <DmxPatchMapPanel {...props.patchMap} />
      </div>
      <Show when={props.fixtureEditor}>
        {(fixtureEditor) => <SetupFixtureEditorPanel {...fixtureEditor()} />}
      </Show>
    </section>
  );
}
