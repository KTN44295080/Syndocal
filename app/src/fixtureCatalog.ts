import type { AttributeResolution, CustomFixtureProfileRequest } from "./types";

export interface GdtfShareModeSummary {
  name: string;
  dmx_footprint: number | null;
}

export interface GdtfShareFixtureSummary {
  rid: number | null;
  uuid: string | null;
  manufacturer: string;
  fixture: string;
  revision: string;
  uploader: string | null;
  rating: string | null;
  version: string | null;
  creator: string | null;
  filesize: number | null;
  release_status: string | null;
  tested_in_visualizer: boolean | null;
  tested_in_real_life: boolean | null;
  modes: GdtfShareModeSummary[];
}

export interface GdtfShareSearchResponse {
  fixtures: GdtfShareFixtureSummary[];
  facets: {
    manufacturers: string[];
    modes: string[];
    versions: string[];
  };
  filter_support: {
    release_status: boolean | null;
    tested_in_visualizer: boolean | null;
    tested_in_real_life: boolean | null;
  };
  total_matches: number;
}

export interface GdtfFixtureCacheEntry {
  key: string;
  rid: number | null;
  uuid: string | null;
  manufacturer: string;
  fixture: string;
  revision: string;
  path: string;
  filesize: number;
  health: "healthy" | "warnings" | "invalid";
  detail: string;
  warnings: string[];
  modes: GdtfShareModeSummary[];
}

export const fixtureCatalogCacheMegabytes = (entries: readonly Pick<GdtfFixtureCacheEntry, "filesize">[]) =>
  (entries.reduce((total, entry) => total + Math.max(0, entry.filesize), 0) / (1024 * 1024)).toFixed(1);

export interface FixtureProfileHealthSummary {
  fixture_id: number;
  label: string;
  manufacturer: string;
  profile_name: string;
  mode_name: string;
  source_path: string;
  status: "healthy" | "warnings" | "embedded" | "fallback" | "missing";
  detail: string;
  repairable: boolean;
}

export interface VerifiedFixtureProfileSummary {
  id: string;
  manufacturer: string;
  category: VerifiedFixtureCategory;
  category_name: string;
  category_description: string;
  fixture_family: string;
  name: string;
  mode_name: string;
  footprint: number;
  description: string;
  attributes: readonly VerifiedFixtureAttributeDefinition[];
}

export type VerifiedFixtureCategory = "par-wash" | "moving-heads" | "led-bars-pixel" | "dimmers" | "strobe" | "effects";

export interface VerifiedFixtureAttributeDefinition {
  attribute: string;
  resolution: AttributeResolution;
}

interface VerifiedFixtureCategoryDefinition {
  id: VerifiedFixtureCategory;
  name: string;
  description: string;
}

const verifiedFixtureCategories: readonly VerifiedFixtureCategoryDefinition[] = [
  { id: "par-wash", name: "PAR / Wash", description: "Additive color layouts for budget PAR cans and wash lights." },
  { id: "moving-heads", name: "Moving heads", description: "Common RGBW wash and spot channel layouts with position and optics controls." },
  { id: "led-bars-pixel", name: "LED bars / Pixel", description: "Segmented RGB and RGBW bars, including master-dimmer pixel families." },
  { id: "dimmers", name: "Dimmers", description: "Straight-through dimmer and relay-pack channel counts." },
  { id: "strobe", name: "Strobe", description: "Compact shutter, rate, and intensity layouts for budget strobes." },
  { id: "effects", name: "Effects", description: "Safe generic controls for fog, UV, pinspot, bubble, and placeholder laser fixtures." },
];

const attribute = (
  name: string,
  resolution: AttributeResolution = "EightBit",
): VerifiedFixtureAttributeDefinition => ({ attribute: name, resolution });

const rgbAttributes = (segments: number, order: readonly ("R" | "G" | "B")[] = ["R", "G", "B"]) =>
  Array.from({ length: segments }, (_, segmentIndex) => order.map((component) =>
    attribute(`ColorAdd_${component}${segmentIndex === 0 ? "" : segmentIndex + 1}`)))
    .flat();

const rgbwAttributes = (segments: number) =>
  Array.from({ length: segments }, (_, segmentIndex) => ["R", "G", "B", "W"].map((component) =>
    attribute(`ColorAdd_${component}${segmentIndex === 0 ? "" : segmentIndex + 1}`)))
    .flat();

const dimmerAttributes = (channels: number) =>
  Array.from({ length: channels }, (_, index) => attribute(`Dimmer${index === 0 ? "" : index + 1}`));

const categoryDefinition = (category: VerifiedFixtureCategory) => {
  const definition = verifiedFixtureCategories.find((candidate) => candidate.id === category);
  if (!definition) throw new Error(`Unknown verified fixture category: ${category}`);
  return definition;
};

const verifiedProfile = (
  category: VerifiedFixtureCategory,
  id: string,
  fixtureFamily: string,
  name: string,
  modeName: string,
  expectedFootprint: number,
  channelOrder: string,
  attributes: readonly VerifiedFixtureAttributeDefinition[],
): VerifiedFixtureProfileSummary => {
  const definition = categoryDefinition(category);
  const footprint = attributes.reduce(
    (total, candidate) => total + (candidate.resolution === "SixteenBit" ? 2 : 1),
    0,
  );
  if (footprint !== expectedFootprint) {
    throw new Error(`Verified fixture profile ${id} expected ${expectedFootprint}ch but defines ${footprint}ch`);
  }
  return {
    id,
    manufacturer: "Syndocal Verified",
    category,
    category_name: definition.name,
    category_description: definition.description,
    fixture_family: fixtureFamily,
    name,
    mode_name: modeName,
    footprint,
    description: `Common budget-fixture layout. Channel order: ${channelOrder}.`,
    attributes,
  };
};

const e = (name: string) => attribute(name);
const s = (name: string) => attribute(name, "SixteenBit");
const color = {
  r: e("ColorAdd_R"),
  g: e("ColorAdd_G"),
  b: e("ColorAdd_B"),
  w: e("ColorAdd_W"),
  a: e("ColorAdd_A"),
  uv: e("ColorAdd_UV"),
};
const d = e("Dimmer");
const shutter = e("Shutter1");

export const previewVerifiedProfiles: VerifiedFixtureProfileSummary[] = [
  // PAR / Wash — direct color, dimmer-first, dimmer-last, and strobe-extended conventions.
  verifiedProfile("par-wash", "par-direct-rgb-3ch", "Direct Color PAR / Wash", "Generic Direct RGB PAR 3ch", "RGB · 3ch", 3, "R → G → B", [color.r, color.g, color.b]),
  verifiedProfile("par-wash", "par-direct-rgbw-4ch", "Direct Color PAR / Wash", "Generic Direct RGBW PAR 4ch", "RGBW · 4ch", 4, "R → G → B → W", [color.r, color.g, color.b, color.w]),
  verifiedProfile("par-wash", "par-direct-rgba-4ch", "Direct Color PAR / Wash", "Generic Direct RGBA PAR 4ch", "RGBA · 4ch", 4, "R → G → B → A", [color.r, color.g, color.b, color.a]),
  verifiedProfile("par-wash", "par-direct-rgbwa-5ch", "Direct Color PAR / Wash", "Generic Direct RGBWA PAR 5ch", "RGBWA · 5ch", 5, "R → G → B → W → A", [color.r, color.g, color.b, color.w, color.a]),
  verifiedProfile("par-wash", "par-direct-rgbwauv-6ch", "Direct Color PAR / Wash", "Generic Direct RGBWA+UV PAR 6ch", "RGBWA+UV · 6ch", 6, "R → G → B → W → A → UV", [color.r, color.g, color.b, color.w, color.a, color.uv]),
  verifiedProfile("par-wash", "rgb-par-4ch", "Dimmer First PAR / Wash", "Generic D+RGB PAR 4ch", "D+RGB · 4ch", 4, "Dimmer → R → G → B", [d, color.r, color.g, color.b]),
  verifiedProfile("par-wash", "rgbw-par-5ch", "Dimmer First PAR / Wash", "Generic D+RGBW PAR 5ch", "D+RGBW · 5ch", 5, "Dimmer → R → G → B → W", [d, color.r, color.g, color.b, color.w]),
  verifiedProfile("par-wash", "par-dimmer-first-rgba-5ch", "Dimmer First PAR / Wash", "Generic D+RGBA PAR 5ch", "D+RGBA · 5ch", 5, "Dimmer → R → G → B → A", [d, color.r, color.g, color.b, color.a]),
  verifiedProfile("par-wash", "par-dimmer-first-rgbwa-6ch", "Dimmer First PAR / Wash", "Generic D+RGBWA PAR 6ch", "D+RGBWA · 6ch", 6, "Dimmer → R → G → B → W → A", [d, color.r, color.g, color.b, color.w, color.a]),
  verifiedProfile("par-wash", "par-dimmer-first-rgbwauv-7ch", "Dimmer First PAR / Wash", "Generic D+RGBWA+UV PAR 7ch", "D+RGBWA+UV · 7ch", 7, "Dimmer → R → G → B → W → A → UV", [d, color.r, color.g, color.b, color.w, color.a, color.uv]),
  verifiedProfile("par-wash", "par-dimmer-last-rgb-4ch", "Dimmer Last PAR / Wash", "Generic RGB+D PAR 4ch", "RGB+D · 4ch", 4, "R → G → B → Dimmer", [color.r, color.g, color.b, d]),
  verifiedProfile("par-wash", "par-dimmer-last-rgbw-5ch", "Dimmer Last PAR / Wash", "Generic RGBW+D PAR 5ch", "RGBW+D · 5ch", 5, "R → G → B → W → Dimmer", [color.r, color.g, color.b, color.w, d]),
  verifiedProfile("par-wash", "par-dimmer-last-rgbwauv-7ch", "Dimmer Last PAR / Wash", "Generic RGBWA+UV+D PAR 7ch", "RGBWA+UV+D · 7ch", 7, "R → G → B → W → A → UV → Dimmer", [color.r, color.g, color.b, color.w, color.a, color.uv, d]),
  verifiedProfile("par-wash", "par-extended-rgb-d-s-5ch", "Dimmer + Strobe PAR / Wash", "Generic RGB+D+Strobe PAR 5ch", "RGB+D+S · 5ch", 5, "R → G → B → Dimmer → Strobe", [color.r, color.g, color.b, d, shutter]),
  verifiedProfile("par-wash", "par-extended-rgbw-d-s-6ch", "Dimmer + Strobe PAR / Wash", "Generic RGBW+D+Strobe PAR 6ch", "RGBW+D+S · 6ch", 6, "R → G → B → W → Dimmer → Strobe", [color.r, color.g, color.b, color.w, d, shutter]),
  verifiedProfile("par-wash", "par-extended-rgba-d-s-6ch", "Dimmer + Strobe PAR / Wash", "Generic RGBA+D+Strobe PAR 6ch", "RGBA+D+S · 6ch", 6, "R → G → B → A → Dimmer → Strobe", [color.r, color.g, color.b, color.a, d, shutter]),
  verifiedProfile("par-wash", "par-extended-rgbwa-d-s-7ch", "Dimmer + Strobe PAR / Wash", "Generic RGBWA+D+Strobe PAR 7ch", "RGBWA+D+S · 7ch", 7, "R → G → B → W → A → Dimmer → Strobe", [color.r, color.g, color.b, color.w, color.a, d, shutter]),
  verifiedProfile("par-wash", "par-extended-rgbwauv-d-s-8ch", "Dimmer + Strobe PAR / Wash", "Generic RGBWA+UV+D+Strobe PAR 8ch", "RGBWA+UV+D+S · 8ch", 8, "R → G → B → W → A → UV → Dimmer → Strobe", [color.r, color.g, color.b, color.w, color.a, color.uv, d, shutter]),
  verifiedProfile("par-wash", "par-extended-d-s-rgbw-6ch", "Dimmer + Strobe PAR / Wash", "Generic D+Strobe+RGBW PAR 6ch", "D+S+RGBW · 6ch", 6, "Dimmer → Strobe → R → G → B → W", [d, shutter, color.r, color.g, color.b, color.w]),
  verifiedProfile("par-wash", "par-extended-d-s-rgbwauv-8ch", "Dimmer + Strobe PAR / Wash", "Generic D+Strobe+RGBWA+UV PAR 8ch", "D+S+RGBWA+UV · 8ch", 8, "Dimmer → Strobe → R → G → B → W → A → UV", [d, shutter, color.r, color.g, color.b, color.w, color.a, color.uv]),

  // Moving heads — the six common footprints in both wash and spot conventions.
  verifiedProfile("moving-heads", "moving-wash-rgbw-9ch", "Generic RGBW Moving Wash", "Generic RGBW Moving Wash 9ch", "9ch · 8-bit P/T", 9, "Pan → Tilt → Speed → Dimmer → Strobe → R → G → B → W", [e("Pan"), e("Tilt"), e("Generic: PanTiltSpeed"), d, shutter, color.r, color.g, color.b, color.w]),
  verifiedProfile("moving-heads", "moving-head-rgbw-10ch", "Generic RGBW Moving Wash", "Generic RGBW Moving Wash 10ch", "10ch · Macro", 10, "Pan → Tilt → Speed → Dimmer → Strobe → R → G → B → W → Macro", [e("Pan"), e("Tilt"), e("Generic: PanTiltSpeed"), d, shutter, color.r, color.g, color.b, color.w, e("Generic: ColorMacro")]),
  verifiedProfile("moving-heads", "moving-wash-rgbw-11ch", "Generic RGBW Moving Wash", "Generic RGBW Moving Wash 11ch", "11ch · 16-bit P/T", 11, "Pan coarse/fine → Tilt coarse/fine → Speed → Dimmer → Strobe → R → G → B → W", [s("Pan"), s("Tilt"), e("Generic: PanTiltSpeed"), d, shutter, color.r, color.g, color.b, color.w]),
  verifiedProfile("moving-heads", "moving-wash-rgbw-12ch", "Generic RGBW Moving Wash", "Generic RGBW Moving Wash 12ch", "12ch · 16-bit + Macro", 12, "Pan coarse/fine → Tilt coarse/fine → Speed → Dimmer → Strobe → R → G → B → W → Macro", [s("Pan"), s("Tilt"), e("Generic: PanTiltSpeed"), d, shutter, color.r, color.g, color.b, color.w, e("Generic: ColorMacro")]),
  verifiedProfile("moving-heads", "moving-wash-rgbw-14ch", "Generic RGBW Moving Wash", "Generic RGBW Moving Wash 14ch", "14ch · Color + Zoom", 14, "Pan coarse/fine → Tilt coarse/fine → Speed → Dimmer → Strobe → R → G → B → W → Color wheel → Zoom → Macro", [s("Pan"), s("Tilt"), e("Generic: PanTiltSpeed"), d, shutter, color.r, color.g, color.b, color.w, e("Color1"), e("Zoom1"), e("Generic: ColorMacro")]),
  verifiedProfile("moving-heads", "moving-wash-rgbw-16ch", "Generic RGBW Moving Wash", "Generic RGBW Moving Wash 16ch", "16ch · Focus + Prism", 16, "Pan coarse/fine → Tilt coarse/fine → Speed → Dimmer → Strobe → R → G → B → W → Color wheel → Zoom → Macro → Focus → Prism", [s("Pan"), s("Tilt"), e("Generic: PanTiltSpeed"), d, shutter, color.r, color.g, color.b, color.w, e("Color1"), e("Zoom1"), e("Generic: ColorMacro"), e("Focus1"), e("Prism1")]),
  verifiedProfile("moving-heads", "moving-spot-rgbw-9ch", "Generic RGBW Moving Spot", "Generic RGBW Moving Spot 9ch", "9ch · 8-bit P/T", 9, "Pan → Tilt → Speed → Dimmer → Strobe → R → G → B → W", [e("Pan"), e("Tilt"), e("Generic: PanTiltSpeed"), d, shutter, color.r, color.g, color.b, color.w]),
  verifiedProfile("moving-heads", "moving-spot-rgbw-10ch", "Generic RGBW Moving Spot", "Generic RGBW Moving Spot 10ch", "10ch · Gobo", 10, "Pan → Tilt → Speed → Dimmer → Strobe → R → G → B → W → Gobo", [e("Pan"), e("Tilt"), e("Generic: PanTiltSpeed"), d, shutter, color.r, color.g, color.b, color.w, e("Gobo1")]),
  verifiedProfile("moving-heads", "moving-spot-rgbw-11ch", "Generic RGBW Moving Spot", "Generic RGBW Moving Spot 11ch", "11ch · Color + Gobo", 11, "Pan → Tilt → Speed → Dimmer → Strobe → R → G → B → W → Color wheel → Gobo", [e("Pan"), e("Tilt"), e("Generic: PanTiltSpeed"), d, shutter, color.r, color.g, color.b, color.w, e("Color1"), e("Gobo1")]),
  verifiedProfile("moving-heads", "moving-spot-rgbw-12ch", "Generic RGBW Moving Spot", "Generic RGBW Moving Spot 12ch", "12ch · 16-bit + Gobo", 12, "Pan coarse/fine → Tilt coarse/fine → Speed → Dimmer → Strobe → R → G → B → W → Gobo", [s("Pan"), s("Tilt"), e("Generic: PanTiltSpeed"), d, shutter, color.r, color.g, color.b, color.w, e("Gobo1")]),
  verifiedProfile("moving-heads", "moving-spot-rgbw-14ch", "Generic RGBW Moving Spot", "Generic RGBW Moving Spot 14ch", "14ch · Color + Gobo + Focus", 14, "Pan coarse/fine → Tilt coarse/fine → Speed → Dimmer → Strobe → R → G → B → W → Color wheel → Gobo → Focus", [s("Pan"), s("Tilt"), e("Generic: PanTiltSpeed"), d, shutter, color.r, color.g, color.b, color.w, e("Color1"), e("Gobo1"), e("Focus1")]),
  verifiedProfile("moving-heads", "moving-spot-rgbw-16ch", "Generic RGBW Moving Spot", "Generic RGBW Moving Spot 16ch", "16ch · Prism + Zoom", 16, "Pan coarse/fine → Tilt coarse/fine → Speed → Dimmer → Strobe → R → G → B → W → Color wheel → Gobo → Focus → Prism → Zoom", [s("Pan"), s("Tilt"), e("Generic: PanTiltSpeed"), d, shutter, color.r, color.g, color.b, color.w, e("Color1"), e("Gobo1"), e("Focus1"), e("Prism1"), e("Zoom1")]),

  // LED bars / pixel — numbered attributes preserve independent segment visualization.
  verifiedProfile("led-bars-pixel", "pixel-rgb-4-segment-12ch", "RGB Pixel Bar", "Generic RGB Pixel Bar 4", "4 segments · 12ch", 12, "[R → G → B] × 4", rgbAttributes(4)),
  verifiedProfile("led-bars-pixel", "pixel-rgb-8-segment-24ch", "RGB Pixel Bar", "Generic RGB Pixel Bar 8", "8 segments · 24ch", 24, "[R → G → B] × 8", rgbAttributes(8)),
  verifiedProfile("led-bars-pixel", "pixel-rgb-16-segment-48ch", "RGB Pixel Bar", "Generic RGB Pixel Bar 16", "16 segments · 48ch", 48, "[R → G → B] × 16", rgbAttributes(16)),
  verifiedProfile("led-bars-pixel", "pixel-master-rgb-4-segment-13ch", "Master + RGB Pixel Bar", "Generic Master + 4x RGB Pixel Bar", "Master + 4 · 13ch", 13, "Master → [R → G → B] × 4", [d, ...rgbAttributes(4)]),
  verifiedProfile("led-bars-pixel", "pixel-master-rgb-8-segment-25ch", "Master + RGB Pixel Bar", "Generic Master + 8x RGB Pixel Bar", "Master + 8 · 25ch", 25, "Master → [R → G → B] × 8", [d, ...rgbAttributes(8)]),
  verifiedProfile("led-bars-pixel", "pixel-master-rgb-16-segment-49ch", "Master + RGB Pixel Bar", "Generic Master + 16x RGB Pixel Bar", "Master + 16 · 49ch", 49, "Master → [R → G → B] × 16", [d, ...rgbAttributes(16)]),
  verifiedProfile("led-bars-pixel", "pixel-master-rgb-40-segment-121ch", "Master + RGB Pixel Bar", "Generic Master + 40x RGB Pixel Bar", "Master + 40 · 121ch", 121, "Master → [R → G → B] × 40", [d, ...rgbAttributes(40)]),
  verifiedProfile("led-bars-pixel", "pixel-rgbw-4-segment-16ch", "RGBW Pixel Bar", "Generic RGBW Pixel Bar 4", "4 segments · 16ch", 16, "[R → G → B → W] × 4", rgbwAttributes(4)),
  verifiedProfile("led-bars-pixel", "pixel-rgbw-8-segment-32ch", "RGBW Pixel Bar", "Generic RGBW Pixel Bar 8", "8 segments · 32ch", 32, "[R → G → B → W] × 8", rgbwAttributes(8)),
  verifiedProfile("led-bars-pixel", "pixel-master-rgbw-4-segment-17ch", "Master + RGBW Pixel Bar", "Generic Master + 4x RGBW Pixel Bar", "Master + 4 · 17ch", 17, "Master → [R → G → B → W] × 4", [d, ...rgbwAttributes(4)]),
  verifiedProfile("led-bars-pixel", "pixel-master-rgbw-8-segment-33ch", "Master + RGBW Pixel Bar", "Generic Master + 8x RGBW Pixel Bar", "Master + 8 · 33ch", 33, "Master → [R → G → B → W] × 8", [d, ...rgbwAttributes(8)]),
  verifiedProfile("led-bars-pixel", "pixel-bgr-8-segment-24ch", "BGR Pixel Bar", "Generic BGR Pixel Bar 8", "8 segments · 24ch", 24, "[B → G → R] × 8", rgbAttributes(8, ["B", "G", "R"])),
  verifiedProfile("led-bars-pixel", "pixel-master-bgr-8-segment-25ch", "Master + BGR Pixel Bar", "Generic Master + 8x BGR Pixel Bar", "Master + 8 · 25ch", 25, "Master → [B → G → R] × 8", [d, ...rgbAttributes(8, ["B", "G", "R"])]),

  // Dimmers.
  verifiedProfile("dimmers", "dimmer-1ch", "Generic Dimmer Pack", "Generic Dimmer Pack 1ch", "1 channel", 1, "Dimmer 1", dimmerAttributes(1)),
  verifiedProfile("dimmers", "dimmer-pack-2ch", "Generic Dimmer Pack", "Generic Dimmer Pack 2ch", "2 channels", 2, "Dimmer 1 → Dimmer 2", dimmerAttributes(2)),
  verifiedProfile("dimmers", "dimmer-pack-4ch", "Generic Dimmer Pack", "Generic Dimmer Pack 4ch", "4 channels", 4, "Dimmer 1 → Dimmer 2 → Dimmer 3 → Dimmer 4", dimmerAttributes(4)),
  verifiedProfile("dimmers", "dimmer-pack-6ch", "Generic Dimmer Pack", "Generic Dimmer Pack 6ch", "6 channels", 6, "Dimmer 1 → Dimmer 2 → Dimmer 3 → Dimmer 4 → Dimmer 5 → Dimmer 6", dimmerAttributes(6)),
  verifiedProfile("dimmers", "dimmer-pack-12ch", "Generic Dimmer Pack", "Generic Dimmer Pack 12ch", "12 channels", 12, "Dimmer 1 → … → Dimmer 12", dimmerAttributes(12)),

  // Strobes.
  verifiedProfile("strobe", "strobe-1ch", "Generic Strobe", "Generic Strobe 1ch", "Rate · 1ch", 1, "Strobe rate", [shutter]),
  verifiedProfile("strobe", "strobe-2ch", "Generic Strobe", "Generic Strobe 2ch", "Dimmer + Rate · 2ch", 2, "Dimmer → Strobe rate", [d, shutter]),
  verifiedProfile("strobe", "strobe-3ch", "Generic Strobe", "Generic Strobe 3ch", "Dimmer + Rate + Mode · 3ch", 3, "Dimmer → Strobe rate → Strobe mode", [d, shutter, e("Generic: Strobe Mode")]),

  // Effects.
  verifiedProfile("effects", "fog-1ch", "Fog / Haze Machine", "Generic Fog Machine 1ch", "Output · 1ch", 1, "Fog output", [e("Fog1")]),
  verifiedProfile("effects", "fog-2ch", "Fog / Haze Machine", "Generic Fog Machine 2ch", "Output + Fan · 2ch", 2, "Fog output → Fan", [e("Fog1"), e("Generic: Fan")]),
  verifiedProfile("effects", "uv-1ch", "UV Light", "Generic UV Light 1ch", "UV · 1ch", 1, "UV", [color.uv]),
  verifiedProfile("effects", "laser-dummy-safe-1ch", "Laser Dummy-Safe", "Generic Laser Dummy-Safe 1ch", "Placeholder · 1ch", 1, "Laser safety placeholder (default 0)", [e("Generic: Laser Safety Dummy")]),
  verifiedProfile("effects", "pinspot-1ch", "Pinspot", "Generic Pinspot 1ch", "Dimmer · 1ch", 1, "Dimmer", [d]),
  verifiedProfile("effects", "pinspot-rgb-3ch", "Pinspot", "Generic RGB Pinspot 3ch", "RGB · 3ch", 3, "R → G → B", [color.r, color.g, color.b]),
  verifiedProfile("effects", "bubble-1ch", "Bubble Machine", "Generic Bubble Machine 1ch", "Output · 1ch", 1, "Bubble output", [e("Generic: Bubble Output")]),
];

export const verifiedFixtureProfileCount = 60;
export const verifiedFixtureCategoryCount = 6;

if (previewVerifiedProfiles.length !== verifiedFixtureProfileCount) {
  throw new Error(`Verified fixture pack must contain ${verifiedFixtureProfileCount} profiles`);
}
if (new Set(previewVerifiedProfiles.map((entry) => entry.category)).size !== verifiedFixtureCategoryCount) {
  throw new Error(`Verified fixture pack must contain ${verifiedFixtureCategoryCount} categories`);
}

export const verifiedFixtureProfileById = (profileId: string) =>
  previewVerifiedProfiles.find((entry) => entry.id === profileId.trim()) ?? null;

export const verifiedFixtureProfileRequest = (profileId: string): CustomFixtureProfileRequest | null => {
  const profile = verifiedFixtureProfileById(profileId);
  if (!profile) return null;
  let offset = 1;
  const attributes = profile.attributes.map((candidate) => {
    const startOffset = offset;
    offset += candidate.resolution === "SixteenBit" ? 2 : 1;
    return `${candidate.attribute}@${startOffset}:${candidate.resolution === "SixteenBit" ? 16 : 8}`;
  });
  return {
    manufacturer: profile.manufacturer,
    name: profile.name,
    mode_name: profile.mode_name,
    attributes,
  };
};

export interface GdtfShareSearchRequest {
  user: string;
  password: string;
  manufacturer: string | null;
  fixture: string | null;
  query: string | null;
  mode: string | null;
  min_footprint: number | null;
  max_footprint: number | null;
  release_only: boolean;
  tested_in_visualizer: boolean;
  tested_in_real_life: boolean;
  limit: number;
}

export interface GdtfShareDownloadRequest {
  user: string;
  password: string;
  rid: number | null;
  uuid: string | null;
  manufacturer: string;
  fixture: string;
  revision: string;
}

export const fixtureCatalogFavoritesStorageKey = "syndocal.fixtureCatalogFavorites.v1";

const normalizedIdentityPart = (value: string) => value.trim().toLocaleLowerCase();

export const fixtureCatalogFavoriteKey = (fixture: {
  rid?: number | null;
  uuid?: string | null;
  manufacturer: string;
  fixture: string;
  revision: string;
}) => fixture.rid != null
  ? `share:rid:${fixture.rid}`
  : fixture.uuid?.trim()
    ? `share:uuid:${normalizedIdentityPart(fixture.uuid)}`
    : `share:name:${normalizedIdentityPart(fixture.manufacturer)}:${normalizedIdentityPart(fixture.fixture)}:${normalizedIdentityPart(fixture.revision)}`;

export const verifiedFixtureFavoriteKey = (profileId: string) => `verified:${normalizedIdentityPart(profileId)}`;

export const fixtureCatalogFavoritesFromUnknown = (value: unknown) => {
  if (!Array.isArray(value)) return [] as string[];
  return [...new Set(value
    .filter((entry): entry is string => typeof entry === "string")
    .map((entry) => entry.trim())
    .filter(Boolean))]
    .slice(0, 256);
};

export const loadFixtureCatalogFavorites = () => {
  try {
    return fixtureCatalogFavoritesFromUnknown(JSON.parse(
      window.localStorage.getItem(fixtureCatalogFavoritesStorageKey) ?? "[]",
    ));
  } catch {
    return [];
  }
};

export const saveFixtureCatalogFavorites = (favorites: string[]) => {
  const normalized = fixtureCatalogFavoritesFromUnknown(favorites);
  try {
    window.localStorage.setItem(fixtureCatalogFavoritesStorageKey, JSON.stringify(normalized));
  } catch {
    // Hardened WebViews can block localStorage; session state still works.
  }
  return normalized;
};

export const toggledFixtureCatalogFavorites = (favorites: string[], key: string) => {
  const normalized = fixtureCatalogFavoritesFromUnknown(favorites);
  return normalized.includes(key)
    ? normalized.filter((candidate) => candidate !== key)
    : fixtureCatalogFavoritesFromUnknown([...normalized, key]);
};

export type FixtureFootprintBand = "any" | "1-4" | "5-16" | "17-32" | "33-512";

export const fixtureFootprintBandBounds = (band: FixtureFootprintBand) => {
  switch (band) {
    case "1-4": return { min: 1, max: 4 };
    case "5-16": return { min: 5, max: 16 };
    case "17-32": return { min: 17, max: 32 };
    case "33-512": return { min: 33, max: 512 };
    default: return { min: null, max: null };
  }
};

export const fixtureCatalogIdentityMatches = (
  left: Pick<GdtfShareFixtureSummary, "rid" | "uuid" | "manufacturer" | "fixture" | "revision">,
  right: Pick<GdtfFixtureCacheEntry, "rid" | "uuid" | "manufacturer" | "fixture" | "revision">,
) => fixtureCatalogFavoriteKey(left) === fixtureCatalogFavoriteKey(right);

export const fixtureCatalogHealthLabel = (status: string) => {
  switch (status) {
    case "healthy": return "Ready";
    case "warnings": return "Warnings";
    case "embedded": return "Embedded";
    case "fallback": return "Fallback";
    case "missing": return "Missing";
    case "invalid": return "Invalid";
    default: return "Unknown";
  }
};

export const fixtureCatalogSearchTextMatches = (
  candidate: { manufacturer: string; fixture: string; revision: string; modes: GdtfShareModeSummary[] },
  query: string,
) => {
  const needle = query.trim().toLocaleLowerCase();
  if (!needle) return true;
  return [candidate.manufacturer, candidate.fixture, candidate.revision, ...candidate.modes.map((mode) => mode.name)]
    .some((value) => value.toLocaleLowerCase().includes(needle));
};
