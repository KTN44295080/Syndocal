import { createMemo, For, Show } from "solid-js";
import type {
  LiveAudioChannelMix,
  LiveAudioInputCapabilities,
  LiveAudioInputDeviceSummary,
  LiveAudioInputStatus,
} from "../types";
import {
  liveAudioChannelMixLabel,
  liveAudioInputAnnouncement,
  liveAudioInputDetail,
  liveAudioInputHealth,
} from "../liveAudioInputPresentation";

export interface LiveAudioInputRailProps {
  compact?: boolean;
  liveAudioInputDevices: LiveAudioInputDeviceSummary[];
  selectedLiveAudioInputDevice: string;
  liveAudioInputCapabilities: LiveAudioInputCapabilities | null;
  liveAudioInputCapabilitiesBusy: boolean;
  liveAudioInputSampleRate: number | null;
  liveAudioInputBufferFrames: number | null;
  liveAudioInputChannelMix: LiveAudioChannelMix;
  liveAudioInputStatus: LiveAudioInputStatus;
  liveAudioInputStatusKnown: boolean;
  liveAudioInputBusy: boolean;
  onSetLiveAudioInputDevice: (deviceName: string) => void;
  onSetLiveAudioInputSampleRate: (sampleRate: number | null) => void;
  onSetLiveAudioInputBufferFrames: (bufferFrames: number | null) => void;
  onSetLiveAudioInputChannelMix: (channelMix: LiveAudioChannelMix) => void;
  onRefreshLiveAudioInputDevices: () => void | Promise<void>;
  onStartLiveAudioInput: () => void | Promise<void>;
  onStopLiveAudioInput: () => void | Promise<void>;
}

const meterBands = [
  ["B", "Bass", "bass"],
  ["M", "Mid", "mid"],
  ["H", "High", "high"],
] as const;
const liveAudioBandIndices = Array.from({ length: 16 }, (_, index) => index);

const COMMON_SAMPLE_RATES = [44_100, 48_000, 88_200, 96_000, 176_400, 192_000] as const;
const COMMON_BUFFER_FRAMES = [32, 64, 128, 256, 512, 1_024, 2_048, 4_096, 8_192] as const;

const channelMixValue = (mix: LiveAudioChannelMix): string => {
  switch (mix.mode) {
    case "average_all":
      return "average_all";
    case "single":
      return `single:${mix.channel_index}`;
    case "stereo_pair":
      return `stereo_pair:${mix.left_channel_index}:${mix.right_channel_index}`;
  }
};

const parseChannelMix = (value: string): LiveAudioChannelMix => {
  const [mode, first, second] = value.split(":");
  if (mode === "single") return { mode, channel_index: Number(first) };
  if (mode === "stereo_pair") {
    return {
      mode,
      left_channel_index: Number(first),
      right_channel_index: Number(second),
    };
  }
  return { mode: "average_all" };
};

export function LiveAudioInputRail(props: LiveAudioInputRailProps) {
  const health = createMemo(() =>
    liveAudioInputHealth(props.liveAudioInputStatus, props.liveAudioInputStatusKnown),
  );
  const detail = createMemo(() =>
    liveAudioInputDetail(props.liveAudioInputStatus, props.liveAudioInputStatusKnown),
  );
  const selectableSampleRates = createMemo(() => {
    const capabilities = props.liveAudioInputCapabilities;
    if (!capabilities) return [];
    const rates = new Set<number>();
    if (capabilities.default_config.sample_rate > 0) {
      rates.add(capabilities.default_config.sample_rate);
    }
    for (const config of capabilities.supported_configs) {
      if (config.min_sample_rate > 0) rates.add(config.min_sample_rate);
      if (config.max_sample_rate > 0) rates.add(config.max_sample_rate);
    }
    for (const sampleRate of COMMON_SAMPLE_RATES) {
      if (
        capabilities.supported_configs.some(
          (config) => config.min_sample_rate <= sampleRate && sampleRate <= config.max_sample_rate,
        )
      ) {
        rates.add(sampleRate);
      }
    }
    return [...rates].sort((left, right) => left - right);
  });
  const selectableBufferFrames = createMemo(() => {
    const capabilities = props.liveAudioInputCapabilities;
    if (!capabilities) return [];
    const resolved = capabilities.resolved_config;
    const candidates = new Set<number>(COMMON_BUFFER_FRAMES);
    if (resolved.buffer_size.kind === "range") {
      candidates.add(resolved.buffer_size.min_frames);
      candidates.add(resolved.buffer_size.max_frames);
    }
    return [...candidates]
      .filter(
        (frames) =>
          frames > 0 &&
          frames <= capabilities.max_capture_frames &&
          frames * 1_000 <= resolved.sample_rate * 200 &&
          (resolved.buffer_size.kind === "unknown" ||
            (resolved.buffer_size.min_frames <= frames &&
              frames <= resolved.buffer_size.max_frames)),
      )
      .sort((left, right) => left - right);
  });
  const inputChannels = createMemo(
    () => props.liveAudioInputCapabilities?.resolved_config.channels ?? 0,
  );
  const stereoPairs = createMemo(() =>
    Array.from({ length: Math.floor(inputChannels() / 2) }, (_, index) => [index * 2, index * 2 + 1] as const),
  );
  const selectedDevice = createMemo(() =>
    props.liveAudioInputDevices.find(
      (device) => device.id === props.selectedLiveAudioInputDevice,
    ),
  );
  const selectedDeviceRequiresReselection = createMemo(
    () => Boolean(props.selectedLiveAudioInputDevice && !selectedDevice()),
  );
  const selectedDeviceLabel = createMemo(() =>
    selectedDeviceRequiresReselection()
      ? "Reselect audio input"
      : selectedDevice()?.label ?? "System default audio input",
  );
  const configFormat = createMemo(() =>
    selectedDeviceRequiresReselection()
      ? "Reselect input"
      : props.liveAudioInputCapabilitiesBusy
        ? "I/O…"
        : props.liveAudioInputCapabilities
          ? `${props.liveAudioInputCapabilities.backend} · ${props.liveAudioInputCapabilities.resolved_config.sample_format} · ${props.liveAudioInputCapabilities.resolved_config.channels}ch`
          : "I/O unavailable",
  );
  const inputLocked = () =>
    props.liveAudioInputBusy ||
    !props.liveAudioInputStatusKnown ||
    props.liveAudioInputStatus.running ||
    props.liveAudioInputStatus.safety_clear_pending;
  const visualBandPercent = (index: number) =>
    Math.round(
      Math.max(0, Math.min(1, props.liveAudioInputStatus.bands?.[index] ?? 0)) * 100,
    );
  const rhythmLabel = createMemo(() => {
    const bpm = props.liveAudioInputStatus.bpm;
    const confidence = Math.round(
      Math.max(0, Math.min(1, props.liveAudioInputStatus.bpm_confidence ?? 0)) * 100,
    );
    return bpm && confidence > 0 ? `${bpm.toFixed(1)} · ${confidence}%` : "—";
  });

  return (
    <section
      class={`liveAudioInputBar ${props.compact ? "compact" : ""} ${props.liveAudioInputStatus.running ? "active" : ""} ${health()}`}
      data-health={health()}
      aria-label="Live audio analysis input"
    >
      <div class="liveAudioInputControls">
        <label>
          <span class="liveAudioInputLabel">Audio input</span>
          <select
            aria-label="Live audio input device"
            disabled={inputLocked()}
            value={props.selectedLiveAudioInputDevice}
            title={selectedDeviceLabel()}
            aria-invalid={selectedDeviceRequiresReselection() ? "true" : undefined}
            onInput={(event) => props.onSetLiveAudioInputDevice(event.currentTarget.value)}
          >
            <Show when={selectedDeviceRequiresReselection()}>
              <option
                value={props.selectedLiveAudioInputDevice}
                selected={selectedDeviceRequiresReselection()}
              >
                Reselect input
              </option>
            </Show>
            <option value="">System default</option>
            <For each={props.liveAudioInputDevices}>
              {(device) => <option value={device.id} data-no-localize>{device.label}</option>}
            </For>
          </select>
        </label>
        <button
          disabled={props.liveAudioInputBusy || props.liveAudioInputStatus.running}
          aria-label="Refresh audio input devices"
          title="Refresh audio input devices"
          onClick={() => void props.onRefreshLiveAudioInputDevices()}
        >
          ↻
        </button>
        <button
          class={props.liveAudioInputStatus.running ? "danger" : "primary"}
          disabled={
            props.liveAudioInputBusy ||
            (!props.liveAudioInputStatus.running && props.liveAudioInputCapabilitiesBusy) ||
            (!props.liveAudioInputStatus.running && !props.liveAudioInputCapabilities) ||
            (!props.liveAudioInputStatusKnown && !props.liveAudioInputStatus.running) ||
            (!props.liveAudioInputStatus.running && props.liveAudioInputStatus.safety_clear_pending)
          }
          onClick={() =>
            props.liveAudioInputStatus.running
              ? void props.onStopLiveAudioInput()
              : void props.onStartLiveAudioInput()
          }
        >
          {props.liveAudioInputBusy
            ? "Checking"
            : props.liveAudioInputStatus.running
              ? "Stop"
              : props.liveAudioInputStatus.safety_clear_pending
                ? "Clear Pending"
                : props.liveAudioInputStatusKnown
                  ? "Start"
                  : "Checking"}
        </button>
      </div>
      <div
        class={`liveAudioMeters ${props.liveAudioInputStatus.onset ? "onset" : ""}`}
        aria-label="Live audio frequency levels"
        data-onset={props.liveAudioInputStatus.onset ? "true" : "false"}
      >
        <div class="liveAudioBandSpectrum" aria-hidden="true" data-no-localize>
          <For each={liveAudioBandIndices}>
            {(index) => {
              const percent = () => visualBandPercent(index);
              return (
                <em title={`Band ${index + 1} ${percent()}%`}>
                  <i style={{ transform: `scaleY(${percent() / 100})` }} />
                </em>
              );
            }}
          </For>
        </div>
        <For each={meterBands}>
          {([label, name, field]) => {
            const percent = () =>
              Math.round(Math.max(0, Math.min(1, props.liveAudioInputStatus[field])) * 100);
            return (
              <span
                role="meter"
                aria-label={`${name} level`}
                aria-valuemin="0"
                aria-valuemax="100"
                aria-valuenow={percent()}
                aria-valuetext={`${percent()} percent`}
                title={`${name} ${percent()}%`}
              >
                <i style={{ width: `${percent()}%` }} />
                <b>{label}</b>
              </span>
            );
          }}
        </For>
      </div>
      <Show when={health() === "stopped"}>
        <div class="liveAudioConfigControls">
          <label>
            <span>Rate</span>
            <select
              aria-label="Audio input sample rate"
              disabled={inputLocked() || props.liveAudioInputCapabilitiesBusy}
              value={props.liveAudioInputSampleRate ?? ""}
              onInput={(event) =>
                props.onSetLiveAudioInputSampleRate(
                  event.currentTarget.value ? Number(event.currentTarget.value) : null,
                )
              }
            >
              <option value="">Rate auto</option>
              <For each={selectableSampleRates()}>
                {(sampleRate) => <option value={sampleRate}>{(sampleRate / 1_000).toFixed(1)} kHz</option>}
              </For>
            </select>
          </label>
          <label>
            <span>Buffer</span>
            <select
              aria-label="Audio input requested buffer frames"
              disabled={inputLocked() || props.liveAudioInputCapabilitiesBusy}
              value={props.liveAudioInputBufferFrames ?? ""}
              onInput={(event) =>
                props.onSetLiveAudioInputBufferFrames(
                  event.currentTarget.value ? Number(event.currentTarget.value) : null,
                )
              }
            >
              <option value="">Buffer default</option>
              <For each={selectableBufferFrames()}>
                {(frames) => <option value={frames}>{frames} frames</option>}
              </For>
            </select>
          </label>
          <label>
            <span>Mix</span>
            <select
              aria-label="Audio input channel mix"
              disabled={inputLocked() || props.liveAudioInputCapabilitiesBusy}
              value={channelMixValue(props.liveAudioInputChannelMix)}
              onInput={(event) => props.onSetLiveAudioInputChannelMix(parseChannelMix(event.currentTarget.value))}
            >
              <option value="average_all">All → mono</option>
              <For each={Array.from({ length: inputChannels() }, (_, index) => index)}>
                {(channelIndex) => <option value={`single:${channelIndex}`}>Ch {channelIndex + 1}</option>}
              </For>
              <For each={stereoPairs()}>
                {([leftChannelIndex, rightChannelIndex]) => (
                  <option value={`stereo_pair:${leftChannelIndex}:${rightChannelIndex}`}>
                    Ch {leftChannelIndex + 1}+{rightChannelIndex + 1} → mono
                  </option>
                )}
              </For>
            </select>
          </label>
          <span class="liveAudioConfigFormat" title={configFormat()}>
            {configFormat()}
          </span>
        </div>
      </Show>
      <Show when={health() === "live"}>
        <div class="liveAudioTelemetry tabularNums" title={detail()} data-no-localize>
          <span class={`liveAudioTelemetryOvr ${props.liveAudioInputStatus.dropped_chunks > 0 ? "warning" : ""}`}>
            OVR {props.liveAudioInputStatus.dropped_chunks}/{props.liveAudioInputStatus.dropped_frames}f
          </span>
          <span class="liveAudioTelemetryLatency">
            C→W {(props.liveAudioInputStatus.capture_to_worker_us / 1_000).toFixed(1)}/
            {(props.liveAudioInputStatus.max_capture_to_worker_us / 1_000).toFixed(1)} ms
          </span>
          <span class="liveAudioTelemetryIo">
            <b
              class={`liveAudioTelemetryRhythm ${props.liveAudioInputStatus.onset ? "onset" : ""}`}
              title={`BPM ${rhythmLabel()} · RMS ${Math.round((props.liveAudioInputStatus.rms ?? 0) * 100)}% · Peak ${Math.round((props.liveAudioInputStatus.peak ?? 0) * 100)}%`}
            >
              BPM {rhythmLabel()}
            </b>
            <i>
              I/O {(props.liveAudioInputStatus.sample_rate / 1_000).toFixed(1)}k · {liveAudioChannelMixLabel(props.liveAudioInputStatus)} · CB {props.liveAudioInputStatus.last_callback_frames}/{props.liveAudioInputStatus.min_callback_frames}/{props.liveAudioInputStatus.max_callback_frames}f
            </i>
          </span>
          <span class="liveAudioTelemetryQueue">Q {props.liveAudioInputStatus.queue_depth}/{props.liveAudioInputStatus.queue_depth_high_water}/{props.liveAudioInputStatus.queue_capacity}</span>
        </div>
      </Show>
      <Show when={health() !== "live" && health() !== "stopped"}>
        <small class="liveAudioTelemetry liveAudioSafetyMessage" title={detail()}>{detail()}</small>
      </Show>
      <output class="liveAudioHealthAnnouncement" role="status" aria-live="polite" aria-atomic="true">
        {liveAudioInputAnnouncement(props.liveAudioInputStatus, props.liveAudioInputStatusKnown)}
      </output>
    </section>
  );
}
