import { For, Show } from "solid-js";

export type FixtureLayoutMode = "line" | "grid" | "circle";

interface PatchFixtureFormPanelProps {
  armed: boolean;
  /** A server-authoritative PATCH/Repair lane is awaiting its terminal receipt. */
  operationBusy: boolean;
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
  onPatch: () => unknown;
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
    <section class="patchFixtureFormPanel" data-patch-left-form aria-busy={props.operationBusy}>
      <div class="patchFixtureMinimalForm" data-patch-minimal-form>
        <div class="patchFixtureField" data-patch-field="universe">
          <label for="patch-fixture-universe">Universe</label>
          <input
            id="patch-fixture-universe"
            type="number"
            min="0"
            value={props.universe}
            onInput={(event) => props.onUniverse(Number(event.currentTarget.value))}
          />
        </div>
        <div class="patchFixtureField" data-patch-field="address">
          <label for="patch-fixture-address">Address</label>
          <span class="patchFixtureAddressControl">
            <input
              id="patch-fixture-address"
              type="number"
              min="1"
              max="512"
              value={props.address}
              onInput={(event) => props.onAddress(Number(event.currentTarget.value))}
            />
            <button
              type="button"
              class="patchFixtureNextFree"
              onClick={props.onNextFreeAddress}
              disabled={props.operationBusy || props.nextFreeAddress === null}
            >
              Next Free
            </button>
          </span>
        </div>
        <div class="patchFixtureField" data-patch-field="count">
          <label for="patch-fixture-count">Count</label>
          <input
            id="patch-fixture-count"
            type="number"
            min="1"
            max="256"
            value={props.count}
            onInput={(event) => props.onCount(Number(event.currentTarget.value))}
          />
        </div>
        <button class="primary patchFixtureSubmit" onClick={props.onPatch} disabled={props.operationBusy || !props.armed || props.invalid}>
          {props.operationBusy ? "Applying…" : "PATCH"}
        </button>
        {/* #64: placement options only make sense once a profile is armed -
            unarmed they showed a meaningless "0ch / end address 1" footer. */}
        <Show when={props.armed}>
        <details class="patchFixtureOptionsDisclosure" data-patch-placement-disclosure>
          <summary>
            Placement options
            <Show when={props.normalizedCount > 1}>
              <small data-patch-placement-multi>{props.normalizedCount}x</small>
            </Show>
          </summary>
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
        </Show>
      </div>
      <Show when={!props.armed}>
        <p class="patchProfileEmptyState" data-patch-profile-empty>
          Choose a fixture profile above to arm patching.
        </p>
      </Show>
      <Show when={props.armed && props.invalid}>
        <p class="patchFixtureConflict" role="alert">
          {props.conflictText || `End address ${props.endAddress}`}
        </p>
      </Show>
    </section>
  );
}
