//! Monotonic output cadence. Compensates sleep overshoot without queued frames.
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

#[cfg(test)]
mod tests {
    use super::*;

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
