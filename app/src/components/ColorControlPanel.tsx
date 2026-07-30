import { For, Show } from "solid-js";
import { HsvColorAdjustments, HsvColorPicker, type HsvColor } from "./HsvColorPicker";

export type { HsvColor } from "./HsvColorPicker";

export type ColorExtraChannelKey = "white" | "amber" | "uv";

export interface ColorExtraControl {
  key: ColorExtraChannelKey;
  label: string;
  shortLabel: string;
  attribute: string;
  value: number;
}

export interface ColorControlSet {
  red: string;
  green: string;
  blue: string;
  redValue: number;
  greenValue: number;
  blueValue: number;
  extras: ColorExtraControl[];
  value: string;
}

interface ColorQuickLook {
  label: string;
  color: string;
}

interface ColorControlPanelProps {
  controls: ColorControlSet;
  hsv: HsvColor;
  saturationRamp: string;
  autoWhite: boolean;
  quickLooks: ColorQuickLook[];
  palette: string[];
  favorites: string[];
  targetLabel: string;
  formatPercent: (value: number) => string;
  previewForSaturation: (saturation: number) => string;
  onPointerColor: (event: PointerEvent) => void;
  onSetColor: (hexColor: string) => void;
  onSetAutoWhite: (enabled: boolean) => void;
  onSetChannel: (channel: "red" | "green" | "blue", value: number) => void;
  onSetExtraChannel: (extra: ColorExtraControl, value: number) => void;
  onSetHsv: (updates: Partial<HsvColor>) => void;
  onAddFavorite: () => void;
  onResetFavorites: () => void;
  onRemoveFavorite: (color: string) => void;
}

const normalizeColorHex = (value: string) => {
  const trimmed = value.trim().toLowerCase();
  return /^#[0-9a-f]{6}$/.test(trimmed) ? trimmed : "";
};

export function ColorControlPanel(props: ColorControlPanelProps) {
  const rgbLevelLabel = () =>
    `${props.formatPercent(props.controls.redValue)} / ${props.formatPercent(props.controls.greenValue)} / ${props.formatPercent(
      props.controls.blueValue,
    )}`;
  const colorMatchesCurrent = (color: string) =>
    normalizeColorHex(color) !== "" && normalizeColorHex(color) === normalizeColorHex(props.controls.value);

  return (
    <div class="visualControlPanel colorControlPanel">
      <div class="visualControlHeader">
        <div>
          <strong>Color</strong>
          <span>
            Hue {Math.round(props.hsv.hue)} deg / Sat {Math.round(props.hsv.saturation * 100)}%
          </span>
        </div>
        <span title={props.targetLabel}>
          {[props.controls.red, props.controls.green, props.controls.blue, ...props.controls.extras.map((extra) => extra.attribute)].join(" / ")}
        </span>
      </div>
      <div class="visualReadoutStrip colorReadoutStrip">
        <span class="currentColorReadout">
          <i style={{ "background-color": props.controls.value }} />
          <strong>{props.controls.value.toUpperCase()}</strong>
        </span>
        <span>
          <small>Hue</small>
          <strong>{Math.round(props.hsv.hue)} deg</strong>
        </span>
        <span>
          <small>Sat / Val</small>
          <strong>{Math.round(props.hsv.saturation * 100)}% / {Math.round(props.hsv.value * 100)}%</strong>
        </span>
        <span title={rgbLevelLabel()}>
          <small>RGB</small>
          <strong>{rgbLevelLabel()}</strong>
        </span>
      </div>
      <HsvColorPicker
        color={props.controls.value}
        hsv={props.hsv}
        onPointerColor={props.onPointerColor}
        onSetColor={props.onSetColor}
        onSetHsv={props.onSetHsv}
        sideContent={
          <>
          <Show when={props.controls.extras.some((extra) => extra.key === "white")}>
            <label class="colorAutoWhiteToggle">
              <input
                type="checkbox"
                checked={props.autoWhite}
                onChange={(event) => props.onSetAutoWhite(event.currentTarget.checked)}
              />
              Auto White
            </label>
          </Show>
          <div class="rgbChannelRack" aria-label="RGB channel sliders">
            <label class="rgbChannelSlider red">
              <span>R</span>
              <input
                type="range"
                min="0"
                max="65535"
                value={props.controls.redValue}
                aria-label="Red channel"
                onInput={(event) => props.onSetChannel("red", Number(event.currentTarget.value))}
              />
              <small>{props.formatPercent(props.controls.redValue)}</small>
            </label>
            <label class="rgbChannelSlider green">
              <span>G</span>
              <input
                type="range"
                min="0"
                max="65535"
                value={props.controls.greenValue}
                aria-label="Green channel"
                onInput={(event) => props.onSetChannel("green", Number(event.currentTarget.value))}
              />
              <small>{props.formatPercent(props.controls.greenValue)}</small>
            </label>
            <label class="rgbChannelSlider blue">
              <span>B</span>
              <input
                type="range"
                min="0"
                max="65535"
                value={props.controls.blueValue}
                aria-label="Blue channel"
                onInput={(event) => props.onSetChannel("blue", Number(event.currentTarget.value))}
              />
              <small>{props.formatPercent(props.controls.blueValue)}</small>
            </label>
          </div>
          <Show when={props.controls.extras.length > 0}>
            <div class="extraColorChannelRack" aria-label="Additional color channel sliders">
              <For each={props.controls.extras}>
                {(extra) => (
                  <label class={`rgbChannelSlider extra ${extra.key}`}>
                    <span>{extra.shortLabel}</span>
                    <input
                      type="range"
                      min="0"
                      max="65535"
                      value={extra.value}
                      aria-label={`${extra.label} channel`}
                      onInput={(event) => props.onSetExtraChannel(extra, Number(event.currentTarget.value))}
                    />
                    <small>{props.formatPercent(extra.value)}</small>
                  </label>
                )}
              </For>
            </div>
          </Show>
          </>
        }
      />
      <div class="colorQuickLookRow">
        <For each={props.quickLooks}>
          {(quick) => (
            <button
              class={colorMatchesCurrent(quick.color) ? "active" : ""}
              onClick={() => props.onSetColor(quick.color)}
              title={`${quick.color}${colorMatchesCurrent(quick.color) ? " / current" : ""}`}
            >
              <i style={{ "background-color": quick.color }} />
              <span>{quick.label}</span>
            </button>
          )}
        </For>
        <button class={Math.abs(props.hsv.saturation - 1) < 0.005 ? "active" : ""} onClick={() => props.onSetHsv({ saturation: 1 })}>
          <i style={{ "background-color": props.previewForSaturation(1) }} />
          <span>Full Sat</span>
        </button>
        <button class={Math.abs(props.hsv.saturation - 0.25) < 0.005 ? "active" : ""} onClick={() => props.onSetHsv({ saturation: 0.25 })}>
          <i style={{ "background-color": props.previewForSaturation(0.25) }} />
          <span>Soft</span>
        </button>
      </div>
      <HsvColorAdjustments hsv={props.hsv} saturationRamp={props.saturationRamp} onSetHsv={props.onSetHsv} />
      <div class="colorSwatchPanel">
        <div class="swatchSection">
          <strong>Palette</strong>
          <div class="colorSwatchGrid">
            <For each={props.palette}>
              {(color) => (
                <button
                  class={colorMatchesCurrent(color) ? "colorSwatch active" : "colorSwatch"}
                  style={{ "background-color": color }}
                  title={`${color}${colorMatchesCurrent(color) ? " / current" : ""}`}
                  onClick={() => props.onSetColor(color)}
                  aria-label={`Set color ${color}${colorMatchesCurrent(color) ? ", current" : ""}`}
                />
              )}
            </For>
          </div>
        </div>
        <div class="swatchSection">
          <div class="swatchHeader">
            <strong>Favorites</strong>
            <div class="miniButtonRow">
              <button onClick={props.onAddFavorite}>+</button>
              <button onClick={props.onResetFavorites}>Reset</button>
            </div>
          </div>
          <div class="colorSwatchGrid">
            <For each={props.favorites}>
              {(color) => (
                <button
                  class={colorMatchesCurrent(color) ? "colorSwatch favorite active" : "colorSwatch favorite"}
                  style={{ "background-color": color }}
                  title={`${color}${colorMatchesCurrent(color) ? " / current" : ""} / Shift-click removes`}
                  onClick={(event) => {
                    if (event.shiftKey) {
                      props.onRemoveFavorite(color);
                    } else {
                      props.onSetColor(color);
                    }
                  }}
                  aria-label={`Favorite color ${color}${colorMatchesCurrent(color) ? ", current" : ""}`}
                />
              )}
            </For>
          </div>
        </div>
      </div>
    </div>
  );
}
