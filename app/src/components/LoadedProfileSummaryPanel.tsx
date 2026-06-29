import { For, Show } from "solid-js";
import type { ProfileGeometryRow } from "../customFixtureProfile";
import type { AttributeControl, ChannelFunctionSummary, DmxModeSummary, FixtureProfileSummary } from "../types";

export interface LoadedProfileDmxCell {
  channel: number;
  control: AttributeControl;
  category: string;
  bitLabel: string;
}

export interface LoadedProfileFunctionEntry {
  control: AttributeControl;
  fn: ChannelFunctionSummary;
  category: string;
}

interface LoadedProfileSummaryPanelProps {
  profile: FixtureProfileSummary;
  selectedMode: string;
  selectedModeSummary?: DmxModeSummary;
  selectedFootprint: number;
  dmxCells: LoadedProfileDmxCell[];
  functionEntries: LoadedProfileFunctionEntry[];
  visibleFunctionEntries: LoadedProfileFunctionEntry[];
  geometryRows: ProfileGeometryRow[];
  unresolvedGeometryReferences: string[];
  functionLabel: (fn: ChannelFunctionSummary) => string;
  functionRangeLabel: (fn: ChannelFunctionSummary) => string;
  functionDetail: (fn: ChannelFunctionSummary) => string;
  onSelectedMode: (modeName: string) => void;
}

export function LoadedProfileSummaryPanel(props: LoadedProfileSummaryPanelProps) {
  return (
    <>
      <strong>{props.profile.manufacturer} {props.profile.name}</strong>
      <span>{props.profile.dmx_modes.length} mode(s), {props.profile.geometries.length} geometry node(s)</span>
      <Show when={props.profile.warnings.length > 0}>
        <div class="profileWarningBanner">
          <strong>{props.profile.warnings.length} warning{props.profile.warnings.length === 1 ? "" : "s"}</strong>
          <span>Review before patching this profile.</span>
        </div>
      </Show>
      <label>
        Mode
        <select value={props.selectedMode} onInput={(event) => props.onSelectedMode(event.currentTarget.value)}>
          <For each={props.profile.dmx_modes}>
            {(mode) => <option value={mode.name}>{mode.name}</option>}
          </For>
        </select>
      </label>
      <Show when={props.selectedModeSummary}>
        {(mode) => (
          <>
            <div class="profileDmxSummary">
              <div class="customProfilePreviewHeader">
                <strong>Mode DMX Map</strong>
                <span>{props.selectedFootprint} ch / {mode().controls.length} control(s)</span>
              </div>
              <Show when={props.dmxCells.length > 0} fallback={<p class="empty">No DMX controls.</p>}>
                <div class="profileDmxGrid" aria-label="Selected mode DMX map">
                  <For each={props.dmxCells}>
                    {(cell) => (
                      <div class={`profileDmxCell ${cell.category}`}>
                        <small>CH {cell.channel}</small>
                        <strong title={`${cell.control.attribute} / ${cell.control.channel_name}`}>
                          {cell.control.attribute}
                        </strong>
                        <span>{cell.bitLabel}</span>
                      </div>
                    )}
                  </For>
                </div>
              </Show>
            </div>
            <Show when={props.functionEntries.length > 0}>
              <div class="profileFunctionSummary">
                <div class="customProfilePreviewHeader">
                  <strong>Channel Functions</strong>
                  <span>{props.functionEntries.length} range(s)</span>
                </div>
                <div class="profileFunctionList">
                  <For each={props.visibleFunctionEntries}>
                    {(entry) => (
                      <div class="profileFunctionRow">
                        <strong title={`${entry.control.attribute} / ${entry.control.channel_name}`}>
                          {entry.control.attribute}
                        </strong>
                        <span>{props.functionRangeLabel(entry.fn)}</span>
                        <small>
                          <b title={props.functionLabel(entry.fn)}>{props.functionLabel(entry.fn)}</b>
                          <b title={props.functionDetail(entry.fn)}>{props.functionDetail(entry.fn)}</b>
                        </small>
                      </div>
                    )}
                  </For>
                  <Show when={props.functionEntries.length - props.visibleFunctionEntries.length}>
                    {(remaining) => (
                      <span class="profileFunctionMore">+ {remaining()} more function(s)</span>
                    )}
                  </Show>
                </div>
              </div>
            </Show>
            <div class="profileGeometrySummary">
              <div class="customProfilePreviewHeader">
                <strong>Geometry</strong>
                <span>{props.geometryRows.length} node(s)</span>
              </div>
              <Show when={props.geometryRows.length > 0} fallback={<p class="empty">No geometry nodes.</p>}>
                <div class="profileGeometryList" aria-label="Selected profile geometry">
                  <For each={props.geometryRows}>
                    {(row) => (
                      <div class="profileGeometryRow">
                        <strong
                          style={{ "padding-left": `${Math.min(row.depth, 8) * 12}px` }}
                          title={row.geometry.name}
                        >
                          {row.geometry.name}
                        </strong>
                        <span>{row.geometry.kind}</span>
                        <small title={row.geometry.parent ?? "root"}>
                          {row.geometry.parent ? `parent ${row.geometry.parent}` : "root"}
                        </small>
                        <small>{row.controlCount} ctrl</small>
                        <small class="profileGeometryPosition" title={row.positionLabel}>
                          <Show when={row.geometry.model_file || row.geometry.model_name || row.geometry.model_primitive}>
                            <b
                              class={`profileGeometryAsset ${
                                row.geometry.model_file || row.geometry.model_primitive ? "available" : "missing"
                              }`}
                            >
                              {row.geometry.model_file ? "mesh" : row.geometry.model_primitive ? "primitive" : "model"}
                            </b>
                          </Show>
                          <span>{row.positionLabel || "no transform detail"}</span>
                        </small>
                      </div>
                    )}
                  </For>
                </div>
              </Show>
              <Show when={props.unresolvedGeometryReferences.length > 0}>
                <ul class="warnings">
                  <For each={props.unresolvedGeometryReferences}>
                    {(reference) => <li>Unresolved geometry reference: {reference}</li>}
                  </For>
                </ul>
              </Show>
            </div>
          </>
        )}
      </Show>
    </>
  );
}
