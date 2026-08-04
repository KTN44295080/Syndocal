import { Show, type ComponentProps, type JSX } from "solid-js";
import { DmxPatchMapPanel } from "./DmxPatchMapPanel";
import { SetupFixtureEditorPanel } from "./SetupFixtureEditorPanel";

interface SetupMappingWorkspaceProps {
  className: string;
  panelRef: (element: HTMLElement) => void;
  patchArmed: boolean;
  patchForm: JSX.Element;
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
        <Show
          when={props.patchArmed}
          fallback={
            <p class="empty patchProfileEmptyState" data-patch-profile-empty>
              Choose a fixture profile in Patch Source on the left to arm patching.
            </p>
          }
        >
          {props.patchForm}
        </Show>
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
