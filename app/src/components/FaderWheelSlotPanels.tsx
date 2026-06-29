import { Show } from "solid-js";
import {
  ColorWheelSlotPanel,
  GoboWheelSlotPanel,
  type ColorWheelFunctionEntry,
  type GoboWheelFunctionEntry,
} from "./WheelSlotPanel";
import type { AttributeControl, ChannelFunctionSummary } from "../types";

interface FaderWheelSlotPanelsProps {
  showColorWheel: boolean;
  showGoboWheel: boolean;
  targetLabel: string;
  colorEntries: ColorWheelFunctionEntry[];
  goboEntries: GoboWheelFunctionEntry[];
  wheelMediaUrlFor: (media: string | null | undefined) => string | null;
  wheelSlotMediaPath: (media: string | null | undefined) => string | null;
  functionLabel: (fn: ChannelFunctionSummary) => string;
  functionRangeLabel: (fn: ChannelFunctionSummary) => string;
  functionDetail: (fn: ChannelFunctionSummary) => string;
  onSetValue: (control: AttributeControl, value: number) => void;
  onApplyFunction: (control: AttributeControl, fn: ChannelFunctionSummary) => void;
}

export function FaderWheelSlotPanels(props: FaderWheelSlotPanelsProps) {
  return (
    <>
      <Show when={props.showColorWheel}>
        <ColorWheelSlotPanel
          targetLabel={props.targetLabel}
          entries={props.colorEntries}
          wheelMediaUrlFor={props.wheelMediaUrlFor}
          wheelSlotMediaPath={props.wheelSlotMediaPath}
          functionLabel={props.functionLabel}
          functionRangeLabel={props.functionRangeLabel}
          functionDetail={props.functionDetail}
          onSetValue={props.onSetValue}
          onApplyFunction={props.onApplyFunction}
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
