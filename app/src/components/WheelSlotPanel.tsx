import { For, Show } from "solid-js";
import type { JSX } from "solid-js";
import type { AttributeControl, ChannelFunctionSummary } from "../types";
import { HsvColorAdjustments, HsvColorPicker, type HsvColor } from "./HsvColorPicker";

export interface ColorWheelFunctionEntry {
  control: AttributeControl;
  fn: ChannelFunctionSummary;
  color: string;
  label: string;
  currentValue: number;
  active: boolean;
}

export type GoboSlotPattern = "open" | "bars" | "dots" | "breakup" | "ring" | "spin";

export interface GoboWheelFunctionEntry {
  control: AttributeControl;
  fn: ChannelFunctionSummary;
  label: string;
  currentValue: number;
  active: boolean;
  pattern: GoboSlotPattern;
}

interface WheelSlotPanelProps<T> {
  title: string;
  countLabel: string;
  targetLabel: string;
  entries: T[];
  itemClass: string;
  itemIcon: (entry: T) => JSX.Element;
  itemLabel: (entry: T) => string;
  itemDetail: (entry: T) => string;
  itemTitle: (entry: T) => string;
  itemActive: (entry: T) => boolean;
  emptyMessage: string;
  inactiveMessage: string;
  entryValue: (entry: T) => number;
  onSetValue: (entry: T, value: number) => void;
  onApply: (entry: T) => void;
}

function WheelSlotPanel<T>(props: WheelSlotPanelProps<T>) {
  const clampDmxValue = (value: number) => Math.min(65_535, Math.max(0, Math.round(Number.isFinite(value) ? value : 0)));
  const valueToPercent = (value: number) => Math.round((clampDmxValue(value) / 65_535) * 1000) / 10;
  const percentToValue = (value: number) => {
    const clamped = Math.min(100, Math.max(0, Number.isFinite(value) ? value : 0));
    return clampDmxValue((clamped / 100) * 65_535);
  };
  const activeIndex = () => props.entries.findIndex((entry) => props.itemActive(entry));
  const navigationIndex = () => {
    const index = activeIndex();
    return index >= 0 ? index : 0;
  };
  const activeEntry = () => {
    const index = activeIndex();
    return index >= 0 ? props.entries[index] : undefined;
  };
  const directEntry = () => activeEntry() ?? props.entries[0];
  const applyIndex = (index: number) => {
    const count = props.entries.length;
    if (count === 0) {
      return;
    }
    const wrappedIndex = ((index % count) + count) % count;
    props.onApply(props.entries[wrappedIndex]);
  };
  const handleSlotKeyDown = (event: KeyboardEvent, index: number) => {
    switch (event.key) {
      case "ArrowRight":
      case "ArrowDown":
        event.preventDefault();
        applyIndex(index + 1);
        break;
      case "ArrowLeft":
      case "ArrowUp":
        event.preventDefault();
        applyIndex(index - 1);
        break;
      case "Home":
        event.preventDefault();
        applyIndex(0);
        break;
      case "End":
        event.preventDefault();
        applyIndex(props.entries.length - 1);
        break;
    }
  };

  return (
    <div class={`visualControlPanel ${props.itemClass}Panel`}>
      <div class="visualControlHeader">
        <div>
          <strong>{props.title}</strong>
          <span>{props.countLabel}</span>
        </div>
        <span>{props.targetLabel}</span>
      </div>
      <Show when={activeEntry()}>
        {(entry) => (
          <div class="wheelSlotActiveRow">
            <div class="wheelSlotActivePreview">
              <span class="wheelSlotActiveIcon">{props.itemIcon(entry())}</span>
              <span>
                <strong>{props.itemLabel(entry())}</strong>
                <small>{props.itemDetail(entry())}</small>
              </span>
            </div>
            <div class="wheelSlotStepper">
              <button type="button" onClick={() => applyIndex(navigationIndex() - 1)}>
                Prev
              </button>
              <button type="button" onClick={() => applyIndex(0)}>
                First
              </button>
              <button type="button" onClick={() => applyIndex(navigationIndex() + 1)}>
                Next
              </button>
            </div>
          </div>
        )}
      </Show>
      <Show when={props.entries.length > 0 && !activeEntry()}>
        <div class="wheelSlotInactiveRow">
          <span>{props.inactiveMessage}</span>
          <div class="wheelSlotStepper">
            <button type="button" onClick={() => applyIndex(0)}>
              First
            </button>
            <button type="button" onClick={() => applyIndex(1)}>
              Next
            </button>
          </div>
        </div>
      </Show>
      <Show when={directEntry()}>
        {(entry) => (
          <div class="wheelSlotDirectRow">
            <span>{props.itemActive(entry()) ? "Active value" : "Manual value"}</span>
            <div class="visualNumberGrid wheelSlotDirectGrid">
              <label>
                %
                <input
                  type="number"
                  min="0"
                  max="100"
                  step="0.1"
                  value={valueToPercent(props.entryValue(entry()))}
                  onInput={(event) => props.onSetValue(entry(), percentToValue(Number(event.currentTarget.value)))}
                />
              </label>
              <label>
                DMX
                <input
                  type="number"
                  min="0"
                  max="65535"
                  value={clampDmxValue(props.entryValue(entry()))}
                  onInput={(event) => props.onSetValue(entry(), clampDmxValue(Number(event.currentTarget.value)))}
                />
              </label>
            </div>
          </div>
        )}
      </Show>
      <div class={`${props.itemClass}Grid`}>
        <For each={props.entries}>
          {(entry, index) => (
            <button
              type="button"
              class={props.itemActive(entry) ? "active" : ""}
              title={props.itemTitle(entry)}
              aria-pressed={props.itemActive(entry)}
              onClick={() => props.onApply(entry)}
              onKeyDown={(event) => handleSlotKeyDown(event, index())}
            >
              {props.itemIcon(entry)}
              <strong>{props.itemLabel(entry)}</strong>
              <small>{props.itemDetail(entry)}</small>
            </button>
          )}
        </For>
      </div>
      <Show when={props.entries.length === 0}>
        <p class="wheelSlotEmpty">{props.emptyMessage}</p>
      </Show>
    </div>
  );
}

interface ColorWheelSlotPanelProps {
  showPicker: boolean;
  targetLabel: string;
  entries: ColorWheelFunctionEntry[];
  pickerColor: string;
  pickerHsv: HsvColor;
  pickerSaturationRamp: string;
  approximationLabel: string;
  wheelMediaUrlFor: (media: string | null | undefined) => string | null;
  wheelSlotMediaPath: (media: string | null | undefined) => string | null;
  functionLabel: (fn: ChannelFunctionSummary) => string;
  functionRangeLabel: (fn: ChannelFunctionSummary) => string;
  functionDetail: (fn: ChannelFunctionSummary) => string;
  onPointerColor: (event: PointerEvent) => void;
  onSetColor: (hexColor: string) => void;
  onSetHsv: (updates: Partial<HsvColor>) => void;
  onSetValue: (control: AttributeControl, value: number) => void;
  onApplyFunction: (control: AttributeControl, fn: ChannelFunctionSummary) => void;
}

export function ColorWheelSlotPanel(props: ColorWheelSlotPanelProps) {
  return (
    <>
      <Show when={props.showPicker}>
        <div class="visualControlPanel colorWheelPickerPanel" data-color-wheel-picker>
          <div class="visualControlHeader">
            <div>
              <strong>Wheel Color</strong>
              <span>Generic HSV</span>
            </div>
            <span title={props.targetLabel}>{props.targetLabel}</span>
          </div>
          <HsvColorPicker
            color={props.pickerColor}
            hsv={props.pickerHsv}
            approximationLabel={props.approximationLabel}
            onPointerColor={props.onPointerColor}
            onSetColor={props.onSetColor}
            onSetHsv={props.onSetHsv}
          />
          <HsvColorAdjustments
            hsv={props.pickerHsv}
            saturationRamp={props.pickerSaturationRamp}
            onSetHsv={props.onSetHsv}
          />
        </div>
      </Show>
      <WheelSlotPanel
        title="Wheel Slots"
        countLabel={`${props.entries.length} color function(s)`}
        targetLabel={props.targetLabel}
        entries={props.entries}
        itemClass="colorWheelSlot"
        emptyMessage="No color wheel functions in this category."
        inactiveMessage="Current DMX value is outside listed color wheel slots."
        entryValue={(entry) => entry.currentValue}
        onSetValue={(entry, value) => props.onSetValue(entry.control, value)}
        itemActive={(entry) => entry.active}
        itemLabel={(entry) => entry.label}
        itemDetail={(entry) =>
          `${entry.control.attribute} / ${
            props.wheelSlotMediaPath(entry.fn.wheel_slot_media) ?? props.functionRangeLabel(entry.fn)
          }`
        }
        itemTitle={(entry) =>
          `${entry.control.attribute} / ${props.functionLabel(entry.fn)} / ${props.functionRangeLabel(entry.fn)} / ${props.functionDetail(entry.fn)}`
        }
        itemIcon={(entry) => (
          <Show
            when={props.wheelMediaUrlFor(entry.fn.wheel_slot_media)}
            fallback={<span class="colorWheelSlotSwatch" style={{ "background-color": entry.color }} />}
          >
            {(url) => <img class="wheelSlotImage color" src={url()} alt="" />}
          </Show>
        )}
        onApply={(entry) => props.onApplyFunction(entry.control, entry.fn)}
      />
    </>
  );
}

interface GoboWheelSlotPanelProps {
  targetLabel: string;
  entries: GoboWheelFunctionEntry[];
  wheelMediaUrlFor: (media: string | null | undefined) => string | null;
  wheelSlotMediaPath: (media: string | null | undefined) => string | null;
  functionLabel: (fn: ChannelFunctionSummary) => string;
  functionRangeLabel: (fn: ChannelFunctionSummary) => string;
  functionDetail: (fn: ChannelFunctionSummary) => string;
  onSetValue: (control: AttributeControl, value: number) => void;
  onApplyFunction: (control: AttributeControl, fn: ChannelFunctionSummary) => void;
}

export function GoboWheelSlotPanel(props: GoboWheelSlotPanelProps) {
  return (
    <WheelSlotPanel
      title="Gobo Slots"
      countLabel={`${props.entries.length} function(s)`}
      targetLabel={props.targetLabel}
      entries={props.entries}
      itemClass="goboWheelSlot"
      emptyMessage="No gobo wheel functions in this category."
      inactiveMessage="Current DMX value is outside listed gobo slots."
      entryValue={(entry) => entry.currentValue}
      onSetValue={(entry, value) => props.onSetValue(entry.control, value)}
      itemActive={(entry) => entry.active}
      itemLabel={(entry) => entry.label}
      itemDetail={(entry) =>
        `${entry.control.attribute} / ${
          props.wheelSlotMediaPath(entry.fn.wheel_slot_media) ?? props.functionRangeLabel(entry.fn)
        }`
      }
      itemTitle={(entry) =>
        `${entry.control.attribute} / ${props.functionLabel(entry.fn)} / ${props.functionRangeLabel(entry.fn)} / ${props.functionDetail(entry.fn)}`
      }
      itemIcon={(entry) => (
        <Show
          when={props.wheelMediaUrlFor(entry.fn.wheel_slot_media)}
          fallback={<span class={`goboWheelSlotIcon ${entry.pattern}`} />}
        >
          {(url) => <img class="wheelSlotImage gobo" src={url()} alt="" />}
        </Show>
      )}
      onApply={(entry) => props.onApplyFunction(entry.control, entry.fn)}
    />
  );
}
