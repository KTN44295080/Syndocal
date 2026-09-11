import type { Accessor, Setter } from "solid-js";
import type { FrontendTauriInvoke } from "./tauriInvokeCommands";
import type { ProjectAuthorityToken } from "./projectAuthority";
import type { DvcImportReport, ProjectLoadResult } from "./types";
import type { SetupSubTab, WorkspaceTab } from "./uiModes";

export type DvcImportProjectLoadResult = {
  report: DvcImportReport;
  load: ProjectLoadResult;
};

export type DvcImportControllerOptions<AppliedResult> = {
  invoke: FrontendTauriInvoke;
  projectTransactionOwnerId: string;
  daslightProjectImportBusy: Accessor<boolean>;
  captureProjectAuthorityIdentity: () => ProjectAuthorityToken;
  isProjectAuthorityIdentityCurrent: (captured: ProjectAuthorityToken) => boolean;
  confirmDiscardProjectChanges: (actionLabel: string) => Promise<boolean>;
  applyLoadedProjectResult: (
    result: ProjectLoadResult,
    legacyCurrentPath: string | null,
  ) => Promise<AppliedResult>;
  projectAuthorityApplicationResultIsCurrent: (result: AppliedResult) => boolean;
  setDaslightProjectImportBusy: Setter<boolean>;
  setMessage: (text: string, key?: string) => unknown;
  setWorkspaceTab: Setter<WorkspaceTab>;
  setSetupSubTab: Setter<SetupSubTab>;
  setDvcImportReport: Setter<DvcImportReport | null>;
};

export function createDvcImportController<AppliedResult>(
  options: DvcImportControllerOptions<AppliedResult>,
) {
  const importDaslightProject = async () => {
    if (options.daslightProjectImportBusy()) {
      return;
    }
    const authority = options.captureProjectAuthorityIdentity();
    if (!await options.confirmDiscardProjectChanges("import a Daslight Project (.dvc)")) {
      options.setMessage("Daslight Project import canceled.");
      return;
    }
    options.setDaslightProjectImportBusy(true);
    options.setMessage("Importing Daslight Project...", "daslight-project-import-busy");
    try {
      const imported = await options.invoke<DvcImportProjectLoadResult | null>("import_daslight_project_with_result", {
        path: null,
        ownerId: options.projectTransactionOwnerId,
        expectedEpoch: authority.project_epoch,
        expectedRevision: authority.project_revision,
        expectedCheckpointHash: authority.checkpoint_hash,
      });
      if (!imported) {
        options.setMessage("Daslight Project import canceled.");
        return;
      }
      // The paired backend result is the same fenced publication that owns
      // the report mappings. Applying it directly avoids racing a later
      // authority event/refresh and keeps imported MIDI+DMX bindings in the
      // coordinator before the UI advertises their counts.
      const applied = await options.applyLoadedProjectResult(imported.load, null);
      if (!options.projectAuthorityApplicationResultIsCurrent(applied)) return;
      const report = imported.report;
      options.setWorkspaceTab("setup");
      options.setSetupSubTab("patch");
      options.setDvcImportReport(report);
      options.setMessage(
        `Imported Daslight Project (.dvc): ${report.summary.fixtures} fixtures, ${report.summary.cues} cues, ${report.midi_mappings?.length ?? 0} MIDI and ${report.dmx_mappings?.length ?? 0} DMX mappings. Save As to create a Syndocal Project (.sdc).`,
      );
    } catch (error) {
      if (!options.isProjectAuthorityIdentityCurrent(authority)) return;
      options.setMessage(`Daslight Project import failed: ${String(error)}`);
    } finally {
      options.setDaslightProjectImportBusy(false);
    }
  };

  return { importDaslightProject };
}
