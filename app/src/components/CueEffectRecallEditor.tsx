import { createEffect, createMemo, createSignal, For, Show } from "solid-js";
import {
  currentCueEffectTargets,
  refreshListedCueEffectStates,
  selectAllCueEffects,
  setCueEffectIncluded,
  setCueEffectTargetEnabled,
} from "../cueEffectRecall";
import type { CueEffectRecallChange } from "../cueEffectRecall";
import type { CueEffectTarget, EffectKind, EffectParamsSnapshot, EffectSummary } from "../types";

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

const effectKindLabel = (effectKind: EffectKind) => {
  if (effectKind === "PositionWave") return "Position Wave";
  if (effectKind === "Lfo") return "LFO";
  return effectKind;
};

const effectParamsDescriptor = (params: EffectParamsSnapshot): { kind: EffectKind; label: string } => {
  if ("Lfo" in params) return { kind: "Lfo", label: params.Lfo.label };
  if ("PositionWave" in params) return { kind: "PositionWave", label: params.PositionWave.label };
  if ("Color" in params) return { kind: "Color", label: params.Color.label };
  if ("Chaser" in params) return { kind: "Chaser", label: params.Chaser.label };
  if ("Move" in params) return { kind: "Move", label: params.Move.label };
  return { kind: "Value", label: params.Value.label };
};

interface CueEffectRecallRowModel {
  id: number;
  label: string;
  kind: EffectKind;
}

const effectRowsPerPage = 48;

export function CueEffectRecallEditor(props: CueEffectRecallEditorProps) {
  const [open, setOpen] = createSignal(Boolean(props.expanded));
  const [filter, setFilter] = createSignal("");
  const [page, setPage] = createSignal(0);
  const targetById = createMemo(() => new Map(props.targets.map((target) => [target.effect_id, target])));
  const effectById = createMemo(() => new Map(props.effects.map((effect) => [effect.id, effect])));
  const rows = createMemo<CueEffectRecallRowModel[]>(() => [
    ...props.effects.map((effect) => ({ id: effect.id, label: effect.label, kind: effect.effect_type })),
    ...props.targets.flatMap((target) => {
      const params = target.params;
      if (params == null || effectById().has(target.effect_id)) return [];
      const descriptor = effectParamsDescriptor(params);
      return [{
        id: target.effect_id,
        label: descriptor.label || `Effect ${target.effect_id}`,
        kind: descriptor.kind,
      }];
    }),
  ]);
  const includedCount = createMemo(() => rows().filter((row) => targetById().has(row.id)).length);
  const listedCurrentEffectCount = createMemo(() => props.effects.filter((effect) => targetById().has(effect.id)).length);
  const allCurrentEffectsIncluded = createMemo(() => props.effects.length > 0 && listedCurrentEffectCount() === props.effects.length);
  const matchingRows = createMemo(() => {
    const query = filter().trim().toLocaleLowerCase();
    if (!query) return rows();
    return rows().filter((row) =>
      row.label.toLocaleLowerCase().includes(query)
      || String(row.id).includes(query)
      || effectKindLabel(row.kind).toLocaleLowerCase().includes(query)
    );
  });
  const pageCount = createMemo(() => Math.max(1, Math.ceil(matchingRows().length / effectRowsPerPage)));
  const visibleRows = createMemo(() => {
    const start = page() * effectRowsPerPage;
    return matchingRows().slice(start, start + effectRowsPerPage);
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
        <small class="tabularNums">{includedCount()} / {rows().length}</small>
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
                disabled={props.effects.length === 0 || allCurrentEffectsIncluded()}
                onClick={() => props.onChange(selectAllCueEffects(props.effects, props.targets), { kind: "selectAll" })}
              >
                Select All
              </button>
              <button type="button" disabled={includedCount() === 0} onClick={() => props.onChange([], { kind: "clear" })}>
                Clear
              </button>
              <button type="button" disabled={props.effects.length === 0 || (props.currentMode === "refresh" && listedCurrentEffectCount() === 0)} onClick={useCurrentState}>
                {props.currentMode === "capture" ? "Capture Current" : "Refresh States"}
              </button>
            </div>
          </div>
          <Show when={props.applyHint}>
            {(hint) => <p class="cueEffectRecallApplyHint">{hint()}</p>}
          </Show>
          <Show when={rows().length > effectRowsPerPage}>
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
            when={rows().length > 0}
            fallback={
              <p class="empty">
                {props.hasAnyEffects ? "No Effects match this Cue scope." : "Create an Effect in the Effect rack first."}
              </p>
            }
          >
            <Show when={matchingRows().length > 0} fallback={<p class="empty">No Effects match this filter.</p>}>
              <div class="cueEffectRecallList" role="list" aria-label="Cue Effect Recall targets">
                <For each={visibleRows()}>
                  {(row, index) => {
                    const target = () => targetById().get(row.id);
                    const included = () => Boolean(target());
                    return (
                      <div
                        class={`cueEffectRecallRow ${included() ? "included" : ""}`}
                        role="listitem"
                        aria-posinset={page() * effectRowsPerPage + index() + 1}
                        aria-setsize={matchingRows().length}
                      >
                        <label>
                          <input
                            type="checkbox"
                            checked={included()}
                            aria-label={`Include Effect ${row.id} ${row.label}`}
                            onChange={(event) => {
                              const included = event.currentTarget.checked;
                              props.onChange(
                                setCueEffectIncluded(props.effects, props.targets, row.id, included),
                                { kind: "include", effectId: row.id, included },
                              );
                            }}
                          />
                          <span>
                            <b data-no-localize>{row.label}</b>
                            <small class="cueEffectRecallMeta tabularNums">
                              <span class="cueEffectRecallMetaText">#{row.id} · {effectKindLabel(row.kind)}</span>
                              <Show when={target()?.params != null}>
                                <span class="cueEffectParamsChip">Owns params</span>
                              </Show>
                            </small>
                          </span>
                        </label>
                        <button
                          type="button"
                          class={`cueEffectRecallState ${target()?.enabled ? "on" : "off"}`}
                          disabled={!included()}
                          aria-label={`Recall Effect ${row.id} ${target()?.enabled ? "ON" : "OFF"}`}
                          aria-pressed={target()?.enabled ?? false}
                          onClick={() => props.onChange(
                            setCueEffectTargetEnabled(
                              props.effects,
                              props.targets,
                              row.id,
                              !(target()?.enabled ?? false),
                            ),
                            { kind: "state", effectId: row.id },
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
