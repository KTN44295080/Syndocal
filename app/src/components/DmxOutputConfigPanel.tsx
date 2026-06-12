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

export function DmxOutputConfigPanel(props: DmxOutputConfigPanelProps) {
  return (
    <>
      <h2>Output</h2>
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
          <option value="EnttecOpenDmx">Enttec Open DMX</option>
        </select>
      </label>
      <Show when={!props.isSerialProtocol(props.output.protocol)}>
        <label>
          Target IP
          <input
            value={props.output.target_ip}
            onInput={(event) => props.onOutputChange({ ...props.output, target_ip: event.currentTarget.value })}
          />
        </label>
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
    </>
  );
}
