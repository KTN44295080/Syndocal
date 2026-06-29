import type { JSX } from "solid-js";
import type { FixtureLimits, PatchedFixtureSummary } from "../types";
import type { FixtureLayoutMode } from "./PatchFixtureFormPanel";

type FixtureTransformUpdate = {
  position?: PatchedFixtureSummary["position"];
  rotation?: PatchedFixtureSummary["rotation"];
};

type NumericFixtureLimitField = "dimmer_min" | "dimmer_max" | "pan_min" | "pan_max" | "tilt_min" | "tilt_max";
type ToggleFixtureLimitField = "invert_pan" | "invert_tilt" | "swap_pan_tilt";
type MovementLimitPointerEvent = PointerEvent & { currentTarget: HTMLElement };

interface SetupFixtureEditorPanelProps {
  fixture: PatchedFixtureSummary;
  labelDraft: string;
  universeDraft: number;
  addressDraft: number;
  groupText: string;
  limitsDraft: FixtureLimits;
  normalizedLimits: FixtureLimits;
  limitWindowStyle: JSX.CSSProperties;
  movementLimitDragging: boolean;
  filteredFixtureCount: number;
  formatDmxPercent: (value: number) => string;
  onUseProfileForPatch: (fixture: PatchedFixtureSummary) => void | Promise<void>;
  onDuplicateFixture: (fixture: PatchedFixtureSummary) => void | Promise<void>;
  onLabelDraft: (value: string) => void;
  onUniverseDraft: (value: number) => void;
  onAddressDraft: (value: number) => void;
  onApplyPatch: (fixture: PatchedFixtureSummary) => void | Promise<void>;
  onGroupText: (value: string) => void;
  onApplyGroups: (fixture: PatchedFixtureSummary) => void | Promise<void>;
  onUpdateNumericLimit: (field: NumericFixtureLimitField, value: number) => void;
  onUpdateToggleLimit: (field: ToggleFixtureLimitField, value: boolean) => void;
  onResetLimits: () => void;
  onApplyLimits: (fixture: PatchedFixtureSummary) => void | Promise<void>;
  onMovementLimitPointerDown: (event: MovementLimitPointerEvent) => void;
  onMovementLimitPointerMove: (event: MovementLimitPointerEvent) => void;
  onMovementLimitPointerEnd: (event: MovementLimitPointerEvent) => void;
  onSetTransform: (fixture: PatchedFixtureSummary, next: FixtureTransformUpdate) => void | Promise<void>;
  onLayoutFixtures: (mode: FixtureLayoutMode) => void | Promise<void>;
}

export function SetupFixtureEditorPanel(props: SetupFixtureEditorPanelProps) {
  return (
    <div class="fixtureSetupEditor">
      <div class="panelHeader">
        <h3>Fixture Setup</h3>
        <span>{props.fixture.label}</span>
      </div>
      <button onClick={() => void props.onUseProfileForPatch(props.fixture)}>
        Use Profile for Patch
      </button>
      <button onClick={() => void props.onDuplicateFixture(props.fixture)}>
        Duplicate Fixture
      </button>
      <label>
        Label
        <input value={props.labelDraft} onInput={(event) => props.onLabelDraft(event.currentTarget.value)} />
      </label>
      <div class="split">
        <label>
          Universe
          <input
            type="number"
            min="0"
            value={props.universeDraft}
            onInput={(event) => props.onUniverseDraft(Number(event.currentTarget.value))}
          />
        </label>
        <label>
          Address
          <input
            type="number"
            min="1"
            max="512"
            value={props.addressDraft}
            onInput={(event) => props.onAddressDraft(Number(event.currentTarget.value))}
          />
        </label>
      </div>
      <button onClick={() => void props.onApplyPatch(props.fixture)}>Apply Patch</button>
      <label>
        Groups
        <input
          value={props.groupText}
          onInput={(event) => props.onGroupText(event.currentTarget.value)}
          placeholder="front, movers"
        />
      </label>
      <button onClick={() => void props.onApplyGroups(props.fixture)}>Apply Groups</button>
      <div class="limitEditor">
        <div class="limitEditorHeader">
          <strong>Dimmer Limits</strong>
          <span>
            {props.formatDmxPercent(props.normalizedLimits.dimmer_min)} -{" "}
            {props.formatDmxPercent(props.normalizedLimits.dimmer_max)}
          </span>
        </div>
        <div class="split">
          <label>
            Min
            <input
              type="number"
              min="0"
              max="65535"
              value={props.limitsDraft.dimmer_min}
              onInput={(event) => props.onUpdateNumericLimit("dimmer_min", Number(event.currentTarget.value))}
            />
          </label>
          <label>
            Max
            <input
              type="number"
              min="0"
              max="65535"
              value={props.limitsDraft.dimmer_max}
              onInput={(event) => props.onUpdateNumericLimit("dimmer_max", Number(event.currentTarget.value))}
            />
          </label>
        </div>
        <div class="limitEditorHeader">
          <strong>Movement Limits</strong>
          <span>
            Pan {props.formatDmxPercent(props.normalizedLimits.pan_min)} -{" "}
            {props.formatDmxPercent(props.normalizedLimits.pan_max)}
          </span>
        </div>
        <div class="movementLimitEditor">
          <div
            class={props.movementLimitDragging ? "movementLimitMap dragging" : "movementLimitMap"}
            aria-label="Pan tilt movement limits"
            role="slider"
            aria-valuetext={`Pan ${props.normalizedLimits.pan_min}-${props.normalizedLimits.pan_max}, Tilt ${props.normalizedLimits.tilt_min}-${props.normalizedLimits.tilt_max}`}
            onPointerDown={props.onMovementLimitPointerDown}
            onPointerMove={props.onMovementLimitPointerMove}
            onPointerUp={props.onMovementLimitPointerEnd}
            onPointerCancel={props.onMovementLimitPointerEnd}
          >
            <i style={props.limitWindowStyle} />
          </div>
          <div class="movementLimitFields">
            <div class="split">
              <label>
                Pan Min
                <input
                  type="number"
                  min="0"
                  max="65535"
                  value={props.limitsDraft.pan_min}
                  onInput={(event) => props.onUpdateNumericLimit("pan_min", Number(event.currentTarget.value))}
                />
              </label>
              <label>
                Pan Max
                <input
                  type="number"
                  min="0"
                  max="65535"
                  value={props.limitsDraft.pan_max}
                  onInput={(event) => props.onUpdateNumericLimit("pan_max", Number(event.currentTarget.value))}
                />
              </label>
            </div>
            <div class="split">
              <label>
                Tilt Min
                <input
                  type="number"
                  min="0"
                  max="65535"
                  value={props.limitsDraft.tilt_min}
                  onInput={(event) => props.onUpdateNumericLimit("tilt_min", Number(event.currentTarget.value))}
                />
              </label>
              <label>
                Tilt Max
                <input
                  type="number"
                  min="0"
                  max="65535"
                  value={props.limitsDraft.tilt_max}
                  onInput={(event) => props.onUpdateNumericLimit("tilt_max", Number(event.currentTarget.value))}
                />
              </label>
            </div>
            <div class="limitToggleRow">
              <label>
                <input
                  type="checkbox"
                  checked={props.limitsDraft.invert_pan}
                  onChange={(event) => props.onUpdateToggleLimit("invert_pan", event.currentTarget.checked)}
                />
                Invert Pan
              </label>
              <label>
                <input
                  type="checkbox"
                  checked={props.limitsDraft.invert_tilt}
                  onChange={(event) => props.onUpdateToggleLimit("invert_tilt", event.currentTarget.checked)}
                />
                Invert Tilt
              </label>
              <label>
                <input
                  type="checkbox"
                  checked={props.limitsDraft.swap_pan_tilt}
                  onChange={(event) => props.onUpdateToggleLimit("swap_pan_tilt", event.currentTarget.checked)}
                />
                Swap
              </label>
            </div>
          </div>
        </div>
        <div class="presetRow">
          <button onClick={props.onResetLimits}>Reset</button>
          <button class="primary" onClick={() => void props.onApplyLimits(props.fixture)}>
            Apply Limits
          </button>
        </div>
      </div>
      <div class="transformEditor">
        <strong>2D Mapping</strong>
        <div class="triple">
          <label>
            X
            <input
              type="number"
              value={props.fixture.position.x}
              onInput={(event) =>
                void props.onSetTransform(props.fixture, {
                  position: { ...props.fixture.position, x: Number(event.currentTarget.value) },
                })
              }
            />
          </label>
          <label>
            Y
            <input
              type="number"
              value={props.fixture.position.y}
              onInput={(event) =>
                void props.onSetTransform(props.fixture, {
                  position: { ...props.fixture.position, y: Number(event.currentTarget.value) },
                })
              }
            />
          </label>
          <label>
            Z
            <input
              type="number"
              value={props.fixture.position.z}
              onInput={(event) =>
                void props.onSetTransform(props.fixture, {
                  position: { ...props.fixture.position, z: Number(event.currentTarget.value) },
                })
              }
            />
          </label>
        </div>
        <div class="triple">
          <label>
            Pitch
            <input
              type="number"
              value={props.fixture.rotation.pitch}
              onInput={(event) =>
                void props.onSetTransform(props.fixture, {
                  rotation: { ...props.fixture.rotation, pitch: Number(event.currentTarget.value) },
                })
              }
            />
          </label>
          <label>
            Yaw
            <input
              type="number"
              value={props.fixture.rotation.yaw}
              onInput={(event) =>
                void props.onSetTransform(props.fixture, {
                  rotation: { ...props.fixture.rotation, yaw: Number(event.currentTarget.value) },
                })
              }
            />
          </label>
          <label>
            Roll
            <input
              type="number"
              value={props.fixture.rotation.roll}
              onInput={(event) =>
                void props.onSetTransform(props.fixture, {
                  rotation: { ...props.fixture.rotation, roll: Number(event.currentTarget.value) },
                })
              }
            />
          </label>
        </div>
        <div class="presetRow">
          <button onClick={() => void props.onLayoutFixtures("line")} disabled={props.filteredFixtureCount === 0}>
            Line X
          </button>
          <button onClick={() => void props.onLayoutFixtures("grid")} disabled={props.filteredFixtureCount === 0}>
            Grid
          </button>
          <button onClick={() => void props.onLayoutFixtures("circle")} disabled={props.filteredFixtureCount === 0}>
            Circle
          </button>
        </div>
      </div>
    </div>
  );
}
