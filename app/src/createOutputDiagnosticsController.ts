import { createSignal, type Accessor, type Setter } from "solid-js";
import {
  executeOutputControl,
  OUTPUT_DSF2026_ARTNET_ACCEPTANCE_PROBE_STATUS_QUERY_OPERATION_ID,
  queryDsf2026ArtNetAcceptanceProbeStatus,
  queryOutputLeaseAuthority,
  selectOnlyActiveOutputLease,
} from "./outputControlController";
import type { Dsf2026ArtNetAcceptanceProbeStatusQuery } from "./outputControlController";
import type { FrontendTauriInvoke } from "./tauriInvokeCommands";
import type {
  DmxOutputConfig,
  EngineSnapshot,
  EngineTelemetryReport,
  SerialPortSummary,
} from "./types";

type Invoke = FrontendTauriInvoke;

export const defaultOutput: DmxOutputConfig = {
  enabled: true,
  protocol: "ArtNet",
  target_ip: "127.0.0.1",
  port: 6454,
  universe: 0,
  serial_port: "",
  serial_baud_rate: 57_600,
};

const enttecUsbProBaudRate = 57_600;
const enttecOpenDmxBaudRate = 250_000;

export const isSerialDmxProtocol = (protocol: DmxOutputConfig["protocol"]) =>
  protocol === "EnttecUsbPro" || protocol === "DmxKingUltraDmx" || protocol === "EnttecOpenDmx";

export const outputProtocolLabel = (protocol: DmxOutputConfig["protocol"]) => {
  switch (protocol) {
    case "ArtNet": return "Art-Net";
    case "Sacn": return "sACN";
    case "EnttecUsbPro": return "Enttec USB PRO";
    case "DmxKingUltraDmx": return "DMXKing ultraDMX";
    case "EnttecOpenDmx": return "Enttec Open DMX";
  }
};

const outputWithProtocol = (
  current: DmxOutputConfig,
  protocol: DmxOutputConfig["protocol"],
  serialPorts: SerialPortSummary[],
): DmxOutputConfig => {
  const port = protocol === "Sacn" && current.port === 6454
    ? 5568
    : protocol === "ArtNet" && current.port === 5568 ? 6454 : current.port;
  const universe = protocol === "Sacn" && current.universe === 0 ? 1 : current.universe;
  const target_ip = protocol === "Sacn" && (current.target_ip === defaultOutput.target_ip || current.target_ip.trim() === "")
    ? "multicast"
    : protocol === "ArtNet" && current.target_ip === "multicast" ? defaultOutput.target_ip : current.target_ip;
  const serial_port = isSerialDmxProtocol(protocol) && !current.serial_port && serialPorts.length > 0
    ? serialPorts[0].name
    : current.serial_port;
  const serial_baud_rate = protocol === "EnttecOpenDmx"
    ? enttecOpenDmxBaudRate
    : (protocol === "EnttecUsbPro" || protocol === "DmxKingUltraDmx") &&
        current.serial_baud_rate === enttecOpenDmxBaudRate
      ? enttecUsbProBaudRate
      : current.serial_baud_rate;
  return { ...current, protocol, target_ip, port, universe, serial_port, serial_baud_rate };
};

interface OutputDiagnosticsControllerOptions {
  invoke: Invoke;
  setMessage: (message: string) => unknown;
  refreshSnapshot: () => Promise<EngineSnapshot | null>;
  serialPorts: Accessor<SerialPortSummary[]>;
  setSerialPorts: Setter<SerialPortSummary[]>;
}

export function createOutputDiagnosticsController(options: OutputDiagnosticsControllerOptions) {
  const [output, setOutput] = createSignal<DmxOutputConfig>(defaultOutput);
  const [dmxOutputRoutes, setDmxOutputRoutes] = createSignal<DmxOutputConfig[]>([defaultOutput]);
  const [engineTelemetryReport, setEngineTelemetryReport] = createSignal<EngineTelemetryReport | null>(null);
  const [dmxTestChannel, setDmxTestChannel] = createSignal(1);
  const [dmxTestWidth, setDmxTestWidth] = createSignal(1);
  const [dmxTestValue, setDmxTestValue] = createSignal(255);
  // Unknown remains disabled in the panel.  The fixed route must never become
  // clickable merely because the status refresh has not completed yet.
  const [dsf2026ArtNetAcceptanceProbeStatus, setDsf2026ArtNetAcceptanceProbeStatus] =
    createSignal<Dsf2026ArtNetAcceptanceProbeStatusQuery | null>(null);
  const refreshDsf2026ArtNetAcceptanceProbeStatus = async () => {
    try {
      const status = await queryDsf2026ArtNetAcceptanceProbeStatus(options.invoke);
      setDsf2026ArtNetAcceptanceProbeStatus(status);
      return status;
    } catch {
      setDsf2026ArtNetAcceptanceProbeStatus(null);
      return null;
    }
  };
  const refreshEngineTelemetryReport = async () => {
    try {
      setEngineTelemetryReport(await options.invoke<EngineTelemetryReport>("get_engine_telemetry_report"));
    } catch (error) {
      options.setMessage(String(error));
    }
  };

  const resetEngineTelemetry = async () => {
    try {
      await options.invoke("reset_engine_telemetry");
      options.setMessage("Reset engine telemetry.");
      await options.refreshSnapshot();
      await refreshEngineTelemetryReport();
    } catch (error) {
      options.setMessage(String(error));
    }
  };

  const saveEngineTelemetryReport = async () => {
    try {
      const path = await options.invoke<string | null>("save_engine_telemetry_report");
      options.setMessage(path ? `Saved telemetry report ${path}` : "Telemetry report save canceled.");
      await refreshEngineTelemetryReport();
    } catch (error) {
      options.setMessage(String(error));
    }
  };

  const applyOutput = async () => {
    options.setMessage(
      "DMX output configuration is unavailable until a lease-bound OutputControl action is reviewed; no state changed.",
    );
  };

  const enableStagedShowArtNetLoopbackRoute = async () => {
    try {
      const lease = selectOnlyActiveOutputLease(
        await queryOutputLeaseAuthority(options.invoke),
        ["lighting", "video"],
      );
      await executeOutputControl(options.invoke, {
        kind: "enable_show_artnet_loopback_route",
        lease,
      });
      await options.refreshSnapshot();
      options.setMessage("Staged same-PC Art-Net loopback show route enabled.");
    } catch (error) {
      options.setMessage(String(error));
    }
  };

  const sendDsf2026ArtNetAcceptanceProbe = async () => {
    try {
      const status = await refreshDsf2026ArtNetAcceptanceProbeStatus();
      if (status?.status !== "available") {
        throw new Error("DSF2026 fixed probe is unavailable: its durable one-shot status was not available for a new send.");
      }
      const lease = selectOnlyActiveOutputLease(
        await queryOutputLeaseAuthority(options.invoke),
        ["lighting", "video"],
      );
      await executeOutputControl(options.invoke, {
        kind: "send_dsf2026_artnet_acceptance_probe",
        lease,
      });
      await options.refreshSnapshot();
      setDsf2026ArtNetAcceptanceProbeStatus({
        operationId: OUTPUT_DSF2026_ARTNET_ACCEPTANCE_PROBE_STATUS_QUERY_OPERATION_ID,
        status: "consumed",
      });
      options.setMessage("DSF2026 fixed red probe: OS accepted one 530-byte ArtDmx U0 datagram to 127.0.0.1:6454; receiver and physical output remain unverified. A second probe is permanently prohibited, including after restart.");
    } catch (error) {
      options.setMessage(String(error));
    }
  };

  const acknowledgeDsf2026ArtNetAcceptanceProbeInDoubt = async () => {
    try {
      const status = await refreshDsf2026ArtNetAcceptanceProbeStatus();
      if (status?.status !== "in_doubt") {
        throw new Error("DSF2026 probe reconciliation is unavailable: there is no durable InDoubt outcome to resolve.");
      }
      const lease = selectOnlyActiveOutputLease(
        await queryOutputLeaseAuthority(options.invoke),
        ["lighting", "video"],
      );
      await executeOutputControl(options.invoke, {
        kind: "acknowledge_dsf2026_artnet_acceptance_probe_in_doubt",
        lease,
      });
      await options.refreshSnapshot();
      setDsf2026ArtNetAcceptanceProbeStatus({
        operationId: OUTPUT_DSF2026_ARTNET_ACCEPTANCE_PROBE_STATUS_QUERY_OPERATION_ID,
        status: "consumed",
      });
      options.setMessage("DSF2026 probe InDoubt hold reconciled without sending Art-Net. Receiver and physical output were independently verified by the operator. A fresh probe remains permanently prohibited.");
    } catch (error) {
      options.setMessage(String(error));
    }
  };

  const enableShowSpoutOutputs = async () => {
    try {
      const lease = selectOnlyActiveOutputLease(
        await queryOutputLeaseAuthority(options.invoke),
        ["lighting", "video"],
      );
      await executeOutputControl(options.invoke, {
        kind: "enable_show_spout_outputs",
        lease,
      });
      await options.refreshSnapshot();
      options.setMessage("Same-PC Syndocal Background/Foreground Spout outputs enabled.");
    } catch (error) {
      options.setMessage(String(error));
    }
  };

  const sendDmxTestFrame = async () => {
    options.setMessage(
      "DMX test output is unavailable until a lease-bound OutputControl action is reviewed; no frame was sent.",
    );
  };

  const sendDmxRoutesTestFrame = async () => {
    options.setMessage(
      "DMX route test output is unavailable until a lease-bound OutputControl action is reviewed; no frames were sent.",
    );
  };

  const dmxRouteLabel = (route: DmxOutputConfig) => isSerialDmxProtocol(route.protocol)
    ? `${outputProtocolLabel(route.protocol)} ${route.serial_port || "(no port)"}`
    : `${outputProtocolLabel(route.protocol)} ${route.target_ip}:${route.port} U${route.universe}`;

  const applyDmxOutputRoutes = async (routes: DmxOutputConfig[]) => {
    void routes;
    options.setMessage(
      "DMX output routes are unavailable until a lease-bound OutputControl action is reviewed; no state changed.",
    );
  };

  const applyCurrentDmxRoutes = async () => {
    await applyDmxOutputRoutes([output(), ...dmxOutputRoutes().slice(1)]);
  };

  const addCurrentDmxRoute = async () => {
    await applyDmxOutputRoutes([...dmxOutputRoutes(), output()]);
  };

  const addDmxNetworkRoute = async (protocol: "ArtNet" | "Sacn") => {
    const candidate = outputWithProtocol(output(), protocol, options.serialPorts());
    await applyDmxOutputRoutes([...dmxOutputRoutes(), candidate]);
  };

  const removeDmxRoute = async (index: number) => {
    const routes = dmxOutputRoutes().filter((_, candidate) => candidate !== index);
    await applyDmxOutputRoutes(routes.length > 0 ? routes : [{ ...defaultOutput, enabled: false }]);
  };

  const setDmxRouteEnabled = async (index: number, enabled: boolean) => {
    const routes = [output(), ...dmxOutputRoutes().slice(1)].map((route, candidate) =>
      candidate === index ? { ...route, enabled } : route
    );
    await applyDmxOutputRoutes(routes);
  };

  const setOutputProtocol = (protocol: DmxOutputConfig["protocol"]) => {
    setOutput((current) => outputWithProtocol(current, protocol, options.serialPorts()));
  };

  const refreshSerialPorts = async () => {
    try {
      const ports = await options.invoke<SerialPortSummary[]>("list_serial_ports");
      options.setSerialPorts(ports);
    } catch (error) {
      options.setMessage(String(error));
    }
  };

  void refreshDsf2026ArtNetAcceptanceProbeStatus();

  return {
    output,
    setOutput,
    dmxOutputRoutes,
    setDmxOutputRoutes,
    engineTelemetryReport,
    dmxTestChannel,
    setDmxTestChannel,
    dmxTestWidth,
    setDmxTestWidth,
    dmxTestValue,
    setDmxTestValue,
    dsf2026ArtNetAcceptanceProbeStatus,
    refreshEngineTelemetryReport,
    resetEngineTelemetry,
    saveEngineTelemetryReport,
    applyOutput,
    enableStagedShowArtNetLoopbackRoute,
    sendDsf2026ArtNetAcceptanceProbe,
    acknowledgeDsf2026ArtNetAcceptanceProbeInDoubt,
    enableShowSpoutOutputs,
    sendDmxTestFrame,
    sendDmxRoutesTestFrame,
    dmxRouteLabel,
    applyCurrentDmxRoutes,
    addCurrentDmxRoute,
    addDmxNetworkRoute,
    removeDmxRoute,
    setDmxRouteEnabled,
    setOutputProtocol,
    refreshSerialPorts,
  };
}
