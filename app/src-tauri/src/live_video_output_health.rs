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
// Zero is a valid preparation duration. Unmeasured startup / cached sends
// must not become fabricated zero-cost preparation samples.
const NO_RENDER_TIME_NS: u64 = u64::MAX;
type Registry = BTreeMap<(u64, &'static str), Weak<Samples>>;
static REGISTRY: OnceLock<Mutex<Registry>> = OnceLock::new();

struct Samples {
    origin: Instant,
    timestamps_ns: [AtomicU64; CAPACITY],
    render_times_ns: [AtomicU64; CAPACITY],
    count: AtomicU64,
    revision: AtomicU64,
    dropped: AtomicU64,
}

pub(crate) struct OutputHealthWriter {
    samples: Arc<Samples>,
    first_ns: Option<u64>,
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
}

impl OutputHealthWriter {
    pub(crate) fn register(output_id: u64, kind: &'static str) -> Self {
        let samples = Arc::new(Samples {
            origin: Instant::now(),
            timestamps_ns: std::array::from_fn(|_| AtomicU64::new(0)),
            render_times_ns: std::array::from_fn(|_| AtomicU64::new(NO_RENDER_TIME_NS)),
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
        let age_ns = timestamps.last().map(|last| {
            (elapsed.as_nanos().min(u128::from(u64::MAX)) as u64).saturating_sub(*last)
        });
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
