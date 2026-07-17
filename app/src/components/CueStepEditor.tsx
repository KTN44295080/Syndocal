import { createEffect, createMemo, createSignal, For, Show } from "solid-js";
import {
  appendCueStep,
  cloneCueSteps,
  cueStepsAuthoredBeats,
  cueStepsTotalDurationMs,
  duplicateCueStep,
  moveCueStep,
  sanitizeCueStepDuration,
} from "../cueSteps";
import type { CueStepSummary, CueSummary } from "../types";

interface CueStepEditorProps {
  cue: CueSummary;
  bpm: number;
  onSave: (cueId: number, steps: CueStepSummary[]) => void | Promise<void>;
  onUseAuthoredBeats: (beats: number) => void;
}

const stepSignature = (steps: CueStepSummary[] | null | undefined) => JSON.stringify(steps ?? []);

export function CueStepEditor(props: CueStepEditorProps) {
  const [steps, setSteps] = createSignal(cloneCueSteps(props.cue.steps));
  const [dirty, setDirty] = createSignal(false);
  let sourceSignature = stepSignature(props.cue.steps);

  createEffect(() => {
    const nextSignature = stepSignature(props.cue.steps);
    if (nextSignature === sourceSignature) return;
    sourceSignature = nextSignature;
    setSteps(cloneCueSteps(props.cue.steps));
    setDirty(false);
  });

  const replaceSteps = (next: CueStepSummary[]) => {
    setSteps(next);
    setDirty(stepSignature(next) !== sourceSignature);
  };
  const totalDurationMs = createMemo(() => cueStepsTotalDurationMs(steps()));
  const derivedBeats = createMemo(() => cueStepsAuthoredBeats(steps(), props.bpm));
  const updateDuration = (index: number, field: "fade_ms" | "hold_ms", value: number) => {
    replaceSteps(steps().map((step, candidate) => candidate === index
      ? { ...step, [field]: sanitizeCueStepDuration(value) }
      : step));
  };
  const valueCount = (step: CueStepSummary) =>
    step.values.reduce((count, target) => count + target.values.length, 0);

  return (
    <details
      class="cueStepEditor cueEditOnly"
      data-cue-step-editor
      data-cue-step-count={steps().length}
    >
      <summary>
        <span>{`Static steps (${steps().length})`}</span>
        <small class="tabularNums">{`${totalDurationMs()} ms total`}</small>
      </summary>
      <p>
        Each step stores a fixture-value snapshot. Fade moves into the step, then Hold keeps it before the next step.
      </p>
      <div class="cueStepToolbar">
        <button
          type="button"
          data-cue-step-add
          onClick={() => replaceSteps(appendCueStep(steps(), props.cue.targets, props.cue.fade_ms))}
        >
          Add Step
        </button>
        <span class="tabularNums">
          {derivedBeats() === null
            ? "Add timed steps to derive authored beats."
            : `${derivedBeats()!.toFixed(3)} beats at ${props.bpm.toFixed(1)} BPM`}
        </span>
        <button
          type="button"
          disabled={derivedBeats() === null}
          data-cue-step-use-beats
          onClick={() => {
            const beats = derivedBeats();
            if (beats !== null) props.onUseAuthoredBeats(beats);
          }}
        >
          Use Step Timing
        </button>
      </div>
      <Show
        when={steps().length > 0}
        fallback={<p class="cueStepEmpty">This Cue uses its stored single state. Add a step to create a sequence.</p>}
      >
        <div class="cueStepList" role="list" aria-label="Cue static steps">
          <For each={steps()}>
            {(step, index) => (
              <div class="cueStepRow" role="listitem" data-cue-step-index={index()}>
                <strong class="tabularNums">{`Step ${index() + 1}`}</strong>
                <span>
                  {`${step.values.length} fixture(s) · ${valueCount(step)} value(s)`}
                </span>
                <label>
                  Fade ms
                  <input
                    class="tabularNums"
                    type="number"
                    min="0"
                    value={step.fade_ms}
                    data-cue-step-fade
                    onInput={(event) => updateDuration(index(), "fade_ms", Number(event.currentTarget.value))}
                  />
                </label>
                <label>
                  Hold ms
                  <input
                    class="tabularNums"
                    type="number"
                    min="0"
                    value={step.hold_ms}
                    data-cue-step-hold
                    onInput={(event) => updateDuration(index(), "hold_ms", Number(event.currentTarget.value))}
                  />
                </label>
                <button
                  type="button"
                  data-cue-step-duplicate
                  onClick={() => replaceSteps(duplicateCueStep(steps(), index()))}
                >
                  Duplicate
                </button>
                <button
                  type="button"
                  disabled={index() === 0}
                  data-cue-step-up
                  onClick={() => replaceSteps(moveCueStep(steps(), index(), -1))}
                >
                  Up
                </button>
                <button
                  type="button"
                  disabled={index() === steps().length - 1}
                  data-cue-step-down
                  onClick={() => replaceSteps(moveCueStep(steps(), index(), 1))}
                >
                  Down
                </button>
                <button
                  type="button"
                  class="danger"
                  data-cue-step-remove
                  onClick={() => replaceSteps(steps().filter((_, candidate) => candidate !== index()))}
                >
                  Remove
                </button>
              </div>
            )}
          </For>
        </div>
      </Show>
      <div class="cueStepFooter">
        <span>{dirty() ? "Step edits are not saved." : "Step sequence is saved."}</span>
        <button
          type="button"
          class="primary"
          disabled={!dirty()}
          data-cue-step-save
          onClick={() => void props.onSave(props.cue.id, cloneCueSteps(steps()))}
        >
          Save Steps
        </button>
      </div>
    </details>
  );
}
