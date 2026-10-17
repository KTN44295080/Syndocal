//! DEC-PLAYBACK-LAYER-001: playback never moves the authored project image,
//! live output is unchanged, Undo/Redo keeps the playback layer, a Standby
//! applies the Primary's, and opening a file starts with no Cue live.
use super::*;
use protocol::PlaybackLayerSnapshot;

/// The authored image as the app's save projection sees it: authored video,
/// without runtime telemetry, previews, programmer, Auto VJ status or clock
/// phase.
fn authored_image(runtime: &EngineRuntime) -> EngineSnapshot {
    let mut snapshot = runtime.build_persistence_snapshot();
    if let Some(authored) = snapshot.authored_video.clone() {
        snapshot.video = authored;
    }
    snapshot.video.auto_vj.status = Default::default();
    if let Some(authored) = snapshot.authored_video.as_mut() {
        authored.auto_vj.status = Default::default();
    }
    snapshot.telemetry = Default::default();
    snapshot.dmx_preview.clear();
    snapshot.dmx_previews.clear();
    snapshot.programmer = Default::default();
    snapshot.clock = ClockSnapshot {
        bpm: snapshot.clock.bpm,
        ..ClockSnapshot::default()
    };
    snapshot
}

fn image_diff(path: &str, a: &serde_json::Value, b: &serde_json::Value, out: &mut Vec<String>) {
    use serde_json::Value;
    match (a, b) {
        (Value::Object(left), Value::Object(right)) => {
            let mut keys = left.keys().chain(right.keys()).collect::<Vec<_>>();
            keys.sort();
            keys.dedup();
            for key in keys {
                match (left.get(key), right.get(key)) {
                    (Some(l), Some(r)) => image_diff(&format!("{path}.{key}"), l, r, out),
                    _ => out.push(format!("{path}.{key}")),
                }
            }
        }
        (Value::Array(left), Value::Array(right)) if left.len() == right.len() => {
            for (index, (l, r)) in left.iter().zip(right).enumerate() {
                image_diff(&format!("{path}[{index}]"), l, r, out);
            }
        }
        _ if a == b => {}
        _ => out.push(format!("{path}: {a} -> {b}")),
    }
}

/// Asserts two images are equal and names the differing JSON paths.
fn assert_same_image(left: &EngineSnapshot, right: &EngineSnapshot) {
    let mut differences = Vec::new();
    image_diff(
        "",
        &serde_json::to_value(left).unwrap(),
        &serde_json::to_value(right).unwrap(),
        &mut differences,
    );
    assert!(differences.is_empty(), "images differ: {differences:#?}");
}

fn level(runtime: &EngineRuntime, attribute: &str) -> Option<u16> {
    runtime.values.get(&(1, attribute.to_string())).copied()
}

fn persisted_level(snapshot: &EngineSnapshot, attribute: &str) -> u16 {
    snapshot.fixtures[0]
        .attribute_values
        .iter()
        .find(|value| value.attribute == attribute)
        .unwrap()
        .value
}

fn settle(runtime: &mut EngineRuntime) {
    runtime.apply_active_fade(Instant::now() + Duration::from_secs(10));
}

/// Fixture 1 (Dimmer, Pan) with a fader-authored Dimmer, a disabled LFO, a
/// disabled node graph, video layer 1 and a disabled video output 7. Cue 1
/// targets all of them; Cue 2 fades Pan only; Cue 3 is a group Cue.
fn playback_show() -> EngineRuntime {
    let mut runtime = runtime_with_lfo_effects(&[(10, false)]);
    runtime.apply_command(EngineCommand::AddVideoLayer {
        layer_id: 1,
        label: "Layer 1".to_string(),
        source: VideoSourceSummary {
            kind: protocol::VideoSourceKind::File,
            path: Some("memory://clip.mp4".to_string()),
            name: None,
            codec: Some("H264".to_string()),
            metadata: None,
        },
    });
    runtime.apply_command(EngineCommand::AddVideoOutput(allocator_video_output(7)));
    let mut graph = sample_node_graph(20, 1);
    graph.enabled = false;
    runtime.apply_command(EngineCommand::UpsertNodeGraph(graph));
    runtime.apply_command(EngineCommand::SetAttribute {
        fixture_id: 1,
        attribute: "Dimmer".into(),
        value: 1_000,
    });
    runtime.apply_command(EngineCommand::CreateCue {
        authored_beats: None,
        cue_id: 1,
        label: "Look A".to_string(),
        fade_ms: 0,
        targets: vec![CueFixtureTarget {
            fixture_id: 1,
            values: vec![AttributeValueSummary {
                attribute: "Dimmer".to_string(),
                value: 40_000,
            }],
        }],
        video_targets: vec![VideoLayerTarget {
            layer_id: 1,
            state: VideoLayerState {
                opacity: 0.25,
                playing: true,
                position_ms: 250,
                ..VideoLayerState::default()
            },
        }],
        video_output_targets: vec![VideoOutputTarget {
            output_id: 7,
            enabled: true,
            opacity: 0.4,
            blackout: true,
        }],
        node_graph_targets: vec![CueNodeGraphTarget {
            graph_id: 20,
            enabled: true,
        }],
        effect_targets: vec![CueEffectTarget {
            effect_id: 10,
            enabled: true,
            params: None,
            transition_ms: None,
        }],
    });
    runtime.apply_command(EngineCommand::CreateCue {
        authored_beats: None,
        cue_id: 2,
        label: "Look B".to_string(),
        fade_ms: 1_000,
        targets: vec![CueFixtureTarget {
            fixture_id: 1,
            values: vec![AttributeValueSummary {
                attribute: "Pan".to_string(),
                value: 50_000,
            }],
        }],
        video_targets: Vec::new(),
        video_output_targets: Vec::new(),
        node_graph_targets: Vec::new(),
        effect_targets: Vec::new(),
    });
    create_group_recall_cue(
        &mut runtime,
        3,
        "front",
        RecallMode::Coexist,
        GroupRecallTarget {
            fade_ms: 0,
            attribute: "Pan",
            value: 30_000,
        },
        Vec::new(),
    );
    assert_eq!(runtime.last_error, None);
    runtime
}

fn effect_enabled(runtime: &EngineRuntime) -> bool {
    runtime
        .effects
        .iter()
        .find(|effect| effect.id == 10)
        .unwrap()
        .enabled
}

fn output_summary(runtime: &EngineRuntime) -> VideoOutputSummary {
    runtime
        .video_outputs
        .iter()
        .find(|output| output.summary.id == 7)
        .unwrap()
        .summary
        .clone()
}

fn graph_enabled(runtime: &EngineRuntime) -> bool {
    runtime
        .node_graphs
        .iter()
        .find(|graph| graph.summary.id == 20)
        .unwrap()
        .summary
        .enabled
}

#[test]
fn cue_and_timeline_playback_never_move_the_authored_image() {
    let mut runtime = playback_show();
    runtime.apply_command(EngineCommand::AddTimelineCueEvent {
        time_beats: None,
        event_id: 50,
        cue_id: 2,
        time_ms: 20,
        track: TimelineTrackKind::Lighting,
        layer_id: None,
    });
    runtime.apply_command(EngineCommand::AddTimelineAutomation {
        automation_id: 60,
        fixture_id: 1,
        attribute: "Pan".to_string(),
        keyframes: vec![
            AutomationKeyframeSummary {
                time_ms: 0,
                value: 0,
                interpolation: AutomationInterpolation::Linear,
            },
            AutomationKeyframeSummary {
                time_ms: 1_000,
                value: 60_000,
                interpolation: AutomationInterpolation::Linear,
            },
        ],
    });
    assert_eq!(runtime.last_error, None);
    let authored = authored_image(&runtime);

    // Cue 1 drives every live layer exactly as before.
    runtime.apply_command(EngineCommand::TriggerCue(1));
    settle(&mut runtime);
    assert_eq!(level(&runtime, "Dimmer"), Some(40_000));
    assert!(effect_enabled(&runtime));
    assert!(graph_enabled(&runtime));
    let output = output_summary(&runtime);
    assert!(output.enabled && output.blackout && (output.opacity - 0.4).abs() < 1e-6);
    assert_eq!(runtime.active_cue_id, Some(1));
    assert_same_image(&authored_image(&runtime), &authored);

    // A fade sampled mid-way, GO/back, toggle, group recall and release.
    runtime.apply_command(EngineCommand::TriggerCue(2));
    runtime.apply_active_fade(Instant::now() + Duration::from_millis(500));
    assert!(level(&runtime, "Pan").unwrap() > 0);
    assert_same_image(&authored_image(&runtime), &authored);
    settle(&mut runtime);
    runtime.apply_command(EngineCommand::TriggerPreviousCue);
    runtime.apply_command(EngineCommand::TriggerNextCue);
    runtime.apply_command(EngineCommand::TriggerCue(3));
    runtime.apply_command(EngineCommand::ToggleCue(3));
    settle(&mut runtime);
    assert_same_image(&authored_image(&runtime), &authored);
    for cue_id in [1, 2, 3] {
        runtime.apply_command(EngineCommand::ReleaseCue(cue_id));
        settle(&mut runtime);
    }
    // Release still returns to fixture defaults live; the authored fader
    // level stays in the saved image.
    assert_eq!(level(&runtime, "Dimmer"), Some(0));
    assert_same_image(&authored_image(&runtime), &authored);

    // Timeline playback fires the Scene Block and the automation lane.
    runtime.last_tick_interval = Duration::from_millis(10);
    runtime.apply_command(EngineCommand::SetTimelinePlaying(true));
    let started = Instant::now();
    for step in 1..=50u64 {
        let now = started + Duration::from_millis(step * 10);
        runtime.advance_timeline(now);
        runtime.advance_pending_cue(now);
        runtime.apply_timeline_automations();
        runtime.apply_active_fade(now);
    }
    assert_eq!(runtime.active_cue_id, Some(2));
    assert!(level(&runtime, "Pan").unwrap() > 0);
    runtime.apply_command(EngineCommand::SetTimelinePlaying(false));
    assert_same_image(&authored_image(&runtime), &authored);
    assert_eq!(persisted_level(&authored, "Dimmer"), 1_000);
    assert_eq!(authored.active_cue_id, None);
}

#[test]
fn the_live_look_projection_keeps_what_capture_records() {
    let mut runtime = playback_show();
    runtime.apply_command(EngineCommand::TriggerCue(1));
    settle(&mut runtime);
    let live = runtime.build_live_look_persistence_snapshot();
    assert_eq!(persisted_level(&live, "Dimmer"), 40_000);
    assert_eq!(live.active_cue_id, Some(1));
    assert!(live
        .effects
        .iter()
        .any(|effect| effect.id == 10 && effect.enabled));
    assert!(live
        .node_graphs
        .iter()
        .any(|graph| graph.id == 20 && graph.enabled));
    let authored_video = live.authored_video.as_ref().unwrap();
    assert!(authored_video
        .outputs
        .iter()
        .any(|output| output.id == 7 && output.enabled));

    let authored = runtime.build_persistence_snapshot();
    assert_eq!(persisted_level(&authored, "Dimmer"), 1_000);
    assert!(authored
        .effects
        .iter()
        .any(|effect| effect.id == 10 && !effect.enabled));
    assert!(authored
        .node_graphs
        .iter()
        .any(|graph| graph.id == 20 && !graph.enabled));
    let authored_video = authored.authored_video.as_ref().unwrap();
    let output = authored_video
        .outputs
        .iter()
        .find(|output| output.id == 7)
        .unwrap();
    assert!(!output.enabled && !output.blackout && output.opacity == 1.0);
}

#[test]
fn authored_edits_over_live_playback_reach_the_saved_image() {
    let mut runtime = playback_show();
    runtime.apply_command(EngineCommand::TriggerCue(1));
    settle(&mut runtime);

    runtime.apply_command(EngineCommand::SetAttribute {
        fixture_id: 1,
        attribute: "Dimmer".into(),
        value: 20_000,
    });
    assert_eq!(level(&runtime, "Dimmer"), Some(20_000));
    runtime.apply_command(EngineCommand::SetEffectEnabled {
        effect_id: 10,
        enabled: true,
    });
    runtime.apply_command(EngineCommand::SetVideoOutputBlackout {
        output_id: 7,
        blackout: true,
    });
    runtime.apply_command(EngineCommand::SetNodeGraphEnabled {
        graph_id: 20,
        enabled: true,
    });
    let saved = authored_image(&runtime);
    assert_eq!(persisted_level(&saved, "Dimmer"), 20_000);
    assert!(saved
        .effects
        .iter()
        .any(|effect| effect.id == 10 && effect.enabled));
    assert!(saved
        .node_graphs
        .iter()
        .any(|graph| graph.id == 20 && graph.enabled));
    let output = saved
        .video
        .outputs
        .iter()
        .find(|output| output.id == 7)
        .unwrap();
    // Only the blackout was authored; enable and opacity stay Cue-owned.
    assert!(output.blackout && !output.enabled && output.opacity == 1.0);

    // An edit that resends the live graph flag keeps the Cue's override.
    runtime.apply_command(EngineCommand::ReleaseCue(1));
    runtime.apply_command(EngineCommand::SetNodeGraphEnabled {
        graph_id: 20,
        enabled: false,
    });
    runtime.apply_command(EngineCommand::TriggerCue(1));
    settle(&mut runtime);
    let mut resent = runtime
        .node_graphs
        .iter()
        .find(|graph| graph.summary.id == 20)
        .unwrap()
        .summary
        .clone();
    resent.label = "Renamed".to_string();
    runtime.apply_command(EngineCommand::UpsertNodeGraph(resent));
    assert!(graph_enabled(&runtime));
    let saved = authored_image(&runtime);
    let graph = saved
        .node_graphs
        .iter()
        .find(|graph| graph.id == 20)
        .unwrap();
    assert_eq!(graph.label, "Renamed");
    assert!(!graph.enabled);
}

#[test]
fn opening_a_file_saved_with_a_live_cue_starts_with_no_cue_live() {
    let mut source = playback_show();
    source.apply_command(EngineCommand::TriggerCue(1));
    settle(&mut source);
    // A version-1 file saved before DEC-PLAYBACK-LAYER-001 carried the live
    // look and its active references.
    let legacy = source.build_live_look_persistence_snapshot();
    assert!(protocol::has_active_cue_references(&legacy));

    let mut opened = child_graph_test_engine();
    opened.load_project_snapshot_checked(legacy).unwrap();
    assert_eq!(opened.active_cue_id, None);
    assert!(opened
        .cue_lists
        .iter()
        .all(|cue_list| cue_list.active_cue_id.is_none()));
    assert!(opened.active_group_cue_ids.is_empty());
    // The saved levels open as authored state, as the file showed them.
    assert_eq!(level(&opened, "Dimmer"), Some(40_000));
    let saved = authored_image(&opened);
    assert_eq!(persisted_level(&saved, "Dimmer"), 40_000);
    assert_eq!(saved.active_cue_id, None);
}

/// The live part of a show: what renders, independent of authored state.
fn live_state(runtime: &EngineRuntime) -> (PlaybackLayerSnapshot, Option<u16>, Option<u16>) {
    (
        runtime.capture_playback_layer(),
        level(runtime, "Dimmer"),
        level(runtime, "Pan"),
    )
}

fn live_show_after_edit() -> EngineRuntime {
    let mut runtime = playback_show();
    runtime.apply_command(EngineCommand::SetAttribute {
        fixture_id: 1,
        attribute: "Pan".into(),
        value: 9_000,
    });
    runtime.apply_command(EngineCommand::TriggerCue(1));
    settle(&mut runtime);
    runtime.apply_command(EngineCommand::TriggerCue(3));
    settle(&mut runtime);
    runtime
}

#[test]
fn undo_redo_load_keeps_the_current_playback_layer() {
    let before_edit = playback_show().build_persistence_snapshot();
    let mut runtime = live_show_after_edit();
    let mut twin = live_show_after_edit();
    let live = live_state(&runtime);
    assert!(!live.0.is_empty());
    assert_eq!(level(&runtime, "Pan"), Some(30_000));

    // Undo the Pan edit: authored state reverts, the live look stays.
    runtime
        .load_project_snapshot_with_playback(before_edit, PlaybackLoadPolicy::KeepCurrent)
        .unwrap();
    assert_eq!(live_state(&runtime), live);
    assert!(effect_enabled(&runtime) && graph_enabled(&runtime));
    assert!(output_summary(&runtime).enabled);
    let saved = authored_image(&runtime);
    assert_eq!(persisted_level(&saved, "Pan"), 0);
    assert_eq!(persisted_level(&saved, "Dimmer"), 1_000);
    assert!(!saved
        .effects
        .iter()
        .any(|effect| effect.id == 10 && effect.enabled));
    assert_eq!(saved.active_cue_id, None);

    // Playback continues exactly as in a show that never undid anything.
    for show in [&mut runtime, &mut twin] {
        show.apply_command(EngineCommand::ReleaseCue(3));
        settle(show);
    }
    assert_eq!(live_state(&runtime), live_state(&twin));
    for show in [&mut runtime, &mut twin] {
        show.apply_command(EngineCommand::ReleaseCue(1));
        settle(show);
    }
    assert_eq!(live_state(&runtime), live_state(&twin));
}

#[test]
fn undo_that_removes_a_live_cue_releases_it_like_remove_cue() {
    let mut without_group_cue = live_show_after_edit().build_persistence_snapshot();
    without_group_cue.cues.retain(|cue| cue.id != 3);
    let mut runtime = live_show_after_edit();
    let mut removed = live_show_after_edit();
    removed.apply_command(EngineCommand::RemoveCue(3));
    settle(&mut removed);
    assert_eq!(removed.last_error, None);

    runtime
        .load_project_snapshot_with_playback(without_group_cue, PlaybackLoadPolicy::KeepCurrent)
        .unwrap();
    assert!(runtime.active_group_cue_ids.is_empty());
    assert_eq!(runtime.active_cue_id, removed.active_cue_id);
    assert_eq!(level(&runtime, "Pan"), level(&removed, "Pan"));
    assert_eq!(level(&runtime, "Dimmer"), level(&removed, "Dimmer"));
    assert_eq!(level(&runtime, "Dimmer"), Some(40_000));
}

#[test]
fn undo_keeps_a_live_cue_that_the_target_puts_in_another_bank_or_group() {
    let mut runtime = live_show_after_edit();
    let live = live_state(&runtime);
    // Group Cue 3 is live as Bank 1's active Cue and the "front" group's.
    assert_eq!(live.0.active_cue_id, Some(3));
    assert_eq!(live.0.cue_list_active_cue_ids[0].cue_list_id, 1);
    let mut target = runtime.build_persistence_snapshot();
    let mut bank = target.cue_lists[0].clone();
    bank.id = 2;
    bank.label = "Bank 2".to_string();
    target.cue_lists.push(bank);
    let cue = target.cues.iter_mut().find(|cue| cue.id == 3).unwrap();
    cue.cue_list_id = 2;
    cue.group_id = Some("back".to_string());

    runtime
        .load_project_snapshot_with_playback(target, PlaybackLoadPolicy::KeepCurrent)
        .unwrap();
    assert_eq!(runtime.last_error, None);
    assert_eq!(runtime.active_cue_id, Some(3));
    let bank_active = |id| {
        runtime
            .cue_lists
            .iter()
            .find(|cue_list| cue_list.id == id)
            .unwrap()
            .active_cue_id
    };
    assert_eq!((bank_active(1), bank_active(2)), (None, Some(3)));
    assert_eq!(
        runtime.active_group_cue_ids,
        HashMap::from([("back".to_string(), 3)])
    );
    assert_eq!(
        (level(&runtime, "Dimmer"), level(&runtime, "Pan")),
        (live.1, live.2)
    );
    assert!(effect_enabled(&runtime) && graph_enabled(&runtime));
}

#[test]
fn undo_during_a_fade_lands_on_the_fade_targets() {
    let mut runtime = playback_show();
    runtime.apply_command(EngineCommand::TriggerCue(2));
    runtime.apply_active_fade(Instant::now() + Duration::from_millis(500));
    let midway = level(&runtime, "Pan").unwrap();
    assert!(midway > 0 && midway < 50_000, "{midway}");
    let mut twin = playback_show();
    twin.apply_command(EngineCommand::TriggerCue(2));
    settle(&mut twin);

    let target = runtime.build_persistence_snapshot();
    runtime
        .load_project_snapshot_with_playback(target, PlaybackLoadPolicy::KeepCurrent)
        .unwrap();
    assert!(runtime.active_fade.is_none());
    assert_eq!(level(&runtime, "Pan"), Some(50_000));
    assert_eq!(live_state(&runtime), live_state(&twin));
    for show in [&mut runtime, &mut twin] {
        show.apply_command(EngineCommand::ReleaseCue(2));
        settle(show);
    }
    assert_eq!(live_state(&runtime), live_state(&twin));
}

#[test]
fn standby_apply_installs_the_primary_layer_and_rejects_a_mismatch() {
    let mut primary = playback_show();
    primary.apply_command(EngineCommand::TriggerCue(1));
    settle(&mut primary);
    let generation = primary.build_persistence_snapshot();
    let layer = primary.capture_playback_layer();

    let mut standby = child_graph_test_engine();
    standby
        .load_project_snapshot_with_playback(
            generation.clone(),
            PlaybackLoadPolicy::Apply(layer.clone()),
        )
        .unwrap();
    assert_eq!(standby.capture_playback_layer(), layer);
    assert_eq!(standby.active_cue_id, Some(1));
    assert_eq!(level(&standby, "Dimmer"), Some(40_000));
    let canonical = |runtime: &EngineRuntime| {
        let mut image = authored_image(runtime);
        for fixture in &mut image.fixtures {
            fixture.profile_source_path = format!("snapshot://fixture/{}", fixture.id);
        }
        image
    };
    assert_same_image(&canonical(&standby), &canonical(&primary));

    let installed = standby.capture_playback_layer();
    let mut foreign = layer.clone();
    foreign.cue_list_active_cue_ids[0].cue_id = 999;
    foreign.active_cue_id = Some(999);
    let error = standby
        .load_project_snapshot_with_playback(generation.clone(), PlaybackLoadPolicy::Apply(foreign))
        .unwrap_err();
    assert!(
        error.contains("Playback layer does not match the project"),
        "{error}"
    );
    // Rejected unchanged.
    assert_eq!(standby.capture_playback_layer(), installed);

    let mut unknown_level = layer;
    unknown_level.levels.push(protocol::PlaybackLevel {
        fixture_id: 77,
        attribute: "Dimmer".to_string(),
        value: 1,
        cue_list_id: None,
    });
    assert!(standby
        .load_project_snapshot_with_playback(generation, PlaybackLoadPolicy::Apply(unknown_level))
        .unwrap_err()
        .contains("fixture 77"));
    assert_eq!(standby.capture_playback_layer(), installed);
}

#[test]
fn playback_commands_are_classified_as_never_moving_the_authored_image() {
    // The classification the external-input seqlock relies on is now true.
    for command in [
        EngineCommand::TriggerCue(1),
        EngineCommand::ReleaseCue(1),
        EngineCommand::ToggleCue(1),
        EngineCommand::TriggerNextCue,
        EngineCommand::SetTimelinePlaying(true),
    ] {
        assert!(!command.mutates_persistence_snapshot());
    }
}

#[test]
fn a_committed_edit_of_a_live_scene_stays_playback_and_a_plain_commit_is_authored() {
    let mut runtime = playback_show();
    runtime.apply_command(EngineCommand::TriggerCue(1));
    settle(&mut runtime);
    let authored = authored_image(&runtime);

    // Blind edit of the live scene, committed into it.
    runtime.apply_command(EngineCommand::SetProgrammerMode {
        enabled: true,
        blind: true,
        edited_scene_cue_id: Some(1),
    });
    runtime.apply_command(EngineCommand::SetProgrammerAttribute {
        fixture_id: 1,
        attribute: "Dimmer".into(),
        value: 50_000,
    });
    runtime.apply_command(EngineCommand::CommitProgrammer {
        edited_scene_cue_id: Some(1),
    });
    assert_eq!(level(&runtime, "Dimmer"), Some(50_000));
    assert_same_image(&authored_image(&runtime), &authored);
    runtime.apply_command(EngineCommand::ReleaseCue(1));
    settle(&mut runtime);
    assert_eq!(level(&runtime, "Dimmer"), Some(0));
    assert_same_image(&authored_image(&runtime), &authored);

    // A plain programmer commit is an authored edit, like a fader.
    runtime.apply_command(EngineCommand::SetProgrammerMode {
        enabled: true,
        blind: true,
        edited_scene_cue_id: None,
    });
    runtime.apply_command(EngineCommand::SetProgrammerAttribute {
        fixture_id: 1,
        attribute: "Pan".into(),
        value: 12_000,
    });
    runtime.apply_command(EngineCommand::CommitProgrammer {
        edited_scene_cue_id: None,
    });
    assert_eq!(level(&runtime, "Pan"), Some(12_000));
    assert_eq!(persisted_level(&authored_image(&runtime), "Pan"), 12_000);
}
