interface RemoteControlPanelProps {
  bindIp: string;
  port: number;
  running: boolean;
  onBindIp: (value: string) => void;
  onPort: (value: number) => void;
  onStart: () => void | Promise<void>;
  onStop: () => void | Promise<void>;
}

export function RemoteControlPanel(props: RemoteControlPanelProps) {
  return (
    <div class="remoteControl">
      <h3>Web Remote</h3>
      <div class="split">
        <label>
          Bind IP
          <input value={props.bindIp} onInput={(event) => props.onBindIp(event.currentTarget.value)} />
        </label>
        <label>
          Port
          <input type="number" min="1" value={props.port} onInput={(event) => props.onPort(Number(event.currentTarget.value))} />
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
    </div>
  );
}
