export const TOPBAR_PULSE_STALE_MS = 250;

type TopbarPulseMeterProps = {
  running: boolean;
  engineStale: boolean;
  safetyClearPending: boolean;
  telemetryFresh: boolean;
  rms: number;
  peak: number;
  onOpenSettings: () => void;
};

const clampLevel = (value: number) =>
  Number.isFinite(value) ? Math.max(0, Math.min(1, value)) : 0;

export function TopbarPulseMeter(props: TopbarPulseMeterProps) {
  const available = () =>
    props.running &&
    !props.engineStale &&
    !props.safetyClearPending &&
    props.telemetryFresh;
  const rms = () => (available() ? clampLevel(props.rms) : 0);
  const peak = () => (available() ? clampLevel(props.peak) : 0);
  const level = () => Math.max(rms(), peak());
  const lit = () => level() >= 0.005;
  const percent = () => Math.round(level() * 100);
  const freshnessStale = () =>
    props.running &&
    (props.engineStale || props.safetyClearPending || !props.telemetryFresh);

  return (
    <button
      type="button"
      class={`topbarPulseMeter${lit() ? " active" : ""}`}
      data-topbar-pulse
      data-pulse-lit={lit() ? "true" : "false"}
      data-pulse-fresh={available() ? "true" : "false"}
      data-pulse-stale={freshnessStale() ? "true" : "false"}
      data-pulse-stale-ms={TOPBAR_PULSE_STALE_MS}
      title="音声入力設定を開く"
      aria-label="音声入力設定を開く"
      onClick={props.onOpenSettings}
    >
      <span class="topbarPulseLabel" data-no-localize>PULSE</span>
      <span
        class="topbarPulseTrack"
        role="meter"
        aria-label="ライブ音声入力レベル"
        aria-valuemin="0"
        aria-valuemax="100"
        aria-valuenow={percent()}
        aria-valuetext={`${percent()}パーセント`}
      >
        <i
          aria-hidden="true"
          style={{
            opacity: lit() ? "1" : "0",
            transform: `scaleY(${rms()})`,
          }}
        />
        <b
          aria-hidden="true"
          style={{
            opacity: lit() ? "1" : "0",
            transform: `scaleY(${peak()})`,
          }}
        />
      </span>
    </button>
  );
}
