use super::*;
use std::sync::{
    atomic::{AtomicU64, AtomicUsize, Ordering},
    Arc, Barrier,
};

#[derive(Debug, Clone, Copy)]
enum Mode {
    Ok,
    StopFail,
    FactoryFail,
    ResourcePanic,
    Panic,
    RecoveryFail,
    RecoveryPanic,
}
#[derive(Debug)]
struct FakeFactory {
    counter: Arc<AtomicUsize>,
    mode: Mode,
    has_partial_resource: bool,
}
#[derive(Debug)]
struct FakeResource {
    counter: Arc<AtomicUsize>,
    mode: Mode,
}
impl seal::Factory for FakeFactory {}
impl seal::Resource for FakeResource {}
impl OutputFactory for FakeFactory {
    type Resource = FakeResource;
    fn start(&mut self, _: &FactoryStartCapability) -> Result<Self::Resource, BridgeDiagnostic> {
        self.counter.fetch_add(1, Ordering::SeqCst);
        match self.mode {
            Mode::Ok => Ok(FakeResource {
                counter: self.counter.clone(),
                mode: Mode::Ok,
            }),
            Mode::StopFail => Ok(FakeResource {
                counter: self.counter.clone(),
                mode: Mode::StopFail,
            }),
            Mode::FactoryFail => Err(diag("factory_failed")),
            Mode::ResourcePanic => Ok(FakeResource {
                counter: self.counter.clone(),
                mode: Mode::ResourcePanic,
            }),
            Mode::Panic | Mode::RecoveryFail | Mode::RecoveryPanic => {
                self.has_partial_resource = true;
                panic!("factory panic")
            }
        }
    }
    fn recover_after_panic(
        &mut self,
        _: &FactoryPanicCapability,
    ) -> Result<Self::Resource, BridgeDiagnostic> {
        assert!(
            self.has_partial_resource,
            "recovery was not the panicked factory"
        );
        self.counter.fetch_add(1, Ordering::SeqCst);
        match self.mode {
            Mode::RecoveryFail => Err(diag("panic_recovery_failed")),
            Mode::RecoveryPanic => panic!("panic recovery panic"),
            _ => {
                self.has_partial_resource = false;
                Ok(FakeResource {
                    counter: self.counter.clone(),
                    mode: Mode::Ok,
                })
            }
        }
    }
}
impl OutputResource for FakeResource {
    fn stop_and_join(&mut self) -> Result<(), BridgeDiagnostic> {
        self.counter.fetch_add(1, Ordering::SeqCst);
        match self.mode {
            Mode::Ok => Ok(()),
            Mode::StopFail => Err(diag("stop_failed")),
            Mode::FactoryFail => unreachable!(),
            Mode::ResourcePanic => panic!("resource stop panic"),
            Mode::Panic => panic!("stop panic"),
            Mode::RecoveryFail | Mode::RecoveryPanic => unreachable!(),
        }
    }
}
struct FakeStart {
    result: Result<(), BridgeDiagnostic>,
}
impl seal::AsioStart for FakeStart {}
impl AsioStartLifecycle for FakeStart {
    fn start(&mut self, _: &AsioStartLease) -> Result<(), BridgeDiagnostic> {
        self.result.clone()
    }
}
struct FakeRevalidation {
    result: Result<(), BridgeDiagnostic>,
}

struct PanicStart;
impl seal::AsioStart for PanicStart {}
impl AsioStartLifecycle for PanicStart {
    fn start(&mut self, _: &AsioStartLease) -> Result<(), BridgeDiagnostic> {
        panic!("start panic")
    }
}
struct PanicRevalidation;
impl seal::Revalidation for PanicRevalidation {}
impl AsioRevalidationLifecycle for PanicRevalidation {
    fn revalidate(&mut self, _: &AsioRevalidationLease) -> Result<(), BridgeDiagnostic> {
        panic!("revalidation panic")
    }
}
impl seal::Revalidation for FakeRevalidation {}
impl AsioRevalidationLifecycle for FakeRevalidation {
    fn revalidate(&mut self, _: &AsioRevalidationLease) -> Result<(), BridgeDiagnostic> {
        self.result.clone()
    }
}

struct FakeBridge {
    counter: Arc<AtomicUsize>,
    result: Result<(), BridgeStopFailureDetail>,
}
impl seal::Bridge for FakeBridge {}
impl BridgeStop for FakeBridge {
    fn stop_unpublish_drain_close(
        &mut self,
        _: &BridgeStopLease,
    ) -> Result<(), BridgeStopFailureDetail> {
        self.counter.fetch_add(1, Ordering::SeqCst);
        self.result.clone()
    }
}
struct PanicBridge;
impl seal::Bridge for PanicBridge {}
impl BridgeStop for PanicBridge {
    fn stop_unpublish_drain_close(
        &mut self,
        _: &BridgeStopLease,
    ) -> Result<(), BridgeStopFailureDetail> {
        panic!("bridge panic")
    }
}
fn diag(code: &str) -> BridgeDiagnostic {
    BridgeDiagnostic::checked(code, "exact bridge diagnostic").unwrap()
}
fn factory(counter: &Arc<AtomicUsize>, mode: Mode) -> FakeFactory {
    FakeFactory {
        counter: counter.clone(),
        mode,
        has_partial_resource: false,
    }
}
fn active(router: &mut Router) -> StartTicket {
    let q = router.quiesce().unwrap();
    let s = router.admit_start(q).unwrap();
    let operation = router
        .begin_start_io(s, FakeStart { result: Ok(()) })
        .unwrap();
    let receipt = match operation.perform() {
        AsioStartIo::Started(receipt) => receipt,
        _ => panic!(),
    };
    router.complete_start(receipt).unwrap();
    s
}

#[test]
fn process_owner_is_single_use_and_every_ticket_is_router_bound() {
    let _production = Router::acquire_process_owner().unwrap();
    assert!(matches!(
        Router::acquire_process_owner(),
        Err(Error::ProcessOwnerTaken)
    ));
    let count = Arc::new(AtomicUsize::new(0));
    let mut left = Router::test_router();
    let mut right = Router::test_router();
    let p = left
        .begin_publication(Route::Program, factory(&count, Mode::Ok))
        .unwrap();
    let prepared = match p.perform_io() {
        FactoryIo::Prepared(value) => value,
        _ => panic!(),
    };
    let prepared = match right.commit(prepared) {
        PublicationCommit::Rejected(prepared, Error::ForeignTicket) => prepared,
        _ => panic!("a foreign router must not consume a sealed resource"),
    };
    let owned = match left.commit(prepared) {
        PublicationCommit::Published(owned) => owned,
        _ => panic!(),
    };
    let receipt = match owned.begin_retire(&mut left).unwrap().perform() {
        RetirementIo::Joined(receipt) => receipt,
        _ => panic!(),
    };
    assert!(matches!(
        right.complete_retire(receipt),
        Err(Error::ForeignTicket)
    ));
}

#[test]
fn one_factory_result_is_sealed_and_only_real_stop_mints_retirement_receipt() {
    let count = Arc::new(AtomicUsize::new(0));
    let mut router = Router::test_router();
    let p = router
        .begin_publication(Route::Program, factory(&count, Mode::Ok))
        .unwrap();
    let prepared = match p.perform_io() {
        FactoryIo::Prepared(value) => value,
        _ => panic!(),
    };
    let owned = match router.commit(prepared) {
        PublicationCommit::Published(value) => value,
        _ => panic!(),
    };
    assert_eq!(count.load(Ordering::SeqCst), 1);
    let retire = owned.begin_retire(&mut router).unwrap();
    let receipt = match retire.perform() {
        RetirementIo::Joined(value) => value,
        _ => panic!(),
    };
    assert_eq!(count.load(Ordering::SeqCst), 2);
    router.complete_retire(receipt).unwrap();
}

#[test]
fn late_prepare_completion_is_never_publishable_and_is_immediately_retired() {
    let count = Arc::new(AtomicUsize::new(0));
    let mut router = Router::test_router();
    let p = router
        .begin_publication(Route::Program, factory(&count, Mode::Ok))
        .unwrap();
    let q = router.quiesce().unwrap();
    let prepared = match p.perform_io() {
        FactoryIo::Prepared(value) => value,
        _ => panic!(),
    };
    let retire = match router.commit(prepared) {
        PublicationCommit::MustRetire(value) => value,
        _ => panic!("late resource became publishable"),
    };
    let receipt = match retire.perform() {
        RetirementIo::Joined(value) => value,
        _ => panic!(),
    };
    router.complete_retire(receipt).unwrap();
    assert!(router.admit_start(q).is_ok());
    assert_eq!(count.load(Ordering::SeqCst), 2);
}

#[test]
fn retirement_failure_retains_the_same_resource_for_a_fresh_retry_permit() {
    let count = Arc::new(AtomicUsize::new(0));
    let mut router = Router::test_router();
    let p = router
        .begin_publication(Route::Program, factory(&count, Mode::StopFail))
        .unwrap();
    let prepared = match p.perform_io() {
        FactoryIo::Prepared(value) => value,
        _ => panic!(),
    };
    let owned = match router.commit(prepared) {
        PublicationCommit::Published(value) => value,
        _ => panic!(),
    };
    let (mut resource, failure) = match owned.begin_retire(&mut router).unwrap().perform() {
        RetirementIo::Failed(resource, failure) => (resource, failure),
        _ => panic!(),
    };
    let stale_failure = failure.clone();
    let stale_operation = failure.operation;
    let stale_receipt = Receipt {
        ticket: failure.ticket,
        operation: failure.operation,
    };
    assert_eq!(resource.resource.counter.load(Ordering::SeqCst), 2);
    router.retire_failed(failure).unwrap();
    assert_eq!(router.snapshot().state, State::Fault);
    assert!(matches!(
        router.retire_failed(stale_failure),
        Err(Error::FailureMismatch)
    ));
    assert!(matches!(
        router.complete_retire(stale_receipt),
        Err(Error::ReceiptMismatch)
    ));
    assert!(matches!(
        router.quiesce(),
        Err(Error::Blocked(State::Fault))
    ));

    // The returned opaque owner is the only possible retry subject.  Its
    // second attempt gets a new operation, and only its real join unlocks.
    resource.resource.mode = Mode::Ok;
    let retry = resource.begin_retire(&mut router).unwrap();
    assert_ne!(retry.operation, stale_operation);
    let receipt = match retry.perform() {
        RetirementIo::Joined(receipt) => receipt,
        _ => panic!(),
    };
    router.complete_retire(receipt).unwrap();
    assert_eq!(router.snapshot().state, State::Locked);
}

#[test]
fn publication_panic_uses_a_sealed_recovery_resource_before_unlocking() {
    let count = Arc::new(AtomicUsize::new(0));
    let mut router = Router::test_router();
    let p = router
        .begin_publication(Route::Program, factory(&count, Mode::Panic))
        .unwrap();
    let handle = match p.perform_io() {
        FactoryIo::Panicked(value) => value,
        _ => panic!(),
    };
    router.publication_panicked(&handle).unwrap();
    assert!(matches!(
        router.publication_panicked(&handle),
        Err(Error::PublicationRejected)
    ));
    let mut foreign = Router::test_router();
    let (handle, error) = handle.begin_recovery(&mut foreign).unwrap_err();
    assert!(matches!(error, Error::ForeignTicket));

    // There is no recovery argument here: the opaque capability remains bound
    // to the exact factory instance which actually panicked.
    let recovery = handle.begin_recovery(&mut router).unwrap();
    let retire = match recovery.perform() {
        PanicRecoveryIo::Recovered(value) => value,
        _ => panic!(),
    };
    let receipt = match retire.perform() {
        RetirementIo::Joined(value) => value,
        _ => panic!(),
    };
    router.complete_retire(receipt).unwrap();
    assert_eq!(router.snapshot().state, State::Locked);
    assert_eq!(count.load(Ordering::SeqCst), 3);
}

#[test]
fn panic_recovery_capability_cannot_be_replayed_or_replaced_after_failure() {
    let count = Arc::new(AtomicUsize::new(0));
    let mut router = Router::test_router();
    let publication = router
        .begin_publication(Route::Program, factory(&count, Mode::RecoveryFail))
        .unwrap();
    let handle = match publication.perform_io() {
        FactoryIo::Panicked(handle) => handle,
        _ => panic!(),
    };
    router.publication_panicked(&handle).unwrap();
    let recovery = handle.begin_recovery(&mut router).unwrap();
    let (mut recovery, failure) = match recovery.perform() {
        PanicRecoveryIo::Failed(recovery, failure) => (recovery, failure),
        _ => panic!(),
    };
    assert_eq!(failure.diagnostic.code, "panic_recovery_failed");
    let (returned, error) = recovery.retry(&mut router).unwrap_err();
    recovery = returned;
    assert!(matches!(error, Error::RetirementRejected));
    let stale = failure.clone();
    router.retire_failed(failure).unwrap();
    assert!(matches!(
        router.retire_failed(stale),
        Err(Error::FailureMismatch)
    ));

    // `perform` returns the exact factory/capability pair on failure.  A
    // copied number cannot manufacture another pair; only this pair can obtain
    // a fresh permit once the former failure has been committed.
    recovery.factory.mode = Mode::Panic;
    let retry = recovery.retry(&mut router).unwrap();
    let retire = match retry.perform() {
        PanicRecoveryIo::Recovered(retire) => retire,
        _ => panic!(),
    };
    let receipt = match retire.perform() {
        RetirementIo::Joined(receipt) => receipt,
        _ => panic!(),
    };
    router.complete_retire(receipt).unwrap();

    let mut panicked = Router::test_router();
    let publication = panicked
        .begin_publication(Route::Program, factory(&count, Mode::RecoveryPanic))
        .unwrap();
    let handle = match publication.perform_io() {
        FactoryIo::Panicked(handle) => handle,
        _ => panic!(),
    };
    panicked.publication_panicked(&handle).unwrap();
    let recovery = handle.begin_recovery(&mut panicked).unwrap();
    let (_same_recovery, failure) = match recovery.perform() {
        PanicRecoveryIo::Panicked(recovery, failure) => (recovery, failure),
        _ => panic!(),
    };
    assert_eq!(failure.diagnostic.code, "panic_recovery_panic");
    panicked.retire_failed(failure).unwrap();
    assert_eq!(panicked.snapshot().state, State::Fault);
}

#[test]
fn bridge_receipts_are_session_bound_and_unproven_drains_never_shrink() {
    let mut router = Router::test_router();
    let _start = active(&mut router);
    let count = Arc::new(AtomicUsize::new(0));
    let first = router
        .begin_stop(FakeBridge {
            counter: count.clone(),
            result: Err(BridgeStopFailureDetail::drain(
                NonZeroUsize::new(3).unwrap(),
                diag("drain_timeout"),
            )),
        })
        .unwrap();
    let failure = match first.perform() {
        StopIo::Failed(_, value) => value,
        _ => panic!(),
    };
    router.stop_failed(failure).unwrap();
    assert_eq!(router.snapshot().remaining_drains, 3);
    let retry = router
        .begin_stop(FakeBridge {
            counter: count.clone(),
            result: Err(BridgeStopFailureDetail::dispatch(diag("unpublish_failed"))),
        })
        .unwrap();
    let failure = match retry.perform() {
        StopIo::Failed(_, value) => value,
        _ => panic!(),
    };
    router.stop_failed(failure).unwrap();
    assert_eq!(router.snapshot().remaining_drains, 3);
    let close = router
        .begin_stop(FakeBridge {
            counter: count.clone(),
            result: Err(BridgeStopFailureDetail::close(diag("close_failed"))),
        })
        .unwrap();
    let failure = match close.perform() {
        StopIo::Failed(_, value) => value,
        _ => panic!(),
    };
    router.stop_failed(failure).unwrap();
    assert_eq!(router.snapshot().reason, Reason::CloseFailed);
    let done = router
        .begin_stop(FakeBridge {
            counter: count,
            result: Ok(()),
        })
        .unwrap();
    let receipt = match done.perform() {
        StopIo::Drained(value) => value,
        _ => panic!(),
    };
    router.complete_stop(receipt).unwrap();
    assert_eq!(router.snapshot().state, State::Locked);
}

#[test]
fn revalidation_is_one_shot_and_ready_never_reopens_normal() {
    let mut router = Router::test_router();
    let start = active(&mut router);
    let stop = router
        .begin_stop(FakeBridge {
            counter: Arc::new(AtomicUsize::new(0)),
            result: Ok(()),
        })
        .unwrap();
    let receipt = match stop.perform() {
        StopIo::Drained(value) => value,
        _ => panic!(),
    };
    router.complete_stop(receipt).unwrap();
    let ticket = router.begin_revalidation().unwrap();
    let operation = router
        .begin_revalidation_io(ticket, FakeRevalidation { result: Ok(()) })
        .unwrap();
    let receipt = match operation.perform() {
        AsioRevalidationIo::Validated(receipt) => receipt,
        _ => panic!(),
    };
    let foreign = AsioRevalidationReceipt {
        ticket: receipt.ticket,
    };
    let mut other = Router::test_router();
    assert!(matches!(
        other.complete_revalidation(foreign),
        Err(Error::RevalidationMismatch)
    ));
    let ready = router.complete_revalidation(receipt).unwrap();
    assert!(matches!(
        router.begin_revalidation_io(ticket, FakeRevalidation { result: Ok(()) }),
        Err(Error::RevalidationMismatch)
    ));
    assert!(matches!(
        router.begin_publication(
            Route::Program,
            factory(&Arc::new(AtomicUsize::new(0)), Mode::Ok)
        ),
        Err(Error::Blocked(State::AsioReady))
    ));
    let valid = router.start_validated(ready).unwrap();
    assert_ne!(valid.operation, start.operation);
}

#[test]
fn normal_selection_is_explicit_one_shot_and_ready_cancel_uses_the_same_boundary() {
    let mut router = Router::test_router();
    let _ = active(&mut router);
    let stop = router
        .begin_stop(FakeBridge {
            counter: Arc::new(AtomicUsize::new(0)),
            result: Ok(()),
        })
        .unwrap();
    let receipt = match stop.perform() {
        StopIo::Drained(receipt) => receipt,
        _ => panic!(),
    };
    router.complete_stop(receipt).unwrap();
    router.select_normal().unwrap();
    assert_eq!(router.snapshot().state, State::Normal);
    assert!(matches!(
        router.select_normal(),
        Err(Error::NormalSelectionRejected)
    ));

    let _ = active(&mut router);
    let stop = router
        .begin_stop(FakeBridge {
            counter: Arc::new(AtomicUsize::new(0)),
            result: Ok(()),
        })
        .unwrap();
    let receipt = match stop.perform() {
        StopIo::Drained(receipt) => receipt,
        _ => panic!(),
    };
    router.complete_stop(receipt).unwrap();
    let pending = router.begin_revalidation().unwrap();
    let receipt = match router
        .begin_revalidation_io(pending, FakeRevalidation { result: Ok(()) })
        .unwrap()
        .perform()
    {
        AsioRevalidationIo::Validated(receipt) => receipt,
        _ => panic!(),
    };
    let ready = router.complete_revalidation(receipt).unwrap();
    router.cancel_ready(ready).unwrap();
    assert_eq!(router.snapshot().state, State::Locked);
    assert!(router.snapshot().operation.is_none());
    assert!(router.snapshot().session.is_none());
    router.select_normal().unwrap();
    assert_eq!(router.snapshot().state, State::Normal);

    let _ = active(&mut router);
    let failed = router
        .begin_stop(FakeBridge {
            counter: Arc::new(AtomicUsize::new(0)),
            result: Err(BridgeStopFailureDetail::drain(
                NonZeroUsize::new(1).unwrap(),
                diag("drain_still_open"),
            )),
        })
        .unwrap();
    let failure = match failed.perform() {
        StopIo::Failed(_, failure) => failure,
        _ => panic!(),
    };
    router.stop_failed(failure).unwrap();
    assert!(matches!(
        router.select_normal(),
        Err(Error::NormalSelectionRejected)
    ));
}

#[test]
fn every_identity_counter_fails_closed_before_wrapping() {
    assert!(matches!(
        Router::next_instance(&AtomicU64::new(u64::MAX)),
        Err(Error::IdentityExhausted)
    ));
    let count = Arc::new(AtomicUsize::new(0));

    let mut route = Router::test_router();
    route.next_id = u64::MAX;
    assert!(matches!(
        route.begin_publication(Route::Program, factory(&count, Mode::Ok)),
        Err(Error::IdentityExhausted)
    ));

    let mut operation = Router::test_router();
    operation.next_op = u64::MAX;
    assert!(matches!(
        operation.begin_publication(Route::Program, factory(&count, Mode::Ok)),
        Err(Error::IdentityExhausted)
    ));

    let mut generation = Router::test_router();
    generation.next_gen = u64::MAX;
    assert!(matches!(
        generation.quiesce(),
        Err(Error::IdentityExhausted)
    ));

    let mut session = Router::test_router();
    let quiesce = session.quiesce().unwrap();
    session.next_session = u64::MAX;
    assert!(matches!(
        session.admit_start(quiesce),
        Err(Error::IdentityExhausted)
    ));
}

#[test]
fn failure_and_panic_result_variants_latch_exact_faults() {
    let count = Arc::new(AtomicUsize::new(0));
    let mut failed_publication = Router::test_router();
    let publication = failed_publication
        .begin_publication(Route::Program, factory(&count, Mode::FactoryFail))
        .unwrap();
    let failure = match publication.perform_io() {
        FactoryIo::Failed(failure) => failure,
        _ => panic!(),
    };
    failed_publication.publication_failed(failure).unwrap();
    assert_eq!(
        failed_publication.snapshot().reason,
        Reason::PublicationFailed
    );

    let mut panic_retire = Router::test_router();
    let publication = panic_retire
        .begin_publication(Route::Program, factory(&count, Mode::Panic))
        .unwrap();
    let handle = match publication.perform_io() {
        FactoryIo::Panicked(handle) => handle,
        _ => panic!(),
    };
    panic_retire.publication_panicked(&handle).unwrap();
    let action = handle.begin_recovery(&mut panic_retire).unwrap();
    let retire = match action.perform() {
        PanicRecoveryIo::Recovered(retire) => retire,
        _ => panic!(),
    };
    let receipt = match retire.perform() {
        RetirementIo::Joined(receipt) => receipt,
        _ => panic!(),
    };
    panic_retire.complete_retire(receipt).unwrap();

    let mut start_failure = Router::test_router();
    let q = start_failure.quiesce().unwrap();
    let start = start_failure.admit_start(q).unwrap();
    let operation = start_failure
        .begin_start_io(
            start,
            FakeStart {
                result: Err(diag("driver_missing")),
            },
        )
        .unwrap();
    let failure = match operation.perform() {
        AsioStartIo::Failed(_, failure) => failure,
        _ => panic!(),
    };
    start_failure.start_failed(failure).unwrap();
    assert_eq!(start_failure.snapshot().reason, Reason::AsioStartFailed);
    assert_eq!(start_failure.snapshot().diagnostic.code, "driver_missing");
    assert!(matches!(
        start_failure.begin_publication(Route::Program, factory(&count, Mode::Ok)),
        Err(Error::Blocked(State::Fault))
    ));

    let mut start_panic = Router::test_router();
    let q = start_panic.quiesce().unwrap();
    let start = start_panic.admit_start(q).unwrap();
    let failure = match start_panic
        .begin_start_io(start, PanicStart)
        .unwrap()
        .perform()
    {
        AsioStartIo::Panicked(_, failure) => failure,
        _ => panic!(),
    };
    assert_eq!(failure.diagnostic.code, "asio_start_panic");
    start_panic.start_failed(failure).unwrap();

    let mut revalidation_failure = Router::test_router();
    let _ = active(&mut revalidation_failure);
    let stop = revalidation_failure
        .begin_stop(FakeBridge {
            counter: Arc::new(AtomicUsize::new(0)),
            result: Ok(()),
        })
        .unwrap();
    let receipt = match stop.perform() {
        StopIo::Drained(receipt) => receipt,
        _ => panic!(),
    };
    revalidation_failure.complete_stop(receipt).unwrap();
    let ticket = revalidation_failure.begin_revalidation().unwrap();
    let operation = revalidation_failure
        .begin_revalidation_io(
            ticket,
            FakeRevalidation {
                result: Err(diag("profile_stale")),
            },
        )
        .unwrap();
    let failure = match operation.perform() {
        AsioRevalidationIo::Failed(_, failure) => failure,
        _ => panic!(),
    };
    revalidation_failure.revalidation_failed(failure).unwrap();

    let mut revalidation_panic = Router::test_router();
    let _ = active(&mut revalidation_panic);
    let stop = revalidation_panic
        .begin_stop(FakeBridge {
            counter: Arc::new(AtomicUsize::new(0)),
            result: Ok(()),
        })
        .unwrap();
    let receipt = match stop.perform() {
        StopIo::Drained(receipt) => receipt,
        _ => panic!(),
    };
    revalidation_panic.complete_stop(receipt).unwrap();
    let ticket = revalidation_panic.begin_revalidation().unwrap();
    let failure = match revalidation_panic
        .begin_revalidation_io(ticket, PanicRevalidation)
        .unwrap()
        .perform()
    {
        AsioRevalidationIo::Panicked(_, failure) => failure,
        _ => panic!(),
    };
    assert_eq!(failure.diagnostic.code, "asio_revalidation_panic");
    revalidation_panic.revalidation_failed(failure).unwrap();

    let mut bridge_panic = Router::test_router();
    let _ = active(&mut bridge_panic);
    let stop = bridge_panic.begin_stop(PanicBridge).unwrap();
    let failure = match stop.perform() {
        StopIo::Panicked(_, failure) => failure,
        _ => panic!(),
    };
    bridge_panic.stop_failed(failure).unwrap();
    assert_eq!(bridge_panic.snapshot().reason, Reason::DrainUnproven);

    let mut output_panic = Router::test_router();
    let publication = output_panic
        .begin_publication(Route::Program, factory(&count, Mode::ResourcePanic))
        .unwrap();
    let prepared = match publication.perform_io() {
        FactoryIo::Prepared(prepared) => prepared,
        _ => panic!(),
    };
    let owned = match output_panic.commit(prepared) {
        PublicationCommit::Published(owned) => owned,
        _ => panic!(),
    };
    let action = owned.begin_retire(&mut output_panic).unwrap();
    let (resource, failure) = match action.perform() {
        RetirementIo::Panicked(resource, failure) => (resource, failure),
        _ => panic!(),
    };
    assert_eq!(
        resource.resource.counter.load(Ordering::SeqCst),
        count.load(Ordering::SeqCst)
    );
    output_panic.retire_failed(failure).unwrap();
}

#[test]
fn start_completion_is_bound_to_its_exact_router_generation_operation_and_session() {
    let mut left = Router::test_router();
    let quiesce = left.quiesce().unwrap();
    let ticket = left.admit_start(quiesce).unwrap();
    let receipt = match left
        .begin_start_io(ticket, FakeStart { result: Ok(()) })
        .unwrap()
        .perform()
    {
        AsioStartIo::Started(receipt) => receipt,
        _ => panic!(),
    };
    let replay = AsioStartReceipt {
        ticket: receipt.ticket,
    };
    let mut foreign = Router::test_router();
    assert!(matches!(
        foreign.complete_start(replay),
        Err(Error::StartMismatch)
    ));
    left.complete_start(receipt).unwrap();
    let replay = AsioStartReceipt { ticket };
    assert!(matches!(
        left.complete_start(replay),
        Err(Error::StartMismatch)
    ));
}

#[test]
fn concrete_normal_factory_keeps_the_application_resource_sealed_until_exact_retirement() {
    let calls = Arc::new(AtomicUsize::new(0));
    let start_calls = calls.clone();
    let recovery_calls = calls.clone();
    let retire_calls = calls.clone();
    let factory = normal_route_factory(
        move || {
            start_calls.fetch_add(1, Ordering::SeqCst);
            NormalRouteStart::Ready("normal-rodio-handle".to_owned())
        },
        move || {
            recovery_calls.fetch_add(10, Ordering::SeqCst);
            NormalRouteStart::Ready("recovered-normal-rodio-handle".to_owned())
        },
        move |handle: &mut String| {
            retire_calls.fetch_add(100, Ordering::SeqCst);
            assert_eq!(handle, "normal-rodio-handle");
            Ok(())
        },
    );
    let mut router = Router::test_router();
    let prepared = match router
        .begin_publication(Route::Program, factory)
        .unwrap()
        .perform_io()
    {
        FactoryIo::Prepared(prepared) => prepared,
        _ => panic!("concrete normal factory must publish only through router IO"),
    };
    let owned = match router.commit(prepared) {
        PublicationCommit::Published(owned) => owned,
        _ => panic!("normal resource must not become directly extractable"),
    };
    let retire = match owned.begin_retire(&mut router) {
        Ok(retire) => retire,
        Err(_) => panic!("the admitted normal resource must receive its first retirement permit"),
    };
    let receipt = match retire.perform() {
        RetirementIo::Joined(receipt) => receipt,
        _ => panic!("normal resource must produce a router retirement receipt"),
    };
    router.complete_retire(receipt).unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 101);
}

#[test]
fn concrete_normal_factory_panic_recovery_retains_its_exact_retirement_closure() {
    let calls = Arc::new(AtomicUsize::new(0));
    let recover_calls = calls.clone();
    let retire_calls = calls.clone();
    let factory = normal_route_factory(
        || -> NormalRouteStart<String> {
            panic!("normal constructor panicked after partial platform handle")
        },
        move || {
            recover_calls.fetch_add(1, Ordering::SeqCst);
            NormalRouteStart::Ready("recovered-normal-rodio-handle".to_owned())
        },
        move |handle: &mut String| {
            retire_calls.fetch_add(10, Ordering::SeqCst);
            assert_eq!(handle, "recovered-normal-rodio-handle");
            Ok(())
        },
    );
    let mut router = Router::test_router();
    let handle = match router
        .begin_publication(Route::Program, factory)
        .unwrap()
        .perform_io()
    {
        FactoryIo::Panicked(handle) => handle,
        _ => panic!("the router must catch a normal-route constructor panic"),
    };
    router.publication_panicked(&handle).unwrap();
    let recovery = match handle.begin_recovery(&mut router) {
        Ok(recovery) => recovery,
        Err(_) => panic!("only the panicked factory must retain its recovery capability"),
    };
    let retire = match recovery.perform() {
        PanicRecoveryIo::Recovered(retire) => retire,
        _ => panic!("the original retirement closure must survive factory panic"),
    };
    let receipt = match retire.perform() {
        RetirementIo::Joined(receipt) => receipt,
        _ => panic!("recovered normal resource must still prove retirement"),
    };
    router.complete_retire(receipt).unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 11);
    assert_eq!(router.snapshot().state, State::Locked);
}

#[test]
fn concrete_normal_start_error_keeps_its_partial_resource_for_router_governed_retry() {
    let retire_attempts = Arc::new(AtomicUsize::new(0));
    let attempts = retire_attempts.clone();
    let factory = normal_route_factory(
        || NormalRouteStart::FailedWithResource {
            resource: "partially-open-normal-handle".to_owned(),
            diagnostic: diag("normal_start_after_open_failed"),
        },
        || NormalRouteStart::Ready("unused-recovery".to_owned()),
        move |handle: &mut String| {
            assert_eq!(handle, "partially-open-normal-handle");
            let attempt = attempts.fetch_add(1, Ordering::SeqCst);
            if attempt == 0 {
                Err(diag("partial_handle_stop_failed"))
            } else {
                Ok(())
            }
        },
    );
    let mut router = Router::test_router();
    let prepared = match router
        .begin_publication(Route::Program, factory)
        .unwrap()
        .perform_io()
    {
        FactoryIo::Prepared(prepared) => prepared,
        _ => panic!("error-with-resource must remain a prepared router resource"),
    };
    let retire = match router.commit(prepared) {
        PublicationCommit::MustRetire(retire) => retire,
        _ => panic!("partial normal handle must never become a dropped factory error"),
    };
    assert_eq!(router.snapshot().state, State::Fault);
    assert_eq!(router.snapshot().reason, Reason::PublicationFailed);
    assert_eq!(
        router.snapshot().diagnostic.code,
        "normal_start_after_open_failed"
    );
    let (owned, failure) = match retire.perform() {
        RetirementIo::Failed(owned, failure) => (owned, failure),
        _ => panic!("failed partial-handle cleanup must return the same resource"),
    };
    router.retire_failed(failure).unwrap();
    let retry = match owned.begin_retire(&mut router) {
        Ok(retry) => retry,
        Err(_) => panic!("only the returned partial handle may receive a fresh retry permit"),
    };
    let receipt = match retry.perform() {
        RetirementIo::Joined(receipt) => receipt,
        _ => panic!("partial-handle retry must eventually prove retirement"),
    };
    router.complete_retire(receipt).unwrap();
    assert_eq!(retire_attempts.load(Ordering::SeqCst), 2);
    assert_eq!(router.snapshot().state, State::Locked);
}

#[test]
fn concrete_normal_recovery_error_keeps_its_partial_resource_and_exact_diagnostic() {
    let retire_attempts = Arc::new(AtomicUsize::new(0));
    let attempts = retire_attempts.clone();
    let factory = normal_route_factory(
        || -> NormalRouteStart<String> { panic!("primary normal constructor panic") },
        || NormalRouteStart::FailedWithResource {
            resource: "recovery-partial-normal-handle".to_owned(),
            diagnostic: diag("normal_recovery_after_open_failed"),
        },
        move |handle: &mut String| {
            assert_eq!(handle, "recovery-partial-normal-handle");
            attempts.fetch_add(1, Ordering::SeqCst);
            Ok(())
        },
    );
    let mut router = Router::test_router();
    let handle = match router
        .begin_publication(Route::Program, factory)
        .unwrap()
        .perform_io()
    {
        FactoryIo::Panicked(handle) => handle,
        _ => panic!("primary panic must remain paired with its exact recovery factory"),
    };
    router.publication_panicked(&handle).unwrap();
    let recovery = match handle.begin_recovery(&mut router) {
        Ok(recovery) => recovery,
        Err(_) => panic!("the original panic handle must own recovery"),
    };
    let (retire, failure) = match recovery.perform() {
        PanicRecoveryIo::RecoveredWithStartupFailure(retire, failure) => (retire, failure),
        _ => panic!("recovery error with a resource must not become retry-impossible"),
    };
    router.recovery_startup_failed(failure).unwrap();
    assert_eq!(router.snapshot().state, State::Fault);
    assert_eq!(router.snapshot().reason, Reason::PublicationFailed);
    assert_eq!(
        router.snapshot().diagnostic.code,
        "normal_recovery_after_open_failed"
    );
    let receipt = match retire.perform() {
        RetirementIo::Joined(receipt) => receipt,
        _ => panic!("the recovery error resource must still receive cleanup"),
    };
    router.complete_retire(receipt).unwrap();
    assert_eq!(retire_attempts.load(Ordering::SeqCst), 1);
    assert_eq!(router.snapshot().state, State::Locked);
}

#[test]
fn closure_adapters_follow_the_only_admitted_show_asio_lifecycle() {
    let calls = Arc::new(AtomicUsize::new(0));
    let mut router = Router::test_router();

    let quiesce = router.quiesce().unwrap();
    let start_ticket = router.admit_start(quiesce).unwrap();
    let start_calls = calls.clone();
    let start = router
        .begin_start_io(
            start_ticket,
            asio_start_adapter(move || {
                start_calls.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }),
        )
        .unwrap();
    let receipt = match start.perform() {
        AsioStartIo::Started(receipt) => receipt,
        _ => panic!("router-minted start lease must admit exactly one closure"),
    };
    router.complete_start(receipt).unwrap();
    assert_eq!(router.snapshot().state, State::AsioActive);

    let stop_calls = calls.clone();
    let stop = router
        .begin_stop(asio_stop_adapter(move || {
            stop_calls.fetch_add(10, Ordering::SeqCst);
            Ok(())
        }))
        .unwrap();
    let receipt = match stop.perform() {
        StopIo::Drained(receipt) => receipt,
        _ => panic!("router-minted stop lease must drain before it unlocks"),
    };
    router.complete_stop(receipt).unwrap();
    assert_eq!(router.snapshot().state, State::Locked);

    let pending = router.begin_revalidation().unwrap();
    let revalidate_calls = calls.clone();
    let revalidate = router
        .begin_revalidation_io(
            pending,
            asio_revalidation_adapter(move || {
                revalidate_calls.fetch_add(100, Ordering::SeqCst);
                Ok(())
            }),
        )
        .unwrap();
    let receipt = match revalidate.perform() {
        AsioRevalidationIo::Validated(receipt) => receipt,
        _ => panic!("revalidation must retain the exact locked-state lease"),
    };
    let ready = router.complete_revalidation(receipt).unwrap();
    assert_eq!(router.snapshot().state, State::AsioReady);
    router.start_validated(ready).unwrap();
    assert_eq!(router.snapshot().state, State::AsioStarting);
    assert_eq!(calls.load(Ordering::SeqCst), 111);
    assert!(matches!(
        router.select_normal(),
        Err(Error::NormalSelectionRejected)
    ));
}

#[test]
fn closure_adapter_panics_and_stale_receipts_fail_closed_without_normal_reentry() {
    let mut router = Router::test_router();
    let q = router.quiesce().unwrap();
    let ticket = router.admit_start(q).unwrap();
    let failure = match router
        .begin_start_io(
            ticket,
            asio_start_adapter(|| -> Result<_, _> { panic!("start closure panic") }),
        )
        .unwrap()
        .perform()
    {
        AsioStartIo::Panicked(_, failure) => failure,
        _ => panic!("start closure panic must be caught by the router operation"),
    };
    let stale = AsioStartFailure {
        ticket: failure.ticket,
        diagnostic: failure.diagnostic.clone(),
    };
    router.start_failed(failure).unwrap();
    assert_eq!(router.snapshot().state, State::Fault);
    assert_eq!(router.snapshot().diagnostic.code, "asio_start_panic");
    assert!(matches!(
        router.start_failed(stale),
        Err(Error::StartMismatch)
    ));
    assert!(matches!(
        router.begin_publication(
            Route::Program,
            normal_route_factory(
                || NormalRouteStart::Ready(()),
                || NormalRouteStart::Ready(()),
                |_: &mut ()| Ok::<_, BridgeDiagnostic>(()),
            )
        ),
        Err(Error::Blocked(State::Fault))
    ));
    assert!(matches!(
        router.select_normal(),
        Err(Error::NormalSelectionRejected)
    ));
}

#[test]
fn closure_stop_adapter_panic_latches_a_drain_fault_and_never_reopens_normal() {
    let mut router = Router::test_router();
    let q = router.quiesce().unwrap();
    let ticket = router.admit_start(q).unwrap();
    let receipt = match router
        .begin_start_io(ticket, asio_start_adapter(|| Ok(())))
        .unwrap()
        .perform()
    {
        AsioStartIo::Started(receipt) => receipt,
        _ => panic!(),
    };
    router.complete_start(receipt).unwrap();
    let failure = match router
        .begin_stop(asio_stop_adapter(
            || -> Result<(), BridgeStopFailureDetail> { panic!("stop closure panic") },
        ))
        .unwrap()
        .perform()
    {
        StopIo::Panicked(_, failure) => failure,
        _ => panic!("stop closure panic must be caught"),
    };
    router.stop_failed(failure).unwrap();
    assert_eq!(router.snapshot().state, State::Fault);
    assert_eq!(router.snapshot().reason, Reason::DrainUnproven);
    assert!(matches!(
        router.select_normal(),
        Err(Error::NormalSelectionRejected)
    ));
}

fn test_slot() -> Arc<RouterSlot> {
    Arc::new(RouterSlot {
        inner: std::sync::Mutex::new(Some(Router::test_router())),
    })
}

#[derive(Debug)]
struct StoredNormalResource {
    mixer_name: String,
}

/// Deliberately separate from RouterSlot/AppState: this models the playback
/// runtime retaining a live normal stream while it also holds its own mixer
/// configuration snapshot.  The Arc-backed lease makes this non-self-
/// referential, while the resource itself remains sealed.
struct DetachedNormalRuntime<RetireFn> {
    lease: SlotNormalRouteLease<StoredNormalResource, RetireFn>,
    mixer_name: String,
}

#[test]
fn slot_asio_tasks_release_router_guard_before_every_closure_and_restore_after_receipts() {
    let slot = test_slot();
    slot.quiesce().unwrap();
    let start = slot
        .into_admitted_start_with(|| {
            assert!(matches!(
                slot.snapshot(),
                Err(Error::RouterOperationInProgress)
            ));
            Ok(())
        })
        .unwrap();
    let result = start.perform();
    assert!(result.succeeded());
    assert_eq!(result.snapshot().state, State::AsioActive);
    assert_eq!(slot.snapshot().unwrap().state, State::AsioActive);

    let stop = slot
        .into_stop_with(|| {
            assert!(matches!(
                slot.snapshot(),
                Err(Error::RouterOperationInProgress)
            ));
            Ok(())
        })
        .unwrap();
    let result = stop.perform();
    assert!(result.succeeded());
    assert_eq!(result.snapshot().state, State::Locked);

    let revalidate = slot
        .into_revalidation_with(|| {
            assert!(matches!(
                slot.snapshot(),
                Err(Error::RouterOperationInProgress)
            ));
            Ok(())
        })
        .unwrap();
    let result = revalidate.perform();
    assert!(result.succeeded());
    assert_eq!(result.snapshot().state, State::AsioReady);

    let restart = slot
        .into_validated_start_with(|| {
            assert!(matches!(
                slot.snapshot(),
                Err(Error::RouterOperationInProgress)
            ));
            Ok(())
        })
        .unwrap();
    let result = restart.perform();
    assert!(result.succeeded());
    assert_eq!(result.snapshot().state, State::AsioActive);

    let final_stop = slot.into_stop_with(|| Ok(())).unwrap();
    let result = final_stop.perform();
    assert!(result.succeeded());
    assert_eq!(result.snapshot().state, State::Locked);
    assert_eq!(slot.select_normal().unwrap().state, State::Normal);

    let slot = test_slot();
    slot.quiesce().unwrap();
    assert!(slot
        .into_admitted_start_with(|| Ok(()))
        .unwrap()
        .perform()
        .succeeded());
    assert!(slot
        .into_stop_with(|| Ok(()))
        .unwrap()
        .perform()
        .succeeded());
    assert!(
        slot.into_revalidation_with(|| Ok(()))
            .unwrap()
            .perform()
            .succeeded(),
        "Ready must be created only after a successful locked revalidation"
    );
    assert_eq!(slot.cancel_ready().unwrap().state, State::Locked);
}

#[test]
fn slot_normal_publication_cannot_cross_an_inflight_or_completed_asio_start() {
    let slot = test_slot();
    slot.quiesce().unwrap();

    let start_entered = Arc::new(Barrier::new(2));
    let allow_start_to_finish = Arc::new(Barrier::new(2));
    let start_calls = Arc::new(AtomicUsize::new(0));
    let start_slot = Arc::clone(&slot);
    let entered = Arc::clone(&start_entered);
    let finish = Arc::clone(&allow_start_to_finish);
    let calls = Arc::clone(&start_calls);
    let start_worker = std::thread::spawn(move || {
        let task = start_slot
            .into_admitted_start_with(move || {
                calls.fetch_add(1, Ordering::SeqCst);
                entered.wait();
                finish.wait();
                Ok(())
            })
            .expect("the quiesced router must admit the ASIO start");
        task.perform()
    });

    start_entered.wait();
    assert!(matches!(
        slot.snapshot(),
        Err(Error::RouterOperationInProgress)
    ));
    let during_start = slot.into_normal_publication(
        Route::Program,
        normal_route_factory(
            || NormalRouteStart::Ready(()),
            || NormalRouteStart::Ready(()),
            |_: &mut ()| Ok::<_, BridgeDiagnostic>(()),
        ),
    );
    assert!(matches!(
        during_start,
        Err(Error::RouterOperationInProgress)
    ));

    allow_start_to_finish.wait();
    let start_result = start_worker.join().expect("ASIO start worker must join");
    assert!(start_result.succeeded());
    assert_eq!(start_result.snapshot().state, State::AsioActive);
    assert_eq!(start_calls.load(Ordering::SeqCst), 1);

    let after_start = slot.into_normal_publication(
        Route::Program,
        normal_route_factory(
            || NormalRouteStart::Ready(()),
            || NormalRouteStart::Ready(()),
            |_: &mut ()| Ok::<_, BridgeDiagnostic>(()),
        ),
    );
    assert!(matches!(
        after_start,
        Err(Error::Blocked(State::AsioActive))
    ));
    assert_eq!(slot.snapshot().unwrap().state, State::AsioActive);
}

#[test]
fn slot_normal_publication_success_and_retry_never_drop_the_concrete_resource() {
    let slot = test_slot();
    let retire_attempts = Arc::new(AtomicUsize::new(0));
    let attempts = retire_attempts.clone();
    let factory = normal_route_factory(
        || {
            NormalRouteStart::Ready(StoredNormalResource {
                mixer_name: "program-mixer".to_owned(),
            })
        },
        || {
            NormalRouteStart::Ready(StoredNormalResource {
                mixer_name: "unused-recovery-mixer".to_owned(),
            })
        },
        move |resource: &mut StoredNormalResource| {
            assert_eq!(resource.mixer_name, "program-mixer");
            let attempt = attempts.fetch_add(1, Ordering::SeqCst);
            if attempt == 0 {
                Err(diag("slot_normal_first_retire_failed"))
            } else {
                Ok(())
            }
        },
    );
    let result = slot
        .into_normal_publication(Route::Program, factory)
        .unwrap()
        .perform();
    assert!(result.succeeded());
    let (published, retry) = result.into_handles();
    assert!(retry.is_none());
    let published = published.expect("normal construction must retain the exact resource");
    let mixer_name = published.with_resource(|resource| resource.mixer_name.clone());
    let stored = DetachedNormalRuntime {
        lease: published,
        mixer_name,
    };
    assert_eq!(stored.mixer_name, "program-mixer");
    assert_eq!(Arc::strong_count(&slot), 2);
    drop(slot);
    let retirement = match stored.lease.into_retirement() {
        Ok(retirement) => retirement,
        Err((_published, error)) => {
            panic!("normal route retirement admission was rejected: {error:?}")
        }
    };
    let result = retirement.perform();
    assert!(!result.succeeded());
    assert_eq!(result.snapshot().state, State::Fault);
    let retry = result
        .into_retry()
        .expect("failed first retirement must retain exactly one retry handle");
    let retry = match retry.into_retry() {
        Ok(retry) => retry,
        Err((_retry, error)) => panic!("normal route retry admission was rejected: {error:?}"),
    };
    let result = retry.perform();
    assert!(result.succeeded());
    assert!(result.into_retry().is_none());
    assert_eq!(retire_attempts.load(Ordering::SeqCst), 2);
}

#[test]
fn slot_normal_start_error_and_panic_recovery_retire_before_any_unlock() {
    let slot = test_slot();
    let start_cleanup = Arc::new(AtomicUsize::new(0));
    let cleanup = start_cleanup.clone();
    let result = slot
        .into_normal_publication(
            Route::Program,
            normal_route_factory(
                || NormalRouteStart::FailedWithResource {
                    resource: Some("slot-start-error-resource".to_owned()),
                    diagnostic: diag("slot_start_error_after_open"),
                },
                || NormalRouteStart::Ready(None),
                move |resource: &mut Option<String>| {
                    assert_eq!(resource.as_deref(), Some("slot-start-error-resource"));
                    *resource = None;
                    cleanup.fetch_add(1, Ordering::SeqCst);
                    Ok(())
                },
            ),
        )
        .unwrap()
        .perform();
    assert!(result.succeeded());
    assert_eq!(result.snapshot().state, State::Locked);
    assert!(result.into_handles().0.is_none());
    assert_eq!(start_cleanup.load(Ordering::SeqCst), 1);

    let slot = test_slot();
    let panic_cleanup = Arc::new(AtomicUsize::new(0));
    let cleanup = panic_cleanup.clone();
    let result = slot
        .into_normal_publication(
            Route::Program,
            normal_route_factory(
                || -> NormalRouteStart<String> { panic!("slot normal constructor panic") },
                || NormalRouteStart::Ready("slot-recovered-resource".to_owned()),
                move |resource: &mut String| {
                    assert_eq!(resource, "slot-recovered-resource");
                    cleanup.fetch_add(1, Ordering::SeqCst);
                    Ok(())
                },
            ),
        )
        .unwrap()
        .perform();
    assert!(result.succeeded());
    assert_eq!(result.snapshot().state, State::Locked);
    assert!(result.into_handles().0.is_none());
    assert_eq!(panic_cleanup.load(Ordering::SeqCst), 1);
}

#[test]
fn normal_failure_before_resource_has_no_dummy_handle_or_retirement() {
    let slot = test_slot();
    let retirement_calls = Arc::new(AtomicUsize::new(0));
    let calls = retirement_calls.clone();
    let result = slot
        .into_normal_publication(
            Route::Program,
            normal_route_factory(
                || NormalRouteStart::FailedBeforeResource(diag("no_default_output_device")),
                || NormalRouteStart::Ready("unreachable recovery".to_owned()),
                move |_resource: &mut String| {
                    calls.fetch_add(1, Ordering::SeqCst);
                    Ok(())
                },
            ),
        )
        .unwrap()
        .perform();
    assert!(!result.succeeded());
    assert_eq!(result.snapshot().state, State::Fault);
    assert_eq!(result.snapshot().reason, Reason::PublicationFailed);
    assert_eq!(
        result.snapshot().diagnostic.code,
        "no_default_output_device"
    );
    let (published, retry) = result.into_handles();
    assert!(published.is_none());
    assert!(retry.is_none());
    assert_eq!(retirement_calls.load(Ordering::SeqCst), 0);
    assert!(
        slot.lock().as_ref().unwrap().routes.is_empty(),
        "a before-resource failure must delete the publishing record rather than strand a retire"
    );
}

#[test]
fn production_output_surface_exposes_only_router_slot_closure_tasks() {
    let source = include_str!("audio_output_router.rs");
    for hidden in [
        "pub(crate) trait AsioStartLifecycle",
        "pub(crate) trait AsioRevalidationLifecycle",
        "pub(crate) trait BridgeStop",
        "pub(crate) fn asio_start_adapter",
        "pub(crate) fn asio_stop_adapter",
        "pub(crate) fn asio_revalidation_adapter",
        "pub(crate) struct Router {",
        "pub(crate) struct Publication",
        "pub(crate) struct StopOperation",
        "pub(crate) struct AsioStartOperation",
        "pub(crate) struct AsioRevalidationOperation",
        "pub(crate) fn begin_publication",
        "pub(crate) fn begin_start_io",
        "pub(crate) fn begin_stop",
        "pub(crate) fn begin_revalidation_io",
    ] {
        assert!(
            !source.contains(hidden),
            "production code must not bypass RouterSlot through {hidden}"
        );
    }
    for admitted in [
        "pub(crate) struct RouterSlot",
        "pub(crate) enum NormalRouteStart",
        "pub(crate) fn into_admitted_start_with",
        "pub(crate) fn into_validated_start_with",
        "pub(crate) fn into_stop_with",
        "pub(crate) fn into_revalidation_with",
        "pub(crate) fn into_normal_publication",
        "slot: Arc<RouterSlot>",
        "pub(crate) fn with_resource<R>",
    ] {
        assert!(
            source.contains(admitted),
            "the closure-only slot surface must retain {admitted}"
        );
    }
    assert!(
        !source.contains("NormalRouteFailure"),
        "the old dummy-resource constructor error API must be absent"
    );
}
