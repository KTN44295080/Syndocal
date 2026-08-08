import { createEffect, createMemo, createSignal, For, onCleanup, onMount, Show, type JSX } from "solid-js";
import { createStore, reconcile } from "solid-js/store";
import { cueIdentityCss, groupIdentityCss, groupIdentityHue } from "../identityColor";
import { handleHorizontalWheel } from "../horizontalWheel";
import { displayNumber } from "../numberDisplay";
import { sceneCueKind } from "../sceneCueKind";
import type {
  ActiveFadeSummary,
  CueLiveDirection,
  CueLiveModifierState,
  CueSummary,
  TimelineTrackKind,
} from "../types";
import type { TimelineCueDragPoint } from "../timelineCueDrag";
import { authoredCueLiveModifier } from "../cueLiveModifier";
import { CueLiveModifierStrip } from "./CueLiveModifierStrip";
import { controlMappingTargetData } from "../controlMappingLearn";

interface SceneMatrixPanelProps {
  toolbar?: JSX.Element;
  cues: CueSummary[];
  groupColors?: Record<string, string>;
  onSetGroupColor?: (groupId: string, color: string | null) => void | Promise<void>;
  groupIds: string[];
  activeCueId: number | null | undefined;
  activeGroupCueIds: Record<string, number>;
  selectedCueId?: number | null;
  activeFade?: ActiveFadeSummary | null;
  cueLiveModifiers?: CueLiveModifierState[];
  onSetCueLiveModifier?: (
    cueId: number,
    speed: number,
    size: number,
    phase: number,
    direction: CueLiveDirection,
    segment: number,
  ) => void | Promise<void>;
  onClearCueLiveModifier?: (cueId: number) => void | Promise<void>;
  onReleaseCue: (cueId: number) => void | Promise<void>;
  onTriggerCue: (cueId: number) => void | Promise<void>;
  onSelectCue: (cueId: number) => void;
  onOpenSuperScene: (cueId: number) => void | Promise<void>;
  onOpenCueEditor: () => void;
  onBeginTimelineCueDrag: (
    cue: CueSummary,
    point: TimelineCueDragPoint,
    sourceSurface?: "scene-matrix",
  ) => void;
  onMoveTimelineCueDrag: (point: TimelineCueDragPoint) => void;
  onEndTimelineCueDrag: (point: TimelineCueDragPoint, moved: boolean, canceled: boolean) => void;
  timelineTrack: TimelineTrackKind;
}

interface SceneMatrixColumn {
  id: string | null;
  label: string;
  cues: CueSummary[];
}

const SCENE_MATRIX_STRIP_DRAG_THRESHOLD_PX = 4;

export function SceneMatrixPanel(props: SceneMatrixPanelProps) {
  const [dragCueId, setDragCueId] = createSignal<number | null>(null);
  const [activeBankId, setActiveBankId] = createSignal("Show");
  const [dropTargetColumnId, setDropTargetColumnId] = createSignal<string | null>(null);
  const [dropIndicator, setDropIndicator] = createSignal<{
    cueId: number;
    position: "before" | "after";
  } | null>(null);
  let scrollerElement: HTMLDivElement | undefined;
  let preferredBankId: string | null = null;
  let dragPointer: {
    pointerId: number;
    startClientX: number;
    startClientY: number;
    moved: boolean;
    cue: CueSummary;
  } | null = null;
  let suppressClickCueId: number | null = null;

  // Engine snapshots deserialize every Cue into a fresh object. Keep the
  // rendered Cue objects keyed by id so a polling-only snapshot cannot remount
  // an active card and dismiss its native select/input interaction.
  const [stableCues, setStableCues] = createStore<CueSummary[]>(props.cues);
  createEffect(() => {
    setStableCues(reconcile(props.cues, { key: "id" }));
  });

  const calculatedColumns = createMemo<SceneMatrixColumn[]>(() => {
    const groupSet = new Set(props.groupIds);
    const groupColumns = props.groupIds.map((groupId) => ({
      id: groupId,
      label: groupId,
      cues: stableCues.filter((cue) => cue.group_id === groupId),
    }));
    const showCues = stableCues.filter((cue) => !cue.group_id || !groupSet.has(cue.group_id));
    return showCues.length > 0
      ? [...groupColumns, { id: null, label: "Show", cues: showCues }]
      : groupColumns;
  });
  const [stableColumns, setStableColumns] = createStore<SceneMatrixColumn[]>([]);
  createEffect(() => {
    setStableColumns(reconcile(calculatedColumns(), { key: "id" }));
  });
  const columns = () => stableColumns;

  const syncVisibleBank = () => {
    if (!scrollerElement) return;
    const scrollerRect = scrollerElement.getBoundingClientRect();
    if (preferredBankId) {
      const preferredColumn = [...scrollerElement.querySelectorAll<HTMLElement>(
        "[data-scene-matrix-column]",
      )].find((column) => column.dataset.sceneMatrixColumn === preferredBankId);
      if (preferredColumn) {
        const preferredRect = preferredColumn.getBoundingClientRect();
        const preferredVisibleWidth = Math.max(
          0,
          Math.min(preferredRect.right, scrollerRect.right) -
            Math.max(preferredRect.left, scrollerRect.left),
        );
        if (preferredVisibleWidth >= 2) {
          setActiveBankId(preferredBankId);
          return;
        }
      }
      preferredBankId = null;
    }
    let bestColumnId = columns()[0]?.id ?? "Show";
    let bestVisibleWidth = -1;
    for (const column of scrollerElement.querySelectorAll<HTMLElement>("[data-scene-matrix-column]")) {
      const rect = column.getBoundingClientRect();
      const visibleWidth = Math.max(
        0,
        Math.min(rect.right, scrollerRect.right) - Math.max(rect.left, scrollerRect.left),
      );
      if (visibleWidth > bestVisibleWidth) {
        bestVisibleWidth = visibleWidth;
        bestColumnId = column.dataset.sceneMatrixColumn ?? "Show";
      }
    }
    setActiveBankId(bestColumnId);
  };

  const jumpToBank = (columnId: string) => {
    if (!scrollerElement) return;
    const target = [...scrollerElement.querySelectorAll<HTMLElement>("[data-scene-matrix-column]")]
      .find((column) => column.dataset.sceneMatrixColumn === columnId);
    if (!target) return;
    preferredBankId = columnId;
    scrollerElement.scrollTo({
      left: Math.max(0, target.offsetLeft - 3),
      behavior: "auto",
    });
    setActiveBankId(columnId);
    window.requestAnimationFrame(syncVisibleBank);
  };

  const handleMatrixWheel = (event: WheelEvent & { currentTarget: HTMLDivElement }) => {
    const columnScroller = event.target instanceof Element
      ? event.target.closest<HTMLElement>("[data-scene-matrix-column-scroll]")
      : null;
    const verticalWheelDominant = Math.abs(event.deltaY) >= Math.abs(event.deltaX);
    if (
      !event.shiftKey &&
      verticalWheelDominant &&
      columnScroller &&
      columnScroller.scrollHeight > columnScroller.clientHeight + 1
    ) {
      return;
    }
    handleHorizontalWheel(event);
  };

  onMount(() => {
    syncVisibleBank();
    const resizeObserver = new ResizeObserver(syncVisibleBank);
    if (scrollerElement) resizeObserver.observe(scrollerElement);
    onCleanup(() => resizeObserver.disconnect());
  });

  const beginDrag = (event: PointerEvent & { currentTarget: HTMLElement }, cue: CueSummary) => {
    if (event.button !== 0 || !event.isPrimary) return;
    dragPointer = {
      pointerId: event.pointerId,
      startClientX: event.clientX,
      startClientY: event.clientY,
      moved: false,
      cue,
    };
  };

  const updateDropIndicator = (
    event: PointerEvent & { currentTarget: HTMLElement },
  ) => {
    if (!dragPointer?.moved) {
      setDropIndicator(null);
      setDropTargetColumnId(null);
      return;
    }
    const sourceCard = event.currentTarget.closest<HTMLElement>("[data-scene-matrix-cue-id]");
    const hitElement = document.elementFromPoint(event.clientX, event.clientY);
    const targetCard = hitElement
      ?.closest<HTMLElement>("[data-scene-matrix-cue-id]");
    const sourceColumn = sourceCard?.closest<HTMLElement>("[data-scene-matrix-column]");
    const targetColumn = hitElement?.closest<HTMLElement>("[data-scene-matrix-column]");
    const sourceColumnId = sourceColumn?.dataset.sceneMatrixColumn;
    const targetColumnId = targetColumn?.dataset.sceneMatrixColumn;
    const targetCueId = Number(targetCard?.dataset.sceneMatrixCueId);
    const sameColumn = sourceColumnId === targetColumnId;
    const sameCueList = !targetCard ||
      sourceCard?.dataset.sceneMatrixCueListId === targetCard.dataset.sceneMatrixCueListId;
    if (
      !targetColumnId ||
      !sameCueList ||
      (sameColumn && !targetCard) ||
      (targetCard && (!Number.isFinite(targetCueId) || targetCueId === dragPointer.cue.id))
    ) {
      setDropIndicator(null);
      setDropTargetColumnId(null);
      return;
    }
    setDropTargetColumnId(targetColumnId);
    if (!targetCard) {
      setDropIndicator(null);
      return;
    }
    const targetRect = targetCard.getBoundingClientRect();
    setDropIndicator({
      cueId: targetCueId,
      position: event.clientY < targetRect.top + targetRect.height / 2 ? "before" : "after",
    });
  };

  const moveDrag = (event: PointerEvent & { currentTarget: HTMLElement }) => {
    if (!dragPointer || dragPointer.pointerId !== event.pointerId) return;
    const crossedThreshold = Math.hypot(
        event.clientX - dragPointer.startClientX,
        event.clientY - dragPointer.startClientY,
      ) > SCENE_MATRIX_STRIP_DRAG_THRESHOLD_PX;
    if (!dragPointer.moved && crossedThreshold) {
      dragPointer.moved = true;
      event.currentTarget.setPointerCapture(event.pointerId);
      setDragCueId(dragPointer.cue.id);
      props.onBeginTimelineCueDrag(
        dragPointer.cue,
        {
          pointerId: event.pointerId,
          clientX: dragPointer.startClientX,
          clientY: dragPointer.startClientY,
        },
        "scene-matrix",
      );
    }
    if (!dragPointer.moved) return;
    event.preventDefault();
    updateDropIndicator(event);
    props.onMoveTimelineCueDrag({
      pointerId: event.pointerId,
      clientX: event.clientX,
      clientY: event.clientY,
    });
  };

  const finishDrag = (
    event: PointerEvent & { currentTarget: HTMLElement },
    canceled: boolean,
  ) => {
    if (!dragPointer || dragPointer.pointerId !== event.pointerId) return;
    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }
    const moved = dragPointer.moved;
    const cueId = dragPointer.cue.id;
    dragPointer = null;
    setDragCueId(null);
    setDropTargetColumnId(null);
    setDropIndicator(null);
    if (!moved) {
      if (canceled) return;
      // WebView pointer sequences do not always synthesize a trailing click.
      // Select on pointer-up so the strip remains a reliable click target,
      // then swallow the compatibility click when the browser does emit one.
      event.preventDefault();
      suppressClickCueId = cueId;
      props.onSelectCue(cueId);
      window.setTimeout(() => {
        if (suppressClickCueId === cueId) suppressClickCueId = null;
      }, 0);
      return;
    }
    event.preventDefault();
    suppressClickCueId = cueId;
    window.setTimeout(() => {
      if (suppressClickCueId === cueId) suppressClickCueId = null;
    }, 0);
    props.onEndTimelineCueDrag({
      pointerId: event.pointerId,
      clientX: event.clientX,
      clientY: event.clientY,
    }, moved, canceled);
  };

  const isActive = (cue: CueSummary) => cue.group_id
    ? props.activeGroupCueIds[cue.group_id] === cue.id
    : props.activeCueId === cue.id;
  const progressValue = (cue: CueSummary) => props.activeFade?.cue_id === cue.id
    ? Math.max(0, Math.min(1, props.activeFade.progress))
    : 1;

  return (
    <section
      class="sceneMatrixPanel"
      aria-label="Scene matrix grouped by scene bank"
      data-timeline-track={props.timelineTrack}
    >
      <header class="sceneMatrixSurfaceHeader">
        <nav
          class="sceneMatrixBankJumpStrip"
          aria-label="Scene bank navigation"
          data-wheel-scroll-surface="scene-bank-chips"
          onWheel={handleHorizontalWheel}
        >
          <For each={columns()}>
            {(column) => {
              const columnId = () => column.id ?? "Show";
              const groupText = () =>
                groupIdentityCss(columnId(), props.groupColors, "text");
              return (
                <button
                  type="button"
                  classList={{ active: activeBankId() === columnId() }}
                  style={{ "--group-identity-text": groupText() }}
                  data-scene-matrix-bank-jump={columnId()}
                  aria-current={activeBankId() === columnId() ? "true" : undefined}
                  aria-label={`Jump to scene bank ${column.label}`}
                  onClick={() => jumpToBank(columnId())}
                >
                  <span data-no-localize={column.id !== null ? true : undefined}>{column.label}</span>
                  <small>{column.cues.length}</small>
                </button>
              );
            }}
          </For>
        </nav>
        {props.toolbar}
      </header>
      <div
        class="sceneMatrixScroller"
        ref={(element) => {
          scrollerElement = element;
        }}
        onScroll={syncVisibleBank}
        data-wheel-scroll-surface="scene-matrix-banks"
        onWheel={handleMatrixWheel}
      >
        <Show when={stableCues.length > 0} fallback={
          <div class="sceneMatrixEmptyAction">
            <strong>No scenes yet.</strong>
            <button
              type="button"
              data-scene-matrix-open-cue-editor
              onClick={props.onOpenCueEditor}
            >
              Create scene
            </button>
          </div>
        }>
        <div class="sceneMatrixColumns">
          <For each={columns()}>
            {(column) => {
              // The data attribute keeps the deterministic hash hue for the
              // harness; the rendered colors prefer the persisted group color.
              const hue = () => groupIdentityHue(column.id ?? "Show");
              const groupCss = (role: "fill" | "text") =>
                groupIdentityCss(column.id ?? "Show", props.groupColors, role);
              return (
                <section
                  class="sceneMatrixColumn"
                  style={{
                    "--group-identity": groupCss("fill"),
                    "--group-identity-text": groupCss("text"),
                  }}
                  data-scene-matrix-column={column.id ?? "Show"}
                  data-scene-matrix-drop-target={
                    dropTargetColumnId() === (column.id ?? "Show") ? "true" : undefined
                  }
                  aria-label={`Scene matrix column ${column.label}`}
                >
                  <i class="sceneMatrixBankStrip" aria-hidden="true" />
                  <header
                    class="sceneMatrixColumnHeader"
                    data-scene-matrix-group-hue={hue()}
                  >
                    <strong data-no-localize={column.id !== null ? true : undefined}>{column.label}</strong>
                    <span>{column.cues.length}</span>
                    <Show when={column.id !== null && props.onSetGroupColor}>
                      <span class="groupColorControls">
                        <input
                          type="color"
                          value={props.groupColors?.[column.id!] ?? "#5f6b76"}
                          data-group-color-input={column.id!}
                          aria-label={`Identity color for group ${column.label}`}
                          onChange={(event) => void props.onSetGroupColor!(column.id!, event.currentTarget.value)}
                        />
                        <button
                          type="button"
                          disabled={!props.groupColors?.[column.id!]}
                          data-group-color-clear={column.id!}
                          title="Clear Color"
                          aria-label={`Clear identity color for group ${column.label}`}
                          onClick={() => void props.onSetGroupColor!(column.id!, null)}
                        >
                          ×
                        </button>
                      </span>
                    </Show>
                  </header>
                  <div
                    class="sceneMatrixCards"
                    data-scene-matrix-column-scroll
                    data-scene-matrix-column-drop-position={
                      dropTargetColumnId() === (column.id ?? "Show") && !dropIndicator()
                        ? "after"
                        : undefined
                    }
                  >
                    <Show when={column.cues.length > 0} fallback={<p class="empty">No scenes in this column.</p>}>
                      <For each={column.cues}>
                        {(cue) => {
                          const flashMode = () => authoredCueLiveModifier(cue).flash;
                          const kind = () => sceneCueKind(cue);
                          const flashRelease = (event: PointerEvent) => {
                            if (!flashMode()) return;
                            event.stopPropagation();
                            void props.onReleaseCue(cue.id);
                          };
                          return (
                            <article
                              class="sceneMatrixCard"
                              classList={{
                                active: isActive(cue),
                                selected: props.selectedCueId === cue.id,
                              }}
                              style={{
                                "--cue-identity": cueIdentityCss(
                                  cue.id,
                                  cue.color,
                                  "fill",
                                  cue.group_id,
                                  cue.group_id ? props.groupColors?.[cue.group_id] : null,
                                ),
                                "--cue-identity-text": cueIdentityCss(
                                  cue.id,
                                  cue.color,
                                  "text",
                                  cue.group_id,
                                  cue.group_id ? props.groupColors?.[cue.group_id] : null,
                                ),
                              }}
                              data-scene-matrix-cue-id={cue.id}
                              data-scene-matrix-cue-list-id={cue.cue_list_id}
                              data-scene-matrix-cue-hue={hue()}
                              data-scene-matrix-active={isActive(cue) ? "true" : "false"}
                              data-scene-matrix-selected={props.selectedCueId === cue.id ? "true" : "false"}
                              data-scene-matrix-drop-position={
                                dropIndicator()?.cueId === cue.id
                                  ? dropIndicator()!.position
                                  : undefined
                              }
                              data-timeline-cue-drag-source={cue.id}
                            >
                              <button
                                type="button"
                                class="sceneMatrixTrigger"
                                classList={{ flash: flashMode() }}
                                data-scene-flash-cue={flashMode() ? cue.id : undefined}
                                {...controlMappingTargetData({
                                  action: "TriggerCue",
                                  cue_id: cue.id,
                                  label: `Cue ${cue.cue_number || cue.id} ${cue.label}`,
                                })}
                                aria-label={
                                  flashMode()
                                    ? `Flash Cue ${cue.label}`
                                    : isActive(cue)
                                      ? `Release Cue ${cue.label}`
                                      : `Trigger Cue ${cue.label}`
                                }
                                onPointerDown={(event) => {
                                  if (!flashMode()) return;
                                  // Flash pads are momentary: press activates the
                                  // scene, release always releases it, and the
                                  // press never starts a timeline drag.
                                  event.stopPropagation();
                                  event.preventDefault();
                                  event.currentTarget.setPointerCapture(event.pointerId);
                                  void props.onTriggerCue(cue.id);
                                }}
                                onPointerUp={flashRelease}
                                onPointerCancel={flashRelease}
                                onKeyDown={(event) => {
                                  if (!flashMode() || event.repeat) return;
                                  if (event.key === " " || event.key === "Enter") {
                                    event.preventDefault();
                                    void props.onTriggerCue(cue.id);
                                  }
                                }}
                                onKeyUp={(event) => {
                                  if (!flashMode()) return;
                                  if (event.key === " " || event.key === "Enter") {
                                    event.preventDefault();
                                    void props.onReleaseCue(cue.id);
                                  }
                                }}
                                onClick={(event) => {
                                  if (flashMode()) {
                                    event.preventDefault();
                                    return;
                                  }
                                  if (suppressClickCueId === cue.id) {
                                    suppressClickCueId = null;
                                    event.preventDefault();
                                    return;
                                  }
                                  if (isActive(cue)) {
                                    void props.onReleaseCue(cue.id);
                                  } else {
                                    void props.onTriggerCue(cue.id);
                                  }
                                }}
                              >
                                <span
                                  class="sceneMatrixCuePrimaryRow"
                                  data-scene-matrix-primary-row
                                >
                                  <span data-no-localize class="sceneMatrixCueNumber">{cue.cue_number || cue.id}</span>
                                  <strong
                                    data-no-localize
                                    data-scene-matrix-cue-name
                                    title={cue.label}
                                  >
                                    {cue.label}
                                  </strong>
                                </span>
                                <span
                                  class="sceneMatrixCueMetaRow"
                                  classList={{ hasSuperScene: Boolean(cue.child_timeline) }}
                                  data-scene-matrix-meta-row
                                >
                                  <span class="sceneMatrixTypeBadges">
                                    <span
                                      class={`sceneMatrixKindBadge uiMicroLabel ${kind() === "TIMELINE" ? "super" : kind().toLowerCase()}`}
                                      data-scene-matrix-kind={kind()}
                                      data-no-localize
                                    >
                                      {kind()}
                                    </span>
                                  </span>
                                  <Show when={flashMode()}>
                                    <span class="sceneMatrixFlashBadge" data-scene-flash-badge={cue.id}>
                                      FLASH
                                    </span>
                                  </Show>
                                  <Show when={(cue.recall_mode ?? "Coexist") === "ReplaceGroup"}>
                                    <span class="sceneMatrixReplaceBadge">Replace group</span>
                                  </Show>
                                  <small data-scene-matrix-time>{displayNumber(cue.fade_ms, 0)}ms</small>
                                </span>
                              </button>
                              <Show when={cue.child_timeline}>
                                <button
                                  type="button"
                                  class="sceneMatrixKindBadge superScene sceneMatrixSuperSceneAction"
                                  data-scene-matrix-super-scene={cue.id}
                                  data-no-localize
                                  title={`Open Timeline ${cue.label}`}
                                  aria-label={`Open Timeline ${cue.label}`}
                                  onPointerDown={(event) => event.stopPropagation()}
                                  onPointerMove={(event) => event.stopPropagation()}
                                  onPointerUp={(event) => event.stopPropagation()}
                                  onPointerCancel={(event) => event.stopPropagation()}
                                  onClick={(event) => {
                                    event.preventDefault();
                                    event.stopPropagation();
                                    void props.onOpenSuperScene(cue.id);
                                  }}
                                >
                                  TL
                                </button>
                              </Show>
                              <button
                                type="button"
                                class="sceneMatrixEditStrip"
                                classList={{ dragging: dragCueId() === cue.id }}
                                data-scene-matrix-edit-strip={cue.id}
                                data-scene-matrix-drag-threshold={SCENE_MATRIX_STRIP_DRAG_THRESHOLD_PX}
                                title={`Click to select Cue ${cue.label}; drag to reorder, move between banks, or place on Timeline`}
                                aria-label={`Edit scene settings for Cue ${cue.label}`}
                                aria-pressed={props.selectedCueId === cue.id}
                                onPointerDown={(event) => {
                                  event.stopPropagation();
                                  beginDrag(event, cue);
                                }}
                                onPointerMove={(event) => {
                                  event.stopPropagation();
                                  moveDrag(event);
                                }}
                                onPointerUp={(event) => {
                                  event.stopPropagation();
                                  finishDrag(event, false);
                                }}
                                onPointerCancel={(event) => {
                                  event.stopPropagation();
                                  finishDrag(event, true);
                                }}
                                onClick={(event) => {
                                  event.stopPropagation();
                                  if (suppressClickCueId === cue.id) {
                                    suppressClickCueId = null;
                                    event.preventDefault();
                                    return;
                                  }
                                  props.onSelectCue(cue.id);
                                }}
                              >
                                <span
                                  class="sceneMatrixEditStripBand"
                                  aria-hidden="true"
                                  data-no-localize
                                />
                              </button>
                              <Show when={isActive(cue)}>
                                <div class="sceneMatrixProgress" data-scene-matrix-progress={cue.id}>
                                  <span>LIVE</span>
                                  <progress
                                    max="1"
                                    value={progressValue(cue)}
                                    aria-label={`Cue ${cue.label} progress`}
                                  />
                                </div>
                              </Show>
                              <Show
                                when={
                                  isActive(cue) &&
                                  props.onSetCueLiveModifier &&
                                  props.onClearCueLiveModifier
                                }
                              >
                                <CueLiveModifierStrip
                                  cue={cue}
                                  liveStates={props.cueLiveModifiers}
                                  onSetCueLiveModifier={props.onSetCueLiveModifier!}
                                  onClearCueLiveModifier={props.onClearCueLiveModifier!}
                                />
                              </Show>
                            </article>
                          );
                        }}
                      </For>
                    </Show>
                  </div>
                </section>
              );
            }}
          </For>
        </div>
        </Show>
      </div>
    </section>
  );
}
