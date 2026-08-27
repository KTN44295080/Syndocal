import type { Setter } from "solid-js";
import type { FrontendTauriInvoke } from "./tauriInvokeCommands";
import type {
  Phase1SmokeReport,
  ProjectLoadResult,
  UserTemplateLoadResult,
  VisualizerRenderPayload,
} from "./types";
import type { ProjectAuthorityToken } from "./projectAuthority";
import type { SetupSubTab, WorkspaceTab } from "./uiModes";
import { downloadTextFile, safeExportFileNamePart } from "./svgExportHelpers";

export type Phase1ActionContext = {
  invoke: FrontendTauriInvoke;
  confirmDiscardProjectChanges: (actionLabel: string) => Promise<boolean>;
  captureProjectAuthorityIdentity: () => ProjectAuthorityToken;
  projectTransactionOwnerId: string;
  applyLoadedProjectResult: (result: ProjectLoadResult) => Promise<boolean>;
  loadedProjectMessage: (result: ProjectLoadResult) => string;
  refreshProjectControlMappings: () => Promise<unknown>;
  refreshSnapshot: (syncProjectState?: boolean, resetEditorDrafts?: boolean) => Promise<unknown>;
  projectFileLabel: () => string;
  setPhase1SmokeReport: Setter<Phase1SmokeReport | null>;
  setCurrentProjectPath: Setter<string | null>;
  setWorkspaceTab: Setter<WorkspaceTab>;
  setSetupSubTab: Setter<SetupSubTab>;
  setRawDmxUniverse: Setter<number>;
  setDmxTestChannel: Setter<number>;
  setDmxTestWidth: Setter<number>;
  setDmxTestValue: Setter<number>;
  setMessage: (text: string) => unknown;
};

export async function exportVisualizerRenderPayloadAction(context: Phase1ActionContext): Promise<void> {
  try {
    const payload = await context.invoke<VisualizerRenderPayload>("get_visualizer_render_payload", { config: null });
    const projectLabel = context.projectFileLabel().replace(/\s+\*$/, "");
    const envelope = {
      version: 1,
      software: "Syndocal",
      kind: "visualizer-render-payload",
      project: projectLabel,
      exported_at: new Date().toISOString(),
      payload,
    };
    const jsonText = `${JSON.stringify(envelope, null, 2)}\n`;
    const fileName = `${safeExportFileNamePart(projectLabel)}-visualizer-scene.json`;
    downloadTextFile(fileName, jsonText, "application/json;charset=utf-8");
    context.setMessage(
      `Exported visualizer scene JSON ${fileName} (${payload.scene.fixtures.length} fixture(s), ${payload.scene.video_surfaces.length} projection surface(s)).`,
    );
  } catch (error) {
    context.setMessage(String(error));
  }
}

export async function loadUserTemplateAction(context: Phase1ActionContext): Promise<void> {
  const authority = context.captureProjectAuthorityIdentity();
  if (!await context.confirmDiscardProjectChanges("create a project from a user template")) {
    context.setMessage("Template load canceled.");
    return;
  }
  try {
    const result = await context.invoke<UserTemplateLoadResult | null>("load_user_template", {
      ownerId: context.projectTransactionOwnerId,
      expectedEpoch: authority.project_epoch,
      expectedRevision: authority.project_revision,
      expectedCheckpointHash: authority.checkpoint_hash,
    });
    if (!result) {
      context.setMessage("Template load canceled.");
      return;
    }
    const applied = await context.applyLoadedProjectResult(result);
    if (!applied) return;
    context.setWorkspaceTab("setup");
    context.setSetupSubTab("patch");
    context.setMessage(
      `Created an unsaved project from ${result.label} (${result.profiles.length} embedded profiles, ${result.midi_mappings.length} MIDI, ${result.osc_mappings.length} OSC, ${result.dmx_mappings.length} DMX mappings). Outputs remain disarmed until explicit Arm; authored output settings were preserved.`,
    );
  } catch (error) {
    context.setMessage(`Template load failed: ${String(error)}`);
  }
}

export async function loadPhase1SampleProjectAction(context: Phase1ActionContext): Promise<void> {
  if (!await context.confirmDiscardProjectChanges("load the Phase 1 sample project")) {
    context.setMessage("Sample project load canceled.");
    return;
  }
  try {
    const result = await context.invoke<ProjectLoadResult>("load_phase1_sample_project");
    const applied = await context.applyLoadedProjectResult(result);
    if (!applied) return;
    context.setPhase1SmokeReport(null);
    context.setWorkspaceTab("setup");
    context.setSetupSubTab("patch");
    context.setMessage(context.loadedProjectMessage(result));
  } catch (error) {
    context.setMessage(String(error));
  }
}

export async function runPhase1SmokeAction(context: Phase1ActionContext): Promise<void> {
  try {
    const report = await context.invoke<Phase1SmokeReport>("run_phase1_smoke");
    await context.refreshProjectControlMappings();
    context.setPhase1SmokeReport(report);
    context.setCurrentProjectPath(null);
    context.setWorkspaceTab("setup");
    context.setSetupSubTab("video");
    context.setRawDmxUniverse(0);
    context.setDmxTestChannel(1);
    context.setDmxTestWidth(8);
    context.setDmxTestValue(255);
    await context.refreshSnapshot(true, true);
    const values = report.first_8.map((value) => value.toString(16).padStart(2, "0").toUpperCase()).join(" ");
    const expected = report.expected_first_8.map((value) => value.toString(16).padStart(2, "0").toUpperCase()).join(" ");
    context.setMessage(
      report.passed
        ? `Smoke passed: ${report.path}, ${report.cue_label}, ${report.primary_output_label}, U0 A1-A8 ${values}.`
        : `Smoke failed: ${report.primary_output_label}, active cue ${report.active_cue_id ?? "none"}, got ${values}, expected ${expected}.`,
    );
  } catch (error) {
    context.setMessage(String(error));
  }
}
