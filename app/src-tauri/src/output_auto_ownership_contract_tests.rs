// Backend contracts that automatic output ownership (app/src/outputAutoOwnership.ts)
// relies on. The renderer only decides whether to send the canonical local
// Enable; these tests pin the states it keys on and prove that the existing
// fence and role-ordering guard keep Standby, project replacement, Take Over and
// main-window close in charge of every interleaving.

fn auto_ownership_test_fence(state: &AppState) -> (ControlPlaneQueryState, OutputControlFenceV1) {
    let process_incarnation = state
        .output_lease_registry
        .lock()
        .expect("auto ownership output-lease registry")
        .process_session_incarnation();
    let query_state = ControlPlaneQueryState::for_process_incarnation(process_incarnation)
        .expect("auto ownership query state");
    let fence = query_state
        .issue_output_control_fence_for_window("media-asset-a6", state)
        .expect("auto ownership fence");
    (query_state, fence)
}

fn auto_ownership_fence_matches(state: &AppState, fence: &OutputControlFenceV1) -> bool {
    let coordinator = state
        .project_coordinator
        .lock()
        .expect("auto ownership coordinator");
    control_plane_runtime::exact_output_control_fence_matches(state, &coordinator, fence)
}

struct AutoOwnershipReplacementPlatform;

impl ProjectReplacementPlatform for AutoOwnershipReplacementPlatform {
    fn advance_recovery_authority(
        &self,
        _state: &AppState,
        coordinator: &mut ProjectCoordinator,
        transition: ProjectRecoveryAuthorityTransition,
    ) -> Result<u64, String> {
        advance_project_recovery_authority_serial_with_persist(coordinator, transition, |_, _| {
            Ok(())
        })
    }
    fn fence_and_retire_outputs(&self, state: &AppState) -> Result<(), String> {
        state.engine.fence_output_ownership()
    }
    fn emit_authority_event(&self, _: ProjectReplacementCoordinatorEffect, _: &ProjectLoadResult) {}
}

#[test]
fn auto_ownership_start_state_is_startup_denied_with_the_stored_role() {
    // A normal start publishes the stored role as desired and persisted while
    // outputs stay denied: the only start state the renderer takes.
    let directory = unique_test_directory("auto-ownership-start");
    let path = directory.join(OUTPUT_OWNERSHIP_STATE_FILE);
    assert_eq!(
        load_or_initialize_output_ownership_preference(&path).unwrap(),
        MachineOutputRole::Both,
        "first run stores Both"
    );
    // Main-window close always ends in Standby, so the next start reads Standby.
    persist_output_ownership_role_to_path(&path, MachineOutputRole::Standby).unwrap();
    assert_eq!(
        load_or_initialize_output_ownership_preference(&path).unwrap(),
        MachineOutputRole::Standby
    );
    let _ = fs::remove_dir_all(&directory);

    // A production engine starts fail-closed with no stored role; a startup
    // failure (unreadable or corrupt role file) leaves it like that, which the
    // renderer treats as not eligible.
    let engine = EngineHandle::start(DmxOutputConfig {
        enabled: false,
        ..DmxOutputConfig::default()
    });
    let failed = engine.output_ownership_status();
    assert_eq!(failed.state, protocol::OutputOwnershipState::Failed);
    assert_eq!(
        failed.lighting_reason,
        protocol::OutputOwnershipReason::StartupDenied
    );
    assert_eq!(
        failed.persisted_role, None,
        "a startup failure is not an eligible start"
    );
    engine.mark_output_ownership_startup_failure("corrupt role file");
    assert_eq!(engine.output_ownership_status().persisted_role, None);
    let status = engine
        .publish_output_ownership_startup_preference(MachineOutputRole::Standby)
        .unwrap();
    assert_eq!(status.state, protocol::OutputOwnershipState::Failed);
    assert_eq!(status.effective_role, MachineOutputRole::Standby);
    assert_eq!(status.desired_role, MachineOutputRole::Standby);
    assert_eq!(status.persisted_role, Some(MachineOutputRole::Standby));
    assert_eq!(
        status.lighting_reason,
        protocol::OutputOwnershipReason::StartupDenied
    );
    assert_eq!(
        status.video_reason,
        protocol::OutputOwnershipReason::StartupDenied
    );
    assert!(
        !status.lighting_allowed && !status.video_allowed,
        "nothing is sent before the take"
    );
}

#[test]
fn auto_ownership_project_replacement_supersedes_a_take_prepared_before_it() {
    let harness = MediaAssetA6CommandHarness::new();
    let state = &harness.state;
    state
        .engine
        .publish_output_ownership_startup_preference(MachineOutputRole::Standby)
        .unwrap();
    let (query_state, before) = auto_ownership_test_fence(state);
    assert!(auto_ownership_fence_matches(state, &before));
    let queued = state.output_role_ordering.capture().unwrap();

    let prepared = prepare_project_load(
        empty_project_file(),
        ProjectControlMappings::default(),
        "auto-ownership".to_string(),
        None,
    )
    .unwrap();
    replace_prepared_project_snapshot_with_platform(
        state,
        prepared,
        ProjectSnapshotReplacementScope::ExternalCaller,
        None,
        &AutoOwnershipReplacementPlatform,
    )
    .unwrap();

    // The take prepared before the open can never land on the new project.
    assert!(matches!(
        state.output_role_ordering.enter(queued),
        Err(crate::output_role_ordering::OrderingError::Superseded)
    ));
    assert!(
        !auto_ownership_fence_matches(state, &before),
        "the pre-open fence is refused"
    );
    let after = state.engine.output_ownership_status();
    assert_eq!(after.effective_role, MachineOutputRole::Standby);
    assert!(
        !after.lighting_allowed && !after.video_allowed,
        "an open leaves outputs denied until taken again"
    );
    // A fresh take after the open is admitted (this is the automatic re-take).
    let fresh = query_state
        .issue_output_control_fence_for_window("media-asset-a6", state)
        .unwrap();
    assert!(auto_ownership_fence_matches(state, &fresh));
    assert!(state
        .output_role_ordering
        .enter(state.output_role_ordering.capture().unwrap())
        .is_ok());
}

#[test]
fn auto_ownership_standby_transition_refences_a_take_prepared_before_it() {
    // The renderer captures the fence, checks the operator's Standby intent and
    // dispatches. A Standby that lands after the fence was captured advances the
    // output counters, so the take is refused at its revalidation.
    let harness = MediaAssetA6CommandHarness::new();
    let state = &harness.state;
    state
        .engine
        .publish_output_ownership_startup_preference(MachineOutputRole::Standby)
        .unwrap();
    let (_query_state, before) = auto_ownership_test_fence(state);
    assert!(auto_ownership_fence_matches(state, &before));
    let standby = state
        .engine
        .begin_output_ownership_transition(MachineOutputRole::Standby)
        .unwrap()
        .complete()
        .unwrap();
    assert_eq!(standby.effective_role, MachineOutputRole::Standby);
    assert!(
        !auto_ownership_fence_matches(state, &before),
        "Standby wins over a prepared take"
    );
}

#[test]
fn auto_ownership_standby_and_take_over_boundaries_win_over_queued_and_inflight_takes() {
    use crate::output_role_ordering::{OrderingError, OutputRoleOrdering};
    // Standby (and Take Over / project replacement, which use the same deny
    // boundary) against a take queued before it: the take is refused.
    let ordering = OutputRoleOrdering::default();
    let queued = ordering.capture().unwrap();
    let boundary = ordering.deny().unwrap();
    assert!(
        matches!(ordering.capture(), Err(OrderingError::Superseded)),
        "no take starts during the boundary"
    );
    drop(boundary);
    assert!(
        matches!(ordering.enter(queued), Err(OrderingError::Superseded)),
        "nor after it"
    );

    // Against a take already publishing: the boundary waits for it, the take's
    // final validation fails, and the boundary then commits last.
    let ordering = OutputRoleOrdering::default();
    let publication = ordering.enter(ordering.capture().unwrap()).unwrap();
    let order = std::sync::Mutex::new(Vec::new());
    std::thread::scope(|scope| {
        let standby = scope.spawn(|| {
            let _deny = ordering.deny().unwrap();
            order.lock().unwrap().push("standby");
        });
        ordering.wait_until_denial_pending();
        assert_eq!(publication.validate(), Err(OrderingError::Superseded));
        order.lock().unwrap().push("take-drained");
        drop(publication);
        standby.join().unwrap();
    });
    assert_eq!(
        *order.lock().unwrap(),
        vec!["take-drained", "standby"],
        "Standby is applied after the take"
    );
    assert!(
        ordering.enter(ordering.capture().unwrap()).is_ok(),
        "only a new explicit take may follow"
    );
}

#[test]
fn auto_ownership_main_window_close_seals_every_later_take() {
    use crate::output_role_ordering::{OrderingError, OutputRoleOrdering};
    let ordering = OutputRoleOrdering::default();
    let queued = ordering.capture().unwrap();
    assert!(!ordering.seal().unwrap(), "idle close");
    assert!(matches!(
        ordering.enter(queued),
        Err(OrderingError::Superseded)
    ));
    assert!(
        matches!(ordering.capture(), Err(OrderingError::Superseded)),
        "no take after close"
    );
    // An automatic take in flight at close forces the Standby shutdown path.
    let ordering = OutputRoleOrdering::default();
    let inflight = ordering.enter(ordering.capture().unwrap()).unwrap();
    assert!(ordering.seal().unwrap(), "close reports the in-flight take");
    assert_eq!(inflight.validate(), Err(OrderingError::Superseded));
    assert!(
        crate::main_window_close_shutdown::main_close_requires_output_shutdown(
            crate::main_window_close_shutdown::MAIN_WINDOW_LABEL,
            true,
            MachineOutputRole::Standby,
            false,
        )
    );
}
