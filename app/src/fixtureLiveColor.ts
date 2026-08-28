import { channelFunctionWheelColor } from "./channelFunctionHelpers";
import { colorCandidates } from "./fixtureControlRuntime";
import type {
  AttributeControl,
  AttributeValueSummary,
  DmxUniversePreview,
  PatchedFixtureSummary,
} from "./types";

export const liveDmxPollHz = 30;
export const liveDmxPollIntervalMs = 1_000 / liveDmxPollHz;
export const liveFixtureDarkColor = "rgb(31, 38, 46)";

export interface FixtureLiveColorSegment {
  key: string;
  color: string;
  intensity: number;
}

export interface FixtureLiveColor {
  color: string;
  intensity: number;
  segmentCount: number;
  segments: FixtureLiveColorSegment[];
  source: "rgb" | "wheel" | "dimmer";
  valueSource: "preview" | "attribute";
}

export interface StageLiveColorViewport {
  x: number;
  z: number;
  width: number;
  height: number;
}

type ColorRole = "red" | "green" | "blue" | "white" | "amber" | "uv";

const exactStageColorRole: Record<NonNullable<PatchedFixtureSummary["stage_layout"]>["logical_segments"][number]["color_controls"][number]["role"], ColorRole> = {
  Red: "red",
  Green: "green",
  Blue: "blue",
  White: "white",
  Amber: "amber",
  Uv: "uv",
};

interface ControlDescriptor {
  control: AttributeControl;
  colorRole: ColorRole | null;
  dimmer: boolean;
  segmentIndex: number | null;
  geometry: string | null;
}

interface SegmentControlGroup {
  key: string;
  controls: ControlDescriptor[];
}

type ControlValueReader = (control: AttributeControl) => number;

interface FixtureBrightness {
  hasDimmer: boolean;
  dimmerlessProxy: number;
}

const clampByte = (value: number) =>
  Math.max(0, Math.min(255, Number.isFinite(value) ? Math.round(value) : 0));

const clampControlValue = (value: number) =>
  Math.max(0, Math.min(65_535, Number.isFinite(value) ? value : 0));

const normalizeControlName = (value: string) => value.toLowerCase().replace(/[^a-z0-9]+/g, "");

const exactColorRoleByName = new Map<string, ColorRole>(
  (Object.entries(colorCandidates) as [ColorRole, readonly string[]][]).flatMap(([role, names]) =>
    names.map((name) => [normalizeControlName(name), role] as const),
  ),
);

const trailingSegmentIndex = (value: string) => {
  const match = normalizeControlName(value).match(/(\d+)$/);
  return match ? Number(match[1]) : null;
};

const controlColorRole = (control: AttributeControl): ColorRole | null => {
  const candidates = [control.attribute, control.channel_name];
  for (const candidate of candidates) {
    const normalized = normalizeControlName(candidate).replace(/\d+$/, "");
    const exactRole = exactColorRoleByName.get(normalized);
    if (exactRole) return exactRole;
    if (/^(?:coloradd|colorrgb|color)?(?:r|red)$/.test(normalized)) return "red";
    if (/^(?:coloradd|colorrgb|color)?(?:g|green)$/.test(normalized)) return "green";
    if (/^(?:coloradd|colorrgb|color)?(?:b|blue)$/.test(normalized)) return "blue";
    if (/^(?:coloradd|colorrgb|color)?(?:w|white|warmwhite|coldwhite|coolwhite)$/.test(normalized)) {
      return "white";
    }
    if (/^(?:coloradd|colorrgb|color)?(?:a|amber)$/.test(normalized)) return "amber";
    if (/^(?:coloradd|colorrgb|color)?(?:uv|ultraviolet)$/.test(normalized)) return "uv";
  }
  return null;
};

const controlIsDimmer = (control: AttributeControl) => {
  return [control.attribute, control.channel_name].some((candidate) => {
    const normalized = normalizeControlName(candidate).replace(/\d+$/, "");
    return normalized === "dimmer"
      || normalized === "intensity"
      || normalized === "masterdimmer"
      || normalized === "masterintensity";
  });
};

const describeControl = (control: AttributeControl): ControlDescriptor => ({
  control,
  colorRole: controlColorRole(control),
  dimmer: controlIsDimmer(control),
  segmentIndex: trailingSegmentIndex(control.attribute) ?? trailingSegmentIndex(control.channel_name),
  geometry: control.geometry?.trim() || null,
});

const fixtureSegmentControlGroups = (fixture: PatchedFixtureSummary): SegmentControlGroup[] => {
  const descriptors = fixture.controls.map(describeControl);
  const exactLayout = fixture.stage_layout;
  const exactSegments = fixture.stage_layout?.logical_segments;
  if (exactSegments?.length) {
    const globalDimmer = exactLayout?.global_dimmer_control_index;
    return [...exactSegments]
      .sort((left, right) => left.logical_index - right.logical_index)
      .map((segment) => {
        const controls: ControlDescriptor[] = [];
        for (const binding of segment.color_controls) {
          const descriptor = descriptors[binding.control_index];
          if (descriptor) {
            controls.push({
              ...descriptor,
              colorRole: exactStageColorRole[binding.role],
              dimmer: false,
            });
          }
        }
        if (globalDimmer !== null && globalDimmer !== undefined) {
          const descriptor = descriptors[globalDimmer];
          if (descriptor) controls.push({ ...descriptor, colorRole: null, dimmer: true });
        }
        return {
          key: `stage-layout-${segment.logical_index}`,
          controls,
        };
      });
  }
  if (exactLayout) {
    const globalDimmer = exactLayout.global_dimmer_control_index;
    const descriptor = globalDimmer === null ? undefined : descriptors[globalDimmer];
    return [{
      key: "stage-layout-physical-only",
      controls: descriptor ? [{ ...descriptor, colorRole: null, dimmer: true }] : [],
    }];
  }
  const hasUnnumberedColor = descriptors.some((descriptor) =>
    descriptor.colorRole !== null && descriptor.segmentIndex === null);
  const numberedColorIndices = [...new Set(
    descriptors
      .filter((descriptor) => descriptor.colorRole && descriptor.segmentIndex !== null)
      .map((descriptor) => descriptor.segmentIndex!),
  )].sort((left, right) => left - right);
  if (numberedColorIndices.length > 1 || (hasUnnumberedColor && numberedColorIndices.length > 0)) {
    const segmentIndices = [...new Set([
      ...(hasUnnumberedColor ? [1] : []),
      ...numberedColorIndices,
    ])].sort((left, right) => left - right);
    return segmentIndices.map((segmentIndex) => ({
      key: `attribute-${segmentIndex}`,
      controls: descriptors.filter((descriptor) =>
        descriptor.segmentIndex === segmentIndex
        || (descriptor.segmentIndex === null && descriptor.dimmer)
        || (segmentIndex === 1 && descriptor.segmentIndex === null && descriptor.colorRole !== null)),
    }));
  }

  const colorGeometryNames = [...new Set(
    descriptors
      .filter((descriptor) => descriptor.colorRole && descriptor.geometry)
      .map((descriptor) => descriptor.geometry!),
  )];
  if (colorGeometryNames.length > 1) {
    const geometryOrder = new Map(fixture.geometries.map((geometry, index) => [geometry.name, index]));
    colorGeometryNames.sort((left, right) =>
      (geometryOrder.get(left) ?? Number.MAX_SAFE_INTEGER) - (geometryOrder.get(right) ?? Number.MAX_SAFE_INTEGER)
      || left.localeCompare(right));
    return colorGeometryNames.map((geometry) => ({
      key: `geometry-${geometry}`,
      controls: descriptors.filter((descriptor) =>
        descriptor.geometry === geometry
        || (descriptor.geometry === null && (descriptor.dimmer || descriptor.colorRole !== null))),
    }));
  }

  return [{ key: "fixture", controls: descriptors }];
};

export const fixtureLiveSegmentCount = (fixture: PatchedFixtureSummary) =>
  fixtureSegmentControlGroups(fixture).length;

export const fixtureLiveSegmentSkeleton = (
  fixture: PatchedFixtureSummary,
): FixtureLiveColorSegment[] =>
  fixtureSegmentControlGroups(fixture).map((group) => ({
    key: group.key,
    color: liveFixtureDarkColor,
    intensity: 0,
  }));

export const dmxPreviewMap = (previews: DmxUniversePreview[]) =>
  new Map(previews.map((preview) => [preview.universe, preview.values] as const));

const previewControlValueReader = (
  fixture: PatchedFixtureSummary,
  universeValues: number[],
): ControlValueReader => (control) => {
  const byteAtOffset = (offset: number | undefined) => {
    if (!offset) return 0;
    const absoluteIndex = fixture.address + offset - 2;
    return clampByte(universeValues[absoluteIndex] ?? 0);
  };
  const msb = byteAtOffset(control.offsets[0]);
  if (control.resolution === "SixteenBit") {
    return msb * 256 + byteAtOffset(control.offsets[1]);
  }
  return msb * 257;
};

const attributeControlValueReader = (
  attributeValues: AttributeValueSummary[],
): ControlValueReader => {
  const valuesByAttribute = new Map(
    attributeValues.map((entry) => [entry.attribute.toLowerCase(), entry.value]),
  );
  return (control) => valuesByAttribute.get(control.attribute.toLowerCase()) ?? control.default_value;
};

const previewDimmerlessBrightness = (
  fixture: PatchedFixtureSummary,
  universeValues: number[],
) => {
  const footprint = Math.max(
    0,
    ...fixture.controls.flatMap((control) =>
      control.offsets.filter((offset) => Number.isInteger(offset) && offset > 0)),
  );
  let brightestByte = 0;
  for (let offset = 1; offset <= footprint; offset += 1) {
    brightestByte = Math.max(
      brightestByte,
      clampByte(universeValues[fixture.address + offset - 2] ?? 0),
    );
  }
  return brightestByte / 255;
};

const attributeDimmerlessBrightness = (
  attributeValues: AttributeValueSummary[],
) =>
  attributeValues.reduce(
    (brightest, entry) => Math.max(brightest, clampControlValue(entry.value) / 65_535),
    0,
  );

const parseHexColor = (color: string) => {
  const match = color.match(/^#([0-9a-f]{2})([0-9a-f]{2})([0-9a-f]{2})$/i);
  return match
    ? [Number.parseInt(match[1], 16), Number.parseInt(match[2], 16), Number.parseInt(match[3], 16)] as const
    : null;
};

const segmentLiveColor = (
  group: SegmentControlGroup,
  readControlValue: ControlValueReader,
  brightness: FixtureBrightness,
): FixtureLiveColorSegment & Pick<FixtureLiveColor, "source"> => {
  const globalDimmers = group.controls.filter((descriptor) =>
    descriptor.dimmer && descriptor.segmentIndex === null && descriptor.geometry === null);
  const localDimmers = group.controls.filter((descriptor) =>
    descriptor.dimmer && (descriptor.segmentIndex !== null || descriptor.geometry !== null));
  const dimmerLevel = (dimmers: ControlDescriptor[]) =>
    dimmers.length > 0
      ? Math.max(...dimmers.map(({ control }) => readControlValue(control) / 65_535))
      : 1;
  const dimmer = brightness.hasDimmer
    ? dimmerLevel(globalDimmers) * dimmerLevel(localDimmers)
    : brightness.dimmerlessProxy;
  const channelLevel = (role: ColorRole) =>
    Math.min(
      1,
      group.controls
        .filter((descriptor) => descriptor.colorRole === role)
        .reduce(
          (sum, { control }) => sum + readControlValue(control) / 65_535,
          0,
        ),
    );
  const hasAdditiveColor = group.controls.some((descriptor) => descriptor.colorRole !== null);
  let red = 0;
  let green = 0;
  let blue = 0;
  let source: FixtureLiveColor["source"] = "dimmer";

  if (hasAdditiveColor) {
    const white = channelLevel("white");
    const amber = channelLevel("amber");
    const uv = channelLevel("uv");
    red = Math.min(1, channelLevel("red") + white + amber);
    green = Math.min(1, channelLevel("green") + white + amber * 0.66);
    blue = Math.min(1, channelLevel("blue") + white + uv * 0.72);
    source = "rgb";
  } else {
    const wheelColor = group.controls.flatMap(({ control }) => {
      const value = readControlValue(control);
      const fn = (control.functions ?? []).find((candidate) =>
        value >= Math.min(candidate.dmx_from, candidate.dmx_to)
        && value <= Math.max(candidate.dmx_from, candidate.dmx_to)
        && channelFunctionWheelColor(control, candidate));
      const color = fn ? channelFunctionWheelColor(control, fn) : null;
      const parsed = color ? parseHexColor(color) : null;
      return parsed ? [parsed] : [];
    })[0];
    if (wheelColor) {
      [red, green, blue] = wheelColor.map((value) => value / 255) as [number, number, number];
      source = "wheel";
    } else {
      red = 1;
      green = 1;
      blue = 1;
    }
  }

  if (!brightness.hasDimmer) {
    const colorPeak = Math.max(red, green, blue);
    if (colorPeak > 0) {
      red /= colorPeak;
      green /= colorPeak;
      blue /= colorPeak;
    } else if (dimmer > 0) {
      red = 1;
      green = 1;
      blue = 1;
    }
  }

  const output = [
    clampByte(red * dimmer * 255),
    clampByte(green * dimmer * 255),
    clampByte(blue * dimmer * 255),
  ] as const;
  const intensity = Math.max(...output) / 255;
  return {
    key: group.key,
    color: intensity > 0 ? `rgb(${output[0]}, ${output[1]}, ${output[2]})` : liveFixtureDarkColor,
    intensity,
    source,
  };
};

export const fixtureLiveColor = (
  fixture: PatchedFixtureSummary,
  previewsByUniverse: ReadonlyMap<number, number[]>,
  attributeValues: AttributeValueSummary[] = fixture.attribute_values,
): FixtureLiveColor => {
  const hasPreview = previewsByUniverse.has(fixture.universe);
  const universeValues = previewsByUniverse.get(fixture.universe) ?? [];
  const valueSource = hasPreview ? "preview" : "attribute";
  const readControlValue = valueSource === "preview"
    ? previewControlValueReader(fixture, universeValues)
    : attributeControlValueReader(attributeValues);
  const hasDimmer = fixture.stage_layout
    ? fixture.stage_layout.global_dimmer_control_index !== null
    : fixture.controls.some(controlIsDimmer);
  // Daslight parity for dimmerless fixtures: use the normalized maximum current
  // DMX byte in the fixture footprint, or attribute value on fallback, as a
  // surrogate dimmer so an all-zero fixture is unlit instead of assumed full.
  const brightness: FixtureBrightness = {
    hasDimmer,
    dimmerlessProxy: hasDimmer
      ? 1
      : valueSource === "preview"
        ? previewDimmerlessBrightness(fixture, universeValues)
        : attributeDimmerlessBrightness(attributeValues),
  };
  const segments = fixtureSegmentControlGroups(fixture)
    .map((group) => segmentLiveColor(group, readControlValue, brightness));
  const brightest = segments.reduce(
    (current, segment) => segment.intensity > current.intensity ? segment : current,
    segments[0],
  );
  return {
    color: brightest?.color ?? liveFixtureDarkColor,
    intensity: brightest?.intensity ?? 0,
    segmentCount: segments.length,
    segments: segments.map(({ key, color, intensity }) => ({ key, color, intensity })),
    source: brightest?.source ?? "dimmer",
    valueSource,
  };
};

export const fixtureIntersectsLiveColorViewport = (
  fixture: { x: number; z: number; width: number; height: number },
  viewport: StageLiveColorViewport,
) =>
  fixture.x + fixture.width / 2 >= viewport.x
  && fixture.x - fixture.width / 2 <= viewport.x + viewport.width
  && fixture.z + fixture.height / 2 >= viewport.z
  && fixture.z - fixture.height / 2 <= viewport.z + viewport.height;
