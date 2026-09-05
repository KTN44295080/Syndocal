use super::*;

fn runtime() -> EngineRuntime {
    let mut runtime = EngineRuntime::new(DmxOutputConfig { enabled: false, ..DmxOutputConfig::default() });
    runtime.video_layers.clear();
    runtime.timeline_layers = [50, 51].into_iter().map(|id| TimelineLayerSummary {
        id, label: format!("Video {id}"), order: id, kind: TimelineLayerKind::Video,
        muted: false, locked: false, solo: false, expanded: true,
    }).collect();
    runtime.media_assets = vec![MediaAssetSummary {
        id: 90, label: "Fixture".into(), source: VideoSourceSummary {
            kind: VideoSourceKind::File, path: Some("fixture://video".into()), name: None, codec: None,
            metadata: Some(protocol::VideoMediaMetadata { duration_ms: Some(10_000), width: Some(320), height: Some(180), frame_rate: Some(30.0), has_audio: false }),
        }, content_hash: None, byte_size: None,
    }];
    runtime.timeline_video_clips = [50, 51].into_iter().map(|layer_id| TimelineVideoClipSummary {
        id: TimelineVideoClipId(u64::from(layer_id)), layer_id, media_asset_id: 90,
        start_ms: 0, offset_ms: u64::from(layer_id - 50) * 400, duration_ms: 5_000, fade_in_ms: 0, fade_out_ms: 0,
    }).collect();
    runtime.timeline_playing = true;
    runtime.timeline_count_in_until = None;
    runtime.timeline_position_ms = 100;
    runtime
}

fn publish(runtime: &EngineRuntime, tick: bool) -> EngineSnapshot {
    let snapshot = runtime.build_snapshot(0);
    runtime.publish_video_sampling(&snapshot, tick);
    snapshot
}

fn sample(runtime: &EngineRuntime, snapshot: &EngineSnapshot, at: Instant) -> Result<Option<VideoRenderSample>, String> {
    runtime.shared_telemetry.video_render_sampling.read().unwrap().capture(snapshot, runtime.output_ownership_gate.status().epoch, at)
}

#[test]
fn video_sample_30hz_positions_follow_capture_time_between_44hz_publications() {
    let mut runtime = runtime();
    runtime.timeline_position_ms = 0;
    let start = runtime.last_tick;
    publish(&runtime, true);
    for index in 1..90u64 {
        let elapsed_us = index * 1_000_000 / 30;
        let tick_us = elapsed_us / 22_727 * 22_727;
        let tick = start + Duration::from_micros(tick_us);
        runtime.timeline_position_ms = tick_us / 1_000;
        runtime.last_tick = tick;
        runtime.timeline_tick_remainder = Some(TimelineTickRemainder {
            timeline_id: runtime.timeline_id, transport_epoch: runtime.timeline_transport_epoch,
            transport_generation: runtime.timeline_transport_generation, position_ms: runtime.timeline_position_ms,
            nanoseconds: ((tick_us % 1_000) * 1_000) as u32,
        });
        let snapshot = publish(&runtime, true);
        let original = snapshot.clone();
        let frame = sample(&runtime, &snapshot, start + Duration::from_micros(elapsed_us)).unwrap().unwrap();
        assert_eq!(frame.mode, VideoSamplingMode::ContinuousRootTimeline);
        assert_eq!(frame.video.layers[0].state.position_ms, elapsed_us / 1_000);
        assert_eq!(frame.video.layers[1].state.position_ms, elapsed_us / 1_000 + 400);
        assert_eq!(snapshot, original, "sampling cannot edit the canonical publication");
        assert_eq!(runtime.timeline_position_ms, tick_us / 1_000);
        assert_eq!(frame.clip_runtime, snapshot.video_clip_runtime);
        assert_eq!(frame.transition_runtime, snapshot.video_transition_runtime);
    }
}

#[test]
fn video_sample_boundaries_are_half_open_including_future_overlap_and_source_end() {
    for boundary in ["clip-end", "future-start", "source-end"] {
        let mut runtime = runtime();
        match boundary {
            "clip-end" => runtime.timeline_video_clips[0].duration_ms = 120,
            "future-start" => {
                let mut next = runtime.timeline_video_clips[0].clone(); next.id = TimelineVideoClipId(99); next.start_ms = 120;
                runtime.timeline_video_clips.push(next);
            }
            _ => runtime.media_assets[0].source.metadata.as_mut().unwrap().duration_ms = Some(520),
        }
        let snapshot = publish(&runtime, true);
        let deadline = runtime.last_tick + Duration::from_millis(20);
        let frame = sample(&runtime, &snapshot, deadline - Duration::from_nanos(1)).unwrap().unwrap();
        assert_eq!(frame.video.layers[0].state.position_ms, 119);
        assert!(sample(&runtime, &snapshot, deadline).unwrap().is_none(), "{boundary}");
        let publication = runtime.shared_telemetry.video_render_sampling.read().unwrap();
        assert_eq!(publication.validate(&frame.fence, frame.project_render_epoch, deadline), VideoRenderSampleValidation::DeadlineExpired);
    }
}

#[test]
fn video_sample_command_publication_retires_old_fence_and_resumes_after_next_tick() {
    let runtime = runtime();
    let snapshot = publish(&runtime, true);
    let frame = sample(&runtime, &snapshot, runtime.last_tick).unwrap().unwrap();
    publish(&runtime, false);
    let canonical = sample(&runtime, &snapshot, runtime.last_tick + Duration::from_secs(10)).unwrap().unwrap();
    assert_eq!(canonical.mode, VideoSamplingMode::Canonical("command-publication"));
    assert_eq!(canonical.video, snapshot.video);
    let publication = runtime.shared_telemetry.video_render_sampling.read().unwrap();
    assert_eq!(publication.validate(&frame.fence, frame.project_render_epoch, runtime.last_tick), VideoRenderSampleValidation::SemanticStale);
    drop(publication);
    publish(&runtime, true);
    assert_eq!(sample(&runtime, &snapshot, runtime.last_tick).unwrap().unwrap().mode, VideoSamplingMode::ContinuousRootTimeline);
}

#[test]
fn video_sample_plain_presentation_command_retires_fence_on_tick_publication() {
    let mut runtime = runtime();
    let snapshot = publish(&runtime, true);
    let frame = sample(&runtime, &snapshot, runtime.last_tick).unwrap().unwrap();
    runtime.apply_command(EngineCommand::SetVideoMasterOpacity(0.25));
    assert!(runtime.video_presentation_config_dirty);
    let next = publish(&runtime, true);
    runtime.fence_video_presentation_config_after_publication();
    assert_eq!(next.video.master_opacity, 0.25);
    let publication = runtime.shared_telemetry.video_render_sampling.read().unwrap();
    assert_eq!(publication.validate(&frame.fence, frame.project_render_epoch, runtime.last_tick), VideoRenderSampleValidation::SemanticStale);
}

#[test]
fn video_sample_pause_seek_loop_countin_external_and_unknown_source_remain_canonical() {
    for reason in ["paused", "count-in", "external-sync", "root-loop", "source-duration-unavailable", "clip-fade"] {
        let mut runtime = runtime();
        match reason {
            "paused" => runtime.timeline_playing = false,
            "count-in" => runtime.timeline_count_in_until = Some(runtime.last_tick + Duration::from_secs(1)),
            "external-sync" => runtime.timeline_external_sync_source = Some(ClockSource::MidiTimecode),
            "root-loop" => runtime.timeline_loop_runtime.status = TimelineLoopRuntimeStatus::Armed,
            "clip-fade" => runtime.timeline_video_clips[0].fade_in_ms = 100,
            _ => runtime.media_assets[0].source.metadata.as_mut().unwrap().duration_ms = None,
        }
        let snapshot = publish(&runtime, true);
        let frame = sample(&runtime, &snapshot, runtime.last_tick + Duration::from_secs(10)).unwrap().unwrap();
        assert_eq!(frame.mode, VideoSamplingMode::Canonical(reason));
        assert_eq!(frame.video, snapshot.video);
    }
    let mut runtime = runtime(); let snapshot = publish(&runtime, true);
    let frame = sample(&runtime, &snapshot, runtime.last_tick).unwrap().unwrap();
    runtime.timeline_transport_generation += 1;
    publish(&runtime, true);
    let publication = runtime.shared_telemetry.video_render_sampling.read().unwrap();
    assert_eq!(publication.validate(&frame.fence, frame.project_render_epoch, runtime.last_tick), VideoRenderSampleValidation::SemanticStale);
    assert_eq!(publication.validate(&frame.fence, frame.project_render_epoch + 1, runtime.last_tick), VideoRenderSampleValidation::SemanticStale);
}

#[test]
fn video_sample_dj_link_reserves_sidecar_before_publication_and_retires_after_success() {
    let mut runtime = runtime();
    let old_snapshot = publish(&runtime, true);
    let old_frame = sample(&runtime, &old_snapshot, runtime.last_tick).unwrap().unwrap();
    let snapshot = RwLock::new(old_snapshot.clone());
    let shared = Arc::clone(&runtime.shared_telemetry);
    let blocked = shared.video_render_sampling.read().unwrap();
    let mut prepared = old_snapshot.clone();
    prepared.clock.bpm = 137.0;
    assert!(!runtime.try_publish_dj_link_snapshot(&snapshot, None, prepared.clone(), &mut None, &mut None, &mut None));
    assert_eq!(*snapshot.read().unwrap(), old_snapshot);
    assert_eq!(blocked.generation, old_frame.fence.generation);
    drop(blocked);
    assert!(runtime.try_publish_dj_link_snapshot(&snapshot, None, prepared.clone(), &mut None, &mut None, &mut None));
    assert_eq!(*snapshot.read().unwrap(), prepared);
    let publication = shared.video_render_sampling.read().unwrap();
    assert_eq!(publication.validate(&old_frame.fence, old_frame.project_render_epoch, runtime.last_tick), VideoRenderSampleValidation::SemanticStale);
    assert_eq!(publication.mode, VideoSamplingMode::Canonical("command-publication"));
}

#[test]
fn video_sample_clip_take_queue_and_auto_vj_never_mix_with_root_interpolation() {
    let mut runtime = runtime();
    let mut snapshot = runtime.build_snapshot(0);
    let mut clip_runtime = protocol::VideoClipLayerRuntimeSummary::default();
    clip_runtime.layer_id = 900;
    clip_runtime.transition = Some(protocol::VideoClipTakeTransitionSummary {
        origin_slot_id: protocol::VideoClipSlotId(1), outgoing_slot_id: protocol::VideoClipSlotId(1), incoming_slot_id: protocol::VideoClipSlotId(2),
        kind: protocol::VideoClipTakeKind::Crossfade, elapsed_ms: 10, duration_ms: 100,
        duration: protocol::VideoClipTakeDuration::milliseconds(100), progress_millis: 100,
        incoming_playhead_ms: 10, incoming_playing: true,
    });
    snapshot.video_clip_runtime.layers.push(clip_runtime);
    assert_eq!(runtime.video_sampling_window(&snapshot).0, VideoSamplingMode::Canonical("independent-video-launch"));
    snapshot.video_clip_runtime.layers.last_mut().unwrap().transition = None;
    snapshot.video_clip_runtime.layers.last_mut().unwrap().queued_slot_id = Some(protocol::VideoClipSlotId(2));
    assert_eq!(runtime.video_sampling_window(&snapshot).0, VideoSamplingMode::Canonical("independent-video-launch"));
    snapshot.video_clip_runtime.layers.pop();
    runtime.auto_vj.status.armed = true;
    assert_eq!(runtime.video_sampling_window(&snapshot).0, VideoSamplingMode::Canonical("independent-video-launch"));
}

#[test]
fn video_sample_cold_decode_delivery_outlives_capture_and_expires_at_500ms_from_capture() {
    let runtime = runtime();
    let snapshot = publish(&runtime, true);
    let captured_at = runtime.last_tick + Duration::from_millis(30);
    let frame = sample(&runtime, &snapshot, captured_at).unwrap().unwrap();
    let delivery_deadline = captured_at + VIDEO_SAMPLE_DELIVERY_MAX_AGE;
    assert_eq!(frame.fence.deadline, Some(delivery_deadline));
    assert!(sample(&runtime, &snapshot, runtime.last_tick + VIDEO_SAMPLE_PUBLICATION_MAX_AGE).unwrap().is_none());
    let publication = runtime.shared_telemetry.video_render_sampling.read().unwrap();
    assert_eq!(publication.validate(&frame.fence, frame.project_render_epoch, runtime.last_tick + Duration::from_millis(120)),
        VideoRenderSampleValidation::Current, "cold decode may cross capture expiry and the former artificial 100ms cap");
    assert_eq!(publication.validate(&frame.fence, frame.project_render_epoch, delivery_deadline - Duration::from_nanos(1)),
        VideoRenderSampleValidation::Current);
    assert_eq!(publication.validate(&frame.fence, frame.project_render_epoch, delivery_deadline),
        VideoRenderSampleValidation::DeadlineExpired);
}

#[test]
fn video_sample_delivery_survives_timely_ticks_but_not_a_publication_gap() {
    let mut runtime = runtime();
    let start = runtime.last_tick;
    let snapshot = publish(&runtime, true);
    runtime.fence_video_presentation_config_after_publication();
    let frame = sample(&runtime, &snapshot, start).unwrap().unwrap();
    for index in 1..=8 {
        runtime.last_tick = start + Duration::from_millis(index * 20);
        runtime.timeline_position_ms = 100 + index * 20;
        publish(&runtime, true);
        let publication = runtime.shared_telemetry.video_render_sampling.read().unwrap();
        assert_eq!(publication.validate(&frame.fence, frame.project_render_epoch, runtime.last_tick),
            VideoRenderSampleValidation::Current, "ordinary continuous ticks preserve an admitted delivery");
    }
    runtime.last_tick += VIDEO_SAMPLE_PUBLICATION_MAX_AGE;
    runtime.timeline_position_ms += VIDEO_SAMPLE_PUBLICATION_MAX_AGE.as_millis() as u64;
    publish(&runtime, true);
    let publication = runtime.shared_telemetry.video_render_sampling.read().unwrap();
    assert_eq!(publication.validate(&frame.fence, frame.project_render_epoch, runtime.last_tick),
        VideoRenderSampleValidation::SemanticStale, "publishing at the old capture deadline still retires its generation");
}

#[test]
fn video_sample_delivery_cannot_cross_real_semantic_boundaries_after_100ms() {
    for boundary in ["clip-end", "future-start", "source-end", "root-end", "follow", "event"] {
        let mut runtime = runtime();
        match boundary {
            "clip-end" => runtime.timeline_video_clips[0].duration_ms = 300,
            "future-start" => {
                let mut next = runtime.timeline_video_clips[0].clone();
                next.id = TimelineVideoClipId(99); next.start_ms = 300;
                runtime.timeline_video_clips.push(next);
            }
            "source-end" => runtime.media_assets[0].source.metadata.as_mut().unwrap().duration_ms = Some(700),
            "root-end" => for clip in &mut runtime.timeline_video_clips { clip.duration_ms = 300; },
            "follow" => runtime.timeline_follow = Some(serde_json::from_value(serde_json::json!({
                "enabled": true, "next_timeline_id": 2, "preroll_ms": 4_700
            })).unwrap()),
            _ => runtime.timeline_events.push(RuntimeTimelineEvent {
                id: 1, cue_id: 1, time_ms: 300, time_beats: None, track: TimelineTrackKind::Lighting,
                layer_id: None, resolved_layer_id: 1, layer_order: 0, layer_muted_effective: false,
                duration_ms: 100, duration_beats: None, conform_to_tempo: false, loop_fill: false,
                source_offset_ms: 0, iteration_period_ms: 100, rate: Some(1.0), fade_in_ms: 0, fade_out_ms: 0,
                loop_count: 1, jump_to_event_id: None,
            }),
        }
        let snapshot = publish(&runtime, true);
        let frame = sample(&runtime, &snapshot, runtime.last_tick + Duration::from_millis(10)).unwrap().unwrap();
        let deadline = runtime.last_tick + Duration::from_millis(200);
        assert_eq!(frame.fence.deadline, Some(deadline), "{boundary}");
        let publication = runtime.shared_telemetry.video_render_sampling.read().unwrap();
        assert_eq!(publication.validate(&frame.fence, frame.project_render_epoch, deadline - Duration::from_nanos(1)),
            VideoRenderSampleValidation::Current, "{boundary}");
        assert_eq!(publication.validate(&frame.fence, frame.project_render_epoch, deadline),
            VideoRenderSampleValidation::DeadlineExpired, "{boundary}");
    }
}

#[test]
fn video_sample_expiry_is_bounded_and_future_anchor_or_generation_exhaustion_rejects() {
    let runtime = runtime(); let snapshot = publish(&runtime, true);
    assert!(sample(&runtime, &snapshot, runtime.last_tick - Duration::from_nanos(1)).is_err());
    assert!(sample(&runtime, &snapshot, runtime.last_tick + VIDEO_SAMPLE_PUBLICATION_MAX_AGE).unwrap().is_none());
    assert!(sample(&runtime, &snapshot, runtime.last_tick + Duration::from_secs(60)).unwrap().is_none());
    runtime.shared_telemetry.video_render_sampling.write().unwrap().generation = u64::MAX;
    publish(&runtime, false);
    assert!(sample(&runtime, &snapshot, runtime.last_tick).unwrap_err().contains("exhausted"));
}

#[test]
fn video_sample_follow_admission_and_jump_end_stop_before_boundary() {
    let mut runtime = runtime();
    runtime.timeline_follow = Some(serde_json::from_value(serde_json::json!({
        "enabled": true, "next_timeline_id": 2, "preroll_ms": 4_880
    })).unwrap());
    let snapshot = publish(&runtime, true);
    assert!(sample(&runtime, &snapshot, runtime.last_tick + Duration::from_millis(20)).unwrap().is_none());
    runtime.timeline_position_ms = 120;
    let snapshot = publish(&runtime, true);
    assert_eq!(sample(&runtime, &snapshot, runtime.last_tick).unwrap().unwrap().mode, VideoSamplingMode::Canonical("follow-admission"));
    runtime.timeline_follow = None;
    runtime.timeline_position_ms = 100;
    runtime.timeline_events.push(RuntimeTimelineEvent {
        id: 1, cue_id: 1, time_ms: 0, time_beats: None, track: TimelineTrackKind::Lighting,
        layer_id: None, resolved_layer_id: 1, layer_order: 0, layer_muted_effective: false,
        duration_ms: 120, duration_beats: None, conform_to_tempo: false, loop_fill: false,
        source_offset_ms: 0, iteration_period_ms: 120, rate: Some(1.0), fade_in_ms: 0, fade_out_ms: 0,
        loop_count: 1, jump_to_event_id: Some(2),
    });
    let snapshot = publish(&runtime, true);
    assert!(sample(&runtime, &snapshot, runtime.last_tick + Duration::from_millis(20)).unwrap().is_none());
}

#[test]
fn video_sample_poisoned_publication_is_never_revived_by_writer() {
    let runtime = runtime(); let snapshot = publish(&runtime, true);
    let shared = Arc::clone(&runtime.shared_telemetry);
    assert!(std::thread::spawn(move || {
        let _guard = shared.video_render_sampling.write().unwrap();
        panic!("intentional sample sidecar poison");
    }).join().is_err());
    runtime.publish_video_sampling(&snapshot, true);
    assert!(runtime.shared_telemetry.video_render_sampling.read().is_err());
}

#[test]
#[ignore = "set SYNDOCAL_VIDEO_SAMPLE_TEST_PROJECT to a local fixture; reads only, no engine thread or physical output"]
fn video_sample_real_project_with_lighting_child_admits_plain_root_video() {
    let path = std::env::var("SYNDOCAL_VIDEO_SAMPLE_TEST_PROJECT").expect("set SYNDOCAL_VIDEO_SAMPLE_TEST_PROJECT");
    let document: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let mut source: EngineSnapshot = serde_json::from_value(document["snapshot"].clone()).unwrap();
    source.output.enabled = false;
    let mut runtime = EngineRuntime::new(DmxOutputConfig { enabled: false, ..DmxOutputConfig::default() });
    runtime.load_project_snapshot_checked(source).unwrap();
    runtime.timeline_playing = true;
    runtime.timeline_position_ms = 20_000;
    // Fixture-only availability verdict; this test proves source structure and
    // sampling admission, not machine hash verification or decoder operation.
    for asset in &runtime.media_assets {
        runtime.media_asset_availability.write().unwrap().insert(asset.id, true);
    }
    let snapshot = publish(&runtime, true);
    let frame = sample(&runtime, &snapshot, runtime.last_tick + Duration::from_millis(10)).unwrap().unwrap();
    assert_eq!(frame.mode, VideoSamplingMode::ContinuousRootTimeline);
    assert_eq!(frame.video.layers.len(), 2);
    assert!(frame.video.layers.iter().all(|layer| layer.state.position_ms == 20_010));
    assert!(runtime.cues.iter().any(|cue| cue.child_timeline.is_some()), "real fixture must retain its lighting child");
}
