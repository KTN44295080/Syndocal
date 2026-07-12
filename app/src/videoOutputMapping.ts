import type {
  Vec3,
  VideoOutputAspectMode,
  VideoOutputMapping,
  VideoOutputSummary,
} from "./types";

export type NumericVideoOutputMappingField = Exclude<
  keyof VideoOutputMapping,
  "aspect_mode" | "mask_invert" | "mask_points" | "bitmap_mask_luma_words"
>;

export const defaultVideoOutputMapping: VideoOutputMapping = {
  stage_x: 0,
  stage_y: 0,
  stage_z: 0,
  offset_x: 0,
  offset_y: 0,
  scale_x: 1,
  scale_y: 1,
  rotation_deg: 0,
  aspect_ratio: 1,
  aspect_mode: "Stretch",
  lens_distortion: 0,
  edge_blend_left: 0,
  edge_blend_right: 0,
  edge_blend_top: 0,
  edge_blend_bottom: 0,
  edge_blend_gamma: 2.2,
  black_level: 0,
  mask_point_count: 0,
  mask_invert: false,
  mask_softness: 0,
  mask_points: Array.from({ length: 8 }, () => ({ x: 0, y: 0 })),
  bitmap_mask_width: 0,
  bitmap_mask_height: 0,
  bitmap_mask_luma_words: Array.from({ length: 32 }, () => 0),
  keystone_x: 0,
  keystone_y: 0,
  corner_top_left_x: 0,
  corner_top_left_y: 0,
  corner_top_right_x: 0,
  corner_top_right_y: 0,
  corner_bottom_right_x: 0,
  corner_bottom_right_y: 0,
  corner_bottom_left_x: 0,
  corner_bottom_left_y: 0,
};

export const videoOutputAspectModes: VideoOutputAspectMode[] = ["Stretch", "Fit", "Fill"];

export type MappingVideoOutputCornerKey = "topLeft" | "topRight" | "bottomRight" | "bottomLeft";

export const mappingVideoOutputCorners = [
  {
    key: "topLeft",
    label: "TL",
    baseX: -1,
    baseZ: -1,
    xField: "corner_top_left_x",
    zField: "corner_top_left_y",
  },
  {
    key: "topRight",
    label: "TR",
    baseX: 1,
    baseZ: -1,
    xField: "corner_top_right_x",
    zField: "corner_top_right_y",
  },
  {
    key: "bottomRight",
    label: "BR",
    baseX: 1,
    baseZ: 1,
    xField: "corner_bottom_right_x",
    zField: "corner_bottom_right_y",
  },
  {
    key: "bottomLeft",
    label: "BL",
    baseX: -1,
    baseZ: 1,
    xField: "corner_bottom_left_x",
    zField: "corner_bottom_left_y",
  },
] as const satisfies readonly {
  key: MappingVideoOutputCornerKey;
  label: string;
  baseX: -1 | 1;
  baseZ: -1 | 1;
  xField: NumericVideoOutputMappingField;
  zField: NumericVideoOutputMappingField;
}[];

export const mappingVideoOutputCornerGain = 0.28;

export const videoOutputAspectPresets = [
  { label: "16:9", ratio: 16 / 9 },
  { label: "16:10", ratio: 16 / 10 },
  { label: "4:3", ratio: 4 / 3 },
  { label: "1:1", ratio: 1 },
  { label: "9:16", ratio: 9 / 16 },
  { label: "21:9", ratio: 21 / 9 },
  { label: "2.39:1", ratio: 2.39 },
] as const;

export const resetVideoOutputCornerOffsets = (mapping: VideoOutputMapping): VideoOutputMapping => ({
  ...mapping,
  corner_top_left_x: 0,
  corner_top_left_y: 0,
  corner_top_right_x: 0,
  corner_top_right_y: 0,
  corner_bottom_right_x: 0,
  corner_bottom_right_y: 0,
  corner_bottom_left_x: 0,
  corner_bottom_left_y: 0,
});

export const resetVideoOutputWarp = (mapping: VideoOutputMapping): VideoOutputMapping => ({
  ...resetVideoOutputCornerOffsets(mapping),
  lens_distortion: 0,
  keystone_x: 0,
  keystone_y: 0,
});

export const resetVideoOutputBlend = (mapping: VideoOutputMapping): VideoOutputMapping => ({
  ...mapping,
  edge_blend_left: 0,
  edge_blend_right: 0,
  edge_blend_top: 0,
  edge_blend_bottom: 0,
  edge_blend_gamma: 2.2,
  black_level: 0,
});

export const resetVideoOutputMask = (mapping: VideoOutputMapping): VideoOutputMapping => ({
  ...mapping,
  mask_point_count: 0,
  mask_invert: false,
  mask_softness: 0,
  mask_points: Array.from({ length: 8 }, () => ({ x: 0, y: 0 })),
  bitmap_mask_width: 0,
  bitmap_mask_height: 0,
  bitmap_mask_luma_words: Array.from({ length: 32 }, () => 0),
});

export const resetVideoOutputLensKeystone = (mapping: VideoOutputMapping): VideoOutputMapping => ({
  ...mapping,
  lens_distortion: 0,
  keystone_x: 0,
  keystone_y: 0,
});

export const resetVideoOutputPose = (mapping: VideoOutputMapping): VideoOutputMapping => ({
  ...mapping,
  offset_x: 0,
  offset_y: 0,
  scale_x: 1,
  scale_y: 1,
  rotation_deg: 0,
});

export const resetVideoOutputStagePosition = (mapping: VideoOutputMapping): VideoOutputMapping => ({
  ...mapping,
  stage_x: 0,
  stage_y: 0,
  stage_z: 0,
});

export const outputAspectRatio = (width: number, height: number) => {
  if (!Number.isFinite(width) || !Number.isFinite(height) || height <= 0) {
    return 1;
  }
  return Number((width / height).toFixed(4));
};

export const mappingNumber = (
  mapping: VideoOutputMapping,
  field: NumericVideoOutputMappingField,
  fallback: number,
) => (Number.isFinite(mapping[field]) ? mapping[field] : fallback);

export const videoOutputStagePosition = (output: VideoOutputSummary): Vec3 => ({
  x: mappingNumber(output.mapping, "stage_x", 0),
  y: mappingNumber(output.mapping, "stage_y", 0),
  z: mappingNumber(output.mapping, "stage_z", 0),
});

export const stagePositionLabel = (position: Vec3) =>
  `${position.x.toFixed(1)}, ${position.y.toFixed(1)}, ${position.z.toFixed(1)}`;

export const aspectRatioLabel = (ratio: number) => {
  const normalized = Number.isFinite(ratio) && ratio > 0 ? ratio : 1;
  const preset = videoOutputAspectPresets.find((candidate) => Math.abs(candidate.ratio - normalized) < 0.01);
  return preset?.label ?? `${normalized.toFixed(2)}:1`;
};

export const cornerWarpAmount = (mapping: VideoOutputMapping) =>
  Math.abs(mappingNumber(mapping, "corner_top_left_x", 0)) +
  Math.abs(mappingNumber(mapping, "corner_top_left_y", 0)) +
  Math.abs(mappingNumber(mapping, "corner_top_right_x", 0)) +
  Math.abs(mappingNumber(mapping, "corner_top_right_y", 0)) +
  Math.abs(mappingNumber(mapping, "corner_bottom_right_x", 0)) +
  Math.abs(mappingNumber(mapping, "corner_bottom_right_y", 0)) +
  Math.abs(mappingNumber(mapping, "corner_bottom_left_x", 0)) +
  Math.abs(mappingNumber(mapping, "corner_bottom_left_y", 0));

export const mappingCorrectionReadout = (mapping: VideoOutputMapping) =>
  `Aspect ${mapping.aspect_mode} ${aspectRatioLabel(mappingNumber(mapping, "aspect_ratio", 1))} / Lens ${mappingNumber(
    mapping,
    "lens_distortion",
    0,
  ).toFixed(2)} / Key ${mappingNumber(mapping, "keystone_x", 0).toFixed(2)}, ${mappingNumber(
    mapping,
    "keystone_y",
    0,
  ).toFixed(2)} / Corners ${cornerWarpAmount(mapping).toFixed(2)} / Blend ${[
    mappingNumber(mapping, "edge_blend_left", 0),
    mappingNumber(mapping, "edge_blend_right", 0),
    mappingNumber(mapping, "edge_blend_top", 0),
    mappingNumber(mapping, "edge_blend_bottom", 0),
  ].some((value) => value > 0) ? "On" : "Off"} / Black ${mappingNumber(mapping, "black_level", 0).toFixed(2)} / Mask ${mapping.mask_point_count >= 3 ? `${mapping.mask_point_count}pt` : "Off"}`;

export const mappingFieldRange = (field: NumericVideoOutputMappingField): [number, number] => {
  if (field === "stage_x" || field === "stage_y" || field === "stage_z") {
    return [-1000, 1000];
  }
  if (field === "scale_x" || field === "scale_y") {
    return [0.01, 8];
  }
  if (field === "aspect_ratio") {
    return [0.1, 10];
  }
  if (field === "edge_blend_gamma") {
    return [0.1, 8];
  }
  if (field === "mask_point_count") {
    return [0, 8];
  }
  if (field === "mask_softness") {
    return [0, 0.5];
  }
  if (
    field === "edge_blend_left" ||
    field === "edge_blend_right" ||
    field === "edge_blend_top" ||
    field === "edge_blend_bottom" ||
    field === "black_level"
  ) {
    return [0, 1];
  }
  if (field === "rotation_deg") {
    return [-180, 180];
  }
  if (field === "offset_x" || field === "offset_y") {
    return [-4, 4];
  }
  return [-1, 1];
};

export const mappingReadout = (mapping: VideoOutputMapping) =>
  `Stage ${mappingNumber(mapping, "stage_x", 0).toFixed(1)}, ${mappingNumber(mapping, "stage_y", 0).toFixed(1)}, ${mappingNumber(mapping, "stage_z", 0).toFixed(1)} / X ${mappingNumber(mapping, "offset_x", 0).toFixed(2)} / Y ${mappingNumber(mapping, "offset_y", 0).toFixed(2)} / Key ${mappingNumber(mapping, "keystone_x", 0).toFixed(2)}, ${mappingNumber(mapping, "keystone_y", 0).toFixed(2)} / ${mapping.aspect_mode} ${aspectRatioLabel(mappingNumber(mapping, "aspect_ratio", 1))}`;
