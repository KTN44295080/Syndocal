import { For, Show } from "solid-js";

interface EffectGroupRow {
  groupId: string;
  count: number;
}

interface EffectGroupTargetPanelProps {
  value: string;
  groups: EffectGroupRow[];
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
                <button class={isActive() ? "groupChip active" : "groupChip"} onClick={() => props.onToggleGroup(group.groupId)}>
                  {group.groupId}
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
