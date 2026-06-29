interface TouchDimmerControlPanelProps {
  value: number;
  sliderMin: number;
  sliderMax: number;
  canControl: boolean;
  flashActive: boolean;
  formatDmxPercent: (value: number) => string;
  onSetValue: (value: number) => void | Promise<void>;
  onQuickLevel: (level: "out" | "half" | "full") => void;
  onFlashStart: () => void;
  onFlashEnd: () => void;
  onFlashKeyDown: (event: KeyboardEvent) => void;
  onFlashKeyUp: (event: KeyboardEvent) => void;
}

export function TouchDimmerControlPanel(props: TouchDimmerControlPanelProps) {
  return (
    <>
      <label class="touchSlider">
        Dimmer
        <input
          type="range"
          min={props.sliderMin}
          max={props.sliderMax}
          value={props.value}
          onInput={(event) => void props.onSetValue(Number(event.currentTarget.value))}
        />
        <strong>{props.formatDmxPercent(props.value)}</strong>
      </label>
      <div class="touchDimmerQuickRow">
        <button onClick={() => props.onQuickLevel("out")} disabled={!props.canControl}>
          Out
        </button>
        <button onClick={() => props.onQuickLevel("half")} disabled={!props.canControl}>
          Half
        </button>
        <button class="primary" onClick={() => props.onQuickLevel("full")} disabled={!props.canControl}>
          Full
        </button>
        <button
          class={props.flashActive ? "momentary active" : "momentary"}
          disabled={!props.canControl}
          onPointerDown={() => props.onFlashStart()}
          onPointerUp={() => props.onFlashEnd()}
          onPointerCancel={() => props.onFlashEnd()}
          onPointerLeave={() => props.onFlashEnd()}
          onBlur={() => props.onFlashEnd()}
          onKeyDown={props.onFlashKeyDown}
          onKeyUp={props.onFlashKeyUp}
        >
          Flash
        </button>
      </div>
    </>
  );
}
