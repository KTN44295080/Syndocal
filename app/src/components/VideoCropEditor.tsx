import type { Transform2D } from "../types";

const cropViewBoxWidth = 100;
const cropViewBoxHeight = 60;
const cropFrame = {
  x: 5,
  y: 6,
  width: 90,
  height: 48,
};
const minCropSpan = 0.03;

const finiteOr = (value: number, fallback: number) => (Number.isFinite(value) ? value : fallback);
const clampRange = (value: number, min: number, max: number) => Math.min(max, Math.max(min, value));
const roundedCropValue = (value: number) => Number(clampRange(value, 0, 1).toFixed(3));

const cropPointerPoint = (event: PointerEvent, svg: SVGSVGElement | null) => {
  if (!svg) {
    return null;
  }
  const rect = svg.getBoundingClientRect();
  if (rect.width <= 0 || rect.height <= 0) {
    return null;
  }
  return {
    x: ((event.clientX - rect.left) / rect.width) * cropViewBoxWidth,
    y: ((event.clientY - rect.top) / rect.height) * cropViewBoxHeight,
  };
};

const cropArrowStep = (event: KeyboardEvent) => {
  if (event.shiftKey) {
    return 0.05;
  }
  if (event.altKey) {
    return 0.005;
  }
  return 0.02;
};

const cropEdges = (transform: Transform2D) => {
  const left = clampRange(finiteOr(transform.crop_left, 0), 0, 1);
  const top = clampRange(finiteOr(transform.crop_top, 0), 0, 1);
  const right = clampRange(finiteOr(transform.crop_right, 0), 0, 1);
  const bottom = clampRange(finiteOr(transform.crop_bottom, 0), 0, 1);
  const maxLeft = Math.max(0, 1 - right - minCropSpan);
  const maxTop = Math.max(0, 1 - bottom - minCropSpan);
  return {
    left: clampRange(left, 0, maxLeft),
    top: clampRange(top, 0, maxTop),
    right: clampRange(right, 0, Math.max(0, 1 - left - minCropSpan)),
    bottom: clampRange(bottom, 0, Math.max(0, 1 - top - minCropSpan)),
  };
};

const cropRect = (transform: Transform2D) => {
  const edges = cropEdges(transform);
  const x = cropFrame.x + edges.left * cropFrame.width;
  const y = cropFrame.y + edges.top * cropFrame.height;
  const width = Math.max(1, cropFrame.width * (1 - edges.left - edges.right));
  const height = Math.max(1, cropFrame.height * (1 - edges.top - edges.bottom));
  return {
    ...edges,
    x,
    y,
    width,
    height,
    centerX: x + width / 2,
    centerY: y + height / 2,
  };
};

interface VideoCropEditorProps {
  layerId: number;
  label: string;
  transform: Transform2D;
  onPatch: (patch: Partial<Transform2D>) => void;
}

export function VideoCropEditor(props: VideoCropEditorProps) {
  const setCropEdgeFromPointer = (
    event: PointerEvent & { currentTarget: SVGCircleElement },
    edge: "left" | "right" | "top" | "bottom",
  ) => {
    event.preventDefault();
    const pointer = cropPointerPoint(event, event.currentTarget.ownerSVGElement);
    if (!pointer) {
      return;
    }
    const current = cropEdges(props.transform);
    if (edge === "left") {
      props.onPatch({
        crop_left: roundedCropValue(clampRange((pointer.x - cropFrame.x) / cropFrame.width, 0, 1 - current.right - minCropSpan)),
      });
      return;
    }
    if (edge === "right") {
      props.onPatch({
        crop_right: roundedCropValue(clampRange((cropFrame.x + cropFrame.width - pointer.x) / cropFrame.width, 0, 1 - current.left - minCropSpan)),
      });
      return;
    }
    if (edge === "top") {
      props.onPatch({
        crop_top: roundedCropValue(clampRange((pointer.y - cropFrame.y) / cropFrame.height, 0, 1 - current.bottom - minCropSpan)),
      });
      return;
    }
    props.onPatch({
      crop_bottom: roundedCropValue(clampRange((cropFrame.y + cropFrame.height - pointer.y) / cropFrame.height, 0, 1 - current.top - minCropSpan)),
    });
  };

  const setCropWindowFromPointer = (event: PointerEvent & { currentTarget: SVGCircleElement }) => {
    event.preventDefault();
    const pointer = cropPointerPoint(event, event.currentTarget.ownerSVGElement);
    if (!pointer) {
      return;
    }
    const current = cropEdges(props.transform);
    const width = Math.max(minCropSpan, 1 - current.left - current.right);
    const height = Math.max(minCropSpan, 1 - current.top - current.bottom);
    const centerX = clampRange((pointer.x - cropFrame.x) / cropFrame.width, width / 2, 1 - width / 2);
    const centerY = clampRange((pointer.y - cropFrame.y) / cropFrame.height, height / 2, 1 - height / 2);
    props.onPatch({
      crop_left: roundedCropValue(centerX - width / 2),
      crop_right: roundedCropValue(1 - centerX - width / 2),
      crop_top: roundedCropValue(centerY - height / 2),
      crop_bottom: roundedCropValue(1 - centerY - height / 2),
    });
  };

  const nudgeCropEdgeFromKeyboard = (event: KeyboardEvent, edge: "left" | "right" | "top" | "bottom") => {
    if (event.key === "Home") {
      event.preventDefault();
      props.onPatch({
        [edge === "left" ? "crop_left" : edge === "right" ? "crop_right" : edge === "top" ? "crop_top" : "crop_bottom"]: 0,
      } as Partial<Transform2D>);
      return;
    }
    const current = cropEdges(props.transform);
    const step = cropArrowStep(event);
    if (edge === "left" && (event.key === "ArrowLeft" || event.key === "ArrowRight")) {
      event.preventDefault();
      props.onPatch({
        crop_left: roundedCropValue(clampRange(current.left + (event.key === "ArrowRight" ? step : -step), 0, 1 - current.right - minCropSpan)),
      });
      return;
    }
    if (edge === "right" && (event.key === "ArrowLeft" || event.key === "ArrowRight")) {
      event.preventDefault();
      props.onPatch({
        crop_right: roundedCropValue(clampRange(current.right + (event.key === "ArrowLeft" ? step : -step), 0, 1 - current.left - minCropSpan)),
      });
      return;
    }
    if (edge === "top" && (event.key === "ArrowUp" || event.key === "ArrowDown")) {
      event.preventDefault();
      props.onPatch({
        crop_top: roundedCropValue(clampRange(current.top + (event.key === "ArrowDown" ? step : -step), 0, 1 - current.bottom - minCropSpan)),
      });
      return;
    }
    if (edge === "bottom" && (event.key === "ArrowUp" || event.key === "ArrowDown")) {
      event.preventDefault();
      props.onPatch({
        crop_bottom: roundedCropValue(clampRange(current.bottom + (event.key === "ArrowUp" ? step : -step), 0, 1 - current.top - minCropSpan)),
      });
    }
  };

  const nudgeCropWindowFromKeyboard = (event: KeyboardEvent) => {
    if (event.key === "Home") {
      event.preventDefault();
      props.onPatch({ crop_left: 0, crop_top: 0, crop_right: 0, crop_bottom: 0 });
      return;
    }
    const current = cropEdges(props.transform);
    const width = Math.max(minCropSpan, 1 - current.left - current.right);
    const height = Math.max(minCropSpan, 1 - current.top - current.bottom);
    const step = cropArrowStep(event);
    let dx = 0;
    let dy = 0;
    if (event.key === "ArrowLeft") {
      dx = -step;
    } else if (event.key === "ArrowRight") {
      dx = step;
    } else if (event.key === "ArrowUp") {
      dy = -step;
    } else if (event.key === "ArrowDown") {
      dy = step;
    } else {
      return;
    }
    event.preventDefault();
    const left = clampRange(current.left + dx, 0, 1 - width);
    const top = clampRange(current.top + dy, 0, 1 - height);
    props.onPatch({
      crop_left: roundedCropValue(left),
      crop_right: roundedCropValue(1 - left - width),
      crop_top: roundedCropValue(top),
      crop_bottom: roundedCropValue(1 - top - height),
    });
  };

  const rect = () => cropRect(props.transform);
  const patternId = () => `video-crop-grid-${props.layerId}`;

  return (
    <div class="videoCropEditor">
      <svg class="videoCropSurface" viewBox={`0 0 ${cropViewBoxWidth} ${cropViewBoxHeight}`} role="img">
        <defs>
          <pattern id={patternId()} width="10" height="10" patternUnits="userSpaceOnUse">
            <path d="M 10 0 L 0 0 0 10" />
          </pattern>
        </defs>
        <rect class="videoCropSource" x={cropFrame.x} y={cropFrame.y} width={cropFrame.width} height={cropFrame.height} />
        <rect
          class="videoCropGrid"
          x={cropFrame.x}
          y={cropFrame.y}
          width={cropFrame.width}
          height={cropFrame.height}
          fill={`url(#${patternId()})`}
        />
        <rect class="videoCropMask" x={cropFrame.x} y={cropFrame.y} width={cropFrame.width} height={rect().y - cropFrame.y} />
        <rect
          class="videoCropMask"
          x={cropFrame.x}
          y={rect().y + rect().height}
          width={cropFrame.width}
          height={cropFrame.y + cropFrame.height - rect().y - rect().height}
        />
        <rect class="videoCropMask" x={cropFrame.x} y={rect().y} width={rect().x - cropFrame.x} height={rect().height} />
        <rect
          class="videoCropMask"
          x={rect().x + rect().width}
          y={rect().y}
          width={cropFrame.x + cropFrame.width - rect().x - rect().width}
          height={rect().height}
        />
        <rect class="videoCropWindow" x={rect().x} y={rect().y} width={rect().width} height={rect().height} />
        <circle
          class="videoCropMoveHandle"
          cx={rect().centerX}
          cy={rect().centerY}
          r="4"
          tabIndex={0}
          onPointerDown={(event) => {
            event.currentTarget.setPointerCapture(event.pointerId);
            setCropWindowFromPointer(event);
          }}
          onPointerMove={(event) => {
            if (event.buttons === 1) {
              setCropWindowFromPointer(event);
            }
          }}
          onPointerUp={(event) => event.currentTarget.releasePointerCapture(event.pointerId)}
          onPointerCancel={(event) => event.currentTarget.releasePointerCapture(event.pointerId)}
          onKeyDown={nudgeCropWindowFromKeyboard}
        >
          <title>Move crop window for {props.label}. Arrow keys nudge, Shift coarse, Alt fine, Home reset.</title>
        </circle>
        <circle
          class="videoCropEdgeHandle left"
          cx={rect().x}
          cy={rect().centerY}
          r="3.5"
          tabIndex={0}
          onPointerDown={(event) => {
            event.currentTarget.setPointerCapture(event.pointerId);
            setCropEdgeFromPointer(event, "left");
          }}
          onPointerMove={(event) => {
            if (event.buttons === 1) {
              setCropEdgeFromPointer(event, "left");
            }
          }}
          onPointerUp={(event) => event.currentTarget.releasePointerCapture(event.pointerId)}
          onPointerCancel={(event) => event.currentTarget.releasePointerCapture(event.pointerId)}
          onKeyDown={(event) => nudgeCropEdgeFromKeyboard(event, "left")}
        >
          <title>Crop left for {props.label}. Arrow keys nudge, Shift coarse, Alt fine, Home reset.</title>
        </circle>
        <circle
          class="videoCropEdgeHandle right"
          cx={rect().x + rect().width}
          cy={rect().centerY}
          r="3.5"
          tabIndex={0}
          onPointerDown={(event) => {
            event.currentTarget.setPointerCapture(event.pointerId);
            setCropEdgeFromPointer(event, "right");
          }}
          onPointerMove={(event) => {
            if (event.buttons === 1) {
              setCropEdgeFromPointer(event, "right");
            }
          }}
          onPointerUp={(event) => event.currentTarget.releasePointerCapture(event.pointerId)}
          onPointerCancel={(event) => event.currentTarget.releasePointerCapture(event.pointerId)}
          onKeyDown={(event) => nudgeCropEdgeFromKeyboard(event, "right")}
        >
          <title>Crop right for {props.label}. Arrow keys nudge, Shift coarse, Alt fine, Home reset.</title>
        </circle>
        <circle
          class="videoCropEdgeHandle top"
          cx={rect().centerX}
          cy={rect().y}
          r="3.5"
          tabIndex={0}
          onPointerDown={(event) => {
            event.currentTarget.setPointerCapture(event.pointerId);
            setCropEdgeFromPointer(event, "top");
          }}
          onPointerMove={(event) => {
            if (event.buttons === 1) {
              setCropEdgeFromPointer(event, "top");
            }
          }}
          onPointerUp={(event) => event.currentTarget.releasePointerCapture(event.pointerId)}
          onPointerCancel={(event) => event.currentTarget.releasePointerCapture(event.pointerId)}
          onKeyDown={(event) => nudgeCropEdgeFromKeyboard(event, "top")}
        >
          <title>Crop top for {props.label}. Arrow keys nudge, Shift coarse, Alt fine, Home reset.</title>
        </circle>
        <circle
          class="videoCropEdgeHandle bottom"
          cx={rect().centerX}
          cy={rect().y + rect().height}
          r="3.5"
          tabIndex={0}
          onPointerDown={(event) => {
            event.currentTarget.setPointerCapture(event.pointerId);
            setCropEdgeFromPointer(event, "bottom");
          }}
          onPointerMove={(event) => {
            if (event.buttons === 1) {
              setCropEdgeFromPointer(event, "bottom");
            }
          }}
          onPointerUp={(event) => event.currentTarget.releasePointerCapture(event.pointerId)}
          onPointerCancel={(event) => event.currentTarget.releasePointerCapture(event.pointerId)}
          onKeyDown={(event) => nudgeCropEdgeFromKeyboard(event, "bottom")}
        >
          <title>Crop bottom for {props.label}. Arrow keys nudge, Shift coarse, Alt fine, Home reset.</title>
        </circle>
      </svg>
      <div class="videoCropReadout">
        <span>L {rect().left.toFixed(2)}</span>
        <span>R {rect().right.toFixed(2)}</span>
        <span>T {rect().top.toFixed(2)}</span>
        <span>B {rect().bottom.toFixed(2)}</span>
      </div>
    </div>
  );
}
