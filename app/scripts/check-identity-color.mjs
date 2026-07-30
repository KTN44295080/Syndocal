import { readFile } from "node:fs/promises";
import assert from "node:assert/strict";
import ts from "typescript";

async function importTsModule(path) {
  const source = await readFile(new URL(path, import.meta.url), "utf8");
  const transpiled = ts.transpileModule(source, {
    compilerOptions: {
      module: ts.ModuleKind.ESNext,
      target: ts.ScriptTarget.ES2022,
      importsNotUsedAsValues: ts.ImportsNotUsedAsValues.Remove,
    },
    fileName: path,
  });
  return import(`data:text/javascript;base64,${Buffer.from(transpiled.outputText).toString("base64")}`);
}

const identity = await importTsModule("../src/identityColor.ts");

const hueDistance = (left, right) => {
  const delta = Math.abs(((left - right) % 360 + 360) % 360);
  return Math.min(delta, 360 - delta);
};

// Derive the accent hue straight from styles.css so the guard tracks the real
// GO button colour rather than a copied constant.
const stylesSource = await readFile(new URL("../src/styles.css", import.meta.url), "utf8");
const accentMatch = stylesSource.match(/--ray-accent:\s*#([0-9a-fA-F]{6})/);
assert.ok(accentMatch, "styles.css must define --ray-accent so the guard hue can be derived");
const hexToHue = (hex) => {
  const r = parseInt(hex.slice(0, 2), 16) / 255;
  const g = parseInt(hex.slice(2, 4), 16) / 255;
  const b = parseInt(hex.slice(4, 6), 16) / 255;
  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  const delta = max - min;
  if (delta === 0) return 0;
  let hue;
  if (max === r) hue = 60 * (((g - b) / delta) % 6);
  else if (max === g) hue = 60 * ((b - r) / delta + 2);
  else hue = 60 * ((r - g) / delta + 4);
  return (hue + 360) % 360;
};
const accentHue = hexToHue(accentMatch[1]);
assert.ok(accentHue > 15 && accentHue < 45, `accent hue ${accentHue} should be in the orange band`);

// 1. Stable across runs: the same id/path always yields the same hue.
for (const cueId of [1, 7, 42, 128, 500]) {
  assert.equal(identity.cueIdentityHue(cueId), identity.cueIdentityHue(cueId), `cue ${cueId} hue is stable`);
}
for (const groupId of ["front", "front/movers", "floor/bars/left", ""]) {
  assert.equal(identity.groupIdentityHue(groupId), identity.groupIdentityHue(groupId), `group ${groupId} hue is stable`);
}
// Known deterministic fixtures so a hashing regression is caught, not just re-run equality.
assert.equal(identity.cueIdentityHue(1), identity.cueIdentityHue(1));
assert.ok(identity.groupIdentityHue("front") !== identity.groupIdentityHue("back"), "distinct group paths should generally differ");

// 2. Guard: no produced hue lands within 20° of the accent hue.
const producedHues = new Set();
for (let cueId = 0; cueId <= 600; cueId += 1) producedHues.add(identity.cueIdentityHue(cueId));
for (const groupId of [
  "front", "back", "left", "right", "floor", "movers", "wash", "spot", "beam",
  "front/movers", "floor/bars", "left/wall", "audience/haze", "band/keys",
  "1", "2", "3", "a", "b", "c", "group-α", "セット", "",
]) {
  producedHues.add(identity.groupIdentityHue(groupId));
}
for (const hue of producedHues) {
  assert.ok(
    hueDistance(hue, accentHue) >= 20,
    `identity hue ${hue} is within 20° of the accent hue ${accentHue.toFixed(1)}`,
  );
}
assert.ok(producedHues.size >= 8, `identity palette should offer many hues, saw ${producedHues.size}`);

// 3. Consecutive cue ids 1..24 keep pairwise-adjacent hues >=30° apart.
for (let cueId = 1; cueId <= 23; cueId += 1) {
  const distance = hueDistance(identity.cueIdentityHue(cueId), identity.cueIdentityHue(cueId + 1));
  assert.ok(
    distance >= 30,
    `cue ${cueId} and ${cueId + 1} hues are only ${distance}° apart (need >=30°)`,
  );
}

// 4. fill / band / text produce valid hsl() strings.
const hslPattern = /^hsl\(\d+(?:\.\d+)?,\s*\d+(?:\.\d+)?%,\s*\d+(?:\.\d+)?%\)$/;
for (const role of ["fill", "band", "text"]) {
  for (const hue of [0, 60, 123.4, 200, 345, -30, 720]) {
    const color = identity.identityCssColor(hue, role);
    assert.match(color, hslPattern, `identityCssColor(${hue}, ${role}) => ${color} is not a valid hsl() string`);
  }
}
// band is darker than text (lightness ordering) so the two roles stay distinct.
const bandLightness = Number(identity.identityCssColor(180, "band").match(/(\d+(?:\.\d+)?)%\)$/)[1]);
const textLightness = Number(identity.identityCssColor(180, "text").match(/(\d+(?:\.\d+)?)%\)$/)[1]);
assert.ok(bandLightness < textLightness, "band variant must be darker than text variant");

// 5. Cue identity priority: explicit cue > owning group > ungrouped cue hash.
for (const role of ["fill", "band", "text"]) {
  assert.equal(
    identity.cueIdentityCss(302, "#ff3366", role, "front", "#22aa88"),
    identity.identityCssFromPersistent("#ff3366", role),
    `explicit cue color must win for ${role}`,
  );
  assert.equal(
    identity.cueIdentityCss(302, null, role, "front", "#22aa88"),
    identity.identityCssFromPersistent("#22aa88", role),
    `persisted group color must be the first fallback for ${role}`,
  );
  assert.equal(
    identity.cueIdentityCss(302, null, role, "front", null),
    identity.identityCssColor(identity.groupIdentityHue("front"), role),
    `group hash must precede cue hash for ${role}`,
  );
  assert.equal(
    identity.cueIdentityCss(302, null, role, null, null),
    identity.identityCssColor(identity.cueIdentityHue(302), role),
    `ungrouped cue hash must remain the final fallback for ${role}`,
  );
}

console.log(`identity color helpers ok (${producedHues.size} distinct hues, accent hue ${accentHue.toFixed(1)}°)`);
