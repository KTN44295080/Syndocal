import { For, Show, createSignal } from "solid-js";
import type { FrontendTauriInvoke } from "../tauriInvokeCommands";
import type {
  DjLinkMachineStatus,
  DjLinkWiredCandidate,
  DjTrackTriggerMapping,
  RemoteControlStatus,
} from "../types";
import { StandbySyncPanel } from "./StandbySyncPanel";
import { djLinkMachineBlockReasonText } from "../uiLocalization";

interface RemoteControlPanelProps {
  backendAvailable: boolean;
  invokeCommand: FrontendTauriInvoke;
  bindIp: string;
  port: number;
  pairingPin: string;
  allowLan: boolean;
  maxConnections: number;
  maxMessageBytes: number;
  maxMessagesPerSecond: number;
  djLinkEnabled: boolean;
  djLinkMachineStatus: DjLinkMachineStatus;
  djLinkWiredCandidates: DjLinkWiredCandidate[];
  djLinkSelectedBinding: string;
  djLinkToken: string | null;
  djLinkTokenCopied: boolean;
  djTrackTriggers: DjTrackTriggerMapping[];
  timelineOptions: { id: number; label: string }[];
  listenerRunning: boolean;
  listenerStatusHydrated: boolean;
  genericRunning: boolean;
  djListenerRunning: boolean;
  djLinkTokenOperationBusy: boolean;
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
  onDjLinkSelectedBinding: (value: string) => void;
  onRefreshDjLinkWiredCandidates: () => void | Promise<unknown>;
  onArmDjLinkMachine: () => void | Promise<void>;
  onDisarmDjLinkMachine: () => void | Promise<void>;
  onRotateDjLinkToken: () => void | Promise<void>;
  onCopyDjLinkToken: () => void | Promise<void>;
  onDjTrackTriggers: (value: DjTrackTriggerMapping[]) => void;
  onCopyRemoteUrl: (url: string) => void | Promise<void>;
  onOpenRemoteUrl: (url: string) => void | Promise<void>;
  onStart: () => void | Promise<void>;
  onStop: () => void | Promise<void>;
  onDisconnectClient: (clientId: number) => void | Promise<void>;
}

export function RemoteControlPanel(props: RemoteControlPanelProps) {
  const [selectorMode, setSelectorMode] = createSignal<"content" | "title_artist">("content");
  const [contentId, setContentId] = createSignal("");
  const [title, setTitle] = createSignal("");
  const [artist, setArtist] = createSignal("");
  const [timelineId, setTimelineId] = createSignal<number | null>(null);
  const [editingId, setEditingId] = createSignal<string | null>(null);
  const [mappingError, setMappingError] = createSignal<string | null>(null);
  const linkStatus = () => {
    if (!props.djListenerRunning) return null;
    const status = props.status.dj_link;
    return status && status.available !== false ? status : null;
  };
  const machineStatusKnown = () => props.djLinkMachineStatus.blockReason !== "machine_status_unavailable";
  const hasDjLinkDisarmWork = () => props.djLinkMachineStatus.autoStartArmed
    || props.djLinkMachineStatus.credentialGeneration !== null
    || props.djLinkMachineStatus.credentialCleanupPending
    || props.djLinkMachineStatus.blockReason === "disarm_cleanup_pending";
  const resetMappingDraft = () => {
    setSelectorMode("content");
    setContentId("");
    setTitle("");
    setArtist("");
    setTimelineId(null);
    setEditingId(null);
    setMappingError(null);
  };
  const useCurrentTrack = () => {
    const current = linkStatus();
    if (!current?.trackContentId && !(current?.trackTitle && current?.trackArtist)) {
      setMappingError("No current DJ Link track metadata is available.");
      return;
    }
    if (current.trackContentId) {
      setSelectorMode("content");
      setContentId(current.trackContentId);
    } else {
      setSelectorMode("title_artist");
      setTitle(current.trackTitle ?? "");
      setArtist(current.trackArtist ?? "");
    }
    setMappingError(null);
  };
  const editMapping = (mapping: DjTrackTriggerMapping) => {
    if (mapping.selector.contentId) {
      setSelectorMode("content");
      setContentId(mapping.selector.contentId);
    } else {
      setSelectorMode("title_artist");
      setContentId("");
      setTitle(mapping.selector.title ?? "");
      setArtist(mapping.selector.artist ?? "");
    }
    setTimelineId(mapping.timelineId);
    setEditingId(mapping.id);
    setMappingError(null);
  };
  const saveMapping = () => {
    const targetTimeline = timelineId();
    const content = contentId().trim();
    const trackTitle = title().trim();
    const trackArtist = artist().trim();
    if (!Number.isSafeInteger(targetTimeline) || targetTimeline === null || targetTimeline <= 0) {
      setMappingError("Choose an authored Timeline target.");
      return;
    }
    if (selectorMode() === "content" && !content) {
      setMappingError("Enter a Content ID, or choose exact Title + Artist.");
      return;
    }
    if (selectorMode() === "title_artist" && (!trackTitle || !trackArtist)) {
      setMappingError("Exact Title and Artist are both required.");
      return;
    }
    const id = editingId() ?? (globalThis.crypto?.randomUUID?.() ?? `dj-${Date.now().toString(36)}`);
    const nextMapping: DjTrackTriggerMapping = {
      id,
      selector: selectorMode() === "content"
        ? { contentId: content, title: null, artist: null }
        : { contentId: null, title: trackTitle, artist: trackArtist },
      timelineId: targetTimeline,
      retrigger: "once_per_play_session",
    };
    const next = editingId()
      ? props.djTrackTriggers.map((mapping) => mapping.id === id ? nextMapping : mapping)
      : [...props.djTrackTriggers, nextMapping];
    props.onDjTrackTriggers(next);
    resetMappingDraft();
  };
  const removeMapping = (id: string) => {
    if (!globalThis.confirm("Remove this DJ Link mapping?")) return;
    props.onDjTrackTriggers(props.djTrackTriggers.filter((mapping) => mapping.id !== id));
    if (editingId() === id) resetMappingDraft();
  };
  return (
    <div class="remoteControl ioOperatorSurface">
      <div class="ioDisclosureStack">
        <details class="ioDisclosure" data-io-disclosure="web-remote" data-io-default-surface="remote">
          <summary>Web Remote</summary>
          <div class="ioDisclosureBody" data-io-disclosure-body>
            <section class="remoteServerDesk ioConnectionDesk">
          <header class="ioDeskHeader">
            <div>
              <h2>Web Remote</h2>
              <span>PIN-protected operator access</span>
            </div>
            <span class={`ioConnectionState ${props.genericRunning ? "ok" : "idle"}`}><i aria-hidden="true" />{props.genericRunning ? "Running" : "Stopped"}</span>
          </header>
          <div class="ioConnectionControls remoteConnectionControls">
            <details class="ioDisclosure" data-io-disclosure="remote-connection-settings">
              <summary>Connection and access settings</summary>
              <div class="ioDisclosureBody">
                <label>
                  Bind IP
                  <input
                    data-io-control="remote-bind-ip"
                    value={props.bindIp}
                    disabled={props.listenerRunning || !props.allowLan}
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
                    disabled={props.listenerRunning || props.djLinkMachineStatus.autoStartArmed}
                    onInput={(event) => props.onPort(Number(event.currentTarget.value))}
                  />
                </label>
                <label class="remoteLanToggle">
                  <input
                    data-io-control="remote-lan"
                    type="checkbox"
                    checked={props.allowLan}
                    disabled={props.listenerRunning}
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
                      disabled={props.listenerRunning}
                      onInput={(event) => props.onPairingPin(event.currentTarget.value.replace(/\D/g, "").slice(0, 6))}
                    />
                    <button data-io-control="remote-new-pin" onClick={props.onRegeneratePairingPin} disabled={props.listenerRunning}>New PIN</button>
                  </div>
                </label>
              </div>
            </details>
            <Show when={props.genericRunning} fallback={
              <Show
                when={props.djListenerRunning}
                fallback={
                  <Show
                    when={props.listenerStatusHydrated}
                    fallback={<button data-io-control="remote-start" disabled>Checking listener…</button>}
                  >
                    <button data-io-control="remote-start" class="primary" onClick={() => void props.onStart()}>Start Remote</button>
                  </Show>
                }
              >
                <button data-io-control="remote-start" disabled>DJ Link listener active</button>
              </Show>
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
          </div>
        </details>
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
                  disabled={props.listenerRunning}
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
                  disabled={props.listenerRunning}
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
                  disabled={props.listenerRunning}
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
              <Show when={props.genericRunning} fallback={<p class="emptyHint">DJ Link is listener-only; no Web Remote URL is available.</p>}>
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
              </Show>
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

        <details class="ioDisclosure" data-io-disclosure="dj-link">
          <summary>DJ Link</summary>
          <div class="ioDisclosureBody remoteEndpointGrid" data-io-disclosure-body>
            <section class="remoteEndpointDesk">
              <header class="ioDeskHeader">
                <div>
                  <h2>DJ Link authority</h2>
                  <span>Dedicated authenticated WebSocket for track-trigger mappings.</span>
                </div>
                <span class={`ioConnectionState ${linkStatus()?.connected ? "ok" : "idle"}`}>
                  <i aria-hidden="true" />{linkStatus()?.connected ? "Connected" : "Unavailable"}
                </span>
              </header>
              <div class="ioConnectionControls remoteConnectionControls">
                <label>
                  Wired binding
                  <select
                    data-io-control="dj-link-wired-binding"
                    value={props.djLinkSelectedBinding}
                    disabled={props.listenerRunning || props.djLinkTokenOperationBusy || !machineStatusKnown()}
                    onChange={(event) => props.onDjLinkSelectedBinding(event.currentTarget.value)}
                  >
                    <option value="">Select one exact wired binding</option>
                    <For each={props.djLinkWiredCandidates}>
                      {(candidate) => {
                        const key = `${candidate.networkGuid}\u001f${candidate.adapterGuid}\u001f${candidate.bindIp}`;
                        return <option data-no-localize value={key}>{candidate.adapterAlias ?? "Ethernet"} · {candidate.bindIp}</option>;
                      }}
                    </For>
                  </select>
                </label>
                <button
                  type="button"
                  data-io-control="dj-link-refresh-wired-candidates"
                  disabled={props.listenerRunning || props.djLinkTokenOperationBusy || !machineStatusKnown()}
                  onClick={() => void props.onRefreshDjLinkWiredCandidates()}
                >Refresh wired bindings</button>
                <Show when={hasDjLinkDisarmWork()} fallback={
                  <button
                    type="button"
                    data-io-control="dj-link-arm"
                    disabled={props.listenerRunning || !props.listenerStatusHydrated || props.djLinkTokenOperationBusy || !machineStatusKnown() || !props.djLinkSelectedBinding}
                    onClick={() => void props.onArmDjLinkMachine()}
                  >Arm DJ Link</button>
                }>
                  <button
                    type="button"
                    class="danger"
                    data-io-control="dj-link-disarm"
                    disabled={props.djLinkTokenOperationBusy}
                    onClick={() => void props.onDisarmDjLinkMachine()}
                  >Disarm DJ Link</button>
                </Show>
                <button
                  type="button"
                  data-io-control="dj-link-rotate-token"
                  disabled={props.listenerRunning || !props.listenerStatusHydrated || props.djLinkTokenOperationBusy || !machineStatusKnown() || !props.djLinkEnabled}
                  onClick={() => void props.onRotateDjLinkToken()}
                >Rotate token</button>
              </div>
              <p class={props.djLinkMachineStatus.blockReason ? "inlineWarning" : "hint"}>
                {props.djLinkMachineStatus.autoStartArmed
                  ? `Endpoint: ws://${props.djLinkMachineStatus.bindIp ?? "[select LAN IP]"}:${props.djLinkMachineStatus.bindPort ?? "[select port]"}/dj-link`
                  : "DJ Link is disabled. It never uses the Web Remote pairing PIN."}
              </p>
              <Show when={props.djLinkMachineStatus.blockReason}>
                {(reason) => <p class="inlineWarning">{djLinkMachineBlockReasonText(reason())}</p>}
              </Show>
              <Show when={props.djLinkMachineStatus.credentialCleanupPending}>
                <p class="inlineWarning">DJ Link credential cleanup is pending and will be retried at the next launch.</p>
              </Show>
              <Show when={props.djLinkToken}>
                {(token) => (
                  <div class="remoteUrlItem">
                    <strong>Show-once token</strong>
                    <code data-no-localize>{token()}</code>
                    <button type="button" onClick={() => void props.onCopyDjLinkToken()}>Copy token</button>
                  </div>
                )}
              </Show>
              <Show when={props.djLinkTokenCopied}>
                <p class="inlineSuccess">Token copied and cleared.</p>
              </Show>
              <Show when={linkStatus()} fallback={<p class="emptyHint">DJ Link status is unavailable while its listener is stopped.</p>}>
                {(status) => (
                  <div class="split remoteLimitGrid" data-dj-link-status>
                    <span>Available <strong>{status().available === false ? "No" : "Yes"}</strong></span>
                    <span>Enabled <strong>{props.djLinkMachineStatus.autoStartArmed ? "Yes" : "No"}</strong></span>
                    <span>Peer <strong data-no-localize>{status().peer ?? "—"}</strong></span>
                    <span>Generation <strong class="tabularNums" data-no-localize>{status().generation}</strong></span>
                    <span>Heartbeat <strong class="tabularNums" data-no-localize>{status().ageMs ?? "—"} ms</strong></span>
                    <span>Owner deck / playing <strong data-no-localize>{status().ownerDeck ?? "—"} / {status().trackPlaying ? "Yes" : "No"}</strong></span>
                    <span>Track <strong data-no-localize>{status().trackTitle ?? "—"} · {status().trackArtist ?? "—"}</strong></span>
                    <span>Content ID <strong data-no-localize>{status().trackContentId ?? "—"}</strong></span>
                    <span>Loop / released <strong data-no-localize>{status().loopDivision ?? "—"} / {status().released ? "Yes" : "No"}</strong></span>
                    <span>Outcome <strong data-no-localize>{status().outcome ?? "—"}</strong></span>
                    <span>Last event <strong data-no-localize>{status().lastEventId ?? "—"}</strong></span>
                  </div>
                )}
              </Show>
            </section>

            <section class="remoteEndpointDesk">
              <header class="ioDeskHeader">
                <div>
                  <h2>Track mappings</h2>
                  <span>Exact Content ID or exact Title + Artist; each starts one authored Timeline.</span>
                </div>
                <span class="tabularNums">{props.djTrackTriggers.length} / 128</span>
              </header>
              <div class="remoteConnectionControls">
                <label>
                  Selector
                  <select value={selectorMode()} onChange={(event) => setSelectorMode(event.currentTarget.value as "content" | "title_artist")}>
                    <option value="content">Content ID</option>
                    <option value="title_artist">Exact Title + Artist</option>
                  </select>
                </label>
                <Show when={selectorMode() === "content"}>
                  <label>Content ID<input data-no-localize value={contentId()} onInput={(event) => setContentId(event.currentTarget.value)} /></label>
                </Show>
                <Show when={selectorMode() === "title_artist"}>
                  <label>Exact title<input data-no-localize value={title()} onInput={(event) => setTitle(event.currentTarget.value)} /></label>
                  <label>Exact artist<input data-no-localize value={artist()} onInput={(event) => setArtist(event.currentTarget.value)} /></label>
                </Show>
                <label>
                  Start Timeline
                  <select value={timelineId() ?? ""} onChange={(event) => setTimelineId(event.currentTarget.value ? Number(event.currentTarget.value) : null)}>
                    <option value="">Choose authored Timeline</option>
                    <For each={props.timelineOptions}>
                      {(timeline) => <option data-no-localize value={timeline.id}>{timeline.label} ({timeline.id})</option>}
                    </For>
                  </select>
                </label>
                <div class="remoteUrlActions">
                  <button type="button" onClick={useCurrentTrack} disabled={!linkStatus()?.trackContentId && !(linkStatus()?.trackTitle && linkStatus()?.trackArtist)}>Use Current Track</button>
                  <button type="button" class="primary" onClick={saveMapping}>{editingId() ? "Update mapping" : "Add mapping"}</button>
                  <Show when={editingId()}><button type="button" onClick={resetMappingDraft}>Cancel edit</button></Show>
                </div>
                <Show when={mappingError()}>{(error) => <p class="inlineWarning">{error()}</p>}</Show>
              </div>
              <Show when={props.djTrackTriggers.length > 0} fallback={<p class="emptyHint">No DJ Link mappings. Add one exact selector.</p>}>
                <div class="remoteUrlList">
                  <For each={props.djTrackTriggers}>
                    {(mapping) => (
                      <div class="remoteUrlItem remoteClientItem">
                        <div>
                          <strong data-no-localize>{mapping.selector.contentId ?? `${mapping.selector.title} · ${mapping.selector.artist}`}</strong>
                          <small>
                            Start Timeline <span data-no-localize>{mapping.timelineId}</span>
                            <Show when={!props.timelineOptions.some((timeline) => timeline.id === mapping.timelineId)}>
                              <span class="inlineWarning"> · Missing authored Timeline</span>
                            </Show>
                            <Show when={props.timelineOptions.some((timeline) => timeline.id === mapping.timelineId)}>
                              <span> · Once per play session</span>
                            </Show>
                          </small>
                        </div>
                        <div class="remoteUrlActions">
                          <button type="button" disabled={!props.timelineOptions.some((timeline) => timeline.id === mapping.timelineId)} onClick={() => editMapping(mapping)}>Edit</button>
                          <button type="button" class="danger" onClick={() => removeMapping(mapping.id)}>Remove</button>
                        </div>
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
