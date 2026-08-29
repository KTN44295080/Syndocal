import type { DmxOutputConfig } from "../types";

interface DmxOutputConfigPanelProps {
  output: DmxOutputConfig;
  onEnableStagedShowArtNetLoopbackRoute: () => void | Promise<void>;
}

const isExactShowArtNetLoopbackRoute = (output: DmxOutputConfig) =>
  output.protocol === "ArtNet"
  && output.target_ip === "127.0.0.1"
  && output.port === 6454
  && output.universe === 0
  && output.serial_port === "";

/**
 * The show route is deliberately not an editor: Unity shares this machine's
 * Art-Net socket, so this action admits exactly one fixed local ArtDmx route.
 */
export function DmxOutputConfigPanel(props: DmxOutputConfigPanelProps) {
  const exactRoute = () => isExactShowArtNetLoopbackRoute(props.output);
  const routeState = () => !exactRoute()
    ? "Logical route mismatch"
    : props.output.enabled ? "Enabled" : "Staged disabled";
  const routeStateTone = () => !exactRoute() ? "error" : props.output.enabled ? "ready" : "idle";

  return (
    <section class="dmxOutputConfigPanel ioConnectionDesk" data-io-default-surface="dmx">
      <header class="ioDeskHeader">
        <div><h2>DMX Connections</h2><span>Same-PC production show route</span></div>
        <span class={`ioConnectionState ${routeStateTone()}`}><i aria-hidden="true" />{routeState()}</span>
      </header>

      <div class="dmxRouteBuilder dmxPrimaryControls">
        <button
          data-io-control="dmx-enable-staged-show-artnet-loopback-route"
          class="primary"
          disabled={!exactRoute() || props.output.enabled}
          aria-describedby="dmx-show-route-confirmation"
          onClick={() => void props.onEnableStagedShowArtNetLoopbackRoute()}
        >Confirm and enable Art-Net loopback</button>
      </div>

      <div class="dmxRouteList" data-io-route-list data-io-route-total="1" data-io-route-page-count="1">
        <div class="panelHeader"><h3>Authored logical route</h3><span>1</span></div>
        <div class="dmxRouteRows"><div class="dmxRouteRow" data-io-route-row data-route-index="0">
          <span class={`ioStatusDot ${routeStateTone() === "ready" ? "ok" : routeStateTone()}`} aria-hidden="true" />
          <strong>Art-Net · ArtDmx</strong><span data-io-route-target>127.0.0.1:6454</span>
          <span data-io-route-universe>Wire U0 · 512ch · 40–44fps</span>
        </div></div>
      </div>

      <p id="dmx-show-route-confirmation" class="ioDisclosureDescription">
        Same-PC only: completed DMX Universe 1 is emitted unchanged as ArtDmx wire Universe 0. DMX ch1 maps to payload[0]; unused ch500 is forced to 0.
      </p>
      {!exactRoute() && <p class="ioDisclosureDescription" role="alert">
        The show route must remain Art-Net / 127.0.0.1:6454 / wire U0 with no serial interface. It is intentionally not configurable from this control.
      </p>}
      <p class="ioDisclosureDescription">
        Confirmation retains the native lease, safety-blackout, exact route, sender-open, acknowledgement, and rollback fences. USB serial DMX is not a show-route fallback.
      </p>
    </section>
  );
}
