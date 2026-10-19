import { createSignal, type JSX } from "solid-js";

/** Drag payload type of a chain stage. Distinct from the effect-card payload, so a stage drag is never
 * mistaken for an "add effect" drop on a layer row or the FX tab. */
export const FX_STAGE_DRAG_MIME = "application/x-syndocal-fx-stage";

export interface FxStageReorderOptions {
  /** The layer whose chain is shown; a drag started on another layer is ignored. */
  layerId: () => number | null;
  count: () => number;
  /** True while a command is in flight or the chain cannot be edited. */
  disabled: () => boolean;
  /** Moves the stage at `from` so it ends at `to`. Never called with equal indexes. */
  onReorder: (layerId: number, from: number, to: number) => void;
}

type DragHandlers = Pick<JSX.HTMLAttributes<HTMLElement>, "onDragStart" | "onDragEnd" | "onDragOver" | "onDragLeave" | "onDrop">;

/** Pointer drag-and-drop for chain stages. It only reports an intent; the owner decides which ticketed
 * chain command carries it, and the Up/Down buttons stay as the keyboard path. */
export function createFxStageReorder(options: FxStageReorderOptions) {
  const [source, setSource] = createSignal<{ layerId: number; index: number } | null>(null);
  const [target, setTarget] = createSignal<number | null>(null);
  const reset = () => { setSource(null); setTarget(null); };
  const accepts = (event: DragEvent) => {
    const active = source();
    return Boolean(active) && active!.layerId === options.layerId() && !options.disabled()
      && Boolean(event.dataTransfer?.types.includes(FX_STAGE_DRAG_MIME));
  };
  return {
    dragging: () => source()?.index ?? null,
    /** "before" or "after" the row at `index` when it is the current drop target. */
    dropEdge: (index: number): "before" | "after" | undefined => {
      const from = source()?.index;
      if (target() !== index || from === undefined || from === index) return undefined;
      return from < index ? "after" : "before";
    },
    canDrag: () => options.count() > 1 && !options.disabled() && options.layerId() !== null,
    handlers(index: () => number): DragHandlers {
      return {
        onDragStart: (event) => {
          const layerId = options.layerId();
          if (layerId === null || options.disabled() || options.count() < 2 || !event.dataTransfer) { event.preventDefault(); return; }
          event.dataTransfer.setData(FX_STAGE_DRAG_MIME, `${layerId}:${index()}`);
          event.dataTransfer.effectAllowed = "move";
          setSource({ layerId, index: index() });
        },
        onDragEnd: reset,
        onDragOver: (event) => {
          if (!accepts(event)) return;
          event.preventDefault();
          if (event.dataTransfer) event.dataTransfer.dropEffect = "move";
          setTarget(index());
        },
        onDragLeave: (event) => {
          if (!(event.relatedTarget instanceof Node) || !(event.currentTarget as Node).contains(event.relatedTarget)) {
            if (target() === index()) setTarget(null);
          }
        },
        onDrop: (event) => {
          if (!accepts(event)) { reset(); return; }
          event.preventDefault();
          const active = source()!;
          const to = index();
          reset();
          if (to !== active.index && to >= 0 && to < options.count()) options.onReorder(active.layerId, active.index, to);
        },
      };
    },
  };
}
