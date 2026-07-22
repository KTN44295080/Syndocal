// Pure GDTF channel-function helpers extracted from App.tsx.
// Depend only on the AttributeControl type and numeric helpers; no SolidJS/state deps.
import type { GoboSlotPattern } from "./components/WheelSlotPanel";
import { clampDmxValue, formatShortDmxPercent, normalizeHexColor } from "./numericHelpers";
import type { AttributeControl } from "./types";

type ChannelFunction = NonNullable<AttributeControl["functions"]>[number];

export const normalizedFunctionText = (control: AttributeControl, fn: ChannelFunction) =>
  `${fn.name} ${fn.attribute} ${fn.wheel_slot ?? ""} ${control.channel_name}`.toLowerCase().replace(/[^a-z0-9]+/g, " ");

export const channelFunctionValue = (fn: ChannelFunction) =>
  clampDmxValue((fn.dmx_from + fn.dmx_to) / 2);

export const sortedChannelFunctions = (control: AttributeControl) =>
  [...(control.functions ?? [])]
    .filter((fn) => Number.isFinite(fn.dmx_from) && Number.isFinite(fn.dmx_to))
    .sort((first, second) => first.dmx_from - second.dmx_from);

export const pickFunctionValue = (
  control: AttributeControl,
  predicate: (text: string, fn: ChannelFunction) => boolean,
) => {
  const fn = sortedChannelFunctions(control).find((candidate) =>
    predicate(normalizedFunctionText(control, candidate), candidate),
  );
  return fn ? channelFunctionValue(fn) : null;
};

export const indexedFunctionValue = (
  control: AttributeControl,
  predicate: (text: string, fn: ChannelFunction) => boolean,
  index: number,
) => {
  const matches = sortedChannelFunctions(control).filter((candidate) =>
    predicate(normalizedFunctionText(control, candidate), candidate),
  );
  return matches[index] ? channelFunctionValue(matches[index]) : null;
};

export const rankedFunctionValue = (
  control: AttributeControl,
  rank: "first" | "middle" | "last",
  predicate: (text: string, fn: ChannelFunction) => boolean = () => true,
) => {
  const matches = sortedChannelFunctions(control).filter((candidate) =>
    predicate(normalizedFunctionText(control, candidate), candidate),
  );
  if (matches.length === 0) {
    return null;
  }
  const index = rank === "first" ? 0 : rank === "last" ? matches.length - 1 : Math.floor(matches.length / 2);
  return channelFunctionValue(matches[index]);
};

export const channelFunctionLabel = (fn: ChannelFunction) =>
  (fn.name || fn.attribute || "Function").trim();

export const channelFunctionRangeLabel = (fn: ChannelFunction) =>
  `${formatShortDmxPercent(fn.dmx_from)}-${formatShortDmxPercent(fn.dmx_to)}`;

export const channelFunctionDetail = (fn: ChannelFunction) => {
  if (fn.emitter) {
    if (fn.emitter.color) {
      const color = fn.emitter.color;
      return `Emitter ${fn.emitter.name} xyY ${color.x.toFixed(4)},${color.y.toFixed(4)},${color.luminance.toFixed(4)}`;
    }
    if (fn.emitter.dominant_wavelength_nm) {
      return `Emitter ${fn.emitter.name} ${fn.emitter.dominant_wavelength_nm.toFixed(1)} nm`;
    }
    return `Emitter ${fn.emitter.name} uncalibrated`;
  }
  if (fn.wheel_slot_name) {
    const color = normalizeHexColor(fn.wheel_slot_color);
    return color ? `${fn.wheel_slot_name} ${color}` : fn.wheel_slot_name;
  }
  if (fn.wheel_slot) {
    return fn.wheel_slot;
  }
  if (fn.physical_from !== null && fn.physical_from !== undefined && fn.physical_to !== null && fn.physical_to !== undefined) {
    return `Phys ${Number(fn.physical_from).toFixed(2)}-${Number(fn.physical_to).toFixed(2)}`;
  }
  return fn.attribute;
};

export const isColorWheelFunction = (control: AttributeControl, fn: ChannelFunction) => {
  if (normalizeHexColor(fn.wheel_slot_color)) {
    return true;
  }
  const text = normalizedFunctionText(control, fn);
  return /\b(open|clear|white|red|green|blue|cyan|magenta|yellow|amber|orange|uv|cto|ctb|color|colour|slot)\b/.test(text);
};

export const isGoboWheelFunction = (control: AttributeControl, fn: ChannelFunction) => {
  const text = normalizedFunctionText(control, fn);
  return Boolean(fn.wheel_slot) || /\b(open|clear|empty|gobo|slot|pattern|breakup|dot|dots|bars|stripe|ring|beam|rotate|rotation|spin|shake)\b/.test(text);
};

export const goboPatternForFunction = (control: AttributeControl, fn: ChannelFunction): GoboSlotPattern => {
  const text = normalizedFunctionText(control, fn);
  if (/\b(open|clear|empty|none|white)\b/.test(text)) {
    return "open";
  }
  if (/\b(spin|rotate|rotation|shake)\b/.test(text)) {
    return "spin";
  }
  if (/\b(ring|circle|donut)\b/.test(text)) {
    return "ring";
  }
  if (/\b(dot|dots|spot)\b/.test(text)) {
    return "dots";
  }
  if (/\b(bar|bars|stripe|stripes|line|lines)\b/.test(text)) {
    return "bars";
  }
  return "breakup";
};
