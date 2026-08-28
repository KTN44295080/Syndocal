import type { DmxControlMapping, DmxInputConfig, DmxInputStatus } from "../types";
import { controlMappingTargetLabel } from "../controlMappingLabels";

interface DmxInputPanelProps {
  config: DmxInputConfig;
  status: DmxInputStatus;
  mappings: DmxControlMapping[];
  onConfig: (config: DmxInputConfig) => void;
  onStart: () => void | Promise<void>;
  onStop: () => void | Promise<void>;
  onRemoveMapping: (index: number) => void;
}

export function DmxInputPanel(props: DmxInputPanelProps) {
  const setConfig = (patch: Partial<DmxInputConfig>) => props.onConfig({ ...props.config, ...patch });

  return (
    <div class="dmxInputPanel">
      <div class="panelHeader">
        <h3>DMX input and merge</h3>
        <span class={props.status.signal_present ? "pill ok" : props.status.running ? "pill warn" : "pill"}>
          {props.status.signal_present ? "Signal" : props.status.running ? "Waiting" : "Stopped"}
        </span>
      </div>
      <p class="ioDisclosureDescription">
        Choose one mode: raw merge sends incoming levels to the DMX output; control mappings turn selected channels into Syndocal controls.
      </p>
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
          Input mode
          <select
            data-io-control="dmx-input-use"
            value={props.config.merge_enabled ? "merge" : "control"}
            disabled={props.status.running}
            onInput={(event) => setConfig({ merge_enabled: event.currentTarget.value === "merge" })}
          >
            <option value="merge">Raw merge (HTP/LTP)</option>
            <option value="control">Control mappings</option>
          </select>
        </label>
        <label>
          Merge rule (raw merge only)
          <select
            data-io-control="dmx-input-merge-rule"
            value={props.config.merge_mode}
            disabled={props.status.running || !props.config.merge_enabled}
            onInput={(event) => setConfig({ merge_mode: event.currentTarget.value as DmxInputConfig["merge_mode"] })}
          >
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
      <div class="dmxControlMappings" data-dmx-control-mappings>
        <div class="panelHeader">
          <h4>DMX control mappings</h4>
          <span class="pill">{props.mappings.length}</span>
        </div>
        {props.mappings.length === 0 ? (
          <p class="muted">No DMX controls are mapped. Raw merge remains available above.</p>
        ) : (
          <div class="mappingList">
            {props.mappings.map((mapping, index) => (
              <div class="mappingRow" data-dmx-mapping-index={index}>
                <span>U{mapping.universe + 1} Ch {mapping.channel}</span>
                <span>{controlMappingTargetLabel(mapping)}</span>
                <span>{Math.round(mapping.low)}–{Math.round(mapping.high)}</span>
                <button disabled={props.status.running} onClick={() => props.onRemoveMapping(index)}>Remove</button>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
