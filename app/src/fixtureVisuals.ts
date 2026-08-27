// Fixture visual-kind classification and mapping-stage label/size helpers extracted from App.tsx.
// Pure functions over PatchedFixtureSummary; no SolidJS/state deps.
import type { PatchedFixtureSummary } from "./types";
import { mappingStageSvgFrame, type StageWorldBounds } from "./stageGeometry";

export type MappingFixtureVisualKind = "point" | "moving" | "bar" | "panel" | "laser" | "par";

export interface MappingFixtureSegmentGrid {
  columns: number;
  rows: number;
}

export interface MappingFixtureSegmentCell {
  column: number;
  row: number;
}

export type MappingFixtureSegmentOrder =
  | "row-major-top-left"
  | "column-major-bottom-left";

const fixtureIdentityText = (fixture: PatchedFixtureSummary) =>
  `${fixture.manufacturer} ${fixture.profile_name} ${fixture.mode_name} ${fixture.label}`.toLowerCase();

export const isSoundWavesStrongpointFixture = (fixture: PatchedFixtureSummary) =>
  /(?:960\s*)?sound\s*waves\s*strongpoint/.test(fixtureIdentityText(fixture));

export const fixtureVisualKind = (fixture: PatchedFixtureSummary): MappingFixtureVisualKind => {
  const text = fixtureIdentityText(fixture);
  const attributes = fixture.controls.map((control) => control.attribute.toLowerCase()).join(" ");
  if (/(laser)/.test(text)) {
    return "laser";
  }
  if (isSoundWavesStrongpointFixture(fixture) || /(matrix|panel|pixel)/.test(text)) {
    return "panel";
  }
  if (/(bar|strip|batten|tube|linear)/.test(text)) {
    return "bar";
  }
  if (/(pan|tilt)/.test(attributes)) {
    return "moving";
  }
  if (/(par|wash|rgb|rgba|rgbw|led)/.test(text) || /(red|green|blue|amber|white|uv)/.test(attributes)) {
    return "par";
  }
  return "point";
};

// A standard fixture is five stage-world units, capped at one existing five-SVG-
// unit minor-grid cell on compact stages. Broad imported stages scale the glyph
// down with their world bounds so fixture spacing and glyph size stay comparable.
// Multi-cell fixtures extend by one adjacent cell per rendered segment.
export const mappingFixtureGridUnit = 5;
export const mappingFixtureCellGap = 0.6;
export const mappingFixtureCellInkSize = mappingFixtureGridUnit - mappingFixtureCellGap;

export const mappingFixtureWorldToSvgScale = (bounds: StageWorldBounds) => {
  return mappingStageSvgFrame(bounds).worldToSvgScale;
};

export const mappingFixtureGridStageSize = (bounds: StageWorldBounds) =>
  mappingFixtureGridUnit * mappingFixtureWorldToSvgScale(bounds);

export const mappingFixtureSegmentGrid = (
  fixture: PatchedFixtureSummary,
  segmentCount = 1,
): MappingFixtureSegmentGrid => {
  const count = Math.max(1, Math.floor(segmentCount));
  if (isSoundWavesStrongpointFixture(fixture) && count >= 4 && count % 4 === 0) {
    return { columns: 4, rows: count / 4 };
  }
  return { columns: count, rows: 1 };
};

export const mappingFixtureSegmentOrder = (
  fixture: PatchedFixtureSummary,
): MappingFixtureSegmentOrder => isSoundWavesStrongpointFixture(fixture)
  ? "column-major-bottom-left"
  : "row-major-top-left";

export const mappingFixtureSegmentCell = (
  grid: MappingFixtureSegmentGrid,
  index: number,
  order: MappingFixtureSegmentOrder = "row-major-top-left",
): MappingFixtureSegmentCell => {
  const columns = Math.max(1, Math.floor(grid.columns));
  const rows = Math.max(1, Math.floor(grid.rows));
  const safeIndex = Math.max(0, Math.floor(index));
  if (order === "column-major-bottom-left") {
    return {
      column: Math.floor(safeIndex / rows),
      row: rows - 1 - (safeIndex % rows),
    };
  }
  return {
    column: safeIndex % columns,
    row: Math.floor(safeIndex / columns),
  };
};

export const mappingFixtureStageSize = (
  _visualKind: MappingFixtureVisualKind,
  segmentColumns = 1,
  worldToSvgScale = 1,
  segmentRows = 1,
) => ({
  width: mappingFixtureGridUnit * worldToSvgScale * Math.max(1, Math.floor(segmentColumns)),
  height: mappingFixtureGridUnit * worldToSvgScale * Math.max(1, Math.floor(segmentRows)),
});

export const fixtureTypeKey = (fixture: PatchedFixtureSummary) =>
  `${fixture.manufacturer}::${fixture.profile_name}::${fixture.mode_name}`;

export const fixtureTypeLabel = (fixture: PatchedFixtureSummary) => `${fixture.profile_name} / ${fixture.mode_name}`;

export const mappingTypeGlyphClass = (kind: MappingFixtureVisualKind) => `mappingTypeGlyph kind-${kind}`;
