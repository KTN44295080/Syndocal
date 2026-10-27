//! ADR-01 section 7.2: the resident route's asynchronous final-snapshot ring.
//!
//! Tick N submits output N and returns output N-1, whose final premapping
//! snapshot was copied while N's GPU work ran. Per output the order stays
//! "snapshot complete -> CPU LastValid -> Fresh Ready -> present"; only the
//! tick that observes it moves by one (one frame of latency).
//!
//! Slot lifecycle (at most one output per state, so two snapshot readbacks):
//! 1. `submitted`: output N after tick N; its G transaction may still execute
//!    and owns the logical slot. Tick N's physical present of N-1 runs beside
//!    it as G's auxiliary MappingPresent transaction, queued after N.
//! 2. `resolved`: at the start of tick N+1 the transaction drains and the ISF
//!    history commits (stage A, before the next history plan or quote). Only
//!    the snapshot copy/unmap and the cache publication remain.
//! 3. hand-off: after tick N+1 submits N+1, stage B copies N's snapshot,
//!    publishes it to the CPU cache and constructs its Ready (stage B), or the
//!    output is discarded with zero presents when its key, mapping, geometry,
//!    blackout or configuration fence no longer matches the current request.
//! 4. The unmapped readback returns to `free` for the next composition plan,
//!    so the ledger charges exactly two readbacks per geometry in steady state.
//!
//! Only Stop interrupts stages A/B: they complete committed work. A revoked
//! tick drains its own in-flight output before it reports Revoked, Stop
//! retires it with the session, and every non-resident tick (blackout, CPU,
//! Follow, recovery) drains the ring first and re-arms only after the next
//! synchronous resident success. No new thread, no speculative tick.

use super::present::ResidentDisplayContent;
use super::*;
use crate::native_gpu_composition::NativeChargedBuffer;
use crate::native_gpu_contract::{GpuContextId, GpuPhase, NativeGpuFault, NativeGpuFaultKind};
use crate::native_gpu_output::OutputRejection;

/// Two snapshot readbacks: one being copied by the CPU while the next output's
/// composition writes the other.
const RING_READBACKS: usize = 2;
const RING_DEADLINE: std::time::Duration = std::time::Duration::from_secs(5);

#[derive(Default)]
pub(super) struct ResidentSnapshotRing {
    submitted: Option<PendingSharedNativeOutput>,
    resolved: Option<PendingSharedNativeOutput>,
    free: Vec<Arc<NativeChargedBuffer>>,
    armed: bool,
    /// The caller ended a tick without settling it through this session
    /// (an app-level revocation or settlement): the in-flight output belongs
    /// to a tick that was never presented and is discarded, never published.
    abandoned: bool,
    #[cfg(test)]
    pub(super) discarded: u64,
}

impl ResidentSnapshotRing {
    /// The only logical output this session may leave between ticks.
    pub(super) fn owns_in_flight(&self) -> bool {
        self.submitted.is_some()
    }

    #[cfg(test)]
    pub(super) fn free_readbacks(&self) -> usize {
        self.free.len()
    }

    #[cfg(test)]
    pub(super) fn armed(&self) -> bool {
        self.armed
    }

    /// A completed readback of exactly `bytes` on `context`; foreign-generation
    /// or old-geometry buffers are dropped here (resize reallocation).
    pub(super) fn take_free(
        &mut self,
        context: GpuContextId,
        bytes: u64,
    ) -> Option<Arc<NativeChargedBuffer>> {
        while let Some(readback) = self.free.pop() {
            if readback.allocation_id().context == context
                && readback.capacity() == bytes
                && Arc::strong_count(&readback) == 1
            {
                return Some(readback);
            }
        }
        None
    }

    pub(super) fn recycle(&mut self, readback: Option<Arc<NativeChargedBuffer>>) {
        if let Some(readback) = readback {
            if self.free.len() < RING_READBACKS && Arc::strong_count(&readback) == 1 {
                self.free.push(readback);
            }
        }
    }

    pub(super) fn arm(&mut self) {
        self.armed = true;
    }

    pub(super) fn disarm(&mut self) {
        self.armed = false;
    }

    /// Leaving the resident route: idle readbacks are released so CPU,
    /// blackout or Follow ticks do not keep their charge.
    pub(super) fn release_idle(&mut self) {
        self.armed = false;
        self.free.clear();
    }

    /// Whether this tick may leave its output in flight. Only after one
    /// synchronous resident success, so a first frame is always presentable.
    pub(super) fn pipelined(&self) -> bool {
        self.armed && self.submitted.is_none()
    }

    pub(super) fn keep_submitted(&mut self, pending: PendingSharedNativeOutput) {
        debug_assert!(self.submitted.is_none(), "one in-flight ring output");
        self.submitted = Some(pending);
        self.abandoned = false;
    }
}

impl NativeResidentDisplaySession {
    /// Stage A: drain the previous tick's transaction and commit its history.
    /// Called before any history plan, quote, CPU handover or recovery of the
    /// current tick. Only Stop interrupts it (`committed` is Stop-only).
    pub(super) fn resolve_ring_submitted<P: VideoFrameProvider>(
        &mut self,
        renderer: &mut VideoPreviewRenderer<P>,
        snapshot: &protocol::VideoSnapshot,
        committed: &(impl VideoRenderCancellation + NativeResidentPresentationAuthority),
    ) -> Result<(), String> {
        let Some(mut pending) = self.ring.submitted.take() else {
            return Ok(());
        };
        #[cfg(feature = "qa-native-frame-timing")]
        let _wait = crate::native_frame_timing::span(crate::native_frame_timing::Phase::IsfMapWait);
        let authority = super::present::ResidentAuthority(committed);
        let request = pending.request().clone();
        let deadline = std::time::Instant::now() + RING_DEADLINE;
        let mut first = true;
        while !pending.phases_resolved() {
            if !first {
                if std::time::Instant::now() >= deadline {
                    let fault = self.ring_fault(
                        NativeGpuFaultKind::ScopeDrain,
                        "ring output completion deadline expired; complete owner retained",
                    );
                    let retirement = pending.retire(&mut self.owner, OutputRejection::Fault(fault));
                    return self.reject_ring_retirement(retirement);
                }
                std::thread::park_timeout(std::time::Duration::from_millis(1));
            }
            first = false;
            let Some(runtime) = renderer
                .isf_runtime
                .as_mut()
                .and_then(|runtime| runtime.as_mut().ok())
            else {
                let fault = self.ring_fault(
                    NativeGpuFaultKind::StateMismatch,
                    "ring output lost its shared ISF runtime",
                );
                let retirement = pending.retire(&mut self.owner, OutputRejection::Fault(fault));
                return self.reject_ring_retirement(retirement);
            };
            let Some(ResidentHistoryState::Active(history)) = self.history.as_mut() else {
                let fault = self.ring_fault(
                    NativeGpuFaultKind::StateMismatch,
                    "ring output lost its history owner",
                );
                let retirement = pending.retire(&mut self.owner, OutputRejection::Fault(fault));
                return self.reject_ring_retirement(retirement);
            };
            match pending.poll_phases_with_history(
                &mut self.owner,
                runtime,
                &mut self.cache,
                &mut history.session,
                &authority,
                committed,
            ) {
                SharedNativeOutputProgress::Pending(next) => pending = next,
                SharedNativeOutputProgress::Retiring(retirement)
                    if retirement.reason().is_none() =>
                {
                    // Fixed-clock CPU recovery of the previous output: commit
                    // its canonical history and faults. That output itself is
                    // not presented; the current tick continues from CPU
                    // history exactly as the tick after a synchronous recovery.
                    self.ring.armed = false;
                    self.commit_recovered_history(
                        renderer, retirement, &request, snapshot, &authority, committed,
                    )?;
                    return Ok(());
                }
                SharedNativeOutputProgress::Retiring(retirement) => {
                    return self.reject_ring_retirement(retirement)
                }
                SharedNativeOutputProgress::Rejected { pending, reason } => {
                    let retirement = pending.retire(&mut self.owner, reason);
                    return self.reject_ring_retirement(retirement);
                }
                SharedNativeOutputProgress::Ready { .. }
                | SharedNativeOutputProgress::HistoryReady { .. } => {
                    // Unreachable: the phases-only poll never publishes. The
                    // progress value already dropped; its transaction drained.
                    self.terminal_fault = Some("ring stage A published an output".into());
                    return Err("ring stage A published an output".into());
                }
            }
        }
        for retired in pending.take_history_retired() {
            let (entry, owner) = retired.into_parts();
            if owner.identity().entry != entry {
                let fault = "ring history retirement has a foreign entry".to_string();
                self.terminal_fault = Some(fault.clone());
                self.ring.resolved = Some(pending);
                self.discard_ring_resolved();
                return Err(fault);
            }
            drop(owner);
        }
        self.ring.resolved = Some(pending);
        Ok(())
    }

    /// Stage B: after the current tick submitted its own output, copy and
    /// publish the resolved predecessor and return its Ready. An output whose
    /// publication the current request no longer admits is discarded.
    pub(super) fn hand_off_ring_resolved<P: VideoFrameProvider>(
        &mut self,
        renderer: &mut VideoPreviewRenderer<P>,
        current: &NativeOutputRequest,
        committed: &(impl VideoRenderCancellation + NativeResidentPresentationAuthority),
    ) -> Result<Option<NativeResidentDisplayFrame>, String> {
        let Some(mut pending) = self.ring.resolved.take() else {
            return Ok(None);
        };
        let previous = pending.request();
        let publishable = !self.ring.abandoned
            && previous.context == current.context
            && previous.context == self.owner.context().id()
            && previous.key == current.key
            && previous.mapping_identity == current.mapping_identity
            && previous.frame.geometry_generation == current.frame.geometry_generation
            && previous.frame.geometry_generation == self.geometry_generation
            && previous.frame.request_sequence < current.frame.request_sequence
            && self.cache.accepts_publication(previous);
        if !publishable {
            self.ring.resolved = Some(pending);
            self.discard_ring_resolved();
            return Ok(None);
        }
        #[cfg(feature = "qa-native-frame-timing")]
        let _copy =
            crate::native_frame_timing::span(crate::native_frame_timing::Phase::IsfMappedCopy);
        let authority = super::present::ResidentAuthority(committed);
        let deadline = std::time::Instant::now() + RING_DEADLINE;
        let mut first = true;
        loop {
            if !first {
                if std::time::Instant::now() >= deadline {
                    let fault = self.ring_fault(
                        NativeGpuFaultKind::ScopeDrain,
                        "ring snapshot map deadline expired; complete owner retained",
                    );
                    let retirement = pending.retire(&mut self.owner, OutputRejection::Fault(fault));
                    return self.reject_ring_retirement(retirement);
                }
                std::thread::park_timeout(std::time::Duration::from_millis(1));
            }
            first = false;
            let Some(runtime) = renderer
                .isf_runtime
                .as_mut()
                .and_then(|runtime| runtime.as_mut().ok())
            else {
                let fault = self.ring_fault(
                    NativeGpuFaultKind::StateMismatch,
                    "ring hand-off lost its shared ISF runtime",
                );
                let retirement = pending.retire(&mut self.owner, OutputRejection::Fault(fault));
                return self.reject_ring_retirement(retirement);
            };
            let Some(ResidentHistoryState::Active(history)) = self.history.as_mut() else {
                let fault = self.ring_fault(
                    NativeGpuFaultKind::StateMismatch,
                    "ring hand-off lost its history owner",
                );
                let retirement = pending.retire(&mut self.owner, OutputRejection::Fault(fault));
                return self.reject_ring_retirement(retirement);
            };
            match pending.poll_with_history(
                &mut self.owner,
                runtime,
                &mut self.cache,
                &mut history.session,
                &authority,
                committed,
            ) {
                SharedNativeOutputProgress::Pending(next) => pending = next,
                SharedNativeOutputProgress::HistoryReady {
                    ready,
                    mut history,
                    readback,
                } => {
                    if !history.output_phases_valid() {
                        return Err("ring Ready lacks committed output history".into());
                    }
                    for retired in history.take_retired() {
                        let (entry, owner) = retired.into_parts();
                        if owner.identity().entry != entry {
                            return Err("ring Ready retirement has a foreign entry".into());
                        }
                        drop(owner);
                    }
                    self.ring.recycle(readback);
                    let mut frame = resident_display_frame(ready);
                    frame.pipelined = true;
                    return Ok(Some(frame));
                }
                SharedNativeOutputProgress::Ready { .. } => {
                    self.terminal_fault = Some("ring hand-off omitted its bound history".into());
                    return Err("ring hand-off omitted its bound history".into());
                }
                SharedNativeOutputProgress::Retiring(retirement) => {
                    return self.reject_ring_retirement(retirement)
                }
                SharedNativeOutputProgress::Rejected { pending, reason } => {
                    let retirement = pending.retire(&mut self.owner, reason);
                    return self.reject_ring_retirement(retirement);
                }
            }
        }
    }

    /// Zero-present discard of the resolved predecessor. Its transaction has
    /// drained and its history committed; only charged owners remain, which G
    /// reaps once they drop.
    pub(super) fn discard_ring_resolved(&mut self) {
        let Some(pending) = self.ring.resolved.take() else {
            return;
        };
        self.ring.abandoned = false;
        #[cfg(test)]
        {
            self.ring.discarded += 1;
        }
        if let Err(pending) = pending.discard_resolved() {
            // Not reachable: only resolved outputs enter this slot. Keep the
            // complete owner with the session's ordinary retirement.
            let retirement = pending.retire(&mut self.owner, OutputRejection::Cancelled);
            self.retirements.push(retirement);
        }
    }

    /// Drain the ring for a non-resident or revoked tick: stage A on the
    /// in-flight output, then a zero-present discard. Re-arms only after the
    /// next synchronous resident success.
    pub(super) fn drain_ring<P: VideoFrameProvider>(
        &mut self,
        renderer: &mut VideoPreviewRenderer<P>,
        snapshot: &protocol::VideoSnapshot,
        committed: &(impl VideoRenderCancellation + NativeResidentPresentationAuthority),
    ) -> Result<(), String> {
        self.ring.armed = false;
        let resolved = self.resolve_ring_submitted(renderer, snapshot, committed);
        self.discard_ring_resolved();
        resolved
    }

    /// The caller ended this tick without a session preparation (an app-level
    /// revocation, a disabled output or a settlement-only Follow tick). The
    /// in-flight output stays owned and drains on the next tick, but it is
    /// discarded there with zero presents: it belongs to an unpresented tick.
    pub fn abandon_pipelined_output(&mut self) {
        self.ring.armed = false;
        self.owner.release_resource_pools();
        if let Some(compositor) = &self.compositor {
            compositor.release_pooled_outputs();
        }
        if self.ring.submitted.is_some() {
            self.ring.abandoned = true;
        }
    }

    /// Whether a resident output prepared by the last tick is still in flight
    /// and will be returned (or discarded) by the next tick.
    pub fn has_pipelined_output(&self) -> bool {
        self.ring.owns_in_flight()
    }

    /// Stop: the in-flight output joins the session's retirement owner and is
    /// reaped with the generation; idle readbacks drop now so Reaped can reach
    /// zero allocations.
    pub(super) fn retire_ring_for_stop(&mut self) {
        self.ring.armed = false;
        self.discard_ring_resolved();
        if let Some(pending) = self.ring.submitted.take() {
            let retirement = pending.retire(&mut self.owner, OutputRejection::Cancelled);
            self.retirements.push(retirement);
        }
        self.ring.free.clear();
    }

    /// Re-arm builds a new generation: foreign readbacks are never reused.
    /// Returns whether no ring output remains (re-arm requires that).
    pub(super) fn reset_ring_for_rearm(&mut self) -> bool {
        self.ring.armed = false;
        self.ring.free.clear();
        self.ring.submitted.is_none() && self.ring.resolved.is_none()
    }

    fn reject_ring_retirement<T>(
        &mut self,
        retirement: SharedNativeOutputRetirement,
    ) -> Result<T, String> {
        self.ring.armed = false;
        self.reject_retiring_output(retirement)
    }

    fn ring_fault(&self, kind: NativeGpuFaultKind, message: &str) -> NativeGpuFault {
        NativeGpuFault {
            context: self.owner.context().id(),
            frame: None,
            phase: GpuPhase::Retirement,
            kind,
            message: message.into(),
        }
    }
}

pub(super) fn resident_display_frame(
    ready: crate::native_gpu_output::ReadyNativeGpuOutput<
        Arc<crate::native_gpu_snapshot::NativeSnapshotGpu>,
    >,
) -> NativeResidentDisplayFrame {
    let evidence = ready.evidence();
    let extent = (ready.request.key.width, ready.request.key.height);
    let pts_ms = ready.request.pts_ms;
    let output_mapping = ready.mapping().clone();
    NativeResidentDisplayFrame {
        content: ResidentDisplayContent::Resident(ready),
        evidence,
        extent,
        pts_ms,
        output_mapping,
        pipelined: false,
    }
}

/// The tick that leaves output N in flight with no predecessor to present.
/// Not Fresh and no pixels: the presenter rejects it before any surface work.
pub(super) fn in_flight_display_frame(
    request: &NativeOutputRequest,
    reason: &str,
) -> NativeResidentDisplayFrame {
    NativeResidentDisplayFrame {
        content: ResidentDisplayContent::InFlight,
        evidence: crate::VideoOutputRenderEvidence {
            project_render_epoch: request.frame.project_epoch,
            output_id: request.frame.output_id,
            freshness: crate::VideoOutputRenderFreshness::Error,
            error: Some(format!(
                "resident output {} is in flight; {reason}",
                request.frame.request_sequence
            )),
        },
        extent: (request.key.width, request.key.height),
        pts_ms: request.pts_ms,
        output_mapping: request.mapping.clone(),
        pipelined: false,
    }
}
