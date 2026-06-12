import { For, Show } from "solid-js";

export interface CueCapturePreviewFixture {
  id: number;
  label: string;
  dmxLabel: string;
  groupLabel: string;
  x: number;
  z: number;
  color: string;
  intensity: number;
  attributes: number;
}

export interface CueCapturePreviewVideoRow {
  label: string;
  meta: string;
}

export interface CueCapturePreviewModel {
  scopeLabel: string;
  scopeDetail: string;
  fixtures: CueCapturePreviewFixture[];
  videoRows: CueCapturePreviewVideoRow[];
  fixtureAttributeCount: number;
  layerCount: number;
  outputCount: number;
  nodeGraphCount: number;
}

interface CueCapturePreviewPanelProps {
  preview: CueCapturePreviewModel;
  invalid: boolean;
  stageViewBoxSize: number;
  stageOrigin: { x: number; z: number };
  selectedFixtureId: number | null;
  onSelectFixture: (fixtureId: number) => void;
}

export function CueCapturePreviewPanel(props: CueCapturePreviewPanelProps) {
  return (
    <div class="cueCapturePreview">
      <div class="cuePreviewHeader">
        <div>
          <strong>{props.preview.scopeLabel}</strong>
          <span>{props.preview.scopeDetail}</span>
        </div>
        <span>{props.invalid ? "Invalid" : "Ready"}</span>
      </div>
      <div class="cuePreviewStats">
        <span>
          <small>Fixtures</small>
          <strong>{props.preview.fixtures.length}</strong>
        </span>
        <span>
          <small>Attrs</small>
          <strong>{props.preview.fixtureAttributeCount}</strong>
        </span>
        <span>
          <small>Layers</small>
          <strong>{props.preview.layerCount}</strong>
        </span>
        <span>
          <small>Outputs</small>
          <strong>{props.preview.outputCount}</strong>
        </span>
        <span>
          <small>Graphs</small>
          <strong>{props.preview.nodeGraphCount}</strong>
        </span>
      </div>
      <div class="cuePreviewBody">
        <svg class="cuePreviewStage" viewBox={`0 0 ${props.stageViewBoxSize} ${props.stageViewBoxSize}`}>
          <rect class="cuePreviewFloor" x="0" y="0" width={props.stageViewBoxSize} height={props.stageViewBoxSize} />
          <line class="cuePreviewAxis" x1={props.stageOrigin.x} y1="0" x2={props.stageOrigin.x} y2={props.stageViewBoxSize} />
          <line class="cuePreviewAxis" x1="0" y1={props.stageOrigin.z} x2={props.stageViewBoxSize} y2={props.stageOrigin.z} />
          <For each={props.preview.fixtures.slice(0, 48)}>
            {(fixture) => (
              <g
                class={fixture.id === props.selectedFixtureId ? "cuePreviewFixture selected" : "cuePreviewFixture"}
                tabIndex={0}
                role="button"
                aria-label={`Select ${fixture.label}`}
                onClick={() => props.onSelectFixture(fixture.id)}
              >
                <circle
                  cx={fixture.x}
                  cy={fixture.z}
                  r={Math.max(1.3, 1.4 + fixture.intensity / 45)}
                  fill={fixture.color}
                  opacity={0.45 + Math.min(0.5, fixture.intensity / 200)}
                />
              </g>
            )}
          </For>
        </svg>
        <div class="cuePreviewLists">
          <div class="cuePreviewList">
            <strong>Fixtures</strong>
            <For each={props.preview.fixtures.slice(0, 5)}>
              {(fixture) => (
                <button
                  class={fixture.id === props.selectedFixtureId ? "active" : ""}
                  onClick={() => props.onSelectFixture(fixture.id)}
                >
                  <i style={{ background: fixture.color }} />
                  <span>{fixture.label}</span>
                  <small>
                    {fixture.dmxLabel} / {fixture.groupLabel} / {fixture.attributes} attr
                  </small>
                </button>
              )}
            </For>
            <Show when={props.preview.fixtures.length === 0}>
              <small>No lighting targets in this scope.</small>
            </Show>
          </div>
          <div class="cuePreviewList">
            <strong>Video</strong>
            <For each={props.preview.videoRows.slice(0, 11)}>
              {(row) => (
                <div class="cuePreviewVideoRow">
                  <b>{row.label}</b>
                  <small>{row.meta}</small>
                </div>
              )}
            </For>
            <Show when={props.preview.videoRows.length === 0}>
              <small>No video targets in this scope.</small>
            </Show>
          </div>
        </div>
      </div>
    </div>
  );
}
