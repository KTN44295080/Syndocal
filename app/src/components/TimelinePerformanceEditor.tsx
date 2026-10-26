import { createSignal, createUniqueId, For } from "solid-js";
import type {
  TimelineFollowRuntimeSummary,
  TimelineFollowAbortFocusFence,
  TimelineFollowSummary,
  TimelineCueAudioStatus,
  TimelineLoopRegionSummary,
  TimelinePhaseRole,
  TimelinePhaseSummary,
  TimelineSnapshot,
} from "../types";
import { timelineVisibleWindowSpanMs, type TimelineVisibleWindow } from "../timelineViewport";
import { TimelineBankPanel } from "./TimelineBankPanel";
import {
  TimelineCueAudioStatusSummary,
  timelineCueAudioLifecycleLabel,
} from "./TimelineCueAudioRoutingPanel";

interface TimelinePerformanceEditorProps {
  timelines: TimelineSnapshot[];
  activeTimelineId: number;
  bpm: number;
  positionMs: number;
  durationMs: number;
  phases: TimelinePhaseSummary[];
  visibleWindow: TimelineVisibleWindow;
  followRuntime: TimelineFollowRuntimeSummary;
  followAbortBusy: boolean;
  followAbortFocusFence: TimelineFollowAbortFocusFence;
  loopRegion: TimelineLoopRegionSummary | null;
  cueAudioStatus: TimelineCueAudioStatus;
  onCreateTimeline: (label: string) => void | Promise<void>;
  onDuplicateTimeline: (timelineId: number) => void | Promise<void>;
  onRemoveTimeline: (timelineId: number) => void | Promise<void>;
  onReorderTimelines: (timelineIds: number[]) => void | Promise<void>;
  onSelectTimeline: (timelineId: number, play: boolean) => void | Promise<void>;
  onSetFollow: (follow: TimelineFollowSummary | null) => void | Promise<void>;
  onAbortFollow: () => Promise<boolean>;
  onSetPhases: (phases: TimelinePhaseSummary[]) => void | Promise<void>;
  onSetLoopRegion: (region: TimelineLoopRegionSummary | null) => void | Promise<void>;
  onSeek: (timeMs: number) => void | Promise<void>;
}

const phaseTime = (timeMs: number) => `${(Math.max(0, timeMs) / 1_000).toFixed(3)}s`;
const timelineToolSections = ["bank", "phases", "audio"] as const;
type TimelineToolSection = typeof timelineToolSections[number];
const timelineToolLabels: Record<TimelineToolSection, string> = {
  bank: "Bank",
  phases: "Phases",
  audio: "Audio",
};

export function TimelinePerformanceEditor(props: TimelinePerformanceEditorProps) {
  const [activeTool, setActiveTool] = createSignal<TimelineToolSection>("bank");
  const toolsId = createUniqueId();
  const toolButtons: Partial<Record<TimelineToolSection, HTMLButtonElement>> = {};
  let toolsRoot: HTMLDivElement | undefined;
  const activateTool = (tool: TimelineToolSection, focus = false) => {
    setActiveTool(tool);
    queueMicrotask(() => {
      const owner = toolsRoot?.closest<HTMLElement>(".timelineToolsDisclosurePanel");
      if (owner) owner.scrollTop = 0;
      if (focus) toolButtons[tool]?.focus({ preventScroll: true });
    });
  };
  const moveToolFocus = (tool: TimelineToolSection, key: string) => {
    const index = timelineToolSections.indexOf(tool);
    const next = key === "Home" ? 0 : key === "End" ? timelineToolSections.length - 1
      : (index + (key === "ArrowRight" ? 1 : -1) + timelineToolSections.length) % timelineToolSections.length;
    activateTool(timelineToolSections[next], true);
  };
  const setLoopBoundary = (edge: "a" | "b") => {
    const current = props.loopRegion;
    const position = Math.max(0, Math.round(props.positionMs));
    const next = edge === "a"
      ? { a_ms: position, b_ms: Math.max(position + 1, current?.b_ms ?? position + 4_000), enabled: current?.enabled ?? false, musical_length_beats: current?.musical_length_beats ?? null }
      : { a_ms: Math.min(current?.a_ms ?? 0, Math.max(0, position - 1)), b_ms: Math.max(position, (current?.a_ms ?? 0) + 1), enabled: current?.enabled ?? false, musical_length_beats: current?.musical_length_beats ?? null };
    void props.onSetLoopRegion(next);
  };
  const updatePhase = (phaseId: number, patch: Partial<TimelinePhaseSummary>) => {
    void props.onSetPhases(props.phases.map((phase) => phase.id === phaseId ? { ...phase, ...patch } : phase));
  };
  const addPhase = () => {
    const startMs = Math.max(0, Math.round(props.positionMs));
    const endMs = Math.max(startMs + 1, Math.min(props.durationMs || startMs + 4_000, startMs + 4_000));
    const nextId = Math.max(0, ...props.phases.map((phase) => phase.id)) + 1;
    void props.onSetPhases([...props.phases, { id: nextId, label: "Phase", role: "custom", start_ms: startMs, end_ms: endMs }]);
  };
  return (
    <div class="timelinePerformanceEditor" data-timeline-performance-editor ref={toolsRoot}>
      <div class="timelinePerformanceLoopBoundaries" role="group" aria-label="Loop boundaries">
        <button type="button" title="Set loop A at playhead" onClick={() => setLoopBoundary("a")}>A</button>
        <button type="button" title="Set loop B at playhead" onClick={() => setLoopBoundary("b")}>B</button>
      </div>
      <div class="timelineToolsTabs" role="tablist" aria-label="Timeline tools">
        <For each={timelineToolSections}>
          {(tool) => (
            <button
              type="button"
              role="tab"
              id={toolsId + "-tab-" + tool}
              aria-controls={toolsId + "-panel-" + tool}
              aria-selected={activeTool() === tool}
              tabIndex={activeTool() === tool ? 0 : -1}
              classList={{ active: activeTool() === tool }}
              data-timeline-tools-tab={tool}
              ref={(button) => { toolButtons[tool] = button; }}
              onClick={() => activateTool(tool)}
              onKeyDown={(event) => {
                if (!["ArrowLeft", "ArrowRight", "Home", "End"].includes(event.key)) return;
                event.preventDefault();
                moveToolFocus(tool, event.key);
              }}
            >
              {timelineToolLabels[tool]}
            </button>
          )}
        </For>
      </div>
      <section
        role="tabpanel"
        id={toolsId + "-panel-bank"}
        aria-labelledby={toolsId + "-tab-bank"}
        data-timeline-tools-panel="bank"
        hidden={activeTool() !== "bank"}
        tabIndex={0}
      >
        <TimelineBankPanel
          embedded
          timelines={props.timelines}
          activeTimelineId={props.activeTimelineId}
          bpm={props.bpm}
          followRuntime={props.followRuntime}
          followAbortBusy={props.followAbortBusy}
          followAbortFocusFence={props.followAbortFocusFence}
          onCreate={props.onCreateTimeline}
          onDuplicate={props.onDuplicateTimeline}
          onRemove={props.onRemoveTimeline}
          onReorder={props.onReorderTimelines}
          onSelect={props.onSelectTimeline}
          onSetFollow={props.onSetFollow}
          onAbortFollow={props.onAbortFollow}
        />
      </section>
      <section
        role="tabpanel"
        id={toolsId + "-panel-audio"}
        aria-labelledby={toolsId + "-tab-audio"}
        data-timeline-tools-panel="audio"
        hidden={activeTool() !== "audio"}
        tabIndex={0}
      >
        <details class="timelineCueAudioEditor" data-timeline-cue-audio-editor data-timeline-cue-audio-status open>
          <summary>
            <span>Timeline authoring audio</span>
            <output classList={{ fault: props.cueAudioStatus.lifecycle === "fault" || Boolean(props.cueAudioStatus.lastError) }} role="status">{timelineCueAudioLifecycleLabel(props.cueAudioStatus)}</output>
          </summary>
          <TimelineCueAudioStatusSummary status={props.cueAudioStatus} />
        </details>
      </section>
      <section
        role="tabpanel"
        id={toolsId + "-panel-phases"}
        aria-labelledby={toolsId + "-tab-phases"}
        data-timeline-tools-panel="phases"
        hidden={activeTool() !== "phases"}
        tabIndex={0}
      >
        <details class="timelinePhaseEditor" data-timeline-phase-editor open>
          <summary>
            <span>Phases</span>
            <strong class="tabularNums" data-no-localize>{props.phases.length}</strong>
          </summary>
          <div class="timelinePhaseEditorBody">
            <div class="timelinePhaseRail" aria-label="Timeline phases">
              <For each={props.phases.filter((phase) => phase.end_ms > props.visibleWindow.start_ms && phase.start_ms < props.visibleWindow.end_ms)}>
                {(phase) => {
                  const span = Math.max(1, timelineVisibleWindowSpanMs(props.visibleWindow));
                  const left = Math.max(0, phase.start_ms - props.visibleWindow.start_ms) / span * 100;
                  const right = Math.min(props.visibleWindow.end_ms, phase.end_ms) - props.visibleWindow.start_ms;
                  const width = Math.max(0.5, right / span * 100 - left);
                  return (
                    <button type="button" class={`timelinePhaseBand ${phase.role}`} style={{ left: `${left}%`, width: `${width}%` }} title={`${phase.label}: ${phaseTime(phase.start_ms)}–${phaseTime(phase.end_ms)}`} onClick={() => void props.onSeek(phase.start_ms)}>
                      <span data-no-localize>{phase.label}</span>
                    </button>
                  );
                }}
              </For>
            </div>
            <div class="timelinePhaseRows">
              <For each={props.phases}>
                {(phase) => (
                  <div class="timelinePhaseRow" data-phase-id={phase.id}>
                    <input aria-label="Phase label" value={phase.label} maxlength="64" onChange={(event) => updatePhase(phase.id, { label: event.currentTarget.value.trim() || phase.label })} />
                    <select aria-label="Phase role" value={phase.role} onChange={(event) => updatePhase(phase.id, { role: event.currentTarget.value as TimelinePhaseRole })}>
                      <option value="intro">Intro</option><option value="verse">Verse</option><option value="pre_chorus">Pre-Chorus</option><option value="chorus">Chorus</option><option value="interlude">Interlude</option><option value="bridge">Bridge</option><option value="breakdown">Breakdown</option><option value="outro">Outro</option><option value="custom">Custom</option>
                    </select>
                    <label><span>Start</span><input class="tabularNums" type="number" min="0" value={phase.start_ms} onChange={(event) => updatePhase(phase.id, { start_ms: Math.max(0, Number(event.currentTarget.value)) })} /></label>
                    <label><span>End</span><input class="tabularNums" type="number" min={phase.start_ms + 1} value={phase.end_ms} onChange={(event) => updatePhase(phase.id, { end_ms: Math.max(phase.start_ms + 1, Number(event.currentTarget.value)) })} /></label>
                    <button type="button" class="danger ghost" aria-label={`Remove phase ${phase.label}`} onClick={() => void props.onSetPhases(props.phases.filter((candidate) => candidate.id !== phase.id))}>Remove</button>
                  </div>
                )}
              </For>
            </div>
            <button type="button" class="secondary" onClick={addPhase}>Add Phase at Playhead</button>
          </div>
        </details>
      </section>
    </div>
  );
}
