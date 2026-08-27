import type { Accessor, Setter } from "solid-js";
import type { SceneSettingsSurface } from "./components/SceneSettingsPane";
import type { ProjectAuthorityToken } from "./projectAuthority";
import type { FrontendTauriInvoke } from "./tauriInvokeCommands";
import type {
  CueListSummary,
  EngineSnapshot,
  ProjectHistoryMutationResult,
} from "./types";

interface SceneBankSceneCreationControllerOptions {
  snapshot: Accessor<EngineSnapshot>;
  setSelectedCueListId: Setter<number | null>;
  setCueLabel: Setter<string>;
  setSelectedSceneCueId: Setter<number | null>;
  setSelectedSceneEffectId: Setter<number | null>;
  setSceneSettingsSurface: (surface: SceneSettingsSurface) => void;
  setMessage: (message: string) => unknown;
  requireAuthoritativeCueList: (cueListId: number) => CueListSummary | null;
  flushProjectControlMappingsBeforeMutation: () => Promise<number>;
  projectMappingsAuthority: () => ProjectAuthorityToken;
  authoritativeApplicationIsCurrent: (value: unknown) => boolean;
  refreshSnapshot: () => Promise<EngineSnapshot | null>;
  invoke: FrontendTauriInvoke;
  projectTransactionOwnerId: string;
}

export function createSceneBankSceneCreationController(
  options: SceneBankSceneCreationControllerOptions,
): (cueListId: number) => Promise<boolean> {
  const createSceneInCueList = async (cueListId: number): Promise<boolean> => {
    const cueList = options.requireAuthoritativeCueList(cueListId);
    if (!cueList) {
      return false;
    }
    options.setSelectedCueListId(cueListId);
    const beforeCueIds = new Set(options.snapshot().cues.map((cue) => cue.id));
    try {
      const flushedEpoch = await options.flushProjectControlMappingsBeforeMutation();
      const currentAuthority = options.projectMappingsAuthority();
      if (flushedEpoch !== currentAuthority.project_epoch) {
        options.setMessage("Project changed while creating the Scene; nothing was applied.");
        return false;
      }
      const result = await options.invoke<ProjectHistoryMutationResult & { cue_id: number }>("create_scene_authoritative_v1", {
        mode: "empty",
        cueListId,
        expectedEpoch: currentAuthority.project_epoch,
        expectedRevision: currentAuthority.project_revision,
        expectedCheckpointHash: currentAuthority.checkpoint_hash,
        ownerId: options.projectTransactionOwnerId,
      });
      if (!options.authoritativeApplicationIsCurrent(result)) {
        options.setMessage("New Scene acknowledgement was stale; refresh before retrying.");
        return false;
      }
      const cueId = result.cue_id;
      if (!Number.isSafeInteger(cueId) || cueId <= 0 || beforeCueIds.has(cueId)) {
        options.setMessage("New Scene acknowledgement omitted a new committed Scene ID; refresh before retrying.");
        return false;
      }
      await options.refreshSnapshot();
      const createdCue = options.snapshot().cues
        .find((cue) => cue.id === cueId && cue.cue_list_id === cueListId);
      if (!createdCue) {
        options.setMessage("New Scene was acknowledged, but the refreshed project did not contain it.");
        return false;
      }
      options.setCueLabel("New Scene");
      options.setSelectedSceneCueId(createdCue.id);
      options.setSelectedSceneEffectId(null);
      options.setSceneSettingsSurface("contents");
      options.setMessage(`Created scene in ${cueList.label}`);
      return true;
    } catch (error) {
      options.setMessage(String(error));
      return false;
    }
  };
  return createSceneInCueList;
}
