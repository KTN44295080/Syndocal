import { createMemo, type Accessor } from "solid-js";
import {
  dmxPreviewMap,
  fixtureIntersectsLiveColorViewport,
  fixtureLiveColor,
  fixtureLiveSegmentSkeleton,
} from "./fixtureLiveColor";
import { colorCandidates, readFixtureAttribute } from "./fixtureControlRuntime";
import {
  fixtureTypeKey,
  fixtureVisualKind,
  mappingFixtureSegmentCell,
  mappingFixtureSegmentGrid,
  mappingFixtureSegmentOrder,
  mappingFixtureStageSize,
  mappingFixtureWorldToSvgScale,
} from "./fixtureVisuals";
import {
  mappingGeometryClass,
  surfaceWorldHalfSize,
  type MappingDragState,
  type MappingGeometryNode2d,
  type VisualizerFixture,
  type VisualizerStageObject2d,
  type VisualizerVideoSurface2d,
} from "./mappingRuntime";
import {
  clamp01,
  clampRange,
  cumulativeGeometryMatrix,
  geometryMatrixTranslation,
  rotateStageOffsetYaw,
} from "./numericHelpers";
import {
  beamPoints,
  mappingStageSvgFrame,
  mappingStageWorldToSvgPoint,
  type StageWorldBounds,
} from "./stageGeometry";
import { stageObjectDefaultColor } from "./stageObjects";
import type {
  DmxUniversePreview,
  EngineSnapshot,
  PatchFixtureRequest,
  PatchedFixtureSummary,
  StageObjectSummary,
  VideoOutputMapping,
  VideoOutputSummary,
} from "./types";
import { mappingVideoOutputCornerGain, mappingVideoOutputCorners } from "./videoOutputMapping";
import type { MappingViewportBox } from "./createMappingViewportModel";

interface MappingRenderModelOptions {
  mappingDrag: Accessor<MappingDragState | null>;
  mappingShowGeometry: Accessor<boolean>;
  mappingFilteredFixtures: Accessor<PatchedFixtureSummary[]>;
  liveFixtures: Accessor<PatchedFixtureSummary[]>;
  mappingViewportBox: Accessor<MappingViewportBox>;
  dmxPreviews: Accessor<DmxUniversePreview[]>;
  stageWorldBounds: Accessor<StageWorldBounds>;
  selectedMappingFixtureIdSet: Accessor<Set<number>>;
  selectedFixtureGroupFilter: Accessor<string | null>;
  selectedFixtureId: Accessor<number | null>;
  faderValues: Accessor<Record<string, number>>;
  snapshot: Accessor<EngineSnapshot>;
  selectedStageObjectId: Accessor<number | null>;
  snapStagePoint: (point: { x: number; z: number }) => { x: number; z: number };
  snapStagePosition: (position: PatchFixtureRequest["position"]) => PatchFixtureRequest["position"];
  snapStageLength: (value: number) => number;
}

export const createMappingRenderModel = (options: MappingRenderModelOptions) => {
  const dragWorldDelta = (drag: MappingDragState) => ({
    x: drag.currentWorld.x - drag.startWorld.x,
    z: drag.currentWorld.z - drag.startWorld.z,
  });
  const mappingOutputHandleAngleDeg = (center: { x: number; z: number }, point: { x: number; z: number }) =>
    (Math.atan2(point.z - center.z, point.x - center.x) * 180) / Math.PI;
  const mappingOutputHandleDistance = (center: { x: number; z: number }, point: { x: number; z: number }) => {
    const dx = point.x - center.x;
    const dz = point.z - center.z;
    return Math.sqrt(dx * dx + dz * dz);
  };
  const mappingVideoSurfaceCornerLocals = (surface: VisualizerVideoSurface2d, mapping: VideoOutputMapping) =>
    mappingVideoOutputCorners.map((corner) => ({
      key: corner.key,
      label: corner.label,
      x:
        corner.baseX * (surface.width / 2) +
        clampRange(mapping[corner.xField], -1, 1) * surface.width * mappingVideoOutputCornerGain,
      z:
        corner.baseZ * (surface.height / 2) +
        clampRange(mapping[corner.zField], -1, 1) * surface.height * mappingVideoOutputCornerGain,
    }));
  const mappingVideoSurfaceCornerPointList = (surface: VisualizerVideoSurface2d, mapping: VideoOutputMapping) =>
    mappingVideoSurfaceCornerLocals(surface, mapping)
      .map((corner) => `${corner.x},${corner.z}`)
      .join(" ");
  const mappingWorldToVideoOutputLocal = (point: { x: number; z: number }, mapping: VideoOutputMapping) => {
    const dx = point.x - mapping.stage_x;
    const dz = point.z - mapping.stage_z;
    const rotation = (-mapping.rotation_deg * Math.PI) / 180;
    const cos = Math.cos(rotation);
    const sin = Math.sin(rotation);
    return { x: dx * cos - dz * sin, z: dx * sin + dz * cos };
  };
  const mappingWorldToStageObjectLocal = (point: { x: number; z: number }, object: StageObjectSummary) => {
    const dx = point.x - object.x;
    const dz = point.z - object.z;
    const rotation = (-object.rotation_deg * Math.PI) / 180;
    const cos = Math.cos(rotation);
    const sin = Math.sin(rotation);
    return { x: dx * cos - dz * sin, z: dx * sin + dz * cos };
  };
  const mappingVideoOutputCornerPreviewMapping = (
    drag: Extract<MappingDragState, { kind: "videoOutputCorner" }>,
    output: VideoOutputSummary,
  ): VideoOutputMapping => {
    const corner = mappingVideoOutputCorners.find((candidate) => candidate.key === drag.corner);
    if (!corner) {
      return drag.startMapping;
    }
    const size = surfaceWorldHalfSize(output, drag.startMapping);
    const local = mappingWorldToVideoOutputLocal(drag.currentWorld, drag.startMapping);
    const fullWidth = Math.max(0.001, size.width * 2);
    const fullHeight = Math.max(0.001, size.height * 2);
    const nextX = Number(
      clampRange(
        (local.x - corner.baseX * size.width) / (fullWidth * mappingVideoOutputCornerGain),
        -1,
        1,
      ).toFixed(3),
    );
    const nextZ = Number(
      clampRange(
        (local.z - corner.baseZ * size.height) / (fullHeight * mappingVideoOutputCornerGain),
        -1,
        1,
      ).toFixed(3),
    );
    return { ...drag.startMapping, [corner.xField]: nextX, [corner.zField]: nextZ };
  };
  const isMappingVideoOutputDrag = (
    drag: MappingDragState,
  ): drag is Extract<MappingDragState, { outputId: number }> =>
    drag.kind === "videoOutput" ||
    drag.kind === "videoOutputRotate" ||
    drag.kind === "videoOutputScale" ||
    drag.kind === "videoOutputCorner";
  const mappingVideoOutputPreviewMapping = (
    drag: Extract<MappingDragState, { outputId: number }>,
    output: VideoOutputSummary,
  ): VideoOutputMapping => {
    if (drag.kind === "videoOutput") {
      const delta = dragWorldDelta(drag);
      const point = options.snapStagePoint({
        x: drag.startMapping.stage_x + delta.x,
        z: drag.startMapping.stage_z + delta.z,
      });
      return { ...drag.startMapping, stage_x: point.x, stage_z: point.z };
    }
    if (drag.kind === "videoOutputRotate") {
      const currentAngle = mappingOutputHandleAngleDeg(drag.centerWorld, drag.currentWorld);
      return {
        ...drag.startMapping,
        rotation_deg: Number((drag.startMapping.rotation_deg + currentAngle - drag.startAngleDeg).toFixed(1)),
      };
    }
    if (drag.kind === "videoOutputScale") {
      const currentDistance = mappingOutputHandleDistance(drag.centerWorld, drag.currentWorld);
      const ratio = currentDistance / Math.max(0.001, drag.startDistance);
      return {
        ...drag.startMapping,
        scale_x: Number(clampRange(drag.startMapping.scale_x * ratio, 0.25, 3).toFixed(3)),
        scale_y: Number(clampRange(drag.startMapping.scale_y * ratio, 0.25, 3).toFixed(3)),
      };
    }
    return drag.kind === "videoOutputCorner"
      ? mappingVideoOutputCornerPreviewMapping(drag, output)
      : output.mapping;
  };
  const mappingFixturePosition = (fixture: PatchedFixtureSummary) => {
    const drag = options.mappingDrag();
    if (drag?.kind !== "fixture" || !drag.fixtureIds.includes(fixture.id)) {
      return fixture.position;
    }
    const startPosition = drag.startPositions[fixture.id];
    if (!startPosition) {
      return fixture.position;
    }
    const delta = dragWorldDelta(drag);
    return options.snapStagePosition({
      ...startPosition,
      x: startPosition.x + delta.x,
      z: startPosition.z + delta.z,
    });
  };
  const mappingFixtureYawFromPoint = (center: { x: number; z: number }, point: { x: number; z: number }) => {
    const dx = point.x - center.x;
    const dz = point.z - center.z;
    return Math.abs(dx) < 0.001 && Math.abs(dz) < 0.001
      ? null
      : Number((((Math.atan2(dz, dx) * 180) / Math.PI + 90 + 360) % 360).toFixed(1));
  };
  const mappingFixtureYaw = (fixture: PatchedFixtureSummary) => {
    const drag = options.mappingDrag();
    if (drag?.kind !== "fixtureYaw" || drag.fixtureId !== fixture.id) {
      return fixture.rotation.yaw;
    }
    return mappingFixtureYawFromPoint(drag.centerWorld, drag.currentWorld) ?? fixture.rotation.yaw;
  };
  const mappingVideoOutputMapping = (output: VideoOutputSummary): VideoOutputMapping => {
    const drag = options.mappingDrag();
    return !drag || !isMappingVideoOutputDrag(drag) || drag.outputId !== output.id
      ? output.mapping
      : mappingVideoOutputPreviewMapping(drag, output);
  };
  const isMappingStageObjectDrag = (
    drag: MappingDragState,
  ): drag is Extract<MappingDragState, { objectId: number }> =>
    drag.kind === "stageObject" || drag.kind === "stageObjectRotate" || drag.kind === "stageObjectResize";
  const mappingStageObjectPreview = (object: StageObjectSummary): StageObjectSummary => {
    const drag = options.mappingDrag();
    if (!drag || !isMappingStageObjectDrag(drag) || drag.objectId !== object.id) {
      return object;
    }
    if (drag.kind === "stageObjectRotate") {
      const currentAngle = mappingOutputHandleAngleDeg(drag.centerWorld, drag.currentWorld);
      return {
        ...drag.startObject,
        rotation_deg: Number((drag.startObject.rotation_deg + currentAngle - drag.startAngleDeg).toFixed(1)),
      };
    }
    if (drag.kind === "stageObjectResize") {
      const local = mappingWorldToStageObjectLocal(drag.currentWorld, drag.startObject);
      return {
        ...drag.startObject,
        width: drag.resizeMode === "depth" ? drag.startObject.width : options.snapStageLength(Math.abs(local.x) * 2),
        depth: drag.resizeMode === "width" ? drag.startObject.depth : options.snapStageLength(Math.abs(local.z) * 2),
      };
    }
    const delta = dragWorldDelta(drag);
    const point = options.snapStagePoint({
      x: drag.startObject.x + delta.x,
      z: drag.startObject.z + delta.z,
    });
    return { ...drag.startObject, x: point.x, z: point.z };
  };
  const isDraggingMappingFixture = (fixtureId: number) => {
    const drag = options.mappingDrag();
    return drag?.kind === "fixture" && drag.fixtureIds.includes(fixtureId);
  };
  const isDraggingMappingVideoOutput = (outputId: number) => {
    const drag = options.mappingDrag();
    return Boolean(drag && isMappingVideoOutputDrag(drag) && drag.outputId === outputId);
  };
  const isDraggingMappingStageObject = (objectId: number) => {
    const drag = options.mappingDrag();
    return Boolean(drag && isMappingStageObjectDrag(drag) && drag.objectId === objectId);
  };
  const mappingGeometryNodes2d = createMemo<MappingGeometryNode2d[]>(() => {
    if (!options.mappingShowGeometry()) {
      return [];
    }
    const fixtures = options.mappingFilteredFixtures();
    if (fixtures.length === 0) {
      return [];
    }
    const bounds = options.stageWorldBounds();
    const worldToSvgScale = mappingStageSvgFrame(bounds).worldToSvgScale;
    const selectedIds = options.selectedMappingFixtureIdSet();
    const selectedGroupId = options.selectedFixtureGroupFilter();
    const dimensionOrZero = (value: number | null | undefined) =>
      value !== null && value !== undefined && Number.isFinite(value) ? Math.abs(value) : 0;

    return fixtures.flatMap((fixture) => {
      const fixturePosition = mappingFixturePosition(fixture);
      const fixtureYaw = mappingFixtureYaw(fixture);
      const inGroupFilter = selectedGroupId ? fixture.group_ids.includes(selectedGroupId) : true;
      const selected = selectedIds.has(fixture.id) || options.selectedFixtureId() === fixture.id;
      const geometryByName = new Map(fixture.geometries.map((geometry) => [geometry.name, geometry]));
      const mappedChannelCounts = new Map<string, number>();
      for (const control of fixture.controls) {
        const geometryName = control.geometry?.trim();
        if (geometryName) {
          mappedChannelCounts.set(geometryName, (mappedChannelCounts.get(geometryName) ?? 0) + 1);
        }
      }
      return fixture.geometries.map((geometry) => {
        const local = geometryMatrixTranslation(cumulativeGeometryMatrix(geometry, geometryByName));
        const rotated = rotateStageOffsetYaw({ x: local.x, z: local.z }, fixtureYaw);
        const point = mappingStageWorldToSvgPoint(
          fixturePosition.x + rotated.x,
          fixturePosition.z + rotated.z,
          bounds,
        );
        const beamDiameter = dimensionOrZero(geometry.beam_radius) * 2;
        const widthWorld = Math.max(dimensionOrZero(geometry.model_dimensions?.x), beamDiameter, 0.28);
        const heightWorld = Math.max(dimensionOrZero(geometry.model_dimensions?.z), beamDiameter, 0.28);
        const footprintWidth = clampRange(widthWorld * worldToSvgScale, 1.7, 8);
        const footprintHeight = clampRange(heightWorld * worldToSvgScale, 1.7, 8);
        const mappedChannelCount = mappedChannelCounts.get(geometry.name) ?? 0;
        return {
          key: `${fixture.id}:${geometry.name}`,
          fixtureId: fixture.id,
          fixtureLabel: fixture.label,
          name: geometry.name,
          kind: geometry.kind,
          x: point.x,
          z: point.z,
          radius: clampRange((footprintWidth + footprintHeight) / 5, 1.05, 2.2),
          footprintWidth,
          footprintHeight,
          mappedChannelCount,
          className: mappingGeometryClass(geometry, mappedChannelCount, inGroupFilter, selected),
          inGroupFilter,
          selected,
        };
      });
    });
  });
  const segmentSkeletonCache = new WeakMap<
    PatchedFixtureSummary,
    ReturnType<typeof fixtureLiveSegmentSkeleton>
  >();
  const fixtureSegmentSkeleton = (fixture: PatchedFixtureSummary) => {
    let skeleton = segmentSkeletonCache.get(fixture);
    if (!skeleton) {
      skeleton = fixtureLiveSegmentSkeleton(fixture);
      segmentSkeletonCache.set(fixture, skeleton);
    }
    return skeleton;
  };
  const visualizerFixtures = createMemo<VisualizerFixture[]>(() => {
    const fixtures = options.mappingFilteredFixtures();
    if (fixtures.length === 0) {
      return [];
    }
    const bounds = options.stageWorldBounds();
    const fixtureWorldToSvgScale = mappingFixtureWorldToSvgScale(bounds);
    const currentValues = options.faderValues();
    const groupFilter = options.selectedFixtureGroupFilter();
    return fixtures.map((fixture) => {
      const position = mappingFixturePosition(fixture);
      const point = mappingStageWorldToSvgPoint(position.x, position.z, bounds);
      const dimmer = readFixtureAttribute(fixture, currentValues, ["Dimmer", "Intensity"]) ?? 0;
      const pan = readFixtureAttribute(fixture, currentValues, ["Pan"]);
      const red = readFixtureAttribute(fixture, currentValues, colorCandidates.red);
      const green = readFixtureAttribute(fixture, currentValues, colorCandidates.green);
      const blue = readFixtureAttribute(fixture, currentValues, colorCandidates.blue);
      const color =
        red !== undefined || green !== undefined || blue !== undefined
          ? `rgb(${red ? red >> 8 : 0}, ${green ? green >> 8 : 0}, ${blue ? blue >> 8 : 0})`
          : "rgb(88, 167, 246)";
      const intensity = clamp01(dimmer / 65_535);
      const panDegrees = pan === undefined ? 0 : ((pan - 32_768) / 65_535) * 540;
      const yaw = mappingFixtureYaw(fixture);
      const visualKind = fixtureVisualKind(fixture);
      const segmentGrid = mappingFixtureSegmentGrid(
        fixture,
        fixtureSegmentSkeleton(fixture).length,
      );
      const segmentOrder = mappingFixtureSegmentOrder(fixture);
      const size = mappingFixtureStageSize(
        visualKind,
        segmentGrid.columns,
        fixtureWorldToSvgScale,
        segmentGrid.rows,
      );
      return {
        id: fixture.id,
        label: fixture.label,
        addressOrder: fixture.universe * 512 + fixture.address,
        dmxLabel: `U${fixture.universe} A${fixture.address}`,
        groupLabel: fixture.group_ids.length > 0 ? fixture.group_ids.join(", ") : "No group",
        typeKey: fixtureTypeKey(fixture),
        visualKind,
        x: point.x,
        z: point.z,
        width: size.width,
        height: size.height,
        segmentColumns: segmentGrid.columns,
        segmentRows: segmentGrid.rows,
        segmentOrder,
        yaw,
        beamYaw: yaw + panDegrees,
        beamPoints: beamPoints(point.x, point.z, yaw + panDegrees, intensity),
        intensity,
        color,
        inGroupFilter: groupFilter ? fixture.group_ids.includes(groupFilter) : true,
        highlighted: fixture.highlighted,
        soloed: fixture.soloed,
        parked: fixture.parked,
      };
    });
  });
  const mappingStageFixtureCache = new Map<number, {
    base: VisualizerFixture;
    signature: string;
    fixture: VisualizerFixture & {
      liveColorApplied: boolean;
      liveColorValueSource?: ReturnType<typeof fixtureLiveColor>["valueSource"];
      liveSegments?: ReturnType<typeof fixtureLiveColor>["segments"];
    };
  }>();
  const mappingStageFixtures = createMemo(() => {
    const baseFixtures = visualizerFixtures();
    const sourceFixtures = options.mappingFilteredFixtures();
    const sourceById = new Map(sourceFixtures.map((fixture) => [fixture.id, fixture]));
    const liveSourceById = new Map(options.liveFixtures().map((fixture) => [fixture.id, fixture]));
    const previewsByUniverse = dmxPreviewMap(options.dmxPreviews());
    const viewportBox = options.mappingViewportBox();
    const viewport = {
      x: viewportBox.x,
      z: viewportBox.z,
      width: viewportBox.width,
      height: viewportBox.height,
    };
    const liveIds = new Set<number>();
    const fixtures = baseFixtures.map((base) => {
      liveIds.add(base.id);
      const source = sourceById.get(base.id);
      const visible = fixtureIntersectsLiveColorViewport(base, viewport);
      const liveSource = liveSourceById.get(base.id);
      const segmentSkeleton = source ? fixtureSegmentSkeleton(source) : [];
      const live = source && visible
        ? fixtureLiveColor(source, previewsByUniverse, liveSource?.attribute_values)
        : null;
      const liveSegments = segmentSkeleton.length > 1
        ? live?.segments ?? segmentSkeleton
        : undefined;
      const beams = live && liveSegments
        ? liveSegments.map((segment, index) => {
            const columns = Math.max(1, base.segmentColumns);
            const rows = Math.max(1, base.segmentRows);
            const cell = mappingFixtureSegmentCell(
              { columns, rows },
              index,
              base.segmentOrder,
            );
            const cellPitchX = base.width / columns;
            const cellPitchZ = base.height / rows;
            const localX = (cell.column - (columns - 1) / 2) * cellPitchX;
            const localZ = (cell.row - (rows - 1) / 2) * cellPitchZ;
            const offset = rotateStageOffsetYaw({ x: localX, z: localZ }, base.yaw);
            const x = base.x + offset.x;
            const z = base.z + offset.z;
            return {
              cellIndex: index + 1,
              points: beamPoints(x, z, base.beamYaw, segment.intensity),
              intensity: segment.intensity,
              color: segment.color,
            };
          })
        : undefined;
      const signature = live
        ? `live:${live.valueSource}:${live.color}:${live.intensity}:${liveSegments?.map((segment) => `${segment.color}:${segment.intensity}`).join("|") ?? "single"}`
        : `offscreen:${liveSegments?.map((segment) => segment.key).join("|") ?? "single"}`;
      const cached = mappingStageFixtureCache.get(base.id);
      if (cached?.base === base && cached.signature === signature) {
        return cached.fixture;
      }
      const fixture = {
        ...base,
        ...(live ? { color: live.color, intensity: live.intensity } : {}),
        liveColorApplied: Boolean(live),
        liveColorValueSource: live?.valueSource,
        liveSegments,
        beams,
      };
      mappingStageFixtureCache.set(base.id, { base, signature, fixture });
      return fixture;
    });
    for (const fixtureId of mappingStageFixtureCache.keys()) {
      if (!liveIds.has(fixtureId)) mappingStageFixtureCache.delete(fixtureId);
    }
    return fixtures;
  });
  const visualizerVideoSurfaces2d = createMemo<VisualizerVideoSurface2d[]>(() => {
    const bounds = options.stageWorldBounds();
    return options.snapshot().video.outputs.map((output) => {
      const mapping = mappingVideoOutputMapping(output);
      const center = mappingStageWorldToSvgPoint(mapping.stage_x, mapping.stage_z, bounds);
      const size = surfaceWorldHalfSize(output, mapping);
      const xEdge = mappingStageWorldToSvgPoint(mapping.stage_x + size.width, mapping.stage_z, bounds);
      const zEdge = mappingStageWorldToSvgPoint(mapping.stage_x, mapping.stage_z + size.height, bounds);
      return {
        id: output.id,
        label: output.label,
        x: center.x,
        z: center.z,
        width: Math.max(3, Math.abs(xEdge.x - center.x) * 2),
        height: Math.max(2, Math.abs(zEdge.z - center.z) * 2),
        rotationDeg: mapping.rotation_deg,
        opacity: clampRange(output.opacity, 0, 1),
        active: output.enabled && !output.blackout,
      };
    });
  });
  const visualizerStageObjects2d = createMemo<VisualizerStageObject2d[]>(() => {
    const bounds = options.stageWorldBounds();
    return options.snapshot().stage_objects.map((object) => {
      const preview = mappingStageObjectPreview(object);
      const center = mappingStageWorldToSvgPoint(preview.x, preview.z, bounds);
      const xEdge = mappingStageWorldToSvgPoint(preview.x + preview.width / 2, preview.z, bounds);
      const zEdge = mappingStageWorldToSvgPoint(preview.x, preview.z + preview.depth / 2, bounds);
      return {
        id: preview.id,
        label: preview.label,
        kind: preview.kind,
        x: center.x,
        z: center.z,
        width: Math.max(1, Math.abs(xEdge.x - center.x) * 2),
        depth: Math.max(1, Math.abs(zEdge.z - center.z) * 2),
        rotationDeg: preview.rotation_deg,
        color: preview.color ?? stageObjectDefaultColor(preview.kind),
        selected: options.selectedStageObjectId() === preview.id,
      };
    });
  });

  return {
    dragWorldDelta,
    mappingOutputHandleAngleDeg,
    mappingOutputHandleDistance,
    mappingVideoSurfaceCornerLocals,
    mappingVideoSurfaceCornerPointList,
    mappingWorldToStageObjectLocal,
    mappingVideoOutputPreviewMapping,
    mappingFixturePosition,
    mappingFixtureYawFromPoint,
    mappingFixtureYaw,
    mappingVideoOutputMapping,
    mappingStageObjectPreview,
    isMappingStageObjectDrag,
    isDraggingMappingFixture,
    isDraggingMappingVideoOutput,
    isDraggingMappingStageObject,
    mappingGeometryNodes2d,
    visualizerFixtures,
    mappingStageFixtures,
    visualizerVideoSurfaces2d,
    visualizerStageObjects2d,
  };
};
