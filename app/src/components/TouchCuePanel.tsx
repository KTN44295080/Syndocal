import { For, Show } from "solid-js";
import type { CueSummary, EngineSnapshot } from "../types";

type MaybePromise = void | Promise<void>;

export interface TouchCuePad {
  slot: string;
  index: number;
  cue: CueSummary | null;
}

interface TouchCuePanelProps {
  snapshot: EngineSnapshot;
  activeCue: CueSummary | null;
  nextCue: CueSummary | null;
  cuePads: TouchCuePad[];
  cuePadRangeLabel: string;
  cuePadBank: number;
  cuePadBankCount: number;
  onPreviousBank: () => void;
  onNextBank: () => void;
  onTriggerPreviousCue: () => MaybePromise;
  onTriggerNextCue: () => MaybePromise;
  onTriggerCue: (cueId: number) => MaybePromise;
  onSetCueFadePaused: (paused: boolean) => MaybePromise;
  onSetLightingMaster: (level: number) => MaybePromise;
  onSetVideoMasterOpacity: (level: number) => MaybePromise;
  onSetBlackout: (enabled: boolean) => MaybePromise;
  onSetVideoBlackout: (enabled: boolean) => MaybePromise;
  onSetAllBlackout: (enabled: boolean) => MaybePromise;
}

export function TouchCuePanel(props: TouchCuePanelProps) {
  return (
    <section class="panel touchPanel touchCuePanel">
      <div class="panelHeader">
        <h2>Touch Cues</h2>
        <Show when={props.activeCue} fallback={<span>Standby</span>}>
          {(cue) => <span data-no-localize>{cue().label}</span>}
        </Show>
      </div>
      <div class="touchGoDeck">
        <button onClick={() => void props.onTriggerPreviousCue()} disabled={props.snapshot.cues.length === 0}>
          Back
        </button>
        <button class="primary" onClick={() => void props.onTriggerNextCue()} disabled={props.snapshot.cues.length === 0}>
          GO
        </button>
        <button
          onClick={() => void props.onSetCueFadePaused(!props.snapshot.active_fade?.paused)}
          disabled={!props.snapshot.active_fade}
        >
          {props.snapshot.active_fade?.paused ? "Resume" : "Pause"}
        </button>
      </div>
      <div class="touchCueStatus">
        <div>
          <span>Active</span>
          <Show when={props.activeCue} fallback={<strong>None</strong>}>
            {(cue) => <strong data-no-localize>{cue().label}</strong>}
          </Show>
        </div>
        <div>
          <span>Next</span>
          <Show when={props.nextCue} fallback={<strong>None</strong>}>
            {(cue) => <strong data-no-localize>{cue().label}</strong>}
          </Show>
        </div>
      </div>
      <div class="liveCuePadHeader">
        <h3>Cue Pads</h3>
        <span>{props.cuePadRangeLabel}</span>
        <button onClick={props.onPreviousBank} disabled={props.cuePadBank === 0}>Prev</button>
        <button onClick={props.onNextBank} disabled={props.cuePadBank >= props.cuePadBankCount - 1}>Next</button>
      </div>
      <div class="touchCuePadGrid">
        <For each={props.cuePads}>
          {(pad) => (
            <button
              class={`liveCuePad ${pad.cue?.id === props.snapshot.active_cue_id ? "active" : ""} ${
                pad.cue?.id === props.nextCue?.id ? "next" : ""
              }`}
              disabled={!pad.cue}
              onClick={() => {
                if (pad.cue) void props.onTriggerCue(pad.cue.id);
              }}
            >
              <span>{pad.slot}</span>
              <Show when={pad.cue} fallback={<strong>Empty</strong>}>
                {(cue) => <strong data-no-localize>{cue().label}</strong>}
              </Show>
              <small>{pad.cue ? `${pad.cue.fade_ms} ms` : "-"}</small>
            </button>
          )}
        </For>
      </div>
      <Show when={props.snapshot.cues.length === 0}>
        <p class="empty">No cues. Create one in Control &gt; Live with Store Cue.</p>
      </Show>
      <Show when={props.snapshot.active_fade}>
        {(fade) => (
          <div class="liveFadeMeter touchFadeMeter">
            <span>{Math.round(fade().progress * 100)}%</span>
            <progress max="1" value={fade().progress} />
          </div>
        )}
      </Show>
      <div class="touchMasterGrid">
        <label>
          Lighting
          <input
            type="range"
            min="0"
            max="1"
            step="0.01"
            value={props.snapshot.lighting_master}
            onInput={(event) => void props.onSetLightingMaster(Number(event.currentTarget.value))}
          />
          <strong>{Math.round(props.snapshot.lighting_master * 100)}%</strong>
        </label>
        <label>
          Video
          <input
            type="range"
            min="0"
            max="1"
            step="0.01"
            value={props.snapshot.video.master_opacity}
            onInput={(event) => void props.onSetVideoMasterOpacity(Number(event.currentTarget.value))}
          />
          <strong>{Math.round(props.snapshot.video.master_opacity * 100)}%</strong>
        </label>
      </div>
      <div class="touchGuardRow">
        <button
          class={props.snapshot.blackout ? "primary" : ""}
          onClick={() => void props.onSetBlackout(!props.snapshot.blackout)}
        >
          {props.snapshot.blackout ? "Clear DMX BO" : "DMX BO"}
        </button>
        <button
          class={props.snapshot.video.blackout ? "primary" : ""}
          onClick={() => void props.onSetVideoBlackout(!props.snapshot.video.blackout)}
        >
          {props.snapshot.video.blackout ? "Clear Video BO" : "Video BO"}
        </button>
        <button
          class={props.snapshot.blackout && props.snapshot.video.blackout ? "primary" : ""}
          onClick={() => void props.onSetAllBlackout(true)}
          disabled={props.snapshot.blackout && props.snapshot.video.blackout}
        >
          All BO
        </button>
        <button
          onClick={() => void props.onSetAllBlackout(false)}
          disabled={!props.snapshot.blackout && !props.snapshot.video.blackout}
        >
          All Clear
        </button>
      </div>
    </section>
  );
}
