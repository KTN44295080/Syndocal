import { createEffect, createMemo, createSignal, For, Show } from "solid-js";
import {
  currentCueEffectTargets,
  refreshListedCueEffectStates,
  selectAllCueEffects,
  setCueEffectIncluded,
  setCueEffectTargetEnabled,
} from "../cueEffectRecall";
import type { CueEffectRecallChange } from "../cueEffectRecall";
import type { CueEffectTarget, EffectSummary } from "../types";

interface CueEffectRecallEditorProps {
  id?: string;
  effects: EffectSummary[];
  hasAnyEffects?: boolean;
  targets: CueEffectTarget[];
  expanded?: boolean;
  currentMode: "capture" | "refresh";
  applyHint?: string;
  onChange: (targets: CueEffectTarget[], change: CueEffectRecallChange) => void;
}

const effectKindLabel = (effect: EffectSummary) => {
  if (effect.effect_type === "Color") return "Color";
  if (effect.effect_type === "Chaser") return "Chaser";
  if (effect.effect_type === "Move") return "Move";
  if (effect.effect_type === "PositionWave") return "Position Wave";
  return "LFO";
};

const effectRowsPerPage = 48;

export function CueEffectRecallEditor(props: CueEffectRecallEditorProps) {
  const [open, setOpen] = createSignal(Boolean(props.expanded));
  const [filter, setFilter] = createSignal("");
  const [page, setPage] = createSignal(0);
  const targetById = createMemo(() => new Map(props.targets.map((target) => [target.effect_id, target])));
  const includedCount = createMemo(() => props.effects.filter((effect) => targetById().has(effect.id)).length);
  const matchingEffects = createMemo(() => {
    const query = filter().trim().toLocaleLowerCase();
    if (!query) return props.effects;
    return props.effects.filter((effect) =>
      effect.label.toLocaleLowerCase().includes(query)
      || String(effect.id).includes(query)
      || effectKindLabel(effect).toLocaleLowerCase().includes(query)
    );
  });
  const pageCount = createMemo(() => Math.max(1, Math.ceil(matchingEffects().length / effectRowsPerPage)));
  const visibleEffects = createMemo(() => {
    const start = page() * effectRowsPerPage;
    return matchingEffects().slice(start, start + effectRowsPerPage);
  });
  createEffect(() => {
    const lastPage = pageCount() - 1;
    if (page() > lastPage) setPage(lastPage);
  });

  const useCurrentState = () => {
    props.onChange(
      props.currentMode === "capture"
        ? currentCueEffectTargets(props.effects)
        : refreshListedCueEffectStates(props.effects, props.targets),
      { kind: props.currentMode === "capture" ? "captureCurrent" : "refreshStates" },
    );
  };

  return (
    <details
      id={props.id}
      class="cueEffectRecallEditor cueEditOnly"
      open={props.expanded}
      onToggle={(event) => setOpen(event.currentTarget.open)}
    >
      <summary>
        <span>Effect Recall</span>
        <small class="tabularNums">{includedCount()} / {props.effects.length}</small>
      </summary>
      <Show when={props.expanded || open()}>
        <div class="cueEffectRecallBody">
          <div class="cueEffectRecallToolbar">
            <p>
              Included Effects recall their stored ON/OFF state on GO. Effects not listed here are left unchanged.
            </p>
            <div class="buttonRow" role="group" aria-label="Effect Recall selection tools">
              <button
                type="button"
                disabled={props.effects.length === 0 || includedCount() === props.effects.length}
                onClick={() => props.onChange(selectAllCueEffects(props.effects, props.targets), { kind: "selectAll" })}
              >
                Select All
              </button>
              <button type="button" disabled={includedCount() === 0} onClick={() => props.onChange([], { kind: "clear" })}>
                Clear
              </button>
              <button type="button" disabled={props.effects.length === 0 || (props.currentMode === "refresh" && includedCount() === 0)} onClick={useCurrentState}>
                {props.currentMode === "capture" ? "Capture Current" : "Refresh States"}
              </button>
            </div>
          </div>
          <Show when={props.applyHint}>
            {(hint) => <p class="cueEffectRecallApplyHint">{hint()}</p>}
          </Show>
          <Show when={props.effects.length > effectRowsPerPage}>
            <div class="cueEffectRecallFilter">
              <label>
                <span>Filter Effects</span>
                <input
                  type="search"
                  value={filter()}
                  placeholder="Name, ID, or type"
                  onInput={(event) => {
                    setFilter(event.currentTarget.value);
                    setPage(0);
                  }}
                />
              </label>
              <Show when={pageCount() > 1}>
                <nav aria-label="Effect Recall pages">
                  <button type="button" aria-label="Previous Effect Recall page" disabled={page() === 0} onClick={() => setPage(page() - 1)}>Prev</button>
                  <span class="tabularNums" data-no-localize>{page() + 1} / {pageCount()}</span>
                  <button type="button" aria-label="Next Effect Recall page" disabled={page() >= pageCount() - 1} onClick={() => setPage(page() + 1)}>Next</button>
                </nav>
              </Show>
            </div>
          </Show>
          <Show
            when={props.effects.length > 0}
            fallback={
              <p class="empty">
                {props.hasAnyEffects ? "No Effects match this Cue scope." : "Create an Effect in the Effect rack first."}
              </p>
            }
          >
            <Show when={matchingEffects().length > 0} fallback={<p class="empty">No Effects match this filter.</p>}>
              <div class="cueEffectRecallList" role="list" aria-label="Cue Effect Recall targets">
                <For each={visibleEffects()}>
                  {(effect, index) => {
                    const target = () => targetById().get(effect.id);
                    const included = () => Boolean(target());
                    return (
                      <div
                        class={`cueEffectRecallRow ${included() ? "included" : ""}`}
                        role="listitem"
                        aria-posinset={page() * effectRowsPerPage + index() + 1}
                        aria-setsize={matchingEffects().length}
                      >
                        <label>
                          <input
                            type="checkbox"
                            checked={included()}
                            aria-label={`Include Effect ${effect.id} ${effect.label}`}
                            onChange={(event) => {
                              const included = event.currentTarget.checked;
                              props.onChange(
                                setCueEffectIncluded(props.effects, props.targets, effect.id, included),
                                { kind: "include", effectId: effect.id, included },
                              );
                            }}
                          />
                          <span>
                            <b data-no-localize>{effect.label}</b>
                            <small class="tabularNums">#{effect.id} · {effectKindLabel(effect)}</small>
                          </span>
                        </label>
                        <button
                          type="button"
                          class={`cueEffectRecallState ${target()?.enabled ? "on" : "off"}`}
                          disabled={!included()}
                          aria-label={`Recall Effect ${effect.id} ${target()?.enabled ? "ON" : "OFF"}`}
                          aria-pressed={target()?.enabled ?? false}
                          onClick={() => props.onChange(
                            setCueEffectTargetEnabled(
                              props.effects,
                              props.targets,
                              effect.id,
                              !(target()?.enabled ?? false),
                            ),
                            { kind: "state", effectId: effect.id },
                          )}
                        >
                          {target()?.enabled ? "ON" : "OFF"}
                        </button>
                      </div>
                    );
                  }}
                </For>
              </div>
            </Show>
          </Show>
        </div>
      </Show>
    </details>
  );
}
