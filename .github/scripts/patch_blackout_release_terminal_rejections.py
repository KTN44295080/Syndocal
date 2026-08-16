from pathlib import Path


def replace_exact(path: Path, old: str, new: str, label: str) -> None:
    text = path.read_text(encoding="utf-8")
    count = text.count(old)
    if count != 1:
        raise RuntimeError(f"{label}: expected exactly one match, found {count}")
    path.write_text(text.replace(old, new, 1), encoding="utf-8")


runtime = Path("app/src-tauri/src/control_plane_runtime.rs")

replace_exact(
    runtime,
    '''fn output_control_rejection(
    request: &OutputControlCommandRequestV1,
    error: OutputControlErrorCodeV1,
) -> OutputControlResponseV1 {
    OutputControlResponseV1::Rejected(OutputControlRejectionV1 {
        operation_id: request.action.operation_id().to_string(),
        request_id: request.request_id.clamp(1, MAX_SAFE_JAVASCRIPT_INTEGER),
        error,
    })
}
''',
    '''fn output_control_rejection(
    request: &OutputControlCommandRequestV1,
    error: OutputControlErrorCodeV1,
) -> OutputControlResponseV1 {
    OutputControlResponseV1::Rejected(OutputControlRejectionV1 {
        operation_id: request.action.operation_id().to_string(),
        request_id: request.request_id.clamp(1, MAX_SAFE_JAVASCRIPT_INTEGER),
        error,
    })
}

fn retain_output_control_rejection(
    state: &AppState,
    request: &OutputControlCommandRequestV1,
    key: OutputControlReceiptKey,
    shape_sha256: String,
    error: OutputControlErrorCodeV1,
) -> OutputControlResponseV1 {
    let response = output_control_rejection(request, error);
    if state
        .runtime_control_plane
        .store_output_control_terminal(key.clone(), shape_sha256, response.clone(), Instant::now())
        .is_err()
    {
        state.runtime_control_plane.release_output_control_lane(&key);
        return output_control_rejection(request, OutputControlErrorCodeV1::Internal);
    }
    response
}
''',
    "terminal output rejection helper",
)

replace_exact(
    runtime,
    '''    if query_state
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
''',
    '''    if query_state
        .validate_output_control_fence_window(
            window.label(),
            &request.expected_fence,
            binding.owner_incarnation,
        )
        .is_err()
    {
        return retain_output_control_rejection(
            state,
            &request,
            key,
            shape_sha256,
            OutputControlErrorCodeV1::Forbidden,
        );
    }
''',
    "retain invalid issued-fence rejection",
)

replace_exact(
    runtime,
    '''    if reconcile_project_checkpoint_for_coordinator(state, &mut coordinator).is_err()
        || !exact_output_control_fence_matches(state, &coordinator, &request.expected_fence)
        || ensure_no_pending_project_transaction(&coordinator).is_err()
    {
        state
            .runtime_control_plane
            .release_output_control_lane(&key);
        return output_control_rejection(&request, OutputControlErrorCodeV1::StaleFence);
    }
''',
    '''    if reconcile_project_checkpoint_for_coordinator(state, &mut coordinator).is_err()
        || !exact_output_control_fence_matches(state, &coordinator, &request.expected_fence)
        || ensure_no_pending_project_transaction(&coordinator).is_err()
    {
        drop(coordinator);
        drop(external_admission);
        return retain_output_control_rejection(
            state,
            &request,
            key,
            shape_sha256,
            OutputControlErrorCodeV1::StaleFence,
        );
    }
''',
    "retain stale output fence rejection",
)

replace_exact(
    runtime,
    '''    if ensure_project_operator_video_clip_slot_runtime_allowed(
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
''',
    '''    if ensure_project_operator_video_clip_slot_runtime_allowed(
        state,
        &coordinator,
        &binding.principal,
    )
    .is_err()
    {
        drop(coordinator);
        drop(external_admission);
        return retain_output_control_rejection(
            state,
            &request,
            key,
            shape_sha256,
            OutputControlErrorCodeV1::Forbidden,
        );
    }
''',
    "retain Full Lock/operator rejection",
)

replace_exact(
    runtime,
    '''    if validate_output_action_current(state, &request.action).is_err() {
        state
            .runtime_control_plane
            .release_output_control_lane(&key);
        return output_control_rejection(&request, OutputControlErrorCodeV1::StaleFence);
    }
''',
    '''    if validate_output_action_current(state, &request.action).is_err() {
        drop(coordinator);
        drop(external_admission);
        return retain_output_control_rejection(
            state,
            &request,
            key,
            shape_sha256,
            OutputControlErrorCodeV1::StaleFence,
        );
    }
''',
    "retain stale action-specific rejection",
)

# Extend the existing storage-level reply-loss test with a terminal rejection.
# It proves an exact retry returns the same reject while a same-ID token change
# cannot reinterpret that request identity as a new command.
replace_exact(
    runtime,
    '''        assert!(matches!(
            state.reserve_output_control_lane(&key, &shape, now + RECEIPT_TTL),
            OutputControlLaneReservation::Rejected(OutputControlErrorCodeV1::ReceiptExpired)
        ));
    }

''',
    '''        assert!(matches!(
            state.reserve_output_control_lane(&key, &shape, now + RECEIPT_TTL),
            OutputControlLaneReservation::Rejected(OutputControlErrorCodeV1::ReceiptExpired)
        ));

        let rejected_request = OutputControlCommandRequestV1 {
            request_id: 90_102,
            ..request.clone()
        };
        let rejected_shape =
            hex_sha256(&rejected_request.canonical_shape_bytes().unwrap());
        let rejected_key = output_control_receipt_key(&rejected_request, &binding);
        assert!(matches!(
            state.reserve_output_control_lane(&rejected_key, &rejected_shape, now),
            OutputControlLaneReservation::Lane(_)
        ));
        let rejected = output_control_rejection(
            &rejected_request,
            OutputControlErrorCodeV1::StaleFence,
        );
        state
            .store_output_control_terminal(
                rejected_key.clone(),
                rejected_shape.clone(),
                rejected.clone(),
                now,
            )
            .unwrap();
        assert!(matches!(
            state.reserve_output_control_lane(&rejected_key, &rejected_shape, now),
            OutputControlLaneReservation::Terminal(actual) if actual == rejected
        ));
        let mut changed_rejected = rejected_request;
        changed_rejected.consent_token = "AQEBAQEBAQEBAQEBAQEBAQ".to_string();
        let changed_rejected_shape =
            hex_sha256(&changed_rejected.canonical_shape_bytes().unwrap());
        assert!(matches!(
            state.reserve_output_control_lane(
                &rejected_key,
                &changed_rejected_shape,
                now,
            ),
            OutputControlLaneReservation::Rejected(OutputControlErrorCodeV1::InvalidRequest)
        ));
    }

''',
    "focused terminal preflight rejection identity test",
)
