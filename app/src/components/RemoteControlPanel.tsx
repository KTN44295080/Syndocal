import { For, Show } from "solid-js";
import type { RemoteControlStatus } from "../types";
import { StandbySyncPanel } from "./StandbySyncPanel";

interface RemoteControlPanelProps {
  backendAvailable: boolean;
  invokeCommand: <T>(command: string, args?: Record<string, unknown>) => Promise<T>;
  bindIp: string;
  port: number;
  pairingPin: string;
  allowLan: boolean;
  maxConnections: number;
  maxMessageBytes: number;
  maxMessagesPerSecond: number;
  running: boolean;
  remoteUrls: string[];
  status: RemoteControlStatus;
  onBindIp: (value: string) => void;
  onPort: (value: number) => void;
  onPairingPin: (value: string) => void;
  onRegeneratePairingPin: () => void;
  onAllowLan: (value: boolean) => void;
  onMaxConnections: (value: number) => void;
  onMaxMessageBytes: (value: number) => void;
  onMaxMessagesPerSecond: (value: number) => void;
  onCopyRemoteUrl: (url: string) => void | Promise<void>;
  onOpenRemoteUrl: (url: string) => void | Promise<void>;
  onStart: () => void | Promise<void>;
  onStop: () => void | Promise<void>;
  onDisconnectClient: (clientId: number) => void | Promise<void>;
}

export function RemoteControlPanel(props: RemoteControlPanelProps) {
  return (
    <div class="remoteControl ioOperatorSurface">
      <section class="remoteServerDesk ioConnectionDesk" data-io-default-surface="remote">
        <header class="ioDeskHeader">
          <div>
            <h2>Web Remote</h2>
            <span>PIN-protected operator access</span>
          </div>
          <span class={`ioConnectionState ${props.running ? "ok" : "idle"}`}><i aria-hidden="true" />{props.running ? "Running" : "Stopped"}</span>
        </header>
        <div class="ioConnectionControls remoteConnectionControls">
          <label>
            Bind IP
            <input
              data-io-control="remote-bind-ip"
              value={props.bindIp}
              disabled={props.running || !props.allowLan}
              onInput={(event) => props.onBindIp(event.currentTarget.value)}
            />
          </label>
          <label>
            Port
            <input
              data-io-control="remote-port"
              type="number"
              min="1"
              value={props.port}
              disabled={props.running}
              onInput={(event) => props.onPort(Number(event.currentTarget.value))}
            />
          </label>
          <label class="remoteLanToggle">
            <input
              data-io-control="remote-lan"
              type="checkbox"
              checked={props.allowLan}
              disabled={props.running}
              onChange={(event) => props.onAllowLan(event.currentTarget.checked)}
            />
            Trusted LAN access
          </label>
          <label class="remotePinField">
            Pairing PIN
            <div class="remoteUrlActions">
              <input
                data-io-control="remote-pin"
                inputmode="numeric"
                maxlength="6"
                pattern="[0-9]{6}"
                value={props.pairingPin}
                disabled={props.running}
                onInput={(event) => props.onPairingPin(event.currentTarget.value.replace(/\D/g, "").slice(0, 6))}
              />
              <button data-io-control="remote-new-pin" onClick={props.onRegeneratePairingPin} disabled={props.running}>New PIN</button>
            </div>
          </label>
          <Show when={props.running} fallback={
            <button data-io-control="remote-start" class="primary" onClick={() => void props.onStart()}>Start Remote</button>
          }>
            <button data-io-control="remote-stop" onClick={() => void props.onStop()}>Stop Remote</button>
          </Show>
        </div>
        <p class={props.allowLan ? "inlineWarning" : "inlineSuccess"}>
          {props.allowLan
            ? "HTTP is not encrypted. Use only on a dedicated trusted LAN; never expose this port to the internet."
            : "Local-only mode: connections are restricted to this computer."}
        </p>
      </section>

      <div class="ioDisclosureStack">
        <details class="ioDisclosure" data-io-disclosure="remote-security">
          <summary>Security limits</summary>
          <div class="ioDisclosureBody" data-io-disclosure-body>
            <p class="hint">Share only the PIN-protected endpoint with trusted operators.</p>
            <div class="split remoteLimitGrid">
              <label>
                Max Clients
                <input
                  data-io-control="remote-max-clients"
                  type="number"
                  min="1"
                  max="64"
                  value={props.maxConnections}
                  disabled={props.running}
                  onInput={(event) => props.onMaxConnections(Number(event.currentTarget.value))}
                />
              </label>
              <label>
                Max Message KiB
                <input
                  type="number"
                  min="1"
                  max="1024"
                  value={Math.round(props.maxMessageBytes / 1024)}
                  disabled={props.running}
                  onInput={(event) => props.onMaxMessageBytes(Number(event.currentTarget.value) * 1024)}
                />
              </label>
              <label>
                Messages / sec
                <input
                  type="number"
                  min="1"
                  max="1000"
                  value={props.maxMessagesPerSecond}
                  disabled={props.running}
                  onInput={(event) => props.onMaxMessagesPerSecond(Number(event.currentTarget.value))}
                />
              </label>
            </div>
          </div>
        </details>

        <details class="ioDisclosure" data-io-disclosure="remote-endpoints">
          <summary>Endpoints and clients</summary>
          <div class="ioDisclosureBody remoteEndpointGrid" data-io-disclosure-body>
            <section class="remoteEndpointDesk">
              <header class="ioDeskHeader">
                <h2>Endpoints</h2>
                <span>{props.remoteUrls.length}</span>
              </header>
              <div class="remoteUrlList">
                <For each={props.remoteUrls}>
                  {(url) => (
                    <div class="remoteUrlItem">
                      <strong data-no-localize>{url}</strong>
                      <div class="remoteUrlActions">
                        <button data-io-control="remote-copy-url" onClick={() => void props.onCopyRemoteUrl(url)}>Copy</button>
                        <button class="remoteUrlOpenButton" onClick={() => void props.onOpenRemoteUrl(url)}>Open</button>
                      </div>
                    </div>
                  )}
                </For>
              </div>
            </section>
            <section class="remoteEndpointDesk remoteClientDesk">
              <header class="ioDeskHeader">
                <h2>Connected Clients</h2>
                <span>{props.status.active_connections} active / {props.status.rejected_connections} rejected</span>
              </header>
              <Show when={props.status.clients.length > 0} fallback={<p class="emptyHint">No remote clients connected.</p>}>
                <div class="remoteUrlList">
                  <For each={props.status.clients}>
                    {(client) => (
                      <div class="remoteUrlItem remoteClientItem">
                        <div>
                          <strong data-no-localize>{client.peer_addr}</strong>
                          <small>Client #{client.id} · {client.messages_received} message(s)</small>
                        </div>
                        <button onClick={() => void props.onDisconnectClient(client.id)}>Disconnect</button>
                      </div>
                    )}
                  </For>
                </div>
              </Show>
            </section>
          </div>
        </details>

        <details class="ioDisclosure" data-io-disclosure="remote-standby">
          <summary>Active / Standby sync</summary>
          <div class="ioDisclosureBody" data-io-disclosure-body>
            <StandbySyncPanel backendAvailable={props.backendAvailable} invokeCommand={props.invokeCommand} />
          </div>
        </details>
      </div>
    </div>
  );
}
