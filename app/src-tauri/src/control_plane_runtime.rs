//! Strict local runtime control-plane vertical for Timeline Play/Pause.
//!
//! This deliberately does not reuse the authored control-plane lane: transport
//! state has no persistence/history transition, and its ABA fence includes the
//! engine-owned `timeline.transport` generation.

use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use engine::{SafetyBlackoutEngageDisposition, TimelineTransportSetPlayingDisposition};
use protocol::control_plane_command::{
    OutputConsentChallengeV1, OutputConsentPhysicalStateV1, OutputConsentPrepareRequestV1,
    OutputConsentStatusRequestV1, OutputConsentStatusV1, OutputControlActionV1,
    OutputControlAuthorityBundleV1, OutputControlCommandRequestV1, OutputControlErrorCodeV1,
    OutputControlFenceV1, OutputControlReceiptOutcomeV1, OutputControlReceiptV1,
    OutputControlRejectionV1, OutputControlResponseV1, RuntimeCommandAuthorityBundleV1,
    RuntimeCommandErrorCodeV1, RuntimeCommandErrorV1, RuntimeCommandReceiptOutcomeV1,
    RuntimeCommandReceiptV1, RuntimeCommandRejectionV1, RuntimeCommandRequestV1,
    RuntimeCommandResponseV1, SafetyBlackoutEngageOutcomeV1, SafetyBlackoutEngageReceiptV1,
    SafetyBlackoutEngageRejectionV1, SafetyBlackoutEngageRequestV1, SafetyBlackoutEngageResponseV1,
    TimelineFollowAbortAuthorityBundleV1, TimelineFollowAbortRuntimeFenceV1,
    TimelineFollowAbortRuntimeReceiptV1, TimelineFollowAbortRuntimeRejectionV1,
    TimelineFollowAbortRuntimeRequestV1, TimelineFollowAbortRuntimeResponseV1,
    TimelineTransportRuntimeFenceV1, MAX_SAFE_JAVASCRIPT_INTEGER,
    OUTPUT_CONSENT_PREPARE_OPERATION_ID, OUTPUT_CONSENT_STATUS_QUERY_OPERATION_ID,
    OUTPUT_CONTROL_AUTHORITY_QUERY_OPERATION_ID, SAFETY_BLACKOUT_ENGAGE_OPERATION_ID,
    SAFETY_BLACKOUT_ENGAGE_SHAPE_DOMAIN_V1, TIMELINE_FOLLOW_ABORT_OPERATION_ID,
    TIMELINE_FOLLOW_ABORT_RUNTIME_DOMAIN_V1, TIMELINE_FOLLOW_ABORT_SHAPE_DOMAIN_V1,
    TIMELINE_TRANSPORT_RUNTIME_DOMAIN_V1, TIMELINE_TRANSPORT_SET_PLAYING_OPERATION_ID,
    TIMELINE_TRANSPORT_SET_PLAYING_SHAPE_DOMAIN_V1,
};
use sha2::{Digest, Sha256};
use tauri::WebviewWindow;

use super::control_plane_security::{
    ConsentAuthorityBinding, ConsentCallerBinding, ConsentConsumeError, PreparedConsentState,
};

use super::{
    ensure_no_pending_project_transaction, ensure_project_operator_video_clip_slot_runtime_allowed,
    lock_project_coordinator, lock_project_external_command_admission,
    reconcile_project_checkpoint_for_coordinator, AppState, ControlPlaneQueryState,
    ProjectCoordinator, StandbyCheckpointIdentity, StandbyTakeoverCheckpointSelector,
};

const RECEIPT_TTL: Duration = Duration::from_secs(10 * 60);
const TOMBSTONE_TTL: Duration = Duration::from_secs(10 * 60);
const AUTHORITY_TTL: Duration = Duration::from_secs(60);
const MAX_RECEIPTS_PER_PRINCIPAL_OPERATION: usize = 512;
const MAX_TOTAL_RECEIPTS: usize = 1_024;
const MAX_TOTAL_TOMBSTONES: usize = 1_024;
const MAX_TOTAL_AUTHORITIES: usize = 1_024;
const MAX_LANES: usize = 1_024;
const TOKEN_BUCKET_BURST: f64 = 8.0;
const TOKEN_BUCKET_PER_SECOND: f64 = 4.0;
const SAFETY_BLACKOUT_DOMAIN: &str = "safety.blackout.engage";
/// Immutable for the lifetime of the process: entries are append-only and
/// never evicted or rewritten. Persistent cross-process audit is added by the
/// sidecar tranche; this bound prevents an in-process memory sink.
const MAX_SAFETY_AUDIT_RECORDS: usize = 65_536;

pub(crate) fn issue_output_control_authority(
    window: &WebviewWindow,
    state: &AppState,
    query_state: &ControlPlaneQueryState,
) -> Result<OutputControlAuthorityBundleV1, String> {
    let fence = query_state
        .issue_output_control_fence_for_window(window.label(), state)
        .map_err(|error| format!("Output control authority is unavailable: {error:?}"))?;
    Ok(OutputControlAuthorityBundleV1 {
        operation_id: OUTPUT_CONTROL_AUTHORITY_QUERY_OPERATION_ID.to_string(),
        fence,
    })
}

pub(crate) fn prepare_output_consent(
    window: &WebviewWindow,
    state: &AppState,
    query_state: &ControlPlaneQueryState,
    request: OutputConsentPrepareRequestV1,
) -> Result<OutputConsentChallengeV1, String> {
    request
        .validate()
        .map_err(|_| "Output consent request is invalid".to_string())?;
    let binding = capture_binding(state, window.label())
        .map_err(|_| "Output consent caller is not registered".to_string())?;
    query_state
        .validate_output_control_fence_window(
            window.label(),
            &request.expected_fence,
            binding.owner_incarnation,
        )
        .map_err(|_| "Output consent authority was not issued to this renderer".to_string())?;

    {
        let _external_admission = lock_project_external_command_admission(state)?;
        let mut coordinator = lock_project_coordinator(state)?;
        reconcile_project_checkpoint_for_coordinator(state, &mut coordinator)?;
        if !exact_output_control_fence_matches(state, &coordinator, &request.expected_fence) {
            return Err("Output consent authority is stale".to_string());
        }
        ensure_no_pending_project_transaction(&coordinator)?;
        ensure_project_operator_video_clip_slot_runtime_allowed(
            state,
            &coordinator,
            &binding.principal,
        )?;
        validate_output_action_current(state, &request.action)?;
    }

    let argument_fingerprint = hex_sha256(
        &request
            .argument_fingerprint_bytes()
            .map_err(|_| "Output consent request shape is invalid".to_string())?,
    );
    let authority = ConsentAuthorityBinding {
        caller: ConsentCallerBinding {
            principal: binding.principal,
            window_label: binding.window_label,
            owner_incarnation: binding.owner_incarnation,
        },
        operation_id: request.action.operation_id().to_string(),
        argument_fingerprint: argument_fingerprint.clone(),
        project_epoch: request.expected_fence.project_epoch,
        project_revision: request.expected_fence.project_revision,
        project_checkpoint_hash: request.expected_fence.project_checkpoint_hash.clone(),
        project_publication_generation: request.expected_fence.project_publication_generation,
        output_epoch: request.expected_fence.output_epoch,
        output_generation: request.expected_fence.output_generation,
        safety_blackout_epoch: request.expected_fence.safety_blackout_epoch,
        safety_blackout_generation: request.expected_fence.safety_blackout_generation,
    };
    let prepared = state.control_plane_security.prepare_consent(authority)?;
    Ok(OutputConsentChallengeV1 {
        operation_id: OUTPUT_CONSENT_PREPARE_OPERATION_ID.to_string(),
        request_id: request.request_id,
        target_operation_id: request.action.operation_id().to_string(),
        challenge_id: prepared.challenge_id,
        consent_token: prepared.consent_token,
        display_code: prepared.display_code,
        argument_fingerprint,
        expires_at_unix_ms: prepared.expires_at_unix_ms,
    })
}

pub(crate) fn output_consent_status(
    window: &WebviewWindow,
    state: &AppState,
    request: OutputConsentStatusRequestV1,
) -> Result<OutputConsentStatusV1, String> {
    request
        .validate()
        .map_err(|_| "Output consent status request is invalid".to_string())?;
    let binding = capture_binding(state, window.label())
        .map_err(|_| "Output consent caller is not registered".to_string())?;
    let caller = ConsentCallerBinding {
        principal: binding.principal,
        window_label: binding.window_label,
        owner_incarnation: binding.owner_incarnation,
    };
    let status = state
        .control_plane_security
        .consent_status(&caller, &request.challenge_id)
        .map_err(output_consent_error_message)?;
    Ok(OutputConsentStatusV1 {
        operation_id: OUTPUT_CONSENT_STATUS_QUERY_OPERATION_ID.to_string(),
        request_id: request.request_id,
        challenge_id: request.challenge_id,
        state: match status.state {
            PreparedConsentState::PendingPhysicalInput => {
                OutputConsentPhysicalStateV1::PendingPhysicalInput
            }
            PreparedConsentState::Ready => OutputConsentPhysicalStateV1::Ready,
        },
        expires_at_unix_ms: status.expires_at_unix_ms,
    })
}

pub(crate) fn execute_output_control(
    app: &tauri::AppHandle,
    window: &WebviewWindow,
    state: &AppState,
    query_state: &ControlPlaneQueryState,
    request: OutputControlCommandRequestV1,
) -> OutputControlResponseV1 {
    if request.validate().is_err() {
        return output_control_rejection(&request, OutputControlErrorCodeV1::InvalidRequest);
    }
    let shape_sha256 = match request.canonical_shape_bytes() {
        Ok(bytes) => hex_sha256(&bytes),
        Err(_) => {
            return output_control_rejection(&request, OutputControlErrorCodeV1::InvalidRequest)
        }
    };
    let argument_fingerprint = match request.argument_fingerprint_bytes() {
        Ok(bytes) => hex_sha256(&bytes),
        Err(_) => {
            return output_control_rejection(&request, OutputControlErrorCodeV1::InvalidRequest)
        }
    };
    let now = Instant::now();
    let _owner_rotation = match state.project_transaction_owner_rotation.lock() {
        Ok(guard) => guard,
        Err(_) => return output_control_rejection(&request, OutputControlErrorCodeV1::Internal),
    };
    let binding = match capture_binding(state, window.label()) {
        Ok(binding) => binding,
        Err(_) => return output_control_rejection(&request, OutputControlErrorCodeV1::Forbidden),
    };
    let key = output_control_receipt_key(&request, &binding);
    let lane =
        match state
            .runtime_control_plane
            .reserve_output_control_lane(&key, &shape_sha256, now)
        {
            OutputControlLaneReservation::Terminal(response) => return response,
            OutputControlLaneReservation::Rejected(code) => {
                return output_control_rejection(&request, code)
            }
            OutputControlLaneReservation::Lane(lane) => lane,
        };
    let _lane = match lane.lock() {
        Ok(guard) => guard,
        Err(_) => {
            state
                .runtime_control_plane
                .release_output_control_lane(&key);
            return output_control_rejection(&request, OutputControlErrorCodeV1::Internal);
        }
    };
    if let Some(reservation) = state.runtime_control_plane.recheck_output_control_terminal(
        &key,
        &shape_sha256,
        Instant::now(),
    ) {
        return match reservation {
            OutputControlLaneReservation::Terminal(response) => response,
            OutputControlLaneReservation::Rejected(code) => {
                output_control_rejection(&request, code)
            }
            OutputControlLaneReservation::Lane(_) => unreachable!(),
        };
    }
    if query_state
        .validate_output_control_fence_window(
            window.label(),
            &request.expected_fence,
            binding.owner_incarnation,
        )
        .is_err()
    {
        state
            .runtime_control_plane
            .release_output_control_lane(&key);
        return output_control_rejection(&request, OutputControlErrorCodeV1::Forbidden);
    }

    let external_admission = match lock_project_external_command_admission(state) {
        Ok(guard) => guard,
        Err(_) => {
            state
                .runtime_control_plane
                .release_output_control_lane(&key);
            return output_control_rejection(&request, OutputControlErrorCodeV1::Busy);
        }
    };
    let mut coordinator = match lock_project_coordinator(state) {
        Ok(coordinator) => coordinator,
        Err(_) => {
            state
                .runtime_control_plane
                .release_output_control_lane(&key);
            return output_control_rejection(&request, OutputControlErrorCodeV1::Busy);
        }
    };
    if reconcile_project_checkpoint_for_coordinator(state, &mut coordinator).is_err()
        || !exact_output_control_fence_matches(state, &coordinator, &request.expected_fence)
        || ensure_no_pending_project_transaction(&coordinator).is_err()
    {
        state
            .runtime_control_plane
            .release_output_control_lane(&key);
        return output_control_rejection(&request, OutputControlErrorCodeV1::StaleFence);
    }
    if ensure_project_operator_video_clip_slot_runtime_allowed(
        state,
        &coordinator,
        &binding.principal,
    )
    .is_err()
    {
        state
            .runtime_control_plane
            .release_output_control_lane(&key);
        return output_control_rejection(&request, OutputControlErrorCodeV1::Forbidden);
    }
    if validate_output_action_current(state, &request.action).is_err() {
        state
            .runtime_control_plane
            .release_output_control_lane(&key);
        return output_control_rejection(&request, OutputControlErrorCodeV1::StaleFence);
    }

    let (inflight, audit_sequence) =
        match state.runtime_control_plane.admit_and_audit_output_control(
            &binding,
            &key,
            &shape_sha256,
            &argument_fingerprint,
            Instant::now(),
        ) {
            Ok(admission) => admission,
            Err(code) => {
                state
                    .runtime_control_plane
                    .release_output_control_lane(&key);
                return output_control_rejection(&request, code);
            }
        };

    let consent_binding = ConsentAuthorityBinding {
        caller: ConsentCallerBinding {
            principal: binding.principal.clone(),
            window_label: binding.window_label.clone(),
            owner_incarnation: binding.owner_incarnation,
        },
        operation_id: request.operation_id.clone(),
        argument_fingerprint: argument_fingerprint.clone(),
        project_epoch: request.expected_fence.project_epoch,
        project_revision: request.expected_fence.project_revision,
        project_checkpoint_hash: request.expected_fence.project_checkpoint_hash.clone(),
        project_publication_generation: request.expected_fence.project_publication_generation,
        output_epoch: request.expected_fence.output_epoch,
        output_generation: request.expected_fence.output_generation,
        safety_blackout_epoch: request.expected_fence.safety_blackout_epoch,
        safety_blackout_generation: request.expected_fence.safety_blackout_generation,
    };
    if let Err(error) = state
        .control_plane_security
        .consume_consent(&consent_binding, &request.consent_token)
    {
        state
            .runtime_control_plane
            .finish_output_control_inflight(&inflight);
        let response = output_control_rejection(&request, output_consent_error_code(error));
        state.runtime_control_plane.store_output_control_terminal(
            key,
            shape_sha256,
            response.clone(),
            Instant::now(),
        );
        return response;
    }

    // The exact fence and consent have now been consumed. Each mutating
    // action re-enters the lifecycle/project admission boundary through its
    // actual transition or publication commit.
    drop(coordinator);
    drop(external_admission);

    let operation_result = match &request.action {
        OutputControlActionV1::ReleaseBlackout => {
            super::release_safety_blackout_with_output_control_fence(state, &request.expected_fence)
        }
        OutputControlActionV1::Arm { role } => {
            let target = match role {
                protocol::control_plane_command::OutputControlTargetRoleV1::Lighting => {
                    protocol::MachineOutputRole::Lighting
                }
                protocol::control_plane_command::OutputControlTargetRoleV1::Video => {
                    protocol::MachineOutputRole::Video
                }
                protocol::control_plane_command::OutputControlTargetRoleV1::Both => {
                    protocol::MachineOutputRole::Both
                }
            };
            super::apply_output_ownership_role_with_output_control_fence(
                app,
                state,
                target,
                &request.expected_fence,
            )
        }
        OutputControlActionV1::TakeOverStandby { force, .. } => {
            standby_takeover_selector(&request.action)
                .and_then(|selector| {
                    super::take_over_standby_core(state, *force, selector, &request.expected_fence)
                })
                .map(|(_load_result, fence_after)| (true, fence_after))
        }
    };

    let (applied, fence_after) = match operation_result {
        Ok(result) => result,
        Err(_) => {
            state
                .runtime_control_plane
                .finish_output_control_inflight(&inflight);
            let response =
                output_control_rejection(&request, OutputControlErrorCodeV1::PublicationFailed);
            state.runtime_control_plane.store_output_control_terminal(
                key,
                shape_sha256,
                response.clone(),
                Instant::now(),
            );
            return response;
        }
    };

    let response = OutputControlResponseV1::Receipt(OutputControlReceiptV1 {
        operation_id: request.operation_id.clone(),
        request_id: request.request_id,
        shape_sha256: shape_sha256.clone(),
        argument_fingerprint,
        audit_sequence,
        fence_before: request.expected_fence.clone(),
        fence_after,
        outcome: if applied {
            OutputControlReceiptOutcomeV1::Applied
        } else {
            OutputControlReceiptOutcomeV1::NoOp
        },
    });
    debug_assert!(response.validate().is_ok());
    state
        .runtime_control_plane
        .finish_output_control_inflight(&inflight);
    state.runtime_control_plane.store_output_control_terminal(
        key,
        shape_sha256,
        response.clone(),
        Instant::now(),
    );
    response
}

pub(crate) fn exact_output_control_fence_matches(
    state: &AppState,
    coordinator: &ProjectCoordinator,
    fence: &OutputControlFenceV1,
) -> bool {
    let output = state.engine.output_ownership_status();
    let safety = state.engine.safety_blackout_authority();
    coordinator.epoch == fence.project_epoch
        && coordinator.revision == fence.project_revision
        && coordinator.checkpoint_hash == fence.project_checkpoint_hash
        && coordinator.publication_generation == fence.project_publication_generation
        && output.epoch.checked_add(1) == Some(fence.output_epoch)
        && output.generation.checked_add(1) == Some(fence.output_generation)
        && safety.epoch == fence.safety_blackout_epoch
        && safety.generation == fence.safety_blackout_generation
}

/// Reserve the externally projected output counters that a successful Arm or
/// project-replacement transition will publish. This must run before the
/// physical action so terminal-fence exhaustion cannot be discovered after a
/// successful commit.
pub(crate) fn preflight_output_control_successor(
    fence: &OutputControlFenceV1,
) -> Result<(u64, u64), String> {
    let output_epoch = fence
        .output_epoch
        .checked_add(1)
        .filter(|value| *value <= MAX_SAFE_JAVASCRIPT_INTEGER)
        .ok_or_else(|| "Output ownership epoch is exhausted".to_string())?;
    let output_generation = fence
        .output_generation
        .checked_add(1)
        .filter(|value| *value <= MAX_SAFE_JAVASCRIPT_INTEGER)
        .ok_or_else(|| "Output ownership generation is exhausted".to_string())?;
    Ok((output_epoch, output_generation))
}

/// Build the terminal fence from values preflighted before action commit and
/// state captured while the coordinator/output transition guards are still
/// held. No fallible lock or allocation occurs after the physical action.
pub(crate) fn committed_output_control_fence(
    basis: &OutputControlFenceV1,
    project_epoch: u64,
    project_revision: u64,
    project_checkpoint_hash: &str,
    project_publication_generation: u64,
    output_epoch: u64,
    output_generation: u64,
    safety_blackout_epoch: u64,
    safety_blackout_generation: u64,
) -> OutputControlFenceV1 {
    let fence = OutputControlFenceV1 {
        process_incarnation: basis.process_incarnation,
        session_incarnation: basis.session_incarnation,
        project_epoch,
        project_revision,
        project_checkpoint_hash: project_checkpoint_hash.to_string(),
        project_publication_generation,
        output_epoch,
        output_generation,
        safety_blackout_epoch,
        safety_blackout_generation,
    };
    debug_assert!(fence.validate().is_ok());
    fence
}

fn validate_output_action_current(
    state: &AppState,
    action: &OutputControlActionV1,
) -> Result<(), String> {
    match action {
        OutputControlActionV1::Arm { .. } | OutputControlActionV1::ReleaseBlackout => Ok(()),
        OutputControlActionV1::TakeOverStandby {
            force,
            standby_session_id,
            standby_generation,
        } => {
            let runtime = state
                .standby_sync
                .lock()
                .map_err(|_| "Standby synchronization lock was poisoned".to_string())?;
            let status = runtime
                .status
                .lock()
                .map_err(|_| "Standby synchronization status lock was poisoned".to_string())?
                .clone();
            super::validate_takeover_running_status(&status)?;
            if status.session_id.as_deref() != Some(standby_session_id.as_str())
                || status.generation != Some(*standby_generation)
            {
                return Err("Standby takeover checkpoint is stale".to_string());
            }
            if (status.split_brain || !status.heartbeat_stale) && !force {
                return Err("Standby takeover requires explicit force confirmation".to_string());
            }
            Ok(())
        }
    }
}

/// Dispatch one operation-specific Tauri ingress into the shared local
/// OutputControl controller. Keeping the operation identity at the wrapper
/// boundary prevents the generic action envelope from becoming a source that
/// ambiguously projects to three different canonical operations.
pub(crate) fn execute_output_control_for_operation(
    app: &tauri::AppHandle,
    window: &WebviewWindow,
    state: &AppState,
    query_state: &ControlPlaneQueryState,
    expected_operation_id: &'static str,
    request: OutputControlCommandRequestV1,
) -> OutputControlResponseV1 {
    if request.operation_id != expected_operation_id
        || request.action.operation_id() != expected_operation_id
    {
        return output_control_rejection_for_operation(
            &request,
            expected_operation_id,
            OutputControlErrorCodeV1::InvalidRequest,
        );
    }
    execute_output_control(app, window, state, query_state, request)
}

fn standby_takeover_selector(
    action: &OutputControlActionV1,
) -> Result<StandbyTakeoverCheckpointSelector, String> {
    match action {
        OutputControlActionV1::TakeOverStandby {
            standby_session_id,
            standby_generation,
            ..
        } => Ok(StandbyTakeoverCheckpointSelector::Exact(
            StandbyCheckpointIdentity {
                session_id: standby_session_id.clone(),
                generation: *standby_generation,
            },
        )),
        _ => Err("Output control action is not a Standby Take Over".to_string()),
    }
}

fn output_consent_error_message(error: ConsentConsumeError) -> String {
    match error {
        ConsentConsumeError::Missing => "Output consent challenge was not found",
        ConsentConsumeError::Expired => "Output consent challenge expired",
        ConsentConsumeError::WrongBinding => "Output consent challenge belongs to another caller",
        ConsentConsumeError::PhysicalInputPending => "Physical confirmation is still pending",
        ConsentConsumeError::DeviceRemoved => "Physical confirmation device was removed",
        ConsentConsumeError::Replayed => "Output consent token was already consumed",
        ConsentConsumeError::Internal => "Output consent state is unavailable",
    }
    .to_string()
}

fn output_consent_error_code(error: ConsentConsumeError) -> OutputControlErrorCodeV1 {
    match error {
        ConsentConsumeError::Missing => OutputControlErrorCodeV1::ConsentMissing,
        ConsentConsumeError::Expired => OutputControlErrorCodeV1::ConsentExpired,
        ConsentConsumeError::WrongBinding => OutputControlErrorCodeV1::ConsentWrongBinding,
        ConsentConsumeError::PhysicalInputPending => OutputControlErrorCodeV1::ConsentPending,
        ConsentConsumeError::DeviceRemoved => OutputControlErrorCodeV1::ConsentDeviceRemoved,
        ConsentConsumeError::Replayed => OutputControlErrorCodeV1::ConsentReplayed,
        ConsentConsumeError::Internal => OutputControlErrorCodeV1::Internal,
    }
}

fn output_control_rejection(
    request: &OutputControlCommandRequestV1,
    error: OutputControlErrorCodeV1,
) -> OutputControlResponseV1 {
    output_control_rejection_for_operation(request, request.action.operation_id(), error)
}

fn output_control_rejection_for_operation(
    request: &OutputControlCommandRequestV1,
    operation_id: &str,
    error: OutputControlErrorCodeV1,
) -> OutputControlResponseV1 {
    OutputControlResponseV1::Rejected(OutputControlRejectionV1 {
        operation_id: operation_id.to_string(),
        request_id: request.request_id.clamp(1, MAX_SAFE_JAVASCRIPT_INTEGER),
        error,
    })
}

fn output_control_receipt_key(
    request: &OutputControlCommandRequestV1,
    binding: &CallerBinding,
) -> OutputControlReceiptKey {
    OutputControlReceiptKey {
        principal: binding.principal.clone(),
        window_label: binding.window_label.clone(),
        owner_incarnation: binding.owner_incarnation,
        operation_id: request.action.operation_id().to_string(),
        request_id: request.request_id,
    }
}

fn hex_sha256(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CallerBinding {
    principal: String,
    window_label: String,
    owner_incarnation: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ReceiptKey {
    process_incarnation: u64,
    session_incarnation: u64,
    principal: String,
    window_label: String,
    owner_incarnation: u64,
    operation_id: String,
    request_id: u64,
    project_epoch: u64,
    project_revision: u64,
    project_checkpoint_hash: String,
    project_publication_generation: u64,
    domain: String,
    source_runtime_epoch: u64,
    source_runtime_generation: u64,
    authority_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct PrincipalDomainKey {
    principal: String,
    domain: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct SafetyReceiptKey {
    principal: String,
    window_label: String,
    owner_incarnation: u64,
    operation_id: String,
    request_id: u64,
}

#[derive(Debug, Clone)]
struct SafetyTerminalRecord {
    shape_sha256: String,
    response: SafetyBlackoutEngageResponseV1,
    expires_at: Instant,
    last_used: u64,
}

#[derive(Debug, Clone)]
struct SafetyTombstoneRecord {
    expires_at: Instant,
    last_used: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SafetyAuditRecord {
    sequence: u64,
    principal: String,
    window_label: String,
    owner_incarnation: u64,
    operation_id: String,
    request_id: u64,
    shape_sha256: String,
}

#[derive(Debug, Default)]
struct SafetyBlackoutControlPlaneInner {
    receipts: HashMap<SafetyReceiptKey, SafetyTerminalRecord>,
    tombstones: HashMap<SafetyReceiptKey, SafetyTombstoneRecord>,
    lanes: HashMap<SafetyReceiptKey, Arc<Mutex<()>>>,
    token_buckets: HashMap<PrincipalDomainKey, TokenBucket>,
    inflight: HashSet<PrincipalDomainKey>,
    audit: Vec<SafetyAuditRecord>,
    sequence: u64,
    audit_sequence: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct OutputControlReceiptKey {
    principal: String,
    window_label: String,
    owner_incarnation: u64,
    operation_id: String,
    request_id: u64,
}

#[derive(Debug, Clone)]
struct OutputControlTerminalRecord {
    shape_sha256: String,
    response: OutputControlResponseV1,
    expires_at: Instant,
    last_used: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct OutputControlAuditRecord {
    sequence: u64,
    principal: String,
    window_label: String,
    owner_incarnation: u64,
    operation_id: String,
    request_id: u64,
    shape_sha256: String,
    argument_fingerprint: String,
}

#[derive(Debug, Default)]
struct OutputControlPlaneInner {
    receipts: HashMap<OutputControlReceiptKey, OutputControlTerminalRecord>,
    tombstones: HashMap<OutputControlReceiptKey, SafetyTombstoneRecord>,
    lanes: HashMap<OutputControlReceiptKey, Arc<Mutex<()>>>,
    token_buckets: HashMap<PrincipalDomainKey, TokenBucket>,
    inflight: HashSet<PrincipalDomainKey>,
    audit: Vec<OutputControlAuditRecord>,
    sequence: u64,
    audit_sequence: u64,
}

#[derive(Debug, Clone)]
struct TerminalRecord {
    shape_sha256: String,
    response: RuntimeCommandResponseV1,
    expires_at: Instant,
    last_used: u64,
}

#[derive(Debug, Clone)]
struct TombstoneRecord {
    expires_at: Instant,
    last_used: u64,
}

#[derive(Debug, Clone)]
struct AuthorityRecord {
    binding: CallerBinding,
    fence: TimelineTransportRuntimeFenceV1,
    expires_at: Instant,
    last_used: u64,
}

#[derive(Debug, Clone)]
struct TokenBucket {
    tokens: f64,
    observed_at: Instant,
}

impl TokenBucket {
    fn fresh(now: Instant) -> Self {
        Self {
            tokens: TOKEN_BUCKET_BURST,
            observed_at: now,
        }
    }

    fn try_take(&mut self, now: Instant) -> bool {
        let elapsed = now
            .saturating_duration_since(self.observed_at)
            .as_secs_f64();
        self.tokens = (self.tokens + elapsed * TOKEN_BUCKET_PER_SECOND).min(TOKEN_BUCKET_BURST);
        self.observed_at = now;
        if self.tokens < 1.0 {
            return false;
        }
        self.tokens -= 1.0;
        true
    }
}

#[derive(Debug, Default)]
struct RuntimeControlPlaneInner {
    receipts: HashMap<ReceiptKey, TerminalRecord>,
    tombstones: HashMap<ReceiptKey, TombstoneRecord>,
    lanes: HashMap<ReceiptKey, Arc<Mutex<()>>>,
    authorities: HashMap<String, AuthorityRecord>,
    token_buckets: HashMap<PrincipalDomainKey, TokenBucket>,
    inflight: HashSet<PrincipalDomainKey>,
    sequence: u64,
}

#[derive(Debug, Clone)]
struct FollowAbortTerminalRecord {
    shape_sha256: String,
    response: TimelineFollowAbortRuntimeResponseV1,
    expires_at: Instant,
    last_used: u64,
}

#[derive(Debug, Clone)]
struct FollowAbortAuthorityRecord {
    binding: CallerBinding,
    fence: TimelineFollowAbortRuntimeFenceV1,
    expires_at: Instant,
    last_used: u64,
}

#[derive(Debug, Default)]
struct FollowAbortControlPlaneInner {
    receipts: HashMap<ReceiptKey, FollowAbortTerminalRecord>,
    tombstones: HashMap<ReceiptKey, TombstoneRecord>,
    lanes: HashMap<ReceiptKey, Arc<Mutex<()>>>,
    authorities: HashMap<String, FollowAbortAuthorityRecord>,
    token_buckets: HashMap<PrincipalDomainKey, TokenBucket>,
    inflight: HashSet<PrincipalDomainKey>,
    sequence: u64,
}

/// Bounded server-side state for the Timeline runtime terminal lane. It owns
/// no project bytes, engine state, output role, or caller-supplied identity.
pub(crate) struct RuntimeControlPlaneState {
    inner: Mutex<RuntimeControlPlaneInner>,
    follow_abort: Mutex<FollowAbortControlPlaneInner>,
    safety_blackout: Mutex<SafetyBlackoutControlPlaneInner>,
    output_control: Mutex<OutputControlPlaneInner>,
}

impl Default for RuntimeControlPlaneState {
    fn default() -> Self {
        Self {
            inner: Mutex::new(RuntimeControlPlaneInner::default()),
            follow_abort: Mutex::new(FollowAbortControlPlaneInner::default()),
            safety_blackout: Mutex::new(SafetyBlackoutControlPlaneInner::default()),
            output_control: Mutex::new(OutputControlPlaneInner::default()),
        }
    }
}

enum LaneReservation {
    Terminal(RuntimeCommandResponseV1),
    Rejected(RuntimeCommandErrorCodeV1),
    Lane(Arc<Mutex<()>>),
}

enum FollowAbortLaneReservation {
    Terminal(TimelineFollowAbortRuntimeResponseV1),
    Rejected(RuntimeCommandErrorCodeV1),
    Lane(Arc<Mutex<()>>),
}

enum SafetyBlackoutLaneReservation {
    Terminal(SafetyBlackoutEngageResponseV1),
    Rejected(RuntimeCommandErrorCodeV1),
    Lane(Arc<Mutex<()>>),
}

enum OutputControlLaneReservation {
    Terminal(OutputControlResponseV1),
    Rejected(OutputControlErrorCodeV1),
    Lane(Arc<Mutex<()>>),
}

impl RuntimeControlPlaneState {
    fn issue_authority(
        &self,
        binding: CallerBinding,
        fence: TimelineTransportRuntimeFenceV1,
        authority_id: String,
        now: Instant,
    ) -> Result<(), RuntimeCommandErrorCodeV1> {
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| RuntimeCommandErrorCodeV1::Internal)?;
        purge_expired(&mut inner, now);
        // Never overwrite a previously issued opaque id. Each issuance has a
        // distinct single-use/TTL lifecycle even if it carries the same fence.
        if inner.authorities.contains_key(&authority_id) {
            return Err(RuntimeCommandErrorCodeV1::Conflict);
        }
        while inner.authorities.len() >= MAX_TOTAL_AUTHORITIES {
            let oldest = inner
                .authorities
                .iter()
                .min_by_key(|(_, record)| record.last_used)
                .map(|(key, _)| key.clone());
            if let Some(oldest) = oldest {
                inner.authorities.remove(&oldest);
            } else {
                return Err(RuntimeCommandErrorCodeV1::Overloaded);
            }
        }
        let last_used = next_sequence(&mut inner);
        inner.authorities.insert(
            authority_id,
            AuthorityRecord {
                binding,
                fence,
                expires_at: now + AUTHORITY_TTL,
                last_used,
            },
        );
        Ok(())
    }

    fn reserve_lane(&self, key: &ReceiptKey, shape_sha256: &str, now: Instant) -> LaneReservation {
        let mut inner = match self.inner.lock() {
            Ok(inner) => inner,
            Err(_) => return LaneReservation::Rejected(RuntimeCommandErrorCodeV1::Internal),
        };
        purge_expired(&mut inner, now);
        let last_used = next_sequence(&mut inner);
        if let Some(record) = inner.receipts.get_mut(key) {
            record.last_used = last_used;
            return if record.shape_sha256 == shape_sha256 {
                LaneReservation::Terminal(record.response.clone())
            } else {
                LaneReservation::Rejected(RuntimeCommandErrorCodeV1::Conflict)
            };
        }
        if let Some(tombstone) = inner.tombstones.get_mut(key) {
            tombstone.last_used = last_used;
            return LaneReservation::Rejected(RuntimeCommandErrorCodeV1::Conflict);
        }
        if let Some(lane) = inner.lanes.get(key) {
            return LaneReservation::Lane(Arc::clone(lane));
        }
        if inner.lanes.len() >= MAX_LANES {
            return LaneReservation::Rejected(RuntimeCommandErrorCodeV1::Overloaded);
        }
        let lane = Arc::new(Mutex::new(()));
        inner.lanes.insert(key.clone(), Arc::clone(&lane));
        LaneReservation::Lane(lane)
    }

    fn recheck_terminal(
        &self,
        key: &ReceiptKey,
        shape_sha256: &str,
        now: Instant,
    ) -> Option<LaneReservation> {
        let mut inner = self.inner.lock().ok()?;
        purge_expired(&mut inner, now);
        let last_used = next_sequence(&mut inner);
        if let Some(record) = inner.receipts.get_mut(key) {
            record.last_used = last_used;
            return Some(if record.shape_sha256 == shape_sha256 {
                LaneReservation::Terminal(record.response.clone())
            } else {
                LaneReservation::Rejected(RuntimeCommandErrorCodeV1::Conflict)
            });
        }
        if let Some(tombstone) = inner.tombstones.get_mut(key) {
            tombstone.last_used = last_used;
            return Some(LaneReservation::Rejected(
                RuntimeCommandErrorCodeV1::Conflict,
            ));
        }
        None
    }

    /// Consume the single-use authority only after all app preflight checks
    /// have succeeded, then claim one in-flight slot and one rate token.
    fn admit_new(
        &self,
        binding: &CallerBinding,
        fence: &TimelineTransportRuntimeFenceV1,
        authority_id: &str,
        key: &ReceiptKey,
        now: Instant,
    ) -> Result<PrincipalDomainKey, RuntimeCommandErrorCodeV1> {
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| RuntimeCommandErrorCodeV1::Internal)?;
        purge_expired(&mut inner, now);
        let Some(authority) = inner.authorities.get(authority_id) else {
            return Err(RuntimeCommandErrorCodeV1::Forbidden);
        };
        if authority.binding != *binding || authority.fence != *fence {
            return Err(RuntimeCommandErrorCodeV1::Forbidden);
        }
        if authority.expires_at <= now {
            inner.authorities.remove(authority_id);
            return Err(RuntimeCommandErrorCodeV1::StaleFence);
        }
        let last_used = next_sequence(&mut inner);
        if let Some(authority) = inner.authorities.get_mut(authority_id) {
            authority.last_used = last_used;
        }
        let domain_key = PrincipalDomainKey {
            principal: binding.principal.clone(),
            domain: fence.domain.clone(),
        };
        if inner.inflight.contains(&domain_key) {
            return Err(RuntimeCommandErrorCodeV1::Busy);
        }
        let bucket = inner
            .token_buckets
            .entry(domain_key.clone())
            .or_insert_with(|| TokenBucket::fresh(now));
        if !bucket.try_take(now) {
            return Err(RuntimeCommandErrorCodeV1::Overloaded);
        }
        // An authority is consumed precisely at successful new-request
        // admission. The lane prevents a duplicate from consuming another
        // token or publishing a second engine command.
        inner.authorities.remove(authority_id);
        inner.inflight.insert(domain_key.clone());
        // Keep this explicit even though `key` has already been lane-reserved:
        // it documents the receipt identity which owns the admission.
        debug_assert!(inner.lanes.contains_key(key));
        Ok(domain_key)
    }

    fn finish_inflight(&self, key: &PrincipalDomainKey) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.inflight.remove(key);
        }
    }

    fn store_terminal(
        &self,
        key: ReceiptKey,
        shape_sha256: String,
        response: RuntimeCommandResponseV1,
        now: Instant,
    ) {
        let Ok(mut inner) = self.inner.lock() else {
            return;
        };
        purge_expired(&mut inner, now);
        enforce_receipt_capacity(&mut inner, &key, now);
        let last_used = next_sequence(&mut inner);
        inner.receipts.insert(
            key.clone(),
            TerminalRecord {
                shape_sha256,
                response,
                expires_at: now + RECEIPT_TTL,
                last_used,
            },
        );
        inner.lanes.remove(&key);
    }

    fn release_lane(&self, key: &ReceiptKey) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.lanes.remove(key);
        }
    }

    fn issue_follow_abort_authority(
        &self,
        binding: CallerBinding,
        fence: TimelineFollowAbortRuntimeFenceV1,
        authority_id: String,
        now: Instant,
    ) -> Result<(), RuntimeCommandErrorCodeV1> {
        let mut inner = self
            .follow_abort
            .lock()
            .map_err(|_| RuntimeCommandErrorCodeV1::Internal)?;
        purge_follow_abort_expired(&mut inner, now);
        if inner.authorities.contains_key(&authority_id) {
            return Err(RuntimeCommandErrorCodeV1::Conflict);
        }
        while inner.authorities.len() >= MAX_TOTAL_AUTHORITIES {
            let oldest = inner
                .authorities
                .iter()
                .min_by_key(|(_, record)| record.last_used)
                .map(|(key, _)| key.clone());
            if let Some(oldest) = oldest {
                inner.authorities.remove(&oldest);
            } else {
                return Err(RuntimeCommandErrorCodeV1::Overloaded);
            }
        }
        let last_used = next_follow_abort_sequence(&mut inner);
        inner.authorities.insert(
            authority_id,
            FollowAbortAuthorityRecord {
                binding,
                fence,
                expires_at: now + AUTHORITY_TTL,
                last_used,
            },
        );
        Ok(())
    }

    fn reserve_follow_abort_lane(
        &self,
        key: &ReceiptKey,
        shape_sha256: &str,
        now: Instant,
    ) -> FollowAbortLaneReservation {
        let mut inner = match self.follow_abort.lock() {
            Ok(inner) => inner,
            Err(_) => {
                return FollowAbortLaneReservation::Rejected(RuntimeCommandErrorCodeV1::Internal)
            }
        };
        purge_follow_abort_expired(&mut inner, now);
        let last_used = next_follow_abort_sequence(&mut inner);
        if let Some(record) = inner.receipts.get_mut(key) {
            record.last_used = last_used;
            return if record.shape_sha256 == shape_sha256 {
                FollowAbortLaneReservation::Terminal(record.response.clone())
            } else {
                FollowAbortLaneReservation::Rejected(RuntimeCommandErrorCodeV1::Conflict)
            };
        }
        if let Some(tombstone) = inner.tombstones.get_mut(key) {
            tombstone.last_used = last_used;
            return FollowAbortLaneReservation::Rejected(RuntimeCommandErrorCodeV1::Conflict);
        }
        if let Some(lane) = inner.lanes.get(key) {
            return FollowAbortLaneReservation::Lane(Arc::clone(lane));
        }
        if inner.lanes.len() >= MAX_LANES {
            return FollowAbortLaneReservation::Rejected(RuntimeCommandErrorCodeV1::Overloaded);
        }
        let lane = Arc::new(Mutex::new(()));
        inner.lanes.insert(key.clone(), Arc::clone(&lane));
        FollowAbortLaneReservation::Lane(lane)
    }

    fn recheck_follow_abort_terminal(
        &self,
        key: &ReceiptKey,
        shape_sha256: &str,
        now: Instant,
    ) -> Option<FollowAbortLaneReservation> {
        let mut inner = self.follow_abort.lock().ok()?;
        purge_follow_abort_expired(&mut inner, now);
        let last_used = next_follow_abort_sequence(&mut inner);
        if let Some(record) = inner.receipts.get_mut(key) {
            record.last_used = last_used;
            return Some(if record.shape_sha256 == shape_sha256 {
                FollowAbortLaneReservation::Terminal(record.response.clone())
            } else {
                FollowAbortLaneReservation::Rejected(RuntimeCommandErrorCodeV1::Conflict)
            });
        }
        if let Some(tombstone) = inner.tombstones.get_mut(key) {
            tombstone.last_used = last_used;
            return Some(FollowAbortLaneReservation::Rejected(
                RuntimeCommandErrorCodeV1::Conflict,
            ));
        }
        None
    }

    fn admit_follow_abort_new(
        &self,
        binding: &CallerBinding,
        fence: &TimelineFollowAbortRuntimeFenceV1,
        authority_id: &str,
        key: &ReceiptKey,
        now: Instant,
    ) -> Result<PrincipalDomainKey, RuntimeCommandErrorCodeV1> {
        let mut inner = self
            .follow_abort
            .lock()
            .map_err(|_| RuntimeCommandErrorCodeV1::Internal)?;
        purge_follow_abort_expired(&mut inner, now);
        let Some(authority) = inner.authorities.get(authority_id) else {
            return Err(RuntimeCommandErrorCodeV1::Forbidden);
        };
        if authority.binding != *binding || authority.fence != *fence {
            return Err(RuntimeCommandErrorCodeV1::Forbidden);
        }
        if authority.expires_at <= now {
            inner.authorities.remove(authority_id);
            return Err(RuntimeCommandErrorCodeV1::StaleFence);
        }
        let domain_key = PrincipalDomainKey {
            principal: binding.principal.clone(),
            domain: fence.domain.clone(),
        };
        if inner.inflight.contains(&domain_key) {
            return Err(RuntimeCommandErrorCodeV1::Busy);
        }
        let bucket = inner
            .token_buckets
            .entry(domain_key.clone())
            .or_insert_with(|| TokenBucket::fresh(now));
        if !bucket.try_take(now) {
            return Err(RuntimeCommandErrorCodeV1::Overloaded);
        }
        inner.authorities.remove(authority_id);
        inner.inflight.insert(domain_key.clone());
        debug_assert!(inner.lanes.contains_key(key));
        Ok(domain_key)
    }

    fn finish_follow_abort_inflight(&self, key: &PrincipalDomainKey) {
        if let Ok(mut inner) = self.follow_abort.lock() {
            inner.inflight.remove(key);
        }
    }

    fn store_follow_abort_terminal(
        &self,
        key: ReceiptKey,
        shape_sha256: String,
        response: TimelineFollowAbortRuntimeResponseV1,
        now: Instant,
    ) {
        let Ok(mut inner) = self.follow_abort.lock() else {
            return;
        };
        purge_follow_abort_expired(&mut inner, now);
        enforce_follow_abort_receipt_capacity(&mut inner, &key, now);
        let last_used = next_follow_abort_sequence(&mut inner);
        inner.receipts.insert(
            key.clone(),
            FollowAbortTerminalRecord {
                shape_sha256,
                response,
                expires_at: now + RECEIPT_TTL,
                last_used,
            },
        );
        inner.lanes.remove(&key);
    }

    fn release_follow_abort_lane(&self, key: &ReceiptKey) {
        if let Ok(mut inner) = self.follow_abort.lock() {
            inner.lanes.remove(key);
        }
    }

    fn reserve_safety_blackout_lane(
        &self,
        key: &SafetyReceiptKey,
        shape_sha256: &str,
        now: Instant,
    ) -> SafetyBlackoutLaneReservation {
        let mut inner = match self.safety_blackout.lock() {
            Ok(inner) => inner,
            Err(_) => {
                return SafetyBlackoutLaneReservation::Rejected(RuntimeCommandErrorCodeV1::Internal)
            }
        };
        purge_safety_blackout_expired(&mut inner, now);
        let last_used = next_safety_blackout_sequence(&mut inner);
        if let Some(record) = inner.receipts.get_mut(key) {
            record.last_used = last_used;
            return if record.shape_sha256 == shape_sha256 {
                SafetyBlackoutLaneReservation::Terminal(record.response.clone())
            } else {
                SafetyBlackoutLaneReservation::Rejected(RuntimeCommandErrorCodeV1::Conflict)
            };
        }
        if let Some(tombstone) = inner.tombstones.get_mut(key) {
            tombstone.last_used = last_used;
            return SafetyBlackoutLaneReservation::Rejected(RuntimeCommandErrorCodeV1::Conflict);
        }
        if let Some(lane) = inner.lanes.get(key) {
            return SafetyBlackoutLaneReservation::Lane(Arc::clone(lane));
        }
        if inner.lanes.len() >= MAX_LANES {
            return SafetyBlackoutLaneReservation::Rejected(RuntimeCommandErrorCodeV1::Overloaded);
        }
        let lane = Arc::new(Mutex::new(()));
        inner.lanes.insert(key.clone(), Arc::clone(&lane));
        SafetyBlackoutLaneReservation::Lane(lane)
    }

    fn recheck_safety_blackout_terminal(
        &self,
        key: &SafetyReceiptKey,
        shape_sha256: &str,
        now: Instant,
    ) -> Option<SafetyBlackoutLaneReservation> {
        let mut inner = self.safety_blackout.lock().ok()?;
        purge_safety_blackout_expired(&mut inner, now);
        let last_used = next_safety_blackout_sequence(&mut inner);
        if let Some(record) = inner.receipts.get_mut(key) {
            record.last_used = last_used;
            return Some(if record.shape_sha256 == shape_sha256 {
                SafetyBlackoutLaneReservation::Terminal(record.response.clone())
            } else {
                SafetyBlackoutLaneReservation::Rejected(RuntimeCommandErrorCodeV1::Conflict)
            });
        }
        if let Some(tombstone) = inner.tombstones.get_mut(key) {
            tombstone.last_used = last_used;
            return Some(SafetyBlackoutLaneReservation::Rejected(
                RuntimeCommandErrorCodeV1::Conflict,
            ));
        }
        None
    }

    /// Atomically consume one S0 rate token, claim the principal's single
    /// in-flight safety slot, and append the immutable accepted-attempt audit
    /// record before the engine command is sent.
    fn admit_and_audit_safety_blackout(
        &self,
        binding: &CallerBinding,
        key: &SafetyReceiptKey,
        shape_sha256: &str,
        now: Instant,
    ) -> Result<(PrincipalDomainKey, u64), RuntimeCommandErrorCodeV1> {
        let mut inner = self
            .safety_blackout
            .lock()
            .map_err(|_| RuntimeCommandErrorCodeV1::Internal)?;
        purge_safety_blackout_expired(&mut inner, now);
        if inner.audit.len() >= MAX_SAFETY_AUDIT_RECORDS {
            return Err(RuntimeCommandErrorCodeV1::Overloaded);
        }
        let domain_key = PrincipalDomainKey {
            principal: binding.principal.clone(),
            domain: SAFETY_BLACKOUT_DOMAIN.to_string(),
        };
        if inner.inflight.contains(&domain_key) {
            return Err(RuntimeCommandErrorCodeV1::Busy);
        }
        let bucket = inner
            .token_buckets
            .entry(domain_key.clone())
            .or_insert_with(|| TokenBucket::fresh(now));
        if !bucket.try_take(now) {
            return Err(RuntimeCommandErrorCodeV1::Overloaded);
        }
        let audit_sequence = inner
            .audit_sequence
            .checked_add(1)
            .filter(|sequence| *sequence <= MAX_SAFE_JAVASCRIPT_INTEGER)
            .ok_or(RuntimeCommandErrorCodeV1::Overloaded)?;
        inner.audit_sequence = audit_sequence;
        inner.audit.push(SafetyAuditRecord {
            sequence: audit_sequence,
            principal: binding.principal.clone(),
            window_label: binding.window_label.clone(),
            owner_incarnation: binding.owner_incarnation,
            operation_id: key.operation_id.clone(),
            request_id: key.request_id,
            shape_sha256: shape_sha256.to_string(),
        });
        inner.inflight.insert(domain_key.clone());
        debug_assert!(inner.lanes.contains_key(key));
        Ok((domain_key, audit_sequence))
    }

    fn finish_safety_blackout_inflight(&self, key: &PrincipalDomainKey) {
        if let Ok(mut inner) = self.safety_blackout.lock() {
            inner.inflight.remove(key);
        }
    }

    fn store_safety_blackout_terminal(
        &self,
        key: SafetyReceiptKey,
        shape_sha256: String,
        response: SafetyBlackoutEngageResponseV1,
        now: Instant,
    ) -> Result<(), RuntimeCommandErrorCodeV1> {
        let mut inner = self
            .safety_blackout
            .lock()
            .map_err(|_| RuntimeCommandErrorCodeV1::Internal)?;
        purge_safety_blackout_expired(&mut inner, now);
        enforce_safety_blackout_receipt_capacity(&mut inner, &key, now);
        let last_used = next_safety_blackout_sequence(&mut inner);
        inner.receipts.insert(
            key.clone(),
            SafetyTerminalRecord {
                shape_sha256,
                response,
                expires_at: now + RECEIPT_TTL,
                last_used,
            },
        );
        inner.lanes.remove(&key);
        Ok(())
    }

    fn release_safety_blackout_lane(&self, key: &SafetyReceiptKey) {
        if let Ok(mut inner) = self.safety_blackout.lock() {
            inner.lanes.remove(key);
        }
    }

    fn reserve_output_control_lane(
        &self,
        key: &OutputControlReceiptKey,
        shape_sha256: &str,
        now: Instant,
    ) -> OutputControlLaneReservation {
        let mut inner = self
            .output_control
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        purge_output_control_expired(&mut inner, now);
        let last_used = next_output_control_sequence(&mut inner);
        if let Some(record) = inner.receipts.get_mut(key) {
            record.last_used = last_used;
            return if record.shape_sha256 == shape_sha256 {
                OutputControlLaneReservation::Terminal(record.response.clone())
            } else {
                OutputControlLaneReservation::Rejected(OutputControlErrorCodeV1::InvalidRequest)
            };
        }
        if let Some(tombstone) = inner.tombstones.get_mut(key) {
            tombstone.last_used = last_used;
            return OutputControlLaneReservation::Rejected(
                OutputControlErrorCodeV1::InvalidRequest,
            );
        }
        if let Some(lane) = inner.lanes.get(key) {
            return OutputControlLaneReservation::Lane(Arc::clone(lane));
        }
        if inner.lanes.len() >= MAX_LANES {
            return OutputControlLaneReservation::Rejected(OutputControlErrorCodeV1::Overloaded);
        }
        let lane = Arc::new(Mutex::new(()));
        inner.lanes.insert(key.clone(), Arc::clone(&lane));
        OutputControlLaneReservation::Lane(lane)
    }

    fn recheck_output_control_terminal(
        &self,
        key: &OutputControlReceiptKey,
        shape_sha256: &str,
        now: Instant,
    ) -> Option<OutputControlLaneReservation> {
        let mut inner = self
            .output_control
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        purge_output_control_expired(&mut inner, now);
        let last_used = next_output_control_sequence(&mut inner);
        if let Some(record) = inner.receipts.get_mut(key) {
            record.last_used = last_used;
            return Some(if record.shape_sha256 == shape_sha256 {
                OutputControlLaneReservation::Terminal(record.response.clone())
            } else {
                OutputControlLaneReservation::Rejected(OutputControlErrorCodeV1::InvalidRequest)
            });
        }
        if let Some(tombstone) = inner.tombstones.get_mut(key) {
            tombstone.last_used = last_used;
            return Some(OutputControlLaneReservation::Rejected(
                OutputControlErrorCodeV1::InvalidRequest,
            ));
        }
        None
    }

    fn admit_and_audit_output_control(
        &self,
        binding: &CallerBinding,
        key: &OutputControlReceiptKey,
        shape_sha256: &str,
        argument_fingerprint: &str,
        now: Instant,
    ) -> Result<(PrincipalDomainKey, u64), OutputControlErrorCodeV1> {
        let mut inner = self
            .output_control
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        purge_output_control_expired(&mut inner, now);
        if inner.audit.len() >= MAX_SAFETY_AUDIT_RECORDS {
            return Err(OutputControlErrorCodeV1::Overloaded);
        }
        let domain_key = PrincipalDomainKey {
            principal: binding.principal.clone(),
            domain: key.operation_id.clone(),
        };
        if inner.inflight.contains(&domain_key) {
            return Err(OutputControlErrorCodeV1::Busy);
        }
        let bucket = inner
            .token_buckets
            .entry(domain_key.clone())
            .or_insert_with(|| TokenBucket::fresh(now));
        if !bucket.try_take(now) {
            return Err(OutputControlErrorCodeV1::Overloaded);
        }
        let audit_sequence = inner
            .audit_sequence
            .checked_add(1)
            .filter(|sequence| *sequence <= MAX_SAFE_JAVASCRIPT_INTEGER)
            .ok_or(OutputControlErrorCodeV1::Overloaded)?;
        inner.audit_sequence = audit_sequence;
        inner.audit.push(OutputControlAuditRecord {
            sequence: audit_sequence,
            principal: binding.principal.clone(),
            window_label: binding.window_label.clone(),
            owner_incarnation: binding.owner_incarnation,
            operation_id: key.operation_id.clone(),
            request_id: key.request_id,
            shape_sha256: shape_sha256.to_string(),
            argument_fingerprint: argument_fingerprint.to_string(),
        });
        inner.inflight.insert(domain_key.clone());
        Ok((domain_key, audit_sequence))
    }

    fn finish_output_control_inflight(&self, key: &PrincipalDomainKey) {
        let mut inner = self
            .output_control
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        inner.inflight.remove(key);
    }

    fn store_output_control_terminal(
        &self,
        key: OutputControlReceiptKey,
        shape_sha256: String,
        response: OutputControlResponseV1,
        now: Instant,
    ) {
        let mut inner = self
            .output_control
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        purge_output_control_expired(&mut inner, now);
        enforce_output_control_receipt_capacity(&mut inner, &key, now);
        let last_used = next_output_control_sequence(&mut inner);
        inner.receipts.insert(
            key.clone(),
            OutputControlTerminalRecord {
                shape_sha256,
                response,
                expires_at: now + RECEIPT_TTL,
                last_used,
            },
        );
        inner.lanes.remove(&key);
    }

    fn release_output_control_lane(&self, key: &OutputControlReceiptKey) {
        let mut inner = self
            .output_control
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        inner.lanes.remove(key);
    }

    /// Retire all server records for a renderer principal before its owner
    /// string can be reused. Tombstones preserve fail-closed behavior for a
    /// delayed exact key that races the ABA rotation.
    pub(crate) fn retire_principal(&self, principal: &str) {
        let now = Instant::now();
        let Ok(mut inner) = self.inner.lock() else {
            return;
        };
        purge_expired(&mut inner, now);
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
            insert_tombstone(&mut inner, key, now);
        }
        inner
            .authorities
            .retain(|_, record| record.binding.principal != principal);
        inner.inflight.retain(|key| key.principal != principal);
        inner
            .token_buckets
            .retain(|key, _| key.principal != principal);

        drop(inner);
        let Ok(mut follow_abort) = self.follow_abort.lock() else {
            return;
        };
        purge_follow_abort_expired(&mut follow_abort, now);
        let keys = follow_abort
            .receipts
            .keys()
            .chain(follow_abort.lanes.keys())
            .filter(|key| key.principal == principal)
            .cloned()
            .collect::<HashSet<_>>();
        for key in keys {
            follow_abort.receipts.remove(&key);
            follow_abort.lanes.remove(&key);
            insert_follow_abort_tombstone(&mut follow_abort, key, now);
        }
        follow_abort
            .authorities
            .retain(|_, record| record.binding.principal != principal);
        follow_abort
            .inflight
            .retain(|key| key.principal != principal);
        follow_abort
            .token_buckets
            .retain(|key, _| key.principal != principal);

        drop(follow_abort);
        let Ok(mut safety_blackout) = self.safety_blackout.lock() else {
            return;
        };
        purge_safety_blackout_expired(&mut safety_blackout, now);
        let keys = safety_blackout
            .receipts
            .keys()
            .chain(safety_blackout.lanes.keys())
            .filter(|key| key.principal == principal)
            .cloned()
            .collect::<HashSet<_>>();
        for key in keys {
            safety_blackout.receipts.remove(&key);
            safety_blackout.lanes.remove(&key);
            insert_safety_blackout_tombstone(&mut safety_blackout, key, now);
        }
        safety_blackout
            .inflight
            .retain(|key| key.principal != principal);
        safety_blackout
            .token_buckets
            .retain(|key, _| key.principal != principal);

        drop(safety_blackout);
        let mut output_control = self
            .output_control
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        purge_output_control_expired(&mut output_control, now);
        let keys = output_control
            .receipts
            .keys()
            .chain(output_control.lanes.keys())
            .filter(|key| key.principal == principal)
            .cloned()
            .collect::<HashSet<_>>();
        for key in keys {
            output_control.receipts.remove(&key);
            output_control.lanes.remove(&key);
            insert_output_control_tombstone(&mut output_control, key, now);
        }
        output_control
            .inflight
            .retain(|key| key.principal != principal);
        output_control
            .token_buckets
            .retain(|key, _| key.principal != principal);
    }
}

fn next_sequence(inner: &mut RuntimeControlPlaneInner) -> u64 {
    inner.sequence = inner.sequence.wrapping_add(1);
    inner.sequence
}

fn purge_expired(inner: &mut RuntimeControlPlaneInner, now: Instant) {
    let expired_receipts = inner
        .receipts
        .iter()
        .filter_map(|(key, record)| (record.expires_at <= now).then_some(key.clone()))
        .collect::<Vec<_>>();
    for key in expired_receipts {
        inner.receipts.remove(&key);
        inner.lanes.remove(&key);
        insert_tombstone(inner, key, now);
    }
    inner.tombstones.retain(|_, record| record.expires_at > now);
}

fn enforce_receipt_capacity(inner: &mut RuntimeControlPlaneInner, key: &ReceiptKey, now: Instant) {
    while inner.receipts.len() >= MAX_TOTAL_RECEIPTS && !inner.receipts.contains_key(key) {
        if !evict_oldest_receipt(inner, |_| true, now) {
            break;
        }
    }
    while inner
        .receipts
        .keys()
        .filter(|candidate| {
            candidate.principal == key.principal && candidate.operation_id == key.operation_id
        })
        .count()
        >= MAX_RECEIPTS_PER_PRINCIPAL_OPERATION
        && !inner.receipts.contains_key(key)
    {
        if !evict_oldest_receipt(
            inner,
            |candidate| {
                candidate.principal == key.principal && candidate.operation_id == key.operation_id
            },
            now,
        ) {
            break;
        }
    }
}

fn evict_oldest_receipt(
    inner: &mut RuntimeControlPlaneInner,
    predicate: impl Fn(&ReceiptKey) -> bool,
    now: Instant,
) -> bool {
    let oldest = inner
        .receipts
        .iter()
        .filter(|(key, _)| predicate(key))
        .min_by_key(|(_, record)| record.last_used)
        .map(|(key, _)| key.clone());
    let Some(oldest) = oldest else {
        return false;
    };
    inner.receipts.remove(&oldest);
    insert_tombstone(inner, oldest, now);
    true
}

fn insert_tombstone(inner: &mut RuntimeControlPlaneInner, key: ReceiptKey, now: Instant) {
    while inner.tombstones.len() >= MAX_TOTAL_TOMBSTONES && !inner.tombstones.contains_key(&key) {
        let oldest = inner
            .tombstones
            .iter()
            .min_by_key(|(_, record)| record.last_used)
            .map(|(key, _)| key.clone());
        if let Some(oldest) = oldest {
            inner.tombstones.remove(&oldest);
        } else {
            break;
        }
    }
    let last_used = next_sequence(inner);
    inner.tombstones.insert(
        key,
        TombstoneRecord {
            expires_at: now + TOMBSTONE_TTL,
            last_used,
        },
    );
}

fn next_follow_abort_sequence(inner: &mut FollowAbortControlPlaneInner) -> u64 {
    inner.sequence = inner.sequence.wrapping_add(1);
    inner.sequence
}

fn purge_follow_abort_expired(inner: &mut FollowAbortControlPlaneInner, now: Instant) {
    let expired_receipts = inner
        .receipts
        .iter()
        .filter_map(|(key, record)| (record.expires_at <= now).then_some(key.clone()))
        .collect::<Vec<_>>();
    for key in expired_receipts {
        inner.receipts.remove(&key);
        inner.lanes.remove(&key);
        insert_follow_abort_tombstone(inner, key, now);
    }
    inner.tombstones.retain(|_, record| record.expires_at > now);
}

fn enforce_follow_abort_receipt_capacity(
    inner: &mut FollowAbortControlPlaneInner,
    key: &ReceiptKey,
    now: Instant,
) {
    while inner.receipts.len() >= MAX_TOTAL_RECEIPTS && !inner.receipts.contains_key(key) {
        if !evict_oldest_follow_abort_receipt(inner, |_| true, now) {
            break;
        }
    }
    while inner
        .receipts
        .keys()
        .filter(|candidate| {
            candidate.principal == key.principal && candidate.operation_id == key.operation_id
        })
        .count()
        >= MAX_RECEIPTS_PER_PRINCIPAL_OPERATION
        && !inner.receipts.contains_key(key)
    {
        if !evict_oldest_follow_abort_receipt(
            inner,
            |candidate| {
                candidate.principal == key.principal && candidate.operation_id == key.operation_id
            },
            now,
        ) {
            break;
        }
    }
}

fn evict_oldest_follow_abort_receipt(
    inner: &mut FollowAbortControlPlaneInner,
    predicate: impl Fn(&ReceiptKey) -> bool,
    now: Instant,
) -> bool {
    let oldest = inner
        .receipts
        .iter()
        .filter(|(key, _)| predicate(key))
        .min_by_key(|(_, record)| record.last_used)
        .map(|(key, _)| key.clone());
    let Some(oldest) = oldest else {
        return false;
    };
    inner.receipts.remove(&oldest);
    inner.lanes.remove(&oldest);
    insert_follow_abort_tombstone(inner, oldest, now);
    true
}

fn insert_follow_abort_tombstone(
    inner: &mut FollowAbortControlPlaneInner,
    key: ReceiptKey,
    now: Instant,
) {
    while inner.tombstones.len() >= MAX_TOTAL_TOMBSTONES && !inner.tombstones.contains_key(&key) {
        let oldest = inner
            .tombstones
            .iter()
            .min_by_key(|(_, record)| record.last_used)
            .map(|(key, _)| key.clone());
        if let Some(oldest) = oldest {
            inner.tombstones.remove(&oldest);
        } else {
            break;
        }
    }
    let last_used = next_follow_abort_sequence(inner);
    inner.tombstones.insert(
        key,
        TombstoneRecord {
            expires_at: now + TOMBSTONE_TTL,
            last_used,
        },
    );
}

fn next_safety_blackout_sequence(inner: &mut SafetyBlackoutControlPlaneInner) -> u64 {
    inner.sequence = inner.sequence.saturating_add(1);
    inner.sequence
}

fn purge_safety_blackout_expired(inner: &mut SafetyBlackoutControlPlaneInner, now: Instant) {
    let expired = inner
        .receipts
        .iter()
        .filter_map(|(key, record)| (record.expires_at <= now).then_some(key.clone()))
        .collect::<Vec<_>>();
    for key in expired {
        inner.receipts.remove(&key);
        inner.lanes.remove(&key);
        insert_safety_blackout_tombstone(inner, key, now);
    }
    inner.tombstones.retain(|_, record| record.expires_at > now);
}

fn enforce_safety_blackout_receipt_capacity(
    inner: &mut SafetyBlackoutControlPlaneInner,
    key: &SafetyReceiptKey,
    now: Instant,
) {
    while inner.receipts.len() >= MAX_TOTAL_RECEIPTS && !inner.receipts.contains_key(key) {
        if !evict_oldest_safety_blackout_receipt(inner, |_| true, now) {
            break;
        }
    }
    while inner
        .receipts
        .keys()
        .filter(|candidate| {
            candidate.principal == key.principal && candidate.operation_id == key.operation_id
        })
        .count()
        >= MAX_RECEIPTS_PER_PRINCIPAL_OPERATION
        && !inner.receipts.contains_key(key)
    {
        if !evict_oldest_safety_blackout_receipt(
            inner,
            |candidate| {
                candidate.principal == key.principal && candidate.operation_id == key.operation_id
            },
            now,
        ) {
            break;
        }
    }
}

fn evict_oldest_safety_blackout_receipt(
    inner: &mut SafetyBlackoutControlPlaneInner,
    predicate: impl Fn(&SafetyReceiptKey) -> bool,
    now: Instant,
) -> bool {
    let oldest = inner
        .receipts
        .iter()
        .filter(|(key, _)| predicate(key))
        .min_by_key(|(_, record)| record.last_used)
        .map(|(key, _)| key.clone());
    let Some(oldest) = oldest else {
        return false;
    };
    inner.receipts.remove(&oldest);
    inner.lanes.remove(&oldest);
    insert_safety_blackout_tombstone(inner, oldest, now);
    true
}

fn insert_safety_blackout_tombstone(
    inner: &mut SafetyBlackoutControlPlaneInner,
    key: SafetyReceiptKey,
    now: Instant,
) {
    while inner.tombstones.len() >= MAX_TOTAL_TOMBSTONES && !inner.tombstones.contains_key(&key) {
        let oldest = inner
            .tombstones
            .iter()
            .min_by_key(|(_, record)| record.last_used)
            .map(|(key, _)| key.clone());
        if let Some(oldest) = oldest {
            inner.tombstones.remove(&oldest);
        } else {
            break;
        }
    }
    let last_used = next_safety_blackout_sequence(inner);
    inner.tombstones.insert(
        key,
        SafetyTombstoneRecord {
            expires_at: now + TOMBSTONE_TTL,
            last_used,
        },
    );
}

fn next_output_control_sequence(inner: &mut OutputControlPlaneInner) -> u64 {
    inner.sequence = inner.sequence.saturating_add(1);
    inner.sequence
}

fn purge_output_control_expired(inner: &mut OutputControlPlaneInner, now: Instant) {
    let expired = inner
        .receipts
        .iter()
        .filter_map(|(key, record)| (record.expires_at <= now).then_some(key.clone()))
        .collect::<Vec<_>>();
    for key in expired {
        inner.receipts.remove(&key);
        inner.lanes.remove(&key);
        insert_output_control_tombstone(inner, key, now);
    }
    inner.tombstones.retain(|_, record| record.expires_at > now);
}

fn enforce_output_control_receipt_capacity(
    inner: &mut OutputControlPlaneInner,
    key: &OutputControlReceiptKey,
    now: Instant,
) {
    while inner.receipts.len() >= MAX_TOTAL_RECEIPTS && !inner.receipts.contains_key(key) {
        if !evict_oldest_output_control_receipt(inner, |_| true, now) {
            break;
        }
    }
    while inner
        .receipts
        .keys()
        .filter(|candidate| {
            candidate.principal == key.principal && candidate.operation_id == key.operation_id
        })
        .count()
        >= MAX_RECEIPTS_PER_PRINCIPAL_OPERATION
        && !inner.receipts.contains_key(key)
    {
        if !evict_oldest_output_control_receipt(
            inner,
            |candidate| {
                candidate.principal == key.principal && candidate.operation_id == key.operation_id
            },
            now,
        ) {
            break;
        }
    }
}

fn evict_oldest_output_control_receipt(
    inner: &mut OutputControlPlaneInner,
    predicate: impl Fn(&OutputControlReceiptKey) -> bool,
    now: Instant,
) -> bool {
    let oldest = inner
        .receipts
        .iter()
        .filter(|(key, _)| predicate(key))
        .min_by_key(|(_, record)| record.last_used)
        .map(|(key, _)| key.clone());
    let Some(oldest) = oldest else {
        return false;
    };
    inner.receipts.remove(&oldest);
    inner.lanes.remove(&oldest);
    insert_output_control_tombstone(inner, oldest, now);
    true
}

fn insert_output_control_tombstone(
    inner: &mut OutputControlPlaneInner,
    key: OutputControlReceiptKey,
    now: Instant,
) {
    while inner.tombstones.len() >= MAX_TOTAL_TOMBSTONES && !inner.tombstones.contains_key(&key) {
        let oldest = inner
            .tombstones
            .iter()
            .min_by_key(|(_, record)| record.last_used)
            .map(|(key, _)| key.clone());
        if let Some(oldest) = oldest {
            inner.tombstones.remove(&oldest);
        } else {
            break;
        }
    }
    let last_used = next_output_control_sequence(inner);
    inner.tombstones.insert(
        key,
        SafetyTombstoneRecord {
            expires_at: now + TOMBSTONE_TTL,
            last_used,
        },
    );
}

/// Execute the local desktop's safer-direction emergency blackout. This path
/// deliberately takes no project coordinator or operator-lock gate: Full Lock
/// and an authored transaction may not delay the priority safety latch.
pub(crate) fn engage_safety_blackout(
    window: &WebviewWindow,
    state: &AppState,
    request: SafetyBlackoutEngageRequestV1,
) -> SafetyBlackoutEngageResponseV1 {
    if request.validate().is_err() {
        return safety_blackout_rejection(&request, RuntimeCommandErrorCodeV1::InvalidRequest);
    }
    let shape_sha256 = match safety_blackout_shape_sha256(&request) {
        Ok(shape) => shape,
        Err(()) => {
            return safety_blackout_rejection(&request, RuntimeCommandErrorCodeV1::InvalidRequest)
        }
    };
    let now = Instant::now();
    let _owner_rotation = match state.project_transaction_owner_rotation.lock() {
        Ok(guard) => guard,
        Err(_) => return safety_blackout_rejection(&request, RuntimeCommandErrorCodeV1::Internal),
    };
    let binding = match capture_binding(state, window.label()) {
        Ok(binding) => binding,
        Err(code) => return safety_blackout_rejection(&request, code),
    };
    engage_safety_blackout_bound(state, binding, request, shape_sha256, now)
}

fn engage_safety_blackout_bound(
    state: &AppState,
    binding: CallerBinding,
    request: SafetyBlackoutEngageRequestV1,
    shape_sha256: String,
    now: Instant,
) -> SafetyBlackoutEngageResponseV1 {
    let key = safety_blackout_receipt_key(&request, &binding);
    let lane =
        match state
            .runtime_control_plane
            .reserve_safety_blackout_lane(&key, &shape_sha256, now)
        {
            SafetyBlackoutLaneReservation::Terminal(response) => return response,
            SafetyBlackoutLaneReservation::Rejected(code) => {
                return safety_blackout_rejection(&request, code)
            }
            SafetyBlackoutLaneReservation::Lane(lane) => lane,
        };
    let _lane_guard = match lane.lock() {
        Ok(guard) => guard,
        Err(_) => {
            state
                .runtime_control_plane
                .release_safety_blackout_lane(&key);
            return safety_blackout_rejection(&request, RuntimeCommandErrorCodeV1::Internal);
        }
    };
    if let Some(result) = state
        .runtime_control_plane
        .recheck_safety_blackout_terminal(&key, &shape_sha256, now)
    {
        return match result {
            SafetyBlackoutLaneReservation::Terminal(response) => response,
            SafetyBlackoutLaneReservation::Rejected(code) => {
                safety_blackout_rejection(&request, code)
            }
            SafetyBlackoutLaneReservation::Lane(_) => {
                unreachable!("safety terminal recheck never creates a lane")
            }
        };
    }

    let (inflight, audit_sequence) = match state
        .runtime_control_plane
        .admit_and_audit_safety_blackout(&binding, &key, &shape_sha256, now)
    {
        Ok(admission) => admission,
        Err(code) => {
            state
                .runtime_control_plane
                .release_safety_blackout_lane(&key);
            return safety_blackout_rejection(&request, code);
        }
    };
    let engine_result = state
        .engine
        .safety_blackout_engage_published(Instant::now() + Duration::from_secs(2));
    state
        .runtime_control_plane
        .finish_safety_blackout_inflight(&inflight);
    let response = match engine_result {
        Ok(disposition) => SafetyBlackoutEngageResponseV1::Receipt(SafetyBlackoutEngageReceiptV1 {
            operation_id: SAFETY_BLACKOUT_ENGAGE_OPERATION_ID.to_string(),
            request_id: request.request_id,
            shape_sha256: shape_sha256.clone(),
            audit_sequence,
            outcome: match disposition {
                SafetyBlackoutEngageDisposition::Applied => SafetyBlackoutEngageOutcomeV1::Applied,
                SafetyBlackoutEngageDisposition::NoOp => SafetyBlackoutEngageOutcomeV1::NoOp,
            },
        }),
        Err(_) => safety_blackout_rejection(&request, RuntimeCommandErrorCodeV1::PublicationFailed),
    };
    if state
        .runtime_control_plane
        .store_safety_blackout_terminal(key.clone(), shape_sha256, response.clone(), Instant::now())
        .is_err()
    {
        state
            .runtime_control_plane
            .release_safety_blackout_lane(&key);
    }
    response
}

#[cfg(test)]
pub(crate) fn engage_safety_blackout_for_test_window(
    state: &AppState,
    window_label: &str,
    request: SafetyBlackoutEngageRequestV1,
    now: Instant,
) -> SafetyBlackoutEngageResponseV1 {
    if request.validate().is_err() {
        return safety_blackout_rejection(&request, RuntimeCommandErrorCodeV1::InvalidRequest);
    }
    let shape_sha256 = match safety_blackout_shape_sha256(&request) {
        Ok(shape) => shape,
        Err(()) => {
            return safety_blackout_rejection(&request, RuntimeCommandErrorCodeV1::InvalidRequest)
        }
    };
    let _owner_rotation = match state.project_transaction_owner_rotation.lock() {
        Ok(guard) => guard,
        Err(_) => return safety_blackout_rejection(&request, RuntimeCommandErrorCodeV1::Internal),
    };
    let binding = match capture_binding(state, window_label) {
        Ok(binding) => binding,
        Err(code) => return safety_blackout_rejection(&request, code),
    };
    engage_safety_blackout_bound(state, binding, request, shape_sha256, now)
}

pub(crate) fn issue_timeline_transport_authority(
    window: &WebviewWindow,
    state: &AppState,
    query_state: &ControlPlaneQueryState,
) -> Result<RuntimeCommandAuthorityBundleV1, RuntimeCommandErrorV1> {
    let now = Instant::now();
    let _owner_rotation = state
        .project_transaction_owner_rotation
        .lock()
        .map_err(|_| RuntimeCommandErrorV1::new(RuntimeCommandErrorCodeV1::Internal))?;
    let binding =
        capture_binding(state, window.label()).map_err(|code| RuntimeCommandErrorV1::new(code))?;
    let project = query_state
        .issue_project_mutation_fence_for_window(window.label(), state)
        .map_err(|_| RuntimeCommandErrorV1::new(RuntimeCommandErrorCodeV1::Forbidden))?;
    let source_runtime_authority = state.engine.timeline_transport_authority();
    let fence = TimelineTransportRuntimeFenceV1 {
        project,
        domain: TIMELINE_TRANSPORT_RUNTIME_DOMAIN_V1.to_string(),
        source_runtime_epoch: source_runtime_authority.epoch,
        source_runtime_generation: source_runtime_authority.generation,
    };
    fence
        .validate()
        .map_err(|_| RuntimeCommandErrorV1::new(RuntimeCommandErrorCodeV1::Internal))?;
    let mut authority_id = None;
    for _ in 0..8 {
        let candidate = random_authority_id()
            .map_err(|_| RuntimeCommandErrorV1::new(RuntimeCommandErrorCodeV1::Internal))?;
        match state.runtime_control_plane.issue_authority(
            binding.clone(),
            fence.clone(),
            candidate.clone(),
            now,
        ) {
            Ok(()) => {
                authority_id = Some(candidate);
                break;
            }
            Err(RuntimeCommandErrorCodeV1::Conflict) => continue,
            Err(code) => return Err(RuntimeCommandErrorV1::new(code)),
        }
    }
    let authority_id = authority_id
        .ok_or_else(|| RuntimeCommandErrorV1::new(RuntimeCommandErrorCodeV1::Internal))?;
    Ok(RuntimeCommandAuthorityBundleV1 {
        operation_id: TIMELINE_TRANSPORT_SET_PLAYING_OPERATION_ID.to_string(),
        authority_id,
        fence,
    })
}

pub(crate) fn set_timeline_transport_playing(
    window: &WebviewWindow,
    state: &AppState,
    query_state: &ControlPlaneQueryState,
    request: RuntimeCommandRequestV1,
) -> RuntimeCommandResponseV1 {
    if request.validate().is_err() {
        return rejection(&request, RuntimeCommandErrorCodeV1::InvalidRequest);
    }
    let shape_sha256 = match shape_sha256(&request) {
        Ok(shape) => shape,
        Err(()) => return rejection(&request, RuntimeCommandErrorCodeV1::InvalidRequest),
    };
    let now = Instant::now();
    let _owner_rotation = match state.project_transaction_owner_rotation.lock() {
        Ok(guard) => guard,
        Err(_) => return rejection(&request, RuntimeCommandErrorCodeV1::Internal),
    };
    let binding = match capture_binding(state, window.label()) {
        Ok(binding) => binding,
        Err(code) => return rejection(&request, code),
    };
    let key = receipt_key(&request, &binding);
    let lane = match state
        .runtime_control_plane
        .reserve_lane(&key, &shape_sha256, now)
    {
        LaneReservation::Terminal(response) => return response,
        LaneReservation::Rejected(code) => return rejection(&request, code),
        LaneReservation::Lane(lane) => lane,
    };
    let _lane_guard = match lane.lock() {
        Ok(guard) => guard,
        Err(_) => {
            state.runtime_control_plane.release_lane(&key);
            return rejection(&request, RuntimeCommandErrorCodeV1::Internal);
        }
    };
    if let Some(result) = state
        .runtime_control_plane
        .recheck_terminal(&key, &shape_sha256, now)
    {
        return match result {
            LaneReservation::Terminal(response) => response,
            LaneReservation::Rejected(code) => rejection(&request, code),
            LaneReservation::Lane(_) => unreachable!("terminal recheck never creates a lane"),
        };
    }
    if query_state
        .validate_project_mutation_fence_window(
            window.label(),
            &request.expected_fence.project,
            binding.owner_incarnation,
        )
        .is_err()
    {
        state.runtime_control_plane.release_lane(&key);
        return rejection(&request, RuntimeCommandErrorCodeV1::Forbidden);
    }

    let response = execute_new_request(state, &binding, &request, &shape_sha256, &key, now);
    let terminal = matches!(
        response,
        RuntimeCommandResponseV1::Receipt(_) | RuntimeCommandResponseV1::Rejected(_)
    );
    // Every result emitted after engine admission is retained. Pre-admission
    // Busy/Overloaded outcomes intentionally leave the capability reusable.
    let should_retain = match &response {
        RuntimeCommandResponseV1::Receipt(_) => true,
        RuntimeCommandResponseV1::Rejected(rejection) => !matches!(
            rejection.error.code,
            RuntimeCommandErrorCodeV1::Busy | RuntimeCommandErrorCodeV1::Overloaded
        ),
    };
    if terminal && should_retain {
        state.runtime_control_plane.store_terminal(
            key,
            shape_sha256,
            response.clone(),
            Instant::now(),
        );
    } else {
        state.runtime_control_plane.release_lane(&key);
    }
    response
}

fn execute_new_request(
    state: &AppState,
    binding: &CallerBinding,
    request: &RuntimeCommandRequestV1,
    shape_sha256: &str,
    key: &ReceiptKey,
    now: Instant,
) -> RuntimeCommandResponseV1 {
    let _external_admission = match lock_project_external_command_admission(state) {
        Ok(guard) => guard,
        Err(_) => return rejection(request, RuntimeCommandErrorCodeV1::Internal),
    };
    let mut coordinator = match lock_project_coordinator(state) {
        Ok(coordinator) => coordinator,
        Err(_) => return rejection(request, RuntimeCommandErrorCodeV1::Internal),
    };
    if reconcile_project_checkpoint_for_coordinator(state, &mut coordinator).is_err()
        || !exact_project_fence_matches(&coordinator, &request.expected_fence)
    {
        return rejection(request, RuntimeCommandErrorCodeV1::StaleFence);
    }
    if ensure_no_pending_project_transaction(&coordinator).is_err()
        || state
            .project_transaction_active
            .load(std::sync::atomic::Ordering::Acquire)
    {
        return rejection(request, RuntimeCommandErrorCodeV1::Busy);
    }
    // Runtime transport is intentionally permitted under Partial Lock but
    // Full Lock is a hard safety fence. It is not coupled to output ownership,
    // so a Standby machine can keep its local runtime warm.
    if ensure_project_operator_video_clip_slot_runtime_allowed(
        state,
        &coordinator,
        &binding.principal,
    )
    .is_err()
    {
        return rejection(request, RuntimeCommandErrorCodeV1::Forbidden);
    }
    let inflight = match state.runtime_control_plane.admit_new(
        binding,
        &request.expected_fence,
        &request.authority_id,
        key,
        now,
    ) {
        Ok(inflight) => inflight,
        Err(code) => return rejection(request, code),
    };
    let engine_result = state.engine.set_timeline_playing_published(
        request.expected_fence.source_runtime_epoch,
        request.expected_fence.source_runtime_generation,
        request.payload.playing,
        Instant::now() + Duration::from_secs(2),
    );
    state.runtime_control_plane.finish_inflight(&inflight);
    match engine_result {
        Ok(ack) => RuntimeCommandResponseV1::Receipt(RuntimeCommandReceiptV1 {
            operation_id: TIMELINE_TRANSPORT_SET_PLAYING_OPERATION_ID.to_string(),
            request_id: request.request_id,
            fence_before: request.expected_fence.clone(),
            requested_playing: request.payload.playing,
            epoch_after: ack.epoch_after,
            generation_after: ack.generation_after,
            shape_sha256: shape_sha256.to_string(),
            outcome: match ack.disposition {
                TimelineTransportSetPlayingDisposition::Applied => {
                    RuntimeCommandReceiptOutcomeV1::Applied
                }
                TimelineTransportSetPlayingDisposition::NoOp => {
                    RuntimeCommandReceiptOutcomeV1::NoOp
                }
            },
        }),
        Err(error) => rejection(
            request,
            if error.contains("stale") || error.contains("expired") {
                RuntimeCommandErrorCodeV1::StaleFence
            } else {
                RuntimeCommandErrorCodeV1::PublicationFailed
            },
        ),
    }
}

/// Issue exactly one local, owner-bound capability for the currently observed
/// Follow generation. Full Lock does not block this runtime safety read.
pub(crate) fn issue_timeline_follow_abort_authority(
    window: &WebviewWindow,
    state: &AppState,
    query_state: &ControlPlaneQueryState,
) -> Result<TimelineFollowAbortAuthorityBundleV1, RuntimeCommandErrorV1> {
    let now = Instant::now();
    let _owner_rotation = state
        .project_transaction_owner_rotation
        .lock()
        .map_err(|_| RuntimeCommandErrorV1::new(RuntimeCommandErrorCodeV1::Internal))?;
    let binding = capture_binding(state, window.label()).map_err(RuntimeCommandErrorV1::new)?;
    let project = query_state
        .issue_project_mutation_fence_for_window(window.label(), state)
        .map_err(|_| RuntimeCommandErrorV1::new(RuntimeCommandErrorCodeV1::Forbidden))?;
    let output_ownership_epoch = state.engine.output_ownership_status().epoch;
    let runtime = state
        .engine
        .timeline_follow_runtime_status(output_ownership_epoch);
    if output_ownership_epoch == 0 || runtime.generation == 0 {
        return Err(RuntimeCommandErrorV1::new(
            RuntimeCommandErrorCodeV1::StaleFence,
        ));
    }
    let fence = TimelineFollowAbortRuntimeFenceV1 {
        project,
        domain: TIMELINE_FOLLOW_ABORT_RUNTIME_DOMAIN_V1.to_string(),
        output_ownership_epoch,
        follow_generation: runtime.generation,
    };
    fence
        .validate()
        .map_err(|_| RuntimeCommandErrorV1::new(RuntimeCommandErrorCodeV1::Internal))?;
    let mut authority_id = None;
    for _ in 0..8 {
        let candidate = random_authority_id()
            .map_err(|_| RuntimeCommandErrorV1::new(RuntimeCommandErrorCodeV1::Internal))?;
        match state.runtime_control_plane.issue_follow_abort_authority(
            binding.clone(),
            fence.clone(),
            candidate.clone(),
            now,
        ) {
            Ok(()) => {
                authority_id = Some(candidate);
                break;
            }
            Err(RuntimeCommandErrorCodeV1::Conflict) => continue,
            Err(code) => return Err(RuntimeCommandErrorV1::new(code)),
        }
    }
    Ok(TimelineFollowAbortAuthorityBundleV1 {
        operation_id: TIMELINE_FOLLOW_ABORT_OPERATION_ID.to_string(),
        authority_id: authority_id
            .ok_or_else(|| RuntimeCommandErrorV1::new(RuntimeCommandErrorCodeV1::Internal))?,
        fence,
    })
}

pub(crate) fn abort_timeline_follow_runtime(
    window: &WebviewWindow,
    state: &AppState,
    query_state: &ControlPlaneQueryState,
    request: TimelineFollowAbortRuntimeRequestV1,
) -> TimelineFollowAbortRuntimeResponseV1 {
    if request.validate().is_err() {
        return follow_abort_rejection(&request, RuntimeCommandErrorCodeV1::InvalidRequest);
    }
    let shape_sha256 = match follow_abort_shape_sha256(&request) {
        Ok(shape) => shape,
        Err(()) => {
            return follow_abort_rejection(&request, RuntimeCommandErrorCodeV1::InvalidRequest)
        }
    };
    let now = Instant::now();
    let _owner_rotation = match state.project_transaction_owner_rotation.lock() {
        Ok(guard) => guard,
        Err(_) => return follow_abort_rejection(&request, RuntimeCommandErrorCodeV1::Internal),
    };
    let binding = match capture_binding(state, window.label()) {
        Ok(binding) => binding,
        Err(code) => return follow_abort_rejection(&request, code),
    };
    let expected_project = request.expected_fence.project.clone();
    let window_label = window.label().to_string();
    abort_timeline_follow_runtime_bound(
        state,
        binding,
        request,
        shape_sha256,
        now,
        move |binding| {
            query_state
                .validate_project_mutation_fence_window(
                    &window_label,
                    &expected_project,
                    binding.owner_incarnation,
                )
                .is_ok()
        },
    )
}

fn abort_timeline_follow_runtime_bound<F>(
    state: &AppState,
    binding: CallerBinding,
    request: TimelineFollowAbortRuntimeRequestV1,
    shape_sha256: String,
    now: Instant,
    validate_project_fence: F,
) -> TimelineFollowAbortRuntimeResponseV1
where
    F: FnOnce(&CallerBinding) -> bool,
{
    let key = follow_abort_receipt_key(&request, &binding);
    let lane = match state
        .runtime_control_plane
        .reserve_follow_abort_lane(&key, &shape_sha256, now)
    {
        FollowAbortLaneReservation::Terminal(response) => return response,
        FollowAbortLaneReservation::Rejected(code) => {
            return follow_abort_rejection(&request, code)
        }
        FollowAbortLaneReservation::Lane(lane) => lane,
    };
    let _lane_guard = match lane.lock() {
        Ok(guard) => guard,
        Err(_) => {
            state.runtime_control_plane.release_follow_abort_lane(&key);
            return follow_abort_rejection(&request, RuntimeCommandErrorCodeV1::Internal);
        }
    };
    if let Some(result) =
        state
            .runtime_control_plane
            .recheck_follow_abort_terminal(&key, &shape_sha256, now)
    {
        return match result {
            FollowAbortLaneReservation::Terminal(response) => response,
            FollowAbortLaneReservation::Rejected(code) => follow_abort_rejection(&request, code),
            FollowAbortLaneReservation::Lane(_) => {
                unreachable!("Follow abort terminal recheck never creates a lane")
            }
        };
    }
    if !validate_project_fence(&binding) {
        state.runtime_control_plane.release_follow_abort_lane(&key);
        return follow_abort_rejection(&request, RuntimeCommandErrorCodeV1::Forbidden);
    }

    let response = execute_new_follow_abort(state, &binding, &request, &shape_sha256, &key, now);
    let should_retain = match &response {
        TimelineFollowAbortRuntimeResponseV1::Receipt(_) => true,
        TimelineFollowAbortRuntimeResponseV1::Rejected(rejection) => !matches!(
            rejection.error.code,
            RuntimeCommandErrorCodeV1::Busy | RuntimeCommandErrorCodeV1::Overloaded
        ),
    };
    if should_retain {
        state.runtime_control_plane.store_follow_abort_terminal(
            key,
            shape_sha256,
            response.clone(),
            Instant::now(),
        );
    } else {
        state.runtime_control_plane.release_follow_abort_lane(&key);
    }
    response
}

#[cfg(test)]
pub(crate) fn issue_timeline_follow_abort_authority_for_test_window(
    state: &AppState,
    window_label: &str,
    fence: TimelineFollowAbortRuntimeFenceV1,
    authority_id: String,
    now: Instant,
) -> Result<(), RuntimeCommandErrorCodeV1> {
    TimelineFollowAbortAuthorityBundleV1 {
        operation_id: TIMELINE_FOLLOW_ABORT_OPERATION_ID.to_string(),
        authority_id: authority_id.clone(),
        fence: fence.clone(),
    }
    .validate()
    .map_err(|_| RuntimeCommandErrorCodeV1::InvalidRequest)?;
    let _owner_rotation = state
        .project_transaction_owner_rotation
        .lock()
        .map_err(|_| RuntimeCommandErrorCodeV1::Internal)?;
    let binding = capture_binding(state, window_label)?;
    state
        .runtime_control_plane
        .issue_follow_abort_authority(binding, fence, authority_id, now)
}

#[cfg(test)]
pub(crate) fn abort_timeline_follow_runtime_for_test_window(
    state: &AppState,
    window_label: &str,
    request: TimelineFollowAbortRuntimeRequestV1,
    now: Instant,
) -> TimelineFollowAbortRuntimeResponseV1 {
    if request.validate().is_err() {
        return follow_abort_rejection(&request, RuntimeCommandErrorCodeV1::InvalidRequest);
    }
    let shape_sha256 = match follow_abort_shape_sha256(&request) {
        Ok(shape) => shape,
        Err(()) => {
            return follow_abort_rejection(&request, RuntimeCommandErrorCodeV1::InvalidRequest)
        }
    };
    let _owner_rotation = match state.project_transaction_owner_rotation.lock() {
        Ok(guard) => guard,
        Err(_) => return follow_abort_rejection(&request, RuntimeCommandErrorCodeV1::Internal),
    };
    let binding = match capture_binding(state, window_label) {
        Ok(binding) => binding,
        Err(code) => return follow_abort_rejection(&request, code),
    };
    abort_timeline_follow_runtime_bound(state, binding, request, shape_sha256, now, |_| true)
}

fn execute_new_follow_abort(
    state: &AppState,
    binding: &CallerBinding,
    request: &TimelineFollowAbortRuntimeRequestV1,
    shape_sha256: &str,
    key: &ReceiptKey,
    now: Instant,
) -> TimelineFollowAbortRuntimeResponseV1 {
    let _external_admission = match lock_project_external_command_admission(state) {
        Ok(guard) => guard,
        Err(_) => return follow_abort_rejection(request, RuntimeCommandErrorCodeV1::Internal),
    };
    let mut coordinator = match lock_project_coordinator(state) {
        Ok(coordinator) => coordinator,
        Err(_) => return follow_abort_rejection(request, RuntimeCommandErrorCodeV1::Internal),
    };
    if reconcile_project_checkpoint_for_coordinator(state, &mut coordinator).is_err()
        || coordinator.epoch != request.expected_fence.project.project_epoch
        || coordinator.revision != request.expected_fence.project.project_revision
        || coordinator.checkpoint_hash != request.expected_fence.project.project_checkpoint_hash
        || coordinator.publication_generation
            != request
                .expected_fence
                .project
                .project_publication_generation
    {
        return follow_abort_rejection(request, RuntimeCommandErrorCodeV1::StaleFence);
    }
    if ensure_no_pending_project_transaction(&coordinator).is_err()
        || state
            .project_transaction_active
            .load(std::sync::atomic::Ordering::Acquire)
    {
        return follow_abort_rejection(request, RuntimeCommandErrorCodeV1::Busy);
    }
    let current_output_epoch = state.engine.output_ownership_status().epoch;
    let before = state
        .engine
        .timeline_follow_runtime_status(current_output_epoch);
    if current_output_epoch != request.expected_fence.output_ownership_epoch
        || before.generation != request.expected_fence.follow_generation
        || before.generation == MAX_SAFE_JAVASCRIPT_INTEGER
    {
        return follow_abort_rejection(request, RuntimeCommandErrorCodeV1::StaleFence);
    }
    // Follow abort is a safety action and intentionally remains available under
    // Full Lock. The registered local owner, exact project fence and one-use
    // server capability are the complete admission boundary.
    let inflight = match state.runtime_control_plane.admit_follow_abort_new(
        binding,
        &request.expected_fence,
        &request.authority_id,
        key,
        now,
    ) {
        Ok(inflight) => inflight,
        Err(code) => return follow_abort_rejection(request, code),
    };
    let engine_result = state.engine.abort_timeline_follow_published(
        request.expected_fence.output_ownership_epoch,
        request.expected_fence.follow_generation,
        Instant::now() + Duration::from_secs(2),
    );
    state
        .runtime_control_plane
        .finish_follow_abort_inflight(&inflight);
    match engine_result {
        Ok(()) => {
            let output_ownership_epoch_after = state.engine.output_ownership_status().epoch;
            let after = state
                .engine
                .timeline_follow_runtime_status(output_ownership_epoch_after);
            let outcome = if output_ownership_epoch_after
                == request.expected_fence.output_ownership_epoch
                && after.generation == request.expected_fence.follow_generation
            {
                RuntimeCommandReceiptOutcomeV1::NoOp
            } else {
                RuntimeCommandReceiptOutcomeV1::Applied
            };
            let receipt = TimelineFollowAbortRuntimeReceiptV1 {
                operation_id: TIMELINE_FOLLOW_ABORT_OPERATION_ID.to_string(),
                request_id: request.request_id,
                fence_before: request.expected_fence.clone(),
                output_ownership_epoch_after,
                follow_generation_after: after.generation,
                shape_sha256: shape_sha256.to_string(),
                outcome,
            };
            if receipt.validate().is_err() {
                follow_abort_rejection(request, RuntimeCommandErrorCodeV1::Internal)
            } else {
                TimelineFollowAbortRuntimeResponseV1::Receipt(receipt)
            }
        }
        Err(error) => follow_abort_rejection(
            request,
            if error.contains("stale") || error.contains("expired") {
                RuntimeCommandErrorCodeV1::StaleFence
            } else {
                RuntimeCommandErrorCodeV1::PublicationFailed
            },
        ),
    }
}

fn capture_binding(
    state: &AppState,
    window_label: &str,
) -> Result<CallerBinding, RuntimeCommandErrorCodeV1> {
    let owners = state
        .project_transaction_owners
        .lock()
        .map_err(|_| RuntimeCommandErrorCodeV1::Internal)?;
    let principal = owners
        .get(window_label)
        .cloned()
        .ok_or(RuntimeCommandErrorCodeV1::Forbidden)?;
    let incarnations = state
        .project_transaction_owner_incarnations
        .lock()
        .map_err(|_| RuntimeCommandErrorCodeV1::Internal)?;
    let owner_incarnation = incarnations
        .get(window_label)
        .copied()
        .ok_or(RuntimeCommandErrorCodeV1::Forbidden)?;
    Ok(CallerBinding {
        principal,
        window_label: window_label.to_string(),
        owner_incarnation,
    })
}

fn receipt_key(request: &RuntimeCommandRequestV1, binding: &CallerBinding) -> ReceiptKey {
    let project = &request.expected_fence.project;
    ReceiptKey {
        process_incarnation: project.process_incarnation,
        session_incarnation: project.session_incarnation,
        principal: binding.principal.clone(),
        window_label: binding.window_label.clone(),
        owner_incarnation: binding.owner_incarnation,
        operation_id: request.operation_id.clone(),
        request_id: request.request_id,
        project_epoch: project.project_epoch,
        project_revision: project.project_revision,
        project_checkpoint_hash: project.project_checkpoint_hash.clone(),
        project_publication_generation: project.project_publication_generation,
        domain: request.expected_fence.domain.clone(),
        source_runtime_epoch: request.expected_fence.source_runtime_epoch,
        source_runtime_generation: request.expected_fence.source_runtime_generation,
        authority_id: request.authority_id.clone(),
    }
}

fn follow_abort_receipt_key(
    request: &TimelineFollowAbortRuntimeRequestV1,
    binding: &CallerBinding,
) -> ReceiptKey {
    let project = &request.expected_fence.project;
    ReceiptKey {
        process_incarnation: project.process_incarnation,
        session_incarnation: project.session_incarnation,
        principal: binding.principal.clone(),
        window_label: binding.window_label.clone(),
        owner_incarnation: binding.owner_incarnation,
        operation_id: request.operation_id.clone(),
        request_id: request.request_id,
        project_epoch: project.project_epoch,
        project_revision: project.project_revision,
        project_checkpoint_hash: project.project_checkpoint_hash.clone(),
        project_publication_generation: project.project_publication_generation,
        domain: request.expected_fence.domain.clone(),
        source_runtime_epoch: request.expected_fence.output_ownership_epoch,
        source_runtime_generation: request.expected_fence.follow_generation,
        authority_id: request.authority_id.clone(),
    }
}

fn safety_blackout_receipt_key(
    request: &SafetyBlackoutEngageRequestV1,
    binding: &CallerBinding,
) -> SafetyReceiptKey {
    SafetyReceiptKey {
        principal: binding.principal.clone(),
        window_label: binding.window_label.clone(),
        owner_incarnation: binding.owner_incarnation,
        operation_id: request.operation_id.clone(),
        request_id: request.request_id,
    }
}

fn shape_sha256(request: &RuntimeCommandRequestV1) -> Result<String, ()> {
    let typed = request.canonical_shape_bytes().map_err(|_| ())?;
    let mut hasher = Sha256::new();
    hasher.update(TIMELINE_TRANSPORT_SET_PLAYING_SHAPE_DOMAIN_V1);
    hasher.update((typed.len() as u64).to_be_bytes());
    hasher.update(typed);
    Ok(format!("{:x}", hasher.finalize()))
}

fn follow_abort_shape_sha256(request: &TimelineFollowAbortRuntimeRequestV1) -> Result<String, ()> {
    let typed = request.canonical_shape_bytes().map_err(|_| ())?;
    let mut hasher = Sha256::new();
    hasher.update(TIMELINE_FOLLOW_ABORT_SHAPE_DOMAIN_V1);
    hasher.update((typed.len() as u64).to_be_bytes());
    hasher.update(typed);
    Ok(format!("{:x}", hasher.finalize()))
}

fn safety_blackout_shape_sha256(request: &SafetyBlackoutEngageRequestV1) -> Result<String, ()> {
    let typed = request.canonical_shape_bytes().map_err(|_| ())?;
    let mut hasher = Sha256::new();
    hasher.update(SAFETY_BLACKOUT_ENGAGE_SHAPE_DOMAIN_V1);
    hasher.update((typed.len() as u64).to_be_bytes());
    hasher.update(typed);
    Ok(format!("{:x}", hasher.finalize()))
}

fn random_authority_id() -> Result<String, ()> {
    let mut bytes = [0_u8; protocol::control_plane_command::TIMELINE_TRANSPORT_AUTHORITY_ID_BYTES];
    getrandom::getrandom(&mut bytes).map_err(|_| ())?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

fn exact_project_fence_matches(
    coordinator: &ProjectCoordinator,
    fence: &TimelineTransportRuntimeFenceV1,
) -> bool {
    coordinator.epoch == fence.project.project_epoch
        && coordinator.revision == fence.project.project_revision
        && coordinator.checkpoint_hash == fence.project.project_checkpoint_hash
        && coordinator.publication_generation == fence.project.project_publication_generation
}

fn rejection(
    request: &RuntimeCommandRequestV1,
    code: RuntimeCommandErrorCodeV1,
) -> RuntimeCommandResponseV1 {
    RuntimeCommandResponseV1::Rejected(RuntimeCommandRejectionV1 {
        operation_id: TIMELINE_TRANSPORT_SET_PLAYING_OPERATION_ID.to_string(),
        request_id: request.request_id,
        fence_before: request.expected_fence.clone(),
        error: RuntimeCommandErrorV1::new(code),
    })
}

fn follow_abort_rejection(
    request: &TimelineFollowAbortRuntimeRequestV1,
    code: RuntimeCommandErrorCodeV1,
) -> TimelineFollowAbortRuntimeResponseV1 {
    TimelineFollowAbortRuntimeResponseV1::Rejected(TimelineFollowAbortRuntimeRejectionV1 {
        operation_id: TIMELINE_FOLLOW_ABORT_OPERATION_ID.to_string(),
        request_id: request.request_id,
        fence_before: request.expected_fence.clone(),
        error: RuntimeCommandErrorV1::new(code),
    })
}

fn safety_blackout_rejection(
    request: &SafetyBlackoutEngageRequestV1,
    code: RuntimeCommandErrorCodeV1,
) -> SafetyBlackoutEngageResponseV1 {
    SafetyBlackoutEngageResponseV1::Rejected(SafetyBlackoutEngageRejectionV1 {
        operation_id: SAFETY_BLACKOUT_ENGAGE_OPERATION_ID.to_string(),
        request_id: request.request_id,
        error: RuntimeCommandErrorV1::new(code),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use protocol::control_plane_command::{
        OutputControlActionV1, ProjectMutationFenceV1, SetTimelinePlayingRuntimePayloadV1,
        OUTPUT_BLACKOUT_RELEASE_OPERATION_ID, OUTPUT_OWNERSHIP_ARM_OPERATION_ID,
    };

    fn test_binding(principal: &str, window_label: &str, owner_incarnation: u64) -> CallerBinding {
        CallerBinding {
            principal: principal.to_string(),
            window_label: window_label.to_string(),
            owner_incarnation,
        }
    }

    fn test_fence(source_runtime_generation: u64) -> TimelineTransportRuntimeFenceV1 {
        TimelineTransportRuntimeFenceV1 {
            project: ProjectMutationFenceV1 {
                process_incarnation: 1,
                session_incarnation: 2,
                project_epoch: 3,
                project_revision: 4,
                project_checkpoint_hash: "a".repeat(64),
                project_publication_generation: 5,
            },
            domain: TIMELINE_TRANSPORT_RUNTIME_DOMAIN_V1.to_string(),
            source_runtime_epoch: 1,
            source_runtime_generation,
        }
    }

    fn test_authority_id(seed: u8) -> String {
        URL_SAFE_NO_PAD
            .encode([seed; protocol::control_plane_command::TIMELINE_TRANSPORT_AUTHORITY_ID_BYTES])
    }

    fn test_request(
        request_id: u64,
        fence: TimelineTransportRuntimeFenceV1,
        playing: bool,
    ) -> RuntimeCommandRequestV1 {
        RuntimeCommandRequestV1 {
            operation_id: TIMELINE_TRANSPORT_SET_PLAYING_OPERATION_ID.to_string(),
            authority_id: test_authority_id(request_id as u8),
            request_id,
            expected_fence: fence,
            payload: SetTimelinePlayingRuntimePayloadV1 { playing },
        }
    }

    fn test_follow_abort_fence(generation: u64) -> TimelineFollowAbortRuntimeFenceV1 {
        TimelineFollowAbortRuntimeFenceV1 {
            project: test_fence(1).project,
            domain: TIMELINE_FOLLOW_ABORT_RUNTIME_DOMAIN_V1.to_string(),
            output_ownership_epoch: 7,
            follow_generation: generation,
        }
    }

    fn test_follow_abort_request(
        request_id: u64,
        generation: u64,
    ) -> TimelineFollowAbortRuntimeRequestV1 {
        TimelineFollowAbortRuntimeRequestV1 {
            operation_id: TIMELINE_FOLLOW_ABORT_OPERATION_ID.to_string(),
            authority_id: test_authority_id(request_id as u8),
            request_id,
            expected_fence: test_follow_abort_fence(generation),
        }
    }

    fn test_follow_abort_receipt(
        request: &TimelineFollowAbortRuntimeRequestV1,
        shape_sha256: &str,
    ) -> TimelineFollowAbortRuntimeResponseV1 {
        TimelineFollowAbortRuntimeResponseV1::Receipt(TimelineFollowAbortRuntimeReceiptV1 {
            operation_id: TIMELINE_FOLLOW_ABORT_OPERATION_ID.to_string(),
            request_id: request.request_id,
            fence_before: request.expected_fence.clone(),
            output_ownership_epoch_after: request.expected_fence.output_ownership_epoch,
            follow_generation_after: request.expected_fence.follow_generation + 1,
            shape_sha256: shape_sha256.to_string(),
            outcome: RuntimeCommandReceiptOutcomeV1::Applied,
        })
    }

    fn test_receipt(
        request: &RuntimeCommandRequestV1,
        shape_sha256: &str,
    ) -> RuntimeCommandResponseV1 {
        RuntimeCommandResponseV1::Receipt(RuntimeCommandReceiptV1 {
            operation_id: TIMELINE_TRANSPORT_SET_PLAYING_OPERATION_ID.to_string(),
            request_id: request.request_id,
            fence_before: request.expected_fence.clone(),
            requested_playing: request.payload.playing,
            epoch_after: request.expected_fence.source_runtime_epoch,
            generation_after: request.expected_fence.source_runtime_generation + 1,
            shape_sha256: shape_sha256.to_string(),
            outcome: RuntimeCommandReceiptOutcomeV1::Applied,
        })
    }

    fn reserve_new_lane(
        state: &RuntimeControlPlaneState,
        key: &ReceiptKey,
        shape_sha256: &str,
        now: Instant,
    ) {
        assert!(matches!(
            state.reserve_lane(key, shape_sha256, now),
            LaneReservation::Lane(_)
        ));
    }

    fn test_output_key(
        principal: &str,
        operation_id: &str,
        request_id: u64,
        owner_incarnation: u64,
    ) -> OutputControlReceiptKey {
        OutputControlReceiptKey {
            principal: principal.to_string(),
            window_label: "main".to_string(),
            owner_incarnation,
            operation_id: operation_id.to_string(),
            request_id,
        }
    }

    fn test_output_rejection(
        key: &OutputControlReceiptKey,
        error: OutputControlErrorCodeV1,
    ) -> OutputControlResponseV1 {
        OutputControlResponseV1::Rejected(OutputControlRejectionV1 {
            operation_id: key.operation_id.clone(),
            request_id: key.request_id,
            error,
        })
    }

    fn test_output_fence() -> OutputControlFenceV1 {
        OutputControlFenceV1 {
            process_incarnation: 1,
            session_incarnation: 2,
            project_epoch: 3,
            project_revision: 4,
            project_checkpoint_hash: "a".repeat(64),
            project_publication_generation: 5,
            output_epoch: 6,
            output_generation: 7,
            safety_blackout_epoch: 8,
            safety_blackout_generation: 9,
        }
    }

    #[test]
    fn output_control_successor_is_preflighted_before_commit_and_receipt_is_infallible() {
        let basis = test_output_fence();
        assert_eq!(preflight_output_control_successor(&basis).unwrap(), (7, 8));

        let fence =
            committed_output_control_fence(&basis, 10, 11, &"b".repeat(64), 12, 13, 14, 15, 16);
        assert_eq!(fence.process_incarnation, basis.process_incarnation);
        assert_eq!(fence.session_incarnation, basis.session_incarnation);
        assert_eq!(fence.project_epoch, 10);
        assert_eq!(fence.project_revision, 11);
        assert_eq!(fence.project_checkpoint_hash, "b".repeat(64));
        assert_eq!(fence.project_publication_generation, 12);
        assert_eq!(fence.output_epoch, 13);
        assert_eq!(fence.output_generation, 14);
        assert_eq!(fence.safety_blackout_epoch, 15);
        assert_eq!(fence.safety_blackout_generation, 16);
        assert!(fence.validate().is_ok());

        let exhausted = OutputControlFenceV1 {
            output_epoch: MAX_SAFE_JAVASCRIPT_INTEGER,
            ..basis
        };
        assert!(preflight_output_control_successor(&exhausted).is_err());
    }

    #[test]
    fn output_control_terminal_survives_bookkeeping_mutex_poison_for_exact_retry() {
        let state = RuntimeControlPlaneState::default();
        let now = Instant::now();
        let key = test_output_key(
            "renderer-output-poison",
            OUTPUT_OWNERSHIP_ARM_OPERATION_ID,
            12_000,
            6,
        );
        let shape = "f".repeat(64);
        let response = test_output_rejection(&key, OutputControlErrorCodeV1::PublicationFailed);

        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = state.output_control.lock().unwrap();
            panic!("inject output-control bookkeeping poison");
        }));
        state.store_output_control_terminal(key.clone(), shape.clone(), response.clone(), now);

        assert!(matches!(
            state.reserve_output_control_lane(&key, &shape, now),
            OutputControlLaneReservation::Terminal(actual) if actual == response
        ));
    }

    #[test]
    fn output_control_takeover_passes_exact_standby_identity_to_core() {
        let action = OutputControlActionV1::TakeOverStandby {
            force: false,
            standby_session_id: "primary-a".to_string(),
            standby_generation: 42,
        };
        assert_eq!(
            standby_takeover_selector(&action),
            Ok(StandbyTakeoverCheckpointSelector::Exact(
                StandbyCheckpointIdentity {
                    session_id: "primary-a".to_string(),
                    generation: 42,
                },
            ))
        );
    }

    #[test]
    fn output_control_exact_shape_retry_conflict_and_expiry_tombstone_are_fail_closed() {
        let state = RuntimeControlPlaneState::default();
        let now = Instant::now();
        let binding = test_binding("renderer-output-receipt", "main", 7);
        let key = test_output_key(
            &binding.principal,
            OUTPUT_OWNERSHIP_ARM_OPERATION_ID,
            12_001,
            binding.owner_incarnation,
        );
        let shape = "a".repeat(64);
        let response = test_output_rejection(&key, OutputControlErrorCodeV1::PublicationFailed);

        assert!(matches!(
            state.reserve_output_control_lane(&key, &shape, now),
            OutputControlLaneReservation::Lane(_)
        ));
        state.store_output_control_terminal(key.clone(), shape.clone(), response.clone(), now);

        assert!(matches!(
            state.reserve_output_control_lane(&key, &shape, now),
            OutputControlLaneReservation::Terminal(actual) if actual == response
        ));
        assert!(matches!(
            state.recheck_output_control_terminal(&key, &shape, now),
            Some(OutputControlLaneReservation::Terminal(actual)) if actual == response
        ));
        assert!(matches!(
            state.reserve_output_control_lane(&key, &"b".repeat(64), now),
            OutputControlLaneReservation::Rejected(OutputControlErrorCodeV1::InvalidRequest)
        ));

        // Expiry removes the terminal response but leaves a tombstone so the
        // exact key cannot be reused with either the old or a new shape.
        assert!(matches!(
            state.reserve_output_control_lane(&key, &shape, now + RECEIPT_TTL),
            OutputControlLaneReservation::Rejected(OutputControlErrorCodeV1::InvalidRequest)
        ));
        assert!(matches!(
            state.reserve_output_control_lane(&key, &"c".repeat(64), now + RECEIPT_TTL),
            OutputControlLaneReservation::Rejected(OutputControlErrorCodeV1::InvalidRequest)
        ));
    }

    #[test]
    fn output_control_owner_retirement_tombstones_receipt_and_lane_without_erasing_audit() {
        let state = RuntimeControlPlaneState::default();
        let now = Instant::now();
        let binding = test_binding("renderer-output-retire", "main", 8);
        let shape = "d".repeat(64);
        let terminal_key = test_output_key(
            &binding.principal,
            OUTPUT_OWNERSHIP_ARM_OPERATION_ID,
            12_002,
            binding.owner_incarnation,
        );
        assert!(matches!(
            state.reserve_output_control_lane(&terminal_key, &shape, now),
            OutputControlLaneReservation::Lane(_)
        ));
        let (inflight, audit_sequence) = state
            .admit_and_audit_output_control(&binding, &terminal_key, &shape, &"e".repeat(64), now)
            .unwrap();
        assert_eq!(audit_sequence, 1);
        state.finish_output_control_inflight(&inflight);
        state.store_output_control_terminal(
            terminal_key.clone(),
            shape.clone(),
            test_output_rejection(&terminal_key, OutputControlErrorCodeV1::Busy),
            now,
        );

        let pending_key = test_output_key(
            &binding.principal,
            OUTPUT_BLACKOUT_RELEASE_OPERATION_ID,
            12_003,
            binding.owner_incarnation,
        );
        assert!(matches!(
            state.reserve_output_control_lane(&pending_key, &shape, now),
            OutputControlLaneReservation::Lane(_)
        ));

        state.retire_principal(&binding.principal);

        for key in [&terminal_key, &pending_key] {
            assert!(matches!(
                state.reserve_output_control_lane(key, &shape, now),
                OutputControlLaneReservation::Rejected(OutputControlErrorCodeV1::InvalidRequest)
            ));
        }
        let inner = state.output_control.lock().unwrap();
        assert!(inner.receipts.is_empty());
        assert!(inner.lanes.is_empty());
        assert!(inner.tombstones.contains_key(&terminal_key));
        assert!(inner.tombstones.contains_key(&pending_key));
        assert_eq!(
            inner.audit.len(),
            1,
            "retirement does not rewrite immutable audit"
        );
        assert_eq!(inner.audit[0].sequence, audit_sequence);
        assert_eq!(inner.audit[0].principal, binding.principal);
        assert_eq!(inner.audit[0].owner_incarnation, binding.owner_incarnation);
    }

    #[test]
    fn output_control_fake_clock_burst_is_eight_and_ninth_is_rejected_before_audit() {
        let state = RuntimeControlPlaneState::default();
        let now = Instant::now();
        let binding = test_binding("renderer-output-rate", "main", 9);
        let shape = "f".repeat(64);
        let argument_fingerprint = "1".repeat(64);

        for offset in 0..8_u64 {
            let key = test_output_key(
                &binding.principal,
                OUTPUT_OWNERSHIP_ARM_OPERATION_ID,
                12_100 + offset,
                binding.owner_incarnation,
            );
            assert!(matches!(
                state.reserve_output_control_lane(&key, &shape, now),
                OutputControlLaneReservation::Lane(_)
            ));
            let (inflight, sequence) = state
                .admit_and_audit_output_control(&binding, &key, &shape, &argument_fingerprint, now)
                .unwrap();
            assert_eq!(sequence, offset + 1);
            state.finish_output_control_inflight(&inflight);
            state.release_output_control_lane(&key);
        }

        let ninth_key = test_output_key(
            &binding.principal,
            OUTPUT_OWNERSHIP_ARM_OPERATION_ID,
            12_108,
            binding.owner_incarnation,
        );
        assert!(matches!(
            state.reserve_output_control_lane(&ninth_key, &shape, now),
            OutputControlLaneReservation::Lane(_)
        ));
        assert_eq!(
            state
                .admit_and_audit_output_control(
                    &binding,
                    &ninth_key,
                    &shape,
                    &argument_fingerprint,
                    now,
                )
                .unwrap_err(),
            OutputControlErrorCodeV1::Overloaded
        );
        let inner = state.output_control.lock().unwrap();
        assert_eq!(
            inner.audit.len(),
            8,
            "the ninth over-limit attempt is rejected before audit"
        );
        drop(inner);
        state.release_output_control_lane(&ninth_key);
    }

    #[test]
    fn output_control_single_flight_is_busy_per_principal_and_domain() {
        let state = RuntimeControlPlaneState::default();
        let now = Instant::now();
        let binding = test_binding("renderer-output-flight", "main", 10);
        let shape = "2".repeat(64);
        let argument_fingerprint = "3".repeat(64);
        let first_key = test_output_key(
            &binding.principal,
            OUTPUT_OWNERSHIP_ARM_OPERATION_ID,
            12_201,
            binding.owner_incarnation,
        );
        let second_key = test_output_key(
            &binding.principal,
            OUTPUT_OWNERSHIP_ARM_OPERATION_ID,
            12_202,
            binding.owner_incarnation,
        );
        for key in [&first_key, &second_key] {
            assert!(matches!(
                state.reserve_output_control_lane(key, &shape, now),
                OutputControlLaneReservation::Lane(_)
            ));
        }

        let (first_inflight, first_sequence) = state
            .admit_and_audit_output_control(
                &binding,
                &first_key,
                &shape,
                &argument_fingerprint,
                now,
            )
            .unwrap();
        assert_eq!(first_sequence, 1);
        assert_eq!(
            state
                .admit_and_audit_output_control(
                    &binding,
                    &second_key,
                    &shape,
                    &argument_fingerprint,
                    now,
                )
                .unwrap_err(),
            OutputControlErrorCodeV1::Busy
        );
        {
            let inner = state.output_control.lock().unwrap();
            assert_eq!(inner.audit.len(), 1, "busy does not append a second audit");
        }

        state.finish_output_control_inflight(&first_inflight);
        let (second_inflight, second_sequence) = state
            .admit_and_audit_output_control(
                &binding,
                &second_key,
                &shape,
                &argument_fingerprint,
                now,
            )
            .unwrap();
        assert_eq!(second_sequence, 2);
        state.finish_output_control_inflight(&second_inflight);
        state.release_output_control_lane(&first_key);
        state.release_output_control_lane(&second_key);
    }

    #[test]
    fn token_bucket_fake_clock_caps_ten_thousand_new_admissions_at_eight_then_refills_at_four_per_second(
    ) {
        let now = Instant::now();
        let mut bucket = TokenBucket::fresh(now);
        let accepted = (0..10_000).filter(|_| bucket.try_take(now)).count();
        assert_eq!(accepted, 8);
        assert!(bucket.try_take(now + Duration::from_millis(250)));
        assert!(!bucket.try_take(now + Duration::from_millis(250)));
    }

    #[test]
    fn exact_reply_retry_is_terminal_bounded_and_shape_conflicts_tombstone() {
        let state = RuntimeControlPlaneState::default();
        let now = Instant::now();
        let binding = test_binding("renderer-a", "main", 1);
        let request = test_request(17, test_fence(9), true);
        let key = receipt_key(&request, &binding);
        let shape = shape_sha256(&request).unwrap();

        reserve_new_lane(&state, &key, &shape, now);
        state.store_terminal(
            key.clone(),
            shape.clone(),
            test_receipt(&request, &shape),
            now,
        );
        assert!(matches!(
            state.reserve_lane(&key, &shape, now),
            LaneReservation::Terminal(RuntimeCommandResponseV1::Receipt(_))
        ));
        assert!(matches!(
            state.reserve_lane(&key, &"b".repeat(64), now),
            LaneReservation::Rejected(RuntimeCommandErrorCodeV1::Conflict)
        ));
        assert!(matches!(
            state.reserve_lane(&key, &shape, now + RECEIPT_TTL),
            LaneReservation::Rejected(RuntimeCommandErrorCodeV1::Conflict)
        ));
    }

    #[test]
    fn authority_is_single_use_owner_bound_and_inflight_busy_does_not_consume_the_next_authority() {
        let state = RuntimeControlPlaneState::default();
        let now = Instant::now();
        let binding = test_binding("renderer-a", "main", 1);
        let first = test_request(18, test_fence(10), true);
        let first_key = receipt_key(&first, &binding);
        let first_shape = shape_sha256(&first).unwrap();
        state
            .issue_authority(
                binding.clone(),
                first.expected_fence.clone(),
                first.authority_id.clone(),
                now,
            )
            .unwrap();
        reserve_new_lane(&state, &first_key, &first_shape, now);
        let inflight = state
            .admit_new(
                &binding,
                &first.expected_fence,
                &first.authority_id,
                &first_key,
                now,
            )
            .unwrap();

        let second = test_request(19, test_fence(11), false);
        let second_key = receipt_key(&second, &binding);
        let second_shape = shape_sha256(&second).unwrap();
        state
            .issue_authority(
                binding.clone(),
                second.expected_fence.clone(),
                second.authority_id.clone(),
                now,
            )
            .unwrap();
        reserve_new_lane(&state, &second_key, &second_shape, now);
        assert_eq!(
            state
                .admit_new(
                    &binding,
                    &second.expected_fence,
                    &second.authority_id,
                    &second_key,
                    now,
                )
                .unwrap_err(),
            RuntimeCommandErrorCodeV1::Busy
        );

        state.finish_inflight(&inflight);
        let second_inflight = state
            .admit_new(
                &binding,
                &second.expected_fence,
                &second.authority_id,
                &second_key,
                now,
            )
            .unwrap();
        state.finish_inflight(&second_inflight);
        assert_eq!(
            state
                .admit_new(
                    &binding,
                    &second.expected_fence,
                    &second.authority_id,
                    &second_key,
                    now,
                )
                .unwrap_err(),
            RuntimeCommandErrorCodeV1::Forbidden
        );

        let cross_window = test_binding("renderer-a", "secondary", 1);
        let cross_key = receipt_key(&first, &cross_window);
        reserve_new_lane(&state, &cross_key, &first_shape, now);
        assert_eq!(
            state
                .admit_new(
                    &cross_window,
                    &first.expected_fence,
                    &first.authority_id,
                    &cross_key,
                    now,
                )
                .unwrap_err(),
            RuntimeCommandErrorCodeV1::Forbidden
        );
        let aba = test_binding("renderer-a", "main", 2);
        let aba_key = receipt_key(&first, &aba);
        reserve_new_lane(&state, &aba_key, &first_shape, now);
        assert_eq!(
            state
                .admit_new(
                    &aba,
                    &first.expected_fence,
                    &first.authority_id,
                    &aba_key,
                    now,
                )
                .unwrap_err(),
            RuntimeCommandErrorCodeV1::Forbidden
        );
    }

    #[test]
    fn expired_authority_is_stale_not_reusable() {
        let state = RuntimeControlPlaneState::default();
        let now = Instant::now();
        let binding = test_binding("renderer-a", "main", 1);
        let request = test_request(20, test_fence(12), true);
        let key = receipt_key(&request, &binding);
        let shape = shape_sha256(&request).unwrap();
        state
            .issue_authority(
                binding.clone(),
                request.expected_fence.clone(),
                request.authority_id.clone(),
                now,
            )
            .unwrap();
        reserve_new_lane(&state, &key, &shape, now);
        assert_eq!(
            state
                .admit_new(
                    &binding,
                    &request.expected_fence,
                    &request.authority_id,
                    &key,
                    now + AUTHORITY_TTL,
                )
                .unwrap_err(),
            RuntimeCommandErrorCodeV1::StaleFence
        );
    }

    #[test]
    fn authority_ids_are_single_lifecycle_ttl_renewal_and_owner_bound() {
        let state = RuntimeControlPlaneState::default();
        let now = Instant::now();
        let owner = test_binding("renderer-a", "main", 1);
        let fence = test_fence(13);

        // Consume A, then mint B for the exact same project/runtime fence.
        // A must never come back merely because the fence is reusable.
        let consumed = test_request(31, fence.clone(), true);
        let consumed_key = receipt_key(&consumed, &owner);
        let consumed_shape = shape_sha256(&consumed).unwrap();
        state
            .issue_authority(
                owner.clone(),
                fence.clone(),
                consumed.authority_id.clone(),
                now,
            )
            .unwrap();
        reserve_new_lane(&state, &consumed_key, &consumed_shape, now);
        let inflight = state
            .admit_new(&owner, &fence, &consumed.authority_id, &consumed_key, now)
            .unwrap();
        state.finish_inflight(&inflight);

        let renewed = test_request(32, fence.clone(), false);
        let renewed_key = receipt_key(&renewed, &owner);
        let renewed_shape = shape_sha256(&renewed).unwrap();
        state
            .issue_authority(
                owner.clone(),
                fence.clone(),
                renewed.authority_id.clone(),
                now,
            )
            .unwrap();
        reserve_new_lane(&state, &renewed_key, &renewed_shape, now);
        assert_eq!(
            state
                .admit_new(&owner, &fence, &consumed.authority_id, &consumed_key, now,)
                .unwrap_err(),
            RuntimeCommandErrorCodeV1::Forbidden,
            "a consumed authority_id cannot resurrect after same-fence reissue"
        );

        // A valid but non-issued opaque ID is not a substitute for B, and an
        // owner/window rotation cannot present B either.
        let mut tampered = renewed.clone();
        tampered.authority_id = test_authority_id(99);
        let tampered_key = receipt_key(&tampered, &owner);
        let tampered_shape = shape_sha256(&tampered).unwrap();
        reserve_new_lane(&state, &tampered_key, &tampered_shape, now);
        assert_eq!(
            state
                .admit_new(&owner, &fence, &tampered.authority_id, &tampered_key, now,)
                .unwrap_err(),
            RuntimeCommandErrorCodeV1::Forbidden
        );
        let rotated = test_binding("renderer-a", "main", 2);
        let rotated_key = receipt_key(&renewed, &rotated);
        reserve_new_lane(&state, &rotated_key, &renewed_shape, now);
        assert_eq!(
            state
                .admit_new(&rotated, &fence, &renewed.authority_id, &rotated_key, now,)
                .unwrap_err(),
            RuntimeCommandErrorCodeV1::Forbidden
        );

        // An expired C may be renewed as D under the same fence, but C reports
        // staleness and cannot consume D's lifecycle/token.
        let expiring = test_request(33, fence.clone(), true);
        let expiring_key = receipt_key(&expiring, &owner);
        let expiring_shape = shape_sha256(&expiring).unwrap();
        state
            .issue_authority(
                owner.clone(),
                fence.clone(),
                expiring.authority_id.clone(),
                now,
            )
            .unwrap();
        reserve_new_lane(&state, &expiring_key, &expiring_shape, now);
        let after_ttl = now + AUTHORITY_TTL;
        let fresh = test_request(34, fence.clone(), false);
        let fresh_key = receipt_key(&fresh, &owner);
        let fresh_shape = shape_sha256(&fresh).unwrap();
        state
            .issue_authority(
                owner.clone(),
                fence.clone(),
                fresh.authority_id.clone(),
                after_ttl,
            )
            .unwrap();
        reserve_new_lane(&state, &fresh_key, &fresh_shape, after_ttl);
        assert_eq!(
            state
                .admit_new(
                    &owner,
                    &fence,
                    &expiring.authority_id,
                    &expiring_key,
                    after_ttl,
                )
                .unwrap_err(),
            RuntimeCommandErrorCodeV1::StaleFence
        );
        let inflight = state
            .admit_new(&owner, &fence, &fresh.authority_id, &fresh_key, after_ttl)
            .unwrap();
        state.finish_inflight(&inflight);
    }

    #[test]
    fn follow_abort_authority_receipt_owner_aba_and_tombstone_are_independent() {
        let state = RuntimeControlPlaneState::default();
        let now = Instant::now();
        let binding = test_binding("renderer-follow", "main", 41);
        let request = test_follow_abort_request(77, 12);
        let key = follow_abort_receipt_key(&request, &binding);
        let shape = follow_abort_shape_sha256(&request).unwrap();
        state
            .issue_follow_abort_authority(
                binding.clone(),
                request.expected_fence.clone(),
                request.authority_id.clone(),
                now,
            )
            .unwrap();
        assert!(matches!(
            state.reserve_follow_abort_lane(&key, &shape, now),
            FollowAbortLaneReservation::Lane(_)
        ));
        let wrong_owner = test_binding("renderer-other", "main", 41);
        assert_eq!(
            state
                .admit_follow_abort_new(
                    &wrong_owner,
                    &request.expected_fence,
                    &request.authority_id,
                    &key,
                    now,
                )
                .unwrap_err(),
            RuntimeCommandErrorCodeV1::Forbidden
        );
        let inflight = state
            .admit_follow_abort_new(
                &binding,
                &request.expected_fence,
                &request.authority_id,
                &key,
                now,
            )
            .unwrap();
        state.finish_follow_abort_inflight(&inflight);
        state.store_follow_abort_terminal(
            key.clone(),
            shape.clone(),
            test_follow_abort_receipt(&request, &shape),
            now,
        );
        assert!(matches!(
            state.reserve_follow_abort_lane(&key, &shape, now),
            FollowAbortLaneReservation::Terminal(TimelineFollowAbortRuntimeResponseV1::Receipt(_))
        ));
        assert!(matches!(
            state.reserve_follow_abort_lane(&key, &"b".repeat(64), now),
            FollowAbortLaneReservation::Rejected(RuntimeCommandErrorCodeV1::Conflict)
        ));

        state.retire_principal("renderer-follow");
        assert!(matches!(
            state.reserve_follow_abort_lane(&key, &shape, now),
            FollowAbortLaneReservation::Rejected(RuntimeCommandErrorCodeV1::Conflict)
        ));

        let expired = test_follow_abort_request(78, 13);
        let expired_key = follow_abort_receipt_key(&expired, &binding);
        assert!(matches!(
            state.reserve_follow_abort_lane(
                &expired_key,
                &follow_abort_shape_sha256(&expired).unwrap(),
                now
            ),
            FollowAbortLaneReservation::Lane(_)
        ));
        state
            .issue_follow_abort_authority(
                binding.clone(),
                expired.expected_fence.clone(),
                expired.authority_id.clone(),
                now,
            )
            .unwrap();
        assert_eq!(
            state
                .admit_follow_abort_new(
                    &binding,
                    &expired.expected_fence,
                    &expired.authority_id,
                    &expired_key,
                    now + AUTHORITY_TTL,
                )
                .unwrap_err(),
            RuntimeCommandErrorCodeV1::StaleFence,
            "expired opaque authorities cannot be revived"
        );
    }

    #[test]
    fn safety_blackout_exact_receipt_owner_aba_and_immutable_audit_are_independent() {
        let state = RuntimeControlPlaneState::default();
        let now = Instant::now();
        let binding = test_binding("renderer-safety", "main", 51);
        let request = SafetyBlackoutEngageRequestV1 {
            operation_id: SAFETY_BLACKOUT_ENGAGE_OPERATION_ID.to_string(),
            request_id: 90_001,
        };
        let key = safety_blackout_receipt_key(&request, &binding);
        let shape = safety_blackout_shape_sha256(&request).unwrap();
        assert!(matches!(
            state.reserve_safety_blackout_lane(&key, &shape, now),
            SafetyBlackoutLaneReservation::Lane(_)
        ));
        let (inflight, audit_sequence) = state
            .admit_and_audit_safety_blackout(&binding, &key, &shape, now)
            .unwrap();
        assert_eq!(audit_sequence, 1);
        state.finish_safety_blackout_inflight(&inflight);
        let response = SafetyBlackoutEngageResponseV1::Receipt(SafetyBlackoutEngageReceiptV1 {
            operation_id: SAFETY_BLACKOUT_ENGAGE_OPERATION_ID.to_string(),
            request_id: request.request_id,
            shape_sha256: shape.clone(),
            audit_sequence,
            outcome: SafetyBlackoutEngageOutcomeV1::Applied,
        });
        state
            .store_safety_blackout_terminal(key.clone(), shape.clone(), response.clone(), now)
            .unwrap();
        assert!(matches!(
            state.reserve_safety_blackout_lane(&key, &shape, now),
            SafetyBlackoutLaneReservation::Terminal(actual) if actual == response
        ));
        assert!(matches!(
            state.reserve_safety_blackout_lane(&key, &"b".repeat(64), now),
            SafetyBlackoutLaneReservation::Rejected(RuntimeCommandErrorCodeV1::Conflict)
        ));

        state.retire_principal("renderer-safety");
        assert!(matches!(
            state.reserve_safety_blackout_lane(&key, &shape, now),
            SafetyBlackoutLaneReservation::Rejected(RuntimeCommandErrorCodeV1::Conflict)
        ));
        let inner = state.safety_blackout.lock().unwrap();
        assert_eq!(
            inner.audit.len(),
            1,
            "principal retirement never erases the immutable audit"
        );
        let audit = &inner.audit[0];
        assert_eq!(audit.sequence, 1);
        assert_eq!(audit.principal, "renderer-safety");
        assert_eq!(audit.window_label, "main");
        assert_eq!(audit.owner_incarnation, 51);
        assert_eq!(audit.operation_id, SAFETY_BLACKOUT_ENGAGE_OPERATION_ID);
        assert_eq!(audit.request_id, request.request_id);
        assert_eq!(audit.shape_sha256, shape);
    }

    #[test]
    fn safety_blackout_fake_clock_burst_is_eight_and_ninth_is_rejected_before_audit() {
        let state = RuntimeControlPlaneState::default();
        let now = Instant::now();
        let binding = test_binding("renderer-safety-rate", "main", 61);
        for offset in 0..8_u64 {
            let request = SafetyBlackoutEngageRequestV1 {
                operation_id: SAFETY_BLACKOUT_ENGAGE_OPERATION_ID.to_string(),
                request_id: 91_000 + offset,
            };
            let key = safety_blackout_receipt_key(&request, &binding);
            let shape = safety_blackout_shape_sha256(&request).unwrap();
            assert!(matches!(
                state.reserve_safety_blackout_lane(&key, &shape, now),
                SafetyBlackoutLaneReservation::Lane(_)
            ));
            let (inflight, sequence) = state
                .admit_and_audit_safety_blackout(&binding, &key, &shape, now)
                .unwrap();
            assert_eq!(sequence, offset + 1);
            state.finish_safety_blackout_inflight(&inflight);
            state.release_safety_blackout_lane(&key);
        }
        let ninth = SafetyBlackoutEngageRequestV1 {
            operation_id: SAFETY_BLACKOUT_ENGAGE_OPERATION_ID.to_string(),
            request_id: 91_008,
        };
        let ninth_key = safety_blackout_receipt_key(&ninth, &binding);
        let ninth_shape = safety_blackout_shape_sha256(&ninth).unwrap();
        assert!(matches!(
            state.reserve_safety_blackout_lane(&ninth_key, &ninth_shape, now),
            SafetyBlackoutLaneReservation::Lane(_)
        ));
        assert_eq!(
            state
                .admit_and_audit_safety_blackout(&binding, &ninth_key, &ninth_shape, now)
                .unwrap_err(),
            RuntimeCommandErrorCodeV1::Overloaded
        );
        let inner = state.safety_blackout.lock().unwrap();
        assert_eq!(
            inner.audit.len(),
            8,
            "rejected over-limit attempts are not admitted/audited"
        );
    }
}
