// Converts Q Light Controller Plus fixture definitions (Apache-2.0) into the
// compact offline profile representation used by Syndocal.
//
//   node scripts/build-qlc-library.mjs <path-to-qlcplus-checkout> [source-revision] [ofl-bundle]
//
// OFL has higher precedence. An exact normalized manufacturer + model match is
// omitted from this supplemental bundle and recorded in the conversion audit.
// Output: src/generated/qlcLibrary.json (lazy-loaded; never in main chunk)
import { readdirSync, readFileSync, mkdirSync, writeFileSync, statSync } from "node:fs";
import { join, dirname, basename } from "node:path";
import { fileURLToPath } from "node:url";
import {
  encodeAttributeSlots,
  encodedAttributeFootprint,
  fixtureIdentity,
} from "./profile-library-common.mjs";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const appDir = join(scriptDir, "..");
const qlcRoot = process.argv[2];
if (!qlcRoot) {
  console.error("usage: node scripts/build-qlc-library.mjs <path-to-qlcplus-checkout> [source-revision] [ofl-bundle]");
  process.exit(1);
}
const fixturesRoot = join(qlcRoot, "resources", "fixtures");
const oflBundlePath = process.argv[4] ?? join(appDir, "src", "generated", "oflLibrary.json");

const decodeXml = (value) => value
  .replace(/&#x([0-9a-f]+);/gi, (_, digits) => String.fromCodePoint(Number.parseInt(digits, 16)))
  .replace(/&#(\d+);/g, (_, digits) => String.fromCodePoint(Number.parseInt(digits, 10)))
  .replace(/&quot;/g, "\"")
  .replace(/&apos;/g, "'")
  .replace(/&lt;/g, "<")
  .replace(/&gt;/g, ">")
  .replace(/&amp;/g, "&");

const xmlAttribute = (attributes, name) => {
  const match = attributes.match(new RegExp(`\\b${name}=(?:"([^"]*)"|'([^']*)')`));
  return match ? decodeXml(match[1] ?? match[2]) : null;
};

const xmlText = (xml, tag) => {
  const match = xml.match(new RegExp(`<${tag}\\b[^>]*>([\\s\\S]*?)<\\/${tag}>`));
  return match ? decodeXml(match[1].trim()) : null;
};

const collectFiles = (directory) => readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
  const path = join(directory, entry.name);
  if (entry.isDirectory()) return collectFiles(path);
  return entry.name.endsWith(".qxf") ? [path] : [];
}).sort();

const colorPresetAttribute = (preset) => {
  const colors = {
    Red: "ColorAdd_R",
    Green: "ColorAdd_G",
    Blue: "ColorAdd_B",
    White: "ColorAdd_W",
    Amber: "ColorAdd_A",
    UV: "ColorAdd_UV",
    Lime: "ColorAdd_L",
    Cyan: "ColorAdd_C",
    Magenta: "ColorAdd_M",
    Yellow: "ColorAdd_Y",
    Indigo: "ColorAdd_IN",
  };
  const match = preset.match(/^Intensity(Red|Green|Blue|White|Amber|UV|Lime|Cyan|Magenta|Yellow|Indigo)$/);
  return match ? colors[match[1]] : null;
};

const attributeForPreset = (presetValue) => {
  const preset = presetValue.replace(/Fine$/, "");
  const color = colorPresetAttribute(preset);
  if (color) return color;
  if (/^Intensity(?:Master)?Dimmer$|^IntensityValue$/.test(preset)) return "Dimmer";
  if (/^PositionPan$|^PositionXAxis$/.test(preset)) return "Pan";
  if (/^PositionTilt$|^PositionYAxis$/.test(preset)) return "Tilt";
  if (/^SpeedPanTilt/.test(preset)) return "PanTiltSpeed";
  if (/^(?:Shutter|Strobe|Pulse|Ramp)/.test(preset)) return "Shutter1";
  if (/^Color(?:Wheel|Macro|DoubleMacro)/.test(preset)) return "Color1";
  if (/^Color(?:CTO|CTC|CTB)Mixer$/.test(preset)) return "CTO";
  if (/^Gobo/.test(preset)) return "Gobo1";
  if (/^BeamFocus/.test(preset)) return "Focus1";
  if (/^BeamZoom/.test(preset)) return "Zoom";
  if (/^ShutterIris/.test(preset)) return "Iris";
  if (/^Prism/.test(preset)) return "Prism1";
  return null;
};

const colorNameAttribute = (name) => {
  const tests = [
    [/\bred\b/i, "ColorAdd_R"],
    [/\bgreen\b/i, "ColorAdd_G"],
    [/\bblue\b/i, "ColorAdd_B"],
    [/\b(?:warm\s*)?white\b/i, "ColorAdd_W"],
    [/\bamber\b/i, "ColorAdd_A"],
    [/\b(?:uv|ultra\s*violet)\b/i, "ColorAdd_UV"],
    [/\blime\b/i, "ColorAdd_L"],
    [/\bcyan\b/i, "ColorAdd_C"],
    [/\bmagenta\b/i, "ColorAdd_M"],
    [/\byellow\b/i, "ColorAdd_Y"],
    [/\bindigo\b/i, "ColorAdd_IN"],
  ];
  return tests.find(([pattern]) => pattern.test(name))?.[1] ?? null;
};

const attributeForChannel = (channel) => {
  for (const preset of channel.presets) {
    const attribute = attributeForPreset(preset);
    if (attribute) return attribute;
  }
  const name = channel.name;
  const color = colorNameAttribute(name);
  if (color) return color;
  if (/pan\s*[/&-]?\s*tilt.*speed|speed.*pan\s*[/&-]?\s*tilt/i.test(name)) return "PanTiltSpeed";
  if (/\bpan\b/i.test(name)) return "Pan";
  if (/\btilt\b/i.test(name)) return "Tilt";
  if (/master\s*dimmer|\bdimmer\b|\bintensity\b/i.test(name)) return "Dimmer";
  if (/\bblade\b/i.test(name)) return "Blade1A";
  if (/\bstrob|\bshutter/i.test(name)) return "Shutter1";
  if (/\bgobo\b/i.test(name)) return "Gobo1";
  if (/\bcolou?r\b/i.test(name) || channel.group === "Colour") return "Color1";
  if (/\bzoom\b/i.test(name)) return "Zoom";
  if (/\bfocus\b/i.test(name)) return "Focus1";
  if (/\biris\b/i.test(name)) return "Iris";
  if (/\bprism\b/i.test(name) || channel.group === "Prism") return "Prism1";
  if (/\bfrost\b/i.test(name)) return "Frost1";
  if (/\b(?:fog|haze|smoke)\b/i.test(name)) return "Fog";
  if (channel.group === "Intensity") return "Dimmer";
  if (channel.group === "Shutter") return "Shutter1";
  if (channel.group === "Gobo") return "Gobo1";
  if (channel.group === "Pan") return "Pan";
  if (channel.group === "Tilt") return "Tilt";
  return null;
};

const fineChannel = (channel) =>
  channel.presets.some((preset) => /Fine$/.test(preset)) || /\bfine\b|\blsb\b/i.test(channel.name);

const canonicalPairName = (name) => name
  .toLocaleLowerCase()
  .replace(/\b(?:coarse|fine|msb|lsb|16[ -]?bit)\b/g, "")
  .replace(/[^a-z0-9]+/g, "");

const fineMatchesCoarse = (fine, coarse) => {
  if (!fineChannel(fine)) return false;
  const finePresets = fine.presets
    .filter((preset) => /Fine$/.test(preset))
    .map((preset) => preset.replace(/Fine$/, ""));
  if (finePresets.some((preset) => coarse.presets.includes(preset))) return true;
  return canonicalPairName(fine.name) === canonicalPairName(coarse.name);
};

const parseChannels = (xml) => {
  const firstMode = xml.search(/<Mode\b/);
  const header = firstMode >= 0 ? xml.slice(0, firstMode) : xml;
  const channels = new Map();
  const pattern = /<Channel\b([^>]*?)(?:\/>|>([\s\S]*?)<\/Channel>)/g;
  for (const match of header.matchAll(pattern)) {
    const name = xmlAttribute(match[1], "Name");
    if (!name) continue;
    const body = match[2] ?? "";
    const groupMatch = body.match(/<Group\b([^>]*)>([\s\S]*?)<\/Group>/);
    const presets = [
      xmlAttribute(match[1], "Preset"),
      ...[...body.matchAll(/<Capability\b([^>]*)>/g)].map((capability) => xmlAttribute(capability[1], "Preset")),
    ].filter((preset, index, all) => preset && all.indexOf(preset) === index);
    channels.set(name, {
      name,
      group: groupMatch ? decodeXml(groupMatch[2].trim()) : "",
      byte: groupMatch ? xmlAttribute(groupMatch[1], "Byte") : null,
      presets,
    });
  }
  return channels;
};

const parseModes = (xml) => {
  const modes = [];
  for (const match of xml.matchAll(/<Mode\b([^>]*)>([\s\S]*?)<\/Mode>/g)) {
    const name = xmlAttribute(match[1], "Name");
    if (!name) continue;
    const entries = [...match[2].matchAll(/<Channel\b([^>]*)>([\s\S]*?)<\/Channel>/g)]
      .map((channel) => {
        const number = xmlAttribute(channel[1], "Number");
        return {
          number: number === null ? null : Number(number),
          name: decodeXml(channel[2].trim()),
        };
      })
      .filter((channel) => Number.isInteger(channel.number) && channel.number >= 0)
      .sort((left, right) => left.number - right.number);
    modes.push({ name, entries });
  }
  return modes;
};

const convertMode = (manufacturer, model, mode, channels, audit) => {
  if (mode.entries.length === 0) return null;
  const sourceFootprint = mode.entries[mode.entries.length - 1].number + 1;
  const numbers = new Set(mode.entries.map((entry) => entry.number));
  if (numbers.size !== mode.entries.length || numbers.size !== sourceFootprint) {
    audit.invalidModeNumbering += 1;
    audit.skippedModes += 1;
    return null;
  }
  if (sourceFootprint > 512) {
    audit.oversizedModes += 1;
    audit.skippedModes += 1;
    return null;
  }
  const slots = [];
  let genericIndex = 0;
  for (const entry of mode.entries) {
    const channel = channels.get(entry.name) ?? {
      name: entry.name,
      group: "",
      byte: null,
      presets: [],
    };
    if (!channels.has(entry.name)) audit.unresolvedChannelReferences += 1;
    if (fineChannel(channel)) {
      const previous = slots[slots.length - 1];
      if (previous && previous.bits === 8 && fineMatchesCoarse(channel, previous.sourceChannel)) {
        previous.bits = 16;
        continue;
      }
      genericIndex += 1;
      slots.push({ attribute: `Fine${genericIndex}`, bits: 8, sourceChannel: channel });
      continue;
    }
    const attribute = attributeForChannel(channel);
    if (attribute) slots.push({ attribute, bits: 8, sourceChannel: channel });
    else {
      genericIndex += 1;
      slots.push({ attribute: `Control${genericIndex}`, bits: 8, sourceChannel: channel });
    }
  }
  const attributes = encodeAttributeSlots(slots);
  if (encodedAttributeFootprint(attributes) !== sourceFootprint) {
    throw new Error(`${manufacturer} ${model} ${mode.name}: converted footprint drifted from ${sourceFootprint}`);
  }
  return { n: mode.name, a: attributes };
};

const ofl = JSON.parse(readFileSync(oflBundlePath, "utf8"));
const higherPriorityIdentities = new Set(ofl.fixtures.map((fixture) => fixtureIdentity(fixture.m, fixture.n)));
const files = collectFiles(fixturesRoot);
const audit = {
  inputFixtures: files.length,
  inputModes: 0,
  bundledFixtures: 0,
  bundledModes: 0,
  deduplicatedByOfl: 0,
  deduplicatedModesByOfl: 0,
  duplicateModesRemoved: 0,
  internallyMergedFixtures: 0,
  malformedFixtures: 0,
  emptyFixtures: 0,
  skippedModes: 0,
  invalidModeNumbering: 0,
  oversizedModes: 0,
  unresolvedChannelReferences: 0,
  adjacentFinePairsMerged: 0,
};
const bundleByIdentity = new Map();

for (const file of files) {
  let xml;
  try {
    xml = readFileSync(file, "utf8");
  } catch {
    audit.malformedFixtures += 1;
    continue;
  }
  const manufacturer = xmlText(xml, "Manufacturer");
  const model = xmlText(xml, "Model");
  const type = xmlText(xml, "Type") ?? "Other";
  const channels = parseChannels(xml);
  const sourceModes = parseModes(xml);
  audit.inputModes += sourceModes.length;
  if (!manufacturer || !model || sourceModes.length === 0) {
    audit.emptyFixtures += 1;
    continue;
  }
  const identity = fixtureIdentity(manufacturer, model);
  if (higherPriorityIdentities.has(identity)) {
    audit.deduplicatedByOfl += 1;
    audit.deduplicatedModesByOfl += sourceModes.length;
    continue;
  }
  const beforePairs = audit.adjacentFinePairsMerged;
  const convertedModes = sourceModes.flatMap((mode) => {
    const converted = convertMode(manufacturer, model, mode, channels, audit);
    if (!converted) return [];
    const sourceBytes = mode.entries.length;
    const convertedControls = converted.a.length;
    audit.adjacentFinePairsMerged += sourceBytes - convertedControls;
    return [converted];
  });
  const exactModes = new Set();
  const modes = convertedModes.filter((mode) => {
    const key = `${mode.n}\u0000${JSON.stringify(mode.a)}`;
    if (exactModes.has(key)) {
      audit.duplicateModesRemoved += 1;
      return false;
    }
    exactModes.add(key);
    return true;
  });
  if (modes.length === 0) {
    audit.adjacentFinePairsMerged = beforePairs;
    audit.emptyFixtures += 1;
    continue;
  }
  const existing = bundleByIdentity.get(identity);
  if (existing) {
    audit.internallyMergedFixtures += 1;
    const modeKeys = new Set(existing.modes.map((mode) => `${mode.n}\u0000${encodedAttributeFootprint(mode.a)}`));
    for (const mode of modes) {
      const key = `${mode.n}\u0000${encodedAttributeFootprint(mode.a)}`;
      if (!modeKeys.has(key)) existing.modes.push(mode);
    }
  } else {
    bundleByIdentity.set(identity, { m: manufacturer, n: model, c: type, modes });
  }
}

const bundleFixtures = [...bundleByIdentity.values()].sort((left, right) =>
  left.m.localeCompare(right.m) || left.n.localeCompare(right.n));
audit.bundledFixtures = bundleFixtures.length;
audit.bundledModes = bundleFixtures.reduce((total, fixture) => total + fixture.modes.length, 0);

const bundle = {
  v: 2,
  source: "Q Light Controller Plus fixture definitions",
  sourceRevision: process.argv[3] ?? "unpinned",
  license: "Apache-2.0",
  copyright: "Copyright QLC+ contributors and fixture definition authors",
  url: "https://github.com/mcallegari/qlcplus",
  priority: 50,
  supersededBy: "Open Fixture Library exact normalized manufacturer/model match",
  audit,
  fixtures: bundleFixtures,
};

const outDir = join(appDir, "src", "generated");
mkdirSync(outDir, { recursive: true });
const outPath = join(outDir, "qlcLibrary.json");
writeFileSync(outPath, JSON.stringify(bundle));
const bytes = statSync(outPath).size;
console.log(
  `bundled ${audit.bundledFixtures}/${audit.inputFixtures} fixtures, ` +
    `${audit.bundledModes}/${audit.inputModes} modes (${(bytes / 1024).toFixed(0)} kB); ` +
    `OFL superseded ${audit.deduplicatedByOfl}, merged ${audit.internallyMergedFixtures}; ` +
    `skipped ${audit.malformedFixtures} malformed fixtures, ${audit.emptyFixtures} empty fixtures, ` +
    `${audit.skippedModes} modes; ${audit.unresolvedChannelReferences} unresolved references preserved; ` +
    `${audit.adjacentFinePairsMerged} adjacent fine pairs merged`,
);
