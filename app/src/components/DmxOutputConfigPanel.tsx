import type { DmxOutputConfig } from "../types";
import type { Dsf2026ArtNetAcceptanceProbeStatusQuery } from "../outputControlController";

interface DmxOutputConfigPanelProps {
  output: DmxOutputConfig;
  dsf2026ArtNetAcceptanceProbeStatus: () => Dsf2026ArtNetAcceptanceProbeStatusQuery | null;
  onEnableStagedShowArtNetLoopbackRoute: () => void | Promise<void>;
  onSendDsf2026ArtNetAcceptanceProbe: () => void | Promise<void>;
  onAcknowledgeDsf2026ArtNetAcceptanceProbeInDoubt: () => void | Promise<void>;
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
  const probeStatus = () => props.dsf2026ArtNetAcceptanceProbeStatus();
  const probeStatusReason = () => {
    switch (probeStatus()?.status) {
      case "available":
        return "DSF2026 fixed probe is available once, only while the exact route remains staged disabled.";
      case "in_doubt":
        return "DSF2026 probe outcome is InDoubt. Reconcile without sending after independent receiver and physical-output verification; it records the unobservable result as permanently consumed and never re-enables retry or a new probe.";
      case "consumed":
        return "DSF2026 fixed red probe is permanently consumed. A second probe is prohibited, including after restart or reconciliation.";
      default:
        return "DSF2026 probe status is loading; no probe can be sent.";
    }
  };
  const probeSendDisabled = () =>
    !exactRoute() || props.output.enabled || probeStatus()?.status !== "available";
  const probeReconcileDisabled = () =>
    !exactRoute() || props.output.enabled || probeStatus()?.status !== "in_doubt";
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
        <button
          data-io-control="dmx-send-dsf2026-artnet-acceptance-probe"
          class="danger"
          disabled={probeSendDisabled()}
          aria-describedby="dmx-dsf2026-artnet-acceptance-probe"
          onClick={() => void props.onSendDsf2026ArtNetAcceptanceProbe()}
        >Send fixed red DSF2026 Art-Net probe once</button>
        <button
          data-io-control="dmx-acknowledge-dsf2026-artnet-acceptance-probe-in-doubt"
          class="danger"
          disabled={probeReconcileDisabled()}
          aria-describedby="dmx-dsf2026-artnet-acceptance-probe"
          onClick={() => void props.onAcknowledgeDsf2026ArtNetAcceptanceProbeInDoubt()}
        >Reconcile DSF2026 probe InDoubt (no send)</button>
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
      <p id="dmx-dsf2026-artnet-acceptance-probe" class="ioDisclosureDescription">
        One-shot fixed red 530-byte ArtDmx U0 proof only to 127.0.0.1:6454: payload[0] and payload[4] are 255; payload[499] remains 0. The authored route stays staged disabled. OS UDP acceptance only; receiver and physical output remain unverified.
      </p>
      <p class="ioDisclosureDescription" role="status">
        {probeStatusReason()}
      </p>
      <p class="ioDisclosureDescription">
        Reconcile is a separate no-send action after independent receiver and physical-output verification; it records the unobservable result as permanently consumed and never re-enables retry or another fixed probe.
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
