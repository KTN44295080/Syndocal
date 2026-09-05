import type { CueSummary, TimelineLayerSummary } from "./types";
import { timelineSceneBlockSpanMs } from "./timelineSceneBlocks";

export interface InlineChildParent {
  id: number;
  cue_id: number;
  layer_id: number;
  time_ms: number;
  duration_ms: number;
  total_duration_ms: number;
  source_offset_ms?: number;
  rate: number | null;
  conform_to_tempo: boolean;
  loop_fill: boolean;
  loop_count: number;
}
export interface InlineChildBlock {
  key: string;
  cueId: number;
  label: string;
  startMs: number;
  endMs: number;
  rail: number;
}
export interface InlineChildRow {
  key: string;
  parentEventId: number;
  parentCueId: number;
  parentLabel: string;
  label: string;
  muted: boolean;
  railCount: number;
  blocks: InlineChildBlock[];
  issue: string | null;
}

/** Read-only authored positions. This does not create editable Timeline items. */
export function projectInlineChildTimelineRows(
  parents: readonly InlineChildParent[],
  cues: readonly CueSummary[],
): InlineChildRow[] {
  const byCue = new Map(cues.map((cue) => [cue.id, cue]));
  return parents.flatMap((parent) => {
    const cue = byCue.get(parent.cue_id);
    const child = cue?.child_timeline;
    if (!cue || !child) return [];
    const base = { parentEventId: parent.id, parentCueId: cue.id, parentLabel: cue.label };
    const issueRow = (issue: string): InlineChildRow[] => [{
      ...base, key: `${parent.id}:issue`, label: "子タイムライン", muted: false,
      railCount: 1, blocks: [], issue,
    }];
    // Tempo/rate inheritance and repeat windows need a separate engine-backed
    // projection contract. Do not invent positions for those cases.
    if (parent.conform_to_tempo || parent.loop_fill || parent.loop_count !== 1
      || (parent.rate ?? 1) !== 1 || child.tempo_driven) {
      return issueRow("テンポ・反復設定の詳細は子タイムラインを開いて確認");
    }
    const offset = parent.source_offset_ms ?? 0;
    if (offset < 0 || (child.events ?? []).some((event) => event.track === "Lighting" && event.conform_to_tempo)) {
      return issueRow("テンポ・開始位置設定の詳細は子タイムラインを開いて確認");
    }
    const end = parent.time_ms + parent.total_duration_ms;
    if (![parent.time_ms, parent.total_duration_ms, offset, end].every(Number.isSafeInteger)
      || parent.time_ms < 0 || parent.total_duration_ms <= 0 || parent.duration_ms <= 0) {
      return issueRow("親の時間範囲を表示できません。子タイムラインを開いて確認");
    }
    const layers: readonly TimelineLayerSummary[] = child.layers?.length ? child.layers : [{
      id: 0, label: "Lighting", kind: "Lighting", order: 0,
      muted: false, locked: false, solo: false, expanded: false,
    }];
    const lighting = layers.filter((layer) => layer.kind === "Lighting")
      .slice().sort((a, b) => a.order - b.order || a.id - b.id);
    const events = (child.events ?? []).filter((event) => event.track === "Lighting");
    if (events.some((event) => !lighting.some((layer) => layer.id === (event.layer_id ?? 0)))) {
      return issueRow("照明レーンを特定できません。子タイムラインを開いて確認");
    }
    return lighting.map((layer): InlineChildRow => {
      const row: InlineChildRow = {
        ...base, key: `${parent.id}:${layer.id}`, label: layer.label, muted: layer.muted,
        railCount: 1, blocks: [], issue: null,
      };
      const candidates = events.filter((event) => (event.layer_id ?? 0) === layer.id)
        .slice().sort((a, b) => a.time_ms - b.time_ms || a.id - b.id);
      const ends: number[] = [];
      for (const event of candidates) {
        const span = timelineSceneBlockSpanMs(event);
        const rawStart = parent.time_ms + event.time_ms - offset;
        const rawEnd = rawStart + span;
        if (![event.time_ms, event.duration_ms, span, rawStart, rawEnd].every(Number.isSafeInteger)
          || event.time_ms < 0 || event.duration_ms < 0 || span < 0) {
          row.issue = "照明イベントの時間を表示できません。子タイムラインを開いて確認";
          row.blocks = [];
          break;
        }
        if (span === 0 ? rawStart < parent.time_ms || rawStart >= end : rawEnd <= parent.time_ms || rawStart >= end) continue;
        const startMs = Math.max(parent.time_ms, rawStart);
        const endMs = Math.min(end, rawEnd);
        let rail = ends.findIndex((railEnd) => railEnd <= startMs);
        if (rail === -1) rail = ends.length;
        ends[rail] = Math.max(endMs, startMs + 1);
        row.blocks.push({ key: `${parent.id}:${event.id}`, cueId: event.cue_id,
          label: byCue.get(event.cue_id)?.label ?? `Cue ${event.cue_id}`, startMs, endMs, rail });
      }
      row.railCount = Math.max(1, ends.length);
      return row;
    });
  });
}
