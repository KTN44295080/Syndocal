//! Fixed-capacity slow-work selection. The caller owns clocks and lifecycle.
//!
//! All timestamps and work samples come from the resident owner. In particular,
//! `work_us` must be the same truncated sample submitted to the G3 validator;
//! it is not the enclosing trace parent's elapsed time. Only `snapshot` allocates.
use serde::Serialize;
use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::atomic::Ordering;
use std::time::Instant;

use super::{Phase, PHASE_COUNT, PHASE_ORDER};

pub(super) const MAX_SPIKE_RECORDS: usize = 16;
pub(super) const WORK_THRESHOLD_US: u64 = 33_333;
const MAX_RETAINED_BYTES: usize = 16 * 1024;
const MAX_PHASE_DEPTH: usize = 16;

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
    /// Inclusive wall durations: nested phases overlap and must not be summed
    /// as an exclusive work budget or a critical path.
    pub phase_wall_us: [u64; PHASE_COUNT],
    /// Successfully completed synchronous owner spans, in `phase_order`.
    pub phase_calls: [u64; PHASE_COUNT],
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
    current: Option<CurrentSpike>,
    next_span_serial: u64,
    retired: bool,
    unfinished_iteration: Option<u64>,
    counters: SpikeCounters,
}

struct CurrentSpike {
    iteration: u64,
    phase_wall_us: [u64; PHASE_COUNT],
    phase_calls: [u64; PHASE_COUNT],
    phase_stack: [u64; MAX_PHASE_DEPTH],
    phase_depth: usize,
    phases_invalid: bool,
}

impl CurrentSpike {
    fn new(iteration: u64) -> Self {
        Self {
            iteration,
            phase_wall_us: [0; PHASE_COUNT],
            phase_calls: [0; PHASE_COUNT],
            phase_stack: [0; MAX_PHASE_DEPTH],
            phase_depth: 0,
            phases_invalid: false,
        }
    }
}

#[derive(Clone, Copy)]
struct PhaseToken {
    generation: u64,
    iteration: u64,
    serial: u64,
    phase: Phase,
}

pub(super) const RETAINED_BYTES: usize = std::mem::size_of::<SpikeBuffer>();
const _: () = assert!(RETAINED_BYTES <= MAX_RETAINED_BYTES);

#[derive(Debug, Serialize)]
pub(super) struct SpikeSnapshot {
    pub(super) generation: u64,
    pub(super) retained_bytes: usize,
    pub(super) record_capacity: usize,
    pub(super) work_threshold_us: u64,
    pub(super) phase_order: [Phase; PHASE_COUNT],
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
            next_span_serial: 0,
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
        self.current = Some(CurrentSpike::new(iteration));
        Some(iteration)
    }

    fn invalidate_current_phases(&mut self) {
        self.invalid();
        if let Some(current) = self.current.as_mut() {
            current.phases_invalid = true;
            current.phase_depth = 0;
        }
    }

    /// No clock is read until the caller has an owner-local frame and token.
    /// Stop deliberately does not participate: an admitted frame may finish.
    fn begin_phase(&mut self, phase: Phase) -> Option<PhaseToken> {
        let current = self.current.as_ref()?;
        if self.retired || current.phases_invalid {
            return None;
        }
        if current.phase_depth == MAX_PHASE_DEPTH {
            self.invalidate_current_phases();
            return None;
        }
        let Some(serial) = self.next_span_serial.checked_add(1) else {
            self.invalidate_current_phases();
            return None;
        };
        self.next_span_serial = serial;
        let current = self.current.as_mut()?;
        current.phase_stack[current.phase_depth] = serial;
        current.phase_depth += 1;
        Some(PhaseToken {
            generation: self.generation,
            iteration: current.iteration,
            serial,
            phase,
        })
    }

    /// Invoke the clock only after token validation. Stale spans never modify
    /// another frame's summaries, even when it reuses the same phase slots.
    fn finish_phase(&mut self, token: PhaseToken, elapsed_us: impl FnOnce() -> u64) -> bool {
        if token.generation != self.generation
            || self.retired
            || self.current.as_ref().map(|c| c.iteration) != Some(token.iteration)
        {
            self.invalid();
            return false;
        }
        let Some(current) = self.current.as_ref() else {
            return false;
        };
        if current.phases_invalid
            || current.phase_depth == 0
            || current.phase_stack[current.phase_depth - 1] != token.serial
        {
            self.invalidate_current_phases();
            return false;
        }
        let elapsed_us = elapsed_us();
        let Some(current) = self.current.as_mut() else {
            return false;
        };
        let index = token.phase as usize;
        let wall_us = current.phase_wall_us[index].checked_add(elapsed_us);
        let calls = current.phase_calls[index].checked_add(1);
        if let (Some(wall_us), Some(calls)) = (wall_us, calls) {
            current.phase_depth -= 1;
            current.phase_wall_us[index] = wall_us;
            current.phase_calls[index] = calls;
            true
        } else {
            self.invalidate_current_phases();
            false
        }
    }

    /// Complete only the exact admitted begin token. No product operation is
    /// retried, interrupted or panicked when diagnostic ownership is invalid.
    pub(super) fn finish(&mut self, generation: u64, mut record: SpikeRecord) -> bool {
        if generation == 0
            || generation != self.generation
            || self.retired
            || self.current.as_ref().map(|c| c.iteration) != Some(record.iteration)
        {
            self.invalid();
            return false;
        }
        let Some(current) = self.current.take() else {
            return false;
        };
        if current.phase_depth != 0 {
            self.invalid();
        }
        record.partial |= current.phases_invalid || current.phase_depth != 0;
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
                record.phase_wall_us = current.phase_wall_us;
                record.phase_calls = current.phase_calls;
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
            if let Some(current) = self.current.take() {
                self.unfinished_iteration = Some(current.iteration);
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
            phase_order: PHASE_ORDER,
            counters: self.counters,
            retired: self.retired,
            current_iteration: self.current.as_ref().map(|c| c.iteration),
            unfinished_iteration: self.unfinished_iteration,
            records: self.records[..self.len].to_vec(),
        }
    }
}

/// The guard cannot cross threads; its token addresses only this thread's
/// current frame. Rc appears solely in PhantomData and never allocates.
pub(super) struct SpikeSpanGuard {
    token: PhaseToken,
    started: Instant,
    _not_send: PhantomData<Rc<()>>,
}

#[cfg(test)]
pub(super) static PHASE_CLOCK_READS: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(0);

fn phase_clock() -> Instant {
    #[cfg(test)]
    PHASE_CLOCK_READS.fetch_add(1, Ordering::Relaxed);
    Instant::now()
}

/// Called only after the parent module's startup atomic dispatch. A worker
/// without the render owner's buffer/current frame reads no clock or resource.
pub(super) fn begin_span(phase: Phase) -> Option<SpikeSpanGuard> {
    super::SPIKES.with(|cell| {
        let mut slot = cell.borrow_mut();
        let token = slot.as_mut()?.begin_phase(phase)?;
        Some(SpikeSpanGuard {
            token,
            started: phase_clock(),
            _not_send: PhantomData,
        })
    })
}

impl Drop for SpikeSpanGuard {
    fn drop(&mut self) {
        super::SPIKES.with(|cell| {
            if let Some(buffer) = cell.borrow_mut().as_mut() {
                buffer.finish_phase(self.token, || {
                    duration_us(phase_clock().saturating_duration_since(self.started))
                });
            }
        });
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
        for (index, phase) in snapshot.phase_order.iter().enumerate() {
            assert_eq!(*phase as usize, index);
        }
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

    fn finish_current(buffer: &mut SpikeBuffer, iteration: u64) {
        assert!(buffer.finish(
            GENERATION,
            SpikeRecord {
                iteration,
                work_us: 40_000,
                work_observed: true,
                admitted: true,
                ..SpikeRecord::default()
            }
        ));
    }

    #[test]
    fn spike_phase_nested_summaries_are_inclusive_and_repeatable() {
        let mut buffer = SpikeBuffer::new(GENERATION);
        assert!(buffer.begin_phase(Phase::Prepare).is_none());
        let iteration = buffer.begin(GENERATION).unwrap();
        let outer = buffer.begin_phase(Phase::Prepare).unwrap();
        let demand = buffer.begin_phase(Phase::Demand).unwrap();
        let nested_demand = buffer.begin_phase(Phase::Demand).unwrap();
        assert!(buffer.finish_phase(nested_demand, || 10));
        assert!(buffer.finish_phase(demand, || 40));
        assert!(buffer.finish_phase(outer, || 100));
        let next = buffer.begin_phase(Phase::Prepare).unwrap();
        assert!(buffer.finish_phase(next, || 20));
        finish_current(&mut buffer, iteration);
        let snapshot = buffer.snapshot();
        let record = &snapshot.records[0];
        assert_eq!(record.phase_wall_us[Phase::Prepare as usize], 120);
        assert_eq!(record.phase_calls[Phase::Prepare as usize], 2);
        assert_eq!(record.phase_wall_us[Phase::Demand as usize], 50);
        assert_eq!(record.phase_calls[Phase::Demand as usize], 2);
        assert_eq!(record.phase_calls.iter().sum::<u64>(), 4);
        assert_eq!(record.phase_wall_us.iter().sum::<u64>(), 170);
        assert!(!record.partial);
        assert_eq!(snapshot.counters.invalid_count, 0);
    }

    #[test]
    fn spike_phase_stale_generation_and_iteration_never_reach_clock_or_next_frame() {
        let mut buffer = SpikeBuffer::new(GENERATION);
        let first = buffer.begin(GENERATION).unwrap();
        let old = buffer.begin_phase(Phase::Isf).unwrap();
        finish_current(&mut buffer, first); // Open span invalidates this frame.
        let second = buffer.begin(GENERATION).unwrap();
        let current = buffer.begin_phase(Phase::Demand).unwrap();
        assert!(!buffer.finish_phase(old, || panic!("stale token read the clock")));
        assert!(!buffer.finish_phase(
            PhaseToken {
                generation: GENERATION + 1,
                ..current
            },
            || panic!("wrong generation read the clock")
        ));
        assert!(buffer.finish_phase(current, || 60));
        finish_current(&mut buffer, second);
        let snapshot = buffer.snapshot();
        assert_eq!(snapshot.counters.invalid_count, 3);
        assert!(snapshot.records[0].partial);
        assert!(!snapshot.records[1].partial);
        assert_eq!(
            snapshot.records[1].phase_wall_us[Phase::Demand as usize],
            60
        );
        assert_eq!(snapshot.records[1].phase_calls[Phase::Isf as usize], 0);
    }

    #[test]
    fn spike_phase_misordered_drop_invalidates_tree_without_fabricating_time() {
        let mut buffer = SpikeBuffer::new(GENERATION);
        let iteration = buffer.begin(GENERATION).unwrap();
        let outer = buffer.begin_phase(Phase::Prepare).unwrap();
        let inner = buffer.begin_phase(Phase::Demand).unwrap();
        assert!(!buffer.finish_phase(outer, || panic!("misordered span read clock")));
        assert!(!buffer.finish_phase(inner, || panic!("invalid tree read clock")));
        assert!(buffer.begin_phase(Phase::Isf).is_none());
        finish_current(&mut buffer, iteration);
        let snapshot = buffer.snapshot();
        assert_eq!(snapshot.counters.invalid_count, 2);
        assert!(snapshot.records[0].partial);
        assert_eq!(snapshot.records[0].phase_calls, [0; PHASE_COUNT]);
    }

    #[test]
    fn spike_phase_depth_and_serial_overflow_are_bounded_and_explicit() {
        let mut buffer = SpikeBuffer::new(GENERATION);
        let iteration = buffer.begin(GENERATION).unwrap();
        for _ in 0..MAX_PHASE_DEPTH {
            assert!(buffer.begin_phase(Phase::Prepare).is_some());
        }
        assert!(buffer.begin_phase(Phase::Prepare).is_none());
        finish_current(&mut buffer, iteration);
        assert!(buffer.snapshot().records[0].partial);
        assert_eq!(buffer.snapshot().counters.invalid_count, 1);
        let next = buffer.begin(GENERATION).unwrap();
        buffer.next_span_serial = u64::MAX;
        assert!(buffer.begin_phase(Phase::Demand).is_none());
        finish_current(&mut buffer, next);
        let snapshot = buffer.snapshot();
        assert!(snapshot.records[1].partial);
        assert_eq!(snapshot.counters.invalid_count, 2);
        assert!(snapshot.retained_bytes <= MAX_RETAINED_BYTES);
    }

    #[test]
    fn spike_phase_aggregate_overflow_and_duplicate_token_are_rejected() {
        let mut buffer = SpikeBuffer::new(GENERATION);
        let iteration = buffer.begin(GENERATION).unwrap();
        let first = buffer.begin_phase(Phase::Demand).unwrap();
        assert!(buffer.finish_phase(first, || u64::MAX));
        let second = buffer.begin_phase(Phase::Demand).unwrap();
        assert!(!buffer.finish_phase(second, || 1));
        finish_current(&mut buffer, iteration);
        let next = buffer.begin(GENERATION).unwrap();
        let token = buffer.begin_phase(Phase::Isf).unwrap();
        assert!(buffer.finish_phase(token, || 7));
        assert!(!buffer.finish_phase(token, || panic!("duplicate token read clock")));
        finish_current(&mut buffer, next);
        let snapshot = buffer.snapshot();
        assert_eq!(snapshot.counters.invalid_count, 2);
        assert!(snapshot.records.iter().all(|record| record.partial));
        assert_eq!(
            snapshot.records[0].phase_wall_us[Phase::Demand as usize],
            u64::MAX
        );
        assert_eq!(snapshot.records[0].phase_calls[Phase::Demand as usize], 1);
        assert_eq!(snapshot.records[1].phase_calls[Phase::Isf as usize], 1);
    }
}
