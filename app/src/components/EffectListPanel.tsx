import { For, Show } from "solid-js";
import type { EffectSummary } from "../types";

interface EffectListPanelProps {
  effects: EffectSummary[];
  onMoveEffect: (effectId: number, delta: -1 | 1) => void | Promise<void>;
  onSetEnabled: (effectId: number, enabled: boolean) => void | Promise<void>;
  onUseOutputPosition: (effectId: number, targets: EffectSummary["video_targets"]) => void | Promise<void>;
  onDuplicateEffect: (effectId: number) => void | Promise<void>;
  onUseAsDraft: (effect: EffectSummary) => void;
  onSavePreset: (effectId: number) => void | Promise<void>;
  onRemoveEffect: (effectId: number) => void | Promise<void>;
}

export function EffectListPanel(props: EffectListPanelProps) {
  const effectKindLabel = (effect: EffectSummary) => {
    if (effect.effect_type === "Color") return "Color";
    return effect.effect_type === "PositionWave" ? "Wave" : "LFO";
  };
  const effectKindClass = (effect: EffectSummary) => {
    if (effect.effect_type === "Color") return "color";
    return effect.effect_type === "PositionWave" ? "wave" : "lfo";
  };
  const timingLabel = (effect: EffectSummary) => {
    const clockSync = effect.effect_type === "Color" ? effect.color?.clock_sync : effect.clock_sync;
    if (clockSync) {
      return `sync ${clockSync.beats} beat`;
    }
    const periodMs = effect.effect_type === "Color" ? effect.color?.period_ms : effect.period_ms;
    if (periodMs) {
      return `${periodMs}ms`;
    }
    if (effect.speed !== null && effect.speed !== undefined) {
      return `speed ${effect.speed.toFixed(1)}`;
    }
    return "free";
  };
  const colorToHex = (red: number, green: number, blue: number) => {
    const byte = (value: number) => Math.round(Math.min(65_535, Math.max(0, value)) / 257).toString(16).padStart(2, "0");
    return `#${byte(red)}${byte(green)}${byte(blue)}`;
  };
  const colorInterpolationLabel = (interpolation: NonNullable<EffectSummary["color"]>["interpolation"]) => {
    if (interpolation === "HsvShortest") return "HSV shortest";
    if (interpolation === "HsvLongest") return "HSV longest";
    return "RGB";
  };
  const rangeLabel = (effect: EffectSummary) => `${Math.round(effect.low)}-${Math.round(effect.high)}`;
  const targetLabel = (effect: EffectSummary) => {
    const targets: string[] = [];
    if (effect.fixture_ids.length > 0) {
      targets.push(`${effect.fixture_ids.length} fixture${effect.fixture_ids.length === 1 ? "" : "s"}`);
    }
    if (effect.target_group_ids.length > 0) {
      targets.push(`groups ${effect.target_group_ids.join(",")}`);
    }
    if (effect.video_targets.length > 0) {
      targets.push(`video ${effect.video_targets.map((target) => target.param).join(",")}`);
    }
    return targets.length > 0 ? targets.join(" / ") : "no target";
  };

  return (
    <div class="effectList">
      <Show when={props.effects.length > 0} fallback={<div class="effectListEmpty">Choose a recipe in Effect Library, then Apply to Current.</div>}>
        <For each={props.effects}>
          {(effect, index) => (
            <div class={effect.enabled ? "effectItem" : "effectItem disabled"}>
              <button
                class="effectItemSummary"
                aria-label={`Edit ${effect.label}`}
                onClick={() => props.onUseAsDraft(effect)}
              >
                <div class="effectItemTitleRow">
                  <strong>{effect.label}</strong>
                  <span class={effect.enabled ? "effectMetaChip state-on" : "effectMetaChip state-off"}>
                    {effect.enabled ? "on" : "off"}
                  </span>
                </div>
                <div class="effectMetaGrid" aria-label={`Effect summary for ${effect.label}`}>
                  <span class={`effectMetaChip ${effectKindClass(effect)}`}>{effectKindLabel(effect)}</span>
                  <Show when={effect.effect_type !== "Color"}>
                    <span class="effectMetaChip">{effect.attribute}</span>
                    <span class="effectMetaChip">{effect.shape}</span>
                  </Show>
                  <Show when={effect.effect_type === "Color" ? effect.color : null}>
                    {(color) => (
                      <>
                        <span
                          class="effectColorSwatches"
                          role="img"
                          aria-label={`${color().stops.length} color stops`}
                          title={`${color().stops.length} color stops`}
                        >
                          <For each={color().stops}>
                            {(stop) => (
                              <i
                                style={{ "background-color": colorToHex(stop.color.red, stop.color.green, stop.color.blue) }}
                                aria-hidden="true"
                              />
                            )}
                          </For>
                        </span>
                        <span class="effectMetaChip">{color().algorithm}</span>
                        <span class="effectMetaChip">{colorInterpolationLabel(color().interpolation)}</span>
                        <span class="effectMetaChip tabularNums">spread {Math.round(color().fixture_spread * 100)}%</span>
                      </>
                    )}
                  </Show>
                  <span
                    class={(effect.effect_type === "Color" ? effect.color?.clock_sync : effect.clock_sync) ? "effectMetaChip sync" : "effectMetaChip"}
                  >
                    {timingLabel(effect)}
                  </span>
                  <Show when={effect.wavelength}>
                    {(wavelength) => <span class="effectMetaChip">wl {wavelength().toFixed(1)}</span>}
                  </Show>
                  <Show when={effect.effect_type !== "Color"}>
                    <span class="effectMetaChip range">{rangeLabel(effect)}</span>
                  </Show>
                  <span class="effectMetaChip target">{targetLabel(effect)}</span>
                  <span class="effectMetaChip blend">{effect.color?.blend_mode ?? effect.blend_mode}</span>
                </div>
              </button>
              <div class="effectItemActions">
                <button aria-label={`Move ${effect.label} up`} title="Move up" onClick={() => void props.onMoveEffect(effect.id, -1)} disabled={index() === 0}>
                  ↑
                </button>
                <button aria-label={`Move ${effect.label} down`} title="Move down" onClick={() => void props.onMoveEffect(effect.id, 1)} disabled={index() === props.effects.length - 1}>
                  ↓
                </button>
                <button onClick={() => void props.onSetEnabled(effect.id, !effect.enabled)}>
                  {effect.enabled ? "Disable" : "Enable"}
                </button>
                <details class="effectItemMore">
                  <summary>More</summary>
                  <div>
                    <button onClick={() => props.onUseAsDraft(effect)}>Edit</button>
                    <button onClick={() => void props.onDuplicateEffect(effect.id)}>Duplicate</button>
                    <Show when={effect.effect_type === "PositionWave" && effect.video_targets.length > 0}>
                      <button onClick={() => void props.onUseOutputPosition(effect.id, effect.video_targets)}>Use Video Output Pos</button>
                    </Show>
                    <button onClick={() => void props.onSavePreset(effect.id)}>Save Preset</button>
                    <button onClick={() => void props.onRemoveEffect(effect.id)}>Remove</button>
                  </div>
                </details>
              </div>
            </div>
          )}
        </For>
      </Show>
    </div>
  );
}
