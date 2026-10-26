import { createEffect, createSignal, createUniqueId, For, on, Show } from "solid-js";
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
  geometryRows: ProfileGeometryRow[];
  unresolvedGeometryReferences: string[];
  functionLabel: (fn: ChannelFunctionSummary) => string;
  functionRangeLabel: (fn: ChannelFunctionSummary) => string;
  functionDetail: (fn: ChannelFunctionSummary) => string;
  onSelectedMode: (modeName: string) => void;
}

const detailTabs = [
  { id: "dmx", label: "DMX" },
  { id: "functions", label: "Functions" },
  { id: "geometry", label: "Geometry" },
] as const;
type DetailTab = typeof detailTabs[number]["id"];
const functionPageSize = 12;

export function LoadedProfileSummaryPanel(props: LoadedProfileSummaryPanelProps) {
  const id = createUniqueId();
  const [activeTab, setActiveTab] = createSignal<DetailTab>("dmx");
  const [functionPage, setFunctionPage] = createSignal(0);
  const functionPageCount = () => Math.max(1, Math.ceil(props.functionEntries.length / functionPageSize));
  const visibleFunctions = () => props.functionEntries.slice(
    functionPage() * functionPageSize,
    (functionPage() + 1) * functionPageSize,
  );
  createEffect(on([() => props.profile, () => props.selectedMode], () => setFunctionPage(0)));

  const selectTabWithKeyboard = (event: KeyboardEvent & { currentTarget: HTMLButtonElement }) => {
    const index = detailTabs.findIndex((tab) => tab.id === activeTab());
    const next = event.key === "ArrowRight" ? (index + 1) % detailTabs.length
      : event.key === "ArrowLeft" ? (index + detailTabs.length - 1) % detailTabs.length
      : event.key === "Home" ? 0 : event.key === "End" ? detailTabs.length - 1 : null;
    if (next === null) return;
    event.preventDefault();
    event.stopPropagation();
    setActiveTab(detailTabs[next].id);
    event.currentTarget.parentElement?.querySelector<HTMLButtonElement>(
      `[data-loaded-profile-tab="${detailTabs[next].id}"]`,
    )?.focus();
  };

  return (
    <section class="loadedProfileSummaryPanel">
      <header class="loadedProfileHeader">
        <strong data-no-localize title={`${props.profile.manufacturer} ${props.profile.name}`}>
          {props.profile.manufacturer} {props.profile.name}
        </strong>
        <span>{props.profile.dmx_modes.length} mode(s) · {props.profile.geometries.length} geometry node(s)</span>
      </header>
      <Show when={props.profile.warnings.length > 0}>
        <div class="profileWarningBanner">
          <strong>{props.profile.warnings.length} warning{props.profile.warnings.length === 1 ? "" : "s"}</strong>
          <span>Review before patching this profile.</span>
        </div>
      </Show>
      <label class="loadedProfileModeField">
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
            <div class="loadedProfileTabs" role="tablist" aria-label="Profile details">
              <For each={detailTabs}>
                {(tab) => (
                  <button
                    type="button"
                    role="tab"
                    id={`${id}-tab-${tab.id}`}
                    aria-controls={`${id}-panel-${tab.id}`}
                    aria-selected={activeTab() === tab.id}
                    tabIndex={activeTab() === tab.id ? 0 : -1}
                    class={activeTab() === tab.id ? "active" : ""}
                    data-loaded-profile-tab={tab.id}
                    onClick={() => setActiveTab(tab.id)}
                    onKeyDown={selectTabWithKeyboard}
                  >
                    {tab.label}
                  </button>
                )}
              </For>
            </div>
            <div class="loadedProfileDetailGrid">
              <section
                class="loadedProfileTabPanel"
                role="tabpanel"
                id={`${id}-panel-dmx`}
                aria-labelledby={`${id}-tab-dmx`}
                data-loaded-profile-panel="dmx"
                hidden={activeTab() !== "dmx"}
                tabIndex={0}
              >
                <Show when={activeTab() === "dmx"}>
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
                </Show>
              </section>
              <section
                class="loadedProfileTabPanel"
                role="tabpanel"
                id={`${id}-panel-functions`}
                aria-labelledby={`${id}-tab-functions`}
                data-loaded-profile-panel="functions"
                hidden={activeTab() !== "functions"}
                tabIndex={0}
              >
                <Show when={activeTab() === "functions"}>
                  <div class="profileFunctionSummary">
                    <div class="customProfilePreviewHeader">
                      <strong>Channel Functions</strong>
                      <span>{props.functionEntries.length} range(s)</span>
                    </div>
                    <Show when={props.functionEntries.length > 0} fallback={<p class="empty">No channel functions.</p>}>
                      <div class="profileFunctionList">
                        <For each={visibleFunctions()}>
                          {(entry) => (
                            <div class="profileFunctionRow">
                              <strong title={`${entry.control.attribute} / ${entry.control.channel_name}`}>
                                {entry.control.attribute}
                              </strong>
                              <span>{props.functionRangeLabel(entry.fn)}</span>
                              <small>
                                <b>{props.functionLabel(entry.fn)}</b>
                                <b>{props.functionDetail(entry.fn)}</b>
                              </small>
                            </div>
                          )}
                        </For>
                      </div>
                      <Show when={functionPageCount() > 1}>
                        <div class="loadedProfileFunctionPaging">
                          <button type="button" data-loaded-profile-page="previous" disabled={functionPage() === 0}
                            onClick={() => setFunctionPage((page) => Math.max(0, page - 1))}>Previous</button>
                          <span data-no-localize aria-live="polite">{functionPage() + 1} / {functionPageCount()}</span>
                          <button type="button" data-loaded-profile-page="next" disabled={functionPage() + 1 >= functionPageCount()}
                            onClick={() => setFunctionPage((page) => Math.min(functionPageCount() - 1, page + 1))}>Next</button>
                        </div>
                      </Show>
                    </Show>
                  </div>
                </Show>
              </section>
              <section
                class="loadedProfileTabPanel"
                role="tabpanel"
                id={`${id}-panel-geometry`}
                aria-labelledby={`${id}-tab-geometry`}
                data-loaded-profile-panel="geometry"
                hidden={activeTab() !== "geometry"}
                tabIndex={0}
              >
                <Show when={activeTab() === "geometry"}>
                  <div class="profileGeometrySummary">
                    <div class="customProfilePreviewHeader">
                      <strong>Geometry</strong>
                      <span>{props.geometryRows.length} node(s)</span>
                    </div>
                    <Show when={props.geometryRows.length > 0} fallback={<p class="empty">No geometry nodes.</p>}>
                      <div class="profileGeometryList" aria-label="Selected profile geometry">
                        <For each={props.geometryRows}>
                          {(row) => (
                            <details class="profileGeometryRow">
                              <summary>
                                <strong data-no-localize style={{ "padding-left": `${Math.min(row.depth, 8) * 12}px` }}
                                  title={row.geometry.name}>{row.geometry.name}</strong>
                                <span>{row.geometry.kind}</span>
                                <small>{row.controlCount} ctrl</small>
                              </summary>
                              <div class="profileGeometryDetails">
                                <strong data-no-localize>{row.geometry.name}</strong>
                                <small>{row.geometry.parent ? `parent ${row.geometry.parent}` : "root"}</small>
                                <small class="profileGeometryPosition">
                                  <Show when={row.geometry.model_file || row.geometry.model_name || row.geometry.model_primitive}>
                                    <b class={`profileGeometryAsset ${row.geometry.model_file || row.geometry.model_primitive ? "available" : "missing"}`}>
                                      {row.geometry.model_file ? "mesh" : row.geometry.model_primitive ? "primitive" : "model"}
                                    </b>
                                  </Show>
                                  <span>{row.positionLabel || "no transform detail"}</span>
                                </small>
                              </div>
                            </details>
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
                </Show>
              </section>
            </div>
          </>
        )}
      </Show>
    </section>
  );
}
