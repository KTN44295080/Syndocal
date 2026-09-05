import { For, Show } from "solid-js";
import type { InlineChildRow } from "../timelineInlineChildProjection";
import { cueIdentityCss, type CueIdentitySource } from "../identityColor";
import "./TimelineInlineChildRows.css";

type PositionedRow = InlineChildRow & { top: number; height: number };
type CommonProps = { rows: readonly PositionedRow[]; onOpen: (cueId: number) => void };
const stopEditing = (event: Event) => { event.preventDefault(); event.stopPropagation(); };
const description = (row: InlineChildRow) => `${row.parentLabel} / ${row.label} — 表示のみ。子タイムラインを開いて編集`;

export function TimelineInlineChildGutters(props: CommonProps & { sectionTop: number }) {
  return <For each={props.rows}>{(row) => <button
    type="button" class="timelineInlineChildGutter" data-timeline-inline-child-gutter
    style={{ top: `${row.top - props.sectionTop}px`, height: `${row.height}px` }}
    title={description(row)} aria-label={description(row)}
    onPointerDown={(event) => event.stopPropagation()}
    onClick={(event) => { event.stopPropagation(); props.onOpen(row.parentCueId); }}
    onKeyDown={(event) => {
      if (event.key !== "Enter" && event.key !== " ") return;
      stopEditing(event); props.onOpen(row.parentCueId);
    }}
  ><span data-no-localize>↳ {row.parentLabel} / {row.label}</span></button>}</For>;
}

export function TimelineInlineChildCanvas(props: CommonProps & {
  width: number; visibleStartMs: number; visibleEndMs: number; laneHeight: number;
  cueIdentities?: Record<number, CueIdentitySource>;
}) {
  const x = (time: number) => (time - props.visibleStartMs)
    / Math.max(1, props.visibleEndMs - props.visibleStartMs) * props.width;
  return <For each={props.rows}>{(row) => <g data-timeline-inline-child
    data-parent-event-id={row.parentEventId}
    class="timelineInlineChildCanvas" opacity={row.muted ? 0.5 : 1}
    onPointerDown={stopEditing} onClick={stopEditing} onContextMenu={stopEditing}
    onDragOver={stopEditing} onDrop={stopEditing}
    onDblClick={(event) => { stopEditing(event); props.onOpen(row.parentCueId); }}
  >
    <title>{description(row)}</title>
    <rect class="timelineInlineChildBackground" x={0} y={row.top} width={props.width} height={row.height} />
    <Show when={row.issue}><text class="timelineInlineChildLabel" x={8} y={row.top + 20}>{row.issue}</text></Show>
    <For each={row.blocks.filter((block) => (block.startMs === block.endMs
      ? block.startMs >= props.visibleStartMs : block.endMs > props.visibleStartMs)
      && block.startMs < props.visibleEndMs)}>{(block) => {
      const left = () => Math.max(0, x(block.startMs));
      const width = () => Math.min(props.width - left(), Math.max(2, x(block.endMs) - left()));
      const color = (role: "fill" | "band" | "text") => {
        const identity = props.cueIdentities?.[block.cueId];
        return cueIdentityCss(block.cueId, identity?.color, role, identity?.groupId, identity?.groupColor);
      };
      return <g style={{ "--identity": color("fill"), "--identity-band": color("band"), "--identity-text": color("text") }}>
        <title>{block.label} — {description(row)}</title>
        <rect class="timelineInlineChildBlock" x={left()} y={row.top + block.rail * props.laneHeight + 2}
          width={width()} height={props.laneHeight - 4} />
        <Show when={width() > 28}>
          <svg x={left() + 4} y={row.top + block.rail * props.laneHeight + 2} width={Math.max(0, width() - 8)} height={props.laneHeight - 4} overflow="hidden">
            <text class="timelineInlineChildLabel" x={0} y={18} data-no-localize>{block.label}</text>
          </svg>
        </Show>
      </g>;
    }}</For>
  </g>}</For>;
}
