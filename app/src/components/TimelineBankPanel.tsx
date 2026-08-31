import { createMemo, createSignal, For, Show } from "solid-js";
import type {
  TimelineFollowRuntimeSummary,
  TimelineFollowAbortFocusFence,
  TimelineFollowSummary,
  TimelineSnapshot,
  VideoClipTakeKind,
  VideoLayerTransitionCurve,
} from "../types";

interface TimelineBankPanelProps {
  timelines: TimelineSnapshot[];
  activeTimelineId: number;
  bpm: number;
  followRuntime: TimelineFollowRuntimeSummary;
  followAbortBusy: boolean;
  followAbortFocusFence: TimelineFollowAbortFocusFence;
  onCreate: (label: string) => void | Promise<void>;
  onDuplicate: (timelineId: number) => void | Promise<void>;
  onRemove: (timelineId: number) => void | Promise<void>;
  onReorder: (timelineIds: number[]) => void | Promise<void>;
  onSelect: (timelineId: number, play: boolean) => void | Promise<void>;
  onSetFollow: (follow: TimelineFollowSummary | null) => void | Promise<void>;
  onAbortFollow: () => Promise<boolean>;
}

const timelineId = (timeline: TimelineSnapshot) => timeline.id ?? 0;
const timelineLabel = (timeline: TimelineSnapshot, index: number) =>
  timeline.label?.trim() || `Timeline ${index + 1}`;

export function TimelineBankPanel(props: TimelineBankPanelProps) {
  const [newLabel, setNewLabel] = createSignal("");
  const activeIndex = createMemo(() =>
    props.timelines.findIndex((timeline) => timelineId(timeline) === props.activeTimelineId));
  const activeTimeline = createMemo(() => props.timelines[activeIndex()] ?? null);
  const nextTimeline = createMemo(() => props.timelines[activeIndex() + 1] ?? null);
  const currentFollow = createMemo(() => activeTimeline()?.follow ?? null);
  const defaultFollow = (): TimelineFollowSummary | null => {
    const next = nextTimeline();
    if (!next || timelineId(next) === 0) return null;
    return {
      enabled: true,
      next_timeline_id: timelineId(next),
      duration: { unit: "Milliseconds", value_milliunits: 1_000 },
      curve: "ease_in_out",
      video_kind: "Crossfade",
      lighting_policy: "linear_merge",
      destination_bpm: props.bpm,
      preroll_ms: 1_000,
      trans_cadence_bars: 4,
      fault_policy: "hold",
      hold_first_destination_measure: false,
      destination_start_mode: "play",
    };
  };
  const followIsCut = () => (currentFollow()?.video_kind ?? "Crossfade") === "Cut";
  const followDestinationHoldEnabled = () =>
    !followIsCut() && (currentFollow()?.hold_first_destination_measure ?? false);
  const followDestinationWaitsForPedal = () =>
    !followIsCut() && (currentFollow()?.destination_start_mode ?? "play") === "wait_for_pedal";
  const followDurationLocked = () =>
    followDestinationHoldEnabled() || followDestinationWaitsForPedal();
  const updateFollow = (patch: Partial<TimelineFollowSummary>) => {
    const base = currentFollow() ?? defaultFollow();
    if (!base) return;
    const nextFollow: TimelineFollowSummary = {
      ...base,
      ...patch,
      next_timeline_id: timelineId(nextTimeline()!),
    };
    if (nextFollow.video_kind === "Cut") {
      nextFollow.hold_first_destination_measure = false;
      nextFollow.destination_start_mode = "play";
    } else if (nextFollow.destination_start_mode === "wait_for_pedal") {
      nextFollow.hold_first_destination_measure = false;
    }
    void props.onSetFollow(nextFollow);
  };
  const reorder = (index: number, direction: -1 | 1) => {
    const target = index + direction;
    if (target < 0 || target >= props.timelines.length) return;
    const ids = props.timelines.map(timelineId);
    [ids[index], ids[target]] = [ids[target], ids[index]];
    void props.onReorder(ids);
  };
  const createTimeline = () => {
    const label = newLabel().trim();
    if (!label) return;
    setNewLabel("");
    void props.onCreate(label);
  };
  const removeTimeline = (timeline: TimelineSnapshot, index: number) => {
    if (props.timelines.length <= 1) return;
    const label = timelineLabel(timeline, index);
    if (!window.confirm(`Remove ${label}? This can be undone from Project History.`)) return;
    void props.onRemove(timelineId(timeline));
  };
  const runtimeLabel = createMemo(() => {
    const runtime = props.followRuntime;
    const status = runtime.status === "pending" ? "armed" : runtime.status;
    if (status === "transitioning" || status === "settling") {
      return `${status} ${Math.round(runtime.progress_millis / 10)}%`;
    }
    return status;
  });
  const runtimeSourceTarget = createMemo(() => {
    const runtime = props.followRuntime;
    const labelFor = (id: number | null | undefined) => {
      if (id === null || id === undefined) return "—";
      const index = props.timelines.findIndex((timeline) => timelineId(timeline) === id);
      return index >= 0 ? timelineLabel(props.timelines[index], index) : `Timeline ${id}`;
    };
    return `${labelFor(runtime.source_timeline_id)} → ${labelFor(runtime.target_timeline_id)}`;
  });
  const canAbort = createMemo(() => props.followRuntime.status !== "idle" && props.followRuntime.generation > 0);
  const abortFollow = async (invokingButton: HTMLButtonElement) => {
    if (props.followAbortBusy || !canAbort()) return;
    const applied = await props.onAbortFollow();
    props.followAbortFocusFence.schedule(applied, () => invokingButton.focus(), requestAnimationFrame);
  };

  return (
    <details class="timelineBankPanel" data-timeline-bank>
      <summary>
        <span>Timeline Bank</span>
        <strong data-no-localize>{timelineLabel(activeTimeline() ?? {}, Math.max(0, activeIndex()))}</strong>
        <output class={`timelineFollowState ${props.followRuntime.status}`} aria-live="polite" data-timeline-follow-runtime-badge>
          {runtimeLabel()}
        </output>
      </summary>
      <div class="timelineBankBody">
        <ol class="timelineBankList" aria-label="Timeline bank order">
          <For each={props.timelines}>
            {(timeline, index) => {
              const id = () => timelineId(timeline);
              const active = () => id() === props.activeTimelineId;
              return (
                <li classList={{ active: active() }}>
                  <button
                    type="button"
                    class="timelineBankSelect"
                    aria-current={active() ? "true" : undefined}
                    onClick={() => void props.onSelect(id(), false)}
                  >
                    <span data-no-localize>{timelineLabel(timeline, index())}</span>
                    <small data-no-localize>{Math.round(timeline.duration_ms / 1_000)}s</small>
                  </button>
                  <button type="button" aria-label="Move Timeline earlier" disabled={index() === 0} onClick={() => reorder(index(), -1)}>↑</button>
                  <button type="button" aria-label="Move Timeline later" disabled={index() === props.timelines.length - 1} onClick={() => reorder(index(), 1)}>↓</button>
                  <button type="button" aria-label="Duplicate Timeline" onClick={() => void props.onDuplicate(id())}>Duplicate</button>
                  <button type="button" aria-label="Remove Timeline" disabled={props.timelines.length <= 1} onClick={() => removeTimeline(timeline, index())}>Remove</button>
                </li>
              );
            }}
          </For>
        </ol>
        <div class="timelineBankCreate">
          <label>
            New Timeline
            <input value={newLabel()} placeholder="e.g. Encore" onInput={(event) => setNewLabel(event.currentTarget.value)} onKeyDown={(event) => {
              if (event.key === "Enter") createTimeline();
            }} />
          </label>
          <button type="button" class="primary" disabled={!newLabel().trim()} onClick={createTimeline}>Create</button>
        </div>
        <fieldset class="timelineFollowEditor" disabled={!nextTimeline()}>
          <legend>Follow to next Timeline</legend>
          <label class="toggleRow">
            <input
              type="checkbox"
              checked={currentFollow()?.enabled ?? false}
              onChange={(event) => void props.onSetFollow(event.currentTarget.checked ? defaultFollow() : null)}
            />
            Follow to next Timeline
          </label>
          <label>
            Video transition
            <select value={currentFollow()?.video_kind ?? "Crossfade"} onChange={(event) => {
              const videoKind = event.currentTarget.value as VideoClipTakeKind;
              updateFollow({
                video_kind: videoKind,
                duration: {
                  unit: "Milliseconds",
                  value_milliunits: videoKind === "Cut" ? 0 : Math.max(1, currentFollow()?.duration.value_milliunits ?? 1_000),
                },
              });
            }}>
              <option value="Cut">Cut</option>
              <option value="Crossfade">Crossfade</option>
              <option value="Dip">Dip</option>
            </select>
          </label>
          <label>
            Destination start
            <select
              data-timeline-follow-destination-start
              value={currentFollow()?.destination_start_mode ?? "play"}
              disabled={followIsCut()}
              onChange={(event) => updateFollow({
                destination_start_mode: event.currentTarget.value as TimelineFollowSummary["destination_start_mode"],
              })}
            >
              <option value="play">Play immediately</option>
              <option value="wait_for_pedal">Wait for Pedal 1</option>
            </select>
          </label>
          <label class="toggleRow">
            <input
              type="checkbox"
              data-timeline-follow-hold
              checked={followDestinationHoldEnabled()}
              disabled={followIsCut() || followDestinationWaitsForPedal()}
              onChange={(event) => updateFollow({ hold_first_destination_measure: event.currentTarget.checked })}
            />
            Hold destination first measure until pedal release
          </label>
          <small class="timelineFollowHoldHint" data-timeline-follow-hold-hint>
            {followDestinationWaitsForPedal()
              ? "Transition uses one source measure, then the destination waits at its start until Pedal 1."
              : "Transition uses one source measure, then the destination first measure loops until F13."}
          </small>
          <label>
            Curve
            <select value={currentFollow()?.curve ?? "ease_in_out"} onChange={(event) => updateFollow({ curve: event.currentTarget.value as VideoLayerTransitionCurve })}>
              <option value="linear">Linear</option>
              <option value="ease_in_out">Ease in/out</option>
            </select>
          </label>
          <label>
            Fade (ms)
            <input type="number" min="0" step="50" value={currentFollow()?.duration.value_milliunits ?? 1_000} disabled={followIsCut() || followDurationLocked()} onChange={(event) => updateFollow({ duration: { unit: "Milliseconds", value_milliunits: Math.max(1, Math.round(Number(event.currentTarget.value) || 1)) } })} />
          </label>
          <label>
            Preroll (ms)
            <input type="number" min="0" step="50" value={currentFollow()?.preroll_ms ?? 1_000} onChange={(event) => updateFollow({ preroll_ms: Math.max(0, Math.round(Number(event.currentTarget.value) || 0)) })} />
          </label>
          <label>
            Destination BPM
            <input type="number" min="20" max="300" step="0.1" value={currentFollow()?.destination_bpm ?? props.bpm} onChange={(event) => updateFollow({ destination_bpm: Math.max(20, Math.min(300, Number(event.currentTarget.value) || props.bpm)) })} />
          </label>
          <label>
            Trans cadence (bars)
            <input type="number" min="1" max="64" value={currentFollow()?.trans_cadence_bars ?? 4} onChange={(event) => updateFollow({ trans_cadence_bars: Math.max(1, Math.min(64, Math.round(Number(event.currentTarget.value) || 4))) })} />
          </label>
          <label>
            Lighting
            <select value={currentFollow()?.lighting_policy ?? "linear_merge"} onChange={(event) => updateFollow({ lighting_policy: event.currentTarget.value as TimelineFollowSummary["lighting_policy"] })}>
              <option value="linear_merge">Crossfade</option>
              <option value="hold_then_cut">Hold then cut</option>
            </select>
          </label>
        </fieldset>
        <section class="timelineFollowRuntimeDetails" aria-label="Timeline Follow runtime" data-timeline-follow-runtime-details>
          <div>
            <strong>Runtime</strong>
            <output class={`timelineFollowState ${props.followRuntime.status}`} aria-live="polite">{runtimeLabel()}</output>
          </div>
          <p data-timeline-follow-source-target>{runtimeSourceTarget()}</p>
          <progress
            aria-label="Timeline Follow progress"
            max="1000"
            value={Math.max(0, Math.min(1000, props.followRuntime.settlement?.progress_millis ?? props.followRuntime.progress_millis))}
          />
          <Show when={props.followRuntime.admission_reason}>
            {(reason) => <p>Admission: {reason()}</p>}
          </Show>
          <Show when={props.followRuntime.outcome}>
            {(outcome) => {
              const current = outcome();
              return <p>Outcome: {current.kind === "aborted" ? `aborted (${current.reason})` : current.kind}</p>;
            }}
          </Show>
          <Show when={props.followRuntime.settlement}>
            {(settlement) => (
              <ul aria-label="Timeline Follow settlement domains" data-timeline-follow-domains>
                <For each={settlement().domains}>
                  {(domain) => <li data-domain={domain.domain} data-state={domain.state}>{domain.domain}: {domain.state}{domain.fault ? ` — ${domain.fault}` : ""}</li>}
                </For>
              </ul>
            )}
          </Show>
          <Show when={props.followRuntime.fault ?? props.followRuntime.settlement?.fault}>
            {(fault) => <p class="inlineError" role="alert">{fault()}</p>}
          </Show>
          <Show when={canAbort()}>
            <button
              type="button"
              class="danger"
              style={{ "min-height": "44px", "min-width": "44px" }}
              aria-label="Abort Timeline Follow"
              aria-busy={props.followAbortBusy ? "true" : undefined}
              disabled={props.followAbortBusy || props.followRuntime.status === "aborting"}
              data-timeline-follow-abort
              onClick={(event) => void abortFollow(event.currentTarget)}
            >
              {props.followAbortBusy ? "Aborting…" : "Abort Follow"}
            </button>
          </Show>
        </section>
      </div>
    </details>
  );
}
