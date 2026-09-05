use super::*;

fn populated_runtime() -> EngineRuntime {
    let mut runtime =
        direct_child_static_test_runtime(vec![direct_child_static_event(201, 2, 0, 10_000)]);
    runtime.request_cue(1, runtime.last_tick);
    runtime.advance_pending_cue(runtime.last_tick);
    assert!(!runtime
        .active_child_timeline_transport_summaries()
        .is_empty());
    runtime.timeline_audio_clips = (0..64)
        .map(|index| timeline_test_audio_clip(1_000 + index, 30))
        .collect();
    runtime.timeline_audio_clips_derived = true;
    runtime.timeline_playing = true;
    runtime.timeline_position_ms = 875;
    runtime.timeline_transport_epoch = 7;
    runtime.timeline_transport_generation = 8;
    runtime.timeline_audio_transport_revision = 9;
    runtime.timeline_count_in_until = Some(runtime.last_tick + Duration::from_secs(1));
    runtime.timeline_loop_runtime.generation = 10;
    runtime.timeline_loop_runtime.wrap_count = 3;
    runtime.timeline_follow_runtime.generation = 11;
    runtime.timeline_follow_runtime.fault = Some("retired transition".repeat(16));
    runtime.timeline_guide_cues = (0..128)
        .map(|index| TimelineGuideCueSummary {
            generation: 12,
            sequence: index,
            at_ms: index * 500,
            label: format!("Guide {index}: {}", "authored label ".repeat(8)),
            cue: TimelineGuideCueKind::Break,
            asset: TimelineGuideAssetKey::Verse,
            playback_rate_milli: 1_000,
            sample_frame: 0,
            epoch: 7,
            transport_generation: 8,
            schedule_generation: 12,
            source: TimelineScheduleSource::Root,
        })
        .collect();
    let authority = TimelineTempoMeterAuthority::new(120.0, Vec::new()).unwrap();
    runtime
        .timeline_click_scheduler
        .configure_authority(authority.clone())
        .unwrap();
    runtime
        .timeline_click_scheduler
        .rearm(authority, 0, 128)
        .unwrap();
    assert!(!runtime.timeline_click_scheduler.queued_events().is_empty());
    runtime
}

fn assert_previous_projection(runtime: &EngineRuntime) {
    let live = previous_timeline_snapshot(runtime);
    let authored = previous_authored_timeline_snapshot(runtime);
    assert_eq!(runtime.timeline_snapshot(), live);
    assert_eq!(runtime.authored_timeline_snapshot(), authored);
    // Repeated authored reads must not consume or reset any live projection.
    assert_eq!(runtime.timeline_snapshot(), live);
}

#[test]
fn timeline_snapshot_projection_matches_previous_empty_legacy_explicit_and_live_state() {
    assert_previous_projection(&runtime_with_lfo_effects(&[]));
    let mut runtime = populated_runtime();
    for explicit in [false, true] {
        runtime.timeline_layers = if explicit {
            vec![timeline_test_layer(
                10,
                0,
                false,
                false,
                false,
                TimelineLayerKind::Lighting,
            )]
        } else {
            Vec::new()
        };
        for derived in [false, true] {
            runtime.timeline_audio_clips_derived = derived;
            assert_previous_projection(&runtime);
            let authored = runtime.authored_timeline_snapshot();
            assert!(authored.layers.iter().any(|layer| layer.id == 30));
            assert_eq!(authored.audio_clips.is_empty(), derived);
            assert!(authored.active_child_transports.is_empty());
            assert!(authored.guide_cues.is_empty());
            assert!(authored.click_events.is_empty());
        }
    }
    // Overflow is a separately owned runtime diagnostic. Authored reads omit
    // it without changing the scheduler's failed-admission evidence or queue.
    assert!(runtime
        .timeline_click_scheduler
        .rearm(
            runtime.timeline_click_scheduler.authority.clone().unwrap(),
            0,
            (TIMELINE_CLICK_BATCH_CAPACITY as u64 + 1) * TIMELINE_CLICK_QUARTER_UNITS,
        )
        .is_err());
    assert!(runtime.timeline_click_scheduler.overflow().is_some());
    assert_previous_projection(&runtime);
    assert!(runtime
        .authored_timeline_snapshot()
        .click_queue_overflow
        .is_none());
}

#[test]
#[ignore = "fixed projection workload; not real-show FPS or publication-lock acceptance"]
fn benchmark_timeline_snapshot_projection_modes() {
    use std::hint::black_box;

    fn measure(runtime: &EngineRuntime, build: fn(&EngineRuntime) -> TimelineSnapshot) -> Duration {
        let started = Instant::now();
        for _ in 0..1_000 {
            black_box(build(black_box(runtime)));
        }
        started.elapsed()
    }
    let light = runtime_with_lfo_effects(&[]);
    let populated = populated_runtime();
    for (name, runtime) in [("light", &light), ("populated", &populated)] {
        for (mode, old, new) in [
            (
                "live",
                previous_timeline_snapshot as fn(&EngineRuntime) -> TimelineSnapshot,
                EngineRuntime::timeline_snapshot as fn(&EngineRuntime) -> TimelineSnapshot,
            ),
            (
                "authored",
                previous_authored_timeline_snapshot as fn(&EngineRuntime) -> TimelineSnapshot,
                EngineRuntime::authored_timeline_snapshot as fn(&EngineRuntime) -> TimelineSnapshot,
            ),
        ] {
            assert_eq!(old(runtime), new(runtime));
            let mut previous_times = Vec::new();
            let mut current_times = Vec::new();
            for round in 0..3 {
                let (previous, current) = if round % 2 == 0 {
                    (measure(runtime, old), measure(runtime, new))
                } else {
                    let current = measure(runtime, new);
                    (measure(runtime, old), current)
                };
                println!("timeline {name}/{mode} round {round}: previous={previous:?}, current={current:?}");
                previous_times.push(previous);
                current_times.push(current);
            }
            previous_times.sort_unstable();
            current_times.sort_unstable();
            println!(
                "timeline {name}/{mode}, 1000 snapshots median: previous={:?}, current={:?}",
                previous_times[1], current_times[1]
            );
        }
    }
}

// Exact pre-split construction and clearing are retained only as test oracles.
fn previous_timeline_snapshot(runtime: &EngineRuntime) -> TimelineSnapshot {
    let mut layers = if runtime.timeline_layers.is_empty() {
        implicit_timeline_layers()
    } else {
        runtime.timeline_layers.clone()
    };
    for clip in &runtime.timeline_audio_clips {
        if !layers.iter().any(|layer| layer.id == clip.layer_id) {
            layers.push(default_timeline_audio_layer(clip.layer_id));
        }
    }
    layers.sort_by_key(|layer| (layer.kind.display_section_rank(), layer.order, layer.id));
    TimelineSnapshot {
        id: runtime.timeline_id,
        label: runtime.timeline_label.clone(),
        layers,
        events: runtime
            .timeline_events
            .iter()
            .map(timeline_event_summary)
            .collect(),
        automations: runtime
            .timeline_automations
            .iter()
            .map(timeline_automation_summary)
            .collect(),
        video_automations: runtime
            .timeline_video_automations
            .iter()
            .map(timeline_video_automation_summary)
            .collect(),
        audio: runtime.timeline_audio.clone(),
        audio_clips: runtime.timeline_audio_clips.clone(),
        video_clips: runtime.timeline_video_clips.clone(),
        phases: runtime.timeline_phases.clone(),
        item_groups: runtime.timeline_item_groups.clone(),
        loop_region: runtime.timeline_loop_region.clone(),
        follow: runtime.timeline_follow.clone(),
        guide_enabled: runtime.timeline_guide_enabled,
        audio_offset_ms: runtime.timeline_audio_offset_ms,
        audio_muted: runtime.timeline_audio_muted,
        metronome_enabled: runtime.timeline_metronome_enabled,
        count_in_beats: runtime.timeline_count_in_beats,
        tempo_meter_map: runtime.timeline_tempo_meter_map.clone(),
        tempo_meter_map_version: runtime.timeline_tempo_meter_map_version,
        count_in_remaining_ms: runtime
            .timeline_count_in_until
            .map(|until| {
                until
                    .saturating_duration_since(runtime.last_tick)
                    .as_millis() as u64
            })
            .unwrap_or(0),
        audio_transport_revision: runtime.timeline_audio_transport_revision,
        transport_epoch: runtime.timeline_transport_epoch,
        transport_generation: runtime.timeline_transport_generation,
        active_child_transports: runtime.active_child_timeline_transport_summaries(),
        loop_runtime: runtime.timeline_loop_runtime.clone(),
        follow_runtime: runtime.timeline_follow_runtime.clone(),
        guide_cues: runtime.timeline_guide_cues.clone(),
        click_events: runtime
            .timeline_click_scheduler
            .queued_events()
            .iter()
            .copied()
            .collect(),
        click_schedule_generation: runtime
            .timeline_click_scheduler
            .identity()
            .schedule_generation,
        click_queue_overflow: runtime
            .timeline_click_scheduler
            .overflow()
            .map(ToOwned::to_owned),
        playing: runtime.timeline_playing,
        position_ms: runtime.timeline_position_ms,
        duration_ms: runtime.timeline_duration_ms(),
    }
}

fn previous_authored_timeline_snapshot(runtime: &EngineRuntime) -> TimelineSnapshot {
    let mut timeline = previous_timeline_snapshot(runtime);
    timeline.playing = false;
    timeline.position_ms = 0;
    timeline.count_in_remaining_ms = 0;
    timeline.audio_transport_revision = 0;
    timeline.transport_epoch = 0;
    timeline.transport_generation = 0;
    timeline.active_child_transports.clear();
    timeline.loop_runtime = TimelineLoopRuntimeSummary::default();
    timeline.follow_runtime = TimelineFollowRuntimeSummary::default();
    timeline.guide_cues.clear();
    timeline.click_events.clear();
    timeline.click_schedule_generation = 0;
    timeline.click_queue_overflow = None;
    if runtime.timeline_audio_clips_derived {
        timeline.audio_clips.clear();
    }
    timeline
}
