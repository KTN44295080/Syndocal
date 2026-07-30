import { Show } from "solid-js";
import {
  ColorWheelSlotPanel,
  GoboWheelSlotPanel,
  type ColorWheelFunctionEntry,
  type GoboWheelFunctionEntry,
} from "./WheelSlotPanel";
import type { HsvColor } from "./HsvColorPicker";
import type { AttributeControl, ChannelFunctionSummary } from "../types";

interface FaderWheelSlotPanelsProps {
  showColorWheel: boolean;
  showColorWheelPicker: boolean;
  showGoboWheel: boolean;
  targetLabel: string;
  colorEntries: ColorWheelFunctionEntry[];
  goboEntries: GoboWheelFunctionEntry[];
  colorWheelPickerColor: string;
  colorWheelPickerHsv: HsvColor;
  colorWheelSaturationRamp: string;
  colorWheelApproximationLabel: string;
  wheelMediaUrlFor: (media: string | null | undefined) => string | null;
  wheelSlotMediaPath: (media: string | null | undefined) => string | null;
  functionLabel: (fn: ChannelFunctionSummary) => string;
  functionRangeLabel: (fn: ChannelFunctionSummary) => string;
  functionDetail: (fn: ChannelFunctionSummary) => string;
  onColorWheelPointerColor: (event: PointerEvent) => void;
  onSetColorWheelColor: (hexColor: string) => void;
  onSetColorWheelHsv: (updates: Partial<HsvColor>) => void;
  onSetValue: (control: AttributeControl, value: number) => void;
  onApplyColorFunction: (control: AttributeControl, fn: ChannelFunctionSummary) => void;
  onApplyFunction: (control: AttributeControl, fn: ChannelFunctionSummary) => void;
}

export function FaderWheelSlotPanels(props: FaderWheelSlotPanelsProps) {
  return (
    <>
      <Show when={props.showColorWheel}>
        <ColorWheelSlotPanel
          showPicker={props.showColorWheelPicker}
          targetLabel={props.targetLabel}
          entries={props.colorEntries}
          pickerColor={props.colorWheelPickerColor}
          pickerHsv={props.colorWheelPickerHsv}
          pickerSaturationRamp={props.colorWheelSaturationRamp}
          approximationLabel={props.colorWheelApproximationLabel}
          wheelMediaUrlFor={props.wheelMediaUrlFor}
          wheelSlotMediaPath={props.wheelSlotMediaPath}
          functionLabel={props.functionLabel}
          functionRangeLabel={props.functionRangeLabel}
          functionDetail={props.functionDetail}
          onPointerColor={props.onColorWheelPointerColor}
          onSetColor={props.onSetColorWheelColor}
          onSetHsv={props.onSetColorWheelHsv}
          onSetValue={props.onSetValue}
          onApplyFunction={props.onApplyColorFunction}
        />
      </Show>
      <Show when={props.showGoboWheel}>
        <GoboWheelSlotPanel
          targetLabel={props.targetLabel}
          entries={props.goboEntries}
          wheelMediaUrlFor={props.wheelMediaUrlFor}
          wheelSlotMediaPath={props.wheelSlotMediaPath}
          functionLabel={props.functionLabel}
          functionRangeLabel={props.functionRangeLabel}
          functionDetail={props.functionDetail}
          onSetValue={props.onSetValue}
          onApplyFunction={props.onApplyFunction}
        />
      </Show>
    </>
  );
}
