import type { EngineSnapshot, EngineTelemetryBudgetReport, Phase1SmokeReport } from "../types";
import { DmxTestFramePanel } from "./DmxTestFramePanel";
import { EngineTelemetryPanel } from "./EngineTelemetryPanel";
import { Phase1SmokeReportPanel } from "./Phase1SmokeReportPanel";

type MaybePromise = void | Promise<void>;

interface OutputDiagnosticsPanelProps {
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
  onResetTelemetry: () => MaybePromise;
  onSaveTelemetryReport: () => MaybePromise;
}

export function OutputDiagnosticsPanel(props: OutputDiagnosticsPanelProps) {
  return (
    <section class="outputDiagnosticsDesk">
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
