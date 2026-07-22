import type { AttributeControl, PatchedFixtureSummary } from "./types";

export const GROUP_STROBE_MAX_HZ = 30;

export const normalizedGroupStrobeFunctionAttribute = (attribute: string) =>
  [...attribute]
    .filter((character) => /[A-Za-z]/.test(character))
    .join("")
    .toLowerCase();

export const channelFunctionHasGroupStrobeMetadata = (
  fn: NonNullable<AttributeControl["functions"]>[number],
) => {
  const attribute = normalizedGroupStrobeFunctionAttribute(fn.attribute);
  const physicalFrom = fn.physical_from;
  const physicalTo = fn.physical_to;
  return (attribute === "shutterstrobe" || attribute === "strobe")
    && typeof physicalFrom === "number"
    && Number.isFinite(physicalFrom)
    && typeof physicalTo === "number"
    && Number.isFinite(physicalTo)
    && Math.abs(physicalTo - physicalFrom) > Number.EPSILON;
};

export const fixtureHasGroupStrobeMetadata = (
  fixture: Pick<PatchedFixtureSummary, "controls">,
) => fixture.controls.some((control) =>
  (control.functions ?? []).some(channelFunctionHasGroupStrobeMetadata));

export const groupStrobeCompatibleFixtureCount = (
  fixtures: Pick<PatchedFixtureSummary, "controls">[],
) => fixtures.filter(fixtureHasGroupStrobeMetadata).length;
