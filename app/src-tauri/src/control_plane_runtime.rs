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
    OutputControlActionV2, OutputControlAuthorityBundleV1, OutputControlCommandRequestV2,
    OutputControlErrorCodeV2, OutputControlFenceV1, OutputControlLeaseResultV2,
    OutputControlReceiptOutcomeV2, OutputControlReceiptV2, OutputControlRejectionV2,
    OutputControlResponseV2, OutputLeaseReceiptChangeV2, OutputLeaseReceiptOutcomeV2,
    OutputLeaseReceiptPhaseV2, RuntimeCommandAuthorityBundleV1, RuntimeCommandErrorCodeV1,
    RuntimeCommandErrorV1, RuntimeCommandReceiptOutcomeV1, RuntimeCommandReceiptV1,
    RuntimeCommandRejectionV1, RuntimeCommandRequestV1, RuntimeCommandResponseV1,
    SafetyBlackoutEngageOutcomeV1, SafetyBlackoutEngageReceiptV1, SafetyBlackoutEngageRejectionV1,
    SafetyBlackoutEngageRequestV1, SafetyBlackoutEngageResponseV1,
    TimelineFollowAbortAuthorityBundleV1, TimelineFollowAbortRuntimeFenceV1,
    TimelineFollowAbortRuntimeReceiptV1, TimelineFollowAbortRuntimeRejectionV1,
    TimelineFollowAbortRuntimeRequestV1, TimelineFollowAbortRuntimeResponseV1,
    TimelineTransportRuntimeFenceV1, MAX_SAFE_JAVASCRIPT_INTEGER,
    OUTPUT_CONTROL_AUTHORITY_QUERY_OPERATION_ID, SAFETY_BLACKOUT_ENGAGE_OPERATION_ID,
    SAFETY_BLACKOUT_ENGAGE_SHAPE_DOMAIN_V1, TIMELINE_FOLLOW_ABORT_OPERATION_ID,
    TIMELINE_FOLLOW_ABORT_RUNTIME_DOMAIN_V1, TIMELINE_FOLLOW_ABORT_SHAPE_DOMAIN_V1,
    TIMELINE_TRANSPORT_RUNTIME_DOMAIN_V1, TIMELINE_TRANSPORT_SET_PLAYING_OPERATION_ID,
    TIMELINE_TRANSPORT_SET_PLAYING_SHAPE_DOMAIN_V1,
};
use rfd::{MessageButtons, MessageDialog, MessageDialogResult, MessageLevel};
use sha2::{Digest, Sha256};
use tauri::WebviewWindow;

use super::output_lease::{
    OutputLeaseId, OutputLeaseOperationOutcome, OutputLeaseOwner, OutputLeasePhase,
    OutputLeaseRequest, OutputLeaseRequestAction, OutputLeaseRequestReceipt, OutputLeaseResource,
    OutputLeaseResources, MAX_OUTPUT_LEASE_TTL_MS,
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
const MAX_TOTAL_OUTPUT_CONTROL_IDENTITIES: usize = 2_048;
const MAX_TOTAL_AUTHORITIES: usize = 1_024;
const MAX_LANES: usize = 1_024;
const TOKEN_BUCKET_BURST: f64 = 8.0;
const TOKEN_BUCKET_PER_SECOND: f64 = 4.0;
const SAFETY_BLACKOUT_DOMAIN: &str = "safety.blackout.engage";
/// Immutable for the lifetime of the process: entries are append-only and
/// never evicted or rewritten. Persistent cross-process audit is added by the
/// sidecar tranche; this bound prevents an in-process memory sink.
const MAX_SAFETY_AUDIT_RECORDS: usize = 65_536;

/// Advanced output transitions require an OS-owned confirmation that is
/// parented to the exact local Tauri window.  The ordinary Enable action is a
/// deliberate one-click local action and must never open this dialog.
pub(crate) fn output_action_requires_native_danger_confirmation(
    action: &OutputControlActionV2,
) -> bool {
    matches!(
        action,
        OutputControlActionV2::ReleaseBlackout { .. }
            | OutputControlActionV2::EnableShowArtNetLoopbackRoute { .. }
            | OutputControlActionV2::Arm { .. }
            | OutputControlActionV2::TakeOverStandby { .. }
            | OutputControlActionV2::AddDisplay { .. }
            | OutputControlActionV2::AssignVideoOutputComposition { .. }
            | OutputControlActionV2::ForceTransferLease { .. }
    )
}

fn native_output_confirmation_copy_for_locale(locale: &str) -> (&'static str, &'static str) {
    if locale.eq_ignore_ascii_case("ja") || locale.to_ascii_lowercase().starts_with("ja-") {
        (
            "出力制御の確認",
            "この詳細操作は、ライブ出力の所有権または出力状態を変更します。続行しますか？",
        )
    } else {
        (
            "Confirm output control",
            "This advanced output operation changes live ownership or output state. Continue?",
        )
    }
}

fn native_output_confirmation_copy_for_editor_target(
    editor_target: bool,
) -> (&'static str, &'static str) {
    let (title, description) = native_output_confirmation_copy();
    if !editor_target {
        return (title, description);
    }
    if title == "出力制御の確認" {
        (
            "エディタ画面への出力確認",
            "現在の操作画面と同じディスプレイへ映像を出力します。操作画面を覆う可能性があります。続行しますか？",
        )
    } else {
        (
            "Confirm editor display output",
            "This output targets the same display as the editor and may cover the control surface. Continue?",
        )
    }
}

#[cfg(target_os = "windows")]
fn native_output_confirmation_copy() -> (&'static str, &'static str) {
    use windows::Win32::Globalization::GetUserDefaultLocaleName;

    let mut locale = [0u16; 85];
    let length = unsafe { GetUserDefaultLocaleName(&mut locale) };
    if length > 1 {
        if let Ok(locale) = String::from_utf16(&locale[..length as usize - 1]) {
            return native_output_confirmation_copy_for_locale(&locale);
        }
    }
    native_output_confirmation_copy_for_locale("en")
}

#[cfg(not(target_os = "windows"))]
fn native_output_confirmation_copy() -> (&'static str, &'static str) {
    native_output_confirmation_copy_for_locale(std::env::var("LANG").as_deref().unwrap_or("en"))
}

fn confirm_native_dangerous_output_action(
    window: &WebviewWindow,
    action: &OutputControlActionV2,
) -> bool {
    if !output_action_requires_native_danger_confirmation(action) {
        return true;
    }
    let editor_target = match action {
        OutputControlActionV2::AddDisplay { spec, .. } => {
            super::validate_editor_monitor_for_window(window)
                .ok()
                .is_some_and(|index| index == spec.monitor_index)
        }
        // SetDisplayWindowOpen is LocalExplicitAction.  Its narrow
        // editor-target warning is performed by the physical-window core,
        // after it has resolved the persisted target; non-editor close/open
        // intentionally has no modal here.
        OutputControlActionV2::SetDisplayWindowOpen { .. } => false,
        _ => false,
    };
    let (title, description) = native_output_confirmation_copy_for_editor_target(editor_target);
    matches!(
        MessageDialog::new()
            .set_level(MessageLevel::Warning)
            .set_title(title)
            .set_description(description)
            .set_buttons(MessageButtons::YesNo)
            .set_parent(window)
            .show(),
        MessageDialogResult::Yes
    )
}

/// The physical Display-window operation is normally a LocalExplicitAction.
/// Opening over the editor is the sole exception: use the same parented
/// Warning/Yes-No helper as AddDisplay and treat every response except Yes as
/// a terminal cancellation.
pub(crate) fn confirm_editor_display_window_open(window: &WebviewWindow) -> bool {
    let (title, description) = native_output_confirmation_copy_for_editor_target(true);
    matches!(
        MessageDialog::new()
            .set_level(MessageLevel::Warning)
            .set_title(title)
            .set_description(description)
            .set_buttons(MessageButtons::YesNo)
            .set_parent(window)
            .show(),
        MessageDialogResult::Yes
    )
}

fn output_confirmation_gate<F>(
    action: &OutputControlActionV2,
    confirmation: &F,
) -> Result<(), OutputControlErrorCodeV2>
where
    F: Fn(&OutputControlActionV2) -> bool,
{
    if output_action_requires_native_danger_confirmation(action) && !confirmation(action) {
        Err(OutputControlErrorCodeV2::Forbidden)
    } else {
        Ok(())
    }
}

/// Label-only variant used by the asynchronous Tauri query wrapper. The
/// wrapper captures the renderer label on the event-loop thread, then runs
/// the bounded authority capture off that thread; no WebviewWindow getter is
/// needed while the query holds backend locks.
pub(crate) fn issue_output_control_authority_for_window_label(
    window_label: &str,
    state: &AppState,
    query_state: &ControlPlaneQueryState,
) -> Result<OutputControlAuthorityBundleV1, String> {
    let fence = query_state
        .issue_output_control_fence_for_window(window_label, state)
        .map_err(|error| format!("Output control authority is unavailable: {error:?}"))?;
    Ok(OutputControlAuthorityBundleV1 {
        operation_id: OUTPUT_CONTROL_AUTHORITY_QUERY_OPERATION_ID.to_string(),
        fence,
    })
}

pub(crate) fn execute_output_control(
    app: &tauri::AppHandle,
    window: &WebviewWindow,
    state: &AppState,
    query_state: &ControlPlaneQueryState,
    request: OutputControlCommandRequestV2,
) -> OutputControlResponseV2 {
    execute_output_control_with_confirmation(app, window, state, query_state, request, |action| {
        confirm_native_dangerous_output_action(window, action)
    })
}

fn execute_output_control_with_confirmation<F>(
    app: &tauri::AppHandle,
    window: &WebviewWindow,
    state: &AppState,
    query_state: &ControlPlaneQueryState,
    request: OutputControlCommandRequestV2,
    confirmation: F,
) -> OutputControlResponseV2
where
    F: Fn(&OutputControlActionV2) -> bool,
{
    if request.validate().is_err() {
        return output_control_rejection(&request, OutputControlErrorCodeV2::InvalidRequest);
    }
    let shape_sha256 = match request.canonical_shape_bytes() {
        Ok(bytes) => hex_sha256(&bytes),
        Err(_) => {
            return output_control_rejection(&request, OutputControlErrorCodeV2::InvalidRequest)
        }
    };
    let argument_fingerprint = match request.argument_fingerprint_bytes() {
        Ok(bytes) => hex_sha256(&bytes),
        Err(_) => {
            return output_control_rejection(&request, OutputControlErrorCodeV2::InvalidRequest)
        }
    };
    let now = Instant::now();
    let owner_rotation = match state.project_transaction_owner_rotation.lock() {
        Ok(guard) => guard,
        Err(_) => return output_control_rejection(&request, OutputControlErrorCodeV2::Internal),
    };
    let binding = match capture_binding(state, window.label()) {
        Ok(binding) => binding,
        Err(_) => return output_control_rejection(&request, OutputControlErrorCodeV2::Forbidden),
    };
    if let Err(code) = state
        .runtime_control_plane
        .reserve_output_control_request_identity(
            &binding,
            request.action.operation_id(),
            request.request_id,
            &shape_sha256,
            now,
        )
    {
        return output_control_rejection(&request, code);
    }
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
            return output_control_rejection(&request, OutputControlErrorCodeV2::Internal);
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
            OutputControlLaneReservation::Lane(_) => {
                output_control_rejection(&request, OutputControlErrorCodeV2::Internal)
            }
        };
    }
    // Reject stale fences, vanished/duplicate display targets, and invalid or
    // expired lease authority before showing a native danger dialog. These
    // checks are repeated after the dialog because the owner, project, target,
    // and lease can still change while the blocking OS prompt is open.
    if query_state
        .validate_output_control_fence_window(
            window.label(),
            &request.expected_fence,
            binding.owner_incarnation,
        )
        .is_err()
        || validate_output_action_current(state, &request.action).is_err()
        || validate_display_output_monitor(window, &request.action).is_err()
    {
        return retain_output_control_rejection(
            state,
            key,
            shape_sha256,
            output_control_rejection(&request, OutputControlErrorCodeV2::StaleFence),
        );
    }
    if super::preflight_output_lease_for_control_action(
        state,
        &binding.principal,
        &binding.window_label,
        binding.owner_incarnation,
        request.request_id,
        &request.action,
        request.expected_fence.project_epoch,
    )
    .is_err()
    {
        return retain_output_control_rejection(
            state,
            key,
            shape_sha256,
            output_control_rejection(&request, OutputControlErrorCodeV2::InvalidRequest),
        );
    }
    {
        let _preconfirmation_external_admission =
            match lock_project_external_command_admission(state) {
                Ok(guard) => guard,
                Err(_) => {
                    return retain_output_control_rejection(
                        state,
                        key,
                        shape_sha256,
                        output_control_rejection(&request, OutputControlErrorCodeV2::Busy),
                    );
                }
            };
        let mut preconfirmation_coordinator = match lock_project_coordinator(state) {
            Ok(coordinator) => coordinator,
            Err(_) => {
                return retain_output_control_rejection(
                    state,
                    key,
                    shape_sha256,
                    output_control_rejection(&request, OutputControlErrorCodeV2::Busy),
                );
            }
        };
        if reconcile_project_checkpoint_for_coordinator(state, &mut preconfirmation_coordinator)
            .is_err()
            || !exact_output_control_fence_matches(
                state,
                &preconfirmation_coordinator,
                &request.expected_fence,
            )
            || ensure_no_pending_project_transaction(&preconfirmation_coordinator).is_err()
        {
            return retain_output_control_rejection(
                state,
                key,
                shape_sha256,
                output_control_rejection(&request, OutputControlErrorCodeV2::StaleFence),
            );
        }
        if ensure_project_operator_video_clip_slot_runtime_allowed(
            state,
            &preconfirmation_coordinator,
            &binding.principal,
        )
        .is_err()
        {
            return retain_output_control_rejection(
                state,
                key,
                shape_sha256,
                output_control_rejection(&request, OutputControlErrorCodeV2::Forbidden),
            );
        }
    }
    // Replay is resolved before prompting so a lost-reply retry returns the
    // durable terminal receipt without showing the native dialog again.  A
    // cancelled/closed dialog exits before admission, lease work, or engine
    // publication; the fence and window are revalidated immediately after it.
    drop(owner_rotation);
    if let Err(code) = output_confirmation_gate(&request.action, &confirmation) {
        let response = output_control_rejection(&request, code);
        state.runtime_control_plane.store_output_control_terminal(
            key,
            shape_sha256,
            response.clone(),
            Instant::now(),
        );
        return response;
    }
    // Owner rotation is an ABA guard for capture/revalidation, not a mutex
    // around native shell creation, GPU setup, engine ACK, or durable commit.
    // Holding it across that physical lane makes the synchronous authority
    // query wait behind an event-loop-affine operation. The lifecycle and
    // external-admission guards acquired below still serialize an actual
    // owner handoff with this commit.
    {
        let _owner_rotation = match state.project_transaction_owner_rotation.lock() {
            Ok(guard) => guard,
            Err(_) => {
                state
                    .runtime_control_plane
                    .release_output_control_lane(&key);
                return output_control_rejection(&request, OutputControlErrorCodeV2::Internal);
            }
        };
        if capture_binding(state, window.label()).ok().as_ref() != Some(&binding) {
            let response = output_control_rejection(&request, OutputControlErrorCodeV2::Forbidden);
            state.runtime_control_plane.store_output_control_terminal(
                key,
                shape_sha256,
                response.clone(),
                Instant::now(),
            );
            return response;
        }
        if query_state
            .validate_output_control_fence_window(
                window.label(),
                &request.expected_fence,
                binding.owner_incarnation,
            )
            .is_err()
        {
            return retain_output_control_rejection(
                state,
                key,
                shape_sha256,
                output_control_rejection(&request, OutputControlErrorCodeV2::Forbidden),
            );
        }
    }

    // Re-resolve the physical target after the native prompt, but before any
    // project/coordinator guard is acquired. WebviewWindow monitor getters
    // are event-loop RPCs; keeping one inside the coordinator scope creates
    // a circular wait with the synchronous authority poll. AddDisplay repeats
    // the final topology check using the captured, lock-free descriptor.
    if validate_display_output_monitor(window, &request.action).is_err() {
        return retain_output_control_rejection(
            state,
            key,
            shape_sha256,
            output_control_rejection(&request, OutputControlErrorCodeV2::StaleFence),
        );
    }

    let external_admission = match lock_project_external_command_admission(state) {
        Ok(guard) => guard,
        Err(_) => {
            state
                .runtime_control_plane
                .release_output_control_lane(&key);
            return output_control_rejection(&request, OutputControlErrorCodeV2::Busy);
        }
    };
    let mut coordinator = match lock_project_coordinator(state) {
        Ok(coordinator) => coordinator,
        Err(_) => {
            state
                .runtime_control_plane
                .release_output_control_lane(&key);
            return output_control_rejection(&request, OutputControlErrorCodeV2::Busy);
        }
    };
    if reconcile_project_checkpoint_for_coordinator(state, &mut coordinator).is_err()
        || !exact_output_control_fence_matches(state, &coordinator, &request.expected_fence)
        || ensure_no_pending_project_transaction(&coordinator).is_err()
    {
        return retain_output_control_rejection(
            state,
            key,
            shape_sha256,
            output_control_rejection(&request, OutputControlErrorCodeV2::StaleFence),
        );
    }
    if ensure_project_operator_video_clip_slot_runtime_allowed(
        state,
        &coordinator,
        &binding.principal,
    )
    .is_err()
    {
        return retain_output_control_rejection(
            state,
            key,
            shape_sha256,
            output_control_rejection(&request, OutputControlErrorCodeV2::Forbidden),
        );
    }
    if validate_output_action_current(state, &request.action).is_err() {
        return retain_output_control_rejection(
            state,
            key,
            shape_sha256,
            output_control_rejection(&request, OutputControlErrorCodeV2::StaleFence),
        );
    }

    // Probe the shared lease request identity before the authoritative
    // candidate commit. A same request ID with a different operation/shape
    // must be rejected before any external output work.
    if super::preflight_output_lease_for_control_action(
        state,
        &binding.principal,
        &binding.window_label,
        binding.owner_incarnation,
        request.request_id,
        &request.action,
        request.expected_fence.project_epoch,
    )
    .is_err()
    {
        return retain_output_control_rejection(
            state,
            key,
            shape_sha256,
            output_control_rejection(&request, OutputControlErrorCodeV2::InvalidRequest),
        );
    }
    let (lease_request, lease_now_ms) = match super::build_output_lease_authorization_request(
        state,
        &binding.principal,
        &binding.window_label,
        binding.owner_incarnation,
        request.request_id,
        &request.action,
        request.expected_fence.project_epoch,
    ) {
        Ok(request) => request,
        Err(_) => {
            return retain_output_control_rejection(
                state,
                key,
                shape_sha256,
                output_control_rejection(&request, OutputControlErrorCodeV2::InvalidRequest),
            );
        }
    };

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
                return retain_output_control_rejection(
                    state,
                    key,
                    shape_sha256,
                    output_control_rejection(&request, code),
                );
            }
        };

    // The exact local-window fence has now been admitted. Each mutating
    // action re-enters the lifecycle/project admission boundary through its
    // actual transition or publication commit.
    drop(coordinator);
    drop(external_admission);

    let operation_result = match &request.action {
        OutputControlActionV2::EnableOutput => super::enable_output_with_output_control_fence(
            app,
            state,
            &request.expected_fence,
            &lease_request,
            lease_now_ms,
        ),
        OutputControlActionV2::EnableShowArtNetLoopbackRoute { .. } => {
            super::enable_show_artnet_loopback_route_with_output_control_fence(
                state,
                &request.expected_fence,
                &lease_request,
                lease_now_ms,
            )
        }
        OutputControlActionV2::ReleaseBlackout { .. } => {
            super::release_safety_blackout_with_output_control_fence(
                state,
                &request.expected_fence,
                &lease_request,
                lease_now_ms,
            )
        }
        OutputControlActionV2::Arm { role, .. } => {
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
                &lease_request,
                lease_now_ms,
            )
        }
        OutputControlActionV2::TakeOverStandby {
            force,
            standby_session_id,
            standby_generation,
            ..
        } => {
            let selector = StandbyTakeoverCheckpointSelector::Exact(StandbyCheckpointIdentity {
                session_id: standby_session_id.clone(),
                generation: *standby_generation,
            });
            super::take_over_standby_core_with_lease(
                state,
                *force,
                selector,
                &request.expected_fence,
                &lease_request,
                lease_now_ms,
            )
            .map(|(_load_result, fence_after, receipt)| (true, fence_after, receipt))
        }
        OutputControlActionV2::AddDisplay { spec, .. } => {
            super::add_display_output_with_output_control_fence(
                app,
                window,
                state,
                super::DisplayOutputControlRequest {
                    spec,
                    expected_fence: &request.expected_fence,
                    lease_request: &lease_request,
                    lease_now_ms,
                    expected_owner_principal: &binding.principal,
                    expected_owner_window_label: &binding.window_label,
                    expected_owner_incarnation: binding.owner_incarnation,
                },
            )
        }
        OutputControlActionV2::SetDisplayWindowOpen {
            output_id, open, ..
        } => super::set_display_output_window_open_with_output_control_fence(
            app,
            window,
            state,
            *output_id,
            *open,
            super::DisplayOutputWindowControlRequest {
                expected_fence: &request.expected_fence,
                lease_request: &lease_request,
                lease_now_ms,
                expected_owner_principal: &binding.principal,
                expected_owner_window_label: &binding.window_label,
                expected_owner_incarnation: binding.owner_incarnation,
            },
        ),
        OutputControlActionV2::AssignVideoOutputComposition {
            output_id,
            composition_id,
            ..
        } => super::assign_video_output_composition_with_output_control_fence(
            state,
            *output_id,
            *composition_id,
            super::VideoOutputCompositionAssignmentControlRequest {
                expected_fence: &request.expected_fence,
                lease_request: &lease_request,
                lease_now_ms,
                expected_owner_principal: &binding.principal,
                expected_owner_window_label: &binding.window_label,
                expected_owner_incarnation: binding.owner_incarnation,
            },
        ),
        OutputControlActionV2::AcquireLease { .. }
        | OutputControlActionV2::RenewLease { .. }
        | OutputControlActionV2::RecoverLease { .. }
        | OutputControlActionV2::RelinquishOutputLease { .. }
        | OutputControlActionV2::ForceTransferLease { .. } => {
            Err("Output lease lifecycle is not wired in this operation path".to_string())
        }
    };

    let (applied, fence_after, lease_receipt) = match operation_result {
        Ok(result) => result,
        Err(error) => {
            eprintln!(
                "OutputControl operation {} failed before publication: {}",
                request.action.operation_id(),
                error
            );
            state
                .runtime_control_plane
                .finish_output_control_inflight(&inflight);
            let response =
                output_control_rejection(&request, OutputControlErrorCodeV2::PublicationFailed);
            state.runtime_control_plane.store_output_control_terminal(
                key,
                shape_sha256,
                response.clone(),
                Instant::now(),
            );
            return response;
        }
    };
    let lease_result =
        match output_control_lease_result_from_registry_receipt(&request.action, &lease_receipt) {
            Ok(result) => result,
            Err(_) => {
                state
                    .runtime_control_plane
                    .finish_output_control_inflight(&inflight);
                let response =
                    output_control_rejection(&request, OutputControlErrorCodeV2::Internal);
                state.runtime_control_plane.store_output_control_terminal(
                    key,
                    shape_sha256,
                    response.clone(),
                    Instant::now(),
                );
                return response;
            }
        };

    let response = OutputControlResponseV2::Receipt(Box::new(OutputControlReceiptV2 {
        operation_id: request.operation_id.clone(),
        request_id: request.request_id,
        shape_sha256: shape_sha256.clone(),
        argument_fingerprint,
        audit_sequence,
        fence_before: request.expected_fence.clone(),
        fence_after,
        outcome: if applied {
            OutputControlReceiptOutcomeV2::Applied
        } else {
            OutputControlReceiptOutcomeV2::NoOp
        },
        lease_result: Some(lease_result),
    }));
    if let Err(error) = response.validate() {
        eprintln!(
            "OutputControl operation {} produced an invalid terminal response: {}",
            request.action.operation_id(),
            error
        );
    }
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

/// Final AddDisplay owner ABA check. Callers hold the owner-rotation guard, so
/// the two registry reads form one exact binding observation.
pub(crate) fn exact_output_control_owner_matches(
    state: &AppState,
    expected_principal: &str,
    expected_window_label: &str,
    expected_owner_incarnation: u64,
) -> bool {
    capture_binding(state, expected_window_label).is_ok_and(|binding| {
        binding.principal == expected_principal
            && binding.window_label == expected_window_label
            && binding.owner_incarnation == expected_owner_incarnation
    })
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
pub(crate) struct CommittedOutputControlFenceValues<'a> {
    pub(crate) project_epoch: u64,
    pub(crate) project_revision: u64,
    pub(crate) project_checkpoint_hash: &'a str,
    pub(crate) project_publication_generation: u64,
    pub(crate) output_epoch: u64,
    pub(crate) output_generation: u64,
    pub(crate) safety_blackout_epoch: u64,
    pub(crate) safety_blackout_generation: u64,
}

pub(crate) fn committed_output_control_fence(
    basis: &OutputControlFenceV1,
    values: CommittedOutputControlFenceValues<'_>,
) -> OutputControlFenceV1 {
    let CommittedOutputControlFenceValues {
        project_epoch,
        project_revision,
        project_checkpoint_hash,
        project_publication_generation,
        output_epoch,
        output_generation,
        safety_blackout_epoch,
        safety_blackout_generation,
    } = values;
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
    action: &OutputControlActionV2,
) -> Result<(), String> {
    match action {
        OutputControlActionV2::EnableOutput => Ok(()),
        OutputControlActionV2::EnableShowArtNetLoopbackRoute { .. } => {
            super::validate_current_staged_show_artnet_loopback_route(state).map(|_| ())
        }
        OutputControlActionV2::Arm { .. } | OutputControlActionV2::ReleaseBlackout { .. } => Ok(()),
        OutputControlActionV2::AddDisplay { spec, .. } => {
            let snapshot = state.engine.snapshot();
            if snapshot.video.outputs.iter().any(|output| {
                output.kind == protocol::VideoOutputKind::Display
                    && output.monitor_id == Some(spec.monitor_index)
            }) {
                return Err("A Display output already targets this monitor".to_string());
            }
            Ok(())
        }
        OutputControlActionV2::SetDisplayWindowOpen { output_id, .. } => {
            let snapshot = state.engine.snapshot();
            let output = snapshot
                .video
                .outputs
                .iter()
                .find(|output| output.id == *output_id)
                .ok_or_else(|| "Display output no longer exists".to_string())?;
            if output.kind != protocol::VideoOutputKind::Display {
                return Err("Only Display outputs have a physical live window".to_string());
            }
            if output.monitor_id.is_none()
                || output.monitor_identity.as_deref().is_none_or(str::is_empty)
            {
                return Err("Display output monitor identity is missing".to_string());
            }
            Ok(())
        }
        OutputControlActionV2::AssignVideoOutputComposition {
            output_id,
            composition_id,
            ..
        } => {
            let snapshot = state.engine.snapshot();
            if !snapshot
                .video
                .outputs
                .iter()
                .any(|output| output.id == *output_id)
            {
                return Err("Video output no longer exists".to_string());
            }
            if !snapshot
                .video
                .compositions
                .iter()
                .any(|composition| composition.id == *composition_id)
            {
                return Err("Video composition no longer exists".to_string());
            }
            Ok(())
        }
        OutputControlActionV2::TakeOverStandby {
            force,
            standby_session_id,
            standby_generation,
            ..
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
        OutputControlActionV2::AcquireLease { .. }
        | OutputControlActionV2::RenewLease { .. }
        | OutputControlActionV2::RecoverLease { .. }
        | OutputControlActionV2::RelinquishOutputLease { .. }
        | OutputControlActionV2::ForceTransferLease { .. } => Ok(()),
    }
}

fn validate_display_output_monitor(
    window: &WebviewWindow,
    action: &OutputControlActionV2,
) -> Result<(), String> {
    let OutputControlActionV2::AddDisplay { spec, .. } = action else {
        return Ok(());
    };
    spec.validate()
        .map_err(|_| "Display output specification is invalid".to_string())?;
    let monitors = window
        .available_monitors()
        .map_err(|error| format!("Display enumeration failed: {error}"))?;
    let monitors = super::video_display_monitors_from_available(monitors)?;
    // The command is authorized by the exact current editor target as well
    // as by the selected output target. Re-resolve it on every preflight so
    // an index-only match cannot survive a monitor move/reorder/ambiguity.
    super::validate_editor_monitor_for_window(window)?;
    let Some(monitor) = monitors
        .iter()
        .find(|monitor| monitor.index == spec.monitor_index)
    else {
        return Err("Display output monitor index is not available".to_string());
    };
    if spec.monitor_identity != monitor.identity {
        return Err("Display output monitor identity is stale".to_string());
    }
    super::validate_display_monitor_dimensions(
        spec.width,
        spec.height,
        monitor.physical_width,
        monitor.physical_height,
    )
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
    request: OutputControlCommandRequestV2,
) -> OutputControlResponseV2 {
    if request.operation_id != expected_operation_id
        || request.action.operation_id() != expected_operation_id
    {
        return output_control_rejection_for_operation(
            &request,
            expected_operation_id,
            OutputControlErrorCodeV2::InvalidRequest,
        );
    }
    execute_output_control(app, window, state, query_state, request)
}

fn output_lease_error_code(
    error: super::output_lease::OutputLeaseError,
) -> OutputControlErrorCodeV2 {
    match error {
        super::output_lease::OutputLeaseError::Busy => OutputControlErrorCodeV2::Busy,
        super::output_lease::OutputLeaseError::RateLimited
        | super::output_lease::OutputLeaseError::RequestCapacity
        | super::output_lease::OutputLeaseError::LeaseCapacity
        | super::output_lease::OutputLeaseError::LaneCapacity
        | super::output_lease::OutputLeaseError::AuditCapacity => {
            OutputControlErrorCodeV2::Overloaded
        }
        super::output_lease::OutputLeaseError::Conflict
        | super::output_lease::OutputLeaseError::ReceiptNotRetained
        | super::output_lease::OutputLeaseError::InvalidRequest
        | super::output_lease::OutputLeaseError::InvalidOwner
        | super::output_lease::OutputLeaseError::InvalidResources
        | super::output_lease::OutputLeaseError::InvalidProject => {
            OutputControlErrorCodeV2::InvalidRequest
        }
        super::output_lease::OutputLeaseError::Expired
        | super::output_lease::OutputLeaseError::UnknownLease
        | super::output_lease::OutputLeaseError::ResourceConflict
        | super::output_lease::OutputLeaseError::InvalidTransition
        | super::output_lease::OutputLeaseError::StaleOwner
        | super::output_lease::OutputLeaseError::StaleGeneration
        | super::output_lease::OutputLeaseError::InvalidTtl
        | super::output_lease::OutputLeaseError::LeaseIdExhausted
        | super::output_lease::OutputLeaseError::RequestNotInFlight
        | super::output_lease::OutputLeaseError::ClockRollback
        | super::output_lease::OutputLeaseError::GenerationExhausted
        | super::output_lease::OutputLeaseError::ClockExhausted => {
            OutputControlErrorCodeV2::Forbidden
        }
    }
}

fn build_output_lease_lifecycle_request(
    state: &AppState,
    binding: &CallerBinding,
    request: &OutputControlCommandRequestV2,
) -> Result<(OutputLeaseRequest, u64), String> {
    let now_ms = state.output_lease_now_ms()?;
    let process_incarnation = state
        .output_lease_registry
        .lock()
        .map_err(|_| "Output lease registry lock was poisoned".to_string())?
        .process_session_incarnation();
    let owner = OutputLeaseOwner::new(
        binding.principal.clone(),
        binding.window_label.clone(),
        process_incarnation,
        binding.owner_incarnation,
    )
    .map_err(|error| format!("Output lease owner is invalid: {error:?}"))?;
    let action = match &request.action {
        OutputControlActionV2::AcquireLease { role } => {
            let resources = match role {
                protocol::control_plane_command::OutputControlTargetRoleV1::Lighting => {
                    OutputLeaseResources::new(&[OutputLeaseResource::Lighting])
                }
                protocol::control_plane_command::OutputControlTargetRoleV1::Video => {
                    OutputLeaseResources::new(&[OutputLeaseResource::Video])
                }
                protocol::control_plane_command::OutputControlTargetRoleV1::Both => {
                    OutputLeaseResources::new(&[
                        OutputLeaseResource::Lighting,
                        OutputLeaseResource::Video,
                    ])
                }
            }
            .map_err(|error| format!("Output lease resources are invalid: {error:?}"))?;
            OutputLeaseRequestAction::Acquire {
                owner,
                resources,
                project_identity: format!("project_epoch:{}", request.expected_fence.project_epoch),
                ttl_ms: MAX_OUTPUT_LEASE_TTL_MS,
            }
        }
        OutputControlActionV2::RenewLease { lease } => OutputLeaseRequestAction::Renew {
            lease_id: OutputLeaseId::decode(&lease.lease_id)
                .map_err(|error| format!("Output lease authority is invalid: {error:?}"))?,
            owner,
            expected_generation: lease.generation,
            ttl_ms: MAX_OUTPUT_LEASE_TTL_MS,
        },
        OutputControlActionV2::RecoverLease { lease } => OutputLeaseRequestAction::Recover {
            lease_id: OutputLeaseId::decode(&lease.lease_id)
                .map_err(|error| format!("Output lease authority is invalid: {error:?}"))?,
            owner,
            expected_generation: lease.generation,
            ttl_ms: MAX_OUTPUT_LEASE_TTL_MS,
        },
        OutputControlActionV2::RelinquishOutputLease { lease } => {
            OutputLeaseRequestAction::Relinquish {
                lease_id: OutputLeaseId::decode(&lease.lease_id)
                    .map_err(|error| format!("Output lease authority is invalid: {error:?}"))?,
                owner,
                expected_generation: lease.generation,
            }
        }
        OutputControlActionV2::ForceTransferLease { lease } => {
            OutputLeaseRequestAction::ForceTransfer {
                lease_id: OutputLeaseId::decode(&lease.lease_id)
                    .map_err(|error| format!("Output lease authority is invalid: {error:?}"))?,
                expected_generation: lease.generation,
                new_owner: owner,
                ttl_ms: MAX_OUTPUT_LEASE_TTL_MS,
            }
        }
        _ => return Err("Output lease lifecycle action is required".to_string()),
    };
    let lease_request = OutputLeaseRequest::from_action(
        binding.principal.clone(),
        "output-control",
        request.request_id,
        action,
    )
    .map_err(|error| format!("Output lease lifecycle request is invalid: {error:?}"))?;
    Ok((lease_request, now_ms))
}

pub(crate) fn execute_output_lease_lifecycle_for_operation(
    window: &WebviewWindow,
    state: &AppState,
    query_state: &ControlPlaneQueryState,
    expected_operation_id: &'static str,
    request: OutputControlCommandRequestV2,
) -> OutputControlResponseV2 {
    execute_output_lease_lifecycle_with_confirmation(
        window,
        state,
        query_state,
        expected_operation_id,
        request,
        |action| confirm_native_dangerous_output_action(window, action),
    )
}

fn execute_output_lease_lifecycle_with_confirmation<F>(
    window: &WebviewWindow,
    state: &AppState,
    query_state: &ControlPlaneQueryState,
    expected_operation_id: &'static str,
    request: OutputControlCommandRequestV2,
    confirmation: F,
) -> OutputControlResponseV2
where
    F: Fn(&OutputControlActionV2) -> bool,
{
    if request.operation_id != expected_operation_id
        || request.action.operation_id() != expected_operation_id
        || request.validate().is_err()
    {
        return output_control_rejection_for_operation(
            &request,
            expected_operation_id,
            OutputControlErrorCodeV2::InvalidRequest,
        );
    }
    let shape_sha256 = match request.canonical_shape_bytes() {
        Ok(bytes) => hex_sha256(&bytes),
        Err(_) => {
            return output_control_rejection_for_operation(
                &request,
                expected_operation_id,
                OutputControlErrorCodeV2::InvalidRequest,
            )
        }
    };
    let argument_fingerprint = match request.argument_fingerprint_bytes() {
        Ok(bytes) => hex_sha256(&bytes),
        Err(_) => {
            return output_control_rejection_for_operation(
                &request,
                expected_operation_id,
                OutputControlErrorCodeV2::InvalidRequest,
            )
        }
    };
    let owner_rotation = match state.project_transaction_owner_rotation.lock() {
        Ok(guard) => guard,
        Err(_) => return output_control_rejection(&request, OutputControlErrorCodeV2::Internal),
    };
    let binding = match capture_binding(state, window.label()) {
        Ok(binding) => binding,
        Err(_) => return output_control_rejection(&request, OutputControlErrorCodeV2::Forbidden),
    };
    if let Err(code) = state
        .runtime_control_plane
        .reserve_output_control_request_identity(
            &binding,
            request.action.operation_id(),
            request.request_id,
            &shape_sha256,
            Instant::now(),
        )
    {
        return output_control_rejection_for_operation(&request, expected_operation_id, code);
    }
    let key = output_control_receipt_key(&request, &binding);
    let lane = match state.runtime_control_plane.reserve_output_control_lane(
        &key,
        &shape_sha256,
        Instant::now(),
    ) {
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
            return output_control_rejection(&request, OutputControlErrorCodeV2::Internal);
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
            OutputControlLaneReservation::Lane(_) => {
                output_control_rejection(&request, OutputControlErrorCodeV2::Internal)
            }
        };
    }
    // The dangerous Force Transfer path must not prompt for a request which
    // is already stale or whose lease transition cannot be admitted. Repeat
    // the same checks after the dialog before the authoritative commit.
    if query_state
        .validate_output_control_fence_window(
            window.label(),
            &request.expected_fence,
            binding.owner_incarnation,
        )
        .is_err()
    {
        return retain_output_control_rejection(
            state,
            key,
            shape_sha256,
            output_control_rejection(&request, OutputControlErrorCodeV2::StaleFence),
        );
    }
    let (confirmation_lease_request, confirmation_lease_now_ms) =
        match build_output_lease_lifecycle_request(state, &binding, &request) {
            Ok(request) => request,
            Err(_) => {
                return retain_output_control_rejection(
                    state,
                    key,
                    shape_sha256,
                    output_control_rejection(&request, OutputControlErrorCodeV2::InvalidRequest),
                );
            }
        };
    {
        let registry = match state.output_lease_registry.lock() {
            Ok(registry) => registry,
            Err(_) => {
                return retain_output_control_rejection(
                    state,
                    key,
                    shape_sha256,
                    output_control_rejection(&request, OutputControlErrorCodeV2::Internal),
                );
            }
        };
        if let Err(error) =
            registry.preflight_request(&confirmation_lease_request, confirmation_lease_now_ms)
        {
            return retain_output_control_rejection(
                state,
                key,
                shape_sha256,
                output_control_rejection(&request, output_lease_error_code(error)),
            );
        }
    }
    {
        let _preconfirmation_external_admission =
            match lock_project_external_command_admission(state) {
                Ok(guard) => guard,
                Err(_) => {
                    return retain_output_control_rejection(
                        state,
                        key,
                        shape_sha256,
                        output_control_rejection(&request, OutputControlErrorCodeV2::Busy),
                    );
                }
            };
        let mut preconfirmation_coordinator = match lock_project_coordinator(state) {
            Ok(coordinator) => coordinator,
            Err(_) => {
                return retain_output_control_rejection(
                    state,
                    key,
                    shape_sha256,
                    output_control_rejection(&request, OutputControlErrorCodeV2::Busy),
                );
            }
        };
        if reconcile_project_checkpoint_for_coordinator(state, &mut preconfirmation_coordinator)
            .is_err()
            || !exact_output_control_fence_matches(
                state,
                &preconfirmation_coordinator,
                &request.expected_fence,
            )
            || ensure_no_pending_project_transaction(&preconfirmation_coordinator).is_err()
            || ensure_project_operator_video_clip_slot_runtime_allowed(
                state,
                &preconfirmation_coordinator,
                &binding.principal,
            )
            .is_err()
        {
            return retain_output_control_rejection(
                state,
                key,
                shape_sha256,
                output_control_rejection(&request, OutputControlErrorCodeV2::StaleFence),
            );
        }
    }
    drop(owner_rotation);
    if let Err(code) = output_confirmation_gate(&request.action, &confirmation) {
        let response = output_control_rejection(&request, code);
        state.runtime_control_plane.store_output_control_terminal(
            key,
            shape_sha256,
            response.clone(),
            Instant::now(),
        );
        return response;
    }
    let _owner_rotation = match state.project_transaction_owner_rotation.lock() {
        Ok(guard) => guard,
        Err(_) => {
            state
                .runtime_control_plane
                .release_output_control_lane(&key);
            return output_control_rejection(&request, OutputControlErrorCodeV2::Internal);
        }
    };
    if capture_binding(state, window.label()).ok().as_ref() != Some(&binding) {
        let response = output_control_rejection(&request, OutputControlErrorCodeV2::Forbidden);
        state.runtime_control_plane.store_output_control_terminal(
            key,
            shape_sha256,
            response.clone(),
            Instant::now(),
        );
        return response;
    }
    if query_state
        .validate_output_control_fence_window(
            window.label(),
            &request.expected_fence,
            binding.owner_incarnation,
        )
        .is_err()
    {
        return retain_output_control_rejection(
            state,
            key,
            shape_sha256,
            output_control_rejection(&request, OutputControlErrorCodeV2::StaleFence),
        );
    }
    let _lifecycle_guard = match state.standby_sync_lifecycle.lock() {
        Ok(guard) => guard,
        Err(_) => {
            state
                .runtime_control_plane
                .release_output_control_lane(&key);
            return output_control_rejection(&request, OutputControlErrorCodeV2::Internal);
        }
    };
    let external_admission = match lock_project_external_command_admission(state) {
        Ok(guard) => guard,
        Err(_) => {
            state
                .runtime_control_plane
                .release_output_control_lane(&key);
            return output_control_rejection(&request, OutputControlErrorCodeV2::Busy);
        }
    };
    let mut coordinator = match lock_project_coordinator(state) {
        Ok(coordinator) => coordinator,
        Err(_) => {
            state
                .runtime_control_plane
                .release_output_control_lane(&key);
            return output_control_rejection(&request, OutputControlErrorCodeV2::Busy);
        }
    };
    if reconcile_project_checkpoint_for_coordinator(state, &mut coordinator).is_err()
        || !exact_output_control_fence_matches(state, &coordinator, &request.expected_fence)
        || ensure_no_pending_project_transaction(&coordinator).is_err()
        || ensure_project_operator_video_clip_slot_runtime_allowed(
            state,
            &coordinator,
            &binding.principal,
        )
        .is_err()
    {
        return retain_output_control_rejection(
            state,
            key,
            shape_sha256,
            output_control_rejection(&request, OutputControlErrorCodeV2::StaleFence),
        );
    }
    let (lease_request, lease_now_ms) =
        match build_output_lease_lifecycle_request(state, &binding, &request) {
            Ok(request) => request,
            Err(_) => {
                return retain_output_control_rejection(
                    state,
                    key,
                    shape_sha256,
                    output_control_rejection(&request, OutputControlErrorCodeV2::InvalidRequest),
                );
            }
        };
    {
        let registry = match state.output_lease_registry.lock() {
            Ok(registry) => registry,
            Err(_) => {
                return retain_output_control_rejection(
                    state,
                    key,
                    shape_sha256,
                    output_control_rejection(&request, OutputControlErrorCodeV2::Internal),
                );
            }
        };
        if let Err(error) = registry.preflight_request(&lease_request, lease_now_ms) {
            return retain_output_control_rejection(
                state,
                key,
                shape_sha256,
                output_control_rejection(&request, output_lease_error_code(error)),
            );
        }
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
                return retain_output_control_rejection(
                    state,
                    key,
                    shape_sha256,
                    output_control_rejection(&request, code),
                );
            }
        };
    let require_transition = matches!(
        request.action,
        OutputControlActionV2::ForceTransferLease { .. }
    );
    let receipt = match super::submit_output_lease_lifecycle_request(
        state,
        &lease_request,
        lease_now_ms,
        require_transition,
    ) {
        Ok(receipt) => receipt,
        Err(error) => {
            state
                .runtime_control_plane
                .finish_output_control_inflight(&inflight);
            let response = output_control_rejection(&request, output_lease_error_code(error));
            state.runtime_control_plane.store_output_control_terminal(
                key,
                shape_sha256,
                response.clone(),
                Instant::now(),
            );
            return response;
        }
    };
    drop(coordinator);
    drop(external_admission);
    if let Err(error) = receipt.outcome.clone() {
        state
            .runtime_control_plane
            .finish_output_control_inflight(&inflight);
        let response = output_control_rejection(&request, output_lease_error_code(error));
        state.runtime_control_plane.store_output_control_terminal(
            key,
            shape_sha256,
            response.clone(),
            Instant::now(),
        );
        return response;
    }
    let lease_result =
        match output_control_lease_result_from_registry_receipt(&request.action, &receipt) {
            Ok(result) => result,
            Err(_) => {
                state
                    .runtime_control_plane
                    .finish_output_control_inflight(&inflight);
                let response =
                    output_control_rejection(&request, OutputControlErrorCodeV2::Internal);
                state.runtime_control_plane.store_output_control_terminal(
                    key,
                    shape_sha256,
                    response.clone(),
                    Instant::now(),
                );
                return response;
            }
        };
    let response = OutputControlResponseV2::Receipt(Box::new(OutputControlReceiptV2 {
        operation_id: request.operation_id.clone(),
        request_id: request.request_id,
        shape_sha256: shape_sha256.clone(),
        argument_fingerprint,
        audit_sequence,
        fence_before: request.expected_fence.clone(),
        fence_after: request.expected_fence,
        outcome: OutputControlReceiptOutcomeV2::NoOp,
        lease_result: Some(lease_result),
    }));
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

#[cfg(test)]
fn standby_takeover_selector(
    action: &OutputControlActionV2,
) -> Result<StandbyTakeoverCheckpointSelector, String> {
    match action {
        OutputControlActionV2::TakeOverStandby {
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

#[cfg(test)]
fn execute_standby_takeover_action<F, T>(
    action: &OutputControlActionV2,
    expected_fence: &OutputControlFenceV1,
    execute_core: F,
) -> Result<(bool, OutputControlFenceV1), String>
where
    F: FnOnce(
        bool,
        StandbyTakeoverCheckpointSelector,
        &OutputControlFenceV1,
    ) -> Result<(T, OutputControlFenceV1), String>,
{
    let OutputControlActionV2::TakeOverStandby { force, .. } = action else {
        return Err("Output control action is not Standby Take Over".to_string());
    };
    let selector = standby_takeover_selector(action)?;
    execute_core(*force, selector, expected_fence)
        .map(|(_load_result, fence_after)| (true, fence_after))
}

pub(crate) fn output_control_executor_failure(
    request: &OutputControlCommandRequestV2,
) -> OutputControlResponseV2 {
    output_control_rejection(request, OutputControlErrorCodeV2::Internal)
}

fn output_control_rejection(
    request: &OutputControlCommandRequestV2,
    error: OutputControlErrorCodeV2,
) -> OutputControlResponseV2 {
    output_control_rejection_for_operation(request, request.action.operation_id(), error)
}

fn output_control_rejection_for_operation(
    request: &OutputControlCommandRequestV2,
    operation_id: &str,
    error: OutputControlErrorCodeV2,
) -> OutputControlResponseV2 {
    OutputControlResponseV2::Rejected(OutputControlRejectionV2 {
        operation_id: operation_id.to_string(),
        request_id: request.request_id.clamp(1, MAX_SAFE_JAVASCRIPT_INTEGER),
        error,
    })
}

fn retain_output_control_rejection(
    state: &AppState,
    key: OutputControlReceiptKey,
    shape_sha256: String,
    response: OutputControlResponseV2,
) -> OutputControlResponseV2 {
    state.runtime_control_plane.store_output_control_terminal(
        key,
        shape_sha256,
        response.clone(),
        Instant::now(),
    );
    response
}

pub(crate) fn output_control_lease_result_from_registry_receipt(
    action: &OutputControlActionV2,
    receipt: &OutputLeaseRequestReceipt,
) -> Result<OutputControlLeaseResultV2, String> {
    let outcome = receipt
        .outcome
        .as_ref()
        .map(|outcome| match outcome {
            OutputLeaseOperationOutcome::Acquired => OutputLeaseReceiptOutcomeV2::Acquired,
            OutputLeaseOperationOutcome::Renewed => OutputLeaseReceiptOutcomeV2::Renewed,
            #[cfg(test)]
            OutputLeaseOperationOutcome::ExpiryObserved => {
                OutputLeaseReceiptOutcomeV2::ExpiryObserved
            }
            OutputLeaseOperationOutcome::Recovered => OutputLeaseReceiptOutcomeV2::Recovered,
            OutputLeaseOperationOutcome::Relinquished => OutputLeaseReceiptOutcomeV2::Relinquished,
            OutputLeaseOperationOutcome::Transferred => OutputLeaseReceiptOutcomeV2::Transferred,
            OutputLeaseOperationOutcome::OwnerRetired => OutputLeaseReceiptOutcomeV2::OwnerRetired,
            OutputLeaseOperationOutcome::ProjectOrphaned => {
                OutputLeaseReceiptOutcomeV2::ProjectOrphaned
            }
            OutputLeaseOperationOutcome::Authorized => OutputLeaseReceiptOutcomeV2::Authorized,
        })
        .map_err(|error| format!("output lease operation did not succeed: {error:?}"))?;
    let expected_outcome = match action {
        OutputControlActionV2::EnableOutput => OutputLeaseReceiptOutcomeV2::Acquired,
        OutputControlActionV2::Arm { .. }
        | OutputControlActionV2::EnableShowArtNetLoopbackRoute { .. }
        | OutputControlActionV2::ReleaseBlackout { .. }
        | OutputControlActionV2::TakeOverStandby { .. }
        | OutputControlActionV2::AddDisplay { .. }
        | OutputControlActionV2::AssignVideoOutputComposition { .. }
        | OutputControlActionV2::SetDisplayWindowOpen { .. } => {
            OutputLeaseReceiptOutcomeV2::Authorized
        }
        OutputControlActionV2::AcquireLease { .. } => OutputLeaseReceiptOutcomeV2::Acquired,
        OutputControlActionV2::RenewLease { .. } => OutputLeaseReceiptOutcomeV2::Renewed,
        OutputControlActionV2::RecoverLease { .. } => OutputLeaseReceiptOutcomeV2::Recovered,
        OutputControlActionV2::RelinquishOutputLease { .. } => {
            OutputLeaseReceiptOutcomeV2::Relinquished
        }
        OutputControlActionV2::ForceTransferLease { .. } => {
            OutputLeaseReceiptOutcomeV2::Transferred
        }
    };
    let enable_outcome = matches!(action, OutputControlActionV2::EnableOutput)
        && matches!(
            outcome,
            OutputLeaseReceiptOutcomeV2::Acquired | OutputLeaseReceiptOutcomeV2::Recovered
        );
    let takeover_project_orphan = matches!(action, OutputControlActionV2::TakeOverStandby { .. })
        && outcome == OutputLeaseReceiptOutcomeV2::ProjectOrphaned;
    if outcome != expected_outcome && !enable_outcome && !takeover_project_orphan {
        return Err("output lease receipt outcome does not match operation".to_string());
    }
    let lease_id = receipt
        .lease_id
        .ok_or_else(|| "output lease receipt omitted its lease id".to_string())?;
    let generation = receipt
        .generation_after
        .or(receipt.generation_before)
        .ok_or_else(|| "output lease receipt omitted its generation".to_string())?;
    let resources = receipt
        .resources
        .as_ref()
        .ok_or_else(|| "output lease receipt omitted its resources".to_string())?
        .as_slice()
        .iter()
        .map(|resource| match resource {
            OutputLeaseResource::Lighting => {
                protocol::control_plane_command::OutputControlTargetRoleV1::Lighting
            }
            OutputLeaseResource::Video => {
                protocol::control_plane_command::OutputControlTargetRoleV1::Video
            }
        })
        .collect::<Vec<_>>();
    let phase_for_snapshot =
        |snapshot: &super::output_lease::OutputLeaseSnapshot| match snapshot.phase {
            OutputLeasePhase::Unclaimed => OutputLeaseReceiptPhaseV2::Unclaimed,
            OutputLeasePhase::HeldActive => OutputLeaseReceiptPhaseV2::HeldActive,
            OutputLeasePhase::HeldOrphaned => OutputLeaseReceiptPhaseV2::HeldOrphaned,
        };
    let resources_for_snapshot = |snapshot: &super::output_lease::OutputLeaseSnapshot| -> Vec<_> {
        snapshot
            .resources
            .as_ref()
            .map(|resources| {
                resources
                    .as_slice()
                    .iter()
                    .map(|resource| match resource {
                        OutputLeaseResource::Lighting => {
                            protocol::control_plane_command::OutputControlTargetRoleV1::Lighting
                        }
                        OutputLeaseResource::Video => {
                            protocol::control_plane_command::OutputControlTargetRoleV1::Video
                        }
                    })
                    .collect()
            })
            .unwrap_or_default()
    };
    if receipt.changes.is_empty() {
        return Err("output lease receipt omitted its change truth".to_string());
    }
    let changes = receipt
        .changes
        .iter()
        .map(|change| OutputLeaseReceiptChangeV2 {
            lease_id: change.lease_id.encode(),
            before_generation: change.before.as_ref().map(|snapshot| snapshot.generation),
            after_generation: change.after.as_ref().map(|snapshot| snapshot.generation),
            before_resources: change
                .before
                .as_ref()
                .map(resources_for_snapshot)
                .unwrap_or_default(),
            after_resources: change
                .after
                .as_ref()
                .map(resources_for_snapshot)
                .unwrap_or_default(),
            before_phase: change.before.as_ref().map(phase_for_snapshot),
            after_phase: change.after.as_ref().map(phase_for_snapshot),
        })
        .collect::<Vec<_>>();
    let last_change = receipt
        .changes
        .last()
        .ok_or_else(|| "output lease receipt omitted its final change".to_string())?;
    let phase = last_change
        .after
        .as_ref()
        .or(last_change.before.as_ref())
        .map(phase_for_snapshot)
        .ok_or_else(|| "output lease receipt omitted its final phase".to_string())?;
    let result = OutputControlLeaseResultV2 {
        authority: protocol::control_plane_command::OutputLeaseAuthorityV1 {
            lease_id: lease_id.encode(),
            generation,
        },
        resources,
        phase,
        outcome,
        audit_sequence: receipt.audit_sequence,
        changes,
    };
    result
        .validate()
        .map_err(|error| format!("output lease receipt failed protocol validation: {error}"))?;
    Ok(result)
}

fn output_control_receipt_key(
    request: &OutputControlCommandRequestV2,
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

pub(crate) fn hex_sha256(bytes: &[u8]) -> String {
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct OutputControlRequestIdentityKey {
    principal: String,
    window_label: String,
    owner_incarnation: u64,
    request_id: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct OutputControlRequestOriginKey {
    principal: String,
    window_label: String,
    owner_incarnation: u64,
}

#[derive(Debug, Clone)]
struct OutputControlRequestIdentityRecord {
    operation_id: String,
    shape_sha256: String,
    expires_at: Instant,
}

#[derive(Debug, Clone)]
struct OutputControlTerminalRecord {
    shape_sha256: String,
    response: OutputControlResponseV2,
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
    common_request_identities:
        HashMap<OutputControlRequestIdentityKey, OutputControlRequestIdentityRecord>,
    request_id_high_water: HashMap<OutputControlRequestOriginKey, u64>,
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
    Terminal(OutputControlResponseV2),
    Rejected(OutputControlErrorCodeV2),
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

    /// Reserve the operation-independent caller/incarnation/request identity
    /// before any fence, local-confirmation, rate, audit, registry, or physical work.
    /// The operation-specific lane remains responsible for exact terminal
    /// replay; this bounded identity map prevents a different operation from
    /// laundering the same request id after a terminal is evicted.
    fn reserve_output_control_request_identity(
        &self,
        binding: &CallerBinding,
        operation_id: &str,
        request_id: u64,
        shape_sha256: &str,
        now: Instant,
    ) -> Result<(), OutputControlErrorCodeV2> {
        let mut inner = match self.output_control.lock() {
            Ok(inner) => inner,
            Err(poisoned) => {
                // A poisoned bookkeeping mutex cannot safely admit new work.
                // An intact, already-recorded terminal may still be replayed;
                // no new identity, rate token, lane, audit, or authority work
                // is permitted on this path.
                let inner = poisoned.into_inner();
                let identity_key = OutputControlRequestIdentityKey {
                    principal: binding.principal.clone(),
                    window_label: binding.window_label.clone(),
                    owner_incarnation: binding.owner_incarnation,
                    request_id,
                };
                let operation_key = OutputControlReceiptKey {
                    principal: binding.principal.clone(),
                    window_label: binding.window_label.clone(),
                    owner_incarnation: binding.owner_incarnation,
                    operation_id: operation_id.to_string(),
                    request_id,
                };
                let Some(identity) = inner.common_request_identities.get(&identity_key) else {
                    return Err(OutputControlErrorCodeV2::Internal);
                };
                if identity.operation_id != operation_id || identity.shape_sha256 != shape_sha256 {
                    return Err(OutputControlErrorCodeV2::InvalidRequest);
                }
                let Some(receipt) = inner.receipts.get(&operation_key) else {
                    return Err(OutputControlErrorCodeV2::Internal);
                };
                if receipt.shape_sha256 == shape_sha256 {
                    return Ok(());
                }
                return Err(OutputControlErrorCodeV2::InvalidRequest);
            }
        };
        purge_output_control_expired(&mut inner, now);
        let identity_key = OutputControlRequestIdentityKey {
            principal: binding.principal.clone(),
            window_label: binding.window_label.clone(),
            owner_incarnation: binding.owner_incarnation,
            request_id,
        };
        let operation_key = OutputControlReceiptKey {
            principal: binding.principal.clone(),
            window_label: binding.window_label.clone(),
            owner_incarnation: binding.owner_incarnation,
            operation_id: operation_id.to_string(),
            request_id,
        };
        if let Some(record) = inner.common_request_identities.get_mut(&identity_key) {
            if record.operation_id != operation_id || record.shape_sha256 != shape_sha256 {
                return Err(OutputControlErrorCodeV2::InvalidRequest);
            }
            if inner.receipts.contains_key(&operation_key) {
                return Ok(());
            }
            if inner.lanes.contains_key(&operation_key) {
                return Err(OutputControlErrorCodeV2::Busy);
            }
            // The identity survived terminal eviction but its receipt did
            // not. Do not re-execute an old request id.
            return Err(OutputControlErrorCodeV2::InvalidRequest);
        }
        let origin_key = OutputControlRequestOriginKey {
            principal: binding.principal.clone(),
            window_label: binding.window_label.clone(),
            owner_incarnation: binding.owner_incarnation,
        };
        if inner
            .request_id_high_water
            .get(&origin_key)
            .is_some_and(|high_water| request_id <= *high_water)
        {
            return Err(OutputControlErrorCodeV2::InvalidRequest);
        }
        if inner.common_request_identities.len() >= MAX_TOTAL_OUTPUT_CONTROL_IDENTITIES {
            return Err(OutputControlErrorCodeV2::Overloaded);
        }
        if !inner.request_id_high_water.contains_key(&origin_key)
            && inner.request_id_high_water.len() >= MAX_TOTAL_OUTPUT_CONTROL_IDENTITIES
        {
            return Err(OutputControlErrorCodeV2::Overloaded);
        }
        inner.request_id_high_water.insert(origin_key, request_id);
        inner.common_request_identities.insert(
            identity_key,
            OutputControlRequestIdentityRecord {
                operation_id: operation_id.to_string(),
                shape_sha256: shape_sha256.to_string(),
                expires_at: now + TOMBSTONE_TTL,
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
        let mut inner = match self.output_control.lock() {
            Ok(inner) => inner,
            Err(poisoned) => {
                let inner = poisoned.into_inner();
                if let Some(record) = inner.receipts.get(key) {
                    return if record.shape_sha256 == shape_sha256 {
                        OutputControlLaneReservation::Terminal(record.response.clone())
                    } else {
                        OutputControlLaneReservation::Rejected(
                            OutputControlErrorCodeV2::InvalidRequest,
                        )
                    };
                }
                return OutputControlLaneReservation::Rejected(OutputControlErrorCodeV2::Internal);
            }
        };
        purge_output_control_expired(&mut inner, now);
        let last_used = next_output_control_sequence(&mut inner);
        if let Some(record) = inner.receipts.get_mut(key) {
            record.last_used = last_used;
            return if record.shape_sha256 == shape_sha256 {
                OutputControlLaneReservation::Terminal(record.response.clone())
            } else {
                OutputControlLaneReservation::Rejected(OutputControlErrorCodeV2::InvalidRequest)
            };
        }
        if let Some(tombstone) = inner.tombstones.get_mut(key) {
            tombstone.last_used = last_used;
            return OutputControlLaneReservation::Rejected(
                OutputControlErrorCodeV2::InvalidRequest,
            );
        }
        if let Some(lane) = inner.lanes.get(key) {
            return OutputControlLaneReservation::Lane(Arc::clone(lane));
        }
        if inner.lanes.len() >= MAX_LANES {
            return OutputControlLaneReservation::Rejected(OutputControlErrorCodeV2::Overloaded);
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
        let mut inner = match self.output_control.lock() {
            Ok(inner) => inner,
            Err(poisoned) => {
                let inner = poisoned.into_inner();
                if let Some(record) = inner.receipts.get(key) {
                    return Some(if record.shape_sha256 == shape_sha256 {
                        OutputControlLaneReservation::Terminal(record.response.clone())
                    } else {
                        OutputControlLaneReservation::Rejected(
                            OutputControlErrorCodeV2::InvalidRequest,
                        )
                    });
                }
                return Some(OutputControlLaneReservation::Rejected(
                    OutputControlErrorCodeV2::Internal,
                ));
            }
        };
        purge_output_control_expired(&mut inner, now);
        let last_used = next_output_control_sequence(&mut inner);
        if let Some(record) = inner.receipts.get_mut(key) {
            record.last_used = last_used;
            return Some(if record.shape_sha256 == shape_sha256 {
                OutputControlLaneReservation::Terminal(record.response.clone())
            } else {
                OutputControlLaneReservation::Rejected(OutputControlErrorCodeV2::InvalidRequest)
            });
        }
        if let Some(tombstone) = inner.tombstones.get_mut(key) {
            tombstone.last_used = last_used;
            return Some(OutputControlLaneReservation::Rejected(
                OutputControlErrorCodeV2::InvalidRequest,
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
    ) -> Result<(PrincipalDomainKey, u64), OutputControlErrorCodeV2> {
        let mut inner = self
            .output_control
            .lock()
            .map_err(|_| OutputControlErrorCodeV2::Internal)?;
        purge_output_control_expired(&mut inner, now);
        if inner.audit.len() >= MAX_SAFETY_AUDIT_RECORDS {
            return Err(OutputControlErrorCodeV2::Overloaded);
        }
        let domain_key = PrincipalDomainKey {
            principal: binding.principal.clone(),
            domain: key.operation_id.clone(),
        };
        if inner.inflight.contains(&domain_key) {
            return Err(OutputControlErrorCodeV2::Busy);
        }
        let bucket = inner
            .token_buckets
            .entry(domain_key.clone())
            .or_insert_with(|| TokenBucket::fresh(now));
        if !bucket.try_take(now) {
            return Err(OutputControlErrorCodeV2::Overloaded);
        }
        let audit_sequence = inner
            .audit_sequence
            .checked_add(1)
            .filter(|sequence| *sequence <= MAX_SAFE_JAVASCRIPT_INTEGER)
            .ok_or(OutputControlErrorCodeV2::Overloaded)?;
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
        if let Ok(mut inner) = self.output_control.lock() {
            inner.inflight.remove(key);
        }
    }

    fn store_output_control_terminal(
        &self,
        key: OutputControlReceiptKey,
        shape_sha256: String,
        response: OutputControlResponseV2,
        now: Instant,
    ) {
        let Ok(mut inner) = self.output_control.lock() else {
            return;
        };
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
        if let Ok(mut inner) = self.output_control.lock() {
            inner.lanes.remove(key);
        }
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
        let Ok(mut output_control) = self.output_control.lock() else {
            return;
        };
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
        output_control
            .common_request_identities
            .retain(|key, _| key.principal != principal);
        output_control
            .request_id_high_water
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
    inner
        .common_request_identities
        .retain(|_, record| record.expires_at > now);
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
                safety_blackout_rejection(&request, RuntimeCommandErrorCodeV1::Internal)
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
    let binding = capture_binding(state, window.label()).map_err(RuntimeCommandErrorV1::new)?;
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
            LaneReservation::Lane(_) => rejection(&request, RuntimeCommandErrorCodeV1::Internal),
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
                follow_abort_rejection(&request, RuntimeCommandErrorCodeV1::Internal)
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
    state
        .ensure_window_authority_not_blocked(window_label)
        .map_err(|_| RuntimeCommandErrorCodeV1::Forbidden)?;
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

#[cfg(test)]
pub(crate) fn capture_binding_for_test(
    state: &AppState,
    window_label: &str,
) -> Result<(), RuntimeCommandErrorCodeV1> {
    capture_binding(state, window_label).map(|_| ())
}

#[cfg(test)]
pub(crate) fn test_state_signature(state: &RuntimeControlPlaneState) -> String {
    let inner = state
        .inner
        .lock()
        .expect("runtime control-plane state lock");
    let follow_abort = state
        .follow_abort
        .lock()
        .expect("runtime Follow abort state lock");
    let safety_blackout = state
        .safety_blackout
        .lock()
        .expect("runtime safety blackout state lock");
    let output_control = state
        .output_control
        .lock()
        .expect("runtime output-control state lock");
    format!("{inner:?}|{follow_abort:?}|{safety_blackout:?}|{output_control:?}")
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
    use crate::output_lease::OutputLeaseRegistry;
    use protocol::control_plane_command::{
        OutputControlActionV2, OutputLeaseAuthorityV1, ProjectMutationFenceV1,
        SetTimelinePlayingRuntimePayloadV1, OUTPUT_BLACKOUT_RELEASE_OPERATION_ID,
        OUTPUT_LEASE_RENEW_OPERATION_ID, OUTPUT_OWNERSHIP_ARM_OPERATION_ID,
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
        error: OutputControlErrorCodeV2,
    ) -> OutputControlResponseV2 {
        OutputControlResponseV2::Rejected(OutputControlRejectionV2 {
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
    fn native_danger_confirmation_copy_follows_the_os_locale() {
        assert_eq!(
            native_output_confirmation_copy_for_locale("ja-JP"),
            (
                "出力制御の確認",
                "この詳細操作は、ライブ出力の所有権または出力状態を変更します。続行しますか？",
            )
        );
        assert_eq!(
            native_output_confirmation_copy_for_locale("en-US"),
            (
                "Confirm output control",
                "This advanced output operation changes live ownership or output state. Continue?",
            )
        );
    }

    #[test]
    fn editor_display_confirmation_copy_is_distinct_from_ordinary_output_copy() {
        let ordinary = native_output_confirmation_copy_for_editor_target(false);
        let editor = native_output_confirmation_copy_for_editor_target(true);
        assert_ne!(ordinary, editor);
        assert!(editor.0.contains("editor") || editor.0.contains("エディタ"));
        assert!(editor.1.contains("display") || editor.1.contains("ディスプレイ"));
    }

    #[test]
    fn native_danger_confirmation_gate_is_exact_and_terminal_replay_is_dialog_free() {
        let calls = std::sync::atomic::AtomicUsize::new(0);
        let deny = |_: &OutputControlActionV2| {
            calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            false
        };
        assert!(output_confirmation_gate(&OutputControlActionV2::EnableOutput, &deny).is_ok());
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 0);

        let dangerous = OutputControlActionV2::ReleaseBlackout {
            lease: OutputLeaseAuthorityV1 {
                lease_id: "lease-0000000000000001".to_string(),
                generation: 1,
            },
        };
        assert_eq!(
            output_confirmation_gate(&dangerous, &deny),
            Err(OutputControlErrorCodeV2::Forbidden)
        );
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);

        let show_artnet_route = OutputControlActionV2::EnableShowArtNetLoopbackRoute {
            lease: OutputLeaseAuthorityV1 {
                lease_id: "lease-0000000000000001".to_string(),
                generation: 1,
            },
        };
        assert_eq!(
            output_confirmation_gate(&show_artnet_route, &deny),
            Err(OutputControlErrorCodeV2::Forbidden),
            "the Art-Net loopback show route can never bypass the local native R4 confirmation"
        );
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 2);

        let accept_calls = std::sync::atomic::AtomicUsize::new(0);
        let accept = |_: &OutputControlActionV2| {
            accept_calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            true
        };
        assert!(output_confirmation_gate(&dangerous, &accept).is_ok());
        assert_eq!(accept_calls.load(std::sync::atomic::Ordering::SeqCst), 1);

        // A cancelled operation is retained as a terminal Forbidden result;
        // an exact retry therefore returns without invoking confirmation or
        // entering any engine/lease mutation path.
        let state = RuntimeControlPlaneState::default();
        let binding = test_binding("dialog-replay", "main", 3);
        let key = test_output_key(
            &binding.principal,
            OUTPUT_BLACKOUT_RELEASE_OPERATION_ID,
            91,
            binding.owner_incarnation,
        );
        let shape = "a".repeat(64);
        let now = Instant::now();
        state
            .reserve_output_control_request_identity(
                &binding,
                &key.operation_id,
                key.request_id,
                &shape,
                now,
            )
            .unwrap();
        assert!(matches!(
            state.reserve_output_control_lane(&key, &shape, now),
            OutputControlLaneReservation::Lane(_)
        ));
        let cancelled = test_output_rejection(&key, OutputControlErrorCodeV2::Forbidden);
        state.store_output_control_terminal(key.clone(), shape.clone(), cancelled.clone(), now);

        let retry_calls = std::sync::atomic::AtomicUsize::new(0);
        let retry_confirmation = |_: &OutputControlActionV2| {
            retry_calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            panic!("terminal retry must not prompt");
        };
        state
            .reserve_output_control_request_identity(
                &binding,
                &key.operation_id,
                key.request_id,
                &shape,
                now,
            )
            .unwrap();
        assert!(matches!(
            state.reserve_output_control_lane(&key, &shape, now),
            OutputControlLaneReservation::Terminal(response) if response == cancelled
        ));
        assert!(output_confirmation_gate(
            &OutputControlActionV2::EnableOutput,
            &retry_confirmation
        )
        .is_ok());
        assert_eq!(retry_calls.load(std::sync::atomic::Ordering::SeqCst), 0);
    }

    #[test]
    fn output_control_successor_is_preflighted_before_commit_and_receipt_is_infallible() {
        let basis = test_output_fence();
        assert_eq!(preflight_output_control_successor(&basis).unwrap(), (7, 8));

        let checkpoint_hash = "b".repeat(64);
        let fence = committed_output_control_fence(
            &basis,
            CommittedOutputControlFenceValues {
                project_epoch: 10,
                project_revision: 11,
                project_checkpoint_hash: &checkpoint_hash,
                project_publication_generation: 12,
                output_epoch: 13,
                output_generation: 14,
                safety_blackout_epoch: 15,
                safety_blackout_generation: 16,
            },
        );
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
        let response = test_output_rejection(&key, OutputControlErrorCodeV2::PublicationFailed);
        let binding = test_binding(&key.principal, &key.window_label, key.owner_incarnation);
        assert!(state
            .reserve_output_control_request_identity(
                &binding,
                &key.operation_id,
                key.request_id,
                &shape,
                now,
            )
            .is_ok());

        assert!(matches!(
            state.reserve_output_control_lane(&key, &shape, now),
            OutputControlLaneReservation::Lane(_)
        ));
        state.store_output_control_terminal(key.clone(), shape.clone(), response.clone(), now);

        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = state.output_control.lock().unwrap();
            panic!("inject output-control bookkeeping poison");
        }));

        assert!(state
            .reserve_output_control_request_identity(
                &binding,
                &key.operation_id,
                key.request_id,
                &shape,
                now,
            )
            .is_ok());
        assert!(matches!(
            state.reserve_output_control_lane(&key, &shape, now),
            OutputControlLaneReservation::Terminal(actual) if actual == response
        ));
    }

    #[test]
    fn output_control_common_identity_reserves_cross_operation_ids_before_work() {
        let state = RuntimeControlPlaneState::default();
        let now = Instant::now();
        let binding = test_binding("renderer-output-identity", "main", 9);
        let request_id = 12_010;
        let arm_key = test_output_key(
            &binding.principal,
            OUTPUT_OWNERSHIP_ARM_OPERATION_ID,
            request_id,
            binding.owner_incarnation,
        );
        let arm_shape = "a".repeat(64);
        let release_shape = "b".repeat(64);

        assert!(state
            .reserve_output_control_request_identity(
                &binding,
                OUTPUT_OWNERSHIP_ARM_OPERATION_ID,
                request_id,
                &arm_shape,
                now,
            )
            .is_ok());
        assert!(matches!(
            state.reserve_output_control_lane(&arm_key, &arm_shape, now),
            OutputControlLaneReservation::Lane(_)
        ));
        assert_eq!(
            state.reserve_output_control_request_identity(
                &binding,
                OUTPUT_OWNERSHIP_ARM_OPERATION_ID,
                request_id,
                &arm_shape,
                now,
            ),
            Err(OutputControlErrorCodeV2::Busy)
        );

        let response = test_output_rejection(&arm_key, OutputControlErrorCodeV2::PublicationFailed);
        state.store_output_control_terminal(
            arm_key.clone(),
            arm_shape.clone(),
            response.clone(),
            now,
        );
        assert!(state
            .reserve_output_control_request_identity(
                &binding,
                OUTPUT_OWNERSHIP_ARM_OPERATION_ID,
                request_id,
                &arm_shape,
                now,
            )
            .is_ok());
        assert!(matches!(
            state.reserve_output_control_lane(&arm_key, &arm_shape, now),
            OutputControlLaneReservation::Terminal(actual) if actual == response
        ));

        let before = {
            let inner = state.output_control.lock().unwrap();
            (
                inner.audit.len(),
                inner.audit_sequence,
                inner.token_buckets.len(),
            )
        };
        assert_eq!(
            state.reserve_output_control_request_identity(
                &binding,
                OUTPUT_BLACKOUT_RELEASE_OPERATION_ID,
                request_id,
                &release_shape,
                now,
            ),
            Err(OutputControlErrorCodeV2::InvalidRequest)
        );
        assert_eq!(
            state.reserve_output_control_request_identity(
                &binding,
                OUTPUT_OWNERSHIP_ARM_OPERATION_ID,
                request_id,
                &release_shape,
                now,
            ),
            Err(OutputControlErrorCodeV2::InvalidRequest)
        );
        let inner = state.output_control.lock().unwrap();
        assert_eq!(
            (
                inner.audit.len(),
                inner.audit_sequence,
                inner.token_buckets.len()
            ),
            before,
            "cross-operation and changed-shape identity conflicts do not admit rate/audit work"
        );
    }

    #[test]
    fn output_control_common_identity_high_water_survives_terminal_and_tombstone_expiry() {
        let state = RuntimeControlPlaneState::default();
        let now = Instant::now();
        let expired_now = now + TOMBSTONE_TTL + Duration::from_millis(1);
        let binding = test_binding("renderer-output-high-water", "main", 19);
        let request_id = 22_010;
        let arm_shape = "c".repeat(64);
        let arm_key = test_output_key(
            &binding.principal,
            OUTPUT_OWNERSHIP_ARM_OPERATION_ID,
            request_id,
            binding.owner_incarnation,
        );

        state
            .reserve_output_control_request_identity(
                &binding,
                OUTPUT_OWNERSHIP_ARM_OPERATION_ID,
                request_id,
                &arm_shape,
                now,
            )
            .expect("first valid envelope reserves the common identity");
        assert!(matches!(
            state.reserve_output_control_lane(&arm_key, &arm_shape, now),
            OutputControlLaneReservation::Lane(_)
        ));
        state.store_output_control_terminal(
            arm_key.clone(),
            arm_shape,
            test_output_rejection(&arm_key, OutputControlErrorCodeV2::StaleFence),
            now,
        );

        let before = {
            let mut inner = state.output_control.lock().unwrap();
            purge_output_control_expired(&mut inner, expired_now);
            assert!(inner.common_request_identities.is_empty());
            (
                inner.lanes.len(),
                inner.audit.len(),
                inner.audit_sequence,
                inner.token_buckets.len(),
                inner.inflight.len(),
                inner.request_id_high_water.clone(),
            )
        };

        for operation_id in [
            OUTPUT_BLACKOUT_RELEASE_OPERATION_ID,
            OUTPUT_LEASE_RENEW_OPERATION_ID,
        ] {
            assert_eq!(
                state.reserve_output_control_request_identity(
                    &binding,
                    operation_id,
                    request_id,
                    &"d".repeat(64),
                    expired_now,
                ),
                Err(OutputControlErrorCodeV2::InvalidRequest),
                "an expired terminal cannot reopen its request id through another operation"
            );
        }
        let after_rejections = {
            let inner = state.output_control.lock().unwrap();
            (
                inner.lanes.len(),
                inner.audit.len(),
                inner.audit_sequence,
                inner.token_buckets.len(),
                inner.inflight.len(),
                inner.request_id_high_water.clone(),
            )
        };
        assert_eq!(after_rejections, before);

        state
            .reserve_output_control_request_identity(
                &binding,
                OUTPUT_LEASE_RENEW_OPERATION_ID,
                request_id + 1,
                &"e".repeat(64),
                expired_now,
            )
            .expect("the next monotonic request id remains admissible");
        let inner = state.output_control.lock().unwrap();
        assert_eq!(
            inner
                .request_id_high_water
                .get(&OutputControlRequestOriginKey {
                    principal: binding.principal,
                    window_label: binding.window_label,
                    owner_incarnation: binding.owner_incarnation,
                }),
            Some(&(request_id + 1))
        );
    }

    #[test]
    fn output_control_takeover_identity_swap_returns_committed_orphan_truth() {
        let mut registry = OutputLeaseRegistry::fresh_process(51).expect("orphan test registry");
        let owner = OutputLeaseOwner::new("orphan-owner", "main", 51, 1).expect("orphan owner");
        let resources =
            OutputLeaseResources::new(&[OutputLeaseResource::Lighting]).expect("orphan resources");
        let acquired = registry
            .submit_request(
                &OutputLeaseRequest::from_action(
                    "orphan-owner",
                    "orphan-test",
                    1,
                    OutputLeaseRequestAction::Acquire {
                        owner: owner.clone(),
                        resources: resources.clone(),
                        project_identity: "project_epoch:3".to_string(),
                        ttl_ms: MAX_OUTPUT_LEASE_TTL_MS,
                    },
                )
                .expect("orphan acquire request"),
                0,
            )
            .expect("orphan acquire");
        let lease_id = acquired.lease_id.expect("orphan lease id");
        let orphan_receipt = registry
            .submit_request(
                &OutputLeaseRequest::from_action(
                    "syndocal.lifecycle",
                    "orphan-test",
                    2,
                    OutputLeaseRequestAction::ProjectOrphan {
                        project_identity: "project_epoch:3".to_string(),
                        selected_lease_ids: Vec::new(),
                        resource_scope: None,
                    },
                )
                .expect("orphan project request"),
                0,
            )
            .expect("orphan transition");
        let selected = crate::select_output_lease_receipt_change(&orphan_receipt, lease_id)
            .expect("selected orphan truth");
        let action = OutputControlActionV2::TakeOverStandby {
            force: true,
            standby_session_id: "standby-a".to_string(),
            standby_generation: 7,
            lease: protocol::control_plane_command::OutputLeaseAuthorityV1 {
                lease_id: lease_id.encode(),
                generation: 1,
            },
        };
        let result = output_control_lease_result_from_registry_receipt(&action, &selected)
            .expect("Take Over response uses final orphan truth");
        assert_eq!(result.outcome, OutputLeaseReceiptOutcomeV2::ProjectOrphaned);
        assert_eq!(result.phase, OutputLeaseReceiptPhaseV2::HeldOrphaned);
        assert_eq!(result.authority.generation, 2);
        assert_eq!(result.changes.len(), 1);
        assert_eq!(result.changes[0].after_generation, Some(2));
        assert_eq!(
            result.changes[0].after_phase,
            Some(OutputLeaseReceiptPhaseV2::HeldOrphaned)
        );
    }

    #[test]
    fn output_control_add_display_authorization_serializes_as_a_valid_terminal_receipt() {
        let mut registry = OutputLeaseRegistry::fresh_process(52).expect("display test registry");
        let owner =
            OutputLeaseOwner::new("display-owner", "main", 52, 1).expect("display test owner");
        let resources =
            OutputLeaseResources::new(&[OutputLeaseResource::Lighting, OutputLeaseResource::Video])
                .expect("display Both resources");
        let acquired = registry
            .submit_request(
                &OutputLeaseRequest::from_action(
                    "display-owner",
                    "display-test",
                    1,
                    OutputLeaseRequestAction::Acquire {
                        owner: owner.clone(),
                        resources: resources.clone(),
                        project_identity: "project_epoch:3".to_string(),
                        ttl_ms: MAX_OUTPUT_LEASE_TTL_MS,
                    },
                )
                .expect("display acquire request"),
                0,
            )
            .expect("display acquire");
        let lease_id = acquired.lease_id.expect("display lease id");
        let generation = acquired.generation_after.expect("display lease generation");
        let authorized = registry
            .submit_request(
                &OutputLeaseRequest::from_action(
                    "display-owner",
                    "display-test",
                    2,
                    OutputLeaseRequestAction::AuthorizeOrdinary {
                        lease_id,
                        owner,
                        expected_generation: generation,
                        exact_resources: resources,
                    },
                )
                .expect("display authorization request"),
                1,
            )
            .expect("display authorization");
        let action = OutputControlActionV2::AddDisplay {
            spec: protocol::control_plane_command::DisplayOutputSpecV2 {
                label: "DISPLAY5".to_string(),
                monitor_identity: "d".repeat(64),
                monitor_index: 3,
                width: 1920,
                height: 1080,
                fullscreen: true,
            },
            lease: OutputLeaseAuthorityV1 {
                lease_id: lease_id.encode(),
                generation,
            },
        };
        let lease_result = output_control_lease_result_from_registry_receipt(&action, &authorized)
            .expect("Display authorization converts to protocol truth");
        let fence_before = test_output_fence();
        let mut fence_after = fence_before.clone();
        fence_after.project_revision += 1;
        fence_after.project_checkpoint_hash = "b".repeat(64);
        fence_after.project_publication_generation += 1;
        fence_after.output_epoch += 1;
        fence_after.output_generation += 1;
        let receipt = OutputControlResponseV2::Receipt(Box::new(OutputControlReceiptV2 {
            operation_id: action.operation_id().to_string(),
            request_id: 2,
            shape_sha256: "c".repeat(64),
            argument_fingerprint: "e".repeat(64),
            audit_sequence: 1,
            fence_before,
            fence_after,
            outcome: OutputControlReceiptOutcomeV2::Applied,
            lease_result: Some(lease_result),
        }));
        serde_json::to_value(&receipt).expect("Display terminal receipt must serialize");
    }

    #[test]
    fn output_control_takeover_passes_exact_standby_identity_to_core() {
        let action = OutputControlActionV2::TakeOverStandby {
            force: true,
            standby_session_id: "primary-a".to_string(),
            standby_generation: 42,
            lease: protocol::control_plane_command::OutputLeaseAuthorityV1 {
                lease_id: "lease-0000000000000001".to_string(),
                generation: 1,
            },
        };
        let expected_fence = test_output_fence();
        let fence_after = OutputControlFenceV1 {
            output_generation: expected_fence.output_generation + 1,
            ..expected_fence.clone()
        };
        let mut observed = None;

        let result = execute_standby_takeover_action(
            &action,
            &expected_fence,
            |force, selector, actual_fence| {
                observed = Some((force, selector, actual_fence.clone()));
                Ok(((), fence_after.clone()))
            },
        );

        assert_eq!(result, Ok((true, fence_after)));
        assert_eq!(
            observed,
            Some((
                true,
                StandbyTakeoverCheckpointSelector::Exact(StandbyCheckpointIdentity {
                    session_id: "primary-a".to_string(),
                    generation: 42,
                }),
                expected_fence,
            )),
            "the real OutputControl action branch must pass force, Exact identity, and fence to the core"
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
        let response = test_output_rejection(&key, OutputControlErrorCodeV2::PublicationFailed);

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
            OutputControlLaneReservation::Rejected(OutputControlErrorCodeV2::InvalidRequest)
        ));

        // Expiry removes the terminal response but leaves a tombstone so the
        // exact key cannot be reused with either the old or a new shape.
        assert!(matches!(
            state.reserve_output_control_lane(&key, &shape, now + RECEIPT_TTL),
            OutputControlLaneReservation::Rejected(OutputControlErrorCodeV2::InvalidRequest)
        ));
        assert!(matches!(
            state.reserve_output_control_lane(&key, &"c".repeat(64), now + RECEIPT_TTL),
            OutputControlLaneReservation::Rejected(OutputControlErrorCodeV2::InvalidRequest)
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
            test_output_rejection(&terminal_key, OutputControlErrorCodeV2::Busy),
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
                OutputControlLaneReservation::Rejected(OutputControlErrorCodeV2::InvalidRequest)
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
            OutputControlErrorCodeV2::Overloaded
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
            OutputControlErrorCodeV2::Busy
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
