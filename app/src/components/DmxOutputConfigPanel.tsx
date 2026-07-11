import { For, Show } from "solid-js";
import type { DmxOutputConfig, SerialPortSummary } from "../types";

interface DmxOutputConfigPanelProps {
  output: DmxOutputConfig;
  serialPorts: SerialPortSummary[];
  isSerialProtocol: (protocol: DmxOutputConfig["protocol"]) => boolean;
  onOutputChange: (output: DmxOutputConfig) => void;
  onProtocolChange: (protocol: DmxOutputConfig["protocol"]) => void;
  onRefreshSerialPorts: () => void | Promise<void>;
  onApply: () => void | Promise<void>;
}

const SACN_MAX_UNIVERSE = 63_999;

const normalizeSacnUniverse = (universe: number) =>
  Math.max(1, Math.min(SACN_MAX_UNIVERSE, Math.trunc(Number.isFinite(universe) ? universe : 1)));

const sacnMulticastAddress = (universe: number) => {
  const normalized = normalizeSacnUniverse(universe);
  return `239.255.${normalized >> 8}.${normalized & 0xff}`;
};

const isSacnMulticastTarget = (target: string) => {
  const normalized = target.trim().toLowerCase();
  return normalized === "" || normalized === "auto" || normalized === "multicast";
};

const serialProtocolHint = (protocol: DmxOutputConfig["protocol"]) => {
  switch (protocol) {
    case "EnttecUsbPro":
      return {
        tone: "ok",
        label: "Device-timed USB PRO",
        detail: "Break and mark timing are handled by the interface firmware.",
      };
    case "DmxKingUltraDmx":
      return {
        tone: "ok",
        label: "PRO-compatible DMXKing",
        detail: "Uses the Enttec USB PRO packet path with DMXKing serial interfaces.",
      };
    case "EnttecOpenDmx":
      return {
        tone: "warn",
        label: "Host-timed Open DMX",
        detail: "FTDI break timing can vary by OS; use PRO/DMXKing for critical live output.",
      };
    default:
      return null;
  }
};

export function DmxOutputConfigPanel(props: DmxOutputConfigPanelProps) {
  return (
    <section class="dmxOutputConfigPanel">
      <header class="ioDeskHeader">
        <h2>DMX Output</h2>
        <span>{props.output.protocol}</span>
      </header>
      <label class="checkbox">
        <input
          type="checkbox"
          checked={props.output.enabled}
          onChange={(event) => props.onOutputChange({ ...props.output, enabled: event.currentTarget.checked })}
        />
        DMX output enabled
      </label>
      <label>
        Protocol
        <select value={props.output.protocol} onInput={(event) => props.onProtocolChange(event.currentTarget.value as DmxOutputConfig["protocol"])}>
          <option value="ArtNet">Art-Net</option>
          <option value="Sacn">sACN / E1.31</option>
          <option value="EnttecUsbPro">Enttec USB PRO</option>
          <option value="DmxKingUltraDmx">DMXKing ultraDMX</option>
          <option value="EnttecOpenDmx">Enttec Open DMX</option>
        </select>
      </label>
      <Show when={!props.isSerialProtocol(props.output.protocol)}>
        <label>
          Target IP
          <input
            value={props.output.target_ip}
            placeholder={props.output.protocol === "Sacn" ? "multicast, auto, or unicast IP" : "127.0.0.1"}
            onInput={(event) => props.onOutputChange({ ...props.output, target_ip: event.currentTarget.value })}
          />
        </label>
        <Show when={props.output.protocol === "Sacn"}>
          <div class="dmxOutputNetworkHint">
            <span>sACN multicast</span>
            <strong>{sacnMulticastAddress(props.output.universe)}</strong>
            <button
              type="button"
              title="Use the standard sACN multicast target for the selected universe"
              disabled={isSacnMulticastTarget(props.output.target_ip)}
              onClick={() => props.onOutputChange({ ...props.output, target_ip: "multicast" })}
            >
              Multicast
            </button>
          </div>
        </Show>
        <div class="split">
          <label>
            Port
            <input
              type="number"
              value={props.output.port}
              onInput={(event) => props.onOutputChange({ ...props.output, port: Number(event.currentTarget.value) })}
            />
          </label>
          <label>
            Universe
            <input
              type="number"
              min={props.output.protocol === "Sacn" ? "1" : "0"}
              value={props.output.universe}
              onInput={(event) => props.onOutputChange({ ...props.output, universe: Number(event.currentTarget.value) })}
            />
          </label>
        </div>
      </Show>
      <Show when={props.isSerialProtocol(props.output.protocol)}>
        <div class="serialOutput">
          <Show when={serialProtocolHint(props.output.protocol)}>
            {(hint) => (
              <div class={`dmxOutputProtocolHint ${hint().tone}`}>
                <strong>{hint().label}</strong>
                <span>{hint().detail}</span>
              </div>
            )}
          </Show>
          <div class="buttonRow">
            <button onClick={props.onRefreshSerialPorts}>Scan Serial</button>
            <button onClick={() => props.onOutputChange({ ...props.output, serial_port: "" })}>Clear Port</button>
          </div>
          <label>
            Serial port
            <select
              value={props.output.serial_port}
              onInput={(event) => props.onOutputChange({ ...props.output, serial_port: event.currentTarget.value })}
            >
              <For each={props.serialPorts}>
                {(port) => (
                  <option value={port.name}>
                    {port.name} / {port.port_type}
                  </option>
                )}
              </For>
            </select>
          </label>
          <label>
            Manual port
            <input
              value={props.output.serial_port}
              placeholder="COM3 or /dev/ttyUSB0"
              onInput={(event) => props.onOutputChange({ ...props.output, serial_port: event.currentTarget.value })}
            />
          </label>
          <label>
            Baud rate
            <input
              type="number"
              min="1"
              disabled={props.output.protocol === "EnttecOpenDmx"}
              value={props.output.serial_baud_rate}
              onInput={(event) => props.onOutputChange({ ...props.output, serial_baud_rate: Number(event.currentTarget.value) })}
            />
          </label>
        </div>
      </Show>
      <button class="primary" onClick={props.onApply}>
        Apply Output
      </button>
    </section>
  );
}
