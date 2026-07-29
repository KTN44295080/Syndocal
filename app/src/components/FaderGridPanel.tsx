import { For, Show } from "solid-js";
import {
  channelFunctionLabel,
  normalizedFunctionText,
  sortedChannelFunctions,
} from "../channelFunctionHelpers";
import type { AttributeControl } from "../types";
import { VerticalFaderInput } from "./VerticalFaderInput";

interface FaderGridPanelProps {
  controls: AttributeControl[];
  selectedFixtureId?: number | null;
  selectedGroupId?: string | null;
  valueForControl: (control: AttributeControl) => number;
  isControlWritten: (control: AttributeControl) => boolean;
  fullWidth?: boolean;
  onSetControlValue?: (control: AttributeControl, value: number) => void;
  onSetFixtureAttribute?: (fixtureId: number, attribute: string, value: number) => void | Promise<void>;
  onSetGroupAttribute?: (groupId: string, attribute: string, value: number) => void | Promise<void>;
}

export function FaderGridPanel(props: FaderGridPanelProps) {
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
  const shortLabel = (control: AttributeControl, value: number) => {
    const label = displayLabel(control, value).replace(/[^\p{L}\p{N}]+/gu, " ").trim();
    return (label.split(/\s+/)[0] || "No Func").slice(0, 7);
  };
  const swatchColor = (control: AttributeControl, value: number) => {
    const fn = activeFunction(control, value);
    const wheelColor = fn?.wheel_slot_color?.trim();
    if (wheelColor && /^#[0-9a-f]{6}$/i.test(wheelColor)) {
      return wheelColor;
    }
    const normalized = (fn
      ? normalizedFunctionText(control, fn)
      : `${control.attribute} ${control.channel_name}`.toLowerCase()
    ).replace(/[^\p{L}\p{N}]+/gu, " ");
    if (/\b(red)\b/.test(normalized)) return "#e4564f";
    if (/\b(green)\b/.test(normalized)) return "#57b85b";
    if (/\b(blue)\b/.test(normalized)) return "#5686d8";
    if (/\b(amber|orange)\b/.test(normalized)) return "#d79243";
    if (/\b(uv|violet)\b/.test(normalized)) return "#8d6bc4";
    if (/\b(white|open|clear)\b/.test(normalized)) return "#d9dde0";
    return null;
  };
  const displayValue = (control: AttributeControl, value: number) =>
    control.resolution === "SixteenBit" ? `${clampDmxValue(value)}` : `${Math.round(clampDmxValue(value) / 257)}`;
  const fineStep = (control: AttributeControl) => (control.resolution === "SixteenBit" ? 1 : 257);

  return (
    <div
      classList={{ faderGrid: true, faderChannelBank: props.fullWidth }}
      role="group"
      aria-label="Attribute faders"
      data-fader-view-channel-bank={props.fullWidth ? "true" : undefined}
    >
      <For each={props.controls}>
        {(control) => {
          const value = () => clampDmxValue(props.valueForControl(control));
          const written = () => props.isControlWritten(control);
          const swatch = () => swatchColor(control, value());
          return (
            <div classList={{ fader: true, attributeFaderColumn: true, written: written(), off: !written() }}>
              <div class="attributeFaderWriteRow">
                <span
                  class="attributeFaderWriteIndicator"
                  role="img"
                  title={written() ? "Written value" : "Default value, off"}
                  aria-label={written() ? "Written value" : "Default value, off"}
                />
                <small class="attributeFaderChannel">CH {control.offsets.join("/") || "-"}</small>
              </div>
              <span class="attributeFaderIcon" title={displayLabel(control, value())}>
                <Show when={swatch()} fallback={<b>{shortLabel(control, value())}</b>}>
                  {(color) => <i style={{ "background-color": color() }} />}
                </Show>
              </span>
              <strong class="attributeFaderLabel" title={`${control.attribute} / ${control.channel_name}`}>
                {control.attribute}
              </strong>
              <output class="attributeFaderValue">
                {written() ? displayValue(control, value()) : "OFF"}
              </output>
              <VerticalFaderInput
                chromeClass="attributeVerticalFaderChrome"
                inputClass="verticalFaderInput"
                min="0"
                max="65535"
                value={value()}
                aria-label={control.attribute}
                aria-valuetext={written() ? `${displayValue(control, value())} DMX` : "OFF"}
                onInput={(event) => setControlValue(control, Number(event.currentTarget.value))}
              />
              <div class="attributeFaderFine" role="group" aria-label="Fine adjustment">
                <button
                  type="button"
                  aria-label="Decrease value"
                  title="Decrease value"
                  onClick={() => setControlValue(control, value() - fineStep(control))}
                >
                  −
                </button>
                <button
                  type="button"
                  aria-label="Increase value"
                  title="Increase value"
                  onClick={() => setControlValue(control, value() + fineStep(control))}
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
