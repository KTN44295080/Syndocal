import { createEffect, createMemo, createSignal, For, onCleanup, onMount, Show } from "solid-js";
import { createStore, reconcile } from "solid-js/store";
import type { VideoLayerSummary } from "../types";
import { filterFxCatalog, fxCategoryLabel, normalizeFxSource, writeFxDragPayload, type FxCatalogEntry, type FxCategory } from "../fxCatalogModel";
import { renderFxThumbnail, type FxThumbnail } from "../fxThumbnail";
import { explainFxRejection } from "../fxRejectionExplanation";
import type { FxAuditionError, FxAuditionSession } from "../createFxAuditionSession";
import "./VideoEffectBrowser.css";

export interface VideoEffectBrowserProps {
  entries: FxCatalogEntry[];
  layer: VideoLayerSummary | null;
  locale?: string;
  busy?: boolean;
  onAdd: (entry: FxCatalogEntry, layerId: number) => void | Promise<void>;
  /** Preview-only trial; nothing reaches Program until the audition is taken. */
  audition?: FxAuditionSession;
  onAudition?: (entry: FxCatalogEntry, layerId: number) => void | Promise<void>;
  onImport?: () => void;
  onOpenFx?: () => void;
}

function Thumbnail(props: { entry: FxCatalogEntry; ja: boolean }) {
  const [thumbnail, setThumbnail] = createSignal<FxThumbnail>({ rendered: false });
  let element!: HTMLDivElement;
  onMount(() => {
    // Compile only visible cards; polling never recompiles unchanged source.
    const render = () => { if (!props.entry.rejection && props.entry.source) setThumbnail(renderFxThumbnail(props.entry.source)); };
    if (typeof IntersectionObserver === "undefined") { render(); return; }
    const observer = new IntersectionObserver((entries) => {
      if (entries.some((entry) => entry.isIntersecting)) { render(); observer.disconnect(); }
    });
    observer.observe(element); onCleanup(() => observer.disconnect());
  });
  return <div ref={element} class="videoFxCatalogThumbnail" data-fx-thumbnail={thumbnail().rendered ? "rendered" : "fallback"}>
    <Show when={thumbnail().url} fallback={<span class="videoFxCatalogFallback" aria-hidden="true">FX</span>}>
      {(url) => <img src={url()} alt={props.ja ? "標準画像に適用した例" : "Effect rendered on the standard test image"} width="160" height="90" />}
    </Show>
    <span class="videoFxCatalogThumbnailNote">{thumbnail().rendered ? (props.ja ? "適用例" : "Test image") : (props.ja ? "画像なし" : "No preview")}</span>
  </div>;
}

/** The narrow strip hides its scrollbar (it would cost Inspector height), so the mouse wheel scrolls it
 * sideways while it can still move that way; at either end the wheel scrolls the effect list again. */
function sideScroll(event: WheelEvent & { currentTarget: HTMLDivElement }) {
  const strip = event.currentTarget;
  const max = strip.scrollWidth - strip.clientWidth;
  if (event.deltaX !== 0 || max <= 1 || event.deltaY === 0) return;
  if ((event.deltaY < 0 && strip.scrollLeft <= 0) || (event.deltaY > 0 && strip.scrollLeft >= max - 1)) return;
  event.preventDefault();
  strip.scrollLeft += event.deltaY;
}

/** A catalog adapter. All additions still go through the existing scoped chain command. */
export function VideoEffectBrowser(props: VideoEffectBrowserProps) {
  const [entries, setEntries] = createStore<FxCatalogEntry[]>([]);
  createEffect(() => setEntries(reconcile(props.entries, { key: "id" })));
  const [query, setQuery] = createSignal("");
  const [category, setCategory] = createSignal<FxCategory | "All">("All");
  const ja = () => props.locale === "ja";
  const categories = createMemo(() => ["All", ...new Set(entries.map((entry) => entry.category))] as (FxCategory | "All")[]);
  // A category that no longer exists (for example after an import is removed) falls back to All.
  createEffect(() => { if (!categories().includes(category())) setCategory("All"); });
  const filtered = createMemo(() => filterFxCatalog(entries, query(), category(), props.locale));
  const chainSources = createMemo(() => new Set(props.layer?.isf_effect ? [props.layer.isf_effect, ...(props.layer.isf_effect.stack ?? [])].map((stage) => normalizeFxSource(stage.source)) : []));
  const target = () => props.layer ? `${ja() ? "追加先：レイヤー" : "Add to: Layer"} ${props.layer.id} · ${props.layer.label}` : (ja() ? "追加先のレイヤーを選択してください" : "Select a layer to add effects");
  const add = (entry: FxCatalogEntry) => {
    if (props.layer && !props.busy && !entry.rejection) void props.onAdd(entry, props.layer.id);
  };
  const auditioning = () => {
    const value = props.audition?.active(props.layer?.id);
    return value && !value.stale ? value : null;
  };
  const trialBusy = () => props.busy || Boolean(props.audition?.busy());
  const tryOut = (entry: FxCatalogEntry) => {
    if (props.layer && props.onAudition && !trialBusy() && !entry.rejection) void props.onAudition(entry, props.layer.id);
  };
  const auditionError = (error: FxAuditionError) => ({
    "preview-unavailable": ja() ? "このレイヤーはプレビューに表示できません。対応するのはローカルファイル、静止画、ジェネレーターです。" : "This layer cannot be shown in Preview. Only local files, still images and generators can.",
    "audition-rejected": ja() ? "プレビューがこのエフェクトを受け付けませんでした。本番映像は変わっていません。" : "Preview did not accept the effect chain. Your live output was not changed.",
    "take-failed": ja() ? "本番のレイヤーに反映できませんでした。" : "The live layer did not take the new chain.",
    "stage-limit": ja() ? "1つのレイヤーに入れられるエフェクトは最大8個です。" : "A layer holds at most 8 effects.",
    "builtin-unavailable": ja() ? "このエフェクトをプレビュー用に準備できませんでした。" : "That effect could not be prepared for Preview.",
    "preview-busy": ja() ? "プレビューが処理中のため、試聴を終了できませんでした。FXタブで「破棄」をもう一度押してください。" : "Preview is busy and may still show the audition. Press Discard again in the FX tab.",
  })[error];
  return <section class="videoFxCatalog" data-effect-browser data-no-localize aria-label={ja() ? "エフェクトブラウザー" : "Effect browser"}>
    <header class="videoFxCatalogHeader">
      <div class="videoFxCatalogScope"><strong class="videoFxCatalogTarget" data-fx-add-target title={target()}>{target()}</strong>
        <span class="videoFxCatalogLive" data-fx-live-warning title={props.onAudition ? (ja() ? "「追加」は本番(PROGRAM)にもすぐ反映されます。「試す」はプレビューだけに表示し、本番映像は変わりません。" : "Add changes Program immediately. Try shows the effect on Preview only; Program stays unchanged.") : (ja() ? "「追加」は本番(PROGRAM)にもすぐ反映されます。" : "Add changes Program immediately.")}>{props.onAudition ? (ja() ? "試す＝プレビュー／追加＝LIVE" : "Try = Preview · Add = LIVE") : (ja() ? "追加＝LIVE（本番に反映）" : "Add = LIVE (Program)")}</span>
        <Show when={auditioning()}>
          <span class="videoFxCatalogAuditioning" data-fx-auditioning>{ja() ? "試聴中: FXタブのA/Bで比較し、「本番へ反映」または「破棄」を選びます。" : "Auditioning on Preview. Compare A/B in the FX tab, then take or discard."}</span>
        </Show>
        <Show when={props.audition?.error()}>{(error) => <span class="videoFxCatalogProblem" role="alert" data-fx-audition-error>{auditionError(error())}</span>}</Show>
      </div>
    </header>
    <div class="videoFxCatalogTools">
      <label class="videoFxCatalogSearch"><span>{ja() ? "検索" : "Search"}</span>
        <input type="search" data-fx-catalog-search value={query()} placeholder={ja() ? "エフェクトを検索" : "Search effects"}
          title={ja() ? "名前・種類・説明で検索" : "Search by name, category or description"}
          onInput={(event) => setQuery(event.currentTarget.value)} />
      </label>
      {/* One strip: wide panes wrap the chips beside the search; narrow panes keep a single row that
          scrolls sideways, so the filter costs no extra height in the Inspector. */}
      <div class="videoFxCatalogStrip" onWheel={sideScroll}>
        <div class="videoFxCatalogChips" role="group" data-fx-category-chips aria-label={ja() ? "種類で絞り込む" : "Filter by category"}>
          <For each={categories()}>{(item) => <button type="button" class="videoFxCatalogChip" data-fx-category={item} aria-pressed={category() === item}
            onClick={() => setCategory(item)}>{fxCategoryLabel(item, props.locale)}</button>}</For>
        </div>
        <div class="videoFxCatalogActions">
          <Show when={props.onImport}><button type="button" disabled={props.busy} onClick={props.onImport}>{ja() ? "ISFを読み込む" : "Import ISF"}</button></Show>
          <Show when={props.onOpenFx}><button type="button" onClick={props.onOpenFx}>{ja() ? "チェーンへ" : "FX Chain"}</button></Show>
        </div>
      </div>
    </div>
    <div class="videoFxCatalogResults" role="list" aria-label={ja() ? "エフェクト一覧" : "Effects"}>
      <Show when={filtered().length > 0} fallback={<p class="videoFxCatalogEmpty">{ja() ? "条件に合うエフェクトがありません。検索語や種類を変えてください。" : "No effects match. Try another search or category."}</p>}>
        <For each={filtered()}>{(entry) => {
          const rejected = () => entry.rejection ? explainFxRejection(entry.rejection, props.locale, entry.source) : undefined;
          const already = () => chainSources().has(normalizeFxSource(entry.source));
          return <article class="videoFxCatalogCard" data-fx-catalog-card={entry.id} data-fx-card-state={entry.rejection ? "rejected" : already() ? "already" : "usable"}
            role="listitem" tabIndex="0" aria-label={`${entry.label} · ${entry.description[ja() ? "ja" : "en"]}`}
            draggable={!entry.rejection && Boolean(props.layer) && !props.busy}
            onDragStart={(event) => { if (event.dataTransfer) writeFxDragPayload(event.dataTransfer, entry); }}
            onDblClick={() => add(entry)} onKeyDown={(event) => {
              if (event.key === "Enter" && event.target === event.currentTarget) { event.preventDefault(); add(entry); }
            }}>
            <Thumbnail entry={entry} ja={ja()} />
            <div class="videoFxCatalogInfo">
              <div class="videoFxCatalogName"><strong>{entry.label}</strong><span>{fxCategoryLabel(entry.category, props.locale)}</span></div>
              <p>{entry.description[ja() ? "ja" : "en"]}</p>
              <span class="videoFxCatalogState">{entry.rejection ? (ja() ? "使用不可" : "UNAVAILABLE") : already() ? (ja() ? "このレイヤーで使用中" : "Already in this layer") : (ja() ? "使用できます" : "Ready to add")}</span>
              <Show when={rejected()}>{(reason) => <div class="videoFxCatalogRejection" data-fx-rejection>
                <strong>{reason().title}</strong><p>{reason().detail}</p><p>{reason().action}</p>
                <details onDblClick={(event) => event.stopPropagation()}><summary>{ja() ? "技術的な詳細" : "Technical details"}</summary><pre>{reason().raw}</pre></details>
              </div>}</Show>
            </div>
            <div class="videoFxCatalogButtons">
              <Show when={props.onAudition}>
                <button type="button" class="videoFxCatalogTry" data-fx-catalog-audition disabled={!props.layer || trialBusy() || Boolean(entry.rejection)}
                  aria-label={`${ja() ? "プレビューで試す" : "Try on Preview"}: ${entry.label} · ${target()}`}
                  title={ja() ? "プレビューだけに表示します。本番映像は変わりません。" : "Shows on Preview only. Program is unchanged."}
                  onClick={() => tryOut(entry)} onDblClick={(event) => event.stopPropagation()}>{ja() ? "試す" : "Try"}</button>
              </Show>
              <button type="button" class="videoFxCatalogAdd" data-fx-catalog-add disabled={!props.layer || props.busy || Boolean(entry.rejection)}
                aria-label={`${ja() ? "追加(LIVE)" : "Add (LIVE)"}: ${entry.label} · ${target()}`} title={`${target()} — ${ja() ? "本番にも反映されます" : "affects Program"}`}
                onClick={() => add(entry)} onDblClick={(event) => event.stopPropagation()}>{ja() ? "追加" : "Add"}</button>
            </div>
          </article>;
        }}</For>
      </Show>
    </div>
    <p class="videoFxCatalogHint">{props.onAudition
      ? (ja() ? "試す: プレビューのみ。追加(Enter・ダブルクリック): 本番にも反映。レイヤー行やFXタブへドラッグして追加もできます。" : "Try: Preview only. Add (Enter, double-click): affects Program. You can also drag onto a layer row or the FX tab.")
      : (ja() ? "追加(Enter・ダブルクリック)は本番にも反映されます。" : "Add (Enter or double-click) affects Program.")}</p>
  </section>;
}
