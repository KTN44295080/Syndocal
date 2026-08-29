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
            if state.completed_senders == 2 {
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

impl Default for ShowSpoutTransportState {
    fn default() -> Self {
        Self {
            active: None,
            blocked_engine_retirement: None,
        }
    }
}

impl ShowSpoutTransportState {
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
        self.retry_blocked_engine_retirement(engine)?;
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

    /// The physical workers already dropped before a blocked entry is made.
    /// Only a new authenticated R4 reaches this retry, and it may retire only
    /// the original exact pair. Missing, substituted, or conflicted engine
    /// state is not normalized into a fresh pair.
    fn retry_blocked_engine_retirement(&mut self, engine: &EngineHandle) -> Result<(), String> {
        let Some(blocked) = self.blocked_engine_retirement.as_ref().cloned() else {
            return Ok(());
        };
        match decide_show_spout_ensure(&engine.snapshot().video.outputs, &blocked.expected) {
            Ok(ShowSpoutEnsureDecision::NoOp) => {}
            Ok(ShowSpoutEnsureDecision::CreatePair) => {
                // The exact retire ACK may have been lost after the engine
                // already applied it.  Zero Spout outputs is the only safe
                // terminal reconciliation: clear this stale block but force
                // the operator to submit a *new* R4 before construction.
                self.blocked_engine_retirement = None;
                return Err(format!(
                    "strict show Spout prior engine retirement is already reconciled (no Spout senders remain); submit a fresh R4 before constructing a new pair (prior automatic retirement error: {})",
                    blocked.prior_error
                ));
            }
            Err(error) => {
                return Err(format!(
                    "strict show Spout retry is blocked because its old engine pair was substituted or conflicted ({error}); explicit operator recovery is required (prior automatic retirement error: {})",
                    blocked.prior_error
                ));
            }
        }
        engine
            .retire_show_spout_outputs_published(
                blocked.expected.background.clone(),
                blocked.expected.foreground.clone(),
                Instant::now() + Duration::from_secs(2),
            )
            .map_err(|error| {
                format!(
                    "strict show Spout retry remains blocked; exact old engine-pair retirement failed again: {error}"
                )
            })?;
        self.blocked_engine_retirement = None;
        // This R4 was admitted solely to reconcile a possibly-ambiguous
        // rollback.  Do not let it construct a new physical pair after that
        // repair: the operator must issue a second, fresh R4 once both the
        // old engine publication and its fixed names are known gone.
        Err(format!(
            "strict show Spout prior engine retirement was repaired; submit a fresh R4 before constructing a new pair (prior automatic retirement error: {})",
            blocked.prior_error
        ))
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
        self.active
            .as_ref()
            .ok_or_else(|| "strict show Spout pair is not established".to_string())?
            .control
            .sync_timeline_playing(timeline_playing)
    }

    /// Output-role transitions and project replacement retire the strict pair
    /// through this sole owner. `SpoutRouteWorker::stop` joins after dropping
    /// the SDK sender; only its second physical-drop callback may retire the
    /// exact engine pair. The transition waits for that result instead of
    /// leaving the old pair live across a new authority generation.
    pub(crate) fn retire_active_for_authority_change(
        &mut self,
    ) -> Result<(), ShowSpoutTransportFailure> {
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
            return Ok(());
        };
        let expected = pair.expected.clone();
        pair.control.mark_authority_lost();
        let control = Arc::clone(&pair.control);
        let cleanup = pair.retire();
        let cleanup_failed = cleanup.is_err();
        let automatic_engine_retirement = control.automatic_engine_retirement();
        let message = match cleanup {
            Ok(()) => {
                "strict show Spout active pair was physically retired for authority transition"
                    .to_string()
            }
            Err(error) => format!(
                "strict show Spout active pair authority-transition retirement reported physical teardown failure: {error}"
            ),
        };
        match automatic_engine_retirement.as_ref() {
            Some(Ok(())) => {
                if cleanup_failed {
                    Err(ShowSpoutTransportFailure {
                        message,
                        automatic_engine_retirement,
                    })
                } else {
                    Ok(())
                }
            }
            Some(Err(error)) => {
                self.blocked_engine_retirement = Some(ShowSpoutBlockedEngineRetirement {
                    expected,
                    prior_error: error.clone(),
                });
                Err(ShowSpoutTransportFailure {
                    message,
                    automatic_engine_retirement,
                })
            }
            None => {
                self.blocked_engine_retirement = Some(ShowSpoutBlockedEngineRetirement {
                    expected,
                    prior_error: "authority transition worker teardown did not report automatic engine retirement"
                        .to_string(),
                });
                Err(ShowSpoutTransportFailure {
                    message,
                    automatic_engine_retirement,
                })
            }
        }
    }

    pub(crate) fn harvest_show_spout_output_failure(
        &mut self,
    ) -> Result<(), ShowSpoutTransportFailure> {
        let failed = self
            .active
            .as_ref()
            .is_some_and(ShowSpoutOutputPair::has_failed_worker);
        if !failed {
            return Ok(());
        }
        let pair = self
            .active
            .take()
            .expect("active show Spout pair was checked");
        let expected = pair.expected.clone();
        pair.control.mark_authority_lost();
        let message = pair.failure_message().unwrap_or_else(|| {
            "strict show Spout worker stopped before a failure reason was recorded".to_string()
        });
        let control = Arc::clone(&pair.control);
        let cleanup = pair.retire();
        let automatic_engine_retirement = control.automatic_engine_retirement();
        let record_result = if let Some(Err(error)) = automatic_engine_retirement.as_ref() {
            self.record_unresolved_engine_retirement(expected.clone(), error.clone())
        } else if automatic_engine_retirement.is_none() {
            self.record_unresolved_engine_retirement(
                expected.clone(),
                "worker teardown completed without the required automatic engine-retirement result"
                    .to_string(),
            )
        } else {
            Ok(())
        };
        if let Err(record_error) = record_result {
            return Err(ShowSpoutTransportFailure {
                message: format!(
                    "strict show Spout authority lost and its unresolved exact engine cleanup could not be recorded: {record_error}"
                ),
                automatic_engine_retirement,
            });
        }
        Err(ShowSpoutTransportFailure {
            message: rollback_show_spout_startup_error(
                "strict show Spout authority lost",
                message,
                cleanup,
            ),
            automatic_engine_retirement,
        })
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
        let _ = pair.retire();
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

        assert!(transport
            .retire_active_for_authority_change()
            .expect_err("role/project transitions may not bypass unresolved engine cleanup")
            .message
            .contains("blocked until"));

        // A lost retirement reply can leave no old pair at all. Reconcile
        // that terminal state, but this R4 still cannot construct a new pair.
        let missing = transport
            .retry_blocked_engine_retirement(&engine)
            .expect_err("the reconciliation request must not also construct");
        assert!(missing.contains("already reconciled"));
        assert!(transport.blocked_engine_retirement.is_none());

        // Model a second failed automatic retirement, this time with the old
        // engine pair still present. Only this exact pair may be retried.
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
        let repaired = transport
            .retry_blocked_engine_retirement(&engine)
            .expect_err("the repair request must not also construct a new pair");
        assert!(repaired.contains("submit a fresh R4"));
        assert!(transport.blocked_engine_retirement.is_none());
        assert!(engine.snapshot().video.outputs.is_empty());
        assert!(transport.active.is_none());
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
    fn enable_ack_loss_with_failed_serial_retire_blocks_then_reconciles_without_constructor() {
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
        let error = show
            .prepare_show_spout_outputs(&mut generic, &engine, expected, activation, || Ok(()))
            .err()
            .expect("reconciliation R4 must not reach SDK construction");
        assert!(error.contains("already reconciled"));
        assert!(show.blocked_engine_retirement.is_none());
        assert_eq!(generic.strict_show_startup_resource_count(), 0);

        // The cleared state is intentionally eligible only on a *next* R4.
        // No sender was constructed by the reconciliation request itself.
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
        let error = show
            .prepare_show_spout_outputs(&mut generic, &engine, expected, activation, || Ok(()))
            .err()
            .expect("repair R4 must not construct a replacement pair");
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
