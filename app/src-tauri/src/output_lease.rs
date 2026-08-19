//! Pure AI3 output-lease authority transitions.
//!
//! This module deliberately has no Engine, output-worker, Blackout, role, or
//! Tauri dependency. AppState wiring, receipts, admission/rate limits, consent,
//! and the final R4 commit-boundary revalidation belong to the next slice.

const MAX_OUTPUT_LEASE_TTL_MS: u64 = 60_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum OutputLeaseResource {
    Lighting,
    Video,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OutputLeaseResources(Vec<OutputLeaseResource>);

impl OutputLeaseResources {
    pub(crate) fn new(resources: &[OutputLeaseResource]) -> Result<Self, OutputLeaseError> {
        if resources.is_empty() {
            return Err(OutputLeaseError::InvalidResources);
        }
        let mut canonical = resources.to_vec();
        canonical.sort_unstable();
        if canonical.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(OutputLeaseError::InvalidResources);
        }
        Ok(Self(canonical))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OutputLeaseOwner {
    principal: String,
    window_label: String,
    process_session_incarnation: u64,
    owner_incarnation: u64,
}

impl OutputLeaseOwner {
    pub(crate) fn new(
        principal: impl Into<String>,
        window_label: impl Into<String>,
        process_session_incarnation: u64,
        owner_incarnation: u64,
    ) -> Result<Self, OutputLeaseError> {
        let principal = principal.into();
        let window_label = window_label.into();
        if principal.trim().is_empty()
            || window_label.trim().is_empty()
            || process_session_incarnation == 0
            || owner_incarnation == 0
        {
            return Err(OutputLeaseError::InvalidOwner);
        }
        Ok(Self {
            principal,
            window_label,
            process_session_incarnation,
            owner_incarnation,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OutputLeasePhase {
    Unclaimed,
    HeldActive,
    HeldOrphaned,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OutputLeaseSnapshot {
    pub(crate) phase: OutputLeasePhase,
    pub(crate) owner: Option<OutputLeaseOwner>,
    pub(crate) resources: Option<OutputLeaseResources>,
    pub(crate) generation: u64,
    pub(crate) expires_at_monotonic_ms: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OutputLeaseError {
    InvalidOwner,
    InvalidResources,
    InvalidTtl,
    InvalidTransition,
    StaleOwner,
    StaleGeneration,
    Expired,
    GenerationExhausted,
    ClockExhausted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OutputLeaseState {
    process_session_incarnation: u64,
    snapshot: OutputLeaseSnapshot,
}

impl OutputLeaseState {
    /// A process restart never reconstructs authority from cached state.
    pub(crate) fn fresh_process(
        process_session_incarnation: u64,
    ) -> Result<Self, OutputLeaseError> {
        if process_session_incarnation == 0 {
            return Err(OutputLeaseError::InvalidOwner);
        }
        Ok(Self {
            process_session_incarnation,
            snapshot: OutputLeaseSnapshot {
                phase: OutputLeasePhase::Unclaimed,
                owner: None,
                resources: None,
                generation: 0,
                expires_at_monotonic_ms: None,
            },
        })
    }

    pub(crate) fn snapshot(&self) -> OutputLeaseSnapshot {
        self.snapshot.clone()
    }

    pub(crate) fn acquire(
        &mut self,
        owner: OutputLeaseOwner,
        resources: OutputLeaseResources,
        now_ms: u64,
        ttl_ms: u64,
    ) -> Result<OutputLeaseSnapshot, OutputLeaseError> {
        if self.snapshot.phase != OutputLeasePhase::Unclaimed {
            return Err(OutputLeaseError::InvalidTransition);
        }
        self.require_current_process(&owner)?;
        let generation = next_generation(self.snapshot.generation)?;
        let deadline = checked_deadline(now_ms, ttl_ms)?;
        self.snapshot = OutputLeaseSnapshot {
            phase: OutputLeasePhase::HeldActive,
            owner: Some(owner),
            resources: Some(resources),
            generation,
            expires_at_monotonic_ms: Some(deadline),
        };
        Ok(self.snapshot())
    }

    pub(crate) fn renew(
        &mut self,
        owner: &OutputLeaseOwner,
        expected_generation: u64,
        now_ms: u64,
        ttl_ms: u64,
    ) -> Result<OutputLeaseSnapshot, OutputLeaseError> {
        self.require_generation(expected_generation)?;
        self.require_owner(owner)?;
        if self.snapshot.phase != OutputLeasePhase::HeldActive {
            return Err(OutputLeaseError::InvalidTransition);
        }
        if now_ms >= self.snapshot.expires_at_monotonic_ms.unwrap_or(0) {
            self.orphan_without_owner_check()?;
            return Err(OutputLeaseError::Expired);
        }
        let generation = next_generation(self.snapshot.generation)?;
        let deadline = checked_deadline(now_ms, ttl_ms)?;
        self.snapshot.generation = generation;
        self.snapshot.expires_at_monotonic_ms = Some(deadline);
        Ok(self.snapshot())
    }

    /// Observing expiry changes authority only. There is no physical-output API here.
    pub(crate) fn observe_expiry(&mut self, now_ms: u64) -> Result<bool, OutputLeaseError> {
        if self.snapshot.phase != OutputLeasePhase::HeldActive
            || now_ms < self.snapshot.expires_at_monotonic_ms.unwrap_or(u64::MAX)
        {
            return Ok(false);
        }
        self.orphan_without_owner_check()?;
        Ok(true)
    }

    /// Used by owner retirement and project-identity replacement.
    pub(crate) fn orphan(
        &mut self,
        expected_generation: u64,
    ) -> Result<OutputLeaseSnapshot, OutputLeaseError> {
        self.require_generation(expected_generation)?;
        if self.snapshot.phase != OutputLeasePhase::HeldActive {
            return Err(OutputLeaseError::InvalidTransition);
        }
        self.orphan_without_owner_check()?;
        Ok(self.snapshot())
    }

    pub(crate) fn recover(
        &mut self,
        owner: &OutputLeaseOwner,
        expected_generation: u64,
        now_ms: u64,
        ttl_ms: u64,
    ) -> Result<OutputLeaseSnapshot, OutputLeaseError> {
        self.require_generation(expected_generation)?;
        self.require_owner(owner)?;
        if self.snapshot.phase != OutputLeasePhase::HeldOrphaned {
            return Err(OutputLeaseError::InvalidTransition);
        }
        let generation = next_generation(self.snapshot.generation)?;
        let deadline = checked_deadline(now_ms, ttl_ms)?;
        self.snapshot.phase = OutputLeasePhase::HeldActive;
        self.snapshot.generation = generation;
        self.snapshot.expires_at_monotonic_ms = Some(deadline);
        Ok(self.snapshot())
    }

    pub(crate) fn relinquish_output_lease(
        &mut self,
        owner: &OutputLeaseOwner,
        expected_generation: u64,
    ) -> Result<OutputLeaseSnapshot, OutputLeaseError> {
        self.require_generation(expected_generation)?;
        self.require_owner(owner)?;
        if !matches!(
            self.snapshot.phase,
            OutputLeasePhase::HeldActive | OutputLeasePhase::HeldOrphaned
        ) {
            return Err(OutputLeaseError::InvalidTransition);
        }
        let generation = next_generation(self.snapshot.generation)?;
        self.snapshot = OutputLeaseSnapshot {
            phase: OutputLeasePhase::Unclaimed,
            owner: None,
            resources: None,
            generation,
            expires_at_monotonic_ms: None,
        };
        Ok(self.snapshot())
    }

    /// Atomic authority-only transfer. The pending phase is built on a clone and
    /// never becomes externally visible if deadline/generation validation fails.
    pub(crate) fn force_transfer(
        &mut self,
        expected_generation: u64,
        new_owner: OutputLeaseOwner,
        now_ms: u64,
        ttl_ms: u64,
    ) -> Result<OutputLeaseSnapshot, OutputLeaseError> {
        self.require_generation(expected_generation)?;
        self.require_current_process(&new_owner)?;
        if self.snapshot.phase == OutputLeasePhase::HeldActive
            && now_ms >= self.snapshot.expires_at_monotonic_ms.unwrap_or(0)
        {
            self.orphan_without_owner_check()?;
            return Err(OutputLeaseError::Expired);
        }
        if !matches!(
            self.snapshot.phase,
            OutputLeasePhase::HeldActive | OutputLeasePhase::HeldOrphaned
        ) || self.snapshot.owner.as_ref() == Some(&new_owner)
        {
            return Err(OutputLeaseError::InvalidTransition);
        }
        let generation = next_generation(self.snapshot.generation)?;
        let deadline = checked_deadline(now_ms, ttl_ms)?;
        let mut candidate = self.snapshot.clone();
        candidate.phase = OutputLeasePhase::HeldActive;
        candidate.owner = Some(new_owner);
        candidate.generation = generation;
        candidate.expires_at_monotonic_ms = Some(deadline);
        self.snapshot = candidate;
        Ok(self.snapshot())
    }

    fn orphan_without_owner_check(&mut self) -> Result<(), OutputLeaseError> {
        let generation = next_generation(self.snapshot.generation)?;
        self.snapshot.phase = OutputLeasePhase::HeldOrphaned;
        self.snapshot.generation = generation;
        self.snapshot.expires_at_monotonic_ms = None;
        Ok(())
    }

    fn require_owner(&self, owner: &OutputLeaseOwner) -> Result<(), OutputLeaseError> {
        if self.snapshot.owner.as_ref() == Some(owner) {
            Ok(())
        } else {
            Err(OutputLeaseError::StaleOwner)
        }
    }

    fn require_current_process(&self, owner: &OutputLeaseOwner) -> Result<(), OutputLeaseError> {
        if owner.process_session_incarnation == self.process_session_incarnation {
            Ok(())
        } else {
            Err(OutputLeaseError::StaleOwner)
        }
    }

    fn require_generation(&self, expected: u64) -> Result<(), OutputLeaseError> {
        if self.snapshot.generation == expected {
            Ok(())
        } else {
            Err(OutputLeaseError::StaleGeneration)
        }
    }
}

fn next_generation(current: u64) -> Result<u64, OutputLeaseError> {
    current
        .checked_add(1)
        .ok_or(OutputLeaseError::GenerationExhausted)
}

fn checked_deadline(now_ms: u64, ttl_ms: u64) -> Result<u64, OutputLeaseError> {
    if ttl_ms == 0 || ttl_ms > MAX_OUTPUT_LEASE_TTL_MS {
        return Err(OutputLeaseError::InvalidTtl);
    }
    now_ms
        .checked_add(ttl_ms)
        .ok_or(OutputLeaseError::ClockExhausted)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn owner(process: u64, incarnation: u64) -> OutputLeaseOwner {
        OutputLeaseOwner::new("local-ui", "main", process, incarnation).unwrap()
    }

    fn both() -> OutputLeaseResources {
        OutputLeaseResources::new(&[OutputLeaseResource::Video, OutputLeaseResource::Lighting])
            .unwrap()
    }

    #[test]
    fn output_lease_validates_owner_and_canonical_resources() {
        assert_eq!(
            OutputLeaseOwner::new("", "main", 1, 1),
            Err(OutputLeaseError::InvalidOwner)
        );
        assert_eq!(
            OutputLeaseResources::new(&[]),
            Err(OutputLeaseError::InvalidResources)
        );
        assert_eq!(
            OutputLeaseResources::new(&[
                OutputLeaseResource::Lighting,
                OutputLeaseResource::Lighting,
            ]),
            Err(OutputLeaseError::InvalidResources)
        );
        assert_eq!(
            both().0,
            vec![OutputLeaseResource::Lighting, OutputLeaseResource::Video]
        );
    }

    #[test]
    fn output_lease_acquire_renew_expire_and_recover_are_generation_fenced() {
        let mut state = OutputLeaseState::fresh_process(7).unwrap();
        let owner = owner(7, 11);
        let acquired = state.acquire(owner.clone(), both(), 100, 50).unwrap();
        assert_eq!(acquired.generation, 1);
        assert_eq!(acquired.expires_at_monotonic_ms, Some(150));
        let renewed = state.renew(&owner, 1, 120, 50).unwrap();
        assert_eq!(renewed.generation, 2);
        assert!(!state.observe_expiry(169).unwrap());
        assert!(state.observe_expiry(170).unwrap());
        assert_eq!(state.snapshot.phase, OutputLeasePhase::HeldOrphaned);
        let recovered = state.recover(&owner, 3, 200, 50).unwrap();
        assert_eq!(recovered.generation, 4);
        assert_eq!(recovered.phase, OutputLeasePhase::HeldActive);
    }

    #[test]
    fn output_lease_rejects_owner_incarnation_aba_without_mutation() {
        let mut state = OutputLeaseState::fresh_process(9).unwrap();
        let current = owner(9, 1);
        state.acquire(current.clone(), both(), 0, 10).unwrap();
        let before = state.snapshot();
        assert_eq!(
            state.renew(&owner(9, 2), 1, 1, 10),
            Err(OutputLeaseError::StaleOwner)
        );
        assert_eq!(state.snapshot(), before);
        assert_eq!(
            state.renew(&owner(10, 1), 1, 1, 10),
            Err(OutputLeaseError::StaleOwner)
        );
        assert_eq!(state.snapshot(), before);
    }

    #[test]
    fn output_lease_relinquish_and_restart_do_not_reclaim_authority() {
        let mut state = OutputLeaseState::fresh_process(3).unwrap();
        let owner = owner(3, 4);
        state.acquire(owner.clone(), both(), 0, 10).unwrap();
        let released = state.relinquish_output_lease(&owner, 1).unwrap();
        assert_eq!(released.phase, OutputLeasePhase::Unclaimed);
        assert!(released.owner.is_none());
        let restarted = OutputLeaseState::fresh_process(99).unwrap();
        assert_eq!(restarted.snapshot().phase, OutputLeasePhase::Unclaimed);
        assert_eq!(restarted.snapshot().generation, 0);
    }

    #[test]
    fn output_lease_force_transfer_is_atomic_and_authority_only() {
        let mut state = OutputLeaseState::fresh_process(1).unwrap();
        let old = owner(1, 1);
        let new = owner(1, 2);
        state.acquire(old, both(), 5, 10).unwrap();
        let before = state.snapshot();
        assert_eq!(
            state.force_transfer(2, new.clone(), 6, 10),
            Err(OutputLeaseError::StaleGeneration)
        );
        assert_eq!(state.snapshot(), before);
        let transferred = state.force_transfer(1, new.clone(), 6, 10).unwrap();
        assert_eq!(transferred.owner, Some(new));
        assert_eq!(transferred.resources, before.resources);
        assert_eq!(transferred.generation, 2);
        assert_eq!(transferred.phase, OutputLeasePhase::HeldActive);
    }

    #[test]
    fn output_lease_project_orphan_and_invalid_requests_are_atomic() {
        let mut state = OutputLeaseState::fresh_process(2).unwrap();
        let owner = owner(2, 3);
        state.acquire(owner.clone(), both(), 10, 10).unwrap();
        let orphaned = state.orphan(1).unwrap();
        assert_eq!(orphaned.phase, OutputLeasePhase::HeldOrphaned);
        assert_eq!(orphaned.generation, 2);
        let before = state.snapshot();
        assert_eq!(
            state.recover(&owner, 2, u64::MAX, 1),
            Err(OutputLeaseError::ClockExhausted)
        );
        assert_eq!(state.snapshot(), before);
        assert_eq!(
            state.recover(&owner, 2, 0, MAX_OUTPUT_LEASE_TTL_MS + 1),
            Err(OutputLeaseError::InvalidTtl)
        );
        assert_eq!(state.snapshot(), before);
    }

    #[test]
    fn output_lease_restart_rejects_cached_owner_and_overflow_is_atomic() {
        let cached_owner = owner(1, 5);
        let mut restarted = OutputLeaseState::fresh_process(2).unwrap();
        let before = restarted.snapshot();
        assert_eq!(
            restarted.acquire(cached_owner, both(), 0, 10),
            Err(OutputLeaseError::StaleOwner)
        );
        assert_eq!(restarted.snapshot(), before);

        restarted.snapshot.generation = u64::MAX;
        let exhausted = restarted.snapshot();
        assert_eq!(
            restarted.acquire(owner(2, 1), both(), 0, 10),
            Err(OutputLeaseError::GenerationExhausted)
        );
        assert_eq!(restarted.snapshot(), exhausted);
    }

    #[test]
    fn output_lease_renew_at_deadline_observes_orphaning() {
        let mut state = OutputLeaseState::fresh_process(4).unwrap();
        let owner = owner(4, 1);
        state.acquire(owner.clone(), both(), 10, 5).unwrap();
        assert_eq!(
            state.renew(&owner, 1, 15, 5),
            Err(OutputLeaseError::Expired)
        );
        assert_eq!(state.snapshot().phase, OutputLeasePhase::HeldOrphaned);
        assert_eq!(state.snapshot().generation, 2);
    }

    #[test]
    fn output_lease_transfer_at_deadline_observes_expiry_before_transfer() {
        let mut state = OutputLeaseState::fresh_process(6).unwrap();
        state.acquire(owner(6, 1), both(), 20, 5).unwrap();
        assert_eq!(
            state.force_transfer(1, owner(6, 2), 25, 5),
            Err(OutputLeaseError::Expired)
        );
        let orphaned = state.snapshot();
        assert_eq!(orphaned.phase, OutputLeasePhase::HeldOrphaned);
        assert_eq!(orphaned.generation, 2);
        assert_eq!(orphaned.owner, Some(owner(6, 1)));
        let transferred = state.force_transfer(2, owner(6, 2), 25, 5).unwrap();
        assert_eq!(transferred.generation, 3);
        assert_eq!(transferred.phase, OutputLeasePhase::HeldActive);
    }

    #[test]
    fn output_lease_transfer_rejects_a_different_process_without_mutation() {
        let mut state = OutputLeaseState::fresh_process(8).unwrap();
        state.acquire(owner(8, 1), both(), 0, 10).unwrap();
        let before = state.snapshot();
        assert_eq!(
            state.force_transfer(1, owner(9, 1), 1, 5),
            Err(OutputLeaseError::StaleOwner)
        );
        assert_eq!(state.snapshot(), before);
    }
}
