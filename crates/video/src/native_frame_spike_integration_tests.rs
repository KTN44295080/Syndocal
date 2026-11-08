use super::*;
use std::time::Duration;

struct SlowMode;
impl SlowMode {
    fn enable() -> Self {
        enable_for_tests();
        PROCESS_ENABLED.store(false, Ordering::Relaxed);
        SPIKE_ENABLED.store(true, Ordering::Relaxed);
        Self
    }
}
impl Drop for SlowMode {
    fn drop(&mut self) {
        let _ = stop_capture();
        retire_thread();
        let _ = acknowledge_stop();
        SPIKE_ENABLED.store(false, Ordering::Relaxed);
        PROCESS_ENABLED.store(true, Ordering::Relaxed);
    }
}

fn finish(frame: &mut SpikeFrame, elapsed: u64, admitted: bool) {
    frame.mark(SpikeStage::Extent);
    frame.mark(SpikeStage::Permit);
    frame.mark(SpikeStage::RetirementConfigure);
    frame.mark(SpikeStage::Prepare);
    frame.mark(SpikeStage::Present);
    frame.set_pacing(Duration::from_micros(100));
    frame.finish(SpikeMetricSample {
        sample_us: frame.sample_clock(Instant::now()),
        elapsed: Duration::from_micros(elapsed),
        metrics_lock_wait: Duration::from_micros(3),
        admitted,
        validation_epoch: 2,
        last_presented_request_sequence: 42,
        frames_presented: 40,
        work_sample_sequence: 39,
    });
}

#[test]
fn slow_work_handoff_preserves_samples_and_shared_stop_guards() {
    let _serial = TIMING_TEST_SERIAL.lock().unwrap();
    let _mode = SlowMode::enable();
    start().unwrap();
    prepare_thread().unwrap();
    let mut frame = begin_spike_frame(1, Instant::now());
    finish(&mut frame, 33_334, true);
    assert!(spike_snapshot().is_err());
    retire_thread(); // Cold first-frame -> render-worker handoff, not final retirement.
    prepare_thread().unwrap();
    let mut frame = begin_spike_frame(1, Instant::now());
    finish(&mut frame, 33_333, true);
    let mut frame = begin_spike_frame(1, Instant::now());
    finish(&mut frame, 99_999, false);
    stop_capture().unwrap();
    assert!(acknowledge_stop().is_err());
    retire_thread();
    assert!(spike_snapshot().is_err());
    acknowledge_stop().unwrap();
    let snapshot = spike_snapshot().unwrap();
    assert!(snapshot.samples.retired);
    assert_eq!(snapshot.samples.counters.attempted_iterations, 3);
    assert_eq!(snapshot.samples.counters.matched_count, 1);
    assert_eq!(snapshot.samples.counters.fast_count, 1);
    assert_eq!(snapshot.samples.counters.excluded_count, 1);
    assert_eq!(snapshot.samples.records.len(), 1);
    let sample = snapshot.samples.records[0];
    assert_eq!(
        (
            sample.iteration,
            sample.work_us,
            sample.last_presented_request_sequence
        ),
        (1, 33_334, 42)
    );
    assert_eq!((sample.pacing_us, sample.metrics_lock_wait_us), (100, 3));
    assert!(super::snapshot().is_err());
}

#[test]
fn slow_work_exact_boundary_clock_and_pacing_are_not_resampled_on_finish() {
    let _serial = TIMING_TEST_SERIAL.lock().unwrap();
    let _mode = SlowMode::enable();
    start().unwrap();
    prepare_thread().unwrap();
    let started = Instant::now();
    let mut frame = begin_spike_frame(1, started);
    frame.mark(SpikeStage::Extent);
    frame.mark(SpikeStage::Permit);
    frame.mark(SpikeStage::RetirementConfigure);
    frame.mark(SpikeStage::Prepare);
    std::thread::sleep(Duration::from_millis(50));
    frame.mark(SpikeStage::Present);
    let pacing = Duration::from_millis(10);
    frame.set_pacing(pacing);
    let sample_instant = Instant::now();
    let elapsed = sample_instant.saturating_duration_since(started + pacing);
    frame.finish(SpikeMetricSample {
        sample_us: frame.sample_clock(sample_instant),
        elapsed,
        metrics_lock_wait: Duration::ZERO,
        admitted: true,
        validation_epoch: 2,
        last_presented_request_sequence: 41,
        frames_presented: 40,
        work_sample_sequence: 39,
    });
    stop_capture().unwrap();
    retire_thread();
    acknowledge_stop().unwrap();
    let record = spike_snapshot().unwrap().samples.records[0];
    assert_eq!(record.work_us, elapsed.as_micros() as u64);
    assert_eq!(record.work_sample_sequence, 39);
    assert_eq!(record.last_presented_request_sequence, 41);
    assert!(record.present_with_pacing_us >= record.pacing_us);
    let wall_us = record.sample_us - record.start_us;
    assert!(wall_us.abs_diff(record.work_us + record.pacing_us) <= 1);
}

#[test]
fn slow_work_admitted_frame_finishes_after_stop_and_partial_is_explicit() {
    let _serial = TIMING_TEST_SERIAL.lock().unwrap();
    let _mode = SlowMode::enable();
    start().unwrap();
    prepare_thread().unwrap();
    let mut frame = begin_spike_frame(1, Instant::now());
    stop_capture().unwrap();
    finish(&mut frame, 35_604, true);
    assert!(begin_spike_frame(1, Instant::now()).lock_clock().is_none());
    retire_thread();
    acknowledge_stop().unwrap();
    assert_eq!(spike_snapshot().unwrap().samples.records[0].work_us, 35_604);
    start().unwrap();
    prepare_thread().unwrap();
    let frame = begin_spike_frame(1, Instant::now());
    drop(frame);
    stop_capture().unwrap();
    retire_thread();
    acknowledge_stop().unwrap();
    let snapshot = spike_snapshot().unwrap();
    assert_eq!(snapshot.samples.counters.partial_count, 1);
    assert_eq!(snapshot.samples.counters.unobserved_count, 1);
    assert!(snapshot.samples.records[0].partial);
    assert!(!snapshot.samples.records[0].work_observed);
}

#[test]
fn slow_work_disabled_startup_never_touches_tls_or_clock() {
    const CHILD: &str = "SYNDOCAL_QA_DISABLED_SPIKE_TEST_CHILD";
    if std::env::var_os(CHILD).is_none() {
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "native_frame_timing::spike_tests::slow_work_disabled_startup_never_touches_tls_or_clock", "--test-threads=1"])
            .env(CHILD, "1").env_remove(OUTPUT_ENV).env_remove(SPIKE_OUTPUT_ENV)
            .status().unwrap();
        assert!(status.success());
        return;
    }
    configure_from_environment().unwrap();
    let mut frame = begin_spike_frame(1, Instant::now());
    finish(&mut frame, 90_000, true);
    assert!(frame.lock_clock().is_none());
    assert_eq!(frame.sample_clock(Instant::now()), 0);
    drop(frame);
    assert_eq!(SPIKE_TLS_INITIALIZATIONS.load(Ordering::Relaxed), 0);
    assert!(CLOCK.get().is_none());
}

#[test]
fn slow_work_configuration_and_export_require_exclusivity_and_ack() {
    let _serial = TIMING_TEST_SERIAL.lock().unwrap();
    let _mode = SlowMode::enable();
    let directory = std::env::temp_dir().join(format!(
        "syndocal-spike-test-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&directory).unwrap();
    let output = directory.join("spikes.json");
    let configured = automatic_from_values(None, Some(output.clone().into_os_string()))
        .unwrap()
        .unwrap();
    assert!(configured.kind == CaptureKind::SlowWork);
    assert!(automatic_from_values(
        Some(output.clone().into_os_string()),
        Some(output.clone().into_os_string())
    )
    .is_err());
    configured.start_once().unwrap();
    prepare_thread().unwrap();
    let mut frame = begin_spike_frame(1, Instant::now());
    finish(&mut frame, 35_604, true);
    stop_capture().unwrap();
    assert!(configured.export_once().is_err());
    retire_thread();
    acknowledge_stop().unwrap();
    assert!(configured.export_once().unwrap());
    let value: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&output).unwrap()).unwrap();
    assert_eq!(value["schema"], "native-work-spikes-1");
    assert_eq!(value["samples"]["records"][0]["work_us"], 35_604);
    assert!(!configured.export_once().unwrap());
    assert!(AutomaticCapture::new(output.clone()).is_err());
    std::fs::remove_file(output).unwrap();
    std::fs::remove_dir(directory).unwrap();
}
