use super::*;

fn previous_persistence_timelines(
    runtime: &EngineRuntime,
) -> (TimelineSnapshot, Vec<TimelineSnapshot>) {
    let snapshot = runtime.build_snapshot(0);
    let mut timeline = snapshot.timeline;
    timeline.layers = runtime.timeline_layers.clone();
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
    for (summary, event) in timeline.events.iter_mut().zip(&runtime.timeline_events) {
        summary.layer_id = event.layer_id;
        summary.track = event.track.clone();
    }
    // The old path cloned the fully normalized root to replace the active bank
    // entry; its second runtime reset was idempotent on this normalized image.
    let mut bank = snapshot.timeline_bank;
    if let Some(active) = bank.iter_mut().find(|entry| entry.id == timeline.id) {
        *active = timeline.clone();
    } else {
        bank.push(timeline.clone());
    }
    (timeline, bank)
}

#[test]
fn persistence_timeline_reuse_matches_previous_legacy_explicit_and_live_bank_images() {
    for (explicit_layers, derived_audio) in [(false, true), (true, false), (true, true)] {
        let mut runtime = runtime_with_lfo_effects(&[]);
        runtime.timeline_id = TimelineId(42);
        if explicit_layers {
            runtime.timeline_layers = vec![
                timeline_test_layer(10, 0, false, false, false, TimelineLayerKind::Lighting),
                timeline_test_layer(20, 1, false, false, false, TimelineLayerKind::Video),
                timeline_test_layer(30, 2, false, false, false, TimelineLayerKind::Audio),
            ];
        }
        create_effect_only_cue(&mut runtime, 1, Vec::new());
        create_effect_only_cue(&mut runtime, 2, Vec::new());
        runtime
            .add_timeline_cue_event_state(100, 1, 0, None, TimelineTrackKind::Lighting, None)
            .unwrap();
        runtime
            .add_timeline_cue_event_state(200, 2, 10, None, TimelineTrackKind::Video, None)
            .unwrap();
        runtime.timeline_audio_clips = vec![timeline_test_audio_clip(80, 30)];
        runtime.timeline_audio_clips_derived = derived_audio;
        runtime.timeline_audio_offset_ms = -125;
        runtime.timeline_audio_muted = true;
        runtime.timeline_playing = true;
        runtime.timeline_position_ms = 875;
        runtime.timeline_transport_epoch = 7;
        runtime.timeline_transport_generation = 8;
        runtime.timeline_audio_transport_revision = 9;
        runtime.timeline_count_in_until = Some(runtime.last_tick + Duration::from_secs(1));
        runtime.timeline_loop_runtime.generation = 10;
        runtime.timeline_loop_runtime.wrap_count = 3;
        runtime.timeline_follow_runtime.generation = 11;
        runtime.timeline_follow_runtime.fault = Some("retired transition".into());
        runtime.timeline_guide_cues.push(TimelineGuideCueSummary {
            generation: 12,
            sequence: 1,
            at_ms: 0,
            label: "Live guide".into(),
            cue: TimelineGuideCueKind::Break,
            asset: TimelineGuideAssetKey::Verse,
            playback_rate_milli: 1_000,
            sample_frame: 0,
            epoch: 7,
            transport_generation: 8,
            schedule_generation: 12,
            source: TimelineScheduleSource::Root,
        });

        // Internal duplicate states retain first-match semantics; validation of
        // authored project input is unchanged and still rejects duplicate IDs.
        for ids in [
            vec![],
            vec![1, 2],
            vec![42],
            vec![42, 2],
            vec![1, 42],
            vec![1, 42, 42, 2],
        ] {
            runtime.timeline_bank = ids
                .iter()
                .enumerate()
                .map(|(index, id)| TimelineSnapshot {
                    id: TimelineId(*id),
                    label: format!("Stored bank {index}"),
                    playing: true,
                    position_ms: index as u64 + 99,
                    ..TimelineSnapshot::default()
                })
                .collect();
            let before = runtime.build_snapshot(0);
            assert!(before.timeline.playing);
            assert_eq!(before.timeline.guide_cues.len(), 1);
            assert_eq!(before.timeline.audio_clips.len(), 1);
            let expected = previous_persistence_timelines(&runtime);
            let persisted = runtime.build_persistence_snapshot();
            assert_eq!(
                (&persisted.timeline, &persisted.timeline_bank),
                (&expected.0, &expected.1),
                "explicit={explicit_layers}, derived={derived_audio}, bank={ids:?}"
            );
            assert_eq!(persisted.timeline.layers.is_empty(), !explicit_layers);
            assert_eq!(persisted.timeline.audio_clips.is_empty(), derived_audio);
            assert_eq!(
                persisted
                    .timeline
                    .events
                    .iter()
                    .map(|event| (event.layer_id, event.track.clone()))
                    .collect::<Vec<_>>(),
                runtime
                    .timeline_events
                    .iter()
                    .map(|event| (event.layer_id, event.track.clone()))
                    .collect::<Vec<_>>()
            );
            let active_index = ids.iter().position(|id| *id == 42).unwrap_or(ids.len());
            assert_eq!(persisted.timeline_bank[active_index], persisted.timeline);
            assert_eq!(runtime.build_snapshot(0), before);
        }
    }
}
