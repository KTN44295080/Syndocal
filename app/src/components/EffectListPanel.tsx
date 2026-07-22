import { createMemo, createSignal, For, Show } from "solid-js";
import type { EffectSummary } from "../types";
import { EffectGraphicalPreview } from "./EffectGraphicalPreview";

const effectsPerPage = 10;

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
  const [page, setPage] = createSignal(0);
  const pageCount = createMemo(() => Math.max(1, Math.ceil(props.effects.length / effectsPerPage)));
  const currentPage = createMemo(() => Math.min(page(), pageCount() - 1));
  const pageStart = createMemo(() => currentPage() * effectsPerPage);
  const visibleEffects = createMemo(() => props.effects.slice(pageStart(), pageStart() + effectsPerPage));
  const effectKindLabel = (effect: EffectSummary) => {
    if (effect.effect_type === "Color") return "Color";
    if (effect.effect_type === "Chaser") return "Chaser";
    if (effect.effect_type === "Move") return "Move";
    if (effect.effect_type === "Value") return "Value";
    if (effect.effect_type === "Curve") return "Curve";
    return effect.effect_type === "PositionWave" ? "Wave" : "LFO";
  };
  const effectKindClass = (effect: EffectSummary) => {
    if (effect.effect_type === "Color") return "color";
    if (effect.effect_type === "Chaser") return "chaser";
    if (effect.effect_type === "Move") return "move";
    if (effect.effect_type === "Value") return "value";
    if (effect.effect_type === "Curve") return "curve";
    return effect.effect_type === "PositionWave" ? "wave" : "lfo";
  };
  const timingLabel = (effect: EffectSummary) => {
    const clockSync = effect.effect_type === "Color"
      ? effect.color?.clock_sync
      : effect.effect_type === "Chaser"
        ? effect.chaser?.clock_sync
        : effect.effect_type === "Move"
          ? effect.move_effect?.clock_sync
        : effect.effect_type === "Curve"
          ? effect.curve?.clock_sync
        : effect.clock_sync;
    if (clockSync) {
      return `sync ${clockSync.beats} beat`;
    }
    const periodMs = effect.effect_type === "Color"
      ? effect.color?.period_ms
      : effect.effect_type === "Chaser"
        ? effect.chaser?.step_duration_ms
        : effect.effect_type === "Move"
          ? effect.move_effect?.period_ms
        : effect.effect_type === "Curve"
          ? effect.curve?.period_ms
        : effect.period_ms;
    if (periodMs) {
      return effect.effect_type === "Chaser" ? `${periodMs}ms/step` : `${periodMs}ms`;
    }
    if (effect.speed !== null && effect.speed !== undefined) {
      return `speed ${effect.speed.toFixed(1)}`;
    }
    return "free";
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
        <Show when={pageCount() > 1}>
          <nav class="effectListPager" aria-label="Live effect pages">
            <button type="button" onClick={() => setPage(Math.max(0, currentPage() - 1))} disabled={currentPage() === 0}>Previous</button>
            <span class="tabularNums">Page {currentPage() + 1} / {pageCount()}</span>
            <button type="button" onClick={() => setPage(Math.min(pageCount() - 1, currentPage() + 1))} disabled={currentPage() === pageCount() - 1}>Next</button>
          </nav>
        </Show>
        <div class="effectListRows" role="list" aria-label="Live effects" aria-rowcount={props.effects.length}>
        <For each={visibleEffects()}>
          {(effect, pageIndex) => {
            const index = () => pageStart() + pageIndex();
            return (
            <div
              class={effect.enabled ? "effectItem" : "effectItem disabled"}
              role="listitem"
              aria-posinset={index() + 1}
              aria-setsize={props.effects.length}
            >
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
                <EffectGraphicalPreview effect={effect} compact />
                <div class="effectMetaGrid" aria-label={`Effect summary for ${effect.label}`}>
                  <span class={`effectMetaChip ${effectKindClass(effect)}`}>{effectKindLabel(effect)}</span>
                  <Show when={effect.effect_type === "Lfo" || effect.effect_type === "PositionWave"}>
                    <span class="effectMetaChip">{effect.attribute}</span>
                    <span class="effectMetaChip">{effect.shape}</span>
                  </Show>
                  <Show when={effect.effect_type === "Chaser" ? effect.chaser : null}>
                    {(chaser) => (
                      <>
                        <span class="effectMetaChip tabularNums">{chaser().steps.length} steps</span>
                        <span class="effectMetaChip tabularNums">{chaser().active_step_count} on</span>
                        <span class="effectMetaChip">{chaser().direction}</span>
                        <span class="effectMetaChip tabularNums">{chaser().features.length} feature{chaser().features.length === 1 ? "" : "s"}</span>
                      </>
                    )}
                  </Show>
                  <Show when={effect.effect_type === "Color" ? effect.color : null}>
                    {(color) => (
                      <>
                        <span class="effectMetaChip">{color().algorithm}</span>
                        <span class="effectMetaChip">{colorInterpolationLabel(color().interpolation)}</span>
                        <span class="effectMetaChip tabularNums">spread {Math.round(color().fixture_spread * 100)}%</span>
                      </>
                    )}
                  </Show>
                  <Show when={effect.effect_type === "Move" ? effect.move_effect : null}>
                    {(move) => (
                      <>
                        <span class="effectMetaChip tabularNums">{move().points.length} points</span>
                        <span class="effectMetaChip">{move().interpolation}</span>
                        <span class="effectMetaChip">{move().direction}</span>
                        <span class="effectMetaChip">{move().coordinate_mode}</span>
                        <span class="effectMetaChip tabularNums">spread {Math.round(move().fixture_spread * 100)}%</span>
                      </>
                    )}
                  </Show>
                  <Show when={effect.effect_type === "Curve" ? effect.curve : null}>
                    {(curve) => (
                      <>
                        <span class="effectMetaChip tabularNums">{curve().points.length} points</span>
                        <span class="effectMetaChip">Cubic</span>
                        <span class="effectMetaChip">{curve().direction}</span>
                        <span class="effectMetaChip tabularNums">spread {Math.round(curve().fixture_spread * 100)}%</span>
                      </>
                    )}
                  </Show>
                  <span
                    class={(
                      effect.effect_type === "Color"
                        ? effect.color?.clock_sync
                        : effect.effect_type === "Chaser"
                          ? effect.chaser?.clock_sync
                          : effect.effect_type === "Move"
                            ? effect.move_effect?.clock_sync
                          : effect.effect_type === "Curve"
                            ? effect.curve?.clock_sync
                          : effect.clock_sync
                    ) ? "effectMetaChip sync" : "effectMetaChip"}
                  >
                    {timingLabel(effect)}
                  </span>
                  <Show when={effect.wavelength}>
                    {(wavelength) => <span class="effectMetaChip">wl {wavelength().toFixed(1)}</span>}
                  </Show>
                  <Show when={effect.effect_type !== "Color" && effect.effect_type !== "Move"}>
                    <span class="effectMetaChip range">{rangeLabel(effect)}</span>
                  </Show>
                  <span class="effectMetaChip target">{targetLabel(effect)}</span>
                  <span class="effectMetaChip blend">{effect.color?.blend_mode ?? effect.chaser?.blend_mode ?? effect.move_effect?.blend_mode ?? effect.blend_mode}</span>
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
            );
          }}
        </For>
        </div>
      </Show>
    </div>
  );
}
