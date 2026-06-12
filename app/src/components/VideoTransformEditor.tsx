import type { Transform2D } from "../types";

const transformViewBoxWidth = 100;
const transformViewBoxHeight = 60;
const transformFrame = {
  x: 5,
  y: 6,
  width: 90,
  height: 48,
};
const minScale = 0.05;
const maxScale = 3;

const finiteOr = (value: number, fallback: number) => (Number.isFinite(value) ? value : fallback);
const clampRange = (value: number, min: number, max: number) => Math.min(max, Math.max(min, value));
const roundedValue = (value: number, digits = 3) => Number(value.toFixed(digits));
const roundedRangeValue = (value: number, min: number, max: number, digits = 3) =>
  Number(clampRange(value, min, max).toFixed(digits));

const pointerPoint = (event: PointerEvent, svg: SVGSVGElement | null) => {
  if (!svg) {
    return null;
  }
  const rect = svg.getBoundingClientRect();
  if (rect.width <= 0 || rect.height <= 0) {
    return null;
  }
  return {
    x: ((event.clientX - rect.left) / rect.width) * transformViewBoxWidth,
    y: ((event.clientY - rect.top) / rect.height) * transformViewBoxHeight,
  };
};

const keyboardStep = (event: KeyboardEvent) => {
  if (event.shiftKey) {
    return 0.05;
  }
  if (event.altKey) {
    return 0.005;
  }
  return 0.02;
};

const normalizeRotationDeg = (value: number) => {
  let normalized = value;
  while (normalized > 180) {
    normalized -= 360;
  }
  while (normalized < -180) {
    normalized += 360;
  }
  return normalized;
};

const transformGeometry = (transform: Transform2D) => {
  const centerX = transformFrame.x + transformFrame.width / 2 + finiteOr(transform.x, 0) * transformFrame.width;
  const centerY = transformFrame.y + transformFrame.height / 2 + finiteOr(transform.y, 0) * transformFrame.height;
  const halfWidth = (transformFrame.width * clampRange(finiteOr(transform.scale_x, 1), minScale, maxScale)) / 2;
  const halfHeight = (transformFrame.height * clampRange(finiteOr(transform.scale_y, 1), minScale, maxScale)) / 2;
  const angle = finiteOr(transform.rotation_deg, 0) * (Math.PI / 180);
  const sin = Math.sin(angle);
  const cos = Math.cos(angle);
  const rotatePoint = (localX: number, localY: number) => ({
    x: centerX + localX * cos - localY * sin,
    y: centerY + localX * sin + localY * cos,
  });
  const corners = [
    rotatePoint(-halfWidth, -halfHeight),
    rotatePoint(halfWidth, -halfHeight),
    rotatePoint(halfWidth, halfHeight),
    rotatePoint(-halfWidth, halfHeight),
  ];
  const rightAnchor = rotatePoint(halfWidth, 0);
  const bottomAnchor = rotatePoint(0, halfHeight);
  const rotateAnchor = rotatePoint(0, -halfHeight);
  const rotateHandle = rotatePoint(0, -halfHeight - 7);
  return {
    centerX,
    centerY,
    halfWidth,
    halfHeight,
    angle,
    sin,
    cos,
    corners,
    rightAnchor,
    bottomAnchor,
    rotateAnchor,
    rotateHandle,
  };
};

const polygonPoints = (transform: Transform2D) =>
  transformGeometry(transform).corners.map((point) => `${point.x},${point.y}`).join(" ");

interface VideoTransformEditorProps {
  layerId: number;
  label: string;
  transform: Transform2D;
  onPatch: (patch: Partial<Transform2D>) => void;
}

export function VideoTransformEditor(props: VideoTransformEditorProps) {
  const setPositionFromPointer = (event: PointerEvent & { currentTarget: SVGCircleElement }) => {
    event.preventDefault();
    const pointer = pointerPoint(event, event.currentTarget.ownerSVGElement);
    if (!pointer) {
      return;
    }
    props.onPatch({
      x: roundedRangeValue((pointer.x - (transformFrame.x + transformFrame.width / 2)) / transformFrame.width, -1, 1),
      y: roundedRangeValue((pointer.y - (transformFrame.y + transformFrame.height / 2)) / transformFrame.height, -1, 1),
    });
  };

  const setScaleFromPointer = (event: PointerEvent & { currentTarget: SVGCircleElement }, axis: "x" | "y") => {
    event.preventDefault();
    const pointer = pointerPoint(event, event.currentTarget.ownerSVGElement);
    if (!pointer) {
      return;
    }
    const geometry = transformGeometry(props.transform);
    const dx = pointer.x - geometry.centerX;
    const dy = pointer.y - geometry.centerY;
    const localX = dx * geometry.cos + dy * geometry.sin;
    const localY = -dx * geometry.sin + dy * geometry.cos;
    props.onPatch({
      [axis === "x" ? "scale_x" : "scale_y"]: roundedRangeValue(
        (Math.abs(axis === "x" ? localX : localY) * 2) / (axis === "x" ? transformFrame.width : transformFrame.height),
        minScale,
        maxScale,
      ),
    } as Partial<Transform2D>);
  };

  const setRotationFromPointer = (event: PointerEvent & { currentTarget: SVGCircleElement }) => {
    event.preventDefault();
    const pointer = pointerPoint(event, event.currentTarget.ownerSVGElement);
    if (!pointer) {
      return;
    }
    const geometry = transformGeometry(props.transform);
    const angleDeg = (Math.atan2(pointer.y - geometry.centerY, pointer.x - geometry.centerX) * 180) / Math.PI;
    props.onPatch({ rotation_deg: roundedRangeValue(normalizeRotationDeg(angleDeg + 90), -180, 180, 1) });
  };

  const nudgePosition = (event: KeyboardEvent) => {
    if (event.key === "Home") {
      event.preventDefault();
      props.onPatch({ x: 0, y: 0 });
      return;
    }
    const step = keyboardStep(event);
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
    props.onPatch({
      x: roundedRangeValue(finiteOr(props.transform.x, 0) + dx, -1, 1),
      y: roundedRangeValue(finiteOr(props.transform.y, 0) + dy, -1, 1),
    });
  };

  const nudgeScale = (event: KeyboardEvent, axis: "x" | "y") => {
    const field = axis === "x" ? "scale_x" : "scale_y";
    if (event.key === "Home") {
      event.preventDefault();
      props.onPatch({ [field]: 1 } as Partial<Transform2D>);
      return;
    }
    const step = keyboardStep(event);
    const axisKey = axis === "x" ? ["ArrowLeft", "ArrowRight"] : ["ArrowUp", "ArrowDown"];
    if (!axisKey.includes(event.key)) {
      return;
    }
    event.preventDefault();
    const direction = event.key === "ArrowRight" || event.key === "ArrowDown" ? 1 : -1;
    props.onPatch({
      [field]: roundedRangeValue(finiteOr(props.transform[field], 1) + step * direction, minScale, maxScale),
    } as Partial<Transform2D>);
  };

  const nudgeRotation = (event: KeyboardEvent) => {
    if (event.key === "Home") {
      event.preventDefault();
      props.onPatch({ rotation_deg: 0 });
      return;
    }
    if (event.key !== "ArrowLeft" && event.key !== "ArrowRight") {
      return;
    }
    event.preventDefault();
    const step = event.shiftKey ? 5 : event.altKey ? 0.1 : 1;
    props.onPatch({
      rotation_deg: roundedRangeValue(
        normalizeRotationDeg(finiteOr(props.transform.rotation_deg, 0) + (event.key === "ArrowRight" ? step : -step)),
        -180,
        180,
        1,
      ),
    });
  };

  const geometry = () => transformGeometry(props.transform);
  const patternId = () => `video-transform-grid-${props.layerId}`;

  return (
    <div class="videoTransformEditor">
      <svg class="videoTransformSurface" viewBox={`0 0 ${transformViewBoxWidth} ${transformViewBoxHeight}`} role="img">
        <defs>
          <pattern id={patternId()} width="10" height="10" patternUnits="userSpaceOnUse">
            <path d="M 10 0 L 0 0 0 10" />
          </pattern>
        </defs>
        <rect class="videoTransformFrame" x={transformFrame.x} y={transformFrame.y} width={transformFrame.width} height={transformFrame.height} />
        <rect
          class="videoTransformGrid"
          x={transformFrame.x}
          y={transformFrame.y}
          width={transformFrame.width}
          height={transformFrame.height}
          fill={`url(#${patternId()})`}
        />
        <line class="videoTransformAxis" x1={transformFrame.x + transformFrame.width / 2} y1={transformFrame.y} x2={transformFrame.x + transformFrame.width / 2} y2={transformFrame.y + transformFrame.height} />
        <line class="videoTransformAxis" x1={transformFrame.x} y1={transformFrame.y + transformFrame.height / 2} x2={transformFrame.x + transformFrame.width} y2={transformFrame.y + transformFrame.height / 2} />
        <polygon class="videoTransformLayer" points={polygonPoints(props.transform)} />
        <line
          class="videoTransformGuide"
          x1={geometry().rightAnchor.x}
          y1={geometry().rightAnchor.y}
          x2={geometry().rightAnchor.x + (geometry().rightAnchor.x - geometry().centerX) * 0.12}
          y2={geometry().rightAnchor.y + (geometry().rightAnchor.y - geometry().centerY) * 0.12}
        />
        <line
          class="videoTransformGuide"
          x1={geometry().bottomAnchor.x}
          y1={geometry().bottomAnchor.y}
          x2={geometry().bottomAnchor.x + (geometry().bottomAnchor.x - geometry().centerX) * 0.12}
          y2={geometry().bottomAnchor.y + (geometry().bottomAnchor.y - geometry().centerY) * 0.12}
        />
        <line class="videoTransformRotateGuide" x1={geometry().rotateAnchor.x} y1={geometry().rotateAnchor.y} x2={geometry().rotateHandle.x} y2={geometry().rotateHandle.y} />
        <circle
          class="videoTransformMoveHandle"
          cx={geometry().centerX}
          cy={geometry().centerY}
          r="4"
          tabIndex={0}
          onPointerDown={(event) => {
            event.currentTarget.setPointerCapture(event.pointerId);
            setPositionFromPointer(event);
          }}
          onPointerMove={(event) => {
            if (event.buttons === 1) {
              setPositionFromPointer(event);
            }
          }}
          onPointerUp={(event) => event.currentTarget.releasePointerCapture(event.pointerId)}
          onPointerCancel={(event) => event.currentTarget.releasePointerCapture(event.pointerId)}
          onKeyDown={nudgePosition}
        >
          <title>Move {props.label}. Arrow keys nudge, Shift coarse, Alt fine, Home reset.</title>
        </circle>
        <circle
          class="videoTransformScaleHandle x"
          cx={geometry().rightAnchor.x + (geometry().rightAnchor.x - geometry().centerX) * 0.12}
          cy={geometry().rightAnchor.y + (geometry().rightAnchor.y - geometry().centerY) * 0.12}
          r="3.7"
          tabIndex={0}
          onPointerDown={(event) => {
            event.currentTarget.setPointerCapture(event.pointerId);
            setScaleFromPointer(event, "x");
          }}
          onPointerMove={(event) => {
            if (event.buttons === 1) {
              setScaleFromPointer(event, "x");
            }
          }}
          onPointerUp={(event) => event.currentTarget.releasePointerCapture(event.pointerId)}
          onPointerCancel={(event) => event.currentTarget.releasePointerCapture(event.pointerId)}
          onKeyDown={(event) => nudgeScale(event, "x")}
        >
          <title>Scale X for {props.label}. Arrow keys nudge, Shift coarse, Alt fine, Home reset.</title>
        </circle>
        <circle
          class="videoTransformScaleHandle y"
          cx={geometry().bottomAnchor.x + (geometry().bottomAnchor.x - geometry().centerX) * 0.12}
          cy={geometry().bottomAnchor.y + (geometry().bottomAnchor.y - geometry().centerY) * 0.12}
          r="3.7"
          tabIndex={0}
          onPointerDown={(event) => {
            event.currentTarget.setPointerCapture(event.pointerId);
            setScaleFromPointer(event, "y");
          }}
          onPointerMove={(event) => {
            if (event.buttons === 1) {
              setScaleFromPointer(event, "y");
            }
          }}
          onPointerUp={(event) => event.currentTarget.releasePointerCapture(event.pointerId)}
          onPointerCancel={(event) => event.currentTarget.releasePointerCapture(event.pointerId)}
          onKeyDown={(event) => nudgeScale(event, "y")}
        >
          <title>Scale Y for {props.label}. Arrow keys nudge, Shift coarse, Alt fine, Home reset.</title>
        </circle>
        <circle
          class="videoTransformRotateHandle"
          cx={geometry().rotateHandle.x}
          cy={geometry().rotateHandle.y}
          r="3.7"
          tabIndex={0}
          onPointerDown={(event) => {
            event.currentTarget.setPointerCapture(event.pointerId);
            setRotationFromPointer(event);
          }}
          onPointerMove={(event) => {
            if (event.buttons === 1) {
              setRotationFromPointer(event);
            }
          }}
          onPointerUp={(event) => event.currentTarget.releasePointerCapture(event.pointerId)}
          onPointerCancel={(event) => event.currentTarget.releasePointerCapture(event.pointerId)}
          onKeyDown={nudgeRotation}
        >
          <title>Rotate {props.label}. Arrow keys nudge, Shift coarse, Alt fine, Home reset.</title>
        </circle>
      </svg>
      <div class="videoTransformReadout">
        <span>X {roundedValue(finiteOr(props.transform.x, 0), 2).toFixed(2)}</span>
        <span>Y {roundedValue(finiteOr(props.transform.y, 0), 2).toFixed(2)}</span>
        <span>S {roundedValue(finiteOr(props.transform.scale_x, 1), 2).toFixed(2)}x/{roundedValue(finiteOr(props.transform.scale_y, 1), 2).toFixed(2)}x</span>
        <span>R {roundedValue(finiteOr(props.transform.rotation_deg, 0), 1).toFixed(1)}</span>
      </div>
    </div>
  );
}
