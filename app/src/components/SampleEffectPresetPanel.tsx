import { For, Show } from "solid-js";

export type SampleEffectPreset =
  | "pulse"
  | "shared"
  | "wave"
  | "flash"
  | "random"
  | "perlin"
  | "chase"
  | "ball"
  | "fan"
  | "circle";

interface SampleEffectPresetOption {
  value: SampleEffectPreset;
  label: string;
  family: "LFO" | "Wave" | "Bundle";
  target: string;
  description: string;
  supportsTarget: boolean;
}

export const sampleEffectPresetOptions: SampleEffectPresetOption[] = [
  {
    value: "pulse",
    label: "Pulse",
    family: "LFO",
    target: "Dimmer",
    description: "Four-beat dimmer rise for a steady front wash pulse.",
    supportsTarget: true,
  },
  {
    value: "shared",
    label: "Shared",
    family: "LFO",
    target: "Dimmer+Video",
    description: "Two-beat pulse intended for linked lighting and VJ targets.",
    supportsTarget: true,
  },
  {
    value: "wave",
    label: "Wave",
    family: "Wave",
    target: "Dimmer",
    description: "Position wave across the Front group with beat-synced travel.",
    supportsTarget: true,
  },
  {
    value: "flash",
    label: "Flash",
    family: "LFO",
    target: "Dimmer",
    description: "Short square flash for cue accents and blackout hits.",
    supportsTarget: true,
  },
  {
    value: "random",
    label: "Random",
    family: "LFO",
    target: "Dimmer",
    description: "Random dimmer modulation for loose live texture.",
    supportsTarget: true,
  },
  {
    value: "perlin",
    label: "Perlin",
    family: "LFO",
    target: "Dimmer",
    description: "Smooth noise modulation for organic brightness movement.",
    supportsTarget: true,
  },
  {
    value: "chase",
    label: "Chase",
    family: "Wave",
    target: "Dimmer",
    description: "Directional front-group chase driven by stage position.",
    supportsTarget: true,
  },
  {
    value: "ball",
    label: "Ball",
    family: "Wave",
    target: "Dimmer",
    description: "Radial dimmer ball expanding from the stage origin.",
    supportsTarget: true,
  },
  {
    value: "fan",
    label: "Fan",
    family: "Wave",
    target: "Pan",
    description: "Position-based pan fan for moving-head spread looks.",
    supportsTarget: true,
  },
  {
    value: "circle",
    label: "Circle",
    family: "Bundle",
    target: "Pan/Tilt",
    description: "Paired pan and tilt effects for circular movement.",
    supportsTarget: false,
  },
];

export const sampleEffectPresetSupportsTarget = (preset: SampleEffectPreset) =>
  sampleEffectPresetOptions.find((option) => option.value === preset)?.supportsTarget ?? false;

interface SampleEffectPresetPanelProps {
  selectedPreset: SampleEffectPreset;
  targetErrorForPreset: (preset: SampleEffectPreset) => string;
  onSelectPreset: (preset: SampleEffectPreset) => void;
  onLoadPreset: (preset: SampleEffectPreset) => void | Promise<void>;
  onLoadPresetForTarget: (preset: SampleEffectPreset) => void | Promise<void>;
}

export function SampleEffectPresetPanel(props: SampleEffectPresetPanelProps) {
  const selectedOption = () =>
    sampleEffectPresetOptions.find((option) => option.value === props.selectedPreset) ?? sampleEffectPresetOptions[0];
  const targetError = (option: SampleEffectPresetOption) => props.targetErrorForPreset(option.value);
  const selectedTargetError = () => targetError(selectedOption());
  const targetDisabled = (option: SampleEffectPresetOption) => !option.supportsTarget || Boolean(targetError(option));
  const targetTitle = (option: SampleEffectPresetOption) => {
    if (!option.supportsTarget) {
      return "This preset creates multiple coordinated effects and cannot target one current attribute";
    }
    return targetError(option) || "Load this sample onto the current effect target";
  };

  return (
    <div class="sampleEffectPresetPanel">
      <div class="sampleEffectPresetHeader">
        <div>
          <strong>Sample Presets</strong>
          <span>
            {selectedOption().family} / {selectedOption().target}
          </span>
        </div>
        <div class="sampleEffectPresetActions">
          <button onClick={() => void props.onLoadPreset(props.selectedPreset)}>Load Selected</button>
          <button
            class="primary"
            onClick={() => void props.onLoadPresetForTarget(props.selectedPreset)}
            disabled={targetDisabled(selectedOption())}
            title={targetTitle(selectedOption())}
          >
            Target Selected
          </button>
        </div>
      </div>
      <div class="sampleEffectPresetGrid">
        <For each={sampleEffectPresetOptions}>
          {(option) => (
            <div class={props.selectedPreset === option.value ? "sampleEffectPresetCard active" : "sampleEffectPresetCard"}>
              <button class="sampleEffectPresetPick" onClick={() => props.onSelectPreset(option.value)}>
                <strong>{option.label}</strong>
                <span>
                  {option.family} / {option.target}
                </span>
                <small>{option.description}</small>
              </button>
              <div class="sampleEffectPresetActions">
                <button onClick={() => void props.onLoadPreset(option.value)}>Load</button>
                <button
                  onClick={() => void props.onLoadPresetForTarget(option.value)}
                  disabled={targetDisabled(option)}
                  title={targetTitle(option)}
                >
                  Target
                </button>
              </div>
            </div>
          )}
        </For>
      </div>
      <Show when={selectedTargetError() && selectedOption().supportsTarget}>
        <small class="sampleEffectPresetHint">{selectedTargetError()}</small>
      </Show>
    </div>
  );
}
