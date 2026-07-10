import { For, Show } from "solid-js";
import type { CueMetadataDraft } from "../editorDrafts";
import type { ActiveFadeSummary, CueSummary, TimelineCueEventSummary, TimelineTrackKind } from "../types";
import { CueCapturePreviewPanel, type CueCapturePreviewModel } from "./CueCapturePreviewPanel";

export type CueCaptureScopeMode = "all" | "lighting" | "selectedFixture" | "selectedGroup" | "video";

interface CueManagementPanelProps {
  mode: "edit" | "live";
  cues: CueSummary[];
  activeCueId: number | null | undefined;
  activeFade: ActiveFadeSummary | null | undefined;
  timelinePositionMs: number;
  timelineTrack: TimelineTrackKind;
  cueLabel: string;
  cueFadeMs: number;
  cueCaptureScope: CueCaptureScopeMode;
  cueCaptureScopeError: string | null;
  hasCueSources: boolean;
  cueCapturePreview: CueCapturePreviewModel;
  stageViewBoxSize: number;
  stageOrigin: { x: number; z: number };
  selectedFixtureId: number | null;
  timelinePlacementNudgeMs: number;
  cueMetadataDraft: (cue: CueSummary) => CueMetadataDraft;
  cueTimelinePlacementsForCue: (cueId: number) => TimelineCueEventSummary[];
  onCueLabel: (value: string) => void;
  onCueFadeMs: (value: number) => void;
  onCueCaptureScope: (scope: CueCaptureScopeMode) => void;
  onCreateCue: () => void | Promise<void>;
  onSelectFixture: (fixtureId: number) => void;
  onTriggerPreviousCue: () => void | Promise<void>;
  onTriggerNextCue: () => void | Promise<void>;
  onSetCueFadePaused: (paused: boolean) => void | Promise<void>;
  onUpdateCueMetadataDraft: (cue: CueSummary, patch: Partial<CueMetadataDraft>) => void;
  onMoveCue: (cueId: number, delta: -1 | 1) => void | Promise<void>;
  onSetCueMetadata: (cue: CueSummary) => void | Promise<void>;
  onDuplicateCue: (cue: CueSummary) => void | Promise<void>;
  onUpdateCue: (cueId: number, label: string, fadeMs: number) => void | Promise<void>;
  onTriggerCue: (cueId: number) => void | Promise<void>;
  onAddTimelineCueEventAt: (
    cueId: number | null,
    timeMs: number,
    track: TimelineTrackKind,
    nextDraftTime?: boolean,
  ) => void | Promise<void>;
  onRemoveCue: (cueId: number) => void | Promise<void>;
  onSeekTimeline: (timeMs: number) => void | Promise<void>;
  onMoveTimelineCueEvent: (event: TimelineCueEventSummary, deltaMs: number) => void | Promise<void>;
  onRemoveTimelineEvent: (eventId: number) => void | Promise<void>;
}

export function CueManagementPanel(props: CueManagementPanelProps) {
  return (
    <div class={props.mode === "live" ? "cuePanel cuePanelLive" : "cuePanel"}>
      <div class="panelHeader">
        <h2>Cues</h2>
        <span>{props.cues.length}</span>
      </div>
      <div class="cueForm">
        <label>
          Label
          <input value={props.cueLabel} onInput={(event) => props.onCueLabel(event.currentTarget.value)} />
        </label>
        <label>
          Fade ms
          <input type="number" min="0" value={props.cueFadeMs} onInput={(event) => props.onCueFadeMs(Number(event.currentTarget.value))} />
        </label>
        <label>
          Scope
          <select value={props.cueCaptureScope} onInput={(event) => props.onCueCaptureScope(event.currentTarget.value as CueCaptureScopeMode)}>
            <option value="all">Lighting + Video</option>
            <option value="lighting">Lighting Only</option>
            <option value="selectedFixture">Selected Fixture</option>
            <option value="selectedGroup">Selected Group</option>
            <option value="video">Video Only</option>
          </select>
        </label>
        <Show when={props.cueCaptureScopeError}>
          {(error) => <span class="cueScopeHint invalid">{error()}</span>}
        </Show>
        <button class="primary" onClick={() => void props.onCreateCue()} disabled={!props.hasCueSources || Boolean(props.cueCaptureScopeError)}>
          Store Cue
        </button>
      </div>
      <CueCapturePreviewPanel
        preview={props.cueCapturePreview}
        invalid={Boolean(props.cueCaptureScopeError)}
        stageViewBoxSize={props.stageViewBoxSize}
        stageOrigin={props.stageOrigin}
        selectedFixtureId={props.selectedFixtureId}
        onSelectFixture={props.onSelectFixture}
      />
      <div class="buttonRow">
        <button onClick={() => void props.onTriggerPreviousCue()} disabled={props.cues.length === 0}>
          Back
        </button>
        <button class="primary" onClick={() => void props.onTriggerNextCue()} disabled={props.cues.length === 0}>
          GO
        </button>
        <button
          onClick={() => void props.onSetCueFadePaused(!props.activeFade?.paused)}
          disabled={!props.activeFade}
        >
          {props.activeFade?.paused ? "Resume" : "Pause"}
        </button>
      </div>
      <Show when={props.activeFade}>
        {(fade) => (
          <div class="fadeMeter">
            <span>{fade().paused ? "Paused " : ""}{Math.round(fade().progress * 100)}%</span>
            <div>
              <i style={{ width: `${Math.round(fade().progress * 100)}%` }} />
            </div>
          </div>
        )}
      </Show>
      <div class="cueList">
        <Show when={props.cues.length === 0}>
          <p class="empty">{props.hasCueSources ? "No cues. Choose a scope, then Store Cue." : "No cues. Patch fixtures or add a video layer, then Store Cue."}</p>
        </Show>
        <For each={props.cues}>
          {(cue, index) => {
            const draft = () => props.cueMetadataDraft(cue);
            const placements = () => props.cueTimelinePlacementsForCue(cue.id);
            return (
              <div class={cue.id === props.activeCueId ? "cueItem active" : "cueItem"}>
                <div class="cueMetaLine">
                  <strong>{cue.label}</strong>
                  <span>
                    {cue.targets.length} fixture(s) / {cue.video_targets.length} video /{" "}
                    {cue.video_output_targets.length} output(s) / {cue.fade_ms}ms
                  </span>
                </div>
                <div class="cueEditRow">
                  <input value={draft().label} onInput={(event) => props.onUpdateCueMetadataDraft(cue, { label: event.currentTarget.value })} />
                  <input
                    type="number"
                    min="0"
                    value={draft().fade_ms}
                    onInput={(event) => props.onUpdateCueMetadataDraft(cue, { fade_ms: Number(event.currentTarget.value) })}
                  />
                </div>
                <div class="cueActionRow">
                  <button class="cueEditOnly" onClick={() => void props.onMoveCue(cue.id, -1)} disabled={index() === 0}>
                    Up
                  </button>
                  <button class="cueEditOnly" onClick={() => void props.onMoveCue(cue.id, 1)} disabled={index() === props.cues.length - 1}>
                    Down
                  </button>
                  <button class="cueEditOnly" onClick={() => void props.onSetCueMetadata(cue)}>
                    Save
                  </button>
                  <button class="cueEditOnly" onClick={() => void props.onDuplicateCue(cue)}>
                    Copy
                  </button>
                  <button
                    class="cueEditOnly"
                    onClick={() => void props.onUpdateCue(cue.id, draft().label, draft().fade_ms)}
                    disabled={!props.hasCueSources || Boolean(props.cueCaptureScopeError)}
                  >
                    Update
                  </button>
                  <button class="cueLiveGo" onClick={() => void props.onTriggerCue(cue.id)}>GO</button>
                  <button class="cueEditOnly" onClick={() => void props.onAddTimelineCueEventAt(cue.id, props.timelinePositionMs, props.timelineTrack, false)}>
                    At Playhead
                  </button>
                  <button class="cueEditOnly" onClick={() => void props.onRemoveCue(cue.id)}>Remove</button>
                </div>
                <Show when={placements().length > 0}>
                  <div class="cueTimelinePlacements">
                    <For each={placements()}>
                      {(placement) => (
                        <span
                          class={[
                            "cueTimelinePlacementChip",
                            placement.track === "Lighting" ? "lighting" : "video",
                            placement.time_ms < props.timelinePositionMs ? "past" : "",
                          ].filter(Boolean).join(" ")}
                        >
                          <button
                            class="cueTimelinePlacementMain"
                            title={`Seek to ${placement.time_ms} ms / ${placement.track}`}
                            onClick={() => void props.onSeekTimeline(placement.time_ms)}
                          >
                            <b>{placement.track === "Lighting" ? "L" : "V"}</b>
                            <small>{placement.time_ms} ms</small>
                          </button>
                          <button
                            class="cueTimelinePlacementMove"
                            title={`Nudge ${props.timelinePlacementNudgeMs} ms. Shift-click nudges left.`}
                            onClick={(event) =>
                              void props.onMoveTimelineCueEvent(
                                placement,
                                event.shiftKey ? -props.timelinePlacementNudgeMs : props.timelinePlacementNudgeMs,
                              )
                            }
                          >
                            &gt;
                          </button>
                          <button
                            class="cueTimelinePlacementRemove"
                            title="Remove timeline placement"
                            onClick={() => void props.onRemoveTimelineEvent(placement.id)}
                          >
                            x
                          </button>
                        </span>
                      )}
                    </For>
                  </div>
                </Show>
              </div>
            );
          }}
        </For>
      </div>
    </div>
  );
}
