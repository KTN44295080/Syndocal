import { For, createMemo, createSignal, onCleanup, onMount } from "solid-js";
import type {
  SerialDmxMachineBindingIdentity,
  SerialPortSummary,
  UsbDmxCableStatus,
  UsbDmxUniverseBindingStatus,
  UsbDmxUniversesStatus,
} from "../types";
import { hasExactMachineLocalOpenDmxIdentity, sameUsbDmxCable } from "../usbDmxIdentity";
import { serialDmxDeviceInstance } from "../serialDmxDeviceInstance";

export const MAX_USB_DMX_CABLES = 3;
const MIN_UNIVERSE = 1;
const MAX_UNIVERSE = 32_767;
const STATUS_REFRESH_MS = 2_000;

export interface UsbDmxCableSource {
  readonly status: UsbDmxUniversesStatus | null;
  /** The last read failed; `status` is the last known state. */
  readonly stale: boolean;
  readonly serialPorts: SerialPortSummary[];
  /** The Universe 0 (Show DMX) cable, which no universe row may take. */
  readonly showDmxCable: SerialDmxMachineBindingIdentity | null;
  readonly safetyBlackoutEngaged: boolean;
  onRefresh: () => void | Promise<void>;
  onRescan: () => void | Promise<void>;
  onBind: (universe: number, port: SerialPortSummary) => void | Promise<void>;
  onClear: (universe: number) => void | Promise<void>;
  onStart: (universe: number) => void | Promise<void>;
  onStop: (universe: number) => void | Promise<void>;
}

export const usbDmxPortKey = (port: SerialPortSummary) => `${port.name}\u0000${serialDmxDeviceInstance(port) ?? ""}`;

type CableState = "missing" | "fault" | "sending" | "armed" | "stopping" | "stopped";

/** A universe row: its binding (if readable) and its engine cable (if any). */
interface CableRow {
  universe: number;
  binding: UsbDmxUniverseBindingStatus | null;
  cable: UsbDmxCableStatus | null;
}

/**
 * The independent USB-DMX cables (Universe 1 and up): one cable outputs one
 * universe. Universe 0 stays on Show DMX. Binding is machine-local and never
 * written to the project. Starts the status poll for as long as its owner
 * is mounted.
 */
export function createUsbDmxCableModel(props: UsbDmxCableSource) {
  const [busy, setBusy] = createSignal(false);
  const [newUniverse, setNewUniverse] = createSignal<number>(Number.NaN);
  const [newPortKey, setNewPortKey] = createSignal("");

  onMount(() => {
    void props.onRefresh();
    const timer = window.setInterval(() => void props.onRefresh(), STATUS_REFRESH_MS);
    onCleanup(() => window.clearInterval(timer));
  });

  const bindings = createMemo(() =>
    [...(props.status?.bindings ?? [])].sort((left, right) => left.universe - right.universe));
  const cableFor = (universe: number): UsbDmxCableStatus | undefined =>
    props.status?.cables.find((cable) => cable.universe === universe);
  // A running, faulted or unjoined cable keeps its row (and Stop) even when
  // its binding cannot be read.
  const rows = createMemo<CableRow[]>(() => {
    const byUniverse = new Map<number, CableRow>();
    for (const binding of bindings()) {
      byUniverse.set(binding.universe, { universe: binding.universe, binding, cable: cableFor(binding.universe) ?? null });
    }
    for (const cable of props.status?.cables ?? []) {
      if (cable.universe === null || byUniverse.has(cable.universe)) continue;
      if (cable.active || cable.faulted || !cable.workerShutdownCompleted || cable.liveFrameQueued) {
        byUniverse.set(cable.universe, { universe: cable.universe, binding: null, cable });
      }
    }
    return [...byUniverse.values()].sort((left, right) => left.universe - right.universe);
  });
  // Each status poll builds new row objects. The table is keyed by universe
  // so a poll updates a row in place instead of rebuilding it (which would
  // drop focus or a click in flight).
  const universes = createMemo(() => rows().map((row) => row.universe), [], {
    equals: (left, right) => left.length === right.length && left.every((universe, index) => universe === right[index]),
  });
  const rowFor = (universe: number): CableRow =>
    rows().find((row) => row.universe === universe) ?? { universe, binding: null, cable: null };
  const nextFreeUniverse = createMemo(() => {
    const used = new Set(bindings().map((binding) => binding.universe));
    let universe = MIN_UNIVERSE;
    while (used.has(universe)) universe++;
    return universe;
  });
  const universeToBind = () => Number.isNaN(newUniverse()) ? nextFreeUniverse() : newUniverse();
  const availablePorts = createMemo(() => props.serialPorts.filter((port) =>
    hasExactMachineLocalOpenDmxIdentity(port)
    && !(props.showDmxCable && sameUsbDmxCable(props.showDmxCable, port))
    && !bindings().some((binding) => sameUsbDmxCable(binding.identity, port))));
  const selectedPort = () => {
    const key = newPortKey();
    const ports = availablePorts();
    return ports.find((port) => usbDmxPortKey(port) === key) ?? (ports.length === 1 ? ports[0] : undefined);
  };
  const universeValid = () => {
    const universe = universeToBind();
    return Number.isInteger(universe) && universe >= MIN_UNIVERSE && universe <= MAX_UNIVERSE
      && !bindings().some((binding) => binding.universe === universe);
  };
  /** Another cable may be added at all (the device and universe aside). */
  const canAddCable = () => props.status !== null && !props.stale && bindings().length < MAX_USB_DMX_CABLES
    && !props.status?.persistenceError;
  const canBind = () => !busy() && canAddCable() && universeValid() && Boolean(selectedPort());

  const cableState = (row: CableRow): CableState => {
    const cable = row.cable;
    if (cable?.faulted) return "fault";
    if (cable?.active) return cable.liveFrameQueued ? "sending" : "armed";
    if (cable && !cable.workerShutdownCompleted) return "stopping";
    if (cable?.zeroFrameQueued && !cable.zeroFramePhysicalWriteCompleted) return "stopping";
    if (row.binding?.state !== "selected_and_present") return "missing";
    return "stopped";
  };
  const present = (row: CableRow) => row.binding?.state === "selected_and_present";
  const joined = (row: CableRow) => row.cable?.workerShutdownCompleted !== false;
  // The same admission native code applies; it stays the authority.
  const canStart = (row: CableRow) => !props.stale && present(row)
    && (cableState(row) === "stopped" || (cableState(row) === "fault" && joined(row) && props.safetyBlackoutEngaged));
  const canStop = (row: CableRow) => props.safetyBlackoutEngaged
    && (cableState(row) === "sending" || cableState(row) === "armed" || (cableState(row) === "fault" && joined(row)));
  const canRemove = (row: CableRow) => !props.stale && row.binding !== null
    && (cableState(row) === "stopped" || cableState(row) === "missing"
      || (cableState(row) === "fault" && joined(row) && props.safetyBlackoutEngaged));

  const run = async (operation: () => void | Promise<void>) => {
    if (busy()) return;
    setBusy(true);
    try {
      await operation();
    } finally {
      setBusy(false);
    }
  };
  const bind = () => {
    const port = selectedPort();
    const universe = universeToBind();
    if (!port || !canBind()) return;
    void run(async () => {
      await props.onBind(universe, port);
      setNewUniverse(Number.NaN);
      setNewPortKey("");
    });
  };

  return {
    source: props,
    busy, rows, universes, rowFor, bindings, cableState, canStart, canStop, canRemove, run,
    availablePorts, selectedPort, setNewPortKey, universeToBind, setNewUniverse,
    canAddCable, canBind, bind,
  };
}

export type UsbDmxCableModel = ReturnType<typeof createUsbDmxCableModel>;

const stateLabel: Record<CableState, string> = {
  missing: "Cable missing",
  fault: "Fault · S0 latched",
  sending: "Sending",
  armed: "Armed · S0",
  stopping: "Shutdown pending",
  stopped: "Stopped",
};
const stateTone: Record<CableState, "ready" | "idle" | "error"> = {
  missing: "error",
  fault: "error",
  sending: "ready",
  armed: "ready",
  stopping: "error",
  stopped: "idle",
};

/** The USB-DMX cables as rows of the DMX universes table. */
export function UsbDmxCableRows(props: { model: UsbDmxCableModel }) {
  const { busy, universes, rowFor, cableState, canStart, canStop, canRemove, run, source } = props.model;
  return (
    <tbody class="dmxUniverseGroup" data-usb-dmx-cables>
      <For each={universes()}>{(universe) => {
        const row = () => rowFor(universe);
        const state = () => cableState(row());
        const identity = () => row().binding?.identity;
        return (
          <tr data-usb-dmx-cable={universe}>
            <th scope="row" class="tabularNums" data-no-localize>{`U${universe}`}</th>
            <td>USB-DMX</td>
            <td>
              <span class="dmxUniverseTarget" data-no-localize title={row().binding?.detail ?? row().cable?.detail ?? ""}>
                {identity()
                  ? `${identity()!.port_name} · ${identity()!.manufacturer} / ${identity()!.product} · ${identity()!.serial_number}`
                  : "—"}
              </span>
            </td>
            <td>
              <span
                class={`ioConnectionState ${stateTone[state()]}`}
                data-usb-dmx-cable-state={state()}
                title={row().cable?.detail ?? row().binding?.detail ?? ""}
              >
                <i aria-hidden="true" />{stateLabel[state()]}
              </span>
            </td>
            <td>
              <div class="dmxUniverseActions">
                <button
                  type="button"
                  class="primary"
                  data-usb-dmx-cable-start
                  disabled={busy() || !canStart(row())}
                  title="Opens the cable and writes one zero frame first; the safety blackout state is unchanged."
                  onClick={() => void run(() => source.onStart(universe))}
                >Start</button>
                <button
                  type="button"
                  data-usb-dmx-cable-stop
                  disabled={busy() || !canStop(row())}
                  title={source.safetyBlackoutEngaged
                    ? "Writes one zero frame, then stops the cable. A faulted cable is taken out of service."
                    : "Stopping a cable requires the safety blackout (S0)."}
                  onClick={() => void run(() => source.onStop(universe))}
                >Stop</button>
                <button
                  type="button"
                  data-usb-dmx-cable-remove
                  disabled={busy() || !canRemove(row())}
                  onClick={() => void run(() => source.onClear(universe))}
                >Remove</button>
              </div>
            </td>
          </tr>
        );
      }}</For>
    </tbody>
  );
}
