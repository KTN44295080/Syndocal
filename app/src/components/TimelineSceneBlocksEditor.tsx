import { createEffect, createMemo, createSignal, For, onCleanup, Show } from "solid-js";
import type { TimelineEventDraft } from "../editorDrafts";
import { confirmTimelinePlacementRemoval } from "../destructiveActions";
import { cueIdentityCss } from "../identityColor";
import {
  reconcileTimelineSceneBlockJumpTarget,
  reconcileTimelineSceneBlockPickerTarget,
  timelineConformRateBadge,
  timelineSceneBlockSpanMs,
  timelineSceneBlockPlaybackStatus,
  timelineEventDraftMatchesSummary,
  timelineSceneBlockSourcePickerOptions,
} from "../timelineSceneBlocks";
import type { TimelineCueEventSummary, TimelineTrackKind } from "../types";
import { formatTimelineTimeInput, parseTimelineTimeInput } from "../timelineTimeInput";

export interface TimelineSceneBlockCueOption {
  id: number;
  cue_list_id: number;
  cue_number: string;
  label: string;
  authored_beats: number | null;
  step_count: number;
  source_summary: string;
}

export interface TimelineSceneBlockRow extends TimelineCueEventSummary {
  cue_list_id: number;
  cue_number: string;
  cue_label: string;
  source_summary: string;
}

interface TimelineSceneBlocksEditorProps {
  inspectorOnly?: boolean;
  positionMs: number;
  bpm: number;
  cueColors?: Record<number, string>;
  executionLive: boolean;
  selectedCueId: number | null;
  startMs: number;
  durationMs: number;
  loopCount: number;
  track: TimelineTrackKind;
  jumpToEventId: number | null;
  cueOptions: TimelineSceneBlockCueOption[];
  eventRows: TimelineSceneBlockRow[];
  filterEventIds: number[] | null;
  filterLabel: string | null;
  selectedEventId: number | null;
  selectionRevision: number;
  timelineEventDraft: (event: TimelineCueEventSummary) => TimelineEventDraft;
  onSelectedCueId: (cueId: number) => void;
  onStartMs: (timeMs: number) => void;
  onDurationMs: (durationMs: number) => void;
  onLoopCount: (loopCount: number) => void;
  onTrack: (track: TimelineTrackKind) => void;
  onJumpToEventId: (eventId: number | null) => void;
  onAddBlock: () => void | Promise<void>;
  onAddBlockAtPlayhead: () => void | Promise<void>;
  onUpdateEventDraft: (event: TimelineCueEventSummary, patch: Partial<TimelineEventDraft>) => void;
  onSaveEvent: (event: TimelineCueEventSummary) => void | Promise<void>;
  onRemoveEvent: (event: TimelineCueEventSummary) => void | Promise<void>;
  onSelectEvent: (eventId: number) => void;
  onClearEventFilter: () => void;
  onOpenSourceCue: (cueId: number) => void;
  armedCueId: number | null;
  onArmCue: (cueId: number | null) => void;
}

const totalDurationMs = (
  event: Pick<TimelineCueEventSummary, "duration_ms" | "loop_count" | "conform_to_tempo">,
) => timelineSceneBlockSpanMs(event);
const sceneBlockRowsPerPage = 12;
const sourcePickerResultLimit = 80;
const selectedRowScrollAttemptLimit = 8;

const jumpOptionLabel = (event: TimelineSceneBlockRow) =>
  `${event.duration_ms > 0 ? "Block" : "Point"} ${event.id} · L${event.cue_list_id} / Cue #${event.cue_id} · ${event.cue_number} ${event.cue_label} @ ${event.time_ms}ms`;

const sourceCueOptionLabel = (cue: TimelineSceneBlockCueOption) =>
  `L${cue.cue_list_id} · Cue #${cue.id} · ${cue.cue_number} · ${cue.label}`;

const searchTokensMatch = (searchable: string, query: string) =>
  query.split(/\s+/).filter(Boolean).every((token) => searchable.includes(token));

export function TimelineSceneBlocksEditor(props: TimelineSceneBlocksEditorProps) {
  const [page, setPage] = createSignal(0);
  const [rowQuery, setRowQuery] = createSignal("");
  // T3: the Finder + fixed inspector is the primary editing surface; the
  // legacy paged row editor stays reachable behind the List toggle.
  const [editorView, setEditorView] = createSignal<"finder" | "list">("finder");
  const [jumpPickerEventId, setJumpPickerEventId] = createSignal<number | null>(null);
  const [jumpPickerTargetId, setJumpPickerTargetId] = createSignal<number | null>(null);
  const [jumpPickerQuery, setJumpPickerQuery] = createSignal("");
  const [sourcePickerEventId, setSourcePickerEventId] = createSignal<number | null>(null);
  const [sourcePickerCueId, setSourcePickerCueId] = createSignal<number | null>(null);
  const [sourcePickerQuery, setSourcePickerQuery] = createSignal("");
  let workspaceElement: HTMLElement | undefined;
  let listElement: HTMLDivElement | undefined;
  const scrollRowIntoView = (eventId: number) => {
    requestAnimationFrame(() => {
      listElement
        ?.querySelector<HTMLElement>(`[data-scene-block-id="${eventId}"]`)
        ?.scrollIntoView({ block: "nearest" });
    });
  };
  let selectedRowScrollTargetId: number | null = null;
  let selectedRowScrollFrame: number | null = null;
  let selectedRowScrollAttempts = 0;
  let lastSelectedRowScrollTargetId: number | null = null;
  let lastSelectedRowScrollTargetRevision = -1;
  const cancelSelectedRowScroll = () => {
    if (selectedRowScrollFrame !== null) cancelAnimationFrame(selectedRowScrollFrame);
    selectedRowScrollTargetId = null;
    selectedRowScrollFrame = null;
    selectedRowScrollAttempts = 0;
  };
  const selectedRowIsFullyVisible = (row: HTMLElement) => {
    if (!listElement || !workspaceElement) return false;
    const rowRect = row.getBoundingClientRect();
    const listRect = listElement.getBoundingClientRect();
    const workspaceRect = workspaceElement.getBoundingClientRect();
    return rowRect.left >= Math.max(0, listRect.left, workspaceRect.left) - 1 &&
      rowRect.top >= Math.max(0, listRect.top, workspaceRect.top) - 1 &&
      rowRect.right <= Math.min(window.innerWidth, listRect.right, workspaceRect.right) + 1 &&
      rowRect.bottom <= Math.min(window.innerHeight, listRect.bottom, workspaceRect.bottom) + 1;
  };
  const scheduleSelectedRowScroll = (eventId: number, selectionRevision: number) => {
    if (selectedRowScrollTargetId === eventId) {
      lastSelectedRowScrollTargetRevision = selectionRevision;
      return;
    }
    if (
      lastSelectedRowScrollTargetId === eventId &&
      lastSelectedRowScrollTargetRevision === selectionRevision
    ) return;
    cancelSelectedRowScroll();
    selectedRowScrollTargetId = eventId;
    selectedRowScrollAttempts = 0;
    lastSelectedRowScrollTargetId = eventId;
    lastSelectedRowScrollTargetRevision = selectionRevision;
    const attemptScroll = () => {
      selectedRowScrollFrame = null;
      if (selectedRowScrollTargetId !== eventId || props.selectedEventId !== eventId) {
        if (selectedRowScrollTargetId === eventId) cancelSelectedRowScroll();
        return;
      }
      const row = listElement
        ?.querySelector<HTMLElement>(`[data-scene-block-id="${eventId}"]`);
      if (!row) {
        selectedRowScrollAttempts += 1;
        if (selectedRowScrollAttempts >= selectedRowScrollAttemptLimit) {
          cancelSelectedRowScroll();
          return;
        }
        selectedRowScrollFrame = requestAnimationFrame(attemptScroll);
        return;
      }
      if (!selectedRowIsFullyVisible(row)) row.scrollIntoView({ block: "nearest" });
      cancelSelectedRowScroll();
    };
    selectedRowScrollFrame = requestAnimationFrame(attemptScroll);
  };
  onCleanup(cancelSelectedRowScroll);
  const blockCount = () => props.eventRows.filter((event) => event.duration_ms > 0).length;
  const pointCount = () => props.eventRows.length - blockCount();
  const sourceIndexByEventId = createMemo(() => new Map(
    props.eventRows.map((event, index) => [event.id, index]),
  ));
  const cueOptionById = createMemo(() => new Map(props.cueOptions.map((cue) => [cue.id, cue])));
  const playbackTimingForEvent = (event: TimelineCueEventSummary) => ({
    authoredBeats: cueOptionById().get(event.cue_id)?.authored_beats ?? null,
    bpm: props.bpm,
  });
  const filterEventIdSet = createMemo(() => props.filterEventIds === null
    ? null
    : new Set(props.filterEventIds));
  const filteredRows = createMemo(() => {
    const filterIds = filterEventIdSet();
    const scopedRows = filterIds === null
      ? props.eventRows
      : props.eventRows.filter((event) => filterIds.has(event.id));
    const query = rowQuery().trim().toLocaleLowerCase();
    if (!query) return scopedRows;
    if (/^\d+$/.test(query)) {
      const numericQuery = Number(query);
      return scopedRows.filter((event) =>
        event.id === numericQuery || event.cue_id === numericQuery || event.cue_number === query || event.time_ms === numericQuery,
      );
    }
    return scopedRows.filter((event) => searchTokensMatch(
      `#${event.id} list ${event.cue_list_id} l${event.cue_list_id} cue ${event.cue_id} ${event.cue_number} ${event.cue_label} ${event.time_ms}`
        .toLocaleLowerCase(),
      query,
    ));
  });
  const filteredIndexByEventId = createMemo(() => new Map(
    filteredRows().map((event, index) => [event.id, index]),
  ));
  const pageCount = createMemo(() => Math.max(1, Math.ceil(filteredRows().length / sceneBlockRowsPerPage)));
  const visibleRows = createMemo(() => {
    const start = page() * sceneBlockRowsPerPage;
    return filteredRows().slice(start, start + sceneBlockRowsPerPage);
  });
  const pageRangeLabel = createMemo(() => {
    if (filteredRows().length === 0) return `0 / 0 · ${props.eventRows.length} total`;
    const start = page() * sceneBlockRowsPerPage + 1;
    const end = Math.min(filteredRows().length, start + sceneBlockRowsPerPage - 1);
    return `${start}-${end} / ${filteredRows().length} · ${props.eventRows.length} total`;
  });
  const jumpPickerEvent = createMemo(() =>
    props.eventRows.find((event) => event.id === jumpPickerEventId()) ?? null,
  );
  const jumpPickerOpen = () => jumpPickerEventId() === 0 || Boolean(jumpPickerEvent());
  const sourcePickerEvent = createMemo(() =>
    props.eventRows.find((event) => event.id === sourcePickerEventId()) ?? null,
  );
  const sourcePickerOpen = () => sourcePickerEventId() === 0 || Boolean(sourcePickerEvent());
  const matchingSourceOptions = createMemo(() => timelineSceneBlockSourcePickerOptions(
    props.cueOptions,
    sourcePickerQuery(),
    sourcePickerCueId(),
    sourcePickerResultLimit,
  ));
  const composerSourceCue = createMemo(() =>
    props.cueOptions.find((cue) => cue.id === props.selectedCueId) ?? null,
  );
  const composerJumpTarget = createMemo(() =>
    props.eventRows.find((event) => event.id === props.jumpToEventId) ?? null,
  );
  const beatDurationMs = () => 60_000 / Math.max(1, Number.isFinite(props.bpm) ? props.bpm : 120);
  const millisecondsToBeats = (timeMs: number) => Number((Math.max(0, timeMs) / beatDurationMs()).toFixed(6));
  const matchingJumpRows = createMemo(() => {
    const query = jumpPickerQuery().trim().toLocaleLowerCase();
    const filtered = !query
      ? props.eventRows
      : /^\d+$/.test(query)
        ? props.eventRows.filter((event) =>
            event.id === Number(query) || event.cue_id === Number(query) || event.cue_number === query,
          )
        : props.eventRows.filter((event) => searchTokensMatch(
            `block ${event.id} list ${event.cue_list_id} l${event.cue_list_id} cue ${event.cue_id} ${event.cue_number} ${event.cue_label} ${event.time_ms}`
              .toLocaleLowerCase(),
            query,
          ));
    const visible = filtered.slice(0, sourcePickerResultLimit);
    if (query || jumpPickerTargetId() === null || visible.some((event) => event.id === jumpPickerTargetId())) {
      return visible;
    }
    const selected = props.eventRows.find((event) => event.id === jumpPickerTargetId());
    return selected
      ? [selected, ...visible.filter((event) => event.id !== selected.id)].slice(0, sourcePickerResultLimit)
      : visible;
  });

  createEffect(() => {
    if (page() >= pageCount()) setPage(pageCount() - 1);
  });
  let lastSyncedSelectionRevision = -1;
  let lastSyncedSelectedIndex = -1;
  let lastSyncedFilterEventIds: number[] | null | undefined;
  let selectedPageSyncRevision = 0;
  createEffect(() => {
    const selectedEventId = props.selectedEventId;
    const selectionRevision = props.selectionRevision;
    const filterEventIds = props.filterEventIds;
    const filterChanged = filterEventIds !== lastSyncedFilterEventIds;
    const sourceIndex = selectedEventId === null
      ? -1
      : (sourceIndexByEventId().get(selectedEventId) ?? -1);
    if (selectedEventId === null) {
      cancelSelectedRowScroll();
      lastSelectedRowScrollTargetId = null;
      lastSelectedRowScrollTargetRevision = -1;
      return;
    }
    if (
      sourceIndex < 0 ||
      (!filterChanged && selectionRevision === lastSyncedSelectionRevision && sourceIndex === lastSyncedSelectedIndex)
    ) return;
    lastSyncedSelectionRevision = selectionRevision;
    lastSyncedSelectedIndex = sourceIndex;
    lastSyncedFilterEventIds = filterEventIds;
    const filteredIndex = filteredIndexByEventId().get(selectedEventId) ?? -1;
    if (filteredIndex >= 0) {
      const revision = ++selectedPageSyncRevision;
      setPage(Math.floor(filteredIndex / sceneBlockRowsPerPage));
      scheduleSelectedRowScroll(selectedEventId, revision);
      return;
    }
    if (filterChanged && filterEventIdSet() !== null) {
      ++selectedPageSyncRevision;
      setRowQuery("");
      setPage(0);
      return;
    }
    const revision = ++selectedPageSyncRevision;
    if (filterEventIdSet() !== null) props.onClearEventFilter();
    setRowQuery("");
    queueMicrotask(() => {
      if (revision === selectedPageSyncRevision) {
        setPage(Math.floor(sourceIndex / sceneBlockRowsPerPage));
        scheduleSelectedRowScroll(selectedEventId, revision);
      }
    });
  });
  const liveFollowEvent = createMemo(() => {
    if (!props.executionLive) return null;
    const liveEvents = props.eventRows.filter((event) =>
      timelineSceneBlockPlaybackStatus(event, props.positionMs, true, playbackTimingForEvent(event)).live,
    );
    return liveEvents.at(-1) ?? null;
  });
  let lastFollowedLiveEventId: number | null = null;
  createEffect(() => {
    const liveEvent = liveFollowEvent();
    if (!liveEvent) {
      lastFollowedLiveEventId = null;
      return;
    }
    if (
      rowQuery().trim() ||
      props.filterEventIds !== null ||
      props.eventRows.some((event) => !timelineEventDraftMatchesSummary(event, props.timelineEventDraft(event)))
    ) return;
    if (lastFollowedLiveEventId === liveEvent.id) return;
    lastFollowedLiveEventId = liveEvent.id;
    const index = sourceIndexByEventId().get(liveEvent.id) ?? -1;
    if (index < 0) return;
    setPage(Math.floor(index / sceneBlockRowsPerPage));
    queueMicrotask(() => scrollRowIntoView(liveEvent.id));
  });
  createEffect(() => {
    if (jumpPickerEventId() !== null && jumpPickerEventId() !== 0 && !jumpPickerEvent()) {
      setJumpPickerEventId(null);
      setJumpPickerTargetId(null);
      setJumpPickerQuery("");
    }
  });
  createEffect(() => {
    const pickerOwnerId = jumpPickerEventId();
    if (pickerOwnerId === null) return;
    const reconciledTarget = reconcileTimelineSceneBlockPickerTarget(
      props.eventRows.map((candidate) => candidate.id),
      pickerOwnerId === 0 ? null : pickerOwnerId,
      jumpPickerTargetId(),
    );
    if (reconciledTarget !== jumpPickerTargetId()) {
      setJumpPickerTargetId(reconciledTarget);
    }
  });
  createEffect(() => {
    if (sourcePickerEventId() !== null && sourcePickerEventId() !== 0 && !sourcePickerEvent()) {
      setSourcePickerEventId(null);
      setSourcePickerCueId(null);
      setSourcePickerQuery("");
    }
  });
  createEffect(() => {
    const options = matchingSourceOptions();
    if (!sourcePickerOpen()) return;
    if (options.length === 0) {
      return;
    }
    if (!options.some((cue) => cue.id === sourcePickerCueId())) {
      setSourcePickerCueId(options[0].id);
    }
  });
  createEffect(() => {
    const reconciledJumpTarget = reconcileTimelineSceneBlockJumpTarget(
      props.eventRows.map((event) => event.id),
      props.jumpToEventId,
    );
    if (reconciledJumpTarget !== props.jumpToEventId) {
      props.onJumpToEventId(reconciledJumpTarget);
    }
  });

  const openJumpPicker = (event: TimelineSceneBlockRow, draft: TimelineEventDraft) => {
    setSourcePickerEventId(null);
    setSourcePickerCueId(null);
    setSourcePickerQuery("");
    setJumpPickerEventId(event.id);
    setJumpPickerQuery("");
    setJumpPickerTargetId(
      draft.jump_to_event_id !== null && props.eventRows.some((candidate) => candidate.id === draft.jump_to_event_id)
        ? draft.jump_to_event_id
        : props.eventRows[0]?.id ?? null,
    );
  };

  const applyJumpPicker = () => {
    const event = jumpPickerEvent();
    const targetId = jumpPickerTargetId();
    if (targetId === null || !matchingJumpRows().some((candidate) => candidate.id === targetId)) return;
    if (jumpPickerEventId() === 0) {
      props.onJumpToEventId(targetId);
    } else if (event) {
      props.onUpdateEventDraft(event, { jump_to_event_id: targetId });
    } else {
      return;
    }
    setJumpPickerEventId(null);
    setJumpPickerTargetId(null);
    setJumpPickerQuery("");
  };

  const openComposerJumpPicker = () => {
    setSourcePickerEventId(null);
    setSourcePickerCueId(null);
    setSourcePickerQuery("");
    setJumpPickerEventId(0);
    setJumpPickerTargetId(
      props.jumpToEventId !== null && props.eventRows.some((event) => event.id === props.jumpToEventId)
        ? props.jumpToEventId
        : props.eventRows[0]?.id ?? null,
    );
    setJumpPickerQuery("");
  };

  const openSourcePicker = (event: TimelineSceneBlockRow, draft: TimelineEventDraft) => {
    setJumpPickerEventId(null);
    setJumpPickerTargetId(null);
    setJumpPickerQuery("");
    setSourcePickerEventId(event.id);
    setSourcePickerCueId(draft.cue_id);
    setSourcePickerQuery("");
  };

  const applySourcePicker = () => {
    const event = sourcePickerEvent();
    const cueId = sourcePickerCueId();
    if (cueId === null || !matchingSourceOptions().some((cue) => cue.id === cueId)) return;
    if (sourcePickerEventId() === 0) {
      props.onSelectedCueId(cueId);
    } else if (event) {
      const sourceCue = props.cueOptions.find((cue) => cue.id === cueId);
      props.onUpdateEventDraft(event, {
        cue_id: cueId,
        ...(sourceCue?.authored_beats === null
          ? { conform_to_tempo: false, loop_fill: false, loop_count: 1 }
          : {}),
      });
    } else {
      return;
    }
    setSourcePickerEventId(null);
    setSourcePickerCueId(null);
    setSourcePickerQuery("");
  };

  const openComposerSourcePicker = () => {
    setJumpPickerEventId(null);
    setJumpPickerTargetId(null);
    setJumpPickerQuery("");
    setSourcePickerEventId(0);
    setSourcePickerCueId(props.selectedCueId);
    setSourcePickerQuery("");
  };

  // T3 inspector: one fixed properties pane for the selected block with
  // Enter/blur commit (no Save button). Text inputs accept M:SS.ff or raw ms.
  const selectedRow = createMemo(() =>
    props.eventRows.find((event) => event.id === props.selectedEventId) ?? null,
  );
  const inspectorDraft = () => {
    const row = selectedRow();
    return row ? props.timelineEventDraft(row) : null;
  };
  const inspectorDirty = () => {
    const row = selectedRow();
    const draft = inspectorDraft();
    return Boolean(row && draft && !timelineEventDraftMatchesSummary(row, draft));
  };
  const commitInspectorPatch = (patch: Partial<TimelineEventDraft>) => {
    const row = selectedRow();
    if (!row) return;
    props.onUpdateEventDraft(row, patch);
    void props.onSaveEvent(row);
  };
  const commitInspectorTime = (field: "time_ms" | "duration_ms", input: HTMLInputElement) => {
    const row = selectedRow();
    const draft = inspectorDraft();
    if (!row || !draft) return;
    const parsed = parseTimelineTimeInput(input.value);
    const current = field === "time_ms" ? draft.time_ms : draft.duration_ms;
    if (parsed === null) {
      input.value = formatTimelineTimeInput(current);
      return;
    }
    const nextMs = field === "duration_ms" && row.duration_ms > 0 ? Math.max(1, parsed) : Math.max(0, parsed);
    if (nextMs === current) {
      input.value = formatTimelineTimeInput(current);
      return;
    }
    if (field === "time_ms") {
      commitInspectorPatch({
        time_ms: nextMs,
        ...(draft.conform_to_tempo ? { time_beats: millisecondsToBeats(nextMs) } : {}),
      });
    } else {
      commitInspectorPatch({
        duration_ms: nextMs,
        ...(draft.conform_to_tempo && !draft.loop_fill ? { duration_beats: millisecondsToBeats(nextMs) } : {}),
      });
    }
  };
  const blurOnEnter = (event: KeyboardEvent & { currentTarget: HTMLInputElement }) => {
    if (event.key === "Enter") {
      event.preventDefault();
      event.currentTarget.blur();
    }
  };
  const finderStartStamp = (event: TimelineSceneBlockRow) => formatTimelineTimeInput(event.time_ms);
  const finderLaneLabel = (event: TimelineSceneBlockRow) =>
    event.layer_id !== null && event.layer_id !== undefined ? `L${event.layer_id}` : event.track;

  return (
    <section
      class="sceneBlockWorkspace"
      classList={{ sceneBlockWorkspaceInspectorOnly: Boolean(props.inspectorOnly) }}
      aria-label="Linked Scene Blocks"
      data-scene-block-inspector-only={props.inspectorOnly ? "true" : undefined}
      ref={(element) => { workspaceElement = element; }}
    >
      <Show when={!props.inspectorOnly}>
      <>
      <header class="sceneBlockWorkspaceHeader">
        <div class="sceneBlockHeading">
          <small>SHOW SEQUENCE</small>
          <h3>Scene Blocks</h3>
        </div>
        <p>
          <span class="sceneBlockLinkBadge">LINKED</span>
          <span>Source Cue edits update every placed instance.</span>
        </p>
        <div class="sceneBlockStats" aria-label="Scene Block summary">
          <span><small>Blocks</small><strong>{blockCount()}</strong></span>
          <span><small>Points</small><strong>{pointCount()}</strong></span>
        </div>
        <nav class="sceneBlockViewToggle" aria-label="Scene Block editor view">
          <button
            type="button"
            classList={{ active: editorView() === "finder" }}
            aria-pressed={editorView() === "finder"}
            data-scene-block-view-toggle="finder"
            onClick={() => setEditorView("finder")}
          >
            Finder
          </button>
          <button
            type="button"
            classList={{ active: editorView() === "list" }}
            aria-pressed={editorView() === "list"}
            data-scene-block-view-toggle="list"
            onClick={() => setEditorView("list")}
          >
            List
          </button>
        </nav>
      </header>

      <form
        class="sceneBlockComposer"
        onSubmit={(event) => {
          event.preventDefault();
          void props.onAddBlock();
        }}
      >
        <div class="sceneBlockSourceField sceneBlockComposerPickerField">
          <span>Source Cue</span>
          <button
            type="button"
            class="sceneBlockComposerPickerButton sceneBlockComposerSourceButton"
            disabled={props.cueOptions.length === 0}
            onClick={openComposerSourcePicker}
          >
            <span data-no-localize>
              {composerSourceCue()
                ? sourceCueOptionLabel(composerSourceCue()!)
                : "Choose source…"}
            </span>
            <small>Find…</small>
          </button>
          <button
            type="button"
            classList={{ active: props.selectedCueId !== null && props.armedCueId === props.selectedCueId }}
            aria-pressed={props.selectedCueId !== null && props.armedCueId === props.selectedCueId}
            aria-label={props.armedCueId === props.selectedCueId ? "Disarm Cue" : "Arm Cue"}
            data-timeline-arm-cue={props.selectedCueId ?? undefined}
            disabled={props.selectedCueId === null}
            onClick={() => props.onArmCue(props.selectedCueId)}
          >
            {props.armedCueId === props.selectedCueId ? "Disarm Cue" : "Arm Cue"}
          </button>
        </div>
        <label>
          Start ms
          <input
            type="number"
            min="0"
            value={props.startMs}
            onInput={(event) => props.onStartMs(Number(event.currentTarget.value))}
          />
        </label>
        <label>
          Duration ms
          <input
            type="number"
            min="1"
            value={props.durationMs}
            onInput={(event) => props.onDurationMs(Number(event.currentTarget.value))}
          />
        </label>
        <label>
          Loops
          <input
            type="number"
            min="1"
            max="256"
            value={props.loopCount}
            onInput={(event) => props.onLoopCount(Number(event.currentTarget.value))}
          />
        </label>
        <label>
          <span class="sceneBlockFieldLabel">
            Lane
            <small class="sceneBlockLaneScopeHint" title="Lane is visual; the full linked Cue fires.">FULL CUE</small>
          </span>
          <select value={props.track} onInput={(event) => props.onTrack(event.currentTarget.value as TimelineTrackKind)}>
            <option value="Lighting">Lighting</option>
            <option value="Video">Video</option>
          </select>
        </label>
        <div class="sceneBlockAfterField sceneBlockComposerPickerField">
          <span class="sceneBlockFieldLabel">
            After
            <small class="sceneBlockPlaybackJumpHint" title="After jumps apply to Timeline Play only. External MTC/LTC timecode remains authoritative.">PLAY ONLY</small>
          </span>
          <div class="sceneBlockComposerAfterButtons">
            <button
              type="button"
              class="sceneBlockComposerPickerButton sceneBlockComposerAfterButton"
              disabled={props.eventRows.length === 0}
              onClick={openComposerJumpPicker}
            >
              <span>{composerJumpTarget() ? `Jump · ${jumpOptionLabel(composerJumpTarget()!)}` : "Continue"}</span>
              <small>Find…</small>
            </button>
            <button type="button" onClick={() => props.onJumpToEventId(null)} disabled={props.jumpToEventId === null}>Clear</button>
          </div>
        </div>
        <div class="sceneBlockComposerActions">
          <button class="primary" type="submit" disabled={props.cueOptions.length === 0}>Add Linked Block</button>
          <button
            type="button"
            disabled={props.cueOptions.length === 0}
            title={`Place at the current playhead (${props.positionMs} ms)`}
            onClick={() => void props.onAddBlockAtPlayhead()}
          >
            At Playhead
          </button>
        </div>
      </form>
      </>
      </Show>

      <Show when={sourcePickerOpen()}>
        <div class="sceneBlockSourcePicker" role="group" aria-label="Shared source Cue picker">
            <span>
              <small>Source Cue</small>
              <strong>{sourcePickerEventId() === 0 ? "New Block" : `Block #${sourcePickerEvent()?.id}`}</strong>
            </span>
            <input
              type="search"
              aria-label="Search source Cues"
              placeholder="Search number or name"
              value={sourcePickerQuery()}
              onInput={(inputEvent) => setSourcePickerQuery(inputEvent.currentTarget.value)}
            />
            <select
              aria-label="Source Cue result"
              value={sourcePickerCueId() ?? ""}
              disabled={matchingSourceOptions().length === 0}
              onInput={(inputEvent) => setSourcePickerCueId(Number(inputEvent.currentTarget.value))}
            >
              <For each={matchingSourceOptions()}>
                {(cue) => <option data-no-localize value={cue.id}>{sourceCueOptionLabel(cue)}</option>}
              </For>
            </select>
            <button
              type="button"
              classList={{ active: sourcePickerCueId() !== null && props.armedCueId === sourcePickerCueId() }}
              aria-pressed={sourcePickerCueId() !== null && props.armedCueId === sourcePickerCueId()}
              aria-label={props.armedCueId === sourcePickerCueId() ? "Disarm Cue" : "Arm Cue"}
              data-timeline-arm-cue={sourcePickerCueId() ?? undefined}
              disabled={sourcePickerCueId() === null}
              onClick={() => props.onArmCue(sourcePickerCueId())}
            >
              {props.armedCueId === sourcePickerCueId() ? "Disarm Cue" : "Arm Cue"}
            </button>
            <small>{Math.min(sourcePickerResultLimit, matchingSourceOptions().length)} shown · {props.cueOptions.length} total</small>
            <button
              type="button"
              class="primary"
              onClick={applySourcePicker}
              disabled={sourcePickerCueId() === null || !matchingSourceOptions().some((cue) => cue.id === sourcePickerCueId())}
            >
              Apply Source
            </button>
            <button
              type="button"
              onClick={() => {
                setSourcePickerEventId(null);
                setSourcePickerCueId(null);
                setSourcePickerQuery("");
              }}
            >
              Cancel
            </button>
          </div>
      </Show>

      <Show when={jumpPickerOpen()}>
        <div class="sceneBlockJumpPicker" role="group" aria-label="Shared jump target picker">
            <span>
              <small>After target</small>
              <strong>{jumpPickerEventId() === 0 ? "New Block" : `Block #${jumpPickerEvent()?.id}`}</strong>
            </span>
            <input
              type="search"
              aria-label="Search jump targets"
              placeholder="Find block ID, Cue, or name"
              value={jumpPickerQuery()}
              onInput={(inputEvent) => setJumpPickerQuery(inputEvent.currentTarget.value)}
            />
            <select
              aria-label="Jump target placement"
              value={jumpPickerTargetId() ?? ""}
              disabled={matchingJumpRows().length === 0}
              onInput={(inputEvent) => setJumpPickerTargetId(Number(inputEvent.currentTarget.value))}
            >
              <For each={matchingJumpRows()}>
                {(candidate) => (
                  <option value={candidate.id}>
                    {candidate.id === jumpPickerEvent()?.id ? "Repeat this block" : "Jump"} · {jumpOptionLabel(candidate)}
                  </option>
                )}
              </For>
            </select>
            <small>{matchingJumpRows().length} shown · {props.eventRows.length} total</small>
            <button
              type="button"
              class="primary"
              onClick={applyJumpPicker}
              disabled={jumpPickerTargetId() === null || !matchingJumpRows().some((event) => event.id === jumpPickerTargetId())}
            >
              Apply Target
            </button>
            <button
              type="button"
              onClick={() => {
                setJumpPickerEventId(null);
                setJumpPickerTargetId(null);
                setJumpPickerQuery("");
              }}
            >
              Cancel
            </button>
          </div>
      </Show>

      <Show when={!props.inspectorOnly && editorView() === "list"}>
        <div class="sceneBlockColumnGuide" aria-hidden="true">
          <span>Source / live status</span>
          <span>Instance timing and flow</span>
          <span>Actions</span>
        </div>
      </Show>

      <Show when={!props.inspectorOnly && props.filterEventIds !== null}>
        <div class="sceneBlockOverlapFilter" role="status">
          <span>{props.filterLabel ?? "Overlap"} · {props.filterEventIds?.length ?? 0} blocks</span>
          <button type="button" onClick={props.onClearEventFilter}>Clear overlap filter</button>
        </div>
      </Show>

      <Show when={!props.inspectorOnly && (props.eventRows.length > sceneBlockRowsPerPage || rowQuery().length > 0 || props.filterEventIds !== null)}>
        <nav class="sceneBlockPager" aria-label="Scene Block pages">
          <input
            class="sceneBlockRowSearch"
            type="search"
            aria-label="Search Scene Blocks"
            placeholder="Find ID, Cue, name, or start"
            value={rowQuery()}
            onInput={(event) => {
              setRowQuery(event.currentTarget.value);
              setPage(0);
            }}
          />
          <button
            type="button"
            onClick={() => {
              setRowQuery("");
              setPage(0);
            }}
            disabled={!rowQuery()}
          >
            Clear
          </button>
          <button type="button" onClick={() => setPage(0)} disabled={page() === 0}>First</button>
          <button type="button" onClick={() => setPage(Math.max(0, page() - 1))} disabled={page() === 0}>Prev</button>
          <span>Blocks {pageRangeLabel()}</span>
          <button type="button" onClick={() => setPage(Math.min(pageCount() - 1, page() + 1))} disabled={page() >= pageCount() - 1}>Next</button>
          <button type="button" onClick={() => setPage(pageCount() - 1)} disabled={page() >= pageCount() - 1}>Last</button>
        </nav>
      </Show>

      <Show when={props.inspectorOnly || editorView() === "finder"}>
        <div
          class="sceneBlockFinderSplit"
          classList={{ sceneBlockFinderSplitInspectorOnly: Boolean(props.inspectorOnly) }}
          data-scene-block-finder-split
          data-scene-block-inspector-only={props.inspectorOnly ? "true" : undefined}
        >
          <Show when={!props.inspectorOnly}>
          <div class="sceneBlockFinder" role="list" ref={(element) => { listElement = element; }}>
            <Show
              when={filteredRows().length > 0}
              fallback={
                <div class="sceneBlockEmpty">
                  <strong>No Scene Blocks yet</strong>
                  <span>Select a source Cue, set its instance timing, then place the first linked block.</span>
                </div>
              }
            >
              <For each={visibleRows()}>
                {(event) => (
                  <button
                    type="button"
                    role="listitem"
                    class="sceneBlockRow sceneBlockFinderRow"
                    classList={{ selected: props.selectedEventId === event.id }}
                    data-scene-block-id={event.id}
                    onClick={() => props.onSelectEvent(event.id)}
                  >
                    <span
                      class="sceneBlockFinderChip"
                      aria-hidden="true"
                      style={{ background: cueIdentityCss(event.cue_id, props.cueColors?.[event.cue_id], "band") }}
                    />
                    <span class="sceneBlockFinderName" data-no-localize>
                      <small>#{event.id}</small> {event.cue_label}
                    </span>
                    <span class="sceneBlockFinderStamp" data-no-localize>{finderStartStamp(event)}</span>
                    <span class="sceneBlockFinderLane" data-no-localize>{finderLaneLabel(event)}</span>
                  </button>
                )}
              </For>
            </Show>
          </div>
          </Show>
          <aside
            class="sceneBlockInspector"
            classList={{ sceneBlockInspectorOnly: Boolean(props.inspectorOnly) }}
            data-scene-block-inspector
            data-scene-block-inspector-only={props.inspectorOnly ? "true" : undefined}
            aria-label="Block Properties"
          >
            <Show
              when={selectedRow()}
              fallback={
                <div class="sceneBlockEmpty">
                  <strong>Block Properties</strong>
                  <span>Select a block in the Finder or on the canvas to edit it here.</span>
                </div>
              }
            >
              {(row) => {
                const draft = () => props.timelineEventDraft(row());
                const sourceCue = () => cueOptionById().get(draft().cue_id) ?? null;
                return (
                  <div class="sceneBlockInspectorBody">
                    <div class="sceneBlockInspectorHead">
                      <span
                        class="sceneBlockFinderChip"
                        aria-hidden="true"
                        style={{ background: cueIdentityCss(row().cue_id, props.cueColors?.[row().cue_id], "band") }}
                      />
                      <strong data-no-localize>#{row().id} {row().cue_label}</strong>
                      <Show when={inspectorDirty()}>
                        <span class="sceneBlockUnsavedBadge" data-scene-block-inspector-dirty>UNSAVED</span>
                      </Show>
                    </div>
                    <div class="sceneBlockInspectorFields">
                      <label>
                        Start
                        <input
                          type="text"
                          data-scene-block-inspector-start
                          title="M:SS.ff or raw ms"
                          value={formatTimelineTimeInput(draft().time_ms)}
                          onKeyDown={blurOnEnter}
                          onBlur={(inputEvent) => commitInspectorTime("time_ms", inputEvent.currentTarget)}
                        />
                      </label>
                      <label>
                        Duration
                        <input
                          type="text"
                          data-scene-block-inspector-duration
                          title="M:SS.ff or raw ms"
                          value={formatTimelineTimeInput(draft().duration_ms)}
                          onKeyDown={blurOnEnter}
                          onBlur={(inputEvent) => commitInspectorTime("duration_ms", inputEvent.currentTarget)}
                        />
                      </label>
                      <label>
                        Loops
                        <input
                          type="number"
                          min="1"
                          max="256"
                          disabled={draft().loop_fill || row().duration_ms <= 0}
                          value={draft().loop_count}
                          onKeyDown={blurOnEnter}
                          onBlur={(inputEvent) => {
                            const loops = Math.min(256, Math.max(1, Number(inputEvent.currentTarget.value) || 1));
                            if (loops !== draft().loop_count) commitInspectorPatch({ loop_count: loops });
                          }}
                        />
                      </label>
                      <label>
                        Fade In ms
                        <input
                          type="number"
                          min="0"
                          max={Math.max(0, draft().duration_ms)}
                          value={draft().fade_in_ms}
                          onKeyDown={blurOnEnter}
                          onBlur={(inputEvent) => {
                            const fadeMs = Math.min(draft().duration_ms, Math.max(0, Number(inputEvent.currentTarget.value) || 0));
                            if (fadeMs !== draft().fade_in_ms) commitInspectorPatch({ fade_in_ms: fadeMs });
                          }}
                        />
                      </label>
                      <label>
                        Fade Out ms
                        <input
                          type="number"
                          min="0"
                          max={Math.max(0, draft().duration_ms)}
                          value={draft().fade_out_ms}
                          onKeyDown={blurOnEnter}
                          onBlur={(inputEvent) => {
                            const fadeMs = Math.min(draft().duration_ms, Math.max(0, Number(inputEvent.currentTarget.value) || 0));
                            if (fadeMs !== draft().fade_out_ms) commitInspectorPatch({ fade_out_ms: fadeMs });
                          }}
                        />
                      </label>
                      <span class="sceneBlockInspectorLane">
                        <small>Lane</small>
                        <strong data-no-localize>{finderLaneLabel(row())}</strong>
                      </span>
                    </div>
                    <div class="sceneBlockInspectorSource">
                      <small>Source Cue</small>
                      <span data-no-localize>{sourceCue() ? sourceCueOptionLabel(sourceCue()!) : `Missing Cue ${draft().cue_id}`}</span>
                      <span class="tabularNums" data-scene-block-step-count>
                        {`${sourceCue()?.step_count ?? 0} Static step(s)`}
                      </span>
                    </div>
                    <div class="sceneBlockInspectorActions">
                      <button type="button" onClick={() => openSourcePicker(row(), draft())}>Change Source…</button>
                      <button type="button" onClick={() => openJumpPicker(row(), draft())}>After…</button>
                      <button type="button" onClick={() => props.onOpenSourceCue(draft().cue_id)}>Edit Source</button>
                      <button
                        type="button"
                        classList={{ active: props.armedCueId === draft().cue_id }}
                        aria-pressed={props.armedCueId === draft().cue_id}
                        data-timeline-arm-cue={draft().cue_id}
                        onClick={() => props.onArmCue(draft().cue_id)}
                      >
                        {props.armedCueId === draft().cue_id ? "Disarm Cue" : "Arm Cue"}
                      </button>
                      <button
                        type="button"
                        onClick={() => {
                          if (confirmTimelinePlacementRemoval(
                            row().id,
                            `${row().cue_number} · ${row().cue_label}`,
                            totalDurationMs(row()),
                          )) {
                            void props.onRemoveEvent(row());
                          }
                        }}
                      >
                        Remove
                      </button>
                    </div>
                  </div>
                );
              }}
            </Show>
          </aside>
        </div>
      </Show>

      <Show when={!props.inspectorOnly && editorView() === "list"}>
      <div class="sceneBlockList" role="list" ref={(element) => { listElement = element; }}>
        <Show
          when={props.eventRows.length > 0}
          fallback={
            <div class="sceneBlockEmpty">
              <strong>No Scene Blocks yet</strong>
              <span>Select a source Cue, set its instance timing, then place the first linked block.</span>
            </div>
          }
        >
          <For each={visibleRows()}>
            {(event) => {
              const draft = () => props.timelineEventDraft(event);
              const isDirty = () => !timelineEventDraftMatchesSummary(event, draft());
              const playbackStatus = () => timelineSceneBlockPlaybackStatus(
                event,
                props.positionMs,
                props.executionLive,
                playbackTimingForEvent(event),
              );
              const isBlock = () => event.duration_ms > 0 || draft().duration_ms > 0;
              const currentSourceCue = () => props.cueOptions.find((cue) => cue.id === draft().cue_id);
              const canConform = () => currentSourceCue()?.authored_beats !== null
                && currentSourceCue()?.authored_beats !== undefined;
              const rateBadge = () => timelineConformRateBadge({
                conform_to_tempo: draft().conform_to_tempo,
                rate: event.rate ?? null,
              });
              const setConformToTempo = (enabled: boolean) => {
                if (!enabled) {
                  props.onUpdateEventDraft(event, {
                    conform_to_tempo: false,
                    loop_fill: false,
                    loop_count: 1,
                  });
                  return;
                }
                const windowMs = Math.max(1, Math.round(totalDurationMs(draft())));
                props.onUpdateEventDraft(event, {
                  time_beats: millisecondsToBeats(draft().time_ms),
                  duration_ms: windowMs,
                  duration_beats: millisecondsToBeats(windowMs),
                  conform_to_tempo: true,
                  loop_fill: false,
                  loop_count: 1,
                });
              };
              const setLoopFill = (enabled: boolean) => props.onUpdateEventDraft(event, {
                loop_fill: enabled,
                ...(!enabled
                  ? {
                      duration_beats: millisecondsToBeats(draft().duration_ms),
                      loop_count: 1,
                    }
                  : {}),
              });
              const savedSourceCue = () => props.cueOptions.find((cue) => cue.id === event.cue_id);
              const currentJumpTarget = () => props.eventRows.find(
                (candidate) => candidate.id === draft().jump_to_event_id && candidate.id !== event.id,
              );
              const jumpMode = () => {
                if (draft().jump_to_event_id === null) return "continue";
                if (draft().jump_to_event_id === event.id) return "self";
                return "current";
              };
              const nextPlaybackLabel = () => {
                if (event.jump_to_event_id === event.id) return "Repeat this block";
                if (event.jump_to_event_id === null) return "Continue";
                const target = props.eventRows.find((candidate) => candidate.id === event.jump_to_event_id);
                return target ? `Block #${target.id}` : `Missing #${event.jump_to_event_id}`;
              };
              return (
                <article
                  class={`sceneBlockRow ${isBlock() ? "linkedBlock" : "legacyPoint"} ${playbackStatus().under_playhead ? "underPlayhead" : ""} ${playbackStatus().live ? "live" : ""} ${isDirty() ? "dirty" : ""} ${props.selectedEventId === event.id ? "selected" : ""}`}
                  role="listitem"
                  style={{ "--identity": cueIdentityCss(event.cue_id, props.cueColors?.[event.cue_id], "fill") }}
                  data-scene-block-id={event.id}
                  data-source-cue-id={draft().cue_id}
                  data-dirty={isDirty() ? "true" : "false"}
                  data-live={playbackStatus().live ? "true" : "false"}
                  onClick={() => props.onSelectEvent(event.id)}
                  onFocusIn={() => props.onSelectEvent(event.id)}
                >
                  <div class="sceneBlockSource">
                    <div class="sceneBlockSourceBadges">
                      <span class={isBlock() ? "sceneBlockLinkBadge" : "sceneBlockPointBadge"}>
                        {isBlock() ? "LINKED" : "POINT"}
                      </span>
                      <Show when={isDirty()}><span class="sceneBlockUnsavedBadge">UNSAVED</span></Show>
                      <Show when={playbackStatus().live}>
                        <span class="sceneBlockLiveBadge">LIVE</span>
                      </Show>
                      <Show when={playbackStatus().under_playhead && !playbackStatus().live}>
                        <span class="sceneBlockPlayheadBadge">PLAYHEAD</span>
                      </Show>
                      <Show when={rateBadge()}>
                        {(badge) => (
                          <span
                            class="sceneBlockRateBadge tabularNums"
                            title="Tempo conform rate"
                            data-no-localize
                          >
                            {badge()}
                          </span>
                        )}
                      </Show>
                      <small>#{event.id}</small>
                    </div>
                    <strong data-no-localize>
                      {currentSourceCue()
                        ? sourceCueOptionLabel(currentSourceCue()!)
                        : `Missing Cue ${draft().cue_id}`}
                    </strong>
                    <span>{currentSourceCue()?.source_summary ?? "Source Cue is unavailable"}</span>
                    <small>
                      {isBlock()
                        ? draft().conform_to_tempo
                          ? `${draft().duration_ms} ms window · ${draft().loop_count} ${draft().loop_fill ? "fill loops" : "tempo iterations"}`
                          : `${draft().duration_ms} ms × ${draft().loop_count} = ${totalDurationMs(draft())} ms span`
                        : "Legacy instant trigger · set Duration to upgrade"}
                    </small>
                    <Show when={playbackStatus().under_playhead}>
                      <div class="sceneBlockLiveStatus">
                        <strong>
                          {isBlock()
                            ? `Loop ${playbackStatus().iteration}/${playbackStatus().iteration_count}`
                            : "At playhead"}
                        </strong>
                        <span>{playbackStatus().remaining_ms} ms left · After: {nextPlaybackLabel()}</span>
                      </div>
                    </Show>
                    <Show when={playbackStatus().live && isDirty()}>
                      <div class="sceneBlockLiveOutput">
                        OUTPUT: {savedSourceCue()
                          ? sourceCueOptionLabel(savedSourceCue()!)
                          : `Cue #${event.cue_id}`} · {event.duration_ms} ms {event.conform_to_tempo
                            ? `window · ${event.loop_count} ${event.loop_fill ? "fill loops" : "tempo iterations"}`
                            : `× ${event.loop_count}`}
                      </div>
                    </Show>
                  </div>

                  <div class="sceneBlockRowFields">
                    <div class="sceneBlockSourceField">
                      <span>Source Cue</span>
                      <button
                        type="button"
                        class="sceneBlockSourceButton"
                        aria-label={`Change source Cue for block ${event.id}`}
                        onClick={() => openSourcePicker(event, draft())}
                      >
                        <span data-no-localize>
                          {currentSourceCue()
                            ? sourceCueOptionLabel(currentSourceCue()!)
                            : `Missing Cue ${draft().cue_id}`}
                        </span>
                        <small>Change…</small>
                      </button>
                      <button
                        type="button"
                        classList={{ active: props.armedCueId === draft().cue_id }}
                        aria-pressed={props.armedCueId === draft().cue_id}
                        aria-label={props.armedCueId === draft().cue_id ? "Disarm Cue" : "Arm Cue"}
                        data-timeline-arm-cue={draft().cue_id}
                        onClick={() => props.onArmCue(draft().cue_id)}
                      >
                        {props.armedCueId === draft().cue_id ? "Disarm Cue" : "Arm Cue"}
                      </button>
                    </div>
                    <label>
                      Start ms
                      <input
                        type="number"
                        min="0"
                        value={draft().time_ms}
                        onInput={(inputEvent) => {
                          const timeMs = Number(inputEvent.currentTarget.value);
                          props.onUpdateEventDraft(event, {
                            time_ms: timeMs,
                            ...(draft().conform_to_tempo ? { time_beats: millisecondsToBeats(timeMs) } : {}),
                          });
                        }}
                      />
                    </label>
                    <label>
                      Duration ms
                      <input
                        type="number"
                        min={event.duration_ms > 0 ? "1" : "0"}
                        value={draft().duration_ms}
                        onInput={(inputEvent) => {
                          const durationMs = event.duration_ms > 0
                            ? Math.max(1, Number(inputEvent.currentTarget.value))
                            : Number(inputEvent.currentTarget.value);
                          props.onUpdateEventDraft(event, {
                            duration_ms: durationMs,
                            ...(draft().conform_to_tempo && !draft().loop_fill
                              ? { duration_beats: millisecondsToBeats(durationMs) }
                              : {}),
                          });
                        }}
                      />
                    </label>
                    <label>
                      Loops
                      <input
                        type="number"
                        min="1"
                        max="256"
                        value={draft().loop_count}
                        disabled={!isBlock() || draft().loop_fill}
                        onInput={(inputEvent) => props.onUpdateEventDraft(event, { loop_count: Number(inputEvent.currentTarget.value) })}
                      />
                    </label>
                    <label>
                      Fade In ms
                      <input
                        type="number"
                        min="0"
                        max={Math.max(0, draft().duration_ms)}
                        value={draft().fade_in_ms}
                        onInput={(inputEvent) => props.onUpdateEventDraft(event, {
                          fade_in_ms: Math.min(draft().duration_ms, Math.max(0, Number(inputEvent.currentTarget.value))),
                        })}
                      />
                    </label>
                    <label>
                      Fade Out ms
                      <input
                        type="number"
                        min="0"
                        max={Math.max(0, draft().duration_ms)}
                        value={draft().fade_out_ms}
                        onInput={(inputEvent) => props.onUpdateEventDraft(event, {
                          fade_out_ms: Math.min(draft().duration_ms, Math.max(0, Number(inputEvent.currentTarget.value))),
                        })}
                      />
                    </label>
                    <fieldset class="sceneBlockConformControls">
                      <legend>Tempo</legend>
                      <label
                        class="sceneBlockConformToggle"
                        title={canConform() ? "Conform to tempo" : "Set Authored beats on the source Cue first."}
                      >
                        <input
                          type="checkbox"
                          checked={draft().conform_to_tempo}
                          disabled={!isBlock() || !canConform()}
                          onChange={(inputEvent) => setConformToTempo(inputEvent.currentTarget.checked)}
                        />
                        Conform
                      </label>
                      <label class="sceneBlockConformToggle" title="Fill the current block window with tempo-sized iterations.">
                        <input
                          type="checkbox"
                          checked={draft().loop_fill}
                          disabled={!draft().conform_to_tempo}
                          onChange={(inputEvent) => setLoopFill(inputEvent.currentTarget.checked)}
                        />
                        Loop fill
                      </label>
                      <Show when={rateBadge()}>
                        {(badge) => <span class="sceneBlockRateBadge tabularNums" data-no-localize>{badge()}</span>}
                      </Show>
                    </fieldset>
                    <label>
                      <span class="sceneBlockFieldLabel">
                        Lane
                        <small class="sceneBlockLaneScopeHint" title="Lane is visual; the full linked Cue fires.">FULL CUE</small>
                      </span>
                      <select
                        value={draft().track}
                        onInput={(inputEvent) => props.onUpdateEventDraft(event, { track: inputEvent.currentTarget.value as TimelineTrackKind })}
                      >
                        <option value="Lighting">Lighting</option>
                        <option value="Video">Video</option>
                      </select>
                    </label>
                    <label class="sceneBlockAfterField">
                      <span class="sceneBlockFieldLabel">
                        After
                        <small class="sceneBlockPlaybackJumpHint" title="After jumps apply to Timeline Play only. External MTC/LTC timecode remains authoritative.">PLAY ONLY</small>
                      </span>
                      <select
                        value={jumpMode()}
                        disabled={!isBlock()}
                        onInput={(inputEvent) => {
                          const mode = inputEvent.currentTarget.value;
                          if (mode === "continue") {
                            props.onUpdateEventDraft(event, { jump_to_event_id: null });
                          } else if (mode === "self") {
                            props.onUpdateEventDraft(event, { jump_to_event_id: event.id });
                          } else if (mode === "pick") {
                            openJumpPicker(event, draft());
                          }
                        }}
                      >
                        <option value="continue">Continue</option>
                        <option value="self">Repeat this block</option>
                        <Show when={currentJumpTarget()}>
                          {(candidate) => <option value="current">Jump · {jumpOptionLabel(candidate())}</option>}
                        </Show>
                        <option value="pick">Choose target…</option>
                      </select>
                    </label>
                  </div>

                  <div class="sceneBlockRowActions">
                    <button
                      type="button"
                      class="sceneBlockOpenSourceButton"
                      onClick={() => props.onOpenSourceCue(draft().cue_id)}
                    >
                      Edit Source
                    </button>
                    <button type="button" class="primary" disabled={!isDirty()} onClick={() => void props.onSaveEvent(event)}>
                      {isBlock() ? "Save Block" : "Save Point"}
                    </button>
                    <button
                      type="button"
                      onClick={() => {
                        if (confirmTimelinePlacementRemoval(
                          event.id,
                          `${event.cue_number} · ${event.cue_label}`,
                          totalDurationMs(event),
                        )) {
                          void props.onRemoveEvent(event);
                        }
                      }}
                    >
                      Remove
                    </button>
                  </div>
                </article>
              );
            }}
          </For>
        </Show>
      </div>
      </Show>
    </section>
  );
}
