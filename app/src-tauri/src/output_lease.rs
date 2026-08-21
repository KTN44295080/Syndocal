//! Pure AI3 output-lease authority transitions.
//!
//! Pure/process-local AI3 output-lease authority, registry, exact receipts,
//! bounded admission, rate limits, and audit truth. This layer deliberately
//! has no Engine, output-worker, Blackout, role, consent, AppState, or Tauri
//! dependency. The main/runtime integration submits only through the registry
//! request boundary and owns physical-output commit ordering separately.

use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    fmt,
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub(crate) const MAX_OUTPUT_LEASE_TTL_MS: u64 = 60_000;
const OUTPUT_LEASE_RECEIPT_TTL_MS: u64 = 60_000;
const OUTPUT_LEASE_UNCLAIMED_RETENTION_MS: u64 = 60_000;
const OUTPUT_LEASE_TOKEN_BUCKET_IDLE_PURGE_MS: u64 = 60_000;
const MAX_OUTPUT_LEASE_OWNER_PRINCIPAL_BYTES: usize = 128;
const MAX_OUTPUT_LEASE_OWNER_WINDOW_BYTES: usize = 128;
const MAX_OUTPUT_LEASE_PROJECT_BYTES: usize = 256;
const MAX_OUTPUT_LEASE_REQUEST_PRINCIPAL_BYTES: usize = 128;
const MAX_OUTPUT_LEASE_REQUEST_DOMAIN_BYTES: usize = 128;
const MAX_OUTPUT_LEASE_OPERATION_BYTES: usize = 128;
const MAX_OUTPUT_LEASE_SELECTED_IDS: usize = 64;

/// These bounds are intentionally small enough to make this process-local
/// layer incapable of becoming an unbounded memory sink. They are authority
/// bounds, not a reason to evict a live lease.
pub(crate) const MAX_OUTPUT_LEASES: usize = 64;
pub(crate) const MAX_OUTPUT_LEASE_REQUESTS: usize = 128;
pub(crate) const MAX_OUTPUT_LEASE_LANES: usize = 128;
pub(crate) const MAX_OUTPUT_LEASE_TOKEN_BUCKETS: usize = 128;
pub(crate) const MAX_OUTPUT_LEASE_REQUEST_ORIGINS: usize = 128;
pub(crate) const MAX_OUTPUT_LEASE_AUDIT_RECORDS: usize = 256;
const OUTPUT_LEASE_TOKEN_BURST_MILLI: u64 = 8_000;
const OUTPUT_LEASE_TOKEN_RATE_PER_SECOND_MILLI: u64 = 4;

fn bounded_nonempty(value: &str, max_bytes: usize) -> bool {
    !value.trim().is_empty() && value.len() <= max_bytes
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub(crate) enum OutputLeaseResource {
    Lighting,
    Video,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct OutputLeaseResources(Vec<OutputLeaseResource>);

impl OutputLeaseResources {
    pub(crate) fn new(resources: &[OutputLeaseResource]) -> Result<Self, OutputLeaseError> {
        if resources.is_empty() {
            return Err(OutputLeaseError::InvalidResources);
        }
        let mut canonical = resources.to_vec();
        canonical.sort_unstable();
        if canonical.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(OutputLeaseError::InvalidResources);
        }
        Ok(Self(canonical))
    }

    pub(crate) fn as_slice(&self) -> &[OutputLeaseResource] {
        &self.0
    }

    fn overlaps(&self, other: &Self) -> bool {
        self.0
            .iter()
            .any(|resource| other.0.binary_search(resource).is_ok())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct OutputLeaseOwner {
    principal: String,
    window_label: String,
    process_session_incarnation: u64,
    owner_incarnation: u64,
}

/// Opaque, backend-issued lease identity. The numeric representation never
/// crosses this module's API; equality is the only operation callers need.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct OutputLeaseId(u64);

impl fmt::Debug for OutputLeaseId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("OutputLeaseId")
            .field(&self.encode())
            .finish()
    }
}

impl OutputLeaseId {
    pub(crate) fn encode(self) -> String {
        format!("lease-{:016x}", self.0)
    }

    pub(crate) fn decode(value: &str) -> Result<Self, OutputLeaseError> {
        if value.len() != 22
            || !value.is_ascii()
            || !value.as_bytes().starts_with(b"lease-")
            || !value.as_bytes()[6..]
                .iter()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(byte))
        {
            return Err(OutputLeaseError::InvalidRequest);
        }
        let numeric =
            u64::from_str_radix(&value[6..], 16).map_err(|_| OutputLeaseError::InvalidRequest)?;
        if numeric == 0 {
            return Err(OutputLeaseError::InvalidRequest);
        }
        Ok(Self(numeric))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OutputLeaseGrant {
    pub(crate) lease_id: OutputLeaseId,
    pub(crate) snapshot: OutputLeaseSnapshot,
}

impl OutputLeaseOwner {
    pub(crate) fn new(
        principal: impl Into<String>,
        window_label: impl Into<String>,
        process_session_incarnation: u64,
        owner_incarnation: u64,
    ) -> Result<Self, OutputLeaseError> {
        let principal = principal.into();
        let window_label = window_label.into();
        if !bounded_nonempty(&principal, MAX_OUTPUT_LEASE_OWNER_PRINCIPAL_BYTES)
            || !bounded_nonempty(&window_label, MAX_OUTPUT_LEASE_OWNER_WINDOW_BYTES)
            || process_session_incarnation == 0
            || owner_incarnation == 0
        {
            return Err(OutputLeaseError::InvalidOwner);
        }
        Ok(Self {
            principal,
            window_label,
            process_session_incarnation,
            owner_incarnation,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum OutputLeasePhase {
    Unclaimed,
    HeldActive,
    HeldOrphaned,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct OutputLeaseSnapshot {
    pub(crate) phase: OutputLeasePhase,
    pub(crate) owner: Option<OutputLeaseOwner>,
    pub(crate) resources: Option<OutputLeaseResources>,
    pub(crate) generation: u64,
    pub(crate) expires_at_monotonic_ms: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum OutputLeaseError {
    InvalidOwner,
    InvalidResources,
    InvalidProject,
    InvalidRequest,
    InvalidTtl,
    InvalidTransition,
    StaleOwner,
    StaleGeneration,
    Expired,
    UnknownLease,
    ResourceConflict,
    LeaseCapacity,
    LeaseIdExhausted,
    RequestCapacity,
    LaneCapacity,
    Busy,
    Conflict,
    ReceiptNotRetained,
    RateLimited,
    AuditCapacity,
    RequestNotInFlight,
    ClockRollback,
    GenerationExhausted,
    ClockExhausted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct OutputLeaseState {
    process_session_incarnation: u64,
    snapshot: OutputLeaseSnapshot,
}

impl OutputLeaseState {
    /// A process restart never reconstructs authority from cached state.
    fn fresh_process(process_session_incarnation: u64) -> Result<Self, OutputLeaseError> {
        if process_session_incarnation == 0 {
            return Err(OutputLeaseError::InvalidOwner);
        }
        Ok(Self {
            process_session_incarnation,
            snapshot: OutputLeaseSnapshot {
                phase: OutputLeasePhase::Unclaimed,
                owner: None,
                resources: None,
                generation: 0,
                expires_at_monotonic_ms: None,
            },
        })
    }

    fn snapshot(&self) -> OutputLeaseSnapshot {
        self.snapshot.clone()
    }

    fn acquire(
        &mut self,
        owner: OutputLeaseOwner,
        resources: OutputLeaseResources,
        now_ms: u64,
        ttl_ms: u64,
    ) -> Result<OutputLeaseSnapshot, OutputLeaseError> {
        if self.snapshot.phase != OutputLeasePhase::Unclaimed {
            return Err(OutputLeaseError::InvalidTransition);
        }
        self.require_current_process(&owner)?;
        let generation = next_generation(self.snapshot.generation)?;
        let deadline = checked_deadline(now_ms, ttl_ms)?;
        self.snapshot = OutputLeaseSnapshot {
            phase: OutputLeasePhase::HeldActive,
            owner: Some(owner),
            resources: Some(resources),
            generation,
            expires_at_monotonic_ms: Some(deadline),
        };
        Ok(self.snapshot())
    }

    fn renew(
        &mut self,
        owner: &OutputLeaseOwner,
        expected_generation: u64,
        now_ms: u64,
        ttl_ms: u64,
    ) -> Result<OutputLeaseSnapshot, OutputLeaseError> {
        self.require_generation(expected_generation)?;
        self.require_owner(owner)?;
        if self.snapshot.phase != OutputLeasePhase::HeldActive {
            return Err(OutputLeaseError::InvalidTransition);
        }
        if now_ms >= self.snapshot.expires_at_monotonic_ms.unwrap_or(0) {
            self.orphan_without_owner_check()?;
            return Err(OutputLeaseError::Expired);
        }
        let generation = next_generation(self.snapshot.generation)?;
        let deadline = checked_deadline(now_ms, ttl_ms)?;
        self.snapshot.generation = generation;
        self.snapshot.expires_at_monotonic_ms = Some(deadline);
        Ok(self.snapshot())
    }

    /// Observing expiry changes authority only. There is no physical-output API here.
    #[cfg(test)]
    fn observe_expiry(&mut self, now_ms: u64) -> Result<bool, OutputLeaseError> {
        if self.snapshot.phase != OutputLeasePhase::HeldActive
            || now_ms < self.snapshot.expires_at_monotonic_ms.unwrap_or(u64::MAX)
        {
            return Ok(false);
        }
        self.orphan_without_owner_check()?;
        Ok(true)
    }

    /// Used by owner retirement and project-identity replacement.
    fn orphan(
        &mut self,
        expected_generation: u64,
    ) -> Result<OutputLeaseSnapshot, OutputLeaseError> {
        self.require_generation(expected_generation)?;
        if self.snapshot.phase != OutputLeasePhase::HeldActive {
            return Err(OutputLeaseError::InvalidTransition);
        }
        self.orphan_without_owner_check()?;
        Ok(self.snapshot())
    }

    fn recover(
        &mut self,
        owner: &OutputLeaseOwner,
        expected_generation: u64,
        now_ms: u64,
        ttl_ms: u64,
    ) -> Result<OutputLeaseSnapshot, OutputLeaseError> {
        self.require_generation(expected_generation)?;
        self.require_owner(owner)?;
        if self.snapshot.phase != OutputLeasePhase::HeldOrphaned {
            return Err(OutputLeaseError::InvalidTransition);
        }
        let generation = next_generation(self.snapshot.generation)?;
        let deadline = checked_deadline(now_ms, ttl_ms)?;
        self.snapshot.phase = OutputLeasePhase::HeldActive;
        self.snapshot.generation = generation;
        self.snapshot.expires_at_monotonic_ms = Some(deadline);
        Ok(self.snapshot())
    }

    fn relinquish_output_lease(
        &mut self,
        owner: &OutputLeaseOwner,
        expected_generation: u64,
    ) -> Result<OutputLeaseSnapshot, OutputLeaseError> {
        self.require_generation(expected_generation)?;
        self.require_owner(owner)?;
        if !matches!(
            self.snapshot.phase,
            OutputLeasePhase::HeldActive | OutputLeasePhase::HeldOrphaned
        ) {
            return Err(OutputLeaseError::InvalidTransition);
        }
        let generation = next_generation(self.snapshot.generation)?;
        self.snapshot = OutputLeaseSnapshot {
            phase: OutputLeasePhase::Unclaimed,
            owner: None,
            resources: None,
            generation,
            expires_at_monotonic_ms: None,
        };
        Ok(self.snapshot())
    }

    /// Atomic authority-only transfer. The pending phase is built on a clone and
    /// never becomes externally visible if deadline/generation validation fails.
    fn force_transfer(
        &mut self,
        expected_generation: u64,
        new_owner: OutputLeaseOwner,
        now_ms: u64,
        ttl_ms: u64,
    ) -> Result<OutputLeaseSnapshot, OutputLeaseError> {
        self.require_generation(expected_generation)?;
        self.require_current_process(&new_owner)?;
        if self.snapshot.phase == OutputLeasePhase::HeldActive
            && now_ms >= self.snapshot.expires_at_monotonic_ms.unwrap_or(0)
        {
            self.orphan_without_owner_check()?;
            return Err(OutputLeaseError::Expired);
        }
        if !matches!(
            self.snapshot.phase,
            OutputLeasePhase::HeldActive | OutputLeasePhase::HeldOrphaned
        ) || self.snapshot.owner.as_ref() == Some(&new_owner)
        {
            return Err(OutputLeaseError::InvalidTransition);
        }
        let generation = next_generation(self.snapshot.generation)?;
        let deadline = checked_deadline(now_ms, ttl_ms)?;
        let mut candidate = self.snapshot.clone();
        candidate.phase = OutputLeasePhase::HeldActive;
        candidate.owner = Some(new_owner);
        candidate.generation = generation;
        candidate.expires_at_monotonic_ms = Some(deadline);
        self.snapshot = candidate;
        Ok(self.snapshot())
    }

    fn orphan_without_owner_check(&mut self) -> Result<(), OutputLeaseError> {
        let generation = next_generation(self.snapshot.generation)?;
        self.snapshot.phase = OutputLeasePhase::HeldOrphaned;
        self.snapshot.generation = generation;
        self.snapshot.expires_at_monotonic_ms = None;
        Ok(())
    }

    fn require_owner(&self, owner: &OutputLeaseOwner) -> Result<(), OutputLeaseError> {
        if self.snapshot.owner.as_ref() == Some(owner) {
            Ok(())
        } else {
            Err(OutputLeaseError::StaleOwner)
        }
    }

    fn require_current_process(&self, owner: &OutputLeaseOwner) -> Result<(), OutputLeaseError> {
        if owner.process_session_incarnation == self.process_session_incarnation {
            Ok(())
        } else {
            Err(OutputLeaseError::StaleOwner)
        }
    }

    fn require_generation(&self, expected: u64) -> Result<(), OutputLeaseError> {
        if self.snapshot.generation == expected {
            Ok(())
        } else {
            Err(OutputLeaseError::StaleGeneration)
        }
    }
}

fn next_generation(current: u64) -> Result<u64, OutputLeaseError> {
    current
        .checked_add(1)
        .ok_or(OutputLeaseError::GenerationExhausted)
}

fn checked_deadline(now_ms: u64, ttl_ms: u64) -> Result<u64, OutputLeaseError> {
    if ttl_ms == 0 || ttl_ms > MAX_OUTPUT_LEASE_TTL_MS {
        return Err(OutputLeaseError::InvalidTtl);
    }
    now_ms
        .checked_add(ttl_ms)
        .ok_or(OutputLeaseError::ClockExhausted)
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct OutputLeaseRequestKey {
    pub(crate) principal: String,
    pub(crate) domain: String,
    pub(crate) request_id: u64,
}

impl OutputLeaseRequestKey {
    pub(crate) fn new(
        principal: impl Into<String>,
        domain: impl Into<String>,
        request_id: u64,
    ) -> Result<Self, OutputLeaseError> {
        let key = Self {
            principal: principal.into(),
            domain: domain.into(),
            request_id,
        };
        if !bounded_nonempty(&key.principal, MAX_OUTPUT_LEASE_REQUEST_PRINCIPAL_BYTES)
            || !bounded_nonempty(&key.domain, MAX_OUTPUT_LEASE_REQUEST_DOMAIN_BYTES)
            || key.request_id == 0
        {
            return Err(OutputLeaseError::InvalidRequest);
        }
        Ok(key)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct OutputLeaseRequestOrigin {
    principal: String,
    domain: String,
}

impl From<&OutputLeaseRequestKey> for OutputLeaseRequestOrigin {
    fn from(key: &OutputLeaseRequestKey) -> Self {
        Self {
            principal: key.principal.clone(),
            domain: key.domain.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct OutputLeaseShapeHash([u8; 32]);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OutputLeaseRequestShape {
    pub(crate) operation: String,
    pub(crate) lease_id: Option<OutputLeaseId>,
    pub(crate) owner: Option<OutputLeaseOwner>,
    pub(crate) resources: Option<OutputLeaseResources>,
    pub(crate) resource_scope: Option<OutputLeaseResources>,
    pub(crate) expected_generation: Option<u64>,
    pub(crate) ttl_ms: Option<u64>,
    pub(crate) transfer_owner: Option<OutputLeaseOwner>,
    pub(crate) project_identity: Option<String>,
    pub(crate) selected_lease_ids: Vec<OutputLeaseId>,
}

impl OutputLeaseRequestShape {
    pub(crate) fn new(operation: impl Into<String>) -> Result<Self, OutputLeaseError> {
        let operation = operation.into();
        if !bounded_nonempty(&operation, MAX_OUTPUT_LEASE_OPERATION_BYTES) {
            return Err(OutputLeaseError::InvalidRequest);
        }
        Ok(Self {
            operation,
            lease_id: None,
            owner: None,
            resources: None,
            resource_scope: None,
            expected_generation: None,
            ttl_ms: None,
            transfer_owner: None,
            project_identity: None,
            selected_lease_ids: Vec::new(),
        })
    }

    pub(crate) fn canonical_hash(&self) -> Result<OutputLeaseShapeHash, OutputLeaseError> {
        if !bounded_nonempty(&self.operation, MAX_OUTPUT_LEASE_OPERATION_BYTES) {
            return Err(OutputLeaseError::InvalidRequest);
        }
        if self
            .project_identity
            .as_deref()
            .is_some_and(|project| !bounded_nonempty(project, MAX_OUTPUT_LEASE_PROJECT_BYTES))
        {
            return Err(OutputLeaseError::InvalidProject);
        }
        let mut bytes = Vec::new();
        append_string(&mut bytes, &self.operation)?;
        append_optional_lease_id(&mut bytes, self.lease_id);
        append_optional_owner(&mut bytes, self.owner.as_ref())?;
        append_optional_resources(&mut bytes, self.resources.as_ref())?;
        append_optional_resources(&mut bytes, self.resource_scope.as_ref())?;
        append_optional_u64(&mut bytes, self.expected_generation);
        append_optional_u64(&mut bytes, self.ttl_ms);
        append_optional_owner(&mut bytes, self.transfer_owner.as_ref())?;
        append_optional_string(&mut bytes, self.project_identity.as_deref())?;
        let mut selected = self.selected_lease_ids.clone();
        if selected.len() > MAX_OUTPUT_LEASE_SELECTED_IDS {
            return Err(OutputLeaseError::InvalidRequest);
        }
        selected.sort_unstable();
        if selected.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(OutputLeaseError::InvalidRequest);
        }
        append_u64(
            &mut bytes,
            u64::try_from(selected.len()).map_err(|_| OutputLeaseError::InvalidRequest)?,
        );
        for lease_id in selected {
            append_u64(&mut bytes, lease_id.0);
        }
        let digest = Sha256::digest(bytes);
        Ok(OutputLeaseShapeHash(digest.into()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum OutputLeaseRequestAction {
    Acquire {
        owner: OutputLeaseOwner,
        resources: OutputLeaseResources,
        project_identity: String,
        ttl_ms: u64,
    },
    Renew {
        lease_id: OutputLeaseId,
        owner: OutputLeaseOwner,
        expected_generation: u64,
        ttl_ms: u64,
    },
    #[cfg(test)]
    ObserveExpiry {
        lease_id: OutputLeaseId,
    },
    Recover {
        lease_id: OutputLeaseId,
        owner: OutputLeaseOwner,
        expected_generation: u64,
        ttl_ms: u64,
    },
    Relinquish {
        lease_id: OutputLeaseId,
        owner: OutputLeaseOwner,
        expected_generation: u64,
    },
    ForceTransfer {
        lease_id: OutputLeaseId,
        expected_generation: u64,
        new_owner: OutputLeaseOwner,
        ttl_ms: u64,
    },
    AuthorizeOrdinary {
        lease_id: OutputLeaseId,
        owner: OutputLeaseOwner,
        expected_generation: u64,
        exact_resources: OutputLeaseResources,
    },
    RetireOwner {
        owner: OutputLeaseOwner,
    },
    ProjectOrphan {
        project_identity: String,
        selected_lease_ids: Vec<OutputLeaseId>,
        resource_scope: Option<OutputLeaseResources>,
    },
}

impl OutputLeaseRequestAction {
    fn operation(&self) -> &'static str {
        match self {
            Self::Acquire { .. } => "acquire",
            Self::Renew { .. } => "renew",
            #[cfg(test)]
            Self::ObserveExpiry { .. } => "observe_expiry",
            Self::Recover { .. } => "recover",
            Self::Relinquish { .. } => "relinquish_output_lease",
            Self::ForceTransfer { .. } => "force_transfer",
            Self::AuthorizeOrdinary { .. } => "authorize_ordinary",
            Self::RetireOwner { .. } => "retire_owner",
            Self::ProjectOrphan { .. } => "project_orphan",
        }
    }

    fn shape(&self) -> Result<OutputLeaseRequestShape, OutputLeaseError> {
        let mut shape = OutputLeaseRequestShape::new(self.operation())?;
        match self {
            Self::Acquire {
                owner,
                resources,
                project_identity,
                ttl_ms,
            } => {
                shape.owner = Some(owner.clone());
                shape.resources = Some(resources.clone());
                shape.project_identity = Some(project_identity.clone());
                shape.ttl_ms = Some(*ttl_ms);
            }
            Self::Renew {
                lease_id,
                owner,
                expected_generation,
                ttl_ms,
            }
            | Self::Recover {
                lease_id,
                owner,
                expected_generation,
                ttl_ms,
            } => {
                shape.lease_id = Some(*lease_id);
                shape.owner = Some(owner.clone());
                shape.expected_generation = Some(*expected_generation);
                shape.ttl_ms = Some(*ttl_ms);
            }
            #[cfg(test)]
            Self::ObserveExpiry { lease_id } => {
                shape.lease_id = Some(*lease_id);
            }
            Self::Relinquish {
                lease_id,
                owner,
                expected_generation,
            } => {
                shape.lease_id = Some(*lease_id);
                shape.owner = Some(owner.clone());
                shape.expected_generation = Some(*expected_generation);
            }
            Self::ForceTransfer {
                lease_id,
                expected_generation,
                new_owner,
                ttl_ms,
            } => {
                shape.lease_id = Some(*lease_id);
                shape.expected_generation = Some(*expected_generation);
                shape.transfer_owner = Some(new_owner.clone());
                shape.ttl_ms = Some(*ttl_ms);
            }
            Self::AuthorizeOrdinary {
                lease_id,
                owner,
                expected_generation,
                exact_resources,
            } => {
                shape.lease_id = Some(*lease_id);
                shape.owner = Some(owner.clone());
                shape.expected_generation = Some(*expected_generation);
                shape.resources = Some(exact_resources.clone());
            }
            Self::RetireOwner { owner } => {
                shape.owner = Some(owner.clone());
            }
            Self::ProjectOrphan {
                project_identity,
                selected_lease_ids,
                resource_scope,
            } => {
                shape.project_identity = Some(project_identity.clone());
                shape.selected_lease_ids = selected_lease_ids.clone();
                shape.resource_scope = resource_scope.clone();
            }
        }
        Ok(shape)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OutputLeaseRequest {
    pub(crate) key: OutputLeaseRequestKey,
    pub(crate) shape: OutputLeaseRequestShape,
    pub(crate) shape_hash: OutputLeaseShapeHash,
    pub(crate) action: Option<OutputLeaseRequestAction>,
}

impl OutputLeaseRequest {
    #[cfg(test)]
    pub(crate) fn new(
        key: OutputLeaseRequestKey,
        shape: OutputLeaseRequestShape,
    ) -> Result<Self, OutputLeaseError> {
        let shape_hash = shape.canonical_hash()?;
        Ok(Self {
            key,
            shape,
            shape_hash,
            action: None,
        })
    }

    pub(crate) fn from_action(
        principal: impl Into<String>,
        domain: impl Into<String>,
        request_id: u64,
        action: OutputLeaseRequestAction,
    ) -> Result<Self, OutputLeaseError> {
        let key = OutputLeaseRequestKey::new(principal, domain, request_id)?;
        let shape = action.shape()?;
        let shape_hash = shape.canonical_hash()?;
        Ok(Self {
            key,
            shape,
            shape_hash,
            action: Some(action),
        })
    }
}

fn append_u64(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn append_string(bytes: &mut Vec<u8>, value: &str) -> Result<(), OutputLeaseError> {
    append_u64(
        bytes,
        u64::try_from(value.len()).map_err(|_| OutputLeaseError::InvalidRequest)?,
    );
    bytes.extend_from_slice(value.as_bytes());
    Ok(())
}

fn append_optional_string(
    bytes: &mut Vec<u8>,
    value: Option<&str>,
) -> Result<(), OutputLeaseError> {
    match value {
        Some(value) => {
            bytes.push(1);
            append_string(bytes, value)?;
        }
        None => bytes.push(0),
    }
    Ok(())
}

fn append_optional_u64(bytes: &mut Vec<u8>, value: Option<u64>) {
    match value {
        Some(value) => {
            bytes.push(1);
            append_u64(bytes, value);
        }
        None => bytes.push(0),
    }
}

fn append_optional_lease_id(bytes: &mut Vec<u8>, value: Option<OutputLeaseId>) {
    append_optional_u64(bytes, value.map(|lease_id| lease_id.0));
}

fn append_optional_owner(
    bytes: &mut Vec<u8>,
    owner: Option<&OutputLeaseOwner>,
) -> Result<(), OutputLeaseError> {
    match owner {
        Some(owner) => {
            bytes.push(1);
            append_string(bytes, &owner.principal)?;
            append_string(bytes, &owner.window_label)?;
            append_u64(bytes, owner.process_session_incarnation);
            append_u64(bytes, owner.owner_incarnation);
        }
        None => bytes.push(0),
    }
    Ok(())
}

fn append_optional_resources(
    bytes: &mut Vec<u8>,
    resources: Option<&OutputLeaseResources>,
) -> Result<(), OutputLeaseError> {
    match resources {
        Some(resources) => {
            bytes.push(1);
            append_u64(
                bytes,
                u64::try_from(resources.0.len()).map_err(|_| OutputLeaseError::InvalidRequest)?,
            );
            for resource in &resources.0 {
                bytes.push(match resource {
                    OutputLeaseResource::Lighting => 1,
                    OutputLeaseResource::Video => 2,
                });
            }
        }
        None => bytes.push(0),
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum OutputLeaseOperationOutcome {
    Acquired,
    Renewed,
    #[cfg(test)]
    ExpiryObserved,
    Recovered,
    Relinquished,
    Transferred,
    OwnerRetired,
    ProjectOrphaned,
    Authorized,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct OutputLeaseRequestReceipt {
    pub(crate) key: OutputLeaseRequestKey,
    pub(crate) shape_hash: OutputLeaseShapeHash,
    pub(crate) lease_id: Option<OutputLeaseId>,
    pub(crate) affected_lease_ids: Vec<OutputLeaseId>,
    pub(crate) owner: Option<OutputLeaseOwner>,
    pub(crate) process_session_incarnation: u64,
    pub(crate) resources: Option<OutputLeaseResources>,
    pub(crate) generation_before: Option<u64>,
    pub(crate) generation_after: Option<u64>,
    pub(crate) audit_sequence: u64,
    pub(crate) changes: Vec<OutputLeaseChange>,
    pub(crate) outcome: Result<OutputLeaseOperationOutcome, OutputLeaseError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct OutputLeaseChange {
    pub(crate) lease_id: OutputLeaseId,
    pub(crate) before: Option<OutputLeaseSnapshot>,
    pub(crate) after: Option<OutputLeaseSnapshot>,
}

fn validate_persisted_owner(owner: &OutputLeaseOwner) -> Result<(), OutputLeaseError> {
    if !bounded_nonempty(&owner.principal, MAX_OUTPUT_LEASE_OWNER_PRINCIPAL_BYTES)
        || !bounded_nonempty(&owner.window_label, MAX_OUTPUT_LEASE_OWNER_WINDOW_BYTES)
        || owner.process_session_incarnation == 0
        || owner.owner_incarnation == 0
    {
        return Err(OutputLeaseError::InvalidOwner);
    }
    Ok(())
}

fn validate_persisted_resources(resources: &OutputLeaseResources) -> Result<(), OutputLeaseError> {
    if resources.0.len() > 2 {
        return Err(OutputLeaseError::InvalidResources);
    }
    OutputLeaseResources::new(&resources.0).map(|_| ())
}

fn validate_persisted_snapshot(snapshot: &OutputLeaseSnapshot) -> Result<(), OutputLeaseError> {
    if snapshot.generation == 0 && snapshot.phase != OutputLeasePhase::Unclaimed {
        return Err(OutputLeaseError::InvalidTransition);
    }
    if let Some(owner) = snapshot.owner.as_ref() {
        validate_persisted_owner(owner)?;
    }
    if let Some(resources) = snapshot.resources.as_ref() {
        validate_persisted_resources(resources)?;
    }
    match snapshot.phase {
        OutputLeasePhase::Unclaimed => {
            if snapshot.owner.is_some()
                || snapshot.resources.is_some()
                || snapshot.expires_at_monotonic_ms.is_some()
            {
                return Err(OutputLeaseError::InvalidTransition);
            }
        }
        OutputLeasePhase::HeldActive => {
            if snapshot.owner.is_none()
                || snapshot.resources.is_none()
                || snapshot.expires_at_monotonic_ms.is_none()
            {
                return Err(OutputLeaseError::InvalidTransition);
            }
        }
        OutputLeasePhase::HeldOrphaned => {
            if snapshot.owner.is_none()
                || snapshot.resources.is_none()
                || snapshot.expires_at_monotonic_ms.is_some()
            {
                return Err(OutputLeaseError::InvalidTransition);
            }
        }
    }
    Ok(())
}

pub(crate) fn validate_persisted_request_key(
    key: &OutputLeaseRequestKey,
) -> Result<(), OutputLeaseError> {
    OutputLeaseRequestKey::new(key.principal.clone(), key.domain.clone(), key.request_id)
        .map(|_| ())
}

pub(crate) fn validate_persisted_shape_hash(
    shape_hash: &OutputLeaseShapeHash,
) -> Result<(), OutputLeaseError> {
    if shape_hash.0.iter().all(|byte| *byte == 0) {
        Err(OutputLeaseError::InvalidRequest)
    } else {
        Ok(())
    }
}

pub(crate) fn validate_persisted_receipt(
    receipt: &OutputLeaseRequestReceipt,
) -> Result<(), OutputLeaseError> {
    validate_persisted_request_key(&receipt.key)?;
    validate_persisted_shape_hash(&receipt.shape_hash)?;
    if receipt.process_session_incarnation == 0
        || receipt.audit_sequence == 0
        || receipt.affected_lease_ids.len() > MAX_OUTPUT_LEASE_SELECTED_IDS
        || receipt.changes.len() > MAX_OUTPUT_LEASES
    {
        return Err(OutputLeaseError::InvalidRequest);
    }
    if let Some(lease_id) = receipt.lease_id {
        if lease_id.0 == 0 {
            return Err(OutputLeaseError::InvalidRequest);
        }
        if !receipt.affected_lease_ids.contains(&lease_id) {
            return Err(OutputLeaseError::InvalidRequest);
        }
    }
    if receipt
        .affected_lease_ids
        .iter()
        .any(|lease_id| lease_id.0 == 0)
        || receipt
            .affected_lease_ids
            .windows(2)
            .any(|pair| pair[0] >= pair[1])
    {
        return Err(OutputLeaseError::InvalidRequest);
    }
    if let Some(owner) = receipt.owner.as_ref() {
        validate_persisted_owner(owner)?;
    }
    if let Some(resources) = receipt.resources.as_ref() {
        validate_persisted_resources(resources)?;
    }
    for (index, change) in receipt.changes.iter().enumerate() {
        if change.lease_id.0 == 0
            || change.before.is_none() && change.after.is_none()
            || receipt
                .changes
                .iter()
                .take(index)
                .any(|previous| previous.lease_id == change.lease_id)
        {
            return Err(OutputLeaseError::InvalidRequest);
        }
        if let Some(before) = change.before.as_ref() {
            validate_persisted_snapshot(before)?;
        }
        if let Some(after) = change.after.as_ref() {
            validate_persisted_snapshot(after)?;
        }
    }
    if let Some(first_change) = receipt.changes.first() {
        if receipt.generation_before
            != first_change
                .before
                .as_ref()
                .map(|snapshot| snapshot.generation)
            || receipt.generation_after
                != first_change
                    .after
                    .as_ref()
                    .map(|snapshot| snapshot.generation)
        {
            return Err(OutputLeaseError::InvalidRequest);
        }
    } else if receipt.generation_before.is_some() || receipt.generation_after.is_some() {
        return Err(OutputLeaseError::InvalidRequest);
    }
    if receipt
        .changes
        .iter()
        .any(|change| !receipt.affected_lease_ids.contains(&change.lease_id))
    {
        return Err(OutputLeaseError::InvalidRequest);
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OutputLeaseAuditRecord {
    pub(crate) sequence: u64,
    pub(crate) receipt: OutputLeaseRequestReceipt,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OutputLeaseLeaseView {
    pub(crate) lease_id: OutputLeaseId,
    pub(crate) project_identity: String,
    pub(crate) snapshot: OutputLeaseSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum OutputLeaseRequestAdmission {
    Terminal(OutputLeaseRequestReceipt),
    InFlight(OutputLeaseRequestPermit),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OutputLeaseRequestPermit {
    key: OutputLeaseRequestKey,
    shape_hash: OutputLeaseShapeHash,
    process_session_incarnation: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct OutputLeaseRecord {
    state: OutputLeaseState,
    project_identity: String,
    last_touched_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct OutputLeaseTerminalRecord {
    shape_hash: OutputLeaseShapeHash,
    receipt: OutputLeaseRequestReceipt,
    expires_at_ms: u64,
    last_used_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct OutputLeaseOriginState {
    high_water_request_id: u64,
    last_used_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct OutputLeaseInFlight {
    shape_hash: OutputLeaseShapeHash,
    operation_key: OutputLeaseRateKey,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct OutputLeaseRateKey {
    principal: String,
    operation: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct OutputLeaseTokenBucket {
    tokens_milli: u64,
    observed_at_ms: u64,
}

impl OutputLeaseTokenBucket {
    fn fresh(now_ms: u64) -> Self {
        Self {
            tokens_milli: OUTPUT_LEASE_TOKEN_BURST_MILLI,
            observed_at_ms: now_ms,
        }
    }

    fn try_take(&mut self, now_ms: u64) -> Result<bool, OutputLeaseError> {
        if now_ms < self.observed_at_ms {
            return Err(OutputLeaseError::ClockRollback);
        }
        let elapsed_ms = now_ms - self.observed_at_ms;
        let refill = elapsed_ms
            .checked_mul(OUTPUT_LEASE_TOKEN_RATE_PER_SECOND_MILLI)
            .ok_or(OutputLeaseError::ClockExhausted)?;
        self.tokens_milli = self
            .tokens_milli
            .saturating_add(refill)
            .min(OUTPUT_LEASE_TOKEN_BURST_MILLI);
        self.observed_at_ms = now_ms;
        if self.tokens_milli < 1_000 {
            return Ok(false);
        }
        self.tokens_milli -= 1_000;
        Ok(true)
    }
}

#[cfg(test)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OutputLeasePurgeSummary {
    pub(crate) expired_leases_orphaned: usize,
    pub(crate) unclaimed_leases_purged: usize,
    pub(crate) receipts_purged: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OutputLeaseRegistry {
    process_session_incarnation: u64,
    leases: BTreeMap<OutputLeaseId, OutputLeaseRecord>,
    resource_index: BTreeMap<OutputLeaseResource, BTreeSet<OutputLeaseId>>,
    next_lease_id: u64,
    last_now_ms: u64,
    receipts: BTreeMap<OutputLeaseRequestKey, OutputLeaseTerminalRecord>,
    origins: BTreeMap<OutputLeaseRequestOrigin, OutputLeaseOriginState>,
    in_flight: BTreeMap<OutputLeaseRequestKey, OutputLeaseInFlight>,
    operation_lanes: BTreeSet<OutputLeaseRateKey>,
    token_buckets: BTreeMap<OutputLeaseRateKey, OutputLeaseTokenBucket>,
    reserved_receipts: usize,
    reserved_audit: usize,
    audit: VecDeque<OutputLeaseAuditRecord>,
    next_audit_sequence: u64,
}

impl OutputLeaseRegistry {
    pub(crate) fn fresh_process(
        process_session_incarnation: u64,
    ) -> Result<Self, OutputLeaseError> {
        if process_session_incarnation == 0 {
            return Err(OutputLeaseError::InvalidOwner);
        }
        Ok(Self {
            process_session_incarnation,
            leases: BTreeMap::new(),
            resource_index: BTreeMap::new(),
            next_lease_id: 1,
            last_now_ms: 0,
            receipts: BTreeMap::new(),
            origins: BTreeMap::new(),
            in_flight: BTreeMap::new(),
            operation_lanes: BTreeSet::new(),
            token_buckets: BTreeMap::new(),
            reserved_receipts: 0,
            reserved_audit: 0,
            audit: VecDeque::new(),
            next_audit_sequence: 0,
        })
    }

    pub(crate) fn process_session_incarnation(&self) -> u64 {
        self.process_session_incarnation
    }

    #[cfg(test)]
    pub(crate) fn lease_count(&self) -> usize {
        self.leases.len()
    }

    #[cfg(test)]
    pub(crate) fn active_lease_count(&self) -> usize {
        self.leases
            .values()
            .filter(|record| {
                matches!(
                    record.state.snapshot.phase,
                    OutputLeasePhase::HeldActive | OutputLeasePhase::HeldOrphaned
                )
            })
            .count()
    }

    #[cfg(test)]
    pub(crate) fn receipt_count(&self) -> usize {
        self.receipts.len()
    }

    #[cfg(test)]
    pub(crate) fn origin_count(&self) -> usize {
        self.origins.len()
    }

    #[cfg(test)]
    pub(crate) fn audit(&self) -> &VecDeque<OutputLeaseAuditRecord> {
        &self.audit
    }

    pub(crate) fn lease_view(
        &self,
        lease_id: OutputLeaseId,
    ) -> Result<OutputLeaseLeaseView, OutputLeaseError> {
        let record = self
            .leases
            .get(&lease_id)
            .ok_or(OutputLeaseError::UnknownLease)?;
        Ok(OutputLeaseLeaseView {
            lease_id,
            project_identity: record.project_identity.clone(),
            snapshot: record.state.snapshot(),
        })
    }

    pub(crate) fn lease_views(&self) -> Vec<OutputLeaseLeaseView> {
        self.leases
            .iter()
            .map(|(&lease_id, record)| OutputLeaseLeaseView {
                lease_id,
                project_identity: record.project_identity.clone(),
                snapshot: record.state.snapshot(),
            })
            .collect()
    }

    /// Read-only owner query. It never changes expiry authority or appends an
    /// audit record; an active lease at its deadline is conservatively omitted
    /// as unavailable and is orphaned by the next mutating registry request.
    pub(crate) fn query_owner_leases(
        &self,
        owner: &OutputLeaseOwner,
        now_ms: u64,
    ) -> Result<Vec<OutputLeaseLeaseView>, OutputLeaseError> {
        self.ensure_now(now_ms)?;
        if owner.process_session_incarnation != self.process_session_incarnation {
            return Err(OutputLeaseError::StaleOwner);
        }
        Ok(self
            .lease_views()
            .into_iter()
            .filter(|view| {
                if view.snapshot.owner.as_ref() != Some(owner) {
                    return false;
                }
                match view.snapshot.phase {
                    OutputLeasePhase::HeldActive => view
                        .snapshot
                        .expires_at_monotonic_ms
                        .is_some_and(|deadline| now_ms < deadline),
                    OutputLeasePhase::HeldOrphaned => true,
                    OutputLeasePhase::Unclaimed => false,
                }
            })
            .collect())
    }

    fn ensure_now(&self, now_ms: u64) -> Result<(), OutputLeaseError> {
        if now_ms < self.last_now_ms {
            Err(OutputLeaseError::ClockRollback)
        } else {
            Ok(())
        }
    }

    fn advance_now(&mut self, now_ms: u64) -> Result<(), OutputLeaseError> {
        self.ensure_now(now_ms)?;
        self.last_now_ms = now_ms;
        Ok(())
    }

    fn lookup_mut(
        &mut self,
        lease_id: OutputLeaseId,
    ) -> Result<&mut OutputLeaseRecord, OutputLeaseError> {
        self.leases
            .get_mut(&lease_id)
            .ok_or(OutputLeaseError::UnknownLease)
    }

    fn allocate_lease_id(&mut self) -> Result<OutputLeaseId, OutputLeaseError> {
        let value = self.next_lease_id;
        if value == 0 {
            return Err(OutputLeaseError::LeaseIdExhausted);
        }
        self.next_lease_id = value
            .checked_add(1)
            .ok_or(OutputLeaseError::LeaseIdExhausted)?;
        Ok(OutputLeaseId(value))
    }

    fn acquire(
        &mut self,
        owner: OutputLeaseOwner,
        resources: OutputLeaseResources,
        project_identity: impl Into<String>,
        now_ms: u64,
        ttl_ms: u64,
    ) -> Result<OutputLeaseGrant, OutputLeaseError> {
        let mut candidate = self.clone();
        let result =
            candidate.acquire_inner(owner, resources, project_identity.into(), now_ms, ttl_ms);
        match result {
            Ok(grant) => {
                *self = candidate;
                Ok(grant)
            }
            Err(error) => Err(error),
        }
    }

    fn acquire_inner(
        &mut self,
        owner: OutputLeaseOwner,
        resources: OutputLeaseResources,
        project_identity: String,
        now_ms: u64,
        ttl_ms: u64,
    ) -> Result<OutputLeaseGrant, OutputLeaseError> {
        self.advance_now(now_ms)?;
        checked_deadline(now_ms, ttl_ms)?;
        if owner.process_session_incarnation != self.process_session_incarnation {
            return Err(OutputLeaseError::StaleOwner);
        }
        if !bounded_nonempty(&project_identity, MAX_OUTPUT_LEASE_PROJECT_BYTES) {
            return Err(OutputLeaseError::InvalidProject);
        }
        let resources = OutputLeaseResources::new(resources.as_slice())?;
        self.purge_unclaimed_for_capacity(now_ms);
        if self.leases.len() >= MAX_OUTPUT_LEASES {
            return Err(OutputLeaseError::LeaseCapacity);
        }
        if self.resources_overlap(&resources) {
            return Err(OutputLeaseError::ResourceConflict);
        }
        let lease_id = self.allocate_lease_id()?;
        let mut state = OutputLeaseState::fresh_process(self.process_session_incarnation)?;
        let snapshot = state.acquire(owner, resources.clone(), now_ms, ttl_ms)?;
        self.leases.insert(
            lease_id,
            OutputLeaseRecord {
                state,
                project_identity,
                last_touched_ms: now_ms,
            },
        );
        self.index_resources(lease_id, &resources);
        Ok(OutputLeaseGrant { lease_id, snapshot })
    }

    fn renew(
        &mut self,
        lease_id: OutputLeaseId,
        owner: &OutputLeaseOwner,
        expected_generation: u64,
        now_ms: u64,
        ttl_ms: u64,
    ) -> Result<OutputLeaseSnapshot, OutputLeaseError> {
        let mut candidate = self.clone();
        let result = candidate.renew_inner(lease_id, owner, expected_generation, now_ms, ttl_ms);
        match result {
            Ok(snapshot) => {
                *self = candidate;
                Ok(snapshot)
            }
            Err(OutputLeaseError::Expired) => {
                // Deadline expiry is an intentional authority transition. The
                // core has already advanced the generation to orphaned.
                *self = candidate;
                Err(OutputLeaseError::Expired)
            }
            Err(error) => Err(error),
        }
    }

    fn renew_inner(
        &mut self,
        lease_id: OutputLeaseId,
        owner: &OutputLeaseOwner,
        expected_generation: u64,
        now_ms: u64,
        ttl_ms: u64,
    ) -> Result<OutputLeaseSnapshot, OutputLeaseError> {
        self.advance_now(now_ms)?;
        checked_deadline(now_ms, ttl_ms)?;
        let record = self.lookup_mut(lease_id)?;
        let result = record
            .state
            .renew(owner, expected_generation, now_ms, ttl_ms);
        if result.is_ok() || matches!(result, Err(OutputLeaseError::Expired)) {
            record.last_touched_ms = now_ms;
        }
        result
    }

    #[cfg(test)]
    fn observe_expiry(
        &mut self,
        lease_id: OutputLeaseId,
        now_ms: u64,
    ) -> Result<bool, OutputLeaseError> {
        let mut candidate = self.clone();
        candidate.advance_now(now_ms)?;
        let record = candidate.lookup_mut(lease_id)?;
        let changed = record.state.observe_expiry(now_ms)?;
        if changed {
            record.last_touched_ms = now_ms;
        }
        *self = candidate;
        Ok(changed)
    }

    fn recover(
        &mut self,
        lease_id: OutputLeaseId,
        owner: &OutputLeaseOwner,
        expected_generation: u64,
        now_ms: u64,
        ttl_ms: u64,
    ) -> Result<OutputLeaseSnapshot, OutputLeaseError> {
        let mut candidate = self.clone();
        candidate.advance_now(now_ms)?;
        checked_deadline(now_ms, ttl_ms)?;
        let record = candidate.lookup_mut(lease_id)?;
        let snapshot = record
            .state
            .recover(owner, expected_generation, now_ms, ttl_ms)?;
        record.last_touched_ms = now_ms;
        *self = candidate;
        Ok(snapshot)
    }

    fn relinquish_output_lease(
        &mut self,
        lease_id: OutputLeaseId,
        owner: &OutputLeaseOwner,
        expected_generation: u64,
        now_ms: u64,
    ) -> Result<OutputLeaseSnapshot, OutputLeaseError> {
        let mut candidate = self.clone();
        candidate.advance_now(now_ms)?;
        let resources = candidate
            .leases
            .get(&lease_id)
            .ok_or(OutputLeaseError::UnknownLease)?
            .state
            .snapshot()
            .resources;
        let record = candidate.lookup_mut(lease_id)?;
        let snapshot = record
            .state
            .relinquish_output_lease(owner, expected_generation)?;
        record.last_touched_ms = now_ms;
        if let Some(resources) = resources.as_ref() {
            candidate.unindex_resources(lease_id, resources);
        }
        *self = candidate;
        Ok(snapshot)
    }

    fn force_transfer(
        &mut self,
        lease_id: OutputLeaseId,
        expected_generation: u64,
        new_owner: OutputLeaseOwner,
        now_ms: u64,
        ttl_ms: u64,
    ) -> Result<OutputLeaseSnapshot, OutputLeaseError> {
        let mut candidate = self.clone();
        candidate.advance_now(now_ms)?;
        checked_deadline(now_ms, ttl_ms)?;
        let record = candidate.lookup_mut(lease_id)?;
        let result = record
            .state
            .force_transfer(expected_generation, new_owner, now_ms, ttl_ms);
        if result.is_ok() || matches!(result, Err(OutputLeaseError::Expired)) {
            record.last_touched_ms = now_ms;
        }
        match result {
            Ok(snapshot) => {
                *self = candidate;
                Ok(snapshot)
            }
            Err(OutputLeaseError::Expired) => {
                *self = candidate;
                Err(OutputLeaseError::Expired)
            }
            Err(error) => Err(error),
        }
    }

    fn authorize_ordinary(
        &mut self,
        lease_id: OutputLeaseId,
        owner: &OutputLeaseOwner,
        expected_generation: u64,
        exact_resources: &OutputLeaseResources,
        now_ms: u64,
    ) -> Result<OutputLeaseSnapshot, OutputLeaseError> {
        let mut candidate = self.clone();
        candidate.advance_now(now_ms)?;
        let exact_resources = OutputLeaseResources::new(exact_resources.as_slice())?;
        let record = candidate.lookup_mut(lease_id)?;
        record.state.require_current_process(owner)?;
        record.state.require_generation(expected_generation)?;
        if record.state.snapshot.phase != OutputLeasePhase::HeldActive {
            return Err(OutputLeaseError::InvalidTransition);
        }
        if record.state.snapshot.resources.as_ref() != Some(&exact_resources) {
            return Err(OutputLeaseError::ResourceConflict);
        }
        record.state.require_owner(owner)?;
        if now_ms >= record.state.snapshot.expires_at_monotonic_ms.unwrap_or(0) {
            record.state.orphan_without_owner_check()?;
            record.last_touched_ms = now_ms;
            *self = candidate;
            return Err(OutputLeaseError::Expired);
        }
        Ok(record.state.snapshot())
    }

    fn resources_overlap(&self, resources: &OutputLeaseResources) -> bool {
        resources.as_slice().iter().any(|resource| {
            self.resource_index
                .get(resource)
                .is_some_and(|ids| !ids.is_empty())
        })
    }

    fn index_resources(&mut self, lease_id: OutputLeaseId, resources: &OutputLeaseResources) {
        for resource in resources.as_slice() {
            self.resource_index
                .entry(*resource)
                .or_default()
                .insert(lease_id);
        }
    }

    fn unindex_resources(&mut self, lease_id: OutputLeaseId, resources: &OutputLeaseResources) {
        for resource in resources.as_slice() {
            if let Some(ids) = self.resource_index.get_mut(resource) {
                ids.remove(&lease_id);
                if ids.is_empty() {
                    self.resource_index.remove(resource);
                }
            }
        }
    }

    fn retire_owner(
        &mut self,
        owner: &OutputLeaseOwner,
        now_ms: u64,
    ) -> Result<Vec<OutputLeaseId>, OutputLeaseError> {
        let mut candidate = self.clone();
        candidate.advance_now(now_ms)?;
        if owner.process_session_incarnation != candidate.process_session_incarnation {
            return Err(OutputLeaseError::StaleOwner);
        }
        let ids = candidate
            .leases
            .iter()
            .filter_map(|(&lease_id, record)| {
                (record.state.snapshot.owner.as_ref() == Some(owner)
                    && record.state.snapshot.phase == OutputLeasePhase::HeldActive)
                    .then_some(lease_id)
            })
            .collect::<Vec<_>>();
        for lease_id in &ids {
            let record = candidate.lookup_mut(*lease_id)?;
            let generation = record.state.snapshot.generation;
            record.state.orphan(generation)?;
            record.last_touched_ms = now_ms;
        }
        *self = candidate;
        Ok(ids)
    }

    /// Orphan only selected leases belonging to `project_identity` and, when
    /// supplied, overlapping the selected resource scope. Empty selection
    /// means all leases in that project; no physical operation is possible.
    fn project_orphan(
        &mut self,
        project_identity: &str,
        selected_lease_ids: &[OutputLeaseId],
        resource_scope: Option<&OutputLeaseResources>,
        now_ms: u64,
    ) -> Result<Vec<OutputLeaseId>, OutputLeaseError> {
        let mut candidate = self.clone();
        candidate.advance_now(now_ms)?;
        if !bounded_nonempty(project_identity, MAX_OUTPUT_LEASE_PROJECT_BYTES) {
            return Err(OutputLeaseError::InvalidProject);
        }
        let mut selected = selected_lease_ids.to_vec();
        selected.sort_unstable();
        if selected.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(OutputLeaseError::InvalidRequest);
        }
        if let Some(scope) = resource_scope {
            OutputLeaseResources::new(scope.as_slice())?;
        }
        let is_selected = |lease_id: OutputLeaseId| {
            selected.is_empty() || selected.binary_search(&lease_id).is_ok()
        };
        let ids = candidate
            .leases
            .iter()
            .filter_map(|(&lease_id, record)| {
                if record.project_identity != project_identity
                    || !is_selected(lease_id)
                    || record.state.snapshot.phase != OutputLeasePhase::HeldActive
                {
                    return None;
                }
                if let Some(scope) = resource_scope {
                    if !record
                        .state
                        .snapshot
                        .resources
                        .as_ref()
                        .is_some_and(|resources| resources.overlaps(scope))
                    {
                        return None;
                    }
                }
                Some(lease_id)
            })
            .collect::<Vec<_>>();
        for lease_id in &ids {
            let record = candidate.lookup_mut(*lease_id)?;
            let generation = record.state.snapshot.generation;
            record.state.orphan(generation)?;
            record.last_touched_ms = now_ms;
        }
        *self = candidate;
        Ok(ids)
    }

    fn purge_unclaimed_for_capacity(&mut self, now_ms: u64) {
        let stale = self
            .leases
            .iter()
            .filter_map(|(&lease_id, record)| {
                (record.state.snapshot.phase == OutputLeasePhase::Unclaimed
                    && unclaimed_retention_elapsed(record.last_touched_ms, now_ms))
                .then_some(lease_id)
            })
            .collect::<Vec<_>>();
        for lease_id in stale {
            self.leases.remove(&lease_id);
        }
    }

    #[cfg(test)]
    fn purge(&mut self, now_ms: u64) -> Result<OutputLeasePurgeSummary, OutputLeaseError> {
        let mut candidate = self.clone();
        candidate.advance_now(now_ms)?;
        let mut summary = OutputLeasePurgeSummary {
            expired_leases_orphaned: 0,
            unclaimed_leases_purged: 0,
            receipts_purged: 0,
        };
        let active_ids = candidate
            .leases
            .iter()
            .filter_map(|(&lease_id, record)| {
                (record.state.snapshot.phase == OutputLeasePhase::HeldActive).then_some(lease_id)
            })
            .collect::<Vec<_>>();
        for lease_id in active_ids {
            let record = candidate.lookup_mut(lease_id)?;
            if record.state.observe_expiry(now_ms)? {
                record.last_touched_ms = now_ms;
                summary.expired_leases_orphaned += 1;
            }
        }
        let stale_leases = candidate
            .leases
            .iter()
            .filter_map(|(&lease_id, record)| {
                (record.state.snapshot.phase == OutputLeasePhase::Unclaimed
                    && unclaimed_retention_elapsed(record.last_touched_ms, now_ms))
                .then_some(lease_id)
            })
            .collect::<Vec<_>>();
        for lease_id in stale_leases {
            candidate.leases.remove(&lease_id);
            summary.unclaimed_leases_purged += 1;
        }
        candidate.purge_inactive_token_buckets(now_ms);
        summary.receipts_purged = candidate.purge_receipts(now_ms)?;
        *self = candidate;
        Ok(summary)
    }

    fn purge_receipts(&mut self, now_ms: u64) -> Result<usize, OutputLeaseError> {
        checked_deadline(now_ms, OUTPUT_LEASE_RECEIPT_TTL_MS)?;
        let mut purged = 0;
        let expired = self
            .receipts
            .iter()
            .filter_map(|(key, record)| (record.expires_at_ms <= now_ms).then_some(key.clone()))
            .collect::<Vec<_>>();
        for key in expired {
            self.receipts.remove(&key);
            purged += 1;
        }
        Ok(purged)
    }

    fn make_room_for_receipt(&mut self, now_ms: u64) -> Result<(), OutputLeaseError> {
        self.purge_receipts(now_ms)?;
        while self.receipts.len() + self.reserved_receipts >= MAX_OUTPUT_LEASE_REQUESTS {
            let oldest = self
                .receipts
                .iter()
                .min_by_key(|(_, record)| record.last_used_ms)
                .map(|(key, _)| key.clone());
            let Some(oldest) = oldest else {
                return Err(OutputLeaseError::RequestCapacity);
            };
            self.receipts.remove(&oldest);
        }
        Ok(())
    }

    fn purge_inactive_token_buckets(&mut self, now_ms: u64) {
        let stale = self
            .token_buckets
            .iter()
            .filter_map(|(key, bucket)| {
                let lane_in_use = self.operation_lanes.contains(key)
                    || self
                        .in_flight
                        .values()
                        .any(|in_flight| in_flight.operation_key == *key);
                (!lane_in_use && bucket_idle_elapsed(bucket.observed_at_ms, now_ms))
                    .then_some(key.clone())
            })
            .collect::<Vec<_>>();
        for key in stale {
            self.token_buckets.remove(&key);
        }
    }

    #[cfg(test)]
    fn begin_request(
        &mut self,
        request: &OutputLeaseRequest,
        now_ms: u64,
    ) -> Result<OutputLeaseRequestAdmission, OutputLeaseError> {
        let expected_hash = request.shape.canonical_hash()?;
        if expected_hash != request.shape_hash {
            return Err(OutputLeaseError::InvalidRequest);
        }
        let mut candidate = self.clone();
        let admission = candidate.begin_request_inner(request, now_ms)?;
        if matches!(admission, OutputLeaseRequestAdmission::InFlight(_)) {
            *self = candidate;
        }
        Ok(admission)
    }

    fn begin_request_inner(
        &mut self,
        request: &OutputLeaseRequest,
        now_ms: u64,
    ) -> Result<OutputLeaseRequestAdmission, OutputLeaseError> {
        if request.shape.canonical_hash()? != request.shape_hash {
            return Err(OutputLeaseError::InvalidRequest);
        }
        self.advance_now(now_ms)?;
        self.purge_receipts(now_ms)?;
        if let Some(record) = self.receipts.get_mut(&request.key) {
            record.last_used_ms = now_ms.max(record.last_used_ms);
            if record.shape_hash == request.shape_hash {
                return Ok(OutputLeaseRequestAdmission::Terminal(
                    record.receipt.clone(),
                ));
            }
            return Err(OutputLeaseError::Conflict);
        }
        if let Some(in_flight) = self.in_flight.get(&request.key) {
            // An in-flight different-shape request is a conflict, not Busy;
            // this ordering prevents a caller from probing a live request.
            if in_flight.shape_hash != request.shape_hash {
                return Err(OutputLeaseError::Conflict);
            }
            return Err(OutputLeaseError::Busy);
        }
        let origin = OutputLeaseRequestOrigin::from(&request.key);
        if self
            .origins
            .get(&origin)
            .is_some_and(|state| request.key.request_id <= state.high_water_request_id)
        {
            return Err(OutputLeaseError::ReceiptNotRetained);
        }
        if !self.origins.contains_key(&origin)
            && self.origins.len() >= MAX_OUTPUT_LEASE_REQUEST_ORIGINS
        {
            return Err(OutputLeaseError::RequestCapacity);
        }
        let operation_key = OutputLeaseRateKey {
            principal: request.key.principal.clone(),
            operation: request.shape.operation.clone(),
        };
        if self.operation_lanes.contains(&operation_key) {
            return Err(OutputLeaseError::Busy);
        }
        if self.in_flight.len() >= MAX_OUTPUT_LEASE_LANES {
            return Err(OutputLeaseError::LaneCapacity);
        }
        self.make_room_for_receipt(now_ms)?;
        if !self.token_buckets.contains_key(&operation_key) {
            self.purge_inactive_token_buckets(now_ms);
            if self.token_buckets.len() >= MAX_OUTPUT_LEASE_TOKEN_BUCKETS {
                return Err(OutputLeaseError::RequestCapacity);
            }
        }
        let bucket = self
            .token_buckets
            .entry(operation_key.clone())
            .or_insert_with(|| OutputLeaseTokenBucket::fresh(now_ms));
        if !bucket.try_take(now_ms)? {
            return Err(OutputLeaseError::RateLimited);
        }
        let origin_state = self
            .origins
            .entry(origin)
            .or_insert(OutputLeaseOriginState {
                high_water_request_id: 0,
                last_used_ms: now_ms,
            });
        origin_state.high_water_request_id = request.key.request_id;
        origin_state.last_used_ms = now_ms;
        self.in_flight.insert(
            request.key.clone(),
            OutputLeaseInFlight {
                shape_hash: request.shape_hash.clone(),
                operation_key: operation_key.clone(),
            },
        );
        self.operation_lanes.insert(operation_key);
        self.reserved_receipts += 1;
        self.reserved_audit += 1;
        Ok(OutputLeaseRequestAdmission::InFlight(
            OutputLeaseRequestPermit {
                key: request.key.clone(),
                shape_hash: request.shape_hash.clone(),
                process_session_incarnation: self.process_session_incarnation,
            },
        ))
    }

    #[cfg(test)]
    fn complete_request(
        &mut self,
        permit: OutputLeaseRequestPermit,
        receipt: OutputLeaseRequestReceipt,
        now_ms: u64,
    ) -> Result<OutputLeaseRequestReceipt, OutputLeaseError> {
        let mut candidate = self.clone();
        let result = candidate.complete_request_inner(permit, receipt, now_ms);
        match result {
            Ok(receipt) => {
                *self = candidate;
                Ok(receipt)
            }
            Err(error) => Err(error),
        }
    }

    fn complete_request_inner(
        &mut self,
        permit: OutputLeaseRequestPermit,
        mut receipt: OutputLeaseRequestReceipt,
        now_ms: u64,
    ) -> Result<OutputLeaseRequestReceipt, OutputLeaseError> {
        self.advance_now(now_ms)?;
        if permit.process_session_incarnation != self.process_session_incarnation
            || receipt.key != permit.key
            || receipt.shape_hash != permit.shape_hash
        {
            return Err(OutputLeaseError::InvalidRequest);
        }
        let in_flight = self
            .in_flight
            .get(&permit.key)
            .ok_or(OutputLeaseError::RequestNotInFlight)?;
        if in_flight.shape_hash != permit.shape_hash {
            return Err(OutputLeaseError::RequestNotInFlight);
        }
        let terminal_deadline = checked_deadline(now_ms, OUTPUT_LEASE_RECEIPT_TTL_MS)?;
        let sequence = self
            .next_audit_sequence
            .checked_add(1)
            .ok_or(OutputLeaseError::AuditCapacity)?;
        if sequence == 0 || sequence > u64::MAX - 1 {
            return Err(OutputLeaseError::AuditCapacity);
        }
        let operation_key = in_flight.operation_key.clone();
        self.in_flight.remove(&permit.key);
        self.operation_lanes.remove(&operation_key);
        self.reserved_receipts = self.reserved_receipts.saturating_sub(1);
        self.reserved_audit = self.reserved_audit.saturating_sub(1);
        if self.receipts.len() >= MAX_OUTPUT_LEASE_REQUESTS {
            return Err(OutputLeaseError::RequestCapacity);
        }
        receipt.audit_sequence = sequence;
        self.receipts.insert(
            permit.key.clone(),
            OutputLeaseTerminalRecord {
                shape_hash: permit.shape_hash,
                receipt: receipt.clone(),
                expires_at_ms: terminal_deadline,
                last_used_ms: now_ms,
            },
        );
        self.next_audit_sequence = sequence;
        if self.audit.len() >= MAX_OUTPUT_LEASE_AUDIT_RECORDS {
            self.audit.pop_front();
        }
        self.audit.push_back(OutputLeaseAuditRecord {
            sequence,
            receipt: receipt.clone(),
        });
        Ok(receipt)
    }

    #[cfg(test)]
    fn cancel_request(
        &mut self,
        permit: OutputLeaseRequestPermit,
        now_ms: u64,
    ) -> Result<(), OutputLeaseError> {
        let mut candidate = self.clone();
        candidate.advance_now(now_ms)?;
        let in_flight = candidate
            .in_flight
            .get(&permit.key)
            .ok_or(OutputLeaseError::RequestNotInFlight)?;
        if in_flight.shape_hash != permit.shape_hash {
            return Err(OutputLeaseError::RequestNotInFlight);
        }
        let operation_key = in_flight.operation_key.clone();
        candidate.in_flight.remove(&permit.key);
        candidate.operation_lanes.remove(&operation_key);
        candidate.reserved_receipts = candidate.reserved_receipts.saturating_sub(1);
        candidate.reserved_audit = candidate.reserved_audit.saturating_sub(1);
        *self = candidate;
        Ok(())
    }

    pub(crate) fn submit_request(
        &mut self,
        request: &OutputLeaseRequest,
        now_ms: u64,
    ) -> Result<OutputLeaseRequestReceipt, OutputLeaseError> {
        self.submit_request_with_commit(request, now_ms, |_| Ok(()))
    }

    /// Submit one authority request while giving the caller a durable commit
    /// seam.  The candidate registry reaches its terminal receipt before the
    /// callback runs, but the live registry is not replaced until the callback
    /// has durably recorded that receipt.  A failed journal write therefore
    /// leaves both authority and receipt state uncommitted; a crash after this
    /// seam cannot expose a committed request without its terminal evidence.
    pub(crate) fn submit_request_with_commit<Commit>(
        &mut self,
        request: &OutputLeaseRequest,
        now_ms: u64,
        commit: Commit,
    ) -> Result<OutputLeaseRequestReceipt, OutputLeaseError>
    where
        Commit: FnOnce(&OutputLeaseRequestReceipt) -> Result<(), OutputLeaseError>,
    {
        let mut candidate = self.clone();
        let admission = candidate.begin_request_inner(request, now_ms)?;
        match admission {
            OutputLeaseRequestAdmission::Terminal(receipt) => Ok(receipt),
            OutputLeaseRequestAdmission::InFlight(permit) => {
                let Some(action) = request.action.as_ref() else {
                    return Err(OutputLeaseError::InvalidRequest);
                };
                let applied = candidate.apply_action(action, now_ms);
                let receipt = candidate.receipt_for_action(request, applied);
                let receipt = candidate.complete_request_inner(permit, receipt, now_ms)?;
                commit(&receipt)?;
                *self = candidate;
                Ok(receipt)
            }
        }
    }

    /// Check admission and identity on a private candidate. A fresh request
    /// reserves nothing in live authority; this seam exists so outer consent
    /// code can reject cross-operation request laundering before consuming a
    /// physical consent token.
    pub(crate) fn preflight_request(
        &self,
        request: &OutputLeaseRequest,
        now_ms: u64,
    ) -> Result<(), OutputLeaseError> {
        let mut candidate = self.clone();
        candidate.begin_request_inner(request, now_ms).map(|_| ())
    }

    fn apply_action(
        &mut self,
        action: &OutputLeaseRequestAction,
        now_ms: u64,
    ) -> OutputLeaseApplyResult {
        match action {
            OutputLeaseRequestAction::Acquire {
                owner,
                resources,
                project_identity,
                ttl_ms,
            } => {
                let result = self.acquire(
                    owner.clone(),
                    resources.clone(),
                    project_identity.clone(),
                    now_ms,
                    *ttl_ms,
                );
                match result {
                    Ok(grant) => OutputLeaseApplyResult::success(
                        vec![grant.lease_id],
                        Some(owner.clone()),
                        grant.snapshot.resources.clone(),
                        vec![OutputLeaseChange {
                            lease_id: grant.lease_id,
                            before: None,
                            after: Some(grant.snapshot.clone()),
                        }],
                        OutputLeaseOperationOutcome::Acquired,
                    ),
                    Err(error) => {
                        OutputLeaseApplyResult::error(Vec::new(), None, None, Vec::new(), error)
                    }
                }
            }
            OutputLeaseRequestAction::Renew {
                lease_id,
                owner,
                expected_generation,
                ttl_ms,
            } => self.apply_single_lease_action(
                *lease_id,
                Some(owner.clone()),
                OutputLeaseOperationOutcome::Renewed,
                |registry| registry.renew(*lease_id, owner, *expected_generation, now_ms, *ttl_ms),
            ),
            #[cfg(test)]
            OutputLeaseRequestAction::ObserveExpiry { lease_id } => {
                let before = self.lease_view(*lease_id).ok();
                let result = self.observe_expiry(*lease_id, now_ms);
                let after = self.lease_view(*lease_id).ok();
                let changes = generation_change(*lease_id, before.as_ref(), after.as_ref());
                match result {
                    Ok(_) => OutputLeaseApplyResult::success(
                        vec![*lease_id],
                        before.as_ref().and_then(|view| view.snapshot.owner.clone()),
                        after
                            .as_ref()
                            .and_then(|view| view.snapshot.resources.clone()),
                        changes,
                        OutputLeaseOperationOutcome::ExpiryObserved,
                    ),
                    Err(error) => OutputLeaseApplyResult::error(
                        vec![*lease_id],
                        before.as_ref().and_then(|view| view.snapshot.owner.clone()),
                        before
                            .as_ref()
                            .and_then(|view| view.snapshot.resources.clone()),
                        changes,
                        error,
                    ),
                }
            }
            OutputLeaseRequestAction::Recover {
                lease_id,
                owner,
                expected_generation,
                ttl_ms,
            } => self.apply_single_lease_action(
                *lease_id,
                Some(owner.clone()),
                OutputLeaseOperationOutcome::Recovered,
                |registry| {
                    registry.recover(*lease_id, owner, *expected_generation, now_ms, *ttl_ms)
                },
            ),
            OutputLeaseRequestAction::Relinquish {
                lease_id,
                owner,
                expected_generation,
            } => self.apply_single_lease_action(
                *lease_id,
                Some(owner.clone()),
                OutputLeaseOperationOutcome::Relinquished,
                |registry| {
                    registry.relinquish_output_lease(*lease_id, owner, *expected_generation, now_ms)
                },
            ),
            OutputLeaseRequestAction::ForceTransfer {
                lease_id,
                expected_generation,
                new_owner,
                ttl_ms,
            } => self.apply_single_lease_action(
                *lease_id,
                Some(new_owner.clone()),
                OutputLeaseOperationOutcome::Transferred,
                |registry| {
                    registry.force_transfer(
                        *lease_id,
                        *expected_generation,
                        new_owner.clone(),
                        now_ms,
                        *ttl_ms,
                    )
                },
            ),
            OutputLeaseRequestAction::AuthorizeOrdinary {
                lease_id,
                owner,
                expected_generation,
                exact_resources,
            } => {
                let before = self.lease_view(*lease_id).ok();
                let result = self.authorize_ordinary(
                    *lease_id,
                    owner,
                    *expected_generation,
                    exact_resources,
                    now_ms,
                );
                let after = self.lease_view(*lease_id).ok();
                let changes = match (before.as_ref(), after.as_ref()) {
                    (Some(before), Some(after)) => vec![OutputLeaseChange {
                        lease_id: *lease_id,
                        before: Some(before.snapshot.clone()),
                        after: Some(after.snapshot.clone()),
                    }],
                    _ => generation_change(*lease_id, before.as_ref(), after.as_ref()),
                };
                match result {
                    Ok(_) => OutputLeaseApplyResult::success(
                        vec![*lease_id],
                        Some(owner.clone()),
                        Some(exact_resources.clone()),
                        changes,
                        OutputLeaseOperationOutcome::Authorized,
                    ),
                    Err(error) => OutputLeaseApplyResult::error(
                        vec![*lease_id],
                        Some(owner.clone()),
                        Some(exact_resources.clone()),
                        changes,
                        error,
                    ),
                }
            }
            OutputLeaseRequestAction::RetireOwner { owner } => {
                let before = self
                    .lease_views()
                    .into_iter()
                    .filter(|view| {
                        view.snapshot.phase == OutputLeasePhase::HeldActive
                            && view.snapshot.owner.as_ref() == Some(owner)
                    })
                    .collect::<Vec<_>>();
                let result = self.retire_owner(owner, now_ms);
                let affected_lease_ids = result.clone().unwrap_or_default();
                let after = affected_lease_ids
                    .iter()
                    .filter_map(|lease_id| self.lease_view(*lease_id).ok())
                    .collect::<Vec<_>>();
                let changes = generation_changes(&before, &after);
                match result {
                    Ok(_) => OutputLeaseApplyResult::success(
                        affected_lease_ids,
                        Some(owner.clone()),
                        None,
                        changes,
                        OutputLeaseOperationOutcome::OwnerRetired,
                    ),
                    Err(error) => OutputLeaseApplyResult::error(
                        affected_lease_ids,
                        Some(owner.clone()),
                        None,
                        changes,
                        error,
                    ),
                }
            }
            OutputLeaseRequestAction::ProjectOrphan {
                project_identity,
                selected_lease_ids,
                resource_scope,
            } => {
                let before = self
                    .lease_views()
                    .into_iter()
                    .filter(|view| {
                        view.project_identity == *project_identity
                            && (selected_lease_ids.is_empty()
                                || selected_lease_ids.contains(&view.lease_id))
                            && resource_scope.as_ref().is_none_or(|scope| {
                                view.snapshot
                                    .resources
                                    .as_ref()
                                    .is_some_and(|resources| resources.overlaps(scope))
                            })
                            && view.snapshot.phase == OutputLeasePhase::HeldActive
                    })
                    .collect::<Vec<_>>();
                let result = self.project_orphan(
                    project_identity,
                    selected_lease_ids,
                    resource_scope.as_ref(),
                    now_ms,
                );
                let affected_lease_ids = result.clone().unwrap_or_default();
                let after = affected_lease_ids
                    .iter()
                    .filter_map(|lease_id| self.lease_view(*lease_id).ok())
                    .collect::<Vec<_>>();
                let changes = generation_changes(&before, &after);
                match result {
                    Ok(_) => OutputLeaseApplyResult::success(
                        affected_lease_ids,
                        None,
                        None,
                        changes,
                        OutputLeaseOperationOutcome::ProjectOrphaned,
                    ),
                    Err(error) => OutputLeaseApplyResult::error(
                        affected_lease_ids,
                        None,
                        None,
                        changes,
                        error,
                    ),
                }
            }
        }
    }

    fn apply_single_lease_action<F>(
        &mut self,
        lease_id: OutputLeaseId,
        owner: Option<OutputLeaseOwner>,
        success: OutputLeaseOperationOutcome,
        action: F,
    ) -> OutputLeaseApplyResult
    where
        F: FnOnce(&mut Self) -> Result<OutputLeaseSnapshot, OutputLeaseError>,
    {
        let before = self.lease_view(lease_id).ok();
        let result = action(self);
        let after = self.lease_view(lease_id).ok();
        let resources = before
            .as_ref()
            .and_then(|view| view.snapshot.resources.clone())
            .or_else(|| {
                after
                    .as_ref()
                    .and_then(|view| view.snapshot.resources.clone())
            });
        let changes = generation_change(lease_id, before.as_ref(), after.as_ref());
        match result {
            Ok(_) => OutputLeaseApplyResult::success(
                vec![lease_id],
                owner.or_else(|| before.and_then(|view| view.snapshot.owner)),
                resources,
                changes,
                success,
            ),
            Err(error) => OutputLeaseApplyResult::error(
                vec![lease_id],
                owner.or_else(|| before.and_then(|view| view.snapshot.owner)),
                resources,
                changes,
                error,
            ),
        }
    }

    fn receipt_for_action(
        &self,
        request: &OutputLeaseRequest,
        applied: OutputLeaseApplyResult,
    ) -> OutputLeaseRequestReceipt {
        let lease_id = applied.lease_ids.first().copied();
        let (generation_before, generation_after) = applied
            .changes
            .first()
            .map(|change| {
                (
                    change.before.as_ref().map(|snapshot| snapshot.generation),
                    change.after.as_ref().map(|snapshot| snapshot.generation),
                )
            })
            .unwrap_or((None, None));
        OutputLeaseRequestReceipt {
            key: request.key.clone(),
            shape_hash: request.shape_hash.clone(),
            lease_id,
            affected_lease_ids: applied.lease_ids,
            owner: applied.owner,
            process_session_incarnation: self.process_session_incarnation,
            resources: applied.resources,
            generation_before,
            generation_after,
            audit_sequence: 0,
            changes: applied.changes,
            outcome: applied.outcome,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct OutputLeaseApplyResult {
    lease_ids: Vec<OutputLeaseId>,
    owner: Option<OutputLeaseOwner>,
    resources: Option<OutputLeaseResources>,
    changes: Vec<OutputLeaseChange>,
    outcome: Result<OutputLeaseOperationOutcome, OutputLeaseError>,
}

impl OutputLeaseApplyResult {
    fn success(
        lease_ids: Vec<OutputLeaseId>,
        owner: Option<OutputLeaseOwner>,
        resources: Option<OutputLeaseResources>,
        changes: Vec<OutputLeaseChange>,
        outcome: OutputLeaseOperationOutcome,
    ) -> Self {
        Self {
            lease_ids,
            owner,
            resources,
            changes,
            outcome: Ok(outcome),
        }
    }

    fn error(
        lease_ids: Vec<OutputLeaseId>,
        owner: Option<OutputLeaseOwner>,
        resources: Option<OutputLeaseResources>,
        changes: Vec<OutputLeaseChange>,
        error: OutputLeaseError,
    ) -> Self {
        Self {
            lease_ids,
            owner,
            resources,
            changes,
            outcome: Err(error),
        }
    }
}

fn generation_change(
    lease_id: OutputLeaseId,
    before: Option<&OutputLeaseLeaseView>,
    after: Option<&OutputLeaseLeaseView>,
) -> Vec<OutputLeaseChange> {
    match (before, after) {
        (Some(before), Some(after)) if before.snapshot.generation != after.snapshot.generation => {
            vec![OutputLeaseChange {
                lease_id,
                before: Some(before.snapshot.clone()),
                after: Some(after.snapshot.clone()),
            }]
        }
        _ => Vec::new(),
    }
}

fn generation_changes(
    before: &[OutputLeaseLeaseView],
    after: &[OutputLeaseLeaseView],
) -> Vec<OutputLeaseChange> {
    before
        .iter()
        .filter_map(|before| {
            let after = after
                .iter()
                .find(|candidate| candidate.lease_id == before.lease_id)?;
            (before.snapshot.generation != after.snapshot.generation).then_some(OutputLeaseChange {
                lease_id: before.lease_id,
                before: Some(before.snapshot.clone()),
                after: Some(after.snapshot.clone()),
            })
        })
        .collect()
}

fn unclaimed_retention_elapsed(last_touched_ms: u64, now_ms: u64) -> bool {
    last_touched_ms
        .checked_add(OUTPUT_LEASE_UNCLAIMED_RETENTION_MS)
        .is_some_and(|deadline| now_ms >= deadline)
}

fn bucket_idle_elapsed(observed_at_ms: u64, now_ms: u64) -> bool {
    observed_at_ms
        .checked_add(OUTPUT_LEASE_TOKEN_BUCKET_IDLE_PURGE_MS)
        .is_some_and(|deadline| now_ms >= deadline)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn owner(process: u64, incarnation: u64) -> OutputLeaseOwner {
        OutputLeaseOwner::new("local-ui", "main", process, incarnation).unwrap()
    }

    fn both() -> OutputLeaseResources {
        OutputLeaseResources::new(&[OutputLeaseResource::Video, OutputLeaseResource::Lighting])
            .unwrap()
    }

    #[test]
    fn output_lease_validates_owner_and_canonical_resources() {
        assert_eq!(
            OutputLeaseOwner::new("", "main", 1, 1),
            Err(OutputLeaseError::InvalidOwner)
        );
        assert_eq!(
            OutputLeaseResources::new(&[]),
            Err(OutputLeaseError::InvalidResources)
        );
        assert_eq!(
            OutputLeaseResources::new(&[
                OutputLeaseResource::Lighting,
                OutputLeaseResource::Lighting,
            ]),
            Err(OutputLeaseError::InvalidResources)
        );
        assert_eq!(
            both().0,
            vec![OutputLeaseResource::Lighting, OutputLeaseResource::Video]
        );
    }

    #[test]
    fn output_lease_acquire_renew_expire_and_recover_are_generation_fenced() {
        let mut state = OutputLeaseState::fresh_process(7).unwrap();
        let owner = owner(7, 11);
        let acquired = state.acquire(owner.clone(), both(), 100, 50).unwrap();
        assert_eq!(acquired.generation, 1);
        assert_eq!(acquired.expires_at_monotonic_ms, Some(150));
        let renewed = state.renew(&owner, 1, 120, 50).unwrap();
        assert_eq!(renewed.generation, 2);
        assert!(!state.observe_expiry(169).unwrap());
        assert!(state.observe_expiry(170).unwrap());
        assert_eq!(state.snapshot.phase, OutputLeasePhase::HeldOrphaned);
        let recovered = state.recover(&owner, 3, 200, 50).unwrap();
        assert_eq!(recovered.generation, 4);
        assert_eq!(recovered.phase, OutputLeasePhase::HeldActive);
    }

    #[test]
    fn output_lease_rejects_owner_incarnation_aba_without_mutation() {
        let mut state = OutputLeaseState::fresh_process(9).unwrap();
        let current = owner(9, 1);
        state.acquire(current.clone(), both(), 0, 10).unwrap();
        let before = state.snapshot();
        assert_eq!(
            state.renew(&owner(9, 2), 1, 1, 10),
            Err(OutputLeaseError::StaleOwner)
        );
        assert_eq!(state.snapshot(), before);
        assert_eq!(
            state.renew(&owner(10, 1), 1, 1, 10),
            Err(OutputLeaseError::StaleOwner)
        );
        assert_eq!(state.snapshot(), before);
    }

    #[test]
    fn output_lease_relinquish_and_restart_do_not_reclaim_authority() {
        let mut state = OutputLeaseState::fresh_process(3).unwrap();
        let owner = owner(3, 4);
        state.acquire(owner.clone(), both(), 0, 10).unwrap();
        let released = state.relinquish_output_lease(&owner, 1).unwrap();
        assert_eq!(released.phase, OutputLeasePhase::Unclaimed);
        assert!(released.owner.is_none());
        let restarted = OutputLeaseState::fresh_process(99).unwrap();
        assert_eq!(restarted.snapshot().phase, OutputLeasePhase::Unclaimed);
        assert_eq!(restarted.snapshot().generation, 0);
    }

    #[test]
    fn output_lease_force_transfer_is_atomic_and_authority_only() {
        let mut state = OutputLeaseState::fresh_process(1).unwrap();
        let old = owner(1, 1);
        let new = owner(1, 2);
        state.acquire(old, both(), 5, 10).unwrap();
        let before = state.snapshot();
        assert_eq!(
            state.force_transfer(2, new.clone(), 6, 10),
            Err(OutputLeaseError::StaleGeneration)
        );
        assert_eq!(state.snapshot(), before);
        let transferred = state.force_transfer(1, new.clone(), 6, 10).unwrap();
        assert_eq!(transferred.owner, Some(new));
        assert_eq!(transferred.resources, before.resources);
        assert_eq!(transferred.generation, 2);
        assert_eq!(transferred.phase, OutputLeasePhase::HeldActive);
    }

    #[test]
    fn output_lease_project_orphan_and_invalid_requests_are_atomic() {
        let mut state = OutputLeaseState::fresh_process(2).unwrap();
        let owner = owner(2, 3);
        state.acquire(owner.clone(), both(), 10, 10).unwrap();
        let orphaned = state.orphan(1).unwrap();
        assert_eq!(orphaned.phase, OutputLeasePhase::HeldOrphaned);
        assert_eq!(orphaned.generation, 2);
        let before = state.snapshot();
        assert_eq!(
            state.recover(&owner, 2, u64::MAX, 1),
            Err(OutputLeaseError::ClockExhausted)
        );
        assert_eq!(state.snapshot(), before);
        assert_eq!(
            state.recover(&owner, 2, 0, MAX_OUTPUT_LEASE_TTL_MS + 1),
            Err(OutputLeaseError::InvalidTtl)
        );
        assert_eq!(state.snapshot(), before);
    }

    #[test]
    fn output_lease_restart_rejects_cached_owner_and_overflow_is_atomic() {
        let cached_owner = owner(1, 5);
        let mut restarted = OutputLeaseState::fresh_process(2).unwrap();
        let before = restarted.snapshot();
        assert_eq!(
            restarted.acquire(cached_owner, both(), 0, 10),
            Err(OutputLeaseError::StaleOwner)
        );
        assert_eq!(restarted.snapshot(), before);

        restarted.snapshot.generation = u64::MAX;
        let exhausted = restarted.snapshot();
        assert_eq!(
            restarted.acquire(owner(2, 1), both(), 0, 10),
            Err(OutputLeaseError::GenerationExhausted)
        );
        assert_eq!(restarted.snapshot(), exhausted);
    }

    #[test]
    fn output_lease_renew_at_deadline_observes_orphaning() {
        let mut state = OutputLeaseState::fresh_process(4).unwrap();
        let owner = owner(4, 1);
        state.acquire(owner.clone(), both(), 10, 5).unwrap();
        assert_eq!(
            state.renew(&owner, 1, 15, 5),
            Err(OutputLeaseError::Expired)
        );
        assert_eq!(state.snapshot().phase, OutputLeasePhase::HeldOrphaned);
        assert_eq!(state.snapshot().generation, 2);
    }

    #[test]
    fn output_lease_transfer_at_deadline_observes_expiry_before_transfer() {
        let mut state = OutputLeaseState::fresh_process(6).unwrap();
        state.acquire(owner(6, 1), both(), 20, 5).unwrap();
        assert_eq!(
            state.force_transfer(1, owner(6, 2), 25, 5),
            Err(OutputLeaseError::Expired)
        );
        let orphaned = state.snapshot();
        assert_eq!(orphaned.phase, OutputLeasePhase::HeldOrphaned);
        assert_eq!(orphaned.generation, 2);
        assert_eq!(orphaned.owner, Some(owner(6, 1)));
        let transferred = state.force_transfer(2, owner(6, 2), 25, 5).unwrap();
        assert_eq!(transferred.generation, 3);
        assert_eq!(transferred.phase, OutputLeasePhase::HeldActive);
    }

    #[test]
    fn output_lease_transfer_rejects_a_different_process_without_mutation() {
        let mut state = OutputLeaseState::fresh_process(8).unwrap();
        state.acquire(owner(8, 1), both(), 0, 10).unwrap();
        let before = state.snapshot();
        assert_eq!(
            state.force_transfer(1, owner(9, 1), 1, 5),
            Err(OutputLeaseError::StaleOwner)
        );
        assert_eq!(state.snapshot(), before);
    }

    fn lighting() -> OutputLeaseResources {
        OutputLeaseResources::new(&[OutputLeaseResource::Lighting]).unwrap()
    }

    fn video() -> OutputLeaseResources {
        OutputLeaseResources::new(&[OutputLeaseResource::Video]).unwrap()
    }

    fn registry(process: u64) -> OutputLeaseRegistry {
        OutputLeaseRegistry::fresh_process(process).unwrap()
    }

    fn observe_request(request_id: u64, lease_id: u64) -> OutputLeaseRequest {
        OutputLeaseRequest::from_action(
            "principal-a",
            "output-lease",
            request_id,
            OutputLeaseRequestAction::ObserveExpiry {
                lease_id: OutputLeaseId(lease_id),
            },
        )
        .unwrap()
    }

    #[test]
    fn output_lease_registry_authorize_ordinary_is_exact_and_deadline_fenced() {
        let mut registry = registry(19);
        let grant = registry
            .acquire(owner(19, 1), lighting(), "project-a", 0, 10)
            .unwrap();
        assert_eq!(grant.lease_id.encode(), "lease-0000000000000001");
        assert_eq!(
            OutputLeaseId::decode(&grant.lease_id.encode()),
            Ok(grant.lease_id)
        );
        assert_eq!(
            OutputLeaseId::decode("lease-000000000000000A"),
            Err(OutputLeaseError::InvalidRequest)
        );

        let request = OutputLeaseRequest::from_action(
            "local-ui",
            "output-control",
            1,
            OutputLeaseRequestAction::AuthorizeOrdinary {
                lease_id: grant.lease_id,
                owner: owner(19, 1),
                expected_generation: 1,
                exact_resources: lighting(),
            },
        )
        .unwrap();
        let receipt = registry.submit_request(&request, 1).unwrap();
        assert_eq!(receipt.outcome, Ok(OutputLeaseOperationOutcome::Authorized));
        assert_eq!(receipt.audit_sequence, 1);
        assert_eq!(receipt.changes.len(), 1);
        assert_eq!(
            receipt.changes[0].before.as_ref().unwrap().generation,
            receipt.changes[0].after.as_ref().unwrap().generation
        );
        let retry = registry.submit_request(&request, 1).unwrap();
        assert_eq!(retry, receipt);
        assert_eq!(registry.audit().len(), 1);

        let expired_request = OutputLeaseRequest::from_action(
            "local-ui",
            "output-control",
            2,
            OutputLeaseRequestAction::AuthorizeOrdinary {
                lease_id: grant.lease_id,
                owner: owner(19, 1),
                expected_generation: 1,
                exact_resources: lighting(),
            },
        )
        .unwrap();
        let expired = registry.submit_request(&expired_request, 10).unwrap();
        assert_eq!(expired.outcome, Err(OutputLeaseError::Expired));
        assert_eq!(expired.generation_before, Some(1));
        assert_eq!(expired.generation_after, Some(2));
        assert_eq!(
            registry.lease_view(grant.lease_id).unwrap().snapshot.phase,
            OutputLeasePhase::HeldOrphaned
        );
    }

    #[test]
    fn output_lease_registry_coexists_only_for_non_overlapping_canonical_resources() {
        let mut registry = registry(20);
        let first = registry
            .acquire(owner(20, 1), lighting(), "project-a", 0, 100)
            .unwrap();
        let second = registry
            .acquire(owner(20, 2), video(), "project-b", 0, 100)
            .unwrap();
        assert_ne!(first.lease_id, second.lease_id);
        assert_eq!(registry.active_lease_count(), 2);
        let before = registry.clone();
        for resources in [lighting(), video(), both()] {
            assert_eq!(
                registry.acquire(owner(20, 3), resources, "project-c", 0, 100),
                Err(OutputLeaseError::ResourceConflict)
            );
            assert_eq!(registry, before);
        }
        assert_eq!(
            registry
                .lease_view(first.lease_id)
                .unwrap()
                .snapshot
                .resources,
            Some(lighting())
        );
    }

    #[test]
    fn output_lease_registry_expiry_receipt_records_orphan_generation_without_physical_delta() {
        let mut registry = registry(21);
        let grant = registry
            .acquire(owner(21, 1), lighting(), "project-a", 0, 10)
            .unwrap();
        let request = OutputLeaseRequest::from_action(
            "local-ui",
            "output-lease",
            1,
            OutputLeaseRequestAction::Renew {
                lease_id: grant.lease_id,
                owner: owner(21, 1),
                expected_generation: 1,
                ttl_ms: 10,
            },
        )
        .unwrap();
        let receipt = registry.submit_request(&request, 10).unwrap();
        assert_eq!(receipt.outcome, Err(OutputLeaseError::Expired));
        assert_eq!(receipt.generation_before, Some(1));
        assert_eq!(receipt.generation_after, Some(2));
        assert_eq!(registry.active_lease_count(), 1);
        assert_eq!(
            registry.lease_view(grant.lease_id).unwrap().snapshot.phase,
            OutputLeasePhase::HeldOrphaned
        );
        assert_eq!(registry.process_session_incarnation(), 21);
        assert_eq!(registry.receipt_count(), 1);
        assert_eq!(receipt.shape_hash.0.len(), 32);
    }

    #[test]
    fn output_lease_registry_request_retry_conflict_and_busy_are_exact_and_atomic() {
        let mut registry = registry(22);
        let request = observe_request(7, 900);
        let first = registry.submit_request(&request, 0).unwrap();
        let audit_count = registry.audit().len();
        let retry = registry.submit_request(&request, 0).unwrap();
        assert_eq!(retry, first);
        assert_eq!(registry.audit().len(), audit_count);

        let mut changed_shape = OutputLeaseRequestShape::new("observe_expiry").unwrap();
        changed_shape.lease_id = Some(OutputLeaseId(901));
        let changed = OutputLeaseRequest::new(request.key.clone(), changed_shape).unwrap();
        let before_conflict = registry.clone();
        assert_eq!(
            registry.begin_request(&changed, 0),
            Err(OutputLeaseError::Conflict)
        );
        assert_eq!(registry, before_conflict);

        let busy_request = observe_request(8, 902);
        let admission = registry.begin_request(&busy_request, 0).unwrap();
        let OutputLeaseRequestAdmission::InFlight(permit) = admission else {
            panic!("new request must reserve an in-flight lane")
        };
        assert_eq!(
            registry.begin_request(&busy_request, 0),
            Err(OutputLeaseError::Busy)
        );
        let mut changed_in_flight_shape = OutputLeaseRequestShape::new("observe_expiry").unwrap();
        changed_in_flight_shape.lease_id = Some(OutputLeaseId(903));
        let changed_in_flight =
            OutputLeaseRequest::new(busy_request.key.clone(), changed_in_flight_shape).unwrap();
        assert_eq!(
            registry.begin_request(&changed_in_flight, 0),
            Err(OutputLeaseError::Conflict)
        );
        registry.cancel_request(permit, 0).unwrap();
        assert_eq!(registry.audit().len(), audit_count);
    }

    #[test]
    fn output_lease_registry_same_identity_cross_operation_is_conflict_before_admission() {
        let mut registry = registry(29);
        let acquire = OutputLeaseRequest::from_action(
            "principal-a",
            "output-lease",
            1,
            OutputLeaseRequestAction::Acquire {
                owner: owner(29, 1),
                resources: lighting(),
                project_identity: "project".to_string(),
                ttl_ms: 100,
            },
        )
        .unwrap();
        let acquired = registry.submit_request(&acquire, 0).unwrap();
        let transfer = OutputLeaseRequest::from_action(
            "principal-a",
            "output-lease",
            1,
            OutputLeaseRequestAction::ForceTransfer {
                lease_id: acquired.lease_id.unwrap(),
                expected_generation: 1,
                new_owner: owner(29, 2),
                ttl_ms: 100,
            },
        )
        .unwrap();
        let before = registry.clone();
        assert_eq!(
            registry.submit_request(&transfer, 0),
            Err(OutputLeaseError::Conflict)
        );
        assert_eq!(registry, before);
    }

    #[test]
    fn output_lease_registry_submit_completion_failure_is_full_registry_atomic() {
        let mut submit_registry = registry(30);
        submit_registry.next_audit_sequence = u64::MAX - 1;
        let before = submit_registry.clone();
        assert_eq!(
            submit_registry.submit_request(&observe_request(1, 3_000), 0),
            Err(OutputLeaseError::AuditCapacity)
        );
        assert_eq!(submit_registry, before);

        let mut public_registry = registry(30);
        let request = observe_request(1, 3_001);
        let OutputLeaseRequestAdmission::InFlight(permit) =
            public_registry.begin_request(&request, 0).unwrap()
        else {
            panic!("a fresh request must reserve a completion lane")
        };
        public_registry.next_audit_sequence = u64::MAX - 1;
        let before_public_failure = public_registry.clone();
        let receipt = OutputLeaseRequestReceipt {
            key: request.key.clone(),
            shape_hash: request.shape_hash.clone(),
            lease_id: None,
            affected_lease_ids: Vec::new(),
            owner: None,
            process_session_incarnation: public_registry.process_session_incarnation,
            resources: None,
            generation_before: None,
            generation_after: None,
            audit_sequence: 0,
            changes: Vec::new(),
            outcome: Err(OutputLeaseError::UnknownLease),
        };
        assert_eq!(
            public_registry.complete_request(permit, receipt, 0),
            Err(OutputLeaseError::AuditCapacity)
        );
        assert_eq!(public_registry, before_public_failure);
    }

    #[test]
    fn output_lease_registry_string_and_cardinality_bounds_fail_closed() {
        assert_eq!(
            OutputLeaseOwner::new(
                "x".repeat(MAX_OUTPUT_LEASE_OWNER_PRINCIPAL_BYTES + 1),
                "main",
                1,
                1,
            ),
            Err(OutputLeaseError::InvalidOwner)
        );
        assert_eq!(
            OutputLeaseOwner::new(
                "local",
                "x".repeat(MAX_OUTPUT_LEASE_OWNER_WINDOW_BYTES + 1),
                1,
                1,
            ),
            Err(OutputLeaseError::InvalidOwner)
        );
        assert_eq!(
            OutputLeaseRequestKey::new(
                "x".repeat(MAX_OUTPUT_LEASE_REQUEST_PRINCIPAL_BYTES + 1),
                "domain",
                1,
            ),
            Err(OutputLeaseError::InvalidRequest)
        );
        assert_eq!(
            OutputLeaseRequestShape::new("x".repeat(MAX_OUTPUT_LEASE_OPERATION_BYTES + 1)),
            Err(OutputLeaseError::InvalidRequest)
        );
        let mut shape = OutputLeaseRequestShape::new("observe_expiry").unwrap();
        shape.selected_lease_ids = (1..=(MAX_OUTPUT_LEASE_SELECTED_IDS as u64 + 1))
            .map(OutputLeaseId)
            .collect();
        assert_eq!(
            shape.canonical_hash(),
            Err(OutputLeaseError::InvalidRequest)
        );
        assert_eq!(
            OutputLeaseRequest::from_action(
                "principal",
                "domain",
                1,
                OutputLeaseRequestAction::Acquire {
                    owner: owner(1, 1),
                    resources: lighting(),
                    project_identity: "x".repeat(MAX_OUTPUT_LEASE_PROJECT_BYTES + 1),
                    ttl_ms: 10,
                },
            ),
            Err(OutputLeaseError::InvalidProject)
        );
    }

    #[test]
    fn output_lease_registry_origin_and_token_bounds_are_bounded() {
        let mut registry = registry(31);
        for origin_id in 0..MAX_OUTPUT_LEASE_REQUEST_ORIGINS {
            let request = OutputLeaseRequest::from_action(
                format!("principal-{origin_id}"),
                "output-lease",
                1,
                OutputLeaseRequestAction::ObserveExpiry {
                    lease_id: OutputLeaseId(4_000 + origin_id as u64),
                },
            )
            .unwrap();
            registry.submit_request(&request, 0).unwrap();
        }
        assert_eq!(registry.origin_count(), MAX_OUTPUT_LEASE_REQUEST_ORIGINS);
        assert_eq!(registry.token_buckets.len(), MAX_OUTPUT_LEASE_TOKEN_BUCKETS);
        let next = OutputLeaseRequest::from_action(
            "principal-overflow",
            "output-lease",
            1,
            OutputLeaseRequestAction::ObserveExpiry {
                lease_id: OutputLeaseId(9_999),
            },
        )
        .unwrap();
        assert_eq!(
            registry.submit_request(&next, 0),
            Err(OutputLeaseError::RequestCapacity)
        );
        assert_eq!(registry.origin_count(), MAX_OUTPUT_LEASE_REQUEST_ORIGINS);
    }

    #[test]
    fn output_lease_registry_ten_thousand_sequential_requests_keep_truth_bounded() {
        let mut registry = registry(32);
        for request_id in 1..=10_000_u64 {
            let request = observe_request(request_id, 5_000 + request_id);
            registry
                .submit_request(&request, (request_id - 1) * 250)
                .unwrap();
        }
        assert_eq!(registry.lease_count(), 0);
        assert_eq!(registry.active_lease_count(), 0);
        assert_eq!(registry.origin_count(), 1);
        assert_eq!(registry.token_buckets.len(), 1);
        assert!(registry.receipt_count() <= MAX_OUTPUT_LEASE_REQUESTS);
        assert!(registry.audit().len() <= MAX_OUTPUT_LEASE_AUDIT_RECORDS);
        assert!(registry.in_flight.is_empty());
        assert!(registry.operation_lanes.is_empty());
        let before = registry.clone();
        assert_eq!(
            registry.submit_request(&observe_request(1, 5_001), 2_500_000),
            Err(OutputLeaseError::ReceiptNotRetained)
        );
        assert_eq!(registry, before);
    }

    #[test]
    fn output_lease_registry_inflight_old_owner_is_fenced_after_transfer() {
        let mut registry = registry(33);
        let old_owner = owner(33, 1);
        let grant = registry
            .acquire(old_owner.clone(), lighting(), "project", 0, 100)
            .unwrap();
        let request = OutputLeaseRequest::from_action(
            "principal-old",
            "output-lease",
            1,
            OutputLeaseRequestAction::Renew {
                lease_id: grant.lease_id,
                owner: old_owner.clone(),
                expected_generation: 1,
                ttl_ms: 100,
            },
        )
        .unwrap();
        let admission = registry.begin_request(&request, 0).unwrap();
        let OutputLeaseRequestAdmission::InFlight(permit) = admission else {
            panic!("request must be in flight")
        };
        registry
            .force_transfer(grant.lease_id, 1, owner(33, 2), 0, 100)
            .unwrap();
        let before = registry.clone();
        assert_eq!(
            registry.renew(grant.lease_id, &old_owner, 1, 0, 100),
            Err(OutputLeaseError::StaleGeneration)
        );
        assert_eq!(registry, before);
        registry.cancel_request(permit, 0).unwrap();
    }

    #[test]
    fn output_lease_registry_multi_lease_receipt_restores_actual_change_truth() {
        let mut registry = registry(34);
        let first = registry
            .acquire(owner(34, 1), lighting(), "project", 0, 100)
            .unwrap();
        let second = registry
            .acquire(owner(34, 2), video(), "project", 0, 100)
            .unwrap();
        let request = OutputLeaseRequest::from_action(
            "principal-project",
            "output-lease",
            1,
            OutputLeaseRequestAction::ProjectOrphan {
                project_identity: "project".to_string(),
                selected_lease_ids: vec![second.lease_id, first.lease_id],
                resource_scope: None,
            },
        )
        .unwrap();
        let receipt = registry.submit_request(&request, 0).unwrap();
        assert_eq!(receipt.changes.len(), 2);
        for change in &receipt.changes {
            assert!(change.before.is_some());
            assert_eq!(
                change.before.as_ref().unwrap().phase,
                OutputLeasePhase::HeldActive
            );
            assert_eq!(
                change.after.as_ref().unwrap().phase,
                OutputLeasePhase::HeldOrphaned
            );
            assert_eq!(
                change.after.as_ref().unwrap().generation,
                change.before.as_ref().unwrap().generation + 1
            );
            assert!(change.after.as_ref().unwrap().resources.is_some());
        }
        assert_eq!(registry.audit().back().unwrap().receipt, receipt);
    }

    #[test]
    fn output_lease_registry_request_lifecycle_covers_recover_relinquish_and_retirement() {
        let mut registry = registry(35);
        let first = OutputLeaseRequest::from_action(
            "principal-lifecycle",
            "output-lease",
            1,
            OutputLeaseRequestAction::Acquire {
                owner: owner(35, 1),
                resources: lighting(),
                project_identity: "project".to_string(),
                ttl_ms: 10,
            },
        )
        .unwrap();
        let first_receipt = registry.submit_request(&first, 0).unwrap();
        let lease_id = first_receipt.lease_id.unwrap();
        registry
            .submit_request(
                &OutputLeaseRequest::from_action(
                    "principal-lifecycle",
                    "output-lease",
                    2,
                    OutputLeaseRequestAction::Renew {
                        lease_id,
                        owner: owner(35, 1),
                        expected_generation: 1,
                        ttl_ms: 10,
                    },
                )
                .unwrap(),
                5,
            )
            .unwrap();
        registry
            .submit_request(
                &OutputLeaseRequest::from_action(
                    "principal-lifecycle",
                    "output-lease",
                    3,
                    OutputLeaseRequestAction::ObserveExpiry { lease_id },
                )
                .unwrap(),
                15,
            )
            .unwrap();
        registry
            .submit_request(
                &OutputLeaseRequest::from_action(
                    "principal-lifecycle",
                    "output-lease",
                    4,
                    OutputLeaseRequestAction::Recover {
                        lease_id,
                        owner: owner(35, 1),
                        expected_generation: 3,
                        ttl_ms: 10,
                    },
                )
                .unwrap(),
                16,
            )
            .unwrap();
        registry
            .submit_request(
                &OutputLeaseRequest::from_action(
                    "principal-lifecycle",
                    "output-lease",
                    5,
                    OutputLeaseRequestAction::Relinquish {
                        lease_id,
                        owner: owner(35, 1),
                        expected_generation: 4,
                    },
                )
                .unwrap(),
                16,
            )
            .unwrap();
        let second = OutputLeaseRequest::from_action(
            "principal-lifecycle",
            "output-lease",
            6,
            OutputLeaseRequestAction::Acquire {
                owner: owner(35, 2),
                resources: video(),
                project_identity: "project-two".to_string(),
                ttl_ms: 100,
            },
        )
        .unwrap();
        let second_id = registry
            .submit_request(&second, 16)
            .unwrap()
            .lease_id
            .unwrap();
        let retired = registry
            .submit_request(
                &OutputLeaseRequest::from_action(
                    "principal-lifecycle",
                    "output-lease",
                    7,
                    OutputLeaseRequestAction::RetireOwner {
                        owner: owner(35, 2),
                    },
                )
                .unwrap(),
                16,
            )
            .unwrap();
        assert_eq!(retired.affected_lease_ids, vec![second_id]);
        assert_eq!(retired.changes.len(), 1);
        assert_eq!(
            registry.lease_view(second_id).unwrap().snapshot.phase,
            OutputLeasePhase::HeldOrphaned
        );
    }

    #[test]
    fn output_lease_registry_rate_bucket_is_burst_eight_and_clock_rollback_fails_closed() {
        let mut registry = registry(23);
        for request_id in 1..=8 {
            registry
                .submit_request(&observe_request(request_id, 1_000 + request_id), 0)
                .unwrap();
        }
        let before = registry.clone();
        assert_eq!(
            registry.submit_request(&observe_request(9, 1_009), 0),
            Err(OutputLeaseError::RateLimited)
        );
        assert_eq!(registry, before);
        assert_eq!(
            registry.submit_request(&observe_request(10, 1_010), 0),
            Err(OutputLeaseError::RateLimited)
        );
        let refilled = registry
            .submit_request(&observe_request(11, 1_011), 250)
            .unwrap();
        assert_eq!(refilled.outcome, Err(OutputLeaseError::UnknownLease));
        assert_eq!(
            registry.submit_request(&observe_request(12, 1_012), 200),
            Err(OutputLeaseError::ClockRollback)
        );
    }

    #[test]
    fn output_lease_registry_lifecycle_recover_relinquish_and_restart_are_process_bound() {
        let mut registry = registry(26);
        let current = owner(26, 1);
        let grant = registry
            .acquire(current.clone(), lighting(), "project", 0, 10)
            .unwrap();
        assert!(registry.observe_expiry(grant.lease_id, 10).unwrap());
        assert_eq!(
            registry
                .recover(grant.lease_id, &current, 2, 11, 10)
                .unwrap()
                .generation,
            3
        );
        assert_eq!(
            registry
                .relinquish_output_lease(grant.lease_id, &current, 3, 11)
                .unwrap()
                .phase,
            OutputLeasePhase::Unclaimed
        );
        let restarted = OutputLeaseRegistry::fresh_process(27).unwrap();
        assert_eq!(restarted.lease_count(), 0);
        assert_eq!(restarted.active_lease_count(), 0);
    }

    #[test]
    fn output_lease_registry_receipt_purge_keeps_high_water_and_bounded_history() {
        let mut registry = registry(28);
        let first = observe_request(1, 2_000);
        let first_receipt = registry.submit_request(&first, 0).unwrap();
        assert_eq!(registry.receipt_count(), 1);
        let purged = registry.purge(60_000).unwrap();
        assert_eq!(purged.receipts_purged, 1);
        assert_eq!(registry.receipt_count(), 0);
        assert_eq!(registry.origin_count(), 1);
        assert_eq!(
            registry.submit_request(&first, 60_000),
            Err(OutputLeaseError::ReceiptNotRetained)
        );
        assert_eq!(registry.origin_count(), 1);
        assert_eq!(first_receipt.key, first.key);

        for request_id in 2..=MAX_OUTPUT_LEASE_REQUESTS as u64 + 1 {
            let request = observe_request(request_id, 2_000 + request_id);
            let _ = registry.submit_request(&request, 60_000);
        }
        assert!(registry.receipt_count() <= MAX_OUTPUT_LEASE_REQUESTS);
        assert!(registry.origin_count() <= MAX_OUTPUT_LEASE_REQUEST_ORIGINS);
    }

    #[test]
    fn output_lease_registry_owner_project_orphan_and_transfer_are_generation_exact() {
        let mut registry = registry(24);
        let owner_a = owner(24, 1);
        let owner_b = owner(24, 2);
        let lighting_lease = registry
            .acquire(owner_a.clone(), lighting(), "project-a", 0, 100)
            .unwrap();
        let video_lease = registry
            .acquire(owner_b.clone(), video(), "project-b", 0, 100)
            .unwrap();
        assert_eq!(
            registry.project_orphan(
                "project-a",
                &[lighting_lease.lease_id, video_lease.lease_id],
                Some(&lighting()),
                0,
            ),
            Ok(vec![lighting_lease.lease_id])
        );
        assert_eq!(
            registry
                .lease_view(lighting_lease.lease_id)
                .unwrap()
                .snapshot
                .phase,
            OutputLeasePhase::HeldOrphaned
        );
        assert_eq!(
            registry
                .lease_view(video_lease.lease_id)
                .unwrap()
                .snapshot
                .phase,
            OutputLeasePhase::HeldActive
        );
        assert_eq!(
            registry.retire_owner(&owner_b, 0),
            Ok(vec![video_lease.lease_id])
        );
        let before = registry.clone();
        assert_eq!(
            registry.force_transfer(lighting_lease.lease_id, 1, owner(24, 3), 0, 100,),
            Err(OutputLeaseError::StaleGeneration)
        );
        assert_eq!(registry, before);
    }

    #[test]
    fn output_lease_registry_capacity_purge_and_id_overflow_preserve_live_authority() {
        let mut registry = registry(25);
        for incarnation in 1..=MAX_OUTPUT_LEASES as u64 {
            let grant = registry
                .acquire(owner(25, incarnation), lighting(), "project", 0, 100)
                .unwrap();
            registry
                .relinquish_output_lease(grant.lease_id, &owner(25, incarnation), 1, 0)
                .unwrap();
        }
        let live = registry
            .acquire(owner(25, 99), video(), "live", 0, 100)
            .unwrap_err();
        assert_eq!(live, OutputLeaseError::LeaseCapacity);
        let purged = registry.purge(60_000).unwrap();
        assert_eq!(purged.unclaimed_leases_purged, MAX_OUTPUT_LEASES);
        let grant = registry
            .acquire(owner(25, 99), video(), "live", 60_000, 100)
            .unwrap();
        let before = registry.clone();
        registry.next_lease_id = u64::MAX;
        assert_eq!(
            registry.acquire(owner(25, 100), lighting(), "overflow", 60_000, 100),
            Err(OutputLeaseError::LeaseIdExhausted)
        );
        assert_eq!(
            registry.leases.get(&grant.lease_id),
            before.leases.get(&grant.lease_id)
        );
        assert_eq!(registry.resource_index, before.resource_index);
    }
}
