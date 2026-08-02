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

export const defaultTouchSurface = (
  cues: ReadonlyArray<{ id: number; label: string }> = [],
): TouchSurfaceSummary => {
  const visibleCues = cues.slice(0, 4);
  return {
    pages: [
      {
      id: 1,
      label: "Default Desk",
      controls: [
        { ...createTouchControl(1, "Label", 0, 0), label: "SHOW CONTROL" },
        { ...createTouchControl(2, "Image", 0, 1), h: 1, label: "Touch Stage" },
        {
          ...createTouchControl(3, "Button", 2, 0),
          label: "BACK",
          binding: { kind: "cue_previous" },
        },
        {
          ...createTouchControl(5, "Button", 4, 0),
          label: "PAUSE",
          binding: { kind: "cue_fade_pause" },
        },
        {
          ...createTouchControl(6, "Fader", 9, 0),
          w: 3,
          h: 2,
          label: "FRONT",
          binding: { kind: "group_submaster", group_id: "front" },
        },
        {
          ...createTouchControl(8, "Button", 6, 0),
          w: 3,
          label: "ALL BO",
          binding: { kind: "all_blackout" },
        },
        {
          ...createTouchControl(9, "Dial", 0, 2),
          h: 4,
          label: "DIMMER",
          binding: { kind: "selected_fixture_attribute", attribute: "Dimmer" },
        },
        {
          ...createTouchControl(10, "IncrementalWheel", 2, 2),
          h: 4,
          label: "FINE",
          binding: { kind: "selected_fixture_attribute", attribute: "Dimmer" },
        },
        {
          ...createTouchControl(11, "ColorWheel", 4, 2),
          w: 2,
          h: 4,
          label: "COLOR",
          binding: { kind: "selected_fixture_color" },
        },
        {
          ...createTouchControl(12, "XyGrid", 6, 2),
          h: 6,
          label: "POSITION",
          binding: { kind: "selected_fixture_pan_tilt", pan_attribute: "Pan", tilt_attribute: "Tilt" },
        },
        {
          ...createTouchControl(13, "Button", 9, 2),
          w: 3,
          h: 3,
          label: "DMX BO",
          binding: { kind: "blackout" },
        },
        {
          ...createTouchControl(14, "Button", 9, 5),
          w: 3,
          h: 3,
          label: "VIDEO BO",
          binding: { kind: "video_blackout" },
        },
        ...visibleCues.map((cue, index) => {
          const start = Math.floor(index * 6 / visibleCues.length);
          const end = Math.floor((index + 1) * 6 / visibleCues.length);
          return {
            ...createTouchControl(15 + index, "Button", start, 6),
            w: end - start,
            label: cue.label,
            binding: { kind: "cue" as const, cue_id: cue.id },
          };
        }),
      ],
      },
    ],
  };
};

export const effectiveTouchSurface = (
  surface: TouchSurfaceSummary | undefined,
  cues: ReadonlyArray<{ id: number; label: string }> = [],
): TouchSurfaceSummary => withoutTopbarDuplicateTouchControls(
  surface && surface.pages.length > 0 ? surface : defaultTouchSurface(cues),
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
