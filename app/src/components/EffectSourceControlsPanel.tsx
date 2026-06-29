import { For, Show } from "solid-js";
import type { EffectKind, LfoShape } from "../types";

type WaveStageDragMode = "origin" | "direction" | "videoTarget";
type WavePointerEvent = PointerEvent & { currentTarget: SVGElement };

interface StagePoint {
  x: number;
  z: number;
}

interface StageBounds {
  minX: number;
  maxX: number;
  minZ: number;
  maxZ: number;
}

interface WaveFixture {
  id: number;
  x: number;
  z: number;
  color: string;
}

interface WaveVideoSurface {
  id: number;
  x: number;
  z: number;
  width: number;
  height: number;
  rotationDeg: number;
  active: boolean;
}

interface WaveStageObject {
  id: number;
  label: string;
  kind: string;
  x: number;
  z: number;
  width: number;
  depth: number;
  rotationDeg: number;
  color: string;
}

interface EffectSourceControlsPanelProps {
  effectType: EffectKind;
  shape: LfoShape;
  periodMs: number;
  bpm: number;
  clockSyncBeats: number | null;
  stageViewBoxSize: number;
  originX: number;
  originY: number;
  originZ: number;
  directionX: number;
  directionY: number;
  directionZ: number;
  speed: number;
  wavelength: number;
  stageOrigin: StagePoint;
  stageBounds: StageBounds;
  originPoint: StagePoint;
  directionPoint: StagePoint;
  videoTargetPoint: StagePoint;
  directionIsRadial: boolean;
  radialRadius: number;
  fixtures: WaveFixture[];
  videoSurfaces: WaveVideoSurface[];
  stageObjects: WaveStageObject[];
  targetFixtureIds: ReadonlySet<number>;
  selectedFixtureId: number | null;
  selectedVideoOutputId: number | null;
  videoTargetMode: boolean;
  dragging: WaveStageDragMode | null;
  hasSelectedFixture: boolean;
  hasSelectedStageObject: boolean;
  onShape: (shape: LfoShape) => void;
  onPeriodMs: (periodMs: number) => void;
  onClockSyncBeats: (beats: number | null) => void;
  onUseStageCenter: () => void;
  onUseSelectedFixture: () => void;
  onUseSelectedStageObject: () => void;
  onDirectionPreset: (x: number, y: number, z: number) => void;
  onStagePointerDown: (event: WavePointerEvent, mode: WaveStageDragMode) => void;
  onStagePointerMove: (event: WavePointerEvent) => void;
  onStagePointerUp: (event: WavePointerEvent) => void;
  onPickVideoSurface: (surfaceId: number) => void;
  onOriginX: (value: number) => void;
  onOriginY: (value: number) => void;
  onOriginZ: (value: number) => void;
  onDirectionX: (value: number) => void;
  onDirectionY: (value: number) => void;
  onDirectionZ: (value: number) => void;
  onSpeed: (value: number) => void;
  onWavelength: (value: number) => void;
}

const clockSyncBeatPresets = [
  { label: "Free", beats: null },
  { label: "1/4", beats: 0.25 },
  { label: "1/2", beats: 0.5 },
  { label: "1", beats: 1 },
  { label: "2", beats: 2 },
  { label: "4", beats: 4 },
] as const;

const waveSpeedPresets = [
  { label: "Stop", speed: 0 },
  { label: "Slow", speed: 0.5 },
  { label: "1x", speed: 1 },
  { label: "2x", speed: 2 },
] as const;

const waveLengthPresets = [
  { label: "Tight", wavelength: 1 },
  { label: "Med", wavelength: 2 },
  { label: "Wide", wavelength: 4 },
  { label: "Huge", wavelength: 8 },
] as const;

const lfoShapePresets: { shape: LfoShape; label: string; detail: string; path: string }[] = [
  {
    shape: "Sine",
    label: "Sine",
    detail: "smooth",
    path: "M 2 9 C 8 1 14 1 20 9 C 26 17 32 17 38 9 C 42 4 45 4 46 6",
  },
  {
    shape: "Cosine",
    label: "Cos",
    detail: "offset",
    path: "M 2 3 C 8 1 14 1 20 9 C 26 17 32 17 38 9 C 42 4 45 4 46 12",
  },
  {
    shape: "Triangle",
    label: "Tri",
    detail: "linear",
    path: "M 2 16 L 12 2 L 24 16 L 36 2 L 46 16",
  },
  {
    shape: "Saw",
    label: "Saw",
    detail: "ramp",
    path: "M 2 16 L 13 2 L 13 16 L 25 2 L 25 16 L 37 2 L 37 16 L 46 5",
  },
  {
    shape: "Square",
    label: "Sq",
    detail: "step",
    path: "M 2 16 L 2 3 L 14 3 L 14 16 L 26 16 L 26 3 L 38 3 L 38 16 L 46 16",
  },
  {
    shape: "Random",
    label: "Rand",
    detail: "sample",
    path: "M 2 14 L 7 14 L 7 4 L 13 4 L 13 11 L 20 11 L 20 2 L 27 2 L 27 16 L 35 16 L 35 8 L 42 8 L 42 13 L 46 13",
  },
  {
    shape: "Perlin",
    label: "Perlin",
    detail: "noise",
    path: "M 2 13 C 7 10 9 4 14 6 C 18 8 20 15 26 12 C 31 10 31 3 37 5 C 42 7 42 13 46 11",
  },
];

const nearlyEqual = (a: number | null, b: number | null) =>
  a === null || b === null ? a === b : Math.abs(a - b) < 0.001;

export function EffectSourceControlsPanel(props: EffectSourceControlsPanelProps) {
  const beatPeriodMs = (beats: number) => {
    const bpm = Number.isFinite(props.bpm) && props.bpm > 0 ? props.bpm : 120;
    return Math.max(10, Math.round((60_000 / bpm) * beats));
  };

  return (
    <>
      <div class="split">
        <label>
          Shape
          <select value={props.shape} onInput={(event) => props.onShape(event.currentTarget.value as LfoShape)}>
            <option value="Sine">Sine</option>
            <option value="Cosine">Cosine</option>
            <option value="Triangle">Triangle</option>
            <option value="Saw">Saw</option>
            <option value="Square">Square</option>
            <option value="Random">Random</option>
            <option value="Perlin">Perlin</option>
          </select>
        </label>
      </div>
      <div class="effectShapePresetPanel">
        <div class="effectBpmPeriodHeader">
          <strong>Waveform</strong>
          <span>{props.shape}</span>
        </div>
        <div class="effectShapePresetGrid" aria-label="Effect waveform presets">
          <For each={lfoShapePresets}>
            {(preset) => (
              <button
                class={props.shape === preset.shape ? "active" : ""}
                onClick={() => props.onShape(preset.shape)}
                aria-pressed={props.shape === preset.shape}
              >
                <svg class="effectShapeMini" viewBox="0 0 48 18" aria-hidden="true">
                  <path d={preset.path} />
                </svg>
                <strong>{preset.label}</strong>
                <span>{preset.detail}</span>
              </button>
            )}
          </For>
        </div>
      </div>
      <Show when={props.effectType === "Lfo"}>
        <div class="effectBpmPeriodPanel">
          <div class="effectBpmPeriodHeader">
            <strong>LFO clock</strong>
            <span>{props.clockSyncBeats === null ? `${props.periodMs}ms free` : `${props.clockSyncBeats} beat sync`}</span>
          </div>
          <label>
            Period ms
            <input type="number" min="10" value={props.periodMs} onInput={(event) => props.onPeriodMs(Number(event.currentTarget.value))} />
          </label>
          <div class="effectBpmPeriodGrid" aria-label="LFO clock sync presets">
            <For each={clockSyncBeatPresets}>
              {(preset) => (
                <button
                  class={nearlyEqual(props.clockSyncBeats, preset.beats) ? "active" : ""}
                  onClick={() => props.onClockSyncBeats(preset.beats)}
                >
                  <strong>{preset.label}</strong>
                  <span>{preset.beats === null ? "manual ms" : `${beatPeriodMs(preset.beats)}ms`}</span>
                </button>
              )}
            </For>
          </div>
        </div>
      </Show>
      <Show when={props.effectType === "PositionWave"}>
        <div class="waveControls">
          <div class="waveStagePicker">
            <div class="waveStageHeader">
              <div>
                <strong>Stage Wave</strong>
                <span>
                  O {props.originX.toFixed(1)}, {props.originY.toFixed(1)}, {props.originZ.toFixed(1)} / D{" "}
                  {props.directionX.toFixed(1)}, {props.directionY.toFixed(1)}, {props.directionZ.toFixed(1)}
                </span>
              </div>
            </div>
            <div class="buttonRow">
              <button onClick={props.onUseStageCenter}>Stage Center</button>
              <button onClick={props.onUseSelectedFixture} disabled={!props.hasSelectedFixture}>
                Selected Fixture
              </button>
              <button onClick={props.onUseSelectedStageObject} disabled={!props.hasSelectedStageObject}>
                Stage Object
              </button>
              <button onClick={() => props.onDirectionPreset(1, 0, 0)}>X</button>
              <button onClick={() => props.onDirectionPreset(0, 0, 1)}>Z</button>
              <button onClick={() => props.onDirectionPreset(0, 0, 0)}>Radial</button>
            </div>
            <svg
              class="waveStageMap"
              viewBox={`0 0 ${props.stageViewBoxSize} ${props.stageViewBoxSize}`}
              onPointerDown={(event) =>
                props.onStagePointerDown(event, event.altKey ? "videoTarget" : event.shiftKey ? "direction" : "origin")
              }
              onPointerMove={props.onStagePointerMove}
              onPointerUp={props.onStagePointerUp}
              onPointerCancel={props.onStagePointerUp}
            >
              <defs>
                <pattern id="wave-stage-grid" width="5" height="5" patternUnits="userSpaceOnUse">
                  <path d="M 5 0 L 0 0 0 5" />
                </pattern>
              </defs>
              <rect class="stageFloor" x="0" y="0" width={props.stageViewBoxSize} height={props.stageViewBoxSize} />
              <rect class="stageGrid" x="0" y="0" width={props.stageViewBoxSize} height={props.stageViewBoxSize} />
              <For each={props.stageObjects}>
                {(object) => (
                  <g
                    class={`waveStageObject kind-${object.kind.toLowerCase()}`}
                    transform={`translate(${object.x} ${object.z}) rotate(${object.rotationDeg})`}
                  >
                    <rect x={-object.width / 2} y={-object.depth / 2} width={object.width} height={object.depth} fill={object.color} />
                    <line x1={-object.width / 2} y1="0" x2={object.width / 2} y2="0" />
                    <line x1="0" y1={-object.depth / 2} x2="0" y2={object.depth / 2} />
                    <text x={-object.width / 2 + 1} y={-object.depth / 2 - 1}>
                      {object.label}
                    </text>
                  </g>
                )}
              </For>
              <line class="waveGuideLine" x1={props.stageOrigin.x} y1="0" x2={props.stageOrigin.x} y2={props.stageViewBoxSize} />
              <line class="waveGuideLine" x1="0" y1={props.stageOrigin.z} x2={props.stageViewBoxSize} y2={props.stageOrigin.z} />
              <For each={props.videoSurfaces}>
                {(surface) => (
                  <g
                    class={[
                      "waveVideoSurface",
                      props.selectedVideoOutputId === surface.id ? "selected" : "",
                      surface.active ? "" : "inactive",
                      props.videoTargetMode ? "" : "inert",
                    ]
                      .filter(Boolean)
                      .join(" ")}
                    transform={`translate(${surface.x} ${surface.z}) rotate(${surface.rotationDeg})`}
                    onPointerDown={(event) => {
                      if (!props.videoTargetMode) {
                        return;
                      }
                      event.preventDefault();
                      event.stopPropagation();
                      props.onPickVideoSurface(surface.id);
                    }}
                  >
                    <rect x={-surface.width / 2} y={-surface.height / 2} width={surface.width} height={surface.height} />
                    <line x1={-surface.width / 2} y1="0" x2={surface.width / 2} y2="0" />
                    <line x1="0" y1={-surface.height / 2} x2="0" y2={surface.height / 2} />
                  </g>
                )}
              </For>
              <For each={props.fixtures}>
                {(fixture) => (
                  <circle
                    class={[
                      "waveStageFixture",
                      props.targetFixtureIds.has(fixture.id) ? "target" : "muted",
                      props.selectedFixtureId === fixture.id ? "selected" : "",
                    ]
                      .filter(Boolean)
                      .join(" ")}
                    cx={fixture.x}
                    cy={fixture.z}
                    r={props.targetFixtureIds.has(fixture.id) ? 1.8 : 1.35}
                    fill={fixture.color}
                  />
                )}
              </For>
              <Show when={props.directionIsRadial}>
                <circle class="waveGuideRing" cx={props.originPoint.x} cy={props.originPoint.z} r={props.radialRadius} />
              </Show>
              <Show when={!props.directionIsRadial}>
                <line
                  class="waveDirectionLine"
                  x1={props.originPoint.x}
                  y1={props.originPoint.z}
                  x2={props.directionPoint.x}
                  y2={props.directionPoint.z}
                />
              </Show>
              <Show when={props.videoTargetMode}>
                <line
                  class="waveVideoTargetLine"
                  x1={props.originPoint.x}
                  y1={props.originPoint.z}
                  x2={props.videoTargetPoint.x}
                  y2={props.videoTargetPoint.z}
                />
              </Show>
              <circle
                class={props.dragging === "origin" ? "waveOriginHandle dragging" : "waveOriginHandle"}
                cx={props.originPoint.x}
                cy={props.originPoint.z}
                r="2.1"
              />
              <Show when={!props.directionIsRadial}>
                <circle
                  class={props.dragging === "direction" ? "waveDirectionHandle dragging" : "waveDirectionHandle"}
                  cx={props.directionPoint.x}
                  cy={props.directionPoint.z}
                  r="2"
                  onPointerDown={(event) => props.onStagePointerDown(event, "direction")}
                  onPointerMove={props.onStagePointerMove}
                  onPointerUp={props.onStagePointerUp}
                  onPointerCancel={props.onStagePointerUp}
                />
              </Show>
              <Show when={props.videoTargetMode}>
                <circle
                  class={props.dragging === "videoTarget" ? "waveVideoTargetHandle dragging" : "waveVideoTargetHandle"}
                  cx={props.videoTargetPoint.x}
                  cy={props.videoTargetPoint.z}
                  r="2.1"
                  onPointerDown={(event) => props.onStagePointerDown(event, "videoTarget")}
                  onPointerMove={props.onStagePointerMove}
                  onPointerUp={props.onStagePointerUp}
                  onPointerCancel={props.onStagePointerUp}
                />
              </Show>
            </svg>
            <div class="waveStageReadout">
              <span>
                X {props.stageBounds.minX.toFixed(1)} to {props.stageBounds.maxX.toFixed(1)}
              </span>
              <span>
                Z {props.stageBounds.minZ.toFixed(1)} to {props.stageBounds.maxZ.toFixed(1)}
              </span>
            </div>
          </div>
          <div class="triple">
            <label>
              Origin X
              <input type="number" value={props.originX} onInput={(event) => props.onOriginX(Number(event.currentTarget.value))} />
            </label>
            <label>
              Origin Y
              <input type="number" value={props.originY} onInput={(event) => props.onOriginY(Number(event.currentTarget.value))} />
            </label>
            <label>
              Origin Z
              <input type="number" value={props.originZ} onInput={(event) => props.onOriginZ(Number(event.currentTarget.value))} />
            </label>
          </div>
          <div class="triple">
            <label>
              Dir X
              <input type="number" value={props.directionX} onInput={(event) => props.onDirectionX(Number(event.currentTarget.value))} />
            </label>
            <label>
              Dir Y
              <input type="number" value={props.directionY} onInput={(event) => props.onDirectionY(Number(event.currentTarget.value))} />
            </label>
            <label>
              Dir Z
              <input type="number" value={props.directionZ} onInput={(event) => props.onDirectionZ(Number(event.currentTarget.value))} />
            </label>
          </div>
          <div class="split">
            <label>
              Speed
              <input type="number" step="0.1" value={props.speed} onInput={(event) => props.onSpeed(Number(event.currentTarget.value))} />
            </label>
            <label>
              Wavelength
              <input
                type="number"
                min="0.001"
                step="0.1"
                value={props.wavelength}
                onInput={(event) => props.onWavelength(Number(event.currentTarget.value))}
              />
            </label>
          </div>
          <div class="effectBpmPeriodPanel">
            <div class="effectBpmPeriodHeader">
              <strong>Wave timing</strong>
              <span>{props.clockSyncBeats === null ? "free speed" : `${props.clockSyncBeats} beat sync`}</span>
            </div>
            <div class="effectBpmPeriodGrid" aria-label="Position wave clock sync presets">
              <For each={clockSyncBeatPresets}>
                {(preset) => (
                  <button
                    class={nearlyEqual(props.clockSyncBeats, preset.beats) ? "active" : ""}
                    onClick={() => props.onClockSyncBeats(preset.beats)}
                  >
                    <strong>{preset.label}</strong>
                    <span>{preset.beats === null ? "manual" : `${beatPeriodMs(preset.beats)}ms`}</span>
                  </button>
                )}
              </For>
            </div>
            <div class="effectBpmPeriodHeader">
              <strong>Wave shape</strong>
              <span>speed {props.speed.toFixed(1)} / wavelength {props.wavelength.toFixed(1)}</span>
            </div>
            <div class="effectBpmPeriodGrid" aria-label="Position wave speed presets">
              <For each={waveSpeedPresets}>
                {(preset) => (
                  <button class={Math.abs(props.speed - preset.speed) < 0.001 ? "active" : ""} onClick={() => props.onSpeed(preset.speed)}>
                    <strong>{preset.label}</strong>
                    <span>{preset.speed.toFixed(1)}</span>
                  </button>
                )}
              </For>
            </div>
            <div class="effectBpmPeriodGrid" aria-label="Position wave wavelength presets">
              <For each={waveLengthPresets}>
                {(preset) => (
                  <button
                    class={Math.abs(props.wavelength - preset.wavelength) < 0.001 ? "active" : ""}
                    onClick={() => props.onWavelength(preset.wavelength)}
                  >
                    <strong>{preset.label}</strong>
                    <span>{preset.wavelength.toFixed(1)}m</span>
                  </button>
                )}
              </For>
            </div>
          </div>
        </div>
      </Show>
    </>
  );
}
