import type { DmxOutputConfig, SerialPortSummary, UsbDmxUniversesStatus, SerialDmxMachineBindingIdentity } from "./types";
import { isNoNetworkDmxSentinel, validateUniverseTablePlan, type DmxUniverseTablePlan } from "./dmxUniverseTable";
import { networkDmxRoutePayload } from "./networkDmxRoutes";
export interface DmxUniverseApplyContext {
  network: DmxOutputConfig[];
  usbStatus: UsbDmxUniversesStatus | null;
  usbStatusStale: boolean;
  safetyBlackoutEngaged: boolean;
  legacyRouteActive: boolean;
  isCurrent(): boolean;
  ensureAuthority(): Promise<void>;
  stopUsb(universe: number): Promise<void>;
  bindUsb(universe: number, port: SerialPortSummary): Promise<void>;
  applyNetwork(configs: DmxOutputConfig[]): Promise<void>;
  startUsb(universe: number): Promise<void>;
  clearUsb(universe: number): Promise<void>;
  refresh?(): Promise<void>;
}
export class DmxUniverseApplyError extends Error {
  readonly completedStages: readonly string[];
  constructor(stage: string, completed: string[], cause: unknown) {
    super(`DMX Apply failed at ${stage}. Confirmed completed stages: ${completed.length ? completed.join(", ") : "none"}. The failed stage may already have applied; reload authoritative settings before retrying. ${String(cause)}`, { cause });
    this.name = "DmxUniverseApplyError";
    this.completedStages = [...completed];
  }
}
const sameBinding = (identity: SerialDmxMachineBindingIdentity | undefined, port: SerialPortSummary) => identity !== undefined
  && identity.port_name === port.name && identity.port_type === port.port_type
  && identity.usb_vid === port.usb_vid && identity.usb_pid === port.usb_pid
  && identity.serial_number === port.serial_number && identity.manufacturer === port.manufacturer
  && identity.product === port.product
  && (identity.windows_device_instance_id ?? null) === (port.windows_device_instance_id ?? null)
  && (identity.macos_device_instance_id ?? null) === (port.macos_device_instance_id ?? null);
/** Executes individually fenced actions in order. Completed actions are retained on failure. */
export async function applyDmxUniverseTable(plan: DmxUniverseTablePlan, context: DmxUniverseApplyContext): Promise<void> {
  const completed: string[] = [];
  let stage = "preflight";
  const current = () => {
    if (!context.isCurrent())
      throw new Error("The project changed. Reload output settings before applying.");
  };
  const run = async (name: string, operation: () => Promise<void>) => {
    stage = name;
    current();
    await operation();
    // Record physical/settings completion even when the project changed during the await.
    completed.push(name);
    current();
  };
  try {
    current();
    // Capture plan values before yielding; callers cannot change the admitted operations in flight.
    const desired: DmxUniverseTablePlan = { network: plan.network.map(row => ({ ...row })), usb: plan.usb.map(row => ({ ...row, ...(row.port ? { port: { ...row.port } } : {}) })) };
    if (!context.usbStatus || context.usbStatusStale)
      throw new Error("Refresh USB-DMX status before applying output settings.");
    if (context.legacyRouteActive)
      throw new Error("A previous USB-DMX output is still running. Engage safety blackout, stop it, then refresh status before applying.");
    for (const row of desired.usb)
      if (row.port && sameBinding(context.usbStatus.bindings.find(binding => binding.universe === row.universe)?.identity, row.port))
        delete row.port;
    validateUniverseTablePlan(desired, context.usbStatus, context.usbStatusStale, context.safetyBlackoutEngaged);
    const status = context.usbStatus;
    const rebind = desired.usb.filter(row => row.port && !sameBinding(status.bindings.find(binding => binding.universe === row.universe)?.identity, row.port));
    const removed = status.bindings.filter(binding => !desired.usb.some(row => row.universe === binding.universe));
    const stop = status.cables.filter(cable => cable.universe !== null && cable.active && (!desired.usb.some(row => row.universe === cable.universe && row.enabled)
      || rebind.some(row => row.universe === cable.universe)));
    if (stop.length && !context.safetyBlackoutEngaged)
      throw new Error("Engage safety blackout before disabling, removing or changing an active USB-DMX device.");
    const start = desired.usb.filter(row => row.enabled && (!status.cables.some(cable => cable.universe === row.universe && cable.active) || rebind.some(value => value.universe === row.universe)));
    // Preflight every planned USB mutation before the first authority or output action.
    const mutating = new Set([...rebind.map(row => row.universe), ...removed.map(row => row.universe), ...start.map(row => row.universe)]);
    for (const universe of mutating) {
      const cable = status.cables.find(row => row.universe === universe);
      const willStop = stop.some(row => row.universe === universe);
      if (cable && !willStop) {
        if (!cable.workerShutdownCompleted || cable.liveFrameQueued || (cable.zeroFrameQueued && !cable.zeroFramePhysicalWriteCompleted && !cable.faulted))
          throw new Error(`Wait for USB-DMX universe ${universe} to finish stopping, then refresh status.`);
        if (cable.faulted && !context.safetyBlackoutEngaged)
          throw new Error(`Engage safety blackout before recovering USB-DMX universe ${universe}.`);
      }
      const binding = status.bindings.find(row => row.universe === universe);
      if (start.some(row => row.universe === universe) && !rebind.some(row => row.universe === universe) && binding?.state !== "selected_and_present")
        throw new Error(`Reconnect or select the USB-DMX device for universe ${universe}.`);
    }
    const appliedNetwork = context.network.filter(row => !isNoNetworkDmxSentinel(row));
    const networkChanged = JSON.stringify(networkDmxRoutePayload(appliedNetwork)) !== JSON.stringify(networkDmxRoutePayload(desired.network));
    if (!stop.length && !rebind.length && !removed.length && !start.length && !networkChanged)
      return;
    await run("output authority", () => context.ensureAuthority());
    for (const cable of stop)
      await run(`stop USB universe ${cable.universe}`, () => context.stopUsb(cable.universe!));
    // Release safely stopped old assignments before binding a transferred cable.
    const clear = [...removed, ...rebind.map(row => status.bindings.find(binding => binding.universe === row.universe)).filter((row): row is NonNullable<typeof row> => Boolean(row))];
    for (const binding of clear)
      await run(`clear USB universe ${binding.universe}`, () => context.clearUsb(binding.universe));
    for (const row of rebind)
      await run(`bind USB universe ${row.universe}`, () => context.bindUsb(row.universe, row.port!));
    if (networkChanged)
      await run("network outputs", () => context.applyNetwork(desired.network));
    for (const row of start)
      await run(`start USB universe ${row.universe}`, () => context.startUsb(row.universe));

    if (context.refresh)
      await run("refresh output settings", () => context.refresh!());
  }
  catch (cause) {
    const failure = new DmxUniverseApplyError(stage, completed, cause);
    // Recovery is read-only. A refresh failure must not hide the operation that failed.
    if (context.refresh && context.isCurrent() && stage !== "refresh output settings") {
      try {
        current();
        await context.refresh();
        current();
      }
      catch { /* retain primary failure */ }
    }
    throw failure;
  }
}
