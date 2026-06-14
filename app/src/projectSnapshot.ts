// Project snapshot comparison helpers extracted from App.tsx. Strip volatile/runtime
// fields so two snapshots can be compared for "has the saved project actually changed".
import type { EngineSnapshot } from "./types";

export const projectComparableSnapshot = (snapshot: EngineSnapshot) => {
  const comparable = JSON.parse(JSON.stringify(snapshot)) as EngineSnapshot;
  comparable.active_cue_id = null;
  comparable.active_fade = null;
  comparable.timeline = {
    ...comparable.timeline,
    playing: false,
    position_ms: 0,
  };
  comparable.video = {
    ...comparable.video,
    layers: comparable.video.layers.map((layer) => ({
      ...layer,
      state: {
        ...layer.state,
        playing: false,
        position_ms: 0,
      },
    })),
  };
  comparable.clock = {
    ...comparable.clock,
    beat_phase: 0,
    beat_counter: 0,
    tap_count: 0,
  };
  comparable.dmx_preview = [];
  comparable.dmx_previews = [];
  comparable.telemetry = {} as EngineSnapshot["telemetry"];
  return comparable;
};

export const projectSnapshotSignature = (snapshot: EngineSnapshot) =>
  JSON.stringify(projectComparableSnapshot(snapshot));
