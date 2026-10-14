import type {
  DmxOutputConfig,
  DmxOutputRouteTelemetry,
  SerialDmxMachineBindingStatus,
  ShowSerialDmxSafetyBlackoutRouteStatus,
  UsbDmxUniversesStatus,
} from "./types";

/** One output universe in the DMX overview, whatever carries it. */
export interface DmxUniverseOverviewRow {
  key: string;
  universe: number;
  transport: "USB-DMX" | "Art-Net" | "sACN" | "USB-DMX + Art-Net";
  target: string;
  state: string;
  tone: "ready" | "idle" | "error";
}

export interface DmxUniverseOverviewInput {
  /** `dmx_outputs` from the snapshot; index 0 is the Universe 0 show route. */
  routes: DmxOutputConfig[];
  routeTelemetry: DmxOutputRouteTelemetry[];
  showDmxBinding: SerialDmxMachineBindingStatus | null;
  showDmxWorker: ShowSerialDmxSafetyBlackoutRouteStatus | null;
  usbCables: UsbDmxUniversesStatus | null;
  safetyBlackoutEngaged: boolean;
}

const identityLabel = (identity: { port_name: string; manufacturer: string; product: string } | null | undefined) =>
  identity ? `${identity.port_name} · ${identity.manufacturer} / ${identity.product}` : "—";

function showDmxRow(input: DmxUniverseOverviewInput): DmxUniverseOverviewRow {
  const route = input.routes[0];
  const worker = input.showDmxWorker;
  const usb = identityLabel(input.showDmxBinding?.selected);
  let state: string;
  let tone: DmxUniverseOverviewRow["tone"];
  if (!route?.enabled) {
    state = "Not prepared";
    tone = "idle";
  } else if (worker?.faulted) {
    state = "Fault · S0 latched";
    tone = "error";
  } else if (worker?.active) {
    state = input.safetyBlackoutEngaged ? "Armed · S0" : worker.liveFrameQueued ? "Sending" : "Started";
    tone = "ready";
  } else {
    state = input.showDmxBinding?.state === "selected_and_present" ? "Art-Net only · USB stopped" : "Art-Net only";
    tone = "idle";
  }
  return {
    key: "show-dmx",
    universe: 0,
    transport: "USB-DMX + Art-Net",
    target: `${usb} · 127.0.0.1:6454`,
    state,
    tone,
  };
}

function usbCableRows(input: DmxUniverseOverviewInput): DmxUniverseOverviewRow[] {
  const status = input.usbCables;
  if (!status) return [];
  const universes = new Set([
    ...status.bindings.map((binding) => binding.universe),
    ...status.cables.flatMap((cable) => cable.universe === null ? [] : [cable.universe]),
  ]);
  return [...universes].sort((left, right) => left - right).map((universe) => {
    const binding = status.bindings.find((candidate) => candidate.universe === universe);
    const cable = status.cables.find((candidate) => candidate.universe === universe);
    let state: string;
    let tone: DmxUniverseOverviewRow["tone"];
    if (cable?.faulted) {
      state = "Fault · S0 latched";
      tone = "error";
    } else if (cable?.active) {
      state = input.safetyBlackoutEngaged ? "Armed · S0" : cable.liveFrameQueued ? "Sending" : "Started";
      tone = "ready";
    } else if (cable && !cable.workerShutdownCompleted) {
      state = "Shutdown pending";
      tone = "error";
    } else if (binding?.state !== "selected_and_present") {
      state = "Cable missing";
      tone = "error";
    } else {
      state = "Stopped";
      tone = "idle";
    }
    return {
      key: `usb-${universe}`,
      universe,
      transport: "USB-DMX" as const,
      target: identityLabel(binding?.identity),
      state,
      tone,
      };
  });
}

function networkRows(input: DmxUniverseOverviewInput): DmxUniverseOverviewRow[] {
  return input.routes.slice(1).flatMap((route, offset) => {
    if (route.protocol !== "ArtNet" && route.protocol !== "Sacn") return [];
    const index = offset + 1;
    const telemetry = input.routeTelemetry.find((result) => result.index === index);
    let state: string;
    let tone: DmxUniverseOverviewRow["tone"];
    if (!route.enabled) {
      state = "Disabled";
      tone = "idle";
    } else if (telemetry?.reconnecting || (telemetry && telemetry.attempted && !telemetry.success)) {
      state = telemetry.error ? `Send failed · ${telemetry.error}` : "Send failed";
      tone = "error";
    } else if (telemetry?.success) {
      state = "Sending";
      tone = "ready";
    } else {
      state = "Enabled";
      tone = "idle";
    }
    return [{
      key: `network-${index}`,
      universe: route.universe,
      transport: route.protocol === "ArtNet" ? "Art-Net" as const : "sACN" as const,
      target: route.protocol === "Sacn" && route.target_ip === "multicast"
        ? `multicast:${route.port}`
        : `${route.target_ip}:${route.port}`,
      state,
      tone,
      }];
  });
}

/**
 * Every output universe in one list, ordered by universe. The same universe
 * on two transports (for example a USB cable and an Art-Net node) is a
 * mirror of the same data, not a conflict.
 */
export function dmxUniverseOverviewRows(input: DmxUniverseOverviewInput): DmxUniverseOverviewRow[] {
  return [showDmxRow(input), ...usbCableRows(input), ...networkRows(input)]
    .sort((left, right) => left.universe - right.universe || left.key.localeCompare(right.key));
}
