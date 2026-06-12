import { For, Show } from "solid-js";
import type { JSX } from "solid-js";
import type { AttributeControl, ChannelFunctionSummary } from "../types";

export interface OpticsControlEntry {
  control: AttributeControl;
  value: number;
  role: string;
  activeFunction: ChannelFunctionSummary | undefined;
}

interface OpticsPresetButton {
  label: string;
  value: number;
}

type ChannelFunction = NonNullable<AttributeControl["functions"]>[number];

interface OpticsControlPanelProps {
  title: string;
  count: number;
  targetLabel: string;
  entries: OpticsControlEntry[];
  formatShortDmxPercent: (value: number) => string;
  clampDmxValue: (value: number) => number;
  previewClass: (role: string) => string;
  previewStyle: (entry: OpticsControlEntry) => JSX.CSSProperties;
  presetButtons: (entry: OpticsControlEntry) => OpticsPresetButton[];
  sortedFunctions: (control: AttributeControl) => ChannelFunction[];
  functionContainsValue: (fn: ChannelFunction, value: number) => boolean;
  functionBandStyle: (control: AttributeControl, fn: ChannelFunction) => JSX.CSSProperties;
  functionLabel: (fn: ChannelFunction) => string;
  functionRangeLabel: (fn: ChannelFunction) => string;
  functionDetail: (fn: ChannelFunction) => string;
  onPointerValue: (event: PointerEvent, control: AttributeControl) => void;
  onKeyValue: (event: KeyboardEvent, control: AttributeControl, value: number) => void;
  onSetValue: (control: AttributeControl, value: number) => void;
  onApplyFunction: (control: AttributeControl, fn: ChannelFunction) => void;
}

export function OpticsControlPanel(props: OpticsControlPanelProps) {
  const valueToPercent = (value: number) => Math.round((props.clampDmxValue(value) / 65_535) * 1000) / 10;
  const percentToValue = (value: number) => {
    const clamped = Math.min(100, Math.max(0, Number.isFinite(value) ? value : 0));
    return props.clampDmxValue((clamped / 100) * 65_535);
  };

  return (
    <div class="visualControlPanel opticsPanel">
      <div class="visualControlHeader">
        <div>
          <strong>{props.title}</strong>
          <span>{props.count} attribute(s)</span>
        </div>
        <span>{props.targetLabel}</span>
      </div>
      <div class="opticsControlGrid">
        <For each={props.entries}>
          {(entry) => (
            <div class="opticsControlCard">
              <div class="opticsControlTitle">
                <strong>{entry.role}</strong>
                <span>{props.formatShortDmxPercent(entry.value)}</span>
              </div>
              <div
                class={`opticsBeamPreview ${props.previewClass(entry.role)}`}
                style={props.previewStyle(entry)}
                role="slider"
                aria-label={`${entry.control.attribute} visual value`}
                aria-valuemin="0"
                aria-valuemax="65535"
                aria-valuenow={props.clampDmxValue(entry.value)}
                tabIndex={0}
                onPointerDown={(event) => {
                  event.currentTarget.setPointerCapture(event.pointerId);
                  props.onPointerValue(event, entry.control);
                }}
                onPointerMove={(event) => {
                  if (event.buttons === 1) {
                    props.onPointerValue(event, entry.control);
                  }
                }}
                onPointerUp={(event) => event.currentTarget.releasePointerCapture(event.pointerId)}
                onPointerCancel={(event) => event.currentTarget.releasePointerCapture(event.pointerId)}
                onKeyDown={(event) => props.onKeyValue(event, entry.control, entry.value)}
              >
                <b class="opticsBeamHalo" />
                <b class="opticsBeamCore" />
                <b class="opticsPrismGuide" />
                <span class="opticsBeamScale">
                  <i />
                </span>
              </div>
              <input
                type="range"
                min="0"
                max="65535"
                value={entry.value}
                aria-label={`${entry.control.attribute} value`}
                onInput={(event) => props.onSetValue(entry.control, Number(event.currentTarget.value))}
              />
              <div class="visualNumberGrid opticsDirectGrid">
                <label>
                  %
                  <input
                    type="number"
                    min="0"
                    max="100"
                    step="0.1"
                    value={valueToPercent(entry.value)}
                    onInput={(event) => props.onSetValue(entry.control, percentToValue(Number(event.currentTarget.value)))}
                  />
                </label>
                <label>
                  DMX
                  <input
                    type="number"
                    min="0"
                    max="65535"
                    value={props.clampDmxValue(entry.value)}
                    onInput={(event) => props.onSetValue(entry.control, props.clampDmxValue(Number(event.currentTarget.value)))}
                  />
                </label>
              </div>
              <Show when={props.sortedFunctions(entry.control).length > 0}>
                <div class="opticsRangeMap" role="group" aria-label={`${entry.control.attribute} function ranges`}>
                  <For each={props.sortedFunctions(entry.control)}>
                    {(fn) => (
                      <button
                        type="button"
                        class={props.functionContainsValue(fn, entry.value) ? "active" : ""}
                        style={props.functionBandStyle(entry.control, fn)}
                        title={`${props.functionLabel(fn)} / ${props.functionRangeLabel(fn)} / ${props.functionDetail(fn)}`}
                        aria-label={`Apply ${entry.control.attribute} ${props.functionLabel(fn)}`}
                        onClick={() => props.onApplyFunction(entry.control, fn)}
                      />
                    )}
                  </For>
                  <span
                    class="opticsRangeMarker"
                    style={{ left: `${(props.clampDmxValue(entry.value) / 65_535) * 100}%` }}
                  />
                </div>
              </Show>
              <div class="opticsQuickRow">
                <For each={props.presetButtons(entry)}>
                  {(preset) => (
                    <button onClick={() => props.onSetValue(entry.control, preset.value)}>
                      {preset.label}
                    </button>
                  )}
                </For>
              </div>
              <div class="opticsMetaRow">
                <span>{entry.control.attribute}</span>
                <Show when={entry.activeFunction}>
                  {(fn) => <b>{props.functionLabel(fn())}</b>}
                </Show>
              </div>
              <Show when={props.sortedFunctions(entry.control).length > 0}>
                <div class="opticsFunctionChips">
                  <For each={props.sortedFunctions(entry.control).slice(0, 5)}>
                    {(fn) => (
                      <button
                        class={props.functionContainsValue(fn, entry.value) ? "active" : ""}
                        title={`${props.functionLabel(fn)} / ${props.functionRangeLabel(fn)} / ${props.functionDetail(fn)}`}
                        onClick={() => props.onApplyFunction(entry.control, fn)}
                      >
                        {props.functionLabel(fn)}
                      </button>
                    )}
                  </For>
                </div>
              </Show>
            </div>
          )}
        </For>
      </div>
    </div>
  );
}
