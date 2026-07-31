import { For, Show } from "solid-js";

export type FixtureLayoutMode = "line" | "grid" | "circle";

interface PatchFixtureFormPanelProps {
  universe: number;
  address: number;
  count: number;
  addressStride: number;
  layoutMode: FixtureLayoutMode;
  gridColumns: number;
  circleRadius: number;
  x: number;
  y: number;
  z: number;
  xStep: number;
  zStep: number;
  pitch: number;
  yaw: number;
  roll: number;
  footprint: number;
  normalizedCount: number;
  normalizedAddressStride: number;
  normalizedGridColumns: number;
  normalizedCircleRadius: number;
  endAddress: number;
  conflictText: string;
  invalid: boolean;
  nextFreeAddress: number | null;
  warnings: string[];
  onUniverse: (value: number) => void;
  onAddress: (value: number) => void;
  onCount: (value: number) => void;
  onAddressStride: (value: number) => void;
  onLayoutMode: (value: FixtureLayoutMode) => void;
  onGridColumns: (value: number) => void;
  onCircleRadius: (value: number) => void;
  onX: (value: number) => void;
  onY: (value: number) => void;
  onZ: (value: number) => void;
  onXStep: (value: number) => void;
  onZStep: (value: number) => void;
  onPitch: (value: number) => void;
  onYaw: (value: number) => void;
  onRoll: (value: number) => void;
  onPatch: () => void | Promise<void>;
  onNextFreeAddress: () => void;
}

export function PatchFixtureFormPanel(props: PatchFixtureFormPanelProps) {
  const layoutSummary = () =>
    props.layoutMode === "grid"
      ? `Grid ${props.normalizedGridColumns} col`
      : props.layoutMode === "circle"
        ? `Circle r${props.normalizedCircleRadius.toFixed(1)}`
        : "Line";

  return (
    <section class="patchFixtureFormPanel">
      <div class="patchFixtureMinimalForm" data-patch-minimal-form>
        <label data-patch-field="universe">
          Universe
          <input
            type="number"
            min="0"
            value={props.universe}
            onInput={(event) => props.onUniverse(Number(event.currentTarget.value))}
          />
        </label>
        <label data-patch-field="address">
          Address
          <input
            type="number"
            min="1"
            max="512"
            value={props.address}
            onInput={(event) => props.onAddress(Number(event.currentTarget.value))}
          />
        </label>
        <label data-patch-field="count">
          Count
          <input
            type="number"
            min="1"
            max="256"
            value={props.count}
            onInput={(event) => props.onCount(Number(event.currentTarget.value))}
          />
        </label>
        <button class="primary patchFixtureSubmit" onClick={props.onPatch} disabled={props.invalid}>
          PATCH
        </button>
        <button
          class="patchFixtureNextFree"
          onClick={props.onNextFreeAddress}
          disabled={props.nextFreeAddress === null}
        >
          Next Free A{props.nextFreeAddress ?? "-"}
        </button>
        <details class="patchFixtureOptionsDisclosure" data-patch-placement-disclosure>
          <summary>Placement options</summary>
          <div class="patchFixtureOptions">
            <div class="fieldWithAction">
              <label>
                Addr Step
                <input
                  type="number"
                  min="0"
                  value={props.addressStride}
                  title="0 uses the selected profile footprint as the spacing."
                  onInput={(event) => props.onAddressStride(Number(event.currentTarget.value))}
                />
              </label>
              <button
                type="button"
                title={`Use the selected profile footprint (${props.footprint || 1}ch) as the fixture spacing.`}
                disabled={props.addressStride <= 0}
                onClick={() => props.onAddressStride(0)}
              >
                Auto
              </button>
            </div>
            <div class="split">
              <label>
                Layout
                <select
                  value={props.layoutMode}
                  onInput={(event) => props.onLayoutMode(event.currentTarget.value as FixtureLayoutMode)}
                >
                  <option value="line">Line</option>
                  <option value="grid">Grid</option>
                  <option value="circle">Circle</option>
                </select>
              </label>
              <Show
                when={props.layoutMode === "circle"}
                fallback={
                  <label>
                    Grid Cols
                    <input
                      type="number"
                      min="1"
                      max="64"
                      value={props.gridColumns}
                      onInput={(event) => props.onGridColumns(Number(event.currentTarget.value))}
                      disabled={props.layoutMode !== "grid"}
                    />
                  </label>
                }
              >
                <label>
                  Radius
                  <input
                    type="number"
                    min="0.1"
                    step="0.1"
                    value={props.circleRadius}
                    onInput={(event) => props.onCircleRadius(Number(event.currentTarget.value))}
                  />
                </label>
              </Show>
            </div>
            <div class="triple">
              <label>
                X
                <input type="number" value={props.x} onInput={(event) => props.onX(Number(event.currentTarget.value))} />
              </label>
              <label>
                Y
                <input type="number" value={props.y} onInput={(event) => props.onY(Number(event.currentTarget.value))} />
              </label>
              <label>
                Z
                <input type="number" value={props.z} onInput={(event) => props.onZ(Number(event.currentTarget.value))} />
              </label>
            </div>
            <div class="split">
              <label>
                X Step
                <input type="number" value={props.xStep} onInput={(event) => props.onXStep(Number(event.currentTarget.value))} />
              </label>
              <label>
                Z Step
                <input type="number" value={props.zStep} onInput={(event) => props.onZStep(Number(event.currentTarget.value))} />
              </label>
            </div>
            <div class="triple">
              <label>
                Pitch
                <input type="number" value={props.pitch} onInput={(event) => props.onPitch(Number(event.currentTarget.value))} />
              </label>
              <label>
                Yaw
                <input type="number" value={props.yaw} onInput={(event) => props.onYaw(Number(event.currentTarget.value))} />
              </label>
              <label>
                Roll
                <input type="number" value={props.roll} onInput={(event) => props.onRoll(Number(event.currentTarget.value))} />
              </label>
            </div>
            <output class={props.invalid ? "footprint bad" : "footprint"}>
              <span>Footprint {props.footprint}ch</span>
              <span>Count {props.normalizedCount}</span>
              <span>Step {props.normalizedAddressStride}ch</span>
              <span>{layoutSummary()}</span>
              <span>End address {props.endAddress}</span>
              <Show when={props.conflictText}>
                {(text) => <span>{text()}</span>}
              </Show>
            </output>
            <Show when={props.warnings.length > 0}>
              <ul class="warnings">
                <For each={props.warnings}>{(warning) => <li>{warning}</li>}</For>
              </ul>
            </Show>
          </div>
        </details>
      </div>
      <Show when={props.invalid}>
        <p class="patchFixtureConflict" role="alert">
          {props.conflictText || `End address ${props.endAddress}`}
        </p>
      </Show>
    </section>
  );
}
