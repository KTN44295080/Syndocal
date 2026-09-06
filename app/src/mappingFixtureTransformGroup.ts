import { fixtureTransformMatchesExpectation } from "./fixtureTransformConfirmation";
import type { MappingFixtureTransform } from "./mappingFixtureTransformBatch";
import type { EngineSnapshot } from "./types";

export type MappingFixtureTransformGroupCommit = (
  transforms: MappingFixtureTransform[],
  expectedEpoch: number,
  isCurrent: () => boolean,
) => Promise<boolean>;

/** One gesture is one admitted native transaction and one history entry. */
export function createMappingFixtureTransformGroupCommit(ports: {
  commit: (args: Record<string, unknown>) => Promise<unknown>;
  refreshSnapshot: () => Promise<EngineSnapshot | null>;
  setMessage: (message: string) => unknown;
}): MappingFixtureTransformGroupCommit {
  return async (transforms, expectedEpoch, isCurrent) => {
    if (!isCurrent()) return false;
    try {
      await ports.commit({
        transforms: transforms.map(({ fixture, update }) => ({
          fixtureId: fixture.id,
          position: update.position ?? fixture.position,
          rotation: update.rotation ?? fixture.rotation,
        })),
        __expectedProjectEpoch: expectedEpoch,
        __shouldAbortProjectMutation: () => !isCurrent(),
      });
      if (!isCurrent()) return false;
      const snapshot = await ports.refreshSnapshot();
      if (!isCurrent()) return false;
      const fixtures = new Map(snapshot?.fixtures.map((fixture) => [fixture.id, fixture]));
      if (!snapshot || !transforms.every(({ fixture, update }) =>
        fixtureTransformMatchesExpectation(fixtures.get(fixture.id), update))) {
        ports.setMessage("Could not confirm the fixture group transform. Refresh before retrying.");
        return false;
      }
      return true;
    } catch (error) {
      if (isCurrent()) ports.setMessage(String(error));
      return false;
    }
  };
}
