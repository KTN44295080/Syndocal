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
  const effectKindLabel = (effect: EffectSummary) => (effect.effect_type === "PositionWave" ? "Wave" : "LFO");
  const timingLabel = (effect: EffectSummary) => {
    if (effect.clock_sync) {
      return `sync ${effect.clock_sync.beats} beat`;
    }
    if (effect.period_ms) {
      return `${effect.period_ms}ms`;
    }
    if (effect.speed !== null && effect.speed !== undefined) {
      return `speed ${effect.speed.toFixed(1)}`;
    }
    return "free";
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
      <Show when={props.effects.length > 0} fallback={<div class="effectListEmpty">No effects in the stack.</div>}>
        <For each={props.effects}>
          {(effect, index) => (
            <div class={effect.enabled ? "effectItem" : "effectItem disabled"}>
              <div class="effectItemSummary">
                <div class="effectItemTitleRow">
                  <strong>{effect.label}</strong>
                  <span class={effect.enabled ? "effectMetaChip state-on" : "effectMetaChip state-off"}>
                    {effect.enabled ? "on" : "off"}
                  </span>
                </div>
                <div class="effectMetaGrid" aria-label={`Effect summary for ${effect.label}`}>
                  <span class={`effectMetaChip ${effect.effect_type === "PositionWave" ? "wave" : "lfo"}`}>{effectKindLabel(effect)}</span>
                  <span class="effectMetaChip">{effect.attribute}</span>
                  <span class="effectMetaChip">{effect.shape}</span>
                  <span class={effect.clock_sync ? "effectMetaChip sync" : "effectMetaChip"}>{timingLabel(effect)}</span>
                  <Show when={effect.wavelength}>
                    {(wavelength) => <span class="effectMetaChip">wl {wavelength().toFixed(1)}</span>}
                  </Show>
                  <span class="effectMetaChip range">{rangeLabel(effect)}</span>
                  <span class="effectMetaChip target">{targetLabel(effect)}</span>
                  <span class="effectMetaChip blend">{effect.blend_mode}</span>
                </div>
              </div>
              <div class="effectItemActions">
                <button onClick={() => void props.onMoveEffect(effect.id, -1)} disabled={index() === 0}>
                  Up
                </button>
                <button onClick={() => void props.onMoveEffect(effect.id, 1)} disabled={index() === props.effects.length - 1}>
                  Down
                </button>
                <button onClick={() => void props.onSetEnabled(effect.id, !effect.enabled)}>
                  {effect.enabled ? "Disable" : "Enable"}
                </button>
                <button onClick={() => props.onUseAsDraft(effect)}>Use Draft</button>
                <button onClick={() => void props.onDuplicateEffect(effect.id)}>Duplicate</button>
                <Show when={effect.effect_type === "PositionWave" && effect.video_targets.length > 0}>
                  <button onClick={() => void props.onUseOutputPosition(effect.id, effect.video_targets)}>Use Video Output Pos</button>
                </Show>
                <button onClick={() => void props.onSavePreset(effect.id)}>Save</button>
                <button onClick={() => void props.onRemoveEffect(effect.id)}>Remove</button>
              </div>
            </div>
          )}
        </For>
      </Show>
    </div>
  );
}
