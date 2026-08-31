//! Backend-only keepalive state machine for a managed exact-`Both` output lease.
//!
//! A future integration adapter must prove exact Lighting+Video authority and
//! a durable `Enable` receipt before issuing
//! [`OutputLeaseKeepaliveDurableEnableEvidence`]. This module deliberately
//! imports neither Tauri nor sibling output/engine types, so its APIs are split
//! into three phases:
//!
//! 1. prepare under the application's manager lock;
//! 2. renew/sleep/spawn/retire outside that lock;
//! 3. complete under the lock with the captured [`OutputLeaseKeepaliveRunId`].
//!
//! The run ID is an ABA fence. A late result from old run A is a no-op after A
//! has faulted, its exact registry relinquish has completed, and a later
//! durable Enable has armed run B.

use std::{
    fmt,
    sync::{
        atomic::{AtomicU8, Ordering},
        Arc,
    },
};

pub(crate) const OUTPUT_LEASE_KEEPALIVE_TTL_MS: u64 = 60_000;
pub(crate) const OUTPUT_LEASE_KEEPALIVE_RENEW_INTERVAL_MS: u64 = 20_000;
const MAX_OUTPUT_LEASE_KEEPALIVE_PROJECT_BYTES: usize = 256;
const MAX_OUTPUT_LEASE_KEEPALIVE_OWNER_BYTES: usize = 128;
const MAX_OUTPUT_LEASE_KEEPALIVE_WINDOW_BYTES: usize = 128;
const MAX_OUTPUT_LEASE_KEEPALIVE_SESSION_BYTES: usize = 128;
const MAX_OUTPUT_LEASE_KEEPALIVE_REQUEST_BYTES: usize = 128;
const MAX_OUTPUT_LEASE_KEEPALIVE_ERROR_BYTES: usize = 256;

fn bounded_nonempty(value: &str, max_bytes: usize) -> bool {
    !value.trim().is_empty() && value.len() <= max_bytes
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct OutputLeaseKeepaliveRunId(u64);

impl OutputLeaseKeepaliveRunId {
    pub(crate) fn get(self) -> u64 {
        self.0
    }
}

/// Validated exact-`Both` lease component for a durable Enable evidence.
///
/// The integration layer has already checked the registry scope is exactly
/// Both; this core cannot and must not infer resource scope itself. This value
/// alone cannot arm the worker: it must be bound to all durable-Enable
/// identities through [`OutputLeaseKeepaliveDurableEnableEvidence`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OutputLeaseKeepaliveExactBothLease {
    lease_id: String,
    generation: u64,
}

impl OutputLeaseKeepaliveExactBothLease {
    pub(crate) fn from_verified_exact_both(
        lease_id: impl Into<String>,
        generation: u64,
    ) -> Result<Self, OutputLeaseKeepaliveError> {
        let lease_id = lease_id.into();
        if !is_canonical_lease_id(&lease_id) || generation == 0 {
            return Err(OutputLeaseKeepaliveError::InvalidLeaseAuthority);
        }
        Ok(Self {
            lease_id,
            generation,
        })
    }

    pub(crate) fn lease_id(&self) -> &str {
        &self.lease_id
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }
}

fn is_canonical_lease_id(value: &str) -> bool {
    let bytes = value.as_bytes();
    value.len() == 22
        && value.is_ascii()
        && bytes.starts_with(b"lease-")
        && bytes[6..]
            .iter()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(byte))
        && bytes[6..].iter().any(|byte| *byte != b'0')
}

/// Exact ownership and request correlation carried into every renewal. The
/// outer adapter maps these fields to the existing local owner, project, and
/// durable output-control request/receipt identities.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OutputLeaseKeepaliveIdentity {
    project_identity: String,
    owner_identity: String,
    window_label: String,
    session_identity: String,
    request_correlation: String,
}

impl OutputLeaseKeepaliveIdentity {
    pub(crate) fn new(
        project_identity: impl Into<String>,
        owner_identity: impl Into<String>,
        window_label: impl Into<String>,
        session_identity: impl Into<String>,
        request_correlation: impl Into<String>,
    ) -> Result<Self, OutputLeaseKeepaliveError> {
        let value = Self {
            project_identity: project_identity.into(),
            owner_identity: owner_identity.into(),
            window_label: window_label.into(),
            session_identity: session_identity.into(),
            request_correlation: request_correlation.into(),
        };
        if !bounded_nonempty(
            &value.project_identity,
            MAX_OUTPUT_LEASE_KEEPALIVE_PROJECT_BYTES,
        ) || !bounded_nonempty(
            &value.owner_identity,
            MAX_OUTPUT_LEASE_KEEPALIVE_OWNER_BYTES,
        ) || !bounded_nonempty(&value.window_label, MAX_OUTPUT_LEASE_KEEPALIVE_WINDOW_BYTES)
            || !bounded_nonempty(
                &value.session_identity,
                MAX_OUTPUT_LEASE_KEEPALIVE_SESSION_BYTES,
            )
            || !bounded_nonempty(
                &value.request_correlation,
                MAX_OUTPUT_LEASE_KEEPALIVE_REQUEST_BYTES,
            )
        {
            return Err(OutputLeaseKeepaliveError::InvalidIdentity);
        }
        Ok(value)
    }

    pub(crate) fn project_identity(&self) -> &str {
        &self.project_identity
    }

    pub(crate) fn owner_identity(&self) -> &str {
        &self.owner_identity
    }

    pub(crate) fn window_label(&self) -> &str {
        &self.window_label
    }

    pub(crate) fn session_identity(&self) -> &str {
        &self.session_identity
    }

    pub(crate) fn request_correlation(&self) -> &str {
        &self.request_correlation
    }
}

/// Registry-owner components copied only from a verified canonical durable
/// Enable receipt. The parent `output_lease` module constructs this type; no
/// sibling adapter can manufacture it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct OutputLeaseKeepaliveRegistryOwner {
    principal: String,
    window_label: String,
    process_session_incarnation: u64,
    owner_incarnation: u64,
}

impl OutputLeaseKeepaliveRegistryOwner {
    pub(super) fn from_verified_components(
        principal: String,
        window_label: String,
        process_session_incarnation: u64,
        owner_incarnation: u64,
    ) -> Result<Self, OutputLeaseKeepaliveError> {
        if !bounded_nonempty(&principal, MAX_OUTPUT_LEASE_KEEPALIVE_OWNER_BYTES)
            || !bounded_nonempty(&window_label, MAX_OUTPUT_LEASE_KEEPALIVE_WINDOW_BYTES)
            || process_session_incarnation == 0
            || owner_incarnation == 0
        {
            return Err(OutputLeaseKeepaliveError::InvalidDurableEnableEvidence);
        }
        Ok(Self {
            principal,
            window_label,
            process_session_incarnation,
            owner_incarnation,
        })
    }

    pub(super) fn principal(&self) -> &str {
        &self.principal
    }

    pub(super) fn window_label(&self) -> &str {
        &self.window_label
    }

    pub(super) fn process_session_incarnation(&self) -> u64 {
        self.process_session_incarnation
    }

    pub(super) fn owner_incarnation(&self) -> u64 {
        self.owner_incarnation
    }
}

/// Exact registry claim carried by a manager-issued CAS capability. It is
/// populated exclusively from a verified durable Enable receipt and remains
/// private to this child/parent authority boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct OutputLeaseKeepaliveRegistryClaim {
    lease_id: String,
    generation: u64,
    project_identity: String,
    owner: OutputLeaseKeepaliveRegistryOwner,
    durable_receipt_principal: String,
    durable_receipt_domain: String,
    durable_receipt_request_id: u64,
    durable_receipt_issuance: u64,
}

impl OutputLeaseKeepaliveRegistryClaim {
    pub(super) fn from_verified_durable_enable_receipt(
        lease_id: String,
        generation: u64,
        project_identity: String,
        owner: OutputLeaseKeepaliveRegistryOwner,
        durable_receipt_principal: String,
        durable_receipt_domain: String,
        durable_receipt_request_id: u64,
        durable_receipt_issuance: u64,
    ) -> Result<Self, OutputLeaseKeepaliveError> {
        if !is_canonical_lease_id(&lease_id)
            || generation == 0
            || !bounded_nonempty(&project_identity, MAX_OUTPUT_LEASE_KEEPALIVE_PROJECT_BYTES)
            || !bounded_nonempty(
                &durable_receipt_principal,
                MAX_OUTPUT_LEASE_KEEPALIVE_REQUEST_BYTES,
            )
            || !bounded_nonempty(
                &durable_receipt_domain,
                MAX_OUTPUT_LEASE_KEEPALIVE_REQUEST_BYTES,
            )
            || durable_receipt_request_id == 0
            || durable_receipt_issuance == 0
        {
            return Err(OutputLeaseKeepaliveError::InvalidDurableEnableEvidence);
        }
        Ok(Self {
            lease_id,
            generation,
            project_identity,
            owner,
            durable_receipt_principal,
            durable_receipt_domain,
            durable_receipt_request_id,
            durable_receipt_issuance,
        })
    }

    pub(super) fn lease_id(&self) -> &str {
        &self.lease_id
    }

    pub(super) fn generation(&self) -> u64 {
        self.generation
    }

    pub(super) fn project_identity(&self) -> &str {
        &self.project_identity
    }

    pub(super) fn owner(&self) -> &OutputLeaseKeepaliveRegistryOwner {
        &self.owner
    }

    pub(super) fn durable_receipt_principal(&self) -> &str {
        &self.durable_receipt_principal
    }

    pub(super) fn durable_receipt_domain(&self) -> &str {
        &self.durable_receipt_domain
    }

    pub(super) fn durable_receipt_request_id(&self) -> u64 {
        self.durable_receipt_request_id
    }

    pub(super) fn durable_receipt_issuance(&self) -> u64 {
        self.durable_receipt_issuance
    }
}

/// Opaque, non-`Clone` evidence issued only after the parent registry has
/// correlated one durable canonical exact-`Both` Enable receipt with its
/// current record. It binds the actual lease/owner/project/generation and the
/// monotonic receipt issuance to the local identity in one value.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct OutputLeaseKeepaliveDurableEnableEvidence {
    identity: OutputLeaseKeepaliveIdentity,
    lease: OutputLeaseKeepaliveExactBothLease,
    registry_claim: OutputLeaseKeepaliveRegistryClaim,
}

impl OutputLeaseKeepaliveDurableEnableEvidence {
    /// Parent-only issuance seam. Production adapters must first prove the
    /// durable terminal receipt through `OutputLeaseRegistry`; tests use that
    /// same production boundary rather than a synthetic evidence constructor.
    pub(super) fn issue_after_verified_durable_enable_receipt(
        identity: OutputLeaseKeepaliveIdentity,
        registry_claim: OutputLeaseKeepaliveRegistryClaim,
    ) -> Result<Self, OutputLeaseKeepaliveError> {
        let lease = verified_durable_enable_lease(&identity, &registry_claim)?;
        Ok(Self {
            identity,
            lease,
            registry_claim,
        })
    }

    fn into_components_for_arm(
        self,
    ) -> (
        OutputLeaseKeepaliveIdentity,
        OutputLeaseKeepaliveExactBothLease,
        OutputLeaseKeepaliveRegistryClaim,
    ) {
        (self.identity, self.lease, self.registry_claim)
    }
}

/// Opaque non-arm authority used only to retire outputs after a durable
/// exact-`Both` Enable committed but normal evidence issuance/admission could
/// not proceed.  Unlike [`OutputLeaseKeepaliveDurableEnableEvidence`], this
/// type intentionally has no conversion into arm, renewal, or deferred-enable
/// components.  Its only manager entrypoint creates or resumes a fail-stop.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct OutputLeaseKeepaliveDurableEnableCleanupOnlyEvidence {
    identity: OutputLeaseKeepaliveIdentity,
    lease: OutputLeaseKeepaliveExactBothLease,
    registry_claim: OutputLeaseKeepaliveRegistryClaim,
}

impl OutputLeaseKeepaliveDurableEnableCleanupOnlyEvidence {
    /// Parent-only issuance after the caller's durable commit boundary. The
    /// parent validates request/receipt shape before constructing this value;
    /// this core deliberately performs no current-registry read, because the
    /// originating failure may itself be a registry or journal lock poison.
    pub(super) fn issue_after_durable_enable_commit_without_current_registry_proof(
        identity: OutputLeaseKeepaliveIdentity,
        registry_claim: OutputLeaseKeepaliveRegistryClaim,
    ) -> Result<Self, OutputLeaseKeepaliveError> {
        let lease = verified_durable_enable_lease(&identity, &registry_claim)?;
        Ok(Self {
            identity,
            lease,
            registry_claim,
        })
    }

    fn durable_receipt_issuance(&self) -> u64 {
        self.registry_claim.durable_receipt_issuance
    }

    fn matches_armed(&self, armed: &OutputLeaseKeepaliveArmedState) -> bool {
        armed.identity == self.identity
            && armed.lease == self.lease
            && armed.registry_claim == self.registry_claim
    }

    fn matches_fault(&self, fault: &OutputLeaseKeepaliveFaultState) -> bool {
        fault.identity == self.identity
            && fault.lease == self.lease
            && fault.registry_claim == self.registry_claim
    }

    fn into_components_for_cleanup(
        self,
    ) -> (
        OutputLeaseKeepaliveIdentity,
        OutputLeaseKeepaliveExactBothLease,
        OutputLeaseKeepaliveRegistryClaim,
    ) {
        (self.identity, self.lease, self.registry_claim)
    }
}

fn verified_durable_enable_lease(
    identity: &OutputLeaseKeepaliveIdentity,
    registry_claim: &OutputLeaseKeepaliveRegistryClaim,
) -> Result<OutputLeaseKeepaliveExactBothLease, OutputLeaseKeepaliveError> {
    if identity.project_identity != registry_claim.project_identity
        || identity.owner_identity != registry_claim.owner.principal
        || identity.window_label != registry_claim.owner.window_label
        || identity.session_identity
            != format!(
                "process:{};owner:{}",
                registry_claim.owner.process_session_incarnation,
                registry_claim.owner.owner_incarnation
            )
        || identity.request_correlation
            != format!(
                "principal:{};domain:{};request:{}",
                registry_claim.durable_receipt_principal,
                registry_claim.durable_receipt_domain,
                registry_claim.durable_receipt_request_id
            )
        || registry_claim.generation == 0
        || registry_claim.durable_receipt_request_id == 0
        || registry_claim.durable_receipt_issuance == 0
        || !bounded_nonempty(
            &registry_claim.durable_receipt_principal,
            MAX_OUTPUT_LEASE_KEEPALIVE_REQUEST_BYTES,
        )
        || !bounded_nonempty(
            &registry_claim.durable_receipt_domain,
            MAX_OUTPUT_LEASE_KEEPALIVE_REQUEST_BYTES,
        )
    {
        return Err(OutputLeaseKeepaliveError::InvalidDurableEnableEvidence);
    }
    OutputLeaseKeepaliveExactBothLease::from_verified_exact_both(
        registry_claim.lease_id.clone(),
        registry_claim.generation,
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OutputLeaseKeepaliveBoundary {
    /// A durable exact-`Both` Enable succeeded, but this process could not
    /// arm its keepalive worker. The returned admission plan must execute
    /// before the caller can expose the unarmed route to any output port.
    ArmAdmissionFailed,
    /// A newer durable Enable arrived while this manager still owns an older
    /// exact-Both run. The newer authority stays opaque and unarmed until this
    /// current run has completed its canonical fail-stop and exact registry
    /// relinquish. This is deliberately an automatic boundary: a manual
    /// Relinquish label must never be able to bypass the current run's fence.
    ConflictingDurableEnable,
    Relinquish,
    Standby,
    ProjectIdentityChanged,
    #[cfg(test)]
    OwnerIdentityChanged,
    WindowIdentityChanged,
    SessionIdentityChanged,
    #[cfg(test)]
    RequestCorrelationChanged,
    RenewalFailed,
    InvalidRenewalReceipt,
    ClockFailed,
    ClockRollback,
    WorkerSpawnFailed,
    /// The worker which owned an Armed run returned or panicked before the
    /// manager had transitioned that exact run into fail-stop.  A live lease
    /// must never remain Armed merely because its worker disappeared.
    WorkerTerminated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OutputLeaseKeepaliveState {
    Idle,
    Armed,
    Faulting,
    Faulted,
}

const RENEWAL_GATE_ISSUED: u8 = 0;
const RENEWAL_GATE_CLAIMED_BY_REGISTRY: u8 = 1;
const RENEWAL_GATE_INVALIDATED: u8 = 2;

#[derive(Debug)]
struct OutputLeaseKeepaliveRenewalGate {
    state: AtomicU8,
}

const RELINQUISH_GATE_ISSUED: u8 = 0;
const RELINQUISH_GATE_CONSUMED_BY_REGISTRY: u8 = 1;
const RELINQUISH_GATE_ABORTED_BEFORE_REGISTRY: u8 = 2;
const RELINQUISH_GATE_INVALIDATED: u8 = 3;

#[derive(Debug)]
struct OutputLeaseKeepaliveRelinquishGate {
    state: AtomicU8,
}

impl PartialEq for OutputLeaseKeepaliveRelinquishGate {
    fn eq(&self, other: &Self) -> bool {
        self.state.load(Ordering::Acquire) == other.state.load(Ordering::Acquire)
    }
}

impl Eq for OutputLeaseKeepaliveRelinquishGate {}

impl OutputLeaseKeepaliveRelinquishGate {
    fn issued() -> Arc<Self> {
        Arc::new(Self {
            state: AtomicU8::new(RELINQUISH_GATE_ISSUED),
        })
    }

    fn consume_for_registry(&self) -> bool {
        self.state
            .compare_exchange(
                RELINQUISH_GATE_ISSUED,
                RELINQUISH_GATE_CONSUMED_BY_REGISTRY,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .is_ok()
    }

    fn is_issued(&self) -> bool {
        self.state.load(Ordering::Acquire) == RELINQUISH_GATE_ISSUED
    }

    /// Consume the permit only when an adapter failed before invoking the
    /// registry CAS. A later registry receipt cannot be fabricated from this
    /// aborted gate, and the manager retains Faulted for a fresh retry.
    fn abort_before_registry(&self) -> bool {
        self.state
            .compare_exchange(
                RELINQUISH_GATE_ISSUED,
                RELINQUISH_GATE_ABORTED_BEFORE_REGISTRY,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .is_ok()
    }

    fn invalidate(&self) {
        self.state
            .store(RELINQUISH_GATE_INVALIDATED, Ordering::Release);
    }
}

impl PartialEq for OutputLeaseKeepaliveRenewalGate {
    fn eq(&self, other: &Self) -> bool {
        self.state.load(Ordering::Acquire) == other.state.load(Ordering::Acquire)
    }
}

impl Eq for OutputLeaseKeepaliveRenewalGate {}

impl OutputLeaseKeepaliveRenewalGate {
    fn issued() -> Arc<Self> {
        Arc::new(Self {
            state: AtomicU8::new(RENEWAL_GATE_ISSUED),
        })
    }

    fn claim_for_registry(&self) -> bool {
        self.state
            .compare_exchange(
                RENEWAL_GATE_ISSUED,
                RENEWAL_GATE_CLAIMED_BY_REGISTRY,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .is_ok()
    }

    fn invalidate(&self) {
        let _ = self.state.compare_exchange(
            RENEWAL_GATE_ISSUED,
            RENEWAL_GATE_INVALIDATED,
            Ordering::AcqRel,
            Ordering::Acquire,
        );
    }

    fn is_issued(&self) -> bool {
        self.state.load(Ordering::Acquire) == RENEWAL_GATE_ISSUED
    }
}

const FAIL_STOP_GATE_ISSUED: u8 = 0;
const FAIL_STOP_GATE_EXECUTING: u8 = 1;
const FAIL_STOP_GATE_EXECUTED: u8 = 2;
const FAIL_STOP_GATE_COMPLETED: u8 = 3;
const FAIL_STOP_GATE_INVALIDATED: u8 = 4;

#[derive(Debug)]
struct OutputLeaseKeepaliveFailStopExecutionGate {
    state: AtomicU8,
}

impl PartialEq for OutputLeaseKeepaliveFailStopExecutionGate {
    fn eq(&self, other: &Self) -> bool {
        self.state.load(Ordering::Acquire) == other.state.load(Ordering::Acquire)
    }
}

impl Eq for OutputLeaseKeepaliveFailStopExecutionGate {}

impl OutputLeaseKeepaliveFailStopExecutionGate {
    fn issued() -> Arc<Self> {
        Arc::new(Self {
            state: AtomicU8::new(FAIL_STOP_GATE_ISSUED),
        })
    }

    fn claim_execution(&self) -> bool {
        self.state
            .compare_exchange(
                FAIL_STOP_GATE_ISSUED,
                FAIL_STOP_GATE_EXECUTING,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .is_ok()
    }

    fn mark_executed(&self) -> bool {
        self.state
            .compare_exchange(
                FAIL_STOP_GATE_EXECUTING,
                FAIL_STOP_GATE_EXECUTED,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .is_ok()
    }

    fn mark_completed(&self) -> bool {
        self.state
            .compare_exchange(
                FAIL_STOP_GATE_EXECUTED,
                FAIL_STOP_GATE_COMPLETED,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .is_ok()
    }

    /// A newer, durably committed cleanup-only Enable has superseded this
    /// plan's exact authority. Existing execution may still be finishing
    /// all-deny ports, but it can no longer produce a manager completion.
    fn invalidate(&self) {
        self.state
            .store(FAIL_STOP_GATE_INVALIDATED, Ordering::Release);
    }
}

const SUPERSESSION_GATE_ISSUED: u8 = 0;
const SUPERSESSION_GATE_CLAIMED_BY_REGISTRY: u8 = 1;
const SUPERSESSION_GATE_ABORTED_BEFORE_REGISTRY: u8 = 2;
const SUPERSESSION_GATE_INVALIDATED: u8 = 3;

/// One-shot ingress fence for a deferred B/C registry proof. The gate lives
/// in the current Faulted run as well as its exported capability so a newer
/// cleanup-only durable Enable can make an unsubmitted proof incapable of
/// reaching the registry at all.
#[derive(Debug)]
struct OutputLeaseKeepaliveSupersessionGate {
    state: AtomicU8,
}

impl PartialEq for OutputLeaseKeepaliveSupersessionGate {
    fn eq(&self, other: &Self) -> bool {
        self.state.load(Ordering::Acquire) == other.state.load(Ordering::Acquire)
    }
}

impl Eq for OutputLeaseKeepaliveSupersessionGate {}

impl OutputLeaseKeepaliveSupersessionGate {
    fn issued() -> Arc<Self> {
        Arc::new(Self {
            state: AtomicU8::new(SUPERSESSION_GATE_ISSUED),
        })
    }

    fn claim_for_registry(&self) -> bool {
        self.state
            .compare_exchange(
                SUPERSESSION_GATE_ISSUED,
                SUPERSESSION_GATE_CLAIMED_BY_REGISTRY,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .is_ok()
    }

    fn abort_before_registry(&self) -> bool {
        self.state
            .compare_exchange(
                SUPERSESSION_GATE_ISSUED,
                SUPERSESSION_GATE_ABORTED_BEFORE_REGISTRY,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .is_ok()
    }

    fn is_claimed_by_registry(&self) -> bool {
        self.state.load(Ordering::Acquire) == SUPERSESSION_GATE_CLAIMED_BY_REGISTRY
    }

    fn invalidate(&self) {
        self.state
            .store(SUPERSESSION_GATE_INVALIDATED, Ordering::Release);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OutputLeaseKeepaliveArmedState {
    run_id: OutputLeaseKeepaliveRunId,
    identity: OutputLeaseKeepaliveIdentity,
    lease: OutputLeaseKeepaliveExactBothLease,
    registry_claim: OutputLeaseKeepaliveRegistryClaim,
    durable_receipt_issuance: u64,
    next_renew_at_ms: u64,
    last_observed_now_ms: u64,
    renewal_in_flight: bool,
    renewal_attempt: u64,
    renewal_gate: Option<Arc<OutputLeaseKeepaliveRenewalGate>>,
}

impl OutputLeaseKeepaliveArmedState {
    pub(crate) fn run_id(&self) -> OutputLeaseKeepaliveRunId {
        self.run_id
    }

    /// Exact identity captured from the durable Enable receipt. Runtime
    /// workers use this only to correlate a successful automatic-boundary
    /// fail-stop with its matching opaque relinquish permit.
    pub(crate) fn identity(&self) -> &OutputLeaseKeepaliveIdentity {
        &self.identity
    }

    pub(crate) fn lease(&self) -> &OutputLeaseKeepaliveExactBothLease {
        &self.lease
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OutputLeaseKeepaliveFaultState {
    run_id: OutputLeaseKeepaliveRunId,
    /// Monotonic within one run. An older best-effort pass must never finish a
    /// later retry merely because the logical run and physical plan match.
    fail_stop_attempt: u64,
    identity: OutputLeaseKeepaliveIdentity,
    lease: OutputLeaseKeepaliveExactBothLease,
    registry_claim: OutputLeaseKeepaliveRegistryClaim,
    boundary: OutputLeaseKeepaliveBoundary,
    /// `None` only while an ordered best-effort fail-stop is in progress.
    fail_stop_report: Option<OutputLeaseKeepaliveFailStopReport>,
    /// Error which caused fail-stop before the ordered physical steps ran.
    trigger_error: Option<String>,
    /// Last failed ordered step, or the trigger error if all steps succeeded.
    last_error: Option<String>,
    fail_stop_gate: Arc<OutputLeaseKeepaliveFailStopExecutionGate>,
    /// A successful faulted run may authorize at most one exact registry
    /// relinquish CAS. Losing or failing that capability leaves the route
    /// Faulted rather than minting a replayable release token.
    relinquish_permit_issued: bool,
    /// Pointer-identity companion for the one-shot permit. A cleanup-only
    /// durable successor explicitly invalidates this gate before it replaces
    /// an inconsistent partially recovered manager state, so an old permit
    /// cannot reach the registry after its authority was superseded.
    relinquish_permit_gate: Option<Arc<OutputLeaseKeepaliveRelinquishGate>>,
    /// A newer durable Enable may supersede this old Faulted run only through
    /// one exact registry proof. While such a proof is in flight, neither a
    /// second proof nor a relinquish permit may be minted for this run.
    supersession_attempt: u64,
    supersession_in_flight: bool,
    /// Pointer-identity companion for the one in-flight deferred B/C proof.
    /// It prevents a capability that left the lock before cleanup superseded
    /// the run from entering the parent registry later.
    supersession_gate: Option<Arc<OutputLeaseKeepaliveSupersessionGate>>,
}

impl OutputLeaseKeepaliveFaultState {
    pub(crate) fn run_id(&self) -> OutputLeaseKeepaliveRunId {
        self.run_id
    }

    /// Exact identity captured for this Faulting/Faulted run. It is read-only
    /// correlation data, never a capability or a substitute for the opaque
    /// registry permit/receipt boundaries.
    pub(crate) fn identity(&self) -> &OutputLeaseKeepaliveIdentity {
        &self.identity
    }

    pub(crate) fn lease(&self) -> &OutputLeaseKeepaliveExactBothLease {
        &self.lease
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OutputLeaseKeepaliveSnapshot {
    pub(crate) state: OutputLeaseKeepaliveState,
    pub(crate) armed: Option<OutputLeaseKeepaliveArmedState>,
    pub(crate) fault: Option<OutputLeaseKeepaliveFaultState>,
}

/// Test-only model of the retired generic lifecycle entrypoint. Production
/// routing exposes only the exact Relinquish bridge below.
#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OutputLeaseKeepaliveManualOperation {
    Renew,
    Recover,
    ForceTransfer,
    Relinquish,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum OutputLeaseKeepaliveManualOperationDecision {
    /// The target is not managed by this keepalive instance.
    AllowUnmanaged,
    RejectManagedLease,
    /// The active run was atomically fenced into fail-stop. Execute and
    /// complete this exact plan before asking to relinquish again.
    BeginFailStop(OutputLeaseKeepaliveFailStopPlan),
    /// A matching run is already faulting. Only this current plan may finish
    /// before a relinquish permit can exist.
    RequireFailStop(OutputLeaseKeepaliveFailStopPlan),
    /// The matching run reached Faulted only after a full canonical fail-stop.
    /// The opaque permit correlates a later registry relinquish to that run.
    AllowRelinquish(OutputLeaseKeepaliveRelinquishPermit),
}

/// Opaque evidence for the adapter's eventual registry relinquish. It is
/// emitted only from a complete matching Faulted run, never while a managed
/// lease is Armed or Faulting.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct OutputLeaseKeepaliveRelinquishPermit {
    run_id: OutputLeaseKeepaliveRunId,
    identity: OutputLeaseKeepaliveIdentity,
    lease: OutputLeaseKeepaliveExactBothLease,
    registry_claim: OutputLeaseKeepaliveRegistryClaim,
    gate: Arc<OutputLeaseKeepaliveRelinquishGate>,
}

impl OutputLeaseKeepaliveRelinquishPermit {
    pub(super) fn consume_for_registry(self) -> OutputLeaseKeepaliveRegistryRelinquishIngress {
        let claim = OutputLeaseKeepaliveRegistryRelinquishClaim {
            run_id: self.run_id,
            identity: self.identity,
            lease: self.lease,
            registry_claim: self.registry_claim,
        };
        if self.gate.consume_for_registry() {
            OutputLeaseKeepaliveRegistryRelinquishIngress::Claimed(claim)
        } else {
            OutputLeaseKeepaliveRegistryRelinquishIngress::Rejected(
                OutputLeaseKeepaliveRelinquishCasReceipt::Rejected(
                    OutputLeaseKeepaliveRelinquishFailureReceipt::from_registry_claim(
                        claim,
                        internal_port_error(OutputLeaseKeepaliveError::StaleRun),
                    ),
                ),
            )
        }
    }
}

#[derive(Debug, Clone)]
pub(super) struct OutputLeaseKeepaliveRegistryRelinquishClaim {
    run_id: OutputLeaseKeepaliveRunId,
    identity: OutputLeaseKeepaliveIdentity,
    lease: OutputLeaseKeepaliveExactBothLease,
    registry_claim: OutputLeaseKeepaliveRegistryClaim,
}

/// Parent-only ingress after a one-shot relinquish permit was consumed. A
/// replayed permit never reaches registry mutation and returns a manager-
/// completable rejected receipt instead.
pub(super) enum OutputLeaseKeepaliveRegistryRelinquishIngress {
    Claimed(OutputLeaseKeepaliveRegistryRelinquishClaim),
    Rejected(OutputLeaseKeepaliveRelinquishCasReceipt),
}

impl OutputLeaseKeepaliveRegistryRelinquishClaim {
    pub(super) fn identity(&self) -> &OutputLeaseKeepaliveIdentity {
        &self.identity
    }

    pub(super) fn lease(&self) -> &OutputLeaseKeepaliveExactBothLease {
        &self.lease
    }

    pub(super) fn registry_claim(&self) -> &OutputLeaseKeepaliveRegistryClaim {
        &self.registry_claim
    }
}

/// Opaque receipt required to clear a managed Faulted run. There is no
/// production constructor outside the parent registry: only an exact parent
/// CAS success can construct it. It proves the matching lease moved to an
/// unclaimed `generation + 1` snapshot with owner/resources/deadline cleared.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct OutputLeaseKeepaliveExactRelinquishReceipt {
    run_id: OutputLeaseKeepaliveRunId,
    identity: OutputLeaseKeepaliveIdentity,
    lease: OutputLeaseKeepaliveExactBothLease,
    registry_claim: OutputLeaseKeepaliveRegistryClaim,
    prior_generation: u64,
    released_generation: u64,
    released_unclaimed: bool,
    released_owner_cleared: bool,
    released_resources_cleared: bool,
    released_expiry_cleared: bool,
    released_index_cleared: bool,
}

impl OutputLeaseKeepaliveExactRelinquishReceipt {
    pub(super) fn issue_after_verified_exact_managed_registry_relinquish(
        claim: OutputLeaseKeepaliveRegistryRelinquishClaim,
        released_generation: u64,
        released_unclaimed: bool,
        released_owner_cleared: bool,
        released_resources_cleared: bool,
        released_expiry_cleared: bool,
        released_index_cleared: bool,
    ) -> Result<Self, OutputLeaseKeepaliveError> {
        let prior_generation = claim.lease.generation;
        let expected_released_generation = prior_generation
            .checked_add(1)
            .ok_or(OutputLeaseKeepaliveError::RelinquishGenerationOverflow)?;
        if released_generation != expected_released_generation
            || !released_unclaimed
            || !released_owner_cleared
            || !released_resources_cleared
            || !released_expiry_cleared
            || !released_index_cleared
        {
            return Err(OutputLeaseKeepaliveError::InvalidRelinquishReceipt);
        }
        Ok(Self {
            run_id: claim.run_id,
            identity: claim.identity,
            lease: claim.lease,
            registry_claim: claim.registry_claim,
            prior_generation,
            released_generation,
            released_unclaimed,
            released_owner_cleared,
            released_resources_cleared,
            released_expiry_cleared,
            released_index_cleared,
        })
    }

    fn matches_fault(&self, fault: &OutputLeaseKeepaliveFaultState) -> bool {
        self.run_id == fault.run_id
            && self.identity == fault.identity
            && self.lease == fault.lease
            && self.registry_claim == fault.registry_claim
            && self.prior_generation == fault.lease.generation
            && fault
                .lease
                .generation
                .checked_add(1)
                .is_some_and(|expected| self.released_generation == expected)
            && self.released_unclaimed
            && self.released_owner_cleared
            && self.released_resources_cleared
            && self.released_expiry_cleared
            && self.released_index_cleared
    }

    /// Read-only response evidence for the public control-plane completion.
    /// The managed CAS deliberately bypasses the public registry audit queue,
    /// so it has no invented "release audit sequence". The exposed sequence is
    /// instead the canonical durable Enable audit sequence that originally
    /// authorized this exact lease/run; its field name makes that distinction
    /// explicit on the wire.
    ///
    /// This borrows no capability and does not consume the receipt. An adapter
    /// can capture it before handing the opaque receipt to
    /// `complete_relinquish_cas_receipt`, then use the detached value to form a
    /// successful control-plane response only if manager completion succeeds.
    pub(crate) fn response_evidence(&self) -> OutputLeaseKeepaliveExactRelinquishResponseEvidence {
        OutputLeaseKeepaliveExactRelinquishResponseEvidence {
            lease_id: self.lease.lease_id.clone(),
            prior_generation: self.prior_generation,
            released_generation: self.released_generation,
            durable_enable_audit_sequence: self.registry_claim.durable_receipt_issuance,
        }
    }
}

/// Non-authorizing data suitable for a successful managed-release wire result.
/// It intentionally omits owner/project/identity and all capabilities: callers
/// can report the exact lease transition but cannot use this DTO to renew,
/// relinquish, complete, or re-arm anything.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OutputLeaseKeepaliveExactRelinquishResponseEvidence {
    lease_id: String,
    prior_generation: u64,
    released_generation: u64,
    durable_enable_audit_sequence: u64,
}

impl OutputLeaseKeepaliveExactRelinquishResponseEvidence {
    pub(crate) fn lease_id(&self) -> &str {
        &self.lease_id
    }

    pub(crate) fn prior_generation(&self) -> u64 {
        self.prior_generation
    }

    pub(crate) fn released_generation(&self) -> u64 {
        self.released_generation
    }

    /// Correlation to the durable Enable receipt audit record, not a synthetic
    /// managed-release audit record (the managed CAS has no public audit row).
    pub(crate) fn durable_enable_audit_sequence(&self) -> u64 {
        self.durable_enable_audit_sequence
    }
}

/// Opaque exact-registry CAS result. The registry always consumes the permit
/// and returns a receipt bound to that same faulted run, even for expiry or
/// CAS rejection. The manager alone decides whether to clear Faulted or make
/// one later serialized retry admissible.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum OutputLeaseKeepaliveRelinquishCasReceipt {
    Released(OutputLeaseKeepaliveExactRelinquishReceipt),
    Rejected(OutputLeaseKeepaliveRelinquishFailureReceipt),
}

impl OutputLeaseKeepaliveRelinquishCasReceipt {
    /// Returns wire-safe exact release evidence only for a verified registry
    /// success. Rejected CAS outcomes have no success result to serialize.
    pub(crate) fn exact_release_response_evidence(
        &self,
    ) -> Option<OutputLeaseKeepaliveExactRelinquishResponseEvidence> {
        match self {
            Self::Released(receipt) => Some(receipt.response_evidence()),
            Self::Rejected(_) => None,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct OutputLeaseKeepaliveRelinquishFailureReceipt {
    run_id: OutputLeaseKeepaliveRunId,
    identity: OutputLeaseKeepaliveIdentity,
    lease: OutputLeaseKeepaliveExactBothLease,
    registry_claim: OutputLeaseKeepaliveRegistryClaim,
    error: OutputLeaseKeepalivePortError,
}

impl OutputLeaseKeepaliveRelinquishFailureReceipt {
    pub(super) fn from_registry_claim(
        claim: OutputLeaseKeepaliveRegistryRelinquishClaim,
        error: OutputLeaseKeepalivePortError,
    ) -> Self {
        Self {
            run_id: claim.run_id,
            identity: claim.identity,
            lease: claim.lease,
            registry_claim: claim.registry_claim,
            error,
        }
    }

    fn matches_fault(&self, fault: &OutputLeaseKeepaliveFaultState) -> bool {
        self.run_id == fault.run_id
            && self.identity == fault.identity
            && self.lease == fault.lease
            && self.registry_claim == fault.registry_claim
    }
}

/// One-shot manager-issued authority to prove that a fully stopped old run has
/// been superseded by a separate current durable Enable. It is deliberately
/// non-Clone and has no public constructor: the parent registry receives it
/// only after the manager verified a zero-failure canonical A fail-stop.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct OutputLeaseKeepaliveSupersessionCapability {
    old_run_id: OutputLeaseKeepaliveRunId,
    old_fail_stop_attempt: u64,
    supersession_attempt: u64,
    old_identity: OutputLeaseKeepaliveIdentity,
    old_lease: OutputLeaseKeepaliveExactBothLease,
    old_registry_claim: OutputLeaseKeepaliveRegistryClaim,
    new_identity: OutputLeaseKeepaliveIdentity,
    new_lease: OutputLeaseKeepaliveExactBothLease,
    new_registry_claim: OutputLeaseKeepaliveRegistryClaim,
    gate: Arc<OutputLeaseKeepaliveSupersessionGate>,
}

impl OutputLeaseKeepaliveSupersessionCapability {
    pub(super) fn consume_for_registry(self) -> OutputLeaseKeepaliveRegistrySupersessionIngress {
        let claim = OutputLeaseKeepaliveRegistrySupersessionClaim {
            old_run_id: self.old_run_id,
            old_fail_stop_attempt: self.old_fail_stop_attempt,
            supersession_attempt: self.supersession_attempt,
            old_identity: self.old_identity,
            old_lease: self.old_lease,
            old_registry_claim: self.old_registry_claim,
            new_identity: self.new_identity,
            new_lease: self.new_lease,
            new_registry_claim: self.new_registry_claim,
            gate: self.gate,
        };
        if claim.gate.claim_for_registry() {
            OutputLeaseKeepaliveRegistrySupersessionIngress::Claimed(claim)
        } else {
            OutputLeaseKeepaliveRegistrySupersessionIngress::Rejected(
                OutputLeaseKeepaliveSupersessionCasReceipt::Rejected(
                    OutputLeaseKeepaliveSupersessionFailureReceipt::from_registry_claim(
                        claim,
                        internal_port_error(OutputLeaseKeepaliveError::StaleRun),
                    ),
                ),
            )
        }
    }

    fn matches_fault(&self, fault: &OutputLeaseKeepaliveFaultState) -> bool {
        self.old_run_id == fault.run_id
            && self.old_fail_stop_attempt == fault.fail_stop_attempt
            && self.supersession_attempt == fault.supersession_attempt
            && self.old_identity == fault.identity
            && self.old_lease == fault.lease
            && self.old_registry_claim == fault.registry_claim
            && fault.supersession_in_flight
            && fault
                .fail_stop_report
                .as_ref()
                .is_some_and(OutputLeaseKeepaliveFailStopReport::is_successful_canonical)
            && fault
                .supersession_gate
                .as_ref()
                .is_some_and(|gate| Arc::ptr_eq(gate, &self.gate))
    }

    fn matches_deferred(&self, deferred: &OutputLeaseKeepaliveDeferredDurableEnable) -> bool {
        deferred.matches_components(
            &self.new_identity,
            &self.new_lease,
            &self.new_registry_claim,
        )
    }
}

/// Parent-registry-only view of a consumed supersession capability. Raw owner,
/// project, lease, generation, run, or durable-receipt arguments never enter
/// the proof boundary from its caller.
#[derive(Debug, Clone)]
pub(super) struct OutputLeaseKeepaliveRegistrySupersessionClaim {
    old_run_id: OutputLeaseKeepaliveRunId,
    old_fail_stop_attempt: u64,
    supersession_attempt: u64,
    old_identity: OutputLeaseKeepaliveIdentity,
    old_lease: OutputLeaseKeepaliveExactBothLease,
    old_registry_claim: OutputLeaseKeepaliveRegistryClaim,
    new_identity: OutputLeaseKeepaliveIdentity,
    new_lease: OutputLeaseKeepaliveExactBothLease,
    new_registry_claim: OutputLeaseKeepaliveRegistryClaim,
    gate: Arc<OutputLeaseKeepaliveSupersessionGate>,
}

/// Parent-only ingress for the one-shot deferred proof. A stale or cleanup-
/// invalidated capability returns a manager-completable rejection before the
/// registry can inspect or mutate any durable state.
pub(super) enum OutputLeaseKeepaliveRegistrySupersessionIngress {
    Claimed(OutputLeaseKeepaliveRegistrySupersessionClaim),
    Rejected(OutputLeaseKeepaliveSupersessionCasReceipt),
}

impl OutputLeaseKeepaliveRegistrySupersessionClaim {
    pub(super) fn old_identity(&self) -> &OutputLeaseKeepaliveIdentity {
        &self.old_identity
    }

    pub(super) fn old_lease(&self) -> &OutputLeaseKeepaliveExactBothLease {
        &self.old_lease
    }

    pub(super) fn old_registry_claim(&self) -> &OutputLeaseKeepaliveRegistryClaim {
        &self.old_registry_claim
    }

    pub(super) fn new_identity(&self) -> &OutputLeaseKeepaliveIdentity {
        &self.new_identity
    }

    pub(super) fn new_lease(&self) -> &OutputLeaseKeepaliveExactBothLease {
        &self.new_lease
    }

    pub(super) fn new_registry_claim(&self) -> &OutputLeaseKeepaliveRegistryClaim {
        &self.new_registry_claim
    }
}

/// Opaque result of the registry's non-mutating supersession proof. A
/// successful proof establishes both that A is no longer current exact
/// authority and that B is the current active durable exact-Both authority.
/// A failure keeps A Faulted; it can never clear or arm B.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum OutputLeaseKeepaliveSupersessionCasReceipt {
    Proven(OutputLeaseKeepaliveSupersessionSuccessReceipt),
    Rejected(OutputLeaseKeepaliveSupersessionFailureReceipt),
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct OutputLeaseKeepaliveSupersessionSuccessReceipt {
    old_run_id: OutputLeaseKeepaliveRunId,
    old_fail_stop_attempt: u64,
    supersession_attempt: u64,
    old_identity: OutputLeaseKeepaliveIdentity,
    old_lease: OutputLeaseKeepaliveExactBothLease,
    old_registry_claim: OutputLeaseKeepaliveRegistryClaim,
    new_identity: OutputLeaseKeepaliveIdentity,
    new_lease: OutputLeaseKeepaliveExactBothLease,
    new_registry_claim: OutputLeaseKeepaliveRegistryClaim,
    gate: Arc<OutputLeaseKeepaliveSupersessionGate>,
}

impl OutputLeaseKeepaliveSupersessionSuccessReceipt {
    pub(super) fn from_registry_verified_current_success(
        claim: OutputLeaseKeepaliveRegistrySupersessionClaim,
    ) -> Self {
        Self {
            old_run_id: claim.old_run_id,
            old_fail_stop_attempt: claim.old_fail_stop_attempt,
            supersession_attempt: claim.supersession_attempt,
            old_identity: claim.old_identity,
            old_lease: claim.old_lease,
            old_registry_claim: claim.old_registry_claim,
            new_identity: claim.new_identity,
            new_lease: claim.new_lease,
            new_registry_claim: claim.new_registry_claim,
            gate: claim.gate,
        }
    }

    fn matches_fault(&self, fault: &OutputLeaseKeepaliveFaultState) -> bool {
        self.old_run_id == fault.run_id
            && self.old_fail_stop_attempt == fault.fail_stop_attempt
            && self.supersession_attempt == fault.supersession_attempt
            && self.old_identity == fault.identity
            && self.old_lease == fault.lease
            && self.old_registry_claim == fault.registry_claim
            && fault.supersession_in_flight
            && fault
                .fail_stop_report
                .as_ref()
                .is_some_and(OutputLeaseKeepaliveFailStopReport::is_successful_canonical)
            && fault
                .supersession_gate
                .as_ref()
                .is_some_and(|gate| Arc::ptr_eq(gate, &self.gate))
            && self.gate.is_claimed_by_registry()
    }

    fn matches_deferred(&self, deferred: &OutputLeaseKeepaliveDeferredDurableEnable) -> bool {
        deferred.matches_components(
            &self.new_identity,
            &self.new_lease,
            &self.new_registry_claim,
        )
    }

    fn into_new_components(
        self,
    ) -> (
        OutputLeaseKeepaliveIdentity,
        OutputLeaseKeepaliveExactBothLease,
        OutputLeaseKeepaliveRegistryClaim,
    ) {
        (self.new_identity, self.new_lease, self.new_registry_claim)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct OutputLeaseKeepaliveSupersessionFailureReceipt {
    old_run_id: OutputLeaseKeepaliveRunId,
    old_fail_stop_attempt: u64,
    supersession_attempt: u64,
    old_identity: OutputLeaseKeepaliveIdentity,
    old_lease: OutputLeaseKeepaliveExactBothLease,
    old_registry_claim: OutputLeaseKeepaliveRegistryClaim,
    new_identity: OutputLeaseKeepaliveIdentity,
    new_lease: OutputLeaseKeepaliveExactBothLease,
    new_registry_claim: OutputLeaseKeepaliveRegistryClaim,
    gate: Arc<OutputLeaseKeepaliveSupersessionGate>,
    error: OutputLeaseKeepalivePortError,
}

impl OutputLeaseKeepaliveSupersessionFailureReceipt {
    pub(super) fn from_registry_claim(
        claim: OutputLeaseKeepaliveRegistrySupersessionClaim,
        error: OutputLeaseKeepalivePortError,
    ) -> Self {
        Self {
            old_run_id: claim.old_run_id,
            old_fail_stop_attempt: claim.old_fail_stop_attempt,
            supersession_attempt: claim.supersession_attempt,
            old_identity: claim.old_identity,
            old_lease: claim.old_lease,
            old_registry_claim: claim.old_registry_claim,
            new_identity: claim.new_identity,
            new_lease: claim.new_lease,
            new_registry_claim: claim.new_registry_claim,
            gate: claim.gate,
            error,
        }
    }

    fn matches_fault(&self, fault: &OutputLeaseKeepaliveFaultState) -> bool {
        self.old_run_id == fault.run_id
            && self.old_fail_stop_attempt == fault.fail_stop_attempt
            && self.supersession_attempt == fault.supersession_attempt
            && self.old_identity == fault.identity
            && self.old_lease == fault.lease
            && self.old_registry_claim == fault.registry_claim
            && fault.supersession_in_flight
            && fault
                .fail_stop_report
                .as_ref()
                .is_some_and(OutputLeaseKeepaliveFailStopReport::is_successful_canonical)
            && fault
                .supersession_gate
                .as_ref()
                .is_some_and(|gate| Arc::ptr_eq(gate, &self.gate))
            && self.gate.is_claimed_by_registry()
    }

    fn matches_deferred(&self, deferred: &OutputLeaseKeepaliveDeferredDurableEnable) -> bool {
        deferred.matches_components(
            &self.new_identity,
            &self.new_lease,
            &self.new_registry_claim,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum OutputLeaseKeepaliveError {
    InvalidLeaseAuthority,
    InvalidIdentity,
    InvalidDurableEnableEvidence,
    DurableEnableAdmissionFailed,
    InvalidErrorDetail,
    InvalidRelinquishReceipt,
    RelinquishGenerationOverflow,
    ClockOverflow,
    RunIdExhausted,
    NotArmed,
    RenewalAlreadyInFlight,
    StaleRun,
    UnexpectedState,
    ClockRollback,
    RenewalLeaseIdentityChanged,
    RenewalGenerationDidNotAdvance,
    RenewalGenerationSkipped,
    RenewalGenerationOverflow,
    RenewalAttemptExhausted,
    SupersessionAttemptExhausted,
    IncompleteFailStopReport,
}

impl fmt::Display for OutputLeaseKeepaliveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidLeaseAuthority => "invalid managed exact-Both output-lease authority",
            Self::InvalidIdentity => "invalid managed output-lease identity",
            Self::InvalidDurableEnableEvidence => {
                "durable Enable evidence does not match canonical output-lease authority"
            }
            Self::DurableEnableAdmissionFailed => {
                "durable exact-Both Enable could not enter managed keepalive admission"
            }
            Self::InvalidErrorDetail => "managed output-lease error detail exceeds its byte bound",
            Self::InvalidRelinquishReceipt => {
                "managed exact registry relinquish receipt is invalid"
            }
            Self::RelinquishGenerationOverflow => {
                "managed exact registry relinquish generation overflowed"
            }
            Self::ClockOverflow => "managed output-lease keepalive clock overflow",
            Self::RunIdExhausted => "managed output-lease keepalive run identifier exhausted",
            Self::NotArmed => "managed output-lease keepalive is not armed",
            Self::RenewalAlreadyInFlight => "managed output-lease renewal is already in flight",
            Self::StaleRun => "managed output-lease keepalive run is stale",
            Self::UnexpectedState => "managed output-lease keepalive state is not admissible",
            Self::ClockRollback => "managed output-lease keepalive monotonic clock rolled back",
            Self::RenewalLeaseIdentityChanged => {
                "renewal returned a different output-lease identity"
            }
            Self::RenewalGenerationDidNotAdvance => {
                "renewal did not advance output-lease generation"
            }
            Self::RenewalGenerationSkipped => "renewal skipped output-lease generation",
            Self::RenewalGenerationOverflow => "renewal output-lease generation overflowed",
            Self::RenewalAttemptExhausted => "renewal attempt identifier exhausted",
            Self::SupersessionAttemptExhausted => {
                "managed output-lease supersession attempt identifier exhausted"
            }
            Self::IncompleteFailStopReport => {
                "fail-stop report did not execute the full ordered plan"
            }
        })
    }
}

impl std::error::Error for OutputLeaseKeepaliveError {}

/// Validated human-readable failure detail. The inner string is intentionally
/// private so every retained diagnostic obeys the same UTF-8 byte bound as the
/// lease identities. Adapters must reject, never truncate, oversized details.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OutputLeaseKeepaliveFailureDetail(String);

impl OutputLeaseKeepaliveFailureDetail {
    pub(crate) fn new(value: impl Into<String>) -> Result<Self, OutputLeaseKeepaliveError> {
        let value = value.into();
        if !bounded_nonempty(&value, MAX_OUTPUT_LEASE_KEEPALIVE_ERROR_BYTES) {
            return Err(OutputLeaseKeepaliveError::InvalidErrorDetail);
        }
        Ok(Self(value))
    }

    fn as_str(&self) -> &str {
        &self.0
    }
}

/// A port cannot pass an unchecked OS/driver error into durable manager state.
/// Callers must reject an over-bound detail before they can request fail-stop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum OutputLeaseKeepalivePortError {
    Detail(OutputLeaseKeepaliveFailureDetail),
}

impl OutputLeaseKeepalivePortError {
    pub(crate) fn detail(value: impl Into<String>) -> Result<Self, OutputLeaseKeepaliveError> {
        Ok(Self::Detail(OutputLeaseKeepaliveFailureDetail::new(value)?))
    }

    fn stored_detail(&self) -> String {
        match self {
            Self::Detail(detail) => detail.as_str().to_owned(),
        }
    }
}

/// Production adapters must never truncate, normalize, or persist arbitrary
/// driver/OS diagnostics.  This static evidence records that the supplied
/// diagnostic was rejected while still allowing the already-required physical
/// fail-stop to continue with a bounded, actionable failure class.
pub(crate) const OUTPUT_LEASE_KEEPALIVE_DIAGNOSTIC_REJECTED: &str =
    "Managed output-lease diagnostic evidence was rejected as invalid or oversized";

pub(crate) fn output_lease_keepalive_adapter_port_error(
    detail: impl Into<String>,
) -> OutputLeaseKeepalivePortError {
    match OutputLeaseKeepalivePortError::detail(detail) {
        Ok(error) => error,
        Err(OutputLeaseKeepaliveError::InvalidErrorDetail) => {
            OutputLeaseKeepalivePortError::detail(OUTPUT_LEASE_KEEPALIVE_DIAGNOSTIC_REJECTED)
                .expect("static rejected-diagnostic detail fits the keepalive bound")
        }
        Err(error) => internal_port_error(error),
    }
}

fn internal_port_error(error: OutputLeaseKeepaliveError) -> OutputLeaseKeepalivePortError {
    OutputLeaseKeepalivePortError::detail(error.to_string())
        .expect("static keepalive error details fit the bounded diagnostic field")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OutputLeaseKeepaliveWait {
    pub(crate) run_id: OutputLeaseKeepaliveRunId,
    pub(crate) deadline_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum OutputLeaseKeepaliveTimerAction {
    Sleep(OutputLeaseKeepaliveWait),
    RenewDue(OutputLeaseKeepaliveRunId),
}

/// One-shot registry CAS capability. Only
/// `prepare_due_renewal_for_run` can issue it while an exact current run is
/// armed and marked in-flight. The parent registry consumes it, rather than
/// accepting caller-supplied owner/project/lease/generation components.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct OutputLeaseKeepaliveRenewalCapability {
    run_id: OutputLeaseKeepaliveRunId,
    renewal_attempt: u64,
    identity: OutputLeaseKeepaliveIdentity,
    lease: OutputLeaseKeepaliveExactBothLease,
    registry_claim: OutputLeaseKeepaliveRegistryClaim,
    gate: Arc<OutputLeaseKeepaliveRenewalGate>,
}

#[derive(Debug)]
pub(super) struct OutputLeaseKeepaliveRegistryRenewalClaim {
    run_id: OutputLeaseKeepaliveRunId,
    renewal_attempt: u64,
    identity: OutputLeaseKeepaliveIdentity,
    lease: OutputLeaseKeepaliveExactBothLease,
    registry_claim: OutputLeaseKeepaliveRegistryClaim,
}

impl OutputLeaseKeepaliveRegistryRenewalClaim {
    pub(super) fn identity(&self) -> &OutputLeaseKeepaliveIdentity {
        &self.identity
    }

    pub(super) fn lease(&self) -> &OutputLeaseKeepaliveExactBothLease {
        &self.lease
    }

    pub(super) fn registry_claim(&self) -> &OutputLeaseKeepaliveRegistryClaim {
        &self.registry_claim
    }
}

/// Parent-only ingress after the capability's one-shot gate has been claimed.
/// A stale/replayed capability never reaches registry mutation and is returned
/// as a manager-completable failure receipt instead.
pub(super) enum OutputLeaseKeepaliveRegistryRenewalIngress {
    Claimed(OutputLeaseKeepaliveRegistryRenewalClaim),
    Rejected(OutputLeaseKeepaliveRenewalCasReceipt),
}

impl OutputLeaseKeepaliveRenewalCapability {
    pub(super) fn consume_for_registry(self) -> OutputLeaseKeepaliveRegistryRenewalIngress {
        let claim = OutputLeaseKeepaliveRegistryRenewalClaim {
            run_id: self.run_id,
            renewal_attempt: self.renewal_attempt,
            identity: self.identity,
            lease: self.lease,
            registry_claim: self.registry_claim,
        };
        if self.gate.claim_for_registry() {
            OutputLeaseKeepaliveRegistryRenewalIngress::Claimed(claim)
        } else {
            OutputLeaseKeepaliveRegistryRenewalIngress::Rejected(
                OutputLeaseKeepaliveRenewalCasReceipt::Rejected(
                    OutputLeaseKeepaliveRenewalFailureReceipt::from_registry_claim(
                        claim,
                        internal_port_error(OutputLeaseKeepaliveError::StaleRun),
                    ),
                ),
            )
        }
    }
}

/// A registry-created completion receipt. Its constructors are parent-visible
/// only, so a caller cannot forge a CAS success/failure and complete an
/// in-flight manager renewal without consuming the issued capability.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum OutputLeaseKeepaliveRenewalCasReceipt {
    Renewed(OutputLeaseKeepaliveRenewalSuccessReceipt),
    Rejected(OutputLeaseKeepaliveRenewalFailureReceipt),
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct OutputLeaseKeepaliveRenewalSuccessReceipt {
    run_id: OutputLeaseKeepaliveRunId,
    renewal_attempt: u64,
    identity: OutputLeaseKeepaliveIdentity,
    previous_lease: OutputLeaseKeepaliveExactBothLease,
    renewed_lease: OutputLeaseKeepaliveExactBothLease,
    registry_claim: OutputLeaseKeepaliveRegistryClaim,
}

impl OutputLeaseKeepaliveRenewalSuccessReceipt {
    pub(super) fn from_registry_success(
        claim: OutputLeaseKeepaliveRegistryRenewalClaim,
        renewed_lease: OutputLeaseKeepaliveExactBothLease,
    ) -> Self {
        Self {
            run_id: claim.run_id,
            renewal_attempt: claim.renewal_attempt,
            identity: claim.identity,
            previous_lease: claim.lease,
            renewed_lease,
            registry_claim: claim.registry_claim,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct OutputLeaseKeepaliveRenewalFailureReceipt {
    run_id: OutputLeaseKeepaliveRunId,
    renewal_attempt: u64,
    identity: OutputLeaseKeepaliveIdentity,
    lease: OutputLeaseKeepaliveExactBothLease,
    registry_claim: OutputLeaseKeepaliveRegistryClaim,
    error: OutputLeaseKeepalivePortError,
}

impl OutputLeaseKeepaliveRenewalFailureReceipt {
    pub(super) fn from_registry_claim(
        claim: OutputLeaseKeepaliveRegistryRenewalClaim,
        error: OutputLeaseKeepalivePortError,
    ) -> Self {
        Self {
            run_id: claim.run_id,
            renewal_attempt: claim.renewal_attempt,
            identity: claim.identity,
            lease: claim.lease,
            registry_claim: claim.registry_claim,
            error,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OutputLeaseKeepaliveFailStopStep {
    CancelRenewal,
    RequestSafetyBlackout,
    AdvanceFailureFence,
    RetireDmxUsbArtNet,
    RetireSpoutNative,
    PersistStandbyFailed,
    RequireExplicitEnable,
}

const OUTPUT_LEASE_KEEPALIVE_FAIL_STOP_ORDER: [OutputLeaseKeepaliveFailStopStep; 7] = [
    OutputLeaseKeepaliveFailStopStep::CancelRenewal,
    OutputLeaseKeepaliveFailStopStep::RequestSafetyBlackout,
    OutputLeaseKeepaliveFailStopStep::AdvanceFailureFence,
    OutputLeaseKeepaliveFailStopStep::RetireDmxUsbArtNet,
    OutputLeaseKeepaliveFailStopStep::RetireSpoutNative,
    OutputLeaseKeepaliveFailStopStep::PersistStandbyFailed,
    OutputLeaseKeepaliveFailStopStep::RequireExplicitEnable,
];

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct OutputLeaseKeepaliveFailStopPlan {
    run_id: OutputLeaseKeepaliveRunId,
    fail_stop_attempt: u64,
    identity: OutputLeaseKeepaliveIdentity,
    lease: OutputLeaseKeepaliveExactBothLease,
    boundary: OutputLeaseKeepaliveBoundary,
    initial_error: Option<String>,
    gate: Arc<OutputLeaseKeepaliveFailStopExecutionGate>,
}

impl OutputLeaseKeepaliveFailStopPlan {
    pub(crate) fn run_id(&self) -> OutputLeaseKeepaliveRunId {
        self.run_id
    }

    pub(crate) fn identity(&self) -> &OutputLeaseKeepaliveIdentity {
        &self.identity
    }

    #[cfg(test)]
    pub(crate) fn lease(&self) -> &OutputLeaseKeepaliveExactBothLease {
        &self.lease
    }

    #[cfg(test)]
    pub(crate) fn boundary(&self) -> OutputLeaseKeepaliveBoundary {
        self.boundary
    }

    #[cfg(test)]
    pub(crate) fn initial_error(&self) -> Option<&str> {
        self.initial_error.as_deref()
    }

    #[cfg(test)]
    pub(crate) fn is_admission_failure(&self) -> bool {
        self.boundary == OutputLeaseKeepaliveBoundary::ArmAdmissionFailed
    }

    fn matches_fault(&self, fault: &OutputLeaseKeepaliveFaultState) -> bool {
        self.run_id == fault.run_id
            && self.fail_stop_attempt == fault.fail_stop_attempt
            && self.identity == fault.identity
            && self.lease == fault.lease
            && self.boundary == fault.boundary
            && self.initial_error == fault.trigger_error
            && Arc::ptr_eq(&self.gate, &fault.fail_stop_gate)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OutputLeaseKeepaliveStepFailure {
    step: OutputLeaseKeepaliveFailStopStep,
    error: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OutputLeaseKeepaliveFailStopReport {
    attempted_steps: Vec<OutputLeaseKeepaliveFailStopStep>,
    failures: Vec<OutputLeaseKeepaliveStepFailure>,
}

impl OutputLeaseKeepaliveFailStopReport {
    fn new_canonical_attempt() -> Self {
        Self {
            attempted_steps: Vec::with_capacity(OUTPUT_LEASE_KEEPALIVE_FAIL_STOP_ORDER.len()),
            failures: Vec::new(),
        }
    }

    fn is_complete_canonical(&self) -> bool {
        if self.attempted_steps.as_slice() != OUTPUT_LEASE_KEEPALIVE_FAIL_STOP_ORDER {
            return false;
        }
        let mut previous_index = None;
        for failure in &self.failures {
            if !bounded_nonempty(&failure.error, MAX_OUTPUT_LEASE_KEEPALIVE_ERROR_BYTES) {
                return false;
            }
            let Some(index) = OUTPUT_LEASE_KEEPALIVE_FAIL_STOP_ORDER
                .iter()
                .position(|step| *step == failure.step)
            else {
                return false;
            };
            if previous_index.is_some_and(|previous| previous >= index) {
                return false;
            }
            previous_index = Some(index);
        }
        true
    }

    fn is_successful_canonical(&self) -> bool {
        self.failures.is_empty() && self.is_complete_canonical()
    }

    pub(crate) fn last_error(&self) -> Option<&str> {
        self.failures.last().map(|failure| failure.error.as_str())
    }
}

/// All calls occur outside the manager lock. Fail-stop is best effort after
/// the first error: an unavailable USB retirement never prevents Spout/native
/// retirement or durable Standby Failed recording.
pub(crate) trait OutputLeaseKeepaliveFailStopPorts {
    fn cancel_renewal(
        &mut self,
        plan: &OutputLeaseKeepaliveFailStopPlan,
    ) -> Result<(), OutputLeaseKeepalivePortError>;
    fn request_safety_blackout(
        &mut self,
        plan: &OutputLeaseKeepaliveFailStopPlan,
    ) -> Result<(), OutputLeaseKeepalivePortError>;
    fn advance_failure_fence(
        &mut self,
        plan: &OutputLeaseKeepaliveFailStopPlan,
    ) -> Result<(), OutputLeaseKeepalivePortError>;
    fn retire_dmx_usb_artnet(
        &mut self,
        plan: &OutputLeaseKeepaliveFailStopPlan,
    ) -> Result<(), OutputLeaseKeepalivePortError>;
    fn retire_spout_native(
        &mut self,
        plan: &OutputLeaseKeepaliveFailStopPlan,
    ) -> Result<(), OutputLeaseKeepalivePortError>;
    fn persist_standby_failed(
        &mut self,
        plan: &OutputLeaseKeepaliveFailStopPlan,
    ) -> Result<(), OutputLeaseKeepalivePortError>;
}

/// The only object accepted by `complete_fail_stop`. It is produced by the
/// one-shot executor after the full canonical I/O sequence ran outside the
/// manager lock; callers cannot submit a separately mutable plan/report pair.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct OutputLeaseKeepaliveFailStopExecution {
    plan: OutputLeaseKeepaliveFailStopPlan,
    report: OutputLeaseKeepaliveFailStopReport,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum OutputLeaseKeepaliveFailStopExecutionOutcome {
    Executed(OutputLeaseKeepaliveFailStopExecution),
    /// A duplicate, stale, or superseded plan lost the execution claim before
    /// any physical port call. It is intentionally not an error.
    StaleNoop,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum OutputLeaseKeepaliveRenewalCompletion {
    Renewed,
    FailStop(OutputLeaseKeepaliveFailStopPlan),
    /// A previous run's I/O result arrived after it had been fenced. No
    /// current lease/output state is touched.
    StaleNoop,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OutputLeaseKeepaliveFailStopCompletion {
    Completed,
    /// A duplicate or late plan/report arrived after the captured run was
    /// completed, rearmed, or otherwise replaced. It cannot mutate current
    /// state and is intentionally not an error.
    StaleNoop,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OutputLeaseKeepaliveRelinquishCompletion {
    Completed,
    /// The exact registry CAS rejected the current one-shot permit. The run
    /// remains Faulted and may obtain one later serialized revalidation
    /// permit; it never clears authority on a failed CAS.
    RetainedForRetry,
    /// The permit belonged to a completed/rearmed/other run and cannot clear
    /// current manager state.
    StaleNoop,
}

/// Completion of the one-shot registry proof which retires a fully stopped
/// stale run A in favour of a separately current durable Enable B.  A proof
/// never releases or mutates B: only a matching proof can retire A and admit
/// B through the same arm-or-bound-fail-stop boundary as an ordinary Enable.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum OutputLeaseKeepaliveSupersessionCompletion {
    Armed(OutputLeaseKeepaliveRunId),
    /// B was proven current, but this process could not arm its worker.  The
    /// returned plan is now bound to B and must retire B before its registry
    /// authority can be released.
    FailStop(OutputLeaseKeepaliveFailStopPlan),
    /// The registry rejected a matching proof. A remains Faulted and a fresh
    /// serialized proof may be requested later; B is never armed implicitly.
    RetainedFaulted,
    /// A late/mismatched receipt arrived after the captured A run was already
    /// released, retried, or superseded. It has no state effect.
    StaleNoop,
}

/// The only safe result of accepting a durable exact-`Both` Enable receipt.
/// Callers cannot receive a bare arm error and leave a live authority without
/// either a worker run or an ordered fail-stop obligation.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum OutputLeaseKeepaliveArmOutcome {
    Armed(OutputLeaseKeepaliveRunId),
    /// The exact current durable receipt already owns the worker. No second
    /// worker spawn or physical retirement is permitted.
    AlreadyArmed(OutputLeaseKeepaliveRunId),
    /// The exact current durable receipt found a Faulting run. The adapter may
    /// resume only this immutable, one-shot plan; it must not spawn another
    /// worker or silently discard the captured fail-stop obligation.
    ResumeFailStop(OutputLeaseKeepaliveFailStopPlan),
    /// The exact current durable receipt found a successfully faulted run.
    /// The adapter must perform the matching exact registry CAS before the
    /// manager can become Idle; a permit is not by itself a completion token.
    RequireCurrentRelinquish(OutputLeaseKeepaliveRelinquishPermit),
    /// The current Faulted run already has an outstanding one-shot registry
    /// CAS permit. A duplicate hook cannot mint another capability or re-arm.
    AwaitCurrentRelease,
    /// A different newer durable authority arrived while a current run still
    /// exists. It remains opaque and unarmed while the caller finishes the
    /// supplied action for the current run; only after that run reaches Idle
    /// may the deferred evidence be retried.
    ManagedConflict(OutputLeaseKeepaliveManagedAdmissionConflict),
    /// A receipt previously consumed by this manager arrived late after its
    /// run was replaced or retired. It has no current authority effect.
    StaleNoop,
    FailStop(OutputLeaseKeepaliveFailStopPlan),
}

/// Result of consuming cleanup-only authority for a durable Enable that was
/// already committed but never reached ordinary keepalive admission. No branch
/// can create an Armed run, a renewal capability, or deferred enable state.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum OutputLeaseKeepaliveCleanupOnlyOutcome {
    /// The previously unadmitted authority now owns one canonical all-deny
    /// plan. It is Faulting, never Armed.
    FailStop(OutputLeaseKeepaliveFailStopPlan),
    /// The same authority was already Faulting, so only its existing immutable
    /// fail-stop plan may continue.
    ResumeFailStop(OutputLeaseKeepaliveFailStopPlan),
    /// The same authority already completed all-deny. Its exact registry CAS
    /// release must be attempted before any later transition.
    RequireCurrentRelinquish(OutputLeaseKeepaliveRelinquishPermit),
    /// A matching Faulted run already has an in-flight exact release or a
    /// deferred supersession. No second capability may be minted.
    AwaitCurrentRelease,
    /// This durable receipt was previously consumed or is older than the
    /// monotonic issuance high-water mark.
    StaleNoop,
}

/// Opaque current-run action for a newer durable Enable which the manager has
/// retained internally. The action owns the sole executable plan or exact
/// registry permit for the existing run; it cannot be forged from raw
/// identity components.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum OutputLeaseKeepaliveManagedConflictAction {
    ResumeFailStop(OutputLeaseKeepaliveFailStopPlan),
    /// The current A run completed physical fail-stop successfully and the
    /// deferred B evidence must now be submitted to the opaque registry proof
    /// before A can be retired or B can arm. This action carries no mutable
    /// authority itself; `prepare_deferred_supersession` issues it only under
    /// the manager lock.
    PrepareDeferredSupersession,
    /// A prior adapter already owns the current run's one-shot registry CAS
    /// permit. The caller must wait for its matching receipt; no new permit is
    /// available and the deferred enable remains physically unarmed.
    AwaitCurrentRelease,
}

/// The manager deliberately retains the deferred durable Enable instead of
/// returning it to an adapter. That gives one serialized authority owner for
/// B/C arbitration: a later issuance replaces an older deferred value, and a
/// stale B receipt cannot arm after C becomes the retained current candidate.
#[derive(Debug, PartialEq, Eq)]
struct OutputLeaseKeepaliveDeferredDurableEnable {
    identity: OutputLeaseKeepaliveIdentity,
    lease: OutputLeaseKeepaliveExactBothLease,
    registry_claim: OutputLeaseKeepaliveRegistryClaim,
}

impl OutputLeaseKeepaliveDeferredDurableEnable {
    fn from_evidence(evidence: OutputLeaseKeepaliveDurableEnableEvidence) -> Self {
        let (identity, lease, registry_claim) = evidence.into_components_for_arm();
        Self {
            identity,
            lease,
            registry_claim,
        }
    }

    fn durable_receipt_issuance(&self) -> u64 {
        self.registry_claim.durable_receipt_issuance
    }

    fn matches_evidence(&self, evidence: &OutputLeaseKeepaliveDurableEnableEvidence) -> bool {
        self.identity == evidence.identity
            && self.lease == evidence.lease
            && self.registry_claim == evidence.registry_claim
    }

    fn matches_components(
        &self,
        identity: &OutputLeaseKeepaliveIdentity,
        lease: &OutputLeaseKeepaliveExactBothLease,
        registry_claim: &OutputLeaseKeepaliveRegistryClaim,
    ) -> bool {
        self.identity == *identity && self.lease == *lease && self.registry_claim == *registry_claim
    }
}

/// An opaque conflict result. The manager has retained the deferred durable
/// authority itself; adapters receive only the current-run action and must
/// ask the manager to mint a proof capability at the later admissible point.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct OutputLeaseKeepaliveManagedAdmissionConflict {
    action: OutputLeaseKeepaliveManagedConflictAction,
}

impl OutputLeaseKeepaliveManagedAdmissionConflict {
    pub(crate) fn into_action(self) -> OutputLeaseKeepaliveManagedConflictAction {
        self.action
    }
}

/// Pure state holder; applications generally place it behind a mutex.
#[derive(Debug, Default)]
pub(crate) struct OutputLeaseKeepaliveManager {
    next_run_id: u64,
    highest_durable_enable_issuance: u64,
    armed: Option<OutputLeaseKeepaliveArmedState>,
    faulting: Option<OutputLeaseKeepaliveFaultState>,
    faulted: Option<OutputLeaseKeepaliveFaultState>,
    deferred: Option<OutputLeaseKeepaliveDeferredDurableEnable>,
}

impl OutputLeaseKeepaliveManager {
    pub(crate) fn snapshot(&self) -> OutputLeaseKeepaliveSnapshot {
        if let Some(armed) = &self.armed {
            return OutputLeaseKeepaliveSnapshot {
                state: OutputLeaseKeepaliveState::Armed,
                armed: Some(armed.clone()),
                fault: None,
            };
        }
        if let Some(fault) = &self.faulting {
            return OutputLeaseKeepaliveSnapshot {
                state: OutputLeaseKeepaliveState::Faulting,
                armed: None,
                fault: Some(fault.clone()),
            };
        }
        if let Some(fault) = &self.faulted {
            return OutputLeaseKeepaliveSnapshot {
                state: OutputLeaseKeepaliveState::Faulted,
                armed: None,
                fault: Some(fault.clone()),
            };
        }
        OutputLeaseKeepaliveSnapshot {
            state: OutputLeaseKeepaliveState::Idle,
            armed: None,
            fault: None,
        }
    }

    /// Phase 1: atomically consume one verified durable Enable evidence into
    /// either an armed run, a duplicate no-op, or a manager-owned ordered
    /// physical fail-stop plan. This is the only durable-Enable admission
    /// entrypoint: callers must not rely on a separate Idle preflight, because
    /// clock/run/replay/state failures can otherwise leave a live exact-`Both`
    /// authority without a worker.
    ///
    /// A normal `Armed` result is followed by worker spawn outside the lock;
    /// a spawn failure uses `record_worker_spawn_failure` for that exact run.
    /// A `FailStop` result is a normal manager-owned Faulting run: execute it
    /// outside the lock, complete it, then use the explicit automatic-boundary
    /// permit to perform the exact registry release. A conflicting newer
    /// durable authority never supersedes a current Faulting/Faulted run: it
    /// is retained only as an opaque conflict until the old run is fully
    /// retired and released.
    pub(crate) fn arm_or_return_bound_fail_stop_plan(
        &mut self,
        now_ms: u64,
        evidence: OutputLeaseKeepaliveDurableEnableEvidence,
    ) -> OutputLeaseKeepaliveArmOutcome {
        if let Some(armed) = self.armed.as_ref() {
            if Self::armed_matches_evidence(armed, &evidence) {
                return OutputLeaseKeepaliveArmOutcome::AlreadyArmed(armed.run_id);
            }
        }
        if let Some(fault) = self.faulting.as_ref() {
            if Self::fault_matches_evidence(fault, &evidence) {
                return OutputLeaseKeepaliveArmOutcome::ResumeFailStop(
                    Self::fail_stop_plan_from_fault(fault),
                );
            }
        }
        if let Some(fault) = self.faulted.as_ref() {
            if Self::fault_matches_evidence(fault, &evidence) {
                return self.current_faulted_arm_outcome();
            }
        }
        if self.armed.is_some() || self.faulting.is_some() || self.faulted.is_some() {
            return self.defer_new_durable_enable_until_current_run_released(evidence);
        }

        let durable_receipt_issuance = evidence.registry_claim.durable_receipt_issuance;
        if durable_receipt_issuance <= self.highest_durable_enable_issuance
            || self.deferred.is_some()
        {
            return OutputLeaseKeepaliveArmOutcome::StaleNoop;
        }

        let (identity, lease, registry_claim) = evidence.into_components_for_arm();
        self.arm_verified_components_or_return_bound_fail_stop(
            now_ms,
            identity,
            lease,
            registry_claim,
        )
    }

    /// Consume a distinct cleanup-only durable Enable evidence after ordinary
    /// evidence issuance or keepalive admission failed *before* the manager
    /// could accept it. This has no clock input because it cannot create a
    /// renewal deadline or an Armed worker: on an Idle manager it immediately
    /// creates an `ArmAdmissionFailed` Faulting run and its canonical all-deny
    /// plan. The later exact registry relinquish CAS remains the sole current
    /// authority revalidation point.
    ///
    /// A returned durable exact-`Both` commit is the authority proof that a
    /// different current manager run is stale on the single-resource index.
    /// This entrypoint therefore fences the old renewal/plan/relinquish gates,
    /// inherits its worker run ID for the new plan's `CancelRenewal` step, and
    /// replaces it with cleanup-only Faulting authority. Old late completion,
    /// renewal, and release results then fail their exact identity/gate match.
    pub(crate) fn begin_cleanup_only_after_unadmitted_durable_enable(
        &mut self,
        evidence: OutputLeaseKeepaliveDurableEnableCleanupOnlyEvidence,
    ) -> OutputLeaseKeepaliveCleanupOnlyOutcome {
        let durable_receipt_issuance = evidence.durable_receipt_issuance();
        let populated_slots = self.armed.is_some() as usize
            + self.faulting.is_some() as usize
            + self.faulted.is_some() as usize;
        let partial_state = populated_slots > 1;
        let matches_existing = self
            .armed
            .as_ref()
            .is_some_and(|armed| evidence.matches_armed(armed))
            || self
                .faulting
                .as_ref()
                .is_some_and(|fault| evidence.matches_fault(fault))
            || self
                .faulted
                .as_ref()
                .is_some_and(|fault| evidence.matches_fault(fault));

        if !partial_state {
            if let Some(armed) = self.armed.as_ref() {
                if evidence.matches_armed(armed) {
                    let run_id = armed.run_id;
                    return self
                        .begin_fail_stop_for_run_with_error(
                            run_id,
                            OutputLeaseKeepaliveBoundary::ArmAdmissionFailed,
                            Some(internal_port_error(
                                OutputLeaseKeepaliveError::DurableEnableAdmissionFailed,
                            )),
                        )
                        .map_or(
                            OutputLeaseKeepaliveCleanupOnlyOutcome::AwaitCurrentRelease,
                            OutputLeaseKeepaliveCleanupOnlyOutcome::FailStop,
                        );
                }
            }
            if let Some(fault) = self.faulting.as_ref() {
                if evidence.matches_fault(fault) {
                    return OutputLeaseKeepaliveCleanupOnlyOutcome::ResumeFailStop(
                        Self::fail_stop_plan_from_fault(fault),
                    );
                }
            }
            if let Some(fault) = self.faulted.as_ref() {
                if evidence.matches_fault(fault) {
                    return self.current_faulted_cleanup_only_outcome();
                }
            }
        }

        if durable_receipt_issuance < self.highest_durable_enable_issuance
            || (durable_receipt_issuance == self.highest_durable_enable_issuance
                && !matches_existing)
        {
            return OutputLeaseKeepaliveCleanupOnlyOutcome::StaleNoop;
        }
        let (identity, lease, registry_claim) = evidence.into_components_for_cleanup();
        self.highest_durable_enable_issuance = self
            .highest_durable_enable_issuance
            .max(durable_receipt_issuance);
        let inherited_run_id = self.take_current_run_id_for_cleanup_supersession();
        self.deferred = None;
        let run_id = inherited_run_id
            .or_else(|| self.next_run_id().ok())
            .unwrap_or(OutputLeaseKeepaliveRunId(u64::MAX));
        // This boundary is intentionally visible as one stable causal class:
        // the durable Enable committed but never reached normal admission.
        // Even a poisoned/overflowed local run counter cannot erase that
        // diagnosis or turn this non-arm path into an implicit retry.
        let initial_error =
            internal_port_error(OutputLeaseKeepaliveError::DurableEnableAdmissionFailed)
                .stored_detail();
        let fault = OutputLeaseKeepaliveFaultState {
            run_id,
            fail_stop_attempt: 1,
            identity,
            lease,
            registry_claim,
            boundary: OutputLeaseKeepaliveBoundary::ArmAdmissionFailed,
            fail_stop_report: None,
            trigger_error: Some(initial_error.clone()),
            last_error: Some(initial_error),
            fail_stop_gate: OutputLeaseKeepaliveFailStopExecutionGate::issued(),
            relinquish_permit_issued: false,
            relinquish_permit_gate: None,
            supersession_attempt: 0,
            supersession_in_flight: false,
            supersession_gate: None,
        };
        self.faulting = Some(fault.clone());
        OutputLeaseKeepaliveCleanupOnlyOutcome::FailStop(Self::fail_stop_plan_from_fault(&fault))
    }

    /// Retire the manager-side representation of the old current run while
    /// preserving its run ID so the replacement cleanup plan cancels the only
    /// possible old worker before touching physical outputs. Every exported
    /// old capability is explicitly fenced; its late result cannot mutate the
    /// new cleanup fault.
    fn take_current_run_id_for_cleanup_supersession(
        &mut self,
    ) -> Option<OutputLeaseKeepaliveRunId> {
        // A poisoned mutex may leave more than one state slot populated. Do
        // not return after the first slot: every old capability must become
        // stale before the cleanup-only fault is installed. Keep the Armed
        // run ID when present because its worker is the one which may still
        // need the replacement plan's `CancelRenewal` instruction.
        let armed = self.armed.take();
        let faulting = self.faulting.take();
        let faulted = self.faulted.take();
        self.deferred = None;

        let inherited_run_id = armed.as_ref().map(|state| state.run_id).or_else(|| {
            faulting
                .as_ref()
                .map(|state| state.run_id)
                .or_else(|| faulted.as_ref().map(|state| state.run_id))
        });

        if let Some(mut armed) = armed {
            if let Some(gate) = armed.renewal_gate.take() {
                gate.invalidate();
            }
        }
        if let Some(mut faulting) = faulting {
            faulting.fail_stop_gate.invalidate();
            if let Some(gate) = faulting.relinquish_permit_gate.take() {
                gate.invalidate();
            }
            if let Some(gate) = faulting.supersession_gate.take() {
                gate.invalidate();
            }
            faulting.relinquish_permit_issued = false;
            faulting.supersession_in_flight = false;
        }
        if let Some(mut faulted) = faulted {
            faulted.fail_stop_gate.invalidate();
            if let Some(gate) = faulted.relinquish_permit_gate.take() {
                gate.invalidate();
            }
            if let Some(gate) = faulted.supersession_gate.take() {
                gate.invalidate();
            }
            faulted.relinquish_permit_issued = false;
            faulted.supersession_in_flight = false;
        }

        inherited_run_id
    }

    /// Admit already-verified durable components only while no other manager
    /// run exists. This shared internal boundary is used by normal evidence
    /// admission and by a successful opaque supersession proof, so neither
    /// path can leave a live B authority without an Armed run or a B-bound
    /// canonical fail-stop plan.
    fn arm_verified_components_or_return_bound_fail_stop(
        &mut self,
        now_ms: u64,
        identity: OutputLeaseKeepaliveIdentity,
        lease: OutputLeaseKeepaliveExactBothLease,
        registry_claim: OutputLeaseKeepaliveRegistryClaim,
    ) -> OutputLeaseKeepaliveArmOutcome {
        if self.armed.is_some() || self.faulting.is_some() || self.faulted.is_some() {
            return OutputLeaseKeepaliveArmOutcome::StaleNoop;
        }
        let durable_receipt_issuance = registry_claim.durable_receipt_issuance;
        let (run_id, run_id_error) = match self.next_run_id() {
            Ok(run_id) => (run_id, None),
            Err(error) => (OutputLeaseKeepaliveRunId(u64::MAX), Some(error)),
        };
        let next_renew_at_ms = now_ms.checked_add(OUTPUT_LEASE_KEEPALIVE_RENEW_INTERVAL_MS);
        self.highest_durable_enable_issuance = durable_receipt_issuance;

        if run_id_error.is_none() {
            if let Some(next_renew_at_ms) = next_renew_at_ms {
                self.armed = Some(OutputLeaseKeepaliveArmedState {
                    run_id,
                    identity,
                    lease,
                    registry_claim,
                    durable_receipt_issuance,
                    next_renew_at_ms,
                    last_observed_now_ms: now_ms,
                    renewal_in_flight: false,
                    renewal_attempt: 0,
                    renewal_gate: None,
                });
                return OutputLeaseKeepaliveArmOutcome::Armed(run_id);
            }
        }

        let error = if let Some(error) = run_id_error {
            error
        } else {
            OutputLeaseKeepaliveError::ClockOverflow
        };
        let initial_error = internal_port_error(error).stored_detail();
        let fault = OutputLeaseKeepaliveFaultState {
            run_id,
            fail_stop_attempt: 1,
            identity,
            lease,
            registry_claim,
            boundary: OutputLeaseKeepaliveBoundary::ArmAdmissionFailed,
            fail_stop_report: None,
            trigger_error: Some(initial_error.clone()),
            last_error: Some(initial_error),
            fail_stop_gate: OutputLeaseKeepaliveFailStopExecutionGate::issued(),
            relinquish_permit_issued: false,
            relinquish_permit_gate: None,
            supersession_attempt: 0,
            supersession_in_flight: false,
            supersession_gate: None,
        };
        self.faulting = Some(fault.clone());
        OutputLeaseKeepaliveArmOutcome::FailStop(Self::fail_stop_plan_from_fault(&fault))
    }

    fn armed_matches_evidence(
        armed: &OutputLeaseKeepaliveArmedState,
        evidence: &OutputLeaseKeepaliveDurableEnableEvidence,
    ) -> bool {
        armed.identity == evidence.identity
            && armed.lease == evidence.lease
            && armed.registry_claim == evidence.registry_claim
    }

    fn fault_matches_evidence(
        fault: &OutputLeaseKeepaliveFaultState,
        evidence: &OutputLeaseKeepaliveDurableEnableEvidence,
    ) -> bool {
        Self::fault_matches_components(
            fault,
            &evidence.identity,
            &evidence.lease,
            &evidence.registry_claim,
        )
    }

    fn fault_matches_components(
        fault: &OutputLeaseKeepaliveFaultState,
        identity: &OutputLeaseKeepaliveIdentity,
        lease: &OutputLeaseKeepaliveExactBothLease,
        registry_claim: &OutputLeaseKeepaliveRegistryClaim,
    ) -> bool {
        fault.identity == *identity
            && fault.lease == *lease
            && fault.registry_claim == *registry_claim
    }

    fn current_faulted_arm_outcome(&mut self) -> OutputLeaseKeepaliveArmOutcome {
        if self.deferred.is_some() {
            return OutputLeaseKeepaliveArmOutcome::AwaitCurrentRelease;
        }
        let Some(fault) = self.faulted.as_ref() else {
            return OutputLeaseKeepaliveArmOutcome::StaleNoop;
        };
        let run_id = fault.run_id;
        let successful = fault
            .fail_stop_report
            .as_ref()
            .is_some_and(OutputLeaseKeepaliveFailStopReport::is_successful_canonical);
        if !successful {
            return self.retry_faulted_fail_stop_for_run(run_id).map_or(
                OutputLeaseKeepaliveArmOutcome::AwaitCurrentRelease,
                |plan| OutputLeaseKeepaliveArmOutcome::ResumeFailStop(plan),
            );
        }
        let Some(fault) = self.faulted.as_mut() else {
            return OutputLeaseKeepaliveArmOutcome::StaleNoop;
        };
        Self::take_relinquish_permit_from_fault(fault).map_or(
            OutputLeaseKeepaliveArmOutcome::AwaitCurrentRelease,
            OutputLeaseKeepaliveArmOutcome::RequireCurrentRelinquish,
        )
    }

    fn current_faulted_cleanup_only_outcome(&mut self) -> OutputLeaseKeepaliveCleanupOnlyOutcome {
        if self.deferred.is_some() {
            return OutputLeaseKeepaliveCleanupOnlyOutcome::AwaitCurrentRelease;
        }
        let Some(fault) = self.faulted.as_ref() else {
            return OutputLeaseKeepaliveCleanupOnlyOutcome::StaleNoop;
        };
        let run_id = fault.run_id;
        let successful = fault
            .fail_stop_report
            .as_ref()
            .is_some_and(OutputLeaseKeepaliveFailStopReport::is_successful_canonical);
        if !successful {
            return self.retry_faulted_fail_stop_for_run(run_id).map_or(
                OutputLeaseKeepaliveCleanupOnlyOutcome::AwaitCurrentRelease,
                OutputLeaseKeepaliveCleanupOnlyOutcome::ResumeFailStop,
            );
        }
        let Some(fault) = self.faulted.as_mut() else {
            return OutputLeaseKeepaliveCleanupOnlyOutcome::StaleNoop;
        };
        Self::take_relinquish_permit_from_fault(fault).map_or(
            OutputLeaseKeepaliveCleanupOnlyOutcome::AwaitCurrentRelease,
            OutputLeaseKeepaliveCleanupOnlyOutcome::RequireCurrentRelinquish,
        )
    }

    fn defer_new_durable_enable_until_current_run_released(
        &mut self,
        evidence: OutputLeaseKeepaliveDurableEnableEvidence,
    ) -> OutputLeaseKeepaliveArmOutcome {
        let issuance = evidence.registry_claim.durable_receipt_issuance;
        let matches_current_deferred = self
            .deferred
            .as_ref()
            .is_some_and(|deferred| deferred.matches_evidence(&evidence));
        let replaces_current_deferred = self.deferred.as_ref().map_or(true, |deferred| {
            issuance > deferred.durable_receipt_issuance()
        });
        if self.deferred.is_none() && issuance <= self.highest_durable_enable_issuance {
            return OutputLeaseKeepaliveArmOutcome::StaleNoop;
        }
        if !matches_current_deferred && !replaces_current_deferred {
            return OutputLeaseKeepaliveArmOutcome::StaleNoop;
        }

        if replaces_current_deferred {
            self.deferred = Some(OutputLeaseKeepaliveDeferredDurableEnable::from_evidence(
                evidence,
            ));
            self.highest_durable_enable_issuance = issuance;
        }

        let action = if let Some(armed) = self.armed.as_ref() {
            let run_id = armed.run_id;
            let plan = self
                .begin_fail_stop_for_run(
                    run_id,
                    OutputLeaseKeepaliveBoundary::ConflictingDurableEnable,
                )
                .expect("armed run was checked before beginning conflict fail-stop");
            OutputLeaseKeepaliveManagedConflictAction::ResumeFailStop(plan)
        } else if let Some(fault) = self.faulting.as_ref() {
            OutputLeaseKeepaliveManagedConflictAction::ResumeFailStop(
                Self::fail_stop_plan_from_fault(fault),
            )
        } else if self.faulted.is_some() {
            let fault = self
                .faulted
                .as_ref()
                .expect("faulted run was checked before preparing its conflict action");
            let run_id = fault.run_id;
            let successful = fault
                .fail_stop_report
                .as_ref()
                .is_some_and(OutputLeaseKeepaliveFailStopReport::is_successful_canonical);
            if successful {
                if fault.relinquish_permit_issued || fault.supersession_in_flight {
                    OutputLeaseKeepaliveManagedConflictAction::AwaitCurrentRelease
                } else {
                    // A different newer B does not consume A's release permit:
                    // it must first prove that B is current and A is stale.
                    OutputLeaseKeepaliveManagedConflictAction::PrepareDeferredSupersession
                }
            } else if let Some(plan) = self.retry_faulted_fail_stop_for_run(run_id) {
                OutputLeaseKeepaliveManagedConflictAction::ResumeFailStop(plan)
            } else {
                OutputLeaseKeepaliveManagedConflictAction::AwaitCurrentRelease
            }
        } else {
            unreachable!("caller established that a current run exists")
        };
        OutputLeaseKeepaliveArmOutcome::ManagedConflict(
            OutputLeaseKeepaliveManagedAdmissionConflict { action },
        )
    }

    /// Issue exactly one opaque proof capability for a deferred newer durable
    /// Enable B. This is admissible only after A's own full zero-failure
    /// canonical physical fail-stop completed. The capability never grants a
    /// raw registry release: the registry must prove that B is current and A
    /// is no longer current exact authority before this manager may retire A.
    ///
    /// The caller keeps the manager lock only for this state transition and
    /// performs the registry proof after dropping it.
    pub(crate) fn can_retry_deferred_supersession(&self) -> bool {
        self.deferred.is_some()
            && self.faulted.as_ref().is_some_and(|fault| {
                !fault.relinquish_permit_issued
                    && !fault.supersession_in_flight
                    && fault
                        .fail_stop_report
                        .as_ref()
                        .is_some_and(OutputLeaseKeepaliveFailStopReport::is_successful_canonical)
            })
    }

    pub(crate) fn prepare_deferred_supersession(
        &mut self,
    ) -> Result<OutputLeaseKeepaliveSupersessionCapability, OutputLeaseKeepaliveError> {
        let deferred = self
            .deferred
            .as_ref()
            .ok_or(OutputLeaseKeepaliveError::UnexpectedState)?;
        let fault = self
            .faulted
            .as_mut()
            .ok_or(OutputLeaseKeepaliveError::UnexpectedState)?;
        if Self::fault_matches_components(
            fault,
            &deferred.identity,
            &deferred.lease,
            &deferred.registry_claim,
        ) {
            // The current A receipt follows the ordinary exact relinquish
            // path. Treating it as B would permit a same-run self-retirement.
            return Err(OutputLeaseKeepaliveError::UnexpectedState);
        }
        if fault.relinquish_permit_issued
            || fault.supersession_in_flight
            || !fault
                .fail_stop_report
                .as_ref()
                .is_some_and(OutputLeaseKeepaliveFailStopReport::is_successful_canonical)
        {
            return Err(OutputLeaseKeepaliveError::UnexpectedState);
        }
        let supersession_attempt = fault
            .supersession_attempt
            .checked_add(1)
            .ok_or(OutputLeaseKeepaliveError::SupersessionAttemptExhausted)?;
        let gate = OutputLeaseKeepaliveSupersessionGate::issued();
        fault.supersession_attempt = supersession_attempt;
        fault.supersession_in_flight = true;
        fault.supersession_gate = Some(Arc::clone(&gate));
        Ok(OutputLeaseKeepaliveSupersessionCapability {
            old_run_id: fault.run_id,
            old_fail_stop_attempt: fault.fail_stop_attempt,
            supersession_attempt,
            old_identity: fault.identity.clone(),
            old_lease: fault.lease.clone(),
            old_registry_claim: fault.registry_claim.clone(),
            new_identity: deferred.identity.clone(),
            new_lease: deferred.lease.clone(),
            new_registry_claim: deferred.registry_claim.clone(),
            gate,
        })
    }

    /// Consume an opaque deferred-supersession capability when the adapter
    /// failed *before* it invoked the registry proof (for example because the
    /// registry mutex was poisoned). The current A run remains Faulted and the
    /// latest retained B/C authority remains opaque. A later serialized retry
    /// can mint a fresh capability; a stale B capability cannot clear or
    /// replace a newer C retained while it was outside the manager lock.
    ///
    /// This must not be called after a registry proof may have run. Such a
    /// proof always returns its own opaque receipt and completes through
    /// [`Self::complete_deferred_supersession_receipt`].
    pub(crate) fn abort_deferred_supersession_before_registry(
        &mut self,
        capability: OutputLeaseKeepaliveSupersessionCapability,
        error: OutputLeaseKeepalivePortError,
    ) -> OutputLeaseKeepaliveSupersessionCompletion {
        let fault_matches = self
            .faulted
            .as_ref()
            .is_some_and(|fault| capability.matches_fault(fault));
        if !fault_matches || !capability.gate.abort_before_registry() {
            return OutputLeaseKeepaliveSupersessionCompletion::StaleNoop;
        }
        let deferred_matches = self
            .deferred
            .as_ref()
            .is_some_and(|deferred| capability.matches_deferred(deferred));
        let fault = self
            .faulted
            .as_mut()
            .expect("matching faulted run remains present while aborting supersession proof");
        fault.supersession_in_flight = false;
        fault.supersession_gate = None;
        if !deferred_matches {
            // A newer C replaced the captured B while the adapter held B's
            // capability. C remains retained and must obtain a fresh proof;
            // never let the old adapter report a current error against C.
            return OutputLeaseKeepaliveSupersessionCompletion::StaleNoop;
        }
        fault.last_error = Some(error.stored_detail());
        OutputLeaseKeepaliveSupersessionCompletion::RetainedFaulted
    }

    /// Complete the opaque registry proof issued by
    /// [`Self::prepare_deferred_supersession`]. A success proves that A is no
    /// longer current authority and B is still the precise current durable
    /// authority; it does not mutate the registry. Only then can A retire and
    /// B pass through the regular arm-or-bound-fail-stop admission boundary.
    pub(crate) fn complete_deferred_supersession_receipt(
        &mut self,
        now_ms: u64,
        receipt: OutputLeaseKeepaliveSupersessionCasReceipt,
    ) -> OutputLeaseKeepaliveSupersessionCompletion {
        match receipt {
            OutputLeaseKeepaliveSupersessionCasReceipt::Proven(receipt) => {
                let fault_matches = self
                    .faulted
                    .as_ref()
                    .is_some_and(|fault| receipt.matches_fault(fault));
                if !fault_matches {
                    return OutputLeaseKeepaliveSupersessionCompletion::StaleNoop;
                }
                let deferred_matches = self
                    .deferred
                    .as_ref()
                    .is_some_and(|deferred| receipt.matches_deferred(deferred));
                if !deferred_matches {
                    // A newer C was retained while this old B proof was out
                    // of lock. B may have been current when proven, but it
                    // is no longer the manager's latest deferred authority.
                    // Discard only the stale completion and let C request its
                    // own fresh proof; never arm B after C.
                    if let Some(fault) = self.faulted.as_mut() {
                        fault.supersession_in_flight = false;
                        fault.supersession_gate = None;
                    }
                    return OutputLeaseKeepaliveSupersessionCompletion::StaleNoop;
                }
                let (identity, lease, registry_claim) = receipt.into_new_components();
                // A is retired only after the receipt matched the still
                // faulted A run. The helper cannot overwrite a run because A
                // is removed first and checks the empty-state invariant.
                self.faulted = None;
                self.deferred = None;
                match self.arm_verified_components_or_return_bound_fail_stop(
                    now_ms,
                    identity,
                    lease,
                    registry_claim,
                ) {
                    OutputLeaseKeepaliveArmOutcome::Armed(run_id) => {
                        OutputLeaseKeepaliveSupersessionCompletion::Armed(run_id)
                    }
                    OutputLeaseKeepaliveArmOutcome::FailStop(plan) => {
                        OutputLeaseKeepaliveSupersessionCompletion::FailStop(plan)
                    }
                    // This helper is only called after A was removed and it
                    // accepts components already verified by the registry.
                    // Keep fail-closed if a future refactor violates that
                    // invariant rather than resurrecting A or accepting B.
                    _ => OutputLeaseKeepaliveSupersessionCompletion::StaleNoop,
                }
            }
            OutputLeaseKeepaliveSupersessionCasReceipt::Rejected(receipt) => {
                let Some(fault) = self.faulted.as_mut() else {
                    return OutputLeaseKeepaliveSupersessionCompletion::StaleNoop;
                };
                if !receipt.matches_fault(fault) {
                    return OutputLeaseKeepaliveSupersessionCompletion::StaleNoop;
                }
                let deferred_matches = self
                    .deferred
                    .as_ref()
                    .is_some_and(|deferred| receipt.matches_deferred(deferred));
                fault.supersession_in_flight = false;
                fault.supersession_gate = None;
                if !deferred_matches {
                    return OutputLeaseKeepaliveSupersessionCompletion::StaleNoop;
                }
                fault.last_error = Some(receipt.error.stored_detail());
                OutputLeaseKeepaliveSupersessionCompletion::RetainedFaulted
            }
        }
    }

    /// Phase 3 after a clock read. Rollback is visible and must be followed
    /// by `begin_fail_stop_for_run(wait.run_id, ClockRollback)` under lock.
    pub(crate) fn observe_clock_for_run(
        &mut self,
        run_id: OutputLeaseKeepaliveRunId,
        now_ms: u64,
    ) -> Result<OutputLeaseKeepaliveTimerAction, OutputLeaseKeepaliveError> {
        let armed = self
            .armed
            .as_mut()
            .ok_or(OutputLeaseKeepaliveError::NotArmed)?;
        if armed.run_id != run_id {
            return Err(OutputLeaseKeepaliveError::StaleRun);
        }
        if now_ms < armed.last_observed_now_ms {
            return Err(OutputLeaseKeepaliveError::ClockRollback);
        }
        armed.last_observed_now_ms = now_ms;
        if now_ms >= armed.next_renew_at_ms {
            Ok(OutputLeaseKeepaliveTimerAction::RenewDue(run_id))
        } else {
            Ok(OutputLeaseKeepaliveTimerAction::Sleep(
                OutputLeaseKeepaliveWait {
                    run_id,
                    deadline_ms: armed.next_renew_at_ms,
                },
            ))
        }
    }

    /// Phase 1 for a renewal I/O call. The captured run from the timer phase
    /// must still be current, so a stale worker cannot renew a newer lease.
    pub(crate) fn prepare_due_renewal_for_run(
        &mut self,
        run_id: OutputLeaseKeepaliveRunId,
        now_ms: u64,
    ) -> Result<OutputLeaseKeepaliveRenewalCapability, OutputLeaseKeepaliveError> {
        let armed = self
            .armed
            .as_mut()
            .ok_or(OutputLeaseKeepaliveError::NotArmed)?;
        if armed.run_id != run_id {
            return Err(OutputLeaseKeepaliveError::StaleRun);
        }
        if armed.renewal_in_flight {
            return Err(OutputLeaseKeepaliveError::RenewalAlreadyInFlight);
        }
        if now_ms < armed.last_observed_now_ms {
            return Err(OutputLeaseKeepaliveError::ClockRollback);
        }
        if now_ms < armed.next_renew_at_ms {
            return Err(OutputLeaseKeepaliveError::UnexpectedState);
        }
        armed.last_observed_now_ms = now_ms;
        armed.renewal_in_flight = true;
        armed.renewal_attempt = armed
            .renewal_attempt
            .checked_add(1)
            .ok_or(OutputLeaseKeepaliveError::RenewalAttemptExhausted)?;
        let gate = OutputLeaseKeepaliveRenewalGate::issued();
        armed.renewal_gate = Some(Arc::clone(&gate));
        Ok(OutputLeaseKeepaliveRenewalCapability {
            run_id,
            renewal_attempt: armed.renewal_attempt,
            identity: armed.identity.clone(),
            lease: armed.lease.clone(),
            registry_claim: armed.registry_claim.clone(),
            gate,
        })
    }

    /// Phase 3 after the parent registry consumed one capability. No raw
    /// component result can complete an in-flight renewal: only its opaque
    /// registry receipt can renew or fault the captured run.
    pub(crate) fn complete_renewal_receipt(
        &mut self,
        now_ms: u64,
        receipt: OutputLeaseKeepaliveRenewalCasReceipt,
    ) -> Result<OutputLeaseKeepaliveRenewalCompletion, OutputLeaseKeepaliveError> {
        match receipt {
            OutputLeaseKeepaliveRenewalCasReceipt::Renewed(receipt) => {
                match self.commit_renewal_receipt(now_ms, &receipt) {
                    Ok(()) => Ok(OutputLeaseKeepaliveRenewalCompletion::Renewed),
                    Err(error) => {
                        let boundary = if error == OutputLeaseKeepaliveError::ClockRollback {
                            OutputLeaseKeepaliveBoundary::ClockRollback
                        } else {
                            OutputLeaseKeepaliveBoundary::InvalidRenewalReceipt
                        };
                        Ok(self.fail_for_current_renewal(
                            receipt.run_id,
                            boundary,
                            Some(internal_port_error(error)),
                        ))
                    }
                }
            }
            OutputLeaseKeepaliveRenewalCasReceipt::Rejected(receipt) => {
                if !self.renewal_failure_receipt_matches_current(&receipt) {
                    return Ok(OutputLeaseKeepaliveRenewalCompletion::StaleNoop);
                }
                Ok(self.fail_for_current_renewal(
                    receipt.run_id,
                    OutputLeaseKeepaliveBoundary::RenewalFailed,
                    Some(receipt.error),
                ))
            }
        }
    }

    /// Consume a prepared renewal when an adapter failed *before* calling the
    /// registry (for example, a poisoned registry lock or a local adapter
    /// precondition failure). The caller must not use this after a CAS may
    /// have run: a registry transition always completes through its opaque
    /// receipt instead. A matching issued capability becomes this exact run's
    /// canonical fail-stop obligation; stale/replayed capabilities do nothing.
    pub(crate) fn abort_due_renewal_before_registry(
        &mut self,
        capability: OutputLeaseKeepaliveRenewalCapability,
        error: OutputLeaseKeepalivePortError,
    ) -> OutputLeaseKeepaliveRenewalCompletion {
        let matches = self.armed.as_ref().is_some_and(|armed| {
            armed.run_id == capability.run_id
                && armed.renewal_in_flight
                && armed.renewal_attempt == capability.renewal_attempt
                && armed.identity == capability.identity
                && armed.lease == capability.lease
                && armed.registry_claim == capability.registry_claim
                && armed
                    .renewal_gate
                    .as_ref()
                    .is_some_and(|gate| Arc::ptr_eq(gate, &capability.gate))
                && capability.gate.is_issued()
        });
        if !matches {
            return OutputLeaseKeepaliveRenewalCompletion::StaleNoop;
        }
        capability.gate.invalidate();
        self.fail_for_current_renewal(
            capability.run_id,
            OutputLeaseKeepaliveBoundary::RenewalFailed,
            Some(error),
        )
    }

    fn commit_renewal_receipt(
        &mut self,
        now_ms: u64,
        receipt: &OutputLeaseKeepaliveRenewalSuccessReceipt,
    ) -> Result<(), OutputLeaseKeepaliveError> {
        let armed = self
            .armed
            .as_mut()
            .ok_or(OutputLeaseKeepaliveError::NotArmed)?;
        if armed.run_id != receipt.run_id
            || !armed.renewal_in_flight
            || armed.renewal_attempt != receipt.renewal_attempt
            || armed.identity != receipt.identity
            || armed.lease != receipt.previous_lease
            || armed.registry_claim != receipt.registry_claim
        {
            return Err(OutputLeaseKeepaliveError::StaleRun);
        }
        if receipt.renewed_lease.lease_id != armed.lease.lease_id {
            return Err(OutputLeaseKeepaliveError::RenewalLeaseIdentityChanged);
        }
        let expected_generation = armed
            .lease
            .generation
            .checked_add(1)
            .ok_or(OutputLeaseKeepaliveError::RenewalGenerationOverflow)?;
        if receipt.renewed_lease.generation < expected_generation {
            return Err(OutputLeaseKeepaliveError::RenewalGenerationDidNotAdvance);
        }
        if receipt.renewed_lease.generation > expected_generation {
            return Err(OutputLeaseKeepaliveError::RenewalGenerationSkipped);
        }
        if now_ms < armed.last_observed_now_ms {
            return Err(OutputLeaseKeepaliveError::ClockRollback);
        }
        let next_renew_at_ms = now_ms
            .checked_add(OUTPUT_LEASE_KEEPALIVE_RENEW_INTERVAL_MS)
            .ok_or(OutputLeaseKeepaliveError::ClockOverflow)?;
        armed.lease = receipt.renewed_lease.clone();
        armed.registry_claim.generation = armed.lease.generation;
        armed.next_renew_at_ms = next_renew_at_ms;
        armed.last_observed_now_ms = now_ms;
        armed.renewal_in_flight = false;
        armed.renewal_gate = None;
        Ok(())
    }

    fn renewal_failure_receipt_matches_current(
        &self,
        receipt: &OutputLeaseKeepaliveRenewalFailureReceipt,
    ) -> bool {
        self.armed.as_ref().is_some_and(|armed| {
            armed.run_id == receipt.run_id
                && armed.renewal_in_flight
                && armed.renewal_attempt == receipt.renewal_attempt
                && armed.identity == receipt.identity
                && armed.lease == receipt.lease
                && armed.registry_claim == receipt.registry_claim
        })
    }

    fn fail_for_current_renewal(
        &mut self,
        run_id: OutputLeaseKeepaliveRunId,
        boundary: OutputLeaseKeepaliveBoundary,
        initial_error: Option<OutputLeaseKeepalivePortError>,
    ) -> OutputLeaseKeepaliveRenewalCompletion {
        match self.begin_fail_stop_for_run_with_error(run_id, boundary, initial_error) {
            Some(plan) => OutputLeaseKeepaliveRenewalCompletion::FailStop(plan),
            None => OutputLeaseKeepaliveRenewalCompletion::StaleNoop,
        }
    }

    /// Phase 1 for external failure/boundary. Execute the returned plan outside
    /// the manager lock, then call `complete_fail_stop` under it.
    pub(crate) fn begin_fail_stop_for_run(
        &mut self,
        run_id: OutputLeaseKeepaliveRunId,
        boundary: OutputLeaseKeepaliveBoundary,
    ) -> Option<OutputLeaseKeepaliveFailStopPlan> {
        self.begin_fail_stop_for_run_with_error(run_id, boundary, None)
    }

    pub(crate) fn begin_fail_stop_for_run_with_error(
        &mut self,
        run_id: OutputLeaseKeepaliveRunId,
        boundary: OutputLeaseKeepaliveBoundary,
        initial_error: Option<OutputLeaseKeepalivePortError>,
    ) -> Option<OutputLeaseKeepaliveFailStopPlan> {
        let (identity, lease, registry_claim) = {
            let armed = self.armed.as_mut()?;
            if armed.run_id != run_id {
                return None;
            }
            if let Some(gate) = armed.renewal_gate.take() {
                gate.invalidate();
            }
            (
                armed.identity.clone(),
                armed.lease.clone(),
                armed.registry_claim.clone(),
            )
        };
        let initial_error = initial_error.map(|error| error.stored_detail());
        let fail_stop_gate = OutputLeaseKeepaliveFailStopExecutionGate::issued();
        let fault = OutputLeaseKeepaliveFaultState {
            run_id,
            fail_stop_attempt: 1,
            identity,
            lease,
            registry_claim,
            boundary,
            fail_stop_report: None,
            trigger_error: initial_error.clone(),
            last_error: initial_error.clone(),
            fail_stop_gate,
            relinquish_permit_issued: false,
            relinquish_permit_gate: None,
            supersession_attempt: 0,
            supersession_in_flight: false,
            supersession_gate: None,
        };
        self.armed = None;
        self.faulting = Some(fault.clone());
        Some(Self::fail_stop_plan_from_fault(&fault))
    }

    /// Phase 3 after a worker spawn attempted outside the manager lock. The
    /// returned plan is absent if its captured run was fenced or replaced.
    pub(crate) fn record_worker_spawn_failure(
        &mut self,
        run_id: OutputLeaseKeepaliveRunId,
        error: OutputLeaseKeepalivePortError,
    ) -> Option<OutputLeaseKeepaliveFailStopPlan> {
        self.begin_fail_stop_for_run_with_error(
            run_id,
            OutputLeaseKeepaliveBoundary::WorkerSpawnFailed,
            Some(error),
        )
    }

    /// Convert an unexpected worker panic or early exit into the same exact
    /// run-bound canonical fail-stop obligation as a spawn failure. A normal
    /// cancellation/boundary has already moved the manager out of `Armed`, so
    /// this returns `None` for it instead of creating a second plan.
    pub(crate) fn record_worker_termination(
        &mut self,
        run_id: OutputLeaseKeepaliveRunId,
        error: OutputLeaseKeepalivePortError,
    ) -> Option<OutputLeaseKeepaliveFailStopPlan> {
        self.begin_fail_stop_for_run_with_error(
            run_id,
            OutputLeaseKeepaliveBoundary::WorkerTerminated,
            Some(error),
        )
    }

    /// Safe resume after an interrupted fail-stop: returns only the same
    /// faulting run's immutable plan, never a newly armed run.
    pub(crate) fn resume_fail_stop_for_run(
        &self,
        run_id: OutputLeaseKeepaliveRunId,
    ) -> Option<OutputLeaseKeepaliveFailStopPlan> {
        let fault = self.faulting.as_ref()?;
        if fault.run_id != run_id {
            return None;
        }
        Some(Self::fail_stop_plan_from_fault(fault))
    }

    fn fail_stop_plan_from_fault(
        fault: &OutputLeaseKeepaliveFaultState,
    ) -> OutputLeaseKeepaliveFailStopPlan {
        OutputLeaseKeepaliveFailStopPlan {
            run_id: fault.run_id,
            fail_stop_attempt: fault.fail_stop_attempt,
            identity: fault.identity.clone(),
            lease: fault.lease.clone(),
            boundary: fault.boundary,
            initial_error: fault.trigger_error.clone(),
            gate: Arc::clone(&fault.fail_stop_gate),
        }
    }

    /// A failed best-effort pass remains Faulted and cannot release the lease.
    /// An explicit matching Relinquish may move only that same immutable run
    /// back to Faulting for one new complete canonical pass.
    fn retry_faulted_fail_stop_for_run(
        &mut self,
        run_id: OutputLeaseKeepaliveRunId,
    ) -> Option<OutputLeaseKeepaliveFailStopPlan> {
        let fault = self.faulted.as_ref()?;
        if fault.run_id != run_id
            || !fault
                .fail_stop_report
                .as_ref()
                .is_some_and(|report| report.is_complete_canonical() && !report.failures.is_empty())
        {
            return None;
        }
        let next_attempt = fault.fail_stop_attempt.checked_add(1)?;
        let Some(mut retry) = self.faulted.take() else {
            return None;
        };
        retry.fail_stop_attempt = next_attempt;
        retry.fail_stop_report = None;
        retry.fail_stop_gate = OutputLeaseKeepaliveFailStopExecutionGate::issued();
        retry.relinquish_permit_issued = false;
        retry.relinquish_permit_gate = None;
        if let Some(gate) = retry.supersession_gate.take() {
            gate.invalidate();
        }
        retry.supersession_in_flight = false;
        let plan = Self::fail_stop_plan_from_fault(&retry);
        self.faulting = Some(retry);
        Some(plan)
    }

    #[cfg(test)]
    pub(crate) fn reconcile_identity(
        &mut self,
        observed: &OutputLeaseKeepaliveIdentity,
    ) -> Option<OutputLeaseKeepaliveFailStopPlan> {
        let armed = self.armed.as_ref()?;
        if armed.identity == *observed {
            return None;
        }
        let boundary = if armed.identity.project_identity != observed.project_identity {
            OutputLeaseKeepaliveBoundary::ProjectIdentityChanged
        } else if armed.identity.owner_identity != observed.owner_identity {
            OutputLeaseKeepaliveBoundary::OwnerIdentityChanged
        } else if armed.identity.window_label != observed.window_label {
            OutputLeaseKeepaliveBoundary::WindowIdentityChanged
        } else if armed.identity.session_identity != observed.session_identity {
            OutputLeaseKeepaliveBoundary::SessionIdentityChanged
        } else {
            OutputLeaseKeepaliveBoundary::RequestCorrelationChanged
        };
        self.begin_fail_stop_for_run(armed.run_id, boundary)
    }

    /// Phase 3 after the one-shot executor made one full best-effort pass.
    /// Duplicate/stale plans cannot submit an independently forged report or
    /// alter a rearmed run.
    pub(crate) fn complete_fail_stop(
        &mut self,
        execution: OutputLeaseKeepaliveFailStopExecution,
    ) -> Result<OutputLeaseKeepaliveFailStopCompletion, OutputLeaseKeepaliveError> {
        let Some(fault) = self.faulting.as_ref() else {
            return Ok(OutputLeaseKeepaliveFailStopCompletion::StaleNoop);
        };
        if !execution.plan.matches_fault(fault) {
            return Ok(OutputLeaseKeepaliveFailStopCompletion::StaleNoop);
        }
        if !execution.report.is_complete_canonical() {
            return Err(OutputLeaseKeepaliveError::IncompleteFailStopReport);
        }
        if !execution.plan.gate.mark_completed() {
            return Ok(OutputLeaseKeepaliveFailStopCompletion::StaleNoop);
        }
        let Some(mut faulted) = self.faulting.take() else {
            return Ok(OutputLeaseKeepaliveFailStopCompletion::StaleNoop);
        };
        let report_error = execution.report.last_error().map(ToOwned::to_owned);
        faulted.last_error = report_error.or(faulted.last_error);
        faulted.fail_stop_report = Some(execution.report);
        self.faulted = Some(faulted);
        Ok(OutputLeaseKeepaliveFailStopCompletion::Completed)
    }

    /// The exact managed Relinquish path. A matching active lease atomically creates its
    /// fail-stop plan; a faulting run requires that same plan to finish; only a
    /// matching Faulted run with a zero-failure canonical report emits an
    /// opaque relinquish permit. A failed report re-enters the same run's
    /// canonical fail-stop instead of releasing authority.
    pub(crate) fn manual_relinquish_decision(
        &mut self,
        lease_id: &str,
    ) -> OutputLeaseKeepaliveManualOperationDecision {
        if let Some(armed) = self.armed.as_ref() {
            if armed.lease.lease_id == lease_id {
                let run_id = armed.run_id;
                let plan = self
                    .begin_fail_stop_for_run(run_id, OutputLeaseKeepaliveBoundary::Relinquish)
                    .expect("armed run was checked before beginning its fail-stop");
                return OutputLeaseKeepaliveManualOperationDecision::BeginFailStop(plan);
            }
        }
        if let Some(fault) = self.faulting.as_ref() {
            if fault.lease.lease_id == lease_id {
                let plan = self
                    .resume_fail_stop_for_run(fault.run_id)
                    .expect("faulting run was checked before resuming its fail-stop");
                return OutputLeaseKeepaliveManualOperationDecision::RequireFailStop(plan);
            }
        }
        if let Some(fault) = self.faulted.as_ref() {
            if fault.lease.lease_id == lease_id {
                if fault.boundary != OutputLeaseKeepaliveBoundary::Relinquish {
                    // Automatic boundaries (standby, identity loss, clock,
                    // shutdown) must use the explicit boundary handoff below.
                    // A manual Relinquish label must not launder their
                    // authority transition into a user-requested release.
                    return OutputLeaseKeepaliveManualOperationDecision::RejectManagedLease;
                }
                let run_id = fault.run_id;
                let successful = fault
                    .fail_stop_report
                    .as_ref()
                    .is_some_and(OutputLeaseKeepaliveFailStopReport::is_successful_canonical);
                if successful {
                    if self.deferred.is_some() {
                        return OutputLeaseKeepaliveManualOperationDecision::RejectManagedLease;
                    }
                    let fault = self
                        .faulted
                        .as_mut()
                        .expect("faulted run was checked before issuing its permit");
                    return Self::take_relinquish_permit_from_fault(fault).map_or(
                        OutputLeaseKeepaliveManualOperationDecision::RejectManagedLease,
                        OutputLeaseKeepaliveManualOperationDecision::AllowRelinquish,
                    );
                }
                if let Some(plan) = self.retry_faulted_fail_stop_for_run(run_id) {
                    return OutputLeaseKeepaliveManualOperationDecision::RequireFailStop(plan);
                }
                return OutputLeaseKeepaliveManualOperationDecision::RejectManagedLease;
            }
        }
        OutputLeaseKeepaliveManualOperationDecision::AllowUnmanaged
    }

    /// Regression seam for the retired generic lifecycle API. It retains the
    /// proof that non-Relinquish operations cannot mutate a managed lease,
    /// while production exposes no such mutation surface.
    #[cfg(test)]
    pub(crate) fn manual_operation_decision(
        &mut self,
        lease_id: &str,
        operation: OutputLeaseKeepaliveManualOperation,
    ) -> OutputLeaseKeepaliveManualOperationDecision {
        if matches!(operation, OutputLeaseKeepaliveManualOperation::Relinquish) {
            return self.manual_relinquish_decision(lease_id);
        }
        if self
            .armed
            .as_ref()
            .is_some_and(|armed| armed.lease.lease_id == lease_id)
            || self
                .faulting
                .as_ref()
                .is_some_and(|fault| fault.lease.lease_id == lease_id)
            || self
                .faulted
                .as_ref()
                .is_some_and(|fault| fault.lease.lease_id == lease_id)
        {
            OutputLeaseKeepaliveManualOperationDecision::RejectManagedLease
        } else {
            OutputLeaseKeepaliveManualOperationDecision::AllowUnmanaged
        }
    }

    /// Explicit automatic-boundary handoff. A standby/identity/clock/etc.
    /// coordinator may request this only after the exact captured run has
    /// completed the entire canonical fail-stop with no failed steps. It
    /// cannot reuse the manual Relinquish decision or a different identity.
    /// The returned opaque permit is one-shot at the registry boundary; a
    /// stale duplicate CAS result can never clear this manager.
    pub(crate) fn permit_relinquish_after_successful_boundary(
        &mut self,
        run_id: OutputLeaseKeepaliveRunId,
        identity: &OutputLeaseKeepaliveIdentity,
    ) -> Option<OutputLeaseKeepaliveRelinquishPermit> {
        if self.deferred.is_some() {
            return None;
        }
        let fault = self.faulted.as_mut()?;
        if fault.run_id != run_id
            || fault.identity != *identity
            || fault.boundary == OutputLeaseKeepaliveBoundary::Relinquish
            || !fault
                .fail_stop_report
                .as_ref()
                .is_some_and(OutputLeaseKeepaliveFailStopReport::is_successful_canonical)
        {
            return None;
        }
        Self::take_relinquish_permit_from_fault(fault)
    }

    fn take_relinquish_permit_from_fault(
        fault: &mut OutputLeaseKeepaliveFaultState,
    ) -> Option<OutputLeaseKeepaliveRelinquishPermit> {
        if fault.relinquish_permit_issued || fault.supersession_in_flight {
            return None;
        }
        let gate = OutputLeaseKeepaliveRelinquishGate::issued();
        fault.relinquish_permit_issued = true;
        fault.relinquish_permit_gate = Some(Arc::clone(&gate));
        Some(OutputLeaseKeepaliveRelinquishPermit {
            run_id: fault.run_id,
            identity: fault.identity.clone(),
            lease: fault.lease.clone(),
            registry_claim: fault.registry_claim.clone(),
            gate,
        })
    }

    /// The integration adapter must validate this immediately before touching
    /// the exact external registry CAS. A permit is not a completion token:
    /// only the resulting exact-success receipt can clear Faulted.
    pub(crate) fn admits_relinquish_permit(
        &self,
        permit: &OutputLeaseKeepaliveRelinquishPermit,
    ) -> bool {
        let no_deferred_supersession = self.deferred.is_none();
        self.faulted.as_ref().is_some_and(|fault| {
            fault.run_id == permit.run_id
                && fault.identity == permit.identity
                && fault.lease == permit.lease
                && fault.registry_claim == permit.registry_claim
                && fault.relinquish_permit_issued
                && fault
                    .relinquish_permit_gate
                    .as_ref()
                    .is_some_and(|gate| Arc::ptr_eq(gate, &permit.gate))
                && !fault.supersession_in_flight
                && no_deferred_supersession
                && permit.gate.is_issued()
                && fault
                    .fail_stop_report
                    .as_ref()
                    .is_some_and(OutputLeaseKeepaliveFailStopReport::is_successful_canonical)
        })
    }

    /// Consume an exact relinquish permit after an adapter failed before it
    /// invoked the registry CAS. This is deliberately distinct from a
    /// registry rejection: it marks the one-shot gate aborted, preserves the
    /// already-complete physical fail-stop, retains Faulted, and permits one
    /// later serialized retry/proof. If a newer B/C was retained after the
    /// permit left the lock, it remains retained for supersession instead of
    /// being dropped with the adapter error.
    pub(crate) fn abort_relinquish_before_registry(
        &mut self,
        permit: OutputLeaseKeepaliveRelinquishPermit,
        error: OutputLeaseKeepalivePortError,
    ) -> OutputLeaseKeepaliveRelinquishCompletion {
        let matches = self.faulted.as_ref().is_some_and(|fault| {
            fault.run_id == permit.run_id
                && fault.identity == permit.identity
                && fault.lease == permit.lease
                && fault.registry_claim == permit.registry_claim
                && fault.relinquish_permit_issued
                && fault
                    .relinquish_permit_gate
                    .as_ref()
                    .is_some_and(|gate| Arc::ptr_eq(gate, &permit.gate))
                && !fault.supersession_in_flight
                && permit.gate.is_issued()
                && fault
                    .fail_stop_report
                    .as_ref()
                    .is_some_and(OutputLeaseKeepaliveFailStopReport::is_successful_canonical)
        });
        if !matches || !permit.gate.abort_before_registry() {
            return OutputLeaseKeepaliveRelinquishCompletion::StaleNoop;
        }
        let Some(fault) = self.faulted.as_mut() else {
            return OutputLeaseKeepaliveRelinquishCompletion::StaleNoop;
        };
        fault.relinquish_permit_issued = false;
        fault.relinquish_permit_gate = None;
        fault.last_error = Some(error.stored_detail());
        OutputLeaseKeepaliveRelinquishCompletion::RetainedForRetry
    }

    /// Phase 3 after the exact managed registry CAS consumes an admitted
    /// permit. There is deliberately no permit-only completion path: only a
    /// matching successful receipt clears Faulted. A matching rejected receipt
    /// retains Faulted, records bounded detail, and reopens one serialized
    /// retry permit rather than stranding the authority on a dropped token.
    pub(crate) fn complete_relinquish_cas_receipt(
        &mut self,
        receipt: OutputLeaseKeepaliveRelinquishCasReceipt,
    ) -> OutputLeaseKeepaliveRelinquishCompletion {
        match receipt {
            OutputLeaseKeepaliveRelinquishCasReceipt::Released(receipt) => {
                let Some(fault) = self.faulted.as_ref() else {
                    return OutputLeaseKeepaliveRelinquishCompletion::StaleNoop;
                };
                if !receipt.matches_fault(fault)
                    || fault.supersession_in_flight
                    || !fault
                        .fail_stop_report
                        .as_ref()
                        .is_some_and(OutputLeaseKeepaliveFailStopReport::is_successful_canonical)
                {
                    return OutputLeaseKeepaliveRelinquishCompletion::StaleNoop;
                }
                if self.deferred.is_some() {
                    // A B/C Enable arrived after the A permit left the lock.
                    // An exact A CAS can only have succeeded if it did not
                    // mutate that B/C authority. Keep A Faulted, consume the
                    // old permit state, and let the retained latest deferred
                    // authority proceed through its own opaque proof.
                    let Some(fault) = self.faulted.as_mut() else {
                        return OutputLeaseKeepaliveRelinquishCompletion::StaleNoop;
                    };
                    fault.relinquish_permit_issued = false;
                    fault.relinquish_permit_gate = None;
                    return OutputLeaseKeepaliveRelinquishCompletion::RetainedForRetry;
                }
                self.faulted = None;
                OutputLeaseKeepaliveRelinquishCompletion::Completed
            }
            OutputLeaseKeepaliveRelinquishCasReceipt::Rejected(receipt) => {
                let Some(fault) = self.faulted.as_mut() else {
                    return OutputLeaseKeepaliveRelinquishCompletion::StaleNoop;
                };
                if !receipt.matches_fault(fault)
                    || fault.supersession_in_flight
                    || !fault
                        .fail_stop_report
                        .as_ref()
                        .is_some_and(OutputLeaseKeepaliveFailStopReport::is_successful_canonical)
                {
                    return OutputLeaseKeepaliveRelinquishCompletion::StaleNoop;
                }
                fault.relinquish_permit_issued = false;
                fault.relinquish_permit_gate = None;
                fault.last_error = Some(receipt.error.stored_detail());
                OutputLeaseKeepaliveRelinquishCompletion::RetainedForRetry
            }
        }
    }

    fn next_run_id(&mut self) -> Result<OutputLeaseKeepaliveRunId, OutputLeaseKeepaliveError> {
        let next = self
            .next_run_id
            .checked_add(1)
            .ok_or(OutputLeaseKeepaliveError::RunIdExhausted)?;
        self.next_run_id = next;
        Ok(OutputLeaseKeepaliveRunId(next))
    }
}

/// Phase 2: execute outside the manager lock, attempting the full order even
/// when individual output retirement operations fail.
pub(crate) fn execute_fail_stop_plan<P: OutputLeaseKeepaliveFailStopPorts>(
    ports: &mut P,
    plan: OutputLeaseKeepaliveFailStopPlan,
) -> OutputLeaseKeepaliveFailStopExecutionOutcome {
    if !plan.gate.claim_execution() {
        return OutputLeaseKeepaliveFailStopExecutionOutcome::StaleNoop;
    }
    let mut report = OutputLeaseKeepaliveFailStopReport::new_canonical_attempt();
    for step in OUTPUT_LEASE_KEEPALIVE_FAIL_STOP_ORDER {
        report.attempted_steps.push(step);
        let result = match step {
            OutputLeaseKeepaliveFailStopStep::CancelRenewal => ports.cancel_renewal(&plan),
            OutputLeaseKeepaliveFailStopStep::RequestSafetyBlackout => {
                ports.request_safety_blackout(&plan)
            }
            OutputLeaseKeepaliveFailStopStep::AdvanceFailureFence => {
                ports.advance_failure_fence(&plan)
            }
            OutputLeaseKeepaliveFailStopStep::RetireDmxUsbArtNet => {
                ports.retire_dmx_usb_artnet(&plan)
            }
            OutputLeaseKeepaliveFailStopStep::RetireSpoutNative => ports.retire_spout_native(&plan),
            OutputLeaseKeepaliveFailStopStep::PersistStandbyFailed => {
                ports.persist_standby_failed(&plan)
            }
            OutputLeaseKeepaliveFailStopStep::RequireExplicitEnable => Ok(()),
        };
        if let Err(error) = result {
            report.failures.push(OutputLeaseKeepaliveStepFailure {
                step,
                error: error.stored_detail(),
            });
        }
    }
    if !plan.gate.mark_executed() {
        return OutputLeaseKeepaliveFailStopExecutionOutcome::StaleNoop;
    }
    OutputLeaseKeepaliveFailStopExecutionOutcome::Executed(OutputLeaseKeepaliveFailStopExecution {
        plan,
        report,
    })
}

#[cfg(test)]
#[path = "output_lease_keepalive_tests.rs"]
mod tests;
