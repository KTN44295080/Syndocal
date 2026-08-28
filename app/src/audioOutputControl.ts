import { createMemo, createSignal, type Accessor } from "solid-js";
import type {
  AudioOutputBackend,
  AudioOutputBufferOption,
  AudioOutputChannelOption,
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
const MACHINE_PROFILE_SCHEMA_VERSION = 1;

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
  spare: number | null;
}

export interface MachineAudioOutputProfile {
  schemaVersion: 1;
  driverId: string;
  catalogGeneration: number;
  sampleRateHz: number;
  nativeFormat: string;
  fixedBufferFrames: number;
  deviceOutputChannels: number;
  programLeftOutput: number;
  programRightOutput: number;
  cueOutput: number;
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
  setCue: (channelIndex: number | null) => void;
  setSpare: (channelIndex: number | null) => void;
  refresh: () => Promise<void>;
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

const requireBoolean = (value: unknown, field: string): boolean => {
  if (typeof value !== "boolean") throw new Error(`${field} must be boolean`);
  return value;
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
  const cueOutput = channelToOneBased(draft.cue, "CUE", deviceOutputChannels);
  const selected = new Set([programLeftOutput, programRightOutput, cueOutput]);
  if (selected.size !== 3) throw new Error("PROGRAM L, PROGRAM R, and CUE must use distinct outputs");
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
    cueOutput,
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
  hasSpare: false,
});

const errorText = (error: unknown): string =>
  error instanceof Error ? error.message : String(error);

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

  const clearPreflightState = (): void => {
    setTestMode(null);
    setSoloModeState("none");
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
    setLocked(`Show ASIO ${operation} is unavailable or invalid: ${errorText(error)}`);
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
        commandFailure("status", error);
      }
      return null;
    } finally {
      if (!disposed && request === requestGeneration) setBusy(false);
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
      setView(normalView());
      setOptions(emptyOptions());
      setCapabilities(null);
      return;
    }
    if (!dependencies.backendAvailable) {
      setLocked("Show ASIO command is unavailable in this build; output remains Locked.");
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
        setLocked("Select an explicit Show ASIO driver; no driver is selected automatically.");
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
        setLocked(`Selected Show ASIO driver ${currentDriverId} is absent from the current catalog.`);
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
    } catch (error) {
      if (!disposed && request === requestGeneration) commandFailure("driver/status", error);
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
    setView((current) => ({ ...current, backend: "show-asio", state: "Locked", reason: "Show ASIO requires an explicit Refresh and revalidation." }));
    setCapabilities(null);
    nativeStatus = null;
    void refresh();
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
      spare: null,
    }));
    if (!accepted || !dependencies.backendAvailable) return;
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
  const setCue = (cue: number | null) => {
    if (setConfigurationValue((current) => ({ ...current, cue }))) reselectConfiguredProfile();
  };
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
      setLocked("Show ASIO Revalidate command is unavailable in this build; output remains Locked.");
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
    } catch (error) {
      setLocked(`ASIO Start is Locked: ${errorText(error)}`);
      return;
    }
    if (!dependencies.backendAvailable) {
      setLocked("Show ASIO Start command is unavailable in this build; output remains Locked.");
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
        setFault(`ASIO Start failed and output remains stopped: ${errorText(error)}`);
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
      setFault("Show ASIO Stop/Close command is unavailable; output ownership remains Locked.");
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
        setFault(`ASIO Stop/Close failed; output remains Locked: ${errorText(error)}`);
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
    if (dependencies.backendAvailable
      && (!nativeStatus
        || !isSafeForEnumeration(nativeStatus)
        || (nativeStatus.routerState !== "Locked" && nativeStatus.routerState !== "AsioReady"))) {
      setLocked("Return to normal requires the native router to be Locked or AsioReady.");
      return;
    }
    if (!dependencies.backendAvailable) {
      nativeStatus = null;
      clearPreflightState();
      setView(normalView());
      setOptions(emptyOptions());
      setCapabilities(null);
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
        setView(normalView());
        setOptions(emptyOptions());
        setCapabilities(null);
      }
    } catch (error) {
      if (!disposed && request === requestGeneration) commandFailure("Return to normal", error);
    } finally {
      if (!disposed && request === requestGeneration) setBusy(false);
    }
  };

  const preflightActionIsAllowed = (operation: string): boolean => {
    if (disposed) return false;
    if (!dependencies.backendAvailable) {
      setLocked(`Show ASIO ${operation} command is unavailable in this build; output remains Locked.`);
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
    const request = ++requestGeneration;
    setBusy(true);
    try {
      const rawStatus = await dependencies.invoke<unknown>("set_asio_output_test", {
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
  const canRefresh = createMemo(() => view().backend === "show-asio" && dependencies.backendAvailable && !busy() && view().state !== "Active" && view().state !== "Fault");
  const canRevalidate = createMemo(() =>
    view().backend === "show-asio"
    && dependencies.backendAvailable
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
      return true;
    } catch {
      return false;
    }
  };
  const canStop = createMemo(() =>
    view().backend === "show-asio"
    && dependencies.backendAvailable
    && !busy()
    && (view().state === "Active" || view().state === "Fault")
    && nativeStatus !== null
    && (nativeStatus.routerState === "AsioActive" || nativeStatus.routerState === "Fault"));
  const canReturnToNormal = createMemo(() =>
    view().backend === "show-asio"
    && !busy()
    && view().state !== "Active"
    && view().state !== "Fault"
    && (!dependencies.backendAvailable
      || (nativeStatus !== null
        && isSafeForEnumeration(nativeStatus)
        && (nativeStatus.routerState === "Locked" || nativeStatus.routerState === "AsioReady"))));
  const canTest = (): boolean =>
    dependencies.backendAvailable
    && !busy()
    && view().backend === "show-asio"
    && view().state === "Active"
    && isActiveAsioOutputStatus(nativeStatus);
  const canSolo = (): boolean =>
    dependencies.backendAvailable
    && !busy()
    && view().backend === "show-asio"
    && view().state === "Active"
    && isActiveAsioOutputStatus(nativeStatus);
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
    setSpare,
    refresh,
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
      clearPreflightState();
      setBusy(false);
    },
  };
};
