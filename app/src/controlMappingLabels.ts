import { normalizeFixtureFlagClearKind } from "./controlMappingActions";
import type { MidiControlMapping, OscControlMapping } from "./types";

export const controlMappingTargetLabel = (mapping: MidiControlMapping | OscControlMapping) => {
  switch (mapping.action) {
    case "FixtureAttribute": return `Fixture ${mapping.fixture_id} ${mapping.attribute}`;
    case "FixtureHighlight": return `Fixture ${mapping.fixture_id} highlight`;
    case "FixtureSolo": return `Fixture ${mapping.fixture_id} solo`;
    case "FixturePark": return `Fixture ${mapping.fixture_id} park`;
    case "GroupHighlight": return `Group ${mapping.group_id} highlight`;
    case "GroupSolo": return `Group ${mapping.group_id} solo`;
    case "GroupPark": return `Group ${mapping.group_id} park`;
    case "TriggerCue": return `Cue ${mapping.cue_id}`;
    case "TriggerNextCue": return "Cue next";
    case "TriggerPreviousCue": return "Cue previous";
    case "EffectEnabled": return `Effect ${mapping.cue_id} enabled`;
    case "NodeGraphEnabled": return `Node graph ${mapping.cue_id} enabled`;
    case "VideoParam": return `Layer ${mapping.layer_id} ${mapping.video_param}`;
    case "VideoCuePointAdd":
      return `Layer ${mapping.layer_id} add cue ${
        mapping.duration_ms === null || mapping.duration_ms === undefined ? "current" : `${mapping.duration_ms}ms`
      }`;
    case "VideoCuePointRemove": return `Layer ${mapping.layer_id} remove cue ${mapping.duration_ms ?? 0}ms`;
    case "VideoCuePointJump": return `Layer ${mapping.layer_id} cue ${mapping.cue_point_index ?? 0}`;
    case "VideoCuePointPrevious": return `Layer ${mapping.layer_id} previous cue`;
    case "VideoCuePointNext": return `Layer ${mapping.layer_id} next cue`;
    case "VideoLayerEnabled": return `Layer ${mapping.layer_id} enabled`;
    case "VideoLayerSolo": return `Layer ${mapping.layer_id} solo`;
    case "VideoPlay": return `Layer ${mapping.layer_id} play`;
    case "VideoLoop": return `Layer ${mapping.layer_id} loop ${mapping.low}-${mapping.high}ms`;
    case "VideoLayerFade": return `Layer ${mapping.layer_id} fade ${mapping.duration_ms ?? 1000}ms`;
    case "VideoOutputEnabled": return `Output ${mapping.output_id} enabled`;
    case "VideoOutputOpacity": return `Output ${mapping.output_id} opacity`;
    case "VideoOutputFade": return `Output ${mapping.output_id} fade ${mapping.duration_ms ?? 1000}ms`;
    case "VideoOutputMappingField": return `Output ${mapping.output_id} map ${mapping.attribute}`;
    case "VideoOutputMappingPreset": return `Output ${mapping.output_id} preset ${mapping.attribute}`;
    case "VideoOutputBlackout": return `Output ${mapping.output_id} blackout`;
    case "TimelinePlay": return "Timeline play";
    case "TimelineSeek": return "Timeline seek";
    case "TimelineBeatPrevious": return "Timeline previous beat";
    case "TimelineBeatNext": return "Timeline next beat";
    case "SetBpm": return `Set BPM ${mapping.low}-${mapping.high}`;
    case "TapBpm": return "Tap BPM";
    case "LightingMaster": return "Lighting master";
    case "GroupSubmaster": return `Group ${mapping.group_id} submaster`;
    case "CueFadePause": return "Cue fade pause";
    case "Blackout": return "Lighting blackout";
    case "AllBlackout": return "All blackout";
    case "VideoBlackout": return "Video blackout";
    case "ClearFixtureFlags": return `Clear fixture ${normalizeFixtureFlagClearKind(mapping.attribute)}`;
  }
};
