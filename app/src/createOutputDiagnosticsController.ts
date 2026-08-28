import { createSignal, type Accessor, type Setter } from "solid-js";
import {
  executeOutputControl,
  queryOutputLeaseAuthority,
  selectOnlyActiveOutputLease,
} from "./outputControlController";
import type { FrontendTauriInvoke } from "./tauriInvokeCommands";
import type {
  DmxOutputConfig,
  EngineSnapshot,
  EngineTelemetryReport,
  SerialDmxMachineBindingStatus,
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
  const [serialDmxMachineBinding, setSerialDmxMachineBinding] = createSignal<SerialDmxMachineBindingStatus | null>(null);

  const refreshSerialDmxMachineBinding = async () => {
    try {
      setSerialDmxMachineBinding(await options.invoke<SerialDmxMachineBindingStatus>(
        "get_serial_dmx_machine_binding_status_v1",
      ));
    } catch (error) {
      setSerialDmxMachineBinding(null);
      options.setMessage(String(error));
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

  const enableStagedShowSerialDmxRoute = async () => {
    try {
      const lease = selectOnlyActiveOutputLease(
        await queryOutputLeaseAuthority(options.invoke),
        ["lighting", "video"],
      );
      await executeOutputControl(options.invoke, {
        kind: "enable_show_serial_dmx_route",
        lease,
      });
      await options.refreshSnapshot();
      options.setMessage("Staged machine-local USB-DMX show route enabled.");
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
      await refreshSerialDmxMachineBinding();
    } catch (error) {
      options.setMessage(String(error));
    }
  };

  const selectSerialDmxMachineBinding = async (port: SerialPortSummary) => {
    try {
      const instance = port.windows_device_instance_id?.trim();
      if (!instance) throw new Error("Selected USB-DMX interface does not expose a Windows PnP instance.");
      setSerialDmxMachineBinding(await options.invoke<SerialDmxMachineBindingStatus>(
        "select_serial_dmx_machine_binding_v1",
        { request: { portName: port.name, windowsDeviceInstanceId: instance } },
      ));
      options.setMessage(`Confirmed machine-local USB-DMX interface ${port.name}.`);
    } catch (error) {
      options.setMessage(String(error));
      await refreshSerialDmxMachineBinding();
    }
  };

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
    serialDmxMachineBinding,
    setDmxTestValue,
    refreshEngineTelemetryReport,
    resetEngineTelemetry,
    saveEngineTelemetryReport,
    applyOutput,
    enableStagedShowSerialDmxRoute,
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
    refreshSerialDmxMachineBinding,
    selectSerialDmxMachineBinding,
  };
}
