use protocol::EngineSnapshot;

// Keep this construction exhaustive: a new protocol field must be deliberately
// included here without copying the backend-only authored video payload.
pub(super) fn clone_public_snapshot(snapshot: &EngineSnapshot) -> EngineSnapshot {
    EngineSnapshot {
        fixtures: snapshot.fixtures.clone(),
        cues: snapshot.cues.clone(),
        cue_lists: snapshot.cue_lists.clone(),
        palettes: snapshot.palettes.clone(),
        playback_executors: snapshot.playback_executors.clone(),
        playback_master: snapshot.playback_master,
        active_cue_id: snapshot.active_cue_id,
        direct_child_timeline_transports: snapshot.direct_child_timeline_transports.clone(),
        active_group_cue_ids: snapshot.active_group_cue_ids.clone(),
        cue_live_modifiers: snapshot.cue_live_modifiers.clone(),
        group_colors: snapshot.group_colors.clone(),
        active_fade: snapshot.active_fade.clone(),
        programmer: snapshot.programmer.clone(),
        timeline: snapshot.timeline.clone(),
        timeline_bank: snapshot.timeline_bank.clone(),
        video: snapshot.video.clone(),
        video_clip_runtime: snapshot.video_clip_runtime.clone(),
        video_transition_runtime: snapshot.video_transition_runtime.clone(),
        authored_video: None,
        effects: snapshot.effects.clone(),
        node_graphs: snapshot.node_graphs.clone(),
        output: snapshot.output.clone(),
        dmx_outputs: snapshot.dmx_outputs.clone(),
        lighting_master: snapshot.lighting_master,
        submasters: snapshot.submasters.clone(),
        blackout: snapshot.blackout,
        clock: snapshot.clock.clone(),
        stage_map: snapshot.stage_map,
        stage_map_presets: snapshot.stage_map_presets.clone(),
        stage_objects: snapshot.stage_objects.clone(),
        touch_surface: snapshot.touch_surface.clone(),
        dmx_preview: snapshot.dmx_preview.clone(),
        dmx_previews: snapshot.dmx_previews.clone(),
        telemetry: snapshot.telemetry.clone(),
    }
}
