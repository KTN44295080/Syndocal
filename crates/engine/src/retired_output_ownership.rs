//! Short authority commit boundary after physical resources have retired.
use super::*;

impl EngineHandle {
    /// The callback must only commit authority; it must not call the engine
    /// or perform physical I/O. The gate stays locked so an activation/failure
    /// cannot race a fresh lease identity into an unretired output state.
    pub fn with_retired_output_ownership<R>(
        &self,
        expected_epoch: u64,
        expected_generation: u64,
        commit: impl FnOnce() -> R,
    ) -> Result<R, String> {
        self.output_ownership_gate.with_retired_commit(expected_epoch, expected_generation, commit)
    }
}

impl OutputOwnershipGate {
    fn with_retired_commit<R>(&self, expected_epoch: u64, expected_generation: u64, commit: impl FnOnce() -> R) -> Result<R, String> {
        let state = self.inner.state.lock()
            .map_err(|_| "Output ownership gate lock was poisoned".to_string())?;
        if state.transition_active || state.in_flight != 0
            || state.status.state != OutputOwnershipState::Ready
            || state.status.effective_role != MachineOutputRole::Standby
            || state.status.lighting_allowed || state.status.video_allowed
            || state.status.error.is_some()
            || state.status.epoch != expected_epoch
            || state.status.generation != expected_generation {
            return Err("Previous-project output authority can be replaced only after output retirement completed in Standby".to_string());
        }
        Ok(commit())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retired_output_commit_requires_exact_quiescent_standby() {
        let gate = OutputOwnershipGate::for_role(MachineOutputRole::Standby);
        let status = gate.status();
        let calls = std::cell::Cell::new(0);
        let commit = || calls.set(calls.get() + 1);
        assert!(gate.with_retired_commit(status.epoch + 1, status.generation, commit).is_err());
        assert!(gate.with_retired_commit(status.epoch, status.generation + 1, commit).is_err());
        assert_eq!(calls.get(), 0);
        gate.with_retired_commit(status.epoch, status.generation, commit).unwrap();
        assert_eq!(gate.status(), status, "authority preparation must not arm or rotate output fences");
        assert_eq!(calls.get(), 1);
        for role in [MachineOutputRole::Video, MachineOutputRole::Lighting, MachineOutputRole::Both] {
            let active = OutputOwnershipGate::for_role(role);
            let status = active.status();
            assert!(active.with_retired_commit(status.epoch, status.generation, commit).is_err());
        }
        let retained = gate.begin_failure_fence("SDK retirement pending");
        let failed = gate.status();
        assert!(gate.with_retired_commit(failed.epoch, failed.generation, commit).is_err());
        assert_eq!(calls.get(), 1);
        drop(retained);
    }

    #[test]
    fn retired_output_commit_serializes_against_activation() {
        let gate = OutputOwnershipGate::for_role(MachineOutputRole::Standby);
        let status = gate.status();
        let (entered, entered_rx) = mpsc::sync_channel(1);
        let (release, release_rx) = mpsc::sync_channel(1);
        let owned = gate.clone();
        let commit = std::thread::spawn(move || owned.with_retired_commit(status.epoch, status.generation, || {
            entered.send(()).unwrap();
            release_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        }));
        entered_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        let (started, started_rx) = mpsc::sync_channel(1);
        let (activated, activated_rx) = mpsc::sync_channel(1);
        let activation = std::thread::spawn(move || {
            started.send(()).unwrap();
            let transition = gate.begin_transition(MachineOutputRole::Video).unwrap();
            activated.send(()).unwrap();
            transition.complete().unwrap();
        });
        started_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        assert!(matches!(activated_rx.try_recv(), Err(mpsc::TryRecvError::Empty)));
        release.send(()).unwrap();
        commit.join().unwrap().unwrap();
        activated_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        activation.join().unwrap();
    }
}
