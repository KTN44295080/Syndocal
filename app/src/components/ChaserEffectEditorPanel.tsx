import { createMemo, createSignal, For, Show } from "solid-js";
import {
  canonicalChaserAttribute,
  chaserMaximumSteps,
  chaserActivePreviewIndices,
  chaserPreviewOrder,
  clampChaserLevel,
  clampChaserSeed,
  clampChaserUnit,
  clampChaserWings,
  normalizedChaserStep,
} from "../chaserDraft";
import type { ChaserDirection, ChaserFeature, ChaserStep } from "../types";

const chaserStepsPerPage = 8;
const clockSyncBeatPresets = [
  { label: "Free", beats: null },
  { label: "1/4", beats: 0.25 },
  { label: "1/2", beats: 0.5 },
  { label: "1", beats: 1 },
  { label: "2", beats: 2 },
  { label: "4", beats: 4 },
] as const;
const directionOptions: { value: ChaserDirection; label: string; detail: string }[] = [
  { value: "Forward", label: "One way only", detail: "first to last" },
  { value: "Reverse", label: "Reverse", detail: "last to first" },
  { value: "Bounce", label: "Bounce", detail: "return pass" },
  { value: "BuildUpDown", label: "Build / clear", detail: "fill then clear" },
  { value: "Random", label: "Random", detail: "seeded order" },
];

interface ChaserFixtureOption {
  id: number;
  label: string;
}

interface ChaserEffectEditorPanelProps {
  steps: ChaserStep[];
  features: ChaserFeature[];
  fixtureOptions: ChaserFixtureOption[];
  attributeOptions: string[];
  attributeCoverage: Record<string, string>;
  currentTargetLabel: string;
  currentTargetSteps: ChaserStep[];
  stepDurationMs: number;
  bpm: number;
  clockSyncBeats: number | null;
  direction: ChaserDirection;
  wings: number;
  activeStepCount: number;
  dutyCycle: number;
  overlap: number;
  phase: number;
  fixtureSpread: number;
  randomSeed: number;
  error: string;
  onSteps: (steps: ChaserStep[]) => void;
  onFeatures: (features: ChaserFeature[]) => void;
  onStepDurationMs: (durationMs: number) => void;
  onClockSyncBeats: (beats: number | null) => void;
  onDirection: (direction: ChaserDirection) => void;
  onWings: (wings: number) => void;
  onActiveStepCount: (activeStepCount: number) => void;
  onDutyCycle: (dutyCycle: number) => void;
  onOverlap: (overlap: number) => void;
  onFixtureSpread: (fixtureSpread: number) => void;
  onRandomSeed: (randomSeed: number) => void;
}

const nearlyEqual = (first: number | null, second: number | null) =>
  first === null || second === null ? first === second : Math.abs(first - second) < 0.001;

export function ChaserEffectEditorPanel(props: ChaserEffectEditorPanelProps) {
  const [page, setPage] = createSignal(0);
  const fixtureLabels = createMemo(() => new Map(props.fixtureOptions.map((fixture) => [fixture.id, fixture.label])));
  const pageCount = createMemo(() => Math.max(1, Math.ceil(props.steps.length / chaserStepsPerPage)));
  const currentPage = createMemo(() => Math.min(page(), pageCount() - 1));
  const pageStart = createMemo(() => currentPage() * chaserStepsPerPage);
  const visibleSteps = createMemo(() => props.steps.slice(pageStart(), pageStart() + chaserStepsPerPage));
  const activeStepMaximum = createMemo(() => Math.max(1, Math.min(props.steps.length, 64)));
  const wingMaximum = createMemo(() => Math.max(1, Math.min(props.steps.length, 16)));
  const directionLabel = createMemo(() =>
    directionOptions.find((option) => option.value === props.direction)?.label ?? props.direction,
  );
  const directionOrder = createMemo(() => chaserPreviewOrder(props.steps.length, props.direction, props.randomSeed));
  const activeOrderPosition = createMemo(() => {
    const order = directionOrder();
    if (order.length === 0) return 0;
    return Math.floor(clampChaserUnit(props.phase) * order.length) % order.length;
  });
  const previewEntries = createMemo(() => {
    const order = directionOrder();
    const maximum = 24;
    const start = Math.max(0, Math.min(order.length - maximum, activeOrderPosition() - Math.floor(maximum / 2)));
    return order.slice(start, start + maximum).map((stepIndex, offset) => ({
      stepIndex,
      orderPosition: start + offset,
    }));
  });
  const activePixels = createMemo(() => {
    const order = directionOrder();
    return new Set(chaserActivePreviewIndices(order, activeOrderPosition(), props.activeStepCount, props.direction));
  });
  const clockSummary = createMemo(() =>
    props.clockSyncBeats === null
      ? `${Math.max(10, Math.round(props.stepDurationMs))} ms per step`
      : `${props.clockSyncBeats} beat per step`,
  );
  const beatPeriodMs = (beats: number) => {
    const bpm = Number.isFinite(props.bpm) && props.bpm > 0 ? props.bpm : 120;
    return Math.max(10, Math.round((60_000 / bpm) * beats));
  };
  const stepTargetLabel = (step: ChaserStep) => {
    const labels = step.fixture_ids.map((fixtureId) => fixtureLabels().get(fixtureId) ?? `Fixture ${fixtureId}`);
    labels.push(...step.target_group_ids.map((groupId) => `Group ${groupId}`));
    return labels.length > 0 ? labels.join(" + ") : "Gap";
  };
  const publishSteps = (steps: ChaserStep[]) => {
    const normalized = steps.slice(0, chaserMaximumSteps).map(normalizedChaserStep);
    props.onSteps(normalized);
    props.onActiveStepCount(Math.min(props.activeStepCount, Math.max(1, Math.min(normalized.length, 64))));
    props.onWings(Math.min(Math.max(1, props.wings), Math.max(1, Math.min(normalized.length, 16))));
  };
  const updateStep = (index: number, patch: Partial<ChaserStep>) => {
    const next = props.steps.map((step) => ({ ...step, fixture_ids: [...step.fixture_ids], target_group_ids: [...step.target_group_ids] }));
    if (!next[index]) return;
    next[index] = normalizedChaserStep({ ...next[index], ...patch });
    publishSteps(next);
  };
  const moveStep = (index: number, direction: -1 | 1) => {
    const targetIndex = index + direction;
    if (!props.steps[index] || !props.steps[targetIndex]) return;
    const next = [...props.steps];
    [next[index], next[targetIndex]] = [next[targetIndex], next[index]];
    publishSteps(next);
  };
  const removeStep = (index: number) => publishSteps(props.steps.filter((_, candidate) => candidate !== index));
  const replaceStepWithCurrent = (index: number) => {
    if (!props.steps[index] || props.currentTargetSteps.length === 0) return;
    const room = chaserMaximumSteps - props.steps.length + 1;
    const replacements = props.currentTargetSteps.slice(0, room).map(normalizedChaserStep);
    publishSteps([
      ...props.steps.slice(0, index),
      ...replacements,
      ...props.steps.slice(index + 1),
    ]);
  };
  const appendCurrent = () => {
    const room = Math.max(0, chaserMaximumSteps - props.steps.length);
    if (room === 0 || props.currentTargetSteps.length === 0) return;
    const next = [...props.steps, ...props.currentTargetSteps.slice(0, room).map(normalizedChaserStep)];
    if (next.length === 1 && next.length < chaserMaximumSteps) {
      next.push({ fixture_ids: [], target_group_ids: [], level: 0 });
    }
    publishSteps(next);
    setPage(Math.max(0, Math.ceil(next.length / chaserStepsPerPage) - 1));
  };
  const addGap = () => {
    if (props.steps.length >= chaserMaximumSteps) return;
    const next = [...props.steps, { fixture_ids: [], target_group_ids: [], level: 0 }];
    publishSteps(next);
    setPage(Math.max(0, Math.ceil(next.length / chaserStepsPerPage) - 1));
  };
  const updateFeature = (index: number, patch: Partial<ChaserFeature>) => {
    const next = props.features.map((feature) => ({ ...feature }));
    if (!next[index]) return;
    next[index] = {
      ...next[index],
      ...patch,
      low: patch.low === undefined ? next[index].low : clampChaserLevel(patch.low),
      high: patch.high === undefined ? next[index].high : clampChaserLevel(patch.high),
    };
    props.onFeatures(next);
  };
  const moveFeature = (index: number, direction: -1 | 1) => {
    const targetIndex = index + direction;
    if (!props.features[index] || !props.features[targetIndex]) return;
    const next = props.features.map((feature) => ({ ...feature }));
    [next[index], next[targetIndex]] = [next[targetIndex], next[index]];
    props.onFeatures(next);
  };
  const addFeature = () => {
    if (props.features.length >= 16) return;
    const used = new Set(props.features.map((feature) => feature.attribute.toLocaleLowerCase()));
    const attribute = props.attributeOptions.find((candidate) => !used.has(candidate.toLocaleLowerCase())) ?? "Dimmer";
    props.onFeatures([...props.features, { attribute, low: 0, high: 65_535 }]);
  };
  const featureOptions = (feature: ChaserFeature) =>
    [...new Set([feature.attribute, ...props.attributeOptions].filter(Boolean))];
  const featureCoverage = (feature: ChaserFeature) =>
    props.attributeCoverage[canonicalChaserAttribute(feature.attribute)] ?? "0/0 fixtures";

  return (
    <section class="chaserEffectEditor" aria-label="Chaser effect editor">
      <fieldset class="chaserPreviewPanel">
        <legend>Chase preview</legend>
        <div class="chaserPreviewHeader">
          <div>
            <strong>{directionLabel()}</strong>
            <span class="tabularNums">{props.activeStepCount} pixels on · {Math.round(props.dutyCycle * 100)}% size</span>
          </div>
          <span class="tabularNums">phase {Math.round(clampChaserUnit(props.phase) * 100)}%</span>
        </div>
        <div
          class="chaserPreviewStrip"
          role="img"
          aria-label={`Chaser preview: ${directionLabel()}, ${props.activeStepCount} pixels on, ${Math.round(props.dutyCycle * 100)} percent size, ${Math.round(props.phase * 100)} percent phase`}
          data-direction={props.direction}
          data-active-step-count={props.activeStepCount}
          data-size-percent={Math.round(props.dutyCycle * 100)}
          data-phase-percent={Math.round(props.phase * 100)}
        >
          <Show when={previewEntries().length > 0} fallback={<span class="chaserPreviewEmpty">Add a target to build Chaser steps.</span>}>
            <For each={previewEntries()}>
              {(entry) => {
                const step = () => props.steps[entry.stepIndex];
                const active = () => activePixels().has(entry.stepIndex);
                return (
                  <span
                    class={active() ? "chaserPreviewCell active" : "chaserPreviewCell"}
                    title={`Step ${entry.stepIndex + 1}: ${stepTargetLabel(step())}`}
                    data-step-index={entry.stepIndex}
                    style={{
                      "--chaser-level": `${Math.round((step().level / 65_535) * 100)}%`,
                      "--chaser-size": `${Math.round(clampChaserUnit(props.dutyCycle) * 100)}%`,
                    }}
                    aria-hidden="true"
                  >
                    <i />
                    <b>{entry.stepIndex + 1}</b>
                  </span>
                );
              }}
            </For>
          </Show>
        </div>
        <small class="chaserPreviewWindow tabularNums">
          {Math.min(previewEntries().length, props.steps.length)} visible / {props.steps.length} steps
        </small>
      </fieldset>

      <fieldset class="chaserFlowPanel">
        <legend>Chase flow</legend>
        <div class="chaserDirectionGrid" aria-label="Chaser direction">
          <For each={directionOptions}>
            {(option) => (
              <button
                type="button"
                class={props.direction === option.value ? "active" : ""}
                aria-pressed={props.direction === option.value}
                onClick={() => {
                  props.onDirection(option.value);
                  if (option.value === "BuildUpDown") props.onActiveStepCount(1);
                }}
              >
                <strong>{option.label}</strong>
                <span>{option.detail}</span>
              </button>
            )}
          </For>
        </div>
        <div class="chaserParameterGrid">
          <label>
            Pixels on
            <input
              class="tabularNums"
              type="number"
              min="1"
              max={activeStepMaximum()}
              step="1"
              value={props.direction === "BuildUpDown" ? 1 : Math.min(props.activeStepCount, activeStepMaximum())}
              disabled={props.direction === "BuildUpDown"}
              title={props.direction === "BuildUpDown" ? "Build / clear uses one transition frontier" : undefined}
              onChange={(event) => props.onActiveStepCount(Math.round(Number(event.currentTarget.value)))}
            />
          </label>
          <label>
            Wings
            <input
              class="tabularNums"
              type="number"
              min="1"
              max={wingMaximum()}
              step="1"
              value={Math.min(props.wings, wingMaximum())}
              onChange={(event) => props.onWings(Math.min(clampChaserWings(Number(event.currentTarget.value)), wingMaximum()))}
            />
          </label>
          <label class="chaserPercentControl">
            <span>Size</span>
            <input
              type="range"
              min="1"
              max="100"
              step="1"
              value={Math.round(props.dutyCycle * 100)}
              aria-label="Chaser size percent"
              onInput={(event) => props.onDutyCycle(Math.max(0.01, Number(event.currentTarget.value) / 100))}
            />
            <output class="tabularNums">{Math.round(props.dutyCycle * 100)}%</output>
          </label>
          <label class="chaserPercentControl">
            <span>Fixture spread</span>
            <input
              type="range"
              min="0"
              max="100"
              step="1"
              value={Math.round(props.fixtureSpread * 100)}
              aria-label="Chaser fixture spread percent"
              onInput={(event) => props.onFixtureSpread(clampChaserUnit(Number(event.currentTarget.value) / 100))}
            />
            <output class="tabularNums">{Math.round(props.fixtureSpread * 100)}%</output>
          </label>
        </div>
        <label class="checkbox inlineCheckbox chaserFadingToggle">
          <input
            type="checkbox"
            checked={props.overlap > 0}
            onChange={(event) => props.onOverlap(event.currentTarget.checked ? Math.max(0.25, props.overlap) : 0)}
          />
          Fading
        </label>
        <Show when={props.overlap > 0}>
          <label class="chaserPercentControl chaserOverlapControl">
            <span>Fade overlap</span>
            <input
              type="range"
              min="1"
              max="100"
              step="1"
              value={Math.max(1, Math.round(props.overlap * 100))}
              aria-label="Chaser fade overlap percent"
              onInput={(event) => props.onOverlap(Math.max(0.01, Number(event.currentTarget.value) / 100))}
            />
            <output class="tabularNums">{Math.round(props.overlap * 100)}%</output>
          </label>
        </Show>
        <Show when={props.direction === "Random"}>
          <label>
            Random seed
            <input
              class="tabularNums"
              type="number"
              min="1"
              max="4294967295"
              step="1"
              value={props.randomSeed}
              onChange={(event) => props.onRandomSeed(clampChaserSeed(Number(event.currentTarget.value)))}
            />
          </label>
        </Show>
        <div class="chaserClockHeader">
          <strong>Step clock</strong>
          <span class="tabularNums">{clockSummary()}</span>
        </div>
        <label class="chaserStepDurationInput">
          Step ms
          <input
            class="tabularNums"
            type="number"
            min="10"
            step="10"
            value={props.stepDurationMs}
            onInput={(event) => props.onStepDurationMs(Math.max(10, Math.round(Number(event.currentTarget.value) || 10)))}
          />
        </label>
        <div class="chaserClockPresets" aria-label="Chaser clock sync presets">
          <For each={clockSyncBeatPresets}>
            {(preset) => (
              <button
                type="button"
                class={nearlyEqual(props.clockSyncBeats, preset.beats) ? "active" : ""}
                aria-pressed={nearlyEqual(props.clockSyncBeats, preset.beats)}
                onClick={() => props.onClockSyncBeats(preset.beats)}
              >
                <strong>{preset.label}</strong>
                <span class="tabularNums">{preset.beats === null ? "manual" : `${beatPeriodMs(preset.beats)} ms`}</span>
              </button>
            )}
          </For>
        </div>
      </fieldset>

      <fieldset class="chaserFeaturesPanel">
        <legend>Features</legend>
        <div class="chaserFeatureList" role="list" aria-label="Chaser feature ranges">
          <For each={props.features}>
            {(feature, index) => (
              <div class="chaserFeatureRow" role="listitem" aria-posinset={index() + 1} aria-setsize={props.features.length}>
                <span class="chaserFeatureNumber tabularNums" aria-hidden="true">{index() + 1}</span>
                <label>
                  <span>Attribute</span>
                  <select value={feature.attribute} onInput={(event) => updateFeature(index(), { attribute: event.currentTarget.value })}>
                    <For each={featureOptions(feature)}>{(attribute) => <option value={attribute} data-no-localize>{attribute}</option>}</For>
                  </select>
                  <small class="chaserFeatureCoverage tabularNums" data-no-localize>{featureCoverage(feature)}</small>
                </label>
                <label>
                  <span>Low</span>
                  <input class="tabularNums" type="number" min="0" max="65535" value={feature.low} onChange={(event) => updateFeature(index(), { low: Number(event.currentTarget.value) })} />
                </label>
                <label>
                  <span>High</span>
                  <input class="tabularNums" type="number" min="0" max="65535" value={feature.high} onChange={(event) => updateFeature(index(), { high: Number(event.currentTarget.value) })} />
                </label>
                <button type="button" aria-label={`Move Chaser feature ${index() + 1} up`} title="Move feature up" disabled={index() === 0} onClick={() => moveFeature(index(), -1)}>↑</button>
                <button type="button" aria-label={`Move Chaser feature ${index() + 1} down`} title="Move feature down" disabled={index() === props.features.length - 1} onClick={() => moveFeature(index(), 1)}>↓</button>
                <button type="button" class="remove" aria-label={`Remove Chaser feature ${index() + 1}`} title="Remove feature" disabled={props.features.length <= 1} onClick={() => props.onFeatures(props.features.filter((_, candidate) => candidate !== index()))}>×</button>
              </div>
            )}
          </For>
        </div>
        <div class="chaserFeatureFooter">
          <span class="tabularNums">{props.features.length} / 16 features</span>
          <button type="button" onClick={addFeature} disabled={props.features.length >= 16}>Add feature</button>
        </div>
      </fieldset>

      <fieldset class="chaserStepsPanel">
        <legend>Beam order</legend>
        <div class="chaserTargetDock">
          <div>
            <strong>Current target</strong>
            <span data-no-localize>{props.currentTargetLabel}</span>
          </div>
          <button type="button" onClick={appendCurrent} disabled={props.currentTargetSteps.length === 0 || props.steps.length >= chaserMaximumSteps}>Append current</button>
          <button type="button" onClick={addGap} disabled={props.steps.length >= chaserMaximumSteps}>Add gap</button>
        </div>
        <div class="chaserStepList" role="list" aria-label="Chaser beam steps" aria-rowcount={props.steps.length}>
          <For each={visibleSteps()}>
            {(step, pageIndex) => {
              const index = () => pageStart() + pageIndex();
              return (
                <div class="chaserStepRow" role="listitem" aria-posinset={index() + 1} aria-setsize={props.steps.length}>
                  <span class="chaserStepNumber tabularNums" aria-hidden="true">{index() + 1}</span>
                  <div class="chaserStepTarget">
                    <strong data-no-localize>{stepTargetLabel(step)}</strong>
                    <span>{step.fixture_ids.length + step.target_group_ids.length === 0 ? "No beam output" : "Ordered beam target"}</span>
                  </div>
                  <label>
                    <span>Level</span>
                    <input class="tabularNums" type="number" min="0" max="65535" value={step.level} onChange={(event) => updateStep(index(), { level: Number(event.currentTarget.value) })} />
                  </label>
                  <button type="button" aria-label={`Replace Chaser step ${index() + 1} with current target`} title="Replace with current target" disabled={props.currentTargetSteps.length === 0} onClick={() => replaceStepWithCurrent(index())}>↺</button>
                  <button type="button" aria-label={`Move Chaser step ${index() + 1} up`} title="Move step up" disabled={index() === 0} onClick={() => moveStep(index(), -1)}>↑</button>
                  <button type="button" aria-label={`Move Chaser step ${index() + 1} down`} title="Move step down" disabled={index() === props.steps.length - 1} onClick={() => moveStep(index(), 1)}>↓</button>
                  <button type="button" class="remove" aria-label={`Remove Chaser step ${index() + 1}`} title="Remove step" onClick={() => removeStep(index())}>×</button>
                </div>
              );
            }}
          </For>
        </div>
        <Show when={pageCount() > 1}>
          <nav class="chaserStepPager" aria-label="Chaser step pages">
            <button type="button" onClick={() => setPage(Math.max(0, currentPage() - 1))} disabled={currentPage() === 0}>Previous</button>
            <span class="tabularNums">Page {currentPage() + 1} / {pageCount()}</span>
            <button type="button" onClick={() => setPage(Math.min(pageCount() - 1, currentPage() + 1))} disabled={currentPage() === pageCount() - 1}>Next</button>
          </nav>
        </Show>
        <div class="chaserStepFooter">
          <span class="tabularNums">{props.steps.length} / {chaserMaximumSteps} steps</span>
          <span>One fixture per step preserves the patched beam order.</span>
        </div>
      </fieldset>

      <Show when={props.error}>
        <p class="fieldError textPretty" role="alert">{props.error}</p>
      </Show>
    </section>
  );
}
