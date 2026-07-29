import { For, Match, Show, Switch, createMemo, onCleanup } from "solid-js";
import { effectivePanTiltValues } from "../fixtureLimits";
import {
  fixtureControlForAttribute,
  fixtureTypeColorControls,
  fixtureTypeControlsForCategory,
  fixtureTypeDimmerControl,
  fixtureTypePositionControls,
  type FixtureTypeSelectionGroup,
} from "../fixtureTypeLiveEdit";
import type { AttributeControl, PatchedFixtureSummary } from "../types";
import type { ControlCategory } from "../uiModes";
import { FaderGridPanel } from "./FaderGridPanel";
import { TouchPanTiltPad } from "./TouchPanTiltPad";

interface FixtureTypeAttributeColumnsProps {
  groups: FixtureTypeSelectionGroup[];
  activeCategory: ControlCategory;
  valueForControl: (fixture: PatchedFixtureSummary, control: AttributeControl) => number;
  onSelectType: (typeKey: string) => void;
  onSetControl: (typeKey: string, attribute: string, value: number) => void;
  onResetControl: (typeKey: string, attribute: string) => void;
  onSetColor: (typeKey: string, color: string) => void;
  onSetPosition: (typeKey: string, pan: number, tilt: number) => void;
}

const clampDmxValue = (value: number) =>
  Math.min(65_535, Math.max(0, Math.round(Number.isFinite(value) ? value : 0)));

const valueToHexByte = (value: number) =>
  Math.round(clampDmxValue(value) / 257).toString(16).padStart(2, "0");

const percentLabel = (value: number) => `${Math.round((clampDmxValue(value) / 65_535) * 100)}%`;
const fineStep = (control: AttributeControl) => control.resolution === "SixteenBit" ? 1 : 257;

const fullLimitOverlay = {
  left: "0%",
  width: "100%",
  top: "0%",
  height: "100%",
};

export function FixtureTypeAttributeColumns(props: FixtureTypeAttributeColumnsProps) {
  let animationFrame: number | null = null;
  const pendingWrites = new Map<string, () => void>();

  const scheduleWrite = (key: string, write: () => void) => {
    pendingWrites.set(key, write);
    if (animationFrame !== null) {
      return;
    }
    animationFrame = requestAnimationFrame(() => {
      animationFrame = null;
      const writes = [...pendingWrites.values()];
      pendingWrites.clear();
      for (const pendingWrite of writes) {
        pendingWrite();
      }
    });
  };

  onCleanup(() => {
    if (animationFrame !== null) {
      cancelAnimationFrame(animationFrame);
    }
    pendingWrites.clear();
  });

  return (
    <section
      class="fixtureTypeColumnGrid"
      aria-label="Fixture type columns"
      data-fixture-type-column-count={props.groups.length}
    >
      <For each={props.groups}>
        {(group) => {
          const referenceFixture = () => group.fixtures[0];
          const categoryControls = createMemo(() =>
            fixtureTypeControlsForCategory(group, props.activeCategory)
          );
          const dimmerControl = createMemo(() =>
            props.activeCategory === "dimmer" ? fixtureTypeDimmerControl(group) : undefined
          );
          const colorControls = createMemo(() =>
            props.activeCategory === "color" ? fixtureTypeColorControls(group) : null
          );
          const positionControls = createMemo(() =>
            props.activeCategory === "position" ? fixtureTypePositionControls(group) : null
          );
          const valueFor = (control: AttributeControl) => {
            const fixture = referenceFixture();
            return fixture ? clampDmxValue(props.valueForControl(fixture, control)) : control.default_value;
          };
          const isMixed = (control: AttributeControl) => {
            const reference = valueFor(control);
            return group.fixtures.some((fixture) => {
              const fixtureControl = fixtureControlForAttribute(fixture, control.attribute);
              return fixtureControl
                ? clampDmxValue(props.valueForControl(fixture, fixtureControl)) !== reference
                : true;
            });
          };
          const colorValue = () => {
            const controls = colorControls();
            return controls
              ? `#${valueToHexByte(valueFor(controls.red))}${valueToHexByte(valueFor(controls.green))}${valueToHexByte(valueFor(controls.blue))}`
              : "#000000";
          };
          const positionValue = () => {
            const controls = positionControls();
            const fixture = referenceFixture();
            if (!controls || !fixture) {
              return { pan: 32_768, tilt: 32_768 };
            }
            return effectivePanTiltValues(
              fixture,
              valueFor(controls.pan),
              valueFor(controls.tilt),
            );
          };
          const setPointerPosition = (event: PointerEvent) => {
            const bounds = (event.currentTarget as HTMLElement).getBoundingClientRect();
            const x = Math.min(1, Math.max(0, (event.clientX - bounds.left) / Math.max(1, bounds.width)));
            const y = Math.min(1, Math.max(0, (event.clientY - bounds.top) / Math.max(1, bounds.height)));
            const pan = Math.round(x * 65_535);
            const tilt = Math.round((1 - y) * 65_535);
            scheduleWrite(`position:${group.key}`, () =>
              props.onSetPosition(group.key, pan, tilt)
            );
          };

          return (
            <article
              class={`fixtureTypeAttributeColumn category-${props.activeCategory} kind-${group.visualKind}`}
              data-fixture-type-column={group.key}
              data-fixture-type-count={group.fixtures.length}
              data-fixture-type-category={props.activeCategory}
              data-fixture-type-supported={categoryControls().length > 0 ? "true" : "false"}
              style={props.activeCategory === "fader"
                ? { "--fixture-type-column-width": `${Math.max(168, categoryControls().length * 97 + 8)}px` }
                : undefined}
            >
              <button
                type="button"
                class="fixtureTypeColumnHeader"
                aria-label="Select only this fixture type"
                title="Select only this fixture type"
                data-fixture-type-select={group.key}
                onClick={() => props.onSelectType(group.key)}
              >
                <i class={`fixtureTypeKindMark kind-${group.visualKind}`} aria-hidden="true" />
                <span>
                  <strong data-no-localize>{group.label}</strong>
                  <small>
                    <b data-no-localize>{group.fixtures.length}</b>{" "}
                    <span>fixtures</span>
                  </small>
                </span>
              </button>
              <div class="fixtureTypeColumnBody">
                <Switch>
                  <Match when={dimmerControl()}>
                    {(control) => (
                      <div
                        class="fixtureTypeDimmerControl"
                        data-fixture-type-primary-control="dimmer"
                      >
                        <span class="fixtureTypeDimmerIcon" aria-hidden="true" data-no-localize>☀</span>
                        <output class="fixtureTypePrimaryValue">
                          {isMixed(control()) ? "Mixed" : percentLabel(valueFor(control()))}
                        </output>
                        <input
                          class="fixtureTypeVerticalFader"
                          type="range"
                          min="0"
                          max="65535"
                          value={valueFor(control())}
                          aria-label={control().attribute}
                          aria-valuetext={isMixed(control()) ? "Mixed values" : percentLabel(valueFor(control()))}
                          onInput={(event) => {
                            const value = Number(event.currentTarget.value);
                            scheduleWrite(`control:${group.key}:${control().attribute}`, () =>
                              props.onSetControl(group.key, control().attribute, value)
                            );
                          }}
                        />
                        <div class="fixtureTypeTrimControls" role="group" aria-label="Fine adjustment">
                          <button
                            type="button"
                            class="fixtureTypeOffButton"
                            aria-label="Set dimmer off"
                            title="Set dimmer off"
                            onClick={() => props.onResetControl(group.key, control().attribute)}
                          >
                            <span data-no-localize>{"OFF"}</span>
                          </button>
                          <button
                            type="button"
                            aria-label="Decrease value"
                            title="Decrease value"
                            data-fixture-type-trim-control="decrease"
                            onClick={() =>
                              props.onSetControl(
                                group.key,
                                control().attribute,
                                valueFor(control()) - fineStep(control()),
                              )
                            }
                          >
                            <span aria-hidden="true" data-no-localize>−</span>
                          </button>
                          <button
                            type="button"
                            aria-label="Increase value"
                            title="Increase value"
                            data-fixture-type-trim-control="increase"
                            onClick={() =>
                              props.onSetControl(
                                group.key,
                                control().attribute,
                                valueFor(control()) + fineStep(control()),
                              )
                            }
                          >
                            <span aria-hidden="true" data-no-localize>+</span>
                          </button>
                        </div>
                      </div>
                    )}
                  </Match>
                  <Match when={colorControls()}>
                    <div
                      class="fixtureTypeColorControl"
                      data-fixture-type-primary-control="color"
                    >
                      <input
                        class="fixtureTypeColorPicker"
                        type="color"
                        value={colorValue()}
                        aria-label="Open color picker"
                        title="Open color picker"
                        onInput={(event) => {
                          const color = event.currentTarget.value;
                          scheduleWrite(`color:${group.key}`, () =>
                            props.onSetColor(group.key, color)
                          );
                        }}
                      />
                      <output>{colorValue().toUpperCase()}</output>
                      <span>Color picker</span>
                    </div>
                  </Match>
                  <Match when={positionControls()}>
                    <div
                      class="fixtureTypePositionControl"
                      data-fixture-type-primary-control="position"
                    >
                      <TouchPanTiltPad
                        panValue={positionValue().pan}
                        tiltValue={positionValue().tilt}
                        limitOverlayStyle={fullLimitOverlay}
                        onPointerValue={setPointerPosition}
                      />
                      <div class="fixtureTypePositionReadout">
                        <output>
                          <span>Pan</span> {percentLabel(positionValue().pan)}
                        </output>
                        <output>
                          <span>Tilt</span> {percentLabel(positionValue().tilt)}
                        </output>
                      </div>
                    </div>
                  </Match>
                  <Match when={props.activeCategory === "fader" && categoryControls().length > 0}>
                    <div
                      class="fixtureTypeFaderBank"
                      data-fixture-type-primary-control="fader-bank"
                    >
                      <FaderGridPanel
                        controls={categoryControls()}
                        selectedFixtureId={referenceFixture()?.id ?? null}
                        valueForControl={valueFor}
                        isControlWritten={(control) =>
                          group.fixtures.some((fixture) => {
                            const fixtureControl = fixtureControlForAttribute(fixture, control.attribute);
                            return fixtureControl
                              ? props.valueForControl(fixture, fixtureControl) !== fixtureControl.default_value
                              : false;
                          })
                        }
                        onSetControlValue={(control, value) =>
                          scheduleWrite(`fader:${group.key}:${control.attribute}`, () =>
                            props.onSetControl(group.key, control.attribute, value)
                          )
                        }
                      />
                    </div>
                  </Match>
                  <Match when={categoryControls().length > 0}>
                    <div
                      class="fixtureTypeSliderRack"
                      data-fixture-type-primary-control="sliders"
                    >
                      <For each={categoryControls()}>
                        {(control) => (
                          <label class="fixtureTypeRepresentativeSlider">
                            <span data-no-localize>{control.attribute}</span>
                            <input
                              type="range"
                              min="0"
                              max="65535"
                              value={valueFor(control)}
                              aria-label={control.attribute}
                              aria-valuetext={isMixed(control) ? "Mixed values" : percentLabel(valueFor(control))}
                              onInput={(event) => {
                                const value = Number(event.currentTarget.value);
                                scheduleWrite(`control:${group.key}:${control.attribute}`, () =>
                                  props.onSetControl(group.key, control.attribute, value)
                                );
                              }}
                            />
                            <output>{isMixed(control) ? "Mixed" : percentLabel(valueFor(control))}</output>
                          </label>
                        )}
                      </For>
                    </div>
                  </Match>
                </Switch>
                <Show when={categoryControls().length === 0}>
                  <div
                    class="fixtureTypeUnsupported"
                    data-fixture-type-primary-control="unsupported"
                  >
                    <strong>Unsupported</strong>
                    <span>Not supported by this fixture type.</span>
                  </div>
                </Show>
              </div>
            </article>
          );
        }}
      </For>
    </section>
  );
}
