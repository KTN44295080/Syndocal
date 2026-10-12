export interface SceneRenameScope { projectEpoch: number; identityGeneration: number; }

/** Keep a name gesture bound to its project across mapping and snapshot awaits. */
export function createSceneRenameController(options: {
  captureScope: () => SceneRenameScope;
  isCurrent: (scope: SceneRenameScope) => boolean;
  flushMappings: () => Promise<unknown>;
  refreshSnapshot: () => Promise<unknown>;
  rename: (cueId: number, label: string) => Promise<boolean>;
  message: (message: string) => void;
}) {
  return async (cueId: number, label: string): Promise<boolean> => {
    const name = label.trim();
    if (!name) return false;
    const scope = options.captureScope();
    const current = () => {
      if (options.isCurrent(scope)) return true;
      options.message('Project changed while renaming the scene; refresh before retrying.');
      return false;
    };
    try {
      await options.flushMappings();
      if (!current()) return false;
      await options.refreshSnapshot();
      if (!current()) return false;
      const saved = await options.rename(cueId, name);
      return current() && saved;
    } catch (error) {
      if (current()) options.message(String(error));
      return false;
    }
  };
}
