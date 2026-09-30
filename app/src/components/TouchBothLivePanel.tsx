import type { ControlBothPanelProps } from "./ControlBothPanel";

const percent = (value: number) => `${Math.round(value * 100)}%`;

/** One compact live desk for Control > Both. Detailed authoring stays in Edit. */
export function TouchBothLivePanel(props: ControlBothPanelProps) {
  return (
    <section
      class="panel touchBothLivePanel"
      role="tabpanel"
      aria-labelledby="control-domain-both"
      aria-label="Lighting and Video live control"
      data-workspace-pane="upper"
    >
      <header class="touchBothHeader">
        <strong>SHOW CONTROL</strong>
        <span class="tabularNums" data-no-localize>{props.timecode}</span>
      </header>

      <div class="touchBothLane" aria-label="Lighting live controls">
        <div class="touchBothIdentity">
          <strong>LIGHTING</strong>
          <span title={props.activeCueLabel}><span>Current</span> · <span data-no-localize>{props.activeCueLabel}</span></span>
          <span title={props.nextCueLabel}><span>Next</span> · <span data-no-localize>{props.nextCueLabel}</span></span>
        </div>
        <div class="touchBothActions" aria-label="Cue actions">
          <button type="button" disabled={props.cueCount === 0} onClick={props.onBack}>Back</button>
          <button type="button" class="primary" disabled={props.cueCount === 0} onClick={props.onGo}>GO</button>
          <button type="button" disabled={props.activeCueId === null} onClick={props.onRelease}>Release</button>
        </div>
        <label class="touchBothMaster">
          <span>Lighting Master <output>{percent(props.lightingMaster)}</output></span>
          <input type="range" min="0" max="1" step="0.01" value={props.lightingMaster}
            aria-label="Lighting Master"
            onChange={(event) => void props.onSetLightingMaster(Number(event.currentTarget.value))} />
        </label>
        <div class="touchBothEndActions">
          <button type="button" class={props.blackout ? "active danger" : ""}
            aria-pressed={props.blackout}
            aria-label={props.blackout ? "Clear DMX blackout" : "Enable DMX blackout"}
            onClick={() => void props.onSetBlackout(!props.blackout)}>
            {props.blackout ? "Clear DMX BO" : "DMX BO"}
          </button>
          <button type="button" data-touch-both-open="lighting" onClick={props.onOpenLighting}>Lighting controls</button>
        </div>
      </div>

      <div class="touchBothLane" aria-label="Video live controls">
        <div class="touchBothIdentity">
          <strong>VIDEO</strong>
          <span title={props.programLabel ?? undefined}><span>Program</span> · {props.programLabel
            ? <span data-no-localize>{props.programLabel}</span> : <span>No output selected</span>}</span>
          <span title={props.previewLabel ?? undefined}><span>Preview</span> · {props.previewLabel
            ? <span data-no-localize>{props.previewLabel}</span> : <span>No clip staged</span>}</span>
        </div>
        <div class="touchBothVideoStatus" aria-label="Video status">
          <span>Video {props.enabledVideoOutputCount}/{props.videoOutputCount}</span>
          <span><span>Recording</span> · <span data-no-localize>{props.recordingLabel}</span></span>
        </div>
        <label class="touchBothMaster">
          <span>Video Master <output>{percent(props.videoMaster)}</output></span>
          <input type="range" min="0" max="1" step="0.01" value={props.videoMaster}
            aria-label="Video Master"
            onChange={(event) => void props.onSetVideoMaster(Number(event.currentTarget.value))} />
        </label>
        <div class="touchBothEndActions">
          <button type="button" class={props.videoBlackout ? "active danger" : ""}
            aria-pressed={props.videoBlackout}
            aria-label={props.videoBlackout ? "Clear video blackout" : "Enable video blackout"}
            onClick={() => void props.onSetVideoBlackout(!props.videoBlackout)}>
            {props.videoBlackout ? "Clear Video BO" : "Video BO"}
          </button>
          <button type="button" data-touch-both-open="video" onClick={props.onOpenVideo}>Video controls</button>
        </div>
      </div>

      <footer class="touchBothFooter">
        <span>DMX {props.enabledDmxOutputCount}/{props.dmxOutputCount}</span>
        <span>Video {props.enabledVideoOutputCount}/{props.videoOutputCount}</span>
        <span>{props.timelinePlaying ? "Playing" : "Stopped"}</span>
        <button type="button" class={props.blackout && props.videoBlackout ? "active danger" : ""}
          aria-pressed={props.blackout && props.videoBlackout}
          aria-label={props.blackout && props.videoBlackout ? "Clear all blackouts" : "Enable all blackouts"}
          onClick={() => void props.onSetAllBlackout(!(props.blackout && props.videoBlackout))}>
          {props.blackout && props.videoBlackout ? "Clear All BO" : "All BO"}
        </button>
      </footer>
    </section>
  );
}
