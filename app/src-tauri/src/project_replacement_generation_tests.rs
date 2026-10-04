fn generation_replacement_test_authority(state: &AppState) -> (ProjectReplacementAuthorityExpectation, u64) {
    let coordinator = state.project_coordinator.lock().unwrap();
    ((Some(1), coordinator.epoch, coordinator.revision, coordinator.checkpoint_hash.clone()),
        coordinator.publication_generation)
}

#[test]
fn project_replacement_generation_stale_initial_preflight_has_no_replacement_effect() {
    let harness = MediaAssetA6CommandHarness::new();
    let state = &harness.state;
    let (authority, generation) = generation_replacement_test_authority(state);
    let before_output = state.engine.output_ownership_status();
    let before_callback = state.project_callback_epoch.load(Ordering::Acquire);
    let error = with_project_replacement_invocation_generation::<()>(
        state, "media-asset-a6", "new_project", MEDIA_ASSET_A6_OWNER.into(),
        authority.clone(), Some(generation + 1), || {
            preflight_project_replacement_invocation(state)?;
            panic!("stale generation cannot reach the physical replacement boundary");
        },
    ).unwrap_err();
    assert!(error.contains("publication generation"), "{error}");
    assert_eq!(generation_replacement_test_authority(state), (authority, generation));
    assert_eq!(state.engine.output_ownership_status(), before_output);
    assert_eq!(state.project_callback_epoch.load(Ordering::Acquire), before_callback);
    assert!(PROJECT_REPLACEMENT_INVOCATION_FENCE.with(|slot| slot.borrow().is_none()));
}

#[test]
fn project_replacement_generation_detects_unchanged_content_after_initial_preflight() {
    let harness = MediaAssetA6CommandHarness::new();
    let state = &harness.state;
    let (authority, generation) = generation_replacement_test_authority(state);
    let error = with_project_replacement_invocation_generation(
        state, "media-asset-a6", "load_project_path", MEDIA_ASSET_A6_OWNER.into(),
        authority.clone(), Some(generation), || {
            preflight_project_replacement_invocation(state)?;
            // Simulate an intervening publication returning to the same E/R/H.
            state.project_coordinator.lock().unwrap().publication_generation += 1;
            let coordinator = state.project_coordinator.lock().unwrap();
            validate_project_replacement_invocation_at_publication(state, &coordinator)
        },
    ).unwrap_err();
    assert!(error.contains("publication generation"), "{error}");
    assert_eq!(generation_replacement_test_authority(state), (authority, generation + 1));
    assert!(PROJECT_REPLACEMENT_INVOCATION_FENCE.with(|slot| slot.borrow().is_none()));
}

#[test]
fn project_replacement_generation_exact_fence_reaches_both_validation_boundaries() {
    let harness = MediaAssetA6CommandHarness::new();
    let state = &harness.state;
    let (authority, generation) = generation_replacement_test_authority(state);
    let value = with_project_replacement_invocation_generation(
        state, "media-asset-a6", "new_project", MEDIA_ASSET_A6_OWNER.into(),
        authority, Some(generation), || {
            preflight_project_replacement_invocation(state)?;
            let coordinator = state.project_coordinator.lock().unwrap();
            validate_project_replacement_invocation_at_publication(state, &coordinator)?;
            Ok(17)
        },
    ).unwrap();
    assert_eq!(value, 17);
}

#[test]
fn project_replacement_generation_unsafe_identity_rejects_before_invocation() {
    let harness = MediaAssetA6CommandHarness::new();
    let state = &harness.state;
    let (authority, _) = generation_replacement_test_authority(state);
    let error = with_project_replacement_invocation_generation::<()>(
        state, "media-asset-a6", "new_project", MEDIA_ASSET_A6_OWNER.into(),
        authority, Some(protocol::control_plane_command::MAX_SAFE_JAVASCRIPT_INTEGER + 1),
        || panic!("unsafe authority must reject before invoking the replacement"),
    ).unwrap_err();
    assert!(error.contains("safe integer"));
    assert!(PROJECT_REPLACEMENT_INVOCATION_FENCE.with(|slot| slot.borrow().is_none()));
}

#[test]
fn project_replacement_generation_retains_owner_incarnation_revalidation() {
    let harness = MediaAssetA6CommandHarness::new();
    let state = &harness.state;
    let (authority, generation) = generation_replacement_test_authority(state);
    let error = with_project_replacement_invocation_generation(
        state, "media-asset-a6", "new_project", MEDIA_ASSET_A6_OWNER.into(),
        authority, Some(generation), || {
            preflight_project_replacement_invocation(state)?;
            state.project_transaction_owner_incarnations.lock().unwrap()
                .insert("media-asset-a6".into(), 2);
            preflight_project_replacement_invocation(state)
        },
    ).unwrap_err();
    assert!(error.contains("retired renderer incarnation"), "{error}");
}

#[test]
fn project_replacement_generation_legacy_invocation_retains_its_existing_contract() {
    let harness = MediaAssetA6CommandHarness::new();
    let state = &harness.state;
    let (authority, generation) = generation_replacement_test_authority(state);
    with_project_replacement_invocation(
        state, "media-asset-a6", "new_project", MEDIA_ASSET_A6_OWNER.into(),
        authority.clone(), || {
            state.project_coordinator.lock().unwrap().publication_generation += 1;
            preflight_project_replacement_invocation(state)
        },
    ).unwrap();
    assert_eq!(generation_replacement_test_authority(state), (authority, generation + 1));
}
