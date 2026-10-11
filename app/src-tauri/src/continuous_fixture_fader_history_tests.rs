fn canonical_fixture_image(mut fixtures: Vec<PatchedFixtureSummary>) -> Vec<PatchedFixtureSummary> {
    for fixture in &mut fixtures {
        fixture.profile_source_path = format!("snapshot://fixture/{}", fixture.id);
    }
    fixtures
}
// Included by the real Stage/history harness; all output routes are disabled.
#[test]
fn continuous_fixture_fader_real_engine_commit_is_one_undo_redo_entry() {
    let harness = MediaAssetA6CommandHarness::new();
    let state = &harness.state;
    let window = "fader-history-window";
    let owner = "fader-history-owner";
    register_project_transaction_owner_for_window_label(state, window, owner.into()).unwrap();
    seed_two_stage_fixtures(state);
    let before = stage_image(state);
    let status = stage_history_status(state);
    let command = "stream_fixture_attribute_v1";
    let ticket = d4_begin_stage_ticket(state, window, owner, command, 910);
    let request = |sequence, value| continuous_fixture_fader::Request {
        targets: vec![TouchFeaturePresetTarget {
            fixture_id: 1,
            attribute: "Dimmer".into(),
        }],
        value,
        sequence,
        project_transaction_id: ticket.transaction_id,
        expected_epoch: ticket.project_epoch,
        owner_id: owner.into(),
    };
    continuous_fixture_fader::write(state, window, request(1, 10000)).unwrap();
    let intermediate = stage_image(state);
    assert_ne!(intermediate.fixtures, before.fixtures);
    assert_eq!(stage_history_status(state).undo_depth, status.undo_depth);
    assert!(continuous_fixture_fader::write(state, window, request(1, 1)).is_err());
    assert!(continuous_fixture_fader::write(state, "other-window", request(2, 1)).is_err());
    let mut stale = request(2, 1);
    stale.expected_epoch += 1;
    assert!(continuous_fixture_fader::write(state, window, stale).is_err());
    let mut foreign = request(2, 1);
    foreign.owner_id = "other-owner".into();
    assert!(continuous_fixture_fader::write(state, window, foreign).is_err());
    let mut invalid = request(2, 1);
    invalid.targets.push(TouchFeaturePresetTarget {
        fixture_id: 999,
        attribute: "Dimmer".into(),
    });
    assert!(continuous_fixture_fader::write(state, window, invalid).is_err());
    assert_eq!(stage_image(state).fixtures, intermediate.fixtures);
    continuous_fixture_fader::write(state, window, request(2, 60000)).unwrap();
    let final_image = stage_image(state);
    assert_ne!(final_image.fixtures, intermediate.fixtures);
    d4_commit_stage_ticket(state, window, owner, command, &ticket);
    let committed = stage_history_status(state);
    assert_eq!(committed.undo_depth, status.undo_depth + 1);
    assert!(continuous_fixture_fader::write(state, window, request(3, 1)).is_err());
    let undone = navigate_stage_history(
        state,
        true,
        committed.project_epoch,
        committed.undo_entry_id,
        Some(committed.checkpoint_hash),
    );
    assert_eq!(
        canonical_fixture_image(stage_image(state).fixtures),
        canonical_fixture_image(before.fixtures)
    );
    navigate_stage_history(
        state,
        false,
        undone.history_status.project_epoch,
        undone.history_status.redo_entry_id,
        undone.history_status.redo_checkpoint_hash,
    );
    assert_eq!(
        canonical_fixture_image(stage_image(state).fixtures),
        canonical_fixture_image(final_image.fixtures)
    );
}
#[test]
fn continuous_fixture_fader_real_engine_cancel_retains_interrupted_undo() {
    let harness = MediaAssetA6CommandHarness::new();
    let state = &harness.state;
    let window = "fader-cancel-window";
    let owner = "fader-cancel-owner";
    register_project_transaction_owner_for_window_label(state, window, owner.into()).unwrap();
    seed_two_stage_fixtures(state);
    let before = stage_image(state);
    let status = stage_history_status(state);
    let command = "stream_fixture_attribute_v1";
    let ticket = d4_begin_stage_ticket(state, window, owner, command, 911);
    continuous_fixture_fader::write(
        state,
        window,
        continuous_fixture_fader::Request {
            targets: vec![TouchFeaturePresetTarget {
                fixture_id: 1,
                attribute: "Dimmer".into(),
            }],
            value: 50000,
            sequence: 1,
            project_transaction_id: ticket.transaction_id,
            expected_epoch: ticket.project_epoch,
            owner_id: owner.into(),
        },
    )
    .unwrap();
    let changed = stage_image(state);
    assert_ne!(changed.fixtures, before.fixtures);
    cancel_project_transaction_for_window_label(
        state,
        project_transaction_cancel_request!(
            window,
            ticket.transaction_id,
            ticket.project_epoch,
            ticket.client_operation_id,
            ticket.shape_fingerprint,
            command,
            ticket.schema_version,
            owner
        ),
    )
    .unwrap();
    // Cancel preserves values already sent live, with an Interrupted Undo entry.
    assert_eq!(stage_image(state).fixtures, changed.fixtures);
    let interrupted = stage_history_status(state);
    assert_eq!(interrupted.undo_depth, status.undo_depth + 1);
    navigate_stage_history(
        state,
        true,
        interrupted.project_epoch,
        interrupted.undo_entry_id,
        Some(interrupted.checkpoint_hash),
    );
    assert_eq!(
        canonical_fixture_image(stage_image(state).fixtures),
        canonical_fixture_image(before.fixtures)
    );
}

#[test]
fn fixture_repatch_real_engine_bulk_publication_is_one_undo_redo_entry() {
    let harness = MediaAssetA6CommandHarness::new();
    let state = &harness.state;
    let window = "bulk-patch-history-window";
    let owner = "bulk-patch-history-owner";
    register_project_transaction_owner_for_window_label(state, window, owner.into()).unwrap();
    seed_two_stage_fixtures(state);
    let before = stage_image(state);
    let status = stage_history_status(state);
    let command = "set_fixture_patches";
    let ticket = d4_begin_stage_ticket(state, window, owner, command, 912);
    let (external, coordinator, nested) = lock_renderer_ticketed_project_mutation(
        state,
        window,
        command,
        ticket.transaction_id,
        ticket.project_epoch,
        owner,
    )
    .unwrap();
    let lane = project_transaction_lane_for_operation(state, &ticket.client_operation_id).unwrap();
    let dispatch = admit_outer_renderer_dispatch(command, &lane).unwrap();
    drop(nested);
    drop(coordinator);
    drop(external);
    let changes = before
        .fixtures
        .iter()
        .map(|f| engine::FixturePatchChange {
            fixture_id: f.id,
            expected_universe: f.universe,
            expected_address: f.address,
            expected_footprint: f
                .controls
                .iter()
                .flat_map(|c| c.offsets.iter())
                .copied()
                .max()
                .unwrap() as u16,
            label: f.label.clone(),
            universe: 1,
            address: f.address + 64,
        })
        .collect();
    state
        .engine
        .set_fixture_patches_published_with_timeout(changes, Duration::from_secs(2))
        .unwrap();
    drop(dispatch);
    let final_image = stage_image(state);
    assert_eq!(
        final_image
            .fixtures
            .iter()
            .map(|f| (f.universe, f.address))
            .collect::<Vec<_>>(),
        vec![(1, 65), (1, 84)]
    );
    d4_commit_stage_ticket(state, window, owner, command, &ticket);
    let committed = stage_history_status(state);
    assert_eq!(committed.undo_depth, status.undo_depth + 1);
    let undone = navigate_stage_history(
        state,
        true,
        committed.project_epoch,
        committed.undo_entry_id,
        Some(committed.checkpoint_hash),
    );
    assert_eq!(
        canonical_fixture_image(stage_image(state).fixtures),
        canonical_fixture_image(before.fixtures)
    );
    navigate_stage_history(
        state,
        false,
        undone.history_status.project_epoch,
        undone.history_status.redo_entry_id,
        undone.history_status.redo_checkpoint_hash,
    );
    assert_eq!(
        canonical_fixture_image(stage_image(state).fixtures),
        canonical_fixture_image(final_image.fixtures)
    );
}
