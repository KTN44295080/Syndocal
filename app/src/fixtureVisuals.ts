// Fixture visual-kind classification and mapping-stage label/size helpers extracted from App.tsx.
// Pure functions over PatchedFixtureSummary; no SolidJS/state deps.
import type { PatchedFixtureSummary } from "./types";

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

// The stage minor grid is five SVG units. A fixture owns one complete grid cell,
// and multi-cell fixtures extend by one adjacent cell per independently rendered
// segment. visualKind remains part of the signature for call-site readability;
// facing marks and CSS still provide the type distinction.
export const mappingFixtureGridUnit = 5;
export const mappingFixtureCellGap = 0.6;
export const mappingFixtureCellInkSize = mappingFixtureGridUnit - mappingFixtureCellGap;

export const mappingFixtureStageSize = (
  _visualKind: MappingFixtureVisualKind,
  segmentCount = 1,
) => ({
  width: mappingFixtureGridUnit * Math.max(1, Math.floor(segmentCount)),
  height: mappingFixtureGridUnit,
});

export const fixtureTypeKey = (fixture: PatchedFixtureSummary) =>
  `${fixture.manufacturer}::${fixture.profile_name}::${fixture.mode_name}`;

export const fixtureTypeLabel = (fixture: PatchedFixtureSummary) => `${fixture.profile_name} / ${fixture.mode_name}`;

export const mappingTypeGlyphClass = (kind: MappingFixtureVisualKind) => `mappingTypeGlyph kind-${kind}`;
