import type { ColorEffectColor, ColorEffectStop, ReferencePaletteSummary } from "./types";

export interface FxColorPaletteDefinition {
  id: string;
  label: string;
  stops: ColorEffectStop[];
  customPaletteId?: number;
}

const clamp = (value: number, minimum: number, maximum: number) =>
  Math.min(maximum, Math.max(minimum, Number.isFinite(value) ? value : minimum));

const colorFromHex = (hex: string): ColorEffectColor => {
  const normalized = hex.replace(/^#/, "");
  return {
    red: Number.parseInt(normalized.slice(0, 2), 16) * 257,
    green: Number.parseInt(normalized.slice(2, 4), 16) * 257,
    blue: Number.parseInt(normalized.slice(4, 6), 16) * 257,
  };
};

export const fxPaletteColorToHex = (color: ColorEffectColor) => {
  const byte = (value: number) => Math.round(clamp(value, 0, 65_535) / 257)
    .toString(16)
    .padStart(2, "0");
  return `#${byte(color.red)}${byte(color.green)}${byte(color.blue)}`;
};

const palette = (id: string, label: string, hexColors: string[]): FxColorPaletteDefinition => ({
  id: `builtin:${id}`,
  label,
  stops: hexColors.map((hex, index) => ({
    position: hexColors.length === 1 ? 0 : index / (hexColors.length - 1),
    color: colorFromHex(hex),
  })),
});

// Thirty-two production-ready looks plus project-saved custom palettes. The
// observed Daslight show palettes contained five or ten slots; Syndocal keeps
// up to sixteen editable stops per palette and does not cap custom creation
// below the project's shared 128-palette safety boundary.
export const builtinFxColorPalettes: FxColorPaletteDefinition[] = [
  palette("rainbow", "Full Rainbow", ["#ff0000", "#ff7a00", "#ffff00", "#00ff00", "#00ffff", "#0066ff", "#7a00ff", "#ff00cc"]),
  palette("primary", "RGB Primary", ["#ff0000", "#00ff00", "#0000ff"]),
  palette("secondary", "CMY Secondary", ["#00ffff", "#ff00ff", "#ffff00"]),
  palette("warm-white", "Warm White", ["#ff6a18", "#ffb35a", "#fff0c7", "#ffffff"]),
  palette("cold-white", "Cold White", ["#ffffff", "#d7efff", "#8dc9ff", "#4f8cff"]),
  palette("fire", "Fire", ["#2b0000", "#8f0800", "#ff3100", "#ff8a00", "#fff0a0"]),
  palette("embers", "Embers", ["#160000", "#4d0300", "#b51c00", "#ff5a00"]),
  palette("sunset", "Sunset", ["#ffb000", "#ff5a1f", "#e60054", "#6d1aa8", "#17125c"]),
  palette("sunrise", "Sunrise", ["#1b174d", "#a02b70", "#ff6745", "#ffd07a", "#fff6d6"]),
  palette("ocean", "Ocean", ["#00183d", "#0055a8", "#00a8c8", "#3de6d1", "#d8ffff"]),
  palette("deep-sea", "Deep Sea", ["#00071c", "#001c4a", "#003b75", "#006b78"]),
  palette("aurora", "Aurora", ["#091540", "#006a8d", "#00d6a3", "#9aff71", "#9e55ff"]),
  palette("forest", "Forest", ["#071f0d", "#0d5c2a", "#36a344", "#b4d66d"]),
  palette("wine", "Wine Red", ["#190008", "#540018", "#9c1235", "#d84a67", "#ff9cb0"]),
  palette("amber", "Amber Gold", ["#3d1300", "#a84600", "#ff8a00", "#ffd15a"]),
  palette("magenta-cyan", "Magenta Cyan", ["#ff007f", "#7a00ff", "#006eff", "#00ffff"]),
  palette("ultraviolet", "Ultraviolet Club", ["#12002e", "#4f00aa", "#a000ff", "#ff00c8"]),
  palette("neon", "Neon", ["#ff005d", "#ffef00", "#00ff85", "#00c8ff", "#a600ff"]),
  palette("pastel", "Pastel", ["#ffb7c9", "#ffd6a5", "#fdffb6", "#caffbf", "#9bf6ff", "#bdb2ff"]),
  palette("candy", "Candy", ["#ff4fa3", "#ff9ed8", "#8ee8ff", "#6f8cff", "#d16fff"]),
  palette("ice", "Ice", ["#07182b", "#174f7a", "#4fb8df", "#baf4ff", "#ffffff"]),
  palette("steel", "Steel", ["#101820", "#33414f", "#71808d", "#c4d0d8", "#ffffff"]),
  palette("red-white-blue", "Red White Blue", ["#d60020", "#ffffff", "#1557ff"]),
  palette("traffic", "Traffic", ["#ff0000", "#ffb000", "#00d84a"]),
  palette("black-white", "Black White", ["#000000", "#ffffff"]),
  palette("red-black", "Red Black", ["#ff0000", "#210000", "#000000"]),
  palette("green-blue", "Green Blue", ["#00ff58", "#00b5a6", "#0055ff"]),
  palette("blue-magenta", "Blue Magenta", ["#004cff", "#6a00ff", "#ff00ad"]),
  palette("tropical", "Tropical", ["#00e5c8", "#32ff6a", "#ffe14a", "#ff6b38"]),
  palette("lavender", "Lavender", ["#28104e", "#7046aa", "#b58ae8", "#f0dbff"]),
  palette("acid", "Acid", ["#d9ff00", "#39ff14", "#00b84a", "#102600"]),
  palette("theatre-warm", "Theatre Warm", ["#ff5a1f", "#ff9a45", "#ffd19a", "#fff2d8"]),
];

export const cloneFxPaletteStops = (stops: ColorEffectStop[]) =>
  stops.map((stop) => ({
    position: stop.position,
    color: { ...stop.color },
  }));

export const normalizeFxPaletteStops = (stops: ColorEffectStop[]) =>
  cloneFxPaletteStops(stops)
    .slice(0, 16)
    .map((stop) => ({
      position: clamp(stop.position, 0, 1),
      color: {
        red: Math.round(clamp(stop.color.red, 0, 65_535)),
        green: Math.round(clamp(stop.color.green, 0, 65_535)),
        blue: Math.round(clamp(stop.color.blue, 0, 65_535)),
      },
    }))
    .sort((left, right) => left.position - right.position);

export const customFxColorPalettes = (palettes: ReferencePaletteSummary[]) =>
  palettes.flatMap<FxColorPaletteDefinition>((entry) => {
    const stops = normalizeFxPaletteStops(entry.color_stops ?? []);
    return stops.length >= 2
      ? [{ id: `custom:${entry.id}`, label: entry.label, stops, customPaletteId: entry.id }]
      : [];
  });

export const fxPaletteStopsToColorMappingFrame = (stops: ColorEffectStop[]) => ({
  pixels: normalizeFxPaletteStops(stops).map((stop) =>
    stop.color.red * 4_294_967_296 + stop.color.green * 65_536 + stop.color.blue),
});
