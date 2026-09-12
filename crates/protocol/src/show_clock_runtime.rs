//! Deterministic ShowClock estimation and action scheduling.
//!
//! This module deliberately has no wall-clock, socket, thread, or output
//! dependency.  Callers provide monotonic microseconds, authenticate a sample
//! with [`crate::show_clock::ShowClockPeerValidator`], and then feed the body
//! here.  That keeps transport admission, clock policy, and output ownership
//! independently testable.

use std::collections::BTreeMap;

use crate::show_clock::{
    ShowClockAction, ShowClockFenceState, ShowClockHash, ShowClockLatePolicy, ShowClockManualFence,
    ShowClockNodeId, ShowClockPeerValidator, ShowClockSample, ShowClockValidationError,
    SHOW_CLOCK_DEFAULT_MAX_SLEW_US_PER_SAMPLE,
};

/// A peer is considered stale after three missed 250 ms samples by default.
pub const SHOW_CLOCK_DEFAULT_STALE_AFTER_US: u64 = 750_000;
pub const SHOW_CLOCK_DEFAULT_LOCK_SAMPLES: u32 = 3;
pub const SHOW_CLOCK_DEFAULT_ACTION_HORIZON_US: u64 = 10_000_000;
pub const SHOW_CLOCK_MAX_SCHEDULED_ACTIONS: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShowClockEstimatorConfig {
    pub max_slew_us_per_sample: u64,
    pub stale_after_us: u64,
    pub lock_after_samples: u32,
}

impl Default for ShowClockEstimatorConfig {
    fn default() -> Self {
        Self {
            max_slew_us_per_sample: SHOW_CLOCK_DEFAULT_MAX_SLEW_US_PER_SAMPLE,
            stale_after_us: SHOW_CLOCK_DEFAULT_STALE_AFTER_US,
            lock_after_samples: SHOW_CLOCK_DEFAULT_LOCK_SAMPLES,
        }
    }
}

impl ShowClockEstimatorConfig {
    fn validate(self) -> Result<Self, ShowClockValidationError> {
        if self.max_slew_us_per_sample == 0
            || self.max_slew_us_per_sample > i64::MAX as u64
            || self.stale_after_us == 0
            || self.lock_after_samples == 0
        {
            return Err(ShowClockValidationError::InvalidEstimatorConfig);
        }
        Ok(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShowClockEstimatorState {
    Acquiring,
    Locked,
    Hold,
    Stale,
    Fault,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShowClockEstimate {
    pub state: ShowClockEstimatorState,
    pub show_time_us: u64,
    pub offset_us: i64,
    pub sample_age_us: Option<u64>,
    pub accepted_samples: u32,
    pub clock_generation: u64,
    pub fencing_generation: u64,
}

/// A bounded, monotonic estimator for the authenticated ShowClock domain.
///
/// The first accepted sample establishes the local-to-show offset.  Later
/// samples may correct that offset only by `max_slew_us_per_sample`; the
/// published show time is clamped so it never steps backwards.  A stale peer
/// never keeps advancing output: the estimator enters `Stale` and holds the
/// last published time until an explicit Manual Fence re-arm resumes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShowClockEstimator {
    config: ShowClockEstimatorConfig,
    state: ShowClockEstimatorState,
    clock_generation: u64,
    fencing_generation: u64,
    offset_us: i64,
    last_received_at_us: Option<u64>,
    last_sample_expiry_us: Option<u64>,
    last_local_time_us: u64,
    last_show_time_us: u64,
    accepted_samples: u32,
}

impl ShowClockEstimator {
    pub fn new(
        config: ShowClockEstimatorConfig,
        clock_generation: u64,
        fencing_generation: u64,
    ) -> Result<Self, ShowClockValidationError> {
        config.validate()?;
        if clock_generation == 0 || fencing_generation == 0 {
            return Err(ShowClockValidationError::ZeroGeneration);
        }
        Ok(Self {
            config,
            state: ShowClockEstimatorState::Acquiring,
            clock_generation,
            fencing_generation,
            offset_us: 0,
            last_received_at_us: None,
            last_sample_expiry_us: None,
            last_local_time_us: 0,
            last_show_time_us: 0,
            accepted_samples: 0,
        })
    }

    pub fn config(&self) -> ShowClockEstimatorConfig {
        self.config
    }

    pub fn state(&self) -> ShowClockEstimatorState {
        self.state
    }

    pub fn generations(&self) -> (u64, u64) {
        (self.clock_generation, self.fencing_generation)
    }

    pub fn accepted_samples(&self) -> u32 {
        self.accepted_samples
    }

    pub fn offset_us(&self) -> i64 {
        self.offset_us
    }

    /// Incorporate a sample after its authenticated peer admission succeeded.
    pub fn observe(
        &mut self,
        sample: &ShowClockSample,
        received_at_us: u64,
    ) -> Result<(), ShowClockValidationError> {
        sample.validate_shape()?;
        if sample.clock_generation != self.clock_generation {
            return Err(ShowClockValidationError::ClockGenerationMismatch);
        }
        if sample.fencing_generation != self.fencing_generation {
            return Err(ShowClockValidationError::FencingGenerationMismatch);
        }
        if self
            .last_received_at_us
            .is_some_and(|last| received_at_us < last)
        {
            return Err(ShowClockValidationError::Reordered);
        }

        let raw_offset = signed_difference(sample.show_time_us, received_at_us);
        if self.last_received_at_us.is_none() {
            self.offset_us = raw_offset;
        } else {
            let delta = raw_offset.saturating_sub(self.offset_us);
            let limit = self.config.max_slew_us_per_sample as i64;
            self.offset_us = self.offset_us.saturating_add(delta.clamp(-limit, limit));
        }

        self.last_received_at_us = Some(received_at_us);
        self.last_sample_expiry_us = Some(sample.expires_after_us);
        self.accepted_samples = self.accepted_samples.saturating_add(1);
        self.last_local_time_us = self.last_local_time_us.max(received_at_us);
        let sample_time = apply_offset(received_at_us, self.offset_us);
        self.last_show_time_us = self.last_show_time_us.max(sample_time);
        if !matches!(
            self.state,
            ShowClockEstimatorState::Hold
                | ShowClockEstimatorState::Fault
                | ShowClockEstimatorState::Stale
        ) {
            self.state = if self.accepted_samples >= self.config.lock_after_samples {
                ShowClockEstimatorState::Locked
            } else {
                ShowClockEstimatorState::Acquiring
            };
        }
        Ok(())
    }

    /// Advance the estimate using caller-supplied local monotonic time.
    pub fn advance(&mut self, local_time_us: u64) -> ShowClockEstimate {
        let local_time_us = self.last_local_time_us.max(local_time_us);
        self.last_local_time_us = local_time_us;
        if let Some(received_at_us) = self.last_received_at_us {
            let stale_after_us = self
                .last_sample_expiry_us
                .unwrap_or(self.config.stale_after_us)
                .min(self.config.stale_after_us);
            if local_time_us.saturating_sub(received_at_us) > stale_after_us
                && matches!(
                    self.state,
                    ShowClockEstimatorState::Acquiring | ShowClockEstimatorState::Locked
                )
            {
                self.state = ShowClockEstimatorState::Stale;
            }
        }

        if matches!(
            self.state,
            ShowClockEstimatorState::Acquiring | ShowClockEstimatorState::Locked
        ) {
            self.last_show_time_us = self
                .last_show_time_us
                .max(apply_offset(local_time_us, self.offset_us));
        }
        self.snapshot(local_time_us)
    }

    pub fn enter_hold(&mut self) {
        self.state = ShowClockEstimatorState::Hold;
    }

    pub fn enter_fault(&mut self) {
        self.state = ShowClockEstimatorState::Fault;
    }

    /// Re-arm only from an explicitly armed fence carrying an advanced
    /// fencing generation.  A stale or faulted estimator must first be placed
    /// in Manual Hold; no network sample can arm it implicitly.
    pub fn rearm_from_fence(
        &mut self,
        fence: ShowClockManualFence,
    ) -> Result<(), ShowClockValidationError> {
        if self.state != ShowClockEstimatorState::Hold {
            return Err(ShowClockValidationError::HoldRequired);
        }
        if fence.state() != ShowClockFenceState::Armed {
            return Err(ShowClockValidationError::HoldRequired);
        }
        if fence.clock_generation() < self.clock_generation {
            return Err(ShowClockValidationError::ReArmClockGenerationRewound);
        }
        if fence.fencing_generation() <= self.fencing_generation {
            return Err(ShowClockValidationError::ReArmGenerationNotAdvanced);
        }
        self.clock_generation = fence.clock_generation();
        self.fencing_generation = fence.fencing_generation();
        self.state = ShowClockEstimatorState::Acquiring;
        self.accepted_samples = 0;
        self.last_received_at_us = None;
        self.last_sample_expiry_us = None;
        Ok(())
    }

    fn snapshot(&self, local_time_us: u64) -> ShowClockEstimate {
        ShowClockEstimate {
            state: self.state,
            show_time_us: self.last_show_time_us,
            offset_us: self.offset_us,
            sample_age_us: self
                .last_received_at_us
                .map(|received| local_time_us.saturating_sub(received)),
            accepted_samples: self.accepted_samples,
            clock_generation: self.clock_generation,
            fencing_generation: self.fencing_generation,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShowClockActionGeneration {
    pub clock_generation: u64,
    pub fencing_generation: u64,
}

impl ShowClockActionGeneration {
    pub fn new(
        clock_generation: u64,
        fencing_generation: u64,
    ) -> Result<Self, ShowClockValidationError> {
        if clock_generation == 0 || fencing_generation == 0 {
            return Err(ShowClockValidationError::ZeroGeneration);
        }
        Ok(Self {
            clock_generation,
            fencing_generation,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct QueuedShowClockAction {
    action: ShowClockAction,
    hold_notified: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShowClockActionDispatch {
    Execute(ShowClockAction),
    Dropped { action_id: [u8; 16] },
    Held { action_id: [u8; 16] },
}

/// Fixed-capacity, generation-bound action queue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShowClockActionScheduler {
    generation: ShowClockActionGeneration,
    horizon_us: u64,
    actions: BTreeMap<(u64, u64), QueuedShowClockAction>,
    action_ids: BTreeMap<[u8; 16], (u64, u64)>,
}

impl ShowClockActionScheduler {
    pub fn new(
        generation: ShowClockActionGeneration,
        horizon_us: u64,
    ) -> Result<Self, ShowClockValidationError> {
        if horizon_us == 0 {
            return Err(ShowClockValidationError::InvalidEstimatorConfig);
        }
        Ok(Self {
            generation,
            horizon_us,
            actions: BTreeMap::new(),
            action_ids: BTreeMap::new(),
        })
    }

    pub fn generation(&self) -> ShowClockActionGeneration {
        self.generation
    }

    pub fn len(&self) -> usize {
        self.actions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }

    pub fn schedule(
        &mut self,
        action: ShowClockAction,
        current_show_time_us: u64,
    ) -> Result<(), ShowClockValidationError> {
        action.validate_shape()?;
        self.validate_generation(&action)?;
        if action.target_show_time_us > current_show_time_us.saturating_add(self.horizon_us)
            || current_show_time_us.saturating_sub(action.target_show_time_us) > self.horizon_us
        {
            return Err(ShowClockValidationError::ActionOutsideHorizon);
        }
        if self.action_ids.contains_key(&action.action_id) {
            return Err(ShowClockValidationError::DuplicateAction);
        }
        if self.actions.len() >= SHOW_CLOCK_MAX_SCHEDULED_ACTIONS {
            return Err(ShowClockValidationError::ActionScheduleCapacityExceeded);
        }
        let key = (action.target_show_time_us, action.sequence);
        self.actions.insert(
            key,
            QueuedShowClockAction {
                action: action.clone(),
                hold_notified: false,
            },
        );
        self.action_ids.insert(action.action_id, key);
        Ok(())
    }

    /// Poll at most one action.  Only a locked estimator may dispatch; stale,
    /// held, or faulted state produces one observable hold transition per
    /// queued action and never emits output.
    pub fn poll(
        &mut self,
        current_show_time_us: u64,
        state: ShowClockEstimatorState,
    ) -> Option<ShowClockActionDispatch> {
        let key = self.actions.keys().next().copied()?;
        if key.0 > current_show_time_us {
            if state != ShowClockEstimatorState::Locked {
                return self.mark_held(key);
            }
            return None;
        }
        if state != ShowClockEstimatorState::Locked {
            return self.mark_held(key);
        }

        let queued = self.actions.remove(&key)?;
        self.action_ids.remove(&queued.action.action_id);
        if queued.action.target_show_time_us < current_show_time_us {
            match queued.action.late_policy {
                ShowClockLatePolicy::ExecuteImmediately => {
                    return Some(ShowClockActionDispatch::Execute(queued.action));
                }
                ShowClockLatePolicy::Drop => {
                    return Some(ShowClockActionDispatch::Dropped {
                        action_id: queued.action.action_id,
                    });
                }
                ShowClockLatePolicy::Hold if queued.hold_notified => {
                    return Some(ShowClockActionDispatch::Execute(queued.action));
                }
                ShowClockLatePolicy::Hold => {
                    self.reinsert_held(key, queued);
                    return Some(ShowClockActionDispatch::Held {
                        action_id: self.actions.get(&key)?.action.action_id,
                    });
                }
            }
        }
        Some(ShowClockActionDispatch::Execute(queued.action))
    }

    /// Poll through the output-ownership gate.  Authorization is checked
    /// before the queue is mutated, so a disarmed or stale owner leaves the
    /// action available for inspection after the fault is handled.
    pub fn poll_authorized(
        &mut self,
        current_show_time_us: u64,
        state: ShowClockEstimatorState,
        gate: &ShowClockOutputGate,
        owner: &ShowClockNodeId,
        context: ShowClockOutputContext,
    ) -> Result<Option<ShowClockActionDispatch>, ShowClockValidationError> {
        gate.authorize(owner, context, state)?;
        Ok(self.poll(current_show_time_us, state))
    }

    /// Replace the queue's generation only after the Manual Fence has been
    /// explicitly armed.  All old actions are discarded, preventing a prior
    /// project/fence from executing after re-arm.
    pub fn rebind_to_armed_fence(
        &mut self,
        fence: ShowClockManualFence,
    ) -> Result<usize, ShowClockValidationError> {
        if fence.state() != ShowClockFenceState::Armed {
            return Err(ShowClockValidationError::HoldRequired);
        }
        if fence.clock_generation() < self.generation.clock_generation {
            return Err(ShowClockValidationError::ReArmClockGenerationRewound);
        }
        if fence.fencing_generation() <= self.generation.fencing_generation {
            return Err(ShowClockValidationError::ReArmGenerationNotAdvanced);
        }
        self.generation =
            ShowClockActionGeneration::new(fence.clock_generation(), fence.fencing_generation())?;
        let removed = self.actions.len();
        self.actions.clear();
        self.action_ids.clear();
        Ok(removed)
    }

    fn validate_generation(
        &self,
        action: &ShowClockAction,
    ) -> Result<(), ShowClockValidationError> {
        if action.clock_generation != self.generation.clock_generation {
            return Err(ShowClockValidationError::ClockGenerationMismatch);
        }
        if action.fencing_generation != self.generation.fencing_generation {
            return Err(ShowClockValidationError::FencingGenerationMismatch);
        }
        Ok(())
    }

    fn mark_held(&mut self, key: (u64, u64)) -> Option<ShowClockActionDispatch> {
        let queued = self.actions.get_mut(&key)?;
        if queued.hold_notified {
            return None;
        }
        queued.hold_notified = true;
        Some(ShowClockActionDispatch::Held {
            action_id: queued.action.action_id,
        })
    }

    fn reinsert_held(&mut self, key: (u64, u64), mut queued: QueuedShowClockAction) {
        queued.hold_notified = true;
        self.action_ids.insert(queued.action.action_id, key);
        self.actions.insert(key, queued);
    }
}

/// Generation tuple captured by every output-producing ShowClock operation.
/// A project, lease, audio, or recording replacement invalidates the tuple;
/// a caller must explicitly arm the replacement before dispatch is possible.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShowClockOutputContext {
    pub project_generation: u64,
    pub lease_generation: u64,
    pub audio_generation: u64,
    pub recording_generation: u64,
    pub clock_generation: u64,
    pub fencing_generation: u64,
    pub project_hash: ShowClockHash,
    pub media_hash: ShowClockHash,
}

impl ShowClockOutputContext {
    pub fn new(
        project_generation: u64,
        lease_generation: u64,
        audio_generation: u64,
        recording_generation: u64,
        clock_generation: u64,
        fencing_generation: u64,
        project_hash: ShowClockHash,
        media_hash: ShowClockHash,
    ) -> Result<Self, ShowClockValidationError> {
        if [
            project_generation,
            lease_generation,
            audio_generation,
            recording_generation,
            clock_generation,
            fencing_generation,
        ]
        .into_iter()
        .any(|generation| generation == 0)
            || project_hash.is_zero()
            || media_hash.is_zero()
        {
            return Err(ShowClockValidationError::ZeroGeneration);
        }
        Ok(Self {
            project_generation,
            lease_generation,
            audio_generation,
            recording_generation,
            clock_generation,
            fencing_generation,
            project_hash,
            media_hash,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShowClockOutputGate {
    owner: ShowClockNodeId,
    context: ShowClockOutputContext,
    armed: bool,
}

impl ShowClockOutputGate {
    pub fn new(
        owner: ShowClockNodeId,
        context: ShowClockOutputContext,
    ) -> Result<Self, ShowClockValidationError> {
        Ok(Self {
            owner,
            context,
            armed: false,
        })
    }

    pub fn is_armed(&self) -> bool {
        self.armed
    }

    pub fn context(&self) -> ShowClockOutputContext {
        self.context
    }

    pub fn arm(
        &mut self,
        fence: ShowClockManualFence,
        context: ShowClockOutputContext,
    ) -> Result<(), ShowClockValidationError> {
        if self.armed {
            return Err(ShowClockValidationError::AlreadyArmed);
        }
        if fence.state() != ShowClockFenceState::Armed {
            return Err(ShowClockValidationError::HoldRequired);
        }
        if context != self.context
            || fence.clock_generation() != context.clock_generation
            || fence.fencing_generation() != context.fencing_generation
        {
            return Err(ShowClockValidationError::OutputGenerationMismatch);
        }
        self.armed = true;
        Ok(())
    }

    /// Project/lease/audio/recording replacement disarms before adopting its
    /// new context.  This is intentionally not an automatic re-arm.
    pub fn replace_context(
        &mut self,
        context: ShowClockOutputContext,
    ) -> Result<(), ShowClockValidationError> {
        self.context = context;
        self.armed = false;
        Ok(())
    }

    pub fn disarm(&mut self) {
        self.armed = false;
    }

    pub fn authorize(
        &self,
        owner: &ShowClockNodeId,
        context: ShowClockOutputContext,
        state: ShowClockEstimatorState,
    ) -> Result<(), ShowClockValidationError> {
        if !self.armed {
            return Err(ShowClockValidationError::OutputNotArmed);
        }
        if owner != &self.owner {
            return Err(ShowClockValidationError::OutputOwnerMismatch);
        }
        if context != self.context {
            return Err(ShowClockValidationError::OutputGenerationMismatch);
        }
        if state != ShowClockEstimatorState::Locked {
            return Err(ShowClockValidationError::HoldRequired);
        }
        Ok(())
    }
}

/// Small convenience owner for callers that want the authenticated peer
/// admission and deterministic estimator in one explicit boundary.  Action
/// admission remains separate because the caller must choose the scheduling
/// horizon/current show time before mutating its replay receiver.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShowClockPeerEstimator {
    estimator: ShowClockEstimator,
}

impl ShowClockPeerEstimator {
    pub fn new(
        config: ShowClockEstimatorConfig,
        clock_generation: u64,
        fencing_generation: u64,
    ) -> Result<Self, ShowClockValidationError> {
        Ok(Self {
            estimator: ShowClockEstimator::new(config, clock_generation, fencing_generation)?,
        })
    }

    pub fn estimator(&self) -> &ShowClockEstimator {
        &self.estimator
    }

    pub fn estimator_mut(&mut self) -> &mut ShowClockEstimator {
        &mut self.estimator
    }

    pub fn accept_authenticated_sample(
        &mut self,
        validator: &mut ShowClockPeerValidator,
        sample: &crate::show_clock::AuthenticatedShowClockSample,
        received_at_us: u64,
    ) -> Result<(), ShowClockValidationError> {
        if self
            .estimator
            .last_received_at_us
            .is_some_and(|last| received_at_us < last)
        {
            return Err(ShowClockValidationError::Reordered);
        }
        validator.accept(sample)?;
        self.estimator.observe(&sample.body, received_at_us)
    }
}

fn signed_difference(remote: u64, local: u64) -> i64 {
    if remote >= local {
        remote.saturating_sub(local).min(i64::MAX as u64) as i64
    } else {
        -(local.saturating_sub(remote).min(i64::MAX as u64) as i64)
    }
}

fn apply_offset(local: u64, offset: i64) -> u64 {
    if offset >= 0 {
        local.saturating_add(offset as u64)
    } else {
        local.saturating_sub(offset.unsigned_abs())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::show_clock::{
        AuthenticatedShowClockSample, ShowClockActionKind, ShowClockHash, ShowClockNodeId,
        ShowClockNonce, ShowClockSessionId, ShowClockSource, ShowTransportState,
        SHOW_CLOCK_PROTOCOL_VERSION, SHOW_CLOCK_SCHEMA_VERSION,
    };

    const KEY: [u8; 32] = [0x42; 32];
    const PROJECT: ShowClockHash = ShowClockHash([0x11; 32]);
    const MEDIA: ShowClockHash = ShowClockHash([0x22; 32]);

    fn sample(sequence: u64, show_time_us: u64) -> ShowClockSample {
        ShowClockSample {
            schema_version: SHOW_CLOCK_SCHEMA_VERSION,
            protocol_version: SHOW_CLOCK_PROTOCOL_VERSION,
            session_id: ShowClockSessionId::new("session-1").unwrap(),
            sender: ShowClockNodeId::new("node-primary").unwrap(),
            clock_generation: 1,
            fencing_generation: 1,
            sequence,
            sender_monotonic_us: show_time_us,
            show_time_us,
            bpm_milli: 120_000,
            beat_phase_ppm: 0,
            transport: ShowTransportState::Playing,
            source: ShowClockSource::ShowClock,
            project_hash: PROJECT,
            media_hash: MEDIA,
            expires_after_us: 250_000,
            nonce: ShowClockNonce([((sequence % 255) + 1) as u8; 16]),
        }
    }

    fn action(sequence: u64, target: u64, late_policy: ShowClockLatePolicy) -> ShowClockAction {
        let mut action_id = [0_u8; 16];
        action_id[..8].copy_from_slice(&sequence.to_le_bytes());
        ShowClockAction {
            schema_version: SHOW_CLOCK_SCHEMA_VERSION,
            protocol_version: SHOW_CLOCK_PROTOCOL_VERSION,
            session_id: ShowClockSessionId::new("session-1").unwrap(),
            sender: ShowClockNodeId::new("node-primary").unwrap(),
            action_id,
            sequence,
            clock_generation: 1,
            fencing_generation: 1,
            target_show_time_us: target,
            action: ShowClockActionKind::Go,
            late_policy,
            payload: None,
            project_hash: PROJECT,
            media_hash: MEDIA,
        }
    }

    #[test]
    fn estimator_bounds_correction_and_never_steps_show_time_backwards() {
        let config = ShowClockEstimatorConfig {
            max_slew_us_per_sample: 1_000,
            stale_after_us: 750_000,
            lock_after_samples: 3,
        };
        let mut estimator = ShowClockEstimator::new(config, 1, 1).unwrap();
        estimator.observe(&sample(1, 1_000_000), 1_000_000).unwrap();
        let first = estimator.advance(1_000_000);
        estimator.observe(&sample(2, 1_010_000), 1_000_000).unwrap();
        assert_eq!(estimator.offset_us(), 1_000);
        let second = estimator.advance(1_000_000);
        assert!(second.show_time_us >= first.show_time_us);
        estimator.observe(&sample(3, 999_000), 1_000_000).unwrap();
        let third = estimator.advance(1_000_000);
        assert!(third.show_time_us >= second.show_time_us);
        assert_eq!(third.state, ShowClockEstimatorState::Locked);
    }

    #[test]
    fn stale_enters_hold_and_only_explicit_armed_fence_resumes() {
        let mut estimator = ShowClockEstimator::new(Default::default(), 1, 1).unwrap();
        estimator.observe(&sample(1, 1_000_000), 1_000_000).unwrap();
        estimator.observe(&sample(2, 1_250_000), 1_250_000).unwrap();
        estimator.observe(&sample(3, 1_500_000), 1_500_000).unwrap();
        assert_eq!(
            estimator.advance(2_250_001).state,
            ShowClockEstimatorState::Stale
        );
        let held_time = estimator.advance(4_000_000).show_time_us;
        estimator.enter_hold();
        assert_eq!(estimator.advance(5_000_000).show_time_us, held_time);

        let mut fence = ShowClockManualFence::new(1, 1).unwrap();
        fence.enter_hold();
        fence
            .rearm(crate::show_clock::ShowClockReArm {
                operator_confirmed_primary_stopped: true,
                clock_generation: 1,
                fencing_generation: 2,
            })
            .unwrap();
        estimator.rearm_from_fence(fence).unwrap();
        assert_eq!(estimator.state(), ShowClockEstimatorState::Acquiring);
        assert_eq!(estimator.generations(), (1, 2));
    }

    #[test]
    fn authenticated_estimator_keeps_authentication_before_clock_policy() {
        let mut validator = ShowClockPeerValidator::new(
            ShowClockSessionId::new("session-1").unwrap(),
            ShowClockNodeId::new("node-primary").unwrap(),
            PROJECT,
            MEDIA,
            1,
            1,
            KEY,
        )
        .unwrap();
        let mut peer = ShowClockPeerEstimator::new(Default::default(), 1, 1).unwrap();
        let signed = AuthenticatedShowClockSample::sign(sample(1, 1_000_000), &KEY).unwrap();
        peer.accept_authenticated_sample(&mut validator, &signed, 1_000_050)
            .unwrap();
        let mut tampered = signed;
        tampered.body.show_time_us += 1;
        assert_eq!(
            peer.accept_authenticated_sample(&mut validator, &tampered, 1_000_100),
            Err(ShowClockValidationError::InvalidAuthentication)
        );
    }

    #[test]
    fn scheduler_applies_horizon_late_policy_and_generation_fence() {
        let generation = ShowClockActionGeneration::new(1, 1).unwrap();
        let mut scheduler = ShowClockActionScheduler::new(generation, 10_000).unwrap();
        assert_eq!(
            scheduler.schedule(action(1, 20_001, ShowClockLatePolicy::Drop), 10_000),
            Err(ShowClockValidationError::ActionOutsideHorizon)
        );
        scheduler
            .schedule(action(2, 9_000, ShowClockLatePolicy::Drop), 10_000)
            .unwrap();
        assert_eq!(
            scheduler.poll(10_000, ShowClockEstimatorState::Locked),
            Some(ShowClockActionDispatch::Dropped {
                action_id: {
                    let mut id = [0_u8; 16];
                    id[..8].copy_from_slice(&2_u64.to_le_bytes());
                    id
                }
            })
        );
        scheduler
            .schedule(
                action(3, 10_100, ShowClockLatePolicy::ExecuteImmediately),
                10_000,
            )
            .unwrap();
        assert!(matches!(
            scheduler.poll(10_100, ShowClockEstimatorState::Stale),
            Some(ShowClockActionDispatch::Held { .. })
        ));
        assert_eq!(scheduler.len(), 1);
    }

    #[test]
    fn scheduler_rebind_discards_old_generation_actions() {
        let generation = ShowClockActionGeneration::new(1, 1).unwrap();
        let mut scheduler = ShowClockActionScheduler::new(generation, 10_000).unwrap();
        scheduler
            .schedule(action(1, 10_100, ShowClockLatePolicy::Drop), 10_000)
            .unwrap();
        let mut fence = ShowClockManualFence::new(1, 1).unwrap();
        fence.enter_hold();
        fence
            .rearm(crate::show_clock::ShowClockReArm {
                operator_confirmed_primary_stopped: true,
                clock_generation: 2,
                fencing_generation: 2,
            })
            .unwrap();
        assert_eq!(scheduler.rebind_to_armed_fence(fence), Ok(1));
        assert!(scheduler.is_empty());
        assert_eq!(scheduler.generation().clock_generation, 2);
    }

    #[test]
    fn output_gate_requires_all_generations_and_blocks_faulted_dispatch() {
        let context = ShowClockOutputContext::new(1, 1, 1, 1, 1, 2, PROJECT, MEDIA).unwrap();
        let owner = ShowClockNodeId::new("node-standby").unwrap();
        let mut gate = ShowClockOutputGate::new(owner.clone(), context).unwrap();
        let mut fence = ShowClockManualFence::new(1, 1).unwrap();
        fence.enter_hold();
        fence
            .rearm(crate::show_clock::ShowClockReArm {
                operator_confirmed_primary_stopped: true,
                clock_generation: 1,
                fencing_generation: 2,
            })
            .unwrap();
        assert_eq!(gate.arm(fence, context), Ok(()));
        assert_eq!(
            gate.authorize(&owner, context, ShowClockEstimatorState::Stale),
            Err(ShowClockValidationError::HoldRequired)
        );
        let mut scheduler =
            ShowClockActionScheduler::new(ShowClockActionGeneration::new(1, 2).unwrap(), 10_000)
                .unwrap();
        let action = action(1, 10_100, ShowClockLatePolicy::Drop);
        let mut action = action;
        action.fencing_generation = 2;
        scheduler.schedule(action, 10_000).unwrap();
        let mut wrong_context = context;
        wrong_context.audio_generation = 2;
        assert_eq!(
            scheduler.poll_authorized(
                10_200,
                ShowClockEstimatorState::Locked,
                &gate,
                &owner,
                wrong_context,
            ),
            Err(ShowClockValidationError::OutputGenerationMismatch)
        );
        assert_eq!(scheduler.len(), 1);
        assert_eq!(
            scheduler
                .poll_authorized(
                    10_200,
                    ShowClockEstimatorState::Locked,
                    &gate,
                    &owner,
                    context,
                )
                .unwrap(),
            Some(ShowClockActionDispatch::Dropped {
                action_id: {
                    let mut id = [0_u8; 16];
                    id[..8].copy_from_slice(&1_u64.to_le_bytes());
                    id
                }
            })
        );
    }

    #[test]
    fn deterministic_fault_soak_is_bounded_and_monotonic() {
        let mut estimator = ShowClockEstimator::new(Default::default(), 1, 1).unwrap();
        let mut previous = 0;
        for sequence in 1..=10_000 {
            let now = sequence * 20_000;
            let remote = now + if sequence % 2 == 0 { 2_000 } else { 0 };
            estimator.observe(&sample(sequence, remote), now).unwrap();
            let current = estimator.advance(now).show_time_us;
            assert!(current >= previous);
            previous = current;
        }
        assert_eq!(estimator.accepted_samples(), 10_000);
        assert_ne!(estimator.state(), ShowClockEstimatorState::Fault);
    }

    #[test]
    fn stale_estimator_requires_hold_and_rearm_before_relocking() {
        let mut estimator = ShowClockEstimator::new(Default::default(), 1, 1).unwrap();
        for sequence in 1..=3 {
            estimator
                .observe(&sample(sequence, 1_000_000 + sequence), 1_000_000 + sequence)
                .unwrap();
        }
        assert_eq!(estimator.state(), ShowClockEstimatorState::Locked);
        estimator.advance(1_751_000);
        assert_eq!(estimator.state(), ShowClockEstimatorState::Stale);

        estimator.observe(&sample(4, 1_751_004), 1_751_004).unwrap();
        assert_eq!(estimator.state(), ShowClockEstimatorState::Stale);
        estimator.enter_hold();
        let mut fence = ShowClockManualFence::new(1, 1).unwrap();
        fence.enter_hold();
        fence
            .rearm(crate::show_clock::ShowClockReArm {
                operator_confirmed_primary_stopped: true,
                clock_generation: 1,
                fencing_generation: 2,
            })
            .unwrap();
        estimator.rearm_from_fence(fence).unwrap();
        assert_eq!(estimator.state(), ShowClockEstimatorState::Acquiring);
    }

    #[test]
    fn authenticated_estimator_rejects_reordered_receive_time_atomically() {
        let mut validator = ShowClockPeerValidator::new(
            ShowClockSessionId::new("session-1").unwrap(),
            ShowClockNodeId::new("node-primary").unwrap(),
            PROJECT,
            MEDIA,
            1,
            1,
            KEY,
        )
        .unwrap();
        let mut peer = ShowClockPeerEstimator::new(Default::default(), 1, 1).unwrap();
        let first = AuthenticatedShowClockSample::sign(sample(1, 1_000_000), &KEY).unwrap();
        peer.accept_authenticated_sample(&mut validator, &first, 1_000_000)
            .unwrap();
        let second = AuthenticatedShowClockSample::sign(sample(2, 1_001_000), &KEY).unwrap();
        assert_eq!(
            peer.accept_authenticated_sample(&mut validator, &second, 999_000),
            Err(ShowClockValidationError::Reordered)
        );
        assert_eq!(validator.last_sequence(), 1);
        assert_eq!(peer.estimator().accepted_samples(), 1);
    }
}
