import { createSignal, type Accessor, type Setter } from "solid-js";
import {
  executeOutputControl,
  enableOutput,
  OUTPUT_DSF2026_ARTNET_ACCEPTANCE_PROBE_STATUS_QUERY_OPERATION_ID,
  hasOnlyActiveOutputLease,
  queryDsf2026ArtNetAcceptanceProbeStatus,
  queryOutputLeaseAuthority,
  selectOnlyActiveOutputLease,
} from "./outputControlController";
import type { Dsf2026ArtNetAcceptanceProbeStatusQuery } from "./outputControlController";
import type { FrontendTauriInvoke } from "./tauriInvokeCommands";
import { createSerialDmxStatusPoller } from "./serialDmxStatusPoller";
import { createSafetyBlackoutRuntimeController } from "./safetyBlackoutRuntimeController";
import {
  parseSerialDmxRouteStatusEventRevision,
  validateSerialDmxStatusSnapshot,
} from "./serialDmxStatusValidation";
import type {
  DmxOutputConfig,
  EngineSnapshot,
  EngineTelemetryReport,
  SerialDmxMachineBindingStatus,
  SerialPortSummary,
  ShowSerialDmxSafetyBlackoutRouteStatus,
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

const isExactShowArtNetLoopbackRoute = (output: DmxOutputConfig) =>
  output.protocol === "ArtNet"
  && output.target_ip === "127.0.0.1"
  && output.port === 6454
  && output.universe === 0
  && output.serial_port === "";

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
  safetyBlackout: Accessor<boolean>;
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
  const [serialDmxMachineBindingStatus, setSerialDmxMachineBindingStatus] =
    createSignal<SerialDmxMachineBindingStatus | null>(null);
  const [showSerialDmxSafetyBlackoutRouteStatus, setShowSerialDmxSafetyBlackoutRouteStatus] =
    createSignal<ShowSerialDmxSafetyBlackoutRouteStatus | null>(null);
  const [showDmxPreparationBusy, setShowDmxPreparationBusy] = createSignal(false);
  const [showDmxPreparationStage, setShowDmxPreparationStage] = createSignal<string | null>(null);
  let showDmxPreparationPromise: Promise<void> | null = null;
  // Reuse the canonical safer-direction S0 command. This controller never
  // invokes the release operation, so a failed preparation cannot clear S0.
  const safetyBlackoutRuntime = createSafetyBlackoutRuntimeController({
    invoke: options.invoke,
  });
  const serialDmxStatusPoller = createSerialDmxStatusPoller({
    queryBinding: () => options.invoke<SerialDmxMachineBindingStatus>(
      "get_serial_dmx_machine_binding_status_v1",
    ),
    queryRoute: () => options.invoke<ShowSerialDmxSafetyBlackoutRouteStatus>(
      "get_show_serial_dmx_safety_blackout_route_status_v1",
    ),
    commit: ({ binding, route }) => {
      setSerialDmxMachineBindingStatus(binding);
      setShowSerialDmxSafetyBlackoutRouteStatus(route);
    },
    validateSnapshot: validateSerialDmxStatusSnapshot,
    coherentRouteStatusRevision: (binding, route) =>
      binding.routeStatusRevision === route.routeStatusRevision
        ? route.routeStatusRevision
        : null,
    requireRouteStatusEventFence: true,
    onInvalidSnapshot: (reason) => {
      // The validator deliberately supplies field names only, never raw COM,
      // PnP, serial, or USB identity values. The panel remains Unknown while
      // this diagnostic is available to the local developer/operator log.
      console.warn(`USB-DMX status payload rejected: ${reason}`);
    },
  });
  const refreshSerialDmxRuntimeStatuses = (): Promise<void> => serialDmxStatusPoller.refresh();
  const setSerialDmxRouteStatusEventFenceAvailable = (available: boolean) => {
    serialDmxStatusPoller.setRouteStatusEventFenceAvailable(available);
    if (available) void refreshSerialDmxRuntimeStatuses();
  };
  const handleSerialDmxRouteStatusEvent = (payload: unknown) => {
    const revision = parseSerialDmxRouteStatusEventRevision(payload);
    if (!revision) {
      // An event is not allowed to make an unknown state look current. Do not
      // include raw payload text in the log because it may contain hardware
      // identifiers in a malformed/native regression case.
      serialDmxStatusPoller.invalidate();
      console.warn("USB-DMX status event rejected: malformed route revision fence");
    } else if (!serialDmxStatusPoller.invalidateAtOrAfterRouteStatusRevision(revision)) {
      console.warn("USB-DMX status event rejected: invalid route revision fence");
    }
    // This is intentionally independent of telemetry. A currently pending
    // uncancellable raw pair stays retained, but the visible state is already
    // Unknown and the next eligible status pair must meet this revision fence.
    void refreshSerialDmxRuntimeStatuses();
  };
  // Invalidate before and after a stateful action. A Tauri invoke may resolve
  // after its timeout, so only this fresh generation can update the panel.
  const refreshAuthoritativeSerialDmxRuntimeStatuses = async () => {
    serialDmxStatusPoller.invalidate();
    await refreshSerialDmxRuntimeStatuses();
  };
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

  const selectBothOutputLease = async () => selectOnlyActiveOutputLease(
    await queryOutputLeaseAuthority(options.invoke),
    ["lighting", "video"],
  );

  /**
   * A successful Enable can install the managed Both keepalive and therefore
   * advance the lease generation after the stage-1 receipt.  Do not carry a
   * lease selected before that transition into a later physical route action:
   * refresh the output-control fence first, then select the exact current Both
   * authority.  This is read-only; the canonical OutputControl executor still
   * performs its own fenced validation immediately before publication.
   */
  const selectFreshBothOutputLease = async () => {
    await options.invoke<unknown>("query_output_control_authority_v1");
    return selectBothOutputLease();
  };

  /**
   * The normal Enable path is the only lease-acquire/recover transaction. A
   * fresh process has no exact Both lease yet, so requiring a selected lease
   * here would make the quick setup impossible to start. Once Enable returns,
   * re-query the authoritative active Both lease before any route action.
   */
  const ensureBothOutputLease = async () => {
    const query = await queryOutputLeaseAuthority(options.invoke);
    if (hasOnlyActiveOutputLease(query, ["lighting", "video"])) {
      return {
        lease: selectOnlyActiveOutputLease(query, ["lighting", "video"]),
        enabled: false,
      };
    }
    await enableOutput(options.invoke);
    return {
      lease: await selectBothOutputLease(),
      enabled: true,
    };
  };

  const enableStagedShowArtNetLoopbackRouteInternal = async (
    lease?: Awaited<ReturnType<typeof selectFreshBothOutputLease>>,
  ) => {
    const currentLease = lease ?? await selectFreshBothOutputLease();
    await executeOutputControl(options.invoke, {
      kind: "enable_show_art_net_loopback_route",
      lease: currentLease,
    });
    return options.refreshSnapshot();
  };

  const enableStagedShowArtNetLoopbackRoute = async () => {
    try {
      await enableStagedShowArtNetLoopbackRouteInternal();
      options.setMessage("Staged same-PC Art-Net loopback show route enabled.");
    } catch (error) {
      options.setMessage(String(error));
    }
  };

  const confirmSerialDmxMachineBindingInternal = async (port: SerialPortSummary) => {
    const instance = port.windows_device_instance_id?.trim();
    if (!instance) {
      throw new Error("USB-DMX confirmation requires the current Windows PnP instance; no binding was written.");
    }
    serialDmxStatusPoller.invalidate();
    try {
      await options.invoke<SerialDmxMachineBindingStatus>(
        "select_serial_dmx_machine_binding_v1",
        { request: { portName: port.name, windowsDeviceInstanceId: instance } },
      );
    } finally {
      await refreshAuthoritativeSerialDmxRuntimeStatuses();
    }
  };

  const confirmSerialDmxMachineBinding = async (port: SerialPortSummary) => {
    try {
      await confirmSerialDmxMachineBindingInternal(port);
      options.setMessage("USB-DMX machine-local identity confirmed. Project files were not changed.");
    } catch (error) {
      options.setMessage(String(error));
    }
  };

  const enableShowSerialDmxSafetyBlackoutRouteInternal = async () => {
    serialDmxStatusPoller.invalidate();
    try {
      if (!options.safetyBlackout()) {
        throw new Error("Engage S0 safety blackout first; the USB-DMX worker is intentionally zero-first and will not start while S0 is clear.");
      }
      await refreshSerialDmxRuntimeStatuses();
      const binding = serialDmxMachineBindingStatus();
      if (binding?.state !== "selected_and_present") {
        throw new Error("USB-DMX worker is unavailable until exactly one confirmed machine-local identity is present.");
      }
      const lease = await selectFreshBothOutputLease();
      await executeOutputControl(options.invoke, {
        kind: "enable_show_serial_dmx_safety_blackout_route",
        lease,
      });
    } finally {
      await refreshAuthoritativeSerialDmxRuntimeStatuses();
    }
  };

  const enableShowSerialDmxSafetyBlackoutRoute = async () => {
    try {
      await enableShowSerialDmxSafetyBlackoutRouteInternal();
      options.setMessage("USB-DMX Open DMX worker activated with S0 queued. This is not a fixture or physical-wire acceptance result.");
    } catch (error) {
      options.setMessage(String(error));
    }
  };

  const prepareShowDmx = (port: SerialPortSummary): Promise<void> => {
    if (showDmxPreparationPromise) return showDmxPreparationPromise;
    setShowDmxPreparationBusy(true);
    setShowDmxPreparationStage("Preflight");
    const run = async () => {
      let stage = "Preflight";
      const runStage = async <T,>(label: string, operation: () => Promise<T>): Promise<T> => {
        stage = label;
        setShowDmxPreparationStage(label);
        options.setMessage(`Show DMX setup [${label}] starting; S0 will not be cleared.`);
        try {
          return await operation();
        } catch (error) {
          throw new Error(`stage=${label}; ${String(error)}`);
        }
      };

      try {
        if (!port.name.trim() || !port.windows_device_instance_id?.trim()) {
          throw new Error("the selected USB-DMX device has no complete machine-local identity");
        }
        if (!isExactShowArtNetLoopbackRoute(output())) {
          throw new Error("the exact Art-Net 127.0.0.1:6454/U0 show route is not staged");
        }
        if (showSerialDmxSafetyBlackoutRouteStatus()?.active) {
          throw new Error("the Open DMX worker is already active; no duplicate arm was attempted");
        }
        if (!output().enabled && options.safetyBlackout()) {
          throw new Error("the validated Art-Net loopback enable requires clear S0; S0 was not cleared");
        }

        await runStage("1/4 output role Both", async () => {
          const prepared = await ensureBothOutputLease();
          // enable_output already atomically arms the engine's Both role. An
          // additional arm would only add a redundant mutation and receipt.
          if (!prepared.enabled) {
            const lease = await selectFreshBothOutputLease();
            await executeOutputControl(options.invoke, {
              kind: "arm",
              role: "both",
              lease,
            });
          }
        });
        await runStage("2/4 machine-local binding", () => confirmSerialDmxMachineBindingInternal(port));
        await runStage("3/4 Art-Net loopback", async () => {
          if (output().enabled) return;
          if (options.safetyBlackout()) {
            throw new Error("the validated Art-Net loopback enable requires clear S0; S0 was not cleared");
          }
          // Refresh again after stage 1/2.  Both lease generation and the
          // output-control fence may have changed while the binding action
          // completed; stage 3 must never reuse either earlier observation.
          const lease = await selectFreshBothOutputLease();
          await enableStagedShowArtNetLoopbackRouteInternal(lease);
        });
        await runStage("4/4 S0 + Open DMX arm", async () => {
          if (!options.safetyBlackout()) {
            await safetyBlackoutRuntime.engage();
            const refreshed = await options.refreshSnapshot();
            if (!refreshed?.blackout && !options.safetyBlackout()) {
              throw new Error("S0 engagement was not confirmed by a fresh snapshot");
            }
          }
          await enableShowSerialDmxSafetyBlackoutRouteInternal();
        });
        setShowDmxPreparationStage("Complete");
        options.setMessage("Show DMX setup complete: output role Both, machine binding, Art-Net loopback, S0, and Open DMX arm succeeded.");
      } catch (error) {
        setShowDmxPreparationStage(`Stopped at ${stage}`);
        options.setMessage(`Show DMX setup stopped at ${stage}: ${String(error).replace(/^Error:\s*/i, "")}`);
      }
    };
    const promise = run().finally(() => {
      setShowDmxPreparationBusy(false);
      showDmxPreparationPromise = null;
    });
    showDmxPreparationPromise = promise;
    return promise;
  };

  const stopShowSerialDmxSafetyBlackoutRoute = async () => {
    serialDmxStatusPoller.invalidate();
    try {
      const lease = selectOnlyActiveOutputLease(
        await queryOutputLeaseAuthority(options.invoke),
        ["lighting", "video"],
      );
      await executeOutputControl(options.invoke, {
        kind: "stop_show_serial_dmx_safety_blackout_route",
        lease,
      });
      options.setMessage("USB-DMX Open DMX worker stopped. No project route was changed.");
    } catch (error) {
      options.setMessage(String(error));
    } finally {
      await refreshAuthoritativeSerialDmxRuntimeStatuses();
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
      options.setMessage("Same-PC V2 Syndocal Background/Foreground Spout outputs enabled.");
    } catch (error) {
      options.setMessage(String(error));
    }
  };

  const resetShowSpoutOutputs = async () => {
    try {
      await executeOutputControl(options.invoke, { kind: "reset_show_spout_outputs" });
      await options.refreshSnapshot();
      options.setMessage("Recognized show Spout outputs were reset.");
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
    serialDmxStatusPoller.invalidate();
    try {
      const ports = await options.invoke<SerialPortSummary[]>("list_serial_ports");
      options.setSerialPorts(ports);
    } catch (error) {
      options.setMessage(String(error));
    } finally {
      await refreshAuthoritativeSerialDmxRuntimeStatuses();
    }
  };

  void refreshDsf2026ArtNetAcceptanceProbeStatus();
  void refreshSerialDmxRuntimeStatuses();

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
    serialDmxMachineBindingStatus,
    showSerialDmxSafetyBlackoutRouteStatus,
    showDmxPreparationBusy,
    showDmxPreparationStage,
    refreshEngineTelemetryReport,
    refreshSerialDmxRuntimeStatuses,
    handleSerialDmxRouteStatusEvent,
    setSerialDmxRouteStatusEventFenceAvailable,
    disposeSerialDmxRuntimeStatuses: serialDmxStatusPoller.dispose,
    resetEngineTelemetry,
    saveEngineTelemetryReport,
    applyOutput,
    enableStagedShowArtNetLoopbackRoute,
    confirmSerialDmxMachineBinding,
    enableShowSerialDmxSafetyBlackoutRoute,
    prepareShowDmx,
    stopShowSerialDmxSafetyBlackoutRoute,
    sendDsf2026ArtNetAcceptanceProbe,
    acknowledgeDsf2026ArtNetAcceptanceProbeInDoubt,
    enableShowSpoutOutputs,
    resetShowSpoutOutputs,
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
