import { createMemo, Show } from "solid-js";
import type { MappingStageTool } from "../mappingViewPresets";
import type { StageLabelViewport } from "../stageLabelLayout";
import { stageWorldPerCssPixel } from "../stageOverlayLayout";
import { MappingBeamsLayer, type MappingBeamFixture } from "./MappingBeamsLayer";
import { MappingFixturesLayer, type MappingFixture2D, type MappingPlacePreview2D } from "./MappingFixturesLayer";
import { MappingGeometryLayer, type MappingGeometryNode2D } from "./MappingGeometryLayer";
import {
  MappingStageObjectHandlesLayer,
  MappingStageObjectsLayer,
  type MappingStageObject2D,
} from "./MappingStageObjectsLayer";
import { MappingVideoSurfacesLayer, type MappingVideoSurface2D } from "./MappingVideoSurfacesLayer";

interface MappingStageLayersPanelProps {
  readOnly?: boolean;
  fixtureTransformsEditable?: boolean;
  blindActive?: boolean;
  showStageObjects: boolean;
  showProjectors: boolean;
  showBeams: boolean;
  showGeometry: boolean;
  showLabels: boolean;
  labelViewport: StageLabelViewport;
  labelViewportPixelSize: { width: number; height: number };
  labelZoom: number;
  showLevels: boolean;
  stageTool: MappingStageTool;
  stageObjects: MappingStageObject2D[];
  videoSurfaces: MappingVideoSurface2D[];
  beamFixtures: MappingBeamFixture[];
  geometryNodes: MappingGeometryNode2D[];
  fixtures: MappingFixture2D[];
  labelFixtures?: MappingFixture2D[];
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
  const screenWorldPerCssPixel = createMemo(() =>
    stageWorldPerCssPixel(props.labelViewport, props.labelViewportPixelSize));
  const blindWatermark = createMemo(() => {
    const unit = screenWorldPerCssPixel();
    const fontSize = unit * 28;
    return {
      x: props.labelViewport.x + props.labelViewport.width / 2,
      y: props.labelViewport.z + props.labelViewport.height / 2,
      fontSize,
      width: fontSize * 3.7,
      height: fontSize * 1.55,
      radius: unit * 5,
      strokeWidth: unit * 1.5,
    };
  });

  return (
    <>
      <Show when={props.showStageObjects}>
        <MappingStageObjectsLayer
          objects={props.stageObjects}
          readOnly={props.readOnly}
          worldPerCssPixel={screenWorldPerCssPixel()}
          isDragging={props.isDraggingStageObject}
          onBeginDrag={props.onBeginStageObjectDrag}
        />
      </Show>
      <Show when={props.showProjectors}>
        <MappingVideoSurfacesLayer
          surfaces={props.videoSurfaces}
          selectedVideoOutputId={props.selectedVideoOutputId}
          stageTool={props.stageTool}
          worldPerCssPixel={screenWorldPerCssPixel()}
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
        labelFixtures={props.labelFixtures}
        selectedFixtureIds={props.selectedFixtureIds}
        selectedFixtureId={props.selectedFixtureId}
        selectedGroupId={props.selectedGroupId}
        selectedTypeKey={props.selectedTypeKey}
        showLabels={props.showLabels}
        labelViewport={props.labelViewport}
        worldPerCssPixel={screenWorldPerCssPixel()}
        labelZoom={props.labelZoom}
        showLevels={props.showLevels}
        placePreview={props.placePreview}
        readOnly={props.readOnly}
        fixtureTransformsEditable={props.fixtureTransformsEditable}
        stageTool={props.stageTool}
        isDragging={props.isDraggingFixture}
        isYawDragging={props.isYawDragging}
        onBeginYawDrag={props.onBeginFixtureYawDrag}
        onFixturePointerDown={props.onFixturePointerDown}
      />
      <Show when={props.showStageObjects}>
        <MappingStageObjectHandlesLayer
          objects={props.stageObjects}
          readOnly={props.readOnly}
          worldPerCssPixel={screenWorldPerCssPixel()}
          onBeginRotate={props.onBeginStageObjectRotate}
          onBeginResize={props.onBeginStageObjectResize}
        />
      </Show>
      <Show when={props.blindActive}>
        <g
          class="mappingBlindWatermark"
          data-mapping-blind-watermark
          aria-hidden="true"
          pointer-events="none"
        >
          <rect
            x={blindWatermark().x - blindWatermark().width / 2}
            y={blindWatermark().y - blindWatermark().height / 2}
            width={blindWatermark().width}
            height={blindWatermark().height}
            rx={blindWatermark().radius}
            stroke-width={blindWatermark().strokeWidth}
          />
          <text
            x={blindWatermark().x}
            y={blindWatermark().y}
            font-size={String(blindWatermark().fontSize)}
            data-no-localize
          >
            BLIND
          </text>
        </g>
      </Show>
    </>
  );
}
