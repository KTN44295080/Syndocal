use super::*;

#[test]
fn native_video_blackout_identity_scope_preserves_next_gui_id_and_rejects_old_native_ids() {
    let state = RuntimeControlPlaneState::default();
    let binding = CallerBinding {
        principal: "same-owner".into(),
        window_label: "main".into(),
        owner_incarnation: 1,
    };
    let now = Instant::now();
    let operation = "syndocal.output.blackout.set.v2";
    let shape = "a".repeat(64);
    state
        .reserve_output_control_request_identity(&binding, operation, 2, &shape, now)
        .unwrap();
    state
        .reserve_output_control_request_identity_scoped(
            &binding,
            operation,
            1_u64 << 52,
            &shape,
            now,
            OutputControlRequestScope::NativeVideoBlackout,
        )
        .unwrap();
    state
        .reserve_output_control_request_identity(&binding, operation, 3, &shape, now)
        .unwrap();
    assert_eq!(
        state.reserve_output_control_request_identity_scoped(
            &binding,
            operation,
            (1_u64 << 52) - 1,
            &shape,
            now,
            OutputControlRequestScope::NativeVideoBlackout,
        ),
        Err(OutputControlErrorCodeV2::InvalidRequest)
    );
    assert_eq!(
        state.reserve_output_control_request_identity_scoped(
            &binding,
            "syndocal.output.ownership.arm.v2",
            1_u64 << 52,
            &shape,
            now,
            OutputControlRequestScope::NativeVideoBlackout,
        ),
        Err(OutputControlErrorCodeV2::InvalidRequest)
    );
}
