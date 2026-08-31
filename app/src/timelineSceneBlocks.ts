import type { TimelineEventDraft } from "./editorDrafts";
import type { FullBankAuthoritySnapshot } from "./bankAuthority";
import type { FrontendTauriInvoke } from "./tauriInvokeCommands";
import { sceneCueKind, type SceneCueKind } from "./sceneCueKind";
import type { ClockSource, CueSummary, TimelineCueEventSummary, TimelineTrackKind, VideoLayerSummary } from "./types";
import { projectTimelineBlockStretch, type TimelineStretchMode } from "./timelineBlockGestures";

export interface TimelineSceneBlockAddDraft {
  cue_id: number;
  time_ms: number;
  time_beats: number | null;
  track: TimelineTrackKind;
  layer_id: number | null;
  duration_ms: number;
  duration_beats: number | null;
  conform_to_tempo: boolean;
  loop_fill: boolean;
  source_offset_ms: number;
  fade_in_ms: number;
  fade_out_ms: number;
  loop_count: number;
  jump_to_event_id: number | null;
}

export interface TimelineSceneBlockPlacementUpdate {
  event_id: number;
  cue_id: number;
  time_ms: number;
  time_beats: number | null;
  track: TimelineTrackKind;
  layer_id: number | null;
  duration_ms: number;
  duration_beats: number | null;
  conform_to_tempo: boolean;
  loop_fill: boolean;
  fade_in_ms: number;
  fade_out_ms: number;
  loop_count: number;
  jump_to_event_id: number | null;
}

interface TimelineSceneBlockControllerOptions {
  invoke: FrontendTauriInvoke;
  /**
   * Root-Timeline-only authoritative timing commit. The existing Apply DTO
   * cannot carry source_offset_ms, so this callback returns false whenever an
   * edit is outside the proven timing-only boundary. Callers then retain the
   * legacy direct command instead of silently dropping source semantics.
   */
  commitRootSceneBlockTiming?: (
    event: TimelineCueEventSummary,
    next: TimelineEventDraft,
  ) => Promise<boolean>;
  snapTimeMs: (timeMs: number) => number;
  /** Returns beat intent for Beat/Bar snap, or undefined when the current snap mode is not musical. */
  snappedTimeBeats: (timeMs: number) => number | null | undefined;
  /** Converts a direct millisecond edit into beat intent at the engine's current BPM. */
  timeBeatsAtCurrentBpm: (timeMs: number) => number | null;
  hasEventId: (eventId: number) => boolean;
  getEventById: (eventId: number) => TimelineCueEventSummary | undefined;
  getEventDraft: (event: TimelineCueEventSummary) => TimelineEventDraft;
  setEventDraft: (eventId: number, draft: TimelineEventDraft) => void;
  getAddDurationMs: () => number;
  getAddLoopCount: () => number;
  getAddJumpToEventId: () => number | null;
  getBpm: () => number;
  getCueAuthoredBeats: (cueId: number) => number | null;
  setNextStartMs: (timeMs: number) => void;
  setMessage: (message: string) => void;
  refreshSnapshot: () => Promise<unknown>;
}

const clamp = (value: number, min: number, max: number) => Math.min(max, Math.max(min, value));
const finiteOr = (value: number, fallback: number) => Number.isFinite(value) ? value : fallback;

export const timelineExecutionIsLive = (
  playing: boolean,
  clockSource: ClockSource,
  externalSyncLocked: boolean,
  _externalSyncAgeMs: number | null = null,
) => playing || ((clockSource === "MidiTimecode" || clockSource === "Ltc") && externalSyncLocked);
const timelineEventDraftFromEvent = (event: TimelineCueEventSummary): TimelineEventDraft => ({
  cue_id: event.cue_id,
  time_ms: event.time_ms,
  time_beats: event.time_beats ?? null,
  track: event.track,
  layer_id: event.layer_id ?? null,
  duration_ms: event.duration_ms ?? 0,
  duration_beats: event.duration_beats ?? null,
  conform_to_tempo: event.conform_to_tempo ?? false,
  loop_fill: event.loop_fill ?? false,
  source_offset_ms: event.source_offset_ms ?? 0,
  fade_in_ms: event.fade_in_ms ?? 0,
  fade_out_ms: event.fade_out_ms ?? 0,
  loop_count: event.loop_count ?? 1,
  jump_to_event_id: event.jump_to_event_id ?? null,
});

export interface TimelineMarkerDragProjection {
  x: number;
  max_x: number;
  grab_offset_x: number;
  start_client_x: number;
  moved: boolean;
}

export const beginTimelineMarkerDragProjection = (
  eventX: number,
  eventWidth: number,
  pointerX: number,
  clientX: number,
): TimelineMarkerDragProjection => ({
  x: eventX,
  max_x: Math.max(0, 100 - eventWidth),
  grab_offset_x: pointerX - eventX,
  start_client_x: clientX,
  moved: false,
});

export const updateTimelineMarkerDragProjection = (
  drag: TimelineMarkerDragProjection,
  pointerX: number,
  clientX: number,
  thresholdPx = 4,
): TimelineMarkerDragProjection => {
  const moved = drag.moved || Math.abs(clientX - drag.start_client_x) >= thresholdPx;
  return {
    ...drag,
    moved,
    x: moved ? clamp(pointerX - drag.grab_offset_x, 0, drag.max_x) : drag.x,
  };
};

export const shouldCommitTimelineMarkerDrag = (
  drag: Pick<TimelineMarkerDragProjection, "moved">,
  canceled = false,
) => drag.moved && !canceled;

export const timelineSceneBlockLoopDivisionPositions = (
  eventWidth: number,
  loopCount: number,
  visibleDivisions: number,
) => {
  const normalizedLoopCount = Math.max(1, Math.round(finiteOr(loopCount, 1)));
  const normalizedVisibleDivisions = Math.min(
    Math.max(0, normalizedLoopCount - 1),
    Math.max(0, Math.round(finiteOr(visibleDivisions, 0))),
  );
  if (eventWidth <= 0 || normalizedVisibleDivisions === 0) return [];
  return Array.from({ length: normalizedVisibleDivisions }, (_, index) => {
    const iteration = clamp(
      Math.round(((index + 1) * normalizedLoopCount) / (normalizedVisibleDivisions + 1)),
      1,
      normalizedLoopCount - 1,
    );
    return eventWidth * (iteration / normalizedLoopCount);
  });
};

export const reconcileTimelineSceneBlockJumpTarget = (
  eventIds: Iterable<number>,
  jumpToEventId: number | null,
) => {
  if (jumpToEventId === null) return null;
  return new Set(eventIds).has(jumpToEventId) ? jumpToEventId : null;
};

export const reconcileTimelineSceneBlockPickerTarget = (
  eventIds: number[],
  _sourceEventId: number | null,
  targetEventId: number | null,
) => {
  const availableIds = new Set(eventIds);
  if (targetEventId !== null && availableIds.has(targetEventId)) return targetEventId;
  return null;
};

export const timelineSceneBlockSourceSummary = (cue: CueSummary | undefined) => {
  if (!cue) return "Source Cue is missing";
  const lightingFixtureIds = new Set([
    ...cue.targets.map((target) => target.fixture_id),
    ...(cue.palette_targets ?? []).flatMap((target) => target.fixture_ids),
  ]);
  const lightingTargetCount = lightingFixtureIds.size;
  const paletteTargetCount = (cue.palette_targets ?? []).length;
  const videoTargetCount = cue.video_targets.length + cue.video_output_targets.length;
  const effectTargetCount = (cue.effect_targets ?? []).length + (cue.node_graph_targets ?? []).length;
  return `L ${lightingTargetCount} · P ${paletteTargetCount} · V ${videoTargetCount} · FX ${effectTargetCount} · Fade ${cue.fade_ms} ms`;
};

// Timeline and Lighting deliberately share one classification authority.
// The focused data-URL checkers inject this canonical module explicitly.
export const timelineSceneBlockCueKind: (
  cue: Pick<CueSummary, "child_timeline" | "effect_targets">,
) => SceneCueKind = sceneCueKind;

export const buildTimelineSceneBlockCueOptions = (cues: CueSummary[]) => cues.map((cue) => ({
  id: cue.id,
  cue_list_id: cue.cue_list_id,
  cue_number: cue.cue_number || String(cue.id),
  label: cue.label,
  fade_ms: Math.max(0, finiteOr(cue.fade_ms, 0)),
  kind: timelineSceneBlockCueKind(cue),
  replace_group: (cue.recall_mode ?? "Coexist") === "ReplaceGroup",
  flash: cue.live_modifiers?.flash ?? false,
  super_scene: Boolean(cue.child_timeline),
  authored_beats: cue.authored_beats ?? null,
  step_count: (cue.steps ?? []).length,
  source_summary: timelineSceneBlockSourceSummary(cue),
}));

export const timelineSceneBlockSourcePickerOptions = <T extends {
  id: number;
  cue_list_id: number;
  cue_number: string;
  label: string;
  authored_beats: number | null;
  step_count: number;
  source_summary: string;
}>(
  cueOptions: T[],
  query: string,
  selectedCueId: number | null,
  limit = 80,
) => {
  const normalizedQuery = query.trim().toLocaleLowerCase();
  const queryTokens = normalizedQuery.split(/\s+/).filter(Boolean);
  const filtered = normalizedQuery.length === 0
    ? cueOptions
    : cueOptions.filter((cue) => {
        const searchable = `list ${cue.cue_list_id} l${cue.cue_list_id} cue ${cue.id} ${cue.cue_number} ${cue.label} ${cue.source_summary}`
          .toLocaleLowerCase();
        return queryTokens.every((token) => searchable.includes(token));
      });
  const visible = filtered.slice(0, Math.max(1, limit));
  if (normalizedQuery.length > 0 || selectedCueId === null || visible.some((cue) => cue.id === selectedCueId)) {
    return visible;
  }
  const selectedCue = cueOptions.find((cue) => cue.id === selectedCueId);
  return selectedCue
    ? [selectedCue, ...visible.filter((cue) => cue.id !== selectedCue.id)].slice(0, Math.max(1, limit))
    : visible;
};

export const timelineSceneBlockCueOptionsEqual = (
  previous: ReturnType<typeof buildTimelineSceneBlockCueOptions>,
  next: ReturnType<typeof buildTimelineSceneBlockCueOptions>,
) => previous.length === next.length && previous.every((option, index) => {
  const candidate = next[index];
    return option.id === candidate.id &&
    option.cue_list_id === candidate.cue_list_id &&
    option.cue_number === candidate.cue_number &&
    option.label === candidate.label &&
    option.fade_ms === candidate.fade_ms &&
    option.kind === candidate.kind &&
    option.replace_group === candidate.replace_group &&
    option.flash === candidate.flash &&
    option.super_scene === candidate.super_scene &&
    option.authored_beats === candidate.authored_beats &&
    option.step_count === candidate.step_count &&
    option.source_summary === candidate.source_summary;
});

export interface TimelineSourceShelfBankView<OptionT> {
  /** Stable unique key derived from identity ids; never from display labels. */
  key: string;
  cue_list_id: number;
  /** Exact authoritative Bank label; never rewritten or synthesized. */
  label: string;
  scenes: OptionT[];
}

/**
 * The Scene Matrix Bank identity key.  Timeline uses this exact key rather
 * than a Cue colour so every Scene in one authoritative Bank has one colour.
 */
export const timelineSourceShelfBankIdentity = (cueListId: number) => `bank:${cueListId}`;

const validTimelineSceneBlockIdentity = (value: unknown): value is number =>
  typeof value === "number" && Number.isSafeInteger(value) && value > 0;
const validTimelineSceneBlockEventIdentity = (value: unknown): value is number =>
  typeof value === "number" && Number.isSafeInteger(value) && value >= 0;

/**
 * Canonicalize the complete Cue-to-Cue child Timeline graph before admitting
 * any source. A graph with a missing endpoint, malformed/duplicate event id,
 * or cycle is not partly usable: callers receive null and close every input.
 */
const timelineSceneBlockChildReferenceGraph = (
  authority: FullBankAuthoritySnapshot,
): ReadonlyMap<number, readonly number[]> | null => {
  if (authority.issue || authority.cueById.size !== authority.cues.length) return null;
  const graph = new Map<number, number[]>();
  for (const cue of authority.cues) {
    if (!validTimelineSceneBlockIdentity(cue.id) || authority.cueById.get(cue.id) !== cue) return null;
    const child = cue.child_timeline;
    const references: number[] = [];
    if (child !== null && child !== undefined) {
      if (typeof child !== "object" || Array.isArray(child) || !Array.isArray(child.events)) return null;
      const eventIds = new Set<number>();
      for (const event of child.events) {
        if (
          event === null
          || typeof event !== "object"
          || !validTimelineSceneBlockEventIdentity(event.id)
          || !validTimelineSceneBlockIdentity(event.cue_id)
          || eventIds.has(event.id)
          || !authority.cueById.has(event.cue_id)
        ) return null;
        eventIds.add(event.id);
        references.push(event.cue_id);
      }
    }
    graph.set(cue.id, references);
  }

  // An iterative depth-first walk avoids accepting a cycle hidden beyond a
  // nested Super Scene, without depending on stack depth or partial subsets.
  const state = new Map<number, 0 | 1 | 2>();
  for (const rootCueId of graph.keys()) {
    if (state.get(rootCueId) === 2) continue;
    state.set(rootCueId, 1);
    const pending: Array<{ cueId: number; nextIndex: number }> = [{ cueId: rootCueId, nextIndex: 0 }];
    while (pending.length > 0) {
      const frame = pending[pending.length - 1];
      const references = graph.get(frame.cueId);
      if (!references) return null;
      if (frame.nextIndex >= references.length) {
        state.set(frame.cueId, 2);
        pending.pop();
        continue;
      }
      const nextCueId = references[frame.nextIndex];
      frame.nextIndex += 1;
      const nextState = state.get(nextCueId) ?? 0;
      if (nextState === 1) return null;
      if (nextState === 0) {
        state.set(nextCueId, 1);
        pending.push({ cueId: nextCueId, nextIndex: 0 });
      }
    }
  }
  return graph;
};

const childReferenceCueIdsReaching = (
  graph: ReadonlyMap<number, readonly number[]>,
  targetCueId: number,
): ReadonlySet<number> => {
  const reverse = new Map<number, number[]>();
  for (const [sourceCueId, references] of graph) {
    for (const referencedCueId of references) {
      const sources = reverse.get(referencedCueId);
      if (sources) sources.push(sourceCueId);
      else reverse.set(referencedCueId, [sourceCueId]);
    }
  }
  const reaching = new Set<number>([targetCueId]);
  const pending = [targetCueId];
  while (pending.length > 0) {
    const cueId = pending.pop();
    if (cueId === undefined) break;
    for (const sourceCueId of reverse.get(cueId) ?? []) {
      if (reaching.has(sourceCueId)) continue;
      reaching.add(sourceCueId);
      pending.push(sourceCueId);
    }
  }
  return reaching;
};

/**
 * One authoritative Scene-source admission set for every Timeline authoring
 * edge. A root Timeline may reference every exact Scene. A child Timeline may
 * reference an acyclic nested Super Scene, but never itself or a Scene whose
 * child-reference graph reaches that owner. This preserves recursive DVC
 * import/transport while rejecting exactly the cyclic authoring targets.
 *
 * This returns null, rather than a partial set, whenever the full Bank/Cue
 * graph or child owner cannot be proven. Callers must treat null as closed and
 * must not substitute an ID-only membership check.
 */
export const timelineSceneBlockAllowedCueIds = (
  authority: FullBankAuthoritySnapshot,
  timelineChildCueId: number | null = null,
): ReadonlySet<number> | null => {
  const graph = timelineSceneBlockChildReferenceGraph(authority);
  if (!graph) return null;
  if (timelineChildCueId === null) return new Set(authority.cues.map((cue) => cue.id));
  if (!validTimelineSceneBlockIdentity(timelineChildCueId) || !authority.cueById.get(timelineChildCueId)?.child_timeline) {
    return null;
  }
  const reachesOwnerCueIds = childReferenceCueIdsReaching(graph, timelineChildCueId);
  return new Set(authority.cues
    .filter((cue) => !reachesOwnerCueIds.has(cue.id))
    .map((cue) => cue.id));
};

/** Exact terminal admission predicate; never infer eligibility from a cue ID alone. */
export const timelineSceneBlockCueAllowedByAuthority = (
  cueId: number,
  authority: FullBankAuthoritySnapshot,
  timelineChildCueId: number | null = null,
) => Number.isSafeInteger(cueId)
  && cueId > 0
  && timelineSceneBlockAllowedCueIds(authority, timelineChildCueId)?.has(cueId) === true;

/**
 * The Timeline root must expose every authoritative Scene. A child Timeline
 * excludes only the owner and Cue graph paths that return to it; acyclic
 * nested Super Scenes remain valid. Every other omission is invalid.
 */
export const timelineSourceShelfCueOptionsMatchAuthority = <OptionT extends { id: number; cue_list_id: number }>(
  cueOptions: readonly OptionT[],
  authority: FullBankAuthoritySnapshot,
  timelineChildCueId: number | null = null,
): boolean => {
  const expectedCueIds = timelineSceneBlockAllowedCueIds(authority, timelineChildCueId);
  if (expectedCueIds === null) return false;
  if (cueOptions.length !== expectedCueIds.size) return false;
  const seenOptionIds = new Set<number>();
  for (const option of cueOptions) {
    const exactCue = authority.cueById.get(option.id);
    if (
      seenOptionIds.has(option.id) ||
      !expectedCueIds.has(option.id) ||
      !exactCue ||
      exactCue.cue_list_id !== option.cue_list_id
    ) return false;
    seenOptionIds.add(option.id);
  }
  return seenOptionIds.size === expectedCueIds.size;
};

/**
 * Groups Timeline source-shelf scenes from the one complete App authority.
 * Any authority fault or transformed option that no longer matches its exact
 * authoritative Scene closes the complete result. There is deliberately no
 * partial "unavailable Bank" grouping path.
 */
export const groupTimelineSourceShelfBanks = <OptionT extends { id: number; cue_list_id: number }>(
  cueOptions: readonly OptionT[],
  authority: FullBankAuthoritySnapshot,
  timelineChildCueId: number | null = null,
): TimelineSourceShelfBankView<OptionT>[] => {
  if (!timelineSourceShelfCueOptionsMatchAuthority(cueOptions, authority, timelineChildCueId)) return [];
  const scenesByBank = new Map<number, OptionT[]>();
  for (const option of cueOptions) {
    const bucket = scenesByBank.get(option.cue_list_id);
    if (bucket) bucket.push(option);
    else scenesByBank.set(option.cue_list_id, [option]);
  }
  return authority.cueLists.map((bank) => ({
    key: `bank:${bank.id}`,
    cue_list_id: bank.id,
    label: bank.label,
    scenes: scenesByBank.get(bank.id) ?? [],
  }));
};

/** Preserve keyed DOM/card state when polling returns semantically unchanged Banks. */
export const timelineSourceShelfBankViewsEqual = <OptionT>(
  previous: readonly TimelineSourceShelfBankView<OptionT>[],
  next: readonly TimelineSourceShelfBankView<OptionT>[],
) => previous.length === next.length && previous.every((bank, index) => {
  const candidate = next[index];
  return bank.key === candidate.key &&
    bank.cue_list_id === candidate.cue_list_id &&
    bank.label === candidate.label &&
    bank.scenes.length === candidate.scenes.length &&
    bank.scenes.every((scene, sceneIndex) => Object.is(scene, candidate.scenes[sceneIndex]));
});

export const buildTimelineSceneBlockRows = (
  events: TimelineCueEventSummary[],
  authority: FullBankAuthoritySnapshot,
) => {
  // A Timeline event with an unresolved source cannot be represented as an
  // invented Bank 0. The complete surface is unavailable when Bank authority
  // is faulted; otherwise omit only a source that vanished between snapshots.
  // Neither case may produce a placement candidate.
  if (authority.issue) return [];
  return events.flatMap((event) => {
    const cue = authority.cueById.get(event.cue_id);
    if (!cue) return [];
    return [{
      ...event,
      duration_ms: event.duration_ms ?? 0,
      source_offset_ms: event.source_offset_ms ?? 0,
      fade_in_ms: event.fade_in_ms ?? 0,
      fade_out_ms: event.fade_out_ms ?? 0,
      loop_count: event.loop_count ?? 1,
      jump_to_event_id: event.jump_to_event_id ?? null,
      cue_number: cue.cue_number || String(event.cue_id),
      cue_label: cue.label,
      cue_list_id: cue.cue_list_id,
      source_summary: timelineSceneBlockSourceSummary(cue),
    }];
  });
};

export const timelineSceneBlockRowsEqual = (
  previous: ReturnType<typeof buildTimelineSceneBlockRows>,
  next: ReturnType<typeof buildTimelineSceneBlockRows>,
) => previous.length === next.length && previous.every((event, index) => {
  const candidate = next[index];
  return event.id === candidate.id &&
    event.cue_id === candidate.cue_id &&
    event.time_ms === candidate.time_ms &&
    (event.time_beats ?? null) === (candidate.time_beats ?? null) &&
    event.track === candidate.track &&
    (event.layer_id ?? null) === (candidate.layer_id ?? null) &&
    event.duration_ms === candidate.duration_ms &&
    (event.duration_beats ?? null) === (candidate.duration_beats ?? null) &&
    (event.conform_to_tempo ?? false) === (candidate.conform_to_tempo ?? false) &&
    (event.loop_fill ?? false) === (candidate.loop_fill ?? false) &&
    (event.source_offset_ms ?? 0) === (candidate.source_offset_ms ?? 0) &&
    (event.rate ?? null) === (candidate.rate ?? null) &&
    (event.fade_in_ms ?? 0) === (candidate.fade_in_ms ?? 0) &&
    (event.fade_out_ms ?? 0) === (candidate.fade_out_ms ?? 0) &&
    event.loop_count === candidate.loop_count &&
    event.jump_to_event_id === candidate.jump_to_event_id &&
    event.cue_number === candidate.cue_number &&
    event.cue_label === candidate.cue_label &&
    event.cue_list_id === candidate.cue_list_id &&
    event.source_summary === candidate.source_summary;
});

export const reconcileTimelineEventDrafts = (
  events: TimelineCueEventSummary[],
  current: Record<number, TimelineEventDraft>,
) => {
  const eventIds = new Set(events.map((event) => event.id));
  const next: Record<number, TimelineEventDraft> = {};
  let changed = Object.keys(current).length !== events.length;
  for (const event of events) {
    const existing = current[event.id];
    const draft = existing ?? timelineEventDraftFromEvent(event);
    const jumpToEventId = draft.jump_to_event_id !== null && !eventIds.has(draft.jump_to_event_id)
      ? null
      : draft.jump_to_event_id;
    next[event.id] = jumpToEventId === draft.jump_to_event_id
      ? draft
      : { ...draft, jump_to_event_id: jumpToEventId };
    if (!existing || next[event.id] !== existing) changed = true;
  }
  return changed ? next : current;
};

export const timelineEventDraftMatchesSummary = (
  event: TimelineCueEventSummary,
  draft: TimelineEventDraft,
) => draft.cue_id === event.cue_id &&
  draft.time_ms === event.time_ms &&
  draft.time_beats === (event.time_beats ?? null) &&
  draft.track === event.track &&
  draft.layer_id === (event.layer_id ?? null) &&
  draft.duration_ms === (event.duration_ms ?? 0) &&
  draft.duration_beats === (event.duration_beats ?? null) &&
  draft.conform_to_tempo === (event.conform_to_tempo ?? false) &&
  draft.loop_fill === (event.loop_fill ?? false) &&
  draft.source_offset_ms === (event.source_offset_ms ?? 0) &&
  draft.fade_in_ms === (event.fade_in_ms ?? 0) &&
  draft.fade_out_ms === (event.fade_out_ms ?? 0) &&
  draft.loop_count === (event.loop_count ?? 1) &&
  draft.jump_to_event_id === (event.jump_to_event_id ?? null);

export const timelineEventDraftsAreDirty = (
  events: TimelineCueEventSummary[],
  drafts: Record<number, TimelineEventDraft>,
) => {
  const eventIds = new Set(events.map((event) => event.id));
  if (Object.keys(drafts).some((eventId) => !eventIds.has(Number(eventId)))) return true;
  return events.some((event) => {
    const draft = drafts[event.id];
    return Boolean(draft && !timelineEventDraftMatchesSummary(event, draft));
  });
};

export const createTimelineSceneBlockViewportFixture = (
  baseCue: CueSummary,
  videoLayer: VideoLayerSummary,
) => {
  const cues: CueSummary[] = [
    {
      ...baseCue,
      label: "Opening Wash",
      effect_targets: [],
      node_graph_targets: [],
    },
    {
      ...baseCue,
      id: baseCue.id + 1,
      cue_number: "2",
      label: "Video Drop",
      fade_ms: 320,
      targets: [],
      video_targets: [{ layer_id: videoLayer.id, state: videoLayer.state }],
      effect_targets: [],
      node_graph_targets: [],
    },
  ];
  const events: TimelineCueEventSummary[] = [
    {
      id: 1,
      cue_id: cues[0].id,
      time_ms: 0,
      track: "Lighting",
      duration_ms: 1000,
      loop_count: 4,
      jump_to_event_id: null,
    },
    {
      id: 2,
      cue_id: cues[1].id,
      time_ms: 1500,
      track: "Video",
      duration_ms: 750,
      loop_count: 2,
      jump_to_event_id: 1,
    },
    {
      id: 3,
      cue_id: cues[0].id,
      time_ms: 4200,
      track: "Lighting",
      duration_ms: 0,
      loop_count: 1,
      jump_to_event_id: null,
    },
  ];
  return { cues, events };
};

export const createTimelineSceneBlockLargeViewportFixture = (baseCue: CueSummary) => {
  const cues: CueSummary[] = Array.from({ length: 500 }, (_, index) => ({
    ...baseCue,
    id: baseCue.id + index,
    cue_number: String(index + 1),
    label: index === 499
      ? "Scale Cue 500 — Long Finale Source Name For Clip Boundary Validation"
      : `Scale Cue ${String(index + 1).padStart(3, "0")}`,
    fade_ms: 120 + (index % 8) * 80,
    effect_targets: [],
    node_graph_targets: [],
  }));
  const events: TimelineCueEventSummary[] = Array.from({ length: 500 }, (_, index) => ({
    id: index + 1,
    cue_id: index === 499 ? cues[499].id : cues[0].id,
    time_ms: 0,
    track: index % 2 === 0 ? "Lighting" : "Video",
    duration_ms: 1000,
    loop_count: index === 499 ? 32 : 256,
    jump_to_event_id: index % 7 === 0 ? ((index + 1) % 500) + 1 : null,
  }));
  return { cues, events };
};

export const createTimelineSceneBlockHourViewportFixture = (baseCue: CueSummary) => {
  const showDurationMs = 3_600_000;
  const cues: CueSummary[] = Array.from({ length: 500 }, (_, index) => ({
    ...baseCue,
    id: baseCue.id + index,
    cue_number: String(index + 1),
    label: `Hour Cue ${String(index + 1).padStart(3, "0")}`,
    fade_ms: 120 + (index % 8) * 80,
    effect_targets: [],
    node_graph_targets: [],
  }));
  const events: TimelineCueEventSummary[] = Array.from({ length: 500 }, (_, index) => ({
    id: index + 1,
    cue_id: cues[index].id,
    time_ms: Math.round((index * (showDurationMs - 1_000)) / 499),
    track: index % 2 === 0 ? "Lighting" : "Video",
    duration_ms: 1_000,
    loop_count: 1,
    jump_to_event_id: null,
  }));
  return { cues, events, showDurationMs };
};

export const timelineSceneBlockIterationCount = (event: Pick<TimelineCueEventSummary, "loop_count">) =>
  Math.round(clamp(finiteOr(event.loop_count ?? 1, 1), 1, 256));

export const timelineSceneBlockSpanMs = (
  event: Pick<TimelineCueEventSummary, "duration_ms" | "loop_count" | "conform_to_tempo">,
) => {
  const durationMs = Math.max(0, finiteOr(event.duration_ms ?? 0, 0));
  return event.conform_to_tempo ? durationMs : durationMs * timelineSceneBlockIterationCount(event);
};

export const timelineConformRateBadge = (
  event: Pick<TimelineCueEventSummary, "conform_to_tempo" | "rate">,
) => {
  if (
    !event.conform_to_tempo
    || event.rate === null
    || event.rate === undefined
    || !Number.isFinite(event.rate)
    || event.rate <= 0
  ) return null;
  return `[${event.rate.toFixed(2)}x]`;
};

export interface TimelineSceneBlockPlaybackTiming {
  authoredBeats?: number | null;
  bpm?: number | null;
}

export const timelineSceneBlockPlaybackStatus = (
  event: TimelineCueEventSummary,
  positionMs: number,
  executionLive: boolean,
  timing: TimelineSceneBlockPlaybackTiming = {},
) => {
  const startMs = Math.max(0, event.time_ms);
  const durationMs = Math.max(0, event.duration_ms ?? 0);
  const loopCount = timelineSceneBlockIterationCount(event);
  const spanMs = timelineSceneBlockSpanMs(event);
  const conformedIterationMs = event.conform_to_tempo
    && timing.authoredBeats !== null
    && timing.authoredBeats !== undefined
    && Number.isFinite(timing.authoredBeats)
    && timing.authoredBeats > 0
    && timing.bpm !== null
    && timing.bpm !== undefined
    && Number.isFinite(timing.bpm)
    && timing.bpm > 0
      ? timing.authoredBeats * 60_000 / timing.bpm
      : null;
  const iterationDurationMs = conformedIterationMs ?? durationMs;
  const underPlayhead = durationMs > 0
    ? positionMs >= startMs && positionMs < startMs + spanMs
    : positionMs === startMs;
  const elapsedMs = underPlayhead ? Math.max(0, positionMs - startMs) : 0;
  const iteration = durationMs > 0
    ? Math.min(loopCount, Math.floor(elapsedMs / Math.max(1, iterationDurationMs)) + 1)
    : 1;
  return {
    under_playhead: underPlayhead,
    live: executionLive && underPlayhead,
    iteration,
    iteration_count: loopCount,
    remaining_ms: underPlayhead ? Math.max(0, startMs + spanMs - positionMs) : 0,
    jump_to_event_id: event.jump_to_event_id ?? null,
  };
};

export const timelineSceneBlockViewportDurationMs = (contentEndMs: number, minimumDurationMs = 5000) => {
  const safeContentEndMs = Math.max(0, finiteOr(contentEndMs, 0));
  const headroomMs = Math.min(60_000, Math.max(1000, Math.ceil(safeContentEndMs * 0.1)));
  return Math.max(1, Math.round(minimumDurationMs), Math.ceil(safeContentEndMs + headroomMs));
};

export const growTimelineSceneBlockViewportDurationMs = (
  currentDurationMs: number,
  contentEndMs: number,
) => contentEndMs > currentDurationMs
  ? timelineSceneBlockViewportDurationMs(contentEndMs, currentDurationMs)
  : currentDurationMs;

export const timelinePlacementDisplayEndMs = (
  event: Pick<TimelineCueEventSummary, "time_ms" | "duration_ms" | "loop_count" | "conform_to_tempo">,
) => event.time_ms + (event.duration_ms > 0 ? timelineSceneBlockSpanMs(event) : 1000);

export const normalizeTimelineEventDraft = (
  event: TimelineCueEventSummary,
  patch: Partial<TimelineEventDraft>,
  snapTimeMs: (timeMs: number) => number,
  hasEventId: (eventId: number) => boolean,
  snappedTimeBeats: (timeMs: number) => number | null | undefined = () => undefined,
  timeBeatsAtCurrentBpm: (timeMs: number) => number | null = () => null,
): TimelineEventDraft => {
  const source: TimelineEventDraft = {
    cue_id: event.cue_id,
    time_ms: event.time_ms,
    time_beats: event.time_beats ?? null,
    track: event.track,
    layer_id: event.layer_id ?? null,
    duration_ms: event.duration_ms ?? 0,
    duration_beats: event.duration_beats ?? null,
    conform_to_tempo: event.conform_to_tempo ?? false,
    loop_fill: event.loop_fill ?? false,
    source_offset_ms: event.source_offset_ms ?? 0,
    fade_in_ms: event.fade_in_ms ?? 0,
    fade_out_ms: event.fade_out_ms ?? 0,
    loop_count: event.loop_count ?? 1,
    jump_to_event_id: event.jump_to_event_id ?? null,
    ...patch,
  };
  const minimumDurationMs = (event.duration_ms ?? 0) > 0 ? 1 : 0;
  const durationMs = Math.max(minimumDurationMs, Math.round(finiteOr(source.duration_ms, minimumDurationMs)));
  const loopCount = durationMs > 0
    ? Math.round(clamp(finiteOr(source.loop_count, 1), 1, 256))
    : 1;
  const requestedJumpId = source.jump_to_event_id === null
    ? null
    : Math.max(0, Math.round(source.jump_to_event_id));
  const jumpToEventId = durationMs > 0 && requestedJumpId !== null && hasEventId(requestedJumpId)
    ? requestedJumpId
    : null;
  const timeMs = snapTimeMs(source.time_ms);
  const snappedBeats = snappedTimeBeats(timeMs);
  const conformToTempo = durationMs > 0 && source.conform_to_tempo;
  const timeBeats = snappedBeats !== undefined
    ? snappedBeats
    : timeMs !== event.time_ms && conformToTempo
      ? timeBeatsAtCurrentBpm(timeMs)
      : timeMs === event.time_ms
      ? (source.time_beats !== null && Number.isFinite(source.time_beats) ? source.time_beats : null)
      : null;
  return {
    cue_id: Math.max(0, Math.round(finiteOr(source.cue_id, event.cue_id))),
    time_ms: timeMs,
    time_beats: timeBeats,
    track: source.track,
    layer_id: source.layer_id === null || Number.isFinite(source.layer_id)
      ? source.layer_id
      : event.layer_id ?? null,
    duration_ms: durationMs,
    duration_beats: source.duration_beats !== null && Number.isFinite(source.duration_beats)
      ? Math.max(0, source.duration_beats)
      : null,
    conform_to_tempo: conformToTempo,
    loop_fill: conformToTempo && source.loop_fill,
    source_offset_ms: durationMs > 0
      ? Math.round(finiteOr(source.source_offset_ms, 0))
      : 0,
    fade_in_ms: durationMs > 0 ? Math.round(clamp(finiteOr(source.fade_in_ms, 0), 0, durationMs)) : 0,
    fade_out_ms: durationMs > 0 ? Math.round(clamp(finiteOr(source.fade_out_ms, 0), 0, durationMs)) : 0,
    loop_count: loopCount,
    jump_to_event_id: jumpToEventId,
  };
};

export const normalizeTimelineSceneBlockAddDraft = (
  draft: TimelineSceneBlockAddDraft,
  snapTimeMs: (timeMs: number) => number,
  hasEventId: (eventId: number) => boolean,
  snappedTimeBeats: (timeMs: number) => number | null | undefined = () => undefined,
): TimelineSceneBlockAddDraft => {
  const requestedJumpId = draft.jump_to_event_id === null
    ? null
    : Math.max(0, Math.round(draft.jump_to_event_id));
  const timeMs = snapTimeMs(draft.time_ms);
  return {
    cue_id: Math.max(0, Math.round(finiteOr(draft.cue_id, 0))),
    time_ms: timeMs,
    time_beats: snappedTimeBeats(timeMs) ?? null,
    track: draft.track,
    layer_id: draft.layer_id === null || Number.isFinite(draft.layer_id) ? draft.layer_id : null,
    duration_ms: Math.max(1, Math.round(finiteOr(draft.duration_ms, 1000))),
    duration_beats: draft.duration_beats !== null && Number.isFinite(draft.duration_beats)
      ? Math.max(0, draft.duration_beats)
      : null,
    conform_to_tempo: draft.conform_to_tempo,
    loop_fill: draft.conform_to_tempo && draft.loop_fill,
    source_offset_ms: Math.round(finiteOr(draft.source_offset_ms, 0)),
    fade_in_ms: Math.round(clamp(finiteOr(draft.fade_in_ms, 0), 0, Math.max(1, draft.duration_ms))),
    fade_out_ms: Math.round(clamp(finiteOr(draft.fade_out_ms, 0), 0, Math.max(1, draft.duration_ms))),
    loop_count: Math.round(clamp(finiteOr(draft.loop_count, 1), 1, 256)),
    jump_to_event_id: requestedJumpId !== null && hasEventId(requestedJumpId) ? requestedJumpId : null,
  };
};

export const buildTimelineSceneBlockSnapPlacements = (
  events: TimelineCueEventSummary[],
  getDraft: (event: TimelineCueEventSummary) => TimelineEventDraft,
  snapTimeMs: (timeMs: number) => number,
  hasEventId: (eventId: number) => boolean,
  snappedTimeBeats: (timeMs: number) => number | null | undefined = () => undefined,
) => events.map((event) => {
  const currentDraft = getDraft(event);
  const normalizedDraft = normalizeTimelineEventDraft(
    event,
    { ...currentDraft, time_ms: snapTimeMs(currentDraft.time_ms) },
    snapTimeMs,
    hasEventId,
    snappedTimeBeats,
  );
  // Bulk Snap retains beat-domain intent only for Beat/Bar. Grid and Off are
  // millisecond-domain commits and must explicitly clear any older beat intent.
  const draft = {
    ...normalizedDraft,
    time_beats: snappedTimeBeats(normalizedDraft.time_ms) ?? null,
  };
  const request: TimelineSceneBlockPlacementUpdate = {
    event_id: event.id,
    cue_id: draft.cue_id,
    time_ms: draft.time_ms,
    time_beats: draft.time_beats,
    track: draft.track,
    layer_id: draft.layer_id,
    duration_ms: draft.duration_ms,
    duration_beats: draft.duration_beats,
    conform_to_tempo: draft.conform_to_tempo,
    loop_fill: draft.loop_fill,
    fade_in_ms: draft.fade_in_ms,
    fade_out_ms: draft.fade_out_ms,
    loop_count: draft.loop_count,
    jump_to_event_id: draft.jump_to_event_id,
  };
  return { event, draft, request };
});

export const createTimelineSceneBlockController = (options: TimelineSceneBlockControllerOptions) => {
  const add = async (draft: TimelineSceneBlockAddDraft, snapEnabled = true) => {
    const next = normalizeTimelineSceneBlockAddDraft(
      draft,
      snapEnabled ? options.snapTimeMs : (timeMs) => Math.max(0, Math.round(timeMs)),
      options.hasEventId,
      snapEnabled ? options.snappedTimeBeats : () => undefined,
    );
    const eventId = await options.invoke<number>("add_timeline_scene_block", {
      cueId: next.cue_id,
      timeMs: next.time_ms,
      timeBeats: next.time_beats,
      track: next.track,
      layerId: next.layer_id,
      durationMs: next.duration_ms,
      durationBeats: next.duration_beats,
      conformToTempo: next.conform_to_tempo,
      loopFill: next.loop_fill,
      sourceOffsetMs: next.source_offset_ms,
      fadeInMs: next.fade_in_ms,
      fadeOutMs: next.fade_out_ms,
      loopCount: next.loop_count,
      jumpToEventId: next.jump_to_event_id,
    });
    return { eventId, draft: next };
  };

  const set = async (
    event: TimelineCueEventSummary,
    patch: Partial<TimelineEventDraft> = {},
    snapEnabled = true,
  ) => {
    const next = normalizeTimelineEventDraft(
      event,
      patch,
      snapEnabled ? options.snapTimeMs : (timeMs) => Math.max(0, Math.round(timeMs)),
      options.hasEventId,
      snapEnabled ? options.snappedTimeBeats : () => undefined,
      options.timeBeatsAtCurrentBpm,
    );
    if (next.duration_ms > 0) {
      await options.invoke("set_timeline_scene_block", {
        eventId: event.id,
        cueId: next.cue_id,
        timeMs: next.time_ms,
        timeBeats: next.time_beats,
        track: next.track,
        layerId: next.layer_id,
        durationMs: next.duration_ms,
        durationBeats: next.duration_beats,
        conformToTempo: next.conform_to_tempo,
        loopFill: next.loop_fill,
        sourceOffsetMs: next.source_offset_ms,
        fadeInMs: next.fade_in_ms,
        fadeOutMs: next.fade_out_ms,
        loopCount: next.loop_count,
        jumpToEventId: next.jump_to_event_id,
      });
    } else {
      await options.invoke("set_timeline_cue_event", {
        eventId: event.id,
        cueId: next.cue_id,
        timeMs: next.time_ms,
        timeBeats: next.time_beats,
        track: next.track,
        layerId: next.layer_id,
      });
    }
    return next;
  };

  const removeCommand = async (eventId: number, event?: TimelineCueEventSummary) => {
    const isSceneBlock = (event?.duration_ms ?? 0) > 0;
    await options.invoke(isSceneBlock ? "remove_timeline_scene_block" : "remove_timeline_event", { eventId });
    return isSceneBlock;
  };

  /**
   * A gesture previews locally and reaches this helper once at pointer-up.
   * Root Timeline timing-only edits can use the authoritative Apply/history
   * lane; every non-provable edit remains on the existing direct route.
   */
  const commitTimingOnly = async (
    event: TimelineCueEventSummary,
    next: TimelineEventDraft,
    snapEnabled: boolean,
  ) => {
    if (await options.commitRootSceneBlockTiming?.(event, next) === true) return next;
    return set(event, next, snapEnabled);
  };

  const moveToPlacement = async (
    eventId: number,
    requestedTimeMs: number,
    layerId: number | null,
    snapEnabled = true,
  ) => {
    const event = options.getEventById(eventId);
    if (!event) {
      options.setMessage(`Timeline event ${eventId} was not found.`);
      return;
    }
    const requested = Math.max(0, Math.round(finiteOr(requestedTimeMs, event.time_ms)));
    const timeMs = snapEnabled ? options.snapTimeMs(requested) : requested;
    try {
      const next = await commitTimingOnly(event, normalizeTimelineEventDraft(
        event,
        {
        ...options.getEventDraft(event),
        time_ms: timeMs,
        layer_id: layerId,
        },
        snapEnabled ? options.snapTimeMs : (value) => Math.max(0, Math.round(value)),
        options.hasEventId,
        snapEnabled ? options.snappedTimeBeats : () => undefined,
        options.timeBeatsAtCurrentBpm,
      ), snapEnabled);
      options.setEventDraft(event.id, next);
      options.setMessage(`Moved Cue ${event.cue_id} to ${timeMs} ms on layer ${layerId ?? "legacy"}`);
      await options.refreshSnapshot();
    } catch (error) {
      options.setMessage(String(error));
    }
  };

  return {
    add,
    set,

    async addAt(
      cueId: number | null,
      timeMs: number,
      track: TimelineTrackKind,
      nextDraftTime = true,
      placement: {
        layerId?: number | null;
        durationMs?: number;
        authoredBeats?: number | null;
        stretchMode?: TimelineStretchMode;
        snapEnabled?: boolean;
        loopCount?: number;
      } = {},
    ) {
      if (cueId === null) {
        options.setMessage("Create a Cue before adding linked Scene Blocks.");
        return;
      }
      try {
        const durationMs = placement.durationMs ?? options.getAddDurationMs();
        const authoredBeats = placement.authoredBeats ?? null;
        const conformToTempo = authoredBeats !== null && authoredBeats > 0;
        const { eventId, draft } = await add({
          cue_id: cueId,
          time_ms: timeMs,
          time_beats: null,
          track,
          layer_id: placement.layerId ?? null,
          duration_ms: durationMs,
          duration_beats: conformToTempo ? durationMs * options.getBpm() / 60_000 : null,
          conform_to_tempo: conformToTempo,
          loop_fill: conformToTempo && placement.stretchMode === "WINDOW",
          source_offset_ms: 0,
          fade_in_ms: 0,
          fade_out_ms: 0,
          loop_count: placement.loopCount ?? options.getAddLoopCount(),
          jump_to_event_id: options.getAddJumpToEventId(),
        }, placement.snapEnabled ?? true);
        options.setNextStartMs(nextDraftTime
          ? options.snapTimeMs(draft.time_ms + timelineSceneBlockSpanMs(draft))
          : draft.time_ms);
        options.setMessage(
          draft.conform_to_tempo
            ? `Added linked Scene Block ${eventId} at ${draft.time_ms} ms (${draft.duration_ms} ms window · ${draft.loop_count} ${draft.loop_fill ? "fill loops" : "tempo iterations"})`
            : `Added linked Scene Block ${eventId} at ${draft.time_ms} ms (${draft.duration_ms} ms × ${draft.loop_count})`,
        );
        await options.refreshSnapshot();
      } catch (error) {
        options.setMessage(String(error));
      }
    },

    async save(event: TimelineCueEventSummary) {
      try {
        const next = await set(event, options.getEventDraft(event));
        options.setEventDraft(event.id, next);
        options.setMessage(next.duration_ms > 0
          ? `Saved linked Scene Block ${event.id}`
          : `Saved legacy timeline point ${event.id}`);
        await options.refreshSnapshot();
      } catch (error) {
        options.setMessage(String(error));
      }
    },

    async moveBy(event: TimelineCueEventSummary, deltaMs: number) {
      const currentDraft = options.getEventDraft(event);
      const timeMs = options.snapTimeMs(Math.max(0, currentDraft.time_ms + deltaMs));
      try {
        const next = await commitTimingOnly(event, normalizeTimelineEventDraft(
          event,
          { ...currentDraft, time_ms: timeMs },
          options.snapTimeMs,
          options.hasEventId,
          options.snappedTimeBeats,
          options.timeBeatsAtCurrentBpm,
        ), true);
        options.setEventDraft(event.id, next);
        options.setMessage(`Moved ${event.duration_ms > 0 ? "Scene Block" : "timeline point"} ${event.id} to ${timeMs} ms`);
        await options.refreshSnapshot();
      } catch (error) {
        options.setMessage(String(error));
      }
    },

    async moveToTime(eventId: number, requestedTimeMs: number) {
      const event = options.getEventById(eventId);
      await moveToPlacement(eventId, requestedTimeMs, event?.layer_id ?? null);
    },

    moveToPlacement,

    async resizeToTime(
      eventId: number,
      edge: "start" | "end",
      requestedTimeMs: number,
      mode: TimelineStretchMode = "WINDOW",
      snapEnabled = true,
    ) {
      const event = options.getEventById(eventId);
      if (!event || event.duration_ms <= 0) {
        options.setMessage(`Scene Block ${eventId} was not found.`);
        return;
      }
      const currentDraft = options.getEventDraft(event);
      const startMs = currentDraft.time_ms;
      const endMs = startMs + timelineSceneBlockSpanMs(currentDraft);
      const requested = Math.max(0, Math.round(finiteOr(requestedTimeMs, edge === "start" ? startMs : endMs)));
      const edgeMs = snapEnabled ? options.snapTimeMs(requested) : requested;
      const projection = projectTimelineBlockStretch({
        mode,
        edge,
        originalStartMs: startMs,
        originalEndMs: endMs,
        requestedEdgeMs: edgeMs,
        authoredBeats: options.getCueAuthoredBeats(currentDraft.cue_id),
        bpm: options.getBpm(),
      });
      try {
        const next = await commitTimingOnly(event, normalizeTimelineEventDraft(
          event,
          {
          ...currentDraft,
          time_ms: projection.start_ms,
          duration_ms: projection.duration_ms,
          duration_beats: projection.duration_beats,
          conform_to_tempo: projection.conform_to_tempo,
          loop_fill: projection.loop_fill,
          loop_count: projection.loop_count,
          },
          snapEnabled ? options.snapTimeMs : (value) => Math.max(0, Math.round(value)),
          options.hasEventId,
          snapEnabled ? options.snappedTimeBeats : () => undefined,
          options.timeBeatsAtCurrentBpm,
        ), snapEnabled);
        options.setEventDraft(event.id, next);
        options.setMessage(projection.fallback_to_window
          ? `Scene Block ${event.id} has no Authored beats; RATE stretch used WINDOW behavior (${projection.duration_ms} ms).`
          : `Stretched Scene Block ${event.id} in ${mode} mode to ${projection.duration_ms} ms${projection.rate ? ` [${projection.rate.toFixed(2)}x]` : ""}`);
        await options.refreshSnapshot();
      } catch (error) {
        options.setMessage(String(error));
      }
    },

    async setFade(eventId: number, edge: "in" | "out", requestedFadeMs: number, snapEnabled = true) {
      const event = options.getEventById(eventId);
      if (!event || event.duration_ms <= 0) {
        options.setMessage(`Scene Block ${eventId} was not found.`);
        return;
      }
      const currentDraft = options.getEventDraft(event);
      const windowMs = Math.max(1, currentDraft.duration_ms);
      const requested = Math.round(clamp(finiteOr(requestedFadeMs, 0), 0, windowMs));
      const fadeMs = snapEnabled
        ? Math.round(clamp(options.snapTimeMs(requested), 0, windowMs))
        : requested;
      try {
        const next = await commitTimingOnly(event, normalizeTimelineEventDraft(
          event,
          {
          ...currentDraft,
          [edge === "in" ? "fade_in_ms" : "fade_out_ms"]: fadeMs,
          },
          snapEnabled ? options.snapTimeMs : (value) => Math.max(0, Math.round(value)),
          options.hasEventId,
          snapEnabled ? options.snappedTimeBeats : () => undefined,
          options.timeBeatsAtCurrentBpm,
        ), snapEnabled);
        options.setEventDraft(event.id, next);
        options.setMessage(`Set Scene Block ${event.id} Fade ${edge === "in" ? "In" : "Out"} to ${fadeMs} ms`);
        await options.refreshSnapshot();
      } catch (error) {
        options.setMessage(String(error));
      }
    },

    async remove(eventOrId: TimelineCueEventSummary | number) {
      const eventId = typeof eventOrId === "number" ? eventOrId : eventOrId.id;
      const event = typeof eventOrId === "number" ? options.getEventById(eventId) : eventOrId;
      try {
        const removedSceneBlock = await removeCommand(eventId, event);
        options.setMessage(`Removed ${removedSceneBlock ? "Scene Block" : "timeline point"} ${eventId}`);
        await options.refreshSnapshot();
      } catch (error) {
        options.setMessage(String(error));
      }
    },
  };
};
