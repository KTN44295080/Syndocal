import { For, type JSX } from "solid-js";
import type { DmxUniversePreview } from "../types";

interface DmxRawCell {
  channel: number;
  value: number;
}

interface DmxRawMonitorProps {
  previews: DmxUniversePreview[];
  activeUniverse: number;
  activeCount: number;
  cells: DmxRawCell[];
  children?: JSX.Element;
  onUniverseChange: (universe: number) => void;
}

export function DmxRawMonitor(props: DmxRawMonitorProps) {
  return (
    <div class="rawMonitor">
      <div class="panelHeader">
        <h2>DMX Raw</h2>
        <div class="panelHeaderActions">
          <select value={props.activeUniverse} onInput={(event) => props.onUniverseChange(Number(event.currentTarget.value))}>
            <For each={props.previews}>{(preview) => <option value={preview.universe}>U{preview.universe}</option>}</For>
          </select>
          <span>{props.activeCount} active</span>
        </div>
      </div>
      <div class="rawGrid">
        <For each={props.cells}>
          {(cell) => (
            <div class={cell.value > 0 ? "rawCell active" : "rawCell"}>
              <span>{cell.channel}</span>
              <strong>{cell.value}</strong>
            </div>
          )}
        </For>
      </div>
      {props.children}
    </div>
  );
}
