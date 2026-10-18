import { For, Show, onCleanup, onMount } from "solid-js";
import type { VideoClipLayerRuntimeSummary } from "../types";
import type { VideoControlPanelProps } from "./VideoControlPanel";
import { LiveVideoMonitorPanel } from "./LiveVideoMonitorPanel";
import { VideoStagePreviewButton } from "./VideoClipGridPanel";
import { VideoPerformMatrix } from "./VideoPerformMatrix";
import { VideoPerformQuickFx } from "./VideoPerformQuickFx";
import { controlMappingTargetData } from "../controlMappingLearn";

export interface VideoPerformContext {
  runtimes: VideoClipLayerRuntimeSummary[];
  keyboardEnabled: boolean;
  onOpenEditor: () => void;
}

/** A presentation adapter over the existing publication, transport and FX owners. */
export function VideoPerformDesk(props: VideoControlPanelProps) {
  let legend!: HTMLDialogElement;
  let blackout!: HTMLButtonElement;
  const keyboardEnabled = () => props.performContext?.keyboardEnabled !== false;
  const shortcut = (event: KeyboardEvent) => {
    if (!keyboardEnabled() || event.repeat || event.isComposing || event.defaultPrevented ||
      document.querySelector("dialog[open]") || (event.target instanceof Element &&
        event.target.closest("input,select,textarea,[contenteditable=true]"))) return;
    if (event.ctrlKey && event.shiftKey && event.code === "KeyB" && !event.altKey && !event.metaKey) {
      event.preventDefault(); event.stopImmediatePropagation(); blackout.click();
    } else if (event.key === "?" && !event.ctrlKey && !event.altKey && !event.metaKey) {
      event.preventDefault(); event.stopImmediatePropagation(); legend.showModal();
    }
  };
  onMount(() => window.addEventListener("keydown", shortcut, true));
  onCleanup(() => window.removeEventListener("keydown", shortcut, true));
  const unhealthy = () => props.mediaLibrary.assets.filter(asset => {
    const kind = props.mediaLibrary.availabilityById[asset.id]?.kind;
    return kind === "missing" || kind === "hash_mismatch" || kind === "unreadable";
  });
  const unchecked = () => props.mediaLibrary.assets.filter(asset =>
    !props.mediaLibrary.availabilityById[asset.id] ||
    props.mediaLibrary.availabilityById[asset.id]?.kind === "available_unverified").length;
  const errors = () => [...new Set([props.clipGrid.previewError, props.clipGrid.firstRunError, props.liveMonitors.preview.error, props.liveMonitors.program.error,
    props.liveMonitors.previewTransportError,
    ...(props.layerList.isfRuntimeErrors ?? []).map(error => error.message),
    ...unhealthy().map(asset => `${asset.label}: ${props.mediaLibrary.availabilityById[asset.id]?.kind}`),
    ...props.mediaLibrary.lastImportReport?.entries.filter(entry => entry.status === "failed").map(entry => entry.message) ?? [],
  ].filter(Boolean))].join(" · ");
  return <section id="edit-domain-panel-perform" role="tabpanel" aria-labelledby="edit-domain-tab-perform"
    class="panel videoControlPanel videoPerformLayout" data-video-perform-layout aria-label="VJ performance desk">
    <header class="videoPerformDeskHeader">
      <h2>Perform</h2>
      <section class="videoPerformHealth" data-video-perform-region="health" aria-label="Media and monitor health">
        <strong>Media health</strong><span title={errors() || undefined}><Show when={errors()} fallback={<><span>Assets</span>: <span data-no-localize>{props.mediaLibrary.assets.length}</span> · <span>Not verified</span>: <span data-no-localize>{unchecked()}</span></>}>{error => <span data-no-localize>{error()}</span>}</Show></span>
        <Show when={!props.clipGrid.thumbnailsAuthorized}><button onClick={props.clipGrid.onRequestThumbnails}>Load Thumbnails</button></Show>
        <Show when={props.liveMonitors.preview.status === "error" || props.liveMonitors.program.status === "error"}>
          <button onClick={props.liveMonitors.onRetry}>Retry monitors</button>
        </Show>
      </section>
      <button data-video-perform-legend-toggle onClick={() => legend.showModal()}>Keyboard shortcuts</button>
    </header>
    <section class="videoPerformMonitors" aria-label="Preview and Program">
      <LiveVideoMonitorPanel {...props.liveMonitors} />
      <section class="videoPerformTransition" data-video-perform-region="transition" aria-label="Performance transition">
        <label>Transition<select data-video-perform-transition-kind value={props.clipSlotBank.transitionKind}
          disabled={Boolean(props.clipSlotBank.runtime?.transition)} onInput={event => props.clipSlotBank.onSetTransitionKind(event.currentTarget.value as typeof props.clipSlotBank.transitionKind)}>
          <For each={["Cut", "Crossfade", "Dip", "Wipe", "Luma", "Displacement", "Blur", "Glitch", "Custom"]}>{kind => <option value={kind}>{kind}</option>}</For>
        </select></label>
        <label class="videoPerformDuration">Time<input type="number" data-video-perform-transition-duration min="0.001" max="600" step="0.001"
          value={props.clipSlotBank.transitionDurationMilliunits / 1000}
          disabled={Boolean(props.clipSlotBank.runtime?.transition)}
          onChange={event => { const value = Number(event.currentTarget.value); if (Number.isFinite(value) && value > 0) props.clipSlotBank.onSetTransitionDurationMilliunits(value * 1000); }} />
          <select aria-label="Transition time unit" value={props.clipSlotBank.transitionDurationUnit}
            disabled={Boolean(props.clipSlotBank.runtime?.transition)} onInput={event => props.clipSlotBank.onSetTransitionDurationUnit(event.currentTarget.value as typeof props.clipSlotBank.transitionDurationUnit)}>
            <option value="Milliseconds">Seconds</option><option value="Beats">Beats</option><option value="Bars">Bars</option>
          </select>
        </label>
        <div class="videoPerformTakeRow">
          <VideoStagePreviewButton layer={props.clipSlotBank.layer} previewLayerId={props.clipGrid.previewLayerId}
            backendAvailable={props.clipGrid.previewBackendAvailable} busy={props.clipGrid.previewBusy} onStagePreview={props.clipGrid.onStagePreview} />
          <button class="primary" data-video-perform-take data-video-clip-slot-take disabled={!props.clipSlotTake.enabled}
            onClick={() => void props.clipSlotTake.onTake()}>{props.clipSlotTake.label}</button>
        </div>
      </section>
      <section class="videoPerformMaster" aria-label="Performance master">
        <label>Master <strong>{Math.round(props.masterControls.masterOpacity * 100)}%</strong>
          <input data-video-perform-master type="range" min="0" max="1" step="0.01" value={props.masterControls.masterOpacity}
            onInput={event => void props.masterControls.onSetMasterOpacity(Number(event.currentTarget.value))}
            {...controlMappingTargetData({ action: "VideoMaster", label: "Video Master" })} /></label>
        <button ref={blackout} class={`killButton${props.masterControls.blackout ? " engaged" : ""}`} data-video-perform-blackout
          aria-pressed={props.masterControls.blackout} onClick={() => void props.masterControls.onSetBlackout(!props.masterControls.blackout)}
          {...controlMappingTargetData({ action: "VideoBlackout", label: "Video Blackout" })}>
          {props.masterControls.blackout ? "Recover video…" : "Blackout…"}
        </button>
        <span>{props.masterControls.blackout ? "BLACKOUT ENGAGED" : "Confirmation required"}</span>
      </section>
    </section>
    <VideoPerformMatrix bank={props.clipSlotBank} layerList={props.layerList}
      runtimes={props.performContext?.runtimes ?? (props.clipSlotBank.runtime ? [props.clipSlotBank.runtime] : [])}
      availabilityById={props.mediaLibrary.availabilityById} previewLayerId={props.clipGrid.previewLayerId}
      thumbnailsAuthorized={props.clipGrid.thumbnailsAuthorized} keyboardEnabled={keyboardEnabled()}
      onStop={layerId => props.clipGrid.onStop(layerId, props.clipGrid.fadeMs)} onTake={() => { if (props.clipSlotTake.enabled) return props.clipSlotTake.onTake(); }} />
    <VideoPerformQuickFx layer={props.clipSlotBank.layer} layerList={props.layerList} onOpenEditor={() => props.performContext?.onOpenEditor()} />
    <dialog ref={legend} class="videoPerformLegend" data-video-perform-legend>
      <h2>Keyboard shortcuts</h2>
      <dl><dt>1–9, 0, Q–P, A–L, Z–C</dt><dd>Queue a clip in the selected layer</dd>
        <dt>← → ↑ ↓ · Home / End</dt><dd>Move clip and layer selection</dd>
        <dt>Space / Enter</dt><dd>Take the queued clip</dd>
        <dt data-no-localize>Ctrl + Shift + B</dt><dd>Blackout or recover, with confirmation</dd>
        <dt>?</dt><dd>Show keyboard shortcuts</dd></dl>
      <p>Right-click or long-press a clip for its options. Inspect and Clip… in the matrix corner act on the selected clip; Inspect shows a thumbnail only.</p>
      <p>The P button stages the current layer in Preview; the P key queues column 20.</p>
      <p>Shortcuts pause while editing a value, a dialog is open, or MIDI Learn is active.</p>
      <p>Clip, Take, blend, transition and ISF stage MIDI targets are not supported by the current control model.</p>
      <button onClick={() => legend.close()}>Close</button>
    </dialog>
  </section>;
}
