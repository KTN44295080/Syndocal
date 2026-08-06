// Converts an Open Fixture Library snapshot (MIT, Florian & Felix Edelmann)
// into Syndocal's compact bundled-library format.
//
//   node scripts/build-ofl-library.mjs <path-to-ofl-checkout>
//
// Output: src/generated/oflLibrary.json  (lazy-loaded; never in the main chunk)
//
// The bundle is intentionally lossy in the same way Syndocal's own custom
// profiles are: every mode becomes an ordered list of `Attribute@offset:bits`
// slots, which is exactly what create_custom_fixture_profile consumes. Channel
// semantics beyond the attribute identity (capability ranges, wheel slots,
// physical data) are NOT carried - operators who need them download the
// manufacturer GDTF from Share.
import { readdirSync, readFileSync, mkdirSync, writeFileSync, statSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const appDir = join(scriptDir, "..");
const oflRoot = process.argv[2];
if (!oflRoot) {
  console.error("usage: node scripts/build-ofl-library.mjs <path-to-ofl-checkout>");
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

// OFL capability type -> Syndocal attribute. Unknown types fall through to a
// stable generic slot so the footprint always matches the real fixture.
const attributeForCapability = (capability, channelName) => {
  switch (capability?.type) {
    case "Intensity":
      return "Dimmer";
    case "ColorIntensity":
      return colorAttribute(capability.color);
    case "ColorPreset":
      return "Color1";
    case "ColorTemperature":
      return "CTO";
    case "Pan":
      return "Pan";
    case "Tilt":
      return "Tilt";
    case "PanTiltSpeed":
      return "PanTiltSpeed";
    case "ShutterStrobe":
    case "StrobeSpeed":
    case "StrobeDuration":
      return "Shutter1";
    case "WheelSlot":
    case "WheelShake":
    case "WheelSlotRotation":
    case "WheelRotation":
      return wheelAttribute(capability.wheel ?? channelName);
    case "Zoom":
      return "Zoom";
    case "Focus":
      return "Focus1";
    case "Iris":
    case "IrisEffect":
      return "Iris";
    case "Prism":
    case "PrismRotation":
      return "Prism1";
    case "Frost":
    case "FrostEffect":
      return "Frost1";
    case "Fog":
    case "FogOutput":
    case "FogType":
      return "Fog";
    case "BladeInsertion":
    case "BladeRotation":
    case "BladeSystemRotation":
      return "Blade1A";
    case "Speed":
    case "EffectSpeed":
    case "EffectDuration":
    case "EffectParameter":
    case "Effect":
    case "SoundSensitivity":
    case "Rotation":
    case "Time":
    case "Maintenance":
    case "Generic":
    case "NoFunction":
    default:
      return null;
  }
};

// A channel's identity is the first capability that maps to something real;
// channels that are purely effects/maintenance get a numbered generic slot.
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

const manufacturers = JSON.parse(readFileSync(join(oflRoot, "fixtures", "manufacturers.json"), "utf8"));
const fixtureDirs = readdirSync(join(oflRoot, "fixtures"))
  .filter((entry) => statSync(join(oflRoot, "fixtures", entry)).isDirectory())
  .sort();

const bundleFixtures = [];
let skippedModes = 0;
let skippedFixtures = 0;

for (const slug of fixtureDirs) {
  const manufacturerName = manufacturers[slug]?.name;
  if (!manufacturerName) continue;
  const files = readdirSync(join(oflRoot, "fixtures", slug)).filter((file) => file.endsWith(".json")).sort();
  for (const file of files) {
    let fixture;
    try {
      fixture = JSON.parse(readFileSync(join(oflRoot, "fixtures", slug, file), "utf8"));
    } catch {
      skippedFixtures += 1;
      continue;
    }
    if (!fixture?.name || !Array.isArray(fixture.modes)) continue;
    const available = fixture.availableChannels ?? {};
    // Fine channels are listed in modes under their alias; map alias -> coarse.
    const fineOwner = new Map();
    for (const [name, channel] of Object.entries(available)) {
      for (const alias of channel?.fineChannelAliases ?? []) fineOwner.set(alias, name);
    }

    const modes = [];
    for (const mode of fixture.modes) {
      const channels = mode?.channels;
      // Matrix modes embed channel objects (templates/inserts); their real
      // footprint depends on pixel counts, so they are out of scope here.
      if (!Array.isArray(channels) || channels.some((channel) => typeof channel !== "string" && channel !== null)) {
        skippedModes += 1;
        continue;
      }
      const slots = [];
      let genericIndex = 0;
      for (let index = 0; index < channels.length; index += 1) {
        const name = channels[index];
        if (name === null) {
          // OFL uses null for "unused DMX slot".
          genericIndex += 1;
          slots.push({ attribute: `Unused${genericIndex}`, bits: 8 });
          continue;
        }
        const coarseName = fineOwner.get(name);
        if (coarseName) {
          // Fine channel: widen the previous slot when it is its coarse
          // partner and directly adjacent, otherwise keep an 8-bit slot.
          const previous = slots[slots.length - 1];
          if (previous && previous.sourceName === coarseName && previous.bits === 8) {
            previous.bits = 16;
            continue;
          }
          genericIndex += 1;
          slots.push({ attribute: `Fine${genericIndex}`, bits: 8, sourceName: name });
          continue;
        }
        const attribute = attributeForChannel(available[name], name);
        if (attribute) {
          slots.push({ attribute, bits: 8, sourceName: name });
        } else {
          genericIndex += 1;
          slots.push({ attribute: `Control${genericIndex}`, bits: 8, sourceName: name });
        }
      }
      if (slots.length === 0) {
        skippedModes += 1;
        continue;
      }
      // Attribute identities must be unique inside a mode (Syndocal patches by
      // attribute name); suffix duplicates in order.
      const seen = new Map();
      let offset = 1;
      const attributes = slots.map((slot) => {
        const count = (seen.get(slot.attribute) ?? 0) + 1;
        seen.set(slot.attribute, count);
        const attribute = count === 1 ? slot.attribute : `${slot.attribute}_${count}`;
        const encoded = `${attribute}@${offset}:${slot.bits}`;
        offset += slot.bits === 16 ? 2 : 1;
        return encoded;
      });
      modes.push({ n: mode.name, a: attributes });
    }
    if (modes.length === 0) {
      skippedFixtures += 1;
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

const bundle = {
  v: 1,
  source: "Open Fixture Library",
  license: "MIT",
  copyright: "Copyright (c) 2017 Florian & Felix Edelmann and OFL contributors",
  url: "https://github.com/OpenLightingProject/open-fixture-library",
  fixtures: bundleFixtures,
};

const outDir = join(appDir, "src", "generated");
mkdirSync(outDir, { recursive: true });
const outPath = join(outDir, "oflLibrary.json");
writeFileSync(outPath, JSON.stringify(bundle));
const bytes = statSync(outPath).size;
console.log(
  `bundled ${bundleFixtures.length} fixtures / ` +
    `${bundleFixtures.reduce((total, fixture) => total + fixture.modes.length, 0)} modes ` +
    `(${(bytes / 1024).toFixed(0)} kB); skipped ${skippedFixtures} fixtures, ${skippedModes} matrix/empty modes`,
);
