import { For, Show, createSignal } from "solid-js";
import type { DmxOutputConfig, DmxOutputRouteTelemetry, SerialPortSummary } from "../types";

interface DmxOutputConfigPanelProps {
  output: DmxOutputConfig;
  routes: DmxOutputConfig[];
  routeStatuses: DmxOutputRouteTelemetry[];
  serialPorts: SerialPortSummary[];
  isSerialProtocol: (protocol: DmxOutputConfig["protocol"]) => boolean;
  onOutputChange: (output: DmxOutputConfig) => void;
  onProtocolChange: (protocol: DmxOutputConfig["protocol"]) => void;
  onRefreshSerialPorts: () => void | Promise<void>;
  onApply: () => void | Promise<void>;
  onAddCurrentRoute: () => void | Promise<void>;
  onApplyRoutes: () => void | Promise<void>;
  onRouteEnabled: (index: number, enabled: boolean) => void | Promise<void>;
  onRemoveRoute: (index: number) => void | Promise<void>;
}

const SACN_MAX_UNIVERSE = 63_999;
const ROUTES_PER_PAGE = 6;

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

const protocolLabel = (protocol: DmxOutputConfig["protocol"]) => {
  switch (protocol) {
    case "ArtNet": return "Art-Net";
    case "Sacn": return "sACN";
    case "EnttecUsbPro": return "Enttec USB PRO";
    case "DmxKingUltraDmx": return "DMXKing ultraDMX";
    case "EnttecOpenDmx": return "Enttec Open DMX";
  }
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
  const [routePage, setRoutePage] = createSignal(0);
  const selectedSerialPort = () => props.serialPorts.find((port) => port.name === props.output.serial_port);
  const routePageCount = () => Math.max(1, Math.ceil(props.routes.length / ROUTES_PER_PAGE));
  const normalizedRoutePage = () => Math.min(routePage(), routePageCount() - 1);
  const visibleRoutes = () => {
    const start = normalizedRoutePage() * ROUTES_PER_PAGE;
    return props.routes.slice(start, start + ROUTES_PER_PAGE).map((route, offset) => ({ route, index: start + offset }));
  };
  const routeTarget = (route: DmxOutputConfig) => props.isSerialProtocol(route.protocol)
    ? route.serial_port || "No serial port"
    : `${route.target_ip || "auto"}:${route.port}`;
  const routeStatus = (index: number, route: DmxOutputConfig) => {
    if (!route.enabled) return { tone: "idle", label: "Route disabled" };
    const runtime = props.routeStatuses.find((candidate) => candidate.index === index);
    if (!runtime) return { tone: "ready", label: "Route enabled" };
    if (runtime.success) return { tone: "ok", label: "Route active" };
    if (runtime.reconnecting) return { tone: "warn", label: "Route reconnecting" };
    if (runtime.attempted) return { tone: "bad", label: "Route failed" };
    return { tone: "ready", label: "Route enabled" };
  };
  const addNetworkRoute = async (protocol: "ArtNet" | "Sacn") => {
    props.onProtocolChange(protocol);
    await Promise.resolve();
    await props.onAddCurrentRoute();
  };

  return (
    <section class="dmxOutputConfigPanel ioConnectionDesk" data-io-default-surface="dmx">
      <header class="ioDeskHeader">
        <div>
          <h2>DMX Connections</h2>
          <span>Output routes and detected interfaces</span>
        </div>
        <span class={`ioConnectionState ${props.output.enabled ? "ready" : "idle"}`}>
          <i aria-hidden="true" />{props.output.enabled ? "Enabled" : "Disabled"}
        </span>
      </header>

      <div class="dmxRouteBuilder">
        <label class="checkbox dmxRouteEnable">
          <input
            data-io-control="dmx-output-enabled"
            type="checkbox"
            checked={props.output.enabled}
            onChange={(event) => props.onOutputChange({ ...props.output, enabled: event.currentTarget.checked })}
          />
          Output enabled
        </label>
        <label>
          Protocol
          <select
            data-io-control="dmx-protocol"
            value={props.output.protocol}
            onInput={(event) => props.onProtocolChange(event.currentTarget.value as DmxOutputConfig["protocol"])}
          >
            <option value="ArtNet">Art-Net</option>
            <option value="Sacn">sACN / E1.31</option>
            <option value="EnttecUsbPro">Enttec USB PRO</option>
            <option value="DmxKingUltraDmx">DMXKing ultraDMX</option>
            <option value="EnttecOpenDmx">Enttec Open DMX</option>
          </select>
        </label>
        <Show when={!props.isSerialProtocol(props.output.protocol)} fallback={
          <>
            <label class="dmxSerialPortSelect">
              Serial port
              <select
                data-io-control="dmx-serial-port"
                value={props.output.serial_port}
                onInput={(event) => props.onOutputChange({ ...props.output, serial_port: event.currentTarget.value })}
              >
                <option value="">Select port</option>
                <For each={props.serialPorts}>
                  {(port) => <option value={port.name}>{port.name} / {port.port_type}</option>}
                </For>
              </select>
            </label>
            <button data-io-control="dmx-scan-serial" type="button" onClick={props.onRefreshSerialPorts}>Scan Serial</button>
          </>
        }>
          <label>
            Target IP
            <input
              data-io-control="dmx-target"
              value={props.output.target_ip}
              placeholder={props.output.protocol === "Sacn" ? "multicast, auto, or unicast IP" : "127.0.0.1"}
              onInput={(event) => props.onOutputChange({ ...props.output, target_ip: event.currentTarget.value })}
            />
          </label>
          <label>
            Port
            <input
              data-io-control="dmx-port"
              type="number"
              value={props.output.port}
              onInput={(event) => props.onOutputChange({ ...props.output, port: Number(event.currentTarget.value) })}
            />
          </label>
        </Show>
        <label>
          Universe
          <input
            data-io-control="dmx-universe"
            type="number"
            min={props.output.protocol === "Sacn" ? "1" : "0"}
            value={props.output.universe}
            onInput={(event) => props.onOutputChange({ ...props.output, universe: Number(event.currentTarget.value) })}
          />
        </label>
        <button data-io-control="dmx-apply-output" class="primary" onClick={props.onApply}>Apply Output</button>
      </div>

      <div class="dmxRouteActions">
        <button data-io-control="dmx-add-artnet" onClick={() => void addNetworkRoute("ArtNet")}>Add Art-Net route</button>
        <button data-io-control="dmx-add-sacn" onClick={() => void addNetworkRoute("Sacn")}>Add sACN route</button>
        <button data-io-control="dmx-add-current" onClick={props.onAddCurrentRoute}>Add current route</button>
        <button data-io-control="dmx-apply-routes" onClick={props.onApplyRoutes}>Apply routes</button>
      </div>

      <div
        class="dmxRouteList"
        data-io-route-list
        data-io-route-total={props.routes.length}
        data-io-route-page-count={routePageCount()}
      >
        <div class="panelHeader">
          <h3>Routes</h3>
          <span>{props.routes.length}</span>
        </div>
        <div class="dmxRouteRows">
          <For each={visibleRoutes()}>
            {(entry) => {
              const status = () => routeStatus(entry.index, entry.route);
              return (
                <div class="dmxRouteRow" data-io-route-row data-route-index={entry.index}>
                  <span class={`ioStatusDot ${status().tone}`} title={status().label} aria-label={status().label} />
                  <strong>{protocolLabel(entry.route.protocol)}</strong>
                  <span data-io-route-target>{routeTarget(entry.route)}</span>
                  <span data-io-route-universe>U{entry.route.universe}</span>
                  <label class="checkbox" title="Enable route">
                    <input
                      data-io-control="dmx-route-enabled"
                      type="checkbox"
                      checked={entry.route.enabled}
                      onChange={(event) => void props.onRouteEnabled(entry.index, event.currentTarget.checked)}
                    />
                    <span class="srOnly">Enable route</span>
                  </label>
                  <button data-io-control="dmx-remove-route" onClick={() => void props.onRemoveRoute(entry.index)} disabled={props.routes.length <= 1}>
                    Remove
                  </button>
                </div>
              );
            }}
          </For>
        </div>
        <Show when={routePageCount() > 1}>
          <div class="dmxRoutePagination">
            <button
              aria-label="Previous route page"
              disabled={normalizedRoutePage() === 0}
              onClick={() => setRoutePage((current) => Math.max(0, current - 1))}
            >‹</button>
            <span>{normalizedRoutePage() + 1} / {routePageCount()}</span>
            <button
              aria-label="Next route page"
              disabled={normalizedRoutePage() >= routePageCount() - 1}
              onClick={() => setRoutePage((current) => Math.min(routePageCount() - 1, current + 1))}
            >›</button>
          </div>
        </Show>
      </div>

      <details class="ioDisclosure" data-io-disclosure="dmx-output-options">
        <summary>Output options</summary>
        <div class="ioDisclosureBody" data-io-disclosure-body>
          <Show when={props.output.protocol === "Sacn"}>
            <div class="dmxOutputNetworkHint">
              <span>sACN multicast</span>
              <strong>{sacnMulticastAddress(props.output.universe)}</strong>
              <button
                type="button"
                data-io-control="dmx-multicast"
                title="Use the standard sACN multicast target for the selected universe"
                disabled={isSacnMulticastTarget(props.output.target_ip)}
                onClick={() => props.onOutputChange({ ...props.output, target_ip: "multicast" })}
              >Multicast</button>
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
              <Show when={selectedSerialPort()}>
                {(port) => (
                  <div class="serialPortIdentity">
                    <div>
                      <strong data-no-localize>{port().product || port().port_type}</strong>
                      <span data-no-localize>{[port().manufacturer, port().usb_vid != null && port().usb_pid != null
                        ? `${port().usb_vid!.toString(16).padStart(4, "0")}:${port().usb_pid!.toString(16).padStart(4, "0")}`
                        : null, port().serial_number].filter(Boolean).join(" / ")}</span>
                    </div>
                    <Show when={port().recommended_protocol} fallback={<span class="serialProtocolManual">Manual protocol selection</span>}>
                      {(recommended) => <button type="button" data-io-control="dmx-recommended-protocol" onClick={() => props.onProtocolChange(recommended())}>Use recommended protocol</button>}
                    </Show>
                  </div>
                )}
              </Show>
              <div class="buttonRow">
                <button data-io-control="dmx-clear-port" onClick={() => props.onOutputChange({ ...props.output, serial_port: "" })}>Clear Port</button>
              </div>
              <label>
                Manual port
                <input
                  data-io-control="dmx-manual-port"
                  value={props.output.serial_port}
                  placeholder="COM3 or /dev/ttyUSB0"
                  onInput={(event) => props.onOutputChange({ ...props.output, serial_port: event.currentTarget.value })}
                />
              </label>
              <label>
                Baud rate
                <input
                  data-io-control="dmx-baud-rate"
                  type="number"
                  min="1"
                  disabled={props.output.protocol === "EnttecOpenDmx"}
                  value={props.output.serial_baud_rate}
                  onInput={(event) => props.onOutputChange({ ...props.output, serial_baud_rate: Number(event.currentTarget.value) })}
                />
              </label>
            </div>
          </Show>
        </div>
      </details>
    </section>
  );
}
