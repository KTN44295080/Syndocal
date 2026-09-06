import type { Accessor, Setter } from "solid-js";
import type { MappingStageTool } from "./mappingViewPresets";
import type { MappingViewportBox } from "./createMappingViewportModel";
import type {
  MappingDragState,
  MappingMarqueeState,
  MappingStageObjectResizeMode,
  MappingViewportPanDragState,
  VisualizerFixture,
} from "./mappingRuntime";
import { mappingFixtureRotationWithYawDelta } from "./mappingRuntime";
import { clampRange } from "./numericHelpers";
import type { ProjectAuthorityToken } from "./projectAuthority";
import {
  mappingStageSvgDeltaToWorld,
  mappingStageSvgPointToWorld,
  type StageWorldBounds,
} from "./stageGeometry";
import type {
  EngineSnapshot,
  PatchFixtureRequest,
  PatchedFixtureSummary,
  StageObjectSummary,
  VideoOutputMapping,
  VideoOutputSummary,
} from "./types";
import { mappingVideoOutputCorners, type MappingVideoOutputCornerKey } from "./videoOutputMapping";
import { applyMappingFixtureTransformBatch } from "./mappingFixtureTransformBatch";

type StagePoint = { x: number; z: number };
type MappingOutputDrag = Extract<MappingDragState, { outputId: number }>;
type MappingStageObjectDrag = Extract<MappingDragState, { objectId: number }>;
type MappingFixtureYawDrag = Extract<MappingDragState, { kind: "fixtureYaw" }>;
type FixtureTransformUpdate = {
  position?: PatchFixtureRequest["position"];
  rotation?: PatchFixtureRequest["rotation"];
};

export const MAPPING_FIXTURE_DRAG_THRESHOLD_PX = 4;

interface MappingInteractionControllerOptions {
  snapshot: Accessor<EngineSnapshot>;
  mappingViewportBox: Accessor<MappingViewportBox>;
  stageWorldBounds: Accessor<StageWorldBounds>;
  zoomMappingViewportAtPoint: (direction: -1 | 1, point: StagePoint) => void;
  selectedFixture: Accessor<PatchedFixtureSummary | undefined>;
  snapStagePoint: (point: StagePoint) => StagePoint;
  setFixtureTransform: (
    fixture: PatchedFixtureSummary,
    update: FixtureTransformUpdate,
    refresh?: boolean,
  ) => Promise<boolean>;
  mappingStageTool: Accessor<MappingStageTool>;
  isAdditiveMappingSelectionEvent: (event: Pick<MouseEvent, "ctrlKey" | "metaKey" | "shiftKey">) => boolean;
  selectMappingFixture: (
    fixture: PatchedFixtureSummary,
    event?: Pick<MouseEvent, "ctrlKey" | "metaKey" | "shiftKey">,
  ) => void;
  selectedMappingFixtureIdSet: Accessor<Set<number>>;
  selectedMappingFixtureIds: Accessor<number[]>;
  setSelectedMappingFixtureIds: Setter<number[]>;
  selectFixture: (fixture: PatchedFixtureSummary) => void;
  activateFixture: (fixture: PatchedFixtureSummary) => void;
  setSelectedFixtureId: Setter<number | null>;
  setSelectedStageObjectId: Setter<number | null>;
  setSelectedVideoOutputId: Setter<number | null>;
  clearMappingFixtureSelection: () => void;
  mappingDrag: Accessor<MappingDragState | null>;
  setMappingDrag: Setter<MappingDragState | null>;
  currentProjectAuthority: Accessor<ProjectAuthorityToken>;
  mappingMarquee: Accessor<MappingMarqueeState | null>;
  setMappingMarquee: Setter<MappingMarqueeState | null>;
  mappingViewportPanDrag: Accessor<MappingViewportPanDragState | null>;
  setMappingViewportPanDrag: Setter<MappingViewportPanDragState | null>;
  normalizedMappingViewportZoom: Accessor<number>;
  setMappingViewport: (zoom: number, centerX: number, centerZ: number) => void;
  setMappingStageCursorWorld: Setter<StagePoint | null>;
  mappingOutputHandleAngleDeg: (center: StagePoint, point: StagePoint) => number;
  mappingOutputHandleDistance: (center: StagePoint, point: StagePoint) => number;
  mappingFixtureYawFromPoint: (center: StagePoint, point: StagePoint) => number | null;
  dragWorldDelta: (drag: MappingDragState) => StagePoint;
  snapStagePosition: (position: PatchFixtureRequest["position"]) => PatchFixtureRequest["position"];
  refreshSnapshot: () => Promise<EngineSnapshot | null>;
  setMessage: (message: string) => unknown;
  isMappingStageObjectDrag: (drag: MappingDragState) => drag is MappingStageObjectDrag;
  mappingStageObjectPreview: (object: StageObjectSummary) => StageObjectSummary;
  setStageObject: (object: StageObjectSummary, updates: Partial<StageObjectSummary>) => Promise<void>;
  mappingVideoOutputPreviewMapping: (drag: MappingOutputDrag, output: VideoOutputSummary) => VideoOutputMapping;
  setVideoOutputMapping: (outputId: number, mapping: VideoOutputMapping) => Promise<boolean>;
  visualizerFixtures: Accessor<VisualizerFixture[]>;
}

export function createMappingInteractionController(options: MappingInteractionControllerOptions) {
  // Keep a yaw drag alive while its transform is being persisted. The drag
  // object is the lease: a later interaction gets a new object, so a late
  // completion cannot clear or report against that newer interaction.
  const pendingFixtureYawDrags = new Set<MappingFixtureYawDrag>();
  type MappingStageRotationOperation = {
    projectEpoch: number;
    selectedFixtureIds: number[];
  };
  let pendingStageRotationOperation: MappingStageRotationOperation | null = null;

  const sameFixtureIdSet = (left: readonly number[], right: readonly number[]) =>
    left.length === right.length && left.every((fixtureId) => right.includes(fixtureId));

  const stageRotationOperationIsCurrent = (
    operation: MappingStageRotationOperation,
  ) => pendingStageRotationOperation === operation
    && options.currentProjectAuthority().project_epoch === operation.projectEpoch
    && sameFixtureIdSet(operation.selectedFixtureIds, options.selectedMappingFixtureIds())
    && options.mappingDrag() === null;

  const mappingProjectEpochIsCurrent = (projectEpoch: number) =>
    options.currentProjectAuthority().project_epoch === projectEpoch;

  const invalidatePendingStageRotation = () => {
    pendingStageRotationOperation = null;
  };

  const stageSvgPointFromClient = (clientX: number, clientY: number, targetSvg: SVGSVGElement) => {
    const viewBox = options.mappingViewportBox();
    const clampToViewport = (point: { x: number; z: number }) => ({
      x: clampRange(point.x, viewBox.x, viewBox.x + viewBox.width),
      z: clampRange(point.z, viewBox.z, viewBox.z + viewBox.height),
    });
    const screenMatrix = targetSvg.getScreenCTM();
    if (screenMatrix) {
      const point = new DOMPoint(clientX, clientY).matrixTransform(screenMatrix.inverse());
      return clampToViewport({ x: point.x, z: point.y });
    }
    const rect = targetSvg.getBoundingClientRect();
    return clampToViewport({
      x: viewBox.x + ((clientX - rect.left) / rect.width) * viewBox.width,
      z: viewBox.z + ((clientY - rect.top) / rect.height) * viewBox.height,
    });
  };

  const stageSvgPointFromPointer = (event: PointerEvent, svg?: SVGSVGElement) => {
    const targetSvg = svg ?? (event.currentTarget as SVGSVGElement);
    return stageSvgPointFromClient(event.clientX, event.clientY, targetSvg);
  };

  const stageWorldPointFromPointer = (event: PointerEvent, svg?: SVGSVGElement) => {
    const point = stageSvgPointFromPointer(event, svg);
    return mappingStageSvgPointToWorld(point.x, point.z, options.stageWorldBounds());
  };

  const fixtureDragWorldPointFromPointer = (
    drag: Extract<MappingDragState, { kind: "fixture" | "fixtureYaw" }>,
    event: PointerEvent,
    svg: SVGSVGElement,
  ) => {
    const startSvg = stageSvgPointFromClient(drag.startClient.x, drag.startClient.y, svg);
    const currentSvg = stageSvgPointFromClient(event.clientX, event.clientY, svg);
    const bounds = options.stageWorldBounds();
    const delta = mappingStageSvgDeltaToWorld(
      currentSvg.x - startSvg.x,
      currentSvg.z - startSvg.z,
      bounds,
    );
    return {
      x: drag.startWorld.x + delta.x,
      z: drag.startWorld.z + delta.z,
    };
  };

  const handleMappingStageWheel = (event: WheelEvent & { currentTarget: SVGSVGElement }) => {
    event.preventDefault();
    const point = stageSvgPointFromClient(event.clientX, event.clientY, event.currentTarget);
    options.zoomMappingViewportAtPoint(event.deltaY < 0 ? 1 : -1, point);
  };

  const placeSelectedFixtureFromStage = async (
    event: PointerEvent & { currentTarget: SVGSVGElement },
  ) => {
    invalidatePendingStageRotation();
    const fixture = options.selectedFixture();
    if (!fixture) return;
    const point = options.snapStagePoint(stageWorldPointFromPointer(event));
    await options.setFixtureTransform(fixture, {
      position: { ...fixture.position, x: point.x, z: point.z },
    });
  };

  const rotateSelectedFixtureFromStage = async (
    event: PointerEvent & { currentTarget: SVGSVGElement },
  ) => {
    invalidatePendingStageRotation();
    const selected = options.selectedFixture();
    if (!selected) return;
    const snapshot = options.snapshot();
    const anchor = snapshot.fixtures.find((fixture) => fixture.id === selected.id);
    if (!anchor) return;
    const point = stageWorldPointFromPointer(event);
    const dx = point.x - anchor.position.x;
    const dz = point.z - anchor.position.z;
    if (Math.abs(dx) < 0.001 && Math.abs(dz) < 0.001) return;
    const yaw = Math.round(((Math.atan2(dz, dx) * 180) / Math.PI + 90 + 360) % 360);
    const selectionBaseline = [...options.selectedMappingFixtureIds()];
    const selectedIds = options.selectedMappingFixtureIdSet();
    const fixtureIds = [...new Set(selectedIds.has(anchor.id)
      ? options.selectedMappingFixtureIds()
      : [anchor.id])];
    const fixtures = snapshot.fixtures.filter((fixture) => fixtureIds.includes(fixture.id));
    if (fixtures.length === 0 || !fixtures.some((fixture) => fixture.id === anchor.id)) return;
    if (!fixtures.every((fixture) => Object.values(fixture.rotation).every(Number.isFinite))) {
      options.setMessage("Cannot rotate fixtures with an invalid saved rotation.");
      return;
    }
    const yawDelta = yaw - anchor.rotation.yaw;
    const transforms = fixtures.map((fixture) => ({
      fixture,
      update: { rotation: mappingFixtureRotationWithYawDelta(fixture.rotation, yawDelta) },
    }));
    const operation: MappingStageRotationOperation = {
      projectEpoch: options.currentProjectAuthority().project_epoch,
      selectedFixtureIds: selectionBaseline,
    };
    pendingStageRotationOperation = operation;
    try {
      const persisted = await applyMappingFixtureTransformBatch({
        transforms,
        setFixtureTransform: (fixture, update, refresh) => {
          if (!stageRotationOperationIsCurrent(operation)) return Promise.resolve(false);
          return options.setFixtureTransform(fixture, update, refresh);
        },
        refreshSnapshot: options.refreshSnapshot,
        setMessage: (message) => {
          if (stageRotationOperationIsCurrent(operation)) options.setMessage(message);
        },
      });
      if (!stageRotationOperationIsCurrent(operation)) return;
      if (persisted) {
        options.setMessage(
          fixtures.length === 1
            ? `Set ${anchor.label} yaw to ${yaw} deg.`
            : `Rotated ${fixtures.length} selected fixtures by ${yawDelta} deg.`,
        );
      }
    } finally {
      if (pendingStageRotationOperation === operation) {
        pendingStageRotationOperation = null;
      }
    }
  };

  const beginMappingFixtureDrag = (event: PointerEvent, fixtureId: number) => {
    if (event.button !== 0 || options.mappingStageTool() !== "select") return;
    invalidatePendingStageRotation();
    const fixture = options.snapshot().fixtures.find((candidate) => candidate.id === fixtureId);
    const svg = (event.currentTarget as SVGElement).ownerSVGElement;
    if (!fixture || !svg) return;
    if (options.isAdditiveMappingSelectionEvent(event)) {
      options.selectMappingFixture(fixture, event);
      return;
    }
    const selectedIds = options.selectedMappingFixtureIdSet();
    const fixtureIds = selectedIds.has(fixtureId) ? options.selectedMappingFixtureIds() : [fixtureId];
    if (!selectedIds.has(fixtureId)) options.selectFixture(fixture);
    else options.activateFixture(fixture);
    options.setSelectedStageObjectId(null);
    const startPositions = Object.fromEntries(
      options.snapshot().fixtures
        .filter((candidate) => fixtureIds.includes(candidate.id))
        .map((candidate) => [candidate.id, candidate.position]),
    ) as Record<number, PatchFixtureRequest["position"]>;
    const point = stageWorldPointFromPointer(event, svg);
    svg.setPointerCapture(event.pointerId);
    options.setMappingDrag({
      kind: "fixture",
      pointerId: event.pointerId,
      fixtureIds,
      startWorld: point,
      currentWorld: point,
      startClient: { x: event.clientX, y: event.clientY },
      currentClient: { x: event.clientX, y: event.clientY },
      startPositions,
    });
  };

  const beginMappingFixtureYawDrag = (event: PointerEvent, fixtureId: number) => {
    if (event.button !== 0 || options.mappingStageTool() === "pan" || options.mappingStageTool() === "place") return;
    invalidatePendingStageRotation();
    const snapshot = options.snapshot();
    const fixture = snapshot.fixtures.find((candidate) => candidate.id === fixtureId);
    const svg = (event.currentTarget as SVGElement).ownerSVGElement;
    if (!fixture || !svg) return;
    event.preventDefault();
    event.stopPropagation();
    const selectedIds = options.selectedMappingFixtureIdSet();
    const fixtureIds = [...new Set(
      (selectedIds.has(fixtureId) ? options.selectedMappingFixtureIds() : [fixtureId])
        .filter((id) => snapshot.fixtures.some((candidate) => candidate.id === id)),
    )];
    if (!fixtureIds.includes(fixtureId)) fixtureIds.unshift(fixtureId);
    const selectedFixtures = snapshot.fixtures.filter((candidate) => fixtureIds.includes(candidate.id));
    if (!selectedFixtures.every((candidate) => Object.values(candidate.rotation).every(Number.isFinite))) return;
    const startRotations = Object.fromEntries(
      selectedFixtures.map((candidate) => [candidate.id, { ...candidate.rotation }]),
    ) as Record<number, PatchFixtureRequest["rotation"]>;
    if (selectedIds.has(fixtureId)) options.activateFixture(fixture);
    else options.selectFixture(fixture);
    const point = stageWorldPointFromPointer(event, svg);
    svg.setPointerCapture(event.pointerId);
    options.setMappingDrag({
      kind: "fixtureYaw",
      pointerId: event.pointerId,
      fixtureId,
      fixtureIds,
      startRotations,
      projectEpoch: options.currentProjectAuthority().project_epoch,
      startWorld: point,
      currentWorld: point,
      startClient: { x: event.clientX, y: event.clientY },
      currentClient: { x: event.clientX, y: event.clientY },
      centerWorld: { x: fixture.position.x, z: fixture.position.z },
    });
  };

  const beginMappingVideoOutputDrag = (event: PointerEvent, outputId: number) => {
    if (event.button !== 0 || options.mappingStageTool() !== "select") return;
    invalidatePendingStageRotation();
    const output = options.snapshot().video.outputs.find((candidate) => candidate.id === outputId);
    const svg = (event.currentTarget as SVGElement).ownerSVGElement;
    if (!output || !svg) return;
    const point = stageWorldPointFromPointer(event, svg);
    svg.setPointerCapture(event.pointerId);
    options.setSelectedStageObjectId(null);
    options.setMappingDrag({
      kind: "videoOutput",
      pointerId: event.pointerId,
      outputId,
      startWorld: point,
      currentWorld: point,
      startMapping: output.mapping,
    });
  };

  const beginMappingVideoOutputRotate = (event: PointerEvent, outputId: number) => {
    if (event.button !== 0 || options.mappingStageTool() !== "select") return;
    invalidatePendingStageRotation();
    const output = options.snapshot().video.outputs.find((candidate) => candidate.id === outputId);
    const svg = (event.currentTarget as SVGElement).ownerSVGElement;
    if (!output || !svg) return;
    const point = stageWorldPointFromPointer(event, svg);
    const centerWorld = { x: output.mapping.stage_x, z: output.mapping.stage_z };
    svg.setPointerCapture(event.pointerId);
    options.setMappingDrag({
      kind: "videoOutputRotate",
      pointerId: event.pointerId,
      outputId,
      startWorld: point,
      currentWorld: point,
      centerWorld,
      startAngleDeg: options.mappingOutputHandleAngleDeg(centerWorld, point),
      startMapping: output.mapping,
    });
  };

  const beginMappingVideoOutputScale = (event: PointerEvent, outputId: number) => {
    if (event.button !== 0 || options.mappingStageTool() !== "select") return;
    invalidatePendingStageRotation();
    const output = options.snapshot().video.outputs.find((candidate) => candidate.id === outputId);
    const svg = (event.currentTarget as SVGElement).ownerSVGElement;
    if (!output || !svg) return;
    const point = stageWorldPointFromPointer(event, svg);
    const centerWorld = { x: output.mapping.stage_x, z: output.mapping.stage_z };
    svg.setPointerCapture(event.pointerId);
    options.setMappingDrag({
      kind: "videoOutputScale",
      pointerId: event.pointerId,
      outputId,
      startWorld: point,
      currentWorld: point,
      centerWorld,
      startDistance: options.mappingOutputHandleDistance(centerWorld, point),
      startMapping: output.mapping,
    });
  };

  const beginMappingVideoOutputCornerDrag = (
    event: PointerEvent,
    outputId: number,
    corner: MappingVideoOutputCornerKey,
  ) => {
    if (event.button !== 0 || options.mappingStageTool() !== "select") return;
    invalidatePendingStageRotation();
    const output = options.snapshot().video.outputs.find((candidate) => candidate.id === outputId);
    const svg = (event.currentTarget as SVGElement).ownerSVGElement;
    if (!output || !svg) return;
    event.preventDefault();
    event.stopPropagation();
    options.setSelectedVideoOutputId(outputId);
    const point = stageWorldPointFromPointer(event, svg);
    svg.setPointerCapture(event.pointerId);
    options.setMappingDrag({
      kind: "videoOutputCorner",
      pointerId: event.pointerId,
      outputId,
      corner,
      startWorld: point,
      currentWorld: point,
      startMapping: output.mapping,
    });
  };

  const beginMappingStageObjectDrag = (event: PointerEvent, objectId: number) => {
    if (event.button !== 0 || options.mappingStageTool() !== "select") return;
    invalidatePendingStageRotation();
    const object = options.snapshot().stage_objects.find((candidate) => candidate.id === objectId);
    const svg = (event.currentTarget as SVGElement).ownerSVGElement;
    if (!object || !svg) return;
    event.preventDefault();
    event.stopPropagation();
    options.clearMappingFixtureSelection();
    options.setSelectedVideoOutputId(null);
    options.setSelectedStageObjectId(objectId);
    const point = stageWorldPointFromPointer(event, svg);
    svg.setPointerCapture(event.pointerId);
    options.setMappingDrag({
      kind: "stageObject",
      pointerId: event.pointerId,
      objectId,
      startWorld: point,
      currentWorld: point,
      startObject: object,
    });
  };

  const beginMappingStageObjectRotate = (event: PointerEvent, objectId: number) => {
    if (event.button !== 0 || options.mappingStageTool() !== "select") return;
    invalidatePendingStageRotation();
    const object = options.snapshot().stage_objects.find((candidate) => candidate.id === objectId);
    const svg = (event.currentTarget as SVGElement).ownerSVGElement;
    if (!object || !svg) return;
    event.preventDefault();
    event.stopPropagation();
    options.clearMappingFixtureSelection();
    options.setSelectedVideoOutputId(null);
    options.setSelectedStageObjectId(objectId);
    const point = stageWorldPointFromPointer(event, svg);
    const centerWorld = { x: object.x, z: object.z };
    svg.setPointerCapture(event.pointerId);
    options.setMappingDrag({
      kind: "stageObjectRotate",
      pointerId: event.pointerId,
      objectId,
      startWorld: point,
      currentWorld: point,
      centerWorld,
      startAngleDeg: options.mappingOutputHandleAngleDeg(centerWorld, point),
      startObject: object,
    });
  };

  const beginMappingStageObjectResize = (
    event: PointerEvent,
    objectId: number,
    resizeMode: MappingStageObjectResizeMode,
  ) => {
    if (event.button !== 0 || options.mappingStageTool() !== "select") return;
    invalidatePendingStageRotation();
    const object = options.snapshot().stage_objects.find((candidate) => candidate.id === objectId);
    const svg = (event.currentTarget as SVGElement).ownerSVGElement;
    if (!object || !svg) return;
    event.preventDefault();
    event.stopPropagation();
    options.clearMappingFixtureSelection();
    options.setSelectedVideoOutputId(null);
    options.setSelectedStageObjectId(objectId);
    const point = stageWorldPointFromPointer(event, svg);
    svg.setPointerCapture(event.pointerId);
    options.setMappingDrag({
      kind: "stageObjectResize",
      pointerId: event.pointerId,
      objectId,
      resizeMode,
      startWorld: point,
      currentWorld: point,
      startObject: object,
    });
  };

  const beginMappingViewportPan = (event: PointerEvent & { currentTarget: SVGSVGElement }) => {
    const isMiddleButtonPan = event.button === 1;
    if (!isMiddleButtonPan && (event.button !== 0 || options.mappingStageTool() !== "pan")) return;
    invalidatePendingStageRotation();
    const rect = event.currentTarget.getBoundingClientRect();
    const box = options.mappingViewportBox();
    event.preventDefault();
    event.currentTarget.setPointerCapture(event.pointerId);
    options.setMappingViewportPanDrag({
      pointerId: event.pointerId,
      startClientX: event.clientX,
      startClientY: event.clientY,
      startCenterX: box.x + box.width / 2,
      startCenterZ: box.z + box.height / 2,
      viewBoxWidth: box.width,
      viewBoxHeight: box.height,
      rectWidth: Math.max(1, rect.width),
      rectHeight: Math.max(1, rect.height),
    });
  };

  const updateMappingViewportPan = (event: PointerEvent & { currentTarget: SVGSVGElement }) => {
    const drag = options.mappingViewportPanDrag();
    if (!drag || drag.pointerId !== event.pointerId) return false;
    const deltaX = ((event.clientX - drag.startClientX) / drag.rectWidth) * drag.viewBoxWidth;
    const deltaZ = ((event.clientY - drag.startClientY) / drag.rectHeight) * drag.viewBoxHeight;
    options.setMappingViewport(options.normalizedMappingViewportZoom(), drag.startCenterX - deltaX, drag.startCenterZ - deltaZ);
    return true;
  };

  const finishMappingViewportPan = (event: PointerEvent & { currentTarget: SVGSVGElement }) => {
    const drag = options.mappingViewportPanDrag();
    if (!drag || drag.pointerId !== event.pointerId) return false;
    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }
    options.setMappingViewportPanDrag(null);
    return true;
  };

  const beginMappingMarquee = (event: PointerEvent & { currentTarget: SVGSVGElement }) => {
    if (event.button !== 0 || options.mappingStageTool() !== "select") return;
    invalidatePendingStageRotation();
    const point = stageSvgPointFromPointer(event);
    event.currentTarget.setPointerCapture(event.pointerId);
    options.setMappingMarquee({
      pointerId: event.pointerId,
      start: point,
      current: point,
      additive: options.isAdditiveMappingSelectionEvent(event),
    });
  };

  const handleMappingStagePointerDown = async (
    event: PointerEvent & { currentTarget: SVGSVGElement },
  ) => {
    if (event.button === 1) {
      beginMappingViewportPan(event);
      return;
    }
    if (event.button !== 0) return;
    if (options.mappingStageTool() === "place") await placeSelectedFixtureFromStage(event);
    else if (options.mappingStageTool() === "rotate") await rotateSelectedFixtureFromStage(event);
    else if (options.mappingStageTool() === "pan") beginMappingViewportPan(event);
    else beginMappingMarquee(event);
  };

  const handleMappingStageAuxClick = (
    event: MouseEvent & { currentTarget: SVGSVGElement },
  ) => {
    if (event.button === 1) event.preventDefault();
  };

  const handleMappingStagePointerMove = (event: PointerEvent & { currentTarget: SVGSVGElement }) => {
    const cursorWorld = stageWorldPointFromPointer(event);
    options.setMappingStageCursorWorld(
      options.mappingStageTool() === "place" ? options.snapStagePoint(cursorWorld) : cursorWorld,
    );
    if (updateMappingViewportPan(event)) return;

    const drag = options.mappingDrag();
    if (drag && drag.pointerId === event.pointerId) {
      if (drag.kind === "fixtureYaw" && pendingFixtureYawDrags.has(drag)) return;
      const currentWorld = drag.kind === "fixture" || drag.kind === "fixtureYaw"
        ? fixtureDragWorldPointFromPointer(drag, event, event.currentTarget)
        : cursorWorld;
      options.setMappingDrag({
        ...drag,
        currentWorld,
        ...(
          drag.kind === "fixture" || drag.kind === "fixtureYaw"
            ? { currentClient: { x: event.clientX, y: event.clientY } }
            : {}
        ),
      });
      return;
    }
    const marquee = options.mappingMarquee();
    if (!marquee || marquee.pointerId !== event.pointerId) return;
    options.setMappingMarquee({ ...marquee, current: stageSvgPointFromPointer(event) });
  };

  const finishMappingStageDrag = async (event: PointerEvent & { currentTarget: SVGSVGElement }) => {
    if (finishMappingViewportPan(event)) return;

    const drag = options.mappingDrag();
    if (drag && drag.pointerId === event.pointerId) {
      if (drag.kind === "fixtureYaw" && pendingFixtureYawDrags.has(drag)) return;
      if (event.currentTarget.hasPointerCapture(event.pointerId)) {
        event.currentTarget.releasePointerCapture(event.pointerId);
      }
      const fixtureDragDistance = drag.kind === "fixture" || drag.kind === "fixtureYaw"
        ? Math.hypot(
            drag.currentClient.x - drag.startClient.x,
            drag.currentClient.y - drag.startClient.y,
          )
        : Number.POSITIVE_INFINITY;
      if (drag.kind === "fixtureYaw") {
        if (event.type === "pointercancel") {
          options.setMappingDrag(null);
          return;
        }
        if (!mappingProjectEpochIsCurrent(drag.projectEpoch)) {
          options.setMappingDrag(null);
          return;
        }
        if (fixtureDragDistance < MAPPING_FIXTURE_DRAG_THRESHOLD_PX) {
          options.setMappingDrag(null);
          return;
        }
        const snapshot = options.snapshot();
        const fixturesById = new Map(snapshot.fixtures.map((fixture) => [fixture.id, fixture]));
        const fixtures = drag.fixtureIds.map((fixtureId) => fixturesById.get(fixtureId));
        if (fixtures.some((fixture) => !fixture)) {
          options.setMappingDrag(null);
          return;
        }
        const anchorRotation = drag.startRotations[drag.fixtureId];
        if (!anchorRotation || !Object.values(anchorRotation).every(Number.isFinite)) {
          options.setMappingDrag(null);
          return;
        }
        const yaw = options.mappingFixtureYawFromPoint(drag.centerWorld, drag.currentWorld);
        if (yaw === null || !Number.isFinite(yaw)) {
          options.setMappingDrag(null);
          return;
        }
        const yawDelta = yaw - anchorRotation.yaw;
        const transforms = [] as Array<{
          fixture: PatchedFixtureSummary;
          update: { rotation: PatchFixtureRequest["rotation"] };
        }>;
        for (const fixture of fixtures) {
          if (!fixture) {
            options.setMappingDrag(null);
            return;
          }
          const startRotation = drag.startRotations[fixture.id];
          if (!startRotation || !Object.values(startRotation).every(Number.isFinite)) {
            options.setMappingDrag(null);
            return;
          }
          transforms.push({
            fixture,
            update: { rotation: mappingFixtureRotationWithYawDelta(startRotation, yawDelta) },
          });
        }
        pendingFixtureYawDrags.add(drag);
        try {
          const persisted = await applyMappingFixtureTransformBatch({
            transforms,
            setFixtureTransform: (fixture, update, refresh) => {
              if (options.mappingDrag() !== drag || !mappingProjectEpochIsCurrent(drag.projectEpoch)) {
                return Promise.resolve(false);
              }
              return options.setFixtureTransform(fixture, update, refresh);
            },
            refreshSnapshot: options.refreshSnapshot,
            setMessage: (message) => {
              if (options.mappingDrag() === drag && mappingProjectEpochIsCurrent(drag.projectEpoch)) {
                options.setMessage(message);
              }
            },
          });
          // A cancellation, project replacement, or newer drag may have
          // replaced this lease while the authority round-trip was pending.
          if (options.mappingDrag() !== drag || !mappingProjectEpochIsCurrent(drag.projectEpoch)) return;
          if (persisted) {
            options.setMessage(
              transforms.length === 1
                ? `Set ${transforms[0].fixture.label} yaw to ${yaw} deg.`
                : `Rotated ${transforms.length} selected fixtures by ${yawDelta} deg.`,
            );
          }
        } finally {
          pendingFixtureYawDrags.delete(drag);
          // Clearing after the boolean result lets the confirmed snapshot
          // replace the preview. A false result therefore visibly reverts to
          // the authoritative state as well.
          if (options.mappingDrag() === drag) {
            options.setMappingDrag(null);
          }
        }
        return;
      }
      const delta = options.dragWorldDelta(drag);
      if (
        (drag.kind === "fixture" && fixtureDragDistance < MAPPING_FIXTURE_DRAG_THRESHOLD_PX)
        || (Math.abs(delta.x) < 0.01 && Math.abs(delta.z) < 0.01)
      ) {
        options.setMappingDrag(null);
        return;
      }
      if (drag.kind === "fixture") {
        options.setMappingDrag(null);
        const movedFixtures = options.snapshot().fixtures.filter((candidate) => drag.fixtureIds.includes(candidate.id));
        if (movedFixtures.length === 0) return;
        const transforms = movedFixtures.map((fixture) => {
          const startPosition = drag.startPositions[fixture.id] ?? fixture.position;
          const nextPosition = options.snapStagePosition({
            ...startPosition,
            x: startPosition.x + delta.x,
            z: startPosition.z + delta.z,
          });
          return { fixture, update: { position: nextPosition } };
        });
        if (!await applyMappingFixtureTransformBatch({
          transforms,
          setFixtureTransform: options.setFixtureTransform,
          refreshSnapshot: options.refreshSnapshot,
          setMessage: options.setMessage,
        })) {
          return;
        }
        options.setMessage(
          movedFixtures.length === 1
            ? `Moved ${movedFixtures[0].label}`
            : `Moved ${movedFixtures.length} selected fixtures`,
        );
        return;
      }
      if (options.isMappingStageObjectDrag(drag)) {
        const object = options.snapshot().stage_objects.find((candidate) => candidate.id === drag.objectId);
        if (!object) {
          options.setMappingDrag(null);
          return;
        }
        const nextObject = options.mappingStageObjectPreview(object);
        options.setMappingDrag(null);
        await options.setStageObject(object, {
          x: nextObject.x,
          z: nextObject.z,
          width: nextObject.width,
          depth: nextObject.depth,
          rotation_deg: nextObject.rotation_deg,
        });
        if (drag.kind === "stageObjectRotate") {
          options.setMessage(`Rotated ${object.label} to ${nextObject.rotation_deg} deg.`);
        } else if (drag.kind === "stageObjectResize") {
          options.setMessage(`Resized ${object.label} to ${nextObject.width} x ${nextObject.depth}.`);
        } else {
          options.setMessage(`Moved ${object.label} to X ${nextObject.x}, Z ${nextObject.z}.`);
        }
        return;
      }

      options.setMappingDrag(null);
      const output = options.snapshot().video.outputs.find((candidate) => candidate.id === drag.outputId);
      if (!output) return;
      const nextMapping = options.mappingVideoOutputPreviewMapping(drag, output);
      await options.setVideoOutputMapping(output.id, nextMapping);
      if (drag.kind === "videoOutput") {
        options.setMessage(`Moved ${output.label} to X ${nextMapping.stage_x}, Z ${nextMapping.stage_z}`);
      } else if (drag.kind === "videoOutputRotate") {
        options.setMessage(`Rotated ${output.label} to ${nextMapping.rotation_deg} deg`);
      } else if (drag.kind === "videoOutputCorner") {
        const corner = mappingVideoOutputCorners.find((candidate) => candidate.key === drag.corner);
        options.setMessage(`Adjusted ${corner?.label ?? "corner"} warp for ${output.label}.`);
      } else {
        options.setMessage(`Scaled ${output.label} to ${nextMapping.scale_x.toFixed(2)} x ${nextMapping.scale_y.toFixed(2)}`);
      }
      return;
    }

    const marquee = options.mappingMarquee();
    if (!marquee || marquee.pointerId !== event.pointerId) return;
    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }
    const box = {
      x: Math.min(marquee.start.x, marquee.current.x),
      z: Math.min(marquee.start.z, marquee.current.z),
      width: Math.abs(marquee.current.x - marquee.start.x),
      height: Math.abs(marquee.current.z - marquee.start.z),
    };
    options.setMappingMarquee(null);
    if (box.width < 0.8 && box.height < 0.8) {
      if (!marquee.additive) options.clearMappingFixtureSelection();
      return;
    }
    const pickedIds = options.visualizerFixtures()
      .filter((fixture) =>
        fixture.x >= box.x &&
        fixture.x <= box.x + box.width &&
        fixture.z >= box.z &&
        fixture.z <= box.z + box.height,
      )
      .map((fixture) => fixture.id);
    const nextIds = marquee.additive
      ? [...new Set([...options.selectedMappingFixtureIds(), ...pickedIds])]
      : pickedIds;
    options.setSelectedMappingFixtureIds(nextIds);
    const active = options.snapshot().fixtures.find((fixture) => fixture.id === nextIds[0]);
    if (active) options.activateFixture(active);
    else if (!marquee.additive) options.setSelectedFixtureId(null);
    options.setMessage(`Selected ${nextIds.length} fixture${nextIds.length === 1 ? "" : "s"}`);
  };

  return {
    handleMappingStageWheel,
    beginMappingFixtureDrag,
    beginMappingFixtureYawDrag,
    beginMappingVideoOutputDrag,
    beginMappingVideoOutputRotate,
    beginMappingVideoOutputScale,
    beginMappingVideoOutputCornerDrag,
    beginMappingStageObjectDrag,
    beginMappingStageObjectRotate,
    beginMappingStageObjectResize,
    handleMappingStagePointerDown,
    handleMappingStageAuxClick,
    handleMappingStagePointerMove,
    finishMappingStageDrag,
  };
}
