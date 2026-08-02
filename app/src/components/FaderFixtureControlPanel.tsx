import { Show } from "solid-js";
import type { PatchedFixtureSummary } from "../types";

interface FixtureFlagState {
  count: number;
  anyHighlighted: boolean;
  anySoloed: boolean;
  anyParked: boolean;
}

interface FaderFixtureControlPanelProps {
  selectedGroupId: string | null;
  selectedGroupLabel: string | null;
  selectedGroupFixtureCount: number;
  patchedFixtureCount: number;
  selectedGroupSubmasterLevel: number;
  selectedFixture: PatchedFixtureSummary | null;
  groupFlagState: FixtureFlagState;
  globalAnyFlagged: boolean;
  onSetGroupSubmaster: (groupId: string, level: number) => void | Promise<void>;
  onSavePreset: () => void | Promise<void>;
  onLoadPreset: () => void | Promise<void>;
  onLoadPresetForGroup: (groupId: string) => void | Promise<void>;
  onLoadPresetForAllMatching: () => void | Promise<void>;
  onDuplicateFixture: (fixture: PatchedFixtureSummary) => void | Promise<void>;
  onRemoveFixture: (fixtureId: number) => void | Promise<void>;
  onSetFixtureHighlight: (fixtureId: number, highlighted: boolean) => void | Promise<void>;
  onSetFixtureSolo: (fixtureId: number, soloed: boolean) => void | Promise<void>;
  onSetFixturePark: (fixtureId: number, parked: boolean) => void | Promise<void>;
  onSetGroupHighlight: (groupId: string, highlighted: boolean) => void | Promise<void>;
  onSetGroupSolo: (groupId: string, soloed: boolean) => void | Promise<void>;
  onSetGroupPark: (groupId: string, parked: boolean) => void | Promise<void>;
  onClearFixtureFlags: () => void | Promise<void>;
  onSetFixtureTransform: (
    fixture: PatchedFixtureSummary,
    next: {
      position?: PatchedFixtureSummary["position"];
      rotation?: PatchedFixtureSummary["rotation"];
    },
  ) => void | Promise<void>;
}

export function FaderFixtureControlPanel(props: FaderFixtureControlPanelProps) {
  return (
    <>
      <Show when={props.selectedGroupId}>
        {(groupId) => (
          <div class="groupControlBanner">
            <div>
              <strong data-no-localize>{props.selectedGroupLabel ?? groupId()}</strong>
              <span>{props.selectedGroupFixtureCount} fixture(s)</span>
            </div>
            <label class="groupSubmasterControl">
              Submaster
              <input
                type="range"
                min="0"
                max="1"
                step="0.01"
                value={props.selectedGroupSubmasterLevel}
                onChange={(event) => void props.onSetGroupSubmaster(groupId(), Number(event.currentTarget.value))}
              />
              <span>{Math.round(props.selectedGroupSubmasterLevel * 100)}%</span>
            </label>
          </div>
        )}
      </Show>
      <div class="buttonRow">
        <button disabled={!props.selectedFixture} onClick={() => void props.onSavePreset()}>
          Save Preset
        </button>
        <button disabled={!props.selectedFixture} onClick={() => void props.onLoadPreset()}>
          Load Preset
        </button>
        <button
          disabled={!props.selectedGroupId || props.selectedGroupFixtureCount === 0}
          onClick={() => props.selectedGroupId && void props.onLoadPresetForGroup(props.selectedGroupId)}
        >
          Load Group Preset
        </button>
        <button disabled={props.patchedFixtureCount === 0} onClick={() => void props.onLoadPresetForAllMatching()}>
          Load Matching Preset
        </button>
      </div>
      <Show when={props.selectedFixture}>
        {(fixture) => (
          <>
            <div class="buttonRow">
              <button onClick={() => void props.onDuplicateFixture(fixture())}>Duplicate</button>
              <button onClick={() => void props.onRemoveFixture(fixture().id)}>Remove Fixture</button>
            </div>
            <div class="buttonRow">
              <Show
                when={props.selectedGroupId}
                fallback={
                  <>
                    <button onClick={() => void props.onSetFixtureHighlight(fixture().id, !fixture().highlighted)}>
                      {fixture().highlighted ? "Clear Highlight" : "Highlight"}
                    </button>
                    <button onClick={() => void props.onSetFixtureSolo(fixture().id, !fixture().soloed)}>
                      {fixture().soloed ? "Clear Solo" : "Solo"}
                    </button>
                    <button onClick={() => void props.onSetFixturePark(fixture().id, !fixture().parked)}>
                      {fixture().parked ? "Clear Park" : "Park"}
                    </button>
                    <button disabled={!props.globalAnyFlagged} onClick={() => void props.onClearFixtureFlags()}>
                      Clear Flags
                    </button>
                  </>
                }
              >
                {(groupId) => (
                  <>
                    <button
                      disabled={props.groupFlagState.count === 0}
                      onClick={() => void props.onSetGroupHighlight(groupId(), !props.groupFlagState.anyHighlighted)}
                    >
                      {props.groupFlagState.anyHighlighted ? "Clear Group Highlight" : "Group Highlight"}
                    </button>
                    <button
                      disabled={props.groupFlagState.count === 0}
                      onClick={() => void props.onSetGroupSolo(groupId(), !props.groupFlagState.anySoloed)}
                    >
                      {props.groupFlagState.anySoloed ? "Clear Group Solo" : "Group Solo"}
                    </button>
                    <button
                      disabled={props.groupFlagState.count === 0}
                      onClick={() => void props.onSetGroupPark(groupId(), !props.groupFlagState.anyParked)}
                    >
                      {props.groupFlagState.anyParked ? "Clear Group Park" : "Group Park"}
                    </button>
                    <button disabled={!props.globalAnyFlagged} onClick={() => void props.onClearFixtureFlags()}>
                      Clear Flags
                    </button>
                  </>
                )}
              </Show>
            </div>
            <div class="transformEditor">
              <strong>Transform</strong>
              <div class="triple">
                <label>
                  X
                  <input
                    type="number"
                    value={fixture().position.x}
                    onInput={(event) =>
                      void props.onSetFixtureTransform(fixture(), {
                        position: { ...fixture().position, x: Number(event.currentTarget.value) },
                      })
                    }
                  />
                </label>
                <label>
                  Y
                  <input
                    type="number"
                    value={fixture().position.y}
                    onInput={(event) =>
                      void props.onSetFixtureTransform(fixture(), {
                        position: { ...fixture().position, y: Number(event.currentTarget.value) },
                      })
                    }
                  />
                </label>
                <label>
                  Z
                  <input
                    type="number"
                    value={fixture().position.z}
                    onInput={(event) =>
                      void props.onSetFixtureTransform(fixture(), {
                        position: { ...fixture().position, z: Number(event.currentTarget.value) },
                      })
                    }
                  />
                </label>
              </div>
              <div class="triple">
                <label>
                  Pitch
                  <input
                    type="number"
                    value={fixture().rotation.pitch}
                    onInput={(event) =>
                      void props.onSetFixtureTransform(fixture(), {
                        rotation: { ...fixture().rotation, pitch: Number(event.currentTarget.value) },
                      })
                    }
                  />
                </label>
                <label>
                  Yaw
                  <input
                    type="number"
                    value={fixture().rotation.yaw}
                    onInput={(event) =>
                      void props.onSetFixtureTransform(fixture(), {
                        rotation: { ...fixture().rotation, yaw: Number(event.currentTarget.value) },
                      })
                    }
                  />
                </label>
                <label>
                  Roll
                  <input
                    type="number"
                    value={fixture().rotation.roll}
                    onInput={(event) =>
                      void props.onSetFixtureTransform(fixture(), {
                        rotation: { ...fixture().rotation, roll: Number(event.currentTarget.value) },
                      })
                    }
                  />
                </label>
              </div>
            </div>
          </>
        )}
      </Show>
    </>
  );
}
