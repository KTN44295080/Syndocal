import { For, Show } from "solid-js";
import type { AttributeControl } from "../types";

interface FaderGridPanelProps {
  controls: AttributeControl[];
  selectedFixtureId?: number | null;
  selectedGroupId?: string | null;
  valueForControl: (control: AttributeControl) => number;
  onSetFixtureAttribute: (fixtureId: number, attribute: string, value: number) => void | Promise<void>;
  onSetGroupAttribute: (groupId: string, attribute: string, value: number) => void | Promise<void>;
}

export function FaderGridPanel(props: FaderGridPanelProps) {
  const hasSelectedFixture = () => props.selectedFixtureId !== null && props.selectedFixtureId !== undefined;

  const setControlValue = (control: AttributeControl, value: number) => {
    const fixtureId = props.selectedFixtureId;
    if (fixtureId === null || fixtureId === undefined) {
      return;
    }

    const groupId = props.selectedGroupId;
    if (groupId) {
      void props.onSetGroupAttribute(groupId, control.attribute, value);
    } else {
      void props.onSetFixtureAttribute(fixtureId, control.attribute, value);
    }
  };

  return (
    <div class="faderGrid">
      <For each={props.controls}>
        {(control) => (
          <div class="fader">
            <input
              type="range"
              min="0"
              max="65535"
              value={props.valueForControl(control)}
              onInput={(event) => setControlValue(control, Number(event.currentTarget.value))}
            />
            <strong>{control.attribute}</strong>
            <span>{control.resolution === "SixteenBit" ? "16-bit" : "8-bit"}</span>
            <small>{control.offsets.join(", ")}</small>
          </div>
        )}
      </For>
      <Show when={!hasSelectedFixture()}>
        <p class="empty">Select or patch a fixture.</p>
      </Show>
      <Show when={hasSelectedFixture() && props.controls.length === 0}>
        <p class="empty">No controls in this category.</p>
      </Show>
    </div>
  );
}
