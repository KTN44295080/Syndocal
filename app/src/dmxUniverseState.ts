import type {
  DmxOutputConfig,
  DmxOutputRouteTelemetry,
  SerialDmxMachineBindingStatus,
  ShowSerialDmxSafetyBlackoutRouteStatus,
} from "./types";

export type DmxUniverseTone = "ready" | "idle" | "error";

/** The State cell of one row in the DMX universes table. */
export interface DmxUniverseState {
  label: string;
  tone: DmxUniverseTone;
}

export interface ShowDmxUniverseStateInput {
  /** `dmx_outputs[0]`, the Universe 0 Art-Net loopback route. */
  route: DmxOutputConfig | undefined;
  binding: SerialDmxMachineBindingStatus | null;
  worker: ShowSerialDmxSafetyBlackoutRouteStatus | null;
  safetyBlackoutEngaged: boolean;
}

/** Universe 0 (Show DMX): the Art-Net loopback and its optional USB-DMX cable. */
export function showDmxUniverseState(input: ShowDmxUniverseStateInput): DmxUniverseState {
  const worker = input.worker;
  if (!input.route?.enabled) return { label: "Not prepared", tone: "idle" };
  if (worker?.faulted) return { label: "Fault · S0 latched", tone: "error" };
  if (worker?.active) {
    return {
      label: input.safetyBlackoutEngaged ? "Armed · S0" : worker.liveFrameQueued ? "Sending" : "Started",
      tone: "ready",
    };
  }
  return {
    label: input.binding?.state === "selected_and_present" ? "Art-Net only · USB stopped" : "Art-Net only",
    tone: "idle",
  };
}

/** An applied Art-Net / sACN route and its last send result. */
export function networkDmxRouteState(
  route: DmxOutputConfig,
  telemetry: DmxOutputRouteTelemetry | undefined,
): DmxUniverseState {
  if (!route.enabled) return { label: "Disabled", tone: "idle" };
  if (telemetry?.reconnecting || (telemetry && telemetry.attempted && !telemetry.success)) {
    return { label: telemetry.error ? `Send failed · ${telemetry.error}` : "Send failed", tone: "error" };
  }
  if (telemetry?.success) return { label: "Sending", tone: "ready" };
  return { label: "Enabled", tone: "idle" };
}

/** Whether a draft network row carries exactly the applied route's settings. */
export function sameNetworkDmxRoute(left: DmxOutputConfig | undefined, right: DmxOutputConfig | undefined): boolean {
  return Boolean(left && right)
    && left!.enabled === right!.enabled
    && left!.protocol === right!.protocol
    && left!.target_ip === right!.target_ip
    && left!.port === right!.port
    && left!.universe === right!.universe;
}
