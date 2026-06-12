import type { AttributeControl, AttributeResolution, GeometrySummary } from "./types";

export interface CustomProfileAttributePreview {
  attribute: string;
  resolution: AttributeResolution;
  startOffset: number | null;
  offsets: number[];
}

export interface CustomProfileAttributeDraft {
  attribute: string;
  resolution: AttributeResolution;
  startOffset: string;
}

export interface CustomProfileAttributeTemplate {
  label: string;
  rows: CustomProfileAttributeDraft[];
}

export interface ProfileGeometryRow {
  geometry: GeometrySummary;
  depth: number;
  controlCount: number;
  positionLabel: string;
}

export interface CustomProfilePreview {
  controls: CustomProfileAttributePreview[];
  footprint: number;
  errors: string[];
}

export const customProfileAttributeTemplates: CustomProfileAttributeTemplate[] = [
  {
    label: "Dimmer",
    rows: [{ attribute: "Dimmer", resolution: "EightBit", startOffset: "" }],
  },
  {
    label: "Pan/Tilt",
    rows: [
      { attribute: "Pan", resolution: "SixteenBit", startOffset: "" },
      { attribute: "Tilt", resolution: "SixteenBit", startOffset: "" },
    ],
  },
  {
    label: "RGB",
    rows: [
      { attribute: "ColorRed", resolution: "EightBit", startOffset: "" },
      { attribute: "ColorGreen", resolution: "EightBit", startOffset: "" },
      { attribute: "ColorBlue", resolution: "EightBit", startOffset: "" },
    ],
  },
  {
    label: "RGBW",
    rows: [
      { attribute: "ColorRed", resolution: "EightBit", startOffset: "" },
      { attribute: "ColorGreen", resolution: "EightBit", startOffset: "" },
      { attribute: "ColorBlue", resolution: "EightBit", startOffset: "" },
      { attribute: "ColorWhite", resolution: "EightBit", startOffset: "" },
    ],
  },
  {
    label: "Beam",
    rows: [
      { attribute: "Shutter1", resolution: "EightBit", startOffset: "" },
      { attribute: "Dimmer", resolution: "EightBit", startOffset: "" },
      { attribute: "Zoom", resolution: "EightBit", startOffset: "" },
      { attribute: "Focus", resolution: "EightBit", startOffset: "" },
    ],
  },
  {
    label: "Wheels",
    rows: [
      { attribute: "ColorWheel1", resolution: "EightBit", startOffset: "" },
      { attribute: "Gobo1", resolution: "EightBit", startOffset: "" },
    ],
  },
];

export const normalizeCustomResolutionText = (value: string) => value.toLowerCase().replace(/[-_\s]/g, "");

const parseCustomAttributeStartOffset = (attribute: string, rawStart: string) => {
  const startOffset = Number(rawStart);
  if (!Number.isInteger(startOffset) || startOffset < 1 || startOffset > 512) {
    return {
      startOffset: null,
      error: `Invalid start channel '${rawStart}' for ${attribute || "attribute"}`,
    };
  }
  return { startOffset, error: null };
};

const parseCustomAttributePreviewSpec = (value: string) => {
  const trimmed = value.trim();
  let attribute = trimmed;
  let rawResolution = "";
  let startOffset: number | null = null;
  let startError: string | null = null;

  const channelSeparatorIndex = trimmed.lastIndexOf("@");
  if (channelSeparatorIndex >= 0) {
    attribute = trimmed.slice(0, channelSeparatorIndex).trim();
    const channelAndResolution = trimmed.slice(channelSeparatorIndex + 1).trim();
    const resolutionSeparatorIndex = channelAndResolution.indexOf(":");
    if (resolutionSeparatorIndex >= 0) {
      const parsedStart = parseCustomAttributeStartOffset(
        attribute,
        channelAndResolution.slice(0, resolutionSeparatorIndex).trim(),
      );
      startOffset = parsedStart.startOffset;
      startError = parsedStart.error;
      rawResolution = channelAndResolution.slice(resolutionSeparatorIndex + 1).trim();
    } else if (channelAndResolution.length > 0 && /^\d+$/.test(channelAndResolution)) {
      const parsedStart = parseCustomAttributeStartOffset(attribute, channelAndResolution);
      startOffset = parsedStart.startOffset;
      startError = parsedStart.error;
    } else {
      rawResolution = channelAndResolution;
    }
  } else {
    const resolutionSeparatorIndex = trimmed.lastIndexOf(":");
    if (resolutionSeparatorIndex >= 0) {
      attribute = trimmed.slice(0, resolutionSeparatorIndex).trim();
      rawResolution = trimmed.slice(resolutionSeparatorIndex + 1).trim();
    }
  }

  const normalizedResolution = normalizeCustomResolutionText(rawResolution);
  const resolution: AttributeResolution =
    normalizedResolution === "" || normalizedResolution === "8" || normalizedResolution === "8bit"
      ? "EightBit"
      : normalizedResolution === "16" || normalizedResolution === "16bit"
        ? "SixteenBit"
        : "EightBit";
  const error =
    attribute.length === 0
      ? "Attribute name is required"
      : startError
        ? startError
        : normalizedResolution !== "" &&
            normalizedResolution !== "8" &&
            normalizedResolution !== "8bit" &&
            normalizedResolution !== "16" &&
            normalizedResolution !== "16bit"
          ? `Invalid resolution '${rawResolution}' for ${attribute}`
          : null;
  return { attribute, resolution, startOffset, error };
};

export const customProfilePreviewFromText = (value: string): CustomProfilePreview => {
  const controls: CustomProfileAttributePreview[] = [];
  const errors: string[] = [];
  const seen = new Set<string>();
  const occupied = new Set<number>();
  let nextOffset = 1;
  let footprint = 0;

  for (const rawAttribute of value.split(",")) {
    const trimmed = rawAttribute.trim();
    if (!trimmed) {
      continue;
    }
    const spec = parseCustomAttributePreviewSpec(trimmed);
    if (spec.error) {
      errors.push(spec.error);
      continue;
    }
    const key = spec.attribute.toLowerCase();
    if (seen.has(key)) {
      errors.push(`Duplicate attribute '${spec.attribute}'`);
      continue;
    }
    seen.add(key);
    const width = spec.resolution === "SixteenBit" ? 2 : 1;
    const startOffset = spec.startOffset ?? nextOffset;
    const offsets = Array.from({ length: width }, (_, index) => startOffset + index);
    const outOfRange = offsets.find((offset) => offset < 1 || offset > 512);
    if (outOfRange !== undefined) {
      errors.push(`Attribute '${spec.attribute}' exceeds 512 DMX channels`);
      continue;
    }
    const overlap = offsets.find((offset) => occupied.has(offset));
    if (overlap !== undefined) {
      errors.push(`Attribute '${spec.attribute}' overlaps DMX channel ${overlap}`);
      continue;
    }
    offsets.forEach((offset) => occupied.add(offset));
    footprint = Math.max(footprint, ...offsets);
    controls.push({
      attribute: spec.attribute,
      resolution: spec.resolution,
      startOffset: spec.startOffset,
      offsets,
    });
    nextOffset = Math.max(nextOffset, startOffset + width);
  }

  if (footprint > 512) {
    errors.push(`Footprint ${footprint} exceeds 512 DMX channels`);
  }

  return { controls, footprint, errors };
};

export const customProfileAttributeDraftsFromText = (value: string): CustomProfileAttributeDraft[] =>
  value
    .split(",")
    .map((rawAttribute) => rawAttribute.trim())
    .filter(Boolean)
    .map((rawAttribute) => {
      const spec = parseCustomAttributePreviewSpec(rawAttribute);
      return {
        attribute: spec.attribute,
        resolution: spec.resolution,
        startOffset: spec.startOffset === null ? "" : String(spec.startOffset),
      };
    });

export const formatCustomProfileAttributeDraft = (draft: CustomProfileAttributeDraft) => {
  const attribute = draft.attribute.trim();
  const startOffset = draft.startOffset.trim();
  const resolution = draft.resolution === "SixteenBit" ? "16" : "8";
  return `${attribute}${startOffset ? `@${startOffset}` : ""}:${resolution}`;
};

export const customProfileAttributeTextFromDrafts = (drafts: CustomProfileAttributeDraft[]) =>
  drafts.map(formatCustomProfileAttributeDraft).join(", ");

export const customProfileAttributeDraftChannelLabel = (
  drafts: CustomProfileAttributeDraft[],
  index: number,
) => {
  const preview = customProfilePreviewFromText(customProfileAttributeTextFromDrafts(drafts.slice(0, index + 1)));
  const control = preview.controls[preview.controls.length - 1];
  return control && preview.errors.length === 0 ? `CH ${control.offsets.join("/")}` : "Pending";
};

export const formatCompactNumber = (value: number) => {
  if (!Number.isFinite(value)) {
    return "0";
  }
  const fixed = Math.abs(value) >= 10 ? value.toFixed(1) : value.toFixed(2);
  return fixed.replace(/\.?0+$/, "");
};

const geometryPositionLabel = (matrix: number[]) => {
  const x = matrix[3] ?? 0;
  const y = matrix[7] ?? 0;
  const z = matrix[11] ?? 0;
  return `x ${formatCompactNumber(x)} / y ${formatCompactNumber(y)} / z ${formatCompactNumber(z)}`;
};

const geometryOpticsLabel = (geometry: GeometrySummary) => {
  const parts = [
    geometry.beam_type,
    geometry.beam_angle_deg !== undefined && geometry.beam_angle_deg !== null
      ? `beam ${formatCompactNumber(geometry.beam_angle_deg)}deg`
      : null,
    geometry.field_angle_deg !== undefined && geometry.field_angle_deg !== null
      ? `field ${formatCompactNumber(geometry.field_angle_deg)}deg`
      : null,
    geometry.beam_radius !== undefined && geometry.beam_radius !== null
      ? `r ${formatCompactNumber(geometry.beam_radius)}`
      : null,
  ].filter((part): part is string => Boolean(part));
  return parts.join(" / ");
};

const geometryModelLabel = (geometry: GeometrySummary) => {
  const dimensions = geometry.model_dimensions
    ? `${formatCompactNumber(geometry.model_dimensions.x)}x${formatCompactNumber(geometry.model_dimensions.y)}x${formatCompactNumber(geometry.model_dimensions.z)}`
    : null;
  const parts = [
    geometry.model_name,
    geometry.model_primitive,
    dimensions,
    geometry.model_file,
  ].filter((part): part is string => Boolean(part));
  return parts.length > 0 ? `model ${parts.join(" / ")}` : "";
};

const geometryDepth = (geometry: GeometrySummary, geometryByName: Map<string, GeometrySummary>) => {
  let depth = 0;
  let parent = geometry.parent ?? null;
  const seen = new Set<string>([geometry.name]);
  while (parent && !seen.has(parent) && depth < 16) {
    seen.add(parent);
    depth += 1;
    parent = geometryByName.get(parent)?.parent ?? null;
  }
  return depth;
};

export const profileGeometryRows = (
  geometries: GeometrySummary[],
  controls: AttributeControl[],
): ProfileGeometryRow[] => {
  const geometryByName = new Map(geometries.map((geometry) => [geometry.name, geometry]));
  const controlCounts = new Map<string, number>();
  for (const control of controls) {
    const geometry = control.geometry?.trim();
    if (geometry) {
      controlCounts.set(geometry, (controlCounts.get(geometry) ?? 0) + 1);
    }
  }
  return geometries.map((geometry) => ({
    geometry,
    depth: geometryDepth(geometry, geometryByName),
    controlCount: controlCounts.get(geometry.name) ?? 0,
    positionLabel: [geometryPositionLabel(geometry.matrix), geometryModelLabel(geometry), geometryOpticsLabel(geometry)]
      .filter(Boolean)
      .join(" / "),
  }));
};

export const unresolvedGeometryReferences = (
  geometries: GeometrySummary[],
  controls: AttributeControl[],
) => {
  const geometryNames = new Set(geometries.map((geometry) => geometry.name));
  const references = new Set<string>();
  for (const control of controls) {
    const geometry = control.geometry?.trim();
    if (geometry && !geometryNames.has(geometry)) {
      references.add(geometry);
    }
  }
  return Array.from(references).sort((first, second) => first.localeCompare(second));
};
