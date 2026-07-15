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

// Compact, Daslight-grade stage glyph footprints (stage units on the 100-unit viewBox).
// Type-differentiated so par/wash squares, moving-head circles, and bars/strips read
// distinctly while staying small enough to stay legible at 2,000 fixtures.
export const mappingFixtureStageSize = (visualKind: MappingFixtureVisualKind) =>
  visualKind === "bar"
    ? { width: 5.2, height: 1.0 }
    : visualKind === "panel"
      ? { width: 3.8, height: 2.6 }
      : visualKind === "laser"
        ? { width: 2.6, height: 2.6 }
        : visualKind === "moving"
          ? { width: 2.6, height: 2.6 }
          : visualKind === "point"
            ? { width: 2.2, height: 2.2 }
            : { width: 2.4, height: 2.4 };

export const fixtureTypeKey = (fixture: PatchedFixtureSummary) =>
  `${fixture.manufacturer}::${fixture.profile_name}::${fixture.mode_name}`;

export const fixtureTypeLabel = (fixture: PatchedFixtureSummary) => `${fixture.profile_name} / ${fixture.mode_name}`;

export const mappingTypeGlyphClass = (kind: MappingFixtureVisualKind) => `mappingTypeGlyph kind-${kind}`;
