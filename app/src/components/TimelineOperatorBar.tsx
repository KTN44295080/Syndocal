import { Show } from "solid-js";
import type { TimelineContextDrawer, TimelineDeskSurface } from "../uiModes";
import type {
  TimelineFollowRuntimeSummary,
  TimelineFollowAbortFocusFence,
  TimelineFollowSummary,
  MachineTimelineCueAudioSettingsV1,
  TimelineCueAudioStatus,
  TimelineLoopRegionSummary,
  TimelineLoopRuntimeSummary,
  TimelinePhaseSummary,
  TimelineSnapshot,
} from "../types";
import type { TimelineStretchMode } from "../timelineBlockGestures";
import {
  TIMELINE_MIN_VISIBLE_WINDOW_MS,
  timelineVisibleWindowSpanMs,
  type TimelineVisibleWindow,
} from "../timelineViewport";
import { TimelinePerformanceEditor } from "./TimelinePerformanceEditor";

interface TimelineOperatorBarProps {
  childTimelineLabel: string | null;
  positionMs: number;
  durationMs: number;
  playing: boolean;
  bpm: number;
  metronomeEnabled: boolean;
  countInBeats: number;
  countInRemainingMs: number;
  phases: TimelinePhaseSummary[];
  guideEnabled: boolean;
  cueAudioStatus: TimelineCueAudioStatus;
  cueAudioMutationBusy: boolean;
  cueAudioLocalError: string | null;
  loopRegion: TimelineLoopRegionSummary | null;
  loopRuntime: TimelineLoopRuntimeSummary;
  timelines: TimelineSnapshot[];
  activeTimelineId: number;
  followRuntime: TimelineFollowRuntimeSummary;
  followAbortBusy: boolean;
  followAbortFocusFence: TimelineFollowAbortFocusFence;
  visibleWindow: TimelineVisibleWindow;
  overviewShowDurationMs: number;
  overviewEditExtentMs: number;
  selectedEventId: number | null;
  selectedCueId: number | null;
  selectedCueIsSuperScene: boolean;
  contextDrawer: TimelineContextDrawer;
  stretchMode: TimelineStretchMode;
  magnetEnabled: boolean;
  armedCueId: number | null;
  armedCueLabel: string | null;
  deskSurface: TimelineDeskSurface;
  showDeskSurfaceTabs?: boolean;
  onExitChildTimeline: () => void;
  onSeek: (timeMs: number) => void | Promise<void>;
  onPause: () => void | Promise<void>;
  onPlay: () => void | Promise<void>;
  onSetMetronome: (enabled: boolean, countInBeats: number) => void | Promise<void>;
  onSetGuideEnabled: (enabled: boolean) => void | Promise<void>;
  onConfigureCueAudio: (settings: MachineTimelineCueAudioSettingsV1) => void;
  onRefreshCueAudio: () => void | Promise<void>;
  onSetPhases: (phases: TimelinePhaseSummary[]) => void | Promise<void>;
  onSetLoopRegion: (region: TimelineLoopRegionSummary | null) => void | Promise<void>;
  onSetLoopEnabled: (enabled: boolean) => void | Promise<void>;
  onScaleLoop: (scale: "half" | "double") => void | Promise<void>;
  onCreateTimeline: (label: string) => void | Promise<void>;
  onDuplicateTimeline: (timelineId: number) => void | Promise<void>;
  onRemoveTimeline: (timelineId: number) => void | Promise<void>;
  onReorderTimelines: (timelineIds: number[]) => void | Promise<void>;
  onSelectTimeline: (timelineId: number, play: boolean) => void | Promise<void>;
  onSetFollow: (follow: TimelineFollowSummary | null) => void | Promise<void>;
  onAbortFollow: () => Promise<boolean>;
  onPanOverview: (direction: -1 | 1) => void;
  onZoomOverview: (scale: number) => void;
  onFitOverview: () => void;
  onRevealSelected: () => void;
  onRevealPlayhead: () => void;
  onStretchMode: (mode: TimelineStretchMode) => void;
  onMagnetEnabled: (enabled: boolean) => void;
  onArmCue: (cueId: number | null) => void;
  onOpenOrCreateSuperScene: (cueId: number) => void | Promise<void>;
  onContextDrawer: (drawer: TimelineContextDrawer) => void;
  onDeskSurface: (surface: TimelineDeskSurface) => void;
}

const formatOperatorTime = (timeMs: number) => {
  const safeMs = Math.max(0, Math.round(timeMs));
  const milliseconds = String(safeMs % 1000).padStart(3, "0");
  const totalSeconds = Math.floor(safeMs / 1000);
  const seconds = String(totalSeconds % 60).padStart(2, "0");
  const totalMinutes = Math.floor(totalSeconds / 60);
  if (totalMinutes === 0) return `${Math.floor(totalSeconds)}.${milliseconds}s`;
  const minutes = String(totalMinutes % 60).padStart(2, "0");
  const hours = Math.floor(totalMinutes / 60);
  return hours > 0
    ? `${hours}:${minutes}:${seconds}.${milliseconds}`
    : `${totalMinutes}:${seconds}.${milliseconds}`;
};

export function TimelineOperatorBar(props: TimelineOperatorBarProps) {
  const selectedCueArmed = () => props.selectedCueId !== null && props.armedCueId === props.selectedCueId;
  const loopEnabled = () => props.loopRuntime.status !== "disabled";
  const setLoopBoundary = (edge: "a" | "b") => {
    const current = props.loopRegion;
    const position = Math.max(0, Math.round(props.positionMs));
    const next = edge === "a"
      ? { a_ms: position, b_ms: Math.max(position + 1, current?.b_ms ?? position + 4_000), enabled: current?.enabled ?? false, musical_length_beats: current?.musical_length_beats ?? null }
      : { a_ms: Math.min(current?.a_ms ?? 0, Math.max(0, position - 1)), b_ms: Math.max(position, (current?.a_ms ?? 0) + 1), enabled: current?.enabled ?? false, musical_length_beats: current?.musical_length_beats ?? null };
    void props.onSetLoopRegion(next);
  };
  const followStatus = () => props.followRuntime.status === "pending" ? "armed" : props.followRuntime.status;
  const abortFollowFromOperator = async (button: HTMLButtonElement) => {
    props.followAbortFocusFence.schedule(await props.onAbortFollow(), () => button.focus(), requestAnimationFrame);
  };
  return (
    <div class="timelineOperatorBar" role="toolbar" aria-label="Timeline tools" data-timeline-operator-bar>
      <div class="timelineTransport" role="group" aria-label="Timeline transport">
        <button type="button" title="Go to timeline start" aria-label="Go to timeline start" onClick={() => void props.onSeek(0)}>
          <span class="timelineToolIcon" aria-hidden="true" data-no-localize>|◀</span>
        </button>
        <button type="button" title="Pause timeline" aria-label="Pause timeline" onClick={() => void props.onPause()} disabled={!props.playing}>
          <span class="timelineToolIcon" aria-hidden="true" data-no-localize>Ⅱ</span>
        </button>
        <button type="button" class="primary" title="Play timeline" aria-label="Play timeline" onClick={() => void props.onPlay()} disabled={props.durationMs === 0 || props.playing}>
          <span class="timelineToolIcon" aria-hidden="true" data-no-localize>▶</span>
        </button>
      </div>
      <div class="timelineFollowOperatorRuntime" role="group" aria-label="Timeline Follow runtime" data-timeline-follow-operator-runtime>
        <output class={`timelineFollowState ${props.followRuntime.status}`} aria-live="polite" data-timeline-follow-operator-badge>
          Follow: {followStatus()}
          <Show when={props.followRuntime.status === "transitioning" || props.followRuntime.status === "settling"}>
            {` ${Math.round(props.followRuntime.progress_millis / 10)}%`}
          </Show>
        </output>
        <Show when={props.followRuntime.status !== "idle" && props.followRuntime.generation > 0}>
          <button
            type="button"
            class="danger"
            style={{ "min-height": "44px", "min-width": "44px" }}
            aria-label="Abort Timeline Follow"
            aria-busy={props.followAbortBusy ? "true" : undefined}
            disabled={props.followAbortBusy || props.followRuntime.status === "aborting"}
            data-timeline-follow-operator-abort
            onClick={(event) => void abortFollowFromOperator(event.currentTarget)}
          >
            Abort
          </button>
        </Show>
      </div>
      <div class="timelineMetronomeControls" role="group" aria-label="Timeline click controls">
        <button
          type="button"
          classList={{ active: props.metronomeEnabled }}
          aria-pressed={props.metronomeEnabled}
          data-timeline-metronome
          onClick={() => void props.onSetMetronome(!props.metronomeEnabled, props.countInBeats || 4)}
        >
          クリック
        </button>
        <button
          type="button"
          classList={{ active: props.countInBeats > 0 }}
          aria-pressed={props.countInBeats > 0}
          data-timeline-count-in
          title="One bar count-in (4 beats)"
          onClick={() => void props.onSetMetronome(props.metronomeEnabled, props.countInBeats > 0 ? 0 : 4)}
        >
          1小節
        </button>
        <Show when={props.countInRemainingMs > 0}>
          <output class="timelineCountInReadout tabularNums" aria-label="Count-in remaining">
            {`${Math.ceil(props.countInRemainingMs / Math.max(1, 60_000 / props.bpm))}`}
          </output>
        </Show>
        <button type="button" classList={{ active: props.guideEnabled }} aria-pressed={props.guideEnabled} data-timeline-guide title="Announce phases and loop transitions" onClick={() => void props.onSetGuideEnabled(!props.guideEnabled)}>Guide</button>
        <Show when={props.cueAudioStatus.lifecycle === "fault" || props.cueAudioStatus.lastError}>
          {(error) => <output class="timelineCueAudioFault" role="status" title={props.cueAudioStatus.lastError ?? props.cueAudioStatus.faultCode}>Cue fault</output>}
        </Show>
      </div>
      <div class="timelineLoopControls" role="group" aria-label="Timeline A-B loop controls">
        <button type="button" data-timeline-loop-boundary title="Set loop A at playhead" onClick={() => setLoopBoundary("a")}>A</button>
        <button type="button" data-timeline-loop-boundary title="Set loop B at playhead" onClick={() => setLoopBoundary("b")}>B</button>
        <button type="button" classList={{ active: loopEnabled() }} aria-pressed={loopEnabled()} disabled={props.loopRegion === null} title={loopEnabled() ? "Disable loop (Break)" : "Enable or arm loop"} data-timeline-loop-toggle onClick={() => void props.onSetLoopEnabled(!loopEnabled())}>Loop</button>
        <button type="button" title="Halve loop length" disabled={props.loopRegion === null} onClick={() => void props.onScaleLoop("half")}>1/2</button>
        <button type="button" title="Double loop length" disabled={props.loopRegion === null} onClick={() => void props.onScaleLoop("double")}>×2</button>
        <output class={`timelineLoopState ${props.loopRuntime.status}`} aria-live="polite" data-no-localize>{props.loopRuntime.status === "armed" ? "ARMED" : props.loopRuntime.status === "looping" ? `LOOP ×${props.loopRuntime.wrap_count}` : "OFF"}</output>
      </div>
      <details class="timelineToolsDisclosure" name="timeline-header-disclosure">
        <summary title="Timeline tools" aria-label="Timeline tools">
          <span class="timelineToolIcon" aria-hidden="true" data-no-localize>▤</span>
        </summary>
        <div class="timelineToolsDisclosurePanel">
          <Show when={props.childTimelineLabel}>
            {(label) => (
              <nav class="timelineBreadcrumb" aria-label="Timeline breadcrumb" data-timeline-breadcrumb>
                <button type="button" onClick={props.onExitChildTimeline}>Show</button>
                <span aria-hidden="true" data-no-localize>›</span>
                <strong data-child-timeline-label data-no-localize>{label()}</strong>
              </nav>
            )}
          </Show>
          <div class="timelineToolsScrubGroup">
            <input
              class="timelineScrubber"
              type="range"
              aria-label="Timeline position"
              min="0"
              max={Math.max(props.durationMs, 1)}
              value={props.positionMs}
              onInput={(event) => void props.onSeek(Number(event.currentTarget.value))}
            />
            <output class="timelineOperatorTime tabularNums" title={`${props.positionMs} / ${props.durationMs} ms`}>
              {formatOperatorTime(props.positionMs)} / {formatOperatorTime(props.durationMs)}
            </output>
          </div>
          <Show when={props.showDeskSurfaceTabs !== false}>
          <nav class="timelineDeskTabs timelineDeskTabs-disclosure" aria-label="Timeline desk surface">
            <button
              type="button"
              class={props.deskSurface === "show" ? "active" : ""}
              title="Show Timeline"
              aria-label="Show Timeline"
              aria-pressed={props.deskSurface === "show"}
              data-timeline-desk-surface="show"
              onClick={() => props.onDeskSurface("show")}
            >
              <span class="timelineToolIcon" aria-hidden="true" data-no-localize>▤</span>
            </button>
            <button
              type="button"
              class={props.deskSurface === "automation" ? "active" : ""}
              title="Automation"
              aria-label="Automation"
              aria-pressed={props.deskSurface === "automation"}
              data-timeline-desk-surface="automation"
              onClick={() => props.onDeskSurface("automation")}
            >
              <span class="timelineToolIcon" aria-hidden="true" data-no-localize>∿</span>
            </button>
            <button
              type="button"
              class={props.deskSurface === "playback" ? "active" : ""}
              title="Playback"
              aria-label="Playback"
              aria-pressed={props.deskSurface === "playback"}
              data-timeline-desk-surface="playback"
              onClick={() => props.onDeskSurface("playback")}
            >
              <span class="timelineToolIcon" aria-hidden="true" data-no-localize>▦</span>
            </button>
          </nav>
          </Show>
          <nav class="timelineViewportToolbar" aria-label="Timeline visible range controls">
            <button type="button" title="Pan Prev" aria-label="Pan Prev" data-timeline-tool="pan-prev" onClick={() => props.onPanOverview(-1)} disabled={props.visibleWindow.start_ms <= 0}>
              <span class="timelineToolIcon" aria-hidden="true" data-no-localize>←</span>
            </button>
            <button type="button" title="Zoom Out" aria-label="Zoom Out" data-timeline-tool="zoom-out" onClick={() => props.onZoomOverview(2)} disabled={timelineVisibleWindowSpanMs(props.visibleWindow) >= props.overviewEditExtentMs}>
              <span class="timelineToolIcon" aria-hidden="true" data-no-localize>−</span>
            </button>
            <button type="button" title="Fit All" aria-label="Fit All" data-timeline-tool="fit-all" onClick={props.onFitOverview}>
              <span class="timelineToolIcon" aria-hidden="true" data-no-localize>↔</span>
            </button>
            <button type="button" title="Zoom In" aria-label="Zoom In" data-timeline-tool="zoom-in" onClick={() => props.onZoomOverview(0.5)} disabled={timelineVisibleWindowSpanMs(props.visibleWindow) <= TIMELINE_MIN_VISIBLE_WINDOW_MS}>
              <span class="timelineToolIcon" aria-hidden="true" data-no-localize>+</span>
            </button>
            <button type="button" title="Pan Next" aria-label="Pan Next" data-timeline-tool="pan-next" onClick={() => props.onPanOverview(1)} disabled={props.visibleWindow.end_ms >= props.overviewEditExtentMs}>
              <span class="timelineToolIcon" aria-hidden="true" data-no-localize>→</span>
            </button>
            <button type="button" title="Reveal Selected" aria-label="Reveal Selected" data-timeline-tool="reveal-selected" onClick={props.onRevealSelected} disabled={props.selectedEventId === null}>
              <span class="timelineToolIcon" aria-hidden="true" data-no-localize>◎</span>
            </button>
            <button type="button" title="Reveal Playhead" aria-label="Reveal Playhead" data-timeline-tool="reveal-playhead" onClick={props.onRevealPlayhead}>
              <span class="timelineToolIcon" aria-hidden="true" data-no-localize>⌖</span>
            </button>
            <output
              class="timelineVisibleRange tabularNums"
              aria-label="Visible timeline range"
              data-visible-start-ms={props.visibleWindow.start_ms}
              data-visible-end-ms={props.visibleWindow.end_ms}
              data-show-duration-ms={props.overviewShowDurationMs}
              data-edit-extent-ms={props.overviewEditExtentMs}
            >
              {formatOperatorTime(props.visibleWindow.start_ms)} – {formatOperatorTime(props.visibleWindow.end_ms)}
            </output>
          </nav>
          <div class="timelineDirectToolbar" aria-label="Timeline block direct manipulation">
            <div class="timelineStretchModeToggle" role="group" aria-label="Stretch mode" data-timeline-stretch-mode-toggle>
              <button type="button" title="Stretch by rate" aria-label="Stretch by rate" classList={{ active: props.stretchMode === "RATE" }} aria-pressed={props.stretchMode === "RATE"} data-timeline-stretch-mode="RATE" data-timeline-tool="stretch-rate" onClick={() => props.onStretchMode("RATE")}>
                <span class="timelineToolIcon" aria-hidden="true" data-no-localize>↯</span>
              </button>
              <button type="button" title="Stretch block window" aria-label="Stretch block window" classList={{ active: props.stretchMode === "WINDOW" }} aria-pressed={props.stretchMode === "WINDOW"} data-timeline-stretch-mode="WINDOW" data-timeline-tool="stretch-window" onClick={() => props.onStretchMode("WINDOW")}>
                <span class="timelineToolIcon" aria-hidden="true" data-no-localize>↔</span>
              </button>
            </div>
            <button type="button" classList={{ active: props.magnetEnabled }} aria-pressed={props.magnetEnabled} aria-label={props.magnetEnabled ? "Disable magnet snap" : "Enable magnet snap"} title={props.magnetEnabled ? "Disable magnet snap" : "Enable magnet snap"} data-timeline-magnet-toggle data-timeline-tool="magnet" onClick={() => props.onMagnetEnabled(!props.magnetEnabled)}>
              <span class="timelineToolIcon" aria-hidden="true" data-no-localize>∩</span>
            </button>
            <button type="button" classList={{ active: selectedCueArmed() }} aria-pressed={selectedCueArmed()} aria-label={selectedCueArmed() ? "Disarm Cue" : "Arm Cue"} title={selectedCueArmed() ? "Disarm Cue" : "Arm Cue"} data-timeline-arm-cue={props.selectedCueId ?? undefined} data-timeline-tool="arm-cue" disabled={props.selectedCueId === null} onClick={() => props.onArmCue(props.selectedCueId)}>
              <span class="timelineToolIcon" aria-hidden="true" data-no-localize>{selectedCueArmed() ? "●" : "○"}</span>
            </button>
            <button type="button" title={props.selectedCueIsSuperScene ? "Edit Timeline" : "Create Timeline"} aria-label={props.selectedCueIsSuperScene ? "Edit Timeline" : "Create Timeline"} data-open-super-scene={props.selectedCueId ?? undefined} data-timeline-tool="super-scene" disabled={props.selectedCueId === null || props.childTimelineLabel !== null} onClick={() => props.selectedCueId !== null && void props.onOpenOrCreateSuperScene(props.selectedCueId)}>
              <span class="timelineToolIcon" aria-hidden="true" data-no-localize>◇</span>
            </button>
            <button
              type="button"
              title="Block Properties"
              aria-label="Block Properties"
              classList={{ active: props.contextDrawer === "block" }}
              aria-pressed={props.contextDrawer === "block"}
              data-timeline-block-properties-toggle
              data-timeline-tool="block-properties"
              onClick={() => props.onContextDrawer(props.contextDrawer === "block" ? "none" : "block")}
            >
              <span class="timelineToolIcon" aria-hidden="true" data-no-localize>▤</span>
            </button>
            <Show when={props.armedCueLabel}>
              {(label) => <output class="timelineArmedCue" data-timeline-armed-cue={props.armedCueId ?? undefined}>Armed: {label()}</output>}
            </Show>
          </div>
          <TimelinePerformanceEditor
            timelines={props.timelines}
            activeTimelineId={props.activeTimelineId}
            bpm={props.bpm}
            positionMs={props.positionMs}
            durationMs={props.durationMs}
            phases={props.phases}
            visibleWindow={props.visibleWindow}
            followRuntime={props.followRuntime}
            followAbortBusy={props.followAbortBusy}
            followAbortFocusFence={props.followAbortFocusFence}
            loopRegion={props.loopRegion}
            cueAudioStatus={props.cueAudioStatus}
            cueAudioMutationBusy={props.cueAudioMutationBusy}
            cueAudioLocalError={props.cueAudioLocalError}
            onCreateTimeline={props.onCreateTimeline}
            onDuplicateTimeline={props.onDuplicateTimeline}
            onRemoveTimeline={props.onRemoveTimeline}
            onReorderTimelines={props.onReorderTimelines}
            onSelectTimeline={props.onSelectTimeline}
            onSetFollow={props.onSetFollow}
            onAbortFollow={props.onAbortFollow}
            onSetPhases={props.onSetPhases}
            onSetLoopRegion={props.onSetLoopRegion}
            onConfigureCueAudio={props.onConfigureCueAudio}
            onRefreshCueAudio={props.onRefreshCueAudio}
            onSeek={props.onSeek}
          />
        </div>
      </details>
    </div>
  );
}
