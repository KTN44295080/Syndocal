import { serialDmxDeviceInstance } from "../serialDmxDeviceInstance";
import { hasExactMachineLocalOpenDmxIdentity } from "../usbDmxIdentity";
import { For, Show, createEffect, createSignal } from "solid-js";
import type {
  DmxOutputConfig,
  DmxOutputRouteTelemetry,
  SerialDmxMachineBindingIdentity,
  SerialDmxMachineBindingStatus,
  SerialPortSummary,
  ShowSerialDmxSafetyBlackoutRouteStatus,
  UsbDmxUniversesStatus,
} from "../types";
import { showDmxStageLabel } from "../showDmxStageLabels";
import { showDmxUniverseState } from "../dmxUniverseState";
import {
  MAX_NETWORK_DMX_ROUTES,
  NetworkDmxRouteRows,
  createNetworkDmxRouteDraft,
  type NetworkDmxProtocol,
  type NetworkDmxRouteDraft,
} from "./NetworkDmxRoutesPanel";
import {
  MAX_USB_DMX_CABLES,
  UsbDmxCableRows,
  createUsbDmxCableModel,
  usbDmxPortKey,
  type UsbDmxCableModel,
} from "./UsbDmxCablesPanel";

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
  /** `dmx_outputs`; index 0 is `output`, the rest are the network universes. */
  routes: DmxOutputConfig[];
  routeTelemetry: DmxOutputRouteTelemetry[];
  projectEpoch: number;
  onApplyNetworkRoutes: (routes: DmxOutputConfig[]) => Promise<void>;
  usbDmxUniverses: UsbDmxUniversesStatus | null;
  /** The last USB-DMX universes read failed; the status is the last known one. */
  usbDmxUniversesStale: boolean;
  onRefreshUsbDmxUniverses: () => void | Promise<void>;
  onRescanSerialPorts: () => void | Promise<void>;
  onBindUsbDmxUniverse: (universe: number, port: SerialPortSummary) => void | Promise<void>;
  onClearUsbDmxUniverse: (universe: number) => void | Promise<void>;
  onStartUsbDmxUniverse: (universe: number) => void | Promise<void>;
  onStopUsbDmxUniverse: (universe: number) => void | Promise<void>;
}

interface UsbDmxIdentityDisplay {
  portName: string;
  manufacturer: string | null | undefined;
  product: string | null | undefined;
  usbVid: number | null | undefined;
  usbPid: number | null | undefined;
  serialNumber: string | null | undefined;
  windowsDeviceInstanceId: string | null | undefined;
  macosDeviceInstanceId: string | null | undefined;
}

const rawUsbDmxIdentityForPort = (port: SerialPortSummary): UsbDmxIdentityDisplay => ({
  portName: port.name,
  manufacturer: port.manufacturer,
  product: port.product,
  usbVid: port.usb_vid,
  usbPid: port.usb_pid,
  serialNumber: port.serial_number,
  windowsDeviceInstanceId: port.windows_device_instance_id,
  macosDeviceInstanceId: port.macos_device_instance_id,
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
  macosDeviceInstanceId: identity.macos_device_instance_id,
});

const usbHex = (value: number | null | undefined) => value === null || value === undefined
  ? "missing"
  : `0x${value.toString(16).padStart(4, "0").toUpperCase()}`;

const rawUsbValue = (value: string | null | undefined) => value?.trim() || "missing";

function RawUsbDmxIdentity(props: {
  dataAttribute: "data-io-usb-dmx-observed-identity" | "data-io-usb-dmx-confirmed-identity";
  heading: string;
  identity: UsbDmxIdentityDisplay | null;
}) {
  return <section class="ioDisclosureDescription" {...{ [props.dataAttribute]: "" }}>
    <strong>{props.heading}</strong>
    <dl class="usbDmxIdentityFields">
      <dt>Port</dt><dd><code data-no-localize>{props.identity?.portName ?? "missing"}</code></dd>
      <dt>Manufacturer / product</dt><dd><code data-no-localize>{`${rawUsbValue(props.identity?.manufacturer)} / ${rawUsbValue(props.identity?.product)}`}</code></dd>
      <dt>USB VID / PID</dt><dd><code data-no-localize>{`${usbHex(props.identity?.usbVid)} / ${usbHex(props.identity?.usbPid)}`}</code></dd>
      <dt>Serial</dt><dd><code data-no-localize>{rawUsbValue(props.identity?.serialNumber)}</code></dd>
      <dt>Device instance</dt><dd><code data-no-localize>{rawUsbValue(props.identity?.windowsDeviceInstanceId ?? props.identity?.macosDeviceInstanceId)}</code></dd>
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
  `${port.name}\u0000${serialDmxDeviceInstance(port) ?? ""}`;

type AddTransport = NetworkDmxProtocol | "Usb";

/**
 * The table's last row: one Add for every transport. A network universe is
 * added to the draft and sent with Apply; a USB-DMX cable is bound at once,
 * because the binding is machine-local and never part of the project.
 */
function DmxUniverseAddRow(props: { usb: UsbDmxCableModel; network: NetworkDmxRouteDraft }) {
  const [transport, setTransport] = createSignal<AddTransport>("ArtNet");
  const [networkUniverse, setNetworkUniverse] = createSignal(Number.NaN);
  const usbSelected = () => transport() === "Usb";
  // USB-DMX is offered only while another cable may be added; fall back when
  // that changes under a selected USB-DMX transport.
  createEffect(() => {
    if (usbSelected() && !props.usb.canAddCable()) setTransport("ArtNet");
  });
  const universe = () => usbSelected()
    ? props.usb.universeToBind()
    : Number.isNaN(networkUniverse())
      ? props.network.nextFreeUniverse(transport() as NetworkDmxProtocol)
      : networkUniverse();
  const canAdd = () => usbSelected()
    ? props.usb.canBind()
    : props.network.canAdd() && Number.isInteger(universe());
  const add = () => {
    if (!canAdd()) return;
    if (usbSelected()) {
      props.usb.bind();
      return;
    }
    props.network.add(transport() as NetworkDmxProtocol, universe());
    setNetworkUniverse(Number.NaN);
  };
  return (
    <tfoot>
      <tr class="dmxUniverseAddRow" data-dmx-universe-add>
        <td>
          <input
            data-dmx-universe-add-universe
            aria-label="Universe"
            type="number"
            min={transport() === "ArtNet" ? 0 : 1}
            max={transport() === "Sacn" ? 63999 : 32767}
            value={universe()}
            onInput={(event) => {
              const value = event.currentTarget.value === "" ? Number.NaN : Number(event.currentTarget.value);
              if (usbSelected()) props.usb.setNewUniverse(value);
              else setNetworkUniverse(value);
            }}
          />
        </td>
        <td>
          <select
            data-dmx-universe-add-transport
            aria-label="Transport"
            value={transport()}
            onChange={(event) => {
              setTransport(event.currentTarget.value as AddTransport);
              setNetworkUniverse(Number.NaN);
            }}
          >
            <option value="ArtNet">Art-Net</option>
            <option value="Sacn">sACN</option>
            <option value="Usb" disabled={!props.usb.canAddCable()}>USB-DMX</option>
          </select>
        </td>
        <td>
          <Show
            when={usbSelected()}
            fallback={<span class="dmxUniverseAddHint">Edit the destination in the new row, then Apply.</span>}
          >
            <select
              data-usb-dmx-cable-device
              aria-label="USB-DMX device"
              value={props.usb.selectedPort() ? usbDmxPortKey(props.usb.selectedPort()!) : ""}
              onInput={(event) => props.usb.setNewPortKey(event.currentTarget.value)}
            >
              <option value="">{props.usb.availablePorts().length === 0 ? "No free USB-DMX device" : "Select a device"}</option>
              <For each={props.usb.availablePorts()}>{(port) => (
                <option value={usbDmxPortKey(port)} data-no-localize>
                  {`${port.name} · ${port.manufacturer ?? "unknown"} / ${port.product ?? "unknown"} · ${port.serial_number ?? ""}`}
                </option>
              )}</For>
            </select>
          </Show>
        </td>
        <td class="tabularNums dmxUniverseAddCount" data-no-localize>
          {usbSelected()
            ? `${props.usb.bindings().length} / ${MAX_USB_DMX_CABLES}`
            : `${props.network.draft().length} / ${MAX_NETWORK_DMX_ROUTES}`}
        </td>
        <td>
          <div class="dmxUniverseActions">
            <button
              type="button"
              class="primary"
              data-dmx-universe-add-button
              disabled={!canAdd()}
              onClick={add}
            >{usbSelected() ? "Add cable" : "Add"}</button>
            <button
              type="button"
              data-usb-dmx-cable-rescan
              disabled={props.usb.busy()}
              title="Lists the USB-DMX devices again, for Show DMX and for new cables."
              onClick={() => void props.usb.run(() => props.usb.source.onRescan())}
            >Rescan devices</button>
          </div>
        </td>
      </tr>
    </tfoot>
  );
}

/**
 * Every DMX output universe in one table: Universe 0 (Show DMX), the
 * USB-DMX cables and the Art-Net / sACN universes, one Add row and one
 * Apply for the network edits. The show route itself is deliberately not an
 * editor: Unity shares this machine's Art-Net socket, so Prepare admits
 * exactly one fixed local ArtDmx route.
 */
export function DmxOutputConfigPanel(props: DmxOutputConfigPanelProps) {
  const [selectedSerialPortKey, setSelectedSerialPortKey] = createSignal("");
  const network = createNetworkDmxRouteDraft({
    get routes() { return props.routes; },
    get projectEpoch() { return props.projectEpoch; },
    onApply: (routes) => props.onApplyNetworkRoutes(routes),
  });
  const usb = createUsbDmxCableModel({
    get status() { return props.usbDmxUniverses; },
    get stale() { return props.usbDmxUniversesStale; },
    get serialPorts() { return props.serialPorts; },
    get showDmxCable() { return props.serialDmxMachineBindingStatus?.selected ?? null; },
    get safetyBlackoutEngaged() { return props.safetyBlackoutEngaged; },
    onRefresh: () => props.onRefreshUsbDmxUniverses(),
    onRescan: () => props.onRescanSerialPorts(),
    onBind: (universe, port) => props.onBindUsbDmxUniverse(universe, port),
    onClear: (universe) => props.onClearUsbDmxUniverse(universe),
    onStart: (universe) => props.onStartUsbDmxUniverse(universe),
    onStop: (universe) => props.onStopUsbDmxUniverse(universe),
  });
  const showState = () => showDmxUniverseState({
    route: props.output,
    binding: props.serialDmxMachineBindingStatus,
    worker: props.showSerialDmxSafetyBlackoutRouteStatus,
    safetyBlackoutEngaged: props.safetyBlackoutEngaged,
  });
  const exactRoute = () => isExactShowArtNetLoopbackRoute(props.output);
  const artNetUnityMirrorEnabled = () => exactRoute() && props.output.enabled;
  const persistedBindingKey = () => {
    const identity = props.serialDmxMachineBindingStatus?.selected;
    return identity ? `${identity.port_name}\u0000${identity.windows_device_instance_id ?? identity.macos_device_instance_id ?? ""}` : "";
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
  const showDmxPreparationStatus = () => props.showDmxPreparationStage !== null
    ? showDmxStageLabel(props.showDmxPreparationStage)
    : props.showDmxPreparationBusy ? "Checking…" : "Ready to prepare DMX output";
  const routeState = () => !exactRoute()
    ? "Logical route mismatch"
    : props.output.enabled ? "Enabled" : "Staged disabled";
  const routeStateTone = () => !exactRoute() ? "error" : props.output.enabled ? "ready" : "idle";

  return (
    <section class="dmxOutputConfigPanel ioConnectionDesk" data-io-default-surface="dmx">
      <header class="ioDeskHeader">
        <div><h2>DMX universes</h2><span>USB-DMX · Art-Net · sACN</span></div>
        <div class="dmxUniverseHeaderStates">
          <span class={`ioConnectionState ${routeStateTone()}`}><i aria-hidden="true" />{routeState()}</span>
          <span class={`ioConnectionState ${props.safetyBlackoutEngaged ? "error" : "idle"}`} data-dmx-safety-state>
            <i aria-hidden="true" />{props.safetyBlackoutEngaged ? "S0 engaged" : "S0 clear"}
          </span>
        </div>
      </header>

      <table class="dmxUniverseTable" data-dmx-universe-table>
        <thead>
          <tr>
            <th scope="col">Universe</th>
            <th scope="col">Transport</th>
            <th scope="col">Device / destination</th>
            <th scope="col">State</th>
            <th scope="col">Actions</th>
          </tr>
        </thead>
        <tbody class="dmxUniverseGroup">
          <tr class="dmxShowSetup" data-io-show-dmx-setup data-dmx-universe-row="show-dmx">
            <th scope="row" class="tabularNums" data-no-localize>U0</th>
            <td>
              <div class="dmxShowSetupCopy">
                <strong>Show DMX</strong>
                <span>USB-DMX + Art-Net</span>
              </div>
            </td>
            <td>
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
            </td>
            <td>
              <div class="dmxShowSetupState">
                <span class={`ioConnectionState ${showState().tone}`} data-dmx-show-state title={showState().label}>
                  <i aria-hidden="true" />{showState().label}
                </span>
                <p id="dmx-show-dmx-setup-status" class="dmxShowSetupStatus ioDisclosureDescription" role="status" data-io-show-dmx-setup-status>
                  {showDmxPreparationStatus()}
                </p>
              </div>
            </td>
            <td>
              <div class="dmxUniverseActions">
                <button
                  data-io-control="dmx-prepare-show-dmx"
                  class="primary"
                  disabled={!showDmxPreparationCanStart()}
                  aria-describedby="dmx-show-dmx-setup-status"
                  onClick={() => {
                    void props.onPrepareShowDmx(selectedSerialPort());
                  }}
                >{props.showDmxPreparationBusy ? "Preparing…" : "Prepare"}</button>
              </div>
            </td>
          </tr>
        </tbody>
        <UsbDmxCableRows model={usb} />
        <NetworkDmxRouteRows draft={network} telemetry={props.routeTelemetry} />
        <DmxUniverseAddRow usb={usb} network={network} />
      </table>

      <div class="dmxUniverseFooter">
        <Show when={network.pageCount() > 1}>
          <nav class="buttonRow" aria-label="Network route pages">
            <button type="button" disabled={network.activePage() === 0} onClick={() => network.setPage(network.activePage() - 1)}>Previous</button>
            <span class="tabularNums" data-no-localize>{network.activePage() + 1} / {network.pageCount()}</span>
            <button type="button" disabled={network.activePage() + 1 >= network.pageCount()} onClick={() => network.setPage(network.activePage() + 1)}>Next</button>
          </nav>
        </Show>
        <div class="buttonRow dmxUniverseApply">
          <button
            type="button"
            class="primary"
            data-network-dmx-save
            disabled={!network.canApply()}
            onClick={() => void network.save()}
          >{network.busy() ? "Saving…" : "Apply network routes"}</button>
          <button type="button" disabled={network.busy()} onClick={() => network.reset()}>Reload routes</button>
        </div>
        <Show when={network.dirty()}>
          <p class="hint" data-network-dmx-unapplied>Network changes are not applied yet.</p>
        </Show>
        <Show when={!network.showRoutePrepared()}>
          <p class="hint">Prepare U0 output before applying network universes.</p>
        </Show>
        <Show when={network.conflict()}>
          <p role="alert">Routes changed while editing. Reload routes before applying.</p>
        </Show>
        <Show when={network.error()}>
          <p class="fieldError" role="alert" data-no-localize>{network.error()}</p>
        </Show>
        <Show when={props.usbDmxUniverses === null || props.usbDmxUniversesStale}>
          <p class="hint" role="status" data-usb-dmx-cables-unavailable>
            {props.usbDmxUniverses === null
              ? "USB-DMX cable status is unavailable; cables cannot be changed."
              : "USB-DMX cable status could not be refreshed; showing the last known state. Stop stays available."}
          </p>
        </Show>
        <Show when={props.usbDmxUniverses?.persistenceError}>
          {(error) => <p class="fieldError" role="alert" data-no-localize>{error()}</p>}
        </Show>
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
                  disabled={!serialWorkerArmAdmissible() || !serialWorkerStatusKnown() || !bindingReady() || !artNetUnityMirrorEnabled()}
                  onClick={() => void props.onEnableShowSerialDmxSafetyBlackoutRoute()}
                >Start USB-DMX</button>
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
