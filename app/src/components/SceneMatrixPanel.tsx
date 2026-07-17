import { createMemo, createSignal, For, Show } from "solid-js";
import { cueIdentityHue, groupIdentityHue, identityCssColor } from "../identityColor";
import type { CueSummary, TimelineTrackKind } from "../types";
import type { TimelineCueDragPoint } from "../timelineCueDrag";

interface SceneMatrixPanelProps {
  cues: CueSummary[];
  groupIds: string[];
  activeCueId: number | null | undefined;
  activeGroupCueIds: Record<string, number>;
  onTriggerCue: (cueId: number) => void | Promise<void>;
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
  } | null = null;

  const columns = createMemo<SceneMatrixColumn[]>(() => {
    const groupSet = new Set(props.groupIds);
    return [
      ...props.groupIds.map((groupId) => ({
        id: groupId,
        label: groupId,
        cues: props.cues.filter((cue) => cue.group_id === groupId),
      })),
      {
        id: null,
        label: "Show",
        cues: props.cues.filter((cue) => !cue.group_id || !groupSet.has(cue.group_id)),
      },
    ];
  });

  const beginDrag = (event: PointerEvent & { currentTarget: HTMLButtonElement }, cue: CueSummary) => {
    if (event.button !== 0) return;
    event.preventDefault();
    event.stopPropagation();
    event.currentTarget.setPointerCapture(event.pointerId);
    dragPointer = {
      pointerId: event.pointerId,
      startClientX: event.clientX,
      startClientY: event.clientY,
      moved: false,
    };
    setDragCueId(cue.id);
    props.onBeginTimelineCueDrag(cue, {
      pointerId: event.pointerId,
      clientX: event.clientX,
      clientY: event.clientY,
    });
  };

  const moveDrag = (event: PointerEvent & { currentTarget: HTMLButtonElement }) => {
    if (!dragPointer || dragPointer.pointerId !== event.pointerId) return;
    event.preventDefault();
    event.stopPropagation();
    dragPointer.moved = dragPointer.moved
      || Math.hypot(
        event.clientX - dragPointer.startClientX,
        event.clientY - dragPointer.startClientY,
      ) >= 4;
    props.onMoveTimelineCueDrag({
      pointerId: event.pointerId,
      clientX: event.clientX,
      clientY: event.clientY,
    });
  };

  const finishDrag = (
    event: PointerEvent & { currentTarget: HTMLButtonElement },
    canceled: boolean,
  ) => {
    if (!dragPointer || dragPointer.pointerId !== event.pointerId) return;
    event.preventDefault();
    event.stopPropagation();
    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }
    const moved = dragPointer.moved;
    dragPointer = null;
    setDragCueId(null);
    props.onEndTimelineCueDrag({
      pointerId: event.pointerId,
      clientX: event.clientX,
      clientY: event.clientY,
    }, moved, canceled);
  };

  const isActive = (cue: CueSummary) => cue.group_id
    ? props.activeGroupCueIds[cue.group_id] === cue.id
    : props.activeCueId === cue.id;

  return (
    <section class="sceneMatrixPanel" aria-label="Scene matrix grouped by fixture group">
      <div class="panelHeader sceneMatrixHeader">
        <div>
          <p class="eyebrow">{props.timelineTrack}</p>
          <h2>Scene Matrix</h2>
        </div>
        <span>{props.cues.length} scenes</span>
      </div>
      <div class="sceneMatrixScroller">
        <div class="sceneMatrixColumns">
          <For each={columns()}>
            {(column) => {
              const hue = () => groupIdentityHue(column.id ?? "Show");
              return (
                <section
                  class="sceneMatrixColumn"
                  data-scene-matrix-column={column.id ?? "Show"}
                  aria-label={`Scene matrix column ${column.label}`}
                >
                  <header
                    class="sceneMatrixColumnHeader"
                    style={{
                      "--group-identity": identityCssColor(hue(), "fill"),
                      "--group-identity-text": identityCssColor(hue(), "text"),
                    }}
                    data-scene-matrix-group-hue={hue()}
                  >
                    <strong data-no-localize={column.id !== null ? true : undefined}>{column.label}</strong>
                    <span>{column.cues.length}</span>
                  </header>
                  <div class="sceneMatrixCards">
                    <Show when={column.cues.length > 0} fallback={<p class="empty">No scenes in this column.</p>}>
                      <For each={column.cues}>
                        {(cue) => {
                          const hue = cueIdentityHue(cue.id);
                          return (
                            <article
                              class="sceneMatrixCard"
                              classList={{ active: isActive(cue) }}
                              style={{
                                "--cue-identity": identityCssColor(hue, "fill"),
                                "--cue-identity-text": identityCssColor(hue, "text"),
                              }}
                              data-scene-matrix-cue-id={cue.id}
                              data-scene-matrix-cue-hue={hue}
                              data-scene-matrix-active={isActive(cue) ? "true" : "false"}
                            >
                              <button
                                type="button"
                                class="sceneMatrixTrigger"
                                aria-label={`Trigger Cue ${cue.label}`}
                                onClick={() => void props.onTriggerCue(cue.id)}
                              >
                                <span data-no-localize class="sceneMatrixCueNumber">{cue.cue_number || cue.id}</span>
                                <strong data-no-localize>{cue.label}</strong>
                                <small>{cue.fade_ms}ms</small>
                              </button>
                              <div class="sceneMatrixCardFooter">
                                <Show when={(cue.recall_mode ?? "Coexist") === "ReplaceGroup"}>
                                  <span class="sceneMatrixReplaceBadge">Replace group</span>
                                </Show>
                                <button
                                  type="button"
                                  class="cueTimelineDragHandle"
                                  classList={{ dragging: dragCueId() === cue.id }}
                                  title={`Drag Cue ${cue.label} to Timeline`}
                                  aria-label={`Drag Cue ${cue.label} to Timeline`}
                                  data-timeline-cue-drag-source={cue.id}
                                  onPointerDown={(event) => beginDrag(event, cue)}
                                  onPointerMove={moveDrag}
                                  onPointerUp={(event) => finishDrag(event, false)}
                                  onPointerCancel={(event) => finishDrag(event, true)}
                                >
                                  <span aria-hidden="true" data-no-localize>⠿</span>
                                </button>
                              </div>
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
      </div>
    </section>
  );
}
