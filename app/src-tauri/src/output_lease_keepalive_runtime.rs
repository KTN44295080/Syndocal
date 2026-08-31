//! Process-local orchestration for the managed exact-`Both` lease keepalive.
//!
//! This layer intentionally owns only ordering, cancellation and worker
//! lifetime.  `output_lease_keepalive` remains the authority state machine;
//! this module never manufactures evidence, a renewal receipt or a registry
//! relinquish receipt.  Production wiring supplies the registry CAS and the
//! seven fail-stop ports from the application boundary.
//!
//! The central rule is that a registry CAS and its corresponding manager
//! completion are serialized by `operation_serial`, while physical I/O is
//! deliberately outside every runtime/manager/registry lock.  A boundary
//! first changes the manager to Faulting, invalidating any old renewal
//! capability; it can then execute a bounded best-effort fail-stop without
//! blocking a caller which only needs to observe the runtime.

use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, Condvar, Mutex, MutexGuard,
    },
    thread::JoinHandle,
    time::{Duration, Instant},
};

use crate::output_lease::output_lease_keepalive::{
    execute_fail_stop_plan, OutputLeaseKeepaliveArmOutcome, OutputLeaseKeepaliveBoundary,
    OutputLeaseKeepaliveCleanupOnlyOutcome, OutputLeaseKeepaliveDurableEnableCleanupOnlyEvidence,
    OutputLeaseKeepaliveDurableEnableEvidence, OutputLeaseKeepaliveError,
    OutputLeaseKeepaliveFailStopCompletion, OutputLeaseKeepaliveFailStopExecutionOutcome,
    OutputLeaseKeepaliveFailStopPlan, OutputLeaseKeepaliveFailStopPorts,
    OutputLeaseKeepaliveIdentity, OutputLeaseKeepaliveManagedAdmissionConflict,
    OutputLeaseKeepaliveManager, OutputLeaseKeepaliveManualOperationDecision,
    OutputLeaseKeepalivePortError, OutputLeaseKeepaliveRelinquishCasReceipt,
    OutputLeaseKeepaliveRelinquishCompletion, OutputLeaseKeepaliveRelinquishPermit,
    OutputLeaseKeepaliveRenewalCapability, OutputLeaseKeepaliveRenewalCasReceipt,
    OutputLeaseKeepaliveRenewalCompletion, OutputLeaseKeepaliveRunId, OutputLeaseKeepaliveState,
    OutputLeaseKeepaliveSupersessionCapability, OutputLeaseKeepaliveSupersessionCasReceipt,
    OutputLeaseKeepaliveSupersessionCompletion, OutputLeaseKeepaliveTimerAction,
};
use tauri::Manager as _;

#[cfg(test)]
use crate::output_lease::output_lease_keepalive::OutputLeaseKeepaliveManualOperation;

/// A run-bound cancellation handle.  It is safe to retain after the worker has
/// exited: cancellation is idempotent and a wake only releases a waiter which
/// still belongs to this exact run.
#[derive(Debug)]
pub(crate) struct OutputLeaseKeepaliveWorkerControl {
    run_id: OutputLeaseKeepaliveRunId,
    cancelled: AtomicBool,
    wake: (Mutex<bool>, Condvar),
    start: (Mutex<bool>, Condvar),
}

impl OutputLeaseKeepaliveWorkerControl {
    fn new(run_id: OutputLeaseKeepaliveRunId) -> Self {
        Self {
            run_id,
            cancelled: AtomicBool::new(false),
            wake: (Mutex::new(false), Condvar::new()),
            start: (Mutex::new(false), Condvar::new()),
        }
    }

    fn await_start_or_cancelled(&self) -> bool {
        let (started, notifier) = &self.start;
        let mut started = match started.lock() {
            Ok(started) => started,
            Err(poisoned) => poisoned.into_inner(),
        };
        while !*started && !self.is_cancelled() {
            started = match notifier.wait(started) {
                Ok(started) => started,
                Err(poisoned) => poisoned.into_inner(),
            };
        }
        *started && !self.is_cancelled()
    }

    fn start_after_slot_publication(&self) {
        let (started, notifier) = &self.start;
        let mut started = match started.lock() {
            Ok(started) => started,
            Err(poisoned) => poisoned.into_inner(),
        };
        *started = true;
        notifier.notify_all();
    }

    pub(crate) fn run_id(&self) -> OutputLeaseKeepaliveRunId {
        self.run_id
    }

    pub(crate) fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }

    /// Interrupt a wait immediately.  No manager or registry lock is taken.
    pub(crate) fn cancel_and_wake(&self) {
        self.cancelled.store(true, Ordering::Release);
        let (wake, notifier) = &self.wake;
        match wake.lock() {
            Ok(mut wake) => {
                *wake = true;
                notifier.notify_all();
            }
            Err(poisoned) => {
                let mut wake = poisoned.into_inner();
                *wake = true;
                notifier.notify_all();
            }
        }
        self.start.1.notify_all();
    }

    /// Bounded cancellable wait used by the worker's 20-second renewal timer.
    /// It never sleeps while holding an authority lock.
    pub(crate) fn wait_until_or_cancelled(&self, deadline: Instant) -> bool {
        let (wake, notifier) = &self.wake;
        let mut wake = match wake.lock() {
            Ok(wake) => wake,
            Err(poisoned) => poisoned.into_inner(),
        };
        loop {
            if self.is_cancelled() {
                return false;
            }
            if *wake {
                *wake = false;
                return !self.is_cancelled();
            }
            let now = Instant::now();
            if now >= deadline {
                return true;
            }
            let timeout = deadline.saturating_duration_since(now);
            let (next, result) = match notifier.wait_timeout(wake, timeout) {
                Ok(value) => value,
                Err(poisoned) => poisoned.into_inner(),
            };
            wake = next;
            if result.timed_out() {
                return !self.is_cancelled();
            }
        }
    }
}

#[derive(Debug)]
struct OutputLeaseKeepaliveWorkerSlot {
    run_id: OutputLeaseKeepaliveRunId,
    control: Arc<OutputLeaseKeepaliveWorkerControl>,
    join: Option<JoinHandle<()>>,
    completion: mpsc::Receiver<()>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputLeaseKeepaliveWorkerRetirementKind {
    /// The worker itself initiated canonical fail-stop.  The manager has
    /// already fenced the old run, so an exactly Armed successor may install
    /// from any caller thread while the old worker finishes returning.
    WorkerSelfHandoff,
    /// An external cancel did not receive the worker's bounded completion
    /// acknowledgement.  Its JoinHandle remains owned here and blocks a
    /// successor until a non-worker reaper has joined it.
    AwaitingExternalReap,
}

#[derive(Debug)]
struct OutputLeaseKeepaliveRetiringWorkerSlot {
    worker: OutputLeaseKeepaliveWorkerSlot,
    kind: OutputLeaseKeepaliveWorkerRetirementKind,
}

impl OutputLeaseKeepaliveRetiringWorkerSlot {
    fn blocks_successor(&self) -> bool {
        self.kind == OutputLeaseKeepaliveWorkerRetirementKind::AwaitingExternalReap
    }
}

#[derive(Debug, Default)]
struct WorkerSlot {
    active: Option<OutputLeaseKeepaliveWorkerSlot>,
    reserving: Option<OutputLeaseKeepaliveRunId>,
    /// `active` always belongs to the one exactly Armed run.  Retiring slots
    /// retain completion and JoinHandle ownership until a non-worker thread
    /// joins them; only an acknowledged worker-self handoff may coexist with
    /// a successor in `active`.
    retiring: Vec<OutputLeaseKeepaliveRetiringWorkerSlot>,
}

/// Deterministic post-OS-spawn/pre-publication fault injection.  It proves
/// that a lost reservation keeps the cancelled, unpublished JoinHandle in
/// `retiring` until its normal non-worker reap path joins it.
#[cfg(test)]
#[derive(Debug, Default)]
struct OutputLeaseKeepaliveWorkerSpawnTestHooks {
    force_next_reservation_loss: AtomicBool,
    poison_before_external_retain: AtomicBool,
    block_cancelled_prestart: AtomicBool,
    cancelled_prestart_entered: (Mutex<bool>, Condvar),
}

#[cfg(test)]
impl OutputLeaseKeepaliveWorkerSpawnTestHooks {
    fn arm_reservation_loss(&self) {
        self.force_next_reservation_loss
            .store(true, Ordering::Release);
        self.block_cancelled_prestart.store(true, Ordering::Release);
    }

    fn consume_reservation_loss(&self) -> bool {
        self.force_next_reservation_loss
            .swap(false, Ordering::AcqRel)
    }

    fn arm_external_retain_poison(&self) {
        self.poison_before_external_retain
            .store(true, Ordering::Release);
    }

    fn consume_external_retain_poison(&self) -> bool {
        self.poison_before_external_retain
            .swap(false, Ordering::AcqRel)
    }

    fn block_cancelled_prestart_until_released(&self) {
        if !self.block_cancelled_prestart.load(Ordering::Acquire) {
            return;
        }
        let (entered, notifier) = &self.cancelled_prestart_entered;
        let mut entered = match entered.lock() {
            Ok(entered) => entered,
            Err(poisoned) => poisoned.into_inner(),
        };
        *entered = true;
        notifier.notify_all();
        while self.block_cancelled_prestart.load(Ordering::Acquire) {
            entered = match notifier.wait(entered) {
                Ok(entered) => entered,
                Err(poisoned) => poisoned.into_inner(),
            };
        }
    }

    fn wait_for_cancelled_prestart(&self, timeout: Duration) -> bool {
        let (entered, notifier) = &self.cancelled_prestart_entered;
        let entered = match entered.lock() {
            Ok(entered) => entered,
            Err(poisoned) => poisoned.into_inner(),
        };
        if *entered {
            return true;
        }
        match notifier.wait_timeout(entered, timeout) {
            Ok((entered, _)) => *entered,
            Err(poisoned) => *poisoned.into_inner().0,
        }
    }

    fn release_cancelled_prestart(&self) {
        self.block_cancelled_prestart
            .store(false, Ordering::Release);
        self.cancelled_prestart_entered.1.notify_all();
    }
}

/// Worker retirement can only be joined after this bounded acknowledgement.
/// That prevents an external lifecycle boundary from blocking forever and
/// makes joining from the worker itself structurally impossible.
#[derive(Debug)]
pub(crate) struct OutputLeaseKeepaliveWorkerTermination {
    run_id: OutputLeaseKeepaliveRunId,
    control: Arc<OutputLeaseKeepaliveWorkerControl>,
    completion: mpsc::Receiver<()>,
    join: JoinHandle<()>,
}

#[derive(Debug)]
pub(crate) enum OutputLeaseKeepaliveWorkerWaitFailure {
    Unacknowledged(OutputLeaseKeepaliveWorkerTermination, String),
    Panicked(String),
}

impl OutputLeaseKeepaliveWorkerTermination {
    /// A timeout returns this exact termination to its caller.  In
    /// particular, it never drops/detaches the JoinHandle merely because the
    /// bounded acknowledgement wait expired.
    pub(crate) fn wait_and_join(
        self,
        timeout: Duration,
    ) -> Result<(), OutputLeaseKeepaliveWorkerWaitFailure> {
        if let Err(error) = self.completion.recv_timeout(timeout) {
            let detail = format!(
                "Managed output-lease keepalive worker {} did not acknowledge cancellation: {error}",
                self.run_id.get()
            );
            return Err(OutputLeaseKeepaliveWorkerWaitFailure::Unacknowledged(
                self, detail,
            ));
        }
        self.join.join().map_err(|_| {
            OutputLeaseKeepaliveWorkerWaitFailure::Panicked(format!(
                "Managed output-lease keepalive worker {} panicked",
                self.run_id.get()
            ))
        })
    }

    fn into_retiring_worker(self) -> OutputLeaseKeepaliveWorkerSlot {
        OutputLeaseKeepaliveWorkerSlot {
            run_id: self.run_id,
            control: self.control,
            join: Some(self.join),
            completion: self.completion,
        }
    }
}

/// Result of a durable-Enable admission.  The caller must not treat `Armed`
/// as live until it has successfully installed exactly one worker.  Every
/// other variant carries the current run's already-authoritative action.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum OutputLeaseKeepaliveRuntimeAdmission {
    SpawnWorker(OutputLeaseKeepaliveRunId),
    AlreadyArmed(OutputLeaseKeepaliveRunId),
    FailStop(OutputLeaseKeepaliveFailStopPlan),
    RequireCurrentRelinquish(OutputLeaseKeepaliveRelinquishPermit),
    ManagedConflict(OutputLeaseKeepaliveManagedAdmissionConflict),
    AwaitCurrentRelease,
    StaleNoop,
}

/// Cleanup-only admission for an Enable which has already committed durably
/// but never reached normal keepalive admission.  The wrapped core outcome is
/// deliberately unable to arm a worker.  Poison recovery is explicit so the
/// integration boundary can record that an all-deny plan was created from a
/// recovered authority mutex rather than a clean serial lane.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct OutputLeaseKeepaliveRuntimeCleanupOnly {
    outcome: OutputLeaseKeepaliveCleanupOnlyOutcome,
    recovered_operation_serial_poison: bool,
    recovered_manager_poison: bool,
}

impl OutputLeaseKeepaliveRuntimeCleanupOnly {
    pub(crate) fn recovered_operation_serial_poison(&self) -> bool {
        self.recovered_operation_serial_poison
    }

    pub(crate) fn recovered_manager_poison(&self) -> bool {
        self.recovered_manager_poison
    }

    pub(crate) fn into_outcome(self) -> OutputLeaseKeepaliveCleanupOnlyOutcome {
        self.outcome
    }
}

/// Narrow result of a serialized renewal.  A caller executes the returned
/// fail-stop plan outside this runtime's locks.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum OutputLeaseKeepaliveRuntimeRenewal {
    Renewed,
    FailStop(OutputLeaseKeepaliveFailStopPlan),
    StaleNoop,
}

/// Lifecycle interception result without leaking/reconstructing a run ID at
/// the app boundary. A Faulted result intentionally retains authority until
/// the explicit release/supersession path resolves it.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum OutputLeaseKeepaliveRuntimeCurrentBoundary {
    Captured(OutputLeaseKeepaliveFailStopPlan),
    RetainedFaulted,
    Idle,
}

/// Managed manual lifecycle response.  `Released` contains only evidence
/// captured by the registry closure before the manager consumes the exact
/// successful receipt; rejected/stale paths never expose it.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum OutputLeaseKeepaliveRuntimeManualRelinquish<E> {
    AllowUnmanaged,
    RejectManagedLease,
    FailStop(OutputLeaseKeepaliveFailStopPlan),
    Released(E),
    RetainedForRetry,
    StaleNoop,
}

/// Deferred durable Enable B may supersede an old Faulted A only after the
/// registry proves B is still current under the same serialized operation.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum OutputLeaseKeepaliveRuntimeSupersession {
    Armed(OutputLeaseKeepaliveRunId),
    FailStop(OutputLeaseKeepaliveFailStopPlan),
    RetainedFaulted,
    StaleNoop,
}

/// Internal result which keeps caller-provided evidence unavailable when an
/// opaque permit is stale before it reaches the registry boundary.
enum RelinquishPermitCasOutcome<E> {
    StaleBeforeCas,
    Aborted(OutputLeaseKeepaliveRelinquishCompletion),
    Completed(OutputLeaseKeepaliveRelinquishCompletion, E),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OutputLeaseKeepaliveFailStopExecutionContext {
    WorkerSelf(OutputLeaseKeepaliveRunId),
    ExternalBoundary,
}

/// One worker-loop decision after an external monotonic clock sample.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum OutputLeaseKeepaliveRuntimeTimer {
    Sleep(crate::output_lease::output_lease_keepalive::OutputLeaseKeepaliveWait),
    RenewDue(OutputLeaseKeepaliveRunId),
    FailStop(OutputLeaseKeepaliveFailStopPlan),
    StaleNoop,
}

/// Owns no application state.  The concrete `AppState` adapter is expected to
/// keep this value in an `Arc`, prove durable Enable evidence in the registry,
/// and route physical fail-stop calls through `OutputLeaseKeepaliveFailStopPorts`.
#[derive(Debug, Default)]
pub(crate) struct OutputLeaseKeepaliveRuntime {
    /// Covers manager prepare -> registry CAS -> manager completion only.
    /// It is intentionally not held while executing physical ports or joining
    /// a worker.
    operation_serial: Mutex<()>,
    manager: Mutex<OutputLeaseKeepaliveManager>,
    worker: Arc<Mutex<WorkerSlot>>,
    #[cfg(test)]
    worker_spawn_test_hooks: Arc<OutputLeaseKeepaliveWorkerSpawnTestHooks>,
}

/// Opaque bridge admission for one ordinary physical-output authorization.
/// Route callers retain it through the complete private candidate and bounded
/// physical callback, then release it with their registry guard. Renewal
/// takes this lane before its registry CAS, so it cannot advance the managed
/// lease generation anywhere between authorization and publication.
#[must_use]
pub(crate) struct OutputLeaseKeepaliveManagedExactBothAuthorizationGuard<'a> {
    _operation_serial: MutexGuard<'a, ()>,
    lease_id: String,
}

impl OutputLeaseKeepaliveManagedExactBothAuthorizationGuard<'_> {
    /// A guard is bound to the one Armed lease which issued it.  Holding a
    /// guard for another run/lease is never an authorization substitute.
    pub(crate) fn authorizes_lease_id(&self, lease_id: &str) -> bool {
        self.lease_id == lease_id
    }
}

impl OutputLeaseKeepaliveRuntime {
    pub(crate) fn snapshot_state(&self) -> Result<OutputLeaseKeepaliveState, String> {
        self.manager
            .lock()
            .map(|manager| manager.snapshot().state)
            .map_err(|_| "Managed output-lease keepalive manager lock was poisoned".to_string())
    }

    /// Pure preflight for public lifecycle routing. It neither issues a
    /// permit nor captures a fail-stop plan, so callers can route a managed
    /// Relinquish into the serialized bridge without first mutating the
    /// manager through the generic registry command path.
    pub(crate) fn manages_lease_id(&self, lease_id: &str) -> Result<bool, String> {
        let _serial = self.operation_serial.lock().map_err(|_| {
            "Managed output-lease keepalive operation serialization lock was poisoned".to_string()
        })?;
        let snapshot = self
            .manager
            .lock()
            .map_err(|_| "Managed output-lease keepalive manager lock was poisoned".to_string())?
            .snapshot();
        Ok(snapshot
            .armed
            .as_ref()
            .is_some_and(|armed| armed.lease().lease_id() == lease_id)
            || snapshot
                .fault
                .as_ref()
                .is_some_and(|fault| fault.lease().lease_id() == lease_id))
    }

    /// Admit the private managed-exact-Both authorization action.  Faulted,
    /// idle, and changed runs fail closed; this is not a general stale-lease
    /// retry or a way to rebase an ordinary caller-supplied generation.
    pub(crate) fn begin_managed_exact_both_ordinary_authorization(
        &self,
        lease_id: &str,
    ) -> Result<OutputLeaseKeepaliveManagedExactBothAuthorizationGuard<'_>, String> {
        let operation_serial = self.operation_serial.lock().map_err(|_| {
            "Managed output-lease keepalive operation serialization lock was poisoned".to_string()
        })?;
        let snapshot = self
            .manager
            .lock()
            .map_err(|_| "Managed output-lease keepalive manager lock was poisoned".to_string())?
            .snapshot();
        if !snapshot
            .armed
            .as_ref()
            .is_some_and(|armed| armed.lease().lease_id() == lease_id)
        {
            return Err(
                "Managed exact-Both output lease is no longer armed for this authorization"
                    .to_string(),
            );
        }
        Ok(OutputLeaseKeepaliveManagedExactBothAuthorizationGuard {
            _operation_serial: operation_serial,
            lease_id: lease_id.to_string(),
        })
    }

    fn identity_for_exact_run(
        &self,
        run_id: OutputLeaseKeepaliveRunId,
    ) -> Result<Option<OutputLeaseKeepaliveIdentity>, String> {
        let snapshot = self
            .manager
            .lock()
            .map_err(|_| "Managed output-lease keepalive manager lock was poisoned".to_string())?
            .snapshot();
        if let Some(armed) = snapshot.armed {
            return Ok((armed.run_id() == run_id).then(|| armed.identity().clone()));
        }
        if let Some(fault) = snapshot.fault {
            return Ok((fault.run_id() == run_id).then(|| fault.identity().clone()));
        }
        Ok(None)
    }

    /// Prove the durable receipt and issue opaque evidence while holding the
    /// operation serial lane. A lifecycle boundary cannot race verification
    /// to arm this way.
    pub(crate) fn admit_verified_durable_enable_with<F>(
        &self,
        now_ms: u64,
        evidence_factory: F,
    ) -> Result<OutputLeaseKeepaliveRuntimeAdmission, String>
    where
        F: FnOnce() -> Result<OutputLeaseKeepaliveDurableEnableEvidence, String>,
    {
        let _serial = self.operation_serial.lock().map_err(|_| {
            "Managed output-lease keepalive operation serialization lock was poisoned".to_string()
        })?;
        let evidence = evidence_factory()?;
        let outcome = self
            .manager
            .lock()
            .map_err(|_| "Managed output-lease keepalive manager lock was poisoned".to_string())?
            .arm_or_return_bound_fail_stop_plan(now_ms, evidence);
        Ok(Self::map_arm_outcome(outcome))
    }

    /// Convert a verified durable Enable which failed *before* normal
    /// keepalive admission into the cleanup-only all-deny path. This is not
    /// an arm retry: its evidence has no arm conversion, this method has no
    /// clock input, and no outcome can publish an Armed run or spawn a worker.
    ///
    /// Unlike normal admission/renewal, mutex poison is recovered here so a
    /// durably committed exact-Both route cannot remain live and workerless.
    /// The recovery is retained in the returned result for the caller's
    /// durable failure evidence. Physical ports are still executed only by
    /// the returned plan, after both locks have been released.
    pub(crate) fn begin_cleanup_only_after_unadmitted_durable_enable(
        &self,
        evidence: OutputLeaseKeepaliveDurableEnableCleanupOnlyEvidence,
    ) -> OutputLeaseKeepaliveRuntimeCleanupOnly {
        let (serial, recovered_operation_serial_poison) = match self.operation_serial.lock() {
            Ok(serial) => (serial, false),
            Err(poisoned) => (poisoned.into_inner(), true),
        };
        let (mut manager, recovered_manager_poison) = match self.manager.lock() {
            Ok(manager) => (manager, false),
            Err(poisoned) => (poisoned.into_inner(), true),
        };
        let outcome = manager.begin_cleanup_only_after_unadmitted_durable_enable(evidence);
        drop(manager);
        drop(serial);
        // A recovered poison flag would otherwise make the ordinary
        // fail-stop completion/relinquish bridge reject this newly captured
        // canonical plan and strand Faulting forever.  Cleanup is the one
        // deliberate recovery boundary: after core rebuilt one exact
        // all-deny state, clear only the mutexes it recovered so later phases
        // can complete that same plan. Normal arm/renew paths retain their
        // fail-closed poison behavior.
        if recovered_manager_poison {
            self.manager.clear_poison();
        }
        if recovered_operation_serial_poison {
            self.operation_serial.clear_poison();
        }
        OutputLeaseKeepaliveRuntimeCleanupOnly {
            outcome,
            recovered_operation_serial_poison,
            recovered_manager_poison,
        }
    }

    #[cfg(test)]
    fn admit_durable_enable_for_tests(
        &self,
        now_ms: u64,
        evidence: OutputLeaseKeepaliveDurableEnableEvidence,
    ) -> Result<OutputLeaseKeepaliveRuntimeAdmission, String> {
        self.admit_verified_durable_enable_with(now_ms, || Ok(evidence))
    }

    #[cfg(test)]
    fn force_next_worker_spawn_reservation_loss(&self) {
        self.worker_spawn_test_hooks.arm_reservation_loss();
    }

    #[cfg(test)]
    fn wait_for_forced_reservation_loss_prestart(&self, timeout: Duration) -> bool {
        self.worker_spawn_test_hooks
            .wait_for_cancelled_prestart(timeout)
    }

    #[cfg(test)]
    fn release_forced_reservation_loss_prestart(&self) {
        self.worker_spawn_test_hooks.release_cancelled_prestart();
    }

    #[cfg(test)]
    fn force_next_external_retain_worker_poison(&self) {
        self.worker_spawn_test_hooks.arm_external_retain_poison();
    }

    fn map_arm_outcome(
        outcome: OutputLeaseKeepaliveArmOutcome,
    ) -> OutputLeaseKeepaliveRuntimeAdmission {
        match outcome {
            OutputLeaseKeepaliveArmOutcome::Armed(run_id) => {
                OutputLeaseKeepaliveRuntimeAdmission::SpawnWorker(run_id)
            }
            OutputLeaseKeepaliveArmOutcome::AlreadyArmed(run_id) => {
                OutputLeaseKeepaliveRuntimeAdmission::AlreadyArmed(run_id)
            }
            OutputLeaseKeepaliveArmOutcome::ResumeFailStop(plan)
            | OutputLeaseKeepaliveArmOutcome::FailStop(plan) => {
                OutputLeaseKeepaliveRuntimeAdmission::FailStop(plan)
            }
            OutputLeaseKeepaliveArmOutcome::RequireCurrentRelinquish(permit) => {
                OutputLeaseKeepaliveRuntimeAdmission::RequireCurrentRelinquish(permit)
            }
            OutputLeaseKeepaliveArmOutcome::ManagedConflict(conflict) => {
                OutputLeaseKeepaliveRuntimeAdmission::ManagedConflict(conflict)
            }
            OutputLeaseKeepaliveArmOutcome::AwaitCurrentRelease => {
                OutputLeaseKeepaliveRuntimeAdmission::AwaitCurrentRelease
            }
            OutputLeaseKeepaliveArmOutcome::StaleNoop => {
                OutputLeaseKeepaliveRuntimeAdmission::StaleNoop
            }
        }
    }

    /// Install a worker after `SpawnWorker`.  The worker body is supplied by
    /// the concrete adapter so it can obtain time and perform registry CAS
    /// without this generic layer importing Tauri or `AppState`.
    pub(crate) fn spawn_worker<F>(
        &self,
        run_id: OutputLeaseKeepaliveRunId,
        worker_body: F,
    ) -> Result<Arc<OutputLeaseKeepaliveWorkerControl>, String>
    where
        F: FnOnce(Arc<OutputLeaseKeepaliveWorkerControl>) + Send + 'static,
    {
        // Only an already-acknowledged worker is reaped here, so this never
        // delays a new Enable behind a running worker or holds a lock in join.
        self.reap_any_completed_worker()?;
        let _serial = self.operation_serial.lock().map_err(|_| {
            "Managed output-lease keepalive operation serialization lock was poisoned".to_string()
        })?;
        let exact_armed_run = self
            .manager
            .lock()
            .map_err(|_| "Managed output-lease keepalive manager lock was poisoned".to_string())?
            .snapshot()
            .armed
            .is_some_and(|armed| armed.run_id() == run_id);
        if !exact_armed_run {
            return Err(
                "Managed output-lease keepalive worker run is no longer exactly Armed".to_string(),
            );
        }
        {
            let mut slot = self.worker.lock().map_err(|_| {
                "Managed output-lease keepalive worker slot was poisoned".to_string()
            })?;
            if slot.active.is_some() {
                return Err("Managed output-lease keepalive worker already exists".to_string());
            }
            if slot
                .retiring
                .iter()
                .any(OutputLeaseKeepaliveRetiringWorkerSlot::blocks_successor)
            {
                return Err(
                    "Managed output-lease keepalive worker retirement is not yet acknowledged"
                        .to_string(),
                );
            }
            if slot.reserving.is_some() {
                return Err("Managed output-lease keepalive worker already exists".to_string());
            }
            slot.reserving = Some(run_id);
        }
        let control = Arc::new(OutputLeaseKeepaliveWorkerControl::new(run_id));
        let worker_control = Arc::clone(&control);
        let worker_slots = Arc::clone(&self.worker);
        #[cfg(test)]
        let worker_spawn_test_hooks = Arc::clone(&self.worker_spawn_test_hooks);
        let (completion_tx, completion_rx) = mpsc::sync_channel(1);
        let join = std::thread::Builder::new()
            .name(format!("syndocal-output-lease-keepalive-{}", run_id.get()))
            .spawn(move || {
                if !worker_control.await_start_or_cancelled() {
                    #[cfg(test)]
                    worker_spawn_test_hooks.block_cancelled_prestart_until_released();
                    let _ = completion_tx.send(());
                    Self::schedule_retiring_reap_after_completion(&worker_slots, run_id);
                    return;
                }
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    worker_body(worker_control)
                }));
                let _ = completion_tx.send(());
                Self::schedule_retiring_reap_after_completion(&worker_slots, run_id);
                if let Err(payload) = result {
                    std::panic::resume_unwind(payload);
                }
            })
            .map_err(|error| {
                let mut slot = self
                    .worker
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                if slot.reserving == Some(run_id) {
                    slot.reserving = None;
                }
                format!("Managed output-lease keepalive worker spawn failed: {error}")
            })?;

        #[cfg(test)]
        if self.worker_spawn_test_hooks.consume_reservation_loss() {
            if let Ok(mut slot) = self.worker.lock() {
                if slot.reserving == Some(run_id) {
                    slot.reserving = None;
                }
            }
        }

        let (mut slot, recovered_worker_poison) = match self.worker.lock() {
            Ok(slot) => (slot, false),
            Err(poisoned) => (poisoned.into_inner(), true),
        };
        if recovered_worker_poison || slot.reserving != Some(run_id) || slot.active.is_some() {
            control.cancel_and_wake();
            if slot.reserving == Some(run_id) {
                slot.reserving = None;
            }
            slot.retiring.push(OutputLeaseKeepaliveRetiringWorkerSlot {
                worker: OutputLeaseKeepaliveWorkerSlot {
                    run_id,
                    control,
                    join: Some(join),
                    completion: completion_rx,
                },
                kind: OutputLeaseKeepaliveWorkerRetirementKind::AwaitingExternalReap,
            });
            drop(slot);
            if recovered_worker_poison {
                // The raw worker was cancelled and is now exclusively owned
                // by `retiring`; clear only this structural mutex poison so a
                // later non-worker reaper can join it.  Authority mutexes are
                // not recovered by this worker-publication boundary.
                self.worker.clear_poison();
                return Err(
                    "Managed output-lease keepalive worker slot was poisoned after OS spawn; the cancelled handle is retained for reaping"
                        .to_string(),
                );
            }
            return Err("Managed output-lease keepalive worker reservation was lost".to_string());
        }
        slot.reserving = None;
        slot.active = Some(OutputLeaseKeepaliveWorkerSlot {
            run_id,
            control: Arc::clone(&control),
            join: Some(join),
            completion: completion_rx,
        });
        control.start_after_slot_publication();
        Ok(control)
    }

    /// Convert a failed worker creation into the manager's canonical
    /// fail-stop obligation.  This does not execute I/O.
    pub(crate) fn record_worker_spawn_failure(
        &self,
        run_id: OutputLeaseKeepaliveRunId,
        error: OutputLeaseKeepalivePortError,
    ) -> Result<Option<OutputLeaseKeepaliveFailStopPlan>, String> {
        let _serial = self.operation_serial.lock().map_err(|_| {
            "Managed output-lease keepalive operation serialization lock was poisoned".to_string()
        })?;
        let plan = self
            .manager
            .lock()
            .map_err(|_| "Managed output-lease keepalive manager lock was poisoned".to_string())?
            .record_worker_spawn_failure(run_id, error);
        Ok(plan)
    }

    /// A worker may only disappear normally after cancellation or after the
    /// manager has already left Armed. Any early return/panic while the exact
    /// run is still Armed becomes its own canonical WorkerTerminated plan.
    pub(crate) fn record_worker_termination(
        &self,
        run_id: OutputLeaseKeepaliveRunId,
        error: OutputLeaseKeepalivePortError,
    ) -> Result<Option<OutputLeaseKeepaliveFailStopPlan>, String> {
        let _serial = self.operation_serial.lock().map_err(|_| {
            "Managed output-lease keepalive operation serialization lock was poisoned".to_string()
        })?;
        Ok(self
            .manager
            .lock()
            .map_err(|_| "Managed output-lease keepalive manager lock was poisoned".to_string())?
            .record_worker_termination(run_id, error))
    }

    /// Cancel only the matching active worker. The returned termination must
    /// be awaited and joined outside all runtime, registry and application
    /// locks.
    pub(crate) fn cancel_worker_for_run(
        &self,
        run_id: OutputLeaseKeepaliveRunId,
    ) -> Result<Option<OutputLeaseKeepaliveWorkerTermination>, String> {
        let mut slot = self
            .worker
            .lock()
            .map_err(|_| "Managed output-lease keepalive worker slot was poisoned".to_string())?;
        let matches = slot
            .active
            .as_ref()
            .is_some_and(|active| active.run_id == run_id);
        if !matches {
            return Ok(None);
        }
        let mut active = slot
            .active
            .take()
            .expect("matching active worker was checked");
        if active
            .join
            .as_ref()
            .is_some_and(|join| join.thread().id() == std::thread::current().id())
        {
            // A worker receives only `OutputLeaseKeepaliveWorkerControl`, not
            // a join capability.  Keep this hard guard as a second line of
            // defense if a future closure captures the runtime by mistake.
            slot.active = Some(active);
            return Err("Managed output-lease keepalive worker cannot join itself".to_string());
        }
        active.control.cancel_and_wake();
        Ok(active
            .join
            .take()
            .map(|join| OutputLeaseKeepaliveWorkerTermination {
                run_id,
                control: active.control,
                completion: active.completion,
                join,
            }))
    }

    /// Cancel a worker as the first canonical fail-stop port. The worker which
    /// detected its own failed renewal is never joined from itself: it is
    /// already committed to returning after the fail-stop sequence.
    pub(crate) fn cancel_worker_for_fail_stop(
        &self,
        run_id: OutputLeaseKeepaliveRunId,
        context: OutputLeaseKeepaliveFailStopExecutionContext,
    ) -> Result<(), String> {
        self.cancel_worker_for_fail_stop_with_timeout(run_id, context, Duration::from_secs(3))
    }

    fn cancel_worker_for_fail_stop_with_timeout(
        &self,
        run_id: OutputLeaseKeepaliveRunId,
        context: OutputLeaseKeepaliveFailStopExecutionContext,
        timeout: Duration,
    ) -> Result<(), String> {
        if context == OutputLeaseKeepaliveFailStopExecutionContext::WorkerSelf(run_id) {
            self.cancel_worker_from_its_own_thread(run_id)?;
            return Ok(());
        }
        let Some(termination) = self.cancel_worker_for_run(run_id)? else {
            return Ok(());
        };
        #[cfg(test)]
        if self
            .worker_spawn_test_hooks
            .consume_external_retain_poison()
        {
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let _guard = self.worker.lock().unwrap();
                panic!("poison worker slot before external timeout retention");
            }));
        }
        match termination.wait_and_join(timeout) {
            Ok(()) => Ok(()),
            Err(OutputLeaseKeepaliveWorkerWaitFailure::Unacknowledged(termination, detail)) => {
                self.retain_for_external_reap(termination.into_retiring_worker())?;
                Err(detail)
            }
            Err(OutputLeaseKeepaliveWorkerWaitFailure::Panicked(detail)) => Err(detail),
        }
    }

    #[cfg(test)]
    fn cancel_worker_for_fail_stop_with_test_timeout(
        &self,
        run_id: OutputLeaseKeepaliveRunId,
        context: OutputLeaseKeepaliveFailStopExecutionContext,
        timeout: Duration,
    ) -> Result<(), String> {
        self.cancel_worker_for_fail_stop_with_timeout(run_id, context, timeout)
    }

    /// Mark the exact worker currently executing a fail-stop as cancelled and
    /// immediately transfer its handle to `retiring`.  This clears `active`
    /// before the worker continues physical fail-stop, so a later exactly
    /// Armed successor is not dependent on the caller thread which performs
    /// its spawn.  The worker wrapper never removes/drops this entry; an
    /// external reaper joins it after the completion acknowledgement.
    fn cancel_worker_from_its_own_thread(
        &self,
        run_id: OutputLeaseKeepaliveRunId,
    ) -> Result<(), String> {
        let mut slot = self
            .worker
            .lock()
            .map_err(|_| "Managed output-lease keepalive worker slot was poisoned".to_string())?;
        let Some(active) = slot.active.as_ref() else {
            return Ok(());
        };
        if active.run_id != run_id {
            return Ok(());
        }
        if !active
            .join
            .as_ref()
            .is_some_and(|join| join.thread().id() == std::thread::current().id())
        {
            return Err("Managed output-lease keepalive worker-self cancellation was called from another thread".to_string());
        }
        let active = slot
            .active
            .take()
            .expect("matching active worker was checked before worker-self handoff");
        active.control.cancel_and_wake();
        slot.retiring.push(OutputLeaseKeepaliveRetiringWorkerSlot {
            worker: active,
            kind: OutputLeaseKeepaliveWorkerRetirementKind::WorkerSelfHandoff,
        });
        Ok(())
    }

    fn retain_for_external_reap(
        &self,
        worker: OutputLeaseKeepaliveWorkerSlot,
    ) -> Result<(), String> {
        let (mut slot, recovered_worker_poison) = match self.worker.lock() {
            Ok(slot) => (slot, false),
            Err(poisoned) => (poisoned.into_inner(), true),
        };
        slot.retiring.push(OutputLeaseKeepaliveRetiringWorkerSlot {
            worker,
            kind: OutputLeaseKeepaliveWorkerRetirementKind::AwaitingExternalReap,
        });
        drop(slot);
        if recovered_worker_poison {
            // This is structural recovery only: external timeout has already
            // cancelled the worker and the sole remaining obligation is to
            // retain/join its exact handle.  Clearing this mutex lets the
            // non-worker reaper complete that obligation; authority mutexes
            // remain fail-closed.
            self.worker.clear_poison();
        }
        Ok(())
    }

    /// Reap a naturally completed exact worker without waiting.  This test
    /// seam uses the same non-worker join path as production rather than
    /// removing or dropping any handle directly.
    #[cfg(test)]
    pub(crate) fn reap_completed_worker(
        &self,
        run_id: OutputLeaseKeepaliveRunId,
    ) -> Result<bool, String> {
        let Some(worker) = Self::take_completed_worker(&self.worker, Some(run_id))? else {
            return Ok(false);
        };
        Self::join_reaped_worker(worker)
    }

    /// Reap every completed retiring worker first, then one completed active
    /// worker.  A JoinHandle is moved to a local vector while holding the
    /// worker mutex and joined only after releasing it; all local joins are
    /// attempted so one panicked worker cannot strand a later completed one.
    fn reap_any_completed_worker(&self) -> Result<bool, String> {
        let retired = Self::take_all_completed_retiring_workers(&self.worker)?;
        let mut reaped = !retired.is_empty();
        let mut failures = Vec::new();
        for worker in retired {
            if let Err(error) = Self::join_reaped_worker(worker) {
                failures.push(error);
            }
        }
        if let Some(active) = Self::take_completed_active_worker(&self.worker, None)? {
            reaped = true;
            if let Err(error) = Self::join_reaped_worker(active) {
                failures.push(error);
            }
        }
        if failures.is_empty() {
            Ok(reaped)
        } else {
            Err(failures.join("; "))
        }
    }

    #[cfg(test)]
    fn take_completed_worker(
        worker_slots: &Arc<Mutex<WorkerSlot>>,
        requested_run: Option<OutputLeaseKeepaliveRunId>,
    ) -> Result<Option<OutputLeaseKeepaliveWorkerSlot>, String> {
        if let Some(active) = Self::take_completed_active_worker(worker_slots, requested_run)? {
            return Ok(Some(active));
        }
        let mut slot = worker_slots
            .lock()
            .map_err(|_| "Managed output-lease keepalive worker slot was poisoned".to_string())?;
        let current_thread = std::thread::current().id();
        let Some(index) = slot.retiring.iter().position(|retiring| {
            requested_run.is_none_or(|run_id| retiring.worker.run_id == run_id)
                && retiring
                    .worker
                    .join
                    .as_ref()
                    .is_some_and(|join| join.thread().id() != current_thread)
                && matches!(
                    retiring.worker.completion.try_recv(),
                    Ok(()) | Err(mpsc::TryRecvError::Disconnected)
                )
        }) else {
            return Ok(None);
        };
        Ok(Some(slot.retiring.swap_remove(index).worker))
    }

    fn take_completed_active_worker(
        worker_slots: &Arc<Mutex<WorkerSlot>>,
        requested_run: Option<OutputLeaseKeepaliveRunId>,
    ) -> Result<Option<OutputLeaseKeepaliveWorkerSlot>, String> {
        let mut slot = worker_slots
            .lock()
            .map_err(|_| "Managed output-lease keepalive worker slot was poisoned".to_string())?;
        let current_thread = std::thread::current().id();
        let completed = slot.active.as_ref().is_some_and(|active| {
            requested_run.is_none_or(|run_id| active.run_id == run_id)
                && active
                    .join
                    .as_ref()
                    .is_some_and(|join| join.thread().id() != current_thread)
                && matches!(
                    active.completion.try_recv(),
                    Ok(()) | Err(mpsc::TryRecvError::Disconnected)
                )
        });
        Ok(completed.then(|| {
            slot.active
                .take()
                .expect("completed active worker was checked")
        }))
    }

    fn take_all_completed_retiring_workers(
        worker_slots: &Arc<Mutex<WorkerSlot>>,
    ) -> Result<Vec<OutputLeaseKeepaliveWorkerSlot>, String> {
        let mut slot = worker_slots
            .lock()
            .map_err(|_| "Managed output-lease keepalive worker slot was poisoned".to_string())?;
        let current_thread = std::thread::current().id();
        let mut completed = Vec::new();
        let mut pending = Vec::with_capacity(slot.retiring.len());
        for retiring in slot.retiring.drain(..) {
            let joinable = retiring
                .worker
                .join
                .as_ref()
                .is_some_and(|join| join.thread().id() != current_thread);
            if joinable
                && matches!(
                    retiring.worker.completion.try_recv(),
                    Ok(()) | Err(mpsc::TryRecvError::Disconnected)
                )
            {
                completed.push(retiring.worker);
            } else {
                pending.push(retiring);
            }
        }
        slot.retiring = pending;
        Ok(completed)
    }

    fn join_reaped_worker(worker: OutputLeaseKeepaliveWorkerSlot) -> Result<bool, String> {
        let run_id = worker.run_id;
        worker
            .join
            .expect("reaped worker owns one join handle")
            .join()
            .map_err(|_| {
                format!(
                    "Managed output-lease keepalive worker {} panicked",
                    run_id.get()
                )
            })?;
        Ok(true)
    }

    /// The returning worker only acknowledges completion and schedules this
    /// helper; it never touches its own retiring entry.  The helper is a
    /// distinct thread and drains all completed retiring slots before joining
    /// their handles outside the worker mutex.  If thread creation fails, the
    /// entry remains owned by `retiring` for the next lifecycle/spawn reap.
    fn schedule_retiring_reap_after_completion(
        worker_slots: &Arc<Mutex<WorkerSlot>>,
        run_id: OutputLeaseKeepaliveRunId,
    ) {
        let has_own_retirement = match worker_slots.lock() {
            Ok(slot) => slot
                .retiring
                .iter()
                .any(|retiring| retiring.worker.run_id == run_id),
            Err(poisoned) => poisoned
                .into_inner()
                .retiring
                .iter()
                .any(|retiring| retiring.worker.run_id == run_id),
        };
        if !has_own_retirement {
            return;
        }
        let worker_slots = Arc::clone(worker_slots);
        if let Err(error) = std::thread::Builder::new()
            .name(format!(
                "syndocal-output-lease-keepalive-reaper-{}",
                run_id.get()
            ))
            .spawn(move || {
                if let Err(error) = Self::reap_all_completed_retiring_workers(&worker_slots) {
                    eprintln!("Managed output-lease retiring worker reap failed: {error}");
                }
            })
        {
            eprintln!("Managed output-lease retiring worker reaper could not start: {error}");
        }
    }

    fn reap_all_completed_retiring_workers(
        worker_slots: &Arc<Mutex<WorkerSlot>>,
    ) -> Result<bool, String> {
        let retired = Self::take_all_completed_retiring_workers(worker_slots)?;
        let mut failures = Vec::new();
        let reaped = !retired.is_empty();
        for worker in retired {
            if let Err(error) = Self::join_reaped_worker(worker) {
                failures.push(error);
            }
        }
        if failures.is_empty() {
            Ok(reaped)
        } else {
            Err(failures.join("; "))
        }
    }

    /// Start an external boundary.  It invalidates a late renewal capability
    /// under the same serial lane but does not touch physical ports.
    pub(crate) fn begin_boundary(
        &self,
        run_id: OutputLeaseKeepaliveRunId,
        boundary: OutputLeaseKeepaliveBoundary,
    ) -> Result<Option<OutputLeaseKeepaliveFailStopPlan>, String> {
        let _serial = self.operation_serial.lock().map_err(|_| {
            "Managed output-lease keepalive operation serialization lock was poisoned".to_string()
        })?;
        let plan = self
            .manager
            .lock()
            .map_err(|_| "Managed output-lease keepalive manager lock was poisoned".to_string())?
            .begin_fail_stop_for_run(run_id, boundary);
        Ok(plan)
    }

    /// Capture or resume the exact currently managed run for an app lifecycle
    /// boundary. Integration supplies only the reason, never a reconstructed
    /// run/identity. Faulted authority is reported distinctly because a
    /// different generic command must not launder it into a new boundary.
    pub(crate) fn begin_current_boundary(
        &self,
        boundary: OutputLeaseKeepaliveBoundary,
    ) -> Result<OutputLeaseKeepaliveRuntimeCurrentBoundary, String> {
        let _serial = self.operation_serial.lock().map_err(|_| {
            "Managed output-lease keepalive operation serialization lock was poisoned".to_string()
        })?;
        let mut manager = self
            .manager
            .lock()
            .map_err(|_| "Managed output-lease keepalive manager lock was poisoned".to_string())?;
        let snapshot = manager.snapshot();
        match snapshot.state {
            OutputLeaseKeepaliveState::Idle => Ok(OutputLeaseKeepaliveRuntimeCurrentBoundary::Idle),
            OutputLeaseKeepaliveState::Armed => {
                let run_id = snapshot
                    .armed
                    .expect("Armed snapshot carries its exact run")
                    .run_id();
                Ok(manager
                    .begin_fail_stop_for_run(run_id, boundary)
                    .map(OutputLeaseKeepaliveRuntimeCurrentBoundary::Captured)
                    .unwrap_or(OutputLeaseKeepaliveRuntimeCurrentBoundary::Idle))
            }
            OutputLeaseKeepaliveState::Faulting => {
                let run_id = snapshot
                    .fault
                    .expect("Faulting snapshot carries its exact run")
                    .run_id();
                Ok(manager
                    .resume_fail_stop_for_run(run_id)
                    .map(OutputLeaseKeepaliveRuntimeCurrentBoundary::Captured)
                    .unwrap_or(OutputLeaseKeepaliveRuntimeCurrentBoundary::Idle))
            }
            OutputLeaseKeepaliveState::Faulted => {
                Ok(OutputLeaseKeepaliveRuntimeCurrentBoundary::RetainedFaulted)
            }
        }
    }

    /// Window destruction is filtered before it can fence a managed run.
    /// Syndocal owns auxiliary native windows, so an unrelated display/pane
    /// close must be a strict no-op rather than a broad output shutdown.
    pub(crate) fn begin_window_destroyed_boundary_if_matches(
        &self,
        window_label: &str,
    ) -> Result<OutputLeaseKeepaliveRuntimeCurrentBoundary, String> {
        let _serial = self.operation_serial.lock().map_err(|_| {
            "Managed output-lease keepalive operation serialization lock was poisoned".to_string()
        })?;
        let mut manager = self
            .manager
            .lock()
            .map_err(|_| "Managed output-lease keepalive manager lock was poisoned".to_string())?;
        let snapshot = manager.snapshot();
        match snapshot.state {
            OutputLeaseKeepaliveState::Idle => Ok(OutputLeaseKeepaliveRuntimeCurrentBoundary::Idle),
            OutputLeaseKeepaliveState::Armed => {
                let armed = snapshot.armed.expect("Armed snapshot carries identity");
                if armed.identity().window_label() != window_label {
                    return Ok(OutputLeaseKeepaliveRuntimeCurrentBoundary::Idle);
                }
                Ok(manager
                    .begin_fail_stop_for_run(
                        armed.run_id(),
                        OutputLeaseKeepaliveBoundary::WindowIdentityChanged,
                    )
                    .map(OutputLeaseKeepaliveRuntimeCurrentBoundary::Captured)
                    .unwrap_or(OutputLeaseKeepaliveRuntimeCurrentBoundary::Idle))
            }
            OutputLeaseKeepaliveState::Faulting => {
                let fault = snapshot.fault.expect("Faulting snapshot carries identity");
                if fault.identity().window_label() != window_label {
                    return Ok(OutputLeaseKeepaliveRuntimeCurrentBoundary::Idle);
                }
                Ok(manager
                    .resume_fail_stop_for_run(fault.run_id())
                    .map(OutputLeaseKeepaliveRuntimeCurrentBoundary::Captured)
                    .unwrap_or(OutputLeaseKeepaliveRuntimeCurrentBoundary::Idle))
            }
            OutputLeaseKeepaliveState::Faulted => {
                Ok(OutputLeaseKeepaliveRuntimeCurrentBoundary::RetainedFaulted)
            }
        }
    }

    /// Test-only interception for the retired generic lifecycle API. The
    /// production surface is limited to `manual_relinquish_with`.
    #[cfg(test)]
    pub(crate) fn manual_lifecycle_decision(
        &self,
        lease_id: &str,
        operation: OutputLeaseKeepaliveManualOperation,
    ) -> Result<OutputLeaseKeepaliveManualOperationDecision, String> {
        let _serial = self.operation_serial.lock().map_err(|_| {
            "Managed output-lease keepalive operation serialization lock was poisoned".to_string()
        })?;
        Ok(self
            .manager
            .lock()
            .map_err(|_| "Managed output-lease keepalive manager lock was poisoned".to_string())?
            .manual_operation_decision(lease_id, operation))
    }

    /// Atomic managed manual-Relinquish bridge. The generic lifecycle path
    /// calls this rather than receiving an opaque permit itself. The registry
    /// closure must capture any wire-safe release response before it returns
    /// its receipt; that evidence is released to the caller only when the
    /// manager accepted the matching terminal CAS receipt.
    pub(crate) fn manual_relinquish_with<E, F>(
        &self,
        lease_id: &str,
        registry_cas_and_evidence: F,
    ) -> Result<OutputLeaseKeepaliveRuntimeManualRelinquish<E>, String>
    where
        F: FnOnce(
            OutputLeaseKeepaliveRelinquishPermit,
        ) -> Result<
            (OutputLeaseKeepaliveRelinquishCasReceipt, E),
            (
                OutputLeaseKeepaliveRelinquishPermit,
                OutputLeaseKeepalivePortError,
            ),
        >,
    {
        let _serial = self.operation_serial.lock().map_err(|_| {
            "Managed output-lease keepalive operation serialization lock was poisoned".to_string()
        })?;
        // Decide/issue under the manager lock, but never retain that lock
        // across the registry CAS.  The global serial lane keeps the three
        // phases atomic without inverting manager -> registry lock order.
        let decision = self
            .manager
            .lock()
            .map_err(|_| "Managed output-lease keepalive manager lock was poisoned".to_string())?
            .manual_relinquish_decision(lease_id);
        match decision {
            OutputLeaseKeepaliveManualOperationDecision::AllowUnmanaged => {
                Ok(OutputLeaseKeepaliveRuntimeManualRelinquish::AllowUnmanaged)
            }
            OutputLeaseKeepaliveManualOperationDecision::RejectManagedLease => {
                Ok(OutputLeaseKeepaliveRuntimeManualRelinquish::RejectManagedLease)
            }
            OutputLeaseKeepaliveManualOperationDecision::BeginFailStop(plan)
            | OutputLeaseKeepaliveManualOperationDecision::RequireFailStop(plan) => {
                Ok(OutputLeaseKeepaliveRuntimeManualRelinquish::FailStop(plan))
            }
            OutputLeaseKeepaliveManualOperationDecision::AllowRelinquish(permit) => Ok(match self
                .complete_relinquish_permit_with_under_serial(permit, registry_cas_and_evidence)?
            {
                RelinquishPermitCasOutcome::Completed(
                    OutputLeaseKeepaliveRelinquishCompletion::Completed,
                    evidence,
                ) => OutputLeaseKeepaliveRuntimeManualRelinquish::Released(evidence),
                RelinquishPermitCasOutcome::Completed(
                    OutputLeaseKeepaliveRelinquishCompletion::RetainedForRetry,
                    _,
                ) => OutputLeaseKeepaliveRuntimeManualRelinquish::RetainedForRetry,
                RelinquishPermitCasOutcome::Aborted(
                    OutputLeaseKeepaliveRelinquishCompletion::RetainedForRetry,
                ) => OutputLeaseKeepaliveRuntimeManualRelinquish::RetainedForRetry,
                // An abort path cannot complete a release; retain a defensive
                // stale result if a future core variant violates that rule.
                RelinquishPermitCasOutcome::Aborted(
                    OutputLeaseKeepaliveRelinquishCompletion::Completed,
                ) => OutputLeaseKeepaliveRuntimeManualRelinquish::StaleNoop,
                RelinquishPermitCasOutcome::StaleBeforeCas
                | RelinquishPermitCasOutcome::Aborted(
                    OutputLeaseKeepaliveRelinquishCompletion::StaleNoop,
                )
                | RelinquishPermitCasOutcome::Completed(
                    OutputLeaseKeepaliveRelinquishCompletion::StaleNoop,
                    _,
                ) => OutputLeaseKeepaliveRuntimeManualRelinquish::StaleNoop,
            }),
        }
    }

    /// Complete a permit which an earlier durable-enable admission returned
    /// as `RequireCurrentRelinquish`.  This is the only runtime boundary that
    /// consumes such a permit: it revalidates, performs the exact registry
    /// CAS, and completes the manager under one operation serial lane.  The
    /// supplied evidence is exposed only when the receipt actually completed
    /// the current faulted run.
    pub(crate) fn complete_current_relinquish_with<E, F>(
        &self,
        permit: OutputLeaseKeepaliveRelinquishPermit,
        registry_cas_and_evidence: F,
    ) -> Result<OutputLeaseKeepaliveRuntimeManualRelinquish<E>, String>
    where
        F: FnOnce(
            OutputLeaseKeepaliveRelinquishPermit,
        ) -> Result<
            (OutputLeaseKeepaliveRelinquishCasReceipt, E),
            (
                OutputLeaseKeepaliveRelinquishPermit,
                OutputLeaseKeepalivePortError,
            ),
        >,
    {
        let _serial = self.operation_serial.lock().map_err(|_| {
            "Managed output-lease keepalive operation serialization lock was poisoned".to_string()
        })?;
        match self
            .complete_relinquish_permit_with_under_serial(permit, registry_cas_and_evidence)?
        {
            RelinquishPermitCasOutcome::Completed(
                OutputLeaseKeepaliveRelinquishCompletion::Completed,
                evidence,
            ) => Ok(OutputLeaseKeepaliveRuntimeManualRelinquish::Released(
                evidence,
            )),
            RelinquishPermitCasOutcome::Completed(
                OutputLeaseKeepaliveRelinquishCompletion::RetainedForRetry,
                _,
            ) => Ok(OutputLeaseKeepaliveRuntimeManualRelinquish::RetainedForRetry),
            RelinquishPermitCasOutcome::Aborted(
                OutputLeaseKeepaliveRelinquishCompletion::RetainedForRetry,
            ) => Ok(OutputLeaseKeepaliveRuntimeManualRelinquish::RetainedForRetry),
            RelinquishPermitCasOutcome::Aborted(
                OutputLeaseKeepaliveRelinquishCompletion::Completed,
            ) => Ok(OutputLeaseKeepaliveRuntimeManualRelinquish::StaleNoop),
            RelinquishPermitCasOutcome::StaleBeforeCas
            | RelinquishPermitCasOutcome::Aborted(
                OutputLeaseKeepaliveRelinquishCompletion::StaleNoop,
            )
            | RelinquishPermitCasOutcome::Completed(
                OutputLeaseKeepaliveRelinquishCompletion::StaleNoop,
                _,
            ) => Ok(OutputLeaseKeepaliveRuntimeManualRelinquish::StaleNoop),
        }
    }

    /// The caller must already hold `operation_serial`.  No manager guard is
    /// ever live while `registry_cas_and_evidence` executes.
    fn complete_relinquish_permit_with_under_serial<E, F>(
        &self,
        permit: OutputLeaseKeepaliveRelinquishPermit,
        registry_cas_and_evidence: F,
    ) -> Result<RelinquishPermitCasOutcome<E>, String>
    where
        F: FnOnce(
            OutputLeaseKeepaliveRelinquishPermit,
        ) -> Result<
            (OutputLeaseKeepaliveRelinquishCasReceipt, E),
            (
                OutputLeaseKeepaliveRelinquishPermit,
                OutputLeaseKeepalivePortError,
            ),
        >,
    {
        let admitted = self
            .manager
            .lock()
            .map_err(|_| "Managed output-lease keepalive manager lock was poisoned".to_string())?
            .admits_relinquish_permit(&permit);
        if !admitted {
            return Ok(RelinquishPermitCasOutcome::StaleBeforeCas);
        }
        match registry_cas_and_evidence(permit) {
            Ok((receipt, evidence)) => {
                let completion = self
                    .manager
                    .lock()
                    .map_err(|_| {
                        "Managed output-lease keepalive manager lock was poisoned".to_string()
                    })?
                    .complete_relinquish_cas_receipt(receipt);
                Ok(RelinquishPermitCasOutcome::Completed(completion, evidence))
            }
            Err((permit, error)) => {
                let completion = self
                    .manager
                    .lock()
                    .map_err(|_| {
                        "Managed output-lease keepalive manager lock was poisoned".to_string()
                    })?
                    .abort_relinquish_before_registry(permit, error);
                Ok(RelinquishPermitCasOutcome::Aborted(completion))
            }
        }
    }

    /// Revalidate deferred durable Enable B after old A has completed its
    /// canonical physical fail-stop.  The registry proof is non-mutating;
    /// only the matching opaque receipt may retire A and arm B.
    pub(crate) fn prove_deferred_supersession_with<F>(
        &self,
        now_ms: u64,
        registry_proof: F,
    ) -> Result<OutputLeaseKeepaliveRuntimeSupersession, String>
    where
        F: FnOnce(
            OutputLeaseKeepaliveSupersessionCapability,
        ) -> Result<
            OutputLeaseKeepaliveSupersessionCasReceipt,
            (
                OutputLeaseKeepaliveSupersessionCapability,
                OutputLeaseKeepalivePortError,
            ),
        >,
    {
        // Preparation and completion serialize manager ownership.  The
        // non-mutating registry proof deliberately runs outside that serial
        // section: a newer durable Enable C may replace B while B is being
        // proved, in which case B's exact receipt becomes StaleNoop and this
        // same flow arbitrates C below.
        let capability = {
            let _serial = self.operation_serial.lock().map_err(|_| {
                "Managed output-lease keepalive operation serialization lock was poisoned"
                    .to_string()
            })?;
            self.manager
                .lock()
                .map_err(|_| {
                    "Managed output-lease keepalive manager lock was poisoned".to_string()
                })?
                .prepare_deferred_supersession()
                .map_err(|error| {
                    format!("Managed deferred supersession preparation rejected: {error:?}")
                })?
        };
        let completion = match registry_proof(capability) {
            Ok(receipt) => {
                let _serial = self.operation_serial.lock().map_err(|_| {
                    "Managed output-lease keepalive operation serialization lock was poisoned"
                        .to_string()
                })?;
                self.manager
                    .lock()
                    .map_err(|_| {
                        "Managed output-lease keepalive manager lock was poisoned".to_string()
                    })?
                    .complete_deferred_supersession_receipt(now_ms, receipt)
            }
            Err((capability, error)) => {
                let _serial = self.operation_serial.lock().map_err(|_| {
                    "Managed output-lease keepalive operation serialization lock was poisoned"
                        .to_string()
                })?;
                self.manager
                    .lock()
                    .map_err(|_| {
                        "Managed output-lease keepalive manager lock was poisoned".to_string()
                    })?
                    .abort_deferred_supersession_before_registry(capability, error)
            }
        };
        Ok(match completion {
            OutputLeaseKeepaliveSupersessionCompletion::Armed(run_id) => {
                OutputLeaseKeepaliveRuntimeSupersession::Armed(run_id)
            }
            OutputLeaseKeepaliveSupersessionCompletion::FailStop(plan) => {
                OutputLeaseKeepaliveRuntimeSupersession::FailStop(plan)
            }
            OutputLeaseKeepaliveSupersessionCompletion::RetainedFaulted => {
                OutputLeaseKeepaliveRuntimeSupersession::RetainedFaulted
            }
            OutputLeaseKeepaliveSupersessionCompletion::StaleNoop => {
                OutputLeaseKeepaliveRuntimeSupersession::StaleNoop
            }
        })
    }

    /// Resolve the internally retained newer durable Enable immediately after
    /// a prior current-run relinquish CAS was retained/rejected.  The deferred
    /// evidence never crosses this API: Phase A retains it so a B/C race can
    /// reject stale B and continue with the newest current candidate.
    pub(crate) fn resolve_deferred_after_relinquish<F>(
        &self,
        now_ms: u64,
        mut registry_proof: F,
    ) -> Result<OutputLeaseKeepaliveRuntimeSupersession, String>
    where
        F: FnMut(
            OutputLeaseKeepaliveSupersessionCapability,
        ) -> Result<
            OutputLeaseKeepaliveSupersessionCasReceipt,
            (
                OutputLeaseKeepaliveSupersessionCapability,
                OutputLeaseKeepalivePortError,
            ),
        >,
    {
        // A B proof can become stale while it is outside the manager lock if
        // C commits and replaces B as the retained deferred authority.  The
        // first flow already owns A's completed fail-stop, so it must perform
        // one fresh C arbitration itself rather than forcing the caller to
        // replay C.  The bound is deliberate: a third concurrent candidate
        // remains Faulted and visible instead of permitting an unbounded
        // arbitration loop under continual writes.
        for attempt in 0..2 {
            let outcome = self.prove_deferred_supersession_with(now_ms, |capability| {
                registry_proof(capability)
            })?;
            if !matches!(outcome, OutputLeaseKeepaliveRuntimeSupersession::StaleNoop) {
                return Ok(outcome);
            }
            if attempt == 1 || !self.can_retry_deferred_supersession()? {
                return Ok(OutputLeaseKeepaliveRuntimeSupersession::StaleNoop);
            }
        }
        unreachable!("bounded deferred supersession arbitration must return")
    }

    fn can_retry_deferred_supersession(&self) -> Result<bool, String> {
        let _serial = self.operation_serial.lock().map_err(|_| {
            "Managed output-lease keepalive operation serialization lock was poisoned".to_string()
        })?;
        Ok(self
            .manager
            .lock()
            .map_err(|_| "Managed output-lease keepalive manager lock was poisoned".to_string())?
            .can_retry_deferred_supersession())
    }

    /// Clock observation is serialized with boundary capture, so an old
    /// worker can never renew after an observed standby/identity boundary.
    /// Clock rollback becomes the current run's fail-stop obligation.
    pub(crate) fn observe_worker_clock(
        &self,
        run_id: OutputLeaseKeepaliveRunId,
        now_ms: u64,
    ) -> Result<OutputLeaseKeepaliveRuntimeTimer, String> {
        let _serial = self.operation_serial.lock().map_err(|_| {
            "Managed output-lease keepalive operation serialization lock was poisoned".to_string()
        })?;
        let mut manager = self
            .manager
            .lock()
            .map_err(|_| "Managed output-lease keepalive manager lock was poisoned".to_string())?;
        match manager.observe_clock_for_run(run_id, now_ms) {
            Ok(OutputLeaseKeepaliveTimerAction::Sleep(wait)) => {
                Ok(OutputLeaseKeepaliveRuntimeTimer::Sleep(wait))
            }
            Ok(OutputLeaseKeepaliveTimerAction::RenewDue(run_id)) => {
                Ok(OutputLeaseKeepaliveRuntimeTimer::RenewDue(run_id))
            }
            Err(OutputLeaseKeepaliveError::ClockRollback) => Ok(manager
                .begin_fail_stop_for_run(run_id, OutputLeaseKeepaliveBoundary::ClockRollback)
                .map_or(
                    OutputLeaseKeepaliveRuntimeTimer::StaleNoop,
                    OutputLeaseKeepaliveRuntimeTimer::FailStop,
                )),
            Err(OutputLeaseKeepaliveError::StaleRun | OutputLeaseKeepaliveError::NotArmed) => {
                Ok(OutputLeaseKeepaliveRuntimeTimer::StaleNoop)
            }
            Err(error) => Err(format!(
                "Managed output-lease worker clock observation rejected: {error:?}"
            )),
        }
    }

    /// Execute all seven physical fail-stop steps without holding runtime,
    /// manager or registry locks.  Completion is separately serialized.
    pub(crate) fn execute_fail_stop<P: OutputLeaseKeepaliveFailStopPorts>(
        &self,
        ports: &mut P,
        plan: OutputLeaseKeepaliveFailStopPlan,
    ) -> Result<OutputLeaseKeepaliveFailStopCompletion, String> {
        let execution = execute_fail_stop_plan(ports, plan);
        let OutputLeaseKeepaliveFailStopExecutionOutcome::Executed(execution) = execution else {
            return Ok(OutputLeaseKeepaliveFailStopCompletion::StaleNoop);
        };
        let _serial = self.operation_serial.lock().map_err(|_| {
            "Managed output-lease keepalive operation serialization lock was poisoned".to_string()
        })?;
        self.manager
            .lock()
            .map_err(|_| "Managed output-lease keepalive manager lock was poisoned".to_string())?
            .complete_fail_stop(execution)
            .map_err(|error| {
                format!("Managed output-lease fail-stop completion rejected: {error:?}")
            })
    }

    /// Manager prepare -> exact registry CAS -> manager completion is one
    /// serialized operation.  The supplied closure must lock and mutate only
    /// the exact registry; it must not call physical ports or recurse here.
    pub(crate) fn renew_once<F>(
        &self,
        now_ms: u64,
        run_id: OutputLeaseKeepaliveRunId,
        registry_cas: F,
    ) -> Result<OutputLeaseKeepaliveRuntimeRenewal, String>
    where
        F: FnOnce(
            OutputLeaseKeepaliveRenewalCapability,
        ) -> Result<
            OutputLeaseKeepaliveRenewalCasReceipt,
            (
                OutputLeaseKeepaliveRenewalCapability,
                OutputLeaseKeepalivePortError,
            ),
        >,
    {
        let _serial = self.operation_serial.lock().map_err(|_| {
            "Managed output-lease keepalive operation serialization lock was poisoned".to_string()
        })?;
        let capability = self
            .manager
            .lock()
            .map_err(|_| "Managed output-lease keepalive manager lock was poisoned".to_string())?
            .prepare_due_renewal_for_run(run_id, now_ms)
            .map_err(|error| {
                format!("Managed output-lease renewal preparation rejected: {error:?}")
            })?;
        let completion = match registry_cas(capability) {
            Ok(receipt) => self
                .manager
                .lock()
                .map_err(|_| {
                    "Managed output-lease keepalive manager lock was poisoned".to_string()
                })?
                .complete_renewal_receipt(now_ms, receipt)
                .map_err(|error| {
                    format!("Managed output-lease renewal completion rejected: {error:?}")
                })?,
            Err((capability, error)) => self
                .manager
                .lock()
                .map_err(|_| {
                    "Managed output-lease keepalive manager lock was poisoned".to_string()
                })?
                .abort_due_renewal_before_registry(capability, error),
        };
        Ok(match completion {
            OutputLeaseKeepaliveRenewalCompletion::Renewed => {
                OutputLeaseKeepaliveRuntimeRenewal::Renewed
            }
            OutputLeaseKeepaliveRenewalCompletion::FailStop(plan) => {
                OutputLeaseKeepaliveRuntimeRenewal::FailStop(plan)
            }
            OutputLeaseKeepaliveRenewalCompletion::StaleNoop => {
                OutputLeaseKeepaliveRuntimeRenewal::StaleNoop
            }
        })
    }

    /// Request the automatic-boundary release permit only after a successful
    /// canonical fail-stop.  The exact registry CAS and result completion are
    /// kept in the same serial lane; a failed CAS retains Faulted for retry.
    pub(crate) fn relinquish_after_successful_boundary<F>(
        &self,
        run_id: OutputLeaseKeepaliveRunId,
        identity: &OutputLeaseKeepaliveIdentity,
        registry_cas: F,
    ) -> Result<Option<OutputLeaseKeepaliveRelinquishCompletion>, String>
    where
        F: FnOnce(
            OutputLeaseKeepaliveRelinquishPermit,
        ) -> Result<
            OutputLeaseKeepaliveRelinquishCasReceipt,
            (
                OutputLeaseKeepaliveRelinquishPermit,
                OutputLeaseKeepalivePortError,
            ),
        >,
    {
        let _serial = self.operation_serial.lock().map_err(|_| {
            "Managed output-lease keepalive operation serialization lock was poisoned".to_string()
        })?;
        let permit = self
            .manager
            .lock()
            .map_err(|_| "Managed output-lease keepalive manager lock was poisoned".to_string())?
            .permit_relinquish_after_successful_boundary(run_id, identity);
        let Some(permit) = permit else {
            return Ok(None);
        };
        self.complete_relinquish_permit_with_under_serial(permit, |permit| {
            registry_cas(permit).map(|receipt| (receipt, ()))
        })
        .map(|outcome| match outcome {
            RelinquishPermitCasOutcome::StaleBeforeCas => {
                Some(OutputLeaseKeepaliveRelinquishCompletion::StaleNoop)
            }
            RelinquishPermitCasOutcome::Aborted(completion) => Some(completion),
            RelinquishPermitCasOutcome::Completed(completion, _) => Some(completion),
        })
    }

    /// Concrete renewal loop used by the production wiring.  It is deliberately
    /// narrow: no lifecycle command is accepted here, and every failure first
    /// captures the exact run's canonical fail-stop plan.  Physical I/O and
    /// its bounded waits occur only through `ProductionKeepaliveFailStopPorts`
    /// after all authority locks are released.
    pub(crate) fn run_production_worker(
        app: tauri::AppHandle,
        runtime: Arc<Self>,
        control: Arc<OutputLeaseKeepaliveWorkerControl>,
    ) {
        // AppState is managed by Tauri, not Arc-owned by the worker.  The
        // handle is the sole cross-thread capability and resolves the exact
        // live application state when the worker begins.
        let state = app.state::<super::AppState>();
        let run_id = control.run_id();
        while !control.is_cancelled() {
            let now_ms = match state.output_lease_now_ms() {
                Ok(now_ms) => now_ms,
                Err(error) => {
                    Self::finish_production_worker_fail_stop(
                        &app,
                        &state,
                        &runtime,
                        run_id,
                        OutputLeaseKeepaliveBoundary::ClockFailed,
                        format!("Managed keepalive clock failed: {error}"),
                    );
                    break;
                }
            };
            match runtime.observe_worker_clock(run_id, now_ms) {
                Ok(OutputLeaseKeepaliveRuntimeTimer::Sleep(wait)) => {
                    let delay_ms = wait.deadline_ms.saturating_sub(now_ms);
                    if !control
                        .wait_until_or_cancelled(Instant::now() + Duration::from_millis(delay_ms))
                    {
                        break;
                    }
                }
                Ok(OutputLeaseKeepaliveRuntimeTimer::RenewDue(run)) => {
                    let renewal = runtime.renew_once(now_ms, run, |capability| {
                        let mut registry = match state.output_lease_registry.lock() {
                            Ok(registry) => registry,
                            Err(_) => {
                                return Err((
                                    capability,
                                    OutputLeaseKeepalivePortError::detail(
                                        "Managed output-lease registry lock was poisoned before renewal",
                                    )
                                    .expect("static registry poison detail is bounded"),
                                ));
                            }
                        };
                        Ok(registry.renew_managed_exact_both(capability, now_ms))
                    });
                    match renewal {
                        Ok(OutputLeaseKeepaliveRuntimeRenewal::Renewed) => {}
                        Ok(OutputLeaseKeepaliveRuntimeRenewal::FailStop(plan)) => {
                            Self::complete_production_fail_stop(
                                &app,
                                &state,
                                &runtime,
                                run_id,
                                plan,
                                "Managed keepalive renewal was rejected",
                                OutputLeaseKeepaliveFailStopExecutionContext::WorkerSelf(run_id),
                            );
                            break;
                        }
                        Ok(OutputLeaseKeepaliveRuntimeRenewal::StaleNoop) => break,
                        Err(error) => {
                            Self::finish_production_worker_fail_stop(
                                &app,
                                &state,
                                &runtime,
                                run_id,
                                OutputLeaseKeepaliveBoundary::RenewalFailed,
                                format!("Managed keepalive renewal failed: {error}"),
                            );
                            break;
                        }
                    }
                }
                Ok(OutputLeaseKeepaliveRuntimeTimer::FailStop(plan)) => {
                    Self::complete_production_fail_stop(
                        &app,
                        &state,
                        &runtime,
                        run_id,
                        plan,
                        "Managed keepalive clock rollback",
                        OutputLeaseKeepaliveFailStopExecutionContext::WorkerSelf(run_id),
                    );
                    break;
                }
                Ok(OutputLeaseKeepaliveRuntimeTimer::StaleNoop) => break,
                Err(error) => {
                    Self::finish_production_worker_fail_stop(
                        &app,
                        &state,
                        &runtime,
                        run_id,
                        OutputLeaseKeepaliveBoundary::ClockFailed,
                        format!("Managed keepalive clock observation failed: {error}"),
                    );
                    break;
                }
            }
        }
    }

    fn finish_production_worker_fail_stop(
        app: &tauri::AppHandle,
        state: &super::AppState,
        runtime: &Arc<Self>,
        run_id: OutputLeaseKeepaliveRunId,
        boundary: OutputLeaseKeepaliveBoundary,
        detail: String,
    ) {
        match runtime.begin_boundary(run_id, boundary) {
            Ok(Some(plan)) => Self::complete_production_fail_stop(
                app,
                state,
                runtime,
                run_id,
                plan,
                &detail,
                OutputLeaseKeepaliveFailStopExecutionContext::WorkerSelf(run_id),
            ),
            Ok(None) => eprintln!("{detail}; managed run was already fenced"),
            Err(error) => eprintln!("{detail}; unable to capture fail-stop plan: {error}"),
        }
    }

    fn complete_production_fail_stop(
        app: &tauri::AppHandle,
        state: &super::AppState,
        runtime: &Arc<Self>,
        run_id: OutputLeaseKeepaliveRunId,
        plan: OutputLeaseKeepaliveFailStopPlan,
        detail: &str,
        context: OutputLeaseKeepaliveFailStopExecutionContext,
    ) {
        let mut ports = match context {
            OutputLeaseKeepaliveFailStopExecutionContext::WorkerSelf(worker_run)
                if worker_run == run_id =>
            {
                ProductionKeepaliveFailStopPorts::for_worker(app, state, runtime, run_id, detail)
            }
            _ => ProductionKeepaliveFailStopPorts::new(app, state, runtime, detail),
        };
        match runtime.execute_fail_stop(&mut ports, plan) {
            Ok(OutputLeaseKeepaliveFailStopCompletion::Completed) => {}
            Ok(OutputLeaseKeepaliveFailStopCompletion::StaleNoop) => {
                eprintln!("{detail}; fail-stop became stale");
                return;
            }
            Err(error) => {
                eprintln!("{detail}; fail-stop completion failed: {error}");
                return;
            }
        }
        let Some(identity) = runtime
            .identity_for_exact_run(run_id)
            .unwrap_or_else(|error| {
                eprintln!("{detail}; managed identity read failed: {error}");
                None
            })
        else {
            eprintln!(
                "{detail}; faulted run identity is unavailable; explicit Enable remains required"
            );
            return;
        };
        let now_ms = match state.output_lease_now_ms() {
            Ok(now_ms) => now_ms,
            Err(error) => {
                eprintln!("{detail}; release clock failed: {error}");
                return;
            }
        };
        match runtime.relinquish_after_successful_boundary(run_id, &identity, |permit| {
            let mut registry = match state.output_lease_registry.lock() {
                Ok(registry) => registry,
                Err(_) => {
                    return Err((
                        permit,
                        OutputLeaseKeepalivePortError::detail(
                            "Managed output-lease registry lock was poisoned before relinquish",
                        )
                        .expect("static registry poison detail is bounded"),
                    ));
                }
            };
            Ok(registry.relinquish_managed_exact_both(permit, now_ms))
        }) {
            Ok(Some(OutputLeaseKeepaliveRelinquishCompletion::Completed)) => {}
            Ok(Some(OutputLeaseKeepaliveRelinquishCompletion::RetainedForRetry)) => {
                // A newer durable Enable was retained by Phase A while the old
                // permit was in flight. It must be proven immediately; waiting
                // for another Enable would strand a safe, current B/C run.
                Self::resolve_production_deferred_after_relinquish(app, state, runtime, detail);
            }
            Ok(Some(OutputLeaseKeepaliveRelinquishCompletion::StaleNoop)) => {
                eprintln!("{detail}; exact registry relinquish became stale")
            }
            Ok(None) => {
                eprintln!("{detail}; successful fail-stop did not admit automatic relinquish")
            }
            Err(error) => eprintln!("{detail}; automatic relinquish failed: {error}"),
        }
    }

    fn resolve_production_deferred_after_relinquish(
        app: &tauri::AppHandle,
        state: &super::AppState,
        runtime: &Arc<Self>,
        detail: &str,
    ) {
        let now_ms = match state.output_lease_now_ms() {
            Ok(now_ms) => now_ms,
            Err(error) => {
                eprintln!("{detail}; deferred supersession clock failed: {error}");
                return;
            }
        };
        let outcome = runtime.resolve_deferred_after_relinquish(now_ms, |capability| {
            let mut registry = match state.output_lease_registry.lock() {
                Ok(registry) => registry,
                Err(_) => {
                    return Err((
                        capability,
                        OutputLeaseKeepalivePortError::detail(
                            "Managed output-lease registry lock was poisoned before supersession proof",
                        )
                        .expect("static registry poison detail is bounded"),
                    ));
                }
            };
            Ok(registry.prove_managed_exact_both_supersession(capability, now_ms))
        });
        match outcome {
            Ok(OutputLeaseKeepaliveRuntimeSupersession::Armed(run_id)) => {
                Self::spawn_production_worker_after_arm(app, state, runtime, run_id, detail)
            }
            Ok(OutputLeaseKeepaliveRuntimeSupersession::FailStop(plan)) => {
                let run_id = plan.run_id();
                Self::complete_production_fail_stop(
                    app,
                    state,
                    runtime,
                    run_id,
                    plan,
                    detail,
                    OutputLeaseKeepaliveFailStopExecutionContext::ExternalBoundary,
                );
            }
            Ok(OutputLeaseKeepaliveRuntimeSupersession::RetainedFaulted) => {
                eprintln!("{detail}; deferred supersession was rejected and remains faulted")
            }
            Ok(OutputLeaseKeepaliveRuntimeSupersession::StaleNoop) => {
                eprintln!("{detail}; deferred supersession became stale")
            }
            Err(error) => eprintln!("{detail}; deferred supersession failed: {error}"),
        }
    }

    fn spawn_production_worker_after_arm(
        app: &tauri::AppHandle,
        state: &super::AppState,
        runtime: &Arc<Self>,
        run_id: OutputLeaseKeepaliveRunId,
        detail: &str,
    ) {
        let worker_app = app.clone();
        let worker_runtime = Arc::clone(runtime);
        if let Err(error) = runtime.spawn_worker(run_id, move |control| {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                Self::run_production_worker(
                    worker_app.clone(),
                    Arc::clone(&worker_runtime),
                    Arc::clone(&control),
                );
            }));
            let unexpected = result.is_err()
                || (!control.is_cancelled()
                    && worker_runtime
                        .snapshot_state()
                        .is_ok_and(|state| state == OutputLeaseKeepaliveState::Armed));
            if !unexpected {
                return;
            }
            let detail = if result.is_err() {
                "Managed output-lease keepalive worker panicked"
            } else {
                "Managed output-lease keepalive worker exited while still Armed"
            };
            let error = OutputLeaseKeepalivePortError::detail(detail)
                .expect("static worker termination detail is bounded");
            match worker_runtime.record_worker_termination(run_id, error) {
                Ok(Some(plan)) => Self::complete_production_fail_stop(
                    &worker_app,
                    &worker_app.state::<super::AppState>(),
                    &worker_runtime,
                    run_id,
                    plan,
                    detail,
                    OutputLeaseKeepaliveFailStopExecutionContext::WorkerSelf(run_id),
                ),
                Ok(None) => {}
                Err(error) => eprintln!("{detail}; worker termination was not recorded: {error}"),
            }
        }) {
            match runtime.record_worker_spawn_failure(
                run_id,
                crate::output_lease::output_lease_keepalive::output_lease_keepalive_adapter_port_error(
                    error,
                ),
            ) {
                Ok(Some(plan)) => Self::complete_production_fail_stop(
                    app,
                    state,
                    runtime,
                    run_id,
                    plan,
                    detail,
                    OutputLeaseKeepaliveFailStopExecutionContext::ExternalBoundary,
                ),
                Ok(None) => eprintln!("{detail}; worker spawn failure was already fenced"),
                Err(error) => {
                    eprintln!("{detail}; worker spawn failure could not be recorded: {error}")
                }
            }
        }
    }
}

/// Concrete application adapter for the manager's seven physical fail-stop
/// ports.  It is created only after a boundary plan was captured, therefore it
/// holds no manager, registry, durable-journal, coordinator or owner-rotation
/// lock while it performs engine/DMX/Spout/native/persistence work.
pub(crate) struct ProductionKeepaliveFailStopPorts<'a> {
    app: &'a tauri::AppHandle,
    state: &'a super::AppState,
    runtime: &'a OutputLeaseKeepaliveRuntime,
    execution_context: OutputLeaseKeepaliveFailStopExecutionContext,
    teardown_lease: Option<engine::OutputOwnershipTeardownLease>,
    safety: Option<engine::SafetyBlackoutAuthority>,
    failure_epoch: Option<u64>,
    failure_detail: String,
}

impl<'a> ProductionKeepaliveFailStopPorts<'a> {
    pub(crate) fn new(
        app: &'a tauri::AppHandle,
        state: &'a super::AppState,
        runtime: &'a OutputLeaseKeepaliveRuntime,
        failure_detail: impl Into<String>,
    ) -> Self {
        let failure_detail = Self::admit_diagnostic_or_rejection(failure_detail);
        Self {
            app,
            state,
            runtime,
            execution_context: OutputLeaseKeepaliveFailStopExecutionContext::ExternalBoundary,
            teardown_lease: None,
            safety: None,
            failure_epoch: None,
            failure_detail,
        }
    }

    /// Use only from the exact worker which detected its own renewal/clock
    /// failure. `CancelRenewal` remains a canonical successful port but never
    /// consumes or joins that worker's own handle.
    pub(crate) fn for_worker(
        app: &'a tauri::AppHandle,
        state: &'a super::AppState,
        runtime: &'a OutputLeaseKeepaliveRuntime,
        run_id: OutputLeaseKeepaliveRunId,
        failure_detail: impl Into<String>,
    ) -> Self {
        let mut ports = Self::new(app, state, runtime, failure_detail);
        ports.execution_context = OutputLeaseKeepaliveFailStopExecutionContext::WorkerSelf(run_id);
        ports
    }

    fn admit_diagnostic_or_rejection(value: impl Into<String>) -> String {
        let value = value.into();
        if crate::output_lease::output_lease_keepalive::OutputLeaseKeepalivePortError::detail(
            value.clone(),
        )
        .is_ok()
        {
            return value;
        }
        crate::output_lease::output_lease_keepalive::OUTPUT_LEASE_KEEPALIVE_DIAGNOSTIC_REJECTED
            .to_string()
    }

    fn port_error(
        error: impl Into<String>,
    ) -> crate::output_lease::output_lease_keepalive::OutputLeaseKeepalivePortError {
        crate::output_lease::output_lease_keepalive::output_lease_keepalive_adapter_port_error(
            error,
        )
    }

    fn record_failure_detail(&mut self, additional: impl AsRef<str>) {
        let suffix = Self::admit_diagnostic_or_rejection(additional.as_ref());
        if self.failure_detail.is_empty() {
            self.failure_detail = suffix;
            return;
        }
        let candidate = format!("{}; {suffix}", self.failure_detail);
        if crate::output_lease::output_lease_keepalive::OutputLeaseKeepalivePortError::detail(
            candidate.clone(),
        )
        .is_ok()
        {
            self.failure_detail = candidate;
        } else {
            self.failure_detail = crate::output_lease::output_lease_keepalive::OUTPUT_LEASE_KEEPALIVE_DIAGNOSTIC_REJECTED
                .to_string();
        }
    }
}

impl OutputLeaseKeepaliveFailStopPorts for ProductionKeepaliveFailStopPorts<'_> {
    fn cancel_renewal(
        &mut self,
        plan: &OutputLeaseKeepaliveFailStopPlan,
    ) -> Result<(), crate::output_lease::output_lease_keepalive::OutputLeaseKeepalivePortError>
    {
        match self
            .runtime
            .cancel_worker_for_fail_stop(plan.run_id(), self.execution_context)
        {
            Ok(()) => Ok(()),
            Err(error) => {
                self.record_failure_detail(&error);
                Err(Self::port_error(error))
            }
        }
    }

    fn request_safety_blackout(
        &mut self,
        _: &OutputLeaseKeepaliveFailStopPlan,
    ) -> Result<(), crate::output_lease::output_lease_keepalive::OutputLeaseKeepalivePortError>
    {
        let result = self
            .state
            .engine
            .safety_blackout_engage_published(Instant::now() + Duration::from_secs(3));
        let authority = self.state.engine.safety_blackout_authority();
        self.safety = Some(authority);
        match result {
            Ok(_) if authority.engaged => Ok(()),
            Ok(_) => {
                let error = "Safety blackout acknowledgement did not leave S0 engaged".to_string();
                self.record_failure_detail(&error);
                Err(Self::port_error(error))
            }
            Err(error) => {
                self.record_failure_detail(&error);
                Err(Self::port_error(error))
            }
        }
    }

    fn advance_failure_fence(
        &mut self,
        _: &OutputLeaseKeepaliveFailStopPlan,
    ) -> Result<(), crate::output_lease::output_lease_keepalive::OutputLeaseKeepalivePortError>
    {
        let lease = self
            .state
            .engine
            .begin_output_ownership_failure_fence(self.failure_detail.clone());
        let failure_epoch = self.state.engine.output_ownership_status().epoch;
        self.teardown_lease = Some(lease);
        self.failure_epoch = Some(failure_epoch);
        Ok(())
    }

    fn retire_dmx_usb_artnet(
        &mut self,
        _: &OutputLeaseKeepaliveFailStopPlan,
    ) -> Result<(), crate::output_lease::output_lease_keepalive::OutputLeaseKeepalivePortError>
    {
        let safety = self
            .safety
            .unwrap_or_else(|| self.state.engine.safety_blackout_authority());
        let failure_epoch = self
            .failure_epoch
            .unwrap_or_else(|| self.state.engine.output_ownership_status().epoch);
        let result = self
            .state
            .engine
            .begin_retire_managed_show_dmx_after_safety_blackout(
                safety,
                failure_epoch,
                Instant::now() + Duration::from_secs(3),
            )
            .map_err(|error| format!("Managed DMX retire admission failed: {error:?}"))
            .and_then(|operation| {
                operation
                    .wait(Duration::from_secs(5))
                    .map(|_| ())
                    .map_err(|error| format!("Managed DMX retire completion failed: {error:?}"))
            });
        if let Err(error) = result {
            self.record_failure_detail(&error);
            return Err(Self::port_error(error));
        }
        Ok(())
    }

    fn retire_spout_native(
        &mut self,
        _: &OutputLeaseKeepaliveFailStopPlan,
    ) -> Result<(), crate::output_lease::output_lease_keepalive::OutputLeaseKeepalivePortError>
    {
        let mut errors = Vec::new();
        match super::sync_output_ownership_routes_for_role(
            self.state,
            protocol::MachineOutputRole::Standby,
            #[cfg(any(
                feature = "ndi",
                all(feature = "spout", target_os = "windows", target_arch = "x86_64")
            ))]
            None,
        ) {
            Ok(response) if response.report.stop_failed.is_empty() => {}
            Ok(response) => errors.push(format!(
                "External video route retirement failed for {} route(s)",
                response.report.stop_failed.len()
            )),
            Err(error) => errors.push(format!("External video route retirement failed: {error}")),
        }
        if let Err(error) = super::retire_native_video_output_windows(
            self.app,
            &self.state.native_video_output_workers,
        ) {
            errors.push(format!("Native display retirement failed: {error}"));
        }
        if errors.is_empty() {
            Ok(())
        } else {
            let error = errors.join("; ");
            self.record_failure_detail(&error);
            Err(Self::port_error(error))
        }
    }

    fn persist_standby_failed(
        &mut self,
        _: &OutputLeaseKeepaliveFailStopPlan,
    ) -> Result<(), crate::output_lease::output_lease_keepalive::OutputLeaseKeepalivePortError>
    {
        let path = super::output_ownership_state_path(self.app);
        match path.and_then(|path| {
            super::persist_output_ownership_role_to_path(
                &path,
                protocol::MachineOutputRole::Standby,
            )
        }) {
            Ok(()) => {
                let desired_role = self.state.engine.output_ownership_status().desired_role;
                self.state
                    .engine
                    .mark_output_ownership_durable_standby_failure(
                        desired_role,
                        self.failure_detail.clone(),
                    );
                Ok(())
            }
            Err(error) => {
                let error =
                    format!("Durable Standby failure marker could not be persisted: {error}");
                self.record_failure_detail(&error);
                self.state
                    .engine
                    .mark_output_ownership_transition_failure(self.failure_detail.clone());
                Err(Self::port_error(error))
            }
        }
    }
}

#[cfg(test)]
#[path = "output_lease_keepalive_runtime_tests.rs"]
mod tests;
