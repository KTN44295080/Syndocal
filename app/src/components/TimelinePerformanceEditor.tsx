import { createEffect, createSignal, For, Show } from "solid-js";
import type {
  TimelineFollowRuntimeSummary,
  TimelineFollowAbortFocusFence,
  TimelineFollowSummary,
  MachineTimelineCueAudioSettingsV1,
  TimelineCueAudioStatus,
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
  followAbortBusy: boolean;
  followAbortFocusFence: TimelineFollowAbortFocusFence;
  loopRegion: TimelineLoopRegionSummary | null;
  cueAudioStatus: TimelineCueAudioStatus;
  cueAudioMutationBusy: boolean;
  cueAudioLocalError: string | null;
  onCreateTimeline: (label: string) => void | Promise<void>;
  onDuplicateTimeline: (timelineId: number) => void | Promise<void>;
  onRemoveTimeline: (timelineId: number) => void | Promise<void>;
  onReorderTimelines: (timelineIds: number[]) => void | Promise<void>;
  onSelectTimeline: (timelineId: number, play: boolean) => void | Promise<void>;
  onSetFollow: (follow: TimelineFollowSummary | null) => void | Promise<void>;
  onAbortFollow: () => Promise<boolean>;
  onSetPhases: (phases: TimelinePhaseSummary[]) => void | Promise<void>;
  onSetLoopRegion: (region: TimelineLoopRegionSummary | null) => void | Promise<void>;
  onConfigureCueAudio: (settings: MachineTimelineCueAudioSettingsV1) => void;
  onRefreshCueAudio: () => void | Promise<void>;
  onSeek: (timeMs: number) => void | Promise<void>;
}

const phaseTime = (timeMs: number) => `${(Math.max(0, timeMs) / 1_000).toFixed(3)}s`;

const admissibleCueAudioEndpoint = (
  endpoints: TimelineCueAudioStatus["endpoints"],
  requestedName: string | null,
) => {
  if (!requestedName) return null;
  const matches = endpoints.filter((endpoint) => endpoint.name === requestedName);
  return matches.length === 1
    && matches[0]?.selectable === true
    && matches[0]?.occurrences === 1
    ? matches[0]
    : null;
};

const cueAudioLifecycleText = (status: TimelineCueAudioStatus) => {
  switch (status.lifecycle) {
    case "missing_device": return "Missing device";
    case "ambiguous_device": return "Ambiguous device";
    case "topology_changed": return "Topology changed";
    case "stalled": return status.lastError?.includes("restart is required") ? "Restart required" : "Stalled";
    case "fault": return "Fault";
    case "disabled_by_project":
    case "waiting_for_program_output": return "Waiting";
    case "loading_settings": return "Loading settings";
    case "applying": return "Applying";
    case "running": return "Running";
    default: return status.lastError ? "Fault" : status.lifecycle;
  }
};

export function TimelinePerformanceEditor(props: TimelinePerformanceEditorProps) {
  const [pendingExplicitRoute, setPendingExplicitRoute] = createSignal(false);
  let authoringOutputSelect: HTMLSelectElement | undefined;
  createEffect(() => {
    if (props.cueAudioStatus.desiredSettings.route === "explicit_device") {
      setPendingExplicitRoute(false);
    }
  });
  const explicitRouteVisible = () => pendingExplicitRoute() || props.cueAudioStatus.desiredSettings.route === "explicit_device";
  const selectedOutputName = () => {
    const settings = props.cueAudioStatus.desiredSettings;
    return settings.route === "explicit_device"
      && admissibleCueAudioEndpoint(props.cueAudioStatus.endpoints, settings.device_name)
      ? settings.device_name ?? ""
      : "";
  };
  // Endpoint refreshes can reorder native options without changing the desired
  // value. Reapply the exact selectable option after Solid updates the list;
  // assigning the DOM property is silent and cannot configure a device.
  createEffect(() => {
    const settings = props.cueAudioStatus.desiredSettings;
    const endpoints = props.cueAudioStatus.endpoints;
    const statusRevision = props.cueAudioStatus.statusRevision;
    const desiredEndpoint = settings.route === "explicit_device"
      ? admissibleCueAudioEndpoint(endpoints, settings.device_name)
      : null;
    const desiredName = desiredEndpoint
      ? settings.device_name ?? ""
      : "";
    const select = authoringOutputSelect;
    if (!select) return;
    void statusRevision;
    const desiredOption = desiredName
      ? [...select.options].find((option) => option.value === desiredName && !option.disabled)
      : undefined;
    select.value = desiredOption?.value ?? "";
  });
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
      <details class="timelineCueAudioEditor" data-timeline-cue-audio-editor data-timeline-authoring-monitor>
        <summary>
          <span>Timeline authoring monitor</span>
          <output classList={{ fault: props.cueAudioStatus.lifecycle === "fault" || Boolean(props.cueAudioStatus.lastError) }} role="status">
            {cueAudioLifecycleText(props.cueAudioStatus)}
          </output>
        </summary>
        <div class="timelineCueAudioEditorBody">
          <p class="timelineCueAudioAuthority" role="status">In Normal mode, Follow Program uses the normal Program output. Explicit Device sends every Timeline media clip plus Guide and Click to the selected WDM endpoint. Show ASIO uses its separate PROGRAM/CUE routes and ignores this selection.</p>
          <p class="timelineCueAudioPolicy" role="note">A missing, ambiguous, or stale device stays silent until you refresh and select an exact output.</p>
          <label>
            <span>Monitor route</span>
            <select
              value={explicitRouteVisible() ? "explicit_device" : "follow_program"}
              disabled={props.cueAudioMutationBusy}
              onChange={(event) => {
                const settings = props.cueAudioStatus.desiredSettings;
                if (event.currentTarget.value === "follow_program") {
                  setPendingExplicitRoute(false);
                  props.onConfigureCueAudio({ ...settings, route: "follow_program", device_name: null, topology_fingerprint: null });
                  return;
                }
                setPendingExplicitRoute(true);
              }}
            >
              <option value="follow_program">Follow Program</option>
              <option value="explicit_device">Explicit Device</option>
            </select>
          </label>
          <Show when={explicitRouteVisible()}>
            <label>
              <span>Authoring output device</span>
              <select
                data-timeline-cue-audio-output
                ref={(element) => { authoringOutputSelect = element; }}
                value={selectedOutputName()}
                disabled={props.cueAudioMutationBusy}
                onChange={(event) => {
                  const endpoint = admissibleCueAudioEndpoint(props.cueAudioStatus.endpoints, event.currentTarget.value);
                  const fingerprint = props.cueAudioStatus.observedTopologyFingerprint;
                  if (!endpoint || !fingerprint) return;
                  props.onConfigureCueAudio({
                    ...props.cueAudioStatus.desiredSettings,
                    route: "explicit_device",
                    device_name: endpoint.name,
                    topology_fingerprint: fingerprint,
                  });
                }}
              >
                <option value="" disabled>Select an exact output…</option>
                <Show when={props.cueAudioStatus.desiredSettings.device_name && !props.cueAudioStatus.endpoints.some((endpoint) => endpoint.name === props.cueAudioStatus.desiredSettings.device_name)}>
                  <option value={props.cueAudioStatus.desiredSettings.device_name ?? ""} disabled>{props.cueAudioStatus.desiredSettings.device_name} (missing)</option>
                </Show>
                <For each={props.cueAudioStatus.endpoints}>
                  {(endpoint) => (
                    <option value={endpoint.name} disabled={admissibleCueAudioEndpoint(props.cueAudioStatus.endpoints, endpoint.name) === null}>
                      {endpoint.occurrences > 1 ? `${endpoint.name} (${endpoint.occurrences} matching outputs; ambiguous)` : endpoint.name}
                    </option>
                  )}
                </For>
              </select>
            </label>
          </Show>
          <label>
            <span>Click gain</span>
            <input
              type="range"
              min="0"
              max="2"
              step="0.05"
              value={props.cueAudioStatus.desiredSettings.click_gain}
              disabled={props.cueAudioMutationBusy}
              onChange={(event) => props.onConfigureCueAudio({ ...props.cueAudioStatus.desiredSettings, click_gain: Number(event.currentTarget.value) })}
            />
            <output class="tabularNums" data-no-localize>{Math.round(props.cueAudioStatus.desiredSettings.click_gain * 100)}%</output>
          </label>
          <label>
            <span>Guide gain</span>
            <input
              type="range"
              min="0"
              max="2"
              step="0.05"
              value={props.cueAudioStatus.desiredSettings.guide_gain}
              disabled={props.cueAudioMutationBusy}
              onChange={(event) => props.onConfigureCueAudio({ ...props.cueAudioStatus.desiredSettings, guide_gain: Number(event.currentTarget.value) })}
            />
            <output class="tabularNums" data-no-localize>{Math.round(props.cueAudioStatus.desiredSettings.guide_gain * 100)}%</output>
          </label>
          <button type="button" onClick={() => void props.onRefreshCueAudio()} disabled={props.cueAudioMutationBusy}>
            Refresh outputs
          </button>
          <output class="timelineCueAudioRuntime" role="status">
            <Show when={props.cueAudioMutationBusy} fallback={<><span>{cueAudioLifecycleText(props.cueAudioStatus)}</span><span class="tabularNums" data-no-localize>{` · rev ${props.cueAudioStatus.settingsRevision} · frame ${props.cueAudioStatus.nextOutputFrame}`}</span></>}>
              Applying…
            </Show>
          </output>
          <Show when={props.cueAudioStatus.resolvedDeviceName}>
            {(name) => <output class="timelineCueAudioResolved">Authoring output: <span data-no-localize>{name()}</span></output>}
          </Show>
          <Show when={props.cueAudioStatus.requestedTopologyFingerprint !== props.cueAudioStatus.observedTopologyFingerprint && props.cueAudioStatus.desiredSettings.route === "explicit_device"}>
            <p class="inlineError" role="alert">Output topology changed. Reselect the device.</p>
          </Show>
          <Show when={props.cueAudioStatus.lastError}>
            {(error) => <p class="inlineError" role="alert">Backend fault: <span data-no-localize>{error()}</span></p>}
          </Show>
          <Show when={props.cueAudioLocalError}>
            {(error) => <p class="inlineError" role="alert">Local IPC error: <span data-no-localize>{error()}</span></p>}
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
    </div>
  );
}
