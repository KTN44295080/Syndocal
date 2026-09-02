import { createMemo, createSignal, type Accessor } from "solid-js";
import type {
  AudioOutputBackend,
  AudioOutputBufferOption,
  AudioOutputChannelOption,
  AudioOutputCueChange,
  AudioOutputCueEndpointOption,
  AudioOutputCueRoute,
  AudioOutputDriverOption,
  AudioOutputOptions,
  AudioOutputRateOption,
  AudioOutputSoloMode,
  AudioOutputState,
  AudioOutputTest,
  AudioOutputView,
} from "./components/AudioOutputPanel";
import type { FrontendTauriInvoke } from "./tauriInvokeCommands";

const ASIO_SCHEMA_VERSION = 3;
const ASIO_ABI_VERSION = 3;
const ASIO_BACKEND = "asio-sdk-v3-rt";
const MACHINE_PROFILE_SCHEMA_VERSION = 2;

type JsonRecord = Record<string, unknown>;

export interface AsioOutputDriverRecord {
  id: string;
  name: string;
}

export interface AsioOutputDriverCatalog {
  schemaVersion: 3;
  kind: "drivers";
  abiVersion: 3;
  backend: "asio-sdk-v3-rt";
  built: true;
  drivers: readonly AsioOutputDriverRecord[];
}

export interface AsioOutputBufferConstraints {
  min: number;
  max: number;
  preferred: number;
  granularity: number;
}

export interface AsioOutputCapabilityTuple {
  channels: number;
  nativeFormat: string;
  sampleRatesHz: readonly number[];
  fixedBufferFrames: AsioOutputBufferConstraints;
}

export interface AsioOutputCapabilities {
  schemaVersion: 3;
  kind: "capabilities";
  abiVersion: 3;
  backend: "asio-sdk-v3-rt";
  built: true;
  driver: AsioOutputDriverRecord;
  output: AsioOutputCapabilityTuple;
  input: AsioOutputCapabilityTuple | null;
}

/** Exact native status states, including the explicit Normal selection reply. */
export type AsioOutputNativeState = "normal" | "ready" | "locked" | "active" | "fault" | "closed";

/** Exact `audio_output_router::State` Debug representation emitted by native. */
export type AsioOutputRouterState =
  | "Normal"
  | "Quiescing"
  | "AsioReady"
  | "AsioStarting"
  | "AsioActive"
  | "Fault"
  | "Locked";

export interface AsioOutputStatusRecord {
  schemaVersion: 3;
  kind: "status";
  abiVersion: 3;
  backend: "asio-sdk-v3-rt";
  built: true;
  state: AsioOutputNativeState;
  routerState: AsioOutputRouterState;
  catalogGeneration: number;
  profileReady: boolean;
  lastError: string | null;
  lifecycle: string;
  callbacks: number | null;
  xruns: number | null;
  terminalFault: string | null;
}

export interface MachineAudioOutputCueDeliverySameAsio {
  mode: "sameAsio";
  cueOutput: number;
}

export interface MachineAudioOutputCueDeliveryExplicitWdm {
  mode: "explicitWdm";
  deviceName: string;
  topologyFingerprint: string;
}

export type MachineAudioOutputCueDelivery =
  | MachineAudioOutputCueDeliverySameAsio
  | MachineAudioOutputCueDeliveryExplicitWdm;

export type TimelineCueAudioLifecycle =
  | "loading_settings"
  | "disabled_by_project"
  | "waiting_for_program_output"
  | "applying"
  | "running"
  | "missing_device"
  | "ambiguous_device"
  | "topology_changed"
  | "stalled"
  | "fault";

export interface TimelineCueAudioEndpointRecord extends AudioOutputCueEndpointOption {}

export interface TimelineCueAudioSettingsRecord {
  version: number;
  route: "follow_program" | "explicit_device";
  deviceName: string | null;
  topologyFingerprint: string | null;
  clickGain: number;
  guideGain: number;
}

/** Exact backend status used to gate split-device output selection. */
export interface TimelineCueAudioStatusRecord {
  runtimeIncarnation: number;
  statusRevision: number;
  desiredSettings: TimelineCueAudioSettingsRecord;
  appliedSettings: TimelineCueAudioSettingsRecord | null;
  settingsRevision: number;
  lifecycle: TimelineCueAudioLifecycle;
  requestedDeviceName: string | null;
  resolvedDeviceName: string | null;
  requestedTopologyFingerprint: string | null;
  observedTopologyFingerprint: string | null;
  topologyGeneration: number;
  endpoints: readonly TimelineCueAudioEndpointRecord[];
  outputClockEpoch: number;
  scheduleGeneration: number;
  sourceFence: number;
  nextOutputFrame: number;
  callbackLive: boolean;
  faultCode: string;
  faultCount: number;
  faultSequence: number;
  lastError: string | null;
  rotationCount: number;
  stallCount: number;
  configCount: number;
}

export interface MachineAudioOutputProfileDraft {
  driverId: string;
  catalogGeneration: number;
  sampleRateHz: number | null;
  nativeFormat: string;
  fixedBufferFrames: number | null;
  deviceOutputChannels: number;
  programLeft: number | null;
  programRight: number | null;
  cue: number | null;
  cueRoute: AudioOutputCueRoute;
  cueDeviceName: string | null;
  cueTopologyFingerprint: string | null;
  spare: number | null;
}

export interface MachineAudioOutputProfile {
  schemaVersion: 2;
  driverId: string;
  catalogGeneration: number;
  sampleRateHz: number;
  nativeFormat: string;
  fixedBufferFrames: number;
  deviceOutputChannels: number;
  programLeftOutput: number;
  programRightOutput: number;
  cueDelivery: MachineAudioOutputCueDelivery;
  spareOutput?: number;
}

export interface AudioOutputControlDependencies {
  invoke: FrontendTauriInvoke;
  backendAvailable: boolean;
  readLivePlaybackActive: () => boolean;
}

export interface AudioOutputController {
  view: Accessor<AudioOutputView>;
  options: Accessor<AudioOutputOptions>;
  capabilities: Accessor<AsioOutputCapabilities | null>;
  busy: Accessor<boolean>;
  testMode: Accessor<AudioOutputTest | null>;
  soloMode: Accessor<AudioOutputSoloMode>;
  summary: Accessor<string>;
  stateTone: Accessor<"ok" | "idle" | "warning" | "error">;
  canRefresh: Accessor<boolean>;
  canRevalidate: Accessor<boolean>;
  canStart: Accessor<boolean>;
  canStop: Accessor<boolean>;
  canReturnToNormal: Accessor<boolean>;
  canTest: Accessor<boolean>;
  canSolo: Accessor<boolean>;
  livePlaybackActive: Accessor<boolean>;
  setBackend: (backend: AudioOutputBackend) => void;
  setDriver: (driverId: string) => void;
  setSampleRate: (sampleRate: number | null) => void;
  setBufferFrames: (bufferFrames: number | null) => void;
  setProgramLeft: (channelIndex: number | null) => void;
  setProgramRight: (channelIndex: number | null) => void;
  setCue: AudioOutputCueChange;
  setCueRoute: (route: AudioOutputCueRoute) => void;
  setCueDeviceName: (deviceName: string | null) => void;
  setSpare: (channelIndex: number | null) => void;
  refresh: () => Promise<void>;
  refreshCueEndpoints: () => Promise<void>;
  revalidate: () => Promise<void>;
  start: () => Promise<void>;
  stop: () => Promise<void>;
  returnToNormal: () => Promise<void>;
  setTest: (test: AudioOutputTest | null) => void | Promise<void>;
  setSoloMode: (mode: AudioOutputSoloMode) => void;
  dispose: () => void;
}

const isRecord = (value: unknown): value is JsonRecord =>
  typeof value === "object" && value !== null && !Array.isArray(value);

const parseJsonPayload = (raw: unknown, label: string): unknown => {
  if (typeof raw !== "string") return raw;
  try {
    return JSON.parse(raw) as unknown;
  } catch (error) {
    throw new Error(`${label} JSON is malformed: ${String(error)}`);
  }
};

const requireRecord = (value: unknown, label: string): JsonRecord => {
  if (!isRecord(value)) throw new Error(`${label} must be an object`);
  return value;
};

const requireExactKeys = (value: JsonRecord, expected: readonly string[], label: string): void => {
  const actual = Object.keys(value).sort();
  const wanted = [...expected].sort();
  if (actual.length !== wanted.length || actual.some((key, index) => key !== wanted[index])) {
    throw new Error(`${label} has unknown or missing fields`);
  }
};

const requireString = (value: unknown, field: string): string => {
  if (typeof value !== "string" || value.trim() !== value || value.length === 0) {
    throw new Error(`${field} must be a non-empty trimmed string`);
  }
  return value;
};

const requireNullableString = (value: unknown, field: string): string | null => {
  if (value === null) return null;
  return requireString(value, field);
};

const requireInteger = (value: unknown, field: string, minimum = 0): number => {
  if (typeof value !== "number" || !Number.isSafeInteger(value) || value < minimum) {
    throw new Error(`${field} must be an integer >= ${minimum}`);
  }
  return value;
};

const requireFiniteNumber = (value: unknown, field: string): number => {
  if (typeof value !== "number" || !Number.isFinite(value)) {
    throw new Error(`${field} must be a finite number`);
  }
  return value;
};

const requireTrimmedString = (value: unknown, field: string): string => {
  if (typeof value !== "string" || value.trim() !== value) {
    throw new Error(`${field} must be a trimmed string`);
  }
  return value;
};

const requireBoolean = (value: unknown, field: string): boolean => {
  if (typeof value !== "boolean") throw new Error(`${field} must be boolean`);
  return value;
};

const parseTimelineCueAudioSettings = (
  raw: unknown,
  label: string,
): TimelineCueAudioSettingsRecord => {
  const value = requireRecord(raw, label);
  requireExactKeys(
    value,
    ["version", "route", "device_name", "topology_fingerprint", "click_gain", "guide_gain"],
    label,
  );
  const route = value.route;
  if (route !== "follow_program" && route !== "explicit_device") {
    throw new Error(`${label}.route is unknown`);
  }
  const version = requireInteger(value.version, `${label}.version`, 1);
  if (version !== 1) throw new Error(`${label}.version is unsupported`);
  return {
    version,
    route,
    deviceName: requireNullableString(value.device_name, `${label}.device_name`),
    topologyFingerprint: requireNullableString(value.topology_fingerprint, `${label}.topology_fingerprint`),
    clickGain: requireFiniteNumber(value.click_gain, `${label}.click_gain`),
    guideGain: requireFiniteNumber(value.guide_gain, `${label}.guide_gain`),
  };
};

const parseTimelineCueAudioEndpoint = (
  raw: unknown,
  label: string,
): TimelineCueAudioEndpointRecord => {
  const value = requireRecord(raw, label);
  requireExactKeys(value, ["name", "occurrences", "selectable"], label);
  return {
    name: requireString(value.name, `${label}.name`),
    occurrences: requireInteger(value.occurrences, `${label}.occurrences`, 1),
    selectable: requireBoolean(value.selectable, `${label}.selectable`),
  };
};

export const parseTimelineCueAudioStatus = (raw: unknown): TimelineCueAudioStatusRecord => {
  const value = requireRecord(
    parseJsonPayload(raw, "Timeline CUE audio status"),
    "Timeline CUE audio status",
  );
  requireExactKeys(
    value,
    [
      "runtimeIncarnation",
      "statusRevision",
      "desiredSettings",
      "appliedSettings",
      "settingsRevision",
      "lifecycle",
      "requestedDeviceName",
      "resolvedDeviceName",
      "requestedTopologyFingerprint",
      "observedTopologyFingerprint",
      "topologyGeneration",
      "endpoints",
      "outputClockEpoch",
      "scheduleGeneration",
      "sourceFence",
      "nextOutputFrame",
      "callbackLive",
      "faultCode",
      "faultCount",
      "faultSequence",
      "lastError",
      "rotationCount",
      "stallCount",
      "configCount",
    ],
    "Timeline CUE audio status",
  );
  const lifecycle = value.lifecycle;
  const knownLifecycles: readonly TimelineCueAudioLifecycle[] = [
    "loading_settings",
    "disabled_by_project",
    "waiting_for_program_output",
    "applying",
    "running",
    "missing_device",
    "ambiguous_device",
    "topology_changed",
    "stalled",
    "fault",
  ];
  if (typeof lifecycle !== "string" || !knownLifecycles.includes(lifecycle as TimelineCueAudioLifecycle)) {
    throw new Error("Timeline CUE audio status.lifecycle is unknown");
  }
  if (!Array.isArray(value.endpoints)) {
    throw new Error("Timeline CUE audio status.endpoints must be an array");
  }
  const endpoints = value.endpoints.map((endpoint, index) =>
    parseTimelineCueAudioEndpoint(endpoint, `Timeline CUE audio status.endpoints[${index}]`));
  const faultCode = requireTrimmedString(value.faultCode, "Timeline CUE audio status.faultCode");
  return {
    runtimeIncarnation: requireInteger(value.runtimeIncarnation, "Timeline CUE audio status.runtimeIncarnation"),
    statusRevision: requireInteger(value.statusRevision, "Timeline CUE audio status.statusRevision"),
    desiredSettings: parseTimelineCueAudioSettings(
      value.desiredSettings,
      "Timeline CUE audio status.desiredSettings",
    ),
    appliedSettings: value.appliedSettings === null
      ? null
      : parseTimelineCueAudioSettings(value.appliedSettings, "Timeline CUE audio status.appliedSettings"),
    settingsRevision: requireInteger(value.settingsRevision, "Timeline CUE audio status.settingsRevision"),
    lifecycle: lifecycle as TimelineCueAudioLifecycle,
    requestedDeviceName: requireNullableString(
      value.requestedDeviceName,
      "Timeline CUE audio status.requestedDeviceName",
    ),
    resolvedDeviceName: requireNullableString(
      value.resolvedDeviceName,
      "Timeline CUE audio status.resolvedDeviceName",
    ),
    requestedTopologyFingerprint: requireNullableString(
      value.requestedTopologyFingerprint,
      "Timeline CUE audio status.requestedTopologyFingerprint",
    ),
    observedTopologyFingerprint: requireNullableString(
      value.observedTopologyFingerprint,
      "Timeline CUE audio status.observedTopologyFingerprint",
    ),
    topologyGeneration: requireInteger(value.topologyGeneration, "Timeline CUE audio status.topologyGeneration"),
    endpoints,
    outputClockEpoch: requireInteger(value.outputClockEpoch, "Timeline CUE audio status.outputClockEpoch"),
    scheduleGeneration: requireInteger(value.scheduleGeneration, "Timeline CUE audio status.scheduleGeneration"),
    sourceFence: requireInteger(value.sourceFence, "Timeline CUE audio status.sourceFence"),
    nextOutputFrame: requireInteger(value.nextOutputFrame, "Timeline CUE audio status.nextOutputFrame"),
    callbackLive: requireBoolean(value.callbackLive, "Timeline CUE audio status.callbackLive"),
    faultCode,
    faultCount: requireInteger(value.faultCount, "Timeline CUE audio status.faultCount"),
    faultSequence: requireInteger(value.faultSequence, "Timeline CUE audio status.faultSequence"),
    lastError: requireNullableString(value.lastError, "Timeline CUE audio status.lastError"),
    rotationCount: requireInteger(value.rotationCount, "Timeline CUE audio status.rotationCount"),
    stallCount: requireInteger(value.stallCount, "Timeline CUE audio status.stallCount"),
    configCount: requireInteger(value.configCount, "Timeline CUE audio status.configCount"),
  };
};

export const parseAudioOutputDeviceNames = (raw: unknown): readonly string[] => {
  const value = parseJsonPayload(raw, "audio output device inventory");
  if (!Array.isArray(value)) throw new Error("audio output device inventory must be an array");
  return value.map((name, index) => requireString(name, `audio output device inventory[${index}]`));
};

const requireAsioHeader = (
  value: JsonRecord,
  kind: "drivers" | "capabilities" | "status",
  keys: readonly string[],
  label: string,
): void => {
  requireExactKeys(value, keys, label);
  if (value.schemaVersion !== ASIO_SCHEMA_VERSION
    || value.kind !== kind
    || value.abiVersion !== ASIO_ABI_VERSION
    || value.backend !== ASIO_BACKEND
    || value.built !== true) {
    throw new Error(`${label} is not an exact schemaVersion3 ${kind} response`);
  }
};

const parseDriverRecord = (value: unknown, label: string): AsioOutputDriverRecord => {
  const record = requireRecord(value, label);
  requireExactKeys(record, ["id", "name"], label);
  const id = requireString(record.id, `${label}.id`);
  if (!id.startsWith("asio:")) throw new Error(`${label}.id must be an explicit asio identity`);
  return { id, name: requireString(record.name, `${label}.name`) };
};

export const parseAsioOutputDriverCatalog = (raw: unknown): AsioOutputDriverCatalog => {
  const value = requireRecord(parseJsonPayload(raw, "ASIO output driver catalog"), "ASIO output driver catalog");
  requireAsioHeader(
    value,
    "drivers",
    ["schemaVersion", "kind", "abiVersion", "backend", "built", "drivers"],
    "ASIO output driver catalog",
  );
  if (!Array.isArray(value.drivers)) throw new Error("ASIO output driver catalog.drivers must be an array");
  const drivers = value.drivers.map((driver, index) => parseDriverRecord(driver, `drivers[${index}]`));
  const ids = new Set<string>();
  for (const driver of drivers) {
    if (ids.has(driver.id)) throw new Error(`ASIO output driver catalog contains duplicate ${driver.id}`);
    ids.add(driver.id);
  }
  return {
    schemaVersion: 3,
    kind: "drivers",
    abiVersion: 3,
    backend: ASIO_BACKEND,
    built: true,
    drivers,
  };
};

const parseBufferConstraints = (value: unknown, label: string): AsioOutputBufferConstraints => {
  const record = requireRecord(value, label);
  requireExactKeys(record, ["min", "max", "preferred", "granularity"], label);
  const min = requireInteger(record.min, `${label}.min`, 1);
  const max = requireInteger(record.max, `${label}.max`, min);
  const preferred = requireInteger(record.preferred, `${label}.preferred`, min);
  // ASIO uses -1 to signal that the driver accepts any buffer size in its
  // reported range. Preserve that exact native sentinel instead of treating
  // the capability tuple as malformed.
  const granularity = requireInteger(record.granularity, `${label}.granularity`, -1);
  if (preferred > max) throw new Error(`${label}.preferred must be <= max`);
  return { min, max, preferred, granularity };
};

const parseCapabilityTuple = (value: unknown, label: string): AsioOutputCapabilityTuple => {
  const record = requireRecord(value, label);
  requireExactKeys(record, ["channels", "nativeFormat", "sampleRatesHz", "fixedBufferFrames"], label);
  const channels = requireInteger(record.channels, `${label}.channels`, 1);
  const nativeFormat = requireString(record.nativeFormat, `${label}.nativeFormat`);
  if (!["f32", "i16", "i24", "i32", "f64"].includes(nativeFormat)) {
    throw new Error(`${label}.nativeFormat is unsupported`);
  }
  if (!Array.isArray(record.sampleRatesHz) || record.sampleRatesHz.length === 0) {
    throw new Error(`${label}.sampleRatesHz must be a non-empty array`);
  }
  const sampleRatesHz = record.sampleRatesHz.map((rate, index) =>
    requireInteger(rate, `${label}.sampleRatesHz[${index}]`, 1));
  if (!sampleRatesHz.includes(48_000)) throw new Error(`${label}.sampleRatesHz must include 48000`);
  return {
    channels,
    nativeFormat,
    sampleRatesHz,
    fixedBufferFrames: parseBufferConstraints(record.fixedBufferFrames, `${label}.fixedBufferFrames`),
  };
};

export const parseAsioOutputCapabilities = (raw: unknown): AsioOutputCapabilities => {
  const value = requireRecord(parseJsonPayload(raw, "ASIO output capabilities"), "ASIO output capabilities");
  requireAsioHeader(
    value,
    "capabilities",
    ["schemaVersion", "kind", "abiVersion", "backend", "built", "driver", "output", "input"],
    "ASIO output capabilities",
  );
  const input = value.input === null
    ? null
    : parseCapabilityTuple(value.input, "ASIO output capabilities.input");
  return {
    schemaVersion: 3,
    kind: "capabilities",
    abiVersion: 3,
    backend: ASIO_BACKEND,
    built: true,
    driver: parseDriverRecord(value.driver, "ASIO output capabilities.driver"),
    output: parseCapabilityTuple(value.output, "ASIO output capabilities.output"),
    input,
  };
};

export const parseAsioOutputStatus = (raw: unknown): AsioOutputStatusRecord => {
  const value = requireRecord(parseJsonPayload(raw, "ASIO output status"), "ASIO output status");
  requireAsioHeader(
    value,
    "status",
    [
      "schemaVersion",
      "kind",
      "abiVersion",
      "backend",
      "built",
      "state",
      "routerState",
      "catalogGeneration",
      "profileReady",
      "lastError",
      "lifecycle",
      "callbacks",
      "xruns",
      "terminalFault",
    ],
    "ASIO output status",
  );
  const state = String(value.state);
  if (!["normal", "ready", "locked", "active", "fault", "closed"].includes(state)) {
    throw new Error("ASIO output status.state is unknown");
  }
  const routerState = String(value.routerState);
  if (!["Normal", "Quiescing", "AsioReady", "AsioStarting", "AsioActive", "Fault", "Locked"].includes(routerState)) {
    throw new Error("ASIO output status.routerState is unknown");
  }
  const lifecycle = requireString(value.lifecycle, "ASIO output status.lifecycle");
  if (!(["Locked", "Starting", "Active", "Closed"].includes(lifecycle)
    || (lifecycle.startsWith("Fault(") && lifecycle.endsWith(")")))) {
    throw new Error("ASIO output status.lifecycle is unknown");
  }
  const nullableCounter = (counter: unknown, field: string): number | null =>
    counter === null ? null : requireInteger(counter, field);
  return {
    schemaVersion: 3,
    kind: "status",
    abiVersion: 3,
    backend: ASIO_BACKEND,
    built: true,
    state: state as AsioOutputNativeState,
    routerState: routerState as AsioOutputRouterState,
    catalogGeneration: requireInteger(value.catalogGeneration, "ASIO output status.catalogGeneration"),
    profileReady: requireBoolean(value.profileReady, "ASIO output status.profileReady"),
    lastError: requireNullableString(value.lastError, "ASIO output status.lastError"),
    lifecycle,
    callbacks: nullableCounter(value.callbacks, "ASIO output status.callbacks"),
    xruns: nullableCounter(value.xruns, "ASIO output status.xruns"),
    terminalFault: requireNullableString(value.terminalFault, "ASIO output status.terminalFault"),
  };
};

const channelToOneBased = (index: number | null, field: string, deviceChannels: number): number => {
  if (index === null || !Number.isSafeInteger(index) || index < 0 || index >= deviceChannels) {
    throw new Error(`${field} must select one in-range output channel`);
  }
  return index + 1;
};

export const buildMachineAudioOutputProfile = (
  draft: MachineAudioOutputProfileDraft,
): MachineAudioOutputProfile => {
  const driverId = requireString(draft.driverId, "driverId");
  if (!driverId.startsWith("asio:")) throw new Error("driverId must be an explicit asio identity");
  const catalogGeneration = requireInteger(draft.catalogGeneration, "catalogGeneration", 1);
  const sampleRateHz = draft.sampleRateHz;
  if (sampleRateHz === null) throw new Error("sampleRateHz must be selected");
  requireInteger(sampleRateHz, "sampleRateHz", 1);
  const nativeFormat = requireString(draft.nativeFormat, "nativeFormat");
  const fixedBufferFrames = draft.fixedBufferFrames;
  if (fixedBufferFrames === null) throw new Error("fixedBufferFrames must be selected");
  requireInteger(fixedBufferFrames, "fixedBufferFrames", 1);
  const deviceOutputChannels = requireInteger(draft.deviceOutputChannels, "deviceOutputChannels", 1);
  const programLeftOutput = channelToOneBased(draft.programLeft, "PROGRAM L", deviceOutputChannels);
  const programRightOutput = channelToOneBased(draft.programRight, "PROGRAM R", deviceOutputChannels);
  if (programLeftOutput === programRightOutput) {
    throw new Error("PROGRAM L and PROGRAM R must use distinct outputs");
  }
  const selected = new Set([programLeftOutput, programRightOutput]);
  let cueDelivery: MachineAudioOutputCueDelivery;
  if (draft.cueRoute === "same-asio") {
    const cueOutput = channelToOneBased(draft.cue, "CUE", deviceOutputChannels);
    if (selected.has(cueOutput)) {
      throw new Error("PROGRAM L, PROGRAM R, and CUE must use distinct outputs");
    }
    selected.add(cueOutput);
    cueDelivery = { mode: "sameAsio", cueOutput };
  } else if (draft.cueRoute === "split-device") {
    if (draft.cue !== null) {
      throw new Error("split-device CUE delivery cannot carry an ASIO CUE channel");
    }
    cueDelivery = {
      mode: "explicitWdm",
      deviceName: requireString(draft.cueDeviceName, "CUE WDM deviceName"),
      topologyFingerprint: requireString(draft.cueTopologyFingerprint, "CUE WDM topologyFingerprint"),
    };
  } else {
    throw new Error("cueRoute must be same-asio or split-device");
  }
  const spareOutput = draft.spare === null
    ? undefined
    : channelToOneBased(draft.spare, "Spare", deviceOutputChannels);
  if (spareOutput !== undefined && selected.has(spareOutput)) {
    throw new Error("Spare must use a distinct output");
  }
  return {
    schemaVersion: MACHINE_PROFILE_SCHEMA_VERSION,
    driverId,
    catalogGeneration,
    sampleRateHz,
    nativeFormat,
    fixedBufferFrames,
    deviceOutputChannels,
    programLeftOutput,
    programRightOutput,
    cueDelivery,
    ...(spareOutput === undefined ? {} : { spareOutput }),
  };
};

export const machineAudioOutputProfileJson = (draft: MachineAudioOutputProfileDraft): string =>
  JSON.stringify(buildMachineAudioOutputProfile(draft));

const normalView = (): AudioOutputView => ({
  backend: "normal-wasapi",
  driverId: "",
  sampleRate: null,
  bufferFrames: null,
  programLeft: null,
  programRight: null,
  cue: null,
  cueRoute: "same-asio",
  cueDeviceName: null,
  cueTopologyFingerprint: null,
  spare: null,
  catalogGeneration: 0,
  state: "Ready",
  reason: "Normal WASAPI is selected.",
});

const emptyOptions = (): AudioOutputOptions => ({
  drivers: [],
  sampleRates: [],
  bufferFrames: [],
  channels: [],
  cueEndpoints: [],
  hasSpare: false,
});

const mergeCueEndpointOptions = (
  statusEndpoints: readonly TimelineCueAudioEndpointRecord[],
  enumeratedNames: readonly string[],
): readonly AudioOutputCueEndpointOption[] => {
  const occurrencesByName = new Map<string, number>();
  for (const name of enumeratedNames) {
    occurrencesByName.set(name, (occurrencesByName.get(name) ?? 0) + 1);
  }
  const statusOccurrencesByName = new Map<string, number>();
  for (const endpoint of statusEndpoints) {
    statusOccurrencesByName.set(
      endpoint.name,
      (statusOccurrencesByName.get(endpoint.name) ?? 0) + 1,
    );
  }
  const endpointsByName = new Map<string, AudioOutputCueEndpointOption>();
  for (const endpoint of statusEndpoints) {
    const occurrences = Math.max(
      occurrencesByName.get(endpoint.name) ?? 0,
      endpoint.occurrences,
      statusOccurrencesByName.get(endpoint.name) ?? 0,
    );
    endpointsByName.set(endpoint.name, {
      name: endpoint.name,
      occurrences,
      // A repeated exact name is never safe to select, even if a stale or
      // malformed status claims it is selectable.
      selectable: statusOccurrencesByName.get(endpoint.name) === 1
        && occurrences === 1
        && endpoint.selectable,
    });
  }
  for (const name of enumeratedNames) {
    if (endpointsByName.has(name)) continue;
    const occurrences = occurrencesByName.get(name) ?? 0;
    endpointsByName.set(name, {
      name,
      occurrences,
      selectable: occurrences === 1,
    });
  }
  return [...endpointsByName.values()];
};

const errorText = (error: unknown): string =>
  error instanceof Error ? error.message : String(error);

const isMissingNativeCommand = (error: unknown, command: string): boolean =>
  errorText(error).includes(`Command ${command} not found`);

const statusRouterPairIsConsistent = (status: AsioOutputStatusRecord): boolean => {
  switch (status.state) {
    case "normal":
      return status.routerState === "Normal";
    case "ready":
      return status.routerState === "Normal"
        || status.routerState === "Locked"
        || status.routerState === "AsioReady";
    case "locked":
      return status.routerState === "Normal" || status.routerState === "Locked";
    case "closed":
      return status.routerState === "Locked";
    case "active":
      return status.routerState === "AsioActive";
    case "fault":
      return status.routerState === "Fault";
  }
};

const statusView = (current: AudioOutputView, status: AsioOutputStatusRecord): AudioOutputView => {
  const normalReady = status.state === "normal"
    && status.routerState === "Normal"
    && (status.lifecycle === "Locked" || status.lifecycle === "Closed");
  const asioReady = status.state === "ready"
    && status.profileReady
    && (status.routerState === "Normal" || status.routerState === "AsioReady")
    && (status.lifecycle === "Locked" || status.lifecycle === "Closed");
  const state: AudioOutputState = !statusRouterPairIsConsistent(status)
    ? (status.state === "fault" || status.routerState === "Fault" ? "Fault" : "Locked")
    : status.state === "active" || status.routerState === "AsioActive"
    ? "Active"
    : status.state === "fault" || status.routerState === "Fault"
      ? "Fault"
      : status.routerState === "Quiescing" || status.routerState === "AsioStarting"
        || status.lifecycle === "Starting"
        ? "Locked"
      : normalReady || asioReady
        ? "Ready"
        : "Locked";
  const reason = !statusRouterPairIsConsistent(status)
    ? "ASIO status and router state disagree; output remains Locked."
    : status.terminalFault ?? status.lastError ?? `${status.lifecycle} · catalog ${status.catalogGeneration}`;
  return { ...current, state, reason };
};

const safeRouterStatesForRead = new Set<AsioOutputRouterState>([
  "Normal",
  "AsioReady",
  "Locked",
]);
const safeNativeStatesForRead = new Set<AsioOutputNativeState>([
  "normal",
  "ready",
  "locked",
  "closed",
]);
const safeLifecyclesForRead = new Set(["Locked", "Closed"]);

const isSafeForEnumeration = (status: AsioOutputStatusRecord): boolean =>
  statusRouterPairIsConsistent(status)
  && safeRouterStatesForRead.has(status.routerState)
  && safeNativeStatesForRead.has(status.state)
  && safeLifecyclesForRead.has(status.lifecycle);

const isActiveAsioOutputStatus = (status: AsioOutputStatusRecord | null): boolean =>
  status !== null
  && status.state === "active"
  && status.routerState === "AsioActive";

export const createAudioOutputController = (
  dependencies: AudioOutputControlDependencies,
): AudioOutputController => {
  const [view, setView] = createSignal<AudioOutputView>(normalView());
  const [options, setOptions] = createSignal<AudioOutputOptions>(emptyOptions());
  const [capabilities, setCapabilities] = createSignal<AsioOutputCapabilities | null>(null);
  const [busy, setBusy] = createSignal(false);
  const [testMode, setTestMode] = createSignal<AudioOutputTest | null>(null);
  const [soloMode, setSoloModeState] = createSignal<AudioOutputSoloMode>("none");
  let disposed = false;
  let requestGeneration = 0;
  let nativeStatus: AsioOutputStatusRecord | null = null;
  let cueAudioStatus: TimelineCueAudioStatusRecord | null = null;
  // The normal application includes the ASIO loader and commands. The
  // separately licensed bridge is still an explicit runtime payload; detect
  // that exact native boundary once so a missing bridge can return to the safe
  // Normal route without guessing a driver or silently falling back.
  const [asioCommandAvailable, setAsioCommandAvailable] = createSignal(
    dependencies.backendAvailable,
  );

  const clearPreflightState = (): void => {
    setTestMode(null);
    setSoloModeState("none");
  };

  const resetToNormalView = (): void => {
    // Returning to Normal is a hard local boundary.  Invalidate any ASIO
    // catalogue/status request that may still be in flight before clearing
    // the view; otherwise a late ASIO response can overwrite the Normal
    // screen (and leave it Locked again) after the operator already switched
    // back.
    requestGeneration += 1;
    setBusy(false);
    nativeStatus = null;
    cueAudioStatus = null;
    clearPreflightState();
    setView(normalView());
    setOptions(emptyOptions());
    setCapabilities(null);
  };

  const setLocked = (reason: string) => {
    setView((current) => ({ ...current, state: "Locked", reason }));
  };
  const setFault = (reason: string) => {
    clearPreflightState();
    setView((current) => ({ ...current, state: "Fault", reason }));
  };
  const setConfigurationValue = (update: (current: AudioOutputView) => AudioOutputView): boolean => {
    if (disposed) return false;
    let accepted = false;
    setView((current) => {
      if (current.state === "Active" || current.state === "Fault") return current;
      if (dependencies.backendAvailable && (!nativeStatus || !isSafeForEnumeration(nativeStatus))) {
        return current;
      }
      accepted = true;
      return update({ ...current, state: "Locked", reason: "Output configuration changed; Revalidate before Start." });
    });
    return accepted;
  };
  const commandFailure = (operation: string, error: unknown) => {
    setLocked(`ASIO ${operation} is unavailable or invalid: ${errorText(error)}`);
  };
  const cueEndpointIsReady = (): boolean => {
    if (view().cueRoute !== "split-device") return true;
    const current = view();
    const status = cueAudioStatus;
    if (!status || !current.cueDeviceName || !current.cueTopologyFingerprint) return false;
    if (status.observedTopologyFingerprint !== current.cueTopologyFingerprint) return false;
    const endpoint = options().cueEndpoints.find((candidate) => candidate.name === current.cueDeviceName);
    return endpoint?.selectable === true && endpoint.occurrences === 1;
  };
  const profileDraft = (): MachineAudioOutputProfileDraft => {
    const current = view();
    const output = capabilities()?.output;
    if (!output) throw new Error("fresh ASIO output capabilities are not available");
    return {
      driverId: current.driverId,
      catalogGeneration: current.catalogGeneration,
      sampleRateHz: current.sampleRate,
      nativeFormat: output.nativeFormat,
      fixedBufferFrames: current.bufferFrames,
      deviceOutputChannels: output.channels,
      programLeft: current.programLeft,
      programRight: current.programRight,
      cue: current.cue,
      cueRoute: current.cueRoute,
      cueDeviceName: current.cueDeviceName,
      cueTopologyFingerprint: current.cueTopologyFingerprint,
      spare: current.spare,
    };
  };

  /**
   * Persist a complete operator selection without asking the router to admit
   * it. This is the Normal -> ASIO setup path; explicit Revalidate below is
   * reserved for the native Locked -> Ready router admission.
   */
  const reselectConfiguredProfile = (): void => {
    if (!dependencies.backendAvailable || !nativeStatus || !isSafeForEnumeration(nativeStatus)) return;
    if (nativeStatus.routerState !== "Normal"
      && nativeStatus.routerState !== "Locked"
      && nativeStatus.routerState !== "AsioReady") return;
    let profileJson: string;
    try {
      profileJson = machineAudioOutputProfileJson(profileDraft());
    } catch {
      return;
    }
    if (!cueEndpointIsReady()) return;
    const request = ++requestGeneration;
    void (async () => {
      setBusy(true);
      try {
        const rawStatus = await dependencies.invoke<unknown>("reselect_asio_output_profile", {
          request: { profileJson },
        });
        if (disposed || request !== requestGeneration) return;
        const status = parseAsioOutputStatus(rawStatus);
        nativeStatus = status;
        setView((current) => ({
          ...statusView(current, status),
          catalogGeneration: status.catalogGeneration,
        }));
        if (status.state !== "ready" || !status.profileReady) {
          setLocked(status.terminalFault ?? status.lastError ?? "ASIO selection remains Locked after reselection.");
        }
      } catch (error) {
        if (!disposed && request === requestGeneration) {
          nativeStatus = null;
          commandFailure("reselection", error);
        }
      } finally {
        if (!disposed && request === requestGeneration) setBusy(false);
      }
    })();
  };

  const loadCapabilities = async (driverId: string, request: number): Promise<boolean> => {
    if (!nativeStatus || !isSafeForEnumeration(nativeStatus)) {
      if (!disposed && request === requestGeneration) {
        setLocked("ASIO capabilities are Locked until native lifecycle and router state are safe.");
      }
      return false;
    }
    setBusy(true);
    try {
      const raw = await dependencies.invoke<string>("get_asio_output_capabilities", {
        request: { driverId },
      });
      if (disposed || request !== requestGeneration) return false;
      const parsed = parseAsioOutputCapabilities(raw);
      if (parsed.driver.id !== driverId) throw new Error("capability driver identity does not match the selection");
      setCapabilities(parsed);
      const channelOptions: readonly AudioOutputChannelOption[] = Array.from(
        { length: parsed.output.channels },
        (_, index) => ({ index }),
      );
      const sampleRates: readonly AudioOutputRateOption[] = parsed.output.sampleRatesHz.map((value) => ({ value }));
      const bufferFrames: readonly AudioOutputBufferOption[] = [
        { value: parsed.output.fixedBufferFrames.preferred },
        { value: parsed.output.fixedBufferFrames.min },
        { value: parsed.output.fixedBufferFrames.max },
      ].filter((option, index, values) => values.findIndex((candidate) => candidate.value === option.value) === index);
      setOptions((current) => ({
        ...current,
        sampleRates,
        bufferFrames,
        channels: channelOptions,
        hasSpare: parsed.output.channels >= 4,
      }));
      setLocked("ASIO capabilities verified; select every output and Revalidate.");
      return true;
    } catch (error) {
      if (!disposed && request === requestGeneration) {
        setCapabilities(null);
        setOptions((current) => ({ ...current, sampleRates: [], bufferFrames: [], channels: [], hasSpare: false }));
        commandFailure("capabilities", error);
      }
      return false;
    } finally {
      if (!disposed && request === requestGeneration) setBusy(false);
    }
  };

  const loadStatus = async (
    request: number,
    requireCatalogGeneration = false,
  ): Promise<AsioOutputStatusRecord | null> => {
    if (disposed || request !== requestGeneration) return null;
    setBusy(true);
    try {
      const rawStatus = await dependencies.invoke<unknown>("get_asio_output_status");
      if (disposed || request !== requestGeneration) return null;
      const status = parseAsioOutputStatus(rawStatus);
      if (requireCatalogGeneration && status.catalogGeneration < 1) {
        throw new Error("status catalogGeneration is not current");
      }
      nativeStatus = status;
      setView((current) => ({
        ...statusView(current, status),
        catalogGeneration: status.catalogGeneration,
      }));
      if (!isActiveAsioOutputStatus(status)) clearPreflightState();
      if (status.state === "ready" && !status.profileReady) {
        setLocked("ASIO status is Ready without a validated profile; Revalidate is required.");
      }
      return status;
    } catch (error) {
      if (!disposed && request === requestGeneration) {
        nativeStatus = null;
        if (isMissingNativeCommand(error, "get_asio_output_status")) {
          setAsioCommandAvailable(false);
          setLocked("ASIO bridge is not available in this build; choose Normal WASAPI.");
        } else {
          commandFailure("status", error);
        }
      }
      return null;
    } finally {
      if (!disposed && request === requestGeneration) setBusy(false);
    }
  };

  const loadCueEndpoints = async (request: number): Promise<boolean> => {
    if (disposed || request !== requestGeneration || view().backend !== "show-asio") return false;
    try {
      const rawDevices = await dependencies.invoke<unknown>("list_audio_output_devices");
      if (disposed || request !== requestGeneration) return false;
      const deviceNames = parseAudioOutputDeviceNames(rawDevices);
      const rawStatus = await dependencies.invoke<unknown>("get_timeline_cue_audio_status");
      if (disposed || request !== requestGeneration) return false;
      const status = parseTimelineCueAudioStatus(rawStatus);
      cueAudioStatus = status;
      const cueEndpoints = mergeCueEndpointOptions(status.endpoints, deviceNames);
      setOptions((current) => ({ ...current, cueEndpoints }));
      if (view().cueRoute === "split-device" && !cueEndpointIsReady()) {
        setLocked(
          status.observedTopologyFingerprint === null
            ? "CUE WDM endpoints are unavailable; explicitly select an endpoint before Start."
            : "CUE WDM endpoint is missing, ambiguous, or topology-changed; Refresh and reselect before Start.",
        );
      }
      return true;
    } catch (error) {
      if (!disposed && request === requestGeneration) {
        cueAudioStatus = null;
        setOptions((current) => ({ ...current, cueEndpoints: [] }));
        if (view().cueRoute === "split-device") {
          setLocked(`CUE WDM endpoint inventory is unavailable or invalid: ${errorText(error)}`);
        }
      }
      return false;
    }
  };

  const refresh = async (): Promise<void> => {
    if (disposed) return;
    if (view().state === "Active" || view().state === "Fault") {
      // The UI disables Refresh in these lifecycle states, but keep the
      // controller fail-closed for direct callers as well: enumeration must
      // never race an owned/terminal ASIO session.
      setView((current) => ({
        ...current,
        reason: "Refresh is unavailable while ASIO output is Active or Fault; Stop/Close first.",
      }));
      return;
    }
    if (view().backend === "normal-wasapi") {
      clearPreflightState();
      cueAudioStatus = null;
      setView(normalView());
      setOptions(emptyOptions());
      setCapabilities(null);
      return;
    }
    if (!dependencies.backendAvailable || !asioCommandAvailable()) {
      setLocked("ASIO bridge is not available in this build; choose Normal WASAPI.");
      return;
    }
    const request = ++requestGeneration;
    clearPreflightState();
    setBusy(true);
    try {
      const initialStatus = await loadStatus(request);
      if (!initialStatus || disposed || request !== requestGeneration) return;
      if (!isSafeForEnumeration(initialStatus)) {
        setLocked("ASIO driver enumeration is Locked while native output is transitioning or faulted.");
        return;
      }
      const rawCatalog = await dependencies.invoke<string>("list_asio_output_drivers");
      if (disposed || request !== requestGeneration) return;
      const catalog = parseAsioOutputDriverCatalog(rawCatalog);
      const drivers: readonly AudioOutputDriverOption[] = catalog.drivers.map((driver) => ({
        id: driver.id,
        label: driver.name,
      }));
      setOptions((current) => ({ ...current, drivers }));
      const currentDriverId = view().driverId;
      if (!currentDriverId) {
        setCapabilities(null);
        setOptions((current) => ({
          ...current,
          sampleRates: [],
          bufferFrames: [],
          channels: [],
          hasSpare: false,
        }));
        setLocked("Select an explicit ASIO driver; no driver is selected automatically.");
        return;
      }
      if (!catalog.drivers.some((driver) => driver.id === currentDriverId)) {
        setCapabilities(null);
        setOptions((current) => ({
          ...current,
          sampleRates: [],
          bufferFrames: [],
          channels: [],
          hasSpare: false,
        }));
        setLocked(`Selected ASIO driver ${currentDriverId} is absent from the current catalog.`);
        return;
      }
      const beforeCapabilities = await loadStatus(request);
      if (!beforeCapabilities || disposed || request !== requestGeneration) return;
      if (!isSafeForEnumeration(beforeCapabilities)) {
        setLocked("ASIO capabilities are Locked while native output is transitioning or faulted.");
        return;
      }
      if (!(await loadCapabilities(currentDriverId, request)) || disposed || request !== requestGeneration) return;
      await loadStatus(request, true);
      await loadCueEndpoints(request);
    } catch (error) {
      if (!disposed && request === requestGeneration) commandFailure("driver/status", error);
    } finally {
      if (!disposed && request === requestGeneration) setBusy(false);
    }
  };

  const refreshCueEndpoints = async (): Promise<void> => {
    if (disposed || view().backend !== "show-asio") return;
    if (view().state === "Active" || view().state === "Fault") {
      setView((current) => ({
        ...current,
        reason: "CUE WDM endpoint refresh is unavailable while ASIO output is Active or Fault; Stop/Close first.",
      }));
      return;
    }
    if (!dependencies.backendAvailable) {
      if (view().cueRoute === "split-device") {
        setLocked("CUE WDM endpoint inventory is unavailable in this build; output remains Locked.");
      }
      return;
    }
    const request = ++requestGeneration;
    setBusy(true);
    try {
      await loadCueEndpoints(request);
    } finally {
      if (!disposed && request === requestGeneration) setBusy(false);
    }
  };

  const setBackend = (backend: AudioOutputBackend) => {
    if (backend === "normal-wasapi") {
      void returnToNormal();
      return;
    }
    if (disposed) return;
    clearPreflightState();
    cueAudioStatus = null;
    setView((current) => ({ ...current, backend: "show-asio", state: "Locked", reason: "ASIO requires an explicit Refresh and revalidation." }));
    setOptions(emptyOptions());
    setCapabilities(null);
    nativeStatus = null;
    if (asioCommandAvailable()) void refresh();
    else setLocked("ASIO bridge is not available in this build; choose Normal WASAPI.");
  };
  const setDriver = (driverId: string) => {
    if (disposed || view().backend !== "show-asio") return;
    const accepted = setConfigurationValue((current) => ({
      ...current,
      driverId,
      sampleRate: null,
      bufferFrames: null,
      programLeft: null,
      programRight: null,
      cue: null,
      cueDeviceName: null,
      cueTopologyFingerprint: null,
      spare: null,
    }));
    if (!accepted || !dependencies.backendAvailable) return;
    cueAudioStatus = null;
    setOptions((current) => ({ ...current, cueEndpoints: [] }));
    if (!nativeStatus || !isSafeForEnumeration(nativeStatus)) {
      setLocked("Driver capabilities are Locked until native lifecycle and router state are safe.");
      return;
    }
    {
      const request = ++requestGeneration;
      void (async () => {
        const status = await loadStatus(request);
        if (!status || !isSafeForEnumeration(status) || disposed || request !== requestGeneration) return;
        if (!(await loadCapabilities(driverId, request)) || disposed || request !== requestGeneration) return;
        await loadStatus(request);
      })();
    }
  };
  const setSampleRate = (sampleRate: number | null) => {
    if (setConfigurationValue((current) => ({ ...current, sampleRate }))) reselectConfiguredProfile();
  };
  const setBufferFrames = (bufferFrames: number | null) => {
    if (setConfigurationValue((current) => ({ ...current, bufferFrames }))) reselectConfiguredProfile();
  };
  const setProgramLeft = (programLeft: number | null) => {
    if (setConfigurationValue((current) => ({ ...current, programLeft }))) reselectConfiguredProfile();
  };
  const setProgramRight = (programRight: number | null) => {
    if (setConfigurationValue((current) => ({ ...current, programRight }))) reselectConfiguredProfile();
  };
  const setCueRoute = (cueRoute: AudioOutputCueRoute) => {
    if (disposed || view().backend !== "show-asio") return;
    if (view().state === "Active" || view().state === "Fault") return;
    if (cueRoute !== "same-asio" && cueRoute !== "split-device") {
      setLocked("CUE route is invalid; choose Same ASIO or Split device.");
      return;
    }
    const accepted = setConfigurationValue((current) => ({
      ...current,
      cueRoute,
      cue: cueRoute === "same-asio" ? current.cue : null,
      cueDeviceName: cueRoute === "split-device" ? current.cueDeviceName : null,
      cueTopologyFingerprint: cueRoute === "split-device" ? current.cueTopologyFingerprint : null,
    }));
    if (!accepted) return;
    if (cueRoute === "split-device" && !cueEndpointIsReady()) {
      setLocked("CUE WDM endpoint is not ready; explicitly select one before Start.");
      if (cueAudioStatus === null || options().cueEndpoints.length === 0) {
        void refreshCueEndpoints();
      }
    }
    reselectConfiguredProfile();
  };
  const setCueDeviceName = (deviceName: string | null) => {
    if (disposed || view().backend !== "show-asio" || view().cueRoute !== "split-device") return;
    if (view().state === "Active" || view().state === "Fault") return;
    if (deviceName !== null) {
      const endpoint = options().cueEndpoints.find((candidate) => candidate.name === deviceName);
      if (!endpoint || !endpoint.selectable || endpoint.occurrences !== 1) {
        setLocked("CUE WDM endpoint is missing or ambiguous; select an exact endpoint before Start.");
        return;
      }
      const observedTopologyFingerprint = cueAudioStatus?.observedTopologyFingerprint;
      if (!observedTopologyFingerprint) {
        setLocked("CUE WDM topology is unavailable; Refresh endpoints before selecting a device.");
        return;
      }
      const accepted = setConfigurationValue((current) => ({
        ...current,
        cueDeviceName: deviceName,
        cueTopologyFingerprint: observedTopologyFingerprint,
        cue: null,
      }));
      if (accepted) reselectConfiguredProfile();
      return;
    }
    if (setConfigurationValue((current) => ({
      ...current,
      cueDeviceName: null,
      cueTopologyFingerprint: null,
      cue: null,
    }))) reselectConfiguredProfile();
  };
  const setCueChannel = (cue: number | null) => {
    if (view().state === "Active" || view().state === "Fault") return;
    if (view().cueRoute !== "same-asio") {
      setLocked("CUE ASIO channel is unavailable while Split device is selected.");
      return;
    }
    if (setConfigurationValue((current) => ({ ...current, cue }))) reselectConfiguredProfile();
  };
  const setCue = Object.assign(setCueChannel as AudioOutputCueChange, {
    setRoute: setCueRoute,
    setDeviceName: setCueDeviceName,
  });
  const setSpare = (spare: number | null) => {
    if (setConfigurationValue((current) => ({ ...current, spare }))) reselectConfiguredProfile();
  };

  const revalidate = async (): Promise<void> => {
    if (disposed || view().backend !== "show-asio") return;
    if (view().state === "Active" || view().state === "Fault") {
      setView((current) => ({
        ...current,
        reason: "Revalidate is unavailable while ASIO output is Active or Fault; Stop/Close first.",
      }));
      return;
    }
    const currentStatus = nativeStatus;
    if (!currentStatus || !isSafeForEnumeration(currentStatus) || currentStatus.routerState !== "Locked") {
      setLocked("ASIO Revalidate is available only after Stop/Close leaves the router Locked.");
      return;
    }
    if (!dependencies.backendAvailable) {
      setLocked("ASIO Revalidate command is unavailable in this build; output remains Locked.");
      return;
    }
    const request = ++requestGeneration;
    clearPreflightState();
    setBusy(true);
    try {
      const rawStatus = await dependencies.invoke<unknown>("revalidate_asio_program_cue_output");
      if (disposed || request !== requestGeneration) return;
      const selectedStatus = parseAsioOutputStatus(rawStatus);
      nativeStatus = selectedStatus;
      setView((current) => ({
        ...statusView(current, selectedStatus),
        catalogGeneration: selectedStatus.catalogGeneration,
      }));
      if (selectedStatus.state !== "ready"
        || !selectedStatus.profileReady
        || selectedStatus.routerState !== "AsioReady") {
        setLocked(
          selectedStatus.terminalFault
            ?? selectedStatus.lastError
            ?? "ASIO profile remains Locked after explicit native revalidation.",
        );
        return;
      }
    } catch (error) {
      if (!disposed && request === requestGeneration) commandFailure("Revalidate", error);
    } finally {
      if (!disposed && request === requestGeneration) setBusy(false);
    }
  };

  const start = async (): Promise<void> => {
    if (disposed || view().backend !== "show-asio") return;
    if (view().state !== "Ready") {
      setView((current) => ({
        ...current,
        reason: "ASIO Start is Locked until a Ready profile is explicitly validated.",
      }));
      return;
    }
    if (!nativeStatus
      || !isSafeForEnumeration(nativeStatus)
      || (nativeStatus.routerState !== "Normal" && nativeStatus.routerState !== "AsioReady")
      || nativeStatus.state !== "ready"
      || !nativeStatus.profileReady) {
      setLocked("ASIO Start is Locked until native status and router state publish a validated Ready profile.");
      return;
    }
    try {
      machineAudioOutputProfileJson(profileDraft());
      if (!cueEndpointIsReady()) {
        throw new Error("CUE WDM endpoint is missing, ambiguous, or topology-changed");
      }
    } catch (error) {
      setLocked(`ASIO Start is Locked: ${errorText(error)}`);
      return;
    }
    if (!dependencies.backendAvailable) {
      setLocked("ASIO Start command is unavailable in this build; output remains Locked.");
      return;
    }
    const request = ++requestGeneration;
    clearPreflightState();
    setBusy(true);
    try {
      const rawStatus = await dependencies.invoke<unknown>("start_asio_program_cue_output");
      if (disposed || request !== requestGeneration) return;
      const status = parseAsioOutputStatus(rawStatus);
      nativeStatus = status;
      setView((current) => ({
        ...statusView(current, status),
        catalogGeneration: status.catalogGeneration,
      }));
      if (status.state !== "active" || status.routerState !== "AsioActive") {
        setLocked(status.terminalFault ?? status.lastError ?? "ASIO Start did not publish Active.");
      }
    } catch (error) {
      if (!disposed && request === requestGeneration) {
        const startError = errorText(error);
        // Start can fail after native ownership has changed (for example while
        // rolling back a CUE admission). Re-read the authoritative state so
        // Stop remains available for an Active/Fault session and never claim
        // that output is stopped from a stale pre-Start snapshot.
        const recoveredStatus = await loadStatus(request);
        if (disposed || request !== requestGeneration || recoveredStatus === null) return;
        if (recoveredStatus.routerState === "AsioActive"
          || recoveredStatus.routerState === "Fault"
          || recoveredStatus.state === "active"
          || recoveredStatus.state === "fault") {
          setFault(`ASIO Start failed after native ownership changed; Stop/Close is required: ${startError}`);
        } else {
          setLocked(`ASIO Start failed and native output is not Active: ${startError}`);
        }
      }
    } finally {
      if (!disposed && request === requestGeneration) setBusy(false);
    }
  };

  const stop = async (): Promise<void> => {
    if (disposed || view().backend !== "show-asio") return;
    if (view().state !== "Active" && view().state !== "Fault") {
      setView((current) => ({
        ...current,
        reason: "ASIO Stop/Close is unavailable until output is Active or Fault.",
      }));
      return;
    }
    if (!nativeStatus || (nativeStatus.routerState !== "AsioActive" && nativeStatus.routerState !== "Fault")) {
      setFault("ASIO Stop/Close is Locked until native status confirms an owned or faulted session.");
      return;
    }
    if (!dependencies.backendAvailable) {
      setFault("ASIO Stop/Close command is unavailable; output ownership remains Locked.");
      return;
    }
    const request = ++requestGeneration;
    clearPreflightState();
    setBusy(true);
    try {
      const rawStatus = await dependencies.invoke<unknown>("stop_close_asio_program_cue_output");
      if (disposed || request !== requestGeneration) return;
      const status = parseAsioOutputStatus(rawStatus);
      nativeStatus = status;
      setView((current) => ({
        ...statusView(current, status),
        catalogGeneration: status.catalogGeneration,
      }));
    } catch (error) {
      if (!disposed && request === requestGeneration) {
        const stopError = errorText(error);
        // Stop/Close can cross the native router fence and still surface an
        // error (for example a cleanup warning or transport-side exception).
        // Re-read the authoritative native state before deciding whether the
        // UI must remain Faulted.  A Locked router is safe to revalidate or
        // return to Normal; AsioActive/Fault means ownership or fault is still
        // present and must remain visible as Fault.
        const recoveredStatus = await loadStatus(request);
        if (disposed || request !== requestGeneration) return;
        if (recoveredStatus?.routerState === "Locked") {
          setLocked(
            `ASIO Stop/Close reported an error after native output was fenced; router is Locked and can be returned to Normal: ${stopError}`,
          );
        } else if (recoveredStatus?.routerState === "AsioActive"
          || recoveredStatus?.routerState === "Fault"
          || recoveredStatus?.state === "active"
          || recoveredStatus?.state === "fault") {
          setFault(
            `ASIO Stop/Close failed; native output remains ${recoveredStatus.routerState}: ${stopError}`,
          );
        } else {
          setFault(`ASIO Stop/Close failed; output state could not be proven safe: ${stopError}`);
        }
      }
    } finally {
      if (!disposed && request === requestGeneration) setBusy(false);
    }
  };

  const returnToNormal = async (): Promise<void> => {
    if (disposed || view().backend !== "show-asio") return;
    if (view().state === "Active" || view().state === "Fault") {
      setView((current) => ({
        ...current,
        reason: "Return to normal is unavailable while ASIO output is Active or Fault; Stop/Close first.",
      }));
      return;
    }
    // Selecting ASIO only changes the local setup view; native ownership is
    // not acquired until Revalidate/Start. If the first status probe cannot
    // run (for example because the optional bridge is absent), keep the
    // explicit Return-to-Normal action usable without sending a native command
    // that the Normal router would correctly reject while it is already idle.
    if (nativeStatus === null) {
      resetToNormalView();
      return;
    }
    if (dependencies.backendAvailable && asioCommandAvailable()
      && (!nativeStatus
        || !isSafeForEnumeration(nativeStatus)
        || (nativeStatus.routerState !== "Normal"
          && nativeStatus.routerState !== "Locked"
          && nativeStatus.routerState !== "AsioReady"))) {
      setLocked("Return to normal requires the native router to be Normal, Locked, or AsioReady.");
      return;
    }
    // The ASIO selector can be opened without taking native ownership.  In
    // that setup-only state the native router remains Normal, so switching
    // back is a local view change and must not issue a redundant Normal
    // selection command that could race another owner.
    if (nativeStatus?.routerState === "Normal") {
      resetToNormalView();
      return;
    }
    if (!dependencies.backendAvailable || !asioCommandAvailable()) {
      resetToNormalView();
      return;
    }
    const request = ++requestGeneration;
    clearPreflightState();
    setBusy(true);
    try {
      const rawStatus = await dependencies.invoke<unknown>("select_normal_audio_output");
      if (disposed || request !== requestGeneration) return;
      const status = parseAsioOutputStatus(rawStatus);
      nativeStatus = status;
      if (status.state !== "normal" || status.routerState !== "Normal") {
        throw new Error("native Normal selection did not publish the exact Normal status");
      }
      if (!disposed) {
        cueAudioStatus = null;
        setView(normalView());
        setOptions(emptyOptions());
        setCapabilities(null);
      }
    } catch (error) {
      if (!disposed && request === requestGeneration) {
        if (nativeStatus === null && isMissingNativeCommand(error, "select_normal_audio_output")) {
          setAsioCommandAvailable(false);
          resetToNormalView();
        } else {
          commandFailure("Return to normal", error);
        }
      }
    } finally {
      if (!disposed && request === requestGeneration) setBusy(false);
    }
  };

  const preflightActionIsAllowed = (operation: string): boolean => {
    if (disposed) return false;
    if (!dependencies.backendAvailable || !asioCommandAvailable()) {
      setLocked("ASIO bridge is not available in this build; choose Normal WASAPI.");
      return false;
    }
    if (busy()) {
      setLocked(`ASIO ${operation} is unavailable while another output operation is in progress.`);
      return false;
    }
    if (view().backend !== "show-asio"
      || view().state !== "Active"
      || !isActiveAsioOutputStatus(nativeStatus)) {
      setLocked(`ASIO ${operation} is available only while output is Active and the router is AsioActive.`);
      return false;
    }
    if (operation === "solo" && view().cueRoute === "split-device") {
      // Split-device has no native solo transport for either clock domain.
      // Keep the owned Active state intact so a direct caller cannot make Stop
      // unreachable by attempting an unsupported solo action.
      setView((current) => ({
        ...current,
        reason: "ASIO solo is unavailable while Split device is selected; no external-WDM solo fallback exists.",
      }));
      return false;
    }
    return true;
  };

  const parseActivePreflightStatus = (raw: unknown, operation: string): AsioOutputStatusRecord => {
    const status = parseAsioOutputStatus(raw);
    if (!isActiveAsioOutputStatus(status)) {
      throw new Error(
        status.terminalFault
          ?? status.lastError
          ?? `ASIO ${operation} did not publish the exact Active/AsioActive status`,
      );
    }
    return status;
  };

  const setTest = async (test: AudioOutputTest | null): Promise<void> => {
    if (!preflightActionIsAllowed("test")) return;
    const splitDeviceCueCommand = view().cueRoute === "split-device"
      && (test === "cue" || test === null);
    const splitDeviceCueEnabled = test !== null;
    const request = ++requestGeneration;
    setBusy(true);
    try {
      const rawStatus = splitDeviceCueCommand
        ? await dependencies.invoke<unknown>("set_explicit_wdm_cue_test", {
          enabled: splitDeviceCueEnabled,
        })
        : await dependencies.invoke<unknown>("set_asio_output_test", {
          request: { test: test ?? "off" },
        });
      if (disposed || request !== requestGeneration) return;
      const status = parseActivePreflightStatus(rawStatus, "test");
      nativeStatus = status;
      setView((current) => ({
        ...statusView(current, status),
        catalogGeneration: status.catalogGeneration,
      }));
      if (test !== null) setSoloModeState("none");
      // The external WDM tone is bounded, but its native Sink remains owned
      // until an explicit stop, replacement, Timeline Play, or route retire.
      // Keep CUE selected so the panel's Stop test action remains reachable.
      setTestMode(test);
    } catch (error) {
      if (!disposed && request === requestGeneration) commandFailure("test", error);
    } finally {
      if (!disposed && request === requestGeneration) setBusy(false);
    }
  };

  const setSoloMode = async (mode: AudioOutputSoloMode): Promise<void> => {
    if (!preflightActionIsAllowed("solo")) return;
    const request = ++requestGeneration;
    setBusy(true);
    try {
      const rawStatus = await dependencies.invoke<unknown>("set_asio_output_solo", {
        request: { mode },
      });
      if (disposed || request !== requestGeneration) return;
      const status = parseActivePreflightStatus(rawStatus, "solo");
      nativeStatus = status;
      setView((current) => ({
        ...statusView(current, status),
        catalogGeneration: status.catalogGeneration,
      }));
      if (mode !== "none") setTestMode(null);
      setSoloModeState(mode);
    } catch (error) {
      if (!disposed && request === requestGeneration) commandFailure("solo", error);
    } finally {
      if (!disposed && request === requestGeneration) setBusy(false);
    }
  };

  const summary = createMemo(() => `${view().state} · ${view().reason}`);
  const stateTone = createMemo<"ok" | "idle" | "warning" | "error">(() => {
    switch (view().state) {
      case "Ready":
        return "ok";
      case "Active":
        return "warning";
      case "Fault":
        return "error";
      case "Locked":
        return "warning";
    }
  });
  const canRefresh = createMemo(() => view().backend === "show-asio" && dependencies.backendAvailable && asioCommandAvailable() && !busy() && view().state !== "Active" && view().state !== "Fault");
  const canRevalidate = createMemo(() =>
    view().backend === "show-asio"
    && dependencies.backendAvailable
    && asioCommandAvailable()
    && !busy()
    && capabilities() !== null
    && view().state !== "Active"
    && view().state !== "Fault"
    && nativeStatus !== null
    && isSafeForEnumeration(nativeStatus)
    && nativeStatus.routerState === "Locked");
  const canStart = (): boolean => {
    if (view().backend !== "show-asio" || busy() || view().state !== "Ready") return false;
    if (!nativeStatus
      || !isSafeForEnumeration(nativeStatus)
      || (nativeStatus.routerState !== "Normal" && nativeStatus.routerState !== "AsioReady")
      || nativeStatus.state !== "ready"
      || !nativeStatus.profileReady) return false;
    try {
      machineAudioOutputProfileJson(profileDraft());
      if (!cueEndpointIsReady()) return false;
      return true;
    } catch {
      return false;
    }
  };
  const canStop = createMemo(() =>
    view().backend === "show-asio"
    && dependencies.backendAvailable
    && asioCommandAvailable()
    && !busy()
    && (view().state === "Active" || view().state === "Fault")
    && nativeStatus !== null
    && (nativeStatus.routerState === "AsioActive" || nativeStatus.routerState === "Fault"));
  const canReturnToNormal = createMemo(() =>
    view().backend === "show-asio"
    && !busy()
    && view().state !== "Active"
    && view().state !== "Fault"
    && (nativeStatus === null
      || !dependencies.backendAvailable
      || !asioCommandAvailable()
      || (nativeStatus !== null
        && isSafeForEnumeration(nativeStatus)
        && (nativeStatus.routerState === "Normal"
          || nativeStatus.routerState === "Locked"
          || nativeStatus.routerState === "AsioReady"))));
  const canTest = (): boolean =>
    dependencies.backendAvailable
    && asioCommandAvailable()
    && !busy()
    && view().backend === "show-asio"
    && view().state === "Active"
    && isActiveAsioOutputStatus(nativeStatus);
  const canSolo = (): boolean =>
    dependencies.backendAvailable
    && asioCommandAvailable()
    && !busy()
    && view().backend === "show-asio"
    && view().state === "Active"
    && isActiveAsioOutputStatus(nativeStatus)
    && view().cueRoute === "same-asio";
  const livePlaybackActive = createMemo(() => dependencies.readLivePlaybackActive());

  return {
    view,
    options,
    capabilities,
    busy,
    testMode,
    soloMode,
    summary,
    stateTone,
    canRefresh,
    canRevalidate,
    canStart,
    canStop,
    canReturnToNormal,
    canTest,
    canSolo,
    livePlaybackActive,
    setBackend,
    setDriver,
    setSampleRate,
    setBufferFrames,
    setProgramLeft,
    setProgramRight,
    setCue,
    setCueRoute,
    setCueDeviceName,
    setSpare,
    refresh,
    refreshCueEndpoints,
    revalidate,
    start,
    stop,
    returnToNormal,
    setTest,
    setSoloMode,
    dispose: () => {
      disposed = true;
      requestGeneration += 1;
      nativeStatus = null;
      cueAudioStatus = null;
      clearPreflightState();
      setBusy(false);
    },
  };
};
