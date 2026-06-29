import { Show } from "solid-js";
import type { JSX } from "solid-js";
import { ColorControlPanel, type ColorControlSet, type ColorExtraControl, type HsvColor } from "./ColorControlPanel";
import { DimmerControlPanel, type DimmerControlSet } from "./DimmerControlPanel";
import { PositionControlPanel, type PositionControlSet, type PositionFavorite } from "./PositionControlPanel";
import type { FixtureLimits } from "../types";

interface PanTiltNudgeStep {
  label: string;
  value: number;
}

interface PanTiltTargetPoint {
  label: string;
  pan: number;
  tilt: number;
}

interface ColorQuickLook {
  label: string;
  color: string;
}

type DimmerLimitField = "dimmer_min" | "dimmer_max";
type MovementLimitField = "pan_min" | "pan_max" | "tilt_min" | "tilt_max";
type MovementLimitToggle = "invert_pan" | "invert_tilt" | "swap_pan_tilt";

interface FaderPrimaryAttributePanelsProps {
  dimmerControl?: DimmerControlSet;
  positionControls?: PositionControlSet;
  colorControls?: ColorControlSet;
  dimmerSliderMin: number;
  dimmerSliderMax: number;
  canEditLimits: boolean;
  limitsDraft: FixtureLimits;
  normalizedLimits: FixtureLimits;
  applyLimitsLabel: string;
  nudgeAmount: number;
  nudgeSteps: PanTiltNudgeStep[];
  targetPoints: PanTiltTargetPoint[];
  limitOverlayStyle: JSX.CSSProperties;
  limitWindowStyle: JSX.CSSProperties;
  movementLimitDragging: boolean;
  positionFavorites: PositionFavorite[];
  positionFavoriteLabel: string;
  colorHsv: HsvColor;
  colorSaturationRamp: string;
  colorAutoWhite: boolean;
  colorQuickLooks: ColorQuickLook[];
  colorPalette: string[];
  colorFavorites: string[];
  targetLabel: string;
  formatDmxPercent: (value: number) => string;
  formatShortDmxPercent: (value: number) => string;
  dmxValueToPercent: (value: number) => number;
  clampDmxValue: (value: number) => number;
  colorPreviewForSaturation: (saturation: number) => string;
  onSetDimmerValue: (value: number) => void;
  onUpdateDimmerLimit: (field: DimmerLimitField, value: number) => void;
  onResetDimmerLimits: () => void;
  onApplyLimits: () => void;
  onNudgePanTilt: (deltaPan: number, deltaTilt: number) => void;
  onCenterPanTilt: () => void;
  onMirrorPanTiltAxis: (axis: "pan" | "tilt" | "both") => void;
  onSetPanTiltTarget: (panRatio: number, tiltRatio: number) => void;
  onPanTiltPointerPad: (event: PointerEvent) => void;
  onPanTiltPadKeyDown: (event: KeyboardEvent) => void;
  onSetPanTiltValues: (panValue: number, tiltValue: number) => void;
  onSetPanTiltPercent: (axis: "pan" | "tilt", percent: number) => void;
  onSetPanTiltNudgeAmount: (value: number) => void;
  onMovementLimitPointerDown: (event: PointerEvent & { currentTarget: HTMLElement }) => void;
  onMovementLimitPointerMove: (event: PointerEvent & { currentTarget: HTMLElement }) => void;
  onMovementLimitPointerEnd: (event: PointerEvent & { currentTarget: HTMLElement }) => void;
  onUpdateMovementLimit: (field: MovementLimitField, value: number) => void;
  onUpdateMovementLimitToggle: (field: MovementLimitToggle, value: boolean) => void;
  onResetMovementLimits: () => void;
  onSetPositionFavoriteLabel: (value: string) => void;
  onAddPositionFavorite: () => void;
  onResetPositionFavorites: () => void;
  onRemovePositionFavorite: (favoriteId: string) => void;
  onApplyPositionFavorite: (pan: number, tilt: number) => void;
  onPointerColor: (event: PointerEvent) => void;
  onSetColor: (hexColor: string) => void;
  onSetColorAutoWhite: (enabled: boolean) => void;
  onSetColorChannel: (channel: "red" | "green" | "blue", value: number) => void;
  onSetColorExtraChannel: (extra: ColorExtraControl, value: number) => void;
  onSetColorHsv: (updates: Partial<HsvColor>) => void;
  onAddColorFavorite: () => void;
  onResetColorFavorites: () => void;
  onRemoveColorFavorite: (color: string) => void;
}

export function FaderPrimaryAttributePanels(props: FaderPrimaryAttributePanelsProps) {
  return (
    <>
      <Show when={props.dimmerControl}>
        {(dimmerControl) => (
          <DimmerControlPanel
            control={dimmerControl()}
            sliderMin={props.dimmerSliderMin}
            sliderMax={props.dimmerSliderMax}
            canEditLimits={props.canEditLimits}
            limitsDraft={props.limitsDraft}
            normalizedLimits={props.normalizedLimits}
            formatDmxPercent={props.formatDmxPercent}
            applyLimitsLabel={props.applyLimitsLabel}
            onSetValue={props.onSetDimmerValue}
            onUpdateLimit={props.onUpdateDimmerLimit}
            onResetLimits={props.onResetDimmerLimits}
            onApplyLimits={props.onApplyLimits}
          />
        )}
      </Show>
      <Show when={props.positionControls}>
        {(positionControls) => (
          <PositionControlPanel
            controls={positionControls()}
            nudgeAmount={props.nudgeAmount}
            nudgeSteps={props.nudgeSteps}
            targetPoints={props.targetPoints}
            limitOverlayStyle={props.limitOverlayStyle}
            limitWindowStyle={props.limitWindowStyle}
            movementLimitDragging={props.movementLimitDragging}
            canEditLimits={props.canEditLimits}
            limitsDraft={props.limitsDraft}
            normalizedLimits={props.normalizedLimits}
            favorites={props.positionFavorites}
            favoriteLabel={props.positionFavoriteLabel}
            formatDmxPercent={props.formatDmxPercent}
            formatShortDmxPercent={props.formatShortDmxPercent}
            applyLimitsLabel={props.applyLimitsLabel}
            dmxValueToPercent={props.dmxValueToPercent}
            clampDmxValue={props.clampDmxValue}
            onNudge={props.onNudgePanTilt}
            onCenter={props.onCenterPanTilt}
            onMirrorAxis={props.onMirrorPanTiltAxis}
            onSetTarget={props.onSetPanTiltTarget}
            onPointerPad={props.onPanTiltPointerPad}
            onPadKeyDown={props.onPanTiltPadKeyDown}
            onSetValues={props.onSetPanTiltValues}
            onSetPercent={props.onSetPanTiltPercent}
            onSetNudgeAmount={props.onSetPanTiltNudgeAmount}
            onMovementLimitPointerDown={props.onMovementLimitPointerDown}
            onMovementLimitPointerMove={props.onMovementLimitPointerMove}
            onMovementLimitPointerEnd={props.onMovementLimitPointerEnd}
            onUpdateLimit={props.onUpdateMovementLimit}
            onUpdateLimitToggle={props.onUpdateMovementLimitToggle}
            onResetMovementLimits={props.onResetMovementLimits}
            onApplyLimits={props.onApplyLimits}
            onSetFavoriteLabel={props.onSetPositionFavoriteLabel}
            onAddFavorite={props.onAddPositionFavorite}
            onResetFavorites={props.onResetPositionFavorites}
            onRemoveFavorite={props.onRemovePositionFavorite}
            onApplyFavorite={props.onApplyPositionFavorite}
          />
        )}
      </Show>
      <Show when={props.colorControls}>
        {(colorControls) => (
          <ColorControlPanel
            controls={colorControls()}
            hsv={props.colorHsv}
            saturationRamp={props.colorSaturationRamp}
            autoWhite={props.colorAutoWhite}
            quickLooks={props.colorQuickLooks}
            palette={props.colorPalette}
            favorites={props.colorFavorites}
            targetLabel={props.targetLabel}
            formatPercent={props.formatShortDmxPercent}
            previewForSaturation={props.colorPreviewForSaturation}
            onPointerColor={props.onPointerColor}
            onSetColor={props.onSetColor}
            onSetAutoWhite={props.onSetColorAutoWhite}
            onSetChannel={props.onSetColorChannel}
            onSetExtraChannel={props.onSetColorExtraChannel}
            onSetHsv={props.onSetColorHsv}
            onAddFavorite={props.onAddColorFavorite}
            onResetFavorites={props.onResetColorFavorites}
            onRemoveFavorite={props.onRemoveColorFavorite}
          />
        )}
      </Show>
    </>
  );
}
