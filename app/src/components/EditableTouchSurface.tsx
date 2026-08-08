import { For, Show, createMemo, createSignal } from "solid-js";
import { VerticalFaderInput } from "./VerticalFaderInput";

import {
  createTouchControl,
  effectiveTouchSurface,
  nextTouchControlId,
  nextTouchPageId,
  touchControlPalette,
  touchSurfaceGridColumns,
  touchSurfaceGridRows,
} from "../touchSurface";
import type {
  CueLiveDirection,
  CueSummary,
  EngineSnapshot,
  TouchControlBinding,
  TouchControlSummary,
  TouchPageSummary,
  TouchSurfaceSummary,
} from "../types";
import { authoredCueLiveModifier } from "../cueLiveModifier";
import { CueLiveModifierStrip } from "./CueLiveModifierStrip";

type TouchSurfaceMode = "edit" | "live";

interface EditableTouchSurfaceProps {
  snapshot: EngineSnapshot;
  surface?: TouchSurfaceSummary;
  selectedFixtureId?: number | null;
  selectedGroupId?: string | null;
  colorPalette?: readonly string[];
  onSurfaceChange: (surface: TouchSurfaceSummary) => void | Promise<void>;
  onTrigger: (binding: TouchControlBinding) => void | Promise<void>;
  onValue: (binding: TouchControlBinding, value: number) => void | Promise<void>;
  onColor: (binding: TouchControlBinding, color: string) => void | Promise<void>;
  onXy: (binding: TouchControlBinding, x: number, y: number) => void | Promise<void>;
  onReleaseCue?: (cueId: number) => void | Promise<void>;
  onSetCueLiveModifier?: (
    cueId: number,
    speed: number,
    size: number,
    phase: number,
    direction: CueLiveDirection,
    segment: number,
  ) => void | Promise<void>;
  onClearCueLiveModifier?: (cueId: number) => void | Promise<void>;
}

interface TouchDragState {
  pointerId: number;
  control: TouchControlSummary;
  startClientX: number;
  startClientY: number;
  resize: boolean;
}

const bindingOptions: ReadonlyArray<{ value: string; label: string }> = [
  { value: "", label: "Unassigned" },
  { value: "cue_previous", label: "Previous cue" },
  { value: "cue_fade_pause", label: "Cue fade pause" },
  { value: "cue", label: "Cue" },
  { value: "blackout", label: "DMX Blackout" },
  { value: "video_blackout", label: "Video Blackout" },
  { value: "all_blackout", label: "All Blackout" },
  { value: "fixture_attribute", label: "Fixture attribute" },
  { value: "group_attribute", label: "Group attribute" },
  { value: "fixture_color", label: "Fixture color" },
  { value: "group_color", label: "Group color" },
  { value: "fixture_pan_tilt", label: "Fixture Pan/Tilt" },
  { value: "group_pan_tilt", label: "Group Pan/Tilt" },
  { value: "group_select", label: "Group select" },
  { value: "group_submaster", label: "Group submaster" },
  { value: "tap_tempo", label: "Tap tempo" },
  { value: "selected_fixture_attribute", label: "Selected fixture attribute" },
  { value: "selected_fixture_color", label: "Selected fixture color" },
  { value: "selected_fixture_pan_tilt", label: "Selected fixture Pan/Tilt" },
];

const clamp01 = (value: number) => Math.max(0, Math.min(1, value));

const bindingKey = (binding: TouchControlBinding | null | undefined) => binding?.kind ?? "";

const controlGridStyle = (control: TouchControlSummary) => ({
  "grid-column": `${control.x + 1} / span ${control.w}`,
  "grid-row": `${control.y + 1} / span ${control.h}`,
});

const groupIdsFromSnapshot = (snapshot: EngineSnapshot) =>
  [...new Set(snapshot.fixtures.flatMap((fixture) => fixture.group_ids))].sort((left, right) =>
    left.localeCompare(right),
  );

const attributesForFixture = (snapshot: EngineSnapshot, fixtureId: number) =>
  snapshot.fixtures.find((fixture) => fixture.id === fixtureId)?.controls.map((control) => control.attribute) ?? [];

const normalizedFixtureAttribute = (
  snapshot: EngineSnapshot,
  fixtureId: number,
  attribute: string,
) => {
  const value = snapshot.fixtures
    .find((fixture) => fixture.id === fixtureId)
    ?.attribute_values.find((entry) => entry.attribute === attribute)?.value;
  return value === undefined ? 0 : clamp01(value / 65_535);
};

export function EditableTouchSurface(props: EditableTouchSurfaceProps) {
  const [mode, setMode] = createSignal<TouchSurfaceMode>("live");
  const [selectedPageId, setSelectedPageId] = createSignal(1);
  const [selectedControlId, setSelectedControlId] = createSignal<number | null>(null);
  const [previewControl, setPreviewControl] = createSignal<TouchControlSummary | null>(null);
  const [localValues, setLocalValues] = createSignal<Record<number, number>>({});
  const [localXy, setLocalXy] = createSignal<Record<number, { x: number; y: number }>>({});
  const [liveActivations, setLiveActivations] = createSignal<Record<number, boolean>>({});
  let gridElement: HTMLDivElement | undefined;
  let dragState: TouchDragState | null = null;

  const surface = createMemo(() => effectiveTouchSurface(props.surface, props.snapshot.cues));
  const activePage = createMemo<TouchPageSummary | undefined>(() =>
    surface().pages.find((page) => page.id === selectedPageId()) ?? surface().pages[0],
  );
  const selectedControl = createMemo(() =>
    activePage()?.controls.find((control) => control.id === selectedControlId()),
  );
  const groupIds = createMemo(() => groupIdsFromSnapshot(props.snapshot));

  const persistSurface = (next: TouchSurfaceSummary) => {
    void props.onSurfaceChange(next);
  };

  const updateActivePage = (update: (page: TouchPageSummary) => TouchPageSummary) => {
    const page = activePage();
    if (!page) return;
    persistSurface({
      pages: surface().pages.map((candidate) => (candidate.id === page.id ? update(candidate) : candidate)),
    });
  };

  const updateControl = (nextControl: TouchControlSummary) => {
    updateActivePage((page) => ({
      ...page,
      controls: page.controls.map((control) => (control.id === nextControl.id ? nextControl : control)),
    }));
  };

  const addControl = (kind: TouchControlSummary["kind"]) => {
    const page = activePage();
    if (!page) return;
    const id = nextTouchControlId(surface());
    const index = page.controls.length;
    const control = createTouchControl(
      id,
      kind,
      Math.min(touchSurfaceGridColumns - 2, (index % 6) * 2),
      Math.min(touchSurfaceGridRows - 2, Math.floor(index / 6) * 2),
    );
    setSelectedControlId(id);
    updateActivePage((candidate) => ({ ...candidate, controls: [...candidate.controls, control] }));
  };

  const addPage = () => {
    const id = nextTouchPageId(surface());
    setSelectedPageId(id);
    setSelectedControlId(null);
    persistSurface({ pages: [...surface().pages, { id, label: `Page ${id}`, controls: [] }] });
  };

  const removePage = () => {
    const page = activePage();
    if (!page || surface().pages.length <= 1) return;
    if (!globalThis.confirm(`Delete Touch page "${page.label}"?`)) return;
    const pages = surface().pages.filter((candidate) => candidate.id !== page.id);
    setSelectedPageId(pages[0]?.id ?? 1);
    setSelectedControlId(null);
    persistSurface({ pages });
  };

  const removeSelectedControl = () => {
    const control = selectedControl();
    if (!control || !globalThis.confirm(`Remove Touch control "${control.label}"?`)) return;
    updateActivePage((page) => ({
      ...page,
      controls: page.controls.filter((candidate) => candidate.id !== control.id),
    }));
    setSelectedControlId(null);
  };

  const beginDrag = (event: PointerEvent, control: TouchControlSummary, resize: boolean) => {
    if (mode() !== "edit") return;
    event.preventDefault();
    event.stopPropagation();
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
    setSelectedControlId(control.id);
    setPreviewControl(control);
    dragState = {
      pointerId: event.pointerId,
      control,
      startClientX: event.clientX,
      startClientY: event.clientY,
      resize,
    };
  };

  const moveDrag = (event: PointerEvent) => {
    if (!dragState || dragState.pointerId !== event.pointerId || !gridElement) return;
    const bounds = gridElement.getBoundingClientRect();
    const deltaX = Math.round(((event.clientX - dragState.startClientX) / bounds.width) * touchSurfaceGridColumns);
    const deltaY = Math.round(((event.clientY - dragState.startClientY) / bounds.height) * touchSurfaceGridRows);
    const original = dragState.control;
    if (dragState.resize) {
      setPreviewControl({
        ...original,
        w: Math.max(1, Math.min(touchSurfaceGridColumns - original.x, original.w + deltaX)),
        h: Math.max(1, Math.min(touchSurfaceGridRows - original.y, original.h + deltaY)),
      });
    } else {
      setPreviewControl({
        ...original,
        x: Math.max(0, Math.min(touchSurfaceGridColumns - original.w, original.x + deltaX)),
        y: Math.max(0, Math.min(touchSurfaceGridRows - original.h, original.y + deltaY)),
      });
    }
  };

  const endDrag = (event: PointerEvent) => {
    if (!dragState || dragState.pointerId !== event.pointerId) return;
    const next = previewControl();
    dragState = null;
    setPreviewControl(null);
    if (next) updateControl(next);
  };

  const displayedControl = (control: TouchControlSummary) =>
    previewControl()?.id === control.id ? previewControl()! : control;

  const selectedFixtureId = () =>
    props.selectedFixtureId ?? props.snapshot.fixtures[0]?.id ?? 0;

  const valueForBinding = (binding: TouchControlBinding | null | undefined) => {
    if (!binding) return 0;
    switch (binding.kind) {
      case "fixture_attribute":
        return normalizedFixtureAttribute(props.snapshot, binding.fixture_id, binding.attribute);
      case "group_attribute": {
        const fixture = props.snapshot.fixtures.find((candidate) => candidate.group_ids.includes(binding.group_id));
        return fixture ? normalizedFixtureAttribute(props.snapshot, fixture.id, binding.attribute) : 0;
      }
      case "selected_fixture_attribute":
        return normalizedFixtureAttribute(props.snapshot, selectedFixtureId(), binding.attribute);
      case "lighting_master":
        return props.snapshot.lighting_master;
      case "video_master":
        return props.snapshot.video.master_opacity;
      case "group_submaster":
        return props.snapshot.submasters.find((entry) => entry.group_id === binding.group_id)?.level ?? 1;
      case "blackout":
        return props.snapshot.blackout ? 1 : 0;
      case "video_blackout":
        return props.snapshot.video.blackout ? 1 : 0;
      case "all_blackout":
        return props.snapshot.blackout && props.snapshot.video.blackout ? 1 : 0;
      default:
        return 0;
    }
  };

  const controlValue = (control: TouchControlSummary) =>
    localValues()[control.id] ?? valueForBinding(control.binding);

  const setControlValue = (control: TouchControlSummary, value: number) => {
    if (mode() !== "live" || !control.binding) return;
    const next = clamp01(value);
    setLocalValues((current) => ({ ...current, [control.id]: next }));
    setLiveActivations((current) => ({ ...current, [control.id]: true }));
    void props.onValue(control.binding, next);
  };

  const triggerControl = (control: TouchControlSummary) => {
    if (mode() !== "live" || !control.binding) return;
    setLiveActivations((current) => ({ ...current, [control.id]: true }));
    void props.onTrigger(control.binding);
  };

  const setControlColor = (control: TouchControlSummary, color: string) => {
    if (mode() !== "live" || !control.binding) return;
    setLiveActivations((current) => ({ ...current, [control.id]: true }));
    void props.onColor(control.binding, color);
  };

  const setControlXy = (event: PointerEvent, control: TouchControlSummary) => {
    if (mode() !== "live" || !control.binding) return;
    const bounds = (event.currentTarget as HTMLElement).getBoundingClientRect();
    const x = clamp01((event.clientX - bounds.left) / bounds.width);
    const y = clamp01(1 - (event.clientY - bounds.top) / bounds.height);
    setLocalXy((current) => ({ ...current, [control.id]: { x, y } }));
    setLiveActivations((current) => ({ ...current, [control.id]: true }));
    void props.onXy(control.binding, x, y);
  };

  const bindingFromOption = (option: string): TouchControlBinding | null => {
    const fixtureId = selectedFixtureId();
    const groupId = groupIds()[0] ?? "front";
    const cueId = props.snapshot.cues[0]?.id ?? 1;
    switch (option) {
      case "fixture_attribute":
        return { kind: option, fixture_id: fixtureId, attribute: "Dimmer" };
      case "group_attribute":
        return { kind: option, group_id: groupId, attribute: "Dimmer" };
      case "fixture_color":
        return { kind: option, fixture_id: fixtureId };
      case "group_color":
        return { kind: option, group_id: groupId };
      case "fixture_pan_tilt":
        return { kind: option, fixture_id: fixtureId, pan_attribute: "Pan", tilt_attribute: "Tilt" };
      case "group_pan_tilt":
        return { kind: option, group_id: groupId, pan_attribute: "Pan", tilt_attribute: "Tilt" };
      case "cue":
        return { kind: option, cue_id: cueId };
      case "group_submaster":
        return { kind: option, group_id: groupId };
      case "group_select":
        return { kind: option, group_id: groupId };
      case "selected_fixture_attribute":
        return { kind: option, attribute: "Dimmer" };
      case "selected_fixture_pan_tilt":
        return { kind: option, pan_attribute: "Pan", tilt_attribute: "Tilt" };
      case "cue_next":
      case "cue_previous":
      case "cue_fade_pause":
      case "tap_tempo":
      case "lighting_master":
      case "video_master":
      case "blackout":
      case "video_blackout":
      case "all_blackout":
      case "selected_fixture_color":
        return { kind: option };
      default:
        return null;
    }
  };

  const replaceBinding = (binding: TouchControlBinding | null) => {
    const control = selectedControl();
    if (control) updateControl({ ...control, binding });
  };

  const boundCue = (control: TouchControlSummary): CueSummary | null => {
    if (control.binding?.kind !== "cue") return null;
    const cueId = control.binding.cue_id;
    return props.snapshot.cues.find((cue) => cue.id === cueId) ?? null;
  };

  const boundFlashCue = (control: TouchControlSummary): CueSummary | null => {
    const cue = boundCue(control);
    return cue && authoredCueLiveModifier(cue).flash ? cue : null;
  };

  const buttonLabel = (control: TouchControlSummary) => {
    switch (control.binding?.kind) {
      case "cue_fade_pause":
        return props.snapshot.active_fade?.paused ? "Resume" : control.label;
      case "blackout":
        return props.snapshot.blackout ? "Clear DMX BO" : control.label;
      case "video_blackout":
        return props.snapshot.video.blackout ? "Clear Video BO" : control.label;
      case "all_blackout":
        return props.snapshot.blackout && props.snapshot.video.blackout ? "All Clear" : control.label;
      default:
        return control.label;
    }
  };

  const blackoutBinding = (control: TouchControlSummary) =>
    control.binding?.kind === "blackout"
    || control.binding?.kind === "video_blackout"
    || control.binding?.kind === "all_blackout";

  const buttonPressed = (control: TouchControlSummary): boolean | undefined => {
    switch (control.binding?.kind) {
      case "cue_fade_pause":
        return props.snapshot.active_fade?.paused ?? false;
      case "group_select":
        return props.selectedGroupId === control.binding.group_id;
      case "blackout":
      case "video_blackout":
      case "all_blackout":
        return controlValue(control) >= 1;
      default:
        return undefined;
    }
  };

  const renderLiveControl = (control: TouchControlSummary) => {
    const disabled = mode() !== "live" || !control.binding;
    switch (control.kind) {
      case "Label":
        return <strong class="touchPlacedLabel">{control.label}</strong>;
      case "Image":
        return <div class="touchPlacedImage" role="img" aria-label={control.label}><span aria-hidden="true">▧</span><strong>{control.label}</strong></div>;
      case "Button": {
        const flashCue = () => (mode() === "live" ? boundFlashCue(control) : null);
        const flashRelease = () => {
          const cue = flashCue();
          if (cue) void props.onReleaseCue?.(cue.id);
        };
        return (
          <button
            class="touchPlacedButton"
            classList={{
              flash: Boolean(flashCue()),
              safety: blackoutBinding(control),
              engaged: blackoutBinding(control) && controlValue(control) >= 1,
            }}
            disabled={disabled}
            aria-pressed={buttonPressed(control)}
            data-touch-live-activated={liveActivations()[control.id] ? "true" : "false"}
            data-touch-cue-pad={boundCue(control)?.id}
            data-touch-flash-cue={flashCue()?.id}
            onPointerDown={(event) => {
              const cue = flashCue();
              if (!cue) return;
              // Momentary flash pad: press activates the scene, release always
              // releases it; the click handler must not re-trigger.
              event.preventDefault();
              event.currentTarget.setPointerCapture(event.pointerId);
              triggerControl(control);
            }}
            onPointerUp={flashRelease}
            onPointerCancel={flashRelease}
            onKeyDown={(event) => {
              if (!flashCue() || event.repeat) return;
              if (event.key === " " || event.key === "Enter") {
                event.preventDefault();
                triggerControl(control);
              }
            }}
            onKeyUp={(event) => {
              if (!flashCue()) return;
              if (event.key === " " || event.key === "Enter") {
                event.preventDefault();
                flashRelease();
              }
            }}
            onClick={(event) => {
              if (flashCue()) {
                event.preventDefault();
                return;
              }
              triggerControl(control);
            }}
          >
            {buttonLabel(control)}
          </button>
        );
      }
      case "Fader":
        return (
          <label class="touchPlacedFader">
            <span>{control.label}</span>
            <VerticalFaderInput
              chromeClass="touchPlacedVerticalFaderChrome"
              inputClass="touchPlacedVerticalFaderInput"
              aria-label={control.label}
              min="0"
              max="1"
               step="0.01"
               value={controlValue(control)}
               disabled={disabled}
               data-touch-live-activated={liveActivations()[control.id] ? "true" : "false"}
               onInput={(event) => setControlValue(control, Number(event.currentTarget.value))}
            />
            <strong>{Math.round(controlValue(control) * 100)}%</strong>
          </label>
        );
      case "Dial":
        return (
          <label class="touchPlacedDial">
            <span>{control.label}</span>
            <input
              aria-label={control.label}
              type="range"
              min="0"
              max="1"
               step="0.01"
               value={controlValue(control)}
               disabled={disabled}
               data-touch-live-activated={liveActivations()[control.id] ? "true" : "false"}
               onInput={(event) => setControlValue(control, Number(event.currentTarget.value))}
            />
            <strong>{Math.round(controlValue(control) * 100)}%</strong>
          </label>
        );
      case "IncrementalWheel":
        return (
           <div
             class="touchPlacedIncremental"
             data-touch-live-activated={liveActivations()[control.id] ? "true" : "false"}
             onWheel={(event) => {
              event.preventDefault();
              setControlValue(control, controlValue(control) + (event.deltaY < 0 ? 0.01 : -0.01));
            }}
          >
            <span>{control.label}</span>
            <div>
              <button aria-label={`Decrease ${control.label}`} disabled={disabled} onClick={() => setControlValue(control, controlValue(control) - 0.01)}>−</button>
              <strong>{Math.round(controlValue(control) * 100)}%</strong>
              <button aria-label={`Increase ${control.label}`} disabled={disabled} onClick={() => setControlValue(control, controlValue(control) + 0.01)}>+</button>
            </div>
          </div>
        );
      case "ColorWheel":
        return (
          <div class="touchPlacedColor">
            <span>{control.label}</span>
            <input
              aria-label={control.label}
              type="color"
              value="#ff7a00"
              disabled={disabled}
              data-touch-live-activated={liveActivations()[control.id] ? "true" : "false"}
              onInput={(event) => setControlColor(control, event.currentTarget.value)}
            />
            <Show when={props.colorPalette?.length}>
              <div class="touchPlacedColorPalette" aria-label={`${control.label} palette`}>
                <For each={props.colorPalette}>
                  {(color) => (
                    <button
                      type="button"
                      class="colorSwatch"
                      style={{ "background-color": color }}
                      aria-label={`Set ${control.label} to ${color}`}
                      disabled={disabled}
                      onClick={() => setControlColor(control, color)}
                    />
                  )}
                </For>
              </div>
            </Show>
          </div>
        );
      case "XyGrid": {
        const point = () => localXy()[control.id] ?? { x: 0.5, y: 0.5 };
        return (
          <button
            class="touchPlacedXy"
            aria-label={control.label}
            disabled={disabled}
            data-touch-live-activated={liveActivations()[control.id] ? "true" : "false"}
            onPointerDown={(event) => setControlXy(event, control)}
            onPointerMove={(event) => {
              if (event.buttons === 1) setControlXy(event, control);
            }}
          >
            <span>{control.label}</span>
            <i style={{ left: `${point().x * 100}%`, top: `${(1 - point().y) * 100}%` }} />
          </button>
        );
      }
    }
  };

  return (
    <section
      class="panel touchPanel touchSurfacePanel"
      aria-label="Editable Touch surface"
      data-touch-surface
      data-touch-mode={mode()}
      data-touch-default-preset={props.surface?.pages.length ? "false" : "true"}
    >
      <div class="touchSurfaceToolbar">
        <div class="touchSurfaceModeToggle" aria-label="Touch surface mode">
          <button
            class={mode() === "edit" ? "active" : ""}
            aria-pressed={mode() === "edit"}
            onClick={() => setMode("edit")}
          >
            EDIT
          </button>
          <button
            class={mode() === "live" ? "active" : ""}
            aria-pressed={mode() === "live"}
            onClick={() => {
              setMode("live");
              setSelectedControlId(null);
            }}
          >
            LIVE
          </button>
        </div>
        <div class="touchPageTabs" aria-label="Touch pages">
          <For each={surface().pages}>
            {(page) => (
              <button
                 class={activePage()?.id === page.id ? "active" : ""}
                 aria-pressed={activePage()?.id === page.id}
                 data-no-localize
                onClick={() => {
                  setSelectedPageId(page.id);
                  setSelectedControlId(null);
                }}
              >
                {page.label}
              </button>
            )}
          </For>
          <button class="touchPageAdd" aria-label="Add Touch page" onClick={addPage}>+</button>
          <button
            class="touchPageRemove danger"
            aria-label="Delete Touch page"
            disabled={surface().pages.length <= 1}
            onClick={removePage}
          >
            −
          </button>
        </div>
        <span class="touchSurfaceStatus">{mode() === "edit" ? "Place, resize and assign" : "Direct control"}</span>
      </div>

      <Show when={mode() === "edit"}>
        <div class="touchControlPalette" aria-label="ADD CONTROLS">
          <strong>ADD CONTROLS</strong>
          <For each={touchControlPalette}>
            {(entry) => <button onClick={() => addControl(entry.kind)}>{entry.label}</button>}
          </For>
        </div>
      </Show>

      <div
        ref={gridElement}
        class="touchSurfaceGrid"
        data-touch-page={activePage()?.label ?? ""}
        onPointerMove={moveDrag}
        onPointerUp={endDrag}
        onPointerCancel={endDrag}
      >
        <For each={activePage()?.controls ?? []}>
          {(control) => {
            const displayed = () => displayedControl(control);
            return (
              <div
                class={`touchPlacedControl kind-${control.kind} ${selectedControlId() === control.id ? "selected" : ""}`}
                style={controlGridStyle(displayed())}
                 data-touch-control={control.id}
                 data-touch-kind={control.kind}
                 data-touch-binding={control.binding?.kind ?? "unassigned"}
                 data-no-localize
              >
                {renderLiveControl(control)}
                <Show when={mode() === "edit"}>
                  <button
                    class="touchEditMoveHandle"
                    aria-label={`Move ${control.label}`}
                    onPointerDown={(event) => beginDrag(event, displayed(), false)}
                  >
                    ↕
                  </button>
                  <button
                    class="touchEditResizeHandle"
                    aria-label={`Resize ${control.label}`}
                    onPointerDown={(event) => beginDrag(event, displayed(), true)}
                  >
                    ↘
                  </button>
                </Show>
              </div>
            );
          }}
        </For>
        <Show when={(activePage()?.controls.length ?? 0) === 0}>
          <div class="touchSurfaceEmpty">
            <strong>No controls on this page</strong>
            <span>Switch to EDIT and add a control.</span>
          </div>
        </Show>
      </div>

      <Show when={mode() === "edit" && selectedControl()}>
        {(controlAccessor) => {
          const control = () => controlAccessor();
          const binding = () => control().binding;
          const fixtureBinding = () => {
            const current = binding();
            return current?.kind === "fixture_attribute"
              || current?.kind === "fixture_color"
              || current?.kind === "fixture_pan_tilt"
              ? current
              : null;
          };
          const groupBinding = () => {
            const current = binding();
            return current?.kind === "group_attribute"
              || current?.kind === "group_color"
              || current?.kind === "group_pan_tilt"
              || current?.kind === "group_select"
              || current?.kind === "group_submaster"
              ? current
              : null;
          };
          return (
            <aside class="touchBindingEditor" aria-label="Touch control assignment">
              <div>
                <strong>CONTROL</strong>
                <button class="danger" onClick={removeSelectedControl}>Remove</button>
              </div>
              <label>
                Label
                <input
                  value={control().label}
                  onChange={(event) => updateControl({ ...control(), label: event.currentTarget.value.trim() || control().kind })}
                />
              </label>
              <label>
                Binding
                <select
                  value={bindingKey(binding())}
                  onChange={(event) => replaceBinding(bindingFromOption(event.currentTarget.value))}
                >
                  <For each={bindingOptions}>{(option) => <option value={option.value}>{option.label}</option>}</For>
                </select>
              </label>
              <Show when={fixtureBinding()}>
                {(currentAccessor) => (
                  <label>
                    Fixture
                    <select
                      value={currentAccessor().fixture_id}
                      onChange={(event) => replaceBinding({ ...currentAccessor(), fixture_id: Number(event.currentTarget.value) })}
                    >
                      <For each={props.snapshot.fixtures}>
                        {(fixture) => <option value={fixture.id} data-no-localize>{fixture.label}</option>}
                      </For>
                    </select>
                  </label>
                )}
              </Show>
              <Show when={groupBinding()}>
                {(currentAccessor) => (
                  <label>
                    Group
                    <select
                      value={currentAccessor().group_id}
                      onChange={(event) => replaceBinding({ ...currentAccessor(), group_id: event.currentTarget.value })}
                    >
                      <For each={groupIds()}>{(groupId) => <option value={groupId}>{groupId}</option>}</For>
                    </select>
                  </label>
                )}
              </Show>
              <Show when={binding()?.kind === "fixture_attribute"}>
                <label>
                  Attribute
                  <select
                    value={(binding() as Extract<TouchControlBinding, { kind: "fixture_attribute" }>).attribute}
                    onChange={(event) => replaceBinding({
                      ...(binding() as Extract<TouchControlBinding, { kind: "fixture_attribute" }>),
                      attribute: event.currentTarget.value,
                    })}
                  >
                    <For each={attributesForFixture(
                      props.snapshot,
                      (binding() as Extract<TouchControlBinding, { kind: "fixture_attribute" }>).fixture_id,
                    )}>
                      {(attribute) => <option value={attribute}>{attribute}</option>}
                    </For>
                  </select>
                </label>
              </Show>
              <Show when={binding()?.kind === "group_attribute" || binding()?.kind === "selected_fixture_attribute"}>
                <label>
                  Attribute
                  <input
                    value={(binding() as Extract<TouchControlBinding, { attribute: string }>).attribute}
                    onChange={(event) => replaceBinding({
                      ...(binding() as Extract<TouchControlBinding, { attribute: string }>),
                      attribute: event.currentTarget.value,
                    })}
                  />
                </label>
              </Show>
              <Show when={binding()?.kind === "cue"}>
                <label>
                  Cue
                  <select
                    value={(binding() as Extract<TouchControlBinding, { kind: "cue" }>).cue_id}
                    onChange={(event) => replaceBinding({ kind: "cue", cue_id: Number(event.currentTarget.value) })}
                  >
                    <For each={props.snapshot.cues}>{(cue) => <option value={cue.id} data-no-localize>{cue.label}</option>}</For>
                  </select>
                </label>
              </Show>
              <Show when={binding()?.kind.includes("pan_tilt")}>
                <div class="touchBindingPair">
                  <label>
                    Pan attribute
                    <input
                      value={(binding() as Extract<TouchControlBinding, { pan_attribute: string }>).pan_attribute}
                      onChange={(event) => replaceBinding({
                        ...(binding() as Extract<TouchControlBinding, { pan_attribute: string }>),
                        pan_attribute: event.currentTarget.value,
                      })}
                    />
                  </label>
                  <label>
                    Tilt attribute
                    <input
                      value={(binding() as Extract<TouchControlBinding, { tilt_attribute: string }>).tilt_attribute}
                      onChange={(event) => replaceBinding({
                        ...(binding() as Extract<TouchControlBinding, { tilt_attribute: string }>),
                        tilt_attribute: event.currentTarget.value,
                      })}
                    />
                  </label>
                </div>
              </Show>
            </aside>
          );
        }}
      </Show>
      <Show
        when={
          mode() === "live" && props.onSetCueLiveModifier && props.onClearCueLiveModifier
            ? props.snapshot.cues.find((cue) => cue.id === props.snapshot.active_cue_id) ?? null
            : null
        }
      >
        {(cue) => (
          <CueLiveModifierStrip
            touch
            cue={cue()}
            liveStates={props.snapshot.cue_live_modifiers}
            onSetCueLiveModifier={props.onSetCueLiveModifier!}
            onClearCueLiveModifier={props.onClearCueLiveModifier!}
          />
        )}
      </Show>
    </section>
  );
}
