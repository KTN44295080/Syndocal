import { splitProps, type JSX } from "solid-js";

type VerticalFaderInputProps = Omit<
  JSX.InputHTMLAttributes<HTMLInputElement>,
  "class" | "type"
> & {
  chromeClass?: string;
  inputClass?: string;
};

const numericValue = (
  value: JSX.InputHTMLAttributes<HTMLInputElement>["value"] | undefined,
  fallback: number,
) => {
  const parsed = Number(Array.isArray(value) ? value[0] : value);
  return Number.isFinite(parsed) ? parsed : fallback;
};

export function VerticalFaderInput(props: VerticalFaderInputProps) {
  const [local, inputProps] = splitProps(props, [
    "chromeClass",
    "inputClass",
    "min",
    "max",
    "value",
  ]);
  const visualPosition = () => {
    const minimum = numericValue(local.min, 0);
    const maximum = numericValue(local.max, 100);
    const value = numericValue(local.value, minimum);
    const normalized = maximum === minimum
      ? 0
      : Math.min(1, Math.max(0, (value - minimum) / (maximum - minimum)));
    return (1 - normalized) * 100;
  };

  return (
    <span
      class={`verticalFaderChrome${local.chromeClass ? ` ${local.chromeClass}` : ""}`}
      style={{ "--syndocal-fader-position": `${visualPosition()}%` }}
    >
      <span class="verticalFaderTrack" aria-hidden="true" />
      <span class="verticalFaderThumb" aria-hidden="true">
        <span class="verticalFaderGrip" />
      </span>
      <input
        {...inputProps}
        class={`verticalFaderNativeInput${local.inputClass ? ` ${local.inputClass}` : ""}`}
        type="range"
        min={local.min}
        max={local.max}
        value={local.value}
        aria-orientation="vertical"
      />
    </span>
  );
}
