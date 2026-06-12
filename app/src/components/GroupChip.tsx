import { Show } from "solid-js";
import type { JSX } from "solid-js";

export interface GroupChipContentProps {
  groupId: string;
  badge: string | number;
}

export const groupLeafLabel = (groupId: string) => groupId.split("/").filter(Boolean).at(-1) ?? groupId;

export const groupParentPath = (groupId: string) => {
  const parts = groupId.split("/").filter(Boolean);
  return parts.length > 1 ? parts.slice(0, -1).join("/") : "";
};

export const groupChipStyle = (depth: number): JSX.CSSProperties =>
  ({ "padding-left": `${8 + Math.min(Math.max(depth, 0), 4) * 12}px` }) as JSX.CSSProperties;

export function GroupChipContent(props: GroupChipContentProps) {
  return (
    <>
      <b>
        <strong>{groupLeafLabel(props.groupId)}</strong>
        <Show when={groupParentPath(props.groupId)}>{(parent) => <small>{parent()}</small>}</Show>
      </b>
      <span>{props.badge}</span>
    </>
  );
}
