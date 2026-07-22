import type { PatchedFixtureSummary } from "../types";
import { GROUP_STROBE_MAX_HZ } from "../groupStrobe";

export interface TouchFixtureGroupFlagState {
  count: number;
  anyHighlighted: boolean;
  anySoloed: boolean;
  anyParked: boolean;
}

interface TouchFixtureActionsPanelProps {
  fixture: PatchedFixtureSummary;
  selectedGroupId?: string | null;
  groupFlagState: TouchFixtureGroupFlagState;
  globalAnyFlagged: boolean;
  groupStrobeHz: number;
  groupStrobeFixtureCount: number;
  onSetFixtureHighlight: (fixtureId: number, enabled: boolean) => void | Promise<void>;
  onSetFixtureSolo: (fixtureId: number, enabled: boolean) => void | Promise<void>;
  onSetFixturePark: (fixtureId: number, enabled: boolean) => void | Promise<void>;
  onSetGroupHighlight: (groupId: string, enabled: boolean) => void | Promise<void>;
  onSetGroupSolo: (groupId: string, enabled: boolean) => void | Promise<void>;
  onSetGroupStrobe: (groupId: string, rateHz: number) => void | Promise<void>;
  onSetGroupPark: (groupId: string, enabled: boolean) => void | Promise<void>;
  onClearFixtureFlags: () => void | Promise<void>;
}

export function TouchFixtureActionsPanel(props: TouchFixtureActionsPanelProps) {
  return (
    <div class="touchGuardRow">
      {props.selectedGroupId ? (
        <>
          <button
            disabled={props.groupFlagState.count === 0}
            onClick={() => void props.onSetGroupHighlight(props.selectedGroupId!, !props.groupFlagState.anyHighlighted)}
          >
            {props.groupFlagState.anyHighlighted ? "Clear Group Highlight" : "Group Highlight"}
          </button>
          <button
            disabled={props.groupFlagState.count === 0}
            onClick={() => void props.onSetGroupSolo(props.selectedGroupId!, !props.groupFlagState.anySoloed)}
          >
            {props.groupFlagState.anySoloed ? "Clear Group Solo" : "Group Solo"}
          </button>
          <label class="touchGroupStrobeControl">
            Strobe
            <input
              type="range"
              min="0"
              max={GROUP_STROBE_MAX_HZ}
              step="1"
              value={props.groupStrobeHz}
              disabled={props.groupFlagState.count === 0 || props.groupStrobeFixtureCount === 0}
              aria-label="Touch group strobe rate"
              onChange={(event) => void props.onSetGroupStrobe(
                props.selectedGroupId!,
                Number(event.currentTarget.value),
              )}
            />
            <span>{props.groupStrobeHz > 0 ? `${props.groupStrobeHz.toFixed(0)} Hz` : "Off"}</span>
          </label>
          <button
            disabled={props.groupFlagState.count === 0}
            onClick={() => void props.onSetGroupPark(props.selectedGroupId!, !props.groupFlagState.anyParked)}
          >
            {props.groupFlagState.anyParked ? "Clear Group Park" : "Group Park"}
          </button>
          <button disabled={!props.globalAnyFlagged} onClick={() => void props.onClearFixtureFlags()}>
            Clear Flags
          </button>
        </>
      ) : (
        <>
          <button onClick={() => void props.onSetFixtureHighlight(props.fixture.id, !props.fixture.highlighted)}>
            {props.fixture.highlighted ? "Clear Highlight" : "Highlight"}
          </button>
          <button onClick={() => void props.onSetFixtureSolo(props.fixture.id, !props.fixture.soloed)}>
            {props.fixture.soloed ? "Clear Solo" : "Solo"}
          </button>
          <button onClick={() => void props.onSetFixturePark(props.fixture.id, !props.fixture.parked)}>
            {props.fixture.parked ? "Clear Park" : "Park"}
          </button>
          <button disabled={!props.globalAnyFlagged} onClick={() => void props.onClearFixtureFlags()}>
            Clear Flags
          </button>
        </>
      )}
    </div>
  );
}
