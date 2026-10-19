//! One-time sender startup barrier; never used by the frame loop.
use super::*;

impl OutputOwnershipActivation {
    /// Creation publication releases its in-flight lease before the owner can
    /// complete Activating -> Ready. Keep the prepared sender idle until that
    /// exact activation is Ready. This grants no physical presentation permit.
    /// Cancellation returns false; stale/failed activations fail closed.
    pub fn wait_until_ready(&self, stop: &AtomicBool, timeout: Duration) -> Result<bool, String> {
        let started = Instant::now();
        let mut state = self
            .inner
            .state
            .lock()
            .map_err(|_| "Output startup readiness gate was poisoned".to_string())?;
        loop {
            if stop.load(Ordering::Acquire) {
                return Ok(false);
            }
            if state.status.epoch != self.epoch || state.status.desired_role != self.role {
                return Err("Output startup activation was invalidated".to_string());
            }
            if !state.transition_active
                && state.status.state == OutputOwnershipState::Ready
                && state.status.effective_role == self.role
                && self.role.video_allowed()
            {
                return Ok(true);
            }
            if !self.transition_owned
                || !state.transition_active
                || state.status.state != OutputOwnershipState::Activating
            {
                return Err("Output startup activation did not become Ready".to_string());
            }
            let remaining = timeout
                .checked_sub(started.elapsed())
                .filter(|remaining| !remaining.is_zero())
                .ok_or_else(|| "Output startup readiness timed out".to_string())?;
            // Ownership transitions notify the condition variable. The bounded
            // timeout only observes cancellation of this existing sender worker.
            state = self
                .inner
                .changed
                .wait_timeout(state, remaining.min(Duration::from_millis(25)))
                .map_err(|_| "Output startup readiness wait was poisoned".to_string())?
                .0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_startup_waits_without_render_or_fault_until_exact_ready() {
        let gate = OutputOwnershipGate::for_role(MachineOutputRole::Standby);
        let mut transition = gate.begin_transition(MachineOutputRole::Video).unwrap();
        transition.begin_activation().unwrap();
        let activation = transition.admit_output_activation().unwrap();
        activation
            .admit_resource_creation()
            .unwrap()
            .publish()
            .unwrap();
        let (started_tx, started_rx) = mpsc::sync_channel(0);
        let (render_tx, render_rx) = mpsc::sync_channel(0);
        let worker = thread::spawn(move || {
            started_tx.send(()).unwrap();
            if activation
                .wait_until_ready(&AtomicBool::new(false), Duration::from_secs(2))
                .unwrap()
            {
                render_tx.send(()).unwrap();
            }
        });
        started_rx.recv().unwrap();
        assert!(matches!(
            render_rx.recv_timeout(Duration::from_millis(25)),
            Err(mpsc::RecvTimeoutError::Timeout)
        ));
        assert_eq!(gate.status().state, OutputOwnershipState::Activating);
        assert_eq!(gate.inner.state.lock().unwrap().in_flight, 0);
        assert!(gate.acquire(OutputCapability::Video).is_err());
        transition.complete().unwrap();
        render_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        worker.join().unwrap();
    }

    #[test]
    fn output_startup_cancel_and_timeout_never_grant_presentation() {
        let gate = OutputOwnershipGate::for_role(MachineOutputRole::Standby);
        let mut transition = gate.begin_transition(MachineOutputRole::Video).unwrap();
        transition.begin_activation().unwrap();
        let activation = transition.admit_output_activation().unwrap();
        assert!(!activation
            .wait_until_ready(&AtomicBool::new(true), Duration::from_secs(1))
            .unwrap());
        assert!(activation
            .wait_until_ready(&AtomicBool::new(false), Duration::ZERO)
            .is_err());
        assert_eq!(gate.status().state, OutputOwnershipState::Activating);
        assert!(gate.acquire(OutputCapability::Video).is_err());
        transition.complete().unwrap();
    }

    #[test]
    fn output_startup_rejects_failed_and_replaced_activation() {
        let gate = OutputOwnershipGate::for_role(MachineOutputRole::Video);
        let activation = gate
            .admit_ready_activation(MachineOutputRole::Video)
            .unwrap();
        assert!(activation
            .wait_until_ready(&AtomicBool::new(false), Duration::ZERO)
            .unwrap());
        let failure = gate.begin_failure_fence("injected startup invalidation");
        assert!(activation
            .wait_until_ready(&AtomicBool::new(false), Duration::ZERO)
            .is_err());
        drop(failure);
        gate.begin_transition(MachineOutputRole::Video)
            .unwrap()
            .complete()
            .unwrap();
        assert!(activation
            .wait_until_ready(&AtomicBool::new(false), Duration::ZERO)
            .is_err());
    }

    #[test]
    fn output_startup_cancel_or_invalidation_wakes_an_existing_waiter() {
        for cancel in [true, false] {
            let gate = OutputOwnershipGate::for_role(MachineOutputRole::Standby);
            let mut transition = gate.begin_transition(MachineOutputRole::Video).unwrap();
            transition.begin_activation().unwrap();
            let activation = transition.admit_output_activation().unwrap();
            let stop = Arc::new(AtomicBool::new(false));
            let worker_stop = Arc::clone(&stop);
            let (started_tx, started_rx) = mpsc::sync_channel(0);
            let (done_tx, done_rx) = mpsc::sync_channel(0);
            let worker = thread::spawn(move || {
                started_tx.send(()).unwrap();
                done_tx
                    .send(activation.wait_until_ready(&worker_stop, Duration::from_secs(2)))
                    .unwrap();
            });
            started_rx.recv().unwrap();
            assert!(matches!(
                done_rx.recv_timeout(Duration::from_millis(25)),
                Err(mpsc::RecvTimeoutError::Timeout)
            ));
            let failure = if cancel {
                stop.store(true, Ordering::Release);
                None
            } else {
                Some(gate.begin_failure_fence("injected while startup waits"))
            };
            let result = done_rx.recv_timeout(Duration::from_secs(1)).unwrap();
            if cancel {
                assert_eq!(result.unwrap(), false);
            } else {
                assert!(result.is_err());
            }
            worker.join().unwrap();
            assert!(gate.acquire(OutputCapability::Video).is_err());
            drop(failure);
            drop(transition);
        }
    }
}
