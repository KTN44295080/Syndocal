//! Successful native presentation / SDK send cadence, not receiver or scanout proof.
//! Writers retain one fixed ring and never lock, allocate, or perform I/O per frame.
use serde::Serialize;
use std::{
    collections::BTreeMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex, OnceLock, Weak,
    },
    time::{Duration, Instant},
};

const CAPACITY: usize = 600;
const TARGET_FPS: u64 = 60;
const INTERVAL_NS: u64 = 1_000_000_000 / TARGET_FPS;
/// Whole-run present-interval histogram (ADR-01 section 13.3 (2)): bin `k`
/// holds intervals in (k * width, (k + 1) * width]; longer intervals count in
/// the overflow. Cumulative since registration and never reset, so a reader
/// differences two snapshots of the same writer generation to obtain the exact
/// distribution of every interval that ended between them.
const INTERVAL_BIN_NS: u64 = 100_000;
const INTERVAL_BINS: usize = 1_000;
static WRITER_GENERATION: AtomicU64 = AtomicU64::new(0);
// Zero is a valid preparation duration. Unmeasured startup / cached sends
// must not become fabricated zero-cost preparation samples.
const NO_RENDER_TIME_NS: u64 = u64::MAX;
type Registry = BTreeMap<(u64, &'static str), Weak<Samples>>;
static REGISTRY: OnceLock<Mutex<Registry>> = OnceLock::new();

struct Samples {
    origin: Instant,
    generation: u64,
    timestamps_ns: [AtomicU64; CAPACITY],
    render_times_ns: [AtomicU64; CAPACITY],
    interval_bins: [AtomicU64; INTERVAL_BINS],
    interval_overflow: AtomicU64,
    interval_total_ns: AtomicU64,
    interval_max_ns: AtomicU64,
    count: AtomicU64,
    revision: AtomicU64,
    dropped: AtomicU64,
}

pub(crate) struct OutputHealthWriter {
    samples: Arc<Samples>,
    first_ns: Option<u64>,
    last_ns: Option<u64>,
    count: u64,
    /// Deliveries before the current cadence anchor; they are not slot debt.
    anchor_count: u64,
    dropped: u64,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LiveVideoOutputHealth {
    output_id: u64,
    kind: &'static str,
    delivered_fps: f64,
    frame_time_p50_ms: f64,
    frame_time_p95_ms: f64,
    frame_time_p99_ms: f64,
    /// Successful-delivery preparation wall time, excluding pacing and send.
    /// Display GPU composition/presentation is outside this boundary.
    render_time_p50_ms: f64,
    render_time_p95_ms: f64,
    render_time_p99_ms: f64,
    render_sample_count: usize,
    render_measurement_boundary: &'static str,
    /// High-water deficit of elapsed 60 Hz slots versus completed submissions.
    /// Never decreases after a catch-up burst; not receiver-confirmed loss.
    dropped_frames: u64,
    target_fps: u64,
    delivered_frames: u64,
    sample_count: usize,
    measurement_boundary: &'static str,
    last_frame_age_ms: Option<f64>,
    /// Writer-clock time of this snapshot and of the newest delivery, so a
    /// window boundary can be anchored to deliveries instead of IPC timing.
    snapshot_elapsed_ns: u64,
    last_delivery_elapsed_ns: Option<u64>,
    present_intervals: PresentIntervalHistogram,
}

/// Cumulative, consistent with `delivered_frames` (same seqlock read):
/// `interval_count == delivered_frames - 1` once a delivery exists.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PresentIntervalHistogram {
    writer_generation: u64,
    bin_width_ns: u64,
    bin_count: usize,
    /// Sparse `[bin index, cumulative count]` pairs, ascending, nonzero only.
    bins: Vec<[u64; 2]>,
    overflow_count: u64,
    interval_count: u64,
    interval_total_ns: u64,
    /// Lifetime maximum; informational (a maximum cannot be differenced).
    interval_max_ns: u64,
}

impl OutputHealthWriter {
    pub(crate) fn register(output_id: u64, kind: &'static str) -> Self {
        let samples = Arc::new(Samples {
            origin: Instant::now(),
            generation: WRITER_GENERATION.fetch_add(1, Ordering::Relaxed) + 1,
            timestamps_ns: std::array::from_fn(|_| AtomicU64::new(0)),
            render_times_ns: std::array::from_fn(|_| AtomicU64::new(NO_RENDER_TIME_NS)),
            interval_bins: std::array::from_fn(|_| AtomicU64::new(0)),
            interval_overflow: AtomicU64::new(0),
            interval_total_ns: AtomicU64::new(0),
            interval_max_ns: AtomicU64::new(0),
            count: AtomicU64::new(0),
            revision: AtomicU64::new(0),
            dropped: AtomicU64::new(0),
        });
        if let Ok(mut registry) = REGISTRY.get_or_init(Default::default).lock() {
            registry.retain(|_, samples| samples.strong_count() > 0);
            registry.insert((output_id, kind), Arc::downgrade(&samples));
        }
        Self {
            samples,
            first_ns: None,
            last_ns: None,
            count: 0,
            anchor_count: 0,
            dropped: 0,
        }
    }

    /// Re-anchor the missed-slot baseline where steady-state pacing begins.
    /// Startup work between the first present and the render loop is not
    /// a missed output interval; deliveries and stalls after this call are.
    pub(crate) fn restart_cadence(&mut self) {
        self.first_ns = None;
        self.anchor_count = self.count;
    }

    pub(crate) fn delivered(&mut self) {
        self.record_at(self.samples.origin.elapsed(), None);
    }

    /// Attach measured preparation only after the physical boundary succeeds.
    /// The caller stops this timer before present / SDK send / pacing sleep.
    pub(crate) fn delivered_with_render_time(&mut self, render_time: Duration) {
        self.record_at(self.samples.origin.elapsed(), Some(render_time));
    }

    fn record_at(&mut self, elapsed: Duration, render_time: Option<Duration>) {
        let now = elapsed.as_nanos().min(u128::from(u64::MAX)) as u64;
        let first = *self.first_ns.get_or_insert(now);
        let expected_intervals = now.saturating_sub(first) / INTERVAL_NS;
        self.dropped = self
            .dropped
            .max(expected_intervals.saturating_sub(self.count - self.anchor_count));
        self.samples.revision.fetch_add(1, Ordering::AcqRel);
        // Seqlock writer: the odd revision must be visible before ring data.
        std::sync::atomic::fence(Ordering::Release);
        self.samples.timestamps_ns[self.count as usize % CAPACITY].store(now, Ordering::Relaxed);
        self.samples.render_times_ns[self.count as usize % CAPACITY].store(
            render_time
                .map(|time| time.as_nanos().min(u128::from(NO_RENDER_TIME_NS - 1)) as u64)
                .unwrap_or(NO_RENDER_TIME_NS),
            Ordering::Relaxed,
        );
        if let Some(last) = self.last_ns.replace(now) {
            // Single writer: plain load/store keeps this free of locked RMW.
            let interval = now.saturating_sub(last);
            let bin = interval.saturating_sub(1) / INTERVAL_BIN_NS;
            let slot = usize::try_from(bin)
                .ok()
                .and_then(|bin| self.samples.interval_bins.get(bin))
                .unwrap_or(&self.samples.interval_overflow);
            slot.store(slot.load(Ordering::Relaxed) + 1, Ordering::Relaxed);
            let total = &self.samples.interval_total_ns;
            total.store(
                total.load(Ordering::Relaxed).saturating_add(interval),
                Ordering::Relaxed,
            );
            let max = &self.samples.interval_max_ns;
            if interval > max.load(Ordering::Relaxed) {
                max.store(interval, Ordering::Relaxed);
            }
        }
        self.count = self.count.saturating_add(1);
        self.samples.dropped.store(self.dropped, Ordering::Relaxed);
        self.samples.count.store(self.count, Ordering::Release);
        self.samples.revision.fetch_add(1, Ordering::Release);
    }
}

pub(crate) fn snapshots() -> Vec<LiveVideoOutputHealth> {
    // Registry locks are taken by UI queries and worker startup only, never a frame writer.
    let entries = REGISTRY
        .get_or_init(Default::default)
        .lock()
        .map(|registry| {
            registry
                .iter()
                .filter_map(|(&(id, kind), weak)| weak.upgrade().map(|samples| (id, kind, samples)))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    entries
        .into_iter()
        .filter_map(|(id, kind, samples)| snapshot(id, kind, &samples))
        .collect()
}

fn snapshot(
    output_id: u64,
    kind: &'static str,
    samples: &Samples,
) -> Option<LiveVideoOutputHealth> {
    snapshot_at(output_id, kind, samples, samples.origin.elapsed())
}

fn snapshot_at(
    output_id: u64,
    kind: &'static str,
    samples: &Samples,
    elapsed: Duration,
) -> Option<LiveVideoOutputHealth> {
    for _ in 0..32 {
        let revision = samples.revision.load(Ordering::Acquire);
        if revision % 2 != 0 {
            std::hint::spin_loop();
            continue;
        }
        let count = samples.count.load(Ordering::Acquire);
        let retained = count.min(CAPACITY as u64);
        let timestamps = ((count - retained)..count)
            .map(|index| samples.timestamps_ns[index as usize % CAPACITY].load(Ordering::Relaxed))
            .collect::<Vec<_>>();
        let mut render_times = ((count - retained)..count)
            .map(|index| samples.render_times_ns[index as usize % CAPACITY].load(Ordering::Relaxed))
            .filter(|time| *time != NO_RENDER_TIME_NS)
            .collect::<Vec<_>>();
        let dropped = samples.dropped.load(Ordering::Relaxed);
        let bins = samples
            .interval_bins
            .iter()
            .enumerate()
            .filter_map(|(index, bin)| {
                let count = bin.load(Ordering::Relaxed);
                (count != 0).then_some([index as u64, count])
            })
            .collect::<Vec<_>>();
        let overflow_count = samples.interval_overflow.load(Ordering::Relaxed);
        let interval_total_ns = samples.interval_total_ns.load(Ordering::Relaxed);
        let interval_max_ns = samples.interval_max_ns.load(Ordering::Relaxed);
        // Keep every ring load before the final sequence check, including on
        // weakly ordered targets. All concurrent ring accesses remain atomic.
        std::sync::atomic::fence(Ordering::Acquire);
        if samples.revision.load(Ordering::Acquire) != revision {
            continue;
        }
        let mut intervals = timestamps
            .windows(2)
            .map(|pair| pair[1].saturating_sub(pair[0]))
            .collect::<Vec<_>>();
        let snapshot_elapsed_ns = elapsed.as_nanos().min(u128::from(u64::MAX)) as u64;
        let age_ns = timestamps
            .last()
            .map(|last| snapshot_elapsed_ns.saturating_sub(*last));
        // Include an in-progress stall; a stopped producer must not keep showing its old fps.
        let span_ns = timestamps
            .first()
            .zip(timestamps.last())
            .map(|(first, last)| {
                last.saturating_sub(*first)
                    .saturating_add(age_ns.unwrap_or(0))
            })
            .unwrap_or(0);
        let delivered_fps = if span_ns > 0 {
            intervals.len() as f64 * 1e9 / span_ns as f64
        } else {
            0.0
        };
        intervals.sort_unstable();
        render_times.sort_unstable();
        let percentile = |values: &[u64], p: usize| {
            if values.is_empty() {
                0.0
            } else {
                values[(values.len() * p).div_ceil(100).saturating_sub(1)] as f64 / 1e6
            }
        };
        return Some(LiveVideoOutputHealth {
            output_id,
            kind,
            delivered_fps,
            frame_time_p50_ms: percentile(&intervals, 50),
            frame_time_p95_ms: percentile(&intervals, 95),
            frame_time_p99_ms: percentile(&intervals, 99),
            render_time_p50_ms: percentile(&render_times, 50),
            render_time_p95_ms: percentile(&render_times, 95),
            render_time_p99_ms: percentile(&render_times, 99),
            render_sample_count: render_times.len(),
            render_measurement_boundary: if kind == "display" {
                "prepare-wall-excludes-gpu-composition-and-present"
            } else {
                "prepare-wall-includes-readback-excludes-sdk-send"
            },
            dropped_frames: dropped,
            target_fps: TARGET_FPS,
            delivered_frames: count,
            sample_count: intervals.len(),
            measurement_boundary: if kind == "display" {
                "present"
            } else {
                "sender"
            },
            last_frame_age_ms: age_ns.map(|age| age as f64 / 1e6),
            snapshot_elapsed_ns,
            last_delivery_elapsed_ns: timestamps.last().copied(),
            present_intervals: PresentIntervalHistogram {
                writer_generation: samples.generation,
                bin_width_ns: INTERVAL_BIN_NS,
                bin_count: INTERVAL_BINS,
                bins,
                overflow_count,
                interval_count: count.saturating_sub(1),
                interval_total_ns,
                interval_max_ns,
            },
        });
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cadence_uses_successes_and_counts_missed_slots() {
        let mut writer = OutputHealthWriter::register(u64::MAX, "test");
        for index in 0..=60 {
            writer.record_at(Duration::from_nanos(index * INTERVAL_NS), None);
        }
        let health = snapshot(u64::MAX, "test", &writer.samples).unwrap();
        assert_eq!(health.delivered_frames, 61);
        assert_eq!(health.sample_count, 60);
        assert_eq!(health.dropped_frames, 0);
        assert!((health.frame_time_p95_ms - 16.666666).abs() < 0.001);
        writer.record_at(Duration::from_nanos(63 * INTERVAL_NS), None);
        assert_eq!(
            snapshot(u64::MAX, "test", &writer.samples)
                .unwrap()
                .dropped_frames,
            2
        );
    }

    #[test]
    fn cadence_deficit_never_disappears_after_catch_up() {
        let mut writer = OutputHealthWriter::register(u64::MAX - 2, "test");
        writer.record_at(Duration::ZERO, None);
        writer.record_at(Duration::from_nanos(4 * INTERVAL_NS), None);
        assert_eq!(writer.dropped, 3);
        // Successful submissions in a burst cannot erase the observed stall.
        for _ in 0..8 {
            writer.record_at(Duration::from_nanos(4 * INTERVAL_NS), None);
        }
        assert_eq!(writer.dropped, 3);
    }

    #[test]
    fn restarting_cadence_forgets_startup_gap_but_keeps_later_stalls() {
        let mut writer = OutputHealthWriter::register(u64::MAX - 3, "test");
        writer.record_at(Duration::ZERO, None);
        // The render worker starts hundreds of milliseconds after the first present.
        writer.restart_cadence();
        writer.record_at(Duration::from_nanos(30 * INTERVAL_NS), None);
        writer.record_at(Duration::from_nanos(31 * INTERVAL_NS), None);
        assert_eq!(writer.dropped, 0);
        writer.record_at(Duration::from_nanos(35 * INTERVAL_NS), None);
        assert_eq!(writer.dropped, 3);
    }

    #[test]
    fn bounded_ring_preserves_recent_cadence_and_retired_worker_disappears() {
        let mut writer = OutputHealthWriter::register(u64::MAX - 1, "test");
        for index in 0..800 {
            writer.record_at(Duration::from_nanos(index * INTERVAL_NS), None);
        }
        let health = snapshot(u64::MAX - 1, "test", &writer.samples).unwrap();
        assert_eq!(health.sample_count, CAPACITY - 1);
        assert_eq!(health.delivered_frames, 800);
        drop(writer);
        assert!(!snapshots()
            .iter()
            .any(|sample| sample.output_id == u64::MAX - 1));
    }

    #[test]
    fn unmeasured_startup_is_distinct_from_a_measured_zero_duration() {
        let mut writer = OutputHealthWriter::register(u64::MAX - 4, "display");
        writer.record_at(Duration::ZERO, None);
        let startup = snapshot_at(0, "display", &writer.samples, Duration::ZERO).unwrap();
        assert_eq!(startup.delivered_frames, 1);
        assert_eq!(startup.render_sample_count, 0);
        writer.record_at(Duration::from_nanos(INTERVAL_NS), Some(Duration::ZERO));
        let measured = snapshot_at(
            0,
            "display",
            &writer.samples,
            Duration::from_nanos(INTERVAL_NS),
        )
        .unwrap();
        assert_eq!(measured.render_sample_count, 1);
        assert_eq!(measured.render_time_p95_ms, 0.0);
        assert_eq!(
            measured.render_measurement_boundary,
            "prepare-wall-excludes-gpu-composition-and-present"
        );
    }

    #[test]
    fn preparation_ring_wrap_retains_paired_successes_and_clears_unknown_samples() {
        let mut writer = OutputHealthWriter::register(u64::MAX - 5, "spout");
        for index in 0..800 {
            writer.record_at(
                Duration::from_nanos(index * INTERVAL_NS),
                Some(Duration::from_millis(index)),
            );
        }
        let retained = snapshot_at(
            0,
            "spout",
            &writer.samples,
            Duration::from_nanos(799 * INTERVAL_NS),
        )
        .unwrap();
        assert_eq!(retained.sample_count, CAPACITY - 1);
        assert_eq!(retained.render_sample_count, CAPACITY);
        assert_eq!(retained.render_time_p50_ms, 499.0);
        assert_eq!(retained.render_time_p95_ms, 769.0);
        assert_eq!(retained.render_time_p99_ms, 793.0);
        for index in 800..1400 {
            writer.record_at(Duration::from_nanos(index * INTERVAL_NS), None);
        }
        let unknown = snapshot(0, "spout", &writer.samples).unwrap();
        assert_eq!(unknown.render_sample_count, 0);
        assert_eq!(unknown.render_time_p95_ms, 0.0);
        assert_eq!(unknown.delivered_frames, 1400);
    }

    #[test]
    fn present_interval_histogram_is_exact_cumulative_and_differences_between_snapshots() {
        let mut writer = OutputHealthWriter::register(u64::MAX - 7, "display");
        let at = Duration::from_micros;
        // Bin edges are upper-inclusive: exactly 20.0 ms is in (19.9, 20.0].
        let mut t = 0;
        writer.record_at(at(t), None);
        for interval in [16_600, 16_700, 20_000, 20_050, 33_300, 150_000] {
            t += interval;
            writer.record_at(at(t), None);
        }
        let before = snapshot_at(0, "display", &writer.samples, at(t)).unwrap();
        let first = &before.present_intervals;
        assert_eq!(before.delivered_frames, 7);
        assert_eq!(first.interval_count, 6);
        assert_eq!(first.bin_width_ns, INTERVAL_BIN_NS);
        assert_eq!(
            first.bins,
            vec![[165, 1], [166, 1], [199, 1], [200, 1], [332, 1]]
        );
        assert_eq!(first.overflow_count, 1);
        assert_eq!(first.interval_max_ns, 150_000_000);
        assert_eq!(first.interval_total_ns, at(t).as_nanos() as u64);
        assert_eq!(
            before.last_delivery_elapsed_ns,
            Some(at(t).as_nanos() as u64)
        );
        for _ in 0..3 {
            t += 16_700;
            writer.record_at(at(t), None);
        }
        let after = snapshot_at(0, "display", &writer.samples, at(t + 1_000)).unwrap();
        let second = &after.present_intervals;
        assert_eq!(second.writer_generation, first.writer_generation);
        assert_eq!(second.interval_count - first.interval_count, 3);
        assert_eq!(
            after.delivered_frames - before.delivered_frames,
            second.interval_count - first.interval_count
        );
        assert_eq!(second.bins[1], [166, 4]);
        assert_eq!(after.snapshot_elapsed_ns, at(t + 1_000).as_nanos() as u64);
        let next = OutputHealthWriter::register(u64::MAX - 8, "display");
        let fresh = snapshot(0, "display", &next.samples).unwrap();
        assert!(fresh.present_intervals.writer_generation > first.writer_generation);
        assert_eq!(fresh.present_intervals.interval_count, 0);
        assert!(fresh.present_intervals.bins.is_empty());
        assert_eq!(fresh.last_delivery_elapsed_ns, None);
    }

    #[test]
    fn stalled_producer_reports_age_without_fabricating_new_preparation_samples() {
        let mut writer = OutputHealthWriter::register(u64::MAX - 6, "ndi");
        for index in 0..=60 {
            writer.record_at(
                Duration::from_nanos(index * INTERVAL_NS),
                Some(Duration::from_millis(2)),
            );
        }
        let recent = snapshot_at(
            0,
            "ndi",
            &writer.samples,
            Duration::from_nanos(60 * INTERVAL_NS),
        )
        .unwrap();
        let stale = snapshot_at(
            0,
            "ndi",
            &writer.samples,
            Duration::from_nanos(180 * INTERVAL_NS),
        )
        .unwrap();
        assert!((recent.delivered_fps - 60.0).abs() < 0.001);
        assert!((stale.delivered_fps - 20.0).abs() < 0.001);
        assert!(stale.last_frame_age_ms.unwrap() > 1900.0);
        assert_eq!(stale.render_sample_count, 61);
        assert_eq!(stale.render_time_p95_ms, 2.0);
        assert_eq!(stale.delivered_frames, recent.delivered_frames);
    }
}
