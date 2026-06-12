import type {
  Vec3,
  VideoOutputAspectMode,
  VideoOutputMapping,
  VideoOutputSummary,
} from "./types";

export type NumericVideoOutputMappingField = Exclude<keyof VideoOutputMapping, "aspect_mode">;

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
