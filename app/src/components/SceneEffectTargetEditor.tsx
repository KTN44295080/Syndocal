import { For, Show } from "solid-js";

export type SceneEffectTargetMode = "fixture" | "selection" | "group" | "video";

interface SceneEffectTargetFixtureOption {
  id: number;
  label: string;
  selected: boolean;
}

interface SceneEffectTargetGroupOption {
  id: string;
  label: string;
  count: number;
  selected: boolean;
}

export interface SceneEffectTargetEditorProps {
  mode: SceneEffectTargetMode;
  summary: string;
  explicitBeamCount: number;
  activeFixtureId: number | null;
  fixtureOptions: SceneEffectTargetFixtureOption[];
  groupOptions: SceneEffectTargetGroupOption[];
  readOnly?: boolean;
  onMode: (mode: Exclude<SceneEffectTargetMode, "video">) => void;
  onActiveFixture: (fixtureId: number) => void;
  onToggleFixture: (fixtureId: number) => void;
  onToggleGroup: (groupId: string) => void;
}

export function SceneEffectTargetEditor(props: SceneEffectTargetEditorProps) {
  const selectedFixtureCount = () => props.fixtureOptions.filter((fixture) => fixture.selected).length;
  const selectedGroupCount = () => props.groupOptions.filter((group) => group.selected).length;

  return (
    <fieldset class="sceneEffectTargetEditor" data-scene-effect-target-mode={props.mode}>
      <legend>Beams / fixtures</legend>
      <Show when={props.readOnly}>
        <p class="sceneEffectTargetNotice">Explicit beam targets are preserved without retargeting.</p>
        <div class="sceneEffectTargetSummary">
          <strong data-no-localize>{props.summary}</strong>
          <span>{props.explicitBeamCount} explicit beam / segment targets</span>
        </div>
      </Show>
      <Show when={!props.readOnly}>
        <div class="sceneEffectTargetModes" role="group" aria-label="FX target scope">
          <button type="button" classList={{ active: props.mode === "fixture" }} aria-pressed={props.mode === "fixture"} onClick={() => props.onMode("fixture")}>Fixture</button>
          <button type="button" classList={{ active: props.mode === "selection" }} aria-pressed={props.mode === "selection"} onClick={() => props.onMode("selection")}>Selection</button>
          <button type="button" classList={{ active: props.mode === "group" }} aria-pressed={props.mode === "group"} onClick={() => props.onMode("group")}>Groups</button>
        </div>
        <div class="sceneEffectTargetSummary">
          <strong data-no-localize>{props.summary}</strong>
          <span>{props.explicitBeamCount > 0 ? `${props.explicitBeamCount} explicit beam / segment targets` : "Fixture attributes resolve through the patched profile."}</span>
        </div>

      <Show when={props.mode === "fixture"}>
        <label class="sceneEffectTargetFixtureSelect">
          Fixture
          <select
            value={props.activeFixtureId ?? ""}
            disabled={props.fixtureOptions.length === 0}
            onInput={(event) => props.onActiveFixture(Number(event.currentTarget.value))}
          >
            <For each={props.fixtureOptions}>
              {(fixture) => <option value={fixture.id} data-no-localize>{fixture.label}</option>}
            </For>
          </select>
        </label>
      </Show>

      <Show when={props.mode === "selection"}>
        <details class="sceneEffectTargetDisclosure" open>
          <summary>Fixture selection <span class="tabularNums">{selectedFixtureCount()} selected</span></summary>
          <div class="sceneEffectTargetChipGrid" role="group" aria-label="FX fixture selection">
            <For each={props.fixtureOptions}>
              {(fixture) => (
                <button
                  type="button"
                  classList={{ active: fixture.selected }}
                  aria-pressed={fixture.selected}
                  data-effect-target-fixture={fixture.id}
                  onClick={() => props.onToggleFixture(fixture.id)}
                >
                  <span class="tabularNums">{fixture.id}</span>
                  <strong data-no-localize>{fixture.label}</strong>
                </button>
              )}
            </For>
          </div>
        </details>
      </Show>

      <Show when={props.mode === "group"}>
        <details class="sceneEffectTargetDisclosure" open>
          <summary>Fixture groups <span class="tabularNums">{selectedGroupCount()} selected</span></summary>
          <div class="sceneEffectTargetChipGrid groups" role="group" aria-label="FX group selection">
            <For each={props.groupOptions}>
              {(group) => (
                <button
                  type="button"
                  classList={{ active: group.selected }}
                  aria-pressed={group.selected}
                  data-effect-target-group={group.id}
                  onClick={() => props.onToggleGroup(group.id)}
                >
                  <strong data-no-localize>{group.label}</strong>
                  <span class="tabularNums">{group.count}</span>
                </button>
              )}
            </For>
          </div>
        </details>
      </Show>

      <Show when={props.mode === "video"}>
        <p class="sceneEffectTargetNotice">This legacy FX currently targets video. Choose a lighting target above to retarget it.</p>
      </Show>
      </Show>
    </fieldset>
  );
}
