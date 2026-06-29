import type { PatchedFixtureSummary } from "../types";

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
  onSetFixtureHighlight: (fixtureId: number, enabled: boolean) => void | Promise<void>;
  onSetFixtureSolo: (fixtureId: number, enabled: boolean) => void | Promise<void>;
  onSetFixturePark: (fixtureId: number, enabled: boolean) => void | Promise<void>;
  onSetGroupHighlight: (groupId: string, enabled: boolean) => void | Promise<void>;
  onSetGroupSolo: (groupId: string, enabled: boolean) => void | Promise<void>;
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
