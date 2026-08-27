import {
  fixtureTransformMatchesExpectation,
  type FixtureTransformExpectation,
} from "./fixtureTransformConfirmation";
import type { EngineSnapshot, PatchedFixtureSummary } from "./types";

export type MappingFixtureTransform = {
  fixture: PatchedFixtureSummary;
  update: FixtureTransformExpectation;
};

type MappingFixtureTransformBatchOptions = {
  transforms: MappingFixtureTransform[];
  setFixtureTransform: (
    fixture: PatchedFixtureSummary,
    update: FixtureTransformExpectation,
    refresh: false,
  ) => Promise<boolean>;
  refreshSnapshot: () => Promise<EngineSnapshot | null>;
  setMessage: (message: string) => unknown;
};

/**
 * Apply mapping transforms in a bounded order, then treat one refreshed engine
 * snapshot as the only authority. The backend currently exposes per-fixture
 * transactions, so a mid-batch failure cannot be rolled back atomically; stop
 * at the first rejection and state that partial application is possible.
 */
export async function applyMappingFixtureTransformBatch(
  options: MappingFixtureTransformBatchOptions,
) {
  const expectedTransforms = new Map(
    options.transforms.map(({ fixture, update }) => [fixture.id, { label: fixture.label, update }]),
  );
  const failedLabels = new Set<string>();
  for (const { fixture, update } of options.transforms) {
    if (!await options.setFixtureTransform(fixture, update, false)) {
      failedLabels.add(fixture.label);
      break;
    }
  }

  let refreshed: EngineSnapshot | null = null;
  try {
    refreshed = await options.refreshSnapshot();
  } catch {
    // The actionable message below covers a missing/failed authority refresh.
  }
  const refreshedFixtures = new Map(
    (refreshed?.fixtures ?? []).map((fixture) => [fixture.id, fixture]),
  );
  for (const [fixtureId, expected] of expectedTransforms) {
    if (!fixtureTransformMatchesExpectation(refreshedFixtures.get(fixtureId), expected.update)) {
      failedLabels.add(expected.label);
    }
  }

  if (failedLabels.size > 0) {
    options.setMessage(
      `Could not persist all fixture changes: ${[...failedLabels].join(", ")}. `
      + "Refreshed positions are authoritative; earlier fixtures may already have changed.",
    );
    return false;
  }
  return true;
}
