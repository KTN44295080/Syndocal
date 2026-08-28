//! Exclusive process-wide audio-output authority core.
//!
//! No Rodio/ASIO calls are wired here.  A future sealed adapter is the sole OS
//! trust boundary and must own every handle it reports as constructed/drained.

use std::{
    collections::BTreeMap,
    marker::PhantomData,
    num::NonZeroUsize,
    panic::{catch_unwind, AssertUnwindSafe},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex,
    },
};

static PROCESS_OWNER: AtomicBool = AtomicBool::new(false);
static NEXT_INSTANCE: AtomicU64 = AtomicU64::new(1);

mod seal {
    pub(super) trait Factory {}
    pub(super) trait Resource {}
    pub(super) trait Bridge {}
    pub(super) trait AsioStart {}
    pub(super) trait Revalidation {}
}

trait OutputFactory: seal::Factory {
    type Resource: OutputResource;
    /// The adapter receives this router-minted capability before it touches an
    /// OS output.  It cannot construct, clone, or substitute the capability.
    fn start(
        &mut self,
        capability: &FactoryStartCapability,
    ) -> Result<Self::Resource, BridgeDiagnostic>;
    /// Called only on the exact factory instance which panicked while starting.
    /// The capability stays owned by `PanicRecovery` until its resource has
    /// completed the same stop/join proof as any other route.
    fn recover_after_panic(
        &mut self,
        capability: &FactoryPanicCapability,
    ) -> Result<Self::Resource, BridgeDiagnostic>;
}
trait OutputResource: seal::Resource {
    fn stop_and_join(&mut self) -> Result<(), BridgeDiagnostic>;
    /// A constructor may have acquired a concrete OS resource before it
    /// detects an error.  Such a resource is never dropped as an ordinary
    /// factory failure: `commit` moves it directly to router-governed
    /// retirement first.
    fn startup_failure(&self) -> Option<&BridgeDiagnostic> {
        None
    }
}
trait BridgeStop: seal::Bridge {
    fn stop_unpublish_drain_close(
        &mut self,
        lease: &BridgeStopLease,
    ) -> Result<(), BridgeStopFailureDetail>;
}
trait AsioStartLifecycle: seal::AsioStart {
    fn start(&mut self, lease: &AsioStartLease) -> Result<(), BridgeDiagnostic>;
}
trait AsioRevalidationLifecycle: seal::Revalidation {
    fn revalidate(&mut self, lease: &AsioRevalidationLease) -> Result<(), BridgeDiagnostic>;
}

/// The only app-facing way to admit a normal output construction.
///
/// The closure receives no router capability at all: only `RouterSlot` can
/// invoke it, after it has removed Router from its private mutex.  Application
/// code can therefore own concrete Rodio handles without implementing sealed
/// lifecycle traits, manufacturing a publication capability, or holding the
/// router control guard across I/O.  The matching recovery closure is
/// deliberately required: a panic after a platform handle was partly created
/// must still run on the original owner before the router can unlock.
pub(crate) struct NormalRouteFactory<T, Start, Recover, Retire> {
    start: Option<Start>,
    recover: Option<Recover>,
    retire: Option<Retire>,
    _resource: PhantomData<fn() -> T>,
}

/// Concrete wrapper for an application-owned normal output resource.  Its
/// inner value never escapes this module, which makes a successful
/// `stop_and_join` receipt the only route-retirement proof the router accepts.
pub(crate) struct NormalRouteResource<T, Retire> {
    resource: T,
    retire: Retire,
    startup_failure: Option<BridgeDiagnostic>,
}

/// Exact outcome of a normal-output constructor or its panic-recovery path.
///
/// `FailedBeforeResource` is for failures such as "no default device" where
/// there is provably no OS object to retire.  `FailedWithResource` is for a
/// partial-open failure: its exact resource is sealed into a route and must
/// receive router-governed retirement before this process can unlock.  An
/// optional platform handle can therefore use `T = Option<Handle>` without
/// inventing a dummy resource; `None` belongs in `FailedBeforeResource`, and
/// `Some(handle)` belongs in `FailedWithResource`.
pub(crate) enum NormalRouteStart<T> {
    Ready(T),
    FailedBeforeResource(BridgeDiagnostic),
    FailedWithResource {
        resource: T,
        diagnostic: BridgeDiagnostic,
    },
}

/// Creates a sealed normal-route factory from application-owned closures.
/// `start` and `recover` are one-shot by construction; `retire` is retained
/// with the resource so a failed retirement can receive exactly one fresh
/// router permit and retry the same concrete handle.
pub(crate) fn normal_route_factory<T, Start, Recover, Retire>(
    start: Start,
    recover: Recover,
    retire: Retire,
) -> NormalRouteFactory<T, Start, Recover, Retire>
where
    Start: FnOnce() -> NormalRouteStart<T>,
    Recover: FnOnce() -> NormalRouteStart<T>,
    Retire: FnMut(&mut T) -> Result<(), BridgeDiagnostic>,
{
    NormalRouteFactory {
        start: Some(start),
        recover: Some(recover),
        retire: Some(retire),
        _resource: PhantomData,
    }
}

impl<T, Start, Recover, Retire> seal::Factory for NormalRouteFactory<T, Start, Recover, Retire>
where
    Start: FnOnce() -> NormalRouteStart<T>,
    Recover: FnOnce() -> NormalRouteStart<T>,
    Retire: FnMut(&mut T) -> Result<(), BridgeDiagnostic>,
{
}

impl<T, Start, Recover, Retire> OutputFactory for NormalRouteFactory<T, Start, Recover, Retire>
where
    Start: FnOnce() -> NormalRouteStart<T>,
    Recover: FnOnce() -> NormalRouteStart<T>,
    Retire: FnMut(&mut T) -> Result<(), BridgeDiagnostic>,
{
    type Resource = NormalRouteResource<T, Retire>;

    fn start(
        &mut self,
        _capability: &FactoryStartCapability,
    ) -> Result<Self::Resource, BridgeDiagnostic> {
        let start = self.start.take().ok_or_else(|| {
            internal(
                Reason::PublicationFailed,
                "normal_route_start_reused",
                "Normal-route start capability was reused.",
            )
        })?;
        // Do not move the retirement closure until Start has returned.  If
        // platform construction panics after a partial handle exists, the
        // PanicHandle retains this exact factory and its recovery still owns
        // the original retirement closure.
        let result = start();
        let (resource, startup_failure) = match result {
            NormalRouteStart::Ready(resource) => (resource, None),
            NormalRouteStart::FailedBeforeResource(diagnostic) => return Err(diagnostic),
            NormalRouteStart::FailedWithResource {
                resource,
                diagnostic,
            } => (resource, Some(diagnostic)),
        };
        let retire = self.retire.take().ok_or_else(|| {
            internal(
                Reason::PublicationFailed,
                "normal_route_retire_missing",
                "Normal-route retirement capability is missing.",
            )
        })?;
        Ok(NormalRouteResource {
            resource,
            retire,
            startup_failure,
        })
    }

    fn recover_after_panic(
        &mut self,
        _capability: &FactoryPanicCapability,
    ) -> Result<Self::Resource, BridgeDiagnostic> {
        let recover = self.recover.take().ok_or_else(|| {
            internal(
                Reason::PublicationPanicked,
                "normal_route_recovery_reused",
                "Normal-route panic recovery capability was reused.",
            )
        })?;
        // Keep the matching retirement closure in the original factory while
        // recovery executes for the same reason as the primary Start path.
        let result = recover();
        let (resource, startup_failure) = match result {
            NormalRouteStart::Ready(resource) => (resource, None),
            NormalRouteStart::FailedBeforeResource(diagnostic) => return Err(diagnostic),
            NormalRouteStart::FailedWithResource {
                resource,
                diagnostic,
            } => (resource, Some(diagnostic)),
        };
        let retire = self.retire.take().ok_or_else(|| {
            internal(
                Reason::PublicationPanicked,
                "normal_route_retire_missing",
                "Normal-route retirement capability is missing.",
            )
        })?;
        Ok(NormalRouteResource {
            resource,
            retire,
            startup_failure,
        })
    }
}

impl<T, Retire> seal::Resource for NormalRouteResource<T, Retire> where
    Retire: FnMut(&mut T) -> Result<(), BridgeDiagnostic>
{
}

impl<T, Retire> OutputResource for NormalRouteResource<T, Retire>
where
    Retire: FnMut(&mut T) -> Result<(), BridgeDiagnostic>,
{
    fn stop_and_join(&mut self) -> Result<(), BridgeDiagnostic> {
        (self.retire)(&mut self.resource)
    }

    fn startup_failure(&self) -> Option<&BridgeDiagnostic> {
        self.startup_failure.as_ref()
    }
}

/// Module-private one-shot wrapper for a v3 bridge Start operation.  The
/// app-facing closure sees neither the lease nor the lifecycle trait.
struct AsioStartAdapter<'a> {
    start: Option<Box<dyn FnOnce() -> Result<(), BridgeDiagnostic> + 'a>>,
}

fn asio_start_adapter<'a>(
    start: impl FnOnce() -> Result<(), BridgeDiagnostic> + 'a,
) -> AsioStartAdapter<'a> {
    AsioStartAdapter {
        start: Some(Box::new(start)),
    }
}

impl seal::AsioStart for AsioStartAdapter<'_> {}

impl AsioStartLifecycle for AsioStartAdapter<'_> {
    fn start(&mut self, _lease: &AsioStartLease) -> Result<(), BridgeDiagnostic> {
        self.start.take().ok_or_else(|| {
            internal(
                Reason::AsioStartFailed,
                "asio_start_reused",
                "ASIO start lease was reused.",
            )
        })?()
    }
}

/// App-facing one-shot wrapper for v3 Stop/Close.  It preserves the existing
/// dispatch -> callback-drain -> close diagnostic stages while the module
/// retains the router-minted session lease.
struct AsioStopAdapter<'a> {
    stop: Option<Box<dyn FnOnce() -> Result<(), BridgeStopFailureDetail> + 'a>>,
}

fn asio_stop_adapter<'a>(
    stop: impl FnOnce() -> Result<(), BridgeStopFailureDetail> + 'a,
) -> AsioStopAdapter<'a> {
    AsioStopAdapter {
        stop: Some(Box::new(stop)),
    }
}

impl seal::Bridge for AsioStopAdapter<'_> {}

impl BridgeStop for AsioStopAdapter<'_> {
    fn stop_unpublish_drain_close(
        &mut self,
        _lease: &BridgeStopLease,
    ) -> Result<(), BridgeStopFailureDetail> {
        self.stop.take().ok_or_else(|| {
            BridgeStopFailureDetail::drain(
                NonZeroUsize::new(1).expect("one is non-zero"),
                internal(
                    Reason::DrainUnproven,
                    "asio_stop_reused",
                    "ASIO stop lease was reused.",
                ),
            )
        })?()
    }
}

/// App-facing one-shot wrapper for a driver/profile revalidation.  Discovery
/// is impossible until the router has completed Stop into its locked state.
struct AsioRevalidationAdapter<'a> {
    revalidate: Option<Box<dyn FnOnce() -> Result<(), BridgeDiagnostic> + 'a>>,
}

fn asio_revalidation_adapter<'a>(
    revalidate: impl FnOnce() -> Result<(), BridgeDiagnostic> + 'a,
) -> AsioRevalidationAdapter<'a> {
    AsioRevalidationAdapter {
        revalidate: Some(Box::new(revalidate)),
    }
}

impl seal::Revalidation for AsioRevalidationAdapter<'_> {}

impl AsioRevalidationLifecycle for AsioRevalidationAdapter<'_> {
    fn revalidate(&mut self, _lease: &AsioRevalidationLease) -> Result<(), BridgeDiagnostic> {
        self.revalidate.take().ok_or_else(|| {
            internal(
                Reason::RevalidationFailed,
                "asio_revalidation_reused",
                "ASIO revalidation lease was reused.",
            )
        })?()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Route {
    Program,
    CueFollowProgram,
    CueExplicitDevice,
    PrepareWorker,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum State {
    Normal,
    Quiescing,
    AsioReady,
    AsioStarting,
    AsioActive,
    Fault,
    Locked,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Reason {
    Normal,
    Quiescing,
    AsioStarting,
    AsioActive,
    AsioStartFailed,
    StopPending,
    DispatchFailed,
    DrainUnproven,
    CloseFailed,
    PublicationFailed,
    PublicationPanicked,
    RetirementFailed,
    RetirementPanicked,
    Revalidating,
    Ready,
    RevalidationFailed,
    ExplicitSelection,
    InvalidDiagnostic,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BridgeDiagnostic {
    pub(crate) code: String,
    pub(crate) message: String,
}
impl BridgeDiagnostic {
    pub(crate) fn checked(
        code: impl Into<String>,
        message: impl Into<String>,
    ) -> Result<Self, Error> {
        let d = Self {
            code: code.into(),
            message: message.into(),
        };
        d.validate()?;
        Ok(d)
    }
    fn validate(&self) -> Result<(), Error> {
        if self.code.is_empty()
            || self.code.len() > 64
            || self.code.trim() != self.code
            || !self
                .code
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.'))
            || self.message.is_empty()
            || self.message.len() > 256
            || self.message.trim() != self.message
            || self.message.chars().any(char::is_control)
        {
            Err(Error::InvalidDiagnostic)
        } else {
            Ok(())
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Snapshot {
    pub(crate) state: State,
    pub(crate) instance: u64,
    pub(crate) generation: u64,
    pub(crate) operation: Option<u64>,
    pub(crate) session: Option<u64>,
    pub(crate) remaining_drains: usize,
    pub(crate) reason: Reason,
    pub(crate) diagnostic: BridgeDiagnostic,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Error {
    ProcessOwnerTaken,
    ForeignTicket,
    Blocked(State),
    PublicationRejected,
    RetirementRejected,
    ReceiptMismatch,
    FailureMismatch,
    QuiesceMismatch,
    QuiesceIncomplete(usize),
    StartMismatch,
    StopRejected,
    StopMismatch,
    NormalSelectionRejected,
    RevalidationRejected,
    RevalidationMismatch,
    ReadyMismatch,
    /// The sole Router has been removed into a closure-only task.  Callers
    /// must await that task's receipt/result instead of attempting a second
    /// lifecycle operation or re-entering its control mutex.
    RouterOperationInProgress,
    InvalidDiagnostic,
    IdentityExhausted,
}
fn internal(_reason: Reason, code: &str, msg: &str) -> BridgeDiagnostic {
    BridgeDiagnostic {
        code: code.into(),
        message: msg.into(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct RouteTicket {
    instance: u64,
    id: u64,
    generation: u64,
    route: Route,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct QuiesceTicket {
    instance: u64,
    generation: u64,
    operation: u64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct StartTicket {
    instance: u64,
    generation: u64,
    operation: u64,
    session: u64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ReadyTicket {
    instance: u64,
    generation: u64,
    operation: u64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RouteState {
    Publishing,
    Owned,
    RetireRequested,
    Retiring,
    Panic,
}
#[derive(Clone, Copy)]
struct Record {
    ticket: RouteTicket,
    state: RouteState,
    retire: Option<u64>,
    factory_nonce: u64,
}

/// An opaque start capability.  It is intentionally neither `Clone` nor
/// `Copy`; only this module can mint it.
#[derive(Debug)]
struct FactoryStartCapability {
    ticket: RouteTicket,
    factory_nonce: u64,
}
/// The start capability converted after that exact factory panics.  It remains
/// paired with the original factory object in `PanicRecovery`.
#[derive(Debug)]
struct FactoryPanicCapability {
    ticket: RouteTicket,
    factory_nonce: u64,
}
impl FactoryStartCapability {
    fn into_panic(self) -> FactoryPanicCapability {
        FactoryPanicCapability {
            ticket: self.ticket,
            factory_nonce: self.factory_nonce,
        }
    }
}

#[derive(Debug)]
struct Publication<F> {
    ticket: RouteTicket,
    capability: FactoryStartCapability,
    factory: F,
}
#[derive(Debug)]
struct Prepared<R> {
    ticket: RouteTicket,
    resource: R,
}
#[derive(Debug)]
struct Owned<R> {
    ticket: RouteTicket,
    resource: R,
}
#[derive(Debug)]
struct PanicHandle<F> {
    ticket: RouteTicket,
    capability: FactoryPanicCapability,
    factory: F,
}
#[derive(Debug)]
struct PanicRecovery<F> {
    ticket: RouteTicket,
    operation: u64,
    capability: FactoryPanicCapability,
    factory: F,
}
#[derive(Debug)]
struct Retire<R> {
    ticket: RouteTicket,
    operation: u64,
    resource: R,
}
#[derive(Debug, PartialEq, Eq)]
struct Receipt {
    ticket: RouteTicket,
    operation: u64,
}
#[derive(Debug, Clone, PartialEq, Eq)]
struct RetirementFailure {
    ticket: RouteTicket,
    operation: u64,
    diagnostic: BridgeDiagnostic,
    panic: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
struct RecoveryStartupFailure {
    ticket: RouteTicket,
    operation: u64,
    diagnostic: BridgeDiagnostic,
}
#[derive(Debug, Clone, PartialEq, Eq)]
struct PublicationFailure {
    ticket: RouteTicket,
    diagnostic: BridgeDiagnostic,
}
enum FactoryIo<F: OutputFactory> {
    Prepared(Prepared<F::Resource>),
    Failed(PublicationFailure),
    Panicked(PanicHandle<F>),
}
enum PublicationCommit<R> {
    Published(Owned<R>),
    MustRetire(Retire<R>),
    Rejected(Prepared<R>, Error),
}
enum RetirementIo<R> {
    Joined(Receipt),
    Failed(Owned<R>, RetirementFailure),
    Panicked(Owned<R>, RetirementFailure),
}
enum PanicRecoveryIo<F: OutputFactory> {
    Recovered(Retire<F::Resource>),
    RecoveredWithStartupFailure(Retire<F::Resource>, RecoveryStartupFailure),
    Failed(PanicRecovery<F>, RetirementFailure),
    Panicked(PanicRecovery<F>, RetirementFailure),
}

impl<F: OutputFactory> Publication<F> {
    fn perform_io(mut self) -> FactoryIo<F> {
        let result = catch_unwind(AssertUnwindSafe(|| self.factory.start(&self.capability)));
        match result {
            Ok(Ok(r)) => FactoryIo::Prepared(Prepared {
                ticket: self.ticket,
                resource: r,
            }),
            Ok(Err(d)) => FactoryIo::Failed(PublicationFailure {
                ticket: self.ticket,
                diagnostic: d,
            }),
            Err(_) => FactoryIo::Panicked(PanicHandle {
                ticket: self.ticket,
                capability: self.capability.into_panic(),
                factory: self.factory,
            }),
        }
    }
}
impl<R: OutputResource> Owned<R> {
    fn begin_retire(self, router: &mut Router) -> Result<Retire<R>, (Self, Error)> {
        match router.begin_retire(self.ticket) {
            Ok(operation) => Ok(Retire {
                ticket: self.ticket,
                operation,
                resource: self.resource,
            }),
            Err(e) => Err((self, e)),
        }
    }
}
impl<F: OutputFactory> PanicHandle<F> {
    fn begin_recovery(self, router: &mut Router) -> Result<PanicRecovery<F>, (Self, Error)> {
        if let Err(error) = router.check_panic_capability(self.ticket, &self.capability) {
            return Err((self, error));
        }
        match router.begin_retire(self.ticket) {
            Ok(operation) => Ok(PanicRecovery {
                ticket: self.ticket,
                operation,
                capability: self.capability,
                factory: self.factory,
            }),
            Err(error) => Err((self, error)),
        }
    }
}
impl<F: OutputFactory> PanicRecovery<F> {
    fn perform(mut self) -> PanicRecoveryIo<F> {
        let ticket = self.ticket;
        let operation = self.operation;
        let result = catch_unwind(AssertUnwindSafe(|| {
            self.factory.recover_after_panic(&self.capability)
        }));
        match result {
            Ok(Ok(resource)) => {
                let startup_failure = resource.startup_failure().cloned();
                let retire = Retire {
                    ticket,
                    operation,
                    resource,
                };
                match startup_failure {
                    Some(diagnostic) => PanicRecoveryIo::RecoveredWithStartupFailure(
                        retire,
                        RecoveryStartupFailure {
                            ticket,
                            operation,
                            diagnostic,
                        },
                    ),
                    None => PanicRecoveryIo::Recovered(retire),
                }
            }
            Ok(Err(diagnostic)) => PanicRecoveryIo::Failed(
                self,
                RetirementFailure {
                    ticket,
                    operation,
                    diagnostic,
                    panic: false,
                },
            ),
            Err(_) => PanicRecoveryIo::Panicked(
                self,
                RetirementFailure {
                    ticket,
                    operation,
                    diagnostic: internal(
                        Reason::RetirementPanicked,
                        "panic_recovery_panic",
                        "Panic recovery lifecycle panicked.",
                    ),
                    panic: true,
                },
            ),
        }
    }
    fn retry(self, router: &mut Router) -> Result<Self, (Self, Error)> {
        match router.begin_retire(self.ticket) {
            Ok(operation) => Ok(Self { operation, ..self }),
            Err(error) => Err((self, error)),
        }
    }
}
impl<R: OutputResource> Retire<R> {
    fn perform(mut self) -> RetirementIo<R> {
        let t = self.ticket;
        let o = self.operation;
        match catch_unwind(AssertUnwindSafe(|| self.resource.stop_and_join())) {
            Ok(Ok(())) => RetirementIo::Joined(Receipt {
                ticket: t,
                operation: o,
            }),
            Ok(Err(d)) => RetirementIo::Failed(
                Owned {
                    ticket: t,
                    resource: self.resource,
                },
                RetirementFailure {
                    ticket: t,
                    operation: o,
                    diagnostic: d,
                    panic: false,
                },
            ),
            Err(_) => RetirementIo::Panicked(
                Owned {
                    ticket: t,
                    resource: self.resource,
                },
                RetirementFailure {
                    ticket: t,
                    operation: o,
                    diagnostic: internal(
                        Reason::RetirementPanicked,
                        "lifecycle_panic",
                        "Output lifecycle panicked.",
                    ),
                    panic: true,
                },
            ),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BridgeStopStage {
    Dispatch,
    Drain { unresolved: NonZeroUsize },
    Close,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BridgeStopFailureDetail {
    stage: BridgeStopStage,
    diagnostic: BridgeDiagnostic,
}
impl BridgeStopFailureDetail {
    pub(crate) fn dispatch(d: BridgeDiagnostic) -> Self {
        Self {
            stage: BridgeStopStage::Dispatch,
            diagnostic: d,
        }
    }
    pub(crate) fn drain(n: NonZeroUsize, d: BridgeDiagnostic) -> Self {
        Self {
            stage: BridgeStopStage::Drain { unresolved: n },
            diagnostic: d,
        }
    }
    pub(crate) fn close(d: BridgeDiagnostic) -> Self {
        Self {
            stage: BridgeStopStage::Close,
            diagnostic: d,
        }
    }
}
#[derive(Debug)]
struct BridgeStopLease {
    ticket: StartTicket,
}
#[derive(Debug)]
struct StopOperation<B> {
    bridge: B,
    lease: BridgeStopLease,
}
#[derive(Debug, PartialEq, Eq)]
struct DrainReceipt {
    ticket: StartTicket,
}
#[derive(Debug, Clone, PartialEq, Eq)]
struct StopFailure {
    ticket: StartTicket,
    detail: BridgeStopFailureDetail,
}
enum StopIo<B> {
    Drained(DrainReceipt),
    Failed(B, StopFailure),
    Panicked(B, StopFailure),
}
impl<B: BridgeStop> StopOperation<B> {
    fn perform(mut self) -> StopIo<B> {
        let t = self.lease.ticket;
        match catch_unwind(AssertUnwindSafe(|| {
            self.bridge.stop_unpublish_drain_close(&self.lease)
        })) {
            Ok(Ok(())) => StopIo::Drained(DrainReceipt { ticket: t }),
            Ok(Err(d)) => StopIo::Failed(
                self.bridge,
                StopFailure {
                    ticket: t,
                    detail: d,
                },
            ),
            Err(_) => StopIo::Panicked(
                self.bridge,
                StopFailure {
                    ticket: t,
                    detail: BridgeStopFailureDetail::drain(
                        NonZeroUsize::new(1).unwrap(),
                        internal(
                            Reason::DrainUnproven,
                            "bridge_stop_panic",
                            "Bridge stop lifecycle panicked.",
                        ),
                    ),
                },
            ),
        }
    }
}

#[derive(Debug)]
struct AsioStartLease {
    ticket: StartTicket,
}
#[derive(Debug)]
struct AsioStartOperation<B> {
    adapter: B,
    lease: AsioStartLease,
}
#[derive(Debug, PartialEq, Eq)]
struct AsioStartReceipt {
    ticket: StartTicket,
}
#[derive(Debug, Clone, PartialEq, Eq)]
struct AsioStartFailure {
    ticket: StartTicket,
    diagnostic: BridgeDiagnostic,
}
enum AsioStartIo<B> {
    Started(AsioStartReceipt),
    Failed(B, AsioStartFailure),
    Panicked(B, AsioStartFailure),
}
impl<B: AsioStartLifecycle> AsioStartOperation<B> {
    fn perform(mut self) -> AsioStartIo<B> {
        let ticket = self.lease.ticket;
        match catch_unwind(AssertUnwindSafe(|| self.adapter.start(&self.lease))) {
            Ok(Ok(())) => AsioStartIo::Started(AsioStartReceipt { ticket }),
            Ok(Err(diagnostic)) => {
                AsioStartIo::Failed(self.adapter, AsioStartFailure { ticket, diagnostic })
            }
            Err(_) => AsioStartIo::Panicked(
                self.adapter,
                AsioStartFailure {
                    ticket,
                    diagnostic: internal(
                        Reason::AsioStartFailed,
                        "asio_start_panic",
                        "ASIO start lifecycle panicked.",
                    ),
                },
            ),
        }
    }
}
#[derive(Debug)]
struct AsioRevalidationLease {
    ticket: ReadyTicket,
}
#[derive(Debug)]
struct AsioRevalidationOperation<B> {
    adapter: B,
    lease: AsioRevalidationLease,
}
#[derive(Debug, PartialEq, Eq)]
struct AsioRevalidationReceipt {
    ticket: ReadyTicket,
}
#[derive(Debug, Clone, PartialEq, Eq)]
struct AsioRevalidationFailure {
    ticket: ReadyTicket,
    diagnostic: BridgeDiagnostic,
}
enum AsioRevalidationIo<B> {
    Validated(AsioRevalidationReceipt),
    Failed(B, AsioRevalidationFailure),
    Panicked(B, AsioRevalidationFailure),
}
impl<B: AsioRevalidationLifecycle> AsioRevalidationOperation<B> {
    fn perform(mut self) -> AsioRevalidationIo<B> {
        let ticket = self.lease.ticket;
        match catch_unwind(AssertUnwindSafe(|| self.adapter.revalidate(&self.lease))) {
            Ok(Ok(())) => AsioRevalidationIo::Validated(AsioRevalidationReceipt { ticket }),
            Ok(Err(diagnostic)) => AsioRevalidationIo::Failed(
                self.adapter,
                AsioRevalidationFailure { ticket, diagnostic },
            ),
            Err(_) => AsioRevalidationIo::Panicked(
                self.adapter,
                AsioRevalidationFailure {
                    ticket,
                    diagnostic: internal(
                        Reason::RevalidationFailed,
                        "asio_revalidation_panic",
                        "ASIO revalidation lifecycle panicked.",
                    ),
                },
            ),
        }
    }
}

/// The only production result of a bridge I/O operation.  It owns the Router
/// again, so callers must take the `Router` out of `Mutex<Option<Router>>`
/// before I/O and explicitly put it back afterwards.  A `MutexGuard<Router>`
/// therefore cannot survive into an adapter closure and deadlock/re-entry is
/// structurally unavailable.
struct RouterIoResult {
    router: Router,
    succeeded: bool,
}
impl RouterIoResult {
    fn succeeded(&self) -> bool {
        self.succeeded
    }
    fn into_router(self) -> Router {
        self.router
    }
}

/// A consumed-router v3 Start operation.  Its `perform` is deliberately the
/// only production route to call a Start adapter.
struct AsioStartCoordinator<'a> {
    router: Router,
    operation: AsioStartOperation<AsioStartAdapter<'a>>,
}
impl AsioStartCoordinator<'_> {
    fn perform(mut self) -> RouterIoResult {
        let succeeded = match self.operation.perform() {
            AsioStartIo::Started(receipt) => self.router.complete_start(receipt).is_ok(),
            AsioStartIo::Failed(_, failure) | AsioStartIo::Panicked(_, failure) => {
                let _ = self.router.start_failed(failure);
                false
            }
        };
        if !succeeded && self.router.snapshot.state != State::Fault {
            self.router
                .coordinator_fault("asio_start_coordinator_mismatch");
        }
        RouterIoResult {
            router: self.router,
            succeeded,
        }
    }
}

/// A consumed-router v3 Stop/Close operation.  It carries the exact active
/// session lease across I/O and returns Locked only after a real drain receipt.
struct AsioStopCoordinator<'a> {
    router: Router,
    operation: StopOperation<AsioStopAdapter<'a>>,
}
impl AsioStopCoordinator<'_> {
    fn perform(mut self) -> RouterIoResult {
        let succeeded = match self.operation.perform() {
            StopIo::Drained(receipt) => self.router.complete_stop(receipt).is_ok(),
            StopIo::Failed(_, failure) | StopIo::Panicked(_, failure) => {
                let _ = self.router.stop_failed(failure);
                false
            }
        };
        if !succeeded && self.router.snapshot.state != State::Fault {
            self.router
                .coordinator_fault("asio_stop_coordinator_mismatch");
        }
        RouterIoResult {
            router: self.router,
            succeeded,
        }
    }
}

/// A consumed-router driver/profile revalidation.  It is only constructible
/// from Locked; the returned Router is either AsioReady or remains Locked with
/// a strict failure diagnostic.
struct AsioRevalidationCoordinator<'a> {
    router: Router,
    operation: AsioRevalidationOperation<AsioRevalidationAdapter<'a>>,
}
impl AsioRevalidationCoordinator<'_> {
    fn perform(mut self) -> RouterIoResult {
        let succeeded = match self.operation.perform() {
            AsioRevalidationIo::Validated(receipt) => {
                self.router.complete_revalidation(receipt).is_ok()
            }
            AsioRevalidationIo::Failed(_, failure) | AsioRevalidationIo::Panicked(_, failure) => {
                let _ = self.router.revalidation_failed(failure);
                false
            }
        };
        if !succeeded && !matches!(self.router.snapshot.state, State::Locked | State::Fault) {
            self.router
                .coordinator_fault("asio_revalidation_coordinator_mismatch");
        }
        RouterIoResult {
            router: self.router,
            succeeded,
        }
    }
}

/// Opaque application-held proof that a normal route was published.  It owns
/// the concrete resource; there is no dereference/into-inner escape hatch.
struct NormalRouteHandle<T, RetireFn> {
    owned: Owned<NormalRouteResource<T, RetireFn>>,
}

/// The same exact resource after a failed stop/join.  Only this handle may
/// request one fresh router retirement permit; it cannot be discarded into a
/// diagnostic-only error path.
struct NormalRetirementHandle<T, RetireFn> {
    owned: Owned<NormalRouteResource<T, RetireFn>>,
}

/// A consumed-router normal publication.  It is the only production surface
/// that can call `Publication::perform_io`, so normal Rodio construction also
/// happens after Router has left its mutex slot.
struct NormalPublicationCoordinator<T, Start, Recover, RetireFn> {
    router: Router,
    publication: Publication<NormalRouteFactory<T, Start, Recover, RetireFn>>,
}

struct NormalPublicationResult<T, RetireFn> {
    router: Router,
    published: Option<NormalRouteHandle<T, RetireFn>>,
    retry: Option<NormalRetirementHandle<T, RetireFn>>,
    succeeded: bool,
}
impl<T, RetireFn> NormalPublicationResult<T, RetireFn> {
    fn succeeded(&self) -> bool {
        self.succeeded
    }
    fn into_parts(
        self,
    ) -> (
        Router,
        Option<NormalRouteHandle<T, RetireFn>>,
        Option<NormalRetirementHandle<T, RetireFn>>,
    ) {
        (self.router, self.published, self.retry)
    }
}

struct NormalRetirementCoordinator<T, RetireFn> {
    router: Router,
    retire: Retire<NormalRouteResource<T, RetireFn>>,
}
struct NormalRetirementResult<T, RetireFn> {
    router: Router,
    retry: Option<NormalRetirementHandle<T, RetireFn>>,
    succeeded: bool,
}
impl<T, RetireFn> NormalRetirementResult<T, RetireFn> {
    fn succeeded(&self) -> bool {
        self.succeeded
    }
    fn into_parts(self) -> (Router, Option<NormalRetirementHandle<T, RetireFn>>) {
        (self.router, self.retry)
    }
}

impl<T, Start, Recover, RetireFn> NormalPublicationCoordinator<T, Start, Recover, RetireFn>
where
    Start: FnOnce() -> NormalRouteStart<T>,
    Recover: FnOnce() -> NormalRouteStart<T>,
    RetireFn: FnMut(&mut T) -> Result<(), BridgeDiagnostic>,
{
    fn perform(mut self) -> NormalPublicationResult<T, RetireFn> {
        match self.publication.perform_io() {
            FactoryIo::Prepared(prepared) => match self.router.commit(prepared) {
                PublicationCommit::Published(owned) => NormalPublicationResult {
                    router: self.router,
                    published: Some(NormalRouteHandle { owned }),
                    retry: None,
                    succeeded: true,
                },
                PublicationCommit::MustRetire(retire) => {
                    let (retry, succeeded) = retire_normal_resource(&mut self.router, retire);
                    NormalPublicationResult {
                        router: self.router,
                        published: None,
                        retry,
                        succeeded,
                    }
                }
                PublicationCommit::Rejected(_prepared, _error) => {
                    self.router
                        .coordinator_fault("normal_publication_coordinator_mismatch");
                    NormalPublicationResult {
                        router: self.router,
                        published: None,
                        retry: None,
                        succeeded: false,
                    }
                }
            },
            FactoryIo::Failed(failure) => {
                let _ = self.router.publication_failed(failure);
                if self.router.snapshot.state != State::Fault {
                    self.router
                        .coordinator_fault("normal_publication_failure_mismatch");
                }
                NormalPublicationResult {
                    router: self.router,
                    published: None,
                    retry: None,
                    succeeded: false,
                }
            }
            FactoryIo::Panicked(handle) => {
                let _ = self.router.publication_panicked(&handle);
                let recovery = match handle.begin_recovery(&mut self.router) {
                    Ok(recovery) => recovery,
                    Err(_) => {
                        self.router
                            .coordinator_fault("normal_panic_recovery_admission_mismatch");
                        return NormalPublicationResult {
                            router: self.router,
                            published: None,
                            retry: None,
                            succeeded: false,
                        };
                    }
                };
                match recovery.perform() {
                    PanicRecoveryIo::Recovered(retire) => {
                        let (retry, succeeded) = retire_normal_resource(&mut self.router, retire);
                        NormalPublicationResult {
                            router: self.router,
                            published: None,
                            retry,
                            succeeded,
                        }
                    }
                    PanicRecoveryIo::RecoveredWithStartupFailure(retire, failure) => {
                        let _ = self.router.recovery_startup_failed(failure);
                        let (retry, succeeded) = retire_normal_resource(&mut self.router, retire);
                        NormalPublicationResult {
                            router: self.router,
                            published: None,
                            retry,
                            succeeded,
                        }
                    }
                    PanicRecoveryIo::Failed(_, failure) | PanicRecoveryIo::Panicked(_, failure) => {
                        let _ = self.router.retire_failed(failure);
                        NormalPublicationResult {
                            router: self.router,
                            published: None,
                            retry: None,
                            succeeded: false,
                        }
                    }
                }
            }
        }
    }
}

impl<T, RetireFn> NormalRouteHandle<T, RetireFn>
where
    RetireFn: FnMut(&mut T) -> Result<(), BridgeDiagnostic>,
{
    /// Consumes both router and published resource before normal-device I/O.
    fn into_retirement(
        self,
        mut router: Router,
    ) -> Result<NormalRetirementCoordinator<T, RetireFn>, (Router, Self, Error)> {
        let retire = match self.owned.begin_retire(&mut router) {
            Ok(retire) => retire,
            Err((owned, error)) => {
                return Err((router, Self { owned }, error));
            }
        };
        Ok(NormalRetirementCoordinator { router, retire })
    }
}

impl<T, RetireFn> NormalRetirementHandle<T, RetireFn>
where
    RetireFn: FnMut(&mut T) -> Result<(), BridgeDiagnostic>,
{
    fn into_retry(
        self,
        mut router: Router,
    ) -> Result<NormalRetirementCoordinator<T, RetireFn>, (Router, Self, Error)> {
        let retire = match self.owned.begin_retire(&mut router) {
            Ok(retire) => retire,
            Err((owned, error)) => {
                return Err((router, Self { owned }, error));
            }
        };
        Ok(NormalRetirementCoordinator { router, retire })
    }
}

impl<T, RetireFn> NormalRetirementCoordinator<T, RetireFn>
where
    RetireFn: FnMut(&mut T) -> Result<(), BridgeDiagnostic>,
{
    fn perform(mut self) -> NormalRetirementResult<T, RetireFn> {
        let (retry, succeeded) = retire_normal_resource(&mut self.router, self.retire);
        NormalRetirementResult {
            router: self.router,
            retry,
            succeeded,
        }
    }
}

fn retire_normal_resource<T, RetireFn>(
    router: &mut Router,
    retire: Retire<NormalRouteResource<T, RetireFn>>,
) -> (Option<NormalRetirementHandle<T, RetireFn>>, bool)
where
    RetireFn: FnMut(&mut T) -> Result<(), BridgeDiagnostic>,
{
    match retire.perform() {
        RetirementIo::Joined(receipt) => (None, router.complete_retire(receipt).is_ok()),
        RetirementIo::Failed(owned, failure) | RetirementIo::Panicked(owned, failure) => {
            let _ = router.retire_failed(failure);
            (Some(NormalRetirementHandle { owned }), false)
        }
    }
}

struct Router {
    _lease: Option<ProcessLease>,
    snapshot: Snapshot,
    next_gen: u64,
    next_id: u64,
    next_op: u64,
    next_session: u64,
    routes: BTreeMap<u64, Record>,
}
struct ProcessLease;
impl Router {
    fn acquire_process_owner() -> Result<Self, Error> {
        if PROCESS_OWNER
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            Err(Error::ProcessOwnerTaken)
        } else {
            Self::new(Some(ProcessLease))
        }
    }
    #[cfg(test)]
    pub(crate) fn test_router() -> Self {
        Self::new(None).expect("test router identity counter must not exhaust")
    }
    fn new(lease: Option<ProcessLease>) -> Result<Self, Error> {
        let instance = Self::next_instance(&NEXT_INSTANCE)?;
        Ok(Self {
            _lease: lease,
            snapshot: Snapshot {
                state: State::Normal,
                instance,
                generation: 1,
                operation: None,
                session: None,
                remaining_drains: 0,
                reason: Reason::Normal,
                diagnostic: internal(Reason::Normal, "normal_selected", "Normal output selected."),
            },
            next_gen: 2,
            next_id: 1,
            next_op: 1,
            next_session: 1,
            routes: BTreeMap::new(),
        })
    }
    fn snapshot(&self) -> Snapshot {
        self.snapshot.clone()
    }
    /// Consumes the Router before a normal-output constructor is called.
    ///
    /// This is the sole app-facing normal route entry point.  The returned
    /// coordinator owns both the router and the factory, so a caller cannot
    /// keep a `MutexGuard<Router>` alive while Rodio/platform construction or
    /// later resource retirement executes.
    fn into_normal_publication<T, Start, Recover, RetireFn>(
        mut self,
        route: Route,
        factory: NormalRouteFactory<T, Start, Recover, RetireFn>,
    ) -> Result<NormalPublicationCoordinator<T, Start, Recover, RetireFn>, (Self, Error)>
    where
        Start: FnOnce() -> NormalRouteStart<T>,
        Recover: FnOnce() -> NormalRouteStart<T>,
        RetireFn: FnMut(&mut T) -> Result<(), BridgeDiagnostic>,
    {
        let publication = match self.begin_publication(route, factory) {
            Ok(publication) => publication,
            Err(error) => return Err((self, error)),
        };
        Ok(NormalPublicationCoordinator {
            router: self,
            publication,
        })
    }

    fn begin_publication<F: OutputFactory>(
        &mut self,
        route: Route,
        factory: F,
    ) -> Result<Publication<F>, Error> {
        if self.snapshot.state != State::Normal {
            return Err(Error::Blocked(self.snapshot.state));
        }
        let t = RouteTicket {
            instance: self.snapshot.instance,
            id: self.id()?,
            generation: self.snapshot.generation,
            route,
        };
        let factory_nonce = self.op()?;
        self.routes.insert(
            t.id,
            Record {
                ticket: t,
                state: RouteState::Publishing,
                retire: None,
                factory_nonce,
            },
        );
        Ok(Publication {
            ticket: t,
            capability: FactoryStartCapability {
                ticket: t,
                factory_nonce,
            },
            factory,
        })
    }
    fn commit<R: OutputResource>(&mut self, p: Prepared<R>) -> PublicationCommit<R> {
        if p.ticket.instance != self.snapshot.instance {
            return PublicationCommit::Rejected(p, Error::ForeignTicket);
        }
        let Some(r) = self.routes.get(&p.ticket.id).copied() else {
            return PublicationCommit::Rejected(p, Error::PublicationRejected);
        };
        if r.ticket != p.ticket {
            return PublicationCommit::Rejected(p, Error::PublicationRejected);
        }
        if r.state == RouteState::Publishing && self.snapshot.state == State::Normal {
            if let Some(diagnostic) = p.resource.startup_failure().cloned() {
                // A closure-adapter error still owns a concrete OS resource.
                // Do not delete its route record as the old diagnostic-only
                // path did: latch the exact failure, retain the resource, and
                // issue the sole router-governed retirement operation.
                self.routes.get_mut(&p.ticket.id).unwrap().state = RouteState::RetireRequested;
                // `normal_fault` itself converts invalid diagnostics into a
                // visible InvalidDiagnostic fault.  The resource must retire
                // in either case, so its validation error is intentionally
                // not allowed to strand or discard the handle.
                let _ = self.normal_fault(Reason::PublicationFailed, diagnostic, true);
                let o = self
                    .begin_retire(p.ticket)
                    .expect("startup-failed route must retire while faulted");
                return PublicationCommit::MustRetire(Retire {
                    ticket: p.ticket,
                    operation: o,
                    resource: p.resource,
                });
            }
        }
        if r.state == RouteState::Publishing && self.snapshot.state == State::Normal {
            self.routes.get_mut(&p.ticket.id).unwrap().state = RouteState::Owned;
            PublicationCommit::Published(Owned {
                ticket: p.ticket,
                resource: p.resource,
            })
        } else if r.state == RouteState::RetireRequested {
            let o = self
                .begin_retire(p.ticket)
                .expect("late prepare route must retire");
            PublicationCommit::MustRetire(Retire {
                ticket: p.ticket,
                operation: o,
                resource: p.resource,
            })
        } else {
            PublicationCommit::Rejected(p, Error::PublicationRejected)
        }
    }
    fn publication_failed(&mut self, f: PublicationFailure) -> Result<(), Error> {
        if f.ticket.instance != self.snapshot.instance {
            return Err(Error::ForeignTicket);
        }
        match self.routes.get(&f.ticket.id) {
            Some(record)
                if record.ticket == f.ticket
                    && matches!(
                        record.state,
                        RouteState::Publishing | RouteState::RetireRequested
                    ) => {}
            _ => return Err(Error::PublicationRejected),
        }
        self.routes.remove(&f.ticket.id);
        // A failed factory returned no resource, but any previously-owned
        // routes must remain tracked and retire; never clear them here.
        self.normal_fault(Reason::PublicationFailed, f.diagnostic, true)
    }
    fn publication_panicked<F: OutputFactory>(&mut self, h: &PanicHandle<F>) -> Result<(), Error> {
        self.check(h.ticket, RouteState::Publishing, Error::PublicationRejected)?;
        if h.capability.ticket != h.ticket
            || self.routes[&h.ticket.id].factory_nonce != h.capability.factory_nonce
        {
            return Err(Error::PublicationRejected);
        }
        self.routes.get_mut(&h.ticket.id).unwrap().state = RouteState::Panic;
        self.normal_fault(
            Reason::PublicationPanicked,
            internal(
                Reason::PublicationPanicked,
                "publication_panic",
                "Factory panicked.",
            ),
            true,
        )
    }
    /// Records a concrete resource returned alongside a recovery error before
    /// its already-issued retirement operation runs.  The ticket/operation
    /// pair prevents a stale recovery from changing a newer route, and the
    /// resource stays in `Retiring` until a real stop/join receipt arrives.
    fn recovery_startup_failed(&mut self, failure: RecoveryStartupFailure) -> Result<(), Error> {
        self.check(failure.ticket, RouteState::Retiring, Error::FailureMismatch)?;
        if self.routes[&failure.ticket.id].retire != Some(failure.operation) {
            return Err(Error::FailureMismatch);
        }
        self.normal_fault(Reason::PublicationFailed, failure.diagnostic, true)
    }
    fn complete_retire(&mut self, r: Receipt) -> Result<(), Error> {
        self.check(r.ticket, RouteState::Retiring, Error::ReceiptMismatch)?;
        if self.routes[&r.ticket.id].retire != Some(r.operation) {
            return Err(Error::ReceiptMismatch);
        }
        self.routes.remove(&r.ticket.id);
        self.after_drain();
        Ok(())
    }
    fn retire_failed(&mut self, f: RetirementFailure) -> Result<(), Error> {
        self.check(f.ticket, RouteState::Retiring, Error::FailureMismatch)?;
        if self.routes[&f.ticket.id].retire != Some(f.operation) {
            return Err(Error::FailureMismatch);
        }
        // The old operation is consumed by this failure.  The opaque owner
        // returned from `RetirementIo` may obtain exactly one fresh permit.
        let record = self.routes.get_mut(&f.ticket.id).expect("checked route");
        record.state = RouteState::RetireRequested;
        record.retire = None;
        self.normal_fault(
            if f.panic {
                Reason::RetirementPanicked
            } else {
                Reason::RetirementFailed
            },
            f.diagnostic,
            true,
        )
    }
    fn quiesce(&mut self) -> Result<QuiesceTicket, Error> {
        if self.snapshot.state != State::Normal {
            return Err(Error::Blocked(self.snapshot.state));
        }
        let t = QuiesceTicket {
            instance: self.snapshot.instance,
            generation: self.gen()?,
            operation: self.op()?,
        };
        for r in self.routes.values_mut() {
            if matches!(r.state, RouteState::Publishing | RouteState::Owned) {
                r.state = RouteState::RetireRequested
            }
        }
        self.set(
            State::Quiescing,
            t.generation,
            Some(t.operation),
            None,
            self.routes.len(),
            Reason::Quiescing,
            internal(
                Reason::Quiescing,
                "quiescing",
                "Waiting for route receipts.",
            ),
        );
        Ok(t)
    }

    /// Consumes a fully-quiesced Router into the only production ASIO Start
    /// coordinator.  The Router cannot remain inside a mutex guard while the
    /// closure performs driver I/O; `RouterIoResult::into_router` is the sole
    /// re-insertion path afterwards.
    fn into_admitted_start_with<'a>(
        mut self,
        perform_start: impl FnOnce() -> Result<(), BridgeDiagnostic> + 'a,
    ) -> Result<AsioStartCoordinator<'a>, (Self, Error)> {
        let Some(operation) = self.snapshot.operation else {
            return Err((self, Error::QuiesceMismatch));
        };
        let ticket = QuiesceTicket {
            instance: self.snapshot.instance,
            generation: self.snapshot.generation,
            operation,
        };
        let start = match self.admit_start(ticket) {
            Ok(start) => start,
            Err(error) => return Err((self, error)),
        };
        let operation = match self.begin_start_io(start, asio_start_adapter(perform_start)) {
            Ok(operation) => operation,
            Err(error) => return Err((self, error)),
        };
        Ok(AsioStartCoordinator {
            router: self,
            operation,
        })
    }

    /// Consumes an active/faulted Router for the only production Stop/Close
    /// I/O path.  It is intentionally impossible to get the underlying
    /// `StopOperation` without moving Router ownership here.
    fn into_stop_with<'a>(
        mut self,
        stop: impl FnOnce() -> Result<(), BridgeStopFailureDetail> + 'a,
    ) -> Result<AsioStopCoordinator<'a>, (Self, Error)> {
        let operation = match self.begin_stop(asio_stop_adapter(stop)) {
            Ok(operation) => operation,
            Err(error) => return Err((self, error)),
        };
        Ok(AsioStopCoordinator {
            router: self,
            operation,
        })
    }

    /// Starts strict driver/profile revalidation after an explicit successful
    /// Stop.  No live bridge/catalog call can be obtained from `Locked` except
    /// through this consumed-router coordinator.
    fn into_revalidation_with<'a>(
        mut self,
        revalidate: impl FnOnce() -> Result<(), BridgeDiagnostic> + 'a,
    ) -> Result<AsioRevalidationCoordinator<'a>, (Self, Error)> {
        let ticket = match self.begin_revalidation() {
            Ok(ticket) => ticket,
            Err(error) => return Err((self, error)),
        };
        let operation =
            match self.begin_revalidation_io(ticket, asio_revalidation_adapter(revalidate)) {
                Ok(operation) => operation,
                Err(error) => return Err((self, error)),
            };
        Ok(AsioRevalidationCoordinator {
            router: self,
            operation,
        })
    }

    /// Consumes a Ready Router into a validated Start.  It reconstructs the
    /// opaque ready receipt only from its own exact snapshot, so application
    /// code cannot synthesize/replay one.
    fn into_validated_start_with<'a>(
        mut self,
        perform_start: impl FnOnce() -> Result<(), BridgeDiagnostic> + 'a,
    ) -> Result<AsioStartCoordinator<'a>, (Self, Error)> {
        let Some(operation) = self.snapshot.operation else {
            return Err((self, Error::ReadyMismatch));
        };
        let ready = ReadyTicket {
            instance: self.snapshot.instance,
            generation: self.snapshot.generation,
            operation,
        };
        let start = match self.start_validated(ready) {
            Ok(start) => start,
            Err(error) => return Err((self, error)),
        };
        let operation = match self.begin_start_io(start, asio_start_adapter(perform_start)) {
            Ok(operation) => operation,
            Err(error) => return Err((self, error)),
        };
        Ok(AsioStartCoordinator {
            router: self,
            operation,
        })
    }
    fn admit_start(&mut self, t: QuiesceTicket) -> Result<StartTicket, Error> {
        if t.instance != self.snapshot.instance
            || self.snapshot.state != State::Quiescing
            || self.snapshot.generation != t.generation
            || self.snapshot.operation != Some(t.operation)
        {
            return Err(Error::QuiesceMismatch);
        }
        if !self.routes.is_empty() {
            return Err(Error::QuiesceIncomplete(self.routes.len()));
        }
        let s = StartTicket {
            instance: t.instance,
            generation: t.generation,
            operation: t.operation,
            session: self.session()?,
        };
        self.set(
            State::AsioStarting,
            s.generation,
            Some(s.operation),
            Some(s.session),
            0,
            Reason::AsioStarting,
            internal(
                Reason::AsioStarting,
                "asio_starting",
                "ASIO start admitted.",
            ),
        );
        Ok(s)
    }
    fn begin_start_io<B: AsioStartLifecycle>(
        &self,
        ticket: StartTicket,
        adapter: B,
    ) -> Result<AsioStartOperation<B>, Error> {
        self.asio(ticket, State::AsioStarting, Error::StartMismatch)?;
        Ok(AsioStartOperation {
            adapter,
            lease: AsioStartLease { ticket },
        })
    }
    fn complete_start(&mut self, receipt: AsioStartReceipt) -> Result<(), Error> {
        let ticket = receipt.ticket;
        self.asio(ticket, State::AsioStarting, Error::StartMismatch)?;
        self.set(
            State::AsioActive,
            ticket.generation,
            Some(ticket.operation),
            Some(ticket.session),
            0,
            Reason::AsioActive,
            internal(Reason::AsioActive, "asio_active", "ASIO active."),
        );
        Ok(())
    }
    fn start_failed(&mut self, failure: AsioStartFailure) -> Result<(), Error> {
        self.asio(failure.ticket, State::AsioStarting, Error::StartMismatch)?;
        self.asio_fault(
            failure.ticket,
            Reason::AsioStartFailed,
            failure.diagnostic,
            1,
        )
    }
    fn begin_stop<B: BridgeStop>(&mut self, b: B) -> Result<StopOperation<B>, Error> {
        if !matches!(
            self.snapshot.state,
            State::AsioStarting | State::AsioActive | State::Fault
        ) {
            return Err(Error::StopRejected);
        }
        let session = self.snapshot.session.ok_or(Error::StopRejected)?;
        let t = StartTicket {
            instance: self.snapshot.instance,
            generation: self.snapshot.generation,
            operation: self.op()?,
            session,
        };
        let pending = self.snapshot.remaining_drains.max(1);
        self.set(
            State::Quiescing,
            t.generation,
            Some(t.operation),
            Some(t.session),
            pending,
            Reason::StopPending,
            internal(Reason::StopPending, "asio_stop_pending", "Drain unproven."),
        );
        Ok(StopOperation {
            bridge: b,
            lease: BridgeStopLease { ticket: t },
        })
    }
    fn complete_stop(&mut self, r: DrainReceipt) -> Result<(), Error> {
        self.asio(r.ticket, State::Quiescing, Error::StopMismatch)?;
        self.set(
            State::Locked,
            r.ticket.generation,
            None,
            None,
            0,
            Reason::ExplicitSelection,
            internal(
                Reason::ExplicitSelection,
                "explicit_selection",
                "Explicit backend selection required.",
            ),
        );
        Ok(())
    }
    fn stop_failed(&mut self, f: StopFailure) -> Result<(), Error> {
        self.asio(f.ticket, State::Quiescing, Error::StopMismatch)?;
        let (reason, n) = match f.detail.stage {
            BridgeStopStage::Dispatch => (Reason::DispatchFailed, 1),
            BridgeStopStage::Drain { unresolved } => (Reason::DrainUnproven, unresolved.get()),
            BridgeStopStage::Close => (Reason::CloseFailed, 0),
        };
        self.asio_fault(
            f.ticket,
            reason,
            f.detail.diagnostic,
            self.snapshot
                .remaining_drains
                .max(n)
                .max(if reason == Reason::DispatchFailed {
                    1
                } else {
                    0
                }),
        )
    }
    fn begin_revalidation(&mut self) -> Result<ReadyTicket, Error> {
        if self.snapshot.state != State::Locked
            || self.snapshot.remaining_drains != 0
            || self.snapshot.operation.is_some()
        {
            return Err(Error::RevalidationRejected);
        }
        let ticket = ReadyTicket {
            instance: self.snapshot.instance,
            generation: self.snapshot.generation,
            operation: self.op()?,
        };
        self.set(
            State::Locked,
            ticket.generation,
            Some(ticket.operation),
            None,
            0,
            Reason::Revalidating,
            internal(
                Reason::Revalidating,
                "asio_revalidation_pending",
                "ASIO revalidation is pending.",
            ),
        );
        Ok(ticket)
    }
    fn begin_revalidation_io<B: AsioRevalidationLifecycle>(
        &self,
        t: ReadyTicket,
        adapter: B,
    ) -> Result<AsioRevalidationOperation<B>, Error> {
        if t.instance != self.snapshot.instance
            || self.snapshot.state != State::Locked
            || self.snapshot.generation != t.generation
            || self.snapshot.operation != Some(t.operation)
        {
            return Err(Error::RevalidationMismatch);
        }
        Ok(AsioRevalidationOperation {
            adapter,
            lease: AsioRevalidationLease { ticket: t },
        })
    }
    fn complete_revalidation(
        &mut self,
        receipt: AsioRevalidationReceipt,
    ) -> Result<ReadyTicket, Error> {
        let t = receipt.ticket;
        if t.instance != self.snapshot.instance
            || self.snapshot.state != State::Locked
            || self.snapshot.generation != t.generation
            || self.snapshot.operation != Some(t.operation)
        {
            return Err(Error::RevalidationMismatch);
        }
        let generation = self.gen()?;
        let ready = ReadyTicket { generation, ..t };
        self.set(
            State::AsioReady,
            generation,
            Some(ready.operation),
            None,
            0,
            Reason::Ready,
            internal(Reason::Ready, "asio_ready", "Explicit ASIO start required."),
        );
        Ok(ready)
    }
    fn revalidation_failed(&mut self, failure: AsioRevalidationFailure) -> Result<(), Error> {
        let t = failure.ticket;
        if t.instance != self.snapshot.instance
            || self.snapshot.state != State::Locked
            || self.snapshot.generation != t.generation
            || self.snapshot.operation != Some(t.operation)
        {
            return Err(Error::RevalidationMismatch);
        }
        if failure.diagnostic.validate().is_err() {
            self.snapshot.operation = None;
            self.snapshot.reason = Reason::InvalidDiagnostic;
            self.snapshot.diagnostic = internal(
                Reason::InvalidDiagnostic,
                "invalid_bridge_diagnostic",
                "Bridge supplied an invalid diagnostic.",
            );
            return Err(Error::InvalidDiagnostic);
        }
        self.snapshot.operation = None;
        self.snapshot.reason = Reason::RevalidationFailed;
        self.snapshot.diagnostic = failure.diagnostic;
        Ok(())
    }
    fn start_validated(&mut self, t: ReadyTicket) -> Result<StartTicket, Error> {
        if t.instance != self.snapshot.instance
            || self.snapshot.state != State::AsioReady
            || self.snapshot.generation != t.generation
            || self.snapshot.operation != Some(t.operation)
        {
            return Err(Error::ReadyMismatch);
        }
        let s = StartTicket {
            instance: t.instance,
            generation: t.generation,
            operation: self.op()?,
            session: self.session()?,
        };
        self.set(
            State::AsioStarting,
            s.generation,
            Some(s.operation),
            Some(s.session),
            0,
            Reason::AsioStarting,
            internal(
                Reason::AsioStarting,
                "asio_starting",
                "Validated start admitted.",
            ),
        );
        Ok(s)
    }
    fn cancel_ready(&mut self, t: ReadyTicket) -> Result<(), Error> {
        if t.instance != self.snapshot.instance
            || self.snapshot.state != State::AsioReady
            || self.snapshot.generation != t.generation
            || self.snapshot.operation != Some(t.operation)
        {
            return Err(Error::ReadyMismatch);
        }
        self.set(
            State::Locked,
            t.generation,
            None,
            None,
            0,
            Reason::ExplicitSelection,
            internal(
                Reason::ExplicitSelection,
                "explicit_selection",
                "Validated ASIO start cancelled; explicitly select a backend.",
            ),
        );
        Ok(())
    }
    fn select_normal(&mut self) -> Result<(), Error> {
        if self.snapshot.state != State::Locked
            || self.snapshot.remaining_drains != 0
            || self.snapshot.operation.is_some()
            || self.snapshot.session.is_some()
        {
            return Err(Error::NormalSelectionRejected);
        }
        let generation = self.gen()?;
        self.set(
            State::Normal,
            generation,
            None,
            None,
            0,
            Reason::Normal,
            internal(
                Reason::Normal,
                "normal_selected",
                "Normal output explicitly selected.",
            ),
        );
        Ok(())
    }
    fn begin_retire(&mut self, t: RouteTicket) -> Result<u64, Error> {
        if t.instance != self.snapshot.instance {
            return Err(Error::ForeignTicket);
        }
        let r = self
            .routes
            .get(&t.id)
            .copied()
            .ok_or(Error::RetirementRejected)?;
        let allowed = match self.snapshot.state {
            State::Normal => r.state == RouteState::Owned,
            State::Quiescing | State::Fault => matches!(
                r.state,
                RouteState::Owned | RouteState::RetireRequested | RouteState::Panic
            ),
            _ => false,
        };
        if r.ticket != t || !allowed {
            return Err(Error::RetirementRejected);
        }
        let o = self.op()?;
        let r = self.routes.get_mut(&t.id).unwrap();
        r.state = RouteState::Retiring;
        r.retire = Some(o);
        Ok(o)
    }
    fn check(&self, t: RouteTicket, s: RouteState, e: Error) -> Result<(), Error> {
        if t.instance != self.snapshot.instance {
            return Err(Error::ForeignTicket);
        }
        match self.routes.get(&t.id) {
            Some(r) if r.ticket == t && r.state == s => Ok(()),
            _ => Err(e),
        }
    }
    fn check_panic_capability(
        &self,
        ticket: RouteTicket,
        capability: &FactoryPanicCapability,
    ) -> Result<(), Error> {
        self.check(ticket, RouteState::Panic, Error::PublicationRejected)?;
        if capability.ticket != ticket
            || self.routes[&ticket.id].factory_nonce != capability.factory_nonce
        {
            return Err(Error::PublicationRejected);
        }
        Ok(())
    }
    fn normal_fault(
        &mut self,
        reason: Reason,
        d: BridgeDiagnostic,
        keep: bool,
    ) -> Result<(), Error> {
        if d.validate().is_err() {
            self.set(
                State::Fault,
                self.snapshot.generation,
                None,
                None,
                self.routes.len().max(1),
                Reason::InvalidDiagnostic,
                internal(
                    Reason::InvalidDiagnostic,
                    "invalid_bridge_diagnostic",
                    "Bridge supplied an invalid diagnostic.",
                ),
            );
            return Err(Error::InvalidDiagnostic);
        }
        if !keep {
            self.routes.clear()
        } else {
            for r in self.routes.values_mut() {
                if matches!(r.state, RouteState::Publishing | RouteState::Owned) {
                    r.state = RouteState::RetireRequested
                }
            }
        }
        self.set(
            State::Fault,
            self.snapshot.generation,
            None,
            None,
            self.routes.len().max(1),
            reason,
            d,
        );
        Ok(())
    }
    fn after_drain(&mut self) {
        if self.routes.is_empty() && self.snapshot.state == State::Fault {
            self.set(
                State::Locked,
                self.snapshot.generation,
                None,
                None,
                0,
                Reason::ExplicitSelection,
                internal(
                    Reason::ExplicitSelection,
                    "explicit_selection",
                    "All normal routes retired.",
                ),
            )
        } else if self.snapshot.state == State::Quiescing {
            self.snapshot.remaining_drains = self.routes.len()
        }
    }
    fn asio(&self, t: StartTicket, s: State, e: Error) -> Result<(), Error> {
        if t.instance == self.snapshot.instance
            && self.snapshot.state == s
            && self.snapshot.generation == t.generation
            && self.snapshot.operation == Some(t.operation)
            && self.snapshot.session == Some(t.session)
        {
            Ok(())
        } else {
            Err(e)
        }
    }
    fn asio_fault(
        &mut self,
        t: StartTicket,
        reason: Reason,
        d: BridgeDiagnostic,
        n: usize,
    ) -> Result<(), Error> {
        if d.validate().is_err() {
            self.set(
                State::Fault,
                t.generation,
                Some(t.operation),
                Some(t.session),
                n.max(self.snapshot.remaining_drains).max(1),
                Reason::InvalidDiagnostic,
                internal(
                    Reason::InvalidDiagnostic,
                    "invalid_bridge_diagnostic",
                    "Bridge supplied an invalid diagnostic.",
                ),
            );
            return Err(Error::InvalidDiagnostic);
        }
        self.set(
            State::Fault,
            t.generation,
            Some(t.operation),
            Some(t.session),
            n.max(self.snapshot.remaining_drains),
            reason,
            d,
        );
        Ok(())
    }
    /// A completion mismatch can only indicate a programming/invariant fault
    /// inside a consumed-router coordinator.  It must never panic, reopen a
    /// normal output, or leave a usable bridge lease behind.
    fn coordinator_fault(&mut self, code: &str) {
        self.set(
            State::Fault,
            self.snapshot.generation,
            self.snapshot.operation,
            self.snapshot.session,
            self.snapshot.remaining_drains.max(1),
            Reason::InvalidDiagnostic,
            internal(
                Reason::InvalidDiagnostic,
                code,
                "Router coordinator completion did not match its admitted lease.",
            ),
        );
    }
    fn set(
        &mut self,
        state: State,
        generation: u64,
        operation: Option<u64>,
        session: Option<u64>,
        remaining_drains: usize,
        reason: Reason,
        diagnostic: BridgeDiagnostic,
    ) {
        self.snapshot = Snapshot {
            state,
            instance: self.snapshot.instance,
            generation,
            operation,
            session,
            remaining_drains,
            reason,
            diagnostic,
        }
    }
    fn next_instance(counter: &AtomicU64) -> Result<u64, Error> {
        let mut current = counter.load(Ordering::Acquire);
        loop {
            if current == u64::MAX {
                return Err(Error::IdentityExhausted);
            }
            match counter.compare_exchange_weak(
                current,
                current + 1,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => return Ok(current),
                Err(observed) => current = observed,
            }
        }
    }
    fn next_local(counter: &mut u64) -> Result<u64, Error> {
        if *counter == u64::MAX {
            return Err(Error::IdentityExhausted);
        }
        let issued = *counter;
        *counter += 1;
        Ok(issued)
    }
    fn gen(&mut self) -> Result<u64, Error> {
        Self::next_local(&mut self.next_gen)
    }
    fn id(&mut self) -> Result<u64, Error> {
        Self::next_local(&mut self.next_id)
    }
    fn op(&mut self) -> Result<u64, Error> {
        Self::next_local(&mut self.next_op)
    }
    fn session(&mut self) -> Result<u64, Error> {
        Self::next_local(&mut self.next_session)
    }
}

/// The only production owner for the process-wide audio-output Router.
///
/// Its mutex is intentionally private.  Every method that can run driver or
/// normal-device I/O first removes the Router and drops the internal guard;
/// the matching task restores it only after the operation has crossed the
/// router's receipt/failure transition.  App code therefore cannot retain a
/// `MutexGuard<Router>` across a callback which might re-enter the output
/// control plane.
pub(crate) struct RouterSlot {
    inner: Mutex<Option<Router>>,
}

impl RouterSlot {
    pub(crate) fn acquire_process_owner() -> Result<Arc<Self>, Error> {
        Ok(Arc::new(Self {
            inner: Mutex::new(Some(Router::acquire_process_owner()?)),
        }))
    }

    #[cfg(test)]
    pub(crate) fn test_slot() -> Arc<Self> {
        Arc::new(Self {
            inner: Mutex::new(Some(Router::test_router())),
        })
    }

    pub(crate) fn snapshot(&self) -> Result<Snapshot, Error> {
        let guard = self.lock();
        guard
            .as_ref()
            .map(Router::snapshot)
            .ok_or(Error::RouterOperationInProgress)
    }

    /// Normal -> Quiescing is state-only.  It never invokes a platform
    /// closure while the slot is locked.
    pub(crate) fn quiesce(&self) -> Result<Snapshot, Error> {
        let mut guard = self.lock();
        let router = guard.as_mut().ok_or(Error::RouterOperationInProgress)?;
        router.quiesce()?;
        Ok(router.snapshot())
    }

    /// The explicit Locked -> Normal operator action.  This cannot become an
    /// implicit fallback because it remains a state-only, separately invoked
    /// command.
    pub(crate) fn select_normal(&self) -> Result<Snapshot, Error> {
        let mut guard = self.lock();
        let router = guard.as_mut().ok_or(Error::RouterOperationInProgress)?;
        router.select_normal()?;
        Ok(router.snapshot())
    }

    /// Cancels the current Ready admission using a ticket reconstructed only
    /// from the slot's own exact snapshot.  The opaque ReadyTicket never
    /// leaves this module.
    pub(crate) fn cancel_ready(&self) -> Result<Snapshot, Error> {
        let mut guard = self.lock();
        let router = guard.as_mut().ok_or(Error::RouterOperationInProgress)?;
        let operation = router.snapshot.operation.ok_or(Error::ReadyMismatch)?;
        let ticket = ReadyTicket {
            instance: router.snapshot.instance,
            generation: router.snapshot.generation,
            operation,
        };
        router.cancel_ready(ticket)?;
        Ok(router.snapshot())
    }

    pub(crate) fn into_admitted_start_with<'operation>(
        self: &Arc<Self>,
        perform_start: impl FnOnce() -> Result<(), BridgeDiagnostic> + 'operation,
    ) -> Result<SlotAsioStartTask<'operation>, Error> {
        let router = self.take_for_io()?;
        match router.into_admitted_start_with(perform_start) {
            Ok(operation) => Ok(SlotAsioStartTask {
                slot: Arc::clone(self),
                operation,
            }),
            Err((router, error)) => {
                self.restore_after_io(router);
                Err(error)
            }
        }
    }

    pub(crate) fn into_validated_start_with<'operation>(
        self: &Arc<Self>,
        perform_start: impl FnOnce() -> Result<(), BridgeDiagnostic> + 'operation,
    ) -> Result<SlotAsioStartTask<'operation>, Error> {
        let router = self.take_for_io()?;
        match router.into_validated_start_with(perform_start) {
            Ok(operation) => Ok(SlotAsioStartTask {
                slot: Arc::clone(self),
                operation,
            }),
            Err((router, error)) => {
                self.restore_after_io(router);
                Err(error)
            }
        }
    }

    pub(crate) fn into_stop_with<'operation>(
        self: &Arc<Self>,
        stop: impl FnOnce() -> Result<(), BridgeStopFailureDetail> + 'operation,
    ) -> Result<SlotAsioStopTask<'operation>, Error> {
        let router = self.take_for_io()?;
        match router.into_stop_with(stop) {
            Ok(operation) => Ok(SlotAsioStopTask {
                slot: Arc::clone(self),
                operation,
            }),
            Err((router, error)) => {
                self.restore_after_io(router);
                Err(error)
            }
        }
    }

    pub(crate) fn into_revalidation_with<'operation>(
        self: &Arc<Self>,
        revalidate: impl FnOnce() -> Result<(), BridgeDiagnostic> + 'operation,
    ) -> Result<SlotAsioRevalidationTask<'operation>, Error> {
        let router = self.take_for_io()?;
        match router.into_revalidation_with(revalidate) {
            Ok(operation) => Ok(SlotAsioRevalidationTask {
                slot: Arc::clone(self),
                operation,
            }),
            Err((router, error)) => {
                self.restore_after_io(router);
                Err(error)
            }
        }
    }

    /// The sole app-facing normal route publication admission.  As with ASIO,
    /// the factory runs only after Router ownership has left this private
    /// slot.  The returned lease is the only route-retirement entry point.
    pub(crate) fn into_normal_publication<T, Start, Recover, RetireFn>(
        self: &Arc<Self>,
        route: Route,
        factory: NormalRouteFactory<T, Start, Recover, RetireFn>,
    ) -> Result<SlotNormalPublicationTask<T, Start, Recover, RetireFn>, Error>
    where
        Start: FnOnce() -> NormalRouteStart<T>,
        Recover: FnOnce() -> NormalRouteStart<T>,
        RetireFn: FnMut(&mut T) -> Result<(), BridgeDiagnostic>,
    {
        let router = self.take_for_io()?;
        match router.into_normal_publication(route, factory) {
            Ok(operation) => Ok(SlotNormalPublicationTask {
                slot: Arc::clone(self),
                operation,
            }),
            Err((router, error)) => {
                self.restore_after_io(router);
                Err(error)
            }
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Option<Router>> {
        // A prior panic has already been caught and converted to a router
        // fault before this lock is reacquired.  Keep the Router present so
        // the operator can observe that fault instead of silently releasing
        // process ownership because a reporting caller poisoned the mutex.
        self.inner
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
    }

    fn take_for_io(&self) -> Result<Router, Error> {
        let mut guard = self.lock();
        guard.take().ok_or(Error::RouterOperationInProgress)
    }

    fn restore_after_io(&self, router: Router) {
        let mut guard = self.lock();
        if guard.is_some() {
            // This cannot arise through the public API: Router is private and
            // every Slot task owns the only removed value.  Preserve the
            // process lease rather than dropping it if an internal invariant
            // is ever violated; the next operation remains permanently
            // fail-closed instead of reopening a second output owner.
            std::mem::forget(router);
            panic!("audio-output router slot restore invariant violated");
        }
        *guard = Some(router);
    }
}

/// Result visible to app code after a closure-only ASIO task.  It carries no
/// router, receipt, adapter, or lease; only an audited state snapshot.
pub(crate) struct SlotIoResult {
    succeeded: bool,
    snapshot: Snapshot,
}
impl SlotIoResult {
    pub(crate) fn succeeded(&self) -> bool {
        self.succeeded
    }

    pub(crate) fn snapshot(&self) -> &Snapshot {
        &self.snapshot
    }
}

pub(crate) struct SlotAsioStartTask<'operation> {
    slot: Arc<RouterSlot>,
    operation: AsioStartCoordinator<'operation>,
}
impl SlotAsioStartTask<'_> {
    pub(crate) fn perform(self) -> SlotIoResult {
        slot_io_result(&self.slot, self.operation.perform())
    }
}

pub(crate) struct SlotAsioStopTask<'operation> {
    slot: Arc<RouterSlot>,
    operation: AsioStopCoordinator<'operation>,
}
impl SlotAsioStopTask<'_> {
    pub(crate) fn perform(self) -> SlotIoResult {
        slot_io_result(&self.slot, self.operation.perform())
    }
}

pub(crate) struct SlotAsioRevalidationTask<'operation> {
    slot: Arc<RouterSlot>,
    operation: AsioRevalidationCoordinator<'operation>,
}
impl SlotAsioRevalidationTask<'_> {
    pub(crate) fn perform(self) -> SlotIoResult {
        slot_io_result(&self.slot, self.operation.perform())
    }
}

fn slot_io_result(slot: &RouterSlot, result: RouterIoResult) -> SlotIoResult {
    let succeeded = result.succeeded();
    let router = result.into_router();
    let snapshot = router.snapshot();
    slot.restore_after_io(router);
    SlotIoResult {
        succeeded,
        snapshot,
    }
}

/// Router-slot-bound normal route proof.  It cannot expose the concrete
/// resource and cannot begin I/O until it has again removed Router from the
/// slot, so a caller has no guard to hold across retirement.
pub(crate) struct SlotNormalRouteLease<T, RetireFn> {
    slot: Arc<RouterSlot>,
    handle: NormalRouteHandle<T, RetireFn>,
}

pub(crate) struct SlotNormalRetirementLease<T, RetireFn> {
    slot: Arc<RouterSlot>,
    handle: NormalRetirementHandle<T, RetireFn>,
}

pub(crate) struct SlotNormalPublicationTask<T, Start, Recover, RetireFn> {
    slot: Arc<RouterSlot>,
    operation: NormalPublicationCoordinator<T, Start, Recover, RetireFn>,
}

pub(crate) struct SlotNormalPublicationResult<T, RetireFn> {
    succeeded: bool,
    snapshot: Snapshot,
    published: Option<SlotNormalRouteLease<T, RetireFn>>,
    retry: Option<SlotNormalRetirementLease<T, RetireFn>>,
}
impl<T, RetireFn> SlotNormalPublicationResult<T, RetireFn> {
    pub(crate) fn succeeded(&self) -> bool {
        self.succeeded
    }

    pub(crate) fn snapshot(&self) -> &Snapshot {
        &self.snapshot
    }

    pub(crate) fn into_handles(
        self,
    ) -> (
        Option<SlotNormalRouteLease<T, RetireFn>>,
        Option<SlotNormalRetirementLease<T, RetireFn>>,
    ) {
        (self.published, self.retry)
    }
}

impl<T, Start, Recover, RetireFn> SlotNormalPublicationTask<T, Start, Recover, RetireFn>
where
    Start: FnOnce() -> NormalRouteStart<T>,
    Recover: FnOnce() -> NormalRouteStart<T>,
    RetireFn: FnMut(&mut T) -> Result<(), BridgeDiagnostic>,
{
    pub(crate) fn perform(self) -> SlotNormalPublicationResult<T, RetireFn> {
        let SlotNormalPublicationTask { slot, operation } = self;
        let result = operation.perform();
        let succeeded = result.succeeded();
        let (router, published, retry) = result.into_parts();
        let snapshot = router.snapshot();
        slot.restore_after_io(router);
        SlotNormalPublicationResult {
            succeeded,
            snapshot,
            published: published.map(|handle| SlotNormalRouteLease {
                slot: Arc::clone(&slot),
                handle,
            }),
            retry: retry.map(|handle| SlotNormalRetirementLease { slot, handle }),
        }
    }
}

pub(crate) struct SlotNormalRetirementTask<T, RetireFn> {
    slot: Arc<RouterSlot>,
    operation: NormalRetirementCoordinator<T, RetireFn>,
}

pub(crate) struct SlotNormalRetirementResult<T, RetireFn> {
    succeeded: bool,
    snapshot: Snapshot,
    retry: Option<SlotNormalRetirementLease<T, RetireFn>>,
}
impl<T, RetireFn> SlotNormalRetirementResult<T, RetireFn> {
    pub(crate) fn succeeded(&self) -> bool {
        self.succeeded
    }

    pub(crate) fn snapshot(&self) -> &Snapshot {
        &self.snapshot
    }

    pub(crate) fn into_retry(self) -> Option<SlotNormalRetirementLease<T, RetireFn>> {
        self.retry
    }
}

impl<T, RetireFn> SlotNormalRouteLease<T, RetireFn>
where
    RetireFn: FnMut(&mut T) -> Result<(), BridgeDiagnostic>,
{
    /// Runs a bounded read-only projection while the concrete route resource
    /// stays sealed in this lease.  This is intended for values such as a
    /// Mixer/configuration clone needed by an independent playback runtime.
    /// The original stream/device resource is neither moved nor exposed for
    /// retirement bypass; only `into_retirement` can consume this lease.
    pub(crate) fn with_resource<R>(&self, project: impl FnOnce(&T) -> R) -> R {
        project(&self.handle.owned.resource.resource)
    }

    pub(crate) fn into_retirement(
        self,
    ) -> Result<SlotNormalRetirementTask<T, RetireFn>, (Self, Error)> {
        let SlotNormalRouteLease { slot, handle } = self;
        let router = match slot.take_for_io() {
            Ok(router) => router,
            Err(error) => return Err((Self { slot, handle }, error)),
        };
        match handle.into_retirement(router) {
            Ok(operation) => Ok(SlotNormalRetirementTask { slot, operation }),
            Err((router, handle, error)) => {
                slot.restore_after_io(router);
                Err((Self { slot, handle }, error))
            }
        }
    }
}

impl<T, RetireFn> SlotNormalRetirementLease<T, RetireFn>
where
    RetireFn: FnMut(&mut T) -> Result<(), BridgeDiagnostic>,
{
    pub(crate) fn into_retry(self) -> Result<SlotNormalRetirementTask<T, RetireFn>, (Self, Error)> {
        let SlotNormalRetirementLease { slot, handle } = self;
        let router = match slot.take_for_io() {
            Ok(router) => router,
            Err(error) => return Err((Self { slot, handle }, error)),
        };
        match handle.into_retry(router) {
            Ok(operation) => Ok(SlotNormalRetirementTask { slot, operation }),
            Err((router, handle, error)) => {
                slot.restore_after_io(router);
                Err((Self { slot, handle }, error))
            }
        }
    }
}

impl<T, RetireFn> SlotNormalRetirementTask<T, RetireFn>
where
    RetireFn: FnMut(&mut T) -> Result<(), BridgeDiagnostic>,
{
    pub(crate) fn perform(self) -> SlotNormalRetirementResult<T, RetireFn> {
        let SlotNormalRetirementTask { slot, operation } = self;
        let result = operation.perform();
        let succeeded = result.succeeded();
        let (router, retry) = result.into_parts();
        let snapshot = router.snapshot();
        slot.restore_after_io(router);
        SlotNormalRetirementResult {
            succeeded,
            snapshot,
            retry: retry.map(|handle| SlotNormalRetirementLease { slot, handle }),
        }
    }
}

#[cfg(test)]
#[path = "audio_output_router_tests.rs"]
mod tests;
