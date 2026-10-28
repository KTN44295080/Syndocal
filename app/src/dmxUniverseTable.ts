import type { DmxOutputConfig, SerialPortSummary, UsbDmxUniversesStatus } from "./types";
import { networkDmxRoutePayload } from "./networkDmxRoutes";
import { hasExactMachineLocalOpenDmxIdentity, sameUsbDmxCable } from "./usbDmxIdentity";
export interface DmxUniverseTablePlan {
  network: DmxOutputConfig[];
  usb: {
    universe: number;
    enabled: boolean;
    port?: SerialPortSummary;
  }[];
}
export const isNoNetworkDmxSentinel = (row: DmxOutputConfig) => !row.enabled && row.protocol === "ArtNet"
  && row.target_ip === "0.0.0.0" && row.port === 0 && row.universe === 0
  && row.serial_port === "" && row.serial_baud_rate === 57600;
export function usbTableRows(status: UsbDmxUniversesStatus | null): DmxUniverseTablePlan["usb"] {
  const universes = new Set(status?.bindings.map(row => row.universe) ?? []);
  for (const cable of status?.cables ?? [])
    if (cable.universe !== null && (cable.active || cable.faulted || !cable.workerShutdownCompleted || cable.liveFrameQueued))
      universes.add(cable.universe);
  return [...universes].sort((a, b) => a - b).map(universe => ({ universe, enabled: status?.cables.some(row => row.universe === universe && row.active) ?? false }));
}
/** Revisions, details and frame queue counters are telemetry, not draft configuration. */
export function universeTableSignature(network: DmxOutputConfig[], status: UsbDmxUniversesStatus | null) {
  return JSON.stringify({ network: network.filter(row => !isNoNetworkDmxSentinel(row)), usb: usbTableRows(status), bindings: status?.bindings.map(({ universe, identity, state }) => ({ universe, identity, state })), known: status !== null, persistenceError: status?.persistenceError });
}
export function validateUniverseTablePlan(plan: DmxUniverseTablePlan, status: UsbDmxUniversesStatus | null, stale: boolean, blackout: boolean) {
  networkDmxRoutePayload(plan.network);
  if (plan.usb.length > 4)
    throw new Error("At most 4 USB-DMX devices are supported.");
  const old = usbTableRows(status);
  const changed = JSON.stringify(old) !== JSON.stringify(plan.usb);
  if (changed && (!status || stale || status.persistenceError))
    throw new Error("Refresh USB-DMX status before applying device changes.");
  const universes = new Set<number>();
  const ports: SerialPortSummary[] = [];
  for (const row of plan.usb) {
    if (!Number.isInteger(row.universe) || row.universe < 0 || row.universe > 32767 || universes.has(row.universe))
      throw new Error("USB-DMX universes must be unique and between 0 and 32767.");
    universes.add(row.universe);
    const binding = status?.bindings.find(value => value.universe === row.universe);
    if (!binding && !row.port)
      throw new Error("Select a device for the new USB-DMX universe.");
    if (row.port && !hasExactMachineLocalOpenDmxIdentity(row.port))
      throw new Error("Select an identifiable USB-DMX device.");
    const port = row.port ?? (binding ? { ...binding.identity, name: binding.identity.port_name } : undefined);
    if (port) {
      const identity = { ...port, port_name: port.name } as Parameters<typeof sameUsbDmxCable>[0];
      if (ports.some(other => sameUsbDmxCable(identity, other)))
        throw new Error("Each USB-DMX universe needs its own device.");
      ports.push(port);
    }
  }
  for (const cable of status?.cables ?? []) {
    if (cable.universe === null)
      continue;
    const next = plan.usb.find(row => row.universe === cable.universe);
    const stopping = !next || (cable.active && !next.enabled) || Boolean(next.port);
    if (stopping && (cable.active || cable.faulted || cable.liveFrameQueued) && !blackout)
      throw new Error("Engage safety blackout before disabling, removing or changing an active USB-DMX device.");
    if (stopping && !cable.workerShutdownCompleted && !cable.active)
      throw new Error("Wait for USB-DMX shutdown to complete, then refresh status.");
  }
}
