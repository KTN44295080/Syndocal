interface DmxTestFramePanelProps {
  protocolLabel: string;
  channel: number;
  width: number;
  value: number;
  onChannelChange: (value: number) => void;
  onWidthChange: (value: number) => void;
  onValueChange: (value: number) => void;
  onSendTest: () => void;
  onSendRoutes: () => void;
}

export function DmxTestFramePanel(props: DmxTestFramePanelProps) {
  return (
    <div class="dmxTestFramePanel">
      <div class="panelHeader">
        <h3>DMX Test Frame</h3>
        <span>{props.protocolLabel}</span>
      </div>
      <div class="triple">
        <label>
          Channel
          <input
            data-io-control="dmx-test-channel"
            type="number"
            min="1"
            max="512"
            value={props.channel}
            onInput={(event) => props.onChannelChange(Number(event.currentTarget.value))}
          />
        </label>
        <label>
          Width
          <input
            type="number"
            min="1"
            max="512"
            value={props.width}
            onInput={(event) => props.onWidthChange(Number(event.currentTarget.value))}
          />
        </label>
        <label>
          Value
          <input
            type="number"
            min="0"
            max="255"
            value={props.value}
            onInput={(event) => props.onValueChange(Number(event.currentTarget.value))}
          />
        </label>
      </div>
      <div class="buttonRow">
        <button onClick={() => props.onValueChange(0)}>Zero</button>
        <button onClick={() => props.onValueChange(127)}>Half</button>
        <button onClick={() => props.onValueChange(255)}>Full</button>
        <button class="primary" onClick={props.onSendTest}>
          Send Test
        </button>
        <button onClick={props.onSendRoutes}>Send Routes</button>
      </div>
    </div>
  );
}
