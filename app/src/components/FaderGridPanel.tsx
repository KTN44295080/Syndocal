import { For, Show, createEffect, createSignal, onCleanup } from "solid-js";
import { createStore, reconcile } from "solid-js/store";
import {
  channelFunctionLabel,
  channelFunctionWheelColor,
  normalizedFunctionText,
  sortedChannelFunctions,
} from "../channelFunctionHelpers";
import { handleHorizontalWheel } from "../horizontalWheel";
import type { AttributeControl } from "../types";
import { VerticalFaderInput } from "./VerticalFaderInput";

type AttributeFaderGlyph =
  | "dimmer"
  | "shutter"
  | "strobe"
  | "pan"
  | "tilt"
  | "speed"
  | "gobo"
  | "zoom"
  | "focus"
  | "generic";

const attributeFaderGlyphPaths: Record<AttributeFaderGlyph, string> = {
  dimmer: "M8 6a2 2 0 1 0 0 4 2 2 0 0 0 0-4ZM8 2v2m0 8v2M2 8h2m8 0h2",
  shutter: "M8 3a5 5 0 1 0 0 10A5 5 0 0 0 8 3Zm-4 9 8-8",
  strobe: "M9.4 1.8 4.5 8.7h3.3l-1.2 5.5 4.9-7H8.2z",
  pan: "M2 8h12M5 5 2 8l3 3M11 5l3 3-3 3",
  tilt: "M8 2v12M5 5l3-3 3 3M5 11l3 3 3-3",
  speed: "M8 2.25a5.75 5.75 0 1 0 0 11.5 5.75 5.75 0 0 0 0-11.5ZM8 4v4l3 1.75M8 2.25v1M13.75 8h-1M8 13.75v-1M2.25 8h1",
  gobo: "M8 3a5 5 0 1 0 0 10A5 5 0 0 0 8 3Zm0 2v2m-2 3 2-1 2 1",
  zoom: "M7 3a4 4 0 1 0 0 8 4 4 0 0 0 0-8Zm3 7 3 3M5 7h4M7 5v4",
  focus: "M3 6V3h3m4 0h3v3m0 4v3h-3m-4 0H3v-3M8 6v4M6 8h4",
  generic: "m8 2 5 6-5 6-5-6 5-6Zm0 4.8a1.2 1.2 0 1 0 0 2.4 1.2 1.2 0 0 0 0-2.4Z",
};

interface FaderGridPanelProps {
  controls: AttributeControl[];
  selectedFixtureId?: number | null;
  selectedGroupId?: string | null;
  valueForControl: (control: AttributeControl) => number;
  isControlWritten: (control: AttributeControl) => boolean;
  fullWidth?: boolean;
  onPointerCaptureChange?: (captured: boolean) => void;
  onSetControlValue?: (control: AttributeControl, value: number) => void;
  onSetFixtureAttribute?: (fixtureId: number, attribute: string, value: number) => void | Promise<void>;
  onSetGroupAttribute?: (groupId: string, attribute: string, value: number) => void | Promise<void>;
}

interface StableFaderControl {
  id: string;
  control: AttributeControl;
}

const attributeControlId = (control: AttributeControl) =>
  [
    control.attribute.toLowerCase(),
    control.geometry?.toLowerCase() ?? "",
    control.offsets.join(","),
  ].join(":");

const stableFaderControls = (controls: readonly AttributeControl[]): StableFaderControl[] =>
  controls.map((control) => ({
    id: attributeControlId(control),
    control,
  }));

export function FaderGridPanel(props: FaderGridPanelProps) {
  const [stableControls, setStableControls] = createStore<StableFaderControl[]>(
    stableFaderControls(props.controls),
  );
  const capturedControlIds = new Set<string>();
  const [pointerCaptureRevision, setPointerCaptureRevision] = createSignal(0);
  let reportedPointerCapture = false;
  createEffect(() => {
    pointerCaptureRevision();
    const nextControls = props.controls;
    // Snapshot metadata can refresh around the active control, but replacing
    // its keyed row while the native range owns capture would end the drag.
    // Keep the structure frozen until capture ends; valueForControl remains
    // reactive and continues to update the existing input node.
    if (capturedControlIds.size > 0) return;
    setStableControls(reconcile(stableFaderControls(nextControls), { key: "id" }));
  });
  const setControlPointerCapture = (controlId: string, captured: boolean) => {
    if (captured) {
      capturedControlIds.add(controlId);
    } else {
      capturedControlIds.delete(controlId);
    }
    const anyCaptured = capturedControlIds.size > 0;
    if (reportedPointerCapture !== anyCaptured) {
      reportedPointerCapture = anyCaptured;
      props.onPointerCaptureChange?.(anyCaptured);
    }
    setPointerCaptureRevision((revision) => revision + 1);
  };
  onCleanup(() => {
    if (reportedPointerCapture) props.onPointerCaptureChange?.(false);
    capturedControlIds.clear();
  });
  const hasSelectedFixture = () => props.selectedFixtureId !== null && props.selectedFixtureId !== undefined;
  const clampDmxValue = (value: number) =>
    Math.min(65_535, Math.max(0, Math.round(Number.isFinite(value) ? value : 0)));

  const setControlValue = (control: AttributeControl, value: number) => {
    const nextValue = clampDmxValue(value);
    if (props.onSetControlValue) {
      props.onSetControlValue(control, nextValue);
      return;
    }
    const fixtureId = props.selectedFixtureId;
    if (fixtureId === null || fixtureId === undefined) {
      return;
    }

    const groupId = props.selectedGroupId;
    if (groupId && props.onSetGroupAttribute) {
      void props.onSetGroupAttribute(groupId, control.attribute, nextValue);
    } else if (props.onSetFixtureAttribute) {
      void props.onSetFixtureAttribute(fixtureId, control.attribute, nextValue);
    }
  };
  const activeFunction = (control: AttributeControl, value: number) =>
    sortedChannelFunctions(control).find((fn) => {
      const from = clampDmxValue(fn.dmx_from);
      const to = clampDmxValue(fn.dmx_to);
      return value >= Math.min(from, to) && value <= Math.max(from, to);
    });
  const displayLabel = (control: AttributeControl, value: number) => {
    const fn = activeFunction(control, value);
    return fn ? channelFunctionLabel(fn) : control.attribute || "No Func";
  };
  const normalizedAttributeText = (control: AttributeControl, value: number) =>
    `${control.attribute} ${control.channel_name} ${displayLabel(control, value)}`
      .replace(/([a-z])([A-Z])/g, "$1 $2")
      .toLowerCase()
      .replace(/[^\p{L}\p{N}]+/gu, " ");
  const classificationAttributeText = (control: AttributeControl, value: number) =>
    normalizedAttributeText(control, value).replace(/\b(\p{L}+)\d+\b/gu, "$1");
  const swatchColor = (control: AttributeControl, value: number) => {
    const fn = activeFunction(control, value);
    const wheelColor = fn ? channelFunctionWheelColor(control, fn) : null;
    if (wheelColor) {
      return wheelColor;
    }
    const normalized = (fn
      ? normalizedFunctionText(control, fn)
      : classificationAttributeText(control, value)
    )
      .replace(/[^\p{L}\p{N}]+/gu, " ")
      .replace(/\b(\p{L}+)\d+\b/gu, "$1");
    if (/\b(red)\b/.test(normalized)) return "#e4564f";
    if (/\b(green)\b/.test(normalized)) return "#57b85b";
    if (/\b(blue)\b/.test(normalized)) return "#5686d8";
    if (/\b(amber|orange)\b/.test(normalized)) return "#d79243";
    if (/\b(cyan)\b/.test(normalized)) return "#4abdc3";
    if (/\b(magenta)\b/.test(normalized)) return "#c75da8";
    if (/\b(yellow|lime)\b/.test(normalized)) return "#d5ca55";
    if (/\b(uv|violet)\b/.test(normalized)) return "#8d6bc4";
    if (/\b(white|open|clear)\b/.test(normalized)) return "#d9dde0";
    if (/\b(colou?r|rgb|hue|saturation)\b/.test(normalized)) return "#a9b2bc";
    return null;
  };
  const attributeGlyph = (control: AttributeControl, value: number): AttributeFaderGlyph => {
    const normalized = classificationAttributeText(control, value);
    const compactIdentity = `${control.attribute} ${control.channel_name}`
      .replace(/([a-z])([A-Z])/g, "$1 $2")
      .toLowerCase()
      .replace(/\b([a-z]+)\d+\b/g, "$1")
      .replace(/[^a-z0-9]/g, "");
    if (/\b(dimmer|intensity)\b/.test(normalized)) return "dimmer";
    if (/\b(shutter|iris)\b/.test(normalized)) return "shutter";
    if (/\b(strobe|flash)\b/.test(normalized)) return "strobe";
    if (/(pantilt|pt|movement|move|position)speed/.test(compactIdentity)) return "speed";
    if (/\bpan\b/.test(normalized)) return "pan";
    if (/\btilt\b/.test(normalized)) return "tilt";
    if (/\b(gobo|pattern)\b/.test(normalized)) return "gobo";
    if (/\bzoom\b/.test(normalized)) return "zoom";
    if (/\bfocus\b/.test(normalized)) return "focus";
    return "generic";
  };
  const attributeGlyphSvg = (glyph: AttributeFaderGlyph) => (
    <svg viewBox="0 0 16 16" aria-hidden="true">
      <path d={attributeFaderGlyphPaths[glyph]} />
    </svg>
  );
  const displayValue = (control: AttributeControl, value: number) =>
    control.resolution === "SixteenBit" ? `${clampDmxValue(value)}` : `${Math.round(clampDmxValue(value) / 257)}`;
  const fineStep = (control: AttributeControl) => (control.resolution === "SixteenBit" ? 1 : 257);

  return (
    <div
      classList={{ faderGrid: true, faderChannelBank: props.fullWidth }}
      role="group"
      aria-label="Attribute faders"
      data-fader-view-channel-bank={props.fullWidth ? "true" : undefined}
      data-wheel-scroll-surface={props.fullWidth ? "fader-channel-band" : undefined}
      onWheel={props.fullWidth ? handleHorizontalWheel : undefined}
    >
      <For each={stableControls}>
        {(entry) => {
          const control = () => entry.control;
          const value = () => clampDmxValue(props.valueForControl(control()));
          const written = () => props.isControlWritten(control());
          const swatch = () => swatchColor(control(), value());
          const glyph = () => attributeGlyph(control(), value());
          return (
            <div
              classList={{ fader: true, attributeFaderColumn: true, written: written(), off: !written() }}
              data-fader-control-id={entry.id}
            >
              <div class="attributeFaderWriteRow">
                <span
                  class="attributeFaderWriteIndicator"
                  role="img"
                  title={written() ? "Written value" : "Default value, off"}
                  aria-label={written() ? "Written value" : "Default value, off"}
                />
                <small class="attributeFaderChannel">CH {control().offsets.join("/") || "-"}</small>
              </div>
              <span
                class="attributeFaderIcon"
                data-attribute-glyph={swatch() ? "color" : glyph()}
                title={`${control().attribute} / ${displayLabel(control(), value())}`}
                role="img"
                aria-label={control().attribute}
              >
                <Show when={swatch()} fallback={attributeGlyphSvg(glyph())}>
                  {(color) => <i style={{ "background-color": color() }} />}
                </Show>
              </span>
              <output class="attributeFaderValue">
                {written() ? displayValue(control(), value()) : "OFF"}
              </output>
              <VerticalFaderInput
                chromeClass="attributeVerticalFaderChrome"
                inputClass="verticalFaderInput"
                min="0"
                max="65535"
                value={value()}
                aria-label={control().attribute}
                aria-valuetext={written() ? `${displayValue(control(), value())} DMX` : "OFF"}
                onPointerCaptureChange={(captured) => setControlPointerCapture(entry.id, captured)}
                onInput={(event) => setControlValue(control(), Number(event.currentTarget.value))}
              />
              <div class="attributeFaderFine" role="group" aria-label="Fine adjustment">
                <button
                  type="button"
                  aria-label="Decrease value"
                  title="Decrease value"
                  onClick={() => setControlValue(control(), value() - fineStep(control()))}
                >
                  −
                </button>
                <button
                  type="button"
                  aria-label="Increase value"
                  title="Increase value"
                  onClick={() => setControlValue(control(), value() + fineStep(control()))}
                >
                  +
                </button>
              </div>
            </div>
          );
        }}
      </For>
      <Show when={!hasSelectedFixture()}>
        <p class="empty">Select or patch a fixture.</p>
      </Show>
      <Show when={hasSelectedFixture() && props.controls.length === 0}>
        <p class="empty">No controls in this category.</p>
      </Show>
    </div>
  );
}
