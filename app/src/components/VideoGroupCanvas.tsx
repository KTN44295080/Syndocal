import { createEffect, createMemo, createSignal, For, onCleanup, onMount, Show, untrack } from "solid-js";
import type { Transform2D, VideoLayerGroupSummary } from "../types";
import { dragGroupTransform, groupCanvasCorners, groupCanvasHandlePoint, nudgeGroupTransform, type GroupCanvasHandle, type GroupCanvasPoint } from "../videoGroupCanvasGeometry";
import "./VideoGroupCanvas.css";

interface Props {
  group: VideoLayerGroupSummary;
  transform: Transform2D;
  identity: string;
  disabled?: boolean;
  aspect: number;
  onLoadPreview?: (compositionId: number, width: number, height: number) => Promise<string>;
  onDraft: (value: Transform2D | null) => void;
  onBusy: (busy: boolean) => void;
  onCommit: (value: Transform2D) => void | Promise<unknown>;
}

/** Authoring-only DOM overlay; no handles are ever passed to a video renderer. */
export function VideoGroupCanvas(props: Props) {
  let svg!: SVGSVGElement;
  let viewport!: HTMLDivElement;
  let capture: Element | null = null;
  let drag: { pointerId: number; handle: GroupCanvasHandle; start: GroupCanvasPoint; transform: Transform2D; identity: string } | null = null;
  let disposed = false, previewGeneration = 0, previewInFlight = false, previewQueued = false;
  const [pending, setPending] = createSignal(false);
  const [preview, setPreview] = createSignal("");
  const [error, setError] = createSignal("");
  const [previewError, setPreviewError] = createSignal(false);
  const [viewportSize, setViewportSize] = createSignal({ width: 320, height: 180 });
  const point = (event: PointerEvent): GroupCanvasPoint => {
    const matrix = svg.getScreenCTM();
    if (!matrix) throw new Error("Group canvas is not visible.");
    const p = new DOMPoint(event.clientX, event.clientY).matrixTransform(matrix.inverse());
    return { x: p.x, y: p.y };
  };
  const release = () => {
    const id = drag?.pointerId;
    drag = null;
    if (id !== undefined && capture?.hasPointerCapture(id)) capture.releasePointerCapture(id);
    capture = null;
  };
  const cancel = () => { release(); props.onDraft(null); props.onBusy(false); };
  const commit = async (value: Transform2D, identity: string) => {
    if (identity !== props.identity || props.disabled) { cancel(); setError("Project changed during the gesture. Try again."); return; }
    setError("");
    setPending(true); props.onBusy(true);
    try {
      const result = await props.onCommit(value);
      if (result === null) setError("Group transform was not applied. Try again.");
    } catch { if (!disposed) setError("Group transform was not applied. Try again."); }
    finally { if (!disposed) { setPending(false); props.onDraft(null); props.onBusy(false); } }
  };
  const start = (event: PointerEvent, handle: GroupCanvasHandle) => {
    if (event.button !== 0 || drag || pending() || props.disabled) return;
    event.preventDefault(); event.stopPropagation();
    (event.currentTarget as HTMLElement).focus({ preventScroll: true });
    setError("");
    drag = { pointerId: event.pointerId, handle, start: point(event), transform: { ...props.transform }, identity: props.identity };
    capture = event.currentTarget as Element;
    capture.setPointerCapture(event.pointerId);
    props.onBusy(true);
  };
  const move = (event: PointerEvent) => {
    if (!drag || event.pointerId !== drag.pointerId) return;
    props.onDraft(dragGroupTransform(drag.transform, drag.handle, drag.start, point(event), event.shiftKey));
  };
  const finish = (event: PointerEvent) => {
    if (!drag || event.pointerId !== drag.pointerId) return;
    const current = drag;
    const value = dragGroupTransform(current.transform, current.handle, current.start, point(event), event.shiftKey);
    release();
    if (JSON.stringify(value) === JSON.stringify(current.transform)) { cancel(); return; }
    props.onDraft(value);
    void commit(value, current.identity);
  };
  const key = (event: KeyboardEvent) => {
    if (event.key === "Escape" && drag) { event.preventDefault(); event.stopPropagation(); cancel(); return; }
    if (!event.key.startsWith("Arrow")) return;
    event.preventDefault(); event.stopPropagation();
    if (drag || pending() || props.disabled || event.altKey || event.ctrlKey || event.metaKey) return;
    const value = nudgeGroupTransform(props.transform, event.key, event.shiftKey);
    if (JSON.stringify(value) !== JSON.stringify(props.transform)) { props.onDraft(value); void commit(value, props.identity); }
  };
  const refresh = () => {
    const generation = ++previewGeneration;
    setPreview(""); setPreviewError(false);
    if (previewInFlight) { previewQueued = true; return; }
    const loader = props.onLoadPreview;
    if (!loader) return;
    const width = Math.max(1, Math.round(512 * Math.min(1, props.aspect)));
    const height = Math.max(1, Math.min(512, Math.round(width / props.aspect)));
    previewInFlight = true;
    void loader(props.group.composition_id, width, height).then(url => {
      if (!disposed && generation === previewGeneration) setPreview(url);
    }).catch(() => { if (!disposed && generation === previewGeneration) setPreviewError(true); })
      .finally(() => {
        previewInFlight = false;
        if (!disposed && previewQueued) { previewQueued = false; refresh(); }
      });
  };
  const identity = createMemo(() => props.identity);
  createEffect(() => { identity(); untrack(refresh); });
  createEffect(() => {
    const identity = props.identity, disabled = props.disabled;
    if (drag && (drag.identity !== identity || disabled)) { cancel(); setError("Project changed during the gesture. Try again."); }
  });
  onCleanup(() => { disposed = true; previewGeneration++; cancel(); });
  onMount(() => {
    const measure = () => {
      const { width, height } = viewport.getBoundingClientRect();
      if (width === viewportSize().width && height === viewportSize().height) return;
      if (drag) cancel();
      setViewportSize({ width, height });
    };
    const observer = new ResizeObserver(measure);
    observer.observe(viewport); measure();
    onCleanup(() => observer.disconnect());
  });
  // Fit the committed transform; keep coordinates fixed throughout a drag.
  const extent = createMemo(() => {
    const bound = Math.max(0.6, ...[...groupCanvasCorners, "rotate" as const].flatMap(handle => {
      const p = groupCanvasHandlePoint(props.group.transform ?? props.transform, handle);
      return [Math.abs(p.x), Math.abs(p.y)];
    }));
    // Reserve 24 actual pixels on each edge, including at scale=3 on a phone.
    const minimumDimension = Math.max(49, Math.min(viewportSize().width, viewportSize().height));
    return bound / (1 - 48 / minimumDimension);
  });
  const position = (handle: GroupCanvasHandle) => {
    const p = groupCanvasHandlePoint(props.transform, handle);
    return { left: `${(p.x / extent() + 1) * 50}%`, top: `${(p.y / extent() + 1) * 50}%` };
  };
  return <section class="videoGroupCanvas" data-video-group-canvas={props.group.id} aria-label="Group canvas">
    <header><strong>Group canvas</strong><button type="button" disabled={!props.onLoadPreview || pending()} onClick={refresh}>Refresh group preview</button></header>
    <p>Drag inside to move; drag corners to scale; drag the round handle to rotate. Shift locks aspect or snaps rotation to 15°. Arrow keys nudge; Shift moves 10×; Escape cancels.</p>
    <div ref={viewport} class="videoGroupCanvasViewport" style={{ "aspect-ratio": String(props.aspect) }} onKeyDown={key}
      onPointerMove={move} onPointerUp={finish} onPointerCancel={event => { if (drag?.pointerId === event.pointerId) cancel(); }} onLostPointerCapture={event => {
        if (drag?.pointerId === event.pointerId && !capture?.hasPointerCapture(event.pointerId)) cancel();
      }}>
      <svg ref={svg} viewBox={`${-extent()} ${-extent()} ${extent() * 2} ${extent() * 2}`} preserveAspectRatio="none" aria-label="Group transform canvas">
        <rect class="videoGroupCanvasComposition" x="-0.5" y="-0.5" width="1" height="1" />
        <Show when={preview()}>{url => <image href={url()} x="-0.5" y="-0.5" width="1" height="1" preserveAspectRatio="none" />}</Show>
        <line class="videoGroupCanvasOutline" x1={groupCanvasHandlePoint(props.transform, "rotate").x} y1={groupCanvasHandlePoint(props.transform, "rotate").y}
          x2={(groupCanvasHandlePoint(props.transform, "nw").x + groupCanvasHandlePoint(props.transform, "ne").x) / 2} y2={(groupCanvasHandlePoint(props.transform, "nw").y + groupCanvasHandlePoint(props.transform, "ne").y) / 2} />
        <polygon class="videoGroupCanvasBox" points={groupCanvasCorners.map(h => { const p = groupCanvasHandlePoint(props.transform, h); return `${p.x},${p.y}`; }).join(" ")}
          role="button" tabindex="0" aria-label="Move group" aria-disabled={props.disabled || pending()} onPointerDown={event => start(event, "move")} />
      </svg>
      <For each={[...groupCanvasCorners, "rotate"] as const}>{handle => <button type="button" class="videoGroupCanvasHandle" data-group-handle={handle}
        style={position(handle)} aria-label={handle === "rotate" ? "Rotate group" : ({ nw: "Scale group from top left", ne: "Scale group from top right", se: "Scale group from bottom right", sw: "Scale group from bottom left" }[handle])}
        disabled={props.disabled || pending()} onPointerDown={event => start(event, handle)}><span /></button>}</For>
      <span class="videoGroupCanvasPivot" style={position("move")} aria-hidden="true">+</span>
    </div>
    <span>Composition bounds; preview refreshes after applying a transform.</span>
    <Show when={previewError()}><p role="status">Group preview unavailable. Refresh to try again.</p></Show>
    <Show when={error()}><p role="alert">{error()}</p></Show>
  </section>;
}
