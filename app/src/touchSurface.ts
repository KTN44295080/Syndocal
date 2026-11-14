import type {
  TouchControlBinding,
  TouchControlKind,
  TouchControlSummary,
  TouchSurfaceSummary,
} from "./types";

export const touchSurfaceGridColumns = 12;
export const touchSurfaceGridRows = 8;

export const touchControlPalette: ReadonlyArray<{ kind: TouchControlKind; label: string }> = [
  { kind: "Label", label: "Label" },
  { kind: "Image", label: "Image" },
  { kind: "Button", label: "Button" },
  { kind: "Fader", label: "Fader" },
  { kind: "Dial", label: "Dial" },
  { kind: "IncrementalWheel", label: "Incremental Wheel" },
  { kind: "ColorWheel", label: "Color Wheel" },
  { kind: "XyGrid", label: "XY Grid" },
];

const defaultBindingForKind = (kind: TouchControlKind): TouchControlBinding | null => {
  switch (kind) {
    case "Dial":
    case "IncrementalWheel":
      return { kind: "selected_fixture_attribute", attribute: "Dimmer" };
    case "ColorWheel":
      return { kind: "selected_fixture_color" };
    case "XyGrid":
      return { kind: "selected_fixture_pan_tilt", pan_attribute: "Pan", tilt_attribute: "Tilt" };
    default:
      return null;
  }
};

const defaultSizeForKind = (kind: TouchControlKind) => {
  switch (kind) {
    case "Label":
      return { w: 2, h: 1 };
    case "Image":
      return { w: 2, h: 2 };
    case "Fader":
      return { w: 1, h: 4 };
    case "ColorWheel":
    case "XyGrid":
      return { w: 3, h: 3 };
    default:
      return { w: 2, h: 2 };
  }
};

export const createTouchControl = (
  id: number,
  kind: TouchControlKind,
  x = 0,
  y = 0,
): TouchControlSummary => ({
  id,
  kind,
  x,
  y,
  ...defaultSizeForKind(kind),
  label: touchControlPalette.find((entry) => entry.kind === kind)?.label ?? kind,
  binding: defaultBindingForKind(kind),
});

/** Grid columns the Default Desk may use; the Scene panel owns the rest. */
const defaultDeskColumns = touchSurfaceGridColumns - 3;

/**
 * The Default Desk holds only what the rest of Control does not: one
 * submaster per fixture group that exists. Fixture attributes live in the
 * Faders panel, Scenes in the Scene panel and masters/blackouts in the top
 * bar, so the desk repeats none of them (owner 2026-11-11: no clutter).
 */
export const defaultTouchSurface = (
  groups: ReadonlyArray<{ id: string; label: string }> = [],
): TouchSurfaceSummary => ({
  pages: [
    {
      id: 1,
      label: "Default Desk",
      controls: groups.slice(0, defaultDeskColumns * 2).map((group, index) => ({
        ...createTouchControl(index + 1, "Fader", index % defaultDeskColumns, Math.floor(index / defaultDeskColumns) * 4),
        w: 1,
        h: 4,
        label: group.label,
        binding: { kind: "group_submaster" as const, group_id: group.id },
      })),
    },
  ],
});

export const effectiveTouchSurface = (
  surface: TouchSurfaceSummary | undefined,
  groups: ReadonlyArray<{ id: string; label: string }> = [],
): TouchSurfaceSummary => withoutTopbarDuplicateTouchControls(
  surface && surface.pages.length > 0 ? surface : defaultTouchSurface(groups),
);

const topbarDuplicateBindingKinds = new Set<TouchControlBinding["kind"]>([
  "cue_next",
  "lighting_master",
  "video_master",
]);

const duplicatesTopbar = (control: TouchControlSummary) =>
  Boolean(control.binding && topbarDuplicateBindingKinds.has(control.binding.kind))
  || (control.kind === "Button" && control.label.trim().toUpperCase() === "GO");

export const withoutTopbarDuplicateTouchControls = (
  surface: TouchSurfaceSummary,
): TouchSurfaceSummary => ({
  pages: surface.pages.map((page) => ({
    ...page,
    controls: page.controls.filter((control) => !duplicatesTopbar(control)),
  })),
});

export const nextTouchPageId = (surface: TouchSurfaceSummary) =>
  Math.max(0, ...surface.pages.map((page) => page.id)) + 1;

export const nextTouchControlId = (surface: TouchSurfaceSummary) =>
  Math.max(0, ...surface.pages.flatMap((page) => page.controls.map((control) => control.id))) + 1;
