use super::*;

fn projection_runtime() -> EngineRuntime {
    let mut runtime = runtime_with_lfo_effects(&[]);
    runtime.timeline_layers = vec![timeline_test_layer(
        60,
        0,
        false,
        false,
        false,
        TimelineLayerKind::Video,
    )];
    runtime.media_assets = vec![media_asset_test_summary(
        100,
        "Projection",
        VideoSourceSummary {
            kind: VideoSourceKind::File,
            path: Some("C:/show/projection.mp4".into()),
            name: None,
            codec: None,
            metadata: None,
        },
    )];
    runtime.timeline_video_clips = vec![TimelineVideoClipSummary {
        id: TimelineVideoClipId(1_001),
        layer_id: 60,
        media_asset_id: 100,
        start_ms: 10,
        offset_ms: 30,
        duration_ms: 1_000,
        fade_in_ms: 0,
        fade_out_ms: 0,
    }];
    runtime.timeline_position_ms = 100;
    runtime
        .media_asset_availability
        .write()
        .unwrap()
        .insert(100, true);
    runtime
}

fn assert_projection_matches_previous(runtime: &EngineRuntime) {
    let mut previous_ids = HashSet::from([1]);
    let mut current_ids = previous_ids.clone();
    let previous = previous_projection(
        runtime,
        runtime.timeline_id,
        &runtime.timeline_video_clips,
        &runtime.timeline_layers,
        runtime.timeline_position_ms,
        true,
        &mut previous_ids,
    );
    let current = runtime.timeline_video_projection_layers_with_used_ids(
        runtime.timeline_id,
        &runtime.timeline_video_clips,
        &runtime.timeline_layers,
        runtime.timeline_position_ms,
        true,
        &mut current_ids,
    );
    assert_eq!(previous_ids, current_ids);
    let fields = |items: Vec<RuntimeTimelineVideoProjection>| {
        items
            .into_iter()
            .map(|item| {
                (
                    item.timeline_id,
                    item.timeline_layer_id,
                    item.layer_order,
                    item.clip_id,
                    item.layer,
                )
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(fields(previous), fields(current));
}

#[test]
fn empty_video_projection_matches_previous_empty_inactive_muted_and_available_paths() {
    for staged in [false, true] {
        let mut runtime = projection_runtime();
        if staged {
            runtime.staged_media_asset_availability_for_publication =
                Some(HashMap::from([(100, true)]));
        }
        let lanes = runtime.timeline_layers.clone();
        runtime.timeline_layers.clear();
        assert_projection_matches_previous(&runtime);
        runtime.timeline_layers = lanes;
        runtime.timeline_position_ms = 0;
        assert_projection_matches_previous(&runtime);
        runtime.timeline_position_ms = 100;
        runtime.timeline_layers[0].muted = true;
        assert_projection_matches_previous(&runtime);
        runtime.timeline_layers[0].muted = false;
        assert_projection_matches_previous(&runtime);
        assert_eq!(
            runtime
                .timeline_video_projection_layers(
                    runtime.timeline_id,
                    &runtime.timeline_video_clips,
                    &runtime.timeline_layers,
                    100,
                    true
                )
                .len(),
            1
        );
        runtime
            .media_asset_availability
            .write()
            .unwrap()
            .insert(100, false);
        if staged {
            runtime.staged_media_asset_availability_for_publication =
                Some(HashMap::from([(100, false)]));
        }
        assert_projection_matches_previous(&runtime);
        runtime.media_assets.clear();
        assert_projection_matches_previous(&runtime);
    }
}

#[test]
#[ignore = "fixed availability-copy workload; not actual media rendering or FPS acceptance"]
fn benchmark_empty_video_projection_availability_copy() {
    use std::hint::black_box;
    fn old(runtime: &EngineRuntime) -> Vec<RuntimeTimelineVideoProjection> {
        previous_projection(
            runtime,
            runtime.timeline_id,
            &runtime.timeline_video_clips,
            &runtime.timeline_layers,
            runtime.timeline_position_ms,
            true,
            &mut HashSet::new(),
        )
    }
    fn current(runtime: &EngineRuntime) -> Vec<RuntimeTimelineVideoProjection> {
        runtime.timeline_video_projection_layers_with_used_ids(
            runtime.timeline_id,
            &runtime.timeline_video_clips,
            &runtime.timeline_layers,
            runtime.timeline_position_ms,
            true,
            &mut HashSet::new(),
        )
    }
    fn measure(
        runtime: &EngineRuntime,
        project: fn(&EngineRuntime) -> Vec<RuntimeTimelineVideoProjection>,
    ) -> Duration {
        let started = Instant::now();
        for _ in 0..1_000 {
            black_box(project(black_box(runtime)));
        }
        started.elapsed()
    }
    for entries in [1, 4_096] {
        for mode in ["no-lanes", "no-active-clips", "active"] {
            let mut runtime = projection_runtime();
            *runtime.media_asset_availability.write().unwrap() =
                (100..100 + entries).map(|id| (id, true)).collect();
            if mode == "no-lanes" {
                runtime.timeline_layers.clear();
            }
            if mode == "no-active-clips" {
                runtime.timeline_position_ms = 0;
            }
            assert_projection_matches_previous(&runtime);
            let mut old_times = Vec::new();
            let mut new_times = Vec::new();
            for round in 0..3 {
                let (before, after) = if round % 2 == 0 {
                    (measure(&runtime, old), measure(&runtime, current))
                } else {
                    let after = measure(&runtime, current);
                    (measure(&runtime, old), after)
                };
                println!("projection {entries}/{mode} round {round}: previous={before:?}, current={after:?}");
                old_times.push(before);
                new_times.push(after);
            }
            old_times.sort_unstable();
            new_times.sort_unstable();
            println!(
                "projection {entries}/{mode} median1000: previous={:?}, current={:?}",
                old_times[1], new_times[1]
            );
        }
    }
}

fn previous_projection(
    runtime: &EngineRuntime,
    timeline_id: TimelineId,
    clips: &[TimelineVideoClipSummary],
    timeline_layers: &[TimelineLayerSummary],
    position_ms: u64,
    playing: bool,
    used_ids: &mut HashSet<VideoLayerId>,
) -> Vec<RuntimeTimelineVideoProjection> {
    let media_asset_availability = runtime.media_asset_availability_for_current_publication();
    let video_lanes = timeline_layers
        .iter()
        .filter(|layer| matches!(layer.kind, TimelineLayerKind::Video))
        .map(|layer| (layer.id, layer))
        .collect::<BTreeMap<_, _>>();
    if video_lanes.is_empty() {
        // Empty `timeline_layers` is the legacy representation.  In that
        // form the old layer takeover path remains authoritative below.
        return Vec::new();
    }

    let has_video_solo = video_lanes.values().any(|layer| layer.solo);
    let mut desired = BTreeMap::<u32, (TimelineVideoClipSummary, &TimelineLayerSummary)>::new();
    for clip in clips.iter().filter(|clip| {
        clip.start_ms <= position_ms && position_ms < clip.start_ms.saturating_add(clip.duration_ms)
    }) {
        let Some(layer) = video_lanes.get(&clip.layer_id).copied() else {
            // A malformed/legacy clip is not allowed to become a VJ
            // layer by numeric coincidence.  Authoring validation owns
            // the durable error; runtime rendering fails closed here.
            continue;
        };
        if layer.muted || (has_video_solo && !layer.solo) {
            continue;
        }
        match desired.get(&clip.layer_id) {
            Some((current, _))
                if (current.start_ms, current.id.0) >= (clip.start_ms, clip.id.0) => {}
            _ => {
                desired.insert(clip.layer_id, (clip.clone(), layer));
            }
        }
    }

    let mut projections = desired
        .into_iter()
        .filter_map(|(timeline_layer_id, (clip, timeline_layer))| {
            let asset = runtime
                .media_assets
                .iter()
                .find(|asset| asset.id == clip.media_asset_id)?;
            // Availability is established by the media-asset authority
            // before authoring.  A catalog row without a local path is
            // nevertheless never handed to the renderer or audio worker.
            if !timeline_media_asset_source_is_uri(asset)
                && !media_asset_availability
                    .get(&asset.id)
                    .copied()
                    .unwrap_or(false)
            {
                return None;
            }
            let id = timeline_video_runtime_projection_id(clip.id, used_ids)?;
            let position_ms = clip
                .offset_ms
                .saturating_add(position_ms.saturating_sub(clip.start_ms));
            let label = if timeline_layer.label.trim().is_empty() {
                asset.label.clone()
            } else {
                format!("{} · {}", timeline_layer.label.trim(), asset.label)
            };
            Some(RuntimeTimelineVideoProjection {
                timeline_id,
                timeline_layer_id,
                layer_order: timeline_layer.order,
                clip_id: clip.id,
                layer: VideoLayerSummary {
                    id,
                    label,
                    source: asset.source.clone(),
                    media_asset_id: Some(asset.id),
                    blend_mode: VideoBlendMode::Normal,
                    state: VideoLayerState {
                        playing,
                        position_ms,
                        ..VideoLayerState::default()
                    },
                    isf_effect: None,
                    clip_slots: Vec::new(),
                    default_clip_slot_id: None,
                },
            })
        })
        .collect::<Vec<_>>();
    projections.sort_by_key(|projection| {
        (
            projection.layer_order,
            projection.timeline_layer_id,
            projection.clip_id,
        )
    });
    projections
}
