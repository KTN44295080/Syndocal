import { Show } from "solid-js";
import type { MappingStageTool } from "../mappingViewPresets";
import { MappingBeamsLayer, type MappingBeamFixture } from "./MappingBeamsLayer";
import { MappingFixturesLayer, type MappingFixture2D, type MappingPlacePreview2D } from "./MappingFixturesLayer";
import { MappingGeometryLayer, type MappingGeometryNode2D } from "./MappingGeometryLayer";
import { MappingStageObjectsLayer, type MappingStageObject2D } from "./MappingStageObjectsLayer";
import { MappingVideoSurfacesLayer, type MappingVideoSurface2D } from "./MappingVideoSurfacesLayer";

interface MappingStageLayersPanelProps {
  showStageObjects: boolean;
  showProjectors: boolean;
  showBeams: boolean;
  showGeometry: boolean;
  showLabels: boolean;
  showLevels: boolean;
  stageTool: MappingStageTool;
  stageObjects: MappingStageObject2D[];
  videoSurfaces: MappingVideoSurface2D[];
  beamFixtures: MappingBeamFixture[];
  geometryNodes: MappingGeometryNode2D[];
  fixtures: MappingFixture2D[];
  selectedFixtureIds: Set<number>;
  selectedFixtureId: number | null;
  selectedGroupId: string | null;
  selectedTypeKey: string | null;
  selectedVideoOutputId: number | null;
  placePreview: MappingPlacePreview2D | null;
  isDraggingStageObject: (objectId: number) => boolean;
  isDraggingFixture: (fixtureId: number) => boolean;
  isYawDragging: (fixtureId: number) => boolean;
  onBeginStageObjectDrag: (event: PointerEvent, objectId: number) => void;
  onBeginStageObjectRotate: (event: PointerEvent, objectId: number) => void;
  onBeginStageObjectResize: (event: PointerEvent, objectId: number, axis: "width" | "depth" | "both") => void;
  onSelectVideoOutput: (outputId: number) => void;
  onBeginFixtureYawDrag: (event: PointerEvent, fixtureId: number) => void;
  onFixturePointerDown: (event: PointerEvent, fixtureId: number) => void;
}

export function MappingStageLayersPanel(props: MappingStageLayersPanelProps) {
  return (
    <>
      <Show when={props.showStageObjects}>
        <MappingStageObjectsLayer
          objects={props.stageObjects}
          isDragging={props.isDraggingStageObject}
          onBeginDrag={props.onBeginStageObjectDrag}
          onBeginRotate={props.onBeginStageObjectRotate}
          onBeginResize={props.onBeginStageObjectResize}
        />
      </Show>
      <Show when={props.showProjectors}>
        <MappingVideoSurfacesLayer
          surfaces={props.videoSurfaces}
          selectedVideoOutputId={props.selectedVideoOutputId}
          stageTool={props.stageTool}
          onSelectOutput={props.onSelectVideoOutput}
        />
      </Show>
      <Show when={props.showBeams}>
        <MappingBeamsLayer fixtures={props.beamFixtures} />
      </Show>
      <Show when={props.showGeometry}>
        <MappingGeometryLayer nodes={props.geometryNodes} showLabels={props.showLabels} />
      </Show>
      <MappingFixturesLayer
        fixtures={props.fixtures}
        selectedFixtureIds={props.selectedFixtureIds}
        selectedFixtureId={props.selectedFixtureId}
        selectedGroupId={props.selectedGroupId}
        selectedTypeKey={props.selectedTypeKey}
        showLabels={props.showLabels}
        showLevels={props.showLevels}
        placePreview={props.placePreview}
        isDragging={props.isDraggingFixture}
        isYawDragging={props.isYawDragging}
        onBeginYawDrag={props.onBeginFixtureYawDrag}
        onFixturePointerDown={props.onFixturePointerDown}
      />
    </>
  );
}
