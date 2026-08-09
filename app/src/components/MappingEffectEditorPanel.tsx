import { createMemo, For, Show } from "solid-js";
import { buildLfoPreviewPath } from "../effectVisualization";
import type { LfoShape, MappingEffectDirection, ValueEffectMode } from "../types";

export interface MappingEffectFixtureOption {
  id: number;
  label: string;
}

export interface MappingEffectEditorPanelProps {
  shape: LfoShape;
  mode: ValueEffectMode;
  direction: MappingEffectDirection;
  periodMs: number;
  bpm: number;
  clockSyncBeats: number | null;
  phase: number;
  spread: number;
  repetitions: number;
  fixtures: MappingEffectFixtureOption[];
  orderEditable: boolean;
  onShape: (shape: LfoShape) => void;
  onMode: (mode: ValueEffectMode) => void;
  onDirection: (direction: MappingEffectDirection) => void;
  onPeriodMs: (periodMs: number) => void;
  onClockSyncBeats: (beats: number | null) => void;
  onPhase: (phase: number) => void;
  onSpread: (spread: number) => void;
  onRepetitions: (repetitions: number) => void;
  onFixtureOrder: (fixtureIds: number[]) => void;
}

const shapes: LfoShape[] = ["Sine", "Cosine", "Triangle", "Saw", "Square", "Random", "Perlin"];
const modes: ValueEffectMode[] = ["Absolute", "Relative"];
const directions: MappingEffectDirection[] = ["Forward", "Reverse", "Bounce", "Static"];

// P-EXP quick looks: named one-click fixture-order mapping bundles
// (qa/PRESET_EXPANSION_PLAN.md). Fixture order stays authored.
interface MappingQuickLook {
  label: string;
  shape: LfoShape;
  direction: MappingEffectDirection;
  repetitions: number;
  beats: number | null;
}
const mappingQuickLooks: MappingQuickLook[] = [
  { label: "Wave Sweep", shape: "Sine", direction: "Forward", repetitions: 1, beats: 2 },
  { label: "Zigzag", shape: "Triangle", direction: "Bounce", repetitions: 2, beats: 1 },
  { label: "Step March", shape: "Square", direction: "Forward", repetitions: 4, beats: 0.5 },
];
const clocks = [
  { label: "Free", beats: null },
  { label: "1/4", beats: 0.25 },
  { label: "1/2", beats: 0.5 },
  { label: "1", beats: 1 },
  { label: "2", beats: 2 },
  { label: "4", beats: 4 },
] as const;

const clamp = (value: number, minimum: number, maximum: number) =>
  Math.min(maximum, Math.max(minimum, Number.isFinite(value) ? value : minimum));
const nearlyEqual = (first: number | null, second: number | null) =>
  first === null || second === null ? first === second : Math.abs(first - second) < 0.001;

export function MappingEffectEditorPanel(props: MappingEffectEditorPanelProps) {
  const periodMs = createMemo(() => Math.max(10, Math.round(Number.isFinite(props.periodMs) ? props.periodMs : 10)));
  const bpm = createMemo(() => (Number.isFinite(props.bpm) && props.bpm > 0 ? props.bpm : 120));
  const phasePercent = createMemo(() => Math.round(clamp(props.phase, 0, 1) * 100));
  const spreadPercent = createMemo(() => Math.round(clamp(props.spread, 0, 1) * 100));
  const repetitions = createMemo(() => clamp(props.repetitions, 0.25, 16));
  const previewPath = createMemo(() => buildLfoPreviewPath(props.shape, props.phase, 100, 40, 96, 0, 65_535, false));

  const moveFixture = (index: number, delta: -1 | 1) => {
    if (!props.orderEditable) return;
    const target = index + delta;
    if (target < 0 || target >= props.fixtures.length) return;
    const next = props.fixtures.map((fixture) => fixture.id);
    [next[index], next[target]] = [next[target], next[index]];
    props.onFixtureOrder(next);
  };

  return (
    <section class="valueEffectEditor mappingEffectEditor" aria-label="Mapping effect editor">
      <header class="valueEffectHeader">
        <div>
          <strong>Fixture-order Mapping</strong>
          <span>Scalar function distributed deterministically across authored fixture order</span>
        </div>
        <div class="valueEffectStatusStrip">
          <span><small>Fixtures</small><strong>{props.fixtures.length}</strong></span>
          <span><small>Direction</small><strong>{props.direction}</strong></span>
          <span><small>Spread / Repeat</small><strong>{spreadPercent()}% / {repetitions().toFixed(2)}×</strong></span>
        </div>
      </header>

      <div class="mappingEffectPreviewGrid">
        <div class="valueEffectCanvasWrap">
          <div class="valueEffectCanvasToolbar">
            <strong>Mapping function</strong>
            <span>Phase travels in time; spread travels across order</span>
          </div>
          <svg class="valueEffectCanvas mappingEffectCanvas" viewBox="0 0 100 40" preserveAspectRatio="none" role="img" aria-label={`${props.shape} mapping function`}>
            <rect class="valueEffectCanvasBackground" x="0" y="0" width="100" height="40" />
            <g class="valueEffectCanvasGrid" aria-hidden="true">
              <line x1="25" y1="0" x2="25" y2="40" /><line x1="50" y1="0" x2="50" y2="40" /><line x1="75" y1="0" x2="75" y2="40" />
              <line x1="0" y1="20" x2="100" y2="20" />
            </g>
            <path class="valueEffectCanvasPath" d={previewPath()} />
          </svg>
        </div>

        <section class="mappingEffectOrderPanel" aria-label="Authored fixture order">
          <header>
            <div><strong>Fixture order</strong><span>{props.orderEditable ? "Selection order is editable" : "Patch/group order is read-only"}</span></div>
            <span class="tabularNums">1–{props.fixtures.length}</span>
          </header>
          <Show when={props.fixtures.length > 0} fallback={<p>No lighting fixtures in the current target.</p>}>
            <ol class="mappingEffectOrderList">
              <For each={props.fixtures}>
                {(fixture, index) => (
                  <li data-fixture-id={fixture.id}>
                    <span class="mappingEffectOrderIndex tabularNums">{index() + 1}</span>
                    <strong data-no-localize>{fixture.label}</strong>
                    <small class="tabularNums">#{fixture.id}</small>
                    <div>
                      <button type="button" aria-label={`Move ${fixture.label} earlier`} disabled={!props.orderEditable || index() === 0} onClick={() => moveFixture(index(), -1)}>↑</button>
                      <button type="button" aria-label={`Move ${fixture.label} later`} disabled={!props.orderEditable || index() === props.fixtures.length - 1} onClick={() => moveFixture(index(), 1)}>↓</button>
                    </div>
                  </li>
                )}
              </For>
            </ol>
          </Show>
        </section>
      </div>

      <div class="mappingEffectShapeRow">
        <label>Shape<select value={props.shape} onInput={(event) => props.onShape(event.currentTarget.value as LfoShape)}><For each={shapes}>{(shape) => <option value={shape}>{shape}</option>}</For></select></label>
        <div class="valueEffectModeGroup"><span>Mode</span><div class="moveEffectSegmented" aria-label="Mapping mode"><For each={modes}>{(mode) => <button type="button" class={props.mode === mode ? "active" : ""} aria-pressed={props.mode === mode} onClick={() => props.onMode(mode)}>{mode}</button>}</For></div></div>
        <div class="valueEffectModeGroup"><span>Direction</span><div class="moveEffectSegmented mappingDirectionSegmented" aria-label="Mapping direction"><For each={directions}>{(direction) => <button type="button" class={props.direction === direction ? "active" : ""} aria-pressed={props.direction === direction} onClick={() => props.onDirection(direction)}>{direction}</button>}</For></div></div>
      </div>

      <div class="valueEffectTransportRow mappingEffectTransportRow">
        <label class="valueEffectPeriodField">Period ms<input class="tabularNums" type="number" min="10" step="10" value={periodMs()} onInput={(event) => props.onPeriodMs(Math.max(10, Math.round(Number(event.currentTarget.value) || 10)))} /></label>
        <div class="moveEffectClockPresets" aria-label="Mapping clock sync presets"><For each={clocks}>{(clock) => <button type="button" class={nearlyEqual(props.clockSyncBeats, clock.beats) ? "active" : ""} aria-pressed={nearlyEqual(props.clockSyncBeats, clock.beats)} title={clock.beats === null ? `${periodMs()} ms free` : `${Math.max(10, Math.round((60_000 / bpm()) * clock.beats))} ms at ${bpm()} BPM`} onClick={() => props.onClockSyncBeats(clock.beats)}><Show when={clock.beats !== null} fallback="Free"><span data-no-localize>{clock.label}</span></Show></button>}</For></div>
      </div>

      <div class="moveEffectClockPresets" aria-label="Mapping quick looks" data-mapping-quick-looks>
        <For each={mappingQuickLooks}>
          {(look) => (
            <button
              type="button"
              data-mapping-quick-look={look.label}
              title={`${look.label}: ${look.shape} ${look.direction}, x${look.repetitions}${look.beats === null ? "" : `, ${look.beats} beat${look.beats === 1 ? "" : "s"}`}`}
              onClick={() => {
                props.onShape(look.shape);
                props.onDirection(look.direction);
                props.onRepetitions(look.repetitions);
                props.onClockSyncBeats(look.beats);
              }}
            >
              {look.label}
            </button>
          )}
        </For>
      </div>

      <div class="valueEffectPhaseGrid mappingEffectPhaseGrid">
        <label><span>Phase</span><input type="range" min="0" max="100" value={phasePercent()} aria-label="Mapping phase percent" onInput={(event) => props.onPhase(clamp(Number(event.currentTarget.value) / 100, 0, 1))} /><input class="tabularNums" type="number" min="0" max="100" value={phasePercent()} aria-label="Mapping phase percent value" onChange={(event) => props.onPhase(clamp(Number(event.currentTarget.value) / 100, 0, 1))} /></label>
        <label><span>Spread</span><input type="range" min="0" max="100" value={spreadPercent()} aria-label="Mapping fixture spread percent" onInput={(event) => props.onSpread(clamp(Number(event.currentTarget.value) / 100, 0, 1))} /><input class="tabularNums" type="number" min="0" max="100" value={spreadPercent()} aria-label="Mapping fixture spread percent value" onChange={(event) => props.onSpread(clamp(Number(event.currentTarget.value) / 100, 0, 1))} /></label>
        <label class="mappingEffectRepetitions"><span>Repeat</span><input type="range" min="0.25" max="16" step="0.25" value={repetitions()} aria-label="Mapping repetitions" onInput={(event) => props.onRepetitions(clamp(Number(event.currentTarget.value), 0.25, 16))} /><input class="tabularNums" type="number" min="0.25" max="16" step="0.25" value={repetitions()} aria-label="Mapping repetitions value" onChange={(event) => props.onRepetitions(clamp(Number(event.currentTarget.value), 0.25, 16))} /></label>
      </div>
    </section>
  );
}
