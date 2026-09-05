//! Bounded recent-sample statistics, independent of engine and output state.

const CAPACITY: usize = 256;

pub(super) struct PercentileWindow {
    samples: [u64; CAPACITY],
    len: usize,
    next: usize,
}

impl Default for PercentileWindow {
    fn default() -> Self {
        Self {
            samples: [0; CAPACITY],
            len: 0,
            next: 0,
        }
    }
}

impl PercentileWindow {
    pub(super) fn record(&mut self, sample: u64) {
        self.samples[self.next] = sample;
        self.next = (self.next + 1) % CAPACITY;
        self.len = (self.len + 1).min(CAPACITY);
    }

    pub(super) fn reset(&mut self) {
        *self = Self::default();
    }

    /// Both nearest-rank percentiles use the same sorted, stack-only copy.
    pub(super) fn p95_p99(&self) -> (u64, u64) {
        if self.len == 0 {
            return (0, 0);
        }
        let mut samples = self.samples;
        samples[..self.len].sort_unstable();
        let nearest_rank = |percentile: f64| {
            let index = ((self.len as f64 * percentile).ceil() as usize).saturating_sub(1);
            samples[index.min(self.len - 1)]
        };
        (nearest_rank(0.95), nearest_rank(0.99))
    }
}

#[cfg(test)]
#[path = "telemetry_percentiles_tests.rs"]
mod tests;
