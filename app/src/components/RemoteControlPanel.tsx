import { For } from "solid-js";

interface RemoteControlPanelProps {
  bindIp: string;
  port: number;
  running: boolean;
  remoteUrls: string[];
  onBindIp: (value: string) => void;
  onPort: (value: number) => void;
  onCopyRemoteUrl: (url: string) => void | Promise<void>;
  onOpenRemoteUrl: (url: string) => void | Promise<void>;
  onStart: () => void | Promise<void>;
  onStop: () => void | Promise<void>;
}

export function RemoteControlPanel(props: RemoteControlPanelProps) {
  return (
    <div class="remoteControl">
      <section class="remoteServerDesk">
        <header class="ioDeskHeader">
          <h2>Web Remote</h2>
          <span>{props.running ? "Running" : "Stopped"}</span>
        </header>
        <div class="split">
          <label>
            Bind IP
            <input value={props.bindIp} disabled={props.running} onInput={(event) => props.onBindIp(event.currentTarget.value)} />
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
        <div class="buttonRow">
          <button class="primary" onClick={() => void props.onStart()} disabled={props.running}>
            Start Remote
          </button>
          <button onClick={() => void props.onStop()} disabled={!props.running}>
            Stop Remote
          </button>
        </div>
      </section>
      <section class="remoteEndpointDesk">
        <header class="ioDeskHeader">
          <h2>Endpoints</h2>
          <span>{props.remoteUrls.length}</span>
        </header>
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
      </section>
    </div>
  );
}
