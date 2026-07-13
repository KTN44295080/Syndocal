import type { EngineSnapshot } from "../types";

type MaybePromise = void | Promise<void>;

interface TouchSafetyDeckProps {
  snapshot: EngineSnapshot;
  onTriggerPreviousCue: () => MaybePromise;
  onTriggerNextCue: () => MaybePromise;
  onSetCueFadePaused: (paused: boolean) => MaybePromise;
  onSetLightingMaster: (level: number) => MaybePromise;
  onSetVideoMasterOpacity: (level: number) => MaybePromise;
  onSetBlackout: (enabled: boolean) => MaybePromise;
  onSetVideoBlackout: (enabled: boolean) => MaybePromise;
  onSetAllBlackout: (enabled: boolean) => MaybePromise;
}

export function TouchSafetyDeck(props: TouchSafetyDeckProps) {
  return (
    <section class="panel touchPanel touchSafetyDeck" aria-label="Persistent show safety controls">
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
