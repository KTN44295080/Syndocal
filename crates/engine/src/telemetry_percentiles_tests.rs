use super::{PercentileWindow, CAPACITY};

// Reference implementation from the previous engine helper, retained only as an oracle.
fn old_percentile(window: &[u64; CAPACITY], len: usize, percentile: f64) -> u64 {
    if len == 0 {
        return 0;
    }
    let mut samples = *window;
    samples[..len].sort_unstable();
    let clamped_percentile = percentile.clamp(0.0, 1.0);
    let index = ((len as f64 * clamped_percentile).ceil() as usize).saturating_sub(1);
    samples[index.min(len - 1)]
}

#[test]
fn empty_and_single_sample() {
    let mut window = PercentileWindow::default();
    assert_eq!(window.p95_p99(), (0, 0));
    window.record(42);
    assert_eq!(window.p95_p99(), (42, 42));
}

#[test]
fn nearest_rank_boundaries_and_full_window() {
    let mut window = PercentileWindow::default();
    for value in 1..=CAPACITY as u64 {
        window.record(value);
        match value {
            20 => assert_eq!(window.p95_p99(), (19, 20)),
            100 => assert_eq!(window.p95_p99(), (95, 99)),
            256 => assert_eq!(window.p95_p99(), (244, 254)),
            _ => {}
        }
    }
}

#[test]
fn wrapping_keeps_only_latest_samples_and_reads_do_not_mutate() {
    let mut window = PercentileWindow::default();
    for _ in 0..CAPACITY {
        window.record(u64::MAX);
    }
    for value in 1..=CAPACITY as u64 {
        window.record(value);
    }
    assert_eq!(window.p95_p99(), (244, 254));
    assert_eq!(window.p95_p99(), (244, 254));
    for value in 257..=512 {
        window.record(value);
    }
    assert_eq!(window.p95_p99(), (500, 510));
}

#[test]
fn reset_discards_full_window_and_restarts_recording() {
    let mut window = PercentileWindow::default();
    for value in 1..=300 {
        window.record(value);
    }
    window.reset();
    assert_eq!(window.p95_p99(), (0, 0));
    window.record(7);
    assert_eq!(window.p95_p99(), (7, 7));
}

#[test]
fn duplicates_and_extremes_are_preserved() {
    let mut window = PercentileWindow::default();
    for _ in 0..99 {
        window.record(0);
    }
    window.record(u64::MAX);
    assert_eq!(window.p95_p99(), (0, 0));
    window.record(u64::MAX);
    assert_eq!(window.p95_p99(), (0, u64::MAX));
    window.reset();
    for _ in 0..CAPACITY {
        window.record(u64::MAX);
    }
    assert_eq!(window.p95_p99(), (u64::MAX, u64::MAX));
}

#[test]
fn matches_previous_oracle_at_every_length_and_after_multiple_wraps() {
    let mut window = PercentileWindow::default();
    let mut old_window = [0; CAPACITY];
    let mut old_len = 0;
    let mut old_index = 0;
    let mut random = 0x1234_5678_9abc_def0_u64;
    for iteration in 0..CAPACITY * 5 {
        assert_eq!(
            window.p95_p99(),
            (
                old_percentile(&old_window, old_len, 0.95),
                old_percentile(&old_window, old_len, 0.99),
            ),
            "iteration {iteration}"
        );
        random ^= random << 13;
        random ^= random >> 7;
        random ^= random << 17;
        let sample = if iteration % 3 == 0 { 17 } else { random };
        window.record(sample);
        old_window[old_index] = sample;
        old_index = (old_index + 1) % CAPACITY;
        old_len = (old_len + 1).min(CAPACITY);
    }
}

#[test]
#[ignore = "synthetic release benchmark; does not measure whole-show CPU or lock wait"]
fn benchmark_three_snapshot_percentile_windows() {
    use std::{hint::black_box, time::Instant};

    assert!(!cfg!(debug_assertions), "run this benchmark with --release");
    const ITERATIONS: usize = 20_000;
    let windows: [PercentileWindow; 3] = std::array::from_fn(|lane| {
        let mut window = PercentileWindow::default();
        for index in 0..CAPACITY {
            window.record(((index * 173 + lane * 31) % 257) as u64);
        }
        window
    });
    let start = Instant::now();
    for _ in 0..ITERATIONS {
        for window in black_box(&windows) {
            black_box((
                old_percentile(black_box(&window.samples), black_box(window.len), 0.95),
                old_percentile(black_box(&window.samples), black_box(window.len), 0.99),
            ));
        }
    }
    let old = start.elapsed();
    let start = Instant::now();
    for _ in 0..ITERATIONS {
        for window in black_box(&windows) {
            black_box(black_box(window).p95_p99());
        }
    }
    let new = start.elapsed();
    eprintln!(
        "snapshot percentile benchmark: iterations={ITERATIONS} windows=3 capacity={CAPACITY} old_sorts=6 new_sorts=3 old={old:?} new={new:?}"
    );
}
