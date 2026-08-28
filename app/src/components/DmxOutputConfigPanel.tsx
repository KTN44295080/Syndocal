import { createSignal } from "solid-js";
import type {
  DmxOutputConfig,
  SerialDmxMachineBindingStatus,
  SerialPortSummary,
} from "../types";

interface DmxOutputConfigPanelProps {
  output: DmxOutputConfig;
  serialPorts: readonly SerialPortSummary[];
  binding: SerialDmxMachineBindingStatus | null;
  onRefreshSerialPorts: () => void | Promise<void>;
  onSelectMachineBinding: (port: SerialPortSummary) => void | Promise<void>;
  onEnableStagedShowSerialRoute: () => void | Promise<void>;
}

const approvedShowRoute = (output: DmxOutputConfig) =>
  output.protocol === "EnttecOpenDmx"
  && output.serial_port === ""
  && output.serial_baud_rate === 250_000
  && output.universe === 0;

const bindingOptionLabel = (port: SerialPortSummary) => [
  port.name,
  `${port.manufacturer || "unknown manufacturer"} / ${port.product || "unknown product"}`,
  `serial ${port.serial_number || "not exposed"}`,
  port.windows_device_instance_id || "PnP instance not exposed",
].join(" · ");

const bindingKey = (port: SerialPortSummary) => `${port.name}\u0000${port.windows_device_instance_id || ""}`;

/**
 * Production show output is one bounded logical route plus one explicitly
 * selected machine-local USB interface. It intentionally has no arbitrary
 * route editor and never derives a physical COM alias from project data.
 */
export function DmxOutputConfigPanel(props: DmxOutputConfigPanelProps) {
  const [pendingSelectionKey, setPendingSelectionKey] = createSignal("");
  const selectedKey = () => props.binding?.selected
    ? `${props.binding.selected.port_name}\u0000${props.binding.selected.windows_device_instance_id}`
    : "";
  const selectedPort = () => props.serialPorts.find((port) => bindingKey(port) === selectedKey());
  const pendingPort = () => props.serialPorts.find((port) => bindingKey(port) === pendingSelectionKey());
  const bindingReady = () => props.binding?.state === "selected_and_present" && !!selectedPort();
  const ready = () => approvedShowRoute(props.output) && bindingReady();
  const routeState = () => !approvedShowRoute(props.output)
    ? "Logical route mismatch"
    : bindingReady()
      ? props.output.enabled ? "Enabled" : "Staged disabled"
      : "USB-DMX selection required";
  const routeStateTone = () => !ready() ? "error" : props.output.enabled ? "ready" : "idle";

  return (
    <section class="dmxOutputConfigPanel ioConnectionDesk" data-io-default-surface="dmx">
      <header class="ioDeskHeader">
        <div><h2>DMX Connections</h2><span>Production show route</span></div>
        <span class={`ioConnectionState ${routeStateTone()}`}><i aria-hidden="true" />{routeState()}</span>
      </header>

      <div class="dmxRouteBuilder dmxPrimaryControls">
        <button type="button" onClick={() => void props.onRefreshSerialPorts()}>Refresh USB-DMX interfaces</button>
        <label>
          Machine-local USB-DMX interface
          <select
            data-io-control="serial-dmx-machine-binding"
            value={pendingSelectionKey() || selectedKey()}
            onChange={(event) => setPendingSelectionKey(event.currentTarget.value)}
          >
            <option value="">Select and confirm an enumerated interface…</option>
            {props.serialPorts
              .filter((port) => !!port.windows_device_instance_id)
              .map((port) => <option value={bindingKey(port)}>{bindingOptionLabel(port)}</option>)}
          </select>
        </label>
        <button
          type="button"
          data-io-control="confirm-serial-dmx-machine-binding"
          disabled={!pendingPort()}
          onClick={() => {
            const port = pendingPort();
            if (port) void props.onSelectMachineBinding(port);
          }}
        >Confirm selected USB-DMX interface</button>
        <button
          data-io-control="dmx-enable-staged-show-serial-route"
          class="primary"
          disabled={!ready() || props.output.enabled}
          aria-describedby="dmx-show-route-confirmation"
          onClick={() => void props.onEnableStagedShowSerialRoute()}
        >Confirm and enable staged show route</button>
      </div>

      <div class="dmxRouteList" data-io-route-list data-io-route-total="1" data-io-route-page-count="1">
        <div class="panelHeader"><h3>Authored logical route</h3><span>1</span></div>
        <div class="dmxRouteRows"><div class="dmxRouteRow" data-io-route-row data-route-index="0">
          <span class={`ioStatusDot ${routeStateTone() === "ready" ? "ok" : routeStateTone()}`} aria-hidden="true" />
          <strong>Enttec Open DMX</strong><span data-io-route-target>Machine-local USB binding</span>
          <span data-io-route-universe>{`U${props.output.universe} · ${props.output.serial_baud_rate}`}</span>
        </div></div>
      </div>

      <p id="dmx-show-route-confirmation" class="ioDisclosureDescription">
        {props.binding?.detail || "Read the enumerated interfaces, then select one explicitly on this PC."}
      </p>
      {props.binding?.selected && <p class="ioDisclosureDescription" data-io-selected-windows-instance>
        Selected identity: {props.binding.selected.port_name} · {props.binding.selected.manufacturer} / {props.binding.selected.product} · serial {props.binding.selected.serial_number} · {props.binding.selected.windows_device_instance_id}
      </p>}
      {!ready() && <p class="ioDisclosureDescription" role="alert">
        The portable project route is fixed at Enttec Open DMX / U0 / 250000. Select the exact USB-DMX device for this PC; missing, stale, renumbered, or ambiguous hardware is never substituted.
      </p>}
      <p class="ioDisclosureDescription">
        Confirming enables only this staged logical route after native lease, safety-blackout, and exact opened-handle checks. Selecting a USB interface writes only machine-local settings and never changes the project.
      </p>
    </section>
  );
}
