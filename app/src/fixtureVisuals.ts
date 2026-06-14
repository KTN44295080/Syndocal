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

export const mappingFixtureStageSize = (visualKind: MappingFixtureVisualKind) =>
  visualKind === "bar"
    ? { width: 5.8, height: 1.4 }
    : visualKind === "panel"
      ? { width: 4.8, height: 3.2 }
      : visualKind === "laser"
        ? { width: 3.4, height: 3.4 }
        : { width: 3.2, height: 3.2 };

export const fixtureTypeKey = (fixture: PatchedFixtureSummary) =>
  `${fixture.manufacturer}::${fixture.profile_name}::${fixture.mode_name}`;

export const fixtureTypeLabel = (fixture: PatchedFixtureSummary) => `${fixture.profile_name} / ${fixture.mode_name}`;

export const mappingTypeGlyphClass = (kind: MappingFixtureVisualKind) => `mappingTypeGlyph kind-${kind}`;
