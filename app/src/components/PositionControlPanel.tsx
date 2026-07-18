import { For, Show, createSignal } from "solid-js";
import type { JSX } from "solid-js";
import type { FixtureLimits } from "../types";

export interface PositionControlSet {
  pan: string;
  tilt: string;
  panValue: number;
  tiltValue: number;
}

export interface PositionFavorite {
  id: string;
  label: string;
  pan: number;
  tilt: number;
}

interface PanTiltNudgeStep {
  label: string;
  value: number;
}

interface PanTiltTargetPoint {
  label: string;
  pan: number;
  tilt: number;
}

type MovementLimitField = "pan_min" | "pan_max" | "tilt_min" | "tilt_max";
type MovementLimitToggle = "invert_pan" | "invert_tilt" | "swap_pan_tilt";
type PositionToolTab = "position" | "limits";

interface PositionControlPanelProps {
  controls: PositionControlSet;
  nudgeAmount: number;
  nudgeSteps: PanTiltNudgeStep[];
  targetPoints: PanTiltTargetPoint[];
  limitOverlayStyle: JSX.CSSProperties;
  limitWindowStyle: JSX.CSSProperties;
  movementLimitDragging: boolean;
  canEditLimits: boolean;
  limitsDraft: FixtureLimits;
  normalizedLimits: FixtureLimits;
  favorites: PositionFavorite[];
  favoriteLabel: string;
  formatDmxPercent: (value: number) => string;
  formatShortDmxPercent: (value: number) => string;
  applyLimitsLabel: string;
  dmxValueToPercent: (value: number) => number;
  onNudge: (deltaPan: number, deltaTilt: number) => void;
  onCenter: () => void;
  onMirrorAxis: (axis: "pan" | "tilt" | "both") => void;
  onSetTarget: (panRatio: number, tiltRatio: number) => void;
  onPointerPad: (event: PointerEvent) => void;
  onPadKeyDown: (event: KeyboardEvent) => void;
  onSetValues: (panValue: number, tiltValue: number) => void;
  onSetPercent: (axis: "pan" | "tilt", percent: number) => void;
  onSetNudgeAmount: (value: number) => void;
  onMovementLimitPointerDown: (event: PointerEvent & { currentTarget: HTMLElement }) => void;
  onMovementLimitPointerMove: (event: PointerEvent & { currentTarget: HTMLElement }) => void;
  onMovementLimitPointerEnd: (event: PointerEvent & { currentTarget: HTMLElement }) => void;
  onUpdateLimit: (field: MovementLimitField, value: number) => void;
  onUpdateLimitToggle: (field: MovementLimitToggle, value: boolean) => void;
  onResetMovementLimits: () => void;
  onApplyLimits: () => void;
  onSetFavoriteLabel: (value: string) => void;
  onAddFavorite: () => void;
  onResetFavorites: () => void;
  onRemoveFavorite: (favoriteId: string) => void;
  onApplyFavorite: (pan: number, tilt: number) => void;
}

const clampDmxLimit = (value: number) => Math.min(65_535, Math.max(0, Math.round(Number.isFinite(value) ? value : 0)));
const favoriteMatchTolerance = 512;

const limitKeyboardStep = (event: KeyboardEvent, baseAmount: number) => {
  const base = Math.max(1, Math.round(baseAmount || 1));
  if (event.shiftKey) {
    return base * 4;
  }
  if (event.altKey) {
    return Math.max(1, Math.round(base / 4));
  }
  return base;
};

const moveLimitRange = (min: number, max: number, delta: number) => {
  const normalizedMin = Math.min(clampDmxLimit(min), clampDmxLimit(max));
  const normalizedMax = Math.max(clampDmxLimit(min), clampDmxLimit(max));
  const width = normalizedMax - normalizedMin;
  const nextMin = Math.min(65_535 - width, Math.max(0, normalizedMin + delta));
  return { min: nextMin, max: nextMin + width };
};

const resizeLimitRange = (min: number, max: number, delta: number) => {
  const normalizedMin = Math.min(clampDmxLimit(min), clampDmxLimit(max));
  const normalizedMax = Math.max(clampDmxLimit(min), clampDmxLimit(max));
  const center = (normalizedMin + normalizedMax) / 2;
  const halfWidth = Math.max(0, (normalizedMax - normalizedMin) / 2 + delta);
  return {
    min: clampDmxLimit(center - halfWidth),
    max: clampDmxLimit(center + halfWidth),
  };
};

export function PositionControlPanel(props: PositionControlPanelProps) {
  const [positionToolTab, setPositionToolTab] = createSignal<PositionToolTab>("position");
  const panPercent = () => Math.round((props.controls.panValue / 65_535) * 1000) / 10;
  const tiltPercent = () => Math.round((props.controls.tiltValue / 65_535) * 1000) / 10;
  const limitSpanLabel = () =>
    `${props.formatShortDmxPercent(props.normalizedLimits.pan_min)}-${props.formatShortDmxPercent(
      props.normalizedLimits.pan_max,
    )} / ${props.formatShortDmxPercent(props.normalizedLimits.tilt_min)}-${props.formatShortDmxPercent(
      props.normalizedLimits.tilt_max,
    )}`;
  const favoriteMatchesCurrent = (favorite: PositionFavorite) =>
    Math.abs(clampDmxLimit(favorite.pan) - clampDmxLimit(props.controls.panValue)) <= favoriteMatchTolerance &&
    Math.abs(clampDmxLimit(favorite.tilt) - clampDmxLimit(props.controls.tiltValue)) <= favoriteMatchTolerance;
  const activePositionToolTab = (): PositionToolTab =>
    positionToolTab() === "limits" && props.canEditLimits ? "limits" : "position";

  const setMovementLimitRange = (panMin: number, panMax: number, tiltMin: number, tiltMax: number) => {
    props.onUpdateLimit("pan_min", panMin);
    props.onUpdateLimit("pan_max", panMax);
    props.onUpdateLimit("tilt_min", tiltMin);
    props.onUpdateLimit("tilt_max", tiltMax);
  };

  const handleMovementLimitKeyDown = (event: KeyboardEvent) => {
    const step = limitKeyboardStep(event, props.nudgeAmount);
    const limits = props.normalizedLimits;
    if (event.key === "ArrowLeft" || event.key === "ArrowRight") {
      event.preventDefault();
      const delta = event.key === "ArrowLeft" ? -step : step;
      const panRange = moveLimitRange(limits.pan_min, limits.pan_max, delta);
      setMovementLimitRange(panRange.min, panRange.max, limits.tilt_min, limits.tilt_max);
      return;
    }
    if (event.key === "ArrowUp" || event.key === "ArrowDown") {
      event.preventDefault();
      const delta = event.key === "ArrowUp" ? step : -step;
      const tiltRange = moveLimitRange(limits.tilt_min, limits.tilt_max, delta);
      setMovementLimitRange(limits.pan_min, limits.pan_max, tiltRange.min, tiltRange.max);
      return;
    }
    if (event.key === "PageUp" || event.key === "=" || event.key === "+") {
      event.preventDefault();
      const panRange = resizeLimitRange(limits.pan_min, limits.pan_max, step);
      const tiltRange = resizeLimitRange(limits.tilt_min, limits.tilt_max, step);
      setMovementLimitRange(panRange.min, panRange.max, tiltRange.min, tiltRange.max);
      return;
    }
    if (event.key === "PageDown" || event.key === "-") {
      event.preventDefault();
      const panRange = resizeLimitRange(limits.pan_min, limits.pan_max, -step);
      const tiltRange = resizeLimitRange(limits.tilt_min, limits.tilt_max, -step);
      setMovementLimitRange(panRange.min, panRange.max, tiltRange.min, tiltRange.max);
      return;
    }
    if (event.key === "Home") {
      event.preventDefault();
      props.onResetMovementLimits();
    }
  };

  const handlePositionToolTabKeyDown = (event: KeyboardEvent & { currentTarget: HTMLButtonElement }) => {
    if (!["ArrowLeft", "ArrowRight", "Home", "End"].includes(event.key)) {
      return;
    }
    event.preventDefault();
    const availableTabs: PositionToolTab[] = props.canEditLimits ? ["position", "limits"] : ["position"];
    const currentIndex = Math.max(0, availableTabs.indexOf(activePositionToolTab()));
    const nextIndex = event.key === "Home"
      ? 0
      : event.key === "End"
        ? availableTabs.length - 1
        : (currentIndex + (event.key === "ArrowRight" ? 1 : -1) + availableTabs.length) % availableTabs.length;
    const nextTab = availableTabs[nextIndex];
    const tabList = event.currentTarget.parentElement;
    setPositionToolTab(nextTab);
    queueMicrotask(() => tabList?.querySelector<HTMLButtonElement>(`#position-tool-tab-${nextTab}`)?.focus());
  };

  return (
    <div class="visualControlPanel positionControlPanel">
      <div class="visualControlHeader">
        <div>
          <strong>Position</strong>
          <span>
            Pan {panPercent()}% / Tilt {tiltPercent()}%
          </span>
        </div>
        <span>{props.controls.pan} / {props.controls.tilt}</span>
      </div>
      <div class="visualReadoutStrip positionReadoutStrip">
        <span>
          <small>Pan</small>
          <strong>{panPercent()}%</strong>
        </span>
        <span>
          <small>Tilt</small>
          <strong>{tiltPercent()}%</strong>
        </span>
        <span>
          <small>Nudge</small>
          <strong>{props.formatShortDmxPercent(props.nudgeAmount)}</strong>
        </span>
        <span title={limitSpanLabel()}>
          <small>Limits</small>
          <strong>{props.canEditLimits ? limitSpanLabel() : "No fixture"}</strong>
        </span>
      </div>
      <div class="positionConsoleSurface">
        <div class="positionConsolePrimary positionPrimaryDeck">
          <div class="positionPadRow">
            <div
              class="panTiltPad"
              role="group"
              aria-label="Pan tilt pad"
              tabIndex={0}
              onPointerDown={(event) => {
                event.currentTarget.setPointerCapture(event.pointerId);
                props.onPointerPad(event);
              }}
              onPointerMove={(event) => {
                if (event.buttons === 1) {
                  props.onPointerPad(event);
                }
              }}
              onPointerUp={(event) => event.currentTarget.releasePointerCapture(event.pointerId)}
              onPointerCancel={(event) => event.currentTarget.releasePointerCapture(event.pointerId)}
              onKeyDown={props.onPadKeyDown}
            >
              <b class="panTiltLimitWindow" style={props.limitOverlayStyle} />
              <i
                style={{
                  left: `${(props.controls.panValue / 65535) * 100}%`,
                  top: `${100 - (props.controls.tiltValue / 65535) * 100}%`,
                }}
              />
            </div>
            <div class="axisSliderRack" aria-label="Pan tilt axis sliders">
              <label class="axisSlider">
                <span>Pan</span>
                <input
                  type="range"
                  min="0"
                  max="65535"
                  value={props.controls.panValue}
                  aria-label="Pan value"
                  onInput={(event) => props.onSetValues(Number(event.currentTarget.value), props.controls.tiltValue)}
                />
                <small>{props.formatShortDmxPercent(props.controls.panValue)}</small>
              </label>
              <label class="axisSlider">
                <span>Tilt</span>
                <input
                  type="range"
                  min="0"
                  max="65535"
                  value={props.controls.tiltValue}
                  aria-label="Tilt value"
                  onInput={(event) => props.onSetValues(props.controls.panValue, Number(event.currentTarget.value))}
                />
                <small>{props.formatShortDmxPercent(props.controls.tiltValue)}</small>
              </label>
            </div>
          </div>
        </div>
        <div class="positionToolDeck">
          <div class="positionToolTabs" role="tablist" aria-label="Position" aria-orientation="horizontal">
            <button
              id="position-tool-tab-position"
              type="button"
              role="tab"
              class={activePositionToolTab() === "position" ? "active" : ""}
              aria-selected={activePositionToolTab() === "position"}
              aria-controls="position-tool-pane-position"
              tabIndex={activePositionToolTab() === "position" ? 0 : -1}
              onClick={() => setPositionToolTab("position")}
              onKeyDown={handlePositionToolTabKeyDown}
            >
              Position
            </button>
            <button
              id="position-tool-tab-limits"
              type="button"
              role="tab"
              class={activePositionToolTab() === "limits" ? "active" : ""}
              aria-selected={activePositionToolTab() === "limits"}
              aria-controls="position-tool-pane-limits"
              tabIndex={activePositionToolTab() === "limits" ? 0 : -1}
              disabled={!props.canEditLimits}
              onClick={() => setPositionToolTab("limits")}
              onKeyDown={handlePositionToolTabKeyDown}
            >
              Movement Limits
            </button>
          </div>
          <Show when={activePositionToolTab() === "position"}>
            <div
              id="position-tool-pane-position"
              class="positionToolPane positionToolPanePosition"
              role="tabpanel"
              aria-labelledby="position-tool-tab-position"
              tabIndex={0}
            >
              <div class="visualNumberGrid positionDirectGrid">
                <label>
                  Pan %
                  <input
                    type="number"
                    min="0"
                    max="100"
                    step="0.1"
                    value={props.dmxValueToPercent(props.controls.panValue)}
                    onInput={(event) => props.onSetPercent("pan", Number(event.currentTarget.value))}
                  />
                </label>
                <label>
                  Tilt %
                  <input
                    type="number"
                    min="0"
                    max="100"
                    step="0.1"
                    value={props.dmxValueToPercent(props.controls.tiltValue)}
                    onInput={(event) => props.onSetPercent("tilt", Number(event.currentTarget.value))}
                  />
                </label>
                <label>
                  Nudge
                  <select
                    value={props.nudgeAmount}
                    onChange={(event) => props.onSetNudgeAmount(Number(event.currentTarget.value))}
                  >
                    <For each={props.nudgeSteps}>
                      {(step) => <option value={step.value}>{step.label}</option>}
                    </For>
                  </select>
                </label>
              </div>
              <div class="positionQuickGrid">
                <div class="positionNudgeGrid">
                  <span />
                  <button onClick={() => props.onNudge(0, props.nudgeAmount)}>Tilt +</button>
                  <span />
                  <button onClick={() => props.onNudge(-props.nudgeAmount, 0)}>Pan -</button>
                  <button class="primary" onClick={props.onCenter}>Center</button>
                  <button onClick={() => props.onNudge(props.nudgeAmount, 0)}>Pan +</button>
                  <span />
                  <button onClick={() => props.onNudge(0, -props.nudgeAmount)}>Tilt -</button>
                  <span />
                </div>
                <div class="positionTargetGrid" aria-label="Pan tilt target points">
                  <For each={props.targetPoints}>
                    {(point) => (
                      <button
                        title={`Pan ${Math.round(point.pan * 100)}% / Tilt ${Math.round(point.tilt * 100)}%`}
                        onClick={() => props.onSetTarget(point.pan, point.tilt)}
                      >
                        <span class="positionTargetMini">
                          <i
                            style={{
                              left: `${point.pan * 100}%`,
                              top: `${100 - point.tilt * 100}%`,
                            }}
                          />
                        </span>
                        <small>{point.label}</small>
                      </button>
                    )}
                  </For>
                </div>
              </div>
              <div class="positionTransformRow">
                <button onClick={() => props.onMirrorAxis("pan")}>Mirror Pan</button>
                <button onClick={() => props.onMirrorAxis("tilt")}>Mirror Tilt</button>
                <button onClick={() => props.onMirrorAxis("both")}>Opposite</button>
              </div>
              <div class="positionFavoritePanel">
                <div class="swatchHeader">
                  <strong>Position Favorites</strong>
                  <div class="miniButtonRow">
                    <input
                      class="positionFavoriteLabelInput"
                      type="text"
                      maxLength={16}
                      value={props.favoriteLabel}
                      placeholder="Name"
                      aria-label="Position favorite label"
                      onInput={(event) => props.onSetFavoriteLabel(event.currentTarget.value)}
                      onKeyDown={(event) => {
                        if (event.key === "Enter") {
                          event.preventDefault();
                          props.onAddFavorite();
                        }
                      }}
                    />
                    <button aria-label="Save" title="Position Favorites" onClick={props.onAddFavorite}>+</button>
                    <button onClick={props.onResetFavorites}>Reset</button>
                  </div>
                </div>
                <div class="positionFavoriteGrid">
                  <For each={props.favorites}>
                    {(favorite) => (
                      <button
                        class={favoriteMatchesCurrent(favorite) ? "positionFavorite active" : "positionFavorite"}
                        title={`${favorite.label}: Pan ${props.formatShortDmxPercent(favorite.pan)} / Tilt ${props.formatShortDmxPercent(favorite.tilt)}${favoriteMatchesCurrent(favorite) ? " / current" : ""}. Shift-click removes.`}
                        onClick={(event) => {
                          if (event.shiftKey) {
                            props.onRemoveFavorite(favorite.id);
                          } else {
                            props.onApplyFavorite(favorite.pan, favorite.tilt);
                          }
                        }}
                      >
                        <span class="positionFavoriteMap">
                          <i
                            style={{
                              left: `${(favorite.pan / 65_535) * 100}%`,
                              top: `${100 - (favorite.tilt / 65_535) * 100}%`,
                            }}
                          />
                        </span>
                        <strong>{favorite.label}</strong>
                        <small>
                          {props.formatShortDmxPercent(favorite.pan)} / {props.formatShortDmxPercent(favorite.tilt)}
                          {favoriteMatchesCurrent(favorite) ? " / Current" : ""}
                        </small>
                      </button>
                    )}
                  </For>
                </div>
              </div>
            </div>
          </Show>
          <Show when={activePositionToolTab() === "limits"}>
            <div
              id="position-tool-pane-limits"
              class="positionToolPane positionToolPaneLimits"
              role="tabpanel"
              aria-labelledby="position-tool-tab-limits"
              tabIndex={0}
            >
              <div class="controlLimitPanel">
                <div class="limitEditorHeader">
                  <strong>Movement Limits</strong>
                  <span>
                    Pan {props.formatDmxPercent(props.normalizedLimits.pan_min)} -{" "}
                    {props.formatDmxPercent(props.normalizedLimits.pan_max)} / Tilt{" "}
                    {props.formatDmxPercent(props.normalizedLimits.tilt_min)} -{" "}
                    {props.formatDmxPercent(props.normalizedLimits.tilt_max)}
                  </span>
                </div>
                <div class="controlMovementLimitEditor">
                  <div
                    class={props.movementLimitDragging ? "movementLimitMap controlMovementLimitMap dragging" : "movementLimitMap controlMovementLimitMap"}
                    aria-label="Pan tilt movement limits"
                    role="group"
                    tabIndex={0}
                    title="Arrow keys move limits, PageUp/+ expands, PageDown/- shrinks, Shift coarse, Alt fine, Home reset."
                    onPointerDown={props.onMovementLimitPointerDown}
                    onPointerMove={props.onMovementLimitPointerMove}
                    onPointerUp={props.onMovementLimitPointerEnd}
                    onPointerCancel={props.onMovementLimitPointerEnd}
                    onKeyDown={handleMovementLimitKeyDown}
                  >
                    <i style={props.limitWindowStyle} />
                  </div>
                  <div class="controlMovementLimitFields">
                    <label>
                      Pan Min
                      <input
                        type="number"
                        min="0"
                        max="65535"
                        value={props.limitsDraft.pan_min}
                        onInput={(event) => props.onUpdateLimit("pan_min", Number(event.currentTarget.value))}
                      />
                    </label>
                    <label>
                      Pan Max
                      <input
                        type="number"
                        min="0"
                        max="65535"
                        value={props.limitsDraft.pan_max}
                        onInput={(event) => props.onUpdateLimit("pan_max", Number(event.currentTarget.value))}
                      />
                    </label>
                    <label>
                      Tilt Min
                      <input
                        type="number"
                        min="0"
                        max="65535"
                        value={props.limitsDraft.tilt_min}
                        onInput={(event) => props.onUpdateLimit("tilt_min", Number(event.currentTarget.value))}
                      />
                    </label>
                    <label>
                      Tilt Max
                      <input
                        type="number"
                        min="0"
                        max="65535"
                        value={props.limitsDraft.tilt_max}
                        onInput={(event) => props.onUpdateLimit("tilt_max", Number(event.currentTarget.value))}
                      />
                    </label>
                  </div>
                </div>
                <div class="limitToggleRow controlLimitToggleRow">
                  <label>
                    <input
                      type="checkbox"
                      checked={props.limitsDraft.invert_pan}
                      onChange={(event) => props.onUpdateLimitToggle("invert_pan", event.currentTarget.checked)}
                    />
                    Invert Pan
                  </label>
                  <label>
                    <input
                      type="checkbox"
                      checked={props.limitsDraft.invert_tilt}
                      onChange={(event) => props.onUpdateLimitToggle("invert_tilt", event.currentTarget.checked)}
                    />
                    Invert Tilt
                  </label>
                  <label>
                    <input
                      type="checkbox"
                      checked={props.limitsDraft.swap_pan_tilt}
                      onChange={(event) => props.onUpdateLimitToggle("swap_pan_tilt", event.currentTarget.checked)}
                    />
                    Swap
                  </label>
                </div>
                <div class="presetRow controlLimitActions">
                  <button onClick={props.onResetMovementLimits}>Reset Movement</button>
                  <button class="primary" onClick={props.onApplyLimits}>{props.applyLimitsLabel}</button>
                </div>
              </div>
            </div>
          </Show>
        </div>
      </div>
    </div>
  );
}
