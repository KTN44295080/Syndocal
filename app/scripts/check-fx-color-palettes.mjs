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
    DASLIGHT_FX_PALETTE_MAX_STOPS,
    DASLIGHT_FX_PALETTE_MIN_STOPS,
    fxPaletteStopsToColorMappingFrame,
    normalizeFxPaletteStops,
  } = await server.ssrLoadModule("/src/fxColorPalettes.ts");

  assert.equal(DASLIGHT_FX_PALETTE_MIN_STOPS, 1);
  assert.equal(DASLIGHT_FX_PALETTE_MAX_STOPS, 255);
  assert.equal(builtinFxColorPalettes.length, 32, "the built-in library must keep 32 palettes");
  assert.equal(new Set(builtinFxColorPalettes.map((entry) => entry.id)).size, 32);
  assert.equal(new Set(builtinFxColorPalettes.map((entry) => entry.label)).size, 32);
  for (const entry of builtinFxColorPalettes) {
    assert.ok(entry.stops.length >= 1 && entry.stops.length <= 255, `${entry.label} stop count`);
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
  const oneStop = customFxColorPalettes([{
    id: 20,
    label: "Single Color",
    kind: "Color",
    values: [],
    color_stops: [{ position: 0, color: { red: 1, green: 2, blue: 3 } }],
  }]);
  assert.equal(oneStop.length, 1);
  assert.equal(oneStop[0].stops.length, 1);
  const oversized = Array.from({ length: 300 }, (_, index) => ({
    position: index / 299,
    color: { red: index, green: index, blue: index },
  }));
  assert.equal(normalizeFxPaletteStops(oversized).length, 255);

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
  assert.match(editorSource, /<option value="Sweep">Sweep<\/option>/);
  assert.match(editorSource, /<option value="ColorRainbow">Rainbow strip<\/option>/);
  assert.match(editorSource, /<option value="Grid" disabled=\{spatialKindUnavailable\("Grid"\)\}>Grid mapping<\/option>/);
  assert.match(editorSource, /<option value="Lines" disabled=\{spatialKindUnavailable\("Lines"\)\}>Lines mapping<\/option>/);
  assert.match(editorSource, /<option value="Graph" disabled=\{spatialKindUnavailable\("Graph"\)\}>Graph mapping<\/option>/);
  assert.match(editorSource, /min="0" max="20" step="1" value=\{spatialNumber\("size_x", 1\)\}/);
  assert.match(editorSource, /min="-5" max="5" step="1" value=\{spatialNumber\("speed_x", -1\)\}/);
  assert.match(editorSource, /min="0" max="1" step="0\.01" value=\{spatialNumber\("color_width"\)\}/);
  assert.match(editorSource, /checked=\{spatialBoolean\("grayscale"\)\}/);
  assert.match(
    editorSource,
    /<Show when=\{spatialKind\(\) === "Rainbow"\}>[\s\S]*?checked=\{spatialBoolean\("grayscale"\)\}[\s\S]*?<label>Rotation °/,
    "MAPPINGS Rainbow must expose its verified Grayscale field before the existing rotation controls",
  );
  assert.match(editorSource, /<option value="vertical">Vertical symmetry<\/option>/);
  assert.match(editorSource, /<option value="horizontal">Horizontal symmetry<\/option>/);
  for (const sweepContract of [
    "data-color-sweep-controls",
    "data-color-sweep-grayscale",
    "data-color-sweep-transform",
    "data-color-sweep-direction-change",
  ]) {
    assert.ok(editorSource.includes(sweepContract), `COLOR Sweep must expose ${sweepContract}`);
  }
  assert.ok(
    !editorSource.includes("data-color-sweep-evaluator"),
    "COLOR Sweep must use the unified evaluator without a route control",
  );
  assert.match(protocolSource, /Plasma \{[\s\S]*?serde\(default\)[\s\S]*?grayscale: bool,[\s\S]*?vertical_symmetry: bool/);
  assert.match(
    protocolSource,
    /(?:^|\r?\n)    Rainbow \{[\s\S]*?grayscale: bool,[\s\S]*?horizontal_symmetry: bool/,
  );
  assert.match(
    protocolSource,
    /(?:^|\r?\n)    Grid \{[\s\S]*?serde\(default, skip_serializing_if = "is_false"\)[\s\S]*?grayscale: bool,[\s\S]*?size: u16,[\s\S]*?width: u16,/,
    "COLOR MAPPINGS Grid must preserve optional Grayscale and exact integer fields",
  );
  assert.match(
    protocolSource,
    /(?:^|\r?\n)    Lines \{[\s\S]*?serde\(default, skip_serializing_if = "is_false"\)[\s\S]*?grayscale: bool,[\s\S]*?size: u16,/,
    "COLOR MAPPINGS Lines must preserve optional Grayscale and its exact integer field",
  );
  assert.match(
    protocolSource,
    /(?:^|\r?\n)    Graph \{[\s\S]*?serde\(default, skip_serializing_if = "is_false"\)[\s\S]*?grayscale: bool,[\s\S]*?height:[\s\S]*?width:[\s\S]*?pitch:[\s\S]*?frequency:[\s\S]*?amplitude:[\s\S]*?offset:/,
    "COLOR MAPPINGS Graph must preserve its optional Grayscale and exact field shape",
  );
  assert.match(protocolSource, /serde\(default, skip_serializing_if = "Vec::is_empty"\)[\s\S]*pub color_stops: Vec<ColorEffectStop>/);
  assert.match(editorSource, /DASLIGHT_FX_PALETTE_MAX_STOPS/);
  assert.match(protocolSource, /DASLIGHT_COLOR_PALETTE_MIN_STOPS: usize = 1/);
  assert.match(protocolSource, /DASLIGHT_COLOR_PALETTE_MAX_STOPS: usize = u8::MAX as usize/);
  assert.match(tauriSource, /fn validate_fx_palette_stops[\s\S]*DASLIGHT_COLOR_PALETTE_MAX_STOPS/);
  assert.match(engineSource, /fn validate_and_sanitize_palette[\s\S]*palette\.color_stops/);
  assert.match(engineSource, /fn daslight_grayscale_color[\s\S]*red \* 11 \+ green \* 16 \+ blue \* 5/);
  assert.match(engineSource, /fn daslight_symmetry_coordinate[\s\S]*1\.0 - \(coordinate \* 2\.0 - 1\.0\)\.abs\(\)/);
  assert.match(dvcSource, /Plasma Transform PARAM 3 must be None\(0\) or Vertical symmetry\(1\)/);
  assert.match(dvcSource, /Horizontal symmetry\(2\)/);
  assert.ok(
    editorSource.includes("data-color-mapping-placement-controls")
      && editorSource.includes("Mapping transform")
      && editorSource.includes("Mapping rotation °"),
    "placed shared MAPPINGS recipes must expose common Transform and Rotation controls",
  );
  assert.ok(
    editorSource.includes('["KnightRider", "Burst", "Sweep", "RandomFill", "Sparkle", "Spiral", "Butterfly", "Plasma", "Grid", "Lines", "Graph"]'),
    "placed Grid, Lines, and Graph must reuse the shared MAPPINGS Transform and Rotation controls",
  );
  assert.ok(
    editorSource.includes("const palettePolicyForSpatialKind")
      && editorSource.includes("minStops: DASLIGHT_FX_PALETTE_MIN_STOPS")
      && editorSource.includes("maxStops: DASLIGHT_FX_PALETTE_MAX_STOPS")
      && editorSource.includes("minStops: 2,\n  maxStops: 5,")
      && editorSource.includes("minStops: 2,\n  maxStops: 10,")
      && editorSource.includes("minStops: 2,\n  maxStops: DASLIGHT_FX_PALETTE_MAX_STOPS,")
      && editorSource.includes("if (spatialKindUnavailable(kind)) return;")
      && editorSource.includes("orderedStops().length <= currentPalettePolicy().minStops")
      && editorSource.includes("orderedStops().length >= currentPalettePolicy().maxStops"),
    "one recipe-aware helper must preserve generic 1..255, Grid 2..5, Graph 2..10, and Lines 2..255 across selection and editing",
  );
  assert.match(
    protocolSource,
    /pub struct ColorEffectSpatialPlacement \{[\s\S]*?pub vertical_symmetry: bool,[\s\S]*?pub horizontal_symmetry: bool,[\s\S]*?pub raster_rotation_degrees: f32/,
    "placed shared rasters must persist the common MAPPINGS base fields",
  );
  assert.match(
    editorSource,
    /selectSpatialKind[\s\S]*?placement: structuredClone\(props\.spatialPattern\.placement\)/,
    "changing a placed MAPPINGS generator must preserve its imported Rectangle and target coordinates",
  );

  console.log(
    `pass FX color palettes builtIn=${builtinFxColorPalettes.length} ` +
      `custom=${custom.length} minStops=1 maxStops=255 colorMappingPixels=${frame.pixels.length} ` +
      "allSceneFx=true plasmaRainbowEditable=true grayscaleTransform=exact persistence=backward-compatible",
  );
} finally {
  await server.close();
}
