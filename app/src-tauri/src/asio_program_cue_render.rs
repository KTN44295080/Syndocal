//! Non-realtime PROGRAM/CUE mixer drain and the bounded ASIO callback fence.
//!
//! `rodio::mixer::MixerSource` is deliberately pulled here, never by the ASIO
//! callback: Rodio's mixer owns a mutex and dynamically managed source list.
//! The callback below touches only atomics, fixed queue operations and a copy
//! into the bridge-owned output slice.

use crate::asio_program_cue::{
    map_frame, AsioPreflightIdentity, AsioPreflightMapper, AsioPreflightState, CueFrame,
    MachineAsioOutputProfile, OutputProfileLock, PreflightSelectionKind, PreflightSolo,
    PreflightTarget,
};
use crossbeam_queue::ArrayQueue;
use engine::TimelineAudioLiveFence;
use std::sync::OnceLock;
use std::sync::{
    atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering},
    Arc, Mutex,
};
use std::time::Instant;

pub(crate) const CALLBACK_ACCEPTED: u32 = 0;
pub(crate) const CALLBACK_QUEUE_UNDERFLOW: u32 = 1;
pub(crate) const CALLBACK_TERMINAL: u32 = 3;
pub(crate) const CALLBACK_INVALID_BLOCK: u32 = 4;
pub(crate) const CALLBACK_PANIC: u32 = 5;
const MIN_RENDER_QUEUE_SLOTS: usize = 2;
const LIVE_DRAIN_CALLBACKS: u8 = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct LiveFenceObservation {
    word: u64,
    active: bool,
    fence_changed: bool,
    drain_current: bool,
}

/// Keeps Test/Solo admission closed for the whole callback that is consuming
/// a central live-transition drain slot.  The counter is decremented when the
/// callback observes the slot, while this stack guard covers the interval
/// until the callback has recycled its block and returned.
struct LiveDrainCallbackGuard<'a> {
    in_progress: &'a AtomicBool,
}

impl<'a> LiveDrainCallbackGuard<'a> {
    fn new(in_progress: &'a AtomicBool) -> Self {
        Self { in_progress }
    }
}

impl Drop for LiveDrainCallbackGuard<'_> {
    fn drop(&mut self) {
        self.in_progress.store(false, Ordering::Release);
    }
}

/// Shared non-realtime control for the ephemeral ASIO output preflight.
///
/// The state is owned by the application runtime and observed by the one
/// PROGRAM/CUE render worker.  The ASIO callback never receives this object:
/// it only owns the bounded render context below, so no callback path can
/// acquire this mutex or allocate from the preflight control.
pub(crate) struct PreflightRenderControl {
    state: Mutex<AsioPreflightState>,
    mapper: AsioPreflightMapper,
    selection_epoch: Arc<AtomicU64>,
    selection_epoch_exhausted: Arc<AtomicBool>,
    selection_kind: Arc<AtomicU8>,
    selection_signature: Arc<AtomicU8>,
    timeline_audio_live_fence: TimelineAudioLiveFence,
    live_drain_remaining: Arc<AtomicU8>,
    live_drain_in_progress: Arc<AtomicBool>,
}

impl PreflightRenderControl {
    pub(crate) fn new(
        profile: &MachineAsioOutputProfile,
        identity: AsioPreflightIdentity,
        selection_epoch: Arc<AtomicU64>,
        selection_epoch_exhausted: Arc<AtomicBool>,
        selection_kind: Arc<AtomicU8>,
        selection_signature: Arc<AtomicU8>,
        timeline_audio_live_fence: TimelineAudioLiveFence,
        live_drain_remaining: Arc<AtomicU8>,
        live_drain_in_progress: Arc<AtomicBool>,
    ) -> Result<Arc<Self>, OutputProfileLock> {
        let mapper = AsioPreflightMapper::new(profile)?;
        Ok(Arc::new(Self {
            state: Mutex::new(AsioPreflightState::from_identity(identity)),
            mapper,
            selection_epoch,
            selection_epoch_exhausted,
            selection_kind,
            selection_signature,
            timeline_audio_live_fence,
            live_drain_remaining,
            live_drain_in_progress,
        }))
    }

    pub(crate) fn state(&self) -> &Mutex<AsioPreflightState> {
        &self.state
    }

    pub(crate) fn mapper(&self) -> &AsioPreflightMapper {
        &self.mapper
    }

    /// Return the callback-visible epoch for Test/Solo-bearing render blocks.
    /// Epoch zero is reserved for ordinary PROGRAM/CUE blocks.
    pub(crate) fn selection_epoch(&self) -> u64 {
        self.selection_epoch.load(Ordering::Acquire)
    }

    #[cfg(test)]
    pub(crate) fn test_epoch(&self) -> u64 {
        self.selection_epoch()
    }

    pub(crate) fn selection_signature(&self) -> u8 {
        self.selection_signature.load(Ordering::Acquire)
    }

    pub(crate) fn timeline_audio_live_fence(&self) -> &TimelineAudioLiveFence {
        &self.timeline_audio_live_fence
    }

    /// Return whether the callback is currently consuming the bounded central
    /// live-transition drain. The runtime reads this only while holding the
    /// existing worker-only preflight state mutex before admitting Test/Solo.
    pub(crate) fn live_drain_active(&self) -> bool {
        self.live_drain_remaining.load(Ordering::Acquire) != 0
            || self.live_drain_in_progress.load(Ordering::Acquire)
    }

    /// Publish the callback-visible Test/Solo classification.  Every change
    /// advances the epoch before publishing the new kind/signature. This
    /// ordering makes an old block fail closed even when the callback observes
    /// the transition halfway through its publication.
    pub(crate) fn publish_selection_transition(
        &self,
        state: &AsioPreflightState,
        previous_signature: u8,
    ) -> Result<bool, RenderFault> {
        let current = state.selection_kind();
        let current_signature = state.selection_signature();
        if current_signature == previous_signature {
            return Ok(false);
        }
        if let Err(fault) = self.advance_selection_epoch() {
            self.selection_kind
                .store(PreflightSelectionKind::None as u8, Ordering::Release);
            self.selection_signature.store(0, Ordering::Release);
            return Err(fault);
        }
        if current == PreflightSelectionKind::None {
            self.selection_kind.store(current as u8, Ordering::Release);
            self.selection_signature
                .store(current_signature, Ordering::Release);
        } else {
            self.selection_kind.store(current as u8, Ordering::Release);
            self.selection_signature
                .store(current_signature, Ordering::Release);
        }
        Ok(true)
    }

    pub(crate) fn publish_selection_state(&self, state: &AsioPreflightState) {
        self.selection_kind
            .store(state.selection_kind() as u8, Ordering::Release);
        self.selection_signature
            .store(state.selection_signature(), Ordering::Release);
    }

    pub(crate) fn publish_selection_kind(&self, state: &AsioPreflightState) {
        self.publish_selection_state(state);
    }

    /// Fence queued Test/Solo blocks when the authoritative state mutex can
    /// no longer be inspected (for example during poisoned Stop cleanup).
    /// Advancing the epoch first makes an in-flight callback fail its post-copy
    /// check; clearing kind/signature then prevents any later admission. The
    /// caller must also latch the render context terminally for the poisoned
    /// state itself.
    pub(crate) fn invalidate_selection_without_state(&self) -> Result<(), RenderFault> {
        let result = self.advance_selection_epoch();
        self.selection_kind
            .store(PreflightSelectionKind::None as u8, Ordering::Release);
        self.selection_signature.store(0, Ordering::Release);
        result
    }

    /// Invalidate already-queued Test/Solo-bearing blocks without touching ordinary
    /// PROGRAM/CUE blocks.  Epoch zero is reserved for ordinary blocks, so a
    /// counter exhaustion permanently publishes zero and fails closed instead
    /// of wrapping to a value that an ancient Test block could reuse.
    pub(crate) fn advance_selection_epoch(&self) -> Result<(), RenderFault> {
        if self.selection_epoch_exhausted.load(Ordering::Acquire)
            || matches!(self.selection_epoch.load(Ordering::Acquire), 0 | u64::MAX)
        {
            self.selection_epoch_exhausted
                .store(true, Ordering::Release);
            self.selection_epoch.store(0, Ordering::Release);
            return Err(RenderFault::PreflightEpochExhausted);
        }
        self.selection_epoch
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |epoch| {
                if matches!(epoch, 0 | u64::MAX) {
                    None
                } else {
                    epoch.checked_add(1)
                }
            })
            .map(|_| ())
            .map_err(|_| {
                self.selection_epoch_exhausted
                    .store(true, Ordering::Release);
                self.selection_epoch.store(0, Ordering::Release);
                RenderFault::PreflightEpochExhausted
            })
    }

    #[cfg(test)]
    pub(crate) fn advance_test_epoch(&self) -> Result<(), RenderFault> {
        self.advance_selection_epoch()
    }

    pub(crate) fn selection_epoch_exhausted(&self) -> bool {
        self.selection_epoch_exhausted.load(Ordering::Acquire)
            || self.selection_epoch.load(Ordering::Acquire) == 0
    }

    /// Clear selections after a caller supplied render clock moves backwards.
    ///
    /// The production clock is monotonic, but the deterministic worker seam
    /// accepts an explicit clock so it can be tested without sleeping.  A
    /// rollback is therefore treated as an input anomaly: retire the test and
    /// solo transactionally, advance only the test fence, and keep ordinary
    /// PROGRAM/CUE blocks admissible.
    pub(crate) fn clear_for_clock_anomaly(
        &self,
        state: &mut AsioPreflightState,
        identity: AsioPreflightIdentity,
        now_ms: u64,
    ) -> Result<bool, RenderFault> {
        let mut changed = state
            .clear_if_expired(identity, now_ms)
            .map_err(|_| RenderFault::InvalidBlock)?;
        if state
            .test_target(identity, now_ms)
            .map_err(|_| RenderFault::InvalidBlock)?
            .is_some()
        {
            state
                .begin_test_with_mapper(&self.mapper, PreflightTarget::Off, identity, now_ms, 1)
                .map_err(|_| RenderFault::InvalidBlock)?;
            changed = true;
        }
        if state.solo() != PreflightSolo::None {
            state
                .set_solo(PreflightSolo::None, identity, now_ms)
                .map_err(|_| RenderFault::InvalidBlock)?;
            changed = true;
        }
        if changed {
            self.advance_selection_epoch()?;
        }
        Ok(changed)
    }
}

static PREFLIGHT_CLOCK_ORIGIN: OnceLock<Instant> = OnceLock::new();

/// Return the monotonic elapsed-time domain used by the ephemeral preflight
/// expiry contract.  This is intentionally not Unix wall time: a system clock
/// rollback can never make an armed test live longer than its duration.
pub(crate) fn preflight_now_ms() -> u64 {
    u64::try_from(
        PREFLIGHT_CLOCK_ORIGIN
            .get_or_init(Instant::now)
            .elapsed()
            .as_millis(),
    )
    .unwrap_or(u64::MAX)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RenderFault {
    QueueUnderflow,
    InvalidBlock,
    QueueOwnership,
    GenerationExhausted,
    PreflightEpochExhausted,
    CallbackPanic,
}

impl RenderFault {
    pub(crate) fn code(self) -> &'static str {
        match self {
            Self::QueueUnderflow => "output_queue_underflow",
            Self::InvalidBlock => "invalid_output_block",
            Self::QueueOwnership => "output_queue_ownership",
            Self::GenerationExhausted => "transport_generation_exhausted",
            Self::PreflightEpochExhausted => "preflight_epoch_exhausted",
            Self::CallbackPanic => "output_callback_panic",
        }
    }
}

/// A complete preallocated device-width block.  It moves between the worker
/// and callback queues but is never allocated/resized by either callback path.
struct RenderBlock {
    session_generation: u64,
    transport_generation: u64,
    first_output_frame: u64,
    frames: u32,
    /// Zero means this block contains ordinary PROGRAM/CUE output. A nonzero
    /// value identifies the Test/Solo selection epoch that produced its
    /// samples.
    preflight_selection_epoch: u64,
    /// The packed TimelineAudioLiveFence word captured by the non-realtime
    /// worker for an affected Test/Solo block. Ordinary blocks keep zero.
    preflight_live_fence_word: u64,
    /// The callback-independent signature captured for an affected
    /// Test/Solo block. Ordinary blocks keep zero. The epoch distinguishes
    /// most transitions; this extra identity closes the same-kind Solo mode
    /// publication window before the epoch store becomes visible.
    preflight_selection_signature: u8,
    preflight_selection_kind: PreflightSelectionKind,
    samples: Box<[f32]>,
}

impl RenderBlock {
    fn clear_metadata(&mut self) {
        self.session_generation = 0;
        self.transport_generation = 0;
        self.first_output_frame = 0;
        self.frames = 0;
        self.preflight_selection_epoch = 0;
        self.preflight_live_fence_word = 0;
        self.preflight_selection_signature = 0;
        self.preflight_selection_kind = PreflightSelectionKind::None;
    }
}

#[cfg(test)]
struct CallbackPostCopyGate {
    enabled: AtomicBool,
    reached: AtomicBool,
    release: AtomicBool,
}

/// The app-owned mutable generation is deliberately independent of the fixed
/// v3 `Start.renderGeneration`.  Any ordinary transport/project/bus revision
/// rotates this atomic and drops all queued old-generation blocks without an
/// ASIO device restart.
pub(crate) struct TransportGeneration {
    current: AtomicU64,
    faulted: AtomicBool,
}

impl TransportGeneration {
    pub(crate) fn new(initial: u64) -> Result<Self, RenderFault> {
        if initial == 0 {
            return Err(RenderFault::GenerationExhausted);
        }
        Ok(Self {
            current: AtomicU64::new(initial),
            faulted: AtomicBool::new(false),
        })
    }

    pub(crate) fn current(&self) -> u64 {
        self.current.load(Ordering::Acquire)
    }

    pub(crate) fn rotate(&self) -> Result<u64, RenderFault> {
        let mut observed = self.current();
        loop {
            let Some(next) = observed.checked_add(1) else {
                self.faulted.store(true, Ordering::Release);
                return Err(RenderFault::GenerationExhausted);
            };
            match self.current.compare_exchange_weak(
                observed,
                next,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => return Ok(next),
                Err(actual) => observed = actual,
            }
        }
    }

    fn faulted(&self) -> bool {
        self.faulted.load(Ordering::Acquire)
    }
}

pub(crate) struct AsioRenderContext {
    session_generation: u64,
    output_channels: u32,
    fixed_buffer_frames: u32,
    ready: Arc<ArrayQueue<RenderBlock>>,
    free: Arc<ArrayQueue<RenderBlock>>,
    transport: Arc<TransportGeneration>,
    preflight_selection_epoch: Arc<AtomicU64>,
    preflight_selection_epoch_exhausted: Arc<AtomicBool>,
    preflight_selection_kind: Arc<AtomicU8>,
    preflight_selection_signature: Arc<AtomicU8>,
    timeline_audio_live_fence: TimelineAudioLiveFence,
    live_fence_observed_word: AtomicU64,
    live_drain_remaining: Arc<AtomicU8>,
    live_drain_in_progress: Arc<AtomicBool>,
    terminal: AtomicBool,
    fault: AtomicU64,
    #[cfg(test)]
    post_copy_gate: Arc<CallbackPostCopyGate>,
}

impl AsioRenderContext {
    #[cfg(test)]
    pub(crate) fn new(
        session_generation: u64,
        profile: &MachineAsioOutputProfile,
        queue_slots: usize,
        transport: Arc<TransportGeneration>,
    ) -> Result<Arc<Self>, OutputProfileLock> {
        Self::new_with_live_fence(
            session_generation,
            profile,
            queue_slots,
            transport,
            TimelineAudioLiveFence::default(),
        )
    }

    pub(crate) fn new_with_live_fence(
        session_generation: u64,
        profile: &MachineAsioOutputProfile,
        queue_slots: usize,
        transport: Arc<TransportGeneration>,
        timeline_audio_live_fence: TimelineAudioLiveFence,
    ) -> Result<Arc<Self>, OutputProfileLock> {
        profile.validate()?;
        if session_generation == 0 || queue_slots < MIN_RENDER_QUEUE_SLOTS {
            return Err(OutputProfileLock::Invalid(
                "ASIO render context requires a non-zero session generation and at least two queue slots"
                    .to_owned(),
            ));
        }
        let samples = (profile.fixed_buffer_frames() as usize)
            .checked_mul(profile.device_output_channels() as usize)
            .ok_or_else(|| {
                OutputProfileLock::Invalid("ASIO device block byte width overflowed".to_owned())
            })?;
        if samples == 0 {
            return Err(OutputProfileLock::Invalid(
                "ASIO device block must contain at least one sample".to_owned(),
            ));
        }
        let ready = Arc::new(ArrayQueue::new(queue_slots));
        let free = Arc::new(ArrayQueue::new(queue_slots));
        for _ in 0..queue_slots {
            // Allocation is admitted only before Start.  The callback merely
            // transfers these boxes between bounded queues.
            let block = RenderBlock {
                session_generation: 0,
                transport_generation: 0,
                first_output_frame: 0,
                frames: 0,
                preflight_selection_epoch: 0,
                preflight_live_fence_word: 0,
                preflight_selection_signature: 0,
                preflight_selection_kind: PreflightSelectionKind::None,
                samples: vec![0.0; samples].into_boxed_slice(),
            };
            free.push(block).map_err(|_| {
                OutputProfileLock::Invalid("ASIO render free queue prefill failed".to_owned())
            })?;
        }
        Ok(Arc::new(Self {
            session_generation,
            output_channels: profile.device_output_channels(),
            fixed_buffer_frames: profile.fixed_buffer_frames(),
            ready,
            free,
            transport,
            preflight_selection_epoch: Arc::new(AtomicU64::new(1)),
            preflight_selection_epoch_exhausted: Arc::new(AtomicBool::new(false)),
            preflight_selection_kind: Arc::new(AtomicU8::new(0)),
            preflight_selection_signature: Arc::new(AtomicU8::new(0)),
            timeline_audio_live_fence,
            live_fence_observed_word: AtomicU64::new(0),
            live_drain_remaining: Arc::new(AtomicU8::new(0)),
            live_drain_in_progress: Arc::new(AtomicBool::new(false)),
            terminal: AtomicBool::new(false),
            fault: AtomicU64::new(0),
            #[cfg(test)]
            post_copy_gate: Arc::new(CallbackPostCopyGate {
                enabled: AtomicBool::new(false),
                reached: AtomicBool::new(false),
                release: AtomicBool::new(true),
            }),
        }))
    }

    pub(crate) fn transport(&self) -> &Arc<TransportGeneration> {
        &self.transport
    }

    pub(crate) fn preflight_selection_epoch(&self) -> &Arc<AtomicU64> {
        &self.preflight_selection_epoch
    }

    #[cfg(test)]
    pub(crate) fn preflight_test_epoch(&self) -> &Arc<AtomicU64> {
        self.preflight_selection_epoch()
    }

    pub(crate) fn preflight_selection_epoch_exhausted(&self) -> &Arc<AtomicBool> {
        &self.preflight_selection_epoch_exhausted
    }

    pub(crate) fn preflight_selection_kind(&self) -> &Arc<AtomicU8> {
        &self.preflight_selection_kind
    }

    pub(crate) fn preflight_selection_signature(&self) -> &Arc<AtomicU8> {
        &self.preflight_selection_signature
    }

    pub(crate) fn timeline_audio_live_fence(&self) -> &TimelineAudioLiveFence {
        &self.timeline_audio_live_fence
    }

    pub(crate) fn live_drain_remaining(&self) -> &Arc<AtomicU8> {
        &self.live_drain_remaining
    }

    pub(crate) fn live_drain_in_progress(&self) -> &Arc<AtomicBool> {
        &self.live_drain_in_progress
    }

    /// Observe the packed engine liveness word and consume one bounded drain
    /// slot when requested. This is callback-only state: the callback is the
    /// sole writer, while the worker/runtime only read the published drain
    /// count. No mutex, allocation, or blocking operation is permitted here.
    fn observe_live_fence(&self, consume_drain: bool) -> Result<LiveFenceObservation, RenderFault> {
        let word = self.timeline_audio_live_fence.word();
        self.observe_live_fence_word(word, consume_drain)
    }

    fn observe_live_fence_word(
        &self,
        word: u64,
        consume_drain: bool,
    ) -> Result<LiveFenceObservation, RenderFault> {
        let active = word & 1 == 1;
        let previous = self.live_fence_observed_word.load(Ordering::Acquire);
        let epoch = word >> 1;
        let previous_epoch = previous >> 1;
        if word <= 1 || word >= u64::MAX - 1 || (previous != 0 && epoch < previous_epoch) {
            self.latch(RenderFault::GenerationExhausted);
            return Err(RenderFault::GenerationExhausted);
        }

        let previous_active = previous != 0 && previous & 1 == 1;
        let fence_changed = previous != 0 && word != previous;
        // An inactive word with a newer epoch proves that an active
        // generation existed even when both enter and exit completed between
        // callback observations. It must arm the same two-callback physical
        // drain as a directly observed active word; otherwise an already
        // returned A/B tail can be followed by ordinary audio immediately.
        let generation_advanced = previous != 0 && epoch > previous_epoch;
        let entered = if active {
            if previous == 0 {
                true
            } else if !previous_active && epoch == previous_epoch {
                // The engine fence must advance its epoch on every new enter.
                // Seeing an active word in the already-observed inactive epoch
                // is an ABA/replay condition, so stop rather than admit it.
                self.latch(RenderFault::GenerationExhausted);
                return Err(RenderFault::GenerationExhausted);
            } else {
                epoch > previous_epoch
            }
        } else {
            false
        };

        self.live_fence_observed_word.store(word, Ordering::Release);
        if (active && entered) || generation_advanced {
            self.live_drain_remaining
                .store(LIVE_DRAIN_CALLBACKS, Ordering::Release);
        }

        let drain_current = if consume_drain {
            let remaining = self.live_drain_remaining.load(Ordering::Acquire);
            if remaining == 0 {
                false
            } else {
                self.live_drain_in_progress.store(true, Ordering::Release);
                self.live_drain_remaining
                    .store(remaining.saturating_sub(1), Ordering::Release);
                true
            }
        } else {
            false
        };

        Ok(LiveFenceObservation {
            word,
            active,
            fence_changed,
            drain_current,
        })
    }

    pub(crate) fn terminal_fault(&self) -> Option<RenderFault> {
        match self.fault.load(Ordering::Acquire) {
            0 => None,
            1 => Some(RenderFault::QueueUnderflow),
            2 => Some(RenderFault::InvalidBlock),
            3 => Some(RenderFault::QueueOwnership),
            4 => Some(RenderFault::GenerationExhausted),
            5 => Some(RenderFault::CallbackPanic),
            6 => Some(RenderFault::PreflightEpochExhausted),
            _ => Some(RenderFault::InvalidBlock),
        }
    }

    pub(crate) fn latch_worker_fault(&self, fault: RenderFault) {
        self.latch(fault);
    }

    fn latch(&self, fault: RenderFault) {
        self.terminal.store(true, Ordering::Release);
        let encoded = match fault {
            RenderFault::QueueUnderflow => 1,
            RenderFault::InvalidBlock => 2,
            RenderFault::QueueOwnership => 3,
            RenderFault::GenerationExhausted => 4,
            RenderFault::CallbackPanic => 5,
            RenderFault::PreflightEpochExhausted => 6,
        };
        let _ = self
            .fault
            .compare_exchange(0, encoded, Ordering::AcqRel, Ordering::Acquire);
    }

    fn recycle_from_callback(&self, mut block: RenderBlock) {
        block.clear_metadata();
        // With one callback producer and the invariant that each slot is in
        // exactly one queue, this cannot be full after the callback popped a
        // ready slot.  A violation must not deallocate from the RT path.
        if let Err(block) = self.free.push(block) {
            std::mem::forget(block);
            self.latch(RenderFault::QueueOwnership);
        }
    }

    /// Callback-only.  No locks, allocation, decoder, Rodio, filesystem or UI
    /// work appears on this path.  Any mismatch writes an entire silent block
    /// and returns a terminal result for the bridge.
    pub(crate) unsafe fn callback_copy(
        &self,
        destination: *mut f32,
        output_channels: u32,
        frames: u32,
        first_output_frame: u64,
        session_generation: u64,
        render_generation: u64,
    ) -> u32 {
        let sample_len = match (output_channels as usize).checked_mul(frames as usize) {
            Some(value) if !destination.is_null() => value,
            _ => {
                self.latch(RenderFault::InvalidBlock);
                return CALLBACK_INVALID_BLOCK;
            }
        };
        let destination = unsafe { std::slice::from_raw_parts_mut(destination, sample_len) };
        destination.fill(0.0);
        if self.terminal.load(Ordering::Acquire) || self.transport.faulted() {
            self.latch(RenderFault::GenerationExhausted);
            return CALLBACK_TERMINAL;
        }
        if output_channels != self.output_channels
            || frames != self.fixed_buffer_frames
            || session_generation != self.session_generation
            || render_generation != self.session_generation
        {
            self.latch(RenderFault::InvalidBlock);
            return CALLBACK_INVALID_BLOCK;
        }
        // The callback is the sole writer of this bounded drain state.  Read
        // the packed engine word before taking a ready block so a newly
        // observed active generation silences the complete callback, not just
        // the affected Test/Solo samples.  This is deliberately lock/alloc
        // free; engine publication never waits for this path.
        let live_observation = match self.observe_live_fence(true) {
            Ok(observation) => observation,
            Err(_) => return CALLBACK_TERMINAL,
        };
        let _live_drain_guard = LiveDrainCallbackGuard::new(&self.live_drain_in_progress);
        let Some(block) = self.ready.pop() else {
            self.latch(RenderFault::QueueUnderflow);
            return CALLBACK_QUEUE_UNDERFLOW;
        };
        let expected_transport = self.transport.current();
        let exact_block = block.session_generation == self.session_generation
            && block.first_output_frame == first_output_frame
            && block.frames == frames
            && block.samples.len() == sample_len
            && block.samples.iter().all(|sample| sample.is_finite());
        let transport_matches = block.transport_generation == expected_transport;
        let expected_selection_epoch = self.preflight_selection_epoch.load(Ordering::Acquire);
        let selection_epoch_exhausted = self
            .preflight_selection_epoch_exhausted
            .load(Ordering::Acquire)
            || expected_selection_epoch == 0;
        let expected_selection_kind = match self.preflight_selection_kind.load(Ordering::Acquire) {
            1 => PreflightSelectionKind::Test,
            2 => PreflightSelectionKind::Solo,
            _ => PreflightSelectionKind::None,
        };
        let expected_selection_signature =
            self.preflight_selection_signature.load(Ordering::Acquire);
        let expected_live_fence_word = live_observation.word;
        let live_fence_active = live_observation.active;
        let selection_matches = if block.preflight_selection_epoch == 0 {
            // A normal block can continue across a preflight-only retirement,
            // but never after Test/Solo has become current. This closes the
            // race where an operator arms a selection while the callback is
            // copying a previously queued ordinary block.
            expected_selection_kind == PreflightSelectionKind::None
        } else {
            !selection_epoch_exhausted
                && block.preflight_selection_epoch == expected_selection_epoch
                && block.preflight_selection_kind == expected_selection_kind
                && block.preflight_selection_signature == expected_selection_signature
                && block.preflight_live_fence_word == expected_live_fence_word
                && !live_fence_active
        };
        if !exact_block {
            self.recycle_from_callback(block);
            self.latch(RenderFault::InvalidBlock);
            return CALLBACK_INVALID_BLOCK;
        }
        if live_observation.drain_current {
            // The destination was pre-silenced above.  Consume this complete
            // block even when it is ordinary PROGRAM/CUE output: the two
            // callbacks immediately following a newly observed live entry
            // are a physical A/B drain, not a logical selection fence.
            self.recycle_from_callback(block);
            return CALLBACK_ACCEPTED;
        }
        if transport_matches && selection_matches {
            destination.copy_from_slice(&block.samples);
            #[cfg(test)]
            if self.post_copy_gate.enabled.load(Ordering::Acquire) {
                self.post_copy_gate.reached.store(true, Ordering::Release);
                while !self.post_copy_gate.release.load(Ordering::Acquire) {
                    std::hint::spin_loop();
                }
                self.post_copy_gate.enabled.store(false, Ordering::Release);
            }
            // A live transition can occur while the destination is being
            // copied.  Observe it once more after the copy.  The pre-copy
            // observation already consumed this callback's drain slot when
            // the word was unchanged; consume only when the packed word
            // changed during the copy, so one callback can never spend both
            // drain slots.
            let post_live_word = self.timeline_audio_live_fence.word();
            let post_live_observation = match self
                .observe_live_fence_word(post_live_word, post_live_word != expected_live_fence_word)
            {
                Ok(observation) => observation,
                Err(_) => {
                    destination.fill(0.0);
                    self.recycle_from_callback(block);
                    return CALLBACK_TERMINAL;
                }
            };
            let live_transition_during_copy = post_live_observation.fence_changed
                || post_live_observation.word != expected_live_fence_word
                || post_live_observation.drain_current;
            // Rotation or a successful Test Off/expiry may race this copy.
            // Re-read both lock-free fences immediately after copying; a stale
            // test/transport block is converted to silence, while an ordinary
            // PROGRAM/CUE block remains valid across a preflight-only change.
            let session_still_current = block.session_generation == self.session_generation;
            let transport_still_current = self.transport.current() == expected_transport;
            let selection_still_current = if block.preflight_selection_epoch == 0 {
                self.preflight_selection_kind.load(Ordering::Acquire) == 0
            } else {
                let current_epoch = self.preflight_selection_epoch.load(Ordering::Acquire);
                let current_kind = self.preflight_selection_kind.load(Ordering::Acquire);
                let current_signature = self.preflight_selection_signature.load(Ordering::Acquire);
                !self
                    .preflight_selection_epoch_exhausted
                    .load(Ordering::Acquire)
                    && current_epoch != 0
                    && current_epoch == expected_selection_epoch
                    && current_kind == expected_selection_kind as u8
                    && block.preflight_selection_epoch == current_epoch
                    && block.preflight_selection_kind as u8 == current_kind
                    && block.preflight_selection_signature == current_signature
                    && block.preflight_live_fence_word == post_live_observation.word
                    && !post_live_observation.active
            };
            if live_transition_during_copy
                || !session_still_current
                || !transport_still_current
                || !selection_still_current
            {
                destination.fill(0.0);
            }
            self.recycle_from_callback(block);
            CALLBACK_ACCEPTED
        } else if !transport_matches || !selection_matches {
            // This is an expected generation/fence barrier, not a malformed
            // block. The destination was already silenced above; consuming the
            // stale block keeps the bounded queues live for the next block.
            self.recycle_from_callback(block);
            CALLBACK_ACCEPTED
        } else {
            self.recycle_from_callback(block);
            self.latch(RenderFault::InvalidBlock);
            CALLBACK_INVALID_BLOCK
        }
    }

    fn take_free(&self) -> Result<RenderBlock, RenderFault> {
        self.free.pop().ok_or(RenderFault::QueueUnderflow)
    }

    fn publish_ready(&self, block: RenderBlock) -> Result<(), RenderFault> {
        self.ready.push(block).map_err(|block| {
            // The worker is the sole ready producer.  Full means the callback
            // cannot keep up, which is terminal rather than an overwrite.
            std::mem::forget(block);
            self.latch(RenderFault::QueueOwnership);
            RenderFault::QueueOwnership
        })
    }

    pub(crate) fn retire_queued_blocks(&self) {
        while let Some(block) = self.ready.pop() {
            self.recycle_from_callback(block);
        }
    }

    #[cfg(test)]
    fn ready_len(&self) -> usize {
        self.ready.len()
    }
}

/// A non-realtime render worker.  It owns Rodio's two stereo mixer sources;
/// generated Click/Guide must be added to `cue_mixer`, never to a second OS
/// stream or to the PROGRAM mixer.
pub(crate) struct ProgramCueRenderWorker {
    context: Arc<AsioRenderContext>,
    profile: MachineAsioOutputProfile,
    preflight: Arc<PreflightRenderControl>,
    program_source: rodio::mixer::MixerSource,
    cue_source: rodio::mixer::MixerSource,
    next_output_frame: u64,
    last_now_ms: Option<u64>,
    last_engine_live_fence_word: Option<u64>,
}

pub(crate) struct ProgramCueMixers {
    pub(crate) program: rodio::mixer::Mixer,
    pub(crate) cue: rodio::mixer::Mixer,
    worker: ProgramCueRenderWorker,
}

impl ProgramCueMixers {
    pub(crate) fn new(
        context: Arc<AsioRenderContext>,
        profile: MachineAsioOutputProfile,
        first_output_frame: u64,
    ) -> Result<Self, OutputProfileLock> {
        if profile.sample_rate_hz() != 48_000 {
            return Err(OutputProfileLock::Invalid(
                "show-ASIO PROGRAM/CUE renderer requires the explicitly selected 48000 Hz profile"
                    .to_owned(),
            ));
        }
        let identity =
            AsioPreflightIdentity::new(context.session_generation, context.transport.current())
                .map_err(|error| OutputProfileLock::Invalid(error.to_string()))?;
        let preflight = PreflightRenderControl::new(
            &profile,
            identity,
            Arc::clone(context.preflight_selection_epoch()),
            Arc::clone(context.preflight_selection_epoch_exhausted()),
            Arc::clone(context.preflight_selection_kind()),
            Arc::clone(context.preflight_selection_signature()),
            context.timeline_audio_live_fence().clone(),
            Arc::clone(context.live_drain_remaining()),
            Arc::clone(context.live_drain_in_progress()),
        )?;
        let (program, program_source) = rodio::mixer::mixer(2, profile.sample_rate_hz());
        let (cue, cue_source) = rodio::mixer::mixer(2, profile.sample_rate_hz());
        let initial_engine_live_fence_word = context.timeline_audio_live_fence().word();
        Ok(Self {
            worker: ProgramCueRenderWorker {
                context,
                profile,
                preflight,
                program_source,
                cue_source,
                next_output_frame: first_output_frame,
                last_now_ms: None,
                last_engine_live_fence_word: Some(initial_engine_live_fence_word),
            },
            program,
            cue,
        })
    }

    pub(crate) fn worker_mut(&mut self) -> &mut ProgramCueRenderWorker {
        &mut self.worker
    }

    pub(crate) fn preflight_control(&self) -> Arc<PreflightRenderControl> {
        Arc::clone(&self.worker.preflight)
    }

    pub(crate) fn into_parts(
        self,
    ) -> (
        rodio::mixer::Mixer,
        rodio::mixer::Mixer,
        ProgramCueRenderWorker,
    ) {
        (self.program, self.cue, self.worker)
    }
}

impl ProgramCueRenderWorker {
    pub(crate) fn render_one_complete_block(&mut self) -> Result<(), RenderFault> {
        self.render_one_complete_block_at(preflight_now_ms())
    }

    /// Deterministic variant used by focused tests.  Production callers use
    /// `render_one_complete_block`, which supplies the current preflight clock
    /// without adding a clock or allocation to the ASIO callback.
    pub(crate) fn render_one_complete_block_at(&mut self, now_ms: u64) -> Result<(), RenderFault> {
        if self.context.terminal.load(Ordering::Acquire) || self.context.transport.faulted() {
            return Err(RenderFault::InvalidBlock);
        }
        if self.preflight.selection_epoch_exhausted() {
            self.context.latch(RenderFault::PreflightEpochExhausted);
            return Err(RenderFault::PreflightEpochExhausted);
        }
        let clock_rollback = self.last_now_ms.is_some_and(|previous| now_ms < previous);
        let effective_now_ms = self
            .last_now_ms
            .map_or(now_ms, |previous| previous.max(now_ms));
        self.last_now_ms = Some(effective_now_ms);
        let mut block = self.context.take_free()?;
        let frames = self.profile.fixed_buffer_frames();
        let channels = self.profile.device_output_channels() as usize;

        // The preflight mutex is deliberately acquired only by this
        // non-realtime worker.  Keep one guard for the complete device block so
        // a UI transaction cannot change test/solo selection halfway through a
        // block.  The callback never sees this object or this lock.
        let mut preflight = match self.preflight.state().lock() {
            Ok(guard) => guard,
            Err(_) => {
                block.samples.fill(0.0);
                self.context.recycle_from_callback(block);
                self.context.latch(RenderFault::InvalidBlock);
                return Err(RenderFault::InvalidBlock);
            }
        };
        let identity = match AsioPreflightIdentity::new(
            self.context.session_generation,
            self.context.transport.current(),
        ) {
            Ok(identity) => identity,
            Err(_) => {
                block.samples.fill(0.0);
                self.context.recycle_from_callback(block);
                self.context.latch(RenderFault::GenerationExhausted);
                return Err(RenderFault::GenerationExhausted);
            }
        };

        let mut previous_selection_signature = self.preflight.selection_signature();

        // The engine fence is read only while the existing worker-only state
        // mutex is held. A live owner clears both ephemeral selections before
        // this block is rendered; the local bit is synchronized on every
        // block, but no selection is ever restored when live exits.
        let engine_live_fence_word = self.preflight.timeline_audio_live_fence().word();
        let engine_live_fence_changed = self
            .last_engine_live_fence_word
            .is_some_and(|previous| previous != engine_live_fence_word);
        self.last_engine_live_fence_word = Some(engine_live_fence_word);
        let engine_live_active = self.preflight.timeline_audio_live_fence().active();
        let engine_live_changed = match preflight.sync_engine_live(
            identity,
            engine_live_active,
            engine_live_fence_changed,
        ) {
            Ok(changed) => changed,
            Err(_) => {
                block.samples.fill(0.0);
                self.context.recycle_from_callback(block);
                self.context.latch(RenderFault::InvalidBlock);
                return Err(RenderFault::InvalidBlock);
            }
        };
        if engine_live_changed
            || (engine_live_fence_changed && previous_selection_signature != 0)
            || (engine_live_active && previous_selection_signature != 0)
        {
            if let Err(fault) = self.preflight.advance_selection_epoch() {
                block.samples.fill(0.0);
                self.context.recycle_from_callback(block);
                self.context.latch(fault);
                return Err(fault);
            }
            self.preflight.publish_selection_kind(&preflight);
            previous_selection_signature = 0;
        }
        if clock_rollback {
            let result =
                self.preflight
                    .clear_for_clock_anomaly(&mut preflight, identity, effective_now_ms);
            if let Err(fault) = result {
                block.samples.fill(0.0);
                self.context.recycle_from_callback(block);
                self.context.latch(fault);
                return Err(fault);
            }
            self.preflight.publish_selection_kind(&preflight);
            previous_selection_signature = 0;
        }
        // Expiry is observed in the worker's non-realtime clock domain.  The
        // explicit result lets us advance the callback-visible fence exactly
        // when state expiry retires a test, including blocks already queued.
        let expired = match preflight.clear_if_expired(identity, effective_now_ms) {
            Ok(expired) => expired,
            Err(_) => {
                block.samples.fill(0.0);
                self.context.recycle_from_callback(block);
                self.context.latch(RenderFault::InvalidBlock);
                return Err(RenderFault::InvalidBlock);
            }
        };
        if expired {
            if let Err(fault) = self.preflight.advance_selection_epoch() {
                block.samples.fill(0.0);
                self.context.recycle_from_callback(block);
                self.context.latch(fault);
                return Err(fault);
            }
            self.preflight.publish_selection_kind(&preflight);
            previous_selection_signature = 0;
        }
        let test_target = match preflight.test_target(identity, effective_now_ms) {
            Ok(target) => target,
            Err(_) => {
                block.samples.fill(0.0);
                self.context.recycle_from_callback(block);
                self.context.latch(RenderFault::InvalidBlock);
                return Err(RenderFault::InvalidBlock);
            }
        };
        let current_selection_kind = if test_target.is_some() {
            PreflightSelectionKind::Test
        } else if preflight.solo() != PreflightSolo::None {
            PreflightSelectionKind::Solo
        } else {
            PreflightSelectionKind::None
        };
        if preflight.selection_signature() != previous_selection_signature {
            if let Err(fault) = self
                .preflight
                .publish_selection_transition(&preflight, previous_selection_signature)
            {
                block.samples.fill(0.0);
                self.context.recycle_from_callback(block);
                self.context.latch(fault);
                return Err(fault);
            }
        }
        let selection_epoch = if current_selection_kind == PreflightSelectionKind::None {
            0
        } else {
            self.preflight.selection_epoch()
        };
        let selection_signature = if current_selection_kind == PreflightSelectionKind::None {
            0
        } else {
            self.preflight.selection_signature()
        };
        let live_fence_word = if current_selection_kind == PreflightSelectionKind::None {
            0
        } else {
            engine_live_fence_word
        };

        for frame in 0..frames as usize {
            let destination = &mut block.samples[frame * channels..(frame + 1) * channels];

            // `test_target` is used instead of `render_test` here because the
            // latter intentionally rejects while live playback is marked
            // active.  A normal render must continue while no test is armed;
            // test admission itself is what is blocked by live playback.
            if let Some(target) = test_target {
                if self
                    .preflight
                    .mapper()
                    .map_target(target, destination)
                    .is_err()
                {
                    block.samples.fill(0.0);
                    self.context.recycle_from_callback(block);
                    self.context.latch(RenderFault::InvalidBlock);
                    return Err(RenderFault::InvalidBlock);
                }
                continue;
            }

            let program_left = self.program_source.next().unwrap_or(0.0);
            let program_right = self.program_source.next().unwrap_or(0.0);
            let cue_left = self.cue_source.next().unwrap_or(0.0);
            let cue_right = self.cue_source.next().unwrap_or(0.0);

            // Solo is a logical-bus operation.  Pull both private mixers so
            // their source clocks stay aligned, then zero the non-selected bus
            // before the single authoritative physical mapper runs.
            let (program_left, program_right, cue_left, cue_right) = match preflight.solo() {
                PreflightSolo::None => (program_left, program_right, cue_left, cue_right),
                PreflightSolo::ProgramOnly => (program_left, program_right, 0.0, 0.0),
                PreflightSolo::CueOnly => (0.0, 0.0, cue_left, cue_right),
            };
            if map_frame(
                &self.profile,
                program_left,
                program_right,
                CueFrame::Stereo {
                    left: cue_left,
                    right: cue_right,
                },
                destination,
            )
            .is_err()
            {
                block.samples.fill(0.0);
                self.context.recycle_from_callback(block);
                self.context.latch(RenderFault::InvalidBlock);
                return Err(RenderFault::InvalidBlock);
            }
        }
        block.session_generation = self.context.session_generation;
        block.transport_generation = self.context.transport.current();
        block.first_output_frame = self.next_output_frame;
        block.frames = frames;
        block.preflight_selection_epoch = selection_epoch;
        block.preflight_live_fence_word = live_fence_word;
        block.preflight_selection_signature = selection_signature;
        block.preflight_selection_kind = current_selection_kind;
        self.next_output_frame = self
            .next_output_frame
            .checked_add(u64::from(frames))
            .ok_or(RenderFault::GenerationExhausted)?;
        self.context.publish_ready(block)
    }

    pub(crate) fn prefill(&mut self, blocks: usize) -> Result<(), RenderFault> {
        if blocks < MIN_RENDER_QUEUE_SLOTS {
            return Err(RenderFault::QueueUnderflow);
        }
        for _ in 0..blocks {
            self.render_one_complete_block()?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asio_program_cue::{MachineAsioOutputProfile, PreflightTarget};

    fn test_engine() -> engine::EngineHandle {
        let engine = engine::EngineHandle::start_for_tests(protocol::DmxOutputConfig {
            enabled: false,
            ..protocol::DmxOutputConfig::default()
        });
        // Keep the real public Play/Pause transition live long enough for a
        // parallel suite: an empty Timeline reaches its terminal boundary on
        // the next engine tick and can otherwise clear the shared live fence
        // before this callback test consumes its second drain slot.
        let mut timeline = engine.snapshot().timeline;
        timeline.phases = vec![protocol::TimelinePhaseSummary {
            id: protocol::TimelinePhaseId(1),
            label: "ASIO live-fence test body".to_owned(),
            role: protocol::TimelinePhaseRole::Intro,
            start_ms: 0,
            end_ms: 16_000,
        }];
        engine
            .apply_timeline_bank_published(vec![timeline.clone()], timeline.id, false)
            .expect("seed a non-empty Timeline for the public live transition");
        engine
    }

    fn set_engine_live(engine: &engine::EngineHandle, playing: bool) {
        let fence = engine.timeline_audio_live_fence();
        for _ in 0..8 {
            let authority = engine.timeline_transport_authority();
            match engine.set_timeline_playing_published(
                authority.epoch,
                authority.generation,
                playing,
                Instant::now() + std::time::Duration::from_secs(1),
            ) {
                Ok(_) => {
                    let deadline = Instant::now() + std::time::Duration::from_secs(1);
                    while fence.active() != playing && Instant::now() < deadline {
                        std::thread::yield_now();
                    }
                    assert_eq!(fence.active(), playing);
                    return;
                }
                Err(error) if error.contains("stale") => std::thread::yield_now(),
                Err(error) => panic!("Timeline live test mutation failed: {error}"),
            }
        }
        panic!("Timeline live test mutation remained stale after retries");
    }

    fn profile() -> MachineAsioOutputProfile {
        MachineAsioOutputProfile::for_test(7, 4, 1, 7, 3)
    }

    fn context() -> Arc<AsioRenderContext> {
        AsioRenderContext::new(
            70,
            &profile(),
            4,
            Arc::new(TransportGeneration::new(9).unwrap()),
        )
        .unwrap()
    }

    fn context_with_live_fence(fence: TimelineAudioLiveFence) -> Arc<AsioRenderContext> {
        AsioRenderContext::new_with_live_fence(
            70,
            &profile(),
            4,
            Arc::new(TransportGeneration::new(9).unwrap()),
            fence,
        )
        .unwrap()
    }

    fn render_block(
        mixers: &mut ProgramCueMixers,
        context: &Arc<AsioRenderContext>,
        now_ms: u64,
        first_output_frame: u64,
    ) -> [f32; 28] {
        mixers
            .worker_mut()
            .render_one_complete_block_at(now_ms)
            .unwrap();
        let mut destination = [99.0_f32; 28];
        let result = unsafe {
            context.callback_copy(destination.as_mut_ptr(), 7, 4, first_output_frame, 70, 70)
        };
        assert_eq!(result, CALLBACK_ACCEPTED);
        destination
    }

    fn copy_block(context: &Arc<AsioRenderContext>, first_output_frame: u64) -> [f32; 28] {
        let mut destination = [99.0_f32; 28];
        let result = unsafe {
            context.callback_copy(destination.as_mut_ptr(), 7, 4, first_output_frame, 70, 70)
        };
        assert_eq!(result, CALLBACK_ACCEPTED);
        destination
    }

    fn assert_frames(destination: &[f32; 28], expected: [f32; 7]) {
        for frame in destination.chunks_exact(7) {
            assert_eq!(frame, expected.as_slice());
        }
    }

    fn append_stereo(mixer: &rodio::mixer::Mixer, left: f32, right: f32, blocks: usize) {
        mixer.add(rodio::buffer::SamplesBuffer::new(
            2,
            48_000,
            (0..blocks * 4)
                .flat_map(|_| [left, right])
                .collect::<Vec<_>>(),
        ));
    }

    #[test]
    fn callback_requires_both_session_and_transport_generations_and_silences_stale() {
        let context = context();
        let mut mixers = ProgramCueMixers::new(context.clone(), profile(), 100).unwrap();
        mixers.worker_mut().render_one_complete_block().unwrap();
        let mut destination = [9.0_f32; 28];
        let result = unsafe { context.callback_copy(destination.as_mut_ptr(), 7, 4, 100, 70, 70) };
        assert_eq!(result, CALLBACK_ACCEPTED);
        let control = mixers.preflight_control();
        let current = AsioPreflightIdentity::new(70, 9).unwrap();
        let next = AsioPreflightIdentity::new(70, 10).unwrap();
        control
            .state()
            .lock()
            .unwrap()
            .clear_for_transport_rotation(current, next)
            .unwrap();
        control.advance_test_epoch().unwrap();
        assert_eq!(context.transport().rotate().unwrap(), 10);
        mixers.worker_mut().render_one_complete_block().unwrap();
        let result = unsafe { context.callback_copy(destination.as_mut_ptr(), 7, 4, 104, 70, 70) };
        assert_eq!(result, CALLBACK_ACCEPTED);
        assert!(destination.iter().all(|value| value.is_finite()));
    }

    #[test]
    fn exhausted_session_generation_input_is_rejected_with_full_silence() {
        let context = context();
        let mut mixers = ProgramCueMixers::new(context.clone(), profile(), 0).unwrap();
        mixers.worker_mut().render_one_complete_block_at(0).unwrap();
        let mut destination = [99.0_f32; 28];
        assert_eq!(
            unsafe { context.callback_copy(destination.as_mut_ptr(), 7, 4, 0, u64::MAX, u64::MAX) },
            CALLBACK_INVALID_BLOCK
        );
        assert_eq!(destination, [0.0; 28]);
        assert_eq!(context.terminal_fault(), Some(RenderFault::InvalidBlock));
    }

    #[test]
    fn underflow_partial_noncontiguous_and_nonfinite_all_write_full_silence() {
        {
            let context = context();
            let mut destination = [1.0_f32; 28];
            assert_eq!(
                unsafe { context.callback_copy(destination.as_mut_ptr(), 7, 4, 0, 70, 70) },
                CALLBACK_QUEUE_UNDERFLOW
            );
            assert_eq!(destination, [0.0; 28]);
            assert_eq!(context.terminal_fault(), Some(RenderFault::QueueUnderflow));
        }

        {
            let context = context();
            let mut mixers = ProgramCueMixers::new(context.clone(), profile(), 0).unwrap();
            mixers.worker_mut().render_one_complete_block_at(0).unwrap();
            let mut wrong_width = [1.0_f32; 24];
            assert_eq!(
                unsafe { context.callback_copy(wrong_width.as_mut_ptr(), 6, 4, 0, 70, 70) },
                CALLBACK_INVALID_BLOCK
            );
            assert_eq!(wrong_width, [0.0; 24]);
        }

        {
            let context = context();
            let mut mixers = ProgramCueMixers::new(context.clone(), profile(), 0).unwrap();
            mixers.worker_mut().render_one_complete_block_at(0).unwrap();
            let mut partial = [1.0_f32; 21];
            assert_eq!(
                unsafe { context.callback_copy(partial.as_mut_ptr(), 7, 3, 0, 70, 70) },
                CALLBACK_INVALID_BLOCK
            );
            assert_eq!(partial, [0.0; 21]);
        }

        {
            let context = context();
            let mut mixers = ProgramCueMixers::new(context.clone(), profile(), 0).unwrap();
            mixers.worker_mut().render_one_complete_block_at(0).unwrap();
            let mut block = context.ready.pop().unwrap();
            block.samples[0] = f32::NAN;
            assert!(context.ready.push(block).is_ok());
            let mut nonfinite = [1.0_f32; 28];
            assert_eq!(
                unsafe { context.callback_copy(nonfinite.as_mut_ptr(), 7, 4, 0, 70, 70) },
                CALLBACK_INVALID_BLOCK
            );
            assert_eq!(nonfinite, [0.0; 28]);
        }
    }

    #[test]
    fn prefill_is_complete_device_width_and_queue_rotation_retires_old_blocks() {
        let context = context();
        let mut mixers = ProgramCueMixers::new(context.clone(), profile(), 0).unwrap();
        mixers.worker_mut().prefill(2).unwrap();
        assert_eq!(context.ready_len(), 2);
        context.transport().rotate().unwrap();
        context.retire_queued_blocks();
        assert_eq!(context.ready_len(), 0);
    }

    #[test]
    fn worker_applies_solo_to_logical_buses_before_the_single_physical_mapper() {
        let context = context();
        let mut mixers = ProgramCueMixers::new(context.clone(), profile(), 0).unwrap();
        append_stereo(&mixers.program, 0.2, 0.3, 3);
        append_stereo(&mixers.cue, 0.4, 0.6, 3);

        // The profile deliberately reorders the logical outputs: PROGRAM L is
        // physical 0, CUE is physical 2, and PROGRAM R is physical 6.
        let normal = render_block(&mut mixers, &context, 0, 0);
        assert_frames(&normal, [0.2, 0.0, 0.5, 0.0, 0.0, 0.0, 0.3]);

        let control = mixers.preflight_control();
        let identity = AsioPreflightIdentity::new(70, 9).unwrap();
        control
            .state()
            .lock()
            .unwrap()
            .set_solo(PreflightSolo::ProgramOnly, identity, 1)
            .unwrap();
        let program_only = render_block(&mut mixers, &context, 1, 4);
        assert_frames(&program_only, [0.2, 0.0, 0.0, 0.0, 0.0, 0.0, 0.3]);

        control
            .state()
            .lock()
            .unwrap()
            .set_solo(PreflightSolo::CueOnly, identity, 2)
            .unwrap();
        let cue_only = render_block(&mut mixers, &context, 2, 8);
        assert_frames(&cue_only, [0.0, 0.0, 0.5, 0.0, 0.0, 0.0, 0.0]);
    }

    #[test]
    fn queued_solo_block_is_silenced_after_solo_is_cleared() {
        let context = context();
        let mut mixers = ProgramCueMixers::new(context.clone(), profile(), 0).unwrap();
        let control = mixers.preflight_control();
        let identity = AsioPreflightIdentity::new(70, 9).unwrap();
        control
            .state()
            .lock()
            .unwrap()
            .set_solo(PreflightSolo::ProgramOnly, identity, 0)
            .unwrap();
        mixers.worker_mut().render_one_complete_block_at(0).unwrap();

        let previous_signature = context
            .preflight_selection_signature()
            .load(Ordering::Acquire);
        let mut state = control.state().lock().unwrap();
        state.set_solo(PreflightSolo::None, identity, 1).unwrap();
        control
            .publish_selection_transition(&state, previous_signature)
            .unwrap();
        drop(state);

        assert_eq!(copy_block(&context, 0), [0.0; 28]);
    }

    #[test]
    fn queued_solo_block_is_silenced_when_same_kind_signature_changes_before_epoch() {
        let context = context();
        let mut mixers = ProgramCueMixers::new(context.clone(), profile(), 0).unwrap();
        append_stereo(&mixers.program, 0.2, 0.3, 1);
        append_stereo(&mixers.cue, 0.4, 0.6, 1);
        let control = mixers.preflight_control();
        let identity = AsioPreflightIdentity::new(70, 9).unwrap();
        control
            .state()
            .lock()
            .unwrap()
            .set_solo(PreflightSolo::ProgramOnly, identity, 0)
            .unwrap();
        mixers.worker_mut().render_one_complete_block_at(0).unwrap();
        let epoch = context.preflight_selection_epoch().load(Ordering::Acquire);

        // Model the callback-visible half-published same-kind mode change:
        // the state has moved to CueOnly and its signature is visible, while
        // the epoch and Solo kind still describe the queued ProgramOnly block.
        control
            .state()
            .lock()
            .unwrap()
            .set_solo(PreflightSolo::CueOnly, identity, 1)
            .unwrap();
        context
            .preflight_selection_signature()
            .store(7, Ordering::Release);
        assert_eq!(
            context.preflight_selection_epoch().load(Ordering::Acquire),
            epoch
        );
        assert_eq!(
            context.preflight_selection_kind().load(Ordering::Acquire),
            2
        );

        // Epoch and kind alone would accept this old Solo block. The captured
        // signature makes the callback reject it without copying any sample.
        assert_eq!(copy_block(&context, 0), [0.0; 28]);
    }

    #[test]
    fn same_kind_solo_signature_change_after_copy_silences_the_whole_buffer() {
        let context = context();
        let mut mixers = ProgramCueMixers::new(context.clone(), profile(), 0).unwrap();
        append_stereo(&mixers.program, 0.2, 0.3, 1);
        append_stereo(&mixers.cue, 0.4, 0.6, 1);
        let control = mixers.preflight_control();
        let identity = AsioPreflightIdentity::new(70, 9).unwrap();
        control
            .state()
            .lock()
            .unwrap()
            .set_solo(PreflightSolo::ProgramOnly, identity, 0)
            .unwrap();
        mixers.worker_mut().render_one_complete_block_at(0).unwrap();

        context
            .post_copy_gate
            .enabled
            .store(true, Ordering::Release);
        context
            .post_copy_gate
            .reached
            .store(false, Ordering::Release);
        context
            .post_copy_gate
            .release
            .store(false, Ordering::Release);
        let callback_context = Arc::clone(&context);
        let callback = std::thread::spawn(move || {
            let mut destination = [99.0_f32; 28];
            let result = unsafe {
                callback_context.callback_copy(destination.as_mut_ptr(), 7, 4, 0, 70, 70)
            };
            (result, destination)
        });
        while !context.post_copy_gate.reached.load(Ordering::Acquire) {
            std::thread::yield_now();
        }

        // Keep the callback's epoch and kind unchanged so the signature is
        // the only post-copy admission difference.
        control
            .state()
            .lock()
            .unwrap()
            .set_solo(PreflightSolo::CueOnly, identity, 1)
            .unwrap();
        context
            .preflight_selection_signature()
            .store(7, Ordering::Release);
        context
            .post_copy_gate
            .release
            .store(true, Ordering::Release);
        let (result, destination) = callback.join().unwrap();
        assert_eq!(result, CALLBACK_ACCEPTED);
        assert_eq!(destination, [0.0; 28]);
    }

    fn run_selection_entry_during_copy(target: PreflightSelectionKind) -> [f32; 28] {
        let context = context();
        let mut mixers = ProgramCueMixers::new(context.clone(), profile(), 0).unwrap();
        mixers.worker_mut().render_one_complete_block_at(0).unwrap();
        let control = mixers.preflight_control();
        let identity = AsioPreflightIdentity::new(70, 9).unwrap();
        context
            .post_copy_gate
            .enabled
            .store(true, Ordering::Release);
        context
            .post_copy_gate
            .reached
            .store(false, Ordering::Release);
        context
            .post_copy_gate
            .release
            .store(false, Ordering::Release);

        let callback_context = Arc::clone(&context);
        let callback = std::thread::spawn(move || {
            let mut destination = [99.0_f32; 28];
            let result = unsafe {
                callback_context.callback_copy(destination.as_mut_ptr(), 7, 4, 0, 70, 70)
            };
            (result, destination)
        });
        while !context.post_copy_gate.reached.load(Ordering::Acquire) {
            std::thread::yield_now();
        }

        let previous_signature = context
            .preflight_selection_signature()
            .load(Ordering::Acquire);
        let mut state = control.state().lock().unwrap();
        match target {
            PreflightSelectionKind::Test => state
                .begin_test_with_mapper(
                    control.mapper(),
                    PreflightTarget::ProgramLeft,
                    identity,
                    1,
                    1000,
                )
                .unwrap(),
            PreflightSelectionKind::Solo => state
                .set_solo(PreflightSolo::ProgramOnly, identity, 1)
                .unwrap(),
            PreflightSelectionKind::None => unreachable!(),
        }
        control
            .publish_selection_transition(&state, previous_signature)
            .unwrap();
        drop(state);
        context
            .post_copy_gate
            .release
            .store(true, Ordering::Release);
        let (result, destination) = callback.join().unwrap();
        assert_eq!(result, CALLBACK_ACCEPTED);
        destination
    }

    #[test]
    fn test_and_solo_entry_during_copy_silence_the_whole_buffer() {
        assert_eq!(
            run_selection_entry_during_copy(PreflightSelectionKind::Test),
            [0.0; 28]
        );
        assert_eq!(
            run_selection_entry_during_copy(PreflightSelectionKind::Solo),
            [0.0; 28]
        );
    }

    #[test]
    fn live_fence_enter_exit_enter_invalidates_a_queued_test_without_aba() {
        let engine = test_engine();
        let fence = engine.timeline_audio_live_fence();
        let context = context_with_live_fence(fence.clone());
        let mut mixers = ProgramCueMixers::new(context.clone(), profile(), 0).unwrap();
        let control = mixers.preflight_control();
        let identity = AsioPreflightIdentity::new(70, 9).unwrap();
        control
            .state()
            .lock()
            .unwrap()
            .begin_test_with_mapper(
                control.mapper(),
                PreflightTarget::ProgramLeft,
                identity,
                0,
                1000,
            )
            .unwrap();
        mixers.worker_mut().render_one_complete_block_at(0).unwrap();

        set_engine_live(&engine, true);
        set_engine_live(&engine, false);
        set_engine_live(&engine, true);
        assert!(fence.active());
        assert_ne!(fence.word(), 2);
        assert_eq!(copy_block(&context, 0), [0.0; 28]);
    }

    #[test]
    fn ordinary_block_continues_while_engine_live_and_cleared_selection_never_resumes() {
        let engine = test_engine();
        let fence = engine.timeline_audio_live_fence();
        let context = context_with_live_fence(fence.clone());
        let mut mixers = ProgramCueMixers::new(context.clone(), profile(), 0).unwrap();
        append_stereo(&mixers.program, 0.2, 0.3, 4);
        append_stereo(&mixers.cue, 0.4, 0.6, 4);
        let control = mixers.preflight_control();
        let identity = AsioPreflightIdentity::new(70, 9).unwrap();
        control
            .state()
            .lock()
            .unwrap()
            .begin_test_with_mapper(
                control.mapper(),
                PreflightTarget::ProgramLeft,
                identity,
                0,
                1000,
            )
            .unwrap();
        mixers.worker_mut().render_one_complete_block_at(0).unwrap();
        // A normal block is queued after the engine owner enters live mode;
        // the worker clears Test and emits ordinary PROGRAM/CUE output.
        set_engine_live(&engine, true);
        assert!(fence.active());
        mixers.worker_mut().render_one_complete_block_at(1).unwrap();
        assert_eq!(copy_block(&context, 0), [0.0; 28]);
        assert_eq!(context.live_drain_remaining().load(Ordering::Acquire), 1);
        mixers.worker_mut().render_one_complete_block_at(2).unwrap();
        let expected = [0.2, 0.0, 0.5, 0.0, 0.0, 0.0, 0.3];
        assert_eq!(copy_block(&context, 4), [0.0; 28]);
        mixers.worker_mut().render_one_complete_block_at(3).unwrap();
        assert_frames(&copy_block(&context, 8), expected);

        // Exiting live does not restore the cleared selection. A fresh
        // ordinary block is admissible while the packed word is inactive.
        set_engine_live(&engine, false);
        mixers.worker_mut().render_one_complete_block_at(4).unwrap();
        assert_frames(&copy_block(&context, 12), expected);
    }

    #[test]
    fn live_fence_active_generations_rearm_the_two_callback_drain() {
        let engine = test_engine();
        let fence = engine.timeline_audio_live_fence();
        let context = context_with_live_fence(fence.clone());
        let mut mixers = ProgramCueMixers::new(context.clone(), profile(), 0).unwrap();
        append_stereo(&mixers.program, 0.2, 0.3, 8);
        append_stereo(&mixers.cue, 0.4, 0.6, 8);
        let expected = [0.2, 0.0, 0.5, 0.0, 0.0, 0.0, 0.3];

        // Establish the inactive baseline and prove ordinary output is still
        // audible before the first engine-owned live generation.
        mixers.worker_mut().render_one_complete_block_at(0).unwrap();
        assert_frames(&copy_block(&context, 0), expected);

        set_engine_live(&engine, true);
        mixers.worker_mut().render_one_complete_block_at(1).unwrap();
        assert_eq!(copy_block(&context, 4), [0.0; 28]);
        assert_eq!(context.live_drain_remaining().load(Ordering::Acquire), 1);
        mixers.worker_mut().render_one_complete_block_at(2).unwrap();
        assert_eq!(copy_block(&context, 8), [0.0; 28]);
        assert_eq!(context.live_drain_remaining().load(Ordering::Acquire), 0);
        mixers.worker_mut().render_one_complete_block_at(3).unwrap();
        assert_frames(&copy_block(&context, 12), expected);

        // An inactive observation clears any residual drain.  A later active
        // word has a newer monotonic epoch and therefore arms two callbacks
        // again, without restoring the retired Test/Solo selection.
        set_engine_live(&engine, false);
        mixers.worker_mut().render_one_complete_block_at(4).unwrap();
        assert_frames(&copy_block(&context, 16), expected);
        assert_eq!(context.live_drain_remaining().load(Ordering::Acquire), 0);

        set_engine_live(&engine, true);
        mixers.worker_mut().render_one_complete_block_at(5).unwrap();
        assert_eq!(copy_block(&context, 20), [0.0; 28]);
        assert_eq!(context.live_drain_remaining().load(Ordering::Acquire), 1);
        mixers.worker_mut().render_one_complete_block_at(6).unwrap();
        assert_eq!(copy_block(&context, 24), [0.0; 28]);
        assert_eq!(context.live_drain_remaining().load(Ordering::Acquire), 0);
        mixers.worker_mut().render_one_complete_block_at(7).unwrap();
        assert_frames(&copy_block(&context, 28), expected);
    }

    #[test]
    fn live_fence_reentry_with_one_slot_remaining_rearms_without_shortening() {
        let engine = test_engine();
        let fence = engine.timeline_audio_live_fence();
        let context = context_with_live_fence(fence.clone());
        let mut mixers = ProgramCueMixers::new(context.clone(), profile(), 0).unwrap();
        append_stereo(&mixers.program, 0.2, 0.3, 8);
        append_stereo(&mixers.cue, 0.4, 0.6, 8);
        let expected = [0.2, 0.0, 0.5, 0.0, 0.0, 0.0, 0.3];

        mixers.worker_mut().render_one_complete_block_at(0).unwrap();
        assert_frames(&copy_block(&context, 0), expected);

        // Spend one slot of the first active generation.
        set_engine_live(&engine, true);
        mixers.worker_mut().render_one_complete_block_at(1).unwrap();
        assert_eq!(copy_block(&context, 4), [0.0; 28]);
        assert_eq!(context.live_drain_remaining().load(Ordering::Acquire), 1);

        // Leave and re-enter while one slot remains. The new active word must
        // re-arm the complete two-callback drain, not merely preserve one.
        set_engine_live(&engine, false);
        set_engine_live(&engine, true);
        mixers.worker_mut().render_one_complete_block_at(2).unwrap();
        assert_eq!(copy_block(&context, 8), [0.0; 28]);
        assert_eq!(context.live_drain_remaining().load(Ordering::Acquire), 1);
        mixers.worker_mut().render_one_complete_block_at(3).unwrap();
        assert_eq!(copy_block(&context, 12), [0.0; 28]);
        assert_eq!(context.live_drain_remaining().load(Ordering::Acquire), 0);

        set_engine_live(&engine, false);
        mixers.worker_mut().render_one_complete_block_at(4).unwrap();
        assert_frames(&copy_block(&context, 16), expected);
    }

    #[test]
    fn live_fence_entry_after_copy_precheck_drains_boundary_and_next_callback() {
        let engine = test_engine();
        let fence = engine.timeline_audio_live_fence();
        let context = context_with_live_fence(fence.clone());
        let mut mixers = ProgramCueMixers::new(context.clone(), profile(), 0).unwrap();
        append_stereo(&mixers.program, 0.2, 0.3, 3);
        append_stereo(&mixers.cue, 0.4, 0.6, 3);
        mixers.worker_mut().render_one_complete_block_at(0).unwrap();
        let expected = [0.2, 0.0, 0.5, 0.0, 0.0, 0.0, 0.3];

        let gate = Arc::clone(&context.post_copy_gate);
        gate.enabled.store(true, Ordering::Release);
        gate.reached.store(false, Ordering::Release);
        gate.release.store(false, Ordering::Release);
        let callback_context = Arc::clone(&context);
        let callback = std::thread::spawn(move || {
            let mut destination = [99.0_f32; 28];
            let result = unsafe {
                callback_context.callback_copy(destination.as_mut_ptr(), 7, 4, 0, 70, 70)
            };
            (result, destination)
        });
        while !gate.reached.load(Ordering::Acquire) {
            std::thread::yield_now();
        }

        // Publication does not wait for the callback's post-copy gate.  The
        // callback's second fence observation will turn the in-flight block
        // into whole-buffer silence and spend the first central drain slot.
        set_engine_live(&engine, true);
        assert!(fence.active());
        gate.release.store(true, Ordering::Release);
        let (result, destination) = callback.join().unwrap();
        assert_eq!(result, CALLBACK_ACCEPTED);
        assert_eq!(destination, [0.0; 28]);
        assert_eq!(context.live_drain_remaining().load(Ordering::Acquire), 1);

        mixers.worker_mut().render_one_complete_block_at(1).unwrap();
        assert_eq!(copy_block(&context, 4), [0.0; 28]);
        mixers.worker_mut().render_one_complete_block_at(2).unwrap();
        assert_frames(&copy_block(&context, 8), expected);
    }

    #[test]
    fn engine_live_enter_exit_between_worker_blocks_clears_stale_selection() {
        let engine = test_engine();
        let fence = engine.timeline_audio_live_fence();
        let context = context_with_live_fence(fence);
        let mut mixers = ProgramCueMixers::new(context.clone(), profile(), 0).unwrap();
        append_stereo(&mixers.program, 0.2, 0.3, 4);
        append_stereo(&mixers.cue, 0.4, 0.6, 4);
        let expected = [0.2, 0.0, 0.5, 0.0, 0.0, 0.0, 0.3];

        // Establish the callback's inactive baseline before the central owner
        // completes an enter/exit pair between callback observations.
        mixers.worker_mut().render_one_complete_block_at(0).unwrap();
        assert_frames(&copy_block(&context, 0), expected);

        let control = mixers.preflight_control();
        let identity = AsioPreflightIdentity::new(70, 9).unwrap();
        control
            .state()
            .lock()
            .unwrap()
            .begin_test_with_mapper(
                control.mapper(),
                PreflightTarget::ProgramLeft,
                identity,
                0,
                1000,
            )
            .unwrap();
        mixers.worker_mut().render_one_complete_block_at(0).unwrap();

        // The central owner can enter and leave between worker blocks. The
        // monotonic word changes even though the callback never observes the
        // active bit. The newer inactive word still proves a skipped live
        // generation, so both following callbacks must be full silence.
        set_engine_live(&engine, true);
        set_engine_live(&engine, false);
        mixers.worker_mut().render_one_complete_block_at(1).unwrap();
        mixers.worker_mut().render_one_complete_block_at(2).unwrap();

        assert_eq!(copy_block(&context, 4), [0.0; 28]);
        assert_eq!(copy_block(&context, 8), [0.0; 28]);
        mixers.worker_mut().render_one_complete_block_at(3).unwrap();
        assert_frames(&copy_block(&context, 12), expected);
        assert_eq!(
            control
                .state()
                .lock()
                .unwrap()
                .test_target(identity, 1)
                .unwrap(),
            None
        );
    }

    #[test]
    fn engine_live_enter_exit_between_worker_blocks_clears_stale_solo() {
        let engine = test_engine();
        let fence = engine.timeline_audio_live_fence();
        let context = context_with_live_fence(fence);
        let mut mixers = ProgramCueMixers::new(context.clone(), profile(), 0).unwrap();
        append_stereo(&mixers.program, 0.2, 0.3, 4);
        append_stereo(&mixers.cue, 0.4, 0.6, 4);
        let expected = [0.2, 0.0, 0.5, 0.0, 0.0, 0.0, 0.3];

        // Establish the callback's inactive baseline before the live owner
        // completes an enter/exit pair between callback observations.
        mixers.worker_mut().render_one_complete_block_at(0).unwrap();
        assert_frames(&copy_block(&context, 0), expected);

        let control = mixers.preflight_control();
        let identity = AsioPreflightIdentity::new(70, 9).unwrap();
        control
            .state()
            .lock()
            .unwrap()
            .set_solo(PreflightSolo::ProgramOnly, identity, 0)
            .unwrap();
        mixers.worker_mut().render_one_complete_block_at(0).unwrap();

        // The inactive word is newer even though the callback never observed
        // the active bit. Both already-returned A/B tail slots are discarded.
        set_engine_live(&engine, true);
        set_engine_live(&engine, false);
        mixers.worker_mut().render_one_complete_block_at(1).unwrap();
        mixers.worker_mut().render_one_complete_block_at(2).unwrap();
        assert_eq!(copy_block(&context, 4), [0.0; 28]);
        assert_eq!(copy_block(&context, 8), [0.0; 28]);

        mixers.worker_mut().render_one_complete_block_at(3).unwrap();
        assert_frames(&copy_block(&context, 12), expected);
        assert_eq!(control.state().lock().unwrap().solo(), PreflightSolo::None);
    }

    fn run_engine_live_entry_during_copy(target: PreflightSelectionKind) -> [f32; 28] {
        let engine = test_engine();
        let fence = engine.timeline_audio_live_fence();
        let context = context_with_live_fence(fence);
        let mut mixers = ProgramCueMixers::new(context.clone(), profile(), 0).unwrap();
        append_stereo(&mixers.program, 0.2, 0.3, 2);
        append_stereo(&mixers.cue, 0.4, 0.6, 2);
        let control = mixers.preflight_control();
        let identity = AsioPreflightIdentity::new(70, 9).unwrap();
        {
            let mut state = control.state().lock().unwrap();
            match target {
                PreflightSelectionKind::Test => state
                    .begin_test_with_mapper(
                        control.mapper(),
                        PreflightTarget::ProgramLeft,
                        identity,
                        0,
                        1000,
                    )
                    .unwrap(),
                PreflightSelectionKind::Solo => state
                    .set_solo(PreflightSolo::ProgramOnly, identity, 0)
                    .unwrap(),
                PreflightSelectionKind::None => unreachable!(),
            }
        }
        mixers.worker_mut().render_one_complete_block_at(0).unwrap();
        context
            .post_copy_gate
            .enabled
            .store(true, Ordering::Release);
        context
            .post_copy_gate
            .reached
            .store(false, Ordering::Release);
        context
            .post_copy_gate
            .release
            .store(false, Ordering::Release);

        let callback_context = Arc::clone(&context);
        let callback = std::thread::spawn(move || {
            let mut destination = [99.0_f32; 28];
            let result = unsafe {
                callback_context.callback_copy(destination.as_mut_ptr(), 7, 4, 0, 70, 70)
            };
            (result, destination)
        });
        while !context.post_copy_gate.reached.load(Ordering::Acquire) {
            std::thread::yield_now();
        }
        set_engine_live(&engine, true);
        context
            .post_copy_gate
            .release
            .store(true, Ordering::Release);
        let (result, destination) = callback.join().unwrap();
        assert_eq!(result, CALLBACK_ACCEPTED);
        destination
    }

    #[test]
    fn test_and_solo_engine_fence_entry_during_copy_silence_the_whole_buffer() {
        assert_eq!(
            run_engine_live_entry_during_copy(PreflightSelectionKind::Test),
            [0.0; 28]
        );
        assert_eq!(
            run_engine_live_entry_during_copy(PreflightSelectionKind::Solo),
            [0.0; 28]
        );
    }

    #[test]
    fn worker_test_isolates_one_mapped_target_then_expires_to_normal_render() {
        let context = context();
        let mut mixers = ProgramCueMixers::new(context.clone(), profile(), 0).unwrap();
        append_stereo(&mixers.program, 0.2, 0.3, 2);
        append_stereo(&mixers.cue, 0.4, 0.6, 2);
        let control = mixers.preflight_control();
        let identity = AsioPreflightIdentity::new(70, 9).unwrap();
        control
            .state()
            .lock()
            .unwrap()
            .begin_test_with_mapper(
                control.mapper(),
                PreflightTarget::ProgramRight,
                identity,
                100,
                50,
            )
            .unwrap();

        let test = render_block(&mut mixers, &context, 100, 0);
        assert_frames(&test, [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.1]);

        // At the exact expiry boundary the worker retires the test and resumes
        // the normal PROGRAM/CUE pull, proving no stale test frame leaks.
        let normal = render_block(&mut mixers, &context, 150, 4);
        assert_frames(&normal, [0.2, 0.0, 0.5, 0.0, 0.0, 0.0, 0.3]);

        control
            .state()
            .lock()
            .unwrap()
            .begin_test_with_mapper(control.mapper(), PreflightTarget::Cue, identity, 200, 100)
            .unwrap();
        control
            .state()
            .lock()
            .unwrap()
            .begin_test_with_mapper(control.mapper(), PreflightTarget::Off, identity, 201, 1)
            .unwrap();
        let off = render_block(&mut mixers, &context, 201, 8);
        assert_frames(&off, [0.2, 0.0, 0.5, 0.0, 0.0, 0.0, 0.3]);
    }

    #[test]
    fn queued_test_blocks_are_silenced_after_off_but_normal_audio_is_preserved() {
        let context = context();
        let mut mixers = ProgramCueMixers::new(context.clone(), profile(), 0).unwrap();
        append_stereo(&mixers.program, 0.2, 0.3, 2);
        append_stereo(&mixers.cue, 0.4, 0.6, 2);
        let control = mixers.preflight_control();
        let identity = AsioPreflightIdentity::new(70, 9).unwrap();
        control
            .state()
            .lock()
            .unwrap()
            .begin_test_with_mapper(
                control.mapper(),
                PreflightTarget::ProgramLeft,
                identity,
                0,
                100,
            )
            .unwrap();
        mixers.worker_mut().render_one_complete_block_at(0).unwrap();
        mixers.worker_mut().render_one_complete_block_at(0).unwrap();

        control
            .state()
            .lock()
            .unwrap()
            .begin_test_with_mapper(control.mapper(), PreflightTarget::Off, identity, 1, 1)
            .unwrap();
        control.advance_test_epoch().unwrap();
        mixers.worker_mut().render_one_complete_block_at(1).unwrap();

        let first = copy_block(&context, 0);
        let second = copy_block(&context, 4);
        let normal = copy_block(&context, 8);
        assert_eq!(first, [0.0; 28]);
        assert_eq!(second, [0.0; 28]);
        assert_frames(&normal, [0.2, 0.0, 0.5, 0.0, 0.0, 0.0, 0.3]);
    }

    #[test]
    fn queued_test_blocks_are_silenced_after_worker_expiry_but_normal_audio_is_preserved() {
        let context = context();
        let mut mixers = ProgramCueMixers::new(context.clone(), profile(), 0).unwrap();
        append_stereo(&mixers.program, 0.2, 0.3, 2);
        append_stereo(&mixers.cue, 0.4, 0.6, 2);
        let control = mixers.preflight_control();
        let identity = AsioPreflightIdentity::new(70, 9).unwrap();
        control
            .state()
            .lock()
            .unwrap()
            .begin_test_with_mapper(
                control.mapper(),
                PreflightTarget::ProgramLeft,
                identity,
                0,
                1,
            )
            .unwrap();
        mixers.worker_mut().render_one_complete_block_at(0).unwrap();
        mixers.worker_mut().render_one_complete_block_at(0).unwrap();

        // The third block is the non-test block that observes the exact expiry
        // boundary and advances the callback fence before it is published.
        mixers.worker_mut().render_one_complete_block_at(1).unwrap();
        assert_eq!(copy_block(&context, 0), [0.0; 28]);
        assert_eq!(copy_block(&context, 4), [0.0; 28]);
        let normal = copy_block(&context, 8);
        assert_frames(&normal, [0.2, 0.0, 0.5, 0.0, 0.0, 0.0, 0.3]);
    }

    #[test]
    fn worker_clock_rollback_invalidates_queued_test_without_silencing_normal_audio() {
        let context = context();
        let mut mixers = ProgramCueMixers::new(context.clone(), profile(), 0).unwrap();
        append_stereo(&mixers.program, 0.2, 0.3, 2);
        append_stereo(&mixers.cue, 0.4, 0.6, 2);
        let control = mixers.preflight_control();
        let identity = AsioPreflightIdentity::new(70, 9).unwrap();
        control
            .state()
            .lock()
            .unwrap()
            .begin_test_with_mapper(
                control.mapper(),
                PreflightTarget::ProgramLeft,
                identity,
                100,
                1000,
            )
            .unwrap();

        // Keep the first test block queued, then feed a backwards clock to
        // the worker.  The anomaly retires the test epoch, but the next block
        // is still ordinary PROGRAM/CUE output.
        mixers
            .worker_mut()
            .render_one_complete_block_at(100)
            .unwrap();
        mixers
            .worker_mut()
            .render_one_complete_block_at(99)
            .unwrap();

        assert_eq!(copy_block(&context, 0), [0.0; 28]);
        let normal = copy_block(&context, 4);
        assert_frames(&normal, [0.2, 0.0, 0.5, 0.0, 0.0, 0.0, 0.3]);
        assert_eq!(control.state().lock().unwrap().solo(), PreflightSolo::None);
        assert!(control
            .state()
            .lock()
            .unwrap()
            .test_target(identity, 100)
            .unwrap()
            .is_none());
    }

    #[test]
    fn preflight_epoch_exhaustion_silences_stale_test_and_latches_worker_fault() {
        let context = context();
        let mut mixers = ProgramCueMixers::new(context.clone(), profile(), 0).unwrap();
        let control = mixers.preflight_control();
        let identity = AsioPreflightIdentity::new(70, 9).unwrap();
        control
            .state()
            .lock()
            .unwrap()
            .begin_test_with_mapper(
                control.mapper(),
                PreflightTarget::ProgramLeft,
                identity,
                0,
                1000,
            )
            .unwrap();

        // Force the last representable nonzero value so the next fence
        // mutation is deterministic.  The block carries MAX and is already
        // queued before exhaustion publishes the reserved ordinary epoch 0.
        context
            .preflight_test_epoch()
            .store(u64::MAX, Ordering::Release);
        // The direct state mutation above intentionally bypasses the runtime
        // admission helper; mirror its callback-visible classification so the
        // worker can render the final representable affected block before the
        // explicit exhaustion transition below.
        context
            .preflight_selection_kind()
            .store(PreflightSelectionKind::Test as u8, Ordering::Release);
        context
            .preflight_selection_signature()
            .store(1, Ordering::Release);
        mixers.worker_mut().render_one_complete_block_at(0).unwrap();
        assert_eq!(control.test_epoch(), u64::MAX);
        assert_eq!(
            control.advance_test_epoch(),
            Err(RenderFault::PreflightEpochExhausted)
        );
        assert_eq!(control.test_epoch(), 0);
        assert_eq!(
            control.advance_test_epoch(),
            Err(RenderFault::PreflightEpochExhausted)
        );

        // A stale Test block cannot become audible after exhaustion.  The
        // callback still consumes it as a bounded fence barrier.
        assert_eq!(copy_block(&context, 0), [0.0; 28]);

        // Future non-realtime production fails closed with an actionable
        // terminal fault rather than assigning epoch zero to a Test block.
        assert_eq!(
            mixers.worker_mut().render_one_complete_block_at(1),
            Err(RenderFault::PreflightEpochExhausted)
        );
        assert_eq!(
            context.terminal_fault(),
            Some(RenderFault::PreflightEpochExhausted)
        );
    }

    #[test]
    fn callback_copy_does_not_acquire_the_non_realtime_preflight_lock() {
        let context = context();
        let mut mixers = ProgramCueMixers::new(context.clone(), profile(), 0).unwrap();
        mixers.worker_mut().render_one_complete_block_at(0).unwrap();
        let control = mixers.preflight_control();
        let _guard = control.state().lock().unwrap();

        // Holding the worker-only control mutex must not affect the callback's
        // bounded queue transfer.  A callback-side lock would deadlock here.
        let mut destination = [99.0_f32; 28];
        let result = unsafe { context.callback_copy(destination.as_mut_ptr(), 7, 4, 0, 70, 70) };
        assert_eq!(result, CALLBACK_ACCEPTED);
        assert!(destination.iter().all(|sample| sample.is_finite()));
    }

    #[test]
    fn callback_post_copy_transport_recheck_silences_a_rotated_block() {
        let context = context();
        let mut mixers = ProgramCueMixers::new(context.clone(), profile(), 0).unwrap();
        append_stereo(&mixers.program, 0.2, 0.3, 1);
        append_stereo(&mixers.cue, 0.4, 0.6, 1);
        mixers.worker_mut().render_one_complete_block_at(0).unwrap();
        let gate = Arc::clone(&context.post_copy_gate);
        gate.enabled.store(true, Ordering::Release);
        gate.reached.store(false, Ordering::Release);
        gate.release.store(false, Ordering::Release);

        let callback_context = Arc::clone(&context);
        let callback = std::thread::spawn(move || {
            let mut destination = [99.0_f32; 28];
            let result = unsafe {
                callback_context.callback_copy(destination.as_mut_ptr(), 7, 4, 0, 70, 70)
            };
            (result, destination)
        });
        while !gate.reached.load(Ordering::Acquire) {
            std::thread::yield_now();
        }
        assert_eq!(context.transport().rotate().unwrap(), 10);
        gate.release.store(true, Ordering::Release);

        let (result, destination) = callback.join().unwrap();
        assert_eq!(result, CALLBACK_ACCEPTED);
        assert_eq!(destination, [0.0; 28]);
    }
}
