import { createMemo, createSignal, For, Show } from "solid-js";
import { cueIdentityCss, groupIdentityCss, groupIdentityHue } from "../identityColor";
import type { ActiveFadeSummary, CueSummary, TimelineTrackKind } from "../types";
import type { TimelineCueDragPoint } from "../timelineCueDrag";

interface SceneMatrixPanelProps {
  cues: CueSummary[];
  groupColors?: Record<string, string>;
  onSetGroupColor?: (groupId: string, color: string | null) => void | Promise<void>;
  groupIds: string[];
  activeCueId: number | null | undefined;
  activeGroupCueIds: Record<string, number>;
  activeFade?: ActiveFadeSummary | null;
  onTriggerCue: (cueId: number) => void | Promise<void>;
  onEditCue: (cueId: number) => void;
  onOpenCueEditor: () => void;
  onBeginTimelineCueDrag: (cue: CueSummary, point: TimelineCueDragPoint) => void;
  onMoveTimelineCueDrag: (point: TimelineCueDragPoint) => void;
  onEndTimelineCueDrag: (point: TimelineCueDragPoint, moved: boolean, canceled: boolean) => void;
  timelineTrack: TimelineTrackKind;
}

interface SceneMatrixColumn {
  id: string | null;
  label: string;
  cues: CueSummary[];
}

export function SceneMatrixPanel(props: SceneMatrixPanelProps) {
  const [dragCueId, setDragCueId] = createSignal<number | null>(null);
  let dragPointer: {
    pointerId: number;
    startClientX: number;
    startClientY: number;
    moved: boolean;
    cue: CueSummary;
  } | null = null;
  let suppressClickCueId: number | null = null;

  const columns = createMemo<SceneMatrixColumn[]>(() => {
    const groupSet = new Set(props.groupIds);
    const groupColumns = props.groupIds.map((groupId) => ({
      id: groupId,
      label: groupId,
      cues: props.cues.filter((cue) => cue.group_id === groupId),
    }));
    const showCues = props.cues.filter((cue) => !cue.group_id || !groupSet.has(cue.group_id));
    return showCues.length > 0
      ? [...groupColumns, { id: null, label: "Show", cues: showCues }]
      : groupColumns;
  });

  const beginDrag = (event: PointerEvent & { currentTarget: HTMLElement }, cue: CueSummary) => {
    if (event.button !== 0) return;
    dragPointer = {
      pointerId: event.pointerId,
      startClientX: event.clientX,
      startClientY: event.clientY,
      moved: false,
      cue,
    };
  };

  const moveDrag = (event: PointerEvent & { currentTarget: HTMLElement }) => {
    if (!dragPointer || dragPointer.pointerId !== event.pointerId) return;
    const crossedThreshold = Math.hypot(
        event.clientX - dragPointer.startClientX,
        event.clientY - dragPointer.startClientY,
      ) > 4;
    if (!dragPointer.moved && crossedThreshold) {
      dragPointer.moved = true;
      event.currentTarget.setPointerCapture(event.pointerId);
      setDragCueId(dragPointer.cue.id);
      props.onBeginTimelineCueDrag(dragPointer.cue, {
        pointerId: event.pointerId,
        clientX: dragPointer.startClientX,
        clientY: dragPointer.startClientY,
      });
    }
    if (!dragPointer.moved) return;
    event.preventDefault();
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
    if (!moved) return;
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
      <div class="sceneMatrixScroller">
        <Show when={props.cues.length > 0} fallback={
          <div class="sceneMatrixEmptyAction">
            <strong>No scenes yet.</strong>
            <button
              type="button"
              data-scene-matrix-open-cue-editor
              onClick={props.onOpenCueEditor}
            >
              Open Cue editor
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
                  <div class="sceneMatrixCards">
                    <Show when={column.cues.length > 0} fallback={<p class="empty">No scenes in this column.</p>}>
                      <For each={column.cues}>
                        {(cue) => {
                          return (
                            <article
                              class="sceneMatrixCard"
                              classList={{ active: isActive(cue) }}
                              style={{
                                "--cue-identity": cueIdentityCss(cue.id, cue.color, "fill"),
                                "--cue-identity-text": cueIdentityCss(cue.id, cue.color, "text"),
                              }}
                              data-scene-matrix-cue-id={cue.id}
                              data-scene-matrix-cue-hue={hue()}
                              data-scene-matrix-active={isActive(cue) ? "true" : "false"}
                              data-timeline-cue-drag-source={cue.id}
                              onPointerDown={(event) => beginDrag(event, cue)}
                              onPointerMove={moveDrag}
                              onPointerUp={(event) => finishDrag(event, false)}
                              onPointerCancel={(event) => finishDrag(event, true)}
                            >
                              <button
                                type="button"
                                class="sceneMatrixTrigger"
                                aria-label={`Trigger Cue ${cue.label}`}
                                onClick={(event) => {
                                  if (suppressClickCueId === cue.id) {
                                    suppressClickCueId = null;
                                    event.preventDefault();
                                    return;
                                  }
                                  void props.onTriggerCue(cue.id);
                                }}
                              >
                                <span data-no-localize class="sceneMatrixCueNumber">{cue.cue_number || cue.id}</span>
                                <strong data-no-localize>{cue.label}</strong>
                                <small>{cue.fade_ms}ms</small>
                              </button>
                              <div class="sceneMatrixCardFooter">
                                <span
                                  class={`sceneMatrixKindBadge ${cue.effect_targets.length > 0 ? "fx" : "static"}`}
                                  data-scene-matrix-kind={cue.effect_targets.length > 0 ? "FX" : "STATIC"}
                                >
                                  {cue.effect_targets.length > 0 ? "FX" : "STATIC"}
                                </span>
                                <Show when={(cue.recall_mode ?? "Coexist") === "ReplaceGroup"}>
                                  <span class="sceneMatrixReplaceBadge">Replace group</span>
                                </Show>
                                <button
                                  type="button"
                                  class="sceneMatrixEditCue"
                                  data-scene-matrix-edit-cue={cue.id}
                                  title={`Edit Source for Cue ${cue.label}`}
                                  aria-label={`Edit Source for Cue ${cue.label}`}
                                  onPointerDown={(event) => event.stopPropagation()}
                                  onPointerMove={(event) => event.stopPropagation()}
                                  onPointerUp={(event) => event.stopPropagation()}
                                  onPointerCancel={(event) => event.stopPropagation()}
                                  onClick={(event) => {
                                    event.preventDefault();
                                    event.stopPropagation();
                                    props.onEditCue(cue.id);
                                  }}
                                >
                                  <span aria-hidden="true" data-no-localize>✎</span>
                                </button>
                                <button
                                  type="button"
                                  class="cueTimelineDragHandle"
                                  classList={{ dragging: dragCueId() === cue.id }}
                                  title={`Drag Cue ${cue.label} to Timeline`}
                                  aria-label={`Drag Cue ${cue.label} to Timeline`}
                                  onClick={(event) => event.preventDefault()}
                                >
                                  <span aria-hidden="true" data-no-localize>⠿</span>
                                </button>
                              </div>
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
