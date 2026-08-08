import { normalizeFixtureFlagClearKind } from "./controlMappingActions";
import type { MidiControlMapping, OscControlMapping } from "./types";

export const controlMappingTargetLabel = (mapping: MidiControlMapping | OscControlMapping) => {
  switch (mapping.action) {
    case "FixtureAttribute": return `Fixture ${mapping.fixture_id} ${mapping.attribute}`;
    case "SelectedFeatureFader": return `Selected feature fader ${(mapping.cue_point_index ?? 0) + 1}`;
    case "FixtureHighlight": return `Fixture ${mapping.fixture_id} highlight`;
    case "FixtureSolo": return `Fixture ${mapping.fixture_id} solo`;
    case "FixturePark": return `Fixture ${mapping.fixture_id} park`;
    case "GroupHighlight": return `Group ${mapping.group_id} highlight`;
    case "GroupSolo": return `Group ${mapping.group_id} solo`;
    case "GroupPark": return `Group ${mapping.group_id} park`;
    case "TriggerCue": return `Cue ${mapping.cue_id}`;
    case "FlashCue": return `Cue ${mapping.cue_id} flash`;
    case "TriggerCueDirection": return `Cue ${mapping.cue_id} ${mapping.attribute ?? "direction"}`;
    case "FlashCueDirection": return `Cue ${mapping.cue_id} ${mapping.attribute ?? "direction"} flash`;
    case "TriggerCueListNext": return `Cue ${mapping.cue_id} list next`;
    case "TriggerNextCue": return "Cue next";
    case "TriggerPreviousCue": return "Cue previous";
    case "EffectEnabled": return `Effect ${mapping.cue_id} enabled`;
    case "NodeGraphEnabled": return `Node graph ${mapping.cue_id} enabled`;
    case "VideoParam": return `Video Layer ${mapping.layer_id} ${mapping.video_param}`;
    case "VideoCuePointAdd":
      return `Video Layer ${mapping.layer_id} add cue point ${
        mapping.duration_ms === null || mapping.duration_ms === undefined ? "current" : `${mapping.duration_ms}ms`
      }`;
    case "VideoCuePointRemove": return `Video Layer ${mapping.layer_id} remove cue point ${mapping.duration_ms ?? 0}ms`;
    case "VideoCuePointJump": return `Video Layer ${mapping.layer_id} cue point ${mapping.cue_point_index ?? 0}`;
    case "VideoCuePointPrevious": return `Video Layer ${mapping.layer_id} previous cue point`;
    case "VideoCuePointNext": return `Video Layer ${mapping.layer_id} next cue point`;
    case "VideoLayerEnabled": return `Video Layer ${mapping.layer_id} enabled`;
    case "VideoLayerSolo": return `Video Layer ${mapping.layer_id} solo`;
    case "VideoPlay": return `Video Layer ${mapping.layer_id} play`;
    case "VideoLoop": return `Video Layer ${mapping.layer_id} loop ${mapping.low}-${mapping.high}ms`;
    case "VideoLayerFade": return `Video Layer ${mapping.layer_id} fade ${mapping.duration_ms ?? 1000}ms`;
    case "VideoOutputEnabled": return `Video Output ${mapping.output_id} enabled`;
    case "VideoOutputOpacity": return `Video Output ${mapping.output_id} opacity`;
    case "VideoOutputFade": return `Video Output ${mapping.output_id} fade ${mapping.duration_ms ?? 1000}ms`;
    case "VideoOutputMappingField": return `Video Output ${mapping.output_id} map ${mapping.attribute}`;
    case "VideoOutputMappingPreset": return `Video Output ${mapping.output_id} preset ${mapping.attribute}`;
    case "VideoOutputBlackout": return `Video Output ${mapping.output_id} blackout`;
    case "TimelinePlay": return "Timeline play";
    case "TimelineSeek": return "Timeline seek";
    case "TimelineBeatPrevious": return "Timeline previous beat";
    case "TimelineBeatNext": return "Timeline next beat";
    case "SetBpm": return `Set BPM ${mapping.low}-${mapping.high}`;
    case "TapBpm": return "Tap BPM";
    case "LightingMaster": return "Lighting master";
    case "VideoMaster": return "Video master";
    case "GroupSubmaster": return `Group ${mapping.group_id} submaster`;
    case "CueFadePause": return "Cue fade pause";
    case "Blackout": return "Lighting blackout";
    case "AllBlackout": return "All blackout";
    case "VideoBlackout": return "Video blackout";
    case "ClearFixtureFlags": return `Clear fixture ${normalizeFixtureFlagClearKind(mapping.attribute)}`;
  }
};
