// Fixture visual-kind classification and mapping-stage label/size helpers extracted from App.tsx.
// Pure functions over PatchedFixtureSummary; no SolidJS/state deps.
import type { PatchedFixtureSummary } from "./types";
import { stagePadding, stageViewBoxSize, type StageWorldBounds } from "./stageGeometry";

export type MappingFixtureVisualKind = "point" | "moving" | "bar" | "panel" | "laser" | "par";

export const fixtureVisualKind = (fixture: PatchedFixtureSummary): MappingFixtureVisualKind => {
  const text = `${fixture.manufacturer} ${fixture.profile_name} ${fixture.mode_name} ${fixture.label}`.toLowerCase();
  const attributes = fixture.controls.map((control) => control.attribute.toLowerCase()).join(" ");
  if (/(laser)/.test(text)) {
    return "laser";
  }
  if (/(matrix|panel|pixel)/.test(text)) {
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
  const drawableSize = stageViewBoxSize - stagePadding * 2;
  const scaleX = drawableSize / Math.max(Number.EPSILON, bounds.maxX - bounds.minX);
  const scaleZ = drawableSize / Math.max(Number.EPSILON, bounds.maxZ - bounds.minZ);
  return Math.min(1, scaleX, scaleZ);
};

export const mappingFixtureGridStageSize = (bounds: StageWorldBounds) =>
  mappingFixtureGridUnit * mappingFixtureWorldToSvgScale(bounds);

export const mappingFixtureStageSize = (
  _visualKind: MappingFixtureVisualKind,
  segmentCount = 1,
  worldToSvgScale = 1,
) => ({
  width: mappingFixtureGridUnit * worldToSvgScale * Math.max(1, Math.floor(segmentCount)),
  height: mappingFixtureGridUnit * worldToSvgScale,
});

export const fixtureTypeKey = (fixture: PatchedFixtureSummary) =>
  `${fixture.manufacturer}::${fixture.profile_name}::${fixture.mode_name}`;

export const fixtureTypeLabel = (fixture: PatchedFixtureSummary) => `${fixture.profile_name} / ${fixture.mode_name}`;

export const mappingTypeGlyphClass = (kind: MappingFixtureVisualKind) => `mappingTypeGlyph kind-${kind}`;
