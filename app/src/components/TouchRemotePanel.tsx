import { For, Show } from "solid-js";
import type { RemoteControlStatus, SubmasterSummary } from "../types";

interface TouchRemotePanelProps {
  running: boolean;
  remoteUrls: string[];
  bindIp: string;
  port: number;
  pairingPin: string;
  allowLan: boolean;
  status: RemoteControlStatus;
  bpmDraft: string;
  submasters: SubmasterSummary[];
  onBindIp: (value: string) => void;
  onPort: (value: number) => void;
  onPairingPin: (value: string) => void;
  onRegeneratePairingPin: () => void;
  onAllowLan: (value: boolean) => void;
  onCopyRemoteUrl: (url: string) => void | Promise<void>;
  onOpenRemoteUrl: (url: string) => void | Promise<void>;
  onStart: () => void | Promise<void>;
  onStop: () => void | Promise<void>;
  onBpmDraft: (value: string) => void;
  onApplyBpm: () => void | Promise<void>;
  onTapBpm: () => void | Promise<void>;
  onSetSubmaster: (groupId: string, level: number) => void | Promise<void>;
}

export function TouchRemotePanel(props: TouchRemotePanelProps) {
  return (
    <section class="panel touchPanel touchRemotePanel">
      <div class="panelHeader">
        <h2>Touch Remote</h2>
        <span>{props.running ? `Running · ${props.status.active_connections} client(s)` : "Stopped"}</span>
      </div>
      <div class="touchRemoteUrl">
        <span>Remote URLs</span>
        <div class="remoteUrlList">
          <For each={props.remoteUrls}>
            {(url) => (
              <div class="remoteUrlItem">
                <strong>{url}</strong>
                <div class="remoteUrlActions">
                  <button onClick={() => void props.onCopyRemoteUrl(url)}>Copy</button>
                  <button class="remoteUrlOpenButton" onClick={() => void props.onOpenRemoteUrl(url)}>
                    Open
                  </button>
                </div>
              </div>
            )}
          </For>
        </div>
      </div>
      <div class="split">
        <label>
          Bind IP
          <input value={props.bindIp} disabled={props.running || !props.allowLan} onInput={(event) => props.onBindIp(event.currentTarget.value)} />
        </label>
        <label>
          Port
          <input
            type="number"
            min="1"
            value={props.port}
            disabled={props.running}
            onInput={(event) => props.onPort(Number(event.currentTarget.value))}
          />
        </label>
      </div>
      <label class="remoteLanToggle">
        <input
          type="checkbox"
          checked={props.allowLan}
          disabled={props.running}
          onChange={(event) => props.onAllowLan(event.currentTarget.checked)}
        />
        Trusted LAN access (unencrypted HTTP)
      </label>
      <label>
        Pairing PIN
        <div class="remoteUrlActions">
          <input
            inputmode="numeric"
            maxlength="6"
            pattern="[0-9]{6}"
            value={props.pairingPin}
            disabled={props.running}
            onInput={(event) => props.onPairingPin(event.currentTarget.value.replace(/\D/g, "").slice(0, 6))}
          />
          <button onClick={props.onRegeneratePairingPin} disabled={props.running}>New PIN</button>
        </div>
      </label>
      <div class="touchGuardRow">
        <button class="primary" onClick={() => void props.onStart()} disabled={props.running}>
          Start Remote
        </button>
        <button onClick={() => void props.onStop()} disabled={!props.running}>
          Stop Remote
        </button>
      </div>
      <div class="touchMasterGrid compact">
        <label>
          BPM
          <input
            type="number"
            min="20"
            max="300"
            step="0.1"
            value={props.bpmDraft}
            onInput={(event) => props.onBpmDraft(event.currentTarget.value)}
          />
        </label>
        <button onClick={() => void props.onApplyBpm()}>Set BPM</button>
        <button class="primary" onClick={() => void props.onTapBpm()}>
          Tap
        </button>
      </div>
      <Show when={props.submasters.length > 0}>
        <div class="submasterList">
          <h3>Submasters</h3>
          <For each={props.submasters}>
            {(submaster) => (
              <label class="submasterControl">
                <span data-no-localize>{submaster.label}</span>
                <input
                  type="range"
                  min="0"
                  max="1"
                  step="0.01"
                  value={submaster.level}
                  onInput={(event) => void props.onSetSubmaster(submaster.group_id, Number(event.currentTarget.value))}
                />
                <strong>{Math.round(submaster.level * 100)}%</strong>
              </label>
            )}
          </For>
        </div>
      </Show>
    </section>
  );
}
