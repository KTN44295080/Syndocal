import { createMemo, For, Show } from "solid-js";

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

export type EffectChooserFamily =
  | "STEPS"
  | "COLOR FX"
  | "CHASER FX"
  | "MOVE FX"
  | "VALUE FX"
  | "CURVE FX"
  | "MAPPINGS"
  | "COLOR MAPPINGS"
  | "SUPER SCENE";

export type EffectRecipeFamily = Exclude<EffectChooserFamily, "STEPS" | "SUPER SCENE">;

interface SampleEffectPresetOption {
  value: SampleEffectPreset;
  label: string;
  family: EffectRecipeFamily;
  engine: "LFO" | "Mapping" | "Color" | "ColorMapping" | "Chaser" | "Move" | "Curve";
  target: string;
  description: string;
  supportsTarget: boolean;
  requiresTarget?: boolean;
}

export const effectChooserFamilies: EffectChooserFamily[] = [
  "STEPS",
  "COLOR FX",
  "CHASER FX",
  "MOVE FX",
  "VALUE FX",
  "CURVE FX",
  "MAPPINGS",
  "COLOR MAPPINGS",
  "SUPER SCENE",
];

export const sampleEffectPresetOptions: SampleEffectPresetOption[] = [
  {
    value: "pulse",
    label: "Pulse",
    family: "VALUE FX",
    engine: "LFO",
    target: "Dimmer",
    description: "Four-beat dimmer rise for a steady front wash pulse.",
    supportsTarget: true,
  },
  {
    value: "shared",
    label: "Shared",
    family: "VALUE FX",
    engine: "LFO",
    target: "Dimmer+Video",
    description: "Two-beat pulse intended for linked lighting and VJ targets.",
    supportsTarget: true,
  },
  {
    value: "wave",
    label: "Wave",
    family: "MAPPINGS",
    engine: "Mapping",
    target: "Dimmer",
    description: "Beat-synced dimmer function distributed across authored fixture order.",
    supportsTarget: true,
  },
  {
    value: "flash",
    label: "Flash",
    family: "VALUE FX",
    engine: "LFO",
    target: "Dimmer",
    description: "Short square flash for cue accents and blackout hits.",
    supportsTarget: true,
  },
  {
    value: "random",
    label: "Random",
    family: "VALUE FX",
    engine: "LFO",
    target: "Dimmer",
    description: "Random dimmer modulation for loose live texture.",
    supportsTarget: true,
  },
  {
    value: "perlin",
    label: "Perlin",
    family: "VALUE FX",
    engine: "LFO",
    target: "Dimmer",
    description: "Smooth noise modulation for organic brightness movement.",
    supportsTarget: true,
  },
  {
    value: "chase",
    label: "Chase",
    family: "CHASER FX",
    engine: "Chaser",
    target: "Dimmer",
    description: "Fixture-index beam chase with explicit order, active width and fading.",
    supportsTarget: true,
    requiresTarget: true,
  },
  {
    value: "ball",
    label: "Ball",
    family: "MAPPINGS",
    engine: "Mapping",
    target: "Dimmer",
    description: "Bouncing dimmer function distributed across authored fixture order.",
    supportsTarget: true,
  },
  {
    value: "fan",
    label: "Fan",
    family: "MAPPINGS",
    engine: "Mapping",
    target: "Pan",
    description: "Static pan fan distributed deterministically across authored fixture order.",
    supportsTarget: true,
  },
  {
    value: "circle",
    label: "Circle",
    family: "MOVE FX",
    engine: "Move",
    target: "Pan/Tilt",
    description: "Independent paired-axis path with smooth, beat-synced circular movement.",
    supportsTarget: true,
    requiresTarget: true,
  },
  {
    value: "curve",
    label: "Curve Saw",
    family: "CURVE FX",
    engine: "Curve",
    target: "Any value",
    description: "Editable cubic channel function with independent in/out tangents.",
    supportsTarget: true,
  },
  {
    value: "spectrum",
    label: "Colour Spectrum",
    family: "COLOR FX",
    engine: "Color",
    target: "RGB / RGBW / Wheel",
    description: "Seven-stop HSV spectrum rendered across the fixture's complete colour system.",
    supportsTarget: true,
    requiresTarget: true,
  },
  {
    value: "colour-chase",
    label: "Colour Chase",
    family: "COLOR MAPPINGS",
    engine: "ColorMapping",
    target: "2D fixture cells",
    description: "Embedded RGB16 video frames sampled across the current 2D fixture layout.",
    supportsTarget: true,
    requiresTarget: true,
  },
];

export const sampleEffectPresetSupportsTarget = (preset: SampleEffectPreset) =>
  sampleEffectPresetOptions.find((option) => option.value === preset)?.supportsTarget ?? false;

const chooserFamilyCode = (family: EffectChooserFamily) => {
  switch (family) {
    case "STEPS": return "ST";
    case "COLOR FX": return "CO";
    case "CHASER FX": return "CH";
    case "MOVE FX": return "MV";
    case "VALUE FX": return "VL";
    case "CURVE FX": return "CV";
    case "MAPPINGS": return "MP";
    case "COLOR MAPPINGS": return "CM";
    case "SUPER SCENE": return "SS";
  }
};

interface EffectFamilyChooserProps {
  activeFamily: EffectChooserFamily;
  onSelectFamily: (family: EffectChooserFamily) => void | Promise<void>;
}

export function EffectFamilyChooser(props: EffectFamilyChooserProps) {
  return (
    <nav class="effectFamilyChooser" aria-label="Effect family chooser">
      <For each={effectChooserFamilies}>
        {(family, index) => (
          <button
            type="button"
            class={props.activeFamily === family ? "active" : ""}
            aria-pressed={props.activeFamily === family}
            data-effect-family={family}
            data-effect-family-id={family.toLowerCase().replace(/\s+/g, "-")}
            data-family-order={index() + 1}
            onClick={() => void props.onSelectFamily(family)}
          >
            <span class="effectFamilyGlyph" aria-hidden="true">{chooserFamilyCode(family)}</span>
            <strong>{family}</strong>
          </button>
        )}
      </For>
    </nav>
  );
}

interface SampleEffectPresetPanelProps {
  activeFamily: EffectRecipeFamily;
  selectedPreset: SampleEffectPreset;
  targetErrorForPreset: (preset: SampleEffectPreset) => string;
  onSelectPreset: (preset: SampleEffectPreset) => void;
  onLoadPreset: (preset: SampleEffectPreset) => void | Promise<void>;
  onLoadPresetForTarget: (preset: SampleEffectPreset) => void | Promise<void>;
}

export function SampleEffectPresetPanel(props: SampleEffectPresetPanelProps) {
  const visibleOptions = createMemo(() =>
    sampleEffectPresetOptions.filter((option) => option.family === props.activeFamily),
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
  const familyCode = (optionFamily: EffectRecipeFamily) => {
    switch (optionFamily) {
      case "COLOR FX": return "CO";
      case "CHASER FX": return "CH";
      case "MOVE FX": return "MV";
      case "VALUE FX": return "VL";
      case "CURVE FX": return "CV";
      case "MAPPINGS": return "MP";
      case "COLOR MAPPINGS": return "CM";
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
