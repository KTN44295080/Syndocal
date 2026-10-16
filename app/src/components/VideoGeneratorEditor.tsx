import { createSignal, For, Show } from "solid-js";
import type { VideoGenerator, VideoGeneratorColor } from "../types";

interface Props {
  layerId: number;
  generator: VideoGenerator;
  onChange: (generator: VideoGenerator) => void | Promise<void>;
}

const colorHex = (color: VideoGeneratorColor) => "#" + color.slice(0, 3).map((value) => value.toString(16).padStart(2, "0")).join("");

export function VideoGeneratorEditor(props: Props) {
  const [pending, setPending] = createSignal(false);
  const parameters = () => "Solid" in props.generator ? props.generator.Solid
    : "LinearGradient" in props.generator ? props.generator.LinearGradient : props.generator.Noise;
  const change = async (key: string, value: number | VideoGeneratorColor) => {
    if (pending()) return;
    const updated = { ...parameters(), [key]: value };
    const generator = "Solid" in props.generator ? { Solid: updated }
      : "LinearGradient" in props.generator ? { LinearGradient: updated } : { Noise: updated };
    setPending(true);
    try {
      await props.onChange(generator as VideoGenerator);
    } finally {
      setPending(false);
    }
  };
  const colors = () => "Solid" in props.generator
    ? [{ key: "color", label: "Generator colour", alphaLabel: "Generator alpha" }]
    : [{ key: "color_a", label: "Generator colour A", alphaLabel: "Generator alpha A" }, { key: "color_b", label: "Generator colour B", alphaLabel: "Generator alpha B" }];
  const readColor = (key: string) => (parameters() as unknown as Record<string, VideoGeneratorColor>)[key];
  const numericFields = () => "LinearGradient" in props.generator
    ? [{ key: "angle_degrees", label: "Gradient angle", min: 0, max: 359 }]
    : "Noise" in props.generator ? [
      { key: "seed", label: "Noise seed", min: 0, max: 4294967295 },
      { key: "scale", label: "Noise scale", min: 1, max: 256 },
      { key: "speed", label: "Noise speed", min: 0, max: 1000 },
    ] : [];
  return (
    <details class="videoGeneratorEditor" data-video-generator-editor={props.layerId}>
      <summary>Generator parameters</summary>
      <fieldset class="videoGeneratorFields" disabled={pending()}>
      <For each={colors()}>{(field) => <div class="split">
        <label>{field.label}<input type="color" data-generator-field={field.key} value={colorHex(readColor(field.key))}
          onChange={(event) => {
            const hex = event.currentTarget.value;
            change(field.key, [parseInt(hex.slice(1, 3), 16), parseInt(hex.slice(3, 5), 16), parseInt(hex.slice(5, 7), 16), readColor(field.key)[3]]);
          }} /></label>
        <label>{field.alphaLabel}<input type="number" min="0" max="255" step="1" data-generator-field={`${field.key}-alpha`} value={readColor(field.key)[3]}
          onChange={(event) => {
            if (!event.currentTarget.checkValidity() || event.currentTarget.value === "") return;
            const color = readColor(field.key);
            change(field.key, [color[0], color[1], color[2], Number(event.currentTarget.value)]);
          }} /></label>
      </div>}</For>
      <Show when={numericFields().length > 0}>
        <div class="videoGeneratorNumbers"><For each={numericFields()}>{(field) =>
          <label>{field.label}<input type="number" min={field.min} max={field.max} step="1" data-generator-field={field.key}
            value={(parameters() as unknown as Record<string, number>)[field.key]}
            onChange={(event) => {
              if (event.currentTarget.checkValidity() && event.currentTarget.value !== "") change(field.key, Number(event.currentTarget.value));
            }} /></label>
        }</For></div>
      </Show>
      </fieldset>
    </details>
  );
}
