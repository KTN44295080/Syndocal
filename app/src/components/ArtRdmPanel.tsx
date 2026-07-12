import { createEffect, createSignal, onCleanup, Show } from "solid-js";
import type { ArtRdmRequest, ArtRdmResponse, SerialPortSummary, UsbRdmRequest } from "../types";

interface ArtRdmPanelProps {
  gatewayIp: string;
  portAddress: number;
  serialPorts: SerialPortSummary[];
  onRequest: (request: ArtRdmRequest) => Promise<ArtRdmResponse>;
  onUsbRequest: (request: UsbRdmRequest) => Promise<ArtRdmResponse>;
  onUsbDiscover: (serialPort: string, sourceUid: string) => Promise<string[]>;
  onRefreshSerialPorts: () => void | Promise<void>;
  onDiscover: (gatewayIp: string, portAddress: number) => Promise<string[]>;
  onStartFullDiscovery: (gatewayIp: string, portAddress: number) => Promise<void>;
}

const parseHexU16 = (value: string) => {
  const normalized = value.trim().replace(/^0x/i, "");
  const parsed = Number.parseInt(normalized, 16);
  return Number.isFinite(parsed) ? Math.max(0, Math.min(0xffff, parsed)) : 0;
};

export function ArtRdmPanel(props: ArtRdmPanelProps) {
  const [gatewayIp, setGatewayIp] = createSignal(props.gatewayIp);
  const [portAddress, setPortAddress] = createSignal(props.portAddress);
  const [transport, setTransport] = createSignal<"ArtNet" | "UsbPro">("ArtNet");
  const [serialPort, setSerialPort] = createSignal("");
  const [sourceUid, setSourceUid] = createSignal("7FFF:00000001");
  const [targetUid, setTargetUid] = createSignal("");
  const [command, setCommand] = createSignal<"Get" | "Set">("Get");
  const [parameterId, setParameterId] = createSignal("0060");
  const [parameterData, setParameterData] = createSignal("");
  const [timeoutMs, setTimeoutMs] = createSignal(10_000);
  const [busy, setBusy] = createSignal(false);
  const [discovering, setDiscovering] = createSignal(false);
  const [inventoryRunning, setInventoryRunning] = createSignal(false);
  const [inventoryStatus, setInventoryStatus] = createSignal("");
  const [pendingConfirmation, setPendingConfirmation] = createSignal<"set" | "discovery" | null>(null);
  const [discoveredUids, setDiscoveredUids] = createSignal<string[]>([]);
  const [error, setError] = createSignal("");
  const [response, setResponse] = createSignal<ArtRdmResponse | null>(null);
  let inventoryTimer: number | undefined;
  let confirmationDialog!: HTMLDialogElement;

  onCleanup(() => {
    if (inventoryTimer !== undefined) window.clearInterval(inventoryTimer);
  });

  createEffect(() => {
    if (!gatewayIp().trim() && props.gatewayIp.trim()) setGatewayIp(props.gatewayIp);
  });

  createEffect(() => {
    if (!serialPort() && props.serialPorts.length > 0) setSerialPort(props.serialPorts[0].name);
    if (transport() === "UsbPro" && inventoryRunning()) {
      if (inventoryTimer !== undefined) window.clearInterval(inventoryTimer);
      inventoryTimer = undefined;
      setInventoryRunning(false);
    }
  });

  const executeSend = async () => {
    setBusy(true);
    setError("");
    try {
      const common = {
        source_uid: sourceUid().trim(),
        target_uid: targetUid().trim(),
        command: command(),
        parameter_id: parseHexU16(parameterId()),
        parameter_data_hex: command() === "Get" ? "" : parameterData().trim(),
        timeout_ms: Math.max(50, Math.min(120_000, Math.trunc(timeoutMs()))),
      } as const;
      setResponse(transport() === "UsbPro"
        ? await props.onUsbRequest({
          ...common,
          serial_port: serialPort(),
          serial_baud_rate: 57_600,
        })
        : await props.onRequest({
          ...common,
          gateway_ip: gatewayIp().trim(),
          port_address: Math.max(0, Math.min(0x7fff, Math.trunc(portAddress()))),
        }));
    } catch (requestError) {
      setResponse(null);
      setError(String(requestError));
    } finally {
      setBusy(false);
    }
  };

  const send = () => {
    if (command() !== "Set") {
      void executeSend();
      return;
    }
    setPendingConfirmation("set");
    confirmationDialog.showModal();
  };

  const discover = async () => {
    if (discovering()) return;
    setDiscovering(true);
    setError("");
    try {
      const previous = new Set(discoveredUids());
      const uids = await props.onDiscover(
        gatewayIp().trim(),
        Math.max(0, Math.min(0x7fff, Math.trunc(portAddress()))),
      );
      const current = new Set(uids);
      const added = uids.filter((uid) => !previous.has(uid));
      const removed = [...previous].filter((uid) => !current.has(uid));
      setDiscoveredUids(uids);
      setInventoryStatus(
        previous.size === 0
          ? `${uids.length} device(s) seen · ${new Date().toLocaleTimeString()}`
          : `${added.length} added / ${removed.length} removed · ${new Date().toLocaleTimeString()}`,
      );
      if (uids.length > 0 && !targetUid().trim()) setTargetUid(uids[0]);
    } catch (requestError) {
      setDiscoveredUids([]);
      setError(String(requestError));
    } finally {
      setDiscovering(false);
    }
  };

  const toggleInventory = () => {
    if (inventoryRunning()) {
      if (inventoryTimer !== undefined) window.clearInterval(inventoryTimer);
      inventoryTimer = undefined;
      setInventoryRunning(false);
      return;
    }
    setInventoryRunning(true);
    void discover();
    inventoryTimer = window.setInterval(() => void discover(), 5_000);
  };

  const executeFullDiscovery = async () => {
    setDiscovering(true);
    setError("");
    try {
      if (transport() === "UsbPro") {
        const uids = await props.onUsbDiscover(serialPort(), sourceUid().trim());
        setDiscoveredUids(uids);
        setInventoryStatus(`${uids.length} USB fixture(s) discovered · ${new Date().toLocaleTimeString()}`);
        if (uids.length > 0) setTargetUid(uids[0]);
      } else {
        await props.onStartFullDiscovery(
          gatewayIp().trim(),
          Math.max(0, Math.min(0x7fff, Math.trunc(portAddress()))),
        );
        setDiscoveredUids([]);
      }
    } catch (requestError) {
      setError(String(requestError));
    } finally {
      setDiscovering(false);
    }
  };

  const startFullDiscovery = () => {
    setPendingConfirmation("discovery");
    confirmationDialog.showModal();
  };

  const confirmPendingAction = () => {
    const action = pendingConfirmation();
    confirmationDialog.close();
    setPendingConfirmation(null);
    if (action === "set") void executeSend();
    if (action === "discovery") void executeFullDiscovery();
  };

  return (
    <section class="artRdmPanel" aria-label="RDM console">
      <header class="ioDeskHeader">
        <div>
          <h2>RDM Console</h2>
          <span>{transport() === "ArtNet" ? "Unicast ArtRdm" : "ENTTEC USB Pro Label 7 / 5"}</span>
        </div>
        <span class="status warn">Manual UID</span>
      </header>
      <p class="hint">Enter the gateway IP and discovered fixture UID. Replace the development controller UID with your assigned UID for production hardware.</p>
      <div class="artRdmGrid">
        <label>Transport<select value={transport()} onInput={(event) => setTransport(event.currentTarget.value as "ArtNet" | "UsbPro")}><option value="ArtNet">Art-Net gateway</option><option value="UsbPro">ENTTEC USB Pro</option></select></label>
        <Show when={transport() === "ArtNet"} fallback={
          <div class="videoAudioDeviceField rdmSerialPortField">
            <label>Serial port<select value={serialPort()} onInput={(event) => setSerialPort(event.currentTarget.value)}><option value="">Select port</option>{props.serialPorts.map((port) => <option value={port.name}>{port.name} · {port.port_type}</option>)}</select></label>
            <button aria-label="Refresh serial ports" title="Refresh serial ports" onClick={() => void props.onRefreshSerialPorts()}>↻</button>
          </div>
        }>
          <label>Gateway IP<input value={gatewayIp()} onInput={(event) => setGatewayIp(event.currentTarget.value)} /></label>
          <label>Port-Address<input type="number" min="0" max="32767" value={portAddress()} onInput={(event) => setPortAddress(Number(event.currentTarget.value))} /></label>
        </Show>
        <label>Controller UID<input value={sourceUid()} onInput={(event) => setSourceUid(event.currentTarget.value)} /></label>
        <label>Fixture UID<input placeholder="1234:56789ABC" value={targetUid()} onInput={(event) => setTargetUid(event.currentTarget.value)} /></label>
        <label>Command<select value={command()} onInput={(event) => setCommand(event.currentTarget.value as "Get" | "Set")}><option value="Get">GET</option><option value="Set">SET</option></select></label>
        <label>PID (hex)<input value={parameterId()} onInput={(event) => setParameterId(event.currentTarget.value)} /></label>
        <label>Timeout ms<input type="number" min="50" max="120000" step="50" value={timeoutMs()} onInput={(event) => setTimeoutMs(Number(event.currentTarget.value))} /></label>
      </div>
      <Show when={command() === "Set"}>
        <label>Parameter data (hex bytes)<input placeholder="00 01" value={parameterData()} onInput={(event) => setParameterData(event.currentTarget.value)} /></label>
      </Show>
      <div class="buttonRow">
        <Show when={transport() === "ArtNet"}>
          <button disabled={discovering() || !gatewayIp().trim()} onClick={() => void discover()}>{discovering() ? "Discovering…" : "Read Gateway TOD"}</button>
          <button aria-pressed={inventoryRunning()} disabled={!gatewayIp().trim()} onClick={toggleInventory}>{inventoryRunning() ? "Stop TOD Monitor" : "Monitor TOD"}</button>
          <button disabled={discovering() || !gatewayIp().trim()} onClick={startFullDiscovery}>Force Full Discovery</button>
        </Show>
        <Show when={transport() === "UsbPro"}>
          <button disabled={discovering() || !serialPort()} onClick={startFullDiscovery}>{discovering() ? "Discovering…" : "Discover USB Fixtures"}</button>
        </Show>
        <button onClick={() => { setCommand("Get"); setParameterId("0060"); setParameterData(""); }}>Device Info PID</button>
        <button class="primary" disabled={busy() || !(transport() === "ArtNet" ? gatewayIp().trim() : serialPort()) || !targetUid().trim()} onClick={send}>{busy() ? "Waiting…" : `Send ${command().toUpperCase()}`}</button>
      </div>
      <Show when={discoveredUids().length > 0}>
        <label>Discovered fixture ({discoveredUids().length})
          <select value={targetUid()} onInput={(event) => setTargetUid(event.currentTarget.value)}>
            <option value="">Select UID</option>
            {discoveredUids().map((uid) => <option value={uid}>{uid}</option>)}
          </select>
        </label>
      </Show>
      <Show when={inventoryStatus()}><p class="hint tabularNums" aria-live="polite">{inventoryStatus()}</p></Show>
      <Show when={error()}><p class="validationError" role="alert">{error()}</p></Show>
      <Show when={response()}>
        {(result) => (
          <div class="artRdmResponse">
            <strong>{result().response_type} · {result().command_class} · PID 0x{result().parameter_id.toString(16).toUpperCase().padStart(4, "0")}</strong>
            <span>{result().source_uid} → {result().destination_uid}</span>
            <code>{result().parameter_data_hex || "(empty response)"}</code>
            <span>{result().response_blocks} response block(s) · {result().ack_timer_count} timer deferral(s) · {result().queued_message_polls} queued poll(s)</span>
            <Show when={result().nack_reason !== null && result().nack_reason !== undefined}>
              <span class="validationError">NACK reason 0x{result().nack_reason!.toString(16).toUpperCase().padStart(4, "0")}</span>
            </Show>
            <Show when={transport() === "ArtNet"}><span>Gateway FIFO {result().fifo_available}/{result().fifo_max}</span></Show>
            <Show when={result().device_info}>
              {(info) => <span>DMX {info().dmx_start_address} · {info().dmx_footprint}ch · personality {info().current_personality}/{info().personality_count} · subdevices {info().sub_device_count}</span>}
            </Show>
          </div>
        )}
      </Show>
      <dialog
        ref={confirmationDialog}
        class="rdmConfirmationDialog"
        aria-labelledby="rdm-confirmation-title"
        onClose={() => setPendingConfirmation(null)}
      >
        <form method="dialog">
          <h2 id="rdm-confirmation-title" class="textBalance">
            {pendingConfirmation() === "set" ? "Send RDM SET?" : `Start full RDM discovery over ${transport() === "UsbPro" ? "USB" : "Art-Net"}?`}
          </h2>
          <p class="textPretty">
            {pendingConfirmation() === "set"
              ? "This writes the selected PID to the physical fixture. Verify the target UID and parameter bytes."
              : transport() === "UsbPro"
                ? "This unmutes, searches, and mutes responders over the selected USB Pro port. DMX output on that port must remain disabled until discovery finishes."
                : "This flushes the gateway TOD and may temporarily interrupt DMX on the selected port while physical discovery runs."}
          </p>
          <div class="buttonRow">
            <button value="cancel">Cancel</button>
            <button type="button" class="danger" onClick={confirmPendingAction}>Confirm</button>
          </div>
        </form>
      </dialog>
      <p class="hint">{transport() === "ArtNet"
        ? "Read Gateway TOD is non-disruptive. Force Full Discovery flushes the gateway table and may temporarily interrupt that DMX port. ArtRdm requests remain unicast because broadcast is prohibited."
        : "USB Pro RDM opens the selected COM port exclusively. Disable DMX output on that same port first. The widget must run RDM-capable firmware. Discovery uses Label 11 DISC_UNIQUE_BRANCH and may take up to 60 seconds."}</p>
    </section>
  );
}
