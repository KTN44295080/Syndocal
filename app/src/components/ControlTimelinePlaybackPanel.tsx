import { createMemo, createSignal, For, Show } from "solid-js";
import { formatShowTimecode } from "../clockDisplay";
import {
  clampSeekMs,
  currentCueTiming,
  formatCountdown,
  nextCueTiming,
} from "../controlTimelinePlayback";
import type { ActiveFadeSummary, CueListSummary, CueSummary, TimelineCueEventSummary } from "../types";
import { WorkspacePaneHeader } from "./WorkspacePaneHeader";
import "./ControlTimelinePlayback.css";

export interface ControlTimelinePlaybackPanelProps {
  cues: readonly CueSummary[];
  cueLists: readonly CueListSummary[];
  activeCueId: number | null;
  nextCueId: number | null;
  activeFade: ActiveFadeSummary | null;
  /** The Timeline the transport commands currently act on (root show timeline or an open child). */
  timelineEvents: readonly TimelineCueEventSummary[];
  timelinePositionMs: number;
  timelineDurationMs: number;
  timelinePlaying: boolean;
  /** Set while the transport is acting on a Scene's child timeline. */
  childTimelineLabel: string | null;
  blackout: boolean;
  videoBlackout: boolean;
  safetyBlackoutEngaged: boolean;
  /** Why this machine cannot change output state (for example Standby), else null. */
  blackoutBlockedReason: string | null;
  cueIdentity: (cue: CueSummary) => string;
  onGo: () => void;
  onBack: () => void;
  onRelease: () => void;
  onTriggerCue: (cueId: number) => void;
  onToggleFadePause: () => void;
  onPlay: () => void;
  onPause: () => void;
  onSeek: (positionMs: number) => void;
  onSetBlackout: (enabled: boolean) => void;
  onSetVideoBlackout: (enabled: boolean) => void;
  onSetAllBlackout: (enabled: boolean) => void;
  /** Absent in a pane window, which cannot navigate the main window's Edit workspace. */
  onOpenTimelineEditor?: () => void;
}

// The same global shortcuts as the top bar and Control keyboard map (appShortcutActions.ts);
// this surface adds no key bindings of its own.
const shortcutLegend: readonly (readonly [string, string])[] = [
  ["Space", "GO"],
  ["Shift+Space", "Back"],
  ["1-0", "Cue"],
  ["T", "Timeline"],
  ["P", "Fade"],
  ["B", "DMX BO"],
  ["V", "Video BO"],
];

export function ControlTimelinePlaybackPanel(props: ControlTimelinePlaybackPanelProps) {
  const cueById = createMemo(() => new Map(props.cues.map((cue) => [cue.id, cue] as const)));
  const cueListLabelById = createMemo(() => new Map(props.cueLists.map((list) => [list.id, list.label] as const)));
  const activeCue = () => (props.activeCueId === null ? null : cueById().get(props.activeCueId) ?? null);
  const nextCue = () => (props.nextCueId === null ? null : cueById().get(props.nextCueId) ?? null);
  const current = createMemo(() => currentCueTiming(props.activeCueId, props.activeFade));
  const next = createMemo(() =>
    nextCueTiming(props.nextCueId, props.timelineEvents, props.timelinePositionMs, props.timelinePlaying));
  const hasCues = () => props.cues.length > 0;
  const allBlackout = () => props.blackout && props.videoBlackout;
  const remainingMs = () => Math.max(0, props.timelineDurationMs - props.timelinePositionMs);
  // A seek commits when the operator releases the slider; while dragging the
  // slider shows the requested position instead of fighting live updates.
  const [scrubMs, setScrubMs] = createSignal<number | null>(null);
  const shownPositionMs = () => scrubMs() ?? props.timelinePositionMs;
  const canTransport = () => props.timelinePlaying || props.timelineDurationMs > 0;
  const blocked = () => props.blackoutBlockedReason !== null;

  return (
    <section
      id="control-domain-panel-timeline"
      role="tabpanel"
      aria-labelledby="control-domain-timeline"
      class="panel controlPlaybackPanel controlPanel"
      data-workspace-pane="upper"
      data-control-timeline-playback
    >
      <WorkspacePaneHeader
        title="Timeline"
        titleId="control-playback-title"
        context="Playback"
        tools={
          <Show when={props.onOpenTimelineEditor}>
            {(open) => (
              <button type="button" data-playback-open-editor title="Edit Timeline" aria-label="Edit Timeline" onClick={() => open()()}>
                Edit Timeline
              </button>
            )}
          </Show>
        }
      />

      <div class="controlPlaybackBody">
        <div class="controlPlaybackColumn">
          <section class="controlPlaybackCard" aria-label="Cue transport" data-playback-cue-transport>
            <dl class="controlPlaybackReadout">
              <div data-playback-current>
                <dt>Current</dt>
                <dd>
                  <strong data-playback-current-label>
                    <Show when={activeCue()} fallback="None">{(cue) => <span data-no-localize>{cue().label}</span>}</Show>
                  </strong>
                  <small data-playback-current-timing data-timing-kind={current().kind}>
                    {(() => {
                      const timing = current();
                      if (timing.kind === "fading") {
                        return (
                          <>
                            <span>{timing.paused ? "Fade paused" : "Fading"}</span>{" "}
                            <b class="tabularNums" data-no-localize data-playback-countdown>{formatCountdown(timing.remainingMs)}</b>
                          </>
                        );
                      }
                      return timing.kind === "held" ? "Held" : "No cue running";
                    })()}
                  </small>
                </dd>
              </div>
              <div data-playback-next>
                <dt>Next</dt>
                <dd>
                  <strong data-playback-next-label>
                    <Show when={nextCue()} fallback="No cue">{(cue) => <span data-no-localize>{cue().label}</span>}</Show>
                  </strong>
                  <small data-playback-next-timing data-timing-kind={next().kind}>
                    {(() => {
                      const timing = next();
                      if (timing.kind === "timeline") {
                        return (
                          <>
                            <span>{timing.playing ? "Timeline fires in" : "On Timeline in"}</span>{" "}
                            <b class="tabularNums" data-no-localize data-playback-countdown>{formatCountdown(timing.remainingMs)}</b>
                          </>
                        );
                      }
                      return timing.kind === "manual" ? "Manual GO" : "Nothing queued";
                    })()}
                  </small>
                </dd>
              </div>
            </dl>
            <Show when={props.activeFade}>
              {(fade) => (
                <progress class="controlPlaybackFade" max="1" value={fade().progress} aria-label="Cue fade progress" data-playback-fade-progress />
              )}
            </Show>
            <div class="controlPlaybackActions" role="group" aria-label="Cue actions">
              <button type="button" data-playback-action="back" aria-keyshortcuts="Shift+Space" disabled={!hasCues()} onClick={props.onBack}>Back</button>
              <button type="button" class="controlPlaybackGo" data-playback-action="go" aria-keyshortcuts="Space G Enter" disabled={!hasCues()} onClick={props.onGo}>GO</button>
              <button type="button" data-playback-action="release" disabled={props.activeCueId === null} onClick={props.onRelease}>Release</button>
              <button
                type="button"
                data-playback-action="fade-pause" aria-keyshortcuts="P"
                aria-pressed={props.activeFade?.paused ?? false}
                disabled={!props.activeFade}
                onClick={props.onToggleFadePause}
              >
                {props.activeFade?.paused ? "Resume Fade" : "Pause Fade"}
              </button>
            </div>
          </section>

          <div class="controlPlaybackRow">
          <section class="controlPlaybackCard" aria-label="Timeline transport" data-playback-timeline-transport>
            <header class="controlPlaybackCardHeader">
              <strong>Timeline</strong>
              <span class="controlPlaybackBadge" data-playback-transport-state={props.timelinePlaying ? "playing" : "stopped"}>
                {props.timelinePlaying ? "PLAYING" : "STOPPED"}
              </span>
            </header>
            <Show when={props.childTimelineLabel}>
              {(label) => <p class="controlPlaybackNote" data-playback-child-timeline>Child timeline: <span data-no-localize>{label()}</span></p>}
            </Show>
            <div class="controlPlaybackClock">
              <output class="tabularNums" data-no-localize data-playback-position aria-label="Timeline position">{formatShowTimecode(shownPositionMs())}</output>
              <span class="controlPlaybackClockLength">
                <span>Length</span> <b class="tabularNums" data-no-localize>{formatShowTimecode(props.timelineDurationMs)}</b>
                {" "}<span>Remaining</span> <b class="tabularNums" data-no-localize data-playback-remaining>{formatShowTimecode(remainingMs())}</b>
              </span>
            </div>
            <input
              type="range"
              class="controlPlaybackSeek"
              data-playback-seek
              min="0"
              max={Math.max(1, props.timelineDurationMs)}
              step="10"
              value={Math.min(shownPositionMs(), Math.max(1, props.timelineDurationMs))}
              disabled={props.timelineDurationMs <= 0}
              aria-label="Timeline position"
              onInput={(event) => setScrubMs(Number(event.currentTarget.value))}
              onChange={(event) => {
                props.onSeek(clampSeekMs(Number(event.currentTarget.value), props.timelineDurationMs));
                setScrubMs(null);
              }}
            />
            <div class="controlPlaybackActions" role="group" aria-label="Timeline transport">
              <button
                type="button"
                data-playback-action="timeline-start"
                disabled={props.timelineDurationMs <= 0}
                onClick={() => props.onSeek(0)}
              >
                Return to start
              </button>
              <button
                type="button"
                class={props.timelinePlaying ? "active" : ""}
                data-playback-action="timeline-toggle" aria-keyshortcuts="T"
                aria-pressed={props.timelinePlaying}
                disabled={!canTransport()}
                onClick={() => (props.timelinePlaying ? props.onPause() : props.onPlay())}
              >
                {props.timelinePlaying ? "Pause Timeline" : "Play Timeline"}
              </button>
            </div>
          </section>

          <section class="controlPlaybackCard" aria-label="Blackout and safety" data-playback-safety>
            <header class="controlPlaybackCardHeader">
              <strong>Safety</strong>
              <span
                class="controlPlaybackBadge"
                classList={{ engaged: props.blackout || props.videoBlackout || props.safetyBlackoutEngaged }}
                data-playback-safety-state={props.safetyBlackoutEngaged ? "safety" : props.blackout || props.videoBlackout ? "blackout" : "live"}
              >
                {props.safetyBlackoutEngaged ? "Safety blackout" : props.blackout || props.videoBlackout ? "Blackout" : "Outputs live"}
              </span>
            </header>
            <Show when={props.blackoutBlockedReason}>
              {(reason) => <p class="controlPlaybackNote" role="status" data-playback-blackout-blocked>{reason()}</p>}
            </Show>
            <div class="controlPlaybackActions" role="group" aria-label="Blackout actions">
              <button
                type="button"
                class="topbarSafetyButton"
                classList={{ engaged: props.blackout, blocked: blocked() }}
                data-playback-action="dmx-blackout" aria-keyshortcuts="B"
                aria-disabled={blocked() ? "true" : undefined}
                aria-pressed={props.blackout}
                aria-label={props.blackout ? "Clear DMX Blackout" : "DMX Blackout"}
                onClick={() => props.onSetBlackout(!props.blackout)}
              >
                <span data-no-localize>DMX</span>
              </button>
              <button
                type="button"
                class="topbarSafetyButton"
                classList={{ engaged: props.videoBlackout, blocked: blocked() }}
                data-playback-action="video-blackout" aria-keyshortcuts="V"
                aria-disabled={blocked() ? "true" : undefined}
                aria-pressed={props.videoBlackout}
                aria-label={props.videoBlackout ? "Clear Video Blackout" : "Video Blackout"}
                onClick={() => props.onSetVideoBlackout(!props.videoBlackout)}
              >
                <span data-no-localize>VID</span>
              </button>
              <button
                type="button"
                class="topbarSafetyButton"
                classList={{ engaged: allBlackout(), blocked: blocked() }}
                data-playback-action="all-blackout"
                aria-disabled={blocked() ? "true" : undefined}
                aria-pressed={allBlackout()}
                aria-label={allBlackout() ? "Clear All Blackout" : "All Blackout"}
                onClick={() => props.onSetAllBlackout(!allBlackout())}
              >
                <span data-no-localize>ALL</span>
              </button>
            </div>
          </section>
          </div>
          <details class="controlPlaybackKeyDisclosure">
            <summary>Keyboard shortcuts</summary>
            <ul class="controlPlaybackKeys" aria-label="Keyboard shortcuts" data-playback-shortcut-legend>
              <For each={shortcutLegend}>
                {([key, label]) => <li><kbd>{key}</kbd> <span>{label}</span></li>}
              </For>
            </ul>
          </details>
        </div>

        <section class="controlPlaybackCard controlPlaybackCueList" aria-label="Cue list" data-playback-cue-list>
          <header class="controlPlaybackCardHeader">
            <strong>Cue list</strong>
            <span class="controlPlaybackCount" data-no-localize>{props.cues.length}</span>
          </header>
          <Show when={hasCues()} fallback={<p class="controlPlaybackNote" data-playback-no-cues>No cues</p>}>
            <ol class="controlPlaybackCues">
              <For each={props.cues}>
                {(cue) => {
                  const isActive = () => cue.id === props.activeCueId;
                  const isNext = () => cue.id === props.nextCueId && !isActive();
                  return (
                    <li>
                      <button
                        type="button"
                        class="controlPlaybackCue"
                        classList={{ active: isActive(), next: isNext() }}
                        style={{ "--identity": props.cueIdentity(cue) }}
                        data-playback-cue-id={cue.id}
                        aria-pressed={isActive()}
                        aria-current={isNext() ? "step" : undefined}
                        onClick={() => props.onTriggerCue(cue.id)}
                      >
                        <span class="controlPlaybackCueNumber tabularNums" data-no-localize>{cue.cue_number}</span>
                        <span class="controlPlaybackCueName">
                          <strong data-no-localize>{cue.label}</strong>
                          <small data-no-localize>{cueListLabelById().get(cue.cue_list_id) ?? ""}</small>
                        </span>
                        <span class="controlPlaybackCueFade tabularNums" data-no-localize>{formatCountdown(cue.fade_ms)}</span>
                        <Show when={isActive()}><span class="controlPlaybackBadge engaged">ACTIVE</span></Show>
                        <Show when={isNext()}><span class="controlPlaybackBadge">NEXT</span></Show>
                      </button>
                    </li>
                  );
                }}
              </For>
            </ol>
          </Show>
        </section>
      </div>
    </section>
  );
}
