import { Show } from "solid-js";
import type { JSX } from "solid-js";
import { CategoryQuickPanel, type CategoryQuickLook, type CategoryQuickValueMode } from "./CategoryQuickPanel";
import { ChannelFunctionPanel, type ChannelFunctionControlEntry } from "./ChannelFunctionPanel";
import { FaderWheelSlotPanels } from "./FaderWheelSlotPanels";
import { OpticsControlPanel, type OpticsControlEntry } from "./OpticsControlPanel";
import type { ColorWheelFunctionEntry, GoboWheelFunctionEntry } from "./WheelSlotPanel";
import type { HsvColor } from "./HsvColorPicker";
import type { AttributeControl, ChannelFunctionSummary } from "../types";

interface FaderAuxiliaryAttributePanelsProps {
  showColorWheel: boolean;
  showColorWheelPicker: boolean;
  showGoboWheel: boolean;
  showOptics: boolean;
  showCategoryQuick: boolean;
  showFunctions: boolean;
  targetLabel: string;
  categoryLabel: string;
  attributeCount: number;
  colorWheelEntries: ColorWheelFunctionEntry[];
  goboWheelEntries: GoboWheelFunctionEntry[];
  colorWheelPickerColor: string;
  colorWheelPickerHsv: HsvColor;
  colorWheelSaturationRamp: string;
  colorWheelApproximationLabel: string;
  wheelMediaUrlFor: (media: string | null | undefined) => string | null;
  wheelSlotMediaPath: (media: string | null | undefined) => string | null;
  opticsTitle: string;
  opticsEntries: OpticsControlEntry[];
  formatShortDmxPercent: (value: number) => string;
  clampDmxValue: (value: number) => number;
  opticsPreviewClass: (role: string) => string;
  opticsPreviewStyle: (entry: OpticsControlEntry) => JSX.CSSProperties;
  opticsPresetButtons: (entry: OpticsControlEntry) => { label: string; value: number }[];
  sortedFunctions: (control: AttributeControl) => ChannelFunctionSummary[];
  functionContainsValue: (fn: ChannelFunctionSummary, value: number) => boolean;
  functionBandStyle: (control: AttributeControl, fn: ChannelFunctionSummary) => JSX.CSSProperties;
  functionLabel: (fn: ChannelFunctionSummary) => string;
  functionRangeLabel: (fn: ChannelFunctionSummary) => string;
  functionDetail: (fn: ChannelFunctionSummary) => string;
  functionSwatchColor: (control: AttributeControl, fn: ChannelFunctionSummary) => string | null;
  quickLooks: CategoryQuickLook[];
  functionEntries: ChannelFunctionControlEntry[];
  currentValue: (control: AttributeControl) => number;
  onColorWheelPointerColor: (event: PointerEvent) => void;
  onSetColorWheelColor: (hexColor: string) => void;
  onSetColorWheelHsv: (updates: Partial<HsvColor>) => void;
  onOpticsPointerValue: (event: PointerEvent, control: AttributeControl) => void;
  onOpticsKeyValue: (event: KeyboardEvent, control: AttributeControl, value: number) => void;
  onSetValue: (control: AttributeControl, value: number) => void;
  onApplyColorFunction: (control: AttributeControl, fn: ChannelFunctionSummary) => void;
  onApplyFunction: (control: AttributeControl, fn: ChannelFunctionSummary) => void;
  onSetValueMode: (mode: CategoryQuickValueMode) => void;
  onApplyLook: (look: CategoryQuickLook) => void;
}

export function FaderAuxiliaryAttributePanels(props: FaderAuxiliaryAttributePanelsProps) {
  return (
    <>
      <FaderWheelSlotPanels
        showColorWheel={props.showColorWheel}
        showColorWheelPicker={props.showColorWheelPicker}
        showGoboWheel={props.showGoboWheel}
        targetLabel={props.targetLabel}
        colorEntries={props.colorWheelEntries}
        goboEntries={props.goboWheelEntries}
        colorWheelPickerColor={props.colorWheelPickerColor}
        colorWheelPickerHsv={props.colorWheelPickerHsv}
        colorWheelSaturationRamp={props.colorWheelSaturationRamp}
        colorWheelApproximationLabel={props.colorWheelApproximationLabel}
        wheelMediaUrlFor={props.wheelMediaUrlFor}
        wheelSlotMediaPath={props.wheelSlotMediaPath}
        functionLabel={props.functionLabel}
        functionRangeLabel={props.functionRangeLabel}
        functionDetail={props.functionDetail}
        onColorWheelPointerColor={props.onColorWheelPointerColor}
        onSetColorWheelColor={props.onSetColorWheelColor}
        onSetColorWheelHsv={props.onSetColorWheelHsv}
        onSetValue={props.onSetValue}
        onApplyColorFunction={props.onApplyColorFunction}
        onApplyFunction={props.onApplyFunction}
      />
      <Show when={props.showOptics}>
        <OpticsControlPanel
          title={props.opticsTitle}
          count={props.opticsEntries.length}
          targetLabel={props.targetLabel}
          entries={props.opticsEntries}
          formatShortDmxPercent={props.formatShortDmxPercent}
          clampDmxValue={props.clampDmxValue}
          previewClass={props.opticsPreviewClass}
          previewStyle={props.opticsPreviewStyle}
          presetButtons={props.opticsPresetButtons}
          sortedFunctions={props.sortedFunctions}
          functionContainsValue={props.functionContainsValue}
          functionBandStyle={props.functionBandStyle}
          functionLabel={props.functionLabel}
          functionRangeLabel={props.functionRangeLabel}
          functionDetail={props.functionDetail}
          onPointerValue={props.onOpticsPointerValue}
          onKeyValue={props.onOpticsKeyValue}
          onSetValue={props.onSetValue}
          onApplyFunction={props.onApplyFunction}
        />
      </Show>
      <Show when={props.showCategoryQuick}>
        <CategoryQuickPanel
          categoryLabel={props.categoryLabel}
          attributeCount={props.attributeCount}
          looks={props.quickLooks}
          onSetValueMode={props.onSetValueMode}
          onApplyLook={props.onApplyLook}
        />
      </Show>
      <Show when={props.showFunctions && props.functionEntries.length > 0}>
        <ChannelFunctionPanel
          categoryLabel={props.categoryLabel}
          entries={props.functionEntries}
          currentValue={props.currentValue}
          clampDmxValue={props.clampDmxValue}
          functionContainsValue={props.functionContainsValue}
          functionBandStyle={props.functionBandStyle}
          functionLabel={props.functionLabel}
          functionRangeLabel={props.functionRangeLabel}
          functionDetail={props.functionDetail}
          functionSwatchColor={props.functionSwatchColor}
          onApplyFunction={props.onApplyFunction}
        />
      </Show>
    </>
  );
}
