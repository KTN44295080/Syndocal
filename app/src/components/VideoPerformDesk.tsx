import { createMemo, createSignal, For, Match, mergeProps, Show, Switch, onCleanup, onMount, type JSX } from "solid-js";
import type { VideoClipLayerRuntimeSummary } from "../types";
import type { VideoControlPanelProps } from "./VideoControlPanel";
import { outputDriveShortLabel } from "../outputDriveStatus";
import { LiveVideoMonitorPanel } from "./LiveVideoMonitorPanel";
import { LiveVideoOutputHealthReadout } from "./LiveVideoOutputHealthReadout";
import { VideoStagePreviewButton } from "./VideoClipGridPanel";
import { VideoPerformMatrix } from "./VideoPerformMatrix";
import { VideoPerformQuickFx } from "./VideoPerformQuickFx";
import { PaneIconButton, WorkspacePaneHeader } from "./WorkspacePaneHeader";
import { WorkspaceSplitHandle } from "./WorkspaceSplitHandle";
import { defaultWorkspaceLayout } from "../workspaceLayoutStorage";
import { controlMappingTargetData } from "../controlMappingLearn";
import { FX_DRAG_MIME, readFxDragPayload, type FxCatalogEntry, type FxImportRejection } from "../fxCatalogModel";
import { collectFxProblems } from "../fxRejectionExplanation";
import { VideoFxProblemsDialog } from "./VideoFxProblemsDialog";
import { MediaLibraryHealthDialog } from "./MediaHealthDialog";
import { summarizeMediaHealth } from "../mediaHealthModel";

/** Persisted pane ratios for the Perform workspace; the application owns them
 * (workspace layout storage) and the desk falls back to local ones without it. */
export interface VideoPerformLayoutState {
  topRatio: number;
  lowerRatio: number;
  onTopRatio: (ratio: number) => void;
  onLowerRatio: (ratio: number) => void;
}

export interface VideoPerformContext {
  runtimes: VideoClipLayerRuntimeSummary[];
  keyboardEnabled: boolean;
  onOpenEditor: () => void;
  layout?: VideoPerformLayoutState;
  /** The existing Clip Inspector, rendered in-flow for the Inspector's Clip tab. */
  renderClipInspector?: () => JSX.Element;
  /** Task 12's effect browser plugs into the Inspector's Browser tab here. */
  renderEffectBrowser?: (onOpenFx?: () => void) => JSX.Element;
  effectEntries?: FxCatalogEntry[];
  isfImportRejections?: FxImportRejection[];
  locale?: string;
  onAddEffect?: (entry: FxCatalogEntry, layerId: number) => void | Promise<void>;
}

type InspectorTab = "fx" | "clip" | "browser";
type PaneFocus = "matrix" | "output" | "inspector" | null;

const inspectorTabs: { id: InspectorTab; label: string }[] = [
  { id: "fx", label: "FX" },
  { id: "clip", label: "Clip" },
  { id: "browser", label: "Browser" },
];

// Height of a complete clip-matrix layer row (100px plus its 2px rule) and of the
// pane chrome around the rows: pane borders, pane header, matrix head and the
// horizontal scrollbar. The top pane never shrinks below the rows the desk must
// keep visible.
const matrixRowPx = 102;
const matrixChromePx = 2 + 44 + 44 + 16;

/** A presentation adapter over the existing publication, transport and FX owners. */
export function VideoPerformDesk(props: VideoControlPanelProps) {
  let legend!: HTMLDialogElement;
  let blackout!: HTMLButtonElement;
  const blackoutBlockedReason = () => props.masterControls.blackoutBlockedReason ?? null;
  const keyboardEnabled = () => props.performContext?.keyboardEnabled !== false;
  const shortcut = (event: KeyboardEvent) => {
    if (!keyboardEnabled() || event.repeat || event.isComposing || event.defaultPrevented ||
      document.querySelector("dialog[open]") || (event.target instanceof Element &&
        event.target.closest("input,select,textarea,[contenteditable=true]"))) return;
    if (event.ctrlKey && event.shiftKey && event.code === "KeyB" && !event.altKey && !event.metaKey) {
      event.preventDefault(); event.stopImmediatePropagation(); blackout.click();
    } else if (event.key === "?" && !event.ctrlKey && !event.altKey && !event.metaKey) {
      event.preventDefault(); event.stopImmediatePropagation(); legend.showModal();
    }
  };
  onMount(() => window.addEventListener("keydown", shortcut, true));
  onCleanup(() => window.removeEventListener("keydown", shortcut, true));
  const mediaHealth = createMemo(() => summarizeMediaHealth(props.mediaLibrary.assets, props.mediaLibrary.availabilityById));
  // One line for every broken media file; the health view lists which ones.
  const mediaProblemSummary = () => {
    const { counts } = mediaHealth();
    const ja = props.performContext?.locale === "ja";
    const parts = ([
      ["missing", ja ? "見つからない" : "missing"],
      ["unreadable", ja ? "読み取り不可" : "unreadable"],
      ["hash_mismatch", ja ? "内容が不一致" : "content differs"],
    ] as const).filter(([state]) => counts[state] > 0).map(([state, word]) => ja ? `${word} ${counts[state]}` : `${counts[state]} ${word}`);
    return parts.length > 0 ? `${ja ? "メディアの問題" : "Media problems"}: ${parts.join(ja ? "・" : ", ")}` : "";
  };
  let mediaHealthDialog!: HTMLDialogElement;
  const unchecked = () => props.mediaLibrary.assets.filter(asset =>
    !props.mediaLibrary.availabilityById[asset.id] ||
    props.mediaLibrary.availabilityById[asset.id]?.kind === "available_unverified").length;
  let fxProblemsDialog!: HTMLDialogElement;
  const fxProblems = createMemo(() => collectFxProblems({
    layers: props.layerList.layers, runtimeErrors: props.layerList.isfRuntimeErrors ?? [],
    importRejections: props.performContext?.isfImportRejections ?? [], locale: props.performContext?.locale,
  }));
  const errors = () => [...new Set([props.clipGrid.previewError, props.clipGrid.firstRunError, props.liveMonitors.preview.error, props.liveMonitors.program.error,
    props.liveMonitors.previewTransportError,
    ...(fxProblems().length ? [fxProblems().length === 1 ? fxProblems()[0].explanation.title : (props.performContext?.locale === "ja" ? `エフェクトの問題 ${fxProblems().length}件` : `${fxProblems().length} effect problems`)] : []),
    ...(mediaProblemSummary() ? [mediaProblemSummary()] : []),
    ...props.mediaLibrary.lastImportReport?.entries.filter(entry => entry.status === "failed").map(entry => entry.message) ?? [],
  ].filter(Boolean))].join(" · ");

  // Pane arrangement: ratios are persisted by the application; collapse and
  // maximise are session state so a show always opens with the whole desk.
  const [localTop, setLocalTop] = createSignal(defaultWorkspaceLayout.perform_top_split_ratio);
  const [localLower, setLocalLower] = createSignal(defaultWorkspaceLayout.perform_lower_split_ratio);
  const topRatio = () => props.performContext?.layout?.topRatio ?? localTop();
  const lowerRatio = () => props.performContext?.layout?.lowerRatio ?? localLower();
  const commitTop = (ratio: number) => props.performContext?.layout ? props.performContext.layout.onTopRatio(ratio) : setLocalTop(ratio);
  const commitLower = (ratio: number) => props.performContext?.layout ? props.performContext.layout.onLowerRatio(ratio) : setLocalLower(ratio);
  const [focused, setFocused] = createSignal<PaneFocus>(null);
  const [bottomCollapsed, setBottomCollapsed] = createSignal(false);
  const [tab, setTab] = createSignal<InspectorTab>("fx");
  const toggleFocus = (pane: Exclude<PaneFocus, null>) => setFocused(focused() === pane ? null : pane);
  const showClipInspector = () => { setFocused(current => current === "matrix" ? null : current); setBottomCollapsed(false); setTab("clip"); };
  // Clip… and right-click select the clip first, then open its options. In
  // Perform they open the Inspector's Clip tab instead of a popover.
  const matrixBank = mergeProps(props.clipSlotBank, { onMore: () => showClipInspector() });
  const minTopPx = () => matrixChromePx + matrixRowPx * Math.max(1, Math.min(3, props.clipSlotBank.layers.length));
  const outputContext = createMemo(() => props.liveMonitors.programLabel ?? "");

  return <section id="control-domain-panel-video" role="tabpanel" aria-labelledby="control-domain-video"
    class="panel videoControlPanel videoPerformLayout" data-video-perform-layout aria-label="VJ performance desk"
    data-perform-focus={focused() ?? undefined} data-perform-bottom-collapsed={bottomCollapsed() ? "true" : undefined}>
    <section class="videoPerformPane videoPerformPaneMatrix" data-video-perform-pane="matrix" aria-labelledby="video-perform-title">
      <WorkspacePaneHeader
        title="Perform"
        titleId="video-perform-title"
        tools={<>
          <section class="videoPerformHealth" data-video-perform-region="health" aria-label="Media and monitor health">
            <strong>Media health</strong><span title={errors() || undefined}><Show when={errors()} fallback={<><span>Assets</span>: <span data-no-localize>{props.mediaLibrary.assets.length}</span> · <span>Not verified</span>: <span data-no-localize>{unchecked()}</span></>}>{error => <span data-no-localize>{error()}</span>}</Show></span>
            <button type="button" data-media-health-open title="See which media files are missing or changed, and fix them" onClick={() => mediaHealthDialog.showModal()}>Details</button>
            <Show when={fxProblems().length > 0}>
              <button type="button" data-fx-problems-open onClick={() => fxProblemsDialog.showModal()}>Effect problems</button>
            </Show>
            <Show when={!props.clipGrid.thumbnailsAuthorized}><button onClick={props.clipGrid.onRequestThumbnails}>Load Thumbnails</button></Show>
            <Show when={props.liveMonitors.preview.status === "error" || props.liveMonitors.program.status === "error"}>
              <button onClick={props.liveMonitors.onRetry}>Retry monitors</button>
            </Show>
          </section>
          <button data-video-perform-legend-toggle onClick={() => legend.showModal()}>Keyboard shortcuts</button>
        </>}
        actions={<PaneIconButton kind="maximise" active={focused() === "matrix"} controls="video-perform-matrix-body"
          onToggle={() => toggleFocus("matrix")} data={{ "data-video-perform-maximise": "matrix" }} />}
      />
      <div class="workspacePaneBody" id="video-perform-matrix-body">
        <VideoPerformMatrix bank={matrixBank} layerList={props.layerList}
          effectEntries={props.performContext?.effectEntries} onAddEffect={props.performContext?.onAddEffect}
          runtimes={props.performContext?.runtimes ?? (props.clipSlotBank.runtime ? [props.clipSlotBank.runtime] : [])}
          availabilityById={props.mediaLibrary.availabilityById} previewLayerId={props.clipGrid.previewLayerId}
          thumbnailsAuthorized={props.clipGrid.thumbnailsAuthorized} keyboardEnabled={keyboardEnabled()}
          onStop={layerId => props.clipGrid.onStop(layerId, props.clipGrid.fadeMs)} onTake={() => { if (props.clipSlotTake.enabled) return props.clipSlotTake.onTake(); }} />
      </div>
    </section>
    <WorkspaceSplitHandle
      axis="horizontal"
      ratio={topRatio()}
      defaultRatio={defaultWorkspaceLayout.perform_top_split_ratio}
      minFirstPx={minTopPx()}
      minSecondPx={188}
      label="Resize Perform matrix and output panes"
      splitter="upper-lower"
      onCommit={commitTop}
    />
    <div class="videoPerformBottom" data-video-perform-band>
      <section class="videoPerformPane videoPerformPaneOutput" data-video-perform-pane="output" data-video-perform-region="output" aria-labelledby="video-perform-output-title">
        <WorkspacePaneHeader
          title="Output"
          titleId="video-perform-output-title"
          context={<span data-no-localize>{outputContext()}</span>}
          tools={<LiveVideoOutputHealthReadout outputs={props.liveMonitors.outputHealth} error={props.liveMonitors.outputHealthError} />}
          actions={<>
            <PaneIconButton kind="collapse" active={bottomCollapsed()} controls="video-perform-output-body"
              onToggle={() => setBottomCollapsed(!bottomCollapsed())} data={{ "data-video-perform-collapse": "output" }} />
            <PaneIconButton kind="maximise" active={focused() === "output"} controls="video-perform-output-body"
              onToggle={() => toggleFocus("output")} data={{ "data-video-perform-maximise": "output" }} />
          </>}
        />
        <div class="workspacePaneBody videoPerformOutputBody" id="video-perform-output-body">
          <section class="videoPerformMonitors" aria-label="Preview and Program">
            <LiveVideoMonitorPanel {...props.liveMonitors} />
          </section>
          <div class="videoPerformControls">
            <section class="videoPerformTransition" data-video-perform-region="transition" aria-label="Performance transition">
              <div class="videoPerformTransitionRow">
                <label><span class="videoPerformFieldLabel">Transition</span><select data-video-perform-transition-kind value={props.clipSlotBank.transitionKind}
                  disabled={Boolean(props.clipSlotBank.runtime?.transition)} onInput={event => props.clipSlotBank.onSetTransitionKind(event.currentTarget.value as typeof props.clipSlotBank.transitionKind)}>
                  <For each={["Cut", "Crossfade", "Dip", "Wipe", "Luma", "Displacement", "Blur", "Glitch", "Custom"]}>{kind => <option value={kind}>{kind}</option>}</For>
                </select></label>
                <label class="videoPerformDuration"><span class="videoPerformFieldLabel">Time</span><input type="number" data-video-perform-transition-duration min="0.001" max="600" step="0.001"
                  value={props.clipSlotBank.transitionDurationMilliunits / 1000}
                  disabled={Boolean(props.clipSlotBank.runtime?.transition)}
                  onChange={event => { const value = Number(event.currentTarget.value); if (Number.isFinite(value) && value > 0) props.clipSlotBank.onSetTransitionDurationMilliunits(value * 1000); }} />
                  <select aria-label="Transition time unit" value={props.clipSlotBank.transitionDurationUnit}
                    disabled={Boolean(props.clipSlotBank.runtime?.transition)} onInput={event => props.clipSlotBank.onSetTransitionDurationUnit(event.currentTarget.value as typeof props.clipSlotBank.transitionDurationUnit)}>
                    <option value="Milliseconds">Seconds</option><option value="Beats">Beats</option><option value="Bars">Bars</option>
                  </select>
                </label>
              </div>
              <div class="videoPerformTakeRow">
                <VideoStagePreviewButton layer={props.clipSlotBank.layer} previewLayerId={props.clipGrid.previewLayerId}
                  backendAvailable={props.clipGrid.previewBackendAvailable} busy={props.clipGrid.previewBusy} onStagePreview={props.clipGrid.onStagePreview} />
                <button class="primary" data-video-perform-take data-video-clip-slot-take disabled={!props.clipSlotTake.enabled}
                  onClick={() => void props.clipSlotTake.onTake()}>{props.clipSlotTake.label}</button>
              </div>
            </section>
            <section class="videoPerformMaster" aria-label="Performance master">
              <label>Master <strong>{Math.round(props.masterControls.masterOpacity * 100)}%</strong>
                <input data-video-perform-master type="range" min="0" max="1" step="0.01" value={props.masterControls.masterOpacity}
                  onInput={event => void props.masterControls.onSetMasterOpacity(Number(event.currentTarget.value))}
                  {...controlMappingTargetData({ action: "VideoMaster", label: "Video Master" })} /></label>
              <div class="videoPerformBlackoutRow">
                <button ref={blackout} class={`killButton${props.masterControls.blackout ? " engaged" : ""}${blackoutBlockedReason() ? " blocked" : ""}`} data-video-perform-blackout
                  aria-pressed={props.masterControls.blackout} onClick={() => void props.masterControls.onSetBlackout(!props.masterControls.blackout)}
                  aria-disabled={blackoutBlockedReason() ? "true" : undefined} data-blackout-blocked={blackoutBlockedReason() ? "true" : undefined}
                  aria-describedby="videoPerformBlackoutReason"
                  title={blackoutBlockedReason() ?? (props.masterControls.blackout ? "BLACKOUT ENGAGED" : "Confirmation required")}
                  {...controlMappingTargetData({ action: "VideoBlackout", label: "Video Blackout" })}>
                  <span class="videoPerformBlackoutLabel">{props.masterControls.blackout ? "Recover video…" : "Blackout…"}</span>
                  <Show when={blackoutBlockedReason()}><span class="videoPerformBlackoutShort">{outputDriveShortLabel(blackoutBlockedReason() ?? "")}</span></Show>
                </button>
                <span class={`videoPerformBlackoutHint${blackoutBlockedReason() ? " blocked" : ""}`} id="videoPerformBlackoutReason"
                  data-video-perform-blackout-reason role="status">
                  <svg viewBox="0 0 24 24" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><rect x="5" y="11" width="14" height="9" /><path d="M8 11V8a4 4 0 0 1 8 0v3" /></svg>
                  <span>{blackoutBlockedReason() ?? (props.masterControls.blackout ? "BLACKOUT ENGAGED" : "Confirmation required")}</span>
                </span>
              </div>
            </section>
          </div>
        </div>
      </section>
      <WorkspaceSplitHandle
        axis="vertical"
        ratio={lowerRatio()}
        defaultRatio={defaultWorkspaceLayout.perform_lower_split_ratio}
        minFirstPx={520}
        minSecondPx={400}
        label="Resize Output and Inspector panes"
        splitter="lower-left-right"
        onCommit={commitLower}
      />
      <section class="videoPerformPane videoPerformPaneInspector" data-video-perform-pane="inspector" aria-labelledby="video-perform-inspector-title">
        <WorkspacePaneHeader
          title="Inspector"
          titleId="video-perform-inspector-title"
          tabs={<div class="workspacePaneTabs" role="tablist" aria-label="Inspector">
            <For each={inspectorTabs}>{item =>
              <button type="button" role="tab" id={`video-perform-inspector-tab-${item.id}`} data-video-perform-inspector-tab={item.id}
                onDragOver={event => { if (item.id === "fx" && event.dataTransfer?.types.includes(FX_DRAG_MIME)) { event.preventDefault(); event.dataTransfer.dropEffect = "copy"; } }}
                onDrop={event => {
                  if (item.id !== "fx" || !event.dataTransfer) return;
                  const entry = readFxDragPayload(event.dataTransfer, props.performContext?.effectEntries ?? []);
                  const layer = props.clipSlotBank.layer;
                  if (!entry || !layer) return;
                  event.preventDefault(); setTab("fx");
                  void props.performContext?.onAddEffect?.(entry, layer.id);
                }}
                onKeyDown={event => {
                  if (!["ArrowLeft", "ArrowRight", "Home", "End"].includes(event.key)) return;
                  event.preventDefault(); event.stopPropagation();
                  const index = inspectorTabs.findIndex(entry => entry.id === item.id);
                  const next = event.key === "Home" ? 0 : event.key === "End" ? inspectorTabs.length - 1 : (index + (event.key === "ArrowRight" ? 1 : -1) + inspectorTabs.length) % inspectorTabs.length;
                  setTab(inspectorTabs[next].id);
                  document.getElementById(`video-perform-inspector-tab-${inspectorTabs[next].id}`)?.focus();
                }}
                aria-selected={tab() === item.id} aria-controls={`video-perform-inspector-panel-${item.id}`}
                tabIndex={tab() === item.id ? 0 : -1} classList={{ active: tab() === item.id }}
                onClick={() => setTab(item.id)}>{item.label}</button>}
            </For>
          </div>}
          actions={<>
            <PaneIconButton kind="collapse" active={bottomCollapsed()} controls="video-perform-inspector-body"
              onToggle={() => setBottomCollapsed(!bottomCollapsed())} data={{ "data-video-perform-collapse": "inspector" }} />
            <PaneIconButton kind="maximise" active={focused() === "inspector"} controls="video-perform-inspector-body"
              onToggle={() => toggleFocus("inspector")} data={{ "data-video-perform-maximise": "inspector" }} />
          </>}
        />
        <div class="workspacePaneBody videoPerformInspectorBody" id="video-perform-inspector-body"
          role="tabpanel" aria-labelledby={`video-perform-inspector-tab-${tab()}`} data-video-perform-inspector-panel={tab()}>
          <Switch>
            <Match when={tab() === "fx"}>
              <VideoPerformQuickFx layer={props.clipSlotBank.layer} layerList={props.layerList}
                locale={props.performContext?.locale} bpm={props.layerList.bpm}
                projectEpoch={props.layerList.projectEpoch}
                effectCatalog={props.effectScopes.catalog} onApplyEffectCatalog={props.effectScopes.onApplyCatalog} liveSession={props.layerList.liveSession} audition={props.layerList.audition}
                onOpenEditor={() => props.performContext?.onOpenEditor()} />
            </Match>
            <Match when={tab() === "clip"}>
              <div class="videoPerformInspectorClip" data-video-perform-clip-inspector>
                {props.performContext?.renderClipInspector?.() ?? <p class="emptyState">Select a clip to inspect it.</p>}
              </div>
            </Match>
            <Match when={tab() === "browser"}>
              <div class="videoPerformInspectorBrowser" data-video-perform-effect-browser>
                {props.performContext?.renderEffectBrowser?.(() => setTab("fx")) ?? <>
                  <strong>Effect browser</strong>
                  <p>Browse effects and add them to the selected layer from this tab. The browser is not available yet; use Full FX editor in the FX tab to add effects now.</p>
                </>}
              </div>
            </Match>
          </Switch>
        </div>
      </section>
    </div>
    <VideoFxProblemsDialog ref={(element) => { fxProblemsDialog = element; }} problems={fxProblems()} locale={props.performContext?.locale} onClose={() => undefined} />
    <MediaLibraryHealthDialog library={props.mediaLibrary} ref={(element) => { mediaHealthDialog = element; }} />
    <dialog ref={legend} class="videoPerformLegend" data-video-perform-legend>
      <h2>Keyboard shortcuts</h2>
      <dl><dt data-no-localize>F1 / F2 / F3</dt><dd>Setup / Edit / Control</dd>
        <dt data-no-localize>Alt + 1 / 2 / 3</dt><dd>Lighting / Video / Both</dd>
        <dt>1–9, 0, Q–P, A–L, Z–C</dt><dd>Queue a clip in the selected layer</dd>
        <dt>← → ↑ ↓ · Home / End</dt><dd>Move clip and layer selection</dd>
        <dt>Space / Enter</dt><dd>Take the queued clip</dd>
        <dt data-no-localize>Ctrl + Shift + B</dt><dd>Blackout or recover, with confirmation</dd>
        <dt>?</dt><dd>Show keyboard shortcuts</dd></dl>
      <p>Right-click or long-press a clip for its options. Inspect and Clip… in the matrix corner act on the selected clip; Inspect shows a thumbnail only.</p>
      <p>The P button stages the current layer in Preview; the P key queues column 20.</p>
      <p>Shortcuts pause while editing a value, a dialog is open, or MIDI Learn is active.</p>
      <p>Clip, Take, blend, transition and ISF stage MIDI targets are not supported by the current control model.</p>
      <button onClick={() => legend.close()}>Close</button>
    </dialog>
  </section>;
}
