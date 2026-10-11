use super::*;

// Physical workers have joined, but a frame admitted immediately before the
// reservation may still own a permit. Keep the same reservation until that
// permit drains; neither retry the reservation nor allow a new activation.
pub(super) fn finish_retirement_with_timeout(
    mut retirement: OutputOwnershipRetirement,
    timeout: Duration,
) -> Result<OutputOwnershipTransition, String> {
    {
        let gate = &retirement.transition.gate.inner;
        let mut state = gate
            .state
            .lock()
            .map_err(|_| "Output ownership gate lock was poisoned".to_string())?;
        let deadline = Instant::now() + timeout;
        loop {
            if !state.transition_active
                || state.status.state != OutputOwnershipState::Transitioning
                || state.status.epoch != retirement.epoch
                || state.status.desired_role != retirement.transition.target
            {
                // This owner cannot fail or rewrite a newer reservation/fault.
                retirement.transition.finished = true;
                return Err("Project output retirement reservation was superseded".to_string());
            }
            if state.in_flight == 0 {
                break;
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                let error =
                    "Project output retirement timed out waiting for an in-flight output operation"
                        .to_string();
                fail_locked(&mut state, retirement.transition.target, error.clone());
                gate.changed.notify_all();
                retirement.transition.finished = true;
                return Err(error);
            }
            #[cfg(test)]
            let _waiting = RetirementWaitObservation::new(&gate.retirement_waiting);
            state = gate
                .changed
                .wait_timeout(state, remaining)
                .map_err(|_| "Output ownership gate wait was poisoned".to_string())?
                .0;
        }
    }
    Ok(retirement.transition)
}

pub(super) fn fail_locked(
    state: &mut OutputOwnershipGateState,
    target: MachineOutputRole,
    error: String,
) {
    state.status = OutputOwnershipStatus::failed(
        target,
        state.status.persisted_role,
        state.status.generation,
        state.status.epoch,
        OutputOwnershipReason::TransitionFailed,
        error,
    );
    state.transition_active = state.in_flight != 0;
}

// Test-only observation of the actual condvar wait, with no production field,
// callbacks, polling thread or frame-path work.
#[cfg(test)]
struct RetirementWaitObservation<'a>(&'a AtomicBool);

#[cfg(test)]
impl<'a> RetirementWaitObservation<'a> {
    fn new(waiting: &'a AtomicBool) -> Self {
        waiting.store(true, Ordering::Release);
        Self(waiting)
    }
}

#[cfg(test)]
impl Drop for RetirementWaitObservation<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}
