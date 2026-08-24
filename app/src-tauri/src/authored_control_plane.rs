//! First local, server-authoritative authored control-plane mutation vertical.
//!
//! The public Tauri wrapper remains in `main.rs` so source inventory sees the
//! existing command name. This module owns only the strict receipt/admission
//! lane; it never accepts a caller-supplied owner, window label or principal.

use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use protocol::control_plane_command::{
    AuthoredCommandErrorCodeV1, AuthoredCommandErrorV1, AuthoredCommandRejectionV1,
    AuthoredRequestV1, ProjectMutationFenceV1, SetEffectEnabledAppliedV1,
    SetEffectEnabledOutcomeV1, SetEffectEnabledPayload, SetEffectEnabledResponseV1,
    SetEffectEnabledTerminalReceiptV1, SET_EFFECT_ENABLED_OPERATION_ID,
    SET_EFFECT_ENABLED_SHAPE_DOMAIN_V1,
};
use sha2::{Digest, Sha256};
use tauri::WebviewWindow;

use super::{
    checked_project_authority_publication_generation_after_change, current_unix_ms,
    ensure_no_pending_project_transaction, ensure_project_operator_authoritative_mutation_allowed,
    lock_project_coordinator, lock_project_external_command_admission,
    prepare_internal_media_asset_commit, project_checkpoint_hash, project_file_for_save_from_parts,
    reconcile_project_checkpoint_for_coordinator, run_admitted_internal_media_asset_transaction,
    AppState, ControlPlaneQueryState, ProjectCheckpoint, ProjectCoordinator,
};

// A retained terminal fact deliberately outlives the short-lived issued
// mutation fence. A reply-loss retry therefore recovers the exact receipt
// even after its admission capability has expired; only a new mutation needs
// a freshly issued fence.
const TERMINAL_RECEIPT_TTL: Duration = Duration::from_secs(15 * 60);

const RETIRED_KEY_TOMBSTONE_TTL: Duration = Duration::from_secs(10 * 60);
const MAX_TERMINAL_RECEIPTS: usize = 256;
const MAX_PUBLICATION_LANES: usize = 256;
const MAX_RETIRED_KEY_TOMBSTONES: usize = 512;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct SetEffectEnabledReceiptKey {
    process_incarnation: u64,
    session_incarnation: u64,
    principal: String,
    window_label: String,
    owner_incarnation: u64,
    operation_id: String,
    request_id: u64,
    start_epoch: u64,
    start_revision: u64,
    start_checkpoint_hash: String,
    start_publication_generation: u64,
}

#[derive(Debug, Clone)]
struct AuthoredCallerBinding {
    principal: String,
    window_label: String,
    owner_incarnation: u64,
}

#[derive(Debug, Clone)]
struct TerminalReceiptRecord {
    shape_sha256: String,
    receipt: SetEffectEnabledTerminalReceiptV1,
    expires_at: Instant,
    last_used: u64,
}

#[derive(Debug, Clone)]
struct RetiredKeyTombstone {
    expires_at: Instant,
    last_used: u64,
}

#[derive(Debug)]
struct AuthoredControlPlaneInner {
    receipts: HashMap<SetEffectEnabledReceiptKey, TerminalReceiptRecord>,
    lanes: HashMap<SetEffectEnabledReceiptKey, Arc<Mutex<()>>>,
    tombstones: HashMap<SetEffectEnabledReceiptKey, RetiredKeyTombstone>,
    sequence: u64,
}

/// Bounded state for the `syndocal.effects.set_enabled.v1` terminal lane.
/// A receipt key binds OS/session continuity and the backend-derived renderer
/// binding, so a raw IPC payload can never move a receipt across principals or
/// an owner ABA rotation.
pub(crate) struct AuthoredControlPlaneState {
    inner: Mutex<AuthoredControlPlaneInner>,
}

impl Default for AuthoredControlPlaneState {
    fn default() -> Self {
        Self {
            inner: Mutex::new(AuthoredControlPlaneInner {
                receipts: HashMap::new(),
                lanes: HashMap::new(),
                tombstones: HashMap::new(),
                sequence: 0,
            }),
        }
    }
}

enum ReceiptReservation {
    Terminal(Box<SetEffectEnabledTerminalReceiptV1>),
    Rejected(AuthoredCommandErrorCodeV1),
    Lane(Arc<Mutex<()>>),
}

enum RetainedReceiptLookup {
    Terminal(Box<SetEffectEnabledTerminalReceiptV1>),
    Rejected(AuthoredCommandErrorCodeV1),
    Missing,
}

impl AuthoredControlPlaneState {
    fn reserve_at(
        &self,
        key: &SetEffectEnabledReceiptKey,
        shape_sha256: &str,
        now: Instant,
    ) -> ReceiptReservation {
        let mut inner = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        purge_expired(&mut inner, now);
        let last_used = next_sequence(&mut inner);
        if let Some(record) = inner.receipts.get_mut(key) {
            record.last_used = last_used;
            return if record.shape_sha256 == shape_sha256 {
                ReceiptReservation::Terminal(Box::new(record.receipt.clone()))
            } else {
                ReceiptReservation::Rejected(AuthoredCommandErrorCodeV1::Conflict)
            };
        }
        if let Some(tombstone) = inner.tombstones.get_mut(key) {
            tombstone.last_used = last_used;
            return ReceiptReservation::Rejected(AuthoredCommandErrorCodeV1::Conflict);
        }
        if let Some(lane) = inner.lanes.get(key) {
            return ReceiptReservation::Lane(Arc::clone(lane));
        }
        if inner.lanes.len() >= MAX_PUBLICATION_LANES {
            return ReceiptReservation::Rejected(AuthoredCommandErrorCodeV1::Overloaded);
        }
        let lane = Arc::new(Mutex::new(()));
        inner.lanes.insert(key.clone(), Arc::clone(&lane));
        ReceiptReservation::Lane(lane)
    }

    /// Checks only retained terminal/tombstone state. This intentionally runs
    /// after the OS-derived caller binding but before the issued-fence TTL
    /// check: an exact reply-loss retry is a read of an already completed
    /// command, not a request to admit a new mutation.
    fn lookup_retained_at(
        &self,
        key: &SetEffectEnabledReceiptKey,
        shape_sha256: &str,
        now: Instant,
    ) -> RetainedReceiptLookup {
        let mut inner = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        purge_expired(&mut inner, now);
        let last_used = next_sequence(&mut inner);
        if let Some(record) = inner.receipts.get_mut(key) {
            record.last_used = last_used;
            return if record.shape_sha256 == shape_sha256 {
                RetainedReceiptLookup::Terminal(Box::new(record.receipt.clone()))
            } else {
                RetainedReceiptLookup::Rejected(AuthoredCommandErrorCodeV1::Conflict)
            };
        }
        if let Some(tombstone) = inner.tombstones.get_mut(key) {
            tombstone.last_used = last_used;
            return RetainedReceiptLookup::Rejected(AuthoredCommandErrorCodeV1::Conflict);
        }
        RetainedReceiptLookup::Missing
    }

    #[cfg(test)]
    fn terminal_single_flight(
        &self,
        key: SetEffectEnabledReceiptKey,
        request: &AuthoredRequestV1<SetEffectEnabledPayload>,
        shape_sha256: String,
        publish: impl FnOnce() -> SetEffectEnabledTerminalReceiptV1,
    ) -> SetEffectEnabledResponseV1 {
        self.terminal_single_flight_at(key, request, shape_sha256, Instant::now(), publish)
    }

    fn terminal_single_flight_at(
        &self,
        key: SetEffectEnabledReceiptKey,
        request: &AuthoredRequestV1<SetEffectEnabledPayload>,
        shape_sha256: String,
        now: Instant,
        publish: impl FnOnce() -> SetEffectEnabledTerminalReceiptV1,
    ) -> SetEffectEnabledResponseV1 {
        let lane = match self.reserve_at(&key, &shape_sha256, now) {
            ReceiptReservation::Terminal(receipt) => {
                return SetEffectEnabledResponseV1::TerminalReceipt(*receipt)
            }
            ReceiptReservation::Rejected(code) => return rejection(request, code),
            ReceiptReservation::Lane(lane) => lane,
        };
        let _lane_guard = lane.lock().unwrap_or_else(|poisoned| poisoned.into_inner());

        // Recheck after taking the exact key lane. A duplicate that waited for
        // the first publisher receives the immutable receipt, and a different
        // shape never reaches the engine.
        {
            let mut inner = self
                .inner
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            purge_expired(&mut inner, now);
            let last_used = next_sequence(&mut inner);
            if let Some(record) = inner.receipts.get_mut(&key) {
                record.last_used = last_used;
                return if record.shape_sha256 == shape_sha256 {
                    SetEffectEnabledResponseV1::TerminalReceipt(record.receipt.clone())
                } else {
                    rejection(request, AuthoredCommandErrorCodeV1::Conflict)
                };
            }
            if let Some(tombstone) = inner.tombstones.get_mut(&key) {
                tombstone.last_used = last_used;
                return rejection(request, AuthoredCommandErrorCodeV1::Conflict);
            }
        }

        let receipt = publish();
        // Storing the typed terminal fact is intentionally before return. A
        // reply lost after an engine ACK can therefore only recover this exact
        // receipt; it cannot publish B a second time.
        let mut inner = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        purge_expired(&mut inner, now);
        let last_used = next_sequence(&mut inner);
        // `reserve` retains one lane per live receipt and rejects the next
        // publisher at `MAX_PUBLICATION_LANES`. Because both caps are equal,
        // a lane which reached this point proves there is room for its
        // receipt. Do not manufacture a post-ACK overload result here: the
        // one definitive terminal fact must be retained before replying.
        debug_assert!(inner.receipts.len() < MAX_TERMINAL_RECEIPTS);
        inner.receipts.insert(
            key,
            TerminalReceiptRecord {
                shape_sha256,
                receipt: receipt.clone(),
                expires_at: now + TERMINAL_RECEIPT_TTL,
                last_used,
            },
        );
        SetEffectEnabledResponseV1::TerminalReceipt(receipt)
    }

    /// Retire every receipt and lane derived from a renderer principal. This
    /// is called during owner replacement/destruction before the same string
    /// can be registered again; tombstones make an old owner incarnation fail
    /// closed even if it races a delayed IPC retry.
    pub(crate) fn retire_principal(&self, principal: &str) {
        let mut inner = self
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        purge_expired(&mut inner, Instant::now());
        let keys = inner
            .receipts
            .keys()
            .chain(inner.lanes.keys())
            .filter(|key| key.principal == principal)
            .cloned()
            .collect::<HashSet<_>>();
        for key in keys {
            inner.receipts.remove(&key);
            inner.lanes.remove(&key);
            insert_tombstone(&mut inner, key, Instant::now());
        }
    }
}

fn next_sequence(inner: &mut AuthoredControlPlaneInner) -> u64 {
    inner.sequence = inner.sequence.wrapping_add(1);
    inner.sequence
}

fn purge_expired(inner: &mut AuthoredControlPlaneInner, now: Instant) {
    let expired = inner
        .receipts
        .iter()
        .filter_map(|(key, record)| (record.expires_at <= now).then_some(key.clone()))
        .collect::<Vec<_>>();
    for key in expired {
        inner.receipts.remove(&key);
        inner.lanes.remove(&key);
        insert_tombstone(inner, key, now);
    }
    inner
        .tombstones
        .retain(|_, tombstone| tombstone.expires_at > now);
}

fn insert_tombstone(
    inner: &mut AuthoredControlPlaneInner,
    key: SetEffectEnabledReceiptKey,
    now: Instant,
) {
    while inner.tombstones.len() >= MAX_RETIRED_KEY_TOMBSTONES
        && !inner.tombstones.contains_key(&key)
    {
        let oldest = inner
            .tombstones
            .iter()
            .min_by_key(|(_, tombstone)| tombstone.last_used)
            .map(|(oldest, _)| oldest.clone());
        if let Some(oldest) = oldest {
            inner.tombstones.remove(&oldest);
        } else {
            break;
        }
    }
    let last_used = next_sequence(inner);
    inner.tombstones.insert(
        key,
        RetiredKeyTombstone {
            expires_at: now + RETIRED_KEY_TOMBSTONE_TTL,
            last_used,
        },
    );
}

pub(crate) fn set_effect_enabled_authoritative(
    window: &WebviewWindow,
    state: &AppState,
    query_state: &ControlPlaneQueryState,
    request: AuthoredRequestV1<SetEffectEnabledPayload>,
) -> SetEffectEnabledResponseV1 {
    set_effect_enabled_authoritative_for_window_label(state, query_state, window.label(), request)
}

/// The production command body after Tauri injected the concrete window. It
/// is deliberately label-based only at this internal seam so unit tests can
/// exercise the exact query/session, owner-rotation, admission and real
/// `EngineHandle` path without fabricating a WebView object.
fn set_effect_enabled_authoritative_for_window_label(
    state: &AppState,
    query_state: &ControlPlaneQueryState,
    window_label: &str,
    request: AuthoredRequestV1<SetEffectEnabledPayload>,
) -> SetEffectEnabledResponseV1 {
    set_effect_enabled_authoritative_for_window_label_at(
        state,
        query_state,
        window_label,
        request,
        Instant::now(),
    )
}

/// Same production path with an injected clock used only by deterministic
/// receipt/fence-retention tests. It preserves the required lock order:
/// owner rotation, then (inside execution) external admission, then project
/// coordinator.
fn set_effect_enabled_authoritative_for_window_label_at(
    state: &AppState,
    query_state: &ControlPlaneQueryState,
    window_label: &str,
    request: AuthoredRequestV1<SetEffectEnabledPayload>,
    now: Instant,
) -> SetEffectEnabledResponseV1 {
    if request.validate().is_err() {
        return rejection(&request, AuthoredCommandErrorCodeV1::InvalidRequest);
    }
    let shape_sha256 = match shape_sha256(&request) {
        Ok(shape) => shape,
        Err(()) => return rejection(&request, AuthoredCommandErrorCodeV1::InvalidRequest),
    };

    let _owner_rotation = match state.project_transaction_owner_rotation.lock() {
        Ok(guard) => guard,
        Err(_) => return rejection(&request, AuthoredCommandErrorCodeV1::Internal),
    };
    let binding = match capture_caller_binding(state, window_label) {
        Ok(binding) => binding,
        Err(()) => return rejection(&request, AuthoredCommandErrorCodeV1::Forbidden),
    };
    let key = receipt_key(&request, &binding);
    // The key contains the server-derived principal/window/owner incarnation.
    // Never expose a retained receipt to an unregistered or ABA-replaced
    // window, but once that binding is established an exact retry must be
    // resolved before the issued-fence table is expired or purged.
    match state
        .authored_control_plane
        .lookup_retained_at(&key, &shape_sha256, now)
    {
        RetainedReceiptLookup::Terminal(receipt) => {
            return SetEffectEnabledResponseV1::TerminalReceipt(*receipt)
        }
        RetainedReceiptLookup::Rejected(code) => return rejection(&request, code),
        RetainedReceiptLookup::Missing => {}
    }
    // Validate the server-only query/session issue record after the owner
    // rotation lock has captured the registered incarnation. This takes no
    // coordinator lock, so the authoritative path remains rotation ->
    // external admission -> coordinator while an ABA owner swap cannot reuse
    // an old renderer's otherwise-identical E/R/H fence.
    if query_state
        .validate_project_mutation_fence_window_at(
            window_label,
            &request.expected_fence,
            binding.owner_incarnation,
            now,
        )
        .is_err()
    {
        return rejection(&request, AuthoredCommandErrorCodeV1::Forbidden);
    }
    state.authored_control_plane.terminal_single_flight_at(
        key,
        &request,
        shape_sha256.clone(),
        now,
        || execute_set_effect_enabled(state, &binding, &request, &shape_sha256),
    )
}

fn capture_caller_binding(
    state: &AppState,
    window_label: &str,
) -> Result<AuthoredCallerBinding, ()> {
    // A valid owner/incarnation and issued fence are insufficient while a
    // Destroyed callback is between owner retirement and query retirement.
    // The process-local authority fence is checked before any binding is
    // captured, so delayed raw authored IPC fails closed in that interval.
    state
        .ensure_window_authority_not_blocked(window_label)
        .map_err(|_| ())?;
    let owners = state.project_transaction_owners.lock().map_err(|_| ())?;
    let principal = owners.get(window_label).cloned().ok_or(())?;
    let incarnations = state
        .project_transaction_owner_incarnations
        .lock()
        .map_err(|_| ())?;
    let owner_incarnation = incarnations.get(window_label).copied().ok_or(())?;
    Ok(AuthoredCallerBinding {
        principal,
        window_label: window_label.to_string(),
        owner_incarnation,
    })
}

fn receipt_key(
    request: &AuthoredRequestV1<SetEffectEnabledPayload>,
    binding: &AuthoredCallerBinding,
) -> SetEffectEnabledReceiptKey {
    let start = &request.expected_fence;
    SetEffectEnabledReceiptKey {
        process_incarnation: start.process_incarnation,
        session_incarnation: start.session_incarnation,
        principal: binding.principal.clone(),
        window_label: binding.window_label.clone(),
        owner_incarnation: binding.owner_incarnation,
        operation_id: request.operation_id.clone(),
        request_id: request.request_id,
        start_epoch: start.project_epoch,
        start_revision: start.project_revision,
        start_checkpoint_hash: start.project_checkpoint_hash.clone(),
        start_publication_generation: start.project_publication_generation,
    }
}

fn shape_sha256(request: &AuthoredRequestV1<SetEffectEnabledPayload>) -> Result<String, ()> {
    let typed_bytes = request.canonical_shape_bytes().map_err(|_| ())?;
    let mut hasher = Sha256::new();
    hasher.update(SET_EFFECT_ENABLED_SHAPE_DOMAIN_V1);
    hasher.update((typed_bytes.len() as u64).to_be_bytes());
    hasher.update(typed_bytes);
    Ok(format!("{:x}", hasher.finalize()))
}

fn rejection(
    request: &AuthoredRequestV1<SetEffectEnabledPayload>,
    code: AuthoredCommandErrorCodeV1,
) -> SetEffectEnabledResponseV1 {
    SetEffectEnabledResponseV1::Rejected(AuthoredCommandRejectionV1 {
        operation_id: SET_EFFECT_ENABLED_OPERATION_ID.to_string(),
        request_id: request.request_id,
        start_fence: request.expected_fence.clone(),
        error: AuthoredCommandErrorV1::new(code),
    })
}

fn terminal_failure(
    request: &AuthoredRequestV1<SetEffectEnabledPayload>,
    shape_sha256: &str,
    code: AuthoredCommandErrorCodeV1,
) -> SetEffectEnabledTerminalReceiptV1 {
    SetEffectEnabledTerminalReceiptV1 {
        operation_id: SET_EFFECT_ENABLED_OPERATION_ID.to_string(),
        request_id: request.request_id,
        start_fence: request.expected_fence.clone(),
        shape_sha256: shape_sha256.to_string(),
        outcome: SetEffectEnabledOutcomeV1::Failed(AuthoredCommandErrorV1::new(code)),
    }
}

fn terminal_applied(
    request: &AuthoredRequestV1<SetEffectEnabledPayload>,
    shape_sha256: &str,
    post_fence: ProjectMutationFenceV1,
    no_op: bool,
) -> SetEffectEnabledTerminalReceiptV1 {
    let applied = SetEffectEnabledAppliedV1 {
        effect_id: request.payload.effect_id,
        enabled: request.payload.enabled,
        post_fence,
    };
    SetEffectEnabledTerminalReceiptV1 {
        operation_id: SET_EFFECT_ENABLED_OPERATION_ID.to_string(),
        request_id: request.request_id,
        start_fence: request.expected_fence.clone(),
        shape_sha256: shape_sha256.to_string(),
        outcome: if no_op {
            SetEffectEnabledOutcomeV1::NoOp(applied)
        } else {
            SetEffectEnabledOutcomeV1::Applied(applied)
        },
    }
}

fn exact_fence_matches(
    coordinator: &ProjectCoordinator,
    expected: &ProjectMutationFenceV1,
) -> bool {
    coordinator.epoch == expected.project_epoch
        && coordinator.revision == expected.project_revision
        && coordinator.checkpoint_hash == expected.project_checkpoint_hash
        && coordinator.publication_generation == expected.project_publication_generation
}

fn fence_from_coordinator(
    coordinator: &ProjectCoordinator,
    start: &ProjectMutationFenceV1,
) -> ProjectMutationFenceV1 {
    ProjectMutationFenceV1 {
        process_incarnation: start.process_incarnation,
        session_incarnation: start.session_incarnation,
        project_epoch: coordinator.epoch,
        project_revision: coordinator.revision,
        project_checkpoint_hash: coordinator.checkpoint_hash.clone(),
        project_publication_generation: coordinator.publication_generation,
    }
}

/// The only body allowed to reach `set_effect_enabled_published`. Every
/// fallible coordinator/history/hash operation completes before the engine
/// publish; after ACK `run_admitted_internal_media_asset_transaction` makes
/// the coordinator update assignment-only. Therefore an ACK failure returns a
/// terminal typed failure with A/history/coordinator byte-identical.
fn execute_set_effect_enabled(
    state: &AppState,
    binding: &AuthoredCallerBinding,
    request: &AuthoredRequestV1<SetEffectEnabledPayload>,
    shape_sha256: &str,
) -> SetEffectEnabledTerminalReceiptV1 {
    execute_set_effect_enabled_with_publish(state, binding, request, shape_sha256, || {
        #[cfg(test)]
        state
            .authored_effect_enabled_publish_attempts
            .fetch_add(1, std::sync::atomic::Ordering::AcqRel);
        state
            .engine
            .set_effect_enabled_published(request.payload.effect_id, request.payload.enabled)
    })
}

fn execute_set_effect_enabled_with_publish(
    state: &AppState,
    binding: &AuthoredCallerBinding,
    request: &AuthoredRequestV1<SetEffectEnabledPayload>,
    shape_sha256: &str,
    publish: impl FnOnce() -> Result<(), String>,
) -> SetEffectEnabledTerminalReceiptV1 {
    let _external_admission = match lock_project_external_command_admission(state) {
        Ok(guard) => guard,
        Err(_) => {
            return terminal_failure(request, shape_sha256, AuthoredCommandErrorCodeV1::Internal)
        }
    };
    let mut coordinator = match lock_project_coordinator(state) {
        Ok(coordinator) => coordinator,
        Err(_) => {
            return terminal_failure(request, shape_sha256, AuthoredCommandErrorCodeV1::Internal)
        }
    };

    if reconcile_project_checkpoint_for_coordinator(state, &mut coordinator).is_err()
        || !exact_fence_matches(&coordinator, &request.expected_fence)
    {
        return terminal_failure(
            request,
            shape_sha256,
            AuthoredCommandErrorCodeV1::StaleFence,
        );
    }
    if ensure_no_pending_project_transaction(&coordinator).is_err()
        || state
            .project_transaction_active
            .load(std::sync::atomic::Ordering::Acquire)
    {
        return terminal_failure(request, shape_sha256, AuthoredCommandErrorCodeV1::Busy);
    }
    if ensure_project_operator_authoritative_mutation_allowed(
        state,
        &coordinator,
        &binding.principal,
    )
    .is_err()
    {
        return terminal_failure(request, shape_sha256, AuthoredCommandErrorCodeV1::Forbidden);
    }

    // Capture A after every authoritative fence preflight and while external
    // admission is held. No input/remote callback can change the persistence
    // baseline until B either ACKs or fails.
    let before_snapshot = match state.engine.persistence_snapshot() {
        Ok(snapshot) => snapshot,
        Err(_) => {
            return terminal_failure(request, shape_sha256, AuthoredCommandErrorCodeV1::Internal)
        }
    };
    let current_enabled = match before_snapshot
        .effects
        .iter()
        .find(|effect| effect.id == request.payload.effect_id)
    {
        Some(effect) => effect.enabled,
        None => {
            return terminal_failure(request, shape_sha256, AuthoredCommandErrorCodeV1::NotFound)
        }
    };
    if current_enabled == request.payload.enabled {
        // No engine send, publication generation, revision, history entry or
        // Undo/Redo mutation. The DTO validator proves post_fence == start.
        return terminal_applied(
            request,
            shape_sha256,
            fence_from_coordinator(&coordinator, &request.expected_fence),
            true,
        );
    }

    let before_project =
        project_file_for_save_from_parts(before_snapshot.clone(), &coordinator.ancillary);
    let before_hash = match project_checkpoint_hash(&before_project, &coordinator.mappings) {
        Ok(hash) => hash,
        Err(_) => {
            return terminal_failure(request, shape_sha256, AuthoredCommandErrorCodeV1::Internal)
        }
    };
    if before_hash != coordinator.checkpoint_hash {
        return terminal_failure(
            request,
            shape_sha256,
            AuthoredCommandErrorCodeV1::StaleFence,
        );
    }
    let before = ProjectCheckpoint {
        project: before_project,
        mappings: coordinator.mappings.clone(),
        epoch: coordinator.epoch,
        revision: coordinator.revision,
        hash: before_hash,
    };

    let mut candidate_snapshot = before_snapshot;
    let Some(effect) = candidate_snapshot
        .effects
        .iter_mut()
        .find(|effect| effect.id == request.payload.effect_id)
    else {
        return terminal_failure(request, shape_sha256, AuthoredCommandErrorCodeV1::NotFound);
    };
    effect.enabled = request.payload.enabled;
    let after_project =
        project_file_for_save_from_parts(candidate_snapshot, &coordinator.ancillary);
    let after = ProjectCheckpoint {
        project: after_project,
        mappings: coordinator.mappings.clone(),
        epoch: coordinator.epoch,
        revision: coordinator.revision,
        hash: String::new(),
    };
    // This preflights history entry, revision, checkpoint hash and publication
    // generation before the engine ACK. Its post-ACK half has no fallible work.
    let plan = match prepare_internal_media_asset_commit(
        &coordinator,
        "Set effect enabled",
        "",
        before,
        after,
        current_unix_ms().min(u64::MAX as u128) as u64,
    ) {
        Ok(plan) => plan,
        Err(_) => {
            return terminal_failure(request, shape_sha256, AuthoredCommandErrorCodeV1::Internal)
        }
    };
    // The plan itself has already checked the post-change publication counter;
    // keep this explicit preflight visible beside the candidate B audit.
    if checked_project_authority_publication_generation_after_change(&coordinator).is_err() {
        return terminal_failure(request, shape_sha256, AuthoredCommandErrorCodeV1::Internal);
    }
    let published = run_admitted_internal_media_asset_transaction(
        &state.project_transaction_active,
        &mut coordinator,
        plan,
        publish,
    );
    if published.is_err() {
        return terminal_failure(
            request,
            shape_sha256,
            AuthoredCommandErrorCodeV1::PublicationFailed,
        );
    }
    terminal_applied(
        request,
        shape_sha256,
        fence_from_coordinator(&coordinator, &request.expected_fence),
        false,
    )
}

#[cfg(test)]
mod tests {
    use std::{
        sync::{
            atomic::{AtomicU64, Ordering},
            Arc, Mutex, MutexGuard, OnceLock,
        },
        thread,
    };

    use protocol::{
        AttributeControl, AttributeResolution, EffectBlendMode, EffectKind, EffectSummary,
        FixtureLimits, LfoShape, PatchedFixtureSummary, Rotation3, Vec3,
    };

    use super::*;
    use crate::{
        tests::{MediaAssetA6CommandHarness, MEDIA_ASSET_A6_OWNER},
        BeginProjectTransactionRequest,
    };

    const PRIMARY_WINDOW: &str = "media-asset-a6";

    /// The real `EngineHandle` fixture owns process-wide runtime worker
    /// resources. Keep independent authored fixtures serialized while each
    /// test retains its own real engine and preserve the explicit two-principal
    /// concurrent admission race inside its single test body.
    fn serialized_authored_test_guard() -> MutexGuard<'static, ()> {
        static GUARD: OnceLock<Mutex<()>> = OnceLock::new();
        GUARD
            .get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn hash(seed: char) -> String {
        std::iter::repeat_n(seed, 64).collect()
    }

    fn request(enabled: bool) -> AuthoredRequestV1<SetEffectEnabledPayload> {
        AuthoredRequestV1 {
            operation_id: SET_EFFECT_ENABLED_OPERATION_ID.to_string(),
            request_id: 77,
            expected_fence: ProjectMutationFenceV1 {
                process_incarnation: 1,
                session_incarnation: 2,
                project_epoch: 3,
                project_revision: 4,
                project_checkpoint_hash: hash('a'),
                project_publication_generation: 5,
            },
            payload: SetEffectEnabledPayload {
                effect_id: 8,
                enabled,
            },
        }
    }

    fn binding(principal: &str) -> AuthoredCallerBinding {
        AuthoredCallerBinding {
            principal: principal.to_string(),
            window_label: "main".to_string(),
            owner_incarnation: 9,
        }
    }

    fn success_receipt(
        request: &AuthoredRequestV1<SetEffectEnabledPayload>,
        shape: &str,
    ) -> SetEffectEnabledTerminalReceiptV1 {
        terminal_applied(request, shape, request.expected_fence.clone(), true)
    }

    fn real_fixture(id: protocol::FixtureId) -> PatchedFixtureSummary {
        PatchedFixtureSummary {
            id,
            label: "Authored control-plane fixture".to_string(),
            profile_source_path: "memory://authored-control-plane.gdtf".to_string(),
            profile_name: "Authored control-plane fixture".to_string(),
            manufacturer: "Syndocal test".to_string(),
            mode_name: "Standard".to_string(),
            universe: 0,
            address: 1,
            group_ids: Vec::new(),
            position: Vec3::default(),
            rotation: Rotation3::default(),
            geometries: Vec::new(),
            controls: vec![AttributeControl {
                attribute: "Dimmer".to_string(),
                channel_name: "Dimmer".to_string(),
                geometry: None,
                offsets: vec![1],
                resolution: AttributeResolution::EightBit,
                default_value: 0,
                functions: Vec::new(),
            }],
            attribute_values: Vec::new(),
            limits: FixtureLimits::default(),
            highlighted: false,
            soloed: false,
            parked: false,
        }
    }

    fn real_lfo_effect(
        id: protocol::EffectId,
        fixture_id: protocol::FixtureId,
        enabled: bool,
    ) -> EffectSummary {
        EffectSummary {
            id,
            label: "Authored control-plane toggle".to_string(),
            effect_type: EffectKind::Lfo,
            fixture_ids: vec![fixture_id],
            target_group_ids: Vec::new(),
            attribute: "Dimmer".to_string(),
            video_targets: Vec::new(),
            shape: LfoShape::Sine,
            period_ms: Some(500),
            clock_sync: None,
            low: 0,
            high: u16::MAX,
            phase: 0.0,
            fixture_spread: 0.0,
            blend_mode: EffectBlendMode::Override,
            origin: None,
            direction: None,
            speed: None,
            wavelength: None,
            enabled,
            lfo: None,
            color: None,
            chaser: None,
            move_effect: None,
            value: None,
            curve: None,
            mapping: None,
            color_mapping: None,
        }
    }

    fn seeded_real_handler(
        enabled: bool,
    ) -> (
        MediaAssetA6CommandHarness,
        ControlPlaneQueryState,
        protocol::EffectId,
    ) {
        let harness = MediaAssetA6CommandHarness::new();
        let fixture_id = harness.state.engine.allocate_fixture_id();
        let effect_id = harness.state.engine.allocate_effect_id();
        let mut snapshot = harness.state.engine.snapshot();
        snapshot.fixtures.push(real_fixture(fixture_id));
        snapshot
            .effects
            .push(real_lfo_effect(effect_id, fixture_id, enabled));
        harness
            .state
            .engine
            .load_project_snapshot_and_wait(snapshot)
            .expect("install real effect fixture");
        *harness.state.project_coordinator.lock().unwrap() =
            crate::project_coordinator_for_initial_snapshot(harness.state.engine.snapshot());
        (harness, ControlPlaneQueryState::new().unwrap(), effect_id)
    }

    fn issued_request(
        state: &AppState,
        query: &ControlPlaneQueryState,
        window_label: &str,
        effect_id: protocol::EffectId,
        enabled: bool,
        request_id: u64,
    ) -> AuthoredRequestV1<SetEffectEnabledPayload> {
        AuthoredRequestV1 {
            operation_id: SET_EFFECT_ENABLED_OPERATION_ID.to_string(),
            request_id,
            expected_fence: query
                .issue_project_mutation_fence_for_window(window_label, state)
                .expect("issue canonical local mutation fence"),
            payload: SetEffectEnabledPayload { effect_id, enabled },
        }
    }

    fn issued_request_at(
        state: &AppState,
        query: &ControlPlaneQueryState,
        window_label: &str,
        effect_id: protocol::EffectId,
        enabled: bool,
        request_id: u64,
        now: Instant,
    ) -> AuthoredRequestV1<SetEffectEnabledPayload> {
        AuthoredRequestV1 {
            operation_id: SET_EFFECT_ENABLED_OPERATION_ID.to_string(),
            request_id,
            expected_fence: query
                .issue_project_mutation_fence_for_window_at(window_label, state, now)
                .expect("issue canonical local mutation fence at fake clock"),
            payload: SetEffectEnabledPayload { effect_id, enabled },
        }
    }

    fn effect_enabled(state: &AppState, effect_id: protocol::EffectId) -> bool {
        state
            .engine
            .persistence_snapshot()
            .unwrap()
            .effects
            .iter()
            .find(|effect| effect.id == effect_id)
            .expect("fixture effect remains present")
            .enabled
    }

    fn engine_persistence_bytes(state: &AppState) -> Vec<u8> {
        serde_json::to_vec(&crate::project_snapshot_for_save(
            state.engine.persistence_snapshot().unwrap(),
        ))
        .unwrap()
    }

    fn coordinator_audit(state: &AppState) -> String {
        format!("{:#?}", *state.project_coordinator.lock().unwrap())
    }

    fn history_depths(state: &AppState) -> (usize, usize, usize, u64, u64, u64) {
        let coordinator = state.project_coordinator.lock().unwrap();
        (
            coordinator.history.undo.len(),
            coordinator.history.redo.len(),
            coordinator.history.pending.len(),
            coordinator.revision,
            coordinator.publication_generation,
            coordinator.history_generation,
        )
    }

    /// The production navigation core has an unavoidable `AppHandle<Wry>`
    /// boundary for recovery persistence and native-output retirement. The
    /// command fixture intentionally has no GUI/runtime handle, so this is
    /// the smallest headless adapter: it performs a real EngineHandle snapshot
    /// acknowledgement, then invokes the same production post-ACK coordinator
    /// commit used by the Wry path. The caller still takes the production
    /// lifecycle/external/coordinator locks and runs the same generic history
    /// core below.
    fn replace_history_snapshot_in_real_headless_harness(
        state: &AppState,
        mut prepared: crate::PreparedProjectLoad,
        coordinator: &mut ProjectCoordinator,
    ) -> Result<crate::ProjectLoadResult, String> {
        let next_project = crate::project_file_from_prepared_load(&prepared);
        let next_checkpoint_hash =
            crate::project_checkpoint_hash(&next_project, &prepared.mappings)?;
        let next_revision = crate::checked_project_revision_after_mutation(coordinator)?;
        let next_authority_disposition_generation =
            crate::checked_project_authority_disposition_generation_after_change(coordinator)?;
        crate::preflight_project_swap_ancillary_mirrors(state)?;
        crate::preflight_project_runtime_reset(state)?;
        crate::publish_project_snapshot_with_runtime_reset_admission(
            state,
            prepared.snapshot.clone(),
        )?;
        crate::reset_project_runtime_after_published_snapshot_infallible(state);
        crate::commit_history_navigation_coordinator_after_ack(
            state,
            &prepared,
            coordinator,
            next_authority_disposition_generation,
            next_revision,
            next_checkpoint_hash,
        );
        crate::synchronize_project_load_result_authority_metadata(
            &mut prepared.result,
            coordinator,
        );
        prepared.result.authority = Some(crate::project_authority_bundle_from_coordinator(
            state,
            coordinator,
        ));
        Ok(prepared.result)
    }

    fn navigate_history_in_real_headless_harness(
        state: &AppState,
        undo: bool,
    ) -> Result<crate::ProjectHistoryNavigationResult, String> {
        let _lifecycle = crate::lock_standby_sync_lifecycle_for_project_swap(state)?;
        crate::stop_standby_sync_for_project_swap(state)?;
        let _external_admission = crate::lock_project_external_command_admission(state)?;
        let mut coordinator = crate::lock_project_coordinator(state)?;
        crate::navigate_project_history_with_coordinator(
            state,
            &mut coordinator,
            undo,
            None,
            None,
            None,
            replace_history_snapshot_in_real_headless_harness,
        )
    }

    fn install_test_owner(state: &AppState, window_label: &str, principal: &str, incarnation: u64) {
        let _rotation = state.project_transaction_owner_rotation.lock().unwrap();
        state
            .project_transaction_owners
            .lock()
            .unwrap()
            .insert(window_label.to_string(), principal.to_string());
        state
            .project_transaction_owner_incarnations
            .lock()
            .unwrap()
            .insert(window_label.to_string(), incarnation);
    }

    fn terminal_outcome(response: &SetEffectEnabledResponseV1) -> &SetEffectEnabledOutcomeV1 {
        match response {
            SetEffectEnabledResponseV1::TerminalReceipt(receipt) => &receipt.outcome,
            SetEffectEnabledResponseV1::Rejected(_) => panic!("expected terminal receipt"),
        }
    }

    #[test]
    fn exact_reply_retry_is_single_flight_and_byte_identical() {
        let _serial = serialized_authored_test_guard();
        let state = Arc::new(AuthoredControlPlaneState::default());
        let request = request(true);
        let binding = binding("principal-a");
        let key = receipt_key(&request, &binding);
        let shape = shape_sha256(&request).unwrap();
        let published = Arc::new(AtomicU64::new(0));
        let first_state = Arc::clone(&state);
        let first_request = request.clone();
        let first_shape = shape.clone();
        let first_key = key.clone();
        let first_published = Arc::clone(&published);
        let first = thread::spawn(move || {
            first_state.terminal_single_flight(
                first_key,
                &first_request,
                first_shape.clone(),
                || {
                    first_published.fetch_add(1, Ordering::AcqRel);
                    success_receipt(&first_request, &first_shape)
                },
            )
        });
        let second = state.terminal_single_flight(key, &request, shape.clone(), || {
            published.fetch_add(1, Ordering::AcqRel);
            success_receipt(&request, &shape)
        });
        let first = first.join().unwrap();
        assert_eq!(published.load(Ordering::Acquire), 1);
        assert_eq!(
            serde_json::to_vec(&first).unwrap(),
            serde_json::to_vec(&second).unwrap()
        );
    }

    #[test]
    fn same_key_different_shape_is_rejected_without_republishing() {
        let _serial = serialized_authored_test_guard();
        let state = AuthoredControlPlaneState::default();
        let original = request(true);
        let changed = request(false);
        let key = receipt_key(&original, &binding("principal-a"));
        let shape = shape_sha256(&original).unwrap();
        let published = AtomicU64::new(0);
        let first = state.terminal_single_flight(key.clone(), &original, shape.clone(), || {
            published.fetch_add(1, Ordering::AcqRel);
            success_receipt(&original, &shape)
        });
        assert!(matches!(
            first,
            SetEffectEnabledResponseV1::TerminalReceipt(_)
        ));
        let changed_shape = shape_sha256(&changed).unwrap();
        let rejected = state.terminal_single_flight(key, &changed, changed_shape, || {
            published.fetch_add(1, Ordering::AcqRel);
            unreachable!("shape conflict must not invoke publish")
        });
        assert_eq!(published.load(Ordering::Acquire), 1);
        assert!(matches!(
            rejected,
            SetEffectEnabledResponseV1::Rejected(AuthoredCommandRejectionV1 {
                error: AuthoredCommandErrorV1 {
                    code: AuthoredCommandErrorCodeV1::Conflict
                },
                ..
            })
        ));
    }

    #[test]
    fn owner_retirement_tombstones_old_principal_receipts() {
        let _serial = serialized_authored_test_guard();
        let state = AuthoredControlPlaneState::default();
        let request = request(true);
        let key = receipt_key(&request, &binding("principal-a"));
        let shape = shape_sha256(&request).unwrap();
        let _ = state.terminal_single_flight(key.clone(), &request, shape.clone(), || {
            success_receipt(&request, &shape)
        });
        state.retire_principal("principal-a");
        let response = state.terminal_single_flight(key, &request, shape, || {
            unreachable!("retired receipt key must never publish")
        });
        assert!(matches!(
            response,
            SetEffectEnabledResponseV1::Rejected(AuthoredCommandRejectionV1 {
                error: AuthoredCommandErrorV1 {
                    code: AuthoredCommandErrorCodeV1::Conflict
                },
                ..
            })
        ));
    }

    #[test]
    fn real_handler_retry_publishes_once_and_retains_byte_identical_receipt() {
        let _serial = serialized_authored_test_guard();
        let (harness, query, effect_id) = seeded_real_handler(false);
        let before_depths = history_depths(&harness.state);
        let request = issued_request(&harness.state, &query, PRIMARY_WINDOW, effect_id, true, 101);

        // Simulates a lost IPC reply: the exact envelope reaches the real
        // handler again after the first EngineHandle acknowledgement.
        let first = set_effect_enabled_authoritative_for_window_label(
            &harness.state,
            &query,
            PRIMARY_WINDOW,
            request.clone(),
        );
        let retry = set_effect_enabled_authoritative_for_window_label(
            &harness.state,
            &query,
            PRIMARY_WINDOW,
            request,
        );

        assert!(
            matches!(
                terminal_outcome(&first),
                SetEffectEnabledOutcomeV1::Applied(applied)
                    if applied.effect_id == effect_id && applied.enabled
            ),
            "unexpected real handler result: {first:?}"
        );
        assert_eq!(
            serde_json::to_vec(&first).unwrap(),
            serde_json::to_vec(&retry).unwrap(),
            "a reply-loss retry must return the retained terminal bytes"
        );
        assert_eq!(
            harness
                .state
                .authored_effect_enabled_publish_attempts
                .load(Ordering::Acquire),
            1,
            "only the first exact request may reach EngineHandle"
        );
        assert!(effect_enabled(&harness.state, effect_id));
        let after_depths = history_depths(&harness.state);
        assert_eq!(after_depths.0, before_depths.0 + 1);
        assert_eq!(after_depths.1, before_depths.1);
        assert_eq!(after_depths.2, before_depths.2);
        assert_eq!(after_depths.3, before_depths.3 + 1);
        assert_eq!(after_depths.4, before_depths.4 + 1);
        assert_eq!(after_depths.5, before_depths.5 + 1);

        // The exact committed A/B entry is the input to the existing generic
        // Undo/Redo route: its undo side is A and redo side is B.
        let coordinator = harness.state.project_coordinator.lock().unwrap();
        let entry = coordinator.history.undo.last().unwrap();
        assert!(
            !entry
                .before
                .project
                .snapshot
                .effects
                .iter()
                .find(|effect| effect.id == effect_id)
                .unwrap()
                .enabled
        );
        assert!(
            entry
                .after
                .project
                .snapshot
                .effects
                .iter()
                .find(|effect| effect.id == effect_id)
                .unwrap()
                .enabled
        );
    }

    #[test]
    fn real_authored_commit_undoes_to_exact_a_and_redoes_to_exact_b_through_navigation_core() {
        let _serial = serialized_authored_test_guard();
        let (harness, query, effect_id) = seeded_real_handler(false);
        let a_bytes = engine_persistence_bytes(&harness.state);
        let a_hash = harness
            .state
            .project_coordinator
            .lock()
            .unwrap()
            .checkpoint_hash
            .clone();
        let applied = set_effect_enabled_authoritative_for_window_label(
            &harness.state,
            &query,
            PRIMARY_WINDOW,
            issued_request(&harness.state, &query, PRIMARY_WINDOW, effect_id, true, 110),
        );
        assert!(matches!(
            terminal_outcome(&applied),
            SetEffectEnabledOutcomeV1::Applied(change) if change.effect_id == effect_id && change.enabled
        ));
        let b_bytes = engine_persistence_bytes(&harness.state);
        let b_hash = harness
            .state
            .project_coordinator
            .lock()
            .unwrap()
            .checkpoint_hash
            .clone();
        assert_ne!(a_bytes, b_bytes);
        assert_ne!(a_hash, b_hash);

        // This calls the same production history CAS/prepare/pop/publish/
        // rewrite/push core as `undo_project_transaction`; the headless
        // adapter above only substitutes the Wry-only output/recovery shell.
        let undo = navigate_history_in_real_headless_harness(&harness.state, true)
            .expect("Undo authored effect B through the production navigation core");
        assert!(undo.history_status.can_redo);
        assert!(!undo.history_status.can_undo);
        assert_eq!(engine_persistence_bytes(&harness.state), a_bytes);
        assert_eq!(
            harness
                .state
                .project_coordinator
                .lock()
                .unwrap()
                .checkpoint_hash,
            a_hash
        );
        assert!(!effect_enabled(&harness.state, effect_id));

        let redo = navigate_history_in_real_headless_harness(&harness.state, false)
            .expect("Redo authored effect B through the production navigation core");
        assert!(redo.history_status.can_undo);
        assert!(!redo.history_status.can_redo);
        assert_eq!(engine_persistence_bytes(&harness.state), b_bytes);
        assert_eq!(
            harness
                .state
                .project_coordinator
                .lock()
                .unwrap()
                .checkpoint_hash,
            b_hash
        );
        assert!(effect_enabled(&harness.state, effect_id));
        assert_eq!(
            harness
                .state
                .authored_effect_enabled_publish_attempts
                .load(Ordering::Acquire),
            1,
            "Undo/Redo must navigate snapshots rather than replay the authored command"
        );
    }

    #[test]
    fn real_handler_same_receipt_key_with_different_shape_rejects_without_drift() {
        let _serial = serialized_authored_test_guard();
        let (harness, query, effect_id) = seeded_real_handler(false);
        let original = issued_request(&harness.state, &query, PRIMARY_WINDOW, effect_id, true, 102);
        let first = set_effect_enabled_authoritative_for_window_label(
            &harness.state,
            &query,
            PRIMARY_WINDOW,
            original.clone(),
        );
        assert!(matches!(
            terminal_outcome(&first),
            SetEffectEnabledOutcomeV1::Applied(_)
        ));
        let bytes_after_first = engine_persistence_bytes(&harness.state);
        let coordinator_after_first = coordinator_audit(&harness.state);
        let mut different_shape = original;
        different_shape.payload.enabled = false;
        let rejected = set_effect_enabled_authoritative_for_window_label(
            &harness.state,
            &query,
            PRIMARY_WINDOW,
            different_shape,
        );

        assert!(matches!(
            rejected,
            SetEffectEnabledResponseV1::Rejected(AuthoredCommandRejectionV1 {
                error: AuthoredCommandErrorV1 {
                    code: AuthoredCommandErrorCodeV1::Conflict
                },
                ..
            })
        ));
        assert_eq!(
            harness
                .state
                .authored_effect_enabled_publish_attempts
                .load(Ordering::Acquire),
            1
        );
        assert_eq!(engine_persistence_bytes(&harness.state), bytes_after_first);
        assert_eq!(coordinator_audit(&harness.state), coordinator_after_first);
    }

    #[test]
    fn retained_exact_receipt_precedes_expired_issued_fence_and_tombstones_fail_closed() {
        let _serial = serialized_authored_test_guard();
        let (harness, query, effect_id) = seeded_real_handler(false);
        let issued_at = Instant::now();
        let request = issued_request_at(
            &harness.state,
            &query,
            PRIMARY_WINDOW,
            effect_id,
            true,
            111,
            issued_at,
        );
        let first = set_effect_enabled_authoritative_for_window_label_at(
            &harness.state,
            &query,
            PRIMARY_WINDOW,
            request.clone(),
            issued_at,
        );
        assert!(
            matches!(
                terminal_outcome(&first),
                SetEffectEnabledOutcomeV1::Applied(_)
            ),
            "fake-clock first publish must apply, got {first:?}"
        );
        let after_first_engine = engine_persistence_bytes(&harness.state);
        let after_first_coordinator = coordinator_audit(&harness.state);

        // The issued capability has expired, but the longer retained terminal
        // receipt must still recover an exact lost reply before any fence TTL
        // purge/validation is allowed to run.
        let expired_fence_but_live_receipt =
            issued_at + Duration::from_secs(10 * 60) + Duration::from_millis(1);
        let retry = set_effect_enabled_authoritative_for_window_label_at(
            &harness.state,
            &query,
            PRIMARY_WINDOW,
            request.clone(),
            expired_fence_but_live_receipt,
        );
        assert_eq!(
            serde_json::to_vec(&retry).unwrap(),
            serde_json::to_vec(&first).unwrap(),
            "an exact retry must survive issued-fence expiry while receipt retention is live"
        );
        assert_eq!(
            harness
                .state
                .authored_effect_enabled_publish_attempts
                .load(Ordering::Acquire),
            1,
        );

        let mut shape_conflict = request.clone();
        shape_conflict.payload.enabled = false;
        let conflict = set_effect_enabled_authoritative_for_window_label_at(
            &harness.state,
            &query,
            PRIMARY_WINDOW,
            shape_conflict,
            expired_fence_but_live_receipt,
        );
        assert!(matches!(
            conflict,
            SetEffectEnabledResponseV1::Rejected(AuthoredCommandRejectionV1 {
                error: AuthoredCommandErrorV1 {
                    code: AuthoredCommandErrorCodeV1::Conflict
                },
                ..
            })
        ));

        // No caller can select the retained key's principal/window/owner
        // binding: an unregistered injected window is forbidden before lookup.
        let unknown_owner = set_effect_enabled_authoritative_for_window_label_at(
            &harness.state,
            &query,
            "unregistered-window",
            request.clone(),
            expired_fence_but_live_receipt,
        );
        assert!(matches!(
            unknown_owner,
            SetEffectEnabledResponseV1::Rejected(AuthoredCommandRejectionV1 {
                error: AuthoredCommandErrorV1 {
                    code: AuthoredCommandErrorCodeV1::Forbidden
                },
                ..
            })
        ));
        assert_eq!(engine_persistence_bytes(&harness.state), after_first_engine);
        assert_eq!(coordinator_audit(&harness.state), after_first_coordinator);

        // Once receipt retention ends, first lookup creates a tombstone. The
        // retry remains a conflict through its tombstone TTL rather than being
        // mistaken for a new stale-fence mutation.
        let receipt_expired = issued_at + TERMINAL_RECEIPT_TTL + Duration::from_millis(1);
        let tombstoned = set_effect_enabled_authoritative_for_window_label_at(
            &harness.state,
            &query,
            PRIMARY_WINDOW,
            request.clone(),
            receipt_expired,
        );
        assert!(matches!(
            tombstoned,
            SetEffectEnabledResponseV1::Rejected(AuthoredCommandRejectionV1 {
                error: AuthoredCommandErrorV1 {
                    code: AuthoredCommandErrorCodeV1::Conflict
                },
                ..
            })
        ));
        let tombstone_expired =
            receipt_expired + RETIRED_KEY_TOMBSTONE_TTL + Duration::from_millis(1);
        let fully_expired = set_effect_enabled_authoritative_for_window_label_at(
            &harness.state,
            &query,
            PRIMARY_WINDOW,
            request,
            tombstone_expired,
        );
        assert!(matches!(
            fully_expired,
            SetEffectEnabledResponseV1::Rejected(AuthoredCommandRejectionV1 {
                error: AuthoredCommandErrorV1 {
                    code: AuthoredCommandErrorCodeV1::Forbidden
                },
                ..
            })
        ));
        assert_eq!(engine_persistence_bytes(&harness.state), after_first_engine);
        assert_eq!(coordinator_audit(&harness.state), after_first_coordinator);
    }

    #[test]
    fn real_handler_noop_is_receipted_without_publication_or_history_drift() {
        let _serial = serialized_authored_test_guard();
        let (harness, query, effect_id) = seeded_real_handler(true);
        let before_engine = engine_persistence_bytes(&harness.state);
        let before_coordinator = coordinator_audit(&harness.state);
        let request = issued_request(&harness.state, &query, PRIMARY_WINDOW, effect_id, true, 103);
        let response = set_effect_enabled_authoritative_for_window_label(
            &harness.state,
            &query,
            PRIMARY_WINDOW,
            request.clone(),
        );

        match terminal_outcome(&response) {
            SetEffectEnabledOutcomeV1::NoOp(applied) => {
                assert_eq!(applied.post_fence, request.expected_fence);
                assert_eq!(applied.effect_id, effect_id);
                assert!(applied.enabled);
            }
            _ => panic!("unchanged effect must receive a NoOp receipt"),
        }
        assert_eq!(
            harness
                .state
                .authored_effect_enabled_publish_attempts
                .load(Ordering::Acquire),
            0
        );
        assert_eq!(engine_persistence_bytes(&harness.state), before_engine);
        assert_eq!(coordinator_audit(&harness.state), before_coordinator);
    }

    #[test]
    fn publication_failure_keeps_a_and_coordinator_identical_and_is_receipted() {
        let _serial = serialized_authored_test_guard();
        let (harness, query, effect_id) = seeded_real_handler(false);
        let request = issued_request(&harness.state, &query, PRIMARY_WINDOW, effect_id, true, 104);
        let binding = capture_caller_binding(&harness.state, PRIMARY_WINDOW).unwrap();
        let key = receipt_key(&request, &binding);
        let shape = shape_sha256(&request).unwrap();
        let before_engine = engine_persistence_bytes(&harness.state);
        let before_coordinator = coordinator_audit(&harness.state);
        let failed = harness.state.authored_control_plane.terminal_single_flight(
            key.clone(),
            &request,
            shape.clone(),
            || {
                execute_set_effect_enabled_with_publish(
                    &harness.state,
                    &binding,
                    &request,
                    &shape,
                    || Err("forced pre-ack publication failure".to_string()),
                )
            },
        );
        let retry = harness.state.authored_control_plane.terminal_single_flight(
            key,
            &request,
            shape,
            || panic!("a failed terminal receipt must be retained before reply"),
        );

        assert!(matches!(
            terminal_outcome(&failed),
            SetEffectEnabledOutcomeV1::Failed(AuthoredCommandErrorV1 {
                code: AuthoredCommandErrorCodeV1::PublicationFailed
            })
        ));
        assert_eq!(
            serde_json::to_vec(&failed).unwrap(),
            serde_json::to_vec(&retry).unwrap()
        );
        assert_eq!(engine_persistence_bytes(&harness.state), before_engine);
        assert_eq!(coordinator_audit(&harness.state), before_coordinator);
        assert!(!harness
            .state
            .project_transaction_active
            .load(Ordering::Acquire));
    }

    #[test]
    fn two_principals_from_one_start_fence_admit_only_one_and_stale_the_other() {
        let _serial = serialized_authored_test_guard();
        let (harness, query, effect_id) = seeded_real_handler(false);
        install_test_owner(&harness.state, "other-window", "renderer:other", 2);
        let query = Arc::new(query);
        let primary = issued_request(&harness.state, &query, PRIMARY_WINDOW, effect_id, true, 105);
        let other = issued_request(&harness.state, &query, "other-window", effect_id, true, 106);
        assert_eq!(
            (
                primary.expected_fence.process_incarnation,
                primary.expected_fence.project_epoch,
                primary.expected_fence.project_revision,
                &primary.expected_fence.project_checkpoint_hash,
                primary.expected_fence.project_publication_generation,
            ),
            (
                other.expected_fence.process_incarnation,
                other.expected_fence.project_epoch,
                other.expected_fence.project_revision,
                &other.expected_fence.project_checkpoint_hash,
                other.expected_fence.project_publication_generation,
            )
        );
        assert_ne!(
            primary.expected_fence.session_incarnation,
            other.expected_fence.session_incarnation
        );
        let state_a = Arc::clone(&harness.state);
        let query_a = Arc::clone(&query);
        let first = thread::spawn(move || {
            set_effect_enabled_authoritative_for_window_label(
                &state_a,
                &query_a,
                PRIMARY_WINDOW,
                primary,
            )
        });
        let state_b = Arc::clone(&harness.state);
        let query_b = Arc::clone(&query);
        let second = thread::spawn(move || {
            set_effect_enabled_authoritative_for_window_label(
                &state_b,
                &query_b,
                "other-window",
                other,
            )
        });
        let responses = [first.join().unwrap(), second.join().unwrap()];

        assert_eq!(
            responses
                .iter()
                .filter(|response| {
                    matches!(
                        terminal_outcome(response),
                        SetEffectEnabledOutcomeV1::Applied(_)
                    )
                })
                .count(),
            1
        );
        assert_eq!(
            responses
                .iter()
                .filter(|response| {
                    matches!(
                        terminal_outcome(response),
                        SetEffectEnabledOutcomeV1::Failed(AuthoredCommandErrorV1 {
                            code: AuthoredCommandErrorCodeV1::StaleFence
                        })
                    )
                })
                .count(),
            1
        );
        assert_eq!(
            harness
                .state
                .authored_effect_enabled_publish_attempts
                .load(Ordering::Acquire),
            1
        );
        assert!(effect_enabled(&harness.state, effect_id));
    }

    #[test]
    fn owner_aba_and_other_window_requests_are_forbidden_without_drift() {
        let _serial = serialized_authored_test_guard();
        let (harness, query, effect_id) = seeded_real_handler(false);
        let old_request =
            issued_request(&harness.state, &query, PRIMARY_WINDOW, effect_id, true, 107);
        crate::retire_project_transaction_owner_for_window(&harness.state, PRIMARY_WINDOW)
            .expect("retire old renderer owner");
        install_test_owner(&harness.state, PRIMARY_WINDOW, MEDIA_ASSET_A6_OWNER, 2);
        let fresh_request =
            issued_request(&harness.state, &query, PRIMARY_WINDOW, effect_id, true, 108);
        assert_ne!(
            old_request.expected_fence.session_incarnation,
            fresh_request.expected_fence.session_incarnation,
            "owner ABA must get a fresh server-only mutation session"
        );
        install_test_owner(&harness.state, "other-window", "renderer:other", 1);
        let _other_fence =
            issued_request(&harness.state, &query, "other-window", effect_id, true, 109);
        let before_engine = engine_persistence_bytes(&harness.state);
        let before_coordinator = coordinator_audit(&harness.state);
        let aba_rejected = set_effect_enabled_authoritative_for_window_label(
            &harness.state,
            &query,
            PRIMARY_WINDOW,
            old_request,
        );
        let cross_window_rejected = set_effect_enabled_authoritative_for_window_label(
            &harness.state,
            &query,
            "other-window",
            fresh_request.clone(),
        );
        for response in [aba_rejected, cross_window_rejected] {
            assert!(matches!(
                response,
                SetEffectEnabledResponseV1::Rejected(AuthoredCommandRejectionV1 {
                    error: AuthoredCommandErrorV1 {
                        code: AuthoredCommandErrorCodeV1::Forbidden
                    },
                    ..
                })
            ));
        }
        assert_eq!(engine_persistence_bytes(&harness.state), before_engine);
        assert_eq!(coordinator_audit(&harness.state), before_coordinator);

        let applied = set_effect_enabled_authoritative_for_window_label(
            &harness.state,
            &query,
            PRIMARY_WINDOW,
            fresh_request,
        );
        assert!(matches!(
            terminal_outcome(&applied),
            SetEffectEnabledOutcomeV1::Applied(_)
        ));
    }

    #[test]
    fn destroyed_retirement_marker_fences_begin_and_authored_gap_without_drift() {
        let _serial = serialized_authored_test_guard();
        let (harness, query, effect_id) = seeded_real_handler(false);
        let state = Arc::clone(&harness.state);
        let query = Arc::new(query);
        let request = issued_request(&state, &query, PRIMARY_WINDOW, effect_id, true, 112);
        let owner_incarnation = *state
            .project_transaction_owner_incarnations
            .lock()
            .unwrap()
            .get(PRIMARY_WINDOW)
            .expect("seeded owner incarnation");
        let (expected_epoch, expected_revision) = {
            let coordinator = state.project_coordinator.lock().unwrap();
            (coordinator.epoch, coordinator.revision)
        };
        let begin_label = "Retiring Begin".to_string();
        let begin_coalesce_key = "retiring:begin".to_string();
        let begin_shape = crate::canonical_project_transaction_shape(
            "retiring_begin",
            &begin_label,
            &begin_coalesce_key,
        );
        let before_engine = engine_persistence_bytes(&state);
        let before_coordinator = coordinator_audit(&state);
        let before_history = history_depths(&state);
        let before_publish = state
            .authored_effect_enabled_publish_attempts
            .load(Ordering::Acquire);

        let poison_state = Arc::clone(&state);
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
            let _guard = poison_state
                .output_lease_registry
                .lock()
                .expect("output lease registry before gap poison");
            panic!("poison output lease registry between owner and query retirement");
        }));

        let mut begin_thread = None;
        let mut authored_thread = None;
        let retirement =
            crate::handle_destroyed_window_authority_retirement_for_incarnation_with_hook(
                &state,
                &query,
                PRIMARY_WINDOW,
                owner_incarnation,
                || {
                    assert!(
                        state
                            .ensure_window_authority_not_blocked(PRIMARY_WINDOW)
                            .is_err(),
                        "temporary Destroyed-retirement marker must reject the interleaving"
                    );

                    let begin_state = Arc::clone(&state);
                    let begin_owner = MEDIA_ASSET_A6_OWNER.to_string();
                    let begin_label_for_thread = begin_label.clone();
                    let begin_coalesce_for_thread = begin_coalesce_key.clone();
                    let begin_shape_for_thread = begin_shape.clone();
                    begin_thread = Some(thread::spawn(move || {
                        crate::begin_project_transaction_for_window_label(
                            &begin_state,
                            PRIMARY_WINDOW,
                            BeginProjectTransactionRequest {
                                label: begin_label_for_thread,
                                coalesce_key: begin_coalesce_for_thread,
                                expected_epoch,
                                expected_revision,
                                owner_id: begin_owner,
                                client_operation_id: "project-op:99:e1-retiring-gap".to_string(),
                                shape_fingerprint: begin_shape_for_thread,
                                command_name: "retiring_begin".to_string(),
                                schema_version: crate::PROJECT_TRANSACTION_SCHEMA_VERSION,
                            },
                        )
                    }));

                    let authored_state = Arc::clone(&state);
                    let authored_query = Arc::clone(&query);
                    authored_thread = Some(thread::spawn(move || {
                        set_effect_enabled_authoritative_for_window_label(
                            &authored_state,
                            &authored_query,
                            PRIMARY_WINDOW,
                            request,
                        )
                    }));
                },
            );
        assert!(
            retirement.is_err(),
            "poisoned owner retirement must block the label"
        );

        let begin_error = begin_thread
            .expect("interleaved Begin thread")
            .join()
            .expect("interleaved Begin thread joins")
            .expect_err("new Begin must be rejected during Destroyed retirement");
        assert!(begin_error.contains("authority") || begin_error.contains("retirement"));
        let authored_response = authored_thread
            .expect("interleaved authored thread")
            .join()
            .expect("interleaved authored thread joins");
        assert!(matches!(
            authored_response,
            SetEffectEnabledResponseV1::Rejected(AuthoredCommandRejectionV1 {
                error: AuthoredCommandErrorV1 {
                    code: AuthoredCommandErrorCodeV1::Forbidden
                },
                ..
            })
        ));
        assert_eq!(engine_persistence_bytes(&state), before_engine);
        assert_eq!(coordinator_audit(&state), before_coordinator);
        assert_eq!(history_depths(&state), before_history);
        assert_eq!(
            state
                .authored_effect_enabled_publish_attempts
                .load(Ordering::Acquire),
            before_publish
        );
        assert!(state
            .ensure_window_authority_not_blocked(PRIMARY_WINDOW)
            .is_err());
        assert!(state
            .project_transaction_owners
            .lock()
            .unwrap()
            .contains_key(PRIMARY_WINDOW));
    }

    #[test]
    fn wrong_authored_operation_or_payload_is_rejected_before_engine_or_history() {
        let _serial = serialized_authored_test_guard();
        let (harness, query, effect_id) = seeded_real_handler(false);
        let valid = issued_request(&harness.state, &query, PRIMARY_WINDOW, effect_id, true, 110);
        let before_engine = engine_persistence_bytes(&harness.state);
        let before_coordinator = coordinator_audit(&harness.state);
        let mut wrong_operation = valid.clone();
        wrong_operation.operation_id = "syndocal.effects.runtime_set_enabled.v1".to_string();
        let mut wrong_payload = valid;
        wrong_payload.payload.effect_id = 0;
        for invalid in [wrong_operation, wrong_payload] {
            let response = set_effect_enabled_authoritative_for_window_label(
                &harness.state,
                &query,
                PRIMARY_WINDOW,
                invalid,
            );
            assert!(matches!(
                response,
                SetEffectEnabledResponseV1::Rejected(AuthoredCommandRejectionV1 {
                    error: AuthoredCommandErrorV1 {
                        code: AuthoredCommandErrorCodeV1::InvalidRequest
                    },
                    ..
                })
            ));
        }
        assert_eq!(engine_persistence_bytes(&harness.state), before_engine);
        assert_eq!(coordinator_audit(&harness.state), before_coordinator);
    }
}
