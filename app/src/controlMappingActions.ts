// MIDI/OSC control-mapping action sets, the video-output mapping field catalog, and the
// small domain types/predicates that classify a mapping action. Extracted from App.tsx.
import type { MidiControlAction, OscControlAction } from "./types";
import type { NumericVideoOutputMappingField } from "./videoOutputMapping";

// Re-exported so existing App.tsx imports keep a single source of truth in videoOutputMapping.
export type { NumericVideoOutputMappingField };
export type MappingFixtureFlag = "highlight" | "solo" | "park";
export type FixtureFlagClearKind = MappingFixtureFlag | "all";

export const fixtureFlagClearKinds: FixtureFlagClearKind[] = ["all", "highlight", "solo", "park"];

export const fixtureFlagMappingActions = ["FixtureHighlight", "FixtureSolo", "FixturePark"] as const;
export const groupFlagMappingActions = ["GroupHighlight", "GroupSolo", "GroupPark"] as const;
export const videoLayerMappingActions = [
  "VideoParam",
  "VideoCuePointAdd",
  "VideoCuePointRemove",
  "VideoCuePointJump",
  "VideoCuePointPrevious",
  "VideoCuePointNext",
  "VideoLayerEnabled",
  "VideoLayerSolo",
  "VideoPlay",
  "VideoLoop",
  "VideoLayerFade",
] as const;
export const videoOutputMappingActions = [
  "VideoOutputEnabled",
  "VideoOutputOpacity",
  "VideoOutputFade",
  "VideoOutputMappingField",
  "VideoOutputMappingPreset",
  "VideoOutputBlackout",
] as const;

export const videoOutputMappingFieldOptions = [
  { value: "stage_x", label: "Stage X", low: -10, high: 10 },
  { value: "stage_y", label: "Stage Y", low: -10, high: 10 },
  { value: "stage_z", label: "Stage Z", low: -10, high: 10 },
  { value: "offset_x", label: "Offset X", low: -1, high: 1 },
  { value: "offset_y", label: "Offset Y", low: -1, high: 1 },
  { value: "scale_x", label: "Scale X", low: 0.1, high: 2 },
  { value: "scale_y", label: "Scale Y", low: 0.1, high: 2 },
  { value: "rotation_deg", label: "Rotation", low: -180, high: 180 },
  { value: "aspect_ratio", label: "Aspect Ratio", low: 0.5, high: 3 },
  { value: "lens_distortion", label: "Lens Distortion", low: -1, high: 1 },
  { value: "keystone_x", label: "Keystone X", low: -1, high: 1 },
  { value: "keystone_y", label: "Keystone Y", low: -1, high: 1 },
  { value: "corner_top_left_x", label: "Top Left X", low: -1, high: 1 },
  { value: "corner_top_left_y", label: "Top Left Y", low: -1, high: 1 },
  { value: "corner_top_right_x", label: "Top Right X", low: -1, high: 1 },
  { value: "corner_top_right_y", label: "Top Right Y", low: -1, high: 1 },
  { value: "corner_bottom_right_x", label: "Bottom Right X", low: -1, high: 1 },
  { value: "corner_bottom_right_y", label: "Bottom Right Y", low: -1, high: 1 },
  { value: "corner_bottom_left_x", label: "Bottom Left X", low: -1, high: 1 },
  { value: "corner_bottom_left_y", label: "Bottom Left Y", low: -1, high: 1 },
] as const satisfies readonly {
  value: NumericVideoOutputMappingField;
  label: string;
  low: number;
  high: number;
}[];

export const isFixtureFlagMappingAction = (action: MidiControlAction | OscControlAction) =>
  (fixtureFlagMappingActions as readonly string[]).includes(action);

export const isGroupFlagMappingAction = (action: MidiControlAction | OscControlAction) =>
  (groupFlagMappingActions as readonly string[]).includes(action);

export const isVideoLayerMappingAction = (action: MidiControlAction | OscControlAction) =>
  (videoLayerMappingActions as readonly string[]).includes(action);

export const isVideoOutputMappingAction = (action: MidiControlAction | OscControlAction) =>
  (videoOutputMappingActions as readonly string[]).includes(action);

export const normalizeFixtureFlagClearKind = (value: string | null | undefined): FixtureFlagClearKind => {
  const normalized = (value ?? "").trim().toLowerCase();
  return fixtureFlagClearKinds.includes(normalized as FixtureFlagClearKind)
    ? (normalized as FixtureFlagClearKind)
    : "all";
};

export const videoOutputMappingFieldOption = (field: string | null | undefined) =>
  videoOutputMappingFieldOptions.find((option) => option.value === field) ?? videoOutputMappingFieldOptions[10];
