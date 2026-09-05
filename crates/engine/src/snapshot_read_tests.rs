use super::allocator_test_handle;
use crate::{EngineHandle, TimelineTransportAuthority};
use protocol::{
    EngineSnapshot, TimelineFollowRuntimeStatusSnapshot, VideoClipLayerRuntimeSummary,
    VideoLayerTransitionBusRuntimeSummary, VideoLayerTransitionTarget,
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
    snapshot.timeline.follow_runtime.generation = token + 2;
    snapshot.timeline.follow_runtime.fault = Some(format!("publication {token}"));
    snapshot.timeline.follow_runtime.source_timeline_id = Some(protocol::TimelineId(token));
    snapshot.timeline.follow_runtime.target_timeline_id = Some(protocol::TimelineId(token + 1));
    snapshot.timeline.follow_runtime.elapsed_ms = 50;
    snapshot.timeline.follow_runtime.duration_ms = 100;
    snapshot.timeline.follow_runtime.progress_millis = 500;
    snapshot.timeline.follow_runtime.transition_hold_active = true;
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
        handle.video_layer_transition_runtime_snapshot(),
        expected.video_transition_runtime
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
    *published.write().unwrap() = publication(18);
    assert_same_read_model(&handle);
    assert_eq!(retained.layers[0].layer_id, 7);
    assert_eq!(retained_follow.fault.as_deref(), Some("publication 7"));
    assert_eq!(
        retained_transition.buses[0].bus_id,
        protocol::VideoTransitionBusId(7)
    );
    assert_eq!(handle.video_clip_runtime_snapshot().layers[0].layer_id, 18);
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
