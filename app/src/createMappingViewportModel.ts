import { createMemo, type Accessor, type Setter } from "solid-js";
import { fixtureVisualKind, mappingFixtureStageSize } from "./fixtureVisuals";
import type { MappingMarqueeState, MappingSnapLine } from "./mappingRuntime";
import type { MappingStageTool } from "./mappingViewPresets";
import { clampRange } from "./numericHelpers";
import { stageViewBoxSize, stageWorldToSvgPoint, type StageWorldBounds } from "./stageGeometry";
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
  selectedFixture: Accessor<PatchedFixtureSummary | undefined>;
  stageTool: Accessor<MappingStageTool>;
  snapEnabled: Accessor<boolean>;
  snapSize: Accessor<number>;
  marquee: Accessor<MappingMarqueeState | null>;
  onStatus: (message: string) => void;
}

export const createMappingViewportModel = (options: MappingViewportModelOptions) => {
  const normalizedMappingViewportZoom = createMemo(() => clampRange(options.viewportZoom(), 1, 4));
  const mappingViewportSize = createMemo(() => stageViewBoxSize / normalizedMappingViewportZoom());
  const mappingViewportBox = createMemo(() => {
    const size = mappingViewportSize();
    return {
      x: clampRange(options.viewportCenterX() - size / 2, 0, stageViewBoxSize - size),
      z: clampRange(options.viewportCenterZ() - size / 2, 0, stageViewBoxSize - size),
      size,
    };
  });
  const mappingStageViewBox = createMemo(() => {
    const box = mappingViewportBox();
    return `${box.x} ${box.z} ${box.size} ${box.size}`;
  });
  const mappingStageCursorSvgPoint = createMemo(() => {
    const point = options.stageCursorWorld();
    return point ? stageWorldToSvgPoint(point.x, point.z, options.stageWorldBounds()) : null;
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
    const size = mappingFixtureStageSize(visualKind);
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
    const nextZoom = clampRange(zoom, 1, 4);
    const nextSize = stageViewBoxSize / nextZoom;
    options.setViewportZoom(nextZoom);
    options.setViewportCenterX(clampRange(centerX, nextSize / 2, stageViewBoxSize - nextSize / 2));
    options.setViewportCenterZ(clampRange(centerZ, nextSize / 2, stageViewBoxSize - nextSize / 2));
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
      4,
    );
    const nextSize = stageViewBoxSize / nextZoom;
    const anchorX = clampRange((point.x - currentBox.x) / currentBox.size, 0, 1);
    const anchorZ = clampRange((point.z - currentBox.z) / currentBox.size, 0, 1);
    setMappingViewport(nextZoom, point.x + (0.5 - anchorX) * nextSize, point.z + (0.5 - anchorZ) * nextSize);
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
    const firstX = Math.ceil(bounds.minX / step) * step;
    const lastX = Math.floor(bounds.maxX / step) * step;
    const firstZ = Math.ceil(bounds.minZ / step) * step;
    const lastZ = Math.floor(bounds.maxZ / step) * step;
    const xCount = Math.max(0, Math.floor((lastX - firstX) / step) + 1);
    const zCount = Math.max(0, Math.floor((lastZ - firstZ) / step) + 1);
    if (xCount + zCount > 180) {
      return [];
    }
    const lines: MappingSnapLine[] = [];
    for (let index = 0; index < xCount; index += 1) {
      const x = firstX + index * step;
      lines.push({ axis: "x", svg: stageWorldToSvgPoint(x, 0, bounds).x });
    }
    for (let index = 0; index < zCount; index += 1) {
      const z = firstZ + index * step;
      lines.push({ axis: "z", svg: stageWorldToSvgPoint(0, z, bounds).z });
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
