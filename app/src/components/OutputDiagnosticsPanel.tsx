import type { DmxOutputConfig, EngineSnapshot, EngineTelemetryBudgetReport, Phase1SmokeReport } from "../types";
import type { Dsf2026ArtNetAcceptanceProbeStatusQuery } from "../outputControlController";
import { DmxTestFramePanel } from "./DmxTestFramePanel";
import { EngineTelemetryPanel } from "./EngineTelemetryPanel";
import { Phase1SmokeReportPanel } from "./Phase1SmokeReportPanel";

type MaybePromise = void | Promise<void>;

interface OutputDiagnosticsPanelProps {
  output: DmxOutputConfig;
  dsf2026ArtNetAcceptanceProbeStatus: () => Dsf2026ArtNetAcceptanceProbeStatusQuery | null;
  protocolLabel: string;
  testChannel: number;
  testWidth: number;
  testValue: number;
  telemetry: EngineSnapshot["telemetry"];
  telemetryBudget?: EngineTelemetryBudgetReport | null;
  phase1SmokeReport: Phase1SmokeReport | null;
  onTestChannel: (value: number) => void;
  onTestWidth: (value: number) => void;
  onTestValue: (value: number) => void;
  onSendTest: () => MaybePromise;
  onSendRoutes: () => MaybePromise;
  onSendDsf2026ArtNetAcceptanceProbe: () => MaybePromise;
  onAcknowledgeDsf2026ArtNetAcceptanceProbeInDoubt: () => MaybePromise;
  onResetTelemetry: () => MaybePromise;
  onSaveTelemetryReport: () => MaybePromise;
}

const isExactShowArtNetLoopbackRoute = (output: DmxOutputConfig) =>
  output.protocol === "ArtNet"
  && output.target_ip === "127.0.0.1"
  && output.port === 6454
  && output.universe === 0
  && output.serial_port === "";

const probeStatusReason = (status: Dsf2026ArtNetAcceptanceProbeStatusQuery | null) => {
  switch (status?.status) {
    case "available": return "Available · one send";
    case "in_doubt": return "In doubt · reconcile only";
    case "consumed": return "Consumed · no repeat";
    default: return "Loading";
  }
};

export function OutputDiagnosticsPanel(props: OutputDiagnosticsPanelProps) {
  const probeStatus = () => props.dsf2026ArtNetAcceptanceProbeStatus();
  const probeSendDisabled = () =>
    !isExactShowArtNetLoopbackRoute(props.output)
    || props.output.enabled
    || probeStatus()?.status !== "available";
  const probeReconcileDisabled = () =>
    !isExactShowArtNetLoopbackRoute(props.output)
    || props.output.enabled
    || probeStatus()?.status !== "in_doubt";

  return (
    <section class="outputDiagnosticsDesk">
      <section class="dmxProtocolDiagnostics" data-io-dmx-protocol-diagnostics>
        <div class="panelHeader">
          <h3>Protocol diagnostics</h3>
          <span id="dmx-dsf2026-artnet-acceptance-probe" role="status">{probeStatusReason(probeStatus())}</span>
        </div>
        <div class="dmxRouteActions">
          <button
            data-io-control="dmx-send-dsf2026-artnet-acceptance-probe"
            class="danger"
            disabled={probeSendDisabled()}
            aria-describedby="dmx-dsf2026-artnet-acceptance-probe"
            onClick={() => void props.onSendDsf2026ArtNetAcceptanceProbe()}
          >Send fixed probe</button>
          <button
            data-io-control="dmx-acknowledge-dsf2026-artnet-acceptance-probe-in-doubt"
            class="danger"
            disabled={probeReconcileDisabled()}
            aria-describedby="dmx-dsf2026-artnet-acceptance-probe"
            onClick={() => void props.onAcknowledgeDsf2026ArtNetAcceptanceProbeInDoubt()}
          >Reconcile probe (no send)</button>
        </div>
      </section>
      <DmxTestFramePanel
        protocolLabel={props.protocolLabel}
        channel={props.testChannel}
        width={props.testWidth}
        value={props.testValue}
        onChannelChange={props.onTestChannel}
        onWidthChange={props.onTestWidth}
        onValueChange={props.onTestValue}
        onSendTest={props.onSendTest}
        onSendRoutes={props.onSendRoutes}
      />
      <Phase1SmokeReportPanel report={props.phase1SmokeReport} />
      <EngineTelemetryPanel
        telemetry={props.telemetry}
        budget={props.telemetryBudget ?? null}
        onReset={props.onResetTelemetry}
        onSaveReport={props.onSaveTelemetryReport}
      />
    </section>
  );
}
