import assert from "node:assert/strict";
import { readFile, readdir } from "node:fs/promises";

const sourceRoot = new URL("../src/", import.meta.url);

const sourceFiles = async (directory, prefix = "") => {
  const entries = await readdir(directory, { withFileTypes: true });
  const paths = [];
  for (const entry of entries) {
    const relativePath = `${prefix}${entry.name}`;
    if (entry.isDirectory()) {
      paths.push(...(await sourceFiles(new URL(`${entry.name}/`, directory), `${relativePath}/`)));
    } else if (/\.(?:ts|tsx)$/.test(entry.name)) {
      paths.push(relativePath);
    }
  }
  return paths;
};

const files = await sourceFiles(sourceRoot);
const source = (
  await Promise.all(files.map(async (path) => `\n// ${path}\n${await readFile(new URL(path, sourceRoot), "utf8")}`))
).join("");

const forbiddenUiTerms = [
  /projector mapping preset/i,
  /projector surface\(s\)/i,
  /projector\(s\)/i,
  />Projectors</,
  /No projectors\./i,
  /fixture and projector mapping/i,
  /projector warp surface/i,
  /Toggle projector surfaces/i,
  /Select fixture or projector/i,
  /\bout\(s\)/,
  /Composition Output/,
  />Layer (?:Enable|Solo|Fade)</,
  />Output (?:Enable|Opacity|Fade|Mapping|Blackout)/,
  />Add Cue</,
  />\+ Cue</,
  /Projector Map/,
];

for (const pattern of forbiddenUiTerms) {
  assert.ok(!pattern.test(source), `legacy UI terminology found: ${pattern}`);
}

for (const term of ["Video Layer", "Video Output", "Projection Surface", "Cue Point", "DMX Output"]) {
  assert.ok(source.includes(term), `canonical UI term is missing: ${term}`);
}

console.log("ui terminology ok");
