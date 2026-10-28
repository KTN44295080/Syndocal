import "./DmxUniverseTable.css";
import { For, Index, Show, createEffect, createSignal, untrack, onMount, onCleanup } from "solid-js";
import type { DmxOutputConfig, DmxOutputRouteTelemetry, SerialPortSummary, UsbDmxUniversesStatus } from "../types";
import { isNoNetworkDmxSentinel, usbTableRows, universeTableSignature, validateUniverseTablePlan, type DmxUniverseTablePlan } from "../dmxUniverseTable";
import { hasExactMachineLocalOpenDmxIdentity, sameUsbDmxCable } from "../usbDmxIdentity";
import { serialDmxDeviceInstance } from "../serialDmxDeviceInstance";
import { networkDmxRouteState, sameNetworkDmxRoute } from "../dmxUniverseState";
export interface DmxOutputConfigPanelProps {
  output: DmxOutputConfig;
  routes: DmxOutputConfig[];
  routeTelemetry: DmxOutputRouteTelemetry[];
  projectEpoch: number;
  serialPorts: SerialPortSummary[];
  usbDmxUniverses: UsbDmxUniversesStatus | null;
  usbDmxUniversesStale: boolean;
  safetyBlackoutEngaged: boolean;
  onRefreshUsbDmxUniverses: () => void | Promise<void>;
  onRescanSerialPorts: () => void | Promise<void>;
  onApplyUniverseTable: (plan: DmxUniverseTablePlan) => Promise<void>;
  /** Recovery for a pre-table Show DMX worker started elsewhere (for example by an agent). */
  onStopLegacyUsbDmx: () => void | Promise<void>;
}
const portKey = (port: SerialPortSummary) => `${port.name}|${serialDmxDeviceInstance(port)}`;
const PAGE_SIZE = 5;
export function DmxOutputConfigPanel(props: DmxOutputConfigPanelProps) {
  const [network, setNetwork] = createSignal<DmxOutputConfig[]>([]), [usb, setUsb] = createSignal<DmxUniverseTablePlan["usb"]>([]);
  const [dirty, setDirty] = createSignal(false), [conflict, setConflict] = createSignal(false), [busy, setBusy] = createSignal(false), [error, setError] = createSignal("");
  const [transport, setTransport] = createSignal("ArtNet"), [selected, setSelected] = createSignal(""), [page, setPage] = createSignal(0);
  let epoch: number | undefined, observed = "";
  const reload = () => {
    setNetwork(props.routes.filter(row => !isNoNetworkDmxSentinel(row)).map(row => ({ ...row })));
    setUsb(usbTableRows(props.usbDmxUniverses));
    setDirty(false);
    setConflict(false);
    setError("");
    setPage(0);
  };
  createEffect(() => {
    const signature = universeTableSignature(props.routes, props.usbDmxUniverses), nextEpoch = props.projectEpoch;
    untrack(() => {
      if (epoch !== nextEpoch || signature !== observed && !dirty())
        reload();
      else if (signature !== observed && !busy())
        setConflict(true);
      epoch = nextEpoch;
      observed = signature;
    });
  });
  onMount(() => {
    void props.onRefreshUsbDmxUniverses();
    const timer = window.setInterval(() => void props.onRefreshUsbDmxUniverses(), 2000);
    onCleanup(() => window.clearInterval(timer));
  });
  const edit = () => {
    setDirty(true);
    setError("");
  };
  const changeNetwork = (index: number, patch: Partial<DmxOutputConfig>) => {
    setNetwork(rows => rows.map((row, i) => i === index ? { ...row, ...patch } : row));
    edit();
  };
  const available = () => props.serialPorts.filter(port => hasExactMachineLocalOpenDmxIdentity(port) && !usb().some(row => {
    if (row.port)
      return portKey(row.port) === portKey(port) || sameUsbDmxCable({ ...row.port, port_name: row.port.name } as Parameters<typeof sameUsbDmxCable>[0], port);
    const binding = props.usbDmxUniverses?.bindings.find(value => value.universe === row.universe);
    return binding && sameUsbDmxCable(binding.identity, port);
  }));
  const bindingPort = (universe: number): SerialPortSummary | undefined => {
    const identity = props.usbDmxUniverses?.bindings.find(value => value.universe === universe)?.identity;
    return identity ? { ...identity, name: identity.port_name } : undefined;
  };
  const selectedPort = () => available().find(port => portKey(port) === selected()) ?? (available().length === 1 ? available()[0] : undefined);
  const total = () => usb().length + network().length, pageCount = () => Math.max(1, Math.ceil(total() / PAGE_SIZE));
  const activePage = () => Math.min(page(), pageCount() - 1);
  const visible = (index: number) => index >= activePage() * PAGE_SIZE && index < (activePage() + 1) * PAGE_SIZE;
  const add = () => {
    if (busy())
      return;
    if (transport() === "Usb") {
      const port = selectedPort();
      if (!port || usb().length >= 4 || !props.usbDmxUniverses || props.usbDmxUniversesStale)
        return;
      let universe = 0;
      while (usb().some(row => row.universe === universe))
        universe++;
      setUsb(rows => [...rows, { universe, enabled: true, port }]);
      setPage(Math.floor((usb().length - 1) / PAGE_SIZE));
    }
    else {
      if (network().length >= 256)
        return;
      const protocol = transport() as "ArtNet" | "Sacn";
      let universe = protocol === "Sacn" ? 1 : 0;
      while (network().some(row => row.protocol === protocol && row.universe === universe))
        universe++;
      setNetwork(rows => [...rows, { enabled: true, protocol, universe, target_ip: protocol === "Sacn" ? "multicast" : "127.0.0.1", port: protocol === "Sacn" ? 5568 : 6454, serial_port: "", serial_baud_rate: 57600 }]);
      setPage(Math.floor((total() - 1) / PAGE_SIZE));
    }
    edit();
  };
  const plan = (): DmxUniverseTablePlan => ({ network: network().map(row => ({ ...row })), usb: usb().map(row => ({ ...row })) });
  const validation = () => {
    try {
      validateUniverseTablePlan(plan(), props.usbDmxUniverses, props.usbDmxUniversesStale, props.safetyBlackoutEngaged);
      return "";
    }
    catch (failure) {
      return String(failure).replace(/^Error: /, "");
    }
  };
  const apply = async () => {
    if (busy() || conflict() || !dirty())
      return;
    const captured = props.projectEpoch;
    try {
      validateUniverseTablePlan(plan(), props.usbDmxUniverses, props.usbDmxUniversesStale, props.safetyBlackoutEngaged);
      setBusy(true);
      setError("");
      await props.onApplyUniverseTable(plan());
      if (captured === props.projectEpoch)
        reload();
    }
    catch (failure) {
      if (captured === props.projectEpoch) {
        setError(String(failure));
        setConflict(true);
      }
    }
    finally {
      setBusy(false);
    }
  };
  const usbState = (universe: number) => {
    const cable = props.usbDmxUniverses?.cables.find(row => row.universe === universe);
    const binding = props.usbDmxUniverses?.bindings.find(row => row.universe === universe);
    if (props.usbDmxUniversesStale)
      return "Status unavailable";
    if (cable?.faulted)
      return "Fault";
    if (cable?.active)
      return cable.liveFrameQueued ? "Sending" : "Active";
    if (cable && !cable.workerShutdownCompleted)
      return "Shutdown pending";
    return binding?.state === "selected_and_present" ? "Stopped" : "Device unavailable";
  };
  const usbPending = (row: DmxUniverseTablePlan["usb"][number]) => Boolean(row.port) || usbTableRows(props.usbDmxUniverses).find(value => value.universe === row.universe)?.enabled !== row.enabled;
  return (
  <section
    class="dmxOutputConfigPanel ioConnectionDesk"
    data-io-default-surface="dmx">
    <header
      class="ioDeskHeader">
      <div>
        <h2>DMX universes
        </h2>
        <span>USB-DMX · Art-Net · sACN
        </span>
      </div>
      <span
        class={`ioConnectionState ${props.safetyBlackoutEngaged ? "error" : "idle"}`}
        data-dmx-safety-state>{props.safetyBlackoutEngaged ? "Safety blackout engaged" : "Safety blackout clear"}
      </span>
    </header>
    <div
      class="dmxUniverseTableViewport">
      <table
        class="dmxUniverseTable"
        data-dmx-universe-table>
        <thead>
          <tr>
            <th>Universe
            </th>
            <th>Transport
            </th>
            <th>Device / destination
            </th>
            <th>State
            </th>
            <th>Actions
            </th>
          </tr>
        </thead>
        <tbody
          data-usb-dmx-cables>
          <Index
            each={usb()}>{(row, index) =>
              <Show
                when={visible(index)}>
                <tr
                  data-usb-dmx-cable={row().universe}>
                  <th>
                    <input
                      aria-label="USB universe"
                      type="number"
                      min="0"
                      max="32767"
                      value={row().universe}
                      disabled={busy()}
                      onInput={e => {
                        const universe = e.currentTarget.value === "" ? NaN : Number(e.currentTarget.value);
                        setUsb(rows => rows.map((value, i) => i === index ? { ...value, universe, port: value.port ?? bindingPort(value.universe) } : value));
                        edit();
                      }} />
                  </th>
                  <td>USB-DMX
                  </td>
                  <td>
                    <select
                      aria-label="USB-DMX output device"
                      disabled={busy()}
                      value={row().port ? portKey(row().port!) : portKey(bindingPort(row().universe) ?? { name: "", port_type: "" })}
                      onChange={e => {
                        const port = props.serialPorts.find(value => portKey(value) === e.currentTarget.value);
                        if (port) {
                          setUsb(rows => rows.map((value, i) => i === index ? { ...value, port } : value));
                          edit();
                        }
                      }}>
                      <option
                        value={row().port ? portKey(row().port!) : portKey(bindingPort(row().universe) ?? { name: "", port_type: "" })}
                        data-no-localize>{row().port?.name ?? bindingPort(row().universe)?.name ?? "Device unavailable"}
                      </option>
                      <For
                        each={available()}>{port =>
                          <option
                            value={portKey(port)}
                            data-no-localize>{port.name} · {port.product}
                          </option>}
                      </For>
                    </select>
                  </td>
                  <td>
                    <span
                      class="ioConnectionState idle"
                      data-usb-dmx-cable-state>{usbState(row().universe)}
                    </span>
                    <Show
                      when={usbPending(row())}>
                      <span> · Not applied
                      </span>
                    </Show>
                  </td>
                  <td>
                    <div
                      class="dmxUniverseActions">
                      <label
                        class="dmxUniverseEnabled">
                        <input
                          type="checkbox"
                          checked={row().enabled}
                          disabled={busy()}
                          onChange={e => {
                            const enabled = e.currentTarget.checked;
                            setUsb(rows => rows.map((value, i) => i === index ? { ...value, enabled } : value));
                            edit();
                          }} />Enabled
                      </label>
                      <button
                        disabled={busy()}
                        onClick={() => {
                          setUsb(rows => rows.filter((_, i) => i !== index));
                          edit();
                        }}>Remove
                      </button>
                    </div>
                  </td>
                </tr>
              </Show>}
          </Index>
        </tbody>
        <tbody
          data-network-dmx-routes>
          <Index
            each={network()}>{(row, index) => {
              const applied = () => props.routes.filter(value => !isNoNetworkDmxSentinel(value))[index];
              const state = () => applied() ? networkDmxRouteState(applied()!, props.routeTelemetry.find(value => value.index === index)) : { label: "Not applied", tone: "idle" };
              return (
              <Show
                when={visible(usb().length + index)}>
                <tr
                  data-network-dmx-route={index}>
                  <th>
                    <input
                      data-network-dmx-universe
                      aria-label="Universe"
                      type="number"
                      min={row().protocol === "Sacn" ? 1 : 0}
                      max={row().protocol === "Sacn" ? 63999 : 32767}
                      disabled={busy()}
                      value={row().universe}
                      onInput={e => changeNetwork(index, { universe: e.currentTarget.value === "" ? NaN : Number(e.currentTarget.value) })} />
                  </th>
                  <td>
                    <select
                      aria-label="Transport"
                      disabled={busy()}
                      value={row().protocol}
                      onChange={e => changeNetwork(index, { protocol: e.currentTarget.value as "ArtNet" | "Sacn", port: e.currentTarget.value === "Sacn" ? 5568 : 6454, universe: Math.max(e.currentTarget.value === "Sacn" ? 1 : 0, row().universe), target_ip: e.currentTarget.value === "Sacn" ? "multicast" : "127.0.0.1" })}>
                      <option
                        value="ArtNet">Art-Net
                      </option>
                      <option
                        value="Sacn">sACN
                      </option>
                    </select>
                  </td>
                  <td>
                    <div
                      class="dmxUniverseDestination">
                      <input
                        data-network-dmx-destination
                        aria-label="Destination"
                        disabled={busy()}
                        value={row().target_ip}
                        onInput={e => changeNetwork(index, { target_ip: e.currentTarget.value })} />
                      <input
                        aria-label="Port"
                        type="number"
                        min="1"
                        max="65535"
                        disabled={busy()}
                        value={row().port}
                        onInput={e => changeNetwork(index, { port: Number(e.currentTarget.value) })} />
                    </div>
                  </td>
                  <td>
                    <span
                      class={`ioConnectionState ${state().tone}`}>{state().label}
                    </span>
                    <Show
                      when={Boolean(applied()) && !sameNetworkDmxRoute(applied(), row())}>
                      <span> · Not applied
                      </span>
                    </Show>
                  </td>
                  <td>
                    <div
                      class="dmxUniverseActions">
                      <label
                        class="dmxUniverseEnabled">
                        <input
                          type="checkbox"
                          disabled={busy()}
                          checked={row().enabled}
                          onChange={e => changeNetwork(index, { enabled: e.currentTarget.checked })} />Enabled
                      </label>
                      <button
                        disabled={busy()}
                        onClick={() => {
                          setNetwork(rows => rows.filter((_, i) => i !== index));
                          edit();
                        }}>Remove
                      </button>
                    </div>
                  </td>
                </tr>
              </Show>);
            }}
          </Index>
        </tbody>
        <tfoot>
          <tr
            class="dmxUniverseAddRow"
            data-dmx-universe-add>
            <td>New universe
            </td>
            <td>
              <select
                data-dmx-universe-add-transport
                aria-label="New transport"
                disabled={busy()}
                value={transport()}
                onChange={e => setTransport(e.currentTarget.value)}>
                <option
                  value="ArtNet">Art-Net
                </option>
                <option
                  value="Sacn">sACN
                </option>
                <option
                  value="Usb">USB-DMX
                </option>
              </select>
            </td>
            <td>
              <Show
                when={transport() === "Usb"}
                fallback={
                  <span>Edit the destination in the new row, then Apply.
                  </span>}>
                <select
                  data-usb-dmx-cable-device
                  aria-label="USB-DMX device"
                  value={selectedPort() ? portKey(selectedPort()!) : ""}
                  onChange={e => setSelected(e.currentTarget.value)}>
                  <option
                    value="">Select a device
                  </option>
                  <For
                    each={available()}>{port =>
                      <option
                        value={portKey(port)}
                        data-no-localize>{port.name} · {port.product}
                      </option>}
                  </For>
                </select>
              </Show>
            </td>
            <td>{total()} outputs
            </td>
            <td>
              <div
                class="dmxUniverseActions">
                <button
                  class="primary"
                  data-dmx-universe-add-button
                  disabled={busy() || (transport() === "Usb" ? (!selectedPort() || usb().length >= 4 || !props.usbDmxUniverses || props.usbDmxUniversesStale) : network().length >= 256)}
                  onClick={add}>Add
                </button>
                <button
                  disabled={busy()}
                  onClick={() => void props.onRescanSerialPorts()}>Rescan devices
                </button>
              </div>
            </td>
          </tr>
        </tfoot>
      </table>
    </div>
    <div
      class="dmxRouteBuilder">
      <Show
        when={pageCount() > 1}>
        <button
          disabled={activePage() === 0}
          onClick={() => setPage(activePage() - 1)}>Previous
        </button>
        <span
          data-dmx-page>{activePage() + 1} / {pageCount()}
        </span>
        <button
          disabled={activePage() === pageCount() - 1}
          onClick={() => setPage(activePage() + 1)}>Next
        </button>
      </Show>
      <button
        class="primary"
        data-network-dmx-save
        data-dmx-universe-apply
        disabled={!dirty() || busy() || conflict() || Boolean(validation())}
        onClick={() => void apply()}>{busy() ? "Applying…" : "Apply"}
      </button>
      <button
        data-dmx-universe-reload
        disabled={busy()}
        onClick={reload}>Reload
      </button>
      <button
        disabled={busy()}
        onClick={() => void props.onRefreshUsbDmxUniverses()}>Refresh status
      </button>
    </div>
    <Show
      when={dirty()}>
      <p
        data-dmx-draft-pending>Changes pending. Apply updates network outputs and USB-DMX devices. If a device fails, completed changes remain; reload the refreshed settings.
      </p>
    </Show>
    <Show
      when={conflict()}>
      <p
        class="fieldError"
        role="alert">Output settings changed. Reload the current settings before applying.
      </p>
    </Show>
    <Show
      when={error()}>
      <div><p class="fieldError" role="alert">Some output settings could not be applied. Reload the refreshed settings before retrying.</p><details><summary>Diagnostics</summary><pre data-no-localize>{error()}</pre></details></div>
    </Show>
    <Show
      when={dirty() && validation()}>
      <p
        class="fieldError"
        role="alert">{validation()}
      </p>
    </Show>
    <Show
      when={props.usbDmxUniversesStale}>
      <p
        class="fieldError"
        role="alert">USB-DMX status could not be refreshed. Showing the last known state.
      </p>
    </Show>
    <Show
      when={props.usbDmxUniverses?.legacyRouteActive}>
      <div
        class="dmxRouteBuilder"
        data-dmx-legacy-usb-active>
        <p
          class="fieldError"
          role="alert">A previous USB-DMX output is still running. Engage safety blackout, stop it, then refresh status before applying.
        </p>
        <button
          disabled={busy() || !props.safetyBlackoutEngaged}
          onClick={() => void props.onStopLegacyUsbDmx()}>Stop USB-DMX
        </button>
      </div>
    </Show>
    <Show
      when={props.usbDmxUniverses?.persistenceError}>
      <p
        class="fieldError"
        role="alert"
        data-no-localize>{props.usbDmxUniverses?.persistenceError}
      </p>
    </Show>
  </section>);
}
