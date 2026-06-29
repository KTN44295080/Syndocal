import { For } from "solid-js";
import type { AttributeControl } from "../types";

interface TouchGenericAttributeGridProps {
  controls: AttributeControl[];
  currentValue: (control: AttributeControl) => number;
  formatValue: (value: number) => string;
  onSetValue: (control: AttributeControl, value: number) => void | Promise<void>;
}

export function TouchGenericAttributeGrid(props: TouchGenericAttributeGridProps) {
  return (
    <div class="touchGenericAttributeGrid">
      <For each={props.controls}>
        {(control) => (
          <label class="touchGenericAttributeRow">
            <span>
              <strong>{control.channel_name || control.attribute}</strong>
              <small>{control.attribute}</small>
            </span>
            <input
              type="range"
              min="0"
              max="65535"
              value={props.currentValue(control)}
              onInput={(event) => void props.onSetValue(control, Number(event.currentTarget.value))}
            />
            <output>{props.formatValue(props.currentValue(control))}</output>
          </label>
        )}
      </For>
    </div>
  );
}
