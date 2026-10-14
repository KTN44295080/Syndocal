import { For, Show, createMemo, createSignal, onCleanup, onMount } from "solid-js";
import type {
  SerialDmxMachineBindingIdentity,
  SerialPortSummary,
  UsbDmxCableStatus,
  UsbDmxUniverseBindingStatus,
  UsbDmxUniversesStatus,
} from "../types";
import { hasExactMachineLocalOpenDmxIdentity, sameUsbDmxCable } from "../usbDmxIdentity";
import { serialDmxDeviceInstance } from "../serialDmxDeviceInstance";

const MAX_CABLES = 3;
const MIN_UNIVERSE = 1;
const MAX_UNIVERSE = 32_767;
const STATUS_REFRESH_MS = 2_000;

interface UsbDmxCablesPanelProps {
  status: UsbDmxUniversesStatus | null;
  serialPorts: SerialPortSummary[];
  /** The Universe 0 (Show DMX) cable, which no universe row may take. */
  showDmxCable: SerialDmxMachineBindingIdentity | null;
  safetyBlackoutEngaged: boolean;
  onRefresh: () => void | Promise<void>;
  onRescan: () => void | Promise<void>;
  onBind: (universe: number, port: SerialPortSummary) => void | Promise<void>;
  onClear: (universe: number) => void | Promise<void>;
  onStart: (universe: number) => void | Promise<void>;
  onStop: (universe: number) => void | Promise<void>;
}

const portKey = (port: SerialPortSummary) => `${port.name}\u0000${serialDmxDeviceInstance(port) ?? ""}`;

type CableState = "missing" | "fault" | "sending" | "armed" | "stopping" | "stopped";

/**
 * One row per universe that has its own USB-DMX cable (Universe 1 and up).
 * Universe 0 stays on Show DMX above; network universes stay below. Binding
 * is machine-local and never written to the project.
 */
export function UsbDmxCablesPanel(props: UsbDmxCablesPanelProps) {
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
    return ports.find((port) => portKey(port) === key) ?? (ports.length === 1 ? ports[0] : undefined);
  };
  const universeValid = () => {
    const universe = universeToBind();
    return Number.isInteger(universe) && universe >= MIN_UNIVERSE && universe <= MAX_UNIVERSE
      && !bindings().some((binding) => binding.universe === universe);
  };
  const canBind = () => !busy() && props.status !== null && bindings().length < MAX_CABLES
    && universeValid() && Boolean(selectedPort()) && !props.status?.persistenceError;

  const cableState = (binding: UsbDmxUniverseBindingStatus): CableState => {
    const cable = cableFor(binding.universe);
    if (cable?.faulted) return "fault";
    if (cable?.active) return cable.liveFrameQueued ? "sending" : "armed";
    if (cable && !cable.workerShutdownCompleted) return "stopping";
    if (binding.state !== "selected_and_present") return "missing";
    return "stopped";
  };
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

  const run = async (operation: () => void | Promise<void>) => {
    if (busy()) return;
    setBusy(true);
    try {
      await operation();
    } finally {
      setBusy(false);
    }
  };

  return (
    <section class="networkDmxRoutes usbDmxCables" data-usb-dmx-cables>
      <header class="panelHeader">
        <h3>USB-DMX cables</h3>
        <span>{bindings().length} / {MAX_CABLES}</span>
      </header>
      <p class="hint">
        Give a universe its own USB-DMX cable. One cable outputs one universe; Universe 0 uses Show DMX above.
      </p>
      <Show when={props.status === null}>
        <p class="hint" role="status" data-usb-dmx-cables-unavailable>USB-DMX cable status is unavailable; cables cannot be changed.</p>
      </Show>
      <Show when={props.status?.persistenceError}>
        {(error) => <p class="fieldError" role="alert" data-no-localize>{error()}</p>}
      </Show>
      <div class="networkDmxRouteRows">
        <For each={bindings()}>{(binding) => {
          const state = () => cableState(binding);
          const cable = () => cableFor(binding.universe);
          return (
            <fieldset class="networkDmxRouteRow usbDmxCableRow" disabled={busy()} data-usb-dmx-cable={binding.universe}>
              <legend data-no-localize>{`USB-DMX · U${binding.universe}`}</legend>
              <span class="usbDmxCableDevice" data-no-localize title={binding.detail}>
                {`${binding.identity.port_name} · ${binding.identity.manufacturer} / ${binding.identity.product} · ${binding.identity.serial_number}`}
              </span>
              <span
                class={`ioConnectionState ${stateTone[state()]}`}
                data-usb-dmx-cable-state={state()}
                title={cable()?.detail ?? binding.detail}
              >
                <i aria-hidden="true" />{stateLabel[state()]}
              </span>
              <button
                type="button"
                class="primary"
                data-usb-dmx-cable-start
                disabled={state() !== "stopped" && state() !== "fault"}
                title="Opens the cable and writes one zero frame first; the safety blackout state is unchanged."
                onClick={() => void run(() => props.onStart(binding.universe))}
              >Start</button>
              <button
                type="button"
                data-usb-dmx-cable-stop
                disabled={!props.safetyBlackoutEngaged
                  || !(state() === "sending" || state() === "armed"
                    || (state() === "fault" && cable()?.workerShutdownCompleted === true))}
                title={props.safetyBlackoutEngaged
                  ? "Writes one zero frame, then stops the cable. A faulted cable is taken out of service."
                  : "Stopping a cable requires the safety blackout (S0)."}
                onClick={() => void run(() => props.onStop(binding.universe))}
              >Stop</button>
              <button
                type="button"
                data-usb-dmx-cable-remove
                disabled={state() === "sending" || state() === "armed" || state() === "stopping"}
                onClick={() => void run(() => props.onClear(binding.universe))}
              >Remove</button>
            </fieldset>
          );
        }}</For>
      </div>
      <Show when={bindings().length < MAX_CABLES}>
        <fieldset class="networkDmxRouteRow usbDmxCableAdd" disabled={busy()} data-usb-dmx-cable-add>
          <legend>Add a cable</legend>
          <label>Universe
            <input
              data-usb-dmx-cable-universe
              type="number"
              min={MIN_UNIVERSE}
              max={MAX_UNIVERSE}
              value={universeToBind()}
              onInput={(event) => setNewUniverse(event.currentTarget.value === "" ? Number.NaN : Number(event.currentTarget.value))}
            />
          </label>
          <label class="networkDmxDestination">USB-DMX
            <select
              data-usb-dmx-cable-device
              value={selectedPort() ? portKey(selectedPort()!) : ""}
              onInput={(event) => setNewPortKey(event.currentTarget.value)}
            >
              <option value="">{availablePorts().length === 0 ? "No free USB-DMX device" : "Select a device"}</option>
              <For each={availablePorts()}>{(port) => (
                <option value={portKey(port)} data-no-localize>
                  {`${port.name} · ${port.manufacturer ?? "unknown"} / ${port.product ?? "unknown"} · ${port.serial_number ?? ""}`}
                </option>
              )}</For>
            </select>
          </label>
          <button
            type="button"
            class="primary"
            data-usb-dmx-cable-bind
            disabled={!canBind()}
            onClick={() => {
              const port = selectedPort();
              const universe = universeToBind();
              if (!port || !universeValid()) return;
              void run(async () => {
                await props.onBind(universe, port);
                setNewUniverse(Number.NaN);
                setNewPortKey("");
              });
            }}
          >Add cable</button>
          <button type="button" data-usb-dmx-cable-rescan onClick={() => void run(() => props.onRescan())}>Rescan devices</button>
        </fieldset>
      </Show>
    </section>
  );
}
