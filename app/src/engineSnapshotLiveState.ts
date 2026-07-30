import type {
  AttributeValueSummary,
  EngineSnapshot,
  EngineSnapshotSyncResponse,
  PatchedFixtureSummary,
} from "./types";

export const mergeEngineSnapshotSyncResponse = (
  current: EngineSnapshot,
  response: EngineSnapshotSyncResponse,
): EngineSnapshot => {
  if (response.full) return response.full;
  const {
    active_group_cue_ids: activeGroupCueIds,
    cue_live_modifiers: cueLiveModifiers,
    group_colors: groupColors,
    ...delta
  } = response.delta ?? {};
  return {
    ...current,
    ...delta,
    ...(activeGroupCueIds === undefined
      ? {}
      : { active_group_cue_ids: activeGroupCueIds }),
    ...(cueLiveModifiers === undefined
      ? {}
      : { cue_live_modifiers: cueLiveModifiers }),
    ...(groupColors === undefined ? {} : { group_colors: groupColors }),
  } as EngineSnapshot;
};

const activeSnapshotCues = (snapshot: EngineSnapshot) => {
  const activeCueIds = new Set<number>(Object.values(snapshot.active_group_cue_ids ?? {}));
  if (snapshot.active_cue_id !== null && snapshot.active_cue_id !== undefined) {
    activeCueIds.add(snapshot.active_cue_id);
  }
  return snapshot.cues.filter((cue) => activeCueIds.has(cue.id));
};

const cueTargetValuesByFixture = (snapshot: EngineSnapshot) => {
  const valuesByFixture = new Map<number, Map<string, number>>();
  for (const cue of activeSnapshotCues(snapshot)) {
    for (const target of cue.targets) {
      const fixtureValues = valuesByFixture.get(target.fixture_id) ?? new Map<string, number>();
      for (const value of target.values) {
        fixtureValues.set(value.attribute.toLowerCase(), value.value);
      }
      valuesByFixture.set(target.fixture_id, fixtureValues);
    }
  }
  return valuesByFixture;
};

const renderedCueAttributeValues = (
  fixture: PatchedFixtureSummary,
  valuesByAttribute: ReadonlyMap<string, number> | undefined,
): AttributeValueSummary[] =>
  fixture.controls.map((control) => ({
    attribute: control.attribute,
    value: valuesByAttribute?.get(control.attribute.toLowerCase()) ?? 0,
  }));

/**
 * A read-only fixture view for live stage color.
 *
 * A cue's rendered DMX normally arrives through dmx_previews. Some deployed
 * backends return no preview universe when no DMX route is configured. In that
 * state fixture.attribute_values still contains the imported profile home
 * values (often 255 for every RGB cell), not the active cue result. Resolve the
 * already-present active Cue targets into attribute_values so the existing
 * profile color reader receives rendered values and untouched fixtures remain
 * dark. The project snapshot itself is never mutated.
 */
export const snapshotLiveFixtures = (
  snapshot: EngineSnapshot,
): PatchedFixtureSummary[] => {
  const activeCues = activeSnapshotCues(snapshot);
  if (activeCues.length === 0) {
    return snapshot.fixtures;
  }
  const valuesByFixture = cueTargetValuesByFixture(snapshot);
  return snapshot.fixtures.map((fixture) => ({
    ...fixture,
    attribute_values: renderedCueAttributeValues(
      fixture,
      valuesByFixture.get(fixture.id),
    ),
  }));
};
