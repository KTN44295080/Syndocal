use super::allocator_test_handle;
use crate::{EngineHandle, EngineTelemetrySnapshot, TimelineTransportAuthority};
use protocol::{
    AutoVjAction, CompositionSummary, DmxOutputConfig, DmxOutputProtocol, EngineSnapshot,
    PatchedFixtureSummary, Rotation3, StageObjectKind, StageObjectSummary,
    TimelineFollowRuntimeStatusSnapshot, TimelineLayerKind, TimelineLayerSummary, Vec3,
    VideoBlendMode, VideoClipLayerRuntimeSummary, VideoClipTakeDuration, VideoClipTakeKind,
    VideoLayerState, VideoLayerSummary, VideoLayerTransitionBusRuntimeSummary,
    VideoLayerTransitionBusSummary, VideoLayerTransitionCurve, VideoLayerTransitionTarget,
    VideoOutputKind, VideoOutputSummary, VideoSourceKind, VideoSourceSummary, VideoTransitionBusId,
};
use std::{
    hint::black_box,
    sync::{Arc, RwLock},
    time::Instant,
};

fn publication(token: u64) -> EngineSnapshot {
    let mut snapshot = EngineSnapshot::default();
    snapshot.timeline.transport_epoch = token;
    snapshot.timeline.transport_generation = token + 1;
    snapshot.timeline.playing = token % 2 == 0;
    snapshot.timeline.follow_runtime.generation = token + 2;
    snapshot.timeline.follow_runtime.fault = Some(format!("publication {token}"));
    snapshot.timeline.follow_runtime.source_timeline_id = Some(protocol::TimelineId(token));
    snapshot.timeline.follow_runtime.target_timeline_id = Some(protocol::TimelineId(token + 1));
    snapshot.timeline.follow_runtime.elapsed_ms = 50;
    snapshot.timeline.follow_runtime.duration_ms = 100;
    snapshot.timeline.follow_runtime.progress_millis = 500;
    snapshot.timeline.follow_runtime.transition_hold_active = true;
    snapshot.clock.bpm = 120.0 + token as f32;
    snapshot.output = DmxOutputConfig {
        target_ip: format!("192.0.2.{token}"),
        ..DmxOutputConfig::default()
    };
    snapshot.dmx_outputs = vec![DmxOutputConfig {
        protocol: DmxOutputProtocol::ArtNet,
        target_ip: format!("198.51.100.{token}"),
        ..DmxOutputConfig::default()
    }];
    snapshot.stage_objects = vec![StageObjectSummary {
        id: token,
        label: format!("stage object {token}"),
        kind: StageObjectKind::Screen,
        x: token as f32,
        z: -1.0,
        width: 2.0,
        depth: 0.5,
        rotation_deg: 0.0,
        color: Some("#55ccff".to_string()),
    }];
    snapshot.fixtures = vec![PatchedFixtureSummary {
        id: token,
        label: format!("fixture {token}"),
        profile_source_path: String::new(),
        profile_name: "test".to_string(),
        manufacturer: "test".to_string(),
        mode_name: "test".to_string(),
        universe: 1,
        address: 1,
        group_ids: vec![format!("group-{token}"), "shared-group".to_string()],
        position: Vec3::default(),
        rotation: Rotation3::default(),
        geometries: Vec::new(),
        controls: Vec::new(),
        stage_layout: None,
        attribute_values: Vec::new(),
        limits: protocol::FixtureLimits::default(),
        highlighted: false,
        soloed: false,
        parked: false,
    }];
    snapshot.video.layers.push(VideoLayerSummary {
        id: token,
        label: format!("video layer {token}"),
        source: VideoSourceSummary {
            kind: VideoSourceKind::StillImage,
            path: Some(format!("still-{token}.png")),
            name: None,
            codec: None,
            metadata: None,
        },
        media_asset_id: None,
        blend_mode: VideoBlendMode::Normal,
        state: VideoLayerState::default(),
        isf_effect: None,
        clip_slots: Vec::new(),
        default_clip_slot_id: None,
    });
    snapshot
        .video_clip_runtime
        .layers
        .push(VideoClipLayerRuntimeSummary {
            layer_id: token,
            active_slot_id: Some(protocol::VideoClipSlotId(token + 3)),
            playhead_ms: token * 100,
            ..Default::default()
        });
    snapshot
        .video_clip_runtime
        .timeline_video_projection_layer_ids = vec![token + 4];
    snapshot.video.outputs.push(VideoOutputSummary {
        id: token,
        label: format!("output {token}"),
        kind: VideoOutputKind::Display,
        enabled: true,
        composition_id: 1,
        fullscreen: false,
        monitor_id: None,
        monitor_identity: None,
        width: 640,
        height: 360,
        endpoint_name: None,
        opacity: 1.0,
        blackout: false,
        mapping: protocol::VideoOutputMapping::default(),
    });
    snapshot.video.outputs.push(VideoOutputSummary {
        id: token + 10_000,
        label: format!("secondary {token}"),
        kind: VideoOutputKind::Display,
        enabled: true,
        composition_id: 1,
        fullscreen: false,
        monitor_id: None,
        monitor_identity: None,
        width: 640,
        height: 360,
        endpoint_name: None,
        opacity: 1.0,
        blackout: false,
        mapping: protocol::VideoOutputMapping::default(),
    });
    let first_target = VideoLayerTransitionTarget::Layer { layer_id: token };
    let second_target = VideoLayerTransitionTarget::Layer {
        layer_id: token + 1,
    };
    snapshot
        .video
        .transition_buses
        .push(VideoLayerTransitionBusSummary {
            id: VideoTransitionBusId(token),
            label: format!("bus {token}"),
            composition_id: 1,
            enabled: true,
            members: vec![first_target.clone(), second_target.clone()],
            default_from: first_target,
            default_to: second_target,
            default_kind: VideoClipTakeKind::Crossfade,
            default_duration: VideoClipTakeDuration::milliseconds(400),
            default_curve: VideoLayerTransitionCurve::EaseInOut,
            matte_source: None,
        });
    snapshot.video.compositions.push(CompositionSummary {
        id: 1,
        label: format!("composition {token}"),
        layer_ids: vec![token],
        timeline_layer_ids: Vec::new(),
        output_ids: vec![token],
    });
    snapshot.video.auto_vj.status.last_action = Some(AutoVjAction {
        sequence: token,
        boundary_index: token + 1,
        beat: token + 2,
        layer_id: token + 3,
        transition_ms: 400,
        selection_token: token + 4,
        seed: token + 5,
        show_revision: token + 6,
        trigger: protocol::AutoVjTrigger::ClockBoundary,
        live_audio_feature_sequence: Some(token + 7),
    });
    snapshot
        .video_transition_runtime
        .buses
        .push(VideoLayerTransitionBusRuntimeSummary {
            bus_id: protocol::VideoTransitionBusId(token),
            origin_from: VideoLayerTransitionTarget::Layer { layer_id: token },
            from: VideoLayerTransitionTarget::Layer { layer_id: token },
            to: VideoLayerTransitionTarget::Layer {
                layer_id: token + 1,
            },
            kind: protocol::VideoClipTakeKind::Crossfade,
            curve: Default::default(),
            elapsed_ms: 50,
            duration_ms: 100,
            duration: Default::default(),
            progress_millis: 500,
        });
    snapshot
}

fn assert_same_read_model(handle: &EngineHandle) {
    let expected = handle.snapshot();
    assert_eq!(
        handle.timeline_transport_generation(),
        expected.timeline.transport_generation
    );
    assert_eq!(
        handle.timeline_transport_authority(),
        TimelineTransportAuthority {
            epoch: expected.timeline.transport_epoch,
            generation: expected.timeline.transport_generation,
        }
    );
    assert_eq!(
        handle.timeline_follow_runtime_summary(),
        expected.timeline.follow_runtime
    );
    assert_eq!(
        handle.timeline_follow_runtime_status(91),
        TimelineFollowRuntimeStatusSnapshot::from_runtime(91, &expected.timeline.follow_runtime)
    );
    assert_eq!(
        handle.video_clip_runtime_snapshot(),
        expected.video_clip_runtime
    );
    assert_eq!(
        handle.video_layer_thumbnail_snapshot(),
        (expected.video.clone(), expected.clock.bpm)
    );
    assert_eq!(
        handle.video_output_preview_snapshot(),
        (
            expected.video.clone(),
            expected.video_clip_runtime.clone(),
            expected.video_transition_runtime.clone(),
            expected.clock.bpm,
        )
    );
    assert_eq!(
        handle.engine_telemetry_snapshot(),
        EngineTelemetrySnapshot::from_snapshot(&expected)
    );
    assert_eq!(handle.video_snapshot(), expected.video);
    assert_eq!(handle.stage_objects_snapshot(), expected.stage_objects);
    assert_eq!(handle.fixtures_snapshot(), expected.fixtures);
    assert_eq!(
        handle.fixture_group_ids_snapshot(),
        expected
            .fixtures
            .iter()
            .map(|fixture| fixture.group_ids.clone())
            .collect::<Vec<_>>()
    );
    for output in &expected.video.outputs {
        assert!(handle.video_output_exists(output.id));
    }
    assert!(!handle.video_output_exists(99_999));
    assert_eq!(
        handle.video_layer_ids_snapshot(),
        expected
            .video
            .layers
            .iter()
            .map(|layer| layer.id)
            .collect::<Vec<_>>()
    );
    if let Some(layer) = expected.video.layers.first() {
        assert_eq!(
            handle.video_layer_state_snapshot(layer.id),
            Some(layer.state.clone())
        );
        assert_eq!(
            handle.video_layer_states_snapshot(&[layer.id]),
            vec![(layer.id, layer.state.clone())]
        );
        assert_eq!(
            handle.video_layer_audio_monitor_snapshot(layer.id),
            Some((
                layer.state.clone(),
                layer.source.clone(),
                expected.video.auto_vj.status.last_action.clone()
            ))
        );
    } else {
        assert_eq!(handle.video_layer_state_snapshot(7), None);
        assert!(handle.video_layer_states_snapshot(&[7]).is_empty());
        assert_eq!(handle.video_layer_audio_monitor_snapshot(7), None);
    }
    assert_eq!(handle.video_layer_state_snapshot(99_999), None);
    assert_eq!(
        handle.auto_vj_last_action(),
        expected.video.auto_vj.status.last_action
    );
    assert_eq!(
        handle.video_layer_transition_runtime_snapshot(),
        expected.video_transition_runtime
    );
    assert_eq!(handle.video_outputs_snapshot(), expected.video.outputs);
    assert_eq!(
        handle.video_transition_buses_snapshot(),
        expected.video.transition_buses
    );
    assert_eq!(
        handle.video_outputs_and_compositions_snapshot(),
        (expected.video.outputs, expected.video.compositions)
    );
    assert_eq!(
        handle.dmx_outputs_and_output_snapshot(),
        (expected.dmx_outputs, expected.output)
    );
    assert_eq!(handle.timeline_playing(), expected.timeline.playing);
}

#[test]
fn timeline_audio_allocator_reader_matches_published_projection() {
    let mut snapshot = EngineSnapshot::default();
    snapshot.timeline.audio = Some(protocol::AudioAnalysisSummary {
        path: "memory://allocator-reader.wav".to_string(),
        sample_rate: 48_000,
        channels: 2,
        duration_ms: 1_000,
        estimated_bpm: None,
        waveform: Vec::new(),
        spectrum: Vec::new(),
        beats: Vec::new(),
    });
    snapshot.timeline.layers = vec![TimelineLayerSummary {
        id: 41,
        label: "Lighting".to_string(),
        order: 0,
        muted: false,
        locked: false,
        solo: false,
        expanded: false,
        kind: TimelineLayerKind::Lighting,
    }];
    let expected = (
        snapshot.timeline.audio.is_some(),
        snapshot.timeline.audio_clips.is_empty(),
        snapshot.timeline.layers.clone(),
    );
    let handle = allocator_test_handle(Arc::new(RwLock::new(snapshot)));
    assert_eq!(handle.timeline_audio_allocator_snapshot(), expected);
}

#[test]
fn stage_map_preset_allocator_reader_matches_label_selection() {
    let mut snapshot = EngineSnapshot::default();
    snapshot.stage_map_presets = vec![super::published_stage_map_preset("Front Room", 41)];
    let expected = snapshot.stage_map_presets[0].stage_objects.clone();
    let handle = allocator_test_handle(Arc::new(RwLock::new(snapshot)));

    assert_eq!(
        handle.stage_map_preset_allocator_objects("Front Room"),
        expected
    );
    assert_eq!(
        handle.stage_map_preset_allocator_objects("Missing Room"),
        None
    );
}

#[test]
fn narrow_readers_match_public_snapshot_and_observe_replacement() {
    let published = Arc::new(RwLock::new(publication(7)));
    let handle = allocator_test_handle(Arc::clone(&published));
    assert_same_read_model(&handle);
    let retained = handle.video_clip_runtime_snapshot();
    let retained_follow = handle.timeline_follow_runtime_summary();
    let retained_transition = handle.video_layer_transition_runtime_snapshot();
    let mut retained_outputs = handle.video_outputs_snapshot();
    retained_outputs[0].label.push_str(" reader edit");
    assert_eq!(handle.video_outputs_snapshot()[0].label, "output 7");
    *published.write().unwrap() = publication(18);
    assert_same_read_model(&handle);
    assert_eq!(retained.layers[0].layer_id, 7);
    assert_eq!(retained_follow.fault.as_deref(), Some("publication 7"));
    assert_eq!(
        retained_transition.buses[0].bus_id,
        protocol::VideoTransitionBusId(7)
    );
    assert_eq!(handle.video_clip_runtime_snapshot().layers[0].layer_id, 18);
    assert_eq!(handle.video_outputs_snapshot()[0].id, 18);
    assert_eq!(handle.video_outputs_snapshot()[1].label, "secondary 18");
    assert_eq!(
        handle.video_transition_buses_snapshot(),
        publication(18).video.transition_buses
    );
    assert_eq!(
        handle.video_outputs_and_compositions_snapshot(),
        (
            publication(18).video.outputs,
            publication(18).video.compositions
        )
    );
}

#[test]
fn poisoned_publication_preserves_public_snapshot_defaults() {
    let published = Arc::new(RwLock::new(publication(7)));
    let handle = allocator_test_handle(Arc::clone(&published));
    let poison = std::thread::spawn(move || {
        let _guard = published.write().unwrap();
        panic!("deliberate publication poison");
    });
    assert!(poison.join().is_err());
    assert_same_read_model(&handle);
    assert_eq!(
        handle.timeline_transport_authority().epoch,
        EngineSnapshot::default().timeline.transport_epoch
    );
    assert!(handle.video_clip_runtime_snapshot().layers.is_empty());
    assert!(handle.stage_objects_snapshot().is_empty());
    assert!(handle.fixture_group_ids_snapshot().is_empty());
    assert!(!handle.video_output_exists(1));
    assert_eq!(handle.video_layer_state_snapshot(7), None);
    let default_snapshot = EngineSnapshot::default();
    assert_eq!(
        handle.video_layer_thumbnail_snapshot(),
        (default_snapshot.video, default_snapshot.clock.bpm)
    );
    assert_eq!(handle.auto_vj_last_action(), None);
    assert!(handle.video_outputs_snapshot().is_empty());
    assert!(handle.video_transition_buses_snapshot().is_empty());
    assert_eq!(
        handle.video_outputs_and_compositions_snapshot(),
        (Vec::new(), Vec::new())
    );
    assert_eq!(
        handle.dmx_outputs_and_output_snapshot(),
        (vec![DmxOutputConfig::default()], DmxOutputConfig::default())
    );
    assert!(!handle.timeline_playing());
}

#[test]
fn transport_authority_never_mixes_concurrent_publications() {
    let published = Arc::new(RwLock::new(publication(7)));
    let handle = allocator_test_handle(Arc::clone(&published));
    let writer = std::thread::spawn(move || {
        for token in 10..1010 {
            *published.write().unwrap() = publication(token);
        }
    });
    for _ in 0..2000 {
        let authority = handle.timeline_transport_authority();
        assert_eq!(authority.generation, authority.epoch + 1);
    }
    writer.join().unwrap();
}

#[test]
fn video_outputs_reader_never_mixes_concurrent_publications() {
    let published = Arc::new(RwLock::new(publication(7)));
    let handle = allocator_test_handle(Arc::clone(&published));
    let writer = std::thread::spawn(move || {
        for token in 10..1010 {
            *published.write().unwrap() = publication(token);
        }
    });
    for _ in 0..2000 {
        let outputs = handle.video_outputs_snapshot();
        assert_eq!(outputs.len(), 2);
        let token = outputs[0].id;
        assert_eq!(outputs[0].label, format!("output {token}"));
        assert_eq!(outputs[1].id, token + 10_000);
        assert_eq!(outputs[1].label, format!("secondary {token}"));
    }
    writer.join().unwrap();
}

#[test]
#[ignore = "synthetic timing report; run explicitly with --ignored --nocapture"]
fn benchmark_narrow_snapshot_readers() {
    const ITERATIONS: usize = 1000;
    const UNRELATED_ENTRIES: usize = 4096;
    const LABEL_BYTES: usize = 1024;
    let mut snapshot = publication(7);
    // Group labels are unrelated to the runtime fields read below. Fixed input
    // isolates avoided cloning; this is not a representative-show CPU claim.
    snapshot.group_colors = (0..UNRELATED_ENTRIES)
        .map(|index| {
            (
                format!("{index:08}{}", "x".repeat(LABEL_BYTES)),
                "#ffffff".to_string(),
            )
        })
        .collect();
    let handle = allocator_test_handle(Arc::new(RwLock::new(snapshot)));
    let old_start = Instant::now();
    for _ in 0..ITERATIONS {
        black_box(handle.snapshot().video_clip_runtime);
        black_box(handle.snapshot().video_transition_runtime);
        black_box(handle.snapshot().timeline.follow_runtime);
    }
    let old = old_start.elapsed();
    let narrow_start = Instant::now();
    for _ in 0..ITERATIONS {
        black_box(handle.video_clip_runtime_snapshot());
        black_box(handle.video_layer_transition_runtime_snapshot());
        black_box(handle.timeline_follow_runtime_summary());
    }
    let narrow = narrow_start.elapsed();
    println!("synthetic_snapshot_readers iterations={ITERATIONS} unrelated_entries={UNRELATED_ENTRIES} label_bytes={LABEL_BYTES} reads_per_iteration=3 old_us={} narrow_us={}", old.as_micros(), narrow.as_micros());
}
