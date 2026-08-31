import { createSignal } from "solid-js";
import type {
  DmxOutputConfig,
  SerialDmxMachineBindingIdentity,
  SerialDmxMachineBindingStatus,
  SerialPortSummary,
  ShowSerialDmxSafetyBlackoutRouteStatus,
} from "../types";
import type { Dsf2026ArtNetAcceptanceProbeStatusQuery } from "../outputControlController";

interface DmxOutputConfigPanelProps {
  output: DmxOutputConfig;
  serialPorts: SerialPortSummary[];
  serialDmxMachineBindingStatus: SerialDmxMachineBindingStatus | null;
  showSerialDmxSafetyBlackoutRouteStatus: ShowSerialDmxSafetyBlackoutRouteStatus | null;
  safetyBlackoutEngaged: boolean;
  showDmxPreparationBusy: boolean;
  showDmxPreparationStage: string | null;
  dsf2026ArtNetAcceptanceProbeStatus: () => Dsf2026ArtNetAcceptanceProbeStatusQuery | null;
  onPrepareShowDmx: (port: SerialPortSummary) => void | Promise<void>;
  onEnableStagedShowArtNetLoopbackRoute: () => void | Promise<void>;
  onConfirmSerialDmxMachineBinding: (port: SerialPortSummary) => void | Promise<void>;
  onEnableShowSerialDmxSafetyBlackoutRoute: () => void | Promise<void>;
  onStopShowSerialDmxSafetyBlackoutRoute: () => void | Promise<void>;
  onSendDsf2026ArtNetAcceptanceProbe: () => void | Promise<void>;
  onAcknowledgeDsf2026ArtNetAcceptanceProbeInDoubt: () => void | Promise<void>;
}

interface UsbDmxIdentityDisplay {
  portName: string;
  manufacturer: string | null | undefined;
  product: string | null | undefined;
  usbVid: number | null | undefined;
  usbPid: number | null | undefined;
  serialNumber: string | null | undefined;
  windowsDeviceInstanceId: string | null | undefined;
}

const rawUsbDmxIdentityForPort = (port: SerialPortSummary): UsbDmxIdentityDisplay => ({
  portName: port.name,
  manufacturer: port.manufacturer,
  product: port.product,
  usbVid: port.usb_vid,
  usbPid: port.usb_pid,
  serialNumber: port.serial_number,
  windowsDeviceInstanceId: port.windows_device_instance_id,
});

const rawUsbDmxIdentityForBinding = (
  identity: SerialDmxMachineBindingIdentity,
): UsbDmxIdentityDisplay => ({
  portName: identity.port_name,
  manufacturer: identity.manufacturer,
  product: identity.product,
  usbVid: identity.usb_vid,
  usbPid: identity.usb_pid,
  serialNumber: identity.serial_number,
  windowsDeviceInstanceId: identity.windows_device_instance_id,
});

const usbHex = (value: number | null | undefined) => value === null || value === undefined
  ? "missing"
  : `0x${value.toString(16).padStart(4, "0").toUpperCase()}`;

const rawUsbValue = (value: string | null | undefined) => value?.trim() || "missing";

const hasNonemptyUsbIdentityText = (value: string | null | undefined) =>
  typeof value === "string" && value.trim().length > 0;

/**
 * This UI eligibility is deliberately as strict as the native machine-local
 * binding boundary. A visible COM/PnP row with a missing or zero VID/PID is
 * diagnostic data only: it must never become a Confirm candidate.
 */
const hasExactMachineLocalOpenDmxIdentity = (
  port: SerialPortSummary | undefined,
): port is SerialPortSummary => Boolean(
  port
  && hasNonemptyUsbIdentityText(port.name)
  && hasNonemptyUsbIdentityText(port.port_type)
  && typeof port.usb_vid === "number"
  && Number.isInteger(port.usb_vid)
  && port.usb_vid > 0
  && port.usb_vid <= 0xffff
  && typeof port.usb_pid === "number"
  && Number.isInteger(port.usb_pid)
  && port.usb_pid > 0
  && port.usb_pid <= 0xffff
  && hasNonemptyUsbIdentityText(port.serial_number)
  && hasNonemptyUsbIdentityText(port.manufacturer)
  && hasNonemptyUsbIdentityText(port.product)
  && hasNonemptyUsbIdentityText(port.windows_device_instance_id),
);

function RawUsbDmxIdentity(props: {
  dataAttribute: "data-io-usb-dmx-observed-identity" | "data-io-usb-dmx-confirmed-identity";
  heading: string;
  identity: UsbDmxIdentityDisplay | null;
}) {
  return <section class="ioDisclosureDescription" {...{ [props.dataAttribute]: "" }}>
    <strong>{props.heading}</strong>
    <dl class="usbDmxIdentityFields">
      <dt>COM</dt><dd><code data-no-localize>{props.identity?.portName ?? "missing"}</code></dd>
      <dt>Manufacturer / product</dt><dd><code data-no-localize>{`${rawUsbValue(props.identity?.manufacturer)} / ${rawUsbValue(props.identity?.product)}`}</code></dd>
      <dt>USB VID / PID</dt><dd><code data-no-localize>{`${usbHex(props.identity?.usbVid)} / ${usbHex(props.identity?.usbPid)}`}</code></dd>
      <dt>Serial</dt><dd><code data-no-localize>{rawUsbValue(props.identity?.serialNumber)}</code></dd>
      <dt>Windows PnP instance</dt><dd><code data-no-localize>{rawUsbValue(props.identity?.windowsDeviceInstanceId)}</code></dd>
    </dl>
  </section>;
}

const isExactShowArtNetLoopbackRoute = (output: DmxOutputConfig) =>
  output.protocol === "ArtNet"
  && output.target_ip === "127.0.0.1"
  && output.port === 6454
  && output.universe === 0
  && output.serial_port === "";

const serialPortKey = (port: SerialPortSummary) =>
  `${port.name}\u0000${port.windows_device_instance_id ?? ""}`;

/**
 * The show route is deliberately not an editor: Unity shares this machine's
 * Art-Net socket, so this action admits exactly one fixed local ArtDmx route.
 */
export function DmxOutputConfigPanel(props: DmxOutputConfigPanelProps) {
  const [selectedSerialPortKey, setSelectedSerialPortKey] = createSignal("");
  const exactRoute = () => isExactShowArtNetLoopbackRoute(props.output);
  const artNetUnityMirrorEnabled = () => exactRoute() && props.output.enabled;
  const persistedBindingKey = () => {
    const identity = props.serialDmxMachineBindingStatus?.selected;
    return identity ? `${identity.port_name}\u0000${identity.windows_device_instance_id}` : "";
  };
  const selectedSerialPort = () => {
    const explicitKey = selectedSerialPortKey();
    const explicit = explicitKey
      ? props.serialPorts.find((port) => serialPortKey(port) === explicitKey)
      : undefined;
    if (explicit) return explicit;

    const persistedKey = persistedBindingKey();
    const persisted = persistedKey
      ? props.serialPorts.find((port) => serialPortKey(port) === persistedKey)
      : undefined;
    if (persisted) return persisted;

    const eligible = props.serialPorts.filter((port) => hasExactMachineLocalOpenDmxIdentity(port));
    return eligible.length === 1 ? eligible[0] : undefined;
  };
  const selectedSerialPortSelectionSource = () => {
    const selected = selectedSerialPort();
    if (!selected) {
      const eligibleCount = props.serialPorts.filter((port) => hasExactMachineLocalOpenDmxIdentity(port)).length;
      return eligibleCount > 1
        ? "No automatic selection: multiple eligible machine-local devices"
        : "No eligible machine-local device selected";
    }
    if (selectedSerialPortKey() && serialPortKey(selected) === selectedSerialPortKey()) {
      return "Explicit operator selection";
    }
    if (persistedBindingKey() && serialPortKey(selected) === persistedBindingKey()) {
      return "Restored from the exact persisted machine binding";
    }
    return "Auto-selected: exactly one eligible machine-local device";
  };
  const observedSerialIdentity = () => {
    const port = selectedSerialPort();
    return port ? rawUsbDmxIdentityForPort(port) : null;
  };
  const confirmedSerialIdentity = () => {
    const identity = props.serialDmxMachineBindingStatus?.selected;
    return identity ? rawUsbDmxIdentityForBinding(identity) : null;
  };
  const bindingReady = () => props.serialDmxMachineBindingStatus?.state === "selected_and_present";
  const serialWorkerStatusKnown = () => props.showSerialDmxSafetyBlackoutRouteStatus !== null;
  const serialWorkerActive = () => props.showSerialDmxSafetyBlackoutRouteStatus?.active === true;
  const serialWorkerFaulted = () => props.showSerialDmxSafetyBlackoutRouteStatus?.faulted === true;
  const serialBindingMutationAdmissible = () => {
    const status = props.showSerialDmxSafetyBlackoutRouteStatus;
    if (!status) return false;
    if (status.active || status.liveFrameQueued) return false;
    if (!status.workerShutdownCompleted) return false;
    if (status.zeroFrameQueued && !status.zeroFramePhysicalWriteCompleted && !status.faulted) return false;
    // The backend also checks its lock-free physical S0 latch. The UI has
    // only logical S0 truth, so it is deliberately stricter on a faulted
    // recovery and treats native rejection as authoritative.
    if (status.faulted && !props.safetyBlackoutEngaged) return false;
    return true;
  };
  const serialWorkerArmAdmissible = () => {
    const status = props.showSerialDmxSafetyBlackoutRouteStatus;
    if (!status) return false;
    if (status.active || status.liveFrameQueued) return false;
    // A bounded Stop/fault timeout can detach a worker whose driver call later
    // returns. Do not allow an in-process replacement until native status
    // proves that worker completed; faulted joined recovery remains S0-only
    // and native revalidates the physical latch before opening a handle.
    if (!status.workerShutdownCompleted) return false;
    if (status.zeroFrameQueued && !status.zeroFramePhysicalWriteCompleted && !status.faulted) return false;
    if (status.faulted && !props.safetyBlackoutEngaged) return false;
    return true;
  };
  const serialState = () => !artNetUnityMirrorEnabled()
    ? "Art-Net Unity mirror must be enabled"
    : !serialWorkerStatusKnown()
    ? "Worker status unavailable — S0 required"
    : serialWorkerFaulted()
    ? "Faulted — S0 latched"
    : serialWorkerActive() && props.safetyBlackoutEngaged
      ? "S0 armed"
      : serialWorkerActive() && props.showSerialDmxSafetyBlackoutRouteStatus?.liveFrameQueued
        ? "Live U0 mirror queued"
        : serialWorkerActive() ? "Worker active" : bindingReady() ? "Confirmed, stopped" : "Binding required";
  const serialStateTone = () => !artNetUnityMirrorEnabled() || !serialWorkerStatusKnown() || serialWorkerFaulted()
    ? "error"
    : serialWorkerActive() ? "ready" : bindingReady() ? "idle" : "error";
  const showDmxPreparationCanStart = () => Boolean(
    !props.showDmxPreparationBusy
    && exactRoute()
    && serialWorkerStatusKnown()
    && !serialWorkerActive()
    && !serialWorkerFaulted()
    && serialWorkerArmAdmissible()
    && hasExactMachineLocalOpenDmxIdentity(selectedSerialPort()),
  );
  const showDmxPreparationStatus = () => props.showDmxPreparationBusy
    ? `In progress: ${props.showDmxPreparationStage ?? "preflight"}`
    : props.showDmxPreparationStage ?? "Select one exact machine-local device. Loopback is staged first; S0 is engaged before Open DMX is opened.";
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

      <div class="dmxShowSetup dmxRouteBuilder" data-io-show-dmx-setup>
        <div class="dmxShowSetupCopy">
          <strong>Show DMX quick setup</strong>
          <span>Arm output role Both, confirm the selected device, enable the exact Art-Net route, engage S0, then arm Open DMX.</span>
        </div>
        <button
          data-io-control="dmx-prepare-show-dmx"
          class="primary"
          disabled={!showDmxPreparationCanStart()}
          aria-describedby="dmx-show-dmx-setup-status"
          onClick={() => {
            const selected = selectedSerialPort();
            if (hasExactMachineLocalOpenDmxIdentity(selected)) {
              void props.onPrepareShowDmx(selected);
            }
          }}
        >{props.showDmxPreparationBusy ? "Preparing show DMX…" : "Prepare show DMX"}</button>
        <p id="dmx-show-dmx-setup-status" class="ioDisclosureDescription" role="status" data-io-show-dmx-setup-status>
          {showDmxPreparationStatus()}
        </p>
      </div>

      <details class="ioDisclosure dmxIndividualDiagnostics" data-io-disclosure="dmx-individual-diagnostics">
        <summary>Individual DMX diagnostics</summary>
        <div class="ioDisclosureBody">
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

          <div class="dmxRouteBuilder" data-io-usb-dmx-route>
            <label for="show-usb-dmx-device">Machine-local USB-DMX device</label>
            <select
              id="show-usb-dmx-device"
              value={selectedSerialPort() ? serialPortKey(selectedSerialPort()!) : ""}
              onInput={(event) => setSelectedSerialPortKey(event.currentTarget.value)}
            >
              <option value="">Select the exact current PnP device</option>
              {props.serialPorts.map((port) => (
                <option
                  value={serialPortKey(port)}
                  disabled={!hasExactMachineLocalOpenDmxIdentity(port)}
                  data-no-localize
                >
                  {`${port.name} · ${port.manufacturer ?? "unknown"} / ${port.product ?? "unknown"} · VID ${usbHex(port.usb_vid)} PID ${usbHex(port.usb_pid)} · ${port.serial_number ?? "no serial"} · ${port.windows_device_instance_id ?? "no Windows PnP instance"}`}
                </option>
              ))}
            </select>
            <p class="ioDisclosureDescription" role="status" data-io-usb-dmx-selection-state>
              {selectedSerialPortSelectionSource()}
            </p>
            <button
              data-io-control="dmx-confirm-serial-machine-binding"
              disabled={!hasExactMachineLocalOpenDmxIdentity(selectedSerialPort()) || !serialBindingMutationAdmissible()}
              onClick={() => {
                const selected = selectedSerialPort();
                if (hasExactMachineLocalOpenDmxIdentity(selected) && serialBindingMutationAdmissible()) {
                  void props.onConfirmSerialDmxMachineBinding(selected);
                }
              }}
            >Confirm this machine-local Open DMX binding</button>
            <button
              data-io-control="dmx-enable-show-serial-dmx-safety-blackout-route"
              class="primary"
              disabled={!serialWorkerArmAdmissible() || !serialWorkerStatusKnown() || !bindingReady() || !props.safetyBlackoutEngaged || !artNetUnityMirrorEnabled()}
              onClick={() => void props.onEnableShowSerialDmxSafetyBlackoutRoute()}
            >Arm Open DMX worker under S0</button>
            <button
              data-io-control="dmx-stop-show-serial-dmx-safety-blackout-route"
              disabled={!serialWorkerActive() || !serialWorkerStatusKnown() || !props.safetyBlackoutEngaged}
              onClick={() => void props.onStopShowSerialDmxSafetyBlackoutRoute()}
            >Stop Open DMX worker</button>
          </div>

          <RawUsbDmxIdentity
            dataAttribute="data-io-usb-dmx-observed-identity"
            heading="Observed selection — not persisted or protocol-inferred"
            identity={observedSerialIdentity()}
          />
          <RawUsbDmxIdentity
            dataAttribute="data-io-usb-dmx-confirmed-identity"
            heading="Confirmed machine-local identity — expected for the current binding"
            identity={confirmedSerialIdentity()}
          />
        </div>
      </details>

      <div class="dmxRouteList" data-io-route-list data-io-route-total="2" data-io-route-page-count="1">
        <div class="panelHeader"><h3>Logical routes</h3><span>2</span></div>
        <div class="dmxRouteRows"><div class="dmxRouteRow" data-io-route-row data-route-index="0">
          <span class={`ioStatusDot ${routeStateTone() === "ready" ? "ok" : routeStateTone()}`} aria-hidden="true" />
          <strong>Art-Net · ArtDmx</strong><span data-io-route-target>127.0.0.1:6454</span>
          <span data-io-route-universe>Wire U0 · 512ch · 40–44fps</span>
        </div><div class="dmxRouteRow" data-io-route-row data-route-index="1">
          <span class={`ioStatusDot ${serialStateTone() === "ready" ? "ok" : serialStateTone()}`} aria-hidden="true" />
          <strong>Enttec Open DMX · machine local</strong><span data-io-route-target>{serialState()}</span>
          <span data-io-route-universe>Logical U0 · 250000 baud · ≈32.5fps exact-rig USB cadence · S0-first latest-frame mirror</span>
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
        Confirmation retains the native lease, safety-blackout, exact binding, sender-open, acknowledgement, and rollback fences. Binding cannot change while a worker or physical S0 transaction remains in flight. A faulted, joined worker may only accept an explicit replacement while S0 stays latched; selecting it sends nothing and does not clear the fault. It opens only while S0 is engaged and queues zero first; after the separately confirmed Release Blackout, the same worker mirrors the latest completed U0 frame alongside Art-Net. Re-engaging S0 preempts later live bytes with zero. The ≈32.5fps USB cadence is the current FT232R/COM3 rig default, not an Open DMX universal limit; USB does not promise physical delivery on every 44Hz engine tick. USB serial DMX is not a fallback route.
      </p>
      <p class="ioDisclosureDescription" role="status">
        {props.serialDmxMachineBindingStatus?.detail ?? "USB-DMX machine-local selection is loading; no worker can start."}
      </p>
      <p class="ioDisclosureDescription" role="status">
        {props.showSerialDmxSafetyBlackoutRouteStatus?.detail ?? "USB-DMX worker status is loading; no worker can start."}
      </p>
      <p class="ioDisclosureDescription" role="status" data-io-usb-dmx-artnet-mirror-state>
        {props.showSerialDmxSafetyBlackoutRouteStatus?.artnetMirrorDetail
          ?? "Art-Net mirror route state is loading; no USB-DMX worker action is enabled from unknown status."}
      </p>
      <p class="ioDisclosureDescription">
        A generic FTDI VID/PID is not auto-selected and does not imply a protocol. This explicit operator selection is Open DMX only. Worker status distinguishes queue acceptance from the bounded physical zero transaction; neither is fixture or wire delivery. No one-shot USB probe is implemented in this tranche.
      </p>
    </section>
  );
}
