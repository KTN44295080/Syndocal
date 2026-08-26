import type { Accessor, Setter } from "solid-js";
import { nextBankAuthorityId } from "./bankAuthority";
import type { SceneSettingsSurface } from "./components/SceneSettingsPane";
import type { ProjectAuthorityToken } from "./projectAuthority";
import type { FrontendTauriInvoke } from "./tauriInvokeCommands";
import type {
  CueListSummary,
  CueSummary,
  EngineSnapshot,
  ProjectHistoryMutationResult,
} from "./types";
import { viewportFixtureData } from "./viewportFixtureData";

interface SceneBankSceneCreationControllerOptions {
  snapshot: Accessor<EngineSnapshot>;
  setSnapshot: Setter<EngineSnapshot>;
  setSelectedCueListId: Setter<number | null>;
  setCueLabel: Setter<string>;
  setSelectedSceneCueId: Setter<number | null>;
  setSelectedSceneEffectId: Setter<number | null>;
  setSceneSettingsSurface: (surface: SceneSettingsSurface) => void;
  setMessage: (message: string) => unknown;
  viewportFixture: string;
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
    if (options.viewportFixture === "scene-matrix") {
      // The browser fixture deliberately supports an authored empty Scene so
      // the bank action remains testable even when no fixture is patched. The
      // native lane below remains the backend's exact create contract.
      const template = options.snapshot().cues.find((cue) => cue.cue_list_id === cueListId)
        ?? options.snapshot().cues[0]
        ?? viewportFixtureData.cueRecallCue;
      const cueId = nextBankAuthorityId(options.snapshot().cues.map((cue) => cue.id));
      if (cueId === null) {
        options.setMessage("New Scene ID could not be allocated safely.");
        return false;
      }
      const cue: CueSummary = {
        ...structuredClone(template),
        id: cueId,
        cue_list_id: cueListId,
        cue_number: String(options.snapshot().cues.length + 1),
        label: "New Scene",
        group_id: null,
        recall_mode: "Coexist",
        fade_ms: 0,
        authored_beats: null,
        pre_wait_ms: 0,
        follow_ms: null,
        parts: [],
        mark: false,
        mib_fixture_ids: [],
        palette_targets: [],
        tracking: false,
        notes: "",
        targets: [],
        video_targets: [],
        video_output_targets: [],
        node_graph_targets: [],
        effect_targets: [],
        steps: [],
        child_timeline: null,
        live_modifiers: null,
      };
      options.setSnapshot((current) => ({ ...current, cues: [...current.cues, cue] }));
      options.setCueLabel("New Scene");
      options.setSelectedSceneCueId(cueId);
      options.setSelectedSceneEffectId(null);
      options.setSceneSettingsSurface("contents");
      options.setMessage(`Created scene in ${cueList.label}`);
      return true;
    }
    const beforeCueIds = new Set(options.snapshot().cues.map((cue) => cue.id));
    try {
      const flushedEpoch = await options.flushProjectControlMappingsBeforeMutation();
      const currentAuthority = options.projectMappingsAuthority();
      if (flushedEpoch !== currentAuthority.project_epoch) {
        options.setMessage("Project changed while creating the Scene; nothing was applied.");
        return false;
      }
      const result = await options.invoke<ProjectHistoryMutationResult>("create_empty_cue", {
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
      await options.refreshSnapshot();
      const createdCue = options.snapshot().cues
        .filter((cue) => cue.cue_list_id === cueListId && !beforeCueIds.has(cue.id))
        .sort((left, right) => right.id - left.id)[0];
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
