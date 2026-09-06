import { createSignal } from "solid-js";
import type {
  DmxOutputConfig,
  SerialDmxMachineBindingIdentity,
  SerialDmxMachineBindingStatus,
  SerialPortSummary,
  ShowSerialDmxSafetyBlackoutRouteStatus,
} from "../types";

interface DmxOutputConfigPanelProps {
  output: DmxOutputConfig;
  serialPorts: SerialPortSummary[];
  serialDmxMachineBindingStatus: SerialDmxMachineBindingStatus | null;
  showSerialDmxSafetyBlackoutRouteStatus: ShowSerialDmxSafetyBlackoutRouteStatus | null;
  safetyBlackoutEngaged: boolean;
  showDmxPreparationBusy: boolean;
  showDmxPreparationStage: string | null;
  onPrepareShowDmx: (port?: SerialPortSummary) => void | Promise<void>;
  onEnableStagedShowArtNetLoopbackRoute: () => void | Promise<void>;
  onConfirmSerialDmxMachineBinding: (port: SerialPortSummary) => void | Promise<void>;
  onEnableShowSerialDmxSafetyBlackoutRoute: () => void | Promise<void>;
  onStopShowSerialDmxSafetyBlackoutRoute: () => void | Promise<void>;
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
        ? "Select one device"
        : "No eligible device";
    }
    if (selectedSerialPortKey() && serialPortKey(selected) === selectedSerialPortKey()) {
      return "Selected";
    }
    if (persistedBindingKey() && serialPortKey(selected) === persistedBindingKey()) {
      return "Restored";
    }
    return "One eligible device";
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
    ? "Art-Net mirror off"
    : !serialWorkerStatusKnown()
    ? "Worker unavailable · S0 required"
    : serialWorkerFaulted()
    ? "Fault · S0 latched"
    : serialWorkerActive() && props.safetyBlackoutEngaged
    ? "S0 armed"
    : serialWorkerActive() && props.showSerialDmxSafetyBlackoutRouteStatus?.liveFrameQueued
        ? "Live mirror queued"
        : serialWorkerActive() ? "Worker active" : bindingReady() ? "Confirmed" : "Binding required";
  const serialStateTone = () => !artNetUnityMirrorEnabled() || !serialWorkerStatusKnown() || serialWorkerFaulted()
    ? "error"
    : serialWorkerActive() ? "ready" : bindingReady() ? "idle" : "error";
  const showDmxPreparationCanStart = () => Boolean(
    !props.showDmxPreparationBusy
    && exactRoute()
    && !serialWorkerActive(),
  );
  const showDmxPreparationStatus = () => props.showDmxPreparationBusy
    ? `In progress: ${props.showDmxPreparationStage ?? "preflight"}`
    : props.showDmxPreparationStage ?? "Ready · Art-Net output can be prepared without a fixture or USB-DMX device";
  const routeState = () => !exactRoute()
    ? "Logical route mismatch"
    : props.output.enabled ? "Enabled" : "Staged disabled";
  const routeStateTone = () => !exactRoute() ? "error" : props.output.enabled ? "ready" : "idle";

  return (
    <section class="dmxOutputConfigPanel ioConnectionDesk" data-io-default-surface="dmx">
      <header class="ioDeskHeader">
        <div><h2>DMX Connections</h2><span>USB-DMX + Art-Net</span></div>
        <span class={`ioConnectionState ${routeStateTone()}`}><i aria-hidden="true" />{routeState()}</span>
      </header>

      <div class="dmxShowSetup dmxRouteBuilder" data-io-show-dmx-setup>
        <div class="dmxShowSetupCopy">
          <strong>Show DMX</strong>
        </div>
        <div class="dmxShowSetupControls">
          <label class="dmxShowSetupUsb" for="show-usb-dmx-device">
            <span>USB-DMX</span>
            <select
              id="show-usb-dmx-device"
              value={selectedSerialPort() ? serialPortKey(selectedSerialPort()!) : ""}
              onInput={(event) => setSelectedSerialPortKey(event.currentTarget.value)}
            >
              <option value="">No USB device</option>
              {props.serialPorts.map((port) => (
                <option
                  value={serialPortKey(port)}
                  disabled={!hasExactMachineLocalOpenDmxIdentity(port)}
                  data-no-localize
                >
                  {`${port.name} · ${port.manufacturer ?? "unknown"} / ${port.product ?? "unknown"}`}
                </option>
              ))}
            </select>
          </label>
          <span class="dmxShowSetupSelectionState" role="status" data-io-usb-dmx-selection-state>
            {selectedSerialPortSelectionSource()}
          </span>
        </div>
        <button
          data-io-control="dmx-prepare-show-dmx"
          class="primary"
          disabled={!showDmxPreparationCanStart()}
          aria-describedby="dmx-show-dmx-setup-status"
          onClick={() => {
            void props.onPrepareShowDmx(selectedSerialPort());
          }}
        >{props.showDmxPreparationBusy ? "Preparing…" : "Prepare"}</button>
        <p id="dmx-show-dmx-setup-status" class="dmxShowSetupStatus ioDisclosureDescription" role="status" data-io-show-dmx-setup-status>
          {showDmxPreparationStatus()}
        </p>
      </div>

      <details class="ioDisclosure dmxIndividualDiagnostics" data-io-disclosure="dmx-individual-diagnostics">
        <summary>Advanced</summary>
        <div class="ioDisclosureBody">
          <div class="dmxRouteBuilder dmxPrimaryControls">
            <button
              data-io-control="dmx-enable-staged-show-artnet-loopback-route"
              class="primary"
              disabled={!exactRoute() || props.output.enabled}
              aria-describedby="dmx-show-route-confirmation"
              onClick={() => void props.onEnableStagedShowArtNetLoopbackRoute()}
            >Enable Art-Net loopback</button>
          </div>

          <details class="ioDisclosure dmxNestedDisclosure" data-io-disclosure="dmx-usb-manual-controls">
            <summary>Manual controls</summary>
            <div class="ioDisclosureBody">
              <div class="dmxRouteBuilder dmxPrimaryControls" data-io-usb-dmx-route>
                <button
                  data-io-control="dmx-confirm-serial-machine-binding"
                  disabled={!hasExactMachineLocalOpenDmxIdentity(selectedSerialPort()) || !serialBindingMutationAdmissible()}
                  onClick={() => {
                    const selected = selectedSerialPort();
                    if (hasExactMachineLocalOpenDmxIdentity(selected) && serialBindingMutationAdmissible()) {
                      void props.onConfirmSerialDmxMachineBinding(selected);
                    }
                  }}
                >Confirm USB-DMX</button>
                <button
                  data-io-control="dmx-enable-show-serial-dmx-safety-blackout-route"
                  class="primary"
                  disabled={!serialWorkerArmAdmissible() || !serialWorkerStatusKnown() || !bindingReady() || !props.safetyBlackoutEngaged || !artNetUnityMirrorEnabled()}
                  onClick={() => void props.onEnableShowSerialDmxSafetyBlackoutRoute()}
                >Arm USB-DMX under S0</button>
                <button
                  data-io-control="dmx-stop-show-serial-dmx-safety-blackout-route"
                  disabled={!serialWorkerActive() || !serialWorkerStatusKnown() || !props.safetyBlackoutEngaged}
                  onClick={() => void props.onStopShowSerialDmxSafetyBlackoutRoute()}
                >Stop USB-DMX</button>
              </div>
            </div>
          </details>

          <details class="ioDisclosure dmxNestedDisclosure" data-io-disclosure="dmx-usb-identities">
            <summary>Device identity</summary>
            <div class="ioDisclosureBody">
              <RawUsbDmxIdentity
                dataAttribute="data-io-usb-dmx-observed-identity"
                heading="Observed selection"
                identity={observedSerialIdentity()}
              />
              <RawUsbDmxIdentity
                dataAttribute="data-io-usb-dmx-confirmed-identity"
                heading="Confirmed machine-local identity"
                identity={confirmedSerialIdentity()}
              />
            </div>
          </details>

          <details class="ioDisclosure dmxProtocolDetails dmxNestedDisclosure" data-io-disclosure="dmx-protocol-details">
            <summary>Route facts</summary>
            <div class="ioDisclosureBody">
              <dl class="dmxProtocolFacts">
                <dt>Art-Net</dt>
                <dd id="dmx-show-route-confirmation">127.0.0.1:6454 · wire U0 · 512ch · 40–44fps</dd>
                <dt>USB-DMX</dt>
                <dd data-io-usb-dmx-artnet-mirror-state>{serialState()} · logical U0 · ≈32.5fps</dd>
                <dt>Binding</dt>
                <dd data-io-usb-dmx-binding-state>{bindingReady() ? "Confirmed" : "Required"}</dd>
                <dt>Safety</dt>
                <dd data-io-usb-dmx-safety-state>{props.safetyBlackoutEngaged ? "S0 engaged" : "S0 clear"}</dd>
              </dl>
              {!exactRoute() && <p class="ioDisclosureDescription" role="alert">
                Art-Net route mismatch · expected 127.0.0.1:6454 / wire U0.
              </p>}
              <p class="ioDisclosureDescription dmxLogHint">
                Detailed diagnostics are written to the application log.
              </p>
            </div>
          </details>

          <div class="dmxRouteList" data-io-route-list data-io-route-total="2" data-io-route-page-count="1">
            <div class="panelHeader"><h3>Routes</h3><span>2</span></div>
            <div class="dmxRouteRows"><div class="dmxRouteRow" data-io-route-row data-route-index="0">
              <span class={`ioStatusDot ${routeStateTone() === "ready" ? "ok" : routeStateTone()}`} aria-hidden="true" />
              <strong>Art-Net · ArtDmx</strong><span data-io-route-target>127.0.0.1:6454</span>
              <span data-io-route-universe>Wire U0 · 512ch · 40–44fps</span>
            </div><div class="dmxRouteRow" data-io-route-row data-route-index="1">
              <span class={`ioStatusDot ${serialStateTone() === "ready" ? "ok" : serialStateTone()}`} aria-hidden="true" />
              <strong>Enttec Open DMX</strong><span data-io-route-target>{serialState()}</span>
              <span data-io-route-universe>Logical U0 · ≈32.5fps</span>
            </div></div>
          </div>
        </div>
      </details>

    </section>
  );
}
