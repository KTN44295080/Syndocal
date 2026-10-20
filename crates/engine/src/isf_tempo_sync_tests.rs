use super::*;
use protocol::{VideoIsfTempoSync, VideoIsfTempoSyncUnit};

fn runtime_with_synced_strobe() -> (EngineRuntime, VideoIsfEffectSummary) {
    let mut runtime = EngineRuntime::new(DmxOutputConfig {
        enabled: false,
        ..DmxOutputConfig::default()
    });
    add_runtime_test_video_layer(&mut runtime, 1, VideoLayerState::default());
    let mut effect = video::builtin_isf_effect("strobe").unwrap().unwrap();
    let rate = effect
        .controls
        .iter_mut()
        .find(|control| control.name == "rate")
        .unwrap();
    rate.value[0] = 8.0;
    rate.tempo_sync = Some(VideoIsfTempoSync {
        unit: VideoIsfTempoSyncUnit::Beats,
        count: 1.0,
    });
    runtime.apply_command(EngineCommand::SetVideoLayerIsfEffect {
        layer_id: 1,
        effect: Some(effect.clone()),
    });
    assert!(
        runtime.last_error.is_none(),
        "the synced effect must be accepted"
    );
    runtime.apply_command(EngineCommand::SetBpm(126.0));
    assert!(runtime.last_error.is_none());
    (runtime, effect)
}

fn assert_effect_rate(effect: &VideoIsfEffectSummary, clock: &ClockSnapshot, expected: f32) {
    let shader = video::prepare_isf_shader(&effect.source).unwrap();
    let index = shader
        .controls
        .iter()
        .position(|control| control.name == "rate")
        .unwrap();
    let values = video::resolved_isf_control_values(&shader, effect, Some(clock.bpm));
    assert!(
        (values[index][0] - expected).abs() < 0.00001,
        "effective rate {} Hz at {} BPM, expected {expected} Hz",
        values[index][0],
        clock.bpm
    );
}

#[test]
fn video_isf_tempo_sync_follows_external_clock_without_project_writes() {
    let (mut runtime, effect) = runtime_with_synced_strobe();
    let authored = runtime.build_persistence_snapshot();
    assert_effect_rate(&effect, &runtime.clock.snapshot(Instant::now()), 2.1);

    for (source, bpm, expected) in [
        (ClockSource::AbletonLink, 90.0, 1.5),
        (ClockSource::DjLink, 180.0, 3.0),
    ] {
        runtime.apply_command(EngineCommand::SyncExternalClock {
            bpm,
            beat_phase: 0.5,
            source: source.clone(),
        });
        assert!(runtime.last_error.is_none());
        let clock = runtime.clock.snapshot(Instant::now());
        assert_eq!(clock.source, source);
        assert!(clock.external_sync_locked);
        assert_effect_rate(&effect, &clock, expected);
        assert_eq!(runtime.build_persistence_snapshot(), authored);
    }

    // MTC/LTC expose timecode lock, not a musical tempo inferred from frame rate.
    for source in [ClockSource::MidiTimecode, ClockSource::Ltc] {
        let now = Instant::now();
        runtime.clock.mark_timecode_sync(source.clone(), now);
        let clock = runtime.clock.snapshot(now);
        assert_eq!(clock.source, source);
        assert!(clock.external_sync_locked);
        assert_eq!(clock.bpm, 180.0);
        assert_effect_rate(&effect, &clock, 3.0);
        assert_eq!(runtime.build_persistence_snapshot(), authored);
    }

    // Explicit manual edits still save the chosen project tempo; FX values stay authored.
    for (bpm, expected) in [(60.0, 1.0), (120.0, 2.0), (180.0, 3.0)] {
        runtime.apply_command(EngineCommand::SetBpm(bpm));
        assert_effect_rate(&effect, &runtime.clock.snapshot(Instant::now()), expected);
        let mut saved = runtime.build_persistence_snapshot();
        assert_eq!(saved.clock.bpm, bpm);
        saved.clock = authored.clock.clone();
        assert_eq!(saved, authored);
    }
}

#[test]
fn video_isf_tempo_sync_keeps_nominal_rate_when_external_clock_stops() {
    let (mut runtime, effect) = runtime_with_synced_strobe();
    let authored = runtime.build_persistence_snapshot();
    let start = Instant::now();
    for pulse in 0..32_u64 {
        runtime.apply_midi_clock_pulse_at(start + Duration::from_millis(pulse * 25));
    }
    let last_pulse = start + Duration::from_millis(31 * 25);
    let running = runtime.clock.snapshot(last_pulse);
    assert_eq!(running.source, ClockSource::MidiClock);
    assert!(running.external_sync_locked);
    assert!((running.bpm - 100.0).abs() < 0.001);
    assert_effect_rate(&effect, &running, 100.0 / 60.0);

    // No further MIDI pulses: lock expires, but the engine retains its nominal BPM.
    let stopped = runtime.clock.snapshot(last_pulse + Duration::from_secs(2));
    assert!(!stopped.external_sync_locked);
    assert_eq!(stopped.external_sync_age_ms, Some(2_000));
    assert_eq!(stopped.bpm, running.bpm);
    assert_effect_rate(&effect, &stopped, 100.0 / 60.0);
    assert_eq!(runtime.build_persistence_snapshot(), authored);

    // The next valid clock observation resumes following without touching the FX state.
    runtime.apply_command(EngineCommand::SyncExternalClock {
        bpm: 150.0,
        beat_phase: 0.25,
        source: ClockSource::AbletonLink,
    });
    assert_effect_rate(&effect, &runtime.clock.snapshot(Instant::now()), 2.5);
    assert_eq!(runtime.build_persistence_snapshot(), authored);
}
