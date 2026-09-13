import { Show } from "solid-js";

export interface ControlBothPanelProps {
  activeCueId: number | null;
  activeCueLabel: string;
  nextCueLabel: string;
  cueCount: number;
  timelinePlaying: boolean;
  timecode: string;
  lightingMaster: number;
  videoMaster: number;
  blackout: boolean;
  videoBlackout: boolean;
  enabledDmxOutputCount: number;
  dmxOutputCount: number;
  enabledVideoOutputCount: number;
  videoOutputCount: number;
  previewLabel: string | null;
  previewStatus: string;
  programLabel: string | null;
  programStatus: string;
  clipCount: number;
  selectedClipLabel: string | null;
  recordingLabel: string;
  onOpenLighting: () => void;
  onOpenVideo: () => void;
  onBack: () => void;
  onGo: () => void;
  onRelease: () => void;
  onSetLightingMaster: (level: number) => void | Promise<void>;
  onSetVideoMaster: (level: number) => void | Promise<void>;
  onSetBlackout: (enabled: boolean) => void | Promise<void>;
  onSetVideoBlackout: (enabled: boolean) => void | Promise<void>;
  onSetAllBlackout: (enabled: boolean) => void | Promise<void>;
}

const percent = (value: number) => `${Math.round(value * 100)}%`;

export function ControlBothPanel(props: ControlBothPanelProps) {
  return (
    <section
      id="edit-domain-panel-both"
      role="tabpanel"
      aria-labelledby="edit-domain-tab-both"
      class="panel controlBothPanel controlPanel"
      data-workspace-pane="upper"
      aria-label="Lighting and Video control overview"
    >
      <header class="panelHeader controlBothHeader">
        <div>
          <h2>Both</h2>
          <span>Lighting + Video</span>
        </div>
        <div class="controlBothHeaderActions">
          <button type="button" onClick={props.onOpenLighting}>Open Lighting</button>
          <button type="button" onClick={props.onOpenVideo}>Open Video</button>
        </div>
      </header>

      <div class="controlBothGrid">
        <section class="controlBothDomainCard controlBothLightingCard" aria-label="Lighting live controls">
          <header class="controlBothCardHeader">
            <div>
              <span class="uiMicroLabel">LIGHTING</span>
              <strong>Live cue</strong>
            </div>
            <span class="controlBothTransportState">{props.timelinePlaying ? "PLAYING" : "STOPPED"}</span>
          </header>
          <dl class="controlBothCueReadout">
            <div>
              <dt>Current</dt>
              <dd data-no-localize>{props.activeCueLabel}</dd>
            </div>
            <div>
              <dt>Next</dt>
              <dd data-no-localize>{props.nextCueLabel}</dd>
            </div>
          </dl>
          <div class="controlBothActionRow" aria-label="Cue actions">
            <button type="button" disabled={props.cueCount === 0} onClick={props.onBack}>Back</button>
            <button type="button" class="primary" disabled={props.cueCount === 0} onClick={props.onGo}>GO</button>
            <button type="button" disabled={props.activeCueId === null} onClick={props.onRelease}>Release</button>
          </div>
          <label class="controlBothMasterControl">
            <span>Lighting Master</span>
            <output>{percent(props.lightingMaster)}</output>
            <input
              type="range"
              min="0"
              max="1"
              step="0.01"
              value={props.lightingMaster}
              aria-label="Lighting Master"
              onChange={(event) => void props.onSetLightingMaster(Number(event.currentTarget.value))}
            />
          </label>
          <div class="controlBothActionRow controlBothBlackoutRow" aria-label="Lighting blackout actions">
            <button
              type="button"
              class={props.blackout ? "active danger" : ""}
              aria-pressed={props.blackout}
              aria-label={props.blackout ? "Clear DMX blackout" : "Enable DMX blackout"}
              onClick={() => void props.onSetBlackout(!props.blackout)}
            >
              {props.blackout ? "Clear DMX BO" : "DMX BO"}
            </button>
            <button
              type="button"
              class={props.videoBlackout && props.blackout ? "active danger" : ""}
              aria-pressed={props.videoBlackout && props.blackout}
              aria-label={props.videoBlackout && props.blackout ? "Clear all blackouts" : "Enable all blackouts"}
              onClick={() => void props.onSetAllBlackout(!(props.blackout && props.videoBlackout))}
            >
              {props.blackout && props.videoBlackout ? "Clear All BO" : "All BO"}
            </button>
          </div>
        </section>

        <section class="controlBothDomainCard controlBothVideoCard" aria-label="Video live controls">
          <header class="controlBothCardHeader">
            <div>
              <span class="uiMicroLabel">VIDEO</span>
              <strong>Live outputs</strong>
            </div>
            <button type="button" onClick={props.onOpenVideo}>Video controls</button>
          </header>
          <div class="controlBothMonitorReadout">
            <div data-both-monitor="preview">
              <span>PREVIEW</span>
              <strong data-no-localize>{props.previewLabel ?? "No clip staged"}</strong>
              <small>{props.previewStatus}</small>
            </div>
            <div data-both-monitor="program">
              <span>PROGRAM</span>
              <strong data-no-localize>{props.programLabel ?? "No output selected"}</strong>
              <small>{props.programStatus}</small>
            </div>
          </div>
          <label class="controlBothMasterControl">
            <span>Video Master</span>
            <output>{percent(props.videoMaster)}</output>
            <input
              type="range"
              min="0"
              max="1"
              step="0.01"
              value={props.videoMaster}
              aria-label="Video Master"
              onChange={(event) => void props.onSetVideoMaster(Number(event.currentTarget.value))}
            />
          </label>
          <div class="controlBothVideoTruth" data-both-video-truth>
            <div><span>Clip Grid</span><strong>{props.clipCount} clip(s)</strong></div>
            <div><span>Selected</span><strong data-no-localize>{props.selectedClipLabel ?? "None"}</strong></div>
            <div><span>Recording</span><strong>{props.recordingLabel}</strong></div>
          </div>
          <div class="controlBothActionRow controlBothBlackoutRow" aria-label="Video blackout actions">
            <button
              type="button"
              class={props.videoBlackout ? "active danger" : ""}
              aria-pressed={props.videoBlackout}
              aria-label={props.videoBlackout ? "Clear video blackout" : "Enable video blackout"}
              onClick={() => void props.onSetVideoBlackout(!props.videoBlackout)}
            >
              {props.videoBlackout ? "Clear Video BO" : "Video BO"}
            </button>
            <Show when={props.videoOutputCount > 0}>
              <span class="controlBothOutputTruth">Video {props.enabledVideoOutputCount}/{props.videoOutputCount}</span>
            </Show>
          </div>
        </section>
      </div>

      <footer class="controlBothFooter" aria-label="Combined output truth">
        <span>DMX {props.enabledDmxOutputCount}/{props.dmxOutputCount}</span>
        <span>Video {props.enabledVideoOutputCount}/{props.videoOutputCount}</span>
        <span>Timecode <b class="tabularNums" data-no-localize>{props.timecode}</b></span>
      </footer>
    </section>
  );
}
