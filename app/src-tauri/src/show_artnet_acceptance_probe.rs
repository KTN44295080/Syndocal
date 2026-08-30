//! Fixed DSF2026 same-PC Art-Net acceptance probe.
//!
//! This module deliberately has no caller-controlled endpoint, universe,
//! payload, retry, or route-activation parameter.  The R4 adapter validates
//! authority and the staged authored route before it reaches this one-shot
//! transport boundary.

use protocol::{
    control_plane_command::{
        OutputControlCommandRequestV2, OutputControlReceiptOutcomeV2, OutputControlReceiptV2,
        OutputControlResponseV2,
    },
    DmxOutputConfig, DmxOutputProtocol,
};

use super::{
    control_plane_runtime, ensure_no_pending_project_transaction,
    ensure_project_operator_video_clip_slot_runtime_allowed, lock_output_ownership_transition,
    lock_project_coordinator, lock_project_external_command_admission, output_lease,
    reconcile_project_checkpoint_for_coordinator,
    submit_dsf2026_artnet_acceptance_probe_candidate_with_classified_commit_and_durable_record,
    submit_output_lease_candidate_with_classified_commit_and_durable_record,
    validate_current_staged_show_artnet_loopback_route, with_revalidated_output_transition,
    AppState, MachineOutputRole, OutputControlFenceV1, OutputLeaseCandidateCommitFailure,
    OutputLeaseRequest,
};
use std::cell::RefCell;

pub(crate) const DSF2026_ARTNET_ACCEPTANCE_PROBE_TARGET_IP: &str = "127.0.0.1";
pub(crate) const DSF2026_ARTNET_ACCEPTANCE_PROBE_PORT: u16 = 6454;
pub(crate) const DSF2026_ARTNET_ACCEPTANCE_PROBE_UNIVERSE: u16 = 0;
#[cfg(test)]
pub(crate) const DSF2026_ARTNET_ACCEPTANCE_PROBE_SLOT_COUNT: usize = 512;
#[cfg(test)]
pub(crate) const DSF2026_ARTNET_ACCEPTANCE_PROBE_PACKET_BYTES: usize =
    18 + DSF2026_ARTNET_ACCEPTANCE_PROBE_SLOT_COUNT;

/// The exact disabled logical route required before the probe may be sent.
/// It intentionally excludes serial aliases and every non-loopback endpoint.
pub(crate) fn is_exact_staged_dsf2026_artnet_acceptance_probe_route(
    route: &DmxOutputConfig,
) -> bool {
    !route.enabled
        && route.protocol == DmxOutputProtocol::ArtNet
        && route.target_ip == DSF2026_ARTNET_ACCEPTANCE_PROBE_TARGET_IP
        && route.port == DSF2026_ARTNET_ACCEPTANCE_PROBE_PORT
        && route.universe == DSF2026_ARTNET_ACCEPTANCE_PROBE_UNIVERSE
        && route.serial_port.is_empty()
}

/// Build the immutable ArtDmx datagram used by the DSF2026 acceptance probe.
///
/// Art-Net ArtDmx is 18 bytes of header followed by exactly 512 DMX slots.
/// The fixed proof look uses DMX slots 1 and 5 at full and keeps the unpatched
/// DMX slot 500 hard-zeroed. All other slots are zero by construction.
#[cfg(test)]
pub(crate) fn build_dsf2026_artnet_acceptance_probe_packet(
) -> [u8; DSF2026_ARTNET_ACCEPTANCE_PROBE_PACKET_BYTES] {
    let mut packet = [0_u8; DSF2026_ARTNET_ACCEPTANCE_PROBE_PACKET_BYTES];
    packet[..8].copy_from_slice(b"Art-Net\0");
    // OpCode ArtDmx (0x5000) is little-endian on the Art-Net wire.
    packet[8] = 0x00;
    packet[9] = 0x50;
    // Protocol version 14 is big-endian. Sequence and physical remain zero.
    packet[10] = 0x00;
    packet[11] = 14;
    // Wire universe U0, little-endian.
    packet[14] = 0x00;
    packet[15] = 0x00;
    // ArtDmx data length 512, big-endian.
    packet[16] = 0x02;
    packet[17] = 0x00;

    packet[18] = 255; // DMX ch1 / payload[0]
    packet[22] = 255; // DMX ch5 / payload[4]
    packet[18 + 499] = 0; // DMX ch500 / payload[499], explicitly fixed
    packet
}

/// Validate the deliberately narrower DSF2026 proof boundary. The probe is
/// not a route activation: it requires the same single authored route to be
/// disabled, so no engine tick or normal DMX sender can participate.
pub(crate) fn validate_current_dsf2026_artnet_acceptance_probe(
    state: &AppState,
) -> Result<DmxOutputConfig, String> {
    validate_current_staged_show_artnet_loopback_route(state)
}

pub(crate) struct Dsf2026ArtNetAcceptanceProbeControlRequest<'a> {
    pub(crate) expected_fence: &'a OutputControlFenceV1,
    pub(crate) lease_request: &'a OutputLeaseRequest,
    pub(crate) lease_now_ms: u64,
    pub(crate) expected_owner_principal: &'a str,
    pub(crate) expected_owner_window_label: &'a str,
    pub(crate) expected_owner_incarnation: u64,
    /// Canonical public R4 receipt identity. The private lease receipt and
    /// this response are committed as one durable journal candidate.
    pub(crate) public_request: &'a OutputControlCommandRequestV2,
    pub(crate) shape_sha256: &'a str,
    pub(crate) argument_fingerprint: &'a str,
    pub(crate) audit_sequence: u64,
}

fn dsf2026_public_terminal_response(
    public_request: &OutputControlCommandRequestV2,
    shape_sha256: &str,
    argument_fingerprint: &str,
    audit_sequence: u64,
    lease_receipt: &output_lease::OutputLeaseRequestReceipt,
) -> Result<OutputControlResponseV2, String> {
    let lease_result = control_plane_runtime::output_control_lease_result_from_registry_receipt(
        &public_request.action,
        lease_receipt,
    )?;
    let response = OutputControlResponseV2::Receipt(Box::new(OutputControlReceiptV2 {
        operation_id: public_request.operation_id.clone(),
        request_id: public_request.request_id,
        shape_sha256: shape_sha256.to_string(),
        argument_fingerprint: argument_fingerprint.to_string(),
        audit_sequence,
        fence_before: public_request.expected_fence.clone(),
        // Both DSF2026 operations are physically/no-send probe boundaries;
        // neither mutates the project/output/publication fence.
        fence_after: public_request.expected_fence.clone(),
        outcome: OutputControlReceiptOutcomeV2::Applied,
        lease_result: Some(lease_result),
    }));
    response
        .validate()
        .map_err(|error| format!("DSF2026 public terminal response is invalid: {error}"))?;
    Ok(response)
}

/// The only native send of the DSF2026 fixed acceptance look. It holds the
/// canonical R4 lifecycle, owner, external-admission, coordinator and output
/// transition guards across the last observation and the UDP boundary. It
/// never calls the normal tick, creates a DmxSender, enables a route, or
/// publishes a project/output mutation.
pub(crate) fn send_dsf2026_artnet_acceptance_probe_with_output_control_fence(
    state: &AppState,
    request: Dsf2026ArtNetAcceptanceProbeControlRequest<'_>,
) -> Result<
    (
        bool,
        OutputControlFenceV1,
        output_lease::OutputLeaseRequestReceipt,
    ),
    String,
> {
    let Dsf2026ArtNetAcceptanceProbeControlRequest {
        expected_fence,
        lease_request,
        lease_now_ms: _lease_now_ms,
        expected_owner_principal,
        expected_owner_window_label,
        expected_owner_incarnation,
        public_request,
        shape_sha256,
        argument_fingerprint,
        audit_sequence,
    } = request;
    let _lifecycle_guard = state.standby_sync_lifecycle.lock().map_err(|_| {
        "Standby synchronization lifecycle lock was poisoned before DSF2026 Art-Net probe"
            .to_string()
    })?;
    let _owner_rotation = state
        .project_transaction_owner_rotation
        .lock()
        .map_err(|_| {
            "Project transaction owner-rotation lock was poisoned before DSF2026 Art-Net probe"
                .to_string()
        })?;
    let _external_admission = lock_project_external_command_admission(state)?;
    let coordinator = RefCell::new(lock_project_coordinator(state)?);
    with_revalidated_output_transition(
        || lock_output_ownership_transition(state),
        || {
            {
                let mut coordinator = coordinator.borrow_mut();
                if reconcile_project_checkpoint_for_coordinator(state, &mut *coordinator).is_err()
                    || !control_plane_runtime::exact_output_control_owner_matches(
                        state,
                        expected_owner_principal,
                        expected_owner_window_label,
                        expected_owner_incarnation,
                    )
                    || !control_plane_runtime::exact_output_control_fence_matches(
                        state,
                        &*coordinator,
                        expected_fence,
                    )
                    || ensure_no_pending_project_transaction(&*coordinator).is_err()
                    || ensure_project_operator_video_clip_slot_runtime_allowed(
                        state,
                        &*coordinator,
                        expected_owner_principal,
                    )
                    .is_err()
                {
                    return Err(
                        "Output control owner/window/project/publication fence changed before DSF2026 Art-Net probe confirmation"
                            .to_string(),
                    );
                }
            }
            let ownership = state.engine.output_ownership_status();
            if ownership.state != protocol::OutputOwnershipState::Ready
                || ownership.effective_role != MachineOutputRole::Both
                || ownership.desired_role != MachineOutputRole::Both
                || !ownership.lighting_allowed
            {
                return Err(
                    "DSF2026 Art-Net acceptance probe requires the exact active local Both lighting authority"
                        .to_string(),
                );
            }
            let safety = state.engine.safety_blackout_authority();
            if safety.engaged
                || safety.epoch != expected_fence.safety_blackout_epoch
                || safety.generation != expected_fence.safety_blackout_generation
            {
                return Err(
                    "DSF2026 Art-Net acceptance probe requires the exact currently-clear safety blackout authority"
                        .to_string(),
                );
            }
            validate_current_dsf2026_artnet_acceptance_probe(state)
        },
        |_transition_guard, route| {
            let mut lease_registry = state.output_lease_registry.lock().map_err(|_| {
                "Output lease registry lock was poisoned before DSF2026 Art-Net probe".to_string()
            })?;
            let final_lease_now_ms = state.output_lease_now_ms()?;
            let (applied, lease_receipt) =
                submit_dsf2026_artnet_acceptance_probe_candidate_with_classified_commit_and_durable_record(
                    state,
                    &mut lease_registry,
                    lease_request,
                    final_lease_now_ms,
                    "DSF2026 Art-Net acceptance probe",
                    expected_owner_window_label,
                    || {
                        {
                            let mut coordinator = coordinator.borrow_mut();
                            if reconcile_project_checkpoint_for_coordinator(
                                state,
                                &mut *coordinator,
                            )
                            .is_err()
                                || !control_plane_runtime::exact_output_control_owner_matches(
                                    state,
                                    expected_owner_principal,
                                    expected_owner_window_label,
                                    expected_owner_incarnation,
                                )
                                || !control_plane_runtime::exact_output_control_fence_matches(
                                    state,
                                    &*coordinator,
                                    expected_fence,
                                )
                                || ensure_no_pending_project_transaction(&*coordinator).is_err()
                                || ensure_project_operator_video_clip_slot_runtime_allowed(
                                    state,
                                    &*coordinator,
                                    expected_owner_principal,
                                )
                                .is_err()
                            {
                                return Err(OutputLeaseCandidateCommitFailure::safe(
                                "Output control owner/window/project/publication fence changed immediately before DSF2026 Art-Net send",
                            ));
                            }
                        }
                        let ownership = state.engine.output_ownership_status();
                        if ownership.state != protocol::OutputOwnershipState::Ready
                            || ownership.effective_role != MachineOutputRole::Both
                            || ownership.desired_role != MachineOutputRole::Both
                            || !ownership.lighting_allowed
                        {
                            return Err(OutputLeaseCandidateCommitFailure::safe(
                            "DSF2026 Art-Net acceptance probe lost its exact local Both lighting authority",
                        ));
                        }
                        let safety = state.engine.safety_blackout_authority();
                        if safety.engaged
                            || safety.epoch != expected_fence.safety_blackout_epoch
                            || safety.generation != expected_fence.safety_blackout_generation
                        {
                            return Err(OutputLeaseCandidateCommitFailure::safe(
                            "DSF2026 Art-Net acceptance probe was superseded by an emergency blackout authority change",
                        ));
                        }
                        let current = validate_current_staged_show_artnet_loopback_route(state)
                            .map_err(OutputLeaseCandidateCommitFailure::safe)?;
                        if current != route {
                            return Err(OutputLeaseCandidateCommitFailure::safe(
                            "DSF2026 Art-Net acceptance probe route changed immediately before send",
                        ));
                        }
                        // The engine owns the final transport boundary. It holds
                        // the S0 enqueue gate while it rechecks its U0 input/
                        // merge state, staged route, absent sender, authority,
                        // and safety epoch immediately before send_to.
                        state
                            .engine
                            .send_dsf2026_artnet_acceptance_probe(
                                current,
                                expected_fence.safety_blackout_epoch,
                                expected_fence.safety_blackout_generation,
                                std::time::Instant::now() + std::time::Duration::from_secs(2),
                            )
                            .map_err(|error| match error {
                                engine::Dsf2026ArtNetAcceptanceProbeError::PreSend(message) => {
                                    OutputLeaseCandidateCommitFailure::safe(message)
                                }
                                engine::Dsf2026ArtNetAcceptanceProbeError::InDoubt(message) => {
                                    OutputLeaseCandidateCommitFailure::in_doubt(message)
                                }
                            })?;
                        Ok(true)
                    },
                    |durable, lease_receipt| {
                        let response = dsf2026_public_terminal_response(
                            public_request,
                            shape_sha256,
                            argument_fingerprint,
                            audit_sequence,
                            lease_receipt,
                        )?;
                        durable.record_dsf2026_output_control_terminal(
                            lease_receipt,
                            expected_owner_principal,
                            expected_owner_window_label,
                            public_request,
                            shape_sha256,
                            argument_fingerprint,
                            &response,
                        )
                    },
                )?;
            // The probe is physical I/O only. It must leave the project,
            // output route, normal sender and publication identities intact.
            Ok((applied, expected_fence.clone(), lease_receipt))
        },
    )
}

/// Explicitly resolve the probe-only durable InDoubt hold after the operator
/// has independently checked the loopback receiver and physical result. This
/// function never opens a socket or emits Art-Net; it merely records the
/// acknowledged reconciliation behind the same exact owner/fence/lease
/// boundary as the send action.
pub(crate) fn acknowledge_dsf2026_artnet_acceptance_probe_in_doubt_with_output_control_fence(
    state: &AppState,
    request: Dsf2026ArtNetAcceptanceProbeControlRequest<'_>,
) -> Result<
    (
        bool,
        OutputControlFenceV1,
        output_lease::OutputLeaseRequestReceipt,
    ),
    String,
> {
    let Dsf2026ArtNetAcceptanceProbeControlRequest {
        expected_fence,
        lease_request,
        lease_now_ms: _,
        expected_owner_principal,
        expected_owner_window_label,
        expected_owner_incarnation,
        public_request,
        shape_sha256,
        argument_fingerprint,
        audit_sequence,
    } = request;
    let _lifecycle_guard = state.standby_sync_lifecycle.lock().map_err(|_| {
        "Standby synchronization lifecycle lock was poisoned before DSF2026 probe reconciliation"
            .to_string()
    })?;
    let _owner_rotation = state
        .project_transaction_owner_rotation
        .lock()
        .map_err(|_| {
            "Project transaction owner-rotation lock was poisoned before DSF2026 probe reconciliation"
                .to_string()
        })?;
    let _external_admission = lock_project_external_command_admission(state)?;
    let coordinator = RefCell::new(lock_project_coordinator(state)?);
    with_revalidated_output_transition(
        || lock_output_ownership_transition(state),
        || {
            let mut coordinator = coordinator.borrow_mut();
            if reconcile_project_checkpoint_for_coordinator(state, &mut *coordinator).is_err()
                || !control_plane_runtime::exact_output_control_owner_matches(
                    state,
                    expected_owner_principal,
                    expected_owner_window_label,
                    expected_owner_incarnation,
                )
                || !control_plane_runtime::exact_output_control_fence_matches(
                    state,
                    &*coordinator,
                    expected_fence,
                )
                || ensure_no_pending_project_transaction(&*coordinator).is_err()
                || ensure_project_operator_video_clip_slot_runtime_allowed(
                    state,
                    &*coordinator,
                    expected_owner_principal,
                )
                .is_err()
            {
                return Err(
                    "Output control owner/window/project/publication fence changed before DSF2026 probe reconciliation"
                        .to_string(),
                );
            }
            validate_current_dsf2026_artnet_acceptance_probe(state)
        },
        |_transition_guard, _route| {
            let mut lease_registry = state.output_lease_registry.lock().map_err(|_| {
                "Output lease registry lock was poisoned before DSF2026 probe reconciliation"
                    .to_string()
            })?;
            let now_ms = state.output_lease_now_ms()?;
            let (applied, lease_receipt) =
                submit_output_lease_candidate_with_classified_commit_and_durable_record(
                    state,
                    &mut lease_registry,
                    lease_request,
                    now_ms,
                    "DSF2026 Art-Net acceptance probe reconciliation",
                    || {
                        let mut coordinator = coordinator.borrow_mut();
                        if reconcile_project_checkpoint_for_coordinator(state, &mut *coordinator)
                            .is_err()
                            || !control_plane_runtime::exact_output_control_owner_matches(
                                state,
                                expected_owner_principal,
                                expected_owner_window_label,
                                expected_owner_incarnation,
                            )
                            || !control_plane_runtime::exact_output_control_fence_matches(
                                state,
                                &*coordinator,
                                expected_fence,
                            )
                            || ensure_no_pending_project_transaction(&*coordinator).is_err()
                        {
                            return Err(OutputLeaseCandidateCommitFailure::safe(
                            "Output control owner/window/project/publication fence changed immediately before DSF2026 probe reconciliation",
                        ));
                        }
                        validate_current_dsf2026_artnet_acceptance_probe(state)
                            .map_err(OutputLeaseCandidateCommitFailure::safe)?;
                        state
                        .output_lease_durable_receipts
                        .lock()
                        .map_err(|_| {
                            OutputLeaseCandidateCommitFailure::safe(
                                "Output lease durable journal lock was poisoned before DSF2026 probe reconciliation",
                            )
                        })?
                        .validate_dsf2026_probe_pending(expected_owner_window_label)
                        .map_err(|error| {
                            OutputLeaseCandidateCommitFailure::safe(format!(
                                "DSF2026 probe reconciliation requires exactly one pending physical outcome: {error:?}"
                            ))
                        })?;
                        Ok(true)
                    },
                    |durable, lease_receipt| {
                        let response = dsf2026_public_terminal_response(
                            public_request,
                            shape_sha256,
                            argument_fingerprint,
                            audit_sequence,
                            lease_receipt,
                        )?;
                        durable.record_dsf2026_output_control_terminal(
                            lease_receipt,
                            expected_owner_principal,
                            expected_owner_window_label,
                            public_request,
                            shape_sha256,
                            argument_fingerprint,
                            &response,
                        )
                    },
                )?;
            Ok((applied, expected_fence.clone(), lease_receipt))
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packet_is_exact_artdmx_u0_with_a_full_512_slot_payload() {
        let packet = build_dsf2026_artnet_acceptance_probe_packet();
        assert_eq!(packet.len(), 530);
        assert_eq!(
            &packet[..18],
            b"Art-Net\0\x00\x50\x00\x0e\x00\x00\x00\x00\x02\x00"
        );
        assert_eq!(&packet[..8], b"Art-Net\0");
        assert_eq!(&packet[8..10], &[0x00, 0x50]);
        assert_eq!(
            &packet[10..12],
            &[0x00, 14],
            "the production serializer writes Art-Net protocol v14 at bytes 10..12"
        );
        assert_eq!(
            packet[13], 0,
            "the production serializer fixes the ArtDmx physical byte to zero"
        );
        assert_eq!(&packet[14..16], &[0x00, 0x00]);
        assert_eq!(&packet[16..18], &[0x02, 0x00]);
        assert_eq!(packet[18], 255);
        assert_eq!(packet[22], 255);
        assert_eq!(packet[18 + 499], 0);
        assert!(packet[18..].iter().enumerate().all(|(index, value)| {
            matches!((index, value), (0, 255) | (4, 255) | (499, 0) | (_, 0))
        }));
    }

    #[test]
    fn route_matcher_rejects_every_nonfixed_or_active_route() {
        let staged = DmxOutputConfig {
            enabled: false,
            protocol: DmxOutputProtocol::ArtNet,
            target_ip: DSF2026_ARTNET_ACCEPTANCE_PROBE_TARGET_IP.to_string(),
            port: DSF2026_ARTNET_ACCEPTANCE_PROBE_PORT,
            universe: DSF2026_ARTNET_ACCEPTANCE_PROBE_UNIVERSE,
            serial_port: String::new(),
            serial_baud_rate: DmxOutputConfig::default().serial_baud_rate,
        };
        assert!(is_exact_staged_dsf2026_artnet_acceptance_probe_route(
            &staged
        ));
        for invalid in [
            DmxOutputConfig {
                enabled: true,
                ..staged.clone()
            },
            DmxOutputConfig {
                protocol: DmxOutputProtocol::Sacn,
                ..staged.clone()
            },
            DmxOutputConfig {
                target_ip: "192.168.1.2".to_string(),
                ..staged.clone()
            },
            DmxOutputConfig {
                port: 6455,
                ..staged.clone()
            },
            DmxOutputConfig {
                universe: 1,
                ..staged.clone()
            },
            DmxOutputConfig {
                serial_port: "COM3".to_string(),
                ..staged
            },
        ] {
            assert!(!is_exact_staged_dsf2026_artnet_acceptance_probe_route(
                &invalid
            ));
        }
    }
}
