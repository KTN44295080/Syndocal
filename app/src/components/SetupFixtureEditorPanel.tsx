import type { PatchedFixtureSummary } from "../types";

interface SetupFixtureEditorPanelProps {
  fixture: PatchedFixtureSummary;
  labelDraft: string;
  universeDraft: number;
  addressDraft: number;
  groupText: string;
  onUseProfileForPatch: (fixture: PatchedFixtureSummary) => void | Promise<void>;
  onDuplicateFixture: (fixture: PatchedFixtureSummary) => void | Promise<void>;
  onLabelDraft: (value: string) => void;
  onUniverseDraft: (value: number) => void;
  onAddressDraft: (value: number) => void;
  onApplyPatch: (fixture: PatchedFixtureSummary) => void | Promise<void>;
  onGroupText: (value: string) => void;
  onApplyGroups: (fixture: PatchedFixtureSummary) => void | Promise<void>;
}

export function SetupFixtureEditorPanel(props: SetupFixtureEditorPanelProps) {
  return (
    <div class="fixtureSetupEditor" data-fixture-patch-identity-editor>
      <div class="panelHeader">
        <h3>Fixture Setup</h3>
        <span data-no-localize>{props.fixture.label}</span>
      </div>
      <button onClick={() => void props.onUseProfileForPatch(props.fixture)}>
        Use Profile for Patch
      </button>
      <button onClick={() => void props.onDuplicateFixture(props.fixture)}>
        Duplicate Fixture
      </button>
      <label>
        Label
        <input value={props.labelDraft} onInput={(event) => props.onLabelDraft(event.currentTarget.value)} />
      </label>
      <div class="split">
        <label>
          Universe
          <input
            type="number"
            min="0"
            value={props.universeDraft}
            onInput={(event) => props.onUniverseDraft(Number(event.currentTarget.value))}
          />
        </label>
        <label>
          Address
          <input
            type="number"
            min="1"
            max="512"
            value={props.addressDraft}
            onInput={(event) => props.onAddressDraft(Number(event.currentTarget.value))}
          />
        </label>
      </div>
      <button onClick={() => void props.onApplyPatch(props.fixture)}>Apply Patch</button>
      <label>
        Groups
        <input
          value={props.groupText}
          onInput={(event) => props.onGroupText(event.currentTarget.value)}
          placeholder="front, movers"
        />
      </label>
      <button onClick={() => void props.onApplyGroups(props.fixture)}>Apply Groups</button>
    </div>
  );
}
