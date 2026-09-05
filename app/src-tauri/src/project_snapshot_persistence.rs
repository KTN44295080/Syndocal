//! Pure authored snapshot projection for project and node-graph persistence.
//! Callers retain validation order, transaction ownership, and file I/O.
use protocol::{
    ClockSnapshot, EngineSnapshot, EngineTelemetry, NodeGraphSummary, TimelineLayerKind,
    TimelineSnapshot, TimelineTrackKind,
};
use std::collections::HashMap;

pub(super) fn use_authored_video_snapshot(snapshot: &mut EngineSnapshot) {
    if let Some(authored_video) = snapshot.authored_video.take() {
        snapshot.video = authored_video;
    }
}

pub(super) fn node_graph_for_persistence(mut graph: NodeGraphSummary) -> NodeGraphSummary {
    graph.audio_runtime.clear();
    graph
}

fn normalize_project_timeline_snapshot_layers(timeline: &mut TimelineSnapshot) {
    for event in &mut timeline.events {
        if event.duration_ms == 0 {
            event.fade_in_ms = 0;
            event.fade_out_ms = 0;
        } else {
            event.fade_in_ms = event.fade_in_ms.min(event.duration_ms);
            event.fade_out_ms = event.fade_out_ms.min(event.duration_ms);
        }
    }
    for clip in &mut timeline.audio_clips {
        clip.path = clip.path.trim().to_string();
        clip.start_ms = clip.start_ms.min(u64::MAX.saturating_sub(clip.duration_ms));
        clip.gain = if clip.gain.is_finite() {
            clip.gain.clamp(0.0, 2.0)
        } else {
            1.0
        };
        clip.fade_in_ms = clip.fade_in_ms.min(clip.duration_ms);
        clip.fade_out_ms = clip
            .fade_out_ms
            .min(clip.duration_ms.saturating_sub(clip.fade_in_ms));
    }
    timeline
        .audio_clips
        .sort_by_key(|clip| (clip.start_ms, clip.layer_id, clip.id));
    if timeline.layers.is_empty() {
        return;
    }

    timeline
        .layers
        .sort_by_key(|layer| (layer.kind.display_section_rank(), layer.order, layer.id));
    for (index, layer) in timeline.layers.iter_mut().enumerate() {
        layer.order = u32::try_from(index).unwrap_or(u32::MAX);
    }

    let layer_kinds = timeline
        .layers
        .iter()
        .map(|layer| (layer.id, layer.kind))
        .collect::<HashMap<_, _>>();
    for event in &mut timeline.events {
        let Some(kind) = event
            .layer_id
            .and_then(|layer_id| layer_kinds.get(&layer_id).copied())
        else {
            continue;
        };
        match kind {
            TimelineLayerKind::Lighting => event.track = TimelineTrackKind::Lighting,
            TimelineLayerKind::Video => event.track = TimelineTrackKind::Video,
            TimelineLayerKind::Audio => {}
        }
    }
}

pub(super) fn normalize_project_timeline_layers(snapshot: &mut EngineSnapshot) {
    normalize_project_timeline_snapshot_layers(&mut snapshot.timeline);
    for timeline in &mut snapshot.timeline_bank {
        normalize_project_timeline_snapshot_layers(timeline);
    }
}

pub(super) fn project_snapshot_for_save(mut snapshot: EngineSnapshot) -> EngineSnapshot {
    use_authored_video_snapshot(&mut snapshot);
    normalize_project_timeline_layers(&mut snapshot);
    // The shared clock's phase/counter/tap/external-lock fields advance at
    // runtime even while the authored project is idle. Project load only
    // restores the authored BPM, so persist exactly that stable surface.
    snapshot.clock = ClockSnapshot {
        bpm: snapshot.clock.bpm,
        ..ClockSnapshot::default()
    };
    for graph in &mut snapshot.node_graphs {
        graph.audio_runtime.clear();
    }
    snapshot.active_fade = None;
    snapshot.direct_child_timeline_transports.clear();
    snapshot.timeline.playing = false;
    snapshot.video.auto_vj.status = protocol::AutoVjStatus::default();
    snapshot.dmx_preview.clear();
    snapshot.dmx_previews.clear();
    clear_runtime_programmer_state(&mut snapshot);
    snapshot.telemetry = EngineTelemetry::default();
    // T17: latched scene live overrides are runtime-only; the engine's
    // persistence snapshot already strips them, and the `.sdc` writer keeps
    // that guarantee locally too.
    snapshot.cue_live_modifiers.clear();
    // T20: group strobe is a latched Live Mixer control, not project data.
    for submaster in &mut snapshot.submasters {
        submaster.strobe_hz = 0.0;
        submaster.strobe_fixture_count = 0;
    }
    snapshot
}

pub(super) fn clear_runtime_programmer_state(snapshot: &mut EngineSnapshot) {
    snapshot.programmer = protocol::ProgrammerSnapshot::default();
}
