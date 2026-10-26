//! Conformance examples for startup selection and the test-gated bootstrap.
//! These tests do not exercise a GPU, window, cleanup ACK or physical present.
use super::*;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

use BootstrapCheckpoint as Check;
use BootstrapOperation as Op;

const EXPECTED: PhysicalExtent = PhysicalExtent {
    width: 2560,
    height: 1600,
};
const OTHER: PhysicalExtent = PhysicalExtent {
    width: 1920,
    height: 1080,
};
const BACKEND_ERROR: &str = "injected backend failure";
const AUTHORITY_ERROR: &str = "authority changed";

fn hidden() -> BootstrapGeometry {
    BootstrapGeometry::Hidden {
        declared_physical: EXPECTED,
        observed_logical: LogicalExtent {
            width: 1920,
            height: 1080,
        },
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Event {
    Authority(Check),
    Operation(Op, Option<PhysicalExtent>),
}

#[derive(Clone)]
struct Witness {
    events: Arc<Mutex<Vec<Event>>>,
    dropped: Arc<Mutex<Vec<&'static str>>>,
    stop: Arc<AtomicBool>,
}

impl Witness {
    fn operations(&self) -> Vec<(Op, Option<PhysicalExtent>)> {
        self.events
            .lock()
            .unwrap()
            .iter()
            .filter_map(|event| match event {
                Event::Operation(op, extent) => Some((*op, *extent)),
                Event::Authority(_) => None,
            })
            .collect()
    }

    fn operation_kinds(&self) -> Vec<Op> {
        self.operations().into_iter().map(|(op, _)| op).collect()
    }

    fn preparations(&self) -> usize {
        self.operation_kinds()
            .iter()
            .filter(|&&op| op == Op::Prepare)
            .count()
    }

    fn assert_retained(&self) {
        assert!(
            self.dropped.lock().unwrap().is_empty(),
            "owner dropped before handoff"
        );
    }
}

struct OwnedResource {
    name: &'static str,
    dropped: Arc<Mutex<Vec<&'static str>>>,
}

impl Drop for OwnedResource {
    fn drop(&mut self) {
        self.dropped.lock().unwrap().push(self.name);
    }
}

// Neither Ready nor the backend implements Clone.
struct FakeReady {
    extent: PhysicalExtent,
    _resource: OwnedResource,
}

struct FakeBackend {
    resources: Vec<OwnedResource>,
    witness: Witness,
    authority_error: Option<Check>,
    stop_during_authority: Option<Check>,
    fail_operation: Option<Op>,
    stop_after_operation: Option<Op>,
    revoke_after_operation: Option<Op>,
    authority_revoked: bool,
    observed: PhysicalExtent,
    after_configure: Option<PhysicalExtent>,
    ready_extent: PhysicalExtent,
    configured: bool,
}

impl FakeBackend {
    fn new() -> (Self, Witness) {
        let witness = Witness {
            events: Arc::new(Mutex::new(Vec::new())),
            dropped: Arc::new(Mutex::new(Vec::new())),
            stop: Arc::new(AtomicBool::new(false)),
        };
        let resources = ["window", "stop"]
            .into_iter()
            .map(|name| OwnedResource {
                name,
                dropped: Arc::clone(&witness.dropped),
            })
            .collect();
        (
            Self {
                resources,
                witness: witness.clone(),
                authority_error: None,
                stop_during_authority: None,
                fail_operation: None,
                stop_after_operation: None,
                revoke_after_operation: None,
                authority_revoked: false,
                observed: EXPECTED,
                after_configure: None,
                ready_extent: EXPECTED,
                configured: false,
            },
            witness,
        )
    }

    fn operation(&mut self, op: Op, extent: Option<PhysicalExtent>) -> Result<(), &'static str> {
        self.witness
            .events
            .lock()
            .unwrap()
            .push(Event::Operation(op, extent));
        if self.stop_after_operation == Some(op) {
            self.witness.stop.store(true, Ordering::Release);
        }
        if self.revoke_after_operation == Some(op) {
            self.authority_revoked = true;
        }
        if self.fail_operation == Some(op) {
            Err(BACKEND_ERROR)
        } else {
            Ok(())
        }
    }

    fn owned_names(&self) -> Vec<&'static str> {
        self.resources
            .iter()
            .map(|resource| resource.name)
            .collect()
    }
}

impl BootstrapBackend for FakeBackend {
    type Ready = FakeReady;
    type Error = &'static str;

    fn is_stopped(&self) -> bool {
        self.witness.stop.load(Ordering::Acquire)
    }

    fn check_authority(&mut self, at: Check) -> Result<(), Self::Error> {
        self.witness
            .events
            .lock()
            .unwrap()
            .push(Event::Authority(at));
        if self.stop_during_authority == Some(at) {
            self.witness.stop.store(true, Ordering::Release);
        }
        if self.authority_revoked || self.authority_error == Some(at) {
            Err(AUTHORITY_ERROR)
        } else {
            Ok(())
        }
    }

    fn initialize_unconfigured(&mut self) -> Result<(), Self::Error> {
        // Model resources created before a fallible initialization step.
        self.resources.extend(
            [
                "gpu_owner",
                "context",
                "runtime",
                "history",
                "cache_fence",
                "surface",
            ]
            .into_iter()
            .map(|name| OwnedResource {
                name,
                dropped: Arc::clone(&self.witness.dropped),
            }),
        );
        self.operation(Op::InitializeUnconfigured, None)
    }

    fn prepare_once(&mut self, extent: PhysicalExtent) -> Result<Self::Ready, Self::Error> {
        self.operation(Op::Prepare, Some(extent))?;
        Ok(FakeReady {
            extent: self.ready_extent,
            _resource: OwnedResource {
                name: "ready",
                dropped: Arc::clone(&self.witness.dropped),
            },
        })
    }

    fn ready_extent(&self, ready: &Self::Ready) -> PhysicalExtent {
        ready.extent
    }

    fn show_and_activate(
        &mut self,
        expected: PhysicalExtent,
    ) -> Result<PhysicalExtent, Self::Error> {
        self.operation(Op::ShowActivate, Some(expected))?;
        Ok(self.observed)
    }

    fn observe_physical_extent(&mut self) -> Result<PhysicalExtent, Self::Error> {
        self.operation(Op::ObservePhysical, None)?;
        Ok(if self.configured {
            self.after_configure.unwrap_or(self.observed)
        } else {
            self.observed
        })
    }

    fn configure(&mut self, verified: PhysicalExtent) -> Result<(), Self::Error> {
        self.operation(Op::Configure, Some(verified))?;
        self.configured = true;
        Ok(())
    }
}

type Attempt = Result<PreparedBootstrap<FakeBackend>, RetainedBootstrapFailure<FakeBackend>>;

fn prepare_native_display(backend: FakeBackend, geometry: BootstrapGeometry) -> Attempt {
    match dispatch_native_display(BootstrapRouteGate::for_test_joined(), backend, geometry) {
        BootstrapDispatch::Resident(attempt) => attempt,
        BootstrapDispatch::Existing { .. } => panic!("joined fixture did not select bootstrap"),
    }
}

fn success(attempt: Attempt) -> PreparedBootstrap<FakeBackend> {
    match attempt {
        Ok(prepared) => prepared,
        Err(failure) => panic!("unexpected rejection: {:?}", failure.reason()),
    }
}

fn failure(attempt: Attempt) -> RetainedBootstrapFailure<FakeBackend> {
    match attempt {
        Err(failure) => failure,
        Ok(_) => panic!("unexpected bootstrap success"),
    }
}

fn move_send<T: Send>(value: T) -> T {
    value
}

const OWNERS: [&str; 8] = [
    "window",
    "stop",
    "gpu_owner",
    "context",
    "runtime",
    "history",
    "cache_fence",
    "surface",
];

fn assert_complete_drop(witness: &Witness, with_ready: bool) {
    let mut expected = OWNERS.to_vec();
    if with_ready {
        expected.push("ready");
    }
    let mut actual = witness.dropped.lock().unwrap().clone();
    expected.sort_unstable();
    actual.sort_unstable();
    assert_eq!(
        actual, expected,
        "every owned mock resource is released exactly once"
    );
}

#[test]
fn control_plane_native_gpu_startup_requires_the_exact_explicit_opt_in() {
    assert_eq!(
        NativeDisplayStartupRoute::from_opt_in(None),
        NativeDisplayStartupRoute::Existing,
    );
    for value in [
        "", "0", "true", "TRUE", "yes", "on", "01", "1 ", " 1", "1\n",
    ] {
        assert_eq!(
            NativeDisplayStartupRoute::from_opt_in(Some(OsStr::new(value))),
            NativeDisplayStartupRoute::Existing,
            "only the exact QA value may request the resident route",
        );
    }
    assert_eq!(
        NativeDisplayStartupRoute::from_opt_in(Some(OsStr::new("1"))),
        NativeDisplayStartupRoute::ResidentRequested,
    );
}

#[cfg(windows)]
#[test]
fn control_plane_native_gpu_startup_non_unicode_value_does_not_opt_in() {
    use std::os::windows::ffi::OsStringExt;
    let invalid = OsString::from_wide(&[0xD800]);
    assert_eq!(
        NativeDisplayStartupRoute::from_opt_in(Some(invalid.as_os_str())),
        NativeDisplayStartupRoute::Existing,
    );
}

#[test]
fn control_plane_native_gpu_startup_selection_is_captured_once_for_rearm() {
    use std::cell::Cell;
    for initial in [None, Some(OsString::from("1"))] {
        let decision = OnceLock::new();
        let reads = Cell::new(0);
        let expected = NativeDisplayStartupRoute::from_opt_in(initial.as_deref());
        assert_eq!(
            capture_startup_route(&decision, || {
                reads.set(reads.get() + 1);
                initial
            }),
            expected,
        );
        let captured = capture_startup_route(&decision, || {
            reads.set(reads.get() + 1);
            // The opposite setting must never replace an already-captured one.
            match expected {
                NativeDisplayStartupRoute::Existing => Some(OsString::from("1")),
                NativeDisplayStartupRoute::ResidentRequested => None,
            }
        });
        assert_eq!(captured, expected);
        assert_eq!(
            reads.get(),
            1,
            "re-arm must not read a new environment value"
        );
    }
}

#[test]
fn control_plane_native_gpu_startup_entry_keeps_its_process_choice() {
    // Exercise the production entry without modifying process environment or
    // requesting a backend. Either configured choice is stable for this process.
    let first = validate_native_display_startup_route();
    assert_eq!(validate_native_display_startup_route(), first);
    match first {
        Ok(()) => {}
        Err(message) => assert_eq!(message, RESIDENT_ROUTE_UNAVAILABLE),
    }
}

#[test]
fn control_plane_native_gpu_startup_off_preserves_existing_owner_and_hooks() {
    for value in [None, Some(OsStr::new("0")), Some(OsStr::new(""))] {
        for geometry in [
            hidden(),
            BootstrapGeometry::Visible {
                observed_physical: EXPECTED,
            },
        ] {
            let route = NativeDisplayStartupRoute::from_opt_in(value);
            route.require_available().unwrap();
            let (mut backend, witness) = FakeBackend::new();
            backend.authority_error = Some(Check::Initial);
            backend.fail_operation = Some(Op::InitializeUnconfigured);
            witness.stop.store(true, Ordering::Release);
            let BootstrapDispatch::Existing {
                backend,
                geometry: returned,
            } = dispatch_native_display(BootstrapRouteGate::default(), backend, geometry)
            else {
                panic!("OFF cannot enter resident bootstrap");
            };
            assert_eq!(returned, geometry);
            assert_eq!(backend.owned_names(), vec!["window", "stop"]);
            assert!(backend.is_stopped());
            assert!(witness.events.lock().unwrap().is_empty());
            witness.assert_retained();
        }
    }
}

#[test]
fn control_plane_native_gpu_startup_opt_in_is_unavailable_before_backend_hooks() {
    let route = NativeDisplayStartupRoute::from_opt_in(Some(OsStr::new("1")));
    let (backend, witness) = FakeBackend::new();
    let mut untouched = Some(backend);
    let outcome = (|| {
        route.require_available()?;
        let backend = untouched.take().expect("same unconsumed startup owner");
        Ok::<_, &'static str>(dispatch_native_display(
            BootstrapRouteGate::default(),
            backend,
            hidden(),
        ))
    })();
    match outcome {
        Err(message) => assert_eq!(message, RESIDENT_ROUTE_UNAVAILABLE),
        Ok(_) => panic!("an unavailable resident route cannot continue with a CPU fallback"),
    }
    assert_eq!(
        untouched.as_ref().unwrap().owned_names(),
        vec!["window", "stop"],
    );
    assert!(witness.events.lock().unwrap().is_empty());
    witness.assert_retained();
}

#[test]
fn control_plane_native_gpu_bootstrap_default_gate_retains_existing_route_without_side_effects() {
    let geometries = [
        hidden(),
        BootstrapGeometry::Visible {
            observed_physical: EXPECTED,
        },
        BootstrapGeometry::Hidden {
            declared_physical: PhysicalExtent {
                width: 0,
                height: 0,
            },
            observed_logical: LogicalExtent {
                width: 1920,
                height: 1080,
            },
        },
    ];
    for geometry in geometries {
        for stopped in [false, true] {
            let (mut backend, witness) = FakeBackend::new();
            // Neither a failed authority nor a failing resident initializer is
            // consulted while C4 is unavailable. The original route owns both.
            backend.authority_error = Some(Check::Initial);
            backend.fail_operation = Some(Op::InitializeUnconfigured);
            witness.stop.store(stopped, Ordering::Release);
            let outcome = dispatch_native_display(BootstrapRouteGate::default(), backend, geometry);
            let BootstrapDispatch::Existing {
                backend,
                geometry: returned_geometry,
            } = outcome
            else {
                panic!("the default gate must not enter resident bootstrap");
            };
            assert_eq!(returned_geometry, geometry);
            assert_eq!(backend.owned_names(), vec!["window", "stop"]);
            assert_eq!(backend.is_stopped(), stopped);
            assert!(witness.events.lock().unwrap().is_empty());
            assert_eq!(witness.preparations(), 0);
            witness.assert_retained();
            drop(backend);
            assert_eq!(*witness.dropped.lock().unwrap(), vec!["window", "stop"]);
        }
    }
}

#[test]
fn hidden_bootstrap_uses_declared_physical_extent_and_moves_all_owners() {
    let (backend, witness) = FakeBackend::new();
    let prepared = move_send(success(prepare_native_display(backend, hidden())));
    assert_eq!(prepared.extent(), EXPECTED);
    assert_eq!(prepared.geometry(), hidden());
    assert_eq!(
        witness.operations(),
        vec![
            (Op::InitializeUnconfigured, None),
            (Op::Prepare, Some(EXPECTED)),
            (Op::ShowActivate, Some(EXPECTED)),
            (Op::Configure, Some(EXPECTED)),
            (Op::ObservePhysical, None),
        ]
    );
    assert_eq!(witness.preparations(), 1);
    witness.assert_retained();
    let (ready, backend) = prepared.into_parts();
    assert_eq!(ready.extent, EXPECTED);
    assert_eq!(backend.owned_names(), OWNERS);
    assert!(Arc::ptr_eq(&backend.witness.stop, &witness.stop));
    witness.assert_retained();
    drop(ready);
    assert_eq!(*witness.dropped.lock().unwrap(), vec!["ready"]);
    drop(backend);
    assert_complete_drop(&witness, true);
}

#[test]
fn visible_bootstrap_observes_physical_extent_without_showing_again() {
    let (backend, witness) = FakeBackend::new();
    let geometry = BootstrapGeometry::Visible {
        observed_physical: EXPECTED,
    };
    let prepared = success(prepare_native_display(backend, geometry));
    assert_eq!(prepared.geometry(), geometry);
    assert_eq!(
        witness.operation_kinds(),
        vec![
            Op::InitializeUnconfigured,
            Op::Prepare,
            Op::ObservePhysical,
            Op::Configure,
            Op::ObservePhysical,
        ]
    );
    assert_eq!(witness.preparations(), 1);
    drop(prepared);
    assert_eq!(witness.dropped.lock().unwrap()[0], "ready");
    assert_complete_drop(&witness, true);
}

#[test]
fn post_show_mismatch_keeps_ready_and_cleanup_owner_without_retry() {
    let (mut backend, witness) = FakeBackend::new();
    backend.observed = OTHER;
    let rejected = move_send(failure(prepare_native_display(backend, hidden())));
    assert_eq!(
        rejected.reason(),
        &BootstrapRejection::GeometryMismatch {
            expected: EXPECTED,
            observed: OTHER,
        }
    );
    assert!(rejected.has_prepared_frame());
    assert_eq!(
        witness.operation_kinds(),
        vec![Op::InitializeUnconfigured, Op::Prepare, Op::ShowActivate,]
    );
    assert_eq!(witness.preparations(), 1);
    witness.assert_retained();
    let (reason, ready, backend) = rejected.into_parts();
    assert_eq!(
        reason,
        BootstrapRejection::GeometryMismatch {
            expected: EXPECTED,
            observed: OTHER,
        }
    );
    assert_eq!(ready.as_ref().unwrap().extent, EXPECTED);
    assert_eq!(backend.owned_names(), OWNERS);
    witness.assert_retained();
    drop(ready);
    drop(backend);
    assert_complete_drop(&witness, true);
}

#[test]
fn wrong_ready_extent_is_rejected_before_show_or_configure() {
    let (mut backend, witness) = FakeBackend::new();
    backend.ready_extent = OTHER;
    let rejected = failure(prepare_native_display(backend, hidden()));
    assert_eq!(
        rejected.reason(),
        &BootstrapRejection::ReadyGeometryMismatch {
            expected: EXPECTED,
            actual: OTHER,
        }
    );
    assert!(rejected.has_prepared_frame());
    assert_eq!(
        witness.operation_kinds(),
        vec![Op::InitializeUnconfigured, Op::Prepare]
    );
    witness.assert_retained();
    drop(rejected);
    assert_eq!(witness.dropped.lock().unwrap()[0], "ready");
    assert_complete_drop(&witness, true);
}

#[test]
fn zero_initial_extent_suspends_before_initialization_or_preparation() {
    for zero in [
        PhysicalExtent {
            width: 0,
            height: 1600,
        },
        PhysicalExtent {
            width: 2560,
            height: 0,
        },
    ] {
        for geometry in [
            BootstrapGeometry::Hidden {
                declared_physical: zero,
                observed_logical: LogicalExtent {
                    width: 1280,
                    height: 800,
                },
            },
            BootstrapGeometry::Visible {
                observed_physical: zero,
            },
        ] {
            let (backend, witness) = FakeBackend::new();
            let rejected = failure(prepare_native_display(backend, geometry));
            assert_eq!(
                rejected.reason(),
                &BootstrapRejection::Suspended { observed: zero }
            );
            assert!(!rejected.has_prepared_frame());
            assert!(witness.operations().is_empty());
            witness.assert_retained();
            drop(rejected);
            assert_eq!(*witness.dropped.lock().unwrap(), vec!["window", "stop"]);
        }
    }
}

#[test]
fn minimization_after_show_suspends_without_configuration() {
    let (mut backend, witness) = FakeBackend::new();
    let zero = PhysicalExtent {
        width: 0,
        height: 0,
    };
    backend.observed = zero;
    let rejected = failure(prepare_native_display(backend, hidden()));
    assert_eq!(
        rejected.reason(),
        &BootstrapRejection::Suspended { observed: zero }
    );
    assert!(rejected.has_prepared_frame());
    assert_eq!(
        witness.operation_kinds(),
        vec![Op::InitializeUnconfigured, Op::Prepare, Op::ShowActivate,]
    );
    witness.assert_retained();
}

#[test]
fn resize_or_minimization_during_configuration_cannot_offer_a_handoff() {
    for observed in [
        OTHER,
        PhysicalExtent {
            width: 2560,
            height: 0,
        },
    ] {
        let (mut backend, witness) = FakeBackend::new();
        backend.after_configure = Some(observed);
        let rejected = failure(prepare_native_display(backend, hidden()));
        let expected = if observed.height == 0 {
            BootstrapRejection::Suspended { observed }
        } else {
            BootstrapRejection::GeometryMismatch {
                expected: EXPECTED,
                observed,
            }
        };
        assert_eq!(rejected.reason(), &expected);
        assert!(rejected.has_prepared_frame());
        assert_eq!(witness.preparations(), 1);
        assert_eq!(witness.operation_kinds().last(), Some(&Op::ObservePhysical));
        witness.assert_retained();
    }
}

#[test]
fn initially_stopped_bootstrap_does_not_acquire_or_check_authority() {
    let (backend, witness) = FakeBackend::new();
    witness.stop.store(true, Ordering::Release);
    let rejected = failure(prepare_native_display(backend, hidden()));
    assert_eq!(
        rejected.reason(),
        &BootstrapRejection::Cancelled { at: Check::Initial }
    );
    assert!(!rejected.has_prepared_frame());
    assert!(witness.events.lock().unwrap().is_empty());
    witness.assert_retained();
}

#[test]
fn stop_after_each_backend_step_blocks_the_next_step_and_retains_resources() {
    let sequence = [
        (Op::InitializeUnconfigured, Check::BeforePrepare, false),
        (Op::Prepare, Check::BeforeShow, true),
        (Op::ShowActivate, Check::BeforeConfigure, true),
        (Op::Configure, Check::BeforeHandoff, true),
        (Op::ObservePhysical, Check::BeforeHandoff, true),
    ];
    for (index, (operation, at, has_ready)) in sequence.into_iter().enumerate() {
        let (mut backend, witness) = FakeBackend::new();
        backend.stop_after_operation = Some(operation);
        let rejected = failure(prepare_native_display(backend, hidden()));
        assert_eq!(rejected.reason(), &BootstrapRejection::Cancelled { at });
        assert_eq!(rejected.has_prepared_frame(), has_ready);
        assert_eq!(witness.operations().len(), index + 1);
        assert_eq!(witness.operation_kinds().last(), Some(&operation));
        witness.assert_retained();
        drop(rejected);
        assert_complete_drop(&witness, has_ready);
    }
}

#[test]
fn every_authority_boundary_can_reject_without_losing_the_owned_bundle() {
    let checkpoints = [
        Check::Initial,
        Check::BeforePrepare,
        Check::BeforeShow,
        Check::BeforeConfigure,
        Check::BeforeHandoff,
    ];
    for (index, at) in checkpoints.into_iter().enumerate() {
        let (mut backend, witness) = FakeBackend::new();
        backend.authority_error = Some(at);
        let rejected = failure(prepare_native_display(backend, hidden()));
        assert_eq!(
            rejected.reason(),
            &BootstrapRejection::Authority {
                at,
                error: AUTHORITY_ERROR,
            }
        );
        assert_eq!(witness.operations().len(), index);
        assert_eq!(rejected.has_prepared_frame(), index >= 2);
        assert_eq!(
            witness.events.lock().unwrap().last(),
            Some(&Event::Authority(at))
        );
        witness.assert_retained();
    }
}

#[test]
fn stop_during_authority_validation_wins_over_the_authority_error() {
    for at in [
        Check::Initial,
        Check::BeforePrepare,
        Check::BeforeShow,
        Check::BeforeConfigure,
        Check::BeforeHandoff,
    ] {
        let (mut backend, witness) = FakeBackend::new();
        backend.stop_during_authority = Some(at);
        backend.authority_error = Some(at);
        let rejected = failure(prepare_native_display(backend, hidden()));
        assert_eq!(rejected.reason(), &BootstrapRejection::Cancelled { at });
        assert_eq!(
            witness.events.lock().unwrap().last(),
            Some(&Event::Authority(at))
        );
        witness.assert_retained();
    }
}

#[test]
fn authority_revocation_during_final_geometry_query_rejects_handoff() {
    let (mut backend, witness) = FakeBackend::new();
    backend.revoke_after_operation = Some(Op::ObservePhysical);
    let rejected = failure(prepare_native_display(backend, hidden()));
    assert_eq!(
        rejected.reason(),
        &BootstrapRejection::Authority {
            at: Check::BeforeHandoff,
            error: AUTHORITY_ERROR,
        }
    );
    assert!(rejected.has_prepared_frame());
    let events = witness.events.lock().unwrap();
    assert_eq!(
        &events[events.len() - 2..],
        &[
            Event::Operation(Op::ObservePhysical, None),
            Event::Authority(Check::BeforeHandoff),
        ]
    );
    drop(events);
    witness.assert_retained();
}

#[test]
fn all_backend_errors_keep_partial_owners_and_never_repeat_preparation() {
    for operation in [
        Op::InitializeUnconfigured,
        Op::Prepare,
        Op::ShowActivate,
        Op::Configure,
        Op::ObservePhysical,
    ] {
        let (mut backend, witness) = FakeBackend::new();
        backend.fail_operation = Some(operation);
        let rejected = failure(prepare_native_display(backend, hidden()));
        assert_eq!(
            rejected.reason(),
            &BootstrapRejection::Backend {
                operation,
                error: BACKEND_ERROR,
            }
        );
        let has_ready = !matches!(operation, Op::InitializeUnconfigured | Op::Prepare);
        assert_eq!(rejected.has_prepared_frame(), has_ready);
        assert!(witness.preparations() <= 1);
        assert_eq!(witness.operation_kinds().last(), Some(&operation));
        witness.assert_retained();
        let (_, ready, backend) = rejected.into_parts();
        assert_eq!(
            backend.owned_names(),
            OWNERS,
            "including partially initialized resources"
        );
        drop(ready);
        drop(backend);
        assert_complete_drop(&witness, has_ready);
    }
}

#[test]
fn live_stop_wins_over_simultaneous_errors_at_every_backend_call_site() {
    let cases = [
        (
            hidden(),
            Op::InitializeUnconfigured,
            Check::BeforePrepare,
            1,
            false,
        ),
        (hidden(), Op::Prepare, Check::BeforeShow, 2, false),
        (hidden(), Op::ShowActivate, Check::BeforeConfigure, 3, true),
        (hidden(), Op::Configure, Check::BeforeHandoff, 4, true),
        (hidden(), Op::ObservePhysical, Check::BeforeHandoff, 5, true),
        (
            BootstrapGeometry::Visible {
                observed_physical: EXPECTED,
            },
            Op::ObservePhysical,
            Check::BeforeConfigure,
            3,
            true,
        ),
    ];
    for (geometry, operation, at, operation_count, has_ready) in cases {
        let (mut backend, witness) = FakeBackend::new();
        backend.stop_after_operation = Some(operation);
        backend.fail_operation = Some(operation);
        let rejected = failure(prepare_native_display(backend, geometry));
        assert_eq!(rejected.reason(), &BootstrapRejection::Cancelled { at });
        assert_eq!(rejected.has_prepared_frame(), has_ready);
        assert_eq!(witness.operations().len(), operation_count);
        assert_eq!(witness.operation_kinds().last(), Some(&operation));
        assert!(witness.preparations() <= 1);
        witness.assert_retained();
        let (_, ready, backend) = rejected.into_parts();
        assert_eq!(backend.owned_names(), OWNERS);
        drop(ready);
        drop(backend);
        assert_complete_drop(&witness, has_ready);
    }
}
