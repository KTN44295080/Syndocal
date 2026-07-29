import { colorCandidates } from "./fixtureControlRuntime";
import {
  fixtureTypeKey,
  fixtureTypeLabel,
  fixtureVisualKind,
  type MappingFixtureVisualKind,
} from "./fixtureVisuals";
import type { AttributeControl, PatchedFixtureSummary } from "./types";
import { controlCategoryForAttribute, type ControlCategory } from "./uiModes";

export interface FixtureTypeSelectionGroup {
  key: string;
  label: string;
  manufacturer: string;
  mode: string;
  visualKind: MappingFixtureVisualKind;
  fixtures: PatchedFixtureSummary[];
}

export interface FixtureTypeColorControls {
  red: AttributeControl;
  green: AttributeControl;
  blue: AttributeControl;
}

export interface FixtureTypePositionControls {
  pan: AttributeControl;
  tilt: AttributeControl;
  usesFixtureLimits: boolean;
}

const normalizedAttribute = (attribute: string) => attribute.toLowerCase();

export const fixtureControlForAttribute = (
  fixture: PatchedFixtureSummary,
  attribute: string,
) => {
  const normalized = normalizedAttribute(attribute);
  return fixture.controls.find((control) => normalizedAttribute(control.attribute) === normalized);
};

export const groupPickedFixturesByType = (
  fixtures: readonly PatchedFixtureSummary[],
): FixtureTypeSelectionGroup[] => {
  const groups = new Map<string, FixtureTypeSelectionGroup>();
  for (const fixture of fixtures) {
    const key = fixtureTypeKey(fixture);
    const current = groups.get(key);
    if (current) {
      current.fixtures.push(fixture);
      continue;
    }
    groups.set(key, {
      key,
      label: fixtureTypeLabel(fixture),
      manufacturer: fixture.manufacturer,
      mode: fixture.mode_name,
      visualKind: fixtureVisualKind(fixture),
      fixtures: [fixture],
    });
  }
  return [...groups.values()].sort((left, right) =>
    left.label.localeCompare(right.label) ||
    left.manufacturer.localeCompare(right.manufacturer) ||
    left.key.localeCompare(right.key)
  );
};

export const commonFixtureTypeControls = (
  group: FixtureTypeSelectionGroup,
): AttributeControl[] => {
  const reference = group.fixtures[0];
  if (!reference) {
    return [];
  }
  return reference.controls.filter((control) =>
    group.fixtures.every((fixture) => Boolean(fixtureControlForAttribute(fixture, control.attribute)))
  );
};

export const fixtureTypeControlsForCategory = (
  group: FixtureTypeSelectionGroup,
  category: ControlCategory,
) => {
  const controls = commonFixtureTypeControls(group);
  return category === "fader"
    ? controls
    : controls.filter((control) => controlCategoryForAttribute(control.attribute) === category);
};

const controlForCandidateNames = (
  controls: readonly AttributeControl[],
  names: readonly string[],
) => {
  const candidates = new Set(names.map(normalizedAttribute));
  return controls.find((control) => candidates.has(normalizedAttribute(control.attribute)));
};

export const fixtureTypeDimmerControl = (
  group: FixtureTypeSelectionGroup,
) => {
  const controls = fixtureTypeControlsForCategory(group, "dimmer");
  return controlForCandidateNames(controls, ["Dimmer", "Intensity", "MasterIntensity"]) ?? controls[0];
};

export const fixtureTypeColorControls = (
  group: FixtureTypeSelectionGroup,
): FixtureTypeColorControls | null => {
  const controls = fixtureTypeControlsForCategory(group, "color");
  const red = controlForCandidateNames(controls, colorCandidates.red);
  const green = controlForCandidateNames(controls, colorCandidates.green);
  const blue = controlForCandidateNames(controls, colorCandidates.blue);
  return red && green && blue ? { red, green, blue } : null;
};

const axisTokens = (control: AttributeControl) =>
  `${control.attribute} ${control.channel_name}`
    .toLowerCase()
    .split(/[^a-z0-9]+/)
    .filter(Boolean);

const controlForPositionAxis = (
  controls: readonly AttributeControl[],
  axis: "pan" | "tilt",
) => {
  const exact = controlForCandidateNames(controls, [axis]);
  if (exact) {
    return exact;
  }
  const coordinate = axis === "pan" ? "x" : "y";
  const orientation = axis === "pan" ? "horizontal" : "vertical";
  return controls.find((control) => {
    const tokens = axisTokens(control);
    return (
      tokens.includes(axis) ||
      tokens.includes(orientation) ||
      (
        tokens.includes(coordinate) &&
        tokens.some((token) => ["axis", "move", "movement", "position"].includes(token))
      )
    );
  });
};

export const fixtureTypePositionControls = (
  group: FixtureTypeSelectionGroup,
): FixtureTypePositionControls | null => {
  const controls = fixtureTypeControlsForCategory(group, "position");
  const pan = controlForPositionAxis(controls, "pan");
  const tilt = controlForPositionAxis(controls, "tilt");
  if (pan && tilt && pan !== tilt) {
    return {
      pan,
      tilt,
      usesFixtureLimits:
        normalizedAttribute(pan.attribute) === "pan" &&
        normalizedAttribute(tilt.attribute) === "tilt",
    };
  }
  if (controls.length < 2) {
    return null;
  }
  return {
    pan: controls[0],
    tilt: controls[1],
    usesFixtureLimits: false,
  };
};
