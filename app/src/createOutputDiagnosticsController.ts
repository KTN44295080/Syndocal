import { createSignal, type Accessor, type Setter } from "solid-js";
import type {
  DmxOutputConfig,
  DmxTestFrameResult,
  EngineSnapshot,
  EngineTelemetryReport,
  SerialPortSummary,
} from "./types";

type Invoke = <T>(command: string, args?: Record<string, unknown>) => Promise<T>;

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
    try {
      await options.invoke("set_output_config", { config: output() });
      setDmxOutputRoutes((current) => [output(), ...current.slice(1)]);
      if (isSerialDmxProtocol(output().protocol)) {
        options.setMessage(`${outputProtocolLabel(output().protocol)} target ${output().serial_port} @ ${output().serial_baud_rate}`);
      } else {
        options.setMessage(
          `${outputProtocolLabel(output().protocol)} target ${output().target_ip}:${output().port} universe ${output().universe}`,
        );
      }
      await options.refreshSnapshot();
    } catch (error) {
      options.setMessage(String(error));
    }
  };

  const sendDmxTestFrame = async () => {
    try {
      const result = await options.invoke<DmxTestFrameResult>("send_dmx_test_frame", {
        request: {
          config: output(),
          channel: dmxTestChannel(),
          width: dmxTestWidth(),
          value: dmxTestValue(),
        },
      });
      options.setMessage(
        `Sent ${outputProtocolLabel(result.protocol)} test U${result.universe} CH${result.channel} +${result.width} @ ${result.value} (${result.bytes} bytes)`,
      );
    } catch (error) {
      options.setMessage(String(error));
    }
  };

  const sendDmxRoutesTestFrame = async () => {
    const routes = [output(), ...dmxOutputRoutes().slice(1)];
    try {
      const results = await options.invoke<DmxTestFrameResult[]>("send_dmx_routes_test_frame", {
        request: {
          configs: routes,
          channel: dmxTestChannel(),
          width: dmxTestWidth(),
          value: dmxTestValue(),
        },
      });
      const bytes = results.reduce((sum, result) => sum + result.bytes, 0);
      options.setMessage(`Sent test frame to ${results.length} DMX route(s) (${bytes} bytes).`);
    } catch (error) {
      options.setMessage(String(error));
    }
  };

  const dmxRouteLabel = (route: DmxOutputConfig) => isSerialDmxProtocol(route.protocol)
    ? `${outputProtocolLabel(route.protocol)} ${route.serial_port || "(no port)"}`
    : `${outputProtocolLabel(route.protocol)} ${route.target_ip}:${route.port} U${route.universe}`;

  const applyDmxOutputRoutes = async (routes: DmxOutputConfig[]) => {
    try {
      await options.invoke("set_dmx_outputs", { configs: routes });
      setDmxOutputRoutes(routes);
      setOutput(routes[0] ?? defaultOutput);
      options.setMessage(`Applied ${routes.length} DMX output route(s).`);
      await options.refreshSnapshot();
    } catch (error) {
      options.setMessage(String(error));
    }
  };

  const applyCurrentDmxRoutes = async () => {
    await applyDmxOutputRoutes([output(), ...dmxOutputRoutes().slice(1)]);
  };

  const addCurrentDmxRoute = async () => {
    await applyDmxOutputRoutes([...dmxOutputRoutes(), output()]);
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
    const current = output();
    const port = protocol === "Sacn" && current.port === 6454
      ? 5568
      : protocol === "ArtNet" && current.port === 5568 ? 6454 : current.port;
    const universe = protocol === "Sacn" && current.universe === 0 ? 1 : current.universe;
    const target_ip = protocol === "Sacn" && (current.target_ip === defaultOutput.target_ip || current.target_ip.trim() === "")
      ? "multicast"
      : protocol === "ArtNet" && current.target_ip === "multicast" ? defaultOutput.target_ip : current.target_ip;
    const serial_port = isSerialDmxProtocol(protocol) && !current.serial_port && options.serialPorts().length > 0
      ? options.serialPorts()[0].name
      : current.serial_port;
    const serial_baud_rate = protocol === "EnttecOpenDmx"
      ? enttecOpenDmxBaudRate
      : (protocol === "EnttecUsbPro" || protocol === "DmxKingUltraDmx") &&
          current.serial_baud_rate === enttecOpenDmxBaudRate
        ? enttecUsbProBaudRate
        : current.serial_baud_rate;
    setOutput({ ...current, protocol, target_ip, port, universe, serial_port, serial_baud_rate });
  };

  const refreshSerialPorts = async () => {
    try {
      const ports = await options.invoke<SerialPortSummary[]>("list_serial_ports");
      options.setSerialPorts(ports);
      if (!output().serial_port && ports.length > 0) {
        setOutput((current) => ({ ...current, serial_port: ports[0].name }));
      }
    } catch (error) {
      options.setMessage(String(error));
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
    setDmxTestValue,
    refreshEngineTelemetryReport,
    resetEngineTelemetry,
    saveEngineTelemetryReport,
    applyOutput,
    sendDmxTestFrame,
    sendDmxRoutesTestFrame,
    dmxRouteLabel,
    applyCurrentDmxRoutes,
    addCurrentDmxRoute,
    removeDmxRoute,
    setDmxRouteEnabled,
    setOutputProtocol,
    refreshSerialPorts,
  };
}
