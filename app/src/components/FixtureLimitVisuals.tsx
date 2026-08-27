import type { JSX } from "solid-js";

const dmxLimitMaximum = 65_535;

const clampDmxLimit = (value: number) =>
  Math.min(dmxLimitMaximum, Math.max(0, Number.isFinite(value) ? value : 0));

type DimmerLimitMeterProps = {
  minimum: number;
  maximum: number;
};

export function DimmerLimitMeter(props: DimmerLimitMeterProps) {
  const range = () => {
    const first = clampDmxLimit(props.minimum);
    const second = clampDmxLimit(props.maximum);
    return { minimum: Math.min(first, second), maximum: Math.max(first, second) };
  };
  const position = (value: number) =>
    `${((dmxLimitMaximum - value) / dmxLimitMaximum) * 100}%`;
  const rangeDescription = () =>
    `DMX minimum ${range().minimum}, maximum ${range().maximum}`;
  const allowedStyle = (): JSX.CSSProperties => ({
    top: position(range().maximum),
    height: `${((range().maximum - range().minimum) / dmxLimitMaximum) * 100}%`,
  });

  return (
    <div
      class="dimmerLimitMeter"
      data-dimmer-limit-meter
      role="img"
      aria-label="Dimmer Limits"
      aria-description={rangeDescription()}
      data-dimmer-limit-minimum={range().minimum}
      data-dimmer-limit-maximum={range().maximum}
    >
      <span class="dimmerLimitMaximum">100%</span>
      <span class="dimmerLimitTrack" aria-hidden="true">
        <i class="dimmerLimitAllowed" style={allowedStyle()} />
        <i class="dimmerLimitMaximumMark" style={{ top: position(range().maximum) }} />
        <i class="dimmerLimitMinimumMark" style={{ top: position(range().minimum) }} />
      </span>
      <span class="dimmerLimitMinimum">0%</span>
    </div>
  );
}
