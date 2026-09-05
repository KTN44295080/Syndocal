use super::*;

#[test]
fn manual_bpm_ticket_preserves_prior_history_and_undo_redo_exact_images() {
    let _serial = serialized_authored_test_guard();
    let (harness, query, effect_id) = seeded_real_handler(false);
    let state = &harness.state;
    let original = engine_persistence_bytes(state);
    let applied = set_effect_enabled_authoritative_for_window_label(
        state,
        &query,
        PRIMARY_WINDOW,
        issued_request(state, &query, PRIMARY_WINDOW, effect_id, true, 110),
    );
    assert!(matches!(
        terminal_outcome(&applied),
        SetEffectEnabledOutcomeV1::Applied(_)
    ));
    let before_bpm = engine_persistence_bytes(state);
    assert_eq!(history_depths(state).0, 1);
    assert_eq!(
        crate::control_plane::tauri_route_admission_class("set_bpm"),
        Some(crate::control_plane::TauriRouteAdmissionClass::RendererTicketedProjectMutation),
    );
    let (epoch, revision, hash) = {
        let coordinator = state.project_coordinator.lock().unwrap();
        (
            coordinator.epoch,
            coordinator.revision,
            coordinator.checkpoint_hash.clone(),
        )
    };
    let shape = crate::canonical_project_transaction_shape("set_bpm", "Set Bpm", "");
    let ticket = crate::begin_project_transaction_for_window_label(
        state,
        PRIMARY_WINDOW,
        crate::BeginProjectTransactionRequest {
            label: "Set Bpm".into(),
            coalesce_key: String::new(),
            expected_epoch: epoch,
            expected_revision: revision,
            expected_checkpoint_hash: hash,
            owner_id: MEDIA_ASSET_A6_OWNER.into(),
            client_operation_id: "project-op:991:bpm-history".into(),
            shape_fingerprint: shape.clone(),
            command_name: "set_bpm".into(),
            schema_version: crate::PROJECT_TRANSACTION_SCHEMA_VERSION,
        },
    )
    .unwrap();
    // The headless fixture replaces only Tauri's generated argument extraction;
    // use its real ticket validation and one-dispatch seal before the same handler.
    let (external, coordinator, nested) = crate::lock_renderer_ticketed_project_mutation(
        state,
        PRIMARY_WINDOW,
        "set_bpm",
        ticket.transaction_id,
        ticket.project_epoch,
        MEDIA_ASSET_A6_OWNER,
    )
    .unwrap();
    let lane =
        crate::project_transaction_lane_for_operation(state, &ticket.client_operation_id).unwrap();
    let dispatch = crate::admit_outer_renderer_dispatch("set_bpm", &lane).unwrap();
    drop(nested);
    drop(coordinator);
    drop(external);
    crate::set_bpm_for_state(state, 137.0).unwrap();
    drop(dispatch);
    let committed = crate::commit_project_transaction_for_window_label(
        state,
        PRIMARY_WINDOW,
        crate::CommitProjectTransactionRequest {
            transaction_id: ticket.transaction_id,
            expected_epoch: ticket.project_epoch,
            client_operation_id: ticket.client_operation_id.clone(),
            shape_fingerprint: shape.clone(),
            command_name: "set_bpm".into(),
            schema_version: crate::PROJECT_TRANSACTION_SCHEMA_VERSION,
            owner_id: MEDIA_ASSET_A6_OWNER.into(),
        },
    )
    .unwrap();
    assert_eq!(committed.history_status.undo_depth, 2);
    crate::acknowledge_project_transaction_for_window_label(
        state,
        PRIMARY_WINDOW,
        ticket.client_operation_id,
        shape,
        "set_bpm".into(),
        crate::PROJECT_TRANSACTION_SCHEMA_VERSION,
        MEDIA_ASSET_A6_OWNER.into(),
    )
    .unwrap();
    let after_bpm = engine_persistence_bytes(state);
    assert_ne!(after_bpm, before_bpm);
    assert_eq!(
        state.engine.persistence_snapshot().unwrap().clock.bpm,
        137.0
    );
    // An adjacent entry can only be rebased if it names the target checkpoint.
    // Reject a broken chain before publishing or popping either history stack.
    let prior_after = state.project_coordinator.lock().unwrap().history.undo[0]
        .after
        .clone();
    for mismatch in 0..2 {
        {
            let mut coordinator = state.project_coordinator.lock().unwrap();
            let source = &mut coordinator.history.undo[0].after;
            match mismatch {
                0 => source.hash = "broken-adjacent-content".into(),
                _ => source.epoch = source.epoch.checked_add(1).unwrap(),
            }
        }
        let corrupted = coordinator_audit(state);
        assert!(navigate_history_in_real_headless_harness(state, true).is_err());
        assert_eq!(engine_persistence_bytes(state), after_bpm);
        assert_eq!(coordinator_audit(state), corrupted);
        state.project_coordinator.lock().unwrap().history.undo[0].after = prior_after.clone();
    }
    let undo = navigate_history_in_real_headless_harness(state, true).unwrap();
    assert_eq!(undo.history_status.undo_depth, 1);
    assert_eq!(engine_persistence_bytes(state), before_bpm);
    navigate_history_in_real_headless_harness(state, true).unwrap();
    assert_eq!(engine_persistence_bytes(state), original);
    let redo_before = state.project_coordinator.lock().unwrap().history.redo[0]
        .before
        .clone();
    state.project_coordinator.lock().unwrap().history.redo[0]
        .before
        .hash = "broken-adjacent-redo-content".into();
    let corrupted = coordinator_audit(state);
    assert!(navigate_history_in_real_headless_harness(state, false).is_err());
    assert_eq!(engine_persistence_bytes(state), original);
    assert_eq!(coordinator_audit(state), corrupted);
    state.project_coordinator.lock().unwrap().history.redo[0].before = redo_before;
    navigate_history_in_real_headless_harness(state, false).unwrap();
    assert_eq!(engine_persistence_bytes(state), before_bpm);
    navigate_history_in_real_headless_harness(state, false).unwrap();
    assert_eq!(engine_persistence_bytes(state), after_bpm);
    assert_eq!(history_depths(state).0, 2);
    // Repeated navigation publishes new revisions without changing the
    // historical images; both adjacent boundaries must remain traversable.
    for (undo, expected) in [
        (true, &before_bpm),
        (true, &original),
        (false, &before_bpm),
        (false, &after_bpm),
    ] {
        navigate_history_in_real_headless_harness(state, undo).unwrap();
        assert_eq!(&engine_persistence_bytes(state), expected);
    }
    assert_eq!(history_depths(state).0, 2);
}

#[test]
fn manual_bpm_invalid_values_leave_engine_and_existing_history_unchanged() {
    let _serial = serialized_authored_test_guard();
    let (harness, query, effect_id) = seeded_real_handler(false);
    let state = &harness.state;
    let applied = set_effect_enabled_authoritative_for_window_label(
        state,
        &query,
        PRIMARY_WINDOW,
        issued_request(state, &query, PRIMARY_WINDOW, effect_id, true, 111),
    );
    assert!(matches!(
        terminal_outcome(&applied),
        SetEffectEnabledOutcomeV1::Applied(_)
    ));
    let before = engine_persistence_bytes(state);
    let coordinator = coordinator_audit(state);
    for bpm in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, 19.9, 300.1] {
        assert_eq!(
            crate::set_bpm_for_state(state, bpm).unwrap_err(),
            "BPM must be between 20 and 300"
        );
        assert_eq!(engine_persistence_bytes(state), before);
        assert_eq!(coordinator_audit(state), coordinator);
    }
}
