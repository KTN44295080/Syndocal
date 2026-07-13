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

const COMMON_SAMPLE_RATES = [44_100, 48_000, 88_200, 96_000, 176_400, 192_000] as const;
const COMMON_BUFFER_FRAMES = [64, 128, 256, 512, 1_024, 2_048] as const;

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
    const rates = new Set<number>([capabilities.default_config.sample_rate]);
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
    const sampleRate = props.liveAudioInputSampleRate ?? capabilities.default_config.sample_rate;
    return COMMON_BUFFER_FRAMES.filter(
      (frames) =>
        frames <= capabilities.max_capture_frames &&
        capabilities.supported_configs.some(
          (config) =>
            config.min_sample_rate <= sampleRate &&
            sampleRate <= config.max_sample_rate &&
            (config.buffer_size.kind === "unknown" ||
              (config.buffer_size.kind === "range" &&
              config.buffer_size.min_frames <= frames &&
              frames <= config.buffer_size.max_frames)),
        ),
    );
  });
  const inputChannels = createMemo(
    () => props.liveAudioInputCapabilities?.default_config.channels ?? 0,
  );
  const inputLocked = () =>
    props.liveAudioInputBusy ||
    !props.liveAudioInputStatusKnown ||
    props.liveAudioInputStatus.running ||
    props.liveAudioInputStatus.safety_clear_pending;

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
            title={props.selectedLiveAudioInputDevice || "System default audio input"}
            onInput={(event) => props.onSetLiveAudioInputDevice(event.currentTarget.value)}
          >
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
            props.liveAudioInputCapabilitiesBusy ||
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
      <div class="liveAudioMeters" aria-label="Live audio frequency levels">
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
              <Show when={inputChannels() >= 2}>
                <option value="stereo_pair:0:1">Ch 1+2 → mono</option>
              </Show>
            </select>
          </label>
          <span class="liveAudioConfigFormat" data-no-localize>
            {props.liveAudioInputCapabilitiesBusy
              ? "I/O…"
              : props.liveAudioInputCapabilities
                ? `${props.liveAudioInputCapabilities.backend} · ${props.liveAudioInputCapabilities.default_config.sample_format}`
                : "I/O unavailable"}
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
            I/O {(props.liveAudioInputStatus.sample_rate / 1_000).toFixed(1)}k · {liveAudioChannelMixLabel(props.liveAudioInputStatus)} · CB {props.liveAudioInputStatus.last_callback_frames}/{props.liveAudioInputStatus.min_callback_frames}/{props.liveAudioInputStatus.max_callback_frames}f
          </span>
          <span class="liveAudioTelemetryQueue">Q {props.liveAudioInputStatus.queue_depth}/{props.liveAudioInputStatus.queue_depth_high_water}/{props.liveAudioInputStatus.queue_capacity}</span>
        </div>
      </Show>
      <Show when={health() !== "live" && health() !== "stopped"}>
        <small class="liveAudioTelemetry liveAudioSafetyMessage">{detail()}</small>
      </Show>
      <output class="liveAudioHealthAnnouncement" role="status" aria-live="polite" aria-atomic="true">
        {liveAudioInputAnnouncement(props.liveAudioInputStatus, props.liveAudioInputStatusKnown)}
      </output>
    </section>
  );
}
