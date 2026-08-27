import { createEffect, createMemo, For, Show } from "solid-js";
import type {
  LiveAudioChannelMix,
  LiveAudioInputBackendAvailability,
  LiveAudioInputBackendId,
  LiveAudioInputBackendSummary,
  LiveAudioInputCapabilities,
  LiveAudioInputDeviceSummary,
  LiveAudioInputStatus,
} from "../types";
import {
  liveAudioChannelMixLabel,
  liveAudioInputAnnouncement,
  liveAudioInputAsioSelectionVerdict,
  liveAudioInputBackendCanDispatch,
  liveAudioInputBackendScopeLabel,
  liveAudioInputBackendSelectOptions,
  liveAudioInputBackendState,
  liveAudioInputBackendStateLabel,
  liveAudioInputDetail,
  liveAudioInputHealth,
  liveAudioInputBackendVisibleValue,
  parseLiveAudioInputBackendSummary,
} from "../liveAudioInputPresentation";

export interface LiveAudioInputSavedSelectionState {
  phase: "stale" | "invalid" | "ready";
  reason_code: string | null;
  message: string;
  backend: LiveAudioInputBackendId | null;
}

export interface LiveAudioInputRailProps {
  compact?: boolean;
  liveAudioInputBackends: LiveAudioInputBackendSummary[];
  selectedLiveAudioInputBackend: LiveAudioInputBackendId;
  liveAudioInputBackendsKnown: boolean;
  liveAudioInputBackendsBusy: boolean;
  liveAudioInputBackendError: string | null;
  liveAudioInputDevices: LiveAudioInputDeviceSummary[];
  selectedLiveAudioInputDevice: string;
  lastKnownLiveAudioInputDeviceIdentity: Pick<LiveAudioInputDeviceSummary, "id" | "label"> | null;
  localize: (source: string) => string;
  liveAudioInputCapabilities: LiveAudioInputCapabilities | null;
  liveAudioInputCapabilitiesBusy: boolean;
  liveAudioInputSampleRate: number | null;
  liveAudioInputBufferFrames: number | null;
  liveAudioInputChannelMix: LiveAudioChannelMix;
  liveAudioInputStatus: LiveAudioInputStatus;
  liveAudioInputStatusKnown: boolean;
  liveAudioInputBusy: boolean;
  liveAudioInputSavedSelection: LiveAudioInputSavedSelectionState | null;
  liveAudioInputPersistError: string | null;
  /**
   * A one-attempt, session-only arm emitted by the App only after the operator
   * explicitly requests revalidation of the current invalid ASIO selection.
   */
  liveAudioInputAsioRevalidationEligible: boolean;
  /** The exact current arm may unlock the otherwise-invalid ASIO Start once. */
  liveAudioInputAsioRevalidationArmed: boolean;
  onSetLiveAudioInputBackend: (backend: LiveAudioInputBackendId) => void;
  onSetLiveAudioInputDevice: (deviceName: string) => void;
  onSetLiveAudioInputSampleRate: (sampleRate: number | null) => void;
  onSetLiveAudioInputBufferFrames: (bufferFrames: number | null) => void;
  onSetLiveAudioInputChannelMix: (channelMix: LiveAudioChannelMix) => void;
  onRefreshLiveAudioInputDevices: () => void | Promise<void>;
  onArmLiveAudioInputAsioRevalidation: () => void;
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
  const selectedBackend = createMemo(() =>
    props.liveAudioInputBackends.find(
      (backend) => backend.id === props.selectedLiveAudioInputBackend,
    ),
  );
  const selectedBackendParse = createMemo(() =>
    selectedBackend() ? parseLiveAudioInputBackendSummary(selectedBackend()) : null,
  );
  const selectedBackendAvailability = createMemo<LiveAudioInputBackendAvailability | null>(() => {
    const parsed = selectedBackendParse();
    return parsed && parsed.ok ? parsed.summary.availability : null;
  });
  const selectedBackendStartable = createMemo(() => {
    const backend = selectedBackend();
    return backend !== undefined && liveAudioInputBackendCanDispatch(backend);
  });
  const selectedAvailabilityTag = createMemo(() => {
    const parsed = selectedBackendParse();
    if (!parsed) return "none";
    return parsed.ok ? parsed.summary.availability : "invalid";
  });
  const backendRequiresExplicitDevice = createMemo(
    () => selectedBackend()?.requires_explicit_device ?? props.selectedLiveAudioInputBackend === "asio",
  );
  const selectedDeviceRequiresReselection = createMemo(
    () => Boolean(props.selectedLiveAudioInputDevice && !selectedDevice()),
  );
  const unavailableSelectedDeviceLabel = createMemo(() => {
    const selectedId = props.selectedLiveAudioInputDevice;
    const lastKnown = props.lastKnownLiveAudioInputDeviceIdentity;
    const label = lastKnown?.id === selectedId && lastKnown.label.trim()
      ? lastKnown.label
      : props.localize("Prior audio input");
    return props.localize(`Unavailable: ${label} (${selectedId})`);
  });
  const selectedDeviceLabel = createMemo(() =>
    selectedDeviceRequiresReselection()
      ? unavailableSelectedDeviceLabel()
      : selectedDevice()?.label ?? (backendRequiresExplicitDevice()
        ? "Select ASIO driver"
        : "System default audio input"),
  );
  const configFormat = createMemo(() =>
    selectedDeviceRequiresReselection()
      ? props.localize("Unavailable input")
      : props.liveAudioInputCapabilitiesBusy
        ? "I/O…"
        : props.liveAudioInputCapabilities
          ? `${props.liveAudioInputCapabilities.backend} · ${props.liveAudioInputCapabilities.resolved_config.sample_format} · ${props.liveAudioInputCapabilities.resolved_config.channels}ch`
          : "I/O unavailable",
  );
  const backendState = createMemo(() => liveAudioInputBackendState({
    backend: selectedBackend(),
    backendKnown: props.liveAudioInputBackendsKnown,
    backendBusy:
      props.liveAudioInputBackendsBusy ||
      props.liveAudioInputBusy ||
      props.liveAudioInputCapabilitiesBusy,
    backendError: props.liveAudioInputBackendError,
    devicesAvailable: props.liveAudioInputDevices.length,
    selectedDeviceId: props.selectedLiveAudioInputDevice,
    sampleRate: props.liveAudioInputSampleRate,
    bufferFrames: props.liveAudioInputBufferFrames,
    capabilities: props.liveAudioInputCapabilities,
    status: props.liveAudioInputStatus,
    statusKnown: props.liveAudioInputStatusKnown,
  }));
  const backendStateLabel = createMemo(() => liveAudioInputBackendStateLabel(backendState()));
  const backendDetail = createMemo(() => {
    const backend = selectedBackend();
    if (props.liveAudioInputBackendError?.trim()) return props.liveAudioInputBackendError;
    if (!backend) return "Audio capture backend unavailable.";
    const parsed = selectedBackendParse();
    if (!parsed || !parsed.ok) {
      return `${backend.label} reported an invalid availability contract (${parsed?.reason_code ?? "PARSER_UNAVAILABLE"}) · Start stays locked.`;
    }
    const probeDetail = parsed.summary.availability_detail?.trim()
      ? ` · ${parsed.summary.availability_detail}`
      : "";
    switch (parsed.summary.availability) {
      case "not_packaged":
        return `${parsed.summary.label} is not packaged in this application${probeDetail} · ${parsed.summary.distribution}`;
      case "fault":
        return `${parsed.summary.label} driver probe failed${probeDetail}`;
      case "unsupported":
        return `${parsed.summary.label} is unsupported on this platform${probeDetail}`;
      case "ready":
        break;
    }
    if (backendState() === "empty") return `${backend.label} has no available input drivers.`;
    if (backendState() === "select_device") {
      return backend.requires_explicit_device
        ? `Select a ${backend.label} driver. Automatic driver selection is disabled.`
        : "Resolve an input configuration before Start.";
    }
    if (backendState() === "configure") {
      return "Select an explicit sample rate and fixed buffer for ASIO.";
    }
    return `${backend.label} · ${backend.distribution}`;
  });
  const inputLocked = () =>
    props.liveAudioInputBusy ||
    props.liveAudioInputBackendsBusy ||
    !props.liveAudioInputStatusKnown ||
    props.liveAudioInputStatus.running ||
    props.liveAudioInputStatus.safety_clear_pending;
  const backendStartLocked = () =>
    !props.liveAudioInputBackendsKnown ||
    selectedBackendAvailability() !== "ready" ||
    !selectedBackendStartable() ||
    (backendRequiresExplicitDevice() && !props.selectedLiveAudioInputDevice.trim()) ||
    (backendRequiresExplicitDevice() && props.liveAudioInputSampleRate === null) ||
    (backendRequiresExplicitDevice() && props.liveAudioInputBufferFrames === null) ||
    selectedDeviceRequiresReselection() ||
    asioVerdictStartLocked();
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
  const appliedBufferLabel = createMemo(() => {
    const frames = props.liveAudioInputStatus.applied_buffer_frames;
    return frames === null || frames === undefined ? "pending" : `${frames}f`;
  });
  const savedSelection = createMemo(() => props.liveAudioInputSavedSelection);
  const savedSelectionScope = createMemo(() => {
    const saved = savedSelection();
    if (!saved?.backend) return "INPUT";
    return liveAudioInputBackendScopeLabel(saved.backend);
  });
  const savedSelectionLabel = createMemo(() => {
    const saved = savedSelection();
    if (!saved) return "";
    if (saved.phase === "ready") return `SAVED ${savedSelectionScope()} READY`;
    if (saved.phase === "stale") return `SAVED ${savedSelectionScope()} LOCKED`;
    return "SAVED INPUT INVALID";
  });
  const savedSelectionLockActive = () => {
    const saved = savedSelection();
    return saved !== null &&
      saved.backend === props.selectedLiveAudioInputBackend &&
      saved.phase !== "ready";
  };
  const asioVerdict = createMemo(() => {
    const raw = props.liveAudioInputStatus.asio_selection;
    return raw === null || raw === undefined ? null : liveAudioInputAsioSelectionVerdict(raw);
  });
  const asioVerdictStartLocked = createMemo(() => {
    const verdict = asioVerdict();
    if (props.selectedLiveAudioInputBackend !== "asio" || !verdict) return false;
    // `restored` prevents automatic resume only. A current explicit ASIO Start
    // reaches native revalidation. An `invalid` verdict stays locked unless
    // the App has proved a one-attempt exact-request revalidation arm.
    if (verdict.state === "invalid" && props.liveAudioInputAsioRevalidationArmed) return false;
    return verdict.start_locked && verdict.state !== "restored";
  });
  const asioInvalidRevalidationActionVisible = createMemo(() => {
    const verdict = asioVerdict();
    return props.selectedLiveAudioInputBackend === "asio" &&
      verdict?.contract_valid === true &&
      verdict.state === "invalid";
  });
  const asioVerdictTitle = createMemo(() => {
    const verdict = asioVerdict();
    if (!verdict) return "";
    const reasonSuffix = verdict.reason === null ? "" : ` · ${verdict.reason}`;
    return `${verdict.state_label}${reasonSuffix} · ${verdict.message}`;
  });
  const backendSelectOptions = createMemo(() =>
    liveAudioInputBackendSelectOptions({
      selectedBackend: props.selectedLiveAudioInputBackend,
      backends: props.liveAudioInputBackends,
      backendsKnown: props.liveAudioInputBackendsKnown,
      savedBackend: savedSelection()?.backend ?? null,
    }),
  );
  let backendSelect: HTMLSelectElement | undefined;
  createEffect(() => {
    const select = backendSelect;
    if (!select) return;
    const visible = liveAudioInputBackendVisibleValue(
      backendSelectOptions(),
      props.selectedLiveAudioInputBackend,
    );
    if (visible !== null && select.value !== visible) select.value = visible;
  });

  return (
    <section
      class={`liveAudioInputBar ${props.compact ? "compact" : ""} ${props.liveAudioInputStatus.running ? "active" : ""} ${health()}`}
      data-health={health()}
      data-live-audio-backend={props.selectedLiveAudioInputBackend}
      data-live-audio-backend-state={backendState()}
      data-live-audio-backend-availability={
        props.liveAudioInputBackendsKnown ? selectedAvailabilityTag() : "unknown"
      }
      data-live-audio-backend-built={selectedBackend()?.built ? "true" : "false"}
      aria-label="Live audio analysis input"
    >
      <div class="liveAudioInputControls">
        <label>
          <span class="liveAudioInputLabel">Audio input</span>
          <select
            aria-label="Live audio input device"
            data-live-audio-control="device"
            disabled={inputLocked() || !selectedBackendStartable()}
            value={props.selectedLiveAudioInputDevice}
            title={selectedDeviceLabel()}
            aria-invalid={selectedDeviceRequiresReselection() ? "true" : undefined}
            onInput={(event) => props.onSetLiveAudioInputDevice(event.currentTarget.value)}
          >
            <Show when={selectedDeviceRequiresReselection()}>
              <option
                value={props.selectedLiveAudioInputDevice}
                selected={selectedDeviceRequiresReselection()}
                disabled
                data-live-audio-stale-device="true"
              >
                {unavailableSelectedDeviceLabel()}
              </option>
            </Show>
            <Show
              when={!backendRequiresExplicitDevice()}
              fallback={<option value="">Select ASIO driver</option>}
            >
              <option value="">System default</option>
            </Show>
            <For each={props.liveAudioInputDevices}>
              {(device) => <option value={device.id} data-no-localize>{device.label}</option>}
            </For>
          </select>
        </label>
        <button
          data-live-audio-action="refresh"
          disabled={props.liveAudioInputBusy || props.liveAudioInputBackendsBusy || props.liveAudioInputStatus.running}
          aria-label="Refresh audio input devices"
          title="Refresh audio input devices"
          onClick={() => void props.onRefreshLiveAudioInputDevices()}
        >
          ↻
        </button>
        <button
          class={props.liveAudioInputStatus.running ? "danger" : "primary"}
          data-live-audio-action="transport"
          disabled={
            props.liveAudioInputBusy ||
            props.liveAudioInputBackendsBusy ||
            (!props.liveAudioInputStatus.running && savedSelectionLockActive()) ||
            (!props.liveAudioInputStatus.running && backendStartLocked()) ||
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
        <Show when={savedSelection()}>
          <span
            class={`liveAudioConfigFormat liveAudioSavedSelection ${savedSelection()?.phase ?? ""}`}
            data-live-audio-saved-state={savedSelection()?.phase ?? ""}
            data-live-audio-saved-reason={savedSelection()?.reason_code ?? ""}
            title={`${savedSelection()?.message ?? ""}${
              props.liveAudioInputPersistError ? ` · Save failed: ${props.liveAudioInputPersistError}` : ""
            }`}
          >
            <b>{savedSelectionLabel()}</b>
            <Show when={props.liveAudioInputPersistError}>
              <i aria-hidden="true">·</i>
              <span>SAVE FAILED</span>
            </Show>
          </span>
        </Show>
        <Show when={!savedSelection() && props.liveAudioInputPersistError}>
          <span
            class="liveAudioConfigFormat liveAudioSavedSelection persist-failed"
            data-live-audio-saved-state="persist-failed"
            title={`Save failed: ${props.liveAudioInputPersistError ?? ""}`}
          >
            <b>SAVE FAILED</b>
          </span>
        </Show>
        <Show when={asioVerdict()}>
          <span
            class={`liveAudioConfigFormat liveAudioAsioVerdict ${asioVerdict()?.contract_valid ? "" : "contract-invalid"}`}
            data-live-audio-asio-verdict={asioVerdict()?.state ?? ""}
            data-live-audio-asio-contract={asioVerdict()?.contract_valid ? "valid" : "invalid"}
            data-live-audio-asio-start-locked={asioVerdict()?.start_locked ? "true" : "false"}
            data-live-audio-asio-reason={asioVerdict()?.reason ?? ""}
            data-live-audio-asio-message={asioVerdict()?.message ?? ""}
            title={asioVerdictTitle()}
          >
            <b>{asioVerdict()?.state_label}</b>
            <i aria-hidden="true">·</i>
            <span>{asioVerdict()?.reason ? `${asioVerdict()?.reason} · ${asioVerdict()?.message}` : asioVerdict()?.message}</span>
          </span>
        </Show>
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
          <label class="liveAudioBackendControl">
            <span>Backend</span>
            <select
              ref={backendSelect}
              aria-label="Audio input backend"
              data-live-audio-control="backend"
              disabled={inputLocked()}
              value={props.selectedLiveAudioInputBackend}
              onInput={(event) => props.onSetLiveAudioInputBackend(event.currentTarget.value as LiveAudioInputBackendId)}
            >
              <For each={backendSelectOptions()}>
                {(option) => (
                  <option
                    value={option.value}
                    data-no-localize={option.noLocalize || undefined}
                  >
                    {option.label}
                  </option>
                )}
              </For>
            </select>
          </label>
          <label>
            <span>Rate</span>
            <select
              aria-label="Audio input sample rate"
              data-live-audio-control="rate"
              disabled={inputLocked() || props.liveAudioInputCapabilitiesBusy}
              value={props.liveAudioInputSampleRate ?? ""}
              onInput={(event) =>
                props.onSetLiveAudioInputSampleRate(
                  event.currentTarget.value ? Number(event.currentTarget.value) : null,
                )
              }
            >
              <Show
                when={!backendRequiresExplicitDevice()}
                fallback={<option value="">Select rate</option>}
              >
                <option value="">Rate auto</option>
              </Show>
              <For each={selectableSampleRates()}>
                {(sampleRate) => <option value={sampleRate}>{(sampleRate / 1_000).toFixed(1)} kHz</option>}
              </For>
            </select>
          </label>
          <label>
            <span>Buffer</span>
            <select
              aria-label="Audio input requested buffer frames"
              data-live-audio-control="buffer"
              disabled={inputLocked() || props.liveAudioInputCapabilitiesBusy}
              value={props.liveAudioInputBufferFrames ?? ""}
              onInput={(event) =>
                props.onSetLiveAudioInputBufferFrames(
                  event.currentTarget.value ? Number(event.currentTarget.value) : null,
                )
              }
            >
              <Show
                when={!backendRequiresExplicitDevice()}
                fallback={<option value="">Select fixed buffer</option>}
              >
                <option value="">Buffer default</option>
              </Show>
              <For each={selectableBufferFrames()}>
                {(frames) => <option value={frames}>{frames} frames</option>}
              </For>
            </select>
          </label>
          <label>
            <span>Mix</span>
            <select
              aria-label="Audio input channel mix"
              data-live-audio-control="mix"
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
          <Show
            when={asioInvalidRevalidationActionVisible()}
            fallback={
              <span
                class={`liveAudioConfigFormat liveAudioBackendState ${backendState()}`}
                data-live-audio-backend-state-label={backendStateLabel()}
                title={`${backendStateLabel()} · ${backendDetail()} · ${configFormat()}`}
              >
                <b>{backendStateLabel()}</b>
                <i aria-hidden="true">·</i>
                <span>{selectedBackendStartable() ? configFormat() : backendDetail()}</span>
              </span>
            }
          >
            <button
              data-live-audio-action="asio-revalidate"
              disabled={
                inputLocked() ||
                props.liveAudioInputBackendsBusy ||
                !props.liveAudioInputAsioRevalidationEligible
              }
              title="Arm one exact current ASIO Start for native revalidation"
              onClick={() => props.onArmLiveAudioInputAsioRevalidation()}
            >
              Revalidate current ASIO selection
            </button>
          </Show>
        </div>
      </Show>
      <Show when={health() === "live"}>
        <div class="liveAudioTelemetry tabularNums" title={detail()} data-no-localize>
          <span class={`liveAudioBackendLiveState ${backendState()}`}>
            {backendStateLabel()}
          </span>
          <span class={`liveAudioTelemetryOvr ${props.liveAudioInputStatus.dropped_chunks > 0 ? "warning" : ""}`}>
            OVR {props.liveAudioInputStatus.dropped_chunks}/{props.liveAudioInputStatus.dropped_frames}f
          </span>
          <span class={`liveAudioTelemetryXrun ${(props.liveAudioInputStatus.backend_xruns ?? 0) > 0 ? "warning" : ""}`}>
            XRUN {props.liveAudioInputStatus.backend_xruns ?? 0}
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
              I/O {(props.liveAudioInputStatus.sample_rate / 1_000).toFixed(1)}k · {liveAudioChannelMixLabel(props.liveAudioInputStatus)} · BUF {appliedBufferLabel()} · CB {props.liveAudioInputStatus.last_callback_frames}/{props.liveAudioInputStatus.min_callback_frames}/{props.liveAudioInputStatus.max_callback_frames}f
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
