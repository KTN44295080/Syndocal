import { createSignal, type Accessor, type Setter } from "solid-js";
import {
  executeOutputControl,
  executeBlackoutRelease,
  enableOutput,
  OUTPUT_DSF2026_ARTNET_ACCEPTANCE_PROBE_STATUS_QUERY_OPERATION_ID,
  queryDsf2026ArtNetAcceptanceProbeStatus,
  queryOutputLeaseAuthority,
  selectOnlyActiveOutputLease,
} from "./outputControlController";
import type { Dsf2026ArtNetAcceptanceProbeStatusQuery } from "./outputControlController";
import type {
  OutputControlReceipt,
  OutputLeaseAuthority,
  OutputLeaseAuthorityQuery,
} from "./outputControlController";
import type { FrontendTauriInvoke } from "./tauriInvokeCommands";
import type { OutputOwnershipStatus } from "./types";
import type { ProjectAuthorityToken } from "./projectAuthority";
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

const hasSuccessfulShowArtNetSend = (snapshot: EngineSnapshot | null): boolean => {
  if (!snapshot) return false;
  const route = snapshot.dmx_outputs.length === 1
    ? snapshot.dmx_outputs[0]
    : snapshot.output;
  if (!route || !isExactShowArtNetLoopbackRoute(route) || !route.enabled) return false;
  return snapshot.telemetry.last_dmx_route_results.some((result) =>
    result.index === 0
    && result.universe === route.universe
    && result.attempted
    && result.success,
  );
};

const enttecUsbProBaudRate = 57_600;
const enttecOpenDmxBaudRate = 250_000;

export const isSerialDmxProtocol = (protocol: DmxOutputConfig["protocol"]) =>
  protocol === "EnttecUsbPro" || protocol === "DmxKingUltraDmx" || protocol === "EnttecOpenDmx";

export type BothOutputLeasePreparationDecision =
  | { action: "reuse"; lease: OutputLeaseAuthority }
  | { action: "enable" };

export const isBothOutputOwnershipReady = (status: OutputOwnershipStatus): boolean =>
  status.state === "Ready"
  && status.effective_role === "Both"
  && status.desired_role === "Both"
  && status.lighting_allowed
  && status.video_allowed;

/**
 * Resolve stage-1 setup from the authoritative lease query. An exact active
 * Both lease already satisfies the output-role stage; it must not be followed
 * by a second Arm. Sole unavailable or exact orphaned authority takes the
 * canonical atomic enable_output path; ambiguous or wrong-resource authority
 * fails closed before any mutation.
 */
export const decideBothOutputLeasePreparation = (
  query: OutputLeaseAuthorityQuery,
): BothOutputLeasePreparationDecision => {
  const held = query.statuses.filter((status) => status.status !== "unavailable");
  if (
    query.statuses.length === 1
    && held.length === 1
    && held[0].resources.length === 2
    && held[0].resources[0] === "lighting"
    && held[0].resources[1] === "video"
  ) {
    if (held[0].status === "held_orphaned") return { action: "enable" };
    return {
      action: "reuse",
      lease: selectOnlyActiveOutputLease(query, ["lighting", "video"]),
    };
  }
  if (query.statuses.length === 1 && query.statuses[0].status === "unavailable") {
    return { action: "enable" };
  }
  throw new Error(
    "Output lease authority is ambiguous or has the wrong resources; show DMX setup stopped before mutation.",
  );
};

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
  refreshProjectAuthority: (receipt: OutputControlReceipt, operationLabel?: string) => Promise<void>;
  captureProjectAuthorityIdentity: () => ProjectAuthorityToken;
  isProjectAuthorityIdentityCurrent: (captured: ProjectAuthorityToken) => boolean;
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
  let activeShowOutputPreparation: { key: string; promise: Promise<void> } | null = null;
  const reportShowOutputPreparationBusy = (): Promise<void> => {
    options.setMessage("Another show-output preparation is in progress; wait for it to finish, then retry this action. No additional action was queued.");
    return Promise.resolve();
  };
  // Same-action double clicks share one operation. Different actions are
  // explicitly rejected, never queued across a possible project/owner change.
  const runShowOutputAction = (key: string, operation: () => Promise<void>): Promise<void> => {
    if (activeShowOutputPreparation) {
      return activeShowOutputPreparation.key === key
        ? activeShowOutputPreparation.promise
        : reportShowOutputPreparationBusy();
    }
    const promise = Promise.resolve().then(operation).finally(() => {
      activeShowOutputPreparation = null;
    });
    activeShowOutputPreparation = { key, promise };
    return promise;
  };
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
    const decision = decideBothOutputLeasePreparation(query);
    if (decision.action === "reuse") {
      const ownership = await options.invoke<OutputOwnershipStatus>("get_output_ownership_status");
      if (!isBothOutputOwnershipReady(ownership)) {
        throw new Error(
          "An active Both lease exists, but output ownership is not Ready/Both; no Arm or later show-DMX stage was attempted.",
        );
      }
      return decision;
    }
    await enableOutput(options.invoke);
    const ownership = await options.invoke<OutputOwnershipStatus>("get_output_ownership_status");
    if (!isBothOutputOwnershipReady(ownership)) {
      throw new Error(
        "Output enable returned without confirmed Ready/Both ownership; show DMX setup stopped before device or route mutation.",
      );
    }
    return {
      lease: await selectBothOutputLease(),
      action: "enabled" as const,
    };
  };

  const enableStagedShowArtNetLoopbackRouteInternal = async (
    lease?: Awaited<ReturnType<typeof selectFreshBothOutputLease>>,
  ): Promise<OutputControlReceipt> => {
    const currentLease = lease ?? await selectFreshBothOutputLease();
    return executeOutputControl(options.invoke, {
      kind: "enable_show_art_net_loopback_route",
      lease: currentLease,
    });
  };

  const waitForShowArtNetOutput = async () => {
    const deadline = Date.now() + 1_500;
    let snapshot = await options.refreshSnapshot();
    while (!hasSuccessfulShowArtNetSend(snapshot) && Date.now() < deadline) {
      await new Promise((resolve) => setTimeout(resolve, 50));
      snapshot = await options.refreshSnapshot();
    }
    if (!hasSuccessfulShowArtNetSend(snapshot)) {
      const routeError = snapshot?.telemetry.last_dmx_route_results[0]?.error;
      throw new Error(
        routeError
          ? `Art-Net sender did not report a successful frame: ${routeError}`
          : "Art-Net sender did not report a successful frame before preparation completed",
      );
    }
  };

  const waitForShowSerialDmxLive = async () => {
    const deadline = Date.now() + 1_500;
    let status = showSerialDmxSafetyBlackoutRouteStatus();
    while ((!status || !status.active || !status.liveFrameQueued || status.faulted) && Date.now() < deadline) {
      await new Promise((resolve) => setTimeout(resolve, 50));
      await refreshSerialDmxRuntimeStatuses();
      status = showSerialDmxSafetyBlackoutRouteStatus();
    }
    if (!status?.active || !status.liveFrameQueued || status.faulted) {
      throw new Error("Open DMX worker did not confirm a live U0 frame after S0 release");
    }
  };

  const enableStagedShowArtNetLoopbackRoute = () => runShowOutputAction("artnet", async () => {
    let authority = options.captureProjectAuthorityIdentity();
    try {
      if (!options.isProjectAuthorityIdentityCurrent(authority)) return;
      await ensureBothOutputLease();
      if (!options.isProjectAuthorityIdentityCurrent(authority)) return;
      const receipt = await enableStagedShowArtNetLoopbackRouteInternal();
      await options.refreshProjectAuthority(receipt, "Show Art-Net loopback");
      authority = options.captureProjectAuthorityIdentity();
      await options.refreshSnapshot();
      if (!options.isProjectAuthorityIdentityCurrent(authority)) return;
      options.setMessage("Staged same-PC Art-Net loopback show route enabled.");
    } catch (error) {
      if (!options.isProjectAuthorityIdentityCurrent(authority)) return;
      options.setMessage(String(error));
    }
  });

  const confirmSerialDmxMachineBindingInternal = async (port: SerialPortSummary) => {
    const instance = port.windows_device_instance_id?.trim();
    if (!instance) {
      throw new Error("USB-DMX confirmation requires the current Windows PnP instance; no binding was written.");
    }
    serialDmxStatusPoller.invalidate();
    try {
      return await options.invoke<SerialDmxMachineBindingStatus>(
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

  const enableShowSerialDmxSafetyBlackoutRouteInternal = async (
    confirmedBinding?: SerialDmxMachineBindingStatus,
  ) => {
    serialDmxStatusPoller.invalidate();
    try {
      if (!options.safetyBlackout()) {
        throw new Error("Engage S0 safety blackout first; the USB-DMX worker is intentionally zero-first and will not start while S0 is clear.");
      }
      await refreshSerialDmxRuntimeStatuses();
      const binding = confirmedBinding?.state === "selected_and_present"
        ? confirmedBinding
        : serialDmxMachineBindingStatus();
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
    const authority = options.captureProjectAuthorityIdentity();
    try {
      if (!options.isProjectAuthorityIdentityCurrent(authority)) return;
      await enableShowSerialDmxSafetyBlackoutRouteInternal();
      if (!options.isProjectAuthorityIdentityCurrent(authority)) return;
      options.setMessage("USB-DMX Open DMX worker activated with S0 queued. This is not a fixture or physical-wire acceptance result.");
    } catch (error) {
      if (!options.isProjectAuthorityIdentityCurrent(authority)) return;
      options.setMessage(String(error));
    }
  };

  const prepareShowDmx = (port?: SerialPortSummary): Promise<void> => {
    if (showDmxPreparationPromise) return showDmxPreparationPromise;
    if (activeShowOutputPreparation) return reportShowOutputPreparationBusy();
    setShowDmxPreparationBusy(true);
    setShowDmxPreparationStage("Preflight");
    const run = async () => {
      let authority = options.captureProjectAuthorityIdentity();
      let stage = "Preflight";
      let safetyBlackoutEngagedByPreparation = false;
      let safetyBlackoutReleasedByPreparation = false;
      let confirmedBinding: SerialDmxMachineBindingStatus | undefined;
      const runStage = async <T,>(label: string, operation: () => Promise<T>): Promise<T> => {
        if (!options.isProjectAuthorityIdentityCurrent(authority)) {
          throw new Error("Project changed while Show DMX setup was pending; the older setup was discarded.");
        }
        stage = label;
        setShowDmxPreparationStage(label);
        options.setMessage(`Show DMX setup [${label}] starting; S0 changes only through the zero-first USB-DMX startup.`);
        try {
          const result = await operation();
          if (!options.isProjectAuthorityIdentityCurrent(authority)) {
            throw new Error("Project changed while Show DMX setup was pending; the older setup was discarded.");
          }
          return result;
        } catch (error) {
          throw new Error(`stage=${label}; ${String(error)}`);
        }
      };

      try {
        if (port && (!port.name.trim() || !port.windows_device_instance_id?.trim())) {
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
          await ensureBothOutputLease();
        });
        await runStage("2/4 optional machine-local binding", async () => {
          if (!port) {
            options.setMessage("No USB-DMX device selected; continuing with the Art-Net show route.");
            return;
          }
          confirmedBinding = await confirmSerialDmxMachineBindingInternal(port);
        });
        await runStage("3/4 Art-Net loopback", async () => {
          if (!output().enabled) {
            if (options.safetyBlackout()) {
              throw new Error("the validated Art-Net loopback enable requires clear S0; S0 was not cleared");
            }
            // Refresh again after stage 1/2.  Both lease generation and the
            // output-control fence may have changed while the binding action
            // completed; stage 3 must never reuse either earlier observation.
            const lease = await selectFreshBothOutputLease();
            const receipt = await enableStagedShowArtNetLoopbackRouteInternal(lease);
            await options.refreshProjectAuthority(receipt, "Show Art-Net loopback");
            authority = options.captureProjectAuthorityIdentity();
          }
          await waitForShowArtNetOutput();
        });
        await runStage("4/4 optional Open DMX arm", async () => {
          if (!port) {
            options.setMessage(
              "Show DMX prepared: Art-Net loopback is live. No USB-DMX device was selected, so the physical route remains unarmed.",
            );
            return;
          }
          if (!options.safetyBlackout()) {
            await safetyBlackoutRuntime.engage();
            safetyBlackoutEngagedByPreparation = true;
            const engaged = await options.refreshSnapshot();
            if (!engaged?.safety_blackout_engaged) {
              throw new Error("the zero-first USB-DMX startup could not confirm S0 engagement");
            }
          }
          await enableShowSerialDmxSafetyBlackoutRouteInternal(confirmedBinding);
          if (safetyBlackoutEngagedByPreparation) {
            await executeBlackoutRelease(options.invoke);
            const released = await options.refreshSnapshot();
            if (!released || released.safety_blackout_engaged) {
              throw new Error("USB-DMX started, but S0 release was not confirmed; output remains fail-closed");
            }
            safetyBlackoutReleasedByPreparation = true;
          }
          if (safetyBlackoutReleasedByPreparation) {
            await waitForShowSerialDmxLive();
          }
        });
        if (!options.isProjectAuthorityIdentityCurrent(authority)) return;
        setShowDmxPreparationStage("Complete");
        options.setMessage(
          safetyBlackoutReleasedByPreparation
            ? "Show DMX setup complete: output role Both, machine binding, Art-Net loopback, and USB-DMX are live. S0 remains clear."
            : options.safetyBlackout()
              ? "Show DMX setup complete: output role Both, machine binding, Art-Net loopback, and Open DMX arm succeeded with the already-engaged S0."
              : "Show DMX setup complete: output role Both, machine binding, and Art-Net loopback are ready. S0 remains clear and USB-DMX is unarmed.",
        );
      } catch (error) {
        if (!options.isProjectAuthorityIdentityCurrent(authority)) {
          setShowDmxPreparationStage(`Stopped at ${stage}`);
          return;
        }
        setShowDmxPreparationStage(`Stopped at ${stage}`);
        options.setMessage(`Show DMX setup stopped at ${stage}: ${String(error).replace(/^Error:\s*/i, "")}`);
      }
    };
    const promise = runShowOutputAction("dmx", run).finally(() => {
      setShowDmxPreparationBusy(false);
      showDmxPreparationPromise = null;
    });
    showDmxPreparationPromise = promise;
    return promise;
  };

  const stopShowSerialDmxSafetyBlackoutRoute = async () => {
    const authority = options.captureProjectAuthorityIdentity();
    serialDmxStatusPoller.invalidate();
    try {
      if (!options.isProjectAuthorityIdentityCurrent(authority)) return;
      const lease = selectOnlyActiveOutputLease(
        await queryOutputLeaseAuthority(options.invoke),
        ["lighting", "video"],
      );
      await executeOutputControl(options.invoke, {
        kind: "stop_show_serial_dmx_safety_blackout_route",
        lease,
      });
      if (!options.isProjectAuthorityIdentityCurrent(authority)) return;
      options.setMessage("USB-DMX Open DMX worker stopped. No project route was changed.");
    } catch (error) {
      if (!options.isProjectAuthorityIdentityCurrent(authority)) return;
      options.setMessage(String(error));
    } finally {
      await refreshAuthoritativeSerialDmxRuntimeStatuses();
    }
  };

  const sendDsf2026ArtNetAcceptanceProbe = async () => {
    const authority = options.captureProjectAuthorityIdentity();
    try {
      if (!options.isProjectAuthorityIdentityCurrent(authority)) return;
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
      if (!options.isProjectAuthorityIdentityCurrent(authority)) return;
      await options.refreshSnapshot();
      setDsf2026ArtNetAcceptanceProbeStatus({
        operationId: OUTPUT_DSF2026_ARTNET_ACCEPTANCE_PROBE_STATUS_QUERY_OPERATION_ID,
        status: "consumed",
      });
      options.setMessage("DSF2026 fixed red probe: OS accepted one 530-byte ArtDmx U0 datagram to 127.0.0.1:6454; receiver and physical output remain unverified. A second probe is permanently prohibited, including after restart.");
    } catch (error) {
      if (!options.isProjectAuthorityIdentityCurrent(authority)) return;
      options.setMessage(String(error));
    }
  };

  const acknowledgeDsf2026ArtNetAcceptanceProbeInDoubt = async () => {
    const authority = options.captureProjectAuthorityIdentity();
    try {
      if (!options.isProjectAuthorityIdentityCurrent(authority)) return;
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
      if (!options.isProjectAuthorityIdentityCurrent(authority)) return;
      await options.refreshSnapshot();
      setDsf2026ArtNetAcceptanceProbeStatus({
        operationId: OUTPUT_DSF2026_ARTNET_ACCEPTANCE_PROBE_STATUS_QUERY_OPERATION_ID,
        status: "consumed",
      });
      options.setMessage("DSF2026 probe InDoubt hold reconciled without sending Art-Net. Receiver and physical output were independently verified by the operator. A fresh probe remains permanently prohibited.");
    } catch (error) {
      if (!options.isProjectAuthorityIdentityCurrent(authority)) return;
      options.setMessage(String(error));
    }
  };

  const enableShowSpoutOutputs = () => runShowOutputAction("spout", async () => {
    let authority = options.captureProjectAuthorityIdentity();
    try {
      if (!options.isProjectAuthorityIdentityCurrent(authority)) return;
      await ensureBothOutputLease();
      if (!options.isProjectAuthorityIdentityCurrent(authority)) return;
      const lease = await selectFreshBothOutputLease();
      const receipt = await executeOutputControl(options.invoke, {
        kind: "enable_show_spout_outputs",
        lease,
      });
      if (!options.isProjectAuthorityIdentityCurrent(authority)) return;
      await options.refreshProjectAuthority(receipt);
      authority = options.captureProjectAuthorityIdentity();
      await options.refreshSnapshot();
      if (!options.isProjectAuthorityIdentityCurrent(authority)) return;
      options.setMessage("Same-PC V2 Syndocal Background/Foreground Spout outputs enabled.");
    } catch (error) {
      if (!options.isProjectAuthorityIdentityCurrent(authority)) return;
      options.setMessage(String(error));
    }
  });

  const resetShowSpoutOutputs = async () => {
    let authority = options.captureProjectAuthorityIdentity();
    try {
      if (!options.isProjectAuthorityIdentityCurrent(authority)) return;
      const receipt = await executeOutputControl(options.invoke, { kind: "reset_show_spout_outputs" });
      if (!options.isProjectAuthorityIdentityCurrent(authority)) return;
      await options.refreshProjectAuthority(receipt);
      authority = options.captureProjectAuthorityIdentity();
      await options.refreshSnapshot();
      if (!options.isProjectAuthorityIdentityCurrent(authority)) return;
      options.setMessage("Recognized show Spout outputs were reset.");
    } catch (error) {
      if (!options.isProjectAuthorityIdentityCurrent(authority)) return;
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
