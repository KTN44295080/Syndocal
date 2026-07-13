import { createEffect, createMemo, createSignal, For, Show } from "solid-js";
import type {
  LiveAudioInputStatus,
  VideoAudioMonitorStatus,
  VideoLayerSummary,
  VideoRecordingStatus,
} from "../types";

interface VideoClipGridPanelProps {
  compact?: boolean;
  layers: VideoLayerSummary[];
  thumbnails: Record<number, string>;
  fadeMs: number;
  audioMonitorVolume: number;
  audioMonitorLayerIds: number[];
  audioMonitorStatus: VideoAudioMonitorStatus;
  programAudioEnabled: boolean;
  audioOutputDevices: string[];
  selectedAudioOutputDevice: string;
  deckALayerId: number | null;
  deckBLayerId: number | null;
  abMix: number;
  selectedOutputId: number | null;
  recordingStatus: VideoRecordingStatus;
  liveAudioInputDevices: string[];
  selectedLiveAudioInputDevice: string;
  liveAudioInputStatus: LiveAudioInputStatus;
  previewLayerId: number | null;
  previewBusy: boolean;
  previewError: string | null;
  previewBackendAvailable: boolean;
  firstRunAvailable: boolean;
  firstRunBusy: boolean;
  firstRunError: string | null;
  firstRunBackendAvailable: boolean;
  onSetFadeMs: (fadeMs: number) => void;
  onSetAudioMonitorVolume: (volume: number) => void;
  onSetProgramAudioEnabled: (enabled: boolean) => void;
  onSetAudioOutputDevice: (deviceName: string) => void;
  onRefreshAudioOutputDevices: () => void | Promise<void>;
  onAssignDeck: (deck: "A" | "B", layerId: number) => void;
  onSetAbMix: (mix: number) => void | Promise<void>;
  onCommitAbMix: (mix: number) => void | Promise<void>;
  onLaunchDeck: (deck: "A" | "B") => void | Promise<void>;
  onStartRecording: (outputId: number, includeAudio: boolean) => void | Promise<void>;
  onStopRecording: () => void | Promise<void>;
  onSetLiveAudioInputDevice: (deviceName: string) => void;
  onRefreshLiveAudioInputDevices: () => void | Promise<void>;
  onStartLiveAudioInput: () => void | Promise<void>;
  onStopLiveAudioInput: () => void | Promise<void>;
  onStagePreview: (layerId: number) => void | Promise<unknown>;
  onImportMedia: () => void | Promise<void>;
  onCreateFirstRunShow: () => void | Promise<void>;
  onLaunch: (layerId: number, fadeMs: number) => void | Promise<void>;
  onTake: (layerId: number, fadeMs: number) => void | Promise<void>;
  onStop: (layerId: number, fadeMs: number) => void | Promise<void>;
  onMonitorAudio: (layerId: number, volume: number) => void | Promise<void>;
  onStopAudio: (layerId: number) => void | Promise<void>;
}

const CLIPS_PER_BANK = 12;

const sourceLabel = (layer: VideoLayerSummary) => {
  const source = layer.source.path ?? layer.source.name ?? layer.source.kind;
  const normalized = source.replaceAll("\\", "/");
  return normalized.slice(normalized.lastIndexOf("/") + 1);
};

export function VideoClipGridPanel(props: VideoClipGridPanelProps) {
  let panelElement: HTMLElement | undefined;
  let previousLayerCount = props.layers.length;
  const [bank, setBank] = createSignal(0);
  const bankCount = createMemo(() => Math.max(1, Math.ceil(props.layers.length / CLIPS_PER_BANK)));
  const visibleLayers = createMemo(() => {
    const start = bank() * CLIPS_PER_BANK;
    return props.layers.slice(start, start + CLIPS_PER_BANK);
  });
  const previewLayer = createMemo(() =>
    props.layers.find((layer) => layer.id === props.previewLayerId) ?? null,
  );
  const deckALayer = createMemo(() => props.layers.find((layer) => layer.id === props.deckALayerId) ?? null);
  const deckBLayer = createMemo(() => props.layers.find((layer) => layer.id === props.deckBLayerId) ?? null);

  createEffect(() => {
    const layerCount = props.layers.length;
    if (previousLayerCount === 0 && layerCount > 0) {
      window.requestAnimationFrame(() => {
        panelElement?.querySelector<HTMLButtonElement>(".videoClipLaunch")?.focus();
      });
    }
    previousLayerCount = layerCount;
    if (bank() >= bankCount()) setBank(bankCount() - 1);
  });

  return (
    <section ref={panelElement} class={`videoClipGridPanel ${props.layers.length === 0 ? "empty" : ""}`} aria-label="Video clip grid">
      <div class="sectionHeader videoClipGridHeader">
        <div>
          <h3>Clip Grid</h3>
          <span>{props.layers.length} clip(s)</span>
        </div>
        <details class={`videoClipUtilities ${props.compact ? "compact" : ""}`} open={!props.compact}>
          <summary>Audio &amp; Capture</summary>
          <div class="videoClipGridSettings">
          <label>
            Take fade ms
            <input
              type="number"
              min="0"
              step="50"
              value={props.fadeMs}
              onInput={(event) => props.onSetFadeMs(Math.max(0, Number(event.currentTarget.value)))}
            />
          </label>
          <label>
            Monitor gain
            <input
              type="number"
              min="0"
              max="2"
              step="0.05"
              value={props.audioMonitorVolume}
              onInput={(event) => props.onSetAudioMonitorVolume(Math.max(0, Math.min(2, Number(event.currentTarget.value))))}
            />
          </label>
          <label class="checkbox videoProgramAudioToggle">
            <input
              type="checkbox"
              checked={props.programAudioEnabled}
              onChange={(event) => props.onSetProgramAudioEnabled(event.currentTarget.checked)}
            />
            Program audio
          </label>
          <div class="videoAudioDeviceField">
            <label>
              Audio device
              <select
                value={props.selectedAudioOutputDevice}
                onInput={(event) => props.onSetAudioOutputDevice(event.currentTarget.value)}
              >
                <option value="">System default</option>
                <For each={props.audioOutputDevices}>{(device) => <option value={device}>{device}</option>}</For>
              </select>
            </label>
            <button aria-label="Refresh audio output devices" title="Refresh audio output devices" onClick={() => void props.onRefreshAudioOutputDevices()}>↻</button>
          </div>
          </div>
        </details>
      </div>
      <Show when={props.audioMonitorStatus.active_layer_ids.length > 0 || props.audioMonitorStatus.last_sync_error}>
        <div class={`videoAudioSyncStatus ${props.audioMonitorStatus.last_sync_error ? "error" : ""}`} role="status">
          <span class="tabularNums">
            A/V drift {props.audioMonitorStatus.last_drift_ms} ms · max {props.audioMonitorStatus.max_abs_drift_ms} ms · {props.audioMonitorStatus.resync_count} resync
          </span>
          <Show when={props.audioMonitorStatus.last_sync_error}>
            <strong>{props.audioMonitorStatus.last_sync_error}</strong>
          </Show>
        </div>
      </Show>
      <div class="videoClipPreviewBar">
        <div>
          <small>PREVIEW</small>
          <Show when={previewLayer()} fallback={<strong>Select a clip</strong>}>
            {(layer) => <strong data-no-localize>{layer().label}</strong>}
          </Show>
          <span>{props.previewBusy
            ? "Staging preview…"
            : previewLayer() ? sourceLabel(previewLayer()!) : "P buttons stage a clip without taking it live."}</span>
          <Show when={props.previewError}>
            {(error) => <small class="videoClipPreviewError" role="alert" data-no-localize>{error()}</small>}
          </Show>
        </div>
        <div class="videoClipProgramTransfer" aria-label="Transfer staged preview to Program">
          <small>TO PROGRAM</small>
          <div class="buttonRow">
            <button
              class="primary"
              disabled={!previewLayer() || props.previewBusy}
              onClick={() => previewLayer() && void props.onTake(previewLayer()!.id, 0)}
            >
              Cut
            </button>
            <button
              class="primary"
              disabled={!previewLayer() || props.previewBusy}
              onClick={() => previewLayer() && void props.onTake(previewLayer()!.id, props.fadeMs)}
            >
              Take
            </button>
          </div>
        </div>
      </div>
      <div class="videoAbDeck" aria-label="Video A B deck crossfader">
        <button class={props.abMix < 0.5 ? "active" : ""} disabled={!deckALayer()} onClick={() => void props.onLaunchDeck("A")}>
          <small>DECK A</small>
          <Show when={deckALayer()} fallback={<strong>Load A</strong>}>
            {(layer) => <strong data-no-localize>{layer().label}</strong>}
          </Show>
        </button>
        <label>
          A/B Mix
          <input
            type="range"
            min="0"
            max="1"
            step="0.01"
            value={props.abMix}
            onInput={(event) => void props.onSetAbMix(Number(event.currentTarget.value))}
            onChange={(event) => void props.onCommitAbMix(Number(event.currentTarget.value))}
          />
          <span>{Math.round((1 - props.abMix) * 100)} / {Math.round(props.abMix * 100)}</span>
        </label>
        <button class={props.abMix >= 0.5 ? "active" : ""} disabled={!deckBLayer()} onClick={() => void props.onLaunchDeck("B")}>
          <small>DECK B</small>
          <Show when={deckBLayer()} fallback={<strong>Load B</strong>}>
            {(layer) => <strong data-no-localize>{layer().label}</strong>}
          </Show>
        </button>
      </div>
      <div class={`videoRecordingBar ${props.recordingStatus.active ? "active" : ""}`}>
        <div>
          <small>{props.recordingStatus.active ? "● RECORDING" : "OUTPUT RECORD"}</small>
          <span>{props.recordingStatus.active
            ? `${props.recordingStatus.width}x${props.recordingStatus.height} @ ${props.recordingStatus.frame_rate}fps · ${props.recordingStatus.frames_written} frames · ${props.recordingStatus.dropped_frames} dropped · ${props.recordingStatus.audio_included ? `${props.recordingStatus.audio_track_count} audio` : "silent"}`
            : props.recordingStatus.last_error ?? `Records H.264 MP4${props.programAudioEnabled ? " and active Program audio" : " without audio"}.`}</span>
        </div>
        <button
          class={props.recordingStatus.active ? "danger" : ""}
          disabled={!props.recordingStatus.active && props.selectedOutputId === null}
          onClick={() => props.recordingStatus.active
            ? void props.onStopRecording()
            : props.selectedOutputId !== null && void props.onStartRecording(props.selectedOutputId, props.programAudioEnabled)}
        >
          {props.recordingStatus.active ? "Stop Recording" : "Record Output"}
        </button>
      </div>
      <div class={`liveAudioInputBar ${props.liveAudioInputStatus.running ? "active" : ""}`}>
        <div class="liveAudioInputControls">
          <label>
            Live FFT input
            <select
              disabled={props.liveAudioInputStatus.running}
              value={props.selectedLiveAudioInputDevice}
              onInput={(event) => props.onSetLiveAudioInputDevice(event.currentTarget.value)}
            >
              <option value="">System default</option>
              <For each={props.liveAudioInputDevices}>{(device) => <option value={device}>{device}</option>}</For>
            </select>
          </label>
          <button disabled={props.liveAudioInputStatus.running} aria-label="Refresh audio input devices" onClick={() => void props.onRefreshLiveAudioInputDevices()}>↻</button>
          <button class={props.liveAudioInputStatus.running ? "danger" : "primary"} onClick={() => props.liveAudioInputStatus.running ? void props.onStopLiveAudioInput() : void props.onStartLiveAudioInput()}>
            {props.liveAudioInputStatus.running ? "Stop Live FFT" : "Start Live FFT"}
          </button>
        </div>
        <div class="liveAudioMeters" aria-label="Live audio FFT levels">
          <For each={[
            ["B", props.liveAudioInputStatus.bass],
            ["M", props.liveAudioInputStatus.mid],
            ["H", props.liveAudioInputStatus.high],
          ] as const}>
            {([label, level]) => <span title={`${label} ${Math.round(level * 100)}%`}><i style={{ width: `${Math.round(level * 100)}%` }} />{label}</span>}
          </For>
          <small>{props.liveAudioInputStatus.running
            ? `${props.liveAudioInputStatus.sample_rate}Hz / ${props.liveAudioInputStatus.channels}ch / ${props.liveAudioInputStatus.analyzed_windows} FFT`
            : props.liveAudioInputStatus.last_error ?? "Live bands can drive Node Graph Audio sources."}</small>
        </div>
      </div>
      <Show
        when={props.layers.length > 0}
        fallback={
          <Show
            when={props.firstRunAvailable}
            fallback={
              <div class="emptyState emptyStateAction">
                <span>Import video or still images to populate the clip grid.</span>
                <button class="primary" onClick={() => void props.onImportMedia()}>
                  Import Media
                </button>
              </div>
            }
          >
            <div class="emptyState emptyStateAction vjFirstRunEmptyState" aria-busy={props.firstRunBusy} aria-live="polite">
              <h4>Start your first VJ show</h4>
              <span>Choose local video files to build the clip grid and stage the first clip.</span>
              <small class="vjFirstRunSafety">VJ Program is created Off and Blackout. No output window opens automatically.</small>
              <button
                class="primary"
                disabled={props.firstRunBusy || !props.firstRunBackendAvailable}
                title={props.firstRunBackendAvailable ? "Choose media and set up the VJ show" : "Desktop required"}
                onClick={() => void props.onCreateFirstRunShow()}
              >
                <span role={props.firstRunBusy ? "status" : undefined}>
                  {props.firstRunBusy ? "Setting Up…" : props.firstRunBackendAvailable ? "Choose Media & Set Up" : "Desktop required"}
                </span>
              </button>
              <Show when={props.firstRunError}>
                {(error) => <small class="vjFirstRunError" role="alert" data-no-localize>{error()}</small>}
              </Show>
            </div>
          </Show>
        }
      >
        <div class="videoClipGrid">
          <For each={visibleLayers()}>
            {(layer, index) => {
              const live = () => layer.state.enabled && layer.state.playing && layer.state.opacity > 0;
              const previewed = () => props.previewLayerId === layer.id;
              const previewSupported = () => layer.source.kind === "File" || layer.source.kind === "StillImage";
              const audioMonitoring = () => props.audioMonitorLayerIds.includes(layer.id);
              const audioUnavailable = () => layer.source.kind !== "File" || layer.source.metadata?.has_audio === false;
              const clipNumber = () => bank() * CLIPS_PER_BANK + index() + 1;
              return (
                <div class={`videoClipPad ${live() ? "live" : ""} ${previewed() ? "previewed" : ""}`}>
                  <button
                    class="videoClipLaunch"
                    aria-pressed={live()}
                    title={`Launch ${layer.label} from its in point`}
                    onClick={() => void props.onLaunch(layer.id, props.fadeMs)}
                  >
                    <Show when={props.thumbnails[layer.id]}>
                      <img
                        class="videoClipThumbnail"
                        src={props.thumbnails[layer.id]}
                        alt=""
                        loading="lazy"
                        draggable={false}
                      />
                    </Show>
                    <small>{clipNumber().toString().padStart(2, "0")} / {layer.source.kind}</small>
                    <strong data-no-localize>{layer.label}</strong>
                    <span data-no-localize title={sourceLabel(layer)}>{sourceLabel(layer)}</span>
                    <em>{live() ? `LIVE ${Math.round(layer.state.opacity * 100)}%` : "READY"}</em>
                  </button>
                  <button
                    class="videoClipPreview"
                    aria-label={`Preview ${layer.label}`}
                    aria-pressed={previewed()}
                    disabled={!props.previewBackendAvailable || !previewSupported() || props.previewBusy}
                    title={!props.previewBackendAvailable
                      ? "Desktop required"
                      : previewSupported()
                      ? `Stage ${layer.label} in Preview without changing Program`
                      : "Independent Preview supports local video and still image layers"}
                    onClick={() => void props.onStagePreview(layer.id)}
                  >
                    P
                  </button>
                  <button
                    class="videoClipStop"
                    aria-label={`Stop ${layer.label}`}
                    disabled={!live() && !layer.state.playing}
                    onClick={() => void props.onStop(layer.id, props.fadeMs)}
                  >
                    ■
                  </button>
                  <button
                    class={`videoClipAudio ${audioMonitoring() ? "active" : ""}`}
                    aria-label={`${audioMonitoring() ? "Stop" : "Monitor"} embedded audio for ${layer.label}`}
                    aria-pressed={audioMonitoring()}
                    title={audioUnavailable() ? "No embedded audio was detected" : "Monitor embedded audio from the current video position"}
                    disabled={audioUnavailable()}
                    onClick={() => void (audioMonitoring()
                      ? props.onStopAudio(layer.id)
                      : props.onMonitorAudio(layer.id, props.audioMonitorVolume))}
                  >
                    ♪
                  </button>
                  <div class="videoClipDeckLoad" aria-label={`Load ${layer.label} into a video deck`}>
                    <button class={props.deckALayerId === layer.id ? "active" : ""} aria-label={`Load ${layer.label} into Deck A`} onClick={() => props.onAssignDeck("A", layer.id)}>A</button>
                    <button class={props.deckBLayerId === layer.id ? "active" : ""} aria-label={`Load ${layer.label} into Deck B`} onClick={() => props.onAssignDeck("B", layer.id)}>B</button>
                  </div>
                </div>
              );
            }}
          </For>
        </div>
        <Show when={bankCount() > 1}>
          <div class="deckPager">
            <strong>Clip Bank</strong>
            <span>{bank() + 1} / {bankCount()}</span>
            <button onClick={() => setBank(Math.max(0, bank() - 1))} disabled={bank() === 0}>Prev</button>
            <button onClick={() => setBank(Math.min(bankCount() - 1, bank() + 1))} disabled={bank() >= bankCount() - 1}>Next</button>
          </div>
        </Show>
      </Show>
    </section>
  );
}
