//! Monotonic output cadence. Compensates sleep overshoot without queued frames.
use std::collections::VecDeque;
use std::time::{Duration, Instant};

pub(crate) struct VideoOutputPacer {
    interval: Duration,
    deadline: Instant,
}

impl VideoOutputPacer {
    pub(crate) fn new(interval: Duration) -> Self {
        Self::starting_at(Instant::now(), interval)
    }

    fn starting_at(started: Instant, interval: Duration) -> Self {
        assert!(!interval.is_zero());
        Self {
            interval,
            deadline: started + interval,
        }
    }

    /// Start of the slot the current frame belongs to (`wait_after_frame`
    /// retires every overdue slot, so this is never after the frame start).
    pub(crate) fn slot_start(&self) -> Instant {
        self.deadline
            .checked_sub(self.interval)
            .unwrap_or(self.deadline)
    }

    pub(crate) fn interval(&self) -> Duration {
        self.interval
    }

    pub(crate) fn wait_after_frame(&mut self) {
        let remaining = self.remaining_at(Instant::now());
        if !remaining.is_zero() {
            std::thread::sleep(remaining);
        }
    }

    fn remaining_at(&mut self, now: Instant) -> Duration {
        let remaining = self.deadline.saturating_duration_since(now);
        let periods = if now >= self.deadline {
            now.duration_since(self.deadline).as_nanos() / self.interval.as_nanos() + 1
        } else {
            1
        };
        // Retire every elapsed slot arithmetically. Over-budget rendering never
        // incurs an extra sleep, and the next call cannot replay overdue slots.
        self.deadline = u32::try_from(periods)
            .ok()
            .and_then(|periods| self.interval.checked_mul(periods))
            .and_then(|advance| self.deadline.checked_add(advance))
            .unwrap_or(now + self.interval);
        remaining
    }
}

/// Fixed-phase physical present (C4R7). Presenting as soon as preparation
/// finished put each present at a different offset in its slot: preparation
/// varies by several milliseconds, so successive presents were 11-22 ms apart
/// at a 60 Hz cadence (live 3-layer Display: rolling p95 21.1 ms, 646 of 3745
/// intervals over 20 ms) and, on a 180 Hz panel, frames were held for two to
/// four refreshes instead of three. The present instead waits until one
/// offset in the slot: a high percentile of the recent ready offsets plus a
/// small margin, never later than a cap that leaves the present itself room
/// inside the slot. A frame that is ready later presents at once.
pub(crate) struct PresentPhase {
    ready: VecDeque<Duration>,
    cap: Duration,
}

const PRESENT_PHASE_WINDOW: usize = 120;
const PRESENT_PHASE_MIN_SAMPLES: usize = 30;
const PRESENT_PHASE_MARGIN: Duration = Duration::from_micros(500);

impl PresentPhase {
    /// `cap`: the latest offset in the slot a present may be held to.
    pub(crate) fn new(cap: Duration) -> Self {
        Self {
            ready: VecDeque::with_capacity(PRESENT_PHASE_WINDOW),
            cap,
        }
    }

    /// The present cap for a cadence: 85% of the slot.
    pub(crate) fn for_interval(interval: Duration) -> Self {
        Self::new(interval.mul_f64(0.85))
    }

    /// Sleeps until this frame's present phase; returns the time waited,
    /// which is pacing and not preparation work.
    pub(crate) fn wait(&mut self, slot_start: Instant) -> Duration {
        let wait = self.wait_at(slot_start, Instant::now());
        if !wait.is_zero() {
            std::thread::sleep(wait);
        }
        wait
    }

    fn wait_at(&mut self, slot_start: Instant, now: Instant) -> Duration {
        let ready = now.saturating_duration_since(slot_start);
        if self.ready.len() == PRESENT_PHASE_WINDOW {
            self.ready.pop_front();
        }
        self.ready.push_back(ready);
        self.phase().saturating_sub(ready)
    }

    /// 98th percentile of the recent ready offsets plus the margin, capped.
    /// Zero (present at once) until enough frames were observed.
    fn phase(&self) -> Duration {
        if self.ready.len() < PRESENT_PHASE_MIN_SAMPLES {
            return Duration::ZERO;
        }
        let mut sorted = self.ready.iter().copied().collect::<Vec<_>>();
        sorted.sort_unstable();
        let rank = (sorted.len() * 98).div_ceil(100).max(1) - 1;
        (sorted[rank] + PRESENT_PHASE_MARGIN).min(self.cap)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_display_present_phase_aligns_jittered_presents_one_interval_apart() {
        let interval = Duration::from_micros(16_667);
        let mut phase = PresentPhase::for_interval(interval);
        let origin = Instant::now();
        let mut presents = Vec::new();
        for frame in 0..400_u64 {
            let slot = origin + interval * frame as u32;
            // Preparation jitters between 8 and 13 ms (the live 3-layer
            // spread), deterministic.
            let ready = Duration::from_micros(8_000 + (frame * 7_919 % 5_000));
            let wait = phase.wait_at(slot, slot + ready);
            presents.push(slot + ready + wait);
        }
        // After the estimator filled, presents are one interval apart; the
        // phase only moves with the window's 98th percentile (a few hundred
        // microseconds), whereas unpaced presents moved by up to 5 ms.
        let steady = &presents[PRESENT_PHASE_WINDOW..];
        for pair in steady.windows(2) {
            let gap = pair[1] - pair[0];
            assert!(
                gap.abs_diff(interval) <= Duration::from_micros(300),
                "present gap {gap:?}"
            );
        }
        // The offset is above every ready time and below the cap.
        let offset = steady[0] - (origin + interval * PRESENT_PHASE_WINDOW as u32);
        assert!(offset >= Duration::from_micros(13_000) && offset <= interval.mul_f64(0.85));
        // Before enough samples there is no wait.
        let mut cold = PresentPhase::for_interval(interval);
        assert_eq!(
            cold.wait_at(origin, origin + Duration::from_millis(3)),
            Duration::ZERO
        );
    }

    #[test]
    fn native_display_present_phase_never_waits_past_its_cap_or_delays_a_late_frame() {
        let interval = Duration::from_micros(16_667);
        let cap = interval.mul_f64(0.85);
        let mut phase = PresentPhase::for_interval(interval);
        let slot = Instant::now();
        // Rare 30 ms outliers (a loop wrap) cannot lift the phase above the
        // cap; ordinary 10 ms frames still present at a fixed offset.
        for frame in 0..200_u32 {
            let ready = if frame % 60 == 0 {
                Duration::from_millis(30)
            } else {
                Duration::from_millis(10)
            };
            let wait = phase.wait_at(slot, slot + ready);
            assert!(ready + wait <= cap.max(ready), "frame {frame}");
            if ready > cap {
                assert_eq!(wait, Duration::ZERO, "a late frame presents at once");
            }
        }
        // Mostly slow frames: the phase saturates at the cap.
        let mut slow = PresentPhase::for_interval(interval);
        let mut last = Duration::ZERO;
        for _ in 0..200 {
            last = slow.wait_at(slot, slot + Duration::from_millis(5));
        }
        assert_eq!(
            Duration::from_millis(5) + last,
            Duration::from_micros(5_500)
        );
        for _ in 0..200 {
            last = slow.wait_at(slot, slot + Duration::from_millis(15));
        }
        assert_eq!(last, Duration::ZERO);
        assert!(slow.phase() == cap);
    }

    #[test]
    fn output_pacer_compensates_repeated_sleep_overshoot() {
        let start = Instant::now();
        let mut pacer = VideoOutputPacer::starting_at(start, Duration::from_millis(10));
        assert_eq!(
            pacer.remaining_at(start + Duration::from_millis(4)),
            Duration::from_millis(6)
        );
        for frame in 1..100 {
            // Each OS sleep returns 1ms late; each render takes 4ms.
            assert_eq!(
                pacer.remaining_at(start + Duration::from_millis(frame * 10 + 5)),
                Duration::from_millis(5)
            );
            assert_eq!(
                pacer.deadline,
                start + Duration::from_millis((frame + 2) * 10)
            );
        }
    }

    #[test]
    fn output_pacer_skips_stalls_without_replaying_slots_or_limiting_slow_work() {
        let start = Instant::now();
        let mut pacer = VideoOutputPacer::starting_at(start, Duration::from_millis(10));
        assert_eq!(
            pacer.remaining_at(start + Duration::from_millis(35)),
            Duration::ZERO
        );
        assert_eq!(
            pacer.remaining_at(start + Duration::from_millis(35)),
            Duration::from_millis(5)
        );
        assert_eq!(
            pacer.remaining_at(start + Duration::from_millis(50)),
            Duration::ZERO
        );
        // A 20ms render remains maximum-throughput 50fps, not phase-quantized
        // to 33fps by sleeping an additional target slot after every frame.
        for elapsed in [70, 90, 110, 130] {
            assert_eq!(
                pacer.remaining_at(start + Duration::from_millis(elapsed)),
                Duration::ZERO
            );
            assert!(pacer.deadline > start + Duration::from_millis(elapsed));
        }
    }
}
