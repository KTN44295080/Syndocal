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
      <aside class="fixtureSetupContextPane">
        <Show
          when={props.fixtureEditor}
          fallback={
            <p
              class="empty fixtureSetupEmptyState"
              data-fixture-setup-empty
              title="Select a fixture in the patch grid or list to edit its setup."
            >
              Select a fixture in the patch grid or list to edit its setup.
            </p>
          }
        >
          {(fixtureEditor) => <SetupFixtureEditorPanel {...fixtureEditor()} />}
        </Show>
      </aside>
    </section>
  );
}
