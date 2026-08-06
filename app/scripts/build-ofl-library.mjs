// Converts an Open Fixture Library snapshot (MIT, Florian & Felix Edelmann)
// into Syndocal's compact bundled-library format.
//
//   node scripts/build-ofl-library.mjs <path-to-ofl-checkout> [source-revision]
//
// Pass the upstream commit SHA as the second argument so the bundle records
// exactly which snapshot shipped. Regenerate against a pinned checkout, never
// a moving master download, when refreshing for a release.
//
// Output: src/generated/oflLibrary.json (lazy-loaded; never in main chunk)
//
// The conversion carries the ordered DMX attribute layout consumed by
// create_custom_fixture_profile. Capability ranges, wheel media and physical
// data remain intentionally out of scope. Matrix insert blocks are expanded
// according to OFL's documented repeatFor and channelOrder rules.
import { readdirSync, readFileSync, mkdirSync, writeFileSync, statSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { encodeAttributeSlots, encodedAttributeFootprint } from "./profile-library-common.mjs";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const appDir = join(scriptDir, "..");
const oflRoot = process.argv[2];
if (!oflRoot) {
  console.error("usage: node scripts/build-ofl-library.mjs <path-to-ofl-checkout> [source-revision]");
  process.exit(1);
}

const colorAttribute = (color) => {
  const map = {
    Red: "ColorAdd_R",
    Green: "ColorAdd_G",
    Blue: "ColorAdd_B",
    White: "ColorAdd_W",
    "Warm White": "ColorAdd_WW",
    "Cold White": "ColorAdd_CW",
    Amber: "ColorAdd_A",
    UV: "ColorAdd_UV",
    Lime: "ColorAdd_L",
    Cyan: "ColorAdd_C",
    Magenta: "ColorAdd_M",
    Yellow: "ColorAdd_Y",
    Indigo: "ColorAdd_IN",
  };
  return map[color] ?? "ColorAdd_W";
};

const wheelAttribute = (name) => (/gobo/i.test(name) ? "Gobo1" : "Color1");

const attributeForCapability = (capability, channelName) => {
  switch (capability?.type) {
    case "Intensity": return "Dimmer";
    case "ColorIntensity": return colorAttribute(capability.color);
    case "ColorPreset": return "Color1";
    case "ColorTemperature": return "CTO";
    case "Pan": return "Pan";
    case "Tilt": return "Tilt";
    case "PanTiltSpeed": return "PanTiltSpeed";
    case "ShutterStrobe":
    case "StrobeSpeed":
    case "StrobeDuration": return "Shutter1";
    case "WheelSlot":
    case "WheelShake":
    case "WheelSlotRotation":
    case "WheelRotation": return wheelAttribute(capability.wheel ?? channelName);
    case "Zoom": return "Zoom";
    case "Focus": return "Focus1";
    case "Iris":
    case "IrisEffect": return "Iris";
    case "Prism":
    case "PrismRotation": return "Prism1";
    case "Frost":
    case "FrostEffect": return "Frost1";
    case "Fog":
    case "FogOutput":
    case "FogType": return "Fog";
    case "BladeInsertion":
    case "BladeRotation":
    case "BladeSystemRotation": return "Blade1A";
    default: return null;
  }
};

const attributeForChannel = (channel, channelName) => {
  const capabilities = channel?.capability ? [channel.capability] : (channel?.capabilities ?? []);
  for (const capability of capabilities) {
    const attribute = attributeForCapability(capability, channelName);
    if (attribute) return attribute;
  }
  if (/dimmer|intensity|master/i.test(channelName)) return "Dimmer";
  if (/strob|shutter/i.test(channelName)) return "Shutter1";
  if (/gobo/i.test(channelName)) return "Gobo1";
  if (/colou?r/i.test(channelName)) return "Color1";
  if (/zoom/i.test(channelName)) return "Zoom";
  if (/focus/i.test(channelName)) return "Focus1";
  if (/iris/i.test(channelName)) return "Iris";
  if (/prism/i.test(channelName)) return "Prism1";
  return null;
};

const defaultPixelKey = (x, y, z, pixelCount) => {
  const definedAxes = pixelCount.filter((count) => count > 1).length;
  if (definedAxes === 1) return String(Math.max(x, y, z));
  if (definedAxes === 2) {
    const positions = [x, y, z].filter((_, index) => pixelCount[index] > 1);
    return `(${positions[0]}, ${positions[1]})`;
  }
  return `(${x}, ${y}, ${z})`;
};

const matrixPixels = (matrix) => {
  const pixelCount = matrix?.pixelCount ?? (() => {
    const structure = matrix?.pixelKeys ?? [];
    return [
      Math.max(1, ...structure.flatMap((plane) => plane.map((row) => row.length))),
      Math.max(1, ...structure.map((plane) => plane.length)),
      Math.max(1, structure.length),
    ];
  })();
  const structure = matrix?.pixelKeys ?? Array.from({ length: pixelCount[2] }, (_, z) =>
    Array.from({ length: pixelCount[1] }, (_, y) =>
      Array.from({ length: pixelCount[0] }, (_, x) =>
        defaultPixelKey(x + 1, y + 1, z + 1, pixelCount))));
  const pixels = [];
  for (let z = 0; z < structure.length; z += 1) {
    for (let y = 0; y < structure[z].length; y += 1) {
      for (let x = 0; x < structure[z][y].length; x += 1) {
        const key = structure[z][y][x];
        if (key !== null) pixels.push({ key, position: [x + 1, y + 1, z + 1] });
      }
    }
  }
  return pixels;
};

const repeatPixelKeys = (fixture, repeatFor) => {
  if (Array.isArray(repeatFor)) return repeatFor;
  if (repeatFor === "eachPixelGroup") return Object.keys(fixture.matrix?.pixelGroups ?? {});
  const pixels = matrixPixels(fixture.matrix);
  if (repeatFor === "eachPixelABC") {
    return pixels.map((pixel) => pixel.key).sort((left, right) =>
      left.localeCompare(right, undefined, { numeric: true }));
  }
  const axes = repeatFor.replace("eachPixel", "");
  const axisIndex = { X: 0, Y: 1, Z: 2 };
  return pixels.sort((left, right) => {
    for (const axis of [...axes].reverse()) {
      const difference = left.position[axisIndex[axis]] - right.position[axisIndex[axis]];
      if (difference !== 0) return difference;
    }
    return 0;
  }).map((pixel) => pixel.key);
};

const expandModeChannels = (fixture, channels) => channels.flatMap((channel) => {
  if (typeof channel === "string" || channel === null) return [channel];
  if (channel?.insert !== "matrixChannels") return [];
  const pixelKeys = repeatPixelKeys(fixture, channel.repeatFor);
  const resolve = (template, pixelKey) => template === null
    ? null
    : template.replaceAll("$pixelKey", pixelKey);
  return channel.channelOrder === "perChannel"
    ? channel.templateChannels.flatMap((template) => pixelKeys.map((pixelKey) => resolve(template, pixelKey)))
    : pixelKeys.flatMap((pixelKey) => channel.templateChannels.map((template) => resolve(template, pixelKey)));
});

const escapeRegex = (value) => value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
const templateMatcher = (template) => new RegExp(`^${template.split("$pixelKey").map(escapeRegex).join("(.+)")}$`);

const channelResolver = (fixture) => {
  const available = fixture.availableChannels ?? {};
  const fineOwner = new Map();
  for (const [name, channel] of Object.entries(available)) {
    for (const alias of channel?.fineChannelAliases ?? []) fineOwner.set(alias, name);
  }
  const templates = [];
  for (const [key, definition] of Object.entries(fixture.templateChannels ?? {})) {
    templates.push({ matcher: templateMatcher(key), definition, ownerTemplate: key, fine: false });
    for (const alias of definition?.fineChannelAliases ?? []) {
      templates.push({ matcher: templateMatcher(alias), definition, ownerTemplate: key, fine: true });
    }
    for (const alias of definition?.switchingChannelAliases ?? []) {
      templates.push({ matcher: templateMatcher(alias), definition, ownerTemplate: key, fine: false });
    }
  }
  return (name) => {
    if (available[name]) return { definition: available[name], coarseName: null };
    if (fineOwner.has(name)) return { definition: available[fineOwner.get(name)], coarseName: fineOwner.get(name) };
    for (const template of templates) {
      const match = name.match(template.matcher);
      if (!match) continue;
      return {
        definition: template.definition,
        coarseName: template.fine
          ? template.ownerTemplate.replaceAll("$pixelKey", match[1])
          : null,
      };
    }
    return { definition: null, coarseName: null };
  };
};

const manufacturers = JSON.parse(readFileSync(join(oflRoot, "fixtures", "manufacturers.json"), "utf8"));
const fixtureDirs = readdirSync(join(oflRoot, "fixtures"))
  .filter((entry) => statSync(join(oflRoot, "fixtures", entry)).isDirectory())
  .sort();

const bundleFixtures = [];
const audit = {
  inputFixtures: 0,
  inputModes: 0,
  bundledFixtures: 0,
  bundledModes: 0,
  matrixModesExpanded: 0,
  malformedFixtures: 0,
  emptyFixtures: 0,
  skippedModes: 0,
  unresolvedChannels: 0,
};

for (const slug of fixtureDirs) {
  const manufacturerName = manufacturers[slug]?.name;
  if (!manufacturerName) continue;
  const files = readdirSync(join(oflRoot, "fixtures", slug)).filter((file) => file.endsWith(".json")).sort();
  for (const file of files) {
    audit.inputFixtures += 1;
    let fixture;
    try {
      fixture = JSON.parse(readFileSync(join(oflRoot, "fixtures", slug, file), "utf8"));
    } catch {
      audit.malformedFixtures += 1;
      continue;
    }
    if (!fixture?.name || !Array.isArray(fixture.modes)) {
      audit.emptyFixtures += 1;
      continue;
    }
    audit.inputModes += fixture.modes.length;
    const resolveChannel = channelResolver(fixture);
    const modes = [];
    for (const mode of fixture.modes) {
      if (!Array.isArray(mode?.channels)) {
        audit.skippedModes += 1;
        continue;
      }
      const hasMatrixInsert = mode.channels.some((channel) => channel && typeof channel === "object");
      const channels = expandModeChannels(fixture, mode.channels);
      if (hasMatrixInsert) audit.matrixModesExpanded += 1;
      const slots = [];
      let genericIndex = 0;
      for (const name of channels) {
        if (name === null) {
          genericIndex += 1;
          slots.push({ attribute: `Unused${genericIndex}`, bits: 8, sourceName: null });
          continue;
        }
        const resolved = resolveChannel(name);
        if (resolved.coarseName) {
          const previous = slots[slots.length - 1];
          if (previous && previous.sourceName === resolved.coarseName && previous.bits === 8) {
            previous.bits = 16;
            continue;
          }
          genericIndex += 1;
          slots.push({ attribute: `Fine${genericIndex}`, bits: 8, sourceName: name });
          continue;
        }
        if (!resolved.definition) audit.unresolvedChannels += 1;
        const attribute = attributeForChannel(resolved.definition, name);
        if (attribute) slots.push({ attribute, bits: 8, sourceName: name });
        else {
          genericIndex += 1;
          slots.push({ attribute: `Control${genericIndex}`, bits: 8, sourceName: name });
        }
      }
      if (slots.length === 0) {
        audit.skippedModes += 1;
        continue;
      }
      const attributes = encodeAttributeSlots(slots);
      if (encodedAttributeFootprint(attributes) !== channels.length) {
        throw new Error(`${manufacturerName} ${fixture.name} ${mode.name}: converted footprint drifted from ${channels.length}`);
      }
      modes.push({ n: mode.name, a: attributes });
    }
    if (modes.length === 0) {
      audit.emptyFixtures += 1;
      continue;
    }
    bundleFixtures.push({
      m: manufacturerName,
      n: fixture.name,
      c: (fixture.categories ?? [])[0] ?? "Other",
      modes,
    });
  }
}

audit.bundledFixtures = bundleFixtures.length;
audit.bundledModes = bundleFixtures.reduce((total, fixture) => total + fixture.modes.length, 0);

const bundle = {
  v: 2,
  source: "Open Fixture Library",
  sourceRevision: process.argv[3] ?? "unpinned",
  license: "MIT",
  copyright: "Copyright (c) 2017 Florian & Felix Edelmann and OFL contributors",
  url: "https://github.com/OpenLightingProject/open-fixture-library",
  priority: 100,
  audit,
  fixtures: bundleFixtures,
};

const outDir = join(appDir, "src", "generated");
mkdirSync(outDir, { recursive: true });
const outPath = join(outDir, "oflLibrary.json");
writeFileSync(outPath, JSON.stringify(bundle));
const bytes = statSync(outPath).size;
console.log(
  `bundled ${audit.bundledFixtures}/${audit.inputFixtures} fixtures, ` +
    `${audit.bundledModes}/${audit.inputModes} modes (${(bytes / 1024).toFixed(0)} kB); ` +
    `expanded ${audit.matrixModesExpanded} matrix modes; ` +
    `skipped ${audit.malformedFixtures} malformed fixtures, ${audit.emptyFixtures} empty fixtures, ` +
    `${audit.skippedModes} modes; ${audit.unresolvedChannels} unresolved channels preserved as generic controls`,
);
