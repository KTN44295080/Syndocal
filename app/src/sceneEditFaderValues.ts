import type { CueSummary, PatchedFixtureSummary } from "./types";
import type { FaderValueMap } from "./createFaderValueController";

/** The editor reads authored targets, never the running look or MIDI projection. */
export function sceneEditFaderValues(
  fixtures: PatchedFixtureSummary[], cue: CueSummary, edits: FaderValueMap = {},
): FaderValueMap {
  const saved = new Map(cue.targets.map(target => [target.fixture_id,
    new Map(target.values.map(value => [value.attribute, value.value]))]));
  const values: FaderValueMap = {};
  for (const fixture of fixtures) {
    for (const control of fixture.controls) {
      const key = `${fixture.id}:${control.attribute}`;
      values[key] = edits[key] ?? saved.get(fixture.id)?.get(control.attribute) ?? control.default_value;
    }
  }
  return values;
}
