import { createEffect, createMemo, createSignal, For, Show } from "solid-js";
import { createStore, reconcile } from "solid-js/store";
import { bankAuthorityIssueMessage, type FullBankAuthoritySnapshot } from "../bankAuthority";
import type { CueMetadataDraft } from "../editorDrafts";
import type {
  ActiveFadeSummary,
  CueEffectTarget,
  CueListSummary,
  CueLiveDirection,
  CueLiveModifierSettings,
  CueStepSummary,
  CueSummary,
  EffectSummary,
  ReferencePaletteSummary,
  TimelineCueEventSummary,
  TimelineTrackKind,
} from "../types";
import { authoredCueLiveModifier } from "../cueLiveModifier";
import type { CueEffectRecallChange } from "../cueEffectRecall";
import { timelineConformRateBadge } from "../timelineSceneBlocks";
import { cueIdentityCss } from "../identityColor";
import { displayNumber } from "../numberDisplay";
import type { TimelineCueDragPoint } from "../timelineCueDrag";
import { CueCapturePreviewPanel, type CueCapturePreviewModel } from "./CueCapturePreviewPanel";
import { CueEffectRecallEditor } from "./CueEffectRecallEditor";
import { CueStepEditor } from "./CueStepEditor";

export type CueCaptureScopeMode = "all" | "lighting" | "effects" | "selectedFixture" | "selectedGroup" | "video";

const cuesPerPage = 12;
const timelinePlacementsPerCue = 8;

const cueCaptureScopeLabel = (scope: CueCaptureScopeMode) => {
  if (scope === "all") return "All Sources";
  if (scope === "lighting") return "Lighting Only";
  if (scope === "effects") return "Effects Only";
  if (scope === "selectedFixture") return "Selected Fixture";
  if (scope === "selectedGroup") return "Selected Group";
  return "Video Only";
};

interface CueManagementPanelProps {
  mode: "edit" | "live" | "scene-settings";
  onSetCueColor: (cueId: number, color: string | null) => void | Promise<void>;
  groupColors?: Record<string, string>;
  onSetCueLiveModifierDefaults: (
    cueId: number,
    settings: CueLiveModifierSettings | null,
  ) => void | Promise<void>;
  bankAuthority: FullBankAuthoritySnapshot;
  cues: CueSummary[];
  groupIds: string[];
  palettes: ReferencePaletteSummary[];
  effects: EffectSummary[];
  cueCaptureEffects: EffectSummary[];
  selectedCueListId: number | null;
  cueListLabel: string;
  activeCueId: number | null | undefined;
  revealCueId: number | null;
  revealCueRevision: number;
  activeFade: ActiveFadeSummary | null | undefined;
  timelinePositionMs: number;
  bpm: number;
  timelineTrack: TimelineTrackKind;
  cueLabel: string;
  cueFadeMs: number;
  cueAuthoredBeats: number | null;
  cueAuthoredBeatsSeeded: boolean;
  cueAuthoredBeatsError: string | null;
  cueCaptureScope: CueCaptureScopeMode;
  cueCaptureScopeError: string | null;
  hasCueSources: boolean;
  cueEffectCaptureTargets: CueEffectTarget[];
  cueCapturePreview: CueCapturePreviewModel;
  stageViewBoxSize: number;
  stageOrigin: { x: number; z: number };
  selectedFixtureId: number | null;
  timelinePlacementNudgeMs: number;
  cueMetadataDraft: (cue: CueSummary) => CueMetadataDraft;
  cueTimelinePlacementsForCue: (cueId: number) => TimelineCueEventSummary[];
  onCueLabel: (value: string) => void;
  onCueFadeMs: (value: number) => void;
  onCueAuthoredBeats: (value: number | null) => void;
  onCueCaptureScope: (scope: CueCaptureScopeMode) => void;
  onCueEffectCaptureTargets: (targets: CueEffectTarget[], change: CueEffectRecallChange) => void;
  onSelectCueList: (cueListId: number) => void;
  onCueListLabel: (label: string) => void;
  onCreateCueList: () => void | boolean | Promise<void | boolean>;
  onRenameCueList: () => void | boolean | Promise<void | boolean>;
  onRemoveCueList: (cueListId?: number, confirmed?: boolean) => void | boolean | Promise<void | boolean>;
  onSetCueList: (cueId: number, cueListId: number) => void | Promise<void>;
  onSetCuePalette: (cue: CueSummary, paletteId: number, enabled: boolean) => void | Promise<void>;
  onTriggerCueList: (cueListId: number, direction: "next" | "previous") => void | Promise<void>;
  onCreateCue: () => void | Promise<void>;
  onSelectFixture: (fixtureId: number) => void;
  onTriggerPreviousCue: () => void | Promise<void>;
  onTriggerNextCue: () => void | Promise<void>;
  onSetCueFadePaused: (paused: boolean) => void | Promise<void>;
  onUpdateCueMetadataDraft: (cue: CueSummary, patch: Partial<CueMetadataDraft>) => void;
  onMoveCue: (cueId: number, delta: -1 | 1) => void | Promise<void>;
  onSetCueMetadata: (cue: CueSummary) => void | Promise<void>;
  onSetCueEffectTargets: (cueId: number, effectTargets: CueEffectTarget[]) => void | Promise<void>;
  onSetCueSteps: (cueId: number, steps: CueStepSummary[]) => void | Promise<void>;
  onDuplicateCue: (cue: CueSummary) => void | Promise<void>;
  onUpdateCue: (cueId: number, label: string, fadeMs: number) => void | Promise<void>;
  onTriggerCue: (cueId: number) => void | Promise<void>;
  onAddTimelineCueEventAt: (
    cueId: number | null,
    timeMs: number,
    track: TimelineTrackKind,
    nextDraftTime?: boolean,
  ) => void | Promise<void>;
  onRemoveCue: (cueId: number, confirmed?: boolean) => void | boolean | Promise<void | boolean>;
  onSeekTimeline: (timeMs: number) => void | Promise<void>;
  onOpenTimeline: () => void;
  onMoveTimelineCueEvent: (event: TimelineCueEventSummary, deltaMs: number) => void | Promise<void>;
  onRemoveTimelineEvent: (eventId: number) => void | Promise<void>;
  onBeginTimelineCueDrag: (cue: CueSummary, point: TimelineCueDragPoint) => void;
  onMoveTimelineCueDrag: (point: TimelineCueDragPoint) => void;
  onEndTimelineCueDrag: (point: TimelineCueDragPoint, moved: boolean, canceled: boolean) => void;
}

export function CueManagementPanel(props: CueManagementPanelProps) {
  const [cuePage, setCuePage] = createSignal(0);
  const [cueEditing, setCueEditing] = createSignal(props.mode === "scene-settings");
  const [timelineDragCueId, setTimelineDragCueId] = createSignal<number | null>(null);
  const bankAuthority = () => props.bankAuthority;
  // Preserve cue-editor controls across snapshot polling. Solid's keyed <For>
  // otherwise sees each deserialized Cue object as a replacement and remounts
  // native selects/inputs while an operator is using them.
  const [stableCues, setStableCues] = createStore<CueSummary[]>(
    bankAuthority().issue === null ? props.cues : [],
  );
  // Use the exact persisted Bank rows shared by Lighting, Timeline, and the
  // executor. Reconcile by the persisted id so polling replaces field values
  // without remounting the native select/options or executor rows mid-edit.
  const cueListSnapshot = createMemo(() => bankAuthority().issue === null ? [...bankAuthority().cueLists] : null);
  const [stableCueLists, setStableCueLists] = createStore<CueListSummary[]>(
    cueListSnapshot() ?? [],
  );
  createEffect(() => {
    const snapshot = cueListSnapshot();
    setStableCueLists(reconcile(snapshot ?? [], { key: "id" }));
  });
  createEffect(() => {
    setStableCues(reconcile(bankAuthority().issue === null ? props.cues : [], { key: "id" }));
  });
  let timelineDragPointer: {
    cueId: number;
    pointerId: number;
    startClientX: number;
    startClientY: number;
    moved: boolean;
  } | null = null;
  let cuePanelElement: HTMLDivElement | undefined;
  const cuePageCount = createMemo(() => Math.max(1, Math.ceil(stableCues.length / cuesPerPage)));
  const visibleCues = createMemo(() => {
    const start = cuePage() * cuesPerPage;
    return stableCues.slice(start, start + cuesPerPage);
  });

  createEffect(() => {
    if (cuePage() >= cuePageCount()) setCuePage(cuePageCount() - 1);
  });

  const beginTimelineCueDrag = (
    event: PointerEvent & { currentTarget: HTMLButtonElement },
    cue: CueSummary,
  ) => {
    if (event.button !== 0) return;
    event.preventDefault();
    event.stopPropagation();
    event.currentTarget.setPointerCapture(event.pointerId);
    timelineDragPointer = {
      cueId: cue.id,
      pointerId: event.pointerId,
      startClientX: event.clientX,
      startClientY: event.clientY,
      moved: false,
    };
    setTimelineDragCueId(cue.id);
    props.onBeginTimelineCueDrag(cue, {
      pointerId: event.pointerId,
      clientX: event.clientX,
      clientY: event.clientY,
    });
  };

  const moveTimelineCueDrag = (event: PointerEvent & { currentTarget: HTMLButtonElement }) => {
    const drag = timelineDragPointer;
    if (!drag || drag.pointerId !== event.pointerId) return;
    event.preventDefault();
    event.stopPropagation();
    drag.moved = drag.moved
      || Math.hypot(event.clientX - drag.startClientX, event.clientY - drag.startClientY) >= 4;
    props.onMoveTimelineCueDrag({
      pointerId: event.pointerId,
      clientX: event.clientX,
      clientY: event.clientY,
    });
  };

  const finishTimelineCueDrag = (
    event: PointerEvent & { currentTarget: HTMLButtonElement },
    canceled: boolean,
  ) => {
    const drag = timelineDragPointer;
    if (!drag || drag.pointerId !== event.pointerId) return;
    event.preventDefault();
    event.stopPropagation();
    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }
    timelineDragPointer = null;
    setTimelineDragCueId(null);
    props.onEndTimelineCueDrag({
      pointerId: event.pointerId,
      clientX: event.clientX,
      clientY: event.clientY,
    }, drag.moved, canceled);
  };

  let lastRevealRevision = -1;
  let revealPriorityActive = false;
  let activeCueIdAtReveal: number | null | undefined = undefined;
  createEffect(() => {
    const revision = props.revealCueRevision;
    const revealCueId = props.revealCueId;
    const cues = stableCues;
    if (revision === lastRevealRevision || revealCueId === null) return;
    const revealIndex = cues.findIndex((cue) => cue.id === revealCueId);
    if (revealIndex < 0) return;
    lastRevealRevision = revision;
    revealPriorityActive = true;
    activeCueIdAtReveal = props.activeCueId;
    setCuePage(Math.floor(revealIndex / cuesPerPage));
    if (props.mode === "live") setCueEditing(true);
    queueMicrotask(() => requestAnimationFrame(() => requestAnimationFrame(() => {
      const target = cuePanelElement
        ?.querySelector<HTMLElement>(`[data-cue-id="${revealCueId}"]`);
      if (!cuePanelElement || !target) return;
      const panelRect = cuePanelElement.getBoundingClientRect();
      const targetRect = target.getBoundingClientRect();
      cuePanelElement.scrollTop += targetRect.top - panelRect.top;
    })));
  });

  createEffect(() => {
    const activeCueId = props.activeCueId;
    if (revealPriorityActive && activeCueId === activeCueIdAtReveal) return;
    if (activeCueId !== activeCueIdAtReveal) revealPriorityActive = false;
    if (activeCueId === null || activeCueId === undefined) return;
    const activeIndex = stableCues.findIndex((cue) => cue.id === activeCueId);
    if (activeIndex >= 0) setCuePage(Math.floor(activeIndex / cuesPerPage));
  });

  return (
    <div ref={(element) => { cuePanelElement = element; }} class={
      props.mode === "live"
        ? `cuePanel cuePanelLive ${cueEditing() ? "cuePanelEditing" : ""}`
        : props.mode === "scene-settings"
          ? "cuePanel cuePanelSceneSettings cuePanelEditing"
          : "cuePanel"
    }>
      <Show
        when={bankAuthority().issue === null}
        fallback={
          <p
            class="empty"
            role="alert"
            data-bank-authority-unavailable={bankAuthority().issue?.kind}
          >
            {bankAuthority().issue ? bankAuthorityIssueMessage(bankAuthority().issue!) : ""}
          </p>
        }
      >
      <Show when={props.mode !== "scene-settings"}>
      <div class="panelHeader">
        <h2>Cues</h2>
        <div class="cuePanelHeaderActions">
          <span>{stableCues.length}</span>
          <Show when={props.mode === "live"}>
            <button
              type="button"
              class="cuePanelEditToggle"
              aria-expanded={cueEditing()}
              aria-controls="cue-list-editor cue-store-form cue-effect-capture-editor cue-list-items"
              onClick={() => setCueEditing((current) => !current)}
            >
              {cueEditing() ? "Done" : "Edit Cues"}
            </button>
          </Show>
        </div>
      </div>
      </Show>
      <Show when={props.mode !== "scene-settings"}>
      <div id="cue-list-editor" class="cueListManager cueEditOnly" aria-label="Bank manager">
        <label>
          Bank
          <select value={props.selectedCueListId ?? ""} onInput={(event) => props.onSelectCueList(Number(event.currentTarget.value))}>
            <For each={stableCueLists}>{(cueList) => <option data-no-localize value={cueList.id}>{cueList.label}</option>}</For>
          </select>
        </label>
        <label>
          Bank name
          <input maxlength="64" value={props.cueListLabel} onInput={(event) => props.onCueListLabel(event.currentTarget.value)} />
        </label>
        <button onClick={() => void props.onCreateCueList()}>New Bank</button>
        <button onClick={() => void props.onRenameCueList()}>Rename Bank</button>
        <button class="danger" disabled={stableCueLists.length <= 1} onClick={() => void props.onRemoveCueList()}>Remove Bank</button>
      </div>
      </Show>
      <Show when={props.mode !== "scene-settings"}>
      <div class="cueExecutorBank" aria-label="Bank executors">
        <For each={stableCueLists}>
          {(cueList) => {
            const activeCue = () => {
              const bankActiveCueId = cueList.active_cue_id;
              if (bankActiveCueId !== null && bankActiveCueId !== undefined) {
                return bankAuthority().cueById.get(bankActiveCueId);
              }
              const globalActiveCueId = props.activeCueId;
              return cueList.id === props.selectedCueListId
                && globalActiveCueId !== null
                && globalActiveCueId !== undefined
                ? bankAuthority().cueById.get(globalActiveCueId)
                : undefined;
            };
            const cueCount = () => bankAuthority().cues.filter((cue) => cue.cue_list_id === cueList.id).length;
            return (
              <div class={`cueExecutor ${cueList.id === props.selectedCueListId ? "selected" : ""}`}>
                <button class="cueExecutorSelect" aria-pressed={cueList.id === props.selectedCueListId} onClick={() => props.onSelectCueList(cueList.id)}>
                  <strong data-no-localize>{cueList.label}</strong>
                  <small data-no-localize>{activeCue() ? `${activeCue()!.cue_number || activeCue()!.id} ${activeCue()!.label}` : `${cueCount()} cue(s) · Ready`}</small>
                </button>
                <button disabled={cueCount() === 0} aria-label={`Back ${cueList.label}`} onClick={() => void props.onTriggerCueList(cueList.id, "previous")}>Back</button>
                <button disabled={cueCount() === 0} class="primary" aria-label={`GO ${cueList.label}`} onClick={() => void props.onTriggerCueList(cueList.id, "next")}>GO</button>
              </div>
            );
          }}
        </For>
      </div>
      </Show>
      <Show when={props.mode !== "scene-settings"}>
      <div id="cue-store-form" class="cueForm">
        <label>
          Label
          <input value={props.cueLabel} onInput={(event) => props.onCueLabel(event.currentTarget.value)} />
        </label>
        <label>
          Fade ms
          <input
            class="tabularNums"
            type="number"
            min="0"
            value={displayNumber(props.cueFadeMs, 0)}
            onInput={(event) => props.onCueFadeMs(Number(event.currentTarget.value))}
            onBlur={(event) => {
              event.currentTarget.value = displayNumber(props.cueFadeMs, 0);
            }}
          />
        </label>
        <label>
          Authored beats
          <input
            class="tabularNums"
            type="number"
            min="0.25"
            max="1024"
            step="0.25"
            placeholder="Optional"
            value={displayNumber(props.cueAuthoredBeats, 3)}
            aria-invalid={Boolean(props.cueAuthoredBeatsError)}
            aria-describedby={props.cueAuthoredBeatsError ? "cue-authored-beats-error" : undefined}
            title={props.cueAuthoredBeatsSeeded ? "Seeded from enabled Effect timing." : "Scene length in beats."}
            onInput={(event) => props.onCueAuthoredBeats(
              event.currentTarget.value === "" ? null : Number(event.currentTarget.value),
            )}
            onBlur={(event) => {
              event.currentTarget.value = displayNumber(props.cueAuthoredBeats, 3);
            }}
          />
        </label>
        <label>
          Scope
          <select value={props.cueCaptureScope} onInput={(event) => props.onCueCaptureScope(event.currentTarget.value as CueCaptureScopeMode)}>
            <option value="all">All Sources</option>
            <option value="lighting">Lighting Only</option>
            <option value="effects">Effects Only</option>
            <option value="selectedFixture">Selected Fixture</option>
            <option value="selectedGroup">Selected Group</option>
            <option value="video">Video Only</option>
          </select>
        </label>
        <button
          class="primary"
          onClick={() => void props.onCreateCue()}
          disabled={
            (!props.hasCueSources && props.cueEffectCaptureTargets.length === 0)
            || Boolean(props.cueCaptureScopeError)
            || Boolean(props.cueAuthoredBeatsError)
          }
        >
          Store Cue
        </button>
        <Show when={props.cueCaptureScopeError}>
          {(error) => <span class="cueScopeHint invalid">{error()}</span>}
        </Show>
        <Show when={props.cueAuthoredBeatsError}>
          {(error) => <span id="cue-authored-beats-error" class="cueScopeHint invalid">{error()}</span>}
        </Show>
      </div>
      </Show>
      <Show when={props.mode !== "scene-settings"}>
      <CueEffectRecallEditor
        id="cue-effect-capture-editor"
        effects={props.cueCaptureEffects}
        hasAnyEffects={props.effects.length > 0}
        targets={props.cueEffectCaptureTargets}
        expanded
        currentMode="capture"
        applyHint="Only Effects matching this Cue scope are listed. Mixed Lighting/Video Effects appear in both scopes."
        onChange={props.onCueEffectCaptureTargets}
      />
      <CueCapturePreviewPanel
        preview={props.cueCapturePreview}
        invalid={Boolean(props.cueCaptureScopeError)}
        stageViewBoxSize={props.stageViewBoxSize}
        stageOrigin={props.stageOrigin}
        selectedFixtureId={props.selectedFixtureId}
        onSelectFixture={props.onSelectFixture}
      />
      <div class="buttonRow">
        <button onClick={() => void props.onTriggerPreviousCue()} disabled={stableCues.length === 0}>
          Back
        </button>
        <button class="primary" onClick={() => void props.onTriggerNextCue()} disabled={stableCues.length === 0}>
          GO
        </button>
        <button
          onClick={() => void props.onSetCueFadePaused(!props.activeFade?.paused)}
          disabled={!props.activeFade}
        >
          {props.activeFade?.paused ? "Resume" : "Pause"}
        </button>
      </div>
      <Show when={props.activeFade}>
        {(fade) => (
          <div class="fadeMeter">
            <span>{fade().paused ? "Paused " : ""}{Math.round(fade().progress * 100)}%</span>
            <div>
              <i style={{ width: `${Math.round(fade().progress * 100)}%` }} />
            </div>
          </div>
        )}
      </Show>
      </Show>
      <Show when={props.mode === "scene-settings"}>
        <details class="cueUpdateSourceTools cueEditOnly" data-cue-update-source-tools>
          <summary>
            <span>Update Look source</span>
            <small>{cueCaptureScopeLabel(props.cueCaptureScope)}</small>
          </summary>
          <p>
            Choose which current outputs Update Look recaptures. This does not change the Cue's Effect Recall list.
          </p>
          <div class="cueUpdateSourceForm" data-cue-update-source-form>
            <label>
              Scope
              <select
                value={props.cueCaptureScope}
                onInput={(event) => props.onCueCaptureScope(event.currentTarget.value as CueCaptureScopeMode)}
              >
                <option value="all">All Sources</option>
                <option value="lighting">Lighting Only</option>
                <option value="effects">Effects Only</option>
                <option value="selectedFixture">Selected Fixture</option>
                <option value="selectedGroup">Selected Group</option>
                <option value="video">Video Only</option>
              </select>
            </label>
            <Show when={props.cueCaptureScopeError}>
              {(error) => <span class="cueScopeHint invalid">{error()}</span>}
            </Show>
          </div>
          <CueEffectRecallEditor
            id="cue-effect-capture-editor"
            effects={props.cueCaptureEffects}
            hasAnyEffects={props.effects.length > 0}
            targets={props.cueEffectCaptureTargets}
            expanded
            currentMode="capture"
            applyHint="Only Effects matching this Cue scope are listed. Mixed Lighting/Video Effects appear in both scopes."
            onChange={props.onCueEffectCaptureTargets}
          />
          <CueCapturePreviewPanel
            preview={props.cueCapturePreview}
            invalid={Boolean(props.cueCaptureScopeError)}
            stageViewBoxSize={props.stageViewBoxSize}
            stageOrigin={props.stageOrigin}
            selectedFixtureId={props.selectedFixtureId}
            onSelectFixture={props.onSelectFixture}
          />
        </details>
      </Show>
      <Show when={cuePageCount() > 1}>
        <nav class="cuePager" aria-label="Cue Page">
          <button aria-label="Previous cue page" onClick={() => setCuePage(Math.max(0, cuePage() - 1))} disabled={cuePage() === 0}>Prev</button>
          <span class="tabularNums">Page {cuePage() + 1} / {cuePageCount()}</span>
          <button aria-label="Next cue page" onClick={() => setCuePage(Math.min(cuePageCount() - 1, cuePage() + 1))} disabled={cuePage() >= cuePageCount() - 1}>Next</button>
        </nav>
      </Show>
      <div id="cue-list-items" class="cueList" role="list">
        <Show when={stableCues.length === 0}>
          <p class="empty">
            {props.hasCueSources || props.cueEffectCaptureTargets.length > 0
              ? "No cues. Choose a scope, then Store Cue."
              : "No cues. Patch fixtures, add video, create a Node Graph, or create an Effect first."}
          </p>
        </Show>
        <For each={visibleCues()}>
          {(cue, index) => {
            const cueIndex = () => cuePage() * cuesPerPage + index();
            const draft = () => props.cueMetadataDraft(cue);
            const authoredBeatsInvalid = () => draft().authored_beats !== null
              && (!Number.isFinite(draft().authored_beats)
                || draft().authored_beats! < 0.25
                || draft().authored_beats! > 1024);
            const placements = () => props.cueTimelinePlacementsForCue(cue.id);
            const visiblePlacements = () => placements().slice(0, timelinePlacementsPerCue);
            const hiddenPlacementCount = () => Math.max(0, placements().length - timelinePlacementsPerCue);
            const updateIfcbTiming = (patch: Partial<CueMetadataDraft["ifcb_timing"]>) =>
              props.onUpdateCueMetadataDraft(cue, {
                ifcb_timing: { ...draft().ifcb_timing, ...patch },
              });
            const updatePart = (partIndex: number, patch: Partial<CueMetadataDraft["parts"][number]>) =>
              props.onUpdateCueMetadataDraft(cue, {
                parts: draft().parts.map((part, index) => index === partIndex ? { ...part, ...patch } : part),
              });
            const addPart = () => {
              const number = Math.max(0, ...draft().parts.map((part) => part.number)) + 1;
              props.onUpdateCueMetadataDraft(cue, {
                parts: [...draft().parts, {
                  number,
                  label: `Part ${number}`,
                  delay_ms: 0,
                  fade_ms: null,
                  fixture_ids: [],
                  video_layer_ids: [],
                  video_output_ids: [],
                }],
              });
            };
            const togglePartFixture = (partIndex: number, fixtureId: number, enabled: boolean) => {
              const parts = draft().parts.map((part, index) => {
                const fixture_ids = part.fixture_ids.filter((id) => id !== fixtureId);
                return index === partIndex && enabled
                  ? { ...part, fixture_ids: [...fixture_ids, fixtureId] }
                  : { ...part, fixture_ids };
              });
              props.onUpdateCueMetadataDraft(cue, { parts });
            };
            const togglePartVideoLayer = (partIndex: number, layerId: number, enabled: boolean) => {
              const parts = draft().parts.map((part, index) => {
                const video_layer_ids = part.video_layer_ids.filter((id) => id !== layerId);
                return index === partIndex && enabled
                  ? { ...part, video_layer_ids: [...video_layer_ids, layerId] }
                  : { ...part, video_layer_ids };
              });
              props.onUpdateCueMetadataDraft(cue, { parts });
            };
            const togglePartVideoOutput = (partIndex: number, outputId: number, enabled: boolean) => {
              const parts = draft().parts.map((part, index) => {
                const video_output_ids = part.video_output_ids.filter((id) => id !== outputId);
                return index === partIndex && enabled
                  ? { ...part, video_output_ids: [...video_output_ids, outputId] }
                  : { ...part, video_output_ids };
              });
              props.onUpdateCueMetadataDraft(cue, { parts });
            };
            const mibTargetIds = () => [...new Set([
              ...cue.targets.map((target) => target.fixture_id),
              ...cue.palette_targets.flatMap((target) => target.fixture_ids),
            ])].sort((left, right) => left - right);
            const toggleMibFixture = (fixtureId: number, enabled: boolean) => {
              const fixtureIds = draft().mib_fixture_ids.filter((id) => id !== fixtureId);
              props.onUpdateCueMetadataDraft(cue, {
                mib_fixture_ids: enabled ? [...fixtureIds, fixtureId] : fixtureIds,
              });
            };
            return (
              <div
                class={cue.id === props.activeCueId ? "cueItem active" : "cueItem"}
                style={{
                  "--identity": cueIdentityCss(
                    cue.id,
                    cue.color,
                    "fill",
                    cue.group_id,
                    cue.group_id ? props.groupColors?.[cue.group_id] : null,
                  ),
                }}
                data-cue-id={cue.id}
                role="listitem"
                aria-posinset={cueIndex() + 1}
                aria-setsize={stableCues.length}
              >
                <div class="cueMetaLine">
                  <div class="cueTitleRow">
                    <strong data-no-localize><b>{cue.cue_number || cue.id}</b> {cue.label}</strong>
                    <Show when={props.mode !== "scene-settings"}>
                    <button
                      type="button"
                      class="cueTimelineDragHandle"
                      classList={{ dragging: timelineDragCueId() === cue.id }}
                      title={`Drag Cue ${cue.label} to Timeline`}
                      aria-label={`Drag Cue ${cue.label} to Timeline`}
                      data-timeline-cue-drag-source={cue.id}
                      onPointerDown={(event) => beginTimelineCueDrag(event, cue)}
                      onPointerMove={moveTimelineCueDrag}
                      onPointerUp={(event) => finishTimelineCueDrag(event, false)}
                      onPointerCancel={(event) => finishTimelineCueDrag(event, true)}
                    >
                      <span aria-hidden="true" data-no-localize>⠿</span>
                    </button>
                    </Show>
                  </div>
                  <span>
                    {`${cue.targets.length} fixture(s) / ${(cue.steps ?? []).length} step(s) / ${cue.video_targets.length} video / ${cue.video_output_targets.length} video output(s) / ${(cue.effect_targets ?? []).length} effect(s) / ${displayNumber(cue.fade_ms, 0)}ms / ${cue.tracking ? "Track" : "Block"}${cue.mark ? " / MIB" : ""}`}
                  </span>
                </div>
                <div class="cueEditRow">
                  <label>
                    Scene #
                    <input value={draft().cue_number} onInput={(event) => props.onUpdateCueMetadataDraft(cue, { cue_number: event.currentTarget.value })} />
                  </label>
                  <label>
                    Bank
                    <select value={cue.cue_list_id} onInput={(event) => void props.onSetCueList(cue.id, Number(event.currentTarget.value))}>
                    <For each={stableCueLists}>{(cueList) => <option data-no-localize value={cueList.id}>{cueList.label}</option>}</For>
                    </select>
                  </label>
                  <label>
                    Label
                  <input
                    value={draft().label}
                    data-cue-metadata-label={cue.id}
                    onInput={(event) => props.onUpdateCueMetadataDraft(cue, { label: event.currentTarget.value })}
                  />
                  </label>
                  <label>
                    Group
                    <select
                      value={draft().group_id ?? ""}
                      onInput={(event) => props.onUpdateCueMetadataDraft(cue, {
                        group_id: event.currentTarget.value || null,
                      })}
                    >
                      <option value="">Show (ungrouped)</option>
                      <Show when={draft().group_id && !props.groupIds.includes(draft().group_id!)}>
                        <option data-no-localize value={draft().group_id!}>{draft().group_id}</option>
                      </Show>
                      <For each={props.groupIds}>
                        {(groupId) => <option data-no-localize value={groupId}>{groupId}</option>}
                      </For>
                    </select>
                  </label>
                  <label class="cueColorField">
                    Color
                    <span class="cueColorControls">
                      <input
                        type="color"
                        value={cue.color ?? "#5f6b76"}
                        data-cue-color-input={cue.id}
                        aria-label={`Identity color for ${cue.label}`}
                        onChange={(event) => void props.onSetCueColor(cue.id, event.currentTarget.value)}
                      />
                      <button
                        type="button"
                        disabled={!cue.color}
                        data-cue-color-clear={cue.id}
                        title="Clear Color"
                        aria-label={`Clear identity color for ${cue.label}`}
                        onClick={() => void props.onSetCueColor(cue.id, null)}
                      >
                        ×
                      </button>
                    </span>
                  </label>
                  <label class="cueLiveDefaultsField">
                    Live modifier defaults
                    <span class="cueLiveDefaultsControls" data-cue-live-defaults={cue.id}>
                      <input
                        type="number"
                        min="0.05"
                        max="20"
                        step="0.05"
                        value={authoredCueLiveModifier(cue).speed}
                        data-cue-live-defaults-speed={cue.id}
                        aria-label={`Default live speed for ${cue.label}`}
                        onChange={(event) =>
                          void props.onSetCueLiveModifierDefaults(cue.id, {
                            ...authoredCueLiveModifier(cue),
                            speed: Number(event.currentTarget.value),
                          })}
                      />
                      <input
                        type="number"
                        min="0"
                        max="2"
                        step="0.05"
                        value={authoredCueLiveModifier(cue).size}
                        data-cue-live-defaults-size={cue.id}
                        aria-label={`Default live size for ${cue.label}`}
                        onChange={(event) =>
                          void props.onSetCueLiveModifierDefaults(cue.id, {
                            ...authoredCueLiveModifier(cue),
                            size: Number(event.currentTarget.value),
                          })}
                      />
                      <input
                        type="number"
                        min="0"
                        max="1"
                        step="0.01"
                        value={authoredCueLiveModifier(cue).phase}
                        data-cue-live-defaults-phase={cue.id}
                        aria-label={`Default live phase for ${cue.label}`}
                        onChange={(event) =>
                          void props.onSetCueLiveModifierDefaults(cue.id, {
                            ...authoredCueLiveModifier(cue),
                            phase: Number(event.currentTarget.value),
                          })}
                      />
                      <select
                        value={authoredCueLiveModifier(cue).direction ?? "Authored"}
                        data-cue-live-defaults-direction={cue.id}
                        aria-label={`Default live direction for ${cue.label}`}
                        onInput={(event) =>
                          void props.onSetCueLiveModifierDefaults(cue.id, {
                            ...authoredCueLiveModifier(cue),
                            direction: event.currentTarget.value as CueLiveDirection,
                          })}
                      >
                        <option value="Authored">Authored</option>
                        <option value="Forward">Forward</option>
                        <option value="Reverse">Reverse</option>
                        <option value="Bounce">Bounce</option>
                      </select>
                      <select
                        value={authoredCueLiveModifier(cue).segment ?? 0}
                        disabled={(cue.steps?.length ?? 0) === 0}
                        data-cue-live-defaults-segment={cue.id}
                        aria-label={`Default live segment for ${cue.label}`}
                        onInput={(event) =>
                          void props.onSetCueLiveModifierDefaults(cue.id, {
                            ...authoredCueLiveModifier(cue),
                            segment: Number(event.currentTarget.value),
                          })}
                      >
                        <option value="0">Auto segment</option>
                        <For each={cue.steps ?? []}>
                          {(_, index) => <option value={index() + 1}>Segment {index() + 1}</option>}
                        </For>
                      </select>
                      <label class="checkbox compactCheckbox">
                        <input
                          type="checkbox"
                          checked={authoredCueLiveModifier(cue).flash}
                          data-cue-live-defaults-flash={cue.id}
                          aria-label={`Flash mode for ${cue.label}`}
                          onChange={(event) =>
                            void props.onSetCueLiveModifierDefaults(cue.id, {
                              ...authoredCueLiveModifier(cue),
                              flash: event.currentTarget.checked,
                            })}
                        />
                        <span>FLASH</span>
                      </label>
                      <button
                        type="button"
                        disabled={!cue.live_modifiers}
                        data-cue-live-defaults-clear={cue.id}
                        title="Clear live modifier defaults"
                        aria-label={`Clear live modifier defaults for ${cue.label}`}
                        onClick={() => void props.onSetCueLiveModifierDefaults(cue.id, null)}
                      >
                        ×
                      </button>
                    </span>
                  </label>
                  <label>
                    Recall mode
                    <select
                      value={draft().recall_mode}
                      onInput={(event) => props.onUpdateCueMetadataDraft(cue, {
                        recall_mode: event.currentTarget.value as CueMetadataDraft["recall_mode"],
                      })}
                    >
                      <option value="Coexist">Coexist</option>
                      <option value="ReplaceGroup">Replace group</option>
                    </select>
                  </label>
                  <label>
                    Fade ms
                  <input
                    class="tabularNums"
                    type="number"
                    min="0"
                    value={displayNumber(draft().fade_ms, 0)}
                    onInput={(event) => props.onUpdateCueMetadataDraft(cue, { fade_ms: Number(event.currentTarget.value) })}
                    onBlur={(event) => {
                      event.currentTarget.value = displayNumber(draft().fade_ms, 0);
                    }}
                  />
                  </label>
                  <label>
                    Authored beats
                    <input
                      class="tabularNums"
                      type="number"
                      min="0.25"
                      max="1024"
                      step="0.25"
                      placeholder="Optional"
                      data-cue-authored-beats
                      value={displayNumber(draft().authored_beats, 3)}
                      aria-invalid={authoredBeatsInvalid()}
                      onInput={(event) => props.onUpdateCueMetadataDraft(cue, {
                        authored_beats: event.currentTarget.value === "" ? null : Number(event.currentTarget.value),
                      })}
                      onBlur={(event) => {
                        event.currentTarget.value = displayNumber(draft().authored_beats, 3);
                      }}
                    />
                    <Show when={authoredBeatsInvalid()}>
                      <small class="cueFieldError">Use 0.25 to 1024.</small>
                    </Show>
                  </label>
                  <label>
                    Pre-wait ms
                    <input
                      class="tabularNums"
                      type="number"
                      min="0"
                      value={displayNumber(draft().pre_wait_ms, 0)}
                      onInput={(event) => props.onUpdateCueMetadataDraft(cue, { pre_wait_ms: Number(event.currentTarget.value) })}
                      onBlur={(event) => {
                        event.currentTarget.value = displayNumber(draft().pre_wait_ms, 0);
                      }}
                    />
                  </label>
                  <label>
                    Follow ms
                    <input
                      class="tabularNums"
                      type="number"
                      min="0"
                      placeholder="Manual"
                      value={displayNumber(draft().follow_ms, 0)}
                      onInput={(event) => props.onUpdateCueMetadataDraft(cue, {
                        follow_ms: event.currentTarget.value === "" ? null : Number(event.currentTarget.value),
                      })}
                      onBlur={(event) => {
                        event.currentTarget.value = displayNumber(draft().follow_ms, 0);
                      }}
                    />
                  </label>
                  <label class="cueTrackingToggle">
                    <input
                      type="checkbox"
                      checked={draft().tracking}
                      onChange={(event) => props.onUpdateCueMetadataDraft(cue, { tracking: event.currentTarget.checked })}
                    />
                    Tracking
                  </label>
                  <label class="cueTrackingToggle">
                    <input
                      type="checkbox"
                      checked={draft().mark}
                      onChange={(event) => props.onUpdateCueMetadataDraft(cue, { mark: event.currentTarget.checked })}
                    />
                    Mark/MIB
                  </label>
                  <label class="cueNotesField">
                    Notes
                    <textarea
                      rows="2"
                      maxlength="500"
                      value={draft().notes}
                      onInput={(event) => props.onUpdateCueMetadataDraft(cue, { notes: event.currentTarget.value })}
                    />
                  </label>
                </div>
                <CueStepEditor
                  cue={cue}
                  bpm={props.bpm}
                  onSave={props.onSetCueSteps}
                  onUseAuthoredBeats={(authored_beats) =>
                    props.onUpdateCueMetadataDraft(cue, { authored_beats })}
                />
                <details class="cueMibEditor cueEditOnly">
                  <summary>MIB targets ({draft().mib_fixture_ids.length === 0 ? "Auto" : draft().mib_fixture_ids.length})</summary>
                  <p>Auto considers every fixture targeted by this Cue. Selecting fixtures limits MIB to that set; dark-only safety remains enforced.</p>
                  <div class="buttonRow">
                    <button disabled={draft().mib_fixture_ids.length === 0} onClick={() => props.onUpdateCueMetadataDraft(cue, { mib_fixture_ids: [] })}>Use Auto</button>
                  </div>
                  <div class="cueMibTargets">
                    <For each={mibTargetIds()}>
                      {(fixtureId) => (
                        <label class="checkbox">
                          <input
                            type="checkbox"
                            checked={draft().mib_fixture_ids.includes(fixtureId)}
                            onChange={(event) => toggleMibFixture(fixtureId, event.currentTarget.checked)}
                          />
                          Fixture {fixtureId}
                        </label>
                      )}
                    </For>
                  </div>
                </details>
                <details class="cueIfcbTiming">
                  <summary>IFCB timing overrides</summary>
                  <p>Blank Fade inherits the cue Fade. Delay holds the current value before that family starts.</p>
                  <div class="cueIfcbTimingGrid tabularNums">
                    {([
                      ["Intensity", "intensity_fade_ms", "intensity_delay_ms"],
                      ["Focus", "focus_fade_ms", "focus_delay_ms"],
                      ["Color", "color_fade_ms", "color_delay_ms"],
                      ["Beam", "beam_fade_ms", "beam_delay_ms"],
                    ] as const).map(([label, fadeKey, delayKey]) => (
                      <fieldset>
                        <legend>{label}</legend>
                        <label>
                          Fade ms
                          <input
                            type="number"
                            min="0"
                            placeholder="Inherit"
                            value={displayNumber(draft().ifcb_timing[fadeKey], 0)}
                            onInput={(event) => updateIfcbTiming({
                              [fadeKey]: event.currentTarget.value === "" ? null : Number(event.currentTarget.value),
                            })}
                            onBlur={(event) => {
                              event.currentTarget.value = displayNumber(draft().ifcb_timing[fadeKey], 0);
                            }}
                          />
                        </label>
                        <label>
                          Delay ms
                          <input
                            type="number"
                            min="0"
                            value={displayNumber(draft().ifcb_timing[delayKey], 0)}
                            onInput={(event) => updateIfcbTiming({ [delayKey]: Number(event.currentTarget.value) })}
                            onBlur={(event) => {
                              event.currentTarget.value = displayNumber(draft().ifcb_timing[delayKey], 0);
                            }}
                          />
                        </label>
                      </fieldset>
                    ))}
                  </div>
                </details>
                <details class="cuePartsEditor cueEditOnly">
                  <summary>Cue Parts ({draft().parts.length})</summary>
                  <p>Assign each fixture, video layer, or video output to one part. Lighting adds IFCB Delay; video holds until Part Delay. Blank Part Fade inherits the Cue Fade.</p>
                  <div class="buttonRow">
                    <button disabled={draft().parts.length >= 16 || (cue.targets.length === 0 && cue.video_targets.length === 0 && cue.video_output_targets.length === 0)} onClick={addPart}>Add Cue Part</button>
                  </div>
                  <For each={draft().parts}>
                    {(part, partIndex) => (
                      <fieldset class="cuePartRow">
                        <legend>Part {part.number}</legend>
                        <div class="cuePartFields tabularNums">
                          <label>Number<input type="number" min="1" max="999" value={part.number} onInput={(event) => updatePart(partIndex(), { number: Number(event.currentTarget.value) })} /></label>
                          <label>Label<input maxlength="64" value={part.label} onInput={(event) => updatePart(partIndex(), { label: event.currentTarget.value })} /></label>
                          <label>
                            Delay ms
                            <input
                              type="number"
                              min="0"
                              value={displayNumber(part.delay_ms, 0)}
                              onInput={(event) => updatePart(partIndex(), { delay_ms: Number(event.currentTarget.value) })}
                              onBlur={(event) => {
                                event.currentTarget.value = displayNumber(part.delay_ms, 0);
                              }}
                            />
                          </label>
                          <label>
                            Fade ms
                            <input
                              type="number"
                              min="0"
                              placeholder="Inherit"
                              value={displayNumber(part.fade_ms, 0)}
                              onInput={(event) => updatePart(partIndex(), {
                                fade_ms: event.currentTarget.value === "" ? null : Number(event.currentTarget.value),
                              })}
                              onBlur={(event) => {
                                event.currentTarget.value = displayNumber(part.fade_ms, 0);
                              }}
                            />
                          </label>
                          <button class="danger" onClick={() => props.onUpdateCueMetadataDraft(cue, { parts: draft().parts.filter((_, index) => index !== partIndex()) })}>Remove Part</button>
                        </div>
                        <div class="cuePartTargets" aria-label={`Fixture assignments for Cue Part ${part.number}`}>
                          <For each={cue.targets}>
                            {(target) => (
                              <label class="checkbox">
                                <input
                                  type="checkbox"
                                  checked={part.fixture_ids.includes(target.fixture_id)}
                                  onChange={(event) => togglePartFixture(partIndex(), target.fixture_id, event.currentTarget.checked)}
                                />
                                Fixture {target.fixture_id}
                              </label>
                            )}
                          </For>
                          <For each={cue.video_targets}>
                            {(target) => (
                              <label class="checkbox">
                                <input
                                  type="checkbox"
                                  checked={part.video_layer_ids.includes(target.layer_id)}
                                  onChange={(event) => togglePartVideoLayer(partIndex(), target.layer_id, event.currentTarget.checked)}
                                />
                                Video L{target.layer_id}
                              </label>
                            )}
                          </For>
                          <For each={cue.video_output_targets}>
                            {(target) => (
                              <label class="checkbox">
                                <input
                                  type="checkbox"
                                  checked={part.video_output_ids.includes(target.output_id)}
                                  onChange={(event) => togglePartVideoOutput(partIndex(), target.output_id, event.currentTarget.checked)}
                                />
                                Output {target.output_id}
                              </label>
                            )}
                          </For>
                        </div>
                      </fieldset>
                    )}
                  </For>
                </details>
                <details class="cuePaletteEditor cueEditOnly">
                  <summary>Reference Palettes ({cue.palette_targets.length})</summary>
                  <p>Checked palettes resolve their latest values on GO. Explicit values stored in this Cue take priority.</p>
                  <Show when={props.palettes.length > 0} fallback={<p class="empty">Capture a palette in the Programmer area first.</p>}>
                    <div class="cuePaletteTargets">
                      <For each={props.palettes}>
                        {(palette) => (
                          <label class="checkbox">
                            <input
                              type="checkbox"
                              disabled={cue.targets.length === 0}
                              checked={cue.palette_targets.some((target) => target.palette_id === palette.id)}
                              onChange={(event) => void props.onSetCuePalette(cue, palette.id, event.currentTarget.checked)}
                            />
                            <span data-no-localize>{palette.label}</span> <span>{palette.kind}</span>
                          </label>
                        )}
                      </For>
                    </div>
                  </Show>
                </details>
                <CueEffectRecallEditor
                  effects={props.effects}
                  targets={draft().effect_targets}
                  currentMode="refresh"
                  applyHint="Save Recall stores this Effect Recall list. An empty list clears all saved Effect Recall targets."
                  onChange={(effect_targets) => props.onUpdateCueMetadataDraft(cue, { effect_targets })}
                />
                <p class="cueLookUpdateHint cueEditOnly">
                  <span>Update Look recaptures the current Store Scope:</span>{" "}
                  <strong>{cueCaptureScopeLabel(props.cueCaptureScope)}</strong>.{" "}
                  <span>It does not save Recall edits.</span>
                </p>
                <Show when={draft().effect_targets.length === 0}>
                  <p class="cueLookUpdateHint cueEditOnly">
                    No Effect Recall targets are selected. Save Recall will clear all saved Effect Recall targets.
                  </p>
                </Show>
                <div class="cueActionRow">
                  <Show when={props.mode !== "scene-settings"}>
                  <button class="cueEditOnly" onClick={() => void props.onMoveCue(cue.id, -1)} disabled={cueIndex() === 0}>
                    Up
                  </button>
                  <button class="cueEditOnly" onClick={() => void props.onMoveCue(cue.id, 1)} disabled={cueIndex() === stableCues.length - 1}>
                    Down
                  </button>
                  </Show>
                  <button
                    class="cueEditOnly cueSaveDetails"
                    title="Saves Cue details only. Effect Recall uses Save Recall."
                    disabled={authoredBeatsInvalid()}
                    onClick={() => void props.onSetCueMetadata(cue)}
                  >
                    Save Details
                  </button>
                  <button
                    class="cueEditOnly cueSaveRecall"
                    onClick={() => void props.onSetCueEffectTargets(cue.id, draft().effect_targets)}
                  >
                    Save Recall
                  </button>
                  <button class="cueEditOnly" onClick={() => void props.onDuplicateCue(cue)}>
                    Copy
                  </button>
                  <button
                    class="cueEditOnly cueUpdateLook"
                    title="Recaptures the current Store Scope shown above. Recall edits use Save Recall."
                    onClick={() => void props.onUpdateCue(cue.id, draft().label, draft().fade_ms)}
                    disabled={
                      (!props.hasCueSources && props.cueEffectCaptureTargets.length === 0)
                      || Boolean(props.cueCaptureScopeError)
                    }
                  >
                    Update Look
                  </button>
                  <Show when={props.mode !== "scene-settings"}>
                  <button class="cueLiveGo" onClick={() => void props.onTriggerCue(cue.id)}>GO</button>
                  <button
                    class="cueEditOnly"
                    title="Place this Cue as a linked Scene Block at the current playhead."
                    onClick={() => void props.onAddTimelineCueEventAt(cue.id, props.timelinePositionMs, props.timelineTrack, false)}
                  >
                    Block @ Playhead
                  </button>
                  </Show>
                  <button class="cueEditOnly" onClick={() => void props.onRemoveCue(cue.id)}>Remove</button>
                </div>
                <Show when={props.mode !== "scene-settings"}>
                <Show when={placements().length > 0}>
                  <div class="cueTimelinePlacements">
                    <For each={visiblePlacements()}>
                      {(placement) => (
                        <span
                          class={[
                            "cueTimelinePlacementChip",
                            placement.track === "Lighting" ? "lighting" : "video",
                            placement.duration_ms > 0 ? "linkedBlock" : "legacyPoint",
                            placement.time_ms < props.timelinePositionMs ? "past" : "",
                          ].filter(Boolean).join(" ")}
                        >
                          <button
                            class="cueTimelinePlacementMain"
                            title={placement.duration_ms > 0
                              ? placement.conform_to_tempo
                                ? `Linked Scene Block${timelineConformRateBadge(placement) ? ` ${timelineConformRateBadge(placement)}` : ""} · ${displayNumber(placement.time_ms, 0)} ms / ${displayNumber(placement.duration_ms, 0)} ms window / ${placement.loop_count} ${placement.loop_fill ? "fill loops" : "tempo iterations"} / ${placement.track}`
                                : `Linked Scene Block · ${displayNumber(placement.time_ms, 0)} ms / ${displayNumber(placement.duration_ms, 0)} ms × ${placement.loop_count} / ${placement.track}`
                              : `Legacy point · ${displayNumber(placement.time_ms, 0)} ms / ${placement.track}`}
                            onClick={() => void props.onSeekTimeline(placement.time_ms)}
                          >
                            <b>{placement.track === "Lighting" ? "L" : "V"}</b>
                            <small>
                              {displayNumber(placement.time_ms, 0)} · {placement.duration_ms > 0
                                ? placement.conform_to_tempo
                                  ? `${timelineConformRateBadge(placement) ? `${timelineConformRateBadge(placement)} ` : ""}${displayNumber(placement.duration_ms, 0)} window/${placement.loop_count} ${placement.loop_fill ? "fill" : "tempo"}`
                                  : `${displayNumber(placement.duration_ms, 0)}×${placement.loop_count}`
                                : "Point"}
                            </small>
                          </button>
                          <button
                            class="cueTimelinePlacementMove"
                            title={`Nudge ${displayNumber(props.timelinePlacementNudgeMs, 0)} ms. Shift-click nudges left.`}
                            onClick={(event) =>
                              void props.onMoveTimelineCueEvent(
                                placement,
                                event.shiftKey ? -props.timelinePlacementNudgeMs : props.timelinePlacementNudgeMs,
                              )
                            }
                          >
                            &gt;
                          </button>
                          <button
                            class="cueTimelinePlacementRemove"
                            title="Remove timeline placement"
                            onClick={() => void props.onRemoveTimelineEvent(placement.id)}
                          >
                            x
                          </button>
                        </span>
                      )}
                    </For>
                    <Show when={hiddenPlacementCount() > 0}>
                      <button
                        type="button"
                        class="cueTimelinePlacementMore"
                        title="Open the Timeline to inspect every linked placement."
                        onClick={props.onOpenTimeline}
                      >
                        {hiddenPlacementCount()} more · Open Timeline
                      </button>
                    </Show>
                  </div>
                </Show>
                </Show>
              </div>
            );
          }}
        </For>
      </div>
      </Show>
    </div>
  );
}
