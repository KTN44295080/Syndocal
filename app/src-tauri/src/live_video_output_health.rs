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
type Registry = BTreeMap<(u64, &'static str), Weak<Samples>>;
static REGISTRY: OnceLock<Mutex<Registry>> = OnceLock::new();

struct Samples {
    origin: Instant,
    timestamps_ns: [AtomicU64; CAPACITY],
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
        self.record_at(self.samples.origin.elapsed());
    }

    fn record_at(&mut self, elapsed: Duration) {
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
            (samples
                .origin
                .elapsed()
                .as_nanos()
                .min(u128::from(u64::MAX)) as u64)
                .saturating_sub(*last)
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
        let percentile = |p: usize| {
            if intervals.is_empty() {
                0.0
            } else {
                intervals[(intervals.len() * p).div_ceil(100).saturating_sub(1)] as f64 / 1e6
            }
        };
        return Some(LiveVideoOutputHealth {
            output_id,
            kind,
            delivered_fps,
            frame_time_p50_ms: percentile(50),
            frame_time_p95_ms: percentile(95),
            frame_time_p99_ms: percentile(99),
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
            writer.record_at(Duration::from_nanos(index * INTERVAL_NS));
        }
        let health = snapshot(u64::MAX, "test", &writer.samples).unwrap();
        assert_eq!(health.delivered_frames, 61);
        assert_eq!(health.sample_count, 60);
        assert_eq!(health.dropped_frames, 0);
        assert!((health.frame_time_p95_ms - 16.666666).abs() < 0.001);
        writer.record_at(Duration::from_nanos(63 * INTERVAL_NS));
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
        writer.record_at(Duration::ZERO);
        writer.record_at(Duration::from_nanos(4 * INTERVAL_NS));
        assert_eq!(writer.dropped, 3);
        // Successful submissions in a burst cannot erase the observed stall.
        for _ in 0..8 {
            writer.record_at(Duration::from_nanos(4 * INTERVAL_NS));
        }
        assert_eq!(writer.dropped, 3);
    }

    #[test]
    fn restarting_cadence_forgets_startup_gap_but_keeps_later_stalls() {
        let mut writer = OutputHealthWriter::register(u64::MAX - 3, "test");
        writer.record_at(Duration::ZERO);
        // The render worker starts hundreds of milliseconds after the first present.
        writer.restart_cadence();
        writer.record_at(Duration::from_nanos(30 * INTERVAL_NS));
        writer.record_at(Duration::from_nanos(31 * INTERVAL_NS));
        assert_eq!(writer.dropped, 0);
        writer.record_at(Duration::from_nanos(35 * INTERVAL_NS));
        assert_eq!(writer.dropped, 3);
    }

    #[test]
    fn bounded_ring_preserves_recent_cadence_and_retired_worker_disappears() {
        let mut writer = OutputHealthWriter::register(u64::MAX - 1, "test");
        for index in 0..800 {
            writer.record_at(Duration::from_nanos(index * INTERVAL_NS));
        }
        let health = snapshot(u64::MAX - 1, "test", &writer.samples).unwrap();
        assert_eq!(health.sample_count, CAPACITY - 1);
        assert_eq!(health.delivered_frames, 800);
        drop(writer);
        assert!(!snapshots()
            .iter()
            .any(|sample| sample.output_id == u64::MAX - 1));
    }
}
