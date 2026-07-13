import type { TimelineEventDraft } from "./editorDrafts";
import type { ClockSource, CueSummary, TimelineCueEventSummary, TimelineTrackKind, VideoLayerSummary } from "./types";

export interface TimelineSceneBlockAddDraft {
  cue_id: number;
  time_ms: number;
  track: TimelineTrackKind;
  duration_ms: number;
  loop_count: number;
  jump_to_event_id: number | null;
}

export interface TimelineSceneBlockPlacementUpdate {
  event_id: number;
  cue_id: number;
  time_ms: number;
  track: TimelineTrackKind;
  duration_ms: number;
  loop_count: number;
  jump_to_event_id: number | null;
}

interface TimelineSceneBlockControllerOptions {
  invoke: <T = unknown>(command: string, args?: Record<string, unknown>) => Promise<T>;
  snapTimeMs: (timeMs: number) => number;
  hasEventId: (eventId: number) => boolean;
  getEventById: (eventId: number) => TimelineCueEventSummary | undefined;
  getEventDraft: (event: TimelineCueEventSummary) => TimelineEventDraft;
  setEventDraft: (eventId: number, draft: TimelineEventDraft) => void;
  getAddDurationMs: () => number;
  getAddLoopCount: () => number;
  getAddJumpToEventId: () => number | null;
  setNextStartMs: (timeMs: number) => void;
  getOverviewDurationMs: () => number;
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
  track: event.track,
  duration_ms: event.duration_ms ?? 0,
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

export const buildTimelineSceneBlockCueOptions = (cues: CueSummary[]) => cues.map((cue) => ({
  id: cue.id,
  cue_list_id: cue.cue_list_id,
  cue_number: cue.cue_number || String(cue.id),
  label: cue.label,
  source_summary: timelineSceneBlockSourceSummary(cue),
}));

export const timelineSceneBlockSourcePickerOptions = <T extends {
  id: number;
  cue_list_id: number;
  cue_number: string;
  label: string;
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
    option.source_summary === candidate.source_summary;
});

export const buildTimelineSceneBlockRows = (events: TimelineCueEventSummary[], cues: CueSummary[]) => {
  const cuesById = new Map(cues.map((cue) => [cue.id, cue]));
  return events.map((event) => {
    const cue = cuesById.get(event.cue_id);
    return {
      ...event,
      duration_ms: event.duration_ms ?? 0,
      loop_count: event.loop_count ?? 1,
      jump_to_event_id: event.jump_to_event_id ?? null,
      cue_number: cue?.cue_number || String(event.cue_id),
      cue_label: cue?.label ?? `Cue ${event.cue_id}`,
      cue_list_id: cue?.cue_list_id ?? 0,
      source_summary: timelineSceneBlockSourceSummary(cue),
    };
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
    event.track === candidate.track &&
    event.duration_ms === candidate.duration_ms &&
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
  draft.track === event.track &&
  draft.duration_ms === (event.duration_ms ?? 0) &&
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

export const timelineSceneBlockIterationCount = (event: Pick<TimelineCueEventSummary, "loop_count">) =>
  Math.round(clamp(finiteOr(event.loop_count ?? 1, 1), 1, 256));

export const timelineSceneBlockSpanMs = (
  event: Pick<TimelineCueEventSummary, "duration_ms" | "loop_count">,
) => Math.max(0, finiteOr(event.duration_ms ?? 0, 0)) * timelineSceneBlockIterationCount(event);

export const timelineSceneBlockPlaybackStatus = (
  event: TimelineCueEventSummary,
  positionMs: number,
  executionLive: boolean,
) => {
  const startMs = Math.max(0, event.time_ms);
  const durationMs = Math.max(0, event.duration_ms ?? 0);
  const loopCount = timelineSceneBlockIterationCount(event);
  const spanMs = durationMs * loopCount;
  const underPlayhead = durationMs > 0
    ? positionMs >= startMs && positionMs < startMs + spanMs
    : positionMs === startMs;
  const elapsedMs = underPlayhead ? Math.max(0, positionMs - startMs) : 0;
  const iteration = durationMs > 0
    ? Math.min(loopCount, Math.floor(elapsedMs / Math.max(1, durationMs)) + 1)
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
  event: Pick<TimelineCueEventSummary, "time_ms" | "duration_ms" | "loop_count">,
) => event.time_ms + (event.duration_ms > 0 ? timelineSceneBlockSpanMs(event) : 1000);

export const normalizeTimelineEventDraft = (
  event: TimelineCueEventSummary,
  patch: Partial<TimelineEventDraft>,
  snapTimeMs: (timeMs: number) => number,
  hasEventId: (eventId: number) => boolean,
): TimelineEventDraft => {
  const source: TimelineEventDraft = {
    cue_id: event.cue_id,
    time_ms: event.time_ms,
    track: event.track,
    duration_ms: event.duration_ms ?? 0,
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
  return {
    cue_id: Math.max(0, Math.round(finiteOr(source.cue_id, event.cue_id))),
    time_ms: snapTimeMs(source.time_ms),
    track: source.track,
    duration_ms: durationMs,
    loop_count: loopCount,
    jump_to_event_id: jumpToEventId,
  };
};

export const normalizeTimelineSceneBlockAddDraft = (
  draft: TimelineSceneBlockAddDraft,
  snapTimeMs: (timeMs: number) => number,
  hasEventId: (eventId: number) => boolean,
): TimelineSceneBlockAddDraft => {
  const requestedJumpId = draft.jump_to_event_id === null
    ? null
    : Math.max(0, Math.round(draft.jump_to_event_id));
  return {
    cue_id: Math.max(0, Math.round(finiteOr(draft.cue_id, 0))),
    time_ms: snapTimeMs(draft.time_ms),
    track: draft.track,
    duration_ms: Math.max(1, Math.round(finiteOr(draft.duration_ms, 1000))),
    loop_count: Math.round(clamp(finiteOr(draft.loop_count, 1), 1, 256)),
    jump_to_event_id: requestedJumpId !== null && hasEventId(requestedJumpId) ? requestedJumpId : null,
  };
};

export const buildTimelineSceneBlockSnapPlacements = (
  events: TimelineCueEventSummary[],
  getDraft: (event: TimelineCueEventSummary) => TimelineEventDraft,
  snapTimeMs: (timeMs: number) => number,
  hasEventId: (eventId: number) => boolean,
) => events.map((event) => {
  const currentDraft = getDraft(event);
  const draft = normalizeTimelineEventDraft(
    event,
    { ...currentDraft, time_ms: snapTimeMs(currentDraft.time_ms) },
    snapTimeMs,
    hasEventId,
  );
  const request: TimelineSceneBlockPlacementUpdate = {
    event_id: event.id,
    cue_id: draft.cue_id,
    time_ms: draft.time_ms,
    track: draft.track,
    duration_ms: draft.duration_ms,
    loop_count: draft.loop_count,
    jump_to_event_id: draft.jump_to_event_id,
  };
  return { event, draft, request };
});

export const createTimelineSceneBlockController = (options: TimelineSceneBlockControllerOptions) => {
  const add = async (draft: TimelineSceneBlockAddDraft) => {
    const next = normalizeTimelineSceneBlockAddDraft(draft, options.snapTimeMs, options.hasEventId);
    const eventId = await options.invoke<number>("add_timeline_scene_block", {
      cueId: next.cue_id,
      timeMs: next.time_ms,
      track: next.track,
      durationMs: next.duration_ms,
      loopCount: next.loop_count,
      jumpToEventId: next.jump_to_event_id,
    });
    return { eventId, draft: next };
  };

  const set = async (event: TimelineCueEventSummary, patch: Partial<TimelineEventDraft> = {}) => {
    const next = normalizeTimelineEventDraft(event, patch, options.snapTimeMs, options.hasEventId);
    if (next.duration_ms > 0) {
      await options.invoke("set_timeline_scene_block", {
        eventId: event.id,
        cueId: next.cue_id,
        timeMs: next.time_ms,
        track: next.track,
        durationMs: next.duration_ms,
        loopCount: next.loop_count,
        jumpToEventId: next.jump_to_event_id,
      });
    } else {
      await options.invoke("set_timeline_cue_event", {
        eventId: event.id,
        cueId: next.cue_id,
        timeMs: next.time_ms,
        track: next.track,
      });
    }
    return next;
  };

  const removeCommand = async (eventId: number, event?: TimelineCueEventSummary) => {
    const isSceneBlock = (event?.duration_ms ?? 0) > 0;
    await options.invoke(isSceneBlock ? "remove_timeline_scene_block" : "remove_timeline_event", { eventId });
    return isSceneBlock;
  };

  return {
    add,
    set,

    async addAt(
      cueId: number | null,
      timeMs: number,
      track: TimelineTrackKind,
      nextDraftTime = true,
    ) {
      if (cueId === null) {
        options.setMessage("Create a Cue before adding linked Scene Blocks.");
        return;
      }
      try {
        const { eventId, draft } = await add({
          cue_id: cueId,
          time_ms: timeMs,
          track,
          duration_ms: options.getAddDurationMs(),
          loop_count: options.getAddLoopCount(),
          jump_to_event_id: options.getAddJumpToEventId(),
        });
        options.setNextStartMs(nextDraftTime
          ? options.snapTimeMs(draft.time_ms + draft.duration_ms * draft.loop_count)
          : draft.time_ms);
        options.setMessage(
          `Added linked Scene Block ${eventId} at ${draft.time_ms} ms (${draft.duration_ms} ms × ${draft.loop_count})`,
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
        const next = await set(event, { ...currentDraft, time_ms: timeMs });
        options.setEventDraft(event.id, next);
        options.setMessage(`Moved ${event.duration_ms > 0 ? "Scene Block" : "timeline point"} ${event.id} to ${timeMs} ms`);
        await options.refreshSnapshot();
      } catch (error) {
        options.setMessage(String(error));
      }
    },

    async moveToRatio(eventId: number, ratio: number) {
      const event = options.getEventById(eventId);
      if (!event) {
        options.setMessage(`Timeline event ${eventId} was not found.`);
        return;
      }
      const timeMs = options.snapTimeMs(
        Math.round(clamp(ratio, 0, 1) * options.getOverviewDurationMs()),
      );
      try {
        const next = await set(event, { ...options.getEventDraft(event), time_ms: timeMs });
        options.setEventDraft(event.id, next);
        options.setMessage(`Moved Cue ${event.cue_id} to ${timeMs} ms`);
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
