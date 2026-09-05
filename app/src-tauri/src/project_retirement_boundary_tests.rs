#[test]
fn project_retirement_callback_failure_still_joins_outputs_without_publishing() {
    struct Platform {
        retired: AtomicU64,
        published: AtomicU64,
    }
    impl ProjectReplacementPlatform for Platform {
        fn advance_recovery_authority(
            &self, _state: &AppState, coordinator: &mut ProjectCoordinator,
            transition: ProjectRecoveryAuthorityTransition,
        ) -> Result<u64, String> {
            advance_project_recovery_authority_serial_with_persist(coordinator, transition, |_, _| Ok(()))
        }
        fn fence_and_retire_outputs(&self, state: &AppState) -> Result<(), String> {
            assert_eq!(state.engine.output_ownership_status().state, protocol::OutputOwnershipState::Transitioning);
            self.retired.fetch_add(1, Ordering::Relaxed);
            state.engine.fence_output_ownership()
        }
        fn observe_engine_publication(&self, _state: &AppState) {
            self.published.fetch_add(1, Ordering::Relaxed);
        }
        fn emit_authority_event(&self, _: ProjectReplacementCoordinatorEffect, _: &ProjectLoadResult) {
            panic!("callback fencing failure must not publish project authority");
        }
    }
    let harness = MediaAssetA6CommandHarness::new();
    let state = &harness.state;
    let epoch_before = state.project_coordinator.lock().unwrap().epoch;
    let serial = state.project_coordinator.lock().unwrap().recovery_authority_serial;
    state.project_callback_epoch.store(u64::MAX, Ordering::Release);
    let platform = Platform { retired: AtomicU64::new(0), published: AtomicU64::new(0) };
    let error = load_project_checkpoint_core(
        state, serde_json::to_value(empty_project_file()).unwrap(),
        "Rejected replacement".to_string(), None, serial,
        "retirement-callback-failure-0001".to_string(), &platform,
    ).unwrap_err();
    assert!(error.contains("epoch"), "{error}");
    assert_eq!(platform.retired.load(Ordering::Relaxed), 1);
    assert_eq!(platform.published.load(Ordering::Relaxed), 0);
    assert_eq!(state.project_coordinator.lock().unwrap().epoch, epoch_before);
    assert_eq!(state.engine.output_ownership_status().state, protocol::OutputOwnershipState::Failed);
}
