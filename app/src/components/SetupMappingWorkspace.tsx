import { Show, type ComponentProps } from "solid-js";
import { DmxPatchMapPanel } from "./DmxPatchMapPanel";
import { LoadedProfileSummaryPanel } from "./LoadedProfileSummaryPanel";
import { SetupFixtureEditorPanel } from "./SetupFixtureEditorPanel";

interface SetupMappingWorkspaceProps {
  className: string;
  panelRef: (element: HTMLElement) => void;
  patchMap: ComponentProps<typeof DmxPatchMapPanel>;
  fixtureEditor: ComponentProps<typeof SetupFixtureEditorPanel> | null;
  /* #63: with the Library tab merged into Patch, the armed profile's summary
     (mode DMX map, channel functions, geometry) shows in the context pane
     whenever no fixture is selected. */
  profileSummary: ComponentProps<typeof LoadedProfileSummaryPanel> | null;
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
            <Show
              when={props.profileSummary}
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
              {(profileSummary) => (
                <div class="profile fixtureSetupProfileSummary" data-patch-profile-summary>
                  <LoadedProfileSummaryPanel {...profileSummary()} />
                </div>
              )}
            </Show>
          }
        >
          {(fixtureEditor) => <SetupFixtureEditorPanel {...fixtureEditor()} />}
        </Show>
      </aside>
    </section>
  );
}
