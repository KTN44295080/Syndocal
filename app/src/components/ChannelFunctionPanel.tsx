import { For, Show } from "solid-js";
import type { JSX } from "solid-js";
import type { AttributeControl, ChannelFunctionSummary } from "../types";

export interface ChannelFunctionControlEntry {
  control: AttributeControl;
  functions: ChannelFunctionSummary[];
}

interface ChannelFunctionPanelProps {
  categoryLabel: string;
  entries: ChannelFunctionControlEntry[];
  currentValue: (control: AttributeControl) => number;
  clampDmxValue: (value: number) => number;
  functionContainsValue: (fn: ChannelFunctionSummary, value: number) => boolean;
  functionBandStyle: (control: AttributeControl, fn: ChannelFunctionSummary) => JSX.CSSProperties;
  functionLabel: (fn: ChannelFunctionSummary) => string;
  functionRangeLabel: (fn: ChannelFunctionSummary) => string;
  functionDetail: (fn: ChannelFunctionSummary) => string;
  functionSwatchColor: (control: AttributeControl, fn: ChannelFunctionSummary) => string | null;
  onApplyFunction: (control: AttributeControl, fn: ChannelFunctionSummary) => void;
}

export function ChannelFunctionPanel(props: ChannelFunctionPanelProps) {
  const activeFunction = (entry: ChannelFunctionControlEntry) => {
    const value = props.currentValue(entry.control);
    return entry.functions.find((fn) => props.functionContainsValue(fn, value));
  };
  const displayValue = (entry: ChannelFunctionControlEntry) => {
    const value = props.clampDmxValue(props.currentValue(entry.control));
    return entry.control.resolution === "SixteenBit" ? value : Math.round(value / 257);
  };
  const applyIndex = (entry: ChannelFunctionControlEntry, index: number) => {
    const count = entry.functions.length;
    if (count === 0) {
      return;
    }
    const wrappedIndex = ((index % count) + count) % count;
    props.onApplyFunction(entry.control, entry.functions[wrappedIndex]);
  };
  const handleFunctionKeyDown = (
    event: KeyboardEvent,
    entry: ChannelFunctionControlEntry,
    index: number,
  ) => {
    switch (event.key) {
      case "ArrowRight":
      case "ArrowDown":
        event.preventDefault();
        applyIndex(entry, index + 1);
        break;
      case "ArrowLeft":
      case "ArrowUp":
        event.preventDefault();
        applyIndex(entry, index - 1);
        break;
      case "Home":
        event.preventDefault();
        applyIndex(entry, 0);
        break;
      case "End":
        event.preventDefault();
        applyIndex(entry, entry.functions.length - 1);
        break;
    }
  };

  return (
    <details class="visualControlPanel channelFunctionPanel">
      <summary class="visualControlHeader">
        <span class="channelFunctionSummaryIdentity">
          <strong>GDTF Functions</strong>
          <span>{props.entries.length} attribute(s) with function ranges</span>
        </span>
        <span class="channelFunctionSummaryReadoutRegion">
          <span class="channelFunctionSummaryReadouts">
            <For each={props.entries}>
              {(entry) => {
                const currentFunction = () => activeFunction(entry);
                const range = () => {
                  const fn = currentFunction();
                  return fn ? props.functionRangeLabel(fn) : "—";
                };
                const detail = () => {
                  const fn = currentFunction();
                  return fn
                    ? `${props.functionLabel(fn)} / ${props.functionRangeLabel(fn)} / ${props.functionDetail(fn)}`
                    : "—";
                };
                return (
                  <span
                    class="channelFunctionSummaryChip"
                    title={`${entry.control.attribute} ${displayValue(entry)} / ${detail()}`}
                  >
                    <strong>{entry.control.attribute}</strong>
                    <output>{displayValue(entry)}</output>
                    <small>{range()}</small>
                  </span>
                );
              }}
            </For>
          </span>
          <span class="channelFunctionSummaryCategory">{props.categoryLabel}</span>
        </span>
      </summary>
      <div class="channelFunctionList">
        <For each={props.entries}>
          {(entry) => {
            const currentValue = () => props.currentValue(entry.control);
            const markerLeft = () => `${(props.clampDmxValue(currentValue()) / 65_535) * 100}%`;
            const currentFunction = () => activeFunction(entry);
            return (
              <div class="channelFunctionGroup">
                <div class="channelFunctionTitle">
                  <span>
                    <strong>{entry.control.attribute}</strong>
                    <Show when={currentFunction()}>
                      {(fn) => <b>{props.functionLabel(fn())}</b>}
                    </Show>
                    <Show when={!currentFunction()}>
                      <b class="inactive">No matched range</b>
                    </Show>
                  </span>
                  <small>{entry.control.channel_name}</small>
                </div>
                <div class="channelFunctionRangeStrip" aria-label={`${entry.control.attribute} function ranges`}>
                  <For each={entry.functions}>
                    {(fn) => (
                      <button
                        type="button"
                        class={props.functionContainsValue(fn, currentValue()) ? "active" : ""}
                        style={props.functionBandStyle(entry.control, fn)}
                        title={`${props.functionLabel(fn)} / ${props.functionRangeLabel(fn)} / ${props.functionDetail(fn)}`}
                        aria-label={`Apply ${entry.control.attribute} ${props.functionLabel(fn)}`}
                        onClick={() => props.onApplyFunction(entry.control, fn)}
                      />
                    )}
                  </For>
                  <span class="channelFunctionRangeMarker" style={{ left: markerLeft() }} />
                </div>
                <div class="channelFunctionGrid">
                  <For each={entry.functions}>
                    {(fn, index) => (
                      <button
                        type="button"
                        class={props.functionContainsValue(fn, currentValue()) ? "active" : ""}
                        title={`${props.functionLabel(fn)} / ${props.functionRangeLabel(fn)} / ${props.functionDetail(fn)}`}
                        aria-pressed={props.functionContainsValue(fn, currentValue())}
                        onClick={() => props.onApplyFunction(entry.control, fn)}
                        onKeyDown={(event) => handleFunctionKeyDown(event, entry, index())}
                      >
                        <strong>{props.functionLabel(fn)}</strong>
                        <span>{props.functionRangeLabel(fn)}</span>
                        <small>
                          <Show when={props.functionSwatchColor(entry.control, fn)}>
                            {(color) => <i class="channelFunctionSwatch" style={{ "background-color": color() }} />}
                          </Show>
                          {props.functionDetail(fn)}
                        </small>
                      </button>
                    )}
                  </For>
                </div>
              </div>
            );
          }}
        </For>
      </div>
    </details>
  );
}
