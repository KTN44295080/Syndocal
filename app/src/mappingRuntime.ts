import type {
  MappingFixtureSegmentOrder,
  MappingFixtureVisualKind,
} from "./fixtureVisuals";
import { clampRange, finiteOr } from "./numericHelpers";
import type {
  GeometrySummary,
  PatchFixtureRequest,
  StageObjectKind,
  StageObjectSummary,
  VideoOutputMapping,
  VideoOutputSummary,
} from "./types";
import type { MappingVideoOutputCornerKey } from "./videoOutputMapping";

export type MappingAxis = "x" | "z";
export type MappingStageObjectResizeMode = "width" | "depth" | "both";
export type MappingBulkGroupMode = "add" | "remove" | "set";
export type WaveStageDragMode = "origin" | "direction" | "videoTarget";

export interface VisualizerFixtureBeam {
  cellIndex: number;
  points: string;
  intensity: number;
  color: string;
}

export interface MappingFixturePhysicalCell2d {
  beamIndex: number;
  x: number;
  z: number;
  width: number;
  height: number;
  logicalSegmentIndex: number | null;
}

export interface VisualizerFixture {
  id: number;
  label: string;
  addressOrder: number;
  dmxLabel: string;
  groupLabel: string;
  typeKey: string;
  visualKind: MappingFixtureVisualKind;
  x: number;
  z: number;
  width: number;
  height: number;
  segmentColumns: number;
  segmentRows: number;
  segmentOrder: MappingFixtureSegmentOrder;
  yaw: number;
  beamShape: import("./mappingFixtureBeam").MappingBeamShape | null;
  segmentBeamShapes: Array<import("./mappingFixtureBeam").MappingBeamShape | null>;
  beamDescription?: string;
  beamPoints: string;
  beams?: VisualizerFixtureBeam[];
  physicalCells?: MappingFixturePhysicalCell2d[];
  intensity: number;
  color: string;
  inGroupFilter: boolean;
  highlighted: boolean;
  soloed: boolean;
  parked: boolean;
}

export interface VisualizerVideoSurface2d {
  id: number;
  label: string;
  x: number;
  z: number;
  width: number;
  height: number;
  rotationDeg: number;
  opacity: number;
  active: boolean;
}

export interface VisualizerStageObject2d {
  id: number;
  label: string;
  kind: StageObjectKind;
  x: number;
  z: number;
  width: number;
  depth: number;
  rotationDeg: number;
  color: string;
  selected: boolean;
}

export interface MappingGeometryNode2d {
  key: string;
  fixtureId: number;
  fixtureLabel: string;
  name: string;
  kind: string;
  x: number;
  z: number;
  radius: number;
  footprintWidth: number;
  footprintHeight: number;
  mappedChannelCount: number;
  className: string;
  inGroupFilter: boolean;
  selected: boolean;
}

export interface MappingFixtureTypeRow {
  key: string;
  label: string;
  manufacturer: string;
  mode: string;
  visualKind: MappingFixtureVisualKind;
  count: number;
}

export type MappingDragState =
  | {
      kind: "fixture";
      pointerId: number;
      fixtureIds: number[];
      startWorld: { x: number; z: number };
      currentWorld: { x: number; z: number };
      startClient: { x: number; y: number };
      currentClient: { x: number; y: number };
      startPositions: Record<number, PatchFixtureRequest["position"]>;
    }
  | {
      kind: "fixtureYaw";
      pointerId: number;
      fixtureId: number;
      fixtureIds: number[];
      startRotations: Record<number, PatchFixtureRequest["rotation"]>;
      projectEpoch: number;
      startWorld: { x: number; z: number };
      currentWorld: { x: number; z: number };
      startClient: { x: number; y: number };
      currentClient: { x: number; y: number };
      centerWorld: { x: number; z: number };
    }
  | {
      kind: "videoOutput";
      pointerId: number;
      outputId: number;
      startWorld: { x: number; z: number };
      currentWorld: { x: number; z: number };
      startMapping: VideoOutputMapping;
    }
  | {
      kind: "videoOutputRotate";
      pointerId: number;
      outputId: number;
      startWorld: { x: number; z: number };
      currentWorld: { x: number; z: number };
      centerWorld: { x: number; z: number };
      startAngleDeg: number;
      startMapping: VideoOutputMapping;
    }
  | {
      kind: "videoOutputScale";
      pointerId: number;
      outputId: number;
      startWorld: { x: number; z: number };
      currentWorld: { x: number; z: number };
      centerWorld: { x: number; z: number };
      startDistance: number;
      startMapping: VideoOutputMapping;
    }
  | {
      kind: "videoOutputCorner";
      pointerId: number;
      outputId: number;
      corner: MappingVideoOutputCornerKey;
      startWorld: { x: number; z: number };
      currentWorld: { x: number; z: number };
      startMapping: VideoOutputMapping;
    }
  | {
      kind: "stageObject";
      pointerId: number;
      objectId: number;
      startWorld: { x: number; z: number };
      currentWorld: { x: number; z: number };
      startObject: StageObjectSummary;
    }
  | {
      kind: "stageObjectRotate";
      pointerId: number;
      objectId: number;
      startWorld: { x: number; z: number };
      currentWorld: { x: number; z: number };
      centerWorld: { x: number; z: number };
      startAngleDeg: number;
      startObject: StageObjectSummary;
    }
  | {
      kind: "stageObjectResize";
      pointerId: number;
      objectId: number;
      resizeMode: MappingStageObjectResizeMode;
      startWorld: { x: number; z: number };
      currentWorld: { x: number; z: number };
      startObject: StageObjectSummary;
    };

export const normalizeMappingFixtureYaw = (yaw: number) =>
  Number((((yaw % 360) + 360) % 360).toFixed(1));

export const mappingFixtureRotationWithYawDelta = (
  rotation: PatchFixtureRequest["rotation"],
  yawDelta: number,
): PatchFixtureRequest["rotation"] => ({
  ...rotation,
  yaw: normalizeMappingFixtureYaw(rotation.yaw + yawDelta),
});

export interface MappingMarqueeState {
  pointerId: number;
  start: { x: number; z: number };
  current: { x: number; z: number };
  additive: boolean;
}

export interface MappingViewportPanDragState {
  pointerId: number;
  startClientX: number;
  startClientY: number;
  startCenterX: number;
  startCenterZ: number;
  viewBoxWidth: number;
  viewBoxHeight: number;
  rectWidth: number;
  rectHeight: number;
}

export interface MappingSnapLine {
  axis: MappingAxis;
  svg: number;
}

export interface MappingSvgBounds {
  minX: number;
  maxX: number;
  minZ: number;
  maxZ: number;
}

export const surfaceWorldHalfSize = (output: VideoOutputSummary, mapping = output.mapping) => {
  const outputAspect = output.height > 0 ? output.width / output.height : 1;
  const mappedAspect = clampRange(finiteOr(mapping.aspect_ratio, outputAspect), 0.35, 4);
  const aspect = mapping.aspect_mode === "Stretch" ? outputAspect : mappedAspect;
  const baseHeight = 4.5 * clampRange(mapping.scale_y, 0.25, 3);
  return {
    width: baseHeight * Math.max(0.35, aspect) * clampRange(mapping.scale_x, 0.25, 3),
    height: baseHeight,
  };
};

export const mappingGeometryClass = (
  geometry: GeometrySummary,
  mappedChannelCount: number,
  inGroupFilter: boolean,
  selected: boolean,
) => {
  const text = `${geometry.kind} ${geometry.model_primitive ?? ""} ${geometry.model_file ?? ""} ${geometry.beam_type ?? ""}`.toLowerCase();
  const baseClass = text.includes("beam") ? "beam" : text.includes("axis") ? "axis" : "body";
  const primitive = geometry.model_primitive?.toLowerCase();
  const meshClass =
    geometry.model_file && geometry.model_file.trim().length > 0
      ? "mesh-mesh"
      : primitive === "cylinder" || primitive === "sphere" || primitive === "plane"
        ? `mesh-${primitive}`
        : "";
  return [
    "stageGeometryNode",
    baseClass,
    meshClass,
    mappedChannelCount > 0 ? "mapped" : "",
    selected ? "selected" : "",
    inGroupFilter ? "" : "muted",
  ].filter(Boolean).join(" ");
};
