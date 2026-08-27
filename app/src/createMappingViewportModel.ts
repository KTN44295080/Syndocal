import { createMemo, type Accessor, type Setter } from "solid-js";
import {
  fixtureVisualKind,
  mappingFixtureGridStageSize,
  mappingFixtureStageSize,
  mappingFixtureWorldToSvgScale,
} from "./fixtureVisuals";
import type { MappingMarqueeState, MappingSnapLine } from "./mappingRuntime";
import type { MappingStageTool } from "./mappingViewPresets";
import { clampRange } from "./numericHelpers";
import {
  mappingStageSvgPointToWorld,
  mappingStageWorldToSvgPoint,
  stageViewBoxSize,
  type StageWorldBounds,
} from "./stageGeometry";
import type { PatchFixtureRequest, PatchedFixtureSummary } from "./types";

interface MappingViewportModelOptions {
  viewportZoom: Accessor<number>;
  viewportCenterX: Accessor<number>;
  viewportCenterZ: Accessor<number>;
  setViewportZoom: Setter<number>;
  setViewportCenterX: Setter<number>;
  setViewportCenterZ: Setter<number>;
  stageCursorWorld: Accessor<{ x: number; z: number } | null>;
  stageWorldBounds: Accessor<StageWorldBounds>;
  viewportPixelSize: Accessor<{ width: number; height: number }>;
  selectedFixture: Accessor<PatchedFixtureSummary | undefined>;
  stageTool: Accessor<MappingStageTool>;
  snapEnabled: Accessor<boolean>;
  snapSize: Accessor<number>;
  marquee: Accessor<MappingMarqueeState | null>;
  onStatus: (message: string) => void;
}

export const mappingViewportTargetGlyphCellPx = 96;
export const mappingViewportMinimumMaxZoom = 4;
export const mappingViewportZoomStep = 0.05;

export interface MappingViewportBox {
  x: number;
  z: number;
  width: number;
  height: number;
}

const normalizedViewportAspect = (viewportPixelSize: { width: number; height: number }) => {
  const width = Number.isFinite(viewportPixelSize.width) && viewportPixelSize.width > 0
    ? viewportPixelSize.width
    : 1;
  const height = Number.isFinite(viewportPixelSize.height) && viewportPixelSize.height > 0
    ? viewportPixelSize.height
    : 1;
  return width / height;
};

export const mappingViewportDimensions = (
  zoom: number,
  viewportPixelSize: { width: number; height: number },
) => {
  const shortSpan = stageViewBoxSize / Math.max(1, zoom);
  const aspect = normalizedViewportAspect(viewportPixelSize);
  return aspect >= 1
    ? { width: shortSpan * aspect, height: shortSpan }
    : { width: shortSpan, height: shortSpan / aspect };
};

const clampViewportCenter = (center: number, span: number, baseMin: number, baseSpan: number) =>
  span >= baseSpan
    ? baseMin + baseSpan / 2
    : clampRange(center, baseMin + span / 2, baseMin + baseSpan - span / 2);

export const mappingViewportBoxFor = (
  zoom: number,
  centerX: number,
  centerZ: number,
  viewportPixelSize: { width: number; height: number },
): MappingViewportBox => {
  const { width, height } = mappingViewportDimensions(zoom, viewportPixelSize);
  const base = mappingViewportDimensions(1, viewportPixelSize);
  const baseX = stageViewBoxSize / 2 - base.width / 2;
  const baseZ = stageViewBoxSize / 2 - base.height / 2;
  const normalizedCenterX = clampViewportCenter(centerX, width, baseX, base.width);
  const normalizedCenterZ = clampViewportCenter(centerZ, height, baseZ, base.height);
  return {
    x: normalizedCenterX - width / 2,
    z: normalizedCenterZ - height / 2,
    width,
    height,
  };
};

export const mappingViewportMaxZoomForBounds = (
  bounds: StageWorldBounds,
  viewportPixelSize: { width: number; height: number },
) => {
  const viewportSpanPx = Math.max(1, Math.min(viewportPixelSize.width, viewportPixelSize.height));
  const glyphCellStageSize = Math.max(Number.EPSILON, mappingFixtureGridStageSize(bounds));
  const requiredZoom =
    (mappingViewportTargetGlyphCellPx * stageViewBoxSize) /
    (glyphCellStageSize * viewportSpanPx);
  return Number((
    Math.ceil(Math.max(mappingViewportMinimumMaxZoom, requiredZoom) / mappingViewportZoomStep)
    * mappingViewportZoomStep
  ).toFixed(2));
};

export const createMappingViewportModel = (options: MappingViewportModelOptions) => {
  const mappingViewportMaxZoom = createMemo(() =>
    mappingViewportMaxZoomForBounds(options.stageWorldBounds(), options.viewportPixelSize()));
  const normalizedMappingViewportZoom = createMemo(() =>
    clampRange(options.viewportZoom(), 1, mappingViewportMaxZoom()));
  const mappingViewportBox = createMemo(() => mappingViewportBoxFor(
    normalizedMappingViewportZoom(),
    options.viewportCenterX(),
    options.viewportCenterZ(),
    options.viewportPixelSize(),
  ));
  const mappingStageViewBox = createMemo(() => {
    const box = mappingViewportBox();
    return `${box.x} ${box.z} ${box.width} ${box.height}`;
  });
  const mappingStageCursorSvgPoint = createMemo(() => {
    const point = options.stageCursorWorld();
    return point ? mappingStageWorldToSvgPoint(point.x, point.z, options.stageWorldBounds()) : null;
  });
  const mappingStageCursorLabel = createMemo(() => {
    const point = options.stageCursorWorld();
    return point ? `X ${point.x.toFixed(2)} / Z ${point.z.toFixed(2)}` : "No stage cursor";
  });
  const mappingPlacePreview = createMemo(() => {
    const fixture = options.selectedFixture();
    const point = mappingStageCursorSvgPoint();
    if (!fixture || !point || options.stageTool() !== "place") {
      return null;
    }
    const visualKind = fixtureVisualKind(fixture);
    const size = mappingFixtureStageSize(
      visualKind,
      1,
      mappingFixtureWorldToSvgScale(options.stageWorldBounds()),
    );
    return {
      label: fixture.label,
      dmxLabel: `U${fixture.universe} A${fixture.address}`,
      groupLabel: fixture.group_ids.length > 0 ? fixture.group_ids.join(", ") : "No group",
      visualKind,
      x: point.x,
      z: point.z,
      width: size.width,
      height: size.height,
      yaw: fixture.rotation.yaw,
      color: "rgba(255, 221, 116, 0.78)",
    };
  });
  const mappingViewportZoomLabel = createMemo(() => `${Math.round(normalizedMappingViewportZoom() * 100)}%`);

  const setMappingViewport = (
    zoom: number,
    centerX = options.viewportCenterX(),
    centerZ = options.viewportCenterZ(),
  ) => {
    const nextZoom = clampRange(zoom, 1, mappingViewportMaxZoom());
    const nextDimensions = mappingViewportDimensions(nextZoom, options.viewportPixelSize());
    const baseDimensions = mappingViewportDimensions(1, options.viewportPixelSize());
    const baseX = stageViewBoxSize / 2 - baseDimensions.width / 2;
    const baseZ = stageViewBoxSize / 2 - baseDimensions.height / 2;
    options.setViewportZoom(nextZoom);
    options.setViewportCenterX(clampViewportCenter(centerX, nextDimensions.width, baseX, baseDimensions.width));
    options.setViewportCenterZ(clampViewportCenter(centerZ, nextDimensions.height, baseZ, baseDimensions.height));
  };
  const zoomMappingViewport = (direction: -1 | 1) => {
    const currentZoom = normalizedMappingViewportZoom();
    setMappingViewport(Number((direction > 0 ? currentZoom * 1.25 : currentZoom / 1.25).toFixed(3)));
  };
  const zoomMappingViewportAtPoint = (direction: -1 | 1, point: { x: number; z: number }) => {
    const currentBox = mappingViewportBox();
    const currentZoom = normalizedMappingViewportZoom();
    const nextZoom = clampRange(
      Number((direction > 0 ? currentZoom * 1.18 : currentZoom / 1.18).toFixed(3)),
      1,
      mappingViewportMaxZoom(),
    );
    const nextDimensions = mappingViewportDimensions(nextZoom, options.viewportPixelSize());
    const anchorX = clampRange((point.x - currentBox.x) / currentBox.width, 0, 1);
    const anchorZ = clampRange((point.z - currentBox.z) / currentBox.height, 0, 1);
    setMappingViewport(
      nextZoom,
      point.x + (0.5 - anchorX) * nextDimensions.width,
      point.z + (0.5 - anchorZ) * nextDimensions.height,
    );
  };
  const resetMappingViewport = () => {
    setMappingViewport(1, stageViewBoxSize / 2, stageViewBoxSize / 2);
    options.onStatus("Reset 2D mapping viewport.");
  };
  const normalizedMappingSnapSize = createMemo(() => {
    const size = Math.abs(options.snapSize());
    return Number.isFinite(size) ? clampRange(size, 0.05, 20) : 0.5;
  });
  const snapStageCoordinate = (value: number) => {
    const nextValue = options.snapEnabled()
      ? Math.round(value / normalizedMappingSnapSize()) * normalizedMappingSnapSize()
      : value;
    return Number(nextValue.toFixed(2));
  };
  const snapStagePoint = (point: { x: number; z: number }) => ({
    x: snapStageCoordinate(point.x),
    z: snapStageCoordinate(point.z),
  });
  const snapStageLength = (value: number) => {
    const nextValue = options.snapEnabled()
      ? Math.round(value / normalizedMappingSnapSize()) * normalizedMappingSnapSize()
      : value;
    return Number(clampRange(nextValue, 0.05, 1_000).toFixed(2));
  };
  const snapStagePosition = (position: PatchFixtureRequest["position"]) => ({
    ...position,
    x: snapStageCoordinate(position.x),
    z: snapStageCoordinate(position.z),
  });
  const mappingSnapLines = createMemo<MappingSnapLine[]>(() => {
    if (!options.snapEnabled()) {
      return [];
    }
    const step = normalizedMappingSnapSize();
    const bounds = options.stageWorldBounds();
    const viewport = mappingViewportBox();
    const topLeft = mappingStageSvgPointToWorld(viewport.x, viewport.z, bounds);
    const bottomRight = mappingStageSvgPointToWorld(
      viewport.x + viewport.width,
      viewport.z + viewport.height,
      bounds,
    );
    const firstX = Math.ceil(Math.min(topLeft.x, bottomRight.x) / step) * step;
    const lastX = Math.floor(Math.max(topLeft.x, bottomRight.x) / step) * step;
    const firstZ = Math.ceil(Math.min(topLeft.z, bottomRight.z) / step) * step;
    const lastZ = Math.floor(Math.max(topLeft.z, bottomRight.z) / step) * step;
    const xCount = Math.max(0, Math.floor((lastX - firstX) / step) + 1);
    const zCount = Math.max(0, Math.floor((lastZ - firstZ) / step) + 1);
    if (xCount + zCount > 180) {
      return [];
    }
    const lines: MappingSnapLine[] = [];
    for (let index = 0; index < xCount; index += 1) {
      const x = firstX + index * step;
      lines.push({ axis: "x", svg: mappingStageWorldToSvgPoint(x, 0, bounds).x });
    }
    for (let index = 0; index < zCount; index += 1) {
      const z = firstZ + index * step;
      lines.push({ axis: "z", svg: mappingStageWorldToSvgPoint(0, z, bounds).z });
    }
    return lines;
  });
  const mappingMarqueeBox = createMemo(() => {
    const marquee = options.marquee();
    if (!marquee) {
      return null;
    }
    return {
      x: Math.min(marquee.start.x, marquee.current.x),
      z: Math.min(marquee.start.z, marquee.current.z),
      width: Math.abs(marquee.current.x - marquee.start.x),
      height: Math.abs(marquee.current.z - marquee.start.z),
    };
  });

  return {
    normalizedMappingViewportZoom,
    mappingViewportMaxZoom,
    mappingViewportBox,
    mappingStageViewBox,
    mappingStageCursorSvgPoint,
    mappingStageCursorLabel,
    mappingPlacePreview,
    mappingViewportZoomLabel,
    setMappingViewport,
    zoomMappingViewport,
    zoomMappingViewportAtPoint,
    resetMappingViewport,
    normalizedMappingSnapSize,
    snapStageCoordinate,
    snapStagePoint,
    snapStageLength,
    snapStagePosition,
    mappingSnapLines,
    mappingMarqueeBox,
  };
};
