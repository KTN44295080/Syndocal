import type { DmxInputConfig, DmxInputStatus } from "../types";

interface DmxInputPanelProps {
  config: DmxInputConfig;
  status: DmxInputStatus;
  onConfig: (config: DmxInputConfig) => void;
  onStart: () => void | Promise<void>;
  onStop: () => void | Promise<void>;
}

export function DmxInputPanel(props: DmxInputPanelProps) {
  const setConfig = (patch: Partial<DmxInputConfig>) => props.onConfig({ ...props.config, ...patch });

  return (
    <div class="dmxInputPanel">
      <div class="panelHeader">
        <h3>DMX Input / Merge</h3>
        <span class={props.status.signal_present ? "pill ok" : props.status.running ? "pill warn" : "pill"}>
          {props.status.signal_present ? "Signal" : props.status.running ? "Waiting" : "Stopped"}
        </span>
      </div>
      <div class="dmxInputFields">
        <label>
          Protocol
          <select
            data-io-control="dmx-input-protocol"
            value={props.config.protocol}
            disabled={props.status.running}
            onInput={(event) => {
              const protocol = event.currentTarget.value as DmxInputConfig["protocol"];
              setConfig({ protocol, port: protocol === "Sacn" ? 5568 : 6454, universe: protocol === "Sacn" ? Math.max(1, props.config.universe) : props.config.universe });
            }}
          >
            <option value="ArtNet">Art-Net</option>
            <option value="Sacn">sACN E1.31</option>
          </select>
        </label>
        <label>
          Bind IP
          <input value={props.config.bind_ip} disabled={props.status.running} onInput={(event) => setConfig({ bind_ip: event.currentTarget.value })} />
        </label>
        <label>
          Port
          <input type="number" min="1" max="65535" value={props.config.port} disabled={props.status.running} onInput={(event) => setConfig({ port: Number(event.currentTarget.value) })} />
        </label>
        <label>
          Universe
          <input type="number" min={props.config.protocol === "Sacn" ? 1 : 0} max="63999" value={props.config.universe} disabled={props.status.running} onInput={(event) => setConfig({ universe: Number(event.currentTarget.value) })} />
        </label>
        <label>
          Merge
          <select value={props.config.merge_mode} disabled={props.status.running} onInput={(event) => setConfig({ merge_mode: event.currentTarget.value as DmxInputConfig["merge_mode"] })}>
            <option value="Htp">HTP (highest wins)</option>
            <option value="Ltp">LTP (input wins)</option>
          </select>
        </label>
        <label>
          Loss timeout ms
          <input type="number" min="100" max="60000" value={props.config.timeout_ms} disabled={props.status.running} onInput={(event) => setConfig({ timeout_ms: Number(event.currentTarget.value) })} />
        </label>
      </div>
      <div class="dmxInputActions">
        <button class="primary" disabled={props.status.running} onClick={() => void props.onStart()}>Start Input</button>
        <button disabled={!props.status.running} onClick={() => void props.onStop()}>Stop</button>
        <span>{props.status.packets_received} packet(s) · {props.status.invalid_packets} invalid</span>
        <span title={props.status.source_address ?? ""}>{props.status.source_address ?? "No source"}</span>
      </div>
    </div>
  );
}
