import { For, Show } from "solid-js";
import type {
  TimelineFollowRuntimeSummary,
  TimelineFollowSummary,
  TimelineGuideAudioStatus,
  TimelineLoopRegionSummary,
  TimelinePhaseRole,
  TimelinePhaseSummary,
  TimelineSnapshot,
} from "../types";
import { timelineVisibleWindowSpanMs, type TimelineVisibleWindow } from "../timelineViewport";
import { TimelineBankPanel } from "./TimelineBankPanel";

interface TimelinePerformanceEditorProps {
  timelines: TimelineSnapshot[];
  activeTimelineId: number;
  bpm: number;
  positionMs: number;
  durationMs: number;
  phases: TimelinePhaseSummary[];
  visibleWindow: TimelineVisibleWindow;
  followRuntime: TimelineFollowRuntimeSummary;
  loopRegion: TimelineLoopRegionSummary | null;
  guideAudioStatus: TimelineGuideAudioStatus;
  guideAudioDevices: string[];
  onCreateTimeline: (label: string) => void | Promise<void>;
  onDuplicateTimeline: (timelineId: number) => void | Promise<void>;
  onRemoveTimeline: (timelineId: number) => void | Promise<void>;
  onReorderTimelines: (timelineIds: number[]) => void | Promise<void>;
  onSelectTimeline: (timelineId: number, play: boolean) => void | Promise<void>;
  onSetFollow: (follow: TimelineFollowSummary | null) => void | Promise<void>;
  onSetPhases: (phases: TimelinePhaseSummary[]) => void | Promise<void>;
  onSetLoopRegion: (region: TimelineLoopRegionSummary | null) => void | Promise<void>;
  onConfigureGuideAudio: (enabled: boolean, gain: number, deviceName: string | null) => void | Promise<void>;
  onSeek: (timeMs: number) => void | Promise<void>;
}

const phaseTime = (timeMs: number) => `${(Math.max(0, timeMs) / 1_000).toFixed(3)}s`;

export function TimelinePerformanceEditor(props: TimelinePerformanceEditorProps) {
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
    <div class="timelinePerformanceEditor" data-timeline-performance-editor>
      <div class="timelinePerformanceLoopBoundaries" role="group" aria-label="Loop boundaries">
        <button type="button" title="Set loop A at playhead" onClick={() => setLoopBoundary("a")}>A</button>
        <button type="button" title="Set loop B at playhead" onClick={() => setLoopBoundary("b")}>B</button>
      </div>
      <TimelineBankPanel
        timelines={props.timelines}
        activeTimelineId={props.activeTimelineId}
        bpm={props.bpm}
        followRuntime={props.followRuntime}
        onCreate={props.onCreateTimeline}
        onDuplicate={props.onDuplicateTimeline}
        onRemove={props.onRemoveTimeline}
        onReorder={props.onReorderTimelines}
        onSelect={props.onSelectTimeline}
        onSetFollow={props.onSetFollow}
      />
      <details class="timelineGuideAudioEditor" data-timeline-guide-audio-editor>
        <summary>
          <span>Guide Audio</span>
          <output classList={{ fault: Boolean(props.guideAudioStatus.lastError) }} role="status">
            {props.guideAudioStatus.lastError ? "Fault" : props.guideAudioStatus.enabled ? "Ready" : "Muted"}
          </output>
        </summary>
        <div class="timelineGuideAudioEditorBody">
          <label class="toggleRow">
            <input
              type="checkbox"
              checked={props.guideAudioStatus.enabled}
              onChange={(event) => void props.onConfigureGuideAudio(event.currentTarget.checked, props.guideAudioStatus.gain, props.guideAudioStatus.requestedDeviceName ?? null)}
            />
            <span>Guide monitor bus</span>
          </label>
          <label>
            <span>Guide gain</span>
            <input
              type="range"
              min="0"
              max="2"
              step="0.05"
              value={props.guideAudioStatus.gain}
              onChange={(event) => void props.onConfigureGuideAudio(props.guideAudioStatus.enabled, Number(event.currentTarget.value), props.guideAudioStatus.requestedDeviceName ?? null)}
            />
            <output class="tabularNums" data-no-localize>{Math.round(props.guideAudioStatus.gain * 100)}%</output>
          </label>
          <label>
            <span>Guide device</span>
            <select
              value={props.guideAudioStatus.requestedDeviceName ?? ""}
              onChange={(event) => void props.onConfigureGuideAudio(props.guideAudioStatus.enabled, props.guideAudioStatus.gain, event.currentTarget.value || null)}
            >
              <option value="">Default output</option>
              <Show when={props.guideAudioStatus.requestedDeviceName && !props.guideAudioDevices.includes(props.guideAudioStatus.requestedDeviceName)}>
                <option value={props.guideAudioStatus.requestedDeviceName ?? ""}>{props.guideAudioStatus.requestedDeviceName}</option>
              </Show>
              <For each={props.guideAudioDevices}>{(device) => <option value={device}>{device}</option>}</For>
            </select>
          </label>
          <Show when={props.guideAudioStatus.lastSpokenLabel}>
            {(label) => <output class="timelineGuideLastSpoken">Last: <span data-no-localize>{label()}</span></output>}
          </Show>
          <Show when={props.guideAudioStatus.lastError}>
            {(error) => <p class="inlineError" role="alert">{error()}</p>}
          </Show>
        </div>
      </details>
      <details class="timelinePhaseEditor" data-timeline-phase-editor>
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
                    <option value="intro">Intro</option><option value="verse">Verse</option><option value="pre_chorus">Pre-Chorus</option><option value="chorus">Chorus</option><option value="bridge">Bridge</option><option value="breakdown">Breakdown</option><option value="outro">Outro</option><option value="custom">Custom</option>
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
    </div>
  );
}
