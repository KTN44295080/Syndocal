// Per-category "quick look" presets and value resolution extracted from App.tsx.
// The `ControlCategory` enum, `controlCategories` table, and `controlCategoryForAttribute`
// classifier live in `uiModes.ts` (shared with components) — import them from there.
import { indexedFunctionValue, pickFunctionValue, rankedFunctionValue } from "./channelFunctionHelpers";
import type { AttributeControl } from "./types";
import type { ControlCategory } from "./uiModes";

export interface CategoryQuickLook {
  id: string;
  label: string;
  description: string;
}

export const quickLooksForCategory = (category: ControlCategory): CategoryQuickLook[] => {
  switch (category) {
    case "gobo":
      return [
        { id: "open", label: "Open", description: "Wheel open / index zero" },
        { id: "slot1", label: "Slot 1", description: "First approximate gobo slot" },
        { id: "slot2", label: "Slot 2", description: "Second approximate gobo slot" },
        { id: "spin", label: "Spin", description: "High-range rotate/spin area" },
      ];
    case "beam":
      return [
        { id: "open", label: "Open", description: "Open shutter, no strobe/prism/frost" },
        { id: "tight", label: "Tight", description: "Narrow iris/zoom style look" },
        { id: "wide", label: "Wide", description: "Wide beam/iris style look" },
        { id: "soft", label: "Soft", description: "Frost/soft beam emphasis" },
      ];
    case "focus":
      return [
        { id: "near", label: "Near", description: "Low focus range" },
        { id: "mid", label: "Mid", description: "Middle focus range" },
        { id: "far", label: "Far", description: "High focus range" },
        { id: "default", label: "Default", description: "Fixture profile default" },
      ];
    default:
      return [];
  }
};

export const opticsRoleForControl = (control: AttributeControl, category: ControlCategory) => {
  const text = `${control.attribute} ${control.channel_name}`.toLowerCase().replace(/[^a-z0-9]+/g, " ");
  if (/\b(shutter|shutterstrobe)\b/.test(text)) {
    return "Shutter";
  }
  if (/\b(strobe)\b/.test(text)) {
    return "Strobe";
  }
  if (/\b(iris)\b/.test(text)) {
    return "Iris";
  }
  if (/\b(zoom|beam|wash|spot)\b/.test(text)) {
    return "Zoom";
  }
  if (/\b(frost|diffusion|diffuse|soft)\b/.test(text)) {
    return "Frost";
  }
  if (/\b(prism|facet)\b/.test(text)) {
    return "Prism";
  }
  if (/\b(focus|focal)\b/.test(text)) {
    return "Focus";
  }
  return category === "focus" ? "Focus" : "Beam";
};

export const quickLookValueForControl = (
  category: ControlCategory,
  lookId: string,
  control: AttributeControl,
) => {
  const normalized = control.attribute.toLowerCase().replace(/[^a-z0-9]/g, "");
  if (category === "gobo") {
    const openValue = pickFunctionValue(control, (text) => /\b(open|clear|empty|none|white)\b/.test(text));
    const goboValue = (index: number) =>
      indexedFunctionValue(
        control,
        (text, fn) =>
          (Boolean(fn.wheel_slot) || /\b(gobo|slot|pattern|breakup)\b/.test(text)) &&
          !/\b(open|clear|empty|none|white|spin|rotate|rotation|shake)\b/.test(text),
        index,
      );
    if (lookId === "slot1") {
      return goboValue(0) ?? 8192;
    }
    if (lookId === "slot2") {
      return goboValue(1) ?? 16_384;
    }
    if (lookId === "spin") {
      return pickFunctionValue(control, (text) => /\b(spin|rotate|rotation|continuous|shake)\b/.test(text)) ?? 49_152;
    }
    return openValue ?? 0;
  }
  if (category === "focus") {
    if (lookId === "near") {
      return rankedFunctionValue(control, "first") ?? 0;
    }
    if (lookId === "mid") {
      return rankedFunctionValue(control, "middle") ?? 32_768;
    }
    if (lookId === "far") {
      return rankedFunctionValue(control, "last") ?? 65_535;
    }
    return control.default_value;
  }
  if (category === "beam") {
    const isShutter = /shutter/.test(normalized);
    const isStrobe = /strobe/.test(normalized);
    const isIris = /iris/.test(normalized);
    const isZoomOrBeam = /(zoom|beam|wash|spot)/.test(normalized);
    const isFrost = /frost/.test(normalized);
    const isPrism = /prism/.test(normalized);
    const openOrOff = () =>
      pickFunctionValue(control, (text) => /\b(open|off|none|disable|disabled|clear|home)\b/.test(text));
    const narrow = () => pickFunctionValue(control, (text) => /\b(tight|narrow|small|min|minimum)\b/.test(text));
    const wide = () => pickFunctionValue(control, (text) => /\b(wide|large|max|maximum|open)\b/.test(text));
    const soft = () => pickFunctionValue(control, (text) => /\b(soft|frost|diffusion|diffuse|on|enable)\b/.test(text));
    if (lookId === "open") {
      if (isShutter || isIris) {
        return openOrOff() ?? 65_535;
      }
      if (isStrobe || isFrost || isPrism) {
        return openOrOff() ?? 0;
      }
      return control.default_value;
    }
    if (lookId === "tight") {
      if (isShutter) {
        return openOrOff() ?? 65_535;
      }
      if (isIris || isZoomOrBeam || isStrobe || isFrost || isPrism) {
        return (isIris || isZoomOrBeam ? narrow() : openOrOff()) ?? 0;
      }
      return control.default_value;
    }
    if (lookId === "wide") {
      if (isShutter || isIris || isZoomOrBeam) {
        return (isIris || isZoomOrBeam ? wide() : openOrOff()) ?? 65_535;
      }
      if (isStrobe || isFrost || isPrism) {
        return openOrOff() ?? 0;
      }
      return control.default_value;
    }
    if (lookId === "soft") {
      if (isShutter || isIris || isZoomOrBeam || isFrost) {
        return (isFrost ? soft() : wide()) ?? 65_535;
      }
      if (isStrobe || isPrism) {
        return openOrOff() ?? 0;
      }
    }
  }
  return control.default_value;
};
