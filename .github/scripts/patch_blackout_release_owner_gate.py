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
    '''        if !exact_output_control_fence_matches(state, &coordinator, &request.expected_fence) {
            return Err("Output consent authority is stale".to_string());
        }
        ensure_no_pending_project_transaction(&coordinator)?;
''',
    '''        if !exact_output_control_fence_matches(state, &coordinator, &request.expected_fence) {
            return Err("Output consent authority is stale".to_string());
        }
        if matches!(&request.action, OutputControlActionV1::ReleaseBlackout)
            && !blackout_release_has_current_lighting_owner(&state.engine.output_ownership_status())
        {
            return Err("Blackout Release requires the current Lighting output owner".to_string());
        }
        ensure_no_pending_project_transaction(&coordinator)?;
''',
    "Release consent requires current Lighting owner",
)

replace_exact(
    runtime,
    '''    if ensure_project_operator_video_clip_slot_runtime_allowed(
        state,
        &coordinator,
        &binding.principal,
    )
''',
    '''    if matches!(&request.action, OutputControlActionV1::ReleaseBlackout)
        && !blackout_release_has_current_lighting_owner(&state.engine.output_ownership_status())
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
    if ensure_project_operator_video_clip_slot_runtime_allowed(
        state,
        &coordinator,
        &binding.principal,
    )
''',
    "Release execution requires current Lighting owner",
)

replace_exact(
    runtime,
    '''fn exact_output_control_fence_matches(
    state: &AppState,
    coordinator: &ProjectCoordinator,
    fence: &OutputControlFenceV1,
) -> bool {
''',
    '''fn blackout_release_has_current_lighting_owner(status: &protocol::OutputOwnershipStatus) -> bool {
    status.state == protocol::OutputOwnershipState::Ready && status.lighting_allowed
}

fn exact_output_control_fence_matches(
    state: &AppState,
    coordinator: &ProjectCoordinator,
    fence: &OutputControlFenceV1,
) -> bool {
''',
    "Release Lighting ownership helper",
)

replace_exact(
    runtime,
    '''    fn test_output_control_fence() -> OutputControlFenceV1 {
''',
    '''    #[test]
    fn blackout_release_requires_current_ready_lighting_owner() {
        assert!(blackout_release_has_current_lighting_owner(
            &protocol::OutputOwnershipStatus::for_role(protocol::MachineOutputRole::Lighting)
        ));
        assert!(blackout_release_has_current_lighting_owner(
            &protocol::OutputOwnershipStatus::for_role(protocol::MachineOutputRole::Both)
        ));
        assert!(!blackout_release_has_current_lighting_owner(
            &protocol::OutputOwnershipStatus::for_role(protocol::MachineOutputRole::Video)
        ));
        assert!(!blackout_release_has_current_lighting_owner(
            &protocol::OutputOwnershipStatus::for_role(protocol::MachineOutputRole::Standby)
        ));
        let mut transitioning =
            protocol::OutputOwnershipStatus::for_role(protocol::MachineOutputRole::Lighting);
        transitioning.state = protocol::OutputOwnershipState::Transitioning;
        assert!(!blackout_release_has_current_lighting_owner(&transitioning));
    }

    fn test_output_control_fence() -> OutputControlFenceV1 {
''',
    "Release current-owner focused test",
)
