// Project snapshot comparison helpers extracted from App.tsx. Strip volatile/runtime
// fields so two snapshots can be compared for "has the saved project actually changed".
import type { AutoVjStatus, EngineSnapshot } from "./types";

export const defaultPersistedAutoVjStatus = (): AutoVjStatus => ({
  mode: "Off",
  armed: false,
  hold: false,
  show_revision: 0,
  action_sequence: 0,
  last_consumed_boundary: null,
  next_boundary_beat: null,
  last_action: null,
  action_log: [],
  fault: null,
  live_audio_beat_counter: 0,
  last_live_audio_feature_sequence: null,
});

/**
 * Match the desktop save boundary: Auto VJ configuration is project data,
 * while armed/hold state, counters, faults and action history are runtime-only.
 */
export const normalizeProjectAutoVjForPersistence = (snapshot: EngineSnapshot): EngineSnapshot => {
  if (!snapshot.video?.auto_vj) return snapshot;
  return {
    ...snapshot,
    video: {
      ...snapshot.video,
      auto_vj: {
        config: {
          ...snapshot.video.auto_vj.config,
          eligible_layer_ids: [...snapshot.video.auto_vj.config.eligible_layer_ids],
        },
        status: defaultPersistedAutoVjStatus(),
      },
    },
  };
};

export const projectComparableSnapshot = (snapshot: EngineSnapshot) => {
  const comparable = normalizeProjectAutoVjForPersistence(
    JSON.parse(JSON.stringify(snapshot)) as EngineSnapshot,
  );
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
