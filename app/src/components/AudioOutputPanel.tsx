import { For, Show } from "solid-js";
import type {
  MachineTimelineCueAudioSettingsV1,
  TimelineCueAudioStatus,
} from "../types";
import { TimelineCueAudioRoutingPanel } from "./TimelineCueAudioRoutingPanel";
import "./AudioOutputPanel.css";

export type AudioOutputBackend = "normal-wasapi" | "show-asio";
export type AudioOutputState = "Ready" | "Locked" | "Active" | "Fault";
export type AudioOutputCueRoute = "same-asio" | "split-device";
export type AudioOutputTest =
  | "program-left"
  | "program-right"
  | "program-stereo"
  | "cue"
  | "spare";
export type AudioOutputSoloMode = "none" | "program-only" | "cue-only";

export interface AudioOutputBackendOption {
  value: AudioOutputBackend;
  label: string;
}

export interface AudioOutputDriverOption {
  id: string;
  label: string;
  disabled?: boolean;
}

export interface AudioOutputRateOption {
  value: number;
  label?: string;
}

export interface AudioOutputBufferOption {
  value: number;
  label?: string;
}

/** Channel indexes are zero-based at the application/backend boundary. */
export interface AudioOutputChannelOption {
  index: number;
  disabled?: boolean;
}

export interface AudioOutputCueEndpointOption {
  name: string;
  occurrences: number;
  selectable: boolean;
}

/**
 * The callable shape keeps the pre-existing `onCueChange` prop compatible with
 * the current Setup wiring while carrying the two new route mutations.  The
 * optional members let isolated panel fixtures continue to provide only the
 * original channel callback.
 */
export interface AudioOutputCueChange {
  (channelIndex: number | null): void;
  setRoute?: (route: AudioOutputCueRoute) => void;
  setDeviceName?: (deviceName: string | null) => void;
}

export interface AudioOutputOptions {
  drivers: readonly AudioOutputDriverOption[];
  sampleRates: readonly AudioOutputRateOption[];
  bufferFrames: readonly AudioOutputBufferOption[];
  channels: readonly AudioOutputChannelOption[];
  cueEndpoints: readonly AudioOutputCueEndpointOption[];
  hasSpare: boolean;
}

/** A presentation-ready snapshot; the parent adapts its backend DTO to this shape. */
export interface AudioOutputView {
  backend: AudioOutputBackend;
  driverId: string;
  sampleRate: number | null;
  bufferFrames: number | null;
  programLeft: number | null;
  programRight: number | null;
  cue: number | null;
  cueRoute: AudioOutputCueRoute;
  cueDeviceName: string | null;
  cueTopologyFingerprint: string | null;
  spare: number | null;
  catalogGeneration: number;
  state: AudioOutputState;
  reason: string;
}

type AudioOutputAction = () => void | Promise<void>;

export interface AudioOutputPanelProps {
  view: AudioOutputView;
  options: AudioOutputOptions;
  busy: boolean;
  canRefresh: boolean;
  canRevalidate: boolean;
  canStart: boolean;
  canStop: boolean;
  canReturnToNormal: boolean;
  onBackendChange: (backend: AudioOutputBackend) => void;
  onDriverChange: (driverId: string) => void;
  onSampleRateChange: (sampleRate: number | null) => void;
  onBufferFramesChange: (bufferFrames: number | null) => void;
  onProgramLeftChange: (channelIndex: number | null) => void;
  onProgramRightChange: (channelIndex: number | null) => void;
  onCueChange: AudioOutputCueChange;
  onCueRouteChange?: (route: AudioOutputCueRoute) => void;
  onCueDeviceNameChange?: (deviceName: string | null) => void;
  onSpareChange: (channelIndex: number | null) => void;
  onRefresh: AudioOutputAction;
  onRevalidate: AudioOutputAction;
  onStart: AudioOutputAction;
  onStop: AudioOutputAction;
  onReturnToNormal: AudioOutputAction;
  /** Shared Normal-WASAPI Timeline media/Guide/Click route. */
  timelineCueAudioStatus?: TimelineCueAudioStatus;
  timelineCueAudioMutationBusy?: boolean;
  timelineCueAudioLocalError?: string | null;
  onConfigureTimelineCueAudio?: (settings: MachineTimelineCueAudioSettingsV1) => void;
  onRefreshTimelineCueAudio?: () => void | Promise<void>;
}

export const AUDIO_OUTPUT_BACKEND_OPTIONS: readonly AudioOutputBackendOption[] = [
  { value: "normal-wasapi", label: "Normal WASAPI" },
  { value: "show-asio", label: "ASIO" },
];

const channelOptionLabel = (option: AudioOutputChannelOption): string =>
  `Output ${option.index + 1}`;

const parseNullableChannelIndex = (value: string): number | null => {
  if (value.trim() === "") return null;
  const index = Number(value);
  return Number.isInteger(index) && index >= 0 ? index : null;
};

const rateOptionLabel = (option: AudioOutputRateOption): string =>
  option.label ?? `${option.value.toLocaleString()} Hz`;

const bufferOptionLabel = (option: AudioOutputBufferOption): string =>
  option.label ?? `${option.value.toLocaleString()} frames`;

interface AudioOutputChannelSelectProps {
  field: "program-left" | "program-right" | "cue" | "spare";
  label: string;
  value: number | null;
  disabled: boolean;
  options: readonly AudioOutputChannelOption[];
  onChange: (channelIndex: number | null) => void;
}

interface AudioOutputCueEndpointSelectProps {
  value: string | null;
  disabled: boolean;
  options: readonly AudioOutputCueEndpointOption[];
  onChange: (deviceName: string | null) => void;
}

function AudioOutputChannelSelect(props: AudioOutputChannelSelectProps) {
  const selectedChannelIsListed = () =>
    props.value === null || props.options.some((option) => option.index === props.value);

  return (
    <label class="audioOutputField" data-audio-output-field={props.field}>
      <span>{props.label}</span>
      <select
        aria-label={`${props.label} output channel`}
        disabled={props.disabled}
        value={props.value ?? ""}
        onInput={(event) => props.onChange(parseNullableChannelIndex(event.currentTarget.value))}
      >
        <option value="">Select Output</option>
        <Show when={!selectedChannelIsListed() && props.value !== null}>
          <option value={props.value!} disabled>
            Unavailable {`Output ${props.value! + 1}`}
          </option>
        </Show>
        <For each={props.options}>
          {(option) => (
            <option value={option.index} disabled={option.disabled ?? false}>
              {channelOptionLabel(option)}
            </option>
          )}
        </For>
      </select>
    </label>
  );
}

function AudioOutputCueEndpointSelect(props: AudioOutputCueEndpointSelectProps) {
  const selectedEndpointIsListed = () =>
    props.value === null || props.options.some((option) => option.name === props.value);

  return (
    <label class="audioOutputField" data-audio-output-field="cue-endpoint">
      <span>WDM endpoint</span>
      <select
        aria-label="CUE WDM endpoint"
        disabled={props.disabled}
        value={props.value ?? ""}
        onInput={(event) => props.onChange(event.currentTarget.value.trim() || null)}
      >
        <option value="">Select WDM endpoint</option>
        <Show when={!selectedEndpointIsListed() && props.value !== null}>
          <option value={props.value!} disabled data-no-localize>
            Unavailable: {props.value!}
          </option>
        </Show>
        <For each={props.options}>
          {(option) => (
            <option
              value={option.name}
              disabled={!option.selectable}
              data-no-localize
            >
              {option.name}
            </option>
          )}
        </For>
      </select>
    </label>
  );
}

export function AudioOutputPanel(props: AudioOutputPanelProps) {
  const selectedDriverIsListed = () =>
    props.view.driverId === "" || props.options.drivers.some((option) => option.id === props.view.driverId);
  const configurationDisabled = () =>
    props.busy || props.view.state === "Active" || props.view.state === "Fault";
  const refreshDisabled = () =>
    props.busy || !props.canRefresh || props.view.state === "Active" || props.view.state === "Fault";
  const revalidateDisabled = () =>
    props.busy || !props.canRevalidate || props.view.state === "Active" || props.view.state === "Fault";
  const startDisabled = () =>
    props.busy || !props.canStart || props.view.state !== "Ready";
  const stopDisabled = () =>
    props.busy ||
    !props.canStop ||
    (props.view.state !== "Active" && props.view.state !== "Fault");
  const returnToNormalDisabled = () =>
    props.busy ||
    !props.canReturnToNormal ||
    props.view.backend !== "show-asio" ||
    props.view.state === "Active" ||
    props.view.state === "Fault";
  const cueRouteChange = (route: AudioOutputCueRoute) => {
    const handler = props.onCueRouteChange ?? props.onCueChange.setRoute;
    handler?.(route);
  };
  const cueDeviceNameChange = (deviceName: string | null) => {
    const handler = props.onCueDeviceNameChange ?? props.onCueChange.setDeviceName;
    handler?.(deviceName);
  };
  const cueRouteMutationAvailable = () =>
    props.onCueRouteChange !== undefined || props.onCueChange.setRoute !== undefined;
  const cueDeviceMutationAvailable = () =>
    props.onCueDeviceNameChange !== undefined || props.onCueChange.setDeviceName !== undefined;

  return (
    <section
      class="audioOutputPanel"
      data-audio-output-panel
      data-audio-output-backend={props.view.backend}
      data-audio-output-state={props.view.state}
      aria-labelledby="audio-output-panel-title"
    >
      <header class="audioOutputPanelHeader">
        <div>
          <h3 id="audio-output-panel-title">Audio</h3>
          <p>
            {props.view.backend === "normal-wasapi"
              ? "Windows default PROGRAM output"
              : "PROGRAM stereo and CUE output control"}
          </p>
        </div>
        <output
          class={`audioOutputState audioOutputState--${props.view.state.toLowerCase()}`}
          data-audio-output-status
          aria-live="polite"
        >
          <strong>{props.view.state}</strong>
          <span data-audio-output-reason>{props.view.reason}</span>
        </output>
      </header>

      <Show when={props.view.backend === "show-asio"}>
      <div class="audioOutputActions" aria-label="Audio output actions">
        <button
          type="button"
          data-audio-output-action="refresh"
          disabled={refreshDisabled()}
          aria-label="Refresh audio output"
          onClick={() => void props.onRefresh()}
        >
          Refresh
        </button>
        <button
          type="button"
          data-audio-output-action="revalidate"
          disabled={revalidateDisabled()}
          aria-label="Revalidate audio output"
          onClick={() => void props.onRevalidate()}
        >
          Revalidate
        </button>
        <button
          type="button"
          class="primary"
          data-audio-output-action="start"
          disabled={startDisabled()}
          aria-label="Start audio output"
          onClick={() => void props.onStart()}
        >
          Start
        </button>
        <button
          type="button"
          data-audio-output-action="stop"
          disabled={stopDisabled()}
          aria-label="Stop audio output"
          onClick={() => void props.onStop()}
        >
          Stop
        </button>
        <button
          type="button"
          data-audio-output-action="return-to-normal"
          disabled={returnToNormalDisabled()}
          aria-label="Return audio output to normal"
          onClick={() => void props.onReturnToNormal()}
        >
          Return to normal
        </button>
      </div>
      </Show>

      <details
        class="audioOutputDisclosure"
        data-audio-output-disclosure="configuration"
        open={props.view.backend === "normal-wasapi"}
      >
        <summary>Output configuration</summary>
        <div class="audioOutputDisclosureBody">
          <div
            class="audioOutputFieldGrid audioOutputFieldGrid--backend"
            classList={{ "audioOutputFieldGrid--normal": props.view.backend === "normal-wasapi" }}
          >
            <label class="audioOutputField" data-audio-output-field="backend">
              <span>Backend</span>
              <select
                aria-label="Audio output backend"
                disabled={configurationDisabled()}
                value={props.view.backend}
                onInput={(event) =>
                  props.onBackendChange(event.currentTarget.value as AudioOutputBackend)
                }
              >
                <For each={AUDIO_OUTPUT_BACKEND_OPTIONS}>
                  {(option) => <option value={option.value}>{option.label}</option>}
                </For>
              </select>
            </label>
            <Show when={props.view.backend === "show-asio"}>
            <label class="audioOutputField" data-audio-output-field="driver">
              <span>Driver</span>
              <select
                aria-label="Audio output driver"
                disabled={configurationDisabled() || props.view.backend !== "show-asio"}
                value={props.view.driverId}
                onInput={(event) => props.onDriverChange(event.currentTarget.value)}
              >
                <Show when={props.view.backend === "show-asio"}>
                  <Show when={!selectedDriverIsListed() && props.view.driverId !== ""}>
                    <option value={props.view.driverId} disabled>
                      Unavailable: {props.view.driverId}
                    </option>
                  </Show>
                  <Show when={props.options.drivers.length > 0}>
                    <For each={props.options.drivers}>
                      {(option) => (
                        <option value={option.id} disabled={option.disabled ?? false}>
                          {option.label}
                        </option>
                      )}
                    </For>
                  </Show>
                  <Show when={props.options.drivers.length === 0}>
                    <option value="">No driver enumerated</option>
                  </Show>
                </Show>
                <Show when={props.view.backend === "normal-wasapi"}>
                  <option value="">Normal WASAPI</option>
                </Show>
              </select>
            </label>
            </Show>
            <Show when={props.view.backend === "show-asio"}>
            <label class="audioOutputField" data-audio-output-field="sample-rate">
              <span>Sample rate</span>
              <select
                aria-label="Audio output sample rate"
                disabled={configurationDisabled()}
                value={props.view.sampleRate ?? ""}
                onInput={(event) => {
                  const value = event.currentTarget.value;
                  props.onSampleRateChange(value === "" ? null : Number(value));
                }}
              >
                <option value="">Select rate</option>
                <For each={props.options.sampleRates}>
                  {(option) => <option value={option.value}>{rateOptionLabel(option)}</option>}
                </For>
              </select>
            </label>
            </Show>
            <Show when={props.view.backend === "show-asio"}>
            <label class="audioOutputField" data-audio-output-field="buffer">
              <span>Buffer</span>
              <select
                aria-label="Audio output buffer"
                disabled={configurationDisabled()}
                value={props.view.bufferFrames ?? ""}
                onInput={(event) => {
                  const value = event.currentTarget.value;
                  props.onBufferFramesChange(value === "" ? null : Number(value));
                }}
              >
                <option value="">Select buffer</option>
                <For each={props.options.bufferFrames}>
                  {(option) => <option value={option.value}>{bufferOptionLabel(option)}</option>}
                </For>
              </select>
            </label>
            </Show>
          </div>

          <Show when={props.view.backend === "normal-wasapi"}>
            <p class="audioOutputNormalNotice" role="note" data-audio-output-normal-notice>
              Normal WASAPI uses the Windows default PROGRAM output. Timeline media clips, Guide, and Click can use the exact Windows output selected below.
            </p>
            <Show
              when={
                props.timelineCueAudioStatus
                && props.onConfigureTimelineCueAudio
                && props.onRefreshTimelineCueAudio
              }
            >
              <TimelineCueAudioRoutingPanel
                status={props.timelineCueAudioStatus!}
                mutationBusy={props.timelineCueAudioMutationBusy ?? false}
                localError={props.timelineCueAudioLocalError ?? null}
                onConfigure={props.onConfigureTimelineCueAudio!}
                onRefresh={props.onRefreshTimelineCueAudio!}
              />
            </Show>
          </Show>

          <Show when={props.view.backend === "show-asio"}>
          <div class="audioOutputCueRouting" aria-label="PROGRAM and CUE routing">
            <label class="audioOutputField" data-audio-output-field="cue-route">
              <span>CUE route</span>
              <select
                aria-label="CUE route"
                disabled={
                  configurationDisabled()
                  || props.view.backend !== "show-asio"
                  || !cueRouteMutationAvailable()
                }
                value={props.view.cueRoute}
                onInput={(event) => cueRouteChange(event.currentTarget.value as AudioOutputCueRoute)}
              >
                <option value="same-asio">Same ASIO</option>
                <option value="split-device">Split device</option>
              </select>
            </label>
          </div>

          <div class="audioOutputProgramCueGrid" aria-label="PROGRAM and CUE output routes">
            <div class="audioOutputProgramCueRow" data-audio-output-row="program">
              <strong>PROGRAM</strong>
              <AudioOutputChannelSelect
                field="program-left"
                label="PROGRAM L"
                value={props.view.programLeft}
                disabled={configurationDisabled()}
                options={props.options.channels}
                onChange={props.onProgramLeftChange}
              />
              <AudioOutputChannelSelect
                field="program-right"
                label="PROGRAM R"
                value={props.view.programRight}
                disabled={configurationDisabled()}
                options={props.options.channels}
                onChange={props.onProgramRightChange}
              />
            </div>
            <div class="audioOutputProgramCueRow" data-audio-output-row="cue">
              <strong>CUE</strong>
              <Show when={props.view.cueRoute === "same-asio"}>
                <AudioOutputChannelSelect
                  field="cue"
                  label="CUE"
                  value={props.view.cue}
                  disabled={configurationDisabled()}
                  options={props.options.channels}
                  onChange={props.onCueChange}
                />
              </Show>
              <Show when={props.view.cueRoute === "split-device"}>
                <AudioOutputCueEndpointSelect
                  value={props.view.cueDeviceName}
                  disabled={configurationDisabled() || !cueDeviceMutationAvailable()}
                  options={props.options.cueEndpoints}
                  onChange={cueDeviceNameChange}
                />
              </Show>
            </div>
            <Show when={props.options.hasSpare}>
              <div class="audioOutputProgramCueRow" data-audio-output-row="spare">
                <strong>Spare</strong>
                <AudioOutputChannelSelect
                  field="spare"
                  label="Spare"
                  value={props.view.spare}
                  disabled={configurationDisabled()}
                  options={props.options.channels}
                  onChange={props.onSpareChange}
                />
              </div>
            </Show>
          </div>

          <Show when={props.view.cueRoute === "split-device"}>
            <p class="audioOutputClockWarning" data-audio-output-clock-warning role="alert" data-no-localize>
              PROGRAM and CUE use separate device clocks. Timing can drift; no clock lock is claimed.
            </p>
          </Show>
          </Show>
        </div>
      </details>

    </section>
  );
}
