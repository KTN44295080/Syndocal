//! Fixed-capacity slow-work selection. The caller owns clocks and lifecycle.
//!
//! All timestamps and work samples come from the resident owner. In particular,
//! `work_us` must be the same truncated sample submitted to the G3 validator;
//! it is not the enclosing trace parent's elapsed time. Only `snapshot` allocates.
use serde::Serialize;
use std::sync::atomic::Ordering;
use std::time::Instant;

pub(super) const MAX_SPIKE_RECORDS: usize = 16;
pub(super) const WORK_THRESHOLD_US: u64 = 33_333;
const MAX_RETAINED_BYTES: usize = 16 * 1024;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct SpikeRecord {
    pub iteration: u64,
    pub output_id: u64,
    pub last_presented_request_sequence: u64,
    pub frames_presented: u64,
    pub work_sample_sequence: u64,
    pub validation_epoch: u64,
    pub start_us: u64,
    pub sample_us: u64,
    pub work_us: u64,
    pub pacing_us: u64,
    pub extent_us: u64,
    pub permit_us: u64,
    pub retirement_configure_us: u64,
    pub prepare_us: u64,
    /// Wall interval including the separately reported pacing subset.
    pub present_with_pacing_us: u64,
    pub metrics_lock_wait_us: u64,
    pub work_observed: bool,
    pub admitted: bool,
    pub partial: bool,
}

/// Counters overlap deliberately: a partial record can also be excluded or a
/// matched slow sample. Fast counts include only complete, observed samples.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub(super) struct SpikeCounters {
    pub(super) attempted_iterations: u64,
    pub(super) completed_iterations: u64,
    pub(super) excluded_count: u64,
    pub(super) fast_count: u64,
    pub(super) matched_count: u64,
    pub(super) partial_count: u64,
    pub(super) unobserved_count: u64,
    pub(super) overflow_count: u64,
    pub(super) invalid_count: u64,
    pub(super) incomplete_count: u64,
}

pub(super) struct SpikeBuffer {
    generation: u64,
    records: [SpikeRecord; MAX_SPIKE_RECORDS],
    len: usize,
    current: Option<u64>,
    retired: bool,
    unfinished_iteration: Option<u64>,
    counters: SpikeCounters,
}

pub(super) const RETAINED_BYTES: usize = std::mem::size_of::<SpikeBuffer>();
const _: () = assert!(RETAINED_BYTES <= MAX_RETAINED_BYTES);

#[derive(Debug, Serialize)]
pub(super) struct SpikeSnapshot {
    pub(super) generation: u64,
    pub(super) retained_bytes: usize,
    pub(super) record_capacity: usize,
    pub(super) work_threshold_us: u64,
    pub(super) counters: SpikeCounters,
    pub(super) retired: bool,
    pub(super) current_iteration: Option<u64>,
    pub(super) unfinished_iteration: Option<u64>,
    pub(super) records: Vec<SpikeRecord>,
}

impl SpikeBuffer {
    pub(super) fn generation(&self) -> u64 {
        self.generation
    }

    pub(super) fn new(generation: u64) -> Self {
        Self {
            generation,
            records: [SpikeRecord::default(); MAX_SPIKE_RECORDS],
            len: 0,
            current: None,
            retired: false,
            unfinished_iteration: None,
            counters: SpikeCounters::default(),
        }
    }

    fn invalid(&mut self) {
        self.counters.invalid_count = self.counters.invalid_count.saturating_add(1);
    }

    /// Count every attempted tick, including rejected begins. An invalid begin
    /// leaves the active token intact so its legitimate owner can still finish.
    pub(super) fn begin(&mut self, generation: u64) -> Option<u64> {
        let Some(iteration) = self.counters.attempted_iterations.checked_add(1) else {
            self.invalid();
            return None;
        };
        self.counters.attempted_iterations = iteration;
        if generation == 0
            || generation != self.generation
            || self.retired
            || self.current.is_some()
        {
            self.invalid();
            return None;
        }
        self.current = Some(iteration);
        Some(iteration)
    }

    /// Complete only the exact admitted begin token. No product operation is
    /// retried, interrupted or panicked when diagnostic ownership is invalid.
    pub(super) fn finish(&mut self, generation: u64, record: SpikeRecord) -> bool {
        if generation == 0
            || generation != self.generation
            || self.retired
            || self.current != Some(record.iteration)
        {
            self.invalid();
            return false;
        }
        self.current = None;
        self.counters.completed_iterations = self.counters.completed_iterations.saturating_add(1);
        if !record.admitted {
            self.counters.excluded_count = self.counters.excluded_count.saturating_add(1);
        }
        if record.partial {
            self.counters.partial_count = self.counters.partial_count.saturating_add(1);
        }
        if !record.work_observed {
            self.counters.unobserved_count = self.counters.unobserved_count.saturating_add(1);
        }
        let matched = record.admitted && record.work_observed && record.work_us > WORK_THRESHOLD_US;
        if matched {
            self.counters.matched_count = self.counters.matched_count.saturating_add(1);
        } else if record.admitted && record.work_observed && !record.partial {
            self.counters.fast_count = self.counters.fast_count.saturating_add(1);
        }
        if matched || record.partial || !record.work_observed {
            if self.len < MAX_SPIKE_RECORDS {
                self.records[self.len] = record;
                self.len += 1;
            } else {
                self.counters.overflow_count = self.counters.overflow_count.saturating_add(1);
            }
        }
        true
    }

    /// Cold owner retirement. Preserve an unfinished token explicitly rather
    /// than inventing a completed record whose work and identity are unknown.
    /// Repeated retirement is idempotent and keeps the initial result.
    pub(super) fn retire(&mut self, generation: u64) -> bool {
        if generation == 0 || generation != self.generation {
            self.invalid();
            return false;
        }
        if !self.retired {
            self.retired = true;
            if let Some(iteration) = self.current.take() {
                self.unfinished_iteration = Some(iteration);
                self.counters.incomplete_count = self.counters.incomplete_count.saturating_add(1);
                self.invalid();
            }
        }
        self.unfinished_iteration.is_none()
    }

    /// Cold export only. The enclosing capture must enforce Stop/ACK and owner
    /// retirement before exporting; these fields make incomplete state visible.
    pub(super) fn snapshot(&self) -> SpikeSnapshot {
        SpikeSnapshot {
            generation: self.generation,
            retained_bytes: RETAINED_BYTES,
            record_capacity: MAX_SPIKE_RECORDS,
            work_threshold_us: WORK_THRESHOLD_US,
            counters: self.counters,
            retired: self.retired,
            current_iteration: self.current,
            unfinished_iteration: self.unfinished_iteration,
            records: self.records[..self.len].to_vec(),
        }
    }
}

#[derive(Clone, Copy)]
pub enum SpikeStage {
    Extent,
    Permit,
    RetirementConfigure,
    Prepare,
    Present,
}

struct ActiveSpikeFrame {
    generation: u64,
    record: SpikeRecord,
    last_mark: Instant,
}

/// Only slow-mode admission touches its TLS. Full trace decoder tokens and
/// parent slots are never recycled or shared with this coarse recorder.
pub struct SpikeFrame(Option<ActiveSpikeFrame>);

pub struct SpikeMetricSample {
    pub sample_us: u64,
    pub elapsed: std::time::Duration,
    pub metrics_lock_wait: std::time::Duration,
    pub admitted: bool,
    pub validation_epoch: u64,
    pub last_presented_request_sequence: u64,
    pub frames_presented: u64,
    pub work_sample_sequence: u64,
}

fn duration_us(duration: std::time::Duration) -> u64 {
    duration.as_micros().min(u128::from(u64::MAX)) as u64
}

fn relative_instant_us(instant: Instant) -> u64 {
    // Convert the exact work-boundary Instant, rather than sampling two
    // independent clocks around a possible deschedule.
    super::CLOCK.get().map_or(0, |origin| {
        duration_us(instant.saturating_duration_since(*origin))
            .saturating_sub(super::ORIGIN_US.load(Ordering::Relaxed))
    })
}

pub fn begin_spike_frame(output_id: u64, started: Instant) -> SpikeFrame {
    if !super::SPIKE_ENABLED.load(Ordering::Relaxed) {
        return SpikeFrame(None);
    }
    let generation = super::ACTIVE.load(Ordering::Acquire);
    if generation == 0 {
        return SpikeFrame(None);
    }
    let iteration =
        super::SPIKES.with(|cell| cell.borrow_mut().as_mut().and_then(|b| b.begin(generation)));
    let Some(iteration) = iteration else {
        super::UNPREPARED.fetch_add(1, Ordering::Relaxed);
        return SpikeFrame(None);
    };
    let start_us = relative_instant_us(started);
    SpikeFrame(Some(ActiveSpikeFrame {
        generation,
        record: SpikeRecord {
            iteration,
            output_id,
            start_us,
            ..SpikeRecord::default()
        },
        last_mark: started,
    }))
}

impl SpikeFrame {
    pub fn mark(&mut self, stage: SpikeStage) {
        let Some(frame) = self.0.as_mut() else {
            return;
        };
        let now = Instant::now();
        let elapsed = duration_us(now.saturating_duration_since(frame.last_mark));
        match stage {
            SpikeStage::Extent => frame.record.extent_us = elapsed,
            SpikeStage::Permit => frame.record.permit_us = elapsed,
            SpikeStage::RetirementConfigure => frame.record.retirement_configure_us = elapsed,
            SpikeStage::Prepare => frame.record.prepare_us = elapsed,
            SpikeStage::Present => frame.record.present_with_pacing_us = elapsed,
        }
        frame.last_mark = now;
    }

    pub fn lock_clock(&self) -> Option<Instant> {
        self.0.as_ref().map(|_| Instant::now())
    }

    pub fn sample_clock(&self, instant: Instant) -> u64 {
        if self.0.is_some() {
            relative_instant_us(instant)
        } else {
            0
        }
    }

    pub fn set_pacing(&mut self, pacing: std::time::Duration) {
        if let Some(frame) = self.0.as_mut() {
            frame.record.pacing_us = duration_us(pacing);
        }
    }

    pub fn finish(&mut self, sample: SpikeMetricSample) {
        let Some(mut frame) = self.0.take() else {
            return;
        };
        frame.record.sample_us = sample.sample_us;
        // This is the exact Duration supplied to the product validator, not
        // the trace-parent duration or a sum of diagnostic phase timings.
        frame.record.work_us = duration_us(sample.elapsed);
        frame.record.metrics_lock_wait_us = duration_us(sample.metrics_lock_wait);
        frame.record.admitted = sample.admitted;
        frame.record.work_observed = true;
        frame.record.validation_epoch = sample.validation_epoch;
        frame.record.last_presented_request_sequence = sample.last_presented_request_sequence;
        frame.record.frames_presented = sample.frames_presented;
        frame.record.work_sample_sequence = sample.work_sample_sequence;
        super::SPIKES.with(|cell| {
            if let Some(buffer) = cell.borrow_mut().as_mut() {
                buffer.finish(frame.generation, frame.record);
            }
        });
    }
}

impl Drop for SpikeFrame {
    fn drop(&mut self) {
        if let Some(mut frame) = self.0.take() {
            frame.record.partial = true;
            frame.record.sample_us = super::now_us();
            super::SPIKES.with(|cell| {
                if let Some(buffer) = cell.borrow_mut().as_mut() {
                    buffer.finish(frame.generation, frame.record);
                }
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GENERATION: u64 = 7;

    fn finish_sample(buffer: &mut SpikeBuffer, work_us: u64, admitted: bool) -> u64 {
        let iteration = buffer.begin(GENERATION).unwrap();
        assert!(buffer.finish(
            GENERATION,
            SpikeRecord {
                iteration,
                work_us,
                work_observed: true,
                admitted,
                ..SpikeRecord::default()
            }
        ));
        iteration
    }

    #[test]
    fn spike_threshold_matches_truncated_g3_maximum() {
        let mut buffer = SpikeBuffer::new(GENERATION);
        finish_sample(&mut buffer, 33_333, true);
        let slow = finish_sample(&mut buffer, 33_334, true);
        let snapshot = buffer.snapshot();
        assert_eq!(snapshot.counters.fast_count, 1);
        assert_eq!(snapshot.counters.matched_count, 1);
        assert_eq!(snapshot.records.len(), 1);
        assert_eq!(snapshot.records[0].iteration, slow);
        assert_eq!(snapshot.records[0].work_us, 33_334);
        assert_eq!(snapshot.retained_bytes, RETAINED_BYTES);
        assert!(RETAINED_BYTES <= MAX_RETAINED_BYTES);
    }

    #[test]
    fn spike_excluded_partial_and_unobserved_are_distinct() {
        let mut buffer = SpikeBuffer::new(GENERATION);
        finish_sample(&mut buffer, 90_000, false);
        let iteration = buffer.begin(GENERATION).unwrap();
        assert!(buffer.finish(
            GENERATION,
            SpikeRecord {
                iteration,
                admitted: false,
                partial: true,
                ..SpikeRecord::default()
            }
        ));
        let snapshot = buffer.snapshot();
        assert_eq!(snapshot.counters.excluded_count, 2);
        assert_eq!(snapshot.counters.partial_count, 1);
        assert_eq!(snapshot.counters.unobserved_count, 1);
        assert_eq!(snapshot.counters.fast_count, 0);
        assert_eq!(snapshot.counters.matched_count, 0);
        assert_eq!(snapshot.records.len(), 1);
        assert!(snapshot.records[0].partial);
        assert!(!snapshot.records[0].work_observed);
    }

    #[test]
    fn spike_capacity_freezes_first_records_and_counts_overflow() {
        let mut buffer = SpikeBuffer::new(GENERATION);
        for work_us in 40_000..40_000 + MAX_SPIKE_RECORDS as u64 + 3 {
            finish_sample(&mut buffer, work_us, true);
        }
        let iteration = buffer.begin(GENERATION).unwrap();
        assert!(buffer.finish(
            GENERATION,
            SpikeRecord {
                iteration,
                partial: true,
                ..SpikeRecord::default()
            }
        ));
        let snapshot = buffer.snapshot();
        assert_eq!(snapshot.records.len(), MAX_SPIKE_RECORDS);
        assert_eq!(snapshot.records[0].work_us, 40_000);
        assert_eq!(snapshot.records[MAX_SPIKE_RECORDS - 1].work_us, 40_015);
        assert_eq!(snapshot.counters.matched_count, 19);
        assert_eq!(snapshot.counters.overflow_count, 4);
        assert_eq!(snapshot.counters.completed_iterations, 20);
        assert_eq!(snapshot.counters.partial_count, 1);
        assert_eq!(snapshot.counters.unobserved_count, 1);
    }

    #[test]
    fn spike_iteration_gaps_preserve_fast_and_excluded_ticks() {
        let mut buffer = SpikeBuffer::new(GENERATION);
        finish_sample(&mut buffer, 1, true);
        assert_eq!(finish_sample(&mut buffer, 40_000, true), 2);
        finish_sample(&mut buffer, 50_000, false);
        finish_sample(&mut buffer, 2, true);
        assert_eq!(finish_sample(&mut buffer, 60_000, true), 5);
        let snapshot = buffer.snapshot();
        assert_eq!(snapshot.counters.attempted_iterations, 5);
        assert_eq!(snapshot.counters.completed_iterations, 5);
        assert_eq!(
            snapshot
                .records
                .iter()
                .map(|r| r.iteration)
                .collect::<Vec<_>>(),
            vec![2, 5]
        );
    }

    #[test]
    fn spike_after_one_hundred_thousand_fast_ticks_keeps_identity_and_capacity() {
        let mut buffer = SpikeBuffer::new(GENERATION);
        for _ in 0..100_000 {
            finish_sample(&mut buffer, 6_000, true);
        }
        assert_eq!(finish_sample(&mut buffer, 35_604, true), 100_001);
        for _ in 0..10 {
            finish_sample(&mut buffer, 6_000, true);
        }
        assert!(buffer.retire(GENERATION));
        let snapshot = buffer.snapshot();
        assert_eq!(snapshot.counters.attempted_iterations, 100_011);
        assert_eq!(snapshot.counters.completed_iterations, 100_011);
        assert_eq!(snapshot.counters.fast_count, 100_010);
        assert_eq!(snapshot.counters.matched_count, 1);
        assert_eq!(snapshot.counters.overflow_count, 0);
        assert_eq!(snapshot.counters.invalid_count, 0);
        assert_eq!(snapshot.records.len(), 1);
        assert_eq!(snapshot.records[0].iteration, 100_001);
        assert_eq!(snapshot.records[0].work_us, 35_604);
    }

    #[test]
    fn spike_invalid_tokens_do_not_consume_current_owner() {
        let mut buffer = SpikeBuffer::new(GENERATION);
        let iteration = buffer.begin(GENERATION).unwrap();
        assert_eq!(buffer.begin(GENERATION), None);
        assert_eq!(buffer.begin(GENERATION + 1), None);
        let record = SpikeRecord {
            iteration,
            work_us: 40_000,
            admitted: true,
            work_observed: true,
            ..SpikeRecord::default()
        };
        assert!(!buffer.finish(GENERATION + 1, record));
        assert!(!buffer.finish(
            GENERATION,
            SpikeRecord {
                iteration: iteration + 1,
                ..record
            }
        ));
        assert_eq!(buffer.snapshot().current_iteration, Some(iteration));
        assert!(buffer.finish(GENERATION, record));
        assert!(!buffer.finish(GENERATION, record));
        assert_eq!(finish_sample(&mut buffer, 40_001, true), 4);
        let snapshot = buffer.snapshot();
        assert_eq!(snapshot.counters.invalid_count, 5);
        assert_eq!(snapshot.counters.attempted_iterations, 4);
        assert_eq!(snapshot.counters.completed_iterations, 2);
        assert_eq!(snapshot.records.len(), 2);
    }

    #[test]
    fn spike_retirement_reports_unfinished_without_fabricating_work() {
        let mut buffer = SpikeBuffer::new(GENERATION);
        let iteration = buffer.begin(GENERATION).unwrap();
        assert!(!buffer.retire(GENERATION + 1));
        assert_eq!(buffer.snapshot().current_iteration, Some(iteration));
        assert!(!buffer.retire(GENERATION));
        assert!(!buffer.retire(GENERATION));
        let snapshot = buffer.snapshot();
        assert!(snapshot.retired);
        assert_eq!(snapshot.current_iteration, None);
        assert_eq!(snapshot.unfinished_iteration, Some(iteration));
        assert_eq!(snapshot.counters.incomplete_count, 1);
        assert_eq!(snapshot.counters.invalid_count, 2);
        assert!(snapshot.records.is_empty());
        assert_eq!(buffer.begin(GENERATION), None);
    }

    #[test]
    fn spike_clean_retirement_is_idempotent_and_zero_generation_is_invalid() {
        let mut buffer = SpikeBuffer::new(GENERATION);
        finish_sample(&mut buffer, 40_000, true);
        assert!(buffer.retire(GENERATION));
        assert!(buffer.retire(GENERATION));
        assert_eq!(buffer.snapshot().counters.invalid_count, 0);
        let mut zero = SpikeBuffer::new(0);
        assert_eq!(zero.begin(0), None);
        assert!(!zero.finish(0, SpikeRecord::default()));
        assert!(!zero.retire(0));
        assert_eq!(zero.snapshot().counters.invalid_count, 3);
    }
}
