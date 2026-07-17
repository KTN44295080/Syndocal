import { For, Show } from "solid-js";
import { groupIdentityCss } from "../identityColor";

interface EffectGroupRow {
  groupId: string;
  count: number;
}

interface EffectGroupTargetPanelProps {
  value: string;
  groups: EffectGroupRow[];
  groupColors?: Record<string, string>;
  activeGroupIds: string[];
  onValue: (value: string) => void;
  onToggleGroup: (groupId: string) => void;
}

export function EffectGroupTargetPanel(props: EffectGroupTargetPanelProps) {
  return (
    <div class="effectGroupPicker">
      <label>
        Target groups
        <input value={props.value} onInput={(event) => props.onValue(event.currentTarget.value)} placeholder="front, movers" />
      </label>
      <Show when={props.groups.length > 0}>
        <div class="groupOverview">
          <For each={props.groups}>
            {(group) => {
              const isActive = () => props.activeGroupIds.includes(group.groupId);
              return (
                <button
                  class={isActive() ? "groupChip active" : "groupChip"}
                  style={{ "--identity": groupIdentityCss(group.groupId, props.groupColors, "fill") }}
                  onClick={() => props.onToggleGroup(group.groupId)}
                >
                  <span data-no-localize>{group.groupId}</span>
                  <span>{group.count}</span>
                </button>
              );
            }}
          </For>
        </div>
      </Show>
    </div>
  );
}
