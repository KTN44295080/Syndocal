import { For } from "solid-js";
import type { VideoOutputMapping } from "../types";
import { videoOutputAspectPresets } from "../videoOutputMapping";

type NumericVideoOutputMappingField = Exclude<keyof VideoOutputMapping, "aspect_mode">;

export const projectorMapViewBoxSize = 100;

const projectorCornerGain = 18;
const projectorKeystoneGain = 24;
const projectorScaleHandleGap = 7;
const projectorRotateHandleGap = 10;

const finiteOr = (value: number, fallback: number) => (Number.isFinite(value) ? value : fallback);
const clampRange = (value: number, min: number, max: number) => Math.min(max, Math.max(min, value));
const roundedMappingValue = (value: number) => Number(clampRange(value, -1, 1).toFixed(3));
const roundedRangeValue = (value: number, min: number, max: number, digits = 3) =>
  Number(clampRange(value, min, max).toFixed(digits));

const projectorCorners = [
  {
    key: "topLeft",
    label: "TL",
    baseX: -1,
    baseY: -1,
    xField: "corner_top_left_x",
    yField: "corner_top_left_y",
  },
  {
    key: "topRight",
    label: "TR",
    baseX: 1,
    baseY: -1,
    xField: "corner_top_right_x",
    yField: "corner_top_right_y",
  },
  {
    key: "bottomRight",
    label: "BR",
    baseX: 1,
    baseY: 1,
    xField: "corner_bottom_right_x",
    yField: "corner_bottom_right_y",
  },
  {
    key: "bottomLeft",
    label: "BL",
    baseX: -1,
    baseY: 1,
    xField: "corner_bottom_left_x",
    yField: "corner_bottom_left_y",
  },
] as const;

type ProjectorCorner = (typeof projectorCorners)[number];
type ProjectorKeystoneAxis = "x" | "y";
type ProjectorScaleAxis = "x" | "y";

const projectorKeystoneHandles: { axis: ProjectorKeystoneAxis; label: string; baseX: number; baseY: number }[] = [
  { axis: "x", label: "Key H", baseX: 50, baseY: 10 },
  { axis: "y", label: "Key V", baseX: 90, baseY: 50 },
];

const projectorScaleHandles: { axis: ProjectorScaleAxis; label: string }[] = [
  { axis: "x", label: "Scale X" },
  { axis: "y", label: "Scale Y" },
];

const projectorKeystonePresets = [
  { label: "Flat", keystoneX: 0, keystoneY: 0 },
  { label: "H -", keystoneX: -0.35, keystoneY: 0 },
  { label: "H +", keystoneX: 0.35, keystoneY: 0 },
  { label: "V -", keystoneX: 0, keystoneY: -0.35 },
  { label: "V +", keystoneX: 0, keystoneY: 0.35 },
] as const;

const mappingNumber = (mapping: VideoOutputMapping, field: NumericVideoOutputMappingField, fallback: number) =>
  finiteOr(mapping[field], fallback);

const projectorAspectScales = (mapping: VideoOutputMapping) => {
  const aspect = clampRange(finiteOr(mapping.aspect_ratio, 1), 0.25, 4);
  return {
    x: aspect >= 1 ? 1 : aspect,
    y: aspect >= 1 ? 1 / Math.min(aspect, 2.8) : 1,
  };
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

const projectorMapBasePoint = (mapping: VideoOutputMapping, corner: ProjectorCorner) => {
  const aspectScale = projectorAspectScales(mapping);
  const scaleX = clampRange(finiteOr(mapping.scale_x, 1), 0.25, 2.5);
  const scaleY = clampRange(finiteOr(mapping.scale_y, 1), 0.25, 2.5);
  const radius = 30;
  const localX =
    corner.baseX * radius * aspectScale.x * scaleX +
    clampRange(finiteOr(mapping.offset_x, 0), -1, 1) * 20 +
    clampRange(finiteOr(mapping.keystone_x, 0), -1, 1) * corner.baseY * 12;
  const localY =
    corner.baseY * radius * aspectScale.y * scaleY +
    clampRange(finiteOr(mapping.offset_y, 0), -1, 1) * 20 +
    clampRange(finiteOr(mapping.keystone_y, 0), -1, 1) * corner.baseX * 12;
  const rotation = finiteOr(mapping.rotation_deg, 0) * (Math.PI / 180);
  const sin = Math.sin(rotation);
  const cos = Math.cos(rotation);
  return {
    x: projectorMapViewBoxSize / 2 + localX * cos - localY * sin,
    y: projectorMapViewBoxSize / 2 + localX * sin + localY * cos,
  };
};

const projectorMapPoint = (mapping: VideoOutputMapping, corner: ProjectorCorner) => {
  const base = projectorMapBasePoint(mapping, corner);
  return {
    x: base.x + clampRange(finiteOr(mapping[corner.xField], 0), -1, 1) * projectorCornerGain,
    y: base.y + clampRange(finiteOr(mapping[corner.yField], 0), -1, 1) * projectorCornerGain,
  };
};

const projectorMapPointList = (mapping: VideoOutputMapping) => projectorCorners.map((corner) => projectorMapPoint(mapping, corner));
const projectorMapBasePointList = (mapping: VideoOutputMapping) =>
  projectorCorners.map((corner) => projectorMapBasePoint(mapping, corner));

const pointAverage = (points: { x: number; y: number }[]) => ({
  x: points.reduce((sum, point) => sum + point.x, 0) / points.length,
  y: points.reduce((sum, point) => sum + point.y, 0) / points.length,
});

const unitVector = (from: { x: number; y: number }, to: { x: number; y: number }) => {
  const dx = to.x - from.x;
  const dy = to.y - from.y;
  const length = Math.hypot(dx, dy) || 1;
  return { x: dx / length, y: dy / length };
};

const projectorMapCenter = (mapping: VideoOutputMapping) => {
  const points = projectorMapPointList(mapping);
  return pointAverage(points);
};

const projectorMapBaseCenter = (mapping: VideoOutputMapping) => pointAverage(projectorMapBasePointList(mapping));

const projectorScaleAnchorPoint = (mapping: VideoOutputMapping, axis: ProjectorScaleAxis) => {
  const points = projectorMapBasePointList(mapping);
  return axis === "x" ? pointAverage([points[1], points[2]]) : pointAverage([points[2], points[3]]);
};

const projectorScaleHandlePoint = (mapping: VideoOutputMapping, axis: ProjectorScaleAxis) => {
  const center = projectorMapBaseCenter(mapping);
  const anchor = projectorScaleAnchorPoint(mapping, axis);
  const direction = unitVector(center, anchor);
  return {
    x: anchor.x + direction.x * projectorScaleHandleGap,
    y: anchor.y + direction.y * projectorScaleHandleGap,
  };
};

const projectorRotateAnchorPoint = (mapping: VideoOutputMapping) => {
  const points = projectorMapBasePointList(mapping);
  return pointAverage([points[0], points[1]]);
};

const projectorRotateHandlePoint = (mapping: VideoOutputMapping) => {
  const center = projectorMapBaseCenter(mapping);
  const anchor = projectorRotateAnchorPoint(mapping);
  const direction = unitVector(center, anchor);
  return {
    x: anchor.x + direction.x * projectorRotateHandleGap,
    y: anchor.y + direction.y * projectorRotateHandleGap,
  };
};

const projectorKeystoneHandlePoint = (mapping: VideoOutputMapping, axis: ProjectorKeystoneAxis) =>
  axis === "x"
    ? {
        x: 50 + clampRange(finiteOr(mapping.keystone_x, 0), -1, 1) * projectorKeystoneGain,
        y: 10,
      }
    : {
        x: 90,
        y: 50 + clampRange(finiteOr(mapping.keystone_y, 0), -1, 1) * projectorKeystoneGain,
      };

export const projectorMapPoints = (mapping: VideoOutputMapping) =>
  projectorMapPointList(mapping).map((point) => `${point.x},${point.y}`);

export const projectorMapBasePoints = (mapping: VideoOutputMapping) =>
  projectorCorners.map((corner) => {
    const point = projectorMapBasePoint(mapping, corner);
    return `${point.x},${point.y}`;
  });

const projectorKeystonePresetPoints = (keystoneX: number, keystoneY: number) =>
  projectorCorners
    .map((corner) => {
      const x = 20 + corner.baseX * 12 + keystoneX * corner.baseY * 5;
      const y = 12 + corner.baseY * 8 + keystoneY * corner.baseX * 4;
      return `${x},${y}`;
    })
    .join(" ");

const projectorMapPointerPoint = (event: PointerEvent, svg: SVGSVGElement | null) => {
  if (!svg) {
    return null;
  }
  const rect = svg.getBoundingClientRect();
  return {
    x: ((event.clientX - rect.left) / rect.width) * projectorMapViewBoxSize,
    y: ((event.clientY - rect.top) / rect.height) * projectorMapViewBoxSize,
  };
};

const projectorMapKeyboardStep = (event: KeyboardEvent) => {
  if (event.shiftKey) {
    return 0.05;
  }
  if (event.altKey) {
    return 0.005;
  }
  return 0.02;
};

const projectorMapArrowDelta = (event: KeyboardEvent) => {
  const step = projectorMapKeyboardStep(event);
  switch (event.key) {
    case "ArrowLeft":
      return { x: -step, y: 0 };
    case "ArrowRight":
      return { x: step, y: 0 };
    case "ArrowUp":
      return { x: 0, y: -step };
    case "ArrowDown":
      return { x: 0, y: step };
    default:
      return null;
  }
};

interface ProjectorMapEditorProps {
  mapping: VideoOutputMapping;
  outputId: number;
  label: string;
  onPatch: (patch: Partial<VideoOutputMapping>) => void;
}

interface ProjectorMapPreviewProps {
  mapping: VideoOutputMapping;
  outputId: number;
  class?: string;
}

export function ProjectorMapPreview(props: ProjectorMapPreviewProps) {
  return (
    <svg
      class={props.class ?? "projectorMapSurface"}
      viewBox={`0 0 ${projectorMapViewBoxSize} ${projectorMapViewBoxSize}`}
      role="img"
    >
      <defs>
        <pattern id={`projector-map-preview-grid-${props.outputId}`} width="10" height="10" patternUnits="userSpaceOnUse">
          <path d="M 10 0 L 0 0 0 10" />
        </pattern>
      </defs>
      <rect class="projectorMapFloor" x="0" y="0" width="100" height="100" />
      <rect
        class="projectorMapGrid"
        x="0"
        y="0"
        width="100"
        height="100"
        fill={`url(#projector-map-preview-grid-${props.outputId})`}
      />
      <line class="projectorMapAxis" x1="50" y1="0" x2="50" y2="100" />
      <line class="projectorMapAxis" x1="0" y1="50" x2="100" y2="50" />
      <polygon class="projectorMapBase" points={projectorMapBasePoints(props.mapping).join(" ")} />
      <polygon class="projectorMapWarp" points={projectorMapPoints(props.mapping).join(" ")} />
    </svg>
  );
}

export function ProjectorMapEditor(props: ProjectorMapEditorProps) {
  const patchNumber = (field: NumericVideoOutputMappingField, value: number, min: number, max: number, digits = 3) => {
    props.onPatch({ [field]: roundedRangeValue(value, min, max, digits) } as Partial<VideoOutputMapping>);
  };

  const resetCornerOffset = (corner: ProjectorCorner) => {
    props.onPatch({ [corner.xField]: 0, [corner.yField]: 0 } as Partial<VideoOutputMapping>);
  };

  const setCornerFromPointer = (event: PointerEvent & { currentTarget: SVGCircleElement }, corner: ProjectorCorner) => {
    event.preventDefault();
    const pointer = projectorMapPointerPoint(event, event.currentTarget.ownerSVGElement);
    if (!pointer) {
      return;
    }
    const base = projectorMapBasePoint(props.mapping, corner);
    props.onPatch({
      [corner.xField]: roundedMappingValue((pointer.x - base.x) / projectorCornerGain),
      [corner.yField]: roundedMappingValue((pointer.y - base.y) / projectorCornerGain),
    } as Partial<VideoOutputMapping>);
  };

  const setOffsetFromPointer = (event: PointerEvent & { currentTarget: SVGCircleElement }) => {
    event.preventDefault();
    const pointer = projectorMapPointerPoint(event, event.currentTarget.ownerSVGElement);
    if (!pointer) {
      return;
    }
    const center = projectorMapCenter(props.mapping);
    const deltaX = pointer.x - center.x;
    const deltaY = pointer.y - center.y;
    const rotation = finiteOr(props.mapping.rotation_deg, 0) * (Math.PI / 180);
    const cos = Math.cos(rotation);
    const sin = Math.sin(rotation);
    const localX = deltaX * cos + deltaY * sin;
    const localY = -deltaX * sin + deltaY * cos;
    props.onPatch({
      offset_x: roundedMappingValue(finiteOr(props.mapping.offset_x, 0) + localX / 20),
      offset_y: roundedMappingValue(finiteOr(props.mapping.offset_y, 0) + localY / 20),
    });
  };

  const setKeystoneFromPointer = (
    event: PointerEvent & { currentTarget: SVGCircleElement },
    axis: ProjectorKeystoneAxis,
  ) => {
    event.preventDefault();
    const pointer = projectorMapPointerPoint(event, event.currentTarget.ownerSVGElement);
    if (!pointer) {
      return;
    }
    props.onPatch({
      [axis === "x" ? "keystone_x" : "keystone_y"]: roundedMappingValue(
        ((axis === "x" ? pointer.x : pointer.y) - 50) / projectorKeystoneGain,
      ),
    } as Partial<VideoOutputMapping>);
  };

  const setScaleFromPointer = (event: PointerEvent & { currentTarget: SVGCircleElement }, axis: ProjectorScaleAxis) => {
    event.preventDefault();
    const pointer = projectorMapPointerPoint(event, event.currentTarget.ownerSVGElement);
    if (!pointer) {
      return;
    }
    const center = projectorMapBaseCenter(props.mapping);
    const deltaX = pointer.x - center.x;
    const deltaY = pointer.y - center.y;
    const rotation = finiteOr(props.mapping.rotation_deg, 0) * (Math.PI / 180);
    const cos = Math.cos(rotation);
    const sin = Math.sin(rotation);
    const localX = deltaX * cos + deltaY * sin;
    const localY = -deltaX * sin + deltaY * cos;
    const aspectScale = projectorAspectScales(props.mapping);
    const denominator = 30 * (axis === "x" ? aspectScale.x : aspectScale.y);
    const nextScale = Math.abs(axis === "x" ? localX : localY) / denominator;
    props.onPatch({
      [axis === "x" ? "scale_x" : "scale_y"]: roundedRangeValue(nextScale, 0.25, 2.5),
    } as Partial<VideoOutputMapping>);
  };

  const setRotationFromPointer = (event: PointerEvent & { currentTarget: SVGCircleElement }) => {
    event.preventDefault();
    const pointer = projectorMapPointerPoint(event, event.currentTarget.ownerSVGElement);
    if (!pointer) {
      return;
    }
    const center = projectorMapBaseCenter(props.mapping);
    const angleDeg = (Math.atan2(pointer.y - center.y, pointer.x - center.x) * 180) / Math.PI;
    props.onPatch({
      rotation_deg: roundedRangeValue(normalizeRotationDeg(angleDeg + 90), -180, 180, 1),
    });
  };

  const nudgeCornerFromKeyboard = (event: KeyboardEvent, corner: ProjectorCorner) => {
    if (event.key === "Home") {
      event.preventDefault();
      props.onPatch({ [corner.xField]: 0, [corner.yField]: 0 } as Partial<VideoOutputMapping>);
      return;
    }
    const delta = projectorMapArrowDelta(event);
    if (!delta) {
      return;
    }
    event.preventDefault();
    props.onPatch({
      [corner.xField]: roundedMappingValue(mappingNumber(props.mapping, corner.xField, 0) + delta.x),
      [corner.yField]: roundedMappingValue(mappingNumber(props.mapping, corner.yField, 0) + delta.y),
    } as Partial<VideoOutputMapping>);
  };

  const nudgeOffsetFromKeyboard = (event: KeyboardEvent) => {
    if (event.key === "Home") {
      event.preventDefault();
      props.onPatch({ offset_x: 0, offset_y: 0 });
      return;
    }
    const delta = projectorMapArrowDelta(event);
    if (!delta) {
      return;
    }
    event.preventDefault();
    props.onPatch({
      offset_x: roundedMappingValue(mappingNumber(props.mapping, "offset_x", 0) + delta.x),
      offset_y: roundedMappingValue(mappingNumber(props.mapping, "offset_y", 0) + delta.y),
    });
  };

  const nudgeKeystoneFromKeyboard = (event: KeyboardEvent, axis: ProjectorKeystoneAxis) => {
    const field = axis === "x" ? "keystone_x" : "keystone_y";
    if (event.key === "Home") {
      event.preventDefault();
      props.onPatch({ [field]: 0 } as Partial<VideoOutputMapping>);
      return;
    }
    const delta = projectorMapArrowDelta(event);
    const axisDelta = axis === "x" ? delta?.x : delta?.y;
    if (!axisDelta) {
      return;
    }
    event.preventDefault();
    props.onPatch({
      [field]: roundedMappingValue(mappingNumber(props.mapping, field, 0) + axisDelta),
    } as Partial<VideoOutputMapping>);
  };

  const nudgeScaleFromKeyboard = (event: KeyboardEvent, axis: ProjectorScaleAxis) => {
    const field = axis === "x" ? "scale_x" : "scale_y";
    if (event.key === "Home") {
      event.preventDefault();
      props.onPatch({ [field]: 1 } as Partial<VideoOutputMapping>);
      return;
    }
    const delta = projectorMapArrowDelta(event);
    const axisDelta = axis === "x" ? delta?.x : delta?.y;
    if (!axisDelta) {
      return;
    }
    event.preventDefault();
    props.onPatch({
      [field]: roundedRangeValue(mappingNumber(props.mapping, field, 1) + axisDelta, 0.25, 2.5),
    } as Partial<VideoOutputMapping>);
  };

  const nudgeRotationFromKeyboard = (event: KeyboardEvent) => {
    if (event.key === "Home") {
      event.preventDefault();
      props.onPatch({ rotation_deg: 0 });
      return;
    }
    const step = event.shiftKey ? 5 : event.altKey ? 0.1 : 1;
    if (event.key !== "ArrowLeft" && event.key !== "ArrowRight") {
      return;
    }
    event.preventDefault();
    props.onPatch({
      rotation_deg: roundedRangeValue(
        normalizeRotationDeg(mappingNumber(props.mapping, "rotation_deg", 0) + (event.key === "ArrowRight" ? step : -step)),
        -180,
        180,
        1,
      ),
    });
  };

  return (
    <div class="projectorMapEditor">
      <svg
        class="projectorMapSurface"
        viewBox={`0 0 ${projectorMapViewBoxSize} ${projectorMapViewBoxSize}`}
        role="img"
      >
        <defs>
          <pattern id={`projector-map-grid-${props.outputId}`} width="10" height="10" patternUnits="userSpaceOnUse">
            <path d="M 10 0 L 0 0 0 10" />
          </pattern>
        </defs>
        <rect class="projectorMapFloor" x="0" y="0" width="100" height="100" />
        <rect
          class="projectorMapGrid"
          x="0"
          y="0"
          width="100"
          height="100"
          fill={`url(#projector-map-grid-${props.outputId})`}
        />
        <line class="projectorMapAxis" x1="50" y1="0" x2="50" y2="100" />
        <line class="projectorMapAxis" x1="0" y1="50" x2="100" y2="50" />
        <polygon class="projectorMapBase" points={projectorMapBasePoints(props.mapping).join(" ")} />
        <polygon class="projectorMapWarp" points={projectorMapPoints(props.mapping).join(" ")} />
        {(() => {
          const anchor = () => projectorRotateAnchorPoint(props.mapping);
          const point = () => projectorRotateHandlePoint(props.mapping);
          return (
            <>
              <line class="projectorMapRotateGuide" x1={anchor().x} y1={anchor().y} x2={point().x} y2={point().y} />
              <circle
                class="projectorMapRotateHandle"
                cx={point().x}
                cy={point().y}
                r="3.8"
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
                onKeyDown={nudgeRotationFromKeyboard}
              >
                <title>Rotate {props.label}. Arrow keys nudge, Shift coarse, Alt fine, Home reset.</title>
              </circle>
              <text class="projectorMapRotateLabel" x={point().x + 4} y={point().y - 4}>
                R
              </text>
            </>
          );
        })()}
        <For each={projectorScaleHandles}>
          {(handle) => {
            const anchor = () => projectorScaleAnchorPoint(props.mapping, handle.axis);
            const point = () => projectorScaleHandlePoint(props.mapping, handle.axis);
            return (
              <>
                <line class="projectorMapScaleGuide" x1={anchor().x} y1={anchor().y} x2={point().x} y2={point().y} />
                <circle
                  class={`projectorMapScaleHandle ${handle.axis}`}
                  cx={point().x}
                  cy={point().y}
                  r="3.8"
                  tabIndex={0}
                  onPointerDown={(event) => {
                    event.currentTarget.setPointerCapture(event.pointerId);
                    setScaleFromPointer(event, handle.axis);
                  }}
                  onPointerMove={(event) => {
                    if (event.buttons === 1) {
                      setScaleFromPointer(event, handle.axis);
                    }
                  }}
                  onPointerUp={(event) => event.currentTarget.releasePointerCapture(event.pointerId)}
                  onPointerCancel={(event) => event.currentTarget.releasePointerCapture(event.pointerId)}
                  onKeyDown={(event) => nudgeScaleFromKeyboard(event, handle.axis)}
                >
                  <title>
                    {handle.label} {props.label}. Arrow keys nudge, Shift coarse, Alt fine, Home reset.
                  </title>
                </circle>
                <text class="projectorMapScaleLabel" x={point().x + 4} y={point().y - 4}>
                  {handle.axis === "x" ? "SX" : "SY"}
                </text>
              </>
            );
          }}
        </For>
        <For each={projectorKeystoneHandles}>
          {(handle) => {
            const point = () => projectorKeystoneHandlePoint(props.mapping, handle.axis);
            return (
              <>
                <line
                  class="projectorMapKeystoneGuide"
                  x1={handle.baseX}
                  y1={handle.baseY}
                  x2={point().x}
                  y2={point().y}
                />
                <circle
                  class={`projectorMapKeystoneHandle ${handle.axis}`}
                  cx={point().x}
                  cy={point().y}
                  r="3.7"
                  tabIndex={0}
                  onPointerDown={(event) => {
                    event.currentTarget.setPointerCapture(event.pointerId);
                    setKeystoneFromPointer(event, handle.axis);
                  }}
                  onPointerMove={(event) => {
                    if (event.buttons === 1) {
                      setKeystoneFromPointer(event, handle.axis);
                    }
                  }}
                  onPointerUp={(event) => event.currentTarget.releasePointerCapture(event.pointerId)}
                  onPointerCancel={(event) => event.currentTarget.releasePointerCapture(event.pointerId)}
                  onKeyDown={(event) => nudgeKeystoneFromKeyboard(event, handle.axis)}
                >
                  <title>
                    {handle.label} {props.label}. Arrow keys nudge, Shift coarse, Alt fine, Home reset.
                  </title>
                </circle>
                <text class="projectorMapKeystoneLabel" x={point().x + 4} y={point().y - 4}>
                  {handle.label}
                </text>
              </>
            );
          }}
        </For>
        <circle
          class="projectorMapCenterHandle"
          cx={projectorMapCenter(props.mapping).x}
          cy={projectorMapCenter(props.mapping).y}
          r="4.2"
          tabIndex={0}
          onPointerDown={(event) => {
            event.currentTarget.setPointerCapture(event.pointerId);
            setOffsetFromPointer(event);
          }}
          onPointerMove={(event) => {
            if (event.buttons === 1) {
              setOffsetFromPointer(event);
            }
          }}
          onPointerUp={(event) => event.currentTarget.releasePointerCapture(event.pointerId)}
          onPointerCancel={(event) => event.currentTarget.releasePointerCapture(event.pointerId)}
          onKeyDown={nudgeOffsetFromKeyboard}
        >
          <title>Move {props.label}. Arrow keys nudge, Shift coarse, Alt fine, Home reset.</title>
        </circle>
        <For each={projectorCorners}>
          {(corner) => {
            const basePoint = () => projectorMapBasePoint(props.mapping, corner);
            const point = () => projectorMapPoint(props.mapping, corner);
            return (
              <>
                <line
                  class="projectorMapHandleGuide"
                  x1={basePoint().x}
                  y1={basePoint().y}
                  x2={point().x}
                  y2={point().y}
                />
                <circle class="projectorMapBasePoint" cx={basePoint().x} cy={basePoint().y} r="1.6" />
                <circle
                  class="projectorMapHandle"
                  cx={point().x}
                  cy={point().y}
                  r="4"
                  tabIndex={0}
                  onPointerDown={(event) => {
                    event.currentTarget.setPointerCapture(event.pointerId);
                    setCornerFromPointer(event, corner);
                  }}
                  onPointerMove={(event) => {
                    if (event.buttons === 1) {
                      setCornerFromPointer(event, corner);
                    }
                  }}
                  onPointerUp={(event) => event.currentTarget.releasePointerCapture(event.pointerId)}
                  onPointerCancel={(event) => event.currentTarget.releasePointerCapture(event.pointerId)}
                  onKeyDown={(event) => nudgeCornerFromKeyboard(event, corner)}
                >
                  <title>
                    {corner.label} {props.label}. Arrow keys nudge, Shift coarse, Alt fine, Home reset.
                  </title>
                </circle>
                <text class="projectorMapHandleLabel" x={point().x + 5} y={point().y - 5}>
                  {corner.label}
                </text>
              </>
            );
          }}
        </For>
      </svg>
      <div class="projectorMapCornerResetRow" aria-label={`${props.label} corner pin resets`}>
        <span>Corners</span>
        <For each={projectorCorners}>
          {(corner) => (
            <button onClick={() => resetCornerOffset(corner)} title={`Reset ${corner.label} corner pin`}>
              {corner.label} 0
            </button>
          )}
        </For>
      </div>
      <div class="projectorMapModeRow" aria-label={`${props.label} aspect mode`}>
        <button
          class={props.mapping.aspect_mode === "Stretch" ? "active" : ""}
          onClick={() => props.onPatch({ aspect_mode: "Stretch" })}
        >
          Stretch
        </button>
        <button
          class={props.mapping.aspect_mode === "Fit" ? "active" : ""}
          onClick={() => props.onPatch({ aspect_mode: "Fit" })}
        >
          Fit
        </button>
        <button
          class={props.mapping.aspect_mode === "Fill" ? "active" : ""}
          onClick={() => props.onPatch({ aspect_mode: "Fill" })}
        >
          Fill
        </button>
      </div>
      <div class="projectorMapAspectPresetRow" aria-label={`${props.label} aspect ratio presets`}>
        <For each={videoOutputAspectPresets}>
          {(preset) => (
            <button
              class={Math.abs(finiteOr(props.mapping.aspect_ratio, 1) - preset.ratio) < 0.01 ? "active" : ""}
              onClick={() => props.onPatch({ aspect_ratio: roundedRangeValue(preset.ratio, 0.25, 4), aspect_mode: "Fit" })}
            >
              {preset.label}
            </button>
          )}
        </For>
      </div>
      <div class="projectorMapKeystonePresetRow" aria-label={`${props.label} keystone presets`}>
        <For each={projectorKeystonePresets}>
          {(preset) => (
            <button
              class={
                Math.abs(finiteOr(props.mapping.keystone_x, 0) - preset.keystoneX) < 0.01 &&
                Math.abs(finiteOr(props.mapping.keystone_y, 0) - preset.keystoneY) < 0.01
                  ? "active"
                  : ""
              }
              onClick={() =>
                props.onPatch({
                  keystone_x: preset.keystoneX,
                  keystone_y: preset.keystoneY,
                })
              }
            >
              <svg class="projectorKeystoneMini" viewBox="0 0 40 24" aria-hidden="true">
                <polygon points={projectorKeystonePresetPoints(preset.keystoneX, preset.keystoneY)} />
              </svg>
              <span>{preset.label}</span>
            </button>
          )}
        </For>
      </div>
      <div class="projectorMapTuningGrid">
        <label>
          Aspect
          <input
            type="range"
            min="0.25"
            max="4"
            step="0.01"
            value={clampRange(finiteOr(props.mapping.aspect_ratio, 1), 0.25, 4)}
            onInput={(event) => patchNumber("aspect_ratio", Number(event.currentTarget.value), 0.25, 4)}
          />
          <span>{finiteOr(props.mapping.aspect_ratio, 1).toFixed(2)}</span>
        </label>
        <label>
          Scale X
          <input
            type="range"
            min="0.25"
            max="2.5"
            step="0.01"
            value={clampRange(finiteOr(props.mapping.scale_x, 1), 0.25, 2.5)}
            onInput={(event) => patchNumber("scale_x", Number(event.currentTarget.value), 0.25, 2.5)}
          />
          <span>{finiteOr(props.mapping.scale_x, 1).toFixed(2)}</span>
        </label>
        <label>
          Scale Y
          <input
            type="range"
            min="0.25"
            max="2.5"
            step="0.01"
            value={clampRange(finiteOr(props.mapping.scale_y, 1), 0.25, 2.5)}
            onInput={(event) => patchNumber("scale_y", Number(event.currentTarget.value), 0.25, 2.5)}
          />
          <span>{finiteOr(props.mapping.scale_y, 1).toFixed(2)}</span>
        </label>
        <label>
          Rotate
          <input
            type="range"
            min="-180"
            max="180"
            step="0.5"
            value={clampRange(finiteOr(props.mapping.rotation_deg, 0), -180, 180)}
            onInput={(event) => patchNumber("rotation_deg", Number(event.currentTarget.value), -180, 180, 1)}
          />
          <span>{finiteOr(props.mapping.rotation_deg, 0).toFixed(1)} deg</span>
        </label>
        <label>
          Lens
          <input
            type="range"
            min="-1"
            max="1"
            step="0.01"
            value={clampRange(finiteOr(props.mapping.lens_distortion, 0), -1, 1)}
            onInput={(event) => patchNumber("lens_distortion", Number(event.currentTarget.value), -1, 1)}
          />
          <span>{finiteOr(props.mapping.lens_distortion, 0).toFixed(2)}</span>
        </label>
      </div>
    </div>
  );
}
