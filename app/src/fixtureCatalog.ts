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
  manual_url?: string;
  attributes: readonly VerifiedFixtureAttributeDefinition[];
}

export type VerifiedFixtureCategory = "university-rig" | "personal-rig" | "par-wash" | "moving-heads" | "led-bars-pixel" | "dimmers" | "strobe" | "effects";

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
  {
    id: "university-rig",
    name: "University rig",
    description: "Official-manual channel maps for the university's eight primary fixtures, with every documented DMX mode.",
  },
  {
    id: "personal-rig",
    name: "Personal rig",
    description: "Source-matched channel maps for the four user-owned fixtures supplied via Daslight files and hardware manuals, including every documented or embedded mode.",
  },
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

const rgbaAttributes = (segments: number) =>
  Array.from({ length: segments }, (_, segmentIndex) => ["R", "G", "B", "A"].map((component) =>
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

const universityRigProfile = (
  id: string,
  manufacturer: string,
  fixtureFamily: string,
  name: string,
  modeName: string,
  expectedFootprint: number,
  manualUrl: string,
  attributes: readonly VerifiedFixtureAttributeDefinition[],
): VerifiedFixtureProfileSummary => {
  const definition = categoryDefinition("university-rig");
  const footprint = attributes.reduce(
    (total, candidate) => total + (candidate.resolution === "SixteenBit" ? 2 : 1),
    0,
  );
  if (footprint !== expectedFootprint) {
    throw new Error(`University rig profile ${id} expected ${expectedFootprint}ch but defines ${footprint}ch`);
  }
  return {
    id,
    manufacturer,
    category: "university-rig",
    category_name: definition.name,
    category_description: definition.description,
    fixture_family: fixtureFamily,
    name,
    mode_name: modeName,
    footprint,
    description: `Official-manual channel map · ${footprint}ch. Every documented mode for this fixture is included in the university rig pack.`,
    manual_url: manualUrl,
    attributes,
  };
};

const personalRigProfile = (
  id: string,
  fixtureFamily: string,
  name: string,
  modeName: string,
  expectedFootprint: number,
  sourceDescription: string,
  deployedInDsf2026: boolean,
  attributes: readonly VerifiedFixtureAttributeDefinition[],
): VerifiedFixtureProfileSummary => {
  const definition = categoryDefinition("personal-rig");
  const footprint = attributes.reduce(
    (total, candidate) => total + (candidate.resolution === "SixteenBit" ? 2 : 1),
    0,
  );
  if (footprint !== expectedFootprint) {
    throw new Error(`Personal rig profile ${id} expected ${expectedFootprint}ch but defines ${footprint}ch`);
  }
  return {
    id,
    manufacturer: "User Library",
    category: "personal-rig",
    category_name: definition.name,
    category_description: definition.description,
    fixture_family: fixtureFamily,
    name,
    mode_name: modeName,
    footprint,
    description: `${sourceDescription} · ${footprint}ch.${deployedInDsf2026 ? " This mode is deployed in DSF2026.dvc." : ""}`,
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

const epar64Manual = "https://www.soundhouse.co.jp/download/se/epar64rgba_v1.00.pdf";
const miniSpot30Manual = "https://www.soundhouse.co.jp/download/se/minispot30.pdf";
const pinspotQuadManual = "https://assets.centryngroup.com/dl/files/PINSPOTLEDQUADDMX_USERMANUAL.pdf";
const saberSpotManual = "https://www.adj.com/products/saber-spot-rgbw";
const encoreFr50zManual = "https://assets.centryngroup.com/dl/files/ENC846__DL__008.pdf";
const megaBarRgbaManual = "https://www.adj.com/cdn/shop/files/bb469faba6680c0a58a51eecc0be5e50f824b5cc_ADJ_Mega_Bar_RGBA___User_Manual_2023_04_11.pdf";
const mega64ProfileEpManual = "https://www.adj.com/cdn/shop/files/0e7622ace1d8c0c4098428d602db0e71107cd429_Eliminator_Mega_64_Profile_EP___User_Manual.pdf";
const mega64ProfilePlusManual = "https://assets.centryngroup.com/dl/files/MEG340__DL__001.pdf";

const mega64ProfileModes = (
  idPrefix: string,
  manufacturer: string,
  fixtureFamily: string,
  name: string,
  manualUrl: string,
) => [
  universityRigProfile(`${idPrefix}-4ch`, manufacturer, fixtureFamily, name, "4-channel", 4, manualUrl,
    [color.r, color.g, color.b, color.uv]),
  universityRigProfile(`${idPrefix}-5ch`, manufacturer, fixtureFamily, name, "5-channel", 5, manualUrl,
    [color.r, color.g, color.b, color.uv, d]),
  universityRigProfile(`${idPrefix}-6ch`, manufacturer, fixtureFamily, name, "6-channel", 6, manualUrl,
    [color.r, color.g, color.b, color.uv, shutter, d]),
  universityRigProfile(`${idPrefix}-9ch`, manufacturer, fixtureFamily, name, "9-channel", 9, manualUrl,
    [color.r, color.g, color.b, color.uv, shutter, d, e("Control1"), e("Color1"), e("Control2")]),
  universityRigProfile(`${idPrefix}-10ch`, manufacturer, fixtureFamily, name, "10-channel", 10, manualUrl,
    [color.r, color.g, color.b, color.uv, shutter, d, e("Control1"), e("Color1"), e("Control2"), e("Control3")]),
];

export const universityRigProfiles: VerifiedFixtureProfileSummary[] = [
  universityRigProfile("university-epar64-rgba-4ch", "Stage Evolution", "ePAR64 RGBA", "ePAR64 RGBA", "4-channel", 4, epar64Manual,
    [color.r, color.g, color.b, color.a]),
  universityRigProfile("university-epar64-rgba-8ch", "Stage Evolution", "ePAR64 RGBA", "ePAR64 RGBA", "8-channel", 8, epar64Manual,
    [color.r, color.g, color.b, color.a, e("Color1"), e("Control1"), e("Control2"), d]),

  universityRigProfile("university-mini-spot30-9ch", "Stage Evolution", "STAGE EVOLUTION MINI SPOT30", "MINI SPOT30", "9-channel", 9, miniSpot30Manual,
    [e("Pan"), e("Tilt"), e("Color1"), e("Gobo1"), shutter, d, e("PanTiltSpeed"), e("Control1"), e("Control2")]),
  universityRigProfile("university-mini-spot30-11ch", "Stage Evolution", "STAGE EVOLUTION MINI SPOT30", "MINI SPOT30", "11-channel · 16-bit P/T", 11, miniSpot30Manual,
    [s("Pan"), s("Tilt"), e("Color1"), e("Gobo1"), shutter, d, e("PanTiltSpeed"), e("Control1"), e("Control2")]),

  universityRigProfile("university-pinspot-led-quad-dmx-6ch", "ADJ", "Pinspot LED Quad DMX", "Pinspot LED Quad DMX", "6-channel", 6, pinspotQuadManual,
    [color.r, color.g, color.b, color.w, d, shutter]),

  universityRigProfile("university-saber-spot-rgbw-hsi-3ch", "ADJ", "ADJ SABER SPOT RGBW", "SABER SPOT RGBW", "HSI · 3-channel", 3, saberSpotManual,
    [e("Control1"), e("Control2"), d]),
  universityRigProfile("university-saber-spot-rgbw-rgbw-4ch", "ADJ", "ADJ SABER SPOT RGBW", "SABER SPOT RGBW", "RGBW · 4-channel", 4, saberSpotManual,
    [color.r, color.g, color.b, color.w]),
  universityRigProfile("university-saber-spot-rgbw-hsi-curve-4ch", "ADJ", "ADJ SABER SPOT RGBW", "SABER SPOT RGBW", "HSI + dimmer curve · 4-channel", 4, saberSpotManual,
    [e("Control1"), e("Control2"), d, e("Dimmer2")]),
  universityRigProfile("university-saber-spot-rgbw-rgbwd-5ch", "ADJ", "ADJ SABER SPOT RGBW", "SABER SPOT RGBW", "RGBWD · 5-channel", 5, saberSpotManual,
    [color.r, color.g, color.b, color.w, d]),
  universityRigProfile("university-saber-spot-rgbw-strobe-6ch", "ADJ", "ADJ SABER SPOT RGBW", "SABER SPOT RGBW", "RGBWD + strobe · 6-channel", 6, saberSpotManual,
    [shutter, color.r, color.g, color.b, color.w, d]),
  universityRigProfile("university-saber-spot-rgbw-dimmer-fine-6ch", "ADJ", "ADJ SABER SPOT RGBW", "SABER SPOT RGBW", "RGBWD fine · 6-channel", 6, saberSpotManual,
    [color.r, color.g, color.b, color.w, s("Dimmer")]),
  universityRigProfile("university-saber-spot-rgbw-dimmer-fine-strobe-7ch", "ADJ", "ADJ SABER SPOT RGBW", "SABER SPOT RGBW", "RGBWD fine + strobe · 7-channel", 7, saberSpotManual,
    [color.r, color.g, color.b, color.w, shutter, s("Dimmer")]),
  universityRigProfile("university-saber-spot-rgbw-extended-8ch", "ADJ", "ADJ SABER SPOT RGBW", "SABER SPOT RGBW", "Extended · 8-channel", 8, saberSpotManual,
    [color.r, color.g, color.b, color.w, shutter, s("Dimmer"), e("Dimmer2")]),
  universityRigProfile("university-saber-spot-rgbw-fine-8ch", "ADJ", "ADJ SABER SPOT RGBW", "SABER SPOT RGBW", "RGBW fine · 8-channel", 8, saberSpotManual,
    [s("ColorAdd_R"), s("ColorAdd_G"), s("ColorAdd_B"), s("ColorAdd_W")]),
  universityRigProfile("university-saber-spot-rgbw-hsi-programs-9ch", "ADJ", "ADJ SABER SPOT RGBW", "SABER SPOT RGBW", "HSI programs · 9-channel", 9, saberSpotManual,
    [shutter, e("Control1"), e("Control2"), d, e("Color1"), e("Control3"), e("Control4"), e("Control5"), e("Dimmer2")]),
  universityRigProfile("university-saber-spot-rgbw-programs-11ch", "ADJ", "ADJ SABER SPOT RGBW", "SABER SPOT RGBW", "RGBW programs · 11-channel", 11, saberSpotManual,
    [shutter, color.r, color.g, color.b, color.w, d, e("Color1"), e("Control1"), e("Control2"), e("Control3"), e("Dimmer2")]),
  universityRigProfile("university-saber-spot-rgbw-programs-fine-12ch", "ADJ", "ADJ SABER SPOT RGBW", "SABER SPOT RGBW", "RGBW programs fine · 12-channel", 12, saberSpotManual,
    [color.r, color.g, color.b, color.w, e("Color1"), shutter, s("Dimmer"), e("Control1"), e("Control2"), e("Control3"), e("Dimmer2")]),

  universityRigProfile("university-encore-fr50z-1ch", "ADJ", "ADJ Encore FR50Z", "Encore FR50Z", "1CH", 1, encoreFr50zManual, [d]),
  universityRigProfile("university-encore-fr50z-2ch", "ADJ", "ADJ Encore FR50Z", "Encore FR50Z", "2CH", 2, encoreFr50zManual, [d, shutter]),
  universityRigProfile("university-encore-fr50z-2-2ch", "ADJ", "ADJ Encore FR50Z", "Encore FR50Z", "2-2CH", 2, encoreFr50zManual, [s("Dimmer")]),
  universityRigProfile("university-encore-fr50z-3ch", "ADJ", "ADJ Encore FR50Z", "Encore FR50Z", "3CH", 3, encoreFr50zManual, [d, shutter, e("Control1")]),
  universityRigProfile("university-encore-fr50z-2-3ch", "ADJ", "ADJ Encore FR50Z", "Encore FR50Z", "2-3CH", 3, encoreFr50zManual, [s("Dimmer"), e("Control1")]),
  universityRigProfile("university-encore-fr50z-4ch", "ADJ", "ADJ Encore FR50Z", "Encore FR50Z", "4CH", 4, encoreFr50zManual, [s("Dimmer"), shutter, e("Control1")]),

  universityRigProfile("university-mega-bar-rgba-4ch", "ADJ", "ADJ MEGA BAR RGBA", "MEGA BAR RGBA", "4-channel · RGBA", 4, megaBarRgbaManual,
    rgbaAttributes(1)),
  universityRigProfile("university-mega-bar-rgba-6ch", "ADJ", "ADJ MEGA BAR RGBA", "MEGA BAR RGBA", "6-channel · RGBA + strobe/dimmer", 6, megaBarRgbaManual,
    [...rgbaAttributes(1), shutter, d]),
  universityRigProfile("university-mega-bar-rgba-7ch", "ADJ", "ADJ MEGA BAR RGBA", "MEGA BAR RGBA", "7-channel · color macro", 7, megaBarRgbaManual,
    [...rgbaAttributes(1), e("Color1"), shutter, d]),
  universityRigProfile("university-mega-bar-rgba-9ch", "ADJ", "ADJ MEGA BAR RGBA", "MEGA BAR RGBA", "9-channel · programs", 9, megaBarRgbaManual,
    [...rgbaAttributes(1), e("Color1"), e("Control1"), e("Control2"), shutter, d]),
  universityRigProfile("university-mega-bar-rgba-10ch", "ADJ", "ADJ MEGA BAR RGBA", "MEGA BAR RGBA", "10-channel · halves", 10, megaBarRgbaManual,
    [...rgbaAttributes(2), shutter, d]),
  universityRigProfile("university-mega-bar-rgba-18ch", "ADJ", "ADJ MEGA BAR RGBA", "MEGA BAR RGBA", "18-channel · fourths", 18, megaBarRgbaManual,
    [...rgbaAttributes(4), shutter, d]),
  universityRigProfile("university-mega-bar-rgba-34ch", "ADJ", "ADJ MEGA BAR RGBA", "MEGA BAR RGBA", "34-channel · eighths", 34, megaBarRgbaManual,
    [...rgbaAttributes(8), shutter, d]),

  ...mega64ProfileModes("university-mega-64-profile-ep", "Eliminator Lighting", "MEGA 64 Profile EP", "MEGA 64 Profile EP", mega64ProfileEpManual),
  ...mega64ProfileModes("university-mega-64-profile-plus", "ADJ", "MEGA 64 Profile Plus", "MEGA 64 Profile Plus", mega64ProfilePlusManual),
];

export const universityRigFixtureCount = 8;
export const universityRigModeCount = 40;

if (universityRigProfiles.length !== universityRigModeCount) {
  throw new Error(`University rig pack must contain ${universityRigModeCount} modes`);
}
if (new Set(universityRigProfiles.map((entry) => entry.fixture_family)).size !== universityRigFixtureCount) {
  throw new Error(`University rig pack must contain ${universityRigFixtureCount} fixture families`);
}

const f3200aLaserMode6Attributes = [
  e("Generic: Light Off/On"),
  e("Generic: Movement Sound/Auto Control"),
  e("Generic: Effect Library Selection"),
  e("Generic: Scene Selection"),
  e("Generic: Color Selection"),
  e("Generic: Movement Speed"),
];

const f3200aLaserMode34Attributes = [
  e("Generic: Pattern 1 Off/On"),
  e("Generic: Pattern Size 1"),
  e("Generic: Effect Library Selection 1"),
  e("Generic: Pattern Selection 1"),
  e("Generic: Pattern Zoom 1"),
  e("Generic: Pattern Rotation 1"),
  e("Generic: Horizontal Movement 1"),
  e("Generic: Vertical Movement 1"),
  e("Generic: Horizontal Zoom 1"),
  e("Generic: Vertical Zoom 1"),
  e("Generic: Compulsive Section Color 1"),
  e("Generic: Color Selection 1"),
  e("Generic: Dots/Dots Disconnection Control 1"),
  e("Generic: Miscellaneous Function 1"),
  e("Generic: Gradual Drawing Control 1"),
  e("Generic: Pattern Distortion Effect's Miscellaneous Function Control 1"),
  e("Generic: Grating Gobo Selection"),
  e("Generic: Pattern 2 Off/On"),
  e("Generic: Pattern Size 2"),
  e("Generic: Array Each Unit's Divergent Angle"),
  e("Generic: Pattern Selection 2"),
  e("Generic: Pattern Zoom 2"),
  e("Generic: Pattern Rotation 2"),
  e("Generic: Horizontal Movement 2"),
  e("Generic: Vertical Movement 2"),
  e("Generic: Horizontal Zoom 2"),
  e("Generic: Vertical Zoom 2"),
  e("Generic: Compulsive Section Color 2"),
  e("Generic: Color Selection 2"),
  e("Generic: Dots/Dots Disconnection Control 2"),
  e("Generic: Miscellaneous Function 2"),
  e("Generic: Gradual Drawing Control 2"),
  e("Generic: Pattern Distortion Effect's Miscellaneous Function Control 2"),
  e("Generic: Projection Range Control"),
];

const wristbandAttributes = (segments: number) =>
  Array.from({ length: segments }, (_, segmentIndex) => {
    const suffix = segmentIndex === 0 ? "" : segmentIndex + 1;
    return [
      e(`Shutter${segmentIndex + 1}`),
      e(`ColorAdd_R${suffix}`),
      e(`ColorAdd_G${suffix}`),
      e(`ColorAdd_B${suffix}`),
    ];
  }).flat();

const soundWavesManualSource = "User-supplied 960 sound waves strongpoint hardware manual channel map";
const daslightSource = (sourceFile: string) => `Daslight ScanLibrary channel map recovered from ${sourceFile}`;

const soundWavesProfile = (
  footprint: number,
  modeName: string,
  deployedInDsf2026: boolean,
  attributes: readonly VerifiedFixtureAttributeDefinition[],
) => personalRigProfile(
  `personal-960-sound-waves-strongpoint-${footprint}ch`,
  "960 sound waves strongpoint",
  "960 sound waves strongpoint",
  modeName,
  footprint,
  soundWavesManualSource,
  deployedInDsf2026,
  attributes,
);

const soundWavesProfiles: VerifiedFixtureProfileSummary[] = [
  soundWavesProfile(3, "3-channel · RGB", false, rgbAttributes(1)),
  soundWavesProfile(4, "4-channel · master + RGB", false, [d, ...rgbAttributes(1)]),
  soundWavesProfile(8, "8-channel · programs", false, [
    d,
    shutter,
    e("Generic: Effect / Voice Mode"),
    e("Generic: Effect Speed / Voice Sensitivity"),
    e("Color1"),
    ...rgbAttributes(1),
  ]),
  soundWavesProfile(12, "12-channel · 4 blocks", false, rgbAttributes(4)),
  soundWavesProfile(13, "13-channel · master + 4 blocks", true, [d, ...rgbAttributes(4)]),
  soundWavesProfile(24, "24-channel · 8 blocks", false, rgbAttributes(8)),
  soundWavesProfile(25, "25-channel · master + 8 blocks", false, [d, ...rgbAttributes(8)]),
  soundWavesProfile(60, "60-channel · 20 groups", false, rgbAttributes(20)),
  soundWavesProfile(61, "61-channel · master + 20 groups", false, [d, ...rgbAttributes(20)]),
  soundWavesProfile(120, "120-channel · 40 groups", false, rgbAttributes(40)),
  soundWavesProfile(121, "121-channel · master + 40 groups", false, [d, ...rgbAttributes(40)]),
];

export const personalRigProfiles: VerifiedFixtureProfileSummary[] = [
  ...soundWavesProfiles,
  personalRigProfile(
    "personal-mini-moving-head-gobo-light-10ch",
    "Mini Moving Head Gobo Light",
    "Mini Moving Head Gobo Light",
    "10-channel · basic",
    10,
    "Daslight ScanLibrary and user-supplied Mini Moving Head Gobo Light manual channel map",
    true,
    [
      e("Pan"), e("Tilt"), e("Color1"), e("Gobo1"), shutter,
      d, e("PanTiltSpeed"), e("Generic: Rotation Direction / Speed"),
      e("Generic: Auto Motion / Reset"), e("Generic: Light Strip Color / Auto"),
    ],
  ),
  personalRigProfile(
    "personal-mini-moving-head-gobo-light-12ch",
    "Mini Moving Head Gobo Light",
    "Mini Moving Head Gobo Light",
    "12-channel · fine + programs",
    12,
    "User-supplied Mini Moving Head Gobo Light manual channel map",
    false,
    [
      s("Pan"), s("Tilt"), e("Color1"), e("Gobo1"), shutter,
      d, e("PanTiltSpeed"), e("Generic: Auto / Sound Program"),
      e("Generic: Auto Motion / Reset"), e("Generic: Light Strip Color / Auto"),
    ],
  ),
  personalRigProfile(
    "personal-f3200a-laser-6ch",
    "F3200A Laser",
    "F3200A Laser",
    "Mode 1 · 6-channel",
    6,
    daslightSource("F3200A Laser (2).ssl2"),
    false,
    f3200aLaserMode6Attributes,
  ),
  personalRigProfile(
    "personal-f3200a-laser-34ch",
    "F3200A Laser",
    "F3200A Laser",
    "Mode 2 · 34-channel",
    34,
    daslightSource("F3200A Laser (2).ssl2"),
    true,
    f3200aLaserMode34Attributes,
  ),
  personalRigProfile(
    "personal-wristband-4ch",
    "wristband",
    "wristband",
    "Mode 1 · 4-channel",
    4,
    daslightSource("wristband.ssl2"),
    false,
    wristbandAttributes(1),
  ),
  personalRigProfile(
    "personal-wristband-8ch",
    "wristband",
    "wristband",
    "Mode 2 · 8-channel",
    8,
    daslightSource("wristband.ssl2"),
    false,
    wristbandAttributes(2),
  ),
  personalRigProfile(
    "personal-wristband-12ch",
    "wristband",
    "wristband",
    "Mode 3 · 12-channel",
    12,
    daslightSource("wristband.ssl2"),
    true,
    wristbandAttributes(3),
  ),
];

export const personalRigFixtureCount = 4;
export const personalRigModeCount = 18;

if (personalRigProfiles.length !== personalRigModeCount) {
  throw new Error(`Personal rig pack must contain ${personalRigModeCount} modes`);
}
if (new Set(personalRigProfiles.map((entry) => entry.fixture_family)).size !== personalRigFixtureCount) {
  throw new Error(`Personal rig pack must contain ${personalRigFixtureCount} fixture families`);
}

const genericVerifiedProfiles: VerifiedFixtureProfileSummary[] = [
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

export const previewVerifiedProfiles: VerifiedFixtureProfileSummary[] = [
  ...universityRigProfiles,
  ...personalRigProfiles,
  ...genericVerifiedProfiles,
];

export const verifiedFixtureProfileCount = 118;
export const verifiedFixtureCategoryCount = 8;

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
