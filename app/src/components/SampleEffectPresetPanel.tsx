import { createMemo, createSignal, For, Show } from "solid-js";

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
  | "circle"
  | "curve"
  | "spectrum"
  | "colour-chase";

export type EffectLibraryFamily =
  | "Colour"
  | "Chaser"
  | "Move"
  | "Value"
  | "Curve"
  | "Mappings"
  | "Colour Mappings";

interface SampleEffectPresetOption {
  value: SampleEffectPreset;
  label: string;
  family: EffectLibraryFamily;
  engine: "LFO" | "Wave" | "Bundle";
  target: string;
  description: string;
  supportsTarget: boolean;
  requiresTarget?: boolean;
}

const effectLibraryFamilies: EffectLibraryFamily[] = [
  "Colour",
  "Chaser",
  "Move",
  "Value",
  "Curve",
  "Mappings",
  "Colour Mappings",
];

export const sampleEffectPresetOptions: SampleEffectPresetOption[] = [
  {
    value: "pulse",
    label: "Pulse",
    family: "Value",
    engine: "LFO",
    target: "Dimmer",
    description: "Four-beat dimmer rise for a steady front wash pulse.",
    supportsTarget: true,
  },
  {
    value: "shared",
    label: "Shared",
    family: "Value",
    engine: "LFO",
    target: "Dimmer+Video",
    description: "Two-beat pulse intended for linked lighting and VJ targets.",
    supportsTarget: true,
  },
  {
    value: "wave",
    label: "Wave",
    family: "Mappings",
    engine: "Wave",
    target: "Dimmer",
    description: "Position wave across the Front group with beat-synced travel.",
    supportsTarget: true,
  },
  {
    value: "flash",
    label: "Flash",
    family: "Value",
    engine: "LFO",
    target: "Dimmer",
    description: "Short square flash for cue accents and blackout hits.",
    supportsTarget: true,
  },
  {
    value: "random",
    label: "Random",
    family: "Value",
    engine: "LFO",
    target: "Dimmer",
    description: "Random dimmer modulation for loose live texture.",
    supportsTarget: true,
  },
  {
    value: "perlin",
    label: "Perlin",
    family: "Curve",
    engine: "LFO",
    target: "Dimmer",
    description: "Smooth noise modulation for organic brightness movement.",
    supportsTarget: true,
  },
  {
    value: "chase",
    label: "Chase",
    family: "Chaser",
    engine: "Wave",
    target: "Dimmer",
    description: "Directional front-group chase driven by stage position.",
    supportsTarget: true,
  },
  {
    value: "ball",
    label: "Ball",
    family: "Mappings",
    engine: "Wave",
    target: "Dimmer",
    description: "Radial dimmer ball expanding from the stage origin.",
    supportsTarget: true,
  },
  {
    value: "fan",
    label: "Fan",
    family: "Move",
    engine: "Wave",
    target: "Pan",
    description: "Position-based pan fan for moving-head spread looks.",
    supportsTarget: true,
  },
  {
    value: "circle",
    label: "Circle",
    family: "Move",
    engine: "Bundle",
    target: "Pan/Tilt",
    description: "Paired pan and tilt effects for circular movement.",
    supportsTarget: false,
  },
  {
    value: "curve",
    label: "Curve Saw",
    family: "Curve",
    engine: "LFO",
    target: "Any value",
    description: "Beat-synced saw curve for ramps, wheels and continuous channels.",
    supportsTarget: true,
  },
  {
    value: "spectrum",
    label: "Colour Spectrum",
    family: "Colour",
    engine: "LFO",
    target: "Colour attribute",
    description: "Continuous spectrum sweep for hue, wheel or individual colour channels.",
    supportsTarget: true,
    requiresTarget: true,
  },
  {
    value: "colour-chase",
    label: "Colour Chase",
    family: "Colour Mappings",
    engine: "Wave",
    target: "Colour attribute",
    description: "Position-mapped colour sweep across the current fixture layout.",
    supportsTarget: true,
    requiresTarget: true,
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
  const [family, setFamily] = createSignal<EffectLibraryFamily | "All">("All");
  const visibleOptions = createMemo(() =>
    family() === "All"
      ? sampleEffectPresetOptions
      : sampleEffectPresetOptions.filter((option) => option.family === family()),
  );
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
  const selectFamily = (nextFamily: EffectLibraryFamily | "All") => {
    setFamily(nextFamily);
    const first = nextFamily === "All"
      ? sampleEffectPresetOptions[0]
      : sampleEffectPresetOptions.find((option) => option.family === nextFamily);
    if (first) props.onSelectPreset(first.value);
  };
  const familyCode = (optionFamily: EffectLibraryFamily) => {
    switch (optionFamily) {
      case "Colour": return "CO";
      case "Chaser": return "CH";
      case "Move": return "MV";
      case "Value": return "VL";
      case "Curve": return "CV";
      case "Mappings": return "MP";
      case "Colour Mappings": return "CM";
    }
  };

  return (
    <section class="sampleEffectPresetPanel" aria-label="Effect Library">
      <header class="sampleEffectPresetHeader">
        <div>
          <strong>Effect Library</strong>
          <span>{visibleOptions().length} of {sampleEffectPresetOptions.length} recipes</span>
        </div>
        <span>{selectedOption().family}</span>
      </header>
      <nav class="effectFamilyRail" aria-label="Effect families">
        <button class={family() === "All" ? "active" : ""} aria-pressed={family() === "All"} onClick={() => selectFamily("All")}>
          All
        </button>
        <For each={effectLibraryFamilies}>
          {(optionFamily) => (
            <button
              class={family() === optionFamily ? "active" : ""}
              aria-pressed={family() === optionFamily}
              onClick={() => selectFamily(optionFamily)}
            >
              {optionFamily}
            </button>
          )}
        </For>
      </nav>
      <div class="sampleEffectPresetGrid" role="list" aria-label="Effect recipes">
        <For each={visibleOptions()}>
          {(option) => (
            <button
              class={props.selectedPreset === option.value ? "sampleEffectPresetCard active" : "sampleEffectPresetCard"}
              data-requires-target={option.requiresTarget ? "true" : "false"}
              aria-pressed={props.selectedPreset === option.value}
              onClick={() => props.onSelectPreset(option.value)}
            >
              <span class="effectRecipeGlyph" aria-hidden="true">{familyCode(option.family)}</span>
              <span class="sampleEffectPresetPick">
                <strong>{option.label}</strong>
                <small>{option.engine} · {option.target}</small>
              </span>
            </button>
          )}
        </For>
      </div>
      <footer class="effectRecipeDock">
        <div class="effectRecipeSelection">
          <span class="effectRecipeGlyph" aria-hidden="true">{familyCode(selectedOption().family)}</span>
          <div>
            <strong>{selectedOption().label}</strong>
            <span>{selectedOption().family} · {selectedOption().engine} · {selectedOption().target}</span>
            <p>{selectedOption().description}</p>
          </div>
        </div>
        <div class="effectRecipeActions">
          <button
            onClick={() => void props.onLoadPreset(props.selectedPreset)}
            disabled={selectedOption().requiresTarget}
            title={selectedOption().requiresTarget ? "This effect requires the current target" : "Load the embedded sample target"}
          >
            Use Sample Target
          </button>
          <button
            class="primary"
            onClick={() => void props.onLoadPresetForTarget(props.selectedPreset)}
            disabled={targetDisabled(selectedOption())}
            title={targetTitle(selectedOption())}
          >
            Apply to Current
          </button>
        </div>
        <Show when={selectedTargetError() && selectedOption().supportsTarget}>
          <small class="sampleEffectPresetHint" role="status">{selectedTargetError()}</small>
        </Show>
      </footer>
    </section>
  );
}
