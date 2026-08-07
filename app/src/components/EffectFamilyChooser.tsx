import { For } from "solid-js";

export type EffectChooserFamily =
  | "STEPS"
  | "COLOR FX"
  | "CHASER FX"
  | "MOVE FX"
  | "VALUE FX"
  | "CURVE FX"
  | "MAPPINGS"
  | "COLOUR MAPPINGS"
  | "SUPER SCENE";

export type EffectRecipeFamily = Exclude<EffectChooserFamily, "STEPS" | "SUPER SCENE">;

export const effectChooserFamilies: EffectChooserFamily[] = [
  "STEPS",
  "COLOR FX",
  "CHASER FX",
  "MOVE FX",
  "VALUE FX",
  "CURVE FX",
  "MAPPINGS",
  "COLOUR MAPPINGS",
  "SUPER SCENE",
];

const chooserFamilyCode = (family: EffectChooserFamily) => {
  switch (family) {
    case "STEPS": return "ST";
    case "COLOR FX": return "CO";
    case "CHASER FX": return "CH";
    case "MOVE FX": return "MV";
    case "VALUE FX": return "VL";
    case "CURVE FX": return "CV";
    case "MAPPINGS": return "MP";
    case "COLOUR MAPPINGS": return "CM";
    case "SUPER SCENE": return "TL";
  }
};

interface EffectFamilyChooserProps {
  activeFamily: EffectChooserFamily;
  onSelectFamily: (family: EffectChooserFamily) => void | Promise<void>;
  descriptions?: Partial<Record<EffectChooserFamily, string>>;
  disabledFamilies?: Partial<Record<EffectChooserFamily, boolean>>;
}

const quickSceneFamilies = new Set<EffectChooserFamily>([
  "COLOR FX",
  "CHASER FX",
  "VALUE FX",
  "MOVE FX",
]);

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
            data-scene-fx-quick-block={quickSceneFamilies.has(family) ? family : undefined}
            disabled={props.disabledFamilies?.[family] ?? false}
            onClick={() => void props.onSelectFamily(family)}
          >
            <span class="effectFamilyGlyph" aria-hidden="true">{chooserFamilyCode(family)}</span>
            <strong>{family === "SUPER SCENE" ? "TIMELINE" : family}</strong>
            {props.descriptions?.[family]
              ? <small>{props.descriptions[family]}</small>
              : null}
          </button>
        )}
      </For>
    </nav>
  );
}
