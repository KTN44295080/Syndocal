//! Trace-OFF resident Display evidence (ADR-01 section 13.3 (4)-(7)).
//!
//! Cumulative counters that are never reset: a reader differences two status
//! snapshots to obtain a window. Each counter is incremented exactly where its
//! outcome happens in the resident session (tick execution path, history
//! materialization, snapshot publication, physical Presented receipt), never
//! inferred from route selection or from a missing value. Per tick the cost
//! is a few integer adds and one monotonic clock read; nothing allocates,
//! locks or performs I/O.
use serde::Serialize;
use std::time::Instant;

/// Render-to-present latency histogram: bin `k` holds latencies in
/// [k * width, (k + 1) * width); the last bin also holds everything longer.
const NATIVE_RESIDENT_LATENCY_BIN_US: u64 = 2_000;
const LATENCY_BINS: usize = 32;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct NativeResidentPathEvidence {
    /// Ticks whose layers executed as one resident GPU batch, and the sum of
    /// their layer counts (3 per tick for the three-layer fixture = Batch3).
    pub gpu_batch_ticks: u64,
    pub gpu_batch_layers: u64,
    /// Pre-execution budget refusals (tick skipped).
    pub budget_declines: u64,
    /// Ticks declined to the canonical CPU continuation.
    pub cpu_declines: u64,
    /// Full frames composed on the CPU (continuation, recovery, Follow or a
    /// non-resident scope); payload-less blackout is counted separately.
    pub cpu_compositions: u64,
    pub hard_blackout_outputs: u64,
    /// History entries materialized to CPU from a validated GPU texture
    /// (layer readback) or by re-running a frozen recipe (historical replay).
    pub layer_readbacks: u64,
    pub historical_replays: u64,
    /// Fixed-clock CPU recoveries committed into the canonical history, and
    /// outputs retired with a fault.
    pub recoveries: u64,
    pub fault_retirements: u64,
    /// Final premapping snapshots copied and published as CPU LastValid with
    /// their Ready, and resolved outputs discarded with zero presents.
    pub snapshots_published: u64,
    pub ring_discards: u64,
    /// Physical Presented receipts by content. A CPU frame present uploads
    /// the full final frame; a resident present does not.
    pub resident_presents: u64,
    pub cpu_frame_uploads: u64,
    /// Resident presents whose receipt matched the frozen request, whose
    /// context and geometry are the session's current generation, and whose
    /// request and completed snapshot submission are both newer than those of
    /// the previous resident present.
    pub identity_verified_presents: u64,
    /// Resident presents of a Ready that is not newer than the previous one.
    pub stale_ready_presents: u64,
    pub last_presented_request_sequence: u64,
    /// Per executed GPU tick: logical layer entries issued (one per layer)
    /// and how composition selected each layer: this tick's entry, or an
    /// already validated resident history entry (canonical PTS reuse).
    pub logical_layers_issued: u64,
    pub logical_layers_selected_incoming: u64,
    pub logical_layers_selected_resident: u64,
    /// Resident presents prepared by the previous tick (snapshot ring).
    pub pipelined_presents: u64,
    /// Resident presents by ticks between the request that prepared the
    /// output and the tick that presented it: 0, 1 and 2 or more.
    pub present_lag_ticks: [u64; 3],
    /// Request issue (tick preparation start) to Presented receipt.
    pub render_to_present_bin_us: u64,
    pub render_to_present_bins: [u64; LATENCY_BINS],
    pub render_to_present_total_us: u64,
    pub render_to_present_max_us: u64,
    /// Presents whose request issue time was no longer retained.
    pub render_to_present_unknown: u64,
    /// High-water marks since session creation: outputs owned by the ring
    /// between ticks, and retained retirement owners.
    pub pending_outputs_high_water: u64,
    pub retirement_backlog_high_water: u64,
}

/// The session-owned recorder: counters plus the issue time of the two most
/// recent requests (the snapshot ring presents at most one tick late).
#[derive(Debug)]
pub(super) struct ResidentEvidence {
    counters: NativeResidentPathEvidence,
    issued: [Option<(u64, Instant)>; 2],
    last_snapshot_submission: u64,
}

impl Default for ResidentEvidence {
    fn default() -> Self {
        Self {
            counters: NativeResidentPathEvidence {
                render_to_present_bin_us: NATIVE_RESIDENT_LATENCY_BIN_US,
                ..NativeResidentPathEvidence::default()
            },
            issued: [None, None],
            last_snapshot_submission: 0,
        }
    }
}

impl ResidentEvidence {
    pub(super) fn counters(&self) -> NativeResidentPathEvidence {
        self.counters
    }

    pub(super) fn counters_mut(&mut self) -> &mut NativeResidentPathEvidence {
        &mut self.counters
    }

    pub(super) fn request_issued(&mut self, sequence: u64) {
        self.issued[(sequence % 2) as usize] = Some((sequence, Instant::now()));
    }

    pub(super) fn gpu_tick(&mut self, layers: usize, incoming: usize, resident: usize) {
        let c = &mut self.counters;
        c.logical_layers_issued += layers as u64;
        c.logical_layers_selected_incoming += incoming as u64;
        c.logical_layers_selected_resident += resident as u64;
    }

    pub(super) fn pending_outputs(&mut self, pending: usize, retirements: usize) {
        let c = &mut self.counters;
        c.pending_outputs_high_water = c.pending_outputs_high_water.max(pending as u64);
        c.retirement_backlog_high_water = c.retirement_backlog_high_water.max(retirements as u64);
    }

    /// One physical resident Presented receipt, already validated against
    /// its frozen request. `current_generation` is the caller's comparison of
    /// the request context/geometry with the session's current ones.
    pub(super) fn resident_presented(
        &mut self,
        request_sequence: u64,
        snapshot_submission: u64,
        current_request_sequence: u64,
        current_generation: bool,
        pipelined: bool,
    ) {
        let c = &mut self.counters;
        c.resident_presents += 1;
        c.pipelined_presents += u64::from(pipelined);
        let newer = request_sequence > c.last_presented_request_sequence
            && snapshot_submission > self.last_snapshot_submission;
        if newer && current_generation {
            c.identity_verified_presents += 1;
        }
        if request_sequence <= c.last_presented_request_sequence {
            c.stale_ready_presents += 1;
        }
        c.last_presented_request_sequence = c.last_presented_request_sequence.max(request_sequence);
        self.last_snapshot_submission = self.last_snapshot_submission.max(snapshot_submission);
        let lag = current_request_sequence.saturating_sub(request_sequence);
        c.present_lag_ticks[lag.min(2) as usize] += 1;
        let issued = self.issued[(request_sequence % 2) as usize]
            .filter(|(sequence, _)| *sequence == request_sequence)
            .map(|(_, at)| at.elapsed().as_micros().min(u128::from(u64::MAX)) as u64);
        // An unknown issue time is not a zero latency: it is counted on its
        // own and also lands in the overflow bin, which no bound can pass.
        let Some(micros) = issued else {
            c.render_to_present_unknown += 1;
            c.render_to_present_bins[LATENCY_BINS - 1] += 1;
            return;
        };
        let bin = (micros / NATIVE_RESIDENT_LATENCY_BIN_US).min(LATENCY_BINS as u64 - 1);
        c.render_to_present_bins[bin as usize] += 1;
        c.render_to_present_total_us = c.render_to_present_total_us.saturating_add(micros);
        c.render_to_present_max_us = c.render_to_present_max_us.max(micros);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_gpu_output_display_evidence_verifies_each_resident_present_identity_and_lag() {
        let mut evidence = ResidentEvidence::default();
        // Synchronous first frame, then two pipelined presents one tick late.
        evidence.request_issued(1);
        evidence.resident_presented(1, 10, 1, true, false);
        evidence.request_issued(2);
        evidence.request_issued(3);
        evidence.resident_presented(2, 11, 3, true, true);
        evidence.request_issued(4);
        evidence.resident_presented(3, 12, 4, true, true);
        let c = evidence.counters();
        assert_eq!((c.resident_presents, c.identity_verified_presents), (3, 3));
        assert_eq!((c.pipelined_presents, c.stale_ready_presents), (2, 0));
        assert_eq!(c.present_lag_ticks, [1, 2, 0]);
        assert_eq!(c.last_presented_request_sequence, 3);
        assert_eq!(c.render_to_present_unknown, 0);
        assert_eq!(c.render_to_present_bins.iter().sum::<u64>(), 3);
        assert_eq!(c.render_to_present_bin_us, NATIVE_RESIDENT_LATENCY_BIN_US);

        // A replayed old Ready, a stale snapshot and a foreign generation are
        // never identity-verified; a present two ticks late is visible.
        evidence.resident_presented(3, 13, 5, true, false);
        evidence.request_issued(5);
        evidence.request_issued(6);
        evidence.resident_presented(5, 12, 6, true, true);
        evidence.request_issued(7);
        evidence.resident_presented(7, 20, 7, false, false);
        evidence.request_issued(8);
        evidence.request_issued(9);
        evidence.request_issued(10);
        evidence.resident_presented(8, 21, 10, true, false);
        let c = evidence.counters();
        assert_eq!(c.resident_presents, 7);
        assert_eq!(c.identity_verified_presents, 4, "only request 8 is new");
        assert_eq!(c.stale_ready_presents, 1);
        assert_eq!(c.present_lag_ticks, [2, 3, 2]);
        // Request 8's issue time was overwritten by request 10: unknown, and
        // counted in the overflow bin instead of as a zero latency.
        assert_eq!(c.render_to_present_unknown, 1);
        assert_eq!(c.render_to_present_bins[LATENCY_BINS - 1], 1);
    }

    #[test]
    fn native_gpu_output_display_evidence_tracks_logical_layers_and_high_water() {
        let mut evidence = ResidentEvidence::default();
        evidence.gpu_tick(3, 3, 0);
        evidence.gpu_tick(3, 2, 1);
        evidence.pending_outputs(1, 0);
        evidence.pending_outputs(2, 3);
        evidence.pending_outputs(0, 1);
        let c = evidence.counters();
        assert_eq!(c.logical_layers_issued, 6);
        assert_eq!(
            (
                c.logical_layers_selected_incoming,
                c.logical_layers_selected_resident
            ),
            (5, 1)
        );
        assert_eq!(
            (
                c.pending_outputs_high_water,
                c.retirement_backlog_high_water
            ),
            (2, 3)
        );
    }
}
