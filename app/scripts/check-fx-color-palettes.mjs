import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { createServer } from "vite";

const scriptsDir = dirname(fileURLToPath(import.meta.url));
const appRoot = resolve(scriptsDir, "..");
const workspaceRoot = resolve(appRoot, "..");
const server = await createServer({
  root: appRoot,
  configFile: false,
  logLevel: "error",
  appType: "custom",
  optimizeDeps: { noDiscovery: true },
  server: { middlewareMode: true },
});

try {
  const {
    builtinFxColorPalettes,
    customFxColorPalettes,
    fxPaletteStopsToColorMappingFrame,
  } = await server.ssrLoadModule("/src/fxColorPalettes.ts");

  assert.equal(builtinFxColorPalettes.length, 32, "the built-in library must keep 32 palettes");
  assert.equal(new Set(builtinFxColorPalettes.map((entry) => entry.id)).size, 32);
  assert.equal(new Set(builtinFxColorPalettes.map((entry) => entry.label)).size, 32);
  for (const entry of builtinFxColorPalettes) {
    assert.ok(entry.stops.length >= 2 && entry.stops.length <= 16, `${entry.label} stop count`);
    assert.deepEqual([...entry.stops].sort((a, b) => a.position - b.position), entry.stops);
    assert.equal(new Set(entry.stops.map((stop) => stop.position)).size, entry.stops.length);
    for (const stop of entry.stops) {
      assert.ok(stop.position >= 0 && stop.position <= 1);
      for (const channel of Object.values(stop.color)) {
        assert.ok(Number.isInteger(channel) && channel >= 0 && channel <= 65_535);
      }
    }
  }

  const custom = customFxColorPalettes([{
    id: 19,
    label: "Campus Custom",
    kind: "Color",
    values: [],
    color_stops: builtinFxColorPalettes[0].stops,
  }]);
  assert.equal(custom.length, 1);
  assert.equal(custom[0].id, "custom:19");
  assert.equal(custom[0].customPaletteId, 19);
  const frame = fxPaletteStopsToColorMappingFrame(custom[0].stops);
  assert.equal(frame.pixels.length, custom[0].stops.length);
  assert.ok(frame.pixels.every(Number.isSafeInteger));

  const [appSource, paneSource, editorSource, protocolSource, tauriSource, engineSource, dvcSource] = await Promise.all([
    readFile(resolve(appRoot, "src/App.tsx"), "utf8"),
    readFile(resolve(appRoot, "src/components/SceneSettingsPane.tsx"), "utf8"),
    readFile(resolve(appRoot, "src/components/ColorEffectEditorPanel.tsx"), "utf8"),
    readFile(resolve(workspaceRoot, "crates/protocol/src/lib.rs"), "utf8"),
    readFile(resolve(appRoot, "src-tauri/src/main.rs"), "utf8"),
    readFile(resolve(workspaceRoot, "crates/engine/src/lib.rs"), "utf8"),
    readFile(resolve(appRoot, "src-tauri/src/dvc_import.rs"), "utf8"),
  ]);
  assert.match(paneSource, /<FxColorPaletteLibraryPanel \{\.\.\.props\.editor\.palette\} \/>/);
  assert.match(appSource, /effectType: effectType\(\),\s*palettes: snapshot\(\)\.palettes,/);
  assert.match(appSource, /Added \$\{palette\.label\} as a synchronized Colour layer beside/);
  assert.match(appSource, /fxPaletteStopsToColorMappingFrame\(palette\.stops\)/);
  assert.match(editorSource, /<option value="Plasma">Plasma<\/option>/);
  assert.match(editorSource, /<option value="ColorRainbow">Rainbow strip<\/option>/);
  assert.match(editorSource, /min="0" max="20" step="1" value=\{spatialNumber\("size_x", 1\)\}/);
  assert.match(editorSource, /min="-5" max="5" step="1" value=\{spatialNumber\("speed_x", -1\)\}/);
  assert.match(editorSource, /min="0" max="1" step="0\.01" value=\{spatialNumber\("color_width"\)\}/);
  assert.match(editorSource, /checked=\{spatialBoolean\("grayscale"\)\}/);
  assert.match(editorSource, /<option value="vertical">Vertical symmetry<\/option>/);
  assert.match(editorSource, /<option value="horizontal">Horizontal symmetry<\/option>/);
  assert.match(protocolSource, /Plasma \{[\s\S]*?serde\(default\)[\s\S]*?grayscale: bool,[\s\S]*?vertical_symmetry: bool/);
  assert.match(protocolSource, /Rainbow \{[\s\S]*?horizontal_symmetry: bool/);
  assert.match(protocolSource, /serde\(default, skip_serializing_if = "Vec::is_empty"\)[\s\S]*pub color_stops: Vec<ColorEffectStop>/);
  assert.match(tauriSource, /fn validate_fx_palette_stops[\s\S]*2\.\.=16/);
  assert.match(engineSource, /fn validate_and_sanitize_palette[\s\S]*palette\.color_stops/);
  assert.match(engineSource, /fn daslight_grayscale_color[\s\S]*red \* 11 \+ green \* 16 \+ blue \* 5/);
  assert.match(engineSource, /fn daslight_symmetry_coordinate[\s\S]*1\.0 - \(coordinate \* 2\.0 - 1\.0\)\.abs\(\)/);
  assert.match(dvcSource, /Plasma Transform PARAM 3 must be None\(0\) or Vertical symmetry\(1\)/);
  assert.match(dvcSource, /Horizontal symmetry\(2\)/);

  console.log(
    `pass FX color palettes builtIn=${builtinFxColorPalettes.length} ` +
      `custom=${custom.length} maxStops=16 colorMappingPixels=${frame.pixels.length} ` +
      "allSceneFx=true plasmaRainbowEditable=true grayscaleTransform=exact persistence=backward-compatible",
  );
} finally {
  await server.close();
}
