//! Strict same-machine Spout sender lifecycle for the show pair.
//!
//! This module owns the show-only transaction and presentation state. The
//! generic Spout transport exports only its worker adapter; it remains usable
//! for arbitrary sender routes without this contract.

use std::{
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Condvar, Mutex,
    },
    time::{Duration, Instant},
};

#[cfg(test)]
use crate::spout_transport::SpoutOutputSender;
use crate::{
    show_spout_outputs::{
        decide_show_spout_ensure, validate_show_spout_outputs, ShowSpoutBlackFrame,
        ShowSpoutEnsureDecision, ShowSpoutOutputs, ShowSpoutPresentationEvent,
        ShowSpoutPresentationState,
    },
    spout_transport::{SpoutRouteWorker, SpoutTransportState},
};
use engine::{EngineHandle, OutputOwnershipActivation};

pub(crate) enum ShowSpoutPrepareOutcome {
    Pending(ShowSpoutPendingOutputPair),
    /// The old exact engine pair still needs a serialized retirement ACK.
    /// This token is state-reserved before the caller drops transport locks
    /// to submit that ACK; it never permits sender construction in this R4.
    RetryBlockedRetirement(ShowSpoutBlockedRetirementRetry),
    NoOp,
}

/// Obtained only after both initial opaque-black acknowledgements, exact
/// engine-pair verification, and active-pair handoff. `commit` is deliberately
/// infallible: it is invoked after the durable output-lease registry commit,
/// so it may only open the worker's atomic live-content gate.
pub(crate) struct ShowSpoutLiveTransferCommitToken {
    control: Arc<ShowSpoutWorkerControl>,
}

impl ShowSpoutLiveTransferCommitToken {
    pub(crate) fn commit(self) {
        self.control
            .live_transfer_committed
            .store(true, Ordering::Release);
    }
}

/// A physical sender fault retains the exact engine pair identity so the
/// caller can compensate the engine publication after both SDK senders have
/// retired. The IDs are never reconstructed from a later snapshot.
#[derive(Debug, Clone)]
pub(crate) struct ShowSpoutTransportFailure {
    pub(crate) message: String,
    /// Active-worker fault cleanup is autonomous: after both SDK senders were
    /// physically dropped, the second worker retires this exact pair once.
    /// Harvest reads that recorded outcome and never issues a second retire.
    pub(crate) automatic_engine_retirement: Option<Result<(), String>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ShowSpoutReapingKind {
    AuthorityChange,
    FailedWorker,
}

impl ShowSpoutReapingKind {
    fn label(self) -> &'static str {
        match self {
            Self::AuthorityChange => "authority-transition",
            Self::FailedWorker => "failed-worker",
        }
    }
}

/// State-owned interlock that spans the mutex-free physical stop/join and the
/// second worker's exact engine-retirement acknowledgement. `active = None`
/// is therefore never sufficient evidence that fixed sender names may be
/// reused: a fresh R4 must observe this receipt as absent *and* any prior
/// exact engine cleanup as resolved.
#[derive(Clone)]
struct ShowSpoutReaping {
    expected: ShowSpoutOutputs,
    kind: ShowSpoutReapingKind,
    reason: String,
}

/// Detached only while both physical SDK senders are being joined. Removing
/// the pair from `ShowSpoutTransportState::active` before this object is
/// returned makes a concurrent observer fail closed rather than receiving an
/// active receipt during teardown. Its `retire` method must run without the
/// transport-state mutex: the second worker's post-drop callback can wait for
/// the exact engine retirement acknowledgement.
pub(crate) struct ShowSpoutRetiringOutputPair {
    pair: Option<ShowSpoutOutputPair>,
    reaping: ShowSpoutReaping,
    control: Arc<ShowSpoutWorkerControl>,
    worker_failure: Option<String>,
}

pub(crate) struct ShowSpoutPhysicalRetirement {
    expected: ShowSpoutOutputs,
    reaping: ShowSpoutReaping,
    worker_failure: Option<String>,
    cleanup: Result<(), String>,
    automatic_engine_retirement: Option<Result<(), String>>,
}

impl ShowSpoutRetiringOutputPair {
    fn from_pair(
        pair: ShowSpoutOutputPair,
        reaping: ShowSpoutReaping,
        worker_failure: Option<String>,
    ) -> Self {
        Self {
            reaping,
            control: Arc::clone(&pair.control),
            pair: Some(pair),
            worker_failure,
        }
    }

    /// This owns the physical stop/join and deliberately runs outside the
    /// `ShowSpoutTransportState` mutex. The worker's second physical-drop
    /// callback retires the exact engine pair only after both SDK resources
    /// have been destroyed.
    pub(crate) fn retire(mut self) -> ShowSpoutPhysicalRetirement {
        let cleanup = self
            .pair
            .take()
            .expect("strict show Spout detached pair was already consumed")
            .retire();
        ShowSpoutPhysicalRetirement {
            expected: self.reaping.expected.clone(),
            reaping: self.reaping,
            worker_failure: self.worker_failure,
            cleanup,
            automatic_engine_retirement: self.control.automatic_engine_retirement(),
        }
    }
}

impl std::fmt::Display for ShowSpoutTransportFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

type ShowSpoutAuthorityValidator = Mutex<Box<dyn FnMut() -> Result<(), String> + Send + 'static>>;

/// Shared by both sender workers. The fixed black frame is allocated once,
/// remains immutable, and is borrowed directly by the physical send closure.
pub(crate) struct ShowSpoutWorkerControl {
    state: Mutex<ShowSpoutPresentationState>,
    black: ShowSpoutBlackFrame,
    authority_validator: ShowSpoutAuthorityValidator,
    published_senders: AtomicUsize,
    // First-black completion proves that both SDK workers exist, but the R4
    // receipt is still provisional until the output lease commit succeeds.
    // Workers must keep sending opaque black through that latter boundary.
    live_transfer_committed: AtomicBool,
    first_black: Mutex<ShowSpoutInitialBlackState>,
    first_black_changed: Condvar,
    engine: EngineHandle,
    expected: ShowSpoutOutputs,
    teardown: Mutex<ShowSpoutTeardownState>,
}

#[derive(Default)]
struct ShowSpoutInitialBlackState {
    completed_senders: usize,
    failure: Option<String>,
}

#[derive(Default)]
struct ShowSpoutTeardownState {
    physically_dropped_senders: usize,
    // Pending construction and first-black failures stay with their explicit
    // caller rollback. Only an active pair transfers terminal compensation
    // to the two worker exits.
    automatic_retirement_armed: bool,
    automatic_engine_retirement_started: bool,
    automatic_engine_retirement: Option<Result<(), String>>,
}

impl ShowSpoutWorkerControl {
    fn new<F>(engine: EngineHandle, expected: ShowSpoutOutputs, authority_validator: F) -> Self
    where
        F: FnMut() -> Result<(), String> + Send + 'static,
    {
        Self {
            state: Mutex::new(ShowSpoutPresentationState::initial()),
            black: ShowSpoutBlackFrame::new(),
            authority_validator: Mutex::new(Box::new(authority_validator)),
            published_senders: AtomicUsize::new(0),
            live_transfer_committed: AtomicBool::new(false),
            first_black: Mutex::new(ShowSpoutInitialBlackState::default()),
            first_black_changed: Condvar::new(),
            engine,
            expected,
            teardown: Mutex::new(ShowSpoutTeardownState::default()),
        }
    }

    pub(crate) fn revalidate(&self, phase: &str) -> Result<(), String> {
        let result = self.authority_validator.lock().map_err(|_| {
            format!("strict show Spout authority validator lock was poisoned at {phase}")
        })?();
        if let Err(error) = result {
            self.mark_authority_lost();
            return Err(format!(
                "strict show Spout authority was revoked at {phase}: {error}"
            ));
        }
        Ok(())
    }

    fn prepare_keepalive_black(&self) -> Result<(), String> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "strict show Spout presentation state lock was poisoned".to_string())?;
        let reset = if matches!(*state, ShowSpoutPresentationState::Fault) {
            state
                .apply(ShowSpoutPresentationEvent::AuthorityRestored)
                .map_err(|error| error.to_string())?
        } else {
            ShowSpoutPresentationState::initial()
        };
        *state = reset
            .apply(ShowSpoutPresentationEvent::BeginTransition)
            .map_err(|error| error.to_string())?
            .apply(ShowSpoutPresentationEvent::SenderEstablished)
            .map_err(|error| error.to_string())?
            .apply(ShowSpoutPresentationEvent::ContentStopped)
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    fn set_content_live(&self) -> Result<(), String> {
        self.transition(ShowSpoutPresentationEvent::ContentLive)
    }

    fn set_keepalive_black(&self) -> Result<(), String> {
        self.transition(ShowSpoutPresentationEvent::ContentStopped)
    }

    /// Reconciles directly from the authoritative engine timeline on every
    /// show worker tick. This prevents an IPC/UI synchronization lag from
    /// leaving stopped content visible while retaining both registrations.
    pub(crate) fn sync_timeline_playing(&self, timeline_playing: bool) -> Result<(), String> {
        if timeline_playing && self.live_transfer_committed.load(Ordering::Acquire) {
            self.set_content_live()
        } else {
            self.set_keepalive_black()
        }
    }

    #[cfg(test)]
    fn commit_live_transfer_for_test(&self) {
        self.live_transfer_committed.store(true, Ordering::Release);
    }

    fn transition(&self, event: ShowSpoutPresentationEvent) -> Result<(), String> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "strict show Spout presentation state lock was poisoned".to_string())?;
        *state = state.apply(event).map_err(|error| error.to_string())?;
        Ok(())
    }

    pub(crate) fn presentation_is_keepalive_black(&self) -> Result<bool, String> {
        let state = *self
            .state
            .lock()
            .map_err(|_| "strict show Spout presentation state lock was poisoned".to_string())?;
        if matches!(state, ShowSpoutPresentationState::Fault) {
            return Err(
                "strict show Spout authority is faulted; explicit reauthorization is required"
                    .to_string(),
            );
        }
        if !state.sender_remains_open() {
            return Err("strict show Spout sender is not established".to_string());
        }
        Ok(state.sends_black_frame())
    }

    pub(crate) fn mark_authority_lost(&self) {
        match self.state.lock() {
            Ok(mut state) => {
                *state = state
                    .apply(ShowSpoutPresentationEvent::AuthorityLost)
                    .unwrap_or(ShowSpoutPresentationState::Fault)
            }
            Err(poisoned) => {
                let mut state = poisoned.into_inner();
                *state = state
                    .apply(ShowSpoutPresentationEvent::AuthorityLost)
                    .unwrap_or(ShowSpoutPresentationState::Fault)
            }
        }
        // A worker can be parked in the bounded pair-publication wait when
        // authority is revoked. Wake it immediately so pair retirement does
        // not wait for the full publication deadline merely to observe Fault.
        self.first_black_changed.notify_all();
    }

    pub(crate) fn black_frame(&self) -> &ShowSpoutBlackFrame {
        &self.black
    }

    /// Both workers are released only after both creation leases have been
    /// published. The SDK exposes each sender on its individual first frame,
    /// so this is not an atomic name-registration primitive; it is the gate
    /// that ensures both workers can perform their required first opaque-black
    /// sends before the route can be considered established.
    pub(crate) fn note_sender_published(&self) -> Result<(), String> {
        let previous = self.published_senders.fetch_add(1, Ordering::AcqRel);
        if previous >= 2 {
            self.mark_authority_lost();
            return Err(
                "strict show Spout publication exceeded the fixed two-sender pair".to_string(),
            );
        }
        self.first_black_changed.notify_all();
        Ok(())
    }

    pub(crate) fn pair_publication_complete(&self) -> bool {
        self.published_senders.load(Ordering::Acquire) == 2
    }

    fn authority_is_faulted(&self) -> Result<bool, String> {
        Ok(matches!(
            *self
                .state
                .lock()
                .map_err(|_| "strict show Spout presentation state lock was poisoned while observing authority".to_string())?,
            ShowSpoutPresentationState::Fault
        ))
    }

    /// Wait for the second worker publication without ever taking a project,
    /// output-transition, or Spout-transport lock. The caller owns only its
    /// physical worker and therefore cannot deadlock the R4 publication path.
    pub(crate) fn wait_for_pair_publication(
        &self,
        stop: &std::sync::atomic::AtomicBool,
    ) -> Result<(), String> {
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut state = self.first_black.lock().map_err(|_| {
            "strict show Spout first-black state lock was poisoned before publication".to_string()
        })?;
        while !self.pair_publication_complete() {
            if stop.load(Ordering::Acquire) {
                return Err("strict show Spout worker stopped before pair publication".to_string());
            }
            if self.authority_is_faulted()? {
                return Err(
                    "strict show Spout authority was revoked before pair publication".to_string(),
                );
            }
            if state.failure.is_some() {
                return Err(
                    "strict show Spout pair faulted before first-black publication".to_string(),
                );
            }
            let now = Instant::now();
            if now >= deadline {
                return Err(
                    "strict show Spout pair publication timed out before first-black send"
                        .to_string(),
                );
            }
            let (next, _) = self
                .first_black_changed
                .wait_timeout(state, deadline.saturating_duration_since(now))
                .map_err(|_| {
                    "strict show Spout first-black state lock was poisoned while waiting for publication"
                        .to_string()
                })?;
            state = next;
        }
        Ok(())
    }

    /// Records exactly one forced opaque-black frame outcome per worker. A
    /// sender is not established merely by SDK construction or engine
    /// publication; both outcomes must be successful before the R4 succeeds.
    pub(crate) fn note_initial_black_result(
        &self,
        result: Result<(), String>,
    ) -> Result<(), String> {
        let mut state = self.first_black.lock().map_err(|_| {
            "strict show Spout first-black state lock was poisoned while recording send".to_string()
        })?;
        if state.completed_senders >= 2 {
            self.mark_authority_lost();
            return Err(
                "strict show Spout received more than two initial black acknowledgements"
                    .to_string(),
            );
        }
        state.completed_senders += 1;
        if let Err(error) = &result {
            state.failure = Some(error.clone());
            self.mark_authority_lost();
        }
        self.first_black_changed.notify_all();
        result
    }

    /// Bounded R4 barrier. This runs after both workers were released but
    /// before `ShowSpoutTransportState::active` is set, so a partial first
    /// SendImage can never return a successful activation receipt.
    pub(crate) fn wait_for_initial_black_pair(&self, timeout: Duration) -> Result<(), String> {
        let deadline = Instant::now() + timeout;
        let mut state = self.first_black.lock().map_err(|_| {
            "strict show Spout first-black state lock was poisoned while awaiting pair".to_string()
        })?;
        loop {
            if let Some(error) = state.failure.as_ref() {
                return Err(format!(
                    "strict show Spout initial opaque-black send failed: {error}"
                ));
            }
            if self.authority_is_faulted()? {
                return Err(
                    "strict show Spout authority was revoked before the initial opaque-black pair completed"
                        .to_string(),
                );
            }
            if state.completed_senders == 2 {
                // The per-worker checks prove that each physical first frame
                // was opaque black and retained its exact fixed name. Recheck
                // the R4 fence once at the pair boundary as well: otherwise a
                // revocation between the second worker's send and this return
                // could incorrectly make the pair active/live.
                drop(state);
                self.revalidate("initial opaque-black pair confirmation")?;
                if self.authority_is_faulted()? {
                    return Err(
                        "strict show Spout authority was revoked while confirming the initial opaque-black pair"
                            .to_string(),
                    );
                }
                return Ok(());
            }
            let now = Instant::now();
            if now >= deadline {
                self.mark_authority_lost();
                return Err("strict show Spout initial opaque-black send timed out".to_string());
            }
            let (next, _) = self
                .first_black_changed
                .wait_timeout(state, deadline.saturating_duration_since(now))
                .map_err(|_| {
                    "strict show Spout first-black state lock was poisoned while awaiting pair"
                        .to_string()
                })?;
            state = next;
        }
    }

    /// Called only after a show worker has synchronously dropped its sender.
    /// When the second physical drop completes, this backend-owned callback
    /// retires the exact engine pair once, without waiting for a future UI
    /// action or invoking engine mutation from the authority validator.
    pub(crate) fn note_physical_sender_drop(&self) {
        let should_retire = match self.teardown.lock() {
            Ok(mut state) => {
                state.physically_dropped_senders =
                    state.physically_dropped_senders.saturating_add(1);
                if state.automatic_retirement_armed
                    && state.physically_dropped_senders == 2
                    && !state.automatic_engine_retirement_started
                {
                    state.automatic_engine_retirement_started = true;
                    true
                } else {
                    false
                }
            }
            Err(poisoned) => {
                let mut state = poisoned.into_inner();
                state.physically_dropped_senders =
                    state.physically_dropped_senders.saturating_add(1);
                if state.automatic_retirement_armed
                    && state.physically_dropped_senders == 2
                    && !state.automatic_engine_retirement_started
                {
                    state.automatic_engine_retirement_started = true;
                    true
                } else {
                    false
                }
            }
        };
        if !should_retire {
            return;
        }
        let result = self.engine.retire_show_spout_outputs_published(
            self.expected.background.clone(),
            self.expected.foreground.clone(),
            Instant::now() + Duration::from_secs(2),
        );
        match self.teardown.lock() {
            Ok(mut state) => state.automatic_engine_retirement = Some(result),
            Err(poisoned) => poisoned.into_inner().automatic_engine_retirement = Some(result),
        }
    }

    /// Hand terminal pair cleanup to worker exits only after both forced
    /// black sends succeeded. A sender that ended before this point leaves
    /// the pending R4 transaction as the sole engine-compensation owner.
    fn arm_automatic_engine_retirement(&self) -> Result<(), String> {
        let mut state = self.teardown.lock().map_err(|_| {
            "strict show Spout teardown state lock was poisoned before active-pair handoff"
                .to_string()
        })?;
        if state.automatic_retirement_armed {
            return Err(
                "strict show Spout automatic retirement was armed more than once".to_string(),
            );
        }
        if state.physically_dropped_senders != 0 {
            return Err(
                "strict show Spout sender ended before active-pair handoff; pending rollback remains the sole engine owner"
                    .to_string(),
            );
        }
        state.automatic_retirement_armed = true;
        Ok(())
    }

    fn automatic_engine_retirement(&self) -> Option<Result<(), String>> {
        self.teardown
            .lock()
            .map(|state| state.automatic_engine_retirement.clone())
            .unwrap_or_else(|poisoned| poisoned.into_inner().automatic_engine_retirement.clone())
    }
}

pub(crate) struct ShowSpoutTransportState {
    active: Option<ShowSpoutOutputPair>,
    /// Set before an active pair is detached and retained until the detached
    /// pair reports its physical and exact engine cleanup result. No observer
    /// may infer that `active: None` makes the fixed sender names reusable.
    reaping: Option<ShowSpoutReaping>,
    /// Exact old-engine cleanup is in flight outside this mutex. Like
    /// `reaping`, this remains visible to every observer until its matching
    /// engine ACK receipt is finalized under the mutex.
    blocked_retirement_retry: Option<ShowSpoutBlockedRetirementRetry>,
    next_blocked_retirement_retry_id: u64,
    /// A worker may have completed physical teardown while the one exact
    /// engine-retire ACK failed.  Keep that named pair process-locally
    /// blocked: a fresh R4 must repair this exact old publication before it
    /// can allocate new ids or construct either fixed sender name.
    blocked_engine_retirement: Option<ShowSpoutBlockedEngineRetirement>,
}

#[derive(Clone)]
struct ShowSpoutBlockedEngineRetirement {
    expected: ShowSpoutOutputs,
    prior_error: String,
}

/// A one-shot, exact identity receipt for retrying an engine-pair retirement
/// outside `ShowSpoutTransportState`'s mutex. The nonce prevents a delayed or
/// substituted completion from clearing a newer reservation for the same
/// fixed sender names.
#[derive(Clone)]
pub(crate) struct ShowSpoutBlockedRetirementRetry {
    expected: ShowSpoutOutputs,
    prior_error: String,
    reservation_id: u64,
}

impl ShowSpoutBlockedRetirementRetry {
    pub(crate) fn expected(&self) -> &ShowSpoutOutputs {
        &self.expected
    }
}

impl Default for ShowSpoutTransportState {
    fn default() -> Self {
        Self {
            active: None,
            reaping: None,
            blocked_retirement_retry: None,
            next_blocked_retirement_retry_id: 1,
            blocked_engine_retirement: None,
        }
    }
}

impl ShowSpoutTransportState {
    fn reject_reaping(&self, phase: &str) -> Result<(), String> {
        if let Some(reaping) = self.reaping.as_ref() {
            return Err(format!(
                "strict show Spout {phase} is blocked while the exact sender pair is reaping after {} ({})",
                reaping.kind.label(),
                reaping.reason,
            ));
        }
        if let Some(retry) = self.blocked_retirement_retry.as_ref() {
            return Err(format!(
                "strict show Spout {phase} is blocked while exact blocked-retirement retry reservation {} is awaiting its engine acknowledgement ({})",
                retry.reservation_id,
                retry.prior_error,
            ));
        }
        Ok(())
    }

    fn require_matching_reaping(
        &self,
        reaping: &ShowSpoutReaping,
        phase: &str,
    ) -> Result<(), String> {
        let Some(current) = self.reaping.as_ref() else {
            return Err(format!(
                "strict show Spout {phase} did not find the required state-owned reaping receipt"
            ));
        };
        if current.expected != reaping.expected
            || current.kind != reaping.kind
            || current.reason != reaping.reason
        {
            return Err(format!(
                "strict show Spout {phase} found a different state-owned reaping receipt"
            ));
        }
        Ok(())
    }

    fn require_matching_physical_retirement(
        &self,
        retired: &ShowSpoutPhysicalRetirement,
        phase: &str,
    ) -> Result<(), String> {
        if retired.expected != retired.reaping.expected {
            return Err(format!(
                "strict show Spout {phase} physical cleanup receipt did not retain its exact engine pair"
            ));
        }
        self.require_matching_reaping(&retired.reaping, phase)
    }

    fn finish_reaping(&mut self, reaping: &ShowSpoutReaping, phase: &str) -> Result<(), String> {
        self.require_matching_reaping(reaping, phase)?;
        self.reaping = None;
        Ok(())
    }

    /// The common R4 admission gate. It is intentionally separate from SDK
    /// construction so deterministic tests can prove a second request never
    /// reaches sender preparation while state-owned reaping remains present.
    pub(crate) fn ensure_preparation_admitted(&self) -> Result<(), String> {
        self.reject_reaping("sender preparation")
    }

    /// Phase one: construct both SDK senders behind the current authority
    /// fence. No content is published by either worker before phase two.
    pub(crate) fn prepare_show_spout_outputs<F>(
        &mut self,
        transport: &mut SpoutTransportState,
        engine: &EngineHandle,
        expected: ShowSpoutOutputs,
        activation: OutputOwnershipActivation,
        authority_validator: F,
    ) -> Result<ShowSpoutPrepareOutcome, String>
    where
        F: FnMut() -> Result<(), String> + Send + 'static,
    {
        self.ensure_preparation_admitted()?;
        if let Some(retry) = self.reserve_blocked_engine_retirement_retry(engine)? {
            return Ok(ShowSpoutPrepareOutcome::RetryBlockedRetirement(retry));
        }
        transport
            .harvest_failed_workers(engine)
            .map_err(|error| error.message)?;
        let expected_outputs = [expected.background.clone(), expected.foreground.clone()];
        validate_show_spout_outputs(&expected_outputs)
            .map_err(|error| format!("invalid strict show Spout pair: {error}"))?;
        if let Some(current) = self.active.as_ref() {
            if current.matches(&expected) {
                current.revalidate("show Spout no-op confirmation")?;
                return Ok(ShowSpoutPrepareOutcome::NoOp);
            }
            return Err(
                "a different strict show Spout sender pair is already established; retire it before reauthorization"
                    .to_string(),
            );
        }

        let control = Arc::new(ShowSpoutWorkerControl::new(
            engine.clone(),
            expected.clone(),
            authority_validator,
        ));
        let background = transport.start_show_output_worker(
            &expected.background,
            engine.clone(),
            activation.clone(),
            Arc::clone(&control),
        )?;
        let foreground = match transport.start_show_output_worker(
            &expected.foreground,
            engine.clone(),
            activation,
            Arc::clone(&control),
        ) {
            Ok(worker) => worker,
            Err(error) => {
                control.mark_authority_lost();
                let cleanup = background.stop().map_err(|error| error.message);
                return Err(rollback_show_spout_startup_error(
                    "foreground sender construction failed",
                    error,
                    cleanup,
                ));
            }
        };
        let pair = ShowSpoutOutputPair {
            expected,
            control,
            background,
            foreground,
        };
        if let Err(error) = pair.prepare_keepalive_black() {
            let cleanup = pair.retire();
            return Err(rollback_show_spout_startup_error(
                "sender pre-publication state preparation failed",
                error,
                cleanup,
            ));
        }
        Ok(ShowSpoutPrepareOutcome::Pending(
            ShowSpoutPendingOutputPair { pair: Some(pair) },
        ))
    }

    /// Reserve the one exact old-engine retirement retry before releasing the
    /// transport mutex. The caller must perform the ACK outside this mutex and
    /// then submit the returned receipt to `finish_blocked_engine_retirement_retry`.
    /// A reservation never permits this R4 to construct fixed sender names.
    fn reserve_blocked_engine_retirement_retry(
        &mut self,
        engine: &EngineHandle,
    ) -> Result<Option<ShowSpoutBlockedRetirementRetry>, String> {
        self.reject_reaping("blocked-retirement retry")?;
        let Some(blocked) = self.blocked_engine_retirement.as_ref().cloned() else {
            return Ok(None);
        };
        if let Err(error) =
            decide_show_spout_ensure(&engine.snapshot().video.outputs, &blocked.expected)
        {
            return Err(format!(
                "strict show Spout retry is blocked because its old engine pair was substituted or conflicted ({error}); explicit operator recovery is required (prior automatic retirement error: {})",
                blocked.prior_error
            ));
        }
        // `CreatePair` is deliberately not an acknowledgement. A lost old
        // engine reply may leave an empty snapshot, but only the matching
        // exact retry receipt below may clear this state-owned barrier. The
        // caller will still submit `RetireShowSpoutOutputsPublished`, whose
        // acknowledgement either proves the exact outcome or keeps the
        // barrier installed.
        let reservation_id = self.next_blocked_retirement_retry_id;
        self.next_blocked_retirement_retry_id = reservation_id.checked_add(1).ok_or_else(|| {
            "strict show Spout blocked-retirement retry receipt space is exhausted; explicit operator recovery is required"
                .to_string()
        })?;
        let retry = ShowSpoutBlockedRetirementRetry {
            expected: blocked.expected,
            prior_error: blocked.prior_error,
            reservation_id,
        };
        self.blocked_retirement_retry = Some(retry.clone());
        Ok(Some(retry))
    }

    /// Finalize an exact retry receipt after the caller has awaited the engine
    /// ACK without holding either Spout transport mutex. The reservation and
    /// unresolved-retirement barrier are cleared atomically only after the
    /// same receipt reports success; a delayed or substituted completion must
    /// leave both barriers intact.
    pub(crate) fn finish_blocked_engine_retirement_retry(
        &mut self,
        retry: ShowSpoutBlockedRetirementRetry,
        engine_retirement: Result<(), String>,
    ) -> Result<(), String> {
        let Some(current) = self.blocked_retirement_retry.as_ref() else {
            return Err(
                "strict show Spout blocked-retirement retry finish found no state-owned reservation"
                    .to_string(),
            );
        };
        if current.expected != retry.expected
            || current.prior_error != retry.prior_error
            || current.reservation_id != retry.reservation_id
        {
            return Err(
                "strict show Spout blocked-retirement retry finish found a different state-owned reservation"
                    .to_string(),
            );
        }
        let Some(blocked) = self.blocked_engine_retirement.as_ref() else {
            return Err(
                "strict show Spout blocked-retirement retry finish lost its unresolved exact engine-pair barrier"
                    .to_string(),
            );
        };
        if blocked.expected != retry.expected || blocked.prior_error != retry.prior_error {
            return Err(
                "strict show Spout blocked-retirement retry finish found a different unresolved exact engine-pair barrier"
                    .to_string(),
            );
        }
        match engine_retirement {
            Ok(()) => {
                self.blocked_engine_retirement = None;
                self.blocked_retirement_retry = None;
                // This R4 was admitted solely to reconcile a possibly-ambiguous
                // rollback. Do not let it construct a new physical pair after
                // that repair: the operator must issue a second fresh R4.
                Err(format!(
                    "strict show Spout prior engine retirement was repaired; submit a fresh R4 before constructing a new pair (prior automatic retirement error: {})",
                    retry.prior_error
                ))
            }
            Err(error) => {
                self.blocked_retirement_retry = None;
                Err(format!(
                    "strict show Spout retry remains blocked; exact old engine-pair retirement failed again: {error}"
                ))
            }
        }
    }

    /// A pending pair can fail after the engine published it but before the
    /// pair became active (for example, first-black acknowledgement failure).
    /// Its explicit rollback has the same fixed-name and fresh-R4 barrier as
    /// an active worker's automatic rollback; never return an engine retire
    /// error as a bare string and forget the remaining pair identity.
    pub(crate) fn record_unresolved_engine_retirement(
        &mut self,
        expected: ShowSpoutOutputs,
        error: String,
    ) -> Result<(), String> {
        if self.active.is_some() {
            return Err(
                "strict show Spout cannot record a pending rollback failure while a pair is active"
                    .to_string(),
            );
        }
        if let Some(existing) = self.blocked_engine_retirement.as_ref() {
            if existing.expected != expected {
                return Err(
                    "strict show Spout rollback found a different unresolved exact engine pair"
                        .to_string(),
                );
            }
            return Ok(());
        }
        self.blocked_engine_retirement = Some(ShowSpoutBlockedEngineRetirement {
            expected,
            prior_error: error,
        });
        Ok(())
    }

    /// Phase two: verify that the engine atomically published the exact fixed
    /// pair, then publish both sender resources and await both forced opaque
    /// black-frame acknowledgements. Failed verification/publication/first
    /// frame drops both resources before returning.
    pub(crate) fn publish_show_spout_outputs(
        &mut self,
        engine: &EngineHandle,
        mut pending: ShowSpoutPendingOutputPair,
    ) -> Result<ShowSpoutLiveTransferCommitToken, String> {
        self.reject_reaping("pending-pair publication")?;
        if self.active.is_some() {
            return Err(
                "strict show Spout pair is already established; pending pair was retired"
                    .to_string(),
            );
        }
        let pair = pending
            .pair
            .take()
            .ok_or_else(|| "strict show Spout pending pair was already consumed".to_string())?;
        let decision = decide_show_spout_ensure(&engine.snapshot().video.outputs, &pair.expected)
            .map_err(|error| {
                format!("strict show Spout engine publication was not admitted: {error}")
            });
        if !matches!(decision, Ok(ShowSpoutEnsureDecision::NoOp)) {
            let cleanup = pair.retire();
            return Err(rollback_show_spout_startup_error(
                "engine pair publication verification failed",
                decision.err().unwrap_or_else(|| {
                    "strict show Spout engine publication did not produce the exact pair"
                        .to_string()
                }),
                cleanup,
            ));
        }
        let mut pair = pair;
        if let Err(error) = pair.publish() {
            let cleanup = pair.retire();
            return Err(rollback_show_spout_startup_error(
                "sender publication failed",
                error,
                cleanup,
            ));
        }
        if let Err(error) = pair.control.arm_automatic_engine_retirement() {
            let cleanup = pair.retire();
            return Err(rollback_show_spout_startup_error(
                "active-pair handoff failed",
                error,
                cleanup,
            ));
        }
        self.active = Some(pair);
        Ok(ShowSpoutLiveTransferCommitToken {
            control: Arc::clone(
                &self
                    .active
                    .as_ref()
                    .expect("strict show Spout pair was just made active")
                    .control,
            ),
        })
    }

    /// Timeline stop/pause changes content presentation only. Both sender
    /// registrations stay open and the worker borrows its cached black frame
    /// on every tick until this is switched back to live.
    pub(crate) fn sync_content_state(&self, timeline_playing: bool) -> Result<(), String> {
        self.reject_reaping("content status")?;
        self.active
            .as_ref()
            .ok_or_else(|| "strict show Spout pair is not established".to_string())?
            .control
            .sync_timeline_playing(timeline_playing)
    }

    /// Atomically removes the active receipt and returns the physical pair to
    /// its caller. The caller must stop/join it before reacquiring this state
    /// mutex to record the engine-retirement result; holding the mutex across
    /// that acknowledgement can otherwise block a worker's fault/cleanup
    /// reporting path.
    pub(crate) fn take_active_for_authority_change(
        &mut self,
    ) -> Result<Option<ShowSpoutRetiringOutputPair>, ShowSpoutTransportFailure> {
        if let Err(message) = self.reject_reaping("authority transition") {
            return Err(ShowSpoutTransportFailure {
                message,
                automatic_engine_retirement: None,
            });
        }
        let Some(pair) = self.active.take() else {
            if let Some(blocked) = self.blocked_engine_retirement.as_ref() {
                return Err(ShowSpoutTransportFailure {
                    message: format!(
                        "strict show Spout authority transition is blocked until the prior exact engine-pair retirement is reconciled by a fresh R4: {}",
                        blocked.prior_error
                    ),
                    automatic_engine_retirement: Some(Err(blocked.prior_error.clone())),
                });
            }
            return Ok(None);
        };
        pair.control.mark_authority_lost();
        let reaping = ShowSpoutReaping {
            expected: pair.expected.clone(),
            kind: ShowSpoutReapingKind::AuthorityChange,
            reason: "authority transition requested physical sender retirement".to_string(),
        };
        self.reaping = Some(reaping.clone());
        Ok(Some(ShowSpoutRetiringOutputPair::from_pair(
            pair, reaping, None,
        )))
    }

    /// Completes an authority-transition retirement after its physical pair
    /// has been stopped and joined outside the state mutex.
    pub(crate) fn finish_active_for_authority_change(
        &mut self,
        retired: ShowSpoutPhysicalRetirement,
    ) -> Result<(), ShowSpoutTransportFailure> {
        let reaping = retired.reaping.clone();
        if let Err(error) =
            self.require_matching_physical_retirement(&retired, "authority-transition finish")
        {
            return Err(ShowSpoutTransportFailure {
                message: error,
                automatic_engine_retirement: retired.automatic_engine_retirement,
            });
        }
        let cleanup_failed = retired.cleanup.is_err();
        let message = match retired.cleanup {
            Ok(()) => {
                "strict show Spout active pair was physically retired for authority transition"
                    .to_string()
            }
            Err(error) => format!(
                "strict show Spout active pair authority-transition retirement reported physical teardown failure: {error}"
            ),
        };
        match retired.automatic_engine_retirement.as_ref() {
            Some(Ok(())) => {
                if let Err(error) = self.finish_reaping(&reaping, "authority-transition finish") {
                    return Err(ShowSpoutTransportFailure {
                        message: error,
                        automatic_engine_retirement: retired.automatic_engine_retirement,
                    });
                }
                if cleanup_failed {
                    Err(ShowSpoutTransportFailure {
                        message,
                        automatic_engine_retirement: retired.automatic_engine_retirement,
                    })
                } else {
                    Ok(())
                }
            }
            Some(Err(error)) => {
                self.blocked_engine_retirement = Some(ShowSpoutBlockedEngineRetirement {
                    expected: retired.expected,
                    prior_error: error.clone(),
                });
                // The state mutex remains held until this unresolved exact
                // engine-pair barrier is installed. Only then may a fresh R4
                // observe reaping as absent.
                if let Err(error) = self.finish_reaping(&reaping, "authority-transition finish") {
                    return Err(ShowSpoutTransportFailure {
                        message: error,
                        automatic_engine_retirement: retired.automatic_engine_retirement,
                    });
                }
                Err(ShowSpoutTransportFailure {
                    message,
                    automatic_engine_retirement: retired.automatic_engine_retirement,
                })
            }
            None => {
                self.blocked_engine_retirement = Some(ShowSpoutBlockedEngineRetirement {
                    expected: retired.expected,
                    prior_error: "authority transition worker teardown did not report automatic engine retirement"
                        .to_string(),
                });
                if let Err(error) = self.finish_reaping(&reaping, "authority-transition finish") {
                    return Err(ShowSpoutTransportFailure {
                        message: error,
                        automatic_engine_retirement: retired.automatic_engine_retirement,
                    });
                }
                Err(ShowSpoutTransportFailure {
                    message,
                    automatic_engine_retirement: retired.automatic_engine_retirement,
                })
            }
        }
    }

    /// Removes a failed pair before its physical retirement. A missing pair
    /// is not a failure; an active receipt is never retained while cleanup is
    /// still in progress.
    pub(crate) fn take_failed_active_pair(
        &mut self,
    ) -> Result<Option<ShowSpoutRetiringOutputPair>, ShowSpoutTransportFailure> {
        if let Err(message) = self.reject_reaping("failed-worker harvest") {
            return Err(ShowSpoutTransportFailure {
                message,
                automatic_engine_retirement: None,
            });
        }
        let failed = self
            .active
            .as_ref()
            .is_some_and(ShowSpoutOutputPair::has_failed_worker);
        if !failed {
            return Ok(None);
        }
        let pair = self
            .active
            .take()
            .expect("active show Spout pair was checked");
        pair.control.mark_authority_lost();
        let worker_failure = pair.failure_message().unwrap_or_else(|| {
            "strict show Spout worker stopped before a failure reason was recorded".to_string()
        });
        let reaping = ShowSpoutReaping {
            expected: pair.expected.clone(),
            kind: ShowSpoutReapingKind::FailedWorker,
            reason: worker_failure.clone(),
        };
        self.reaping = Some(reaping.clone());
        Ok(Some(ShowSpoutRetiringOutputPair::from_pair(
            pair,
            reaping,
            Some(worker_failure),
        )))
    }

    /// Records a failed-pair cleanup after physical sender retirement. This
    /// stays intentionally fail-closed: if the automatic exact engine retire
    /// did not report success, the next R4 is blocked by this original pair.
    pub(crate) fn finish_failed_active_pair(
        &mut self,
        retired: ShowSpoutPhysicalRetirement,
    ) -> ShowSpoutTransportFailure {
        let reaping = retired.reaping.clone();
        if let Err(error) =
            self.require_matching_physical_retirement(&retired, "failed-worker finish")
        {
            return ShowSpoutTransportFailure {
                message: error,
                automatic_engine_retirement: retired.automatic_engine_retirement,
            };
        }
        let worker_failure = retired.worker_failure.unwrap_or_else(|| {
            "strict show Spout worker stopped before a failure reason was recorded".to_string()
        });
        let automatic_engine_retirement = retired.automatic_engine_retirement;
        let record_result = if let Some(Err(error)) = automatic_engine_retirement.as_ref() {
            self.record_unresolved_engine_retirement(retired.expected.clone(), error.clone())
        } else if automatic_engine_retirement.is_none() {
            self.record_unresolved_engine_retirement(
                retired.expected.clone(),
                "worker teardown completed without the required automatic engine-retirement result"
                    .to_string(),
            )
        } else {
            Ok(())
        };
        if let Err(record_error) = record_result {
            return ShowSpoutTransportFailure {
                message: format!(
                    "strict show Spout authority lost and its unresolved exact engine cleanup could not be recorded: {record_error}"
                ),
                automatic_engine_retirement,
            };
        }
        if let Err(error) = self.finish_reaping(&reaping, "failed-worker finish") {
            return ShowSpoutTransportFailure {
                message: error,
                automatic_engine_retirement,
            };
        }
        ShowSpoutTransportFailure {
            message: rollback_show_spout_startup_error(
                "strict show Spout authority lost",
                worker_failure,
                retired.cleanup,
            ),
            automatic_engine_retirement,
        }
    }
}

pub(crate) struct ShowSpoutPendingOutputPair {
    pair: Option<ShowSpoutOutputPair>,
}

impl ShowSpoutPendingOutputPair {
    /// Explicit rollback used by the R4 owner when engine publication fails.
    /// The error is returned to the caller; `Drop` remains only a final guard
    /// for an unwound transaction.
    pub(crate) fn retire(mut self) -> Result<(), String> {
        let pair = self
            .pair
            .take()
            .ok_or_else(|| "strict show Spout pending pair was already consumed".to_string())?;
        pair.control.mark_authority_lost();
        pair.retire()
    }
}

impl Drop for ShowSpoutPendingOutputPair {
    fn drop(&mut self) {
        let Some(pair) = self.pair.take() else {
            return;
        };
        pair.control.mark_authority_lost();
        if let Err(error) = pair.retire() {
            // `Drop` cannot surface this to the R4 caller, but silently
            // discarding a synchronous stop/join failure would make a live
            // fixed-name collision impossible to diagnose. The sender pair
            // remains faulted and its reservation/ownership fences stay in
            // effect; preserve the physical cleanup evidence in the native
            // log for the recovery path.
            eprintln!("strict show Spout pending sender-pair Drop rollback failed: {error}");
        }
    }
}

struct ShowSpoutOutputPair {
    expected: ShowSpoutOutputs,
    control: Arc<ShowSpoutWorkerControl>,
    background: SpoutRouteWorker,
    foreground: SpoutRouteWorker,
}

impl ShowSpoutOutputPair {
    fn matches(&self, expected: &ShowSpoutOutputs) -> bool {
        self.expected == *expected
    }

    fn revalidate(&self, phase: &str) -> Result<(), String> {
        self.control.revalidate(phase)
    }

    fn prepare_keepalive_black(&self) -> Result<(), String> {
        self.control.prepare_keepalive_black()
    }

    fn publish(&mut self) -> Result<(), String> {
        self.background.publish_with_show(Some(&self.control))?;
        self.foreground.publish_with_show(Some(&self.control))?;
        self.control
            .wait_for_initial_black_pair(Duration::from_secs(5))
    }

    fn has_failed_worker(&self) -> bool {
        self.background.has_finished_or_failed() || self.foreground.has_finished_or_failed()
    }

    fn failure_message(&self) -> Option<String> {
        self.background
            .failure_snapshot()
            .or_else(|| self.foreground.failure_snapshot())
            .map(|error| error.message)
    }

    fn retire(self) -> Result<(), String> {
        // `SpoutRouteWorker::stop` consumes and joins its worker before it
        // can return an error. Invoke both stops unconditionally, so this
        // `Err` is post-join teardown evidence, never permission to leave a
        // live fixed sender behind or silently reuse either reserved name.
        let foreground = self.foreground.stop().map_err(|error| error.message);
        let background = self.background.stop().map_err(|error| error.message);
        match (foreground, background) {
            (Ok(()), Ok(())) => Ok(()),
            (Err(foreground), Ok(())) => Err(format!("foreground sender retirement failed: {foreground}")),
            (Ok(()), Err(background)) => Err(format!("background sender retirement failed: {background}")),
            (Err(foreground), Err(background)) => Err(format!(
                "foreground sender retirement failed: {foreground}; background sender retirement failed: {background}"
            )),
        }
    }
}

fn rollback_show_spout_startup_error<E, C>(phase: &str, error: E, cleanup: Result<(), C>) -> String
where
    E: Into<String>,
    C: Into<String>,
{
    let error = error.into();
    match cleanup {
        Ok(()) => format!(
            "{phase}: {error}; all partially-created strict show Spout senders were retired"
        ),
        Err(cleanup) => format!(
            "{phase}: {error}; strict show Spout rollback also failed: {}",
            cleanup.into()
        ),
    }
}

/// Deterministic physical black-send seam. The `Arc`-free frame ownership
/// stays in the control object; this helper always borrows its one cached
/// RGBA buffer and performs authority validation immediately before send.
#[cfg(test)]
pub(crate) fn send_show_spout_keepalive_black<S: SpoutOutputSender>(
    sender: &mut S,
    control: &ShowSpoutWorkerControl,
) -> Result<(), String> {
    control.revalidate("Spout keepalive black frame send")?;
    let black = control.black_frame();
    sender.send_image(black.as_rgba(), black.width(), black.height())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

    fn control_for_tests<F>(authority_validator: F) -> ShowSpoutWorkerControl
    where
        F: FnMut() -> Result<(), String> + Send + 'static,
    {
        ShowSpoutWorkerControl::new(
            EngineHandle::start_for_tests(protocol::DmxOutputConfig {
                enabled: false,
                ..protocol::DmxOutputConfig::default()
            }),
            crate::show_spout_outputs::build_show_spout_outputs(11, 12, 1).unwrap(),
            authority_validator,
        )
    }

    #[derive(Default)]
    struct FakeSender {
        name: String,
        frames: Vec<(usize, u32, u32, [u8; 4])>,
    }

    impl SpoutOutputSender for FakeSender {
        fn sender_name(&self) -> String {
            if self.name.is_empty() {
                "Syndocal Background".to_string()
            } else {
                self.name.clone()
            }
        }

        fn send_image(&mut self, pixels: &[u8], width: u32, height: u32) -> Result<(), String> {
            self.frames.push((
                pixels.len(),
                width,
                height,
                pixels[0..4].try_into().unwrap(),
            ));
            Ok(())
        }
    }

    struct FirstBlackNameMutatingSender {
        name: String,
        frames: Vec<(usize, u32, u32, [u8; 4])>,
    }

    impl SpoutOutputSender for FirstBlackNameMutatingSender {
        fn sender_name(&self) -> String {
            self.name.clone()
        }

        fn send_image(&mut self, pixels: &[u8], width: u32, height: u32) -> Result<(), String> {
            self.frames.push((
                pixels.len(),
                width,
                height,
                pixels[0..4].try_into().unwrap(),
            ));
            // Models Spout's lazy registration collision: construction and
            // the pre-send name can be exact, then the first physical frame
            // is registered under a suffixed name.
            self.name = "Syndocal Background_1".to_string();
            Ok(())
        }
    }

    #[test]
    fn keepalive_black_borrows_the_fixed_opaque_frame_and_revalidates_at_send() {
        let calls = Arc::new(AtomicUsize::new(0));
        let calls_for_validator = Arc::clone(&calls);
        let control = control_for_tests(move || {
            calls_for_validator.fetch_add(1, Ordering::AcqRel);
            Ok(())
        });
        control.prepare_keepalive_black().unwrap();
        let mut sender = FakeSender::default();
        send_show_spout_keepalive_black(&mut sender, &control).unwrap();
        send_show_spout_keepalive_black(&mut sender, &control).unwrap();
        assert_eq!(calls.load(Ordering::Acquire), 2);
        assert_eq!(sender.frames.len(), 2);
        assert!(sender.frames.iter().all(|(bytes, width, height, pixel)| {
            *bytes
                == crate::show_spout_outputs::SHOW_SPOUT_FRAME_PIXEL_LEN
                    * crate::show_spout_outputs::SHOW_SPOUT_BLACK_PIXEL_RGBA.len()
                && (*width, *height) == (1920, 1080)
                && *pixel == [0, 0, 0, 255]
        }));
    }

    #[test]
    fn initial_black_is_opaque_before_lazy_registration_name_is_rechecked() {
        let control = control_for_tests(|| Ok(()));
        control.prepare_keepalive_black().unwrap();
        let mut sender = FirstBlackNameMutatingSender {
            name: "Syndocal Background".to_string(),
            frames: Vec::new(),
        };

        let error = crate::spout_transport::send_strict_show_spout_black(
            &mut sender,
            "Syndocal Background",
            &control,
            "test initial opaque-black send",
        )
        .expect_err("a post-first-frame name suffix must fail the strict pair");

        assert!(error.contains("Syndocal Background_1"));
        assert_eq!(sender.frames.len(), 1);
        assert_eq!(
            sender.frames[0],
            (
                crate::show_spout_outputs::SHOW_SPOUT_FRAME_PIXEL_LEN
                    * crate::show_spout_outputs::SHOW_SPOUT_BLACK_PIXEL_RGBA.len(),
                1920,
                1080,
                [0, 0, 0, 255],
            )
        );
    }

    #[test]
    fn both_initial_opaque_black_acknowledgements_are_required_before_establishment() {
        let control = control_for_tests(|| Ok(()));
        control.prepare_keepalive_black().unwrap();
        assert!(!control.pair_publication_complete());
        control.note_sender_published().unwrap();
        assert!(!control.pair_publication_complete());
        control.note_sender_published().unwrap();
        assert!(control.pair_publication_complete());
        control.note_initial_black_result(Ok(())).unwrap();
        control.note_initial_black_result(Ok(())).unwrap();
        control
            .wait_for_initial_black_pair(Duration::from_millis(1))
            .unwrap();
        assert!(control.note_sender_published().is_err());
        assert!(control.presentation_is_keepalive_black().is_err());
    }

    #[test]
    fn one_initial_opaque_black_failure_rejects_the_pair_before_activation() {
        let control = control_for_tests(|| Ok(()));
        control.prepare_keepalive_black().unwrap();
        control.note_sender_published().unwrap();
        control.note_sender_published().unwrap();
        let error = control
            .note_initial_black_result(Err("foreground injected first-black failure".to_string()))
            .unwrap_err();
        assert!(error.contains("injected first-black failure"));
        assert!(control
            .wait_for_initial_black_pair(Duration::from_millis(1))
            .unwrap_err()
            .contains("foreground injected first-black failure"));
        assert!(control.presentation_is_keepalive_black().is_err());
    }

    #[test]
    fn initial_black_timeout_faults_before_any_live_receipt() {
        let control = control_for_tests(|| Ok(()));
        control.prepare_keepalive_black().unwrap();
        control.note_sender_published().unwrap();
        control.note_sender_published().unwrap();
        // Only one sender completed its required opaque black frame. The
        // bounded pair barrier must fail closed rather than treating a
        // playing timeline or one successful sender as an active receipt.
        control.note_initial_black_result(Ok(())).unwrap();

        let error = control
            .wait_for_initial_black_pair(Duration::ZERO)
            .expect_err("a one-sided first-black timeout must not activate the pair");
        assert!(error.contains("timed out"));
        assert!(control.presentation_is_keepalive_black().is_err());
        assert!(control.automatic_engine_retirement().is_none());
    }

    #[test]
    fn initial_black_pair_barrier_revalidates_authority_before_active_handoff() {
        let control = control_for_tests(|| Err("R4 fence changed after first black".to_string()));
        control.prepare_keepalive_black().unwrap();
        control.note_sender_published().unwrap();
        control.note_sender_published().unwrap();
        control.note_initial_black_result(Ok(())).unwrap();
        control.note_initial_black_result(Ok(())).unwrap();

        let error = control
            .wait_for_initial_black_pair(Duration::from_millis(1))
            .expect_err("the pair must remain inactive when its post-black R4 confirmation fails");
        assert!(error.contains("R4 fence changed after first black"));
        assert!(control.presentation_is_keepalive_black().is_err());
    }

    #[test]
    fn authority_loss_wakes_pair_publication_wait_without_waiting_for_timeout() {
        let control = control_for_tests(|| Ok(()));
        control.prepare_keepalive_black().unwrap();
        control.mark_authority_lost();
        let stop = std::sync::atomic::AtomicBool::new(false);
        let started = Instant::now();
        let error = control.wait_for_pair_publication(&stop).unwrap_err();
        assert!(error.contains("authority was revoked"));
        assert!(started.elapsed() < Duration::from_millis(50));
    }

    #[test]
    fn pending_first_black_failure_never_auto_retires_the_engine_pair() {
        let control = control_for_tests(|| Ok(()));
        control.prepare_keepalive_black().unwrap();
        control.note_sender_published().unwrap();
        control.note_sender_published().unwrap();
        assert!(control
            .note_initial_black_result(Err("background first-black fault".to_string()))
            .is_err());

        // The pending transaction owns its explicit rollback. Physical drops
        // before active handoff must not race a second engine-retire command.
        control.note_physical_sender_drop();
        control.note_physical_sender_drop();
        assert!(control.automatic_engine_retirement().is_none());
        assert!(control.arm_automatic_engine_retirement().is_err());
    }

    #[test]
    fn active_pair_second_physical_drop_retires_the_exact_engine_pair_once() {
        let engine = EngineHandle::start_for_tests(protocol::DmxOutputConfig {
            enabled: false,
            ..protocol::DmxOutputConfig::default()
        });
        let expected = crate::show_spout_outputs::build_show_spout_outputs(11, 12, 1).unwrap();
        let safety = engine.safety_blackout_authority();
        engine
            .enable_show_spout_outputs_published(
                expected.background.clone(),
                expected.foreground.clone(),
                safety.epoch,
                safety.generation,
                Instant::now() + Duration::from_secs(1),
            )
            .expect("test engine must publish the exact active show pair");
        assert_eq!(engine.snapshot().video.outputs.len(), 2);

        let control = ShowSpoutWorkerControl::new(engine.clone(), expected, || Ok(()));
        control.arm_automatic_engine_retirement().unwrap();
        control.note_physical_sender_drop();
        assert_eq!(engine.snapshot().video.outputs.len(), 2);
        assert!(control.automatic_engine_retirement().is_none());

        control.note_physical_sender_drop();
        assert_eq!(engine.snapshot().video.outputs.len(), 0);
        assert!(matches!(
            control.automatic_engine_retirement(),
            Some(Ok(()))
        ));

        // A duplicate callback cannot submit a second retirement mutation.
        control.note_physical_sender_drop();
        assert_eq!(engine.snapshot().video.outputs.len(), 0);
        assert!(matches!(
            control.automatic_engine_retirement(),
            Some(Ok(()))
        ));
    }

    #[test]
    fn failed_automatic_retirement_blocks_fresh_r4_until_that_exact_old_pair_is_repaired() {
        let engine = EngineHandle::start_for_tests(protocol::DmxOutputConfig {
            enabled: false,
            ..protocol::DmxOutputConfig::default()
        });
        let expected = crate::show_spout_outputs::build_show_spout_outputs(11, 12, 1).unwrap();
        let mut transport = ShowSpoutTransportState::default();
        transport
            .record_unresolved_engine_retirement(
                expected.clone(),
                "injected automatic engine retirement ACK failure".to_string(),
            )
            .unwrap();

        let error = match transport.take_active_for_authority_change() {
            Err(error) => error,
            Ok(_) => panic!("role/project transitions may not bypass unresolved engine cleanup"),
        };
        assert!(error.message.contains("blocked until"));

        // A lost retirement reply can leave an empty snapshot, but that
        // observation is not a receipt. The fresh R4 must reserve and submit
        // the exact old retirement; the engine refuses because the named pair
        // is missing, and the barrier remains installed rather than clearing
        // from `CreatePair` alone.
        let missing_retry = transport
            .reserve_blocked_engine_retirement_retry(&engine)
            .unwrap()
            .expect("an empty snapshot must still reserve the exact old retirement");
        let missing_ack = engine.retire_show_spout_outputs_published(
            missing_retry.expected().background.clone(),
            missing_retry.expected().foreground.clone(),
            Instant::now() + Duration::from_secs(1),
        );
        assert!(missing_ack.is_err());
        let missing = transport
            .finish_blocked_engine_retirement_retry(missing_retry, missing_ack)
            .expect_err("an ACK-less empty snapshot must not reopen sender preparation");
        assert!(missing.contains("retry remains blocked"));
        assert!(transport.blocked_engine_retirement.is_some());
        assert!(transport.blocked_retirement_retry.is_none());

        // Model a second failed automatic retirement, this time with the old
        // engine pair still present. Only this exact pair may be retried.
        let mut transport = ShowSpoutTransportState::default();
        transport
            .record_unresolved_engine_retirement(
                expected.clone(),
                "second injected automatic retirement ACK failure".to_string(),
            )
            .unwrap();

        let safety = engine.safety_blackout_authority();
        engine
            .enable_show_spout_outputs_published(
                expected.background.clone(),
                expected.foreground.clone(),
                safety.epoch,
                safety.generation,
                Instant::now() + Duration::from_secs(1),
            )
            .unwrap();
        let retry = transport
            .reserve_blocked_engine_retirement_retry(&engine)
            .unwrap()
            .expect("the exact old engine pair must reserve a serialized retry");
        let engine_retirement = engine.retire_show_spout_outputs_published(
            retry.expected().background.clone(),
            retry.expected().foreground.clone(),
            Instant::now() + Duration::from_secs(1),
        );
        let repaired = transport
            .finish_blocked_engine_retirement_retry(retry, engine_retirement)
            .expect_err("the repair request must not also construct a new pair");
        assert!(repaired.contains("submit a fresh R4"));
        assert!(transport.blocked_engine_retirement.is_none());
        assert!(engine.snapshot().video.outputs.is_empty());
        assert!(transport.active.is_none());
    }

    #[test]
    fn blocked_retirement_retry_reservation_keeps_second_r4_outside_sender_construction_until_matching_ack_finish(
    ) {
        let engine = EngineHandle::start_for_tests(protocol::DmxOutputConfig {
            enabled: false,
            ..protocol::DmxOutputConfig::default()
        });
        let expected = crate::show_spout_outputs::build_show_spout_outputs(21, 22, 1).unwrap();
        let safety = engine.safety_blackout_authority();
        engine
            .enable_show_spout_outputs_published(
                expected.background.clone(),
                expected.foreground.clone(),
                safety.epoch,
                safety.generation,
                Instant::now() + Duration::from_secs(1),
            )
            .unwrap();

        // This is the state left by a real active-pair detach: both workers
        // physically dropped first, but their exact engine retirement ACK did
        // not return. The first fresh R4 may only reserve that old pair.
        let state = Arc::new(Mutex::new(ShowSpoutTransportState::default()));
        state
            .lock()
            .unwrap()
            .record_unresolved_engine_retirement(
                expected.clone(),
                "injected post-detach engine retirement ACK loss".to_string(),
            )
            .unwrap();
        let mut first_generic = transport_for_blocked_retry();
        let first_activation = engine
            .admit_output_activation(protocol::MachineOutputRole::Both)
            .unwrap();
        let retry = match state
            .lock()
            .unwrap()
            .prepare_show_spout_outputs(
                &mut first_generic,
                &engine,
                expected.clone(),
                first_activation,
                || Ok(()),
            )
            .expect("fresh R4 must reserve the old exact retirement before construction")
        {
            ShowSpoutPrepareOutcome::RetryBlockedRetirement(retry) => retry,
            _ => panic!("fresh R4 must not construct while an exact retirement is unresolved"),
        };
        assert_eq!(first_generic.strict_show_startup_resource_count(), 0);

        // The real engine ACK occurs in this detached continuation. Hold it
        // at the external ACK boundary: the show mutex is deliberately not
        // held while it waits, but its state-owned reservation remains live.
        let (ack_boundary_entered, ack_boundary_observed) = std::sync::mpsc::sync_channel(0);
        let (release_ack, await_release_ack) = std::sync::mpsc::sync_channel(0);
        let retry_for_ack = retry.clone();
        let engine_for_ack = engine.clone();
        let ack_thread = std::thread::spawn(move || {
            ack_boundary_entered
                .send(())
                .expect("test must observe the blocked external ACK boundary");
            await_release_ack
                .recv()
                .expect("test must release the exact engine retirement ACK");
            engine_for_ack.retire_show_spout_outputs_published(
                retry_for_ack.expected().background.clone(),
                retry_for_ack.expected().foreground.clone(),
                Instant::now() + Duration::from_secs(1),
            )
        });
        ack_boundary_observed
            .recv()
            .expect("the external exact engine ACK must now be blocked");

        // A second R4 interleaving while that ACK is outstanding acquires the
        // show mutex and fails at the reservation. It therefore proves the
        // first continuation cannot be holding the mutex while waiting, and
        // neither fixed Sender::new path was reached.
        let second_state = Arc::clone(&state);
        let second_engine = engine.clone();
        let second_expected = expected.clone();
        let second = std::thread::spawn(move || {
            let mut generic = transport_for_blocked_retry();
            let activation = second_engine
                .admit_output_activation(protocol::MachineOutputRole::Both)
                .unwrap();
            let error = match second_state.lock().unwrap().prepare_show_spout_outputs(
                &mut generic,
                &second_engine,
                second_expected,
                activation,
                || Ok(()),
            ) {
                Err(error) => error,
                Ok(_) => panic!("second R4 must fail closed while retry ACK is outstanding"),
            };
            assert!(error.contains("retry reservation"));
            assert_eq!(
                generic.strict_show_startup_resource_count(),
                0,
                "retry reservation must reject before either fixed Sender::new"
            );
        });
        second.join().unwrap();

        let mut show = state.lock().unwrap();
        assert!(show
            .sync_content_state(true)
            .expect_err("status must remain closed while the retry ACK is outstanding")
            .contains("retry reservation"));
        let authority_error = match show.take_active_for_authority_change() {
            Err(error) => error,
            Ok(_) => panic!("authority change must not bypass a retry reservation"),
        };
        assert!(authority_error.message.contains("retry reservation"));
        let harvest_error = match show.take_failed_active_pair() {
            Err(error) => error,
            Ok(_) => panic!("harvest must not bypass a retry reservation"),
        };
        assert!(harvest_error.message.contains("retry reservation"));

        // A delayed or substituted receipt cannot clear a reservation even if
        // it lies about success. Each component of the receipt identity is
        // checked independently and the original exact barrier stays live.
        let mut wrong_expected = retry.clone();
        wrong_expected.expected =
            crate::show_spout_outputs::build_show_spout_outputs(23, 24, 1).unwrap();
        let wrong = show
            .finish_blocked_engine_retirement_retry(wrong_expected, Ok(()))
            .expect_err("a wrong expected pair must not reopen fixed sender names");
        assert!(wrong.contains("different state-owned reservation"));
        assert!(show.blocked_retirement_retry.is_some());
        assert!(show.blocked_engine_retirement.is_some());

        let mut wrong_prior_error = retry.clone();
        wrong_prior_error.prior_error = "substituted prior retirement error".to_string();
        let wrong = show
            .finish_blocked_engine_retirement_retry(wrong_prior_error, Ok(()))
            .expect_err("a wrong prior error must not reopen fixed sender names");
        assert!(wrong.contains("different state-owned reservation"));
        assert!(show.blocked_retirement_retry.is_some());
        assert!(show.blocked_engine_retirement.is_some());

        let mut wrong_retry = retry.clone();
        wrong_retry.reservation_id += 1;
        let wrong = show
            .finish_blocked_engine_retirement_retry(wrong_retry, Ok(()))
            .expect_err("a wrong retry receipt must not reopen fixed sender names");
        assert!(wrong.contains("different state-owned reservation"));
        assert!(show.blocked_retirement_retry.is_some());
        assert!(show.blocked_engine_retirement.is_some());
        drop(show);

        release_ack
            .send(())
            .expect("the exact engine ACK continuation must still be waiting");
        let engine_retirement = ack_thread
            .join()
            .expect("the exact engine ACK continuation must not panic");
        let repaired = state
            .lock()
            .unwrap()
            .finish_blocked_engine_retirement_retry(retry, engine_retirement)
            .expect_err("repair completion must still require a fresh R4");
        assert!(repaired.contains("submit a fresh R4"));
        let show = state.lock().unwrap();
        assert!(show.blocked_retirement_retry.is_none());
        assert!(show.blocked_engine_retirement.is_none());
        show.ensure_preparation_admitted()
            .expect("only the matching exact engine ACK may reopen sender preparation");
        assert!(engine.snapshot().video.outputs.is_empty());
    }

    #[test]
    fn reaping_interlock_blocks_second_r4_and_status_until_matching_finish() {
        let engine = EngineHandle::start_for_tests(protocol::DmxOutputConfig {
            enabled: false,
            ..protocol::DmxOutputConfig::default()
        });
        let expected = crate::show_spout_outputs::build_show_spout_outputs(31, 32, 1).unwrap();
        let reaping = ShowSpoutReaping {
            expected: expected.clone(),
            kind: ShowSpoutReapingKind::AuthorityChange,
            reason: "test detached pair is still joining and awaiting exact engine retirement"
                .to_string(),
        };
        let state = Arc::new(Mutex::new(ShowSpoutTransportState {
            active: None,
            reaping: Some(reaping.clone()),
            blocked_retirement_retry: None,
            next_blocked_retirement_retry_id: 1,
            blocked_engine_retirement: None,
        }));

        // Model a second R4 interleaving after detach but before the original
        // pair's stop/join and engine retirement acknowledgement complete.
        // It must stop at the state-owned receipt, before either fixed sender
        // can be constructed or prepared.
        let second_state = Arc::clone(&state);
        let second_engine = engine.clone();
        let second_expected = expected.clone();
        let second = std::thread::spawn(move || {
            let mut generic = transport_for_blocked_retry();
            let activation = second_engine
                .admit_output_activation(protocol::MachineOutputRole::Both)
                .unwrap();
            let error = match second_state.lock().unwrap().prepare_show_spout_outputs(
                &mut generic,
                &second_engine,
                second_expected,
                activation,
                || Ok(()),
            ) {
                Err(error) => error,
                Ok(_) => {
                    panic!("second R4 must not prepare a sender while the old pair is reaping")
                }
            };
            assert!(error.contains("reaping"));
            assert_eq!(
                generic.strict_show_startup_resource_count(),
                0,
                "reaping must reject before either fixed sender reaches construction"
            );
        });
        second.join().unwrap();

        let mut show = state.lock().unwrap();
        assert!(show
            .sync_content_state(true)
            .expect_err("status must expose reaping rather than a missing active receipt")
            .contains("reaping"));
        let authority_error = match show.take_active_for_authority_change() {
            Err(error) => error,
            Ok(_) => panic!("authority change must not bypass state-owned reaping"),
        };
        assert!(authority_error.message.contains("reaping"));
        let harvest_error = match show.take_failed_active_pair() {
            Err(error) => error,
            Ok(_) => panic!("a second harvest must not bypass state-owned reaping"),
        };
        assert!(harvest_error.message.contains("reaping"));
        let retry_error = match show.reserve_blocked_engine_retirement_retry(&engine) {
            Err(error) => error,
            Ok(_) => panic!("blocked-retirement retry must not bypass reaping"),
        };
        assert!(retry_error.contains("reaping"));

        let substituted = crate::show_spout_outputs::build_show_spout_outputs(41, 42, 1).unwrap();
        let mismatch = show
            .finish_active_for_authority_change(ShowSpoutPhysicalRetirement {
                expected: substituted,
                reaping: reaping.clone(),
                worker_failure: None,
                cleanup: Ok(()),
                automatic_engine_retirement: Some(Ok(())),
            })
            .expect_err("a finish receipt for a different exact engine pair must not reopen R4");
        assert!(mismatch.message.contains("exact engine pair"));
        assert!(show.reaping.is_some());

        // This opaque outcome can only be produced by the detached pair's
        // `retire`: it represents both physical sender joins and the exact
        // engine-retirement acknowledgement. Only this matching finish clears
        // the receipt; subsequent R4 preparation may then pass its state gate.
        show.finish_active_for_authority_change(ShowSpoutPhysicalRetirement {
            expected: expected.clone(),
            reaping: reaping.clone(),
            worker_failure: None,
            cleanup: Ok(()),
            automatic_engine_retirement: Some(Ok(())),
        })
        .unwrap();
        assert!(show.reaping.is_none());
        show.ensure_preparation_admitted()
            .expect("only a matching physical+engine completion may reopen R4 preparation");
    }

    #[test]
    fn joined_cleanup_error_is_visible_without_retaining_an_active_sender_receipt() {
        let expected = crate::show_spout_outputs::build_show_spout_outputs(51, 52, 1).unwrap();
        let reaping = ShowSpoutReaping {
            expected: expected.clone(),
            kind: ShowSpoutReapingKind::AuthorityChange,
            reason: "test joined sender teardown".to_string(),
        };
        let mut show = ShowSpoutTransportState {
            active: None,
            reaping: Some(reaping.clone()),
            blocked_retirement_retry: None,
            next_blocked_retirement_retry_id: 1,
            blocked_engine_retirement: None,
        };

        // `ShowSpoutOutputPair::retire` invokes both consuming worker stops;
        // each stop joins before returning this error. The result must remain
        // visible to the authority command, but must not claim a live active
        // receipt after physical joins and exact engine retirement succeeded.
        let failure = show
            .finish_active_for_authority_change(ShowSpoutPhysicalRetirement {
                expected,
                reaping,
                worker_failure: None,
                cleanup: Err("foreground sender reported a post-join fault".to_string()),
                automatic_engine_retirement: Some(Ok(())),
            })
            .expect_err("post-join cleanup error must be returned to the caller");
        assert!(failure.message.contains("physical teardown failure"));
        assert!(show.active.is_none());
        assert!(show.reaping.is_none());
        assert!(show.blocked_engine_retirement.is_none());
        show.ensure_preparation_admitted().expect(
            "the documented join-before-error invariant is the only basis for releasing fixed names after this visible error",
        );
    }

    fn transport_for_blocked_retry() -> SpoutTransportState {
        SpoutTransportState::new(
            Arc::new(Mutex::new(HashMap::new())),
            #[cfg(feature = "ndi")]
            Arc::new(Mutex::new(HashMap::new())),
            Arc::new(Mutex::new(HashMap::new())),
        )
    }

    #[test]
    fn enable_ack_loss_with_empty_snapshot_keeps_exact_barrier_without_constructor() {
        let engine = EngineHandle::start_for_tests(protocol::DmxOutputConfig {
            enabled: false,
            ..protocol::DmxOutputConfig::default()
        });
        let expected = crate::show_spout_outputs::build_show_spout_outputs(71, 72, 1).unwrap();
        let mut show = ShowSpoutTransportState::default();
        let mut generic = transport_for_blocked_retry();

        // Model an enable reply loss followed by an explicit serial retire
        // whose reply was also lost/no-apply. The initial R4 must retain the
        // unresolved exact identity rather than allocate either fixed name.
        show.record_unresolved_engine_retirement(
            expected.clone(),
            "injected enable ACK loss; serial exact retirement was unverifiable".to_string(),
        )
        .unwrap();
        let activation = engine
            .admit_output_activation(protocol::MachineOutputRole::Both)
            .unwrap();
        let retry = match show
            .prepare_show_spout_outputs(&mut generic, &engine, expected, activation, || Ok(()))
            .expect("reconciliation R4 must reserve the exact old pair before SDK construction")
        {
            ShowSpoutPrepareOutcome::RetryBlockedRetirement(retry) => retry,
            _ => panic!("reconciliation R4 must not reach SDK construction"),
        };
        let engine_retirement = engine.retire_show_spout_outputs_published(
            retry.expected().background.clone(),
            retry.expected().foreground.clone(),
            Instant::now() + Duration::from_secs(1),
        );
        assert!(engine_retirement.is_err());
        let error = show
            .finish_blocked_engine_retirement_retry(retry, engine_retirement)
            .expect_err("an empty snapshot without an exact ACK must not clear the barrier");
        assert!(error.contains("retry remains blocked"));
        assert!(show.blocked_engine_retirement.is_some());
        assert_eq!(generic.strict_show_startup_resource_count(), 0);

        // No sender was constructed by the reconciliation request, and the
        // exact barrier remains until a matching successful engine receipt.
        assert!(show.active.is_none());
    }

    #[test]
    fn first_black_rollback_retire_failure_repairs_old_pair_without_constructor() {
        let engine = EngineHandle::start_for_tests(protocol::DmxOutputConfig {
            enabled: false,
            ..protocol::DmxOutputConfig::default()
        });
        let expected = crate::show_spout_outputs::build_show_spout_outputs(81, 82, 1).unwrap();
        let safety = engine.safety_blackout_authority();
        engine
            .enable_show_spout_outputs_published(
                expected.background.clone(),
                expected.foreground.clone(),
                safety.epoch,
                safety.generation,
                Instant::now() + Duration::from_secs(1),
            )
            .unwrap();

        let mut show = ShowSpoutTransportState::default();
        let mut generic = transport_for_blocked_retry();
        // The SDK pair already physically retired after a first-black
        // failure, but the explicit engine rollback failed to acknowledge.
        show.record_unresolved_engine_retirement(
            expected.clone(),
            "injected first-black rollback retirement ACK failure".to_string(),
        )
        .unwrap();
        let activation = engine
            .admit_output_activation(protocol::MachineOutputRole::Both)
            .unwrap();
        let retry = match show
            .prepare_show_spout_outputs(&mut generic, &engine, expected, activation, || Ok(()))
            .expect("repair R4 must reserve old-pair cleanup before SDK construction")
        {
            ShowSpoutPrepareOutcome::RetryBlockedRetirement(retry) => retry,
            _ => panic!("repair R4 must not construct a replacement pair"),
        };
        let engine_retirement = engine.retire_show_spout_outputs_published(
            retry.expected().background.clone(),
            retry.expected().foreground.clone(),
            Instant::now() + Duration::from_secs(1),
        );
        let error = show
            .finish_blocked_engine_retirement_retry(retry, engine_retirement)
            .expect_err("repair R4 must retain its fresh-R4 barrier");
        assert!(error.contains("prior engine retirement was repaired"));
        assert!(show.blocked_engine_retirement.is_none());
        assert!(engine.snapshot().video.outputs.is_empty());
        assert_eq!(generic.strict_show_startup_resource_count(), 0);
    }

    #[test]
    fn worker_timeline_sync_transitions_live_and_black_without_ui_help() {
        let control = control_for_tests(|| Ok(()));
        control.prepare_keepalive_black().unwrap();
        control.note_sender_published().unwrap();
        control.note_sender_published().unwrap();
        control.note_initial_black_result(Ok(())).unwrap();
        control.note_initial_black_result(Ok(())).unwrap();
        control
            .wait_for_initial_black_pair(Duration::from_millis(1))
            .unwrap();
        assert!(control.presentation_is_keepalive_black().unwrap());
        control.sync_timeline_playing(true).unwrap();
        // First-black success is still not a live-content receipt. Until the
        // R4 lease commit, an already-playing Timeline remains black.
        assert!(control.presentation_is_keepalive_black().unwrap());
        control.commit_live_transfer_for_test();
        control.sync_timeline_playing(true).unwrap();
        assert!(!control.presentation_is_keepalive_black().unwrap());
        control.sync_timeline_playing(false).unwrap();
        assert!(control.presentation_is_keepalive_black().unwrap());
    }

    #[test]
    fn failed_send_revalidation_faults_before_the_sender_is_called() {
        let control = control_for_tests(|| Err("stale R4 fence".to_string()));
        control.prepare_keepalive_black().unwrap();
        let mut sender = FakeSender::default();
        let error = send_show_spout_keepalive_black(&mut sender, &control).unwrap_err();
        assert!(error.contains("stale R4 fence"));
        assert!(sender.frames.is_empty());
        assert!(control.presentation_is_keepalive_black().is_err());
    }
}
