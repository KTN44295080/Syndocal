import type { AttributeControl, PatchedFixtureSummary } from "./types";

export type ColorExtraChannelKey = "white" | "amber" | "uv";

export interface ColorExtraControl {
  key: ColorExtraChannelKey;
  label: string;
  shortLabel: string;
  attribute: string;
  value: number;
}

export interface ColorControlSet {
  red: string;
  green: string;
  blue: string;
  redValue: number;
  greenValue: number;
  blueValue: number;
  extras: ColorExtraControl[];
  value: string;
}

export interface PositionControlSet {
  pan: string;
  tilt: string;
  panValue: number;
  tiltValue: number;
}

export interface DimmerControlSet {
  attribute: string;
  value: number;
}

export interface TouchDimmerQuickEntry {
  fixtureId: number;
  label: string;
  attribute: string;
  value: number;
  outValue: number;
  halfValue: number;
  fullValue: number;
}

export interface TouchDimmerQuickTarget {
  kind: "fixture" | "group";
  label: string;
  entries: TouchDimmerQuickEntry[];
}

export interface TouchDimmerRestoreState {
  label: string;
  entries: Pick<TouchDimmerQuickEntry, "fixtureId" | "attribute" | "value">[];
}

export interface MovementLimitPoint {
  pan: number;
  tilt: number;
}

export interface MovementLimitDragState {
  anchor: MovementLimitPoint;
}

export const colorCandidates = {
  red: ["ColorRed", "Red", "ColorAdd_R"],
  green: ["ColorGreen", "Green", "ColorAdd_G"],
  blue: ["ColorBlue", "Blue", "ColorAdd_B"],
  white: [
    "ColorWhite",
    "White",
    "WarmWhite",
    "ColdWhite",
    "CoolWhite",
    "ColorAdd_W",
    "ColorAdd_WW",
    "ColorAdd_CW",
  ],
  amber: ["ColorAmber", "Amber", "ColorAdd_A"],
  uv: ["ColorUv", "ColorUV", "Uv", "UV", "Ultraviolet", "ColorAdd_UV", "Generic: UV"],
} as const;

export const findControlAttribute = (fixture: PatchedFixtureSummary, names: readonly string[]) => {
  const normalizedNames = names.map((name) => name.toLowerCase());
  return fixture.controls.find((control) => normalizedNames.includes(control.attribute.toLowerCase()))?.attribute;
};

export const findControlAttributeInControls = (controls: AttributeControl[], names: readonly string[]) => {
  const normalizedNames = names.map((name) => name.toLowerCase());
  return controls.find((control) => normalizedNames.includes(control.attribute.toLowerCase()))?.attribute;
};

export const bulkPatchLabel = (baseLabel: string, index: number, count: number) => {
  if (count === 1) {
    return baseLabel;
  }
  const match = baseLabel.match(/^(.*?)(\d+)$/);
  if (match) {
    return `${match[1]}${Number(match[2]) + index}`;
  }
  return `${baseLabel} ${index + 1}`;
};

export const readFixtureAttribute = (
  fixture: PatchedFixtureSummary,
  currentValues: Record<string, number>,
  names: readonly string[],
) => {
  const attributeValues = new Map(
    fixture.attribute_values.map((value) => [value.attribute.toLowerCase(), value.value]),
  );
  for (const name of names) {
    const directValue = currentValues[`${fixture.id}:${name}`];
    if (directValue !== undefined) {
      return directValue;
    }
    const snapshotValue = attributeValues.get(name.toLowerCase());
    if (snapshotValue !== undefined) {
      return snapshotValue;
    }
  }
  return undefined;
};
