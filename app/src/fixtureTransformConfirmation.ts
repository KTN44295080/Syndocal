import type { PatchFixtureRequest, PatchedFixtureSummary } from "./types";

export type FixtureTransformExpectation = {
  position?: PatchFixtureRequest["position"];
  rotation?: PatchFixtureRequest["rotation"];
};

export const fixtureTransformConfirmationTolerance = 0.01;

const approximatelyEqual = (actual: number, expected: number) =>
  Math.abs(actual - expected) <= fixtureTransformConfirmationTolerance;

/**
 * A successful IPC reply is not persistence evidence. Accept a transform only
 * when the refreshed authoritative fixture contains every field requested by
 * the caller within the persisted hundredth-unit precision.
 */
export const fixtureTransformMatchesExpectation = (
  fixture: PatchedFixtureSummary | undefined,
  expected: FixtureTransformExpectation,
) => {
  if (!fixture) return false;
  if (expected.position && !(
    approximatelyEqual(fixture.position.x, expected.position.x)
    && approximatelyEqual(fixture.position.y, expected.position.y)
    && approximatelyEqual(fixture.position.z, expected.position.z)
  )) return false;
  if (expected.rotation && !(
    approximatelyEqual(fixture.rotation.pitch, expected.rotation.pitch)
    && approximatelyEqual(fixture.rotation.yaw, expected.rotation.yaw)
    && approximatelyEqual(fixture.rotation.roll, expected.rotation.roll)
  )) return false;
  return true;
};
