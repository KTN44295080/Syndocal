import { createEffect, createMemo, createSignal, For, Show } from "solid-js";
import type { CueMetadataDraft } from "../editorDrafts";
import type {
  ActiveFadeSummary,
  CueEffectTarget,
  CueListSummary,
  CueSummary,
  EffectSummary,
  ReferencePaletteSummary,
  TimelineCueEventSummary,
  TimelineTrackKind,
} from "../types";
import { canSaveCueEffectTargets } from "../cueEffectRecall";
import type { CueEffectRecallChange } from "../cueEffectRecall";
import { timelineConformRateBadge } from "../timelineSceneBlocks";
import { cueIdentityHue, identityCssColor } from "../identityColor";
import type { TimelineCueDragPoint } from "../timelineCueDrag";
import { CueCapturePreviewPanel, type CueCapturePreviewModel } from "./CueCapturePreviewPanel";
import { CueEffectRecallEditor } from "./CueEffectRecallEditor";

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
  mode: "edit" | "live";
  cues: CueSummary[];
  allCues: CueSummary[];
  cueLists: CueListSummary[];
  groupIds: string[];
  palettes: ReferencePaletteSummary[];
  effects: EffectSummary[];
  cueCaptureEffects: EffectSummary[];
  selectedCueListId: number;
  cueListLabel: string;
  activeCueId: number | null | undefined;
  revealCueId: number | null;
  revealCueRevision: number;
  activeFade: ActiveFadeSummary | null | undefined;
  timelinePositionMs: number;
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
  onCreateCueList: () => void | Promise<void>;
  onRenameCueList: () => void | Promise<void>;
  onRemoveCueList: () => void | Promise<void>;
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
  onDuplicateCue: (cue: CueSummary) => void | Promise<void>;
  onUpdateCue: (cueId: number, label: string, fadeMs: number) => void | Promise<void>;
  onTriggerCue: (cueId: number) => void | Promise<void>;
  onAddTimelineCueEventAt: (
    cueId: number | null,
    timeMs: number,
    track: TimelineTrackKind,
    nextDraftTime?: boolean,
  ) => void | Promise<void>;
  onRemoveCue: (cueId: number) => void | Promise<void>;
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
  const [cueEditing, setCueEditing] = createSignal(false);
  const [timelineDragCueId, setTimelineDragCueId] = createSignal<number | null>(null);
  let timelineDragPointer: {
    cueId: number;
    pointerId: number;
    startClientX: number;
    startClientY: number;
    moved: boolean;
  } | null = null;
  let cuePanelElement: HTMLDivElement | undefined;
  const cuePageCount = createMemo(() => Math.max(1, Math.ceil(props.cues.length / cuesPerPage)));
  const visibleCues = createMemo(() => {
    const start = cuePage() * cuesPerPage;
    return props.cues.slice(start, start + cuesPerPage);
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
    const cues = props.cues;
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
    const activeIndex = props.cues.findIndex((cue) => cue.id === activeCueId);
    if (activeIndex >= 0) setCuePage(Math.floor(activeIndex / cuesPerPage));
  });

  return (
    <div ref={(element) => { cuePanelElement = element; }} class={
      props.mode === "live"
        ? `cuePanel cuePanelLive ${cueEditing() ? "cuePanelEditing" : ""}`
        : "cuePanel"
    }>
      <div class="panelHeader">
        <h2>Cues</h2>
        <div class="cuePanelHeaderActions">
          <span>{props.cues.length}</span>
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
      <div id="cue-list-editor" class="cueListManager cueEditOnly" aria-label="Cue List manager">
        <label>
          Cue List
          <select value={props.selectedCueListId} onInput={(event) => props.onSelectCueList(Number(event.currentTarget.value))}>
            <For each={props.cueLists}>{(cueList) => <option data-no-localize value={cueList.id}>{cueList.label}</option>}</For>
          </select>
        </label>
        <label>
          List label
          <input maxlength="64" value={props.cueListLabel} onInput={(event) => props.onCueListLabel(event.currentTarget.value)} />
        </label>
        <button onClick={() => void props.onCreateCueList()}>New List</button>
        <button onClick={() => void props.onRenameCueList()}>Rename</button>
        <button class="danger" disabled={props.selectedCueListId === 1} onClick={() => void props.onRemoveCueList()}>Remove List</button>
      </div>
      <div class="cueExecutorBank" aria-label="Cue List executors">
        <For each={props.cueLists}>
          {(cueList) => {
            const activeCue = () => props.allCues.find((cue) => cue.id === cueList.active_cue_id)
              ?? (cueList.id === props.selectedCueListId ? props.allCues.find((cue) => cue.id === props.activeCueId) : undefined);
            const cueCount = () => props.allCues.filter((cue) => cue.cue_list_id === cueList.id).length;
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
      <div id="cue-store-form" class="cueForm">
        <label>
          Label
          <input value={props.cueLabel} onInput={(event) => props.onCueLabel(event.currentTarget.value)} />
        </label>
        <label>
          Fade ms
          <input type="number" min="0" value={props.cueFadeMs} onInput={(event) => props.onCueFadeMs(Number(event.currentTarget.value))} />
        </label>
        <label>
          Authored beats
          <input
            type="number"
            min="0.25"
            max="1024"
            step="0.25"
            placeholder="Optional"
            value={props.cueAuthoredBeats ?? ""}
            aria-invalid={Boolean(props.cueAuthoredBeatsError)}
            aria-describedby={props.cueAuthoredBeatsError ? "cue-authored-beats-error" : undefined}
            title={props.cueAuthoredBeatsSeeded ? "Seeded from enabled Effect timing." : "Scene length in beats."}
            onInput={(event) => props.onCueAuthoredBeats(
              event.currentTarget.value === "" ? null : Number(event.currentTarget.value),
            )}
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
        <button onClick={() => void props.onTriggerPreviousCue()} disabled={props.cues.length === 0}>
          Back
        </button>
        <button class="primary" onClick={() => void props.onTriggerNextCue()} disabled={props.cues.length === 0}>
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
      <Show when={cuePageCount() > 1}>
        <nav class="cuePager" aria-label="Cue Page">
          <button aria-label="Previous cue page" onClick={() => setCuePage(Math.max(0, cuePage() - 1))} disabled={cuePage() === 0}>Prev</button>
          <span class="tabularNums">Page {cuePage() + 1} / {cuePageCount()}</span>
          <button aria-label="Next cue page" onClick={() => setCuePage(Math.min(cuePageCount() - 1, cuePage() + 1))} disabled={cuePage() >= cuePageCount() - 1}>Next</button>
        </nav>
      </Show>
      <div id="cue-list-items" class="cueList" role="list">
        <Show when={props.cues.length === 0}>
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
            const canSaveRecall = () => canSaveCueEffectTargets(cue, draft().effect_targets);
            const recallSaveHintId = `cue-${cue.id}-recall-save-hint`;
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
                style={{ "--identity": identityCssColor(cueIdentityHue(cue.id), "fill") }}
                data-cue-id={cue.id}
                role="listitem"
                aria-posinset={cueIndex() + 1}
                aria-setsize={props.cues.length}
              >
                <div class="cueMetaLine">
                  <div class="cueTitleRow">
                    <strong data-no-localize><b>{cue.cue_number || cue.id}</b> {cue.label}</strong>
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
                  </div>
                  <span>
                    {cue.targets.length} fixture(s) / {cue.video_targets.length} video /{" "}
                    {cue.video_output_targets.length} video output(s) / {(cue.effect_targets ?? []).length} effect(s) /{" "}
                    {cue.fade_ms}ms / {cue.tracking ? "Track" : "Block"}{cue.mark ? " / MIB" : ""}
                  </span>
                </div>
                <div class="cueEditRow">
                  <label>
                    Cue #
                    <input value={draft().cue_number} onInput={(event) => props.onUpdateCueMetadataDraft(cue, { cue_number: event.currentTarget.value })} />
                  </label>
                  <label>
                    List
                    <select value={cue.cue_list_id} onInput={(event) => void props.onSetCueList(cue.id, Number(event.currentTarget.value))}>
                      <For each={props.cueLists}>{(cueList) => <option data-no-localize value={cueList.id}>{cueList.label}</option>}</For>
                    </select>
                  </label>
                  <label>
                    Label
                  <input value={draft().label} onInput={(event) => props.onUpdateCueMetadataDraft(cue, { label: event.currentTarget.value })} />
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
                    type="number"
                    min="0"
                    value={draft().fade_ms}
                    onInput={(event) => props.onUpdateCueMetadataDraft(cue, { fade_ms: Number(event.currentTarget.value) })}
                  />
                  </label>
                  <label>
                    Authored beats
                    <input
                      type="number"
                      min="0.25"
                      max="1024"
                      step="0.25"
                      placeholder="Optional"
                      value={draft().authored_beats ?? ""}
                      aria-invalid={authoredBeatsInvalid()}
                      onInput={(event) => props.onUpdateCueMetadataDraft(cue, {
                        authored_beats: event.currentTarget.value === "" ? null : Number(event.currentTarget.value),
                      })}
                    />
                    <Show when={authoredBeatsInvalid()}>
                      <small class="cueFieldError">Use 0.25 to 1024.</small>
                    </Show>
                  </label>
                  <label>
                    Pre-wait ms
                    <input
                      type="number"
                      min="0"
                      value={draft().pre_wait_ms}
                      onInput={(event) => props.onUpdateCueMetadataDraft(cue, { pre_wait_ms: Number(event.currentTarget.value) })}
                    />
                  </label>
                  <label>
                    Follow ms
                    <input
                      type="number"
                      min="0"
                      placeholder="Manual"
                      value={draft().follow_ms ?? ""}
                      onInput={(event) => props.onUpdateCueMetadataDraft(cue, {
                        follow_ms: event.currentTarget.value === "" ? null : Number(event.currentTarget.value),
                      })}
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
                            value={draft().ifcb_timing[fadeKey] ?? ""}
                            onInput={(event) => updateIfcbTiming({
                              [fadeKey]: event.currentTarget.value === "" ? null : Number(event.currentTarget.value),
                            })}
                          />
                        </label>
                        <label>
                          Delay ms
                          <input
                            type="number"
                            min="0"
                            value={draft().ifcb_timing[delayKey]}
                            onInput={(event) => updateIfcbTiming({ [delayKey]: Number(event.currentTarget.value) })}
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
                          <label>Delay ms<input type="number" min="0" value={part.delay_ms} onInput={(event) => updatePart(partIndex(), { delay_ms: Number(event.currentTarget.value) })} /></label>
                          <label>Fade ms<input type="number" min="0" placeholder="Inherit" value={part.fade_ms ?? ""} onInput={(event) => updatePart(partIndex(), { fade_ms: event.currentTarget.value === "" ? null : Number(event.currentTarget.value) })} /></label>
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
                  applyHint="Save Recall stores only this Effect Recall list, including an empty list when the Cue still has another target."
                  onChange={(effect_targets) => props.onUpdateCueMetadataDraft(cue, { effect_targets })}
                />
                <p class="cueLookUpdateHint cueEditOnly">
                  <span>Update Look recaptures the current Store Scope:</span>{" "}
                  <strong>{cueCaptureScopeLabel(props.cueCaptureScope)}</strong>.{" "}
                  <span>It does not save Recall edits.</span>
                </p>
                <Show when={!canSaveRecall()}>
                  <p id={recallSaveHintId} class="cueRecallSaveHint cueEditOnly">
                    A Cue needs at least one target. Remove this Cue instead of saving an empty Effect-only Recall.
                  </p>
                </Show>
                <div class="cueActionRow">
                  <button class="cueEditOnly" onClick={() => void props.onMoveCue(cue.id, -1)} disabled={cueIndex() === 0}>
                    Up
                  </button>
                  <button class="cueEditOnly" onClick={() => void props.onMoveCue(cue.id, 1)} disabled={cueIndex() === props.cues.length - 1}>
                    Down
                  </button>
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
                    aria-describedby={!canSaveRecall() ? recallSaveHintId : undefined}
                    disabled={!canSaveRecall()}
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
                  <button class="cueLiveGo" onClick={() => void props.onTriggerCue(cue.id)}>GO</button>
                  <button
                    class="cueEditOnly"
                    title="Place this Cue as a linked Scene Block at the current playhead."
                    onClick={() => void props.onAddTimelineCueEventAt(cue.id, props.timelinePositionMs, props.timelineTrack, false)}
                  >
                    Block @ Playhead
                  </button>
                  <button class="cueEditOnly" onClick={() => void props.onRemoveCue(cue.id)}>Remove</button>
                </div>
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
                                ? `Linked Scene Block${timelineConformRateBadge(placement) ? ` ${timelineConformRateBadge(placement)}` : ""} · ${placement.time_ms} ms / ${placement.duration_ms} ms window / ${placement.loop_count} ${placement.loop_fill ? "fill loops" : "tempo iterations"} / ${placement.track}`
                                : `Linked Scene Block · ${placement.time_ms} ms / ${placement.duration_ms} ms × ${placement.loop_count} / ${placement.track}`
                              : `Legacy point · ${placement.time_ms} ms / ${placement.track}`}
                            onClick={() => void props.onSeekTimeline(placement.time_ms)}
                          >
                            <b>{placement.track === "Lighting" ? "L" : "V"}</b>
                            <small>
                              {placement.time_ms} · {placement.duration_ms > 0
                                ? placement.conform_to_tempo
                                  ? `${timelineConformRateBadge(placement) ? `${timelineConformRateBadge(placement)} ` : ""}${placement.duration_ms} window/${placement.loop_count} ${placement.loop_fill ? "fill" : "tempo"}`
                                  : `${placement.duration_ms}×${placement.loop_count}`
                                : "Point"}
                            </small>
                          </button>
                          <button
                            class="cueTimelinePlacementMove"
                            title={`Nudge ${props.timelinePlacementNudgeMs} ms. Shift-click nudges left.`}
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
              </div>
            );
          }}
        </For>
      </div>
    </div>
  );
}
